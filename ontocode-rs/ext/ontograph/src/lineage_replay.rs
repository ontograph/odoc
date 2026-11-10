//! Deterministic replay harness for demand-aware evidence lineage (LR-1.0).
//!
//! Measures a fixed question corpus under three retrieval variants so later
//! LineageRAG tasks can attribute gains to demand induction, provenance,
//! selection, or grounding. The harness is offline and never calls a provider
//! or mutates runtime state.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;

#[cfg(test)]
pub(crate) const ROLLOUT_CORPUS_JSON: &str = include_str!("fixtures/lineage_rollout/corpus.json");

/// Loads the checked-in rollout corpus without changing its stored shape.
#[cfg(test)]
pub(crate) fn load_rollout_corpus() -> Result<ReplayFixture, serde_json::Error> {
    serde_json::from_str(ROLLOUT_CORPUS_JSON)
}

/// Retrieval variant under measurement.
///
/// The three variants isolate one mechanism each, so a measured delta can be
/// attributed instead of inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplayVariant {
    /// Single-shot retrieval over the raw question, with no demand induction.
    Baseline,
    /// Demand induction only; candidates keep no provenance identity.
    DecompositionOnly,
    /// Demand induction plus per-passage provenance and demand membership.
    ProvenanceAware,
}

impl ReplayVariant {
    pub(crate) const ALL: [Self; 3] = [
        Self::Baseline,
        Self::DecompositionOnly,
        Self::ProvenanceAware,
    ];

    fn induces_demands(self) -> bool {
        match self {
            Self::Baseline => false,
            Self::DecompositionOnly | Self::ProvenanceAware => true,
        }
    }

    fn retains_provenance(self) -> bool {
        match self {
            Self::Baseline | Self::DecompositionOnly => false,
            Self::ProvenanceAware => true,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::DecompositionOnly => "decomposition_only",
            Self::ProvenanceAware => "provenance_aware",
        }
    }
}

/// One evidence demand induced from a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayDemand {
    pub(crate) demand_id: String,
    pub(crate) description: String,
    /// Passage ids that satisfy this demand in the fixture's ground truth.
    pub(crate) satisfying_passages: BTreeSet<String>,
}

/// One retrievable passage in the fixture corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayPassage {
    pub(crate) passage_id: String,
    pub(crate) path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    /// Lowercase terms used for deterministic lexical matching.
    pub(crate) terms: BTreeSet<String>,
    /// Approximate token cost of including this passage in context.
    pub(crate) token_cost: usize,
}

/// One replayable multi-hop question with its ground truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayCase {
    pub(crate) case_id: String,
    pub(crate) question_terms: Vec<String>,
    pub(crate) demands: Vec<ReplayDemand>,
    /// Passage ids a correct answer must cite.
    pub(crate) required_passages: BTreeSet<String>,
}

/// Fixed corpus plus the settings that make a run reproducible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayFixture {
    pub(crate) corpus_id: String,
    /// Recorded prompt identity; the harness does not call a model.
    pub(crate) prompt_id: String,
    /// Recorded deterministic settings, such as `temperature=0`.
    pub(crate) model_settings: String,
    pub(crate) passages: Vec<ReplayPassage>,
    pub(crate) cases: Vec<ReplayCase>,
    /// Maximum passages a variant may select per question.
    pub(crate) max_selected_passages: usize,
}

/// Settings whose values are part of a reproducible rollout run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ReplaySettings {
    pub(crate) max_selected_passages: usize,
    pub(crate) prompt_id: String,
    pub(crate) model_settings: String,
}

impl ReplayFixture {
    pub(crate) fn settings(&self) -> ReplaySettings {
        ReplaySettings {
            max_selected_passages: self.max_selected_passages,
            prompt_id: self.prompt_id.clone(),
            model_settings: self.model_settings.clone(),
        }
    }
}

