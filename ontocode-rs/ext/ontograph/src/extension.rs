use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use ontocode_core::config::Config;
use ontocode_extension_api::ContextualUserFragment;
use ontocode_extension_api::ExtensionData;
use ontocode_extension_api::HistoryCompactedInput;
use ontocode_extension_api::ThreadLifecycleContributor;
use ontocode_extension_api::ThreadStartInput;
use ontocode_extension_api::ToolCall;
use ontocode_extension_api::ToolContributor;
use ontocode_extension_api::ToolExecutor;
use ontocode_extension_api::TurnDeadline;
use ontocode_extension_api::TurnInputContext;
use ontocode_extension_api::TurnInputContributor;
use ontocode_extension_api::TurnItemContributor;
use ontocode_otel::MetricsClient;
use ontocode_protocol::items::AgentMessageContent;
use ontocode_protocol::items::TurnItem;
use ontocode_protocol::user_input::UserInput;

use crate::audit_build::AuditTool;
use crate::km::load_transitions;
use crate::km_context::render_km_context;
use crate::km_context::render_km_context_with_procedural_guidance;
use crate::km_resolve::resolve_active_km;
use crate::km_store::KmIndexOrigin;
use crate::km_store::KmIndexRecord;
use crate::km_store::KmStore;
use crate::lineage_admission::AdmissionRejection;
use crate::lineage_admission::LineageContextFragment;
use crate::lineage_admission::admit_lineage;
use crate::lineage_admission_record::AdmissionRecord;
use crate::lineage_admission_record::AdmissionRecordSink;
use crate::lineage_grounding::GroundingClaim;
use crate::lineage_grounding::SourceGeneration;
use crate::lineage_grounding::ground_claims;
use crate::lineage_km_retrieval::km_lineage_attempt;
use crate::lineage_km_retrieval::refresh_selected;
use crate::lineage_records::LineageLedger;
use crate::lineage_routing::BaselineOutcome;
use crate::lineage_routing::FallbackReason;
use crate::lineage_routing::LineageAttempt;
use crate::lineage_routing::LineageBudget;
use crate::lineage_routing::RoutingConfig;
use crate::lineage_routing::RoutingDecision;
use crate::lineage_routing::induce_baseline_outcome;
use crate::lineage_routing::is_eligible;
use crate::lineage_routing::route;
use crate::procedural_graph::MAX_REQUESTED_HOPS;
use crate::procedural_graph::ProceduralGraph;
use crate::procedural_graph::ProceduralLookup;
use crate::tool::DiscoverTool;
use crate::tool::DocsTool;
use crate::tool::EvidencePlannerTool;
use crate::tool::ExplainModuleTool;
use crate::tool::ImpactTool;
use crate::tool::InspectTool;
use crate::tool::KmContextTool;
use crate::tool::ManageTool;
use crate::tool::OntographThreadState;
use crate::tool::SearchTool;

#[cfg(test)]
#[path = "extension_tests.rs"]
mod tests;

const MIN_AUTO_KM_USER_WORDS: usize = 8;
const MIN_AUTO_KM_TOPIC_MATCHES: usize = 2;
const MAX_AUTO_KM_CONTEXTS: usize = 3;
const AUTO_KM_CONTEXT_REASON: &str = "automatic topic-word match";

/// Thread-scoped eligibility for agent-authored KM records.
///
/// Seeded once per thread from host config so the turn-input path, which has no
/// config access, can still fail closed when the feature is disabled.
#[derive(Clone, Copy, Default)]
pub(crate) struct AgentAuthoredKmEligibility {
    enabled: bool,
}

impl AgentAuthoredKmEligibility {
    pub(crate) fn from_enabled(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Agent-authored and withdrawn records are ineligible for automatic
    /// injection unless the feature is explicitly enabled. Absent thread state
    /// means disabled, so a missing lifecycle gate cannot admit agent content.
    fn admits(self, record: &KmIndexRecord) -> bool {
        if self.enabled {
            return record.withdrawal.is_none();
        }
        record.withdrawal.is_none()
            && !matches!(record.origin, Some(KmIndexOrigin::AgentThread { .. }))
    }
}

/// Thread-scoped eligibility for the opt-in lineage routing path.
///
/// Seeded once per thread from host config so the turn-input path, which has no
/// config access, can fail closed when the feature is disabled. Absent thread
/// state means disabled, so a missing lifecycle gate cannot reach lineage work.
#[derive(Clone, Copy, Default)]
pub(crate) struct LineageRoutingEligibility {
    enabled: bool,
    /// Run Stage A and record its outcome without changing the request.
    shadow: bool,
}

impl LineageRoutingEligibility {
    #[cfg(test)]
    pub(crate) fn from_enabled(enabled: bool) -> Self {
        Self::from_flags(enabled, /*shadow*/ false)
    }

    pub(crate) fn from_flags(enabled: bool, shadow: bool) -> Self {
        Self { enabled, shadow }
    }

    /// True when only the shadow flag is on: Stage A runs, nothing is injected.
    pub(crate) fn shadow_only(self) -> bool {
        self.shadow && !self.enabled
    }

    pub(crate) fn routing_config(self) -> RoutingConfig {
        RoutingConfig {
            enabled: self.enabled || self.shadow,
            ..RoutingConfig::default()
        }
    }
}

#[derive(Default)]
pub(crate) struct LineageBreaker {
    consecutive_failures: std::sync::atomic::AtomicU32,
}

impl LineageBreaker {
    const TRIP_AFTER: u32 = 3;

    fn tripped(&self) -> bool {
        self.consecutive_failures.load(Ordering::Relaxed) >= Self::TRIP_AFTER
    }

