#[test]
fn install_registers_stable_ontograph_tool_surface_without_workspace_context() {
    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();

    let tools = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| {
            contributor.tools(
                &ExtensionData::new("session"),
                &ExtensionData::new("thread"),
            )
        })
        .collect::<Vec<_>>();
    let tool_names = tools
        .iter()
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();

    assert_eq!(
        tool_names,
        vec![
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EVIDENCE_PLANNER_TOOL_NAME),
        ]
    );

    for tool in tools {
        let ToolSpec::Namespace(namespace) = tool.spec() else {
            panic!("Ontograph tools must use the Ontograph namespace");
        };
        for nested_tool in namespace.tools {
            let ResponsesApiNamespaceTool::Function(function) = nested_tool;
            assert!(
                function
                    .name
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric()
                        || matches!(character, '_' | '-')),
                "invalid Responses API namespace tool name: {}",
                function.name
            );
        }
    }
}

#[tokio::test]
async fn install_registers_index_backed_tools_when_workspace_artifact_exists() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);

    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: Vec::new(),
                environments: vec![TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: workspace,
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    let tool_names = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();

    assert_eq!(
        tool_names,
        vec![
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EVIDENCE_PLANNER_TOOL_NAME),
        ]
    );
}

#[tokio::test]
async fn install_keeps_stable_tool_surface_after_contribute_without_creating_artifact() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");

    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    let tool_names_before = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();
    registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: Vec::new(),
                environments: vec![TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: workspace.clone(),
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    let tool_names = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();

    assert_eq!(
        tool_names_before,
        vec![
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EVIDENCE_PLANNER_TOOL_NAME),
        ]
    );
    assert_eq!(tool_names, tool_names_before);
    assert!(
        !workspace
            .join(".ontocode/ontograph/current-generation")
            .exists()
    );
    assert!(
        !workspace_artifact_root(&workspace)
            .current_pointer_path()
            .exists()
    );
}

#[tokio::test]
async fn install_registers_index_backed_tools_from_nested_workspace_cwd() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: Vec::new(),
                environments: vec![TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: nested,
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    let tool_names = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();

    assert_eq!(
        tool_names,
        vec![
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            ToolName::namespaced(ONTOGRAPH_NAMESPACE, EVIDENCE_PLANNER_TOOL_NAME),
        ]
    );
}

#[test]
fn repo_does_not_restore_removed_ontoindex_plugin_path() {
    let repo_root = repo_root().expect("repo root");
    let forbidden_paths = [
        "plugins/ontocode-ontoindex",
        "ontocode-rs/ontoindex-mcp",
        "ontocode-rs/ontoindex-backend",
        "ontocode-rs/ontoindex-query",
    ];

    for path in forbidden_paths {
        assert!(!repo_root.join(path).exists(), "{path} must stay absent");
    }
}

#[tokio::test]
async fn docs_context_tool_reads_valid_docs_sidecar_with_deterministic_budget() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "context",
            "document_id": "doc-adr",
            "max_items": 2
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DOCS_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("docs.context should read valid docs-index");

    let result = serde_json::from_value::<DocsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DocsResult {
            action: DocsAction::Context,
            items: vec![
                DocsContextItem::Document {
                    document_id: "doc-adr".to_string(),
                    path: "docs/adr.md".to_string(),
                    title: "ADR".to_string(),
                    document_kind: "markdown".to_string(),
                    excerpt: "Docs context summary.".to_string(),
                },
                DocsContextItem::Fact {
                    fact_id: "fact-a".to_string(),
                    document_id: "doc-adr".to_string(),
                    heading_id: Some("h-design".to_string()),
                    fact_kind: "requirement".to_string(),
                    text: "Alpha fact".to_string(),
                    line_start: 3,
                    line_end: 3,
                    requirement_id: Some("REQ-1".to_string()),
                },
            ],
            drift_items: Vec::new(),
            skipped_drift_references: Vec::new(),
            trace_items: Vec::new(),
            readiness: None,
            truncated: true,
            limits: vec![
                "docs.context items are capped at 100 items".to_string(),
                "docs.context result serialization is capped at 65536 bytes".to_string(),
                "docs.context excerpts are capped at 1024 bytes per item".to_string(),
                "docs.context reads only the active native docs-index.json sidecar paired with the active source-index.json"
                    .to_string(),
                "docs.context does not infer docs.trace edges or docs.drift state".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn docs_readiness_tool_reports_valid_docs_sidecar_pairing_and_counts() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);
    let (source_index_generation_id, source_index_content_hash) =
        current_source_index_metadata_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "readiness" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DOCS_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("docs.readiness should read valid docs-index");

    let result = serde_json::from_value::<DocsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DocsResult {
            action: DocsAction::Readiness,
            items: Vec::new(),
            drift_items: Vec::new(),
            skipped_drift_references: Vec::new(),
            trace_items: Vec::new(),
            readiness: Some(DocsReadinessSummary {
                status: "ready".to_string(),
                paired: true,
                schema_version: 1,
                completed_at_unix_seconds: 1_725_000_001,
                source_index_generation_id,
                source_index_content_hash,
                document_count: 1,
                fact_count: 2,
                link_count: 0,
            }),
            truncated: false,
            limits: vec![
                "docs.readiness reads only the active native docs-index.json sidecar paired with the active source-index.json"
                    .to_string(),
                "docs.readiness reports artifact health counts and source-index pairing only"
                    .to_string(),
                "docs.readiness does not infer docs.trace edges or docs.drift state".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn docs_trace_tool_reads_requirement_id_selector() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "requirement_id": "REQ-1" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![trace_document_item(), trace_fact_a_item()]
    );
    assert_eq!(result.action, DocsAction::Trace);
    assert!(!result.truncated);
}

#[tokio::test]
async fn docs_trace_tool_reads_document_id_selector() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "document_id": "doc-adr" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![
            trace_document_item(),
            trace_fact_a_item(),
            DocsTraceItem::Fact {
                fact_id: "fact-b".to_string(),
                document_id: "doc-adr".to_string(),
                heading_id: Some("h-design".to_string()),
                fact_kind: "requirement".to_string(),
                text: "Beta fact".to_string(),
                line_start: 4,
                line_end: 4,
                requirement_id: Some("REQ-2".to_string()),
            },
        ]
    );
}

