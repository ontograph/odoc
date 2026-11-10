async fn call_tool_error(
    workspace: &Path,
    tool_name: &str,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    expect_tool_error(
        install_and_find_named_tool(workspace, tool_name)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, tool_name),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload,
            })
            .await,
        "tool should fail closed",
    )
}

async fn install_and_find_tool(
    cwd: &Path,
) -> Arc<dyn ontocode_extension_api::ToolExecutor<ontocode_extension_api::ToolCall>> {
    install_and_find_named_tool(cwd, EXPLAIN_MODULE_TOOL_NAME).await
}

async fn install_and_find_named_tool(
    cwd: &Path,
    tool_name: &str,
) -> Arc<dyn ontocode_extension_api::ToolExecutor<ontocode_extension_api::ToolCall>> {
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
                    cwd: cwd.to_path_buf(),
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    registry
        .tool_contributors()
        .iter()
        .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
        .find(|tool| tool.tool_name() == ToolName::namespaced(ONTOGRAPH_NAMESPACE, tool_name))
        .expect("ontograph tool")
}

fn index_workspace_for_tests(workspace: &Path) {
    let target_head = test_target_head(workspace);
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.to_path_buf(),
        artifact_root: workspace_artifact_root(workspace),
        generation_id: target_head.clone(),
        target_head,
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");
}
