#[tokio::test]
async fn km_context_tool_ignores_obsolete_km() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_valid_km_for_tool_test(dir.path(), "0.1.0");
    store.import(&source).expect("import KM");
    store
        .obsolete("codebase-graph/creating-ontograph", "0.1.0")
        .expect("obsolete KM");
    let tool = KmContextTool::new_with_store(store);
    let payload = ToolPayload::Function {
        arguments: json!({ "id": "codebase-graph/creating-ontograph" }).to_string(),
    };

    let error = expect_tool_error(
        tool.handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload,
        })
        .await,
        "obsolete KM should not resolve",
    );

    assert!(
        error
            .to_string()
            .contains("has no active KM for id `codebase-graph/creating-ontograph`"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn inspect_context_tool_reads_active_source_index() {
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
            "action": "context",
            "path": "lib.rs",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, INSPECT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, INSPECT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("inspect should read active source index");

    let result = serde_json::from_value::<InspectContextResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        InspectContextResult {
            path: "lib.rs".to_string(),
            language: Some("rust".to_string()),
            symbols: vec![InspectSymbolSummary {
                symbol: "caller".to_string(),
                kind: "fn".to_string(),
                path: "lib.rs".to_string(),
            }],
            truncated: true,
            limits: vec![
                "context symbols are capped at 200 items".to_string(),
                "inspect.context reads only the active native source-index artifact".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn inspect_context_tool_resolves_path_relative_to_nested_workspace_cwd() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        nested.join("mod.rs"),
        "pub async fn run() {}\nfn helper() {}\n",
    )
    .expect("write nested module");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "context",
            "path": "mod.rs",
            "max_items": 2
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
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("inspect should resolve nested cwd paths");

    let result = serde_json::from_value::<InspectContextResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.path, "src/nested/mod.rs".to_string());
    assert_eq!(
        result.symbols,
        vec![
            InspectSymbolSummary {
                symbol: "run".to_string(),
                kind: "fn".to_string(),
                path: "src/nested/mod.rs".to_string(),
            },
            InspectSymbolSummary {
                symbol: "helper".to_string(),
                kind: "fn".to_string(),
                path: "src/nested/mod.rs".to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn inspect_context_tool_prefers_nested_workspace_path_over_repo_root_name_collision() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn root_symbol() {}\n").expect("write root file");
    std::fs::write(
        nested.join("lib.rs"),
        "pub async fn nested_symbol() {}\nfn helper() {}\n",
    )
    .expect("write nested file");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "context",
            "path": "lib.rs",
            "max_items": 3
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
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("inspect should resolve the nested same-name file first");

    let result = serde_json::from_value::<InspectContextResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.path, "src/nested/lib.rs".to_string());
    assert_eq!(
        result.symbols,
        vec![
            InspectSymbolSummary {
                symbol: "nested_symbol".to_string(),
                kind: "fn".to_string(),
                path: "src/nested/lib.rs".to_string(),
            },
            InspectSymbolSummary {
                symbol: "helper".to_string(),
                kind: "fn".to_string(),
                path: "src/nested/lib.rs".to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn inspect_evidence_tool_resolves_line_then_symbol_and_reports_unresolved_targets() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        nested.join("mod.rs"),
        "pub async fn run() {}\nfn helper() {}\n",
    )
    .expect("write nested module");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "evidence",
            "targets": ["mod.rs:1", "helper", "missing.rs"],
            "include_snippet": true,
            "context_lines": 1,
            "max_items": 10
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
        .expect("inspect evidence should resolve mixed targets");

    let result = serde_json::from_value::<InspectEvidenceResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        InspectEvidenceResult {
            evidence: vec![
                InspectEvidenceItem {
                    target: "src/nested/mod.rs:1".to_string(),
                    kind: "line".to_string(),
                    path: "src/nested/mod.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(2),
                    snippet: Some("pub async fn run() {}\nfn helper() {}".to_string()),
                    symbol: None,
                    symbol_kind: None,
                },
                InspectEvidenceItem {
                    target: "helper".to_string(),
                    kind: "symbol".to_string(),
                    path: "src/nested/mod.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(2),
                    snippet: Some("pub async fn run() {}\nfn helper() {}".to_string()),
                    symbol: Some("helper".to_string()),
                    symbol_kind: Some("fn".to_string()),
                },
            ],
            unresolved: vec![InspectUnresolvedTarget {
                target: "missing.rs".to_string(),
                reason: "no matching indexed line, file, or symbol found".to_string(),
            }],
            evidence_truncated: false,
            unresolved_truncated: false,
            limits: vec![
                "evidence targets are capped at 200 items".to_string(),
                "evidence context lines are capped at 20".to_string(),
                "inspect.evidence resolves line targets before file paths before exact symbol names"
                    .to_string(),
                "inspect.evidence reads only the active native source-index artifact".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn explain_module_tool_reports_non_rust_utf8_file_without_symbols() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("docs")).expect("create workspace");
    std::fs::write(workspace.join("docs/notes.md"), "# Demo\n\ntext\n").expect("write notes");

    let output = call_explain_module(&workspace, json!({ "path": "docs/notes.md" }))
        .await
        .expect("tool should explain markdown file");
    let result = serde_json::from_value::<ExplainModuleResult>(
        output
            .post_tool_use_response(
                "call-1",
                &ToolPayload::Function {
                    arguments: json!({ "path": "docs/notes.md" }).to_string(),
                },
            )
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ExplainModuleResult {
            path: "docs/notes.md".to_string(),
            language: Some("markdown".to_string()),
            line_count: 3,
            top_level_symbols: Vec::new(),
            limits: vec![
                format!("file reads are capped at {MAX_FILE_BYTES} bytes"),
                "Rust top_level_symbols use syn when the file parses; malformed Rust and non-Rust files use bounded line-based extraction".to_string()
            ],
        }
    );
}

#[tokio::test]
async fn explain_module_tool_clamps_symbol_count() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    let mut content = String::new();
    for index in 0..40 {
        content.push_str(&format!("pub struct S{index};\n"));
    }
    std::fs::write(workspace.join("src/many.rs"), content).expect("write source");

    let tool = install_and_find_tool(&workspace).await;
    let payload = ToolPayload::Function {
        arguments: json!({
            "path": "src/many.rs",
            "max_symbols": 100
        })
        .to_string(),
    };

    let output = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("tool should explain module");

    let result = serde_json::from_value::<ExplainModuleResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");
    assert_eq!(result.top_level_symbols.len(), 32);
    assert_eq!(result.top_level_symbols.first(), Some(&"S0".to_string()));
    assert_eq!(result.top_level_symbols.last(), Some(&"S31".to_string()));
}

#[tokio::test]
async fn explain_module_tool_reports_async_top_level_functions() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::write(
        workspace.join("src/async_mod.rs"),
        "pub async fn run() {}\nasync fn helper() {}\n",
    )
    .expect("write source");

    let output = call_explain_module(&workspace, json!({ "path": "src/async_mod.rs" }))
        .await
        .expect("tool should explain async rust file");
    let result = serde_json::from_value::<ExplainModuleResult>(
        output
            .post_tool_use_response(
                "call-1",
                &ToolPayload::Function {
                    arguments: json!({ "path": "src/async_mod.rs" }).to_string(),
                },
            )
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result.top_level_symbols,
        vec!["run".to_string(), "helper".to_string()]
    );
}

#[tokio::test]
async fn explain_module_tool_rejects_parent_paths() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let tool = install_and_find_tool(&workspace).await;
    let payload = ToolPayload::Function {
        arguments: json!({
            "path": "../Cargo.toml"
        })
        .to_string(),
    };

    let result = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload,
        })
        .await;
    let err = match result {
        Ok(_) => panic!("parent path should be rejected"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("relative and stay within the current working directory")
    );
}

#[tokio::test]
async fn explain_module_tool_rejects_missing_paths() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let err = expect_tool_error(
        call_explain_module(&workspace, json!({ "path": "missing.rs" })).await,
        "missing path should be rejected",
    );

    assert!(
        err.to_string()
            .contains("ontograph.explain_module path does not exist")
    );
}

#[tokio::test]
async fn explain_module_tool_rejects_directories() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");

    let err = expect_tool_error(
        call_explain_module(&workspace, json!({ "path": "src" })).await,
        "directory path should be rejected",
    );

    assert!(
        err.to_string()
            .contains("ontograph.explain_module path must point to a file")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn explain_module_tool_rejects_symlink_traversal() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(dir.path().join("outside.rs"), "pub struct Outside;\n").expect("write source");
    std::os::unix::fs::symlink(dir.path().join("outside.rs"), workspace.join("linked.rs"))
        .expect("create symlink");

    let err = expect_tool_error(
        call_explain_module(&workspace, json!({ "path": "linked.rs" })).await,
        "symlink path should be rejected",
    );

    assert!(
        err.to_string()
            .contains("ontograph.explain_module path must not traverse symlinks")
    );
}

#[tokio::test]
async fn explain_module_tool_rejects_oversized_files() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("large.rs"), vec![b'a'; MAX_FILE_BYTES + 1])
        .expect("write source");

    let err = expect_tool_error(
        call_explain_module(&workspace, json!({ "path": "large.rs" })).await,
        "oversized file should be rejected",
    );

    assert!(
        err.to_string()
            .contains("ontograph.explain_module path is too large")
    );
}

