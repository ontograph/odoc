use super::*;
use crate::function_tool::FunctionCallError;
use crate::repository_policy::OperationClass;
use crate::repository_policy::PendingRepositoryPolicyState;
use crate::repository_policy::RepositoryPolicyToolEvidence;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use crate::tools::registry::ToolRegistry;
use ontocode_extension_api::ExtensionData;
use ontocode_extension_api::TurnItemContributor;
use ontocode_protocol::ThreadId;
use ontocode_protocol::items::AgentMessageContent;
use ontocode_protocol::protocol::SessionSource;
use ontocode_protocol::protocol::SkillScope;
use ontocode_protocol::protocol::SubAgentSource;
use ontocode_utils_absolute_path::AbsolutePathBuf;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

struct PolicyCountingHandler {
    tool_name: ontocode_tools::ToolName,
    operation: OperationClass,
    executions: Arc<AtomicUsize>,
}

impl ToolExecutor<ToolInvocation> for PolicyCountingHandler {
    fn tool_name(&self) -> ontocode_tools::ToolName {
        self.tool_name.clone()
    }

    fn spec(&self) -> ontocode_tools::ToolSpec {
        ontocode_tools::ToolSpec::Function(ontocode_tools::ResponsesApiTool {
            name: self.tool_name.name.clone(),
            description: "Count policy-admitted executions.".to_string(),
            strict: false,
            defer_loading: None,
            parameters: ontocode_tools::JsonSchema::default(),
            output_schema: None,
        })
    }

    fn handle<'life0, 'async_trait>(
        &'life0 self,
        _invocation: ToolInvocation,
    ) -> futures::future::BoxFuture<
        'async_trait,
        Result<Box<dyn crate::tools::context::ToolOutput>, FunctionCallError>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            self.executions.fetch_add(1, Ordering::AcqRel);
            Ok(
                Box::new(crate::tools::context::FunctionToolOutput::from_text(
                    "ok".to_string(),
                    Some(true),
                )) as Box<dyn crate::tools::context::ToolOutput>,
            )
        })
    }
}

impl CoreToolRuntime for PolicyCountingHandler {
    fn repository_policy_evidence(
        &self,
        _invocation: &ToolInvocation,
    ) -> RepositoryPolicyToolEvidence {
        RepositoryPolicyToolEvidence::opaque_operation(self.operation)
    }
}

fn policy_test_invocation(
    session: &Arc<Session>,
    turn: &Arc<TurnContext>,
    tool_name: &str,
) -> ToolInvocation {
    ToolInvocation {
        session: Arc::clone(session),
        turn: Arc::clone(turn),
        cancellation_token: CancellationToken::new(),
        tracker: Arc::new(tokio::sync::Mutex::new(
            crate::turn_diff_tracker::TurnDiffTracker::new(),
        )),
        call_id: format!("policy-{tool_name}-{}", turn.sub_id),
        tool_name: ontocode_tools::ToolName::plain(tool_name),
        source: crate::tools::context::ToolCallSource::Direct,
        payload: ToolPayload::Function {
            arguments: "{}".to_string(),
        },
    }
}

fn write_turn_policy_fixture() -> anyhow::Result<tempfile::TempDir> {
    let root = tempfile::tempdir_in("/dev/shm")?;
    let policy_root = root.path().join(".ontocode/policies");
    std::fs::create_dir_all(&policy_root)?;
    std::fs::write(
        policy_root.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 2,
            "policies": [
                {
                    "id": "shell-operation",
                    "path": "shell-operation.md",
                    "priority": 10,
                    "max_bytes": 1024,
                    "class": "mandatory",
                    "predicates": {"operations": ["shell"]}
                },
                {
                    "id": "edit-operation",
                    "path": "edit-operation.md",
                    "priority": 20,
                    "max_bytes": 1024,
                    "class": "mandatory",
                    "predicates": {"operations": ["edit"]}
                }
            ]
        }))?,
    )?;
    for operation in ["shell", "edit"] {
        std::fs::write(
            policy_root.join(format!("{operation}-operation.md")),
            format!("# {operation} operation\n\nReview before execution.\n"),
        )?;
    }
    Ok(root)
}

