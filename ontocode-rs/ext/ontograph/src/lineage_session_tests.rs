use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use core_test_support::responses;
use core_test_support::test_ontocode::TestOntocodeHarness;
use core_test_support::test_ontocode::test_ontocode;
use ontocode_extension_api::ExtensionData;
use ontocode_extension_api::ExtensionRegistryBuilder;
use ontocode_extension_api::TurnItemContributor;
use ontocode_features::Feature;
use ontocode_protocol::items::TurnItem;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use sha2::Digest;
use sha2::Sha256;

use crate::extension::LineageAdmissionResult;
use crate::extension::LineageTurnDecision;
use crate::install_agent_knowledge_surface;
use crate::km_context::KM_CONTEXT_END_MARKER;
use crate::km_context::KM_CONTEXT_START_MARKER;
use crate::km_import::run_km_import_for_project;
use crate::lineage_admission::AdmissionRejection;
use crate::lineage_admission::LINEAGE_CONTEXT_END_MARKER;
use crate::lineage_admission::LINEAGE_CONTEXT_START_MARKER;
use crate::lineage_collector::Admission;
use crate::lineage_collector::EvidenceClass;
use crate::lineage_collector::FallbackCategory;
use crate::lineage_collector::MissingCategory;
use crate::lineage_collector::Outcome;
use crate::lineage_collector::RunManifest;
use crate::lineage_collector::RunRecord;
use crate::lineage_collector::Terminal;
use crate::lineage_collector::Variant;
use crate::lineage_routing::FallbackReason;
use crate::lineage_routing::RoutingDecision;

const QUESTION: &str =
    "Trace the owner and then validate the contract using graph source evidence?";
const EXAMPLE: &str = "The owner validates the contract from the graph source.";
const UNSELECTED: &str = "Zebra sleeps quietly.";
const UNRELATED_ANSWER: &str = "Penguins swim beneath ice.";
const KM_ID: &str = "codebase-graph/session-lineage";
const HEALTHY_KM_ID: &str = "codebase-graph/session-baseline";

#[path = "lineage_compaction_tests.rs"]
mod compaction_tests;

#[derive(Clone, Copy)]
enum RoutingFeature {
    Enabled,
    Disabled,
    Default,
    ShadowOnly,
}

#[derive(Clone, Copy)]
enum SourceState {
    Fresh,
    Tampered,
    OverCap,
}

#[derive(Debug, PartialEq, Eq)]
struct CapturedDecision {
    decision: RoutingDecision,
    admission: Option<Result<(), AdmissionRejection>>,
}

struct DecisionObserver(Arc<Mutex<Vec<CapturedDecision>>>);

impl TurnItemContributor for DecisionObserver {
    // Implement the existing object-safe boxed-future contract without adding
    // another async-trait macro use.
    fn contribute<'a, 'b, 'c, 'd, 'fut>(
        &'a self,
        _thread_store: &'b ExtensionData,
        turn_store: &'c ExtensionData,
        item: &'d mut TurnItem,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'fut>>
    where
        'a: 'fut,
        'b: 'fut,
        'c: 'fut,
        'd: 'fut,
        Self: 'fut,
    {
        Box::pin(async move {
            if matches!(item, TurnItem::AgentMessage(_)) {
                let recorded = turn_store
                    .get::<LineageTurnDecision>()
                    .expect("registered contributor must record a decision");
                self.0.lock().unwrap().push(CapturedDecision {
                    decision: recorded.decision.clone(),
                    admission: turn_store
                        .get::<LineageAdmissionResult>()
                        .map(|admission| admission.0),
                });
            }
            Ok(())
        })
    }
}

struct SessionEvidence {
    requests: Vec<Value>,
    decisions: Vec<CapturedDecision>,
}

