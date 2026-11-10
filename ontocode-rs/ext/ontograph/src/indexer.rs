#[cfg(test)]
use std::cell::Cell;
#[cfg(test)]
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use ontocode_secrets::redact_secrets;
use ontocode_utils_path::write_atomically;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

use crate::artifact_io::read_to_string;
use crate::docs_index::DocsIndexBuildError;
use crate::docs_index::DocsIndexRequest;
use crate::docs_index::build_and_publish_docs_index;
use crate::manifest::ArtifactGeneration;
use crate::manifest::ArtifactPublishError;
use crate::manifest::ArtifactVersion;
use crate::manifest::GenerationMetadata;
use crate::manifest::GraphCountSummary;
use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;
use crate::manifest::RepositoryIdentity;
use crate::manifest::SOURCE_INDEX_FILE;
use crate::manifest::repo_ontograph_root;
use crate::query::BoundedItems;
use crate::query::DiffImpactFile;
use crate::query::DiffImpactProcess;
use crate::query::DiffImpactRequest;
use crate::query::DiffImpactResult;
use crate::query::DiffImpactSummary;
use crate::query::DiffImpactSymbol;
use crate::query::EvidenceItem;
use crate::query::EvidenceRequest;
use crate::query::EvidenceResult;
use crate::query::GraphSccInputItem;
use crate::query::GraphSccInputRequest;
use crate::query::GraphSccInputResult;
use crate::query::ImpactItem;
use crate::query::ModuleContextItem;
use crate::query::ModuleContextRequest;
use crate::query::ModuleContextResult;
use crate::query::OntographReadQuery;
pub(crate) use crate::query::QUERY_DEADLINE_CHECK_STRIDE;
use crate::query::QueryBudget;
use crate::query::QueryDeadline;
use crate::query::QueryError;
use crate::query::QueryResult;
use crate::query::RepoMapFileItem;
use crate::query::RepoMapRequest;
use crate::query::RepoMapResult;
use crate::query::RouteImpactItem;
use crate::query::RouteImpactRequest;
use crate::query::RouteImpactResult;
use crate::query::RouteMapItem;
use crate::query::RouteMapRequest;
use crate::query::RouteMapResult;
use crate::query::RouteShapeItem;
use crate::query::RouteShapeRequest;
use crate::query::RouteShapeResult;
use crate::query::SourceReferenceSet;
use crate::query::SymbolImpactRequest;
use crate::query::SymbolImpactResult;
use crate::query::SymbolSearchItem;
use crate::query::SymbolSearchRequest;
use crate::query::SymbolSearchResult;
use crate::query::TestLinkItem;
use crate::query::TestLinkRequest;
use crate::query::TestLinkResult;
use crate::query::UnresolvedEvidenceItem;
use crate::query::bounded_serialized_items;
use crate::query::check_query_scan_deadline;
#[cfg(test)]
pub(crate) use crate::query::clear_query_scan_timeout;
#[cfg(test)]
pub(crate) use crate::query::force_query_scan_timeout_after_items;
use crate::query::query_scan_step;
#[cfg(test)]
pub(crate) use crate::query::query_scan_test_items;
use crate::resource_facts::IndexedResourceFact;
use crate::resource_facts::MAX_RESOURCE_FACTS;
use crate::resource_facts::collect_resource_facts;
use crate::rust_parser::ParsedCallForm;
use crate::rust_parser::ParsedFile;
use crate::rust_parser::ParsedImport;
use crate::rust_parser::parse_rust_file;

const MAX_READ_FILE_ERROR_PATH_BYTES: usize = 256;
const MAX_ARTIFACT_READ_BYTES: usize = 64 * 1024 * 1024;
const MAX_POINTER_READ_BYTES: usize = 4096;
const INDEX_REBUILD_LOCK_STALE: Duration = Duration::from_secs(30);
const UNVERSIONED_TARGET_HEAD: &str = "workspace-unversioned";
static INDEX_REBUILD_LOCK_CLAIM: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    static FULL_WORKSPACE_WALK_COUNT: Cell<u64> = const { Cell::new(0) };
    static SOURCE_SEARCH_TEST_OBSERVATION: Cell<(usize, Option<u64>)> =
        const { Cell::new((0, None)) };
    static IMPACT_SYMBOL_TEST_OBSERVATION: RefCell<Vec<(String, usize, u64)>> =
        const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn reset_source_search_test_observation() {
    SOURCE_SEARCH_TEST_OBSERVATION.with(|observation| observation.set((0, None)));
}