#[tokio::test]
async fn repository_policy_committed_identity_survives_next_user_turn() -> anyhow::Result<()> {
    let root = write_turn_policy_fixture()?;
    let repository_root = AbsolutePathBuf::try_from(root.path().to_path_buf())?;
    let (session, first, _rx_event) =
        crate::session::tests::make_session_and_context_with_auth_and_config_and_rx(
            ontocode_login::CodexAuth::from_api_key("Test API Key"),
            Vec::new(),
            |config| config.cwd = repository_root,
        )
        .await;
    let executions = Arc::new(AtomicUsize::new(0));
    let shell = ToolRegistry::with_handler_for_test(Arc::new(PolicyCountingHandler {
        tool_name: ontocode_tools::ToolName::plain("exec_command"),
        operation: OperationClass::Shell,
        executions: Arc::clone(&executions),
    }));
    let edit = ToolRegistry::with_handler_for_test(Arc::new(PolicyCountingHandler {
        tool_name: ontocode_tools::ToolName::plain("apply_patch"),
        operation: OperationClass::Edit,
        executions: Arc::clone(&executions),
    }));
    let items = build_repository_policy_items(&session, &first, &[]).await?;
    session.record_conversation_items(&first, &items).await;
    let rejected = shell
        .dispatch_any(policy_test_invocation(&session, &first, "exec_command"))
        .await;
    assert!(
        matches!(rejected, Err(FunctionCallError::RespondToModel(message))
        if message.contains("repository policy loaded before execution"))
    );
    assert_eq!(executions.load(Ordering::Acquire), 0);
    commit_pending_repository_policy_state(&session, &first).await?;
    shell
        .dispatch_any(policy_test_invocation(&session, &first, "exec_command"))
        .await?;
    assert_eq!(executions.load(Ordering::Acquire), 1);

    let second = session.new_default_turn().await;
    assert!(!Arc::ptr_eq(&first, &second));
    let items = build_repository_policy_items(&session, &second, &[]).await?;
    session.record_conversation_items(&second, &items).await;
    replay_repository_policy_state(&session, &second).await?;
    shell
        .dispatch_any(policy_test_invocation(&session, &second, "exec_command"))
        .await?;
    assert_eq!(executions.load(Ordering::Acquire), 2);
    assert!(
        second
            .extension_data
            .get::<PendingRepositoryPolicyState>()
            .is_none()
    );

    let rejected = edit
        .dispatch_any(policy_test_invocation(&session, &second, "apply_patch"))
        .await;
    assert!(
        matches!(rejected, Err(FunctionCallError::RespondToModel(message))
        if message.contains("repository policy loaded before execution"))
    );
    assert_eq!(executions.load(Ordering::Acquire), 2);
    commit_pending_repository_policy_state(&session, &second).await?;
    edit.dispatch_any(policy_test_invocation(&session, &second, "apply_patch"))
        .await?;
    assert_eq!(executions.load(Ordering::Acquire), 3);
    shell
        .dispatch_any(policy_test_invocation(&session, &second, "exec_command"))
        .await?;
    assert_eq!(executions.load(Ordering::Acquire), 4);
    assert!(
        second
            .extension_data
            .get::<PendingRepositoryPolicyState>()
            .is_none()
    );
    Ok(())
}

