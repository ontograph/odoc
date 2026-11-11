//! Generations: build, atomic publication, verified load and freshness.
//!
//! Layout under the store directory (normally `<root>/.ontodoc`):
//! - `generations/<id>/index.json`: the [`Generation`] artifact.
//! - `generations/<id>/meta.json`: id, artifact SHA-256 and byte length.
//! - `CURRENT`: id of the active generation.
//! - `HISTORY`: published ids, oldest first; the active one plus two older
//!   generations are kept.
//! - `index.lock`: `flock`-style lock held for the whole publication. The lock
//!   dies with its process, so a lock file left by a crash is reclaimed.
//!
//! Publication writes to a temp directory, fsyncs it, renames it into place,
//! then swaps `CURRENT` by rename. Only failures before that rename leave the
//! previous generation active; committed outcomes report subsequent warnings.
//!
//! Generation id = SHA-256 of the canonical JSON of the index-time
//! [`IndexConfig`], the content manifest (path and SHA-256 per file), and
//! canonical skip diagnostics. Filesystem mtimes are not persisted.
//! Query-time parameters are not inputs, so they cannot change it.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::chunk::{self, CHUNK_BUDGET};
use crate::corpus::{self, CorpusError, WalkConfig, sha256_hex};
use crate::graph::{self, DocInput, EdgeKind};
use crate::parse::{self, Span};
use crate::tiers::{self, L0_CAP, L1_CAP, PSEUDO_DOC_CAP, Tier};

pub const SCHEMA_VERSION: u32 = 2;
pub const TOKENIZER_VERSION: u32 = 1;
const KEEP_OLD: usize = 2;
const MAX_POINTER_BYTES: usize = 128;
const MAX_META_BYTES: usize = 4096;
const MAX_ARTIFACT_BYTES: usize = 512 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 195;
const MAX_CLEANUP_PROBES: usize = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PublishOutcome {
    pub generation_id: String,
    pub durability: Durability,
    pub maintenance: Maintenance,
    pub failure_phase: Option<PublishPhase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Synced,
    Uncertain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Maintenance {
    Complete,
    Failed,
    NotAttempted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishPhase {
    NamespaceCreate,
    NamespaceSync,
    TempInventory,
    TempRemoveFile,
    TempRemoveDir,
    TempSync,
    CandidateCreate,
    IndexCreate,
    IndexWrite,
    IndexSync,
    MetaCreate,
    MetaWrite,
    MetaSync,
    CandidateSync,
    RepairInventory,
    RepairRemoveFile,
    RepairRemoveDir,
    RepairSync,
    CandidateRename,
    GenerationsSync,
    CurrentCreate,
    CurrentWrite,
    CurrentFileSync,
    CurrentRename,
    CurrentSync,
    HistoryRead,
    HistoryCreate,
    HistoryWrite,
    HistoryFileSync,
    HistoryRename,
    HistorySync,
    PruneInventory,
    PruneRemoveFile,
    PruneRemoveDir,
    PruneSync,
}

struct Publication<F> {
    hook: F,
    phase: PublishPhase,
    probes: usize,
}

impl<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>> Publication<F> {
    fn step(&mut self, phase: PublishPhase, path: &Path) -> Result<(), StoreError> {
        self.phase = phase;
        (self.hook)(phase, path).map_err(io_err(path))
    }

    fn sync(&mut self, phase: PublishPhase, path: &Path) -> Result<(), StoreError> {
        self.step(phase, path)?;
        sync_dir(path)
    }

    fn write(
        &mut self,
        path: &Path,
        bytes: &[u8],
        phases: [PublishPhase; 3],
    ) -> Result<(), StoreError> {
        self.step(phases[0], path)?;
        let mut file = File::create(path).map_err(io_err(path))?;
        self.step(phases[1], path)?;
        file.write_all(bytes).map_err(io_err(path))?;
        self.step(phases[2], path)?;
        file.sync_all().map_err(io_err(path))
    }
}

/// Index-time configuration; every field feeds the generation id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexConfig {
    pub schema_version: u32,
    pub tokenizer_version: u32,
    pub chunk_budget: usize,
    pub l0_cap: usize,
    pub l1_cap: usize,
    pub pseudo_doc_cap: usize,
    pub max_file_bytes: u64,
    pub max_files: usize,
    pub max_chunks: usize,
    pub max_artifact_bytes: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        let walk = WalkConfig::default();
        Self {
            schema_version: SCHEMA_VERSION,
            tokenizer_version: TOKENIZER_VERSION,
            chunk_budget: CHUNK_BUDGET,
            l0_cap: L0_CAP,
            l1_cap: L1_CAP,
            pseudo_doc_cap: PSEUDO_DOC_CAP,
            max_file_bytes: walk.max_file_bytes,
            max_files: walk.max_files,
            max_chunks: 500_000,
            max_artifact_bytes: MAX_ARTIFACT_BYTES,
        }
    }
}

