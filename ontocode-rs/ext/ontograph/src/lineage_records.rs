//! Demand and passage lineage records for demand-aware retrieval (LR-2.0).
//!
//! Retrieval exposes the same passage through several demands. Merging those
//! exposures normally discards which demand found a passage and why it ranked,
//! which makes later grounding unable to tell demand support from coincidence.
//! This module merges candidates while retaining demand identity, first
//! exposure, and provenance, and fails closed on duplicate demands, unknown
//! demand references, and over-limit input.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::evidence_planner::SourceLocation;

/// Upper bounds that keep one merge bounded regardless of caller input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LineageLimits {
    pub(crate) max_demands: usize,
    pub(crate) max_candidates_per_demand: usize,
    pub(crate) max_passages: usize,
}

impl Default for LineageLimits {
    fn default() -> Self {
        Self {
            max_demands: 16,
            max_candidates_per_demand: 64,
            max_passages: 256,
        }
    }
}

/// One evidence demand induced from a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EvidenceDemand {
    pub(crate) demand_id: String,
    pub(crate) description: String,
    /// Typed variable bindings resolved for this demand, such as a symbol name.
    #[serde(default)]
    pub(crate) bindings: BTreeMap<String, String>,
}

/// One retrieved candidate passage, as scored for a single demand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CandidatePassage {
    pub(crate) passage_id: String,
    /// Demand that produced this candidate.
    pub(crate) demand_id: String,
    pub(crate) rank: usize,
    pub(crate) score: usize,
    pub(crate) source_location: SourceLocation,
}

/// The demand exposure that first surfaced a passage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FirstExposure {
    pub(crate) demand_id: String,
    pub(crate) rank: usize,
    pub(crate) score: usize,
}

/// Merged lineage for one passage across every demand that retrieved it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PassageLineage {
    pub(crate) passage_id: String,
    pub(crate) source_location: SourceLocation,
    pub(crate) first_exposure: FirstExposure,
    /// Every demand this passage was retrieved for, including the first.
    pub(crate) demand_membership: BTreeSet<String>,
    /// Retained per-demand rank and score, keyed by demand id.
    pub(crate) exposures: BTreeMap<String, FirstExposure>,
}

impl PassageLineage {
    /// Best rank across demands, used by downstream bounded selection.
    #[cfg(test)]
    pub(crate) fn best_rank(&self) -> usize {
        self.exposures
            .values()
            .map(|exposure| exposure.rank)
            .min()
            .unwrap_or(self.first_exposure.rank)
    }

    /// Highest score across demands.
    pub(crate) fn best_score(&self) -> usize {
        self.exposures
            .values()
            .map(|exposure| exposure.score)
            .max()
            .unwrap_or(self.first_exposure.score)
    }
}

/// Deterministic merge result for one question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LineageLedger {
    pub(crate) demands: Vec<EvidenceDemand>,
    /// Passage lineages in deterministic passage-id order.
    pub(crate) passages: Vec<PassageLineage>,
    /// Demands that retrieved no candidate, retained instead of dropped.
    pub(crate) unresolved_demands: Vec<String>,
}

impl LineageLedger {
    pub(crate) fn lineage_for(&self, passage_id: &str) -> Option<&PassageLineage> {
        self.passages
            .iter()
            .find(|lineage| lineage.passage_id == passage_id)
    }

    /// Passages that satisfy the given demand, in deterministic order.
    #[cfg(test)]
    pub(crate) fn passages_for_demand(&self, demand_id: &str) -> Vec<&PassageLineage> {
        self.passages
            .iter()
            .filter(|lineage| lineage.demand_membership.contains(demand_id))
            .collect()
    }
}

