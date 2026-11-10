use pretty_assertions::assert_eq;

use super::*;
use crate::evidence_planner::SourceLocation;
use crate::lineage_records::CandidatePassage;
use crate::lineage_records::EvidenceDemand;
use crate::lineage_records::LineageLimits;
use crate::lineage_records::merge_candidates;
use crate::lineage_selection::SelectionBudget;
use crate::lineage_selection::select_evidence;

const DISPATCH_PATH: &str = "ontocode-rs/core/src/dispatch.rs";
const REVISION_PATH: &str = "ontocode-rs/core/src/revision.rs";
const GENERATION: &str = "gen-0001";

fn demand(demand_id: &str) -> EvidenceDemand {
    EvidenceDemand {
        demand_id: demand_id.to_string(),
        description: format!("demand {demand_id}"),
        bindings: BTreeMap::new(),
    }
}

fn candidate(
    passage_id: &str,
    demand_id: &str,
    path: &str,
    start_line: usize,
    end_line: usize,
) -> CandidatePassage {
    CandidatePassage {
        passage_id: passage_id.to_string(),
        demand_id: demand_id.to_string(),
        rank: 1,
        score: 5,
        source_location: SourceLocation {
            path: path.to_string(),
            start_line: Some(start_line),
            end_line: Some(end_line),
        },
    }
}

fn ledger() -> LineageLedger {
    let demands = vec![demand("d-dispatch"), demand("d-revision")];
    let candidates = vec![
        candidate("p-dispatch", "d-dispatch", DISPATCH_PATH, 2, 3),
        candidate("p-revision", "d-revision", REVISION_PATH, 1, 2),
    ];
    merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge")
}

fn selection(ledger: &LineageLedger) -> SelectionReport {
    let costs = BTreeMap::from([
        ("p-dispatch".to_string(), 10),
        ("p-revision".to_string(), 10),
    ]);
    select_evidence(ledger, &costs, SelectionBudget::default()).expect("selection")
}

fn generation() -> SourceGeneration {
    SourceGeneration::from_files(vec![
        SourceGenerationFile {
            path: DISPATCH_PATH.to_string(),
            generation_id: GENERATION.to_string(),
            lines: vec![
                "// header".to_string(),
                "fn reserve_lease() {".to_string(),
                "    record_dispatch_revision();".to_string(),
                "}".to_string(),
            ],
        },
        SourceGenerationFile {
            path: REVISION_PATH.to_string(),
            generation_id: GENERATION.to_string(),
            lines: vec![
                "fn tracking_revision() -> Revision {".to_string(),
                "    hash_tracking_file()".to_string(),
            ],
        },
    ])
}

fn claim(passage_id: &str, demand_id: &str, quoted_text: &str) -> GroundingClaim {
    GroundingClaim {
        passage_id: passage_id.to_string(),
        demand_id: demand_id.to_string(),
        observed_generation_id: GENERATION.to_string(),
        quoted_text: quoted_text.to_string(),
    }
}

#[test]
fn contained_text_under_the_retrieving_demand_is_usable() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let claims = vec![
        claim("p-dispatch", "d-dispatch", "record_dispatch_revision"),
        claim("p-revision", "d-revision", "hash_tracking_file"),
    ];

    let report = ground_claims(&ledger, &selection, &generation(), &claims).expect("grounding");

    assert_eq!(report.usable_claims().len(), 2);
    assert!(!report.has_unresolved_lineage());
    assert_eq!(
        report.claims[0],
        GroundedClaim {
            passage_id: "p-dispatch".to_string(),
            demand_id: "d-dispatch".to_string(),
            source_span: "ontocode-rs/core/src/dispatch.rs:2-3".to_string(),
            containment: ContainmentStatus::Contained,
            demand_support: DemandSupport::Retrieved,
        }
    );
}

#[test]
fn text_outside_the_span_is_not_contained() {
    let ledger = ledger();
    let selection = selection(&ledger);
    // `// header` is line 1; the recorded span starts at line 2.
    let claims = vec![claim("p-dispatch", "d-dispatch", "// header")];

    let report = ground_claims(&ledger, &selection, &generation(), &claims).expect("grounding");

    assert_eq!(
        report.claims[0].containment,
        ContainmentStatus::NotContained
    );
    assert!(!report.claims[0].is_usable());
    assert!(report.usable_claims().is_empty());
    assert!(report.has_unresolved_lineage());
}

#[test]
fn containment_alone_does_not_establish_demand_support() {
    let ledger = ledger();
    let selection = selection(&ledger);
    // The quoted text really is inside the span, but the passage was retrieved
    // for `d-dispatch`, not for the demand it is cited under here.
    let claims = vec![claim(
        "p-dispatch",
        "d-revision",
        "record_dispatch_revision",
    )];

    let report = ground_claims(&ledger, &selection, &generation(), &claims).expect("grounding");

    assert_eq!(report.claims[0].containment, ContainmentStatus::Contained);
    assert_eq!(report.claims[0].demand_support, DemandSupport::WrongDemand);
    assert!(!report.claims[0].is_usable());
}

#[test]
fn uncovered_demands_are_carried_into_the_grounding_report() {
    let demands = vec![demand("d-dispatch"), demand("d-orphan")];
    let candidates = vec![candidate("p-dispatch", "d-dispatch", DISPATCH_PATH, 2, 3)];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-dispatch".to_string(), 10)]);
    let selection =
        select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");
    let claims = vec![claim(
        "p-dispatch",
        "d-dispatch",
        "record_dispatch_revision",
    )];

    let report = ground_claims(&ledger, &selection, &generation(), &claims).expect("grounding");

    assert_eq!(report.unresolved_demands, vec!["d-orphan".to_string()]);
    assert!(report.has_unresolved_lineage());
    assert_eq!(report.usable_claims().len(), 1);
}

