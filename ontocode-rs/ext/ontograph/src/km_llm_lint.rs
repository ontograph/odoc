use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::km::KmFact;
use crate::km::KmValidationStatus;
use crate::km::load_facts;
use crate::km::validate_km;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::run_model_text;

const LINT_REPORT: &str = "okf/draft_lint.md";
const MAX_MODEL_NOTE_CHARS: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLlmLintRequest {
    pub package_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLlmLintResult {
    pub report_path: String,
    pub findings: usize,
    pub health_checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmLintError {
    MissingPackage,
    HealthFailed(Vec<String>),
    ReadFacts,
    WriteReport,
}

pub fn run_km_llm_lint(
    request: &KmLlmLintRequest,
    model_ctx: Option<KmLlmModelContext<'_>>,
) -> Result<KmLlmLintResult, KmLlmLintError> {
    if !request.package_dir.is_dir() {
        return Err(KmLlmLintError::MissingPackage);
    }
    let health = validate_km(&request.package_dir);
    if health.status != KmValidationStatus::Valid {
        return Err(KmLlmLintError::HealthFailed(health.errors));
    }
    let facts = load_facts(&request.package_dir).map_err(|_| KmLlmLintError::ReadFacts)?;
    let findings = lint_facts(&facts);
    // Only consult the model when there is something to advise on.
    let model_notes = if findings.is_empty() {
        None
    } else {
        model_ctx
            .and_then(|ctx| run_model_text(ctx.config, &assemble_lint_prompt(&findings), ctx.model))
            .and_then(|notes| normalize_notes(&notes))
    };
    let report_path = request.package_dir.join(LINT_REPORT);
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(|_| KmLlmLintError::WriteReport)?;
    }
    fs::write(
        &report_path,
        render_report(&findings, model_notes.as_deref()),
    )
    .map_err(|_| KmLlmLintError::WriteReport)?;
    Ok(KmLlmLintResult {
        report_path: report_path.display().to_string(),
        findings: findings.len(),
        health_checked: true,
    })
}

impl std::fmt::Display for KmLlmLintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPackage => write!(f, "KM package directory is missing"),
            Self::HealthFailed(errors) => {
                write!(
                    f,
                    "KM deterministic health check failed: {}",
                    errors.join("; ")
                )
            }
            Self::ReadFacts => write!(f, "failed to read KM facts"),
            Self::WriteReport => write!(f, "failed to write KM lint report"),
        }
    }
}

impl std::error::Error for KmLlmLintError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LintFinding {
    code: &'static str,
    severity: &'static str,
    message: String,
}

fn lint_facts(facts: &[KmFact]) -> Vec<LintFinding> {
    let mut findings = Vec::new();
    for fact in facts {
        if fact.confidence < 0.7 {
            findings.push(LintFinding {
                code: "weak-confidence",
                severity: "advisory",
                message: format!(
                    "`{}` confidence {:.2} is below the advisory review threshold.",
                    fact.id, fact.confidence
                ),
            });
        }
        if !fact.source.starts_with("bronze:") && !fact.source.starts_with("silver:") {
            findings.push(LintFinding {
                code: "weak-citation-shape",
                severity: "advisory",
                message: format!(
                    "`{}` source `{}` is not a bronze:/silver: citation.",
                    fact.id, fact.source
                ),
            });
        }
    }
    findings.extend(duplicate_claim_findings(facts));
    findings
}

fn duplicate_claim_findings(facts: &[KmFact]) -> Vec<LintFinding> {
    let mut claims = BTreeMap::<String, Vec<&str>>::new();
    for fact in facts {
        claims
            .entry(normalize_claim(&fact.claim))
            .or_default()
            .push(&fact.id);
    }
    claims
        .into_iter()
        .filter_map(|(claim, ids)| {
            if claim.is_empty() || ids.len() < 2 {
                return None;
            }
            Some(LintFinding {
                code: "duplicate-claim",
                severity: "advisory",
                message: format!(
                    "Facts {} share the same normalized claim `{claim}`.",
                    ids.join(", ")
                ),
            })
        })
        .collect()
}

fn normalize_claim(claim: &str) -> String {
    claim
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn assemble_lint_prompt(findings: &[LintFinding]) -> String {
    let findings = findings
        .iter()
        .map(|finding| {
            format!(
                "- {} ({}) {}",
                finding.code, finding.severity, finding.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Write short advisory-only semantic lint notes for a KM package. Keep it under {MAX_MODEL_NOTE_CHARS} characters. These notes are advisory and never gate acceptance. Do not mention providers, models, secrets, endpoints, or credentials.\n\nDeterministic findings:\n{findings}\n"
    )
}

fn normalize_notes(notes: &str) -> Option<String> {
    let notes = notes.split_whitespace().collect::<Vec<_>>().join(" ");
    if notes.is_empty() || notes.chars().count() > MAX_MODEL_NOTE_CHARS {
        return None;
    }
    Some(notes)
}

fn render_report(findings: &[LintFinding], model_notes: Option<&str>) -> String {
    let mut report = String::from(
        "# Draft Semantic Lint\n\nDeterministic KM health check passed before advisory linting.\n\n",
    );
    if findings.is_empty() {
        report.push_str("No advisory findings.\n");
    } else {
        report.push_str("## Findings\n\n");
        for finding in findings {
            report.push_str(&format!(
                "- `{}` ({}) {}\n",
                finding.code, finding.severity, finding.message
            ));
        }
    }
    if let Some(notes) = model_notes {
        report.push_str("\n## Model Advisory Notes\n\n");
        report.push_str(notes);
        report.push('\n');
    }
    report
}

#[cfg(test)]
#[path = "km_llm_lint_tests.rs"]
mod tests;
