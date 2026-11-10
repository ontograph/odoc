use pretty_assertions::assert_eq;

use super::*;
use crate::evidence_planner::SourceLocation;
use crate::lineage_records::CandidatePassage;
use crate::lineage_records::EvidenceDemand;
use crate::lineage_records::LineageLimits;
use crate::lineage_records::merge_candidates;

fn location(path: &str, start_line: usize, end_line: usize) -> SourceLocation {
    SourceLocation {
        path: path.to_string(),
        start_line: Some(start_line),
        end_line: Some(end_line),
    }
}

fn demand(demand_id: &str) -> EvidenceDemand {
    EvidenceDemand {
        demand_id: demand_id.to_string(),
        description: format!("demand {demand_id}"),
        bindings: BTreeMap::new(),
    }
}

fn candidate(passage_id: &str, demand_id: &str, rank: usize, score: usize) -> CandidatePassage {
    CandidatePassage {
        passage_id: passage_id.to_string(),
        demand_id: demand_id.to_string(),
        rank,
        score,
        source_location: location(&format!("src/{passage_id}.rs"), 1, 20),
    }
}

/// `p-high` scores best but only covers `d-a`, which `p-both` also covers.
/// A score-ranked selector would spend the budget on `p-high` and leave `d-b`
/// uncovered; a coverage-first selector takes `p-both` and then `p-c`.
fn ledger() -> LineageLedger {
    let demands = vec![demand("d-a"), demand("d-b"), demand("d-c")];
    let candidates = vec![
        candidate("p-high", "d-a", 1, 20),
        candidate("p-both", "d-a", 2, 9),
        candidate("p-both", "d-b", 1, 8),
        candidate("p-c", "d-c", 1, 5),
    ];
    merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge")
}

fn token_costs() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("p-high".to_string(), 30),
        ("p-both".to_string(), 40),
        ("p-c".to_string(), 25),
    ])
}

