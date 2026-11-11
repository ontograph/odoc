//! Application layer shared by the CLI and the MCP server.
//!
//! Every response is typed, deterministic JSON, bounded by
//! [`MAX_RESPONSE_BYTES`] with an explicit `truncated` flag, and reports
//! whether the corpus still matches the active generation (`fresh`).

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::assemble::{self, TierLevel, TierTexts};
use crate::cite::{self, Citation, CitationState};
use crate::corpus::{CorpusError, RootedCorpus, WalkConfig};
use crate::search::{AbstainReason, Components, Mode, Params, SearchIndex};
use crate::store::{
    self, Freshness, Generation, IndexConfig, LinkDiagnosticRecord, SectionRecord, SkipEntry,
    Store, StoreError,
};

pub const MAX_TOP_K: usize = 50;
pub const MAX_RESPONSE_BYTES: usize = 128 * 1024;
pub const MAX_LIST_ITEMS: usize = 100;
pub(crate) const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_QUERY_BYTES: usize = 4096;
pub(crate) const MAX_ID_BYTES: usize = 16 * 1024;
/// Provisional fusion weights; the frozen evaluation config replaces them.
pub const DEFAULT_ALPHA: f64 = 0.3;
pub const DEFAULT_BETA: f64 = 0.3;

#[derive(Debug, Parser)]
#[command(
    name = "ontodoc",
    version,
    about = "Documentation retrieval with cited, measured results"
)]
pub struct Cli {
    /// Corpus root; the index lives in `<root>/.ontodoc`.
    #[arg(long, global = true, default_value = ".")]
    pub root: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build and publish a new index generation.
    Index,
    /// Search the active generation.
    Search(SearchRequest),
    /// Read a chunk or section by id or `path#anchor`.
    Read { id: String },
    /// List graph neighbours of a chunk.
    Related { chunk_id: String },
    /// Verify citations given as a JSON object or array (`-` reads stdin).
    Verify { citations: String },
    /// Show generation, counts, freshness and diagnostics.
    Status,
    /// Serve the read-only MCP tools over stdio.
    Serve,
    /// Benchmark harness.
    #[command(subcommand)]
    Eval(EvalCommand),
}