#[tokio::test]
async fn repository_policy_retained_identity_is_ignored_after_policy_item_leaves_history()
-> anyhow::Result<()> {
    let root = write_turn_policy_fixture()?;
    let repository_root = AbsolutePathBuf::try_from(root.path().to_path_buf())?;
    let (session, first, _rx_event) =
        crate::session::tests::make_session_and_context_with_auth_and_config_and_rx(
            ontocode_login::CodexAuth::from_api_key("Test API Key"),
            Vec::new(),
            |config| config.cwd = repository_root,
        )
        .await;
    let executions = Arc::new(AtomicUsize::new(0));
    let shell = ToolRegistry::with_handler_for_test(Arc::new(PolicyCountingHandler {
        tool_name: ontocode_tools::ToolName::plain("exec_command"),
        operation: OperationClass::Shell,
        executions: Arc::clone(&executions),
    }));
    let items = build_repository_policy_items(&session, &first, &[]).await?;
    session.record_conversation_items(&first, &items).await;
    let rejected = shell
        .dispatch_any(policy_test_invocation(&session, &first, "exec_command"))
        .await;
    assert!(
        matches!(rejected, Err(FunctionCallError::RespondToModel(message))
        if message.contains("repository policy loaded before execution"))
    );
    commit_pending_repository_policy_state(&session, &first).await?;
    assert!(!session.clone_history().await.raw_items().is_empty());
    session
        .replace_history(Vec::new(), /*reference_context_item*/ None)
        .await;

    let second = session.new_default_turn().await;
    let items = build_repository_policy_items(&session, &second, &[]).await?;
    session.record_conversation_items(&second, &items).await;
    replay_repository_policy_state(&session, &second).await?;
    let rejected = shell
        .dispatch_any(policy_test_invocation(&session, &second, "exec_command"))
        .await;
    assert!(
        matches!(rejected, Err(FunctionCallError::RespondToModel(message))
        if message.contains("repository policy loaded before execution"))
    );
    assert_eq!(executions.load(Ordering::Acquire), 0);
    assert!(
        second
            .extension_data
            .get::<PendingRepositoryPolicyState>()
            .is_some()
    );
    Ok(())
}

struct RewriteAgentMessageContributor;

#[async_trait::async_trait]
impl TurnItemContributor for RewriteAgentMessageContributor {
    async fn contribute(
        &self,
        _thread_store: &ExtensionData,
        _turn_store: &ExtensionData,
        item: &mut TurnItem,
    ) -> Result<(), String> {
        if let TurnItem::AgentMessage(agent_message) = item {
            agent_message.content = vec![AgentMessageContent::Text {
                text: "plan contributed assistant text".to_string(),
            }];
        }
        Ok(())
    }
}

fn assistant_output_text(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: Some("msg-1".to_string()),
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn image_generation_availability_items(items: Vec<ResponseItem>) -> Vec<ResponseItem> {
    items
        .into_iter()
        .filter(|item| {
            matches!(
                item,
                ResponseItem::Message { content, .. }
                    if content.iter().any(|content| matches!(
                        content,
                        ContentItem::InputText { text }
                            if text.contains("\"image_generation_availability\"")
                    ))
            )
        })
        .collect()
}

#[tokio::test]
async fn explicit_imagegen_injects_one_unavailable_availability_context() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let temp = tempfile::tempdir().expect("create imagegen skill directory");
    let skill_path = temp.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: imagegen\ndescription: Generate raster images\n---\n\n# Imagegen\n",
    )
    .expect("write imagegen skill");
    let skill_path = AbsolutePathBuf::from_absolute_path(skill_path)
        .expect("imagegen skill path should be absolute");
    let mut skills = ontocode_core_skills::SkillLoadOutcome::default();
    skills.skills = vec![ontocode_core_skills::SkillMetadata {
        name: "imagegen".to_string(),
        description: "Generate raster images".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: skill_path,
        scope: SkillScope::System,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }];
    turn_context.turn_skills =
        crate::session::turn_context::TurnSkillsContext::new(Arc::new(skills));
    let session = Arc::new(session);
    let cancellation_token = CancellationToken::new();

    let explicit_input = [TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "$imagegen create a cover".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    let (explicit_items, _) = build_skills_and_plugins(
        &session,
        &turn_context,
        &explicit_input,
        &cancellation_token,
    )
    .await
    .expect("explicit imagegen injection should build");
    let availability_items = image_generation_availability_items(explicit_items);
    assert_eq!(
        availability_items,
        vec![ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: r#"{"image_generation_availability":"unavailable"}"#.to_string(),
            }],
            phase: None,
        }]
    );

    let ordinary_input = [TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "create a cover".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    let (ordinary_items, _) = build_skills_and_plugins(
        &session,
        &turn_context,
        &ordinary_input,
        &cancellation_token,
    )
    .await
    .expect("ordinary injection should build");
    assert_eq!(
        image_generation_availability_items(ordinary_items),
        Vec::new()
    );
}

