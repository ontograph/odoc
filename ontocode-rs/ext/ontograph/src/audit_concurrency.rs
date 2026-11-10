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

const MAX_SNIPPET_CHARS: usize = 240;

/// Forward window, in lines, used to flag a second lock acquisition as a
/// nested-lock hint after a prior acquisition in the same file. Shallow and
/// heuristic; it does not prove that both guards are live at the same time.
const NESTED_LOCK_WINDOW_LINES: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditConcurrencyFinding {
    pub category: String,
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditConcurrencyResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditConcurrencyFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Default)]
struct ConcurrencyScan {
    total_candidates: usize,
    findings: Vec<AuditConcurrencyFinding>,
}

pub(crate) fn run_concurrency_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditConcurrencyResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let mut scan = ConcurrencyScan::default();

    for path in files {
        let Some(content) = read_rust_source(&path)? else {
            continue;
        };
        let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
        scan_concurrency_file(&relative_path, &content, max_items, &mut scan);
    }

    Ok(AuditConcurrencyResult {
        action: AuditAction::Concurrency,
        scanned_files,
        total_candidates: scan.total_candidates,
        truncated: scan.total_candidates > scan.findings.len(),
        findings: scan.findings,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("max_snippet_chars={MAX_SNIPPET_CHARS}"),
            format!("nested_lock_window_lines={NESTED_LOCK_WINDOW_LINES}"),
        ],
        notes: vec![
            "audit.concurrency is a shallow Rust source scan for lock-acquisition and nested-lock evidence only.".to_string(),
            "Blocking-work-under-lock is reported by audit.logic; data-race, deadlock, and happens-before proofs are intentionally out of scope.".to_string(),
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

fn scan_concurrency_file(path: &str, content: &str, max_items: usize, scan: &mut ConcurrencyScan) {
    let raw_lines = content.lines().collect::<Vec<_>>();
    let mut previous_lock_line: Option<usize> = None;

    for (index, raw) in raw_lines.iter().enumerate() {
        let sanitized = strip_comments_and_strings(raw);
        if !contains_lock_acquisition(&sanitized) {
            continue;
        }

        push_finding(
            scan,
            max_items,
            AuditConcurrencyFinding {
                category: "lock_scope".to_string(),
                rule: "lock_acquisition".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw.trim(), MAX_SNIPPET_CHARS),
                reason: "Explicit lock acquisition; verify the guard scope stays bounded and does not hold across await or blocking work.".to_string(),
            },
        );

        if let Some(previous) = previous_lock_line
            && index - previous <= NESTED_LOCK_WINDOW_LINES
        {
            push_finding(
                scan,
                max_items,
                AuditConcurrencyFinding {
                    category: "lock_scope".to_string(),
                    rule: "nested_lock_hint".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw.trim(), MAX_SNIPPET_CHARS),
                    reason: format!(
                        "A second lock acquisition appears within {NESTED_LOCK_WINDOW_LINES} lines of a prior one; verify the earlier guard is released to avoid a nested-lock ordering hazard."
                    ),
                },
            );
        }

        previous_lock_line = Some(index);
    }
}

fn push_finding(scan: &mut ConcurrencyScan, max_items: usize, finding: AuditConcurrencyFinding) {
    scan.total_candidates += 1;
    if scan.findings.len() < max_items {
        scan.findings.push(finding);
    }
}

fn contains_lock_acquisition(line: &str) -> bool {
    line.contains(".lock()") || line.contains(".write()") || line.contains(".read()")
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn rust_index_error(err: RustIndexError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!("ontograph.audit source scan failed: {err:?}"))
}
