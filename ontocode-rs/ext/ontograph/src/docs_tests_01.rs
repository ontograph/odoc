#[test]
fn stale_source_index_fails_closed_without_publishing_generation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("workspace");
    std::fs::write(workspace.join("src/lib.rs"), "fn stale() {}\n").expect("source");
    let state = OntographThreadState::default();
    state.set_current_cwd(workspace.clone());

    let err = state
        .load_current_source_index("ontograph.search")
        .expect_err("stale reader must fail closed");

    assert!(
        err.to_string().contains(
            "ontograph.search source index is stale for the current workspace state; run ontograph.manage with action `index` to rebuild it"
        ),
        "unexpected error: {err}"
    );
    let generations = workspace_artifact_root(&workspace)
        .root_path()
        .join("generations");
    assert!(!generations.exists());
}

#[tokio::test]
async fn manage_index_publishes_generation_and_returns_counts() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("workspace");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn caller() { callee(); }\nfn callee() {}\n",
    )
    .expect("source");

    let value = call_tool_value(&workspace, MANAGE_TOOL_NAME, json!({ "action": "index" })).await;
    let result = serde_json::from_value::<ManageResult>(value).expect("manage result");

    assert_eq!(result.action, ManageAction::Index);
    assert!(
        result
            .generation_id
            .as_deref()
            .is_some_and(|id| !id.is_empty())
    );
    assert_eq!(
        result.counts,
        Some(crate::manifest::GraphCountSummary {
            files: 1,
            symbols: 2,
            relationships: 1,
            nodes: 3,
            edges: 1,
        })
    );
    assert_eq!(
        std::fs::read_dir(
            workspace_artifact_root(&workspace)
                .root_path()
                .join("generations")
        )
        .expect("generations")
        .count(),
        1
    );
}

#[tokio::test]
async fn golden_native_read_surface_smoke_suite_reports_stable_success_shapes() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let discover =
        call_tool_value(&workspace, DISCOVER_TOOL_NAME, json!({ "action": "tools" })).await;
    assert_eq!(discover["namespace"], "ontograph");
    assert!(
        discover["tools"]
            .as_array()
            .expect("discover.tools array")
            .iter()
            .any(|tool| tool["full_name"] == "ontograph.search")
    );

    let search = call_tool_value(
        &workspace,
        SEARCH_TOOL_NAME,
        json!({ "action": "semantic", "query": "indexed", "max_items": 1 }),
    )
    .await;
    assert_eq!(search["action"], "semantic");
    assert_eq!(search["query"], "indexed");
    assert_eq!(search["matches"][0]["symbol"], "indexed");

    let inspect = call_tool_value(
        &workspace,
        INSPECT_TOOL_NAME,
        json!({ "action": "context", "path": "src/lib.rs", "max_items": 1 }),
    )
    .await;
    assert_eq!(inspect["path"], "src/lib.rs");
    assert_eq!(inspect["language"], "rust");
    assert_eq!(inspect["symbols"][0]["symbol"], "build_router");

    let impact = call_tool_value(
        &workspace,
        IMPACT_TOOL_NAME,
        json!({ "action": "symbol", "symbol": "indexed", "max_items": 1 }),
    )
    .await;
    assert_eq!(impact["symbol"], "indexed");
    assert!(impact["upstream"].as_array().is_some());
    assert!(impact["downstream"].as_array().is_some());

    let audit = audit_report_result(&workspace).await;
    assert_eq!(audit.action, crate::audit_build::AuditAction::Report);
    assert_eq!(
        audit.source_index.status,
        AuditReportSourceIndexStatus::Ready
    );

    let docs = call_tool_value(&workspace, DOCS_TOOL_NAME, json!({ "action": "readiness" })).await;
    assert_eq!(docs["action"], "readiness");
    assert_eq!(docs["readiness"]["status"], "ready");
    assert_eq!(docs["readiness"]["paired"], true);
}

