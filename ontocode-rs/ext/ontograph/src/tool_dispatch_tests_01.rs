
fn index_workspace_generation_for_tests(workspace: &Path, generation_id: &str) {
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.to_path_buf(),
        artifact_root: workspace_artifact_root(workspace),
        generation_id: generation_id.to_string(),
        target_head: generation_id.to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace generation");
}

fn write_drift_workspace_source(workspace: &Path, handler: &str) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::write(
        workspace.join("src/lib.rs"),
        format!(
            r#"
use axum::routing::get;

fn build_router() -> axum::Router {{
    axum::Router::new().route("/api/alpha", get({handler}))
}}

async fn {handler}() {{}}
"#
        ),
    )
    .expect("write source");
}

fn write_graph_diff_workspace_source(
    workspace: &Path,
    include_gamma: bool,
    include_missing_call: bool,
) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    let (second_name, second_function, route_path) = if include_gamma {
        ("gamma", "fn gamma() {\n}\n", "/api/gamma")
    } else {
        ("beta", "fn beta() {\n}\n", "/api/beta")
    };
    let missing_call = if include_missing_call {
        "    missing_target();\n"
    } else {
        "\n"
    };
    std::fs::write(
        workspace.join("src/lib.rs"),
        format!(
            "use axum::routing::get;\n\nfn alpha() {{\n    root();\n    {second_name}();\n{missing_call}}}\n\nfn root() {{\n}}\n{second_function}\nfn build_router() -> axum::Router {{\n    axum::Router::new().route(\"{route_path}\", get({second_name}))\n}}\n"
        ),
    )
    .expect("write graph diff source");
}

fn write_graph_diff_truncation_source(workspace: &Path, function_count: usize) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    let mut source = "use axum::routing::get;\n".to_string();
    for index in 0..function_count {
        source.push_str(&format!("fn generated_{index}() {{\n}}\n"));
    }
    source.push_str("fn generated_dispatch() {\n");
    for index in 0..function_count {
        source.push_str(&format!("    generated_{index}();\n"));
    }
    source.push_str("}\nfn build_router() -> axum::Router {\n    axum::Router::new()\n");
    for index in 0..function_count {
        source.push_str(&format!(
            "        .route(\"/api/generated-{index}\", get(generated_{index}))\n"
        ));
    }
    source.push_str("}\n");
    std::fs::write(workspace.join("src/lib.rs"), source).expect("write graph diff source");
}

fn rewrite_generation_manifest_for_tests(
    workspace: &Path,
    generation_id: &str,
    rewrite: impl FnOnce(&mut serde_json::Value),
) {
    let root = workspace_artifact_root(workspace);
    let manifest_path = root
        .generation_manifest_path(generation_id)
        .expect("generation manifest path");
    let mut manifest = serde_json::from_str::<serde_json::Value>(
        &std::fs::read_to_string(&manifest_path).unwrap(),
    )
    .expect("manifest json");
    rewrite(&mut manifest);
    std::fs::write(
        manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
    )
    .expect("rewrite manifest");
}

async fn docs_context_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    expect_tool_error(
        install_and_find_named_tool(workspace, DOCS_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DOCS_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload,
            })
            .await,
        "docs.context should fail closed",
    )
}

async fn docs_tool_result(workspace: &Path, arguments: serde_json::Value) -> DocsResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, DOCS_TOOL_NAME)
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
        .expect("ontograph.docs should succeed");
    serde_json::from_value::<DocsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize docs result")
}

async fn audit_session_start_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditSessionStartResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit session_start should succeed");
    serde_json::from_value::<AuditSessionStartResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_start result")
}

async fn manage_session_result(workspace: &Path) -> ManageResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "session" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, MANAGE_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, MANAGE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("manage session should succeed");
    serde_json::from_value::<ManageResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize manage session result")
}

