use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ontocode_extension_api::ContextualUserFragment;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::km_context::render_km_context;
use crate::km_resolve::resolve_active_km;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmIndexRecord;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;

const SINGLE_IMPORT_LOG: &str = "_ops/import/single/progress.jsonl";
const BATCH_MANIFEST: &str = "manifest.jsonl";
const BATCH_PROGRESS: &str = "progress.jsonl";
const BATCH_STATE: &str = "state.toml";
const BATCH_LOG: &str = "import.log";

const TEMP_IMPORT_PREFIX: &str = "ontocode-ontograph-import-";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmImportResult {
    pub id: String,
    pub version: String,
    pub status: String,
    pub cache_path: String,
    pub store_root: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmImportBatchResult {
    pub batch_dir: String,
    pub imported_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub store_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct KmImportLogRecord {
    result: String,
    id: String,
    version: String,
    source_path: String,
    store_root: String,
    cache_path: String,
    duration_ms: u128,
    imported_at_unix_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmImportError {
    MissingSource,
    InvalidKm(Vec<String>),
    InvalidIndex,
    SerializeIndex,
    WriteIndex,
    CreateCacheDir,
    CopyModule,
    HashModule,
    VersionConflict,
    ProjectScopeConflict,
    CreateLogDir,
    WriteLog,
    SerializeLog,
    TemporaryImportStorage,
    ReadManifest,
    InvalidManifest(String),
    ReadProgress,
    WriteState,
    WriteHumanLog,
    CandidateEvalFailed,
    CandidateEvalIncomplete,
    CandidateCleanup,
    ThreadCeiling { ceiling: usize, current: usize },
}

pub fn run_km_import(
    source_path: &Path,
    store_root: &Path,
) -> Result<KmImportResult, KmImportError> {
    run_km_import_with_scope(source_path, store_root, None)
}

pub fn run_km_import_for_project(
    source_path: &Path,
    store_root: &Path,
    project_scope: &Path,
) -> Result<KmImportResult, KmImportError> {
    let project_scope =
        fs::canonicalize(project_scope).map_err(|_| KmImportError::MissingSource)?;
    run_km_import_with_scope(source_path, store_root, Some(project_scope.as_path()))
}

fn run_km_import_with_scope(
    source_path: &Path,
    store_root: &Path,
    project_scope: Option<&Path>,
) -> Result<KmImportResult, KmImportError> {
    let started = Instant::now();
    let store = KmStore::new(store_root);
    let imported = match project_scope {
        Some(project_scope) => {
            store.import_for_project(source_path, &project_scope.to_string_lossy())
        }
        None => store.import(source_path),
    }
    .map_err(import_error)?;
    let result = import_result(imported, store_root);
    append_import_log(&result, started.elapsed().as_millis())?;
    Ok(result)
}

/// Imports an agent-authored candidate only after it passes a full eval in a
/// temporary store. The live store is opened only once every gate has passed,
/// so a failed candidate leaves it byte-for-byte unchanged.
///
/// Human and CLI imports keep using `run_km_import_with_scope`, which has no
/// eval prerequisite.
pub(crate) fn run_km_import_for_agent_candidate(
    source_path: &Path,
    store_root: &Path,
    project_scope: &Path,
    thread_id: &str,
    ceiling: usize,
) -> Result<KmImportResult, KmImportError> {
    let started = Instant::now();
    let temp_root = create_temporary_store_root()?;
    let staged = stage_and_evaluate_candidate(source_path, project_scope, &temp_root, thread_id);
    // Cleanup runs on both paths so no candidate bytes outlive the attempt.
    let cleanup = cleanup_temporary_store(&temp_root);
    let staged = staged?;
    cleanup?;

    // Every gate passed, so the live store is opened for the first time here.
    // The store enforces the per-thread ceiling under its own import lock and
    // persists the typed origin the ceiling counts.
    let store = KmStore::new(store_root);
    let imported = store
        .import_for_agent_thread(
            source_path,
            &project_scope.to_string_lossy(),
            thread_id,
            ceiling,
        )
        .map_err(import_error)?;
    debug_assert_eq!(staged.id, imported.id);
    let result = import_result(imported, store_root);
    // Activation already committed the live index. A diagnostic log failure
    // cannot turn that successful write into a reported rejection.
    let _ = append_import_log(&result, started.elapsed().as_millis());
    Ok(result)
}

/// Imports the candidate into a throwaway store and proves it renders usable
/// context there. Returns the staged record only when every check passed.
///
/// The checks are fixed in code rather than read from an eval-case directory:
/// the previous suite lived under the project's `.ontocode` tree, which the
/// authoring agent can write, so a candidate could supply the cases that judged
/// it.
fn stage_and_evaluate_candidate(
    source_path: &Path,
    project_scope: &Path,
    temp_root: &Path,
    thread_id: &str,
) -> Result<KmImportResult, KmImportError> {
    let temp_store_root = temp_root.join("_store");
    let temp_store = KmStore::new(&temp_store_root);
    let staged = temp_store
        .import_for_project_with_origin(
            source_path,
            &project_scope.to_string_lossy(),
            KmIndexOrigin::AgentThread {
                thread_id: thread_id.to_string(),
            },
        )
        .map_err(import_error)?;
    let staged = import_result(staged, &temp_store_root);
    prove_candidate_renders(&temp_store, &staged)?;
    Ok(staged)
}

/// Proves the staged candidate resolves and renders retrievable context.
///
/// This is the property the eval suite was meant to establish: the record can be
/// resolved from the store, its rendered fragment carries its own identity, and
/// it contributes at least one fact. Anything unresolvable or empty is refused.
fn prove_candidate_renders(
    temp_store: &KmStore,
    staged: &KmImportResult,
) -> Result<(), KmImportError> {
    let resolved = resolve_active_km(temp_store, &staged.id)
        .map_err(|_| KmImportError::CandidateEvalIncomplete)?
        .ok_or(KmImportError::CandidateEvalIncomplete)?;
    if resolved.facts.is_empty() {
        return Err(KmImportError::CandidateEvalFailed);
    }
    let rendered = render_km_context(&resolved, "candidate activation check").render();
    let identity = format!("Knowledge Module: {}@{}", staged.id, staged.version);
    if !rendered.contains(&identity) {
        return Err(KmImportError::CandidateEvalFailed);
    }
    Ok(())
}

pub fn run_km_import_batch(
    batch_dir: &Path,
    store_root: &Path,
    resume: bool,
) -> Result<KmImportBatchResult, KmImportError> {
    run_km_import_batch_with_scope(batch_dir, store_root, resume, None)
}

pub fn run_km_import_batch_for_project(
    batch_dir: &Path,
    store_root: &Path,
    resume: bool,
    project_scope: &Path,
) -> Result<KmImportBatchResult, KmImportError> {
    let project_scope =
        fs::canonicalize(project_scope).map_err(|_| KmImportError::MissingSource)?;
    run_km_import_batch_with_scope(batch_dir, store_root, resume, Some(project_scope.as_path()))
}

fn run_km_import_batch_with_scope(
    batch_dir: &Path,
    store_root: &Path,
    resume: bool,
    project_scope: Option<&Path>,
) -> Result<KmImportBatchResult, KmImportError> {
    let manifest = read_batch_manifest(batch_dir)?;
    let resume_scope = BatchResumeScope {
        store_root: store_root.display().to_string(),
        project_scope: project_scope.map(|scope| scope.display().to_string()),
    };
    let successful_hashes = if resume {
        read_successful_progress_hashes(batch_dir, &resume_scope)?
    } else {
        Vec::new()
    };
    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;

    for source_path in manifest {
        let started = Instant::now();
        let content_hash = hash_import_source(&source_path)?;
        if successful_hashes.contains(&content_hash) {
            skipped_count += 1;
            continue;
        }

        match run_km_import_with_scope(&source_path, store_root, project_scope) {
            Ok(result) => {
                imported_count += 1;
                append_batch_progress(
                    batch_dir,
                    &BatchProgressRecord::imported(
                        result,
                        content_hash,
                        started,
                        resume_scope.clone(),
                    ),
                )?;
            }
            Err(error) => {
                failed_count += 1;
                append_batch_progress(
                    batch_dir,
                    &BatchProgressRecord::failed(
                        &source_path,
                        content_hash,
                        started,
                        &error,
                        resume_scope.clone(),
                    ),
                )?;
            }
        }
    }

    let result = KmImportBatchResult {
        batch_dir: batch_dir.display().to_string(),
        imported_count,
        skipped_count,
        failed_count,
        store_root: store_root.display().to_string(),
    };
    write_batch_state(batch_dir, &result)?;
    append_batch_log(batch_dir, &result)?;
    Ok(result)
}

impl std::fmt::Display for KmImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSource => write!(f, "source KM directory is missing"),
            Self::InvalidKm(errors) if !errors.is_empty() => {
                write!(f, "KM validation failed: {}", errors.join("; "))
            }
            Self::InvalidKm(_) => write!(f, "KM validation failed"),
            Self::InvalidIndex => write!(f, "KM index is invalid"),
            Self::SerializeIndex => write!(f, "failed to serialize the KM index"),
            Self::WriteIndex => write!(f, "failed to write the KM index"),
            Self::CreateCacheDir => write!(f, "failed to create the KM cache directory"),
            Self::CopyModule => write!(f, "failed to copy the KM module into cache"),
            Self::HashModule => write!(f, "failed to hash the KM module"),
            Self::VersionConflict => write!(
                f,
                "KM version content conflict: changed content needs a new version"
            ),
            Self::ProjectScopeConflict => write!(f, "KM project scope conflict"),
            Self::CreateLogDir => write!(f, "failed to create KM import log directory"),
            Self::WriteLog => write!(f, "failed to write KM import log"),
            Self::SerializeLog => write!(f, "failed to serialize KM import log"),
            Self::TemporaryImportStorage => {
                write!(f, "failed to prepare temporary candidate store")
            }
            Self::CandidateEvalFailed => write!(f, "candidate eval did not pass"),
            Self::CandidateEvalIncomplete => {
                write!(f, "candidate eval could not execute a full pass")
            }
            Self::CandidateCleanup => {
                write!(f, "failed to clean up temporary candidate store")
            }
            Self::ThreadCeiling { ceiling, current } => {
                write!(f, "authoring thread holds {current} of {ceiling} records")
            }
            Self::ReadManifest => write!(f, "failed to read KM import batch manifest"),
            Self::InvalidManifest(error) => write!(f, "invalid KM import batch manifest: {error}"),
            Self::ReadProgress => write!(f, "failed to read KM import batch progress"),
            Self::WriteState => write!(f, "failed to write KM import batch state"),
            Self::WriteHumanLog => write!(f, "failed to write KM import batch log"),
        }
    }
}

