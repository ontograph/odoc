use ontocode_extension_api::ContextualUserFragment;

use crate::km::MAX_RENDERED_CONTEXT_BYTES;
use crate::km::MAX_RENDERED_EXAMPLES;
use crate::km::MAX_RENDERED_FACTS;
use crate::km::MAX_RENDERED_TRANSITIONS;
use crate::km_resolve::ResolvedKm;
use crate::km_store::KmIndexOrigin;
use crate::procedural_graph::ProceduralNeighborhood;

pub(crate) const KM_CONTEXT_START_MARKER: &str = "<ontograph_km_context>";
pub(crate) const KM_CONTEXT_END_MARKER: &str = "</ontograph_km_context>";
/// Inert stand-ins for the fragment markers. Both are the same byte length as
/// the markers they replace, so neutralizing cannot push a body past a cap that
/// was already measured.
pub(crate) const NEUTRALIZED_START_MARKER: &str = "[ontograph_km_context]";
pub(crate) const NEUTRALIZED_END_MARKER: &str = "[/ontograph_km_context]";
const MAX_REASON_CHARS: usize = 240;
const MAX_SUMMARY_CHARS: usize = 240;
const MAX_EXAMPLE_EXCERPT_CHARS: usize = 160;
const FRAGMENT_RENDER_OVERHEAD_BYTES: usize =
    KM_CONTEXT_START_MARKER.len() + KM_CONTEXT_END_MARKER.len() + 2;
/// Hard per-fragment token budget enforced with a conservative byte upper bound.
/// One token per byte is a strict worst-case estimate regardless of model or
/// input script, so capping the body at this many bytes proves the fragment
/// cannot exceed the budget.
pub(crate) const MAX_FRAGMENT_TOKENS: usize = 10_000;
/// Fragments crossing this estimated token size are flagged for review.
pub(crate) const LARGE_FRAGMENT_REVIEW_TOKENS: usize = 1_000;
const MAX_FRAGMENT_BODY_BYTES: usize = MAX_FRAGMENT_TOKENS - FRAGMENT_RENDER_OVERHEAD_BYTES;

/// Byte cap actually enforced on a rendered fragment body.
pub(crate) fn max_fragment_body_bytes() -> usize {
    MAX_RENDERED_CONTEXT_BYTES.min(MAX_FRAGMENT_BODY_BYTES)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KmContextFragment {
    body: String,
    #[cfg(test)]
    estimated_tokens: usize,
    exceeds_review_threshold: bool,
}

impl KmContextFragment {
    /// Conservative worst-case token estimate for the fully rendered fragment.
    #[cfg(test)]
    pub(crate) fn estimated_tokens(&self) -> usize {
        self.estimated_tokens
    }

    /// True when the fragment crosses the large-fragment review threshold.
    pub(crate) fn exceeds_review_threshold(&self) -> bool {
        self.exceeds_review_threshold
    }
}

/// Strict upper bound on model tokens for `text`: one token per byte.
pub(crate) fn estimate_tokens_upper_bound(text: &str) -> usize {
    text.len()
}

impl ContextualUserFragment for KmContextFragment {
    fn role(&self) -> &'static str {
        "user"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (KM_CONTEXT_START_MARKER, KM_CONTEXT_END_MARKER)
    }

    fn body(&self) -> String {
        format!("\n{}\n", self.body)
    }
}

pub(crate) fn render_km_context(resolved: &ResolvedKm, reason: &str) -> KmContextFragment {
    render_km_context_internal(resolved, reason, None)
}

pub(crate) fn render_km_context_with_procedural_guidance(
    resolved: &ResolvedKm,
    reason: &str,
    neighborhood: &ProceduralNeighborhood,
) -> KmContextFragment {
    render_km_context_internal(resolved, reason, Some(neighborhood))
}

pub(crate) fn render_km_summary(resolved: &ResolvedKm) -> KmContextFragment {
    fragment_from_body(format!(
        "Knowledge Module: {}@{}\nSummary: {}",
        resolved.id,
        resolved.version,
        single_line_cap(&resolved.summary, MAX_SUMMARY_CHARS)
    ))
}

