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
pub(crate) enum AuditVerifyDiffStatus {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditVerifyDiffResult {
    pub action: AuditAction,
    pub status: AuditVerifyDiffStatus,
    pub summary: AuditVerifyDiffSummary,
    pub missing_expected_files: Vec<String>,
    pub unexpected_changed_files: Vec<String>,
    pub missing_expected_symbols: Vec<String>,
    pub unexpected_changed_symbols: Vec<String>,
    pub missing_expected_tests: Vec<String>,
    pub changed_files_truncated: bool,
    pub changed_symbols_truncated: bool,
    pub warnings: Vec<String>,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditVerifyDiffSummary {
    pub changed_files: usize,
    pub changed_symbols: usize,
    pub affected_processes: usize,
    pub risk_level: String,
}

pub(crate) struct VerifyDiffAuditInput<'a, Store: OntographReadQuery> {
    pub store: &'a Store,
    pub scope: String,
    pub base_ref: Option<String>,
    pub expected_files: Option<Vec<String>>,
    pub expected_symbols: Option<Vec<String>>,
    pub expected_tests: Option<Vec<String>>,
    pub executed_tests: Option<Vec<String>>,
    pub max_items: usize,
}

pub(crate) fn inconclusive_verify_diff_result(note: String) -> AuditVerifyDiffResult {
    AuditVerifyDiffResult {
        action: AuditAction::VerifyDiff,
        status: AuditVerifyDiffStatus::Inconclusive,
        summary: AuditVerifyDiffSummary {
            changed_files: 0,
            changed_symbols: 0,
            affected_processes: 0,
            risk_level: "unknown".to_string(),
        },
        missing_expected_files: Vec::new(),
        unexpected_changed_files: Vec::new(),
        missing_expected_symbols: Vec::new(),
        unexpected_changed_symbols: Vec::new(),
        missing_expected_tests: Vec::new(),
        changed_files_truncated: false,
        changed_symbols_truncated: false,
        warnings: Vec::new(),
        limits: verify_diff_limits(),
        notes: vec![note],
    }
}

pub(crate) fn run_verify_diff_audit(
    input: VerifyDiffAuditInput<'_, impl OntographReadQuery>,
) -> Result<AuditVerifyDiffResult, FunctionCallError> {
    let VerifyDiffAuditInput {
        store,
        scope,
        base_ref,
        expected_files,
        expected_symbols,
        expected_tests,
        executed_tests,
        max_items,
    } = input;
    let scope = scope.trim().to_string();
    if !SUPPORTED_SCOPES.contains(&scope.as_str()) {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit verify_diff scope must be one of: {}",
            SUPPORTED_SCOPES.join(", ")
        )));
    }
    if scope == "compare" && base_ref.is_none() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit base_ref is required for action `verify_diff` with scope `compare`"
                .to_string(),
        ));
    }

    let expected_files = normalize_list(expected_files, "expected_files")?;
    let expected_symbols = normalize_list(expected_symbols, "expected_symbols")?;
    let expected_tests = normalize_list(expected_tests, "expected_tests")?;
    let executed_tests = normalize_list(executed_tests, "executed_tests")?;
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
        .map_err(verify_diff_query_error)?;

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
    let expected_files = expected_files.into_iter().collect::<BTreeSet<_>>();
    let expected_symbols = expected_symbols.into_iter().collect::<BTreeSet<_>>();
    let expected_tests = expected_tests.into_iter().collect::<BTreeSet<_>>();
    let executed_tests = executed_tests.into_iter().collect::<BTreeSet<_>>();

    let missing_expected_files = set_difference(&expected_files, &actual_files);
    let unexpected_changed_files = set_difference(&actual_files, &expected_files);
    let missing_expected_symbols = set_difference(&expected_symbols, &actual_symbols);
    let unexpected_changed_symbols = set_difference(&actual_symbols, &expected_symbols);
    let missing_expected_tests = set_difference(&expected_tests, &executed_tests);
    let changed_files_truncated = diff.changed_files.truncated;
    let changed_symbols_truncated = diff
        .changed_files
        .items
        .iter()
        .any(|file| file.changed_symbols.truncated);

    let has_mismatch = !missing_expected_files.is_empty()
        || !unexpected_changed_files.is_empty()
        || !missing_expected_symbols.is_empty()
        || !unexpected_changed_symbols.is_empty()
        || !missing_expected_tests.is_empty();
    let has_limit =
        changed_files_truncated || changed_symbols_truncated || !diff.warnings.is_empty();
    let status = if has_mismatch {
        AuditVerifyDiffStatus::Fail
    } else if has_limit {
        AuditVerifyDiffStatus::Inconclusive
    } else {
        AuditVerifyDiffStatus::Pass
    };

    Ok(AuditVerifyDiffResult {
        action: AuditAction::VerifyDiff,
        status,
        summary: AuditVerifyDiffSummary {
            changed_files: diff.summary.changed_files,
            changed_symbols: diff.summary.changed_symbols,
            affected_processes: diff.summary.affected_processes,
            risk_level: diff.summary.risk_level,
        },
        missing_expected_files,
        unexpected_changed_files,
        missing_expected_symbols,
        unexpected_changed_symbols,
        missing_expected_tests,
        changed_files_truncated,
        changed_symbols_truncated,
        warnings: diff.warnings,
        limits: verify_diff_limits(),
        notes: vec![
            "audit.verify_diff reuses impact.diff for changed-file and changed-symbol facts."
                .to_string(),
            "Expected tests are compared only against caller-supplied executed_tests; no test discovery is performed."
                .to_string(),
        ],
    })
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
            "ontograph.audit verify_diff {field_name} exceeds the maximum of {MAX_QUERY_ITEMS} items"
        )));
    }
    Ok(values)
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn verify_diff_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::ExternalCommand => {
            "ontograph.audit verify_diff failed to read git diff from the current workspace"
        }
        QueryError::MissingArtifact
        | QueryError::AmbiguousArtifact
        | QueryError::StaleArtifact
        | QueryError::CorruptArtifact
        | QueryError::Timeout
        | QueryError::OverBudget => {
            "ontograph.audit verify_diff could not map changed files through the active source index"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn verify_diff_limits() -> Vec<String> {
    vec![
        format!("changed files are capped at {MAX_QUERY_ITEMS} items"),
        format!("changed symbols are capped at {MAX_QUERY_ITEMS} items per file"),
        format!("expected files, symbols, and tests are capped at {MAX_QUERY_ITEMS} items each"),
        "audit.verify_diff supports scopes `unstaged`, `staged`, `all`, and `compare`".to_string(),
        "audit.verify_diff is read-only and does not discover tests or create tracking records"
            .to_string(),
    ]
}
