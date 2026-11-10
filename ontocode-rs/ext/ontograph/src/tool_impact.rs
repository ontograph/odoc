use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for ImpactTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<ImpactArgs, ImpactResult>(IMPACT_TOOL_NAME, IMPACT_DESCRIPTION)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<ImpactArgs>(&call, "ontograph.impact")?;
        let store = self
            .thread_state
            .load_current_source_index("ontograph.impact")?;
        let budget = QueryBudget::bounded(
            MAX_QUERY_TIMEOUT_MS,
            args.max_items.unwrap_or(MAX_SYMBOLS),
            MAX_SERIALIZED_BYTES,
        );
        let response = match args.action {
            ImpactAction::Symbol => {
                let symbol = parse_symbol_arg(args.symbol.as_deref(), "ontograph.impact")?;
                ImpactResult::Symbol(render_symbol_impact_result(
                    symbol.to_string(),
                    store
                        .impact_symbol(SymbolImpactRequest {
                            symbol: symbol.to_string(),
                            budget,
                        })
                        .map_err(impact_query_error)?,
                    "impact.symbol reads only the active native source-index artifact",
                ))
            }
            ImpactAction::Batch => {
                let symbols = parse_batch_symbol_args(args.symbols.as_deref(), "ontograph.impact")?;
                let results = symbols
                    .into_iter()
                    .map(|symbol| {
                        let symbol_for_error = symbol.clone();
                        store
                            .impact_symbol(SymbolImpactRequest {
                                symbol: symbol.clone(),
                                budget: budget.clone(),
                            })
                            .map(|result| render_batch_impact_result(symbol, result))
                            .map_err(|error| impact_batch_query_error(&symbol_for_error, error))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ImpactResult::Batch(ImpactBatchResult {
                    results,
                    limits: vec![
                        format!("batch symbol lists are capped at {MAX_QUERY_ITEMS} items"),
                        format!("impact items are capped at {MAX_QUERY_ITEMS} per direction"),
                        "impact.batch reads only the active native source-index artifact"
                            .to_string(),
                        "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                            .to_string(),
                    ],
                })
            }
            ImpactAction::Route => {
                let route = args.route.map(|route| route.trim().to_string());
                let route = match route {
                    Some(route) if route.is_empty() => {
                        return Err(FunctionCallError::RespondToModel(
                            "ontograph.impact route must not be empty".to_string(),
                        ));
                    }
                    Some(route) => Some(route),
                    None => None,
                };
                let cwd = self.thread_state.current_cwd().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.impact workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.impact workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                let file = match args.file {
                    Some(file) => {
                        let normalized = normalize_model_path(&file, "ontograph.impact")?;
                        Some(
                            resolve_existing_indexed_path(&normalized, &cwd, &repo_root)
                                .unwrap_or(normalized),
                        )
                    }
                    None => None,
                };
                if route.is_none() && file.is_none() {
                    return Err(FunctionCallError::RespondToModel(
                        "ontograph.impact route or file is required for action `route`".to_string(),
                    ));
                }
                let result = store
                    .impact_route(RouteImpactRequest {
                        route,
                        path: file,
                        budget,
                    })
                    .map_err(impact_route_query_error)?;
                let routes_truncated = result.routes.truncated;
                let results = result
                    .routes
                    .items
                    .into_iter()
                    .map(|route| ImpactRouteItemResult {
                        route: DiscoveredRouteSummary {
                            method: route.method,
                            route_path: route.route_path,
                            handler_symbol: route.handler_symbol,
                            path: route.path,
                            line: route.line,
                        },
                        upstream: route
                            .upstream
                            .items
                            .into_iter()
                            .map(|item| ImpactSymbolSummary {
                                symbol: item.symbol,
                                path: item.path,
                                relationship: item.relationship,
                                site_line: item.site_line,
                            })
                            .collect(),
                        downstream: route
                            .downstream
                            .items
                            .into_iter()
                            .map(|item| ImpactSymbolSummary {
                                symbol: item.symbol,
                                path: item.path,
                                relationship: item.relationship,
                                site_line: item.site_line,
                            })
                            .collect(),
                        upstream_truncated: route.upstream.truncated,
                        downstream_truncated: route.downstream.truncated,
                    })
                    .collect::<Vec<_>>();
                ImpactResult::Route(ImpactRouteResult {
                    results,
                    routes_truncated,
                    limits: vec![
                        format!("matched routes are capped at {MAX_QUERY_ITEMS} items"),
                        format!("impact items are capped at {MAX_QUERY_ITEMS} per direction"),
                        "impact.route matches active native source-index route records and reuses handler symbol impact from the same artifact"
                            .to_string(),
                        "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                            .to_string(),
                    ],
                })
            }
            ImpactAction::Diff => {
                let scope = args.scope.unwrap_or_else(|| "unstaged".to_string());
                if scope == "compare" && args.base_ref.is_none() {
                    return Err(FunctionCallError::RespondToModel(
                        "ontograph.impact base_ref is required for action `diff` with scope `compare`"
                            .to_string(),
                    ));
                }
                let mut include_paths = Vec::new();
                for path in args.include_paths.unwrap_or_default() {
                    include_paths.push(normalize_model_path(&path, "ontograph.impact")?);
                }
                ImpactResult::Diff(render_diff_impact_result(
                    store
                        .impact_diff(DiffImpactRequest {
                            scope,
                            base_ref: args.base_ref,
                            include_paths,
                            budget,
                        })
                        .map_err(impact_query_error)?,
                ))
            }
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.impact result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