fn render_km_context_internal(
    resolved: &ResolvedKm,
    reason: &str,
    procedural_neighborhood: Option<&ProceduralNeighborhood>,
) -> KmContextFragment {
    let reason = single_line_cap(reason, MAX_REASON_CHARS);
    let summary = single_line_cap(&resolved.summary, MAX_SUMMARY_CHARS);
    let mut fact_limit = resolved.facts.len().min(MAX_RENDERED_FACTS);
    let mut example_limit = resolved.examples.len().min(MAX_RENDERED_EXAMPLES);
    let mut body_capped = false;
    // Reserve marker and newline overhead so the fully rendered fragment stays
    // within the 10,000-token worst-case bound.
    let byte_cap = max_fragment_body_bytes();

    loop {
        let rendered = build_context_body(
            resolved,
            &reason,
            &summary,
            fact_limit,
            example_limit,
            procedural_neighborhood,
            &truncation_notes(resolved, body_capped),
        );
        if rendered.len() <= byte_cap {
            return fragment_from_body(rendered);
        }
        body_capped = true;
        if example_limit > 0 {
            example_limit -= 1;
            continue;
        }
        if fact_limit > 1 {
            fact_limit -= 1;
            continue;
        }
        let rendered = build_context_body(
            resolved,
            &reason,
            &summary,
            fact_limit,
            example_limit,
            procedural_neighborhood,
            &truncation_notes(resolved, body_capped),
        );
        let end = (0..=byte_cap)
            .rev()
            .find(|index| rendered.is_char_boundary(*index))
            .unwrap_or(0);
        return fragment_from_body(rendered[..end].to_string());
    }
}

fn fragment_from_body(body: String) -> KmContextFragment {
    let body = neutralize_markers(body);
    let estimated_tokens = estimate_tokens_upper_bound(&body) + FRAGMENT_RENDER_OVERHEAD_BYTES;
    KmContextFragment {
        exceeds_review_threshold: estimated_tokens > LARGE_FRAGMENT_REVIEW_TOKENS,
        #[cfg(test)]
        estimated_tokens,
        body,
    }
}

/// Rewrites fragment markers that appear inside package-authored text.
///
/// Package summaries, fact claims, and example excerpts are quoted user data.
/// Without this, stored text containing an end marker closes the quoted region
/// early and the remainder is presented outside it. Applying the rewrite here,
/// at the single point where every body is built, covers all package-authored
/// fields at once.
fn neutralize_markers(body: String) -> String {
    if !body.contains(KM_CONTEXT_START_MARKER) && !body.contains(KM_CONTEXT_END_MARKER) {
        return body;
    }
    body.replace(KM_CONTEXT_END_MARKER, NEUTRALIZED_END_MARKER)
        .replace(KM_CONTEXT_START_MARKER, NEUTRALIZED_START_MARKER)
}

fn build_context_body(
    resolved: &ResolvedKm,
    reason: &str,
    summary: &str,
    fact_limit: usize,
    example_limit: usize,
    procedural_neighborhood: Option<&ProceduralNeighborhood>,
    truncation_notes: &[String],
) -> String {
    let mut lines = vec![
        format!("Knowledge Module: {}@{}", resolved.id, resolved.version),
        format!("Reason: {reason}"),
        format!("Summary: {summary}"),
        format!(
            "Provenance: created_by={} generated_at={} source_commit={}",
            resolved.provenance.created_by,
            resolved.provenance.generated_at,
            resolved.provenance.source_commit
        ),
        format!("Sources: {}", resolved.sources.join(", ")),
        format!("Licenses: {}", resolved.licenses.join(", ")),
        "Facts:".to_string(),
    ];
    lines.extend(resolved.facts.iter().take(fact_limit).map(|fact| {
        format!(
            "- {} (source: {}; confidence: {:.2})",
            single_line_cap(&fact.claim, MAX_SUMMARY_CHARS),
            fact.source,
            fact.confidence
        )
    }));
    if let Some(agent_origin) = agent_origin_line(resolved.origin.as_ref()) {
        lines.push(agent_origin);
    }

    if let Some(neighborhood) = procedural_neighborhood {
        lines.extend(procedural_guidance_lines(neighborhood));
    }

    lines.push("Examples:".to_string());
    lines.extend(resolved.examples.iter().take(example_limit).map(|example| {
        format!(
            "- {}: {}",
            example.path,
            single_line_cap(&example.excerpt, MAX_EXAMPLE_EXCERPT_CHARS)
        )
    }));
    if !truncation_notes.is_empty() {
        lines.push("Truncation:".to_string());
        lines.extend(truncation_notes.iter().map(|note| format!("- {note}")));
    }
    lines.push("Limits:".to_string());
    lines.push(format!(
        "- rendered fragment capped at {} bytes",
        max_fragment_body_bytes()
    ));
    lines.push(format!(
        "- estimated tokens capped at {MAX_FRAGMENT_TOKENS} (conservative one token per byte)"
    ));
    lines.push(format!("- facts capped at {MAX_RENDERED_FACTS}"));
    lines.push(format!("- examples capped at {MAX_RENDERED_EXAMPLES}"));
    lines.join("\n")
}

