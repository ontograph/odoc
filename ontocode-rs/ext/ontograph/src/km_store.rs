use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ontocode_utils_path::write_atomically;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::artifact_io::read_to_string;
use crate::km::KmValidationReport;
use crate::km::KmValidationStatus;
use crate::km::load_facts;
use crate::km::load_manifest;
use crate::km::load_transitions;
use crate::km::validate_km;
use crate::km::validate_km_for_project;
use crate::km::validate_package_budget;

const KM_INDEX_SCHEMA_VERSION: u32 = 1;
const KM_INDEX_FILE: &str = "index.json";
const KM_MODULES_DIR: &str = "modules";
const MAX_KM_INDEX_BYTES: usize = 4 * 1024 * 1024;
/// Store-owned import lock. Serializes concurrent load-modify-save import
/// cycles so two writers cannot lose each other's appended records.
const KM_IMPORT_LOCK_FILE: &str = "import.lock";
/// Maximum time to wait for a contended import lock before failing.
const KM_IMPORT_LOCK_MAX_WAIT: Duration = Duration::from_secs(5);
/// Poll interval while waiting for a contended import lock.
const KM_IMPORT_LOCK_POLL: Duration = Duration::from_millis(25);
/// Minimum age before an import lock is eligible for owner-liveness checking.
const KM_IMPORT_LOCK_STALE: Duration = Duration::from_secs(30);
const KM_TOPIC_FACT_CLAIM_LIMIT: usize = 30;
const KM_TOPIC_STOP_WORDS: &[&str] = &[
    "about", "after", "also", "and", "are", "before", "between", "for", "from", "into", "must",
    "should", "that", "their", "the", "then", "there", "this", "with",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KmModuleStatus {
    Active,
    Obsolete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KmIndexOrigin {
    AgentThread { thread_id: String },
    Human,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KmModuleWithdrawal {
    pub reason: String,
    pub withdrawn_at_unix_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KmIndexRecord {
    pub id: String,
    pub version: String,
    pub path: String,
    pub content_hash: String,
    pub status: KmModuleStatus,
    pub imported_at_unix_seconds: i64,
    pub source_path: String,
    #[serde(default)]
    pub topic_words: Vec<String>,
    #[serde(default)]
    pub origin: Option<KmIndexOrigin>,
    #[serde(default)]
    pub withdrawal: Option<KmModuleWithdrawal>,
    /// Canonical project/workspace scope this record was imported for. `None`
    /// means the record is unscoped and is NEVER eligible for automatic
    /// injection (per ADR_ONTOGRAPH_KM_PROJECT_SCOPE). Explicit
    /// `ontograph.km_context` lookup by id ignores this field.
    #[serde(default)]
    pub project_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct KmIndex {
    schema_version: u32,
    modules: Vec<KmIndexRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KmStoreError {
    #[allow(dead_code)] // reserved error category
    HomeDir,
    MissingSource,
    InvalidKm(KmValidationReport),
    InvalidIndex,
    InvalidOrigin,
    SerializeIndex,
    WriteIndex,
    CreateCacheDir,
    CopyModule,
    HashModule,
    VersionConflict,
    ProjectScopeConflict,
    PackageOwnershipConflict,
    /// The authoring thread already holds its full budget of records.
    ThreadCeiling {
        ceiling: usize,
        current: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmStore {
    root: PathBuf,
    index_path: PathBuf,
    modules_root: PathBuf,
}

impl KmStore {
    pub(crate) fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            index_path: root.join(KM_INDEX_FILE),
            modules_root: root.join(KM_MODULES_DIR),
            root,
        }
    }

    pub(crate) fn for_project_path(path: &Path) -> Option<Self> {
        project_root(path).map(|root| Self::new(root.join(".ontocode/ontograph/km/_store")))
    }

    pub(crate) fn import(&self, source_path: &Path) -> Result<KmIndexRecord, KmStoreError> {
        self.import_with_scope(source_path, None, None)
    }

    /// Imports a KM associated with an explicit canonical project scope so the
    /// record becomes eligible for automatic injection only in threads whose
    /// active project scope matches.
    pub(crate) fn import_for_project(
        &self,
        source_path: &Path,
        project_scope: &str,
    ) -> Result<KmIndexRecord, KmStoreError> {
        self.import_with_scope(source_path, Some(project_scope.to_string()), None)
    }

    pub(crate) fn import_for_project_with_origin(
        &self,
        source_path: &Path,
        project_scope: &str,
        origin: KmIndexOrigin,
    ) -> Result<KmIndexRecord, KmStoreError> {
        self.import_with_scope(source_path, Some(project_scope.to_string()), Some(origin))
    }

    #[cfg(test)]
    pub(crate) fn import_with_origin(
        &self,
        source_path: &Path,
        origin: KmIndexOrigin,
    ) -> Result<KmIndexRecord, KmStoreError> {
        self.import_with_scope(source_path, None, Some(origin))
    }

    /// Imports one agent-authored candidate, enforcing the per-thread ceiling
    /// under the same lock hold that appends the record.
    ///
    /// Counting outside the lock is not sufficient: two concurrent authorings
    /// both observe a count below the ceiling and both commit, so the check and
    /// the append must be atomic with respect to each other.
    pub(crate) fn import_for_agent_thread(
        &self,
        source_path: &Path,
        project_scope: &str,
        thread_id: &str,
        ceiling: usize,
    ) -> Result<KmIndexRecord, KmStoreError> {
        let origin = KmIndexOrigin::AgentThread {
            thread_id: thread_id.to_string(),
        };
        let source_path = fs::canonicalize(source_path).map_err(|_| KmStoreError::MissingSource)?;
        let validation = validate_import_source(&source_path, Some(project_scope));
        if validation.status != KmValidationStatus::Valid {
            return Err(KmStoreError::InvalidKm(validation));
        }
        validate_index_origin(Some(&origin))?;

        let _lock = self.acquire_import_lock()?;
        let index = self.load_index()?;
        if index.modules.iter().any(|record| {
            record.id == validation.id
                && !matches_origin_filter(&record.origin, Some(&origin), Some(thread_id))
        }) {
            return Err(KmStoreError::PackageOwnershipConflict);
        }
        // Withdrawn and superseded records still count, so a thread cannot free
        // budget by withdrawing (ADR decision D5).
        let current = index
            .modules
            .iter()
            .filter(|record| matches_origin_filter(&record.origin, Some(&origin), Some(thread_id)))
            .count();
        if current >= ceiling {
            return Err(KmStoreError::ThreadCeiling { ceiling, current });
        }
        self.import_locked(
            &source_path,
            validation,
            index,
            Some(project_scope.to_string()),
            Some(origin),
        )
    }

    fn import_with_scope(
        &self,
        source_path: &Path,
        project_scope: Option<String>,
        origin: Option<KmIndexOrigin>,
    ) -> Result<KmIndexRecord, KmStoreError> {
        let source_path = fs::canonicalize(source_path).map_err(|_| KmStoreError::MissingSource)?;
        let validation = validate_import_source(&source_path, project_scope.as_deref());
        if validation.status != KmValidationStatus::Valid {
            return Err(KmStoreError::InvalidKm(validation));
        }

        validate_index_origin(origin.as_ref())?;

        // Serialize the entire load-modify-save cycle so concurrent imports
        // cannot lose each other's appended records (PR-026, PR-027).
        let _lock = self.acquire_import_lock()?;
        let index = self.load_index()?;
        self.import_locked(&source_path, validation, index, project_scope, origin)
    }

    /// Completes one import with the store lock already held and the candidate
    /// already validated.
    fn import_locked(
        &self,
        source_path: &Path,
        validation: KmValidationReport,
        mut index: KmIndex,
        project_scope: Option<String>,
        origin: Option<KmIndexOrigin>,
    ) -> Result<KmIndexRecord, KmStoreError> {
        let source_path = source_path.to_path_buf();
        let content_hash = hash_directory(&source_path)?;
        let destination = self.module_cache_path(&validation.id, &validation.version);
        if let Some(existing) = index
            .modules
            .iter()
            .find(|record| record.id == validation.id && record.version == validation.version)
        {
            let was_withdrawn = existing.withdrawal.is_some();
            if existing.content_hash != content_hash {
                return Err(KmStoreError::VersionConflict);
            }
            if matches!(
                (&existing.project_scope, &project_scope),
                (Some(existing_scope), Some(project_scope)) if existing_scope != project_scope
            ) {
                return Err(KmStoreError::ProjectScopeConflict);
            }
            self.ensure_cached_module(&source_path, &destination, &content_hash)?;
            backfill_topic_words(
                &mut index.modules,
                &validation.id,
                &validation.version,
                &source_path,
            );
            backfill_project_scope(
                &mut index.modules,
                &validation.id,
                &validation.version,
                project_scope.as_deref(),
            );
            backfill_origin(
                &mut index.modules,
                &validation.id,
                &validation.version,
                &origin,
            );
            if !was_withdrawn {
                mark_active_version(&mut index.modules, &validation.id, &validation.version);
            }
            self.save_index(&index)?;
            return index
                .modules
                .iter()
                .find(|record| record.id == validation.id && record.version == validation.version)
                .cloned()
                .ok_or(KmStoreError::InvalidIndex);
        }

        self.ensure_cached_module(&source_path, &destination, &content_hash)?;
        mark_other_versions_obsolete(&mut index.modules, &validation.id);
        let imported = KmIndexRecord {
            id: validation.id,
            version: validation.version,
            path: destination.display().to_string(),
            content_hash,
            status: KmModuleStatus::Active,
            imported_at_unix_seconds: now_unix_seconds(),
            source_path: source_path.display().to_string(),
            topic_words: collect_topic_words(&source_path),
            origin,
            withdrawal: None,
            project_scope,
        };
        index.modules.push(imported.clone());
        sort_records(&mut index.modules);
        self.save_index(&index)?;
        Ok(imported)
    }

    #[cfg(test)]
    pub(crate) fn obsolete(
        &self,
        id: &str,
        version: &str,
    ) -> Result<Option<KmIndexRecord>, KmStoreError> {
        // Same load-modify-save cycle as import, so it needs the same lock or a
        // concurrent import overwrites this status change (PR-026, PR-027).
        let _lock = self.acquire_import_lock()?;
        let mut index = self.load_index()?;
        let mut updated = None;
        for record in &mut index.modules {
            if record.id == id && record.version == version {
                record.status = KmModuleStatus::Obsolete;
                updated = Some(record.clone());
            }
        }
        if updated.is_some() {
            self.save_index(&index)?;
        }
        Ok(updated)
    }

    pub(crate) fn withdraw(
        &self,
        id: &str,
        version: &str,
        reason: String,
    ) -> Result<Option<KmIndexRecord>, KmStoreError> {
        // Withdrawal is the kill-switch for an auto-injected record, so losing
        // it to a concurrent import would silently keep the record eligible.
        let _lock = self.acquire_import_lock()?;
        let mut index = self.load_index()?;
        let mut updated = None;
        for record in &mut index.modules {
            if record.id == id && record.version == version {
                record.status = KmModuleStatus::Obsolete;
                record.withdrawal = Some(KmModuleWithdrawal {
                    reason: reason.clone(),
                    withdrawn_at_unix_seconds: now_unix_seconds(),
                });
                updated = Some(record.clone());
            }
        }
        if updated.is_some() {
            self.save_index(&index)?;
        }
        Ok(updated)
    }

    pub(crate) fn list(&self, include_obsolete: bool) -> Result<Vec<KmIndexRecord>, KmStoreError> {
        let mut modules = self.load_index()?.modules;
        if !include_obsolete {
            modules.retain(|record| record.status == KmModuleStatus::Active);
        }
        sort_records(&mut modules);
        Ok(modules)
    }

    pub(crate) fn list_for_origin(
        &self,
        include_obsolete: bool,
        origin: Option<&KmIndexOrigin>,
        thread_id: Option<&str>,
    ) -> Result<Vec<KmIndexRecord>, KmStoreError> {
        let mut modules = self.load_index()?.modules;
        modules.retain(|record| matches_origin_filter(&record.origin, origin, thread_id));
        if !include_obsolete {
            modules.retain(|record| record.status == KmModuleStatus::Active);
        }
        sort_records(&mut modules);
        Ok(modules)
    }

    pub(crate) fn resolve_active(&self, id: &str) -> Result<Option<KmIndexRecord>, KmStoreError> {
        let modules = self.list(/*include_obsolete*/ false)?;
        Ok(modules
            .into_iter()
            .find(|record| record.id == id && record.withdrawal.is_none()))
    }

    /// Validates that a record's cached module directory is trustworthy before
    /// its content is read or injected, and returns the canonical directory.
    ///
    /// The stored `path` is never trusted as authority (PR-021). This recomputes
    /// the expected cache directory from the record id/version, requires the
    /// stored path to match it, rejects any directory that escapes the modules
    /// root (including via symlinks), and verifies the cached content hash so
    /// tampered caches are refused.
    pub(crate) fn validated_module_root(
        &self,
        record: &KmIndexRecord,
    ) -> Result<PathBuf, KmStoreError> {
        let expected = self.module_cache_path(&record.id, &record.version);
        if record.path != expected {
            return Err(KmStoreError::InvalidIndex);
        }
        if !expected.is_dir() {
            return Err(KmStoreError::InvalidIndex);
        }
        let canonical_modules_root =
            fs::canonicalize(&self.modules_root).map_err(|_| KmStoreError::InvalidIndex)?;
        let canonical_module =
            fs::canonicalize(&expected).map_err(|_| KmStoreError::InvalidIndex)?;
        if !canonical_module.starts_with(&canonical_modules_root) {
            return Err(KmStoreError::InvalidIndex);
        }
        validate_package_budget(&canonical_module).map_err(|_| KmStoreError::InvalidIndex)?;
        let actual_hash = hash_directory(&canonical_module)?;
        if actual_hash != record.content_hash {
            return Err(KmStoreError::InvalidIndex);
        }
        // Procedural records are optional, but when present they are part of
        // the activation boundary. Reuse the package loader so cached records
        // cannot bypass the transition count, endpoint, or attribute caps.
        if canonical_module.join("transitions.jsonl").is_file()
            && load_transitions(&canonical_module).is_err()
        {
            return Err(KmStoreError::InvalidIndex);
        }
        Ok(canonical_module)
    }

    fn load_index(&self) -> Result<KmIndex, KmStoreError> {
        let raw = match read_to_string(&self.index_path, MAX_KM_INDEX_BYTES) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(KmIndex {
                    schema_version: KM_INDEX_SCHEMA_VERSION,
                    modules: Vec::new(),
                });
            }
            Err(_) => return Err(KmStoreError::InvalidIndex),
        };
        let index =
            serde_json::from_str::<KmIndex>(&raw).map_err(|_| KmStoreError::InvalidIndex)?;
        if index.schema_version != KM_INDEX_SCHEMA_VERSION {
            return Err(KmStoreError::InvalidIndex);
        }
        Ok(index)
    }

    fn save_index(&self, index: &KmIndex) -> Result<(), KmStoreError> {
        fs::create_dir_all(&self.root).map_err(|_| KmStoreError::CreateCacheDir)?;
        let contents =
            serde_json::to_string_pretty(index).map_err(|_| KmStoreError::SerializeIndex)?;
        write_atomically(&self.index_path, &format!("{contents}\n"))
            .map_err(|_| KmStoreError::WriteIndex)
    }

    fn module_cache_path(&self, id: &str, version: &str) -> PathBuf {
        let mut path = self.modules_root.clone();
        for segment in id.split('/') {
            path.push(segment);
        }
        path.push(version);
        path
    }

    /// Cached version directories present on disk, as absolute paths.
    ///
    /// A version directory is one that directly contains `km.toml`, which is the
    /// same shape `module_cache_path` produces. Diagnostics compare this against
    /// the index to find cache directories no record points at.
    pub(crate) fn cached_version_dirs(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut pending = vec![self.modules_root.clone()];
        while let Some(dir) = pending.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                // symlink_metadata does not follow links, so a symlinked cache
                // directory is not traversed out of the store.
                let Ok(metadata) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if !metadata.is_dir() {
                    continue;
                }
                if path.join("km.toml").is_file() {
                    found.push(path);
                    continue;
                }
                pending.push(path);
            }
        }
        found.sort();
        found
    }

    fn ensure_cached_module(
        &self,
        source_path: &Path,
        destination: &Path,
        source_hash: &str,
    ) -> Result<(), KmStoreError> {
        if destination.exists() {
            let existing_hash = hash_directory(destination)?;
            if existing_hash != source_hash {
                return Err(KmStoreError::VersionConflict);
            }
            return Ok(());
        }

        let Some(parent) = destination.parent() else {
            return Err(KmStoreError::CreateCacheDir);
        };
        fs::create_dir_all(parent).map_err(|_| KmStoreError::CreateCacheDir)?;
        // Stage the copy in a sibling directory, verify its hash, then rename
        // it into place atomically so a failed import never leaves a partial or
        // orphaned package at the final path (PR-028).
        let staging = staging_path(destination);
        let _ = fs::remove_dir_all(&staging);
        copy_directory(source_path, &staging).inspect_err(|_| {
            let _ = fs::remove_dir_all(&staging);
        })?;
        let staged_hash = hash_directory(&staging).inspect_err(|_| {
            let _ = fs::remove_dir_all(&staging);
        })?;
        if staged_hash != source_hash {
            let _ = fs::remove_dir_all(&staging);
            return Err(KmStoreError::CopyModule);
        }
        fs::rename(&staging, destination).map_err(|_| {
            let _ = fs::remove_dir_all(&staging);
            KmStoreError::CopyModule
        })
    }

    /// Acquires the store-owned import lock, waiting up to
    /// `KM_IMPORT_LOCK_MAX_WAIT` and reclaiming an old lock only when its owner
    /// is known to be gone. The returned guard releases the lock on drop.
    fn acquire_import_lock(&self) -> Result<ImportLockGuard, KmStoreError> {
        fs::create_dir_all(&self.root).map_err(|_| KmStoreError::CreateCacheDir)?;
        let lock_path = self.root.join(KM_IMPORT_LOCK_FILE);
        let started = Instant::now();
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = write!(file, "{}", std::process::id());
                    return Ok(ImportLockGuard { path: lock_path });
                }
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                    if lock_is_stale(&lock_path) {
                        reclaim_stale_import_lock(&lock_path);
                        continue;
                    }
                    if started.elapsed() >= KM_IMPORT_LOCK_MAX_WAIT {
                        return Err(KmStoreError::WriteIndex);
                    }
                    std::thread::sleep(KM_IMPORT_LOCK_POLL);
                }
                Err(_) => return Err(KmStoreError::WriteIndex),
            }
        }
    }
}

