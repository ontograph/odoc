use pretty_assertions::assert_eq;

use super::ProceduralFallbackReason;
use super::ProceduralGraph;
use super::ProceduralLookup;
use crate::km::KmTransition;

fn transition(
    source: &str,
    target: &str,
    relation: &str,
    condition: &str,
    guidance: &str,
    pitfalls: &[&str],
) -> KmTransition {
    KmTransition {
        source: source.to_string(),
        target: target.to_string(),
        relation: relation.to_string(),
        condition: condition.to_string(),
        guidance: guidance.to_string(),
        pitfalls: pitfalls
            .iter()
            .map(|pitfall| (*pitfall).to_string())
            .collect(),
    }
}

fn assert_bounded_prefix(
    graph: &ProceduralGraph,
    identifier: &str,
    requested_hops: usize,
    max_edges: usize,
    expected_truncated: bool,
) {
    let legacy = graph.project(identifier, requested_hops);
    let (bounded, truncated) = graph.project_bounded(identifier, requested_hops, max_edges);

    match (legacy, bounded) {
        (ProceduralLookup::Localized(legacy), ProceduralLookup::Localized(bounded)) => {
            let prefix_len = max_edges.min(legacy.edges.len());
            assert_eq!(bounded.active_node, legacy.active_node);
            assert_eq!(bounded.edges, legacy.edges[..prefix_len]);
            assert_eq!(truncated, expected_truncated);
        }
        (legacy, bounded) => {
            assert_eq!(bounded, legacy);
            assert!(!truncated);
        }
    }
}

#[test]
fn exact_identifier_localizes_source_node() {
    let graph = ProceduralGraph::from_validated_transitions(vec![transition(
        "inspect",
        "validate",
        "next",
        "ready",
        "validate the state",
        &[],
    )]);

    let result = graph.project("inspect", 1);

    assert_eq!(
        result,
        ProceduralLookup::Localized(super::ProceduralNeighborhood {
            active_node: "inspect".to_string(),
            edges: vec![super::ProceduralEdge {
                transition: transition(
                    "inspect",
                    "validate",
                    "next",
                    "ready",
                    "validate the state",
                    &[],
                ),
                hops: 1,
            }],
        })
    );
}

#[test]
fn bounded_projection_matches_explicit_breadth_first_results() {
    let first_hop = (0..32)
        .map(|index| {
            transition(
                "root",
                &format!("node-{index:02}"),
                "next",
                "ready",
                "go",
                &[],
            )
        })
        .collect::<Vec<_>>();
    let second_hop = transition("node-00", "terminal", "next", "ready", "finish", &[]);
    let mut input = first_hop.clone();
    input.push(second_hop.clone());
    input.reverse();
    let graph = ProceduralGraph::from_validated_transitions(input);
    let mut expected_edges = first_hop
        .into_iter()
        .map(|transition| super::ProceduralEdge {
            transition,
            hops: 1,
        })
        .collect::<Vec<_>>();
    expected_edges.push(super::ProceduralEdge {
        transition: second_hop,
        hops: 2,
    });

    for (capacity, truncated) in [(0, true), (1, true), (32, true), (33, false), (34, false)] {
        assert_eq!(
            graph.project_bounded("root", /*requested_hops*/ 2, capacity),
            (
                ProceduralLookup::Localized(super::ProceduralNeighborhood {
                    active_node: "root".to_string(),
                    edges: expected_edges[..capacity.min(expected_edges.len())].to_vec(),
                }),
                truncated,
            )
        );
    }
}

#[test]
fn target_only_terminal_node_localizes_with_empty_neighborhood() {
    let graph = ProceduralGraph::from_validated_transitions(vec![transition(
        "inspect",
        "validate",
        "next",
        "ready",
        "validate the state",
        &[],
    )]);

    assert_eq!(
        graph.project("validate", 2),
        ProceduralLookup::Localized(super::ProceduralNeighborhood {
            active_node: "validate".to_string(),
            edges: Vec::new(),
        })
    );
}

#[test]
fn unknown_identifier_returns_explicit_fallback() {
    let graph = ProceduralGraph::from_validated_transitions(vec![transition(
        "inspect",
        "validate",
        "next",
        "ready",
        "validate the state",
        &[],
    )]);

    assert_eq!(
        graph.project("Inspect", 1),
        ProceduralLookup::Fallback {
            reason: ProceduralFallbackReason::UnknownIdentifier,
        }
    );
}

#[test]
fn empty_graph_returns_explicit_fallback() {
    assert_eq!(
        ProceduralGraph::default().project("inspect", 1),
        ProceduralLookup::Fallback {
            reason: ProceduralFallbackReason::EmptyGraph,
        }
    );
}

#[test]
fn one_hop_excludes_second_hop_edges() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("inspect", "validate", "next", "ready", "validate", &[]),
        transition("validate", "report", "next", "valid", "report", &[]),
    ]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("inspect", 1) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(neighborhood.edges.len(), 1);
    assert_eq!(neighborhood.edges[0].hops, 1);
    assert_eq!(neighborhood.edges[0].transition.target, "validate");
}

#[test]
fn requested_hops_are_capped_at_two() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("a", "b", "next", "", "", &[]),
        transition("b", "c", "next", "", "", &[]),
        transition("c", "d", "next", "", "", &[]),
    ]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("a", 99) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(
        neighborhood
            .edges
            .iter()
            .map(|edge| (edge.transition.target.as_str(), edge.hops))
            .collect::<Vec<_>>(),
        vec![("b", 1), ("c", 2)]
    );
}

