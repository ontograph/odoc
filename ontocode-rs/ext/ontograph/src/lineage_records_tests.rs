use pretty_assertions::assert_eq;

use super::*;

fn location(path: &str, start_line: usize, end_line: usize) -> SourceLocation {
    SourceLocation {
        path: path.to_string(),
        start_line: Some(start_line),
        end_line: Some(end_line),
    }
}

fn demand(demand_id: &str, description: &str) -> EvidenceDemand {
    EvidenceDemand {
        demand_id: demand_id.to_string(),
        description: description.to_string(),
        bindings: BTreeMap::new(),
    }
}

fn candidate(
    passage_id: &str,
    demand_id: &str,
    rank: usize,
    score: usize,
    source_location: SourceLocation,
) -> CandidatePassage {
    CandidatePassage {
        passage_id: passage_id.to_string(),
        demand_id: demand_id.to_string(),
        rank,
        score,
        source_location,
    }
}

fn demands() -> Vec<EvidenceDemand> {
    vec![
        demand("d-dispatch", "dispatch lease"),
        demand("d-revision", "revision tracking"),
    ]
}

/// `p-shared` is retrieved by both demands; `p-revision` only by the second.
fn candidates() -> Vec<CandidatePassage> {
    vec![
        candidate(
            "p-shared",
            "d-dispatch",
            1,
            7,
            location("ontocode-rs/core/src/dispatch.rs", 10, 24),
        ),
        candidate(
            "p-revision",
            "d-revision",
            1,
            9,
            location("ontocode-rs/core/src/revision.rs", 5, 18),
        ),
        candidate(
            "p-shared",
            "d-revision",
            2,
            4,
            location("ontocode-rs/core/src/dispatch.rs", 10, 24),
        ),
    ]
}

#[test]
fn merging_retains_demand_membership_ranks_and_scores() {
    let ledger =
        merge_candidates(&demands(), &candidates(), LineageLimits::default()).expect("merge");

    let shared = ledger.lineage_for("p-shared").expect("shared lineage");
    assert_eq!(
        shared.demand_membership,
        BTreeSet::from(["d-dispatch".to_string(), "d-revision".to_string()])
    );
    assert_eq!(
        shared.exposures.get("d-dispatch"),
        Some(&FirstExposure {
            demand_id: "d-dispatch".to_string(),
            rank: 1,
            score: 7,
        })
    );
    assert_eq!(
        shared.exposures.get("d-revision"),
        Some(&FirstExposure {
            demand_id: "d-revision".to_string(),
            rank: 2,
            score: 4,
        })
    );
    assert_eq!(shared.best_rank(), 1);
    assert_eq!(shared.best_score(), 7);
}

#[test]
fn first_exposure_follows_declared_demand_order() {
    // The shared passage is reported by the later demand first in input order,
    // but first exposure must still credit the earlier demand.
    let reordered = vec![
        candidates()[2].clone(),
        candidates()[0].clone(),
        candidates()[1].clone(),
    ];
    let ledger = merge_candidates(&demands(), &reordered, LineageLimits::default()).expect("merge");

    let shared = ledger.lineage_for("p-shared").expect("shared lineage");
    assert_eq!(
        shared.first_exposure,
        FirstExposure {
            demand_id: "d-dispatch".to_string(),
            rank: 1,
            score: 7,
        }
    );
}

