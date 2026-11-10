use super::HandleOutputCtx;
use super::TurnItemContributorPolicy;
use super::completed_item_defers_mailbox_delivery_to_next_turn;
use super::finalize_non_tool_response_item;
use super::handle_non_tool_response_item;
use super::handle_output_item_done;
use super::image_generation_artifact_path;
use super::last_assistant_message_from_item;
use super::persist_image_generation_item;
use super::record_completed_response_item;
use super::response_item_may_include_external_context;
use super::save_image_generation_result;
use crate::generated_image_persistence::GeneratedImageWriteOutcome;
use crate::session::tests::attach_thread_persistence;
use crate::session::tests::make_session_and_context;
use crate::session::tests::make_session_and_context_with_rx;
use crate::state::TurnState;
use crate::tools::ToolRouter;
use crate::tools::parallel::ToolCallRuntime;
use crate::turn_diff_tracker::TurnDiffTracker;
use ontocode_extension_api::ExtensionData;
use ontocode_extension_api::TurnItemContributor;
use ontocode_protocol::error::CodexErr;
use ontocode_protocol::items::AgentMessageContent;
use ontocode_protocol::items::ImageGenerationItem;
use ontocode_protocol::items::TurnItem;
use ontocode_protocol::memory_citation::MEMORY_CITATION_USAGE_SCHEMA_VERSION;
use ontocode_protocol::memory_citation::MemoryCitation;
use ontocode_protocol::memory_citation::MemoryCitationEntry;
use ontocode_protocol::memory_citation::MemoryCitationRegressionStatus;
use ontocode_protocol::memory_citation::MemoryCitationTaskOutcome;
use ontocode_protocol::memory_citation::MemoryCitationUsageRecord;
use ontocode_protocol::models::ContentItem;
use ontocode_protocol::models::FunctionCallOutputPayload;
use ontocode_protocol::models::LocalShellAction;
use ontocode_protocol::models::LocalShellExecAction;
use ontocode_protocol::models::LocalShellStatus;
use ontocode_protocol::models::MessagePhase;
use ontocode_protocol::models::ResponseItem;
use ontocode_protocol::protocol::EventMsg;
use ontocode_protocol::protocol::WarningEvent;
use ontocode_protocol::read_evidence::ArtifactReceipt;
use ontocode_protocol::read_evidence::ArtifactReceiptOperation;
use ontocode_protocol::read_evidence::ArtifactReceiptOutcome;
use ontocode_protocol::read_evidence::ArtifactReceiptProducer;
use ontocode_protocol::read_evidence::FileReadEvidence;
use ontocode_protocol::read_evidence::OntoIndexEvidenceLevel;
use ontocode_protocol::read_evidence::OntoIndexEvidenceOutcome;
use ontocode_utils_absolute_path::test_support::PathExt;
use pretty_assertions::assert_eq;
use sha2::Digest;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

fn assistant_output_text(text: &str) -> ResponseItem {
    assistant_output_text_with_phase(text, /*phase*/ None)
}

fn assistant_output_text_with_phase(text: &str, phase: Option<MessagePhase>) -> ResponseItem {
    ResponseItem::Message {
        id: Some("msg-1".to_string()),
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase,
    }
}

#[test]
fn external_context_pollution_items_include_web_search_and_tool_search() {
    let polluting_items = [
        ResponseItem::WebSearchCall {
            id: None,
            status: Some("completed".to_string()),
            action: None,
        },
        ResponseItem::ToolSearchCall {
            id: None,
            call_id: Some("search-1".to_string()),
            status: None,
            execution: "client".to_string(),
            arguments: serde_json::json!({"query": "calendar"}),
        },
        ResponseItem::ToolSearchOutput {
            call_id: Some("search-1".to_string()),
            status: "completed".to_string(),
            execution: "client".to_string(),
            tools: Vec::new(),
        },
    ];

    assert!(
        polluting_items
            .iter()
            .all(response_item_may_include_external_context)
    );
}

