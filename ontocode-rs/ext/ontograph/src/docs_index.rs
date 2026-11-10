//! Producer for the `docs-index.json` sidecar.
//!
//! Owns the Markdown walk, the bounded explicit-marker extraction, and the
//! artifact write. Reading and querying the artifact stay in `docs.rs`, so the
//! reader and writer roles remain separate as the storage contract requires.
//!
//! Extraction is restricted to explicit markers, per
//! `ADR_ONTOGRAPH_DOCS_INDEX_PRODUCER.md`: ATX headings outside fenced code
//! blocks, requirement ids only where an author wrote the `Requirement:`
//! marker, and links only for backticked references that resolve exactly
//! against the source index built in the same run. Nothing is inferred.

use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest;
use sha2::Sha256;

use crate::docs::DOCS_INDEX_FILE;
use crate::docs::DOCS_SCHEMA_VERSION;
use crate::docs::DocsDocument;
use crate::docs::DocsFact;
use crate::docs::DocsHeading;
use crate::docs::DocsIndexArtifact;
use crate::docs::DocsIndexManifest;
use crate::docs::DocsLink;
use crate::indexer::SourceIndexArtifact;
use crate::manifest::OntographArtifactRoot;
use ontocode_utils_path::write_atomically;

/// Marker that must begin a line's content for a requirement id to be recorded.
const REQUIREMENT_MARKER: &str = "Requirement:";
/// Fixed document kind for this producer. Markdown is the only supported form.
const MARKDOWN_KIND: &str = "markdown";
/// Byte cap for the stored document summary, applied on a character boundary.
const MAX_SUMMARY_BYTES: usize = 1024;
pub(crate) const MAX_DOCS_DOCUMENTS: usize = 2048;
pub(crate) const MAX_DOCS_ARTIFACT_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_DOCS_FILE_BYTES: usize = 1024 * 1024;

/// Inputs the producer needs from the indexing run that owns the generation.
pub(crate) struct DocsIndexRequest<'a> {
    pub repo_root: &'a Path,
    pub artifact_root: &'a OntographArtifactRoot,
    pub generation_id: &'a str,
    pub source_index_content_hash: &'a str,
    pub implementation_version: &'a str,
    pub timestamp_unix_seconds: i64,
}

/// Why a docs sidecar could not be produced.
///
/// The producer never partially publishes: either a complete artifact with a
/// completion timestamp is written, or nothing is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DocsIndexBuildError {
    Walk,
    ReadFile(String),
    Serialize,
    WriteArtifact,
}

