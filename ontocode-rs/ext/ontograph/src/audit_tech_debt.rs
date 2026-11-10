use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::indexer::RustIndexError;
use crate::indexer::brace_delta;
use crate::indexer::collect_rust_files;
use crate::indexer::contains_call;
use crate::indexer::parse_rust_symbol_line;
use crate::indexer::relative_slash_path;
use crate::indexer::strip_comments_and_strings;
use crate::tool::MAX_FILE_BYTES;

pub(crate) const DEFAULT_TECH_DEBT_MIN_LINES: usize = 10;
pub(crate) const DEFAULT_TECH_DEBT_SINCE: &str = "6 months ago";
const MAX_TECH_DEBT_MIN_LINES: usize = 10_000;
const MAX_SINCE_CHARS: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTechDebtFinding {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub line_count: usize,
    pub parameter_count: usize,
    pub caller_count: usize,
    pub commit_count: usize,
    pub score_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTechDebtResult {
    pub action: AuditAction,
    pub since: String,
    pub min_lines: usize,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditTechDebtFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditHotspotFinding {
    pub path: String,
    pub function_count: usize,
    pub total_line_count: usize,
    pub total_caller_count: usize,
    pub commit_count: usize,
    pub score_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditHotspotsResult {
    pub action: AuditAction,
    pub since: String,
    pub scanned_files: usize,
    pub total_candidate_files: usize,
    pub findings: Vec<AuditHotspotFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditDeadCodeFinding {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditDeadCodeResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditDeadCodeFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ScannedRustFile {
    path: String,
    content: String,
    functions: Vec<DebtFunction>,
}

#[derive(Debug, Clone)]
struct DebtFunction {
    name: String,
    path: String,
    line: usize,
    end_line: usize,
    parameter_count: usize,
    is_public: bool,
}

#[derive(Debug, Clone)]
struct PendingFunction {
    name: String,
    line: usize,
    signature: String,
    brace_depth: i32,
    waiting_for_body: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FunctionRef {
    name: String,
    path: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionScope {
    name: String,
    path: String,
    line: usize,
    brace_depth: i32,
    waiting_for_body: bool,
}

pub(crate) fn run_tech_debt_audit(
    repo_root: &Path,
    max_items: usize,
    min_lines: usize,
    since: Option<&str>,
) -> Result<AuditTechDebtResult, FunctionCallError> {
    let min_lines = min_lines.clamp(1, MAX_TECH_DEBT_MIN_LINES);
    let since = normalize_since(since)?;
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let scanned = scan_rust_files(repo_root, files, min_lines)?;
    let total_candidates = scanned
        .iter()
        .map(|file| file.functions.len())
        .sum::<usize>();
    let caller_counts = collect_caller_counts(&scanned);
    let commit_counts = collect_commit_counts(repo_root, &scanned, &since);
    let used_git_history = !commit_counts.is_empty();

    let mut findings = scanned
        .iter()
        .flat_map(|file| {
            file.functions.iter().map(|function| {
                let line_count = function.end_line - function.line + 1;
                let caller_count = caller_counts
                    .get(&(function.path.clone(), function.name.clone()))
                    .copied()
                    .unwrap_or(0);
                let commit_count = commit_counts.get(&function.path).copied().unwrap_or(0);
                AuditTechDebtFinding {
                    symbol: function.name.clone(),
                    kind: "fn".to_string(),
                    path: function.path.clone(),
                    line: function.line,
                    line_count,
                    parameter_count: function.parameter_count,
                    caller_count,
                    commit_count,
                    score_millis: structural_score_millis(
                        line_count,
                        caller_count,
                        function.parameter_count,
                        commit_count,
                    ),
                }
            })
        })
        .collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        right
            .score_millis
            .cmp(&left.score_millis)
            .then_with(|| right.line_count.cmp(&left.line_count))
            .then_with(|| right.caller_count.cmp(&left.caller_count))
            .then_with(|| right.parameter_count.cmp(&left.parameter_count))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditTechDebtResult {
        action: AuditAction::TechDebt,
        since,
        min_lines,
        scanned_files,
        total_candidates,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("max_since_chars={MAX_SINCE_CHARS}"),
        ],
        notes: tech_debt_notes(used_git_history),
    })
}

pub(crate) fn run_hotspots_audit(
    repo_root: &Path,
    max_items: usize,
    since: Option<&str>,
) -> Result<AuditHotspotsResult, FunctionCallError> {
    let since = normalize_since(since)?;
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let scanned = scan_rust_files(repo_root, files, DEFAULT_TECH_DEBT_MIN_LINES)?;
    let total_candidate_files = scanned
        .iter()
        .filter(|file| !file.functions.is_empty())
        .count();
    let caller_counts = collect_caller_counts(&scanned);
    let commit_counts = collect_commit_counts(repo_root, &scanned, &since);
    let used_git_history = !commit_counts.is_empty();

    let mut findings = scanned
        .iter()
        .filter_map(|file| {
            if file.functions.is_empty() {
                return None;
            }

            let mut total_line_count = 0;
            let mut total_caller_count = 0;
            let mut score_millis = 0;
            let commit_count = commit_counts.get(&file.path).copied().unwrap_or(0);
            for function in &file.functions {
                let line_count = function.end_line - function.line + 1;
                let caller_count = caller_counts
                    .get(&(function.path.clone(), function.name.clone()))
                    .copied()
                    .unwrap_or(0);
                total_line_count += line_count;
                total_caller_count += caller_count;
                score_millis += structural_score_millis(
                    line_count,
                    caller_count,
                    function.parameter_count,
                    commit_count,
                );
            }

            Some(AuditHotspotFinding {
                path: file.path.clone(),
                function_count: file.functions.len(),
                total_line_count,
                total_caller_count,
                commit_count,
                score_millis,
            })
        })
        .collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        right
            .score_millis
            .cmp(&left.score_millis)
            .then_with(|| right.total_line_count.cmp(&left.total_line_count))
            .then_with(|| right.function_count.cmp(&left.function_count))
            .then_with(|| right.total_caller_count.cmp(&left.total_caller_count))
            .then_with(|| left.path.cmp(&right.path))
    });
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditHotspotsResult {
        action: AuditAction::Hotspots,
        since,
        scanned_files,
        total_candidate_files,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("default_min_function_lines={DEFAULT_TECH_DEBT_MIN_LINES}"),
            format!("max_since_chars={MAX_SINCE_CHARS}"),
        ],
        notes: hotspots_notes(used_git_history),
    })
}

pub(crate) fn run_dead_code_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditDeadCodeResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let scanned = scan_rust_files(repo_root, files, 1)?;
    let caller_counts = collect_caller_counts(&scanned);
    let name_counts = collect_name_counts(&scanned);

    let mut findings = scanned
        .iter()
        .flat_map(|file| {
            file.functions.iter().filter_map(|function| {
                if function.is_public {
                    return None;
                }
                if name_counts.get(&function.name).copied().unwrap_or(0) != 1 {
                    return None;
                }
                if caller_counts
                    .get(&(function.path.clone(), function.name.clone()))
                    .copied()
                    .unwrap_or(0)
                    != 0
                {
                    return None;
                }
                Some(AuditDeadCodeFinding {
                    symbol: function.name.clone(),
                    kind: "fn".to_string(),
                    path: function.path.clone(),
                    line: function.line,
                    line_count: function.end_line - function.line + 1,
                })
            })
        })
        .collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        right
            .line_count
            .cmp(&left.line_count)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.symbol.cmp(&right.symbol))
    });
    let total_candidates = findings.len();
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditDeadCodeResult {
        action: AuditAction::DeadCode,
        scanned_files,
        total_candidates,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
        ],
        notes: dead_code_notes(),
    })
}