#[tokio::test]
async fn discover_groups_fail_closed_for_unknown_name() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "groups",
                        "name": "other-workspace"
                    })
                    .to_string(),
                },
            })
            .await,
        "discover groups should fail closed for unsupported names",
    );

    assert!(
        err.to_string().contains(
            "ontograph.discover group name `other-workspace` is not supported; only `current-workspace` is available"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn discover_sync_writes_reads_back_current_workspace_group_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let result = discover_tool_result(
        &workspace,
        json!({ "action": "sync", "name": CURRENT_WORKSPACE_GROUP }),
    )
    .await;
    let readback = read_current_workspace_group_state(&workspace).expect("read synced state");

    assert_eq!(
        result.groups,
        vec![DiscoveredGroupSummary {
            name: CURRENT_WORKSPACE_GROUP.to_string(),
            repo_roots: vec![workspace.display().to_string()],
            artifact_root: workspace_artifact_root(&workspace)
                .root_path()
                .display()
                .to_string(),
        }]
    );
    assert_eq!(
        readback,
        Some(CurrentWorkspaceGroupState::new(
            vec![workspace.display().to_string()],
            workspace_artifact_root(&workspace)
                .root_path()
                .display()
                .to_string()
        ))
    );
    assert!(
        workspace
            .join(".ontocode/ontograph/groups/current-workspace.json")
            .exists()
    );
}

#[tokio::test]
async fn discover_groups_reads_state_after_explicit_sync() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let synced = discover_tool_result(&workspace, json!({ "action": "sync" })).await;
    let groups = discover_tool_result(&workspace, json!({ "action": "groups" })).await;

    assert_eq!(groups.groups, synced.groups);
}

#[tokio::test]
async fn discover_sync_fails_closed_for_unknown_name() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let err = discover_tool_error(&workspace, json!({ "action": "sync", "name": "other" })).await;

    assert!(
        err.to_string().contains(
            "group name `other` is not supported; only `current-workspace` can be synced"
        ),
        "unexpected error: {err}"
    );
    assert!(!group_state_dir(&workspace).exists());
}

#[tokio::test]
async fn discover_sync_fails_closed_without_workspace_context() {
    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();
    let tool = registry.tool_contributors()[0]
        .tools(
            &ExtensionData::new("session"),
            &ExtensionData::new("thread"),
        )
        .into_iter()
        .find(|tool| {
            tool.tool_name() == ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME)
        })
        .expect("discover tool");
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "sync" }).to_string(),
    };

    let err = expect_tool_error(
        tool.handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload,
        })
        .await,
        "sync should require workspace context",
    );

    assert!(
        err.to_string()
            .contains("ontograph.discover workspace context is unavailable for this turn"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn discover_sync_fails_closed_for_corrupt_existing_group_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".ontocode/ontograph/groups"))
        .expect("create groups dir");
    let state_path = workspace.join(".ontocode/ontograph/groups/current-workspace.json");
    std::fs::write(&state_path, "{not json\n").expect("write corrupt group state");

    let err = discover_tool_error(&workspace, json!({ "action": "sync" })).await;
    let contents = std::fs::read_to_string(state_path).expect("read corrupt group state");

    assert!(
        err.to_string().contains(
            "ontograph.discover failed to read current-workspace group state before sync: corrupt group state"
        ),
        "unexpected error: {err}"
    );
    assert_eq!(contents, "{not json\n");
}

#[tokio::test]
async fn discover_sync_does_not_register_donor_group_surfaces() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    for name in [
        "donor-registry",
        "daemon-group",
        "plugin-group",
        "cross-workspace-sync",
    ] {
        let err = discover_tool_error(&workspace, json!({ "action": "sync", "name": name })).await;
        assert!(
            err.to_string()
                .contains("is not supported; only `current-workspace` can be synced"),
            "unexpected error for {name}: {err}"
        );
    }
}

#[tokio::test]
async fn manage_route_map_reports_active_source_index_routes() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"
use axum::routing::{get, post};

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/readyz", get(readiness_handler))
        .route("/api/users/:id/", post(create_user))
}

