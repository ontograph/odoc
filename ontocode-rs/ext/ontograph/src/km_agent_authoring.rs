//! Fail-closed validation for agent-authored KM candidates.
//!
//! Two independent gates run before a candidate may reach the live store:
//! citation proof, which requires committed in-project bytes at a declared
//! commit, and topic discrimination, which rejects generic project prose.
//! Both reject on any doubt; neither weakens `validate_km`.
//!
//! The validation entrypoints are exercised by this module's tests and are
//! wired into the staging path by KAS-2.1, which is the first production
//! caller.
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use crate::km_import::KmImportError;
use crate::km_import::KmImportResult;
use crate::km_import::run_km_import_for_agent_candidate;
#[cfg(test)]
#[path = "km_agent_authoring_tests.rs"]
mod tests;

/// Minimum distinct topic words a candidate must yield after stop-word
/// filtering. Frozen by ADR decision D1; lowering it to admit a failing
/// candidate reopens KAS-0.1.
pub(crate) const MIN_CANDIDATE_TOPIC_WORDS: usize = 6;

/// Maximum successfully activated agent-authored records per authoring thread.
/// Frozen by ADR decision D5. Withdrawn and superseded records still consume
/// the budget, so a thread cannot reset it by withdrawing.
pub(crate) const MAX_RECORDS_PER_AUTHORING_THREAD: usize = 5;

/// Stop words excluded from topic scoring, mirroring the store's list so a
/// candidate cannot pass here and then score differently at import.
const CANDIDATE_STOP_WORDS: &[&str] = &[
    "about", "after", "also", "and", "are", "before", "between", "for", "from", "into", "must",
    "should", "that", "their", "the", "then", "there", "this", "with",
];

/// Why a citation was refused. Each variant is a distinct, provable condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CitationRejection {
    /// The path escaped the project root, so it is not project-local evidence.
    OutsideProject,
    /// The path does not resolve to a file inside the project.
    Missing,
    /// Git does not track the path at the declared commit.
    NotCommitted,
    /// The working-tree bytes differ from the committed bytes.
    DirtyBytes,
    /// The declared commit is absent, unreadable, or not a real commit.
    UnknownCommit,
}

/// Why a candidate's topic words were refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TopicRejection {
    /// Fewer than `MIN_CANDIDATE_TOPIC_WORDS` distinct words survived filtering.
    TooGeneric { distinct_words: usize },
}

/// Verifies that `relative_path` is committed project-local evidence whose
/// working-tree bytes still match the declared commit.
///
/// Fails closed: a git invocation that cannot be run, cannot be parsed, or
/// reports failure is treated as unproven rather than accepted.
pub(crate) fn validate_citation(
    project_root: &Path,
    relative_path: &str,
    declared_commit: &str,
) -> Result<(), CitationRejection> {
    if !is_project_local(relative_path) {
        return Err(CitationRejection::OutsideProject);
    }
    if !project_root.join(relative_path).is_file() {
        return Err(CitationRejection::Missing);
    }
    // A lexically clean path can still resolve outside the project through a
    // symlink, so containment is proven against the canonical root.
    let resolved = resolve_inside_project(project_root, relative_path)
        .ok_or(CitationRejection::OutsideProject)?;
    if !commit_exists(project_root, declared_commit) {
        return Err(CitationRejection::UnknownCommit);
    }

    let committed = git_stdout(
        project_root,
        &["show", &format!("{declared_commit}:{relative_path}")],
    )
    .ok_or(CitationRejection::NotCommitted)?;

    let working = std::fs::read(&resolved).map_err(|_| CitationRejection::Missing)?;
    if committed != working {
        return Err(CitationRejection::DirtyBytes);
    }
    Ok(())
}

/// Rejects candidates whose prose is too generic to retrieve reliably.
pub(crate) fn validate_topic_discrimination(text: &str) -> Result<(), TopicRejection> {
    let distinct_words = candidate_topic_words(text).len();
    if distinct_words < MIN_CANDIDATE_TOPIC_WORDS {
        return Err(TopicRejection::TooGeneric { distinct_words });
    }
    Ok(())
}