async fn run_session(
    feature: RoutingFeature,
    source_state: SourceState,
    answer: &str,
) -> Result<SessionEvidence> {
    let decisions = Arc::new(Mutex::new(Vec::new()));
    let mut builder = test_ontocode();
    match feature {
        RoutingFeature::Enabled => {
            builder = builder.with_config(|config| {
                let _ = config.features.enable(Feature::LineageRouting);
            });
        }
        RoutingFeature::Disabled => {
            builder = builder.with_config(|config| {
                let _ = config.features.disable(Feature::LineageRouting);
            });
        }
        RoutingFeature::ShadowOnly => {
            builder = builder.with_config(|config| {
                let _ = config.features.disable(Feature::LineageRouting);
                let _ = config.features.enable(Feature::LineageShadow);
            });
        }
        RoutingFeature::Default => {}
    }
    let builder = builder
        .with_workspace_setup(move |cwd, _fs| async move {
            let root = cwd.as_path().canonicalize()?;
            std::fs::create_dir(root.join(".git"))?;
            let store_root = root.join(".ontocode/ontograph/km/_store");
            let example = match source_state {
                SourceState::Fresh | SourceState::Tampered => {
                    format!("{EXAMPLE}\n\n\n\n{UNSELECTED}\n")
                }
                SourceState::OverCap => EXAMPLE.repeat(24),
            };
            let package = write_km(&root, KM_ID, &example)?;
            let imported = run_km_import_for_project(&package, &store_root, &root)
                .expect("import session lineage KM");
            if matches!(source_state, SourceState::Tampered) {
                std::fs::write(
                    Path::new(&imported.cache_path).join("examples/README.md"),
                    "The owner validates a changed contract from the graph source.",
                )?;
                // Topic scoring still matches this healthy KM, but its example
                // contains none of the question's terms and cannot admit lineage.
                let healthy = write_km(&root, HEALTHY_KM_ID, UNSELECTED)?;
                run_km_import_for_project(&healthy, &store_root, &root)
                    .expect("import healthy automatic-KM fallback");
            }
            Ok(())
        })
        .with_extension_registry_factory({
            let decisions = Arc::clone(&decisions);
            move |_runtime, _thread_manager| {
                let mut registry = ExtensionRegistryBuilder::new();
                install_agent_knowledge_surface(&mut registry);
                // Ordered after real Stage B; only observe its final state.
                registry.turn_item_contributor(Arc::new(DecisionObserver(decisions)));
                Arc::new(registry.build())
            }
        });
    let harness = TestOntocodeHarness::with_builder(builder).await?;
    let response = responses::mount_sse_once(
        harness.server(),
        responses::sse(vec![
            responses::ev_response_created("resp-lineage"),
            responses::ev_assistant_message("msg-lineage", answer),
            responses::ev_completed("resp-lineage"),
        ]),
    )
    .await;
    harness.submit(QUESTION).await?;
    response.single_request();
    let requests = harness.request_bodies().await;
    let decisions = std::mem::take(&mut *decisions.lock().unwrap());
    Ok(SessionEvidence {
        requests,
        decisions,
    })
}

fn write_km(root: &Path, id: &str, example: &str) -> Result<PathBuf> {
    // Reuse the minimal layout from extension_tests::write_auto_km_for_test,
    // which is private to that sibling module.
    let package = root.join(id.replace('/', "-"));
    std::fs::create_dir_all(package.join("examples"))?;
    std::fs::create_dir_all(package.join("tests"))?;
    std::fs::write(root.join("doc"), "doc")?;
    std::fs::write(
        package.join("km.toml"),
        format!(
            "id = \"{id}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Owner contract graph source evidence\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"human\"\ngenerated_at = \"2026-10-10\"\nsource_commit = \"HEAD\"\n"
        ),
    )?;
    std::fs::write(
        package.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"The owner validates the contract from the graph source.\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )?;
    std::fs::write(package.join("examples/README.md"), example)?;
    std::fs::write(package.join("tests/README.md"), "test")?;
    Ok(package)
}

fn context_items<'a>(request: &'a Value, marker: &str) -> Vec<&'a Value> {
    request["input"]
        .as_array()
        .expect("serialized provider input array")
        .iter()
        .filter(|item| item.to_string().contains(marker))
        .collect()
}

fn assert_one_lineage_block(evidence: &SessionEvidence) {
    assert_eq!(evidence.requests.len(), 1);
    let request = &evidence.requests[0];
    let items = context_items(request, LINEAGE_CONTEXT_START_MARKER);
    assert_eq!(items.len(), 1);
    assert_eq!(context_items(request, LINEAGE_CONTEXT_END_MARKER), items);
    let text = items[0]["content"][0]["text"]
        .as_str()
        .expect("lineage input text");
    assert_eq!(
        items[0],
        &json!({
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": text}],
        })
    );
    assert_eq!(text.matches(LINEAGE_CONTEXT_START_MARKER).count(), 1);
    assert_eq!(text.matches(LINEAGE_CONTEXT_END_MARKER).count(), 1);
    assert_eq!(text.matches(EXAMPLE).count(), 1);
    assert!(text.contains("Untrusted retrieved context."));
    assert!(!text.contains(UNSELECTED));
    assert!(context_items(request, KM_CONTEXT_START_MARKER).is_empty());
    assert!(context_items(request, KM_CONTEXT_END_MARKER).is_empty());
}

