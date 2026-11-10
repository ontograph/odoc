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

use crate::audit_session::AuditSessionBundleStatus;
use crate::audit_session::run_audit_session_bundle;
use crate::manifest::OntographArtifactRoot;

const SUPPORTED_SCOPES: [&str; 4] = ["unstaged", "staged", "all", "compare"];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditWorkerScopeStatus {
    Pass,
    Fail,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditWorkerScopeResult {
    pub action: AuditAction,
    pub status: AuditWorkerScopeStatus,
    pub bundle_id: String,
    pub missing_expected_files: Vec<String>,
    pub unexpected_changed_files: Vec<String>,
    pub missing_expected_symbols: Vec<String>,
    pub unexpected_changed_symbols: Vec<String>,
    pub missing_expected_tests: Vec<String>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) struct WorkerScopeAuditInput<'a, Store: OntographReadQuery> {
    pub action: AuditAction,
    pub store: &'a Store,
    pub root: &'a OntographArtifactRoot,
    pub bundle_id: String,
    pub scope: String,
    pub base_ref: Option<String>,
    pub executed_tests: Option<Vec<String>>,
    pub max_items: usize,
}

pub(crate) fn inconclusive_worker_scope_result(
    bundle_id: String,
    note: String,
) -> AuditWorkerScopeResult {
    inconclusive_worker_scope_result_for_action(AuditAction::WorkerScopeReview, bundle_id, note)
}

pub(crate) fn inconclusive_worker_scope_result_for_action(
    action: AuditAction,
    bundle_id: String,
    note: String,
) -> AuditWorkerScopeResult {
    AuditWorkerScopeResult {
        action,
        status: AuditWorkerScopeStatus::Inconclusive,
        bundle_id,
        missing_expected_files: Vec::new(),
        unexpected_changed_files: Vec::new(),
        missing_expected_symbols: Vec::new(),
        unexpected_changed_symbols: Vec::new(),
        missing_expected_tests: Vec::new(),
        warnings: Vec::new(),
        notes: vec![note],
    }
}

pub(crate) fn run_worker_scope_audit(
    input: WorkerScopeAuditInput<'_, impl OntographReadQuery>,
) -> Result<AuditWorkerScopeResult, FunctionCallError> {
    let WorkerScopeAuditInput {
        action,
        store,
        root,
        bundle_id,
        scope,
        base_ref,
        executed_tests,
        max_items,
    } = input;

    let scope = scope.trim().to_string();
    if !SUPPORTED_SCOPES.contains(&scope.as_str()) {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit worker_scope_review scope must be one of: {}",
            SUPPORTED_SCOPES.join(", ")
        )));
    }
    if scope == "compare" && base_ref.is_none() {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit base_ref is required for action `worker_scope_review` with scope `compare`"
                .to_string(),
        ));
    }

    let bundle_result = run_audit_session_bundle(root)?;
    if bundle_result.status != AuditSessionBundleStatus::Pass {
        return Ok(inconclusive_worker_scope_result_for_action(
            action,
            bundle_id,
            "Could not load valid bundle state for worker scope review.".to_string(),
        ));
    }

    let bundle = bundle_result
        .bundles
        .into_iter()
        .find(|b| b.bundle_id == bundle_id);
    let bundle = match bundle {
        Some(b) => b,
        None => {
            return Ok(inconclusive_worker_scope_result_for_action(
                action,
                bundle_id,
                "Specified bundle_id not found in active audit session.".to_string(),
            ));
        }
    };

    let executed_tests = executed_tests
        .unwrap_or_default()
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();

    let expected_files = bundle.owner_files.into_iter().collect::<BTreeSet<_>>();
    let expected_symbols = bundle.target_symbols.into_iter().collect::<BTreeSet<_>>();
    let expected_tests = bundle.required_tests.into_iter().collect::<BTreeSet<_>>();

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
        .map_err(worker_scope_query_error)?;

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

    let missing_expected_files = set_difference(&expected_files, &actual_files);
    let unexpected_changed_files = set_difference(&actual_files, &expected_files);
    let missing_expected_symbols = set_difference(&expected_symbols, &actual_symbols);
    let unexpected_changed_symbols = set_difference(&actual_symbols, &expected_symbols);
    let missing_expected_tests = set_difference(&expected_tests, &executed_tests);

    let has_mismatch = !missing_expected_files.is_empty()
        || !unexpected_changed_files.is_empty()
        || !missing_expected_symbols.is_empty()
        || !unexpected_changed_symbols.is_empty()
        || !missing_expected_tests.is_empty();

    let has_limit = diff.changed_files.truncated
        || diff
            .changed_files
            .items
            .iter()
            .any(|file| file.changed_symbols.truncated)
        || !diff.warnings.is_empty();

    let status = if has_mismatch {
        AuditWorkerScopeStatus::Fail
    } else if has_limit {
        AuditWorkerScopeStatus::Inconclusive
    } else {
        AuditWorkerScopeStatus::Pass
    };

    Ok(AuditWorkerScopeResult {
        action,
        status,
        bundle_id,
        missing_expected_files,
        unexpected_changed_files,
        missing_expected_symbols,
        unexpected_changed_symbols,
        missing_expected_tests,
        warnings: diff.warnings,
        notes: vec![
            "audit.worker_scope_review reuses impact.diff for changed-file and changed-symbol facts.".to_string(),
            "Expected tests are compared only against caller-supplied executed_tests; no test discovery is performed.".to_string(),
        ],
    })
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn worker_scope_query_error(error: QueryError) -> FunctionCallError {
    let message = match error {
        QueryError::ExternalCommand => {
            "ontograph.audit worker_scope_review failed to read git diff from the current workspace"
        }
        QueryError::MissingArtifact
        | QueryError::AmbiguousArtifact
        | QueryError::StaleArtifact
        | QueryError::CorruptArtifact
        | QueryError::Timeout
        | QueryError::OverBudget => {
            "ontograph.audit worker_scope_review could not map changed files through the active source index"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}
