use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use ontocode_extension_api::AgentSpawnFuture;
use ontocode_extension_api::AgentSpawner;
use ontocode_protocol::ThreadId;
use serde::Deserialize;
use serde::Serialize;

use crate::km::KmValidationStatus;
use crate::km::validate_km;
use crate::km_llm_consolidate::KmLlmConsolidateRequest;
use crate::km_llm_consolidate::KmLlmConsolidateResult;
use crate::km_llm_consolidate::run_km_llm_consolidate;
use crate::km_resolve::KmResolveError;
use crate::km_resolve::KmRetrievalCandidate;
use crate::km_resolve::KmRetrievalReason;
use crate::km_resolve::retrieve_active_kms;
use crate::km_store::KmIndexRecord;
use crate::km_store::KmModuleStatus;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;

mod ontology_query;
mod source_drift;
mod version_diff;

const MAX_KM_LIBRARIAN_QUERY_CANDIDATES: usize = 25;
/// Age after which an obsolete module is reported as retention-expired. It is a
/// diagnostic threshold only: the health check never deletes cached content.
const KM_OBSOLETE_RETENTION_SECONDS: i64 = 90 * 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianHealthRequest {
    pub km_store_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianHealthResult {
    pub active_modules: usize,
    pub obsolete_modules: usize,
    pub issues: Vec<KmLibrarianHealthIssue>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianHealthIssue {
    pub id: String,
    pub version: String,
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianHealthError {
    Store(String),
    Validate(String),
}

pub fn run_km_librarian_health(
    request: &KmLibrarianHealthRequest,
) -> Result<KmLibrarianHealthResult, KmLibrarianHealthError> {
    let store = KmStore::new(&request.km_store_root);
    let mut records = store.list(/*include_obsolete*/ true)?;
    records.sort_by(|left, right| {
        (&left.id, &left.version, &left.path).cmp(&(&right.id, &right.version, &right.path))
    });

    let active_modules = records
        .iter()
        .filter(|record| record.status == KmModuleStatus::Active)
        .count();
    let obsolete_modules = records
        .iter()
        .filter(|record| record.status == KmModuleStatus::Obsolete)
        .count();

    // Canonical set of directories the index claims, used below to find cache
    // directories no record points at.
    let indexed_paths = records
        .iter()
        .map(|record| PathBuf::from(&record.path))
        .collect::<BTreeSet<_>>();

    let mut issues = Vec::new();
    for record in records {
        if let Some(source_issue) = source_drift::issue_for(&record)? {
            issues.push(source_issue);
        }

        // Retention is reported for obsolete records only: an active module is
        // in use regardless of age.
        if record.status == KmModuleStatus::Obsolete
            && let Some(age) = obsolete_age_seconds(&record)
            && age > KM_OBSOLETE_RETENTION_SECONDS
        {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "retention-expired",
                format!("obsolete KM package has been retained for {age} seconds"),
            ));
        }

        let package_dir = Path::new(&record.path);
        if !package_dir.is_dir() {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "package-dir-missing",
                "cached KM package directory is missing",
            ));
            continue;
        }

        let validation = validate_km(package_dir);
        if validation.status != KmValidationStatus::Valid {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "validation-status",
                "cached KM package validation status is invalid",
            ));
        }
        // A hash mismatch means the cached bytes no longer match what was
        // imported. `validate_km` checks package shape, not identity, so this is
        // the only check that detects tampering which leaves the package
        // structurally valid. An already-invalid package is reported by the
        // validation codes above; adding a hash issue there would be duplicate
        // noise for the same underlying edit.
        if validation.status == KmValidationStatus::Valid
            && store.validated_module_root(&record).is_err()
        {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "content-tampered",
                "cached KM package does not match its recorded content hash",
            ));
        }
        for error in validation.errors {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "validation-error",
                error,
            ));
        }
        for warning in validation.warnings {
            issues.push(issue(
                &record.id,
                &record.version,
                &record.path,
                "validation-warning",
                warning,
            ));
        }
    }

    issues.extend(orphan_issues(&store, &indexed_paths));

    Ok(KmLibrarianHealthResult {
        active_modules,
        obsolete_modules,
        issues,
    })
}

/// Age in seconds of an obsolete record, or `None` when the clock is unusable.
fn obsolete_age_seconds(record: &KmIndexRecord) -> Option<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    now.checked_sub(record.imported_at_unix_seconds)
        .filter(|age| *age >= 0)
}

