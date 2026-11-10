use super::*;

#[async_trait::async_trait]
#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for DiscoverTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<DiscoverArgs, DiscoverResult>(
            DISCOVER_TOOL_NAME,
            DISCOVER_TOOL_DESCRIPTION,
        )
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<DiscoverArgs>(&call, "ontograph.discover")?;
        let result = match args.action {
            DiscoverAction::Groups => {
                let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.discover workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                if let Some(name) = args.name.as_deref().map(str::trim)
                    && name != CURRENT_WORKSPACE_GROUP
                {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "ontograph.discover group name `{name}` is not supported; only `{CURRENT_WORKSPACE_GROUP}` is available"
                    )));
                }
                let group = match read_current_workspace_group_state(&repo_root).map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "ontograph.discover failed to read current-workspace group state: {}",
                        group_state_error_message(err)
                    ))
                })? {
                    Some(group) => group,
                    None => {
                        let artifact_root = self.thread_state.current_artifact_root().ok_or_else(
                            || {
                                FunctionCallError::RespondToModel(
                                    "ontograph.discover workspace context is unavailable for this turn"
                                        .to_string(),
                                )
                            },
                        )?;
                        CurrentWorkspaceGroupState::new(
                            vec![repo_root.display().to_string()],
                            artifact_root.root_path().display().to_string(),
                        )
                    }
                };
                discover_groups_result(
                    group,
                    "Only one synthetic local group for the current workspace is listed.",
                    "No donor group registry, manifest sync, or cross-workspace state is used.",
                )
            }
            DiscoverAction::Packs => DiscoverResult {
                namespace: ONTOGRAPH_NAMESPACE.to_string(),
                groups: Vec::new(),
                packs: discovered_pack_summaries(),
                tools: Vec::new(),
                repos: Vec::new(),
                routes: Vec::new(),
                routes_truncated: false,
                parity_matrix_path: PARITY_MATRIX_PATH.to_string(),
                donor_actions_total: DONOR_ACTIONS_TOTAL,
                donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
                donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
                donor_actions_missing: donor_actions_missing(),
                notes: vec![
                    "Only static native Ontograph pack descriptors are listed.".to_string(),
                    "No donor manifests, suites, execution plans, or model-pack metadata are used."
                        .to_string(),
                    "Docs-backed actions require a docs-index.json sidecar, which ontograph.manage action `index` publishes alongside the source index."
                        .to_string(),
                ],
            },
            DiscoverAction::Repos => {
                let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.discover workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                let artifact_root = self.thread_state.current_artifact_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.discover workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                let repo = discover_repo_from_manifest(&artifact_root)?;
                let expectation = manifest_expectation(&repo_root).map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "ontograph.discover failed to inspect the Rust workspace state: {}",
                        rust_index_error_message(err)
                    ))
                })?;
                if artifact_root.current_manifest_readiness(&expectation)
                    != ArtifactReadiness::Ready
                {
                    return Err(FunctionCallError::RespondToModel(
                        "ontograph.discover source index is stale for the current workspace state; run ontograph.manage with action `index` to rebuild it"
                            .to_string(),
                    ));
                }
                DiscoverResult {
                    namespace: ONTOGRAPH_NAMESPACE.to_string(),
                    groups: Vec::new(),
                    packs: Vec::new(),
                    tools: Vec::new(),
                    repos: vec![repo],
                    routes: Vec::new(),
                    routes_truncated: false,
                    parity_matrix_path: PARITY_MATRIX_PATH.to_string(),
                    donor_actions_total: DONOR_ACTIONS_TOTAL,
                    donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
                    donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
                    donor_actions_missing: donor_actions_missing(),
                    notes: vec![
                        "Only the current workspace active native Ontograph artifact is listed."
                            .to_string(),
                        "No donor repo registry, daemon, sync, or group model is used.".to_string(),
                    ],
                }
            }
            DiscoverAction::Routes => {
                let store = self
                    .thread_state
                    .load_current_source_index("ontograph.discover")?;
                let budget = QueryBudget::bounded(
                    MAX_QUERY_TIMEOUT_MS,
                    args.max_items.unwrap_or(MAX_SYMBOLS),
                    MAX_SERIALIZED_BYTES,
                );
                let result = store
                    .discover_routes(RouteMapRequest {
                        route: args.route,
                        path: None,
                        budget,
                    })
                    .map_err(discover_query_error)?;
                let routes_truncated = result.routes.truncated;
                DiscoverResult {
                    namespace: ONTOGRAPH_NAMESPACE.to_string(),
                    groups: Vec::new(),
                    packs: Vec::new(),
                    tools: Vec::new(),
                    repos: Vec::new(),
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
                    routes_truncated,
                    parity_matrix_path: PARITY_MATRIX_PATH.to_string(),
                    donor_actions_total: DONOR_ACTIONS_TOTAL,
                    donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
                    donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
                    donor_actions_missing: donor_actions_missing(),
                    notes: vec![
                        "Only active native source-index route records are listed.".to_string(),
                        "No route shape, consumer, middleware, daemon, or donor runtime analysis is used."
                            .to_string(),
                    ],
                }
            }
            DiscoverAction::Sync => {
                let repo_root = self.thread_state.current_repo_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.discover workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                if let Some(name) = args.name.as_deref().map(str::trim)
                    && name != CURRENT_WORKSPACE_GROUP
                {
                    return Err(FunctionCallError::RespondToModel(format!(
                        "ontograph.discover group name `{name}` is not supported; only `{CURRENT_WORKSPACE_GROUP}` can be synced"
                    )));
                }
                let artifact_root = self.thread_state.current_artifact_root().ok_or_else(|| {
                    FunctionCallError::RespondToModel(
                        "ontograph.discover workspace context is unavailable for this turn"
                            .to_string(),
                    )
                })?;
                let group = CurrentWorkspaceGroupState::new(
                    vec![repo_root.display().to_string()],
                    artifact_root.root_path().display().to_string(),
                );
                read_current_workspace_group_state(&repo_root).map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "ontograph.discover failed to read current-workspace group state before sync: {}",
                        group_state_error_message(err)
                    ))
                })?;
                write_current_workspace_group_state(&repo_root, &group).map_err(|err| {
                    FunctionCallError::RespondToModel(format!(
                        "ontograph.discover failed to sync current-workspace group state: {}",
                        group_state_error_message(err)
                    ))
                })?;
                let group = read_current_workspace_group_state(&repo_root)
                    .map_err(|err| {
                        FunctionCallError::RespondToModel(format!(
                            "ontograph.discover failed to read current-workspace group state after sync: {}",
                            group_state_error_message(err)
                        ))
                    })?
                    .ok_or_else(|| {
                        FunctionCallError::RespondToModel(
                            "ontograph.discover failed to read current-workspace group state after sync: missing synced state"
                                .to_string(),
                        )
                    })?;
                discover_groups_result(
                    group,
                    "Explicit sync wrote and read back the current-workspace group state.",
                    "No donor group registry, daemon, plugin fallback, or cross-workspace state is used.",
                )
            }
            DiscoverAction::Tools => DiscoverResult {
                namespace: ONTOGRAPH_NAMESPACE.to_string(),
                groups: Vec::new(),
                packs: Vec::new(),
                tools: implemented_tool_summaries(),
                repos: Vec::new(),
                routes: Vec::new(),
                routes_truncated: false,
                parity_matrix_path: PARITY_MATRIX_PATH.to_string(),
                donor_actions_total: DONOR_ACTIONS_TOTAL,
                donor_actions_implemented: DONOR_ACTIONS_IMPLEMENTED,
                donor_actions_intentional_divergence: DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
                donor_actions_missing: donor_actions_missing(),
                notes: vec![
                    "Only live native Ontograph tools are listed.".to_string(),
                    "Donor facade placeholders are intentionally excluded.".to_string(),
                    "Docs-backed actions require a docs-index.json sidecar, which ontograph.manage action `index` publishes alongside the source index."
                        .to_string(),
                ],
            },
        };
        let value = serde_json::to_value(result).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.discover result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}

impl DiscoverTool {
    pub(crate) fn new(thread_state: Arc<OntographThreadState>) -> Self {
        Self { thread_state }
    }
}