#[test]
fn external_context_pollution_items_exclude_local_tool_calls() {
    let non_polluting_items = [
        ResponseItem::LocalShellCall {
            id: None,
            call_id: Some("shell-1".to_string()),
            status: LocalShellStatus::Completed,
            action: LocalShellAction::Exec(LocalShellExecAction {
                command: vec!["cat".to_string(), "README.md".to_string()],
                timeout_ms: None,
                working_directory: None,
                env: None,
                user: None,
            }),
        },
        ResponseItem::FunctionCall {
            id: None,
            name: "shell".to_string(),
            namespace: None,
            arguments: "{}".to_string(),
            call_id: "call-1".to_string(),
        },
        ResponseItem::FunctionCallOutput {
            call_id: "call-1".to_string(),
            output: FunctionCallOutputPayload::from_text("ok".to_string()),
        },
        ResponseItem::CustomToolCall {
            id: None,
            status: None,
            call_id: "custom-1".to_string(),
            name: "apply_patch".to_string(),
            input: "*** Begin Patch\n*** End Patch\n".to_string(),
        },
        ResponseItem::CustomToolCallOutput {
            call_id: "custom-1".to_string(),
            name: Some("apply_patch".to_string()),
            output: FunctionCallOutputPayload::from_text("ok".to_string()),
        },
        assistant_output_text("plain assistant text"),
    ];

    assert!(
        !non_polluting_items
            .iter()
            .any(response_item_may_include_external_context)
    );
}

#[tokio::test]
async fn handle_non_tool_response_item_strips_citations_from_assistant_message() {
    let (session, turn_context) = make_session_and_context().await;
    let item = assistant_output_text(
        "hello<oai-mem-citation><citation_entries>\nMEMORY.md:1-2|note=[x]\n</citation_entries>\n<rollout_ids>\n019cc2ea-1dff-7902-8d40-c8f6e5d83cc4\n</rollout_ids></oai-mem-citation> world",
    );

    let turn_item = handle_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Skip,
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    let TurnItem::AgentMessage(agent_message) = turn_item else {
        panic!("expected agent message");
    };
    let text = agent_message
        .content
        .iter()
        .map(|entry| match entry {
            ontocode_protocol::items::AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect::<String>();
    assert_eq!(text, "hello world");
    let memory_citation = agent_message
        .memory_citation
        .expect("memory citation should be parsed");
    assert_eq!(memory_citation.entries.len(), 1);
    assert_eq!(memory_citation.entries[0].path, "MEMORY.md");
    assert_eq!(
        memory_citation.rollout_ids,
        vec!["019cc2ea-1dff-7902-8d40-c8f6e5d83cc4".to_string()]
    );
}

struct TestTurnItemContributor;

#[derive(Debug)]
struct TurnItemContributorRan;

#[async_trait::async_trait]
impl TurnItemContributor for TestTurnItemContributor {
    async fn contribute(
        &self,
        _thread_store: &ExtensionData,
        turn_store: &ExtensionData,
        item: &mut TurnItem,
    ) -> Result<(), String> {
        turn_store.insert(TurnItemContributorRan);
        if let TurnItem::AgentMessage(agent_message) = item {
            agent_message.memory_citation = Some(MemoryCitation {
                entries: Vec::new(),
                rollout_ids: Vec::new(),
            });
        }
        Ok(())
    }
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
                text: "contributed assistant text".to_string(),
            }];
        }
        Ok(())
    }
}

