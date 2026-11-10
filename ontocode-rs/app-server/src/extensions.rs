use std::sync::Arc;
use std::sync::Weak;

use ontocode_analytics::AppServerRpcTransport;
use ontocode_app_server_protocol::ServerNotification;
use ontocode_app_server_protocol::ThreadGoal;
use ontocode_app_server_protocol::ThreadGoalUpdatedNotification;
use ontocode_core::NewThread;
use ontocode_core::StartThreadOptions;
use ontocode_core::ThreadManager;
use ontocode_core::config::Config;
use ontocode_extension_api::AgentSpawnFuture;
use ontocode_extension_api::AgentSpawner;
use ontocode_extension_api::ExtensionEventSink;
use ontocode_extension_api::ExtensionRegistry;
use ontocode_extension_api::ExtensionRegistryBuilder;
use ontocode_goal_extension::GoalService;
use ontocode_login::AuthManager;
use ontocode_protocol::ThreadId;
use ontocode_protocol::error::CodexErr;
use ontocode_protocol::protocol::Event;
use ontocode_protocol::protocol::EventMsg;
use ontocode_rollout::StateDbHandle;

use crate::outgoing_message::OutgoingMessageSender;
use crate::thread_state::ThreadListenerCommand;
use crate::thread_state::ThreadStateManager;

#[cfg(test)]
#[path = "memory_effectiveness_tests.rs"]
mod memory_effectiveness_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExtensionHostSurface {
    Embedded,
    Shared,
}

impl ExtensionHostSurface {
    pub(crate) fn from_rpc_transport(rpc_transport: AppServerRpcTransport) -> Self {
        match rpc_transport {
            AppServerRpcTransport::InProcess => Self::Embedded,
            AppServerRpcTransport::Stdio | AppServerRpcTransport::Websocket => Self::Shared,
        }
    }
}

pub(crate) fn thread_extensions<S>(
    guardian_agent_spawner: S,
    event_sink: Arc<dyn ExtensionEventSink>,
    auth_manager: Arc<AuthManager>,
    host_surface: ExtensionHostSurface,
    state_db: Option<StateDbHandle>,
    thread_manager: Weak<ThreadManager>,
    goal_service: Arc<GoalService>,
) -> Arc<ExtensionRegistry<Config>>
where
    S: AgentSpawner<StartThreadOptions, Spawned = NewThread, Error = CodexErr> + 'static,
{
    let mut builder = ExtensionRegistryBuilder::<Config>::with_event_sink(event_sink);
    if let Some(state_db) = state_db.clone() {
        ontocode_goal_extension::install_with_backend(
            &mut builder,
            state_db,
            ontocode_otel::global(),
            thread_manager,
            goal_service,
            |config: &Config| config.features.enabled(ontocode_features::Feature::Goals),
        );
    }
    ontocode_guardian::install(&mut builder, guardian_agent_spawner);
    ontocode_memories_extension::install(&mut builder, ontocode_otel::global(), state_db);
    ontocode_web_search_extension::install(&mut builder, auth_manager);
    if host_surface == ExtensionHostSurface::Embedded {
        ontocode_excel_extension::install(&mut builder);
        ontocode_lctx_extension::install(&mut builder);
        // Ontograph is local-only by accepted ADR: only the KM context and agent
        // knowledge surface is installed, solely on the in-process embedded
        // host, and its three authoring verbs stay behind the default-off
        // `agent_authored_project_knowledge` feature.
        ontocode_ontograph_extension::install_agent_knowledge_surface_with_metrics(
            &mut builder,
            ontocode_otel::global(),
        );
    }
    Arc::new(builder.build())
}

pub(crate) fn app_server_extension_event_sink(
    outgoing: Arc<OutgoingMessageSender>,
    thread_state_manager: ThreadStateManager,
) -> Arc<dyn ExtensionEventSink> {
    Arc::new(AppServerExtensionEventSink {
        outgoing,
        thread_state_manager,
    })
}

struct AppServerExtensionEventSink {
    outgoing: Arc<OutgoingMessageSender>,
    thread_state_manager: ThreadStateManager,
}

