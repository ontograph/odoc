//! Authoring-side model-call boundary for the KM LLM authoring stages.
//!
//! This module intentionally does not construct a provider client itself. The
//! live implementation of [`KmLlmTextModel`] wraps the existing
//! `ontocode_core::client::ModelClient` in the CLI, where provider, auth, and
//! runtime plumbing already live. The extension keeps only the deterministic
//! guard logic (enable gate, offline-only refusal, prompt caps, source
//! validation, metadata-only logging) so it can be unit tested without a
//! network or a second client stack.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

const MAX_MODEL_PROMPT_BYTES: usize = 10_000;
const MAX_MODEL_OUTPUT_BYTES: usize = 10_000;
pub(crate) const MODEL_EXTRACT_INSTRUCTION: &str =
    "Return only JSON with top-level arrays `facts` and `mappings`. ";
pub(crate) const MAX_MODEL_DRAFT_ROWS: usize = 200;
const MAX_MODEL_FIELD_CHARS: usize = 500;
const MAX_MODEL_SOURCE_CHARS: usize = 300;
const AUTHORING_CALLS_LOG: &str = ".ontocode/ontograph/km/_ops/authoring/calls.jsonl";
const MAX_ACCOUNTING_LABEL_CHARS: usize = 64;

use serde::Deserialize;
use serde::Serialize;

use crate::km::canonical_source_path;

/// A single draft fact row emitted by the extractor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftFact {
    pub id: String,
    pub kind: String,
    pub claim: String,
    pub source: String,
    pub confidence: f64,
}

/// A single draft concept-mapping row emitted by the extractor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftMapping {
    pub id: String,
    pub kind: String,
    pub source_concept: String,
    pub target_concept: String,
    pub relation: String,
    pub source: String,
    pub confidence: f64,
}

/// Structured draft returned by a model-backed extraction call.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KmLlmExtractDraft {
    #[serde(default)]
    pub facts: Vec<DraftFact>,
    #[serde(default)]
    pub mappings: Vec<DraftMapping>,
}

/// Errors surfaced by a model-backed extraction call. Any error triggers the
/// deterministic fallback; it never aborts the extractor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmClientError {
    ModelCall(String),
    Parse(String),
}

impl std::fmt::Display for KmLlmClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelCall(message) => write!(f, "model call failed: {message}"),
            Self::Parse(message) => write!(f, "failed to parse model output: {message}"),
        }
    }
}

impl KmLlmClientError {
    /// Stable machine-readable code for this failure.
    ///
    /// Diagnostics need a value that can be counted and compared across runs.
    /// The `Display` text embeds a provider- or parser-supplied message, which
    /// varies by endpoint and is unsafe to treat as an identifier.
    pub fn reason_code(&self) -> &'static str {
        match self {
            Self::ModelCall(_) => "km_llm.model_call_failed",
            Self::Parse(_) => "km_llm.model_output_unparseable",
        }
    }

    /// Reason code plus a redacted detail, safe to log.
    ///
    /// The underlying message can contain echoed prompt text, model output, or
    /// credentials embedded in a provider URL, so it is never emitted verbatim.
    /// Only its length is reported, which is enough to distinguish an empty
    /// failure from a truncated one without disclosing content.
    pub fn redacted_diagnostic(&self) -> String {
        let detail = match self {
            Self::ModelCall(message) | Self::Parse(message) => message,
        };
        format!(
            "{} (detail redacted, {} bytes)",
            self.reason_code(),
            detail.len()
        )
    }
}

impl std::error::Error for KmLlmClientError {}

/// Authoring-time model-call boundary. The live implementation wraps the
/// existing `ModelClient`; typed parsing stays in the stage owner.
pub trait KmLlmTextModel {
    /// Perform one bounded authoring call for the assembled prompt text.
    fn complete(&self, prompt_text: &str) -> Result<String, KmLlmClientError>;
}

