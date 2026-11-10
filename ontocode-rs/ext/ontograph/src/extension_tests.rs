use std::cell::Cell;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use crate::evidence_planner::SourceLocation;
use crate::km_context::MAX_FRAGMENT_TOKENS;
use crate::lineage_grounding::ContainmentStatus;
use crate::lineage_grounding::DemandSupport;
use crate::lineage_grounding::GroundedClaim;
use crate::lineage_grounding::GroundingReport;
use crate::lineage_grounding::SourceGeneration;
use crate::lineage_grounding::SourceGenerationFile;
use crate::lineage_records::EvidenceDemand;
use crate::lineage_records::FirstExposure;
use crate::lineage_records::LineageLedger;
use crate::lineage_records::PassageLineage;
use crate::lineage_routing::FallbackReason;
use crate::lineage_routing::LineageBudget;
use crate::lineage_routing::LineageCost;
use crate::lineage_selection::SelectedPassage;
use crate::lineage_selection::SelectionReport;
use crate::lineage_selection::SelectionStop;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use tempfile::tempdir;

use super::*;

// Serialize every test that can reach the process-global blocking-worker hook.
// Semaphore permits can span awaits across independent test runtimes.
static STAGE_A_TEST_SERIAL: tokio::sync::Semaphore =
    tokio::sync::Semaphore::const_new(/*permits*/ 1);

struct StageATestHook {
    entered: Option<std::sync::mpsc::Receiver<()>>,
    release: Option<std::sync::mpsc::SyncSender<()>>,
}

impl StageATestHook {
    fn blocking() -> Self {
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(/*bound*/ 0);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(/*bound*/ 0);
        let release_rx = std::sync::Mutex::new(release_rx);
        let hook = Self {
            entered: Some(entered_rx),
            release: Some(release_tx),
        };
        *STAGE_A_TEST_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(move || {
            let _ = entered_tx.send(());
            let _ = release_rx.lock().expect("release receiver").recv();
        }));
        hook
    }

    async fn entered(&mut self) {
        let receiver = self.entered.take().expect("one hook entry");
        tokio::task::spawn_blocking(move || {
            receiver
                .recv_timeout(Duration::from_secs(/*secs*/ 5))
                .expect("Stage A entered its blocking worker");
        })
        .await
        .expect("hook entry waiter");
    }

    fn release(&mut self) {
        // Disconnecting the rendezvous wakes the worker even during unwinding.
        drop(self.release.take());
    }
}

impl Drop for StageATestHook {
    fn drop(&mut self) {
        self.release();
        drop(self.entered.take());
        *STAGE_A_TEST_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}

fn stage_a_test_project() -> (tempfile::TempDir, PathBuf, Arc<ExtensionData>) {
    let dir = tempdir().expect("Stage A project");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/stage-a",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "The owner validates the contract from the graph source.",
    );
    KmStore::for_project_path(&root)
        .expect("project store")
        .import_for_project(&source, &root.display().to_string())
        .expect("import Stage A KM");
    let thread = Arc::new(ExtensionData::new("thread-stage-a"));
    thread.insert(LineageRoutingEligibility::from_enabled(
        /*enabled*/ true,
    ));
    (dir, root, thread)
}

fn stage_a_test_input(root: &Path) -> TurnInputContext {
    // Below the automatic-KM word floor, but eligible for multi-hop lineage.
    let mut input = auto_km_turn_input("Trace owner and then validate contract?");
    input.environments = vec![ontocode_extension_api::TurnInputEnvironment {
        environment_id: "local".to_string(),
        cwd: root.to_path_buf(),
        is_primary: true,
    }];
    input
}

fn stage_a_auto_km_input(root: &Path) -> TurnInputContext {
    let mut input = stage_a_test_input(root);
    input.user_input = auto_km_turn_input(
        "Trace the owner and then validate the contract using graph source evidence?",
    )
    .user_input;
    assert!(user_text_words(&input.user_input).len() >= MIN_AUTO_KM_USER_WORDS);
    let store = KmStore::for_project_path(root).expect("project store");
    assert_eq!(
        matching_km_context_fragments(&input, &store, &root.display().to_string()).len(),
        1,
        "the question must match a healthy automatic-KM fragment"
    );
    input
}

fn assert_stage_a_auto_km_unmarked(thread: &ExtensionData, root: &Path) {
    assert!(
        !thread
            .get_or_init(OntographThreadState::default)
            .auto_km_context_was_injected(
                &root.display().to_string(),
                "codebase-graph/stage-a",
                "0.1.0",
            )
    );
}

fn spawn_stage_a_contribution(
    root: &Path,
    thread: &Arc<ExtensionData>,
    turn: &Arc<ExtensionData>,
) -> tokio::task::JoinHandle<Vec<Box<dyn ContextualUserFragment + Send>>> {
    let input = stage_a_test_input(root);
    let thread = Arc::clone(thread);
    let turn = Arc::clone(turn);
    tokio::spawn(async move {
        SurfaceScopedOntographExtension::default()
            .contribute(
                input,
                &ExtensionData::new("session-stage-a"),
                &thread,
                &turn,
            )
            .await
    })
}

async fn stage_a_worker_finished(thread: &ExtensionData) {
    let busy = thread.get::<StageABusy>().expect("Stage A busy guard");
    tokio::time::timeout(Duration::from_secs(/*secs*/ 5), async {
        while busy.0.load(std::sync::atomic::Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(/*millis*/ 1)).await;
        }
    })
    .await
    .expect("released blocking worker finished");
}

fn assert_stage_a_unpublished(turn: &ExtensionData) {
    assert!(turn.get::<LineageAdmissionResult>().is_none());
    assert!(turn.get::<LineageRetrievalScope>().is_none());
}

fn assert_stage_a_failure(turn: &ExtensionData, reason: &str) {
    assert_eq!(
        decision_in(turn),
        RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(reason.to_string()),
        }
    );
    assert_stage_a_unpublished(turn);
}

#[test]
fn lineage_breaker_trips_after_three_consecutive_failures_and_success_resets() {
    let breaker = LineageBreaker::default();
    assert!(!breaker.tripped());
    breaker.record_failure();
    breaker.record_failure();
    assert!(!breaker.tripped());
    breaker.record_success();
    breaker.record_failure();
    breaker.record_failure();
    assert!(!breaker.tripped());
    breaker.record_failure();
    assert!(breaker.tripped());
    breaker.record_success();
    assert!(!breaker.tripped());

    breaker
        .consecutive_failures
        .store(u32::MAX, Ordering::Relaxed);
    breaker.record_failure();
    assert_eq!(
        breaker.consecutive_failures.load(Ordering::Relaxed),
        u32::MAX
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_slow_worker_times_out_without_blocking_or_late_publication() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let turn = Arc::new(ExtensionData::new("turn-stage-a-timeout"));
    let input = stage_a_auto_km_input(&root);
    let (metrics, exporter) = lineage_test_metrics();
    let extension = SurfaceScopedOntographExtension {
        metrics_client: Some(metrics.clone()),
    };
    let budget = Duration::from_millis(/*millis*/ 300);
    let started_at = Instant::now();
    turn.insert(TurnDeadline::from_start(started_at, budget));
    let mut hook = StageATestHook::blocking();
    let (start_ticks, tick_start) = tokio::sync::oneshot::channel();
    let (send_ticks, receive_ticks) = tokio::sync::oneshot::channel();
    let task_thread = Arc::clone(&thread);
    let task_turn = Arc::clone(&turn);
    let job = tokio::spawn(async move {
        let session = ExtensionData::new("session-stage-a");
        // Both futures are polled in the same task: inline blocking in
        // contribute would also prevent this timer from making progress.
        let (fragments, ()) = tokio::join!(
            extension.contribute(input, &session, &task_thread, &task_turn),
            async {
                tick_start.await.expect("start timer after hook entry");
                let mut ticks = Vec::new();
                for _ in 0..3 {
                    tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
                    ticks.push(Instant::now());
                }
                send_ticks.send(ticks).expect("timer observer");
            }
        );
        fragments
    });
    hook.entered().await;
    assert!(turn.get::<LineageTurnDecision>().is_none());
    assert_stage_a_unpublished(&turn);

    start_ticks.send(()).expect("start runtime timer");
    let ticks = tokio::time::timeout(Duration::from_millis(/*millis*/ 150), receive_ticks)
        .await
        .expect("runtime timer advances while retrieval is blocked")
        .expect("timer ticks");
    assert_eq!(ticks.len(), 3);
    assert!(ticks.windows(/*size*/ 2).all(|pair| pair[0] < pair[1]));
    assert!(
        thread
            .get::<StageABusy>()
            .expect("busy guard")
            .0
            .load(std::sync::atomic::Ordering::Acquire)
    );

    let fragments = tokio::time::timeout(Duration::from_secs(/*secs*/ 1), job)
        .await
        .expect("contribute returns while the worker remains blocked")
        .expect("contribute task");
    // Timer scheduling may deliver the deadline just after its target instant.
    assert!(started_at.elapsed() < budget + Duration::from_millis(/*millis*/ 200));
    assert!(fragments.is_empty());
    let timed_out = decision_in(&turn);
    let RoutingDecision::Baseline {
        reason: FallbackReason::Timeout {
            elapsed_ms,
            limit_ms,
        },
    } = &timed_out
    else {
        panic!("expected Stage A timeout, got {timed_out:?}");
    };
    assert!(*limit_ms > 0 && *limit_ms <= 300);
    assert!(*elapsed_ms >= *limit_ms);
    assert_stage_a_unpublished(&turn);
    assert_stage_a_auto_km_unmarked(&thread, &root);

    hook.release();
    stage_a_worker_finished(&thread).await;
    drop(hook);
    assert_eq!(decision_in(&turn), timed_out);
    assert_stage_a_unpublished(&turn);
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert_stage_a_baseline_metric_once(&metrics, &exporter, &turn, "timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_busy_worker_rejects_overlap_and_allows_the_next_turn() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let first_turn = Arc::new(ExtensionData::new("turn-stage-a-first"));
    let mut hook = StageATestHook::blocking();
    let first = spawn_stage_a_contribution(&root, &thread, &first_turn);
    hook.entered().await;
    assert!(first_turn.get::<LineageTurnDecision>().is_none());
    assert_stage_a_unpublished(&first_turn);

    let second_turn = ExtensionData::new("turn-stage-a-busy");
    let (metrics, exporter) = lineage_test_metrics();
    let extension = SurfaceScopedOntographExtension {
        metrics_client: Some(metrics.clone()),
    };
    let fragments = tokio::time::timeout(
        Duration::from_secs(/*secs*/ 1),
        extension.contribute(
            stage_a_auto_km_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &second_turn,
        ),
    )
    .await
    .expect("busy turn returns before the first worker is released");
    assert!(fragments.is_empty());
    assert_stage_a_failure(&second_turn, "stage_a_busy");
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert!(second_turn.get::<LineageMetricAttempted>().is_some());

    hook.release();
    let first_fragments = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), first)
        .await
        .expect("released first contribution finishes")
        .expect("first contribution task");
    stage_a_worker_finished(&thread).await;
    drop(hook);
    assert_eq!(first_fragments.len(), 1);
    assert!(
        first_fragments[0]
            .render()
            .contains("<ontograph_lineage_context>")
    );
    assert_stage_a_failure(&second_turn, "stage_a_busy");
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert_stage_a_baseline_metric_once(&metrics, &exporter, &second_turn, "retrieval_failed");

    let third_turn = ExtensionData::new("turn-stage-a-third");
    let third_fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &third_turn,
        )
        .await;
    assert_eq!(third_fragments.len(), 1);
    assert!(matches!(
        decision_in(&third_turn),
        RoutingDecision::Selected { .. }
    ));
    assert_eq!(
        third_turn
            .get::<LineageAdmissionResult>()
            .expect("third admission")
            .0,
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_panicking_worker_records_fallback_and_releases_the_guard() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let turn = ExtensionData::new("turn-stage-a-panic");
    let (metrics, exporter) = lineage_test_metrics();
    let extension = SurfaceScopedOntographExtension {
        metrics_client: Some(metrics.clone()),
    };
    let hook = StageATestHook {
        entered: None,
        release: None,
    };
    *STAGE_A_TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Some(Box::new(|| panic!("injected Stage A panic")));
    let fragments = tokio::time::timeout(
        Duration::from_secs(/*secs*/ 1),
        extension.contribute(
            stage_a_auto_km_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &turn,
        ),
    )
    .await
    .expect("panicking worker returns promptly");
    assert!(fragments.is_empty());
    assert_stage_a_failure(&turn, "stage_a_panic");
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert!(turn.get::<LineageMetricAttempted>().is_some());
    stage_a_worker_finished(&thread).await;
    drop(hook);
    assert_stage_a_failure(&turn, "stage_a_panic");
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert_stage_a_baseline_metric_once(&metrics, &exporter, &turn, "retrieval_failed");

    let next_turn = ExtensionData::new("turn-stage-a-after-panic");
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &next_turn,
        )
        .await;
    assert_eq!(fragments.len(), 1);
    assert!(matches!(
        decision_in(&next_turn),
        RoutingDecision::Selected { .. }
    ));
    assert_eq!(
        next_turn
            .get::<LineageAdmissionResult>()
            .expect("next admission")
            .0,
        Ok(())
    );
    assert_stage_a_failure(&turn, "stage_a_panic");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_cancelled_contribution_records_fallback_without_late_publication() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let turn = Arc::new(ExtensionData::new("turn-stage-a-cancelled"));
    let mut hook = StageATestHook::blocking();
    let input = stage_a_test_input(&root);
    let task_thread = Arc::clone(&thread);
    let task_turn = Arc::clone(&turn);
    let (cancel, cancellation) = tokio::sync::oneshot::channel();
    let job = tokio::spawn(async move {
        let extension = SurfaceScopedOntographExtension::default();
        let session = ExtensionData::new("session-stage-a");
        tokio::select! {
            fragments = extension.contribute(input, &session, &task_thread, &task_turn) => {
                Some(fragments)
            }
            signal = cancellation => {
                signal.expect("cancel after hook entry");
                None
            }
        }
    });
    hook.entered().await;
    assert_stage_a_unpublished(&turn);
    cancel
        .send(())
        .expect("drop the blocked contribution future");
    let result = tokio::time::timeout(Duration::from_secs(/*secs*/ 1), job)
        .await
        .expect("cancellation completes while the worker is blocked")
        .expect("cancellation task");
    assert!(result.is_none(), "select must drop the contribution future");
    assert_stage_a_failure(&turn, "stage_a_cancelled");
    let cancelled = decision_in(&turn);
    assert!(
        thread
            .get::<StageABusy>()
            .expect("busy guard")
            .0
            .load(std::sync::atomic::Ordering::Acquire)
    );

    hook.release();
    stage_a_worker_finished(&thread).await;
    drop(hook);
    assert_eq!(decision_in(&turn), cancelled);
    assert_stage_a_unpublished(&turn);
}