/// SHA-256 identity over the source bytes, parsed fixture, and run settings.
pub(crate) fn corpus_digest(
    raw_corpus_bytes: &[u8],
    fixture: &ReplayFixture,
    settings: &ReplaySettings,
) -> Result<String, serde_json::Error> {
    let fixture_bytes = serde_json::to_vec(fixture)?;
    let settings_bytes = serde_json::to_vec(settings)?;
    let mut hasher = Sha256::new();
    hasher.update(b"ontograph-lineage-rollout-corpus-v1");
    for field in [
        raw_corpus_bytes,
        fixture_bytes.as_slice(),
        settings_bytes.as_slice(),
    ] {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Per-passage retrieval record carrying the lineage fields LR-2.0 preserves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplaySelection {
    pub(crate) passage_id: String,
    pub(crate) rank: usize,
    pub(crate) score: usize,
    /// Demand that first exposed this passage; `None` without provenance.
    pub(crate) demand_id: Option<String>,
    /// Source span, retained only when the variant preserves provenance.
    pub(crate) source_span: Option<String>,
}

/// Outcome for one question under one variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayCaseOutcome {
    pub(crate) case_id: String,
    pub(crate) selections: Vec<ReplaySelection>,
    pub(crate) uncovered_demands: Vec<String>,
    pub(crate) token_cost: usize,
    /// Deterministic unit-cost latency proxy: one unit per scored candidate.
    pub(crate) latency_units: usize,
    /// True when every required passage was selected.
    pub(crate) answered_correctly: bool,
}

/// Aggregate metrics for one variant across the corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayMetrics {
    pub(crate) variant: ReplayVariant,
    pub(crate) cases: usize,
    /// Cases whose required passages were all selected, in basis points.
    pub(crate) answer_quality_bp: usize,
    /// Demands with at least one selected satisfying passage, in basis points.
    pub(crate) demand_recall_bp: usize,
    /// Required passages selected, in basis points.
    pub(crate) passage_coverage_bp: usize,
    pub(crate) total_token_cost: usize,
    pub(crate) total_latency_units: usize,
    /// Cases that ended with at least one uncovered demand.
    pub(crate) failure_count: usize,
    /// Selections that carry a resolved source span.
    pub(crate) provenance_bearing_selections: usize,
}

/// Complete evidence artifact for one replay run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplayReport {
    pub(crate) corpus_id: String,
    pub(crate) prompt_id: String,
    pub(crate) model_settings: String,
    pub(crate) metrics: Vec<ReplayMetrics>,
    pub(crate) outcomes: BTreeMap<String, Vec<ReplayCaseOutcome>>,
    /// Identity of the exact corpus and settings these metrics came from.
    ///
    /// Recomputed from fixture content, so a published artifact that no longer
    /// matches the fixture is detectable rather than merely stale.
    pub(crate) fixture_digest: String,
    /// Whether a provider was actually called to produce these metrics.
    ///
    /// The harness is offline, so this is always `false` today. It is recorded
    /// rather than assumed, because an offline number must never be read as
    /// production answer quality.
    pub(crate) provider_backed: bool,
}

impl ReplayReport {
    pub(crate) fn metrics_for(&self, variant: ReplayVariant) -> Option<&ReplayMetrics> {
        self.metrics.iter().find(|entry| entry.variant == variant)
    }

    /// Names the mechanism that explains the measured gain over the baseline.
    ///
    /// Attribution reads recorded metrics only, so the report cannot claim a
    /// benefit the run did not measure.
    pub(crate) fn attribution(&self) -> ReplayAttribution {
        let (Some(baseline), Some(decomposition), Some(provenance)) = (
            self.metrics_for(ReplayVariant::Baseline),
            self.metrics_for(ReplayVariant::DecompositionOnly),
            self.metrics_for(ReplayVariant::ProvenanceAware),
        ) else {
            return ReplayAttribution::Inconclusive;
        };
        let induction_gain = decomposition
            .demand_recall_bp
            .saturating_sub(baseline.demand_recall_bp);
        let selection_gain = provenance
            .demand_recall_bp
            .saturating_sub(decomposition.demand_recall_bp);
        let grounding_gain = provenance
            .provenance_bearing_selections
            .saturating_sub(decomposition.provenance_bearing_selections);
        if induction_gain == 0 && selection_gain == 0 && grounding_gain == 0 {
            return ReplayAttribution::NoMeasuredGain;
        }
        if induction_gain > selection_gain {
            return ReplayAttribution::DemandInduction {
                gain_bp: induction_gain,
            };
        }
        if selection_gain > 0 {
            return ReplayAttribution::Selection {
                gain_bp: selection_gain,
            };
        }
        ReplayAttribution::Grounding {
            additional_grounded_selections: grounding_gain,
        }
    }
}

