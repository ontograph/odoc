use std::collections::BTreeMap;
use std::collections::BTreeSet;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::query::DiffImpactRequest;
use crate::query::MAX_QUERY_ITEMS;
use crate::query::MAX_QUERY_TIMEOUT_MS;
use crate::query::MAX_SERIALIZED_BYTES;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::QueryError;

const SUPPORTED_SCOPES: [&str; 4] = ["unstaged", "staged", "all", "compare"];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditTestGapStatus {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestGapResult {
    pub action: AuditAction,
    pub status: AuditTestGapStatus,
    pub summary: AuditTestGapSummary,
    pub missing_symbol_tests: Vec<AuditTestGapMissingSymbolTests>,
    pub missing_file_tests: Vec<AuditTestGapMissingFileTests>,
    pub symbols_without_expectations: Vec<String>,
    pub files_without_expectations: Vec<String>,
    pub changed_files_truncated: bool,
    pub changed_symbols_truncated: bool,
    pub warnings: Vec<String>,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestGapSummary {
    pub changed_files: usize,
    pub changed_symbols: usize,
    pub affected_processes: usize,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestGapMissingSymbolTests {
    pub symbol: String,
    pub missing_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestGapMissingFileTests {
    pub path: String,
    pub missing_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditTestSuggestionsStatus {
    Suggested,
    NoGaps,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestSuggestionsResult {
    pub action: AuditAction,
    pub status: AuditTestSuggestionsStatus,
    pub suggestions: Vec<AuditTestSuggestion>,
    pub source_gap_status: AuditTestGapStatus,
    pub warnings: Vec<String>,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditTestSuggestion {
    pub target_kind: AuditTestSuggestionTargetKind,
    pub target: String,
    pub candidate_test_path: String,
    pub assertion_shape: String,
    pub missing_tests: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditTestSuggestionTargetKind {
    Symbol,
    File,
}

pub(crate) struct TestGapAuditInput<'a, Store: OntographReadQuery> {
    pub store: &'a Store,
    pub scope: String,
    pub base_ref: Option<String>,
    pub executed_tests: Option<Vec<String>>,
    pub symbol_test_expectations: Option<BTreeMap<String, Vec<String>>>,
    pub file_test_expectations: Option<BTreeMap<String, Vec<String>>>,
    pub max_items: usize,
}

pub(crate) struct TestSuggestionsAuditInput<'a, Store: OntographReadQuery> {
    pub test_gap: TestGapAuditInput<'a, Store>,
}

pub(crate) fn inconclusive_test_gap_result(note: String) -> AuditTestGapResult {
    AuditTestGapResult {
        action: AuditAction::TestGap,
        status: AuditTestGapStatus::Inconclusive,
        summary: AuditTestGapSummary {
            changed_files: 0,
            changed_symbols: 0,
            affected_processes: 0,
            risk_level: "unknown".to_string(),
        },
        missing_symbol_tests: Vec::new(),
        missing_file_tests: Vec::new(),
        symbols_without_expectations: Vec::new(),
        files_without_expectations: Vec::new(),
        changed_files_truncated: false,
        changed_symbols_truncated: false,
        warnings: Vec::new(),
        limits: test_gap_limits(),
        notes: vec![note],
    }
}

pub(crate) fn run_test_gap_audit(
    input: TestGapAuditInput<'_, impl OntographReadQuery>,
) -> Result<AuditTestGapResult, FunctionCallError> {
    let TestGapAuditInput {
        store,
        scope,
        base_ref,
        executed_tests,
        symbol_test_expectations,
        file_test_expectations,
        max_items,
    } = input;
    let scope = scope.trim().to_string();
    if !SUPPORTED_SCOPES.contains(&scope.as_str()) {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit test_gap scope must be one of: {}",
            SUPPORTED_SCOPES.join(", ")
        )));
    }
    if scope == "compare" && base_ref.is_none() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit base_ref is required for action `test_gap` with scope `compare`"
                .to_string(),
        ));
    }

    let executed_tests = normalize_list(executed_tests, "executed_tests")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let symbol_test_expectations =
        normalize_expectations(symbol_test_expectations, "symbol_test_expectations")?;
    let file_test_expectations =
        normalize_expectations(file_test_expectations, "file_test_expectations")?;
    let budget = QueryBudget::bounded(
        MAX_QUERY_TIMEOUT_MS,
        max_items.clamp(1, MAX_QUERY_ITEMS),
        MAX_SERIALIZED_BYTES,
    );
    let diff = store
        .impact_diff(DiffImpactRequest {
            scope,
            base_ref,
            include_paths: Vec::new(),
            budget,
        })
        .map_err(test_gap_query_error)?;

    let actual_files = diff
        .changed_files
        .items
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let actual_symbols = diff
        .changed_files
        .items
        .iter()
        .flat_map(|file| file.changed_symbols.items.iter())
        .map(|symbol| symbol.symbol.clone())
        .collect::<BTreeSet<_>>();

    let mut missing_symbol_tests = Vec::new();
    let mut symbols_without_expectations = Vec::new();
    for symbol in &actual_symbols {
        if let Some(expected_tests) = symbol_test_expectations.get(symbol) {
            let missing_tests = set_difference(expected_tests, &executed_tests);
            if !missing_tests.is_empty() {
                missing_symbol_tests.push(AuditTestGapMissingSymbolTests {
                    symbol: symbol.clone(),
                    missing_tests,
                });
            }
        } else {
            symbols_without_expectations.push(symbol.clone());
        }
    }

    let mut missing_file_tests = Vec::new();
    let mut files_without_expectations = Vec::new();
    for path in &actual_files {
        if let Some(expected_tests) = file_test_expectations.get(path) {
            let missing_tests = set_difference(expected_tests, &executed_tests);
            if !missing_tests.is_empty() {
                missing_file_tests.push(AuditTestGapMissingFileTests {
                    path: path.clone(),
                    missing_tests,
                });
            }
        } else {
            files_without_expectations.push(path.clone());
        }
    }

    let changed_files_truncated = diff.changed_files.truncated;
    let changed_symbols_truncated = diff
        .changed_files
        .items
        .iter()
        .any(|file| file.changed_symbols.truncated);
    let has_gap = !missing_symbol_tests.is_empty()
        || !missing_file_tests.is_empty()
        || !symbols_without_expectations.is_empty()
        || !files_without_expectations.is_empty();
    let has_limit =
        changed_files_truncated || changed_symbols_truncated || !diff.warnings.is_empty();
    let status = if has_gap {
        AuditTestGapStatus::Fail
    } else if has_limit {
        AuditTestGapStatus::Inconclusive
    } else {
        AuditTestGapStatus::Pass
    };

    Ok(AuditTestGapResult {
        action: AuditAction::TestGap,
        status,
        summary: AuditTestGapSummary {
            changed_files: diff.summary.changed_files,
            changed_symbols: diff.summary.changed_symbols,
            affected_processes: diff.summary.affected_processes,
            risk_level: diff.summary.risk_level,
        },
        missing_symbol_tests,
        missing_file_tests,
        symbols_without_expectations,
        files_without_expectations,
        changed_files_truncated,
        changed_symbols_truncated,
        warnings: diff.warnings,
        limits: test_gap_limits(),
        notes: vec![
            "audit.test_gap reuses impact.diff for changed-file and changed-symbol facts."
                .to_string(),
            "Expected tests are caller-supplied; audit.test_gap does not discover tests or infer coverage from filenames."
                .to_string(),
        ],
    })
}

