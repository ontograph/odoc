//! Pure paired-result validation and collection for the LRA-0.3 protocol.

use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;

use CollectError::Overflow;
use InvalidReason as Reason;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Variant {
    Baseline,
    Lineage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceClass {
    Offline,
    Mock,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Baseline,
    Lineage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Admission {
    Accepted,
    Rejected,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FallbackCategory {
    None,
    Disabled,
    NotEligible,
    InductionFailed,
    RetrievalFailed,
    SelectionFailed,
    SchemaInvalid,
    Timeout,
    BudgetExhausted,
    TokenCapExceeded,
    PartialCoverage,
    NoUsableEvidence,
    ScopeInvalid,
    SourceStale,
    SourceUnavailable,
    ByteCapExceeded,
    RenderInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MissingCategory {
    NoAdmissionEvent,
    StreamDropped,
    DeadlineBudgetExhausted,
    ProviderTimeout,
    ProviderError,
    UnknownRemoteState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Terminal {
    Completed { answer_ref: String },
    Missing(MissingCategory),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Counts {
    pub(crate) numerator: u64,
    pub(crate) denominator: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct AnswerQuality {
    pub(crate) rubric_id: String,
    pub(crate) score: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct RunManifest {
    pub(crate) corpus_revision: String,
    pub(crate) order_seed: u64,
    pub(crate) provider_id: Option<String>,
    pub(crate) started_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct RunRecord {
    pub(crate) run_id: String,
    pub(crate) case_id: String,
    pub(crate) variant: Variant,
    pub(crate) evidence_class: EvidenceClass,
    pub(crate) corpus_digest: String,
    pub(crate) prompt_revision: String,
    pub(crate) config_revision: String,
    pub(crate) schema_revision: String,
    pub(crate) model_id: Option<String>,
    pub(crate) request_hash: Option<String>,
    pub(crate) admission: Option<(Outcome, Admission, FallbackCategory)>,
    pub(crate) terminal: Terminal,
    pub(crate) elapsed_ms: Option<u64>,
    pub(crate) manifest: RunManifest,
    pub(crate) demand_coverage: Option<Counts>,
    pub(crate) provenance_validity: Option<Counts>,
    pub(crate) answer_quality: Option<AnswerQuality>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Metric {
    pub(crate) counts: Counts,
    pub(crate) measured_runs: u64,
    pub(crate) basis_points: Option<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Latency {
    pub(crate) measured_runs: u64,
    pub(crate) p50_ms: Option<u64>,
    pub(crate) p95_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct VariantTable {
    pub(crate) completed_runs: u64,
    pub(crate) demand_coverage: Metric,
    pub(crate) provenance_validity: Metric,
    pub(crate) fallback_rate: Metric,
    pub(crate) fallback_categories: BTreeMap<FallbackCategory, u64>,
    pub(crate) latency: Latency,
    pub(crate) missing: BTreeMap<MissingCategory, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct PairedDelta {
    pub(crate) demand_coverage_bp: Option<i32>,
    pub(crate) provenance_validity_bp: Option<i32>,
    pub(crate) elapsed_ms: Option<i128>,
    pub(crate) quality_score: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct PairRow {
    pub(crate) baseline: RunRecord,
    pub(crate) lineage: RunRecord,
    pub(crate) delta: PairedDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct PairedReport {
    pub(crate) tables: BTreeMap<EvidenceClass, BTreeMap<Variant, VariantTable>>,
    pub(crate) pairs: Vec<PairRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidReason {
    EmptyInput,
    DuplicateRunId,
    DuplicateVariant,
    IncompletePair,
    MissingManifestInput { field: String },
    RevisionMismatch { field: String },
    InvalidAdmission,
    InvalidObservation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) struct Invalid {
    pub(crate) case_id: Option<String>,
    pub(crate) run_ids: Vec<String>,
    pub(crate) reason: InvalidReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CollectError {
    Invalid(Vec<Invalid>),
    Overflow,
}

type Groups<'a> = BTreeMap<(EvidenceClass, &'a str), Vec<&'a RunRecord>>;

fn problem(
    errors: &mut Vec<Invalid>,
    case_id: Option<&str>,
    runs: &[&RunRecord],
    reason: InvalidReason,
) {
    let mut run_ids: Vec<_> = runs.iter().map(|run| run.run_id.clone()).collect();
    run_ids.sort();
    errors.push(Invalid {
        case_id: case_id.map(str::to_owned),
        run_ids,
        reason,
    });
}

fn identities(run: &RunRecord) -> [(&str, &str); 7] {
    [
        ("run_id", &run.run_id),
        ("case_id", &run.case_id),
        ("corpus_digest", &run.corpus_digest),
        ("prompt_revision", &run.prompt_revision),
        ("config_revision", &run.config_revision),
        ("schema_revision", &run.schema_revision),
        ("manifest.corpus_revision", &run.manifest.corpus_revision),
    ]
}

fn valid_admission((outcome, admission, category): (Outcome, Admission, FallbackCategory)) -> bool {
    let expected = match category {
        FallbackCategory::None => (Outcome::Lineage, Admission::Accepted),
        FallbackCategory::Disabled => (Outcome::Baseline, Admission::Disabled),
        FallbackCategory::NotEligible
        | FallbackCategory::InductionFailed
        | FallbackCategory::RetrievalFailed
        | FallbackCategory::SelectionFailed
        | FallbackCategory::SchemaInvalid
        | FallbackCategory::Timeout
        | FallbackCategory::BudgetExhausted
        | FallbackCategory::TokenCapExceeded
        | FallbackCategory::PartialCoverage
        | FallbackCategory::NoUsableEvidence
        | FallbackCategory::ScopeInvalid
        | FallbackCategory::SourceStale
        | FallbackCategory::SourceUnavailable
        | FallbackCategory::ByteCapExceeded
        | FallbackCategory::RenderInvalid => (Outcome::Baseline, Admission::Rejected),
    };
    (outcome, admission) == expected
}

fn validate(records: &[RunRecord]) -> Result<Groups<'_>, CollectError> {
    let mut errors = Vec::new();
    let mut groups: Groups<'_> = BTreeMap::new();
    let mut ids: BTreeMap<&str, Vec<&RunRecord>> = BTreeMap::new();
    let mut classes: BTreeMap<EvidenceClass, Vec<&RunRecord>> = BTreeMap::new();
    if records.is_empty() {
        problem(
            &mut errors,
            /*case_id*/ None,
            &[],
            InvalidReason::EmptyInput,
        );
    }
    for run in records {
        groups
            .entry((run.evidence_class, &run.case_id))
            .or_default()
            .push(run);
        ids.entry(&run.run_id).or_default().push(run);
        classes.entry(run.evidence_class).or_default().push(run);
        let mut invalid = |reason| problem(&mut errors, Some(&run.case_id), &[run], reason);
        let mut required = identities(run).to_vec();
        let remote = run.evidence_class != EvidenceClass::Offline;
        if remote {
            required.push(("model_id", run.model_id.as_deref().unwrap_or_default()));
        }
        if run.evidence_class == EvidenceClass::Live {
            required.extend([
                (
                    "manifest.provider_id",
                    run.manifest.provider_id.as_deref().unwrap_or_default(),
                ),
                (
                    "manifest.started_at",
                    run.manifest.started_at.as_deref().unwrap_or_default(),
                ),
            ]);
        }
        if remote && matches!(run.terminal, Terminal::Completed { .. }) {
            let hash = run.request_hash.as_deref().unwrap_or_default();
            required.push(("request_hash", hash));
            if !hash.trim().is_empty()
                && (hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
            {
                invalid(Reason::InvalidObservation);
            }
        }
        for (field, value) in required {
            if value.trim().is_empty() {
                invalid(Reason::MissingManifestInput {
                    field: field.to_owned(),
                });
            }
        }
        if run.admission.is_some_and(|tuple| !valid_admission(tuple))
            || (run.terminal == Terminal::Missing(MissingCategory::NoAdmissionEvent)
                && run.admission.is_some())
        {
            invalid(Reason::InvalidAdmission);
        }
        for counts in [&run.demand_coverage, &run.provenance_validity]
            .into_iter()
            .flatten()
        {
            if counts.numerator > counts.denominator {
                invalid(Reason::InvalidObservation);
            }
        }
    }
    for runs in ids.values().filter(|runs| runs.len() > 1) {
        problem(
            &mut errors,
            /*case_id*/ None,
            runs,
            InvalidReason::DuplicateRunId,
        );
    }
    for ((_, case_id), runs) in &groups {
        let mut invalid =
            |subset: &[&RunRecord], reason| problem(&mut errors, Some(case_id), subset, reason);
        for variant in [Variant::Baseline, Variant::Lineage] {
            let mut variants: Vec<_> = runs
                .iter()
                .copied()
                .filter(|run| run.variant == variant)
                .collect();
            variants.sort_by(|left, right| left.run_id.cmp(&right.run_id));
            if variants.is_empty() {
                invalid(runs, Reason::IncompletePair);
            } else if variants.len() > 1
                && let Some(extras) = variants.get(1..)
            {
                invalid(extras, Reason::DuplicateVariant);
            }
        }
        if runs.iter().any(|run| {
            runs.first()
                .is_some_and(|first| run.model_id != first.model_id)
        }) {
            invalid(
                runs,
                Reason::RevisionMismatch {
                    field: "model_id".to_owned(),
                },
            );
        }
    }
    for runs in classes.values() {
        let mut mismatch = |field: &str| {
            problem(
                &mut errors,
                /*case_id*/ None,
                runs,
                Reason::RevisionMismatch {
                    field: field.to_owned(),
                },
            )
        };
        if let Some(first) = runs.first() {
            for (field, value) in identities(first).into_iter().skip(/*n*/ 2) {
                if runs.iter().any(|run| {
                    identities(run)
                        .into_iter()
                        .any(|(name, other)| name == field && other != value)
                }) {
                    mismatch(field);
                }
            }
            if runs
                .iter()
                .any(|run| run.manifest.order_seed != first.manifest.order_seed)
            {
                mismatch("manifest.order_seed");
            }
        }
    }
    errors.sort();
    if errors.is_empty() {
        Ok(groups)
    } else {
        Err(CollectError::Invalid(errors))
    }
}

fn add(target: &mut u64, value: u64) -> Result<(), CollectError> {
    *target = target.checked_add(value).ok_or(Overflow)?;
    Ok(())
}

fn basis_points(counts: &Counts) -> Result<Option<u16>, CollectError> {
    if counts.denominator == 0 {
        return Ok(None);
    }
    let scaled = u128::from(counts.numerator)
        .checked_mul(/*rhs*/ 10000)
        .ok_or(Overflow)?;
    u16::try_from(scaled / u128::from(counts.denominator))
        .map(Some)
        .map_err(|_| Overflow)
}

fn observe(metric: &mut Metric, counts: &Counts) -> Result<(), CollectError> {
    add(&mut metric.counts.numerator, counts.numerator)?;
    add(&mut metric.counts.denominator, counts.denominator)?;
    add(&mut metric.measured_runs, /*value*/ 1)
}

fn percentile(values: &[u64], percent: u128) -> Result<Option<u64>, CollectError> {
    if values.is_empty() {
        return Ok(None);
    }
    let rank = (values.len() as u128)
        .checked_mul(percent)
        .and_then(|n| n.checked_add(/*rhs*/ 99))
        .ok_or(Overflow)?
        / 100;
    let index =
        usize::try_from(rank.checked_sub(/*rhs*/ 1).ok_or(Overflow)?).map_err(|_| Overflow)?;
    values.get(index).copied().map(Some).ok_or(Overflow)
}

fn table(runs: &[&RunRecord]) -> Result<VariantTable, CollectError> {
    let mut table = VariantTable::default();
    let mut elapsed = Vec::new();
    for run in runs {
        match run.terminal {
            Terminal::Missing(category) => {
                add(table.missing.entry(category).or_default(), /*value*/ 1)?;
                continue;
            }
            Terminal::Completed { .. } => add(&mut table.completed_runs, /*value*/ 1)?,
        }
        for (metric, counts) in [
            (&mut table.demand_coverage, &run.demand_coverage),
            (&mut table.provenance_validity, &run.provenance_validity),
        ] {
            if let Some(counts) = counts {
                observe(metric, counts)?;
            }
        }
        if let Some((outcome, _, category)) = run.admission {
            observe(
                &mut table.fallback_rate,
                &Counts {
                    numerator: u64::from(outcome == Outcome::Baseline),
                    denominator: 1,
                },
            )?;
            add(
                table.fallback_categories.entry(category).or_default(),
                /*value*/ 1,
            )?;
        }
        if let Some(ms) = run.elapsed_ms {
            elapsed.push(ms);
        }
    }
    for metric in [
        &mut table.demand_coverage,
        &mut table.provenance_validity,
        &mut table.fallback_rate,
    ] {
        metric.basis_points = basis_points(&metric.counts)?;
    }
    elapsed.sort_unstable();
    table.latency = Latency {
        measured_runs: u64::try_from(elapsed.len()).map_err(|_| Overflow)?,
        p50_ms: percentile(&elapsed, /*percent*/ 50)?,
        p95_ms: percentile(&elapsed, /*percent*/ 95)?,
    };
    Ok(table)
}

fn metric_delta(
    before: &Option<Counts>,
    after: &Option<Counts>,
) -> Result<Option<i32>, CollectError> {
    if let (Some(before), Some(after)) = (before, after)
        && let (Some(before), Some(after)) = (basis_points(before)?, basis_points(after)?)
    {
        return i32::from(after)
            .checked_sub(i32::from(before))
            .map(Some)
            .ok_or(Overflow);
    }
    Ok(None)
}

fn delta(baseline: &RunRecord, lineage: &RunRecord) -> Result<PairedDelta, CollectError> {
    let mut delta = PairedDelta::default();
    if matches!(baseline.terminal, Terminal::Missing(_))
        || matches!(lineage.terminal, Terminal::Missing(_))
    {
        return Ok(delta);
    }
    delta.demand_coverage_bp = metric_delta(&baseline.demand_coverage, &lineage.demand_coverage)?;
    delta.provenance_validity_bp =
        metric_delta(&baseline.provenance_validity, &lineage.provenance_validity)?;
    if let (Some(before), Some(after)) = (baseline.elapsed_ms, lineage.elapsed_ms) {
        delta.elapsed_ms = Some(
            i128::from(after)
                .checked_sub(i128::from(before))
                .ok_or(Overflow)?,
        );
    }
    if let (Some(before), Some(after)) = (&baseline.answer_quality, &lineage.answer_quality)
        && before.rubric_id == after.rubric_id
        && let (Some(before), Some(after)) = (before.score, after.score)
    {
        delta.quality_score = Some(
            i16::from(after)
                .checked_sub(i16::from(before))
                .ok_or(Overflow)?,
        );
    }
    Ok(delta)
}

pub(crate) fn collect(records: &[RunRecord]) -> Result<PairedReport, CollectError> {
    let groups = validate(records)?;
    let mut table_runs: BTreeMap<(EvidenceClass, Variant), Vec<&RunRecord>> = BTreeMap::new();
    let mut pairs = Vec::new();
    for ((class, case_id), runs) in groups {
        let baseline = runs
            .iter()
            .copied()
            .find(|run| run.variant == Variant::Baseline);
        let lineage = runs
            .iter()
            .copied()
            .find(|run| run.variant == Variant::Lineage);
        let (Some(baseline), Some(lineage)) = (baseline, lineage) else {
            let mut errors = Vec::new();
            problem(&mut errors, Some(case_id), &runs, Reason::IncompletePair);
            return Err(CollectError::Invalid(errors));
        };
        for run in [baseline, lineage] {
            table_runs
                .entry((class, run.variant))
                .or_default()
                .push(run);
        }
        pairs.push(PairRow {
            baseline: baseline.clone(),
            lineage: lineage.clone(),
            delta: delta(baseline, lineage)?,
        });
    }
    let mut tables: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for ((class, variant), runs) in table_runs {
        tables
            .entry(class)
            .or_default()
            .insert(variant, table(&runs)?);
    }
    Ok(PairedReport { tables, pairs })
}

// ---------------------------------------------------------------------------
// JSON-lines ingestion
//
// Records from live runs arrive as JSON lines. Ingestion is bounded (line
// length, record count, identifier length) and refuses raw text where an opaque
// reference is required, so a report can never carry a passage or an answer.
// ---------------------------------------------------------------------------

pub(crate) const MAX_JSONL_LINE_BYTES: usize = 65_536;
pub(crate) const MAX_JSONL_RECORDS: usize = 100_000;
pub(crate) const MAX_IDENTIFIER_CHARS: usize = 256;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IngestError {
    /// 1-based input line, or 0 when the failure is not tied to a line.
    pub(crate) line: usize,
    pub(crate) reason: IngestReason,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum IngestReason {
    Io(String),
    LineTooLong,
    TooManyRecords,
    /// Carries only the parser's category, line and column, never line content.
    Parse(String),
    UnboundedIdentifier {
        field: &'static str,
    },
    RawTextInReference {
        field: &'static str,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CollectJsonlError {
    Ingest(IngestError),
    Collect(CollectError),
}

fn bounded_identifier(field: &'static str, value: &str) -> Result<(), IngestReason> {
    if value.chars().count() > MAX_IDENTIFIER_CHARS || value.chars().any(char::is_control) {
        return Err(IngestReason::UnboundedIdentifier { field });
    }
    Ok(())
}

fn check_record(record: &RunRecord) -> Result<(), IngestReason> {
    bounded_identifier("run_id", &record.run_id)?;
    bounded_identifier("case_id", &record.case_id)?;
    bounded_identifier("corpus_digest", &record.corpus_digest)?;
    bounded_identifier("prompt_revision", &record.prompt_revision)?;
    bounded_identifier("config_revision", &record.config_revision)?;
    bounded_identifier("schema_revision", &record.schema_revision)?;
    bounded_identifier("manifest.corpus_revision", &record.manifest.corpus_revision)?;
    for (field, value) in [
        ("model_id", &record.model_id),
        ("manifest.provider_id", &record.manifest.provider_id),
        ("manifest.started_at", &record.manifest.started_at),
        ("request_hash", &record.request_hash),
    ] {
        if let Some(value) = value {
            bounded_identifier(field, value)?;
        }
    }
    if let Terminal::Completed { answer_ref } = &record.terminal {
        bounded_identifier("answer_ref", answer_ref)?;
        if answer_ref.chars().any(char::is_whitespace) {
            return Err(IngestReason::RawTextInReference {
                field: "answer_ref",
            });
        }
    }
    if let Some(quality) = &record.answer_quality {
        bounded_identifier("answer_quality.rubric_id", &quality.rubric_id)?;
    }
    Ok(())
}

/// Reads one line without buffering more than `MAX_JSONL_LINE_BYTES` of it.
fn read_bounded_line(
    reader: &mut impl std::io::BufRead,
    line: &mut Vec<u8>,
) -> Result<bool, IngestReason> {
    line.clear();
    loop {
        let (consumed, done) = {
            let chunk = reader
                .fill_buf()
                .map_err(|error| IngestReason::Io(error.kind().to_string()))?;
            if chunk.is_empty() {
                return Ok(!line.is_empty());
            }
            let (take, done) = match chunk.iter().position(|byte| *byte == b'\n') {
                Some(index) => (index + 1, true),
                None => (chunk.len(), false),
            };
            if line.len() + take > MAX_JSONL_LINE_BYTES + 1 {
                return Err(IngestReason::LineTooLong);
            }
            line.extend_from_slice(&chunk[..take]);
            (take, done)
        };
        reader.consume(consumed);
        if done {
            return Ok(true);
        }
    }
}

pub(crate) fn ingest_jsonl(
    mut reader: impl std::io::BufRead,
) -> Result<Vec<RunRecord>, IngestError> {
    let mut records = Vec::new();
    let mut line = Vec::new();
    let mut number = 0usize;
    loop {
        number += 1;
        let at = |reason| IngestError {
            line: number,
            reason,
        };
        if !read_bounded_line(&mut reader, &mut line).map_err(at)? {
            return Ok(records);
        }
        let content = line.trim_ascii();
        if content.is_empty() {
            continue;
        }
        if content.len() > MAX_JSONL_LINE_BYTES {
            return Err(at(IngestReason::LineTooLong));
        }
        if records.len() >= MAX_JSONL_RECORDS {
            return Err(at(IngestReason::TooManyRecords));
        }
        let record: RunRecord = serde_json::from_slice(content).map_err(|error| {
            at(IngestReason::Parse(format!(
                "{:?} at line {} column {}",
                error.classify(),
                error.line(),
                error.column()
            )))
        })?;
        check_record(&record).map_err(at)?;
        records.push(record);
    }
}

pub(crate) fn render_report(report: &PairedReport) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(report)?;
    text.push('\n');
    Ok(text)
}

pub(crate) fn collect_jsonl(
    reader: impl std::io::BufRead,
) -> Result<PairedReport, CollectJsonlError> {
    let records = ingest_jsonl(reader).map_err(CollectJsonlError::Ingest)?;
    collect(&records).map_err(CollectJsonlError::Collect)
}

#[cfg(test)]
#[path = "lineage_collector_tests.rs"]
mod tests;