impl std::error::Error for KmImportError {}

fn import_result(record: KmIndexRecord, store_root: &Path) -> KmImportResult {
    KmImportResult {
        id: record.id,
        version: record.version,
        status: format!("{:?}", record.status).to_lowercase(),
        cache_path: record.path,
        store_root: store_root.display().to_string(),
        source_path: record.source_path,
    }
}

fn append_import_log(result: &KmImportResult, duration_ms: u128) -> Result<(), KmImportError> {
    let Some(km_root) = Path::new(&result.store_root).parent() else {
        return Err(KmImportError::CreateLogDir);
    };
    let log_path = km_root.join(SINGLE_IMPORT_LOG);
    let Some(parent) = log_path.parent() else {
        return Err(KmImportError::CreateLogDir);
    };
    fs::create_dir_all(parent).map_err(|_| KmImportError::CreateLogDir)?;

    let record = KmImportLogRecord {
        result: result.status.clone(),
        id: result.id.clone(),
        version: result.version.clone(),
        source_path: result.source_path.clone(),
        store_root: result.store_root.clone(),
        cache_path: result.cache_path.clone(),
        duration_ms,
        imported_at_unix_seconds: now_unix_seconds(),
    };
    let line = serde_json::to_string(&record).map_err(|_| KmImportError::SerializeLog)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|_| KmImportError::WriteLog)?;
    writeln!(file, "{line}").map_err(|_| KmImportError::WriteLog)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BatchManifestRow {
    Path(String),
    Object {
        path: Option<String>,
        #[serde(rename = "sourcePath")]
        source_path: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchProgressRecord {
    result: String,
    source_path: String,
    id: Option<String>,
    version: Option<String>,
    content_hash: String,
    duration_ms: u128,
    error: Option<String>,
    imported_at_unix_seconds: i64,
    #[serde(default)]
    resume_scope: Option<BatchResumeScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct BatchResumeScope {
    store_root: String,
    project_scope: Option<String>,
}

impl BatchProgressRecord {
    fn imported(
        result: KmImportResult,
        content_hash: String,
        started: Instant,
        resume_scope: BatchResumeScope,
    ) -> Self {
        Self {
            result: "imported".to_string(),
            source_path: result.source_path,
            id: Some(result.id),
            version: Some(result.version),
            content_hash,
            duration_ms: started.elapsed().as_millis(),
            error: None,
            imported_at_unix_seconds: now_unix_seconds(),
            resume_scope: Some(resume_scope),
        }
    }

    fn failed(
        source_path: &Path,
        content_hash: String,
        started: Instant,
        error: &KmImportError,
        resume_scope: BatchResumeScope,
    ) -> Self {
        Self {
            result: "failed".to_string(),
            source_path: source_path.display().to_string(),
            id: None,
            version: None,
            content_hash,
            duration_ms: started.elapsed().as_millis(),
            error: Some(truncate_error(error)),
            imported_at_unix_seconds: now_unix_seconds(),
            resume_scope: Some(resume_scope),
        }
    }
}

fn read_batch_manifest(batch_dir: &Path) -> Result<Vec<PathBuf>, KmImportError> {
    let raw = fs::read_to_string(batch_dir.join(BATCH_MANIFEST))
        .map_err(|_| KmImportError::ReadManifest)?;
    raw.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| parse_manifest_line(batch_dir, index + 1, line))
        .collect()
}

fn parse_manifest_line(
    batch_dir: &Path,
    line_number: usize,
    line: &str,
) -> Result<PathBuf, KmImportError> {
    let row = serde_json::from_str::<BatchManifestRow>(line)
        .map_err(|error| KmImportError::InvalidManifest(format!("line {line_number}: {error}")))?;
    let path = match row {
        BatchManifestRow::Path(path) => path,
        BatchManifestRow::Object { path, source_path } => {
            path.or(source_path).ok_or_else(|| {
                KmImportError::InvalidManifest(format!("line {line_number}: missing path"))
            })?
        }
    };
    let path = PathBuf::from(path);
    Ok(if path.is_absolute() {
        path
    } else {
        batch_dir.join(path)
    })
}

fn read_successful_progress_hashes(
    batch_dir: &Path,
    resume_scope: &BatchResumeScope,
) -> Result<Vec<String>, KmImportError> {
    let path = batch_dir.join(BATCH_PROGRESS);
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(KmImportError::ReadProgress),
    };
    let mut hashes = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let record = serde_json::from_str::<BatchProgressRecord>(line)
            .map_err(|_| KmImportError::ReadProgress)?;
        if matches!(
            record.result.as_str(),
            "imported" | "reactivated" | "skipped"
        ) && record.resume_scope.as_ref() == Some(resume_scope)
        {
            hashes.push(record.content_hash);
        }
    }
    Ok(hashes)
}