fn scan_rust_files(
    repo_root: &Path,
    files: Vec<PathBuf>,
    min_lines: usize,
) -> Result<Vec<ScannedRustFile>, FunctionCallError> {
    files
        .into_iter()
        .map(|path| scan_rust_file(repo_root, path, min_lines))
        .collect()
}

fn scan_rust_file(
    repo_root: &Path,
    path: PathBuf,
    min_lines: usize,
) -> Result<ScannedRustFile, FunctionCallError> {
    let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
    let metadata = fs::metadata(&path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect a workspace file: {err}"
        ))
    })?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Ok(ScannedRustFile {
            path: relative_path,
            content: String::new(),
            functions: Vec::new(),
        });
    }

    let content = fs::read_to_string(&path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect a workspace file: {err}"
        ))
    })?;
    let functions = collect_functions(&content, min_lines)
        .into_iter()
        .map(|function| DebtFunction {
            name: function.name,
            path: relative_path.clone(),
            line: function.line,
            end_line: function.end_line,
            parameter_count: function.parameter_count,
            is_public: function.is_public,
        })
        .collect::<Vec<_>>();
    Ok(ScannedRustFile {
        path: relative_path,
        content,
        functions,
    })
}

fn collect_functions(content: &str, min_lines: usize) -> Vec<CollectedFunction> {
    let mut functions = Vec::new();
    let mut pending: Option<PendingFunction> = None;

    for (index, raw_line) in content.lines().enumerate() {
        let line_number = index + 1;
        let line = strip_comments_and_strings(raw_line);

        if let Some(current) = pending.as_mut() {
            current.signature.push(' ');
            current.signature.push_str(&line);
            if current.waiting_for_body {
                if line.contains(';') && !line.contains('{') {
                    pending = None;
                    continue;
                }
                if !line.contains('{') {
                    continue;
                }
                current.waiting_for_body = false;
            }
            current.brace_depth += brace_delta(&line);
            if current.brace_depth <= 0
                && let Some(current) = pending.take()
            {
                let parameter_count = count_signature_parameters(&current.signature);
                if line_number.saturating_sub(current.line) + 1 >= min_lines {
                    functions.push(CollectedFunction {
                        name: current.name,
                        line: current.line,
                        end_line: line_number,
                        parameter_count,
                        is_public: signature_is_public(&current.signature),
                    });
                }
            }
            continue;
        }

        if let Some((kind, name)) = parse_rust_symbol_line(&line)
            && kind == "fn"
        {
            let mut current = PendingFunction {
                name,
                line: line_number,
                signature: line.clone(),
                brace_depth: 0,
                waiting_for_body: !line.contains('{'),
            };
            if current.waiting_for_body {
                if line.contains(';') {
                    continue;
                }
                pending = Some(current);
                continue;
            }

            current.brace_depth = brace_delta(&line);
            if current.brace_depth <= 0 {
                if 1 >= min_lines {
                    functions.push(CollectedFunction {
                        name: current.name,
                        line: current.line,
                        end_line: line_number,
                        parameter_count: count_signature_parameters(&current.signature),
                        is_public: signature_is_public(&current.signature),
                    });
                }
            } else {
                pending = Some(current);
            }
        }
    }

    functions
}