#[tokio::test]
async fn plan_mode_uses_contributed_turn_item_for_last_agent_message() {
    let (mut session, turn_context) = crate::session::tests::make_session_and_context().await;
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(RewriteAgentMessageContributor));
    session.services.extensions = Arc::new(builder.build());
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let mut state = PlanModeStreamState::new(&turn_context.sub_id);
    let mut last_agent_message = None;
    let item = assistant_output_text("original assistant text");

    let handled = handle_assistant_item_done_in_plan_mode(
        &session,
        &turn_context,
        &turn_store,
        &item,
        &mut state,
        /*previously_active_item*/ None,
        &mut last_agent_message,
    )
    .await;

    assert!(handled);
    assert_eq!(
        last_agent_message.as_deref(),
        Some("plan contributed assistant text")
    );
}

#[tokio::test]
async fn auto_compact_token_status_reaches_limit_at_exact_boundary() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let baseline = auto_compact_token_status(&session, &turn_context, 0).await;
    let limit = baseline.active_context_tokens;
    turn_context.model_info.auto_compact_token_limit = Some(limit);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.active_context_tokens, limit);
    assert_eq!(status.auto_compact_scope_tokens, limit);
    assert_eq!(status.auto_compact_scope_limit, limit);
    assert!(status.token_limit_reached);
}

#[tokio::test]
async fn auto_compact_token_status_caps_model_limit_to_usable_sampling_input() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = Some(128_000);
    turn_context.model_info.max_context_window = Some(128_000);
    turn_context.model_info.effective_context_window_percent = 95;
    turn_context.model_info.auto_compact_token_limit = Some(115_200);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 62_700);
}

#[tokio::test]
async fn auto_compact_token_status_uses_total_scope_config_limit() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = Some(150_000);
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::Total;
    turn_context.config = Arc::new(config);
    turn_context.model_info.auto_compact_token_limit = None;

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 150_000);
}

#[tokio::test]
async fn auto_compact_token_status_prefers_config_over_model_limit() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = Some(150_000);
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::Total;
    turn_context.config = Arc::new(config);
    turn_context.model_info.auto_compact_token_limit = Some(200_000);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 150_000);
}

#[tokio::test]
async fn auto_compact_token_status_caps_total_scope_config_limit_to_usable_input() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = Some(128_000);
    turn_context.model_info.max_context_window = Some(128_000);
    turn_context.model_info.effective_context_window_percent = 95;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = Some(115_200);
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::Total;
    turn_context.config = Arc::new(config);
    turn_context.model_info.auto_compact_token_limit = None;

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 62_700);
}

#[tokio::test]
async fn auto_compact_token_status_uses_model_limit_before_default_fallback() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = Some(1_000_000);
    turn_context.model_info.max_context_window = Some(1_000_000);
    turn_context.model_info.auto_compact_token_limit = Some(200_000);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 200_000);
}

#[tokio::test]
async fn auto_compact_token_status_defaults_total_scope_to_350k() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = None;
    turn_context.model_info.max_context_window = None;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = None;
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::Total;
    turn_context.config = Arc::new(config);
    turn_context.model_info.auto_compact_token_limit = None;

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(
        status.auto_compact_scope_limit,
        DEFAULT_AUTO_COMPACT_TOKEN_LIMIT
    );
}

#[tokio::test]
async fn auto_compact_token_status_defaults_body_scope_to_350k() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = None;
    turn_context.model_info.max_context_window = None;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = None;
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::BodyAfterPrefix;
    turn_context.config = Arc::new(config);
    turn_context.model_info.auto_compact_token_limit = None;

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(
        status.auto_compact_scope_limit,
        DEFAULT_AUTO_COMPACT_TOKEN_LIMIT
    );
}