    fn record_success(&self) {
        self.consecutive_failures
            .store(/*val*/ 0, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        let _ = self.consecutive_failures.fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |failures| Some(failures.saturating_add(/*rhs*/ 1)),
        );
    }
}

#[derive(Clone, Default)]
pub(crate) struct SurfaceScopedOntographExtension {
    metrics_client: Option<MetricsClient>,
}

pub(crate) struct LineageRetrievalScope(PathBuf);

/// Typed outcome of this turn's Stage A admission, read by LRA-0.5 telemetry.
pub(crate) struct LineageAdmissionResult(pub(crate) Result<(), AdmissionRejection>);

pub(crate) struct LineageMetricAttempted;

pub(crate) struct LineageRecordAttempted;

/// One in-flight Stage A worker per thread, including workers outliving a turn.
pub(crate) struct StageABusy(std::sync::atomic::AtomicBool);

/// Test-only delay/panic seam, invoked in the blocking worker before retrieval.
/// Tests setting this global hook must serialize access and clear it afterwards.
#[cfg(test)]
pub(crate) static STAGE_A_TEST_HOOK: std::sync::Mutex<Option<Box<dyn Fn() + Send + Sync>>> =
    std::sync::Mutex::new(/*t*/ None);

impl ToolContributor for SurfaceScopedOntographExtension {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        let thread_state = thread_store.get_or_init(OntographThreadState::default);
        vec![
            Arc::new(ExplainModuleTool::new(thread_state.clone())),
            Arc::new(DiscoverTool::new(thread_state.clone())),
            Arc::new(DocsTool::new(thread_state.clone())),
            Arc::new(AuditTool::new(thread_state.clone())),
            Arc::new(ImpactTool::new(thread_state.clone())),
            Arc::new(InspectTool::new(thread_state.clone())),
            Arc::new(KmContextTool::new(thread_state.clone())),
            Arc::new(ManageTool::new(thread_state.clone())),
            Arc::new(SearchTool::new(thread_state.clone())),
            Arc::new(EvidencePlannerTool::new(thread_state)),
        ]
    }
}