fn validate_import_source(source_path: &Path, project_scope: Option<&str>) -> KmValidationReport {
    let Some(project_scope) = project_scope else {
        return validate_km(source_path);
    };
    let project_root = Path::new(project_scope);
    if !project_root.is_absolute() {
        return invalid_source_report(source_path, "project scope must be an absolute path");
    }
    match fs::canonicalize(project_root) {
        Ok(project_root) if project_root.is_dir() => {
            validate_km_for_project(source_path, &project_root)
        }
        _ => validate_km(source_path),
    }
}

fn invalid_source_report(source_path: &Path, error: &str) -> KmValidationReport {
    let mut report = validate_km(source_path);
    report.status = KmValidationStatus::Invalid;
    report.errors.push(error.to_string());
    report
}

/// Releases the store import lock when dropped.
struct ImportLockGuard {
    path: PathBuf,
}

impl Drop for ImportLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn staging_path(destination: &Path) -> PathBuf {
    let mut file_name = destination
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "module".to_string());
    file_name.push_str(".staging");
    match destination.parent() {
        Some(parent) => parent.join(file_name),
        None => PathBuf::from(file_name),
    }
}

/// Reclaims a stale lock by renaming it aside before deleting it.
///
/// Deleting in place is check-then-act: two reclaimers can both observe the
/// same stale lock, and the second `remove_file` can delete a fresh lock that
/// the first reclaimer already created, leaving two live lock holders. The
/// rename is atomic, so exactly one reclaimer captures the stale file and any
/// loser's rename fails harmlessly against the already-moved path.
fn reclaim_stale_import_lock(lock_path: &Path) {
    let claimed = lock_path.with_extension(format!("reclaim.{}", std::process::id()));
    if fs::rename(lock_path, &claimed).is_ok() {
        let _ = fs::remove_file(&claimed);
    }
}