/// Fail-closed rejection reasons for a lineage merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LineageError {
    MissingDemandId,
    DuplicateDemandId(String),
    UnknownDemandId {
        passage_id: String,
        demand_id: String,
    },
    MissingPassageId(String),
    /// The same demand ranked the same passage twice.
    DuplicateCandidate {
        demand_id: String,
        passage_id: String,
    },
    /// The same passage arrived with conflicting provenance.
    ConflictingProvenance {
        passage_id: String,
        existing: String,
        incoming: String,
    },
    InvalidProvenance {
        passage_id: String,
        reason: String,
    },
    TooManyDemands {
        limit: usize,
        found: usize,
    },
    TooManyCandidates {
        demand_id: String,
        limit: usize,
        found: usize,
    },
    TooManyPassages {
        limit: usize,
        found: usize,
    },
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDemandId => write!(f, "evidence demand requires a non-empty id"),
            Self::DuplicateDemandId(id) => write!(f, "duplicate evidence demand id `{id}`"),
            Self::UnknownDemandId {
                passage_id,
                demand_id,
            } => write!(
                f,
                "candidate `{passage_id}` references unknown demand `{demand_id}`"
            ),
            Self::MissingPassageId(demand_id) => write!(
                f,
                "demand `{demand_id}` produced a candidate without a passage id"
            ),
            Self::DuplicateCandidate {
                demand_id,
                passage_id,
            } => write!(
                f,
                "demand `{demand_id}` retrieved passage `{passage_id}` more than once"
            ),
            Self::ConflictingProvenance {
                passage_id,
                existing,
                incoming,
            } => write!(
                f,
                "passage `{passage_id}` has conflicting provenance `{existing}` and `{incoming}`"
            ),
            Self::InvalidProvenance { passage_id, reason } => {
                write!(f, "passage `{passage_id}` has invalid provenance: {reason}")
            }
            Self::TooManyDemands { limit, found } => {
                write!(f, "demand count {found} exceeds limit {limit}")
            }
            Self::TooManyCandidates {
                demand_id,
                limit,
                found,
            } => write!(
                f,
                "demand `{demand_id}` produced {found} candidates, exceeding limit {limit}"
            ),
            Self::TooManyPassages { limit, found } => {
                write!(f, "merged passage count {found} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for LineageError {}

/// Merges per-demand candidates into one lineage ledger.
///
/// The merge is order-independent in its output: passages are emitted in
/// passage-id order and per-demand exposures are keyed by demand id, so the
/// same input always yields the same ledger. First exposure is the earliest
/// candidate in declared demand order, which keeps attribution stable when a
/// passage satisfies several demands.
pub(crate) fn merge_candidates(
    demands: &[EvidenceDemand],
    candidates: &[CandidatePassage],
    limits: LineageLimits,
) -> Result<LineageLedger, LineageError> {
    if demands.len() > limits.max_demands {
        return Err(LineageError::TooManyDemands {
            limit: limits.max_demands,
            found: demands.len(),
        });
    }

    let mut demand_order = BTreeMap::new();
    for (position, demand) in demands.iter().enumerate() {
        if demand.demand_id.trim().is_empty() {
            return Err(LineageError::MissingDemandId);
        }
        if demand_order
            .insert(demand.demand_id.clone(), position)
            .is_some()
        {
            return Err(LineageError::DuplicateDemandId(demand.demand_id.clone()));
        }
    }

    let mut per_demand_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut merged: BTreeMap<String, PassageLineage> = BTreeMap::new();
    let mut first_exposure_order: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for candidate in candidates {
        if candidate.passage_id.trim().is_empty() {
            return Err(LineageError::MissingPassageId(candidate.demand_id.clone()));
        }
        let Some(&demand_position) = demand_order.get(&candidate.demand_id) else {
            return Err(LineageError::UnknownDemandId {
                passage_id: candidate.passage_id.clone(),
                demand_id: candidate.demand_id.clone(),
            });
        };
        candidate
            .source_location
            .validate()
            .map_err(|error| LineageError::InvalidProvenance {
                passage_id: candidate.passage_id.clone(),
                reason: error.to_string(),
            })?;

        let count = per_demand_counts
            .entry(candidate.demand_id.as_str())
            .or_default();
        *count += 1;
        if *count > limits.max_candidates_per_demand {
            return Err(LineageError::TooManyCandidates {
                demand_id: candidate.demand_id.clone(),
                limit: limits.max_candidates_per_demand,
                found: *count,
            });
        }

        let exposure = FirstExposure {
            demand_id: candidate.demand_id.clone(),
            rank: candidate.rank,
            score: candidate.score,
        };

        match merged.get_mut(&candidate.passage_id) {
            Some(lineage) => {
                if lineage.source_location != candidate.source_location {
                    return Err(LineageError::ConflictingProvenance {
                        passage_id: candidate.passage_id.clone(),
                        existing: describe_location(&lineage.source_location),
                        incoming: describe_location(&candidate.source_location),
                    });
                }
                if lineage
                    .exposures
                    .insert(candidate.demand_id.clone(), exposure)
                    .is_some()
                {
                    return Err(LineageError::DuplicateCandidate {
                        demand_id: candidate.demand_id.clone(),
                        passage_id: candidate.passage_id.clone(),
                    });
                }
                lineage
                    .demand_membership
                    .insert(candidate.demand_id.clone());
                let position = (demand_position, candidate.rank);
                let current = first_exposure_order
                    .get(&candidate.passage_id)
                    .copied()
                    .unwrap_or(position);
                if position < current {
                    first_exposure_order.insert(candidate.passage_id.clone(), position);
                    lineage.first_exposure = FirstExposure {
                        demand_id: candidate.demand_id.clone(),
                        rank: candidate.rank,
                        score: candidate.score,
                    };
                }
            }
            None => {
                if merged.len() >= limits.max_passages {
                    return Err(LineageError::TooManyPassages {
                        limit: limits.max_passages,
                        found: merged.len() + 1,
                    });
                }
                first_exposure_order.insert(
                    candidate.passage_id.clone(),
                    (demand_position, candidate.rank),
                );
                merged.insert(
                    candidate.passage_id.clone(),
                    PassageLineage {
                        passage_id: candidate.passage_id.clone(),
                        source_location: candidate.source_location.clone(),
                        first_exposure: exposure.clone(),
                        demand_membership: BTreeSet::from([candidate.demand_id.clone()]),
                        exposures: BTreeMap::from([(candidate.demand_id.clone(), exposure)]),
                    },
                );
            }
        }
    }

    let passages = merged.into_values().collect::<Vec<_>>();
    let unresolved_demands = demands
        .iter()
        .filter(|demand| {
            !passages
                .iter()
                .any(|lineage| lineage.demand_membership.contains(&demand.demand_id))
        })
        .map(|demand| demand.demand_id.clone())
        .collect::<Vec<_>>();

    Ok(LineageLedger {
        demands: demands.to_vec(),
        passages,
        unresolved_demands,
    })
}

fn describe_location(location: &SourceLocation) -> String {
    let path = &location.path;
    match (location.start_line, location.end_line) {
        (Some(start), Some(end)) => format!("{path}:{start}-{end}"),
        (Some(start), None) => format!("{path}:{start}"),
        _ => path.clone(),
    }
}

#[cfg(test)]
#[path = "lineage_records_tests.rs"]
mod tests;
