#[tokio::test]
async fn discover_tools_reports_only_live_native_ontograph_tools() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "tools" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
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
        .expect("discover should report live tools");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: Vec::new(),
            packs: Vec::new(),
            tools: vec![
                ImplementedToolSummary {
                    full_name: "ontograph.explain_module".to_string(),
                    description:
                        "Inspect one local UTF-8 file and return bounded module facts without building an index."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.discover".to_string(),
                    description:
                        "Report the live native Ontograph tool surface and checked-in parity metadata."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.docs".to_string(),
                    description:
                        "Read bounded docs-only context, trace, drift, and readiness from the active native Ontograph docs sidecar. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.audit".to_string(),
                    description:
                        "Run one bounded read-only audit report rooted at the active Ontograph workspace."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.impact".to_string(),
                    description:
                        "Read bounded symbol impact from the active native Ontograph source index."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.inspect".to_string(),
                    description:
                        "Read bounded context, evidence, or route shape from the active native Ontograph source index."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.km_context".to_string(),
                    description:
                        "Read one bounded active Knowledge Module context fragment from the native Ontograph KM cache."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.manage".to_string(),
                    description:
                "Build the native Ontograph source index, manage audit sessions, or read bounded route-map records from the active index."
                            .to_string(),
                },
                ImplementedToolSummary {
                    full_name: "ontograph.search".to_string(),
                    description:
                        "Read bounded symbol matches from the active native Ontograph source index."
                            .to_string(),
                },
            ],
            repos: Vec::new(),
            routes: Vec::new(),
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only live native Ontograph tools are listed.".to_string(),
                "Donor facade placeholders are intentionally excluded.".to_string(),
                "Docs-backed actions require a docs-index.json sidecar, which ontograph.manage action `index` publishes alongside the source index."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn discover_repos_reports_active_manifest_metadata() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "repos" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
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
        .expect("discover repos should read active manifest");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: Vec::new(),
            packs: Vec::new(),
            tools: Vec::new(),
            repos: vec![DiscoveredRepoSummary {
                root: workspace.display().to_string(),
                vcs: "git".to_string(),
                remote_url: None,
                active_generation_id: test_target_head(&workspace),
                target_head: test_target_head(&workspace),
                files: 1,
                symbols: 2,
                relationships: 1,
            }],
            routes: Vec::new(),
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only the current workspace active native Ontograph artifact is listed."
                    .to_string(),
                "No donor repo registry, daemon, sync, or group model is used.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn discover_routes_reports_active_source_index_routes() {
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
        arguments: json!({ "action": "routes", "route": "/api/users/{id}" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover routes should read active source index routes");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: Vec::new(),
            packs: Vec::new(),
            tools: Vec::new(),
            repos: Vec::new(),
            routes: vec![DiscoveredRouteSummary {
                method: "POST".to_string(),
                route_path: "/api/users/{param}".to_string(),
                handler_symbol: "create_user".to_string(),
                path: "src/lib.rs".to_string(),
                line: 7,
            }],
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only active native source-index route records are listed.".to_string(),
                "No route shape, consumer, middleware, daemon, or donor runtime analysis is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn discover_groups_reports_single_synthetic_current_workspace_group() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "groups" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover groups should report the synthetic current workspace group");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: vec![DiscoveredGroupSummary {
                name: "current-workspace".to_string(),
                repo_roots: vec![workspace.display().to_string()],
                artifact_root: workspace_artifact_root(&workspace)
                    .root_path()
                    .display()
                    .to_string(),
            }],
            packs: Vec::new(),
            tools: Vec::new(),
            repos: Vec::new(),
            routes: Vec::new(),
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only one synthetic local group for the current workspace is listed.".to_string(),
                "No donor group registry, manifest sync, or cross-workspace state is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn discover_groups_uses_workspace_context_without_creating_registry_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "groups", "name": "current-workspace" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover groups should not require manifest or registry state");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.groups[0].name, "current-workspace");
    assert_eq!(
        result.groups[0].repo_roots,
        vec![workspace.display().to_string()]
    );
    assert_eq!(
        result.groups[0].artifact_root,
        workspace_artifact_root(&workspace)
            .root_path()
            .display()
            .to_string()
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
    assert!(!group_state_dir(&workspace).exists());
}

#[test]
fn group_state_write_readback_uses_repo_local_groups_path() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let state = CurrentWorkspaceGroupState::new(
        vec![workspace.display().to_string()],
        workspace.join(".ontocode/ontograph").display().to_string(),
    );

    write_current_workspace_group_state(&workspace, &state).expect("write group state");
    let readback = read_current_workspace_group_state(&workspace).expect("read group state");

    assert_eq!(readback, Some(state));
    assert!(
        workspace
            .join(".ontocode/ontograph/groups/current-workspace.json")
            .exists()
    );
    assert!(
        workspace_artifact_root(&workspace)
            .root_path()
            .join("groups/current-workspace.json")
            .is_file()
    );
    assert!(!workspace.join(".ontograph/groups").exists());
}

