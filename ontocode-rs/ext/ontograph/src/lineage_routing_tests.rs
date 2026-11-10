use std::cell::Cell;

use pretty_assertions::assert_eq;

use super::*;
use crate::lineage_grounding::ContainmentStatus;
use crate::lineage_grounding::DemandSupport;
use crate::lineage_grounding::GroundedClaim;
use ontocode_protocol::user_input::UserInput;

fn enabled() -> RoutingConfig {
    RoutingConfig {
        enabled: true,
        ..RoutingConfig::default()
    }
}

/// Budget that always covers the configured latency cap.
fn ample() -> LineageBudget {
    LineageBudget::from_remaining_ms(u64::MAX)
}

fn text(value: &str) -> UserInput {
    UserInput::Text {
        text: value.to_string(),
        text_elements: Vec::new(),
    }
}

#[test]
fn local_induction_is_deterministic_and_counts_bounded_hops() {
    let input = vec![text("Find the owner and then validate the contract?")];
    let first = induce_baseline_outcome(&input).expect("induction");
    let second = induce_baseline_outcome(&input).expect("induction");
    assert_eq!(first, second);
    assert_eq!(first.induced_demands, 2);
    assert_eq!(first.confidence_bp, 9_500);
}

#[test]
fn local_induction_marks_uncertain_questions_low_confidence() {
    let outcome =
        induce_baseline_outcome(&[text("Maybe the owner is unclear?")]).expect("induction");
    assert_eq!(outcome.induced_demands, 1);
    assert_eq!(outcome.confidence_bp, 6_500);
    assert!(is_eligible(enabled(), &outcome));
}

#[test]
fn overlapping_connectives_count_one_hop_and_repeats_count_each() {
    // "and then" overlaps two markers but joins a single extra clause.
    let overlap = induce_baseline_outcome(&[text("Load A and then B?")]).expect("induction");
    assert_eq!(overlap.induced_demands, 2);

    // A repeated connective contributes once per occurrence.
    let repeated = induce_baseline_outcome(&[text("Load A and B and C?")]).expect("induction");
    assert_eq!(repeated.induced_demands, 3);
}

#[test]
fn hedge_words_are_matched_on_word_boundaries_not_substrings() {
    // "shoulder" contains "could"; "unsure_flag" contains "unsure". Neither is
    // hedging, so both must keep full confidence and stay ineligible.
    for question in [
        "Which shoulder joint file owns the parser?",
        "Where is the unsure_flag constant used?",
    ] {
        let outcome = induce_baseline_outcome(&[text(question)]).expect("induction");
        assert_eq!(outcome.confidence_bp, 9_500, "for {question}");
        assert!(!is_eligible(enabled(), &outcome), "for {question}");
    }

    // A real hedge still lowers confidence.
    let hedged = induce_baseline_outcome(&[text("Could this be the owner?")]).expect("induction");
    assert_eq!(hedged.confidence_bp, 8_000);
}

#[test]
fn an_exhausted_budget_declines_the_attempt_before_it_starts() {
    let attempted = Cell::new(false);
    let decision = route(
        enabled(),
        &multi_hop(),
        LineageBudget::from_remaining_ms(0),
        || {
            attempted.set(true);
            completed(true, cost(10, 100))
        },
    );

    assert!(!attempted.get(), "work must not start without budget");
    // Distinct from `Timeout`, which reports an attempt that actually overran.
    assert_eq!(
        decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::BudgetExhausted {
                remaining_ms: 0,
                required_ms: RoutingConfig::default().max_latency_ms,
            }
        }
    );
}

#[test]
fn a_budget_covering_the_latency_cap_runs_the_attempt() {
    let config = enabled();
    let decision = route(
        config,
        &multi_hop(),
        LineageBudget::from_remaining_ms(config.max_latency_ms),
        || completed(true, cost(10, 100)),
    );
    assert!(decision.used_lineage());
}

#[test]
fn local_induction_fails_closed_for_empty_non_text_and_oversized_input() {
    assert!(induce_baseline_outcome(&[]).is_err());
    assert!(
        induce_baseline_outcome(&[UserInput::Image {
            image_url: "data:image/png;base64,AA==".to_string(),
            detail: None,
        }])
        .is_err()
    );
    assert!(induce_baseline_outcome(&[text(&"x".repeat(MAX_INDUCTION_TEXT_CHARS + 1))]).is_err());
}

#[test]
fn routing_accepts_actual_adapter_output_without_provider_work() {
    let baseline = induce_baseline_outcome(&[text("Trace A and then B?")]).expect("induction");
    let attempted = Cell::new(false);
    let decision = route(enabled(), &baseline, ample(), || {
        attempted.set(true);
        completed(true, cost(10, 10))
    });
    assert!(attempted.get());
    assert!(decision.used_lineage());
}