#[tokio::test]
async fn handle_non_tool_response_item_runs_turn_item_contributors_only_when_requested() {
    let (mut session, turn_context) = make_session_and_context().await;
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(TestTurnItemContributor));
    session.services.extensions = Arc::new(builder.build());
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let item = assistant_output_text(
        "hello<oai-mem-citation>ignored by memory parser</oai-mem-citation> world",
    );

    let provisional_turn_item = handle_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Skip,
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    assert!(turn_store.get::<TurnItemContributorRan>().is_none());
    let TurnItem::AgentMessage(provisional_agent_message) = provisional_turn_item else {
        panic!("expected agent message");
    };
    assert_eq!(provisional_agent_message.memory_citation, None);

    let turn_item = handle_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Run(&turn_store),
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    assert!(turn_store.get::<TurnItemContributorRan>().is_some());
    let TurnItem::AgentMessage(agent_message) = turn_item else {
        panic!("expected agent message");
    };
    assert!(agent_message.memory_citation.is_some());
    let text = agent_message
        .content
        .iter()
        .map(|entry| match entry {
            ontocode_protocol::items::AgentMessageContent::Text { text } => text.as_str(),
        })
        .collect::<String>();
    assert_eq!(text, "hello world");
}

#[tokio::test]
async fn handle_output_item_done_returns_contributed_last_agent_message() {
    let (mut session, turn_context) = make_session_and_context().await;
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(RewriteAgentMessageContributor));
    session.services.extensions = Arc::new(builder.build());
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let router = Arc::new(ToolRouter::from_turn_context(
        &turn_context,
        crate::tools::router::ToolRouterParams {
            mcp_tools: None,
            deferred_mcp_tools: None,
            discoverable_tools: None,
            extension_tool_executors: Vec::new(),
            dynamic_tools: turn_context.dynamic_tools.as_slice(),
            searched_tool_names: Vec::new(),
            mcp_resources_advertised: true,
        },
    ));
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let tool_runtime = ToolCallRuntime::new(
        router,
        Arc::clone(&session),
        Arc::clone(&turn_context),
        tracker,
    );
    let item = assistant_output_text("original assistant text");
    let mut ctx = HandleOutputCtx {
        sess: session,
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::new(ExtensionData::new(turn_context.sub_id.clone())),
        tool_runtime,
        cancellation_token: CancellationToken::new(),
    };

    let output = handle_output_item_done(&mut ctx, item, /*previously_active_item*/ None)
        .await
        .expect("assistant message should complete");

    assert_eq!(
        output.last_agent_message.as_deref(),
        Some("contributed assistant text")
    );
}

#[tokio::test]
async fn finalized_turn_item_defers_mailbox_for_contributed_visible_text() {
    let (mut session, turn_context) = make_session_and_context().await;
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(RewriteAgentMessageContributor));
    session.services.extensions = Arc::new(builder.build());
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let item = assistant_output_text("<oai-mem-citation>hidden only</oai-mem-citation>");

    let finalized = finalize_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Run(&turn_store),
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    assert_eq!(
        finalized.facts.last_agent_message.as_deref(),
        Some("contributed assistant text")
    );
    assert!(finalized.facts.defers_mailbox_delivery_to_next_turn);
}

#[tokio::test]
async fn finalized_turn_item_keeps_mailbox_open_for_commentary_text() {
    let (mut session, turn_context) = make_session_and_context().await;
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.turn_item_contributor(Arc::new(RewriteAgentMessageContributor));
    session.services.extensions = Arc::new(builder.build());
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let item = assistant_output_text_with_phase("still working", Some(MessagePhase::Commentary));

    let finalized = finalize_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Run(&turn_store),
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    assert_eq!(
        finalized.facts.last_agent_message.as_deref(),
        Some("contributed assistant text")
    );
    assert!(!finalized.facts.defers_mailbox_delivery_to_next_turn);
}