async fn audit_session_verify_result(workspace: &Path) -> AuditSessionVerifyResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "session_verify" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit session_verify should succeed");
    serde_json::from_value::<AuditSessionVerifyResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_verify result")
}

async fn audit_session_dedupe_result(workspace: &Path) -> AuditSessionDedupeResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "session_dedupe" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit session_dedupe should succeed");
    serde_json::from_value::<AuditSessionDedupeResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_dedupe result")
}

async fn audit_verify_diff_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditVerifyDiffResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit verify_diff should succeed");
    serde_json::from_value::<AuditVerifyDiffResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit verify_diff result")
}

async fn audit_test_gap_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditTestGapResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit test_gap should succeed");
    serde_json::from_value::<AuditTestGapResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit test_gap result")
}

async fn audit_test_suggestions_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditTestSuggestionsResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit test_suggestions should succeed");
    serde_json::from_value::<AuditTestSuggestionsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit test_suggestions result")
}

async fn audit_abi_result(workspace: &Path, arguments: serde_json::Value) -> AuditAbiResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit abi should succeed");
    serde_json::from_value::<AuditAbiResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit abi result")
}

async fn audit_abi_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    match install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: ToolPayload::Function {
                arguments: arguments.to_string(),
            },
        })
        .await
    {
        Ok(_) => panic!("audit abi should fail closed"),
        Err(err) => err,
    }
}

async fn audit_path_verify_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditPathVerifyResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit path_verify should succeed");
    serde_json::from_value::<AuditPathVerifyResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit path_verify result")
}

async fn audit_path_verify_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    match install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: ToolPayload::Function {
                arguments: arguments.to_string(),
            },
        })
        .await
    {
        Ok(_) => panic!("audit path_verify should fail closed"),
        Err(err) => err,
    }
}

fn write_path_verify_fixture(workspace: &Path, content: &str) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), content).expect("write path verify fixture");
}

async fn audit_pressure_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditPressureResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit pressure should succeed");
    serde_json::from_value::<AuditPressureResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit pressure result")
}

fn write_pressure_fixture(workspace: &Path, content: &str) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), content).expect("write pressure fixture");
}

async fn audit_concurrency_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditConcurrencyResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit concurrency should succeed");
    serde_json::from_value::<AuditConcurrencyResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit concurrency result")
}

fn write_concurrency_fixture(workspace: &Path, content: &str) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), content).expect("write concurrency fixture");
}

async fn audit_simulate_fault_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditFaultSimulationResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit simulate_fault should succeed");
    serde_json::from_value::<AuditFaultSimulationResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit simulate_fault result")
}

async fn audit_simulate_fault_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    match install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: ToolPayload::Function {
                arguments: arguments.to_string(),
            },
        })
        .await
    {
        Ok(_) => panic!("audit simulate_fault should fail closed"),
        Err(err) => err,
    }
}

fn write_simulate_fault_fixture(workspace: &Path, content: &str) {
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), content).expect("write simulate_fault fixture");
}

async fn audit_fsm_result(workspace: &Path, arguments: serde_json::Value) -> AuditFsmResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit extract_fsm should succeed");
    serde_json::from_value::<AuditFsmResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit extract_fsm result")
}

async fn audit_cycles_result(workspace: &Path, arguments: serde_json::Value) -> AuditCyclesResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit cycles should succeed");
    serde_json::from_value::<AuditCyclesResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit cycles result")
}

async fn audit_coupling_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditCouplingResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit coupling should succeed");
    serde_json::from_value::<AuditCouplingResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit coupling result")
}

async fn audit_report_result(workspace: &Path) -> AuditReportResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "report" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit report should succeed");
    serde_json::from_value::<AuditReportResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit report result")
}

async fn audit_trace_boundary_result(workspace: &Path) -> TraceBoundaryResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "trace_boundary" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit trace_boundary should succeed");
    serde_json::from_value::<TraceBoundaryResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit trace_boundary result")
}