#[derive(Debug, Clone)]
struct CollectedFunction {
    name: String,
    line: usize,
    end_line: usize,
    parameter_count: usize,
    is_public: bool,
}

fn count_signature_parameters(signature: &str) -> usize {
    let Some(start) = signature.find('(') else {
        return 0;
    };

    let mut current = String::new();
    let mut params = Vec::new();
    let mut paren_depth = 0;
    let mut angle_depth = 0;
    let mut bracket_depth = 0;
    for character in signature[start + 1..].chars() {
        match character {
            '(' => {
                paren_depth += 1;
                current.push(character);
            }
            ')' if paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    params.push(trimmed.to_string());
                }
                break;
            }
            ')' => {
                paren_depth -= 1;
                current.push(character);
            }
            '<' => {
                angle_depth += 1;
                current.push(character);
            }
            '>' if angle_depth > 0 => {
                angle_depth -= 1;
                current.push(character);
            }
            '[' => {
                bracket_depth += 1;
                current.push(character);
            }
            ']' if bracket_depth > 0 => {
                bracket_depth -= 1;
                current.push(character);
            }
            ',' if paren_depth == 0 && angle_depth == 0 && bracket_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    params.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(character),
        }
    }
    params.len()
}

fn signature_is_public(signature: &str) -> bool {
    signature.trim_start().starts_with("pub")
}

