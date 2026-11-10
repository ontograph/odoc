use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::docs::DocsIndexLoadError;
use crate::docs::DocsIndexStore;
use crate::indexer::SourceIndexLoadError;
use crate::indexer::SourceIndexStore;
use crate::query::DocsDriftItem;
use crate::query::DocsDriftRequest;
use crate::query::DocsDriftSkippedReference;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::tool::OntographThreadState;
use crate::tool::ensure_current_source_index_ready;

const MAX_DRIFT_ROWS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditDriftSourceSummary {
    pub status: String,
    pub docs_paired: bool,
    pub source_paths: usize,
    pub source_symbols: usize,
    pub source_routes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditDriftResult {
    pub action: AuditAction,
    pub source: AuditDriftSourceSummary,
    pub findings: Vec<DocsDriftItem>,
    pub skipped_references: Vec<DocsDriftSkippedReference>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_drift_audit(
    thread_state: &OntographThreadState,
    max_items: usize,
) -> Result<AuditDriftResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_DRIFT_ROWS);
    ensure_current_source_index_ready(thread_state, "ontograph.audit")?;
    let artifact_root = thread_state.current_artifact_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.audit workspace context is unavailable for this turn".to_string(),
        )
    })?;
    let docs = DocsIndexStore::load_active_generation_for_drift(&artifact_root)
        .map_err(drift_docs_load_error)?;
    let source_index = SourceIndexStore::load_active_generation(&artifact_root)
        .map_err(drift_source_index_load_error)?;
    let source = source_index.source_references();
    let result = docs
        .drift(
            DocsDriftRequest {
                budget: QueryBudget::bounded(
                    QueryBudget::default().timeout_ms,
                    max_items,
                    QueryBudget::default().max_serialized_bytes,
                ),
            },
            &source,
        )
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit drift references are corrupt".to_string(),
            )
        })?;
    Ok(AuditDriftResult {
        action: AuditAction::Drift,
        source: AuditDriftSourceSummary {
            status: "ready".to_string(),
            docs_paired: true,
            source_paths: source.paths.len(),
            source_symbols: source.symbols.len(),
            source_routes: source.route_paths.len(),
        },
        findings: result.items.items,
        skipped_references: result.skipped.items,
        truncated: result.items.truncated || result.skipped.truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_drift_rows={MAX_DRIFT_ROWS}"),
        ],
        notes: vec![
            "This native slice aggregates explicit docs.drift findings with active source-index readiness counts."
                .to_string(),
            "Only explicit docs-index paths, symbols, routes, and document links are checked."
                .to_string(),
            "No prose similarity, embeddings, requirements scoring, guessed graph edges, donor runtime, or host exposure is used."
                .to_string(),
        ],
    })
}

fn drift_docs_load_error(error: DocsIndexLoadError) -> FunctionCallError {
    let message = match error {
        DocsIndexLoadError::MissingCurrentPointer => {
            "ontograph.audit active source index is missing"
        }
        DocsIndexLoadError::InvalidCurrentPointer => {
            "ontograph.audit active source index pointer is invalid"
        }
        DocsIndexLoadError::MissingSourceManifest => {
            "ontograph.audit active source-index manifest is missing"
        }
        DocsIndexLoadError::ReadSourceManifest => {
            "ontograph.audit active source-index manifest is unreadable"
        }
        DocsIndexLoadError::InvalidSourceManifest => {
            "ontograph.audit active source-index manifest is invalid"
        }
        DocsIndexLoadError::MissingDocsIndex => "ontograph.audit docs-index.json is missing",
        DocsIndexLoadError::ReadDocsIndex => "ontograph.audit docs-index.json is unreadable",
        DocsIndexLoadError::InvalidDocsIndex => "ontograph.audit docs-index.json is invalid",
        DocsIndexLoadError::UnsupportedSchema => {
            "ontograph.audit docs-index.json schema is unsupported"
        }
        DocsIndexLoadError::InProgress => "ontograph.audit docs-index.json is in progress",
        DocsIndexLoadError::SourceIdentityMismatch => {
            "ontograph.audit docs-index.json does not match the active source-index.json identity"
        }
    };
    FunctionCallError::RespondToModel(message.to_string())
}

fn drift_source_index_load_error(error: SourceIndexLoadError) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            "ontograph.audit active source index is missing"
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            "ontograph.audit active source index pointer is invalid"
        }
        SourceIndexLoadError::MissingSourceIndex => "ontograph.audit source-index.json is missing",
        SourceIndexLoadError::ReadSourceIndex => "ontograph.audit source-index.json is unreadable",
        SourceIndexLoadError::InvalidSourceIndex => "ontograph.audit source-index.json is invalid",
    };
    FunctionCallError::RespondToModel(message.to_string())
}
