use super::*;

use crate::km::load_transitions;
use crate::procedural_graph::ProceduralFallbackReason;
use crate::procedural_graph::ProceduralGraph;
use crate::procedural_graph::ProceduralLookup;

/// Bounded cap on transitions returned by one inspection call.
pub(crate) const MAX_PROCEDURAL_CONTEXT_TRANSITIONS: usize = 32;
/// Hop ceiling accepted from the caller; the graph itself caps at two hops.
pub(crate) const MAX_PROCEDURAL_CONTEXT_HOPS: usize = 2;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for ProceduralContextTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, PROCEDURAL_CONTEXT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<ProceduralContextArgs, ProceduralContextResult>(
            PROCEDURAL_CONTEXT_TOOL_NAME,
            PROCEDURAL_CONTEXT_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<ProceduralContextArgs>(&call, "ontograph.procedural_context")?;
        let id = args.id.trim();
        if id.is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.procedural_context id must not be empty".to_string(),
            ));
        }
        let node = args.node.trim();
        if node.is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.procedural_context node must not be empty".to_string(),
            ));
        }

        let store = self.km_store()?;
        let resolved = resolve_active_km(&store, id)
            .map_err(km_context_resolve_error)?
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "ontograph.procedural_context has no active KM for id `{id}`"
                ))
            })?;
        // Agent-authored records belong to the thread that wrote them.
        if let Some(KmIndexOrigin::AgentThread { thread_id }) = resolved.origin.as_ref()
            && self.thread_id.as_deref() != Some(thread_id.as_str())
        {
            return Err(FunctionCallError::RespondToModel(format!(
                "ontograph.procedural_context has no active KM for id `{id}`"
            )));
        }

        // A malformed or tampered package fails closed instead of projecting a
        // partial graph.
        let record = store
            .resolve_active(id)
            .map_err(|err| km_context_resolve_error(KmResolveError::Store(err)))?
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "ontograph.procedural_context has no active KM for id `{id}`"
                ))
            })?;
        let root = store
            .validated_module_root(&record)
            .map_err(|err| km_context_resolve_error(KmResolveError::Store(err)))?;
        // `validated_module_root` already rejects a package whose
        // `transitions.jsonl` fails the loader, so reaching this point means the
        // file is either absent or valid. Transitions are optional in a v1
        // package: absence projects as an empty graph and reports a fallback
        // reason rather than an error.
        let transitions = if root.join("transitions.jsonl").is_file() {
            load_transitions(&root).map_err(|error| {
                FunctionCallError::RespondToModel(format!(
                    "ontograph.procedural_context failed to read the KM cache: {error}"
                ))
            })?
        } else {
            Vec::new()
        };

        let requested_hops = args
            .max_hops
            .unwrap_or(MAX_PROCEDURAL_CONTEXT_HOPS)
            .min(MAX_PROCEDURAL_CONTEXT_HOPS);
        let graph = ProceduralGraph::from_validated_transitions(transitions);

        let (active_node, edges, fallback_reason) = match graph.project(node, requested_hops) {
            ProceduralLookup::Localized(neighborhood) => {
                let edges = neighborhood
                    .edges
                    .iter()
                    .take(MAX_PROCEDURAL_CONTEXT_TRANSITIONS)
                    .map(|edge| ProceduralContextTransition {
                        source: edge.transition.source.clone(),
                        target: edge.transition.target.clone(),
                        relation: edge.transition.relation.clone(),
                        condition: edge.transition.condition.clone(),
                        guidance: edge.transition.guidance.clone(),
                        pitfalls: edge.transition.pitfalls.clone(),
                        hops: edge.hops,
                    })
                    .collect::<Vec<_>>();
                (Some(neighborhood.active_node), edges, None)
            }
            ProceduralLookup::Fallback { reason } => {
                (None, Vec::new(), Some(procedural_fallback_reason(reason)))
            }
        };

        let response = ProceduralContextResult {
            id: resolved.id.clone(),
            version: resolved.version.clone(),
            active_node,
            transitions: edges,
            fallback_reason,
            provenance: ProceduralContextProvenance {
                created_by: resolved.provenance.created_by.clone(),
                generated_at: resolved.provenance.generated_at.clone(),
                source_commit: resolved.provenance.source_commit,
            },
            limits: vec![
                format!("transitions are capped at {MAX_PROCEDURAL_CONTEXT_TRANSITIONS}"),
                format!("traversal is capped at {MAX_PROCEDURAL_CONTEXT_HOPS} hops"),
                "only active imported KMs are projected".to_string(),
                "transitions are advisory and never authorize tool calls".to_string(),
                "this surface is read-only and never mutates the graph".to_string(),
            ],
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.procedural_context result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}

fn procedural_fallback_reason(reason: ProceduralFallbackReason) -> String {
    match reason {
        ProceduralFallbackReason::EmptyGraph => {
            "the active Knowledge Module declares no transitions".to_string()
        }
        ProceduralFallbackReason::UnknownIdentifier => {
            "the requested node is not a declared procedure in this Knowledge Module".to_string()
        }
    }
}