fn lock_is_stale(lock_path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    let old_enough = modified
        .elapsed()
        .map(|age| age >= KM_IMPORT_LOCK_STALE)
        .unwrap_or(false);
    if !old_enough {
        return false;
    }
    let pid = fs::read_to_string(lock_path)
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok());
    should_reclaim_import_lock(old_enough, pid)
}

fn should_reclaim_import_lock(old_enough: bool, owner_pid: Option<u32>) -> bool {
    old_enough && owner_pid.is_some_and(|pid| !process_is_alive(pid))
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    if pid == 0 || pid > libc::pid_t::MAX as u32 {
        return false;
    }
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(unix))]
fn process_is_alive(_pid: u32) -> bool {
    // Without a portable process probe, retaining an old parseable lock is
    // safer than reclaiming one held by another live process.
    true
}

fn sort_records(records: &mut [KmIndexRecord]) {
    records.sort_by_key(|record| {
        (
            record.status != KmModuleStatus::Active,
            record.id.clone(),
            Reverse(parse_version(&record.version)),
        )
    });
}

fn parse_version(version: &str) -> Version {
    Version::parse(version).unwrap_or_else(|_| Version::new(0, 0, 0))
}

fn backfill_topic_words(
    records: &mut [KmIndexRecord],
    id: &str,
    version: &str,
    source_path: &Path,
) {
    let Some(record) = records
        .iter_mut()
        .find(|record| record.id == id && record.version == version)
    else {
        return;
    };
    if record.topic_words.is_empty() {
        record.topic_words = collect_topic_words(source_path);
    }
}