#[tokio::test]
async fn stage_a_shadow_only_runs_admission_but_injects_nothing() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    thread.insert(LineageRoutingEligibility::from_flags(
        /*enabled*/ false, /*shadow*/ true,
    ));
    let turn = ExtensionData::new("turn-stage-a-shadow");
    let (metrics, exporter) = lineage_test_metrics();
    let extension = SurfaceScopedOntographExtension {
        metrics_client: Some(metrics.clone()),
    };
    let fragments = extension
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &turn,
        )
        .await;
    assert!(
        fragments
            .iter()
            .all(|fragment| !fragment.render().contains("<ontograph_lineage_context>"))
    );
    assert!(matches!(
        decision_in(&turn),
        RoutingDecision::Selected { .. }
    ));
    assert!(turn.get::<LineageAdmissionResult>().is_some());
    metrics.shutdown().expect("shutdown metrics");
    let names: Vec<String> = exporter
        .get_finished_metrics()
        .expect("finished metrics")
        .iter()
        .flat_map(opentelemetry_sdk::metrics::data::ResourceMetrics::scope_metrics)
        .flat_map(opentelemetry_sdk::metrics::data::ScopeMetrics::metrics)
        .map(|metric| metric.name().to_string())
        .collect();
    assert!(names.contains(&"lineage_shadow_attempt_total".to_string()));
    assert!(!names.contains(&"lineage_routing_attempt_total".to_string()));
}

#[tokio::test]
async fn stage_a_admitted_turn_publishes_one_fragment_and_one_metric() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    assert!(
        STAGE_A_TEST_HOOK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
    );
    let (_dir, root, thread) = stage_a_test_project();
    let turn = ExtensionData::new("turn-stage-a-admitted");
    let (metrics, exporter) = lineage_test_metrics();
    let extension = SurfaceScopedOntographExtension {
        metrics_client: Some(metrics.clone()),
    };
    let fragments = extension
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &turn,
        )
        .await;
    assert_eq!(fragments.len(), 1);
    assert!(
        fragments[0]
            .render()
            .contains("<ontograph_lineage_context>")
    );
    assert!(!fragments[0].render().contains("<ontograph_km_context>"));
    assert!(matches!(
        decision_in(&turn),
        RoutingDecision::Selected { .. }
    ));
    assert_eq!(
        turn.get::<LineageRetrievalScope>()
            .expect("published scope")
            .0,
        root
    );
    assert_eq!(
        turn.get::<LineageAdmissionResult>()
            .expect("published admission")
            .0,
        Ok(())
    );
    assert!(turn.get::<LineageMetricAttempted>().is_some());
    // The recorded once guard must also suppress a later explicit emission.
    emit_stage_a(Some(&metrics), &turn);
    metrics.shutdown().expect("flush Stage A metrics");
    assert_eq!(
        lineage_counter_points(&exporter),
        vec![(
            1,
            BTreeMap::from([
                ("outcome".to_string(), "lineage".to_string()),
                ("admission".to_string(), "accepted".to_string()),
                ("fallback_category".to_string(), "none".to_string()),
            ]),
        )]
    );
}

#[tokio::test]
async fn stage_a_tripped_breaker_records_baseline_without_starting_worker() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let breaker = thread.get_or_init(LineageBreaker::default);
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_failure();
    let turn = ExtensionData::new("turn-stage-a-breaker");
    let hook = StageATestHook {
        entered: None,
        release: None,
    };
    let worker_runs = Arc::new(std::sync::atomic::AtomicU32::new(/*v*/ 0));
    let hook_runs = Arc::clone(&worker_runs);
    *STAGE_A_TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(move || {
        hook_runs.fetch_add(/*val*/ 1, Ordering::Relaxed);
    }));
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &turn,
        )
        .await;
    drop(hook);

    assert!(fragments.is_empty());
    assert_stage_a_failure(&turn, "stage_a_gated");
    assert_eq!(worker_runs.load(Ordering::Relaxed), 0);
    assert!(thread.get::<StageABusy>().is_none());
    assert_eq!(breaker.consecutive_failures.load(Ordering::Relaxed), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_admitted_elapsed_ms_includes_worker_hook_wall_time() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let source = write_auto_km_for_test(
        &root,
        "codebase-graph/stage-a",
        "0.2.0",
        "Bounded graph context",
        "The owner validates the contract",
        "The owner validates the contract from the graph source.",
    );
    // Package validation hashes this padding on retrieval, making the reported
    // retrieval time nonzero so the old zero-only override cannot pass.
    std::fs::write(
        source.join("tests/README.md"),
        "x".repeat(/*n*/ 1024 * 1024),
    )
    .expect("write timing fixture padding");
    let store = KmStore::for_project_path(&root).expect("project store");
    store
        .import_for_project(&source, &root.display().to_string())
        .expect("import timing fixture");
    let question = "Trace the owner and then validate the contract using graph source evidence?";
    let attempt = km_lineage_attempt(&store, question);
    let LineageAttempt::Selected { cost, .. } = attempt else {
        panic!("expected timing fixture selection, got {attempt:?}");
    };
    assert!(
        cost.elapsed_ms > 0,
        "timing fixture must report retrieval time"
    );
    let turn = ExtensionData::new("turn-stage-a-wall-time");
    let hook = StageATestHook {
        entered: None,
        release: None,
    };
    let hook_delay = Duration::from_millis(/*millis*/ 50);
    *STAGE_A_TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(move || {
        std::thread::sleep(hook_delay);
    }));
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            stage_a_auto_km_input(&root),
            &ExtensionData::new("session-stage-a-wall-time"),
            &thread,
            &turn,
        )
        .await;
    drop(hook);

    assert_eq!(fragments.len(), 1);
    assert!(
        fragments[0]
            .render()
            .contains("<ontograph_lineage_context>")
    );
    assert!(!fragments[0].render().contains("<ontograph_km_context>"));
    assert_eq!(
        turn.get::<LineageAdmissionResult>()
            .expect("published admission")
            .0,
        Ok(())
    );
    let selected = decision_in(&turn);
    let RoutingDecision::Selected { cost, .. } = selected else {
        panic!("expected admitted selection, got {selected:?}");
    };
    assert!(u128::from(cost.elapsed_ms) >= hook_delay.as_millis());
    assert_stage_a_auto_km_unmarked(&thread, &root);
    assert!(
        !thread
            .get_or_init(OntographThreadState::default)
            .auto_km_context_was_injected(
                &root.display().to_string(),
                "codebase-graph/stage-a",
                "0.2.0",
            )
    );
}

#[tokio::test]
async fn agent_knowledge_installer_matches_metrics_none_and_emits_no_metric() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let mut legacy_builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install_agent_knowledge_surface(&mut legacy_builder);
    let mut metrics_builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install_agent_knowledge_surface_with_metrics(
        &mut metrics_builder,
        /*metrics_client*/ None,
    );
    let legacy = legacy_builder.build();
    let explicit_none = metrics_builder.build();
    for registry in [&legacy, &explicit_none] {
        assert_eq!(
            (
                registry.tool_contributors().len(),
                registry.thread_lifecycle_contributors().len(),
                registry.turn_input_contributors().len(),
                registry.turn_item_contributors().len(),
            ),
            (1, 1, 1, 1)
        );
        assert!(registry.turn_lifecycle_contributors().is_empty());
        assert!(registry.config_contributors().is_empty());
        assert!(registry.token_usage_contributors().is_empty());
        assert!(registry.context_contributors().is_empty());
        assert!(registry.tool_lifecycle_contributors().is_empty());
    }
    let (metrics, exporter) = lineage_test_metrics();
    let mut rendered = Vec::new();
    let mut tool_names = Vec::new();
    let mut grounded_decisions = Vec::new();
    for registry in [&legacy, &explicit_none] {
        let (_dir, root, thread) = stage_a_test_project();
        let session = ExtensionData::new("session-installer");
        let turn = ExtensionData::new("turn-installer");
        tool_names.push(
            registry.tool_contributors()[0]
                .tools(&session, &thread)
                .iter()
                .map(|tool| tool.spec().name().to_string())
                .collect::<Vec<_>>(),
        );
        let fragments = registry.turn_input_contributors()[0]
            .contribute(stage_a_auto_km_input(&root), &session, &thread, &turn)
            .await;
        assert_eq!(fragments.len(), 1);
        assert!(
            fragments[0]
                .render()
                .contains("<ontograph_lineage_context>")
        );
        assert_eq!(
            turn.get::<LineageAdmissionResult>()
                .expect("installer admission")
                .0,
            Ok(())
        );
        assert!(turn.get::<LineageMetricAttempted>().is_none());
        rendered.push(fragments[0].render());
        let mut item = agent_message("The owner validates the contract from the graph source.");
        registry.turn_item_contributors()[0]
            .contribute(&thread, &turn, &mut item)
            .await
            .expect("registered grounding contributor");
        let grounded = decision_in(&turn);
        assert!(matches!(&grounded, RoutingDecision::Baseline { .. }));
        grounded_decisions.push(grounded);
        assert!(turn.get::<LineageMetricAttempted>().is_none());
    }
    assert_eq!(tool_names[0], tool_names[1]);
    assert_eq!(rendered[0], rendered[1]);
    assert_eq!(grounded_decisions[0], grounded_decisions[1]);
    metrics.shutdown().expect("flush unrelated metrics client");
    assert_eq!(lineage_counter_points(&exporter), Vec::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stage_a_scope_change_discards_completed_private_state() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let other_dir = tempdir().expect("other project");
    std::fs::create_dir(other_dir.path().join(".git")).expect("other git dir");
    let other_root = std::fs::canonicalize(other_dir.path()).expect("other project root");
    let turn = Arc::new(ExtensionData::new("turn-stage-a-scope-changed"));
    let mut hook = StageATestHook::blocking();
    let job = spawn_stage_a_contribution(&root, &thread, &turn);
    hook.entered().await;
    assert!(turn.get::<LineageTurnDecision>().is_none());
    assert_stage_a_unpublished(&turn);
    thread
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(other_root.clone());
    hook.release();
    let fragments = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), job)
        .await
        .expect("scope-changed contribution finishes")
        .expect("scope change task");
    stage_a_worker_finished(&thread).await;
    drop(hook);
    assert!(fragments.is_empty());
    assert_stage_a_failure(&turn, "scope_changed");
    assert_eq!(
        thread
            .get_or_init(OntographThreadState::default)
            .current_cwd(),
        Some(other_root)
    );
}

#[tokio::test]
async fn stage_a_zero_remaining_deadline_never_spawns_a_worker() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let turn = ExtensionData::new("turn-stage-a-zero-budget");
    turn.insert(TurnDeadline::from_start(Instant::now(), Duration::ZERO));
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(/*v*/ 0));
    let hook_calls = Arc::clone(&calls);
    let hook = StageATestHook {
        entered: None,
        release: None,
    };
    *STAGE_A_TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(move || {
        hook_calls.fetch_add(/*val*/ 1, std::sync::atomic::Ordering::Relaxed);
    }));
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            stage_a_test_input(&root),
            &ExtensionData::new("session-stage-a"),
            &thread,
            &turn,
        )
        .await;
    drop(hook);
    assert!(fragments.is_empty());
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(
        thread.get::<StageABusy>().is_none(),
        "no blocking job was started"
    );
    assert_stage_a_unpublished(&turn);
    assert_eq!(
        decision_in(&turn),
        RoutingDecision::Baseline {
            reason: FallbackReason::BudgetExhausted {
                remaining_ms: 0,
                required_ms: RoutingConfig::default().max_latency_ms,
            },
        }
    );
}

#[test]
fn turn_input_seam_accepts_only_bounded_local_baseline_facts() {
    let input = auto_km_turn_input("Trace the owner and then validate the contract?");
    let outcome = crate::lineage_routing::induce_baseline_outcome(&input.user_input)
        .expect("local baseline induction");
    assert_eq!(outcome.induced_demands, 2);
    assert_eq!(outcome.confidence_bp, 9_500);
}

/// Multi-hop question that is eligible whenever routing is enabled.
fn eligible_baseline() -> BaselineOutcome {
    crate::lineage_routing::induce_baseline_outcome(
        &auto_km_turn_input("Trace the owner and then validate the contract?").user_input,
    )
    .expect("local baseline induction")
}

/// Budget comfortably covering the default `max_latency_ms`.
fn ample_budget() -> LineageBudget {
    LineageBudget::from_remaining_ms(u64::MAX)
}

/// A grounded attempt whose claim is usable and fully resolved.
fn usable_attempt() -> LineageAttempt {
    LineageAttempt::Completed {
        grounding: GroundingReport {
            claims: vec![GroundedClaim {
                passage_id: "p-1".to_string(),
                demand_id: "d-1".to_string(),
                source_span: "src/a.rs:1-4".to_string(),
                containment: ContainmentStatus::Contained,
                demand_support: DemandSupport::Retrieved,
            }],
            unresolved_demands: Vec::new(),
        },
        cost: LineageCost {
            elapsed_ms: 10,
            token_cost: 10,
        },
    }
}

/// Source text the selected passage resolves to, with its generation id.
fn selected_generation() -> SourceGeneration {
    SourceGeneration::from_files(vec![SourceGenerationFile {
        path: "src/a.rs".to_string(),
        generation_id: "gen-1".to_string(),
        lines: vec![
            "fn owner() {".to_string(),
            "    // the owner validates the contract".to_string(),
            "}".to_string(),
            "".to_string(),
        ],
    }])
}

/// One admitted passage with the ledger and generation that produced it.
fn selected_attempt() -> LineageAttempt {
    let exposure = FirstExposure {
        demand_id: "d-1".to_string(),
        rank: 0,
        score: 9,
    };
    let mut exposures = BTreeMap::new();
    exposures.insert("d-1".to_string(), exposure.clone());
    let mut membership = BTreeSet::new();
    membership.insert("d-1".to_string());
    let mut covered = BTreeSet::new();
    covered.insert("d-1".to_string());

    LineageAttempt::Selected {
        ledger: LineageLedger {
            demands: vec![EvidenceDemand {
                demand_id: "d-1".to_string(),
                description: "owner".to_string(),
                bindings: BTreeMap::new(),
            }],
            passages: vec![PassageLineage {
                passage_id: "p-1".to_string(),
                source_location: SourceLocation {
                    path: "src/a.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(4),
                },
                first_exposure: exposure,
                demand_membership: membership,
                exposures,
            }],
            unresolved_demands: Vec::new(),
        },
        selection: SelectionReport {
            selected: vec![SelectedPassage {
                passage_id: "p-1".to_string(),
                rank: 0,
                newly_covered_demands: covered.clone(),
                marginal_gain: 1,
                token_cost: 10,
                source_span: "src/a.rs:1-4".to_string(),
            }],
            covered_demands: covered,
            uncovered_demands: Vec::new(),
            total_token_cost: 10,
            stop_reason: SelectionStop::CoverageExhausted,
        },
        generation: selected_generation(),
        cost: LineageCost {
            elapsed_ms: 10,
            token_cost: 10,
        },
    }
}

