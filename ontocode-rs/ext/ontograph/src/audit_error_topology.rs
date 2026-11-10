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
pub(crate) struct AuditErrorTopologyFinding {
    pub category: String,
    pub rule: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditErrorTopologySkippedRecord {
    pub category: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditErrorTopologyResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditErrorTopologyFinding>,
    pub skipped: Vec<AuditErrorTopologySkippedRecord>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
struct ErrorRule {
    category: &'static str,
    rule: &'static str,
    reason: &'static str,
    predicate: fn(&str) -> bool,
}

#[derive(Debug, Default)]
struct ErrorTopologyScan {
    total_candidates: usize,
    findings: Vec<AuditErrorTopologyFinding>,
    skipped: Vec<AuditErrorTopologySkippedRecord>,
}

pub(crate) fn run_error_topology_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditErrorTopologyResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let mut scan = ErrorTopologyScan::default();

    for path in files {
        let Some(content) = read_rust_source(&path)? else {
            continue;
        };
        let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
        scan_error_topology_file(&relative_path, &content, max_items, &mut scan);
    }

    let emitted = scan.findings.len() + scan.skipped.len();
    Ok(AuditErrorTopologyResult {
        action: AuditAction::ErrorTopology,
        scanned_files,
        total_candidates: scan.total_candidates,
        findings: scan.findings,
        skipped: scan.skipped,
        truncated: scan.total_candidates > emitted,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
        ],
        notes: vec![
            "This native slice scans Rust source files only; graph facts, runtime paths, and donor semantics are intentionally out of scope.".to_string(),
            "Findings are shallow syntax evidence for explicit error sources, checks, returns, and swallowed-error candidates only.".to_string(),
            "Unsupported or ambiguous error-like patterns are emitted as skipped records instead of being inferred.".to_string(),
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

fn scan_error_topology_file(
    path: &str,
    content: &str,
    max_items: usize,
    scan: &mut ErrorTopologyScan,
) {
    let raw_lines = content.lines().collect::<Vec<_>>();
    let sanitized_lines = raw_lines
        .iter()
        .map(|line| strip_comments_and_strings(line))
        .collect::<Vec<_>>();
    let rules = error_rules();

    for (index, sanitized) in sanitized_lines.iter().enumerate() {
        let matches = rules
            .iter()
            .filter(|rule| (rule.predicate)(sanitized))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] if contains_unsupported_error_like_pattern(sanitized) => push_skipped(
                scan,
                max_items,
                AuditErrorTopologySkippedRecord {
                    category: "unsupported".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                    reason: "Error-like panic or assertion syntax is not an error topology edge."
                        .to_string(),
                },
            ),
            [] => {}
            [rule] => push_finding(
                scan,
                max_items,
                AuditErrorTopologyFinding {
                    category: rule.category.to_string(),
                    rule: rule.rule.to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                    reason: rule.reason.to_string(),
                },
            ),
            _ => push_skipped(
                scan,
                max_items,
                AuditErrorTopologySkippedRecord {
                    category: "ambiguous".to_string(),
                    path: path.to_string(),
                    line: index + 1,
                    snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                    reason:
                        "Line matches multiple error-topology categories; this MVP does not infer edge direction."
                            .to_string(),
                },
            ),
        }
    }
}

fn error_rules() -> Vec<ErrorRule> {
    vec![
        ErrorRule {
            category: "source",
            rule: "explicit_error_construction",
            reason: "Explicit error construction is a local error source candidate.",
            predicate: contains_error_source,
        },
        ErrorRule {
            category: "check",
            rule: "explicit_error_check",
            reason: "Explicit error check is a local error handling branch candidate.",
            predicate: contains_error_check,
        },
        ErrorRule {
            category: "return",
            rule: "explicit_error_return",
            reason: "Explicit error return or propagation is a local error sink candidate.",
            predicate: contains_error_return,
        },
        ErrorRule {
            category: "swallowed",
            rule: "discarded_error_candidate",
            reason: "Error-capable result appears discarded or converted away; verify this is intentional.",
            predicate: contains_swallowed_error_candidate,
        },
    ]
}

fn contains_error_source(line: &str) -> bool {
    line.contains("anyhow!(")
        || line.contains("bail!(")
        || line.contains("FunctionCallError::")
        || line.contains("Error::")
}

fn contains_error_check(line: &str) -> bool {
    line.contains(".is_err()") || line.contains("if let Err(") || line.contains("match ")
}

fn contains_error_return(line: &str) -> bool {
    line.trim_start().starts_with("return Err(")
        || line.contains("=> Err(")
        || line.trim_end().ends_with("?)")
        || line.trim_end().ends_with("?;")
}

fn contains_swallowed_error_candidate(line: &str) -> bool {
    line.contains(".ok()")
        || line.contains(".unwrap_or(")
        || line.contains(".unwrap_or_else(")
        || line.trim_start().starts_with("let _ =")
}

fn contains_unsupported_error_like_pattern(line: &str) -> bool {
    line.contains("panic!(") || line.contains("assert!(") || line.contains("debug_assert!(")
}

fn push_finding(
    scan: &mut ErrorTopologyScan,
    max_items: usize,
    finding: AuditErrorTopologyFinding,
) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.findings.push(finding);
    }
}

fn push_skipped(
    scan: &mut ErrorTopologyScan,
    max_items: usize,
    skipped: AuditErrorTopologySkippedRecord,
) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.skipped.push(skipped);
    }
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