#[tokio::test]
async fn finalized_turn_item_attaches_bounded_memory_citation_usage_record() {
    let (session, turn_context) = make_session_and_context().await;
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());
    let item = assistant_output_text(
        "answer<oai-mem-citation><citation_entries>\nMEMORY.md:10-12|note=[bounded]\n</citation_entries>\n<rollout_ids>\nrollout-1\n</rollout_ids></oai-mem-citation>",
    );

    let finalized = finalize_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Run(&turn_store),
        &item,
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    let record = finalized
        .facts
        .memory_citation_usage_record
        .expect("citation usage record should be attached");
    assert_eq!(record.schema_version, MEMORY_CITATION_USAGE_SCHEMA_VERSION);
    assert_eq!(record.citation.rollout_ids, vec!["rollout-1".to_string()]);
    assert_eq!(record.task_outcome, MemoryCitationTaskOutcome::Unknown);
    assert_eq!(
        record.regression_status,
        MemoryCitationRegressionStatus::NotEvaluated
    );
    let encoded = serde_json::to_value(record).expect("record should serialize");
    assert!(
        !encoded
            .as_object()
            .expect("record object")
            .contains_key("response")
    );
    assert!(
        !encoded
            .as_object()
            .expect("record object")
            .contains_key("diagnostics")
    );
}

#[tokio::test]
async fn finalized_turn_item_omits_memory_citation_usage_record_without_citation() {
    let (session, turn_context) = make_session_and_context().await;
    let turn_store = ExtensionData::new(turn_context.sub_id.clone());

    let finalized = finalize_non_tool_response_item(
        &session,
        &turn_context,
        TurnItemContributorPolicy::Run(&turn_store),
        &assistant_output_text("plain answer"),
        /*plan_mode*/ false,
    )
    .await
    .expect("assistant message should parse");

    assert_eq!(finalized.facts.memory_citation_usage_record, None);
}

fn memory_citation_usage_record() -> MemoryCitationUsageRecord {
    MemoryCitationUsageRecord {
        schema_version: MEMORY_CITATION_USAGE_SCHEMA_VERSION,
        citation: MemoryCitation {
            entries: vec![MemoryCitationEntry {
                path: "MEMORY.md".to_string(),
                line_start: 10,
                line_end: 12,
                note: "bounded evidence".to_string(),
            }],
            rollout_ids: vec!["rollout-1".to_string()],
        },
        task_outcome: MemoryCitationTaskOutcome::Unknown,
        regression_status: MemoryCitationRegressionStatus::NotEvaluated,
    }
}

#[test]
fn turn_state_retains_only_the_first_memory_citation_record() {
    let first = memory_citation_usage_record();
    let mut second = memory_citation_usage_record();
    second.citation.rollout_ids = vec!["rollout-2".to_string()];
    let mut turn_state = TurnState::default();

    turn_state.retain_memory_citation_usage_record(first.clone());
    turn_state.retain_memory_citation_usage_record(second);

    assert!(turn_state.has_memory_citation);
    assert_eq!(turn_state.memory_citation_usage_record, Some(first));
}

#[test]
fn turn_state_terminalizes_memory_citation_once_without_evaluating_regression() {
    for outcome in [
        MemoryCitationTaskOutcome::Completed,
        MemoryCitationTaskOutcome::Cancelled,
        MemoryCitationTaskOutcome::Failed,
    ] {
        let mut turn_state = TurnState::default();
        turn_state.retain_memory_citation_usage_record(memory_citation_usage_record());

        assert!(!turn_state.terminalize_memory_citation(MemoryCitationTaskOutcome::Unknown));
        assert!(turn_state.terminalize_memory_citation(outcome));
        assert!(!turn_state.terminalize_memory_citation(MemoryCitationTaskOutcome::Completed));
        let record = turn_state
            .memory_citation_usage_record
            .as_ref()
            .expect("citation record should remain");
        assert_eq!(record.task_outcome, outcome);
        assert_eq!(
            record.regression_status,
            MemoryCitationRegressionStatus::NotEvaluated
        );
    }
}

#[test]
fn turn_state_does_not_terminalize_without_a_memory_citation() {
    let mut turn_state = TurnState::default();

    assert!(!turn_state.terminalize_memory_citation(MemoryCitationTaskOutcome::Completed));
    assert!(!turn_state.has_memory_citation);
    assert_eq!(turn_state.memory_citation_usage_record, None);
}

