use std::fs;
use std::path::Path;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::indexer::RustIndexError;
use crate::indexer::collect_rust_files;
use crate::indexer::relative_slash_path;
use crate::indexer::strip_comments_and_strings;
use crate::tool::MAX_FILE_BYTES;

const LOGIC_LOOKAHEAD_LINES: usize = 6;
const MAX_SNIPPET_CHARS: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditLogicFinding {
    pub category: String,
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditLogicResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditLogicFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_logic_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditLogicResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let mut findings = Vec::new();
    let mut total_candidates = 0;

    for path in files {
        let Some(content) = read_rust_source(&path)? else {
            continue;
        };
        let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
        for finding in scan_logic_file(&relative_path, &content) {
            total_candidates += 1;
            if findings.len() < max_items {
                findings.push(finding);
            }
        }
    }

    let truncated = total_candidates > findings.len();
    Ok(AuditLogicResult {
        action: AuditAction::Logic,
        scanned_files,
        total_candidates,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("lookahead_lines={LOGIC_LOOKAHEAD_LINES}"),
        ],
        notes: vec![
            "This native slice scans Rust source files only; docs, facts, lifecycle state, and graph semantics are intentionally out of scope.".to_string(),
            "Implemented MVP categories: resource-leaks, toctou, concurrency.".to_string(),
            "Deferred donor categories for later slices: fork-safety and signals.".to_string(),
            "Findings are static evidence only and do not change audit lifecycle status.".to_string(),
        ],
    })
}

fn read_rust_source(path: &Path) -> Result<Option<String>, FunctionCallError> {
    let metadata = fs::metadata(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect a Rust source file: {err}"
        ))
    })?;
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Ok(None);
    }
    Ok(fs::read_to_string(path).ok())
}

fn scan_logic_file(path: &str, content: &str) -> Vec<AuditLogicFinding> {
    let raw_lines = content.lines().collect::<Vec<_>>();
    let sanitized_lines = raw_lines
        .iter()
        .map(|line| strip_comments_and_strings(line))
        .collect::<Vec<_>>();
    let mut findings = Vec::new();

    for (index, sanitized) in sanitized_lines.iter().enumerate() {
        if contains_resource_acquisition(sanitized) {
            findings.push(AuditLogicFinding {
                category: "resource-leaks".to_string(),
                rule: "rust_resource_acquisition_review".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                reason: "Resource acquisition is present; verify ownership, handoff, or cleanup because this MVP does not prove resource lifetime.".to_string(),
            });
        }

        if contains_toctou_check(sanitized)
            && let Some((target_index, _)) = find_following_file_use(index, &sanitized_lines)
        {
            findings.push(AuditLogicFinding {
                category: "toctou".to_string(),
                rule: "check_then_file_use_window".to_string(),
                path: path.to_string(),
                line: target_index + 1,
                snippet: truncate_chars(raw_lines[target_index].trim(), MAX_SNIPPET_CHARS),
                reason: format!(
                    "File use follows a prior existence or metadata check within {LOGIC_LOOKAHEAD_LINES} lines; verify the path cannot change between check and use."
                ),
            });
        }

        if contains_lock_acquisition(sanitized)
            && let Some((target_index, _)) = find_following_blocking_work(index, &sanitized_lines)
        {
            findings.push(AuditLogicFinding {
                category: "concurrency".to_string(),
                rule: "blocking_work_after_lock".to_string(),
                path: path.to_string(),
                line: target_index + 1,
                snippet: truncate_chars(raw_lines[target_index].trim(), MAX_SNIPPET_CHARS),
                reason: format!(
                    "Blocking I/O, sleep, or process work follows a visible lock acquisition within {LOGIC_LOOKAHEAD_LINES} lines; verify it does not run under the lock guard."
                ),
            });
        }
    }

    findings
}

fn contains_resource_acquisition(line: &str) -> bool {
    [
        "File::open",
        "File::create",
        "std::fs::File::open",
        "std::fs::File::create",
        "TcpStream::connect",
        "UdpSocket::bind",
        "UnixStream::connect",
        "UnixListener::bind",
    ]
    .iter()
    .any(|pattern| line.contains(pattern))
}

fn contains_toctou_check(line: &str) -> bool {
    [
        ".exists()",
        ".try_exists()",
        "fs::metadata(",
        "std::fs::metadata(",
    ]
    .iter()
    .any(|pattern| line.contains(pattern))
}

fn contains_lock_acquisition(line: &str) -> bool {
    line.contains(".lock()") || line.contains(".write()") || line.contains(".read()")
}

fn find_following_file_use(index: usize, lines: &[String]) -> Option<(usize, &str)> {
    find_in_window(index, lines, contains_file_use)
}

fn find_following_blocking_work(index: usize, lines: &[String]) -> Option<(usize, &str)> {
    find_in_window(index, lines, contains_blocking_work)
}

fn find_in_window(
    index: usize,
    lines: &[String],
    predicate: fn(&str) -> bool,
) -> Option<(usize, &str)> {
    let end = (index + LOGIC_LOOKAHEAD_LINES + 1).min(lines.len());
    lines
        .iter()
        .enumerate()
        .take(end)
        .skip(index + 1)
        .find_map(|(line_index, line)| predicate(line).then_some((line_index, line.as_str())))
}

fn contains_file_use(line: &str) -> bool {
    [
        "File::open",
        "File::create",
        "std::fs::read",
        "std::fs::write",
        "fs::read",
        "fs::write",
    ]
    .iter()
    .any(|pattern| line.contains(pattern))
}

fn contains_blocking_work(line: &str) -> bool {
    [
        "Command::new",
        "std::process::Command::new",
        "thread::sleep",
        "std::thread::sleep",
        "fs::read",
        "fs::write",
        "File::open",
        "File::create",
    ]
    .iter()
    .any(|pattern| line.contains(pattern))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn rust_index_error(err: RustIndexError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!(
        "ontograph.audit failed to inspect the Rust workspace state: {err:?}"
    ))
}
