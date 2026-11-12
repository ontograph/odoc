//! Benchmark harness: split, pool, run and score.
//!
//! Files (all paths in the config are relative to the config file):
//! - config JSON ([`EvalConfig`]): corpora roots, queries, qrels, splits,
//!   seed, resamples, sweep grids and the parameters of the run.
//! - queries JSONL ([`Query`]); qrels JSONL ([`Judgement`], one per
//!   query/section); splits JSON ([`Splits`]).
//! - run directory: `meta.json` (identity), one tab-separated TREC-style file
//!   per system (`<qid> Q0 <section> <rank> <score> <system>`),
//!   `abstention.tsv` (`<qid> <s>`) and `report.json`.
//!
//! Ranking metrics use abstention off and answerable queries only; H3 uses
//! the recorded abstention scores. All outputs are deterministic.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::corpus::sha256_hex;
use crate::search::{Mode, Params, SearchIndex};
use crate::store::StoreError;

use super::metrics::{self, Qrels};
use super::stats::{self, Decision, Interval, SplitMix64};

pub const CUTOFF: usize = 10;
/// Chunks fetched per query before collapsing to sections.
const FETCH: usize = 50;
pub const MIN_EFFECT: f64 = 0.05;
pub const ALPHA: f64 = 0.05;
const Z95: f64 = 1.959_963_984_540_054;

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    #[error("{path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path:?} line {line}: {message}")]
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },
    #[error("invalid benchmark: {0}")]
    Invalid(String),
    #[error("the test split needs --confirm-test-split (one run per frozen configuration)")]
    TestSplitRefused,
    #[error("corpus {corpus}: {source}")]
    Store { corpus: String, source: StoreError },
    #[error("serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> EvalError + '_ {
    move |source| EvalError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stratum {
    Lookup,
    Conceptual,
    MultiHop,
    OffTopic,
}