#[async_trait::async_trait]
impl TurnInputContributor for SurfaceScopedOntographExtension {
    async fn contribute(
        &self,
        input: TurnInputContext,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
        turn_store: &ExtensionData,
    ) -> Vec<Box<dyn ContextualUserFragment + Send>> {
        turn_store.remove::<LineageRetrievalScope>();
        turn_store.remove::<LineageAdmissionResult>();
        turn_store.remove::<LineageMetricAttempted>();
        let mut fragments = update_current_cwd(&input, thread_store);
        let mut stage_a_failed = false;
        let breaker = thread_store.get_or_init(LineageBreaker::default);
        let shadow_only = thread_store
            .get_or_init(LineageRoutingEligibility::default)
            .shadow_only();
        let counter = if shadow_only {
            SHADOW_COUNTER
        } else {
            LIVE_COUNTER
        };
        // The baseline outcome must exist before any lineage work is considered.
        // Induction is bounded and may fail on large or non-text turns; that only
        // skips the lineage path and must never suppress KM context below.
        let stage_a_started = std::time::Instant::now();
        match induce_baseline_outcome(&input.user_input) {
            Ok(baseline) => {
                let routing_eligibility =
                    *thread_store.get_or_init(LineageRoutingEligibility::default);
                let config = routing_eligibility.routing_config();
                let budget = turn_lineage_budget(turn_store);
                if !config.enabled
                    || !is_eligible(config, &baseline)
                    || budget.remaining_ms == 0
                    || config.max_latency_ms == 0
                    || breaker.tripped()
                {
                    record_lineage_routing_decision(
                        &baseline,
                        thread_store,
                        turn_store,
                        budget,
                        || LineageAttempt::RetrievalUnavailable("stage_a_gated".to_string()),
                    );
                } else {
                    stage_a_failed = true;
                    let thread_state = thread_store.get_or_init(OntographThreadState::default);
                    let captured_cwd = thread_state.current_cwd();
                    let private_thread = ExtensionData::new(thread_store.level_id());
                    private_thread.insert(routing_eligibility);
                    private_thread
                        .insert(*thread_store.get_or_init(AgentAuthoredKmEligibility::default));
                    let private_state = OntographThreadState::default();
                    if let Some(cwd) = &captured_cwd {
                        private_state.set_current_cwd(cwd.clone());
                    }
                    private_thread.insert(private_state);
                    let private_turn = ExtensionData::new(turn_store.level_id());
                    if let Some(deadline) = turn_store.get::<TurnDeadline>() {
                        private_turn.insert(*deadline);
                    }
                    let user_input = input.user_input.clone();
                    let started_at = Instant::now();
                    let mut stage_a_succeeded = false;
                    if let Some((private_turn, fragment)) =
                        wait_stage_a(thread_store, turn_store, config, move || {
                            record_lineage_routing_decision(
                                &baseline,
                                &private_thread,
                                &private_turn,
                                turn_lineage_budget(&private_turn),
                                || {
                                    host_lineage_attempt(&user_input, &private_thread, |scope| {
                                        private_turn
                                            .insert(LineageRetrievalScope(scope.to_path_buf()));
                                    })
                                },
                            );
                            let retrieval_scope = private_turn.get::<LineageRetrievalScope>();
                            let fragment = admit_selected_turn(
                                &private_thread,
                                &private_turn,
                                retrieval_scope.as_ref().map(|scope| scope.0.as_path()),
                            );
                            drop(retrieval_scope);
                            (private_turn, fragment)
                        })
                        .await
                    {
                        if thread_state.current_cwd() != captured_cwd {
                            turn_store.insert(LineageTurnDecision {
                                decision: RoutingDecision::Baseline {
                                    reason: FallbackReason::RetrievalFailed(
                                        "scope_changed".to_string(),
                                    ),
                                },
                            });
                        } else {
                            // Unwrap all private state before publishing any of it.
                            let decision = private_turn
                                .remove::<LineageTurnDecision>()
                                .map(Arc::try_unwrap)
                                .transpose();
                            let scope = private_turn
                                .remove::<LineageRetrievalScope>()
                                .map(Arc::try_unwrap)
                                .transpose();
                            let admission = private_turn
                                .remove::<LineageAdmissionResult>()
                                .map(Arc::try_unwrap)
                                .transpose();
                            if let (Ok(Some(mut decision)), Ok(scope), Ok(admission)) =
                                (decision, scope, admission)
                            {
                                stage_a_failed = false;
                                if let RoutingDecision::Selected { cost, .. } =
                                    &mut decision.decision
                                {
                                    cost.elapsed_ms =
                                        started_at.elapsed().as_millis().min(u128::from(u64::MAX))
                                            as u64;
                                }
                                turn_store.insert(decision);
                                if let Some(scope) = scope {
                                    turn_store.insert(scope);
                                }
                                if let Some(admission) = admission {
                                    turn_store.insert(admission);
                                }
                                stage_a_succeeded = fragment.is_some();
                                if stage_a_succeeded {
                                    breaker.record_success();
                                }
                                // Shadow-only never injects: it falls through to the
                                // common emit and the baseline automatic-KM path.
                                if let Some(fragment) = fragment
                                    && !shadow_only
                                {
                                    fragments.push(Box::new(fragment));
                                    emit_stage_a_as(
                                        self.metrics_client.as_ref(),
                                        turn_store,
                                        counter,
                                    );
                                    record_admission(
                                        thread_store,
                                        turn_store,
                                        stage_a_started
                                            .elapsed()
                                            .as_millis()
                                            .min(u128::from(u64::MAX))
                                            as u64,
                                    );
                                    return fragments;
                                }
                            } else {
                                turn_store.insert(LineageTurnDecision {
                                    decision: RoutingDecision::Baseline {
                                        reason: FallbackReason::RetrievalFailed(
                                            "stage_a_state_shared".to_string(),
                                        ),
                                    },
                                });
                            }
                        }
                    }
                    if !stage_a_succeeded {
                        breaker.record_failure();
                    }
                }
            }
            Err(reason) => {
                // `insert` returns the replaced entry; the recording arm above
                // yields `()`, so the displaced value is discarded to keep both
                // match arms at the same type.
                turn_store.insert(LineageTurnDecision {
                    decision: RoutingDecision::Baseline {
                        reason: FallbackReason::InductionFailed(reason),
                    },
                });
            }
        }
        emit_stage_a_as(self.metrics_client.as_ref(), turn_store, counter);
        record_admission(
            thread_store,
            turn_store,
            stage_a_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        );
        if stage_a_failed && !shadow_only {
            // Fail closed: do not start synchronous KM I/O after Stage A fails.
            return fragments;
        }
        let user_words = user_text_words(&input.user_input);
        let procedural_identifiers = user_text_identifiers(&input.user_input);
        if user_words.len() < MIN_AUTO_KM_USER_WORDS {
            return fragments;
        }
        let thread_state = thread_store.get_or_init(OntographThreadState::default);
        // Cross-project isolation (ADR_ONTOGRAPH_KM_PROJECT_SCOPE): automatic
        // injection is limited to KM records whose project scope matches the
        // active thread's canonical Git root. Without a resolvable active
        // scope, no record is auto-injected.
        let Some(current_cwd) = thread_state.current_cwd() else {
            return fragments;
        };
        let Some(project_root) = crate::km_store::project_root(&current_cwd) else {
            return fragments;
        };
        let active_scope = project_root.display().to_string();
        let Some(store) = KmStore::for_project_path(&project_root) else {
            return fragments;
        };
        let eligibility = *thread_store.get_or_init(AgentAuthoredKmEligibility::default);
        let active_thread_id = thread_store.level_id();
        fragments.extend(matching_km_context_fragments_for_thread_words(
            &user_words,
            &active_scope,
            &store,
            &thread_state,
            eligibility,
            Some(active_thread_id),
            &procedural_identifiers,
        ));
        fragments
    }
}

/// Fallback latency budget when the host published no turn deadline.
///
/// Used only when `TurnDeadline` is absent from turn-scoped data, which means
/// the turn start was never marked and elapsed time cannot be measured. It
/// equals the default `RoutingConfig::max_latency_ms`; Stage A uses the smaller
/// of this allowance and the configured cap.
const TURN_LINEAGE_BUDGET_MS: u64 = 10_000;

/// Budget for this turn's lineage attempt, measured against the host deadline.
///
/// A deadline already elapsed yields a zero budget, so `route` records
/// `BudgetExhausted` instead of starting retrieval late in a turn. Absence of
/// the deadline is fail-closed to the fixed allowance rather than unlimited.
fn turn_lineage_budget(turn_store: &ExtensionData) -> LineageBudget {
    let remaining_ms = turn_store
        .get::<TurnDeadline>()
        .map_or(TURN_LINEAGE_BUDGET_MS, |deadline| deadline.remaining_ms());
    LineageBudget::from_remaining_ms(remaining_ms)
}