impl ExtensionEventSink for AppServerExtensionEventSink {
    fn emit(&self, event: Event) {
        match event.msg {
            EventMsg::ThreadGoalUpdated(thread_goal_event) => {
                let thread_id = thread_goal_event.thread_id;
                let turn_id = thread_goal_event.turn_id;
                let goal: ThreadGoal = thread_goal_event.goal.into();
                if let Some(listener_command_tx) = self
                    .thread_state_manager
                    .current_listener_command_tx(thread_id)
                {
                    let command = ThreadListenerCommand::EmitThreadGoalUpdated {
                        turn_id: turn_id.clone(),
                        goal: goal.clone(),
                    };
                    if listener_command_tx.send(command).is_ok() {
                        return;
                    }
                    tracing::warn!(
                        "failed to enqueue extension goal update for {thread_id}: listener command channel is closed"
                    );
                }
                self.outgoing
                    .try_send_server_notification(ServerNotification::ThreadGoalUpdated(
                        ThreadGoalUpdatedNotification {
                            thread_id: thread_id.to_string(),
                            turn_id,
                            goal,
                        },
                    ));
            }
            msg => {
                tracing::debug!(event_id = %event.id, ?msg, "dropping unsupported extension event");
            }
        }
    }
}

pub(crate) fn guardian_agent_spawner(
    thread_manager: Weak<ThreadManager>,
) -> impl AgentSpawner<StartThreadOptions, Spawned = NewThread, Error = CodexErr> {
    move |forked_from_thread_id: ThreadId,
          options: StartThreadOptions|
          -> AgentSpawnFuture<'static, NewThread, CodexErr> {
        let thread_manager = thread_manager.clone();
        Box::pin(async move {
            let thread_manager = thread_manager.upgrade().ok_or_else(|| {
                CodexErr::UnsupportedOperation("thread manager dropped".to_string())
            })?;
            thread_manager
                .spawn_subagent(forked_from_thread_id, options)
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ontocode_analytics::AnalyticsEventsClient;
    use ontocode_app_server_protocol::ServerNotification;
    use ontocode_app_server_protocol::ThreadGoal as AppServerThreadGoal;
    use ontocode_app_server_protocol::ThreadGoalStatus as AppServerThreadGoalStatus;
    use ontocode_extension_api::ExtensionData;
    use ontocode_extension_api::ThreadStartInput;
    use ontocode_extension_api::ToolName;
    use ontocode_login::AuthManager;
    use ontocode_login::CodexAuth;
    use ontocode_protocol::protocol::ThreadGoal;
    use ontocode_protocol::protocol::ThreadGoalStatus;
    use ontocode_protocol::protocol::ThreadGoalUpdatedEvent;
    use pretty_assertions::assert_eq;
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    use super::*;
    use crate::outgoing_message::OutgoingEnvelope;
    use crate::outgoing_message::OutgoingMessage;

    struct NoopTestEventSink;

    impl ExtensionEventSink for NoopTestEventSink {
        fn emit(&self, _event: Event) {}
    }

    fn unused_test_spawner(
        _forked_from_thread_id: ThreadId,
        _options: StartThreadOptions,
    ) -> AgentSpawnFuture<'static, NewThread, CodexErr> {
        Box::pin(async { panic!("spawner not used") })
    }

    #[test]
    fn thread_extensions_register_local_tools_only_for_embedded_hosts() {
        assert_eq!(
            ExtensionHostSurface::from_rpc_transport(AppServerRpcTransport::InProcess),
            ExtensionHostSurface::Embedded
        );
        for rpc_transport in [
            AppServerRpcTransport::Stdio,
            AppServerRpcTransport::Websocket,
        ] {
            assert_eq!(
                ExtensionHostSurface::from_rpc_transport(rpc_transport),
                ExtensionHostSurface::Shared
            );
        }

        let embedded_registry = thread_extensions(
            unused_test_spawner,
            Arc::new(NoopTestEventSink),
            AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
            ExtensionHostSurface::Embedded,
            /*state_db*/ None,
            Weak::new(),
            Arc::new(GoalService::new()),
        );
        let mut excel_builder = ExtensionRegistryBuilder::<Config>::new();
        ontocode_excel_extension::install(&mut excel_builder);
        let excel_registry = excel_builder.build();
        assert_eq!(
            namespace_tool_names(&embedded_registry, "excel"),
            namespace_tool_names(&excel_registry, "excel")
        );
        assert_eq!(
            namespace_tool_names(&embedded_registry, "lctx"),
            vec![ToolName::namespaced("lctx", "read")]
        );
        // Ontograph is embedded-only. With `agent_authored_project_knowledge`
        // off, the embedded host exposes only the read-only KM context verb and
        // none of the three authoring verbs.
        assert_eq!(
            namespace_tool_names(&embedded_registry, "ontograph"),
            vec![ToolName::namespaced("ontograph", "km_context")]
        );

        let shared_registry = thread_extensions(
            unused_test_spawner,
            Arc::new(NoopTestEventSink),
            AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
            ExtensionHostSurface::Shared,
            /*state_db*/ None,
            Weak::new(),
            Arc::new(GoalService::new()),
        );

        assert_eq!(
            namespace_tool_names(&shared_registry, "excel"),
            Vec::<ToolName>::new()
        );
        // The shared host never sees any Ontograph verb, feature on or off.
        assert_eq!(
            namespace_tool_names(&shared_registry, "ontograph"),
            Vec::<ToolName>::new()
        );
    }

    #[tokio::test]
    async fn thread_extensions_register_goal_tools_when_state_db_exists() {
        let ontocode_home = tempfile::tempdir().expect("create temp dir");
        let mut config = ontocode_core::config::ConfigBuilder::default()
            .ontocode_home(ontocode_home.path().to_path_buf())
            .build()
            .await
            .expect("build test config");
        config
            .features
            .enable(ontocode_features::Feature::Goals)
            .expect("enable goals feature");
        let state_db = ontocode_state::StateRuntime::init(
            config.sqlite_home.clone(),
            config.model_provider_id.clone(),
        )
        .await
        .expect("initialize state db");
        let registry = thread_extensions(
            unused_test_spawner,
            Arc::new(NoopTestEventSink),
            AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
            ExtensionHostSurface::Shared,
            Some(state_db),
            Weak::new(),
            Arc::new(GoalService::new()),
        );
        let session_store = ExtensionData::new("session");
        let thread_store = ExtensionData::new(ThreadId::default().to_string());
        let session_source = ontocode_protocol::protocol::SessionSource::Exec;

        for contributor in registry.thread_lifecycle_contributors() {
            contributor
                .on_thread_start(ThreadStartInput {
                    config: &config,
                    session_source: &session_source,
                    persistent_thread_state_available: true,
                    session_store: &session_store,
                    thread_store: &thread_store,
                })
                .await;
        }

        let mut goal_tools = registry
            .tool_contributors()
            .iter()
            .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
            .map(|tool| tool.tool_name())
            .filter(|tool_name| {
                ["create_goal", "get_goal", "update_goal"].contains(&tool_name.name.as_str())
            })
            .collect::<Vec<_>>();
        goal_tools.sort();
        assert_eq!(
            goal_tools,
            vec![
                ToolName::plain("create_goal"),
                ToolName::plain("get_goal"),
                ToolName::plain("update_goal"),
            ]
        );
    }

    fn namespace_tool_names(
        registry: &ExtensionRegistry<Config>,
        namespace: &str,
    ) -> Vec<ToolName> {
        registry
            .tool_contributors()
            .iter()
            .flat_map(|contributor| {
                contributor.tools(
                    &ExtensionData::new("session"),
                    &ExtensionData::new("thread"),
                )
            })
            .map(|tool| tool.tool_name())
            .filter(|tool_name| tool_name.namespace.as_deref() == Some(namespace))
            .collect()
    }

    /// Builds the embedded registry and drives thread start so feature-gated
    /// contributors observe `config`, then lists its `ontograph` tools.
    async fn embedded_ontograph_tool_names(config: &Config) -> Vec<ToolName> {
        let registry = thread_extensions(
            unused_test_spawner,
            Arc::new(NoopTestEventSink),
            AuthManager::from_auth_for_testing(CodexAuth::from_api_key("dummy")),
            ExtensionHostSurface::Embedded,
            /*state_db*/ None,
            Weak::new(),
            Arc::new(GoalService::new()),
        );
        let session_store = ExtensionData::new("session");
        let thread_store = ExtensionData::new(ThreadId::default().to_string());
        let session_source = ontocode_protocol::protocol::SessionSource::Exec;
        for contributor in registry.thread_lifecycle_contributors() {
            contributor
                .on_thread_start(ThreadStartInput {
                    config,
                    session_source: &session_source,
                    persistent_thread_state_available: true,
                    session_store: &session_store,
                    thread_store: &thread_store,
                })
                .await;
        }
        let mut names = registry
            .tool_contributors()
            .iter()
            .flat_map(|contributor| contributor.tools(&session_store, &thread_store))
            .map(|tool| tool.tool_name())
            .filter(|tool_name| tool_name.namespace.as_deref() == Some("ontograph"))
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    // The accepted host ADR permits exactly four embedded verbs once
    // `agent_authored_project_knowledge` is enabled: the read-only KM context
    // verb plus the three authoring verbs, and nothing else.
    #[tokio::test]
    async fn embedded_host_exposes_exactly_four_ontograph_verbs_when_feature_enabled() {
        let ontocode_home = tempfile::tempdir().expect("create temp dir");
        let mut config = ontocode_core::config::ConfigBuilder::default()
            .ontocode_home(ontocode_home.path().to_path_buf())
            .build()
            .await
            .expect("build test config");

        // Default-off: only the read-only context verb is exposed.
        assert_eq!(
            embedded_ontograph_tool_names(&config).await,
            vec![ToolName::namespaced("ontograph", "km_context")]
        );

        config
            .features
            .enable(ontocode_features::Feature::AgentAuthoredProjectKnowledge)
            .expect("enable agent-authored project knowledge");

        let mut expected = vec![
            ToolName::namespaced("ontograph", "km_author"),
            ToolName::namespaced("ontograph", "km_context"),
            ToolName::namespaced("ontograph", "km_list"),
            ToolName::namespaced("ontograph", "km_withdraw"),
        ];
        expected.sort();
        assert_eq!(embedded_ontograph_tool_names(&config).await, expected);
    }

    #[tokio::test]
    async fn app_server_event_sink_forwards_thread_goal_updates() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(4);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            outgoing_tx,
            AnalyticsEventsClient::disabled(),
        ));
        let sink = app_server_extension_event_sink(outgoing, ThreadStateManager::new());
        let thread_id = ThreadId::default();

        sink.emit(Event {
            id: "call-1".to_string(),
            msg: EventMsg::ThreadGoalUpdated(ThreadGoalUpdatedEvent {
                thread_id,
                turn_id: Some("turn-1".to_string()),
                goal: ThreadGoal {
                    thread_id,
                    objective: "wire extension events".to_string(),
                    status: ThreadGoalStatus::Active,
                    token_budget: Some(123),
                    tokens_used: 45,
                    time_used_seconds: 6,
                    created_at: 7,
                    updated_at: 8,
                },
            }),
        });

        let envelope = timeout(Duration::from_secs(1), outgoing_rx.recv())
            .await
            .expect("timed out waiting for forwarded extension event")
            .expect("outgoing channel closed unexpectedly");
        let OutgoingEnvelope::Broadcast { message } = envelope else {
            panic!("expected broadcast notification");
        };
        let OutgoingMessage::AppServerNotification(ServerNotification::ThreadGoalUpdated(
            notification,
        )) = message
        else {
            panic!("expected thread goal updated notification");
        };

        assert_eq!(
            ThreadGoalUpdatedNotification {
                thread_id: thread_id.to_string(),
                turn_id: Some("turn-1".to_string()),
                goal: AppServerThreadGoal {
                    thread_id: thread_id.to_string(),
                    objective: "wire extension events".to_string(),
                    status: AppServerThreadGoalStatus::Active,
                    token_budget: Some(123),
                    tokens_used: 45,
                    time_used_seconds: 6,
                    created_at: 7,
                    updated_at: 8,
                },
            },
            notification
        );
    }

    #[tokio::test]
    async fn app_server_event_sink_enqueues_goal_updates_with_turn_ids_for_listener_ordering() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(4);
        let outgoing = Arc::new(OutgoingMessageSender::new(
            outgoing_tx,
            AnalyticsEventsClient::disabled(),
        ));
        let thread_state_manager = ThreadStateManager::new();
        let thread_id = ThreadId::default();
        let (listener_tx, mut listener_rx) = mpsc::unbounded_channel();
        thread_state_manager.register_listener_command_tx(thread_id, listener_tx);
        let sink = app_server_extension_event_sink(outgoing, thread_state_manager);

        sink.emit(Event {
            id: "call-ordered".to_string(),
            msg: EventMsg::ThreadGoalUpdated(ThreadGoalUpdatedEvent {
                thread_id,
                turn_id: Some("turn-ordered".to_string()),
                goal: ThreadGoal {
                    thread_id,
                    objective: "preserve listener order".to_string(),
                    status: ThreadGoalStatus::Active,
                    token_budget: Some(456),
                    tokens_used: 78,
                    time_used_seconds: 9,
                    created_at: 10,
                    updated_at: 11,
                },
            }),
        });

        let command = timeout(Duration::from_secs(1), listener_rx.recv())
            .await
            .expect("timed out waiting for listener command")
            .expect("listener command channel closed");
        let ThreadListenerCommand::EmitThreadGoalUpdated { turn_id, goal } = command else {
            panic!("expected ordered thread goal update command");
        };
        assert_eq!(turn_id, Some("turn-ordered".to_string()));
        assert_eq!(
            goal,
            AppServerThreadGoal {
                thread_id: thread_id.to_string(),
                objective: "preserve listener order".to_string(),
                status: AppServerThreadGoalStatus::Active,
                token_budget: Some(456),
                tokens_used: 78,
                time_used_seconds: 9,
                created_at: 10,
                updated_at: 11,
            }
        );
        assert!(outgoing_rx.try_recv().is_err());
    }
}
