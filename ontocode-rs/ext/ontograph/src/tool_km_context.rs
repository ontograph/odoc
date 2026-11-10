use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for KmContextTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<KmContextArgs, KmContextResult>(
            KM_CONTEXT_TOOL_NAME,
            KM_CONTEXT_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<KmContextArgs>(&call, "ontograph.km_context")?;
        let id = args.id.trim();
        if id.is_empty() {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.km_context id must not be empty".to_string(),
            ));
        }
        let store = self.km_store()?;
        let resolved = resolve_active_km(&store, id)
            .map_err(km_context_resolve_error)?
            .ok_or_else(|| {
                FunctionCallError::RespondToModel(format!(
                    "ontograph.km_context has no active KM for id `{id}`"
                ))
            })?;
        // Agent-authored records belong to the thread that wrote them. Human and
        // unscoped records stay readable by any thread on this project.
        if let Some(KmIndexOrigin::AgentThread { thread_id }) = resolved.origin.as_ref()
            && self.thread_id.as_deref() != Some(thread_id.as_str())
        {
            return Err(FunctionCallError::RespondToModel(format!(
                "ontograph.km_context has no active KM for id `{id}`"
            )));
        }
        let summary_only = args.detail == Some(KmContextDetail::Summary);
        let fragment = if summary_only {
            crate::km_context::render_km_summary(&resolved)
        } else {
            render_km_context(
                &resolved,
                args.reason
                    .as_deref()
                    .unwrap_or("explicit ontograph.km_context request"),
            )
        };
        let context = fragment.render();
        let mut limits = vec![format!(
            "rendered KM context is capped at {} bytes",
            max_fragment_body_bytes()
        )];
        if !summary_only {
            limits.push(format!(
                "rendered KM facts are capped at {MAX_RENDERED_FACTS}"
            ));
            limits.push(format!(
                "rendered KM examples are capped at {MAX_RENDERED_EXAMPLES}"
            ));
        }
        limits.push("only active imported KMs are resolved".to_string());
        limits.push("raw bronze content is not returned".to_string());
        let response = KmContextResult {
            id: resolved.id,
            version: resolved.version,
            truncated: context.contains("Truncation:"),
            context,
            limits,
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.km_context result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