#[test]
fn last_assistant_message_from_item_strips_citations_and_plan_blocks() {
    let item = assistant_output_text(
        "before<oai-mem-citation>doc1</oai-mem-citation>\n<proposed_plan>\n- x\n</proposed_plan>\nafter",
    );

    let message = last_assistant_message_from_item(&item, /*plan_mode*/ true)
        .expect("assistant text should remain after stripping");

    assert_eq!(message, "before\nafter");
}

#[test]
fn last_assistant_message_from_item_returns_none_for_citation_only_message() {
    let item = assistant_output_text("<oai-mem-citation>doc1</oai-mem-citation>");

    assert_eq!(
        last_assistant_message_from_item(&item, /*plan_mode*/ false),
        None
    );
}

#[test]
fn last_assistant_message_from_item_returns_none_for_plan_only_hidden_message() {
    let item = assistant_output_text("<proposed_plan>\n- x\n</proposed_plan>");

    assert_eq!(
        last_assistant_message_from_item(&item, /*plan_mode*/ true),
        None
    );
}

#[test]
fn completed_item_defers_mailbox_delivery_for_unknown_phase_messages() {
    let item = assistant_output_text("final answer");

    assert!(completed_item_defers_mailbox_delivery_to_next_turn(
        &item, /*plan_mode*/ false,
    ));
}

#[test]
fn completed_item_keeps_mailbox_delivery_open_for_commentary_messages() {
    let item = assistant_output_text_with_phase("still working", Some(MessagePhase::Commentary));

    assert!(!completed_item_defers_mailbox_delivery_to_next_turn(
        &item, /*plan_mode*/ false,
    ));
}

#[test]
fn completed_item_defers_mailbox_delivery_for_image_generation_calls() {
    let item = ResponseItem::ImageGenerationCall {
        id: "ig-1".to_string(),
        status: "completed".to_string(),
        revised_prompt: None,
        result: "Zm9v".to_string(),
    };

    assert!(completed_item_defers_mailbox_delivery_to_next_turn(
        &item, /*plan_mode*/ false,
    ));
}

#[tokio::test]
async fn final_answer_verifier_warns_when_test_claim_lacks_evidence() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;

    record_completed_response_item(
        &session,
        &turn_context,
        &assistant_output_text("Tests passed."),
    )
    .await;

    for _ in 0..4 {
        let event = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("event should arrive")
            .expect("event should be readable");
        if matches!(
            event.msg,
            EventMsg::Warning(WarningEvent { message })
                if message.contains("Final answer claims tests")
        ) {
            return;
        }
    }
    panic!("expected final-answer verifier warning");
}

#[tokio::test]
async fn final_answer_verifier_warns_when_review_has_only_ontoindex_health_evidence() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    {
        let mut evidence = turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock");
        evidence.require_ontoindex_for_code_claims();
        evidence.record_ontoindex_attempt(
            OntoIndexEvidenceOutcome::UsedSuccessfully,
            Some(OntoIndexEvidenceLevel::AvailabilityOnly),
        );
    }

    record_completed_response_item(
        &session,
        &turn_context,
        &assistant_output_text("The router implementation should add a completion policy check."),
    )
    .await;

    for _ in 0..4 {
        let event = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("event should arrive")
            .expect("event should be readable");
        if matches!(
            event.msg,
            EventMsg::Warning(WarningEvent { message })
                if message.contains("required OntoIndex evidence")
        ) {
            return;
        }
    }
    panic!("expected required OntoIndex evidence warning");
}

#[test]
fn file_read_evidence_legacy_payload_defaults_artifact_receipts() {
    let evidence: FileReadEvidence = serde_json::from_value(serde_json::json!({
        "paths": {}
    }))
    .expect("legacy evidence should deserialize");

    assert!(evidence.artifact_receipts.is_empty());
}

