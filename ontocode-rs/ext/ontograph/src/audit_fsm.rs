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

const FSM_LOOKAHEAD_LINES: usize = 8;
const MAX_SNIPPET_CHARS: usize = 240;
const STATE_NAME_HINTS: [&str; 4] = ["state", "status", "phase", "mode"];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFsmFinding {
    pub kind: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub state: String,
    pub source_state: Option<String>,
    pub target_state: Option<String>,
    pub variable: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFsmSkippedRecord {
    pub category: String,
    pub path: String,
    pub line: usize,
    pub snippet: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditFsmResult {
    pub action: AuditAction,
    pub scanned_files: usize,
    pub total_candidates: usize,
    pub findings: Vec<AuditFsmFinding>,
    pub skipped: Vec<AuditFsmSkippedRecord>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Default)]
struct FsmScan {
    total_candidates: usize,
    findings: Vec<AuditFsmFinding>,
    skipped: Vec<AuditFsmSkippedRecord>,
}

#[derive(Debug, Clone)]
struct EnumScope {
    name: String,
    brace_depth: i32,
}

pub(crate) fn run_fsm_audit(
    repo_root: &Path,
    max_items: usize,
) -> Result<AuditFsmResult, FunctionCallError> {
    let files = collect_rust_files(repo_root).map_err(rust_index_error)?;
    let scanned_files = files.len();
    let mut scan = FsmScan::default();

    for path in files {
        let Some(content) = read_rust_source(&path)? else {
            continue;
        };
        let relative_path = relative_slash_path(repo_root, &path).map_err(rust_index_error)?;
        scan_fsm_file(&relative_path, &content, max_items, &mut scan);
    }

    let emitted = scan.findings.len() + scan.skipped.len();
    Ok(AuditFsmResult {
        action: AuditAction::ExtractFsm,
        scanned_files,
        total_candidates: scan.total_candidates,
        findings: scan.findings,
        skipped: scan.skipped,
        truncated: scan.total_candidates > emitted,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
            format!("lookahead_lines={FSM_LOOKAHEAD_LINES}"),
        ],
        notes: vec![
            "This native slice scans Rust source files only; docs, graph facts, lifecycle state, and donor runtime semantics are intentionally out of scope.".to_string(),
            "Findings are shallow syntax evidence for state-like enums, explicit state assignments, and branch-to-assignment transition candidates only.".to_string(),
            "Unsupported or ambiguous state updates are emitted as skipped records instead of being inferred.".to_string(),
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

fn scan_fsm_file(path: &str, content: &str, max_items: usize, scan: &mut FsmScan) {
    let raw_lines = content.lines().collect::<Vec<_>>();
    let sanitized_lines = raw_lines
        .iter()
        .map(|line| strip_comments_and_strings(line))
        .collect::<Vec<_>>();
    let mut enum_scope = None;

    for (index, sanitized) in sanitized_lines.iter().enumerate() {
        update_enum_scope(
            path,
            index,
            &raw_lines,
            sanitized,
            max_items,
            scan,
            &mut enum_scope,
        );
        scan_assignment(path, index, &raw_lines, sanitized, max_items, scan);
        scan_transition(path, index, &raw_lines, &sanitized_lines, max_items, scan);
    }
}

fn update_enum_scope(
    path: &str,
    index: usize,
    raw_lines: &[&str],
    sanitized: &str,
    max_items: usize,
    scan: &mut FsmScan,
    enum_scope: &mut Option<EnumScope>,
) {
    if enum_scope.is_none()
        && let Some(name) = parse_state_like_enum_name(sanitized)
    {
        *enum_scope = Some(EnumScope {
            name,
            brace_depth: brace_delta(sanitized),
        });
        return;
    }

    let Some(scope) = enum_scope else {
        return;
    };
    if let Some(variant) = parse_enum_variant(sanitized) {
        push_finding(
            scan,
            max_items,
            AuditFsmFinding {
                kind: "enum_state".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                state: format!("{}::{variant}", scope.name),
                source_state: None,
                target_state: None,
                variable: None,
                reason: "State-like enum variant declared explicitly.".to_string(),
            },
        );
    }
    scope.brace_depth += brace_delta(sanitized);
    if scope.brace_depth <= 0 {
        *enum_scope = None;
    }
}

fn scan_assignment(
    path: &str,
    index: usize,
    raw_lines: &[&str],
    sanitized: &str,
    max_items: usize,
    scan: &mut FsmScan,
) {
    if sanitized.contains("=>") {
        return;
    }
    let Some((left, right)) = split_assignment(sanitized) else {
        return;
    };
    if !is_state_like_name(left) {
        return;
    }

    let targets = qualified_states(right);
    match targets.as_slice() {
        [target] => push_finding(
            scan,
            max_items,
            AuditFsmFinding {
                kind: "state_assignment".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                state: target.clone(),
                source_state: None,
                target_state: Some(target.clone()),
                variable: Some(left.trim().to_string()),
                reason: "State-like variable is assigned an explicit enum variant.".to_string(),
            },
        ),
        [] => push_skipped(
            scan,
            max_items,
            AuditFsmSkippedRecord {
                category: "unsupported".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                reason: "State-like assignment target is not an explicit enum variant.".to_string(),
            },
        ),
        _ => push_skipped(
            scan,
            max_items,
            AuditFsmSkippedRecord {
                category: "ambiguous".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                reason: "State-like assignment mentions multiple enum variants.".to_string(),
            },
        ),
    }
}

fn scan_transition(
    path: &str,
    index: usize,
    raw_lines: &[&str],
    sanitized_lines: &[String],
    max_items: usize,
    scan: &mut FsmScan,
) {
    let sanitized = sanitized_lines[index].trim();
    if let Some((left, right)) = sanitized.split_once("=>") {
        report_transition_parts(path, index, raw_lines, left, right, max_items, scan);
        return;
    }

    if !is_guard_line(sanitized) {
        return;
    }

    let sources = qualified_states(sanitized);
    match sources.as_slice() {
        [source] => {
            if let Some((target_index, target)) =
                find_following_state_assignment(index, sanitized_lines)
            {
                push_finding(
                    scan,
                    max_items,
                    AuditFsmFinding {
                        kind: "transition_candidate".to_string(),
                        path: path.to_string(),
                        line: target_index + 1,
                        snippet: truncate_chars(raw_lines[target_index].trim(), MAX_SNIPPET_CHARS),
                        state: target.clone(),
                        source_state: Some(source.clone()),
                        target_state: Some(target),
                        variable: None,
                        reason: format!(
                            "A guard on one explicit state is followed by an explicit state assignment within {FSM_LOOKAHEAD_LINES} lines."
                        ),
                    },
                );
            }
        }
        [] => {}
        _ => push_skipped(
            scan,
            max_items,
            AuditFsmSkippedRecord {
                category: "ambiguous".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                reason: "State guard mentions multiple enum variants.".to_string(),
            },
        ),
    }
}

fn report_transition_parts(
    path: &str,
    index: usize,
    raw_lines: &[&str],
    left: &str,
    right: &str,
    max_items: usize,
    scan: &mut FsmScan,
) {
    let sources = qualified_states(left);
    let targets = qualified_states(right);
    match (sources.as_slice(), targets.as_slice()) {
        ([source], [target]) => push_finding(
            scan,
            max_items,
            AuditFsmFinding {
                kind: "transition_candidate".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                state: target.clone(),
                source_state: Some(source.clone()),
                target_state: Some(target.clone()),
                variable: None,
                reason: "A match arm maps one explicit state to one explicit state assignment."
                    .to_string(),
            },
        ),
        ([], _) | (_, []) => {}
        _ => push_skipped(
            scan,
            max_items,
            AuditFsmSkippedRecord {
                category: "ambiguous".to_string(),
                path: path.to_string(),
                line: index + 1,
                snippet: truncate_chars(raw_lines[index].trim(), MAX_SNIPPET_CHARS),
                reason: "Match arm transition mentions multiple enum variants.".to_string(),
            },
        ),
    }
}

fn find_following_state_assignment(index: usize, lines: &[String]) -> Option<(usize, String)> {
    let end = (index + FSM_LOOKAHEAD_LINES + 1).min(lines.len());
    lines
        .iter()
        .enumerate()
        .take(end)
        .skip(index + 1)
        .find_map(|(line_index, line)| {
            let (left, right) = split_assignment(line)?;
            if !is_state_like_name(left) {
                return None;
            }
            let targets = qualified_states(right);
            (targets.len() == 1).then(|| (line_index, targets[0].clone()))
        })
}

fn parse_state_like_enum_name(line: &str) -> Option<String> {
    let mut tokens = line.split(|character: char| {
        character.is_whitespace()
            || matches!(
                character,
                '(' | ')' | '<' | '>' | '{' | '}' | ';' | '=' | ':' | ','
            )
    });
    tokens.find(|token| *token == "enum")?;
    let name = tokens.find(|token| !token.is_empty())?;
    is_state_like_name(name).then(|| name.to_string())
}

fn parse_enum_variant(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with('}')
    {
        return None;
    }
    let variant = trimmed
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '(' | ')' | '{' | '}' | ',' | '=')
        })
        .find(|token| !token.is_empty())?;
    variant
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
        .then(|| variant.to_string())
}

fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'=' {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|index| bytes.get(index));
        let next = bytes.get(index + 1);
        if matches!(previous, Some(b'=' | b'!' | b'<' | b'>' | b'-'))
            || matches!(next, Some(b'=' | b'>'))
        {
            continue;
        }
        return Some((&line[..index], &line[index + 1..]));
    }
    None
}

fn qualified_states(line: &str) -> Vec<String> {
    let mut states = Vec::new();
    for token in line.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '_' || character == ':')
    }) {
        let Some((prefix, variant)) = token.rsplit_once("::") else {
            continue;
        };
        if prefix.is_empty()
            || variant.is_empty()
            || !variant.chars().next().is_some_and(char::is_uppercase)
        {
            continue;
        }
        let state = token.to_string();
        if !states.contains(&state) {
            states.push(state);
        }
    }
    states
}

fn is_guard_line(line: &str) -> bool {
    let trimmed = line.trim_start_matches('}').trim_start();
    trimmed.starts_with("if ")
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else if ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("match ")
}

fn is_state_like_name(name: &str) -> bool {
    let name = name.to_lowercase();
    STATE_NAME_HINTS.iter().any(|hint| name.contains(hint))
}

fn push_finding(scan: &mut FsmScan, max_items: usize, finding: AuditFsmFinding) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.findings.push(finding);
    }
}

fn push_skipped(scan: &mut FsmScan, max_items: usize, skipped: AuditFsmSkippedRecord) {
    scan.total_candidates += 1;
    if scan.findings.len() + scan.skipped.len() < max_items {
        scan.skipped.push(skipped);
    }
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, character| match character {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
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