#[tokio::test]
async fn docs_trace_tool_reads_route_selector_and_explicit_link() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-route",
                    "from_document_id": "doc-adr",
                    "target_kind": "route",
                    "target": "/api/alpha"
                }
            ])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "route": "/api/alpha" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![
            trace_document_item(),
            trace_fact_a_item(),
            DocsTraceItem::Link {
                link_id: "link-route".to_string(),
                from_document_id: "doc-adr".to_string(),
                to_document_id: None,
                target_kind: "route".to_string(),
                target: "/api/alpha".to_string(),
                requirement_id: None,
                route: Some("/api/alpha".to_string()),
                path: None,
                symbol: None,
            },
        ]
    );
}

#[tokio::test]
async fn docs_trace_tool_reports_explicit_link_target_fields() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-doc",
                    "from_document_id": "doc-adr",
                    "target_kind": "document_id",
                    "target": "doc-adr"
                },
                {
                    "id": "link-req",
                    "from_document_id": "doc-adr",
                    "target_kind": "requirement_id",
                    "target": "REQ-LINK"
                },
                {
                    "id": "link-path",
                    "from_document_id": "doc-adr",
                    "target_kind": "path",
                    "target": "docs/linked.md"
                },
                {
                    "id": "link-symbol",
                    "from_document_id": "doc-adr",
                    "target_kind": "symbol",
                    "target": "linked_symbol"
                }
            ])),
            ..Default::default()
        },
    );

    let req_result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "requirement_id": "REQ-LINK" }),
    )
    .await;
    let path_result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "path": "docs/linked.md" }),
    )
    .await;
    let symbol_result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "symbol": "linked_symbol" }),
    )
    .await;
    let document_result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "document_id": "doc-adr", "max_items": 4 }),
    )
    .await;

    assert_eq!(
        req_result.trace_items,
        vec![
            trace_document_item(),
            DocsTraceItem::Link {
                link_id: "link-req".to_string(),
                from_document_id: "doc-adr".to_string(),
                to_document_id: None,
                target_kind: "requirement_id".to_string(),
                target: "REQ-LINK".to_string(),
                requirement_id: Some("REQ-LINK".to_string()),
                route: None,
                path: None,
                symbol: None,
            },
        ]
    );
    assert_eq!(
        path_result.trace_items,
        vec![
            trace_document_item(),
            DocsTraceItem::Link {
                link_id: "link-path".to_string(),
                from_document_id: "doc-adr".to_string(),
                to_document_id: None,
                target_kind: "path".to_string(),
                target: "docs/linked.md".to_string(),
                requirement_id: None,
                route: None,
                path: Some("docs/linked.md".to_string()),
                symbol: None,
            },
        ]
    );
    assert_eq!(
        symbol_result.trace_items,
        vec![
            trace_document_item(),
            DocsTraceItem::Link {
                link_id: "link-symbol".to_string(),
                from_document_id: "doc-adr".to_string(),
                to_document_id: None,
                target_kind: "symbol".to_string(),
                target: "linked_symbol".to_string(),
                requirement_id: None,
                route: None,
                path: None,
                symbol: Some("linked_symbol".to_string()),
            },
        ]
    );
    assert!(document_result.trace_items.contains(&DocsTraceItem::Link {
        link_id: "link-doc".to_string(),
        from_document_id: "doc-adr".to_string(),
        to_document_id: Some("doc-adr".to_string()),
        target_kind: "document_id".to_string(),
        target: "doc-adr".to_string(),
        requirement_id: None,
        route: None,
        path: None,
        symbol: None,
    }));
}

#[tokio::test]
async fn docs_trace_tool_reads_path_selector() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "path": "src/lib.rs" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![trace_document_item(), trace_fact_a_item()]
    );
}

#[tokio::test]
async fn docs_trace_tool_reads_symbol_selector() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "symbol": "indexed" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![trace_document_item(), trace_fact_a_item()]
    );
}

#[tokio::test]
async fn docs_trace_tool_intersects_multiple_selectors() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({
            "action": "trace",
            "requirement_id": "REQ-1",
            "route": "/api/alpha",
            "symbol": "indexed"
        }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![trace_document_item(), trace_fact_a_item()]
    );
}