/// Waits only for the remaining Stage A allowance; the worker owns private data.
/// Aborting a running blocking job cannot stop it, so its busy guard stays held
/// until the closure actually exits, including during panic unwinding.
async fn wait_stage_a<T: Send + 'static>(
    thread: &ExtensionData,
    turn: &ExtensionData,
    config: RoutingConfig,
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    struct WorkerGuard(Arc<StageABusy>);

    impl Drop for WorkerGuard {
        fn drop(&mut self) {
            self.0.0.store(/*val*/ false, Ordering::Release);
        }
    }

    struct WaitGuard<'a> {
        turn: &'a ExtensionData,
        breaker: Arc<LineageBreaker>,
        handle: tokio::task::AbortHandle,
        pending: bool,
    }

    impl Drop for WaitGuard<'_> {
        fn drop(&mut self) {
            if self.pending {
                self.handle.abort();
                self.breaker.record_failure();
                self.turn.insert(LineageTurnDecision {
                    decision: RoutingDecision::Baseline {
                        reason: FallbackReason::RetrievalFailed("stage_a_cancelled".to_string()),
                    },
                });
            }
        }
    }

    let busy = thread.get_or_init(|| {
        StageABusy(std::sync::atomic::AtomicBool::new(/*v*/ false))
    });
    if busy
        .0
        .compare_exchange(
            /*current*/ false,
            /*new*/ true,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        turn.insert(LineageTurnDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::RetrievalFailed("stage_a_busy".to_string()),
            },
        });
        return None;
    }
    let worker_guard = WorkerGuard(busy);
    let started_at = tokio::time::Instant::now();
    let allowance = turn
        .get::<TurnDeadline>()
        .map_or(Duration::from_millis(TURN_LINEAGE_BUDGET_MS), |deadline| {
            deadline.remaining()
        })
        .min(Duration::from_millis(config.max_latency_ms));
    let deadline = started_at + allowance;
    let limit_ms = allowance.as_millis().min(u128::from(u64::MAX)) as u64;
    if allowance.is_zero() {
        turn.insert(LineageTurnDecision {
            decision: RoutingDecision::Baseline {
                reason: FallbackReason::Timeout {
                    elapsed_ms: 0,
                    limit_ms,
                },
            },
        });
        return None;
    }
    let mut handle = tokio::task::spawn_blocking(move || {
        let _worker_guard = worker_guard;
        work()
    });
    let mut wait_guard = WaitGuard {
        turn,
        breaker: thread.get_or_init(LineageBreaker::default),
        handle: handle.abort_handle(),
        pending: true,
    };
    let result = tokio::time::timeout_at(deadline, &mut handle).await;
    wait_guard.pending = false;
    match result {
        Ok(Ok(value)) if tokio::time::Instant::now() < deadline => Some(value),
        Ok(Ok(_)) | Err(_) => {
            handle.abort();
            turn.insert(LineageTurnDecision {
                decision: RoutingDecision::Baseline {
                    reason: FallbackReason::Timeout {
                        elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX))
                            as u64,
                        limit_ms,
                    },
                },
            });
            None
        }
        Ok(Err(_)) => {
            turn.insert(LineageTurnDecision {
                decision: RoutingDecision::Baseline {
                    reason: FallbackReason::RetrievalFailed("stage_a_panic".to_string()),
                },
            });
            None
        }
    }
}