fn assert_automatic_km_only(evidence: &SessionEvidence) {
    assert_eq!(evidence.requests.len(), 1);
    for request in &evidence.requests {
        assert!(context_items(request, LINEAGE_CONTEXT_START_MARKER).is_empty());
        assert!(context_items(request, LINEAGE_CONTEXT_END_MARKER).is_empty());
        let items = context_items(request, KM_CONTEXT_START_MARKER);
        assert_eq!(items.len(), 1);
        assert_eq!(context_items(request, KM_CONTEXT_END_MARKER), items);
        let text = items[0]["content"][0]["text"]
            .as_str()
            .expect("automatic KM input text");
        assert_eq!(
            items[0],
            &json!({
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": text}],
            })
        );
        assert_eq!(text.matches(KM_CONTEXT_START_MARKER).count(), 1);
        assert_eq!(text.matches(KM_CONTEXT_END_MARKER).count(), 1);
        assert!(text.contains(EXAMPLE));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_enabled_admits_exactly_one_user_block() -> Result<()> {
    let evidence = run_session(RoutingFeature::Enabled, SourceState::Fresh, EXAMPLE).await?;
    assert_one_lineage_block(&evidence);
    assert_eq!(evidence.decisions.len(), 1);
    assert_eq!(evidence.decisions[0].admission, Some(Ok(())));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_explicitly_disabled_preserves_automatic_km() -> Result<()> {
    let evidence = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    assert_automatic_km_only(&evidence);
    assert_eq!(
        evidence.decisions,
        vec![CapturedDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::Disabled,
            },
            admission: None,
        }]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_shadow_only_sends_the_baseline_request() -> Result<()> {
    let shadow = run_session(RoutingFeature::ShadowOnly, SourceState::Fresh, EXAMPLE).await?;
    let disabled = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    // No lineage block reaches the model, and automatic KM is unchanged.
    assert_automatic_km_only(&shadow);
    assert_eq!(
        context_items(&shadow.requests[0], KM_CONTEXT_START_MARKER),
        context_items(&disabled.requests[0], KM_CONTEXT_START_MARKER)
    );
    // Stage A still ran and admitted, so the shadow arm measured something.
    assert_eq!(shadow.decisions.len(), 1);
    assert!(matches!(
        shadow.decisions[0].decision,
        RoutingDecision::Selected { .. }
    ));
    assert_eq!(shadow.decisions[0].admission, Some(Ok(())));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_default_matches_explicitly_disabled() -> Result<()> {
    let default = run_session(RoutingFeature::Default, SourceState::Fresh, EXAMPLE).await?;
    let disabled = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    assert_automatic_km_only(&default);
    assert_automatic_km_only(&disabled);
    // Only compare contextual input items; thread/environment metadata differs.
    assert_eq!(
        context_items(&default.requests[0], KM_CONTEXT_START_MARKER),
        context_items(&disabled.requests[0], KM_CONTEXT_START_MARKER)
    );
    assert_eq!(default.decisions, disabled.decisions);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_tampered_source_preserves_healthy_automatic_km() -> Result<()> {
    let evidence = run_session(RoutingFeature::Enabled, SourceState::Tampered, EXAMPLE).await?;
    assert_automatic_km_only(&evidence);
    let items = context_items(&evidence.requests[0], KM_CONTEXT_START_MARKER);
    assert!(items[0].to_string().contains(HEALTHY_KM_ID));
    assert!(!items[0].to_string().contains(KM_ID));
    assert_eq!(evidence.decisions.len(), 1);
    assert!(matches!(
        evidence.decisions[0].decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(_),
        }
    ));
    assert_eq!(evidence.decisions[0].admission, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_over_cap_preserves_automatic_km_and_cannot_ground() -> Result<()> {
    let evidence = run_session(RoutingFeature::Enabled, SourceState::OverCap, EXAMPLE).await?;
    assert_automatic_km_only(&evidence);
    assert_eq!(
        evidence.decisions,
        vec![CapturedDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::NoUsableEvidence,
            },
            admission: Some(Err(AdmissionRejection::ByteCapExceeded)),
        }]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_supporting_answer_records_partial_coverage() -> Result<()> {
    let evidence = run_session(RoutingFeature::Enabled, SourceState::Fresh, EXAMPLE).await?;
    assert_one_lineage_block(&evidence);
    // Stage B grounds the example, but induced connective/request demands have
    // no matching span. Successful admission therefore remains partial coverage.
    assert_eq!(
        evidence.decisions,
        vec![CapturedDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::PartialCoverage {
                    unresolved_demands: vec![
                        "d-0".to_string(),
                        "d-2".to_string(),
                        "d-7".to_string(),
                        "d-8".to_string(),
                        "d-9".to_string(),
                    ],
                },
            },
            admission: Some(Ok(())),
        }]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_unsupporting_answer_records_no_usable_evidence() -> Result<()> {
    let evidence = run_session(
        RoutingFeature::Enabled,
        SourceState::Fresh,
        UNRELATED_ANSWER,
    )
    .await?;
    assert_one_lineage_block(&evidence);
    // Stage A admitted the source, but Stage B finds no contained answer claim.
    assert_eq!(
        evidence.decisions,
        vec![CapturedDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::NoUsableEvidence,
            },
            admission: Some(Ok(())),
        }]
    );
    Ok(())
}

fn admission_rejection_category(rejection: AdmissionRejection) -> FallbackCategory {
    match rejection {
        AdmissionRejection::ScopeInvalid => {
            assert_eq!(rejection.category(), "scope_invalid");
            FallbackCategory::ScopeInvalid
        }
        AdmissionRejection::SourceStale => {
            assert_eq!(rejection.category(), "source_stale");
            FallbackCategory::SourceStale
        }
        AdmissionRejection::SourceUnavailable => {
            assert_eq!(rejection.category(), "source_unavailable");
            FallbackCategory::SourceUnavailable
        }
        AdmissionRejection::SchemaInvalid => {
            assert_eq!(rejection.category(), "schema_invalid");
            FallbackCategory::SchemaInvalid
        }
        AdmissionRejection::ByteCapExceeded => {
            assert_eq!(rejection.category(), "byte_cap_exceeded");
            FallbackCategory::ByteCapExceeded
        }
        AdmissionRejection::RenderInvalid => {
            assert_eq!(rejection.category(), "render_invalid");
            FallbackCategory::RenderInvalid
        }
    }
}

fn session_record(evidence: &SessionEvidence, mut record_template: RunRecord) -> Result<RunRecord> {
    anyhow::ensure!(
        evidence.requests.len() == 1,
        "expected exactly one captured request"
    );
    let request_bytes = serde_json::to_vec(&evidence.requests[0]["input"])?;
    record_template.request_hash = Some(format!("{:x}", Sha256::digest(request_bytes)));

    let answer_text = match &record_template.terminal {
        Terminal::Completed { answer_ref } => answer_ref,
        Terminal::Missing(_) => {
            anyhow::bail!("record template must contain the assistant answer text")
        }
    };
    record_template.terminal = Terminal::Completed {
        answer_ref: format!("{:x}", Sha256::digest(answer_text.as_bytes())),
    };

    let captured = match evidence.decisions.as_slice() {
        [captured] => Some(captured),
        _ => None,
    };
    record_template.admission = match captured {
        Some(CapturedDecision {
            admission: Some(Ok(())),
            ..
        }) => Some((
            Outcome::Lineage,
            Admission::Accepted,
            FallbackCategory::None,
        )),
        Some(CapturedDecision {
            admission: Some(Err(rejection)),
            ..
        }) => Some((
            Outcome::Baseline,
            Admission::Rejected,
            admission_rejection_category(*rejection),
        )),
        Some(CapturedDecision {
            admission: None,
            decision:
                RoutingDecision::Baseline {
                    reason: FallbackReason::Disabled,
                },
        }) => Some((
            Outcome::Baseline,
            Admission::Disabled,
            FallbackCategory::Disabled,
        )),
        Some(CapturedDecision {
            admission: None,
            decision: RoutingDecision::Baseline { reason },
        }) => {
            let category = match reason {
                FallbackReason::Disabled => Some(FallbackCategory::Disabled),
                FallbackReason::NotEligible { .. } => Some(FallbackCategory::NotEligible),
                FallbackReason::InductionFailed(_) => Some(FallbackCategory::InductionFailed),
                FallbackReason::RetrievalFailed(_) => Some(FallbackCategory::RetrievalFailed),
                FallbackReason::SelectionFailed(_) => Some(FallbackCategory::SelectionFailed),
                FallbackReason::GroundingFailed(_) => None,
                FallbackReason::SchemaInvalid(_) => Some(FallbackCategory::SchemaInvalid),
                FallbackReason::Timeout { .. } => Some(FallbackCategory::Timeout),
                FallbackReason::BudgetExhausted { .. } => Some(FallbackCategory::BudgetExhausted),
                FallbackReason::TokenCapExceeded { .. } => Some(FallbackCategory::TokenCapExceeded),
                FallbackReason::PartialCoverage { .. } => Some(FallbackCategory::PartialCoverage),
                FallbackReason::NoUsableEvidence => Some(FallbackCategory::NoUsableEvidence),
            };
            category.map(|category| (Outcome::Baseline, Admission::Rejected, category))
        }
        Some(CapturedDecision {
            admission: None,
            decision: RoutingDecision::Selected { .. } | RoutingDecision::Lineage { .. },
        })
        | None => None,
    };
    Ok(record_template)
}

fn session_record_template(
    evidence: &SessionEvidence,
    variant: Variant,
    run_id: &str,
    case_id: &str,
    corpus_digest: &str,
    corpus_revision: &str,
    answer_text: &str,
) -> RunRecord {
    RunRecord {
        run_id: run_id.to_string(),
        case_id: case_id.to_string(),
        variant,
        evidence_class: EvidenceClass::Mock,
        corpus_digest: corpus_digest.to_string(),
        prompt_revision: "session-prompt-v1".to_string(),
        config_revision: "session-config-v1".to_string(),
        schema_revision: "session-schema-v1".to_string(),
        model_id: Some(
            evidence.requests[0]["model"]
                .as_str()
                .filter(|model| !model.is_empty())
                .unwrap_or("mock-model")
                .to_string(),
        ),
        request_hash: None,
        admission: None,
        terminal: Terminal::Completed {
            answer_ref: answer_text.to_string(),
        },
        elapsed_ms: None,
        manifest: RunManifest {
            corpus_revision: corpus_revision.to_string(),
            order_seed: 0,
            provider_id: None,
            started_at: None,
        },
        demand_coverage: None,
        provenance_validity: None,
        answer_quality: None,
    }
}

fn rollout_identity() -> Result<(String, String, String)> {
    let (digest, corpus_id, mut case_ids) = rollout_identities()?;
    Ok((digest, corpus_id, case_ids.remove(0)))
}

fn rollout_identities() -> Result<(String, String, Vec<String>)> {
    let fixture = crate::lineage_replay::load_rollout_corpus()?;
    let digest = crate::lineage_replay::corpus_digest(
        crate::lineage_replay::ROLLOUT_CORPUS_JSON.as_bytes(),
        &fixture,
        &fixture.settings(),
    )?;
    let case_ids = fixture
        .cases
        .iter()
        .map(|case| case.case_id.clone())
        .collect();
    Ok((digest, fixture.corpus_id, case_ids))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_collector_pairs_admission_independently_of_grounding() -> Result<()> {
    let baseline = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    let lineage = run_session(RoutingFeature::Enabled, SourceState::Fresh, EXAMPLE).await?;
    let (digest, corpus_revision, case_id) = rollout_identity()?;
    let baseline = session_record(
        &baseline,
        session_record_template(
            &baseline,
            Variant::Baseline,
            "session-baseline",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    let lineage = session_record(
        &lineage,
        session_record_template(
            &lineage,
            Variant::Lineage,
            "session-lineage",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    let report = crate::lineage_collector::collect(&[baseline, lineage])
        .expect("valid paired session records");
    let mock = &report.tables[&EvidenceClass::Mock];
    let baseline_table = &mock[&Variant::Baseline];
    let lineage_table = &mock[&Variant::Lineage];
    assert_eq!(
        lineage_table.fallback_categories,
        [(FallbackCategory::None, 1)].into()
    );
    assert_eq!(
        baseline_table.fallback_categories,
        [(FallbackCategory::Disabled, 1)].into()
    );
    assert_eq!(
        lineage_table.fallback_rate.counts,
        crate::lineage_collector::Counts {
            numerator: 0,
            denominator: 1,
        }
    );
    assert_eq!(
        baseline_table.fallback_rate.counts,
        crate::lineage_collector::Counts {
            numerator: 1,
            denominator: 1,
        }
    );
    assert!(baseline_table.missing.is_empty());
    assert!(lineage_table.missing.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_collector_counts_tampered_source_as_resolved_fallback() -> Result<()> {
    let baseline = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    let tampered = run_session(RoutingFeature::Enabled, SourceState::Tampered, EXAMPLE).await?;
    assert!(matches!(
        tampered.decisions[0].decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(_),
        }
    ));
    assert_eq!(tampered.decisions[0].admission, None);
    let (digest, corpus_revision, case_id) = rollout_identity()?;
    let baseline = session_record(
        &baseline,
        session_record_template(
            &baseline,
            Variant::Baseline,
            "tampered-baseline",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    let tampered = session_record(
        &tampered,
        session_record_template(
            &tampered,
            Variant::Lineage,
            "tampered-lineage",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    assert_eq!(
        tampered.admission,
        Some((
            Outcome::Baseline,
            Admission::Rejected,
            FallbackCategory::RetrievalFailed,
        ))
    );
    let report = crate::lineage_collector::collect(&[baseline, tampered])
        .expect("valid paired tampered-source records");
    let lineage_table = &report.tables[&EvidenceClass::Mock][&Variant::Lineage];
    assert_eq!(
        lineage_table.fallback_categories,
        [(FallbackCategory::RetrievalFailed, 1)].into()
    );
    assert_eq!(
        lineage_table.fallback_rate.counts,
        crate::lineage_collector::Counts {
            numerator: 1,
            denominator: 1,
        }
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lineage_session_collector_keeps_no_admission_terminal_only_in_missing() -> Result<()> {
    let baseline = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    let mut lineage = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    lineage.decisions.clear();
    let (digest, corpus_revision, case_id) = rollout_identity()?;
    let baseline = session_record(
        &baseline,
        session_record_template(
            &baseline,
            Variant::Baseline,
            "missing-baseline",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    let mut lineage = session_record(
        &lineage,
        session_record_template(
            &lineage,
            Variant::Lineage,
            "missing-lineage",
            &case_id,
            &digest,
            &corpus_revision,
            EXAMPLE,
        ),
    )?;
    lineage.terminal = Terminal::Missing(MissingCategory::NoAdmissionEvent);
    lineage.admission = None;
    let report = crate::lineage_collector::collect(&[baseline, lineage])
        .expect("valid paired missing-terminal records");
    let lineage_table = &report.tables[&EvidenceClass::Mock][&Variant::Lineage];
    assert_eq!(
        lineage_table.missing,
        [(MissingCategory::NoAdmissionEvent, 1)].into()
    );
    assert!(lineage_table.fallback_categories.is_empty());
    assert_eq!(
        lineage_table.fallback_rate.counts,
        crate::lineage_collector::Counts::default()
    );
    Ok(())
}

const LIVE_RUN_ENV: &str = "LRA_LIVE_RUN";
const LIVE_CAP_ENV: &str = "LRA_MAX_REQUESTS";
const LIVE_OUT_ENV: &str = "LRA_RECORDS_OUT";
const DEFAULT_LIVE_CAP: usize = 30;

/// Resolves the hard request cap; unparsable or zero values fall back to the default.
fn live_request_cap(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|cap| *cap > 0)
        .unwrap_or(DEFAULT_LIVE_CAP)
}

/// Serializes one record per line after proving it carries no prompt or secret text.
fn sanitized_jsonl(records: &[RunRecord], forbidden: &[&str]) -> Result<String> {
    let mut out = String::new();
    for record in records {
        let line = serde_json::to_string(record)?;
        for needle in forbidden.iter().filter(|needle| !needle.is_empty()) {
            anyhow::ensure!(!line.contains(needle), "record leaks forbidden text");
        }
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

#[test]
fn live_request_cap_defaults_and_rejects_zero_or_garbage() {
    assert_eq!(live_request_cap(None), DEFAULT_LIVE_CAP);
    assert_eq!(live_request_cap(Some("0")), DEFAULT_LIVE_CAP);
    assert_eq!(live_request_cap(Some("abc")), DEFAULT_LIVE_CAP);
    assert_eq!(live_request_cap(Some(" 7 ")), 7);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sanitized_jsonl_omits_prompt_and_secrets_and_round_trips() -> Result<()> {
    let secret = "sk-test-SECRET-123";
    let baseline = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
    let lineage = run_session(RoutingFeature::Enabled, SourceState::Fresh, EXAMPLE).await?;
    let (digest, corpus_revision, case_id) = rollout_identity()?;
    let mut records = Vec::new();
    for (evidence, variant, run_id) in [
        (&baseline, Variant::Baseline, "live-baseline"),
        (&lineage, Variant::Lineage, "live-lineage"),
    ] {
        records.push(session_record(
            evidence,
            session_record_template(
                evidence,
                variant,
                run_id,
                &case_id,
                &digest,
                &corpus_revision,
                EXAMPLE,
            ),
        )?);
    }
    let text = sanitized_jsonl(&records, &[QUESTION, EXAMPLE, secret, "Authorization"])?;
    assert_eq!(text.lines().count(), 2);
    let report = crate::lineage_collector::collect_jsonl(text.as_bytes())
        .expect("sanitized JSONL ingests and collects");
    assert!(report.tables.contains_key(&EvidenceClass::Mock));
    // A record that does carry forbidden text must be refused.
    let mut leaky = records[0].clone();
    leaky.run_id = format!("run-{secret}");
    assert!(sanitized_jsonl(&[leaky], &[secret]).is_err());
    Ok(())
}

/// Operator runner. Ignored by default; set LRA_LIVE_RUN=1 to execute.
/// This build drives the local mock Responses server only (evidence class
/// Mock); it stops at LRA_MAX_REQUESTS and writes JSONL to LRA_RECORDS_OUT.
#[ignore = "operator run: set LRA_LIVE_RUN=1"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lra_live_runner_writes_sanitized_records() -> Result<()> {
    if std::env::var(LIVE_RUN_ENV).as_deref() != Ok("1") {
        eprintln!("skipped: {LIVE_RUN_ENV}=1 not set");
        return Ok(());
    }
    let cap = live_request_cap(std::env::var(LIVE_CAP_ENV).ok().as_deref());
    let (digest, corpus_revision, case_ids) = rollout_identities()?;
    let mut records = Vec::new();
    let mut requests = 0usize;
    // One baseline/lineage pair per distinct case; the collector rejects repeats.
    for case_id in &case_ids {
        // Each pair spends two requests; never exceed the cap.
        if requests + 2 > cap {
            break;
        }
        let baseline = run_session(RoutingFeature::Disabled, SourceState::Fresh, EXAMPLE).await?;
        let lineage = run_session(RoutingFeature::Enabled, SourceState::Fresh, EXAMPLE).await?;
        requests += baseline.requests.len() + lineage.requests.len();
        for (evidence, variant, label) in [
            (&baseline, Variant::Baseline, "baseline"),
            (&lineage, Variant::Lineage, "lineage"),
        ] {
            records.push(session_record(
                evidence,
                session_record_template(
                    evidence,
                    variant,
                    &format!("live-{label}-{case_id}"),
                    case_id,
                    &digest,
                    &corpus_revision,
                    EXAMPLE,
                ),
            )?);
        }
    }
    anyhow::ensure!(requests <= cap, "request cap exceeded");
    let text = sanitized_jsonl(&records, &[QUESTION, EXAMPLE, "Authorization"])?;
    let out =
        std::env::var(LIVE_OUT_ENV).unwrap_or_else(|_| "/opt/d1/tmp/lra_records.jsonl".to_string());
    std::fs::write(&out, &text)?;
    let report = crate::lineage_collector::collect_jsonl(text.as_bytes())
        .map_err(|error| anyhow::anyhow!("collect failed: {error:?}"))?;
    eprintln!("{}", crate::lineage_collector::render_report(&report)?);
    Ok(())
}