/// Distinct lowercase topic words of at least three characters, excluding stop
/// words. Mirrors the store's word collection so scoring stays consistent.
pub(crate) fn candidate_topic_words(text: &str) -> Vec<String> {
    let mut words = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
            continue;
        }
        insert_word(&mut current, &mut words);
    }
    insert_word(&mut current, &mut words);
    words.into_iter().collect()
}

fn insert_word(current: &mut String, words: &mut BTreeSet<String>) {
    if current.len() >= 3 && !CANDIDATE_STOP_WORDS.contains(&current.as_str()) {
        words.insert(std::mem::take(current));
    } else {
        current.clear();
    }
}

/// Rejects absolute paths, parent traversal, and Windows-style roots so a
/// citation cannot point outside the project it claims to describe.
fn is_project_local(relative_path: &str) -> bool {
    if relative_path.is_empty() {
        return false;
    }
    let path = Path::new(relative_path);
    if path.is_absolute() {
        return false;
    }
    !path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

/// Resolves `relative_path` under `project_root` and proves the resolved bytes
/// stay inside it.
///
/// The lexical check alone is not enough: a symlink stored inside the project
/// can redirect an otherwise relative, traversal-free path outside the root, so
/// both sides are canonicalized and the prefix is required to hold.
pub(crate) fn resolve_inside_project(project_root: &Path, relative_path: &str) -> Option<PathBuf> {
    if !is_project_local(relative_path) {
        return None;
    }
    let root = std::fs::canonicalize(project_root).ok()?;
    let resolved = std::fs::canonicalize(root.join(relative_path)).ok()?;
    resolved.starts_with(&root).then_some(resolved)
}

fn commit_exists(project_root: &Path, commit: &str) -> bool {
    if commit.is_empty() {
        return false;
    }
    git_stdout(project_root, &["cat-file", "-t", commit])
        .is_some_and(|kind| kind.trim_ascii_end() == b"commit")
}

/// Runs one bounded `git` command, reusing the crate's existing subprocess
/// pattern rather than introducing a git library dependency.
fn git_stdout(project_root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(args)
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| output.stdout)
}

/// Why an agent authoring attempt was refused before the live store was touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivationRejection {
    /// The declared citation was not committed project-local evidence.
    Citation(CitationRejection),
    /// The candidate's prose was too generic to retrieve reliably.
    Topic(TopicRejection),
    /// The authoring thread already holds its full budget of records.
    ThreadCeiling { ceiling: usize, current: usize },
    /// The candidate could not be read, staged, or fully evaluated.
    Staging(KmImportError),
}

/// Validates and activates one agent-authored candidate.
///
/// Citation proof and topic discrimination run before the live store is opened.
/// The per-thread ceiling is enforced by the store itself, under the same lock
/// that appends the record, so concurrent authorings cannot both pass a stale
/// count. Staging and evaluation happen in a temporary store, so any rejection
/// leaves the live store unchanged.
pub(crate) fn activate_agent_candidate(
    store_root: &Path,
    project_root: &Path,
    thread_id: &str,
    candidate: &AgentCandidate<'_>,
) -> Result<KmImportResult, ActivationRejection> {
    validate_citation(
        project_root,
        candidate.citation_path,
        candidate.source_commit,
    )
    .map_err(ActivationRejection::Citation)?;
    validate_topic_discrimination(candidate.topic_text).map_err(ActivationRejection::Topic)?;

    run_km_import_for_agent_candidate(
        candidate.source_path,
        store_root,
        project_root,
        thread_id,
        MAX_RECORDS_PER_AUTHORING_THREAD,
    )
    .map_err(activation_error)
}

/// One agent-authored candidate awaiting validation.
pub(crate) struct AgentCandidate<'a> {
    /// Package directory to import.
    pub source_path: &'a Path,
    /// Project-relative path of the committed evidence backing the claim.
    pub citation_path: &'a str,
    /// Commit the citation bytes must match.
    pub source_commit: &'a str,
    /// Prose scored for topic discrimination.
    pub topic_text: &'a str,
}

/// Preserves the ceiling as a distinct outcome so the model is told it is out of
/// budget rather than that its candidate was malformed.
fn activation_error(error: KmImportError) -> ActivationRejection {
    match error {
        KmImportError::ThreadCeiling { ceiling, current } => {
            ActivationRejection::ThreadCeiling { ceiling, current }
        }
        other => ActivationRejection::Staging(other),
    }
}