/// The lineage attempt available on this host.
///
/// Retrieval is KM-backed and resolves through the same project-scoped store
/// the automatic KM path uses, so lineage never reaches records the active
/// scope would not already admit. Each failure to resolve that scope is a
/// recorded fallback reason rather than an empty success.
fn host_lineage_attempt(
    user_input: &[UserInput],
    thread_store: &ExtensionData,
    capture_scope: impl FnOnce(&Path),
) -> LineageAttempt {
    #[cfg(test)]
    {
        let hook = match STAGE_A_TEST_HOOK.lock() {
            Ok(hook) => hook,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some(hook) = hook.as_ref() {
            hook();
        }
    }
    let thread_state = thread_store.get_or_init(OntographThreadState::default);
    let Some(current_cwd) = thread_state.current_cwd() else {
        return LineageAttempt::RetrievalUnavailable(
            "no active working directory for km lineage retrieval".to_string(),
        );
    };
    let Some(project_root) = crate::km_store::project_root(&current_cwd) else {
        return LineageAttempt::RetrievalUnavailable(
            "active working directory is outside a resolvable project root".to_string(),
        );
    };
    let Some(store) = KmStore::for_project_path(&project_root) else {
        return LineageAttempt::RetrievalUnavailable(
            "no km store is available for the active project".to_string(),
        );
    };
    let question = user_input
        .iter()
        .filter_map(|item| match item {
            UserInput::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    capture_scope(&project_root);
    km_lineage_attempt(&store, &question)
}

fn admit_selected_turn(
    thread: &ExtensionData,
    turn: &ExtensionData,
    retrieval_scope: Option<&Path>,
) -> Option<LineageContextFragment> {
    let recorded = turn.get::<LineageTurnDecision>()?;
    let RoutingDecision::Selected {
        ledger,
        selection,
        generation,
        ..
    } = &recorded.decision
    else {
        return None;
    };
    let result = (|| {
        let thread_state = thread.get_or_init(OntographThreadState::default);
        let current_cwd = thread_state
            .current_cwd()
            .ok_or(AdmissionRejection::ScopeInvalid)?;
        let root =
            crate::km_store::project_root(&current_cwd).ok_or(AdmissionRejection::ScopeInvalid)?;
        let scope_valid = retrieval_scope == Some(root.as_path());
        if !scope_valid {
            return Err(AdmissionRejection::ScopeInvalid);
        }
        let store =
            KmStore::for_project_path(&root).ok_or(AdmissionRejection::SourceUnavailable)?;
        let scope = root.display().to_string();
        let eligibility = *thread.get_or_init(AgentAuthoredKmEligibility::default);
        let active_thread_id = thread.level_id();
        let fresh = refresh_selected(&store, &scope, selection, ledger, |record| {
            eligibility.admits(record)
                && match record.origin.as_ref() {
                    Some(KmIndexOrigin::AgentThread { thread_id }) => {
                        Some(active_thread_id) == Some(thread_id.as_str())
                    }
                    Some(KmIndexOrigin::Human) | None => true,
                }
        })?;
        admit_lineage(selection, ledger, generation, &fresh, scope_valid)
    })();
    match result {
        Ok(fragment) => {
            turn.insert(LineageAdmissionResult(Ok(())));
            Some(fragment)
        }
        Err(rejection) => {
            turn.insert(LineageAdmissionResult(Err(rejection)));
            None
        }
    }
}

/// One turn's lineage routing decision, recorded for the turn-item seam.
///
/// The turn-input seam cannot ground: grounding needs assistant claims, which
/// do not exist until the model answers. Stage A therefore records what it
/// decided, and the turn-item contributor consumes it within the same turn.
/// A recorded `Baseline` reason is also the only place a fallback becomes
/// observable outside `route` itself.
pub(crate) struct LineageTurnDecision {
    pub(crate) decision: RoutingDecision,
}

fn routing_tags(
    d: &RoutingDecision,
    r: Option<AdmissionRejection>,
) -> Option<[(&'static str, &'static str); 3]> {
    let (outcome, admission, fallback_category) = match d {
        RoutingDecision::Selected { .. } => match r {
            // None is the accepted sentinel after admission has resolved.
            None => ("lineage", "accepted", "none"),
            Some(rejection) => ("baseline", "rejected", rejection.category()),
        },
        RoutingDecision::Baseline { reason } => {
            let category = match reason {
                FallbackReason::Disabled => "disabled",
                FallbackReason::NotEligible { .. } => "not_eligible",
                FallbackReason::InductionFailed(_) => "induction_failed",
                FallbackReason::RetrievalFailed(_) => "retrieval_failed",
                FallbackReason::SelectionFailed(_) => "selection_failed",
                FallbackReason::GroundingFailed(_) => return None,
                FallbackReason::SchemaInvalid(_) => "schema_invalid",
                FallbackReason::Timeout { .. } => "timeout",
                FallbackReason::BudgetExhausted { .. } => "budget_exhausted",
                FallbackReason::TokenCapExceeded { .. } => "token_cap_exceeded",
                FallbackReason::PartialCoverage { .. } => "partial_coverage",
                FallbackReason::NoUsableEvidence => "no_usable_evidence",
            };
            let admission = if matches!(reason, FallbackReason::Disabled) {
                "disabled"
            } else {
                "rejected"
            };
            ("baseline", admission, category)
        }
        RoutingDecision::Lineage { .. } => return None,
    };
    Some([
        ("outcome", outcome),
        ("admission", admission),
        ("fallback_category", fallback_category),
    ])
}

const LIVE_COUNTER: &str = "lineage_routing_attempt_total";
const SHADOW_COUNTER: &str = "lineage_shadow_attempt_total";

#[cfg(test)]
fn emit_stage_a(client: Option<&MetricsClient>, turn: &ExtensionData) {
    emit_stage_a_as(client, turn, LIVE_COUNTER);
}

fn emit_stage_a_as(client: Option<&MetricsClient>, turn: &ExtensionData, counter: &str) {
    let Some(client) = client else {
        return;
    };
    let Some(recorded) = turn.get::<LineageTurnDecision>() else {
        return;
    };
    let rejection = match &recorded.decision {
        RoutingDecision::Selected { .. } => {
            let Some(admission) = turn.get::<LineageAdmissionResult>() else {
                // Selection without an admission result is unresolved.
                return;
            };
            admission.0.err()
        }
        RoutingDecision::Baseline { .. } | RoutingDecision::Lineage { .. } => None,
    };
    let Some(tags) = routing_tags(&recorded.decision, rejection) else {
        return;
    };
    if turn.insert(LineageMetricAttempted).is_some() {
        return;
    }
    let _ = client.counter(counter, /*inc*/ 1, &tags);
}

/// Appends this turn's admission tuple to the operator-named directory, once per turn.
///
/// Silent when the thread has no enabled sink, when the decision is unresolved, or
/// when the tuple has no routing tags. The write runs on the blocking pool so the
/// turn never waits on the file system.
fn record_admission(thread: &ExtensionData, turn: &ExtensionData, stage_a_elapsed_ms: u64) {
    let Some(sink) = thread.get::<AdmissionRecordSink>() else {
        return;
    };
    if !sink.is_enabled() {
        return;
    }
    let Some(recorded) = turn.get::<LineageTurnDecision>() else {
        return;
    };
    let rejection = match &recorded.decision {
        RoutingDecision::Selected { .. } => {
            let Some(admission) = turn.get::<LineageAdmissionResult>() else {
                return;
            };
            admission.0.err()
        }
        RoutingDecision::Baseline { .. } | RoutingDecision::Lineage { .. } => None,
    };
    let Some([(_, outcome), (_, admission), (_, fallback_category)]) =
        routing_tags(&recorded.decision, rejection)
    else {
        return;
    };
    if turn.insert(LineageRecordAttempted).is_some() {
        return;
    }
    let record = AdmissionRecord {
        turn_id: turn.level_id().to_string(),
        outcome,
        admission,
        fallback_category,
        stage_a_elapsed_ms,
    };
    match tokio::runtime::Handle::try_current() {
        Ok(runtime) => {
            drop(runtime.spawn_blocking(move || sink.append(&record)));
        }
        Err(_) => sink.append(&record),
    }
}

/// Routes one turn through the opt-in lineage path and records the outcome.
///
/// Emits no contextual fragment. Selection alone is not answer-eligible, so
/// nothing reaches the model from this seam and a baseline turn is unchanged.
/// The `attempt` is supplied by the caller, so this performs no provider work.
fn record_lineage_routing_decision(
    baseline: &BaselineOutcome,
    thread_store: &ExtensionData,
    turn_store: &ExtensionData,
    budget: LineageBudget,
    attempt: impl FnOnce() -> LineageAttempt,
) {
    let config = thread_store
        .get_or_init(LineageRoutingEligibility::default)
        .routing_config();
    // `route` owns the enabled, eligibility, and budget gates.
    let decision = route(config, baseline, budget, attempt);
    turn_store.insert(LineageTurnDecision { decision });
}

/// Second half of the split lineage path.
///
/// A separate type from `SurfaceScopedOntographExtension` because that type
/// already implements `TurnInputContributor::contribute`; one type carrying
/// both traits makes every existing `.contribute(..)` call site ambiguous.
///
/// Role: grounds the selection recorded at turn input against the assistant
/// answer, which is the first point where claim text exists. Contract: it
/// never fails a turn and never mutates the item. The host logs a contributor
/// error and continues, so every rejection leaves the item exactly as the
/// baseline produced it and is recorded as a fallback reason instead.
#[derive(Clone, Default)]
pub(crate) struct LineageGroundingContributor;

#[async_trait::async_trait]
impl TurnItemContributor for LineageGroundingContributor {
    async fn contribute(
        &self,
        thread_store: &ExtensionData,
        turn_store: &ExtensionData,
        item: &mut TurnItem,
    ) -> Result<(), String> {
        if thread_store
            .get::<LineageRoutingEligibility>()
            .is_some_and(|eligibility| (*eligibility).shadow_only())
        {
            // Shadow never showed the model a block, so there is nothing to ground.
            return Ok(());
        }
        let Some(recorded) = turn_store.get::<LineageTurnDecision>() else {
            // No routed turn, so there is nothing to ground.
            return Ok(());
        };
        let RoutingDecision::Selected {
            ledger,
            selection,
            generation,
            cost,
        } = &recorded.decision
        else {
            // A recorded baseline fallback stays on baseline.
            return Ok(());
        };
        if !matches!(
            turn_store.get::<LineageAdmissionResult>().as_deref(),
            Some(LineageAdmissionResult(Ok(())))
        ) {
            // Rejected or unresolved admission cannot become lineage success.
            // Keep the typed admission result for Stage A diagnostics.
            turn_store.insert(LineageTurnDecision {
                decision: RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                },
            });
            return Ok(());
        }
        let TurnItem::AgentMessage(message) = item else {
            return Ok(());
        };

        let answer = message
            .content
            .iter()
            .map(|content| match content {
                AgentMessageContent::Text { text } => text.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        if answer.trim().is_empty() {
            turn_store.insert(LineageTurnDecision {
                decision: RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                },
            });
            return Ok(());
        }

        // Each selected passage is cited under the demand that retrieved it,
        // quoting the smallest answer sentence contained by that passage.
        let claims = selection
            .selected
            .iter()
            .filter_map(|selected| {
                let lineage = ledger.lineage_for(&selected.passage_id)?;
                let demand_id = lineage.first_exposure.demand_id.clone();
                let observed_generation_id = generation
                    .files
                    .get(&lineage.source_location.path)?
                    .generation_id
                    .clone();
                Some(GroundingClaim {
                    passage_id: selected.passage_id.clone(),
                    demand_id,
                    observed_generation_id,
                    quoted_text: passage_quote(&answer, ledger, generation, selected),
                })
            })
            .collect::<Vec<_>>();
        if claims.is_empty() {
            turn_store.insert(LineageTurnDecision {
                decision: RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                },
            });
            return Ok(());
        }

        // A grounding rejection is a fallback, not a turn failure, so the
        // reason is recorded and the item is left untouched.
        let report = match ground_claims(ledger, selection, generation, &claims) {
            Ok(report) => report,
            Err(error) => {
                turn_store.insert(LineageTurnDecision {
                    decision: RoutingDecision::Baseline {
                        reason: FallbackReason::GroundingFailed(error.to_string()),
                    },
                });
                return Ok(());
            }
        };

        let usable_claims = report.usable_claims().len();
        // Partial coverage falls back; the integration contract does not yet
        // permit answering with unresolved demands.
        let decision = if usable_claims == 0 {
            RoutingDecision::Baseline {
                reason: FallbackReason::NoUsableEvidence,
            }
        } else if !report.unresolved_demands.is_empty() {
            RoutingDecision::Baseline {
                reason: FallbackReason::PartialCoverage {
                    unresolved_demands: report.unresolved_demands,
                },
            }
        } else {
            RoutingDecision::Lineage {
                usable_claims,
                unresolved_demands: report.unresolved_demands,
                cost: *cost,
            }
        };
        turn_store.insert(LineageTurnDecision { decision });
        Ok(())
    }
}

/// Chooses the answer sentence grounded by one selected passage.
///
/// A multi-passage answer must not require every passage to contain the full
/// answer. When no sentence is contained, retain the full answer so grounding
/// records an explicit non-containment result instead of silently dropping the
/// claim.
fn passage_quote(
    answer: &str,
    ledger: &LineageLedger,
    generation: &SourceGeneration,
    selected: &crate::lineage_selection::SelectedPassage,
) -> String {
    let span_text = ledger
        .lineage_for(&selected.passage_id)
        .and_then(|lineage| {
            let (Some(start_line), Some(end_line)) = (
                lineage.source_location.start_line,
                lineage.source_location.end_line,
            ) else {
                return None;
            };
            let file = generation.files.get(&lineage.source_location.path)?;
            Some(
                file.lines
                    .get(start_line.saturating_sub(1)..end_line)?
                    .join("\n"),
            )
        })
        .unwrap_or_default();

    answer
        .split(['.', '!', '?', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .filter(|sentence| span_text.contains(*sentence))
        .max_by_key(|sentence| sentence.len())
        .map_or_else(|| answer.to_string(), ToString::to_string)
}

fn update_current_cwd(
    input: &TurnInputContext,
    thread_store: &ExtensionData,
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    let Some(environment) = input
        .environments
        .iter()
        .find(|environment| environment.is_primary)
        .or_else(|| input.environments.first())
    else {
        thread_store
            .get_or_init(OntographThreadState::default)
            .clear_current_cwd();
        return Vec::new();
    };

    thread_store
        .get_or_init(OntographThreadState::default)
        .set_current_cwd(environment.cwd.clone());
    Vec::new()
}

#[cfg(test)]
pub(crate) fn matching_km_context_fragments(
    input: &TurnInputContext,
    store: &KmStore,
    active_scope: &str,
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    matching_km_context_fragments_with_eligibility(
        input,
        store,
        active_scope,
        AgentAuthoredKmEligibility::default(),
    )
}

#[cfg(test)]
pub(crate) fn matching_km_context_fragments_with_eligibility(
    input: &TurnInputContext,
    store: &KmStore,
    active_scope: &str,
    eligibility: AgentAuthoredKmEligibility,
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    let user_words = user_text_words(&input.user_input);
    matching_km_context_fragments_with(
        &user_words,
        active_scope,
        store,
        eligibility,
        None,
        &user_text_identifiers(&input.user_input),
        |_| true,
        |_, _| {},
    )
}

#[cfg(test)]
pub(crate) fn matching_km_context_fragments_with_eligibility_for_thread(
    input: &TurnInputContext,
    store: &KmStore,
    active_scope: &str,
    eligibility: AgentAuthoredKmEligibility,
    active_thread_id: &str,
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    let user_words = user_text_words(&input.user_input);
    matching_km_context_fragments_with(
        &user_words,
        active_scope,
        store,
        eligibility,
        Some(active_thread_id),
        &user_text_identifiers(&input.user_input),
        |_| true,
        |_, _| {},
    )
}

#[cfg(test)]
pub(crate) fn matching_km_context_fragments_for_thread(
    input: &TurnInputContext,
    store: &KmStore,
    active_scope: &str,
    thread_state: &OntographThreadState,
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    let user_words = user_text_words(&input.user_input);
    matching_km_context_fragments_for_thread_words(
        &user_words,
        active_scope,
        store,
        thread_state,
        AgentAuthoredKmEligibility::default(),
        None,
        &user_text_identifiers(&input.user_input),
    )
}

fn matching_km_context_fragments_for_thread_words(
    user_words: &[String],
    active_scope: &str,
    store: &KmStore,
    thread_state: &OntographThreadState,
    eligibility: AgentAuthoredKmEligibility,
    active_thread_id: Option<&str>,
    procedural_identifiers: &[String],
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    matching_km_context_fragments_with(
        user_words,
        active_scope,
        store,
        eligibility,
        active_thread_id,
        procedural_identifiers,
        |record| {
            !thread_state.auto_km_context_was_injected(active_scope, &record.id, &record.version)
        },
        |id, version| thread_state.mark_auto_km_context_injected(active_scope, id, version),
    )
}

#[allow(clippy::too_many_arguments)]
fn matching_km_context_fragments_with(
    user_words: &[String],
    active_scope: &str,
    store: &KmStore,
    eligibility: AgentAuthoredKmEligibility,
    active_thread_id: Option<&str>,
    procedural_identifiers: &[String],
    mut should_consider: impl FnMut(&KmIndexRecord) -> bool,
    mut mark_injected: impl FnMut(&str, &str),
) -> Vec<Box<dyn ContextualUserFragment + Send>> {
    if user_words.len() < MIN_AUTO_KM_USER_WORDS {
        return Vec::new();
    }

    let Ok(records) = store.list(/*include_obsolete*/ false) else {
        return Vec::new();
    };
    let mut matches = records
        .into_iter()
        .filter(|record| record.project_scope.as_deref() == Some(active_scope))
        .filter(|record| eligibility.admits(record))
        .filter(|record| match record.origin.as_ref() {
            Some(KmIndexOrigin::AgentThread { thread_id }) => {
                active_thread_id == Some(thread_id.as_str())
            }
            Some(KmIndexOrigin::Human) | None => true,
        })
        .filter_map(|record| score_record(record, user_words))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .matches
            .cmp(&left.matches)
            .then_with(|| left.record.id.cmp(&right.record.id))
            .then_with(|| left.record.version.cmp(&right.record.version))
    });

    let mut fragments: Vec<Box<dyn ContextualUserFragment + Send>> = Vec::new();
    for matched in matches {
        if fragments.len() >= MAX_AUTO_KM_CONTEXTS {
            break;
        }
        if !should_consider(&matched.record) {
            continue;
        }
        let Ok(Some(resolved)) = resolve_active_km(store, &matched.record.id) else {
            continue;
        };
        let base_fragment = render_km_context(&resolved, AUTO_KM_CONTEXT_REASON);
        let fragment = match procedural_neighborhood_for_record(
            store,
            &matched.record,
            procedural_identifiers,
        ) {
            Some(neighborhood) => {
                let guided = render_km_context_with_procedural_guidance(
                    &resolved,
                    AUTO_KM_CONTEXT_REASON,
                    &neighborhood,
                );
                if guided.exceeds_review_threshold() {
                    if base_fragment.exceeds_review_threshold() {
                        continue;
                    }
                    base_fragment
                } else {
                    guided
                }
            }
            None if !base_fragment.exceeds_review_threshold() => base_fragment,
            None => continue,
        };
        mark_injected(&matched.record.id, &matched.record.version);
        fragments.push(Box::new(fragment));
    }
    fragments
}

fn procedural_neighborhood_for_record(
    store: &KmStore,
    record: &KmIndexRecord,
    identifiers: &[String],
) -> Option<crate::procedural_graph::ProceduralNeighborhood> {
    let root = store.validated_module_root(record).ok()?;
    let graph = ProceduralGraph::from_validated_transitions(load_transitions(&root).ok()?);
    identifiers.iter().rev().find_map(|identifier| {
        match graph.project(identifier, MAX_REQUESTED_HOPS) {
            ProceduralLookup::Localized(neighborhood) => Some(neighborhood),
            ProceduralLookup::Fallback { .. } => None,
        }
    })
}

struct MatchedKm {
    record: KmIndexRecord,
    matches: usize,
}

fn score_record(record: KmIndexRecord, user_words: &[String]) -> Option<MatchedKm> {
    let words = record
        .topic_words
        .iter()
        .filter(|word| user_words.binary_search(word).is_ok())
        .cloned()
        .collect::<Vec<_>>();
    if words.len() < MIN_AUTO_KM_TOPIC_MATCHES {
        return None;
    }
    Some(MatchedKm {
        record,
        matches: words.len(),
    })
}

fn user_text_words(user_input: &[UserInput]) -> Vec<String> {
    let mut words = Vec::new();
    for input in user_input {
        if let UserInput::Text { text, .. } = input {
            collect_words(text, &mut words);
        }
    }
    words.sort();
    words.dedup();
    words
}

fn user_text_identifiers(user_input: &[UserInput]) -> Vec<String> {
    user_input
        .iter()
        .filter_map(|input| match input {
            UserInput::Text { text, .. } => Some(text),
            _ => None,
        })
        .flat_map(|text| text.split_whitespace().map(str::to_string))
        .collect()
}

fn collect_words(text: &str, words: &mut Vec<String>) {
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
            continue;
        }
        insert_word(&mut current, words);
    }
    insert_word(&mut current, words);
}