fn backfill_project_scope(
    records: &mut [KmIndexRecord],
    id: &str,
    version: &str,
    project_scope: Option<&str>,
) {
    let Some(project_scope) = project_scope else {
        return;
    };
    let Some(record) = records
        .iter_mut()
        .find(|record| record.id == id && record.version == version)
    else {
        return;
    };
    if record.project_scope.is_none() {
        record.project_scope = Some(project_scope.to_string());
    }
}

pub(crate) fn project_root(path: &Path) -> Option<PathBuf> {
    let mut current = if path.is_file() {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    };
    loop {
        let git_path = current.join(".git");
        if git_path.is_dir() || git_path.is_file() {
            return Some(fs::canonicalize(&current).unwrap_or(current));
        }
        if !current.pop() {
            return None;
        }
    }
}

fn collect_topic_words(source_path: &Path) -> Vec<String> {
    let mut words = BTreeSet::new();
    if let Ok(manifest) = load_manifest(source_path)
        && let Some(summary) = manifest.summary
    {
        collect_words_from_text(&summary, &mut words);
    }
    if let Ok(facts) = load_facts(source_path) {
        for fact in facts.iter().take(KM_TOPIC_FACT_CLAIM_LIMIT) {
            collect_words_from_text(&fact.claim, &mut words);
        }
    }
    words.into_iter().collect()
}

