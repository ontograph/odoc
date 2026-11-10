use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use schemars::JsonSchema;
use semver::Version;
use serde::Deserialize;
use serde::Serialize;

use crate::artifact_io::read_to_string;
const MAX_KM_TOML_BYTES: u64 = 64 * 1024;
const MAX_FACTS_JSONL_BYTES: u64 = 512 * 1024;
const MAX_FACT_COUNT: usize = 200;
const MAX_SUMMARY_CHARS: usize = 500;
const MAX_FACT_CLAIM_CHARS: usize = 500;
const MAX_FACT_SOURCE_CHARS: usize = 300;
const MAX_TRANSITION_COUNT: usize = 200;
const MAX_TRANSITION_ENDPOINT_CHARS: usize = 200;
const MAX_TRANSITION_ATTRIBUTE_CHARS: usize = 500;
const MAX_EXAMPLE_FILES: usize = 20;
const MAX_TEST_FILES: usize = 20;
const MAX_CODEBASE_REPOS: usize = 20;
const MAX_CODEBASE_REPO_NAME_CHARS: usize = 100;
const MAX_CODEBASE_REPO_URL_CHARS: usize = 300;
const MAX_CODEBASE_REPO_COMMIT_CHARS: usize = 100;
/// Shared ingestion budget applied to the whole KM package before hashing or
/// copy so an oversized, deeply nested, or symlinked package is rejected up
/// front (PR-033, PR-037, PR-038).
pub(crate) const MAX_PACKAGE_TOTAL_BYTES: u64 = 4 * 1024 * 1024;
const MAX_PACKAGE_ENTRIES: usize = 2_000;
const MAX_PACKAGE_DEPTH: usize = 32;
pub(crate) const MAX_RENDERED_CONTEXT_BYTES: usize = 32 * 1024;
pub(crate) const MAX_RENDERED_FACTS: usize = 30;
pub(crate) const MAX_RENDERED_EXAMPLES: usize = 5;
/// Advisory procedural transitions rendered into one context fragment. A
/// two-hop projection over a package holding `MAX_TRANSITION_COUNT`
/// transitions can fan out well past a reviewable size, so guidance is capped
/// here rather than left to blind byte truncation.
pub(crate) const MAX_RENDERED_TRANSITIONS: usize = 32;