#[test]
fn merge_is_order_independent_and_deterministic() {
    let forward =
        merge_candidates(&demands(), &candidates(), LineageLimits::default()).expect("merge");
    let mut reversed = candidates();
    reversed.reverse();
    let backward =
        merge_candidates(&demands(), &reversed, LineageLimits::default()).expect("merge");

    assert_eq!(forward, backward);
    assert_eq!(
        serde_json::to_string(&forward).expect("serialize forward"),
        serde_json::to_string(&backward).expect("serialize backward")
    );
    assert_eq!(
        forward
            .passages
            .iter()
            .map(|lineage| lineage.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-revision", "p-shared"]
    );
}

#[test]
fn provenance_survives_merging() {
    let ledger =
        merge_candidates(&demands(), &candidates(), LineageLimits::default()).expect("merge");

    let revision = ledger.lineage_for("p-revision").expect("revision lineage");
    assert_eq!(
        revision.source_location,
        location("ontocode-rs/core/src/revision.rs", 5, 18)
    );
}

#[test]
fn typed_bindings_survive_merging() {
    let mut demands = demands();
    demands[0]
        .bindings
        .insert("symbol".to_string(), "manager_loop_advance".to_string());

    let ledger =
        merge_candidates(&demands, &candidates(), LineageLimits::default()).expect("merge");

    assert_eq!(
        ledger.demands[0].bindings.get("symbol").map(String::as_str),
        Some("manager_loop_advance")
    );
}

#[test]
fn demands_without_candidates_are_reported_as_unresolved() {
    let mut demands = demands();
    demands.push(demand("d-orphan", "no retrieval hit"));

    let ledger =
        merge_candidates(&demands, &candidates(), LineageLimits::default()).expect("merge");

    assert_eq!(ledger.unresolved_demands, vec!["d-orphan".to_string()]);
    assert!(ledger.passages_for_demand("d-orphan").is_empty());
    assert_eq!(ledger.passages_for_demand("d-revision").len(), 2);
}

#[test]
fn duplicate_demand_id_fails_closed() {
    let mut demands = demands();
    demands.push(demand("d-dispatch", "restated"));

    assert_eq!(
        merge_candidates(&demands, &candidates(), LineageLimits::default()),
        Err(LineageError::DuplicateDemandId("d-dispatch".to_string()))
    );
}

#[test]
fn blank_demand_id_fails_closed() {
    let demands = vec![demand("   ", "blank")];

    assert_eq!(
        merge_candidates(&demands, &[], LineageLimits::default()),
        Err(LineageError::MissingDemandId)
    );
}

#[test]
fn unknown_demand_reference_fails_closed() {
    let mut candidates = candidates();
    candidates[0].demand_id = "d-absent".to_string();

    assert_eq!(
        merge_candidates(&demands(), &candidates, LineageLimits::default()),
        Err(LineageError::UnknownDemandId {
            passage_id: "p-shared".to_string(),
            demand_id: "d-absent".to_string(),
        })
    );
}

#[test]
fn blank_passage_id_fails_closed() {
    let mut candidates = candidates();
    candidates[0].passage_id = String::new();

    assert_eq!(
        merge_candidates(&demands(), &candidates, LineageLimits::default()),
        Err(LineageError::MissingPassageId("d-dispatch".to_string()))
    );
}

#[test]
fn same_demand_ranking_a_passage_twice_fails_closed() {
    let mut candidates = candidates();
    candidates.push(candidate(
        "p-shared",
        "d-dispatch",
        3,
        1,
        location("ontocode-rs/core/src/dispatch.rs", 10, 24),
    ));

    assert_eq!(
        merge_candidates(&demands(), &candidates, LineageLimits::default()),
        Err(LineageError::DuplicateCandidate {
            demand_id: "d-dispatch".to_string(),
            passage_id: "p-shared".to_string(),
        })
    );
}

#[test]
fn conflicting_provenance_for_one_passage_fails_closed() {
    let mut candidates = candidates();
    candidates[2].source_location = location("ontocode-rs/core/src/dispatch.rs", 40, 60);

    assert_eq!(
        merge_candidates(&demands(), &candidates, LineageLimits::default()),
        Err(LineageError::ConflictingProvenance {
            passage_id: "p-shared".to_string(),
            existing: "ontocode-rs/core/src/dispatch.rs:10-24".to_string(),
            incoming: "ontocode-rs/core/src/dispatch.rs:40-60".to_string(),
        })
    );
}

#[test]
fn invalid_provenance_fails_closed() {
    let mut candidates = candidates();
    candidates[0].source_location = SourceLocation {
        path: "ontocode-rs/core/src/dispatch.rs".to_string(),
        start_line: Some(40),
        end_line: Some(10),
    };

    let error = merge_candidates(&demands(), &candidates, LineageLimits::default())
        .expect_err("inverted line range must fail");
    assert!(
        matches!(
            error,
            LineageError::InvalidProvenance { ref passage_id, .. } if passage_id == "p-shared"
        ),
        "unexpected error: {error}"
    );

    let mut missing_path = candidates;
    missing_path[0].source_location = SourceLocation {
        path: "   ".to_string(),
        start_line: None,
        end_line: None,
    };
    assert!(matches!(
        merge_candidates(&demands(), &missing_path, LineageLimits::default()),
        Err(LineageError::InvalidProvenance { .. })
    ));
}

#[test]
fn over_limit_demands_fail_closed() {
    let limits = LineageLimits {
        max_demands: 1,
        ..LineageLimits::default()
    };

    assert_eq!(
        merge_candidates(&demands(), &candidates(), limits),
        Err(LineageError::TooManyDemands { limit: 1, found: 2 })
    );
}

#[test]
fn over_limit_candidates_per_demand_fail_closed() {
    let limits = LineageLimits {
        max_candidates_per_demand: 1,
        ..LineageLimits::default()
    };
    let candidates = vec![
        candidates()[1].clone(),
        candidate(
            "p-extra",
            "d-revision",
            2,
            3,
            location("ontocode-rs/core/src/extra.rs", 1, 4),
        ),
    ];

    assert_eq!(
        merge_candidates(&demands(), &candidates, limits),
        Err(LineageError::TooManyCandidates {
            demand_id: "d-revision".to_string(),
            limit: 1,
            found: 2,
        })
    );
}

#[test]
fn over_limit_merged_passages_fail_closed() {
    let limits = LineageLimits {
        max_passages: 1,
        ..LineageLimits::default()
    };

    assert_eq!(
        merge_candidates(&demands(), &candidates(), limits),
        Err(LineageError::TooManyPassages { limit: 1, found: 2 })
    );
}

#[test]
fn lineage_errors_render_actionable_messages() {
    assert_eq!(
        LineageError::DuplicateDemandId("d-1".to_string()).to_string(),
        "duplicate evidence demand id `d-1`"
    );
    assert_eq!(
        LineageError::UnknownDemandId {
            passage_id: "p-1".to_string(),
            demand_id: "d-9".to_string(),
        }
        .to_string(),
        "candidate `p-1` references unknown demand `d-9`"
    );
    assert_eq!(
        LineageError::ConflictingProvenance {
            passage_id: "p-1".to_string(),
            existing: "a.rs:1-2".to_string(),
            incoming: "a.rs:3-4".to_string(),
        }
        .to_string(),
        "passage `p-1` has conflicting provenance `a.rs:1-2` and `a.rs:3-4`"
    );
    assert_eq!(
        LineageError::TooManyPassages { limit: 2, found: 3 }.to_string(),
        "merged passage count 3 exceeds limit 2"
    );
}