impl IndexConfig {
    fn validate(&self) -> Result<(), StoreError> {
        if self.schema_version != SCHEMA_VERSION || self.tokenizer_version != TOKENIZER_VERSION {
            return Err(StoreError::Corrupt {
                id: String::new(),
                reason: format!(
                    "unsupported schema/tokenizer version {}/{}; run `ontodoc index`",
                    self.schema_version, self.tokenizer_version
                ),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanRecord {
    pub byte_start: usize,
    pub byte_end: usize,
    pub line_start: usize,
    pub line_end: usize,
}

impl From<Span> for SpanRecord {
    fn from(s: Span) -> Self {
        Self {
            byte_start: s.byte_start,
            byte_end: s.byte_end,
            line_start: s.line_start,
            line_end: s.line_end,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkipEntry {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRecord {
    pub path: String,
    pub text: String,
    pub pseudo_doc: String,
    pub pseudo_doc_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionRecord {
    pub id: String,
    pub path: String,
    pub heading_path: Vec<String>,
    pub level: u8,
    pub anchor: String,
    pub span: SpanRecord,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub id: String,
    pub section_id: String,
    pub ordinal: u32,
    pub path: String,
    pub heading_path: Vec<String>,
    pub span: SpanRecord,
    pub sha256: String,
    pub l0: String,
    pub l0_truncated: bool,
    pub l1: String,
    pub l1_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkDiagnosticRecord {
    pub path: String,
    pub span: SpanRecord,
    pub target: String,
    pub problem: String,
}

/// One complete, self-contained index generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generation {
    pub generation_id: String,
    pub config: IndexConfig,
    /// Corpus manifest, sorted by path.
    pub files: Vec<FileEntry>,
    pub skipped: Vec<SkipEntry>,
    pub documents: Vec<DocRecord>,
    pub sections: Vec<SectionRecord>,
    pub chunks: Vec<ChunkRecord>,
    pub edges: Vec<EdgeRecord>,
    pub link_diagnostics: Vec<LinkDiagnosticRecord>,
}

impl Generation {
    fn validate(&self) -> Result<(), StoreError> {
        self.config.validate()?;
        self.validate_unique_ids()?;
        if self.files.windows(2).any(|p| p[0].path >= p[1].path)
            || self
                .skipped
                .windows(2)
                .any(|p| (&p[0].path, &p[0].reason) > (&p[1].path, &p[1].reason))
        {
            return Err(StoreError::Corrupt {
                id: self.generation_id.clone(),
                reason: "noncanonical manifest order".into(),
            });
        }
        if generation_id(&self.config, &self.files, &self.skipped)? != self.generation_id {
            return Err(StoreError::Corrupt {
                id: self.generation_id.clone(),
                reason: "generation id mismatch".into(),
            });
        }
        Ok(())
    }

    /// Reject ambiguous identities before lookup maps can discard duplicates.
    fn validate_unique_ids(&self) -> Result<(), StoreError> {
        let mut seen = HashSet::new();
        for (kind, id) in self
            .sections
            .iter()
            .map(|s| ("section", s.id.as_str()))
            .chain(self.chunks.iter().map(|c| ("chunk", c.id.as_str())))
        {
            if !seen.insert((kind, id)) {
                return Err(StoreError::Corrupt {
                    id: self.generation_id.clone(),
                    reason: format!("duplicate {kind} id"),
                });
            }
        }
        Ok(())
    }

    /// Text of a chunk, taken from its document.
    pub fn chunk_text(&self, chunk: &ChunkRecord) -> Option<&str> {
        let doc = self.documents.iter().find(|d| d.path == chunk.path)?;
        doc.text.get(chunk.span.byte_start..chunk.span.byte_end)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    generation_id: String,
    artifact_sha256: String,
    artifact_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Freshness {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

impl Freshness {
    pub fn is_fresh(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Corpus(#[from] CorpusError),
    #[error("another process holds the index lock")]
    Locked,
    #[error("{what} exceeds cap {cap}")]
    CapExceeded { what: &'static str, cap: usize },
    #[error("no published generation")]
    NoGeneration,
    #[error("corrupt generation {id}: {reason}")]
    Corrupt { id: String, reason: String },
    #[error("serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Generation id from the index-time config and the content manifest.
pub fn generation_id(
    config: &IndexConfig,
    files: &[FileEntry],
    skipped: &[SkipEntry],
) -> Result<String, StoreError> {
    let manifest: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.path.as_str(), f.sha256.as_str()))
        .collect();
    let skips: Vec<(&str, &str)> = skipped
        .iter()
        .map(|s| (s.path.as_str(), s.reason.as_str()))
        .collect();
    let canonical = serde_json::to_vec(&(config, manifest, skips))?;
    Ok(sha256_hex(&canonical))
}

/// Walks, parses, chunks, derives tiers and builds the graph for `root`.
pub fn build(root: &Path, config: &IndexConfig) -> Result<Generation, StoreError> {
    build_rooted(&corpus::RootedCorpus::open(root)?, config)
}

pub(crate) fn build_rooted(
    root: &corpus::RootedCorpus,
    config: &IndexConfig,
) -> Result<Generation, StoreError> {
    config.validate()?;
    let corpus = root.walk(
        &WalkConfig {
            max_file_bytes: config.max_file_bytes,
            max_files: config.max_files,
        },
        &mut corpus::WalkBudget::new(128 * 1024 * 1024, 100_000),
    )?;
    let parsed: Vec<_> = corpus
        .files
        .iter()
        .map(|f| {
            let sections = parse::parse_sections(&f.rel_path, &f.text);
            let chunks = chunk::chunk_sections(&f.text, &sections, config.chunk_budget);
            (sections, chunks)
        })
        .collect();
    let chunk_count: usize = parsed.iter().map(|(_, c)| c.len()).sum();
    if chunk_count > config.max_chunks {
        return Err(StoreError::CapExceeded {
            what: "chunk count",
            cap: config.max_chunks,
        });
    }
    let inputs: Vec<DocInput<'_>> = corpus
        .files
        .iter()
        .zip(&parsed)
        .map(|(f, (sections, chunks))| DocInput {
            rel_path: &f.rel_path,
            source: &f.text,
            sections,
            chunks,
        })
        .collect();
    let graph = graph::build(&inputs);

    let mut generation = Generation {
        generation_id: String::new(),
        config: config.clone(),
        files: Vec::new(),
        skipped: canonical_skips(&corpus.skipped),
        documents: Vec::new(),
        sections: Vec::new(),
        chunks: Vec::with_capacity(chunk_count),
        edges: graph
            .edges
            .into_iter()
            .map(|e| EdgeRecord {
                kind: edge_kind_name(e.kind).to_string(),
                from: e.from,
                to: e.to,
            })
            .collect(),
        link_diagnostics: graph
            .diagnostics
            .into_iter()
            .map(|d| LinkDiagnosticRecord {
                path: d.rel_path,
                span: d.span.into(),
                target: d.target,
                problem: format!("{:?}", d.problem),
            })
            .collect(),
    };
    for (file, (sections, chunks)) in corpus.files.into_iter().zip(parsed) {
        let tiers = tiers::chunk_tiers(&file.text, &sections, &chunks);
        let pseudo = tiers::pseudo_document(&file.text, &sections);
        for (c, t) in chunks.into_iter().zip(tiers) {
            generation.chunks.push(chunk_record(c, t.l0, t.l1));
        }
        generation
            .sections
            .extend(sections.into_iter().map(|s| SectionRecord {
                id: s.id,
                path: s.rel_path,
                heading_path: s.heading_path,
                level: s.level,
                anchor: s.anchor,
                span: s.span.into(),
                sha256: s.sha256,
            }));
        generation.files.push(FileEntry {
            path: file.rel_path.clone(),
            size: file.text.len() as u64,
            sha256: file.sha256,
        });
        generation.documents.push(DocRecord {
            path: file.rel_path,
            text: file.text,
            pseudo_doc: pseudo.text,
            pseudo_doc_truncated: pseudo.truncated,
        });
    }
    generation.generation_id = generation_id(config, &generation.files, &generation.skipped)?;
    Ok(generation)
}

/// Stable, root-independent diagnostics; raw OS messages never enter artifacts.
fn canonical_skips(skipped: &[corpus::Skip]) -> Vec<SkipEntry> {
    use corpus::SkipReason;
    use std::path::Component;
    let mut out: Vec<_> = skipped
        .iter()
        .map(|s| {
            let path = Path::new(&s.path);
            let path = if path
                .components()
                .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
            {
                String::new()
            } else {
                path.components()
                    .filter_map(|c| match c {
                        Component::Normal(s) => Some(s.to_string_lossy()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("/")
            };
            let reason = match &s.reason {
                SkipReason::Symlink => "symlink".into(),
                SkipReason::NotUtf8 => "not_utf8".into(),
                SkipReason::NonUtf8Path => "non_utf8_path".into(),
                SkipReason::Changed => "changed".into(),
                SkipReason::Io { .. } => "io".into(),
                SkipReason::TooLarge { size, cap } => format!("too_large:size={size};cap={cap}"),
            };
            SkipEntry { path, reason }
        })
        .collect();
    out.sort_by(|a, b| (&a.path, &a.reason).cmp(&(&b.path, &b.reason)));
    out
}

fn chunk_record(c: chunk::Chunk, l0: Tier, l1: Tier) -> ChunkRecord {
    ChunkRecord {
        id: c.id,
        section_id: c.section_id,
        ordinal: c.ordinal,
        path: c.rel_path,
        heading_path: c.heading_path,
        span: c.span.into(),
        sha256: c.sha256,
        l0: l0.text,
        l0_truncated: l0.truncated,
        l1: l1.text,
        l1_truncated: l1.truncated,
    }
}

fn edge_kind_name(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Contains => "CONTAINS",
        EdgeKind::Next => "NEXT",
        EdgeKind::LinksTo => "LINKS_TO",
        EdgeKind::Mentions => "MENTIONS",
    }
}

/// Compares the generation's manifest with the corpus as it is now.
pub fn freshness(root: &Path, generation: &Generation) -> Result<Freshness, StoreError> {
    freshness_rooted(&corpus::RootedCorpus::open(root)?, generation)
}

pub(crate) fn freshness_rooted(
    root: &corpus::RootedCorpus,
    generation: &Generation,
) -> Result<Freshness, StoreError> {
    freshness_rooted_bounded(
        root,
        generation,
        &mut corpus::WalkBudget::new(128 * 1024 * 1024, 100_000),
    )
}

fn freshness_rooted_bounded(
    root: &corpus::RootedCorpus,
    generation: &Generation,
    budget: &mut corpus::WalkBudget,
) -> Result<Freshness, StoreError> {
    let mut current = Vec::new();
    root.visit(
        &WalkConfig {
            max_file_bytes: generation.config.max_file_bytes,
            max_files: generation.config.max_files,
        },
        budget,
        |file| {
            current.push(FileEntry {
                path: file.rel_path,
                size: file.size,
                sha256: file.sha256,
            })
        },
    )?;
    // Depth-first, directory-local sorting is not global path order (a.md/a/x.md).
    current.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    Ok(compare_freshness(generation, &current))
}

pub(crate) fn freshness_bounded(
    root: &Path,
    generation: &Generation,
    budget: &mut corpus::WalkBudget,
) -> Result<Freshness, StoreError> {
    freshness_rooted_bounded(&corpus::RootedCorpus::open(root)?, generation, budget)
}

fn compare_freshness(generation: &Generation, current: &[FileEntry]) -> Freshness {
    let mut out = Freshness::default();
    let (mut old, mut new) = (
        generation.files.iter().peekable(),
        current.iter().peekable(),
    );
    loop {
        match (old.peek(), new.peek()) {
            (Some(o), Some(n)) => match o.path.as_bytes().cmp(n.path.as_bytes()) {
                std::cmp::Ordering::Less => {
                    out.removed.push(o.path.clone());
                    old.next();
                }
                std::cmp::Ordering::Greater => {
                    out.added.push(n.path.clone());
                    new.next();
                }
                std::cmp::Ordering::Equal => {
                    if o.sha256 != n.sha256 {
                        out.modified.push(o.path.clone());
                    }
                    old.next();
                    new.next();
                }
            },
            (Some(o), None) => {
                out.removed.push(o.path.clone());
                old.next();
            }
            (None, Some(n)) => {
                out.added.push(n.path.clone());
                new.next();
            }
            (None, None) => return out,
        }
    }
}

/// A generation store rooted at one directory.
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Publishes atomically; `Err` always means CURRENT was not replaced.
    pub fn publish(&self, generation: &Generation) -> Result<PublishOutcome, StoreError> {
        self.publish_inner(generation, |_, _| Ok(()))
    }

    fn publish_inner(
        &self,
        generation: &Generation,
        hook: impl FnMut(PublishPhase, &Path) -> std::io::Result<()>,
    ) -> Result<PublishOutcome, StoreError> {
        generation.validate()?;
        let artifact = serialize_artifact(generation)?;
        let id = &generation.generation_id;
        let meta = serde_json::to_vec(&Meta {
            generation_id: id.clone(),
            artifact_sha256: sha256_hex(&artifact),
            artifact_bytes: artifact.len(),
        })?;
        let mut publication = Publication {
            hook,
            phase: PublishPhase::NamespaceCreate,
            probes: 0,
        };
        let generations = self.dir.join("generations");
        ensure_namespace(&self.dir, true, &mut publication)?;
        ensure_namespace(&generations, true, &mut publication)?;
        let _lock = self.lock()?;
        let current = match self.current_id() {
            Ok(id) => Some(id),
            Err(StoreError::NoGeneration) => None,
            Err(error) => return Err(error),
        };
        self.remove_temp_dirs(&mut publication)?;

        let target = generations.join(id);
        match fs::symlink_metadata(&target) {
            Ok(metadata) if !metadata.is_dir() => {
                return Err(invalid_store("unsafe candidate directory", id));
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(io_err(&target)(error)),
        }
        let existing = match self.verify_dir(id) {
            Ok(bytes) => Some(bytes),
            Err(StoreError::Corrupt { .. }) => None,
            Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                None
            }
            Err(error) => return Err(error),
        };
        if let Some((existing, existing_meta)) = existing {
            if existing != artifact || existing_meta != meta {
                return Err(invalid_store("same-id artifact conflict", id));
            }
            for (name, phase) in [
                ("index.json", PublishPhase::IndexSync),
                ("meta.json", PublishPhase::MetaSync),
            ] {
                let path = target.join(name);
                publication.step(phase, &path)?;
                File::open(&path)
                    .and_then(|file| file.sync_all())
                    .map_err(io_err(&path))?;
            }
            publication.sync(PublishPhase::CandidateSync, &target)?;
            publication.sync(PublishPhase::GenerationsSync, &generations)?;
        } else {
            // Preserve unsafe/capped target refusals even if metadata parsing failed first.
            for (name, cap, what) in [
                ("meta.json", MAX_META_BYTES, "metadata bytes"),
                ("index.json", MAX_ARTIFACT_BYTES, "artifact bytes"),
            ] {
                match read_store_file(&target.join(name), cap, what) {
                    Ok(_) => (),
                    Err(StoreError::Io { source, .. })
                        if source.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error),
                }
            }
            let repair = match fs::symlink_metadata(&target) {
                Ok(_) => {
                    if current.as_deref() == Some(id) {
                        return Err(invalid_store("cannot repair active generation", id));
                    }
                    publication.step(PublishPhase::RepairInventory, &target)?;
                    Some(owned_directory(&target, false, &mut publication)?)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(io_err(&target)(error)),
            };
            let tmp = self.dir.join(format!("tmp-{id}"));
            publication.step(PublishPhase::CandidateCreate, &tmp)?;
            fs::create_dir(&tmp).map_err(io_err(&tmp))?;
            publication.write(
                &tmp.join("index.json"),
                &artifact,
                [
                    PublishPhase::IndexCreate,
                    PublishPhase::IndexWrite,
                    PublishPhase::IndexSync,
                ],
            )?;
            publication.write(
                &tmp.join("meta.json"),
                &meta,
                [
                    PublishPhase::MetaCreate,
                    PublishPhase::MetaWrite,
                    PublishPhase::MetaSync,
                ],
            )?;
            publication.sync(PublishPhase::CandidateSync, &tmp)?;
            if let Some(files) = repair {
                remove_owned(
                    &target,
                    &files,
                    &mut publication,
                    [
                        PublishPhase::RepairRemoveFile,
                        PublishPhase::RepairRemoveDir,
                        PublishPhase::RepairSync,
                    ],
                )?;
            }
            publication.step(PublishPhase::CandidateRename, &target)?;
            fs::rename(&tmp, &target).map_err(io_err(&target))?;
            publication.sync(PublishPhase::GenerationsSync, &generations)?;
        }
        let pointer_tmp = self.dir.join("CURRENT.tmp");
        publication.write(
            &pointer_tmp,
            id.as_bytes(),
            [
                PublishPhase::CurrentCreate,
                PublishPhase::CurrentWrite,
                PublishPhase::CurrentFileSync,
            ],
        )?;
        let pointer = self.dir.join("CURRENT");
        publication.step(PublishPhase::CurrentRename, &pointer)?;
        fs::rename(&pointer_tmp, &pointer).map_err(io_err(&pointer))?;
        // No fallible return may cross this commit point.
        let mut outcome = PublishOutcome {
            generation_id: id.clone(),
            durability: Durability::Synced,
            maintenance: Maintenance::Complete,
            failure_phase: None,
        };
        if publication
            .sync(PublishPhase::CurrentSync, &self.dir)
            .is_err()
        {
            outcome.durability = Durability::Uncertain;
            outcome.maintenance = Maintenance::NotAttempted;
            outcome.failure_phase = Some(PublishPhase::CurrentSync);
        } else if self.record_and_prune(id, &mut publication).is_err() {
            outcome.maintenance = Maintenance::Failed;
            outcome.failure_phase = Some(publication.phase);
        }
        Ok(outcome)
    }

    /// Id of the active generation, validated before use as a path component.
    pub fn current_id(&self) -> Result<String, StoreError> {
        let pointer = self.dir.join("CURRENT");
        let bytes = match read_store_file(&pointer, MAX_POINTER_BYTES, "CURRENT bytes") {
            Ok(bytes) => bytes,
            Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(StoreError::NoGeneration);
            }
            Err(e) => return Err(e),
        };
        parse_pointer(&bytes)
    }

    pub(crate) fn current_id_rooted(
        &self,
        root: &corpus::RootedCorpus,
    ) -> Result<String, StoreError> {
        let bytes = match rooted_store_file(
            root,
            Path::new(".ontodoc/CURRENT"),
            MAX_POINTER_BYTES,
            "CURRENT bytes",
        ) {
            Err(StoreError::Corpus(CorpusError::NotFound)) => return Err(StoreError::NoGeneration),
            result => result?,
        };
        parse_pointer(&bytes)
    }

    /// `id` is selected from CURRENT by the application, never from a citation.
    pub(crate) fn load_id_rooted(
        &self,
        root: &corpus::RootedCorpus,
        id: &str,
    ) -> Result<Generation, StoreError> {
        validate_id(id)?;
        let dir = Path::new(".ontodoc/generations").join(id);
        let meta_bytes = rooted_store_file(
            root,
            &dir.join("meta.json"),
            MAX_META_BYTES,
            "metadata bytes",
        )?;
        let meta = parse_meta(id, &meta_bytes)?;
        let artifact = rooted_store_file(
            root,
            &dir.join("index.json"),
            meta.artifact_bytes,
            "artifact bytes",
        )?;
        verify_captured_artifact(id, &meta_bytes, &artifact)?;
        decode_verified_generation(id, &artifact)
    }

    /// Loads and verifies the active generation.
    pub fn load(&self) -> Result<Generation, StoreError> {
        let id = self.current_id()?;
        let (artifact, _) = self.verify_dir(&id)?;
        decode_verified_generation(&id, &artifact)
    }

    /// Returns exact artifact and metadata bytes when their hash/length agree.
    fn verify_dir(&self, id: &str) -> Result<(Vec<u8>, Vec<u8>), StoreError> {
        validate_id(id)?;
        let dir = self.dir.join("generations").join(id);
        let meta_path = dir.join("meta.json");
        let meta_bytes = read_store_file(&meta_path, MAX_META_BYTES, "metadata bytes")?;
        // Preserve metadata parse failure precedence before opening the artifact.
        let meta = parse_meta(id, &meta_bytes)?;
        let index_path = dir.join("index.json");
        let artifact = read_store_file(&index_path, meta.artifact_bytes, "artifact bytes")?;
        verify_captured_artifact(id, &meta_bytes, &artifact)?;
        Ok((artifact, meta_bytes))
    }

    fn lock(&self) -> Result<File, StoreError> {
        let path = self.dir.join("index.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(io_err(&path))?;
        match file.try_lock() {
            Ok(()) => Ok(file),
            Err(fs::TryLockError::WouldBlock) => Err(StoreError::Locked),
            Err(fs::TryLockError::Error(e)) => Err(io_err(&path)(e)),
        }
    }

    fn remove_temp_dirs<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
        &self,
        publication: &mut Publication<F>,
    ) -> Result<(), StoreError> {
        publication.step(PublishPhase::TempInventory, &self.dir)?;
        let mut inventory = Vec::new();
        for name in directory_names(&self.dir, publication)? {
            if let Some(id) = name.strip_prefix("tmp-") {
                validate_id(id)?;
                let path = self.dir.join(name);
                let files = owned_directory(&path, true, publication)?;
                inventory.push((path, files));
            }
        }
        for (path, files) in inventory {
            remove_owned(
                &path,
                &files,
                publication,
                [
                    PublishPhase::TempRemoveFile,
                    PublishPhase::TempRemoveDir,
                    PublishPhase::TempSync,
                ],
            )?;
        }
        Ok(())
    }

    fn record_and_prune<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
        &self,
        id: &str,
        publication: &mut Publication<F>,
    ) -> Result<(), StoreError> {
        let history_path = self.dir.join("HISTORY");
        publication.step(PublishPhase::HistoryRead, &history_path)?;
        let history = match read_store_file(&history_path, MAX_HISTORY_BYTES, "HISTORY bytes") {
            Ok(bytes) => bytes,
            Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
                Vec::new()
            }
            Err(error) => return Err(error),
        };
        let mut ids = parse_history(&history)?;
        ids.retain(|old| old != id);
        ids.push(id.to_string());
        let keep = &ids[ids.len().saturating_sub(KEEP_OLD + 1)..];
        let tmp = self.dir.join("HISTORY.tmp");
        publication.write(
            &tmp,
            format!("{}\n", keep.join("\n")).as_bytes(),
            [
                PublishPhase::HistoryCreate,
                PublishPhase::HistoryWrite,
                PublishPhase::HistoryFileSync,
            ],
        )?;
        publication.step(PublishPhase::HistoryRename, &history_path)?;
        fs::rename(&tmp, &history_path).map_err(io_err(&history_path))?;
        publication.sync(PublishPhase::HistorySync, &self.dir)?;

        let generations = self.dir.join("generations");
        publication.step(PublishPhase::PruneInventory, &generations)?;
        let mut inventory = Vec::new();
        for name in directory_names(&generations, publication)? {
            validate_id(&name)?;
            let path = generations.join(&name);
            let files = owned_directory(&path, false, publication)?;
            if name != id && !keep.contains(&name) {
                inventory.push((path, files));
            }
        }
        for (path, files) in inventory {
            remove_owned(
                &path,
                &files,
                publication,
                [
                    PublishPhase::PruneRemoveFile,
                    PublishPhase::PruneRemoveDir,
                    PublishPhase::PruneSync,
                ],
            )?;
        }
        Ok(())
    }
}

fn invalid_store(reason: &str, id: &str) -> StoreError {
    StoreError::Corrupt {
        id: id.into(),
        reason: reason.into(),
    }
}

fn parse_history(bytes: &[u8]) -> Result<Vec<String>, StoreError> {
    check_bytes(bytes.len(), MAX_HISTORY_BYTES, "HISTORY bytes")?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if !bytes.ends_with(b"\n") {
        return Err(invalid_store("HISTORY lacks final LF", ""));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| invalid_store("invalid HISTORY", ""))?;
    let mut ids = Vec::new();
    for id in text[..text.len() - 1].split('\n') {
        validate_id(id)?;
        if ids.len() == KEEP_OLD + 1 || ids.iter().any(|old| old == id) {
            return Err(invalid_store("noncanonical HISTORY", ""));
        }
        ids.push(id.to_string());
    }
    Ok(ids)
}

// Re-sync containing namespaces on retries too: an earlier mkdir may have
// succeeded before its parent fsync failed.
fn ensure_namespace<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
    path: &Path,
    owned: bool,
    publication: &mut Publication<F>,
) -> Result<(), StoreError> {
    let parent = path.parent().map(|parent| {
        if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        }
    });
    if let Some(parent) = parent
        && parent != path
        && parent != Path::new(".")
    {
        ensure_namespace(parent, false, publication)?;
    }
    // Publication trusts administrator-controlled ancestors (including host
    // aliases such as /tmp), but its own store/generations must be real dirs.
    let metadata = if owned {
        fs::symlink_metadata(path)
    } else {
        fs::metadata(path)
    };
    match metadata {
        Ok(metadata) if metadata.is_dir() => (),
        Ok(_) => return Err(invalid_store("unsafe store directory", "")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            publication.step(PublishPhase::NamespaceCreate, path)?;
            match fs::create_dir(path) {
                Ok(()) => (),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !fs::symlink_metadata(path).map_err(io_err(path))?.is_dir() {
                        return Err(invalid_store("unsafe store directory", ""));
                    }
                }
                Err(error) => return Err(io_err(path)(error)),
            }
        }
        Err(error) => return Err(io_err(path)(error)),
    }
    if let Some(parent) = parent {
        publication.sync(PublishPhase::NamespaceSync, parent)?;
    }
    // The store's parent must also be synced when generations already exists.
    Ok(())
}

fn directory_names<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
    path: &Path,
    publication: &mut Publication<F>,
) -> Result<Vec<String>, StoreError> {
    use rustix::fs::{Mode, OFlags, RawDir};
    let fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| io_err(path)(error.into()))?;
    let mut buffer = [MaybeUninit::uninit(); 4096];
    let mut entries = RawDir::new(fd, &mut buffer);
    let mut names = Vec::new();
    loop {
        if publication.probes == MAX_CLEANUP_PROBES {
            return Err(StoreError::CapExceeded {
                what: "cleanup probes",
                cap: MAX_CLEANUP_PROBES,
            });
        }
        publication.probes += 1;
        publication.step(publication.phase, path)?;
        let Some(entry) = entries.next() else { break };
        let entry = entry.map_err(|error| io_err(path)(error.into()))?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        let name =
            std::str::from_utf8(name).map_err(|_| invalid_store("invalid cleanup name", ""))?;
        // All owned components are <=68 bytes; other store files have fixed names.
        if name.len() > 68 {
            return Err(invalid_store("invalid cleanup name", ""));
        }
        names.push(name.to_string());
    }
    names.sort();
    Ok(names)
}

fn owned_directory<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
    path: &Path,
    incomplete: bool,
    publication: &mut Publication<F>,
) -> Result<Vec<String>, StoreError> {
    if !fs::symlink_metadata(path).map_err(io_err(path))?.is_dir() {
        return Err(invalid_store("cleanup target is not a real directory", ""));
    }
    let names = directory_names(path, publication)?;
    for name in &names {
        if !matches!(name.as_str(), "index.json" | "meta.json")
            || !fs::symlink_metadata(path.join(name))
                .map_err(io_err(path))?
                .is_file()
        {
            return Err(invalid_store("unexpected cleanup contents", ""));
        }
    }
    if !incomplete && names != ["index.json", "meta.json"] {
        return Err(invalid_store("incomplete generation directory", ""));
    }
    Ok(names)
}

fn remove_owned<F: FnMut(PublishPhase, &Path) -> std::io::Result<()>>(
    path: &Path,
    files: &[String],
    publication: &mut Publication<F>,
    phases: [PublishPhase; 3],
) -> Result<(), StoreError> {
    for name in files {
        let file = path.join(name);
        publication.step(phases[0], &file)?;
        fs::remove_file(&file).map_err(io_err(&file))?;
    }
    publication.step(phases[1], path)?;
    fs::remove_dir(path).map_err(io_err(path))?;
    if let Some(parent) = path.parent() {
        publication.sync(phases[2], parent)?;
    }
    Ok(())
}

fn parse_pointer(bytes: &[u8]) -> Result<String, StoreError> {
    let id = std::str::from_utf8(bytes)
        .map_err(|_| StoreError::Corrupt {
            id: String::new(),
            reason: "CURRENT is not UTF-8".into(),
        })?
        .trim();
    validate_id(id)?;
    Ok(id.to_string())
}

fn rooted_store_file(
    root: &corpus::RootedCorpus,
    relative: &Path,
    cap: usize,
    what: &'static str,
) -> Result<Vec<u8>, StoreError> {
    match root.read_regular(relative, cap as u64) {
        Ok((_, bytes)) => Ok(bytes),
        Err(CorpusError::TooLarge { .. }) => Err(StoreError::CapExceeded { what, cap }),
        Err(e) => Err(e.into()),
    }
}

fn validate_id(id: &str) -> Result<(), StoreError> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(StoreError::Corrupt {
            id: String::new(),
            reason: "invalid generation id".into(),
        });
    }
    Ok(())
}

fn check_bytes(len: usize, cap: usize, what: &'static str) -> Result<(), StoreError> {
    if len > cap {
        return Err(StoreError::CapExceeded { what, cap });
    }
    Ok(())
}

/// Linux final-component protection only; ancestors remain SAFETY-0/OD-3.4.
fn read_store_file(path: &Path, cap: usize, what: &'static str) -> Result<Vec<u8>, StoreError> {
    const O_NOFOLLOW: i32 = 0o400000;
    const O_NONBLOCK: i32 = 0o4000;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .map_err(io_err(path))?;
    let meta = file.metadata().map_err(io_err(path))?;
    if !meta.is_file() {
        return Err(io_err(path)(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "store input is not a regular file",
        )));
    }
    if meta.len() > cap as u64 {
        return Err(StoreError::CapExceeded { what, cap });
    }
    read_capped(file, path, cap, what)
}

fn read_capped(
    reader: impl Read,
    path: &Path,
    cap: usize,
    what: &'static str,
) -> Result<Vec<u8>, StoreError> {
    let mut bytes = Vec::new();
    reader
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_err(path))?;
    check_bytes(bytes.len(), cap, what)?;
    Ok(bytes)
}

fn serialize_artifact(generation: &Generation) -> Result<Vec<u8>, StoreError> {
    struct BoundedWriter {
        bytes: Vec<u8>,
        cap: usize,
        exceeded: bool,
    }
    impl Write for BoundedWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.cap - self.bytes.len() {
                self.exceeded = true;
                return Err(std::io::ErrorKind::FileTooLarge.into());
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let cap = generation.config.max_artifact_bytes.min(MAX_ARTIFACT_BYTES);
    let mut writer = BoundedWriter {
        bytes: Vec::new(),
        cap,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, generation);
    if writer.exceeded {
        return Err(StoreError::CapExceeded {
            what: "artifact bytes",
            cap,
        });
    }
    result?;
    Ok(writer.bytes)
}

fn parse_meta(id: &str, bytes: &[u8]) -> Result<Meta, StoreError> {
    validate_id(id)?;
    check_bytes(bytes.len(), MAX_META_BYTES, "metadata bytes")?;
    let meta: Meta = serde_json::from_slice(bytes).map_err(|e| StoreError::Corrupt {
        id: id.into(),
        reason: format!("meta: {e}"),
    })?;
    check_bytes(meta.artifact_bytes, MAX_ARTIFACT_BYTES, "artifact bytes")?;
    Ok(meta)
}

fn verify_captured_artifact(id: &str, meta: &[u8], artifact: &[u8]) -> Result<(), StoreError> {
    check_bytes(artifact.len(), MAX_ARTIFACT_BYTES, "artifact bytes")?;
    let meta = parse_meta(id, meta)?;
    if meta.generation_id != id
        || meta.artifact_bytes != artifact.len()
        || meta.artifact_sha256 != sha256_hex(artifact)
    {
        return Err(StoreError::Corrupt {
            id: id.into(),
            reason: "artifact hash mismatch".into(),
        });
    }
    Ok(())
}

/// Decode already captured bytes using the same verifier as the normal loader.
pub(crate) fn decode_captured_generation(
    id: &str,
    meta: &[u8],
    artifact: &[u8],
) -> Result<Generation, StoreError> {
    verify_captured_artifact(id, meta, artifact)?;
    decode_verified_generation(id, artifact)
}

fn decode_verified_generation(id: &str, artifact: &[u8]) -> Result<Generation, StoreError> {
    validate_id(id)?;
    check_bytes(artifact.len(), MAX_ARTIFACT_BYTES, "artifact bytes")?;
    let corrupt = |e: String| StoreError::Corrupt {
        id: id.into(),
        reason: e,
    };
    // Version rejection precedes schema-2 field decoding.
    #[derive(Deserialize)]
    struct VersionHeader {
        config: IndexConfig,
    }
    let header: VersionHeader =
        serde_json::from_slice(artifact).map_err(|e| corrupt(e.to_string()))?;
    header.config.validate()?;
    check_bytes(
        artifact.len(),
        header.config.max_artifact_bytes,
        "artifact bytes",
    )?;
    preflight_collections(id, artifact, &header.config)?;
    let generation: Generation =
        serde_json::from_slice(artifact).map_err(|e| corrupt(e.to_string()))?;
    if generation.generation_id != id {
        return Err(corrupt("generation id mismatch".into()));
    }
    generation.validate()?;
    crate::search::validate_input(&generation)?;
    Ok(generation)
}

// Count raw collection elements before allocating typed records. In particular,
// duplicate JSON keys must not reset counters, even though typed decode rejects
// those keys later. Configuration was read separately, independent of key order.
#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum CollectionField {
    Files,
    Documents,
    Sections,
    Chunks,
    #[serde(other)]
    Other,
}

const COLLECTION_NAMES: [&str; 4] = [
    "file count",
    "document count",
    "section count",
    "chunk count",
];

struct CollectionPreflight {
    caps: [usize; 4],
    counts: [usize; 4],
    exceeded: Option<usize>,
}

impl<'de> serde::de::DeserializeSeed<'de> for &mut CollectionPreflight {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> serde::de::Visitor<'de> for &mut CollectionPreflight {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a generation object")
    }

    fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
        while let Some(key) = map.next_key::<CollectionField>()? {
            let index = match key {
                CollectionField::Files => 0,
                CollectionField::Documents => 1,
                CollectionField::Sections => 2,
                CollectionField::Chunks => 3,
                CollectionField::Other => {
                    map.next_value::<serde::de::IgnoredAny>()?;
                    continue;
                }
            };
            map.next_value_seed(CollectionArray { state: self, index })?;
        }
        Ok(())
    }
}

struct CollectionArray<'a> {
    state: &'a mut CollectionPreflight,
    index: usize,
}