/// Rollout thresholds fixed before results are compared.
///
/// Declaring these up front is the point: a threshold chosen after seeing the
/// numbers proves nothing. `evaluate` reads recorded metrics only, so it cannot
/// credit a gain the run did not measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RolloutThresholds {
    /// Minimum demand recall the lineage path must reach, in basis points.
    pub(crate) min_demand_recall_bp: usize,
    /// Minimum passage coverage the lineage path must reach, in basis points.
    pub(crate) min_passage_coverage_bp: usize,
    /// Largest token cost multiple of baseline that still justifies the gain,
    /// expressed in basis points (`20_000` means at most twice the baseline).
    pub(crate) max_token_cost_ratio_bp: usize,
    /// Whether a provider-backed run is required before default enablement.
    pub(crate) require_provider_backed: bool,
}

/// Why the rollout gate did or did not pass.
///
/// Each rejection names the measurement that failed, so a blocked rollout is
/// actionable instead of a bare "no".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RolloutVerdict {
    /// Every threshold passed and the evidence is admissible for enablement.
    Ready,
    /// Metrics were missing, so no verdict is possible.
    Inconclusive,
    /// The lineage path did not reach the required recall.
    RecallBelowThreshold {
        measured_bp: usize,
        required_bp: usize,
    },
    /// The lineage path did not reach the required passage coverage.
    CoverageBelowThreshold {
        measured_bp: usize,
        required_bp: usize,
    },
    /// The recall gain cost more tokens than the threshold allows.
    TokenCostAboveThreshold {
        measured_bp: usize,
        allowed_bp: usize,
    },
    /// Thresholds passed, but the run never called a provider.
    ProviderEvidenceMissing,
}

impl ReplayReport {
    /// Applies pre-declared thresholds to the measured provenance-aware path.
    ///
    /// The comparison target is `ProvenanceAware`, because that is the variant
    /// production would enable. A missing variant is `Inconclusive` rather than
    /// a pass, so an incomplete run cannot authorize rollout.
    pub(crate) fn evaluate(&self, thresholds: RolloutThresholds) -> RolloutVerdict {
        let (Some(baseline), Some(lineage)) = (
            self.metrics_for(ReplayVariant::Baseline),
            self.metrics_for(ReplayVariant::ProvenanceAware),
        ) else {
            return RolloutVerdict::Inconclusive;
        };
        if lineage.demand_recall_bp < thresholds.min_demand_recall_bp {
            return RolloutVerdict::RecallBelowThreshold {
                measured_bp: lineage.demand_recall_bp,
                required_bp: thresholds.min_demand_recall_bp,
            };
        }
        if lineage.passage_coverage_bp < thresholds.min_passage_coverage_bp {
            return RolloutVerdict::CoverageBelowThreshold {
                measured_bp: lineage.passage_coverage_bp,
                required_bp: thresholds.min_passage_coverage_bp,
            };
        }
        let cost_ratio_bp = basis_points(lineage.total_token_cost, baseline.total_token_cost);
        if cost_ratio_bp > thresholds.max_token_cost_ratio_bp {
            return RolloutVerdict::TokenCostAboveThreshold {
                measured_bp: cost_ratio_bp,
                allowed_bp: thresholds.max_token_cost_ratio_bp,
            };
        }
        if thresholds.require_provider_backed && !self.provider_backed {
            return RolloutVerdict::ProviderEvidenceMissing;
        }
        RolloutVerdict::Ready
    }

    /// Fallback counts per variant, keyed by variant, for the published table.
    ///
    /// Uncovered demands are the offline stand-in for a production fallback:
    /// both mean the path could not carry evidence for a demand.
    pub(crate) fn fallback_counts(&self) -> BTreeMap<String, usize> {
        self.metrics
            .iter()
            .map(|metrics| (metrics.variant.key().to_string(), metrics.failure_count))
            .collect()
    }
}

/// Mechanism credited with the measured improvement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplayAttribution {
    DemandInduction {
        gain_bp: usize,
    },
    Selection {
        gain_bp: usize,
    },
    Grounding {
        additional_grounded_selections: usize,
    },
    NoMeasuredGain,
    Inconclusive,
}

/// Rejection reasons for a fixture that cannot produce a reproducible run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReplayFixtureError {
    EmptyCorpus,
    MissingModelSettings,
    DuplicatePassageId(String),
    UnknownPassageReference { case_id: String, passage_id: String },
    ZeroSelectionBudget,
}

impl fmt::Display for ReplayFixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCorpus => write!(f, "replay fixture requires at least one case and passage"),
            Self::MissingModelSettings => write!(
                f,
                "replay fixture requires recorded prompt and model settings"
            ),
            Self::DuplicatePassageId(id) => {
                write!(f, "replay fixture has duplicate passage id `{id}`")
            }
            Self::UnknownPassageReference {
                case_id,
                passage_id,
            } => write!(
                f,
                "case `{case_id}` references unknown passage `{passage_id}`"
            ),
            Self::ZeroSelectionBudget => {
                write!(f, "replay fixture requires a non-zero passage budget")
            }
        }
    }
}

