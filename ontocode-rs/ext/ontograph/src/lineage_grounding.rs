//! Source grounding for selected evidence spans (LR-4.0).
//!
//! Grounding answers a narrow question: does this selected span still resolve
//! to the claimed text in the active source generation, and was it retrieved
//! for the demand it is now cited under. Containment is provenance evidence
//! only. A contained span is never treated as proof that the source is
//! authoritative or logically sufficient for the demand, so containment and
//! demand support are reported as separate facts.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::lineage_records::LineageLedger;
use crate::lineage_selection::SelectionReport;

/// One source file as it exists in the active generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceGenerationFile {
    pub(crate) path: String,
    /// Generation identity, such as a content hash, recorded at retrieval time.
    pub(crate) generation_id: String,
    pub(crate) lines: Vec<String>,
}

/// The active source generation that grounding resolves spans against.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SourceGeneration {
    pub(crate) files: BTreeMap<String, SourceGenerationFile>,
}

impl SourceGeneration {
    pub(crate) fn from_files(files: Vec<SourceGenerationFile>) -> Self {
        Self {
            files: files
                .into_iter()
                .map(|file| (file.path.clone(), file))
                .collect(),
        }
    }
}

/// A claim a selected passage is cited for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GroundingClaim {
    pub(crate) passage_id: String,
    /// Demand this claim is cited under.
    pub(crate) demand_id: String,
    /// Generation identity observed when the passage was retrieved.
    pub(crate) observed_generation_id: String,
    /// Text the claim asserts is present in the span.
    pub(crate) quoted_text: String,
}

/// Whether the quoted text is present in the resolved span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContainmentStatus {
    Contained,
    NotContained,
}

/// Whether the cited demand actually retrieved this passage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DemandSupport {
    /// The passage was retrieved for the cited demand.
    Retrieved,
    /// The passage exists in the ledger but under other demands only.
    WrongDemand,
}

/// One grounded claim, keeping containment and demand support distinct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GroundedClaim {
    pub(crate) passage_id: String,
    pub(crate) demand_id: String,
    pub(crate) source_span: String,
    pub(crate) containment: ContainmentStatus,
    pub(crate) demand_support: DemandSupport,
}

impl GroundedClaim {
    /// A claim is usable only when the text is present and the demand is the
    /// one that retrieved the passage.
    pub(crate) fn is_usable(&self) -> bool {
        matches!(self.containment, ContainmentStatus::Contained)
            && matches!(self.demand_support, DemandSupport::Retrieved)
    }
}

/// Grounding outcome for one selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GroundingReport {
    pub(crate) claims: Vec<GroundedClaim>,
    /// Demands the selection left uncovered, carried through unchanged.
    pub(crate) unresolved_demands: Vec<String>,
}

impl GroundingReport {
    pub(crate) fn usable_claims(&self) -> Vec<&GroundedClaim> {
        self.claims
            .iter()
            .filter(|claim| claim.is_usable())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn has_unresolved_lineage(&self) -> bool {
        !self.unresolved_demands.is_empty() || self.claims.iter().any(|claim| !claim.is_usable())
    }
}

/// Fail-closed rejection reasons for a grounding request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GroundingError {
    /// The claim names a passage that was never selected.
    UnselectedPassage(String),
    /// The claim names a demand absent from the ledger.
    UnknownDemand {
        passage_id: String,
        demand_id: String,
    },
    /// The claimed passage has no lineage record.
    MissingLineage(String),
    /// The source file is absent from the active generation.
    MissingSourceFile {
        passage_id: String,
        path: String,
    },
    /// The source changed since retrieval.
    StaleSource {
        passage_id: String,
        observed: String,
        active: String,
    },
    /// The recorded span does not fit the current file.
    SpanOutOfRange {
        passage_id: String,
        start_line: usize,
        end_line: usize,
        available_lines: usize,
    },
    /// The lineage span lacks the line range grounding requires.
    MalformedSpan {
        passage_id: String,
        reason: String,
    },
    EmptyQuotedText(String),
    /// The same passage and demand were claimed twice.
    DuplicateClaim {
        passage_id: String,
        demand_id: String,
    },
}

