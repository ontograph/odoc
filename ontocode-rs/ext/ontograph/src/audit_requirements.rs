use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::audit_migration::migration_docs_load_error;
use crate::audit_migration::migration_source_index_load_error;
use crate::docs::DocsIndexStore;
use crate::indexer::SourceIndexStore;
use crate::query::DocsRequirementItem;
use crate::query::DocsRequirementLink;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::SourceReferenceSet;
use crate::tool::OntographThreadState;
use crate::tool::ensure_current_source_index_ready;

const MAX_REQUIREMENT_ROWS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditRequirementStatus {
    Referenced,
    MissingSourceReference,
    Unverifiable,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditRequirementItem {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub requirement_id: String,
    pub status: AuditRequirementStatus,
    pub text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub paths: Vec<String>,
    pub symbols: Vec<String>,
    pub routes: Vec<String>,
    pub missing_paths: Vec<String>,
    pub missing_symbols: Vec<String>,
    pub missing_routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditRequirementLink {
    pub link_id: String,
    pub from_document_id: String,
    pub requirement_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditRequirementsResult {
    pub action: AuditAction,
    pub items: Vec<AuditRequirementItem>,
    pub links: Vec<AuditRequirementLink>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_requirements_audit(
    thread_state: &OntographThreadState,
    max_items: usize,
) -> Result<AuditRequirementsResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_REQUIREMENT_ROWS);
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
        .requirement_references(QueryBudget::bounded(
            QueryBudget::default().timeout_ms,
            max_items,
            QueryBudget::default().max_serialized_bytes,
        ))
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit requirement references are corrupt".to_string(),
            )
        })?;
    let truncated = refs.items.truncated || refs.links.truncated;
    let items = refs
        .items
        .items
        .into_iter()
        .map(|item| requirement_item(item, &source))
        .collect();
    let links = refs.links.items.into_iter().map(requirement_link).collect();

    Ok(AuditRequirementsResult {
        action: AuditAction::Requirements,
        items,
        links,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_requirement_rows={MAX_REQUIREMENT_ROWS}"),
            "requirement facts accepted only when requirement_id is explicit".to_string(),
        ],
        notes: vec![
            "This native slice reads only explicit requirement facts and requirement links from the paired docs-index.json sidecar."
                .to_string(),
            "Requirement source status is derived only from explicit path, symbol, and route references in the active source-index.json."
                .to_string(),
            "No semantic requirement inference, completeness scoring, embeddings, donor runtime, or host exposure is used."
                .to_string(),
        ],
    })
}

fn requirement_item(
    item: DocsRequirementItem,
    source: &SourceReferenceSet,
) -> AuditRequirementItem {
    let missing_paths = item
        .paths
        .iter()
        .filter(|path| !source.paths.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    let missing_symbols = item
        .symbols
        .iter()
        .filter(|symbol| !requirement_symbol_is_present(source, &item.paths, symbol))
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
                AuditRequirementStatus::Referenced
            } else {
                AuditRequirementStatus::Unverifiable
            }
        } else {
            AuditRequirementStatus::MissingSourceReference
        };

    AuditRequirementItem {
        document_id: item.document_id,
        fact_id: item.fact_id,
        fact_kind: item.fact_kind,
        requirement_id: item.requirement_id,
        status,
        text: item.text,
        line_start: item.line_start,
        line_end: item.line_end,
        paths: item.paths,
        symbols: item.symbols,
        routes: item.routes,
        missing_paths,
        missing_symbols,
        missing_routes,
    }
}

fn requirement_symbol_is_present(
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

fn requirement_link(link: DocsRequirementLink) -> AuditRequirementLink {
    AuditRequirementLink {
        link_id: link.link_id,
        from_document_id: link.from_document_id,
        requirement_id: link.requirement_id,
    }
}