const ACCEPTED_KM_KINDS: &[&str] = &[
    "domain",
    "framework",
    "decision",
    "recipe",
    "composite",
    "translation",
    "codebase-graph",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KmValidationStatus {
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmValidationReport {
    pub path: String,
    pub id: String,
    pub version: String,
    pub status: KmValidationStatus,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub facts: usize,
    pub examples: usize,
    pub tests: usize,
    pub limits: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct KmManifest {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) kind: String,
    pub(crate) summary: Option<String>,
    pub(crate) sources: Option<Vec<String>>,
    pub(crate) licenses: Option<Vec<String>>,
    pub(crate) provenance: Option<KmProvenance>,
    /// Donor code repositories this KM was distilled from. Provenance only: it
    /// never makes the repository a runtime dependency or a fetch target.
    pub(crate) codebase_repos: Option<Vec<KmCodebaseRepo>>,
}

/// One upstream code repository a KM was distilled from, recorded so the KM
/// stays auditable after a temporary donor checkout is deleted.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmCodebaseRepo {
    pub(crate) name: String,
    pub(crate) url: String,
    /// Donor revision, never the Ontocode revision.
    pub(crate) commit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmProvenance {
    pub(crate) created_by: String,
    pub(crate) generated_at: String,
    pub(crate) source_commit: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
pub(crate) struct KmFact {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) claim: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmTransition {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) relation: String,
    pub(crate) condition: String,
    pub(crate) guidance: String,
    #[serde(default)]
    pub(crate) pitfalls: Vec<String>,
}

pub(crate) fn validate_km(path: &Path) -> KmValidationReport {
    validate_km_with_project_root(path, None)
}

pub(crate) fn validate_km_for_project(path: &Path, project_root: &Path) -> KmValidationReport {
    validate_km_with_project_root(path, Some(project_root))
}

fn validate_km_with_project_root(path: &Path, project_root: Option<&Path>) -> KmValidationReport {
    let mut report = KmValidationReport {
        path: path.display().to_string(),
        id: String::new(),
        version: String::new(),
        status: KmValidationStatus::Invalid,
        warnings: Vec::new(),
        errors: Vec::new(),
        facts: 0,
        examples: 0,
        tests: 0,
        limits: validation_limits(),
    };

    let root = match fs::canonicalize(path) {
        Ok(root) if root.is_dir() => root,
        Ok(_) => {
            report
                .errors
                .push("requested path is not a KM directory".to_string());
            return report;
        }
        Err(_) => {
            report
                .errors
                .push("requested KM directory does not exist".to_string());
            return report;
        }
    };

    let manifest = match load_manifest(&root) {
        Ok(manifest) => manifest,
        Err(err) => {
            report.errors.push(err);
            return report;
        }
    };

    // Enforce the shared package budget before any further reads so oversized,
    // deeply nested, or symlinked packages never reach hashing or copy.
    if let Err(budget_error) = validate_package_budget(&root) {
        report.errors.push(budget_error);
        return report;
    }

    report.id = manifest.id.clone();
    report.version = manifest.version.clone();
    validate_manifest(&manifest, &mut report.errors);
    if let Some(project_root) = project_root {
        validate_sources(
            manifest.sources.as_deref().unwrap_or_default(),
            project_root,
            "km.toml",
            &mut report.errors,
        );
    }

    let facts_text = match load_facts_text(&root) {
        Ok(text) => text,
        Err(err) => {
            report.errors.push(err);
            return report;
        }
    };
    report.facts = validate_facts(&facts_text, project_root, &mut report.errors);

    if let Some(transitions_text) = load_optional_transitions_text(&root, &mut report.errors) {
        validate_transitions(&transitions_text, &mut report.errors);
    }

    if let Some(project_root) = project_root {
        validate_optional_jsonl_sources(&root, "mappings.jsonl", project_root, &mut report.errors);
    }

    report.examples =
        count_required_files(&root, "examples", MAX_EXAMPLE_FILES, &mut report.errors);
    report.tests = count_required_files(&root, "tests", MAX_TEST_FILES, &mut report.errors);

    if report.errors.is_empty() {
        report.status = KmValidationStatus::Valid;
    }
    report
}

pub(crate) fn load_manifest(root: &Path) -> Result<KmManifest, String> {
    let manifest_path = resolve_child_path(root, "km.toml")?;
    let manifest_text = read_bounded_file(&manifest_path, MAX_KM_TOML_BYTES)?;
    toml::from_str::<KmManifest>(&manifest_text).map_err(|err| format!("malformed km.toml: {err}"))
}

pub(crate) fn load_facts(root: &Path) -> Result<Vec<KmFact>, String> {
    let mut facts = Vec::new();
    let facts_text = load_facts_text(root)?;
    for line in facts_text.lines() {
        let fact = serde_json::from_str::<KmFact>(line)
            .map_err(|err| format!("facts.jsonl contains malformed JSONL: {err}"))?;
        facts.push(fact);
    }
    Ok(facts)
}

pub(crate) fn load_transitions(root: &Path) -> Result<Vec<KmTransition>, String> {
    let path = resolve_child_path(root, "transitions.jsonl")?;
    let text = read_bounded_file(&path, MAX_FACTS_JSONL_BYTES)?;
    text.lines()
        .enumerate()
        .map(|(index, line)| {
            if index >= MAX_TRANSITION_COUNT {
                return Err("transitions.jsonl exceeds the v1 transition cap".to_string());
            }
            let transition = serde_json::from_str::<KmTransition>(line)
                .map_err(|err| format!("transitions.jsonl contains malformed JSONL: {err}"))?;
            validate_transition(&transition)?;
            Ok(transition)
        })
        .collect()
}

fn validation_limits() -> Vec<String> {
    vec![
        format!("km.toml is capped at {MAX_KM_TOML_BYTES} bytes"),
        format!("facts.jsonl is capped at {MAX_FACTS_JSONL_BYTES} bytes"),
        format!("fact count is capped at {MAX_FACT_COUNT}"),
        format!("fact claim length is capped at {MAX_FACT_CLAIM_CHARS} characters"),
        format!("fact source length is capped at {MAX_FACT_SOURCE_CHARS} characters"),
        format!("transition count is capped at {MAX_TRANSITION_COUNT}"),
        format!(
            "transition endpoint length is capped at {MAX_TRANSITION_ENDPOINT_CHARS} characters"
        ),
        format!(
            "transition attribute length is capped at {MAX_TRANSITION_ATTRIBUTE_CHARS} characters"
        ),
        format!("example files are capped at {MAX_EXAMPLE_FILES}"),
        format!("test files are capped at {MAX_TEST_FILES}"),
        format!("codebase repos are capped at {MAX_CODEBASE_REPOS}"),
        format!("total package bytes are capped at {MAX_PACKAGE_TOTAL_BYTES}"),
        format!("package entries are capped at {MAX_PACKAGE_ENTRIES}"),
        format!("package directory depth is capped at {MAX_PACKAGE_DEPTH}"),
        "symlinked package entries are rejected".to_string(),
        format!("rendered context is capped at {MAX_RENDERED_CONTEXT_BYTES} bytes"),
        format!("rendered facts are capped at {MAX_RENDERED_FACTS}"),
        format!("rendered examples are capped at {MAX_RENDERED_EXAMPLES}"),
        "context rendering is not performed here".to_string(),
    ]
}

/// Walks the entire canonical package with `symlink_metadata` and enforces the
/// shared ingestion budget: no symlinked entries, bounded directory depth,
/// bounded entry count, and a hard total-byte cap. Returns the first violation.
pub(crate) fn validate_package_budget(root: &Path) -> Result<(), String> {
    let mut total_bytes: u64 = 0;
    let mut entries: usize = 0;
    // Each stack item is (path, depth). The root is depth 0.
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        if depth > MAX_PACKAGE_DEPTH {
            return Err("package directory tree is too deep".to_string());
        }
        let read_dir =
            fs::read_dir(&dir).map_err(|err| format!("failed to read package: {err}"))?;
        for entry in read_dir {
            let entry = entry.map_err(|err| format!("failed to read package: {err}"))?;
            let entry_path = entry.path();
            // symlink_metadata does not follow links, so a symlinked file or
            // directory is detected instead of silently traversed.
            let metadata = fs::symlink_metadata(&entry_path)
                .map_err(|err| format!("failed to inspect package entry: {err}"))?;
            if metadata.file_type().is_symlink() {
                return Err("package contains a symlinked entry".to_string());
            }
            entries += 1;
            if entries > MAX_PACKAGE_ENTRIES {
                return Err("package has too many entries".to_string());
            }
            if metadata.is_dir() {
                pending.push((entry_path, depth + 1));
                continue;
            }
            if !metadata.is_file() {
                return Err("package contains a non-regular entry".into());
            }
            if metadata.is_file() {
                total_bytes = total_bytes.saturating_add(metadata.len());
                if total_bytes > MAX_PACKAGE_TOTAL_BYTES {
                    return Err("package exceeds the total byte budget".to_string());
                }
            }
        }
    }
    Ok(())
}

fn validate_manifest(manifest: &KmManifest, errors: &mut Vec<String>) {
    if !is_path_like_id(&manifest.id) {
        errors.push("km.toml id must be stable and path-like".to_string());
    }
    if Version::parse(&manifest.version).is_err() {
        errors.push("km.toml version must be strict semver".to_string());
    }
    if !ACCEPTED_KM_KINDS.contains(&manifest.kind.as_str()) {
        errors.push(format!("km.toml kind `{}` is unsupported", manifest.kind));
    }
    validate_codebase_repos(manifest.codebase_repos.as_deref(), errors);
    let Some(summary) = manifest.summary.as_deref() else {
        errors.push("km.toml summary is required".to_string());
        return;
    };
    if summary.trim().is_empty() {
        errors.push("km.toml summary is required".to_string());
    } else if summary.chars().count() > MAX_SUMMARY_CHARS {
        errors.push("km.toml summary exceeds the v1 cap".to_string());
    }
    let Some(sources) = manifest.sources.as_ref() else {
        errors.push("km.toml sources must be present".to_string());
        return;
    };
    if sources.is_empty() || sources.iter().any(|item| item.trim().is_empty()) {
        errors.push("km.toml sources must be present".to_string());
    }
    let Some(licenses) = manifest.licenses.as_ref() else {
        errors.push("km.toml licenses must be present".to_string());
        return;
    };
    if licenses.is_empty() || licenses.iter().any(|item| item.trim().is_empty()) {
        errors.push("km.toml licenses must be present".to_string());
    }
    let Some(provenance) = manifest.provenance.as_ref() else {
        errors.push("km.toml provenance must be present".to_string());
        return;
    };
    if provenance.created_by.trim().is_empty()
        || provenance.generated_at.trim().is_empty()
        || provenance.source_commit.trim().is_empty()
    {
        errors.push("km.toml provenance must be present".to_string());
    }
}

/// `codebase_repos` is optional, so absence is valid. When present every entry
/// must be complete and bounded; the validator stays offline and never checks
/// that the URL resolves or the commit exists.
fn validate_codebase_repos(repos: Option<&[KmCodebaseRepo]>, errors: &mut Vec<String>) {
    let Some(repos) = repos else {
        return;
    };
    if repos.len() > MAX_CODEBASE_REPOS {
        errors.push("km.toml codebase_repos exceeds the v1 cap".to_string());
    }
    for repo in repos {
        if repo.name.trim().is_empty()
            || repo.url.trim().is_empty()
            || repo.commit.trim().is_empty()
        {
            errors.push("km.toml codebase_repos entries require name, url, and commit".to_string());
            continue;
        }
        if repo.name.chars().count() > MAX_CODEBASE_REPO_NAME_CHARS
            || repo.url.chars().count() > MAX_CODEBASE_REPO_URL_CHARS
            || repo.commit.chars().count() > MAX_CODEBASE_REPO_COMMIT_CHARS
        {
            errors.push("km.toml codebase_repos entry exceeds the v1 cap".to_string());
        }
    }
}

fn validate_facts(
    facts_text: &str,
    project_root: Option<&Path>,
    errors: &mut Vec<String>,
) -> usize {
    let mut count = 0;
    for line in facts_text.lines() {
        count += 1;
        if count > MAX_FACT_COUNT {
            errors.push("facts.jsonl exceeds the v1 fact cap".to_string());
            return count;
        }

        let fact = match serde_json::from_str::<KmFact>(line) {
            Ok(fact) => fact,
            Err(err) => {
                errors.push(format!("facts.jsonl contains malformed JSONL: {err}"));
                return count - 1;
            }
        };
        validate_fact(&fact, project_root, errors);
    }
    if count == 0 {
        errors.push("facts.jsonl must contain at least one fact".to_string());
    }
    count
}

fn load_facts_text(root: &Path) -> Result<String, String> {
    let facts_path = resolve_child_path(root, "facts.jsonl")?;
    read_bounded_file(&facts_path, MAX_FACTS_JSONL_BYTES)
}

fn validate_fact(fact: &KmFact, project_root: Option<&Path>, errors: &mut Vec<String>) {
    if fact.id.trim().is_empty() {
        errors.push("fact is missing id".to_string());
    }
    if fact.kind.trim().is_empty() {
        errors.push("fact is missing kind".to_string());
    }
    if fact.claim.trim().is_empty() {
        errors.push("fact is missing claim".to_string());
    } else if fact.claim.chars().count() > MAX_FACT_CLAIM_CHARS {
        errors.push("fact claim exceeds the v1 cap".to_string());
    }
    if fact.source.trim().is_empty() {
        errors.push("fact is missing source".to_string());
    } else if project_root.is_some_and(|root| canonical_source_path(root, &fact.source).is_none()) {
        errors.push("fact source is invalid".to_string());
    } else if fact.source.chars().count() > MAX_FACT_SOURCE_CHARS {
        errors.push("fact source exceeds the v1 cap".to_string());
    }
    if !(0.0..=1.0).contains(&fact.confidence) {
        errors.push("fact confidence must be within 0.0..=1.0".to_string());
    }
}

fn load_optional_transitions_text(root: &Path, errors: &mut Vec<String>) -> Option<String> {
    let path = match resolve_child_path(root, "transitions.jsonl") {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("failed to read transitions.jsonl: {error}"));
            return None;
        }
    };
    match read_bounded_file(&path, MAX_FACTS_JSONL_BYTES) {
        Ok(text) => Some(text),
        Err(error) if error.starts_with("missing ") => None,
        Err(error) => {
            errors.push(format!("failed to read transitions.jsonl: {error}"));
            None
        }
    }
}