pub(crate) fn inconclusive_test_suggestions_result(note: String) -> AuditTestSuggestionsResult {
    AuditTestSuggestionsResult {
        action: AuditAction::TestSuggestions,
        status: AuditTestSuggestionsStatus::Inconclusive,
        suggestions: Vec::new(),
        source_gap_status: AuditTestGapStatus::Inconclusive,
        warnings: Vec::new(),
        limits: test_suggestions_limits(),
        notes: vec![note],
    }
}

pub(crate) fn run_test_suggestions_audit(
    input: TestSuggestionsAuditInput<'_, impl OntographReadQuery>,
) -> Result<AuditTestSuggestionsResult, FunctionCallError> {
    let gap = run_test_gap_audit(input.test_gap)?;
    let mut suggestions = Vec::new();
    for missing in &gap.missing_symbol_tests {
        suggestions.push(AuditTestSuggestion {
            target_kind: AuditTestSuggestionTargetKind::Symbol,
            target: missing.symbol.clone(),
            candidate_test_path: candidate_symbol_test_path(&missing.symbol),
            assertion_shape: format!(
                "assert expected behavior for changed symbol `{}`",
                missing.symbol
            ),
            missing_tests: missing.missing_tests.clone(),
        });
    }
    for missing in &gap.missing_file_tests {
        suggestions.push(AuditTestSuggestion {
            target_kind: AuditTestSuggestionTargetKind::File,
            target: missing.path.clone(),
            candidate_test_path: candidate_file_test_path(&missing.path),
            assertion_shape: format!(
                "assert expected behavior for changed file `{}`",
                missing.path
            ),
            missing_tests: missing.missing_tests.clone(),
        });
    }
    suggestions.truncate(MAX_QUERY_ITEMS);

    let status = if !gap.warnings.is_empty() || gap.status == AuditTestGapStatus::Inconclusive {
        AuditTestSuggestionsStatus::Inconclusive
    } else if suggestions.is_empty() {
        AuditTestSuggestionsStatus::NoGaps
    } else {
        AuditTestSuggestionsStatus::Suggested
    };

    Ok(AuditTestSuggestionsResult {
        action: AuditAction::TestSuggestions,
        status,
        suggestions,
        source_gap_status: gap.status,
        warnings: gap.warnings,
        limits: test_suggestions_limits(),
        notes: vec![
            "audit.test_suggestions derives suggestions only from explicit audit.test_gap missing-test evidence.".to_string(),
            "Suggestions are deterministic text/path hints; no tests are created, discovered, or model-generated.".to_string(),
        ],
    })
}

