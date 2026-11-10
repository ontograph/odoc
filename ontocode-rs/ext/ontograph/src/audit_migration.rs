use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::docs::DocsIndexLoadError;
use crate::docs::DocsIndexStore;
use crate::indexer::SourceIndexLoadError;
use crate::indexer::SourceIndexStore;
use crate::query::DocsMigrationItem;
use crate::query::DocsMigrationSkippedReference;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::SourceReferenceSet;
use crate::tool::OntographThreadState;
use crate::tool::ensure_current_source_index_ready;

const MAX_MIGRATION_ROWS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditMigrationStatus {
    Implemented,
    MissingSourceReference,
    Unverifiable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditMigrationItem {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub status: AuditMigrationStatus,
    pub text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub requirement_id: Option<String>,
    pub paths: Vec<String>,
    pub symbols: Vec<String>,
    pub routes: Vec<String>,
    pub missing_paths: Vec<String>,
    pub missing_symbols: Vec<String>,
    pub missing_routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditMigrationSkippedReference {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditMigrationResult {
    pub action: AuditAction,
    pub items: Vec<AuditMigrationItem>,
    pub skipped_references: Vec<AuditMigrationSkippedReference>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_migration_audit(
    thread_state: &OntographThreadState,
    max_items: usize,
) -> Result<AuditMigrationResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_MIGRATION_ROWS);
    ensure_current_source_index_ready(thread_state, "ontograph.audit")?;
    let artifact_root = thread_state.current_artifact_root().ok_or_else(|| {
        FunctionCallError::RespondToModel(
            "ontograph.audit workspace context is unavailable for this turn".to_string(),
        )
    })?;
    let docs = DocsIndexStore::load_active_generation_for_drift(&artifact_root)
        .map_err(migration_docs_load_error)?;
    let source_index = SourceIndexStore::load_active_generation(&artifact_root)
        .map_err(migration_source_index_load_error)?;
    let source = source_index.source_references();
    let refs = docs
        .migration_references(QueryBudget::bounded(
            QueryBudget::default().timeout_ms,
            max_items,
            QueryBudget::default().max_serialized_bytes,
        ))
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit migration references are corrupt".to_string(),
            )
        })?;
    let truncated = refs.items.truncated || refs.skipped.truncated;
    let items = refs
        .items
        .items
        .into_iter()
        .map(|item| migration_item(item, &source))
        .collect();
    let skipped_references = refs
        .skipped
        .items
        .into_iter()
        .map(skipped_reference)
        .collect();

    Ok(AuditMigrationResult {
        action: AuditAction::Migration,
        items,
        skipped_references,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_migration_rows={MAX_MIGRATION_ROWS}"),
            "migration fact kinds accepted: migration, migration_reference".to_string(),
        ],
        notes: vec![
            "This native slice reads only explicit migration facts from the paired docs-index.json sidecar."
                .to_string(),
            "Migration status is derived only from explicit path, symbol, and route references in the active source-index.json."
                .to_string(),
            "No prose inference, migration tooling, graph database, donor runtime, or host exposure is used."
                .to_string(),
        ],
    })
}

fn migration_item(item: DocsMigrationItem, source: &SourceReferenceSet) -> AuditMigrationItem {
    let missing_paths = item
        .paths
        .iter()
        .filter(|path| !source.paths.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    let missing_symbols = item
        .symbols
        .iter()
        .filter(|symbol| !migration_symbol_is_present(source, &item.paths, symbol))
        .cloned()
        .collect::<Vec<_>>();
    let missing_routes = item
        .routes
        .iter()
        .filter(|route| !source.route_paths.contains(*route))
        .cloned()
        .collect::<Vec<_>>();
    let has_references =
        !item.paths.is_empty() || !item.symbols.is_empty() || !item.routes.is_empty();
    let status =
        if missing_paths.is_empty() && missing_symbols.is_empty() && missing_routes.is_empty() {
            if has_references {
                AuditMigrationStatus::Implemented
            } else {
                AuditMigrationStatus::Unverifiable
            }
        } else {
            AuditMigrationStatus::MissingSourceReference
        };

    AuditMigrationItem {
        document_id: item.document_id,
        fact_id: item.fact_id,
        fact_kind: item.fact_kind,
        status,
        text: item.text,
        line_start: item.line_start,
        line_end: item.line_end,
        requirement_id: item.requirement_id,
        paths: item.paths,
        symbols: item.symbols,
        routes: item.routes,
        missing_paths,
        missing_symbols,
        missing_routes,
    }
}

fn migration_symbol_is_present(
    source: &SourceReferenceSet,
    paths: &[String],
    symbol: &str,
) -> bool {
    if paths.is_empty() {
        return source.symbols.contains(symbol);
    }
    paths.iter().any(|path| {
        source
            .symbols_by_path
            .get(path)
            .is_some_and(|symbols| symbols.contains(symbol))
    })
}

fn skipped_reference(skipped: DocsMigrationSkippedReference) -> AuditMigrationSkippedReference {
    AuditMigrationSkippedReference {
        document_id: skipped.document_id,
        fact_id: skipped.fact_id,
        fact_kind: skipped.fact_kind,
        reason: skipped.reason,
    }
}

pub(crate) fn migration_docs_load_error(error: DocsIndexLoadError) -> FunctionCallError {
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

pub(crate) fn migration_source_index_load_error(error: SourceIndexLoadError) -> FunctionCallError {
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
