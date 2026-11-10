use std::sync::Arc;

use anyhow::Result;
use core_test_support::responses;
use core_test_support::test_ontocode::TestOntocodeHarness;
use core_test_support::test_ontocode::test_ontocode;
use core_test_support::wait_for_event;
use ontocode_extension_api::ExtensionRegistryBuilder;
use ontocode_features::Feature;
use ontocode_protocol::protocol::EventMsg;
use ontocode_protocol::protocol::Op;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

use super::EXAMPLE;
use super::KM_ID;
use super::QUESTION;
use super::write_km;
use crate::install_agent_knowledge_surface;
use crate::km_context::KM_CONTEXT_START_MARKER;
use crate::km_import::run_km_import_for_project;

#[derive(Clone, Copy)]
enum CompactionPath {
    Local,
    RemoteV2,
    RemoteFailed,
    RemoteLiteral,
    RemoteDropped,
    RemoteRetained,
}

#[derive(Clone, Copy)]
enum AutomaticCompactionPhase {
    PreTurn,
    MidTurn,
}

#[derive(Clone, Copy)]
struct AutomaticCompactionCase {
    phase: AutomaticCompactionPhase,
    path: CompactionPath,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_local() -> Result<()> {
    characterize_compaction(CompactionPath::Local).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_remote_dropped() -> Result<()> {
    characterize_compaction(CompactionPath::RemoteDropped).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_remote_retained() -> Result<()> {
    characterize_compaction(CompactionPath::RemoteRetained).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_failed() -> Result<()> {
    characterize_compaction(CompactionPath::RemoteFailed).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_literal() -> Result<()> {
    characterize_compaction(CompactionPath::RemoteLiteral).await
}

async fn characterize_compaction(path: CompactionPath) -> Result<()> {
    let builder = test_ontocode()
        .with_config(move |config| {
            let _ = config.features.disable(Feature::LineageRouting);
            let _ = config.features.disable(Feature::LineageShadow);
            config.model_auto_compact_token_limit = Some(200_000);
            // Reuse the host's local/remote dispatch decision and mock transport.
            // No endpoint or credentials are resolved by this test.
            if matches!(path, CompactionPath::Local) {
                config.model_provider.name = "OpenAI (test)".to_string();
            }
        })
        .with_workspace_setup(|cwd, _fs| async move {
            let root = cwd.as_path().canonicalize()?;
            std::fs::create_dir(root.join(".git"))?;
            let package = write_km(&root, KM_ID, EXAMPLE)?;
            run_km_import_for_project(&package, &root.join(".ontocode/ontograph/km/_store"), &root)
                .expect("import isolated compaction fixture");
            Ok(())
        })
        .with_extension_registry_factory(|_runtime, _thread_manager| {
            let mut registry = ExtensionRegistryBuilder::new();
            install_agent_knowledge_surface(&mut registry);
            Arc::new(registry.build())
        });
    let harness = TestOntocodeHarness::with_builder(builder).await?;
    let local = matches!(path, CompactionPath::Local);
    let response_count = if local || matches!(path, CompactionPath::RemoteLiteral) {
        6
    } else {
        5
    };
    let mock = responses::mount_sse_sequence(
        harness.server(),
        (0..response_count)
            .map(|index| {
                responses::sse(vec![
                    responses::ev_assistant_message(
                        &format!("m{index}"),
                        "COMPACTION_FIXTURE_REPLY",
                    ),
                    responses::ev_completed(&format!("r{index}")),
                ])
            })
            .collect(),
    )
    .await;

    harness.submit(QUESTION).await?;
    harness.submit(QUESTION).await?;
    let before = mock.requests();
    assert_eq!(before.len(), 2);
    let fragment = before[0]
        .input()
        .into_iter()
        .find(|item| item.to_string().contains(KM_CONTEXT_START_MARKER))
        .expect("real KM must be admitted before compaction");
    assert!(fragment.to_string().contains(KM_ID));
    assert_eq!(km_items(&before[0]), vec![fragment.clone()]);
    assert_eq!(km_items(&before[1]), vec![fragment.clone()]);

    let literal = fragment["content"][0]["text"].as_str().expect("KM text");
    if matches!(path, CompactionPath::RemoteLiteral) {
        harness.submit(literal).await?;
    }
    let compact_mock = if matches!(path, CompactionPath::RemoteFailed) {
        Some(
            responses::mount_compact_json_once(harness.server(), json!({"output": "invalid"}))
                .await,
        )
    } else if matches!(path, CompactionPath::RemoteLiteral) {
        Some(
            responses::mount_compact_user_history_with_summary_once(
                harness.server(),
                "LITERAL_SUMMARY",
            )
            .await,
        )
    } else if local {
        None
    } else {
        let mut output = Vec::new();
        if matches!(path, CompactionPath::RemoteRetained) {
            // Return the actual captured contextual user message, not a synthetic marker.
            output.push(fragment.clone());
        }
        output.push(json!({
            "type": "compaction",
            "encrypted_content": "COMPACTION_FIXTURE_SUMMARY"
        }));
        Some(
            responses::mount_compact_json_once(harness.server(), json!({ "output": output })).await,
        )
    };
    harness.test().ontocode.submit(Op::Compact).await?;
    let completed = wait_for_event(&harness.test().ontocode, |event| {
        matches!(event, EventMsg::TurnComplete(_) | EventMsg::Error(_))
    })
    .await;
    if matches!(path, CompactionPath::RemoteFailed) {
        assert!(matches!(completed, EventMsg::Error(_)), "{completed:?}");
    } else {
        assert!(
            matches!(completed, EventMsg::TurnComplete(_)),
            "{completed:?}"
        );
    }

    // An irrelevant turn observes replacement history before any new KM admission.
    harness
        .submit("Sourdough baking temperature kneading instructions yeast flour water today")
        .await?;
    harness.submit(QUESTION).await?;
    harness.submit(QUESTION).await?;
    let requests = mock.requests();
    assert_eq!(requests.len(), response_count);
    let after_index = if let Some(compact_mock) = compact_mock {
        let request = compact_mock.single_request();
        assert_eq!(request.path(), "/v1/responses/compact");
        if matches!(path, CompactionPath::RemoteLiteral) {
            assert_eq!(km_items(&request).len(), 2);
            for followup in &requests[3..] {
                let copies = km_items(followup);
                assert_eq!(copies.len(), 2);
                assert!(
                    copies
                        .iter()
                        .all(|item| item["content"][0]["text"] == literal)
                );
            }
            return Ok(());
        }
        assert_eq!(km_items(&request), vec![fragment.clone()]);
        2
    } else {
        assert_eq!(km_items(&requests[2]), vec![fragment.clone()]);
        3
    };
    let surviving = match path {
        CompactionPath::Local | CompactionPath::RemoteRetained | CompactionPath::RemoteFailed => {
            vec![fragment.clone()]
        }
        CompactionPath::RemoteDropped => Vec::new(),
        CompactionPath::RemoteLiteral | CompactionPath::RemoteV2 => unreachable!("separate cases"),
    };
    assert_eq!(km_items(&requests[after_index]), surviving);
    assert_eq!(km_items(&requests[after_index + 1]), vec![fragment.clone()]);
    assert_eq!(km_items(&requests[after_index + 2]), vec![fragment]);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_auto_preturn() -> Result<()> {
    for path in [
        CompactionPath::Local,
        CompactionPath::RemoteDropped,
        CompactionPath::RemoteRetained,
        CompactionPath::RemoteV2,
    ] {
        characterize_automatic_compaction(AutomaticCompactionCase {
            phase: AutomaticCompactionPhase::PreTurn,
            path,
        })
        .await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn memory_effectiveness_host_compaction_auto_midturn() -> Result<()> {
    for path in [
        CompactionPath::Local,
        CompactionPath::RemoteDropped,
        CompactionPath::RemoteRetained,
        CompactionPath::RemoteV2,
    ] {
        characterize_automatic_compaction(AutomaticCompactionCase {
            phase: AutomaticCompactionPhase::MidTurn,
            path,
        })
        .await?;
    }
    Ok(())
}

async fn characterize_automatic_compaction(case: AutomaticCompactionCase) -> Result<()> {
    let AutomaticCompactionCase { phase, path } = case;
    let builder = test_ontocode()
        .with_config(move |config| {
            let _ = config.features.disable(Feature::LineageRouting);
            let _ = config.features.disable(Feature::LineageShadow);
            config.model_auto_compact_token_limit = Some(50_000);
            if matches!(path, CompactionPath::RemoteV2) {
                let _ = config.features.enable(Feature::RemoteCompactionV2);
            }
            if matches!(path, CompactionPath::Local) {
                config.model_provider.name = "OpenAI (test)".to_string();
            }
        })
        .with_workspace_setup(|cwd, _fs| async move {
            let root = cwd.as_path().canonicalize()?;
            std::fs::create_dir(root.join(".git"))?;
            let package = write_km(&root, KM_ID, EXAMPLE)?;
            run_km_import_for_project(&package, &root.join(".ontocode/ontograph/km/_store"), &root)
                .expect("import isolated automatic-compaction fixture");
            Ok(())
        })
        .with_extension_registry_factory(|_runtime, _thread_manager| {
            let mut registry = ExtensionRegistryBuilder::new();
            install_agent_knowledge_surface(&mut registry);
            Arc::new(registry.build())
        });
    let harness = TestOntocodeHarness::with_builder(builder).await?;
    let compact_trigger = match phase {
        AutomaticCompactionPhase::PreTurn => responses::sse(vec![
            responses::ev_assistant_message("auto-first-message", "FIRST_REPLY"),
            responses::ev_completed_with_tokens(
                "auto-first-response",
                /*total_tokens*/ 60_000,
            ),
        ]),
        AutomaticCompactionPhase::MidTurn => responses::sse(vec![
            responses::ev_function_call("auto-mid-call", "test_tool", "{}"),
            responses::ev_completed_with_tokens("auto-mid-response", /*total_tokens*/ 60_000),
        ]),
    };
    let local = matches!(path, CompactionPath::Local);
    let response_count = match (phase, local || matches!(path, CompactionPath::RemoteV2)) {
        (AutomaticCompactionPhase::PreTurn, true) => 4,
        (AutomaticCompactionPhase::PreTurn, false) => 3,
        (AutomaticCompactionPhase::MidTurn, true) => 5,
        (AutomaticCompactionPhase::MidTurn, false) => 4,
    };
    let mock = responses::mount_sse_sequence(
        harness.server(),
        (0..response_count)
            .map(|index| {
                if index == 0 {
                    responses::sse(vec![
                        responses::ev_assistant_message("auto-first-admission", "FIRST_REPLY"),
                        responses::ev_completed("auto-first-response"),
                    ])
                } else if index == 1 {
                    compact_trigger.clone()
                } else if index == 2 && matches!(path, CompactionPath::RemoteV2) {
                    responses::sse(vec![
                        json!({"type": "response.output_item.done", "item": {
                            "type": "compaction", "encrypted_content": "V2_SUMMARY"
                        }}),
                        responses::ev_completed("auto-v2-compact"),
                    ])
                } else {
                    responses::sse(vec![
                        responses::ev_assistant_message("auto-after-message", "AFTER_COMPACTION"),
                        responses::ev_completed("auto-after-response"),
                    ])
                }
            })
            .collect(),
    )
    .await;
    let compact_mock = if local || matches!(path, CompactionPath::RemoteV2) {
        None
    } else if matches!(path, CompactionPath::RemoteRetained) {
        Some(
            responses::mount_compact_user_history_with_summary_once(
                harness.server(),
                "AUTOMATIC_COMPACTION_SUMMARY",
            )
            .await,
        )
    } else {
        Some(
            responses::mount_compact_json_once(
                harness.server(),
                json!({
                    "output": [{
                        "type": "compaction",
                        "encrypted_content": "AUTOMATIC_COMPACTION_SUMMARY"
                    }]
                }),
            )
            .await,
        )
    };

    harness.submit(QUESTION).await?;
    let before = mock.requests();
    assert_eq!(before.len(), 1);
    let fragment = before[0]
        .input()
        .into_iter()
        .find(|item| item.to_string().contains(KM_CONTEXT_START_MARKER))
        .expect("real KM must be admitted before automatic compaction");
    assert!(fragment.to_string().contains(KM_ID));
    assert_eq!(km_items(&before[0]), vec![fragment.clone()]);

    match phase {
        AutomaticCompactionPhase::PreTurn => {
            harness.submit(QUESTION).await?;
            harness.submit(QUESTION).await?;
        }
        AutomaticCompactionPhase::MidTurn => {
            // No new KM output: restore the earlier admission without replaying retrieval.
            harness
                .submit("Sourdough baking temperature kneading yeast flour water")
                .await?;
        }
    }
    if matches!(phase, AutomaticCompactionPhase::MidTurn) {
        harness.submit(QUESTION).await?;
    }

    let requests = mock.requests();
    assert_eq!(requests.len(), response_count);
    if let Some(compact_mock) = compact_mock {
        let compact_request = compact_mock.single_request();
        assert_eq!(compact_request.path(), "/v1/responses/compact");
        assert_eq!(km_items(&compact_request), vec![fragment.clone()]);
    } else {
        assert!(km_items(&requests[1]).iter().any(|item| item == &fragment));
    }
    for request in requests.iter().skip(1) {
        assert_eq!(km_items(request), vec![fragment.clone()]);
    }
    Ok(())
}

fn km_items(request: &responses::ResponsesRequest) -> Vec<Value> {
    request
        .input()
        .into_iter()
        .filter(|item| item.to_string().contains(KM_CONTEXT_START_MARKER))
        .collect()
}