#[tokio::test]
async fn explain_module_tool_clears_workspace_context_when_turn_has_no_environment() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("sample.rs"), "pub struct Demo;\n").expect("write source");

    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    let turn_input_contributor = &registry.turn_input_contributors()[0];
    turn_input_contributor
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
            &ExtensionData::new("turn-1"),
        )
        .await;
    turn_input_contributor
        .contribute(
            TurnInputContext {
                turn_id: "turn-2".to_string(),
                user_input: Vec::new(),
                environments: Vec::new(),
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn-2"),
        )
        .await;

    let tool = registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .find(|tool| {
            tool.tool_name() == ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME)
        })
        .expect("ontograph explain module tool");
    let err = expect_tool_error(
        tool.handle(ToolCall {
            turn_id: "turn-2".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: ToolPayload::Function {
                arguments: json!({ "path": "sample.rs" }).to_string(),
            },
        })
        .await,
        "tool should reject stale workspace context",
    );

    assert!(
        err.to_string()
            .contains("workspace context is unavailable for this turn")
    );
}

async fn call_explain_module(
    cwd: &Path,
    arguments: serde_json::Value,
) -> Result<Box<dyn ontocode_extension_api::ToolOutput>, ontocode_extension_api::FunctionCallError>
{
    let tool = install_and_find_tool(cwd).await;
    tool.handle(ToolCall {
        turn_id: "turn-1".to_string(),
        call_id: "call-1".to_string(),
        tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
        model: "gpt-test".to_string(),
        truncation_policy: TruncationPolicy::Bytes(1024),
        conversation_history: ConversationHistory::default(),
        turn_item_emitter: Arc::new(NoopTurnItemEmitter),
        payload: ToolPayload::Function {
            arguments: arguments.to_string(),
        },
    })
    .await
}

fn expect_tool_error(
    result: Result<
        Box<dyn ontocode_extension_api::ToolOutput>,
        ontocode_extension_api::FunctionCallError,
    >,
    message: &str,
) -> ontocode_extension_api::FunctionCallError {
    match result {
        Ok(_) => panic!("{message}"),
        Err(err) => err,
    }
}

async fn discover_tool_result(workspace: &Path, arguments: serde_json::Value) -> DiscoverResult {
    serde_json::from_value::<DiscoverResult>(
        call_tool_value(workspace, DISCOVER_TOOL_NAME, arguments).await,
    )
    .expect("deserialize discover result")
}

async fn discover_tool_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    call_tool_error(workspace, DISCOVER_TOOL_NAME, arguments).await
}

async fn call_tool_value(
    workspace: &Path,
    tool_name: &str,
    arguments: serde_json::Value,
) -> serde_json::Value {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, tool_name)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, tool_name),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("tool should succeed");
    output
        .post_tool_use_response("call-1", &payload)
        .expect("json response")
}