#[cfg(test)]
pub(crate) fn source_search_test_observation() -> (usize, Option<u64>) {
    SOURCE_SEARCH_TEST_OBSERVATION.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn reset_impact_symbol_test_observation() {
    IMPACT_SYMBOL_TEST_OBSERVATION.with(|observation| observation.borrow_mut().clear());
}

#[cfg(test)]
pub(crate) fn impact_symbol_test_observation() -> Vec<(String, usize, u64)> {
    IMPACT_SYMBOL_TEST_OBSERVATION.with(|observation| observation.borrow().clone())
}

#[cfg(test)]
#[path = "km_gold.rs"]
pub(crate) mod km_gold;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RustIndexRequest {
    pub repo_root: PathBuf,
    pub artifact_root: OntographArtifactRoot,
    pub generation_id: String,
    pub target_head: String,
    pub implementation_version: String,
    pub timestamp_unix_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SourceIndexArtifact {
    pub schema_version: u32,
    pub files: Vec<IndexedFile>,
    pub relationships: Vec<IndexedRelationship>,
    pub processes: Vec<IndexedProcess>,
    #[serde(default)]
    pub routes: Vec<IndexedRoute>,
    #[serde(default)]
    pub unresolved_calls: Vec<IndexedUnresolvedCall>,
    #[serde(default)]
    pub resource_facts: Vec<IndexedResourceFact>,
    #[serde(default)]
    pub resource_facts_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedFile {
    #[serde(default)]
    pub node_id: String,
    pub path: String,
    pub language: String,
    pub symbols: Vec<IndexedSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedSymbol {
    #[serde(default)]
    pub node_id: String,
    pub name: String,
    pub kind: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedRelationship {
    #[serde(default)]
    pub relationship_id: String,
    #[serde(default)]
    pub from_node_id: String,
    pub from_symbol: String,
    pub from_path: String,
    pub from_line: usize,
    #[serde(default)]
    pub to_node_id: String,
    pub to_symbol: String,
    pub to_path: String,
    pub to_line: usize,
    pub kind: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedProcess {
    #[serde(default)]
    pub node_id: String,
    pub name: String,
    pub entry_symbol: String,
    pub path: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedRoute {
    #[serde(default)]
    pub node_id: String,
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct IndexedUnresolvedCall {
    pub unresolved_call_id: String,
    pub from_node_id: String,
    pub from_symbol: String,
    pub from_path: String,
    pub from_line: usize,
    pub target_symbol: String,
    pub path: String,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RustIndexError {
    Walk,
    ReadFile(String),
    RebuildInProgress,
    WriteLock,
    Serialize,
    WriteArtifact,
    PublishManifest(ArtifactPublishError),
    DocsIndex(DocsIndexBuildError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceIndexStore {
    artifact: SourceIndexArtifact,
    repo_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceIndexLoadError {
    MissingCurrentPointer,
    InvalidCurrentPointer,
    MissingSourceIndex,
    ReadSourceIndex,
    InvalidSourceIndex,
}

pub(crate) fn index_rust_workspace(
    request: RustIndexRequest,
) -> Result<(SourceIndexArtifact, Vec<String>), RustIndexError> {
    let _lock = acquire_index_rebuild_lock(&request.artifact_root)?;
    let files = collect_rust_files(&request.repo_root)?;
    let mut working_files = Vec::with_capacity(files.len());
    for path in files {
        let relative_path = relative_slash_path(&request.repo_root, &path)?;
        let content = fs::read_to_string(&path)
            .map_err(|_| RustIndexError::ReadFile(sanitize_read_file_error_path(&relative_path)))?;
        let parsed = parse_rust_file(&content);
        let symbols = parsed
            .as_ref()
            .map(|parsed| {
                parsed
                    .symbols
                    .iter()
                    .map(|symbol| IndexedSymbol {
                        node_id: symbol_node_id(
                            &relative_path,
                            &symbol.kind,
                            &symbol.name,
                            symbol.line,
                        ),
                        name: symbol.name.clone(),
                        kind: symbol.kind.clone(),
                        line: symbol.line,
                    })
                    .collect()
            })
            .unwrap_or_else(|| collect_rust_symbols(&content, &relative_path));
        working_files.push(WorkingFile {
            indexed: IndexedFile {
                node_id: file_node_id(&relative_path),
                path: relative_path,
                language: "rust".to_string(),
                symbols,
            },
            parsed,
            content,
        });
    }

    let (relationships, unresolved_calls) = collect_relationships(&working_files);
    let processes = collect_processes(&relationships);
    let routes = collect_routes(&working_files);
    let mut resource_facts = working_files
        .iter()
        .flat_map(|file| {
            collect_resource_facts(&file.indexed.path, &file.content, file.parsed.as_ref())
        })
        .collect::<Vec<_>>();
    resource_facts.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.operation.cmp(&right.operation))
            .then_with(|| left.resource_fact_id.cmp(&right.resource_fact_id))
    });
    let resource_facts_truncated = resource_facts.len() > MAX_RESOURCE_FACTS;
    resource_facts.truncate(MAX_RESOURCE_FACTS);
    let indexed_files = working_files
        .into_iter()
        .map(|file| file.indexed)
        .collect::<Vec<_>>();
    let artifact = SourceIndexArtifact {
        schema_version: 1,
        files: indexed_files,
        relationships,
        processes,
        routes,
        unresolved_calls,
        resource_facts,
        resource_facts_truncated,
    };
    let limits = publish_index_artifact(&request, &artifact)?;
    Ok((artifact, limits))
}

fn sanitize_read_file_error_path(path: &str) -> String {
    let mut path = redact_secrets(path.to_string())
        .chars()
        .map(|character| {
            if character.is_control() {
                '?'
            } else {
                character
            }
        })
        .collect::<String>();
    if path.len() <= MAX_READ_FILE_ERROR_PATH_BYTES {
        return path;
    }

    let mut end = MAX_READ_FILE_ERROR_PATH_BYTES - 3;
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    path.truncate(end);
    path.push_str("...");
    path
}

pub(crate) fn rust_workspace_state_key(
    repo_root: &Path,
    target_head: &str,
) -> Result<String, RustIndexError> {
    if target_head == UNVERSIONED_TARGET_HEAD {
        return full_rust_workspace_state_key(repo_root);
    }

    let output = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args([
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            "*.rs",
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|_| RustIndexError::Walk)?;
    if !output.status.success() {
        return full_rust_workspace_state_key(repo_root);
    }

    let mut dirty_paths = parse_porcelain_rust_paths(&output.stdout);
    dirty_paths.sort();
    dirty_paths.dedup();
    let mut hash = 0xcbf29ce484222325_u64;
    for relative_path in dirty_paths {
        hash_bytes(&mut hash, relative_path.as_bytes());
        match fs::metadata(repo_root.join(&relative_path)) {
            Ok(metadata) => hash_file_metadata(&mut hash, &metadata)?,
            Err(err) if err.kind() == io::ErrorKind::NotFound => mix_u64(&mut hash, u64::MAX),
            Err(_) => return Err(RustIndexError::Walk),
        }
    }
    Ok(format!("{hash:016x}"))
}

pub(crate) fn resolve_target_head(repo_root: &Path) -> String {
    Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|head| head.trim().to_string())
        .filter(|head| !head.is_empty())
        .unwrap_or_else(|| UNVERSIONED_TARGET_HEAD.to_string())
}

fn full_rust_workspace_state_key(repo_root: &Path) -> Result<String, RustIndexError> {
    let files = collect_rust_files(repo_root)?;
    let mut hash = 0xcbf29ce484222325_u64;
    for path in files {
        let relative_path = relative_slash_path(repo_root, &path)?;
        hash_bytes(&mut hash, relative_path.as_bytes());
        let metadata = fs::metadata(&path).map_err(|_| RustIndexError::Walk)?;
        hash_file_metadata(&mut hash, &metadata)?;
    }
    Ok(format!("{hash:016x}"))
}

fn hash_file_metadata(hash: &mut u64, metadata: &fs::Metadata) -> Result<(), RustIndexError> {
    mix_u64(hash, metadata.len());
    let modified = metadata
        .modified()
        .map_err(|_| RustIndexError::Walk)?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    mix_u64(hash, modified.as_secs());
    mix_u64(hash, modified.subsec_nanos() as u64);
    Ok(())
}

fn parse_porcelain_rust_paths(output: &[u8]) -> Vec<String> {
    let mut records = output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut paths = Vec::new();
    while let Some(record) = records.next() {
        if record.len() < 4 {
            continue;
        }
        let status = &record[..2];
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        if path.ends_with(".rs") {
            paths.push(path);
        }
        if status.iter().any(|status| matches!(status, b'R' | b'C')) {
            let _ = records.next();
        }
    }
    paths
}

struct IndexRebuildLockGuard {
    path: PathBuf,
}

impl Drop for IndexRebuildLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_index_rebuild_lock(
    artifact_root: &OntographArtifactRoot,
) -> Result<IndexRebuildLockGuard, RustIndexError> {
    fs::create_dir_all(artifact_root.root_path()).map_err(|_| RustIndexError::WriteLock)?;
    let lock_path = artifact_root.index_rebuild_lock_path();
    if lock_is_stale(&lock_path) {
        reclaim_stale_index_lock(&lock_path);
    }

    let claim = INDEX_REBUILD_LOCK_CLAIM.fetch_add(1, Ordering::Relaxed);
    let claim_path = lock_path.with_extension(format!("claim.{}.{}", std::process::id(), claim));
    write_atomically(&claim_path, &std::process::id().to_string())
        .map_err(|_| RustIndexError::WriteLock)?;
    let result = fs::hard_link(&claim_path, &lock_path);
    let _ = fs::remove_file(&claim_path);
    match result {
        Ok(()) => Ok(IndexRebuildLockGuard { path: lock_path }),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            Err(RustIndexError::RebuildInProgress)
        }
        Err(_) => Err(RustIndexError::WriteLock),
    }
}

fn lock_is_stale(lock_path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(lock_path) else {
        return false;
    };
    let old_enough = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= INDEX_REBUILD_LOCK_STALE);
    let owner_pid = fs::read_to_string(lock_path)
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok());
    old_enough && owner_pid.is_some_and(|pid| !process_is_alive(pid))
}

fn reclaim_stale_index_lock(lock_path: &Path) {
    let claimed = lock_path.with_extension(format!("reclaim.{}", std::process::id()));
    if fs::rename(lock_path, &claimed).is_ok() {
        let _ = fs::remove_file(claimed);
    }
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
    true
}

#[cfg(test)]
pub(crate) fn reset_full_workspace_walk_count() {
    FULL_WORKSPACE_WALK_COUNT.set(0);
}

#[cfg(test)]
pub(crate) fn full_workspace_walk_count() -> u64 {
    FULL_WORKSPACE_WALK_COUNT.get()
}

impl SourceIndexStore {
    pub(crate) fn load_active_generation(
        root: &OntographArtifactRoot,
    ) -> Result<Self, SourceIndexLoadError> {
        let generation_id = read_to_string(&root.current_pointer_path(), MAX_POINTER_READ_BYTES)
            .map_err(|err| match err.kind() {
                io::ErrorKind::NotFound => SourceIndexLoadError::MissingCurrentPointer,
                _ => SourceIndexLoadError::InvalidCurrentPointer,
            })?;
        let source_index_path = root
            .generation_file_path(generation_id.trim(), SOURCE_INDEX_FILE)
            .map_err(|_| SourceIndexLoadError::InvalidCurrentPointer)?;
        let contents =
            read_to_string(&source_index_path, MAX_ARTIFACT_READ_BYTES).map_err(|err| match err
                .kind()
            {
                io::ErrorKind::NotFound => SourceIndexLoadError::MissingSourceIndex,
                _ => SourceIndexLoadError::ReadSourceIndex,
            })?;
        let artifact = serde_json::from_str::<SourceIndexArtifact>(&contents)
            .map_err(|_| SourceIndexLoadError::InvalidSourceIndex)?;
        Ok(Self {
            artifact,
            repo_root: root.repository_root().to_path_buf(),
        })
    }

    pub(crate) fn load_completed_generation(
        root: &OntographArtifactRoot,
        generation_id: &str,
    ) -> Result<CompletedSourceIndexGeneration, SourceIndexLoadError> {
        let generation_id = generation_id.trim();
        let manifest_path = root
            .generation_manifest_path(generation_id)
            .map_err(|_| SourceIndexLoadError::InvalidCurrentPointer)?;
        let manifest_contents =
            read_to_string(&manifest_path, MAX_ARTIFACT_READ_BYTES).map_err(|err| {
                match err.kind() {
                    io::ErrorKind::NotFound => SourceIndexLoadError::MissingSourceIndex,
                    _ => SourceIndexLoadError::ReadSourceIndex,
                }
            })?;
        let manifest = serde_json::from_str::<OntographManifest>(&manifest_contents)
            .map_err(|_| SourceIndexLoadError::InvalidSourceIndex)?;
        let generation = manifest
            .completed_generations
            .iter()
            .find(|candidate| candidate.metadata.generation_id == generation_id)
            .cloned()
            .ok_or(SourceIndexLoadError::InvalidSourceIndex)?;

        let source_index_path = root
            .generation_file_path(generation_id, SOURCE_INDEX_FILE)
            .map_err(|_| SourceIndexLoadError::InvalidCurrentPointer)?;
        let contents =
            read_to_string(&source_index_path, MAX_ARTIFACT_READ_BYTES).map_err(|err| match err
                .kind()
            {
                io::ErrorKind::NotFound => SourceIndexLoadError::MissingSourceIndex,
                _ => SourceIndexLoadError::ReadSourceIndex,
            })?;
        let artifact = serde_json::from_str::<SourceIndexArtifact>(&contents)
            .map_err(|_| SourceIndexLoadError::InvalidSourceIndex)?;
        let content_hash = source_index_content_hash(&contents);
        if generation.metadata.manifest_hash != content_hash {
            return Err(SourceIndexLoadError::InvalidSourceIndex);
        }

        Ok(CompletedSourceIndexGeneration {
            manifest,
            generation,
            artifact,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletedSourceIndexGeneration {
    pub manifest: OntographManifest,
    pub generation: ArtifactGeneration,
    pub artifact: SourceIndexArtifact,
}

impl OntographReadQuery for SourceIndexStore {
    fn search_symbols(&self, request: SymbolSearchRequest) -> QueryResult<SymbolSearchResult> {
        #[cfg(test)]
        SOURCE_SEARCH_TEST_OBSERVATION.with(|observation| {
            let (request_count, _) = observation.get();
            observation.set((request_count + 1, Some(request.budget.timeout_ms)));
        });
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let query = SearchQuery::new(&request.query);
        let mut matches = Vec::new();
        // Keep one overflow item to preserve the existing truncation semantics.
        let candidate_limit = request.budget.max_items.saturating_add(1);
        let mut scanned = 0;
        check_query_scan_deadline(deadline)?;
        for file in &self.artifact.files {
            if file.symbols.is_empty() {
                query_scan_step();
                scanned += 1;
                if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                    check_query_scan_deadline(deadline)?;
                }
            }
            for symbol in &file.symbols {
                query_scan_step();
                scanned += 1;
                if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                    check_query_scan_deadline(deadline)?;
                }
                if let Some(score) = query.score_symbol(&symbol.name, &file.path) {
                    let position = matches
                        .partition_point(|ranked: &RankedSymbolSearchItem| ranked.score <= score);
                    if position >= candidate_limit {
                        continue;
                    }
                    if matches.len() == candidate_limit {
                        matches.pop();
                    }
                    matches.insert(
                        position,
                        RankedSymbolSearchItem {
                            score,
                            item: SymbolSearchItem {
                                symbol: symbol.name.clone(),
                                kind: symbol.kind.clone(),
                                path: file.path.clone(),
                            },
                        },
                    );
                }
            }
        }
        check_query_scan_deadline(deadline)?;
        let matches = matches
            .into_iter()
            .map(|ranked| ranked.item)
            .collect::<Vec<_>>();
        let result = SymbolSearchResult {
            matches: bounded_items(matches, request.budget)?,
        };
        check_query_scan_deadline(deadline)?;
        Ok(result)
    }

    fn search_repomap(&self, request: RepoMapRequest) -> QueryResult<RepoMapResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let query = SearchQuery::new(&request.query);
        let mut files = Vec::new();
        let candidate_limit = request.budget.max_items.saturating_add(1);
        // A zero-item request still needs the best symbol score to rank its file.
        let symbol_limit = request.budget.max_items.max(1);
        let mut scanned = 0;
        check_query_scan_deadline(deadline)?;
        for file in &self.artifact.files {
            query_scan_step();
            scanned += 1;
            if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                check_query_scan_deadline(deadline)?;
            }
            let path_score = query.score_path(&file.path);
            let mut symbols = Vec::new();
            for symbol in &file.symbols {
                query_scan_step();
                scanned += 1;
                if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                    check_query_scan_deadline(deadline)?;
                }
                if let Some(score) = query.score_symbol(&symbol.name, &file.path).or(path_score) {
                    let position = symbols
                        .partition_point(|ranked: &RankedSymbolSearchItem| ranked.score <= score);
                    if position >= symbol_limit {
                        continue;
                    }
                    if symbols.len() == symbol_limit {
                        symbols.pop();
                    }
                    symbols.insert(
                        position,
                        RankedSymbolSearchItem {
                            score,
                            item: SymbolSearchItem {
                                symbol: symbol.name.clone(),
                                kind: symbol.kind.clone(),
                                path: file.path.clone(),
                            },
                        },
                    );
                }
            }
            let Some(best_score) = symbols.first().map(|ranked| ranked.score).or(path_score) else {
                continue;
            };
            let position = files.partition_point(|ranked: &RankedRepoMapFileItem| {
                (ranked.score, ranked.item.path.as_str()) <= (best_score, file.path.as_str())
            });
            if position >= candidate_limit {
                continue;
            }
            if files.len() == candidate_limit {
                files.pop();
            }
            let symbols = symbols
                .into_iter()
                .take(request.budget.max_items)
                .map(|ranked| ranked.item)
                .collect::<Vec<_>>();
            files.insert(
                position,
                RankedRepoMapFileItem {
                    score: best_score,
                    item: RepoMapFileItem {
                        path: file.path.clone(),
                        language: file.language.clone(),
                        symbols,
                    },
                },
            );
        }
        check_query_scan_deadline(deadline)?;
        let files = files
            .into_iter()
            .map(|ranked| ranked.item)
            .collect::<Vec<_>>();
        let result = RepoMapResult {
            files: bounded_items(files, request.budget)?,
        };
        check_query_scan_deadline(deadline)?;
        Ok(result)
    }

    fn inspect_module(&self, request: ModuleContextRequest) -> QueryResult<ModuleContextResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let file = self
            .artifact
            .files
            .iter()
            .find(|file| file.path == request.path)
            .ok_or(QueryError::MissingArtifact)?;
        let symbols = file
            .symbols
            .iter()
            .map(|symbol| SymbolSearchItem {
                symbol: symbol.name.clone(),
                kind: symbol.kind.clone(),
                path: file.path.clone(),
            })
            .collect::<Vec<_>>();
        deadline.check()?;
        Ok(ModuleContextResult {
            module: ModuleContextItem {
                path: file.path.clone(),
                language: Some(file.language.clone()),
                symbols: bounded_items(symbols, request.budget)?,
            },
        })
    }

    fn inspect_evidence(&self, request: EvidenceRequest) -> QueryResult<EvidenceResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let mut evidence = Vec::new();
        let mut unresolved = Vec::new();
        let mut parse_cache: HashMap<String, Option<ParsedFile>> = HashMap::new();
        for target in request.targets {
            if let Some(item) =
                self.resolve_line_evidence(&target, request.include_snippet, request.context_lines)?
            {
                evidence.push(item);
                deadline.check()?;
                continue;
            }
            if let Some(item) =
                self.resolve_file_evidence(&target, request.include_snippet, request.context_lines)?
            {
                evidence.push(item);
                deadline.check()?;
                continue;
            }
            match self.resolve_symbol_evidence(
                &target,
                request.include_snippet,
                request.context_lines,
                &mut parse_cache,
            )? {
                Some(item) => evidence.push(item),
                None => unresolved.push(UnresolvedEvidenceItem {
                    target: target.clone(),
                    reason: "no matching indexed line, file, or symbol found".to_string(),
                }),
            }
            deadline.check()?;
        }
        Ok(EvidenceResult {
            evidence: bounded_items(evidence, request.budget.clone())?,
            unresolved: bounded_items(unresolved, request.budget)?,
        })
    }

    fn inspect_shape(&self, request: RouteShapeRequest) -> QueryResult<RouteShapeResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let routes = self
            .discover_routes(RouteMapRequest {
                route: request.route,
                path: request.path,
                budget: request.budget.clone(),
            })?
            .routes;
        if routes.items.is_empty() {
            return Err(QueryError::MissingArtifact);
        }
        let routes_truncated = routes.truncated;
        let mut items = routes
            .items
            .into_iter()
            .map(|route| RouteShapeItem {
                method: route.method,
                route_path: route.route_path,
                handler_symbol: route.handler_symbol,
                path: route.path,
                line: route.line,
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.route_path
                .cmp(&right.route_path)
                .then(left.method.cmp(&right.method))
                .then(left.path.cmp(&right.path))
                .then(left.line.cmp(&right.line))
        });
        deadline.check()?;
        let mut routes = bounded_items(items, request.budget)?;
        routes.truncated |= routes_truncated;
        Ok(RouteShapeResult { routes })
    }

    fn impact_symbol(&self, request: SymbolImpactRequest) -> QueryResult<SymbolImpactResult> {
        #[cfg(test)]
        IMPACT_SYMBOL_TEST_OBSERVATION.with(|observation| {
            observation.borrow_mut().push((
                request.symbol.clone(),
                request.budget.max_items,
                request.budget.timeout_ms,
            ));
        });
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let matching_symbols = self.find_symbol_locations(&request.symbol, None, deadline)?;
        let [(symbol_path, symbol_line)] = matching_symbols.as_slice() else {
            return Err(if matching_symbols.is_empty() {
                QueryError::MissingArtifact
            } else {
                QueryError::AmbiguousArtifact
            });
        };
        self.render_symbol_impact_at(
            &request.symbol,
            symbol_path,
            *symbol_line,
            request.budget,
            deadline,
        )
    }

    fn impact_route(&self, request: RouteImpactRequest) -> QueryResult<RouteImpactResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let routes = self
            .discover_routes(RouteMapRequest {
                route: request.route,
                path: request.path,
                budget: request.budget.clone(),
            })?
            .routes;
        if routes.items.is_empty() {
            return Err(QueryError::MissingArtifact);
        }
        let routes_truncated = routes.truncated;
        let mut items = Vec::with_capacity(routes.items.len());
        for route in routes.items {
            let matches = self.find_symbol_locations(
                &route.handler_symbol,
                Some(route.path.as_str()),
                deadline,
            )?;
            let [(symbol_path, symbol_line)] = matches.as_slice() else {
                return Err(if matches.is_empty() {
                    QueryError::MissingArtifact
                } else {
                    QueryError::AmbiguousArtifact
                });
            };
            let impact = self.render_symbol_impact_at(
                &route.handler_symbol,
                symbol_path,
                *symbol_line,
                request.budget.clone(),
                deadline,
            )?;
            items.push(RouteImpactItem {
                method: route.method,
                route_path: route.route_path,
                handler_symbol: route.handler_symbol,
                path: route.path,
                line: route.line,
                upstream: impact.upstream,
                downstream: impact.downstream,
            });
            deadline.check()?;
        }
        let mut routes = bounded_items(items, request.budget)?;
        routes.truncated |= routes_truncated;
        Ok(RouteImpactResult { routes })
    }

    fn impact_diff(&self, request: DiffImpactRequest) -> QueryResult<DiffImpactResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let diff_output =
            git_diff_output(&self.repo_root, &request.scope, request.base_ref.as_deref())?;
        deadline.check()?;

        let mut warnings = Vec::new();
        let changed_files =
            parse_changed_file_ranges(&diff_output, &request.include_paths, &mut warnings);
        if changed_files.is_empty() {
            return Ok(DiffImpactResult {
                summary: DiffImpactSummary {
                    changed_files: 0,
                    changed_symbols: 0,
                    affected_processes: 0,
                    risk_level: "none".to_string(),
                },
                changed_files: bounded_items(Vec::<DiffImpactFile>::new(), request.budget.clone())?,
                affected_processes: bounded_items(Vec::<DiffImpactProcess>::new(), request.budget)?,
                warnings,
            });
        }

        let mut indexed_changed_files = Vec::new();
        let mut changed_symbol_names = BTreeSet::new();
        let mut skipped_paths = Vec::new();
        for changed_file in changed_files {
            let Some(indexed_file) = self
                .artifact
                .files
                .iter()
                .find(|file| file.path == changed_file.path)
            else {
                skipped_paths.push(changed_file.path);
                continue;
            };
            let changed_symbols =
                owned_symbols_for_changed_ranges(indexed_file, &changed_file.ranges);
            if changed_symbols.is_empty() {
                continue;
            }
            changed_symbol_names.extend(changed_symbols.iter().map(|symbol| symbol.symbol.clone()));
            indexed_changed_files.push(DiffImpactFile {
                path: indexed_file.path.clone(),
                changed_symbols: bounded_items(changed_symbols, request.budget.clone())?,
            });
            deadline.check()?;
        }

        if !skipped_paths.is_empty() {
            let preview = skipped_paths.iter().take(3).cloned().collect::<Vec<_>>();
            let suffix = if skipped_paths.len() > preview.len() {
                format!(", and {} more", skipped_paths.len() - preview.len())
            } else {
                String::new()
            };
            warnings.push(format!(
                "Skipped {} changed path(s) that are not present in the active Rust source index: {}{}",
                skipped_paths.len(),
                preview.join(", "),
                suffix
            ));
        }

        indexed_changed_files.sort_by(|left, right| left.path.cmp(&right.path));
        let affected_processes = self
            .artifact
            .processes
            .iter()
            .filter_map(|process| {
                let changed_steps = process
                    .steps
                    .iter()
                    .filter(|step| changed_symbol_names.contains(step.as_str()))
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                if changed_steps.is_empty() {
                    None
                } else {
                    Some(DiffImpactProcess {
                        name: process.name.clone(),
                        entry_symbol: process.entry_symbol.clone(),
                        path: process.path.clone(),
                        changed_steps,
                    })
                }
            })
            .collect::<Vec<_>>();
        deadline.check()?;

        let risk_level = match affected_processes.len() {
            0 => "low",
            1..=5 => "medium",
            6..=15 => "high",
            _ => "critical",
        }
        .to_string();
        let changed_symbols_count = indexed_changed_files
            .iter()
            .map(|file| file.changed_symbols.items.len())
            .sum::<usize>();

        Ok(DiffImpactResult {
            summary: DiffImpactSummary {
                changed_files: indexed_changed_files.len(),
                changed_symbols: changed_symbols_count,
                affected_processes: affected_processes.len(),
                risk_level,
            },
            changed_files: bounded_items(indexed_changed_files, request.budget.clone())?,
            affected_processes: bounded_items(affected_processes, request.budget)?,
            warnings,
        })
    }

    fn discover_routes(&self, request: RouteMapRequest) -> QueryResult<RouteMapResult> {
        let deadline = QueryDeadline::start(request.budget.timeout_ms);
        let route_filter = request.route.as_deref().map(normalize_route_path);
        let path_filter = request.path.as_deref();
        let mut routes = self
            .artifact
            .routes
            .iter()
            .filter(|route| {
                route_filter
                    .as_ref()
                    .is_none_or(|filter| route.route_path == *filter)
                    && path_filter.is_none_or(|filter| route.path == filter)
            })
            .map(|route| RouteMapItem {
                method: route.method.clone(),
                route_path: route.route_path.clone(),
                handler_symbol: route.handler_symbol.clone(),
                path: route.path.clone(),
                line: route.line,
            })
            .collect::<Vec<_>>();
        routes.sort_by(|left, right| {
            left.route_path
                .cmp(&right.route_path)
                .then(left.method.cmp(&right.method))
                .then(left.path.cmp(&right.path))
                .then(left.line.cmp(&right.line))
        });
        deadline.check()?;
        Ok(RouteMapResult {
            routes: bounded_items(routes, request.budget)?,
        })
    }

    fn graph_scc_inputs(&self, request: GraphSccInputRequest) -> QueryResult<GraphSccInputResult> {
        let edges = self
            .artifact
            .relationships
            .iter()
            .filter(|relationship| relationship.kind == "CALLS")
            .map(|relationship| GraphSccInputItem {
                relationship_id: relationship.relationship_id.clone(),
                from_node_id: relationship.from_node_id.clone(),
                from_symbol: relationship.from_symbol.clone(),
                from_path: relationship.from_path.clone(),
                to_node_id: relationship.to_node_id.clone(),
                to_symbol: relationship.to_symbol.clone(),
                to_path: relationship.to_path.clone(),
            })
            .collect::<Vec<_>>();
        Ok(GraphSccInputResult {
            edges: bounded_items(edges, request.budget)?,
        })
    }

    fn test_links(&self, request: TestLinkRequest) -> QueryResult<TestLinkResult> {
        let links = self
            .artifact
            .relationships
            .iter()
            .filter(|relationship| relationship.kind == "TESTS")
            .map(|relationship| TestLinkItem {
                relationship_id: relationship.relationship_id.clone(),
                test_symbol: relationship.from_symbol.clone(),
                test_path: relationship.from_path.clone(),
                target_symbol: relationship.to_symbol.clone(),
                target_path: relationship.to_path.clone(),
            })
            .collect::<Vec<_>>();
        Ok(TestLinkResult {
            links: bounded_items(links, request.budget)?,
        })
    }

    fn source_references(&self) -> SourceReferenceSet {
        let mut paths = BTreeSet::new();
        let mut route_paths = BTreeSet::new();
        let mut symbols = BTreeSet::new();
        let mut symbols_by_path = std::collections::BTreeMap::new();

        for file in &self.artifact.files {
            paths.insert(file.path.clone());
            let file_symbols = symbols_by_path
                .entry(file.path.clone())
                .or_insert_with(BTreeSet::new);
            for symbol in &file.symbols {
                symbols.insert(symbol.name.clone());
                file_symbols.insert(symbol.name.clone());
            }
        }
        for route in &self.artifact.routes {
            route_paths.insert(route.route_path.clone());
        }

        SourceReferenceSet {
            paths,
            route_paths,
            symbols,
            symbols_by_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChangedFileRange {
    path: String,
    ranges: Vec<ChangedLineRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChangedLineRange {
    start: usize,
    end: usize,
}

impl SourceIndexStore {
    fn find_symbol_locations(
        &self,
        symbol: &str,
        path: Option<&str>,
        deadline: QueryDeadline,
    ) -> QueryResult<Vec<(&str, usize)>> {
        let mut matches = Vec::new();
        let mut scanned = 0;
        check_query_scan_deadline(deadline)?;
        for file in &self.artifact.files {
            query_scan_step();
            scanned += 1;
            if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                check_query_scan_deadline(deadline)?;
            }
            if path.is_some_and(|expected| file.path != expected) {
                continue;
            }
            for indexed_symbol in &file.symbols {
                query_scan_step();
                scanned += 1;
                if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                    check_query_scan_deadline(deadline)?;
                }
                if indexed_symbol.name == symbol {
                    matches.push((file.path.as_str(), indexed_symbol.line));
                }
            }
        }
        check_query_scan_deadline(deadline)?;
        Ok(matches)
    }

    fn render_symbol_impact_at(
        &self,
        symbol: &str,
        symbol_path: &str,
        symbol_line: usize,
        budget: QueryBudget,
        deadline: QueryDeadline,
    ) -> QueryResult<SymbolImpactResult> {
        let mut upstream = Vec::new();
        let mut downstream = Vec::new();
        let candidate_limit = budget.max_items.saturating_add(1);
        let mut scanned = 0;
        check_query_scan_deadline(deadline)?;
        for relationship in &self.artifact.relationships {
            query_scan_step();
            scanned += 1;
            if scanned % QUERY_DEADLINE_CHECK_STRIDE == 0 {
                check_query_scan_deadline(deadline)?;
            }
            if relationship.to_symbol == symbol
                && relationship.to_path == symbol_path
                && relationship.to_line == symbol_line
                && upstream.len() < candidate_limit
            {
                upstream.push(ImpactItem {
                    symbol: relationship.from_symbol.clone(),
                    path: relationship.from_path.clone(),
                    relationship: relationship.kind.clone(),
                    site_line: relationship.line,
                });
            }
            if relationship.from_symbol == symbol
                && relationship.from_path == symbol_path
                && relationship.from_line == symbol_line
                && downstream.len() < candidate_limit
            {
                downstream.push(ImpactItem {
                    symbol: relationship.to_symbol.clone(),
                    path: relationship.to_path.clone(),
                    relationship: relationship.kind.clone(),
                    site_line: relationship.line,
                });
            }
        }
        check_query_scan_deadline(deadline)?;
        Ok(SymbolImpactResult {
            upstream: bounded_items(upstream, budget.clone())?,
            downstream: bounded_items(downstream, budget)?,
        })
    }

    fn resolve_line_evidence(
        &self,
        target: &str,
        include_snippet: bool,
        context_lines: usize,
    ) -> QueryResult<Option<EvidenceItem>> {
        let Some((path, line)) = parse_line_target(target) else {
            return Ok(None);
        };
        let Some(file) = self.artifact.files.iter().find(|file| file.path == path) else {
            return Ok(None);
        };
        let lines = match read_indexed_file_lines(&self.repo_root, &file.path) {
            Ok(lines) => lines,
            Err(_) => return Ok(None),
        };
        if line == 0 || line > lines.len() {
            return Ok(None);
        }
        let (start_line, end_line, snippet) =
            render_snippet(&lines, line, line, include_snippet, context_lines);
        Ok(Some(EvidenceItem {
            target: target.to_string(),
            kind: "line".to_string(),
            path: file.path.clone(),
            start_line: Some(start_line),
            end_line: Some(end_line),
            snippet,
            symbol: None,
            symbol_kind: None,
        }))
    }

    fn resolve_file_evidence(
        &self,
        target: &str,
        include_snippet: bool,
        context_lines: usize,
    ) -> QueryResult<Option<EvidenceItem>> {
        let Some(file) = self.artifact.files.iter().find(|file| file.path == target) else {
            return Ok(None);
        };
        let lines = match read_indexed_file_lines(&self.repo_root, &file.path) {
            Ok(lines) => lines,
            Err(_) => return Ok(None),
        };
        let window_size = file_window_size(context_lines);
        let snippet = include_snippet.then(|| {
            redact_secrets(
                lines
                    .iter()
                    .take(window_size)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        });
        Ok(Some(EvidenceItem {
            target: target.to_string(),
            kind: "file".to_string(),
            path: file.path.clone(),
            start_line: Some(1),
            end_line: Some(if include_snippet {
                lines.len().min(window_size)
            } else {
                lines.len()
            }),
            snippet,
            symbol: None,
            symbol_kind: None,
        }))
    }

    fn resolve_symbol_evidence(
        &self,
        target: &str,
        include_snippet: bool,
        context_lines: usize,
        parse_cache: &mut HashMap<String, Option<ParsedFile>>,
    ) -> QueryResult<Option<EvidenceItem>> {
        let matches = self
            .artifact
            .files
            .iter()
            .flat_map(|file| {
                file.symbols
                    .iter()
                    .filter(|symbol| symbol.name == target)
                    .map(move |symbol| (file, symbol))
            })
            .collect::<Vec<_>>();
        let [(file, symbol)] = matches.as_slice() else {
            return Ok(None);
        };
        let lines = match read_indexed_file_lines(&self.repo_root, &file.path) {
            Ok(lines) => lines,
            Err(_) => return Ok(None),
        };
        let focus_end = if include_snippet && file.path.ends_with(".rs") {
            parse_cache
                .entry(file.path.clone())
                .or_insert_with(|| parse_rust_file(&lines.join("\n")))
                .as_ref()
                .and_then(|parsed| {
                    parsed.symbols.iter().find(|parsed_symbol| {
                        parsed_symbol.name == symbol.name
                            && parsed_symbol.line == symbol.line
                            && parsed_symbol.kind == symbol.kind
                    })
                })
                .map(|parsed_symbol| {
                    parsed_symbol
                        .end_line
                        .min(symbol.line + MAX_SYMBOL_SNIPPET_LINES - 1)
                })
                .unwrap_or(symbol.line)
        } else {
            symbol.line
        };
        let (start_line, end_line, snippet) = render_snippet(
            &lines,
            symbol.line,
            focus_end,
            include_snippet,
            context_lines,
        );
        Ok(Some(EvidenceItem {
            target: target.to_string(),
            kind: "symbol".to_string(),
            path: file.path.clone(),
            start_line: Some(start_line),
            end_line: Some(end_line),
            snippet,
            symbol: Some(symbol.name.clone()),
            symbol_kind: Some(symbol.kind.clone()),
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedSymbolSearchItem {
    score: SearchScore,
    item: SymbolSearchItem,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedRepoMapFileItem {
    score: SearchScore,
    item: RepoMapFileItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SearchScore {
    Exact,
    Prefix,
    Token,
    Substring,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchQuery {
    raw: String,
    tokens: Vec<String>,
}

impl SearchQuery {
    fn new(query: &str) -> Self {
        let raw = query.to_lowercase();
        let tokens = split_search_tokens(query);
        Self { raw, tokens }
    }

    fn score_symbol(&self, symbol: &str, path: &str) -> Option<SearchScore> {
        let symbol_lower = symbol.to_lowercase();
        if symbol_lower == self.raw {
            Some(SearchScore::Exact)
        } else if symbol_lower.starts_with(&self.raw) {
            Some(SearchScore::Prefix)
        } else if self
            .tokens
            .iter()
            .all(|token| split_search_tokens(symbol).contains(token))
        {
            Some(SearchScore::Token)
        } else if symbol_lower.contains(&self.raw) {
            Some(SearchScore::Substring)
        } else {
            self.score_path(path)
        }
    }

    fn score_path(&self, path: &str) -> Option<SearchScore> {
        path.to_lowercase()
            .contains(&self.raw)
            .then_some(SearchScore::Path)
    }
}

fn bounded_items<T>(items: Vec<T>, budget: QueryBudget) -> QueryResult<BoundedItems<T>>
where
    T: Serialize,
{
    bounded_serialized_items(items, budget)
}

fn git_diff_output(repo_root: &Path, scope: &str, base_ref: Option<&str>) -> QueryResult<String> {
    let args = match scope {
        "staged" => vec!["diff", "--staged", "-U0"],
        "all" => vec!["diff", "HEAD", "-U0"],
        "compare" => {
            let Some(base_ref) = base_ref else {
                return Err(QueryError::ExternalCommand);
            };
            vec!["diff", base_ref, "-U0"]
        }
        "unstaged" => vec!["diff", "-U0"],
        _ => return Err(QueryError::ExternalCommand),
    };
    let output = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(&args)
        .current_dir(repo_root)
        .output()
        .map_err(|_| QueryError::ExternalCommand)?;
    if !output.status.success() {
        return Err(QueryError::ExternalCommand);
    }
    String::from_utf8(output.stdout).map_err(|_| QueryError::CorruptArtifact)
}

fn parse_changed_file_ranges(
    diff_output: &str,
    include_paths: &[String],
    warnings: &mut Vec<String>,
) -> Vec<ChangedFileRange> {
    let mut changed_files = Vec::new();
    let mut omitted_paths = Vec::new();
    let mut current_index = None;

    for raw_line in diff_output.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("diff --git ") {
            current_index = None;
            let Some(path) = parse_diff_header_path(rest) else {
                continue;
            };
            if !include_paths.is_empty()
                && !include_paths
                    .iter()
                    .any(|prefix| file_matches_include_path(&path, prefix))
            {
                omitted_paths.push(path);
                continue;
            }
            changed_files.push(ChangedFileRange {
                path,
                ranges: Vec::new(),
            });
            current_index = Some(changed_files.len() - 1);
            continue;
        }

        let Some(index) = current_index else {
            continue;
        };
        if let Some(range) = parse_added_hunk_range(line) {
            changed_files[index].ranges.push(range);
        }
    }

    if !omitted_paths.is_empty() {
        let preview = omitted_paths.iter().take(3).cloned().collect::<Vec<_>>();
        let suffix = if omitted_paths.len() > preview.len() {
            format!(", and {} more", omitted_paths.len() - preview.len())
        } else {
            String::new()
        };
        warnings.push(format!(
            "Omitted {} changed path(s) outside include_paths: {}{}",
            omitted_paths.len(),
            preview.join(", "),
            suffix
        ));
    }

    changed_files.retain(|file| !file.ranges.is_empty());
    changed_files
}

fn parse_diff_header_path(rest: &str) -> Option<String> {
    let (left, right) = parse_diff_header_paths(rest)?;
    normalize_diff_path(&right)
        .or_else(|| normalize_diff_path(&left))
        .filter(|path| !path.is_empty())
}

fn parse_diff_header_paths(line: &str) -> Option<(String, String)> {
    let mut chars = line.chars().peekable();
    let left = read_diff_header_token(&mut chars)?;
    let right = read_diff_header_token(&mut chars)?;
    Some((left, right))
}

fn read_diff_header_token(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<String> {
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
    let quote = match chars.peek().copied() {
        Some('"') | Some('\'') => chars.next(),
        _ => None,
    };
    let mut token = String::new();
    while let Some(ch) = chars.next() {
        if let Some(quote) = quote {
            if ch == quote {
                break;
            }
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    token.push(next);
                }
                continue;
            }
        } else if ch.is_whitespace() {
            break;
        }
        token.push(ch);
    }
    if token.is_empty() && quote.is_none() {
        None
    } else {
        Some(token)
    }
}

fn normalize_diff_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "/dev/null" {
        return None;
    }
    let trimmed = trimmed
        .strip_prefix("a/")
        .or_else(|| trimmed.strip_prefix("b/"))
        .unwrap_or(trimmed);
    Some(trimmed.strip_prefix("./").unwrap_or(trimmed).to_string())
}

fn parse_added_hunk_range(line: &str) -> Option<ChangedLineRange> {
    let plus = line.find(" +")?;
    let remainder = &line[plus + 2..];
    let end = remainder.find(" @@").or_else(|| remainder.find("@@"))?;
    let segment = &remainder[..end];
    let (start, count) = match segment.split_once(',') {
        Some((start, count)) => (
            start.trim().parse::<usize>().ok()?,
            count.trim().parse::<usize>().ok()?,
        ),
        None => (segment.trim().parse::<usize>().ok()?, 1),
    };
    (count > 0).then_some(ChangedLineRange {
        start,
        end: start + count.saturating_sub(1),
    })
}

fn file_matches_include_path(path: &str, include_path: &str) -> bool {
    path == include_path || path.starts_with(&format!("{include_path}/"))
}

fn owned_symbols_for_changed_ranges(
    indexed_file: &IndexedFile,
    ranges: &[ChangedLineRange],
) -> Vec<DiffImpactSymbol> {
    indexed_file
        .symbols
        .iter()
        .enumerate()
        .filter_map(|(index, symbol)| {
            let end_line = indexed_file
                .symbols
                .get(index + 1)
                .map(|next| next.line.saturating_sub(1))
                .unwrap_or(usize::MAX);
            ranges
                .iter()
                .any(|range| symbol.line <= range.end && end_line >= range.start)
                .then(|| DiffImpactSymbol {
                    symbol: symbol.name.clone(),
                    kind: symbol.kind.clone(),
                    path: indexed_file.path.clone(),
                    line: symbol.line,
                })
        })
        .collect()
}

fn parse_line_target(target: &str) -> Option<(&str, usize)> {
    let colon = target.rfind(':')?;
    let path = target[..colon].trim();
    let line = target[colon + 1..].trim().parse::<usize>().ok()?;
    (!path.is_empty() && line > 0).then_some((path, line))
}

fn file_window_size(context_lines: usize) -> usize {
    (context_lines.saturating_mul(2)).max(20)
}

const MAX_SYMBOL_SNIPPET_LINES: usize = 80;

fn render_snippet(
    lines: &[String],
    focus_start: usize,
    focus_end: usize,
    include_snippet: bool,
    context_lines: usize,
) -> (usize, usize, Option<String>) {
    if !include_snippet {
        return (focus_start, focus_end, None);
    }
    let start_line = focus_start.saturating_sub(context_lines).max(1);
    let end_line = (focus_end + context_lines).min(lines.len());
    let snippet = redact_secrets(lines[start_line - 1..end_line].join("\n"));
    (start_line, end_line, Some(snippet))
}

fn read_indexed_file_lines(repo_root: &Path, indexed_path: &str) -> Result<Vec<String>, io::Error> {
    let path = repo_root.join(indexed_path);
    let mut options = File::options();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content.lines().map(str::to_string).collect())
}

struct WorkingFile {
    indexed: IndexedFile,
    content: String,
    parsed: Option<ParsedFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IndexedFunctionRef {
    node_id: String,
    name: String,
    path: String,
    line: usize,
    owner: Option<String>,
}

fn collect_relationships(
    files: &[WorkingFile],
) -> (Vec<IndexedRelationship>, Vec<IndexedUnresolvedCall>) {
    let mut relationships = Vec::new();
    let mut unresolved_calls = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_unresolved = BTreeSet::new();
    let function_defs = files
        .iter()
        .flat_map(|file| {
            file.indexed.symbols.iter().filter_map(|symbol| {
                (symbol.kind == "fn").then_some(IndexedFunctionRef {
                    node_id: symbol.node_id.clone(),
                    name: symbol.name.clone(),
                    path: file.indexed.path.clone(),
                    line: symbol.line,
                    owner: file
                        .parsed
                        .as_ref()
                        .and_then(|parsed| {
                            parsed.symbols.iter().find(|parsed_symbol| {
                                parsed_symbol.kind == symbol.kind
                                    && parsed_symbol.name == symbol.name
                                    && parsed_symbol.line == symbol.line
                            })
                        })
                        .and_then(|parsed_symbol| parsed_symbol.owner.clone()),
                })
            })
        })
        .collect::<Vec<_>>();
    for file in files {
        if let Some(parsed) = file.parsed.as_ref() {
            collect_parsed_relationships(
                file,
                parsed,
                &function_defs,
                &mut relationships,
                &mut unresolved_calls,
                &mut seen,
                &mut seen_unresolved,
            );
            continue;
        }
        let mut current_function: Option<FunctionScope> = None;
        let mut next_function_is_test = false;
        for (index, line) in file.content.lines().enumerate() {
            let line = strip_comments_and_strings(line);
            if is_test_attribute_line(&line) {
                next_function_is_test = true;
                continue;
            }
            if let Some((kind, name)) = parse_rust_symbol_line(&line)
                && kind == "fn"
            {
                current_function = Some(FunctionScope {
                    name,
                    path: file.indexed.path.clone(),
                    line: index + 1,
                    brace_depth: 0,
                    waiting_for_body: !line.contains('{'),
                    is_test: next_function_is_test,
                });
                next_function_is_test = false;
            }
            let Some(scope) = current_function.as_mut() else {
                continue;
            };
            if scope.waiting_for_body {
                if !line.contains('{') {
                    continue;
                }
                scope.waiting_for_body = false;
            }
            for target in call_identifiers(&line) {
                if target.is_scoped {
                    push_unresolved_call(
                        &mut unresolved_calls,
                        &mut seen_unresolved,
                        scope,
                        &target.name,
                        &file.indexed.path,
                        index + 1,
                        "scoped_call",
                    );
                    continue;
                }

                if target.name == scope.name && index + 1 == scope.line {
                    continue;
                }
                let matches = function_defs
                    .iter()
                    .filter(|candidate| candidate.name == target.name)
                    .collect::<Vec<_>>();
                match matches.as_slice() {
                    [callee] => {
                        let key = (
                            scope.name.clone(),
                            scope.path.clone(),
                            scope.line,
                            callee.name.clone(),
                            callee.path.clone(),
                            callee.line,
                            index + 1,
                        );
                        if seen.insert(key) {
                            relationships.push(IndexedRelationship {
                                relationship_id: relationship_node_id(
                                    &scope.path,
                                    scope.line,
                                    &scope.name,
                                    &callee.path,
                                    callee.line,
                                    &callee.name,
                                    &file.indexed.path,
                                    index + 1,
                                ),
                                from_node_id: symbol_node_id(
                                    &scope.path,
                                    "fn",
                                    &scope.name,
                                    scope.line,
                                ),
                                from_symbol: scope.name.clone(),
                                from_path: scope.path.clone(),
                                from_line: scope.line,
                                to_node_id: callee.node_id.clone(),
                                to_symbol: callee.name.clone(),
                                to_path: callee.path.clone(),
                                to_line: callee.line,
                                kind: if scope.is_test { "TESTS" } else { "CALLS" }.to_string(),
                                path: file.indexed.path.clone(),
                                line: index + 1,
                            });
                        }
                    }
                    [] => push_unresolved_call(
                        &mut unresolved_calls,
                        &mut seen_unresolved,
                        scope,
                        &target.name,
                        &file.indexed.path,
                        index + 1,
                        "no_indexed_definition",
                    ),
                    _ => push_unresolved_call(
                        &mut unresolved_calls,
                        &mut seen_unresolved,
                        scope,
                        &target.name,
                        &file.indexed.path,
                        index + 1,
                        "ambiguous_target",
                    ),
                }
            }
            scope.brace_depth += brace_delta(&line);
            if scope.brace_depth <= 0 {
                current_function = None;
            }
        }
    }
    (relationships, unresolved_calls)
}

fn collect_parsed_relationships(
    file: &WorkingFile,
    parsed: &ParsedFile,
    function_defs: &[IndexedFunctionRef],
    relationships: &mut Vec<IndexedRelationship>,
    unresolved_calls: &mut Vec<IndexedUnresolvedCall>,
    seen: &mut BTreeSet<(String, String, usize, String, String, usize, usize)>,
    seen_unresolved: &mut BTreeSet<(String, String, usize, String)>,
) {
    for call in &parsed.calls {
        let Some(enclosing_fn) = call.enclosing_fn.as_ref() else {
            continue;
        };
        let scope = FunctionScope {
            name: enclosing_fn.clone(),
            path: file.indexed.path.clone(),
            line: call.enclosing_fn_line,
            brace_depth: 1,
            waiting_for_body: false,
            is_test: function_is_test(&file.content, call.enclosing_fn_line),
        };
        let matches = function_defs
            .iter()
            .filter(|candidate| {
                candidate.name == call.name
                    || parsed.imports.iter().any(|import| {
                        import.local_name == call.name
                            && import.source_path.last() == Some(&candidate.name)
                            && imported_path_matches_candidate(
                                import,
                                &file.indexed.path,
                                &candidate.path,
                            )
                    })
            })
            .filter(|candidate| {
                if let Some(qualifier) = call.qualifier.as_deref() {
                    if !qualifier.contains("::") {
                        return candidate.owner.as_deref() == Some(qualifier);
                    }
                    // A multi-segment path is safe only when it names the current
                    // file or a uniquely mapped imported module.
                    return candidate.path == file.indexed.path;
                }

                let matching_imports = parsed
                    .imports
                    .iter()
                    .filter(|import| import.local_name == call.name)
                    .collect::<Vec<_>>();
                if matching_imports.is_empty() {
                    return true;
                }
                let import_resolves = matching_imports.iter().any(|import| {
                    let Some(imported_name) = import.source_path.last() else {
                        return false;
                    };
                    function_defs.iter().any(|definition| {
                        definition.name == *imported_name
                            && imported_path_matches_candidate(
                                import,
                                &file.indexed.path,
                                &definition.path,
                            )
                    })
                });
                !import_resolves
                    || matching_imports.iter().any(|import| {
                        import.source_path.last().is_some_and(|imported_name| {
                            candidate.name == *imported_name
                                && imported_path_matches_candidate(
                                    import,
                                    &file.indexed.path,
                                    &candidate.path,
                                )
                        })
                    })
            })
            .collect::<Vec<_>>();
        let callee = match (call.form, matches.as_slice()) {
            (ParsedCallForm::Bare, [callee]) => Some(*callee),
            (ParsedCallForm::Method | ParsedCallForm::Path, [callee]) => Some(*callee),
            _ => None,
        };
        if let Some(callee) = callee {
            add_parsed_relationship(&scope, callee, file, call.line, relationships, seen);
        } else {
            let reason = if matches.is_empty() {
                "no_indexed_definition"
            } else {
                "ambiguous_target"
            };
            push_unresolved_call(
                unresolved_calls,
                seen_unresolved,
                &scope,
                &call.name,
                &file.indexed.path,
                call.line,
                reason,
            );
        }
    }
}

fn imported_path_matches_candidate(
    import: &ParsedImport,
    importing_path: &str,
    candidate_path: &str,
) -> bool {
    let Some((root, source_segments)) = import.source_path.split_first() else {
        return false;
    };
    let Some((_, module_segments)) = source_segments.split_last() else {
        return false;
    };

    let base_segments = match root.as_str() {
        "crate" => vec!["src".to_string()],
        "self" => importing_path
            .split('/')
            .collect::<Vec<_>>()
            .split_last()
            .map(|(_, directory)| {
                directory
                    .iter()
                    .map(|segment| (*segment).to_string())
                    .collect()
            })
            .unwrap_or_default(),
        "super" => {
            let mut segments: Vec<String> = importing_path
                .split('/')
                .collect::<Vec<_>>()
                .split_last()
                .map(|(_, directory)| {
                    directory
                        .iter()
                        .map(|segment| (*segment).to_string())
                        .collect()
                })
                .unwrap_or_default();
            segments.pop();
            segments
        }
        _ => return false,
    };

    let mut module_path = base_segments;
    module_path.extend(module_segments.iter().cloned());
    let module_path = module_path.join("/");
    candidate_path == format!("{module_path}.rs")
        || candidate_path == format!("{module_path}/mod.rs")
}

fn add_parsed_relationship(
    scope: &FunctionScope,
    callee: &IndexedFunctionRef,
    file: &WorkingFile,
    line: usize,
    relationships: &mut Vec<IndexedRelationship>,
    seen: &mut BTreeSet<(String, String, usize, String, String, usize, usize)>,
) {
    let key = (
        scope.name.clone(),
        scope.path.clone(),
        scope.line,
        callee.name.clone(),
        callee.path.clone(),
        callee.line,
        line,
    );
    if seen.insert(key) {
        relationships.push(IndexedRelationship {
            relationship_id: relationship_node_id(
                &scope.path,
                scope.line,
                &scope.name,
                &callee.path,
                callee.line,
                &callee.name,
                &file.indexed.path,
                line,
            ),
            from_node_id: symbol_node_id(&scope.path, "fn", &scope.name, scope.line),
            from_symbol: scope.name.clone(),
            from_path: scope.path.clone(),
            from_line: scope.line,
            to_node_id: callee.node_id.clone(),
            to_symbol: callee.name.clone(),
            to_path: callee.path.clone(),
            to_line: callee.line,
            kind: if scope.is_test { "TESTS" } else { "CALLS" }.to_string(),
            path: file.indexed.path.clone(),
            line,
        });
    }
}

fn function_is_test(content: &str, function_line: usize) -> bool {
    content
        .lines()
        .enumerate()
        .take(function_line.saturating_sub(1))
        .filter(|(_, line)| !line.trim().is_empty())
        .last()
        .is_some_and(|(_, line)| is_test_attribute_line(line))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionScope {
    name: String,
    path: String,
    line: usize,
    brace_depth: i32,
    waiting_for_body: bool,
    is_test: bool,
}

fn is_test_attribute_line(line: &str) -> bool {
    let line = line.trim();
    line == "#[test]" || line.starts_with("#[tokio::test") || line.starts_with("#[rstest")
}

fn file_node_id(path: &str) -> String {
    format!("file:{path}")
}

fn symbol_node_id(path: &str, kind: &str, name: &str, line: usize) -> String {
    format!("symbol:{path}:{line}:{kind}:{name}")
}

fn route_node_id(method: &str, route_path: &str, path: &str, line: usize, handler: &str) -> String {
    format!("route:{method}:{route_path}:{path}:{line}:{handler}")
}

fn process_node_id(path: &str, entry_symbol: &str, steps: &[&str]) -> String {
    format!("process:{path}:{entry_symbol}:{}", steps.join(">"))
}

#[allow(clippy::too_many_arguments)]
fn relationship_node_id(
    from_path: &str,
    from_line: usize,
    from_symbol: &str,
    to_path: &str,
    to_line: usize,
    to_symbol: &str,
    path: &str,
    line: usize,
) -> String {
    format!(
        "call:{from_path}:{from_line}:{from_symbol}:{to_path}:{to_line}:{to_symbol}:{path}:{line}"
    )
}

fn unresolved_call_id(
    scope: &FunctionScope,
    target_symbol: &str,
    path: &str,
    line: usize,
) -> String {
    format!(
        "unresolved-call:{}:{}:{}:{}:{}:{}",
        scope.path, scope.line, scope.name, target_symbol, path, line
    )
}

fn push_unresolved_call(
    unresolved_calls: &mut Vec<IndexedUnresolvedCall>,
    seen_unresolved: &mut BTreeSet<(String, String, usize, String)>,
    scope: &FunctionScope,
    target_symbol: &str,
    path: &str,
    line: usize,
    reason: &str,
) {
    let key = (
        scope.path.clone(),
        scope.name.clone(),
        line,
        target_symbol.to_string(),
    );
    if seen_unresolved.insert(key) {
        unresolved_calls.push(IndexedUnresolvedCall {
            unresolved_call_id: unresolved_call_id(scope, target_symbol, path, line),
            from_node_id: symbol_node_id(&scope.path, "fn", &scope.name, scope.line),
            from_symbol: scope.name.clone(),
            from_path: scope.path.clone(),
            from_line: scope.line,
            target_symbol: target_symbol.to_string(),
            path: path.to_string(),
            line,
            reason: reason.to_string(),
        });
    }
}

#[derive(Debug, Clone)]
struct CallIdentifier {
    name: String,
    is_scoped: bool,
}

fn call_identifiers(line: &str) -> Vec<CallIdentifier> {
    let mut identifiers = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '_' || ch.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch == '_' || ch.is_ascii_alphanumeric() {
                    index += 1;
                } else {
                    break;
                }
            }
            let identifier = &line[start..index];
            let rest = line[index..].trim_start();
            if rest.starts_with('(') && !is_rust_control_keyword(identifier) {
                identifiers.push(CallIdentifier {
                    name: identifier.to_string(),
                    is_scoped: is_scoped_call(line, start),
                });
            }
            continue;
        }
        index += 1;
    }
    identifiers
}

fn is_scoped_call(line: &str, identifier_start: usize) -> bool {
    if identifier_start >= line.len() {
        return false;
    }
    let prefix = &line[..identifier_start];
    for &character in prefix.as_bytes().iter().rev() {
        if character.is_ascii_whitespace() {
            continue;
        }
        return matches!(character, b'.' | b':');
    }
    false
}

fn is_rust_control_keyword(identifier: &str) -> bool {
    matches!(
        identifier,
        "if" | "while" | "for" | "loop" | "match" | "return" | "fn"
    )
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

fn mix_u64(hash: &mut u64, value: u64) {
    hash_bytes(hash, &value.to_le_bytes());
}

fn collect_processes(relationships: &[IndexedRelationship]) -> Vec<IndexedProcess> {
    relationships
        .iter()
        .filter(|relationship| relationship.kind == "CALLS")
        .map(|relationship| IndexedProcess {
            node_id: process_node_id(
                &relationship.path,
                &relationship.from_symbol,
                &[
                    relationship.from_symbol.as_str(),
                    relationship.to_symbol.as_str(),
                ],
            ),
            name: format!("{} -> {}", relationship.from_symbol, relationship.to_symbol),
            entry_symbol: relationship.from_symbol.clone(),
            path: relationship.path.clone(),
            steps: vec![
                relationship.from_symbol.clone(),
                relationship.to_symbol.clone(),
            ],
        })
        .collect()
}

fn collect_routes(files: &[WorkingFile]) -> Vec<IndexedRoute> {
    let mut routes = Vec::new();
    let mut seen = BTreeSet::new();
    for file in files {
        for (index, line) in file.content.lines().enumerate() {
            let Some(route) = parse_axum_route_line(line, &file.indexed.path, index + 1) else {
                continue;
            };
            let key = (
                route.method.clone(),
                route.route_path.clone(),
                route.handler_symbol.clone(),
                route.path.clone(),
                route.line,
            );
            if seen.insert(key) {
                routes.push(route);
            }
        }
    }
    routes
}

fn parse_axum_route_line(line: &str, path: &str, line_number: usize) -> Option<IndexedRoute> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") {
        return None;
    }
    let marker_start = trimmed
        .find(".route(")
        .map(|index| index + 1)
        .or_else(|| trimmed.find("route("))?;
    let call = &trimmed[marker_start..];
    let raw_path = read_quoted_after_marker(call, "route(")?;
    let after_path = &call[call.find(raw_path)? + raw_path.len()..];
    let comma = after_path.find(',')?;
    let handler_call = after_path[comma + 1..].trim_start();
    let method_end = handler_call.find('(')?;
    let method = route_method(handler_call[..method_end].trim())?;
    let handler_start = method_end + 1;
    let handler_end = handler_call[handler_start..]
        .find([')', ','])
        .map(|index| handler_start + index)?;
    let handler_symbol = handler_call[handler_start..handler_end].trim();
    let route_path = normalize_route_path(raw_path);
    is_rust_identifier(handler_symbol).then(|| IndexedRoute {
        node_id: route_node_id(method, &route_path, path, line_number, handler_symbol),
        method: method.to_string(),
        route_path,
        handler_symbol: handler_symbol.to_string(),
        path: path.to_string(),
        line: line_number,
    })
}

fn read_quoted_after_marker<'a>(content: &'a str, marker: &str) -> Option<&'a str> {
    let start = content.find(marker)? + marker.len();
    let rest = content[start..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &rest[quote.len_utf8()..];
    let end = body.find(quote)?;
    Some(&body[..end])
}

fn route_method(method: &str) -> Option<&'static str> {
    match method {
        "get" => Some("GET"),
        "post" => Some("POST"),
        "put" => Some("PUT"),
        "delete" => Some("DELETE"),
        "patch" => Some("PATCH"),
        "any" => Some("ANY"),
        _ => None,
    }
}

fn normalize_route_path(path: &str) -> String {
    let mut normalized = path
        .split('?')
        .next()
        .unwrap_or(path)
        .trim()
        .trim_end_matches('/')
        .to_ascii_lowercase();
    if normalized.is_empty() {
        normalized = "/".to_string();
    }
    normalized
        .split('/')
        .map(|segment| {
            if segment.starts_with(':') || (segment.starts_with('{') && segment.ends_with('}')) {
                "{param}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn is_rust_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn publish_index_artifact(
    request: &RustIndexRequest,
    artifact: &SourceIndexArtifact,
) -> Result<Vec<String>, RustIndexError> {
    let root = request.artifact_root.clone();
    let artifact_path = root
        .generation_file_path(&request.generation_id, SOURCE_INDEX_FILE)
        .map_err(RustIndexError::PublishManifest)?;
    let contents = serde_json::to_string_pretty(artifact).map_err(|_| RustIndexError::Serialize)?;
    let contents = format!("{contents}\n");
    let content_hash = source_index_content_hash(&contents);
    write_atomically(&artifact_path, &contents).map_err(|_| RustIndexError::WriteArtifact)?;
    // The docs sidecar is paired to this generation's identity, so it is
    // produced after the source index exists and its hash is known, but before
    // the manifest publishes the generation as active.
    let limits = build_and_publish_docs_index(
        DocsIndexRequest {
            repo_root: &request.repo_root,
            artifact_root: &root,
            generation_id: &request.generation_id,
            source_index_content_hash: &content_hash,
            implementation_version: &request.implementation_version,
            timestamp_unix_seconds: request.timestamp_unix_seconds,
        },
        artifact,
    )
    .map_err(RustIndexError::DocsIndex)?;
    root.publish_manifest(&manifest_for_request(request, artifact, content_hash))
        .map_err(RustIndexError::PublishManifest)?;
    let mut limits = limits;
    limits.extend(root.prune_generations(&request.generation_id));
    Ok(limits)
}

fn manifest_for_request(
    request: &RustIndexRequest,
    artifact: &SourceIndexArtifact,
    source_index_content_hash: String,
) -> OntographManifest {
    OntographManifest {
        repository: RepositoryIdentity {
            root: request.repo_root.display().to_string(),
            vcs: "git".to_string(),
            remote_url: None,
        },
        version: ArtifactVersion {
            schema_version: artifact.schema_version,
            implementation_version: request.implementation_version.clone(),
        },
        active_generation_id: request.generation_id.clone(),
        completed_generations: vec![ArtifactGeneration {
            metadata: GenerationMetadata {
                generation_id: request.generation_id.clone(),
                target_head: request.target_head.clone(),
                created_at_unix_seconds: request.timestamp_unix_seconds,
                completed_at_unix_seconds: request.timestamp_unix_seconds,
                manifest_hash: source_index_content_hash,
            },
            counts: source_index_counts(artifact),
        }],
    }
}

pub(crate) fn source_index_counts(artifact: &SourceIndexArtifact) -> GraphCountSummary {
    let symbols = artifact
        .files
        .iter()
        .map(|file| file.symbols.len() as u64)
        .sum::<u64>();
    let relationships = artifact.relationships.len() as u64;
    let routes = artifact.routes.len() as u64;
    GraphCountSummary {
        files: artifact.files.len() as u64,
        symbols,
        relationships,
        nodes: artifact.files.len() as u64 + symbols + routes,
        edges: relationships,
    }
}

fn source_index_content_hash(contents: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(contents.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn collect_rust_files(repo_root: &Path) -> Result<Vec<PathBuf>, RustIndexError> {
    #[cfg(test)]
    FULL_WORKSPACE_WALK_COUNT.set(FULL_WORKSPACE_WALK_COUNT.get() + 1);
    let mut files = Vec::new();
    collect_rust_files_from(repo_root, repo_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files_from(
    repo_root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), RustIndexError> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(RustIndexError::Walk);
        }
        Err(_) => {
            return Err(RustIndexError::Walk);
        }
    };
    for entry in entries {
        let entry = entry.map_err(|_| RustIndexError::Walk)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|_| RustIndexError::Walk)?;
        if file_type.is_dir() {
            if !is_ignored_dir(repo_root, &path) {
                collect_rust_files_from(repo_root, &path, files)?;
            }
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            files.push(path);
        }
    }
    Ok(())
}

pub(crate) fn is_ignored_dir(repo_root: &Path, path: &Path) -> bool {
    if path == repo_ontograph_root(repo_root) {
        return true;
    }
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | "target" | "third_party" | "tmp")
    )
}

pub(crate) fn relative_slash_path(repo_root: &Path, path: &Path) -> Result<String, RustIndexError> {
    let relative = path
        .strip_prefix(repo_root)
        .map_err(|_| RustIndexError::Walk)?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn collect_rust_symbols(content: &str, path: &str) -> Vec<IndexedSymbol> {
    if let Some(parsed) = parse_rust_file(content) {
        return parsed
            .symbols
            .into_iter()
            .map(|symbol| IndexedSymbol {
                node_id: symbol_node_id(path, &symbol.kind, &symbol.name, symbol.line),
                name: symbol.name,
                kind: symbol.kind,
                line: symbol.line,
            })
            .collect();
    }
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| parse_rust_symbol_line(line).map(|symbol| (index, symbol)))
        .map(|(index, (kind, name))| IndexedSymbol {
            node_id: symbol_node_id(path, &kind, &name, index + 1),
            name,
            kind,
            line: index + 1,
        })
        .collect()
}

pub(crate) fn parse_rust_symbol_line(line: &str) -> Option<(String, String)> {
    let line = strip_comments_and_strings(line);
    let mut tokens = line.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '(' | ')' | '<' | '>' | '{' | '}' | ';' | '=' | ':' | ','
            )
    });
    let kind = tokens.find(|token| is_rust_symbol_kind(token))?;
    let name = tokens.find(|token| !token.is_empty())?;
    Some((kind.to_string(), name.trim_end_matches(':').to_string()))
}

fn is_rust_symbol_kind(token: &str) -> bool {
    matches!(
        token,
        "struct" | "enum" | "trait" | "fn" | "mod" | "type" | "const"
    )
}

pub(crate) fn contains_call(line: &str, callee: &str) -> bool {
    line.match_indices(callee).any(|(start, _)| {
        let before = line[..start].chars().last();
        let after = line[start + callee.len()..].chars().next();
        !matches!(before, Some(character) if character == '_' || character.is_alphanumeric())
            && matches!(after, Some('('))
    })
}

pub(crate) fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, character| match character {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

pub(crate) fn strip_comments_and_strings(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(character) = chars.next() {
        if !in_string && character == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if character == '"' && !escaped {
            in_string = !in_string;
            output.push(' ');
            continue;
        }
        escaped = in_string && character == '\\' && !escaped;
        output.push(if in_string { ' ' } else { character });
        if character != '\\' {
            escaped = false;
        }
    }
    output
}

fn split_search_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    for character in value.chars() {
        if character == '_' || character == '-' || character == '/' || character == '.' {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
        } else if character.is_ascii_uppercase() {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
            token.push(character.to_ascii_lowercase());
        } else if character.is_alphanumeric() {
            token.push(character.to_ascii_lowercase());
        } else if !token.is_empty() {
            tokens.push(std::mem::take(&mut token));
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

#[cfg(test)]
#[path = "indexer_tests.rs"]
mod tests;