async fn readiness_handler() {}
async fn create_user() {}
"#,
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "route_map",
            "path": "src/lib.rs",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, MANAGE_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("manage route_map should read active source index routes");

    let result = serde_json::from_value::<ManageResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ManageResult {
            action: ManageAction::RouteMap,
            generation_id: None,
            counts: None,
            session: None,
            lock: None,
            active_source_identity: None,
            session_path: None,
            lock_path: None,
            notes: vec![],
            routes: vec![DiscoveredRouteSummary {
                method: "POST".to_string(),
                route_path: "/api/users/{param}".to_string(),
                handler_symbol: "create_user".to_string(),
                path: "src/lib.rs".to_string(),
                line: 7,
            }],
            routes_truncated: true,
            limits: vec![
                "matched routes are capped at 200 items".to_string(),
                "manage.route_map reads only the active native source-index route records"
                    .to_string(),
                "No route consumer, middleware, flow, daemon, or donor runtime analysis is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn manage_route_map_filters_and_normalizes_route_only_requests() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"
use axum::routing::{get, post};

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/readyz", get(readiness_handler))
        .route("/api/users/:id/", post(create_user))
}

async fn readiness_handler() {}
async fn create_user() {}
"#,
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "route_map",
            "route": "/API/USERS/:id/"
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, MANAGE_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("manage route_map should normalize and filter route-only requests");

    let result = serde_json::from_value::<ManageResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ManageResult {
            action: ManageAction::RouteMap,
            generation_id: None,
            counts: None,
            session: None,
            lock: None,
            active_source_identity: None,
            session_path: None,
            lock_path: None,
            notes: vec![],
            routes: vec![DiscoveredRouteSummary {
                method: "POST".to_string(),
                route_path: "/api/users/{param}".to_string(),
                handler_symbol: "create_user".to_string(),
                path: "src/lib.rs".to_string(),
                line: 7,
            }],
            routes_truncated: false,
            limits: vec![
                "matched routes are capped at 200 items".to_string(),
                "manage.route_map reads only the active native source-index route records"
                    .to_string(),
                "No route consumer, middleware, flow, daemon, or donor runtime analysis is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn manage_route_map_fails_closed_when_path_is_not_present() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let error = expect_tool_error(
        install_and_find_named_tool(&workspace, MANAGE_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "route_map",
                        "path": "src/missing.rs"
                    })
                    .to_string(),
                },
            })
            .await,
        "manage route_map should fail closed for a missing path",
    );

    assert!(
        error
            .to_string()
            .contains("ontograph.manage path is not present in source-index.json"),
        "unexpected error: {error:?}"
    );
}

#[tokio::test]
async fn inspect_shape_tool_reports_bounded_active_route_shapes() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        nested.join("mod.rs"),
        r#"
use axum::routing::{get, post};

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/readyz", get(readiness_handler))
        .route("/api/users/:id/", post(create_user))
}

async fn readiness_handler() {}
async fn create_user() {}
"#,
    )
    .expect("write nested module");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "shape",
            "path": "mod.rs",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&nested, INSPECT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("inspect shape should read active source index routes");

    let result = serde_json::from_value::<InspectShapeResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        InspectShapeResult {
            routes: vec![InspectShapeRouteSummary {
                method: "POST".to_string(),
                route_path: "/api/users/{param}".to_string(),
                handler_symbol: "create_user".to_string(),
                path: "src/nested/mod.rs".to_string(),
                line: 7,
            }],
            routes_truncated: true,
            limits: vec![
                "matched routes are capped at 200 items".to_string(),
                "inspect.shape reads only the active native source-index route records".to_string(),
                "Only direct indexed Axum route shapes are summarized.".to_string(),
                "No route consumer, middleware, daemon, or donor runtime analysis is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn inspect_shape_tool_requires_route_or_path() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let error = expect_tool_error(
        install_and_find_named_tool(&workspace, INSPECT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "shape"
                    })
                    .to_string(),
                },
            })
            .await,
        "inspect shape should fail closed without route or path",
    );

    assert!(
        error
            .to_string()
            .contains("ontograph.inspect route or path is required for action `shape`"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn inspect_shape_tool_fails_closed_when_path_is_not_present() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let error = expect_tool_error(
        install_and_find_named_tool(&workspace, INSPECT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "shape",
                        "path": "missing.rs"
                    })
                    .to_string(),
                },
            })
            .await,
        "inspect shape should fail closed for a missing path",
    );

    assert!(
        error
            .to_string()
            .contains("ontograph.inspect path is not present in source-index.json"),
        "unexpected error: {error}"
    );
}