/// Caller-resolved configuration for one extraction stage.
///
/// The caller (CLI) resolves provider locality and the effective model/provider
/// identifiers so the extension does not need provider-info dependencies.
#[derive(Debug, Clone)]
pub struct KmLlmClientConfig {
    pub enabled: bool,
    pub offline_only: bool,
    pub provider_is_local: bool,
    pub max_input_tokens: Option<u32>,
    pub model_slug: String,
    pub provider_id: String,
    /// Optional project root used only for best-effort authoring call accounting.
    pub project_root: Option<PathBuf>,
}

/// Bundles the resolved config with the live model boundary for one extractor
/// invocation. Absent means the deterministic scaffold runs unchanged.
pub struct KmLlmModelContext<'a> {
    pub config: &'a KmLlmClientConfig,
    pub model: &'a dyn KmLlmTextModel,
}

/// Rough token estimate used only for pre-call cap enforcement.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() / 4) as u32
}

/// Validate that a model-produced `source` still points at a real, prefixed
/// project source before it is written to a draft package.
pub fn validate_row_source(source: &str, project_root: &Path) -> bool {
    canonical_row_source(source, project_root).is_some()
}

/// Resolve a model-produced source to its canonical project-relative form.
pub(crate) fn canonical_row_source(source: &str, project_root: &Path) -> Option<String> {
    canonical_source_path(project_root, source)
}

/// Drops fact rows that fail field validation. Returns `None` when either row
/// list exceeds `MAX_MODEL_DRAFT_ROWS`, when any mapping row is invalid (a
/// partial mapping set would replace the deterministic mappings), or when no
/// valid row remains, so the caller falls back to the deterministic path.
pub(crate) fn validate_draft(mut draft: KmLlmExtractDraft) -> Option<KmLlmExtractDraft> {
    if draft.facts.len() > MAX_MODEL_DRAFT_ROWS || draft.mappings.len() > MAX_MODEL_DRAFT_ROWS {
        return None;
    }
    if !draft.mappings.iter().all(validate_draft_mapping) {
        return None;
    }
    draft.facts.retain(validate_draft_fact);
    (!draft.facts.is_empty() || !draft.mappings.is_empty()).then_some(draft)
}

pub(crate) fn validate_draft_fact(row: &DraftFact) -> bool {
    valid_field(&row.id)
        && valid_field(&row.kind)
        && valid_field(&row.claim)
        && row.claim.chars().count() <= MAX_MODEL_FIELD_CHARS
        && valid_source_field(&row.source)
        && (0.0..=1.0).contains(&row.confidence)
}

pub(crate) fn validate_draft_mapping(row: &DraftMapping) -> bool {
    valid_field(&row.id)
        && valid_field(&row.kind)
        && valid_field(&row.source_concept)
        && valid_field(&row.target_concept)
        && valid_field(&row.relation)
        && valid_source_field(&row.source)
        && (0.0..=1.0).contains(&row.confidence)
}

fn valid_field(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= MAX_MODEL_FIELD_CHARS
}

fn valid_source_field(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= MAX_MODEL_SOURCE_CHARS
}

/// Returns true when the final path component is a reserved underscore-prefixed
/// draft/scratch directory that authoring stages may create packages in.
pub fn is_reserved_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('_'))
}

/// Which public entry point an authoring call arrived through.
///
/// This is deliberately named for what it measures. It is the call kind, not
/// the authoring stage: `recipe`, `judge`, and `lint` all enter at
/// [`run_model_text`], and `extract` and `consolidate` both enter at
/// [`run_model_extract`], so the two are not separated here. Recording a
/// five-valued stage would require a stage value to reach the call owner,
/// which no current signature carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KmLlmCallKind {
    Text,
    Extract,
}

impl KmLlmCallKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Extract => "extract",
        }
    }
}

/// Bounded, deterministic classification of one authoring call. Every value is
/// decided by the guard path taken inside [`run_model_call`], so the same
/// inputs always produce the same outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KmLlmCallOutcome {
    Disabled,
    OfflineOnlyRefused,
    PromptTooLarge,
    TokenCapExceeded,
    ModelError,
    OutputTooLarge,
    Completed,
}

