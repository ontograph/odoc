//! Converts a rollout-trace bundle into sanitized lineage `RunRecord`s.
//!
//! The bundle is read in place with `serde_json::Value`, so this module does not
//! depend on the rollout-trace crate. Only digests and bounded identifiers leave
//! this module: prompt text, response text, paths and credentials never do.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use crate::lineage_collector::Admission;
use crate::lineage_collector::EvidenceClass;
use crate::lineage_collector::FallbackCategory;
use crate::lineage_collector::MissingCategory;
use crate::lineage_collector::Outcome;
use crate::lineage_collector::RunManifest;
use crate::lineage_collector::RunRecord;
use crate::lineage_collector::Terminal;
use crate::lineage_collector::Variant;

const MAX_TRACE_LINE_BYTES: usize = 1024 * 1024;
const MAX_TRACE_EVENTS: usize = 100_000;
const MAX_PAYLOAD_BYTES: u64 = 64 * 1024 * 1024;
const MAX_FIELD_CHARS: usize = 128;

/// Caller-supplied identity and judgment for one trace. None of it comes from the trace.
pub(crate) struct ConvertInput {
    pub(crate) run_id: String,
    pub(crate) case_id: String,
    pub(crate) variant: Variant,
    pub(crate) corpus_digest: String,
    pub(crate) corpus_revision: String,
    pub(crate) prompt_revision: String,
    pub(crate) config_revision: String,
    pub(crate) schema_revision: String,
    pub(crate) order_seed: u64,
    /// Stable provider key from host configuration, not read from the trace.
    pub(crate) provider_id: String,
    /// The admission tuple observed by the caller; absent means no admission event.
    pub(crate) admission: Option<(Outcome, Admission, FallbackCategory)>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConvertError {
    BundleUnreadable,
    TraceTooLarge,
    MalformedEvent,
    PayloadUnsafe,
    PayloadTooLarge,
    UnsafeField(&'static str),
    NoInference,
    MultipleInferences,
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeField(field) => write!(f, "field `{field}` is not a safe identifier"),
            other => write!(f, "{other:?}"),
        }
    }
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Identifiers must be short and use a conservative charset, so free text cannot fit.
fn safe_identifier(field: &'static str, value: &str) -> Result<(), ConvertError> {
    let ok = !value.is_empty()
        && value.chars().count() <= MAX_FIELD_CHARS
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'));
    if ok {
        Ok(())
    } else {
        Err(ConvertError::UnsafeField(field))
    }
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads a payload by its bundle-relative path, refusing anything that could escape the bundle.
fn read_payload(bundle: &Path, relative: &str) -> Result<Vec<u8>, ConvertError> {
    let relative_path = Path::new(relative);
    let mut components = relative_path.components();
    let in_payloads = matches!(
        (components.next(), components.next(), components.next()),
        (
            Some(std::path::Component::Normal(dir)),
            Some(std::path::Component::Normal(_)),
            None
        ) if dir == "payloads"
    );
    if !in_payloads {
        return Err(ConvertError::PayloadUnsafe);
    }
    let mut file = crate::artifact_io::open_beneath(bundle, relative_path)
        .map_err(|_| ConvertError::PayloadUnsafe)?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(&mut file, MAX_PAYLOAD_BYTES + 1),
        &mut bytes,
    )
    .map_err(|_| ConvertError::BundleUnreadable)?;
    if bytes.len() as u64 > MAX_PAYLOAD_BYTES {
        return Err(ConvertError::PayloadTooLarge);
    }
    Ok(bytes)
}

fn payload_path(event_payload: &Value, key: &str) -> Option<String> {
    event_payload
        .get(key)?
        .get("path")?
        .as_str()
        .map(str::to_owned)
}

/// Converts one single-inference trace bundle into a `Live` run record.
pub(crate) fn convert_bundle(
    bundle: &Path,
    input: &ConvertInput,
) -> Result<RunRecord, ConvertError> {
    for (field, value) in [
        ("run_id", input.run_id.as_str()),
        ("case_id", input.case_id.as_str()),
        ("prompt_revision", input.prompt_revision.as_str()),
        ("config_revision", input.config_revision.as_str()),
        ("schema_revision", input.schema_revision.as_str()),
        ("corpus_revision", input.corpus_revision.as_str()),
        ("provider_id", input.provider_id.as_str()),
    ] {
        safe_identifier(field, value)?;
    }
    if !is_hex_digest(&input.corpus_digest) {
        return Err(ConvertError::UnsafeField("corpus_digest"));
    }

    let trace = crate::artifact_io::read_to_string_beneath(
        bundle,
        Path::new("trace.jsonl"),
        MAX_TRACE_EVENTS * 512,
    )
    .map_err(|_| ConvertError::BundleUnreadable)?;

    let mut started: BTreeMap<String, (i64, String, String)> = BTreeMap::new();
    let mut terminals: BTreeMap<String, (i64, &'static str, Option<String>)> = BTreeMap::new();
    for (index, line) in trace.lines().enumerate() {
        if index >= MAX_TRACE_EVENTS || line.len() > MAX_TRACE_LINE_BYTES {
            return Err(ConvertError::TraceTooLarge);
        }
        if line.trim().is_empty() {
            continue;
        }
        let event: Value = serde_json::from_str(line).map_err(|_| ConvertError::MalformedEvent)?;
        let time = event
            .get("wall_time_unix_ms")
            .and_then(Value::as_i64)
            .ok_or(ConvertError::MalformedEvent)?;
        let payload = event.get("payload").ok_or(ConvertError::MalformedEvent)?;
        let kind = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let call = payload
            .get("inference_call_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        match (kind, call) {
            ("inference_started", Some(call)) => {
                let model = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let request =
                    payload_path(payload, "request_payload").ok_or(ConvertError::MalformedEvent)?;
                started.insert(call, (time, model.to_owned(), request));
            }
            ("inference_completed", Some(call)) => {
                terminals.insert(
                    call,
                    (time, "completed", payload_path(payload, "response_payload")),
                );
            }
            ("inference_failed", Some(call)) => {
                terminals.insert(call, (time, "failed", None));
            }
            ("inference_cancelled", Some(call)) => {
                terminals.insert(call, (time, "cancelled", None));
            }
            _ => {}
        }
    }

    let mut calls = started.into_iter();
    let (call, (start_ms, model, request_path)) = calls.next().ok_or(ConvertError::NoInference)?;
    if calls.next().is_some() {
        return Err(ConvertError::MultipleInferences);
    }
    safe_identifier("model_id", &model)?;

    let request_hash = digest_hex(&read_payload(bundle, &request_path)?);
    let (terminal, elapsed_ms) = match terminals.remove(&call) {
        Some((end_ms, "completed", Some(response_path))) => (
            Terminal::Completed {
                answer_ref: digest_hex(&read_payload(bundle, &response_path)?),
            },
            u64::try_from(end_ms.saturating_sub(start_ms)).ok(),
        ),
        Some((end_ms, "cancelled", _)) => (
            Terminal::Missing(MissingCategory::StreamDropped),
            u64::try_from(end_ms.saturating_sub(start_ms)).ok(),
        ),
        Some((end_ms, _, _)) => (
            Terminal::Missing(MissingCategory::ProviderError),
            u64::try_from(end_ms.saturating_sub(start_ms)).ok(),
        ),
        None => (Terminal::Missing(MissingCategory::UnknownRemoteState), None),
    };

    Ok(RunRecord {
        run_id: input.run_id.clone(),
        case_id: input.case_id.clone(),
        variant: input.variant,
        evidence_class: EvidenceClass::Live,
        corpus_digest: input.corpus_digest.clone(),
        prompt_revision: input.prompt_revision.clone(),
        config_revision: input.config_revision.clone(),
        schema_revision: input.schema_revision.clone(),
        model_id: Some(model),
        request_hash: matches!(terminal, Terminal::Completed { .. }).then_some(request_hash),
        admission: input.admission,
        terminal,
        elapsed_ms,
        manifest: RunManifest {
            corpus_revision: input.corpus_revision.clone(),
            order_seed: input.order_seed,
            provider_id: Some(input.provider_id.clone()),
            started_at: Some(start_ms.to_string()),
        },
        demand_coverage: None,
        provenance_validity: None,
        answer_quality: None,
    })
}

#[cfg(test)]
#[path = "lineage_trace_convert_tests.rs"]
mod tests;