fn collect_words_from_text(text: &str, words: &mut BTreeSet<String>) {
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
            continue;
        }
        insert_topic_word(&mut current, words);
    }
    insert_topic_word(&mut current, words);
}

fn insert_topic_word(current: &mut String, words: &mut BTreeSet<String>) {
    if current.len() >= 3 && !KM_TOPIC_STOP_WORDS.contains(&current.as_str()) {
        words.insert(std::mem::take(current));
    } else {
        current.clear();
    }
}

fn mark_other_versions_obsolete(records: &mut [KmIndexRecord], id: &str) {
    for record in records {
        if record.id == id {
            record.status = KmModuleStatus::Obsolete;
        }
    }
}

fn mark_active_version(records: &mut [KmIndexRecord], id: &str, version: &str) {
    for record in records {
        if record.id != id {
            continue;
        }
        record.status = if record.version == version {
            KmModuleStatus::Active
        } else {
            KmModuleStatus::Obsolete
        };
    }
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), KmStoreError> {
    fs::create_dir_all(destination).map_err(|_| KmStoreError::CopyModule)?;
    let mut entries = fs::read_dir(source)
        .map_err(|_| KmStoreError::CopyModule)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| KmStoreError::CopyModule)?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = entry.metadata().map_err(|_| KmStoreError::CopyModule)?;
        if metadata.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|_| KmStoreError::CopyModule)?;
        }
    }
    Ok(())
}

