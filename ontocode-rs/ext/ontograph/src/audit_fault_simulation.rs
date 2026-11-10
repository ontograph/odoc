use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

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

const DEFAULT_WINDOW_LINES: usize = 8;
const MAX_WINDOW_LINES: usize = 80;
const MAX_LABEL_CHARS: usize = 120;
const MAX_SNIPPET_CHARS: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditFaultSimulationStatus {
    Evidence,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFaultSimulationFinding {
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub target_line: usize,
    pub target_call: String,
    pub modeled_return_value: String,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFaultSimulationSkippedRecord {
    pub category: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub snippet: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFaultSimulationResult {
    pub action: AuditAction,
    pub status: AuditFaultSimulationStatus,
    pub target_call: String,
    pub modeled_return_value: String,
    pub scanned_files: usize,
    pub target_sites: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditFaultSimulationFinding>,
    pub skipped: Vec<AuditFaultSimulationSkippedRecord>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) struct FaultSimulationAuditInput {
    pub repo_root: PathBuf,
    pub path: Option<String>,
    pub target_call: Option<String>,
    pub return_value: Option<String>,
    pub max_items: usize,
    pub window_lines: Option<usize>,
}

#[derive(Debug, Default)]
struct FaultSimulationScan {
    target_sites: usize,
    total_candidates: usize,
    findings: Vec<AuditFaultSimulationFinding>,
    skipped: Vec<AuditFaultSimulationSkippedRecord>,
}

pub(crate) fn run_fault_simulation_audit(
    input: FaultSimulationAuditInput,
) -> Result<AuditFaultSimulationResult, FunctionCallError> {
    let target_call = required_label(input.target_call, "target_call")?;
    let return_value = required_label(input.return_value, "return_value")?;
    let window_lines = input
        .window_lines
        .unwrap_or(DEFAULT_WINDOW_LINES)
        .clamp(1, MAX_WINDOW_LINES);
    let files = scan_files(&input.repo_root, input.path.as_deref())?;
    let scanned_files = files.len();
    let mut scan = FaultSimulationScan::default();

    for path in files {
        let source = read_bounded_source(&path)?;
        let relative_path =
            relative_slash_path(&input.repo_root, &path).map_err(rust_index_error)?;
        scan_fault_file(
            &relative_path,
            &source,
            &target_call,
            &return_value,
            input.max_items,
            window_lines,
            &mut scan,
        );
    }

    if scan.target_sites == 0 {
        scan.skipped.push(AuditFaultSimulationSkippedRecord {
            category: "inconclusive".to_string(),
            path: input.path,
            line: None,
            snippet: None,
            reason: "Target call was not found; no static fault window was evaluated.".to_string(),
        });
    }

    let emitted = scan.findings.len() + scan.skipped.len();
    Ok(AuditFaultSimulationResult {
        action: AuditAction::SimulateFault,
        status: if scan.target_sites == 0 {
            AuditFaultSimulationStatus::Inconclusive
        } else {
            AuditFaultSimulationStatus::Evidence
        },
        target_call,
        modeled_return_value: return_value,
        scanned_files,
        target_sites: scan.target_sites,
        total_candidates: scan.total_candidates,
        findings: scan.findings,
        skipped: scan.skipped,
        truncated: scan.total_candidates > emitted,
        limits: vec![
            format!("max_items={}", input.max_items),
            format!("window_lines={window_lines}"),
            format!("max_window_lines={MAX_WINDOW_LINES}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("max_label_chars={MAX_LABEL_CHARS}"),
        ],
        notes: vec![
            "audit.simulate_fault is a shallow Rust source scan, not dynamic execution or symbolic proof.".to_string(),
            "The caller-supplied return_value is reported only as a label on nearby structural evidence.".to_string(),
            "Missing target calls are inconclusive and must not be treated as runtime proof.".to_string(),
        ],
    })
}

fn required_label(value: Option<String>, field: &str) -> Result<String, FunctionCallError> {
    let value = value.unwrap_or_default();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field} is required for action `simulate_fault`"
        )));
    }
    if trimmed.chars().count() > MAX_LABEL_CHARS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field} exceeds max_label_chars={MAX_LABEL_CHARS}"
        )));
    }
    Ok(trimmed.to_string())
}