impl KmLlmCallOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::OfflineOnlyRefused => "offline-only-refused",
            Self::PromptTooLarge => "prompt-too-large",
            Self::TokenCapExceeded => "token-cap-exceeded",
            Self::ModelError => "model-error",
            Self::OutputTooLarge => "output-too-large",
            Self::Completed => "completed",
        }
    }
}

/// Whether a token count came from the model boundary or was estimated locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KmLlmTokenSource {
    #[allow(dead_code)] // reserved: provider token usage is not wired yet
    Returned,
    Estimated,
}

impl KmLlmTokenSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Returned => "returned",
            Self::Estimated => "estimated",
        }
    }
}

/// Metadata-only accounting record for exactly one authoring call.
///
/// It carries call kind, provider, model, token counts with their provenance,
/// duration, and a bounded outcome. Prompt bodies, response bodies, and source
/// contents are structurally absent. Provider and model labels are shape-bounded
/// (single-line, max 64 chars, no path separators or control chars) and rely on
/// provenance: they are built only from config model-provider map keys/slugs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KmLlmCallRecord {
    call_kind: KmLlmCallKind,
    outcome: KmLlmCallOutcome,
    provider_id: String,
    model_slug: String,
    input_tokens: u32,
    output_tokens: u32,
    token_source: KmLlmTokenSource,
}

#[derive(Debug, Serialize)]
struct KmLlmCallLogLine<'a> {
    call_kind: &'a str,
    outcome: &'a str,
    provider: &'a str,
    model: &'a str,
    input_tokens: u32,
    output_tokens: u32,
    token_source: &'a str,
    duration_ms: u128,
}

/// Render one accounting record as a single metadata-only line.
#[cfg(test)]
fn accounting_line(record: &KmLlmCallRecord, duration_ms: u128) -> String {
    format!(
        "km_llm_call call_kind={} outcome={} provider={} model={} input_tokens={} output_tokens={} token_source={} duration_ms={duration_ms}",
        record.call_kind.as_str(),
        record.outcome.as_str(),
        record.provider_id,
        record.model_slug,
        record.input_tokens,
        record.output_tokens,
        record.token_source.as_str(),
    )
}

fn sanitize_accounting_label(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | ':' | '-' => ch,
            _ => '_',
        })
        .take(MAX_ACCOUNTING_LABEL_CHARS)
        .collect();
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn append_call_record(config: &KmLlmClientConfig, record: &KmLlmCallRecord, duration_ms: u128) {
    let Some(project_root) = config.project_root.as_ref() else {
        return;
    };
    let log_path = project_root.join(AUTHORING_CALLS_LOG);
    let Some(parent) = log_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let line = KmLlmCallLogLine {
        call_kind: record.call_kind.as_str(),
        outcome: record.outcome.as_str(),
        provider: &record.provider_id,
        model: &record.model_slug,
        input_tokens: record.input_tokens,
        output_tokens: record.output_tokens,
        token_source: record.token_source.as_str(),
        duration_ms,
    };
    let Ok(serialized) = serde_json::to_string(&line) else {
        return;
    };
    let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    else {
        return;
    };
    let _ = writeln!(file, "{serialized}");
}

/// Metadata-only log line for one stage. Never includes prompt bodies or model
/// output bodies.
#[cfg(test)]
fn metadata_log_line(
    config: &KmLlmClientConfig,
    call_kind: KmLlmCallKind,
    facts: usize,
    mappings: usize,
) -> String {
    format!(
        "km_llm_extract call_kind={} provider={} model={} facts={facts} mappings={mappings}",
        call_kind.as_str(),
        config.provider_id,
        config.model_slug
    )
}