pub(crate) fn hash_directory(root: &Path) -> Result<String, KmStoreError> {
    let mut hasher = Sha256::new();
    let mut remaining = crate::km::MAX_PACKAGE_TOTAL_BYTES;
    hash_directory_inner(root, root, &mut hasher, &mut remaining)?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn hash_directory_inner(
    root: &Path,
    current: &Path,
    hasher: &mut Sha256,
    remaining: &mut u64,
) -> Result<(), KmStoreError> {
    let mut entries = fs::read_dir(current)
        .map_err(|_| KmStoreError::HashModule)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| KmStoreError::HashModule)?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let metadata = entry.metadata().map_err(|_| KmStoreError::HashModule)?;
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| KmStoreError::HashModule)?;
        let relative = relative_path.to_string_lossy().replace('\\', "/");
        hasher.update(relative.as_bytes());
        if metadata.is_dir() {
            hasher.update(b"/");
            hash_directory_inner(root, &path, hasher, remaining)?;
            continue;
        }
        let mut bytes = Vec::new();
        crate::artifact_io::open_beneath(root, relative_path)
            .map_err(|_| KmStoreError::HashModule)?
            .take(*remaining + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| KmStoreError::HashModule)?;
        let byte_len = bytes.len() as u64;
        if byte_len > *remaining {
            return Err(KmStoreError::HashModule);
        }
        *remaining -= byte_len;
        hasher.update(byte_len.to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(())
}