#[test]
fn grounding_is_deterministic_and_ordered() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let forward = vec![
        claim("p-revision", "d-revision", "hash_tracking_file"),
        claim("p-dispatch", "d-dispatch", "record_dispatch_revision"),
    ];
    let mut reversed = forward.clone();
    reversed.reverse();

    let first = ground_claims(&ledger, &selection, &generation(), &forward).expect("grounding");
    let second = ground_claims(&ledger, &selection, &generation(), &reversed).expect("grounding");

    assert_eq!(first, second);
    assert_eq!(
        first
            .claims
            .iter()
            .map(|entry| entry.passage_id.as_str())
            .collect::<Vec<_>>(),
        vec!["p-dispatch", "p-revision"]
    );
}

#[test]
fn stale_source_fails_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let mut generation = generation();
    generation
        .files
        .get_mut(DISPATCH_PATH)
        .expect("file")
        .generation_id = "gen-0002".to_string();
    let claims = vec![claim(
        "p-dispatch",
        "d-dispatch",
        "record_dispatch_revision",
    )];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation, &claims),
        Err(GroundingError::StaleSource {
            passage_id: "p-dispatch".to_string(),
            observed: GENERATION.to_string(),
            active: "gen-0002".to_string(),
        })
    );
}

#[test]
fn missing_source_file_fails_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let mut generation = generation();
    generation.files.remove(DISPATCH_PATH);
    let claims = vec![claim(
        "p-dispatch",
        "d-dispatch",
        "record_dispatch_revision",
    )];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation, &claims),
        Err(GroundingError::MissingSourceFile {
            passage_id: "p-dispatch".to_string(),
            path: DISPATCH_PATH.to_string(),
        })
    );
}

#[test]
fn span_past_the_end_of_the_file_fails_closed() {
    let demands = vec![demand("d-dispatch")];
    let candidates = vec![candidate("p-dispatch", "d-dispatch", DISPATCH_PATH, 2, 40)];
    let ledger = merge_candidates(&demands, &candidates, LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-dispatch".to_string(), 10)]);
    let selection =
        select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");
    let claims = vec![claim(
        "p-dispatch",
        "d-dispatch",
        "record_dispatch_revision",
    )];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::SpanOutOfRange {
            passage_id: "p-dispatch".to_string(),
            start_line: 2,
            end_line: 40,
            available_lines: 4,
        })
    );
}

#[test]
fn malformed_span_fails_closed() {
    let demands = vec![demand("d-dispatch")];
    let mut candidate = candidate("p-dispatch", "d-dispatch", DISPATCH_PATH, 2, 3);
    candidate.source_location.end_line = None;
    let ledger = merge_candidates(&demands, &[candidate], LineageLimits::default()).expect("merge");
    let costs = BTreeMap::from([("p-dispatch".to_string(), 10)]);
    let selection =
        select_evidence(&ledger, &costs, SelectionBudget::default()).expect("selection");
    let claims = vec![claim(
        "p-dispatch",
        "d-dispatch",
        "record_dispatch_revision",
    )];

    assert!(matches!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::MalformedSpan { .. })
    ));
}

#[test]
fn unselected_passage_fails_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let claims = vec![claim("p-absent", "d-dispatch", "anything")];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::UnselectedPassage("p-absent".to_string()))
    );
}

#[test]
fn unknown_demand_fails_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let claims = vec![claim("p-dispatch", "d-absent", "record_dispatch_revision")];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::UnknownDemand {
            passage_id: "p-dispatch".to_string(),
            demand_id: "d-absent".to_string(),
        })
    );
}

#[test]
fn duplicate_claims_fail_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let claims = vec![
        claim("p-dispatch", "d-dispatch", "record_dispatch_revision"),
        claim("p-dispatch", "d-dispatch", "reserve_lease"),
    ];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::DuplicateClaim {
            passage_id: "p-dispatch".to_string(),
            demand_id: "d-dispatch".to_string(),
        })
    );
}

#[test]
fn empty_quoted_text_fails_closed() {
    let ledger = ledger();
    let selection = selection(&ledger);
    let claims = vec![claim("p-dispatch", "d-dispatch", "   ")];

    assert_eq!(
        ground_claims(&ledger, &selection, &generation(), &claims),
        Err(GroundingError::EmptyQuotedText("p-dispatch".to_string()))
    );
}

#[test]
fn grounding_errors_render_actionable_messages() {
    assert_eq!(
        GroundingError::UnselectedPassage("p-1".to_string()).to_string(),
        "claim cites passage `p-1` that was not selected"
    );
    assert_eq!(
        GroundingError::StaleSource {
            passage_id: "p-1".to_string(),
            observed: "gen-1".to_string(),
            active: "gen-2".to_string(),
        }
        .to_string(),
        "passage `p-1` was retrieved at generation `gen-1` but the active generation is `gen-2`"
    );
    assert_eq!(
        GroundingError::SpanOutOfRange {
            passage_id: "p-1".to_string(),
            start_line: 2,
            end_line: 9,
            available_lines: 4,
        }
        .to_string(),
        "passage `p-1` span 2-9 exceeds the 4 available lines"
    );
}