impl<'de> serde::de::DeserializeSeed<'de> for CollectionArray<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> serde::de::Visitor<'de> for CollectionArray<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an array")
    }

    fn visit_seq<S: serde::de::SeqAccess<'de>>(self, mut seq: S) -> Result<(), S::Error> {
        while seq
            .next_element_seed(CollectionElement {
                state: self.state,
                index: self.index,
            })?
            .is_some()
        {}
        Ok(())
    }
}

struct CollectionElement<'a> {
    state: &'a mut CollectionPreflight,
    index: usize,
}

impl<'de> serde::de::DeserializeSeed<'de> for CollectionElement<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        let count = &mut self.state.counts[self.index];
        if *count >= self.state.caps[self.index] {
            self.state.exceeded = Some(self.index);
            return Err(serde::de::Error::custom(
                "generation collection limit exceeded",
            ));
        }
        *count += 1;
        serde::de::IgnoredAny::deserialize(deserializer).map(|_| ())
    }
}

fn preflight_collections(
    id: &str,
    artifact: &[u8],
    config: &IndexConfig,
) -> Result<(), StoreError> {
    use serde::de::DeserializeSeed;
    let files = config.max_files.min(100_000);
    let chunks = config.max_chunks.min(500_000);
    let mut state = CollectionPreflight {
        caps: [files, files, chunks, chunks],
        counts: [0; 4],
        exceeded: None,
    };
    let mut decoder = serde_json::Deserializer::from_slice(artifact);
    let result = (&mut state)
        .deserialize(&mut decoder)
        .and_then(|()| decoder.end());
    if let Some(index) = state.exceeded {
        return Err(StoreError::CapExceeded {
            what: COLLECTION_NAMES[index],
            cap: state.caps[index],
        });
    }
    result.map_err(|error| StoreError::Corrupt {
        id: id.into(),
        reason: error.to_string(),
    })
}