/// Walks Markdown, extracts explicit facts and links, and writes the sidecar.
///
/// `source_index` supplies the only accepted link targets. A reference that
/// does not match an indexed path, symbol, or route exactly is skipped rather
/// than guessed, because the docs consumers report missing references as
/// findings and a fabricated link would corrupt that distinction.
pub(crate) fn build_and_publish_docs_index(
    request: DocsIndexRequest<'_>,
    source_index: &SourceIndexArtifact,
) -> Result<Vec<String>, DocsIndexBuildError> {
    let markdown_files = collect_markdown_files(request.repo_root)?;
    if markdown_files.len() > MAX_DOCS_DOCUMENTS {
        return Ok(vec![format!(
            "docs-index.json not written: {} Markdown documents exceed MAX_DOCS_DOCUMENTS ({MAX_DOCS_DOCUMENTS})",
            markdown_files.len()
        )]);
    }
    // A workspace with no Markdown has nothing to publish. Writing an empty
    // sidecar would assert that the docs surface exists and is simply empty,
    // which is a different claim than "this workspace has no documentation".
    // The loader's missing-sidecar path is the correct answer here, so leave
    // the artifact absent.
    if markdown_files.is_empty() {
        return Ok(vec![]);
    }
    let targets = SourceTargets::from_source_index(source_index);

    let mut documents = Vec::new();
    let mut facts = Vec::new();
    let mut links = Vec::new();
    let mut retained_bytes = 0usize;

    for path in markdown_files {
        let relative_path = relative_slash_path(request.repo_root, &path)?;
        let Some(content) = read_markdown_file(&path, &relative_path)? else {
            return Ok(vec![format!(
                "docs-index.json not written: Markdown file {relative_path} exceeds MAX_DOCS_FILE_BYTES ({MAX_DOCS_FILE_BYTES})"
            )]);
        };
        let extracted = extract_document(&relative_path, &content, &targets);
        retained_bytes = retained_bytes.saturating_add(extracted_size_hint(&extracted));
        if retained_bytes > MAX_DOCS_ARTIFACT_BYTES {
            return Ok(vec![format!(
                "docs-index.json not written: extracted docs data exceeds MAX_DOCS_ARTIFACT_BYTES ({MAX_DOCS_ARTIFACT_BYTES})"
            )]);
        }
        documents.push(extracted.document);
        facts.extend(extracted.facts);
        links.extend(extracted.links);
    }

    let manifest = DocsIndexManifest {
        schema_version: DOCS_SCHEMA_VERSION,
        generator_version: request.implementation_version.to_string(),
        repository_root: request.repo_root.display().to_string(),
        source_index_generation_id: request.generation_id.to_string(),
        source_index_content_hash: request.source_index_content_hash.to_string(),
        created_at_unix_seconds: request.timestamp_unix_seconds,
        // Written before serialization: the loader treats a missing completion
        // timestamp as `InProgress`, and `write_atomically` makes the file
        // visible only once it is whole.
        completed_at_unix_seconds: Some(request.timestamp_unix_seconds),
        artifact_content_hash: String::new(),
    };
    let mut artifact = DocsIndexArtifact {
        manifest,
        documents,
        facts,
        links,
    };
    artifact.manifest.artifact_content_hash = artifact_content_hash(&artifact)?;

    let mut writer = CappedJsonWriter::new(MAX_DOCS_ARTIFACT_BYTES);
    serde_json::to_writer_pretty(&mut writer, &artifact)
        .map_err(|_| DocsIndexBuildError::Serialize)?;
    writer
        .write_all(b"\n")
        .map_err(|_| DocsIndexBuildError::Serialize)?;
    if writer.exceeded() {
        return Ok(vec![format!(
            "docs-index.json not written: serialized sidecar exceeds MAX_DOCS_ARTIFACT_BYTES ({MAX_DOCS_ARTIFACT_BYTES})"
        )]);
    }
    let artifact_path = request
        .artifact_root
        .generation_file_path(request.generation_id, DOCS_INDEX_FILE)
        .map_err(|_| DocsIndexBuildError::WriteArtifact)?;
    let contents =
        String::from_utf8(writer.into_bytes()).map_err(|_| DocsIndexBuildError::Serialize)?;
    write_atomically(&artifact_path, &contents).map_err(|_| DocsIndexBuildError::WriteArtifact)?;
    Ok(vec![])
}

/// Hash of the artifact with the hash field itself empty, so the value is
/// reproducible from the published file.
fn artifact_content_hash(artifact: &DocsIndexArtifact) -> Result<String, DocsIndexBuildError> {
    let mut hasher = Sha256::new();
    let mut writer = HashWriter(&mut hasher);
    serde_json::to_writer(&mut writer, artifact).map_err(|_| DocsIndexBuildError::Serialize)?;
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

struct HashWriter<'a>(&'a mut Sha256);

impl Write for HashWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CappedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
    exceeded: bool,
}