fn collect_name_counts(files: &[ScannedRustFile]) -> BTreeMap<String, usize> {
    files
        .iter()
        .flat_map(|file| &file.functions)
        .fold(BTreeMap::new(), |mut counts, function| {
            *counts.entry(function.name.clone()).or_insert(0) += 1;
            counts
        })
}

fn collect_caller_counts(files: &[ScannedRustFile]) -> BTreeMap<(String, String), usize> {
    let function_defs = files
        .iter()
        .flat_map(|file| {
            file.functions.iter().map(|function| FunctionRef {
                name: function.name.clone(),
                path: function.path.clone(),
                line: function.line,
            })
        })
        .collect::<Vec<_>>();
    let unique_names = collect_name_counts(files);

    let mut caller_counts = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for file in files {
        let functions_by_line = file
            .functions
            .iter()
            .map(|function| (function.line, function.name.clone()))
            .collect::<BTreeMap<_, _>>();
        let mut current_function: Option<FunctionScope> = None;
        for (index, raw_line) in file.content.lines().enumerate() {
            let line_number = index + 1;
            let line = strip_comments_and_strings(raw_line);
            if let Some(name) = functions_by_line.get(&line_number) {
                current_function = Some(FunctionScope {
                    name: name.clone(),
                    path: file.path.clone(),
                    line: line_number,
                    brace_depth: 0,
                    waiting_for_body: !line.contains('{'),
                });
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
            for callee in function_defs.iter().filter(|candidate| {
                unique_names.get(&candidate.name).copied().unwrap_or(0) == 1
                    && candidate.name != scope.name
                    && contains_call(&line, &candidate.name)
            }) {
                let key = (
                    scope.name.clone(),
                    scope.path.clone(),
                    scope.line,
                    callee.name.clone(),
                    callee.path.clone(),
                    callee.line,
                    line_number,
                );
                if seen.insert(key) {
                    *caller_counts
                        .entry((callee.path.clone(), callee.name.clone()))
                        .or_insert(0) += 1;
                }
            }
            scope.brace_depth += brace_delta(&line);
            if scope.brace_depth <= 0 {
                current_function = None;
            }
        }
    }
    caller_counts
}

fn collect_commit_counts(
    repo_root: &Path,
    files: &[ScannedRustFile],
    since: &str,
) -> BTreeMap<String, usize> {
    let tracked = files
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let Ok(output) = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(repo_root)
        .args([
            "log",
            "--since",
            since,
            "--name-only",
            "--format=tformat:",
            "--",
        ])
        .output()
    else {
        return BTreeMap::new();
    };
    if !output.status.success() {
        return BTreeMap::new();
    }

    let mut counts = BTreeMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let path = line.trim();
        if path.ends_with(".rs") && tracked.contains(path) {
            *counts.entry(path.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn structural_score_millis(
    line_count: usize,
    caller_count: usize,
    parameter_count: usize,
    commit_count: usize,
) -> u64 {
    let churn = ((commit_count as f64) + 1.0).log2().max(1.0);
    let score = (line_count as f64 / 20.0)
        * ((caller_count + 1) as f64)
        * (parameter_count as f64 / 3.0 + 1.0)
        * churn;
    (score * 1000.0).round() as u64
}

fn normalize_since(since: Option<&str>) -> Result<String, FunctionCallError> {
    let since = since.unwrap_or(DEFAULT_TECH_DEBT_SINCE).trim();
    if since.is_empty() {
        return Ok(DEFAULT_TECH_DEBT_SINCE.to_string());
    }
    if since.len() > MAX_SINCE_CHARS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit since exceeds the maximum of {MAX_SINCE_CHARS} characters"
        )));
    }
    Ok(since.to_string())
}

fn tech_debt_notes(used_git_history: bool) -> Vec<String> {
    let mut notes = vec![
        "This native slice ranks Rust functions only; other languages and docs are intentionally out of scope."
            .to_string(),
        "Caller counts reuse the existing unique-name Rust call heuristic, so duplicate function names stay conservative."
            .to_string(),
        "Scores are scaled by 1000 after combining line count, call-site count, parameter count, and git churn."
            .to_string(),
    ];
    if used_git_history {
        notes.push(
            "Git churn comes from local `git log --name-only` history inside the selected time window."
                .to_string(),
        );
    } else {
        notes.push(
            "Git churn was unavailable for this workspace view, so scores fell back to structural signals only."
                .to_string(),
        );
    }
    notes
}