#[tokio::test]
async fn cliproxyapi_gpt_5_6_sol_compacts_before_provider_boundary() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let mut config = (*turn_context.config).clone();
    config.model_provider_id = "cliproxyapi".to_string();
    turn_context.config = Arc::new(config);
    turn_context.model_info.slug = "gpt-5.6-sol".to_string();
    turn_context.model_info.context_window = Some(372_000);
    turn_context.model_info.max_context_window = Some(372_000);
    turn_context.model_info.effective_context_window_percent = 95;
    turn_context.model_info.auto_compact_token_limit = Some(334_800);

    let status = auto_compact_token_status(&session, &turn_context, 225_000).await;

    assert_eq!(status.auto_compact_scope_limit, 225_000);
    assert!(status.token_limit_reached);
}

#[tokio::test]
async fn emergency_retry_uses_stricter_provider_safe_budget() {
    let (_session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let mut config = (*turn_context.config).clone();
    config.model_provider_id = "cliproxyapi".to_string();
    turn_context.config = Arc::new(config);
    turn_context.model_info.slug = "gpt-5.6-sol".to_string();
    turn_context.model_info.context_window = Some(372_000);
    turn_context.model_info.max_context_window = Some(372_000);
    turn_context.model_info.effective_context_window_percent = 95;

    assert_eq!(
        SamplingAdmission::Initial.input_token_limit(&turn_context),
        Some(300_000)
    );
    assert_eq!(
        SamplingAdmission::RecoveryRetry.input_token_limit(&turn_context),
        Some(250_000)
    );
    assert!(!SamplingAdmission::Initial.rejects(&turn_context, 260_000));
    assert!(SamplingAdmission::RecoveryRetry.rejects(&turn_context, 260_000));
}

#[tokio::test]
async fn auto_compact_token_status_preserves_stricter_model_limit() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = Some(128_000);
    turn_context.model_info.max_context_window = Some(128_000);
    turn_context.model_info.effective_context_window_percent = 95;
    turn_context.model_info.auto_compact_token_limit = Some(60_000);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 60_000);
}

#[tokio::test]
async fn auto_compact_token_status_body_scope_keeps_body_limit_and_caps_full_request() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    turn_context.model_info.context_window = Some(128_000);
    turn_context.model_info.max_context_window = Some(128_000);
    turn_context.model_info.effective_context_window_percent = 95;
    let mut config = (*turn_context.config).clone();
    config.model_auto_compact_token_limit = Some(115_200);
    config.model_auto_compact_token_limit_scope = AutoCompactTokenLimitScope::BodyAfterPrefix;
    turn_context.config = Arc::new(config);

    let status = auto_compact_token_status(&session, &turn_context, 0).await;

    assert_eq!(status.auto_compact_scope_limit, 115_200);
    assert_eq!(status.full_context_window_limit, Some(62_700));
}

#[tokio::test]
async fn coder_agent_compaction_preserves_finalization_reserve() {
    let (session, mut turn_context) = crate::session::tests::make_session_and_context().await;
    let status = AutoCompactTokenStatus {
        active_context_tokens: 90_000,
        auto_compact_scope_tokens: 90_000,
        auto_compact_scope_limit: 100_000,
        full_context_window_limit: None,
        auto_compact_window_ordinal: None,
        auto_compact_window_prefill_tokens: None,
        full_context_window_limit_reached: false,
        token_limit_reached: false,
    };

    assert!(!compaction_token_limit_reached(
        &session,
        &turn_context,
        &status
    ));

    turn_context.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: ThreadId::new(),
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: Some("coder-worker".to_string()),
    });

    assert!(compaction_token_limit_reached(
        &session,
        &turn_context,
        &status
    ));

    turn_context.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: ThreadId::new(),
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: Some("explorer".to_string()),
    });

    assert!(!compaction_token_limit_reached(
        &session,
        &turn_context,
        &status
    ));

    turn_context.session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: ThreadId::new(),
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: Some("cdr-worker".to_string()),
    });

    assert!(compaction_token_limit_reached_for_role(
        &turn_context,
        &status,
        Some("coder-worker")
    ));
    assert!(!compaction_token_limit_reached_for_role(
        &turn_context,
        &status,
        None
    ));
}