fn append_batch_progress(
    batch_dir: &Path,
    record: &BatchProgressRecord,
) -> Result<(), KmImportError> {
    let line = serde_json::to_string(record).map_err(|_| KmImportError::SerializeLog)?;
    append_line(&batch_dir.join(BATCH_PROGRESS), &line).map_err(|_| KmImportError::WriteLog)
}

fn write_batch_state(batch_dir: &Path, result: &KmImportBatchResult) -> Result<(), KmImportError> {
    let state = format!(
        "imported_count = {}\nskipped_count = {}\nfailed_count = {}\n",
        result.imported_count, result.skipped_count, result.failed_count
    );
    fs::write(batch_dir.join(BATCH_STATE), state).map_err(|_| KmImportError::WriteState)
}

fn append_batch_log(batch_dir: &Path, result: &KmImportBatchResult) -> Result<(), KmImportError> {
    append_line(
        &batch_dir.join(BATCH_LOG),
        &format!(
            "imported={} skipped={} failed={}",
            result.imported_count, result.skipped_count, result.failed_count
        ),
    )
    .map_err(|_| KmImportError::WriteHumanLog)
}

fn append_line(path: &Path, line: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

fn truncate_error(error: &KmImportError) -> String {
    let message = error.to_string();
    if message.len() <= 512 {
        return message;
    }
    let mut end = 512;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &message[..end])
}