#[test]
fn artifact_receipts_round_trip_without_image_payload_and_keep_latest_bounded_entries() {
    let mut evidence = FileReadEvidence::default();
    for index in 0..40 {
        let path = ontocode_utils_absolute_path::AbsolutePathBuf::try_from(
            std::path::PathBuf::from(format!("/tmp/image-{index}.png")),
        )
        .expect("absolute artifact path");
        evidence.record_artifact_receipt(
            ArtifactReceiptProducer::ImageGeneration,
            format!("call-{index}"),
            path,
            ArtifactReceiptOperation::Create,
            ArtifactReceiptOutcome::Success,
        );
    }

    let value = serde_json::to_value(&evidence).expect("serialize evidence");
    let serialized = value.to_string();
    let round_trip: FileReadEvidence = serde_json::from_value(value).expect("deserialize evidence");

    assert_eq!(round_trip.artifact_receipts.len(), 32);
    assert_eq!(round_trip.artifact_receipts[0].call_id, "call-8");
    assert_eq!(round_trip.artifact_receipts[31].call_id, "call-39");
    assert!(!serialized.contains("base64"));
    assert!(!serialized.contains("payload"));
    assert!(!serialized.contains("result"));
}

#[tokio::test]
async fn persist_image_generation_item_records_receipt_only_after_successful_save() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    let mut successful = ImageGenerationItem {
        id: "ig-success".to_string(),
        status: "completed".to_string(),
        revised_prompt: None,
        result: "Zm9v".to_string(),
        saved_path: None,
    };

    let saved_path = persist_image_generation_item(&session, &turn_context, &mut successful)
        .await
        .expect("image should be saved");
    let successful_evidence = turn_context
        .file_read_evidence
        .lock()
        .expect("evidence lock")
        .clone();
    assert_eq!(successful.saved_path, Some(saved_path.clone()));
    assert_eq!(successful_evidence.artifact_receipts.len(), 1);
    assert_eq!(successful_evidence.artifact_receipts[0].path, saved_path);

    let mut failed = ImageGenerationItem {
        id: "ig-failed".to_string(),
        status: "completed".to_string(),
        revised_prompt: None,
        result: "not base64".to_string(),
        saved_path: None,
    };
    assert_eq!(
        persist_image_generation_item(&session, &turn_context, &mut failed).await,
        None
    );
    assert_eq!(failed.saved_path, None);
    assert_eq!(
        turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock")
            .artifact_receipts,
        successful_evidence.artifact_receipts
    );
    let _ = std::fs::remove_file(saved_path);
}

#[tokio::test]
async fn persist_image_generation_item_removes_file_when_receipt_cannot_be_persisted() {
    let (session, turn_context) = make_session_and_context().await;
    let mut image = ImageGenerationItem {
        id: "ig-no-persistence".to_string(),
        status: "completed".to_string(),
        revised_prompt: None,
        result: "Zm9v".to_string(),
        saved_path: None,
    };
    let output_path = image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.thread_id.to_string(),
        &image.id,
    );

    assert_eq!(
        persist_image_generation_item(&session, &turn_context, &mut image).await,
        None
    );
    assert_eq!(image.saved_path, None);
    assert!(
        turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock")
            .artifact_receipts
            .is_empty()
    );
    assert!(session.artifact_receipts().await.is_empty());
    assert!(!output_path.exists());
}

#[tokio::test]
async fn persist_image_generation_item_preserves_matching_replay_when_receipt_cannot_be_persisted()
{
    let (session, turn_context) = make_session_and_context().await;
    let mut image = ImageGenerationItem {
        id: "ig-no-persistence-replay".to_string(),
        status: "completed".to_string(),
        revised_prompt: None,
        result: "Zm9v".to_string(),
        saved_path: None,
    };
    let output_path = image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.thread_id.to_string(),
        &image.id,
    );
    std::fs::create_dir_all(output_path.parent().expect("output parent"))
        .expect("create output parent");
    std::fs::write(&output_path, b"foo").expect("seed matching replay");

    assert_eq!(
        persist_image_generation_item(&session, &turn_context, &mut image).await,
        None
    );
    assert_eq!(image.saved_path, None);
    assert_eq!(
        std::fs::read(&output_path).expect("matching replay should remain"),
        b"foo"
    );
}

