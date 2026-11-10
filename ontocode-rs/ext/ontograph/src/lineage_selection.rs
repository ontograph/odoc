//! Bounded complementary evidence selection over a lineage ledger (LR-3.0).
//!
//! Ranking by score alone fills the context with passages that answer the same
//! demand, so a later hop stays uncovered while the budget is already spent.
//! Selection here is greedy on marginal demand coverage under explicit passage
//! and context limits, and every demand left uncovered is reported rather than
//! implied by absence.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::lineage_records::LineageLedger;
use crate::lineage_records::PassageLineage;

/// Caller-supplied bounds for one selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectionBudget {
    pub(crate) max_passages: usize,
    pub(crate) max_context_tokens: usize,
    /// Score penalty applied per demand already covered by earlier selections.
    pub(crate) redundancy_penalty: usize,
}

impl Default for SelectionBudget {
    fn default() -> Self {
        Self {
            max_passages: 8,
            max_context_tokens: 4_096,
            redundancy_penalty: 1,
        }
    }
}

/// Why a passage was admitted, retained for the coverage report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectedPassage {
    pub(crate) passage_id: String,
    pub(crate) rank: usize,
    /// Demands this passage newly covered when it was admitted.
    pub(crate) newly_covered_demands: BTreeSet<String>,
    pub(crate) marginal_gain: usize,
    pub(crate) token_cost: usize,
    pub(crate) source_span: String,
}

/// Why selection stopped, so a caller can distinguish exhaustion from limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SelectionStop {
    /// No remaining passage covered an uncovered demand.
    CoverageExhausted,
    PassageLimitReached,
    ContextLimitReached,
}

/// Deterministic selection outcome plus its coverage accounting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SelectionReport {
    pub(crate) selected: Vec<SelectedPassage>,
    pub(crate) covered_demands: BTreeSet<String>,
    /// Demands with no selected passage, including ledger-unresolved demands.
    pub(crate) uncovered_demands: Vec<String>,
    pub(crate) total_token_cost: usize,
    pub(crate) stop_reason: SelectionStop,
}

impl SelectionReport {
    #[cfg(test)]
    pub(crate) fn is_complete(&self) -> bool {
        self.uncovered_demands.is_empty()
    }
}

/// Rejection reasons for a selection request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectionError {
    ZeroPassageBudget,
    ZeroContextBudget,
    MissingTokenCost(String),
    /// One passage alone exceeds the entire context budget.
    PassageExceedsContextBudget {
        passage_id: String,
        token_cost: usize,
        limit: usize,
    },
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroPassageBudget => {
                write!(f, "selection requires a non-zero max_passages budget")
            }
            Self::ZeroContextBudget => {
                write!(f, "selection requires a non-zero max_context_tokens budget")
            }
            Self::MissingTokenCost(passage_id) => {
                write!(f, "passage `{passage_id}` has no recorded token cost")
            }
            Self::PassageExceedsContextBudget {
                passage_id,
                token_cost,
                limit,
            } => write!(
                f,
                "passage `{passage_id}` costs {token_cost} tokens, exceeding the context limit {limit}"
            ),
        }
    }
}

impl std::error::Error for SelectionError {}

