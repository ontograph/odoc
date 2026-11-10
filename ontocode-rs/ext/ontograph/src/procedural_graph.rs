use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

use crate::km::KmTransition;

pub(crate) const MAX_REQUESTED_HOPS: usize = 2;

/// A frozen, read-only projection over already validated KM transitions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProceduralGraph {
    nodes: BTreeSet<String>,
    outgoing: BTreeMap<String, Vec<KmTransition>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProceduralNeighborhood {
    pub(crate) active_node: String,
    pub(crate) edges: Vec<ProceduralEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProceduralEdge {
    pub(crate) transition: KmTransition,
    pub(crate) hops: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProceduralFallbackReason {
    EmptyGraph,
    UnknownIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProceduralLookup {
    Localized(ProceduralNeighborhood),
    Fallback { reason: ProceduralFallbackReason },
}

impl ProceduralGraph {
    /// Builds a projection from records that have already passed KM validation.
    ///
    /// The graph owns a snapshot of its input. Validation and provenance remain
    /// the responsibility of the KM loader and store.
    pub(crate) fn from_validated_transitions(transitions: Vec<KmTransition>) -> Self {
        let mut graph = Self::default();
        let mut transitions = transitions;
        transitions.sort_by(compare_transitions);
        transitions.dedup();

        for transition in transitions {
            graph.nodes.insert(transition.source.clone());
            graph.nodes.insert(transition.target.clone());
            graph
                .outgoing
                .entry(transition.source.clone())
                .or_default()
                .push(transition);
        }

        graph
    }

    /// Bounded projection of the procedural neighborhood around `identifier`.
    ///
    /// Traversal is deterministic breadth-first, capped at `MAX_REQUESTED_HOPS` (2).
    /// Returns a `(ProceduralLookup, bool)` tuple where:
    /// - `ProceduralLookup`: the localized neighborhood or fallback reason.
    /// - `bool`: truncation flag reporting whether any reachable edge within the requested
    ///   hop radius was omitted due to `max_edges`. Truncation is `true` if at least one
    ///   reachable edge was omitted, and `false` if all reachable edges fit within `max_edges`
    ///   (including at exact capacity when no further reachable edges exist, or on fallback).
    pub(crate) fn project_bounded(
        &self,
        identifier: &str,
        requested_hops: usize,
        max_edges: usize,
    ) -> (ProceduralLookup, bool) {
        if self.nodes.is_empty() {
            return (
                ProceduralLookup::Fallback {
                    reason: ProceduralFallbackReason::EmptyGraph,
                },
                /*truncated*/ false,
            );
        }
        if !self.nodes.contains(identifier) {
            return (
                ProceduralLookup::Fallback {
                    reason: ProceduralFallbackReason::UnknownIdentifier,
                },
                /*truncated*/ false,
            );
        }

        let max_hops = requested_hops.min(MAX_REQUESTED_HOPS);
        let max_nodes = max_edges.saturating_add(1);
        let mut edges = Vec::new();
        let mut visited_nodes = BTreeSet::from([identifier.to_string()]);
        let mut pending = VecDeque::from([(identifier.to_string(), 0usize)]);
        let mut truncated = false;

        'traversal: while let Some((source, source_hops)) = pending.pop_front() {
            if source_hops >= max_hops {
                continue;
            }

            let edge_hops = source_hops + 1;
            for transition in self.outgoing.get(&source).into_iter().flatten() {
                if edges.len() >= max_edges {
                    truncated = true;
                    break 'traversal;
                }

                edges.push(ProceduralEdge {
                    transition: transition.clone(),
                    hops: edge_hops,
                });

                if visited_nodes.len() < max_nodes
                    && visited_nodes.insert(transition.target.clone())
                    && pending.len() < max_nodes
                {
                    pending.push_back((transition.target.clone(), edge_hops));
                }
            }
        }

        (
            ProceduralLookup::Localized(ProceduralNeighborhood {
                active_node: identifier.to_string(),
                edges,
            }),
            truncated,
        )
    }

    pub(crate) fn project(&self, identifier: &str, requested_hops: usize) -> ProceduralLookup {
        self.project_bounded(identifier, requested_hops, /*max_edges*/ usize::MAX)
            .0
    }
}

fn compare_transitions(left: &KmTransition, right: &KmTransition) -> Ordering {
    left.source
        .cmp(&right.source)
        .then_with(|| left.target.cmp(&right.target))
        .then_with(|| left.relation.cmp(&right.relation))
        .then_with(|| left.condition.cmp(&right.condition))
        .then_with(|| left.guidance.cmp(&right.guidance))
        .then_with(|| left.pitfalls.cmp(&right.pitfalls))
}

#[cfg(test)]
#[path = "procedural_graph_tests.rs"]
mod tests;