#[test]
fn final_answer_verifier_requires_matching_generated_image_receipt() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let workspace_path = workspace
        .path()
        .canonicalize()
        .expect("canonical workspace");
    let path = workspace_path.join("generated-image.png").abs();
    std::fs::write(path.as_path(), b"image").expect("write generated image");
    let answer = format!("Generated and saved the image at {}.", path.display());
    assert!(
        super::final_answer_verification_warning(&answer, &FileReadEvidence::default()).is_some()
    );

    let mut evidence = FileReadEvidence::default();
    evidence.record_complete_artifact_receipt(ArtifactReceipt {
        producer: ArtifactReceiptProducer::ImageGeneration,
        call_id: "ig-1".to_string(),
        path,
        source_path: None,
        content_sha256: Some(format!("{:x}", Sha256::digest(b"image"))),
        operation: ArtifactReceiptOperation::Create,
        outcome: ArtifactReceiptOutcome::Success,
    });
    assert_eq!(
        super::final_answer_verification_warning(&answer, &evidence),
        None
    );
}

#[tokio::test]
async fn final_answer_verifier_does_not_warn_for_intermediate_image_claim() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    record_completed_response_item(
        &session,
        &turn_context,
        &assistant_output_text_with_phase(
            "Generated /tmp/unreceipted.png.",
            Some(MessagePhase::Commentary),
        ),
    )
    .await;

    while let Ok(Ok(event)) = timeout(Duration::from_millis(25), rx.recv()).await {
        assert!(!matches!(event.msg, EventMsg::Warning(_)));
    }
}

#[tokio::test]
async fn save_image_generation_result_saves_base64_to_png_in_ontocode_home() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let expected_path =
        image_generation_artifact_path(&ontocode_home, "session-1", "ig_save_base64");
    let _ = std::fs::remove_file(&expected_path);

    let saved = save_image_generation_result(&ontocode_home, "session-1", "ig_save_base64", "Zm9v")
        .await
        .expect("image should be saved");

    assert_eq!(saved.path, expected_path);
    assert_eq!(
        saved.content_sha256,
        "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    );
    assert_eq!(saved.write_outcome, GeneratedImageWriteOutcome::Created);
    assert_eq!(std::fs::read(&saved.path).expect("saved file"), b"foo");
    let _ = std::fs::remove_file(&saved.path);
}

#[tokio::test]
async fn save_image_generation_result_rejects_data_url_payload() {
    let result = "data:image/jpeg;base64,Zm9v";
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();

    let err = save_image_generation_result(&ontocode_home, "session-1", "ig_456", result)
        .await
        .expect_err("data url payload should error");
    assert!(matches!(err, CodexErr::InvalidRequest(_)));
}

#[tokio::test]
async fn save_image_generation_result_rejects_existing_file_without_overwriting() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let existing_path = image_generation_artifact_path(&ontocode_home, "session-1", "ig_overwrite");
    std::fs::create_dir_all(
        existing_path
            .parent()
            .expect("generated image path should have a parent"),
    )
    .expect("create image output dir");
    std::fs::write(&existing_path, b"existing").expect("seed existing image");

    let err = save_image_generation_result(&ontocode_home, "session-1", "ig_overwrite", "Zm9v")
        .await
        .expect_err("existing image should not be overwritten");

    assert!(matches!(err, CodexErr::Io(_)));
    assert_eq!(
        std::fs::read(&existing_path).expect("existing file"),
        b"existing"
    );
}