/// Two selected passages whose answer support is split across two sentences.
fn selected_two_passage_attempt() -> LineageAttempt {
    let first_exposure = FirstExposure {
        demand_id: "d-1".to_string(),
        rank: 0,
        score: 9,
    };
    let second_exposure = FirstExposure {
        demand_id: "d-2".to_string(),
        rank: 1,
        score: 8,
    };
    let mut first_exposures = BTreeMap::new();
    first_exposures.insert("d-1".to_string(), first_exposure.clone());
    let mut second_exposures = BTreeMap::new();
    second_exposures.insert("d-2".to_string(), second_exposure.clone());
    let mut first_membership = BTreeSet::new();
    first_membership.insert("d-1".to_string());
    let mut second_membership = BTreeSet::new();
    second_membership.insert("d-2".to_string());
    let mut covered = BTreeSet::new();
    covered.insert("d-1".to_string());
    covered.insert("d-2".to_string());

    LineageAttempt::Selected {
        ledger: LineageLedger {
            demands: vec![
                EvidenceDemand {
                    demand_id: "d-1".to_string(),
                    description: "owner".to_string(),
                    bindings: BTreeMap::new(),
                },
                EvidenceDemand {
                    demand_id: "d-2".to_string(),
                    description: "validated".to_string(),
                    bindings: BTreeMap::new(),
                },
            ],
            passages: vec![
                PassageLineage {
                    passage_id: "p-1".to_string(),
                    source_location: SourceLocation {
                        path: "src/a.rs".to_string(),
                        start_line: Some(1),
                        end_line: Some(1),
                    },
                    first_exposure,
                    demand_membership: first_membership,
                    exposures: first_exposures,
                },
                PassageLineage {
                    passage_id: "p-2".to_string(),
                    source_location: SourceLocation {
                        path: "src/b.rs".to_string(),
                        start_line: Some(1),
                        end_line: Some(1),
                    },
                    first_exposure: second_exposure,
                    demand_membership: second_membership,
                    exposures: second_exposures,
                },
            ],
            unresolved_demands: Vec::new(),
        },
        selection: SelectionReport {
            selected: vec![
                SelectedPassage {
                    passage_id: "p-1".to_string(),
                    rank: 0,
                    newly_covered_demands: ["d-1".to_string()].into_iter().collect(),
                    marginal_gain: 1,
                    token_cost: 10,
                    source_span: "src/a.rs:1-1".to_string(),
                },
                SelectedPassage {
                    passage_id: "p-2".to_string(),
                    rank: 1,
                    newly_covered_demands: ["d-2".to_string()].into_iter().collect(),
                    marginal_gain: 1,
                    token_cost: 10,
                    source_span: "src/b.rs:1-1".to_string(),
                },
            ],
            covered_demands: covered,
            uncovered_demands: Vec::new(),
            total_token_cost: 20,
            stop_reason: SelectionStop::CoverageExhausted,
        },
        generation: SourceGeneration::from_files(vec![
            SourceGenerationFile {
                path: "src/a.rs".to_string(),
                generation_id: "gen-a".to_string(),
                lines: vec!["the owner is alice".to_string()],
            },
            SourceGenerationFile {
                path: "src/b.rs".to_string(),
                generation_id: "gen-b".to_string(),
                lines: vec!["the contract is validated".to_string()],
            },
        ]),
        cost: LineageCost {
            elapsed_ms: 10,
            token_cost: 20,
        },
    }
}

/// Routes one turn and returns the decision recorded for the turn-item seam.
fn recorded_decision(
    baseline: &BaselineOutcome,
    thread_store: &ExtensionData,
    budget: LineageBudget,
    attempt: impl FnOnce() -> LineageAttempt,
) -> RoutingDecision {
    let turn_store = ExtensionData::new("turn-lineage");
    record_lineage_routing_decision(baseline, thread_store, &turn_store, budget, attempt);
    turn_store
        .get::<LineageTurnDecision>()
        .expect("every routed turn records a decision")
        .decision
        .clone()
}

#[test]
fn lineage_routing_is_unreachable_when_the_feature_is_absent_or_disabled() {
    let baseline = eligible_baseline();
    let attempted = Cell::new(false);
    let attempt = || {
        attempted.set(true);
        usable_attempt()
    };

    // Absent thread state must fail closed exactly like an explicit false, and
    // the attempt must never run even though it would have produced evidence.
    let absent = ExtensionData::new("thread-lineage-absent");
    assert_eq!(
        recorded_decision(&baseline, &absent, ample_budget(), attempt),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );

    let disabled = ExtensionData::new("thread-lineage-disabled");
    disabled.insert(LineageRoutingEligibility::from_enabled(false));
    assert_eq!(
        recorded_decision(&baseline, &disabled, ample_budget(), attempt),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
    assert!(
        !attempted.get(),
        "no lineage work may run while routing is disabled"
    );
}

#[test]
fn enabled_lineage_routing_records_selection_without_emitting_a_fragment() {
    let baseline = eligible_baseline();
    let enabled = ExtensionData::new("thread-lineage-enabled");
    enabled.insert(LineageRoutingEligibility::from_enabled(true));

    // Selection is recorded for the turn-item seam, which is the first point
    // where assistant claims exist and grounding can decide eligibility.
    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        ..
    } = selected_attempt()
    else {
        unreachable!("selected_attempt is Selected")
    };
    assert_eq!(
        recorded_decision(&baseline, &enabled, ample_budget(), selected_attempt),
        RoutingDecision::Selected {
            ledger,
            selection,
            generation,
            cost: LineageCost {
                elapsed_ms: 10,
                token_cost: 10,
            },
        }
    );
}

#[tokio::test]
async fn enabled_lineage_routing_falls_back_when_the_attempt_fails() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let baseline = eligible_baseline();
    let enabled = ExtensionData::new("thread-lineage-failing");
    enabled.insert(LineageRoutingEligibility::from_enabled(true));

    // Distinguishes real fallback from the disabled path above. The thread has
    // no active working directory, so the host attempt cannot resolve a KM
    // store and must record why rather than returning empty evidence.
    assert_eq!(
        recorded_decision(&baseline, &enabled, ample_budget(), || {
            host_lineage_attempt(
                &auto_km_turn_input("trace the owner").user_input,
                &enabled,
                |_| {},
            )
        }),
        RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(
                "no active working directory for km lineage retrieval".to_string()
            )
        }
    );
}

#[test]
fn an_exhausted_budget_cancels_lineage_work_before_it_starts() {
    let baseline = eligible_baseline();
    let enabled = ExtensionData::new("thread-lineage-deadline");
    enabled.insert(LineageRoutingEligibility::from_enabled(true));
    let attempted = Cell::new(false);

    // A budget of zero leaves no time to start, so it must cancel pre-emptively.
    let decision = recorded_decision(
        &baseline,
        &enabled,
        LineageBudget::from_remaining_ms(0),
        || {
            attempted.set(true);
            selected_attempt()
        },
    );

    assert_eq!(
        decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::BudgetExhausted {
                remaining_ms: 0,
                required_ms: 10_000,
            }
        }
    );
    assert!(
        !attempted.get(),
        "an exhausted deadline must not start lineage work"
    );
}

#[test]
fn an_empty_selection_falls_back_with_no_usable_evidence() {
    let baseline = eligible_baseline();
    let enabled = ExtensionData::new("thread-lineage-empty");
    enabled.insert(LineageRoutingEligibility::from_enabled(true));

    let decision = recorded_decision(&baseline, &enabled, ample_budget(), || {
        let LineageAttempt::Selected {
            ledger,
            mut selection,
            generation,
            cost,
        } = selected_attempt()
        else {
            unreachable!("selected_attempt is Selected")
        };
        selection.selected.clear();
        LineageAttempt::Selected {
            ledger,
            selection,
            generation,
            cost,
        }
    });

    assert_eq!(
        decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::NoUsableEvidence
        },
        "a selection admitting nothing carries no evidence forward"
    );
}

#[test]
fn simple_questions_stay_on_baseline_even_when_routing_is_enabled() {
    let simple = crate::lineage_routing::induce_baseline_outcome(
        &auto_km_turn_input("Where is the owner defined?").user_input,
    )
    .expect("local baseline induction");
    assert_eq!(simple.induced_demands, 1);

    let enabled = ExtensionData::new("thread-lineage-simple");
    enabled.insert(LineageRoutingEligibility::from_enabled(true));
    assert_eq!(
        recorded_decision(&simple, &enabled, ample_budget(), selected_attempt),
        RoutingDecision::Baseline {
            reason: FallbackReason::NotEligible {
                induced_demands: 1,
                confidence_bp: 9_500,
            }
        }
    );
}

/// Turn store holding one already-routed `Selected` decision.
fn turn_store_with_selection() -> ExtensionData {
    let turn_store = ExtensionData::new("turn-grounding");
    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        cost,
    } = selected_attempt()
    else {
        unreachable!("selected_attempt is Selected")
    };
    turn_store.insert(LineageTurnDecision {
        decision: RoutingDecision::Selected {
            ledger,
            selection,
            generation,
            cost,
        },
    });
    turn_store
}

/// One assistant message carrying `text`.
fn agent_message(text: &str) -> TurnItem {
    TurnItem::AgentMessage(ontocode_protocol::items::AgentMessageItem {
        id: "item-1".to_string(),
        content: vec![AgentMessageContent::Text {
            text: text.to_string(),
        }],
        phase: None,
        memory_citation: None,
    })
}

/// Reads back the decision recorded in a turn store.
fn decision_in(turn_store: &ExtensionData) -> RoutingDecision {
    turn_store
        .get::<LineageTurnDecision>()
        .expect("a decision must be present")
        .decision
        .clone()
}

#[test]
fn routing_tags_cover_all_stage_a_tuples() {
    let selected = decision_in(&turn_store_with_selection());
    let rejections = [
        (AdmissionRejection::ScopeInvalid, "scope_invalid"),
        (AdmissionRejection::SourceStale, "source_stale"),
        (AdmissionRejection::SourceUnavailable, "source_unavailable"),
        (AdmissionRejection::SchemaInvalid, "schema_invalid"),
        (AdmissionRejection::ByteCapExceeded, "byte_cap_exceeded"),
        (AdmissionRejection::RenderInvalid, "render_invalid"),
    ];
    let fallbacks = [
        (
            FallbackReason::InductionFailed("detail".to_string()),
            "induction_failed",
        ),
        (
            FallbackReason::RetrievalFailed("detail".to_string()),
            "retrieval_failed",
        ),
        (
            FallbackReason::SelectionFailed("detail".to_string()),
            "selection_failed",
        ),
        (
            FallbackReason::SchemaInvalid("detail".to_string()),
            "schema_invalid",
        ),
        (
            FallbackReason::Timeout {
                elapsed_ms: 11,
                limit_ms: 10,
            },
            "timeout",
        ),
        (
            FallbackReason::BudgetExhausted {
                remaining_ms: 0,
                required_ms: 10,
            },
            "budget_exhausted",
        ),
        (
            FallbackReason::TokenCapExceeded {
                token_cost: 11,
                limit: 10,
            },
            "token_cap_exceeded",
        ),
        (
            FallbackReason::PartialCoverage {
                unresolved_demands: vec!["d-1".to_string()],
            },
            "partial_coverage",
        ),
        (FallbackReason::NoUsableEvidence, "no_usable_evidence"),
        (
            FallbackReason::NotEligible {
                induced_demands: 1,
                confidence_bp: 9_500,
            },
            "not_eligible",
        ),
    ];
    let mut cases = vec![
        (
            selected.clone(),
            None,
            [
                ("outcome", "lineage"),
                ("admission", "accepted"),
                ("fallback_category", "none"),
            ],
        ),
        (
            RoutingDecision::Baseline {
                reason: FallbackReason::Disabled,
            },
            None,
            [
                ("outcome", "baseline"),
                ("admission", "disabled"),
                ("fallback_category", "disabled"),
            ],
        ),
    ];
    cases.extend(rejections.map(|(rejection, category)| {
        (
            selected.clone(),
            Some(rejection),
            [
                ("outcome", "baseline"),
                ("admission", "rejected"),
                ("fallback_category", category),
            ],
        )
    }));
    cases.extend(fallbacks.map(|(reason, category)| {
        (
            RoutingDecision::Baseline { reason },
            None,
            [
                ("outcome", "baseline"),
                ("admission", "rejected"),
                ("fallback_category", category),
            ],
        )
    }));
    // SchemaInvalid is reachable through both admission and routing.
    assert_eq!(cases.len(), 18);
    assert_eq!(
        cases
            .iter()
            .map(|(_, _, tags)| *tags)
            .collect::<BTreeSet<_>>()
            .len(),
        17
    );
    for (decision, rejection, expected) in cases {
        assert_eq!(
            routing_tags(&decision, rejection),
            Some(expected),
            "{decision:?}, {rejection:?}"
        );
    }
}

#[test]
fn routing_tags_skip_stage_b_decisions() {
    let decisions = [
        RoutingDecision::Baseline {
            reason: FallbackReason::GroundingFailed("detail".to_string()),
        },
        RoutingDecision::Lineage {
            usable_claims: 1,
            unresolved_demands: Vec::new(),
            cost: LineageCost {
                elapsed_ms: 10,
                token_cost: 10,
            },
        },
    ];
    for decision in decisions {
        assert_eq!(routing_tags(&decision, /*r*/ None), None);
    }
}

#[test]
fn emit_stage_a_without_a_client_does_not_consume_the_once_guard() {
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let selected = decision_in(&turn_store);

    emit_stage_a(/*client*/ None, &turn_store);
    emit_stage_a(/*client*/ None, &turn_store);

    assert!(turn_store.get::<LineageMetricAttempted>().is_none());
    assert_eq!(decision_in(&turn_store), selected);
    assert_eq!(
        turn_store
            .get::<LineageAdmissionResult>()
            .expect("admission unchanged")
            .0,
        Ok(())
    );
}

#[test]
fn emit_stage_a_unresolved_selection_does_not_consume_the_once_guard() {
    let turn_store = turn_store_with_selection();
    let selected = decision_in(&turn_store);

    emit_stage_a(/*client*/ None, &turn_store);

    assert!(turn_store.get::<LineageMetricAttempted>().is_none());
    assert!(turn_store.get::<LineageAdmissionResult>().is_none());
    assert_eq!(decision_in(&turn_store), selected);
}

// Exporter-backed tests require the opentelemetry_sdk dev-dependency
// with the testing feature; pure routing/guard tests above use no exporter.
// [dev-dependencies]: opentelemetry_sdk = { workspace = true, features = ["testing"] }
fn lineage_test_metrics() -> (
    ontocode_otel::MetricsClient,
    opentelemetry_sdk::metrics::InMemoryMetricExporter,
) {
    let exporter = opentelemetry_sdk::metrics::InMemoryMetricExporter::default();
    let config = ontocode_otel::MetricsConfig::in_memory(
        "test",
        "ontocode-ontograph-extension",
        env!("CARGO_PKG_VERSION"),
        exporter.clone(),
    );
    let metrics = ontocode_otel::MetricsClient::new(config).expect("in-memory metrics client");
    (metrics, exporter)
}

fn lineage_counter_points(
    exporter: &opentelemetry_sdk::metrics::InMemoryMetricExporter,
) -> Vec<(u64, BTreeMap<String, String>)> {
    use opentelemetry_sdk::metrics::data::AggregatedMetrics;
    use opentelemetry_sdk::metrics::data::MetricData;

    let mut points = Vec::new();
    // Counters are cumulative, so each export repeats earlier totals; only the
    // final export holds the current value of every series.
    let exports = exporter.get_finished_metrics().expect("finished metrics");
    if let Some(resource) = exports.last() {
        for scope in resource.scope_metrics() {
            for metric in scope.metrics() {
                if metric.name() != "lineage_routing_attempt_total"
                    && metric.name() != "lineage_shadow_attempt_total"
                {
                    continue;
                }
                let AggregatedMetrics::U64(MetricData::Sum(sum)) = metric.data() else {
                    panic!("unexpected lineage counter aggregation");
                };
                points.extend(sum.data_points().map(|point| {
                    (
                        point.value(),
                        point
                            .attributes()
                            .map(|attribute| {
                                (
                                    attribute.key.as_str().to_string(),
                                    attribute.value.as_str().to_string(),
                                )
                            })
                            .collect(),
                    )
                }));
            }
        }
    }
    points
}

fn assert_stage_a_baseline_metric_once(
    metrics: &MetricsClient,
    exporter: &opentelemetry_sdk::metrics::InMemoryMetricExporter,
    turn: &ExtensionData,
    category: &str,
) {
    assert!(turn.get::<LineageMetricAttempted>().is_some());
    emit_stage_a(Some(metrics), turn);
    metrics.shutdown().expect("flush Stage A fallback metrics");
    assert_eq!(
        lineage_counter_points(exporter),
        vec![(
            1,
            BTreeMap::from([
                ("outcome".to_string(), "baseline".to_string()),
                ("admission".to_string(), "rejected".to_string()),
                ("fallback_category".to_string(), category.to_string()),
            ]),
        )]
    );
}