fn hotspots_notes(used_git_history: bool) -> Vec<String> {
    let mut notes = vec![
        "This native slice ranks Rust files only; other languages and docs are intentionally out of scope."
            .to_string(),
        "File hotspots sum the same bounded structural signals already used by audit.tech_debt."
            .to_string(),
        "Only functions meeting the default minimum line threshold contribute to hotspot ranking."
            .to_string(),
    ];
    if used_git_history {
        notes.push(
            "Git churn comes from local `git log --name-only` history inside the selected time window."
                .to_string(),
        );
    } else {
        notes.push(
            "Git churn was unavailable for this workspace view, so hotspot scores fell back to structural signals only."
                .to_string(),
        );
    }
    notes
}

fn dead_code_notes() -> Vec<String> {
    vec![
        "This native slice reports Rust functions only; other languages and docs are intentionally out of scope."
            .to_string(),
        "Only unique-name private functions with zero observed callers under the current bounded Rust call heuristic are reported."
            .to_string(),
        "Public functions and duplicate function names are excluded to avoid overclaiming reachability."
            .to_string(),
    ]
}

fn rust_index_error(err: RustIndexError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!(
        "ontograph.audit failed to inspect the Rust workspace state: {}",
        match err {
            RustIndexError::Walk => "failed to walk the Rust workspace".to_string(),
            RustIndexError::ReadFile(path) => {
                format!("failed to read a Rust source file: {path}")
            }
            RustIndexError::Serialize => "failed to serialize the source index".to_string(),
            RustIndexError::WriteArtifact => {
                "failed to write the source index artifact".to_string()
            }
            RustIndexError::PublishManifest(_) => {
                "failed to publish the source index manifest".to_string()
            }
            RustIndexError::RebuildInProgress => {
                "another index rebuild is in progress".to_string()
            }
            RustIndexError::WriteLock => {
                "failed to acquire the index rebuild lock".to_string()
            }
            RustIndexError::DocsIndex(_) => {
                "failed to write the docs index artifact".to_string()
            }
        }
    ))
}

#[cfg(test)]
mod source_index_error_tests {
    use super::*;

    #[test]
    fn source_index_error_renders_relative_path_and_preserves_other_text() {
        let FunctionCallError::RespondToModel(read_file) =
            rust_index_error(RustIndexError::ReadFile("src/invalid.rs".to_string()))
        else {
            panic!("expected model-facing error");
        };
        assert_eq!(
            read_file,
            "ontograph.audit failed to inspect the Rust workspace state: failed to read a Rust source file: src/invalid.rs"
        );

        let FunctionCallError::RespondToModel(walk) = rust_index_error(RustIndexError::Walk) else {
            panic!("expected model-facing error");
        };
        assert_eq!(
            walk,
            "ontograph.audit failed to inspect the Rust workspace state: failed to walk the Rust workspace"
        );

        let FunctionCallError::RespondToModel(serialize) =
            rust_index_error(RustIndexError::Serialize)
        else {
            panic!("expected model-facing error");
        };
        assert_eq!(
            serialize,
            "ontograph.audit failed to inspect the Rust workspace state: failed to serialize the source index"
        );

        let FunctionCallError::RespondToModel(write_artifact) =
            rust_index_error(RustIndexError::WriteArtifact)
        else {
            panic!("expected model-facing error");
        };
        assert_eq!(
            write_artifact,
            "ontograph.audit failed to inspect the Rust workspace state: failed to write the source index artifact"
        );

        let FunctionCallError::RespondToModel(publish_manifest) = rust_index_error(
            RustIndexError::PublishManifest(crate::manifest::ArtifactPublishError::WriteManifest),
        ) else {
            panic!("expected model-facing error");
        };
        assert_eq!(
            publish_manifest,
            "ontograph.audit failed to inspect the Rust workspace state: failed to publish the source index manifest"
        );
    }
}
