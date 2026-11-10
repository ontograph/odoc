use super::*;

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for DocsTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        namespace_function_tool::<DocsArgs, DocsResult>(DOCS_TOOL_NAME, DOCS_DESCRIPTION)
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::DirectModelOnly
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let args = parse_tool_args::<DocsArgs>(&call, "ontograph.docs")?;
        let artifact_root = self.thread_state.current_artifact_root().ok_or_else(|| {
            FunctionCallError::RespondToModel(
                "ontograph.docs workspace context is unavailable for this turn".to_string(),
            )
        })?;
        if args.action == DocsAction::Drift {
            ensure_current_source_index_ready(&self.thread_state, "ontograph.docs")?;
        } else {
            self.thread_state
                .ensure_active_source_index("ontograph.docs")?;
        }
        if args.action == DocsAction::Readiness && docs_args_have_context_selector(&args) {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.docs readiness does not accept context selectors".to_string(),
            ));
        }
        if args.action == DocsAction::Trace && docs_args_have_context_only_selector(&args) {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.docs trace accepts only requirement_id, document_id, route, path, and symbol selectors"
                    .to_string(),
            ));
        }
        if args.action == DocsAction::Trace && !docs_args_have_trace_selector(&args) {
            return Err(FunctionCallError::RespondToModel(
                "ontograph.docs trace requires at least one selector".to_string(),
            ));
        }
        let store = if args.action == DocsAction::Drift {
            DocsIndexStore::load_active_generation_for_drift(&artifact_root)
        } else {
            DocsIndexStore::load_active_generation(&artifact_root)
        }
        .map_err(docs_index_load_error)?;
        let max_items = args
            .max_items
            .unwrap_or(MAX_DOCS_CONTEXT_ITEMS)
            .clamp(1, MAX_DOCS_CONTEXT_ITEMS);
        let budget = QueryBudget::bounded(MAX_QUERY_TIMEOUT_MS, max_items, MAX_SERIALIZED_BYTES);
        let response = match args.action {
            DocsAction::Context => {
                let path = match args.path {
                    Some(path) => Some(normalize_model_path(&path, "ontograph.docs")?),
                    None => None,
                };
                let result = store
                    .context(DocsContextRequest {
                        document_id: trim_optional(args.document_id),
                        path,
                        heading_id: trim_optional(args.heading_id),
                        requirement_id: trim_optional(args.requirement_id),
                        route: trim_optional(args.route),
                        symbol: trim_optional(args.symbol),
                        text: trim_optional(args.text),
                        budget,
                    })
                    .map_err(docs_query_error)?;
                DocsResult {
                    action: DocsAction::Context,
                    items: result.items.items,
                    drift_items: Vec::new(),
                    skipped_drift_references: Vec::new(),
                    trace_items: Vec::new(),
                    readiness: None,
                    truncated: result.items.truncated,
                    limits: vec![
                        format!("docs.context items are capped at {MAX_DOCS_CONTEXT_ITEMS} items"),
                        format!("docs.context result serialization is capped at {MAX_SERIALIZED_BYTES} bytes"),
                        format!("docs.context excerpts are capped at {MAX_DOCS_EXCERPT_BYTES} bytes per item"),
                        "docs.context reads only the active native docs-index.json sidecar paired with the active source-index.json"
                            .to_string(),
                        "docs.context does not infer docs.trace edges or docs.drift state"
                            .to_string(),
                    ],
                }
            }
            DocsAction::Trace => {
                let path = match args.path {
                    Some(path) => Some(normalize_model_path(&path, "ontograph.docs")?),
                    None => None,
                };
                let result = store
                    .trace(DocsTraceRequest {
                        document_id: trim_optional(args.document_id),
                        requirement_id: trim_optional(args.requirement_id),
                        route: trim_optional(args.route),
                        path,
                        symbol: trim_optional(args.symbol),
                        budget,
                    })
                    .map_err(docs_trace_query_error)?;
                DocsResult {
                    action: DocsAction::Trace,
                    items: Vec::new(),
                    drift_items: Vec::new(),
                    skipped_drift_references: Vec::new(),
                    trace_items: result.items.items,
                    readiness: None,
                    truncated: result.items.truncated,
                    limits: vec![
                        format!("docs.trace items are capped at {MAX_DOCS_CONTEXT_ITEMS} items"),
                        format!("docs.trace result serialization is capped at {MAX_SERIALIZED_BYTES} bytes"),
                        format!("docs.trace excerpts are capped at {MAX_DOCS_EXCERPT_BYTES} bytes per fact"),
                        "docs.trace reads only explicit native docs-index.json documents, facts, and links paired with the active source-index.json"
                            .to_string(),
                        "docs.trace does not infer semantic, graph, route, drift, or donor-runtime edges"
                            .to_string(),
                    ],
                }
            }
            DocsAction::Drift => {
                let source_index = SourceIndexStore::load_active_generation(&artifact_root)
                    .map_err(docs_source_index_load_error)?;
                let source = source_index.source_references();
                let result = store
                    .drift(DocsDriftRequest { budget }, &source)
                    .map_err(docs_query_error)?;
                DocsResult {
                    action: DocsAction::Drift,
                    items: Vec::new(),
                    drift_items: result.items.items,
                    skipped_drift_references: result.skipped.items,
                    trace_items: Vec::new(),
                    readiness: None,
                    truncated: result.items.truncated || result.skipped.truncated,
                    limits: vec![
                        format!("docs.drift items are capped at {MAX_DOCS_CONTEXT_ITEMS} items"),
                        format!("docs.drift result serialization is capped at {MAX_SERIALIZED_BYTES} bytes"),
                        "docs.drift compares only explicit docs-index paths, symbols, routes, and document links against the active source-index.json"
                            .to_string(),
                        "docs.drift skips unsupported reference kinds and does not infer semantic, graph, audit, or donor-runtime drift"
                            .to_string(),
                    ],
                }
            }
            DocsAction::Readiness => {
                DocsResult {
                    action: DocsAction::Readiness,
                    items: Vec::new(),
                    drift_items: Vec::new(),
                    skipped_drift_references: Vec::new(),
                    trace_items: Vec::new(),
                    readiness: Some(store.readiness()),
                    truncated: false,
                    limits: vec![
                        "docs.readiness reads only the active native docs-index.json sidecar paired with the active source-index.json"
                            .to_string(),
                        "docs.readiness reports artifact health counts and source-index pairing only"
                            .to_string(),
                        "docs.readiness does not infer docs.trace edges or docs.drift state"
                            .to_string(),
                    ],
                }
            }
        };
        let value = serde_json::to_value(response).map_err(|err| {
            FunctionCallError::RespondToModel(format!(
                "failed to serialize ontograph.docs result: {err}"
            ))
        })?;
        Ok(Box::new(JsonToolOutput::new(value)))
    }
}

fn docs_args_have_trace_selector(args: &DocsArgs) -> bool {
    [
        args.document_id.as_deref(),
        args.requirement_id.as_deref(),
        args.route.as_deref(),
        args.path.as_deref(),
        args.symbol.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}

fn docs_args_have_context_only_selector(args: &DocsArgs) -> bool {
    [args.heading_id.as_deref(), args.text.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| !value.trim().is_empty())
}

fn docs_args_have_context_selector(args: &DocsArgs) -> bool {
    [
        args.document_id.as_deref(),
        args.path.as_deref(),
        args.heading_id.as_deref(),
        args.requirement_id.as_deref(),
        args.route.as_deref(),
        args.symbol.as_deref(),
        args.text.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}