/// Selects a bounded complementary evidence set.
///
/// Candidates are scored by marginal demand coverage, then by best lineage
/// score reduced by `redundancy_penalty` for each already-covered demand, with
/// passage id as the final tiebreak. Ties therefore never depend on ledger
/// iteration order, so the same ledger and budget always select the same ids.
pub(crate) fn select_evidence(
    ledger: &LineageLedger,
    token_costs: &BTreeMap<String, usize>,
    budget: SelectionBudget,
) -> Result<SelectionReport, SelectionError> {
    if budget.max_passages == 0 {
        return Err(SelectionError::ZeroPassageBudget);
    }
    if budget.max_context_tokens == 0 {
        return Err(SelectionError::ZeroContextBudget);
    }
    for lineage in &ledger.passages {
        let Some(&token_cost) = token_costs.get(&lineage.passage_id) else {
            return Err(SelectionError::MissingTokenCost(lineage.passage_id.clone()));
        };
        if token_cost > budget.max_context_tokens {
            return Err(SelectionError::PassageExceedsContextBudget {
                passage_id: lineage.passage_id.clone(),
                token_cost,
                limit: budget.max_context_tokens,
            });
        }
    }

    let all_demands = ledger
        .demands
        .iter()
        .map(|demand| demand.demand_id.clone())
        .collect::<BTreeSet<_>>();

    let mut remaining = ledger.passages.iter().collect::<Vec<_>>();
    let mut covered: BTreeSet<String> = BTreeSet::new();
    let mut selected: Vec<SelectedPassage> = Vec::new();
    let mut total_token_cost = 0usize;
    let mut stop_reason = SelectionStop::CoverageExhausted;

    while !remaining.is_empty() {
        if selected.len() >= budget.max_passages {
            stop_reason = SelectionStop::PassageLimitReached;
            break;
        }

        let Some(position) = best_candidate(&remaining, &covered, budget.redundancy_penalty) else {
            stop_reason = SelectionStop::CoverageExhausted;
            break;
        };
        let lineage = remaining[position];
        let token_cost = token_costs
            .get(&lineage.passage_id)
            .copied()
            .unwrap_or_default();
        if total_token_cost + token_cost > budget.max_context_tokens {
            // Admitting this passage would exceed the context budget; stop
            // rather than silently skipping to a cheaper, less useful passage.
            stop_reason = SelectionStop::ContextLimitReached;
            break;
        }

        let newly_covered = lineage
            .demand_membership
            .difference(&covered)
            .cloned()
            .collect::<BTreeSet<_>>();
        covered.extend(newly_covered.iter().cloned());
        total_token_cost += token_cost;
        selected.push(SelectedPassage {
            passage_id: lineage.passage_id.clone(),
            rank: selected.len() + 1,
            marginal_gain: newly_covered.len(),
            newly_covered_demands: newly_covered,
            token_cost,
            source_span: span_of(lineage),
        });
        remaining.remove(position);
    }

    let uncovered_demands = all_demands
        .difference(&covered)
        .cloned()
        .collect::<Vec<_>>();

    Ok(SelectionReport {
        selected,
        covered_demands: covered,
        uncovered_demands,
        total_token_cost,
        stop_reason,
    })
}

/// Picks the passage with the highest marginal coverage, then penalized score.
///
/// Returns `None` once no remaining passage covers an uncovered demand, which
/// keeps selection from spending budget on redundant evidence.
fn best_candidate(
    remaining: &[&PassageLineage],
    covered: &BTreeSet<String>,
    redundancy_penalty: usize,
) -> Option<usize> {
    let mut best: Option<(usize, usize, usize, &str)> = None;
    for (position, lineage) in remaining.iter().enumerate() {
        let marginal_gain = lineage.demand_membership.difference(covered).count();
        if marginal_gain == 0 {
            continue;
        }
        let redundant = lineage.demand_membership.intersection(covered).count();
        let penalized_score = lineage
            .best_score()
            .saturating_sub(redundant * redundancy_penalty);
        let candidate = (
            marginal_gain,
            penalized_score,
            position,
            lineage.passage_id.as_str(),
        );
        let replace = match best {
            None => true,
            Some((best_gain, best_score, _, best_id)) => {
                (marginal_gain, penalized_score) > (best_gain, best_score)
                    || ((marginal_gain, penalized_score) == (best_gain, best_score)
                        && lineage.passage_id.as_str() < best_id)
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, position, _)| position)
}

fn span_of(lineage: &PassageLineage) -> String {
    let path = &lineage.source_location.path;
    match (
        lineage.source_location.start_line,
        lineage.source_location.end_line,
    ) {
        (Some(start), Some(end)) => format!("{path}:{start}-{end}"),
        (Some(start), None) => format!("{path}:{start}"),
        _ => path.clone(),
    }
}

#[cfg(test)]
#[path = "lineage_selection_tests.rs"]
mod tests;