fn validate_transitions(text: &str, errors: &mut Vec<String>) {
    let mut count = 0usize;
    for line in text.lines() {
        count += 1;
        if count > MAX_TRANSITION_COUNT {
            errors.push("transitions.jsonl exceeds the v1 transition cap".to_string());
            return;
        }
        let transition = match serde_json::from_str::<KmTransition>(line) {
            Ok(transition) => transition,
            Err(err) => {
                errors.push(format!("transitions.jsonl contains malformed JSONL: {err}"));
                return;
            }
        };
        errors.extend(validate_transition_errors(&transition));
    }
}

fn validate_transition(transition: &KmTransition) -> Result<(), String> {
    validate_transition_errors(transition)
        .into_iter()
        .next()
        .map_or(Ok(()), Err)
}

fn validate_transition_errors(transition: &KmTransition) -> Vec<String> {
    let mut errors = Vec::new();
    if transition.source.trim().is_empty() || transition.target.trim().is_empty() {
        errors.push("transition requires source and target".to_string());
    }
    if transition.source.chars().count() > MAX_TRANSITION_ENDPOINT_CHARS
        || transition.target.chars().count() > MAX_TRANSITION_ENDPOINT_CHARS
    {
        errors.push("transition endpoint exceeds the v1 cap".to_string());
    }
    if transition.relation.trim().is_empty()
        || transition.condition.trim().is_empty()
        || transition.guidance.trim().is_empty()
    {
        errors.push("transition requires relation, condition, and guidance".to_string());
    }
    if transition.relation.chars().count() > MAX_TRANSITION_ATTRIBUTE_CHARS
        || transition.condition.chars().count() > MAX_TRANSITION_ATTRIBUTE_CHARS
        || transition.guidance.chars().count() > MAX_TRANSITION_ATTRIBUTE_CHARS
        || transition
            .pitfalls
            .iter()
            .any(|pitfall| pitfall.chars().count() > MAX_TRANSITION_ATTRIBUTE_CHARS)
    {
        errors.push("transition attribute exceeds the v1 cap".to_string());
    }
    errors
}

