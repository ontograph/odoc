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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPressureFinding {
    pub category: String,
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPressureSkippedRecord {
    pub category: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPressureResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditPressureFinding>,
    pub skipped: Vec<AuditPressureSkippedRecord>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Default)]
struct PressureScan {
    total_candidates: usize,
    findings: Vec<AuditPressureFinding>,
    skipped: Vec<AuditPressureSkippedRecord>,
}

pub(crate) fn run_pressure_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditPressureResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let mut scan = PressureScan::default();

    for path in files {
        let Some(content) = read_rust_source(&path)? else {
            continue;
        };
        let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
        scan_pressure_file(&relative_path, &content, max_items, &mut scan);
    }

    let emitted = scan.findings.len() + scan.skipped.len();
    Ok(AuditPressureResult {
        action: AuditAction::Pressure,
        scanned_files,
        total_candidates: scan.total_candidates,
        findings: scan.findings,
        skipped: scan.skipped,
        truncated: scan.total_candidates > emitted,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("max_snippet_chars={MAX_SNIPPET_CHARS}"),
        ],
        notes: vec![
            "audit.pressure is a shallow Rust source scan for quota and global side-effect evidence only.".to_string(),
            "Concurrency lock analysis, resource tracing, taint, and fault simulation are intentionally out of scope.".to_string(),
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

fn scan_pressure_file(path: &str, content: &str, max_items: usize, scan: &mut PressureScan) {
    let raw_lines = content.lines().collect::<Vec<_>>();
    for (index, raw) in raw_lines.iter().enumerate() {
        let sanitized = strip_comments_and_strings(raw);
        if contains_quota_pressure(&sanitized) {
            push_finding(
                scan,
                max_items,
                AuditPressureFinding {
                    category: "quota".to_string(),
                    rule: "quota_or_capacity_bound".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw.trim(), MAX_SNIPPET_CHARS),
                    reason: "Quota-like max/limit/capacity check near count-like state; verify global pressure impact.".to_string(),
                },
            );
        } else if contains_ambiguous_quota(&sanitized) {
            push_skipped(
                scan,
                max_items,
                AuditPressureSkippedRecord {
                    category: "quota".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw.trim(), MAX_SNIPPET_CHARS),
                    reason:
                        "Quota-like wording lacks an explicit bound or count-like state in this shallow scan."
                            .to_string(),
                },
            );
        }

        if contains_global_side_effect(&sanitized) {
            push_finding(
                scan,
                max_items,
                AuditPressureFinding {
                    category: "global_side_effect".to_string(),
                    rule: "process_global_mutation".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw.trim(), MAX_SNIPPET_CHARS),
                    reason: "Process-global state mutation can affect all concurrent work."
                        .to_string(),
                },
            );
        }
    }
}

fn push_finding(scan: &mut PressureScan, max_items: usize, finding: AuditPressureFinding) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.findings.push(finding);
    }
}

fn push_skipped(scan: &mut PressureScan, max_items: usize, skipped: AuditPressureSkippedRecord) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.skipped.push(skipped);
    }
}

fn contains_quota_pressure(line: &str) -> bool {
    contains_any(line, &["max", "limit", "capacity"])
        && contains_any(line, &[">", "<", "==", "!="])
        && contains_any(
            line,
            &[
                "active",
                "pending",
                "queued",
                "in_flight",
                "concurrent",
                "count",
            ],
        )
}

fn contains_ambiguous_quota(line: &str) -> bool {
    contains_any(line, &["quota", "capacity", "limit"])
}

fn contains_global_side_effect(line: &str) -> bool {
    contains_any(
        line,
        &[
            "set_var(",
            "remove_var(",
            "set_current_dir(",
            "process::exit(",
            "std::process::exit(",
        ],
    ) || (line.contains("static mut") && line.contains('='))
}

fn contains_any(line: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| line.contains(needle))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn rust_index_error(err: RustIndexError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!("ontograph.audit source scan failed: {err:?}"))
}