#[test]
fn emit_stage_a_without_a_client_emits_nothing() {
    let (metrics, exporter) = lineage_test_metrics();
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));

    emit_stage_a(/*client*/ None, &turn_store);
    metrics.shutdown().expect("flush metrics");

    assert_eq!(lineage_counter_points(&exporter), Vec::new());
    assert!(turn_store.get::<LineageMetricAttempted>().is_none());
}

#[test]
fn emit_stage_a_with_a_client_increments_exactly_once() {
    let (metrics, exporter) = lineage_test_metrics();
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let selected = decision_in(&turn_store);
    emit_stage_a(/*client*/ None, &turn_store);
    assert!(turn_store.get::<LineageMetricAttempted>().is_none());

    emit_stage_a(Some(&metrics), &turn_store);
    assert!(turn_store.get::<LineageMetricAttempted>().is_some());
    emit_stage_a(Some(&metrics), &turn_store);
    metrics.shutdown().expect("flush metrics");

    assert_eq!(
        lineage_counter_points(&exporter),
        vec![(
            1,
            BTreeMap::from([
                ("outcome".to_string(), "lineage".to_string()),
                ("admission".to_string(), "accepted".to_string()),
                ("fallback_category".to_string(), "none".to_string()),
            ])
        )]
    );
    assert_eq!(decision_in(&turn_store), selected);
}

#[test]
fn emit_stage_a_unresolved_selection_emits_nothing_with_a_client() {
    let (metrics, exporter) = lineage_test_metrics();
    let turn_store = turn_store_with_selection();

    emit_stage_a(Some(&metrics), &turn_store);
    metrics.shutdown().expect("flush metrics");

    assert_eq!(lineage_counter_points(&exporter), Vec::new());
    assert!(turn_store.get::<LineageMetricAttempted>().is_none());
    assert!(turn_store.get::<LineageAdmissionResult>().is_none());
}

#[tokio::test]
async fn rejected_admission_cannot_ground_a_matching_assistant_message() {
    for rejection in [
        AdmissionRejection::ScopeInvalid,
        AdmissionRejection::SourceStale,
        AdmissionRejection::SourceUnavailable,
        AdmissionRejection::SchemaInvalid,
        AdmissionRejection::ByteCapExceeded,
        AdmissionRejection::RenderInvalid,
    ] {
        let turn_store = turn_store_with_selection();
        turn_store.insert(LineageAdmissionResult(Err(rejection)));
        let mut item = agent_message("the owner validates the contract");
        let original = serde_json::to_value(&item).expect("serialize original message");
        let thread_store = ExtensionData::new("thread-rejected-admission");
        // Repeated assistant items must never revive the rejected selection.
        for _ in 0..2 {
            LineageGroundingContributor
                .contribute(&thread_store, &turn_store, &mut item)
                .await
                .expect("grounding never fails the turn");
            assert_eq!(
                decision_in(&turn_store),
                RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                }
            );
            assert_eq!(
                serde_json::to_value(&item).expect("serialize message"),
                original
            );
            assert_eq!(
                turn_store
                    .get::<LineageAdmissionResult>()
                    .expect("rejection unchanged")
                    .0,
                Err(rejection)
            );
        }
    }
}

#[tokio::test]
async fn grounding_accepts_an_answer_quoting_the_selected_span() {
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    // The quoted text appears in the selected span, and the citing demand is
    // the one that retrieved the passage, so the claim is usable.
    let mut item = agent_message("the owner validates the contract");

    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-grounding"),
            &turn_store,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");

    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Lineage {
            usable_claims: 1,
            unresolved_demands: Vec::new(),
            cost: LineageCost {
                elapsed_ms: 10,
                token_cost: 10,
            },
        }
    );
}

#[tokio::test]
async fn grounding_uses_the_sentence_for_each_selected_passage() {
    let turn_store = ExtensionData::new("turn-grounding-multi-passage");
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let LineageAttempt::Selected {
        ledger,
        selection,
        generation,
        cost,
    } = selected_two_passage_attempt()
    else {
        unreachable!("selected_two_passage_attempt is Selected")
    };
    turn_store.insert(LineageTurnDecision {
        decision: RoutingDecision::Selected {
            ledger,
            selection,
            generation,
            cost,
        },
    });
    let mut item = agent_message("the owner is alice. the contract is validated.");

    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-grounding-multi-passage"),
            &turn_store,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");

    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Lineage {
            usable_claims: 2,
            unresolved_demands: Vec::new(),
            cost: LineageCost {
                elapsed_ms: 10,
                token_cost: 20,
            },
        }
    );
}

#[tokio::test]
async fn an_answer_absent_from_the_span_falls_back_without_usable_evidence() {
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let mut item = agent_message("an unrelated sentence that appears nowhere in the source");

    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-grounding-miss"),
            &turn_store,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");

    // Containment failed, so nothing is usable and the turn stays on baseline.
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::NoUsableEvidence
        }
    );
}

#[tokio::test]
async fn grounding_leaves_the_turn_item_untouched() {
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let mut item = agent_message("the owner validates the contract");
    let before = format!("{item:?}");

    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-grounding-immutable"),
            &turn_store,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");

    // The answer contract is untouched: grounding records a decision only.
    assert_eq!(format!("{item:?}"), before);
}

#[tokio::test]
async fn grounding_is_skipped_without_a_recorded_selection() {
    // A turn that never routed, and a turn that recorded a baseline fallback,
    // must both leave grounding inert rather than inventing evidence.
    let unrouted = ExtensionData::new("turn-unrouted");
    let mut item = agent_message("the owner validates the contract");
    LineageGroundingContributor
        .contribute(&ExtensionData::new("thread-unrouted"), &unrouted, &mut item)
        .await
        .expect("grounding never fails the turn");
    assert!(unrouted.get::<LineageTurnDecision>().is_none());

    let fell_back = ExtensionData::new("turn-fell-back");
    fell_back.insert(LineageTurnDecision {
        decision: RoutingDecision::Baseline {
            reason: FallbackReason::Disabled,
        },
    });
    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-fell-back"),
            &fell_back,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");
    assert_eq!(
        decision_in(&fell_back),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
}

#[tokio::test]
async fn registered_lineage_contributors_share_turn_store_fallback_state() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install(&mut builder);
    let registry = builder.build();
    assert_eq!(registry.turn_input_contributors().len(), 1);
    assert_eq!(registry.turn_item_contributors().len(), 1);

    let session_store = ExtensionData::new("session-registered-lineage");
    let thread_store = ExtensionData::new("thread-registered-lineage");
    thread_store.insert(LineageRoutingEligibility::from_enabled(false));
    let turn_store = ExtensionData::new("turn-registered-lineage");
    let input = auto_km_turn_input("Trace the owner and then validate the contract?");

    registry.turn_input_contributors()[0]
        .contribute(input, &session_store, &thread_store, &turn_store)
        .await;

    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );

    let induction_failed_store = ExtensionData::new("turn-registered-induction-failed");
    thread_store.insert(LineageRoutingEligibility::from_enabled(true));
    let empty_input = TurnInputContext {
        turn_id: "turn-registered-induction-failed".to_string(),
        user_input: Vec::new(),
        environments: Vec::new(),
    };
    registry.turn_input_contributors()[0]
        .contribute(
            empty_input,
            &session_store,
            &thread_store,
            &induction_failed_store,
        )
        .await;
    assert!(matches!(
        decision_in(&induction_failed_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::InductionFailed(_)
        }
    ));

    let mut item = agent_message("the owner validates the contract");
    registry.turn_item_contributors()[0]
        .contribute(&thread_store, &turn_store, &mut item)
        .await
        .expect("registered grounding contributor never fails the turn");
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
}

/// An elapsed host deadline must exhaust the lineage budget.
///
/// This is the regression that distinguishes a measured countdown from the
/// former fixed allowance: the same turn that routes normally with a fresh
/// deadline must refuse to start retrieval once the deadline has passed.
#[tokio::test]
async fn an_elapsed_turn_deadline_exhausts_the_lineage_budget() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let session_store = ExtensionData::new("session-elapsed-deadline");
    let thread_store = ExtensionData::new("thread-elapsed-deadline");
    thread_store.insert(LineageRoutingEligibility::from_enabled(true));
    let turn_store = ExtensionData::new("turn-elapsed-deadline");
    // A deadline that has already passed: the turn started a second ago and was
    // granted one millisecond, so no budget remains by the time this seam runs.
    // `checked_sub` avoids relying on the monotonic clock being past one second.
    let started_at = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("test clock supports a one-second-old turn start");
    turn_store.insert(TurnDeadline::from_start(
        started_at,
        Duration::from_millis(1),
    ));

    SurfaceScopedOntographExtension::default()
        .contribute(
            auto_km_turn_input("Trace the owner and then validate the contract?"),
            &session_store,
            &thread_store,
            &turn_store,
        )
        .await;

    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::BudgetExhausted {
                remaining_ms: 0,
                required_ms: 10_000,
            }
        }
    );
}

/// Absence of a deadline is fail-closed, not unlimited time.
///
/// The host only publishes `TurnDeadline` once the turn start is marked, so a
/// turn without one must keep the fixed allowance rather than route as if it
/// had the whole budget left.
#[tokio::test]
async fn a_turn_without_a_deadline_keeps_the_fixed_allowance() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let session_store = ExtensionData::new("session-no-deadline");
    let thread_store = ExtensionData::new("thread-no-deadline");
    thread_store.insert(LineageRoutingEligibility::from_enabled(false));
    let turn_store = ExtensionData::new("turn-no-deadline");

    SurfaceScopedOntographExtension::default()
        .contribute(
            auto_km_turn_input("Trace the owner and then validate the contract?"),
            &session_store,
            &thread_store,
            &turn_store,
        )
        .await;

    // The fixed allowance covers the default latency cap, so the budget gate
    // does not fire and the disabled feature remains the recorded reason.
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
}

/// Every fallback reason must stay distinguishable through the registered
/// extension, not only through the internal recording helper.
///
/// The other fallback tests call `record_lineage_routing_decision` with an
/// injected attempt. This one drives the contributor the host actually
/// registers, so a reason that is only reachable through test plumbing would
/// not satisfy it.
#[tokio::test]
async fn registered_contribute_records_each_fallback_reason_distinctly() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install(&mut builder);
    let registry = builder.build();
    let contributor = &registry.turn_input_contributors()[0];
    let session_store = ExtensionData::new("session-fallback-matrix");
    let multi_hop = "Trace the owner and then validate the contract?";

    // Disabled: the feature gate is checked before any lineage work.
    let disabled_thread = ExtensionData::new("thread-fallback-disabled");
    disabled_thread.insert(LineageRoutingEligibility::from_enabled(false));
    let disabled_turn = ExtensionData::new("turn-fallback-disabled");
    contributor
        .contribute(
            auto_km_turn_input(multi_hop),
            &session_store,
            &disabled_thread,
            &disabled_turn,
        )
        .await;

    // NotEligible: enabled, but a single-hop question never routes.
    let eligible_thread = ExtensionData::new("thread-fallback-not-eligible");
    eligible_thread.insert(LineageRoutingEligibility::from_enabled(true));
    let not_eligible_turn = ExtensionData::new("turn-fallback-not-eligible");
    contributor
        .contribute(
            auto_km_turn_input("Where is the owner defined?"),
            &session_store,
            &eligible_thread,
            &not_eligible_turn,
        )
        .await;

    // InductionFailed: eligible and enabled, but the thread has no working
    // directory, so the host attempt cannot resolve a KM store.
    let retrieval_thread = ExtensionData::new("thread-fallback-retrieval");
    retrieval_thread.insert(LineageRoutingEligibility::from_enabled(true));
    let retrieval_turn = ExtensionData::new("turn-fallback-retrieval");
    contributor
        .contribute(
            auto_km_turn_input(multi_hop),
            &session_store,
            &retrieval_thread,
            &retrieval_turn,
        )
        .await;

    // BudgetExhausted: an elapsed deadline stops work before retrieval.
    let budget_thread = ExtensionData::new("thread-fallback-budget");
    budget_thread.insert(LineageRoutingEligibility::from_enabled(true));
    let budget_turn = ExtensionData::new("turn-fallback-budget");
    let started_at = Instant::now()
        .checked_sub(Duration::from_secs(1))
        .expect("test clock supports a one-second-old turn start");
    budget_turn.insert(TurnDeadline::from_start(
        started_at,
        Duration::from_millis(1),
    ));
    contributor
        .contribute(
            auto_km_turn_input(multi_hop),
            &session_store,
            &budget_thread,
            &budget_turn,
        )
        .await;

    let recorded = vec![
        decision_in(&disabled_turn),
        decision_in(&not_eligible_turn),
        decision_in(&retrieval_turn),
        decision_in(&budget_turn),
    ];

    assert_eq!(
        recorded,
        vec![
            RoutingDecision::Baseline {
                reason: FallbackReason::Disabled
            },
            RoutingDecision::Baseline {
                reason: FallbackReason::NotEligible {
                    induced_demands: 1,
                    confidence_bp: 9_500,
                }
            },
            RoutingDecision::Baseline {
                reason: FallbackReason::RetrievalFailed(
                    "no active working directory for km lineage retrieval".to_string()
                )
            },
            RoutingDecision::Baseline {
                reason: FallbackReason::BudgetExhausted {
                    remaining_ms: 0,
                    required_ms: 10_000,
                }
            },
        ]
    );
}

/// A rejected grounding must leave the answer byte-identical.
///
/// Recording a fallback is diagnostic state; it must never edit the turn item
/// the user sees.
#[tokio::test]
async fn registered_grounding_rejection_leaves_the_answer_byte_identical() {
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install(&mut builder);
    let registry = builder.build();
    let thread_store = ExtensionData::new("thread-grounding-rejection");
    // A selection is recorded, but the answer below quotes none of it, so
    // grounding runs and rejects instead of being skipped outright.
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let mut item = agent_message("an answer quoting nothing from the selected span");
    // `TurnItem` has no `PartialEq`, so the repository idiom compares the debug
    // rendering to prove the answer contract was untouched.
    let before = format!("{item:?}");

    registry.turn_item_contributors()[0]
        .contribute(&thread_store, &turn_store, &mut item)
        .await
        .expect("registered grounding contributor never fails the turn");

    assert_eq!(format!("{item:?}"), before);
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::NoUsableEvidence
        }
    );
}

#[tokio::test]
async fn an_empty_answer_falls_back_without_usable_evidence() {
    let turn_store = turn_store_with_selection();
    turn_store.insert(LineageAdmissionResult(Ok(())));
    let mut item = agent_message("   ");

    LineageGroundingContributor
        .contribute(
            &ExtensionData::new("thread-grounding-empty"),
            &turn_store,
            &mut item,
        )
        .await
        .expect("grounding never fails the turn");

    // An empty answer cannot support a selected passage, so Stage B records an
    // explicit baseline fallback while leaving the item unchanged.
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::NoUsableEvidence
        }
    );
}

#[test]
fn turn_input_seam_fails_closed_without_textual_baseline_facts() {
    let input = TurnInputContext {
        turn_id: "turn-empty".to_string(),
        user_input: Vec::new(),
        environments: Vec::new(),
    };
    assert!(crate::lineage_routing::induce_baseline_outcome(&input.user_input).is_err());
}