fn count_required_files(
    root: &Path,
    child: &str,
    max_files: usize,
    errors: &mut Vec<String>,
) -> usize {
    let dir = match resolve_child_path(root, child) {
        Ok(path) => path,
        Err(err) => {
            errors.push(err);
            return 0;
        }
    };
    if !dir.is_dir() {
        errors.push(format!("{child} must exist"));
        return 0;
    }

    let mut count = 0;
    let mut pending = vec![dir];
    while let Some(path) = pending.pop() {
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(err) => {
                errors.push(format!("failed to read {child}: {err}"));
                return count;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    errors.push(format!("failed to read {child}: {err}"));
                    return count;
                }
            };
            let entry_path = entry.path();
            if entry_path.is_dir() {
                pending.push(entry_path);
                continue;
            }
            if entry_path.is_file() {
                count += 1;
                if count > max_files {
                    errors.push(format!("{child} exceeds the v1 file cap"));
                    return count;
                }
            }
        }
    }

    if count == 0 {
        errors.push(format!("{child} must contain at least one file"));
    }
    count
}

fn read_bounded_file(path: &Path, max_bytes: u64) -> Result<String, String> {
    let (Some(root), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(format!("missing {}", path.display()));
    };
    crate::artifact_io::read_to_string_beneath(root, Path::new(name), max_bytes as usize).map_err(
        |err| match err.kind() {
            std::io::ErrorKind::NotFound => format!("missing {}", path.display()),
            std::io::ErrorKind::InvalidData if err.to_string().contains("byte limit") => {
                format!("{} exceeds the v1 size cap", path.display())
            }
            _ => format!("failed to read {}: {err}", path.display()),
        },
    )
}