#[test]
fn usable_sampling_output_requires_assistant_text_or_actionable_call() {
    let empty_assistant = assistant_output_text("   ");
    let assistant = assistant_output_text("done");
    let reasoning = ResponseItem::Reasoning {
        id: "reasoning-1".to_string(),
        summary: Vec::new(),
        content: None,
        encrypted_content: None,
    };
    let tool_call = ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: "call-1".to_string(),
    };

    assert!(!is_usable_sampling_output(&empty_assistant));
    assert!(is_usable_sampling_output(&assistant));
    assert!(!is_usable_sampling_output(&reasoning));
    assert!(is_usable_sampling_output(&tool_call));
}

#[test]
fn provider_protocol_retry_reuses_original_input() {
    let original = vec![assistant_output_text("original")];

    assert_eq!(
        retry_prompt_input(
            &CodexErr::ProviderProtocol(
                ontocode_protocol::error::ProviderProtocolError::empty_response(
                    "response-1".to_string()
                )
            ),
            &original
        ),
        Some(original)
    );
    assert_eq!(retry_prompt_input(&CodexErr::RequestTimeout, &[]), None);
}

#[test]
fn assistant_prefill_rejection_repairs_input_and_retries_once() {
    let assistant = assistant_output_text("original");
    let original = vec![assistant.clone()];
    let err = CodexErr::InvalidRequest(
        "This model does not support assistant message prefill.\n\
         The conversation must end with a user message."
            .to_string(),
    );
    let mut retry_available = true;

    assert_eq!(
        take_assistant_prefill_retry_input(&err, &original, &mut retry_available),
        Some(vec![
            assistant,
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Continue.".to_string(),
                }],
                phase: None,
            },
        ])
    );
    assert!(!retry_available);
    assert_eq!(
        take_assistant_prefill_retry_input(&err, &original, &mut retry_available),
        None
    );
}

#[test]
fn assistant_prefill_rejection_matches_provider_wrapped_message() {
    let assistant = assistant_output_text("original");
    let original = vec![assistant];
    // Providers wrap the rejection in their own envelope and vary the leading subject,
    // so the repair must key on the stable fragment rather than one exact sentence.
    let err = CodexErr::InvalidRequest(
        r#"{"error":{"type":"invalid_request_error","message":"This model does not support assistant message prefill. The conversation must end with a user message."}}"#
            .to_string(),
    );
    let mut retry_available = true;

    let retry_input = take_assistant_prefill_retry_input(&err, &original, &mut retry_available)
        .expect("wrapped rejection should be repaired");

    assert_eq!(
        retry_input.last(),
        Some(&ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "Continue.".to_string(),
            }],
            phase: None,
        })
    );
    assert!(!retry_available);
}

#[test]
fn other_invalid_requests_do_not_consume_assistant_prefill_retry() {
    let mut retry_available = true;

    assert_eq!(
        take_assistant_prefill_retry_input(
            &CodexErr::InvalidRequest("different invalid request".to_string()),
            &[],
            &mut retry_available,
        ),
        None
    );
    assert!(retry_available);
}

#[test]
fn review_reports_require_ontoindex_only_for_later_code_claims() {
    let review = TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "Review issue report and challenge its recommendations.".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    };
    let summary = TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "Summarize the supplied prose.".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    };

    assert!(turn_input_requires_ontoindex_for_code_claims(&review));
    assert!(!turn_input_requires_ontoindex_for_code_claims(&summary));
    assert!(turn_input_may_produce_review_code_claims(
        &TurnInput::UserInput {
            content: vec![UserInput::Text {
                text: "Analyze these session records and recommend ownership fixes.".to_string(),
                text_elements: Vec::new(),
            }],
            client_id: None,
            origin: None,
        }
    ));
    assert!(instructions_require_ontoindex_for_reviews(
        "OntoIndex evidence is mandatory for implementation reviews."
    ));
}
