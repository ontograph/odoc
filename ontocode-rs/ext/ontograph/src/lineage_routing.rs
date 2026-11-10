//! Opt-in routing between the baseline path and the lineage path (LR-5.0).
//!
//! The lineage path costs more than baseline retrieval, so it is reserved for
//! questions that need it: multi-hop questions and low-confidence baseline
//! answers. Routing is off unless explicitly enabled, and every induction,
//! retrieval, selection, grounding, timeout, or schema failure falls back to
//! the baseline answer with a recorded reason rather than failing the turn.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::lineage_grounding::GroundingReport;
use crate::lineage_grounding::SourceGeneration;
use crate::lineage_records::LineageError;
use crate::lineage_records::LineageLedger;
use crate::lineage_selection::SelectionError;
use crate::lineage_selection::SelectionReport;
use ontocode_protocol::user_input::UserInput;

const MAX_INDUCTION_TEXT_CHARS: usize = 16_384;
const MAX_INDUCED_DEMANDS: usize = 8;
/// Confidence assigned to a question with no hedging language.
const BASE_CONFIDENCE_BP: usize = 9_500;
/// Confidence removed per distinct hedge word.
const HEDGE_PENALTY_BP: usize = 1_500;
/// Hedges counted beyond this add no further penalty.
const MAX_HEDGE_HITS: usize = 6;

/// True when `needle` appears in `haystack` as a whole word.
///
/// Substring matching would let identifiers drive routing: "shoulder" contains
/// "could" and "unsure_flag" contains "unsure", and each false hit silently
/// lowers confidence toward the expensive path. Both inputs are already
/// lowercased by the caller.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let is_boundary = |ch: Option<char>| match ch {
        None => true,
        Some(ch) => !ch.is_alphanumeric() && ch != '_',
    };
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before = haystack[..start].chars().next_back();
        let after = haystack[end..].chars().next();
        if is_boundary(before) && is_boundary(after) {
            return true;
        }
        // Advance by one character so overlapping candidates are still checked.
        let Some(next) = haystack[start..].chars().next().map(char::len_utf8) else {
            break;
        };
        from = start + next;
    }
    false
}

/// Caller-controlled routing configuration. Disabled by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RoutingConfig {
    /// Master switch. When false, every question stays on the baseline path.
    pub(crate) enabled: bool,
    /// Minimum induced demands before a question counts as multi-hop.
    pub(crate) multi_hop_demand_threshold: usize,
    /// Baseline confidence, in basis points, below which routing is allowed.
    pub(crate) low_confidence_threshold_bp: usize,
    pub(crate) max_latency_ms: u64,
    pub(crate) max_token_cost: usize,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            multi_hop_demand_threshold: 2,
            low_confidence_threshold_bp: 7_000,
            max_latency_ms: 10_000,
            max_token_cost: 8_192,
        }
    }
}

/// What the baseline path already produced for this question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BaselineOutcome {
    pub(crate) induced_demands: usize,
    pub(crate) confidence_bp: usize,
}

/// Counts the clause boundaries joined by connectives.
///
/// A demand is added per join, not per marker. Phrases like "and then" contain
/// two markers but join exactly one extra clause, so directly adjacent
/// connectives collapse into a single boundary. Scanning left to right, rather
/// than testing each marker independently, also lets a repeated connective such
/// as "A and B and C" contribute once per occurrence.
fn count_clause_boundaries(clause: &str, markers: &[&str]) -> usize {
    let mut count = 0;
    let mut rest = clause;
    while !rest.is_empty() {
        let Some((at, len)) = markers
            .iter()
            .filter_map(|marker| rest.find(marker).map(|at| (at, marker.len())))
            .min()
        else {
            break;
        };
        count += 1;
        // Re-include the trailing space so an adjacent connective still matches.
        rest = &rest[at + len - 1..];
        // Collapse directly adjacent connectives into the same boundary.
        while let Some(len) = markers
            .iter()
            .filter(|marker| rest.starts_with(**marker))
            .map(|marker| marker.len())
            .max()
        {
            rest = &rest[len - 1..];
        }
    }
    count
}