impl CappedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(8192)),
            limit,
            exceeded: false,
        }
    }

    fn exceeded(&self) -> bool {
        self.exceeded
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for CappedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        if bytes.len() > remaining {
            self.bytes.extend_from_slice(&bytes[..remaining]);
            self.exceeded = true;
        } else {
            self.bytes.extend_from_slice(bytes);
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn read_markdown_file(
    path: &Path,
    relative_path: &str,
) -> Result<Option<String>, DocsIndexBuildError> {
    let file = fs::File::open(path)
        .map_err(|_| DocsIndexBuildError::ReadFile(relative_path.to_string()))?;
    let mut bytes = Vec::with_capacity(MAX_DOCS_FILE_BYTES.min(8192));
    file.take((MAX_DOCS_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| DocsIndexBuildError::ReadFile(relative_path.to_string()))?;
    if bytes.len() > MAX_DOCS_FILE_BYTES {
        return Ok(None);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| DocsIndexBuildError::ReadFile(relative_path.to_string()))
}

fn extracted_size_hint(extracted: &ExtractedDocument) -> usize {
    let mut bytes = 0usize;
    let add = |total: &mut usize, value: &str| {
        *total = total.saturating_add(value.len());
    };
    let document = &extracted.document;
    add(&mut bytes, &document.id);
    add(&mut bytes, &document.path);
    add(&mut bytes, &document.title);
    add(&mut bytes, &document.kind);
    add(&mut bytes, &document.content_hash);
    add(&mut bytes, &document.summary);
    for heading in &document.headings {
        add(&mut bytes, &heading.id);
        add(&mut bytes, &heading.title);
    }
    for fact in &extracted.facts {
        add(&mut bytes, &fact.id);
        add(&mut bytes, &fact.document_id);
        add(&mut bytes, &fact.kind);
        add(&mut bytes, &fact.text);
        if let Some(requirement_id) = &fact.requirement_id {
            add(&mut bytes, requirement_id);
        }
        for value in fact.routes.iter().chain(&fact.paths).chain(&fact.symbols) {
            add(&mut bytes, value);
        }
    }
    for link in &extracted.links {
        add(&mut bytes, &link.id);
        add(&mut bytes, &link.from_document_id);
        add(&mut bytes, &link.target_kind);
        add(&mut bytes, &link.target);
    }
    bytes.saturating_add(
        (document.headings.len() + extracted.facts.len() + extracted.links.len()) * 128,
    )
}

/// Exact-match targets drawn from the source index built in the same run.
struct SourceTargets {
    paths: BTreeSet<String>,
    symbols: BTreeSet<String>,
    routes: BTreeSet<String>,
}

impl SourceTargets {
    fn from_source_index(artifact: &SourceIndexArtifact) -> Self {
        let mut paths = BTreeSet::new();
        let mut symbols = BTreeSet::new();
        let mut routes = BTreeSet::new();
        for file in &artifact.files {
            paths.insert(file.path.clone());
            for symbol in &file.symbols {
                symbols.insert(symbol.name.clone());
            }
        }
        for route in &artifact.routes {
            routes.insert(route.route_path.clone());
        }
        Self {
            paths,
            symbols,
            routes,
        }
    }

    /// Classifies a backticked reference, or returns `None` when it resolves to
    /// nothing. Matching is exact string equality: no fuzzy matching, no case
    /// folding, and no path normalization beyond the stored slash form.
    fn classify(&self, reference: &str) -> Option<&'static str> {
        if self.paths.contains(reference) {
            return Some("path");
        }
        if self.symbols.contains(reference) {
            return Some("symbol");
        }
        if self.routes.contains(reference) {
            return Some("route");
        }
        None
    }
}

struct ExtractedDocument {
    document: DocsDocument,
    facts: Vec<DocsFact>,
    links: Vec<DocsLink>,
}

fn extract_document(
    relative_path: &str,
    content: &str,
    targets: &SourceTargets,
) -> ExtractedDocument {
    let document_id = stable_id("doc", relative_path);
    let mut headings = Vec::new();
    let mut facts = Vec::new();
    let mut links = Vec::new();
    let mut title = None;
    let mut in_fenced_block = false;

    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        if is_fence(line) {
            in_fenced_block = !in_fenced_block;
            continue;
        }
        // A `#` inside a fenced block is a comment in most languages, not a
        // document heading.
        if in_fenced_block {
            continue;
        }

        let Some((level, heading_text)) = atx_heading(line) else {
            if let Some(requirement_id) = explicit_requirement_id(line) {
                facts.push(DocsFact {
                    id: stable_id(&format!("{document_id}-fact"), &line_number.to_string()),
                    document_id: document_id.clone(),
                    heading_id: None,
                    kind: "requirement".to_string(),
                    text: line.trim().to_string(),
                    line_start: line_number,
                    line_end: line_number,
                    requirement_id: Some(requirement_id),
                    routes: Vec::new(),
                    paths: Vec::new(),
                    symbols: Vec::new(),
                });
            }
            continue;
        };
        let heading_id = stable_id(&format!("{document_id}-h"), &line_number.to_string());
        headings.push(DocsHeading {
            id: heading_id.clone(),
            title: heading_text.to_string(),
        });
        if level == 1 && title.is_none() {
            title = Some(heading_text.to_string());
        }

        let requirement_id = explicit_requirement_id(heading_text);
        let mut fact_paths = Vec::new();
        let mut fact_symbols = Vec::new();
        let mut fact_routes = Vec::new();
        for reference in backticked_references(heading_text) {
            match targets.classify(&reference) {
                Some("path") => fact_paths.push(reference.clone()),
                Some("symbol") => fact_symbols.push(reference.clone()),
                Some("route") => fact_routes.push(reference.clone()),
                // Unresolved references are skipped, never guessed.
                _ => continue,
            }
            let Some(target_kind) = targets.classify(&reference) else {
                continue;
            };
            links.push(DocsLink {
                id: stable_id(&format!("{heading_id}-link"), &reference),
                from_document_id: document_id.clone(),
                target_kind: target_kind.to_string(),
                target: reference,
            });
        }

        facts.push(DocsFact {
            id: stable_id(&format!("{document_id}-fact"), &line_number.to_string()),
            document_id: document_id.clone(),
            heading_id: Some(heading_id),
            kind: if requirement_id.is_some() {
                "requirement".to_string()
            } else {
                "heading".to_string()
            },
            text: heading_text.to_string(),
            line_start: line_number,
            line_end: line_number,
            requirement_id,
            routes: fact_routes,
            paths: fact_paths,
            symbols: fact_symbols,
        });
    }

    let document = DocsDocument {
        id: document_id,
        path: relative_path.to_string(),
        title: title.unwrap_or_else(|| relative_path.to_string()),
        kind: MARKDOWN_KIND.to_string(),
        content_hash: content_hash(content),
        byte_length: content.len(),
        headings,
        summary: bounded_summary(content),
    };

    ExtractedDocument {
        document,
        facts,
        links,
    }
}

/// Returns the heading level and text for an ATX heading line.
///
/// Setext headings and HTML headings are not recognized, per the producer
/// contract.
fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let text = rest.trim();
    if text.is_empty() {
        return None;
    }
    Some((hashes, text))
}

