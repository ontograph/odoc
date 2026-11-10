use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for ManageTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<ManageArgs, ManageResult>(MANAGE_TOOL_NAME, MANAGE_DESCRIPTION)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<ManageArgs>(&call, "ontograph.manage")?;
        let cwd = self.thread_state.current_cwd().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.manage workspace context is unavailable for this turn".to_string(),
            )
        })?;
        let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.manage workspace context is unavailable for this turn".to_string(),
            )
        })?;
        let response = match args.action {
            ManageAction::Index => {
                let (generation_id, counts, limits) = rebuild_source_index(&repo_root)?;
                ManageResult {
                    action: ManageAction::Index,
                    generation_id: Some(generation_id),
                    counts: Some(counts),
                    routes: vec![],
                    routes_truncated: false,
                    session: None,
                    lock: None,
                    active_source_identity: None,
                    session_path: None,
                    lock_path: None,
                    notes: vec![],
                    limits,
                }
            }
            ManageAction::Session => {
                let result = crate::manage_session::run_manage_session(
                    &crate::manifest::OntographArtifactRoot::new(repo_root),
                )?;

                ManageResult {
                    action: ManageAction::Session,
                    generation_id: None,
                    counts: None,

                    routes: vec![],

                    routes_truncated: false,

                    session: result.session,

                    lock: result.lock,

                    active_source_identity: result.active_source_identity,

                    session_path: result.session_path,

                    lock_path: result.lock_path,

                    notes: result.notes,

                    limits: vec![],
                }
            }
            ManageAction::RouteMap => {
                let store = self
                    .thread_state
                    .load_current_source_index("ontograph.manage")?;
                let budget = QueryBudget::bounded(
                    MAX_QUERY_TIMEOUT_MS,
                    args.max_items.unwrap_or(MAX_SYMBOLS),
                    MAX_SERIALIZED_BYTES,
                );
                let route = args.route.map(|route| route.trim().to_string());
                let route = match route {
                    Some(route) if route.is_empty() => {
                        return Err(FunctionCallError::RespondToModel(
                            "ontograph.manage route must not be empty".to_string(),
                        ));
                    }
                    Some(route) => Some(route),
                    None => None,
                };
                let path = match args.path {
                    Some(path) => {
                        let normalized = normalize_model_path(&path, "ontograph.manage")?;
                        Some(
                            resolve_existing_indexed_path(&normalized, &cwd, &repo_root)
                                .ok_or_else(|| {
                                    FunctionCallError::RespondToModel(
                                        "ontograph.manage path is not present in source-index.json"
                                            .to_string(),
                                    )
                                })?,
                        )
                    }
                    None => None,
                };
                let result = store
                    .discover_routes(RouteMapRequest {
                        route,
                        path,
                        budget,
                    })
                    .map_err(manage_query_error)?;
                ManageResult {
                    action: ManageAction::RouteMap,
                    generation_id: None,
                    counts: None,
                    routes: result
                        .routes
                        .items
                        .into_iter()
                        .map(|route| DiscoveredRouteSummary {
                            method: route.method,
                            route_path: route.route_path,
                            handler_symbol: route.handler_symbol,
                            path: route.path,
                            line: route.line,
                        })
                        .collect(),
                    routes_truncated: result.routes.truncated,
                    session: None,
                    lock: None,
                    active_source_identity: None,
                    session_path: None,
                    lock_path: None,
                    notes: vec![],
                    limits: vec![
                        format!("matched routes are capped at {MAX_QUERY_ITEMS} items"),
                        "manage.route_map reads only the active native source-index route records"
                            .to_string(),
                        "No route consumer, middleware, flow, daemon, or donor runtime analysis is used."
                            .to_string(),
                    ],
                }
            }
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.manage result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}