#[tokio::test]
async fn save_image_generation_result_accepts_existing_file_with_matching_digest() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let existing_path = image_generation_artifact_path(&ontocode_home, "session-1", "ig_replay");
    std::fs::create_dir_all(
        existing_path
            .parent()
            .expect("generated image path should have a parent"),
    )
    .expect("create image output dir");
    std::fs::write(&existing_path, b"foo").expect("seed existing image");

    let saved = save_image_generation_result(&ontocode_home, "session-1", "ig_replay", "Zm9v")
        .await
        .expect("matching existing image should be accepted");

    assert_eq!(saved.path, existing_path);
    assert_eq!(
        saved.content_sha256,
        "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    );
    assert_eq!(
        saved.write_outcome,
        GeneratedImageWriteOutcome::ExistingMatch
    );
    assert_eq!(
        std::fs::read(&existing_path).expect("existing file"),
        b"foo"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn save_image_generation_result_rejects_final_symlink() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let output_path = image_generation_artifact_path(&ontocode_home, "session-1", "ig_symlink");
    std::fs::create_dir_all(output_path.parent().expect("output parent"))
        .expect("create output parent");
    let target_path = ontocode_home.join("existing.png");
    std::fs::write(&target_path, b"existing").expect("seed symlink target");
    std::os::unix::fs::symlink(&target_path, &output_path).expect("create final symlink");

    let err = save_image_generation_result(&ontocode_home, "session-1", "ig_symlink", "Zm9v")
        .await
        .expect_err("final symlink should be rejected");

    assert!(matches!(err, CodexErr::Io(_)));
    assert_eq!(
        std::fs::read(&target_path).expect("symlink target"),
        b"existing"
    );
    assert!(
        std::fs::symlink_metadata(&output_path)
            .expect("final symlink metadata")
            .file_type()
            .is_symlink()
    );
}

#[cfg(unix)]
#[tokio::test]
async fn save_image_generation_result_rejects_symlinked_parent() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let redirected = tempfile::tempdir().expect("create redirected output directory");
    std::os::unix::fs::symlink(redirected.path(), ontocode_home.join("generated_images"))
        .expect("create parent symlink");

    let err = save_image_generation_result(&ontocode_home, "session-1", "ig_parent", "Zm9v")
        .await
        .expect_err("symlinked parent should be rejected");

    assert!(matches!(err, CodexErr::Io(_)));
    assert!(!redirected.path().join("session-1/ig_parent.png").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn save_image_generation_result_accepts_symlinked_ontocode_home() {
    let workspace = tempfile::tempdir().expect("create workspace");
    let real_home = workspace.path().join("real-home");
    std::fs::create_dir(&real_home).expect("create real ontocode home");
    let linked_home = workspace.path().join("linked-home");
    std::os::unix::fs::symlink(&real_home, &linked_home).expect("link ontocode home");
    let linked_home = linked_home.abs();

    let saved = save_image_generation_result(&linked_home, "session-1", "ig_linked_home", "Zm9v")
        .await
        .expect("symlinked ontocode home should be supported");

    assert_eq!(std::fs::read(saved.path).expect("saved file"), b"foo");
}

#[tokio::test]
async fn save_image_generation_result_sanitizes_call_id_for_ontocode_home_output_path() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let expected_path = image_generation_artifact_path(&ontocode_home, "session-1", "../ig/..");
    let _ = std::fs::remove_file(&expected_path);

    let saved = save_image_generation_result(&ontocode_home, "session-1", "../ig/..", "Zm9v")
        .await
        .expect("image should be saved");

    assert_eq!(saved.path, expected_path);
    assert_eq!(std::fs::read(&saved.path).expect("saved file"), b"foo");
    let _ = std::fs::remove_file(&saved.path);
}

#[tokio::test]
async fn save_image_generation_result_rejects_non_standard_base64() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let err = save_image_generation_result(&ontocode_home, "session-1", "ig_urlsafe", "_-8")
        .await
        .expect_err("non-standard base64 should error");
    assert!(matches!(err, CodexErr::InvalidRequest(_)));
}

#[tokio::test]
async fn save_image_generation_result_rejects_non_base64_data_urls() {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let ontocode_home = ontocode_home.path().abs();
    let err = save_image_generation_result(
        &ontocode_home,
        "session-1",
        "ig_svg",
        "data:image/svg+xml,<svg/>",
    )
    .await
    .expect_err("non-base64 data url should error");
    assert!(matches!(err, CodexErr::InvalidRequest(_)));
}