/// Derives bounded routing facts from the turn-input seam without provider work.
pub(crate) fn induce_baseline_outcome(user_input: &[UserInput]) -> Result<BaselineOutcome, String> {
    let mut text = String::new();
    for item in user_input {
        let UserInput::Text { text: value, .. } = item else {
            continue;
        };
        if text.len().saturating_add(value.len()) > MAX_INDUCTION_TEXT_CHARS {
            return Err("turn input exceeds local induction bound".to_string());
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(value);
    }
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err("no textual baseline facts available".to_string());
    }

    let clauses = normalized
        .split(['?', '!', ';', '.', '\n'])
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .collect::<Vec<_>>();
    let conjunctions = [" and ", " then ", " after ", " before ", " while "];
    let clause_count = clauses
        .iter()
        .map(|clause| 1 + count_clause_boundaries(clause, &conjunctions))
        .sum::<usize>();
    let induced_demands = clause_count.clamp(1, MAX_INDUCED_DEMANDS);
    let uncertainty_markers = [
        "maybe",
        "unclear",
        "uncertain",
        "not sure",
        "unsure",
        "could",
        "possibly",
    ];
    let uncertainty_hits = uncertainty_markers
        .iter()
        .filter(|marker| contains_word(&normalized, marker))
        .count();
    let confidence_bp =
        BASE_CONFIDENCE_BP.saturating_sub(HEDGE_PENALTY_BP * uncertainty_hits.min(MAX_HEDGE_HITS));

    Ok(BaselineOutcome {
        induced_demands,
        confidence_bp,
    })
}

/// Observed cost of one lineage attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LineageCost {
    pub(crate) elapsed_ms: u64,
    pub(crate) token_cost: usize,
}

/// Result of one lineage attempt, as reported by the caller.
#[allow(dead_code)] // outcome variants are reserved for the live provider path
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LineageAttempt {
    /// Retrieval and bounded selection succeeded. Grounding has not run,
    /// because the turn-input seam has no assistant claims to ground against.
    Selected {
        ledger: LineageLedger,
        selection: SelectionReport,
        /// Source generation observed while retrieving, carried so grounding
        /// can detect a source that changed after selection.
        generation: SourceGeneration,
        cost: LineageCost,
    },
    Completed {
        grounding: GroundingReport,
        cost: LineageCost,
    },
    InductionFailed(String),
    RetrievalFailed(LineageError),
    /// Retrieval could not produce usable candidates because the backing KM
    /// store failed or returned no matching spans.
    RetrievalUnavailable(String),
    SelectionFailed(SelectionError),
    GroundingFailed(String),
    SchemaInvalid(String),
    TimedOut {
        elapsed_ms: u64,
    },
}

/// Why a question stayed on, or returned to, the baseline path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FallbackReason {
    /// Routing is not enabled.
    Disabled,
    /// The question is neither multi-hop nor low-confidence.
    NotEligible {
        induced_demands: usize,
        confidence_bp: usize,
    },
    InductionFailed(String),
    RetrievalFailed(String),
    SelectionFailed(String),
    GroundingFailed(String),
    SchemaInvalid(String),
    Timeout {
        elapsed_ms: u64,
        limit_ms: u64,
    },
    /// The caller's budget could not cover the configured worst-case latency,
    /// so no attempt was started. Distinct from `Timeout`, which reports an
    /// attempt that actually overran.
    BudgetExhausted {
        remaining_ms: u64,
        required_ms: u64,
    },
    TokenCapExceeded {
        token_cost: usize,
        limit: usize,
    },
    /// Grounding produced usable claims but left demands uncovered. Distinct
    /// from `NoUsableEvidence`, which means nothing grounded at all.
    PartialCoverage {
        unresolved_demands: Vec<String>,
    },
    /// The lineage path ran but produced no usable grounded claim.
    NoUsableEvidence,
}