/// Cache directories that no index record points at.
///
/// An orphan is storage the store will never read but also never reclaims, so a
/// tampered or half-removed import stays invisible without this check.
fn orphan_issues(
    store: &KmStore,
    indexed_paths: &BTreeSet<PathBuf>,
) -> Vec<KmLibrarianHealthIssue> {
    let mut orphans = Vec::new();
    for cached in store.cached_version_dirs() {
        if indexed_paths.contains(&cached) {
            continue;
        }
        let path = cached.to_string_lossy().to_string();
        orphans.push(issue(
            /*id*/ "",
            /*version*/ "",
            &path,
            "orphan-cache-dir",
            "cached KM package directory has no index record",
        ));
    }
    orphans
}

impl From<KmStoreError> for KmLibrarianHealthError {
    fn from(error: KmStoreError) -> Self {
        Self::Store(format!("{error:?}"))
    }
}

impl std::fmt::Display for KmLibrarianHealthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(f, "KM store health read failed: {error}"),
            Self::Validate(message) => write!(f, "KM health validation failed: {message}"),
        }
    }
}

impl std::error::Error for KmLibrarianHealthError {}

fn issue(
    id: &str,
    version: &str,
    path: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> KmLibrarianHealthIssue {
    KmLibrarianHealthIssue {
        id: id.to_string(),
        version: version.to_string(),
        path: path.to_string(),
        code: code.into(),
        message: message.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianCatalogRequest {
    pub km_store_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianCatalogEntry {
    pub id: String,
    pub version: String,
    pub path: String,
    pub source_path: String,
    pub content_hash: String,
    pub imported_at_unix_seconds: i64,
    pub topic_words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianCatalogResult {
    pub modules: Vec<KmLibrarianCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianCatalogError {
    Store(String),
}

pub fn run_km_librarian_catalog(
    request: &KmLibrarianCatalogRequest,
) -> Result<KmLibrarianCatalogResult, KmLibrarianCatalogError> {
    let store = KmStore::new(&request.km_store_root);
    let records = store.list(/*include_obsolete*/ false)?;
    let modules = records
        .into_iter()
        .map(|record| KmLibrarianCatalogEntry {
            id: record.id,
            version: record.version,
            path: record.path,
            source_path: record.source_path,
            content_hash: record.content_hash,
            imported_at_unix_seconds: record.imported_at_unix_seconds,
            topic_words: record.topic_words,
        })
        .collect();

    Ok(KmLibrarianCatalogResult { modules })
}

impl From<KmStoreError> for KmLibrarianCatalogError {
    fn from(error: KmStoreError) -> Self {
        Self::Store(format!("{error:?}"))
    }
}

impl std::fmt::Display for KmLibrarianCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(f, "KM store catalog read failed: {error}"),
        }
    }
}

impl std::error::Error for KmLibrarianCatalogError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianVersionDiffRequest {
    pub km_store_root: PathBuf,
    pub module_id: String,
    pub left_version: String,
    pub right_version: String,
}

/// Bounded read-only snapshot comparison of two retained versions of one KM
/// module. Snapshot comparison only: no as-of query, no validity intervals, no
/// supersession edges, no event log, no schema version.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionDiffResult {
    pub module_id: String,
    pub left: KmLibrarianVersionEntry,
    pub right: KmLibrarianVersionEntry,
    pub added_facts: Vec<KmLibrarianVersionFactEntry>,
    pub removed_facts: Vec<KmLibrarianVersionFactEntry>,
    pub changed_facts: Vec<KmLibrarianVersionFactChange>,
    pub added_mappings: Vec<KmLibrarianVersionMappingEntry>,
    pub removed_mappings: Vec<KmLibrarianVersionMappingEntry>,
    pub changed_mappings: Vec<KmLibrarianVersionMappingChange>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionEntry {
    pub version: String,
    #[allow(private_interfaces)]
    pub status: KmModuleStatus,
    pub content_hash: String,
    pub imported_at_unix_seconds: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionFactEntry {
    pub fact_id: String,
    pub kind: String,
    pub claim: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionFactChange {
    pub fact_id: String,
    pub before: KmLibrarianVersionFactEntry,
    pub after: KmLibrarianVersionFactEntry,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionMappingEntry {
    pub mapping_id: String,
    pub source_concept: String,
    pub target_concept: String,
    pub relation: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianVersionMappingChange {
    pub mapping_id: String,
    pub before: KmLibrarianVersionMappingEntry,
    pub after: KmLibrarianVersionMappingEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianVersionDiffError {
    Store(String),
    InvalidRequest(String),
    VersionNotFound(String),
    InvalidSnapshot(String),
}

pub fn run_km_librarian_version_diff(
    request: &KmLibrarianVersionDiffRequest,
) -> Result<KmLibrarianVersionDiffResult, KmLibrarianVersionDiffError> {
    version_diff::run(request)
}

impl From<KmStoreError> for KmLibrarianVersionDiffError {
    fn from(error: KmStoreError) -> Self {
        Self::Store(format!("{error:?}"))
    }
}

impl std::fmt::Display for KmLibrarianVersionDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(f, "KM store version read failed: {error}"),
            Self::InvalidRequest(error) => write!(f, "invalid version diff request: {error}"),
            Self::VersionNotFound(target) => write!(f, "retained version not found: {target}"),
            Self::InvalidSnapshot(error) => write!(f, "cached snapshot is unusable: {error}"),
        }
    }
}

impl std::error::Error for KmLibrarianVersionDiffError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianQueryRequest {
    pub km_store_root: PathBuf,
    pub query: String,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianQueryResult {
    pub query: String,
    pub candidates: Vec<KmLibrarianQueryCandidate>,
    pub truncated: bool,
    pub ontology_paths: Vec<KmLibrarianOntologyPath>,
    pub matched_facts: Vec<KmLibrarianMatchedFact>,
    pub ontology_truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianQueryCandidate {
    pub scope: KmLibrarianQueryScope,
    pub id: String,
    pub version: String,
    pub path: String,
    pub matched_terms: Vec<String>,
    pub reason: String,
    pub referenced_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianOntologyPath {
    pub package_id: String,
    pub package_version: String,
    pub mapping_id: String,
    pub depth: usize,
    pub source_concept: String,
    pub target_concept: String,
    pub relation: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianMatchedFact {
    pub package_id: String,
    pub package_version: String,
    pub fact_id: String,
    pub kind: String,
    pub claim: String,
    pub source: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KmLibrarianQueryScope {
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianQueryError {
    Resolve(String),
    Ontology(String),
}

pub fn run_km_librarian_query(
    request: &KmLibrarianQueryRequest,
) -> Result<KmLibrarianQueryResult, KmLibrarianQueryError> {
    let store = KmStore::new(&request.km_store_root);
    let retrieval = retrieve_active_kms(&store, &request.query)?;
    let mut candidates = retrieval
        .local
        .into_iter()
        .map(|candidate| query_candidate(KmLibrarianQueryScope::Local, candidate))
        .chain(
            retrieval
                .global
                .into_iter()
                .map(|candidate| query_candidate(KmLibrarianQueryScope::Global, candidate)),
        )
        .collect::<Vec<_>>();
    sort_query_candidates(&mut candidates);
    let mut seen_packages = BTreeSet::new();
    candidates.retain(|candidate| {
        seen_packages.insert((candidate.id.clone(), candidate.version.clone()))
    });

    let limit = request
        .max_candidates
        .min(MAX_KM_LIBRARIAN_QUERY_CANDIDATES);
    let truncated = candidates.len() > limit;
    candidates.truncate(limit);
    let ontology = ontology_query::expand(&candidates).map_err(KmLibrarianQueryError::Ontology)?;

    Ok(KmLibrarianQueryResult {
        query: request.query.clone(),
        candidates,
        truncated,
        ontology_paths: ontology.paths,
        matched_facts: ontology.matched_facts,
        ontology_truncated: ontology.truncated,
    })
}

impl From<KmResolveError> for KmLibrarianQueryError {
    fn from(error: KmResolveError) -> Self {
        Self::Resolve(format!("{error:?}"))
    }
}

impl std::fmt::Display for KmLibrarianQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolve(error) => write!(f, "KM librarian query failed: {error}"),
            Self::Ontology(error) => write!(f, "KM librarian ontology query failed: {error}"),
        }
    }
}

impl std::error::Error for KmLibrarianQueryError {}

fn query_candidate(
    scope: KmLibrarianQueryScope,
    candidate: KmRetrievalCandidate,
) -> KmLibrarianQueryCandidate {
    let (reason, referenced_by) = match candidate.reason {
        KmRetrievalReason::Local => ("local".to_string(), None),
        KmRetrievalReason::Summary => ("summary".to_string(), None),
        KmRetrievalReason::ReferenceNeighbor { referenced_by } => {
            ("reference_neighbor".to_string(), Some(referenced_by))
        }
    };
    KmLibrarianQueryCandidate {
        scope,
        id: candidate.id,
        version: candidate.version,
        path: candidate.path,
        matched_terms: candidate.matched_terms,
        reason,
        referenced_by,
    }
}

fn sort_query_candidates(candidates: &mut [KmLibrarianQueryCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .matched_terms
            .len()
            .cmp(&left.matched_terms.len())
            .then_with(|| left.id.cmp(&right.id))
            .then_with(|| left.version.cmp(&right.version))
            .then_with(|| scope_rank(left.scope).cmp(&scope_rank(right.scope)))
            .then_with(|| left.reason.cmp(&right.reason))
            .then_with(|| left.referenced_by.cmp(&right.referenced_by))
    });
}

fn scope_rank(scope: KmLibrarianQueryScope) -> u8 {
    match scope {
        KmLibrarianQueryScope::Local => 0,
        KmLibrarianQueryScope::Global => 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianAgentRequest {
    pub km_store_root: PathBuf,
    pub input_dirs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianAgentPlan {
    pub steps: Vec<KmLibrarianAgentPlanStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianAgentPlanStep {
    Health(KmLibrarianHealthRequest),
    Catalog(KmLibrarianCatalogRequest),
    HealScan(KmLibrarianHealScanRequest),
    Consolidate(KmLibrarianConsolidateRequest),
}

#[derive(Clone, Debug)]
pub struct KmLibrarianAgent<S> {
    agent_spawner: S,
}

impl<S> KmLibrarianAgent<S> {
    pub fn new(agent_spawner: S) -> Self {
        Self { agent_spawner }
    }

    pub fn spawn_subagent<'a>(
        &'a self,
        forked_from_thread_id: ThreadId,
        request: KmLibrarianAgentRequest,
    ) -> AgentSpawnFuture<
        'a,
        <S as AgentSpawner<KmLibrarianAgentRequest>>::Spawned,
        <S as AgentSpawner<KmLibrarianAgentRequest>>::Error,
    >
    where
        S: AgentSpawner<KmLibrarianAgentRequest>,
    {
        self.agent_spawner
            .spawn_subagent(forked_from_thread_id, request)
    }
}

pub fn run_km_librarian_agent_plan(request: &KmLibrarianAgentRequest) -> KmLibrarianAgentPlan {
    KmLibrarianAgentPlan {
        steps: vec![
            KmLibrarianAgentPlanStep::Health(KmLibrarianHealthRequest {
                km_store_root: request.km_store_root.clone(),
            }),
            KmLibrarianAgentPlanStep::Catalog(KmLibrarianCatalogRequest {
                km_store_root: request.km_store_root.clone(),
            }),
            KmLibrarianAgentPlanStep::HealScan(KmLibrarianHealScanRequest {
                km_store_root: request.km_store_root.clone(),
            }),
            KmLibrarianAgentPlanStep::Consolidate(KmLibrarianConsolidateRequest {
                input_dirs: request.input_dirs.clone(),
                output_dir: request.output_dir.clone(),
                km_store_root: request.km_store_root.clone(),
                project_root: request.project_root.clone(),
            }),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianHealScanRequest {
    pub km_store_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianHealScanResult {
    pub reports: Vec<KmLibrarianHealScanReport>,
    pub conflicts: Vec<KmLibrarianHealScanConflict>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianHealScanReport {
    pub id: String,
    pub version: String,
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianHealScanConflict {
    pub package_id: String,
    pub package_version: String,
    pub package_path: String,
    pub id: Option<String>,
    pub source_concept: Option<String>,
    pub target_concept: Option<String>,
    pub relation: Option<String>,
    pub resolution: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianHealScanError {
    Store(String),
}

#[derive(Debug, Deserialize)]
struct KmLibrarianMappingRow {
    kind: Option<String>,
    id: Option<String>,
    source_concept: Option<String>,
    target_concept: Option<String>,
    relation: Option<String>,
    resolution: Option<String>,
    source: Option<String>,
}

pub fn run_km_librarian_heal_scan(
    request: &KmLibrarianHealScanRequest,
) -> Result<KmLibrarianHealScanResult, KmLibrarianHealScanError> {
    let store = KmStore::new(&request.km_store_root);
    let mut records = store.list(/*include_obsolete*/ true)?;
    records.sort_by(|left, right| {
        (&left.id, &left.version, &left.path).cmp(&(&right.id, &right.version, &right.path))
    });

    let mut reports = Vec::new();
    let mut conflicts = Vec::new();
    for record in records {
        let package_dir = Path::new(&record.path);
        if !package_dir.is_dir() {
            continue;
        }

        let validation = validate_km(package_dir);
        if validation.status != KmValidationStatus::Valid {
            reports.push(heal_scan_report(
                &record.id,
                &record.version,
                &record.path,
                "invalid-package",
                "cached KM package validation status is invalid",
            ));
        }

        let mappings_path = package_dir.join("mappings.jsonl");
        let mappings = match fs::read_to_string(&mappings_path) {
            Ok(mappings) => mappings,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                reports.push(heal_scan_report(
                    &record.id,
                    &record.version,
                    &record.path,
                    "mappings-read-error",
                    "failed to read mappings.jsonl",
                ));
                continue;
            }
        };

        for (index, line) in mappings.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row = match serde_json::from_str::<KmLibrarianMappingRow>(line) {
                Ok(row) => row,
                Err(error) => {
                    reports.push(heal_scan_report(
                        &record.id,
                        &record.version,
                        &record.path,
                        "mappings-jsonl-malformed",
                        format!("mappings.jsonl line {} is malformed: {error}", index + 1),
                    ));
                    continue;
                }
            };
            if row.kind.as_deref() == Some("conflict_record") {
                conflicts.push(KmLibrarianHealScanConflict {
                    package_id: record.id.clone(),
                    package_version: record.version.clone(),
                    package_path: record.path.clone(),
                    id: row.id,
                    source_concept: row.source_concept,
                    target_concept: row.target_concept,
                    relation: row.relation,
                    resolution: row.resolution,
                    source: row.source,
                });
            }
        }
    }

    reports.sort_by(|left, right| {
        (
            &left.id,
            &left.version,
            &left.path,
            &left.code,
            &left.message,
        )
            .cmp(&(
                &right.id,
                &right.version,
                &right.path,
                &right.code,
                &right.message,
            ))
    });
    conflicts.sort_by(|left, right| {
        (
            &left.package_id,
            &left.package_version,
            &left.package_path,
            &left.id,
            &left.source_concept,
            &left.target_concept,
            &left.relation,
            &left.resolution,
            &left.source,
        )
            .cmp(&(
                &right.package_id,
                &right.package_version,
                &right.package_path,
                &right.id,
                &right.source_concept,
                &right.target_concept,
                &right.relation,
                &right.resolution,
                &right.source,
            ))
    });

    Ok(KmLibrarianHealScanResult { reports, conflicts })
}

impl From<KmStoreError> for KmLibrarianHealScanError {
    fn from(error: KmStoreError) -> Self {
        Self::Store(format!("{error:?}"))
    }
}

impl std::fmt::Display for KmLibrarianHealScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(error) => write!(f, "KM store heal scan read failed: {error}"),
        }
    }
}

impl std::error::Error for KmLibrarianHealScanError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLibrarianConsolidateRequest {
    pub input_dirs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub km_store_root: PathBuf,
    pub project_root: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLibrarianConsolidateResult {
    pub consolidation: KmLlmConsolidateResult,
    pub health: KmLibrarianHealthResult,
    pub heal_scan: KmLibrarianHealScanResult,
    pub promotion_skipped_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLibrarianConsolidateError {
    Consolidate(String),
    Health(String),
    HealScan(String),
}

pub fn run_km_librarian_consolidate(
    request: &KmLibrarianConsolidateRequest,
) -> Result<KmLibrarianConsolidateResult, KmLibrarianConsolidateError> {
    let consolidation = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: request.input_dirs.clone(),
            output_dir: request.output_dir.clone(),
            project_root: request.project_root.clone(),
        },
        /*model_ctx*/ None,
    )
    .map_err(|error| KmLibrarianConsolidateError::Consolidate(error.to_string()))?;
    let health = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: request.km_store_root.clone(),
    })
    .map_err(|error| KmLibrarianConsolidateError::Health(error.to_string()))?;
    let heal_scan = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: request.km_store_root.clone(),
    })
    .map_err(|error| KmLibrarianConsolidateError::HealScan(error.to_string()))?;

    Ok(KmLibrarianConsolidateResult {
        consolidation,
        health,
        heal_scan,
        promotion_skipped_reason:
            "gold promotion and active-KM mutation were not attempted by this report-only task"
                .to_string(),
    })
}

impl std::fmt::Display for KmLibrarianConsolidateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Consolidate(error) => write!(f, "KM consolidation failed: {error}"),
            Self::Health(error) => write!(f, "KM health check failed: {error}"),
            Self::HealScan(error) => write!(f, "KM heal scan failed: {error}"),
        }
    }
}

impl std::error::Error for KmLibrarianConsolidateError {}

fn heal_scan_report(
    id: &str,
    version: &str,
    path: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> KmLibrarianHealScanReport {
    KmLibrarianHealScanReport {
        id: id.to_string(),
        version: version.to_string(),
        path: path.to_string(),
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
#[path = "km_librarian_tests.rs"]
mod tests;