fn multi_hop() -> BaselineOutcome {
    BaselineOutcome {
        induced_demands: 3,
        confidence_bp: 9_000,
    }
}

fn simple() -> BaselineOutcome {
    BaselineOutcome {
        induced_demands: 1,
        confidence_bp: 9_500,
    }
}

fn grounded(usable: bool) -> GroundingReport {
    GroundingReport {
        claims: vec![GroundedClaim {
            passage_id: "p-1".to_string(),
            demand_id: "d-1".to_string(),
            source_span: "src/a.rs:1-4".to_string(),
            containment: if usable {
                ContainmentStatus::Contained
            } else {
                ContainmentStatus::NotContained
            },
            demand_support: DemandSupport::Retrieved,
        }],
        unresolved_demands: Vec::new(),
    }
}

fn cost(elapsed_ms: u64, token_cost: usize) -> LineageCost {
    LineageCost {
        elapsed_ms,
        token_cost,
    }
}

fn completed(usable: bool, cost: LineageCost) -> LineageAttempt {
    LineageAttempt::Completed {
        grounding: grounded(usable),
        cost,
    }
}

#[test]
fn partial_coverage_reports_unresolved_demands_for_caller_fallback() {
    let mut grounding = grounded(true);
    grounding.unresolved_demands = vec!["d-2".to_string()];
    let decision = route(enabled(), &multi_hop(), ample(), || {
        LineageAttempt::Completed {
            grounding,
            cost: cost(10, 100),
        }
    });

    // `route` still reports lineage; the unresolved list is what lets the
    // runtime caller fall back rather than answer with partial coverage.
    assert_eq!(
        decision,
        RoutingDecision::Lineage {
            usable_claims: 1,
            unresolved_demands: vec!["d-2".to_string()],
            cost: cost(10, 100),
        }
    );
}

#[test]
fn routing_is_off_by_default_and_never_runs_the_lineage_path() {
    let attempted = Cell::new(false);
    let decision = route(RoutingConfig::default(), &multi_hop(), ample(), || {
        attempted.set(true);
        completed(true, cost(10, 100))
    });

    assert!(!attempted.get());
    assert_eq!(
        decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::Disabled
        }
    );
}

#[test]
fn simple_questions_stay_on_the_baseline_path() {
    let attempted = Cell::new(false);
    let decision = route(enabled(), &simple(), ample(), || {
        attempted.set(true);
        completed(true, cost(10, 100))
    });

    assert!(!attempted.get());
    assert_eq!(
        decision,
        RoutingDecision::Baseline {
            reason: FallbackReason::NotEligible {
                induced_demands: 1,
                confidence_bp: 9_500,
            }
        }
    );
}

#[test]
fn multi_hop_questions_use_the_lineage_path() {
    let decision = route(enabled(), &multi_hop(), ample(), || {
        completed(true, cost(50, 400))
    });

    assert!(decision.used_lineage());
    assert_eq!(
        decision,
        RoutingDecision::Lineage {
            usable_claims: 1,
            unresolved_demands: Vec::new(),
            cost: cost(50, 400),
        }
    );
    assert!(decision.fallback_reason().is_none());
}

#[test]
fn low_confidence_single_hop_questions_also_route() {
    let low_confidence = BaselineOutcome {
        induced_demands: 1,
        confidence_bp: 2_000,
    };

    assert!(is_eligible(enabled(), &low_confidence));
    assert!(
        route(enabled(), &low_confidence, ample(), || completed(
            true,
            cost(5, 20)
        ))
        .used_lineage()
    );
}

