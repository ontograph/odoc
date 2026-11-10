use std::collections::BTreeSet;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::indexer::CompletedSourceIndexGeneration;
use crate::indexer::SourceIndexArtifact;
use crate::indexer::SourceIndexLoadError;
use crate::indexer::SourceIndexStore;
use crate::manifest::OntographArtifactRoot;

const MAX_GRAPH_DIFF_ITEMS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditGraphDiffGenerationSummary {
    pub generation_id: String,
    pub target_head: String,
    pub files: u64,
    pub symbols: u64,
    pub relationships: u64,
    pub unresolved_calls: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditGraphDiffSet {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub added_truncated: bool,
    pub removed_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditGraphDiffResult {
    pub action: AuditAction,
    pub base: AuditGraphDiffGenerationSummary,
    pub target: AuditGraphDiffGenerationSummary,
    pub file_nodes: AuditGraphDiffSet,
    pub symbol_nodes: AuditGraphDiffSet,
    pub calls: AuditGraphDiffSet,
    pub unresolved_calls: AuditGraphDiffSet,
    pub process_nodes: AuditGraphDiffSet,
    pub route_nodes: AuditGraphDiffSet,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_graph_diff_audit(
    artifact_root: &OntographArtifactRoot,
    base_generation_id: Option<&str>,
    target_generation_id: Option<&str>,
    max_items: usize,
) -> Result<AuditGraphDiffResult, FunctionCallError> {
    let base_generation_id = parse_generation_id(base_generation_id, "base_generation_id")?;
    let target_generation_id = parse_generation_id(target_generation_id, "target_generation_id")?;
    let max_items = max_items.clamp(1, MAX_GRAPH_DIFF_ITEMS);
    let base = SourceIndexStore::load_completed_generation(artifact_root, base_generation_id)
        .map_err(|err| graph_diff_load_error(err, "base_generation_id"))?;
    let target = SourceIndexStore::load_completed_generation(artifact_root, target_generation_id)
        .map_err(|err| graph_diff_load_error(err, "target_generation_id"))?;
    validate_generation_pair(&base, &target)?;
    validate_graph_ids(&base.artifact)?;
    validate_graph_ids(&target.artifact)?;

    let file_nodes = diff_sets(
        graph_file_node_ids(&base.artifact),
        graph_file_node_ids(&target.artifact),
        max_items,
    );
    let symbol_nodes = diff_sets(
        graph_symbol_node_ids(&base.artifact),
        graph_symbol_node_ids(&target.artifact),
        max_items,
    );
    let calls = diff_sets(
        graph_call_ids(&base.artifact),
        graph_call_ids(&target.artifact),
        max_items,
    );
    let unresolved_calls = diff_sets(
        graph_unresolved_call_ids(&base.artifact),
        graph_unresolved_call_ids(&target.artifact),
        max_items,
    );
    let process_nodes = diff_sets(
        graph_process_node_ids(&base.artifact),
        graph_process_node_ids(&target.artifact),
        max_items,
    );
    let route_nodes = diff_sets(
        graph_route_node_ids(&base.artifact),
        graph_route_node_ids(&target.artifact),
        max_items,
    );
    let truncated = file_nodes.added_truncated
        || file_nodes.removed_truncated
        || symbol_nodes.added_truncated
        || symbol_nodes.removed_truncated
        || calls.added_truncated
        || calls.removed_truncated
        || unresolved_calls.added_truncated
        || unresolved_calls.removed_truncated
        || process_nodes.added_truncated
        || process_nodes.removed_truncated
        || route_nodes.added_truncated
        || route_nodes.removed_truncated;

    Ok(AuditGraphDiffResult {
        action: AuditAction::GraphDiff,
        base: generation_summary(&base),
        target: generation_summary(&target),
        file_nodes,
        symbol_nodes,
        calls,
        unresolved_calls,
        process_nodes,
        route_nodes,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_graph_diff_items={MAX_GRAPH_DIFF_ITEMS}"),
        ],
        notes: vec![
            "This native slice compares completed source-index graph generations by stable identifiers only."
                .to_string(),
            "Only file node ids, symbol node ids, CALLS relationship ids, unresolved-call ids, process node ids, and route node ids are compared."
                .to_string(),
            "No semantic diff, graph database, donor runtime, report fan-out, or host exposure is used."
                .to_string(),
        ],
    })
}

fn parse_generation_id<'a>(
    value: Option<&'a str>,
    field_name: &str,
) -> Result<&'a str, FunctionCallError> {
    let Some(value) = value else {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit graph_diff requires {field_name}"
        )));
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(FunctionCallError::RespondToModel(format!(
            "ontograph.audit graph_diff {field_name} must not be empty"
        )));
    }
    Ok(value)
}