fn procedural_guidance_lines(neighborhood: &ProceduralNeighborhood) -> Vec<String> {
    let mut lines = vec![
        "Procedural guidance (advisory only):".to_string(),
        format!(
            "- Active procedure: {}",
            single_line_cap(&neighborhood.active_node, MAX_SUMMARY_CHARS)
        ),
    ];
    if neighborhood.edges.is_empty() {
        lines.push(
            "- No outgoing transitions were found in the localized neighborhood.".to_string(),
        );
        return lines;
    }

    lines.push("Transitions:".to_string());
    lines.extend(
        neighborhood
            .edges
            .iter()
            .take(MAX_RENDERED_TRANSITIONS)
            .map(|edge| {
                let transition = &edge.transition;
                let mut details = Vec::new();
                if !transition.guidance.is_empty() {
                    details.push(single_line_cap(&transition.guidance, MAX_SUMMARY_CHARS));
                }
                if !transition.pitfalls.is_empty() {
                    let pitfalls = transition
                        .pitfalls
                        .iter()
                        .map(|pitfall| single_line_cap(pitfall, MAX_SUMMARY_CHARS))
                        .collect::<Vec<_>>()
                        .join("; ");
                    details.push(format!("pitfalls: {pitfalls}"));
                }
                let detail_suffix = if details.is_empty() {
                    String::new()
                } else {
                    format!(": {}", details.join(" "))
                };
                format!(
                    "- {} -> {}{detail_suffix}",
                    single_line_cap(&transition.source, MAX_SUMMARY_CHARS),
                    single_line_cap(&transition.target, MAX_SUMMARY_CHARS),
                )
            }),
    );
    if neighborhood.edges.len() > MAX_RENDERED_TRANSITIONS {
        let total = neighborhood.edges.len();
        lines.push(format!(
            "- transitions capped from {total} to {MAX_RENDERED_TRANSITIONS}"
        ));
    }
    lines
}

fn agent_origin_line(origin: Option<&KmIndexOrigin>) -> Option<String> {
    match origin {
        Some(KmIndexOrigin::AgentThread { thread_id }) => {
            Some(format!("Origin: agent-thread ({thread_id})"))
        }
        Some(KmIndexOrigin::Human) => None,
        None => None,
    }
}

fn truncation_notes(resolved: &ResolvedKm, body_capped: bool) -> Vec<String> {
    let mut notes = Vec::new();
    if resolved.facts.len() > MAX_RENDERED_FACTS {
        notes.push(format!(
            "facts capped from {} to {}",
            resolved.facts.len(),
            MAX_RENDERED_FACTS
        ));
    }
    if resolved.examples.len() > MAX_RENDERED_EXAMPLES {
        notes.push(format!(
            "examples capped from {} to {}",
            resolved.examples.len(),
            MAX_RENDERED_EXAMPLES
        ));
    }
    if body_capped {
        notes.push(format!(
            "rendered body capped at {MAX_FRAGMENT_BODY_BYTES} bytes"
        ));
    }
    notes
}

fn single_line_cap(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = normalized.chars().count();
    if count <= max_chars {
        return normalized;
    }
    normalized.chars().take(max_chars).collect()
}

#[cfg(test)]
#[path = "km_context_tests.rs"]
mod tests;