fn insert_word(current: &mut String, words: &mut Vec<String>) {
    if current.len() >= 3 {
        words.push(std::mem::take(current));
    } else {
        current.clear();
    }
}

#[async_trait::async_trait]
impl ThreadLifecycleContributor<Config> for SurfaceScopedOntographExtension {
    async fn on_thread_start(&self, input: ThreadStartInput<'_, Config>) {
        let enabled = input
            .config
            .features
            .enabled(ontocode_features::Feature::AgentAuthoredProjectKnowledge);
        input
            .thread_store
            .insert(AgentAuthoredKmEligibility::from_enabled(enabled));
        let lineage_enabled = input
            .config
            .features
            .enabled(ontocode_features::Feature::LineageRouting);
        let shadow_enabled = input
            .config
            .features
            .enabled(ontocode_features::Feature::LineageShadow);
        input
            .thread_store
            .insert(LineageRoutingEligibility::from_flags(
                lineage_enabled,
                shadow_enabled,
            ));
        if lineage_enabled || shadow_enabled {
            // Resolved once here so the turn path never reads the environment.
            input.thread_store.insert(AdmissionRecordSink::from_env());
        }
    }

    async fn on_history_compacted(&self, input: HistoryCompactedInput<'_>) {
        // Successful compaction drops TurnInputContributor fragments from replacement
        // history. Advance the once-per-generation dedup marks so a later matching turn
        // may re-admit at most one automatic KM fragment for this generation.
        input
            .thread_store
            .get_or_init(OntographThreadState::default)
            .advance_auto_km_compaction_generation();
    }
}