/// Regression: KM injection must not depend on lineage induction succeeding.
///
/// Induction is bounded to 16 KiB of turn text, but KM context has no such
/// bound. A large paste must still receive its KM fragment.
#[tokio::test]
async fn oversized_turn_input_still_receives_km_context() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let nested = dir.path().join("nested/deeper");
    std::fs::create_dir_all(&nested).expect("nested dir");
    let project_root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let store = KmStore::for_project_path(&nested).expect("project store");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, &project_root.display().to_string())
        .expect("import");
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-oversized");

    // Same matching question, plus a paste larger than the induction bound.
    let mut user_input =
        auto_km_turn_input("graph context retrieval question with enough extra words today")
            .user_input;
    user_input.push(UserInput::Text {
        text: "x".repeat(20_000),
        text_elements: Vec::new(),
    });

    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            TurnInputContext {
                turn_id: "turn-oversized".to_string(),
                user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: nested,
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    assert_eq!(
        fragments.len(),
        1,
        "KM context must survive turn input that exceeds the lineage induction bound"
    );
}

#[tokio::test]
async fn automatic_km_context_contributor_uses_the_thread_project_store() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let nested = dir.path().join("nested/deeper");
    std::fs::create_dir_all(&nested).expect("nested dir");
    let project_root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let store = KmStore::for_project_path(&nested).expect("project store");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, &project_root.display().to_string())
        .expect("import");
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-1");

    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: auto_km_turn_input(
                    "graph context retrieval question with enough extra words today",
                )
                .user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: nested,
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    assert_eq!(fragments.len(), 1);
    assert!(
        fragments[0]
            .body()
            .contains("codebase-graph/creating-ontograph")
    );
}

#[tokio::test]
async fn admitted_lineage_context_replaces_automatic_km_context() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "The owner validates the contract from the graph source.",
    );
    KmStore::for_project_path(&root)
        .expect("project store")
        .import_for_project(&source, &root.display().to_string())
        .expect("import KM");
    let thread_store = ExtensionData::new("thread-lineage-admitted");
    thread_store.insert(LineageRoutingEligibility::from_enabled(
        /*enabled*/ true,
    ));
    let turn_store = ExtensionData::new("turn-lineage-admitted");
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            TurnInputContext {
                turn_id: "turn-lineage-admitted".to_string(),
                user_input: auto_km_turn_input("Trace the owner and then validate the contract?")
                    .user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: root.clone(),
                    is_primary: true,
                }],
            },
            &ExtensionData::new("session-lineage-admitted"),
            &thread_store,
            &turn_store,
        )
        .await;

    assert_eq!(fragments.len(), 1);
    let rendered = fragments[0].render();
    assert!(rendered.contains("<ontograph_lineage_context>"));
    assert!(rendered.contains("The owner validates the contract from the graph source."));
    assert!(!rendered.contains("<ontograph_km_context>"));
    assert_eq!(
        turn_store
            .get::<LineageAdmissionResult>()
            .expect("admission result")
            .0,
        Ok(())
    );
    let thread_state = thread_store.get_or_init(OntographThreadState::default);
    assert!(!thread_state.auto_km_context_was_injected(
        &root.display().to_string(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
    ));

    let selected_decision = decision_in(&turn_store);
    assert!(matches!(
        &selected_decision,
        RoutingDecision::Selected { .. }
    ));
    let mut item = agent_message("The owner validates the contract from the graph source.");
    let original = serde_json::to_value(&item).expect("serialize original message");
    let RoutingDecision::Selected { selection, .. } = &selected_decision else {
        unreachable!("admitted lineage remains Selected until Stage B")
    };
    let unresolved_demands = selection.uncovered_demands.clone();
    assert!(!unresolved_demands.is_empty());
    assert_eq!(decision_in(&turn_store), selected_decision);
    LineageGroundingContributor
        .contribute(&thread_store, &turn_store, &mut item)
        .await
        .expect("Stage B grounding never fails the turn");
    // Only one example span matches this multi-demand question, so Stage B
    // grounds it against the untouched `Selected` decision and reports the
    // other demands as partial coverage rather than rejecting the turn.
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::PartialCoverage { unresolved_demands }
        }
    );
    assert_eq!(
        serde_json::to_value(&item).expect("serialize message"),
        original
    );
}

#[tokio::test]
async fn stale_lineage_admission_preserves_the_baseline_and_rejects_the_source() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "The owner validates the contract from the graph source.",
    );
    let store = KmStore::for_project_path(&root).expect("project store");
    let imported = store
        .import_for_project(&source, &root.display().to_string())
        .expect("import KM");
    let thread_store = ExtensionData::new("thread-lineage-stale");
    thread_store.insert(LineageRoutingEligibility::from_enabled(
        /*enabled*/ true,
    ));
    thread_store
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(root.clone());
    let turn_store = ExtensionData::new("turn-lineage-stale");
    let mut captured_scope = None;
    record_lineage_routing_decision(
        &eligible_baseline(),
        &thread_store,
        &turn_store,
        ample_budget(),
        || {
            host_lineage_attempt(
                &auto_km_turn_input("Trace the owner and then validate the contract?").user_input,
                &thread_store,
                |scope| captured_scope = Some(scope.to_path_buf()),
            )
        },
    );
    let selected_decision = decision_in(&turn_store);
    assert!(matches!(
        &selected_decision,
        RoutingDecision::Selected { .. }
    ));
    let baseline_fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        &root.display().to_string(),
    );
    assert_eq!(baseline_fragments.len(), 1);
    assert!(
        baseline_fragments[0]
            .render()
            .contains("<ontograph_km_context>")
    );

    // Tampering with the cached package invalidates it, so admission fails
    // closed as unavailable instead of silently admitting changed text.
    std::fs::write(
        std::path::Path::new(&imported.path).join("examples/README.md"),
        "The owner validates a changed contract from the graph source.",
    )
    .expect("mutate selected example");

    assert_eq!(
        admit_selected_turn(&thread_store, &turn_store, captured_scope.as_deref()),
        None
    );
    assert_eq!(
        turn_store
            .get::<LineageAdmissionResult>()
            .expect("admission rejection")
            .0,
        Err(AdmissionRejection::SourceUnavailable)
    );
    assert_eq!(decision_in(&turn_store), selected_decision);
}

#[tokio::test]
async fn completed_lineage_admission_rejection_preserves_automatic_km() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let store = KmStore::for_project_path(&root).expect("project store");
    let padding = "x".repeat(crate::lineage_admission::MAX_SPAN_BYTES);
    let example = format!("The owner validates the contract from the graph source.{padding}");
    // Stay below the fixture's manifest-source switch while exceeding the
    // lineage span cap: prepare a normal package, then enlarge its example.
    let source = write_auto_km_for_test(
        &root,
        "codebase-graph/admission-rejected",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "example",
    );
    std::fs::write(source.join("examples/README.md"), example)
        .expect("write oversized lineage span");
    store
        .import_for_project(&source, &root.display().to_string())
        .expect("import admission-rejected KM");
    // Withdraw the small span so selection must reach the admission cap.
    store
        .withdraw(
            "codebase-graph/stage-a",
            "0.1.0",
            "exercise oversized admission only".to_string(),
        )
        .expect("withdraw small-span fixture");
    let input = stage_a_auto_km_input(&root);
    let expected = matching_km_context_fragments(&input, &store, &root.display().to_string())
        .into_iter()
        .map(|fragment| fragment.render())
        .collect::<Vec<_>>();
    let turn = ExtensionData::new("turn-admission-rejected");
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            input,
            &ExtensionData::new("session-admission-rejected"),
            &thread,
            &turn,
        )
        .await;
    assert!(matches!(
        decision_in(&turn),
        RoutingDecision::Selected { .. }
    ));
    assert_eq!(
        turn.get::<LineageAdmissionResult>()
            .expect("completed admission rejection")
            .0,
        Err(AdmissionRejection::ByteCapExceeded)
    );
    assert_eq!(
        fragments
            .iter()
            .map(|fragment| fragment.render())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(fragments[0].render().contains("<ontograph_km_context>"));
    assert!(
        !fragments[0]
            .render()
            .contains("<ontograph_lineage_context>")
    );
    assert!(
        thread
            .get_or_init(OntographThreadState::default)
            .auto_km_context_was_injected(
                &root.display().to_string(),
                "codebase-graph/admission-rejected",
                "0.1.0",
            )
    );
}

#[tokio::test]
async fn tampered_lineage_source_preserves_healthy_automatic_km() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let (_dir, root, thread) = stage_a_test_project();
    let store = KmStore::for_project_path(&root).expect("project store");
    let input = stage_a_auto_km_input(&root);
    let healthy = write_auto_km_for_test(
        &root,
        "codebase-graph/healthy-fallback",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "Zebra sleeps quietly.",
    );
    store
        .import_for_project(&healthy, &root.display().to_string())
        .expect("import healthy automatic-KM fallback");
    thread
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(root.clone());
    let turn = ExtensionData::new("turn-tampered-fallback");
    let mut captured_scope = None;
    record_lineage_routing_decision(&eligible_baseline(), &thread, &turn, ample_budget(), || {
        host_lineage_attempt(&input.user_input, &thread, |scope| {
            captured_scope = Some(scope.to_path_buf());
        })
    });
    let selected = decision_in(&turn);
    assert!(matches!(&selected, RoutingDecision::Selected { .. }));
    let record = store
        .list(/*include_obsolete*/ false)
        .expect("active KMs")
        .into_iter()
        .find(|record| record.id == "codebase-graph/stage-a")
        .expect("retrieved KM");
    std::fs::write(
        Path::new(&record.path).join("examples/README.md"),
        "The owner validates a changed contract from the graph source.",
    )
    .expect("tamper selected source after retrieval");
    assert_eq!(
        admit_selected_turn(&thread, &turn, captured_scope.as_deref()),
        None
    );
    assert_eq!(
        turn.get::<LineageAdmissionResult>()
            .expect("tampered admission rejection")
            .0,
        Err(AdmissionRejection::SourceUnavailable)
    );
    assert_eq!(decision_in(&turn), selected);
    let expected = matching_km_context_fragments(&input, &store, &root.display().to_string())
        .into_iter()
        .map(|fragment| fragment.render())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 1);
    // The worker hook runs before retrieval, so this contribution verifies
    // fallback after tampering on a new turn. The byte-cap rejection test
    // above separately covers rejection and fallback in one contribution.
    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            input,
            &ExtensionData::new("session-tampered-fallback"),
            &thread,
            &ExtensionData::new("turn-after-tampering"),
        )
        .await;
    assert_eq!(
        fragments
            .iter()
            .map(|fragment| fragment.render())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        fragments[0]
            .render()
            .contains("codebase-graph/healthy-fallback")
    );
    assert!(fragments[0].render().contains("<ontograph_km_context>"));
    assert!(
        !fragments[0]
            .render()
            .contains("<ontograph_lineage_context>")
    );
    assert_stage_a_auto_km_unmarked(&thread, &root);
}

#[tokio::test]
async fn disabled_lineage_keeps_the_automatic_km_fragment_unchanged() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    let store = KmStore::for_project_path(&root).expect("project store");
    store
        .import_for_project(&source, &root.display().to_string())
        .expect("import KM");
    let input = auto_km_turn_input(
        "Trace graph context retrieval and then validate the answer with enough extra words today?",
    );
    let expected = matching_km_context_fragments(&input, &store, &root.display().to_string())
        .into_iter()
        .map(|fragment| fragment.render())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 1);
    let hook = StageATestHook {
        entered: None,
        release: None,
    };
    let hook_calls = Arc::new(std::sync::atomic::AtomicUsize::new(/*v*/ 0));
    let worker_calls = Arc::clone(&hook_calls);
    *STAGE_A_TEST_HOOK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(move || {
        worker_calls.fetch_add(/*val*/ 1, std::sync::atomic::Ordering::Relaxed);
    }));
    let thread_store = ExtensionData::new("thread-lineage-disabled");
    thread_store.insert(LineageRoutingEligibility::from_enabled(
        /*enabled*/ false,
    ));
    let turn_store = ExtensionData::new("turn-lineage-disabled");
    let actual = SurfaceScopedOntographExtension::default()
        .contribute(
            TurnInputContext {
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: root.clone(),
                    is_primary: true,
                }],
                ..input
            },
            &ExtensionData::new("session-lineage-disabled"),
            &thread_store,
            &turn_store,
        )
        .await
        .into_iter()
        .map(|fragment| fragment.render())
        .collect::<Vec<_>>();
    drop(hook);

    assert_eq!(actual, expected);
    assert!(actual[0].contains("<ontograph_km_context>"));
    assert!(!actual[0].contains("<ontograph_lineage_context>"));
    assert_eq!(hook_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
    assert!(thread_store.get::<StageABusy>().is_none());
    assert_stage_a_unpublished(&turn_store);
    assert!(
        thread_store
            .get_or_init(OntographThreadState::default)
            .auto_km_context_was_injected(
                &root.display().to_string(),
                "codebase-graph/creating-ontograph",
                "0.1.0",
            )
    );
    assert_eq!(
        decision_in(&turn_store),
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
}

#[tokio::test]
async fn changed_project_scope_rejects_the_retrieved_lineage() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let project_a = tempdir().expect("project A");
    let project_b = tempdir().expect("project B");
    std::fs::create_dir(project_a.path().join(".git")).expect("project A git dir");
    std::fs::create_dir(project_b.path().join(".git")).expect("project B git dir");
    let root_a = std::fs::canonicalize(project_a.path()).expect("project A root");
    let root_b = std::fs::canonicalize(project_b.path()).expect("project B root");
    let source = write_auto_km_for_test(
        project_a.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "The owner validates the contract",
        "The owner validates the contract from the graph source.",
    );
    KmStore::for_project_path(&root_a)
        .expect("project A store")
        .import_for_project(&source, &root_a.display().to_string())
        .expect("import KM");
    let thread_store = ExtensionData::new("thread-lineage-scope-change");
    thread_store.insert(LineageRoutingEligibility::from_enabled(
        /*enabled*/ true,
    ));
    thread_store
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(root_a);
    let turn_store = ExtensionData::new("turn-lineage-scope-change");
    let mut captured_scope = None;
    record_lineage_routing_decision(
        &eligible_baseline(),
        &thread_store,
        &turn_store,
        ample_budget(),
        || {
            host_lineage_attempt(
                &auto_km_turn_input("Trace the owner and then validate the contract?").user_input,
                &thread_store,
                |scope| captured_scope = Some(scope.to_path_buf()),
            )
        },
    );
    let selected_decision = decision_in(&turn_store);
    assert!(matches!(
        &selected_decision,
        RoutingDecision::Selected { .. }
    ));
    assert!(!root_b.join(".ontocode").exists());
    thread_store
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(root_b.clone());

    assert_eq!(
        admit_selected_turn(&thread_store, &turn_store, captured_scope.as_deref()),
        None
    );
    assert_eq!(
        turn_store
            .get::<LineageAdmissionResult>()
            .expect("scope rejection")
            .0,
        Err(AdmissionRejection::ScopeInvalid)
    );
    assert!(!root_b.join(".ontocode").exists());
    assert_eq!(decision_in(&turn_store), selected_decision);
}

#[tokio::test]
async fn automatic_km_context_contributor_skips_non_git_directories() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    let project_scope = std::fs::canonicalize(dir.path()).expect("canonical directory");
    let store = KmStore::new(dir.path().join(".ontocode/ontograph/km/_store"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, &project_scope.display().to_string())
        .expect("import");

    let fragments = SurfaceScopedOntographExtension::default()
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: auto_km_turn_input(
                    "graph context retrieval question with enough extra words today",
                )
                .user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: dir.path().to_path_buf(),
                    is_primary: true,
                }],
            },
            &ExtensionData::new("session"),
            &ExtensionData::new("thread"),
            &ExtensionData::new("turn"),
        )
        .await;

    assert!(fragments.is_empty());
}