fn normalize_expectations(
    values: Option<BTreeMap<String, Vec<String>>>,
    field_name: &str,
) -> Result<BTreeMap<String, BTreeSet<String>>, FunctionCallError> {
    let values = values.unwrap_or_default();
    if values.len() > MAX_QUERY_ITEMS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit test_gap {field_name} exceeds the maximum of {MAX_QUERY_ITEMS} entries"
        )));
    }
    let mut normalized = BTreeMap::new();
    for (key, tests) in values {
        let key = key.trim().to_string();
        if key.is_empty() {
            continue;
        }
        normalized.insert(
            key,
            normalize_list(Some(tests), field_name)?
                .into_iter()
                .collect(),
        );
    }
    Ok(normalized)
}

fn normalize_list(
    values: Option<Vec<String>>,
    field_name: &str,
) -> Result<Vec<String>, FunctionCallError> {
    let values = values
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.len() > MAX_QUERY_ITEMS {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit test_gap {field_name} exceeds the maximum of {MAX_QUERY_ITEMS} items"
        )));
    }
    Ok(values)
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn test_gap_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::ExternalCommand => {
            "ontograph.audit test_gap failed to read git diff from the current workspace"
        }
        QueryError::MissingArtifact
        | QueryError::AmbiguousArtifact
        | QueryError::StaleArtifact
        | QueryError::CorruptArtifact
        | QueryError::Timeout
        | QueryError::OverBudget => {
            "ontograph.audit test_gap could not map changed files through the active source index"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn test_gap_limits() -> Vec<String> {
    vec![
        format!("changed files are capped at {MAX_QUERY_ITEMS} items"),
        format!("changed symbols are capped at {MAX_QUERY_ITEMS} items per file"),
        format!("executed tests are capped at {MAX_QUERY_ITEMS} items"),
        format!("symbol and file test expectations are capped at {MAX_QUERY_ITEMS} entries each"),
        "audit.test_gap supports scopes `unstaged`, `staged`, `all`, and `compare`".to_string(),
        "audit.test_gap is read-only and does not discover tests or infer coverage from filenames"
            .to_string(),
    ]
}

fn test_suggestions_limits() -> Vec<String> {
    let mut limits = test_gap_limits();
    limits.push(format!(
        "test suggestions are capped at {MAX_QUERY_ITEMS} items"
    ));
    limits
}

fn candidate_symbol_test_path(symbol: &str) -> String {
    format!("tests/{}_tests.rs", sanitize_test_path_stem(symbol))
}

fn candidate_file_test_path(path: &str) -> String {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("changed_file");
    format!("tests/{}_tests.rs", sanitize_test_path_stem(stem))
}

fn sanitize_test_path_stem(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.is_empty() {
        "target".to_string()
    } else {
        sanitized
    }
}
