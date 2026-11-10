#[tokio::test]
async fn discover_repos_reads_active_manifest_from_repo_root_when_cwd_is_nested() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "repos" }).to_string(),
    };
    let output = install_and_find_named_tool(&nested, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover repos should read active manifest from repo root");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.repos.len(), 1);
    assert_eq!(result.repos[0].root, workspace.display().to_string());
    assert_eq!(
        result.repos[0].active_generation_id,
        test_target_head(&workspace)
    );
}

#[tokio::test]
async fn discover_repos_fails_closed_for_stale_manifest_metadata() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    )
    .expect("update source");

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
                    arguments: json!({ "action": "repos" }).to_string(),
                },
            })
            .await,
        "discover repos should fail closed for stale manifests",
    );

    assert!(
        err.to_string()
            .contains("ontograph.discover source index is stale for the current workspace state"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn discover_repos_stays_inside_inner_rust_workspace_under_outer_git_checkout() {
    let dir = tempdir().expect("temp dir");
    let outer = dir.path().join("outer");
    let workspace = outer.join("inner-workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(outer.join(".git")).expect("create outer git marker");
    std::fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"inner-workspace\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write cargo manifest");
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "repos" }).to_string(),
    };
    let output = install_and_find_named_tool(&nested, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover repos should stay inside the inner workspace");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.repos.len(), 1);
    assert_eq!(result.repos[0].root, workspace.display().to_string());
    assert_eq!(
        result.repos[0].active_generation_id,
        test_target_head(&workspace)
    );
}

#[tokio::test]
async fn discover_repos_does_not_build_source_index_for_fresh_workspace() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");

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
                    arguments: json!({ "action": "repos" }).to_string(),
                },
            })
            .await,
        "discover repos should fail closed without an active manifest",
    );

    assert!(
        err.to_string()
            .contains("ontograph.discover active manifest pointer is missing or unreadable"),
        "unexpected error: {err}"
    );
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
async fn impact_symbol_tool_reads_active_source_index_relationships() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    )
    .expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: test_target_head(&workspace),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "symbol",
            "symbol": "callee",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact should read active source index");

    let result = serde_json::from_value::<ImpactSymbolResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ImpactSymbolResult {
            symbol: "callee".to_string(),
            upstream: vec![ImpactSymbolSummary {
                symbol: "caller".to_string(),
                path: "lib.rs".to_string(),
                relationship: "CALLS".to_string(),
                site_line: 2,
            }],
            downstream: Vec::new(),
            upstream_truncated: false,
            downstream_truncated: false,
            limits: vec![
                "impact items are capped at 200 per direction".to_string(),
                "impact.symbol reads only the active native source-index artifact".to_string(),
                "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn impact_symbol_tool_fails_closed_for_ambiguous_symbol_names() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/a.rs"),
        "mod nested {\n    pub fn run() {}\n}\n",
    )
    .expect("write a.rs");
    std::fs::write(
        workspace.join("src/b.rs"),
        "fn caller() {\n    run();\n}\npub fn run() {}\n",
    )
    .expect("write b.rs");
    index_workspace_for_tests(&workspace);

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "symbol",
                        "symbol": "run",
                        "max_items": 1
                    })
                    .to_string(),
                },
            })
            .await,
        "ambiguous impact symbol should fail closed",
    );

    assert!(
        err.to_string()
            .contains("symbol is ambiguous across multiple indexed definitions"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn impact_symbol_tool_reads_active_source_index_from_nested_workspace_cwd() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    )
    .expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: test_target_head(&workspace),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "symbol",
            "symbol": "callee",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&nested, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact should read active source index from repo root");

    let result = serde_json::from_value::<ImpactSymbolResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result.upstream,
        vec![ImpactSymbolSummary {
            symbol: "caller".to_string(),
            path: "lib.rs".to_string(),
            relationship: "CALLS".to_string(),
            site_line: 2,
        }]
    );
}