impl std::error::Error for ReplayFixtureError {}

impl ReplayFixture {
    /// Fails closed when the corpus or settings cannot be fixed, which is the
    /// task's stop condition.
    pub(crate) fn validate(&self) -> Result<(), ReplayFixtureError> {
        if self.cases.is_empty() || self.passages.is_empty() {
            return Err(ReplayFixtureError::EmptyCorpus);
        }
        if self.prompt_id.trim().is_empty() || self.model_settings.trim().is_empty() {
            return Err(ReplayFixtureError::MissingModelSettings);
        }
        if self.max_selected_passages == 0 {
            return Err(ReplayFixtureError::ZeroSelectionBudget);
        }
        let mut seen = BTreeSet::new();
        for passage in &self.passages {
            if !seen.insert(passage.passage_id.clone()) {
                return Err(ReplayFixtureError::DuplicatePassageId(
                    passage.passage_id.clone(),
                ));
            }
        }
        for case in &self.cases {
            let referenced = case.required_passages.iter().chain(
                case.demands
                    .iter()
                    .flat_map(|demand| demand.satisfying_passages.iter()),
            );
            for passage_id in referenced {
                if !seen.contains(passage_id) {
                    return Err(ReplayFixtureError::UnknownPassageReference {
                        case_id: case.case_id.clone(),
                        passage_id: passage_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn passage(&self, passage_id: &str) -> Option<&ReplayPassage> {
        self.passages
            .iter()
            .find(|passage| passage.passage_id == passage_id)
    }

    /// Stable identity of this fixture's content and settings.
    ///
    /// A published artifact records this value, so a corpus edit that would
    /// silently invalidate the numbers changes the digest instead. This is an
    /// integrity check against drift, not a cryptographic commitment.
    pub(crate) fn digest(&self) -> String {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut absorb = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        absorb(self.corpus_id.as_bytes());
        absorb(self.prompt_id.as_bytes());
        absorb(self.model_settings.as_bytes());
        absorb(&self.max_selected_passages.to_le_bytes());
        for passage in &self.passages {
            absorb(passage.passage_id.as_bytes());
            absorb(passage.path.as_bytes());
            absorb(&passage.start_line.to_le_bytes());
            absorb(&passage.end_line.to_le_bytes());
            absorb(&passage.token_cost.to_le_bytes());
            for term in &passage.terms {
                absorb(term.as_bytes());
            }
        }
        for case in &self.cases {
            absorb(case.case_id.as_bytes());
            for term in &case.question_terms {
                absorb(term.as_bytes());
            }
            for required in &case.required_passages {
                absorb(required.as_bytes());
            }
            for demand in &case.demands {
                absorb(demand.demand_id.as_bytes());
                absorb(demand.description.as_bytes());
                for satisfying in &demand.satisfying_passages {
                    absorb(satisfying.as_bytes());
                }
            }
        }
        format!("{hash:016x}")
    }
}

/// Runs every variant over the fixture and returns one evidence artifact.
pub(crate) fn run_replay(fixture: &ReplayFixture) -> Result<ReplayReport, ReplayFixtureError> {
    fixture.validate()?;
    let mut metrics = Vec::new();
    let mut outcomes = BTreeMap::new();
    for variant in ReplayVariant::ALL {
        let case_outcomes = fixture
            .cases
            .iter()
            .map(|case| run_case(fixture, case, variant))
            .collect::<Vec<_>>();
        metrics.push(aggregate(variant, fixture, &case_outcomes));
        outcomes.insert(variant.key().to_string(), case_outcomes);
    }
    Ok(ReplayReport {
        corpus_id: fixture.corpus_id.clone(),
        prompt_id: fixture.prompt_id.clone(),
        model_settings: fixture.model_settings.clone(),
        metrics,
        outcomes,
        fixture_digest: fixture.digest(),
        // This harness never calls a provider; see the module header.
        provider_backed: false,
    })
}

/// Scores candidates for one question under one variant.
///
/// Scoring is lexical term overlap, so a replay is byte-identical across runs.
fn run_case(
    fixture: &ReplayFixture,
    case: &ReplayCase,
    variant: ReplayVariant,
) -> ReplayCaseOutcome {
    let mut scored: Vec<(usize, String, Option<String>)> = Vec::new();
    let mut latency_units = 0usize;

    if variant.induces_demands() {
        // Each demand contributes its own candidate exposure, so a passage that
        // only satisfies a later hop is still reachable.
        for demand in &case.demands {
            for passage in &fixture.passages {
                latency_units += 1;
                let score = demand_score(demand, passage);
                if score > 0 {
                    scored.push((
                        score,
                        passage.passage_id.clone(),
                        Some(demand.demand_id.clone()),
                    ));
                }
            }
        }
    } else {
        for passage in &fixture.passages {
            latency_units += 1;
            let score = question_score(&case.question_terms, passage);
            if score > 0 {
                scored.push((score, passage.passage_id.clone(), None));
            }
        }
    }

    // Deterministic order: score descending, then passage id, then demand id.
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });

    let mut selections = Vec::new();
    let mut selected_ids = BTreeSet::new();
    let mut token_cost = 0usize;
    for (score, passage_id, demand_id) in scored {
        if selections.len() >= fixture.max_selected_passages {
            break;
        }
        if !selected_ids.insert(passage_id.clone()) {
            continue;
        }
        let passage = fixture.passage(&passage_id);
        token_cost += passage.map_or(0, |passage| passage.token_cost);
        let source_span = if variant.retains_provenance() {
            passage.map(|passage| {
                let path = &passage.path;
                let start = passage.start_line;
                let end = passage.end_line;
                format!("{path}:{start}-{end}")
            })
        } else {
            None
        };
        selections.push(ReplaySelection {
            passage_id,
            rank: selections.len() + 1,
            score,
            demand_id: demand_id.filter(|_| variant.retains_provenance()),
            source_span,
        });
    }

    let uncovered_demands = case
        .demands
        .iter()
        .filter(|demand| demand.satisfying_passages.is_disjoint(&selected_ids))
        .map(|demand| demand.demand_id.clone())
        .collect::<Vec<_>>();
    let answered_correctly = case.required_passages.is_subset(&selected_ids);

    ReplayCaseOutcome {
        case_id: case.case_id.clone(),
        selections,
        uncovered_demands,
        token_cost,
        latency_units,
        answered_correctly,
    }
}

fn question_score(question_terms: &[String], passage: &ReplayPassage) -> usize {
    question_terms
        .iter()
        .filter(|term| passage.terms.contains(*term))
        .count()
}

fn demand_score(demand: &ReplayDemand, passage: &ReplayPassage) -> usize {
    demand
        .description
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|term| passage.terms.contains(term))
        .count()
}

fn aggregate(
    variant: ReplayVariant,
    fixture: &ReplayFixture,
    outcomes: &[ReplayCaseOutcome],
) -> ReplayMetrics {
    let cases = outcomes.len();
    let answered = outcomes
        .iter()
        .filter(|outcome| outcome.answered_correctly)
        .count();
    let total_demands: usize = fixture.cases.iter().map(|case| case.demands.len()).sum();
    let uncovered: usize = outcomes
        .iter()
        .map(|outcome| outcome.uncovered_demands.len())
        .sum();
    let covered_demands = total_demands.saturating_sub(uncovered);
    let total_required: usize = fixture
        .cases
        .iter()
        .map(|case| case.required_passages.len())
        .sum();
    let covered_required: usize = fixture
        .cases
        .iter()
        .zip(outcomes.iter())
        .map(|(case, outcome)| {
            let selected = outcome
                .selections
                .iter()
                .map(|selection| selection.passage_id.clone())
                .collect::<BTreeSet<_>>();
            case.required_passages.intersection(&selected).count()
        })
        .sum();

    ReplayMetrics {
        variant,
        cases,
        answer_quality_bp: basis_points(answered, cases),
        demand_recall_bp: basis_points(covered_demands, total_demands),
        passage_coverage_bp: basis_points(covered_required, total_required),
        total_token_cost: outcomes.iter().map(|outcome| outcome.token_cost).sum(),
        total_latency_units: outcomes.iter().map(|outcome| outcome.latency_units).sum(),
        failure_count: outcomes
            .iter()
            .filter(|outcome| !outcome.uncovered_demands.is_empty())
            .count(),
        provenance_bearing_selections: outcomes
            .iter()
            .flat_map(|outcome| outcome.selections.iter())
            .filter(|selection| selection.source_span.is_some())
            .count(),
    }
}

/// Integer basis points avoid float drift, keeping replays byte-identical.
fn basis_points(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    numerator * 10_000 / denominator
}

#[cfg(test)]
#[path = "lineage_replay_tests.rs"]
mod tests;
