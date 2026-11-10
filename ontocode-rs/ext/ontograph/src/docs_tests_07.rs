#[tokio::test]
async fn impact_route_tool_fails_closed_when_handler_is_not_defined_in_route_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/shared.rs"),
        "pub async fn shared_handler() {}\n",
    )
    .expect("write shared source");
    std::fs::write(
        workspace.join("src/router.rs"),
        r#"use axum::routing::get;
use crate::shared::shared_handler;

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/shared", get(shared_handler))
}
"#,
    )
    .expect("write router source");
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
                        "action": "route",
                        "route": "/shared"
                    })
                    .to_string(),
                },
            })
            .await,
        "impact route should fail closed when the handler is not defined in the route file",
    );

    assert!(
        err.to_string()
            .contains("matched route handler is not present in source-index.json"),
        "unexpected error: {err}"
    );
}
