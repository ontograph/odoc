use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::tool::MAX_FILE_BYTES;

const DEFAULT_WINDOW_LINES: usize = 20;
const MAX_WINDOW_LINES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditPathVerifyStatus {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditPathVerifyFindingKind {
    MissingRequiredPattern,
    ForbiddenPatternPresent,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPathVerifyFinding {
    pub kind: AuditPathVerifyFindingKind,
    pub pattern: String,
    pub line: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditPathVerifyResult {
    pub action: AuditAction,
    pub status: AuditPathVerifyStatus,
    pub path: String,
    pub trigger_pattern: String,
    pub trigger_line: Option<usize>,
    pub checked_lines: usize,
    pub findings: Vec<AuditPathVerifyFinding>,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) struct PathVerifyAuditInput {
    pub repo_root: PathBuf,
    pub path: Option<String>,
    pub trigger_pattern: Option<String>,
    pub required_patterns: Option<Vec<String>>,
    pub forbidden_patterns: Option<Vec<String>>,
    pub window_lines: Option<usize>,
}

pub(crate) fn run_path_verify_audit(
    input: PathVerifyAuditInput,
) -> Result<AuditPathVerifyResult, FunctionCallError> {
    let path = required_non_empty(input.path, "path")?;
    let trigger_pattern = required_non_empty(input.trigger_pattern, "trigger_pattern")?;
    let required_patterns = normalize_patterns(input.required_patterns, "required_patterns")?;
    let forbidden_patterns = normalize_patterns(input.forbidden_patterns, "forbidden_patterns")?;
    let window_lines = input
        .window_lines
        .unwrap_or(DEFAULT_WINDOW_LINES)
        .clamp(1, MAX_WINDOW_LINES);
    let source_path = resolve_repo_relative_path(&input.repo_root, &path)?;
    let source = read_bounded_source(&source_path)?;
    let lines = source.lines().collect::<Vec<_>>();

    let Some(trigger_index) = lines
        .iter()
        .position(|line| line.contains(&trigger_pattern))
    else {
        return Ok(AuditPathVerifyResult {
            action: AuditAction::PathVerify,
            status: AuditPathVerifyStatus::Inconclusive,
            path,
            trigger_pattern,
            trigger_line: None,
            checked_lines: 0,
            findings: Vec::new(),
            limits: path_verify_limits(window_lines),
            notes: vec![
                "Trigger pattern was not found; no path window was evaluated.".to_string(),
                path_verify_contract_note(),
            ],
        });
    };

    let window_start = trigger_index + 1;
    let window_end = (window_start + window_lines).min(lines.len());
    let window = &lines[window_start..window_end];
    let mut findings = Vec::new();

    for pattern in required_patterns {
        if !window.iter().any(|line| line.contains(&pattern)) {
            findings.push(AuditPathVerifyFinding {
                kind: AuditPathVerifyFindingKind::MissingRequiredPattern,
                pattern: pattern.clone(),
                line: None,
                detail: format!(
                    "required pattern `{pattern}` was not found after trigger within the bounded window"
                ),
            });
        }
    }
    for pattern in forbidden_patterns {
        if let Some((offset, _)) = window
            .iter()
            .enumerate()
            .find(|(_, line)| line.contains(&pattern))
        {
            findings.push(AuditPathVerifyFinding {
                kind: AuditPathVerifyFindingKind::ForbiddenPatternPresent,
                pattern: pattern.clone(),
                line: Some(window_start + offset + 1),
                detail: format!(
                    "forbidden pattern `{pattern}` was found after trigger within the bounded window"
                ),
            });
        }
    }

    let status = if findings.is_empty() {
        AuditPathVerifyStatus::Pass
    } else {
        AuditPathVerifyStatus::Fail
    };
    Ok(AuditPathVerifyResult {
        action: AuditAction::PathVerify,
        status,
        path,
        trigger_pattern,
        trigger_line: Some(trigger_index + 1),
        checked_lines: window.len(),
        findings,
        limits: path_verify_limits(window_lines),
        notes: vec![path_verify_contract_note()],
    })
}

fn required_non_empty(value: Option<String>, field: &str) -> Result<String, FunctionCallError> {
    let value = value.unwrap_or_default();
    if value.trim().is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field} is required for action `path_verify`"
        )));
    }
    Ok(value)
}

fn normalize_patterns(
    patterns: Option<Vec<String>>,
    field: &str,
) -> Result<Vec<String>, FunctionCallError> {
    let patterns = patterns.unwrap_or_default();
    if patterns.iter().any(|pattern| pattern.trim().is_empty()) {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit {field} must not contain blank patterns"
        )));
    }
    Ok(patterns)
}

fn resolve_repo_relative_path(repo_root: &Path, path: &str) -> Result<PathBuf, FunctionCallError> {
    let candidate = Path::new(path);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit path must be a repo-relative file path without traversal".to_string(),
        ));
    }
    Ok(repo_root.join(candidate))
}

fn read_bounded_source(path: &Path) -> Result<String, FunctionCallError> {
    let metadata = fs::metadata(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to inspect path_verify file: {err}"
        ))
    })?;
    if !metadata.is_file() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit path_verify path must point to a file".to_string(),
        ));
    }
    if metadata.len() as usize > MAX_FILE_BYTES {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit path_verify file exceeds max_file_bytes={MAX_FILE_BYTES}"
        )));
    }
    fs::read_to_string(path).map_err(|err| {
        FunctionCallError::RespondToModel(format!(
            "ontograph.audit failed to read path_verify file: {err}"
        ))
    })
}

fn path_verify_limits(window_lines: usize) -> Vec<String> {
    vec![
        format!("window_lines={window_lines}"),
        format!("max_window_lines={MAX_WINDOW_LINES}"),
        format!("max_file_bytes={MAX_FILE_BYTES}"),
    ]
}

fn path_verify_contract_note() -> String {
    "audit.path_verify is a bounded substring window check, not path-sensitive proof.".to_string()
}
