use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for InspectTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<InspectArgs, InspectResult>(
            INSPECT_TOOL_NAME,
            INSPECT_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<InspectArgs>(&call, "ontograph.inspect")?;
        let cwd = self.thread_state.current_cwd().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.inspect workspace context is unavailable for this turn".to_string(),
            )
        })?;
        let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.inspect workspace context is unavailable for this turn".to_string(),
            )
        })?;
        let store = self
            .thread_state
            .load_current_source_index("ontograph.inspect")?;
        let budget = QueryBudget::bounded(
            MAX_QUERY_TIMEOUT_MS,
            args.max_items.unwrap_or(MAX_SYMBOLS),
            MAX_SERIALIZED_BYTES,
        );
        let response = match args.action {
            InspectAction::Context => {
                let normalized_path = normalize_model_path(
                    parse_inspect_path_arg(args.path.as_deref())?,
                    "ontograph.inspect",
                )?;
                let indexed_path = resolve_indexed_module_path(&normalized_path, &cwd, &repo_root);
                let result = store
                    .inspect_module(ModuleContextRequest {
                        path: indexed_path,
                        budget,
                    })
                    .map_err(inspect_query_error)?;
                InspectResult::Context(InspectContextResult {
                    path: result.module.path,
                    language: result.module.language,
                    symbols: result
                        .module
                        .symbols
                        .items
                        .into_iter()
                        .map(|symbol| InspectSymbolSummary {
                            symbol: symbol.symbol,
                            kind: symbol.kind,
                            path: symbol.path,
                        })
                        .collect(),
                    truncated: result.module.symbols.truncated,
                    limits: vec![
                        format!("context symbols are capped at {MAX_QUERY_ITEMS} items"),
                        "inspect.context reads only the active native source-index artifact"
                            .to_string(),
                    ],
                })
            }
            InspectAction::Shape => {
                let route = args.route.map(|route| route.trim().to_string());
                let route = match route {
                    Some(route) if route.is_empty() => {
                        return Err(FunctionCallError::RespondToModel(
                            "ontograph.inspect route must not be empty".to_string(),
                        ));
                    }
                    Some(route) => Some(route),
                    None => None,
                };
                let path = match args.path {
                    Some(path) => {
                        let normalized = normalize_model_path(&path, "ontograph.inspect")?;
                        Some(
                            resolve_existing_indexed_path(&normalized, &cwd, &repo_root)
                                .ok_or_else(|| {
                                    FunctionCallError::RespondToModel(
                                        "ontograph.inspect path is not present in source-index.json"
                                            .to_string(),
                                    )
                                })?,
                        )
                    }
                    None => None,
                };
                if route.is_none() && path.is_none() {
                    return Err(FunctionCallError::RespondToModel(
                        "ontograph.inspect route or path is required for action `shape`"
                            .to_string(),
                    ));
                }
                let result = store
                    .inspect_shape(RouteShapeRequest {
                        route,
                        path,
                        budget,
                    })
                    .map_err(inspect_shape_query_error)?;
                if result.routes.items.is_empty() {
                    return Err(FunctionCallError::RespondToModel(
                        "ontograph.inspect route or path is not present in source-index.json"
                            .to_string(),
                    ));
                }
                InspectResult::Shape(InspectShapeResult {
                    routes: result
                        .routes
                        .items
                        .into_iter()
                        .map(|route| InspectShapeRouteSummary {
                            method: route.method,
                            route_path: route.route_path,
                            handler_symbol: route.handler_symbol,
                            path: route.path,
                            line: route.line,
                        })
                        .collect(),
                    routes_truncated: result.routes.truncated,
                    limits: vec![
                        format!("matched routes are capped at {MAX_QUERY_ITEMS} items"),
                        "inspect.shape reads only the active native source-index route records"
                            .to_string(),
                        "Only direct indexed Axum route shapes are summarized.".to_string(),
                        "No route consumer, middleware, daemon, or donor runtime analysis is used."
                            .to_string(),
                    ],
                })
            }
            InspectAction::Evidence => {
                let targets = normalize_inspect_targets(args.targets.as_deref(), &cwd, &repo_root)?;
                let context_lines = args.context_lines.unwrap_or(3).min(20);
                let result = store
                    .inspect_evidence(EvidenceRequest {
                        targets,
                        include_snippet: args.include_snippet != Some(false),
                        context_lines,
                        budget,
                    })
                    .map_err(inspect_query_error)?;
                InspectResult::Evidence(InspectEvidenceResult {
                    evidence: result
                        .evidence
                        .items
                        .into_iter()
                        .map(|item| InspectEvidenceItem {
                            target: item.target,
                            kind: item.kind,
                            path: item.path,
                            start_line: item.start_line,
                            end_line: item.end_line,
                            snippet: item.snippet,
                            symbol: item.symbol,
                            symbol_kind: item.symbol_kind,
                        })
                        .collect(),
                    unresolved: result
                        .unresolved
                        .items
                        .into_iter()
                        .map(|item| InspectUnresolvedTarget {
                            target: item.target,
                            reason: item.reason,
                        })
                        .collect(),
                    evidence_truncated: result.evidence.truncated,
                    unresolved_truncated: result.unresolved.truncated,
                    limits: vec![
                        format!("evidence targets are capped at {MAX_QUERY_ITEMS} items"),
                        "evidence context lines are capped at 20".to_string(),
                        "inspect.evidence resolves line targets before file paths before exact symbol names"
                            .to_string(),
                        "inspect.evidence reads only the active native source-index artifact"
                            .to_string(),
                    ],
                })
            }
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.inspect result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