/// Installs the full local Ontograph surface, including the read-only analysis
/// tools. Used by hosts that already own the complete dormant surface.
pub fn install(registry: &mut ontocode_extension_api::ExtensionRegistryBuilder<Config>) {
    let extension = Arc::new(SurfaceScopedOntographExtension::default());
    registry.tool_contributor(extension.clone());
    registry.thread_lifecycle_contributor(extension.clone());
    registry.turn_input_contributor(extension);
    // Grounding runs after the answer exists; both halves must be installed
    // together or an enabled feature can never produce grounded evidence.
    registry.turn_item_contributor(Arc::new(LineageGroundingContributor));
}

/// Installs only the surface the accepted host ADR authorizes: the KM context
/// and agent knowledge verbs, plus the lifecycle and turn-input contributors
/// that gate them. The broader Ontograph analysis tools are deliberately not
/// registered on this host.
pub fn install_agent_knowledge_surface(
    registry: &mut ontocode_extension_api::ExtensionRegistryBuilder<Config>,
) {
    install_agent_knowledge_surface_with_metrics(registry, /*metrics_client*/ None);
}

/// Installs the agent knowledge surface with optional Stage A metrics.
pub fn install_agent_knowledge_surface_with_metrics(
    registry: &mut ontocode_extension_api::ExtensionRegistryBuilder<Config>,
    metrics_client: Option<MetricsClient>,
) {
    let extension = Arc::new(SurfaceScopedOntographExtension { metrics_client });
    registry.tool_contributor(Arc::new(AgentKnowledgeToolContributor));
    registry.thread_lifecycle_contributor(extension.clone());
    registry.turn_input_contributor(extension);
    registry.turn_item_contributor(Arc::new(LineageGroundingContributor));
}

/// Contributes exactly the four verbs named by the accepted host ADR.
#[derive(Clone, Default)]
struct AgentKnowledgeToolContributor;

impl ToolContributor for AgentKnowledgeToolContributor {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        let thread_state = thread_store.get_or_init(OntographThreadState::default);
        let thread_id = thread_store.level_id().to_string();
        let mut tools: Vec<Arc<dyn ToolExecutor<ToolCall>>> = vec![Arc::new(
            KmContextTool::new_for_thread(thread_state.clone(), thread_id.clone()),
        )];
        // The three agent verbs exist only while the feature is enabled; when it
        // is off the model sees no authoring surface at all.
        let eligibility = *thread_store.get_or_init(AgentAuthoredKmEligibility::default);
        if eligibility.enabled {
            tools.extend(crate::km_agent_tools::agent_knowledge_tools(
                thread_state,
                thread_id,
            ));
        }
        tools
    }
}