#[tokio::test]
async fn impact_batch_tool_reads_active_source_index_relationships() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn root() {\n    alpha();\n    beta();\n}\nfn alpha() {\n    gamma();\n}\nfn beta() {}\nfn gamma() {}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "batch",
            "symbols": ["alpha", "gamma"],
            "max_items": 2
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact batch should read active source index");

    let result = serde_json::from_value::<ImpactBatchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ImpactBatchResult {
            results: vec![
                ImpactBatchItemResult {
                    symbol: "alpha".to_string(),
                    upstream: vec![ImpactSymbolSummary {
                        symbol: "root".to_string(),
                        path: "lib.rs".to_string(),
                        relationship: "CALLS".to_string(),
                        site_line: 2,
                    }],
                    downstream: vec![ImpactSymbolSummary {
                        symbol: "gamma".to_string(),
                        path: "lib.rs".to_string(),
                        relationship: "CALLS".to_string(),
                        site_line: 6,
                    }],
                    upstream_truncated: false,
                    downstream_truncated: false,
                },
                ImpactBatchItemResult {
                    symbol: "gamma".to_string(),
                    upstream: vec![ImpactSymbolSummary {
                        symbol: "alpha".to_string(),
                        path: "lib.rs".to_string(),
                        relationship: "CALLS".to_string(),
                        site_line: 6,
                    }],
                    downstream: Vec::new(),
                    upstream_truncated: false,
                    downstream_truncated: false,
                },
            ],
            limits: vec![
                "batch symbol lists are capped at 200 items".to_string(),
                "impact items are capped at 200 per direction".to_string(),
                "impact.batch reads only the active native source-index artifact".to_string(),
                "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn impact_route_tool_matches_route_records_and_reuses_handler_symbol_impact() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"use axum::routing::get;

fn helper() {
    readiness_handler();
}

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/readyz", get(readiness_handler))
}

async fn readiness_handler() {}
"#,
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "route",
            "route": "/readyz",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact route should reuse route and symbol impact facts");

    let result = serde_json::from_value::<ImpactRouteResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ImpactRouteResult {
            results: vec![ImpactRouteItemResult {
                route: DiscoveredRouteSummary {
                    method: "GET".to_string(),
                    route_path: "/readyz".to_string(),
                    handler_symbol: "readiness_handler".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 9,
                },
                upstream: vec![ImpactSymbolSummary {
                    symbol: "helper".to_string(),
                    path: "src/lib.rs".to_string(),
                    relationship: "CALLS".to_string(),
                    site_line: 4,
                }],
                downstream: Vec::new(),
                upstream_truncated: false,
                downstream_truncated: false,
            }],
            routes_truncated: false,
            limits: vec![
                "matched routes are capped at 200 items".to_string(),
                "impact items are capped at 200 per direction".to_string(),
                "impact.route matches active native source-index route records and reuses handler symbol impact from the same artifact"
                    .to_string(),
                "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn impact_route_tool_filters_routes_by_file_path() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/admin.rs"),
        r#"use axum::routing::get;

fn admin_router() -> axum::Router {
    axum::Router::new()
        .route("/admin/health", get(admin_health))
}

async fn admin_health() {}
"#,
    )
    .expect("write admin source");
    std::fs::write(
        workspace.join("src/public.rs"),
        r#"use axum::routing::get;

fn public_router() -> axum::Router {
    axum::Router::new()
        .route("/public/health", get(public_health))
}

async fn public_health() {}
"#,
    )
    .expect("write public source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "route",
            "file": "src/admin.rs",
            "max_items": 10
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact route should filter route matches by file");

    let result = serde_json::from_value::<ImpactRouteResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result.results,
        vec![ImpactRouteItemResult {
            route: DiscoveredRouteSummary {
                method: "GET".to_string(),
                route_path: "/admin/health".to_string(),
                handler_symbol: "admin_health".to_string(),
                path: "src/admin.rs".to_string(),
                line: 5,
            },
            upstream: Vec::new(),
            downstream: Vec::new(),
            upstream_truncated: false,
            downstream_truncated: false,
        }]
    );
    assert!(!result.routes_truncated);
}

#[tokio::test]
async fn impact_route_tool_requires_route_or_file_filter() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(2048),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "route"
                    })
                    .to_string(),
                },
            })
            .await,
        "impact route should fail closed without route or file",
    );

    assert!(
        err.to_string()
            .contains("ontograph.impact route or file is required for action `route`"),
        "unexpected error: {err}"
    );
}