impl fmt::Display for GroundingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnselectedPassage(passage_id) => {
                write!(
                    f,
                    "claim cites passage `{passage_id}` that was not selected"
                )
            }
            Self::UnknownDemand {
                passage_id,
                demand_id,
            } => write!(
                f,
                "claim for passage `{passage_id}` cites unknown demand `{demand_id}`"
            ),
            Self::MissingLineage(passage_id) => {
                write!(f, "passage `{passage_id}` has no lineage record")
            }
            Self::MissingSourceFile { passage_id, path } => write!(
                f,
                "passage `{passage_id}` resolves to `{path}`, which is absent from the active generation"
            ),
            Self::StaleSource {
                passage_id,
                observed,
                active,
            } => write!(
                f,
                "passage `{passage_id}` was retrieved at generation `{observed}` but the active generation is `{active}`"
            ),
            Self::SpanOutOfRange {
                passage_id,
                start_line,
                end_line,
                available_lines,
            } => write!(
                f,
                "passage `{passage_id}` span {start_line}-{end_line} exceeds the {available_lines} available lines"
            ),
            Self::MalformedSpan { passage_id, reason } => {
                write!(f, "passage `{passage_id}` has a malformed span: {reason}")
            }
            Self::EmptyQuotedText(passage_id) => {
                write!(f, "claim for passage `{passage_id}` quotes no text")
            }
            Self::DuplicateClaim {
                passage_id,
                demand_id,
            } => write!(
                f,
                "passage `{passage_id}` is claimed twice under demand `{demand_id}`"
            ),
        }
    }
}

impl std::error::Error for GroundingError {}

/// Grounds claims against the active source generation.
///
/// Every rejection is an error rather than a silently dropped claim, so a
/// caller cannot mistake a missing claim for an unsupported one.
pub(crate) fn ground_claims(
    ledger: &LineageLedger,
    selection: &SelectionReport,
    generation: &SourceGeneration,
    claims: &[GroundingClaim],
) -> Result<GroundingReport, GroundingError> {
    let mut seen: BTreeMap<(String, String), ()> = BTreeMap::new();
    let mut grounded = Vec::new();

    for claim in claims {
        if claim.quoted_text.trim().is_empty() {
            return Err(GroundingError::EmptyQuotedText(claim.passage_id.clone()));
        }
        if seen
            .insert((claim.passage_id.clone(), claim.demand_id.clone()), ())
            .is_some()
        {
            return Err(GroundingError::DuplicateClaim {
                passage_id: claim.passage_id.clone(),
                demand_id: claim.demand_id.clone(),
            });
        }
        let Some(selected) = selection
            .selected
            .iter()
            .find(|entry| entry.passage_id == claim.passage_id)
        else {
            return Err(GroundingError::UnselectedPassage(claim.passage_id.clone()));
        };
        if !ledger
            .demands
            .iter()
            .any(|demand| demand.demand_id == claim.demand_id)
        {
            return Err(GroundingError::UnknownDemand {
                passage_id: claim.passage_id.clone(),
                demand_id: claim.demand_id.clone(),
            });
        }
        let Some(lineage) = ledger.lineage_for(&claim.passage_id) else {
            return Err(GroundingError::MissingLineage(claim.passage_id.clone()));
        };

        let (Some(start_line), Some(end_line)) = (
            lineage.source_location.start_line,
            lineage.source_location.end_line,
        ) else {
            return Err(GroundingError::MalformedSpan {
                passage_id: claim.passage_id.clone(),
                reason: "grounding requires both a start and an end line".to_string(),
            });
        };
        if start_line == 0 {
            return Err(GroundingError::MalformedSpan {
                passage_id: claim.passage_id.clone(),
                reason: "line numbers are 1-based".to_string(),
            });
        }

        let path = &lineage.source_location.path;
        let Some(file) = generation.files.get(path) else {
            return Err(GroundingError::MissingSourceFile {
                passage_id: claim.passage_id.clone(),
                path: path.clone(),
            });
        };
        if file.generation_id != claim.observed_generation_id {
            return Err(GroundingError::StaleSource {
                passage_id: claim.passage_id.clone(),
                observed: claim.observed_generation_id.clone(),
                active: file.generation_id.clone(),
            });
        }
        if end_line > file.lines.len() {
            return Err(GroundingError::SpanOutOfRange {
                passage_id: claim.passage_id.clone(),
                start_line,
                end_line,
                available_lines: file.lines.len(),
            });
        }

        let span_text = file.lines[start_line - 1..end_line].join("\n");
        let containment = if span_text.contains(claim.quoted_text.trim()) {
            ContainmentStatus::Contained
        } else {
            ContainmentStatus::NotContained
        };
        let demand_support = if lineage.demand_membership.contains(&claim.demand_id) {
            DemandSupport::Retrieved
        } else {
            DemandSupport::WrongDemand
        };

        grounded.push(GroundedClaim {
            passage_id: claim.passage_id.clone(),
            demand_id: claim.demand_id.clone(),
            source_span: selected.source_span.clone(),
            containment,
            demand_support,
        });
    }

    grounded.sort_by(|left, right| {
        left.passage_id
            .cmp(&right.passage_id)
            .then_with(|| left.demand_id.cmp(&right.demand_id))
    });

    Ok(GroundingReport {
        claims: grounded,
        unresolved_demands: selection.uncovered_demands.clone(),
    })
}

#[cfg(test)]
#[path = "lineage_grounding_tests.rs"]
mod tests;