/// Owner-private call boundary shared by both public entry points.
///
/// Every authoring call passes through here exactly once, so exactly one
/// accounting record is emitted per call. `run_model_extract` calls this
/// directly rather than delegating to `run_model_text`, which is what keeps the
/// nested path from double counting.
///
/// Emission is best effort and never fatal: a failure to write the accounting
/// line cannot change what the caller receives.
fn run_model_call(
    config: &KmLlmClientConfig,
    prompt_text: &str,
    model: &dyn KmLlmTextModel,
    call_kind: KmLlmCallKind,
) -> Option<String> {
    let started = std::time::Instant::now();
    let input_tokens = estimate_tokens(prompt_text);

    let (outcome, output, output_tokens) = if !config.enabled {
        (KmLlmCallOutcome::Disabled, None, 0)
    } else if config.offline_only && !config.provider_is_local {
        (KmLlmCallOutcome::OfflineOnlyRefused, None, 0)
    } else if prompt_text.len() > MAX_MODEL_PROMPT_BYTES {
        (KmLlmCallOutcome::PromptTooLarge, None, 0)
    } else if config
        .max_input_tokens
        .is_some_and(|cap| input_tokens > cap)
    {
        (KmLlmCallOutcome::TokenCapExceeded, None, 0)
    } else {
        match model.complete(prompt_text) {
            Err(_) => (KmLlmCallOutcome::ModelError, None, 0),
            Ok(output) if output.len() > MAX_MODEL_OUTPUT_BYTES => {
                let output_tokens = estimate_tokens(&output);
                (KmLlmCallOutcome::OutputTooLarge, None, output_tokens)
            }
            Ok(output) => {
                let output_tokens = estimate_tokens(&output);
                (KmLlmCallOutcome::Completed, Some(output), output_tokens)
            }
        }
    };

    let record = KmLlmCallRecord {
        call_kind,
        outcome,
        provider_id: sanitize_accounting_label(&config.provider_id),
        model_slug: sanitize_accounting_label(&config.model_slug),
        input_tokens,
        output_tokens,
        // The model boundary returns text only, so no provider usage figure is
        // available here. Both counts are local estimates and say so.
        token_source: KmLlmTokenSource::Estimated,
    };
    append_call_record(config, &record, started.elapsed().as_millis());

    output
}

/// Run one bounded model-backed authoring call with deterministic-fallback
/// guards.
///
/// Returns `Some(output)` only when the stage is enabled, the offline-only
/// policy is satisfied, the assembled prompt is within caps, and the model call
/// succeeds. Every other outcome returns `None` so the caller falls back to the
/// deterministic scaffold. A model call never becomes required.
pub fn run_model_text(
    config: &KmLlmClientConfig,
    prompt_text: &str,
    model: &dyn KmLlmTextModel,
) -> Option<String> {
    run_model_call(config, prompt_text, model, KmLlmCallKind::Text)
}

/// Run the model-backed extraction path with deterministic-fallback guards.
///
/// Returns `Some(draft)` only when the stage is enabled, the offline-only
/// policy is satisfied, the assembled prompt is within caps, and the model call
/// succeeds with valid JSON holding at least one valid row (invalid rows are
/// dropped). Every other outcome returns `None` so the caller falls back to
/// the deterministic scaffold. A model call never becomes required.
pub fn run_model_extract(
    config: &KmLlmClientConfig,
    prompt_text: &str,
    model: &dyn KmLlmTextModel,
) -> Option<KmLlmExtractDraft> {
    let remaining = MAX_MODEL_PROMPT_BYTES.saturating_sub(MODEL_EXTRACT_INSTRUCTION.len());
    let mut end = remaining.min(prompt_text.len());
    while !prompt_text.is_char_boundary(end) {
        end -= 1;
    }
    let prompt_text = format!("{MODEL_EXTRACT_INSTRUCTION}{}", &prompt_text[..end]);
    let output = run_model_call(config, &prompt_text, model, KmLlmCallKind::Extract)?;
    let draft = serde_json::from_str(&output).ok()?;
    validate_draft(draft)
}

#[cfg(test)]
#[path = "km_llm_client_tests.rs"]
mod tests;