#[tokio::test]
async fn discover_groups_reads_valid_repo_local_group_state_when_present() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let state = CurrentWorkspaceGroupState::new(
        vec![workspace.join("state-root").display().to_string()],
        workspace.join(".ontocode/ontograph").display().to_string(),
    );
    write_current_workspace_group_state(&workspace, &state).expect("write group state");

    let result = discover_tool_result(
        &workspace,
        json!({ "action": "groups", "name": CURRENT_WORKSPACE_GROUP }),
    )
    .await;

    assert_eq!(
        result.groups,
        vec![DiscoveredGroupSummary {
            name: CURRENT_WORKSPACE_GROUP.to_string(),
            repo_roots: state.repo_roots,
            artifact_root: state.artifact_root,
        }]
    );
}

#[tokio::test]
async fn discover_groups_fails_closed_for_corrupt_repo_local_group_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".ontocode/ontograph/groups"))
        .expect("create groups dir");
    std::fs::write(
        workspace.join(".ontocode/ontograph/groups/current-workspace.json"),
        "{not json\n",
    )
    .expect("write corrupt group state");

    let err = discover_tool_error(&workspace, json!({ "action": "groups" })).await;

    assert!(
        err.to_string().contains(
            "ontograph.discover failed to read current-workspace group state: corrupt group state"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn discover_groups_fails_closed_for_missing_repo_local_group_state_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".ontocode/ontograph/groups"))
        .expect("create groups dir");

    let err = discover_tool_error(&workspace, json!({ "action": "groups" })).await;

    assert!(
        err.to_string().contains(
            "ontograph.discover failed to read current-workspace group state: corrupt group state"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn discover_groups_fails_closed_for_blank_repo_root_entries() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".ontocode/ontograph/groups"))
        .expect("create groups dir");
    let state = json!({
        "schema_version": 1,
        "name": CURRENT_WORKSPACE_GROUP,
        "repo_roots": [" "],
        "artifact_root": workspace.join(".ontocode/ontograph").display().to_string(),
    });
    std::fs::write(
        workspace.join(".ontocode/ontograph/groups/current-workspace.json"),
        serde_json::to_string_pretty(&state).expect("serialize state"),
    )
    .expect("write incomplete group state");

    let err = discover_tool_error(&workspace, json!({ "action": "groups" })).await;

    assert!(
        err.to_string().contains(
            "ontograph.discover failed to read current-workspace group state: corrupt group state"
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn failed_group_state_write_preserves_previous_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let original = CurrentWorkspaceGroupState::new(
        vec![workspace.display().to_string()],
        workspace.join(".ontocode/ontograph").display().to_string(),
    );
    let replacement = CurrentWorkspaceGroupState::new(
        vec![workspace.join("replacement").display().to_string()],
        workspace
            .join(".ontocode/ontograph/replacement")
            .display()
            .to_string(),
    );
    write_current_workspace_group_state(&workspace, &original).expect("write original state");

    let err = write_current_workspace_group_state_with(&workspace, &replacement, |_, _| {
        Err(io::Error::other("simulated atomic write failure"))
    })
    .expect_err("simulated write should fail");
    let readback = read_current_workspace_group_state(&workspace).expect("read original state");

    assert_eq!(err, GroupStateError::Write);
    assert_eq!(readback, Some(original));
}

#[test]
fn failed_first_group_state_write_restores_absent_state_dir() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    let state = CurrentWorkspaceGroupState::new(
        vec![workspace.display().to_string()],
        workspace.join(".ontocode/ontograph").display().to_string(),
    );

    let err = write_current_workspace_group_state_with(&workspace, &state, |_, _| {
        Err(io::Error::other("simulated atomic write failure"))
    })
    .expect_err("simulated write should fail");
    let readback = read_current_workspace_group_state(&workspace).expect("read no state");

    assert_eq!(err, GroupStateError::Write);
    assert_eq!(readback, None);
    assert!(!group_state_dir(&workspace).exists());
}

#[tokio::test]
async fn default_read_families_do_not_create_repo_local_group_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let reads = [
        (DISCOVER_TOOL_NAME, json!({ "action": "groups" })),
        (
            SEARCH_TOOL_NAME,
            json!({ "action": "semantic", "query": "indexed" }),
        ),
        (
            INSPECT_TOOL_NAME,
            json!({ "action": "context", "path": "src/lib.rs" }),
        ),
        (AUDIT_TOOL_NAME, json!({ "action": "build" })),
        (DOCS_TOOL_NAME, json!({ "action": "readiness" })),
        (MANAGE_TOOL_NAME, json!({ "action": "route_map" })),
    ];

    for (tool_name, arguments) in reads {
        call_tool_value(&workspace, tool_name, arguments).await;
        assert!(
            !group_state_dir(&workspace).exists(),
            "{tool_name} should not create repo-local group state"
        );
    }
}