#[test]
fn every_stage_failure_falls_back_with_its_own_reason() {
    let cases: Vec<(LineageAttempt, FallbackReason)> = vec![
        (
            LineageAttempt::InductionFailed("no demands".to_string()),
            FallbackReason::InductionFailed("no demands".to_string()),
        ),
        (
            LineageAttempt::RetrievalFailed(LineageError::MissingDemandId),
            FallbackReason::RetrievalFailed(LineageError::MissingDemandId.to_string()),
        ),
        (
            LineageAttempt::RetrievalUnavailable("KM store unavailable".to_string()),
            FallbackReason::RetrievalFailed("KM store unavailable".to_string()),
        ),
        (
            LineageAttempt::SelectionFailed(SelectionError::ZeroPassageBudget),
            FallbackReason::SelectionFailed(SelectionError::ZeroPassageBudget.to_string()),
        ),
        (
            LineageAttempt::GroundingFailed("stale source".to_string()),
            FallbackReason::GroundingFailed("stale source".to_string()),
        ),
        (
            LineageAttempt::SchemaInvalid("unknown field".to_string()),
            FallbackReason::SchemaInvalid("unknown field".to_string()),
        ),
        (
            LineageAttempt::TimedOut { elapsed_ms: 99_999 },
            FallbackReason::Timeout {
                elapsed_ms: 99_999,
                limit_ms: RoutingConfig::default().max_latency_ms,
            },
        ),
    ];

    for (attempt, expected) in cases {
        let decision = route(enabled(), &multi_hop(), ample(), || attempt.clone());
        assert_eq!(
            decision,
            RoutingDecision::Baseline { reason: expected },
            "unexpected decision for {attempt:?}"
        );
    }
}

#[test]
fn latency_cap_is_enforced_on_a_completed_attempt() {
    let config = RoutingConfig {
        max_latency_ms: 100,
        ..enabled()
    };

    assert_eq!(
        route(config, &multi_hop(), ample(), || completed(
            true,
            cost(101, 10)
        )),
        RoutingDecision::Baseline {
            reason: FallbackReason::Timeout {
                elapsed_ms: 101,
                limit_ms: 100,
            }
        }
    );
    assert!(
        route(config, &multi_hop(), ample(), || completed(
            true,
            cost(100, 10)
        ))
        .used_lineage()
    );
}

#[test]
fn token_cap_is_enforced_on_a_completed_attempt() {
    let config = RoutingConfig {
        max_token_cost: 500,
        ..enabled()
    };

    assert_eq!(
        route(config, &multi_hop(), ample(), || completed(
            true,
            cost(10, 501)
        )),
        RoutingDecision::Baseline {
            reason: FallbackReason::TokenCapExceeded {
                token_cost: 501,
                limit: 500,
            }
        }
    );
    assert!(
        route(config, &multi_hop(), ample(), || completed(
            true,
            cost(10, 500)
        ))
        .used_lineage()
    );
}

#[test]
fn a_completed_attempt_without_usable_evidence_falls_back() {
    assert_eq!(
        route(enabled(), &multi_hop(), ample(), || completed(
            false,
            cost(10, 10)
        )),
        RoutingDecision::Baseline {
            reason: FallbackReason::NoUsableEvidence
        }
    );
}

#[test]
fn unresolved_demands_are_reported_without_blocking_the_lineage_path() {
    let mut grounding = grounded(true);
    grounding.unresolved_demands = vec!["d-open".to_string()];

    let decision = route(enabled(), &multi_hop(), ample(), || {
        LineageAttempt::Completed {
            grounding,
            cost: cost(10, 10),
        }
    });

    assert_eq!(
        decision,
        RoutingDecision::Lineage {
            usable_claims: 1,
            unresolved_demands: vec!["d-open".to_string()],
            cost: cost(10, 10),
        }
    );
}

#[test]
fn fallback_reasons_render_diagnostics() {
    assert_eq!(
        FallbackReason::Disabled.to_string(),
        "lineage routing is disabled"
    );
    assert_eq!(
        FallbackReason::Timeout {
            elapsed_ms: 120,
            limit_ms: 100,
        }
        .to_string(),
        "lineage path took 120ms, exceeding 100ms"
    );
    assert_eq!(
        FallbackReason::TokenCapExceeded {
            token_cost: 900,
            limit: 500,
        }
        .to_string(),
        "lineage path used 900 tokens, exceeding the cap 500"
    );
    assert_eq!(
        FallbackReason::NoUsableEvidence.to_string(),
        "lineage path produced no usable grounded evidence"
    );
}

#[test]
fn a_partial_budget_runs_the_attempt_under_the_smaller_cap() {
    let config = enabled();
    // Slightly under the configured cap, as a real host deadline always is.
    let remaining_ms = config.max_latency_ms - 47;
    let admitted = route(
        config,
        &multi_hop(),
        LineageBudget::from_remaining_ms(remaining_ms),
        || completed(true, cost(remaining_ms, 100)),
    );
    assert!(admitted.used_lineage());

    let overran = route(
        config,
        &multi_hop(),
        LineageBudget::from_remaining_ms(remaining_ms),
        || completed(true, cost(remaining_ms + 1, 100)),
    );
    assert_eq!(
        overran,
        RoutingDecision::Baseline {
            reason: FallbackReason::Timeout {
                elapsed_ms: remaining_ms + 1,
                limit_ms: remaining_ms,
            }
        }
    );
}
