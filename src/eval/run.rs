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
const MAX_RECORDS: usize = 100_000;
const MAX_QUERIES: usize = 2_000;
const MAX_RANKINGS: usize = 100_000;

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

fn parse_jsonl<T: for<'de> Deserialize<'de>>(
    path: &Path,
    bytes: &[u8],
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
        if out.len() == MAX_RECORDS || line.len() > 32 * 1024 {
            return Err(EvalError::Invalid("JSONL record limit exceeded".into()));
        }
        out.push(serde_json::from_str(line).map_err(|e| EvalError::Parse {
            path: path.to_path_buf(),
            line: i + 1,
            message: e.to_string(),
        })?);
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

struct CapturedInputs {
    identity: InputIdentity,
    splits: Splits,
    judgements: Vec<Judgement>,
    qrels: BTreeMap<String, Qrels>,
    qrels_sha256: String,
    indexes: Arc<BTreeMap<String, SearchIndex>>,
    gates: super::gates::GateConfig,
    pool: Vec<u8>,
    selected: Option<SplitName>,
    preparation_partitions: Option<[(String, String); 2]>,
    freshness_failures: Vec<super::gates::GateFailure>,
    pin_failures: Vec<super::gates::GateFailure>,
}

/// In-process mechanical prevalidation only. Not serializable, constructible by
/// callers, or evidence of query review, human annotation or durable custody.
pub struct MechanicalEvidence {
    identity: InputIdentity,
    partitions: [(String, String); 2],
}

fn captured_read(path: &Path, cap: usize, budget: &mut usize) -> Result<Vec<u8>, EvalError> {
    let bytes = read_bounded(path, cap.min(*budget))?;
    *budget -= bytes.len();
    Ok(bytes)
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
        || [&config.alpha_grid, &config.beta_grid].iter().any(|grid| {
            grid.is_empty()
                || grid.len() > 32
                || grid
                    .iter()
                    .any(|w| !w.is_finite() || !(0.0..=1.0).contains(w))
        })
    {
        return Err(EvalError::Invalid(
            "invalid or oversized corpus/grid configuration".into(),
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
        if q.id.is_empty()
            || q.id.len() > 256
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
fn capture_indexes(
    dir: &Path,
    config: &EvalConfig,
    budget: &mut usize,
) -> Result<(IndexMap, BTreeMap<String, String>), EvalError> {
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
        hashes.insert(c.name.clone(), sha256_hex(&bytes));
        indexes.insert(c.name.clone(), SearchIndex::new(generation));
    }
    Ok((indexes, hashes))
}

impl Bench {
    /// Full preparation may inspect both partitions. Only a passing full gate
    /// check can mint evidence; a JSON GateReport cannot be admitted here.
    pub fn prepare(config_path: &Path) -> Result<MechanicalEvidence, EvalError> {
        let (bench, partitions) = Self::capture(config_path, None)?;
        let c = bench
            .captured
            .as_ref()
            .ok_or_else(|| EvalError::Invalid("missing capture".into()))?;
        let report = super::gates::check_benchmark(
            &bench,
            &c.gates,
            super::gates::Stage::Qrels,
            None,
            &bench.dir,
        )?;
        if !report.passed {
            return Err(EvalError::Invalid(format!(
                "mechanical prevalidation failed: {}",
                serde_json::to_string(&report.failures)?
            )));
        }
        Ok(MechanicalEvidence {
            identity: c.identity.clone(),
            partitions,
        })
    }

    /// Select paths before any payload read. No combined or held-out fallback.
    pub fn load_dev(config_path: &Path, evidence: &MechanicalEvidence) -> Result<Self, EvalError> {
        let (mut bench, partitions) = Self::capture(config_path, Some(SplitName::Dev))?;
        let c = bench
            .captured
            .as_ref()
            .ok_or_else(|| EvalError::Invalid("missing capture".into()))?;
        if c.identity != evidence.identity || partitions[0] != evidence.partitions[0] {
            return Err(EvalError::Invalid(
                "captured dev identity differs from prevalidation".into(),
            ));
        }
        if !c.freshness_failures.is_empty() || !c.pin_failures.is_empty() {
            return Err(EvalError::Invalid(
                "corpus freshness or pin changed since preparation".into(),
            ));
        }
        if let Some(c) = &mut bench.captured {
            c.preparation_partitions = Some(evidence.partitions.clone());
        }
        Ok(bench)
    }

    pub fn queries(&self) -> &[Query] {
        &self.queries
    }

    pub(crate) fn require_dev_capture(&self) -> Result<(), EvalError> {
        if !self.captured.as_ref().is_some_and(|c| {
            c.selected == Some(SplitName::Dev) && c.preparation_partitions.is_some()
        }) {
            return Err(EvalError::Invalid(
                "split-isolated dev capture required".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn captured_gates(&self) -> Option<&super::gates::GateConfig> {
        self.captured.as_ref().map(|c| &c.gates)
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
                Ok(c.pool.clone())
            }
            Some(_) => Err(EvalError::Invalid("pool path differs from capture".into())),
            None => read(path),
        }
    }

    fn capture(
        config_path: &Path,
        selected: Option<SplitName>,
    ) -> Result<(Self, [(String, String); 2]), EvalError> {
        let mut budget = MAX_CAPTURE_BYTES;
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
            if path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
                || p.is_empty()
                || p.len() > 4096
                || !unique.insert(path.to_path_buf())
                || *p == config.queries
                || *p == config.qrels
            {
                return Err(EvalError::Invalid(
                    "partition paths must be separate relative files".into(),
                ));
            }
        }
        let split_bytes = captured_read(&dir.join(&config.splits), MAX_INPUT_BYTES, &mut budget)?;
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
        let gates: super::gates::GateConfig = serde_json::from_slice(&gates_bytes)?;
        let pool_path = gates
            .pool
            .as_ref()
            .filter(|p| !p.is_empty())
            .ok_or_else(|| EvalError::Invalid("pool required".into()))?;
        let pool = captured_read(&dir.join(pool_path), MAX_INPUT_BYTES, &mut budget)?;
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
            let qs: Vec<Query> = parse_jsonl(&dir.join(&files.queries), &qb)?;
            let js: Vec<Judgement> = parse_jsonl(&dir.join(&files.qrels), &jb)?;
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
        let freshness_failures = check_current_corpora(&dir, &config, &indexes, &mut budget)?;
        let identity = InputIdentity {
            config: sha256_hex(&config_bytes),
            gates: sha256_hex(&gates_bytes),
            splits: sha256_hex(&split_bytes),
            pool: sha256_hex(&pool),
            indexes: hashes,
        };
        // Combined identity is only for full preparation; execution keeps the
        // exact selected payload digest, not a digest of reserialized records.
        let queries_sha256 = if selected == Some(SplitName::Dev) {
            digests[0].0.clone()
        } else {
            sha256_hex(&serde_json::to_vec(&digests)?)
        };
        let qrels_sha256 = if selected == Some(SplitName::Dev) {
            digests[0].1.clone()
        } else {
            sha256_hex(&serde_json::to_vec(&digests)?)
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
        Ok((bench, digests))
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
pub fn pool(bench: &Bench, out: &Path) -> Result<PoolSummary, EvalError> {
    let indexes = bench.indexes()?;
    let (text, queries, pairs) = pool_captured(bench, &indexes)?;
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
) -> Result<(Vec<u8>, usize, usize), EvalError> {
    validate_config(&bench.config)?;
    validate_queries(&bench.queries, &bench.config)?;
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
pub struct RunMeta {
    pub split: SplitName,
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

/// Apply the complete benchmark preflight before computing or exposing scores.
pub(crate) fn preflight(bench: &Bench) -> Result<(), EvalError> {
    use super::gates::{GateConfig, Stage, check_benchmark};
    let path = bench.dir.join("gates.json");
    let gates: GateConfig = match bench.captured_gates() {
        Some(g) => g.clone(),
        None => serde_json::from_slice(&read(&path)?)?,
    };
    let result = check_benchmark(bench, &gates, Stage::Qrels, None, &bench.dir)?;
    if result.passed {
        Ok(())
    } else {
        Err(EvalError::Invalid(format!(
            "benchmark integrity gates failed: {}",
            serde_json::to_string(&result.failures)?
        )))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    version: u32,
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
        if registration.version != 1 {
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
    let resolved = fs::canonicalize(parent(path))
        .map_err(io_err(parent(path)))?
        .join(
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
    if registration.version != 1
        || registration.frozen.as_bytes() != bytes
        || registration.identity != capture.identity
        || Some(&registration.preparation_partitions) != capture.preparation_partitions.as_ref()
    {
        return Err(EvalError::Invalid(
            "registered freeze or captured inputs differ".into(),
        ));
    }
    let frozen = super::sweep::FrozenConfig::from_toml(&registration.frozen)?;
    if frozen.config_sha256 != bench.config_sha256
        || frozen.queries_sha256 != bench.queries_sha256
        || frozen.qrels_sha256 != bench.qrels()?.1
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
    bench.require_dev_capture()?;
    let capture = bench
        .captured
        .as_ref()
        .ok_or_else(|| EvalError::Invalid("registration requires captured inputs".into()))?;
    let registration = Registration {
        version: 1,
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
    if started.version != 1
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

fn held_out_preflight(bench: &Bench) -> Result<super::sweep::FrozenConfig, EvalError> {
    use super::sweep::{CodeIdentity, check_frozen};
    let frozen = check_held_out_code(bench)?;
    let custody = Custody::resolve(Path::new("."))?;
    check_frozen(&frozen, bench, &CodeIdentity::current(&custody.root)?)?;
    Ok(frozen)
}

// Local custody does not establish independent audit provenance. EVAL-1.3
// still owns measurement admission and held-out capture integration.
pub(crate) fn ensure_measurement_ready() -> Result<(), EvalError> {
    Err(EvalError::Invalid(
        "evaluation disabled pending sealed inputs and an independent audit registry".into(),
    ))
}

fn check_score_split(split: SplitName, confirm_test: bool) -> Result<(), EvalError> {
    if split == SplitName::Test {
        if !confirm_test {
            return Err(EvalError::TestSplitRefused);
        }
        ensure_measurement_ready()?;
    }
    Ok(())
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
    if split == SplitName::Test && !confirm_test {
        return Err(EvalError::TestSplitRefused);
    }
    ensure_measurement_ready()?;
    let attempt = if split == SplitName::Test {
        let custody = Custody::resolve(Path::new("."))?;
        let started = start_attempt(&custody, out)?;
        Some((custody, started))
    } else {
        None
    };
    let result = (|| {
        if split == SplitName::Test {
            check_held_out_code(bench)?;
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
        // Snapshot plumbing alone does not satisfy provenance or attempt custody.
        ensure_measurement_ready()?;
        bench.require_dev_capture()?;
        preflight(bench)?;
        let frozen = if split == SplitName::Test {
            Some(held_out_preflight(bench)?)
        } else {
            None
        };
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
        let stage = StagingDir::create(out)?;
        let code_identity = frozen.map(|f| (f.git_commit, f.binary_sha256));
        let stage_path = stage.path();
        let mut run_sha256 = BTreeMap::new();
        for system in systems {
            let name = system.name();
            let mut text = String::new();
            for q in &queries {
                for (rank, (section, score)) in rank_sections(&indexes[&q.corpus], system, &q.text)
                    .iter()
                    .enumerate()
                {
                    text.push_str(&format!(
                        "{}\tQ0\t{}\t{}\t{}\t{}\n",
                        check_tsv_field(&q.id)?,
                        check_tsv_field(section)?,
                        rank + 1,
                        score,
                        name
                    ));
                }
            }
            let file = format!("{name}.tsv");
            run_sha256.insert(file.clone(), sha256_hex(text.as_bytes()));
            write(&stage_path.join(file), text.as_bytes())?;
        }
        let mut abstention = String::new();
        for q in &queries {
            let ranked = indexes[&q.corpus].search(&q.text, &Params::new(Mode::Lexical, 1));
            abstention.push_str(&format!("{}\t{}\n", q.id, ranked.abstention_score));
        }
        write(&stage_path.join("abstention.tsv"), abstention.as_bytes())?;
        run_sha256.insert("abstention.tsv".into(), sha256_hex(abstention.as_bytes()));

        let meta = RunMeta {
            split,
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
        write(&stage_path.join("report.json"), &report_bytes)?;
        stage.publish(out)?;
        if let Some((custody, started)) = &attempt {
            finish_attempt(custody, started, Some((&meta_bytes, &report_bytes)))?;
        }
        Ok(report)
    })();
    if result.is_err()
        && let Some((custody, started)) = &attempt
    {
        // If terminal publication itself failed its partial file remains
        // occupied. Never replace it or make the start reusable.
        finish_attempt(custody, started, None)?;
    }
    result
}

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
    pub mean: f64,
    pub ci_lo: f64,
    pub ci_hi: f64,
    pub p: f64,
    pub holm_p: Option<f64>,
    pub decision: Option<Decision>,
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
    pub systems: BTreeMap<String, SystemSummary>,
    pub comparisons: Vec<Comparison>,
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
    ensure_measurement_ready()?;
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
    if meta.split == SplitName::Test {
        check_held_out_code(bench)?;
    }
    preflight(bench)?;
    if meta.split == SplitName::Test {
        let frozen = held_out_preflight(bench)?;
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

fn score_captured(
    bench: &Bench,
    run_dir: &Path,
    confirm_test: bool,
    meta: RunMeta,
) -> Result<Report, EvalError> {
    bench.require_dev_capture()?;
    check_score_split(meta.split, confirm_test)?;
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
            Comparison {
                id: sp.id.into(),
                role: sp.role.into(),
                treatment: sp.treatment.clone(),
                baseline: sp.baseline.clone(),
                metric: sp.metric.into(),
                stratum: sp.stratum.map(|s| s.name().to_string()),
                n,
                mean: ci.mean,
                ci_lo: ci.lo,
                ci_hi: ci.hi,
                p,
                holm_p: None,
                decision: None,
            }
        })
        .collect();
    let holm = stats::holm(&[comparisons[1].p, comparisons[2].p]);
    let h1_passed = comparisons[0].n > 0
        && comparisons[0].mean > 0.0
        && comparisons[0].ci_lo > 0.0
        && comparisons[0].p < ALPHA;
    for (i, c) in comparisons.iter_mut().enumerate().take(3) {
        let p = if i == 0 { c.p } else { holm[i - 1] };
        if i > 0 && h1_passed {
            c.holm_p = Some(p);
        }
        if i > 0 && !h1_passed {
            continue;
        }
        let ci = Interval {
            mean: c.mean,
            lo: c.ci_lo,
            hi: c.ci_hi,
        };
        c.decision = Some(if c.n == 0 || (i > 0 && !h1_passed) {
            Decision::Inconclusive
        } else {
            stats::decide(ci, p, ALPHA, MIN_EFFECT)
        });
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
        systems,
        comparisons,
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
            version: 1,
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
        let huge = fs::OpenOptions::new()
            .write(true)
            .open(custody.dir.join("registration.json"))
            .unwrap();
        huge.set_len(MAX_INPUT_BYTES as u64 + 1).unwrap();
        assert!(custody.registration().is_err());
    }

    #[test]
    fn registration_binds_exact_export_and_admitted_capture() {
        let dir = fixture();
        let config = dir.path().join("config.json");
        let evidence = Bench::prepare(&config).unwrap();
        let mut bench = Bench::load_dev(&config, &evidence).unwrap();
        let frozen = super::super::sweep::FrozenConfig {
            alpha: bench.config.alpha,
            beta: bench.config.beta,
            threshold: bench.config.threshold,
            seed: bench.config.seed,
            resamples: bench.config.resamples,
            config_sha256: bench.config_sha256.clone(),
            queries_sha256: bench.queries_sha256.clone(),
            qrels_sha256: bench.qrels().unwrap().1,
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
            version: 1,
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
        registration.version = 2;
        assert!(check_registration(&registration, &bench, text.as_bytes()).is_err());

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
        let bench = Bench::load(&dir.path().join("config.json")).unwrap();
        pool(&bench, &dir.path().join("pool.jsonl")).unwrap();
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
        let evidence = Bench::prepare(&path).unwrap();
        let second = Bench::prepare(&path).unwrap();
        assert_eq!(evidence.partitions, second.partitions);
        for name in [
            "test-queries.jsonl",
            "test-qrels.jsonl",
            "queries.jsonl",
            "qrels.jsonl",
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
            "pool.jsonl",
        ] {
            fs::remove_file(dir.path().join(name)).unwrap();
        }
        fs::remove_dir_all(dir.path().join("corpus")).unwrap();
        assert_eq!(super::super::sweep::sweep_captured(&dev).unwrap(), report);
        assert!(super::super::gates::check_qrels(&dev).unwrap().is_empty());
        assert!(sweep_refused(&dev));
    }

    fn sweep_refused(bench: &Bench) -> bool {
        super::super::sweep::sweep(bench, SplitName::Dev)
            .unwrap_err()
            .to_string()
            .contains("evaluation disabled")
    }

    #[test]
    fn dev_admission_rejects_changed_identities_and_stale_corpus() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = Bench::prepare(&path).unwrap();
        for name in [
            "config.json",
            "gates.json",
            "splits.json",
            "pool.jsonl",
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
        let evidence = Bench::prepare(&path).unwrap();
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
        assert!(Bench::prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        js = read_jsonl::<Judgement>(&qpath).unwrap().0;
        js.push(js[0].clone());
        write_lines(&qpath, &js);
        assert!(Bench::prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        js = read_jsonl::<Judgement>(&qpath).unwrap().0;
        js[0].query_id = "unknown".into();
        write_lines(&qpath, &js);
        assert!(Bench::prepare(&path).is_err());
        fs::write(&qpath, &original).unwrap();
        let evidence = Bench::prepare(&path).unwrap();
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
        let evidence = Bench::prepare(&path).unwrap();
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
    fn dev_capture_retains_complete_preparation_identity() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let first = Bench::prepare(&path).unwrap();
        let label_path = dir.path().join("test-qrels.jsonl");
        let (mut labels, _): (Vec<Judgement>, _) = read_jsonl(&label_path).unwrap();
        labels[0].grade = 2;
        write_lines(&label_path, &labels);
        let second = Bench::prepare(&path).unwrap();
        let a = Bench::load_dev(&path, &first).unwrap();
        let b = Bench::load_dev(&path, &second).unwrap();
        assert_eq!(a.queries_sha256, b.queries_sha256);
        assert_ne!(
            a.captured.as_ref().unwrap().preparation_partitions,
            b.captured.as_ref().unwrap().preparation_partitions
        );
    }

    #[test]
    fn captured_scoring_checks_and_parses_identical_saved_bytes() {
        let dir = fixture();
        let path = dir.path().join("config.json");
        let evidence = Bench::prepare(&path).unwrap();
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
            split: SplitName::Dev,
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
        assert_eq!(report.answerable, 75);
        assert_eq!(report.systems["B1"].mean.ndcg10, 1.0);
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