fn scan_files(repo_root: &Path, path: Option<&str>) -> Result<Vec<PathBuf>, FunctionCallError> {
    let Some(path) = path else {
        return collect_rust_files(repo_root).map_err(rust_index_error);
    };
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || candidate.extension().and_then(|ext| ext.to_str()) != Some("rs")
    {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit path must be one repo-relative Rust file without traversal"
                .to_string(),
        ));
    }
    Ok(vec![repo_root.join(candidate)])
}

fn read_bounded_source(path: &Path) -> Result<String, FunctionCallError> {
    let metadata = fs::metadata(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect simulate_fault file: {err}"
        ))
    })?;
    if !metadata.is_file() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit simulate_fault path must point to a file".to_string(),
        ));
    }
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit simulate_fault file exceeds max_file_bytes={MAX_FILE_BYTES}"
        )));
    }
    fs::read_to_string(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to read simulate_fault file: {err}"
        ))
    })
}

fn scan_fault_file(
    path: &str,
    content: &str,
    target_call: &str,
    return_value: &str,
    max_items: usize,
    window_lines: usize,
    scan: &mut FaultSimulationScan,
) {
    let raw_lines = content.lines().collect::<Vec<_>>();
    let sanitized_lines = raw_lines
        .iter()
        .map(|line| strip_comments_and_strings(line))
        .collect::<Vec<_>>();
    let target_pattern = format!("{target_call}(");

    for (target_index, sanitized) in sanitized_lines.iter().enumerate() {
        if !sanitized.contains(&target_pattern) {
            continue;
        }
        scan.target_sites += 1;
        let window_start = target_index.saturating_sub(window_lines);
        let window_end = (target_index + window_lines + 1).min(raw_lines.len());
        for index in window_start..window_end {
            if let Some((kind, reason)) = classify_structural_line(&sanitized_lines[index]) {
                push_finding(
                    scan,
                    max_items,
                    AuditFaultSimulationFinding {
                        kind: kind.to_string(),
                        path: path.to_string(),
                        line: index + 1,
                        target_line: target_index + 1,
                        target_call: target_call.to_string(),
                        modeled_return_value: return_value.to_string(),
                        snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                        reason: reason.to_string(),
                    },
                );
            }
        }
    }
}

fn classify_structural_line(line: &str) -> Option<(&'static str, &'static str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("if ")
        || trimmed.starts_with("if let ")
        || trimmed.starts_with("match ")
        || trimmed.starts_with("else if ")
    {
        return Some((
            "branch",
            "Branch near target call may depend on or handle the modeled return value.",
        ));
    }
    if trimmed.starts_with("return ") || trimmed == "return;" || trimmed.contains('?') {
        return Some((
            "early_return",
            "Early return near target call may be a fault propagation point.",
        ));
    }
    if is_assignment_like(trimmed) {
        return Some((
            "assignment",
            "Assignment near target call may store or transform the modeled return value.",
        ));
    }
    None
}

fn is_assignment_like(line: &str) -> bool {
    line.contains('=')
        && !line.contains("==")
        && !line.contains("!=")
        && !line.contains("<=")
        && !line.contains(">=")
        && !line.contains("=>")
}

fn push_finding(
    scan: &mut FaultSimulationScan,
    max_items: usize,
    finding: AuditFaultSimulationFinding,
) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.findings.push(finding);
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn rust_index_error(err: RustIndexError) -> FunctionCallError {
    FunctionCallError::RespondToModel(format!("ontograph.audit source scan failed: {err:?}"))
}