fn resolve_child_path(root: &Path, child: &str) -> Result<PathBuf, String> {
    let child_path = Path::new(child);
    if child_path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(format!("unsupported path outside KM folder: {child}"));
    }
    Ok(root.join(child_path))
}

fn is_path_like_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('/')
        && id
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn validate_sources(
    sources: &[String],
    project_root: &Path,
    owner: &str,
    errors: &mut Vec<String>,
) {
    if sources
        .iter()
        .any(|source| canonical_source_path(project_root, source).is_none())
    {
        errors.push(format!("{owner} contains an invalid source reference"));
    }
}

fn validate_optional_jsonl_sources(
    package_root: &Path,
    file_name: &str,
    project_root: &Path,
    errors: &mut Vec<String>,
) {
    let path = package_root.join(file_name);
    let text = match read_to_string(&path, MAX_FACTS_JSONL_BYTES as usize) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            errors.push(format!("failed to validate {file_name} source references"));
            return;
        }
    };
    for line in text.lines() {
        let Ok(row) = serde_json::from_str::<serde_json::Value>(line) else {
            errors.push(format!("{file_name} contains malformed JSONL"));
            return;
        };
        let Some(source) = row.get("source").and_then(serde_json::Value::as_str) else {
            errors.push(format!("{file_name} row is missing source"));
            return;
        };
        if canonical_source_path(project_root, source).is_none() {
            errors.push(format!("{file_name} contains an invalid source reference"));
            return;
        }
    }
}

pub(crate) fn canonical_source_path(project_root: &Path, source: &str) -> Option<String> {
    let (prefix, raw_path) = source.split_once(':')?;
    if prefix != "bronze" && prefix != "silver" {
        return None;
    }

    let relative_path = Path::new(raw_path);
    if raw_path.trim().is_empty() {
        return None;
    }
    let raw_bytes = raw_path.as_bytes();
    if relative_path.is_absolute()
        || raw_path.contains('\\')
        || raw_path.starts_with('\\')
        || raw_bytes.get(1) == Some(&b':')
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }

    let canonical_root = project_root.canonicalize().ok()?;
    let mut candidate = canonical_root.clone();
    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                candidate.push(part);
                if fs::symlink_metadata(&candidate)
                    .ok()?
                    .file_type()
                    .is_symlink()
                {
                    return None;
                }
            }
            _ => return None,
        }
    }

    let canonical = match candidate.canonicalize() {
        Ok(c) => c,
        Err(_) => return None,
    };
    let relative_path = canonical
        .strip_prefix(&canonical_root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    if relative_path.is_empty() {
        return None;
    }
    Some(format!("{prefix}:{relative_path}"))
}

#[cfg(test)]
#[path = "km_tests.rs"]
mod tests;
