use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for ExplainModuleTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<ExplainModuleArgs, ExplainModuleResult>(
            EXPLAIN_MODULE_TOOL_NAME,
            EXPLAIN_MODULE_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<ExplainModuleArgs>(&call, "ontograph.explain_module")?;
        let cwd = self.thread_state.current_cwd().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.explain_module workspace context is unavailable for this turn"
                    .to_string(),
            )
        })?;
        let normalized_path = normalize_model_path(&args.path, "ontograph.explain_module")?;
        let resolved_path = match resolve_module_path(&normalized_path, &cwd) {
            Ok(path) => path,
            Err(err) => {
                let Some(repo_root) = self.thread_state.current_repo_root() else {
                    return Err(err);
                };
                if repo_root == cwd || cwd.join(&normalized_path).exists() {
                    return Err(err);
                }
                resolve_module_path(&normalized_path, &repo_root)?
            }
        };
        let content = read_bounded_utf8(&resolved_path)?;
        let language = detect_language(&normalized_path);
        let top_level_symbols = collect_top_level_symbols(
            &content,
            language.as_deref(),
            args.max_symbols
                .unwrap_or(MAX_SYMBOLS)
                .clamp(1, MAX_SYMBOLS),
        );

        let result = ExplainModuleResult {
            path: normalized_path,
            language,
            line_count: line_count(&content),
            top_level_symbols,
            limits: vec![
                format!("file reads are capped at {MAX_FILE_BYTES} bytes"),
                RUST_SCAN_LIMIT_NOTE.to_string(),
            ],
        };
        let value = serde_json::to_value(result).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.explain_module result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
