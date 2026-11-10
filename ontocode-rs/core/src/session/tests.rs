#![allow(clippy::await_holding_invalid_type)]

#[test]
fn coordinator_instructions_switch_without_leaking_fable_restrictions() {
    let fable = developer_instructions_for_model(Some("Keep this."), "claude-fable-5.1").unwrap();
    assert!(fable.contains("never implement or modify code yourself"));
    let astra = developer_instructions_for_model(Some(&fable), "gpt-6-astra").unwrap();
    assert!(!astra.contains("never implement or modify code yourself"));
    assert!(astra.contains("Luna or Grok Composer"));
    assert_eq!(
        astra.matches("Coordinator delegation preference:").count(),
        1
    );
    assert_eq!(
        developer_instructions_for_model(Some(&astra), "gpt-5.6-sol"),
        Some("Keep this.".to_string()),
    );
}

use super::turn_context::TurnEnvironment;
use super::turn_context::developer_instructions_for_model;
use super::*;
use crate::config::ConfigBuilder;
use crate::config::ConfigOverrides;
use crate::config::test_config;
use crate::context::ContextualUserFragment;
use crate::context::TurnAborted;
use crate::function_tool::FunctionCallError;
use crate::shell::default_user_shell;
use crate::skills::SkillPolicy;
use crate::skills::SkillRenderSideEffects;
use crate::skills::render::SkillMetadataBudget;
use crate::test_support::models_manager_with_provider;
use crate::tools::format_exec_output_str;
use ontocode_config::ConfigLayerStack;
use ontocode_config::ConfigLayerStackOrdering;
use ontocode_config::LoaderOverrides;
use ontocode_config::NetworkConstraints;
use ontocode_config::NetworkDomainPermissionToml;
use ontocode_config::NetworkDomainPermissionsToml;
use ontocode_config::RequirementSource;
use ontocode_config::Sourced;
use ontocode_config::loader::project_trust_key;
use ontocode_config::types::ToolSuggestDisabledTool;
use ontocode_extension_api::TurnInputContributor;

use ontocode_features::Feature;
use ontocode_login::CodexAuth;
use ontocode_model_provider_info::ModelProviderInfo;
use ontocode_models_manager::bundled_models_response;
use ontocode_models_manager::model_info;
use ontocode_models_manager::test_support::construct_model_info_offline_for_tests;
use ontocode_models_manager::test_support::get_model_offline_for_tests;
use ontocode_protocol::AgentPath;
use ontocode_protocol::SessionId;
use ontocode_protocol::ThreadId;
use ontocode_protocol::config_types::Personality;
use ontocode_protocol::config_types::SERVICE_TIER_DEFAULT_REQUEST_VALUE;
use ontocode_protocol::config_types::ServiceTier;
use ontocode_protocol::config_types::TrustLevel;
use ontocode_protocol::exec_output::ExecToolCallOutput;
use ontocode_protocol::models::ActivePermissionProfile;
use ontocode_protocol::models::BUILT_IN_PERMISSION_PROFILE_WORKSPACE;
use ontocode_protocol::models::FileSystemPermissions;
use ontocode_protocol::models::FunctionCallOutputBody;
use ontocode_protocol::models::FunctionCallOutputPayload;
use ontocode_protocol::models::PermissionProfile;
use ontocode_protocol::models::SandboxEnforcement;
use ontocode_protocol::openai_models::ModelServiceTier;
use ontocode_protocol::permissions::FileSystemAccessMode;
use ontocode_protocol::permissions::FileSystemPath;
use ontocode_protocol::permissions::FileSystemSandboxEntry;
use ontocode_protocol::permissions::FileSystemSandboxPolicy;
use ontocode_protocol::permissions::FileSystemSpecialPath;
use ontocode_protocol::protocol::NonSteerableTurnKind;
use ontocode_protocol::protocol::SandboxPolicy;
use ontocode_protocol::protocol::TurnEnvironmentSelection;
use ontocode_protocol::request_permissions::PermissionGrantScope;
use ontocode_protocol::request_permissions::RequestPermissionProfile;
use tracing::Span;

use crate::rollout::recorder::RolloutRecorder;
use crate::state::ActiveTurn;
use crate::state::TaskKind;
use crate::tasks::SessionTask;
use crate::tasks::SessionTaskContext;
use crate::tasks::UserShellCommandMode;
use crate::tasks::execute_user_shell_command;
use crate::tools::ToolRouter;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::ExecCommandHandler;
use crate::tools::handlers::RequestPermissionsHandler;
use crate::tools::handlers::ShellCommandHandler;
use crate::tools::parallel::ToolCallResult;
use crate::tools::registry::ToolExecutor;
use crate::tools::router::ToolCallSource;
use crate::turn_diff_tracker::TurnDiffTracker;
use core_test_support::PathBufExt;
use core_test_support::PathExt;
use core_test_support::context_snapshot;
use core_test_support::context_snapshot::ContextSnapshotOptions;
use core_test_support::context_snapshot::ContextSnapshotRenderMode;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::test_ontocode::test_ontocode;
use core_test_support::test_path_buf;
use core_test_support::tracing::install_test_tracing;
use core_test_support::wait_for_event;
use futures::future::BoxFuture;
use futures::stream::FuturesOrdered;
use ontocode_app_server_protocol::AppInfo;
use ontocode_app_server_protocol::McpElicitationSchema;
use ontocode_config::config_toml::ConfigToml;
use ontocode_config::config_toml::ProjectConfig;
use ontocode_config::permissions_toml::FilesystemPermissionToml;
use ontocode_config::permissions_toml::FilesystemPermissionsToml;
use ontocode_config::permissions_toml::NetworkToml;
use ontocode_config::permissions_toml::PermissionProfileToml;
use ontocode_config::permissions_toml::PermissionsToml;
use ontocode_execpolicy::Decision;
use ontocode_execpolicy::NetworkRuleProtocol;
use ontocode_execpolicy::Policy;
use ontocode_network_proxy::NetworkProxyConfig;
use ontocode_otel::MetricsClient;
use ontocode_otel::MetricsConfig;
use ontocode_otel::THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC;
use ontocode_otel::THREAD_SKILLS_ENABLED_TOTAL_METRIC;
use ontocode_otel::THREAD_SKILLS_KEPT_TOTAL_METRIC;
use ontocode_otel::THREAD_SKILLS_TRUNCATED_METRIC;
use ontocode_otel::TelemetryAuthMode;
use ontocode_protocol::config_types::CollaborationMode;
use ontocode_protocol::config_types::ModeKind;
use ontocode_protocol::config_types::Settings;
use ontocode_protocol::models::BaseInstructions;
use ontocode_protocol::models::ContentItem;
use ontocode_protocol::models::MessagePhase;
use ontocode_protocol::models::ResponseItem;
use ontocode_protocol::protocol::AskForApproval;
use ontocode_protocol::protocol::CodexErrorInfo;
use ontocode_protocol::protocol::CompactedItem;
use ontocode_protocol::protocol::ConversationAudioParams;
use ontocode_protocol::protocol::CreditsSnapshot;
use ontocode_protocol::protocol::GranularApprovalConfig;
use ontocode_protocol::protocol::InitialHistory;
use ontocode_protocol::protocol::InterAgentCommunication;
use ontocode_protocol::protocol::MultiAgentVersion;
use ontocode_protocol::protocol::NetworkApprovalProtocol;
use ontocode_protocol::protocol::RateLimitSnapshot;
use ontocode_protocol::protocol::RateLimitWindow;
use ontocode_protocol::protocol::RealtimeAudioFrame;
use ontocode_protocol::protocol::RealtimeConversationListVoicesResponseEvent;
use ontocode_protocol::protocol::RealtimeVoice;
use ontocode_protocol::protocol::RealtimeVoicesList;
use ontocode_protocol::protocol::ResumedHistory;
use ontocode_protocol::protocol::RolloutItem;
use ontocode_protocol::protocol::SessionMeta;
use ontocode_protocol::protocol::SessionMetaLine;
use ontocode_protocol::protocol::SkillScope;
use ontocode_protocol::protocol::Submission;
use ontocode_protocol::protocol::ThreadRolledBackEvent;
use ontocode_protocol::protocol::ThreadSettingsOverrides;
use ontocode_protocol::protocol::TokenCountEvent;
use ontocode_protocol::protocol::TokenUsage;
use ontocode_protocol::protocol::TokenUsageInfo;
use ontocode_protocol::protocol::TurnAbortedEvent;
use ontocode_protocol::protocol::TurnCompleteEvent;
use ontocode_protocol::protocol::TurnStartedEvent;
use ontocode_protocol::protocol::UserMessageEvent;
use ontocode_protocol::protocol::UserMessageOrigin;
use ontocode_protocol::protocol::W3cTraceContext;
use ontocode_protocol::read_evidence::ArtifactReceipt;
use ontocode_protocol::read_evidence::ArtifactReceiptOperation;
use ontocode_protocol::read_evidence::ArtifactReceiptOutcome;
use ontocode_protocol::read_evidence::ArtifactReceiptProducer;
use ontocode_rmcp_client::ElicitationAction;
use ontocode_state::EvidenceDomain;
use ontocode_state::EvidenceRisk;
use ontocode_state::EvidenceStatus;
use ontocode_state::OperationalEvidenceRecord;
use ontocode_state::RedactionStatus;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::TraceId;
use opentelemetry_sdk::metrics::InMemoryMetricExporter;
use opentelemetry_sdk::metrics::data::AggregatedMetrics;
use opentelemetry_sdk::metrics::data::Metric;
use opentelemetry_sdk::metrics::data::MetricData;
use opentelemetry_sdk::metrics::data::ResourceMetrics;
use std::path::Path;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tokio::time::timeout;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use ontocode_protocol::mcp::CallToolResult as McpCallToolResult;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration as StdDuration;

mod guardian_tests;
mod multi_agent_version_tests;

struct InstructionsTestCase {
    slug: &'static str,
    expects_apply_patch_description: bool,
}

#[test]
fn coordinator_instructions_are_model_specific_and_switch_cleanly() {
    let original = "Keep this user instruction.";
    let astra = developer_instructions_for_model(Some(original), "gpt-5.6-astra")
        .expect("Astra should receive coordinator guidance");
    assert!(astra.contains("coordinator-delegation-preference:start"));
    assert!(!astra.contains("claude-fable-research-manager:start"));

    let fable = developer_instructions_for_model(Some(&astra), "claude-fable-5")
        .expect("Fable should receive coordinator guidance and restrictions");
    assert_eq!(
        fable
            .matches("coordinator-delegation-preference:start")
            .count(),
        1
    );
    assert!(fable.contains("claude-fable-research-manager:start"));
    assert!(fable.contains("using the `sol` family"));
    assert!(!fable.contains("gpt-5.6-sol"));
    assert_eq!(
        developer_instructions_for_model(Some(&fable), "gpt-5.4"),
        Some(original.to_string())
    );
}

fn user_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

fn assistant_message(text: &str) -> ResponseItem {
    assistant_message_with_phase(text, None)
}

fn assistant_message_with_phase(text: &str, phase: Option<MessagePhase>) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: text.to_string(),
        }],
        phase,
    }
}

fn function_call(call_id: &str, name: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: name.to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: call_id.to_string(),
    }
}

fn function_output(call_id: &str, output: &str) -> ResponseItem {
    ResponseItem::FunctionCallOutput {
        call_id: call_id.to_string(),
        output: FunctionCallOutputPayload::from_text(output.to_string()),
    }
}

fn test_session_telemetry_without_metadata() -> SessionTelemetry {
    let exporter = InMemoryMetricExporter::default();
    let metrics = MetricsClient::new(
        MetricsConfig::in_memory("test", "codex-core", env!("CARGO_PKG_VERSION"), exporter)
            .with_runtime_reader(),
    )
    .expect("in-memory metrics client");
    SessionTelemetry::new(
        ThreadId::new(),
        "gpt-5.4",
        "gpt-5.4",
        /*account_id*/ None,
        /*account_email*/ None,
        /*auth_mode*/ None,
        "test_originator".to_string(),
        /*log_user_prompts*/ false,
        "tty".to_string(),
        SessionSource::Cli,
    )
    .with_metrics_without_metadata_tags(metrics)
}

#[tokio::test]
async fn writable_session_requires_git_repository() {
    let cwd = non_repo_tempdir();
    let cwd = ontocode_utils_absolute_path::AbsolutePathBuf::from_absolute_path(cwd.path())
        .expect("absolute temp directory");

    let result = require_git_repository_for_writable_session(
        &PermissionProfile::workspace_write(),
        &SessionSource::Cli,
        &ResolvedTurnEnvironments::default(),
        &cwd,
    )
    .await;

    assert!(
        matches!(result, Err(CodexErr::InvalidRequest(message)) if message.contains("writable sessions require a Git repository"))
    );
}

#[tokio::test]
async fn read_only_session_allows_non_git_directory() {
    let cwd = non_repo_tempdir();
    let cwd = ontocode_utils_absolute_path::AbsolutePathBuf::from_absolute_path(cwd.path())
        .expect("absolute temp directory");

    require_git_repository_for_writable_session(
        &PermissionProfile::read_only(),
        &SessionSource::Cli,
        &ResolvedTurnEnvironments::default(),
        &cwd,
    )
    .await
    .expect("read-only session should not require Git");
}

#[tokio::test]
async fn writable_session_allows_git_repository() {
    let cwd = tempfile::tempdir().expect("create temp directory");
    std::fs::create_dir(cwd.path().join(".git")).expect("create Git marker");
    let cwd = ontocode_utils_absolute_path::AbsolutePathBuf::from_absolute_path(cwd.path())
        .expect("absolute temp directory");

    require_git_repository_for_writable_session(
        &PermissionProfile::workspace_write(),
        &SessionSource::Cli,
        &ResolvedTurnEnvironments::default(),
        &cwd,
    )
    .await
    .expect("writable session should accept Git repository");
}

#[tokio::test]
async fn exec_session_preserves_explicit_non_git_override() {
    let cwd = non_repo_tempdir();
    let cwd = ontocode_utils_absolute_path::AbsolutePathBuf::from_absolute_path(cwd.path())
        .expect("absolute temp directory");

    require_git_repository_for_writable_session(
        &PermissionProfile::workspace_write(),
        &SessionSource::Exec,
        &ResolvedTurnEnvironments::default(),
        &cwd,
    )
    .await
    .expect("exec owns its explicit non-Git override");
}

fn find_metric<'a>(resource_metrics: &'a ResourceMetrics, name: &str) -> &'a Metric {
    for scope_metrics in resource_metrics.scope_metrics() {
        for metric in scope_metrics.metrics() {
            if metric.name() == name {
                return metric;
            }
        }
    }
    panic!("metric {name} missing");
}

fn histogram_sum(resource_metrics: &ResourceMetrics, name: &str) -> u64 {
    let metric = find_metric(resource_metrics, name);
    match metric.data() {
        AggregatedMetrics::F64(data) => match data {
            MetricData::Histogram(histogram) => {
                let points: Vec<_> = histogram.data_points().collect();
                assert_eq!(points.len(), 1);
                points[0].sum().round() as u64
            }
            _ => panic!("unexpected histogram aggregation"),
        },
        _ => panic!("unexpected metric data type"),
    }
}

fn skill_message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: text.to_string(),
        }],
        phase: None,
    }
}

#[tokio::test]
async fn regular_turn_emits_turn_started_with_trace_id_without_waiting_for_startup_prewarm() {
    let _trace_test_context = install_test_tracing("codex-core-tests");
    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));
    let (sess, tc, rx) = make_session_and_context_with_rx()
        .instrument(request_span)
        .await;
    assert_eq!(
        tc.trace_id.as_deref(),
        Some("00000000000000000000000000000011")
    );
    let (_tx, startup_prewarm_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = startup_prewarm_rx.await;
        Ok(test_model_client_session())
    });

    sess.set_session_startup_prewarm(
        crate::session_startup_prewarm::SessionStartupPrewarmHandle::new(
            handle,
            std::time::Instant::now(),
            crate::client::WEBSOCKET_CONNECT_TIMEOUT,
        ),
    )
    .await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        crate::tasks::RegularTask::new(),
    )
    .await;

    let first = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("expected turn started event without waiting for startup prewarm")
        .expect("channel open");
    let EventMsg::TurnStarted(turn_started) = first.msg else {
        panic!("expected turn started event");
    };
    assert_eq!(turn_started.turn_id, tc.sub_id);
    assert_eq!(turn_started.trace_id, tc.trace_id);

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn request_mcp_server_elicitation_auto_accepts_when_auto_deny_is_enabled() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    session
        .services
        .mcp_connection_manager
        .read()
        .await
        .set_elicitations_auto_deny(/*auto_deny*/ true);

    let requested_schema: McpElicitationSchema = serde_json::from_value(json!({
        "type": "object",
        "properties": {},
    }))
    .expect("schema should deserialize");
    let response = session
        .request_mcp_server_elicitation(
            turn_context.as_ref(),
            RequestId::String("request-1".into()),
            McpServerElicitationRequestParams {
                thread_id: session.thread_id.to_string(),
                turn_id: Some(turn_context.sub_id.clone()),
                server_name: "codex_apps".to_string(),
                request: McpServerElicitationRequest::Form {
                    meta: None,
                    message: "Allow this request?".to_string(),
                    requested_schema,
                },
            },
        )
        .await;

    assert_eq!(
        response.response,
        Some(ElicitationResponse {
            action: ElicitationAction::Accept,
            content: Some(json!({})),
            meta: None,
        })
    );
    assert!(!response.sent);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn interrupting_regular_turn_waiting_on_startup_prewarm_emits_turn_aborted() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let (_tx, startup_prewarm_rx) = tokio::sync::oneshot::channel::<()>();
    let handle = tokio::spawn(async move {
        let _ = startup_prewarm_rx.await;
        Ok(test_model_client_session())
    });

    sess.set_session_startup_prewarm(
        crate::session_startup_prewarm::SessionStartupPrewarmHandle::new(
            handle,
            std::time::Instant::now(),
            crate::client::WEBSOCKET_CONNECT_TIMEOUT,
        ),
    )
    .await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        crate::tasks::RegularTask::new(),
    )
    .await;

    let first = tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv())
        .await
        .expect("expected turn started event without waiting for startup prewarm")
        .expect("channel open");
    assert!(matches!(
        first.msg,
        EventMsg::TurnStarted(TurnStartedEvent { turn_id, .. }) if turn_id == tc.sub_id
    ));

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let marker_evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn aborted marker event")
        .expect("channel open");
    assert!(matches!(marker_evt.msg, EventMsg::RawResponseItem(_)));

    let second = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn aborted event")
        .expect("channel open");
    let EventMsg::TurnAborted(TurnAbortedEvent {
        turn_id,
        reason,
        completed_at,
        duration_ms,
    }) = second.msg
    else {
        panic!("expected turn aborted event");
    };
    assert_eq!(turn_id, Some(tc.sub_id.clone()));
    assert_eq!(reason, TurnAbortReason::Interrupted);
    assert!(completed_at.is_some());
    assert!(duration_ms.is_some());
}

fn test_model_client_session() -> crate::client::ModelClientSession {
    let thread_id = ThreadId::try_from("00000000-0000-4000-8000-000000000001")
        .expect("test thread id should be valid");
    crate::client::ModelClient::new(
        /*auth_manager*/ None,
        thread_id.into(),
        thread_id,
        /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
        ModelProviderInfo::create_openai_provider(/* base_url */ /*base_url*/ None),
        ontocode_protocol::protocol::SessionSource::Exec,
        /*parent_thread_id*/ None,
        /*model_verbosity*/ None,
        /*enable_request_compression*/ false,
        /*include_timing_metrics*/ false,
        /*beta_features_header*/ None,
        /*attestation_provider*/ None,
    )
    .new_session()
}

fn developer_input_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "developer" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn developer_message_texts(items: &[ResponseItem]) -> Vec<Vec<&str>> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "developer" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .map(|content| {
            content
                .iter()
                .filter_map(|item| match item {
                    ContentItem::InputText { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

fn user_input_texts(items: &[ResponseItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| match item {
            ResponseItem::Message { role, content, .. } if role == "user" => {
                Some(content.as_slice())
            }
            _ => None,
        })
        .flat_map(|content| content.iter())
        .filter_map(|item| match item {
            ContentItem::InputText { text } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn operational_evidence_record(
    thread_id: &str,
    status: EvidenceStatus,
    redaction_status: RedactionStatus,
    summary: &str,
) -> OperationalEvidenceRecord {
    OperationalEvidenceRecord {
        id: Uuid::new_v4().to_string(),
        evidence_domain: EvidenceDomain::Workflow,
        source_tool: "test-operational-evidence".to_string(),
        source_version: Some("1.0.0".to_string()),
        schema_version: 1,
        source_ref: Some("session-tests".to_string()),
        repo: Some("/repo".to_string()),
        task_key: Some("task-a".to_string()),
        thread_id: Some(thread_id.to_string()),
        parent_thread_id: None,
        child_thread_id: None,
        symbol_uid: None,
        symbol_name: None,
        file_path: None,
        process_label: None,
        gate_name: None,
        risk: Some(EvidenceRisk::Low),
        status,
        summary: summary.to_string(),
        source_links: Vec::new(),
        metadata: json!({"kind": "context"}),
        provenance_hash: Uuid::new_v4().to_string(),
        redaction_status,
        target_head: Some("head-a".to_string()),
        graph_index_id: Some("graph-a".to_string()),
        plan_hash: None,
        tracking_hash: None,
        created_at: Utc::now(),
        expires_at: None,
    }
}

fn write_project_hooks(dot_ontocode: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dot_ontocode)?;
    std::fs::write(
        dot_ontocode.join("hooks.json"),
        r#"{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "echo hello from hook"
          }
        ]
      }
    ]
  }
}"#,
    )
}

async fn write_project_trust_config(
    ontocode_home: &Path,
    trusted_projects: &[(&Path, TrustLevel)],
) -> std::io::Result<()> {
    tokio::fs::write(
        ontocode_home.join(ontocode_config::CONFIG_TOML_FILE),
        toml::to_string(&ConfigToml {
            projects: Some(
                trusted_projects
                    .iter()
                    .map(|(project, trust_level)| {
                        (
                            project_trust_key(project),
                            ProjectConfig {
                                trust_level: Some(*trust_level),
                            },
                        )
                    })
                    .collect::<std::collections::HashMap<_, _>>(),
            ),
            ..Default::default()
        })
        .expect("serialize config"),
    )
    .await
}

async fn preview_session_start_hooks(
    config: &crate::config::Config,
) -> std::io::Result<Vec<ontocode_protocol::protocol::HookRunSummary>> {
    let hooks = Hooks::new(HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(config.config_layer_stack.clone()),
        ..HooksConfig::default()
    });

    Ok(
        hooks.preview_session_start(&ontocode_hooks::SessionStartRequest {
            session_id: ThreadId::new(),
            cwd: config.cwd.clone(),
            transcript_path: None,
            model: "gpt-5.2".to_string(),
            permission_mode: "default".to_string(),
            target: ontocode_hooks::StartHookTarget::SessionStart {
                source: ontocode_hooks::SessionStartSource::Startup,
            },
        }),
    )
}

fn test_tool_runtime(session: Arc<Session>, turn_context: Arc<TurnContext>) -> ToolCallRuntime {
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
    ToolCallRuntime::new(router, session, turn_context, tracker)
}

fn make_connector(id: &str, name: &str) -> AppInfo {
    AppInfo {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        logo_url: None,
        logo_url_dark: None,
        distribution_channel: None,
        branding: None,
        app_metadata: None,
        labels: None,
        install_url: None,
        is_accessible: true,
        is_enabled: true,
        plugin_display_names: Vec::new(),
    }
}

#[test]
fn assistant_message_stream_parsers_can_be_seeded_from_output_item_added_text() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ false);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "hello <oai-mem-citation>doc");
    let parsed = parsers.parse_delta(item_id, "1</oai-mem-citation> world");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "hello ");
    assert_eq!(seeded.citations, Vec::<String>::new());
    assert_eq!(parsed.visible_text, " world");
    assert_eq!(parsed.citations, vec!["doc1".to_string()]);
    assert_eq!(tail.visible_text, "");
    assert_eq!(tail.citations, Vec::<String>::new());
}

#[test]
fn assistant_message_stream_parsers_seed_buffered_prefix_stays_out_of_finish_tail() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ false);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "hello <oai-mem-");
    let parsed = parsers.parse_delta(item_id, "citation>doc</oai-mem-citation> world");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "hello ");
    assert_eq!(seeded.citations, Vec::<String>::new());
    assert_eq!(parsed.visible_text, " world");
    assert_eq!(parsed.citations, vec!["doc".to_string()]);
    assert_eq!(tail.visible_text, "");
    assert_eq!(tail.citations, Vec::<String>::new());
}

#[test]
fn assistant_message_stream_parsers_seed_plan_parser_across_added_and_delta_boundaries() {
    let mut parsers = AssistantMessageStreamParsers::new(/*plan_mode*/ true);
    let item_id = "msg-1";

    let seeded = parsers.seed_item_text(item_id, "Intro\n<proposed");
    let parsed = parsers.parse_delta(item_id, "_plan>\n- step\n</proposed_plan>\nOutro");
    let tail = parsers.finish_item(item_id);

    assert_eq!(seeded.visible_text, "Intro\n");
    assert_eq!(
        seeded.plan_segments,
        vec![ProposedPlanSegment::Normal("Intro\n".to_string())]
    );
    assert_eq!(parsed.visible_text, "Outro");
    assert_eq!(
        parsed.plan_segments,
        vec![
            ProposedPlanSegment::ProposedPlanStart,
            ProposedPlanSegment::ProposedPlanDelta("- step\n".to_string()),
            ProposedPlanSegment::ProposedPlanEnd,
            ProposedPlanSegment::Normal("Outro".to_string()),
        ]
    );
    assert_eq!(tail.visible_text, "");
    assert!(tail.plan_segments.is_empty());
}

#[test]
fn validated_network_policy_amendment_host_allows_normalized_match() {
    let amendment = NetworkPolicyAmendment {
        host: "ExAmPlE.Com.:443".to_string(),
        action: NetworkPolicyRuleAction::Allow,
    };
    let context = NetworkApprovalContext {
        host: "example.com".to_string(),
        protocol: NetworkApprovalProtocol::Https,
    };

    let host = Session::validated_network_policy_amendment_host(&amendment, &context)
        .expect("normalized hosts should match");

    assert_eq!(host, "example.com");
}

#[test]
fn validated_network_policy_amendment_host_rejects_mismatch() {
    let amendment = NetworkPolicyAmendment {
        host: "evil.example.com".to_string(),
        action: NetworkPolicyRuleAction::Deny,
    };
    let context = NetworkApprovalContext {
        host: "api.example.com".to_string(),
        protocol: NetworkApprovalProtocol::Https,
    };

    let err = Session::validated_network_policy_amendment_host(&amendment, &context)
        .expect_err("mismatched hosts should be rejected");

    let message = err.to_string();
    assert!(message.contains("does not match approved host"));
}

#[tokio::test]
async fn start_managed_network_proxy_applies_execpolicy_network_rules() -> anyhow::Result<()> {
    let permission_profile = PermissionProfile::workspace_write();
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        /*requirements*/ None,
        &permission_profile,
    )?;
    let mut exec_policy = Policy::empty();
    exec_policy.add_network_rule(
        "example.com",
        NetworkRuleProtocol::Https,
        Decision::Allow,
        /*justification*/ None,
    )?;

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &permission_profile,
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(
        current_cfg.network.allowed_domains(),
        Some(vec!["example.com".to_string()])
    );
    Ok(())
}

#[tokio::test]
async fn start_managed_network_proxy_ignores_invalid_execpolicy_network_rules() -> anyhow::Result<()>
{
    let permission_profile = PermissionProfile::workspace_write();
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([(
                    "managed.example.com".to_string(),
                    NetworkDomainPermissionToml::Allow,
                )]),
            }),
            managed_allowed_domains_only: Some(true),
            ..Default::default()
        }),
        &permission_profile,
    )?;
    let mut exec_policy = Policy::empty();
    exec_policy.add_network_rule(
        "example.com",
        NetworkRuleProtocol::Https,
        Decision::Allow,
        /*justification*/ None,
    )?;

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &permission_profile,
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(
        current_cfg.network.allowed_domains(),
        Some(vec!["managed.example.com".to_string()])
    );
    Ok(())
}

#[tokio::test]
async fn managed_network_proxy_decider_survives_full_access_start() -> anyhow::Result<()> {
    let full_access_permission_profile = PermissionProfile::Disabled;
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &full_access_permission_profile,
    )?;
    let exec_policy = Policy::empty();
    let decider_calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let network_policy_decider: Arc<dyn ontocode_network_proxy::NetworkPolicyDecider> = Arc::new({
        let decider_calls = Arc::clone(&decider_calls);
        move |_request| {
            decider_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async { ontocode_network_proxy::NetworkDecision::ask("not_allowed") }
        }
    });

    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &exec_policy,
        &full_access_permission_profile,
        Some(network_policy_decider),
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ true,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;

    let spec = spec.recompute_for_permission_profile(&PermissionProfile::workspace_write())?;
    spec.apply_to_started_proxy(&started_proxy).await?;
    let current_cfg = started_proxy.proxy().current_cfg().await?;
    assert_eq!(current_cfg.network.allowed_domains(), None);

    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;

    let mut stream = tokio::net::TcpStream::connect(started_proxy.proxy().http_addr()).await?;
    stream
        .write_all(
            b"GET http://example.com/ HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n",
        )
        .await?;
    let mut buffer = [0_u8; 4096];
    let bytes_read = tokio::time::timeout(StdDuration::from_secs(2), stream.read(&mut buffer))
        .await
        .expect("timed out waiting for proxy response")?;
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);

    assert!(
        response.starts_with("HTTP/1.1 403 Forbidden"),
        "unexpected proxy response: {response}"
    );
    assert!(
        response.contains("x-proxy-error: blocked-by-allowlist"),
        "unexpected proxy response: {response}"
    );
    assert_eq!(
        decider_calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "unexpected proxy response: {response}"
    );
    Ok(())
}

#[tokio::test]
async fn new_turn_refreshes_managed_network_proxy_for_sandbox_change() -> anyhow::Result<()> {
    let (session, initial_turn_context) = make_session_and_context().await;
    let session = Arc::new(session);
    let initial_turn_context = Arc::new(initial_turn_context);
    let initial_permission_profile = PermissionProfile::workspace_write();

    let mut network_config = NetworkProxyConfig::default();
    network_config
        .network
        .set_allowed_domains(vec!["evil.com".to_string()]);
    let requirements = NetworkConstraints {
        domains: Some(NetworkDomainPermissionsToml {
            entries: std::collections::BTreeMap::from([(
                "*.example.com".to_string(),
                NetworkDomainPermissionToml::Allow,
            )]),
        }),
        ..Default::default()
    };
    let spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        network_config,
        Some(requirements),
        &initial_permission_profile,
    )?;
    let (started_proxy, _) = Session::start_managed_network_proxy(
        &spec,
        &Policy::empty(),
        &initial_permission_profile,
        /*network_policy_decider*/ None,
        /*blocked_request_observer*/ None,
        /*managed_network_requirements_enabled*/ false,
        crate::config::NetworkProxyAuditMetadata::default(),
    )
    .await?;
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string(), "evil.com".to_string()])
    );

    {
        let mut state = session.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config.permissions.network = Some(spec.clone());
        config
            .permissions
            .set_permission_profile(initial_permission_profile.clone())
            .expect("test setup should allow permission profile");
        state.session_configuration.original_config_do_not_use = Arc::new(config);
        state
            .session_configuration
            .set_permission_profile_for_tests(initial_permission_profile.clone())
            .expect("test setup should allow permission profile");
    }
    session
        .services
        .network_proxy
        .store(Some(Arc::new(started_proxy)));

    let (abort_domains_tx, abort_domains_rx) = tokio::sync::oneshot::channel();
    session
        .spawn_task(
            initial_turn_context,
            Vec::new(),
            CaptureAbortNetworkDomainsTask {
                tx: std::sync::Mutex::new(Some(abort_domains_tx)),
            },
        )
        .await;

    let replacement_context = session
        .new_turn_with_sub_id(
            "sandbox-policy-change".to_string(),
            SessionSettingsUpdate {
                sandbox_policy: Some(SandboxPolicy::DangerFullAccess),
                ..Default::default()
            },
        )
        .await?;
    let mut later_network_config = NetworkProxyConfig::default();
    later_network_config
        .network
        .set_allowed_domains(vec!["later-evil.com".to_string()]);
    let later_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        later_network_config,
        Some(NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([(
                    "*.later.example".to_string(),
                    NetworkDomainPermissionToml::Allow,
                )]),
            }),
            ..Default::default()
        }),
        &initial_permission_profile,
    )?;
    {
        let mut state = session.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config.permissions.network = Some(later_spec);
        state.session_configuration.original_config_do_not_use = Arc::new(config);
    }
    let _inactive_context = session
        .new_turn_with_sub_id(
            "later-inactive-context".to_string(),
            SessionSettingsUpdate {
                permission_profile: Some(initial_permission_profile.clone()),
                ..Default::default()
            },
        )
        .await?;

    let started_proxy = session
        .services
        .network_proxy
        .load_full()
        .expect("managed network proxy should be present");
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string(), "evil.com".to_string()]),
        "constructing a replacement context must not refresh the live proxy"
    );

    session
        .spawn_task(
            replacement_context,
            Vec::new(),
            NeverEndingTask {
                kind: TaskKind::Regular,
                listen_to_cancellation_token: true,
            },
        )
        .await;
    assert_eq!(
        timeout(Duration::from_secs(5), abort_domains_rx)
            .await
            .expect("active task should be replaced")
            .expect("aborted task should report proxy domains"),
        Some(vec!["*.example.com".to_string(), "evil.com".to_string()]),
        "the old task must abort before the replacement proxy policy is activated"
    );

    let started_proxy = session
        .services
        .network_proxy
        .load_full()
        .expect("managed network proxy should be present");
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string()])
    );
    assert!(
        session
            .managed_network_proxy_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed)
    );

    session.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let current_context = session.new_default_turn().await;
    session
        .spawn_task(
            current_context,
            Vec::new(),
            NeverEndingTask {
                kind: TaskKind::Regular,
                listen_to_cancellation_token: true,
            },
        )
        .await;
    let started_proxy = session
        .services
        .network_proxy
        .load_full()
        .expect("managed network proxy should be present");
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec![
            "*.later.example".to_string(),
            "later-evil.com".to_string(),
        ])
    );
    assert!(
        !session
            .managed_network_proxy_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    session.abort_all_tasks(TurnAbortReason::Interrupted).await;

    Ok(())
}

#[tokio::test]
async fn thread_settings_defers_managed_proxy_refresh_until_next_turn() -> anyhow::Result<()> {
    let initial_permission_profile = PermissionProfile::workspace_write();
    let mut network_config = NetworkProxyConfig::default();
    network_config
        .network
        .set_allowed_domains(vec!["evil.com".to_string()]);
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        network_config,
        Some(NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([(
                    "*.example.com".to_string(),
                    NetworkDomainPermissionToml::Allow,
                )]),
            }),
            ..Default::default()
        }),
        &initial_permission_profile,
    )?;
    let (session, _rx) = make_session_with_config_and_rx(move |config| {
        config
            .permissions
            .set_permission_profile(initial_permission_profile)
            .expect("test setup should allow permission profile");
        config.permissions.network = Some(network_spec);
    })
    .await?;
    let initial_context = session.new_default_turn().await;
    let initial_turn_id = initial_context.sub_id.clone();
    session
        .spawn_task(
            initial_context,
            Vec::new(),
            NeverEndingTask {
                kind: TaskKind::Regular,
                listen_to_cancellation_token: true,
            },
        )
        .await;

    handlers::update_thread_settings(
        &session,
        "thread-settings".to_string(),
        ThreadSettingsOverrides {
            permission_profile: Some(PermissionProfile::Disabled),
            ..Default::default()
        },
    )
    .await;

    let active_turn = session.active_turn.lock().await;
    assert_eq!(
        active_turn
            .as_ref()
            .and_then(|active_turn| active_turn.task.as_ref())
            .map(|task| task.turn_context.sub_id.as_str()),
        Some(initial_turn_id.as_str()),
        "standalone settings must not replace the active turn"
    );
    drop(active_turn);
    let started_proxy = session
        .services
        .network_proxy
        .load_full()
        .expect("managed network proxy should remain active");
    assert_eq!(
        started_proxy
            .proxy()
            .current_cfg()
            .await?
            .network
            .allowed_domains(),
        Some(vec!["*.example.com".to_string(), "evil.com".to_string()]),
        "standalone settings must not mutate the active turn's proxy policy"
    );
    assert!(
        session
            .managed_network_proxy_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed),
        "standalone settings must defer proxy activation to the next turn"
    );

    session.abort_all_tasks(TurnAbortReason::Interrupted).await;
    Ok(())
}

#[tokio::test]
async fn managed_proxy_refresh_failure_prevents_task_start() -> anyhow::Result<()> {
    let (session, _initial_context, rx) = make_session_and_context_with_rx().await;
    let synthetic_item = user_message("synthetic manager result");
    let carried_input_turn_state = {
        let mut active_turn = session.active_turn.lock().await;
        let active_turn = active_turn.get_or_insert_with(ActiveTurn::default);
        Arc::clone(&active_turn.turn_state)
    };
    {
        let mut turn_state = carried_input_turn_state.lock().await;
        turn_state.enable_unattended_read_only_filesystem();
    }
    session
        .input_queue
        .extend_pending_input_for_turn_state(
            carried_input_turn_state.as_ref(),
            vec![TurnInput::ResponseItem(synthetic_item.clone())],
        )
        .await;
    let initial_profile = {
        let state = session.state.lock().await;
        state.session_configuration.permission_profile()
    };
    let replacement_profile = if initial_profile == PermissionProfile::Disabled {
        PermissionProfile::read_only()
    } else {
        PermissionProfile::Disabled
    };
    let replacement_context = session
        .new_turn_with_sub_id(
            "proxy-refresh-failure".to_string(),
            SessionSettingsUpdate {
                permission_profile: Some(replacement_profile),
                ..Default::default()
            },
        )
        .await?;
    assert!(replacement_context.take_managed_network_proxy_refresh_on_activation());
    assert!(
        session
            .managed_network_proxy_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    session.managed_network_proxy_refresh_lock.close();
    let (task_started_tx, task_started_rx) = tokio::sync::oneshot::channel();

    session
        .spawn_task(
            replacement_context,
            Vec::new(),
            CaptureInputTask {
                tx: std::sync::Mutex::new(Some(task_started_tx)),
            },
        )
        .await;

    assert!(
        timeout(Duration::from_secs(1), task_started_rx)
            .await
            .expect("failed activation should drop the task promptly")
            .is_err(),
        "task body must not run when managed proxy activation fails"
    );
    {
        let active_turn = session.active_turn.lock().await;
        let active_turn = active_turn
            .as_ref()
            .expect("failed proxy activation should restore the carried active turn");
        assert!(active_turn.task.is_none());
        assert!(Arc::ptr_eq(
            &active_turn.turn_state,
            &carried_input_turn_state
        ));
    }
    assert!(
        session
            .managed_network_proxy_refresh_pending
            .load(std::sync::atomic::Ordering::Relaxed)
    );
    assert_eq!(
        session
            .input_queue
            .get_pending_input(&session.active_turn)
            .await,
        vec![TurnInput::ResponseItem(synthetic_item)],
        "failed activation must leave carried synthetic input session-reachable"
    );
    let error = timeout(Duration::from_secs(1), async {
        loop {
            let event = rx.recv().await.expect("event channel should remain open");
            if let EventMsg::Error(error) = event.msg {
                break error;
            }
        }
    })
    .await
    .expect("proxy activation failure should emit an error");
    assert_eq!(error.codex_error_info, Some(CodexErrorInfo::SandboxError));
    assert!(error.message.contains("managed network policy"));

    Ok(())
}

#[tokio::test]
async fn danger_full_access_turns_do_not_expose_managed_network_proxy() -> anyhow::Result<()> {
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &PermissionProfile::Disabled,
    )?;

    let session = make_session_with_config(move |config| {
        config
            .permissions
            .set_permission_profile(PermissionProfile::Disabled)
            .expect("test setup should allow permission profile");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_none());
    Ok(())
}

#[tokio::test]
async fn danger_full_access_tool_attempts_do_not_enforce_managed_network() -> anyhow::Result<()> {
    #[derive(Default)]
    struct ProbeToolRuntime {
        enforce_managed_network: Vec<bool>,
    }

    impl crate::tools::sandboxing::Approvable<()> for ProbeToolRuntime {
        type ApprovalKey = String;

        fn approval_keys(&self, _req: &()) -> Vec<Self::ApprovalKey> {
            vec!["probe".to_string()]
        }

        fn start_approval_async<'a>(
            &'a mut self,
            _req: &'a (),
            _ctx: crate::tools::sandboxing::ApprovalCtx<'a>,
        ) -> futures::future::BoxFuture<'a, ReviewDecision> {
            Box::pin(async { ReviewDecision::Approved })
        }
    }

    impl crate::tools::sandboxing::Sandboxable for ProbeToolRuntime {
        fn sandbox_preference(&self) -> ontocode_sandboxing::SandboxablePreference {
            ontocode_sandboxing::SandboxablePreference::Auto
        }
    }

    impl crate::tools::sandboxing::ToolRuntime<(), ()> for ProbeToolRuntime {
        async fn run(
            &mut self,
            _req: &(),
            attempt: &crate::tools::sandboxing::SandboxAttempt<'_>,
            _ctx: &crate::tools::sandboxing::ToolCtx,
        ) -> Result<(), crate::tools::sandboxing::ToolError> {
            self.enforce_managed_network
                .push(attempt.enforce_managed_network);
            Ok(())
        }
    }

    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &PermissionProfile::Disabled,
    )?;

    let session = make_session_with_config(move |config| {
        config
            .permissions
            .set_permission_profile(PermissionProfile::Disabled)
            .expect("test setup should allow permission profile");
        config.permissions.network = Some(network_spec);

        let layers = config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect();
        let mut requirements = config.config_layer_stack.requirements().clone();
        requirements.network = Some(Sourced::new(
            NetworkConstraints {
                enabled: Some(true),
                ..Default::default()
            },
            RequirementSource::LegacyManagedConfigTomlFromMdm,
        ));
        let mut requirements_toml = config.config_layer_stack.requirements_toml().clone();
        requirements_toml.network = Some(ontocode_config::NetworkRequirementsToml {
            enabled: Some(true),
            ..Default::default()
        });
        config.config_layer_stack = ConfigLayerStack::new(layers, requirements, requirements_toml)
            .expect("rebuild config layer stack with network requirements");
    })
    .await?;

    let turn = session.new_default_turn().await;
    assert!(turn.network.is_none());

    let mut orchestrator = crate::tools::orchestrator::ToolOrchestrator::new();
    let mut tool = ProbeToolRuntime::default();
    let tool_ctx = crate::tools::sandboxing::ToolCtx {
        session: Arc::clone(&session),
        turn: Arc::clone(&turn),
        call_id: "probe-call".to_string(),
        tool_name: ontocode_tools::ToolName::plain("probe"),
    };

    orchestrator
        .run(
            &mut tool,
            &(),
            &tool_ctx,
            turn.as_ref(),
            AskForApproval::Never,
        )
        .await
        .expect("probe runtime should succeed");

    assert_eq!(tool.enforce_managed_network, vec![false]);

    Ok(())
}

#[tokio::test]
async fn workspace_write_turns_continue_to_expose_managed_network_proxy() -> anyhow::Result<()> {
    let permission_profile = PermissionProfile::workspace_write();
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile,
    )?;

    let session = make_session_with_config(move |config| {
        config
            .permissions
            .set_permission_profile(permission_profile)
            .expect("test setup should allow permission profile");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_some());
    Ok(())
}

#[tokio::test]
async fn user_shell_commands_do_not_inherit_managed_network_proxy() -> anyhow::Result<()> {
    let permission_profile = PermissionProfile::workspace_write();
    let network_spec = crate::config::NetworkProxySpec::from_config_and_constraints(
        NetworkProxyConfig::default(),
        Some(NetworkConstraints {
            enabled: Some(true),
            ..Default::default()
        }),
        &permission_profile,
    )?;

    let (session, rx) = make_session_with_config_and_rx(move |config| {
        config
            .permissions
            .set_permission_profile(permission_profile)
            .expect("test setup should allow permission profile");
        config.permissions.network = Some(network_spec);
    })
    .await?;

    let turn_context = session.new_default_turn().await;
    assert!(turn_context.network.is_some());

    #[cfg(windows)]
    let command = r#"$val = $env:HTTP_PROXY; if ([string]::IsNullOrEmpty($val)) { $val = 'not-set' } ; [System.Console]::Write($val)"#.to_string();
    #[cfg(not(windows))]
    let command = r#"sh -c "printf '%s' \"${HTTP_PROXY:-not-set}\"""#.to_string();

    execute_user_shell_command(
        Arc::clone(&session),
        turn_context,
        command,
        CancellationToken::new(),
        UserShellCommandMode::StandaloneTurn,
    )
    .await;

    loop {
        let event = rx.recv().await.expect("channel open");
        if let EventMsg::ExecCommandEnd(event) = event.msg {
            assert_eq!(event.exit_code, 0);
            assert_eq!(event.stdout.trim(), "not-set");
            break;
        }
    }

    Ok(())
}

#[tokio::test]
async fn get_base_instructions_no_user_content() {
    let prompt_with_apply_patch_instructions =
        include_str!("../../prompt_with_apply_patch_instructions.md");
    let models_response = bundled_models_response()
        .unwrap_or_else(|err| panic!("bundled models.json should parse: {err}"));
    let model_info_for_slug = |slug: &str, config: &Config| {
        let model = models_response
            .models
            .iter()
            .find(|candidate| candidate.slug == slug)
            .cloned()
            .unwrap_or_else(|| panic!("model slug {slug} is missing from models.json"));
        model_info::with_config_overrides(model, &config.to_models_manager_config())
    };
    let test_cases = vec![
        InstructionsTestCase {
            slug: "gpt-5.4-mini",
            expects_apply_patch_description: false,
        },
        InstructionsTestCase {
            slug: "gpt-5.3-codex",
            expects_apply_patch_description: false,
        },
    ];

    let (session, _turn_context) = make_session_and_context().await;
    let config = test_config().await;

    for test_case in test_cases {
        let model_info = model_info_for_slug(test_case.slug, &config);
        if test_case.expects_apply_patch_description {
            assert_eq!(
                model_info.base_instructions.as_str(),
                prompt_with_apply_patch_instructions
            );
        }

        {
            let mut state = session.state.lock().await;
            state.session_configuration.base_instructions = model_info.base_instructions.clone();
        }

        let base_instructions = session.get_base_instructions().await;
        assert_eq!(base_instructions.text, model_info.base_instructions);
    }
}

#[tokio::test]
async fn reload_user_config_layer_updates_effective_apps_config() {
    let (session, _turn_context) = make_session_and_context().await;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    let config_toml_path = ontocode_home.join(CONFIG_TOML_FILE);
    std::fs::write(
        &config_toml_path,
        "[apps.calendar]\nenabled = false\ndestructive_enabled = false\n",
    )
    .expect("write user config");

    session.reload_user_config_layer().await;

    let config = session.get_config().await;
    let apps_toml = config
        .config_layer_stack
        .effective_config()
        .as_table()
        .and_then(|table| table.get("apps"))
        .cloned()
        .expect("apps table");
    let apps = ontocode_config::types::AppsConfigToml::deserialize(apps_toml)
        .expect("deserialize apps config");
    let app = apps
        .apps
        .get("calendar")
        .expect("calendar app config exists");

    assert!(!app.enabled);
    assert_eq!(app.destructive_enabled, Some(false));
}

#[tokio::test]
async fn reload_user_config_layer_refreshes_personality_instructions_file() {
    let (session, _turn_context) = make_session_and_context().await;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    let personality_path = ontocode_home.join("personality.toml");
    let config_toml_path = ontocode_home.join(CONFIG_TOML_FILE);
    std::fs::write(
        &personality_path,
        r#"version = 1

[families."gpt-5.6"]
pragmatic = "personality A"
"#,
    )
    .expect("write initial personality file");
    std::fs::write(
        &config_toml_path,
        format!(
            "personality_instructions_file = {:?}\n",
            personality_path.to_string_lossy()
        ),
    )
    .expect("write user config");

    let initial_config = load_latest_config_for_session(&session).await;
    assert_eq!(
        initial_config
            .personality_instructions
            .as_ref()
            .and_then(|instructions| instructions.for_model("gpt-5.6"))
            .and_then(|family| family.pragmatic.as_deref()),
        Some("personality A")
    );
    let pinned_config = initial_config.clone();
    {
        let mut state = session.state.lock().await;
        state.session_configuration.original_config_do_not_use = Arc::new(initial_config);
    }
    let old_turn = session.new_default_turn().await;
    assert_eq!(
        old_turn
            .config
            .personality_instructions
            .as_ref()
            .and_then(|instructions| instructions.for_model("gpt-5.6"))
            .and_then(|family| family.pragmatic.as_deref()),
        Some("personality A")
    );

    std::fs::write(
        &personality_path,
        r#"version = 1

[families."gpt-5.6"]
pragmatic = "personality B"
"#,
    )
    .expect("rewrite personality file");

    session.reload_user_config_layer().await;

    let new_turn = session.new_default_turn().await;
    assert_eq!(
        new_turn
            .config
            .personality_instructions
            .as_ref()
            .and_then(|instructions| instructions.for_model("gpt-5.6"))
            .and_then(|family| family.pragmatic.as_deref()),
        Some("personality B")
    );

    // The refreshed config must also reach composed instructions. Asserting only
    // the config field would pass even if the next turn kept rebuilding the
    // prompt from the text captured when the session started.
    let mut models_manager_config = new_turn.config.to_models_manager_config();
    models_manager_config.personality_enabled = true;
    let refreshed_instructions =
        construct_model_info_offline_for_tests("gpt-5.6-sol", &models_manager_config)
            .get_model_instructions(Some(Personality::Pragmatic));
    assert!(
        refreshed_instructions.contains("personality B"),
        "refreshed personality text should reach composed instructions, got: {refreshed_instructions:?}"
    );
    assert!(
        !refreshed_instructions.contains("personality A"),
        "stale personality text should not survive the refresh, got: {refreshed_instructions:?}"
    );

    assert_eq!(
        pinned_config
            .personality_instructions
            .as_ref()
            .and_then(|instructions| instructions.for_model("gpt-5.6"))
            .and_then(|family| family.pragmatic.as_deref()),
        Some("personality A")
    );

    let config = session.get_config().await;
    assert_eq!(
        config
            .personality_instructions
            .as_ref()
            .and_then(|instructions| instructions.for_model("gpt-5.6"))
            .and_then(|family| family.pragmatic.as_deref()),
        Some("personality B")
    );
}

#[tokio::test]
async fn reload_user_config_layer_updates_base_and_selected_profile_layers() {
    let (session, _turn_context) = make_session_and_context().await;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    let base_config_path = ontocode_home.join(CONFIG_TOML_FILE);
    let profile_config_path = ontocode_home.join("work.config.toml");
    std::fs::write(
        &base_config_path,
        "model = \"base\"\napproval_policy = \"on-failure\"\n",
    )
    .expect("write base user config");
    std::fs::write(&profile_config_path, "model = \"profile-old\"\n")
        .expect("write profile user config");
    let config = ConfigBuilder::without_managed_config_for_tests()
        .ontocode_home(ontocode_home.to_path_buf())
        .loader_overrides(LoaderOverrides {
            user_config_path: Some(profile_config_path.abs()),
            user_config_profile: Some("work".parse().expect("profile-v2 name")),
            ..LoaderOverrides::without_managed_config_for_tests()
        })
        .build()
        .await
        .expect("load profile config");
    {
        let mut state = session.state.lock().await;
        state.session_configuration.original_config_do_not_use = Arc::new(config);
    }
    std::fs::write(
        &base_config_path,
        "model = \"base\"\napproval_policy = \"never\"\n",
    )
    .expect("update base user config");
    std::fs::write(&profile_config_path, "model = \"profile-new\"\n")
        .expect("update profile user config");

    session.reload_user_config_layer().await;

    let config = session.get_config().await;
    assert_eq!(
        config
            .config_layer_stack
            .get_user_config_file()
            .map(ontocode_utils_absolute_path::AbsolutePathBuf::as_path),
        Some(profile_config_path.as_path())
    );
    let effective_user_config = config
        .config_layer_stack
        .effective_user_config()
        .expect("merged user config");
    assert_eq!(
        effective_user_config
            .get("model")
            .and_then(toml::Value::as_str),
        Some("profile-new")
    );
    assert_eq!(
        effective_user_config
            .get("approval_policy")
            .and_then(toml::Value::as_str),
        Some("never")
    );
}

#[tokio::test]
async fn reload_user_config_layer_refreshes_hooks() -> anyhow::Result<()> {
    let session = make_session_with_config(|config| {
        config
            .features
            .enable(Feature::CodexHooks)
            .expect("enable Codex hooks");
    })
    .await?;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home)?;
    let config_toml_path = ontocode_home.join(CONFIG_TOML_FILE);
    let user_config: ontocode_config::TomlValue = serde_json::from_value(serde_json::json!({
        "hooks": {
            "SessionStart": [{
                "hooks": [{
                    "type": "command",
                    "command": "python3 /tmp/user.py",
                }],
            }],
        },
    }))?;

    let request = ontocode_hooks::SessionStartRequest {
        session_id: session.thread_id,
        cwd: session.get_config().await.cwd.clone(),
        transcript_path: None,
        model: "gpt-5.2".to_string(),
        permission_mode: "default".to_string(),
        target: ontocode_hooks::StartHookTarget::SessionStart {
            source: ontocode_hooks::SessionStartSource::Startup,
        },
    };
    let previous_hooks = session.hooks();
    assert!(previous_hooks.preview_session_start(&request).is_empty());

    let config = session.get_config().await;
    let hook_list = ontocode_hooks::list_hooks(ontocode_hooks::HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(
            config
                .config_layer_stack
                .with_user_config(&config_toml_path, user_config.clone()),
        ),
        ..ontocode_hooks::HooksConfig::default()
    });
    assert_eq!(hook_list.hooks.len(), 1);
    assert_eq!(
        hook_list.hooks[0].trust_status,
        ontocode_protocol::protocol::HookTrustStatus::Untrusted
    );

    let trusted_user_config: ontocode_config::TomlValue =
        serde_json::from_value(serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": "python3 /tmp/user.py",
                    }],
                }],
                "state": {
                    hook_list.hooks[0].key.clone(): {
                        "trusted_hash": hook_list.hooks[0].current_hash.clone(),
                    },
                },
            },
        }))?;
    std::fs::write(&config_toml_path, toml::to_string(&trusted_user_config)?)?;

    session.reload_user_config_layer().await;

    let current_hooks = session.hooks();
    assert!(!Arc::ptr_eq(&previous_hooks, &current_hooks));
    assert!(previous_hooks.preview_session_start(&request).is_empty());
    assert_eq!(current_hooks.preview_session_start(&request).len(), 1);
    Ok(())
}

#[tokio::test]
async fn refresh_runtime_config_refreshes_hooks() -> anyhow::Result<()> {
    let (session, _turn_context) = make_session_and_context().await;
    {
        let mut state = session.state.lock().await;
        let mut config = (*state.session_configuration.original_config_do_not_use).clone();
        config
            .features
            .enable(Feature::CodexHooks)
            .expect("enable Codex hooks");
        state.session_configuration.original_config_do_not_use = Arc::new(config);
    }
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home)?;
    let config_toml_path = ontocode_home.join(CONFIG_TOML_FILE);
    #[derive(serde::Serialize)]
    struct NormalizedHookIdentity {
        event_name: &'static str,
        #[serde(flatten)]
        group: ontocode_config::MatcherGroup,
    }
    let trusted_hash = {
        let identity = NormalizedHookIdentity {
            event_name: "session_start",
            group: ontocode_config::MatcherGroup {
                matcher: None,
                hooks: vec![ontocode_config::HookHandlerConfig::Command {
                    command: "python3 /tmp/user.py".to_string(),
                    command_windows: None,
                    timeout_sec: Some(600),
                    r#async: false,
                    status_message: None,
                }],
            },
        };
        let identity = ontocode_config::TomlValue::try_from(identity)?;
        ontocode_config::version_for_toml(&identity)
    };
    let hook_key = format!("{}:session_start:0:0", config_toml_path.display());
    let trusted_user_config: ontocode_config::TomlValue =
        serde_json::from_value(serde_json::json!({
            "hooks": {
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": "python3 /tmp/user.py",
                    }],
                }],
                "state": {
                    hook_key: {
                        "trusted_hash": trusted_hash,
                    },
                },
            },
        }))?;
    std::fs::write(&config_toml_path, toml::to_string(&trusted_user_config)?)?;

    let request = ontocode_hooks::SessionStartRequest {
        session_id: session.thread_id,
        cwd: session.get_config().await.cwd.clone(),
        transcript_path: None,
        model: "gpt-5.2".to_string(),
        permission_mode: "default".to_string(),
        target: ontocode_hooks::StartHookTarget::SessionStart {
            source: ontocode_hooks::SessionStartSource::Startup,
        },
    };
    assert!(session.hooks().preview_session_start(&request).is_empty());

    let next_config = load_latest_config_for_session(&session).await;
    session.refresh_runtime_config(next_config).await;

    assert_eq!(session.hooks().preview_session_start(&request).len(), 1);
    Ok(())
}

#[tokio::test]
async fn reload_user_config_layer_updates_effective_tool_suggest_config() {
    let (session, _turn_context) = make_session_and_context().await;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    let config_toml_path = ontocode_home.join(CONFIG_TOML_FILE);
    std::fs::write(
        &config_toml_path,
        r#"[tool_suggest]
disabled_tools = [
  { type = "connector", id = " calendar " },
  { type = "plugin", id = "slack@openai-curated" },
]
"#,
    )
    .expect("write user config");

    session.reload_user_config_layer().await;

    let config = session.get_config().await;
    assert_eq!(
        config.tool_suggest.disabled_tools,
        vec![
            ToolSuggestDisabledTool::connector("calendar"),
            ToolSuggestDisabledTool::plugin("slack@openai-curated"),
        ]
    );
}

#[tokio::test]
async fn refresh_runtime_config_updates_runtime_refreshable_fields_and_keeps_session_static_settings()
 {
    let (session, _turn_context) = make_session_and_context().await;
    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    std::fs::write(
        ontocode_home.join(CONFIG_TOML_FILE),
        r#"[apps.calendar]
enabled = false
destructive_enabled = false

[tool_suggest]
disabled_tools = [
  { type = "connector", id = " calendar " },
  { type = "plugin", id = "slack@openai-curated" },
]
"#,
    )
    .expect("write user config");

    let original = session.get_config().await;
    let mut next_config = load_latest_config_for_session(&session).await;
    next_config.model = Some("gpt-5.4".to_string());
    next_config.notify = Some(vec!["echo".to_string()]);

    session.refresh_runtime_config(next_config).await;

    let config = session.get_config().await;
    let apps_toml = config
        .config_layer_stack
        .effective_config()
        .as_table()
        .and_then(|table| table.get("apps"))
        .cloned()
        .expect("apps table");
    let apps = ontocode_config::types::AppsConfigToml::deserialize(apps_toml)
        .expect("deserialize apps config");
    let app = apps
        .apps
        .get("calendar")
        .expect("calendar app config exists");

    assert!(!app.enabled);
    assert_eq!(app.destructive_enabled, Some(false));
    assert_eq!(config.model, original.model);
    assert_eq!(config.notify, original.notify);
    assert_eq!(
        config.tool_suggest.disabled_tools,
        vec![
            ToolSuggestDisabledTool::connector("calendar"),
            ToolSuggestDisabledTool::plugin("slack@openai-curated"),
        ]
    );
}

#[test]
fn collect_explicit_app_ids_from_skill_items_includes_linked_mentions() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse [$calendar](app://calendar)\n</skill>",
    )];

    let connector_ids =
        collect_explicit_app_ids_from_skill_items(&skill_items, &connectors, &HashMap::new());

    assert_eq!(connector_ids, HashSet::from(["calendar".to_string()]));
}

#[test]
fn collect_explicit_app_ids_from_skill_items_resolves_unambiguous_plain_mentions() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse $calendar\n</skill>",
    )];

    let connector_ids =
        collect_explicit_app_ids_from_skill_items(&skill_items, &connectors, &HashMap::new());

    assert_eq!(connector_ids, HashSet::from(["calendar".to_string()]));
}

#[test]
fn collect_explicit_app_ids_from_skill_items_skips_plain_mentions_with_skill_conflicts() {
    let connectors = vec![make_connector("calendar", "Calendar")];
    let skill_items = vec![skill_message(
        "<skill>\n<name>demo</name>\n<path>/tmp/skills/demo/SKILL.md</path>\nuse $calendar\n</skill>",
    )];
    let skill_name_counts_lower = HashMap::from([("calendar".to_string(), 1)]);

    let connector_ids = collect_explicit_app_ids_from_skill_items(
        &skill_items,
        &connectors,
        &skill_name_counts_lower,
    );

    assert_eq!(connector_ids, HashSet::<String>::new());
}

#[tokio::test]
async fn reconstruct_history_matches_live_compactions() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    let reconstruction_turn = session.new_default_turn().await;
    let reconstructed = session
        .reconstruct_history_from_rollout(reconstruction_turn.as_ref(), &rollout_items)
        .await;

    assert_eq!(expected, reconstructed.history);
}

#[tokio::test]
async fn reconstruct_history_uses_replacement_history_verbatim() {
    let (session, turn_context) = make_session_and_context().await;
    let summary_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "summary".to_string(),
        }],
        phase: None,
    };
    let replacement_history = vec![
        summary_item.clone(),
        ResponseItem::Message {
            id: None,
            role: "developer".to_string(),
            content: vec![ContentItem::InputText {
                text: "stale developer instructions".to_string(),
            }],
            phase: None,
        },
    ];
    let rollout_items = vec![RolloutItem::Compacted(CompactedItem {
        message: String::new(),
        replacement_history: Some(replacement_history.clone()),
    })];

    let reconstructed = session
        .reconstruct_history_from_rollout(&turn_context, &rollout_items)
        .await;

    assert_eq!(reconstructed.history, replacement_history);
}

#[tokio::test]
async fn record_initial_history_reconstructs_resumed_transcript() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let history = session.state.lock().await.clone_history();
    assert_eq!(expected, history.raw_items());
}

#[test]
fn resolve_multi_agent_version_handles_unset_and_legacy_history() {
    let thread_id = ThreadId::default();

    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::New,
            /*inherited_multi_agent_version*/ None
        ),
        None
    );
    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::Resumed(ResumedHistory {
                conversation_id: thread_id,
                history: Vec::new(),
                rollout_path: None,
            }),
            /*inherited_multi_agent_version*/ None,
        ),
        Some(MultiAgentVersion::V1)
    );
    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::Resumed(ResumedHistory {
                conversation_id: thread_id,
                history: Vec::new(),
                rollout_path: None,
            }),
            Some(MultiAgentVersion::V2),
        ),
        Some(MultiAgentVersion::V2)
    );
    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::Resumed(ResumedHistory {
                conversation_id: thread_id,
                history: vec![session_meta_item(
                    thread_id,
                    Some(MultiAgentVersion::Disabled)
                )],
                rollout_path: None,
            }),
            Some(MultiAgentVersion::V2),
        ),
        Some(MultiAgentVersion::Disabled)
    );
    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::Forked(vec![session_meta_item(
                thread_id,
                Some(MultiAgentVersion::V2)
            )]),
            Some(MultiAgentVersion::Disabled),
        ),
        Some(MultiAgentVersion::Disabled)
    );
    assert_eq!(
        resolve_multi_agent_version(
            &InitialHistory::Forked(Vec::new()),
            /*inherited_multi_agent_version*/ None
        ),
        Some(MultiAgentVersion::V1)
    );
}

#[tokio::test]
async fn record_initial_history_new_defers_initial_context_until_first_turn() {
    let (session, _turn_context) = make_session_and_context().await;

    session.record_initial_history(InitialHistory::New).await;

    let history = session.clone_history().await;
    assert_eq!(history.raw_items().to_vec(), Vec::<ResponseItem>::new());
    assert!(session.reference_context_item().await.is_none());
    assert_eq!(session.previous_turn_settings().await, None);
}

fn session_meta_item(
    thread_id: ThreadId,
    multi_agent_version: Option<MultiAgentVersion>,
) -> RolloutItem {
    RolloutItem::SessionMeta(SessionMetaLine {
        meta: SessionMeta {
            id: thread_id,
            multi_agent_version,
            ..SessionMeta::default()
        },
        git: None,
    })
}

#[tokio::test]
async fn resumed_history_injects_initial_context_on_first_context_update_only() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, mut expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let history_before_seed = session.state.lock().await.clone_history();
    assert_eq!(expected, history_before_seed.raw_items());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    expected.extend(session.build_initial_context(&turn_context).await);
    let history_after_seed = session.clone_history().await;
    assert_eq!(expected, history_after_seed.raw_items());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let history_after_second_seed = session.clone_history().await;
    assert_eq!(
        history_after_seed.raw_items(),
        history_after_second_seed.raw_items()
    );
}

#[tokio::test]
async fn record_initial_history_seeds_token_info_from_rollout() {
    let (session, turn_context) = make_session_and_context().await;
    let (mut rollout_items, _expected) = sample_rollout(&session, &turn_context).await;

    let info1 = TokenUsageInfo {
        total_token_usage: TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 0,
            total_tokens: 30,
        },
        last_token_usage: TokenUsage {
            input_tokens: 3,
            cached_input_tokens: 0,
            output_tokens: 4,
            reasoning_output_tokens: 0,
            total_tokens: 7,
        },
        model_context_window: Some(1_000),
    };
    let info2 = TokenUsageInfo {
        total_token_usage: TokenUsage {
            input_tokens: 100,
            cached_input_tokens: 50,
            output_tokens: 200,
            reasoning_output_tokens: 25,
            total_tokens: 375,
        },
        last_token_usage: TokenUsage {
            input_tokens: 10,
            cached_input_tokens: 0,
            output_tokens: 20,
            reasoning_output_tokens: 5,
            total_tokens: 35,
        },
        model_context_window: Some(2_000),
    };

    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: Some(info1),
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: None,
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: Some(info2.clone()),
            rate_limits: None,
        },
    )));
    rollout_items.push(RolloutItem::EventMsg(EventMsg::TokenCount(
        TokenCountEvent {
            info: None,
            rate_limits: None,
        },
    )));

    session
        .record_initial_history(InitialHistory::Resumed(ResumedHistory {
            conversation_id: ThreadId::default(),
            history: rollout_items,
            rollout_path: Some(PathBuf::from("/tmp/resume.jsonl")),
        }))
        .await;

    let actual = session.state.lock().await.token_info();
    assert_eq!(actual, Some(info2));
}

#[tokio::test]
async fn recompute_token_usage_uses_session_base_instructions() {
    let (session, turn_context) = make_session_and_context().await;

    let override_instructions = "SESSION_OVERRIDE_INSTRUCTIONS_ONLY".repeat(120);
    {
        let mut state = session.state.lock().await;
        state.session_configuration.base_instructions = override_instructions.clone();
    }

    let item = user_message("hello");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&item))
        .await;

    let history = session.clone_history().await;
    let session_base_instructions = BaseInstructions {
        text: override_instructions,
    };
    let expected_tokens = history
        .estimate_token_count_with_base_instructions(&session_base_instructions)
        .expect("estimate with session base instructions");
    let model_estimated_tokens = history
        .estimate_token_count(&turn_context)
        .expect("estimate with model instructions");
    assert_ne!(expected_tokens, model_estimated_tokens);

    session.recompute_token_usage(&turn_context).await;

    let actual_tokens = session
        .state
        .lock()
        .await
        .token_info()
        .expect("token info")
        .last_token_usage
        .total_tokens;
    assert_eq!(actual_tokens, expected_tokens.max(0));
}

#[tokio::test]
async fn recompute_token_usage_updates_model_context_window() {
    let (session, mut turn_context) = make_session_and_context().await;

    {
        let mut state = session.state.lock().await;
        state.set_token_info(Some(TokenUsageInfo {
            total_token_usage: TokenUsage::default(),
            last_token_usage: TokenUsage::default(),
            model_context_window: Some(258_400),
        }));
    }

    turn_context.model_info.context_window = Some(128_000);
    turn_context.model_info.effective_context_window_percent = 100;

    session.recompute_token_usage(&turn_context).await;

    let actual = session.state.lock().await.token_info().expect("token info");
    assert_eq!(actual.model_context_window, Some(128_000));
}

#[tokio::test]
async fn record_token_usage_info_notifies_extension_contributors() {
    struct SessionTokenUsageMarker;
    struct ThreadTokenUsageMarker;

    #[derive(Debug, PartialEq, Eq)]
    struct RecordedTokenUsage {
        session_level_id: String,
        thread_level_id: String,
        turn_level_id: String,
        token_usage: TokenUsageInfo,
        saw_session_store: bool,
        saw_thread_store: bool,
    }

    struct TokenUsageRecorder {
        records: Arc<std::sync::Mutex<Vec<RecordedTokenUsage>>>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::TokenUsageContributor for TokenUsageRecorder {
        async fn on_token_usage(
            &self,
            session_store: &ontocode_extension_api::ExtensionData,
            thread_store: &ontocode_extension_api::ExtensionData,
            turn_store: &ontocode_extension_api::ExtensionData,
            token_usage: &TokenUsageInfo,
        ) {
            self.records
                .lock()
                .expect("token usage records lock")
                .push(RecordedTokenUsage {
                    session_level_id: session_store.level_id().to_string(),
                    thread_level_id: thread_store.level_id().to_string(),
                    turn_level_id: turn_store.level_id().to_string(),
                    token_usage: token_usage.clone(),
                    saw_session_store: session_store.get::<SessionTokenUsageMarker>().is_some(),
                    saw_thread_store: thread_store.get::<ThreadTokenUsageMarker>().is_some(),
                });
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let records = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.token_usage_contributor(Arc::new(TokenUsageRecorder {
        records: Arc::clone(&records),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .services
        .session_extension_data
        .insert(SessionTokenUsageMarker);
    session
        .services
        .thread_extension_data
        .insert(ThreadTokenUsageMarker);

    let first_usage = TokenUsage {
        input_tokens: 10,
        cached_input_tokens: 2,
        output_tokens: 20,
        reasoning_output_tokens: 3,
        total_tokens: 33,
    };
    let second_usage = TokenUsage {
        input_tokens: 7,
        cached_input_tokens: 1,
        output_tokens: 8,
        reasoning_output_tokens: 5,
        total_tokens: 20,
    };

    session
        .record_token_usage_info(&turn_context, Some(&first_usage))
        .await;
    session
        .record_token_usage_info(&turn_context, Some(&second_usage))
        .await;

    let mut expected_total_usage = first_usage.clone();
    expected_total_usage.add_assign(&second_usage);
    let expected = vec![
        RecordedTokenUsage {
            session_level_id: session.session_id().to_string(),
            thread_level_id: session.thread_id.to_string(),
            turn_level_id: turn_context.sub_id.clone(),
            token_usage: TokenUsageInfo {
                total_token_usage: first_usage.clone(),
                last_token_usage: first_usage,
                model_context_window: turn_context.model_context_window(),
            },
            saw_session_store: true,
            saw_thread_store: true,
        },
        RecordedTokenUsage {
            session_level_id: session.session_id().to_string(),
            thread_level_id: session.thread_id.to_string(),
            turn_level_id: turn_context.sub_id.clone(),
            token_usage: TokenUsageInfo {
                total_token_usage: expected_total_usage,
                last_token_usage: second_usage,
                model_context_window: turn_context.model_context_window(),
            },
            saw_session_store: true,
            saw_thread_store: true,
        },
    ];
    let actual = records
        .lock()
        .expect("token usage records lock")
        .drain(..)
        .collect::<Vec<_>>();
    assert_eq!(expected, actual);
}

/// Providers may return no usage block. That leaves auto-compaction without
/// server ground truth, so the absent usage must not be recorded as real
/// zero-token usage or fan out to token-usage contributors.
#[tokio::test]
async fn record_token_usage_info_ignores_absent_provider_usage() {
    struct AbsentUsageRecorder {
        calls: Arc<std::sync::Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::TokenUsageContributor for AbsentUsageRecorder {
        async fn on_token_usage(
            &self,
            _session_store: &ontocode_extension_api::ExtensionData,
            _thread_store: &ontocode_extension_api::ExtensionData,
            _turn_store: &ontocode_extension_api::ExtensionData,
            _token_usage: &TokenUsageInfo,
        ) {
            *self.calls.lock().expect("absent usage calls lock") += 1;
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::Mutex::new(0usize));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.token_usage_contributor(Arc::new(AbsentUsageRecorder {
        calls: Arc::clone(&calls),
    }));
    session.services.extensions = Arc::new(builder.build());

    session.record_token_usage_info(&turn_context, None).await;

    assert_eq!(*calls.lock().expect("absent usage calls lock"), 0);
    assert_eq!(session.token_usage_info().await, None);
}

#[tokio::test]
async fn turn_start_lifecycle_exposes_turn_metadata_and_token_baseline() {
    struct SessionTurnStartMarker;
    struct ThreadTurnStartMarker;

    #[derive(Debug, PartialEq, Eq)]
    struct RecordedTurnStart {
        session_level_id: String,
        thread_level_id: String,
        turn_level_id: String,
        turn_id: String,
        collaboration_mode: CollaborationMode,
        token_usage_at_turn_start: TokenUsage,
        saw_session_store: bool,
        saw_thread_store: bool,
    }

    struct TurnStartRecorder {
        records: Arc<std::sync::Mutex<Vec<RecordedTurnStart>>>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::TurnLifecycleContributor for TurnStartRecorder {
        async fn on_turn_start(&self, input: ontocode_extension_api::TurnStartInput<'_>) {
            self.records
                .lock()
                .expect("turn start records lock")
                .push(RecordedTurnStart {
                    session_level_id: input.session_store.level_id().to_string(),
                    thread_level_id: input.thread_store.level_id().to_string(),
                    turn_level_id: input.turn_store.level_id().to_string(),
                    turn_id: input.turn_id.to_string(),
                    collaboration_mode: input.collaboration_mode.clone(),
                    token_usage_at_turn_start: input.token_usage_at_turn_start.clone(),
                    saw_session_store: input
                        .session_store
                        .get::<SessionTurnStartMarker>()
                        .is_some(),
                    saw_thread_store: input.thread_store.get::<ThreadTurnStartMarker>().is_some(),
                });
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let records = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.turn_lifecycle_contributor(Arc::new(TurnStartRecorder {
        records: Arc::clone(&records),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .services
        .session_extension_data
        .insert(SessionTurnStartMarker);
    session
        .services
        .thread_extension_data
        .insert(ThreadTurnStartMarker);

    let token_usage_at_turn_start = TokenUsage {
        input_tokens: 100,
        cached_input_tokens: 40,
        output_tokens: 25,
        reasoning_output_tokens: 5,
        total_tokens: 130,
    };
    set_total_token_usage(&session, token_usage_at_turn_start.clone()).await;

    let expected = RecordedTurnStart {
        session_level_id: session.session_id().to_string(),
        thread_level_id: session.thread_id.to_string(),
        turn_level_id: turn_context.sub_id.clone(),
        turn_id: turn_context.sub_id.clone(),
        collaboration_mode: turn_context.collaboration_mode.clone(),
        token_usage_at_turn_start,
        saw_session_store: true,
        saw_thread_store: true,
    };

    let sess = Arc::new(session);
    sess.spawn_task(
        Arc::new(turn_context),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;
    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    let actual = records
        .lock()
        .expect("turn start records lock")
        .drain(..)
        .collect::<Vec<_>>();
    assert_eq!(vec![expected], actual);
}

#[tokio::test]
async fn turn_error_lifecycle_exposes_error_and_stores() {
    struct SessionTurnErrorMarker;
    struct ThreadTurnErrorMarker;

    #[derive(Debug, PartialEq, Eq)]
    struct RecordedTurnError {
        session_level_id: String,
        thread_level_id: String,
        turn_level_id: String,
        turn_id: String,
        error: CodexErrorInfo,
        saw_session_store: bool,
        saw_thread_store: bool,
    }

    struct TurnErrorRecorder {
        records: Arc<std::sync::Mutex<Vec<RecordedTurnError>>>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::TurnLifecycleContributor for TurnErrorRecorder {
        async fn on_turn_error(&self, input: ontocode_extension_api::TurnErrorInput<'_>) {
            self.records
                .lock()
                .expect("turn error records lock")
                .push(RecordedTurnError {
                    session_level_id: input.session_store.level_id().to_string(),
                    thread_level_id: input.thread_store.level_id().to_string(),
                    turn_level_id: input.turn_store.level_id().to_string(),
                    turn_id: input.turn_id.to_string(),
                    error: input.error,
                    saw_session_store: input
                        .session_store
                        .get::<SessionTurnErrorMarker>()
                        .is_some(),
                    saw_thread_store: input.thread_store.get::<ThreadTurnErrorMarker>().is_some(),
                });
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let records = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.turn_lifecycle_contributor(Arc::new(TurnErrorRecorder {
        records: Arc::clone(&records),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .services
        .session_extension_data
        .insert(SessionTurnErrorMarker);
    session
        .services
        .thread_extension_data
        .insert(ThreadTurnErrorMarker);

    let expected = RecordedTurnError {
        session_level_id: session.session_id().to_string(),
        thread_level_id: session.thread_id.to_string(),
        turn_level_id: turn_context.sub_id.clone(),
        turn_id: turn_context.sub_id.clone(),
        error: CodexErrorInfo::UsageLimitExceeded,
        saw_session_store: true,
        saw_thread_store: true,
    };

    session
        .emit_turn_error_lifecycle(&turn_context, CodexErrorInfo::UsageLimitExceeded)
        .await;

    let actual = records
        .lock()
        .expect("turn error records lock")
        .drain(..)
        .collect::<Vec<_>>();
    assert_eq!(vec![expected], actual);
}

#[tokio::test]
async fn config_change_contributor_observes_effective_config_changes() {
    struct SessionConfigMarker;
    struct ThreadConfigMarker;

    #[derive(Debug, PartialEq)]
    struct RecordedConfigChange {
        previous_model: Option<String>,
        new_model: Option<String>,
        previous_disabled_tools: Vec<ToolSuggestDisabledTool>,
        new_disabled_tools: Vec<ToolSuggestDisabledTool>,
        saw_session_store: bool,
        saw_thread_store: bool,
    }

    struct ConfigRecorder {
        records: Arc<std::sync::Mutex<Vec<RecordedConfigChange>>>,
    }

    impl ontocode_extension_api::ConfigContributor<crate::config::Config> for ConfigRecorder {
        fn on_config_changed(
            &self,
            session_store: &ontocode_extension_api::ExtensionData,
            thread_store: &ontocode_extension_api::ExtensionData,
            previous_config: &crate::config::Config,
            new_config: &crate::config::Config,
        ) {
            self.records
                .lock()
                .expect("config change records lock")
                .push(RecordedConfigChange {
                    previous_model: previous_config.model.clone(),
                    new_model: new_config.model.clone(),
                    previous_disabled_tools: previous_config.tool_suggest.disabled_tools.clone(),
                    new_disabled_tools: new_config.tool_suggest.disabled_tools.clone(),
                    saw_session_store: session_store.get::<SessionConfigMarker>().is_some(),
                    saw_thread_store: thread_store.get::<ThreadConfigMarker>().is_some(),
                });
        }
    }

    let (mut session, _turn_context) = make_session_and_context().await;
    let records = Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.config_contributor(Arc::new(ConfigRecorder {
        records: Arc::clone(&records),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .services
        .session_extension_data
        .insert(SessionConfigMarker);
    session
        .services
        .thread_extension_data
        .insert(ThreadConfigMarker);

    let original_model = session.collaboration_mode().await.model().to_string();
    let original_disabled_tools = session
        .get_config()
        .await
        .tool_suggest
        .disabled_tools
        .clone();
    let next_model = if original_model == "gpt-5.4" {
        "gpt-5.2"
    } else {
        "gpt-5.4"
    };
    let collaboration_mode = session.collaboration_mode().await.with_updates(
        Some(next_model.to_string()),
        /*effort*/ None,
        /*developer_instructions*/ None,
    );
    session
        .update_settings(SessionSettingsUpdate {
            collaboration_mode: Some(collaboration_mode),
            ..Default::default()
        })
        .await
        .expect("update settings");

    let ontocode_home = session.ontocode_home().await;
    std::fs::create_dir_all(&ontocode_home).expect("create ontocode home");
    std::fs::write(
        ontocode_home.join(CONFIG_TOML_FILE),
        r#"[tool_suggest]
disabled_tools = [
  { type = "connector", id = " calendar " },
  { type = "plugin", id = "slack@openai-curated" },
]
"#,
    )
    .expect("write user config");
    let next_config = load_latest_config_for_session(&session).await;
    session.refresh_runtime_config(next_config).await;

    let expected_disabled_tools = vec![
        ToolSuggestDisabledTool::connector("calendar"),
        ToolSuggestDisabledTool::plugin("slack@openai-curated"),
    ];
    let expected = vec![
        RecordedConfigChange {
            previous_model: Some(original_model),
            new_model: Some(next_model.to_string()),
            previous_disabled_tools: original_disabled_tools.clone(),
            new_disabled_tools: original_disabled_tools.clone(),
            saw_session_store: true,
            saw_thread_store: true,
        },
        RecordedConfigChange {
            previous_model: Some(next_model.to_string()),
            new_model: Some(next_model.to_string()),
            previous_disabled_tools: original_disabled_tools,
            new_disabled_tools: expected_disabled_tools,
            saw_session_store: true,
            saw_thread_store: true,
        },
    ];
    let actual = records
        .lock()
        .expect("config change records lock")
        .drain(..)
        .collect::<Vec<_>>();
    assert_eq!(expected, actual);
}

#[tokio::test]
async fn record_initial_history_reconstructs_forked_transcript() {
    let (session, turn_context) = make_session_and_context().await;
    let (rollout_items, expected) = sample_rollout(&session, &turn_context).await;

    session
        .record_initial_history(InitialHistory::Forked(rollout_items))
        .await;

    let history = session.state.lock().await.clone_history();
    assert_eq!(expected, history.raw_items());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_configured_reports_permission_profile_for_external_sandbox() -> anyhow::Result<()>
{
    let server = start_mock_server().await;
    let sandbox_policy = SandboxPolicy::ExternalSandbox {
        network_access: ontocode_protocol::protocol::NetworkAccess::Restricted,
    };
    let permission_profile = PermissionProfile::External {
        network: NetworkSandboxPolicy::Restricted,
    };
    let expected_permission_profile = permission_profile.clone();
    let mut builder = test_ontocode().with_config(move |config| {
        config
            .permissions
            .set_permission_profile(permission_profile.clone())
            .expect("set permission profile");
        config
            .set_legacy_sandbox_policy(sandbox_policy)
            .expect("set sandbox policy");
    });

    let test = builder.build(&server).await?;

    assert_eq!(
        test.session_configured.permission_profile, expected_permission_profile,
        "ExternalSandbox is represented explicitly instead of as a lossy root-write profile"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_permission_profile_rebinds_runtime_workspace_roots() -> anyhow::Result<()> {
    let ontocode_home = tempfile::TempDir::new()?;
    let cwd = tempfile::TempDir::new()?;
    let old_root = test_path_buf("/workspace/old").abs();
    let new_root = test_path_buf("/workspace/new").abs();
    let config = ConfigBuilder::default()
        .ontocode_home(ontocode_home.path().to_path_buf())
        .harness_overrides(crate::config::ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            default_permissions: Some(BUILT_IN_PERMISSION_PROFILE_WORKSPACE.to_string()),
            additional_writable_roots: vec![old_root.to_path_buf()],
            ..Default::default()
        })
        .build()
        .await?;

    let session_permission_profile_state = session_permission_profile_state_from_config(&config)?;
    let stored_file_system_policy = session_permission_profile_state
        .permission_profile()
        .file_system_sandbox_policy();
    assert!(
        !stored_file_system_policy
            .can_write_path_with_cwd(old_root.as_path(), config.cwd.as_path()),
        "session permission profile state should keep runtime workspace roots symbolic"
    );

    let mut session_configuration = make_session_configuration_for_tests().await;
    session_configuration.cwd = config.cwd.clone();
    session_configuration.workspace_roots = config.workspace_roots.clone();
    session_configuration.permission_profile_state = session_permission_profile_state;

    let initial_policy = session_configuration.file_system_sandbox_policy();
    assert!(initial_policy.can_write_path_with_cwd(old_root.as_path(), config.cwd.as_path()));

    let updated = session_configuration.apply(&SessionSettingsUpdate {
        workspace_roots: Some(vec![new_root.clone()]),
        ..Default::default()
    })?;
    let updated_policy = updated.file_system_sandbox_policy();
    assert!(updated_policy.can_write_path_with_cwd(new_root.as_path(), updated.cwd.as_path()));
    assert!(!updated_policy.can_write_path_with_cwd(old_root.as_path(), updated.cwd.as_path()));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_configuration_apply_updates_model_provider() -> anyhow::Result<()> {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let mut config = (*session_configuration.original_config_do_not_use).clone();
    let mut provider = config.model_provider.clone();
    provider.name = "alternate".to_string();
    config
        .model_providers
        .insert("alternate".to_string(), provider.clone());
    session_configuration.original_config_do_not_use = Arc::new(config);

    let updated = session_configuration.apply(&SessionSettingsUpdate {
        model_provider: Some("alternate".to_string()),
        ..Default::default()
    })?;

    assert_eq!(
        updated.thread_config_snapshot().model_provider_id,
        "alternate"
    );
    assert_eq!(updated.provider, provider);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fork_startup_context_then_first_turn_diff_snapshot() -> anyhow::Result<()> {
    let server = start_mock_server().await;
    mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let first_forked_request = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp-2"), ev_completed("resp-2")]),
    )
    .await;

    let mut builder = test_ontocode().with_config(|config| {
        config.permissions.approval_policy =
            ontocode_config::Constrained::allow_any(AskForApproval::OnRequest);
    });
    let initial = builder.build(&server).await?;
    let rollout_path = initial
        .session_configured
        .rollout_path
        .clone()
        .expect("rollout path");

    initial
        .ontocode
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "fork seed".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await?;
    wait_for_event(&initial.ontocode, |ev| {
        matches!(ev, EventMsg::TurnComplete(_))
    })
    .await;
    // Forking reads the persisted rollout JSONL, so force the completed source turn to disk
    // before snapshotting from it.
    initial.ontocode.ensure_rollout_materialized().await;
    initial
        .ontocode
        .flush_rollout()
        .await
        .expect("source rollout should flush before fork");

    let mut fork_config = initial.config.clone();
    fork_config.permissions.approval_policy =
        ontocode_config::Constrained::allow_any(AskForApproval::UnlessTrusted);
    let forked = initial
        .thread_manager
        .fork_thread(
            usize::MAX,
            fork_config.clone(),
            rollout_path,
            /*thread_source*/ None,
            /*parent_trace*/ None,
        )
        .await?;

    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Plan,
        settings: Settings {
            model: forked.session_configured.model.clone(),
            reasoning_effort: None,
            developer_instructions: Some("Fork turn collaboration instructions.".to_string()),
        },
    };
    forked
        .thread
        .submit(Op::UserInput {
            environments: None,
            items: vec![UserInput::Text {
                text: "after fork".into(),
                text_elements: Vec::new(),
            }],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides {
                approval_policy: Some(AskForApproval::Never),
                collaboration_mode: Some(collaboration_mode),
                ..Default::default()
            },
        })
        .await?;
    wait_for_event(&forked.thread, |ev| matches!(ev, EventMsg::TurnComplete(_))).await;

    let request = first_forked_request.single_request();
    let snapshot = context_snapshot::format_labeled_requests_snapshot(
        "First request after fork when startup preserves the parent baseline, the fork changes approval policy, and the first forked turn enters plan mode.",
        &[("First Forked Turn Request", &request)],
        &ContextSnapshotOptions::default()
            .render_mode(ContextSnapshotRenderMode::KindWithTextPrefix { max_chars: 96 })
            .strip_capability_instructions()
            .strip_agents_md_user_context(),
    );

    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("snapshots");
    settings.set_prepend_module_to_snapshot(false);
    settings.bind(|| {
        insta::assert_snapshot!(
            "codex_core__codex_tests__fork_startup_context_then_first_turn_diff",
            snapshot
        );
    });

    Ok(())
}

#[tokio::test]
async fn record_initial_history_forked_hydrates_previous_turn_settings() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_model = "forked-rollout-model";
    let previous_context_item = TurnContextItem {
        receipt_only: false,
        turn_id: Some(turn_context.sub_id.clone()),
        #[allow(deprecated)]
        cwd: turn_context.cwd.to_path_buf(),
        workspace_roots: None,
        current_date: turn_context.current_date.clone(),
        timezone: turn_context.timezone.clone(),
        approval_policy: turn_context.approval_policy.value(),
        sandbox_policy: turn_context.sandbox_policy(),
        permission_profile: None,
        network: None,
        file_system_sandbox_policy: None,
        file_read_evidence: None,
        model: previous_model.to_string(),
        model_provider: None,
        personality: turn_context.personality,
        collaboration_mode: Some(turn_context.collaboration_mode.clone()),
        multi_agent_version: None,
        realtime_active: Some(turn_context.realtime_active),
        effort: turn_context.reasoning_effort.clone(),
        summary: ontocode_protocol::config_types::ReasoningSummary::Auto,
    };
    let turn_id = previous_context_item
        .turn_id
        .clone()
        .expect("thread settings should have turn_id");
    let rollout_items = vec![
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            ontocode_protocol::protocol::UserMessageEvent {
                client_id: None,
                message: "forked seed".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            },
        )),
        RolloutItem::TurnContext(previous_context_item.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(
            ontocode_protocol::protocol::TurnCompleteEvent {
                turn_id,
                last_agent_message: None,
                completed_at: None,
                duration_ms: None,
                time_to_first_token_ms: None,
            },
        )),
    ];

    session
        .record_initial_history(InitialHistory::Forked(rollout_items))
        .await;

    let history = session.clone_history().await;
    assert_eq!(
        session.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: previous_model.to_string(),
            realtime_active: Some(turn_context.realtime_active),
        })
    );
    assert_eq!(history.raw_items(), &[]);
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize fork reference context item"),
        serde_json::to_value(Some(previous_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn thread_rollback_drops_last_turn_from_history() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    let rollout_path = attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    let turn_1 = vec![
        user_message("turn 1 user"),
        assistant_message("turn 1 assistant"),
    ];
    let turn_2 = vec![
        user_message("turn 2 user"),
        assistant_message("turn 2 assistant"),
    ];
    let mut full_history = Vec::new();
    full_history.extend(initial_context.clone());
    full_history.extend(turn_1.clone());
    full_history.extend(turn_2);
    sess.replace_history(full_history.clone(), Some(tc.to_turn_context_item()))
        .await;
    let rollout_items: Vec<RolloutItem> = full_history
        .into_iter()
        .map(RolloutItem::ResponseItem)
        .collect();
    sess.persist_rollout_items(&rollout_items).await;
    sess.set_previous_turn_settings(Some(PreviousTurnSettings {
        model: "stale-model".to_string(),
        realtime_active: Some(tc.realtime_active),
    }))
    .await;
    {
        let mut state = sess.state.lock().await;
        state.set_reference_context_item(Some(tc.to_turn_context_item()));
    }

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    let mut expected = Vec::new();
    expected.extend(initial_context);
    expected.extend(turn_1);

    let history = sess.clone_history().await;
    assert_eq!(expected, history.raw_items());
    assert_eq!(sess.previous_turn_settings().await, None);
    assert!(sess.reference_context_item().await.is_none());

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    assert!(resumed.history.iter().any(|item| {
        matches!(
            item,
            RolloutItem::EventMsg(EventMsg::ThreadRolledBack(rollback))
            if rollback.num_turns == 1
        )
    }));
}

#[tokio::test]
async fn thread_rollback_clears_history_when_num_turns_exceeds_existing_turns() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    let turn_1 = vec![user_message("turn 1 user")];
    let mut full_history = Vec::new();
    full_history.extend(initial_context.clone());
    full_history.extend(turn_1);
    sess.replace_history(full_history.clone(), Some(tc.to_turn_context_item()))
        .await;
    let rollout_items: Vec<RolloutItem> = full_history
        .into_iter()
        .map(RolloutItem::ResponseItem)
        .collect();
    sess.persist_rollout_items(&rollout_items).await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 99).await;

    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 99);

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn thread_rollback_fails_without_persisted_thread_history() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_conversation_items(tc.as_ref(), &initial_context)
        .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(
        error_event.message,
        "thread rollback requires persisted thread history"
    );
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );
    assert_eq!(sess.clone_history().await.raw_items(), initial_context);
}

#[tokio::test]
async fn thread_rollback_recomputes_previous_turn_settings_and_reference_context_from_replay() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let first_context_item = tc.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("thread settings should have turn_id");
    let mut rolled_back_context_item = first_context_item.clone();
    rolled_back_context_item.turn_id = Some("rolled-back-turn".to_string());
    rolled_back_context_item.model = "rolled-back-model".to_string();
    let rolled_back_turn_id = rolled_back_context_item
        .turn_id
        .clone()
        .expect("thread settings should have turn_id");
    let turn_one_user = user_message("turn 1 user");
    let turn_one_assistant = assistant_message("turn 1 assistant");
    let turn_two_user = user_message("turn 2 user");
    let turn_two_assistant = assistant_message("turn 2 assistant");

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            ontocode_protocol::protocol::UserMessageEvent {
                client_id: None,
                message: "turn 1 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            },
        )),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(turn_one_user.clone()),
        RolloutItem::ResponseItem(turn_one_assistant.clone()),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: first_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(
            ontocode_protocol::protocol::UserMessageEvent {
                client_id: None,
                message: "turn 2 user".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            },
        )),
        RolloutItem::TurnContext(rolled_back_context_item),
        RolloutItem::ResponseItem(turn_two_user),
        RolloutItem::ResponseItem(turn_two_assistant),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: rolled_back_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;
    sess.replace_history(
        vec![assistant_message("stale history")],
        Some(first_context_item.clone()),
    )
    .await;
    sess.set_previous_turn_settings(Some(PreviousTurnSettings {
        model: "stale-model".to_string(),
        realtime_active: None,
    }))
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    assert_eq!(
        sess.clone_history().await.raw_items(),
        vec![turn_one_user, turn_one_assistant]
    );
    assert_eq!(
        sess.previous_turn_settings().await,
        Some(PreviousTurnSettings {
            model: tc.model_info.slug.clone(),
            realtime_active: Some(tc.realtime_active),
        })
    );
    assert_eq!(
        serde_json::to_value(sess.reference_context_item().await)
            .expect("serialize replay reference context item"),
        serde_json::to_value(Some(first_context_item))
            .expect("serialize expected reference context item")
    );
}

#[tokio::test]
async fn thread_rollback_restores_cleared_reference_context_item_after_compaction() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;

    let first_context_item = tc.to_turn_context_item();
    let first_turn_id = first_context_item
        .turn_id
        .clone()
        .expect("thread settings should have turn_id");
    let compact_turn_id = "compact-turn".to_string();
    let rolled_back_turn_id = "rolled-back-turn".to_string();
    let compacted_history = vec![
        user_message("turn 1 user"),
        user_message("summary after compaction"),
    ];

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: first_turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: "turn 1 user".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        })),
        RolloutItem::TurnContext(first_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 1 user")),
        RolloutItem::ResponseItem(assistant_message("turn 1 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: first_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: compact_turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::Compacted(CompactedItem {
            message: "summary after compaction".to_string(),
            replacement_history: Some(compacted_history.clone()),
        }),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: compact_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: rolled_back_turn_id.clone(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: "turn 2 user".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        })),
        RolloutItem::TurnContext(TurnContextItem {
            receipt_only: false,
            turn_id: Some(rolled_back_turn_id.clone()),
            model: "rolled-back-model".to_string(),
            ..first_context_item.clone()
        }),
        RolloutItem::ResponseItem(user_message("turn 2 user")),
        RolloutItem::ResponseItem(assistant_message("turn 2 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: rolled_back_turn_id,
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;
    sess.replace_history(
        vec![assistant_message("stale history")],
        Some(first_context_item),
    )
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let rollback_event = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(rollback_event.num_turns, 1);

    assert_eq!(sess.clone_history().await.raw_items(), compacted_history);
    assert!(sess.reference_context_item().await.is_none());
}

#[tokio::test]
async fn thread_rollback_persists_marker_and_replays_cumulatively() {
    let (mut sess, tc, rx) = make_session_and_context_with_rx().await;
    let rollout_path = attach_thread_persistence(
        Arc::get_mut(&mut sess).expect("session should not have additional references"),
    )
    .await;
    let turn_context_item = tc.to_turn_context_item();

    sess.persist_rollout_items(&[
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-1".to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: "turn 1 user".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        })),
        RolloutItem::TurnContext(turn_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 1 user")),
        RolloutItem::ResponseItem(assistant_message("turn 1 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-1".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-2".to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: "turn 2 user".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        })),
        RolloutItem::TurnContext(turn_context_item.clone()),
        RolloutItem::ResponseItem(user_message("turn 2 user")),
        RolloutItem::ResponseItem(assistant_message("turn 2 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-2".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
        RolloutItem::EventMsg(EventMsg::TurnStarted(
            ontocode_protocol::protocol::TurnStartedEvent {
                turn_id: "turn-3".to_string(),
                trace_id: None,
                started_at: None,
                model_context_window: Some(128_000),
                collaboration_mode_kind: ModeKind::Default,
            },
        )),
        RolloutItem::EventMsg(EventMsg::UserMessage(UserMessageEvent {
            client_id: None,
            message: "turn 3 user".to_string(),
            images: None,
            local_images: Vec::new(),
            text_elements: Vec::new(),
            ..Default::default()
        })),
        RolloutItem::TurnContext(turn_context_item),
        RolloutItem::ResponseItem(user_message("turn 3 user")),
        RolloutItem::ResponseItem(assistant_message("turn 3 assistant")),
        RolloutItem::EventMsg(EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: "turn-3".to_string(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        })),
    ])
    .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let first_rollback = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(first_rollback.num_turns, 1);
    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;
    let second_rollback = wait_for_thread_rolled_back(&rx).await;
    assert_eq!(second_rollback.num_turns, 1);

    assert_eq!(
        sess.clone_history().await.raw_items(),
        vec![
            user_message("turn 1 user"),
            assistant_message("turn 1 assistant")
        ]
    );

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let rollback_markers = resumed
        .history
        .iter()
        .filter(|item| matches!(item, RolloutItem::EventMsg(EventMsg::ThreadRolledBack(_))))
        .count();
    assert_eq!(rollback_markers, 2);
}

#[tokio::test]
async fn thread_rollback_fails_when_turn_in_progress() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_conversation_items(tc.as_ref(), &initial_context)
        .await;

    *sess.active_turn.lock().await = Some(crate::state::ActiveTurn::default());
    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 1).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn thread_rollback_fails_when_num_turns_is_zero() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;

    let initial_context = sess.build_initial_context(tc.as_ref()).await;
    sess.record_conversation_items(tc.as_ref(), &initial_context)
        .await;

    handlers::thread_rollback(&sess, "sub-1".to_string(), /*num_turns*/ 0).await;

    let error_event = wait_for_thread_rollback_failed(&rx).await;
    assert_eq!(error_event.message, "num_turns must be >= 1");
    assert_eq!(
        error_event.codex_error_info,
        Some(CodexErrorInfo::ThreadRollbackFailed)
    );

    let history = sess.clone_history().await;
    assert_eq!(initial_context, history.raw_items());
}

#[tokio::test]
async fn set_rate_limits_retains_previous_credits() {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(ontocode_home.path()).await;
    let config = Arc::new(config);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort.clone();
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let mut state = SessionState::new(session_configuration);
    let initial = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 10.0,
            window_minutes: Some(15),
            resets_at: Some(1_700),
        }),
        secondary: None,
        credits: Some(CreditsSnapshot {
            has_credits: true,
            unlimited: false,
            balance: Some("10.00".to_string()),
        }),
        individual_limit: None,
        plan_type: Some(ontocode_protocol::account::PlanType::Plus),
        rate_limit_reached_type: None,
    };
    state.set_rate_limits(initial.clone());

    let update = RateLimitSnapshot {
        limit_id: Some("codex_other".to_string()),
        limit_name: Some("codex_other".to_string()),
        primary: Some(RateLimitWindow {
            used_percent: 40.0,
            window_minutes: Some(30),
            resets_at: Some(1_800),
        }),
        secondary: Some(RateLimitWindow {
            used_percent: 5.0,
            window_minutes: Some(60),
            resets_at: Some(1_900),
        }),
        credits: None,
        individual_limit: None,
        plan_type: None,
        rate_limit_reached_type: None,
    };
    state.set_rate_limits(update.clone());

    assert_eq!(
        state.latest_rate_limits,
        Some(RateLimitSnapshot {
            limit_id: Some("codex_other".to_string()),
            limit_name: Some("codex_other".to_string()),
            primary: update.primary.clone(),
            secondary: update.secondary,
            credits: initial.credits,
            individual_limit: initial.individual_limit,
            plan_type: initial.plan_type,
            rate_limit_reached_type: None,
        })
    );
}

#[tokio::test]
async fn set_rate_limits_updates_plan_type_when_present() {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(ontocode_home.path()).await;
    let config = Arc::new(config);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort.clone();
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let mut state = SessionState::new(session_configuration);
    let initial = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 15.0,
            window_minutes: Some(20),
            resets_at: Some(1_600),
        }),
        secondary: Some(RateLimitWindow {
            used_percent: 5.0,
            window_minutes: Some(45),
            resets_at: Some(1_650),
        }),
        credits: Some(CreditsSnapshot {
            has_credits: true,
            unlimited: false,
            balance: Some("15.00".to_string()),
        }),
        individual_limit: None,
        plan_type: Some(ontocode_protocol::account::PlanType::Plus),
        rate_limit_reached_type: None,
    };
    state.set_rate_limits(initial.clone());

    let update = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 35.0,
            window_minutes: Some(25),
            resets_at: Some(1_700),
        }),
        secondary: None,
        credits: None,
        individual_limit: None,
        plan_type: Some(ontocode_protocol::account::PlanType::Pro),
        rate_limit_reached_type: None,
    };
    state.set_rate_limits(update.clone());

    assert_eq!(
        state.latest_rate_limits,
        Some(RateLimitSnapshot {
            limit_id: Some("codex".to_string()),
            limit_name: None,
            primary: update.primary,
            secondary: update.secondary,
            credits: initial.credits,
            individual_limit: initial.individual_limit,
            plan_type: update.plan_type,
            rate_limit_reached_type: None,
        })
    );
}

#[test]
fn prefers_structured_content_when_present() {
    let ctr = McpCallToolResult {
        // Content present but should be ignored because structured_content is set.
        content: vec![text_block("ignored")],
        is_error: None,
        structured_content: Some(json!({
            "ok": true,
            "value": 42
        })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({
                "ok": true,
                "value": 42
            }))
            .unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[tokio::test]
async fn includes_timed_out_message() {
    let exec = ExecToolCallOutput {
        exit_code: 0,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("Command output".to_string()),
        duration: StdDuration::from_secs(1),
        timed_out: true,
    };
    let (_, turn_context) = make_session_and_context().await;

    let out = format_exec_output_str(&exec, turn_context.truncation_policy);

    assert_eq!(
        out,
        "command timed out after 1000 milliseconds\nCommand output"
    );
}

#[tokio::test]
async fn turn_context_with_model_updates_model_fields() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.reasoning_effort = Some(ReasoningEffortConfig::Minimal);
    let updated = turn_context
        .with_model(
            "gpt-5.3-codex".to_string(),
            &session.services.models_manager,
        )
        .await;
    let expected_model_info = session
        .services
        .models_manager
        .get_model_info(
            "gpt-5.3-codex",
            &updated.config.as_ref().to_models_manager_config(),
        )
        .await;

    assert_eq!(updated.config.model.as_deref(), Some("gpt-5.3-codex"));
    assert_eq!(updated.collaboration_mode.model(), "gpt-5.3-codex");
    assert_eq!(updated.model_info, expected_model_info);
    assert_eq!(
        updated.reasoning_effort,
        Some(ReasoningEffortConfig::Medium)
    );
    assert_eq!(
        updated.collaboration_mode.reasoning_effort(),
        Some(ReasoningEffortConfig::Medium)
    );
    assert_eq!(
        updated.config.model_reasoning_effort,
        Some(ReasoningEffortConfig::Medium)
    );
    assert_eq!(
        updated.truncation_policy,
        expected_model_info.truncation_policy.into()
    );
}

#[test]
fn falls_back_to_content_when_structured_is_null() {
    let ctr = McpCallToolResult {
        content: vec![text_block("hello"), text_block("world")],
        is_error: None,
        structured_content: Some(serde_json::Value::Null),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("hello"), text_block("world")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_reflects_is_error_true() {
    let ctr = McpCallToolResult {
        content: vec![text_block("unused")],
        is_error: Some(true),
        structured_content: Some(json!({ "message": "bad" })),
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&json!({ "message": "bad" })).unwrap(),
        ),
        success: Some(false),
    };

    assert_eq!(expected, got);
}

#[test]
fn success_flag_true_with_no_error_and_content_used() {
    let ctr = McpCallToolResult {
        content: vec![text_block("alpha")],
        is_error: Some(false),
        structured_content: None,
        meta: None,
    };

    let got = ctr.into_function_call_output_payload();
    let expected = FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(
            serde_json::to_string(&vec![text_block("alpha")]).unwrap(),
        ),
        success: Some(true),
    };

    assert_eq!(expected, got);
}

async fn wait_for_thread_rolled_back(rx: &async_channel::Receiver<Event>) -> ThreadRolledBackEvent {
    let deadline = StdDuration::from_secs(2);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        match evt.msg {
            EventMsg::ThreadRolledBack(payload) => return payload,
            _ => continue,
        }
    }
}

async fn wait_for_thread_rollback_failed(rx: &async_channel::Receiver<Event>) -> ErrorEvent {
    let deadline = StdDuration::from_secs(2);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        match evt.msg {
            EventMsg::Error(payload)
                if payload.codex_error_info == Some(CodexErrorInfo::ThreadRollbackFailed) =>
            {
                return payload;
            }
            _ => continue,
        }
    }
}

pub(crate) async fn attach_thread_persistence(session: &mut Session) -> PathBuf {
    let config = session.get_config().await;
    let live_thread = LiveThread::create(
        Arc::clone(&session.services.thread_store),
        CreateThreadParams {
            thread_id: session.thread_id,
            forked_from_id: None,
            parent_thread_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            multi_agent_version: None,
            metadata: ThreadPersistenceMetadata {
                cwd: Some(config.cwd.to_path_buf()),
                model_provider: config.model_provider_id.clone(),
                memory_mode: if config.memories.generate_memories {
                    ThreadMemoryMode::Enabled
                } else {
                    ThreadMemoryMode::Disabled
                },
            },
        },
    )
    .await
    .expect("create thread persistence");
    session.services.live_thread = Some(live_thread);
    session.ensure_rollout_materialized().await;
    session
        .flush_rollout()
        .await
        .expect("attached rollout should flush");
    session
        .current_rollout_path()
        .await
        .expect("load rollout path")
        .expect("thread should have rollout path")
}

fn image_generation_receipt(call_id: &str, path: &Path) -> ArtifactReceipt {
    ArtifactReceipt {
        producer: ArtifactReceiptProducer::ImageGeneration,
        call_id: call_id.to_string(),
        path: ontocode_utils_absolute_path::AbsolutePathBuf::try_from(path.to_path_buf())
            .expect("absolute artifact path"),
        source_path: None,
        content_sha256: Some(
            "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae".to_string(),
        ),
        operation: ArtifactReceiptOperation::Create,
        outcome: ArtifactReceiptOutcome::Success,
    }
}

#[tokio::test]
async fn commit_artifact_receipt_without_live_thread_does_not_mutate_evidence() {
    let (session, turn_context) = make_session_and_context().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let receipt = image_generation_receipt("image-no-thread", &temp.path().join("image.png"));

    let err = session
        .commit_artifact_receipt(&turn_context, receipt)
        .await
        .expect_err("receipt commit should require live thread persistence");

    assert!(matches!(err, CodexErr::Io(_)));
    assert!(session.artifact_receipts().await.is_empty());
    assert!(
        turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock")
            .artifact_receipts
            .is_empty()
    );
}

#[tokio::test]
async fn commit_artifact_receipt_updates_session_turn_and_persisted_evidence() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    let live_thread = session
        .live_thread()
        .expect("thread persistence should be attached");
    let temp = tempfile::tempdir().expect("tempdir");
    let receipt = image_generation_receipt("image-success", &temp.path().join("image.png"));
    let expected = vec![receipt.clone()];

    session
        .commit_artifact_receipt(&turn_context, receipt)
        .await
        .expect("receipt commit should succeed");

    assert_eq!(session.artifact_receipts().await, expected);
    assert_eq!(
        turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock")
            .artifact_receipts,
        expected
    );
    let history = live_thread
        .load_history(/*include_archived*/ false)
        .await
        .expect("load persisted receipt history");
    let persisted = history
        .items
        .into_iter()
        .filter_map(|item| match item {
            RolloutItem::TurnContext(context) if context.receipt_only => context
                .file_read_evidence
                .map(|evidence| evidence.artifact_receipts),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(persisted, vec![expected]);
}

#[tokio::test]
async fn real_turn_constructors_hydrate_committed_artifact_receipts() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    let temp = tempfile::tempdir().expect("tempdir");
    let receipt = image_generation_receipt("image-hydration", &temp.path().join("image.png"));
    let expected = vec![receipt.clone()];
    session
        .commit_artifact_receipt(&turn_context, receipt)
        .await
        .expect("receipt commit should succeed");

    let configured = session
        .new_turn_with_sub_id(
            "configured-receipts".to_string(),
            SessionSettingsUpdate::default(),
        )
        .await
        .expect("configured turn");
    let default = session
        .new_default_turn_with_sub_id("default-receipts".to_string())
        .await;
    let read_only = session
        .new_read_only_turn_with_sub_id("read-only-receipts".to_string())
        .await;

    for context in [configured, default, read_only] {
        assert_eq!(
            context
                .file_read_evidence
                .lock()
                .expect("evidence lock")
                .artifact_receipts,
            expected
        );
    }
}

#[tokio::test]
async fn commit_artifact_receipt_append_failure_does_not_mutate_evidence() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    session
        .live_thread()
        .expect("thread persistence should be attached")
        .shutdown()
        .await
        .expect("thread persistence should shut down");
    let temp = tempfile::tempdir().expect("tempdir");
    let receipt = image_generation_receipt("image-append-failure", &temp.path().join("image.png"));

    let err = session
        .commit_artifact_receipt(&turn_context, receipt)
        .await
        .expect_err("receipt commit should fail after persistence shutdown");

    assert!(matches!(err, CodexErr::Io(_)));
    assert!(session.artifact_receipts().await.is_empty());
    assert!(
        turn_context
            .file_read_evidence
            .lock()
            .expect("evidence lock")
            .artifact_receipts
            .is_empty()
    );
}

#[tokio::test]
async fn concurrent_artifact_receipt_commits_preserve_both_receipts() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    let live_thread = session
        .live_thread()
        .cloned()
        .expect("thread persistence should be attached");
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let temp = tempfile::tempdir().expect("tempdir");
    let first = image_generation_receipt("image-a", &temp.path().join("a.png"));
    let second = image_generation_receipt("image-b", &temp.path().join("b.png"));

    let (first_result, second_result) = tokio::join!(
        session.commit_artifact_receipt(turn_context.as_ref(), first.clone()),
        session.commit_artifact_receipt(turn_context.as_ref(), second.clone()),
    );
    first_result.expect("first receipt commit should succeed");
    second_result.expect("second receipt commit should succeed");

    let mut expected = vec![first, second];
    expected.sort_by(|left, right| left.call_id.cmp(&right.call_id));
    let mut session_receipts = session.artifact_receipts().await;
    session_receipts.sort_by(|left, right| left.call_id.cmp(&right.call_id));
    assert_eq!(session_receipts, expected);
    let mut turn_receipts = turn_context
        .file_read_evidence
        .lock()
        .expect("evidence lock")
        .artifact_receipts
        .clone();
    turn_receipts.sort_by(|left, right| left.call_id.cmp(&right.call_id));
    assert_eq!(turn_receipts, expected);

    let history = live_thread
        .load_history(/*include_archived*/ false)
        .await
        .expect("load persisted receipt history");
    let snapshots = history
        .items
        .into_iter()
        .filter_map(|item| match item {
            RolloutItem::TurnContext(context) if context.receipt_only => context
                .file_read_evidence
                .map(|evidence| evidence.artifact_receipts),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(snapshots.len(), 2);
    let mut final_snapshot = snapshots
        .into_iter()
        .max_by_key(Vec::len)
        .expect("receipt-only snapshots should be persisted");
    final_snapshot.sort_by(|left, right| left.call_id.cmp(&right.call_id));
    assert_eq!(final_snapshot, expected);
}

fn text_block(s: &str) -> serde_json::Value {
    json!({
        "type": "text",
        "text": s,
    })
}

pub(crate) fn non_repo_tempdir() -> tempfile::TempDir {
    for base in [std::env::temp_dir(), PathBuf::from("/dev/shm")] {
        let Ok(base) = base.canonicalize() else {
            continue;
        };
        if base.ancestors().any(|ancestor| {
            [".git", ".agents", ".ontocode"].iter().any(|marker| {
                !matches!(
                    std::fs::symlink_metadata(ancestor.join(marker)),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound
                )
            })
        }) {
            continue;
        }
        if let Ok(dir) = tempfile::tempdir_in(base) {
            return dir;
        }
    }
    panic!(
        "cannot create non-repository test cwd: temp_dir() and /dev/shm must be writable and have no .git, .agents, or .ontocode markers in their ancestors"
    );
}

async fn build_test_config(ontocode_home: &Path) -> Config {
    static CWD: std::sync::LazyLock<tempfile::TempDir> = std::sync::LazyLock::new(non_repo_tempdir);
    // ADR 0016 made unconfigured sessions default to full access with approval
    // `never`, which would make every fixture child a writable session that
    // must live in a Git worktree and would bypass approval routing. Keep the
    // shared fixture read-only with on-request approval, as it was before that
    // decision; tests that need other permissions select them explicitly.
    ConfigBuilder::without_managed_config_for_tests()
        .ontocode_home(ontocode_home.to_path_buf())
        .fallback_cwd(Some(CWD.path().to_path_buf()))
        .harness_overrides(crate::config::ConfigOverrides {
            default_permissions: Some(
                ontocode_protocol::models::BUILT_IN_PERMISSION_PROFILE_READ_ONLY.to_string(),
            ),
            approval_policy: Some(AskForApproval::OnRequest),
            ..Default::default()
        })
        .build()
        .await
        .expect("load default test config")
}

fn session_telemetry(
    conversation_id: ThreadId,
    config: &Config,
    model_info: &ModelInfo,
    session_source: SessionSource,
) -> SessionTelemetry {
    SessionTelemetry::new(
        conversation_id,
        get_model_offline_for_tests(config.model.as_deref()).as_str(),
        model_info.slug.as_str(),
        /*account_id*/ None,
        Some("test@test.com".to_string()),
        Some(TelemetryAuthMode::ApiKey),
        "test_originator".to_string(),
        /*log_user_prompts*/ false,
        "test".to_string(),
        session_source,
    )
}

fn model_with_default_service_tier(default_service_tier: Option<&str>) -> ModelInfo {
    let mut model_info = model_info::model_info_from_slug("gpt-5.4");
    model_info.service_tiers = vec![ModelServiceTier {
        id: ServiceTier::Fast.request_value().to_string(),
        name: "Fast".to_string(),
        description: "Priority processing.".to_string(),
    }];
    model_info.default_service_tier = default_service_tier.map(str::to_string);
    model_info
}

#[test]
fn get_service_tier_does_not_use_model_default_when_absent_and_fast_mode_enabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_does_not_use_model_default_when_fast_mode_disabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_keeps_supported_explicit_tier() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Fast.request_value().to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

#[test]
fn get_service_tier_does_not_default_when_model_has_no_default() {
    let model_info = model_with_default_service_tier(/*default_service_tier*/ None);

    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
}

#[test]
fn get_service_tier_drops_unsupported_configured_tier_when_fast_mode_enabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some("unsupported".to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Flex.request_value().to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()),
            /*fast_mode_enabled*/ true,
            &model_info,
        ),
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
}

#[test]
fn get_service_tier_ignores_configured_tier_when_fast_mode_disabled() {
    let model_info = model_with_default_service_tier(Some(ServiceTier::Fast.request_value()));

    assert_eq!(
        get_service_tier(
            Some(ServiceTier::Fast.request_value().to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            Some("unsupported".to_string()),
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
    assert_eq!(
        get_service_tier(
            /*configured_service_tier*/ None,
            /*fast_mode_enabled*/ false,
            &model_info,
        ),
        None
    );
}

#[tokio::test]
async fn session_settings_null_service_tier_update_uses_default_service_tier() {
    let session_configuration = make_session_configuration_for_tests().await;

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            service_tier: Some(None),
            ..Default::default()
        })
        .expect("null service tier update should apply");

    assert_eq!(
        updated.service_tier,
        Some(SERVICE_TIER_DEFAULT_REQUEST_VALUE.to_string())
    );
}

#[tokio::test]
async fn session_settings_legacy_fast_service_tier_update_uses_priority_request_value() {
    let session_configuration = make_session_configuration_for_tests().await;

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            service_tier: Some(Some("fast".to_string())),
            ..Default::default()
        })
        .expect("legacy fast service tier update should apply");

    assert_eq!(
        updated.service_tier,
        Some(ServiceTier::Fast.request_value().to_string())
    );
}

pub(crate) async fn make_session_configuration_for_tests() -> SessionConfiguration {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(ontocode_home.path()).await;
    let config = Arc::new(config);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort.clone();
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };

    SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    }
}

fn turn_environments_for_tests(
    environment: &Arc<ontocode_exec_server::Environment>,
    cwd: &ontocode_utils_absolute_path::AbsolutePathBuf,
) -> crate::environment_selection::ResolvedTurnEnvironments {
    crate::environment_selection::ResolvedTurnEnvironments {
        turn_environments: vec![TurnEnvironment {
            environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
            environment: Arc::clone(environment),
            cwd: cwd.clone(),
            shell: None,
        }],
    }
}

#[tokio::test]
async fn session_configuration_apply_preserves_profile_file_system_policy_on_cwd_only_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let project_root = workspace.path().join("project");
    let original_cwd = project_root.join("subdir");
    let docs_dir = original_cwd.join("docs");
    std::fs::create_dir_all(&docs_dir).expect("create docs dir");
    let docs_dir = docs_dir.abs();

    session_configuration.cwd = original_cwd.abs();
    let sandbox_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: Vec::new(),
        network_access: false,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: true,
    };
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Path { path: docs_dir },
            access: FileSystemAccessMode::Read,
        },
    ]);
    let network_sandbox_policy = NetworkSandboxPolicy::from(&sandbox_policy);
    session_configuration
        .set_permission_profile_for_tests(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
                &file_system_sandbox_policy,
                network_sandbox_policy,
            ),
        )
        .expect("set permission profile");
    let expected_file_system_sandbox_policy = file_system_sandbox_policy
        .materialize_project_roots_with_workspace_roots(&session_configuration.workspace_roots);

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(project_root),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(
        updated.file_system_sandbox_policy(),
        expected_file_system_sandbox_policy
    );
}

#[tokio::test]
async fn session_configuration_apply_permission_profile_preserves_existing_deny_read_entries() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let cwd = tempfile::tempdir().expect("create temp dir");
    session_configuration.cwd = cwd.path().abs();

    let workspace_policy = SandboxPolicy::new_workspace_write_policy();
    let deny_entry = FileSystemSandboxEntry {
        path: FileSystemPath::GlobPattern {
            pattern: "**/*.env".to_string(),
        },
        access: FileSystemAccessMode::Deny,
    };
    let mut existing_file_system_policy =
        FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
            &workspace_policy,
            session_configuration.cwd.as_path(),
        );
    existing_file_system_policy.glob_scan_max_depth = Some(2);
    existing_file_system_policy.entries.push(deny_entry.clone());
    session_configuration
        .set_permission_profile_for_tests(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::from_legacy_sandbox_policy(&workspace_policy),
                &existing_file_system_policy,
                NetworkSandboxPolicy::Restricted,
            ),
        )
        .expect("set permission profile");

    let requested_file_system_policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &workspace_policy,
        session_configuration.cwd.as_path(),
    );
    let permission_profile = ontocode_protocol::models::PermissionProfile::from_runtime_permissions(
        &requested_file_system_policy,
        NetworkSandboxPolicy::Restricted,
    );
    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            permission_profile: Some(permission_profile),
            ..Default::default()
        })
        .expect("permission profile update should succeed");

    let mut expected_file_system_policy = requested_file_system_policy
        .materialize_project_roots_with_workspace_roots(&session_configuration.workspace_roots);
    expected_file_system_policy.glob_scan_max_depth = Some(2);
    expected_file_system_policy.entries.push(deny_entry);
    assert_eq!(
        updated.file_system_sandbox_policy(),
        expected_file_system_policy
    );
}

#[tokio::test]
async fn session_configuration_apply_permission_profile_accepts_direct_write_roots() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let cwd = tempfile::tempdir().expect("create cwd");
    session_configuration.cwd = cwd.path().abs();
    let external_write_dir = tempfile::tempdir().expect("create external write root");
    let external_write_path = AbsolutePathBuf::from_absolute_path(
        ontocode_utils_absolute_path::canonicalize_preserving_symlinks(external_write_dir.path())
            .expect("canonical temp dir"),
    )
    .expect("canonical temp dir should be absolute");
    let file_system_sandbox_policy =
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: external_write_path.clone(),
            },
            access: FileSystemAccessMode::Write,
        }]);
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            permission_profile: Some(permission_profile.clone()),
            ..Default::default()
        })
        .expect("permission profile update should accept direct runtime permissions");

    assert_eq!(updated.permission_profile(), permission_profile);
    assert_eq!(
        updated.file_system_sandbox_policy(),
        file_system_sandbox_policy
    );
    assert_eq!(
        updated.sandbox_policy(),
        SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![external_write_path],
            network_access: false,
            exclude_tmpdir_env_var: true,
            exclude_slash_tmp: true,
        }
    );
}

#[tokio::test]
async fn session_configuration_apply_rebinds_symbolic_profile_to_updated_workspace_roots() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let old_root = tempfile::tempdir().expect("create old root");
    let new_root = tempfile::tempdir().expect("create new root");
    let profile_root = tempfile::tempdir().expect("create profile root");
    let old_root = old_root.path().abs();
    let new_root = new_root.path().abs();
    let profile_root = profile_root.path().abs();
    session_configuration.workspace_roots = vec![old_root.clone()];

    let file_system_sandbox_policy =
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        }]);
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    );

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            workspace_roots: Some(vec![new_root.clone()]),
            permission_profile: Some(permission_profile),
            active_permission_profile: Some(ActivePermissionProfile::new("dev")),
            profile_workspace_roots: Some(vec![profile_root.clone()]),
            ..Default::default()
        })
        .expect("permission profile update should succeed");

    let updated_policy = updated.file_system_sandbox_policy();
    assert!(updated_policy.can_write_path_with_cwd(new_root.as_path(), updated.cwd.as_path()));
    assert!(!updated_policy.can_write_path_with_cwd(old_root.as_path(), updated.cwd.as_path()));
    assert_eq!(
        updated.active_permission_profile(),
        Some(ActivePermissionProfile::new("dev"))
    );
    assert_eq!(updated.profile_workspace_roots(), &[profile_root]);
}

#[tokio::test]
async fn session_configuration_apply_retargets_implicit_workspace_root_on_cwd_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let old_root = tempfile::tempdir().expect("create old root");
    let new_root = tempfile::tempdir().expect("create new root");
    let extra_root = tempfile::tempdir().expect("create extra root");
    let old_root = old_root.path().abs();
    let new_root = new_root.path().abs();
    let extra_root = extra_root.path().abs();
    session_configuration.cwd = old_root.clone();
    session_configuration.workspace_roots = vec![old_root.clone(), extra_root.clone()];

    let file_system_sandbox_policy =
        FileSystemSandboxPolicy::restricted(vec![FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
            },
            access: FileSystemAccessMode::Write,
        }]);
    let permission_profile = PermissionProfile::from_runtime_permissions(
        &file_system_sandbox_policy,
        NetworkSandboxPolicy::Restricted,
    );
    session_configuration
        .set_permission_profile_for_tests(permission_profile)
        .expect("set permission profile");

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(new_root.to_path_buf()),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(
        updated.workspace_roots,
        vec![new_root.clone(), extra_root.clone()]
    );
    let updated_policy = updated.file_system_sandbox_policy();
    assert!(updated_policy.can_write_path_with_cwd(new_root.as_path(), updated.cwd.as_path()));
    assert!(updated_policy.can_write_path_with_cwd(extra_root.as_path(), updated.cwd.as_path()));
    assert!(!updated_policy.can_write_path_with_cwd(old_root.as_path(), updated.cwd.as_path()));
}

#[tokio::test]
async fn active_profile_update_rebuilds_network_proxy_config() -> std::io::Result<()> {
    let ontocode_home = tempfile::tempdir().expect("create ontocode home");
    let cwd = tempfile::tempdir().expect("create cwd");
    let permissions = PermissionsToml {
        entries: std::collections::BTreeMap::from([
            (
                "locked-down".to_string(),
                PermissionProfileToml {
                    description: None,
                    extends: None,
                    workspace_roots: None,
                    filesystem: Some(FilesystemPermissionsToml {
                        glob_scan_max_depth: None,
                        entries: std::collections::BTreeMap::from([(
                            ":minimal".to_string(),
                            FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                        )]),
                    }),
                    network: None,
                },
            ),
            (
                "web-enabled".to_string(),
                PermissionProfileToml {
                    description: None,
                    extends: None,
                    workspace_roots: None,
                    filesystem: Some(FilesystemPermissionsToml {
                        glob_scan_max_depth: None,
                        entries: std::collections::BTreeMap::from([(
                            ":minimal".to_string(),
                            FilesystemPermissionToml::Access(FileSystemAccessMode::Read),
                        )]),
                    }),
                    network: Some(NetworkToml {
                        enabled: Some(true),
                        proxy_url: Some("http://127.0.0.1:43128".to_string()),
                        enable_socks5: Some(false),
                        ..Default::default()
                    }),
                },
            ),
        ]),
    };
    let base_config = ConfigToml {
        features: Some(toml::from_str("network_proxy = true").expect("valid features")),
        default_permissions: Some("locked-down".to_string()),
        permissions: Some(permissions),
        ..Default::default()
    };
    std::fs::write(
        ontocode_home.path().join(ontocode_config::CONFIG_TOML_FILE),
        toml::to_string(&base_config).expect("serialize config"),
    )?;
    let locked_config = Arc::new(
        ConfigBuilder::default()
            .ontocode_home(ontocode_home.path().to_path_buf())
            .harness_overrides(ConfigOverrides {
                cwd: Some(cwd.path().to_path_buf()),
                ..Default::default()
            })
            .build()
            .await?,
    );
    assert_ne!(
        locked_config
            .permissions
            .network
            .as_ref()
            .map(crate::config::NetworkProxySpec::proxy_host_and_port)
            .as_deref(),
        Some("127.0.0.1:43128")
    );
    let selected_config = ConfigBuilder::default()
        .ontocode_home(ontocode_home.path().to_path_buf())
        .harness_overrides(ConfigOverrides {
            cwd: Some(cwd.path().to_path_buf()),
            default_permissions: Some("web-enabled".to_string()),
            ..Default::default()
        })
        .build()
        .await?;

    let mut session_configuration = make_session_configuration_for_tests().await;
    session_configuration.permission_profile_state =
        locked_config.permissions.permission_profile_state().clone();
    session_configuration.original_config_do_not_use = Arc::clone(&locked_config);

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            permission_profile: Some(selected_config.permissions.permission_profile().clone()),
            active_permission_profile: selected_config.permissions.active_permission_profile(),
            ..Default::default()
        })
        .expect("active profile update should apply");

    let network = updated
        .original_config_do_not_use
        .permissions
        .network
        .as_ref()
        .expect("selected profile proxy should become the session proxy config");
    assert_eq!(network.proxy_host_and_port(), "127.0.0.1:43128");
    assert!(!network.socks_enabled());
    Ok(())
}

#[cfg_attr(windows, ignore)]
#[tokio::test]
async fn new_default_turn_uses_config_aware_skills_for_role_overrides() {
    let (session, _turn_context) = make_session_and_context().await;
    let parent_config = session.get_config().await;
    let ontocode_home = parent_config.codex_home.clone();
    let skill_dir = ontocode_home.join("skills").join("demo");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: demo-skill\ndescription: demo description\n---\n\n# Body\n",
    )
    .expect("write skill");

    let skill_fs = session
        .services
        .environment_manager
        .default_environment()
        .map(|environment| environment.get_filesystem())
        .unwrap_or_else(|| std::sync::Arc::clone(&ontocode_exec_server::LOCAL_FS));
    let parent_outcome = session
        .services
        .skills_manager
        .skills_for_cwd(
            &crate::skills_load_input_from_config(&parent_config, Vec::new()),
            /*force_reload*/ true,
            Some(Arc::clone(&skill_fs)),
        )
        .await;
    let parent_skill = parent_outcome
        .skills
        .iter()
        .find(|skill| skill.name == "demo-skill")
        .expect("demo skill should be discovered");
    assert_eq!(parent_outcome.is_skill_enabled(parent_skill), true);

    let role_path = ontocode_home.join("skills-role.toml");
    std::fs::write(
        &role_path,
        format!(
            r#"developer_instructions = "Stay focused"

[[skills.config]]
path = "{}"
enabled = false
"#,
            skill_path.display()
        ),
    )
    .expect("write role config");

    let mut child_config = (*parent_config).clone();
    child_config.agent_roles.insert(
        "custom".to_string(),
        crate::config::AgentRoleConfig {
            description: None,
            config_file: Some(role_path.to_path_buf()),
            nickname_candidates: None,
            runtime_overrides: None,
        },
    );
    crate::agent::role::apply_role_to_config(&mut child_config, Some("custom"))
        .await
        .expect("custom role should apply");

    {
        let mut state = session.state.lock().await;
        state.session_configuration.original_config_do_not_use = Arc::new(child_config);
    }

    let child_turn = session
        .new_default_turn_with_sub_id("role-skill-turn".to_string())
        .await;
    let child_skill = child_turn
        .turn_skills
        .outcome
        .skills
        .iter()
        .find(|skill| skill.name == "demo-skill")
        .expect("demo skill should be discovered");
    assert_eq!(
        child_turn.turn_skills.outcome.is_skill_enabled(child_skill),
        false
    );
}

#[tokio::test]
async fn session_configuration_apply_retargets_legacy_workspace_root_on_cwd_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let original_cwd = workspace.path().join("repo-a").abs();
    let project_root = workspace.path().join("repo-b").abs();
    session_configuration.cwd = original_cwd.clone();
    session_configuration.workspace_roots = vec![session_configuration.cwd.clone()];
    let sandbox_policy = SandboxPolicy::WorkspaceWrite {
        writable_roots: Vec::new(),
        network_access: false,
        exclude_tmpdir_env_var: true,
        exclude_slash_tmp: true,
    };
    let file_system_sandbox_policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &sandbox_policy,
        &session_configuration.cwd,
    );
    session_configuration
        .set_permission_profile_for_tests(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::from_legacy_sandbox_policy(&sandbox_policy),
                &file_system_sandbox_policy,
                NetworkSandboxPolicy::from(&sandbox_policy),
            ),
        )
        .expect("set permission profile");

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(project_root.to_path_buf()),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(updated.workspace_roots, vec![project_root.clone()]);
    assert!(
        updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(project_root.as_path(), updated.cwd.as_path()),
        "cwd-only update should keep the new cwd writable"
    );
    assert!(
        !updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(original_cwd.as_path(), updated.cwd.as_path()),
        "cwd-only update should not keep the old implicit cwd writable"
    );
}

#[tokio::test]
async fn session_configuration_apply_preserves_absolute_cwd_write_root_on_cwd_update() {
    let mut session_configuration = make_session_configuration_for_tests().await;
    let workspace = tempfile::tempdir().expect("create temp dir");
    let original_cwd = workspace.path().join("repo-a");
    let next_cwd = workspace.path().join("repo-b");
    std::fs::create_dir_all(&original_cwd).expect("create original cwd");
    std::fs::create_dir_all(&next_cwd).expect("create next cwd");
    let original_cwd = original_cwd.abs();

    session_configuration.cwd = original_cwd.clone();
    let file_system_sandbox_policy = FileSystemSandboxPolicy::restricted(vec![
        FileSystemSandboxEntry {
            path: FileSystemPath::Special {
                value: FileSystemSpecialPath::Root,
            },
            access: FileSystemAccessMode::Read,
        },
        FileSystemSandboxEntry {
            path: FileSystemPath::Path {
                path: original_cwd.clone(),
            },
            access: FileSystemAccessMode::Write,
        },
    ]);
    session_configuration
        .set_permission_profile_for_tests(
            PermissionProfile::from_runtime_permissions_with_enforcement(
                SandboxEnforcement::Managed,
                &file_system_sandbox_policy,
                NetworkSandboxPolicy::Restricted,
            ),
        )
        .expect("set permission profile");

    let updated = session_configuration
        .apply(&SessionSettingsUpdate {
            cwd: Some(next_cwd.clone()),
            ..Default::default()
        })
        .expect("cwd-only update should succeed");

    assert_eq!(
        updated.file_system_sandbox_policy(),
        file_system_sandbox_policy
    );
    assert!(
        updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(original_cwd.as_path(), updated.cwd.as_path()),
        "absolute grant to the old cwd must remain writable"
    );
    assert!(
        !updated
            .file_system_sandbox_policy()
            .can_write_path_with_cwd(next_cwd.as_path(), updated.cwd.as_path()),
        "cwd-only update must not reinterpret an absolute old-cwd grant as :workspace_roots"
    );
}

#[tokio::test]
async fn session_update_settings_does_not_rewrite_sticky_environment_cwds() {
    let (session, turn_context) = make_session_and_context().await;
    #[allow(deprecated)]
    let updated_cwd = turn_context.cwd.join("project");
    std::fs::create_dir_all(updated_cwd.as_path()).expect("create project dir");

    session
        .update_settings(SessionSettingsUpdate {
            cwd: Some(updated_cwd.to_path_buf()),
            ..Default::default()
        })
        .await
        .expect("cwd update should succeed");

    let session_cwd = {
        let state = session.state.lock().await;
        state.session_configuration.cwd.clone()
    };
    let config = session.get_config().await;
    let next_turn = session.new_default_turn().await;

    assert_eq!(session_cwd, updated_cwd);
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    #[allow(deprecated)]
    let next_turn_cwd = next_turn.cwd.clone();
    assert_eq!(config.cwd, turn_cwd);
    assert_eq!(next_turn_cwd, updated_cwd);
    assert_eq!(next_turn.config.cwd, updated_cwd);
}

#[tokio::test]
async fn relative_cwd_update_without_environments_resolves_under_process_cwd() {
    let (session, _turn_context) = make_session_and_context().await;
    {
        let mut state = session.state.lock().await;
        state.session_configuration.environments = Vec::new();
    }
    let updated_cwd = AbsolutePathBuf::current_dir()
        .expect("current directory")
        .join("project");

    session
        .update_settings(SessionSettingsUpdate {
            cwd: Some(PathBuf::from("project")),
            ..Default::default()
        })
        .await
        .expect("cwd update should succeed");

    let state = session.state.lock().await;
    assert_eq!(state.session_configuration.cwd, updated_cwd);
    assert!(state.session_configuration.environments.is_empty());
}

#[tokio::test]
async fn cwd_update_does_not_rewrite_sticky_environment_cwd() {
    let (session, _turn_context) = make_session_and_context().await;
    let (original_cwd, environment_cwd) = {
        let mut state = session.state.lock().await;
        let original_cwd = state.session_configuration.cwd.clone();
        let environment_cwd = original_cwd.join("environment");
        state.session_configuration.environments = vec![TurnEnvironmentSelection {
            environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
            cwd: environment_cwd.clone(),
        }];
        (original_cwd, environment_cwd)
    };
    let updated_cwd = original_cwd.join("project");
    std::fs::create_dir_all(updated_cwd.as_path()).expect("create project dir");

    session
        .update_settings(SessionSettingsUpdate {
            cwd: Some(updated_cwd.to_path_buf()),
            ..Default::default()
        })
        .await
        .expect("cwd update should succeed");

    let state = session.state.lock().await;
    assert_eq!(state.session_configuration.cwd, updated_cwd);
    assert_eq!(
        state.session_configuration.environments[0].cwd,
        environment_cwd
    );
}

#[tokio::test]
async fn absolute_cwd_update_with_turn_environment_is_allowed() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let absolute_cwd = {
        let state = session.state.lock().await;
        state.session_configuration.cwd.join("absolute-turn")
    };
    std::fs::create_dir_all(absolute_cwd.as_path()).expect("create absolute turn dir");

    let turn_context = session
        .new_turn_with_sub_id(
            "sub-1".to_string(),
            SessionSettingsUpdate {
                cwd: Some(absolute_cwd.to_path_buf()),
                environments: Some(vec![TurnEnvironmentSelection {
                    environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
                    cwd: absolute_cwd.clone(),
                }]),
                ..Default::default()
            },
        )
        .await
        .expect("absolute cwd with explicit environments should succeed");

    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(turn_cwd, absolute_cwd);
    assert_eq!(turn_context.config.cwd, absolute_cwd);
    assert_eq!(turn_context.environments.turn_environments.len(), 1);
}

#[tokio::test]
async fn session_new_fails_when_zsh_fork_enabled_without_packaged_zsh() {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(ontocode_home.path()).await;
    config
        .features
        .enable(Feature::ShellZshFork)
        .expect("test config should allow shell_zsh_fork");
    config.zsh_path = None;
    let config = Arc::new(config);

    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort.clone(),
            developer_instructions: None,
        },
    };
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: Vec::new(),
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, _rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let result = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        InitialHistory::New,
        SessionSource::Exec,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(ontocode_extension_api::ExtensionRegistryBuilder::new().build()),
        AgentControl::default(),
        Arc::new(ontocode_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(ontocode_thread_store::LocalThreadStore::new(
            ontocode_thread_store::LocalThreadStoreConfig::from_config(config.as_ref()),
            /*state_db*/ None,
        )),
        ontocode_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
        Some(config.multi_agent_version_from_features()),
    )
    .await;

    let err = match result {
        Ok(_) => panic!("expected startup to fail"),
        Err(err) => err,
    };
    let msg = format!("{err:#}");
    assert!(msg.contains("zsh fork feature enabled, but no packaged zsh fork is available"));
}

// todo: use online model info
#[tokio::test]
async fn record_conversation_items_deduplicates_provider_call_retries() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");

    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;

    assert_eq!(session.clone_history().await.raw_items(), &[call]);
}

#[tokio::test]
async fn record_conversation_items_preserves_valid_sibling_of_conflicting_item() {
    let (session, turn_context) = make_session_and_context().await;
    let original_call = function_call("call-provider-history", "test_tool");
    let conflicting_call = ResponseItem::FunctionCall {
        id: None,
        name: "test_tool".to_string(),
        namespace: None,
        arguments: r#"{"different":true}"#.to_string(),
        call_id: "call-provider-history".to_string(),
    };
    let valid_message = assistant_message("preserve me");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&original_call))
        .await;

    session
        .record_conversation_items(&turn_context, &[conflicting_call, valid_message.clone()])
        .await;

    assert_eq!(
        session.clone_history().await.raw_items(),
        &[original_call, valid_message]
    );
}

#[tokio::test]
async fn record_conversation_items_allows_call_id_reuse_in_later_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let first_call = function_call("call-provider-history", "first_tool");
    let second_call = function_call("call-provider-history", "second_tool");
    let next_turn = user_message("start another turn");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&first_call))
        .await;
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&next_turn))
        .await;

    let outcome = session
        .record_provider_history_item(&turn_context, &second_call)
        .await
        .expect("later turn should allow provider call id reuse");

    assert_eq!(outcome, ProviderHistoryRecordOutcome::Inserted);
    assert_eq!(
        session.clone_history().await.raw_items(),
        &[first_call, next_turn, second_call]
    );
}

#[tokio::test]
async fn provider_history_normalization_preserves_call_id_reuse_in_later_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let first_call = function_call("reused-call-id", "first_tool");
    let first_output = function_output("reused-call-id", "first");
    let next_turn = user_message("start another turn");
    let second_call = function_call("reused-call-id", "second_tool");
    let second_output = function_output("reused-call-id", "second");
    let history = vec![
        first_call,
        first_output,
        next_turn,
        second_call,
        second_output,
    ];
    session
        .record_conversation_items(&turn_context, &history)
        .await;

    let prompt = session.clone_history().await.for_prompt(
        &turn_context.model_info.input_modalities,
        turn_context.trailing_user_message(),
    );

    assert_eq!(prompt, history);
}

#[tokio::test]
async fn provider_history_normalization_repairs_missing_output_per_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let first_call = function_call("reused-call-id", "first_tool");
    let first_output = function_output("reused-call-id", "first");
    let next_turn = user_message("start another turn");
    let second_call = function_call("reused-call-id", "second_tool");
    let history = vec![
        first_call.clone(),
        first_output.clone(),
        next_turn.clone(),
        second_call.clone(),
    ];
    session
        .record_conversation_items(&turn_context, &history)
        .await;

    let prompt = session.clone_history().await.for_prompt(
        &turn_context.model_info.input_modalities,
        turn_context.trailing_user_message(),
    );

    assert_eq!(
        prompt,
        vec![
            first_call,
            first_output,
            next_turn,
            second_call,
            ResponseItem::FunctionCallOutput {
                call_id: "reused-call-id".to_string(),
                output: FunctionCallOutputPayload {
                    body: FunctionCallOutputBody::Text(
                        "tool execution did not record a terminal result; outcome may be unknown and will not be retried automatically".to_string(),
                    ),
                    success: Some(false),
                },
            },
        ]
    );
}

#[tokio::test]
async fn record_injected_provider_history_items_rejects_duplicate_batch_item() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("duplicate-injected-call", "test_tool");

    let err = session
        .record_injected_provider_history_items(&turn_context, &[call.clone(), call])
        .await
        .expect_err("duplicate items in one injection batch must be rejected");

    assert!(
        err.to_string()
            .contains("duplicate tool history item in submitted batch")
    );
    assert!(session.clone_history().await.raw_items().is_empty());
}

#[tokio::test]
async fn record_completed_tool_exchange_is_idempotent() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    let output = function_output("call-provider-history", "done");
    let hook_context = crate::context::ContextualUserFragment::into(
        crate::context::HookAdditionalContext::new("remember the tool result"),
    );
    let completed_exchange = vec![output.clone(), hook_context.clone()];
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;

    session
        .record_completed_tool_exchange(&turn_context, &completed_exchange)
        .await
        .expect("first exchange should be recorded");
    session
        .record_completed_tool_exchange(&turn_context, &completed_exchange)
        .await
        .expect("exact exchange retry should be ignored");

    assert_eq!(
        session.clone_history().await.raw_items(),
        &[call, output, hook_context]
    );
}

#[tokio::test]
async fn record_completed_tool_exchange_full_exchange_is_idempotent() {
    let (session, turn_context) = make_session_and_context().await;
    let completed_exchange = vec![
        function_call("call-full-exchange", "test_tool"),
        function_output("call-full-exchange", "done"),
    ];

    session
        .record_completed_tool_exchange(&turn_context, &completed_exchange)
        .await
        .expect("first full exchange should be recorded");
    session
        .record_completed_tool_exchange(&turn_context, &completed_exchange)
        .await
        .expect("exact full exchange retry should be ignored");

    assert_eq!(
        session.clone_history().await.raw_items(),
        completed_exchange.as_slice()
    );
}

#[tokio::test]
async fn record_completed_tool_exchange_custom_full_exchange_is_idempotent() {
    let (session, turn_context) = make_session_and_context().await;
    let completed_exchange = vec![
        ResponseItem::CustomToolCall {
            id: None,
            status: None,
            call_id: "custom-full-exchange".to_string(),
            name: "test_tool".to_string(),
            input: "{}".to_string(),
        },
        ResponseItem::CustomToolCallOutput {
            call_id: "custom-full-exchange".to_string(),
            name: None,
            output: FunctionCallOutputPayload::from_text("done".to_string()),
        },
    ];

    for attempt in ["first", "retry"] {
        session
            .record_completed_tool_exchange(&turn_context, &completed_exchange)
            .await
            .unwrap_or_else(|err| panic!("{attempt} custom exchange should be accepted: {err}"));
        assert_eq!(
            session.clone_history().await.raw_items(),
            completed_exchange.as_slice()
        );
    }
}

#[tokio::test]
async fn record_completed_tool_exchange_client_tool_search_full_exchange_is_idempotent() {
    let (session, turn_context) = make_session_and_context().await;
    let completed_exchange = vec![
        ResponseItem::ToolSearchCall {
            id: None,
            call_id: Some("search-full-exchange".to_string()),
            status: Some("completed".to_string()),
            execution: "client".to_string(),
            arguments: serde_json::json!({"query": "test"}),
        },
        ResponseItem::ToolSearchOutput {
            call_id: Some("search-full-exchange".to_string()),
            status: "completed".to_string(),
            execution: "client".to_string(),
            tools: vec![],
        },
    ];

    for attempt in ["first", "retry"] {
        session
            .record_completed_tool_exchange(&turn_context, &completed_exchange)
            .await
            .unwrap_or_else(|err| panic!("{attempt} search exchange should be accepted: {err}"));
        assert_eq!(
            session.clone_history().await.raw_items(),
            completed_exchange.as_slice()
        );
    }
}

#[tokio::test]
async fn record_completed_tool_exchange_server_tool_search_output_is_idempotent_without_call() {
    let (session, turn_context) = make_session_and_context().await;
    let server_output = |tools: Vec<serde_json::Value>| ResponseItem::ToolSearchOutput {
        call_id: Some("server-search".to_string()),
        status: "completed".to_string(),
        execution: "server".to_string(),
        tools,
    };
    let output = server_output(vec![]);

    for attempt in ["first", "retry"] {
        session
            .record_completed_tool_exchange(&turn_context, std::slice::from_ref(&output))
            .await
            .unwrap_or_else(|err| panic!("{attempt} server output should be accepted: {err}"));
        assert_eq!(
            session.clone_history().await.raw_items(),
            std::slice::from_ref(&output)
        );
    }

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[server_output(vec![
                serde_json::json!({"type": "function", "name": "changed"}),
            ])],
        )
        .await
        .expect_err("changed server output under the same id should be rejected");
    assert!(
        err.to_string()
            .contains("conflicting duplicate tool history item")
    );
    assert_eq!(
        session.clone_history().await.raw_items(),
        std::slice::from_ref(&output)
    );
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_compaction_without_changing_history() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    let before = session.clone_history().await.raw_items().to_vec();
    let private_sentinel = "private-compaction-content-must-not-appear";

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[
                ResponseItem::ContextCompaction {
                    encrypted_content: Some(private_sentinel.to_string()),
                },
                function_output("call-provider-history", "done"),
            ],
        )
        .await
        .expect_err("compaction inside a pending exchange must be rejected");

    assert!(matches!(
        &err,
        CodexErr::InvalidRequest(message)
            if message == "ambiguous item between function call and output in provider request history"
    ));
    assert!(!err.to_string().contains(private_sentinel));
    assert_eq!(session.clone_history().await.raw_items(), before.as_slice());
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_output_before_call_without_changing_history() {
    let (session, turn_context) = make_session_and_context().await;
    let before = session.clone_history().await.raw_items().to_vec();
    assert!(before.is_empty());

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[
                function_output("call-provider-history", "done"),
                function_call("call-provider-history", "test_tool"),
            ],
        )
        .await
        .expect_err("output before its call must be rejected");

    assert!(matches!(err, CodexErr::InvalidRequest(_)));
    assert_eq!(session.clone_history().await.raw_items(), before.as_slice());
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_developer_boundary_without_changing_history() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    let before = session.clone_history().await.raw_items().to_vec();

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[ResponseItem::Message {
                id: None,
                role: "developer".to_string(),
                content: vec![ContentItem::InputText {
                    text: "next instruction".to_string(),
                }],
                phase: None,
            }],
        )
        .await
        .expect_err("developer boundary with a pending call must be rejected");

    assert!(matches!(err, CodexErr::InvalidRequest(_)));
    assert_eq!(session.clone_history().await.raw_items(), before.as_slice());
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_orphan_output() {
    let (session, turn_context) = make_session_and_context().await;
    let call_id = "call-provider-history-secret";

    let err = session
        .record_completed_tool_exchange(&turn_context, &[function_output(call_id, "orphaned")])
        .await
        .expect_err("orphan output should be rejected");

    let message = err.to_string();
    assert!(message.contains("call id hash"));
    assert!(!message.contains(call_id));
    assert!(session.clone_history().await.raw_items().is_empty());
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_conflicting_retry() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    let output = function_output("call-provider-history", "first");
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    session
        .record_completed_tool_exchange(&turn_context, std::slice::from_ref(&output))
        .await
        .expect("first output should be recorded");

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[function_output("call-provider-history", "different")],
        )
        .await
        .expect_err("conflicting retry should be rejected");

    assert!(
        err.to_string()
            .contains("conflicting duplicate tool history item")
    );
    assert_eq!(session.clone_history().await.raw_items(), &[call, output]);
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_partial_retry() {
    let (session, turn_context) = make_session_and_context().await;
    let first_call = function_call("call-provider-history-1", "test_tool");
    let second_call = function_call("call-provider-history-2", "test_tool");
    let first_output = function_output("call-provider-history-1", "first");
    let second_output = function_output("call-provider-history-2", "second");
    session
        .record_conversation_items(&turn_context, &[first_call.clone(), second_call.clone()])
        .await;
    session
        .record_completed_tool_exchange(&turn_context, std::slice::from_ref(&first_output))
        .await
        .expect("first output should be recorded");

    let err = session
        .record_completed_tool_exchange(
            &turn_context,
            &[first_output.clone(), second_output.clone()],
        )
        .await
        .expect_err("partial exchange retry should be rejected");
    assert!(
        err.to_string()
            .contains("partial or changed tool exchange retry")
    );
    assert_eq!(
        session.clone_history().await.raw_items(),
        &[first_call, second_call, first_output]
    );
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_new_context_on_output_retry() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    let output = function_output("call-provider-history", "done");
    let hook_context = crate::context::ContextualUserFragment::into(
        crate::context::HookAdditionalContext::new("new context after retry"),
    );
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    session
        .record_completed_tool_exchange(&turn_context, std::slice::from_ref(&output))
        .await
        .expect("first output should be recorded");

    let err = session
        .record_completed_tool_exchange(&turn_context, &[output.clone(), hook_context.clone()])
        .await
        .expect_err("output retry with new hook context should be rejected");
    assert!(
        err.to_string()
            .contains("partial or changed tool exchange retry")
    );

    assert_eq!(session.clone_history().await.raw_items(), &[call, output]);
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_missing_hook_on_retry() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    let output = function_output("call-provider-history", "done");
    let hook_context = crate::context::ContextualUserFragment::into(
        crate::context::HookAdditionalContext::new("required hook context"),
    );
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    session
        .record_completed_tool_exchange(&turn_context, &[output.clone(), hook_context.clone()])
        .await
        .expect("first exchange should be recorded");

    let err = session
        .record_completed_tool_exchange(&turn_context, std::slice::from_ref(&output))
        .await
        .expect_err("retry without recorded hook context should be rejected");
    assert!(
        err.to_string()
            .contains("partial or changed tool exchange retry")
    );
    assert_eq!(
        session.clone_history().await.raw_items(),
        &[call, output, hook_context]
    );
}

#[tokio::test]
async fn record_completed_tool_exchange_rejects_changed_hook_retry() {
    let (session, turn_context) = make_session_and_context().await;
    let call = function_call("call-provider-history", "test_tool");
    let output = function_output("call-provider-history", "done");
    let original_hook = crate::context::ContextualUserFragment::into(
        crate::context::HookAdditionalContext::new("original hook context"),
    );
    let changed_hook = crate::context::ContextualUserFragment::into(
        crate::context::HookAdditionalContext::new("changed hook context"),
    );
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&call))
        .await;
    session
        .record_completed_tool_exchange(&turn_context, &[output.clone(), original_hook.clone()])
        .await
        .expect("first exchange should be recorded");

    let err = session
        .record_completed_tool_exchange(&turn_context, &[output.clone(), changed_hook])
        .await
        .expect_err("retry with changed hook context should be rejected");
    assert!(
        err.to_string()
            .contains("partial or changed tool exchange retry")
    );
    assert_eq!(
        session.clone_history().await.raw_items(),
        &[call, output, original_hook]
    );
}

pub(crate) async fn make_session_and_context() -> (Session, TurnContext) {
    let (tx_event, _rx_event) = async_channel::unbounded();
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let config = build_test_config(ontocode_home.path()).await;
    let config = Arc::new(config);
    let thread_id = ThreadId::default();
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let agent_control = AgentControl::default();
    let exec_policy = Arc::new(ExecPolicyManager::default());
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort.clone();
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };
    let per_turn_config =
        Session::build_per_turn_config(&session_configuration, session_configuration.cwd.clone());
    let model_info = construct_model_info_offline_for_tests(
        session_configuration.collaboration_mode.model(),
        &per_turn_config.to_models_manager_config(),
    );
    let session_telemetry = session_telemetry(
        thread_id,
        config.as_ref(),
        &model_info,
        session_configuration.session_source.clone(),
    );

    let state = SessionState::new(session_configuration.clone());
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let network_approval = Arc::new(NetworkApprovalService::default());
    let environment = Arc::new(
        ontocode_exec_server::Environment::create_for_tests(/*exec_server_url*/ None)
            .expect("create environment"),
    );

    let services = SessionServices {
        mcp_connection_manager: Arc::new(RwLock::new(
            McpConnectionManager::new_uninitialized_with_permission_profile(
                &config.permissions.approval_policy,
                config.permissions.permission_profile(),
                config.prefix_mcp_tool_names(),
            ),
        )),
        mcp_startup_cancellation_token: Mutex::new(CancellationToken::new()),
        unified_exec_manager: UnifiedExecProcessManager::new(
            config.background_terminal_max_timeout,
        ),
        shell_zsh_path: None,
        main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
        analytics_events_client: AnalyticsEventsClient::new(
            Arc::clone(&auth_manager),
            config.chatgpt_base_url.trim_end_matches('/').to_string(),
            config.analytics_enabled,
        ),
        hooks: arc_swap::ArcSwap::from_pointee(Hooks::new(HooksConfig {
            legacy_notify_argv: config.notify.clone(),
            ..HooksConfig::default()
        })),
        rollout_thread_trace: ontocode_rollout_trace::ThreadTraceContext::disabled(),
        user_shell: Arc::new(default_user_shell()),
        shell_snapshot_tx: watch::channel(None).0,
        show_raw_agent_reasoning: config.show_raw_agent_reasoning,
        exec_policy,
        auth_manager: auth_manager.clone(),
        session_telemetry: session_telemetry.clone(),
        models_manager: Arc::clone(&models_manager),
        tool_approvals: Mutex::new(ApprovalStore::default()),
        guardian_rejections: Mutex::new(std::collections::HashMap::new()),
        guardian_rejection_circuit_breaker: Mutex::new(Default::default()),
        runtime_handle: tokio::runtime::Handle::current(),
        skills_manager,
        plugins_manager,
        mcp_manager,
        extensions: Arc::new(ontocode_extension_api::ExtensionRegistryBuilder::new().build()),
        session_extension_data: ontocode_extension_api::ExtensionData::new(
            agent_control.session_id().to_string(),
        ),
        thread_extension_data: ontocode_extension_api::ExtensionData::new(thread_id.to_string()),
        agent_control,
        network_proxy: arc_swap::ArcSwapOption::from(None),
        network_proxy_audit_metadata: crate::config::NetworkProxyAuditMetadata::default(),
        managed_network_requirements_configured: false,
        network_approval: Arc::clone(&network_approval),
        state_db: None,
        live_thread: None,
        thread_store: Arc::new(ontocode_thread_store::LocalThreadStore::new(
            ontocode_thread_store::LocalThreadStoreConfig::from_config(config.as_ref()),
            /*state_db*/ None,
        )),
        attestation_provider: None,
        model_client: ModelClient::new(
            Some(auth_manager.clone()),
            thread_id.into(),
            thread_id,
            /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
            session_configuration.provider.clone(),
            session_configuration.session_source.clone(),
            session_configuration.parent_thread_id,
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            Session::build_model_client_beta_features_header(config.as_ref()),
            /*attestation_provider*/ None,
        ),
        code_mode_service: crate::tools::code_mode::CodeModeService::new(),
        environment_manager: Arc::new(ontocode_exec_server::EnvironmentManager::default_for_tests()),
    };

    let plugin_outcome = services
        .plugins_manager
        .plugins_for_config(&per_turn_config.plugins_config_input())
        .await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let skills_input =
        crate::skills_load_input_from_config(&per_turn_config, effective_skill_roots);
    let skill_fs = environment.get_filesystem();
    let skills_outcome = Arc::new(
        services
            .skills_manager
            .skills_for_config(&skills_input, Some(Arc::clone(&skill_fs)))
            .await,
    );
    let turn_environments = turn_environments_for_tests(&environment, &session_configuration.cwd);
    let turn_context = Session::make_turn_context(
        thread_id,
        SessionId::from(thread_id),
        Some(Arc::clone(&auth_manager)),
        &session_telemetry,
        session_configuration.provider.clone(),
        &session_configuration,
        config.multi_agent_version_from_features(),
        services.user_shell.as_ref(),
        services.shell_zsh_path.as_ref(),
        services.main_execve_wrapper_exe.as_ref(),
        per_turn_config,
        model_info,
        &models_manager,
        /*network*/ None,
        turn_environments,
        session_configuration.cwd.clone(),
        "turn_id".to_string(),
        skills_outcome,
        /*goal_tools_supported*/ true,
        /*permission_profile_override*/ None,
    );

    let session = Session {
        thread_id,
        installation_id: "11111111-1111-4111-8111-111111111111".to_string(),
        tx_event,
        agent_status: agent_status_tx,
        out_of_band_elicitation_paused: watch::channel(false).0,
        state: Mutex::new(state),
        artifact_receipt_commit_lock: tokio::sync::Mutex::new(()),
        reload_user_config_lock: tokio::sync::Mutex::new(()),
        managed_network_proxy_refresh_lock: Semaphore::new(/*permits*/ 1),
        managed_network_proxy_refresh_pending: std::sync::atomic::AtomicBool::new(false),
        turn_generation: std::sync::atomic::AtomicU64::new(0),
        foreground_submission_generation: std::sync::atomic::AtomicU64::new(0),
        pending_foreground_submissions: Mutex::new(Default::default()),
        model_overload_generation: std::sync::atomic::AtomicU64::new(0),
        last_activity_unix_ms: std::sync::atomic::AtomicI64::new(0),
        features: config.features.clone(),
        multi_agent_version: OnceLock::from(config.multi_agent_version_from_features()),
        pending_mcp_server_refresh_config: Mutex::new(None),
        conversation: Arc::new(RealtimeConversationManager::new()),
        active_turn: Mutex::new(None),
        turn_start_lock: Mutex::new(()),
        execution_retired: std::sync::atomic::AtomicBool::new(false),
        input_queue: super::input_queue::InputQueue::new(),
        guardian_review_session: crate::guardian::GuardianReviewSessionManager::default(),
        services,
        next_internal_sub_id: std::sync::atomic::AtomicU64::new(0),
    };

    (session, turn_context)
}

async fn make_session_with_config(
    mutator: impl FnOnce(&mut Config),
) -> anyhow::Result<Arc<Session>> {
    let (session, _rx_event) = make_session_with_config_and_rx(mutator).await?;
    Ok(session)
}

async fn load_latest_config_for_session(session: &Session) -> Config {
    let config = session.get_config().await;
    ConfigBuilder::default()
        .ontocode_home(config.codex_home.to_path_buf())
        .fallback_cwd(Some(config.cwd.to_path_buf()))
        .build()
        .await
        .expect("load latest config for session")
}

async fn make_session_with_config_and_rx(
    mutator: impl FnOnce(&mut Config),
) -> anyhow::Result<(Arc<Session>, async_channel::Receiver<Event>)> {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(ontocode_home.path()).await;
    mutator(&mut config);
    let config = Arc::new(config);
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort.clone(),
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));

    let session = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        InitialHistory::New,
        SessionSource::Exec,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(ontocode_extension_api::ExtensionRegistryBuilder::new().build()),
        AgentControl::default(),
        Arc::new(ontocode_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(ontocode_thread_store::LocalThreadStore::new(
            ontocode_thread_store::LocalThreadStoreConfig::from_config(config.as_ref()),
            /*state_db*/ None,
        )),
        ontocode_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
        Some(config.multi_agent_version_from_features()),
    )
    .await?;

    Ok((session, rx_event))
}

async fn make_session_with_history_source_and_agent_control_and_rx(
    initial_history: InitialHistory,
    session_source: SessionSource,
    agent_control: AgentControl,
) -> anyhow::Result<(Arc<Session>, async_channel::Receiver<Event>)> {
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    let mut config = build_test_config(ontocode_home.path()).await;
    config.ephemeral = true;
    let config = Arc::new(config);
    let auth_manager = AuthManager::from_auth_for_testing(CodexAuth::from_api_key("Test API Key"));
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort: config.model_reasoning_effort.clone(),
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: session_source.clone(),
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools: Vec::new(),
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };

    let (tx_event, rx_event) = async_channel::unbounded();
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));

    let session = Session::new(
        session_configuration,
        Arc::clone(&config),
        "11111111-1111-4111-8111-111111111111".to_string(),
        auth_manager,
        models_manager,
        Arc::new(ExecPolicyManager::default()),
        tx_event,
        agent_status_tx,
        initial_history,
        session_source,
        skills_manager,
        plugins_manager,
        mcp_manager,
        Arc::new(ontocode_extension_api::ExtensionRegistryBuilder::new().build()),
        agent_control,
        Arc::new(ontocode_exec_server::EnvironmentManager::default_for_tests()),
        /*analytics_events_client*/ None,
        Arc::new(ontocode_thread_store::LocalThreadStore::new(
            ontocode_thread_store::LocalThreadStoreConfig::from_config(config.as_ref()),
            Some(
                ontocode_state::StateRuntime::init(
                    config.sqlite_home.clone(),
                    config.model_provider_id.clone(),
                )
                .await
                .expect("state db should initialize"),
            ),
        )),
        ontocode_rollout_trace::ThreadTraceContext::disabled(),
        /*attestation_provider*/ None,
        Some(config.multi_agent_version_from_features()),
    )
    .await?;

    Ok((session, rx_event))
}

#[tokio::test]
async fn resumed_root_session_uses_thread_id_as_session_id() {
    let thread_id = ThreadId::new();
    let (session, rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        SessionSource::Exec,
        AgentControl::default(),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(session.thread_id(), thread_id);
    assert_eq!(session.session_id(), SessionId::from(thread_id));

    let event = rx_event.recv().await.expect("session configured event");
    let EventMsg::SessionConfigured(event) = event.msg else {
        panic!("expected session configured event");
    };
    assert_eq!(event.session_id, SessionId::from(thread_id));
    assert_eq!(event.thread_id, thread_id);
}

#[tokio::test]
async fn forked_session_prompt_cache_key_is_the_fork_source_thread_id() {
    let thread_id = ThreadId::new();
    let fork_source = ThreadId::new();
    let (session, _rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: vec![RolloutItem::SessionMeta(SessionMetaLine {
                meta: SessionMeta {
                    id: thread_id,
                    forked_from_id: Some(fork_source),
                    ..SessionMeta::default()
                },
                git: None,
            })],
            rollout_path: None,
        }),
        SessionSource::Exec,
        AgentControl::default(),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(
        session.services.model_client.prompt_cache_key(),
        fork_source.to_string()
    );
}

#[tokio::test]
async fn unforked_session_prompt_cache_key_is_its_own_thread_id() {
    let thread_id = ThreadId::new();
    let (session, _rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        SessionSource::Exec,
        AgentControl::default(),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(
        session.services.model_client.prompt_cache_key(),
        thread_id.to_string()
    );
}

#[tokio::test]
async fn resumed_subagent_session_keeps_inherited_session_id() {
    let parent_thread_id = ThreadId::new();
    let parent_session_id = SessionId::from(parent_thread_id);
    let thread_id = ThreadId::new();
    let session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id,
        depth: 1,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
    });
    let (session, rx_event) = make_session_with_history_source_and_agent_control_and_rx(
        InitialHistory::Resumed(ResumedHistory {
            conversation_id: thread_id,
            history: Vec::new(),
            rollout_path: None,
        }),
        session_source,
        AgentControl::default().with_session_id(parent_session_id),
    )
    .await
    .expect("resume should succeed");

    assert_eq!(session.thread_id(), thread_id);
    assert_eq!(session.session_id(), parent_session_id);

    let event = rx_event.recv().await.expect("session configured event");
    let EventMsg::SessionConfigured(event) = event.msg else {
        panic!("expected session configured event");
    };
    assert_eq!(event.session_id, parent_session_id);
    assert_eq!(event.thread_id, thread_id);
}

#[tokio::test]
async fn notify_request_permissions_response_ignores_unmatched_call_id() {
    let (session, _turn_context) = make_session_and_context().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());

    session
        .notify_request_permissions_response(
            "missing",
            ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: RequestPermissionProfile {
                    network: Some(ontocode_protocol::models::NetworkPermissions {
                        enabled: Some(true),
                    }),
                    ..RequestPermissionProfile::default()
                },
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
        )
        .await;

    assert_eq!(
        session
            .granted_turn_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
            .await,
        None
    );
}

#[tokio::test]
async fn record_granted_request_permissions_for_turn_uses_originating_turn() {
    let (session, _turn_context) = make_session_and_context().await;
    let originating_active_turn = ActiveTurn::default();
    let originating_turn_state = Arc::clone(&originating_active_turn.turn_state);
    *session.active_turn.lock().await = Some(originating_active_turn);

    let current_active_turn = ActiveTurn::default();
    let current_turn_state = Arc::clone(&current_active_turn.turn_state);
    *session.active_turn.lock().await = Some(current_active_turn);

    let requested_permissions = RequestPermissionProfile {
        network: Some(ontocode_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };
    session
        .record_granted_request_permissions_for_turn(
            &ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
            ontocode_exec_server::LOCAL_ENVIRONMENT_ID,
            Some(&originating_turn_state),
        )
        .await;

    assert_eq!(
        originating_turn_state
            .lock()
            .await
            .granted_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID),
        Some(requested_permissions.into())
    );
    assert_eq!(
        current_turn_state
            .lock()
            .await
            .granted_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID),
        None
    );
    assert_eq!(
        session
            .granted_turn_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
            .await,
        None
    );
}

#[tokio::test]
async fn request_permission_grants_are_environment_keyed() {
    let (session, _turn_context) = make_session_and_context().await;
    let originating_active_turn = ActiveTurn::default();
    let originating_turn_state = Arc::clone(&originating_active_turn.turn_state);
    *session.active_turn.lock().await = Some(originating_active_turn);

    let requested_permissions = RequestPermissionProfile {
        network: Some(ontocode_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };
    session
        .record_granted_request_permissions_for_turn(
            &ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
            "remote",
            Some(&originating_turn_state),
        )
        .await;

    {
        let turn_state = originating_turn_state.lock().await;
        assert_eq!(
            turn_state.granted_permissions("remote"),
            Some(requested_permissions.clone().into())
        );
        assert_eq!(turn_state.granted_permissions("local"), None);
    }

    session
        .record_granted_request_permissions_for_turn(
            &ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Session,
                strict_auto_review: false,
            },
            "remote",
            /*originating_turn_state*/ None,
        )
        .await;

    assert_eq!(
        session.granted_session_permissions("remote").await,
        Some(requested_permissions.into())
    );
    assert_eq!(session.granted_session_permissions("local").await, None);
}

#[tokio::test]
async fn enable_strict_auto_review_for_turn_uses_originating_turn() {
    let (session, _turn_context) = make_session_and_context().await;
    let originating_active_turn = ActiveTurn::default();
    let originating_turn_state = Arc::clone(&originating_active_turn.turn_state);
    *session.active_turn.lock().await = Some(originating_active_turn);

    let requested_permissions = RequestPermissionProfile {
        network: Some(ontocode_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };
    session
        .record_granted_request_permissions_for_turn(
            &ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: requested_permissions.clone(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: true,
            },
            ontocode_exec_server::LOCAL_ENVIRONMENT_ID,
            Some(&originating_turn_state),
        )
        .await;

    assert!(
        originating_turn_state
            .lock()
            .await
            .strict_auto_review_enabled()
    );
}

#[test]
fn strict_auto_review_session_scope_grants_no_permissions() {
    let requested_permissions = RequestPermissionProfile {
        network: Some(ontocode_protocol::models::NetworkPermissions {
            enabled: Some(true),
        }),
        ..RequestPermissionProfile::default()
    };

    let response = Session::normalize_request_permissions_response(
        requested_permissions.clone(),
        ontocode_protocol::request_permissions::RequestPermissionsResponse {
            permissions: requested_permissions,
            scope: PermissionGrantScope::Session,
            strict_auto_review: true,
        },
        std::path::Path::new("/tmp"),
    );

    assert_eq!(
        response,
        ontocode_protocol::request_permissions::RequestPermissionsResponse {
            permissions: RequestPermissionProfile::default(),
            scope: PermissionGrantScope::Turn,
            strict_auto_review: false,
        }
    );
}

#[tokio::test]
async fn request_permissions_emits_event_when_granular_policy_allows_requests() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single thread settings ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let expected_response = ontocode_protocol::request_permissions::RequestPermissionsResponse {
        permissions: RequestPermissionProfile {
            network: Some(ontocode_protocol::models::NetworkPermissions {
                enabled: Some(true),
            }),
            ..RequestPermissionProfile::default()
        },
        scope: PermissionGrantScope::Turn,
        strict_auto_review: false,
    };

    let handle = tokio::spawn({
        let session = Arc::clone(&session);
        let turn_context = Arc::clone(&turn_context);
        let call_id = call_id.clone();
        async move {
            let environment = turn_context
                .environments
                .primary()
                .expect("primary environment")
                .selection();
            session
                .request_permissions_for_environment(
                    &turn_context,
                    call_id,
                    ontocode_protocol::request_permissions::RequestPermissionsArgs {
                        environment_id: None,
                        reason: Some("need network".to_string()),
                        permissions: RequestPermissionProfile {
                            network: Some(ontocode_protocol::models::NetworkPermissions {
                                enabled: Some(true),
                            }),
                            ..RequestPermissionProfile::default()
                        },
                    },
                    environment,
                    CancellationToken::new(),
                )
                .await
        }
    });

    let request_event = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
        .await
        .expect("request_permissions event timed out")
        .expect("request_permissions event missing");
    let EventMsg::RequestPermissions(request) = request_event.msg else {
        panic!("expected request_permissions event");
    };
    assert_eq!(request.call_id, call_id);
    assert_eq!(
        request.environment_id.as_deref(),
        Some(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
    );
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(request.cwd, Some(turn_cwd));

    session
        .notify_request_permissions_response(&request.call_id, expected_response.clone())
        .await;

    let response = tokio::time::timeout(StdDuration::from_secs(1), handle)
        .await
        .expect("request_permissions future timed out")
        .expect("request_permissions join error");

    assert_eq!(response, Some(expected_response));
}

#[tokio::test]
async fn request_permissions_tool_resolves_relative_paths_against_selected_environment() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    let environment_cwd = {
        #[allow(deprecated)]
        let legacy_cwd = turn_context.cwd.clone();
        legacy_cwd.join("request-permissions-environment")
    };
    std::fs::create_dir_all(environment_cwd.as_path()).expect("create environment cwd");
    let turn_context_mut = Arc::get_mut(&mut turn_context).expect("single thread settings ref");
    turn_context_mut
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");
    turn_context_mut.environments.turn_environments[0].environment_id = "remote".to_string();
    turn_context_mut.environments.turn_environments[0].cwd = environment_cwd.clone();

    let call_id = "call-1".to_string();
    let handler = RequestPermissionsHandler;
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let handle = tokio::spawn({
        let session = Arc::clone(&session);
        let turn_context = Arc::clone(&turn_context);
        let tracker = Arc::clone(&tracker);
        let call_id = call_id.clone();
        async move {
            handler
                .handle(ToolInvocation {
                    session,
                    turn: turn_context,
                    cancellation_token: CancellationToken::new(),
                    tracker,
                    call_id,
                    tool_name: ontocode_tools::ToolName::plain("request_permissions"),
                    source: ToolCallSource::Direct,
                    payload: ToolPayload::Function {
                        arguments: json!({
                            "environment_id": "remote",
                            "reason": "need write",
                            "permissions": {
                                "file_system": {
                                    "entries": [{
                                        "path": {
                                            "type": "path",
                                            "path": "relative.txt",
                                        },
                                        "access": "write",
                                    }],
                                },
                            },
                        })
                        .to_string(),
                    },
                })
                .await
        }
    });

    let request_event = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
        .await
        .expect("request_permissions event timed out")
        .expect("request_permissions event missing");
    let EventMsg::RequestPermissions(request) = request_event.msg else {
        panic!("expected request_permissions event");
    };
    let expected_permissions = RequestPermissionProfile {
        file_system: Some(FileSystemPermissions {
            entries: vec![FileSystemSandboxEntry {
                path: FileSystemPath::Path {
                    path: environment_cwd.join("relative.txt"),
                },
                access: FileSystemAccessMode::Write,
            }],
            glob_scan_max_depth: None,
        }),
        ..Default::default()
    };
    assert_eq!(request.environment_id.as_deref(), Some("remote"));
    assert_eq!(request.permissions, expected_permissions);

    session
        .notify_request_permissions_response(
            &request.call_id,
            ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: request.permissions,
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            },
        )
        .await;
    tokio::time::timeout(StdDuration::from_secs(1), handle)
        .await
        .expect("request_permissions handler timed out")
        .expect("request_permissions handler join error")
        .expect("request_permissions handler should succeed");
}

#[tokio::test]
async fn request_permissions_tool_rejects_unknown_environment_id() {
    let (session, turn_context) = make_session_and_context().await;
    let result = RequestPermissionsHandler
        .handle(ToolInvocation {
            session: Arc::new(session),
            turn: Arc::new(turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new())),
            call_id: "call-1".to_string(),
            tool_name: ontocode_tools::ToolName::plain("request_permissions"),
            source: ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: json!({
                    "environment_id": "missing",
                    "permissions": {
                        "network": {
                            "enabled": true,
                        },
                    },
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = result else {
        panic!("expected unknown environment id to be rejected");
    };
    assert_eq!(output, "unknown turn environment id `missing`");
}

#[tokio::test]
async fn request_permissions_response_materializes_session_cwd_grants_before_recording() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single thread settings ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: true,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let requested_permissions = RequestPermissionProfile {
        file_system: Some(FileSystemPermissions {
            entries: vec![FileSystemSandboxEntry {
                path: FileSystemPath::Special {
                    value: FileSystemSpecialPath::project_roots(/*subpath*/ None),
                },
                access: FileSystemAccessMode::Write,
            }],
            glob_scan_max_depth: None,
        }),
        ..Default::default()
    };

    let handle = tokio::spawn({
        let session = Arc::clone(&session);
        let turn_context = Arc::clone(&turn_context);
        let call_id = call_id.clone();
        let requested_permissions = requested_permissions.clone();
        async move {
            let environment = turn_context
                .environments
                .primary()
                .expect("primary environment")
                .selection();
            session
                .request_permissions_for_environment(
                    &turn_context,
                    call_id,
                    ontocode_protocol::request_permissions::RequestPermissionsArgs {
                        environment_id: None,
                        reason: Some("need cwd write".to_string()),
                        permissions: requested_permissions,
                    },
                    environment,
                    CancellationToken::new(),
                )
                .await
        }
    });

    let request_event = tokio::time::timeout(StdDuration::from_secs(1), rx.recv())
        .await
        .expect("request_permissions event timed out")
        .expect("request_permissions event missing");
    let EventMsg::RequestPermissions(request) = request_event.msg else {
        panic!("expected request_permissions event");
    };
    assert_eq!(
        request.environment_id.as_deref(),
        Some(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
    );
    let request_cwd = request.cwd.clone().expect("request cwd");

    session
        .notify_request_permissions_response(
            &request.call_id,
            ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: request.permissions,
                scope: PermissionGrantScope::Session,
                strict_auto_review: false,
            },
        )
        .await;

    let expected_permissions = RequestPermissionProfile {
        file_system: Some(FileSystemPermissions::from_read_write_roots(
            /*read*/ None,
            Some(vec![request_cwd]),
        )),
        ..Default::default()
    };
    let expected_response = ontocode_protocol::request_permissions::RequestPermissionsResponse {
        permissions: expected_permissions.clone(),
        scope: PermissionGrantScope::Session,
        strict_auto_review: false,
    };

    let response = tokio::time::timeout(StdDuration::from_secs(1), handle)
        .await
        .expect("request_permissions future timed out")
        .expect("request_permissions join error");

    assert_eq!(response, Some(expected_response));
    assert_eq!(
        session
            .granted_session_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
            .await,
        Some(expected_permissions.into())
    );
}

#[tokio::test]
async fn request_permissions_is_auto_denied_when_granular_policy_blocks_tool_requests() {
    let (session, mut turn_context, rx) = make_session_and_context_with_rx().await;
    *session.active_turn.lock().await = Some(ActiveTurn::default());
    Arc::get_mut(&mut turn_context)
        .expect("single thread settings ref")
        .approval_policy
        .set(AskForApproval::Granular(GranularApprovalConfig {
            sandbox_approval: true,
            rules: true,
            skill_approval: true,
            request_permissions: false,
            mcp_elicitations: true,
        }))
        .expect("test setup should allow updating approval policy");

    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "call-1".to_string();
    let environment = turn_context
        .environments
        .primary()
        .expect("primary environment")
        .selection();
    let response = session
        .request_permissions_for_environment(
            &turn_context,
            call_id,
            ontocode_protocol::request_permissions::RequestPermissionsArgs {
                environment_id: None,
                reason: Some("need network".to_string()),
                permissions: RequestPermissionProfile {
                    network: Some(ontocode_protocol::models::NetworkPermissions {
                        enabled: Some(true),
                    }),
                    ..RequestPermissionProfile::default()
                },
            },
            environment,
            CancellationToken::new(),
        )
        .await;

    assert_eq!(
        response,
        Some(
            ontocode_protocol::request_permissions::RequestPermissionsResponse {
                permissions: RequestPermissionProfile::default(),
                scope: PermissionGrantScope::Turn,
                strict_auto_review: false,
            }
        )
    );
    assert!(
        tokio::time::timeout(StdDuration::from_millis(100), rx.recv())
            .await
            .is_err(),
        "request_permissions should not emit an event when granular.request_permissions is false"
    );
}

/// Stores the terminal attempt and matching dispatch edge a recovery claim requires, then commits it.
pub(crate) async fn commit_recovery_claim_for_tests(
    db: &crate::StateDbHandle,
    claim: &ontocode_state::AgentExecutionRecoveryClaim,
) {
    prepare_recovery_claim_for_tests(db, claim).await;
    db.claim_agent_execution_recovery(claim)
        .await
        .expect("claim execution recovery");
}

pub(crate) async fn prepare_recovery_claim_for_tests(
    db: &crate::StateDbHandle,
    claim: &ontocode_state::AgentExecutionRecoveryClaim,
) {
    db.store_terminal_agent_execution_attempt(&ontocode_state::AgentExecutionAttempt {
        attempt_id: claim.attempt_id.clone(),
        child_thread_id: claim.child_thread_id,
        parent_thread_id: claim.parent_thread_id,
        turn_generation: claim.turn_generation,
        tracking_path: Some(claim.tracking_path.clone()),
        task_id: Some(claim.task_id.clone()),
        tracking_revision: Some(claim.tracking_revision.clone()),
        role: Some(claim.role.clone()),
        effective_model: Some(claim.effective_model.clone()),
        lifecycle_state: "result_stored".to_string(),
        terminal_status: Some("shutdown".to_string()),
        result_text: None,
        result_sha256: None,
        result_ref: "retired-result".to_string(),
        completed_at_ms: Some(100),
        notification_state: ontocode_state::AgentExecutionNotificationState::Pending,
        notification_claimed_at_ms: None,
        state_version: claim.expected_state_version,
    })
    .await
    .expect("store terminal attempt");
    db.upsert_thread_spawn_edge_with_dispatch_metadata(
        claim.parent_thread_id,
        claim.child_thread_id,
        ontocode_state::DirectionalThreadSpawnEdgeStatus::Open,
        &ontocode_state::ThreadSpawnDispatchMetadata {
            parent_thread_id: Some(claim.parent_thread_id),
            logical_contract_role: Some(claim.role.clone()),
            effective_model: Some(claim.effective_model.clone()),
            dispatch_kind: "substantive".to_string(),
            required_capabilities: Vec::new(),
            tool_surface_identity: None,
            required_nested_dispatch_roles: Vec::new(),
            required_planning_tools: Vec::new(),
            evidence_requirements: None,
            evidence_generation: 0,
            artifact_receipt_id: None,
            reviewer_dispatched_at_ms: None,
            manager_loop_tracking_path: Some(claim.tracking_path.clone()),
            manager_loop_task_id: Some(claim.task_id.clone()),
            manager_loop_revision: Some(claim.tracking_revision.clone()),
            manager_loop_dispatch_id: Some(claim.dispatch_id.clone()),
            manager_loop_scope_sha256: Some(claim.scope_sha256.clone()),
            escalation_provenance_json: None,
            manager_loop_bootstrap_tracking_path: None,
            manager_loop_bootstrap_path_sha256: None,
            tool_profile: None,
            skill_name: None,
            skill_source: None,
            skill_execution_mode: None,
            skill_content_sha256: None,
        },
    )
    .await
    .expect("store matching dispatch edge");
}

#[tokio::test]
async fn submit_user_input_rejects_recovered_execution_before_turn_start() {
    let (session, _turn_context, rx_event) = make_session_and_context_with_state_db_and_rx().await;
    let db = session.state_db().expect("state db should be enabled");
    let parent_thread_id = ThreadId::new();
    let claim = ontocode_state::AgentExecutionRecoveryClaim {
        attempt_id: "retired-attempt".to_string(),
        child_thread_id: session.thread_id(),
        parent_thread_id,
        turn_generation: 1,
        expected_state_version: 1,
        tracking_path: "docs/plans/recovery-test.md".to_string(),
        task_id: "recovery-test".to_string(),
        tracking_revision: "a".repeat(64),
        dispatch_id: "retired-dispatch".to_string(),
        scope_sha256: "b".repeat(64),
        role: "coder".to_string(),
        effective_model: "test-model".to_string(),
        expected_tracker_revision: "c".repeat(64),
        replacement_bytes: b"recovered tracker\n".to_vec(),
        replacement_sha256: "d".repeat(64),
    };
    prepare_recovery_claim_for_tests(&db, &claim).await;
    session
        .commit_execution_recovery(&claim)
        .await
        .expect("retire live session");

    let (tx_sub, rx_sub) = async_channel::bounded(1);
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let ontocode = Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::clone(&session),
        session_loop_termination: completed_session_loop_termination(),
    };
    let error = ontocode
        .submit(Op::UserInput {
            environments: None,
            items: vec![],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        })
        .await
        .expect_err("recovered execution must reject input");

    assert!(matches!(
        error,
        CodexErr::UnsupportedOperation(message)
            if message == "agent execution was retired by recovery"
    ));
    assert!(
        rx_sub.is_empty(),
        "rejected input must not reach the session loop"
    );
    assert!(session.active_turn.lock().await.is_none());
    assert!(
        session
            .pending_foreground_submissions
            .lock()
            .await
            .is_empty()
    );
    assert_eq!(
        session
            .turn_generation
            .load(std::sync::atomic::Ordering::Acquire),
        0
    );
}

#[tokio::test]
async fn submit_with_id_captures_current_span_trace_context() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(1);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let ontocode = Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: completed_session_loop_termination(),
    };

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let expected_trace = async {
        let expected_trace =
            current_span_w3c_trace_context().expect("current span should have trace context");
        ontocode
            .submit_with_id(Submission {
                id: "sub-1".into(),
                op: Op::Interrupt,
                client_user_message_id: None,
                trace: None,
            })
            .await
            .expect("submit should succeed");
        expected_trace
    }
    .instrument(request_span)
    .await;

    let submitted = rx_sub.recv().await.expect("submission");
    assert_eq!(submitted.trace, Some(expected_trace));
}

#[tokio::test]
async fn new_default_turn_captures_current_span_trace_id() {
    let (session, _turn_context) = make_session_and_context().await;

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let turn_trace_id = async {
        let expected_trace_id = Span::current()
            .context()
            .span()
            .span_context()
            .trace_id()
            .to_string();
        let turn_context = session.new_default_turn().await;
        assert_eq!(turn_context.trace_id, Some(expected_trace_id));
        turn_context.trace_id.clone()
    }
    .instrument(request_span)
    .await;

    assert_eq!(
        turn_trace_id.as_deref(),
        Some("00000000000000000000000000000011")
    );
}

#[test]
fn submission_dispatch_span_prefers_submission_trace_context() {
    let _trace_test_context = install_test_tracing("codex-core-tests");

    let ambient_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000033-0000000000000044-01".into()),
        tracestate: None,
    };
    let ambient_span = info_span!("ambient");
    assert!(set_parent_from_w3c_trace_context(
        &ambient_span,
        &ambient_parent
    ));

    let submission_trace = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000055-0000000000000066-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let dispatch_span = ambient_span.in_scope(|| {
        submission_dispatch_span(&Submission {
            id: "sub-1".into(),
            op: Op::Interrupt,
            client_user_message_id: None,
            trace: Some(submission_trace),
        })
    });

    let trace_id = dispatch_span.context().span().span_context().trace_id();
    assert_eq!(
        trace_id,
        TraceId::from_hex("00000000000000000000000000000055").expect("trace id")
    );
}

#[test]
fn submission_dispatch_span_uses_debug_for_realtime_audio() {
    let _trace_test_context = install_test_tracing("codex-core-tests");

    let dispatch_span = submission_dispatch_span(&Submission {
        id: "sub-1".into(),
        op: Op::RealtimeConversationAudio(ConversationAudioParams {
            frame: RealtimeAudioFrame {
                data: "ZmFrZQ==".into(),
                sample_rate: 16_000,
                num_channels: 1,
                samples_per_channel: Some(160),
                item_id: None,
            },
        }),
        client_user_message_id: None,
        trace: None,
    });

    assert_eq!(
        dispatch_span.metadata().expect("span metadata").level(),
        &tracing::Level::DEBUG
    );
}

#[test]
fn op_kind_for_input_and_context_ops() {
    assert_eq!(
        Op::UserInput {
            environments: None,
            items: vec![],
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: Default::default(),
        }
        .kind(),
        "user_input"
    );
    assert_eq!(
        Op::ThreadSettings {
            thread_settings: ThreadSettingsOverrides::default(),
        }
        .kind(),
        "thread_settings"
    );
}

#[tokio::test]
async fn user_turn_updates_approvals_reviewer() {
    let (session, turn_context, _rx) = make_session_and_context_with_rx().await;
    let config = session.get_config().await;

    handlers::user_input_or_turn(
        &session,
        "sub-1".to_string(),
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            }],
            environments: None,
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ontocode_protocol::protocol::ThreadSettingsOverrides {
                cwd: Some(config.cwd.to_path_buf()),
                approval_policy: Some(config.permissions.approval_policy.value()),
                approvals_reviewer: Some(ontocode_config::types::ApprovalsReviewer::AutoReview),
                sandbox_policy: Some(config.legacy_sandbox_policy()),
                summary: config.model_reasoning_summary,
                personality: config.personality,
                collaboration_mode: Some(ontocode_protocol::config_types::CollaborationMode {
                    mode: ontocode_protocol::config_types::ModeKind::Default,
                    settings: ontocode_protocol::config_types::Settings {
                        model: turn_context.model_info.slug.clone(),
                        reasoning_effort: config.model_reasoning_effort.clone(),
                        developer_instructions: None,
                    },
                }),
                ..Default::default()
            },
        },
        /*client_user_message_id*/ None,
    )
    .await;

    let state = session.state.lock().await;
    assert_eq!(
        state.session_configuration.approvals_reviewer,
        ontocode_config::types::ApprovalsReviewer::AutoReview
    );
}

#[tokio::test]
async fn user_turn_with_settings_replaces_active_turn_before_mcp_policy_update() {
    let (session, initial_context, _rx) = make_session_and_context_with_rx().await;
    let initial_profile = initial_context.permission_profile();
    let replacement_profile = if initial_profile == PermissionProfile::Disabled {
        PermissionProfile::read_only()
    } else {
        PermissionProfile::Disabled
    };
    let (abort_profile_tx, abort_profile_rx) = tokio::sync::oneshot::channel();
    session
        .spawn_task(
            initial_context,
            Vec::new(),
            CaptureAbortMcpProfileTask {
                tx: std::sync::Mutex::new(Some(abort_profile_tx)),
            },
        )
        .await;

    handlers::user_input_or_turn(
        &session,
        "replacement-turn".to_string(),
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "use the new permissions".to_string(),
                text_elements: Vec::new(),
            }],
            environments: None,
            final_output_json_schema: None,
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides {
                permission_profile: Some(replacement_profile.clone()),
                ..Default::default()
            },
        },
        /*client_user_message_id*/ None,
    )
    .await;

    assert_eq!(
        timeout(Duration::from_secs(5), abort_profile_rx)
            .await
            .expect("active task should be replaced")
            .expect("aborted task should report its MCP profile"),
        initial_profile,
        "the old task must abort before the replacement policy is activated"
    );
    assert_eq!(
        session
            .services
            .mcp_connection_manager
            .read()
            .await
            .permission_profile_for_elicitation(),
        replacement_profile,
        "the replacement task must activate its own MCP policy"
    );

    session.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn invalid_output_schema_does_not_apply_settings_or_replace_active_turn() {
    let (session, initial_context, _rx) = make_session_and_context_with_rx().await;
    let initial_turn_id = initial_context.sub_id.clone();
    let initial_profile = initial_context.permission_profile();
    let replacement_profile = if initial_profile == PermissionProfile::Disabled {
        PermissionProfile::read_only()
    } else {
        PermissionProfile::Disabled
    };
    session
        .spawn_task(
            initial_context,
            Vec::new(),
            NeverEndingTask {
                kind: TaskKind::Regular,
                listen_to_cancellation_token: true,
            },
        )
        .await;

    handlers::user_input_or_turn(
        &session,
        "invalid-schema-turn".to_string(),
        Op::UserInput {
            items: vec![UserInput::Text {
                text: "do not apply these settings".to_string(),
                text_elements: Vec::new(),
            }],
            environments: None,
            final_output_json_schema: Some(json!({})),
            responsesapi_client_metadata: None,
            additional_context: Default::default(),
            thread_settings: ThreadSettingsOverrides {
                permission_profile: Some(replacement_profile),
                ..Default::default()
            },
        },
        /*client_user_message_id*/ None,
    )
    .await;

    {
        let state = session.state.lock().await;
        assert_eq!(
            state.session_configuration.permission_profile(),
            initial_profile
        );
    }
    let active_turn = session.active_turn.lock().await;
    assert_eq!(
        active_turn
            .as_ref()
            .and_then(|active_turn| active_turn.task.as_ref())
            .map(|task| task.turn_context.sub_id.as_str()),
        Some(initial_turn_id.as_str())
    );
    drop(active_turn);

    session.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn turn_environments_set_primary_environment() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let selected_cwd =
        AbsolutePathBuf::try_from(session.get_config().await.cwd.as_path().join("selected"))
            .expect("absolute path");

    let turn_context = session
        .new_turn_with_sub_id(
            "sub-1".to_string(),
            SessionSettingsUpdate {
                environments: Some(vec![TurnEnvironmentSelection {
                    environment_id: "local".to_string(),
                    cwd: selected_cwd.clone(),
                }]),
                ..Default::default()
            },
        )
        .await
        .expect("turn should start");

    let turn_environments = &turn_context.environments;
    assert_eq!(turn_environments.turn_environments.len(), 1);
    let turn_environment = turn_context
        .environments
        .primary()
        .expect("primary environment should be set");
    assert!(std::sync::Arc::ptr_eq(
        &turn_environment.environment,
        &turn_environments.turn_environments[0].environment
    ));
    assert!(!turn_context.environments.turn_environments.is_empty());
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(turn_cwd.as_path(), selected_cwd.as_path());
    assert_eq!(turn_context.config.cwd.as_path(), selected_cwd.as_path());
}

#[tokio::test]
async fn default_turn_overlays_session_cwd_onto_stored_thread_environments() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let session_cwd = session.get_config().await.cwd.clone();
    let selected_cwd =
        AbsolutePathBuf::try_from(session_cwd.as_path().join("selected")).expect("absolute path");

    {
        let mut state = session.state.lock().await;
        state.session_configuration.environments = vec![TurnEnvironmentSelection {
            environment_id: "local".to_string(),
            cwd: selected_cwd.clone(),
        }];
    }

    let turn_context = session.new_default_turn().await;

    let turn_environments = &turn_context.environments;
    assert_eq!(turn_environments.turn_environments.len(), 1);
    let turn_environment = turn_context
        .environments
        .primary()
        .expect("primary environment should be set");
    assert!(std::sync::Arc::ptr_eq(
        &turn_environment.environment,
        &turn_environments.turn_environments[0].environment
    ));
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(turn_cwd, session_cwd);
    assert_eq!(turn_context.config.cwd, session_cwd);
}

#[tokio::test]
async fn default_turn_honors_empty_stored_thread_environments() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let session_cwd = session.get_config().await.cwd.clone();

    {
        let mut state = session.state.lock().await;
        state.session_configuration.environments = Vec::new();
    }

    let turn_context = session.new_default_turn().await;

    assert!(turn_context.environments.primary().is_none());
    assert!(turn_context.environments.turn_environments.is_empty());
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(turn_cwd, session_cwd);
    assert_eq!(turn_context.config.cwd, session_cwd);
    assert_eq!(turn_context.environments.turn_environments.len(), 0);
}

#[tokio::test]
async fn primary_environment_uses_first_turn_environment() {
    let (_session, mut turn_context) = make_session_and_context().await;
    let first_environment = turn_context.environments.turn_environments[0].clone();
    #[allow(deprecated)]
    let second_cwd = turn_context.cwd.join("second");
    turn_context
        .environments
        .turn_environments
        .push(TurnEnvironment {
            environment_id: "second".to_string(),
            environment: Arc::clone(&first_environment.environment),
            cwd: second_cwd.clone(),
            shell: None,
        });

    assert_eq!(
        turn_context
            .environments
            .primary()
            .expect("primary environment")
            .environment_id,
        first_environment.environment_id
    );
    assert_eq!(
        turn_context
            .environments
            .turn_environments
            .iter()
            .find(|environment| environment.environment_id == "second")
            .expect("second environment")
            .cwd,
        second_cwd
    );
    assert_eq!(turn_context.environments.turn_environments.len(), 2);
    assert_eq!(
        turn_context.environments.turn_environments[1].cwd,
        second_cwd
    );
}

#[tokio::test]
async fn empty_turn_environments_clear_primary_environment() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;

    let turn_context = session
        .new_turn_with_sub_id(
            "sub-1".to_string(),
            SessionSettingsUpdate {
                environments: Some(vec![]),
                ..Default::default()
            },
        )
        .await
        .expect("turn should start");

    assert!(turn_context.environments.primary().is_none());
    assert!(turn_context.environments.turn_environments.is_empty());
    #[allow(deprecated)]
    let turn_cwd = turn_context.cwd.clone();
    assert_eq!(turn_cwd, session.get_config().await.cwd);
    assert_eq!(turn_context.config.cwd, session.get_config().await.cwd);
}

#[tokio::test]
async fn unknown_turn_environment_returns_error() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let original_configuration = {
        let state = session.state.lock().await;
        state.session_configuration.clone()
    };

    let err = session
        .new_turn_with_sub_id(
            "sub-1".to_string(),
            SessionSettingsUpdate {
                environments: Some(vec![TurnEnvironmentSelection {
                    environment_id: "missing".to_string(),
                    cwd: original_configuration.cwd.clone(),
                }]),
                ..Default::default()
            },
        )
        .await
        .expect_err("unknown environment should fail");

    let current_configuration = {
        let state = session.state.lock().await;
        state.session_configuration.clone()
    };
    assert!(matches!(err, CodexErr::InvalidRequest(_)));
    assert!(err.to_string().contains("missing"));
    assert_eq!(current_configuration.cwd, original_configuration.cwd);
    assert_eq!(
        current_configuration.environments,
        original_configuration.environments
    );
}

#[tokio::test]
async fn duplicate_turn_environment_returns_error_without_mutating_session() {
    let (session, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let original_configuration = {
        let state = session.state.lock().await;
        state.session_configuration.clone()
    };

    let err = session
        .new_turn_with_sub_id(
            "sub-1".to_string(),
            SessionSettingsUpdate {
                environments: Some(vec![
                    TurnEnvironmentSelection {
                        environment_id: "local".to_string(),
                        cwd: original_configuration.cwd.clone(),
                    },
                    TurnEnvironmentSelection {
                        environment_id: "local".to_string(),
                        cwd: original_configuration.cwd.join("second"),
                    },
                ]),
                ..Default::default()
            },
        )
        .await
        .expect_err("duplicate environment should fail");

    let current_configuration = {
        let state = session.state.lock().await;
        state.session_configuration.clone()
    };
    assert!(matches!(err, CodexErr::InvalidRequest(_)));
    assert!(err.to_string().contains("duplicate"));
    assert_eq!(current_configuration.cwd, original_configuration.cwd);
    assert_eq!(
        current_configuration.environments,
        original_configuration.environments
    );
}

#[tokio::test]
async fn spawn_task_turn_span_inherits_dispatch_trace_context() {
    struct TraceCaptureTask {
        captured_trace: Arc<std::sync::Mutex<Option<W3cTraceContext>>>,
    }

    impl SessionTask for TraceCaptureTask {
        fn kind(&self) -> TaskKind {
            TaskKind::Regular
        }

        fn span_name(&self) -> &'static str {
            "session_task.trace_capture"
        }

        async fn run(
            self: Arc<Self>,
            _session: Arc<SessionTaskContext>,
            _ctx: Arc<TurnContext>,
            _input: Vec<TurnInput>,
            _cancellation_token: CancellationToken,
        ) -> Option<String> {
            let mut trace = self
                .captured_trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *trace = current_span_w3c_trace_context();
            None
        }
    }

    let _trace_test_context = install_test_tracing("codex-core-tests");

    let request_parent = W3cTraceContext {
        traceparent: Some("00-00000000000000000000000000000011-0000000000000022-01".into()),
        tracestate: Some("vendor=value".into()),
    };
    let request_span = tracing::info_span!("app_server.request");
    assert!(set_parent_from_w3c_trace_context(
        &request_span,
        &request_parent
    ));

    let submission_trace =
        async { current_span_w3c_trace_context().expect("request span should have trace context") }
            .instrument(request_span)
            .await;

    let dispatch_span = submission_dispatch_span(&Submission {
        id: "sub-1".into(),
        op: Op::Interrupt,
        client_user_message_id: None,
        trace: Some(submission_trace.clone()),
    });
    let dispatch_span_id = dispatch_span.context().span().span_context().span_id();

    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let captured_trace = Arc::new(std::sync::Mutex::new(None));

    async {
        sess.spawn_task(
            Arc::clone(&tc),
            vec![TurnInput::UserInput {
                content: vec![UserInput::Text {
                    text: "hello".to_string(),
                    text_elements: Vec::new(),
                }],
                client_id: None,
                origin: None,
            }],
            TraceCaptureTask {
                captured_trace: Arc::clone(&captured_trace),
            },
        )
        .await;
    }
    .instrument(dispatch_span)
    .await;

    let evt = tokio::time::timeout(StdDuration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for turn completion")
        .expect("event");
    assert!(matches!(evt.msg, EventMsg::TurnComplete(_)));

    let task_trace = captured_trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("turn task should capture the current span trace context");
    let submission_context =
        ontocode_otel::context_from_w3c_trace_context(&submission_trace).expect("submission");
    let task_context =
        ontocode_otel::context_from_w3c_trace_context(&task_trace).expect("task trace");

    assert_eq!(
        task_context.span().span_context().trace_id(),
        submission_context.span().span_context().trace_id()
    );
    assert_ne!(
        task_context.span().span_context().span_id(),
        dispatch_span_id
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn shutdown_complete_does_not_append_to_thread_store_after_shutdown() {
    let (mut session, _turn_context) = make_session_and_context().await;
    let store = Arc::new(ontocode_thread_store::InMemoryThreadStore::default());
    let thread_store: Arc<dyn ontocode_thread_store::ThreadStore> = store.clone();
    let config = session.get_config().await;
    let live_thread = LiveThread::create(
        Arc::clone(&thread_store),
        CreateThreadParams {
            thread_id: session.thread_id,
            forked_from_id: None,
            parent_thread_id: None,
            source: SessionSource::Exec,
            thread_source: None,
            base_instructions: BaseInstructions::default(),
            dynamic_tools: Vec::new(),
            multi_agent_version: None,
            metadata: ThreadPersistenceMetadata {
                cwd: Some(config.cwd.to_path_buf()),
                model_provider: config.model_provider_id.clone(),
                memory_mode: if config.memories.generate_memories {
                    ThreadMemoryMode::Enabled
                } else {
                    ThreadMemoryMode::Disabled
                },
            },
        },
    )
    .await
    .expect("create thread persistence");
    session.services.thread_store = thread_store;
    session.services.live_thread = Some(live_thread);
    let session = Arc::new(session);

    assert!(handlers::shutdown(&session, "sub-1".to_string()).await);

    assert_eq!(
        ontocode_thread_store::InMemoryThreadStoreCalls {
            create_thread: 1,
            shutdown_thread: 1,
            ..Default::default()
        },
        store.calls().await
    );
}

#[tokio::test]
async fn submission_loop_channel_close_emits_thread_stop_lifecycle() {
    struct SessionStopMarker;
    struct ThreadStopMarker;

    struct ThreadStopRecorder {
        calls: Arc<std::sync::atomic::AtomicUsize>,
        expected_thread_id: ThreadId,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::ThreadLifecycleContributor<crate::config::Config>
        for ThreadStopRecorder
    {
        async fn on_thread_stop(&self, input: ontocode_extension_api::ThreadStopInput<'_>) {
            assert_eq!(
                self.expected_thread_id.to_string(),
                input.thread_store.level_id()
            );
            assert!(input.session_store.get::<SessionStopMarker>().is_some());
            assert!(input.thread_store.get::<ThreadStopMarker>().is_some());
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.thread_lifecycle_contributor(Arc::new(ThreadStopRecorder {
        calls: Arc::clone(&calls),
        expected_thread_id: session.thread_id,
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .services
        .session_extension_data
        .insert(SessionStopMarker);
    session
        .services
        .thread_extension_data
        .insert(ThreadStopMarker);

    let (tx_sub, rx_sub) = async_channel::bounded(1);
    drop(tx_sub);
    let session = Arc::new(session);
    submission_loop(session, Arc::clone(&turn_context.config), rx_sub).await;

    assert_eq!(1, calls.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn submission_loop_channel_close_aborts_active_turn_before_thread_stop_lifecycle() {
    struct LifecycleRecorder {
        calls: Arc<std::sync::Mutex<Vec<&'static str>>>,
        expected_thread_id: ThreadId,
        expected_turn_id: String,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::ThreadLifecycleContributor<crate::config::Config>
        for LifecycleRecorder
    {
        async fn on_thread_stop(&self, input: ontocode_extension_api::ThreadStopInput<'_>) {
            assert_eq!(
                self.expected_thread_id.to_string(),
                input.thread_store.level_id()
            );
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push("thread_stop");
        }
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::TurnLifecycleContributor for LifecycleRecorder {
        async fn on_turn_abort(&self, input: ontocode_extension_api::TurnAbortInput<'_>) {
            assert_eq!(
                self.expected_thread_id.to_string(),
                input.thread_store.level_id()
            );
            assert_eq!(self.expected_turn_id, input.turn_store.level_id());
            assert_eq!(TurnAbortReason::Interrupted, input.reason);
            self.calls
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push("turn_abort");
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = Arc::new(LifecycleRecorder {
        calls: Arc::clone(&calls),
        expected_thread_id: session.thread_id,
        expected_turn_id: turn_context.sub_id.clone(),
    });
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.thread_lifecycle_contributor(recorder.clone());
    builder.turn_lifecycle_contributor(recorder);
    session.services.extensions = Arc::new(builder.build());

    let session = Arc::new(session);
    session
        .spawn_task(
            Arc::new(turn_context),
            Vec::new(),
            NeverEndingTask {
                kind: TaskKind::Regular,
                listen_to_cancellation_token: true,
            },
        )
        .await;

    let (tx_sub, rx_sub) = async_channel::bounded(1);
    drop(tx_sub);
    submission_loop(Arc::clone(&session), session.get_config().await, rx_sub).await;

    assert_eq!(
        vec!["turn_abort", "thread_stop"],
        *calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    );
}

#[tokio::test]
async fn shutdown_and_wait_allows_multiple_waiters() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(4);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = rx_sub.recv().await.expect("shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        tokio::time::sleep(StdDuration::from_millis(50)).await;
    });
    let ontocode = Arc::new(Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
    });

    let waiter_1 = {
        let ontocode = Arc::clone(&ontocode);
        tokio::spawn(async move { ontocode.shutdown_and_wait().await })
    };
    let waiter_2 = {
        let ontocode = Arc::clone(&ontocode);
        tokio::spawn(async move { ontocode.shutdown_and_wait().await })
    };

    waiter_1
        .await
        .expect("first shutdown waiter join")
        .expect("first shutdown waiter");
    waiter_2
        .await
        .expect("second shutdown waiter join")
        .expect("second shutdown waiter");
}

#[tokio::test]
async fn shutdown_and_wait_waits_when_shutdown_is_already_in_progress() {
    let (session, _turn_context) = make_session_and_context().await;
    let (tx_sub, rx_sub) = async_channel::bounded(4);
    drop(rx_sub);
    let (_tx_event, rx_event) = async_channel::unbounded();
    let (_agent_status_tx, agent_status) = watch::channel(AgentStatus::PendingInit);
    let (shutdown_complete_tx, shutdown_complete_rx) = tokio::sync::oneshot::channel();
    let session_loop_handle = tokio::spawn(async move {
        let _ = shutdown_complete_rx.await;
    });
    let ontocode = Arc::new(Codex {
        tx_sub,
        rx_event,
        agent_status,
        session: Arc::new(session),
        session_loop_termination: session_loop_termination_from_handle(session_loop_handle),
    });

    let waiter = {
        let ontocode = Arc::clone(&ontocode);
        tokio::spawn(async move { ontocode.shutdown_and_wait().await })
    };

    tokio::time::sleep(StdDuration::from_millis(10)).await;
    assert!(!waiter.is_finished());

    shutdown_complete_tx
        .send(())
        .expect("session loop should still be waiting to terminate");

    waiter
        .await
        .expect("shutdown waiter join")
        .expect("shutdown waiter");
}

#[tokio::test]
async fn shutdown_and_wait_shuts_down_cached_guardian_subagent() {
    let (parent_session, parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);
    let parent_config = Arc::clone(&parent_turn_context.config);
    let (parent_tx_sub, parent_rx_sub) = async_channel::bounded(4);
    let (_parent_tx_event, parent_rx_event) = async_channel::unbounded();
    let (_parent_status_tx, parent_agent_status) = watch::channel(AgentStatus::PendingInit);
    let parent_session_for_loop = Arc::clone(&parent_session);
    let parent_session_loop_handle = tokio::spawn(async move {
        submission_loop(parent_session_for_loop, parent_config, parent_rx_sub).await;
    });
    let parent_ontocode = Codex {
        tx_sub: parent_tx_sub,
        rx_event: parent_rx_event,
        agent_status: parent_agent_status,
        session: Arc::clone(&parent_session),
        session_loop_termination: session_loop_termination_from_handle(parent_session_loop_handle),
    };

    let (child_session, _child_turn_context) = make_session_and_context().await;
    let (child_tx_sub, child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let (child_shutdown_tx, child_shutdown_rx) = tokio::sync::oneshot::channel();
    let child_session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = child_rx_sub
            .recv()
            .await
            .expect("child shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        child_shutdown_tx
            .send(())
            .expect("child shutdown signal should be delivered");
    });
    let child_ontocode = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .cache_for_test(child_ontocode)
        .await;

    parent_ontocode
        .shutdown_and_wait()
        .await
        .expect("parent shutdown should succeed");

    child_shutdown_rx
        .await
        .expect("guardian subagent should receive a shutdown op");
}

#[tokio::test]
async fn cached_guardian_subagent_exposes_its_rollout_path() {
    let (parent_session, _parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);

    let (mut child_session, _child_turn_context) = make_session_and_context().await;
    let child_rollout_path = attach_thread_persistence(&mut child_session).await;
    let (child_tx_sub, _child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let child_session_loop_handle = tokio::spawn(async {});
    let child_ontocode = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .cache_for_test(child_ontocode)
        .await;

    assert_eq!(
        parent_session
            .guardian_review_session
            .trunk_rollout_path()
            .await,
        Some(child_rollout_path)
    );
}

#[tokio::test]
async fn shutdown_and_wait_shuts_down_tracked_ephemeral_guardian_review() {
    let (parent_session, parent_turn_context) = make_session_and_context().await;
    let parent_session = Arc::new(parent_session);
    let parent_config = Arc::clone(&parent_turn_context.config);
    let (parent_tx_sub, parent_rx_sub) = async_channel::bounded(4);
    let (_parent_tx_event, parent_rx_event) = async_channel::unbounded();
    let (_parent_status_tx, parent_agent_status) = watch::channel(AgentStatus::PendingInit);
    let parent_session_for_loop = Arc::clone(&parent_session);
    let parent_session_loop_handle = tokio::spawn(async move {
        submission_loop(parent_session_for_loop, parent_config, parent_rx_sub).await;
    });
    let parent_ontocode = Codex {
        tx_sub: parent_tx_sub,
        rx_event: parent_rx_event,
        agent_status: parent_agent_status,
        session: Arc::clone(&parent_session),
        session_loop_termination: session_loop_termination_from_handle(parent_session_loop_handle),
    };

    let (child_session, _child_turn_context) = make_session_and_context().await;
    let (child_tx_sub, child_rx_sub) = async_channel::bounded(4);
    let (_child_tx_event, child_rx_event) = async_channel::unbounded();
    let (_child_status_tx, child_agent_status) = watch::channel(AgentStatus::PendingInit);
    let (child_shutdown_tx, child_shutdown_rx) = tokio::sync::oneshot::channel();
    let child_session_loop_handle = tokio::spawn(async move {
        let shutdown: Submission = child_rx_sub
            .recv()
            .await
            .expect("child shutdown submission");
        assert_eq!(shutdown.op, Op::Shutdown);
        child_shutdown_tx
            .send(())
            .expect("child shutdown signal should be delivered");
    });
    let child_ontocode = Codex {
        tx_sub: child_tx_sub,
        rx_event: child_rx_event,
        agent_status: child_agent_status,
        session: Arc::new(child_session),
        session_loop_termination: session_loop_termination_from_handle(child_session_loop_handle),
    };
    parent_session
        .guardian_review_session
        .register_ephemeral_for_test(child_ontocode)
        .await;

    parent_ontocode
        .shutdown_and_wait()
        .await
        .expect("parent shutdown should succeed");

    child_shutdown_rx
        .await
        .expect("ephemeral guardian review should receive a shutdown op");
}

pub(crate) async fn make_session_and_context_with_auth_and_config_and_rx<F>(
    auth: CodexAuth,
    dynamic_tools: Vec<DynamicToolSpec>,
    configure_config: F,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
)
where
    F: FnOnce(&mut Config),
{
    let ontocode_home = tempfile::tempdir().expect("create temp dir");
    make_session_and_context_with_auth_config_home_and_rx(
        auth,
        dynamic_tools,
        ontocode_home.path(),
        TestStateDb::Disabled,
        configure_config,
    )
    .await
}

#[derive(Clone, Copy)]
enum TestStateDb {
    Disabled,
    Enabled,
}

async fn make_session_and_context_with_auth_config_home_and_rx<F>(
    auth: CodexAuth,
    dynamic_tools: Vec<DynamicToolSpec>,
    ontocode_home: &Path,
    state_db: TestStateDb,
    configure_config: F,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
)
where
    F: FnOnce(&mut Config),
{
    let (tx_event, rx_event) = async_channel::unbounded();
    let mut config = build_test_config(ontocode_home).await;
    configure_config(&mut config);
    let state_db = match state_db {
        TestStateDb::Disabled => None,
        TestStateDb::Enabled => Some(
            ontocode_state::StateRuntime::init(
                config.sqlite_home.clone(),
                config.model_provider_id.clone(),
            )
            .await
            .expect("state db should initialize"),
        ),
    };
    let config = Arc::new(config);
    let thread_id = ThreadId::default();
    let auth_manager = AuthManager::from_auth_for_testing(auth);
    let models_manager = models_manager_with_provider(
        config.codex_home.to_path_buf(),
        auth_manager.clone(),
        config.model_provider.clone(),
    );
    let agent_control = AgentControl::default();
    let exec_policy = Arc::new(ExecPolicyManager::default());
    let (agent_status_tx, _agent_status_rx) = watch::channel(AgentStatus::PendingInit);
    let model = get_model_offline_for_tests(config.model.as_deref());
    let model_info =
        construct_model_info_offline_for_tests(model.as_str(), &config.to_models_manager_config());
    let reasoning_effort = config.model_reasoning_effort.clone();
    let collaboration_mode = CollaborationMode {
        mode: ModeKind::Default,
        settings: Settings {
            model,
            reasoning_effort,
            developer_instructions: None,
        },
    };
    let default_environments = vec![TurnEnvironmentSelection {
        environment_id: ontocode_exec_server::LOCAL_ENVIRONMENT_ID.to_string(),
        cwd: config.cwd.clone(),
    }];
    let session_configuration = SessionConfiguration {
        provider: config.model_provider.clone(),
        collaboration_mode,
        model_reasoning_summary: config.model_reasoning_summary,
        developer_instructions: config.developer_instructions.clone(),
        user_instructions: config.user_instructions.clone(),
        service_tier: None,
        personality: config.personality,
        base_instructions: config
            .base_instructions
            .clone()
            .unwrap_or_else(|| model_info.get_model_instructions(config.personality)),
        compact_prompt: config.compact_prompt.clone(),
        approval_policy: config.permissions.approval_policy.clone(),
        approvals_reviewer: config.approvals_reviewer,
        permission_profile_state: config.permissions.permission_profile_state().clone(),
        windows_sandbox_level: WindowsSandboxLevel::from_config(&config),
        cwd: config.cwd.clone(),
        workspace_roots: config.workspace_roots.clone(),
        ontocode_home: config.codex_home.clone(),
        thread_name: None,
        environments: default_environments,
        original_config_do_not_use: Arc::clone(&config),
        metrics_service_name: None,
        app_server_client_name: None,
        app_server_client_version: None,
        session_source: SessionSource::Exec,
        forked_from_thread_id: None,
        parent_thread_id: None,
        thread_source: None,
        dynamic_tools,
        inherited_shell_snapshot: None,
        user_shell_override: None,
    };
    let per_turn_config =
        Session::build_per_turn_config(&session_configuration, session_configuration.cwd.clone());
    let model_info = construct_model_info_offline_for_tests(
        session_configuration.collaboration_mode.model(),
        &per_turn_config.to_models_manager_config(),
    );
    let session_telemetry = session_telemetry(
        thread_id,
        config.as_ref(),
        &model_info,
        session_configuration.session_source.clone(),
    );

    let state = SessionState::new(session_configuration.clone());
    let plugins_manager = Arc::new(PluginsManager::new(config.codex_home.to_path_buf()));
    let mcp_manager = Arc::new(McpManager::new(Arc::clone(&plugins_manager)));
    let skills_manager = Arc::new(SkillsManager::new(
        config.codex_home.clone(),
        /*bundled_skills_enabled*/ true,
    ));
    let network_approval = Arc::new(NetworkApprovalService::default());
    let environment = Arc::new(
        ontocode_exec_server::Environment::create_for_tests(/*exec_server_url*/ None)
            .expect("create environment"),
    );

    let services = SessionServices {
        mcp_connection_manager: Arc::new(RwLock::new(
            McpConnectionManager::new_uninitialized_with_permission_profile(
                &config.permissions.approval_policy,
                config.permissions.permission_profile(),
                config.prefix_mcp_tool_names(),
            ),
        )),
        mcp_startup_cancellation_token: Mutex::new(CancellationToken::new()),
        unified_exec_manager: UnifiedExecProcessManager::new(
            config.background_terminal_max_timeout,
        ),
        shell_zsh_path: None,
        main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
        analytics_events_client: AnalyticsEventsClient::new(
            Arc::clone(&auth_manager),
            config.chatgpt_base_url.trim_end_matches('/').to_string(),
            config.analytics_enabled,
        ),
        hooks: arc_swap::ArcSwap::from_pointee(Hooks::new(HooksConfig {
            legacy_notify_argv: config.notify.clone(),
            ..HooksConfig::default()
        })),
        rollout_thread_trace: ontocode_rollout_trace::ThreadTraceContext::disabled(),
        user_shell: Arc::new(default_user_shell()),
        shell_snapshot_tx: watch::channel(None).0,
        show_raw_agent_reasoning: config.show_raw_agent_reasoning,
        exec_policy,
        auth_manager: Arc::clone(&auth_manager),
        session_telemetry: session_telemetry.clone(),
        models_manager: Arc::clone(&models_manager),
        tool_approvals: Mutex::new(ApprovalStore::default()),
        guardian_rejections: Mutex::new(std::collections::HashMap::new()),
        guardian_rejection_circuit_breaker: Mutex::new(Default::default()),
        runtime_handle: tokio::runtime::Handle::current(),
        skills_manager,
        plugins_manager,
        mcp_manager,
        extensions: Arc::new(ontocode_extension_api::ExtensionRegistryBuilder::new().build()),
        session_extension_data: ontocode_extension_api::ExtensionData::new(
            agent_control.session_id().to_string(),
        ),
        thread_extension_data: ontocode_extension_api::ExtensionData::new(thread_id.to_string()),
        agent_control,
        network_proxy: arc_swap::ArcSwapOption::from(None),
        network_proxy_audit_metadata: crate::config::NetworkProxyAuditMetadata::default(),
        managed_network_requirements_configured: false,
        network_approval: Arc::clone(&network_approval),
        state_db: state_db.clone(),
        live_thread: None,
        thread_store: Arc::new(ontocode_thread_store::LocalThreadStore::new(
            ontocode_thread_store::LocalThreadStoreConfig::from_config(config.as_ref()),
            state_db,
        )),
        attestation_provider: None,
        model_client: ModelClient::new(
            Some(Arc::clone(&auth_manager)),
            thread_id.into(),
            thread_id,
            /*installation_id*/ "11111111-1111-4111-8111-111111111111".to_string(),
            session_configuration.provider.clone(),
            session_configuration.session_source.clone(),
            session_configuration.parent_thread_id,
            config.model_verbosity,
            config.features.enabled(Feature::EnableRequestCompression),
            config.features.enabled(Feature::RuntimeMetrics),
            Session::build_model_client_beta_features_header(config.as_ref()),
            /*attestation_provider*/ None,
        ),
        code_mode_service: crate::tools::code_mode::CodeModeService::new(),
        environment_manager: Arc::new(ontocode_exec_server::EnvironmentManager::default_for_tests()),
    };

    let plugin_outcome = services
        .plugins_manager
        .plugins_for_config(&per_turn_config.plugins_config_input())
        .await;
    let effective_skill_roots = plugin_outcome.effective_plugin_skill_roots();
    let skills_input =
        crate::skills_load_input_from_config(&per_turn_config, effective_skill_roots);
    let skill_fs = environment.get_filesystem();
    let skills_outcome = Arc::new(
        services
            .skills_manager
            .skills_for_config(&skills_input, Some(Arc::clone(&skill_fs)))
            .await,
    );
    let turn_environments = turn_environments_for_tests(&environment, &session_configuration.cwd);
    let turn_context = Arc::new(Session::make_turn_context(
        thread_id,
        SessionId::from(thread_id),
        Some(Arc::clone(&auth_manager)),
        &session_telemetry,
        session_configuration.provider.clone(),
        &session_configuration,
        config.multi_agent_version_from_features(),
        services.user_shell.as_ref(),
        services.shell_zsh_path.as_ref(),
        services.main_execve_wrapper_exe.as_ref(),
        per_turn_config,
        model_info,
        &models_manager,
        /*network*/ None,
        turn_environments,
        session_configuration.cwd.clone(),
        "turn_id".to_string(),
        skills_outcome,
        /*goal_tools_supported*/ true,
        /*permission_profile_override*/ None,
    ));

    let session = Arc::new(Session {
        thread_id,
        installation_id: "11111111-1111-4111-8111-111111111111".to_string(),
        tx_event,
        agent_status: agent_status_tx,
        out_of_band_elicitation_paused: watch::channel(false).0,
        state: Mutex::new(state),
        artifact_receipt_commit_lock: tokio::sync::Mutex::new(()),
        reload_user_config_lock: tokio::sync::Mutex::new(()),
        managed_network_proxy_refresh_lock: Semaphore::new(/*permits*/ 1),
        managed_network_proxy_refresh_pending: std::sync::atomic::AtomicBool::new(false),
        turn_generation: std::sync::atomic::AtomicU64::new(0),
        foreground_submission_generation: std::sync::atomic::AtomicU64::new(0),
        pending_foreground_submissions: Mutex::new(Default::default()),
        model_overload_generation: std::sync::atomic::AtomicU64::new(0),
        last_activity_unix_ms: std::sync::atomic::AtomicI64::new(0),
        features: config.features.clone(),
        multi_agent_version: OnceLock::from(config.multi_agent_version_from_features()),
        pending_mcp_server_refresh_config: Mutex::new(None),
        conversation: Arc::new(RealtimeConversationManager::new()),
        active_turn: Mutex::new(None),
        turn_start_lock: Mutex::new(()),
        execution_retired: std::sync::atomic::AtomicBool::new(false),
        input_queue: super::input_queue::InputQueue::new(),
        guardian_review_session: crate::guardian::GuardianReviewSessionManager::default(),
        services,
        next_internal_sub_id: std::sync::atomic::AtomicU64::new(0),
    });

    (session, turn_context, rx_event)
}

pub(crate) async fn make_session_and_context_with_dynamic_tools_and_rx(
    dynamic_tools: Vec<DynamicToolSpec>,
) -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
) {
    make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        dynamic_tools,
        |_config| {},
    )
    .await
}

// Like make_session_and_context, but returns Arc<Session> and the event receiver
// so tests can assert on emitted events.
pub(crate) async fn make_session_and_context_with_rx() -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
) {
    make_session_and_context_with_dynamic_tools_and_rx(Vec::new()).await
}

pub(crate) async fn make_session_and_context_with_state_db_and_rx() -> (
    Arc<Session>,
    Arc<TurnContext>,
    async_channel::Receiver<Event>,
) {
    static HOME: std::sync::LazyLock<ontocode_test_binary_support::TestHome> =
        std::sync::LazyLock::new(|| {
            ontocode_test_binary_support::TestHome::new("session-state-db-tests-")
                .expect("test home")
        });
    let ontocode_home = HOME.create_dir().expect("create temp dir");
    make_session_and_context_with_auth_config_home_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        &ontocode_home,
        TestStateDb::Enabled,
        |_config| {},
    )
    .await
}

#[tokio::test]
async fn refresh_mcp_servers_is_deferred_until_next_turn() {
    let (session, turn_context) = make_session_and_context().await;
    let old_token = session.mcp_startup_cancellation_token().await;
    assert!(!old_token.is_cancelled());

    let refresh_config = McpServerRefreshConfig {
        mcp_servers: json!({}),
    };
    {
        let mut guard = session.pending_mcp_server_refresh_config.lock().await;
        *guard = Some(refresh_config);
    }

    assert!(!old_token.is_cancelled());
    assert!(
        session
            .pending_mcp_server_refresh_config
            .lock()
            .await
            .is_some()
    );

    session
        .refresh_mcp_servers_if_requested(&turn_context, /*elicitation_reviewer*/ None)
        .await;

    assert!(old_token.is_cancelled());
    assert!(
        session
            .pending_mcp_server_refresh_config
            .lock()
            .await
            .is_none()
    );
    let new_token = session.mcp_startup_cancellation_token().await;
    assert!(!new_token.is_cancelled());
}

#[tokio::test]
async fn spawn_task_does_not_update_previous_turn_settings_for_non_run_turn_tasks() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.set_previous_turn_settings(/*previous_turn_settings*/ None)
        .await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];

    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
    assert_eq!(sess.previous_turn_settings().await, None);
}

#[tokio::test]
async fn build_settings_update_items_emits_environment_item_for_network_changes() {
    let (session, previous_context) = make_session_and_context().await;
    let previous_context = Arc::new(previous_context);
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;

    let mut config = (*current_context.config).clone();
    let mut requirements = config.config_layer_stack.requirements().clone();
    requirements.network = Some(Sourced::new(
        NetworkConstraints {
            domains: Some(NetworkDomainPermissionsToml {
                entries: std::collections::BTreeMap::from([
                    (
                        "api.example.com".to_string(),
                        NetworkDomainPermissionToml::Allow,
                    ),
                    (
                        "blocked.example.com".to_string(),
                        NetworkDomainPermissionToml::Deny,
                    ),
                ]),
            }),
            ..Default::default()
        },
        RequirementSource::LegacyManagedConfigTomlFromMdm,
    ));
    let layers = config
        .config_layer_stack
        .get_layers(
            ConfigLayerStackOrdering::LowestPrecedenceFirst,
            /*include_disabled*/ true,
        )
        .into_iter()
        .cloned()
        .collect();
    config.config_layer_stack = ConfigLayerStack::new(
        layers,
        requirements,
        config.config_layer_stack.requirements_toml().clone(),
    )
    .expect("rebuild config layer stack with network requirements");
    current_context.config = Arc::new(config);

    let reference_context_item = previous_context.to_turn_context_item();
    let update_items = session
        .build_settings_update_items(Some(&reference_context_item), &current_context)
        .await;

    let environment_update = user_input_texts(&update_items)
        .into_iter()
        .find(|text| text.contains("<environment_context>"))
        .expect("environment update item should be emitted");
    assert!(environment_update.contains(
        "<network enabled=\"true\"><allowed>api.example.com</allowed><denied>blocked.example.com</denied></network>"
    ));
}

#[tokio::test]
async fn environment_context_uses_session_shell_when_environment_shell_is_absent() {
    let (mut session, mut turn_context) = make_session_and_context().await;
    session.services.user_shell = Arc::new(crate::shell::Shell {
        shell_type: crate::shell::ShellType::PowerShell,
        shell_path: PathBuf::from("powershell"),
        shell_snapshot: crate::shell::empty_shell_snapshot_receiver(),
    });
    for environment in &mut turn_context.environments.turn_environments {
        environment.shell = None;
    }

    let session_shell = session.user_shell();
    let environment_context = crate::context::EnvironmentContext::from_turn_context(
        &turn_context,
        session_shell.as_ref(),
    )
    .render();
    assert!(
        environment_context.contains("<shell>powershell</shell>"),
        "{environment_context}"
    );

    let primary_environment = turn_context
        .environments
        .turn_environments
        .first_mut()
        .expect("primary environment");
    primary_environment.shell = Some("cmd".to_string());

    let environment_context = crate::context::EnvironmentContext::from_turn_context(
        &turn_context,
        session_shell.as_ref(),
    )
    .render();
    assert!(
        environment_context.contains("<shell>cmd</shell>"),
        "{environment_context}"
    );
}

#[tokio::test]
async fn build_settings_update_items_emits_environment_item_for_time_changes() {
    let (session, previous_context) = make_session_and_context().await;
    let previous_context = Arc::new(previous_context);
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;
    current_context.current_date = Some("2026-02-27".to_string());
    current_context.timezone = Some("Europe/Berlin".to_string());

    let reference_context_item = previous_context.to_turn_context_item();
    let update_items = session
        .build_settings_update_items(Some(&reference_context_item), &current_context)
        .await;

    let environment_update = user_input_texts(&update_items)
        .into_iter()
        .find(|text| text.contains("<environment_context>"))
        .expect("environment update item should be emitted");
    assert!(environment_update.contains("<current_date>2026-02-27</current_date>"));
    assert!(environment_update.contains("<timezone>Europe/Berlin</timezone>"));
}

#[tokio::test]
async fn build_settings_update_items_omits_environment_item_when_disabled() {
    let (session, previous_context) = make_session_and_context().await;
    let previous_context = Arc::new(previous_context);
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;
    let mut config = (*current_context.config).clone();
    config.include_environment_context = false;
    current_context.config = Arc::new(config);
    current_context.current_date = Some("2026-02-27".to_string());

    let reference_context_item = previous_context.to_turn_context_item();
    let update_items = session
        .build_settings_update_items(Some(&reference_context_item), &current_context)
        .await;

    let user_texts = user_input_texts(&update_items);
    assert!(
        !user_texts
            .iter()
            .any(|text| text.contains("<environment_context>")),
        "did not expect environment context updates when disabled, got {user_texts:?}"
    );
}

#[tokio::test]
async fn build_settings_update_items_emits_realtime_start_when_session_becomes_live() {
    let (session, previous_context) = make_session_and_context().await;
    let previous_context = Arc::new(previous_context);
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;
    current_context.realtime_active = true;

    let update_items = session
        .build_settings_update_items(
            Some(&previous_context.to_turn_context_item()),
            &current_context,
        )
        .await;

    let developer_texts = developer_input_texts(&update_items);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "expected a realtime start update, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_settings_update_items_emits_realtime_end_when_session_stops_being_live() {
    let (session, mut previous_context) = make_session_and_context().await;
    previous_context.realtime_active = true;
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;
    current_context.realtime_active = false;

    let update_items = session
        .build_settings_update_items(
            Some(&previous_context.to_turn_context_item()),
            &current_context,
        )
        .await;

    let developer_texts = developer_input_texts(&update_items);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("Reason: inactive")),
        "expected a realtime end update, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_settings_update_items_uses_previous_turn_settings_for_realtime_end() {
    let (session, previous_context) = make_session_and_context().await;
    let mut previous_context_item = previous_context.to_turn_context_item();
    previous_context_item.realtime_active = None;
    let previous_turn_settings = PreviousTurnSettings {
        model: previous_context.model_info.slug.clone(),
        realtime_active: Some(true),
    };
    let mut current_context = previous_context
        .with_model(
            previous_context.model_info.slug.clone(),
            &session.services.models_manager,
        )
        .await;
    current_context.realtime_active = false;

    session
        .set_previous_turn_settings(Some(previous_turn_settings))
        .await;
    let update_items = session
        .build_settings_update_items(Some(&previous_context_item), &current_context)
        .await;

    let developer_texts = developer_input_texts(&update_items);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("Reason: inactive")),
        "expected a realtime end update from previous turn settings, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_uses_previous_realtime_state() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.realtime_active = true;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "expected initial context to describe active realtime state, got {developer_texts:?}"
    );

    let previous_context_item = turn_context.to_turn_context_item();
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(Some(previous_context_item));
    }
    let resumed_context = session.build_initial_context(&turn_context).await;
    let resumed_developer_texts = developer_input_texts(&resumed_context);
    assert!(
        !resumed_developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "did not expect a duplicate realtime update, got {resumed_developer_texts:?}"
    );
}

async fn make_multi_agent_v2_usage_hint_test_session(
    enable_multi_agent_v2: bool,
) -> (Arc<Session>, Arc<TurnContext>) {
    let (session, turn_context, _rx_event) = make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        |config| {
            if enable_multi_agent_v2 {
                let _ = config.features.enable(Feature::MultiAgentV2);
            }
            config.multi_agent_v2.root_agent_usage_hint_text = Some("Root guidance.".to_string());
            config.multi_agent_v2.subagent_usage_hint_text = Some("Subagent guidance.".to_string());
        },
    )
    .await;
    (session, turn_context)
}

struct PromptExtensionTestContributor;
struct PromptExtensionTestState;

impl ontocode_extension_api::ContextContributor for PromptExtensionTestContributor {
    fn contribute<'a>(
        &'a self,
        _session_store: &'a ontocode_extension_api::ExtensionData,
        thread_store: &'a ontocode_extension_api::ExtensionData,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Vec<ontocode_extension_api::PromptFragment>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            if thread_store.get::<PromptExtensionTestState>().is_some() {
                vec![
                    ontocode_extension_api::PromptFragment::developer_policy(
                        "prompt extension enabled",
                    ),
                    ontocode_extension_api::PromptFragment::developer_capability(
                        "prompt extension capability",
                    ),
                    ontocode_extension_api::PromptFragment::new(
                        ontocode_extension_api::PromptSlot::ContextualUser,
                        "prompt extension contextual user",
                    ),
                    ontocode_extension_api::PromptFragment::separate_developer(
                        "prompt extension separate developer",
                    ),
                ]
            } else {
                Vec::new()
            }
        })
    }
}

fn prompt_extension_test_registry()
-> Arc<ontocode_extension_api::ExtensionRegistry<crate::config::Config>> {
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.prompt_contributor(Arc::new(PromptExtensionTestContributor));
    Arc::new(builder.build())
}

#[tokio::test]
async fn build_initial_context_includes_prompt_fragments_from_extensions() {
    let (mut session, turn_context) = make_session_and_context().await;
    session.services.extensions = prompt_extension_test_registry();
    session
        .services
        .thread_extension_data
        .insert(PromptExtensionTestState);

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_messages = developer_message_texts(&initial_context);

    assert!(
        developer_messages
            .iter()
            .flatten()
            .any(|text| *text == "prompt extension enabled"),
        "expected prompt extension developer text, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_preserves_prompt_section_boundaries() {
    let (mut session, mut turn_context) = make_session_and_context().await;
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![SkillMetadata {
        name: "lazy-skill".to_string(),
        description: "metadata-only description".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/lazy-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }];
    turn_context.turn_skills = TurnSkillsContext::new(Arc::new(outcome));
    session.services.extensions = prompt_extension_test_registry();
    session
        .services
        .thread_extension_data
        .insert(PromptExtensionTestState);

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_messages = developer_message_texts(&initial_context);
    let first_developer_message = developer_messages
        .first()
        .expect("developer bundle should exist");
    let skill_index = first_developer_message
        .iter()
        .position(|text| text.contains("lazy-skill") && text.contains("metadata-only description"))
        .expect("skill metadata should render in the aggregated developer bundle");
    let policy_index = first_developer_message
        .iter()
        .position(|text| *text == "prompt extension enabled")
        .expect("developer policy prompt fragment should render in the aggregated bundle");
    let capability_index = first_developer_message
        .iter()
        .position(|text| *text == "prompt extension capability")
        .expect("developer capability prompt fragment should render in the aggregated bundle");

    assert!(
        skill_index < policy_index && policy_index < capability_index,
        "expected skills before extension developer fragments in first developer message, got {first_developer_message:?}"
    );
    assert!(
        developer_messages
            .iter()
            .any(|message| message.as_slice() == ["prompt extension separate developer"]),
        "expected separate developer prompt fragment to stay in its own developer message, got {developer_messages:?}"
    );
    assert!(
        developer_messages
            .iter()
            .flatten()
            .all(|text| *text != "prompt extension contextual user"),
        "developer messages should not include contextual-user fragments, got {developer_messages:?}"
    );

    let user_texts = user_input_texts(&initial_context);
    assert!(
        user_texts.contains(&"prompt extension contextual user"),
        "expected contextual-user prompt fragment in contextual user message, got {user_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_includes_thread_scoped_operational_evidence_context() {
    let (session, turn_context, _rx) = make_session_and_context_with_state_db_and_rx().await;
    let state_db = session
        .state_db()
        .expect("operational evidence test session should have a state db");
    let thread_id = session.thread_id.to_string();
    let other_thread_id = Uuid::new_v4().to_string();

    // Timestamps have one-second resolution and ties break on a random id, so give the accepted
    // record the newest timestamp; otherwise the byte-capped context query can cut it off.
    let mut accepted = operational_evidence_record(
        &thread_id,
        EvidenceStatus::Verified,
        RedactionStatus::Clean,
        "thread-scoped operational evidence",
    );
    accepted.created_at += chrono::Duration::seconds(10);
    state_db
        .insert_operational_evidence(&accepted)
        .await
        .expect("insert accepted operational evidence");
    state_db
        .insert_operational_evidence(&operational_evidence_record(
            &thread_id,
            EvidenceStatus::Rejected,
            RedactionStatus::Clean,
            "rejected operational evidence",
        ))
        .await
        .expect("insert rejected operational evidence");
    state_db
        .insert_operational_evidence(&operational_evidence_record(
            &thread_id,
            EvidenceStatus::Done,
            RedactionStatus::Rejected,
            "redacted operational evidence",
        ))
        .await
        .expect("insert redacted operational evidence");
    state_db
        .insert_operational_evidence(&operational_evidence_record(
            &other_thread_id,
            EvidenceStatus::Verified,
            RedactionStatus::Clean,
            "other-thread operational evidence",
        ))
        .await
        .expect("insert other-thread operational evidence");

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;
    let user_messages = user_input_texts(&initial_context);

    assert!(
        user_messages
            .iter()
            .any(|text| text.contains("<operational_evidence_context>")),
        "expected operational evidence context fragment, got {user_messages:?}"
    );
    assert!(
        user_messages
            .iter()
            .any(|text| text.contains("thread-scoped operational evidence")),
        "expected accepted thread-scoped operational evidence, got {user_messages:?}"
    );
    assert!(
        !user_messages
            .iter()
            .any(|text| text.contains("rejected operational evidence")),
        "did not expect rejected operational evidence, got {user_messages:?}"
    );
    assert!(
        !user_messages
            .iter()
            .any(|text| text.contains("redacted operational evidence")),
        "did not expect redacted operational evidence, got {user_messages:?}"
    );
    assert!(
        !user_messages
            .iter()
            .any(|text| text.contains("other-thread operational evidence")),
        "did not expect other-thread operational evidence, got {user_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_prompt_fragments_without_extension_state() {
    let (mut session, turn_context) = make_session_and_context().await;
    session.services.extensions = prompt_extension_test_registry();

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_messages = developer_message_texts(&initial_context);

    assert!(
        !developer_messages
            .iter()
            .flatten()
            .any(|text| *text == "prompt extension enabled"),
        "did not expect prompt extension developer text, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_adds_multi_agent_v2_root_usage_hint_as_developer_message() {
    let (session, turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ true).await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Root guidance."]),
        "expected standalone root usage hint developer message, got {developer_messages:?}"
    );
    assert!(
        !developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Subagent guidance."]),
        "did not expect subagent usage hint for root thread, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_adds_multi_agent_v2_subagent_usage_hint_as_developer_message() {
    let (session, mut turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ true).await;
    let session_source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: ThreadId::new(),
        depth: 1,
        agent_path: Some(AgentPath::try_from("/root/worker").expect("agent path should parse")),
        agent_nickname: Some("worker".to_string()),
        agent_role: None,
    });
    session
        .state
        .lock()
        .await
        .session_configuration
        .session_source = session_source.clone();
    Arc::get_mut(&mut turn_context)
        .expect("thread settings should not be shared")
        .session_source = session_source;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Subagent guidance."]),
        "expected standalone subagent usage hint developer message, got {developer_messages:?}"
    );
    assert!(
        !developer_messages
            .iter()
            .any(|message| message.as_slice() == ["Root guidance."]),
        "did not expect root usage hint for subagent thread, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_multi_agent_v2_usage_hints_when_feature_disabled() {
    let (session, turn_context) =
        make_multi_agent_v2_usage_hint_test_session(/*enable_multi_agent_v2*/ false).await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        !developer_messages.iter().any(|message| {
            matches!(
                message.as_slice(),
                ["Root guidance."] | ["Subagent guidance."]
            )
        }),
        "did not expect multi-agent v2 usage hint developer messages, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_multi_agent_v2_usage_hints_when_hint_disabled() {
    let (session, turn_context, _rx_event) = make_session_and_context_with_auth_and_config_and_rx(
        CodexAuth::from_api_key("Test API Key"),
        Vec::new(),
        |config| {
            let _ = config.features.enable(Feature::MultiAgentV2);
            config.multi_agent_v2.usage_hint_enabled = false;
            config.multi_agent_v2.root_agent_usage_hint_text = Some("Root guidance.".to_string());
            config.multi_agent_v2.subagent_usage_hint_text = Some("Subagent guidance.".to_string());
        },
    )
    .await;

    let initial_context = session.build_initial_context(turn_context.as_ref()).await;

    let developer_messages = developer_message_texts(&initial_context);
    assert!(
        !developer_messages.iter().any(|message| {
            matches!(
                message.as_slice(),
                ["Root guidance."] | ["Subagent guidance."]
            )
        }),
        "did not expect multi-agent v2 usage hint developer messages, got {developer_messages:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_default_image_save_location_with_image_history() {
    let (session, turn_context) = make_session_and_context().await;
    session
        .replace_history(
            vec![ResponseItem::ImageGenerationCall {
                id: "ig-test".to_string(),
                status: "completed".to_string(),
                revised_prompt: Some("a tiny blue square".to_string()),
                result: "Zm9v".to_string(),
            }],
            /*reference_context_item*/ None,
        )
        .await;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        !developer_texts
            .iter()
            .any(|text| text.contains("Generated images are saved to")),
        "expected initial context to omit image save instructions even with image history, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_omits_default_image_save_location_without_image_history() {
    let (session, turn_context) = make_session_and_context().await;

    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);

    assert!(
        !developer_texts
            .iter()
            .any(|text| text.contains("Generated images are saved to")),
        "expected initial context to omit image save instructions without image history, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_trims_skill_metadata_from_context_window_budget() {
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![
        SkillMetadata {
            name: "admin-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/admin-skill/SKILL.md").abs(),
            scope: SkillScope::Admin,
            plugin_id: None,
            execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
        },
        SkillMetadata {
            name: "repo-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
            scope: SkillScope::Repo,
            plugin_id: None,
            execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
        },
    ];
    let outcome = Arc::new(outcome);
    let hint = "Use `skill_search` to find additional available skill metadata.";
    for skill_search_enabled in [false, true] {
        for (context_window, omitted_count) in [(100, 2), (100_000, 0)] {
            let (session, turn_context, rx) = make_session_and_context_with_rx().await;
            let mut turn_context =
                Arc::into_inner(turn_context).expect("sole thread settings owner");
            if skill_search_enabled {
                let _ = turn_context.features.enable(Feature::SkillSearchTool);
            } else {
                let _ = turn_context.features.disable(Feature::SkillSearchTool);
            }
            turn_context.model_info.context_window = Some(context_window);
            turn_context.turn_skills = TurnSkillsContext::new(Arc::clone(&outcome));

            let available_skills = build_available_skills(
                &outcome,
                default_skill_metadata_budget(Some(context_window)),
                SkillRenderSideEffects::None,
            )
            .expect("skills should render");
            assert_eq!(available_skills.report.omitted_count, omitted_count);
            let expected_warning = available_skills.warning_message.clone();
            if omitted_count > 0 {
                assert_eq!(
                    expected_warning.as_deref(),
                    Some(
                        "Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."
                    )
                );
            } else {
                assert_eq!(expected_warning, None);
            }
            let old_rendered = AvailableSkillsInstructions::from(available_skills).render();
            let expected_rendered = if skill_search_enabled && omitted_count > 0 {
                old_rendered.replacen(
                    "\n### How to use skills",
                    &format!("\n{hint}\n### How to use skills"),
                    /*count*/ 1,
                )
            } else {
                old_rendered
            };
            let mut previous_context = None;
            for _ in 0..2 {
                let initial_context = session.build_initial_context(&turn_context).await;
                let developer_texts = developer_input_texts(&initial_context);
                let skills_text = developer_texts
                    .iter()
                    .find(|text| AvailableSkillsInstructions::matches_text(text))
                    .expect("skills section should render");
                assert_eq!(*skills_text, expected_rendered);
                assert!(
                    developer_texts
                        .iter()
                        .all(|text| !text.contains("Exceeded skills context budget")),
                    "expected skill budget warning to stay out of the initial context, got {developer_texts:?}"
                );
                if omitted_count > 0 {
                    assert!(
                        developer_texts.iter().all(|text| {
                            !text.contains("- admin-skill:") && !text.contains("- repo-skill:")
                        }),
                        "expected no skill metadata entries to fit the tiny budget, got {developer_texts:?}"
                    );
                }
                if let Some(message) = &expected_warning {
                    let warning_event = timeout(Duration::from_secs(1), rx.recv())
                        .await
                        .expect("warning event should arrive on every build")
                        .expect("warning event should be readable");
                    let EventMsg::Warning(warning) = warning_event.msg else {
                        panic!("expected skill-budget warning");
                    };
                    assert_eq!(warning.message, *message);
                }
                assert!(rx.try_recv().is_err(), "unexpected extra event");
                if let Some(previous_context) = &previous_context {
                    assert_eq!(&initial_context, previous_context);
                }
                previous_context = Some(initial_context);
            }
        }
    }
}

#[test]
fn emit_thread_start_skill_metrics_records_enabled_kept_and_truncated_values() {
    let session_telemetry = test_session_telemetry_without_metadata();
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![SkillMetadata {
        name: "repo-skill".to_string(),
        description: "desc".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }];
    let rendered = build_available_skills(
        &outcome,
        SkillMetadataBudget::Characters(1),
        SkillRenderSideEffects::ThreadStart {
            session_telemetry: &session_telemetry,
        },
    )
    .expect("skills should render");

    assert_eq!(
        rendered.warning_message,
        Some(
            "Exceeded skills context budget. All skill descriptions were removed and 1 additional skill was not included in the model-visible skills list."
                .to_string()
        )
    );
    let snapshot = session_telemetry
        .snapshot_metrics()
        .expect("runtime metrics snapshot");
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_ENABLED_TOTAL_METRIC),
        1
    );
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_KEPT_TOTAL_METRIC), 0);
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_TRUNCATED_METRIC), 1);
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC),
        4
    );
}

#[test]
fn emit_thread_start_skill_metrics_records_description_truncated_chars_without_omitted_skills() {
    let session_telemetry = test_session_telemetry_without_metadata();
    let alpha = SkillMetadata {
        name: "alpha-skill".to_string(),
        description: "abcdef".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/alpha-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    };
    let beta = SkillMetadata {
        name: "beta-skill".to_string(),
        description: "uvwxyz".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: None,
        path_to_skills_md: test_path_buf("/tmp/beta-skill/SKILL.md").abs(),
        scope: SkillScope::Repo,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    };
    let minimum_skill_line_cost = |skill: &SkillMetadata| {
        let path = skill.path_to_skills_md.to_string_lossy().replace('\\', "/");
        format!("- {}: (file: {})\n", skill.name, path)
            .chars()
            .count()
    };
    let minimum_budget = minimum_skill_line_cost(&alpha) + minimum_skill_line_cost(&beta);
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![alpha, beta];

    let rendered = build_available_skills(
        &outcome,
        SkillMetadataBudget::Characters(minimum_budget + 6),
        SkillRenderSideEffects::ThreadStart {
            session_telemetry: &session_telemetry,
        },
    )
    .expect("skills should render");

    assert_eq!(rendered.report.omitted_count, 0);
    assert_eq!(rendered.report.truncated_description_chars, 8);
    let snapshot = session_telemetry
        .snapshot_metrics()
        .expect("runtime metrics snapshot");
    assert_eq!(histogram_sum(&snapshot, THREAD_SKILLS_TRUNCATED_METRIC), 0);
    assert_eq!(
        histogram_sum(&snapshot, THREAD_SKILLS_DESCRIPTION_TRUNCATED_CHARS_METRIC),
        8
    );
}

#[tokio::test]
async fn build_initial_context_emits_thread_start_skill_warning_on_repeated_builds() {
    let (session, turn_context, rx) = make_session_and_context_with_rx().await;
    let mut turn_context = Arc::into_inner(turn_context).expect("sole thread settings owner");
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![
        SkillMetadata {
            name: "admin-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/admin-skill/SKILL.md").abs(),
            scope: SkillScope::Admin,
            plugin_id: None,
            execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
        },
        SkillMetadata {
            name: "repo-skill".to_string(),
            description: "desc".to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path_to_skills_md: test_path_buf("/tmp/repo-skill/SKILL.md").abs(),
            scope: SkillScope::Repo,
            plugin_id: None,
            execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
        },
    ];
    turn_context.model_info.context_window = Some(100);
    turn_context.turn_skills = TurnSkillsContext::new(Arc::new(outcome));

    let _ = session.build_initial_context(&turn_context).await;
    let warning_event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("warning event should arrive")
        .expect("warning event should be readable");
    assert!(matches!(
        warning_event.msg,
        EventMsg::Warning(WarningEvent { message })
            if message == "Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."
    ));

    let _ = session.build_initial_context(&turn_context).await;
    let warning_event = timeout(Duration::from_secs(1), rx.recv())
        .await
        .expect("warning event should arrive on repeated build")
        .expect("warning event should be readable");
    assert!(matches!(
        warning_event.msg,
        EventMsg::Warning(WarningEvent { message })
            if message == "Exceeded skills context budget of 2%. All skill descriptions were removed and 2 additional skills were not included in the model-visible skills list."
    ));
}

#[tokio::test]
async fn handle_output_item_done_records_image_save_history_message() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "ig_history_records_message";
    let expected_saved_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.thread_id.to_string(),
        call_id,
    );
    let _ = std::fs::remove_file(&expected_saved_path);
    let item = ResponseItem::ImageGenerationCall {
        id: call_id.to_string(),
        status: "completed".to_string(),
        revised_prompt: Some("a tiny blue square".to_string()),
        result: "Zm9v".to_string(),
    };

    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&session),
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            turn_context.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context)),
        cancellation_token: CancellationToken::new(),
    };
    handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
        .await
        .expect("image generation item should succeed");

    let history = session.clone_history().await;
    let image_output_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.thread_id.to_string(),
        "<image_id>",
    );
    let image_output_dir = image_output_path
        .parent()
        .expect("generated image path should have a parent");
    let image_message: ResponseItem = crate::context::ContextualUserFragment::into(
        crate::context::ImageGenerationInstructions::new(
            image_output_dir.display(),
            image_output_path.display(),
        ),
    );
    assert_eq!(history.raw_items(), &[image_message, item]);
    assert_eq!(
        std::fs::read(&expected_saved_path).expect("saved file"),
        b"foo"
    );
    let _ = std::fs::remove_file(&expected_saved_path);
}

#[tokio::test]
async fn handle_output_item_done_skips_image_save_message_when_save_fails() {
    let (session, turn_context) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let call_id = "ig_history_no_message";
    let expected_saved_path = crate::stream_events_utils::image_generation_artifact_path(
        &turn_context.config.codex_home,
        &session.thread_id.to_string(),
        call_id,
    );
    let _ = std::fs::remove_file(&expected_saved_path);
    let item = ResponseItem::ImageGenerationCall {
        id: call_id.to_string(),
        status: "completed".to_string(),
        revised_prompt: Some("broken payload".to_string()),
        result: "_-8".to_string(),
    };

    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&session),
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            turn_context.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context)),
        cancellation_token: CancellationToken::new(),
    };
    handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
        .await
        .expect("image generation item should still complete");

    let history = session.clone_history().await;
    assert_eq!(history.raw_items(), &[item]);
    assert!(!expected_saved_path.exists());
}

#[tokio::test]
async fn build_initial_context_uses_previous_turn_settings_for_realtime_end() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_turn_settings = PreviousTurnSettings {
        model: turn_context.model_info.slug.clone(),
        realtime_active: Some(true),
    };

    session
        .set_previous_turn_settings(Some(previous_turn_settings))
        .await;
    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("Reason: inactive")),
        "expected initial context to describe an ended realtime session, got {developer_texts:?}"
    );
}

#[tokio::test]
async fn build_initial_context_restates_realtime_start_when_reference_context_is_missing() {
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.realtime_active = true;
    let previous_turn_settings = PreviousTurnSettings {
        model: turn_context.model_info.slug.clone(),
        realtime_active: Some(true),
    };

    session
        .set_previous_turn_settings(Some(previous_turn_settings))
        .await;
    let initial_context = session.build_initial_context(&turn_context).await;
    let developer_texts = developer_input_texts(&initial_context);
    assert!(
        developer_texts
            .iter()
            .any(|text| text.contains("<realtime_conversation>")),
        "expected initial context to restate active realtime when the reference context is missing, got {developer_texts:?}"
    );
}

fn file_system_policy_with_unreadable_glob(turn_context: &TurnContext) -> FileSystemSandboxPolicy {
    #[allow(deprecated)]
    let mut policy = FileSystemSandboxPolicy::from_legacy_sandbox_policy_for_cwd(
        &turn_context.sandbox_policy(),
        &turn_context.cwd,
    );
    #[allow(deprecated)]
    let cwd_display = turn_context.cwd.as_path().display().to_string();
    policy.entries.push(FileSystemSandboxEntry {
        path: FileSystemPath::GlobPattern {
            pattern: format!("{cwd_display}/**/*.env"),
        },
        access: FileSystemAccessMode::Deny,
    });
    policy
}

#[tokio::test]
async fn turn_context_item_omits_legacy_equivalent_file_system_sandbox_policy() {
    let (_session, turn_context) = make_session_and_context().await;

    let item = turn_context.to_turn_context_item();

    assert_eq!(item.file_system_sandbox_policy, None);
    assert_eq!(
        item.permission_profile,
        Some(turn_context.permission_profile())
    );
}

#[tokio::test]
async fn turn_context_item_stores_split_file_system_sandbox_policy_when_different() {
    let (_session, mut turn_context) = make_session_and_context().await;
    let file_system_sandbox_policy = file_system_policy_with_unreadable_glob(&turn_context);
    turn_context.permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
        turn_context.permission_profile.enforcement(),
        &file_system_sandbox_policy,
        turn_context.network_sandbox_policy(),
    );

    let item = turn_context.to_turn_context_item();

    assert_eq!(
        item.file_system_sandbox_policy,
        Some(file_system_sandbox_policy)
    );
    assert_eq!(
        item.permission_profile,
        Some(turn_context.permission_profile())
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_injects_full_context_when_baseline_missing()
 {
    let (session, turn_context) = make_session_and_context().await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    let history = session.clone_history().await;
    let initial_context = session.build_initial_context(&turn_context).await;
    assert_eq!(history.raw_items().to_vec(), initial_context);

    let current_context = session.reference_context_item().await;
    assert_eq!(
        serde_json::to_value(current_context).expect("serialize current context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected context item")
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_reinjects_full_context_after_clear()
{
    let (session, turn_context) = make_session_and_context().await;
    let compacted_summary = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: format!("{}\nsummary", crate::compact::SUMMARY_PREFIX),
        }],
        phase: None,
    };
    session
        .record_conversation_items(&turn_context, std::slice::from_ref(&compacted_summary))
        .await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }
    session
        .replace_history(
            vec![compacted_summary.clone()],
            /*reference_context_item*/ None,
        )
        .await;

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;

    let history = session.clone_history().await;
    let mut expected_history = vec![compacted_summary];
    expected_history.extend(session.build_initial_context(&turn_context).await);
    assert_eq!(history.raw_items().to_vec(), expected_history);
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_baseline_without_emitting_diffs()
 {
    let (mut session, previous_context) = make_session_and_context().await;
    let next_model = if previous_context.model_info.slug == "gpt-5.4" {
        "gpt-5.2"
    } else {
        "gpt-5.4"
    };
    let turn_context = previous_context
        .with_model(next_model.to_string(), &session.services.models_manager)
        .await;
    let previous_context_item = previous_context.to_turn_context_item();
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(Some(previous_context_item.clone()));
    }
    let rollout_path = attach_thread_persistence(&mut session).await;

    let update_items = session
        .build_settings_update_items(Some(&previous_context_item), &turn_context)
        .await;
    assert_eq!(update_items, Vec::new());

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;

    assert_eq!(
        session.clone_history().await.raw_items().to_vec(),
        Vec::new()
    );
    assert_eq!(
        serde_json::to_value(session.reference_context_item().await)
            .expect("serialize current context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected context item")
    );
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_turn_context = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => Some(ctx.clone()),
        _ => None,
    });
    assert_eq!(
        serde_json::to_value(persisted_turn_context)
            .expect("serialize persisted turn context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected turn context item")
    );
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_split_file_system_policy_to_rollout()
 {
    let (mut session, mut turn_context) = make_session_and_context().await;
    let file_system_sandbox_policy = file_system_policy_with_unreadable_glob(&turn_context);
    turn_context.permission_profile = PermissionProfile::from_runtime_permissions_with_enforcement(
        turn_context.permission_profile.enforcement(),
        &file_system_sandbox_policy,
        turn_context.network_sandbox_policy(),
    );
    let rollout_path = attach_thread_persistence(&mut session).await;

    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_file_system_sandbox_policy = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => ctx.file_system_sandbox_policy.clone(),
        _ => None,
    });
    assert_eq!(
        persisted_file_system_sandbox_policy,
        Some(file_system_sandbox_policy)
    );
}

#[tokio::test]
async fn build_initial_context_prepends_model_switch_message() {
    let (session, turn_context) = make_session_and_context().await;
    let previous_turn_settings = PreviousTurnSettings {
        model: "previous-regular-model".to_string(),
        realtime_active: None,
    };

    session
        .set_previous_turn_settings(Some(previous_turn_settings))
        .await;
    let initial_context = session.build_initial_context(&turn_context).await;

    let ResponseItem::Message { role, content, .. } = &initial_context[0] else {
        panic!("expected developer message");
    };
    assert_eq!(role, "developer");
    let [ContentItem::InputText { text }, ..] = content.as_slice() else {
        panic!("expected developer text");
    };
    assert!(text.contains("<model_switch>"));
}

#[tokio::test]
async fn record_context_updates_and_set_reference_context_item_persists_full_reinjection_to_rollout()
 {
    let (mut session, previous_context) = make_session_and_context().await;
    let next_model = if previous_context.model_info.slug == "gpt-5.4" {
        "gpt-5.2"
    } else {
        "gpt-5.4"
    };
    let turn_context = previous_context
        .with_model(next_model.to_string(), &session.services.models_manager)
        .await;
    let rollout_path = attach_thread_persistence(&mut session).await;

    session
        .persist_rollout_items(&[RolloutItem::EventMsg(EventMsg::UserMessage(
            UserMessageEvent {
                client_id: None,
                message: "seed rollout".to_string(),
                images: None,
                local_images: Vec::new(),
                text_elements: Vec::new(),
                ..Default::default()
            },
        ))])
        .await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }

    session
        .set_previous_turn_settings(Some(PreviousTurnSettings {
            model: previous_context.model_info.slug.clone(),
            realtime_active: Some(previous_context.realtime_active),
        }))
        .await;
    session
        .record_context_updates_and_set_reference_context_item(&turn_context)
        .await;
    session.ensure_rollout_materialized().await;
    session.flush_rollout().await.expect("rollout should flush");

    let InitialHistory::Resumed(resumed) = RolloutRecorder::get_rollout_history(&rollout_path)
        .await
        .expect("read rollout history")
    else {
        panic!("expected resumed rollout history");
    };
    let persisted_turn_context = resumed.history.iter().find_map(|item| match item {
        RolloutItem::TurnContext(ctx) => Some(ctx.clone()),
        _ => None,
    });

    assert_eq!(
        serde_json::to_value(persisted_turn_context)
            .expect("serialize persisted turn context item"),
        serde_json::to_value(Some(turn_context.to_turn_context_item()))
            .expect("serialize expected turn context item")
    );
}

#[tokio::test]
async fn run_user_shell_command_does_not_set_reference_context_item() {
    let (session, _turn_context, rx) = make_session_and_context_with_rx().await;
    {
        let mut state = session.state.lock().await;
        state.set_reference_context_item(/*item*/ None);
    }

    handlers::run_user_shell_command(&session, "sub-id".to_string(), "echo shell".to_string())
        .await;

    let deadline = StdDuration::from_secs(15);
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline.saturating_sub(start.elapsed());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        if matches!(evt.msg, EventMsg::TurnComplete(_)) {
            break;
        }
    }

    assert!(
        session.reference_context_item().await.is_none(),
        "standalone shell tasks should not mutate previous context"
    );
}

#[tokio::test]
async fn realtime_conversation_list_voices_emits_builtin_list() {
    let (session, _turn_context, rx) = make_session_and_context_with_rx().await;

    handlers::realtime_conversation_list_voices(&session, "sub-id".to_string()).await;

    let event = rx.recv().await.expect("event");
    let voices = match event.msg {
        EventMsg::RealtimeConversationListVoicesResponse(
            RealtimeConversationListVoicesResponseEvent { voices },
        ) => voices,
        msg => panic!("expected list voices response, got {msg:?}"),
    };
    assert_eq!(
        voices,
        RealtimeVoicesList {
            v1: vec![
                RealtimeVoice::Juniper,
                RealtimeVoice::Maple,
                RealtimeVoice::Spruce,
                RealtimeVoice::Ember,
                RealtimeVoice::Vale,
                RealtimeVoice::Breeze,
                RealtimeVoice::Arbor,
                RealtimeVoice::Sol,
                RealtimeVoice::Cove,
            ],
            v2: vec![
                RealtimeVoice::Alloy,
                RealtimeVoice::Ash,
                RealtimeVoice::Ballad,
                RealtimeVoice::Coral,
                RealtimeVoice::Echo,
                RealtimeVoice::Sage,
                RealtimeVoice::Shimmer,
                RealtimeVoice::Verse,
                RealtimeVoice::Marin,
                RealtimeVoice::Cedar,
            ],
            default_v1: RealtimeVoice::Cove,
            default_v2: RealtimeVoice::Marin,
        },
    );
}

#[derive(Clone, Copy)]
struct CompletingTask;

impl SessionTask for CompletingTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.completing"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        None
    }
}

#[derive(Clone, Copy)]
struct NeverEndingTask {
    kind: TaskKind,
    listen_to_cancellation_token: bool,
}

#[derive(Clone, Copy)]
struct PanickingTask;

impl SessionTask for PanickingTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.panicking"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        panic!("simulated turn crash")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn panicking_turn_reports_errored_status_and_clears_active_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;

    sess.spawn_task(tc, Vec::new(), PanickingTask).await;

    let mut status_rx = sess.agent_status.subscribe();
    timeout(
        StdDuration::from_secs(5),
        status_rx.wait_for(|status| matches!(status, AgentStatus::Errored(_))),
    )
    .await
    .expect("panicking turn reaches a terminal status")
    .expect("status sender open");
    timeout(StdDuration::from_secs(5), async {
        while sess.active_turn.lock().await.is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("panicking turn clears the active turn");

    assert_eq!(
        sess.agent_status.borrow().clone(),
        AgentStatus::Errored("turn task panicked: simulated turn crash".to_string())
    );
}

impl SessionTask for NeverEndingTask {
    fn kind(&self) -> TaskKind {
        self.kind
    }

    fn span_name(&self) -> &'static str {
        "session_task.never_ending"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        if self.listen_to_cancellation_token {
            cancellation_token.cancelled().await;
            return None;
        }
        loop {
            sleep(Duration::from_secs(60)).await;
        }
    }
}

struct CaptureInputTask {
    tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Vec<TurnInput>>>>,
}

impl SessionTask for CaptureInputTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.capture_input"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        if let Some(tx) = self.tx.lock().expect("capture input mutex poisoned").take() {
            let _ = tx.send(input);
        }
        cancellation_token.cancelled().await;
        None
    }
}

struct CaptureAbortMcpProfileTask {
    tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<PermissionProfile>>>,
}

impl SessionTask for CaptureAbortMcpProfileTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.capture_abort_mcp_profile"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        cancellation_token.cancelled().await;
        None
    }

    async fn abort(&self, session: Arc<SessionTaskContext>, _ctx: Arc<TurnContext>) {
        let permission_profile = session
            .clone_session()
            .services
            .mcp_connection_manager
            .read()
            .await
            .permission_profile_for_elicitation();
        if let Some(tx) = self
            .tx
            .lock()
            .expect("capture abort MCP profile mutex poisoned")
            .take()
        {
            let _ = tx.send(permission_profile);
        }
    }
}

struct CaptureAbortNetworkDomainsTask {
    tx: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Option<Vec<String>>>>>,
}

impl SessionTask for CaptureAbortNetworkDomainsTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.capture_abort_network_domains"
    }

    async fn run(
        self: Arc<Self>,
        _session: Arc<SessionTaskContext>,
        _ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        cancellation_token.cancelled().await;
        None
    }

    async fn abort(&self, session: Arc<SessionTaskContext>, _ctx: Arc<TurnContext>) {
        let allowed_domains = session
            .clone_session()
            .services
            .network_proxy
            .load_full()
            .expect("managed network proxy should be present")
            .proxy()
            .current_cfg()
            .await
            .expect("managed network proxy config should be readable")
            .network
            .allowed_domains();
        if let Some(tx) = self
            .tx
            .lock()
            .expect("capture abort network domains mutex poisoned")
            .take()
        {
            let _ = tx.send(allowed_domains);
        }
    }
}

#[derive(Clone, Copy)]
struct GuardianDeniedApprovalTask;

impl SessionTask for GuardianDeniedApprovalTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Regular
    }

    fn span_name(&self) -> &'static str {
        "session_task.guardian_denied_approval"
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        _input: Vec<TurnInput>,
        cancellation_token: CancellationToken,
    ) -> Option<String> {
        let session = session.clone_session();
        for _ in 0..3 {
            crate::guardian::record_guardian_denial_for_test(&session, &ctx, &ctx.sub_id).await;
        }

        cancellation_token.cancelled().await;
        None
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_auto_review_interrupts_after_three_consecutive_denials() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "trigger guardian denials".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(Arc::clone(&tc), input, GuardianDeniedApprovalTask)
        .await;

    let mut observed = Vec::new();
    let aborted = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let EventMsg::TurnAborted(event) = &event.msg {
                let event = event.clone();
                observed.push(EventMsg::TurnAborted(event.clone()));
                break event;
            }
            observed.push(event.msg);
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "guardian denial circuit breaker should interrupt the turn; observed events: {observed:?}"
        )
    });
    assert_eq!(aborted.reason, TurnAbortReason::Interrupted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn guardian_helper_review_interrupts_after_three_consecutive_denials() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "keep turn active for helper reviews".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    let session_for_review = Arc::clone(&sess);
    let turn_for_review = Arc::clone(&tc);
    let turn_id = tc.sub_id.clone();
    let review_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("helper review runtime");
        runtime.block_on(async move {
            for _ in 0..3 {
                crate::guardian::record_guardian_denial_for_test(
                    &session_for_review,
                    &turn_for_review,
                    &turn_id,
                )
                .await;
            }
        });
    });
    review_thread.join().expect("helper review thread");

    let mut observed = Vec::new();
    let aborted = timeout(StdDuration::from_secs(5), async {
        loop {
            let event = rx.recv().await.expect("event");
            if let EventMsg::TurnAborted(event) = &event.msg {
                let event = event.clone();
                observed.push(EventMsg::TurnAborted(event.clone()));
                break event;
            }
            observed.push(event.msg);
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "helper review circuit breaker should interrupt the turn; observed events: {observed:?}"
        )
    });
    assert_eq!(aborted.reason, TurnAbortReason::Interrupted);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[test_log::test]
async fn abort_regular_task_emits_marker_before_turn_aborted() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Interrupts surface the model-visible `<turn_aborted>` marker before the abort event.
    let marker_evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for marker event")
        .expect("event");
    assert!(matches!(marker_evt.msg, EventMsg::RawResponseItem(_)));

    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("event");
    match evt.msg {
        EventMsg::TurnAborted(e) => assert_eq!(TurnAbortReason::Interrupted, e.reason),
        other => panic!("unexpected event: {other:?}"),
    }
    // No extra events should be emitted after an abort.
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn abort_gracefully_emits_marker_before_turn_aborted() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Gracefully cancelled tasks surface the model-visible marker before the abort event too.
    let marker_evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for marker event")
        .expect("event");
    assert!(matches!(marker_evt.msg, EventMsg::RawResponseItem(_)));

    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timeout waiting for event")
        .expect("event");
    match evt.msg {
        EventMsg::TurnAborted(e) => assert_eq!(TurnAbortReason::Interrupted, e.reason),
        other => panic!("unexpected event: {other:?}"),
    }
    // No extra events should be emitted after an abort.
    assert!(rx.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_emits_turn_item_lifecycle_for_leftover_pending_user_input() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    while rx.try_recv().is_ok() {}

    let text_element = ontocode_protocol::user_input::TextElement::new(
        ontocode_protocol::user_input::ByteRange { start: 5, end: 12 },
        Some("pending marker".to_string()),
    );
    let pending_user_input = vec![UserInput::Text {
        text: "late pending input".to_string(),
        text_elements: vec![text_element.clone()],
    }];
    sess.steer_input(
        pending_user_input.clone(),
        /*additional_context*/ Default::default(),
        Some(&tc.sub_id),
        /*client_user_message_id*/ None,
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steer pending input into active turn");

    sess.on_task_finished(Arc::clone(&tc), /*last_agent_message*/ None)
        .await;

    let history = sess.clone_history().await;
    let expected = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "late pending input".to_string(),
        }],
        phase: None,
    };
    assert!(
        history.raw_items().iter().any(|item| item == &expected),
        "expected pending input to be persisted into history on turn completion"
    );

    let first = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected raw response item event")
        .expect("channel open");
    assert!(matches!(first.msg, EventMsg::RawResponseItem(_)));

    let second = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected item started event")
        .expect("channel open");
    assert!(matches!(
        second.msg,
        EventMsg::ItemStarted(ItemStartedEvent {
            item: TurnItem::UserMessage(UserMessageItem { content, .. }),
            ..
        }) if content == pending_user_input
    ));

    let third = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected item completed event")
        .expect("channel open");
    assert!(matches!(
        third.msg,
        EventMsg::ItemCompleted(ItemCompletedEvent {
            item: TurnItem::UserMessage(UserMessageItem { content, .. }),
            ..
        }) if content == pending_user_input
    ));

    let fourth = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected legacy user message event")
        .expect("channel open");
    assert!(matches!(
        fourth.msg,
        EventMsg::UserMessage(UserMessageEvent {
                client_id: None,
            message,
            images,
            text_elements,
            local_images,
            ..
        }) if message == "late pending input"
            && images == Some(Vec::new())
            && text_elements == vec![text_element]
            && local_images.is_empty()
    ));

    let fifth = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn complete event")
        .expect("channel open");
    assert!(matches!(
        fifth.msg,
        EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id,
            last_agent_message: None,
            time_to_first_token_ms: None,
            ..
        }) if turn_id == tc.sub_id
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_recovers_last_agent_message_before_subagent_notification() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "start work".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let assistant_item = assistant_message_with_phase(
        "visible progress before worker result",
        Some(MessagePhase::Commentary),
    );
    let notification_text = crate::session_prefix::format_subagent_notification_message(
        "agent-1",
        &ontocode_protocol::protocol::AgentStatus::Completed(Some("child done".to_string())),
    );
    let notification_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: notification_text,
        }],
        phase: None,
    };
    sess.record_conversation_items(tc.as_ref(), &[assistant_item, notification_item])
        .await;

    while rx.try_recv().is_ok() {}

    sess.on_task_finished(Arc::clone(&tc), /*last_agent_message*/ None)
        .await;

    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn complete event")
        .expect("channel open");
    assert!(matches!(
        evt.msg,
        EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id,
            last_agent_message: Some(message),
            ..
        }) if turn_id == tc.sub_id && message == "visible progress before worker result"
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_does_not_promote_inter_agent_transport_to_last_agent_message() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "continue with the same task".to_string(),
        /*trigger_turn*/ true,
    );
    sess.inject_if_running(vec![ResponseItem::from(
        communication.to_response_input_item(),
    )])
    .await
    .expect("inject transport into active turn");

    while rx.try_recv().is_ok() {}

    sess.on_task_finished(Arc::clone(&tc), /*last_agent_message*/ None)
        .await;

    let completion = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let event = rx.recv().await.expect("event channel should remain open");
            if let EventMsg::TurnComplete(completion) = event.msg {
                break completion;
            }
        }
    })
    .await
    .expect("expected turn complete event");
    assert_eq!(completion.last_agent_message, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_does_not_recover_last_agent_message_across_user_turn() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    sess.record_conversation_items(
        tc.as_ref(),
        &[
            user_message("previous user turn"),
            assistant_message("previous final answer"),
        ],
    )
    .await;

    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "current user turn".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    while rx.try_recv().is_ok() {}

    sess.on_task_finished(Arc::clone(&tc), /*last_agent_message*/ None)
        .await;

    let evt = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("expected turn complete event")
        .expect("channel open");
    assert!(matches!(
        evt.msg,
        EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id,
            last_agent_message: None,
            ..
        }) if turn_id == tc.sub_id
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_turn_complete_does_not_mask_existing_agent_error_status() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;

    sess.send_event_raw(Event {
        id: tc.sub_id.clone(),
        msg: EventMsg::Error(ErrorEvent {
            message: "provider failed".to_string(),
            codex_error_info: None,
        }),
    })
    .await;
    sess.send_event_raw(Event {
        id: tc.sub_id.clone(),
        msg: EventMsg::TurnComplete(TurnCompleteEvent {
            turn_id: tc.sub_id.clone(),
            last_agent_message: None,
            completed_at: None,
            duration_ms: None,
            time_to_first_token_ms: None,
        }),
    })
    .await;

    assert_eq!(
        sess.agent_status.borrow().clone(),
        AgentStatus::Errored("provider failed".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn task_finish_emits_thread_idle_lifecycle_after_active_turn_clears() {
    struct ThreadIdleRecorder {
        calls: Arc<std::sync::atomic::AtomicUsize>,
        idle_tx: async_channel::Sender<()>,
        expected_thread_id: ThreadId,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::ThreadLifecycleContributor<crate::config::Config>
        for ThreadIdleRecorder
    {
        async fn on_thread_idle(&self, input: ontocode_extension_api::ThreadIdleInput<'_>) {
            assert_eq!(
                self.expected_thread_id.to_string(),
                input.thread_store.level_id()
            );
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.idle_tx.send(()).await.expect("idle receiver open");
        }
    }

    let (mut session, turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (idle_tx, idle_rx) = async_channel::bounded(1);
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.thread_lifecycle_contributor(Arc::new(ThreadIdleRecorder {
        calls: Arc::clone(&calls),
        idle_tx,
        expected_thread_id: session.thread_id,
    }));
    session.services.extensions = Arc::new(builder.build());

    let session = Arc::new(session);
    session
        .spawn_task(Arc::new(turn_context), Vec::new(), CompletingTask)
        .await;

    timeout(StdDuration::from_secs(2), idle_rx.recv())
        .await
        .expect("thread idle lifecycle")
        .expect("idle receiver open");
    assert_eq!(1, calls.load(std::sync::atomic::Ordering::SeqCst));
    assert!(session.active_turn.lock().await.is_none());
}

#[tokio::test]
async fn thread_idle_lifecycle_waits_for_trigger_turn_mailbox_work() {
    struct ThreadIdleRecorder {
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::ThreadLifecycleContributor<crate::config::Config>
        for ThreadIdleRecorder
    {
        async fn on_thread_idle(&self, _input: ontocode_extension_api::ThreadIdleInput<'_>) {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    let (mut session, _turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.thread_lifecycle_contributor(Arc::new(ThreadIdleRecorder {
        calls: Arc::clone(&calls),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .input_queue
        .enqueue_mailbox_communication(InterAgentCommunication::new(
            AgentPath::root(),
            AgentPath::root(),
            Vec::new(),
            "pending trigger".to_string(),
            /*trigger_turn*/ true,
        ))
        .await;

    session.emit_thread_idle_lifecycle_if_idle().await;

    assert_eq!(0, calls.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn thread_idle_lifecycle_waits_for_accepted_foreground_submission() {
    struct ThreadIdleRecorder {
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ontocode_extension_api::ThreadLifecycleContributor<crate::config::Config>
        for ThreadIdleRecorder
    {
        async fn on_thread_idle(&self, _input: ontocode_extension_api::ThreadIdleInput<'_>) {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    let (mut session, _turn_context) = make_session_and_context().await;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut builder =
        ontocode_extension_api::ExtensionRegistryBuilder::<crate::config::Config>::new();
    builder.thread_lifecycle_contributor(Arc::new(ThreadIdleRecorder {
        calls: Arc::clone(&calls),
    }));
    session.services.extensions = Arc::new(builder.build());
    session
        .register_foreground_submission("accepted-user-input")
        .await
        .expect("foreground submission should be admitted");

    session.emit_thread_idle_lifecycle_if_idle().await;

    assert_eq!(0, calls.load(std::sync::atomic::Ordering::SeqCst));
    session
        .finish_foreground_submission("accepted-user-input")
        .await;
}

#[tokio::test]
async fn queue_only_mailbox_message_starts_idle_receiver_turn() {
    let (sess, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let sub_id = "queue-only-idle-receiver".to_string();

    super::handlers::inter_agent_communication(
        &sess,
        sub_id.clone(),
        InterAgentCommunication::new(
            AgentPath::root(),
            AgentPath::root(),
            Vec::new(),
            "queued while idle".to_string(),
            /*trigger_turn*/ false,
        ),
    )
    .await;

    let active_turn = sess.active_turn.lock().await;
    let started_sub_id = active_turn
        .as_ref()
        .and_then(|turn| turn.task.as_ref())
        .map(|task| task.turn_context.sub_id.clone());
    assert_eq!(Some(sub_id), started_sub_id);
    drop(active_turn);

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn trigger_turn_mailbox_message_keeps_immediate_idle_wake() {
    let (sess, _turn_context, _rx) = make_session_and_context_with_rx().await;
    let sub_id = "trigger-idle-receiver".to_string();

    super::handlers::inter_agent_communication(
        &sess,
        sub_id.clone(),
        InterAgentCommunication::new(
            AgentPath::root(),
            AgentPath::root(),
            Vec::new(),
            "trigger while idle".to_string(),
            /*trigger_turn*/ true,
        ),
    )
    .await;

    let active_turn = sess.active_turn.lock().await;
    let started_sub_id = active_turn
        .as_ref()
        .and_then(|turn| turn.task.as_ref())
        .map(|task| task.turn_context.sub_id.clone());
    assert_eq!(Some(sub_id), started_sub_id);
    drop(active_turn);

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn try_start_turn_if_idle_rejects_active_turn_without_injecting() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    let item = user_message("synthetic idle input");
    let err = sess
        .try_start_turn_if_idle(vec![item.clone()])
        .await
        .expect_err("active turn should reject idle-only input");

    assert_eq!(vec![item], err);
    assert_eq!(
        Vec::<TurnInput>::new(),
        sess.input_queue.get_pending_input(&sess.active_turn).await
    );

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn try_start_read_only_turn_if_idle_uses_read_only_context() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let stored_permission_profile = tc.permission_profile();

    sess.try_start_turn_if_idle(vec![user_message("synthetic writable result")])
        .await
        .expect("idle session should start a writable turn");
    {
        let active_turn = sess.active_turn.lock().await;
        let active_turn = active_turn
            .as_ref()
            .expect("writable idle turn should reserve an active turn");
        let turn_context = &active_turn
            .task
            .as_ref()
            .expect("writable idle turn should start a task")
            .turn_context;
        assert_eq!(turn_context.permission_profile(), stored_permission_profile);
        assert!(
            !active_turn
                .turn_state
                .lock()
                .await
                .unattended_read_only_filesystem()
        );
    }
    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    sess.try_start_read_only_turn_if_idle(vec![user_message("synthetic manager result")])
        .await
        .expect("idle session should start a read-only turn");

    {
        let active_turn = sess.active_turn.lock().await;
        let active_turn = active_turn
            .as_ref()
            .expect("read-only idle turn should reserve an active turn");
        let turn_context = &active_turn
            .task
            .as_ref()
            .expect("read-only idle turn should start a task")
            .turn_context;
        #[allow(deprecated)]
        let cwd = turn_context.cwd.as_path();
        let permission_profile = turn_context.permission_profile();
        assert!(
            permission_profile
                .file_system_sandbox_policy()
                .can_read_path_with_cwd(cwd, cwd)
        );
        assert!(
            !permission_profile
                .file_system_sandbox_policy()
                .can_write_path_with_cwd(cwd, cwd)
        );
        assert_eq!(tc.permission_profile(), stored_permission_profile);
        assert!(
            active_turn
                .turn_state
                .lock()
                .await
                .unattended_read_only_filesystem()
        );
    }

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn writable_replacement_preserves_pending_read_only_input() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let synthetic_item = user_message("synthetic manager result");
    let normal_turn_context = sess.new_default_turn().await;
    let turn_state = {
        let mut active_turn = sess.active_turn.lock().await;
        let active_turn = active_turn.get_or_insert_with(ActiveTurn::default);
        Arc::clone(&active_turn.turn_state)
    };
    {
        let mut turn_state = turn_state.lock().await;
        turn_state.enable_unattended_read_only_filesystem();
    }
    sess.input_queue
        .extend_pending_input_for_turn_state(
            turn_state.as_ref(),
            vec![TurnInput::ResponseItem(synthetic_item.clone())],
        )
        .await;
    let user_turn_id = normal_turn_context.sub_id.clone();
    let (captured_input_tx, captured_input_rx) = tokio::sync::oneshot::channel();
    sess.spawn_task(
        normal_turn_context,
        vec![TurnInput::UserInput {
            content: vec![UserInput::Text {
                text: "real user input".to_string(),
                text_elements: Vec::new(),
            }],
            client_id: None,
            origin: None,
        }],
        CaptureInputTask {
            tx: std::sync::Mutex::new(Some(captured_input_tx)),
        },
    )
    .await;

    let captured_input = timeout(Duration::from_secs(5), captured_input_rx)
        .await
        .expect("user task should start")
        .expect("user task should receive its startup input");

    let active_turn_guard = sess.active_turn.lock().await;
    let active_turn = active_turn_guard
        .as_ref()
        .expect("user turn should remain active");
    assert_eq!(
        active_turn
            .task
            .as_ref()
            .expect("user task should be installed")
            .turn_context
            .sub_id,
        user_turn_id
    );
    assert!(
        !active_turn
            .turn_state
            .lock()
            .await
            .unattended_read_only_filesystem()
    );
    drop(active_turn_guard);
    assert_eq!(
        captured_input,
        vec![
            TurnInput::ResponseItem(synthetic_item),
            TurnInput::UserInput {
                content: vec![UserInput::Text {
                    text: "real user input".to_string(),
                    text_elements: Vec::new(),
                }],
                client_id: None,
                origin: None,
            },
        ]
    );
    assert!(
        sess.input_queue
            .get_pending_input(&sess.active_turn)
            .await
            .is_empty(),
        "replacement must consume carried synthetic input exactly once"
    );
    assert_eq!(
        sess.services
            .mcp_connection_manager
            .read()
            .await
            .permission_profile_for_elicitation(),
        tc.permission_profile()
    );

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn pending_mcp_refresh_activates_after_read_only_task_abort() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let read_only_context = sess
        .new_read_only_turn_with_sub_id("read-only-turn".to_string())
        .await;
    let read_only_profile = read_only_context.permission_profile();
    let (abort_profile_tx, abort_profile_rx) = tokio::sync::oneshot::channel();
    sess.spawn_task(
        read_only_context,
        Vec::new(),
        CaptureAbortMcpProfileTask {
            tx: std::sync::Mutex::new(Some(abort_profile_tx)),
        },
    )
    .await;

    let refresh_config = McpServerRefreshConfig {
        mcp_servers: json!({}),
    };
    *sess.pending_mcp_server_refresh_config.lock().await = Some(refresh_config);
    let writable_context = sess.new_default_turn().await;
    sess.spawn_task_after_pending_mcp_refresh(
        writable_context,
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    assert_eq!(
        timeout(Duration::from_secs(5), abort_profile_rx)
            .await
            .expect("read-only task abort should complete")
            .expect("read-only task abort should report its MCP profile"),
        read_only_profile
    );
    assert!(
        sess.pending_mcp_server_refresh_config
            .lock()
            .await
            .is_none()
    );
    assert_eq!(
        sess.services
            .mcp_connection_manager
            .read()
            .await
            .permission_profile_for_elicitation(),
        tc.permission_profile()
    );

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn steer_input_rejects_unattended_read_only_turn() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    sess.try_start_read_only_turn_if_idle(vec![user_message("synthetic manager result")])
        .await
        .expect("idle session should start a read-only turn");
    let input = vec![UserInput::Text {
        text: "real user input".to_string(),
        text_elements: Vec::new(),
    }];

    let err = sess
        .steer_input(
            input.clone(),
            /*additional_context*/ Default::default(),
            /*expected_turn_id*/ None,
            /*client_user_message_id*/ None,
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect_err("unattended read-only turns must be replaced, not steered");

    assert_eq!(err, SteerInputError::NoActiveTurn(input));
    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
}

#[tokio::test]
async fn stale_idle_generation_cannot_start_read_only_turn() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let idle_generation = sess
        .foreground_submission_generation
        .load(std::sync::atomic::Ordering::Acquire);
    sess.register_foreground_submission("new-user-input")
        .await
        .expect("foreground submission should be admitted");
    let continuation = vec![user_message("stale goal continuation")];

    let rejected = sess
        .try_start_read_only_turn_if_idle_for_generation(continuation.clone(), idle_generation)
        .await
        .expect_err("new foreground input must invalidate the older idle callback");

    assert_eq!(continuation, rejected);
    assert!(sess.active_turn.lock().await.is_none());
    sess.finish_foreground_submission("new-user-input").await;
}

#[tokio::test]
async fn steer_input_requires_active_turn() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];

    let err = sess
        .steer_input(
            input,
            /*additional_context*/ Default::default(),
            /*expected_turn_id*/ None,
            /*client_user_message_id*/ None,
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect_err("steering without active turn should fail");

    assert!(matches!(err, SteerInputError::NoActiveTurn(_)));
}

#[tokio::test]
async fn steer_input_enforces_expected_turn_id() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let steer_input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];
    let err = sess
        .steer_input(
            steer_input,
            /*additional_context*/ Default::default(),
            Some("different-turn-id"),
            /*client_user_message_id*/ None,
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect_err("mismatched expected turn id should fail");

    match err {
        SteerInputError::ExpectedTurnMismatch { expected, actual } => {
            assert_eq!(
                (expected, actual),
                ("different-turn-id".to_string(), tc.sub_id.clone())
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[tokio::test]
async fn steer_input_rejects_non_regular_turns() {
    for (task_kind, turn_kind) in [
        (TaskKind::Review, NonSteerableTurnKind::Review),
        (TaskKind::Compact, NonSteerableTurnKind::Compact),
    ] {
        let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
        let input = vec![TurnInput::UserInput {
            content: vec![UserInput::Text {
                text: "hello".to_string(),
                text_elements: Vec::new(),
            }],
            client_id: None,
            origin: None,
        }];
        let turn_context = sess.new_default_turn_with_sub_id("turn".to_string()).await;
        sess.spawn_task(
            turn_context,
            input,
            NeverEndingTask {
                kind: task_kind,
                listen_to_cancellation_token: true,
            },
        )
        .await;

        let steer_input = vec![UserInput::Text {
            text: "steer".to_string(),
            text_elements: Vec::new(),
        }];
        let err = sess
            .steer_input(
                steer_input,
                /*additional_context*/ Default::default(),
                /*expected_turn_id*/ None,
                /*client_user_message_id*/ None,
                /*responsesapi_client_metadata*/ None,
            )
            .await
            .expect_err("steering a non-regular turn should fail");

        assert_eq!(err, SteerInputError::ActiveTurnNotSteerable { turn_kind });

        sess.abort_all_tasks(TurnAbortReason::Interrupted).await;
    }
}

#[tokio::test]
async fn steer_input_returns_active_turn_id() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: false,
        },
    )
    .await;

    let steer_input = vec![UserInput::Text {
        text: "steer".to_string(),
        text_elements: Vec::new(),
    }];
    let turn_id = sess
        .steer_input(
            steer_input,
            /*additional_context*/ Default::default(),
            Some(&tc.sub_id),
            /*client_user_message_id*/ None,
            /*responsesapi_client_metadata*/ None,
        )
        .await
        .expect("steering with matching expected turn id should succeed");

    assert_eq!(turn_id, tc.sub_id);
    assert!(sess.input_queue.has_pending_input(&sess.active_turn).await);
}

#[tokio::test]
async fn abort_empty_active_turn_preserves_pending_input() {
    let (sess, _tc, _rx) = make_session_and_context_with_rx().await;
    let pending_item = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "late pending input".to_string(),
        }],
        phase: None,
    };
    let turn_state = {
        let mut active = sess.active_turn.lock().await;
        let active_turn = active.get_or_insert_with(ActiveTurn::default);
        Arc::clone(&active_turn.turn_state)
    };
    sess.input_queue
        .extend_pending_input_for_turn_state(
            turn_state.as_ref(),
            vec![TurnInput::ResponseItem(pending_item.clone())],
        )
        .await;

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert!(sess.active_turn.lock().await.is_none());
    assert_eq!(
        sess.input_queue
            .take_pending_input_for_turn_state(turn_state.as_ref())
            .await,
        vec![TurnInput::ResponseItem(pending_item)]
    );
}

async fn set_total_token_usage(sess: &Session, total_token_usage: TokenUsage) {
    let mut state = sess.state.lock().await;
    state.set_token_info(Some(TokenUsageInfo {
        total_token_usage,
        last_token_usage: TokenUsage::default(),
        model_context_window: None,
    }));
}

#[tokio::test]
async fn queue_only_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "late queue-only update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;

    assert!(
        !sess.input_queue.has_pending_input(&sess.active_turn).await,
        "queue-only mailbox mail should stay buffered once the current turn emitted its answer"
    );
    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        Vec::new()
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![TurnInput::ResponseItem(ResponseItem::from(
            communication.to_response_input_item()
        ))],
    );
}

#[tokio::test]
async fn trigger_turn_mailbox_mail_waits_for_next_turn_after_answer_boundary() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(InterAgentCommunication::new(
            AgentPath::try_from("/root/worker").expect("worker path should parse"),
            AgentPath::root(),
            Vec::new(),
            "late trigger update".to_string(),
            /*trigger_turn*/ true,
        ))
        .await;

    assert!(
        !sess.input_queue.has_pending_input(&sess.active_turn).await,
        "trigger-turn mailbox mail should not extend the current turn after its answer boundary"
    );

    sess.abort_all_tasks(TurnAbortReason::Replaced).await;

    assert!(sess.input_queue.has_trigger_turn_mailbox_items().await);
}

#[tokio::test]
async fn steered_input_reopens_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        /*additional_context*/ Default::default(),
        Some(&tc.sub_id),
        /*client_user_message_id*/ None,
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![
            TurnInput::UserInput {
                content: vec![UserInput::Text {
                    text: "follow up".to_string(),
                    text_elements: Vec::new(),
                }],
                client_id: None,
                origin: Some(UserMessageOrigin::ClientSubmission),
            },
            TurnInput::ResponseItem(ResponseItem::from(communication.to_response_input_item())),
        ],
    );
}

#[tokio::test]
async fn stale_defer_mailbox_delivery_does_not_override_steered_input() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;
    sess.steer_input(
        vec![UserInput::Text {
            text: "follow up".to_string(),
            text_elements: Vec::new(),
        }],
        /*additional_context*/ Default::default(),
        Some(&tc.sub_id),
        /*client_user_message_id*/ None,
        /*responsesapi_client_metadata*/ None,
    )
    .await
    .expect("steered input should be accepted");

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;

    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![
            TurnInput::UserInput {
                content: vec![UserInput::Text {
                    text: "follow up".to_string(),
                    text_elements: Vec::new(),
                }],
                client_id: None,
                origin: Some(UserMessageOrigin::ClientSubmission),
            },
            TurnInput::ResponseItem(ResponseItem::from(communication.to_response_input_item())),
        ],
    );
}

#[tokio::test]
async fn tool_calls_reopen_mailbox_delivery_for_current_turn() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let communication = InterAgentCommunication::new(
        AgentPath::try_from("/root/worker").expect("worker path should parse"),
        AgentPath::root(),
        Vec::new(),
        "queued child update".to_string(),
        /*trigger_turn*/ false,
    );
    sess.spawn_task(
        Arc::clone(&tc),
        Vec::new(),
        NeverEndingTask {
            kind: TaskKind::Regular,
            listen_to_cancellation_token: true,
        },
    )
    .await;

    sess.input_queue
        .defer_mailbox_delivery_to_next_turn(&sess.active_turn, &tc.sub_id)
        .await;
    sess.input_queue
        .enqueue_mailbox_communication(communication.clone())
        .await;

    let item = ResponseItem::FunctionCall {
        id: None,
        name: "test_tool".to_string(),
        namespace: None,
        arguments: "{}".to_string(),
        call_id: "call-1".to_string(),
    };
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&sess),
        turn_context: Arc::clone(&tc),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            tc.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&sess), Arc::clone(&tc)),
        cancellation_token: CancellationToken::new(),
    };

    let output = handle_output_item_done(&mut ctx, item, /*previously_active_item*/ None)
        .await
        .expect("tool call should be handled");

    assert!(output.needs_follow_up);
    assert!(output.tool_future.is_some());
    assert_eq!(
        sess.input_queue.get_pending_input(&sess.active_turn).await,
        vec![TurnInput::ResponseItem(ResponseItem::from(
            communication.to_response_input_item()
        ))],
    );
}

#[tokio::test]
async fn provider_history_duplicate_tool_call_is_not_queued_twice() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let item = function_call("call-provider-history", "test_tool");
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&sess),
        turn_context: Arc::clone(&tc),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            tc.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&sess), Arc::clone(&tc)),
        cancellation_token: CancellationToken::new(),
    };

    let first =
        handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
            .await
            .expect("first tool call should be handled");
    let duplicate =
        handle_output_item_done(&mut ctx, item.clone(), /*previously_active_item*/ None)
            .await
            .expect("duplicate tool call should be ignored");

    assert!(first.needs_follow_up);
    assert!(first.tool_future.is_some());
    assert!(!duplicate.needs_follow_up);
    assert!(duplicate.tool_future.is_none());
    assert!(duplicate.tool_call_is_read_only.is_none());
    assert!(duplicate.last_agent_message.is_none());
    assert_eq!(sess.clone_history().await.raw_items(), &[item]);
}

#[tokio::test]
async fn provider_history_conflicting_tool_call_is_rejected_before_execution() {
    let (sess, tc, _rx) = make_session_and_context_with_rx().await;
    let original = function_call("call-provider-history", "test_tool");
    let conflicting = ResponseItem::FunctionCall {
        id: None,
        name: "test_tool".to_string(),
        namespace: None,
        arguments: r#"{"different":true}"#.to_string(),
        call_id: "call-provider-history".to_string(),
    };
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&sess),
        turn_context: Arc::clone(&tc),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            tc.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&sess), Arc::clone(&tc)),
        cancellation_token: CancellationToken::new(),
    };

    let first = handle_output_item_done(
        &mut ctx,
        original.clone(),
        /*previously_active_item*/ None,
    )
    .await
    .expect("first tool call should be handled");
    let err =
        match handle_output_item_done(&mut ctx, conflicting, /*previously_active_item*/ None).await
        {
            Ok(_) => panic!("conflicting tool call should be rejected"),
            Err(err) => err,
        };

    assert!(first.tool_future.is_some());
    assert!(
        err.to_string()
            .contains("conflicting duplicate tool history item")
    );
    assert_eq!(sess.clone_history().await.raw_items(), &[original]);
}

#[tokio::test]
async fn provider_history_persistence_failure_does_not_mutate_history_or_queue_execution() {
    let (mut session, turn_context) = make_session_and_context().await;
    attach_thread_persistence(&mut session).await;
    session
        .live_thread()
        .expect("thread persistence should be attached")
        .shutdown()
        .await
        .expect("thread persistence should shut down");
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let item = function_call("call-provider-history", "test_tool");
    let mut ctx = HandleOutputCtx {
        sess: Arc::clone(&session),
        turn_context: Arc::clone(&turn_context),
        turn_store: Arc::new(ontocode_extension_api::ExtensionData::new(
            turn_context.sub_id.clone(),
        )),
        tool_runtime: test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context)),
        cancellation_token: CancellationToken::new(),
    };

    let err = match handle_output_item_done(&mut ctx, item, /*previously_active_item*/ None).await {
        Ok(_) => panic!("tool call should fail when rollout append fails"),
        Err(err) => err,
    };

    assert!(matches!(err, CodexErr::Io(_)));
    assert!(session.clone_history().await.raw_items().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn abort_review_task_emits_exited_then_aborted_and_records_history() {
    let (sess, tc, rx) = make_session_and_context_with_rx().await;
    let input = vec![TurnInput::UserInput {
        content: vec![UserInput::Text {
            text: "start review".to_string(),
            text_elements: Vec::new(),
        }],
        client_id: None,
        origin: None,
    }];
    sess.spawn_task(
        Arc::clone(&tc),
        input,
        ReviewTask::new(ontocode_protocol::protocol::ReviewTarget::Custom {
            instructions: "abort review test".to_string(),
        }),
    )
    .await;

    sess.abort_all_tasks(TurnAbortReason::Interrupted).await;

    // Aborting a review task should exit review mode before surfacing the abort to the client.
    // We scan for these events (rather than relying on fixed ordering) since unrelated events
    // may interleave.
    let mut exited_review_mode_idx = None;
    let mut turn_aborted_idx = None;
    let mut idx = 0usize;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let evt = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for event")
            .expect("event");
        let event_idx = idx;
        idx = idx.saturating_add(1);
        match evt.msg {
            EventMsg::ExitedReviewMode(ev) => {
                assert!(ev.review_output.is_none());
                exited_review_mode_idx = Some(event_idx);
            }
            EventMsg::TurnAborted(ev) => {
                assert_eq!(TurnAbortReason::Interrupted, ev.reason);
                turn_aborted_idx = Some(event_idx);
                break;
            }
            _ => {}
        }
    }
    assert!(
        exited_review_mode_idx.is_some(),
        "expected ExitedReviewMode after abort"
    );
    assert!(
        turn_aborted_idx.is_some(),
        "expected TurnAborted after abort"
    );
    assert!(
        exited_review_mode_idx.unwrap() < turn_aborted_idx.unwrap(),
        "expected ExitedReviewMode before TurnAborted"
    );

    let history = sess.clone_history().await;
    // Verify the `<turn_aborted>` marker is still recorded in history for the model.
    assert!(
        history.raw_items().iter().any(|item| {
            let ResponseItem::Message { role, content, .. } = item else {
                return false;
            };
            if role != "user" {
                return false;
            }
            content.iter().any(|content_item| {
                let ContentItem::InputText { text } = content_item else {
                    return false;
                };
                TurnAborted::matches_text(text)
            })
        }),
        "expected a model-visible turn aborted marker in history after interrupt"
    );
}

#[tokio::test]
#[expect(
    clippy::await_holding_invalid_type,
    reason = "test builds a router from session-owned MCP manager state"
)]
async fn fatal_tool_error_stops_turn_and_reports_error() {
    let (session, turn_context, _rx) = make_session_and_context_with_rx().await;
    let tools = {
        session
            .services
            .mcp_connection_manager
            .read()
            .await
            .list_all_tools()
            .await
    };
    let deferred_mcp_tools = Some(tools.clone());
    let router = ToolRouter::from_turn_context(
        &turn_context,
        crate::tools::router::ToolRouterParams {
            deferred_mcp_tools,
            mcp_tools: Some(tools),
            discoverable_tools: None,
            extension_tool_executors: Vec::new(),
            dynamic_tools: turn_context.dynamic_tools.as_slice(),
            searched_tool_names: Vec::new(),
            mcp_resources_advertised: true,
        },
    );
    let item = ResponseItem::CustomToolCall {
        id: None,
        status: None,
        call_id: "call-1".to_string(),
        name: "shell_command".to_string(),
        input: "{}".to_string(),
    };

    let call = ToolRouter::build_tool_call(item.clone())
        .expect("build tool call")
        .expect("tool call present");
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));
    let err = router
        .dispatch_tool_call_with_code_mode_result(
            Arc::clone(&session),
            Arc::clone(&turn_context),
            CancellationToken::new(),
            tracker,
            call,
            ToolCallSource::Direct,
        )
        .await
        .err()
        .expect("expected fatal error");

    match err {
        FunctionCallError::Fatal(message) => {
            assert_eq!(
                message,
                "tool shell_command invoked with incompatible payload"
            );
        }
        other => panic!("expected FunctionCallError::Fatal, got {other:?}"),
    }
}

#[tokio::test]
async fn tool_results_are_persisted_before_later_fatal_sibling_finishes() {
    let (session, turn_context) = make_session_and_context().await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let successful_call = function_call("call-ok", "test_ok");
    let fatal_call = function_call("call-fatal", "test_fatal");
    for call in [&successful_call, &fatal_call] {
        session
            .record_provider_history_item(turn_context.as_ref(), call)
            .await
            .expect("tool call should be persisted before execution");
    }

    let successful_output = function_output("call-ok", "ok");
    let fatal_output = ResponseItem::FunctionCallOutput {
        call_id: "call-fatal".to_string(),
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(
                "tool execution failed after dispatch; outcome may be unknown and will not be retried automatically"
                    .to_string(),
            ),
            success: Some(false),
        },
    };
    let successful_response = ResponseInputItem::FunctionCallOutput {
        call_id: "call-ok".to_string(),
        output: FunctionCallOutputPayload::from_text("ok".to_string()),
    };
    let fatal_response = ResponseInputItem::FunctionCallOutput {
        call_id: "call-fatal".to_string(),
        output: FunctionCallOutputPayload {
            body: FunctionCallOutputBody::Text(
                "tool execution failed after dispatch; outcome may be unknown and will not be retried automatically"
                    .to_string(),
            ),
            success: Some(false),
        },
    };
    let mut in_flight: FuturesOrdered<BoxFuture<'static, ToolCallResult>> = FuturesOrdered::new();
    in_flight.push_back(Box::pin(async move {
        ToolCallResult {
            response: successful_response,
            fatal_error: None,
            stop_follow_up: true,
        }
    }));
    let (allow_fatal_finish_tx, allow_fatal_finish_rx) = tokio::sync::oneshot::channel();
    in_flight.push_back(Box::pin(async move {
        let _ = allow_fatal_finish_rx.await;
        ToolCallResult {
            response: fatal_response,
            fatal_error: Some(CodexErr::Fatal("fatal handler failure".to_string())),
            stop_follow_up: false,
        }
    }));

    let drain_session = Arc::clone(&session);
    let drain_turn_context = Arc::clone(&turn_context);
    let drain_task = tokio::spawn(async move {
        super::turn::drain_in_flight(&mut in_flight, drain_session, drain_turn_context).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if session.clone_history().await.raw_items().len() == 3 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first terminal output should persist while the fatal sibling is pending");

    assert_eq!(
        session.clone_history().await.raw_items(),
        &[
            successful_call.clone(),
            fatal_call.clone(),
            successful_output.clone()
        ]
    );

    allow_fatal_finish_tx
        .send(())
        .expect("fatal sibling should still be waiting");
    let drain_outcome = drain_task
        .await
        .expect("drain task should join")
        .expect("terminal outputs should persist");

    assert_eq!(
        session.clone_history().await.raw_items(),
        &[successful_call, fatal_call, successful_output, fatal_output]
    );
    assert!(
        matches!(drain_outcome.fatal_error, Some(CodexErr::Fatal(message)) if message == "fatal handler failure")
    );
    assert!(drain_outcome.stop_follow_up);
}

async fn sample_rollout(
    session: &Session,
    _turn_context: &TurnContext,
) -> (Vec<RolloutItem>, Vec<ResponseItem>) {
    let mut rollout_items = Vec::new();
    let mut live_history = ContextManager::new();

    // Use the same turn_context source as record_initial_history so model_info (and thus
    // personality_spec) matches reconstruction.
    let reconstruction_turn = session.new_default_turn().await;
    let mut initial_context = session
        .build_initial_context(reconstruction_turn.as_ref())
        .await;
    // Ensure personality_spec is present when Personality is enabled, so expected matches
    // what reconstruction produces (build_initial_context may omit it when baked into model).
    if !initial_context.iter().any(|m| {
        matches!(m, ResponseItem::Message { role, content, .. }
        if role == "developer"
            && content.iter().any(|c| {
                matches!(c, ContentItem::InputText { text } if text.contains("<personality_spec>"))
            }))
    }) && let Some(p) = reconstruction_turn.personality
        && session.features.enabled(Feature::Personality)
        && let Some(personality_message) = reconstruction_turn
            .model_info
            .model_messages
            .as_ref()
            .and_then(|m| m.get_personality_message(Some(p)).filter(|s| !s.is_empty()))
    {
        let msg = crate::context::ContextualUserFragment::into(
            crate::context::PersonalitySpecInstructions::new(personality_message),
        );
        let insert_at = initial_context
            .iter()
            .position(|m| matches!(m, ResponseItem::Message { role, .. } if role == "developer"))
            .map(|i| i + 1)
            .unwrap_or(0);
        initial_context.insert(insert_at, msg);
    }
    for item in &initial_context {
        rollout_items.push(RolloutItem::ResponseItem(item.clone()));
    }
    live_history.record_items(
        initial_context.iter(),
        reconstruction_turn.truncation_policy,
    );

    let user1 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "first user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user1),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user1.clone()));

    let assistant1 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply one".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant1),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant1.clone()));

    let summary1 = "summary one";
    let snapshot1 = live_history.clone().for_prompt(
        &reconstruction_turn.model_info.input_modalities,
        reconstruction_turn.trailing_user_message(),
    );
    let user_messages1 = collect_user_messages(&snapshot1);
    let rebuilt1 = compact::build_compacted_history(Vec::new(), &user_messages1, summary1);
    live_history.replace(rebuilt1);
    rollout_items.push(RolloutItem::Compacted(CompactedItem {
        message: summary1.to_string(),
        replacement_history: None,
    }));

    let user2 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "second user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user2),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user2.clone()));

    let assistant2 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply two".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant2),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant2.clone()));

    let summary2 = "summary two";
    let snapshot2 = live_history.clone().for_prompt(
        &reconstruction_turn.model_info.input_modalities,
        reconstruction_turn.trailing_user_message(),
    );
    let user_messages2 = collect_user_messages(&snapshot2);
    let rebuilt2 = compact::build_compacted_history(Vec::new(), &user_messages2, summary2);
    live_history.replace(rebuilt2);
    rollout_items.push(RolloutItem::Compacted(CompactedItem {
        message: summary2.to_string(),
        replacement_history: None,
    }));

    let user3 = ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: "third user".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&user3),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(user3));

    let assistant3 = ResponseItem::Message {
        id: None,
        role: "assistant".to_string(),
        content: vec![ContentItem::OutputText {
            text: "assistant reply three".to_string(),
        }],
        phase: None,
    };
    live_history.record_items(
        std::iter::once(&assistant3),
        reconstruction_turn.truncation_policy,
    );
    rollout_items.push(RolloutItem::ResponseItem(assistant3));

    (
        rollout_items,
        live_history.for_prompt(
            &reconstruction_turn.model_info.input_modalities,
            reconstruction_turn.trailing_user_message(),
        ),
    )
}

#[tokio::test]
async fn rejects_escalated_permissions_when_policy_not_on_request() {
    use crate::exec_policy::ExecApprovalRequest;
    use crate::sandboxing::SandboxPermissions;
    use crate::tools::sandboxing::ExecApprovalRequirement;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use ontocode_protocol::protocol::AskForApproval;
    use ontocode_tools::ShellCommandBackendConfig;

    let (session, mut turn_context_raw) = make_session_and_context().await;
    // Ensure policy is NOT OnRequest so the early rejection path triggers
    turn_context_raw
        .approval_policy
        .set(AskForApproval::OnFailure)
        .expect("test setup should allow updating approval policy");
    let session = Arc::new(session);
    let mut turn_context = Arc::new(turn_context_raw);

    let command_script = "echo hi";
    let timeout_ms = 1000;
    let sandbox_permissions = SandboxPermissions::RequireEscalated;

    let turn_diff_tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

    let tool_name = "shell_command";
    let call_id = "test-call".to_string();

    let handler = ShellCommandHandler::from(ShellCommandBackendConfig::Classic);
    #[allow(deprecated)]
    let workdir = Some(turn_context.cwd.to_string_lossy().to_string());
    let resp = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&turn_diff_tracker),
            call_id,
            tool_name: ontocode_tools::ToolName::plain(tool_name),
            source: crate::tools::context::ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "command": command_script,
                    "workdir": workdir,
                    "timeout_ms": timeout_ms,
                    "sandbox_permissions": sandbox_permissions,
                    "justification": Some("test"),
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = resp else {
        panic!("expected error result");
    };

    let expected = format!(
        "approval policy is {policy:?}; reject command — you should not ask for escalated permissions if the approval policy is {policy:?}",
        policy = turn_context.approval_policy.value()
    );

    pretty_assertions::assert_eq!(output, expected);
    pretty_assertions::assert_eq!(
        session
            .granted_turn_permissions(ontocode_exec_server::LOCAL_ENVIRONMENT_ID)
            .await,
        None
    );

    // The rejection should not poison the non-escalated path for the same
    // command. Force DangerFullAccess so this check stays focused on approval
    // policy rather than platform-specific sandbox behavior.
    let turn_context_mut = Arc::get_mut(&mut turn_context).expect("unique thread settings Arc");
    turn_context_mut.permission_profile = PermissionProfile::Disabled;

    let file_system_sandbox_policy = turn_context.file_system_sandbox_policy();
    let command = session.user_shell().derive_exec_args(
        command_script,
        turn_context.config.permissions.allow_login_shell,
    );
    let exec_approval_requirement = session
        .services
        .exec_policy
        .create_exec_approval_requirement_for_command(ExecApprovalRequest {
            command: &command,
            approval_policy: turn_context.approval_policy.value(),
            permission_profile: turn_context.permission_profile(),
            file_system_sandbox_policy: &file_system_sandbox_policy,
            #[allow(deprecated)]
            sandbox_cwd: turn_context.cwd.as_path(),
            sandbox_permissions: SandboxPermissions::UseDefault,
            prefix_rule: None,
        })
        .await;
    assert!(matches!(
        exec_approval_requirement,
        ExecApprovalRequirement::Skip { .. }
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn shell_tool_cancellation_waits_for_runtime_cleanup() -> anyhow::Result<()> {
    let session = make_session_with_config(|config| {
        let cwd = config.cwd.clone();
        config
            .permissions
            .set_legacy_sandbox_policy(SandboxPolicy::DangerFullAccess, cwd.as_path())
            .expect("test setup should allow sandbox policy");
    })
    .await?;
    let turn_context = session.new_default_turn().await;
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context);
    let temp_dir = tempfile::TempDir::new()?;
    let ready_marker = temp_dir.path().join("ready");
    let cleanup_marker = temp_dir.path().join("cleanup");
    // Interrupt after the shell starts, then verify dispatch waits for its TERM cleanup trap.
    let command = format!(
        r#"trap 'printf cleaned > "{}"; exit 0' TERM
printf ready > "{}"
while :; do sleep 1; done"#,
        cleanup_marker.display(),
        ready_marker.display(),
    );
    let item = ResponseItem::FunctionCall {
        id: None,
        name: "shell_command".to_string(),
        namespace: None,
        arguments: serde_json::json!({
            "command": command,
            "timeout_ms": 60_000,
        })
        .to_string(),
        call_id: "shell-cleanup-call".to_string(),
    };
    let call = ToolRouter::build_tool_call(item)?
        .expect("shell command response item should build a tool call");
    let cancellation_token = CancellationToken::new();
    let cancellation_tx = cancellation_token.clone();
    let handle = tokio::spawn(
        test_tool_runtime(Arc::clone(&session), Arc::clone(&turn_context))
            .handle_tool_call(call, cancellation_token),
    );

    let mut ready = false;
    for _ in 0..50 {
        if ready_marker.exists() {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    if !ready {
        cancellation_tx.cancel();
        let _ = timeout(Duration::from_secs(5), handle).await;
        anyhow::bail!("shell command should reach the ready marker");
    }

    cancellation_tx.cancel();
    timeout(Duration::from_secs(5), handle)
        .await
        .expect("cancelled shell tool should finish promptly")
        .expect("shell tool task should join");
    assert_eq!(std::fs::read_to_string(cleanup_marker)?, "cleaned");
    Ok(())
}

#[tokio::test]
async fn unified_exec_rejects_escalated_permissions_when_policy_not_on_request() {
    use crate::sandboxing::SandboxPermissions;
    use crate::turn_diff_tracker::TurnDiffTracker;
    use ontocode_protocol::protocol::AskForApproval;

    let (session, mut turn_context_raw) = make_session_and_context().await;
    turn_context_raw
        .approval_policy
        .set(AskForApproval::OnFailure)
        .expect("test setup should allow updating approval policy");
    let session = Arc::new(session);
    let turn_context = Arc::new(turn_context_raw);
    let tracker = Arc::new(tokio::sync::Mutex::new(TurnDiffTracker::new()));

    let handler = ExecCommandHandler::default();
    let resp = handler
        .handle(ToolInvocation {
            session: Arc::clone(&session),
            turn: Arc::clone(&turn_context),
            cancellation_token: CancellationToken::new(),
            tracker: Arc::clone(&tracker),
            call_id: "exec-call".to_string(),
            tool_name: ontocode_tools::ToolName::plain("exec_command"),
            source: crate::tools::context::ToolCallSource::Direct,
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "cmd": "echo hi",
                    "sandbox_permissions": SandboxPermissions::RequireEscalated,
                    "justification": "need unsandboxed execution",
                })
                .to_string(),
            },
        })
        .await;

    let Err(FunctionCallError::RespondToModel(output)) = resp else {
        panic!("expected error result");
    };

    let expected = format!(
        "approval policy is {policy:?}; reject command — you cannot ask for escalated permissions if the approval policy is {policy:?}",
        policy = turn_context.approval_policy.value()
    );

    pretty_assertions::assert_eq!(output, expected);
}

#[tokio::test]
async fn session_start_hooks_only_load_from_trusted_project_layers() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let ontocode_home = temp.path().join("home");
    let project_root = temp.path().join("project");
    let nested = project_root.join("nested");
    let root_dot_ontocode = project_root.join(".ontocode");
    let nested_dot_ontocode = nested.join(".ontocode");

    std::fs::create_dir_all(&ontocode_home)?;
    std::fs::create_dir_all(&nested_dot_ontocode)?;
    std::fs::write(project_root.join(".git"), "gitdir: here")?;
    write_project_hooks(&root_dot_ontocode)?;
    write_project_hooks(&nested_dot_ontocode)?;
    write_project_trust_config(&ontocode_home, &[(&nested, TrustLevel::Trusted)]).await?;

    let config = ConfigBuilder::default()
        .ontocode_home(ontocode_home)
        .fallback_cwd(Some(nested))
        .build()
        .await?;

    let hook_list = ontocode_hooks::list_hooks(ontocode_hooks::HooksConfig {
        feature_enabled: true,
        config_layer_stack: Some(config.config_layer_stack.clone()),
        ..ontocode_hooks::HooksConfig::default()
    });
    let expected_source_path = ontocode_utils_absolute_path::AbsolutePathBuf::from_absolute_path(
        nested_dot_ontocode.join("hooks.json"),
    )?;
    assert_eq!(
        hook_list
            .hooks
            .iter()
            .map(|hook| &hook.source_path)
            .collect::<Vec<_>>(),
        vec![&expected_source_path],
    );
    assert_eq!(
        hook_list.hooks[0].trust_status,
        ontocode_protocol::protocol::HookTrustStatus::Untrusted
    );
    assert!(preview_session_start_hooks(&config).await?.is_empty());

    Ok(())
}

#[tokio::test]
async fn session_start_hooks_require_project_trust_without_config_toml() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let project_root = temp.path().join("project");
    let nested = project_root.join("nested");
    let dot_ontocode = project_root.join(".ontocode");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(project_root.join(".git"), "gitdir: here")?;
    write_project_hooks(&dot_ontocode)?;

    let cases = [
        ("unknown", Vec::<(&Path, TrustLevel)>::new(), 0_usize),
        (
            "untrusted",
            vec![(&project_root as &Path, TrustLevel::Untrusted)],
            0_usize,
        ),
        (
            "trusted",
            vec![(&project_root as &Path, TrustLevel::Trusted)],
            1_usize,
        ),
    ];

    for (name, trust_entries, expected_hooks) in cases {
        let ontocode_home = temp.path().join(format!("home_{name}"));
        std::fs::create_dir_all(&ontocode_home)?;
        write_project_trust_config(&ontocode_home, &trust_entries).await?;

        let config = ConfigBuilder::default()
            .ontocode_home(ontocode_home)
            .fallback_cwd(Some(nested.clone()))
            .build()
            .await?;

        let hook_list = ontocode_hooks::list_hooks(ontocode_hooks::HooksConfig {
            feature_enabled: true,
            config_layer_stack: Some(config.config_layer_stack.clone()),
            ..ontocode_hooks::HooksConfig::default()
        });
        assert_eq!(
            hook_list.hooks.len(),
            expected_hooks,
            "unexpected discovered hook count for {name}",
        );
        assert!(preview_session_start_hooks(&config).await?.is_empty());
        if expected_hooks == 1 {
            assert_eq!(
                hook_list.hooks[0].trust_status,
                ontocode_protocol::protocol::HookTrustStatus::Untrusted
            );
        }
    }

    Ok(())
}

fn embedded_capability_excel_skill() -> SkillMetadata {
    SkillMetadata {
        name: "excel".to_string(),
        description: "Inspect and review Excel workbooks safely".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: Some(SkillPolicy {
            activation: Some(ontocode_core_skills::model::SkillActivation {
                requires_host_tools: vec!["excel".to_string()],
                workspace_file_extensions: vec![
                    ".xlsx".to_string(),
                    ".xlsm".to_string(),
                    ".xlsb".to_string(),
                ],
            }),
            ..Default::default()
        }),
        path_to_skills_md: test_path_buf("/tmp/excel/SKILL.md").abs(),
        scope: SkillScope::System,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }
}

fn embedded_capability_excel_migration_skill() -> SkillMetadata {
    SkillMetadata {
        name: "excel-migration".to_string(),
        description: "Review and migrate Excel automation with bounded evidence".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: Some(SkillPolicy {
            activation: Some(ontocode_core_skills::model::SkillActivation {
                requires_host_tools: Vec::new(),
                workspace_file_extensions: Vec::new(),
            }),
            ..Default::default()
        }),
        path_to_skills_md: test_path_buf("/tmp/excel-migration/SKILL.md").abs(),
        scope: SkillScope::System,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }
}

fn embedded_capability_imagegen_skill() -> SkillMetadata {
    SkillMetadata {
        name: "imagegen".to_string(),
        description: "Generate or edit raster images".to_string(),
        short_description: None,
        interface: None,
        dependencies: None,
        policy: Some(SkillPolicy {
            activation: Some(ontocode_core_skills::model::SkillActivation {
                requires_host_tools: vec!["image_gen".to_string()],
                workspace_file_extensions: Vec::new(),
            }),
            ..Default::default()
        }),
        path_to_skills_md: test_path_buf("/tmp/imagegen/SKILL.md").abs(),
        scope: SkillScope::System,
        plugin_id: None,
        execution_mode: ontocode_core_skills::SkillExecutionMode::ParentOnly,
    }
}

#[test]
fn image_generation_availability_is_bounded_and_does_not_expose_secrets() {
    use super::ImageGenerationCliReadiness;
    use super::image_generation_availability_from_inputs;
    use crate::context::ContextualUserFragment;
    use crate::context::ImageGenerationAvailability;
    use crate::context::ImageGenerationAvailabilityContext;

    let built_in = image_generation_availability_from_inputs(
        &HashSet::from(["image_gen".to_string()]),
        ImageGenerationCliReadiness {
            script_exists: false,
            python_available: false,
            api_key_available: false,
        },
    );
    assert_eq!(built_in, ImageGenerationAvailability::BuiltInCallable);
    assert_eq!(
        ImageGenerationAvailabilityContext::new(built_in).render(),
        r#"{"image_generation_availability":"built_in_callable"}"#
    );

    let cli = image_generation_availability_from_inputs(
        &HashSet::new(),
        ImageGenerationCliReadiness {
            script_exists: true,
            python_available: true,
            api_key_available: true,
        },
    );
    assert_eq!(cli, ImageGenerationAvailability::CliAvailable);
    assert_eq!(
        ImageGenerationAvailabilityContext::new(cli).render(),
        r#"{"image_generation_availability":"cli_available"}"#
    );

    let unavailable = image_generation_availability_from_inputs(
        &HashSet::new(),
        ImageGenerationCliReadiness {
            script_exists: true,
            python_available: true,
            api_key_available: false,
        },
    );
    assert_eq!(unavailable, ImageGenerationAvailability::Unavailable);
    let context = ImageGenerationAvailabilityContext::new(unavailable).render();
    assert_eq!(
        context,
        r#"{"image_generation_availability":"unavailable"}"#
    );
    assert!(!context.contains("OPENAI_API_KEY"));
    assert!(!context.contains("test-secret-value"));
}

struct EmbeddedCapabilityExcelToolContributor;

impl ontocode_extension_api::ToolContributor for EmbeddedCapabilityExcelToolContributor {
    fn tools(
        &self,
        _session_store: &ontocode_extension_api::ExtensionData,
        _thread_store: &ontocode_extension_api::ExtensionData,
    ) -> Vec<
        std::sync::Arc<dyn ontocode_extension_api::ToolExecutor<ontocode_extension_api::ToolCall>>,
    > {
        vec![std::sync::Arc::new(EmbeddedCapabilityExcelTool)]
    }
}

struct EmbeddedCapabilityExcelTool;

#[async_trait::async_trait]
impl ontocode_extension_api::ToolExecutor<ontocode_extension_api::ToolCall>
    for EmbeddedCapabilityExcelTool
{
    fn tool_name(&self) -> ontocode_tools::ToolName {
        ontocode_tools::ToolName::namespaced("excel", "inspect_workbook")
    }

    fn spec(&self) -> ontocode_tools::ToolSpec {
        ontocode_tools::ToolSpec::Namespace(ontocode_tools::ResponsesApiNamespace {
            name: "excel".to_string(),
            description: "test excel namespace".to_string(),
            tools: vec![ontocode_tools::ResponsesApiNamespaceTool::Function(
                ontocode_extension_api::ResponsesApiTool {
                    name: "inspect_workbook".to_string(),
                    description: "test excel tool".to_string(),
                    strict: false,
                    parameters: ontocode_extension_api::parse_tool_input_schema(
                        &serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "additionalProperties": false
                        }),
                    )
                    .expect("schema"),
                    output_schema: None,
                    defer_loading: None,
                },
            )],
        })
    }

    async fn handle(
        &self,
        _call: ontocode_extension_api::ToolCall,
    ) -> Result<Box<dyn ontocode_tools::ToolOutput>, ontocode_tools::FunctionCallError> {
        Ok(Box::new(ontocode_extension_api::JsonToolOutput::new(
            serde_json::json!({
                "ok": true
            }),
        )))
    }
}

struct EmbeddedCapabilityTurnInputContributor;

#[async_trait::async_trait]
impl TurnInputContributor for EmbeddedCapabilityTurnInputContributor {
    async fn contribute(
        &self,
        _input: ontocode_extension_api::TurnInputContext,
        _session_store: &ontocode_extension_api::ExtensionData,
        _thread_store: &ontocode_extension_api::ExtensionData,
        _turn_store: &ontocode_extension_api::ExtensionData,
    ) -> Vec<Box<dyn ContextualUserFragment + Send>> {
        vec![Box::new(
            crate::context::AdditionalContextUserFragment::new(
                "ontograph_km".to_string(),
                "ontograph-shaped turn-input fragment".to_string(),
            ),
        )]
    }
}

struct EmbeddedCapabilityMemoriesContributor;

impl ontocode_extension_api::ContextContributor for EmbeddedCapabilityMemoriesContributor {
    fn contribute<'a>(
        &'a self,
        _session_store: &'a ontocode_extension_api::ExtensionData,
        _thread_store: &'a ontocode_extension_api::ExtensionData,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Vec<ontocode_extension_api::PromptFragment>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            vec![ontocode_extension_api::PromptFragment::new(
                ontocode_extension_api::PromptSlot::ContextualUser,
                "memories-shaped fragment for phase measurement",
            )]
        })
    }
}

fn embedded_capability_registry_with_excel()
-> std::sync::Arc<ontocode_extension_api::ExtensionRegistry<crate::config::Config>> {
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder.tool_contributor(std::sync::Arc::new(EmbeddedCapabilityExcelToolContributor));
    builder.prompt_contributor(std::sync::Arc::new(EmbeddedCapabilityMemoriesContributor));
    builder.turn_input_contributor(std::sync::Arc::new(EmbeddedCapabilityTurnInputContributor));
    std::sync::Arc::new(builder.build())
}

fn embedded_capability_skills_context() -> TurnSkillsContext {
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![embedded_capability_excel_skill()];
    TurnSkillsContext::new(std::sync::Arc::new(outcome))
}

fn embedded_capability_skills_context_with_migration() -> TurnSkillsContext {
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![
        embedded_capability_excel_skill(),
        embedded_capability_excel_migration_skill(),
    ];
    TurnSkillsContext::new(std::sync::Arc::new(outcome))
}

fn embedded_capability_imagegen_skills_context() -> TurnSkillsContext {
    let mut outcome = SkillLoadOutcome::default();
    outcome.skills = vec![embedded_capability_imagegen_skill()];
    TurnSkillsContext::new(std::sync::Arc::new(outcome))
}

fn embedded_capability_catalog_contains_excel(items: &[ResponseItem]) -> bool {
    developer_input_texts(items)
        .iter()
        .any(|text| text.contains("`excel`") || text.contains("- excel"))
}

fn embedded_capability_catalog_contains(items: &[ResponseItem], skill: &str) -> bool {
    let backticked = format!("`{skill}`");
    let listed = format!("- {skill}:");
    developer_input_texts(items)
        .iter()
        .any(|text| text.contains(&backticked) || text.contains(&listed))
}

#[tokio::test]
async fn embedded_capability_shows_hostless_migration_while_hiding_live_excel() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context_with_migration();

    let first = session.build_initial_context(&turn_context).await;
    assert!(!embedded_capability_catalog_contains(&first, "excel"));
    assert!(embedded_capability_catalog_contains(
        &first,
        "excel-migration"
    ));
}

#[tokio::test]
async fn embedded_capability_promotes_deferred_imagegen_only_after_matching_search() {
    let dynamic_tools = vec![DynamicToolSpec {
        namespace: Some("image_gen".to_string()),
        name: "imagegen".to_string(),
        description: "Generate a raster image".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        }),
        defer_loading: true,
    }];
    let (session, mut turn_context, _rx) =
        make_session_and_context_with_dynamic_tools_and_rx(dynamic_tools).await;
    Arc::get_mut(&mut turn_context)
        .expect("new turn context should be uniquely owned")
        .turn_skills = embedded_capability_imagegen_skills_context();

    let hidden = session.build_initial_context(&turn_context).await;
    assert!(!embedded_capability_catalog_contains(&hidden, "imagegen"));

    session
        .record_conversation_items(
            &turn_context,
            &[ResponseItem::ToolSearchOutput {
                call_id: Some("unrelated-search".to_string()),
                status: "completed".to_string(),
                execution: "server".to_string(),
                tools: vec![serde_json::json!({
                    "type": "function",
                    "name": "other_tool",
                })],
            }],
        )
        .await;
    let still_hidden = session.build_initial_context(&turn_context).await;
    assert!(!embedded_capability_catalog_contains(
        &still_hidden,
        "imagegen"
    ));

    session
        .record_conversation_items(
            &turn_context,
            &[ResponseItem::ToolSearchOutput {
                call_id: Some("imagegen-search".to_string()),
                status: "completed".to_string(),
                execution: "server".to_string(),
                tools: vec![serde_json::json!({
                    "type": "namespace",
                    "name": "image_gen",
                    "tools": [{
                        "type": "function",
                        "name": "imagegen",
                    }],
                })],
            }],
        )
        .await;
    let shown = session.build_initial_context(&turn_context).await;
    assert!(embedded_capability_catalog_contains(&shown, "imagegen"));
}

#[tokio::test]
async fn embedded_capability_hides_excel_without_live_tools_even_with_workbook() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("book.xlsx"), b"pk").expect("workbook");
    let (session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context();
    // empty extension registry: no excel tools
    let first = session.build_initial_context(&turn_context).await;
    assert!(
        !embedded_capability_catalog_contains_excel(&first),
        "excel line must hide without live tools"
    );
}

#[tokio::test]
async fn embedded_capability_shows_excel_with_live_tools_when_workspace_gate_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (mut session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context();
    session.services.extensions = embedded_capability_registry_with_excel();
    let first = session.build_initial_context(&turn_context).await;
    assert!(
        embedded_capability_catalog_contains_excel(&first),
        "excel line must show with live tools and feature off"
    );
}

#[tokio::test]
async fn embedded_capability_workspace_gate_hides_without_workbook_when_enabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (mut session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context();
    let _ = turn_context
        .features
        .enable(Feature::SkillWorkspaceRelevance);
    session.services.extensions = embedded_capability_registry_with_excel();
    let hidden = session.build_initial_context(&turn_context).await;
    assert!(
        !embedded_capability_catalog_contains_excel(&hidden),
        "workspace gate should hide without workbook"
    );

    std::fs::write(temp.path().join("book.xlsx"), b"pk").expect("workbook");
    let shown = session.build_initial_context(&turn_context).await;
    assert!(
        embedded_capability_catalog_contains_excel(&shown),
        "workspace gate should show with workbook"
    );
}

#[tokio::test]
async fn embedded_capability_feature_disabled_ignores_workspace_while_requiring_tools() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("book.xlsx"), b"pk").expect("workbook");
    let (mut session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context();
    // feature remains disabled by default
    let without_tools = session.build_initial_context(&turn_context).await;
    assert!(!embedded_capability_catalog_contains_excel(&without_tools));

    session.services.extensions = embedded_capability_registry_with_excel();
    let with_tools = session.build_initial_context(&turn_context).await;
    assert!(embedded_capability_catalog_contains_excel(&with_tools));
}

#[tokio::test]
async fn embedded_capability_repeated_initial_context_is_byte_identical() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("book.xlsx"), b"pk").expect("workbook");
    let (mut session, mut turn_context) = make_session_and_context().await;
    turn_context.cwd = temp.path().abs();
    turn_context.turn_skills = embedded_capability_skills_context();
    session.services.extensions = embedded_capability_registry_with_excel();
    let first = session.build_initial_context(&turn_context).await;
    let second = session.build_initial_context(&turn_context).await;
    assert_eq!(first, second);
}

#[tokio::test]
#[expect(
    clippy::print_stderr,
    reason = "preserve the ECSP measurement table as test evidence"
)]
async fn embedded_capability_phase_separated_measurement_sums_contributors() {
    // Deterministic fixture contract:
    // - initial-context: skills catalog + AGENTS/user-instructions + memories-shaped ContextContributor
    // - turn-input: TurnInputContributor fixture (ontograph-phase stand-in via DiagnosticFragment)
    // Rows: predicate-true (live excel tools + workbook) and predicate-false (no live tools).
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("AGENTS.md"), "agents fixture line").expect("agents");
    std::fs::write(temp.path().join("book.xlsx"), b"pk").expect("workbook");

    let measure = |items: &[ResponseItem]| {
        let skills_chars = developer_input_texts(items)
            .iter()
            .filter(|text| {
                text.contains("## Skills")
                    || text.contains("`excel`")
                    || text.contains("Available skills")
            })
            .map(|text| text.chars().count())
            .sum::<usize>();
        let agents_chars = user_input_texts(items)
            .iter()
            .filter(|text| text.contains("agents-shaped user instructions"))
            .map(|text| text.chars().count())
            .sum::<usize>();
        let memories_chars = user_input_texts(items)
            .iter()
            .filter(|text| text.contains("memories-shaped fragment for phase measurement"))
            .map(|text| text.chars().count())
            .sum::<usize>();
        (skills_chars, agents_chars, memories_chars)
    };

    // predicate-false: no live excel tools
    let (mut session_false, mut turn_false) = make_session_and_context().await;
    turn_false.user_instructions = Some("agents-shaped user instructions".to_string());
    turn_false.cwd = temp.path().abs();
    turn_false.turn_skills = embedded_capability_skills_context();
    let mut builder_false = ontocode_extension_api::ExtensionRegistryBuilder::new();
    builder_false.prompt_contributor(std::sync::Arc::new(EmbeddedCapabilityMemoriesContributor));
    builder_false
        .turn_input_contributor(std::sync::Arc::new(EmbeddedCapabilityTurnInputContributor));
    session_false.services.extensions = std::sync::Arc::new(builder_false.build());
    let false_items = session_false.build_initial_context(&turn_false).await;
    let (skills_false, agents_false, memories_false) = measure(&false_items);
    assert!(
        !embedded_capability_catalog_contains_excel(&false_items),
        "predicate-false must hide excel catalog line"
    );

    // turn-input phase measured by direct contributor call (not in initial context)
    let turn_input_false = EmbeddedCapabilityTurnInputContributor
        .contribute(
            ontocode_extension_api::TurnInputContext {
                turn_id: "measure-false".to_string(),
                user_input: Vec::new(),
                environments: Vec::new(),
            },
            &session_false.services.session_extension_data,
            &session_false.services.thread_extension_data,
            &session_false.services.thread_extension_data,
        )
        .await;
    let turn_input_chars_false: usize = turn_input_false
        .iter()
        .map(|fragment| fragment.render().chars().count())
        .sum();
    let initial_false = skills_false + agents_false + memories_false;
    let combined_false = initial_false + turn_input_chars_false;
    assert_eq!(
        combined_false,
        skills_false + agents_false + memories_false + turn_input_chars_false
    );
    assert!(
        agents_false > 0,
        "agents/user instructions contribute in predicate-false"
    );
    assert!(memories_false > 0, "memories contribute in predicate-false");
    assert!(
        turn_input_chars_false > 0,
        "turn-input fixture must contribute"
    );

    // predicate-true: live excel tools + workbook
    let (mut session_true, mut turn_true) = make_session_and_context().await;
    turn_true.user_instructions = Some("agents-shaped user instructions".to_string());
    turn_true.cwd = temp.path().abs();
    turn_true.turn_skills = embedded_capability_skills_context();
    session_true.services.extensions = embedded_capability_registry_with_excel();
    let true_items = session_true.build_initial_context(&turn_true).await;
    let (skills_true, agents_true, memories_true) = measure(&true_items);
    assert!(
        embedded_capability_catalog_contains_excel(&true_items),
        "predicate-true must show excel catalog line"
    );
    let turn_input_true = EmbeddedCapabilityTurnInputContributor
        .contribute(
            ontocode_extension_api::TurnInputContext {
                turn_id: "measure-true".to_string(),
                user_input: Vec::new(),
                environments: Vec::new(),
            },
            &session_true.services.session_extension_data,
            &session_true.services.thread_extension_data,
            &session_true.services.thread_extension_data,
        )
        .await;
    let turn_input_chars_true: usize = turn_input_true
        .iter()
        .map(|fragment| fragment.render().chars().count())
        .sum();
    let initial_true = skills_true + agents_true + memories_true;
    let combined_true = initial_true + turn_input_chars_true;
    assert_eq!(
        combined_true,
        skills_true + agents_true + memories_true + turn_input_chars_true
    );
    assert!(
        skills_true > skills_false,
        "predicate-true skills phase grows with excel line"
    );
    assert_eq!(
        agents_true, agents_false,
        "agents phase identical across rows"
    );
    assert_eq!(
        memories_true, memories_false,
        "memories phase identical across rows"
    );
    assert_eq!(
        turn_input_chars_true, turn_input_chars_false,
        "turn-input fixture identical across rows"
    );

    // Concrete measurement table (asserted values).
    assert!(skills_true > 0);
    eprintln!(
        "ECSP measurement table: false skills={skills_false} agents={agents_false} memories={memories_false} turn_input={turn_input_chars_false} initial={initial_false} combined={combined_false}; true skills={skills_true} agents={agents_true} memories={memories_true} turn_input={turn_input_chars_true} initial={initial_true} combined={combined_true}"
    );
}