fn is_fence(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Extracts a requirement id only when the author wrote the explicit marker.
///
/// No `REQ-###` pattern is inferred: consumers compare `requirement_id` by
/// exact equality and their contract states requirement facts are accepted only
/// when the id is explicit.
fn explicit_requirement_id(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix(REQUIREMENT_MARKER)?;
    let identifier = rest.split_whitespace().next()?;
    let identifier = identifier.trim_matches('`');
    if identifier.is_empty() {
        return None;
    }
    Some(identifier.to_string())
}

/// Returns the contents of each backtick-delimited span, in order.
fn backticked_references(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter(|(index, _part)| index % 2 == 1)
        .map(|(_index, part)| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

fn bounded_summary(content: &str) -> String {
    let body = content.trim_start();
    if body.len() <= MAX_SUMMARY_BYTES {
        return body.to_string();
    }
    let mut end = MAX_SUMMARY_BYTES;
    while end > 0 && !body.is_char_boundary(end) {
        end -= 1;
    }
    body[..end].to_string()
}

fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn stable_id(prefix: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{prefix}-{}", &digest[..16])
}

fn relative_slash_path(repo_root: &Path, path: &Path) -> Result<String, DocsIndexBuildError> {
    let relative = path
        .strip_prefix(repo_root)
        .map_err(|_| DocsIndexBuildError::Walk)?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

/// Walks Markdown files using the same skip rules as the source indexer, so the
/// two walks cannot drift apart.
fn collect_markdown_files(repo_root: &Path) -> Result<Vec<PathBuf>, DocsIndexBuildError> {
    let mut files = Vec::new();
    collect_markdown_files_from(repo_root, repo_root, &mut files)?;
    files.sort();
    Ok(files)
}

fn is_docs_ignored_dir(repo_root: &Path, path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()) == Some("node_modules") {
        return true;
    }
    if let Ok(relative) = relative_slash_path(repo_root, path)
        && (relative == ".ontocode" || relative == ".memory-bank/archive")
    {
        return true;
    }
    false
}

fn collect_markdown_files_from(
    repo_root: &Path,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), DocsIndexBuildError> {
    let entries = fs::read_dir(path).map_err(|_| DocsIndexBuildError::Walk)?;
    for entry in entries {
        let entry = entry.map_err(|_| DocsIndexBuildError::Walk)?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|_| DocsIndexBuildError::Walk)?;
        if file_type.is_dir() {
            if !crate::indexer::is_ignored_dir(repo_root, &path)
                && !is_docs_ignored_dir(repo_root, &path)
            {
                collect_markdown_files_from(repo_root, &path, files)?;
                if files.len() > MAX_DOCS_DOCUMENTS {
                    return Ok(());
                }
            }
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            files.push(path);
            if files.len() > MAX_DOCS_DOCUMENTS {
                // Retain one overflow sentinel so the caller can report the
                // cap without collecting the rest of the workspace.
                return Ok(());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "docs_index_tests.rs"]
mod docs_index_tests;