impl fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "lineage routing is disabled"),
            Self::NotEligible {
                induced_demands,
                confidence_bp,
            } => write!(
                f,
                "question is single-hop with {induced_demands} demand(s) and confidence {confidence_bp} bp"
            ),
            Self::InductionFailed(reason) => write!(f, "demand induction failed: {reason}"),
            Self::RetrievalFailed(reason) => write!(f, "retrieval failed: {reason}"),
            Self::SelectionFailed(reason) => write!(f, "selection failed: {reason}"),
            Self::GroundingFailed(reason) => write!(f, "grounding failed: {reason}"),
            Self::SchemaInvalid(reason) => write!(f, "lineage schema invalid: {reason}"),
            Self::Timeout {
                elapsed_ms,
                limit_ms,
            } => write!(
                f,
                "lineage path took {elapsed_ms}ms, exceeding {limit_ms}ms"
            ),
            Self::BudgetExhausted {
                remaining_ms,
                required_ms,
            } => write!(
                f,
                "caller budget {remaining_ms}ms cannot cover the {required_ms}ms latency cap"
            ),
            Self::TokenCapExceeded { token_cost, limit } => write!(
                f,
                "lineage path used {token_cost} tokens, exceeding the cap {limit}"
            ),
            Self::PartialCoverage { unresolved_demands } => write!(
                f,
                "lineage path left {} demand(s) uncovered",
                unresolved_demands.len()
            ),
            Self::NoUsableEvidence => {
                write!(f, "lineage path produced no usable grounded evidence")
            }
        }
    }
}

/// Which path answered the question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RoutingDecision {
    /// The baseline answer stands. A reason is always recorded.
    Baseline { reason: FallbackReason },
    /// Retrieval and selection produced evidence that is not yet grounded.
    ///
    /// This is the terminal decision at the turn-input seam. Answer
    /// eligibility is decided later, where assistant claims exist.
    Selected {
        ledger: LineageLedger,
        selection: SelectionReport,
        generation: SourceGeneration,
        cost: LineageCost,
    },
    /// The lineage path answered with usable grounded evidence.
    Lineage {
        usable_claims: usize,
        unresolved_demands: Vec<String>,
        cost: LineageCost,
    },
}

impl RoutingDecision {
    #[cfg(test)]
    pub(crate) fn used_lineage(&self) -> bool {
        matches!(self, Self::Lineage { .. })
    }

    #[cfg(test)]
    pub(crate) fn fallback_reason(&self) -> Option<&FallbackReason> {
        match self {
            Self::Baseline { reason } => Some(reason),
            Self::Selected { .. } | Self::Lineage { .. } => None,
        }
    }
}

/// Latency budget a caller grants to one lineage attempt.
///
/// Only a zero remaining budget refuses before the attempt starts. Otherwise
/// [`route`] runs the attempt under the smaller of the configured latency cap
/// and remaining allowance; an overrun is reported as `Timeout`.
///
/// This is a caller-supplied allowance, not a wall-clock deadline: the
/// turn-input seam exposes no turn start time, so nothing here decreases as the
/// turn progresses. A true deadline needs an elapsed-time source at the seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineageBudget {
    pub(crate) remaining_ms: u64,
}

impl LineageBudget {
    pub(crate) fn from_remaining_ms(remaining_ms: u64) -> Self {
        Self { remaining_ms }
    }

    /// True when no time is left to start an attempt at all.
    ///
    /// A partial allowance is usable: the attempt then runs under the smaller of
    /// the configured cap and what remains, so a host deadline equal to the cap
    /// (which has always lost a few milliseconds by the time routing runs) does
    /// not make the lineage path permanently unreachable.
    fn is_exhausted(self, config: RoutingConfig) -> bool {
        self.remaining_ms == 0 || config.max_latency_ms == 0
    }
}

/// Decides whether a question is eligible for the lineage path.
///
/// Eligibility is checked before any lineage work runs, so an ineligible
/// question never pays the extra retrieval cost.
pub(crate) fn is_eligible(config: RoutingConfig, baseline: &BaselineOutcome) -> bool {
    config.enabled
        && (baseline.induced_demands >= config.multi_hop_demand_threshold
            || baseline.confidence_bp < config.low_confidence_threshold_bp)
}