#[test]
fn input_order_does_not_change_output_order() {
    let first = ProceduralGraph::from_validated_transitions(vec![
        transition("inspect", "z", "next", "", "", &[]),
        transition("inspect", "a", "next", "", "", &[]),
    ]);
    let second = ProceduralGraph::from_validated_transitions(vec![
        transition("inspect", "a", "next", "", "", &[]),
        transition("inspect", "z", "next", "", "", &[]),
    ]);

    let first_result = first.project("inspect", 1);
    let second_result = second.project("inspect", 1);

    assert_eq!(first_result, second_result);
}

#[test]
fn exact_duplicate_transitions_are_coalesced() {
    let edge = transition("inspect", "validate", "next", "ready", "validate", &[]);
    let graph = ProceduralGraph::from_validated_transitions(vec![edge.clone(), edge]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("inspect", 1) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(neighborhood.edges.len(), 1);
}

#[test]
fn same_endpoints_with_different_attributes_are_preserved() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("inspect", "validate", "next", "ready", "validate", &[]),
        transition(
            "inspect",
            "validate",
            "retry",
            "invalid",
            "inspect again",
            &["stale state"],
        ),
    ]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("inspect", 1) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(neighborhood.edges.len(), 2);
    assert_eq!(neighborhood.edges[0].transition.relation, "next");
    assert_eq!(neighborhood.edges[1].transition.relation, "retry");
}

#[test]
fn cycles_do_not_repeat_or_traverse_unbounded() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("a", "b", "next", "", "", &[]),
        transition("b", "a", "next", "", "", &[]),
        transition("b", "c", "next", "", "", &[]),
    ]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("a", 2) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(neighborhood.edges.len(), 3);
    assert_eq!(
        neighborhood
            .edges
            .iter()
            .map(|edge| edge.hops)
            .collect::<Vec<_>>(),
        vec![1, 2, 2]
    );
}

#[test]
fn self_loop_does_not_requeue_the_active_node() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("inspect", "inspect", "repeat", "", "", &[]),
        transition("inspect", "validate", "next", "", "", &[]),
    ]);

    let ProceduralLookup::Localized(neighborhood) = graph.project("inspect", 2) else {
        panic!("expected localized neighborhood");
    };

    assert_eq!(neighborhood.edges.len(), 2);
    assert!(neighborhood.edges.iter().all(|edge| edge.hops == 1));
}

#[test]
fn bounded_projection_matches_legacy_prefix_at_zero_exact_and_overflow_capacity() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("a", "b", "next", "", "", &[]),
        transition("a", "c", "next", "", "", &[]),
        transition("b", "d", "next", "", "", &[]),
    ]);

    assert_bounded_prefix(&graph, "a", 2, 0, true);
    assert_bounded_prefix(&graph, "a", 2, 3, false);
    assert_bounded_prefix(&graph, "a", 2, 2, true);
}

#[test]
fn exact_capacity_is_not_truncated_when_no_reachable_edge_remains() {
    let graph = ProceduralGraph::from_validated_transitions(vec![transition(
        "a",
        "b",
        "next",
        "",
        "",
        &[],
    )]);

    assert_bounded_prefix(&graph, "a", 1, 1, false);
}

#[test]
fn unbounded_capacity_preserves_legacy_lookup() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("a", "b", "next", "", "", &[]),
        transition("b", "c", "next", "", "", &[]),
    ]);

    let legacy = graph.project("a", 2);
    let (bounded, truncated) = graph.project_bounded("a", 2, usize::MAX);

    assert_eq!(bounded, legacy);
    assert!(!truncated);
}

#[test]
fn bounded_projection_preserves_cycle_and_self_loop_prefixes() {
    let graph = ProceduralGraph::from_validated_transitions(vec![
        transition("a", "a", "repeat", "", "", &[]),
        transition("a", "b", "next", "", "", &[]),
        transition("b", "a", "return", "", "", &[]),
        transition("b", "c", "next", "", "", &[]),
    ]);

    assert_bounded_prefix(&graph, "a", 2, 1, true);
    assert_bounded_prefix(&graph, "a", 2, 3, true);
    assert_bounded_prefix(&graph, "a", 2, 4, false);
}

#[test]
fn bounded_projection_is_deterministic_for_reversed_input() {
    let transitions = vec![
        transition("a", "b", "next", "", "", &[]),
        transition("a", "c", "next", "", "", &[]),
        transition("b", "d", "next", "", "", &[]),
    ];
    let reversed = transitions.iter().cloned().rev().collect();
    let first = ProceduralGraph::from_validated_transitions(transitions);
    let second = ProceduralGraph::from_validated_transitions(reversed);

    assert_eq!(
        first.project_bounded("a", 2, 2),
        second.project_bounded("a", 2, 2)
    );
}

#[test]
fn bounded_projection_keeps_zero_hop_and_fallback_behavior_compatible() {
    let graph = ProceduralGraph::from_validated_transitions(vec![transition(
        "a",
        "b",
        "next",
        "",
        "",
        &[],
    )]);

    assert_bounded_prefix(&graph, "a", 0, 0, false);
    assert_bounded_prefix(&graph, "b", 2, 0, false);
    assert_bounded_prefix(&graph, "unknown", 2, 0, false);
    assert_eq!(
        ProceduralGraph::default().project_bounded("a", 2, 0),
        (
            ProceduralLookup::Fallback {
                reason: ProceduralFallbackReason::EmptyGraph,
            },
            false,
        )
    );
}