fn hash_import_source(root: &Path) -> Result<String, KmImportError> {
    let root = fs::canonicalize(root).map_err(|_| KmImportError::MissingSource)?;
    let mut hasher = Sha256::new();
    hash_import_source_inner(&root, &root, &mut hasher)?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_import_source_inner(
    root: &Path,
    current: &Path,
    hasher: &mut Sha256,
) -> Result<(), KmImportError> {
    let mut entries = fs::read_dir(current)
        .map_err(|_| KmImportError::HashModule)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| KmImportError::HashModule)?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let metadata = entry.metadata().map_err(|_| KmImportError::HashModule)?;
        let relative = path
            .strip_prefix(root)
            .map_err(|_| KmImportError::HashModule)?
            .to_string_lossy()
            .replace('\\', "/");
        hasher.update(relative.as_bytes());
        if metadata.is_dir() {
            hasher.update(b"/");
            hash_import_source_inner(root, &path, hasher)?;
            continue;
        }
        let bytes = fs::read(&path).map_err(|_| KmImportError::HashModule)?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(())
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn import_error(error: KmStoreError) -> KmImportError {
    match error {
        KmStoreError::HomeDir | KmStoreError::MissingSource => KmImportError::MissingSource,
        KmStoreError::InvalidKm(report) => KmImportError::InvalidKm(report.errors),
        KmStoreError::InvalidIndex => KmImportError::InvalidIndex,
        // CLI/human import never supplies a typed origin, so this arm exists to
        // keep the match exhaustive and reports the rejection through the
        // existing validation channel instead of widening the public enum.
        KmStoreError::InvalidOrigin => {
            KmImportError::InvalidKm(vec!["km index origin is invalid".to_string()])
        }
        KmStoreError::SerializeIndex => KmImportError::SerializeIndex,
        KmStoreError::WriteIndex => KmImportError::WriteIndex,
        KmStoreError::CreateCacheDir => KmImportError::CreateCacheDir,
        KmStoreError::CopyModule => KmImportError::CopyModule,
        KmStoreError::HashModule => KmImportError::HashModule,
        KmStoreError::VersionConflict => KmImportError::VersionConflict,
        KmStoreError::ProjectScopeConflict => KmImportError::ProjectScopeConflict,
        KmStoreError::PackageOwnershipConflict => {
            KmImportError::InvalidKm(vec!["KM package id belongs to another author".to_string()])
        }
        KmStoreError::ThreadCeiling { ceiling, current } => {
            KmImportError::ThreadCeiling { ceiling, current }
        }
    }
}

fn create_temporary_store_root() -> Result<PathBuf, KmImportError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("{TEMP_IMPORT_PREFIX}{nanos}"));
    fs::create_dir_all(&path).map_err(|_| KmImportError::TemporaryImportStorage)?;
    Ok(path)
}

fn cleanup_temporary_store(temp_root: &Path) -> Result<(), KmImportError> {
    fs::remove_dir_all(temp_root).map_err(|_| KmImportError::CandidateCleanup)
}

#[cfg(test)]
#[path = "km_import_tests.rs"]
mod tests;