/// Routes one question, falling back to baseline on any failure or cap breach.
///
/// The attempt is supplied by the caller rather than executed here, which keeps
/// this decision function free of provider and I/O concerns and makes every
/// fallback path directly testable.
pub(crate) fn route(
    config: RoutingConfig,
    baseline: &BaselineOutcome,
    budget: LineageBudget,
    attempt: impl FnOnce() -> LineageAttempt,
) -> RoutingDecision {
    if !config.enabled {
        return RoutingDecision::Baseline {
            reason: FallbackReason::Disabled,
        };
    }
    if !is_eligible(config, baseline) {
        return RoutingDecision::Baseline {
            reason: FallbackReason::NotEligible {
                induced_demands: baseline.induced_demands,
                confidence_bp: baseline.confidence_bp,
            },
        };
    }
    // Checked before the attempt runs, so an unaffordable turn costs nothing.
    if budget.is_exhausted(config) {
        return RoutingDecision::Baseline {
            reason: FallbackReason::BudgetExhausted {
                remaining_ms: budget.remaining_ms,
                required_ms: config.max_latency_ms,
            },
        };
    }

    // The attempt may use only what the caller still has, never more than the cap.
    let config = RoutingConfig {
        max_latency_ms: config.max_latency_ms.min(budget.remaining_ms),
        ..config
    };

    match attempt() {
        LineageAttempt::Selected {
            ledger,
            selection,
            generation,
            cost,
        } => {
            if cost.elapsed_ms > config.max_latency_ms {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::Timeout {
                        elapsed_ms: cost.elapsed_ms,
                        limit_ms: config.max_latency_ms,
                    },
                };
            }
            if cost.token_cost > config.max_token_cost {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::TokenCapExceeded {
                        token_cost: cost.token_cost,
                        limit: config.max_token_cost,
                    },
                };
            }
            // Selection that admitted nothing carries no evidence forward.
            if selection.selected.is_empty() {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                };
            }
            RoutingDecision::Selected {
                ledger,
                selection,
                generation,
                cost,
            }
        }
        LineageAttempt::InductionFailed(reason) => RoutingDecision::Baseline {
            reason: FallbackReason::InductionFailed(reason),
        },
        LineageAttempt::RetrievalFailed(error) => RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(error.to_string()),
        },
        LineageAttempt::RetrievalUnavailable(reason) => RoutingDecision::Baseline {
            reason: FallbackReason::RetrievalFailed(reason),
        },
        LineageAttempt::SelectionFailed(error) => RoutingDecision::Baseline {
            reason: FallbackReason::SelectionFailed(error.to_string()),
        },
        LineageAttempt::GroundingFailed(reason) => RoutingDecision::Baseline {
            reason: FallbackReason::GroundingFailed(reason),
        },
        LineageAttempt::SchemaInvalid(reason) => RoutingDecision::Baseline {
            reason: FallbackReason::SchemaInvalid(reason),
        },
        LineageAttempt::TimedOut { elapsed_ms } => RoutingDecision::Baseline {
            reason: FallbackReason::Timeout {
                elapsed_ms,
                limit_ms: config.max_latency_ms,
            },
        },
        LineageAttempt::Completed { grounding, cost } => {
            if cost.elapsed_ms > config.max_latency_ms {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::Timeout {
                        elapsed_ms: cost.elapsed_ms,
                        limit_ms: config.max_latency_ms,
                    },
                };
            }
            if cost.token_cost > config.max_token_cost {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::TokenCapExceeded {
                        token_cost: cost.token_cost,
                        limit: config.max_token_cost,
                    },
                };
            }
            let usable_claims = grounding.usable_claims().len();
            if usable_claims == 0 {
                return RoutingDecision::Baseline {
                    reason: FallbackReason::NoUsableEvidence,
                };
            }
            RoutingDecision::Lineage {
                usable_claims,
                unresolved_demands: grounding.unresolved_demands,
                cost,
            }
        }
    }
}

#[cfg(test)]
#[path = "lineage_routing_tests.rs"]
mod tests;