#[tokio::test]
async fn automatic_km_context_reinjects_same_package_after_project_switch() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let project_a = tempdir().expect("project A");
    let project_b = tempdir().expect("project B");
    std::fs::create_dir(project_a.path().join(".git")).expect("project A git dir");
    std::fs::create_dir(project_b.path().join(".git")).expect("project B git dir");
    let project_a_root = std::fs::canonicalize(project_a.path()).expect("project A root");
    let project_b_root = std::fs::canonicalize(project_b.path()).expect("project B root");
    let source_a = write_auto_km_for_test(
        project_a.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "project A example",
    );
    let source_b = write_auto_km_for_test(
        project_b.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "project B example",
    );
    KmStore::for_project_path(project_a.path())
        .expect("project A store")
        .import_for_project(&source_a, &project_a_root.display().to_string())
        .expect("import project A");
    KmStore::for_project_path(project_b.path())
        .expect("project B store")
        .import_for_project(&source_b, &project_b_root.display().to_string())
        .expect("import project B");
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-1");

    for (turn_id, project_root) in [("turn-a", project_a_root), ("turn-b", project_b_root)] {
        let fragments = SurfaceScopedOntographExtension::default()
            .contribute(
                TurnInputContext {
                    turn_id: turn_id.to_string(),
                    user_input: auto_km_turn_input(
                        "graph context retrieval question with enough extra words today",
                    )
                    .user_input,
                    environments: vec![ontocode_extension_api::TurnInputEnvironment {
                        environment_id: "local".to_string(),
                        cwd: project_root,
                        is_primary: true,
                    }],
                },
                &session_store,
                &thread_store,
                &ExtensionData::new("turn"),
            )
            .await;

        assert_eq!(fragments.len(), 1, "{turn_id}");
    }
}

#[test]
fn automatic_km_context_injects_when_two_topic_words_match() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 1);
    assert!(
        fragments[0]
            .body()
            .contains("codebase-graph/creating-ontograph")
    );
}

#[test]
fn automatic_km_context_skips_obsolete_record() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    assert_eq!(
        matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE).len(),
        1,
        "eligible record is admitted before obsolescence"
    );
    store
        .obsolete("codebase-graph/creating-ontograph", "0.1.0")
        .expect("obsolete succeeds")
        .expect("record updated");

    assert!(
        matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE).is_empty(),
        "obsolete record is not admitted"
    );
}

#[test]
fn automatic_km_context_isolates_records_by_project_scope() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    // Imported under project A only.
    store
        .import_for_project(&source, "/tmp/project-a")
        .expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    // A thread active in project A auto-injects the record.
    assert_eq!(
        matching_km_context_fragments(&input, &store, "/tmp/project-a").len(),
        1
    );
    // A thread active in project B gets zero automatic injection.
    assert_eq!(
        matching_km_context_fragments(&input, &store, "/tmp/project-b").len(),
        0
    );

    // Explicit ontograph.km_context lookup by id ignores project scope and
    // still resolves the active record regardless of the active thread scope.
    let resolved = store
        .resolve_active("codebase-graph/creating-ontograph")
        .expect("resolve active");
    assert!(resolved.is_some());
}

#[test]
fn automatic_km_context_skips_unscoped_records() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    // Legacy/unscoped import (project_scope = None) must never auto-inject.
    store.import(&source).expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    assert_eq!(
        matching_km_context_fragments(&input, &store, "/tmp/project-a").len(),
        0
    );
}

#[test]
fn automatic_km_context_skips_one_match_and_short_turns() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    assert_eq!(
        matching_km_context_fragments(
            &auto_km_turn_input("graph only has enough extra words for threshold today"),
            &store,
            AUTO_KM_TEST_SCOPE,
        )
        .len(),
        0
    );
    assert_eq!(
        matching_km_context_fragments(
            &auto_km_turn_input("graph context short"),
            &store,
            AUTO_KM_TEST_SCOPE
        )
        .len(),
        0
    );
}

#[test]
fn automatic_km_context_ignores_non_text_input() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let input = TurnInputContext {
        turn_id: "turn-1".to_string(),
        user_input: vec![UserInput::Mention {
            name: "graph context retrieval question".to_string(),
            path: "app://example".to_string(),
        }],
        environments: Vec::new(),
    };

    assert_eq!(
        matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE).len(),
        0
    );
}

#[test]
fn automatic_km_context_injects_at_most_three_ranked_modules() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    for id in [
        "codebase-graph/a",
        "codebase-graph/b",
        "codebase-graph/c",
        "codebase-graph/d",
    ] {
        let source = write_auto_km_for_test(
            dir.path(),
            id,
            "0.1.0",
            "Bounded graph context",
            "Graph retrieval context should guide bounded answers",
            "example",
        );
        store
            .import_for_project(&source, AUTO_KM_TEST_SCOPE)
            .expect("import");
    }

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );
    let bodies = fragments
        .iter()
        .map(|fragment| fragment.body())
        .collect::<Vec<_>>();

    assert_eq!(bodies.len(), 3);
    assert!(bodies[0].contains("codebase-graph/a"));
    assert!(bodies[1].contains("codebase-graph/b"));
    assert!(bodies[2].contains("codebase-graph/c"));
}

#[test]
fn automatic_km_context_ranks_stronger_topic_overlap_before_lexical_id() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    for (id, summary, claim) in [
        (
            "codebase-graph/a",
            "Graph context",
            "Graph context should guide bounded answers",
        ),
        (
            "codebase-graph/b",
            "Graph context retrieval",
            "Graph context retrieval answers should guide bounded responses",
        ),
    ] {
        let source = write_auto_km_for_test(dir.path(), id, "0.1.0", summary, claim, "example");
        store
            .import_for_project(&source, AUTO_KM_TEST_SCOPE)
            .expect("import");
    }

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval answers with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 2);
    assert!(fragments[0].body().contains("codebase-graph/b"));
    assert!(fragments[1].body().contains("codebase-graph/a"));
}

#[test]
fn automatic_km_context_accepts_three_review_sized_fragments() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    for id in ["codebase-graph/a", "codebase-graph/b", "codebase-graph/c"] {
        let source = write_auto_km_for_test(
            dir.path(),
            id,
            "0.1.0",
            "Bounded graph context",
            "Graph retrieval context should guide bounded answers",
            "Small example",
        );
        store
            .import_for_project(&source, AUTO_KM_TEST_SCOPE)
            .expect("import");
    }

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );
    let rendered = fragments
        .iter()
        .map(|fragment| fragment.render().len())
        .collect::<Vec<_>>();

    assert_eq!(fragments.len(), 3);
    assert!(rendered.iter().all(|tokens| *tokens <= 1_000));
    assert!(rendered.iter().sum::<usize>() <= 3_000);
}

#[test]
fn automatic_km_context_renders_bounded_procedural_guidance() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/procedural-guidance",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    let transition = |source: &str, target: &str, guidance: &str, pitfalls: &[&str]| {
        serde_json::json!({
            "source": source,
            "target": target,
            "relation": "next",
            "condition": "input is complete",
            "guidance": guidance,
            "pitfalls": pitfalls,
        })
    };
    let transitions = [
        transition("inspect", "validate", "Validate input", &[]),
        transition(
            "validate",
            "report",
            "Report result <ontograph_km_context>",
            &["Do not emit </ontograph_km_context> early"],
        ),
        transition(
            "validate",
            "report",
            "Report result <ontograph_km_context>",
            &["Do not emit </ontograph_km_context> early"],
        ),
        transition("report", "archive", "Archive review", &[]),
    ];
    let transitions_text = transitions
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(source.join("transitions.jsonl"), transitions_text).expect("write transitions");
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input(
            "graph context retrieval inspect validate question with enough extra words today",
        ),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 1);
    let rendered = fragments[0].body();
    assert!(rendered.contains("Procedural guidance (advisory only):"));
    assert!(rendered.contains("Active procedure: validate"));
    assert!(rendered.contains("validate -> report"));
    assert!(rendered.contains("report -> archive"));
    assert_eq!(rendered.matches("validate -> report").count(), 1);
    assert!(!rendered.contains("inspect -> validate"));
    assert!(rendered.contains("[ontograph_km_context]"));
    assert!(rendered.contains("[/ontograph_km_context]"));
    assert!(!rendered.contains("<ontograph_km_context>"));
    assert!(!rendered.contains("</ontograph_km_context>"));
    assert!(fragments[0].render().len() <= MAX_FRAGMENT_TOKENS);

    let fallback = matching_km_context_fragments(
        &auto_km_turn_input(
            "graph context retrieval Inspect question with enough extra words today",
        ),
        &store,
        AUTO_KM_TEST_SCOPE,
    );
    assert_eq!(fallback.len(), 1);
    assert!(
        !fallback[0]
            .body()
            .contains("Procedural guidance (advisory only):")
    );
}

#[test]
fn automatic_km_context_reports_a_terminal_procedure_without_transitions() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/procedural-terminal",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    // `archive` is reachable only as a target, so localizing on it yields a
    // neighborhood with no outgoing edges.
    std::fs::write(
        source.join("transitions.jsonl"),
        serde_json::json!({
            "source": "report",
            "target": "archive",
            "relation": "next",
            "condition": "input is complete",
            "guidance": "Archive review",
            "pitfalls": [],
        })
        .to_string(),
    )
    .expect("write transitions");
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input(
            "graph context retrieval archive question with enough extra words today",
        ),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 1);
    let rendered = fragments[0].body();
    assert!(rendered.contains("Procedural guidance (advisory only):"));
    assert!(rendered.contains("Active procedure: archive"));
    assert!(
        rendered.contains("- No outgoing transitions were found in the localized neighborhood.")
    );
    assert!(!rendered.contains("report -> archive"));
    assert!(fragments[0].render().len() <= MAX_FRAGMENT_TOKENS);
}

#[test]
fn automatic_km_context_omits_procedural_guidance_when_exceeding_review_threshold() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/procedural-threshold",
        "0.1.0",
        "Threshold test KM",
        "Graph context retrieval answers should guide bounded responses and provide reliable facts for active development workflows.",
        "example",
    );
    let facts = [
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"Graph context retrieval answers should guide bounded responses and provide reliable facts for active development workflows.\",\"source\":\"silver:doc\",\"confidence\":0.9}",
        "{\"id\":\"fact-2\",\"kind\":\"rule\",\"claim\":\"Secondary guidance and factual annotations should be detailed enough to test boundary limits effectively in automated suites.\",\"source\":\"silver:doc\",\"confidence\":0.85}",
        "{\"id\":\"fact-3\",\"kind\":\"rule\",\"claim\":\"Procedural transitions should be dropped if their presence would violate the auto injection budget of one thousand tokens.\",\"source\":\"silver:doc\",\"confidence\":0.88}",
    ]
    .join("\n");
    std::fs::write(source.join("facts.jsonl"), facts).expect("write facts");
    let transition = |source: &str, target: &str, guidance: &str| {
        serde_json::json!({
            "source": source,
            "target": target,
            "relation": "next",
            "condition": "input is complete",
            "guidance": guidance,
            "pitfalls": ["Do not exceed review budget"],
        })
    };
    let transitions = [
        transition(
            "inspect",
            "validate",
            "Validate input carefully and verify all preconditions before proceeding to report phase",
        ),
        transition(
            "validate",
            "report",
            "Report result with full details and ensure that artifacts are properly formatted and reviewed",
        ),
        transition(
            "report",
            "archive",
            "Archive review once completed and ensure that tracking states are persistently recorded",
        ),
    ];
    let transitions_text = transitions
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(source.join("transitions.jsonl"), transitions_text).expect("write transitions");
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input(
            "graph context retrieval inspect validate question with enough extra words today",
        ),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 1);
    assert!(
        !fragments[0]
            .body()
            .contains("Procedural guidance (advisory only):")
    );
    assert!(fragments[0].render().len() <= 1_000);
}

#[test]
fn automatic_km_context_skips_fragments_requiring_review() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/large",
        "0.1.0",
        "Graph context retrieval",
        "Graph context retrieval answers should guide bounded responses",
        &"x".repeat(2_000),
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert!(fragments.is_empty());
}

#[test]
fn automatic_km_context_is_injected_once_per_thread() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");
    let thread_state = OntographThreadState::default();

    assert_eq!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .len(),
        1
    );
    assert_eq!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .len(),
        0
    );
}

// Compaction continuity (generation-scoped dedup):
// same-generation later matching turn stays ABSENCE; after the host compaction
// signal advances the generation, exactly one re-admission is allowed.
#[test]
fn automatic_km_context_readmits_once_after_compaction_generation_advance() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let first_turn =
        auto_km_turn_input("graph context retrieval question with enough extra words today");
    let later_matching_turn =
        auto_km_turn_input("bounded graph answers should guide retrieval context today");
    let thread_state = OntographThreadState::default();

    let injected = matching_km_context_fragments_for_thread(
        &first_turn,
        &store,
        AUTO_KM_TEST_SCOPE,
        &thread_state,
    );
    assert_eq!(injected.len(), 1);
    assert!(
        thread_state.auto_km_context_was_injected(
            AUTO_KM_TEST_SCOPE,
            "codebase-graph/creating-ontograph",
            "0.1.0",
        ),
        "first matching turn must consume the current-generation injection slot"
    );

    let same_generation = matching_km_context_fragments_for_thread(
        &later_matching_turn,
        &store,
        AUTO_KM_TEST_SCOPE,
        &thread_state,
    );
    assert_eq!(
        same_generation.len(),
        0,
        "same compaction generation must remain ABSENCE"
    );

    thread_state.advance_auto_km_compaction_generation();
    let after_advance = matching_km_context_fragments_for_thread(
        &later_matching_turn,
        &store,
        AUTO_KM_TEST_SCOPE,
        &thread_state,
    );
    assert_eq!(
        after_advance.len(),
        1,
        "successful compaction generation advance must re-admit exactly once"
    );
    let second_same_generation = matching_km_context_fragments_for_thread(
        &later_matching_turn,
        &store,
        AUTO_KM_TEST_SCOPE,
        &thread_state,
    );
    assert_eq!(
        second_same_generation.len(),
        0,
        "re-admission is still once per generation"
    );
}