fn validate_index_origin(origin: Option<&KmIndexOrigin>) -> Result<(), KmStoreError> {
    match origin {
        Some(KmIndexOrigin::AgentThread { thread_id }) if thread_id.trim().is_empty() => {
            Err(KmStoreError::InvalidOrigin)
        }
        Some(KmIndexOrigin::AgentThread { thread_id: _ }) | Some(KmIndexOrigin::Human) | None => {
            Ok(())
        }
    }
}

fn matches_origin_filter(
    record_origin: &Option<KmIndexOrigin>,
    origin: Option<&KmIndexOrigin>,
    thread_id: Option<&str>,
) -> bool {
    match (origin, thread_id) {
        (Some(KmIndexOrigin::Human), None) => matches!(record_origin, Some(KmIndexOrigin::Human)),
        (
            Some(KmIndexOrigin::AgentThread {
                thread_id: requested,
            }),
            None,
        ) => matches!(
            record_origin,
            Some(KmIndexOrigin::AgentThread { thread_id }) if thread_id == requested
        ),
        (Some(KmIndexOrigin::Human), Some(requested)) => {
            matches!(record_origin, Some(KmIndexOrigin::Human)) && requested.is_empty()
        }
        (
            Some(KmIndexOrigin::AgentThread {
                thread_id: requested,
            }),
            Some(requested_thread),
        ) => {
            matches!(
                record_origin,
                Some(KmIndexOrigin::AgentThread { thread_id })
                    if thread_id == requested && requested_thread == requested
            )
        }
        (None, Some(requested_thread)) => {
            matches!(
                record_origin,
                Some(KmIndexOrigin::AgentThread { thread_id }) if thread_id == requested_thread
            )
        }
        (None, None) => true,
    }
}

fn backfill_origin(
    records: &mut [KmIndexRecord],
    id: &str,
    version: &str,
    origin: &Option<KmIndexOrigin>,
) {
    let Some(record) = records
        .iter_mut()
        .find(|record| record.id == id && record.version == version)
    else {
        return;
    };
    if record.origin.is_none() {
        record.origin = origin.clone();
    }
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
#[path = "km_store_tests.rs"]
mod tests;