fn validate_generation_pair(
    base: &CompletedSourceIndexGeneration,
    target: &CompletedSourceIndexGeneration,
) -> Result<(), FunctionCallError> {
    if base.manifest.repository != target.manifest.repository {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit graph_diff generation repository identity mismatch".to_string(),
        ));
    }
    if base.manifest.version != target.manifest.version {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit graph_diff generation artifact version mismatch".to_string(),
        ));
    }
    Ok(())
}

fn validate_graph_ids(artifact: &SourceIndexArtifact) -> Result<(), FunctionCallError> {
    let file_nodes_ready = artifact.files.iter().all(|file| {
        !file.node_id.is_empty() && file.symbols.iter().all(|symbol| !symbol.node_id.is_empty())
    });
    let calls_ready = artifact
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == "CALLS")
        .all(|relationship| !relationship.relationship_id.is_empty());
    let unresolved_ready = artifact
        .unresolved_calls
        .iter()
        .all(|call| !call.unresolved_call_id.is_empty());
    let processes_ready = artifact
        .processes
        .iter()
        .all(|process| !process.node_id.is_empty());
    let routes_ready = artifact
        .routes
        .iter()
        .all(|route| !route.node_id.is_empty());
    if file_nodes_ready && calls_ready && unresolved_ready && processes_ready && routes_ready {
        Ok(())
    } else {
        Err(FunctionCallError::RespondToModel(
            "ontograph.audit graph_diff generation graph facts are incomplete".to_string(),
        ))
    }
}

fn generation_summary(
    generation: &CompletedSourceIndexGeneration,
) -> AuditGraphDiffGenerationSummary {
    AuditGraphDiffGenerationSummary {
        generation_id: generation.generation.metadata.generation_id.clone(),
        target_head: generation.generation.metadata.target_head.clone(),
        files: generation.generation.counts.files,
        symbols: generation.generation.counts.symbols,
        relationships: generation.generation.counts.relationships,
        unresolved_calls: generation.artifact.unresolved_calls.len(),
    }
}

fn graph_file_node_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .files
        .iter()
        .map(|file| file.node_id.clone())
        .collect()
}

fn graph_symbol_node_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .files
        .iter()
        .flat_map(|file| file.symbols.iter().map(|symbol| symbol.node_id.clone()))
        .collect()
}

fn graph_call_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == "CALLS")
        .map(|relationship| relationship.relationship_id.clone())
        .collect()
}

fn graph_unresolved_call_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .unresolved_calls
        .iter()
        .map(|call| call.unresolved_call_id.clone())
        .collect()
}

fn graph_process_node_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .processes
        .iter()
        .map(|process| process.node_id.clone())
        .collect()
}

fn graph_route_node_ids(artifact: &SourceIndexArtifact) -> BTreeSet<String> {
    artifact
        .routes
        .iter()
        .map(|route| route.node_id.clone())
        .collect()
}

fn diff_sets(
    base: BTreeSet<String>,
    target: BTreeSet<String>,
    max_items: usize,
) -> AuditGraphDiffSet {
    let added = target.difference(&base).cloned().collect::<Vec<_>>();
    let removed = base.difference(&target).cloned().collect::<Vec<_>>();
    let added_truncated = added.len() > max_items;
    let removed_truncated = removed.len() > max_items;
    AuditGraphDiffSet {
        added: added.into_iter().take(max_items).collect(),
        removed: removed.into_iter().take(max_items).collect(),
        added_truncated,
        removed_truncated,
    }
}

fn graph_diff_load_error(error: SourceIndexLoadError, field_name: &str) -> FunctionCallError {
    let message = match error {
        SourceIndexLoadError::MissingCurrentPointer => {
            format!("ontograph.audit graph_diff {field_name} pointer is missing")
        }
        SourceIndexLoadError::InvalidCurrentPointer => {
            format!("ontograph.audit graph_diff {field_name} is invalid")
        }
        SourceIndexLoadError::MissingSourceIndex => {
            format!("ontograph.audit graph_diff {field_name} generation is missing")
        }
        SourceIndexLoadError::ReadSourceIndex => {
            format!("ontograph.audit graph_diff {field_name} generation is unreadable")
        }
        SourceIndexLoadError::InvalidSourceIndex => {
            format!("ontograph.audit graph_diff {field_name} generation is corrupt")
        }
    };
    FunctionCallError::RespondToModel(message)
}