#[tokio::test]
async fn memory_effectiveness_compaction_callback_readmits_once() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import fixture");
    let session_store = ontocode_extension_api::ExtensionData::new("evaluation-session");
    let thread_store = ontocode_extension_api::ExtensionData::new("evaluation-thread");
    let state = thread_store.get_or_init(OntographThreadState::default);
    let first =
        auto_km_turn_input("graph context retrieval question with enough extra words today");
    let reworded = auto_km_turn_input("bounded graph answers should guide retrieval context today");
    let irrelevant =
        auto_km_turn_input("sourdough baking temperature and kneading instructions today");
    assert_eq!(
        matching_km_context_fragments_for_thread(&first, &store, AUTO_KM_TEST_SCOPE, &state).len(),
        1
    );
    assert!(
        matching_km_context_fragments_for_thread(&reworded, &store, AUTO_KM_TEST_SCOPE, &state)
            .is_empty()
    );
    SurfaceScopedOntographExtension::default()
        .on_history_compacted(HistoryCompactedInput {
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;
    assert!(
        matching_km_context_fragments_for_thread(&irrelevant, &store, AUTO_KM_TEST_SCOPE, &state)
            .is_empty()
    );
    assert_eq!(
        matching_km_context_fragments_for_thread(&reworded, &store, AUTO_KM_TEST_SCOPE, &state)
            .len(),
        1
    );
    assert!(
        matching_km_context_fragments_for_thread(&reworded, &store, AUTO_KM_TEST_SCOPE, &state)
            .is_empty()
    );
}

#[test]
fn automatic_km_context_reason_is_stable_across_turn_wording() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let first = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );
    let second = matching_km_context_fragments(
        &auto_km_turn_input("bounded graph answers should guide retrieval context today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].body(), second[0].body());
}

fn auto_km_turn_input(text: &str) -> TurnInputContext {
    // Canonical project scope used by the automatic-injection tests so records
    // are eligible for injection under a matching active thread scope.
    // (ADR_ONTOGRAPH_KM_PROJECT_SCOPE)
    TurnInputContext {
        turn_id: "turn-1".to_string(),
        user_input: vec![UserInput::Text {
            text: text.to_string(),
            text_elements: Vec::new(),
        }],
        environments: Vec::new(),
    }
}

const AUTO_KM_TEST_SCOPE: &str = "/tmp/ontograph-km-test-project";

// Backlog 49: the floor is `< MIN_AUTO_KM_USER_WORDS`, so exactly eight
// distinct words is enough to inject.
#[test]
fn min_user_words_boundary_exactly_eight_injects() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    let eight = "graph retrieval alpha bravo charlie delta echo foxtrot";
    assert_eq!(eight.split_whitespace().count(), 8);
    let seven = "graph retrieval alpha bravo charlie delta echo";

    assert_eq!(
        matching_km_context_fragments(&auto_km_turn_input(eight), &store, AUTO_KM_TEST_SCOPE).len(),
        1
    );
    assert!(
        matching_km_context_fragments(&auto_km_turn_input(seven), &store, AUTO_KM_TEST_SCOPE)
            .is_empty()
    );
}

// Backlog 50: words are deduped before the floor is measured, so repeating a
// word cannot buy a turn past the minimum.
#[test]
fn duplicate_words_in_turn_count_once_toward_floor() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");

    // Twelve whitespace tokens but only six distinct words.
    let repeated = "graph retrieval context bounded answers guide \
                    graph retrieval context bounded answers guide";
    assert_eq!(repeated.split_whitespace().count(), 12);

    assert!(
        matching_km_context_fragments(&auto_km_turn_input(repeated), &store, AUTO_KM_TEST_SCOPE)
            .is_empty(),
        "duplicates must not satisfy the distinct-word floor"
    );
}

// Backlog 51: topic scoring uses binary_search over user words, which is only
// correct while that list stays sorted and deduped.
#[test]
fn topic_match_requires_binary_search_sorted_user_words() {
    let words = user_text_words(&[UserInput::Text {
        text: "zulu graph alpha retrieval graph mike".to_string(),
        text_elements: Vec::new(),
    }]);

    let mut sorted = words.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(words, sorted, "user words must stay sorted and deduped");
    assert!(words.binary_search(&"graph".to_string()).is_ok());
}

// Backlog 53: when several environments are present, the primary one decides
// the active cwd.
#[test]
fn non_primary_environment_is_ignored_when_primary_present() {
    let thread_store = ExtensionData::new("thread-1");
    let primary = std::path::PathBuf::from("/tmp/primary-cwd");
    let secondary = std::path::PathBuf::from("/tmp/secondary-cwd");

    update_current_cwd(
        &TurnInputContext {
            turn_id: "turn-1".to_string(),
            user_input: Vec::new(),
            environments: vec![
                ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "secondary".to_string(),
                    cwd: secondary,
                    is_primary: false,
                },
                ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "primary".to_string(),
                    cwd: primary.clone(),
                    is_primary: true,
                },
            ],
        },
        &thread_store,
    );

    assert_eq!(
        thread_store
            .get_or_init(OntographThreadState::default)
            .current_cwd(),
        Some(primary)
    );
}

// Backlog 54: with no environments at all the cwd is cleared, which blocks
// injection because no project scope can be resolved.
#[test]
fn missing_environment_clears_cwd_and_blocks_injection() {
    let thread_store = ExtensionData::new("thread-1");
    let thread_state = thread_store.get_or_init(OntographThreadState::default);
    thread_state.set_current_cwd(std::path::PathBuf::from("/tmp/previous-cwd"));

    update_current_cwd(
        &TurnInputContext {
            turn_id: "turn-1".to_string(),
            user_input: Vec::new(),
            environments: Vec::new(),
        },
        &thread_store,
    );

    assert_eq!(thread_state.current_cwd(), None);
}

// Backlog 55: a corrupt index during turn input yields no fragments instead of
// propagating an error into the turn.
#[test]
fn store_list_error_yields_no_fragments_not_panic() {
    let dir = tempdir().expect("temp dir");
    let cache_root = dir.path().join("km-cache");
    let store = KmStore::new(&cache_root);
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    std::fs::write(cache_root.join("index.json"), "{ not json").expect("corrupt index");

    assert!(
        matching_km_context_fragments(
            &auto_km_turn_input("graph context retrieval question with enough extra words today"),
            &store,
            AUTO_KM_TEST_SCOPE,
        )
        .is_empty()
    );
}

// Backlog 56: a record whose cache no longer matches its hash is skipped and
// the next ranked match is used instead.
#[test]
fn record_resolving_to_error_is_skipped_and_next_match_used() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let tampered = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/aaa-tampered",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    let healthy = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/zzz-healthy",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    let tampered_record = store
        .import_for_project(&tampered, AUTO_KM_TEST_SCOPE)
        .expect("import tampered");
    store
        .import_for_project(&healthy, AUTO_KM_TEST_SCOPE)
        .expect("import healthy");
    std::fs::write(Path::new(&tampered_record.path).join("facts.jsonl"), "{}\n")
        .expect("tamper cache");

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), 1, "only the healthy record is injected");
}

// Backlog 58: at most three fragments are injected, taken in rank order.
#[test]
fn three_slots_filled_from_five_matches_by_rank_order() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    for index in 0..5 {
        let source = write_auto_km_for_test(
            dir.path(),
            &format!("codebase-graph/module-{index}"),
            "0.1.0",
            "Bounded graph context",
            "Graph retrieval context should guide bounded answers",
            "example",
        );
        store
            .import_for_project(&source, AUTO_KM_TEST_SCOPE)
            .expect("import");
    }

    let fragments = matching_km_context_fragments(
        &auto_km_turn_input("graph context retrieval question with enough extra words today"),
        &store,
        AUTO_KM_TEST_SCOPE,
    );

    assert_eq!(fragments.len(), MAX_AUTO_KM_CONTEXTS);
}

// Backlog 59: the per-thread dedup key includes the version, so a newly
// imported version of the same id is injected again.
#[test]
fn injection_dedup_key_includes_version() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let thread_state = OntographThreadState::default();
    let first = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&first, AUTO_KM_TEST_SCOPE)
        .expect("import v1");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    assert_eq!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .len(),
        1
    );
    assert!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .is_empty(),
        "same id and version is injected once per thread"
    );

    let second = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.2.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&second, AUTO_KM_TEST_SCOPE)
        .expect("import v2");

    assert_eq!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .len(),
        1,
        "a new version re-injects because the dedup key carries the version"
    );
}

// Backlog 60: eligibility is seeded once at thread start, so flipping the
// feature afterwards does not change this thread's admission.
#[test]
fn feature_toggle_mid_thread_is_ignored_until_next_thread_start() {
    let thread_store = ExtensionData::new("thread-1");
    thread_store.insert(AgentAuthoredKmEligibility::from_enabled(false));

    let seeded = *thread_store.get_or_init(AgentAuthoredKmEligibility::default);
    // A later toggle produces a new value; the already-seeded copy is what the
    // turn-input path uses, and it is unchanged.
    let toggled = AgentAuthoredKmEligibility::from_enabled(true);

    assert!(!seeded.enabled);
    assert!(toggled.enabled);
    assert!(
        !thread_store
            .get_or_init(AgentAuthoredKmEligibility::default)
            .enabled,
        "thread-scoped eligibility stays as seeded"
    );
}

// Backlog 61: underscores and hyphens split words, so compound tokens do not
// survive as single words.
#[test]
fn word_extraction_treats_underscores_and_hyphens_as_separators() {
    let words = user_text_words(&[UserInput::Text {
        text: "graph_retrieval bounded-context".to_string(),
        text_elements: Vec::new(),
    }]);

    assert_eq!(
        words,
        vec![
            "bounded".to_string(),
            "context".to_string(),
            "graph".to_string(),
            "retrieval".to_string(),
        ]
    );
}

// Backlog 62: a turn mixing text and non-text input scores only the text.
#[test]
fn mixed_text_and_image_input_uses_only_text_words() {
    let mixed = vec![
        UserInput::Mention {
            name: "alpha bravo charlie delta echo foxtrot golf hotel".to_string(),
            path: "app://example".to_string(),
        },
        UserInput::Text {
            text: "graph retrieval".to_string(),
            text_elements: Vec::new(),
        },
    ];

    assert_eq!(
        user_text_words(&mixed),
        vec!["graph".to_string(), "retrieval".to_string()],
        "only text input contributes words"
    );
}

// Backlog 52: moving the thread's cwd to a second project switches the active
// scope, so only the new project's records are eligible.
#[tokio::test]
async fn cwd_change_to_second_project_switches_active_scope_same_thread() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let first_dir = tempdir().expect("first temp dir");
    let second_dir = tempdir().expect("second temp dir");
    std::fs::create_dir(first_dir.path().join(".git")).expect("first git dir");
    std::fs::create_dir(second_dir.path().join(".git")).expect("second git dir");
    let first_root = std::fs::canonicalize(first_dir.path()).expect("first canonical");
    let second_root = std::fs::canonicalize(second_dir.path()).expect("second canonical");

    for (root, id) in [
        (&first_root, "codebase-graph/first-project"),
        (&second_root, "codebase-graph/second-project"),
    ] {
        let store = KmStore::for_project_path(root).expect("project store");
        let source = write_auto_km_for_test(
            root,
            id,
            "0.1.0",
            "Bounded graph context",
            "Graph retrieval context should guide bounded answers",
            "example",
        );
        store
            .import_for_project(&source, &root.display().to_string())
            .expect("import");
    }

    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-1");
    let text = "graph context retrieval question with enough extra words today";

    let contribute = async |cwd: std::path::PathBuf| {
        SurfaceScopedOntographExtension::default()
            .contribute(
                TurnInputContext {
                    turn_id: "turn-1".to_string(),
                    user_input: auto_km_turn_input(text).user_input,
                    environments: vec![ontocode_extension_api::TurnInputEnvironment {
                        environment_id: "local".to_string(),
                        cwd,
                        is_primary: true,
                    }],
                },
                &session_store,
                &thread_store,
                &ExtensionData::new("turn"),
            )
            .await
    };

    let first = contribute(first_root.clone()).await;
    assert_eq!(first.len(), 1);
    assert!(first[0].render().contains("codebase-graph/first-project"));

    // Same thread, new cwd: the second project's record becomes eligible even
    // though the first was already injected under the previous scope.
    let second = contribute(second_root.clone()).await;
    assert_eq!(second.len(), 1);
    assert!(second[0].render().contains("codebase-graph/second-project"));
}

// Backlog 57 (decision: pin current behavior): an oversized record is skipped
// without consuming one of the three slots, and it is not marked injected, so
// it is re-resolved on every later turn.
#[test]
fn skipped_oversized_record_does_not_consume_injection_slot() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    // A large example body pushes the rendered fragment past the review
    // threshold, so this record is always skipped.
    let oversized = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/aaa-oversized",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        &"x".repeat(4096),
    );
    let healthy = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/zzz-healthy",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&oversized, AUTO_KM_TEST_SCOPE)
        .expect("import oversized");
    store
        .import_for_project(&healthy, AUTO_KM_TEST_SCOPE)
        .expect("import healthy");

    let thread_state = OntographThreadState::default();
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let first =
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state);
    assert_eq!(first.len(), 1, "the healthy record still gets injected");

    // The healthy record is now deduped, but the oversized one was never
    // marked, so it is reconsidered and skipped again on the next turn.
    assert!(
        !thread_state.auto_km_context_was_injected(
            AUTO_KM_TEST_SCOPE,
            "codebase-graph/aaa-oversized",
            "0.1.0"
        ),
        "an oversized record is never marked injected"
    );
    assert!(
        matching_km_context_fragments_for_thread(&input, &store, AUTO_KM_TEST_SCOPE, &thread_state)
            .is_empty()
    );
}

// Backlog 98: the full agent loop. An authored record reaches model context by
// automatic injection, and withdrawing it stops that injection on later turns.
#[test]
fn end_to_end_author_inject_withdraw_stops_injection() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let eligibility = AgentAuthoredKmEligibility::from_enabled(true);
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    // Authored record is eligible and injects.
    assert_eq!(
        matching_km_context_fragments_with_eligibility_for_thread(
            &input,
            &store,
            AUTO_KM_TEST_SCOPE,
            eligibility,
            "thread-1",
        )
        .len(),
        1
    );

    store
        .withdraw(
            "codebase-graph/agent-authored",
            "0.1.0",
            "retracted by author".to_string(),
        )
        .expect("withdraw succeeds");

    assert!(
        matching_km_context_fragments_with_eligibility_for_thread(
            &input,
            &store,
            AUTO_KM_TEST_SCOPE,
            eligibility,
            "thread-1",
        )
        .is_empty(),
        "withdrawal must stop automatic injection"
    );
}

// Backlog 99: importing a new version supersedes the old one, and the injected
// fragment carries the new version's facts.
#[test]
fn end_to_end_import_supersede_inject_uses_new_version_facts() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let first = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&first, AUTO_KM_TEST_SCOPE)
        .expect("import v1");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let before = matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE);
    assert_eq!(before.len(), 1);
    assert!(before[0].render().contains("creating-ontograph@0.1.0"));

    let second = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/creating-ontograph",
        "0.2.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers now revised",
        "example",
    );
    store
        .import_for_project(&second, AUTO_KM_TEST_SCOPE)
        .expect("import v2");

    let after = matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE);
    assert_eq!(after.len(), 1, "only the active version is injected");
    let rendered = after[0].render();
    assert!(rendered.contains("creating-ontograph@0.2.0"));
    assert!(rendered.contains("now revised"));
}

fn write_auto_km_for_test(
    root: &Path,
    id: &str,
    version: &str,
    summary: &str,
    claim: &str,
    example: &str,
) -> std::path::PathBuf {
    let km_root = root.join(format!("auto-km-{}-{version}", id.replace('/', "-")));
    std::fs::create_dir_all(km_root.join("examples")).expect("create examples");
    std::fs::create_dir_all(km_root.join("tests")).expect("create tests");
    let source = if example.len() > 1024 {
        example
    } else {
        "silver:doc"
    };
    std::fs::write(
        km_root.join("km.toml"),
        format!(
            "id = \"{id}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"{summary}\"\nsources = [\"{source}\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .expect("write manifest");
    std::fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("write facts");
    std::fs::write(km_root.join("examples/README.md"), example).expect("write example");
    std::fs::write(km_root.join("tests/README.md"), "test").expect("write test");
    // `silver:doc` is validated against the project root, so the cited file has
    // to exist there for import to accept the package.
    std::fs::write(root.join("doc"), "doc").expect("write cited source");
    km_root
}

/// Imports one agent-authored record and returns the store plus its temp dir.
fn agent_authored_store(dir: &Path) -> KmStore {
    let store = KmStore::new(dir.join("km-cache"));
    let source = write_auto_km_for_test(
        dir,
        "codebase-graph/agent-authored",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project_with_origin(
            &source,
            AUTO_KM_TEST_SCOPE,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            },
        )
        .expect("import agent record");
    store
}

#[test]
fn feature_off_excludes_agent_origin_records_from_automatic_injection() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let disabled = matching_km_context_fragments_with_eligibility(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(false),
    );
    assert_eq!(disabled.len(), 0);

    // The same record is admitted once the feature is enabled, so the empty
    // result above is caused by the gate and not by a non-matching fixture.
    let enabled = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-1",
    );
    assert_eq!(enabled.len(), 1);
    assert!(enabled[0].body().contains("codebase-graph/agent-authored"));
}