fn sync_dir(path: &Path) -> Result<(), StoreError> {
    File::open(path)
        .and_then(|d| d.sync_all())
        .map_err(io_err(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn corpus() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::write(r.join("a.md"), "# A\n\nAlpha text. See [b](b.md).\n").unwrap();
        fs::write(r.join("b.md"), "# B\n\nBeta text.\n\n## Sub\n\nMore.\n").unwrap();
        dir
    }

    fn store_of(root: &Path) -> Store {
        Store::new(root.join(".ontodoc"))
    }

    // A separate libtest process acquires the lock itself. No inherited-lock
    // assumption, global environment mutation, sleeps, or contention retries.
    struct LockChild {
        child: std::process::Child,
        input: Option<std::process::ChildStdin>,
        lines: std::sync::mpsc::Receiver<Result<Option<String>, String>>,
        reader: Option<std::thread::JoinHandle<()>>,
    }

    fn bounded_line(input: &mut impl std::io::Read) -> Result<Option<String>, String> {
        let mut bytes = Vec::new();
        for _ in 0..512 {
            let mut byte = [0];
            match input.read(&mut byte) {
                Ok(0) if bytes.is_empty() => return Ok(None),
                Ok(0) => return Err("partial helper line".into()),
                Ok(_) if byte[0] == b'\n' => {
                    return String::from_utf8(bytes)
                        .map(Some)
                        .map_err(|e| e.to_string());
                }
                Ok(_) => bytes.push(byte[0]),
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("oversized helper line".into())
    }

    impl LockChild {
        fn spawn(mode: &str, root: &Path) -> Self {
            use std::process::{Command, Stdio};
            let (sender, lines) = std::sync::mpsc::sync_channel(129);
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "store::tests::lock_process_child",
                    "--ignored",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("OD39_MODE", mode)
                .env("OD39_ROOT", root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap();
            let mut guard = Self {
                child,
                input: None,
                lines,
                reader: None,
            };
            guard.input = guard.child.stdin.take();
            let mut output = guard.child.stdout.take().unwrap();
            guard.reader = Some(std::thread::spawn(move || {
                for _ in 0..128 {
                    let line = bounded_line(&mut output);
                    let terminal = !matches!(line, Ok(Some(_)));
                    if sender.try_send(line).is_err() || terminal {
                        return;
                    }
                }
                let _ = sender.try_send(Err("helper line budget exhausted".into()));
            }));
            guard
        }

        fn next(&self) -> Option<String> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                let line = self
                    .lines
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .expect("helper deadline/disconnection")
                    .expect("bounded helper output");
                let line = line?;
                // libtest may print its test-name prefix before helper output.
                if let Some((_, message)) = line.split_once("OD39 ") {
                    return Some(message.to_string());
                }
            }
        }

        fn send(&mut self, command: &str) {
            let input = self.input.as_mut().unwrap();
            writeln!(input, "{command}").unwrap();
            input.flush().unwrap();
        }

        fn finish(mut self) {
            self.send("EXIT");
            assert_eq!(self.next(), None);
            assert!(self.child.wait().unwrap().success());
        }
    }

    impl Drop for LockChild {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }

    fn child_emit(message: &str) {
        let mut output = std::io::stdout().lock();
        writeln!(output, "OD39 {message}").unwrap();
        output.flush().unwrap();
    }

    fn child_command() -> String {
        bounded_line(&mut std::io::stdin().lock()).unwrap().unwrap()
    }

    #[test]
    #[ignore = "subprocess helper selected explicitly by process-lock tests"]
    fn lock_process_child() {
        use std::os::unix::fs::MetadataExt;
        let Some(mode) = std::env::var_os("OD39_MODE") else {
            return;
        };
        let root = PathBuf::from(std::env::var_os("OD39_ROOT").unwrap());
        let store = store_of(&root);
        if mode == "lock" {
            let held = store.lock().unwrap();
            // This is the actual held descriptor, unlike publication's path metadata.
            let metadata = held.metadata().unwrap();
            child_emit(&format!(
                "READY Lock {} {} {}",
                std::process::id(),
                metadata.dev(),
                metadata.ino()
            ));
            assert_eq!(child_command(), "RELEASE");
            drop(held);
            child_emit("RELEASED");
        } else {
            assert_eq!(mode, "publish");
            let generation = build(&root, &IndexConfig::default()).unwrap();
            let outcome = store.publish_inner(&generation, |phase, _| {
                if matches!(
                    phase,
                    PublishPhase::CurrentRename
                        | PublishPhase::CurrentSync
                        | PublishPhase::HistoryRead
                        | PublishPhase::HistorySync
                        | PublishPhase::PruneInventory
                        | PublishPhase::PruneRemoveFile
                        | PublishPhase::PruneRemoveDir
                        | PublishPhase::PruneSync
                ) {
                    let metadata = fs::metadata(store.dir.join("index.lock"))?;
                    child_emit(&format!(
                        "READY {phase:?} {} {} {}",
                        std::process::id(),
                        metadata.dev(),
                        metadata.ino()
                    ));
                    match child_command().as_str() {
                        "CONTINUE" => (),
                        "FAIL" => return Err(std::io::Error::other("process phase failure")),
                        _ => panic!("unexpected helper command"),
                    }
                }
                Ok(())
            });
            child_emit(&format!(
                "DONE {}",
                match outcome {
                    Ok(outcome) => serde_json::to_string(&outcome).unwrap(),
                    Err(_) => "error".into(),
                }
            ));
        }
        assert_eq!(child_command(), "EXIT");
    }

    fn assert_child_holds(child: &LockChild, message: &str, store: &Store) -> String {
        use std::os::unix::fs::MetadataExt;
        let fields: Vec<_> = message.split_whitespace().collect();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "READY");
        assert_eq!(fields[2].parse::<u32>().unwrap(), child.child.id());
        assert_ne!(child.child.id(), std::process::id());
        let metadata = fs::metadata(store.dir.join("index.lock")).unwrap();
        assert_eq!(fields[3].parse::<u64>().unwrap(), metadata.dev());
        assert_eq!(fields[4].parse::<u64>().unwrap(), metadata.ino());
        assert!(matches!(store.lock(), Err(StoreError::Locked)));
        fields[1].to_string()
    }

    #[test]
    fn process_lock_owner_release_death_and_store_independence() {
        use std::os::unix::fs::MetadataExt;
        for owner_death in [false, true] {
            let root = corpus();
            let other = tempfile::tempdir().unwrap();
            let store = store_of(root.path());
            fs::create_dir(&store.dir).unwrap();
            let mut child = LockChild::spawn("lock", root.path());
            assert_eq!(
                assert_child_holds(&child, &child.next().unwrap(), &store),
                "Lock"
            );
            let before = fs::metadata(store.dir.join("index.lock")).unwrap();
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            assert_eq!(
                store_of(other.path())
                    .publish(&generation)
                    .unwrap()
                    .maintenance,
                Maintenance::Complete
            );
            if owner_death {
                child.child.kill().unwrap();
                assert!(!child.child.wait().unwrap().success());
            } else {
                child.send("RELEASE");
                assert_eq!(child.next().as_deref(), Some("RELEASED"));
            }
            let after = fs::metadata(store.dir.join("index.lock")).unwrap();
            assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
            // Reacquire without unlinking; normal-release child is still alive.
            drop(store.lock().unwrap());
            if !owner_death {
                child.finish();
            }
        }
    }

    #[test]
    fn process_publication_holds_lock_through_commit_and_maintenance() {
        for fail in [
            None,
            Some("CurrentRename"),
            Some("CurrentSync"),
            Some("HistoryRead"),
            Some("PruneSync"),
        ] {
            let root = corpus();
            let other = tempfile::tempdir().unwrap();
            let store = store_of(root.path());
            for i in 0..3 {
                fs::write(root.path().join("n.md"), format!("# {i}\n")).unwrap();
                store
                    .publish(&build(root.path(), &IndexConfig::default()).unwrap())
                    .unwrap();
            }
            let previous = store.current_id().unwrap();
            fs::write(root.path().join("n.md"), "# fourth\n").unwrap();
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            let mut child = LockChild::spawn("publish", root.path());
            let mut visited = Vec::new();
            let done = loop {
                let message = child.next().unwrap();
                if let Some(result) = message.strip_prefix("DONE ") {
                    break result.to_string();
                }
                let phase = assert_child_holds(&child, &message, &store);
                if phase != "CurrentRename" {
                    assert_eq!(store.current_id().unwrap(), generation.generation_id);
                }
                assert_eq!(
                    store_of(other.path())
                        .publish(&generation)
                        .unwrap()
                        .maintenance,
                    Maintenance::Complete
                );
                child.send(if fail == Some(phase.as_str()) {
                    "FAIL"
                } else {
                    "CONTINUE"
                });
                visited.push(phase);
            };
            drop(store.lock().unwrap()); // Child remains alive awaiting EXIT.
            if fail == Some("CurrentRename") {
                assert_eq!(done, "error");
                assert_eq!(store.current_id().unwrap(), previous);
            } else {
                let outcome: serde_json::Value = serde_json::from_str(&done).unwrap();
                assert_eq!(outcome["generation_id"], generation.generation_id);
                assert_eq!(store.load().unwrap(), generation);
                match fail {
                    Some("CurrentSync") => {
                        assert_eq!(outcome["durability"], "uncertain");
                        assert_eq!(outcome["maintenance"], "not_attempted");
                    }
                    Some("HistoryRead") => {
                        assert_eq!(outcome["durability"], "synced");
                        assert_eq!(outcome["maintenance"], "failed");
                        assert_eq!(outcome["failure_phase"], "history_read");
                    }
                    Some("PruneSync") => {
                        assert_eq!(outcome["durability"], "synced");
                        assert_eq!(outcome["maintenance"], "failed");
                        assert_eq!(outcome["failure_phase"], "prune_sync");
                    }
                    Some(_) => panic!("unexpected failure phase"),
                    None => {
                        assert_eq!(outcome["maintenance"], "complete");
                        assert_eq!(generation_dirs(&store).len(), 3);
                        for phase in [
                            "CurrentRename",
                            "CurrentSync",
                            "HistoryRead",
                            "HistorySync",
                            "PruneInventory",
                            "PruneRemoveFile",
                            "PruneRemoveDir",
                            "PruneSync",
                        ] {
                            assert!(visited.iter().any(|seen| seen == phase), "{phase}");
                        }
                    }
                }
            }
            if let Some(phase) = fail {
                assert!(visited.iter().any(|seen| seen == phase));
            }
            child.finish();
        }
    }

    fn fail_at(phase: PublishPhase) -> impl FnMut(PublishPhase, &Path) -> std::io::Result<()> {
        move |at, _| {
            if at == phase {
                Err(std::io::Error::other("injected publication failure"))
            } else {
                Ok(())
            }
        }
    }

    fn cleanup_budget(
        probes: usize,
    ) -> Publication<impl FnMut(PublishPhase, &Path) -> std::io::Result<()>> {
        Publication {
            hook: |_, _: &Path| Ok(()),
            phase: PublishPhase::TempInventory,
            probes,
        }
    }

    #[test]
    fn precommit_phase_failures_preserve_active_generation() {
        for phase in [
            PublishPhase::NamespaceSync,
            PublishPhase::TempInventory,
            PublishPhase::CandidateCreate,
            PublishPhase::IndexCreate,
            PublishPhase::IndexWrite,
            PublishPhase::IndexSync,
            PublishPhase::MetaCreate,
            PublishPhase::MetaWrite,
            PublishPhase::MetaSync,
            PublishPhase::CandidateSync,
            PublishPhase::CandidateRename,
            PublishPhase::GenerationsSync,
            PublishPhase::CurrentCreate,
            PublishPhase::CurrentWrite,
            PublishPhase::CurrentFileSync,
            PublishPhase::CurrentRename,
        ] {
            let root = corpus();
            let store = store_of(root.path());
            let first = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&first).unwrap();
            fs::write(root.path().join("c.md"), "# C\n").unwrap();
            let second = build(root.path(), &IndexConfig::default()).unwrap();
            assert!(
                store.publish_inner(&second, fail_at(phase)).is_err(),
                "{phase:?}"
            );
            assert_eq!(
                store.current_id().unwrap(),
                first.generation_id,
                "{phase:?}"
            );
            assert_eq!(store.load().unwrap(), first, "{phase:?}");
        }
    }

    #[test]
    fn namespace_creation_and_reuse_sync_precede_current_rename() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        for reused in [false, true] {
            let mut phases = Vec::new();
            let outcome = store
                .publish_inner(&generation, |phase, _| {
                    phases.push(phase);
                    Ok(())
                })
                .unwrap();
            assert_eq!(outcome.maintenance, Maintenance::Complete);
            let commit = phases
                .iter()
                .position(|phase| *phase == PublishPhase::CurrentRename)
                .unwrap();
            for phase in [
                PublishPhase::NamespaceSync,
                PublishPhase::IndexSync,
                PublishPhase::MetaSync,
                PublishPhase::CandidateSync,
                PublishPhase::GenerationsSync,
            ] {
                assert!(phases.iter().position(|at| *at == phase).unwrap() < commit);
            }
            assert_eq!(phases.contains(&PublishPhase::CandidateCreate), !reused);
        }
        // The first namespace sync failure happens before any CURRENT exists.
        let fresh = Store::new(root.path().join("fresh/.ontodoc"));
        assert!(
            fresh
                .publish_inner(&generation, fail_at(PublishPhase::NamespaceSync))
                .is_err()
        );
        assert!(matches!(fresh.current_id(), Err(StoreError::NoGeneration)));
        assert!(
            fresh
                .publish_inner(&generation, fail_at(PublishPhase::NamespaceCreate))
                .is_err()
        );
    }

    #[test]
    fn namespace_parent_sync_failures_after_creation_are_retried() {
        for fail_generations in [false, true] {
            let root = corpus();
            let store = store_of(root.path());
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            let created = if fail_generations {
                store.dir.join("generations")
            } else {
                store.dir.clone()
            };
            let parent = created.parent().unwrap();
            let mut armed = false;
            assert!(
                store
                    .publish_inner(&generation, |phase, path| {
                        if phase == PublishPhase::NamespaceCreate && path == created {
                            armed = true;
                        }
                        if armed && phase == PublishPhase::NamespaceSync && path == parent {
                            return Err(std::io::Error::other("new namespace parent sync"));
                        }
                        Ok(())
                    })
                    .is_err()
            );
            assert!(armed && created.is_dir());
            assert!(matches!(store.current_id(), Err(StoreError::NoGeneration)));
            let mut retried = false;
            let mut committed = false;
            store
                .publish_inner(&generation, |phase, path| {
                    if phase == PublishPhase::NamespaceSync && path == parent {
                        retried = true;
                    }
                    if phase == PublishPhase::CurrentRename {
                        assert!(retried);
                        committed = true;
                    }
                    Ok(())
                })
                .unwrap();
            assert!(retried && committed);
            assert_eq!(store.load().unwrap(), generation);
        }
    }

    #[test]
    fn namespace_accepts_trusted_ancestor_alias_but_refuses_owned_symlinks() {
        use std::os::unix::fs::symlink;
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let alias = root.path().join("alias");
        let real = root.path().join("real");
        fs::create_dir(&real).unwrap();
        symlink(&real, &alias).unwrap();
        let store = store_of(&alias);
        assert_eq!(
            store.publish(&generation).unwrap().maintenance,
            Maintenance::Complete
        );
        assert_eq!(store.load().unwrap(), generation);
        let redirect = root.path().join("redirect");
        symlink(&store.dir, &redirect).unwrap();
        assert!(Store::new(redirect).publish(&generation).is_err());
        let unsafe_store = store_of(&root.path().join("unsafe"));
        fs::create_dir_all(&unsafe_store.dir).unwrap();
        symlink(
            store.dir.join("generations"),
            unsafe_store.dir.join("generations"),
        )
        .unwrap();
        assert!(unsafe_store.publish(&generation).is_err());
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn reused_candidate_sync_failures_preserve_all_published_bytes() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let target = store
            .dir
            .join("generations")
            .join(&generation.generation_id);
        let paths = [
            store.dir.join("CURRENT"),
            store.dir.join("HISTORY"),
            target.join("index.json"),
            target.join("meta.json"),
        ];
        let before = paths.each_ref().map(|path| fs::read(path).unwrap());
        for phase in [
            PublishPhase::IndexSync,
            PublishPhase::MetaSync,
            PublishPhase::CandidateSync,
            PublishPhase::GenerationsSync,
        ] {
            assert!(store.publish_inner(&generation, fail_at(phase)).is_err());
            assert_eq!(
                paths.each_ref().map(|path| fs::read(path).unwrap()),
                before,
                "{phase:?}"
            );
        }
    }

    #[test]
    fn committed_phase_failures_report_exact_outcome_and_keep_current() {
        for phase in [
            PublishPhase::CurrentSync,
            PublishPhase::HistoryRead,
            PublishPhase::HistoryCreate,
            PublishPhase::HistoryWrite,
            PublishPhase::HistoryFileSync,
            PublishPhase::HistoryRename,
            PublishPhase::HistorySync,
            PublishPhase::PruneInventory,
            PublishPhase::PruneRemoveFile,
            PublishPhase::PruneRemoveDir,
            PublishPhase::PruneSync,
        ] {
            let root = corpus();
            let store = store_of(root.path());
            for i in 0..3 {
                fs::write(root.path().join("n.md"), format!("# {i}\n")).unwrap();
                store
                    .publish(&build(root.path(), &IndexConfig::default()).unwrap())
                    .unwrap();
            }
            fs::write(root.path().join("n.md"), "# final\n").unwrap();
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            let history = fs::read(store.dir.join("HISTORY")).unwrap();
            let mut phases = Vec::new();
            let outcome = store
                .publish_inner(&generation, |at, _| {
                    phases.push(at);
                    if at == phase {
                        Err(std::io::Error::other("injected"))
                    } else {
                        Ok(())
                    }
                })
                .unwrap();
            assert_eq!(outcome.failure_phase, Some(phase));
            assert_eq!(outcome.generation_id, generation.generation_id);
            assert_eq!(store.load().unwrap(), generation);
            if phase == PublishPhase::CurrentSync {
                assert_eq!(outcome.durability, Durability::Uncertain);
                assert_eq!(outcome.maintenance, Maintenance::NotAttempted);
                assert_eq!(phases.last(), Some(&PublishPhase::CurrentSync));
                assert_eq!(fs::read(store.dir.join("HISTORY")).unwrap(), history);
            } else {
                assert_eq!(outcome.durability, Durability::Synced);
                assert_eq!(outcome.maintenance, Maintenance::Failed);
            }
        }
    }

    #[test]
    fn history_is_canonical_bounded_and_never_authorizes_current_deletion() {
        let ids = ["a".repeat(64), "b".repeat(64), "c".repeat(64)];
        let exact = format!("{}\n", ids.join("\n"));
        assert_eq!(exact.len(), 195);
        assert_eq!(parse_history(exact.as_bytes()).unwrap(), ids);
        assert!(parse_history(format!("{exact}\n").as_bytes()).is_err());
        assert!(parse_history(b"").unwrap().is_empty());
        for malformed in [
            "\n".into(),
            ids[0].clone(),
            format!("{}\r\n", ids[0]),
            format!("{0}\n{0}\n", ids[0]),
            format!("{}\n", "A".repeat(64)),
        ] {
            assert!(parse_history(malformed.as_bytes()).is_err());
        }
        for unsafe_history in [false, true] {
            let root = corpus();
            let store = store_of(root.path());
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&generation).unwrap();
            let history = store.dir.join("HISTORY");
            fs::remove_file(&history).unwrap();
            if unsafe_history {
                std::os::unix::fs::symlink(root.path().join("a.md"), &history).unwrap();
            } else {
                fs::write(&history, b"bad\n").unwrap();
            }
            let outcome = store.publish(&generation).unwrap();
            assert_eq!(outcome.failure_phase, Some(PublishPhase::HistoryRead));
            assert_eq!(outcome.maintenance, Maintenance::Failed);
            assert_eq!(store.load().unwrap(), generation);
        }
    }

    #[test]
    fn cleanup_validates_entire_inventory_before_deleting() {
        for kind in ["unknown", "nested", "symlink", "bad-name"] {
            let root = corpus();
            let store = store_of(root.path());
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&generation).unwrap();
            let valid = store.dir.join(format!("tmp-{}", "a".repeat(64)));
            fs::create_dir(&valid).unwrap();
            fs::write(valid.join("index.json"), "partial").unwrap();
            let invalid = store.dir.join(if kind == "bad-name" {
                "tmp-invalid".into()
            } else {
                format!("tmp-{}", "f".repeat(64))
            });
            fs::create_dir(&invalid).unwrap();
            match kind {
                "unknown" => fs::write(invalid.join("extra"), "").unwrap(),
                "nested" => fs::create_dir(invalid.join("index.json")).unwrap(),
                "symlink" => {
                    std::os::unix::fs::symlink(root.path().join("a.md"), invalid.join("index.json"))
                        .unwrap()
                }
                _ => (),
            }
            assert!(store.publish(&generation).is_err());
            assert_eq!(fs::read(valid.join("index.json")).unwrap(), b"partial");
            assert!(invalid.is_dir());
            assert_eq!(store.load().unwrap(), generation);
        }
    }

    #[test]
    fn incomplete_temp_subsets_are_cleaned_with_shared_probe_budget() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        for (i, files) in [
            vec![],
            vec!["index.json"],
            vec!["meta.json"],
            vec!["index.json", "meta.json"],
        ]
        .iter()
        .enumerate()
        {
            let path = store.dir.join(format!("tmp-{}", i.to_string().repeat(64)));
            fs::create_dir(&path).unwrap();
            for file in files {
                fs::write(path.join(file), "partial").unwrap();
            }
        }
        let mut budget = cleanup_budget(0);
        store.remove_temp_dirs(&mut budget).unwrap();
        let used = budget.probes;
        assert!(used > 4);
        let empty = tempfile::tempdir().unwrap();
        // Linux exposes '.' and '..', plus the final EOF probe. Every probe,
        // including EOF, must fit; the same counter spans subsequent scans.
        let mut exact = cleanup_budget(MAX_CLEANUP_PROBES - 3);
        assert!(
            directory_names(empty.path(), &mut exact)
                .unwrap()
                .is_empty()
        );
        assert_eq!(exact.probes, MAX_CLEANUP_PROBES);
        assert!(directory_names(empty.path(), &mut exact).is_err());
        let mut short = cleanup_budget(MAX_CLEANUP_PROBES - 2);
        assert!(directory_names(empty.path(), &mut short).is_err());
        assert_eq!(short.probes, MAX_CLEANUP_PROBES);
    }

    #[test]
    fn cleanup_and_repair_phase_failures_preserve_active_data() {
        for phase in [
            PublishPhase::TempRemoveFile,
            PublishPhase::TempRemoveDir,
            PublishPhase::TempSync,
            PublishPhase::RepairInventory,
            PublishPhase::RepairRemoveFile,
            PublishPhase::RepairRemoveDir,
            PublishPhase::RepairSync,
        ] {
            let root = corpus();
            let store = store_of(root.path());
            let inactive = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&inactive).unwrap();
            fs::write(root.path().join("c.md"), "# C\n").unwrap();
            let active = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&active).unwrap();
            let tmp = store.dir.join(format!("tmp-{}", "a".repeat(64)));
            fs::create_dir(&tmp).unwrap();
            fs::write(tmp.join("index.json"), "partial").unwrap();
            fs::write(
                store
                    .dir
                    .join("generations")
                    .join(&inactive.generation_id)
                    .join("meta.json"),
                b"{",
            )
            .unwrap();
            assert!(
                store.publish_inner(&inactive, fail_at(phase)).is_err(),
                "{phase:?}"
            );
            assert_eq!(store.load().unwrap(), active, "{phase:?}");
        }
    }

    #[test]
    fn invalid_generation_inventory_prevents_all_pruning() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let stale = store.dir.join("generations").join("0".repeat(64));
        fs::create_dir(&stale).unwrap();
        for name in ["index.json", "meta.json"] {
            fs::write(stale.join(name), "stale").unwrap();
        }
        let bad = store.dir.join("generations").join("f".repeat(64));
        fs::create_dir(&bad).unwrap();
        fs::write(bad.join("unknown"), "keep").unwrap();
        let outcome = store.publish(&generation).unwrap();
        assert_eq!(outcome.failure_phase, Some(PublishPhase::PruneInventory));
        assert_eq!(fs::read(stale.join("index.json")).unwrap(), b"stale");
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn enumeration_error_probe_is_precharged_and_cannot_delete() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let tmp = store.dir.join(format!("tmp-{}", "a".repeat(64)));
        fs::create_dir(&tmp).unwrap();
        let mut calls = 0;
        let mut publication = Publication {
            hook: |_, _: &Path| {
                calls += 1;
                if calls == 3 {
                    Err(std::io::Error::other("enumeration"))
                } else {
                    Ok(())
                }
            },
            phase: PublishPhase::TempInventory,
            probes: 0,
        };
        assert!(directory_names(&store.dir, &mut publication).is_err());
        assert_eq!(publication.probes, 3);
        assert!(tmp.is_dir());
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn history_file_byte_cap_and_missing_history_are_explicit() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let history = store.dir.join("HISTORY");
        fs::remove_file(&history).unwrap();
        assert_eq!(
            store.publish(&generation).unwrap().maintenance,
            Maintenance::Complete
        );
        let exact = format!(
            "{}\n{}\n{}\n",
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64)
        );
        fs::write(&history, &exact).unwrap();
        assert_eq!(
            store.publish(&generation).unwrap().maintenance,
            Maintenance::Complete
        );
        fs::write(&history, format!("{exact}x")).unwrap();
        let outcome = store.publish(&generation).unwrap();
        assert_eq!(outcome.failure_phase, Some(PublishPhase::HistoryRead));
        assert_eq!(outcome.maintenance, Maintenance::Failed);
        assert_eq!(fs::metadata(&history).unwrap().len(), 196);
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn cleanup_budget_refusal_does_not_delete_validated_prefix() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let tmp = store.dir.join(format!("tmp-{}", "a".repeat(64)));
        fs::create_dir(&tmp).unwrap();
        fs::write(tmp.join("index.json"), "partial").unwrap();
        let mut measure = cleanup_budget(0);
        let _ = directory_names(&store.dir, &mut measure).unwrap();
        let mut limited = cleanup_budget(MAX_CLEANUP_PROBES - measure.probes - 2);
        assert!(matches!(
            store.remove_temp_dirs(&mut limited),
            Err(StoreError::CapExceeded { .. })
        ));
        assert_eq!(fs::read(tmp.join("index.json")).unwrap(), b"partial");
        assert_eq!(store.load().unwrap(), generation);
    }

    fn generation_dirs(store: &Store) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(store.dir.join("generations"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn publish_and_load_roundtrip() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        assert_eq!(generation.files.len(), 2);
        assert_eq!(generation.chunks.len(), 3);
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        assert_eq!(store.load().unwrap(), generation);
        let chunk = &generation.chunks[0];
        assert_eq!(
            generation.chunk_text(chunk),
            Some("# A\n\nAlpha text. See [b](b.md).")
        );
    }

    #[test]
    fn streaming_freshness_matches_sorted_collecting_manifest_and_current_policy() {
        let root = corpus();
        fs::create_dir(root.path().join("a")).unwrap();
        fs::write(root.path().join("a/x.md"), "# X\n\ntext\n").unwrap();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let check = || {
            let collected = corpus::walk(root.path(), &WalkConfig::default()).unwrap();
            let manifest: Vec<_> = collected
                .files
                .into_iter()
                .map(|file| FileEntry {
                    path: file.rel_path,
                    size: file.size,
                    sha256: file.sha256,
                })
                .collect();
            let expected = compare_freshness(&generation, &manifest);
            assert_eq!(freshness(root.path(), &generation).unwrap(), expected);
            assert_eq!(
                freshness_bounded(
                    root.path(),
                    &generation,
                    &mut corpus::WalkBudget::new(10 * 1024 * 1024, 1000)
                )
                .unwrap(),
                expected
            );
        };
        check();
        // Same-size bytes, changed policy, and new invalid/oversized candidates.
        let original = fs::read(root.path().join("a.md")).unwrap();
        fs::write(
            root.path().join("a.md"),
            original
                .iter()
                .map(|b| if *b == b'A' { b'Z' } else { *b })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        fs::write(root.path().join("new.md"), "new\n").unwrap();
        fs::write(root.path().join("bad.md"), [0xff]).unwrap();
        fs::write(
            root.path().join("big.md"),
            vec![b'x'; WalkConfig::default().max_file_bytes as usize + 1],
        )
        .unwrap();
        fs::write(root.path().join(".ontodocignore"), "b.md\n").unwrap();
        check();
        assert_eq!(
            freshness(root.path(), &generation).unwrap(),
            Freshness {
                added: vec!["new.md".into()],
                removed: vec!["b.md".into()],
                modified: vec!["a.md".into()],
            }
        );
        assert!(matches!(
            freshness_bounded(
                root.path(),
                &generation,
                &mut corpus::WalkBudget::new(0, 1000)
            ),
            Err(StoreError::Corpus(CorpusError::CaptureBudget("byte")))
        ));
    }

    #[test]
    fn raw_collection_preflight_checks_exact_caps_duplicates_and_excess_before_contents() {
        let config = IndexConfig {
            max_files: 2,
            max_chunks: 2,
            ..IndexConfig::default()
        };
        for (key, what) in [
            ("files", "file count"),
            ("documents", "document count"),
            ("sections", "section count"),
            ("chunks", "chunk count"),
        ] {
            for raw in [
                format!("{{\"{key}\":[null,null]}}"),
                format!("{{\"{key}\":[null],\"{key}\":[null]}}"),
                format!("{{\"{key}\":[]}}"),
            ] {
                preflight_collections("test", raw.as_bytes(), &config).unwrap();
            }
            for raw in [
                format!("{{\"{key}\":[null,null,null]}}"),
                format!("{{\"{key}\":[null],\"{key}\":[null,null]}}"),
                format!("{{\"{key}\":[null,null,?]}}"),
            ] {
                assert!(
                    matches!(preflight_collections("test",raw.as_bytes(),&config),Err(StoreError::CapExceeded{what:found,cap:2}) if found==what),
                    "{raw}"
                );
            }
            let zero = IndexConfig {
                max_files: 0,
                max_chunks: 0,
                ..IndexConfig::default()
            };
            assert!(matches!(
                preflight_collections("test", format!("{{\"{key}\":[null]}}").as_bytes(), &zero),
                Err(StoreError::CapExceeded { cap: 0, .. })
            ));
        }
        // Limits depend on the separate config pass, not map key order.
        let config_json = serde_json::to_string(&IndexConfig {
            max_files: 1,
            ..config
        })
        .unwrap();
        for raw in [
            format!("{{\"config\":{config_json},\"files\":[null,null]}}"),
            format!("{{\"files\":[null,null],\"config\":{config_json}}}"),
        ] {
            assert!(matches!(
                decode_verified_generation(&"0".repeat(64), raw.as_bytes()),
                Err(StoreError::CapExceeded {
                    what: "file count",
                    cap: 1
                })
            ));
        }
    }

    #[test]
    fn all_load_adapters_apply_preflight_before_typed_records() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        let reader = corpus::RootedCorpus::open(root.path()).unwrap();
        let id = &generation.generation_id;
        let mut limited = generation.clone();
        limited.config.max_files = 1;
        // Struct serialization keeps files before documents, so this checks
        // the first encountered collection's typed refusal in every adapter.
        let bytes = serde_json::to_vec(&limited).unwrap();
        replace_artifact(&store, id, &bytes);
        let meta = fs::read(store.dir.join("generations").join(id).join("meta.json")).unwrap();
        for result in [
            store.load(),
            store.load_id_rooted(&reader, id),
            decode_captured_generation(id, &meta, &bytes),
        ] {
            assert!(
                matches!(
                    result,
                    Err(StoreError::CapExceeded {
                        what: "file count",
                        cap: 1
                    })
                ),
                "{result:?}"
            );
        }
        // Duplicate valid collection keys under the limit still reach typed
        // serde's duplicate-field refusal; preflight never silently merges data.
        let raw = serde_json::to_string(&generation).unwrap();
        let raw = raw.replacen("\"files\":", "\"files\":[],\"files\":", 1);
        assert!(
            matches!(decode_verified_generation(id,raw.as_bytes()),Err(StoreError::Corrupt{reason,..}) if reason.contains("duplicate field"))
        );
    }

    #[test]
    fn rooted_store_adapters_preserve_caps_and_refuse_redirected_ancestors() {
        use std::os::unix::fs::symlink;
        let root = corpus();
        let reader = corpus::RootedCorpus::open(root.path()).unwrap();
        let store = store_of(root.path());
        assert!(matches!(
            store.current_id_rooted(&reader),
            Err(StoreError::NoGeneration)
        ));
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let id = &generation.generation_id;
        assert_eq!(store.load_id_rooted(&reader, id).unwrap(), generation);
        let pointer = store.dir.join("CURRENT");
        fs::write(&pointer, "x".repeat(MAX_POINTER_BYTES + 1)).unwrap();
        assert!(matches!(
            store.current_id_rooted(&reader),
            Err(StoreError::CapExceeded {
                what: "CURRENT bytes",
                ..
            })
        ));
        fs::remove_file(&pointer).unwrap();
        symlink("HISTORY", &pointer).unwrap();
        assert!(matches!(
            store.current_id_rooted(&reader),
            Err(StoreError::Corpus(CorpusError::Symlink))
        ));
        fs::remove_file(&pointer).unwrap();
        fs::write(&pointer, id).unwrap();
        let dir = store.dir.join("generations").join(id);
        let meta_path = dir.join("meta.json");
        let meta = fs::read(&meta_path).unwrap();
        fs::write(&meta_path, vec![b' '; MAX_META_BYTES + 1]).unwrap();
        assert!(matches!(
            store.load_id_rooted(&reader, id),
            Err(StoreError::CapExceeded {
                what: "metadata bytes",
                ..
            })
        ));
        fs::write(&meta_path, meta).unwrap();
        let artifact_path = dir.join("index.json");
        let artifact = fs::read(&artifact_path).unwrap();
        let mut oversized = artifact.clone();
        oversized.push(b' ');
        fs::write(&artifact_path, oversized).unwrap();
        assert!(matches!(
            store.load_id_rooted(&reader, id),
            Err(StoreError::CapExceeded {
                what: "artifact bytes",
                ..
            })
        ));
        fs::write(&artifact_path, artifact).unwrap();
        let saved = root.path().join("saved-store");
        fs::rename(&store.dir, &saved).unwrap();
        symlink(&saved, &store.dir).unwrap();
        assert!(matches!(
            store.current_id_rooted(&reader),
            Err(StoreError::Corpus(CorpusError::Symlink))
        ));
        assert!(matches!(
            store.load_id_rooted(&reader, id),
            Err(StoreError::Corpus(CorpusError::Symlink))
        ));
    }

    #[test]
    fn pointer_is_bounded_utf8_and_canonical_before_any_path_join() {
        let root = corpus();
        let store = store_of(root.path());
        assert!(matches!(store.current_id(), Err(StoreError::NoGeneration)));
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let pointer = store.dir.join("CURRENT");
        for bad in [
            "",
            "../escape",
            "/tmp/escape",
            &"A".repeat(64),
            &"a".repeat(63),
        ] {
            fs::write(&pointer, bad).unwrap();
            assert!(matches!(store.load(), Err(StoreError::Corrupt { .. })));
        }
        fs::write(&pointer, [0xff]).unwrap();
        assert!(matches!(
            store.current_id(),
            Err(StoreError::Corrupt { .. })
        ));
        let padded = format!("{}{}", " ".repeat(64), generation.generation_id);
        assert_eq!(padded.len(), MAX_POINTER_BYTES);
        fs::write(&pointer, &padded).unwrap();
        assert_eq!(store.load().unwrap(), generation);
        fs::write(&pointer, format!(" {padded}")).unwrap();
        assert!(matches!(
            store.current_id(),
            Err(StoreError::CapExceeded {
                what: "CURRENT bytes",
                ..
            })
        ));
    }

    #[test]
    fn actual_reads_enforce_cap_after_metadata_and_stop_at_one_extra_byte() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("growing");
        fs::write(&path, b"1234").unwrap();
        assert_eq!(read_store_file(&path, 4, "test").unwrap(), b"1234");
        let opened = File::open(&path).unwrap();
        assert_eq!(opened.metadata().unwrap().len(), 4);
        // Deterministic growth after the descriptor's metadata check.
        fs::write(&path, b"12345").unwrap();
        assert!(matches!(
            read_capped(opened, &path, 4, "test"),
            Err(StoreError::CapExceeded { cap: 4, .. })
        ));
        let mut endless = std::io::repeat(0);
        assert!(matches!(
            read_capped(&mut endless, &path, 4, "test"),
            Err(StoreError::CapExceeded { .. })
        ));
        assert!(matches!(
            read_store_file(&path, 4, "test"),
            Err(StoreError::CapExceeded { .. })
        ));
    }

    #[test]
    fn store_inputs_refuse_leaf_symlinks_directories_and_fifos() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let regular = root.path().join("regular");
        fs::write(&regular, b"ok").unwrap();
        let link = root.path().join("link");
        symlink(&regular, &link).unwrap();
        let fifo = root.path().join("fifo");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        for path in [&link, &fifo, &root.path().to_path_buf()] {
            assert!(matches!(
                read_store_file(path, 10, "test"),
                Err(StoreError::Io { .. })
            ));
        }
    }

    #[test]
    fn artifact_serialization_and_decoding_enforce_configured_and_global_limits() {
        let root = corpus();
        let mut generation = build(root.path(), &IndexConfig::default()).unwrap();
        // Find the fixed point because the decimal cap itself is serialized.
        loop {
            let len = serde_json::to_vec(&generation).unwrap().len();
            if generation.config.max_artifact_bytes == len {
                break;
            }
            generation.config.max_artifact_bytes = len;
            generation.generation_id =
                generation_id(&generation.config, &generation.files, &generation.skipped).unwrap();
        }
        let bytes = serde_json::to_vec(&generation).unwrap();
        assert_eq!(serialize_artifact(&generation).unwrap(), bytes);
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        assert_eq!(store.load().unwrap(), generation);
        generation.config.max_artifact_bytes -= 1;
        generation.generation_id =
            generation_id(&generation.config, &generation.files, &generation.skipped).unwrap();
        let oversized = serde_json::to_vec(&generation).unwrap();
        assert!(matches!(
            serialize_artifact(&generation),
            Err(StoreError::CapExceeded { .. })
        ));
        let id = &generation.generation_id;
        let meta = serde_json::to_vec(&Meta {
            generation_id: id.clone(),
            artifact_sha256: sha256_hex(&oversized),
            artifact_bytes: oversized.len(),
        })
        .unwrap();
        assert!(matches!(
            decode_captured_generation(id, &meta, &oversized),
            Err(StoreError::CapExceeded { .. })
        ));
        let declared = serde_json::to_vec(&Meta {
            generation_id: id.clone(),
            artifact_sha256: String::new(),
            artifact_bytes: MAX_ARTIFACT_BYTES + 1,
        })
        .unwrap();
        assert!(matches!(
            decode_captured_generation(id, &declared, b""),
            Err(StoreError::CapExceeded {
                cap: MAX_ARTIFACT_BYTES,
                ..
            })
        ));
        assert!(matches!(
            decode_captured_generation(id, &vec![b' '; MAX_META_BYTES + 1], b""),
            Err(StoreError::CapExceeded {
                cap: MAX_META_BYTES,
                ..
            })
        ));
        assert!(check_bytes(MAX_ARTIFACT_BYTES, MAX_ARTIFACT_BYTES, "test").is_ok());
        assert!(matches!(
            check_bytes(MAX_ARTIFACT_BYTES + 1, MAX_ARTIFACT_BYTES, "test"),
            Err(StoreError::CapExceeded { .. })
        ));
    }

    #[test]
    fn oversized_or_unsafe_target_refusal_preserves_publication_state() {
        use std::os::unix::fs::symlink;
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        let target = store
            .dir
            .join("generations")
            .join(&generation.generation_id);
        let meta_path = target.join("meta.json");
        let index_path = target.join("index.json");
        let meta = fs::read(&meta_path).unwrap();
        let index = fs::read(&index_path).unwrap();
        let pointer = fs::read(store.dir.join("CURRENT")).unwrap();
        let history = fs::read(store.dir.join("HISTORY")).unwrap();
        fs::write(&meta_path, vec![b' '; MAX_META_BYTES + 1]).unwrap();
        assert!(matches!(
            store.publish(&generation),
            Err(StoreError::CapExceeded { .. })
        ));
        assert_eq!(
            fs::metadata(&meta_path).unwrap().len(),
            (MAX_META_BYTES + 1) as u64
        );
        assert_eq!(fs::read(&index_path).unwrap(), index);
        fs::write(&meta_path, &meta).unwrap();
        // Sparse oversized artifact rejects without allocating its length.
        File::options()
            .write(true)
            .open(&index_path)
            .unwrap()
            .set_len(MAX_ARTIFACT_BYTES as u64 + 1)
            .unwrap();
        assert!(matches!(
            store.publish(&generation),
            Err(StoreError::CapExceeded { .. })
        ));
        assert_eq!(
            fs::metadata(&index_path).unwrap().len(),
            MAX_ARTIFACT_BYTES as u64 + 1
        );
        fs::remove_file(&index_path).unwrap();
        let outside = root.path().join("outside");
        fs::write(&outside, &index).unwrap();
        symlink(&outside, &index_path).unwrap();
        // Invalid metadata cannot bypass checking the other target input.
        fs::write(&meta_path, b"{").unwrap();
        assert!(matches!(
            store.publish(&generation),
            Err(StoreError::Io { .. })
        ));
        assert!(
            fs::symlink_metadata(&index_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&outside).unwrap(), index);
        assert_eq!(fs::read(&meta_path).unwrap(), b"{");
        assert_eq!(fs::read(store.dir.join("CURRENT")).unwrap(), pointer);
        assert_eq!(fs::read(store.dir.join("HISTORY")).unwrap(), history);
    }

    #[test]
    fn ordinary_inactive_corrupt_target_can_be_repaired() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        let target = store
            .dir
            .join("generations")
            .join(&generation.generation_id);
        fs::write(target.join("meta.json"), b"{").unwrap();
        assert!(store.publish(&generation).is_err());
        fs::write(root.path().join("c.md"), "# C\n").unwrap();
        let second = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&second).unwrap();
        store.publish(&generation).unwrap();
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn captured_decoder_uses_verified_bytes_without_paths() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let artifact = serde_json::to_vec(&generation).unwrap();
        let id = &generation.generation_id;
        let meta = serde_json::to_vec(&Meta {
            generation_id: id.clone(),
            artifact_sha256: sha256_hex(&artifact),
            artifact_bytes: artifact.len(),
        })
        .unwrap();
        assert_eq!(
            decode_captured_generation(id, &meta, &artifact).unwrap(),
            generation
        );
        assert!(decode_captured_generation(id, b"{}", &artifact).is_err());
        assert!(decode_captured_generation(id, &meta, b"{}").is_err());
        assert!(decode_captured_generation("wrong", &meta, &artifact).is_err());
    }

    #[test]
    fn generation_id_depends_only_on_content_and_index_config() {
        let root = corpus();
        let config = IndexConfig::default();
        let a = build(root.path(), &config).unwrap();
        let b = build(root.path(), &config).unwrap();
        assert_eq!(
            serde_json::to_vec(&a).unwrap(),
            serde_json::to_vec(&b).unwrap()
        );

        // Touching mtime without changing bytes keeps the id.
        let path = root.path().join("a.md");
        let text = fs::read(&path).unwrap();
        fs::write(&path, &text).unwrap();
        assert_eq!(
            build(root.path(), &config).unwrap().generation_id,
            a.generation_id
        );

        // An index-time change moves it. Query-time parameters (alpha, beta,
        // threshold) are not inputs of `generation_id` at all.
        let other = IndexConfig {
            chunk_budget: 800,
            ..config
        };
        assert_ne!(
            build(root.path(), &other).unwrap().generation_id,
            a.generation_id
        );
    }

    #[test]
    fn artifact_holds_only_relative_paths() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let json = String::from_utf8(serde_json::to_vec(&generation).unwrap()).unwrap();
        let abs = root.path().canonicalize().unwrap();
        assert!(!json.contains(abs.to_str().unwrap()));
        assert!(generation.files.iter().all(|f| !f.path.starts_with('/')));
    }

    #[test]
    fn held_lock_blocks_and_released_lock_is_reclaimed() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let store = store_of(root.path());
        fs::create_dir_all(&store.dir).unwrap();

        let holder = store.lock().unwrap();
        assert!(matches!(
            store.publish(&generation),
            Err(StoreError::Locked)
        ));
        drop(holder);

        // A lock file left behind (as by a crashed process) is not held.
        assert!(store.dir.join("index.lock").exists());
        store.publish(&generation).unwrap();
    }

    #[test]
    fn failure_before_pointer_swap_keeps_previous_generation() {
        let root = corpus();
        let store = store_of(root.path());
        let first = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&first).unwrap();

        fs::write(root.path().join("c.md"), "# C\n").unwrap();
        let second = build(root.path(), &IndexConfig::default()).unwrap();
        let err = store.publish_inner(&second, |phase, _| {
            if phase == PublishPhase::CurrentRename {
                Err(std::io::Error::other("crash"))
            } else {
                Ok(())
            }
        });
        assert!(err.is_err());
        assert_eq!(store.current_id().unwrap(), first.generation_id);
        assert_eq!(store.load().unwrap(), first);
    }

    #[test]
    fn corrupted_artifact_is_rejected() {
        let root = corpus();
        let store = store_of(root.path());
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&generation).unwrap();
        let index = store
            .dir
            .join("generations")
            .join(&generation.generation_id)
            .join("index.json");
        let mut bytes = fs::read(&index).unwrap();
        let at = bytes.iter().position(|&b| b == b'A').unwrap();
        bytes[at] = b'Z';
        fs::write(&index, bytes).unwrap();
        assert!(matches!(store.load(), Err(StoreError::Corrupt { .. })));
    }

    #[test]
    fn duplicate_ids_refuse_publication_without_mutating_the_store() {
        let root = corpus();
        let store = store_of(root.path());
        let first = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&first).unwrap();
        let current = fs::read(store.dir.join("CURRENT")).unwrap();
        let history = fs::read(store.dir.join("HISTORY")).unwrap();
        let dirs = generation_dirs(&store);
        let generation_dir = store.dir.join("generations").join(&first.generation_id);
        let artifact = fs::read(generation_dir.join("index.json")).unwrap();
        let meta = fs::read(generation_dir.join("meta.json")).unwrap();

        fs::write(root.path().join("c.md"), "# C\n\nNew text.\n").unwrap();
        let second = build(root.path(), &IndexConfig::default()).unwrap();
        for kind in ["section", "chunk"] {
            let mut malformed = second.clone();
            if kind == "section" {
                malformed.sections.push(malformed.sections[0].clone());
            } else {
                malformed.chunks.push(malformed.chunks[0].clone());
            }
            assert!(matches!(
                store.publish(&malformed),
                Err(StoreError::Corrupt { reason, .. })
                    if reason == format!("duplicate {kind} id")
            ));
            assert_eq!(fs::read(store.dir.join("CURRENT")).unwrap(), current);
            assert_eq!(fs::read(store.dir.join("HISTORY")).unwrap(), history);
            assert_eq!(generation_dirs(&store), dirs);
            assert_eq!(
                fs::read(generation_dir.join("index.json")).unwrap(),
                artifact
            );
            assert_eq!(fs::read(generation_dir.join("meta.json")).unwrap(), meta);
            assert_eq!(store.load().unwrap(), first);

            let empty_root = tempfile::tempdir().unwrap();
            let empty_store = store_of(empty_root.path());
            assert!(matches!(
                empty_store.publish(&malformed),
                Err(StoreError::Corrupt { .. })
            ));
            assert!(!empty_store.dir.exists());
        }
    }

    #[test]
    fn preamble_and_empty_heading_publish_distinct_identities() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.md"), "intro\n\n# !!!\n\nbody\n").unwrap();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        assert_eq!(generation.sections[0].id, "a.md#");
        assert_eq!(generation.sections[1].id, "a.md#@empty");
        assert_ne!(generation.chunks[0].id, generation.chunks[1].id);
        let store = store_of(root.path());
        store.publish(&generation).unwrap();
        assert_eq!(store.load().unwrap(), generation);
    }

    #[test]
    fn canonical_bytes_match_independent_roots_and_explicitly_different_mtimes() {
        use std::time::{Duration, UNIX_EPOCH};
        let roots = [corpus(), corpus()];
        let mut built = Vec::new();
        let mut artifacts = Vec::new();
        for (n, root) in roots.iter().enumerate() {
            let before = build(root.path(), &IndexConfig::default()).unwrap();
            for path in ["a.md", "b.md"] {
                let file = File::options()
                    .write(true)
                    .open(root.path().join(path))
                    .unwrap();
                file.set_modified(UNIX_EPOCH + Duration::from_secs(1000 + n as u64 * 1000))
                    .unwrap();
            }
            assert!(freshness(root.path(), &before).unwrap().is_fresh());
            fs::write(root.path().join("bad.md"), [0xff]).unwrap();
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            assert_eq!(before.files, generation.files);
            assert_ne!(before.generation_id, generation.generation_id);
            assert_eq!(
                generation.skipped,
                vec![SkipEntry {
                    path: "bad.md".into(),
                    reason: "not_utf8".into()
                }]
            );
            assert!(
                !serde_json::to_string(&generation)
                    .unwrap()
                    .contains("mtime_ns")
            );
            let store = store_of(root.path());
            store.publish(&generation).unwrap();
            let dir = store
                .dir
                .join("generations")
                .join(&generation.generation_id);
            artifacts.push(
                [
                    dir.join("index.json"),
                    dir.join("meta.json"),
                    store.dir.join("CURRENT"),
                    store.dir.join("HISTORY"),
                ]
                .map(|p| fs::read(p).unwrap()),
            );
            built.push(serde_json::to_vec(&generation).unwrap());
        }
        assert_ne!(
            fs::metadata(roots[0].path().join("a.md"))
                .unwrap()
                .modified()
                .unwrap(),
            fs::metadata(roots[1].path().join("a.md"))
                .unwrap()
                .modified()
                .unwrap()
        );
        assert_eq!(built[0], built[1]);
        assert_eq!(artifacts[0], artifacts[1]);

        let root = &roots[0];
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        let path = root.path().join("a.md");
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        let bytes = fs::read(&path).unwrap();
        let mut changed = bytes.clone();
        changed[0] = b'!';
        fs::write(&path, changed).unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), bytes.len() as u64);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), mtime);
        assert_eq!(
            freshness(root.path(), &generation).unwrap().modified,
            vec!["a.md"]
        );
    }

    #[test]
    fn canonical_skip_records_normalize_paths_reasons_order_and_identity() {
        use corpus::{Skip, SkipReason};
        let skip = |path: &str, reason| Skip {
            path: path.into(),
            reason,
        };
        let mut input = vec![
            skip("./b.md", SkipReason::NotUtf8),
            skip(
                "/private/one",
                SkipReason::Io {
                    message: "private OS text".into(),
                },
            ),
            skip("a/../bad.md", SkipReason::Changed),
            skip("a/./x.md", SkipReason::Symlink),
            skip("a\\b.md", SkipReason::NonUtf8Path),
            skip("large.md", SkipReason::TooLarge { size: 12, cap: 10 }),
            skip("./b.md", SkipReason::NotUtf8),
        ];
        let records = canonical_skips(&input);
        assert_eq!(
            records,
            vec![
                SkipEntry {
                    path: "".into(),
                    reason: "changed".into()
                },
                SkipEntry {
                    path: "".into(),
                    reason: "io".into()
                },
                SkipEntry {
                    path: "a/x.md".into(),
                    reason: "symlink".into()
                },
                SkipEntry {
                    path: "a\\b.md".into(),
                    reason: "non_utf8_path".into()
                },
                SkipEntry {
                    path: "b.md".into(),
                    reason: "not_utf8".into()
                },
                SkipEntry {
                    path: "b.md".into(),
                    reason: "not_utf8".into()
                },
                SkipEntry {
                    path: "large.md".into(),
                    reason: "too_large:size=12;cap=10".into()
                },
            ]
        );
        input.reverse();
        assert_eq!(canonical_skips(&input), records);
        input
            .iter_mut()
            .filter(|s| matches!(s.reason, SkipReason::Io { .. }))
            .for_each(|s| {
                s.path = "/different/root".into();
                s.reason = SkipReason::Io {
                    message: "different host error".into(),
                };
            });
        assert_eq!(canonical_skips(&input), records);
        let root = corpus();
        let mut g = build(root.path(), &IndexConfig::default()).unwrap();
        let before = g.generation_id.clone();
        g.skipped = records;
        assert_ne!(
            generation_id(&g.config, &g.files, &g.skipped).unwrap(),
            before
        );
    }

    fn replace_artifact(store: &Store, id: &str, bytes: &[u8]) {
        let dir = store.dir.join("generations").join(id);
        fs::write(dir.join("index.json"), bytes).unwrap();
        fs::write(
            dir.join("meta.json"),
            serde_json::to_vec(&Meta {
                generation_id: id.into(),
                artifact_sha256: sha256_hex(bytes),
                artifact_bytes: bytes.len(),
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn versions_retired_fields_and_false_ids_refuse_without_store_mutation() {
        let root = corpus();
        let good = build(root.path(), &IndexConfig::default()).unwrap();
        for schema in [1, 3] {
            let config = IndexConfig {
                schema_version: schema,
                ..IndexConfig::default()
            };
            assert!(
                matches!(build(root.path(), &config), Err(StoreError::Corrupt{reason,..}) if reason.contains("unsupported"))
            );
            let mut bad = good.clone();
            bad.config = config;
            let store_root = tempfile::tempdir().unwrap();
            let store = store_of(store_root.path());
            assert!(matches!(
                store.publish(&bad),
                Err(StoreError::Corrupt { .. })
            ));
            assert!(!store.dir.exists());
        }
        let mut bad_tokenizer = good.clone();
        bad_tokenizer.config.tokenizer_version = 99;
        let absent = tempfile::tempdir().unwrap();
        assert!(store_of(absent.path()).publish(&bad_tokenizer).is_err());
        assert!(!store_of(absent.path()).dir.exists());
        assert!(build(root.path(), &bad_tokenizer.config).is_err());

        let store = store_of(root.path());
        store.publish(&good).unwrap();
        let current = fs::read(store.dir.join("CURRENT")).unwrap();
        let history = fs::read(store.dir.join("HISTORY")).unwrap();
        let mut bad = good.clone();
        bad.generation_id = "0".repeat(64);
        assert!(
            matches!(store.publish(&bad),Err(StoreError::Corrupt{reason,..}) if reason=="generation id mismatch")
        );
        assert_eq!(fs::read(store.dir.join("CURRENT")).unwrap(), current);
        assert_eq!(fs::read(store.dir.join("HISTORY")).unwrap(), history);

        for mode in [
            "schema1",
            "schema3",
            "tokenizer",
            "retired",
            "id",
            "manifest",
        ] {
            let mut value = serde_json::to_value(&good).unwrap();
            match mode {
                "schema1" => {
                    value["config"]["schema_version"] = serde_json::json!(1);
                    value["files"][0]["mtime_ns"] = serde_json::json!(123);
                }
                "schema3" => value["config"]["schema_version"] = serde_json::json!(3),
                "tokenizer" => value["config"]["tokenizer_version"] = serde_json::json!(99),
                "retired" => value["files"][0]["mtime_ns"] = serde_json::json!(123),
                "manifest" => value["files"][0]["sha256"] = serde_json::json!("0".repeat(64)),
                _ => value["generation_id"] = serde_json::json!("0".repeat(64)),
            }
            replace_artifact(
                &store,
                &good.generation_id,
                &serde_json::to_vec(&value).unwrap(),
            );
            let error = store.load().unwrap_err().to_string();
            assert!(
                error.contains(match mode {
                    "schema1" | "schema3" | "tokenizer" => "unsupported",
                    "retired" => "mtime_ns",
                    _ => "id mismatch",
                }),
                "{mode}: {error}"
            );
            assert_eq!(fs::read(store.dir.join("CURRENT")).unwrap(), current);
            assert_eq!(fs::read(store.dir.join("HISTORY")).unwrap(), history);
        }
    }

    #[test]
    fn exact_republication_is_idempotent_and_index_or_meta_conflict_preserves_store() {
        let root = corpus();
        let g = build(root.path(), &IndexConfig::default()).unwrap();
        for mode in ["index", "meta"] {
            let other = tempfile::tempdir().unwrap();
            let store = store_of(other.path());
            store.publish(&g).unwrap();
            let dir = store.dir.join("generations").join(&g.generation_id);
            let before = [
                dir.join("index.json"),
                dir.join("meta.json"),
                store.dir.join("CURRENT"),
                store.dir.join("HISTORY"),
            ]
            .map(|p| fs::read(p).unwrap());
            store.publish(&g).unwrap();
            let after = [
                dir.join("index.json"),
                dir.join("meta.json"),
                store.dir.join("CURRENT"),
                store.dir.join("HISTORY"),
            ]
            .map(|p| fs::read(p).unwrap());
            assert_eq!(before, after);
            if mode == "index" {
                let mut altered = g.clone();
                altered.documents[0].pseudo_doc.push_str(" different bytes");
                replace_artifact(
                    &store,
                    &g.generation_id,
                    &serde_json::to_vec(&altered).unwrap(),
                );
            } else {
                let meta = fs::read(dir.join("meta.json")).unwrap();
                let mut altered = meta.clone();
                altered.push(b'\n');
                fs::write(dir.join("meta.json"), altered).unwrap();
            }
            let expected = [
                dir.join("index.json"),
                dir.join("meta.json"),
                store.dir.join("CURRENT"),
                store.dir.join("HISTORY"),
            ]
            .map(|p| fs::read(p).unwrap());
            assert!(
                matches!(store.publish(&g),Err(StoreError::Corrupt{reason,..}) if reason=="same-id artifact conflict")
            );
            assert_eq!(
                [
                    dir.join("index.json"),
                    dir.join("meta.json"),
                    store.dir.join("CURRENT"),
                    store.dir.join("HISTORY")
                ]
                .map(|p| fs::read(p).unwrap()),
                expected
            );
        }
    }

    #[test]
    fn load_rejects_duplicate_ids_even_with_matching_artifact_hashes() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        for kind in ["section", "chunk"] {
            let store_root = tempfile::tempdir().unwrap();
            let store = store_of(store_root.path());
            store.publish(&generation).unwrap();
            let mut malformed = generation.clone();
            if kind == "section" {
                malformed.sections.push(malformed.sections[0].clone());
            } else {
                malformed.chunks.push(malformed.chunks[0].clone());
            }
            // Model an older self-consistent artifact: the manifest/config ID
            // stays valid and both metadata checksums agree with its bytes.
            let bytes = serde_json::to_vec(&malformed).unwrap();
            let meta = serde_json::to_vec(&Meta {
                generation_id: malformed.generation_id.clone(),
                artifact_sha256: sha256_hex(&bytes),
                artifact_bytes: bytes.len(),
            })
            .unwrap();
            let dir = store
                .dir
                .join("generations")
                .join(&generation.generation_id);
            fs::write(dir.join("index.json"), bytes).unwrap();
            fs::write(dir.join("meta.json"), meta).unwrap();
            assert!(matches!(
                store.load(),
                Err(StoreError::Corrupt { reason, .. })
                    if reason == format!("duplicate {kind} id")
            ));
        }
    }

    #[test]
    fn cap_overflow_publishes_nothing() {
        let root = corpus();
        let store = store_of(root.path());
        let small = IndexConfig {
            max_chunks: 2,
            ..IndexConfig::default()
        };
        assert!(matches!(
            build(root.path(), &small),
            Err(StoreError::CapExceeded {
                what: "chunk count",
                cap: 2
            })
        ));

        let first = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&first).unwrap();
        fs::write(root.path().join("c.md"), "# C\n").unwrap();
        let tiny = IndexConfig {
            max_artifact_bytes: 64,
            ..IndexConfig::default()
        };
        let second = build(root.path(), &tiny).unwrap();
        assert!(matches!(
            store.publish(&second),
            Err(StoreError::CapExceeded {
                what: "artifact bytes",
                ..
            })
        ));
        assert_eq!(store.current_id().unwrap(), first.generation_id);
        assert_eq!(generation_dirs(&store), vec![first.generation_id.clone()]);
    }

    #[test]
    fn pruning_keeps_current_and_two_old_generations() {
        let root = corpus();
        let store = store_of(root.path());
        let mut ids = Vec::new();
        for i in 0..5 {
            fs::write(root.path().join("n.md"), format!("# N{i}\n")).unwrap();
            let generation = build(root.path(), &IndexConfig::default()).unwrap();
            store.publish(&generation).unwrap();
            ids.push(generation.generation_id);
        }
        let mut kept = ids[2..].to_vec();
        kept.sort();
        assert_eq!(generation_dirs(&store), kept);
        assert_eq!(store.current_id().unwrap(), ids[4]);
        // Republishing an existing id reuses its verified directory.
        let again = build(root.path(), &IndexConfig::default()).unwrap();
        store.publish(&again).unwrap();
        assert_eq!(generation_dirs(&store), kept);
    }

    #[test]
    fn freshness_lists_added_removed_and_modified_paths() {
        let root = corpus();
        let generation = build(root.path(), &IndexConfig::default()).unwrap();
        assert!(freshness(root.path(), &generation).unwrap().is_fresh());

        fs::write(root.path().join("a.md"), "# A\n\nChanged.\n").unwrap();
        fs::remove_file(root.path().join("b.md")).unwrap();
        fs::write(root.path().join("c.md"), "# C\n").unwrap();
        assert_eq!(
            freshness(root.path(), &generation).unwrap(),
            Freshness {
                added: vec!["c.md".into()],
                removed: vec!["b.md".into()],
                modified: vec!["a.md".into()],
            }
        );
    }
}