#[derive(Debug, Subcommand)]
pub enum EvalCommand {
    /// Write the stratified dev/test split for the config's queries.
    Split {
        config: PathBuf,
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the judgement pool (top 10 of every system at every grid point).
    Pool {
        config: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Version-1 query-review records for every final query.
        #[arg(long)]
        query_review: PathBuf,
        /// Operator-audited SHA-256 of the query-review bytes.
        #[arg(long)]
        query_review_sha256: String,
    },
    /// Run all systems on one split and write runs plus report.
    Run {
        config: PathBuf,
        #[arg(long, value_enum)]
        split: crate::eval::run::SplitName,
        #[arg(long)]
        out: PathBuf,
        /// Required for the held-out test split.
        #[arg(long)]
        confirm_test_split: bool,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
    /// Re-score a saved run directory.
    Score {
        config: PathBuf,
        run_dir: PathBuf,
        #[arg(long)]
        confirm_test_split: bool,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
    /// Run the integrity gates that apply at a stage; exits non-zero on failure.
    CheckBenchmark {
        config: PathBuf,
        #[arg(long, value_enum)]
        stage: crate::eval::gates::Stage,
        /// Gate settings (floors, corpus pins, expected pool) as JSON.
        #[arg(long)]
        gates: Option<PathBuf>,
        /// Run directory whose report is checked at stage `run`.
        #[arg(long)]
        run: Option<PathBuf>,
        /// Version-1 query-review records; absent approval fails the gates.
        #[arg(long, requires = "query_review_sha256")]
        query_review: Option<PathBuf>,
        /// Operator-audited SHA-256 of the query-review bytes.
        #[arg(long, requires = "query_review")]
        query_review_sha256: Option<String>,
        /// Version-1 human annotation records; required at stage `qrels`.
        #[arg(long, requires = "annotation_notes")]
        annotation: Option<PathBuf>,
        /// `annotation.md` with one fenced `annotation-notes` JSON block.
        #[arg(long, requires = "annotation")]
        annotation_notes: Option<PathBuf>,
        /// Publish version-1 `preparation.json` (create-new) after every
        /// qrels-stage gate passes; prints its SHA-256 for the operator audit.
        #[arg(long, requires_all = ["query_review", "annotation", "operator", "operator_evidence"])]
        preparation_out: Option<PathBuf>,
        /// Approving operator recorded in the preparation manifest.
        #[arg(long, requires = "preparation_out")]
        operator: Option<String>,
        /// Operator evidence reference recorded in the preparation manifest.
        #[arg(long, requires = "preparation_out")]
        operator_evidence: Option<String>,
    },
    /// Dev-only grid sweep with per-arm sd of differences and required n.
    Sweep {
        config: PathBuf,
        #[arg(long, value_enum)]
        split: crate::eval::run::SplitName,
        #[arg(long)]
        out: PathBuf,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
    /// Write `frozen-config.toml` from a dev sweep and the clean code identity.
    Freeze {
        config: PathBuf,
        /// Abstention threshold fixed on dev.
        #[arg(long)]
        threshold: f64,
        /// Repository whose commit and Cargo.lock are frozen.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        out: PathBuf,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
    /// Check a frozen config against the current code and benchmark.
    CheckFrozen {
        config: PathBuf,
        frozen: PathBuf,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
    /// Attribute every judged-relevant miss of a dev run to one pipeline stage.
    Misses {
        config: PathBuf,
        run_dir: PathBuf,
        /// Operator-approved `preparation.json` admitting this execution.
        #[arg(long, requires = "preparation_sha256")]
        preparation: Option<PathBuf>,
        /// Out-of-band audit pin: SHA-256 of the exact manifest bytes.
        #[arg(long, requires = "preparation")]
        preparation_sha256: Option<String>,
    },
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// BM25 (B1).
    #[default]
    Lexical,
    /// BM25 fused with Personalized PageRank over links and reading order (B2).
    Graph,
    /// BM25 fused with the document-level score (B3).
    Hier,
}

#[derive(Debug, Clone, Args, Deserialize, JsonSchema)]
pub struct SearchRequest {
    /// Free-text query.
    pub query: String,
    /// Ranking mode.
    #[arg(long, value_enum, default_value_t = SearchMode::Lexical)]
    #[serde(default)]
    pub mode: SearchMode,
    /// Maximum number of results, 1 to 50.
    #[arg(long, default_value_t = 10)]
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    /// Only return chunks whose path starts with this prefix.
    #[arg(long)]
    #[serde(default)]
    pub scope: Option<String>,
    /// Include score components for each result.
    #[arg(long)]
    #[serde(default)]
    pub explain: bool,
    /// Byte budget for breadth-first context assembly (L0, then L1, then L2).
    #[arg(long)]
    #[serde(default)]
    pub budget_bytes: Option<usize>,
    /// Graph-mode PPR weight in [0, 1].
    #[arg(long)]
    #[serde(default)]
    pub alpha: Option<f64>,
    /// Hier-mode document weight in [0, 1].
    #[arg(long)]
    #[serde(default)]
    pub beta: Option<f64>,
    /// Abstain when top BM25 / sum(idf) is below this; disabled when absent.
    #[arg(long)]
    #[serde(default)]
    pub threshold: Option<f64>,
}

fn default_top_k() -> usize {
    10
}

pub(crate) fn check_input_limit(
    len: usize,
    cap: usize,
    what: &'static str,
) -> Result<(), AppError> {
    if len > cap {
        return Err(StoreError::CapExceeded { what, cap }.into());
    }
    Ok(())
}

fn add_argument_bytes(total: &mut usize, bytes: usize) -> Result<(), AppError> {
    let next = total.checked_add(bytes).ok_or(StoreError::CapExceeded {
        what: "argument bytes",
        cap: MAX_ARGUMENT_BYTES,
    })?;
    check_input_limit(next, MAX_ARGUMENT_BYTES, "argument bytes")?;
    *total = next;
    Ok(())
}

/// Count all decoded strings, including unknown values and object keys.
pub(crate) fn validate_json_strings(
    value: &serde_json::Value,
    total: &mut usize,
) -> Result<(), AppError> {
    match value {
        serde_json::Value::String(text) => add_argument_bytes(total, text.len())?,
        serde_json::Value::Array(values) => {
            for value in values {
                validate_json_strings(value, total)?;
            }
        }
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                add_argument_bytes(total, key.len())?;
                validate_json_strings(value, total)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn validate_citation_fields(
    generation_id: &str,
    path: &str,
    heading_path: &[String],
    sha256: &str,
) -> Result<usize, AppError> {
    check_input_limit(path.len(), MAX_QUERY_BYTES, "citation path bytes")?;
    check_input_limit(
        heading_path.len(),
        crate::store::MAX_CITATION_HEADINGS,
        "citation heading count",
    )?;
    let mut headings = 0usize;
    for heading in heading_path {
        headings = headings
            .checked_add(heading.len())
            .ok_or(StoreError::CapExceeded {
                what: "citation heading bytes",
                cap: crate::store::MAX_CITATION_HEADING_BYTES,
            })?;
        check_input_limit(
            headings,
            crate::store::MAX_CITATION_HEADING_BYTES,
            "citation heading bytes",
        )?;
    }
    for value in [generation_id, sha256] {
        validate_citation_hash(value)?;
    }
    // Each term has already been bounded above, so this cannot overflow.
    Ok(generation_id.len() + path.len() + headings + sha256.len())
}

fn validate_citation_hash(value: &str) -> Result<(), AppError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AppError::InvalidArgument(
            "citation hashes must be 64 lowercase hex bytes".into(),
        ));
    }
    Ok(())
}

// Check limits in borrowed JSON before allocating typed citation vectors.
// Missing/wrongly typed fields remain the normal deserializer's responsibility.
pub(crate) fn validate_citation_json(value: &serde_json::Value) -> Result<(), AppError> {
    if let Some(path) = value.get("path").and_then(serde_json::Value::as_str) {
        check_input_limit(path.len(), MAX_QUERY_BYTES, "citation path bytes")?;
    }
    if let Some(headings) = value
        .get("heading_path")
        .and_then(serde_json::Value::as_array)
    {
        check_input_limit(headings.len(), 64, "citation heading count")?;
        let mut bytes = 0;
        for heading in headings.iter().filter_map(serde_json::Value::as_str) {
            add_argument_bytes(&mut bytes, heading.len())?;
            check_input_limit(bytes, MAX_ID_BYTES, "citation heading bytes")?;
        }
    }
    for field in ["generation_id", "sha256"] {
        if let Some(hash) = value.get(field).and_then(serde_json::Value::as_str) {
            validate_citation_hash(hash)?;
        }
    }
    Ok(())
}

fn validate_citations(citations: &[Citation]) -> Result<(), AppError> {
    check_input_limit(citations.len(), MAX_LIST_ITEMS, "citation count")?;
    let mut total = 0;
    for citation in citations {
        add_argument_bytes(
            &mut total,
            validate_citation_fields(
                &citation.generation_id,
                &citation.path,
                &citation.heading_path,
                &citation.sha256,
            )?,
        )?;
    }
    Ok(())
}

impl SearchRequest {
    pub(crate) fn validate(&self) -> Result<Params, AppError> {
        check_input_limit(self.query.len(), MAX_QUERY_BYTES, "query bytes")?;
        if let Some(scope) = &self.scope {
            check_input_limit(scope.len(), MAX_QUERY_BYTES, "scope bytes")?;
        }
        if self.top_k == 0 || self.top_k > MAX_TOP_K {
            return Err(AppError::InvalidArgument(format!(
                "top_k must be 1..={MAX_TOP_K}"
            )));
        }
        let weight = |name: &str, value: Option<f64>, default: f64| match value.unwrap_or(default) {
            w if (0.0..=1.0).contains(&w) => Ok(w),
            _ => Err(AppError::InvalidArgument(format!(
                "{name} must be in [0, 1]"
            ))),
        };
        let mode = match self.mode {
            SearchMode::Lexical => Mode::Lexical,
            SearchMode::Graph => Mode::Graph {
                alpha: weight("alpha", self.alpha, DEFAULT_ALPHA)?,
            },
            SearchMode::Hier => Mode::Hier {
                beta: weight("beta", self.beta, DEFAULT_BETA)?,
            },
        };
        if self.threshold.is_some_and(|t| !t.is_finite()) {
            return Err(AppError::InvalidArgument("threshold must be finite".into()));
        }
        Ok(Params {
            mode,
            top_k: self.top_k,
            scope: self.scope.clone(),
            threshold: self.threshold,
            explain: self.explain,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("no published generation under {0:?}; run `ontodoc index`")]
    NoIndex(PathBuf),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("citation generation is not the active generation")]
    GenerationUnavailable,
    #[error("citation is not an indexed chunk or section")]
    InvalidCitation,
    #[error("corpus filesystem is unavailable")]
    CorpusUnavailable(&'static str),
    #[error(transparent)]
    Store(StoreError),
    #[error("serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Eval(#[from] crate::eval::run::EvalError),
    #[error("integrity gates failed: {0}")]
    GatesFailed(String),
    #[error("response exceeds the byte limit")]
    ResponseTooLarge,
    #[error("service is busy")]
    Busy,
    #[error("internal operation failed")]
    Internal,
}

impl From<StoreError> for AppError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl AppError {
    /// Stable machine-readable error kind.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NoIndex(_) => "no_index",
            Self::NotFound(_) => "not_found",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::GenerationUnavailable => "generation_unavailable",
            Self::InvalidCitation => "invalid_citation",
            Self::CorpusUnavailable(kind) => kind,
            Self::Store(StoreError::Locked) => "locked",
            Self::Store(StoreError::Corrupt { .. }) => "corrupt",
            Self::Store(StoreError::CapExceeded { .. }) => "cap_exceeded",
            Self::Store(StoreError::Corpus(e)) => corpus_error_kind(e),
            Self::Store(_) => "io",
            Self::Json(_) => "internal",
            Self::Internal => "internal",
            Self::ResponseTooLarge => "cap_exceeded",
            Self::Busy => "busy",
            Self::Eval(crate::eval::run::EvalError::TestSplitRefused) => "test_split_refused",
            Self::Eval(_) => "eval",
            Self::GatesFailed(_) => "gates_failed",
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({ "error": self.kind(), "message": self.public_message() }).to_string()
    }

    /// Internal source errors may include private paths and raw OS diagnostics.
    pub(crate) fn public_message(&self) -> String {
        match self {
            Self::NoIndex(_) => "No published generation; run `ontodoc index`".into(),
            Self::Store(StoreError::CapExceeded { .. }) => {
                "Operation exceeds a resource limit".into()
            }
            Self::Store(_) | Self::Json(_) | Self::Internal => "Internal operation failed".into(),
            _ => self.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexReport {
    pub generation_id: String,
    /// This generation became CURRENT under the publication lock.
    pub committed: bool,
    pub durability: store::Durability,
    pub maintenance: store::Maintenance,
    pub failure_phase: Option<store::PublishPhase>,
    pub warnings: Vec<store::PublishPhase>,
    pub documents: usize,
    pub sections: usize,
    pub chunks: usize,
    pub edges: usize,
    pub skipped: usize,
    pub link_diagnostics: usize,
}

impl IndexReport {
    fn published(g: &store::Generation, outcome: store::PublishOutcome) -> Self {
        Self {
            generation_id: outcome.generation_id,
            committed: true,
            durability: outcome.durability,
            maintenance: outcome.maintenance,
            failure_phase: outcome.failure_phase,
            warnings: outcome.failure_phase.into_iter().collect(),
            documents: g.documents.len(),
            sections: g.sections.len(),
            chunks: g.chunks.len(),
            edges: g.edges.len(),
            skipped: g.skipped.len(),
            link_diagnostics: g.link_diagnostics.len(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub rank: usize,
    pub chunk_id: String,
    pub section_id: String,
    pub score: f64,
    /// The cited file changed since indexing.
    pub stale: bool,
    pub tier: TierLevel,
    pub text: String,
    pub citation: Citation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<Components>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub generation_id: String,
    pub fresh: bool,
    pub mode: SearchMode,
    pub abstained: Option<AbstainReason>,
    pub abstention_score: f64,
    pub results: Vec<SearchResult>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct ReadResponse {
    pub generation_id: String,
    pub fresh: bool,
    pub stale: bool,
    /// `chunk` or `section`.
    pub kind: &'static str,
    pub id: String,
    pub citation: Citation,
    pub text: String,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct Neighbour {
    pub kind: String,
    /// `out` (this chunk is the source) or `in`.
    pub direction: &'static str,
    pub id: String,
}

#[derive(Debug, Serialize)]
pub struct RelatedResponse {
    pub generation_id: String,
    pub fresh: bool,
    pub chunk_id: String,
    pub neighbours: Vec<Neighbour>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifyResult {
    pub citation: Citation,
    #[serde(flatten)]
    pub state: CitationState,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub results: Vec<VerifyResult>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub generation_id: String,
    pub schema_version: u32,
    pub documents: usize,
    pub sections: usize,
    pub chunks: usize,
    pub edges: usize,
    pub fresh: bool,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
    pub skipped_count: usize,
    pub skipped: Vec<SkipEntry>,
    pub link_diagnostic_count: usize,
    pub link_diagnostics: Vec<LinkDiagnosticRecord>,
    pub truncated: bool,
}

/// Drops trailing list items until the serialized response fits.
pub(crate) trait Bounded: Serialize {
    fn pop_item(&mut self) -> bool;
    fn mark_truncated(&mut self);
}

fn bound<T: Bounded>(mut response: T) -> Result<T, AppError> {
    while serde_json::to_vec(&response)?.len() > MAX_RESPONSE_BYTES {
        if !response.pop_item() {
            return Err(AppError::ResponseTooLarge);
        }
        response.mark_truncated();
    }
    Ok(response)
}

impl Bounded for SearchResponse {
    fn pop_item(&mut self) -> bool {
        self.results.pop().is_some()
    }
    fn mark_truncated(&mut self) {
        self.truncated = true;
    }
}

impl Bounded for ReadResponse {
    fn pop_item(&mut self) -> bool {
        if self.text.is_empty() {
            return false;
        }
        let mut cut = self.text.len() / 2;
        while !self.text.is_char_boundary(cut) {
            cut -= 1;
        }
        self.text.truncate(cut);
        true
    }
    fn mark_truncated(&mut self) {
        self.truncated = true;
    }
}

impl Bounded for RelatedResponse {
    fn pop_item(&mut self) -> bool {
        self.neighbours.pop().is_some()
    }
    fn mark_truncated(&mut self) {
        self.truncated = true;
    }
}

impl Bounded for VerifyResponse {
    fn pop_item(&mut self) -> bool {
        self.results.pop().is_some()
    }
    fn mark_truncated(&mut self) {
        self.truncated = true;
    }
}

impl Bounded for StatusResponse {
    fn pop_item(&mut self) -> bool {
        self.link_diagnostics.pop().is_some()
            || self.skipped.pop().is_some()
            || self.added.pop().is_some()
            || self.removed.pop().is_some()
            || self.modified.pop().is_some()
    }
    fn mark_truncated(&mut self) {
        self.truncated = true;
    }
}

fn capped<T>(mut items: Vec<T>, truncated: &mut bool) -> Vec<T> {
    if items.len() > MAX_LIST_ITEMS {
        items.truncate(MAX_LIST_ITEMS);
        *truncated = true;
    }
    items
}

/// Like [`capped`], but takes or clones at most `MAX_LIST_ITEMS + 1` items,
/// so large graphs or diagnostic lists are never collected in full.
fn capped_iter<T>(items: impl Iterator<Item = T>, truncated: &mut bool) -> Vec<T> {
    capped(items.take(MAX_LIST_ITEMS + 1).collect(), truncated)
}

/// One corpus root and its generation store.
pub struct App {
    pub root: PathBuf,
    store: Store,
    rooted: Result<RootedCorpus, CorpusError>,
}

impl App {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            store: Store::new(root.join(".ontodoc")),
            rooted: RootedCorpus::open(root),
        }
    }

    pub fn index(&self) -> Result<IndexReport, AppError> {
        let g = store::build_rooted(self.rooted()?, &IndexConfig::default())?;
        let outcome = self.store.publish(&g)?;
        Ok(IndexReport::published(&g, outcome))
    }

    /// Id of the active generation.
    pub fn current_id(&self) -> Result<String, AppError> {
        self.store
            .current_id_rooted(self.rooted()?)
            .map_err(|e| self.no_index(e))
    }

    /// Loads and verifies the active generation.
    pub fn load_index(&self) -> Result<SearchIndex, AppError> {
        self.load_index_for(&self.current_id()?)
    }

    pub(crate) fn load_index_for(&self, id: &str) -> Result<SearchIndex, AppError> {
        let g = self
            .store
            .load_id_rooted(self.rooted()?, id)
            .map_err(|e| self.no_index(e))?;
        Ok(SearchIndex::try_new(g)?)
    }

    fn rooted(&self) -> Result<&RootedCorpus, AppError> {
        self.rooted
            .as_ref()
            .map_err(|e| AppError::CorpusUnavailable(corpus_error_kind(e)))
    }

    fn no_index(&self, e: StoreError) -> AppError {
        match e {
            StoreError::NoGeneration => AppError::NoIndex(self.root.clone()),
            e => AppError::Store(e),
        }
    }

    fn freshness(&self, g: &Generation) -> Result<Freshness, AppError> {
        Ok(store::freshness_rooted(self.rooted()?, g)?)
    }

    pub fn search(
        &self,
        index: &SearchIndex,
        req: &SearchRequest,
    ) -> Result<SearchResponse, AppError> {
        let params = req.validate()?;
        let ranked = index.search(&req.query, &params);
        let g = &index.generation;
        let freshness = self.freshness(g)?;
        let stale = stale_paths(&freshness);

        let tier_texts: Vec<TierTexts<'_>> = ranked
            .hits
            .iter()
            .map(|h| {
                let c = &g.chunks[h.chunk];
                TierTexts {
                    l0: &c.l0,
                    l1: &c.l1,
                    l2: g.chunk_text(c).unwrap_or(""),
                }
            })
            .collect();
        let tiers = match req.budget_bytes {
            Some(budget) => assemble::assemble(&tier_texts, budget).tiers,
            None => vec![Some(TierLevel::L2); tier_texts.len()],
        };
        let mut truncated = false;
        let mut results = Vec::new();
        for (i, (hit, tier)) in ranked.hits.iter().zip(tiers).enumerate() {
            let Some(tier) = tier else {
                truncated = true;
                continue;
            };
            let c = &g.chunks[hit.chunk];
            let text = match tier {
                TierLevel::L0 => tier_texts[i].l0,
                TierLevel::L1 => tier_texts[i].l1,
                TierLevel::L2 => tier_texts[i].l2,
            };
            results.push(SearchResult {
                rank: i + 1,
                chunk_id: c.id.clone(),
                section_id: c.section_id.clone(),
                score: hit.score,
                stale: stale.contains(c.path.as_str()),
                tier,
                text: text.to_string(),
                citation: Citation::for_chunk(g, hit.chunk)
                    .ok_or_else(|| AppError::NotFound(c.id.clone()))?,
                explain: ranked.explain.get(i).copied(),
            });
        }
        bound(SearchResponse {
            generation_id: g.generation_id.clone(),
            fresh: freshness.is_fresh(),
            mode: req.mode,
            abstained: ranked.abstained,
            abstention_score: ranked.abstention_score,
            results,
            truncated,
        })
    }

    pub fn read(&self, index: &SearchIndex, id: &str) -> Result<ReadResponse, AppError> {
        check_input_limit(id.len(), MAX_ID_BYTES, "read id bytes")?;
        let g = &index.generation;
        let (kind, record_id, citation, path) =
            if let Some(i) = g.chunks.iter().position(|c| c.id == id) {
                let c = &g.chunks[i];
                let citation =
                    Citation::for_chunk(g, i).ok_or_else(|| AppError::NotFound(id.to_string()))?;
                ("chunk", c.id.clone(), citation, c.path.clone())
            } else {
                let s = find_section(g, id).ok_or_else(|| AppError::NotFound(id.to_string()))?;
                (
                    "section",
                    s.id.clone(),
                    Citation::for_section(g, s),
                    s.path.clone(),
                )
            };
        let doc = g
            .documents
            .iter()
            .find(|d| d.path == path)
            .ok_or_else(|| AppError::NotFound(path.clone()))?;
        let full = doc
            .text
            .get(citation.byte_start..citation.byte_end)
            .unwrap_or("");
        let mut cut = full.len().min(MAX_RESPONSE_BYTES / 2);
        while !full.is_char_boundary(cut) {
            cut -= 1;
        }
        let freshness = self.freshness(g)?;
        bound(ReadResponse {
            generation_id: g.generation_id.clone(),
            fresh: freshness.is_fresh(),
            stale: stale_paths(&freshness).contains(path.as_str()),
            kind,
            id: record_id,
            citation,
            text: full[..cut].to_string(),
            truncated: cut < full.len(),
        })
    }

    pub fn related(
        &self,
        index: &SearchIndex,
        chunk_id: &str,
    ) -> Result<RelatedResponse, AppError> {
        check_input_limit(chunk_id.len(), MAX_ID_BYTES, "related id bytes")?;
        let g = &index.generation;
        if !g.chunks.iter().any(|c| c.id == chunk_id) {
            return Err(AppError::NotFound(chunk_id.to_string()));
        }
        let mut truncated = false;
        let neighbours = capped_iter(
            g.edges
                .iter()
                .filter_map(|e| {
                    if e.from == chunk_id {
                        Some((e, "out", &e.to))
                    } else if e.to == chunk_id {
                        Some((e, "in", &e.from))
                    } else {
                        None
                    }
                })
                .map(|(e, direction, id)| Neighbour {
                    kind: e.kind.clone(),
                    direction,
                    id: id.clone(),
                }),
            &mut truncated,
        );
        bound(RelatedResponse {
            generation_id: g.generation_id.clone(),
            fresh: self.freshness(g)?.is_fresh(),
            chunk_id: chunk_id.to_string(),
            neighbours,
            truncated,
        })
    }

    pub fn verify(&self, citations: &[Citation]) -> Result<VerifyResponse, AppError> {
        validate_citations(citations)?;
        self.verify_index(&self.load_index()?, citations)
    }

    pub(crate) fn verify_index(
        &self,
        index: &SearchIndex,
        citations: &[Citation],
    ) -> Result<VerifyResponse, AppError> {
        validate_citations(citations)?;
        let cap = index
            .generation
            .config
            .max_file_bytes
            .min(WalkConfig::default().max_file_bytes);
        let states = cite::verify(self.rooted()?, &index.generation, citations, cap).map_err(
            |e| match e {
                cite::VerifyError::GenerationUnavailable => AppError::GenerationUnavailable,
                cite::VerifyError::InvalidCitation => AppError::InvalidCitation,
                cite::VerifyError::Filesystem(e) => AppError::Store(e.into()),
            },
        )?;
        let results = citations
            .iter()
            .cloned()
            .zip(states)
            .map(|(citation, state)| VerifyResult { citation, state })
            .collect();
        bound(VerifyResponse {
            results,
            truncated: false,
        })
    }

    pub fn status(&self, index: &SearchIndex) -> Result<StatusResponse, AppError> {
        let g = &index.generation;
        let f = self.freshness(g)?;
        let mut truncated = false;
        bound(StatusResponse {
            generation_id: g.generation_id.clone(),
            schema_version: g.config.schema_version,
            documents: g.documents.len(),
            sections: g.sections.len(),
            chunks: g.chunks.len(),
            edges: g.edges.len(),
            fresh: f.is_fresh(),
            added: capped(f.added, &mut truncated),
            removed: capped(f.removed, &mut truncated),
            modified: capped(f.modified, &mut truncated),
            skipped_count: g.skipped.len(),
            skipped: capped_iter(g.skipped.iter().cloned(), &mut truncated),
            link_diagnostic_count: g.link_diagnostics.len(),
            link_diagnostics: capped_iter(g.link_diagnostics.iter().cloned(), &mut truncated),
            truncated,
        })
    }
}

fn stale_paths(f: &Freshness) -> HashSet<&str> {
    f.modified
        .iter()
        .chain(&f.removed)
        .map(String::as_str)
        .collect()
}

/// Section by exact id, or by `path#anchor`.
fn find_section<'a>(g: &'a Generation, id: &str) -> Option<&'a SectionRecord> {
    if let Some(s) = g.sections.iter().find(|s| s.id == id) {
        return Some(s);
    }
    let (path, anchor) = id.split_once('#')?;
    let anchor = anchor.to_lowercase();
    g.sections
        .iter()
        .find(|s| s.path == path && s.level > 0 && s.anchor == anchor)
}

fn corpus_error_kind(e: &CorpusError) -> &'static str {
    match e {
        CorpusError::UnsupportedFilesystem => "unsupported_filesystem",
        CorpusError::FilesystemChanged => "filesystem_changed",
        CorpusError::TooLarge { .. }
        | CorpusError::CaptureBudget(_)
        | CorpusError::TooManyFiles { .. } => "cap_exceeded",
        _ => "io",
    }
}

/// Parses one citation object or an array of them.
pub fn parse_citations(text: &str) -> Result<Vec<Citation>, AppError> {
    check_input_limit(text.len(), MAX_RESPONSE_BYTES, "citation JSON bytes")?;
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| AppError::InvalidArgument(e.to_string()))?;
    validate_json_strings(&value, &mut 0)?;
    if let Some(values) = value.as_array() {
        check_input_limit(values.len(), MAX_LIST_ITEMS, "citation count")?;
        for citation in values {
            validate_citation_json(citation)?;
        }
    } else {
        validate_citation_json(&value)?;
    }
    let parsed = if value.is_array() {
        serde_json::from_value(value)
    } else {
        serde_json::from_value(value).map(|c| vec![c])
    };
    let citations = parsed.map_err(|e| AppError::InvalidArgument(e.to_string()))?;
    validate_citations(&citations)?;
    Ok(citations)
}

/// Runs one CLI command and returns its JSON output.
pub fn run(cli: Cli) -> Result<String, AppError> {
    let app = App::new(&cli.root);
    let json = match cli.command {
        Command::Index => serde_json::to_string_pretty(&app.index()?)?,
        Command::Search(req) => {
            req.validate()?;
            serde_json::to_string_pretty(&app.search(&app.load_index()?, &req)?)?
        }
        Command::Read { id } => {
            check_input_limit(id.len(), MAX_ID_BYTES, "read id bytes")?;
            serde_json::to_string_pretty(&app.read(&app.load_index()?, &id)?)?
        }
        Command::Related { chunk_id } => {
            check_input_limit(chunk_id.len(), MAX_ID_BYTES, "related id bytes")?;
            serde_json::to_string_pretty(&app.related(&app.load_index()?, &chunk_id)?)?
        }
        Command::Verify { citations } => {
            let text = if citations == "-" {
                let mut buf = String::new();
                std::io::stdin()
                    .take(MAX_RESPONSE_BYTES as u64 + 1)
                    .read_to_string(&mut buf)
                    .map_err(|e| AppError::InvalidArgument(e.to_string()))?;
                buf
            } else {
                citations
            };
            serde_json::to_string_pretty(&app.verify(&parse_citations(&text)?)?)?
        }
        Command::Status => serde_json::to_string_pretty(&app.status(&app.load_index()?)?)?,
        Command::Eval(cmd) => run_eval(cmd)?,
        Command::Serve => {
            return Err(AppError::InvalidArgument(
                "`serve` is handled by the binary entry point".into(),
            ));
        }
    };
    Ok(json)
}

fn run_eval(cmd: EvalCommand) -> Result<String, AppError> {
    // Reject unavailable operations before Bench::load opens combined inputs
    // or a saved run exposes held-out observations.
    use crate::eval::gates::Stage;
    use crate::eval::run::{SplitName, admission_error};
    match &cmd {
        EvalCommand::Run {
            split: SplitName::Test,
            confirm_test_split: false,
            ..
        } => return Err(crate::eval::run::EvalError::TestSplitRefused.into()),
        EvalCommand::Sweep {
            split: SplitName::Test,
            ..
        } => {
            return Err(crate::eval::run::EvalError::Invalid(
                "sweep runs on the dev split only".into(),
            )
            .into());
        }
        EvalCommand::Run {
            preparation: None, ..
        }
        | EvalCommand::Score {
            preparation: None, ..
        }
        | EvalCommand::Sweep {
            preparation: None, ..
        }
        | EvalCommand::Freeze {
            preparation: None, ..
        }
        | EvalCommand::CheckFrozen {
            preparation: None, ..
        }
        | EvalCommand::Misses {
            preparation: None, ..
        } => return Err(admission_error().into()),
        // Saved-run gates belong to admitted `score`, which checks judged@10.
        EvalCommand::CheckBenchmark { stage, run, .. } if *stage == Stage::Run || run.is_some() => {
            return Err(admission_error().into());
        }
        _ => {}
    }
    use crate::eval::run::{self as er, Bench};
    // Presence of both is enforced above and by clap `requires`.
    let pinned = |config: &Path, p: Option<PathBuf>, pin: Option<String>| match (p, pin) {
        (Some(p), Some(pin)) => Ok(Bench::load_dev_pinned(config, &p, &pin)?),
        _ => Err(AppError::from(admission_error())),
    };
    fn pretty<T: Serialize>(v: &T) -> Result<String, AppError> {
        Ok(serde_json::to_string_pretty(v)?)
    }
    let json = match cmd {
        EvalCommand::Split { config, seed, out } => {
            let bench = Bench::load(&config)?;
            let splits = er::split(&bench.queries, seed);
            let bytes = serde_json::to_vec_pretty(&splits)?;
            std::fs::write(&out, &bytes)
                .map_err(|e| AppError::InvalidArgument(format!("{}: {e}", out.display())))?;
            serde_json::json!({
                "dev": splits.dev.len(),
                "test": splits.test.len(),
                "sha256": crate::corpus::sha256_hex(&bytes),
            })
            .to_string()
        }
        EvalCommand::Pool {
            config,
            out,
            query_review,
            query_review_sha256,
        } => {
            let bench = Bench::load(&config)?;
            let approval = crate::eval::gates::check_query_review(
                bench.queries(),
                &er::read(&query_review)?,
                &query_review_sha256,
            )?;
            pretty(&er::pool(&bench, &out, &approval)?)?
        }
        EvalCommand::Run {
            config,
            split: SplitName::Test,
            out,
            confirm_test_split,
            preparation: Some(preparation),
            preparation_sha256: Some(pin),
        } => pretty(&er::run_held_out(
            &config,
            &preparation,
            &pin,
            confirm_test_split,
            &out,
        )?)?,
        EvalCommand::Run {
            config,
            split,
            out,
            confirm_test_split,
            preparation,
            preparation_sha256,
        } => {
            let bench = pinned(&config, preparation, preparation_sha256)?;
            pretty(&er::run(&bench, split, confirm_test_split, &out)?)?
        }
        EvalCommand::Score {
            config,
            run_dir,
            confirm_test_split: true,
            preparation: Some(preparation),
            preparation_sha256: Some(pin),
        } => pretty(&er::score_held_out(
            &config,
            &preparation,
            &pin,
            true,
            &run_dir,
        )?)?,
        EvalCommand::Score {
            config,
            run_dir,
            preparation,
            preparation_sha256,
            ..
        } => {
            let bench = pinned(&config, preparation, preparation_sha256)?;
            pretty(&er::score(&bench, &run_dir, false)?)?
        }
        EvalCommand::CheckBenchmark {
            config,
            stage,
            gates,
            run,
            query_review,
            query_review_sha256,
            annotation,
            annotation_notes,
            preparation_out: Some(out),
            operator: Some(operator),
            operator_evidence: Some(operator_evidence),
            ..
        } => {
            if stage != Stage::Qrels || gates.is_some() || run.is_some() {
                return Err(crate::eval::run::EvalError::Invalid(
                    "--preparation-out needs stage qrels and the captured gates.json".into(),
                )
                .into());
            }
            let (Some(query_review), Some(pin), Some(annotation), Some(notes)) = (
                query_review,
                query_review_sha256,
                annotation,
                annotation_notes,
            ) else {
                return Err(crate::eval::run::EvalError::Invalid(
                    "preparation requires query review and annotation".into(),
                )
                .into());
            };
            let preparation = Bench::prepare(
                &config,
                &er::TrustedInputs {
                    query_review: &query_review,
                    query_review_sha256: &pin,
                    annotation: &annotation,
                    annotation_notes: &notes,
                    operator: &operator,
                    operator_evidence: &operator_evidence,
                },
            )?;
            let sha256 = preparation.publish(&out)?;
            pretty(&serde_json::json!({
                "preparation": out.display().to_string(),
                "preparation_sha256": sha256,
                "partition_commitment": preparation.partition_commitment(),
            }))?
        }
        EvalCommand::CheckBenchmark {
            config,
            stage,
            gates,
            query_review,
            query_review_sha256,
            annotation,
            annotation_notes,
            ..
        } => {
            use crate::eval::gates::{
                AnnotationInput, GateConfig, Provenance, check_benchmark, check_query_review,
            };
            let bench = Bench::load(&config)?;
            let approval = match (query_review, query_review_sha256) {
                (Some(path), Some(pin)) => Some(check_query_review(
                    bench.queries(),
                    &er::read(&path)?,
                    &pin,
                )?),
                _ => None,
            };
            let annotation_bytes = match (annotation, annotation_notes) {
                (Some(records), Some(notes)) => Some((er::read(&records)?, er::read(&notes)?)),
                _ => None,
            };
            let provenance = Provenance {
                approval: approval.as_ref(),
                annotation: annotation_bytes
                    .as_ref()
                    .map(|(records, notes)| AnnotationInput {
                        records,
                        notes,
                        budget: crate::eval::run::MAX_RECORDS,
                    }),
            };
            let gate_config: GateConfig = match gates {
                Some(path) => serde_json::from_slice(&read_file(&path)?)?,
                None => GateConfig::default(),
            };
            // `--run` and stage `run` were refused above; saved-run gates
            // belong to admitted `score`.
            let result = check_benchmark(&bench, &gate_config, stage, None, &provenance)?;
            let json = pretty(&result)?;
            if !result.passed {
                return Err(AppError::GatesFailed(json));
            }
            json
        }
        EvalCommand::Sweep {
            config,
            split,
            out,
            preparation,
            preparation_sha256,
        } => {
            let bench = pinned(&config, preparation, preparation_sha256)?;
            let report = crate::eval::sweep::sweep(&bench, split)?;
            let json = pretty(&report)?;
            write_file(&out, json.as_bytes())?;
            json
        }
        EvalCommand::Freeze {
            config,
            threshold,
            repo,
            out,
            preparation,
            preparation_sha256,
        } => {
            use crate::eval::sweep::{CodeIdentity, freeze};
            let bench = pinned(&config, preparation, preparation_sha256)?;
            let code = CodeIdentity::current(&repo)?;
            let frozen = freeze(&bench, threshold, &code)?;
            er::register_freeze(&repo, &bench, &frozen.to_toml(), &out)?;
            pretty(&frozen)?
        }
        EvalCommand::CheckFrozen {
            config,
            frozen,
            repo,
            preparation,
            preparation_sha256,
        } => {
            use crate::eval::sweep::{CodeIdentity, check_frozen};
            let bench = pinned(&config, preparation, preparation_sha256)?;
            let bytes = er::read(&frozen)?;
            let frozen = er::check_registered_export(&repo, &bench, &bytes)?;
            check_frozen(&frozen, &bench, &CodeIdentity::current(&repo)?)?;
            serde_json::json!({ "frozen": "ok" }).to_string()
        }
        EvalCommand::Misses {
            config,
            run_dir,
            preparation,
            preparation_sha256,
        } => {
            let bench = pinned(&config, preparation, preparation_sha256)?;
            pretty(&misses(&bench, &run_dir)?)?
        }
    };
    Ok(json)
}

fn read_file(path: &Path) -> Result<Vec<u8>, AppError> {
    std::fs::read(path).map_err(|e| AppError::InvalidArgument(format!("{}: {e}", path.display())))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    std::fs::write(path, bytes)
        .map_err(|e| AppError::InvalidArgument(format!("{}: {e}", path.display())))
}

#[derive(Debug, Serialize)]
struct Miss {
    query_id: String,
    system: String,
    section_id: String,
    stage: crate::eval::sweep::MissStage,
}

/// Every judged-relevant section absent from a system's top 10 in a run.
fn misses(bench: &crate::eval::run::Bench, run_dir: &Path) -> Result<Vec<Miss>, AppError> {
    use crate::eval::run::{CUTOFF, System, rank_sections};
    let report = crate::eval::run::score(bench, run_dir, false)?;
    let (qrels, _) = bench.qrels()?;
    let indexes = bench.indexes()?;
    let meta = &report.meta;
    let systems = [
        System::B0,
        System::B1,
        System::B2(meta.alpha),
        System::B3(meta.beta),
    ];
    let mut out = Vec::new();
    for q in bench
        .queries
        .iter()
        .filter(|q| report.per_query.contains_key(&q.id))
    {
        let index = &indexes[&q.corpus];
        for system in systems {
            let ranking = rank_sections(index, system, &q.text);
            let top: Vec<&str> = ranking
                .iter()
                .take(CUTOFF)
                .map(|(s, _)| s.as_str())
                .collect();
            for (section, grade) in qrels.get(&q.id).into_iter().flatten() {
                if *grade >= 1 && !top.contains(&section.as_str()) {
                    out.push(Miss {
                        query_id: q.id.clone(),
                        system: system.name(),
                        section_id: section.clone(),
                        stage: crate::eval::sweep::attribute_miss(index, &q.text, section, system),
                    });
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_caps_take_at_most_one_extra_item_and_flag_truncation() {
        for (len, kept, flag) in [
            (0, 0, false),
            (MAX_LIST_ITEMS, MAX_LIST_ITEMS, false),
            (MAX_LIST_ITEMS + 1, MAX_LIST_ITEMS, true),
            (10 * MAX_LIST_ITEMS, MAX_LIST_ITEMS, true),
        ] {
            let pulled = std::cell::Cell::new(0);
            let mut truncated = false;
            let items = capped_iter(
                (0..len).inspect(|_| pulled.set(pulled.get() + 1)),
                &mut truncated,
            );
            // Same result as collecting everything first.
            let mut old = false;
            assert_eq!(items, capped((0..len).collect(), &mut old));
            assert_eq!((items.len(), truncated, old), (kept, flag, flag));
            assert!(
                pulled.get() <= MAX_LIST_ITEMS + 1,
                "pulled {}",
                pulled.get()
            );
        }
    }

    #[test]
    fn committed_uncertain_report_retains_success_and_only_finite_warning() {
        let root = tempfile::tempdir().unwrap();
        let generation = store::build(root.path(), &IndexConfig::default()).unwrap();
        let report = IndexReport::published(
            &generation,
            store::PublishOutcome {
                generation_id: generation.generation_id.clone(),
                durability: store::Durability::Uncertain,
                maintenance: store::Maintenance::NotAttempted,
                failure_phase: Some(store::PublishPhase::CurrentSync),
            },
        );
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["generation_id"], generation.generation_id);
        assert_eq!(json["committed"], true);
        assert_eq!(json["durability"], "uncertain");
        assert_eq!(json["maintenance"], "not_attempted");
        assert_eq!(json["failure_phase"], "current_sync");
        assert_eq!(json["warnings"], serde_json::json!(["current_sync"]));
    }

    #[test]
    fn input_limits_precede_cli_index_access_and_count_utf8_bytes() {
        let root = tempfile::tempdir().unwrap();
        for command in [
            Command::Search(
                serde_json::from_value(serde_json::json!({"query":"é".repeat(2049)})).unwrap(),
            ),
            Command::Read {
                id: "x".repeat(MAX_ID_BYTES + 1),
            },
            Command::Related {
                chunk_id: "x".repeat(MAX_ID_BYTES + 1),
            },
            Command::Verify {
                citations: serde_json::json!(vec![serde_json::json!({}); 101]).to_string(),
            },
        ] {
            let error = run(Cli {
                root: root.path().to_path_buf(),
                command,
            })
            .unwrap_err();
            assert_eq!(error.kind(), "cap_exceeded");
        }
        let app = App::new(root.path());
        let citation = Citation {
            generation_id: "0".repeat(64),
            path: "a.md".into(),
            heading_path: vec![],
            byte_start: 0,
            byte_end: 1,
            line_start: 1,
            line_end: 1,
            sha256: "0".repeat(64),
        };
        assert_eq!(
            app.verify(&vec![citation.clone(); 101]).unwrap_err().kind(),
            "cap_exceeded"
        );
        let mut malformed = citation.clone();
        malformed.sha256 = "A".repeat(64);
        assert_eq!(
            app.verify(&[malformed]).unwrap_err().kind(),
            "invalid_argument"
        );
        assert!(
            validate_citation_fields(
                &citation.generation_id,
                &"x".repeat(MAX_QUERY_BYTES),
                &vec![String::new(); 64],
                &citation.sha256
            )
            .is_ok()
        );
        let request: SearchRequest = serde_json::from_value(
            serde_json::json!({"query":"é".repeat(2048),"scope":"x".repeat(MAX_QUERY_BYTES)}),
        )
        .unwrap();
        assert!(request.validate().is_ok());
        let mut total = 0;
        validate_json_strings(
            &serde_json::json!({"unknown":["x".repeat(MAX_ARGUMENT_BYTES-7)]}),
            &mut total,
        )
        .unwrap();
        assert_eq!(total, MAX_ARGUMENT_BYTES);
        assert_eq!(
            add_argument_bytes(&mut total, 1).unwrap_err().kind(),
            "cap_exceeded"
        );
        total = usize::MAX;
        assert_eq!(
            add_argument_bytes(&mut total, 1).unwrap_err().kind(),
            "cap_exceeded"
        );
        assert_eq!(
            parse_citations(&"x".repeat(MAX_RESPONSE_BYTES + 1))
                .unwrap_err()
                .kind(),
            "cap_exceeded"
        );
        // Missing fields would fail typed decode; limits must win before it.
        for oversized in [
            serde_json::json!({"heading_path":vec!["";65]}),
            serde_json::json!({"path":"x".repeat(MAX_QUERY_BYTES+1)}),
            serde_json::json!({"heading_path":["x".repeat(MAX_ID_BYTES+1)]}),
        ] {
            assert_eq!(
                parse_citations(&oversized.to_string()).unwrap_err().kind(),
                "cap_exceeded"
            );
        }
    }

    #[test]
    fn app_retains_root_before_first_load_and_after_cached_load() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("root");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a.md"), "# A\n\noriginal\n").unwrap();
        App::new(&root).index().unwrap();
        let app = App::new(&root);
        let cached_app = App::new(&root);
        let cached = cached_app.load_index().unwrap();
        let citation = Citation::for_chunk(&cached.generation, 0).unwrap();
        let moved = parent.path().join("moved");
        std::fs::rename(&root, &moved).unwrap();
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a.md"), "# A\n\nreplacement\n").unwrap();
        let replacement = App::new(&root).index().unwrap();
        assert_ne!(citation.generation_id, replacement.generation_id);
        assert_eq!(app.current_id().unwrap(), citation.generation_id);
        assert_eq!(app.load_index().unwrap().generation, cached.generation);
        assert!(app.status(&cached).unwrap().fresh);
        for retained in [&app, &cached_app] {
            assert_eq!(
                retained
                    .verify(std::slice::from_ref(&citation))
                    .unwrap()
                    .results[0]
                    .state,
                cite::CitationState::Valid
            );
        }
        // Changing the original (renamed) object affects only its bound App.
        std::fs::remove_file(moved.join("a.md")).unwrap();
        assert_eq!(
            cached_app
                .verify_index(&cached, &[citation])
                .unwrap()
                .results[0]
                .state,
            cite::CitationState::Missing
        );
    }

    #[test]
    fn selected_generation_stays_pinned_across_publication() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.md"), "# A\n\nfirst\n").unwrap();
        let app = App::new(root.path());
        app.index().unwrap();
        let selected = app.current_id().unwrap();
        std::fs::write(root.path().join("a.md"), "# A\n\nsecond\n").unwrap();
        let next = app.index().unwrap().generation_id;
        assert_ne!(selected, next);
        let pinned = app.load_index_for(&selected).unwrap();
        assert_eq!(pinned.generation.generation_id, selected);
        let citation = Citation::for_chunk(&pinned.generation, 0).unwrap();
        assert_eq!(
            app.verify_index(&pinned, std::slice::from_ref(&citation))
                .unwrap()
                .results[0]
                .state,
            cite::CitationState::Stale
        );
        assert_eq!(
            app.verify(&[citation]).unwrap_err().kind(),
            "generation_unavailable"
        );
        assert_eq!(app.load_index().unwrap().generation.generation_id, next);
    }

    #[test]
    fn failed_root_initialization_never_rebinds() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("later");
        let app = App::new(&root);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("a.md"), "# A\n").unwrap();
        App::new(&root).index().unwrap();
        assert_eq!(app.load_index().err().unwrap().kind(), "io");
    }

    #[test]
    fn indivisible_oversized_metadata_returns_cap_error() {
        let response = RelatedResponse {
            generation_id: String::new(),
            fresh: true,
            chunk_id: "\u{1}".repeat(MAX_RESPONSE_BYTES),
            neighbours: Vec::new(),
            truncated: false,
        };
        assert!(matches!(bound(response), Err(AppError::ResponseTooLarge)));
    }
}