#[test]
fn feature_off_default_matches_the_disabled_gate() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    // Absent thread lifecycle state must fail closed exactly like an explicit
    // disabled gate, so a missing on_thread_start cannot admit agent content.
    assert_eq!(
        matching_km_context_fragments(&input, &store, AUTO_KM_TEST_SCOPE).len(),
        0
    );
}

#[test]
fn withdrawn_records_are_excluded_whether_the_feature_is_on_or_off() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/withdrawn-record",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    // Before withdrawal the record injects, which proves the fixture matches.
    assert_eq!(
        matching_km_context_fragments_with_eligibility(
            &input,
            &store,
            AUTO_KM_TEST_SCOPE,
            AgentAuthoredKmEligibility::from_enabled(true),
        )
        .len(),
        1
    );

    store
        .withdraw(
            "codebase-graph/withdrawn-record",
            "0.1.0",
            "policy violation".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdrawn record");

    for enabled in [false, true] {
        assert_eq!(
            matching_km_context_fragments_with_eligibility(
                &input,
                &store,
                AUTO_KM_TEST_SCOPE,
                AgentAuthoredKmEligibility::from_enabled(enabled),
            )
            .len(),
            0
        );
    }
}

#[test]
fn feature_off_leaves_human_origin_injection_unchanged() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/human-authored",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, AUTO_KM_TEST_SCOPE)
        .expect("import");
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let disabled = matching_km_context_fragments_with_eligibility(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(false),
    );
    let enabled = matching_km_context_fragments_with_eligibility(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
    );

    assert_eq!(disabled.len(), 1);
    // Human KM rendering is byte-identical in both feature states.
    assert_eq!(disabled[0].body(), enabled[0].body());
    assert_eq!(disabled[0].role(), enabled[0].role());
}

#[test]
fn agent_authored_context_stays_user_role_quoted_data() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let fragments = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-1",
    );

    assert_eq!(fragments.len(), 1);
    // Agent-authored content must never become developer policy.
    assert_eq!(fragments[0].role(), "user");
    let (start, end) = fragments[0].markers();
    assert!(start.contains("ontograph_km_context"));
    assert!(end.contains("ontograph_km_context"));
    // The agent origin is disclosed inside the quoted user data.
    assert!(fragments[0].body().contains("thread-1"));
}

#[test]
fn trust_agent_authored_context_is_user_role_fragment_with_km_markers() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let fragments = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-1",
    );

    assert_eq!(fragments.len(), 1);
    assert_eq!(fragments[0].role(), "user");
    assert_ne!(fragments[0].role(), "developer");
    let (start_marker, end_marker) = fragments[0].markers();
    assert!(start_marker.contains("ontograph_km_context"));
    assert!(end_marker.contains("ontograph_km_context"));
}

#[test]
fn feature_off_rollback_keeps_human_knowledge_and_excludes_agent_and_withdrawn_records() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));

    let agent_source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/agent-authored-rollback",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "agent example",
    );
    store
        .import_for_project_with_origin(
            &agent_source,
            AUTO_KM_TEST_SCOPE,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-rollback".to_string(),
            },
        )
        .expect("import agent record");

    let human_source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/human-authored-rollback",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "human example",
    );
    store
        .import_for_project(&human_source, AUTO_KM_TEST_SCOPE)
        .expect("import human record");

    let withdrawn_source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/withdrawn-rollback",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "withdrawn example",
    );
    store
        .import_for_project(&withdrawn_source, AUTO_KM_TEST_SCOPE)
        .expect("import withdrawn record");
    store
        .withdraw(
            "codebase-graph/withdrawn-rollback",
            "0.1.0",
            "stale source data".to_string(),
        )
        .expect("withdraw succeeds")
        .expect("withdrawn record");

    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let enabled = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-rollback",
    );
    assert_eq!(enabled.len(), 2);
    assert!(enabled.iter().any(|fragment| {
        fragment
            .body()
            .contains("codebase-graph/agent-authored-rollback")
    }));
    assert!(enabled.iter().any(|fragment| {
        fragment
            .body()
            .contains("codebase-graph/human-authored-rollback")
    }));

    // Mutation-style rollback check: force feature-off and prove the injected
    // payload changes, then restore feature-on and revalidate the stronger
    // injection set.
    let disabled = matching_km_context_fragments_with_eligibility(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(false),
    );
    assert_eq!(disabled.len(), 1);
    let restored = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-rollback",
    );
    assert_eq!(restored.len(), 2);

    let human_disabled_body = disabled[0].body();
    let human_restored = restored
        .iter()
        .find(|fragment| {
            fragment
                .body()
                .contains("codebase-graph/human-authored-rollback")
        })
        .expect("human fragment present after restore");
    assert_eq!(human_disabled_body, human_restored.body());

    assert!(
        !disabled[0]
            .body()
            .contains("codebase-graph/agent-authored-rollback")
    );
    assert!(!disabled.iter().any(|fragment| {
        fragment
            .body()
            .contains("codebase-graph/withdrawn-rollback")
    }));
}

#[test]
fn project_isolation_scopes_agent_records_to_import_scope_only() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/scoped-project",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "project example",
    );

    store
        .import_for_project_with_origin(
            &source,
            "/tmp/project-scope-a",
            KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            },
        )
        .expect("import agent project-scoped record");

    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let scoped_injections = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        "/tmp/project-scope-a",
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-1",
    );
    let unrelated_injections = matching_km_context_fragments_with_eligibility(
        &input,
        &store,
        "/tmp/project-scope-b",
        AgentAuthoredKmEligibility::from_enabled(true),
    );

    assert_eq!(scoped_injections.len(), 1);
    assert!(
        scoped_injections[0]
            .body()
            .contains("codebase-graph/scoped-project")
    );
    assert_eq!(unrelated_injections.len(), 0);
}

#[test]
fn automatic_km_context_hides_another_threads_agent_authored_record() {
    let dir = tempdir().expect("temp dir");
    let store = agent_authored_store(dir.path());
    let input =
        auto_km_turn_input("graph context retrieval question with enough extra words today");

    let fragments = matching_km_context_fragments_with_eligibility_for_thread(
        &input,
        &store,
        AUTO_KM_TEST_SCOPE,
        AgentAuthoredKmEligibility::from_enabled(true),
        "thread-2",
    );

    assert_eq!(fragments.len(), 0);
}

#[tokio::test]
async fn negative_knowledge_injection_asserts_no_skill_loader_artifacts_are_written() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let project_root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let project_scope = project_root.display().to_string();
    let store = KmStore::for_project_path(&project_root).expect("project store");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/agent-artifact-safe",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project_with_origin(
            &source,
            &project_scope,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            },
        )
        .expect("import agent record");

    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-1");
    let mut config =
        ontocode_core::config::Config::load_default_with_cli_overrides_for_ontocode_home(
            dir.path().join("ontocode-home"),
            vec![],
        )
        .await
        .expect("load config");
    let _ = config
        .features
        .enable(ontocode_features::Feature::AgentAuthoredProjectKnowledge);
    let session_source = Default::default();
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install(&mut builder);
    let registry = builder.build();
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ontocode_extension_api::ThreadStartInput {
            config: &config,
            session_source: &session_source,
            persistent_thread_state_available: true,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;

    let fragments = registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: auto_km_turn_input(
                    "graph context retrieval question with enough extra words today",
                )
                .user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: project_root.clone(),
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await;

    assert_eq!(fragments.len(), 1);

    assert_no_skill_loader_artifacts(&project_root);
}

#[test]
fn canonical_store_and_modules_root_are_under_project_dot_ontocode() {
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("project git");
    let project_root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let project_scope = project_root.display().to_string();
    let expected_store = project_root.join(".ontocode/ontograph/km/_store");
    let expected_modules = expected_store.join("modules");

    let store = KmStore::for_project_path(&project_root).expect("project store");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/canonical-storage",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project(&source, &project_scope)
        .expect("import canonical-scoped record");

    let canonical_store = std::fs::canonicalize(&expected_store).expect("canonical store path");
    assert!(canonical_store.ends_with(".ontocode/ontograph/km/_store"));
    assert_eq!(canonical_store, expected_store.join("."));
    assert!(expected_modules.is_dir());
    assert!(
        expected_modules
            .join("codebase-graph/canonical-storage/0.1.0")
            .is_dir()
    );

    let record = store
        .resolve_active("codebase-graph/canonical-storage")
        .expect("resolve active")
        .expect("active module");
    assert_eq!(
        record.path,
        expected_modules
            .join("codebase-graph/canonical-storage/0.1.0")
            .to_string_lossy()
    );
}

fn assert_no_skill_loader_artifacts(root: &Path) {
    let mut frontier = vec![root.to_path_buf()];
    while let Some(current) = frontier.pop() {
        let entries = std::fs::read_dir(&current)
            .expect("read project directory")
            .collect::<Result<Vec<_>, _>>()
            .expect("read dir entries");
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            assert_ne!(
                name, "SKILL.md",
                "SKILL.md must not be written under the project"
            );

            if path.is_dir() {
                if path
                    .strip_prefix(root)
                    .map(|relative| {
                        let mut segments = relative.iter();
                        matches!(
                            (segments.next(), segments.next()),
                            (Some(first), Some(second))
                                if (first == std::ffi::OsStr::new(".ontocode")
                                    && second == std::ffi::OsStr::new("skills"))
                                    ||
                                    (first == std::ffi::OsStr::new(".agents")
                                        && second == std::ffi::OsStr::new("skills"))
                        )
                    })
                    .unwrap_or(false)
                {
                    panic!("skill loader artifact directory must not be created");
                }
                frontier.push(path);
            }
        }
    }
}

/// Drives the real host path: `on_thread_start` seeds eligibility into the same
/// thread store that `contribute` later reads. This fails if the lifecycle
/// contributor is unregistered, reads the wrong feature, or stops persisting
/// state, none of which the eligibility-struct tests can detect.
async fn agent_fragments_via_host_path(feature_enabled: bool) -> usize {
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let project_root = std::fs::canonicalize(dir.path()).expect("canonical project root");
    let store = KmStore::for_project_path(&project_root).expect("project store");
    let source = write_auto_km_for_test(
        dir.path(),
        "codebase-graph/host-path-agent",
        "0.1.0",
        "Bounded graph context",
        "Graph retrieval context should guide bounded answers",
        "example",
    );
    store
        .import_for_project_with_origin(
            &source,
            &project_root.display().to_string(),
            KmIndexOrigin::AgentThread {
                thread_id: "thread-1".to_string(),
            },
        )
        .expect("import agent record");

    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread-1");
    let config_home = dir.path().join("ontocode-home");
    std::fs::create_dir_all(&config_home).expect("config home");
    let mut config =
        ontocode_core::config::Config::load_default_with_cli_overrides_for_ontocode_home(
            config_home,
            vec![],
        )
        .await
        .expect("load config");
    if feature_enabled {
        let _ = config
            .features
            .enable(ontocode_features::Feature::AgentAuthoredProjectKnowledge);
    }
    let session_source = Default::default();
    // Route through the registry so a missing `install` registration fails this
    // test instead of silently disabling agent retrieval in production.
    let mut builder = ontocode_extension_api::ExtensionRegistryBuilder::new();
    install(&mut builder);
    let registry = builder.build();
    registry.thread_lifecycle_contributors()[0]
        .on_thread_start(ontocode_extension_api::ThreadStartInput {
            config: &config,
            session_source: &session_source,
            persistent_thread_state_available: true,
            session_store: &session_store,
            thread_store: &thread_store,
        })
        .await;

    registry.turn_input_contributors()[0]
        .contribute(
            TurnInputContext {
                turn_id: "turn-1".to_string(),
                user_input: auto_km_turn_input(
                    "graph context retrieval question with enough extra words today",
                )
                .user_input,
                environments: vec![ontocode_extension_api::TurnInputEnvironment {
                    environment_id: "local".to_string(),
                    cwd: project_root,
                    is_primary: true,
                }],
            },
            &session_store,
            &thread_store,
            &ExtensionData::new("turn"),
        )
        .await
        .len()
}

#[tokio::test]
async fn host_lifecycle_seeding_gates_agent_records_on_the_contribute_path() {
    let _serial = STAGE_A_TEST_SERIAL
        .acquire()
        .await
        .expect("stage a serial semaphore is never closed");
    assert_eq!(
        agent_fragments_via_host_path(/*feature_enabled*/ false).await,
        0
    );
    assert_eq!(
        agent_fragments_via_host_path(/*feature_enabled*/ true).await,
        1
    );
}

fn thread_with_record_dir(dir: &Path) -> ExtensionData {
    let thread = ExtensionData::new("thread-record");
    thread.insert(AdmissionRecordSink::from_dir(Some(dir.to_path_buf())));
    thread
}

fn read_admission_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("lineage_admission.jsonl"))
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn record_admission_writes_one_line_for_an_accepted_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let thread = thread_with_record_dir(dir.path());
    let turn = turn_store_with_selection();
    turn.insert(LineageAdmissionResult(Ok(())));

    record_admission(&thread, &turn, /*stage_a_elapsed_ms*/ 7);
    record_admission(&thread, &turn, /*stage_a_elapsed_ms*/ 7);
    // The write runs on the blocking pool; wait for it to finish.
    for _ in 0..200 {
        if !read_admission_lines(dir.path()).is_empty() {
            break;
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert_eq!(
        read_admission_lines(dir.path()),
        vec![format!(
            "{{\"turn_id\":\"{}\",\"outcome\":\"lineage\",\"admission\":\"accepted\",\"fallback_category\":\"none\",\"stage_a_elapsed_ms\":7}}",
            turn.level_id()
        )]
    );
}

#[test]
fn record_admission_is_silent_without_a_sink_or_an_unresolved_selection() {
    let dir = tempfile::tempdir().expect("tempdir");

    // No sink on the thread.
    let turn = turn_store_with_selection();
    turn.insert(LineageAdmissionResult(Ok(())));
    record_admission(
        &ExtensionData::new("thread-none"),
        &turn,
        /*stage_a_elapsed_ms*/ 7,
    );
    assert!(turn.get::<LineageRecordAttempted>().is_none());

    // A disabled sink on the thread.
    let thread = ExtensionData::new("thread-off");
    thread.insert(AdmissionRecordSink::default());
    record_admission(&thread, &turn, /*stage_a_elapsed_ms*/ 7);
    assert!(turn.get::<LineageRecordAttempted>().is_none());

    // Selection with no admission result is unresolved and must not consume the guard.
    let unresolved = turn_store_with_selection();
    record_admission(
        &thread_with_record_dir(dir.path()),
        &unresolved,
        /*stage_a_elapsed_ms*/ 7,
    );
    assert!(unresolved.get::<LineageRecordAttempted>().is_none());
    assert_eq!(read_admission_lines(dir.path()), Vec::<String>::new());
}