#[test]
fn selection_prefers_marginal_coverage_over_raw_score() {
    let report =
        select_evidence(&ledger(), &token_costs(), SelectionBudget::default()).expect("selection");

    // `p-high` carries the best score but adds no coverage once `p-both` is
    // admitted, so it is never spent from the budget.
    assert_eq!(
        report
            .selected
            .iter()
            .map(|passage| passage.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-both", "p-c"]
    );
    assert_eq!(report.selected[0].marginal_gain, 2);
    assert_eq!(
        report.selected[0].newly_covered_demands,
        BTreeSet::from(["d-a".to_string(), "d-b".to_string()])
    );
    assert_eq!(report.total_token_cost, 65);
    assert!(report.is_complete());
    assert_eq!(report.stop_reason, SelectionStop::CoverageExhausted);
}

#[test]
fn passage_limit_is_respected_and_uncovered_demands_are_listed() {
    let budget = SelectionBudget {
        max_passages: 1,
        ..SelectionBudget::default()
    };
    let report = select_evidence(&ledger(), &token_costs(), budget).expect("selection");

    assert_eq!(report.selected.len(), 1);
    assert_eq!(report.selected[0].passage_id, "p-both");
    assert_eq!(report.uncovered_demands, vec!["d-c".to_string()]);
    assert_eq!(report.stop_reason, SelectionStop::PassageLimitReached);
    assert!(!report.is_complete());
}

#[test]
fn context_limit_is_respected_and_reported() {
    let budget = SelectionBudget {
        max_context_tokens: 50,
        ..SelectionBudget::default()
    };
    let report = select_evidence(&ledger(), &token_costs(), budget).expect("selection");

    assert_eq!(report.selected.len(), 1);
    assert_eq!(report.total_token_cost, 40);
    assert_eq!(report.uncovered_demands, vec!["d-c".to_string()]);
    assert_eq!(report.stop_reason, SelectionStop::ContextLimitReached);
}

#[test]
fn ledger_unresolved_demands_remain_uncovered() {
    let demands = vec![demand("d-a"), demand("d-orphan")];
    let candidates = vec![candidate("p-a", "d-a", 1, 4)];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-a".to_string(), 10)]);

    let report = select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");

    assert_eq!(report.covered_demands, BTreeSet::from(["d-a".to_string()]));
    assert_eq!(report.uncovered_demands, vec!["d-orphan".to_string()]);
    assert_eq!(report.stop_reason, SelectionStop::CoverageExhausted);
}

#[test]
fn redundant_passages_are_never_selected() {
    // `p-dup` covers only the demand `p-a` already covers, so it adds cost
    // without coverage and must not be admitted.
    let demands = vec![demand("d-a")];
    let candidates = vec![
        candidate("p-a", "d-a", 1, 9),
        candidate("p-dup", "d-a", 2, 8),
    ];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-a".to_string(), 10), ("p-dup".to_string(), 10)]);

    let report = select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");

    assert_eq!(report.selected.len(), 1);
    assert_eq!(report.selected[0].passage_id, "p-a");
    assert_eq!(report.total_token_cost, 10);
    assert_eq!(report.stop_reason, SelectionStop::CoverageExhausted);
}

#[test]
fn redundancy_penalty_breaks_ties_toward_less_overlapping_evidence() {
    // `p-seed` is admitted first on coverage. After it, `p-overlap` and
    // `p-fresh` each newly cover `d-b`, but `p-overlap` repeats the already
    // covered `d-a`, which is exactly what the penalty discounts.
    let demands = vec![demand("d-a"), demand("d-b"), demand("d-c")];
    let candidates = vec![
        candidate("p-seed", "d-a", 1, 30),
        candidate("p-seed", "d-c", 1, 30),
        candidate("p-overlap", "d-a", 2, 10),
        candidate("p-overlap", "d-b", 1, 10),
        candidate("p-fresh", "d-b", 2, 8),
    ];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([
        ("p-seed".to_string(), 10),
        ("p-overlap".to_string(), 10),
        ("p-fresh".to_string(), 10),
    ]);

    let unpenalized = SelectionBudget {
        redundancy_penalty: 0,
        ..SelectionBudget::default()
    };
    let report = select_evidence(&ledger, &costs, unpenalized).expect("selection");
    assert_eq!(
        report
            .selected
            .iter()
            .map(|passage| passage.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-seed", "p-overlap"]
    );

    let penalized = SelectionBudget {
        redundancy_penalty: 5,
        ..SelectionBudget::default()
    };
    let report = select_evidence(&ledger, &costs, penalized).expect("selection");
    assert_eq!(
        report
            .selected
            .iter()
            .map(|passage| passage.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-seed", "p-fresh"]
    );
    assert!(report.is_complete());
}

#[test]
fn selection_is_deterministic_across_repeated_runs() {
    let ledger = ledger();
    let costs = token_costs();
    let first = select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");
    let second = select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("serialize first"),
        serde_json::to_string(&second).expect("serialize second")
    );
}

#[test]
fn equal_coverage_and_score_ties_break_on_passage_id() {
    let demands = vec![demand("d-a"), demand("d-b")];
    let candidates = vec![
        candidate("p-zeta", "d-a", 1, 5),
        candidate("p-alpha", "d-b", 1, 5),
    ];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-zeta".to_string(), 10), ("p-alpha".to_string(), 10)]);

    let report = select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");

    assert_eq!(
        report
            .selected
            .iter()
            .map(|passage| passage.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-alpha", "p-zeta"]
    );
}

#[test]
fn selected_passages_retain_their_source_spans() {
    let report =
        select_evidence(&ledger(), &token_costs(), SelectionBudget::default()).expect("selection");

    assert_eq!(report.selected[0].source_span, "src/p-both.rs:1-20");
}

#[test]
fn zero_budgets_fail_closed() {
    let ledger = ledger();
    let costs = token_costs();

    let no_passages = SelectionBudget {
        max_passages: 0,
        ..SelectionBudget::default()
    };
    assert_eq!(
        select_evidence(&ledger, &costs, no_passages),
        Err(SelectionError::ZeroPassageBudget)
    );

    let no_context = SelectionBudget {
        max_context_tokens: 0,
        ..SelectionBudget::default()
    };
    assert_eq!(
        select_evidence(&ledger, &costs, no_context),
        Err(SelectionError::ZeroContextBudget)
    );
}

#[test]
fn missing_token_cost_fails_closed() {
    let mut costs = token_costs();
    costs.remove("p-c");

    assert_eq!(
        select_evidence(&ledger(), &costs, SelectionBudget::default()),
        Err(SelectionError::MissingTokenCost("p-c".to_string()))
    );
}

#[test]
fn a_passage_larger_than_the_whole_context_fails_closed() {
    let budget = SelectionBudget {
        max_context_tokens: 35,
        ..SelectionBudget::default()
    };

    assert_eq!(
        select_evidence(&ledger(), &token_costs(), budget),
        Err(SelectionError::PassageExceedsContextBudget {
            passage_id: "p-both".to_string(),
            token_cost: 40,
            limit: 35,
        })
    );
}

#[test]
fn selection_errors_render_actionable_messages() {
    assert_eq!(
        SelectionError::ZeroPassageBudget.to_string(),
        "selection requires a non-zero max_passages budget"
    );
    assert_eq!(
        SelectionError::MissingTokenCost("p-1".to_string()).to_string(),
        "passage `p-1` has no recorded token cost"
    );
    assert_eq!(
        SelectionError::PassageExceedsContextBudget {
            passage_id: "p-1".to_string(),
            token_cost: 90,
            limit: 40,
        }
        .to_string(),
        "passage `p-1` costs 90 tokens, exceeding the context limit 40"
    );
}