impl Stratum {
    fn name(self) -> &'static str {
        match self {
            Self::Lookup => "lookup",
            Self::Conceptual => "conceptual",
            Self::MultiHop => "multi-hop",
            Self::OffTopic => "off-topic",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub id: String,
    pub text: String,
    pub stratum: Stratum,
    pub corpus: String,
    /// Drafted by a model; review provenance is recorded separately.
    #[serde(default)]
    pub model_drafted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgement {
    pub query_id: String,
    pub section_id: String,
    pub grade: u8,
    #[serde(default)]
    pub span_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusSpec {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    pub corpora: Vec<CorpusSpec>,
    pub queries: String,
    pub qrels: String,
    pub splits: String,
    /// Execution never falls back to the combined preparation files.
    #[serde(default)]
    pub partitions: Option<Partitions>,
    pub seed: u64,
    pub resamples: usize,
    pub alpha_grid: Vec<f64>,
    pub beta_grid: Vec<f64>,
    pub alpha: f64,
    pub beta: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionFiles {
    pub queries: String,
    pub qrels: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partitions {
    pub dev: PartitionFiles,
    pub test: PartitionFiles,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Splits {
    pub seed: u64,
    pub dev: Vec<String>,
    pub test: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum SplitName {
    Dev,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum System {
    B0,
    B1,
    B2(f64),
    B3(f64),
}

impl System {
    pub fn name(self) -> String {
        match self {
            Self::B0 => "B0".into(),
            Self::B1 => "B1".into(),
            Self::B2(a) => format!("B2@{a}"),
            Self::B3(b) => format!("B3@{b}"),
        }
    }
}

/// A loaded benchmark: config, queries, qrels and one index per corpus.
pub struct Bench {
    pub(crate) dir: PathBuf,
    pub(crate) config: EvalConfig,
    pub(crate) config_sha256: String,
    pub(crate) queries: Vec<Query>,
    pub(crate) queries_sha256: String,
    captured: Option<CapturedInputs>,
}

// Fixed ceilings bound captured payloads, not tunable ranking parameters.
// Freshness capture uses the corpus owner's pre-filter traversal and explicit
// ignore-input budgets; it does not establish descriptor-rooted containment.
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 128 * 1024 * 1024;
const MAX_CAPTURE_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_RECORDS: usize = 100_000;
const MAX_QUERIES: usize = 2_000;
const MAX_RANKINGS: usize = 100_000;
/// Upper bound on one score written as `{:e}` (shortest round-trip mantissa
/// plus exponent, e.g. `-1.2345678901234567e-308` is 24 bytes).
const MAX_SCORE_TEXT: usize = 32;
/// Validated query-id ceiling (see `validate_queries`).
const MAX_QUERY_ID_BYTES: usize = 256;

/// Worst-case total bytes of all run files (one ranking file per system plus
/// `abstention.tsv`) for queries with ids `qids`, given the longest section id
/// of the ranked corpora. Scoring reads the run files as one 16-MiB payload
/// budget. Computed from split ids and index metadata only, so it can refuse
/// before a held-out attempt is reserved.
fn run_file_bound<'a>(
    qids: impl IntoIterator<Item = &'a String>,
    indexes: &IndexMap,
    systems: &[System],
) -> usize {
    let section = indexes
        .values()
        // The emitted field: rankings write each chunk's `section_id`.
        .flat_map(|i| i.generation.chunks.iter().map(|c| c.section_id.len()))
        .max()
        .unwrap_or(0);
    // `qid\tQ0\tsection\trank\tscore\tname\n`: 5 tabs, newline, `Q0` and a
    // rank of at most 2 digits (CUTOFF is 10) add at most 10 bytes.
    let per_query_fixed: usize = systems
        .iter()
        .map(|s| CUTOFF.saturating_mul(section + MAX_SCORE_TEXT + s.name().len() + 10))
        .fold(0, usize::saturating_add);
    let ranked_lines = CUTOFF.saturating_mul(systems.len());
    qids.into_iter()
        .map(|q| {
            // Ranking lines plus one abstention line `qid\tscore\n`.
            let qid = q.len();
            per_query_fixed
                .saturating_add(ranked_lines.saturating_mul(qid))
                .saturating_add(qid + MAX_SCORE_TEXT + 2)
        })
        .fold(0, usize::saturating_add)
}

fn check_run_file_bytes(bytes: usize) -> Result<(), EvalError> {
    if bytes > MAX_INPUT_BYTES {
        return Err(EvalError::Invalid(format!(
            "run files could reach {bytes} bytes, above the {MAX_INPUT_BYTES}-byte payload ceiling"
        )));
    }
    Ok(())
}

pub(crate) fn read(path: &Path) -> Result<Vec<u8>, EvalError> {
    read_bounded(path, MAX_INPUT_BYTES)
}

fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>, EvalError> {
    use std::os::unix::fs::OpenOptionsExt;
    // Linux ABI flags: avoid both a final symlink and blocking on a substituted
    // FIFO before fstat. This is trusted CLI input, not a rooted network reader.
    const O_NOFOLLOW: i32 = 0o400000;
    const O_NONBLOCK: i32 = 0o4000;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .map_err(io_err(path))?;
    let meta = file.metadata().map_err(io_err(path))?;
    if !meta.is_file() || meta.len() > cap as u64 {
        return Err(EvalError::Invalid(format!(
            "input is not a bounded regular file: {}",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_err(path))?;
    if bytes.len() > cap {
        return Err(EvalError::Invalid("input byte limit exceeded".into()));
    }
    Ok(bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), EvalError> {
    fs::write(path, bytes).map_err(io_err(path))
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<(Vec<T>, String), EvalError> {
    let bytes = read(path)?;
    parse_jsonl(path, &bytes).map(|records| (records, sha256_hex(&bytes)))
}

pub(crate) fn parse_jsonl<T: for<'de> Deserialize<'de>>(
    path: &Path,
    bytes: &[u8],
) -> Result<Vec<T>, EvalError> {
    let mut budget = MAX_RECORDS;
    parse_jsonl_within(path, bytes, &mut budget, |_| 1)
}

/// Parses JSONL while charging each record's `weight` (itself plus nested
/// entries) to a shared record budget. Lines are bounded before parsing, so
/// exhaustion is refused before allocation grows past one line.
pub(crate) fn parse_jsonl_within<T: for<'de> Deserialize<'de>>(
    path: &Path,
    bytes: &[u8],
    budget: &mut usize,
    weight: impl Fn(&T) -> usize,
) -> Result<Vec<T>, EvalError> {
    let text = std::str::from_utf8(bytes).map_err(|e| EvalError::Parse {
        path: path.to_path_buf(),
        line: 0,
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if *budget == 0 || line.len() > 32 * 1024 {
            return Err(EvalError::Invalid("JSONL record limit exceeded".into()));
        }
        let record: T = serde_json::from_str(line).map_err(|e| EvalError::Parse {
            path: path.to_path_buf(),
            line: i + 1,
            message: e.to_string(),
        })?;
        let w = weight(&record);
        if w > *budget {
            return Err(EvalError::Invalid("JSONL record limit exceeded".into()));
        }
        *budget -= w;
        out.push(record);
    }
    Ok(out)
}

impl Bench {
    pub fn load(config_path: &Path) -> Result<Self, EvalError> {
        let bytes = read(config_path)?;
        let config: EvalConfig = serde_json::from_slice(&bytes).map_err(|e| EvalError::Parse {
            path: config_path.to_path_buf(),
            line: e.line(),
            message: e.to_string(),
        })?;
        validate_config(&config)?;
        let dir = config_path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let (queries, queries_sha256): (Vec<Query>, String) =
            read_jsonl(&dir.join(&config.queries))?;
        validate_queries(&queries, &config)?;
        let names: BTreeSet<&str> = config.corpora.iter().map(|c| c.name.as_str()).collect();
        let mut ids = BTreeSet::new();
        for q in &queries {
            if !ids.insert(q.id.as_str()) {
                return Err(EvalError::Invalid(format!("duplicate query id {}", q.id)));
            }
            if !names.contains(q.corpus.as_str()) {
                return Err(EvalError::Invalid(format!(
                    "query {}: unknown corpus {}",
                    q.id, q.corpus
                )));
            }
        }
        Ok(Self {
            dir,
            config,
            config_sha256: sha256_hex(&bytes),
            queries,
            queries_sha256,
            captured: None,
        })
    }

    /// Qrels per query id and the qrels file digest.
    pub fn qrels(&self) -> Result<(BTreeMap<String, Qrels>, String), EvalError> {
        if let Some(c) = &self.captured {
            return Ok((c.qrels.clone(), c.qrels_sha256.clone()));
        }
        let (judgements, digest) = self.judgements()?;
        Ok((validate_judgements(&judgements)?, digest))
    }

    pub(crate) fn judgements(&self) -> Result<(Vec<Judgement>, String), EvalError> {
        if let Some(c) = &self.captured {
            return Ok((c.judgements.clone(), c.qrels_sha256.clone()));
        }
        let (judgements, digest) = read_jsonl(&self.dir.join(&self.config.qrels))?;
        validate_judgements(&judgements)?;
        Ok((judgements, digest))
    }

    pub fn splits(&self) -> Result<(Splits, String), EvalError> {
        if let Some(c) = &self.captured {
            return Ok((c.splits.clone(), c.identity.splits.clone()));
        }
        let path = self.dir.join(&self.config.splits);
        let bytes = read(&path)?;
        let splits = serde_json::from_slice(&bytes)?;
        Ok((splits, sha256_hex(&bytes)))
    }

    /// One parsed index per corpus for captured execution; preparation may load lazily.
    pub fn indexes(&self) -> Result<Arc<BTreeMap<String, SearchIndex>>, EvalError> {
        if let Some(c) = &self.captured {
            return Ok(Arc::clone(&c.indexes));
        }
        let mut budget = MAX_CAPTURE_BYTES;
        capture_indexes(&self.dir, &self.config, &mut budget).map(|(indexes, _)| Arc::new(indexes))
    }
}

fn validate_judgements(judgements: &[Judgement]) -> Result<BTreeMap<String, Qrels>, EvalError> {
    let mut out: BTreeMap<String, Qrels> = BTreeMap::new();
    for j in judgements {
        if j.query_id.is_empty()
            || j.section_id.is_empty()
            || j.query_id.len() > 256
            || j.section_id.len() > 4096
        {
            return Err(EvalError::Invalid("invalid judgement identifier".into()));
        }
        if j.grade > 2 {
            return Err(EvalError::Invalid(format!("grade {} > 2", j.grade)));
        }
        let entry = out.entry(j.query_id.clone()).or_default();
        if entry.insert(j.section_id.clone(), j.grade).is_some() {
            return Err(EvalError::Invalid(format!(
                "duplicate judgement {} {}",
                j.query_id, j.section_id
            )));
        }
    }
    Ok(out)
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InputIdentity {
    config: String,
    gates: String,
    splits: String,
    pool: String,
    indexes: BTreeMap<String, String>,
}

/// `[dev, test]` exact (queries, qrels) payload digests.
type PartitionPairs = [(String, String); 2];

struct CapturedInputs {
    identity: InputIdentity,
    splits: Splits,
    judgements: Vec<Judgement>,
    qrels: BTreeMap<String, Qrels>,
    qrels_sha256: String,
    indexes: Arc<BTreeMap<String, SearchIndex>>,
    gates: super::gates::GateConfig,
    /// Full pool bytes; absent on selected execution captures (ADR9: dev
    /// never opens, stats or hashes the full pool).
    pool: Option<Vec<u8>>,
    selected: Option<SplitName>,
    preparation_partitions: Option<[(String, String); 2]>,
    /// SHA-256 of the admitted manifest bytes.
    preparation_sha256: Option<String>,
    /// Manifest-approved answerable test count (positive qrels); lets dev
    /// freeze/check use test power without loading test labels.
    test_answerable: Option<usize>,
    /// Manifest label provenance (ADR revision 13); `None` for human labels.
    label_provenance: Option<String>,
    meta_hashes: BTreeMap<String, String>,
    freshness_failures: Vec<super::gates::GateFailure>,
    pin_failures: Vec<super::gates::GateFailure>,
}

/// Trusted-operator preparation inputs; the review pin comes from the audit.
#[derive(Debug, Clone, Copy)]
pub struct TrustedInputs<'a> {
    pub query_review: &'a Path,
    pub query_review_sha256: &'a str,
    pub annotation: &'a Path,
    pub annotation_notes: &'a Path,
    /// Operator identity and evidence reference recorded in the manifest.
    pub operator: &'a str,
    pub operator_evidence: &'a str,
}

pub const PREPARATION_VERSION: u32 = 1;
pub const PROMOTION_POLICY: &str = "h5-then-h2-v1";
/// Version 2 binds the preparation hash; version 1 is refused.
pub const RUN_META_VERSION: u32 = 2;
const REGISTRATION_VERSION: u32 = 2;
const MAX_PREPARATION_BYTES: usize = 1024 * 1024;
const PARTITION_DOMAIN: &str = "ontodoc-partition-commitment-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparationApproval {
    operator: String,
    evidence: String,
}

/// Exact raw-byte SHA-256 of every preparation input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedInputs {
    config: String,
    gates: String,
    splits: String,
    queries: String,
    qrels: String,
    pool: String,
    query_review: String,
    annotation: String,
    annotation_notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedCorpus {
    name: String,
    commit: String,
    generation_id: String,
    index_sha256: String,
    meta_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedPartition {
    queries: String,
    qrels: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedPartitions {
    dev: PreparedPartition,
    test: PreparedPartition,
    /// Domain-tagged commitment to the ordered (dev, test) partition hashes.
    commitment: String,
}

/// Split -> corpus -> stratum -> query count.
type StratumCounts = BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedCounts {
    queries: StratumCounts,
    /// Per split: answerable queries with at least one grade >= 1.
    answerable_positive: BTreeMap<String, usize>,
    pool_pairs: usize,
    reviewed: usize,
    model_drafted: usize,
    agreement: super::gates::Agreement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedPolicy {
    promotion_policy: String,
    seed: u64,
    alpha_grid: Vec<f64>,
    beta_grid: Vec<f64>,
    gate_report_sha256: String,
}

/// Version-1 qrels-stage `preparation.json` (ADR9 "Exact-Byte Preparation
/// Handoff"). Minted by [`Bench::prepare`] after every full-data gate passes;
/// across processes it is admitted only by [`Preparation::verify`] against an
/// out-of-band operator pin. A trusted handoff, not tamper-proof storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    version: u32,
    stage: super::gates::Stage,
    approval: PreparationApproval,
    inputs: PreparedInputs,
    corpora: Vec<PreparedCorpus>,
    partitions: PreparedPartitions,
    counts: PreparedCounts,
    policy: PreparedPolicy,
    /// SHA-256 of the exact admitted (or about-to-be-published) bytes. Not
    /// serialized: floats need not reserialize byte-identically, so the
    /// digest is kept rather than recomputed.
    #[serde(skip)]
    sha256: String,
}

fn partition_commitment(dev: &PreparedPartition, test: &PreparedPartition) -> String {
    sha256_hex(
        format!(
            "{PARTITION_DOMAIN}\n{}\n{}\n{}\n{}\n",
            dev.queries, dev.qrels, test.queries, test.qrels
        )
        .as_bytes(),
    )
}

fn lower_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn preparation_error(detail: &str) -> EvalError {
    EvalError::Invalid(format!("preparation refused: {detail}"))
}

impl Preparation {
    /// Canonical manifest bytes; the operator pins their SHA-256.
    pub fn to_bytes(&self) -> Result<Vec<u8>, EvalError> {
        let mut bytes = serde_json::to_vec_pretty(self)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_PREPARATION_BYTES {
            return Err(preparation_error("manifest exceeds 1 MiB"));
        }
        Ok(bytes)
    }

    /// Publishes the manifest once (`create_new`, fsync); returns its SHA-256.
    pub fn publish(&self, out: &Path) -> Result<String, EvalError> {
        let bytes = self.to_bytes()?;
        write_new(out, &bytes)?;
        Ok(sha256_hex(&bytes))
    }

    /// Admits manifest bytes only when they match the operator pin exactly
    /// and are a well-formed version-1 qrels-stage manifest.
    pub fn verify(bytes: &[u8], pin: &str) -> Result<Self, EvalError> {
        if !lower_hex64(pin) {
            return Err(preparation_error("pin must be 64 lowercase hex digits"));
        }
        if bytes.len() > MAX_PREPARATION_BYTES {
            return Err(preparation_error("manifest exceeds 1 MiB"));
        }
        if sha256_hex(bytes) != pin {
            return Err(preparation_error("bytes differ from the pinned hash"));
        }
        let mut p: Self = serde_json::from_slice(bytes)
            .map_err(|_| preparation_error("unknown or malformed manifest"))?;
        p.sha256 = pin.into();
        let i = &p.inputs;
        let hashes = [
            &i.config,
            &i.gates,
            &i.splits,
            &i.queries,
            &i.qrels,
            &i.pool,
            &i.query_review,
            &i.annotation,
            &i.annotation_notes,
            &p.partitions.dev.queries,
            &p.partitions.dev.qrels,
            &p.partitions.test.queries,
            &p.partitions.test.qrels,
            &p.policy.gate_report_sha256,
        ];
        let corpora_sorted = p.corpora.windows(2).all(|w| w[0].name < w[1].name);
        if p.version != PREPARATION_VERSION
            || p.stage != super::gates::Stage::Qrels
            || p.policy.promotion_policy != PROMOTION_POLICY
            || !hashes.iter().all(|h| lower_hex64(h))
            || p.corpora.is_empty()
            || !corpora_sorted
            || p.corpora.iter().any(|c| {
                !lower_hex64(&c.index_sha256)
                    || !lower_hex64(&c.meta_sha256)
                    || !lower_hex64(&c.generation_id)
                    || c.commit.len() != 40
            })
            || p.partitions.commitment
                != partition_commitment(&p.partitions.dev, &p.partitions.test)
            || p.counts
                .agreement
                .label_provenance
                .as_deref()
                .is_some_and(|v| v != super::gates::MODEL_JUDGED)
        {
            return Err(preparation_error("unsupported version, stage or identity"));
        }
        Ok(p)
    }

    /// Reads at most 1 MiB without following a final symlink, then verifies.
    pub fn load(path: &Path, pin: &str) -> Result<Self, EvalError> {
        // A malformed pin refuses before the manifest is opened.
        if !lower_hex64(pin) {
            return Err(preparation_error("pin must be 64 lowercase hex digits"));
        }
        Self::verify(&read_bounded(path, MAX_PREPARATION_BYTES)?, pin)
    }

    /// SHA-256 of the manifest bytes: the operator pin once verified, or
    /// the digest of [`Preparation::to_bytes`] for a fresh preparation.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn partition_commitment(&self) -> &str {
        &self.partitions.commitment
    }

    fn partition_pairs(&self) -> [(String, String); 2] {
        let pair = |p: &PreparedPartition| (p.queries.clone(), p.qrels.clone());
        [pair(&self.partitions.dev), pair(&self.partitions.test)]
    }
}

/// Exact parsed records as a duplicate-free canonical set.
fn record_set<T: Serialize>(records: &[T]) -> Result<Option<BTreeSet<String>>, EvalError> {
    let mut set = BTreeSet::new();
    for r in records {
        if !set.insert(serde_json::to_string(r)?) {
            return Ok(None);
        }
    }
    Ok(Some(set))
}

fn captured_read(path: &Path, cap: usize, budget: &mut usize) -> Result<Vec<u8>, EvalError> {
    let bytes = read_bounded(path, cap.min(*budget))?;
    *budget -= bytes.len();
    Ok(bytes)
}

/// Refuses a benchmark directory, or any benchmark payload's parent
/// directory, inside a corpus root or its git checkout: corpus freshness
/// walks and `git status` there could otherwise stat benchmark (including
/// held-out) files. An undeterminable checkout extent refuses. Runs before
/// payload reads; parents are canonicalized so symlinked ancestors resolve.
fn check_layout<P: AsRef<Path>>(
    dir: &Path,
    config: &EvalConfig,
    payloads: &[P],
) -> Result<(), EvalError> {
    let base = if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    };
    let mut placed = vec![fs::canonicalize(base).map_err(io_err(base))?];
    for p in payloads {
        // Absolute (trusted operator) paths replace `base` when joined.
        let parent = base.join(p);
        let parent = parent.parent().unwrap_or(base);
        placed.push(fs::canonicalize(parent).map_err(io_err(parent))?);
    }
    for c in &config.corpora {
        let root = base.join(&c.root);
        let root = fs::canonicalize(&root).map_err(io_err(&root))?;
        let top = std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "--path-format=absolute", "--show-toplevel"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|t| fs::canonicalize(t.trim_end_matches('\n')).ok())
            .ok_or_else(|| {
                EvalError::Invalid(format!("cannot determine checkout of corpus {}", c.name))
            })?;
        if placed
            .iter()
            .any(|b| b.starts_with(&root) || b.starts_with(&top))
        {
            return Err(EvalError::Invalid(format!(
                "benchmark directory must be outside corpus {} and its checkout",
                c.name
            )));
        }
    }
    Ok(())
}

/// Non-empty relative path of only normal components (no `..`, root or `.`).
fn relative_file(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 4096
        && Path::new(p)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn validate_config(config: &EvalConfig) -> Result<(), EvalError> {
    let names: BTreeSet<_> = config.corpora.iter().map(|c| &c.name).collect();
    if config.corpora.is_empty()
        || config.corpora.len() > 8
        || names.len() != config.corpora.len()
        || config
            .corpora
            .iter()
            .any(|c| c.name.is_empty() || c.name.len() > 256 || c.root.len() > 4096)
        || !(1..=super::sweep::REQUIRED_RESAMPLES).contains(&config.resamples)
        || [&config.alpha_grid, &config.beta_grid].iter().any(|grid| {
            grid.is_empty()
                || grid.len() > 32
                || grid
                    .iter()
                    .any(|w| !w.is_finite() || !(0.0..=1.0).contains(w))
        })
    {
        return Err(EvalError::Invalid(
            "invalid or oversized corpus/grid/resamples configuration".into(),
        ));
    }
    Ok(())
}

fn validate_queries(queries: &[Query], config: &EvalConfig) -> Result<(), EvalError> {
    let mut ids = BTreeSet::new();
    if queries.len() > MAX_QUERIES {
        return Err(EvalError::Invalid("query work limit exceeded".into()));
    }
    for q in queries {
        if check_tsv_field(&q.id).is_err()
            || q.id.len() > MAX_QUERY_ID_BYTES
            || q.text.len() > 4096
            || !ids.insert(&q.id)
            || !config.corpora.iter().any(|c| c.name == q.corpus)
        {
            return Err(EvalError::Invalid(
                "invalid, duplicate or unknown-corpus query".into(),
            ));
        }
    }
    if queries.len() * (2 + config.alpha_grid.len() + config.beta_grid.len()) > MAX_RANKINGS {
        return Err(EvalError::Invalid("ranking work limit exceeded".into()));
    }
    Ok(())
}

type IndexMap = BTreeMap<String, SearchIndex>;
/// Bench, raw partition digests and remaining `(bytes, records)` budget.
type Captured = (Bench, [(String, String); 2], (usize, usize));
/// Per corpus: `(index.json sha256, meta.json sha256)`.
type IndexHashes = BTreeMap<String, (String, String)>;
fn capture_indexes(
    dir: &Path,
    config: &EvalConfig,
    budget: &mut usize,
) -> Result<(IndexMap, IndexHashes), EvalError> {
    validate_config(config)?;
    let mut indexes = BTreeMap::new();
    let mut hashes = BTreeMap::new();
    for c in &config.corpora {
        let store = dir.join(&c.root).join(".ontodoc");
        let pointer = captured_read(&store.join("CURRENT"), 128, budget)?;
        let id = std::str::from_utf8(&pointer)
            .map_err(|_| EvalError::Invalid("invalid generation pointer".into()))?
            .trim();
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(EvalError::Invalid("invalid generation pointer".into()));
        }
        let generation_dir = store.join("generations").join(id);
        let meta = captured_read(&generation_dir.join("meta.json"), 4096, budget)?;
        let bytes = captured_read(&generation_dir.join("index.json"), MAX_INDEX_BYTES, budget)?;
        let generation =
            crate::store::decode_captured_generation(id, &meta, &bytes).map_err(|source| {
                EvalError::Store {
                    corpus: c.name.clone(),
                    source,
                }
            })?;
        // Refuse excessive derived work before constructing BM25/graph caches.
        if generation.chunks.len() > 100_000
            || generation.edges.len() > 1_000_000
            || generation.config.max_file_bytes > 16 * 1024 * 1024
            || generation.config.max_files > 100_000
        {
            return Err(EvalError::Invalid(
                "evaluation index work limit exceeded".into(),
            ));
        }
        hashes.insert(c.name.clone(), (sha256_hex(&bytes), sha256_hex(&meta)));
        indexes.insert(c.name.clone(), SearchIndex::new(generation));
    }
    Ok((indexes, hashes))
}

impl Bench {
    /// Full preparation may inspect both partitions. Only a passing full gate
    /// check can mint evidence; a JSON GateReport cannot be admitted here.
    /// `query_review_sha256` is the operator-audited pin, not a hash of
    /// whatever file currently exists.
    pub fn prepare(
        config_path: &Path,
        trusted: &TrustedInputs<'_>,
    ) -> Result<Preparation, EvalError> {
        if trusted.operator.trim().is_empty()
            || trusted.operator.len() > 128
            || trusted.operator_evidence.trim().is_empty()
            || trusted.operator_evidence.len() > 4096
        {
            return Err(preparation_error("operator identity and evidence required"));
        }
        let (bench, partitions, (mut bytes, mut records)) =
            Self::capture(config_path, None, None, Some(trusted))?;
        let c = bench
            .captured
            .as_ref()
            .ok_or_else(|| EvalError::Invalid("missing capture".into()))?;
        // Combined authoring files equal the exact union of both partitions.
        let dir = &bench.dir;
        let combined_q = captured_read(
            &dir.join(&bench.config.queries),
            MAX_INPUT_BYTES,
            &mut bytes,
        )?;
        let combined_j =
            captured_read(&dir.join(&bench.config.qrels), MAX_INPUT_BYTES, &mut bytes)?;
        let qs: Vec<Query> = parse_jsonl_within(
            &dir.join(&bench.config.queries),
            &combined_q,
            &mut records,
            |_| 1,
        )?;
        let js: Vec<Judgement> = parse_jsonl_within(
            &dir.join(&bench.config.qrels),
            &combined_j,
            &mut records,
            |_| 1,
        )?;
        let union_q = record_set(&bench.queries)?;
        let union_j = record_set(&c.judgements)?;
        if record_set(&qs)?.is_none_or(|s| Some(s) != union_q)
            || record_set(&js)?.is_none_or(|s| Some(s) != union_j)
        {
            return Err(preparation_error(
                "combined authoring files differ from the partition union",
            ));
        }
        let review = captured_read(trusted.query_review, MAX_INPUT_BYTES, &mut bytes)?;
        let approval = super::gates::check_query_review_within(
            &bench.queries,
            &review,
            trusted.query_review_sha256,
            &mut records,
        )?;
        let annotation = captured_read(trusted.annotation, MAX_INPUT_BYTES, &mut bytes)?;
        let notes = captured_read(trusted.annotation_notes, MAX_INPUT_BYTES, &mut bytes)?;
        let provenance = super::gates::Provenance {
            approval: Some(&approval),
            annotation: Some(super::gates::AnnotationInput {
                records: &annotation,
                notes: &notes,
                budget: records,
            }),
        };
        let report = super::gates::check_benchmark(
            &bench,
            &c.gates,
            super::gates::Stage::Qrels,
            None,
            &provenance,
        )?;
        if !report.passed {
            return Err(EvalError::Invalid(format!(
                "mechanical prevalidation failed: {}",
                serde_json::to_string(&report.failures)?
            )));
        }
        let agreement = report
            .agreement
            .clone()
            .ok_or_else(|| preparation_error("agreement missing from passing gates"))?;
        let (splits, _) = bench.splits()?;
        let mut queries: StratumCounts = BTreeMap::new();
        let mut answerable_positive = BTreeMap::new();
        for (name, ids) in [("dev", &splits.dev), ("test", &splits.test)] {
            let ids: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
            let mut positive = 0;
            for q in bench.queries.iter().filter(|q| ids.contains(q.id.as_str())) {
                *queries
                    .entry(name.into())
                    .or_default()
                    .entry(q.corpus.clone())
                    .or_default()
                    .entry(q.stratum.name().into())
                    .or_default() += 1;
                positive += usize::from(
                    q.stratum != Stratum::OffTopic
                        && c.qrels
                            .get(&q.id)
                            .is_some_and(|r| r.values().any(|&g| g >= 1)),
                );
            }
            answerable_positive.insert(name.to_string(), positive);
        }
        let pins: BTreeMap<&str, &str> = c
            .gates
            .pins
            .iter()
            .map(|p| (p.name.as_str(), p.commit.as_str()))
            .collect();
        let mut corpora = Vec::new();
        for (name, index) in c.indexes.iter() {
            let pinned = |m: &BTreeMap<String, String>| m.get(name).cloned();
            corpora.push(PreparedCorpus {
                name: name.clone(),
                commit: pins
                    .get(name.as_str())
                    .map(|c| c.to_ascii_lowercase())
                    .ok_or_else(|| preparation_error("corpus pin missing"))?,
                generation_id: index.generation.generation_id.clone(),
                index_sha256: pinned(&c.identity.indexes)
                    .ok_or_else(|| preparation_error("index hash missing"))?,
                meta_sha256: pinned(&c.meta_hashes)
                    .ok_or_else(|| preparation_error("meta hash missing"))?,
            });
        }
        let [dev, test] = partitions.map(|(queries, qrels)| PreparedPartition { queries, qrels });
        let commitment = partition_commitment(&dev, &test);
        let pool_pairs = c
            .pool
            .as_deref()
            .ok_or_else(|| preparation_error("full capture required"))?
            .split(|&b| b == b'\n')
            .filter(|l| !l.trim_ascii().is_empty())
            .count();
        let mut preparation = Preparation {
            version: PREPARATION_VERSION,
            stage: super::gates::Stage::Qrels,
            approval: PreparationApproval {
                operator: trusted.operator.into(),
                evidence: trusted.operator_evidence.into(),
            },
            inputs: PreparedInputs {
                config: c.identity.config.clone(),
                gates: c.identity.gates.clone(),
                splits: c.identity.splits.clone(),
                queries: sha256_hex(&combined_q),
                qrels: sha256_hex(&combined_j),
                pool: c.identity.pool.clone(),
                query_review: approval.sha256().into(),
                annotation: sha256_hex(&annotation),
                annotation_notes: sha256_hex(&notes),
            },
            corpora,
            partitions: PreparedPartitions {
                dev,
                test,
                commitment,
            },
            counts: PreparedCounts {
                queries,
                answerable_positive,
                pool_pairs,
                reviewed: approval.reviewed(),
                model_drafted: approval.model_drafted(),
                agreement,
            },
            policy: PreparedPolicy {
                promotion_policy: PROMOTION_POLICY.into(),
                seed: bench.config.seed,
                alpha_grid: bench.config.alpha_grid.clone(),
                beta_grid: bench.config.beta_grid.clone(),
                gate_report_sha256: sha256_hex(&serde_json::to_vec(&report)?),
            },
            sha256: String::new(),
        };
        preparation.sha256 = sha256_hex(&preparation.to_bytes()?);
        Ok(preparation)
    }

    /// [`Bench::load_dev`] after verifying manifest bytes against the
    /// operator's out-of-band pin; the cross-process admission path.
    pub fn load_dev_pinned(
        config_path: &Path,
        preparation: &Path,
        pin: &str,
    ) -> Result<Self, EvalError> {
        Self::load_dev(config_path, &Preparation::load(preparation, pin)?)
    }

    /// Select paths before any payload read. No combined or held-out fallback.
    /// Crate-private: a deserialized [`Preparation`] is not approval. External
    /// callers must use [`Bench::load_dev_pinned`], which verifies the exact
    /// manifest bytes against the operator pin.
    pub(crate) fn load_dev(config_path: &Path, evidence: &Preparation) -> Result<Self, EvalError> {
        Self::load_selected(config_path, evidence, SplitName::Dev)
    }

    /// Reserves the sole held-out attempt (create-new + fsync) before any
    /// test payload is opened. The registration must already bind this
    /// manifest's partitions; that check reads no held-out data and does not
    /// consume the attempt. A capture failure after reservation records a
    /// failed, consumed attempt.
    fn load_test_reserved(
        config_path: &Path,
        evidence: &Preparation,
        custody: &Custody,
        out: &Path,
    ) -> Result<(Self, AttemptStarted), EvalError> {
        let (registration, _) = custody.registration()?;
        // Metadata-only identity checks before reservation: no payload reads.
        let r = &registration.identity;
        let i = &evidence.inputs;
        let indexes: BTreeMap<String, String> = evidence
            .corpora
            .iter()
            .map(|c| (c.name.clone(), c.index_sha256.clone()))
            .collect();
        if registration.preparation_partitions != evidence.partition_pairs()
            || registration.preparation_sha256 != evidence.sha256()
            || (&r.config, &r.gates, &r.splits, &r.pool, &r.indexes)
                != (&i.config, &i.gates, &i.splits, &i.pool, &indexes)
        {
            return Err(EvalError::Invalid(
                "registration does not bind this preparation".into(),
            ));
        }
        let started = start_attempt(custody, out)?;
        match Self::load_selected(config_path, evidence, SplitName::Test) {
            Ok(bench) => Ok((bench, started)),
            Err(error) => {
                finish_attempt(custody, &started, None)?;
                Err(error)
            }
        }
    }

    fn load_selected(
        config_path: &Path,
        evidence: &Preparation,
        split: SplitName,
    ) -> Result<Self, EvalError> {
        let (mut bench, partitions, _) =
            Self::capture(config_path, Some(split), Some(&evidence.inputs), None)?;
        let c = bench
            .captured
            .as_ref()
            .ok_or_else(|| EvalError::Invalid("missing capture".into()))?;
        let i = &evidence.inputs;
        let corpora: Vec<_> = c
            .indexes
            .iter()
            .map(|(name, index)| {
                (
                    name.as_str(),
                    index.generation.generation_id.as_str(),
                    c.identity.indexes.get(name).map(String::as_str),
                    c.meta_hashes.get(name).map(String::as_str),
                    c.gates
                        .pins
                        .iter()
                        .find(|p| &p.name == name)
                        .map(|p| p.commit.to_ascii_lowercase()),
                )
            })
            .collect();
        let expected: Vec<_> = evidence
            .corpora
            .iter()
            .map(|p| {
                (
                    p.name.as_str(),
                    p.generation_id.as_str(),
                    Some(p.index_sha256.as_str()),
                    Some(p.meta_sha256.as_str()),
                    Some(p.commit.clone()),
                )
            })
            .collect();
        let k = usize::from(split == SplitName::Test);
        let approved = &evidence.partition_pairs()[k];
        if (&c.identity.config, &c.identity.gates, &c.identity.splits)
            != (&i.config, &i.gates, &i.splits)
            || corpora != expected
            || partitions[k] != *approved
        {
            return Err(EvalError::Invalid(
                "captured selected identity differs from prevalidation".into(),
            ));
        }
        if !c.freshness_failures.is_empty() || !c.pin_failures.is_empty() {
            return Err(EvalError::Invalid(
                "corpus freshness or pin changed since preparation".into(),
            ));
        }
        if let Some(c) = &mut bench.captured {
            c.preparation_partitions = Some(evidence.partition_pairs());
            c.preparation_sha256 = Some(evidence.sha256().into());
            c.test_answerable = evidence.counts.answerable_positive.get("test").copied();
            c.label_provenance = evidence.counts.agreement.label_provenance.clone();
            c.identity.pool = i.pool.clone();
        }
        Ok(bench)
    }

    pub fn queries(&self) -> &[Query] {
        &self.queries
    }

    /// Requires a split-isolated capture of `split` admitted by a manifest.
    pub(crate) fn require_prepared_capture(&self, split: SplitName) -> Result<(), EvalError> {
        if self
            .preparation_binding()
            .is_none_or(|(s, _, _)| s != split)
        {
            return Err(EvalError::Invalid(format!(
                "measurement requires an admitted preparation capture of the {split:?} split"
            )));
        }
        Ok(())
    }

    /// Manifest-approved answerable test count of an admitted capture.
    pub(crate) fn prepared_test_answerable(&self) -> Option<usize> {
        self.preparation_binding()?;
        self.captured.as_ref()?.test_answerable
    }

    /// Selected split, manifest hash and `[dev, test]` (queries, qrels)
    /// digests of a manifest-admitted capture; `None` for any other bench.
    pub(crate) fn preparation_binding(&self) -> Option<(SplitName, &str, &PartitionPairs)> {
        let c = self.captured.as_ref()?;
        Some((
            c.selected?,
            c.preparation_sha256.as_deref()?,
            c.preparation_partitions.as_ref()?,
        ))
    }

    pub(crate) fn freshness_failures(&self) -> Result<Vec<super::gates::GateFailure>, EvalError> {
        if let Some(c) = &self.captured {
            return Ok(c.freshness_failures.clone());
        }
        let indexes = self.indexes()?;
        let mut budget = MAX_CAPTURE_BYTES;
        check_current_corpora(&self.dir, &self.config, &indexes, &mut budget)
    }

    pub(crate) fn pin_failures(
        &self,
        pins: &[super::gates::CorpusPin],
    ) -> Option<Vec<super::gates::GateFailure>> {
        self.captured.as_ref().map(|c| {
            if pins == c.gates.pins {
                c.pin_failures.clone()
            } else {
                vec![super::gates::GateFailure {
                    gate: "pin_capture_mismatch",
                    detail: "pin policy differs from capture".into(),
                }]
            }
        })
    }

    pub(crate) fn pool_bytes(&self, path: &Path) -> Result<Vec<u8>, EvalError> {
        match &self.captured {
            Some(c)
                if c.gates
                    .pool
                    .as_ref()
                    .is_some_and(|p| self.dir.join(p) == path) =>
            {
                c.pool
                    .clone()
                    .ok_or_else(|| EvalError::Invalid("selected capture holds no pool".into()))
            }
            Some(_) => Err(EvalError::Invalid("pool path differs from capture".into())),
            None => read(path),
        }
    }

    /// Returns the bench, raw partition digests and the remaining shared
    /// `(bytes, records)` capture budget for trusted provenance inputs.
    /// `expected` (selected captures) binds config, splits and gates bytes to
    /// the admitted manifest before any partition payload is read.
    /// `trusted` (full preparation) adds operator input paths to the layout
    /// check, which precedes all freshness/git corpus inspection.
    fn capture(
        config_path: &Path,
        selected: Option<SplitName>,
        expected: Option<&PreparedInputs>,
        trusted: Option<&TrustedInputs<'_>>,
    ) -> Result<Captured, EvalError> {
        let bound = |actual: &[u8], want: Option<&String>| match want {
            Some(w) if sha256_hex(actual) != *w => Err(EvalError::Invalid(
                "captured selected identity differs from prevalidation".into(),
            )),
            _ => Ok(()),
        };
        let mut budget = MAX_CAPTURE_BYTES;
        let mut records = MAX_RECORDS;
        let dir = config_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let config_bytes = captured_read(config_path, MAX_INPUT_BYTES, &mut budget)?;
        let config: EvalConfig = serde_json::from_slice(&config_bytes)?;
        validate_config(&config)?;
        let files = config
            .partitions
            .as_ref()
            .ok_or_else(|| EvalError::Invalid("physical dev/test partitions required".into()))?;
        // Reject lexical aliases. Membership and preparation-bound byte hashes
        // enforce separation; selected execution never stats test paths.
        let paths = [
            &files.dev.queries,
            &files.dev.qrels,
            &files.test.queries,
            &files.test.qrels,
        ];
        let mut unique = BTreeSet::new();
        for p in paths {
            let path = Path::new(p);
            if !relative_file(p)
                || !unique.insert(path.to_path_buf())
                || *p == config.queries
                || *p == config.qrels
            {
                return Err(EvalError::Invalid(
                    "partition paths must be separate relative files".into(),
                ));
            }
        }
        if !relative_file(&config.splits) {
            return Err(EvalError::Invalid(
                "splits path must be a relative file beneath the benchmark".into(),
            ));
        }
        check_layout(
            &dir,
            &config,
            &[
                &files.dev.queries,
                &files.dev.qrels,
                &files.test.queries,
                &files.test.qrels,
                &config.splits,
                "gates.json",
            ],
        )?;
        bound(&config_bytes, expected.map(|e| &e.config))?;
        let split_bytes = captured_read(&dir.join(&config.splits), MAX_INPUT_BYTES, &mut budget)?;
        bound(&split_bytes, expected.map(|e| &e.splits))?;
        let splits: Splits = serde_json::from_slice(&split_bytes)?;
        if splits.dev.len() + splits.test.len() > MAX_QUERIES {
            return Err(EvalError::Invalid("split work limit exceeded".into()));
        }
        let mut seen = BTreeSet::new();
        if splits.seed != config.seed
            || splits
                .dev
                .iter()
                .chain(&splits.test)
                .any(|id| !seen.insert(id))
        {
            return Err(EvalError::Invalid("invalid split assignment".into()));
        }
        let gates_bytes = captured_read(&dir.join("gates.json"), MAX_INPUT_BYTES, &mut budget)?;
        bound(&gates_bytes, expected.map(|e| &e.gates))?;
        let gates: super::gates::GateConfig = serde_json::from_slice(&gates_bytes)?;
        let pool_path = gates
            .pool
            .as_ref()
            .filter(|p| relative_file(p))
            .ok_or_else(|| EvalError::Invalid("pool required as a relative file".into()))?;
        if selected.is_none() {
            // Combined authoring files and caller-relative trusted paths.
            let mut placed = vec![
                PathBuf::from(pool_path),
                PathBuf::from(&config.queries),
                PathBuf::from(&config.qrels),
            ];
            if let Some(t) = trusted {
                for p in [t.query_review, t.annotation, t.annotation_notes] {
                    placed.push(std::path::absolute(p).map_err(io_err(p))?);
                }
            }
            check_layout(&dir, &config, &placed)?;
        }
        let pool = match selected {
            None => Some(captured_read(
                &dir.join(pool_path),
                MAX_INPUT_BYTES,
                &mut budget,
            )?),
            Some(_) => None,
        };
        let mut queries = Vec::new();
        let mut judgements = Vec::new();
        let mut digests = [
            (String::new(), String::new()),
            (String::new(), String::new()),
        ];
        for (i, (files, ids)) in [(&files.dev, &splits.dev), (&files.test, &splits.test)]
            .into_iter()
            .enumerate()
        {
            if selected.is_some_and(|s| (s == SplitName::Dev) != (i == 0)) {
                continue;
            }
            let qb = captured_read(&dir.join(&files.queries), MAX_INPUT_BYTES, &mut budget)?;
            let jb = captured_read(&dir.join(&files.qrels), MAX_INPUT_BYTES, &mut budget)?;
            let qs: Vec<Query> =
                parse_jsonl_within(&dir.join(&files.queries), &qb, &mut records, |_| 1)?;
            let js: Vec<Judgement> =
                parse_jsonl_within(&dir.join(&files.qrels), &jb, &mut records, |_| 1)?;
            validate_queries(&qs, &config)?;
            let actual: BTreeSet<_> = qs.iter().map(|q| &q.id).collect();
            if actual != ids.iter().collect() || js.iter().any(|j| !actual.contains(&j.query_id)) {
                return Err(EvalError::Invalid(
                    "partition contains missing, unknown or misassigned records".into(),
                ));
            }
            digests[i] = (sha256_hex(&qb), sha256_hex(&jb));
            queries.extend(qs);
            judgements.extend(js);
        }
        validate_queries(&queries, &config)?;
        let qrels = validate_judgements(&judgements)?;
        let (indexes, hashes) = capture_indexes(&dir, &config, &mut budget)?;
        let meta_hashes = hashes
            .iter()
            .map(|(name, (_, meta))| (name.clone(), meta.clone()))
            .collect();
        let freshness_failures = check_current_corpora(&dir, &config, &indexes, &mut budget)?;
        let identity = InputIdentity {
            config: sha256_hex(&config_bytes),
            gates: sha256_hex(&gates_bytes),
            splits: sha256_hex(&split_bytes),
            // Selected captures adopt the verified manifest's committed hash.
            pool: pool.as_deref().map(sha256_hex).unwrap_or_default(),
            indexes: hashes
                .into_iter()
                .map(|(name, (index, _))| (name, index))
                .collect(),
        };
        // Combined identity is only for full preparation; execution keeps the
        // exact selected payload digest, not a digest of reserialized records.
        let (queries_sha256, qrels_sha256) = match selected {
            Some(split) => digests[usize::from(split == SplitName::Test)].clone(),
            None => {
                let combined = sha256_hex(&serde_json::to_vec(&digests)?);
                (combined.clone(), combined)
            }
        };
        let captured = CapturedInputs {
            identity,
            splits,
            judgements,
            qrels,
            qrels_sha256,
            indexes: Arc::new(indexes),
            gates,
            pool,
            selected,
            preparation_partitions: None,
            preparation_sha256: None,
            test_answerable: None,
            label_provenance: None,
            meta_hashes,
            freshness_failures,
            pin_failures: Vec::new(),
        };
        let mut bench = Self {
            dir,
            config,
            config_sha256: sha256_hex(&config_bytes),
            queries,
            queries_sha256,
            captured: None,
        };
        let pin_failures = super::gates::check_pins(&bench, &captured.gates.pins);
        bench.captured = Some(CapturedInputs {
            pin_failures,
            ..captured
        });
        Ok((bench, digests, (budget, records)))
    }
}

fn check_current_corpora(
    dir: &Path,
    config: &EvalConfig,
    indexes: &IndexMap,
    bytes: &mut usize,
) -> Result<Vec<super::gates::GateFailure>, EvalError> {
    let mut failures = Vec::new();
    let mut budget = crate::corpus::WalkBudget::new(*bytes, 100_000);
    for c in &config.corpora {
        let index = indexes
            .get(&c.name)
            .ok_or_else(|| EvalError::Invalid("missing captured corpus".into()))?;
        let fresh =
            crate::store::freshness_bounded(&dir.join(&c.root), &index.generation, &mut budget)
                .map_err(|source| EvalError::Store {
                    corpus: c.name.clone(),
                    source,
                })?;
        if !fresh.is_fresh() {
            failures.push(super::gates::GateFailure {
                gate: "corpus_index_stale",
                detail: c.name.clone(),
            });
        }
    }
    *bytes = budget.bytes;
    Ok(failures)
}

/// Stratified split: within each (corpus, stratum) group, ids are sorted,
/// shuffled with one SplitMix64 stream in sorted group order, and the first
/// `(n + 1) / 3` go to dev.
pub fn split(queries: &[Query], seed: u64) -> Splits {
    let mut groups: BTreeMap<(&str, Stratum), Vec<&str>> = BTreeMap::new();
    for q in queries {
        groups
            .entry((q.corpus.as_str(), q.stratum))
            .or_default()
            .push(q.id.as_str());
    }
    let mut rng = SplitMix64::new(seed);
    let (mut dev, mut test) = (Vec::new(), Vec::new());
    for ids in groups.values_mut() {
        ids.sort_unstable();
        for i in (1..ids.len()).rev() {
            ids.swap(i, rng.below(i + 1));
        }
        let n_dev = (ids.len() + 1) / 3;
        dev.extend(ids[..n_dev].iter().map(|s| s.to_string()));
        test.extend(ids[n_dev..].iter().map(|s| s.to_string()));
    }
    dev.sort();
    test.sort();
    Splits { seed, dev, test }
}

/// Top-[`CUTOFF`] sections with the score of their first chunk.
pub fn rank_sections(index: &SearchIndex, system: System, query: &str) -> Vec<(String, f64)> {
    let hits = match system {
        System::B0 => index.b0(query, FETCH),
        System::B1 => index.search(query, &Params::new(Mode::Lexical, FETCH)).hits,
        System::B2(alpha) => {
            index
                .search(query, &Params::new(Mode::Graph { alpha }, FETCH))
                .hits
        }
        System::B3(beta) => {
            index
                .search(query, &Params::new(Mode::Hier { beta }, FETCH))
                .hits
        }
    };
    let mut out: Vec<(String, f64)> = Vec::new();
    for hit in hits {
        let id = &index.generation.chunks[hit.chunk].section_id;
        if out.len() == CUTOFF {
            break;
        }
        if !out.iter().any(|(s, _)| s == id) {
            out.push((id.clone(), hit.score));
        }
    }
    out
}

#[derive(Debug, Serialize)]
pub struct PoolSummary {
    pub queries: usize,
    pub pairs: usize,
    pub sha256: String,
}

/// Writes the judgement pool: the top-10 sections of every system at every
/// grid point for every non-off-topic query, deduplicated and sorted.
/// Refuses before ranking unless every final query has an accepted review.
pub fn pool(
    bench: &Bench,
    out: &Path,
    approval: &super::gates::QueryApproval,
) -> Result<PoolSummary, EvalError> {
    if !approval.covers(&bench.queries) {
        return Err(EvalError::Invalid(
            "query review refused: approved queries differ from the final queries".into(),
        ));
    }
    let indexes = bench.indexes()?;
    let (text, queries, pairs) = pool_captured(bench, &indexes, approval)?;
    write(out, &text)?;
    Ok(PoolSummary {
        queries,
        pairs,
        sha256: sha256_hex(&text),
    })
}

pub(crate) fn pool_captured(
    bench: &Bench,
    indexes: &IndexMap,
    approval: &super::gates::QueryApproval,
) -> Result<(Vec<u8>, usize, usize), EvalError> {
    validate_config(&bench.config)?;
    validate_queries(&bench.queries, &bench.config)?;
    if !approval.covers(&bench.queries) {
        return Err(EvalError::Invalid(
            "query review does not cover pool".into(),
        ));
    }
    let mut systems = vec![System::B0, System::B1];
    systems.extend(bench.config.alpha_grid.iter().map(|&a| System::B2(a)));
    systems.extend(bench.config.beta_grid.iter().map(|&b| System::B3(b)));
    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut queries = 0;
    for q in bench
        .queries
        .iter()
        .filter(|q| q.stratum != Stratum::OffTopic)
    {
        queries += 1;
        let index = indexes
            .get(&q.corpus)
            .ok_or_else(|| EvalError::Invalid("missing corpus index".into()))?;
        for &system in &systems {
            for (section, _) in rank_sections(index, system, &q.text) {
                pairs.insert((q.id.clone(), section));
                if pairs.len() > MAX_RECORDS {
                    return Err(EvalError::Invalid("pool pair limit exceeded".into()));
                }
            }
        }
    }
    let mut text = String::new();
    for (query_id, section_id) in &pairs {
        text.push_str(&serde_json::to_string(&serde_json::json!({
            "query_id": query_id,
            "section_id": section_id,
        }))?);
        text.push('\n');
        if text.len() > MAX_INPUT_BYTES {
            return Err(EvalError::Invalid("pool byte limit exceeded".into()));
        }
    }
    Ok((text.into_bytes(), queries, pairs.len()))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorpusIdentity {
    pub name: String,
    pub generation_id: String,
}

/// Identity of one run; copied verbatim into its report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMeta {
    /// [`RUN_META_VERSION`]; older metadata is refused, never migrated.
    pub version: u32,
    pub split: SplitName,
    /// SHA-256 of the preparation manifest that admitted the run's capture.
    pub preparation_sha256: String,
    pub seed: u64,
    pub resamples: usize,
    pub alpha: f64,
    pub beta: f64,
    pub threshold: f64,
    pub config_sha256: String,
    pub queries_sha256: String,
    pub qrels_sha256: String,
    pub splits_sha256: String,
    pub corpora: Vec<CorpusIdentity>,
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
    pub binary_sha256: Option<String>,
    pub systems: Vec<String>,
    /// Digests bind saved ranking and abstention bytes to this run.
    pub run_sha256: BTreeMap<String, String>,
}

fn git(args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn binary_sha256() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    fs::read(exe).ok().map(|b| sha256_hex(&b))
}

fn check_tsv_field(value: &str) -> Result<&str, EvalError> {
    if value.contains(['\t', '\n', '\r']) || value.is_empty() {
        return Err(EvalError::Invalid(format!(
            "id {value:?} cannot be written to a run file"
        )));
    }
    Ok(value)
}

/// Execution preflight on a manifest-admitted capture. Full-data floors,
/// leakage, pool reproducibility and provenance were enforced by the approved
/// preparation (ADR9); capture already refused changed identity, freshness or
/// pins. Recheck the selected qrels against the captured generations.
pub(crate) fn preflight(bench: &Bench) -> Result<(), EvalError> {
    let (split, _, _) = bench.preparation_binding().ok_or_else(admission_error)?;
    bench.require_prepared_capture(split)?;
    let failures = super::gates::check_qrels(bench)?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(EvalError::Invalid(format!(
            "benchmark integrity gates failed: {}",
            serde_json::to_string(&failures)?
        )))
    }
}

/// Refusal for measurement without a pinned, admitted preparation.
pub(crate) fn admission_error() -> EvalError {
    EvalError::Invalid(
        "measurement requires an admitted preparation (--preparation and --preparation-sha256)"
            .into(),
    )
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    version: u32,
    /// SHA-256 of the whole admitted preparation manifest.
    preparation_sha256: String,
    frozen: String,
    identity: InputIdentity,
    preparation_partitions: [(String, String); 2],
}

struct Custody {
    root: PathBuf,
    dir: PathBuf,
}

impl Custody {
    fn resolve(repo: &Path) -> Result<Self, EvalError> {
        let resolve = |flag| -> Result<PathBuf, EvalError> {
            let result = std::process::Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(["rev-parse", "--path-format=absolute", flag])
                .output()
                .map_err(io_err(repo))?;
            if !result.status.success() {
                return Err(EvalError::Invalid(
                    "cannot resolve code checkout custody".into(),
                ));
            }
            let text = std::str::from_utf8(&result.stdout)
                .map_err(|_| EvalError::Invalid("non-UTF8 git custody path".into()))?;
            fs::canonicalize(text.trim_end_matches('\n')).map_err(io_err(repo))
        };
        Ok(Self {
            root: resolve("--show-toplevel")?,
            dir: resolve("--git-common-dir")?.join("ontodoc-eval"),
        })
    }

    fn check_dir(&self) -> Result<(), EvalError> {
        // Trusted CLI ancestors remain a limitation; reject the final custody
        // directory symlink rather than claiming descriptor-rooted containment.
        let meta = fs::symlink_metadata(&self.dir).map_err(io_err(&self.dir))?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(EvalError::Invalid(
                "unsafe evaluation custody directory".into(),
            ));
        }
        Ok(())
    }

    fn create_dir(&self) -> Result<(), EvalError> {
        match fs::create_dir(&self.dir) {
            Ok(()) => sync_dir(parent(&self.dir))?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io_err(&self.dir)(error)),
        }
        self.check_dir()
    }

    fn registration(&self) -> Result<(Registration, String), EvalError> {
        self.check_dir()?;
        let bytes = read(&self.dir.join("registration.json"))?;
        let registration: Registration = serde_json::from_slice(&bytes)?;
        if registration.version != REGISTRATION_VERSION {
            return Err(EvalError::Invalid(
                "unsupported evaluation registration version".into(),
            ));
        }
        Ok((registration, sha256_hex(&bytes)))
    }
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn sync_dir(path: &Path) -> Result<(), EvalError> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(io_err(path))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), EvalError> {
    use std::io::Write;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(EvalError::Invalid(
            "custody record byte limit exceeded".into(),
        ));
    }
    // Never remove a partially written record: even a failed write consumes it.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io_err(path))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(io_err(path))?;
    sync_dir(parent(path))
}

fn external_path(root: &Path, path: &Path) -> Result<PathBuf, EvalError> {
    let dir = fs::canonicalize(parent(path)).map_err(io_err(parent(path)))?;
    if !dir.is_dir() {
        return Err(EvalError::Invalid(format!(
            "output parent {} is not a directory",
            dir.display()
        )));
    }
    let resolved = dir.join(
        path.file_name()
            .ok_or_else(|| EvalError::Invalid("missing output filename".into()))?,
    );
    if resolved.starts_with(root) {
        return Err(EvalError::Invalid(
            "evaluation export/output must be outside code checkout".into(),
        ));
    }
    Ok(resolved)
}

fn check_registration(
    registration: &Registration,
    bench: &Bench,
    bytes: &[u8],
) -> Result<super::sweep::FrozenConfig, EvalError> {
    let capture = bench.captured.as_ref().ok_or_else(|| {
        EvalError::Invalid("registration requires admitted captured inputs".into())
    })?;
    if registration.version != REGISTRATION_VERSION
        || registration.frozen.as_bytes() != bytes
        || registration.identity != capture.identity
        || Some(&registration.preparation_partitions) != capture.preparation_partitions.as_ref()
        || Some(&registration.preparation_sha256) != capture.preparation_sha256.as_ref()
    {
        return Err(EvalError::Invalid(
            "registered freeze or captured inputs differ".into(),
        ));
    }
    let frozen = super::sweep::FrozenConfig::from_toml(&registration.frozen)?;
    if !frozen.preparation_problems(bench)?.is_empty()
        || frozen.config_sha256 != bench.config_sha256
        || frozen.splits_sha256 != bench.splits()?.1
        || frozen.generation_ids
            != bench
                .indexes()?
                .iter()
                .map(|(name, index)| (name.clone(), index.generation.generation_id.clone()))
                .collect()
        || (
            frozen.alpha,
            frozen.beta,
            frozen.threshold,
            frozen.seed,
            frozen.resamples,
        ) != (
            bench.config.alpha,
            bench.config.beta,
            bench.config.threshold,
            bench.config.seed,
            bench.config.resamples,
        )
    {
        return Err(EvalError::Invalid(
            "frozen benchmark identity differs".into(),
        ));
    }
    Ok(frozen)
}

fn check_code(frozen: &super::sweep::FrozenConfig, root: &Path) -> Result<(), EvalError> {
    let code = super::sweep::CodeIdentity::current(root)?;
    if code.dirty
        || code.git_commit != frozen.git_commit
        || code.cargo_lock_sha256 != frozen.cargo_lock_sha256
        || code.binary_sha256 != frozen.binary_sha256
    {
        return Err(EvalError::Invalid(
            "frozen code identity differs or checkout is dirty".into(),
        ));
    }
    Ok(())
}

/// Called only behind the universal CLI admission guard. The export is a copy;
/// the one common-directory registration is the authority, even on failure.
pub(crate) fn register_freeze(
    repo: &Path,
    bench: &Bench,
    text: &str,
    out: &Path,
) -> Result<(), EvalError> {
    if text.len() > MAX_INPUT_BYTES / 8 {
        return Err(EvalError::Invalid("frozen text byte limit exceeded".into()));
    }
    let custody = Custody::resolve(repo)?;
    let out = external_path(&custody.root, out)?;
    bench.require_prepared_capture(SplitName::Dev)?;
    let capture = bench
        .captured
        .as_ref()
        .ok_or_else(|| EvalError::Invalid("registration requires captured inputs".into()))?;
    let registration = Registration {
        version: REGISTRATION_VERSION,
        preparation_sha256: capture.preparation_sha256.clone().ok_or_else(|| {
            EvalError::Invalid("registration requires an admitted preparation".into())
        })?,
        frozen: text.into(),
        identity: capture.identity.clone(),
        preparation_partitions: capture.preparation_partitions.clone().ok_or_else(|| {
            EvalError::Invalid("registration requires preparation partitions".into())
        })?,
    };
    let frozen = check_registration(&registration, bench, text.as_bytes())?;
    check_code(&frozen, &custody.root)?;
    custody.create_dir()?;
    write_new(
        &custody.dir.join("registration.json"),
        &serde_json::to_vec(&registration)?,
    )?;
    write_new(&out, text.as_bytes())
}

pub(crate) fn check_registered_export(
    repo: &Path,
    bench: &Bench,
    bytes: &[u8],
) -> Result<super::sweep::FrozenConfig, EvalError> {
    let custody = Custody::resolve(repo)?;
    let (registration, _) = custody.registration()?;
    let frozen = check_registration(&registration, bench, bytes)?;
    check_code(&frozen, &custody.root)?;
    Ok(frozen)
}

fn check_held_out_code(bench: &Bench) -> Result<super::sweep::FrozenConfig, EvalError> {
    check_registered_export(
        Path::new("."),
        bench,
        &read(&bench.dir.join("frozen-config.toml"))?,
    )
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptStarted {
    version: u32,
    registration_sha256: String,
    frozen_sha256: String,
    output: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptResult {
    version: u32,
    registration_sha256: String,
    frozen_sha256: String,
    state: String,
    meta_sha256: Option<String>,
    report_sha256: Option<String>,
    reason: Option<String>,
}

fn start_attempt(custody: &Custody, out: &Path) -> Result<AttemptStarted, EvalError> {
    let (registration, registration_sha256) = custody.registration()?;
    // A stray or malformed terminal record consumes the slot too.
    match fs::symlink_metadata(custody.dir.join("attempt-result.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => {
            return Err(EvalError::Invalid(
                "held-out attempt is already consumed".into(),
            ));
        }
    }
    let started = AttemptStarted {
        version: 1,
        registration_sha256,
        frozen_sha256: sha256_hex(registration.frozen.as_bytes()),
        output: external_path(&custody.root, out)?,
    };
    write_new(
        &custody.dir.join("attempt-started.json"),
        &serde_json::to_vec(&started)?,
    )?;
    Ok(started)
}

fn finish_attempt(
    custody: &Custody,
    started: &AttemptStarted,
    completed: Option<(&[u8], &[u8])>,
) -> Result<(), EvalError> {
    custody.check_dir()?;
    let result = AttemptResult {
        version: 1,
        registration_sha256: started.registration_sha256.clone(),
        frozen_sha256: started.frozen_sha256.clone(),
        state: if completed.is_some() {
            "completed"
        } else {
            "failed"
        }
        .into(),
        meta_sha256: completed.map(|(meta, _)| sha256_hex(meta)),
        report_sha256: completed.map(|(_, report)| sha256_hex(report)),
        reason: completed
            .is_none()
            .then(|| "computation-or-publication-failed".into()),
    };
    write_new(
        &custody.dir.join("attempt-result.json"),
        &serde_json::to_vec(&result)?,
    )
}

fn verify_completed(
    custody: &Custody,
    run_dir: &Path,
    meta: &[u8],
    report: &[u8],
) -> Result<(), EvalError> {
    let (registration, registration_hash) = custody.registration()?;
    let started: AttemptStarted =
        serde_json::from_slice(&read(&custody.dir.join("attempt-started.json"))?)?;
    let result: AttemptResult =
        serde_json::from_slice(&read(&custody.dir.join("attempt-result.json"))?)?;
    let frozen_hash = sha256_hex(registration.frozen.as_bytes());
    let voided = !matches!(
        fs::symlink_metadata(custody.dir.join(VOID_RECORD)),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound
    );
    if voided
        || started.version != 1
        || result.version != 1
        || result.state != "completed"
        || result.reason.is_some()
        || started.registration_sha256 != registration_hash
        || result.registration_sha256 != registration_hash
        || started.frozen_sha256 != frozen_hash
        || result.frozen_sha256 != frozen_hash
        || started.output != external_path(&custody.root, run_dir)?
        || result.meta_sha256.as_deref() != Some(sha256_hex(meta).as_str())
        || result.report_sha256.as_deref() != Some(sha256_hex(report).as_str())
    {
        return Err(EvalError::Invalid(
            "saved run differs from completed held-out attempt".into(),
        ));
    }
    Ok(())
}

/// Full frozen check for a freeze already verified by [`check_held_out_code`].
fn held_out_preflight(
    bench: &Bench,
    frozen: super::sweep::FrozenConfig,
) -> Result<super::sweep::FrozenConfig, EvalError> {
    use super::sweep::{CodeIdentity, check_frozen};
    let custody = Custody::resolve(Path::new("."))?;
    check_frozen(&frozen, bench, &CodeIdentity::current(&custody.root)?)?;
    Ok(frozen)
}

fn check_score_split(split: SplitName, confirm_test: bool) -> Result<(), EvalError> {
    if split == SplitName::Test && !confirm_test {
        return Err(EvalError::TestSplitRefused);
    }
    Ok(())
}

/// Pre-reservation held-out admission from metadata only; opens no test
/// payload. Registration, export bytes, frozen manifest/partition binding,
/// power and manifest test count, clean code identity and output path are
/// all checked before the sole attempt can be consumed.
fn held_out_admission(
    config_path: &Path,
    evidence: &Preparation,
    custody: &Custody,
    out: &Path,
) -> Result<(), EvalError> {
    let (registration, _) = custody.registration()?;
    let dir = parent(config_path);
    let data = fs::canonicalize(dir).map_err(io_err(dir))?;
    if data.starts_with(&custody.root) {
        return Err(EvalError::Invalid(
            "held-out benchmark and attempt receipt must be outside the frozen code checkout"
                .into(),
        ));
    }
    external_path(&custody.root, out)?;
    match fs::symlink_metadata(out) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(EvalError::Io {
                path: out.to_path_buf(),
                source,
            });
        }
        Ok(_) => {
            return Err(EvalError::Invalid(format!(
                "refusing to overwrite run directory {}",
                out.display()
            )));
        }
    }
    let export = read_bounded(&dir.join("frozen-config.toml"), MAX_INPUT_BYTES / 8)?;
    let frozen = super::sweep::FrozenConfig::from_toml(&registration.frozen)?;
    let [dev, test] = evidence.partition_pairs();
    let generations: BTreeMap<String, String> = evidence
        .corpora
        .iter()
        .map(|c| (c.name.clone(), c.generation_id.clone()))
        .collect();
    let answerable = evidence
        .counts
        .answerable_positive
        .get("test")
        .copied()
        .unwrap_or(0);
    if export != registration.frozen.as_bytes()
        || registration.preparation_sha256 != evidence.sha256()
        || frozen.preparation_sha256 != evidence.sha256()
        || (&frozen.dev_queries_sha256, &frozen.dev_qrels_sha256) != (&dev.0, &dev.1)
        || (&frozen.test_queries_sha256, &frozen.test_qrels_sha256) != (&test.0, &test.1)
        || frozen.config_sha256 != evidence.inputs.config
        || frozen.splits_sha256 != evidence.inputs.splits
        || frozen.generation_ids != generations
        || frozen.resamples != super::sweep::REQUIRED_RESAMPLES
        || answerable < frozen.power.required_answerable_test
    {
        return Err(EvalError::Invalid(
            "frozen registration does not admit this preparation".into(),
        ));
    }
    super::sweep::validate_power(&frozen.power)?;
    check_code(&frozen, &custody.root)?;
    // Every non-held-out identity the test capture will recheck (config,
    // splits, gates, indexes, corpus pins and freshness) is verified against
    // the manifest now, through the dev-selected capture, so a stale or
    // dirty corpus refuses without consuming the attempt.
    let dev = Bench::load_selected(config_path, evidence, SplitName::Dev)?;
    // Run files must fit scoring's per-file payload bound; otherwise the
    // attempt would be consumed by an unscorable run.
    held_out_run_bound(&dev, &frozen)
}

/// Split ids are written to run files; refuse TSV-unsafe ones up front.
fn check_run_ids(ids: &[String]) -> Result<(), EvalError> {
    ids.iter().try_for_each(|id| check_tsv_field(id).map(drop))
}

/// Run files of the held-out attempt must fit scoring's payload bound. Uses
/// the test query ids of the manifest-pinned splits (metadata only, no test
/// payload), so an unscorable run refuses before the attempt is reserved.
fn held_out_run_bound(dev: &Bench, frozen: &super::sweep::FrozenConfig) -> Result<(), EvalError> {
    let (splits, _) = dev.splits()?;
    check_run_ids(&splits.test)?;
    let systems = [
        System::B0,
        System::B1,
        System::B2(frozen.alpha),
        System::B3(frozen.beta),
    ];
    check_run_file_bytes(run_file_bound(&splits.test, &*dev.indexes()?, &systems))
}

/// An owned private sibling avoids publishing partial or invalid output.
struct StagingDir(PathBuf);

impl StagingDir {
    fn create(out: &Path) -> Result<Self, EvalError> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        if out.exists() {
            return Err(EvalError::Invalid(format!(
                "refusing to overwrite run directory {}",
                out.display()
            )));
        }
        let parent = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(io_err(parent))?;
        for _ in 0..100 {
            let candidate = parent.join(format!(
                ".ontodoc-run-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&candidate) {
                Ok(()) => return Ok(Self(candidate)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => {
                    return Err(EvalError::Io {
                        path: candidate,
                        source,
                    });
                }
            }
        }
        Err(EvalError::Invalid(
            "could not reserve a run staging directory".into(),
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn publish(self, out: &Path) -> Result<(), EvalError> {
        if out.exists() {
            return Err(EvalError::Invalid(format!(
                "refusing to overwrite run directory {}",
                out.display()
            )));
        }
        for entry in fs::read_dir(&self.0).map_err(io_err(&self.0))? {
            let path = entry.map_err(io_err(&self.0))?.path();
            fs::File::open(&path)
                .and_then(|file| file.sync_all())
                .map_err(io_err(&path))?;
        }
        sync_dir(&self.0)?;
        fs::rename(&self.0, out).map_err(io_err(out))?;
        sync_dir(out)?;
        sync_dir(parent(out))
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs B0..B3 on one split, writes the run directory and returns the report.
pub fn run(
    bench: &Bench,
    split: SplitName,
    confirm_test: bool,
    out: &Path,
) -> Result<Report, EvalError> {
    if split == SplitName::Test {
        if !confirm_test {
            return Err(EvalError::TestSplitRefused);
        }
        // A held-out attempt must be reserved before its payload is read.
        return Err(EvalError::Invalid(
            "held-out runs reserve their attempt through run_held_out".into(),
        ));
    }
    bench.require_prepared_capture(SplitName::Dev)?;
    run_reserved(bench, split, confirm_test, out, None, None)
}

/// Held-out run from the pinned preparation: confirmation and admission are
/// checked first, then the sole attempt is reserved before any test payload
/// is read ([`Bench::load_test_reserved`]); every later failure consumes it.
pub fn run_held_out(
    config_path: &Path,
    preparation: &Path,
    preparation_sha256: &str,
    confirm_test: bool,
    out: &Path,
) -> Result<Report, EvalError> {
    if !confirm_test {
        return Err(EvalError::TestSplitRefused);
    }
    let evidence = Preparation::load(preparation, preparation_sha256)?;
    let custody = Custody::resolve(Path::new("."))?;
    held_out_admission(config_path, &evidence, &custody, out)?;
    // An unusable output location must fail before the sole attempt is spent.
    let stage = StagingDir::create(out)?;
    let (bench, started) = Bench::load_test_reserved(config_path, &evidence, &custody, out)?;
    run_reserved(
        &bench,
        SplitName::Test,
        confirm_test,
        out,
        Some(stage),
        Some((custody, started)),
    )
}

fn run_reserved(
    bench: &Bench,
    split: SplitName,
    confirm_test: bool,
    out: &Path,
    // Held-out runs stage before reserving their attempt; dev runs stage
    // only after their inputs and budgets are validated.
    stage: Option<StagingDir>,
    attempt: Option<(Custody, AttemptStarted)>,
) -> Result<Report, EvalError> {
    let attempt_ref = attempt.as_ref();
    let result = (move || {
        let registered = if split == SplitName::Test {
            Some(check_held_out_code(bench)?)
        } else {
            None
        };
        if split == SplitName::Test {
            let custody = Custody::resolve(Path::new("."))?;
            let data = fs::canonicalize(&bench.dir).map_err(io_err(&bench.dir))?;
            if data.starts_with(&custody.root) {
                return Err(EvalError::Invalid(
                "held-out benchmark and attempt receipt must be outside the frozen code checkout"
                    .into(),
            ));
            }
            external_path(&custody.root, out)?;
        }
        bench.require_prepared_capture(split)?;
        preflight(bench)?;
        let frozen = registered
            .map(|f| held_out_preflight(bench, f))
            .transpose()?;
        let (splits, splits_sha256) = bench.splits()?;
        let (_, qrels_sha256) = bench.qrels()?;
        let indexes = bench.indexes()?;
        let ids: BTreeSet<&str> = match split {
            SplitName::Dev => &splits.dev,
            SplitName::Test => &splits.test,
        }
        .iter()
        .map(String::as_str)
        .collect();
        let queries: Vec<&Query> = bench
            .queries
            .iter()
            .filter(|q| ids.contains(q.id.as_str()))
            .collect();
        let c = &bench.config;
        let systems = [
            System::B0,
            System::B1,
            System::B2(c.alpha),
            System::B3(c.beta),
        ];

        // Complete all ranking and scoring privately; publish only a passing report.
        check_run_file_bytes(run_file_bound(
            queries.iter().map(|q| &q.id),
            &indexes,
            &systems,
        ))?;
        let stage = match stage {
            Some(stage) => stage,
            None => StagingDir::create(out)?,
        };
        let code_identity = frozen.map(|f| (f.git_commit, f.binary_sha256));
        let stage_path = stage.path();
        let mut run_sha256 = BTreeMap::new();
        let mut run_bytes = 0usize;
        for system in systems {
            let name = system.name();
            let mut text = String::new();
            for q in &queries {
                for (rank, (section, score)) in rank_sections(&indexes[&q.corpus], system, &q.text)
                    .iter()
                    .enumerate()
                {
                    text.push_str(&format!(
                        "{}\tQ0\t{}\t{}\t{:e}\t{}\n",
                        check_tsv_field(&q.id)?,
                        check_tsv_field(section)?,
                        rank + 1,
                        score,
                        name
                    ));
                }
            }
            run_bytes = run_bytes.saturating_add(text.len());
            check_run_file_bytes(run_bytes)?;
            let file = format!("{name}.tsv");
            run_sha256.insert(file.clone(), sha256_hex(text.as_bytes()));
            write(&stage_path.join(file), text.as_bytes())?;
        }
        let mut abstention = String::new();
        for q in &queries {
            let ranked = indexes[&q.corpus].search(&q.text, &Params::new(Mode::Lexical, 1));
            abstention.push_str(&format!("{}\t{:e}\n", q.id, ranked.abstention_score));
        }
        check_run_file_bytes(run_bytes.saturating_add(abstention.len()))?;
        write(&stage_path.join("abstention.tsv"), abstention.as_bytes())?;
        run_sha256.insert("abstention.tsv".into(), sha256_hex(abstention.as_bytes()));

        let (_, preparation_sha256, _) = bench
            .preparation_binding()
            .ok_or_else(|| EvalError::Invalid("run requires an admitted preparation".into()))?;
        let meta = RunMeta {
            version: RUN_META_VERSION,
            split,
            preparation_sha256: preparation_sha256.into(),
            seed: c.seed,
            resamples: c.resamples,
            alpha: c.alpha,
            beta: c.beta,
            threshold: c.threshold,
            config_sha256: bench.config_sha256.clone(),
            queries_sha256: bench.queries_sha256.clone(),
            qrels_sha256,
            splits_sha256,
            corpora: indexes
                .iter()
                .map(|(name, index)| CorpusIdentity {
                    name: name.clone(),
                    generation_id: index.generation.generation_id.clone(),
                })
                .collect(),
            git_commit: code_identity
                .as_ref()
                .map(|v| v.0.clone())
                .or_else(|| git(&["rev-parse", "HEAD"])),
            git_dirty: if code_identity.is_some() {
                Some(false)
            } else {
                git(&["status", "--porcelain"]).map(|s| !s.is_empty())
            },
            binary_sha256: code_identity.map(|v| v.1).or_else(binary_sha256),
            systems: systems.iter().map(|s| s.name()).collect(),
            run_sha256,
        };
        let meta_bytes = serde_json::to_vec_pretty(&meta)?;
        write(&stage_path.join("meta.json"), &meta_bytes)?;
        let report = score_captured(bench, stage_path, confirm_test, meta)?;
        let failures = super::gates::check_judged(&report);
        if !failures.is_empty() {
            return Err(EvalError::Invalid(format!(
                "judged@10 gate failed: {}",
                serde_json::to_string(&failures)?
            )));
        }
        if split == SplitName::Test
            && report
                .comparisons
                .first()
                .is_none_or(|c| c.decision != Some(Decision::Pass))
        {
            return Err(EvalError::Invalid(
                "H1 sanity gate failed; held-out comparisons are not accepted".into(),
            ));
        }
        let report_bytes = serde_json::to_vec_pretty(&report)?;
        publish_run(stage, out, &meta_bytes, &report_bytes, attempt_ref)?;
        Ok(report)
    })();
    consume_on_error(attempt.as_ref(), result)
}

/// Publishes a fully computed run, rereads the published bytes and only then
/// records completion. With [`consume_on_error`], a failure leaves output
/// unpublished, visible without completed custody, or voided — unless the
/// void marker write also fails (reported; a double filesystem fault).
fn publish_run(
    stage: StagingDir,
    out: &Path,
    meta: &[u8],
    report: &[u8],
    attempt: Option<&(Custody, AttemptStarted)>,
) -> Result<(), EvalError> {
    write(&stage.path().join("report.json"), report)?;
    stage.publish(out)?;
    if read(&out.join("meta.json"))? != meta || read(&out.join("report.json"))? != report {
        return Err(EvalError::Invalid(
            "published run differs from computed bytes".into(),
        ));
    }
    if let Some((custody, started)) = attempt {
        finish_attempt(custody, started, Some((meta, report)))?;
    }
    Ok(())
}

/// Records a failed, consumed attempt for any held-out error. An occupied
/// terminal slot is never replaced: it may hold a readable `completed` record
/// whose fsync failed. A create-new void marker then makes that record
/// unacceptable; [`verify_completed`] refuses whenever the marker exists.
fn consume_on_error<T>(
    attempt: Option<&(Custody, AttemptStarted)>,
    result: Result<T, EvalError>,
) -> Result<T, EvalError> {
    match (result, attempt) {
        (Err(error), Some((custody, started))) => match finish_attempt(custody, started, None) {
            Ok(()) => Err(error),
            Err(record) => {
                let voided = write_new(
                    &custody.dir.join(VOID_RECORD),
                    started.registration_sha256.as_bytes(),
                )
                .err()
                .map_or_else(String::new, |e| format!("; void marker not written: {e}"));
                Err(EvalError::Invalid(format!(
                    "{error}; failed-attempt record not written: {record}{voided}"
                )))
            }
        },
        (result, _) => result,
    }
}

/// Presence (in any form) voids a terminal record left by a failed completion.
const VOID_RECORD: &str = "attempt-voided.json";

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Metrics {
    pub ndcg10: f64,
    pub condensed_ndcg10: f64,
    pub recall10: f64,
    pub mrr10: f64,
    pub judged10: f64,
}

impl Metrics {
    fn of(qrels: &Qrels, ranking: &[&str]) -> Result<Self, metrics::MetricError> {
        Ok(Self {
            ndcg10: metrics::ndcg_at(qrels, ranking, CUTOFF)?,
            condensed_ndcg10: metrics::condensed_ndcg_at(qrels, ranking, CUTOFF)?,
            recall10: metrics::recall_at(qrels, ranking, CUTOFF)?,
            mrr10: metrics::mrr_at(qrels, ranking, CUTOFF)?,
            judged10: metrics::judged_at(qrels, ranking, CUTOFF),
        })
    }

    fn mean<'a>(items: impl Iterator<Item = &'a Metrics>) -> Self {
        let mut sum = Metrics::default();
        let mut n = 0.0;
        for m in items {
            sum.ndcg10 += m.ndcg10;
            sum.condensed_ndcg10 += m.condensed_ndcg10;
            sum.recall10 += m.recall10;
            sum.mrr10 += m.mrr10;
            sum.judged10 += m.judged10;
            n += 1.0;
        }
        if n > 0.0 {
            sum.ndcg10 /= n;
            sum.condensed_ndcg10 /= n;
            sum.recall10 /= n;
            sum.mrr10 /= n;
            sum.judged10 /= n;
        }
        sum
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSummary {
    pub mean: Metrics,
    pub per_corpus: BTreeMap<String, Metrics>,
    pub per_stratum: BTreeMap<String, Metrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
    pub id: String,
    pub role: String,
    pub treatment: String,
    pub baseline: String,
    pub metric: String,
    pub stratum: Option<String>,
    pub n: usize,
    /// `None` (serialized `null`) when `n = 0`: no statistic exists.
    pub mean: Option<f64>,
    pub ci_lo: Option<f64>,
    pub ci_hi: Option<f64>,
    pub p: Option<f64>,
    pub holm_p: Option<f64>,
    pub decision: Option<Decision>,
}

impl Comparison {
    /// Statistics of one contrast; an empty group reports them unavailable
    /// instead of the bootstrap's zero placeholders.
    fn stats(&mut self, n: usize, ci: Interval, p: f64) {
        self.n = n;
        let some = |v: f64| (n > 0).then_some(v);
        self.mean = some(ci.mean);
        self.ci_lo = some(ci.lo);
        self.ci_hi = some(ci.hi);
        self.p = some(p);
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Rate {
    pub count: usize,
    pub n: usize,
    pub rate: f64,
    pub wilson_lo: f64,
    pub wilson_hi: f64,
}

impl Rate {
    fn new(count: usize, n: usize) -> Self {
        let (wilson_lo, wilson_hi) = stats::wilson(count, n, Z95);
        Self {
            count,
            n,
            rate: if n == 0 { 0.0 } else { count as f64 / n as f64 },
            wilson_lo,
            wilson_hi,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AbstentionReport {
    pub threshold: f64,
    /// Off-topic queries that abstained.
    pub correct_abstention: Rate,
    /// Answerable queries that abstained.
    pub false_abstention: Rate,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub meta: RunMeta,
    pub answerable: usize,
    pub off_topic: usize,
    /// Non-off-topic queries without any relevant judgement; not scored.
    pub unanswerable: Vec<String>,
    pub model_drafted: usize,
    /// `model-judged` when labels come from model judges (ADR revision 13):
    /// every H2/H5 contrast is then exploratory, without Holm or decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label_provenance: Option<String>,
    pub systems: BTreeMap<String, SystemSummary>,
    pub comparisons: Vec<Comparison>,
    /// Every pre-registered contrast (H1, H2, H5 and E1) recomputed within
    /// each corpus (ADR: "reported pooled and per corpus"). Exploratory:
    /// bootstrap CI and raw p only, never in the Holm family, no decision.
    pub per_corpus_comparisons: Vec<Comparison>,
    pub abstention: AbstentionReport,
    pub per_query: BTreeMap<String, BTreeMap<String, Metrics>>,
}

/// One pre-registered comparison.
struct Spec<'a> {
    id: &'static str,
    role: &'static str,
    treatment: &'a String,
    baseline: &'a String,
    metric: &'static str,
    value: fn(&Metrics) -> f64,
    stratum: Option<Stratum>,
}

/// Per-query metric values, keyed by query id.
pub type PerQuery = BTreeMap<String, f64>;

/// Per-query values of one system's metric, optionally within one stratum.
type MetricFn<'a> = dyn Fn(&str, fn(&Metrics) -> f64, Option<Stratum>) -> PerQuery + 'a;

/// Exploratory per-corpus copies of `specs`, in sorted corpus then spec
/// order. Each keeps its stratum filter; none joins Holm or gets a decision.
fn per_corpus_comparisons(
    specs: &[Spec<'_>],
    metric: &MetricFn<'_>,
    corpus_of: &BTreeMap<String, String>,
    resamples: usize,
    seed: u64,
) -> Vec<Comparison> {
    let corpora: BTreeSet<&str> = corpus_of.values().map(String::as_str).collect();
    let mut out = Vec::new();
    for corpus in corpora {
        let only = |m: PerQuery| -> PerQuery {
            m.into_iter()
                .filter(|(qid, _)| corpus_of.get(qid).is_some_and(|c| c == corpus))
                .collect()
        };
        for sp in specs {
            let (n, ci, p) = compare(
                &only(metric(sp.treatment, sp.value, sp.stratum)),
                &only(metric(sp.baseline, sp.value, sp.stratum)),
                corpus_of,
                resamples,
                seed,
            );
            let mut c = Comparison {
                id: format!("{corpus}/{}", sp.id),
                role: "exploratory".into(),
                treatment: sp.treatment.clone(),
                baseline: sp.baseline.clone(),
                metric: sp.metric.into(),
                stratum: sp.stratum.map(|s| s.name().to_string()),
                n: 0,
                mean: None,
                ci_lo: None,
                ci_hi: None,
                p: None,
                holm_p: None,
                decision: None,
            };
            c.stats(n, ci, p);
            out.push(c);
        }
    }
    out
}

/// ADR revision 13: with model-judged labels, every contrast except the H1
/// pipeline sanity check is exploratory, with no Holm p and no decision.
fn model_judged_exploratory(comparisons: &mut [Comparison]) {
    for c in comparisons.iter_mut().filter(|c| c.role != "sanity") {
        c.role = "exploratory".into();
        c.holm_p = None;
        c.decision = None;
    }
}

/// Paired comparison of `treatment - baseline` on queries present in both,
/// bootstrapped within corpora.
pub fn compare(
    treatment: &PerQuery,
    baseline: &PerQuery,
    corpus_of: &BTreeMap<String, String>,
    resamples: usize,
    seed: u64,
) -> (usize, Interval, f64) {
    let mut strata: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    let mut all = Vec::new();
    for (qid, t) in treatment {
        if let Some(b) = baseline.get(qid) {
            let corpus = corpus_of.get(qid).map_or("", String::as_str);
            strata.entry(corpus).or_default().push(t - b);
            all.push(t - b);
        }
    }
    let slices: Vec<&[f64]> = strata.values().map(Vec::as_slice).collect();
    let ci = stats::paired_bootstrap(&slices, resamples, seed);
    let p = stats::randomization_test(&all, resamples, seed);
    (all.len(), ci, p)
}

fn parse_run(
    path: &Path,
    bytes: &[u8],
    system: &str,
) -> Result<BTreeMap<String, Vec<String>>, EvalError> {
    let text = std::str::from_utf8(bytes).map_err(|e| EvalError::Invalid(e.to_string()))?;
    let mut out: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        let parsed = match fields.as_slice() {
            [qid, "Q0", section, rank, _score, name] if *name == system => {
                rank.parse::<usize>().ok().map(|r| (*qid, r, *section))
            }
            _ => None,
        };
        let Some((qid, rank, section)) = parsed else {
            return Err(EvalError::Parse {
                path: path.to_path_buf(),
                line: i + 1,
                message: "expected `qid Q0 section rank score system`".into(),
            });
        };
        out.entry(qid.to_string())
            .or_default()
            .push((rank, section.to_string()));
    }
    Ok(out
        .into_iter()
        .map(|(q, mut v)| {
            v.sort();
            (q, v.into_iter().map(|(_, s)| s).collect())
        })
        .collect())
}

/// Scores a saved run directory. Same inputs give a byte-identical report.
pub fn score(bench: &Bench, run_dir: &Path, confirm_test: bool) -> Result<Report, EvalError> {
    // Even metadata may expose held-out observations. Refuse before opening it.
    if bench.preparation_binding().is_none() {
        return Err(admission_error());
    }
    let meta_path = run_dir.join("meta.json");
    let meta_bytes = read(&meta_path)?;
    let held_out = bench
        .captured
        .as_ref()
        .is_some_and(|c| c.selected == Some(SplitName::Test));
    if held_out {
        let custody = Custody::resolve(Path::new("."))?;
        verify_completed(
            &custody,
            run_dir,
            &meta_bytes,
            &read(&run_dir.join("report.json"))?,
        )?;
    }
    let meta: RunMeta = serde_json::from_slice(&meta_bytes)?;
    if meta.split == SplitName::Test && !held_out {
        return Err(EvalError::TestSplitRefused);
    }
    check_score_split(meta.split, confirm_test)?;
    let registered = if meta.split == SplitName::Test {
        Some(check_held_out_code(bench)?)
    } else {
        None
    };
    preflight(bench)?;
    if let Some(registered) = registered {
        let frozen = held_out_preflight(bench, registered)?;
        if meta.git_commit.as_deref() != Some(&frozen.git_commit)
            || meta.binary_sha256.as_deref() != Some(&frozen.binary_sha256)
            || meta.git_dirty != Some(false)
        {
            return Err(EvalError::Invalid(
                "run code identity differs from frozen identity".into(),
            ));
        }
    }
    let report = score_captured(bench, run_dir, confirm_test, meta.clone())?;
    let failures = super::gates::check_judged(&report);
    if !failures.is_empty() {
        return Err(EvalError::Invalid(format!(
            "judged@10 gate failed: {}",
            serde_json::to_string(&failures)?
        )));
    }
    if meta.split == SplitName::Test
        && report
            .comparisons
            .first()
            .is_none_or(|c| c.decision != Some(Decision::Pass))
    {
        return Err(EvalError::Invalid(
            "H1 sanity gate failed; held-out comparisons are not accepted".into(),
        ));
    }
    Ok(report)
}

/// Saved held-out scoring: completed custody is verified against the exact
/// saved metadata/report bytes and the pinned preparation before any test
/// payload is captured.
pub fn score_held_out(
    config_path: &Path,
    preparation: &Path,
    preparation_sha256: &str,
    confirm_test: bool,
    run_dir: &Path,
) -> Result<Report, EvalError> {
    if !confirm_test {
        return Err(EvalError::TestSplitRefused);
    }
    let evidence = Preparation::load(preparation, preparation_sha256)?;
    let custody = Custody::resolve(Path::new("."))?;
    let (registration, _) = custody.registration()?;
    if registration.preparation_sha256 != evidence.sha256() {
        return Err(EvalError::Invalid(
            "registration does not bind this preparation".into(),
        ));
    }
    verify_completed(
        &custody,
        run_dir,
        &read(&run_dir.join("meta.json"))?,
        &read(&run_dir.join("report.json"))?,
    )?;
    let bench = Bench::load_selected(config_path, &evidence, SplitName::Test)?;
    score(&bench, run_dir, true)
}

fn score_captured(
    bench: &Bench,
    run_dir: &Path,
    confirm_test: bool,
    meta: RunMeta,
) -> Result<Report, EvalError> {
    bench.require_prepared_capture(meta.split)?;
    check_score_split(meta.split, confirm_test)?;
    if meta.version != RUN_META_VERSION
        || bench
            .preparation_binding()
            .is_none_or(|(_, sha, _)| sha != meta.preparation_sha256)
    {
        return Err(EvalError::Invalid(
            "run metadata version or preparation differs".into(),
        ));
    }
    let (qrels, qrels_sha256) = bench.qrels()?;
    if qrels_sha256 != meta.qrels_sha256 {
        return Err(EvalError::Invalid("qrels changed since the run".into()));
    }
    let (splits, splits_sha256) = bench.splits()?;
    let c = &bench.config;
    if meta.config_sha256 != bench.config_sha256
        || meta.queries_sha256 != bench.queries_sha256
        || meta.splits_sha256 != splits_sha256
        || (
            meta.seed,
            meta.resamples,
            meta.alpha,
            meta.beta,
            meta.threshold,
        ) != (c.seed, c.resamples, c.alpha, c.beta, c.threshold)
    {
        return Err(EvalError::Invalid(
            "benchmark inputs changed since the run".into(),
        ));
    }
    let indexes = bench.indexes()?;
    let corpora: Vec<CorpusIdentity> = indexes
        .iter()
        .map(|(name, index)| CorpusIdentity {
            name: name.clone(),
            generation_id: index.generation.generation_id.clone(),
        })
        .collect();
    if meta.corpora != corpora {
        return Err(EvalError::Invalid(
            "corpus generations changed since the run".into(),
        ));
    }
    let expected_systems = [
        System::B0,
        System::B1,
        System::B2(c.alpha),
        System::B3(c.beta),
    ]
    .into_iter()
    .map(System::name)
    .collect::<Vec<_>>();
    if meta.systems != expected_systems {
        return Err(EvalError::Invalid(
            "run system identities or ordering differ".into(),
        ));
    }
    let expected_files: BTreeSet<String> = expected_systems
        .iter()
        .map(|name| format!("{name}.tsv"))
        .chain(std::iter::once("abstention.tsv".into()))
        .collect();
    if meta.run_sha256.keys().cloned().collect::<BTreeSet<_>>() != expected_files {
        return Err(EvalError::Invalid(
            "run file identity manifest is incomplete".into(),
        ));
    }
    let mut run_bytes = BTreeMap::new();
    // All run files together are one bounded payload (checked when written).
    let mut budget = MAX_INPUT_BYTES;
    for name in &expected_files {
        let bytes = captured_read(&run_dir.join(name), MAX_INPUT_BYTES, &mut budget)?;
        if meta.run_sha256.get(name) != Some(&sha256_hex(&bytes)) {
            return Err(EvalError::Invalid(format!(
                "run file {name} changed since the run"
            )));
        }
        run_bytes.insert(name.clone(), bytes);
    }
    let ids: BTreeSet<&str> = match meta.split {
        SplitName::Dev => &splits.dev,
        SplitName::Test => &splits.test,
    }
    .iter()
    .map(String::as_str)
    .collect();
    let queries: Vec<&Query> = bench
        .queries
        .iter()
        .filter(|q| ids.contains(q.id.as_str()))
        .collect();
    let empty = Qrels::new();
    let answerable: Vec<&Query> = queries
        .iter()
        .copied()
        .filter(|q| q.stratum != Stratum::OffTopic)
        .filter(|q| {
            qrels
                .get(&q.id)
                .is_some_and(|r| r.values().any(|&g| g >= 1))
        })
        .collect();
    let unanswerable: Vec<String> = queries
        .iter()
        .filter(|q| q.stratum != Stratum::OffTopic)
        .filter(|q| !answerable.iter().any(|a| a.id == q.id))
        .map(|q| q.id.clone())
        .collect();

    let mut per_query: BTreeMap<String, BTreeMap<String, Metrics>> = BTreeMap::new();
    let mut systems = BTreeMap::new();
    for name in &meta.systems {
        let file = format!("{name}.tsv");
        let runs = parse_run(&run_dir.join(&file), &run_bytes[&file], name)?;
        let mut by_query: BTreeMap<&str, Metrics> = BTreeMap::new();
        for q in &answerable {
            let ranking: Vec<&str> = runs
                .get(&q.id)
                .map(|v| v.iter().map(String::as_str).collect())
                .unwrap_or_default();
            let m = Metrics::of(qrels.get(&q.id).unwrap_or(&empty), &ranking)
                .map_err(|e| EvalError::Invalid(format!("{} {name}: {e}", q.id)))?;
            by_query.insert(q.id.as_str(), m);
            per_query
                .entry(q.id.clone())
                .or_default()
                .insert(name.clone(), m);
        }
        let group = |key: &dyn Fn(&Query) -> String| -> BTreeMap<String, Metrics> {
            let mut groups: BTreeMap<String, Vec<&Metrics>> = BTreeMap::new();
            for q in &answerable {
                if let Some(m) = by_query.get(q.id.as_str()) {
                    groups.entry(key(q)).or_default().push(m);
                }
            }
            groups
                .into_iter()
                .map(|(k, v)| (k, Metrics::mean(v.into_iter())))
                .collect()
        };
        systems.insert(
            name.clone(),
            SystemSummary {
                mean: Metrics::mean(by_query.values()),
                per_corpus: group(&|q| q.corpus.clone()),
                per_stratum: group(&|q| q.stratum.name().to_string()),
            },
        );
    }

    let corpus_of: BTreeMap<String, String> = queries
        .iter()
        .map(|q| (q.id.clone(), q.corpus.clone()))
        .collect();
    let stratum_of: BTreeMap<String, Stratum> =
        queries.iter().map(|q| (q.id.clone(), q.stratum)).collect();
    let metric = |system: &str, f: fn(&Metrics) -> f64, stratum: Option<Stratum>| -> PerQuery {
        per_query
            .iter()
            .filter(|(qid, _)| stratum.is_none_or(|s| stratum_of.get(*qid) == Some(&s)))
            .filter_map(|(qid, m)| m.get(system).map(|m| (qid.clone(), f(m))))
            .collect()
    };
    let [b0, b1, b2, b3] = meta.systems.as_slice() else {
        return Err(EvalError::Invalid(
            "run must contain exactly B0, B1, B2, B3".into(),
        ));
    };
    let ndcg: fn(&Metrics) -> f64 = |m| m.ndcg10;
    let recall: fn(&Metrics) -> f64 = |m| m.recall10;
    let mrr: fn(&Metrics) -> f64 = |m| m.mrr10;
    let spec = |id, role, treatment, baseline, metric, value, stratum| Spec {
        id,
        role,
        treatment,
        baseline,
        metric,
        value,
        stratum,
    };
    let specs = [
        spec("H1", "sanity", b1, b0, "ndcg10", ndcg, None),
        spec("H2", "confirmatory", b2, b1, "ndcg10", ndcg, None),
        spec("H5", "confirmatory", b3, b1, "ndcg10", ndcg, None),
        spec(
            "E1-B2-multi-hop-recall",
            "exploratory",
            b2,
            b1,
            "recall10",
            recall,
            Some(Stratum::MultiHop),
        ),
        spec(
            "E1-B2-multi-hop-mrr",
            "exploratory",
            b2,
            b1,
            "mrr10",
            mrr,
            Some(Stratum::MultiHop),
        ),
        spec(
            "E1-B3-conceptual-recall",
            "exploratory",
            b3,
            b1,
            "recall10",
            recall,
            Some(Stratum::Conceptual),
        ),
        spec(
            "E1-B3-conceptual-mrr",
            "exploratory",
            b3,
            b1,
            "mrr10",
            mrr,
            Some(Stratum::Conceptual),
        ),
    ];
    let mut comparisons: Vec<Comparison> = specs
        .iter()
        .map(|sp| {
            let (n, ci, p) = compare(
                &metric(sp.treatment, sp.value, sp.stratum),
                &metric(sp.baseline, sp.value, sp.stratum),
                &corpus_of,
                meta.resamples,
                meta.seed,
            );
            let mut c = Comparison {
                id: sp.id.into(),
                role: sp.role.into(),
                treatment: sp.treatment.clone(),
                baseline: sp.baseline.clone(),
                metric: sp.metric.into(),
                stratum: sp.stratum.map(|s| s.name().to_string()),
                n: 0,
                mean: None,
                ci_lo: None,
                ci_hi: None,
                p: None,
                holm_p: None,
                decision: None,
            };
            c.stats(n, ci, p);
            c
        })
        .collect();
    let per_corpus_comparisons =
        per_corpus_comparisons(&specs, &metric, &corpus_of, meta.resamples, meta.seed);
    // A missing p (n = 0) enters Holm as 1, the least significant value.
    let p_or_one = |c: &Comparison| c.p.unwrap_or(1.0);
    let holm = stats::holm(&[p_or_one(&comparisons[1]), p_or_one(&comparisons[2])]);
    let h0 = &comparisons[0];
    let h1_passed = h0.n > 0
        && h0.mean.is_some_and(|m| m > 0.0)
        && h0.ci_lo.is_some_and(|lo| lo > 0.0)
        && h0.p.is_some_and(|p| p < ALPHA);
    for (i, c) in comparisons.iter_mut().enumerate().take(3) {
        let p = if i == 0 { p_or_one(c) } else { holm[i - 1] };
        if i > 0 && h1_passed {
            c.holm_p = Some(p);
        }
        if i > 0 && !h1_passed {
            continue;
        }
        let ci = match (c.mean, c.ci_lo, c.ci_hi) {
            (Some(mean), Some(lo), Some(hi)) => Some(Interval { mean, lo, hi }),
            _ => None,
        };
        c.decision = Some(match ci {
            Some(ci) if i == 0 || h1_passed => stats::decide(ci, p, ALPHA, MIN_EFFECT),
            _ => Decision::Inconclusive,
        });
    }
    let label_provenance = bench
        .captured
        .as_ref()
        .and_then(|c| c.label_provenance.clone());
    if label_provenance.is_some() {
        model_judged_exploratory(&mut comparisons);
    }

    let s_path = run_dir.join("abstention.tsv");
    let s_text =
        std::str::from_utf8(&run_bytes["abstention.tsv"]).map_err(|e| EvalError::Parse {
            path: s_path.clone(),
            line: 0,
            message: e.to_string(),
        })?;
    let mut s_of: BTreeMap<&str, f64> = BTreeMap::new();
    for (i, line) in s_text.lines().enumerate() {
        let parsed = line
            .split_once('\t')
            .and_then(|(q, s)| s.parse::<f64>().ok().map(|s| (q, s)));
        let Some((q, s)) = parsed else {
            return Err(EvalError::Parse {
                path: s_path,
                line: i + 1,
                message: "expected `qid score`".into(),
            });
        };
        if !ids.contains(q) || !s.is_finite() || s_of.insert(q, s).is_some() {
            return Err(EvalError::Invalid(
                "invalid or duplicate abstention score".into(),
            ));
        }
    }
    if s_of.len() != ids.len() {
        return Err(EvalError::Invalid("missing abstention scores".into()));
    }
    let abstains = |q: &Query| s_of.get(q.id.as_str()).is_some_and(|&s| s < meta.threshold);
    let off_topic: Vec<&Query> = queries
        .iter()
        .copied()
        .filter(|q| q.stratum == Stratum::OffTopic)
        .collect();
    let abstention = AbstentionReport {
        threshold: meta.threshold,
        correct_abstention: Rate::new(
            off_topic.iter().filter(|q| abstains(q)).count(),
            off_topic.len(),
        ),
        false_abstention: Rate::new(
            answerable.iter().filter(|q| abstains(q)).count(),
            answerable.len(),
        ),
    };

    Ok(Report {
        answerable: answerable.len(),
        off_topic: off_topic.len(),
        unanswerable,
        model_drafted: queries.iter().filter(|q| q.model_drafted).count(),
        label_provenance,
        systems,
        comparisons,
        per_corpus_comparisons,
        abstention,
        per_query,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn custody_fixture() -> (tempfile::TempDir, Custody) {
        let dir = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(dir.path()).unwrap();
        let custody = Custody {
            root: base.join("code"),
            dir: base.join("custody"),
        };
        fs::create_dir(&custody.root).unwrap();
        custody.create_dir().unwrap();
        let registration = Registration {
            version: REGISTRATION_VERSION,
            preparation_sha256: "preparation".into(),
            frozen: "synthetic frozen bytes\n".into(),
            identity: InputIdentity {
                config: "config".into(),
                gates: "gates".into(),
                splits: "splits".into(),
                pool: "pool".into(),
                indexes: BTreeMap::new(),
            },
            preparation_partitions: std::array::from_fn(|_| ("queries".into(), "qrels".into())),
        };
        write_new(
            &custody.dir.join("registration.json"),
            &serde_json::to_vec(&registration).unwrap(),
        )
        .unwrap();
        (dir, custody)
    }

    #[test]
    fn model_judged_labels_downgrade_all_but_the_sanity_check() {
        let c = |id: &str, role: &str, decision| Comparison {
            id: id.into(),
            role: role.into(),
            treatment: "B2@0.2".into(),
            baseline: "B1".into(),
            metric: "ndcg10".into(),
            stratum: None,
            n: 3,
            mean: Some(0.1),
            ci_lo: Some(0.05),
            ci_hi: Some(0.2),
            p: Some(0.01),
            holm_p: Some(0.02),
            decision,
        };
        let mut all = vec![
            Comparison {
                holm_p: None,
                ..c("H1", "sanity", Some(Decision::Pass))
            },
            c("H2", "confirmatory", Some(Decision::Pass)),
            c("H5", "confirmatory", Some(Decision::Negative)),
            c("E1", "exploratory", None),
        ];
        model_judged_exploratory(&mut all);
        assert_eq!(all[0].role, "sanity");
        assert_eq!(all[0].decision, Some(Decision::Pass));
        assert_eq!(all[0].holm_p, None);
        for c in &all[1..] {
            assert_eq!(c.role, "exploratory", "{}", c.id);
            assert_eq!((c.holm_p, c.decision), (None, None), "{}", c.id);
            // Effect estimates stay reported.
            assert_eq!((c.mean, c.ci_lo, c.p), (Some(0.1), Some(0.05), Some(0.01)));
        }
    }

    #[test]
    fn linked_worktrees_share_fixed_registration_slot() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir(&repo).unwrap();
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args([
                    "-c",
                    "user.name=fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "-q"]);
        git(&["commit", "--allow-empty", "-qm", "fixture"]);
        let linked = dir.path().join("linked");
        git(&["worktree", "add", "--detach", linked.to_str().unwrap()]);
        let primary = Custody::resolve(&repo).unwrap();
        let secondary = Custody::resolve(&linked).unwrap();
        assert_eq!(primary.dir, secondary.dir);
        assert_ne!(primary.root, secondary.root);
        fs::create_dir(repo.join("nested")).unwrap();
        assert_eq!(
            Custody::resolve(&repo.join("nested")).unwrap().root,
            primary.root
        );
    }

    #[test]
    fn exclusive_attempt_survives_interruption_failure_and_changed_output() {
        let (dir, custody) = custody_fixture();
        let gate = Arc::new(std::sync::Barrier::new(2));
        let wins = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..2)
                .map(|n| {
                    let gate = gate.clone();
                    let custody = &custody;
                    let out = dir.path().join(format!("output-{n}"));
                    scope.spawn(move || {
                        gate.wait();
                        start_attempt(custody, &out).is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .filter(|won| *won)
                .count()
        });
        assert_eq!(wins, 1);
        assert!(start_attempt(&custody, &dir.path().join("moved-benchmark-output")).is_err());
        let started: AttemptStarted =
            serde_json::from_slice(&read(&custody.dir.join("attempt-started.json")).unwrap())
                .unwrap();
        assert!(verify_completed(&custody, &started.output, b"meta", b"report").is_err());
        finish_attempt(&custody, &started, None).unwrap();
        assert!(verify_completed(&custody, &started.output, b"meta", b"report").is_err());
        assert!(finish_attempt(&custody, &started, Some((b"meta", b"report"))).is_err());
        assert!(start_attempt(&custody, &dir.path().join("third-output")).is_err());
    }

    #[test]
    fn completed_attempt_binds_raw_metadata_report_and_registration() {
        let (dir, custody) = custody_fixture();
        let output = dir.path().join("run");
        let started = start_attempt(&custody, &output).unwrap();
        let meta = br#"{"run_sha256":{"B1.tsv":"original-ranking"}}"#;
        let report = b"original report\n";
        finish_attempt(&custody, &started, Some((meta, report))).unwrap();
        verify_completed(&custody, &output, meta, report).unwrap();
        assert!(
            verify_completed(
                &custody,
                &output,
                b" {\"run_sha256\":{\"B1.tsv\":\"original-ranking\"}}",
                report
            )
            .is_err()
        );
        // Rehashing changed rankings inside metadata cannot change the terminal commitment.
        assert!(
            verify_completed(
                &custody,
                &output,
                br#"{"run_sha256":{"B1.tsv":"changed-ranking"}}"#,
                report
            )
            .is_err()
        );
        assert!(verify_completed(&custody, &output, meta, b"original report\n\n").is_err());
        assert!(verify_completed(&custody, &dir.path().join("copied-run"), meta, report).is_err());
        let registration = custody.dir.join("registration.json");
        let mut bytes = read(&registration).unwrap();
        bytes.push(b' ');
        fs::write(&registration, bytes).unwrap();
        assert!(verify_completed(&custody, &output, meta, report).is_err());
        assert!(write_new(&registration, b"replacement").is_err());
    }

    /// Two synthetic corpora with different effects. Hand-computed: corpus
    /// "a" has q1,q2 (multi-hop) with B2-B1 = +0.5, +0.5 so mean 0.5 and a
    /// degenerate CI [0.5, 0.5]; corpus "b" has q3 (conceptual) with
    /// B2-B1 = -0.25. Pooled B2-B1 over q1..q3 = 0.75/3 = 0.25.
    #[test]
    fn per_corpus_comparisons_do_not_mix_corpora_or_join_family() {
        let (b1, b2) = ("B1".to_string(), "B2".to_string());
        let values: BTreeMap<(&str, &str), f64> = BTreeMap::from([
            (("q1", "B1"), 0.25),
            (("q1", "B2"), 0.75),
            (("q2", "B1"), 0.0),
            (("q2", "B2"), 0.5),
            (("q3", "B1"), 0.5),
            (("q3", "B2"), 0.25),
        ]);
        let corpus_of: BTreeMap<String, String> = [("q1", "a"), ("q2", "a"), ("q3", "b")]
            .into_iter()
            .map(|(q, c)| (q.into(), c.into()))
            .collect();
        let stratum_of = |q: &str| {
            if q == "q3" {
                Stratum::Conceptual
            } else {
                Stratum::MultiHop
            }
        };
        let metric = |system: &str, _: fn(&Metrics) -> f64, stratum: Option<Stratum>| {
            values
                .iter()
                .filter(|((q, s), _)| *s == system && stratum.is_none_or(|x| stratum_of(q) == x))
                .map(|((q, _), v)| (q.to_string(), *v))
                .collect::<PerQuery>()
        };
        let ndcg: fn(&Metrics) -> f64 = |m| m.ndcg10;
        let specs = [
            Spec {
                id: "H2",
                role: "confirmatory",
                treatment: &b2,
                baseline: &b1,
                metric: "ndcg10",
                value: ndcg,
                stratum: None,
            },
            Spec {
                id: "E1",
                role: "exploratory",
                treatment: &b2,
                baseline: &b1,
                metric: "recall10",
                value: ndcg,
                stratum: Some(Stratum::MultiHop),
            },
        ];
        let out = per_corpus_comparisons(&specs, &metric, &corpus_of, 50, 7);
        let ids: Vec<&str> = out.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["a/H2", "a/E1", "b/H2", "b/E1"]);
        assert_eq!(
            (out[0].n, out[0].mean, out[0].ci_lo, out[0].ci_hi),
            (2, Some(0.5), Some(0.5), Some(0.5))
        );
        assert_eq!((out[2].n, out[2].mean), (1, Some(-0.25)));
        // The stratum filter is kept: corpus b has no multi-hop query.
        assert_eq!((out[1].n, out[3].n), (2, 0));
        // An empty group has no statistics: null, never fabricated zeros.
        let empty = serde_json::to_value(&out[3]).unwrap();
        for field in ["mean", "ci_lo", "ci_hi", "p"] {
            assert!(empty[field].is_null(), "{field}: {empty}");
        }
        assert!(out[1].p.is_some());
        assert_eq!(out[1].stratum.as_deref(), Some("multi-hop"));
        assert!(
            out.iter()
                .all(|c| c.role == "exploratory" && c.holm_p.is_none() && c.decision.is_none())
        );
        // Pooled contrast differs from either corpus: no mixing above.
        let (n, ci, _) = compare(
            &metric("B2", ndcg, None),
            &metric("B1", ndcg, None),
            &corpus_of,
            50,
            7,
        );
        assert_eq!((n, ci.mean), (3, 0.25));
        // Deterministic: identical inputs give identical serialized output.
        let again = per_corpus_comparisons(&specs, &metric, &corpus_of, 50, 7);
        assert_eq!(
            serde_json::to_string(&out).unwrap(),
            serde_json::to_string(&again).unwrap()
        );
    }

    /// Deterministic failure injection through real filesystem states.
    #[test]
    fn publication_or_completion_failure_leaves_consumed_unaccepted_attempt() {
        let (meta, report) = (b"meta bytes\n".as_slice(), b"report bytes\n".as_slice());
        let staged = |out: &Path| {
            let stage = StagingDir::create(out).unwrap();
            fs::write(stage.path().join("meta.json"), meta).unwrap();
            stage
        };
        let failed = |custody: &Custody| {
            let r: AttemptResult =
                serde_json::from_slice(&read(&custody.dir.join("attempt-result.json")).unwrap())
                    .unwrap();
            (r.state, r.meta_sha256)
        };

        // Publication failure: the output path appears after staging.
        let (dir, custody) = custody_fixture();
        let out = dir.path().join("run");
        let started = start_attempt(&custody, &out).unwrap();
        let stage = staged(&out);
        fs::create_dir(&out).unwrap();
        let attempt = (custody, started);
        let result = consume_on_error(
            Some(&attempt),
            publish_run(stage, &out, meta, report, Some(&attempt)),
        );
        let (custody, _) = &attempt;
        assert!(result.is_err());
        assert_eq!(failed(custody), ("failed".into(), None));
        assert!(!out.join("report.json").exists());
        assert!(start_attempt(custody, &dir.path().join("retry")).is_err());

        // Completion failure after publication: a partial terminal record
        // already occupies the slot. Output is visible but never accepted,
        // and the partial record is not replaced.
        let (dir, custody) = custody_fixture();
        let out = dir.path().join("run");
        let started = start_attempt(&custody, &out).unwrap();
        let stage = staged(&out);
        write_new(&custody.dir.join("attempt-result.json"), b"{").unwrap();
        let attempt = (custody, started);
        let error = consume_on_error(
            Some(&attempt),
            publish_run(stage, &out, meta, report, Some(&attempt)),
        )
        .unwrap_err();
        let (custody, _) = &attempt;
        assert!(
            error
                .to_string()
                .contains("failed-attempt record not written")
        );
        assert_eq!(read(&out.join("report.json")).unwrap(), report);
        assert_eq!(
            read(&custody.dir.join("attempt-result.json")).unwrap(),
            b"{"
        );
        assert!(verify_completed(custody, &out, meta, report).is_err());
        assert!(start_attempt(custody, &dir.path().join("retry")).is_err());

        // Completion bytes written but completion reported an error (e.g.
        // fsync failed): the readable `completed` record is voided, not
        // replaced, and is never accepted.
        let (dir, custody) = custody_fixture();
        let out = dir.path().join("run");
        let started = start_attempt(&custody, &out).unwrap();
        finish_attempt(&custody, &started, Some((meta, report))).unwrap();
        verify_completed(&custody, &out, meta, report).unwrap();
        let attempt = (custody, started);
        let injected: Result<(), _> = Err(EvalError::Invalid("injected sync failure".into()));
        let error = consume_on_error(Some(&attempt), injected).unwrap_err();
        let (custody, _) = &attempt;
        assert!(error.to_string().contains("injected sync failure"));
        assert_eq!(
            failed(custody),
            ("completed".into(), Some(sha256_hex(meta)))
        );
        assert!(custody.dir.join(VOID_RECORD).exists());
        assert!(verify_completed(custody, &out, meta, report).is_err());
        assert!(start_attempt(custody, &dir.path().join("retry")).is_err());

        // Success binds exactly the reread published bytes.
        let (dir, custody) = custody_fixture();
        let out = dir.path().join("run");
        let started = start_attempt(&custody, &out).unwrap();
        let attempt = (custody, started);
        publish_run(staged(&out), &out, meta, report, Some(&attempt)).unwrap();
        let (custody, _) = &attempt;
        verify_completed(custody, &out, meta, report).unwrap();
        assert_eq!(failed(custody).1, Some(sha256_hex(meta)));
        // A dev run (no attempt) publishes without custody records.
        let dev = dir.path().join("dev-run");
        consume_on_error(None, publish_run(staged(&dev), &dev, meta, report, None)).unwrap();
        assert!(dev.join("report.json").exists());
    }

    #[test]
    fn partial_custody_and_symlinks_are_consumed_or_refused() {
        let (dir, custody) = custody_fixture();
        let started = custody.dir.join("attempt-started.json");
        write_new(&started, b"{").unwrap();
        assert!(start_attempt(&custody, &dir.path().join("run")).is_err());
        assert!(write_new(&started, b"replacement").is_err());
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&custody.dir, &link).unwrap();
        let linked = Custody {
            root: custody.root.clone(),
            dir: link,
        };
        assert!(linked.registration().is_err());
        assert!(linked.create_dir().is_err());
        assert!(external_path(&custody.root, &custody.root.join("export.toml")).is_err());
        // A regular file cannot be an output parent.
        let file = dir.path().join("plain-file");
        fs::write(&file, b"x").unwrap();
        let err = external_path(&custody.root, &file.join("run")).unwrap_err();
        assert!(err.to_string().contains("not a directory"), "{err}");
        let huge = fs::OpenOptions::new()
            .write(true)
            .open(custody.dir.join("registration.json"))
            .unwrap();
        huge.set_len(MAX_INPUT_BYTES as u64 + 1).unwrap();
        assert!(custody.registration().is_err());
    }

    #[test]
    fn held_out_run_refuses_before_reading_manifest_or_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing");
        // Unconfirmed: refused before anything is opened.
        assert!(matches!(
            run_held_out(&missing, &missing, "x", false, &missing),
            Err(EvalError::TestSplitRefused)
        ));
        // Confirmed but unpinned: refused by manifest verification first.
        let err = run_held_out(&missing, &missing, "x", true, &missing).unwrap_err();
        assert!(err.to_string().contains("preparation refused"), "{err}");
    }

    /// Custody whose registration binds `preparation` (or not).
    /// `bind` false: partitions match but the registered config identity differs.
    fn bound_custody(base: &Path, preparation: &Preparation, bind: bool) -> Custody {
        let custody = Custody {
            root: base.join("code"),
            dir: base.join("custody"),
        };
        fs::create_dir(&custody.root).unwrap();
        custody.create_dir().unwrap();
        let partitions = preparation.partition_pairs();
        let registration = Registration {
            version: REGISTRATION_VERSION,
            preparation_sha256: preparation.sha256().into(),
            frozen: "synthetic frozen bytes\n".into(),
            identity: InputIdentity {
                config: if bind {
                    preparation.inputs.config.clone()
                } else {
                    "0".repeat(64)
                },
                gates: preparation.inputs.gates.clone(),
                splits: preparation.inputs.splits.clone(),
                pool: preparation.inputs.pool.clone(),
                indexes: preparation
                    .corpora
                    .iter()
                    .map(|c| (c.name.clone(), c.index_sha256.clone()))
                    .collect(),
            },
            preparation_partitions: partitions,
        };
        write_new(
            &custody.dir.join("registration.json"),
            &serde_json::to_vec(&registration).unwrap(),
        )
        .unwrap();
        custody
    }

    #[test]
    fn test_capture_reserves_attempt_before_any_test_payload_read() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let preparation = prepare(&path).unwrap();
        let out = dir.path().join("run");
        let base = fs::canonicalize(dir.path()).unwrap();
        // Dev, combined, pool and provenance payloads are never needed.
        for name in [
            "dev-queries.jsonl",
            "dev-qrels.jsonl",
            "queries.jsonl",
            "qrels.jsonl",
            "pool.jsonl",
            "query-review.jsonl",
            "annotation.jsonl",
            "annotation.md",
        ] {
            fs::remove_file(dir.path().join(name)).unwrap();
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        // Matching partitions/pool but a different registered config
        // identity: refused before reservation; nothing consumed.
        let unbound = bound_custody(
            &base.join("u"),
            &{
                fs::create_dir(base.join("u")).unwrap();
                preparation.clone()
            },
            false,
        );
        assert!(Bench::load_test_reserved(&path, &preparation, &unbound, &out).is_err());
        assert!(!unbound.dir.join("attempt-started.json").exists());
        // Test payload unreadable: the attempt is reserved first, then failed
        // and consumed; it cannot be retried.
        fs::create_dir(base.join("f")).unwrap();
        let failing = bound_custody(&base.join("f"), &preparation, true);
        let qrels = dir.path().join("test-qrels.jsonl");
        let saved = fs::read(&qrels).unwrap();
        fs::remove_file(&qrels).unwrap();
        fs::create_dir(&qrels).unwrap();
        assert!(Bench::load_test_reserved(&path, &preparation, &failing, &out).is_err());
        let result: AttemptResult =
            serde_json::from_slice(&fs::read(failing.dir.join("attempt-result.json")).unwrap())
                .unwrap();
        assert_eq!(result.state, "failed");
        fs::remove_dir(&qrels).unwrap();
        fs::write(&qrels, &saved).unwrap();
        assert!(Bench::load_test_reserved(&path, &preparation, &failing, &out).is_err());
        // Bound and readable: exactly the test partition is captured.
        fs::create_dir(base.join("ok")).unwrap();
        let custody = bound_custody(&base.join("ok"), &preparation, true);
        let (bench, started) =
            Bench::load_test_reserved(&path, &preparation, &custody, &out).unwrap();
        assert!(custody.dir.join("attempt-started.json").exists());
        assert!(!custody.dir.join("attempt-result.json").exists());
        assert_eq!(started.registration_sha256.len(), 64);
        let c = bench.captured.as_ref().unwrap();
        assert_eq!(c.selected, Some(SplitName::Test));
        assert_eq!(bench.queries_sha256, preparation.partitions.test.queries);
        assert_eq!(c.qrels_sha256, preparation.partitions.test.qrels);
        assert_eq!(bench.queries().len(), 180);
        // A second reservation for the same registration is refused.
        assert!(Bench::load_test_reserved(&path, &preparation, &custody, &out).is_err());

        // Per-split binding: freeze digests taken from the manifest match a
        // test capture (not the dev digests); any partition change differs.
        let [dev, test] = preparation.partition_pairs();
        let mut frozen = super::super::sweep::FrozenConfig {
            version: super::super::sweep::FROZEN_VERSION,
            preparation_sha256: preparation.sha256().into(),
            dev_queries_sha256: dev.0,
            dev_qrels_sha256: dev.1,
            test_queries_sha256: test.0,
            test_qrels_sha256: test.1,
            alpha: 0.0,
            beta: 0.0,
            threshold: 0.0,
            seed: 0,
            resamples: 0,
            config_sha256: String::new(),
            splits_sha256: String::new(),
            git_commit: String::new(),
            cargo_lock_sha256: String::new(),
            binary_sha256: String::new(),
            generation_ids: BTreeMap::new(),
            power: super::super::sweep::PowerInputs {
                alpha: 0.025,
                power: 0.8,
                min_effect: 0.05,
                sd_diff_h2: 0.0,
                sd_diff_h5: 0.0,
                n_h2: 0,
                n_h5: 0,
                required_answerable_test: 150,
            },
        };
        assert!(frozen.preparation_problems(&bench).unwrap().is_empty());
        bench.require_prepared_capture(SplitName::Test).unwrap();
        assert!(bench.require_prepared_capture(SplitName::Dev).is_err());
        frozen.test_qrels_sha256.push('x');
        assert_eq!(frozen.preparation_problems(&bench).unwrap().len(), 1);
        frozen.test_qrels_sha256.pop();
        frozen.preparation_sha256 = "0".repeat(64);
        assert_eq!(frozen.preparation_problems(&bench).unwrap().len(), 1);

        // A registration binding a different whole manifest (same partitions
        // and inputs) is refused before reservation.
        fs::create_dir(base.join("m")).unwrap();
        let other = bound_custody(&base.join("m"), &preparation, true);
        let file = other.dir.join("registration.json");
        let mut registration: Registration =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        registration.preparation_sha256 = "0".repeat(64);
        fs::remove_file(&file).unwrap();
        write_new(&file, &serde_json::to_vec(&registration).unwrap()).unwrap();
        assert!(Bench::load_test_reserved(&path, &preparation, &other, &out).is_err());
        assert!(!other.dir.join("attempt-started.json").exists());
    }

    #[test]
    fn registration_binds_exact_export_and_admitted_capture() {
        let dir = fixture();
        let config = dir.path().join("config.json");
        let evidence = prepare(&config).unwrap();
        let mut bench = Bench::load_dev(&config, &evidence).unwrap();
        let (_, preparation_sha256, [dev, test]) = bench.preparation_binding().unwrap();
        let frozen = super::super::sweep::FrozenConfig {
            version: super::super::sweep::FROZEN_VERSION,
            preparation_sha256: preparation_sha256.into(),
            dev_queries_sha256: dev.0.clone(),
            dev_qrels_sha256: dev.1.clone(),
            test_queries_sha256: test.0.clone(),
            test_qrels_sha256: test.1.clone(),
            alpha: bench.config.alpha,
            beta: bench.config.beta,
            threshold: bench.config.threshold,
            seed: bench.config.seed,
            resamples: bench.config.resamples,
            config_sha256: bench.config_sha256.clone(),
            splits_sha256: bench.splits().unwrap().1,
            git_commit: "synthetic".into(),
            cargo_lock_sha256: "synthetic".into(),
            binary_sha256: "synthetic".into(),
            generation_ids: bench
                .indexes()
                .unwrap()
                .iter()
                .map(|(name, index)| (name.clone(), index.generation.generation_id.clone()))
                .collect(),
            power: super::super::sweep::PowerInputs {
                alpha: 0.025,
                power: 0.8,
                min_effect: 0.05,
                sd_diff_h2: 0.0,
                sd_diff_h5: 0.0,
                n_h2: 0,
                n_h5: 0,
                required_answerable_test: 150,
            },
        };
        let capture = bench.captured.as_ref().unwrap();
        let mut registration = Registration {
            version: REGISTRATION_VERSION,
            preparation_sha256: capture.preparation_sha256.clone().unwrap(),
            frozen: frozen.to_toml(),
            identity: capture.identity.clone(),
            preparation_partitions: capture.preparation_partitions.clone().unwrap(),
        };
        let text = registration.frozen.clone();
        check_registration(&registration, &bench, text.as_bytes()).unwrap();
        assert!(check_registration(&registration, &bench, format!("{text}\n").as_bytes()).is_err());
        assert!(
            check_registration(
                &registration,
                &bench,
                format!("# extra comment\n{text}").as_bytes()
            )
            .is_err()
        );
        let original = bench.captured.as_ref().unwrap().identity.clone();
        for field in ["gates", "pool", "indexes", "config", "splits"] {
            let identity = &mut bench.captured.as_mut().unwrap().identity;
            *identity = original.clone();
            match field {
                "gates" => identity.gates.push('x'),
                "pool" => identity.pool.push('x'),
                "config" => identity.config.push('x'),
                "splits" => identity.splits.push('x'),
                _ => {
                    identity.indexes.insert("changed".into(), "changed".into());
                }
            }
            assert!(
                check_registration(&registration, &bench, text.as_bytes()).is_err(),
                "{field}"
            );
        }
        bench.captured.as_mut().unwrap().identity = original;
        bench
            .captured
            .as_mut()
            .unwrap()
            .preparation_partitions
            .as_mut()
            .unwrap()[1]
            .1
            .push('x');
        assert!(check_registration(&registration, &bench, text.as_bytes()).is_err());
        bench.captured.as_mut().unwrap().preparation_partitions =
            Some(registration.preparation_partitions.clone());
        // The whole manifest hash is bound, not only the partitions.
        bench.captured.as_mut().unwrap().preparation_sha256 = Some("0".repeat(64));
        assert!(check_registration(&registration, &bench, text.as_bytes()).is_err());
        bench.captured.as_mut().unwrap().preparation_sha256 =
            Some(registration.preparation_sha256.clone());
        check_registration(&registration, &bench, text.as_bytes()).unwrap();
        registration.version = 1;
        assert!(check_registration(&registration, &bench, text.as_bytes()).is_err());
        registration.version = REGISTRATION_VERSION;

        // Exercise the real registration/export APIs using a clean synthetic
        // code checkout. This is not a public measurement/readiness bypass.
        let code = dir.path().join("code");
        fs::create_dir(&code).unwrap();
        fs::write(code.join("Cargo.lock"), b"synthetic lock\n").unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "Cargo.lock"],
            vec!["commit", "-qm", "synthetic code"],
        ] {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&code)
                .args([
                    "-c",
                    "user.name=fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let identity = super::super::sweep::CodeIdentity::current(&code).unwrap();
        let mut frozen = frozen;
        frozen.git_commit = identity.git_commit;
        frozen.cargo_lock_sha256 = identity.cargo_lock_sha256;
        frozen.binary_sha256 = identity.binary_sha256;
        let export = dir.path().join("frozen-config.toml");
        let text = frozen.to_toml();
        register_freeze(&code, &bench, &text, &export).unwrap();
        let bytes = read(&export).unwrap();
        assert_eq!(bytes, text.as_bytes());
        assert_eq!(
            check_registered_export(&code, &bench, &bytes).unwrap(),
            frozen
        );
        assert!(register_freeze(&code, &bench, &text, &dir.path().join("another.toml")).is_err());
        assert!(!dir.path().join("another.toml").exists());
        assert!(check_registered_export(&code, &bench, format!("{text}\n").as_bytes()).is_err());
        fs::write(code.join("Cargo.lock"), b"changed lock\n").unwrap();
        assert!(check_registered_export(&code, &bench, &bytes).is_err());
        fs::write(code.join("Cargo.lock"), b"synthetic lock\n").unwrap();
        check_registered_export(&code, &bench, &bytes).unwrap();
    }

    fn write_json(path: &Path, value: &impl Serialize) {
        fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    }

    fn write_lines(path: &Path, values: &[impl Serialize]) {
        let bytes = values
            .iter()
            .map(|v| format!("{}\n", serde_json::to_string(v).unwrap()))
            .collect::<String>();
        fs::write(path, bytes).unwrap();
    }

    /// Synthetic accepted review; GPT-6-Astra independently reviews drafts.
    fn review_record(q: &Query) -> serde_json::Value {
        let digest = sha256_hex(q.text.as_bytes());
        let author = json!({"id":"drafter","kind":"model","model":"drafter","evidence":format!("draft/{}", q.id)});
        let reviewer = json!({"id":"astra","kind":"model","model":"gpt-6-astra","evidence":format!("review/{}", q.id)});
        json!({"version":1,"query_id":q.id,"final_text_sha256":digest,"corpus":q.corpus,
            "stratum":q.stratum,"model_drafted":q.model_drafted,"stratum_evidence":"synthetic",
            "author":author,"reviewer":reviewer,"decision":"accept","rationale":"synthetic",
            "history":[{"text_sha256":digest,"predecessor_sha256":null,"author":author,
                "reviewer":reviewer,"decision":"accept"}]})
    }

    fn review_pin(dir: &Path) -> String {
        sha256_hex(&fs::read(dir.join("query-review.jsonl")).unwrap())
    }

    /// Trusted preparation with the operator-pinned review of `path`'s bench.
    fn prepare(path: &Path) -> Result<Preparation, EvalError> {
        let dir = path.parent().unwrap();
        let pin = review_pin(dir);
        let (review, annotation, notes) = (
            dir.join("query-review.jsonl"),
            dir.join("annotation.jsonl"),
            dir.join("annotation.md"),
        );
        Bench::prepare(
            path,
            &TrustedInputs {
                query_review: &review,
                query_review_sha256: &pin,
                annotation: &annotation,
                annotation_notes: &notes,
                operator: "op",
                operator_evidence: "audit/prep",
            },
        )
    }

    /// Synthetic double-labelled annotation matching `judgements`; never a
    /// substitute for human evidence.
    fn write_annotation(dir: &Path, judgements: &[Judgement]) {
        // Raw grades vary (both humans agree) so kappa is defined; the human
        // adjudicator's final grade is the qrel grade.
        let records: Vec<_> = judgements
            .iter()
            .enumerate()
            .map(|(i, j)| {
                let raw = [1, 2, 0][i % 3];
                json!({"version":1,"query_id":j.query_id,"section_id":j.section_id,
                    "span_sha256":j.span_sha256,
                    "grades":[{"annotator":"h1","grade":raw,"evidence":"g1"},
                        {"annotator":"h2","grade":raw,"evidence":"g2"}],
                    "final":{"grade":j.grade,"adjudicator":{"id":"adj","kind":"human","evidence":"a"},"evidence":"agreed"}})
            })
            .collect();
        write_lines(&dir.join("annotation.jsonl"), &records);
        fs::write(dir.join("annotation.md"), ANNOTATION_NOTES).unwrap();
    }

    const ANNOTATION_NOTES: &str = "```annotation-notes\n{\"version\":1,\"guideline_version\":\"g1\",\"annotators\":[{\"id\":\"h1\",\"kind\":\"human\",\"evidence\":\"e1\"},{\"id\":\"h2\",\"kind\":\"human\",\"evidence\":\"e2\"}],\"assignment_seed\":1,\"overlap\":\"corpus-stratum-ceil30-v1\",\"blinding_evidence\":\"b\",\"adjudication_rule\":\"r\"}\n```\n";

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("corpus");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("a.md"),
            (0..10)
                .map(|n| format!("# Heading {n}\n\nretrieval documentation {n}\n\n"))
                .collect::<String>(),
        )
        .unwrap();
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args([
                    "-c",
                    "user.name=fixture",
                    "-c",
                    "user.email=fixture@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "synthetic"]);
        let commit = git(&["rev-parse", "HEAD"]);
        let generation = crate::store::build(&root, &crate::store::IndexConfig::default()).unwrap();
        crate::store::Store::new(root.join(".ontodoc"))
            .publish(&generation)
            .unwrap();
        let mut queries = Vec::new();
        for stratum in [
            Stratum::Lookup,
            Stratum::Conceptual,
            Stratum::MultiHop,
            Stratum::OffTopic,
        ] {
            for n in 0..if stratum == Stratum::OffTopic { 45 } else { 75 } {
                let id = format!("{}x{n}", stratum.name());
                let text = if stratum == Stratum::OffTopic {
                    format!("unrelated{id}")
                } else {
                    format!("retrieval unique{id}")
                };
                queries.push(Query {
                    id,
                    text,
                    stratum,
                    corpus: "synthetic".into(),
                    model_drafted: true,
                });
            }
        }
        let splits = split(&queries, 7);
        let config = json!({"corpora":[{"name":"synthetic","root":"corpus"}],
            "queries":"queries.jsonl","qrels":"qrels.jsonl","splits":"splits.json",
            "partitions":{"dev":{"queries":"dev-queries.jsonl","qrels":"dev-qrels.jsonl"},
                "test":{"queries":"test-queries.jsonl","qrels":"test-qrels.jsonl"}},
            "seed":7,"resamples":10000,"alpha_grid":[0.0],"beta_grid":[0.0],"alpha":0.0,"beta":0.0,"threshold":0.3});
        write_json(&dir.path().join("config.json"), &config);
        write_json(&dir.path().join("splits.json"), &splits);
        write_lines(&dir.path().join("queries.jsonl"), &queries);
        let reviews: Vec<_> = queries.iter().map(review_record).collect();
        write_lines(&dir.path().join("query-review.jsonl"), &reviews);
        let bench = Bench::load(&dir.path().join("config.json")).unwrap();
        let approval = super::super::gates::check_query_review(
            &bench.queries,
            &fs::read(dir.path().join("query-review.jsonl")).unwrap(),
            &review_pin(dir.path()),
        )
        .unwrap();
        pool(&bench, &dir.path().join("pool.jsonl"), &approval).unwrap();
        let (pairs, _): (Vec<serde_json::Value>, _) =
            read_jsonl(&dir.path().join("pool.jsonl")).unwrap();
        let judgements: Vec<_> = pairs
            .iter()
            .map(|p| Judgement {
                query_id: p["query_id"].as_str().unwrap().into(),
                section_id: p["section_id"].as_str().unwrap().into(),
                grade: 1,
                span_sha256: Some(
                    generation
                        .sections
                        .iter()
                        .find(|s| s.id == p["section_id"])
                        .unwrap()
                        .sha256
                        .clone(),
                ),
            })
            .collect();
        write_lines(&dir.path().join("qrels.jsonl"), &judgements);
        write_annotation(dir.path(), &judgements);
        for (name, ids) in [("dev", &splits.dev), ("test", &splits.test)] {
            write_lines(
                &dir.path().join(format!("{name}-queries.jsonl")),
                &queries
                    .iter()
                    .filter(|q| ids.contains(&q.id))
                    .collect::<Vec<_>>(),
            );
            write_lines(
                &dir.path().join(format!("{name}-qrels.jsonl")),
                &judgements
                    .iter()
                    .filter(|j| ids.contains(&j.query_id))
                    .collect::<Vec<_>>(),
            );
        }
        write_json(
            &dir.path().join("gates.json"),
            &json!({"floors":{"min_answerable_test":150,"min_off_topic_test":30,"min_per_stratum_test":15},
            "pins":[{"name":"synthetic","commit":commit}],"pool":"pool.jsonl"}),
        );
        dir
    }

    #[test]
    fn dev_capture_computes_without_heldout_or_reopening_captured_inputs() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        let second = prepare(&path).unwrap();
        assert_eq!(evidence, second);
        for name in [
            "test-queries.jsonl",
            "test-qrels.jsonl",
            "queries.jsonl",
            "qrels.jsonl",
            "pool.jsonl",
        ] {
            fs::remove_file(dir.path().join(name)).unwrap();
        }
        let dev = Bench::load_dev(&path, &evidence).unwrap();
        let report = super::super::sweep::sweep_captured(&dev).unwrap();
        assert_eq!(report.dev_answerable, 75);
        // All ten retrieved sections are equally relevant: DCG == IDCG, nDCG=1.
        assert_eq!(report.b1_ndcg10, 1.0);
        assert!(Arc::ptr_eq(
            &dev.indexes().unwrap(),
            &dev.indexes().unwrap()
        ));
        for name in [
            "config.json",
            "dev-queries.jsonl",
            "dev-qrels.jsonl",
            "splits.json",
            "gates.json",
        ] {
            fs::remove_file(dir.path().join(name)).unwrap();
        }
        fs::remove_dir_all(dir.path().join("corpus")).unwrap();
        assert_eq!(super::super::sweep::sweep_captured(&dev).unwrap(), report);
        assert!(super::super::gates::check_qrels(&dev).unwrap().is_empty());
        // Public sweep now admits the pinned dev capture with identical output.
        assert_eq!(
            super::super::sweep::sweep(&dev, SplitName::Dev).unwrap(),
            report
        );
    }

    #[test]
    fn dev_admission_rejects_changed_identities_and_stale_corpus() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        for name in [
            "config.json",
            "gates.json",
            "splits.json",
            "dev-queries.jsonl",
            "dev-qrels.jsonl",
        ] {
            let p = dir.path().join(name);
            let original = fs::read(&p).unwrap();
            let mut changed = original.clone();
            changed.push(b'\n');
            fs::write(&p, &changed).unwrap();
            assert!(Bench::load_dev(&path, &evidence).is_err(), "{name}");
            fs::write(p, original).unwrap();
        }
        let source = dir.path().join("corpus/a.md");
        fs::write(&source, "changed bytes").unwrap();
        assert!(Bench::load_dev(&path, &evidence).is_err());
    }

    #[test]
    fn dev_admission_propagates_unsafe_and_oversized_control_refusals() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        let control = dir.path().join("corpus/.ontodocignore");
        fs::write(&control, vec![b'a'; 64 * 1024 + 1]).unwrap();
        let err = Bench::load_dev(&path, &evidence).err().unwrap();
        assert!(matches!(
            err,
            EvalError::Store {
                source: crate::store::StoreError::Corpus(
                    crate::corpus::CorpusError::CaptureBudget("ignore file byte")
                ),
                ..
            }
        ));
        fs::remove_file(&control).unwrap();
        std::os::unix::fs::symlink(dir.path().join("test-qrels.jsonl"), &control).unwrap();
        let err = Bench::load_dev(&path, &evidence).err().unwrap();
        assert!(matches!(
            err,
            EvalError::Store {
                source: crate::store::StoreError::Corpus(
                    crate::corpus::CorpusError::UnsafeControl(_)
                ),
                ..
            }
        ));
    }

    #[test]
    fn mechanical_failures_and_missing_partitions_cannot_mint_evidence() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let qpath = dir.path().join("dev-qrels.jsonl");
        let (mut js, _): (Vec<Judgement>, _) = read_jsonl(&qpath).unwrap();
        let original = fs::read(&qpath).unwrap();
        js[0].span_sha256 = Some("invalid".into());
        write_lines(&qpath, &js);
        assert!(prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        js = read_jsonl::<Judgement>(&qpath).unwrap().0;
        js.push(js[0].clone());
        write_lines(&qpath, &js);
        assert!(prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        js = read_jsonl::<Judgement>(&qpath).unwrap().0;
        js[0].query_id = "unknown".into();
        write_lines(&qpath, &js);
        assert!(prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        let evidence = prepare(&path).unwrap();
        fs::remove_file(dir.path().join("dev-queries.jsonl")).unwrap();
        assert!(Bench::load_dev(&path, &evidence).is_err()); // Combined exists; never a fallback.
    }

    #[test]
    fn bounded_capture_rejects_oversized_malformed_and_special_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("input");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_INPUT_BYTES as u64 + 1).unwrap();
        assert!(read(&path).is_err());
        fs::write(&path, [0xff]).unwrap();
        assert!(read_jsonl::<Query>(&path).is_err());
        std::os::unix::fs::symlink(&path, dir.path().join("link")).unwrap();
        assert!(read(&dir.path().join("link")).is_err());
        assert!(
            std::process::Command::new("mkfifo")
                .arg(dir.path().join("fifo"))
                .status()
                .unwrap()
                .success()
        );
        assert!(read(&dir.path().join("fifo")).is_err());
        let mut budget = 1;
        fs::write(&path, "ab").unwrap();
        assert!(captured_read(&path, 10, &mut budget).is_err());
    }

    #[test]
    fn captured_calculation_refuses_missing_positive_judgements() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        let mut dev = Bench::load_dev(&path, &evidence).unwrap();
        let c = dev.captured.as_mut().unwrap();
        c.qrels.clear();
        assert!(
            super::super::sweep::sweep_captured(&dev)
                .unwrap_err()
                .to_string()
                .contains("missing positive judgement")
        );
    }

    #[test]
    fn preparation_manifest_binds_inputs_counts_and_pinned_bytes() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let preparation = prepare(&path).unwrap();
        let out = dir.path().join("preparation.json");
        let pin = preparation.publish(&out).unwrap();
        // Publication is create-new: never overwritten.
        assert!(preparation.publish(&out).is_err());
        let bytes = fs::read(&out).unwrap();
        assert_eq!(sha256_hex(&bytes), pin);
        assert_eq!(Preparation::verify(&bytes, &pin).unwrap(), preparation);
        let raw = |name: &str| sha256_hex(&fs::read(dir.path().join(name)).unwrap());
        let i = &preparation.inputs;
        assert_eq!(
            [
                &i.queries,
                &i.qrels,
                &i.annotation,
                &i.annotation_notes,
                &i.pool
            ],
            [
                &raw("queries.jsonl"),
                &raw("qrels.jsonl"),
                &raw("annotation.jsonl"),
                &raw("annotation.md"),
                &raw("pool.jsonl"),
            ]
        );
        let p = &preparation.partitions;
        assert_eq!(p.dev.queries, raw("dev-queries.jsonl"));
        assert_eq!(p.test.qrels, raw("test-qrels.jsonl"));
        // The commitment is distinct from every selected execution digest.
        assert!(![&p.dev.queries, &p.dev.qrels].contains(&&p.commitment));
        // 75 per answerable stratum and 45 off-topic, split by seed 7; counts
        // derive from the captured records.
        let c = &preparation.counts;
        let total: usize = c
            .queries
            .values()
            .flat_map(|m| m.values())
            .flat_map(|m| m.values())
            .sum();
        assert_eq!(total, 270);
        assert_eq!(c.answerable_positive.values().sum::<usize>(), 225);
        assert_eq!((c.reviewed, c.model_drafted), (270, 270));
        assert_eq!(c.agreement.answerable_queries, 225);
        assert_eq!(preparation.policy.promotion_policy, PROMOTION_POLICY);
        // Wrong pin, uppercase pin, unknown field and version all refuse.
        assert!(Preparation::verify(&bytes, &"0".repeat(64)).is_err());
        assert!(Preparation::verify(&bytes, &pin.to_uppercase()).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["extra"] = json!(1);
        let tampered = serde_json::to_vec(&value).unwrap();
        assert!(Preparation::verify(&tampered, &sha256_hex(&tampered)).is_err());
        // Each refusal is semantic: the typed mutation alone fails verify.
        assert_eq!(pin, preparation.sha256());
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["version"] = json!(2);
        let tampered = serde_json::to_vec(&value).unwrap();
        assert!(Preparation::verify(&tampered, &sha256_hex(&tampered)).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["partitions"]["test"]["qrels"] = json!("0".repeat(64));
        let tampered = serde_json::to_vec(&value).unwrap();
        assert!(Preparation::verify(&tampered, &sha256_hex(&tampered)).is_err());
        let mut minified = serde_json::to_vec(&preparation).unwrap();
        let admitted = Preparation::verify(&minified, &sha256_hex(&minified)).unwrap();
        assert_eq!(admitted.sha256(), sha256_hex(&minified)); // Bytes, not reserialization.
        for (field, mutate) in [
            (
                "version",
                (|p: &mut Preparation| p.version = 2) as fn(&mut Preparation),
            ),
            ("commitment", |p| p.partitions.commitment = "0".repeat(64)),
            ("policy", |p| p.policy.promotion_policy = "other".into()),
        ] {
            let mut typed = preparation.clone();
            mutate(&mut typed);
            minified = typed.to_bytes().unwrap();
            assert!(
                Preparation::verify(&minified, &sha256_hex(&minified)).is_err(),
                "{field}"
            );
        }
        // Cross-process dev admission uses only the pinned manifest.
        for name in [
            "test-queries.jsonl",
            "test-qrels.jsonl",
            "queries.jsonl",
            "qrels.jsonl",
            "pool.jsonl",
            "query-review.jsonl",
            "annotation.jsonl",
            "annotation.md",
        ] {
            fs::remove_file(dir.path().join(name)).unwrap();
        }
        let dev = Bench::load_dev_pinned(&path, &out, &pin).unwrap();
        assert_eq!(
            dev.queries().len(),
            preparation.counts.queries["dev"]
                .values()
                .flat_map(|m| m.values())
                .sum::<usize>()
        );
        assert!(Bench::load_dev_pinned(&path, &out, &"0".repeat(64)).is_err());
    }

    #[test]
    fn preparation_refuses_combined_union_mismatch_and_missing_operator() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let combined = dir.path().join("queries.jsonl");
        let original = fs::read(&combined).unwrap();
        let (mut queries, _): (Vec<Query>, _) = read_jsonl(&combined).unwrap();
        queries.pop();
        write_lines(&combined, &queries);
        let err = prepare(&path).err().unwrap().to_string();
        assert!(err.contains("partition union"), "{err}");
        queries.push(queries[0].clone());
        write_lines(&combined, &queries);
        assert!(prepare(&path).is_err());
        fs::write(&combined, original).unwrap();
        let d = dir.path();
        let (review, annotation, notes) = (
            d.join("query-review.jsonl"),
            d.join("annotation.jsonl"),
            d.join("annotation.md"),
        );
        let pin = review_pin(d);
        for (operator, evidence) in [("", "e"), ("op", " "), (&*"o".repeat(129), "e")] {
            let trusted = TrustedInputs {
                query_review: &review,
                query_review_sha256: &pin,
                annotation: &annotation,
                annotation_notes: &notes,
                operator,
                operator_evidence: evidence,
            };
            assert!(Bench::prepare(&path, &trusted).is_err());
        }
    }

    #[test]
    fn shared_record_budget_charges_nested_entries() {
        #[derive(Deserialize)]
        struct R {
            n: Vec<u8>,
        }
        let bytes = b"{\"n\":[1,2]}\n{\"n\":[3]}\n";
        // Weights 1+2 and 1+1: exactly 5 fit, 4 refuse at the second record.
        let mut budget = 5;
        let parsed: Vec<R> =
            parse_jsonl_within(Path::new("x"), bytes, &mut budget, |r: &R| 1 + r.n.len()).unwrap();
        assert_eq!((parsed.len(), budget), (2, 0));
        let mut budget = 4;
        assert!(
            parse_jsonl_within(Path::new("x"), bytes, &mut budget, |r: &R| 1 + r.n.len()).is_err()
        );
        let mut budget = 0;
        assert!(parse_jsonl_within::<R>(Path::new("x"), bytes, &mut budget, |_| 1).is_err());
    }

    #[test]
    fn dev_capture_retains_complete_preparation_identity() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let first = prepare(&path).unwrap();
        let label_path = dir.path().join("test-qrels.jsonl");
        let (mut labels, _): (Vec<Judgement>, _) = read_jsonl(&label_path).unwrap();
        labels[0].grade = 2;
        write_lines(&label_path, &labels);
        // Adjudication changed the final grade; the annotation records it.
        let (dev_labels, _): (Vec<Judgement>, _) =
            read_jsonl(&dir.path().join("dev-qrels.jsonl")).unwrap();
        let mut all: Vec<_> = dev_labels.into_iter().chain(labels).collect();
        all.sort_by(|a, b| (&a.query_id, &a.section_id).cmp(&(&b.query_id, &b.section_id)));
        write_annotation(dir.path(), &all);
        // The combined authoring file is the exact union of both partitions.
        write_lines(&dir.path().join("qrels.jsonl"), &all);
        let second = prepare(&path).unwrap();
        let a = Bench::load_dev(&path, &first).unwrap();
        let b = Bench::load_dev(&path, &second).unwrap();
        assert_eq!(a.queries_sha256, b.queries_sha256);
        assert_ne!(
            a.captured.as_ref().unwrap().preparation_partitions,
            b.captured.as_ref().unwrap().preparation_partitions
        );
    }

    #[test]
    fn run_file_bound_refuses_unscorable_output_before_writing() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        let dev = Bench::load_dev(&path, &evidence).unwrap();
        let indexes = dev.indexes().unwrap();
        let section = indexes["synthetic"]
            .generation
            .chunks
            .iter()
            .map(|c| c.section_id.len())
            .max()
            .unwrap();
        let systems = [System::B0, System::B1, System::B2(0.0), System::B3(0.0)];
        // Names B0, B1, B2@0, B3@0: 2 + 2 + 4 + 4 = 12 bytes. Per query:
        // 4 systems x CUTOFF 10 lines x (qid + section + score 32 + 10 fixed)
        // + name bytes x 10, plus one abstention line (qid + 32 + 2).
        let ids = ["q1".to_string(), "q22".to_string()];
        let per = |qid: usize| 40 * (qid + section + MAX_SCORE_TEXT + 10) + 120 + qid + 34;
        assert_eq!(run_file_bound(&ids, &indexes, &systems), per(2) + per(3));
        // The held-out bound uses the manifest-pinned test ids (metadata only).
        let (splits, _) = dev.splits().unwrap();
        // Those ids must be TSV-safe before reservation; queries too.
        assert!(check_run_ids(&splits.test).is_ok());
        for bad in ["a\tb", "a\nb", "a\rb", ""] {
            assert!(check_run_ids(&[bad.to_string()]).is_err(), "{bad:?}");
            let mut queries = dev.queries().to_vec();
            queries[0].id = bad.into();
            assert!(validate_queries(&queries, &dev.config).is_err(), "{bad:?}");
        }
        assert!(check_run_file_bytes(run_file_bound(&splits.test, &indexes, &systems)).is_ok());
        // A section id wide enough that the pinned test split cannot be scored:
        // refused by the same check admission runs before any reservation.
        let mut generation = indexes["synthetic"].generation.clone();
        // Widen only the emitted chunk section id (not `sections[*].id`).
        generation.chunks[0].section_id = "s".repeat(MAX_INPUT_BYTES / (splits.test.len() * 40));
        let wide = BTreeMap::from([("synthetic".to_string(), SearchIndex::new(generation))]);
        assert!(check_run_file_bytes(run_file_bound(&splits.test, &wide, &systems)).is_err());
        // Every written score is `{:e}`, within MAX_SCORE_TEXT, and parses back.
        for v in [f64::MAX, -f64::MAX, f64::MIN_POSITIVE / 3.0, 1.0 / 3.0, 0.0] {
            let text = format!("{v:e}");
            assert!(text.len() <= MAX_SCORE_TEXT, "{text}");
            assert_eq!(text.parse::<f64>().unwrap(), v);
        }
        // Real dev output stays within the bound computed beforehand.
        let out = dir.path().join("run-dev");
        run(&dev, SplitName::Dev, false, &out).unwrap();
        let bound = run_file_bound(&splits.dev, &indexes, &systems);
        let total: u64 = fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap())
            .filter(|e| e.path().extension().is_some_and(|x| x == "tsv"))
            .map(|e| e.metadata().unwrap().len())
            .sum();
        assert!(total > 0 && total as usize <= bound, "{total} > {bound}");
    }

    #[test]
    fn captured_scoring_checks_and_parses_identical_saved_bytes() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = prepare(&path).unwrap();
        let bench = Bench::load_dev(&path, &evidence).unwrap();
        let indexes = bench.indexes().unwrap();
        let run_dir = dir.path().join("run");
        fs::create_dir(&run_dir).unwrap();
        let systems = [System::B0, System::B1, System::B2(0.0), System::B3(0.0)];
        let mut hashes = BTreeMap::new();
        for system in systems {
            let name = system.name();
            let mut text = String::new();
            for q in &bench.queries {
                for (rank, (section, score)) in rank_sections(&indexes[&q.corpus], system, &q.text)
                    .iter()
                    .enumerate()
                {
                    text.push_str(&format!(
                        "{}\tQ0\t{section}\t{}\t{score}\t{name}\n",
                        q.id,
                        rank + 1
                    ));
                }
            }
            let filename = format!("{name}.tsv");
            fs::write(run_dir.join(&filename), &text).unwrap();
            hashes.insert(filename, sha256_hex(text.as_bytes()));
        }
        let abstention = bench
            .queries
            .iter()
            .map(|q| {
                format!(
                    "{}\t{}\n",
                    q.id,
                    if q.stratum == Stratum::OffTopic {
                        0.0
                    } else {
                        1.0
                    }
                )
            })
            .collect::<String>();
        fs::write(run_dir.join("abstention.tsv"), &abstention).unwrap();
        hashes.insert("abstention.tsv".into(), sha256_hex(abstention.as_bytes()));
        let meta = RunMeta {
            version: RUN_META_VERSION,
            split: SplitName::Dev,
            preparation_sha256: bench.preparation_binding().unwrap().1.into(),
            seed: 7,
            resamples: 10,
            alpha: 0.0,
            beta: 0.0,
            threshold: 0.3,
            config_sha256: bench.config_sha256.clone(),
            queries_sha256: bench.queries_sha256.clone(),
            qrels_sha256: bench.qrels().unwrap().1,
            splits_sha256: bench.splits().unwrap().1,
            corpora: indexes
                .iter()
                .map(|(name, i)| CorpusIdentity {
                    name: name.clone(),
                    generation_id: i.generation.generation_id.clone(),
                })
                .collect(),
            git_commit: None,
            git_dirty: None,
            binary_sha256: None,
            systems: systems.iter().map(|s| s.name()).collect(),
            run_sha256: hashes,
        };
        // Synthetic bootstrap uses ten repetitions only, never a scientific run.
        let mut bench = bench;
        bench.config.resamples = 10;
        // Production run passes this same constructed metadata value directly:
        // replacing a staged metadata file must not change scored identities.
        fs::write(run_dir.join("meta.json"), b"replaced metadata").unwrap();
        let report = score_captured(&bench, &run_dir, false, meta.clone()).unwrap();
        // Old metadata version or another manifest's hash is refused.
        let mut stale = meta.clone();
        stale.version = 1;
        assert!(score_captured(&bench, &run_dir, false, stale).is_err());
        let mut other = meta.clone();
        other.preparation_sha256 = "0".repeat(64);
        assert!(score_captured(&bench, &run_dir, false, other).is_err());
        assert_eq!(report.answerable, 75);
        assert_eq!(report.systems["B1"].mean.ndcg10, 1.0);
        // Per-corpus copies of all seven specs, exploratory only.
        assert_eq!(report.per_corpus_comparisons.len(), 7);
        assert!(
            report
                .per_corpus_comparisons
                .iter()
                .all(|c| c.role == "exploratory" && c.holm_p.is_none() && c.decision.is_none())
        );
        assert_eq!(report.abstention.correct_abstention.count, 15);
        let ranking = run_dir.join("B1.tsv");
        let bytes = fs::read(&ranking).unwrap();
        fs::remove_file(&ranking).unwrap();
        assert_eq!(parse_run(&ranking, &bytes, "B1").unwrap().len(), 75);
        fs::write(&ranking, b"tampered").unwrap();
        assert!(
            score_captured(&bench, &run_dir, false, meta.clone())
                .unwrap_err()
                .to_string()
                .contains("changed since the run")
        );
        fs::write(&ranking, bytes).unwrap();
        fs::write(run_dir.join("abstention.tsv"), b"tampered").unwrap();
        assert!(
            score_captured(&bench, &run_dir, false, meta)
                .unwrap_err()
                .to_string()
                .contains("changed since the run")
        );
    }
}
