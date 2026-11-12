//! Integrity gates for the benchmark. Each check returns zero or more
//! [`GateFailure`]s; a non-empty result stops the run.
//!
//! Stages select which gates apply:
//! - `pools`: query floors, dev/test leakage, pool reproducibility, corpus pins.
//! - `qrels`: everything in `pools`, plus judged sections exist with their
//!   stored span hash.
//! - `run`: everything in `qrels`, plus judged@10 >= 0.95 per system.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::corpus::sha256_hex;
use crate::search::tokenize::tokenize;

use super::run::{
    Bench, EvalError, MAX_RECORDS, Query, Report, Splits, Stratum, parse_jsonl_within,
    pool_captured, split,
};

pub const LEAK_JACCARD: f64 = 0.8;
pub const MIN_JUDGED: f64 = 0.95;
/// Independent reviewer required for model-drafted queries (ADR, EVAL-2.0).
pub const QUERY_REVIEW_MODEL: &str = "gpt-6-astra";
const MAX_ID_BYTES: usize = 128;
const MAX_NOTE_BYTES: usize = 4096;
const MAX_REVIEW_ENTRIES: usize = 100_000;
const MAX_REVIEW_BYTES: usize = 16 * 1024 * 1024;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Pools,
    Qrels,
    Run,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GateFailure {
    pub gate: &'static str,
    pub detail: String,
}

fn fail(gate: &'static str, detail: String) -> GateFailure {
    GateFailure { gate, detail }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PartyKind {
    Human,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Party {
    id: String,
    kind: PartyKind,
    #[serde(default)]
    model: Option<String>,
    evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ReviewDecision {
    Accept,
    Revise,
    Reject,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    text_sha256: String,
    predecessor_sha256: Option<String>,
    author: Party,
    reviewer: Party,
    decision: ReviewDecision,
}

/// One version-1 `query-review.jsonl` record (ADR "Trusted Approval And
/// Query Review").
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryReview {
    version: u32,
    query_id: String,
    final_text_sha256: String,
    corpus: String,
    stratum: Stratum,
    model_drafted: bool,
    stratum_evidence: String,
    author: Party,
    reviewer: Party,
    decision: ReviewDecision,
    rationale: String,
    history: Vec<Revision>,
}

type ReviewedQuery = (String, String, Stratum, bool);

/// Operator-pinned query approval, minted only by [`check_query_review`].
/// It records pre-ranking review of exactly these final query texts; it does
/// not authenticate reviewers or prove chronology by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryApproval {
    sha256: String,
    reviewed: BTreeMap<String, ReviewedQuery>,
    model_drafted: usize,
}

impl QueryApproval {
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn reviewed(&self) -> usize {
        self.reviewed.len()
    }

    pub fn model_drafted(&self) -> usize {
        self.model_drafted
    }

    /// Exactly these queries, with identical final text, corpus, stratum and
    /// drafting origin, were accepted.
    pub(crate) fn covers(&self, queries: &[Query]) -> bool {
        queries.len() == self.reviewed.len()
            && queries.iter().all(|q| {
                self.reviewed.get(&q.id).is_some_and(|r| {
                    *r == (
                        sha256_hex(q.text.as_bytes()),
                        q.corpus.clone(),
                        q.stratum,
                        q.model_drafted,
                    )
                })
            })
    }
}

fn bounded(value: &str, cap: usize) -> bool {
    !value.trim().is_empty() && value.len() <= cap
}

fn party_valid(party: &Party) -> bool {
    bounded(&party.id, MAX_ID_BYTES)
        && bounded(&party.evidence, MAX_NOTE_BYTES)
        && match (party.kind, &party.model) {
            (PartyKind::Human, None) => true,
            (PartyKind::Model, Some(model)) => bounded(model, MAX_ID_BYTES),
            _ => false,
        }
}

/// A separate review interaction: valid parties with distinct identity and
/// evidence. A model reviewer is never counted as human.
fn independent(author: &Party, reviewer: &Party) -> bool {
    party_valid(author)
        && party_valid(reviewer)
        && author.id != reviewer.id
        && author.evidence != reviewer.evidence
}

fn astra(party: &Party) -> bool {
    party.kind == PartyKind::Model && party.model.as_deref() == Some(QUERY_REVIEW_MODEL)
}

fn review_error(detail: impl std::fmt::Display) -> EvalError {
    EvalError::Invalid(format!("query review refused: {detail}"))
}

/// Validates operator-pinned query-review bytes against the final queries
/// before any ranking. The pin comes from the trusted preparation audit; the
/// bytes are never hashed into their own approval.
pub fn check_query_review(
    queries: &[Query],
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<QueryApproval, EvalError> {
    let mut budget = MAX_RECORDS;
    check_query_review_within(queries, bytes, expected_sha256, &mut budget)
}

/// [`check_query_review`] charging records and history entries to a shared
/// capture record budget before they are retained.
pub(crate) fn check_query_review_within(
    queries: &[Query],
    bytes: &[u8],
    expected_sha256: &str,
    records_budget: &mut usize,
) -> Result<QueryApproval, EvalError> {
    if expected_sha256.len() != 64
        || !expected_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(review_error("pinned hash must be 64 lowercase hex digits"));
    }
    let sha256 = sha256_hex(bytes);
    if sha256 != expected_sha256 {
        return Err(review_error("bytes differ from the pinned hash"));
    }
    if bytes.len() > MAX_REVIEW_BYTES {
        return Err(review_error("input byte limit exceeded"));
    }
    // Each line is bounded by the shared JSONL parser; a record and its
    // history entries are charged to the shared budget before retention.
    let records: Vec<QueryReview> = parse_jsonl_within(
        Path::new("query-review.jsonl"),
        bytes,
        records_budget,
        |r: &QueryReview| 1 + r.history.len(),
    )?;
    let by_id: BTreeMap<&str, &Query> = queries.iter().map(|q| (q.id.as_str(), q)).collect();
    let mut reviewed = BTreeMap::new();
    let mut model_drafted = 0;
    for r in &records {
        let Some(q) = by_id.get(r.query_id.as_str()) else {
            return Err(review_error(format!("unknown query {:?}", r.query_id)));
        };
        let text_sha256 = sha256_hex(q.text.as_bytes());
        let id = &r.query_id;
        if r.query_id.len() > MAX_ID_BYTES || r.corpus.len() > MAX_ID_BYTES {
            return Err(review_error("identifier exceeds 128 bytes"));
        }
        if r.version != 1 {
            return Err(review_error(format!("{id}: unsupported version")));
        }
        if r.final_text_sha256 != text_sha256
            || r.corpus != q.corpus
            || r.stratum != q.stratum
            || r.model_drafted != q.model_drafted
        {
            return Err(review_error(format!("{id}: differs from the final query")));
        }
        if r.decision != ReviewDecision::Accept {
            return Err(review_error(format!("{id}: final decision is not accept")));
        }
        if !bounded(&r.rationale, MAX_NOTE_BYTES)
            || !bounded(&r.stratum_evidence, MAX_NOTE_BYTES)
            || !independent(&r.author, &r.reviewer)
            || (r.model_drafted && !astra(&r.reviewer))
        {
            return Err(review_error(format!(
                "{id}: missing, oversized or non-independent review evidence"
            )));
        }
        // Ordered history: each entry names its predecessor's digest and has
        // an independent reviewer. Earlier decisions may be any value; the
        // first author fixes the drafting origin, and the last entry is the
        // accepted final review.
        let mut previous: Option<&str> = None;
        for h in &r.history {
            if h.predecessor_sha256.as_deref() != previous
                || !valid_hex(&h.text_sha256, 64)
                || !independent(&h.author, &h.reviewer)
            {
                return Err(review_error(format!("{id}: invalid revision history")));
            }
            previous = Some(&h.text_sha256);
        }
        if r.history.len() > MAX_REVIEW_ENTRIES
            || r.history
                .first()
                .is_none_or(|h| (h.author.kind == PartyKind::Model) != r.model_drafted)
        {
            return Err(review_error(format!(
                "{id}: drafting origin differs from the first revision"
            )));
        }
        let final_entry = r.history.last();
        if final_entry.is_none_or(|h| {
            h.text_sha256 != text_sha256
                || h.decision != ReviewDecision::Accept
                || h.author != r.author
                || h.reviewer != r.reviewer
        }) {
            return Err(review_error(format!(
                "{id}: history does not end at the reviewed final text"
            )));
        }
        model_drafted += usize::from(r.model_drafted);
        let value = (text_sha256, q.corpus.clone(), q.stratum, q.model_drafted);
        if reviewed.insert(r.query_id.clone(), value).is_some() {
            return Err(review_error(format!("duplicate record {id}")));
        }
    }
    if let Some(q) = queries.iter().find(|q| !reviewed.contains_key(&q.id)) {
        return Err(review_error(format!("missing record for {}", q.id)));
    }
    Ok(QueryApproval {
        sha256,
        reviewed,
        model_drafted,
    })
}

/// Fixed agreement protocol (ADR "Human Annotation Companion").
pub const MIN_KAPPA: f64 = 0.6;
pub const AGREEMENT_RESAMPLES: usize = 10_000;
/// Declared overlap algorithm: per (corpus, stratum) group of answerable
/// queries, sorted ids shuffled by one SplitMix64 stream seeded with the
/// notes' assignment seed in sorted group order; the first ceil(30%) overlap.
pub const OVERLAP_POLICY: &str = "corpus-stratum-ceil30-v1";
/// ADR revision 13: labels from two independent model judges; exploratory only.
pub const MODEL_JUDGED: &str = "model-judged";
const MAX_NOTES_BYTES: usize = 1024 * 1024;

/// Raw annotation bytes for the qrels stage; never self-approving.
#[derive(Debug, Clone, Copy)]
pub struct AnnotationInput<'a> {
    pub records: &'a [u8],
    pub notes: &'a [u8],
    /// Remaining shared capture record budget; grade entries are charged.
    pub budget: usize,
}

/// Trusted-preparation evidence consumed by [`check_benchmark`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Provenance<'a> {
    pub approval: Option<&'a QueryApproval>,
    pub annotation: Option<AnnotationInput<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnnotationNotes {
    version: u32,
    guideline_version: String,
    annotators: Vec<Party>,
    assignment_seed: u64,
    overlap: String,
    blinding_evidence: String,
    adjudication_rule: String,
    /// Absent: human annotators (ADR9). [`MODEL_JUDGED`]: ADR revision 13.
    #[serde(default)]
    label_provenance: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGrade {
    annotator: String,
    grade: u8,
    evidence: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelDraft {
    model: String,
    grade: u8,
    evidence: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftReview {
    reviewer: String,
    evidence: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Adjudication {
    grade: u8,
    adjudicator: Party,
    evidence: String,
}

/// One version-1 `annotation.jsonl` record keyed by `(query_id, section_id)`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnnotationRecord {
    version: u32,
    query_id: String,
    section_id: String,
    span_sha256: String,
    #[serde(default)]
    draft: Option<ModelDraft>,
    #[serde(default)]
    draft_review: Option<DraftReview>,
    grades: Vec<RawGrade>,
    #[serde(rename = "final")]
    adjudication: Adjudication,
}

/// Pre-adjudication agreement on the predeclared overlap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agreement {
    pub policy: String,
    pub answerable_queries: usize,
    pub overlap_queries: usize,
    pub overlap_pairs: usize,
    pub kappa: Option<f64>,
    pub ci_lo: Option<f64>,
    pub ci_hi: Option<f64>,
    pub seed: u64,
    pub resamples: usize,
    pub undefined_resamples: usize,
    /// Raw overlap grades 0, 1 and 2 from both annotators.
    pub prevalence: [usize; 3],
    /// [`MODEL_JUDGED`] for ADR revision 13 labels; absent for human labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_provenance: Option<String>,
}

/// The single fenced ```annotation-notes JSON block of `annotation.md`.
fn notes_block(notes: &[u8]) -> Option<&str> {
    let text = std::str::from_utf8(notes).ok()?;
    let mut blocks = Vec::new();
    let mut open: Option<usize> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end();
        match open {
            None if trimmed == "```annotation-notes" => open = Some(offset + line.len()),
            Some(start) if trimmed == "```" => {
                blocks.push(&text[start..offset]);
                open = None;
            }
            _ => {}
        }
        offset += line.len();
    }
    (open.is_none() && blocks.len() == 1).then(|| blocks[0])
}

fn parse_notes(notes: &[u8]) -> Option<AnnotationNotes> {
    if notes.len() > MAX_NOTES_BYTES {
        return None;
    }
    let n: AnnotationNotes = serde_json::from_str(notes_block(notes)?).ok()?;
    let kind = match n.label_provenance.as_deref() {
        None => PartyKind::Human,
        Some(MODEL_JUDGED) => PartyKind::Model,
        Some(_) => return None,
    };
    let parties = n.annotators.len() == 2
        && n.annotators
            .iter()
            .all(|a| a.kind == kind && party_valid(a))
        && independent(&n.annotators[0], &n.annotators[1])
        && (kind == PartyKind::Human || n.annotators[0].model != n.annotators[1].model);
    (n.version == 1
        && parties
        && n.overlap == OVERLAP_POLICY
        && bounded(&n.guideline_version, MAX_ID_BYTES)
        && bounded(&n.blinding_evidence, MAX_NOTE_BYTES)
        && bounded(&n.adjudication_rule, MAX_NOTE_BYTES))
    .then_some(n)
}

/// Deterministic stratified overlap selection ([`OVERLAP_POLICY`]).
fn overlap_selection(queries: &[Query], seed: u64) -> BTreeSet<&str> {
    let mut groups: BTreeMap<(&str, Stratum), Vec<&str>> = BTreeMap::new();
    for q in queries.iter().filter(|q| q.stratum != Stratum::OffTopic) {
        groups
            .entry((q.corpus.as_str(), q.stratum))
            .or_default()
            .push(q.id.as_str());
    }
    let mut rng = super::stats::SplitMix64::new(seed);
    let mut selected = BTreeSet::new();
    for ids in groups.values_mut() {
        ids.sort_unstable();
        for i in (1..ids.len()).rev() {
            ids.swap(i, rng.below(i + 1));
        }
        selected.extend(ids[..(3 * ids.len()).div_ceil(10)].iter().copied());
    }
    selected
}

type Counts = [[f64; 3]; 3];

fn percentile(sorted: &[f64], p: f64) -> Option<f64> {
    (!sorted.is_empty()).then(|| super::stats::quantile(sorted, p))
}

/// Point kappa and a percentile 95% CI from query-cluster resamples.
fn cluster_agreement(
    clusters: &[Counts],
    seed: u64,
) -> (Option<f64>, Option<f64>, Option<f64>, usize) {
    let mut total = [[0.0; 3]; 3];
    for c in clusters {
        for i in 0..3 {
            for j in 0..3 {
                total[i][j] += c[i][j];
            }
        }
    }
    let kappa = crate::eval::stats::kappa_from_counts(&total);
    if clusters.is_empty() {
        return (kappa, None, None, 0);
    }
    let mut rng = super::stats::SplitMix64::new(seed);
    let mut values = Vec::with_capacity(AGREEMENT_RESAMPLES);
    for _ in 0..AGREEMENT_RESAMPLES {
        let mut sample = [[0.0; 3]; 3];
        for _ in 0..clusters.len() {
            let c = &clusters[rng.below(clusters.len())];
            for i in 0..3 {
                for j in 0..3 {
                    sample[i][j] += c[i][j];
                }
            }
        }
        values.extend(crate::eval::stats::kappa_from_counts(&sample));
    }
    values.sort_by(f64::total_cmp);
    let undefined = AGREEMENT_RESAMPLES - values.len();
    (
        kappa,
        percentile(&values, 0.025),
        percentile(&values, 0.975),
        undefined,
    )
}

/// Validates `annotation.jsonl` and its notes against the final qrels and
/// computes agreement on the predeclared overlap. Records must match qrels
/// one to one; with the pooled-judgement gate this covers every pooled pair.
fn check_annotation(
    bench: &Bench,
    input: AnnotationInput<'_>,
) -> Result<(Vec<GateFailure>, Option<Agreement>), EvalError> {
    let mut failures = Vec::new();
    let Some(notes) = parse_notes(input.notes) else {
        failures.push(fail(
            "annotation_notes_invalid",
            "one valid version-1 annotation-notes block with two independent humans".into(),
        ));
        return Ok((failures, None));
    };
    if input.records.len() > MAX_REVIEW_BYTES {
        return Err(EvalError::Invalid("annotation byte limit exceeded".into()));
    }
    let mut budget = input.budget;
    let records: Vec<AnnotationRecord> = parse_jsonl_within(
        Path::new("annotation.jsonl"),
        input.records,
        &mut budget,
        |r: &AnnotationRecord| 1 + r.grades.len(),
    )?;
    let (judgements, _) = bench.judgements()?;
    let qrels: BTreeMap<(&str, &str), (u8, Option<&str>)> = judgements
        .iter()
        .map(|j| {
            (
                (j.query_id.as_str(), j.section_id.as_str()),
                (j.grade, j.span_sha256.as_deref()),
            )
        })
        .collect();
    let ids: [&str; 2] = [&notes.annotators[0].id, &notes.annotators[1].id];
    let model_judged = notes.label_provenance.is_some();
    let mut seen = BTreeSet::new();
    // Per query: raw (first, second) annotator grades when both graded.
    let mut double: BTreeMap<&str, Vec<Option<(u8, u8)>>> = BTreeMap::new();
    for r in &records {
        let pair = format!("{} {}", r.query_id, r.section_id);
        if r.version != 1 || r.query_id.len() > MAX_ID_BYTES || r.section_id.len() > 4096 {
            failures.push(fail("annotation_record_invalid", pair));
            continue;
        }
        let key = (r.query_id.as_str(), r.section_id.as_str());
        if !seen.insert(key) {
            failures.push(fail("annotation_duplicate", pair));
            continue;
        }
        let Some(&(grade, span)) = qrels.get(&key) else {
            failures.push(fail("annotation_unknown_pair", pair));
            continue;
        };
        if r.adjudication.grade != grade || span != Some(r.span_sha256.as_str()) {
            failures.push(fail("annotation_qrel_mismatch", pair.clone()));
        }
        let mut annotators = BTreeSet::new();
        let grades_valid = !r.grades.is_empty()
            && r.grades.iter().all(|g| {
                g.grade <= 2
                    && ids.contains(&g.annotator.as_str())
                    && annotators.insert(g.annotator.as_str())
                    && bounded(&g.evidence, MAX_NOTE_BYTES)
            });
        if !grades_valid {
            failures.push(fail("annotation_raw_grade_invalid", pair.clone()));
        }
        if model_judged {
            // Both judges grade every pair; the final grade is the fixed
            // floor((a + b) / 2), never a free adjudication.
            let both = ids.map(|id| r.grades.iter().find(|g| g.annotator == id));
            let rule = match both {
                // Widened: out-of-range raw grades already fail above and
                // must refuse, not overflow.
                [Some(a), Some(b)] => Some((u16::from(a.grade) + u16::from(b.grade)) / 2),
                _ => None,
            };
            if r.draft.is_some() || rule != Some(u16::from(r.adjudication.grade)) {
                failures.push(fail("annotation_model_rule_invalid", pair.clone()));
            }
        }
        let draft_valid = match (&r.draft, &r.draft_review) {
            (None, None) => true,
            (Some(d), Some(review)) => {
                d.grade <= 2
                    && bounded(&d.model, MAX_ID_BYTES)
                    && bounded(&d.evidence, MAX_NOTE_BYTES)
                    && ids.contains(&review.reviewer.as_str())
                    && bounded(&review.evidence, MAX_NOTE_BYTES)
            }
            _ => false,
        };
        if !draft_valid {
            failures.push(fail("annotation_draft_unreviewed", pair.clone()));
        }
        let a = &r.adjudication.adjudicator;
        // Model-judged labels use the fixed, pre-declared floor rule; a human
        // adjudicator is required only for human labels.
        let adjudicator_kind = if model_judged {
            PartyKind::Model
        } else {
            PartyKind::Human
        };
        if a.kind != adjudicator_kind
            || !party_valid(a)
            || !bounded(&r.adjudication.evidence, MAX_NOTE_BYTES)
        {
            failures.push(fail("annotation_adjudication_invalid", pair));
        }
        let raw = |id: &str| r.grades.iter().find(|g| g.annotator == id).map(|g| g.grade);
        let both = raw(ids[0]).zip(raw(ids[1])).filter(|_| grades_valid);
        double.entry(r.query_id.as_str()).or_default().push(both);
    }
    for (q, s) in qrels.keys() {
        if !seen.contains(&(*q, *s)) {
            failures.push(fail("annotation_pair_missing", format!("{q} {s}")));
        }
    }
    let selected = overlap_selection(&bench.queries, notes.assignment_seed);
    let mut clusters = Vec::new();
    let mut prevalence = [0usize; 3];
    let mut overlap_pairs = 0;
    for q in &selected {
        let pairs = double.get(q).map(Vec::as_slice).unwrap_or_default();
        if pairs.is_empty() || pairs.iter().any(Option::is_none) {
            failures.push(fail("annotation_overlap_incomplete", (*q).to_string()));
            continue;
        }
        let mut counts = [[0.0; 3]; 3];
        for &(a, b) in pairs.iter().flatten() {
            counts[usize::from(a)][usize::from(b)] += 1.0;
            prevalence[usize::from(a)] += 1;
            prevalence[usize::from(b)] += 1;
            overlap_pairs += 1;
        }
        clusters.push(counts);
    }
    let (kappa, ci_lo, ci_hi, undefined_resamples) =
        cluster_agreement(&clusters, bench.config.seed);
    if clusters.len() < 2 {
        failures.push(fail(
            "agreement_overlap_too_small",
            clusters.len().to_string(),
        ));
    }
    match kappa {
        None => failures.push(fail(
            "agreement_undefined",
            "weighted kappa is undefined".into(),
        )),
        Some(k) if k < MIN_KAPPA => {
            failures.push(fail(
                "agreement_below_floor",
                format!("{k:.4} < {MIN_KAPPA}"),
            ));
        }
        Some(_) => {}
    }
    if undefined_resamples > 0 {
        failures.push(fail(
            "agreement_ci_undefined",
            format!("{undefined_resamples} of {AGREEMENT_RESAMPLES} resamples undefined"),
        ));
    }
    let answerable_queries = bench
        .queries
        .iter()
        .filter(|q| q.stratum != Stratum::OffTopic)
        .count();
    Ok((
        failures,
        Some(Agreement {
            policy: OVERLAP_POLICY.into(),
            answerable_queries,
            overlap_queries: clusters.len(),
            overlap_pairs,
            kappa,
            ci_lo,
            ci_hi,
            seed: bench.config.seed,
            resamples: AGREEMENT_RESAMPLES,
            undefined_resamples,
            prevalence,
            label_provenance: notes.label_provenance.clone(),
        }),
    ))
}

/// Minimum query counts per split (ADR "Benchmark Data").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Floors {
    pub min_answerable_test: usize,
    pub min_off_topic_test: usize,
    /// Minimum test queries per answerable stratum.
    pub min_per_stratum_test: usize,
}

impl Default for Floors {
    fn default() -> Self {
        Self {
            min_answerable_test: 150,
            min_off_topic_test: 30,
            min_per_stratum_test: 15,
        }
    }
}

/// A pinned corpus: its root must be a clean git checkout at `commit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusPin {
    pub name: String,
    pub commit: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateConfig {
    #[serde(default)]
    pub floors: Option<Floors>,
    #[serde(default)]
    pub pins: Vec<CorpusPin>,
    /// Expected pool file (relative to the config) for the reproducibility gate.
    #[serde(default)]
    pub pool: Option<String>,
}

/// Normalised token Jaccard similarity.
pub fn jaccard(a: &str, b: &str) -> f64 {
    jaccard_sets(&token_set(a), &token_set(b))
}

fn token_set(text: &str) -> BTreeSet<String> {
    tokenize(text).into_iter().collect()
}

fn jaccard_sets(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        return 1.0;
    }
    a.intersection(b).count() as f64 / union as f64
}

/// Every judged section exists in its corpus's generation, and every
/// `span_sha256` recorded with a judgement matches the generation.
pub fn check_qrels(bench: &Bench) -> Result<Vec<GateFailure>, EvalError> {
    // Reuse the canonical grade and duplicate-judgement validation.
    let (judgements, _) = bench.judgements()?;
    let indexes = bench.indexes()?;
    let corpus_of: BTreeMap<&str, &str> = bench
        .queries
        .iter()
        .map(|q| (q.id.as_str(), q.corpus.as_str()))
        .collect();
    let off_topic: BTreeSet<&str> = bench
        .queries
        .iter()
        .filter(|q| q.stratum == Stratum::OffTopic)
        .map(|q| q.id.as_str())
        .collect();
    // Borrowed (corpus, section id) lookup built once; the first section
    // with an id wins, matching the former linear `find`.
    let mut sections = BTreeMap::new();
    for (name, index) in indexes.iter() {
        for s in &index.generation.sections {
            sections.entry((name.as_str(), s.id.as_str())).or_insert(s);
        }
    }
    let mut failures = bench.freshness_failures()?;
    for j in judgements {
        // ADR: off-topic queries carry no relevance labels at all.
        if off_topic.contains(j.query_id.as_str()) {
            failures.push(fail(
                "off_topic_judged",
                format!("{} {}", j.query_id, j.section_id),
            ));
        }
        if j.span_sha256.is_none() {
            failures.push(fail(
                "span_hash_missing",
                format!("{} {}", j.query_id, j.section_id),
            ));
        }
        let Some(corpus) = corpus_of.get(j.query_id.as_str()) else {
            failures.push(fail("judged_query_missing", j.query_id));
            continue;
        };
        let section = sections.get(&(*corpus, j.section_id.as_str()));
        match (section, j.span_sha256) {
            (None, _) => failures.push(fail(
                "judged_section_missing",
                format!("{} {}", j.query_id, j.section_id),
            )),
            (Some(s), Some(hash)) if !valid_hex(&hash, 64) || s.sha256 != hash => {
                failures.push(fail(
                    "span_hash_mismatch",
                    format!("{} {}", j.query_id, j.section_id),
                ));
            }
            _ => {}
        }
    }
    Ok(failures)
}

/// No test query is a near duplicate of a dev query.
pub fn check_leakage(bench: &Bench) -> Result<Vec<GateFailure>, EvalError> {
    let (splits, _) = bench.splits()?;
    // Tokenize each query once rather than once per dev/test pair.
    let mut tokens: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for q in &bench.queries {
        // Last duplicate id wins, as the former `collect` did.
        tokens.insert(q.id.as_str(), token_set(&q.text));
    }
    let mut failures = check_split_assignment(&bench.queries, bench.config.seed, &splits);
    for t in &splits.test {
        for d in &splits.dev {
            let (Some(a), Some(b)) = (tokens.get(t.as_str()), tokens.get(d.as_str())) else {
                continue;
            };
            let j = jaccard_sets(a, b);
            if j >= LEAK_JACCARD {
                failures.push(fail("dev_test_leak", format!("{t} ~ {d} (jaccard {j:.3})")));
            }
        }
    }
    let overlap: Vec<&String> = splits
        .test
        .iter()
        .filter(|t| splits.dev.contains(t))
        .collect();
    for id in overlap {
        failures.push(fail("dev_test_overlap", id.clone()));
    }
    Ok(failures)
}

fn check_split_assignment(queries: &[Query], seed: u64, splits: &Splits) -> Vec<GateFailure> {
    let known: BTreeSet<&str> = queries.iter().map(|q| q.id.as_str()).collect();
    let mut seen = BTreeSet::new();
    let mut failures = Vec::new();
    for id in splits.dev.iter().chain(&splits.test) {
        if !known.contains(id.as_str()) {
            failures.push(fail("split_query_unknown", id.clone()));
        }
        if !seen.insert(id.as_str()) {
            failures.push(fail("split_query_duplicate", id.clone()));
        }
    }
    for id in known.difference(&seen) {
        failures.push(fail("split_query_missing", (*id).into()));
    }
    if *splits != split(queries, seed) {
        failures.push(fail(
            "split_not_reproducible",
            "seeded assignment differs".into(),
        ));
    }
    failures
}

/// Query-count floors on the test split.
pub fn check_floors(bench: &Bench, floors: &Floors) -> Result<Vec<GateFailure>, EvalError> {
    let (splits, _) = bench.splits()?;
    let test: BTreeSet<&str> = splits.test.iter().map(String::as_str).collect();
    let mut per_stratum: BTreeMap<Stratum, usize> = BTreeMap::new();
    for q in bench
        .queries
        .iter()
        .filter(|q| test.contains(q.id.as_str()))
    {
        *per_stratum.entry(q.stratum).or_default() += 1;
    }
    let off_topic = per_stratum.get(&Stratum::OffTopic).copied().unwrap_or(0);
    let answerable: usize = per_stratum
        .iter()
        .filter(|(s, _)| **s != Stratum::OffTopic)
        .map(|(_, n)| n)
        .sum();
    let mut failures = Vec::new();
    if answerable < floors.min_answerable_test {
        failures.push(fail(
            "answerable_floor",
            format!("{answerable} < {}", floors.min_answerable_test),
        ));
    }
    if off_topic < floors.min_off_topic_test {
        failures.push(fail(
            "off_topic_floor",
            format!("{off_topic} < {}", floors.min_off_topic_test),
        ));
    }
    for stratum in [Stratum::Lookup, Stratum::Conceptual, Stratum::MultiHop] {
        let n = per_stratum.get(&stratum).copied().unwrap_or(0);
        if n < floors.min_per_stratum_test {
            failures.push(fail(
                "stratum_floor",
                format!("{stratum:?} {n} < {}", floors.min_per_stratum_test),
            ));
        }
    }
    Ok(failures)
}

/// The pool regenerates byte for byte.
/// Query approval is checked before any ranking; without it nothing is ranked.
pub fn check_pool(
    bench: &Bench,
    expected: &Path,
    approval: Option<&QueryApproval>,
) -> Result<Vec<GateFailure>, EvalError> {
    let Some(approval) = approval else {
        return Ok(vec![fail(
            "query_review_missing",
            "--query-review and --query-review-sha256 are required".into(),
        )]);
    };
    if !approval.covers(&bench.queries) {
        return Ok(vec![fail(
            "query_review_mismatch",
            "approved queries differ from the final queries".into(),
        )]);
    }
    let indexes = bench.indexes()?;
    let (regenerated, _, _) = pool_captured(bench, &indexes, approval)?;
    let same = bench.pool_bytes(expected)? == regenerated;
    Ok(if same {
        Vec::new()
    } else {
        vec![fail(
            "pool_not_reproducible",
            expected.display().to_string(),
        )]
    })
}

/// Clean git checkout state of a directory: `(HEAD, dirty)`.
pub fn git_state(dir: &Path) -> Option<(String, bool)> {
    git_state_impl(dir, true)
}

pub(crate) fn code_git_state(dir: &Path) -> Option<(String, bool)> {
    git_state_impl(dir, false)
}

fn git_state_impl(dir: &Path, exclude_index: bool) -> Option<(String, bool)> {
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };
    let head = git(&["rev-parse", "HEAD"])?;
    // The corpus root's `.ontodoc/` holds generated indexes and is not part of
    // the pin. The pathspec is relative to `dir`, so a corpus rooted in a
    // checkout subdirectory (e.g. `book/src`) excludes its own store.
    let mut status_args = vec![
        "status",
        "--porcelain",
        "--untracked-files=all",
        "--ignore-submodules=none",
        "--",
        ":/",
    ];
    if exclude_index {
        status_args.push(":(exclude).ontodoc");
    }
    let status = git(&status_args)?;
    Some((head, !status.is_empty()))
}

/// Every pinned corpus root is a clean checkout at the pinned commit.
pub fn check_pins(bench: &Bench, pins: &[CorpusPin]) -> Vec<GateFailure> {
    if let Some(failures) = bench.pin_failures(pins) {
        return failures;
    }
    let mut failures = Vec::new();
    let mut seen = BTreeSet::new();
    let mut corpora = BTreeSet::new();
    for corpus in &bench.config.corpora {
        if !corpora.insert(corpus.name.as_str()) {
            failures.push(fail("corpus_name_duplicate", corpus.name.clone()));
        }
        if pins.iter().all(|p| p.name != corpus.name) {
            failures.push(fail("corpus_pin_missing", corpus.name.clone()));
        }
    }
    if corpora.is_empty() {
        failures.push(fail("corpora_missing", "no named corpora".into()));
    }
    for pin in pins {
        if !seen.insert(pin.name.as_str()) {
            failures.push(fail("corpus_pin_duplicate", pin.name.clone()));
        }
        if !valid_hex(&pin.commit, 40) {
            failures.push(fail("corpus_pin_invalid", pin.name.clone()));
            continue;
        }
        let Some(spec) = bench.config.corpora.iter().find(|c| c.name == pin.name) else {
            failures.push(fail("corpus_pin_unknown", pin.name.clone()));
            continue;
        };
        match git_state(&bench.dir.join(&spec.root)) {
            None => failures.push(fail("corpus_not_a_checkout", pin.name.clone())),
            Some((head, _)) if !head.eq_ignore_ascii_case(&pin.commit) => failures.push(fail(
                "corpus_wrong_commit",
                format!("{} at {head}, pinned {}", pin.name, pin.commit),
            )),
            Some((_, true)) => failures.push(fail("corpus_dirty", pin.name.clone())),
            Some(_) => {}
        }
    }
    failures
}

fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// judged@10 >= [`MIN_JUDGED`] for every system in a report.
pub fn check_judged(report: &Report) -> Vec<GateFailure> {
    report
        .systems
        .iter()
        .filter(|(_, s)| s.mean.judged10 < MIN_JUDGED)
        .map(|(name, s)| {
            fail(
                "judged_below_floor",
                format!("{name} judged@10 {:.3}", s.mean.judged10),
            )
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct GateReport {
    pub stage: Stage,
    pub passed: bool,
    pub failures: Vec<GateFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agreement: Option<Agreement>,
}

/// Runs every gate that applies at `stage`. `report` is required for `run`;
/// missing query approval (pools) or annotation (qrels) is a failure, never a
/// skipped gate.
pub fn check_benchmark(
    bench: &Bench,
    gates: &GateConfig,
    stage: Stage,
    report: Option<&Report>,
    provenance: &Provenance<'_>,
) -> Result<GateReport, EvalError> {
    let mut failures = Vec::new();
    let mut agreement = None;
    if let Some(floors) = &gates.floors {
        let minimum = Floors::default();
        if floors.min_answerable_test < minimum.min_answerable_test
            || floors.min_off_topic_test < minimum.min_off_topic_test
            || floors.min_per_stratum_test < minimum.min_per_stratum_test
        {
            failures.push(fail(
                "floors_below_minimum",
                "floors must be at least 150/30/15".into(),
            ));
        }
        failures.extend(check_floors(bench, floors)?);
    } else {
        failures.push(fail("floors_missing", "query floors are required".into()));
    }
    failures.extend(check_leakage(bench)?);
    failures.extend(check_pins(bench, &gates.pins));
    if let Some(pool_file) = gates.pool.as_ref().filter(|p| !p.trim().is_empty()) {
        failures.extend(check_pool(
            bench,
            &bench.dir.join(pool_file),
            provenance.approval,
        )?);
    } else {
        failures.push(fail("pool_missing", "expected pool is required".into()));
    }
    if stage >= Stage::Qrels && !failures.is_empty() {
        return Ok(GateReport {
            stage,
            passed: false,
            failures,
            agreement,
        });
    }
    if stage >= Stage::Qrels {
        failures.extend(check_qrels(bench)?);
        let (qrels, _) = bench.qrels()?;
        let pool_path = gates
            .pool
            .as_ref()
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| EvalError::Invalid("expected pool is required for qrels".into()))?;
        let pool_path = bench.dir.join(pool_path);
        let pool_bytes = bench.pool_bytes(&pool_path)?;
        let pool_text =
            std::str::from_utf8(&pool_bytes).map_err(|e| EvalError::Invalid(e.to_string()))?;
        #[derive(Deserialize)]
        struct Pair {
            query_id: String,
            section_id: String,
        }
        for (i, line) in pool_text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
        {
            let pair: Pair = serde_json::from_str(line).map_err(|e| EvalError::Parse {
                path: pool_path.clone(),
                line: i + 1,
                message: e.to_string(),
            })?;
            if !qrels
                .get(&pair.query_id)
                .is_some_and(|q| q.contains_key(&pair.section_id))
            {
                failures.push(fail(
                    "pooled_judgment_missing",
                    format!("{} {}", pair.query_id, pair.section_id),
                ));
            }
        }
        for q in bench
            .queries
            .iter()
            .filter(|q| q.stratum != Stratum::OffTopic)
        {
            let relevant = qrels
                .get(&q.id)
                .is_some_and(|r| r.values().any(|&grade| grade >= 1));
            if !relevant {
                failures.push(fail("answerable_unjudged", q.id.clone()));
            }
        }
        // Annotation is validated only against qrels that passed their own
        // gates, so each failure has one cause; the report fails either way.
        match provenance.annotation {
            _ if !failures.is_empty() => {}
            Some(input) => {
                let (found, computed) = check_annotation(bench, input)?;
                failures.extend(found);
                agreement = computed;
            }
            None => failures.push(fail(
                "annotation_missing",
                "--annotation and --annotation-notes are required".into(),
            )),
        }
    }
    if stage >= Stage::Run {
        match report {
            Some(r) => failures.extend(check_judged(r)),
            None => failures.push(fail("run_report_missing", "--run is required".into())),
        }
    }
    Ok(GateReport {
        stage,
        passed: failures.is_empty(),
        failures,
        agreement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subdirectory_corpus_root_excludes_only_its_own_store() {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
                .args(["-c", "commit.gpgsign=false"])
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("a.md"), "# A\n").unwrap();
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "fixture"]);
        // The corpus root `src/` owns `src/.ontodoc`; it is not a pin change.
        std::fs::create_dir(src.join(".ontodoc")).unwrap();
        std::fs::write(src.join(".ontodoc/CURRENT"), "x").unwrap();
        assert_eq!(git_state(&src).map(|s| s.1), Some(false));
        // Code identity still sees it, and any other untracked file is dirty.
        assert_eq!(code_git_state(&src).map(|s| s.1), Some(true));
        std::fs::write(dir.path().join("stray.md"), "x").unwrap();
        assert_eq!(git_state(&src).map(|s| s.1), Some(true));
    }

    #[test]
    fn jaccard_on_normalised_tokens() {
        // {install, with, cargo} vs {install, with, cargo, now}: 3/4.
        assert!((jaccard("Install with cargo", "install WITH cargo now") - 0.75).abs() < 1e-12);
        assert_eq!(jaccard("a b", "a b"), 1.0);
        assert_eq!(jaccard("a", "b"), 0.0);
    }

    #[test]
    fn cluster_kappa_matches_worked_example_and_is_seeded() {
        // O = [[3,0,0],[1,2,1],[0,1,2]]: observed 0.75, expected 3.25 (see
        // stats::quadratic_weighted_kappa_worked_example), kappa = 10/13.
        let o: Counts = [[3.0, 0.0, 0.0], [1.0, 2.0, 1.0], [0.0, 1.0, 2.0]];
        let k = crate::eval::stats::kappa_from_counts(&o).unwrap();
        assert!((k - 10.0 / 13.0).abs() < 1e-12);
        assert_eq!(crate::eval::stats::kappa_from_counts(&[[0.0; 3]; 3]), None);
        // All mass in one cell: expected disagreement 0, undefined.
        let mut same = [[0.0; 3]; 3];
        same[1][1] = 4.0;
        assert_eq!(crate::eval::stats::kappa_from_counts(&same), None);
        // Splitting O into two clusters keeps the pooled point estimate.
        let a: Counts = [[3.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let b: Counts = [[0.0, 0.0, 0.0], [0.0, 2.0, 1.0], [0.0, 1.0, 2.0]];
        let first = cluster_agreement(&[a, b], 7);
        assert!((first.0.unwrap() - 10.0 / 13.0).abs() < 1e-12);
        assert_eq!(first, cluster_agreement(&[a, b], 7));
        let (_, lo, hi, undefined) = first;
        // Cluster `a` alone: rows (3,1,0), columns (4,0,0), n = 4. Observed
        // weighted disagreement 1/4; expected r1*c0*w/n^2 = 1*4*1/16 = 1/4,
        // so kappa = 1 - 1 = 0, defined. No resample of {a, b} is undefined.
        assert_eq!(undefined, 0);
        assert!(lo.unwrap() <= hi.unwrap());
        // {same, O}: drawing `same` twice (probability 1/4) leaves all mass
        // in one cell, which is undefined; roughly 2500 of 10000 resamples.
        let (_, _, _, undefined) = cluster_agreement(&[same, o], 7);
        assert!((2000..3000).contains(&undefined), "{undefined}");
    }

    #[test]
    fn overlap_selection_is_stratified_ceil_thirty_percent() {
        let queries: Vec<Query> = (0..10)
            .map(|i| Query {
                id: format!("q{i}"),
                text: String::new(),
                stratum: [Stratum::Lookup, Stratum::OffTopic][i % 2],
                corpus: "c".into(),
                model_drafted: false,
            })
            .collect();
        // Five lookup queries: ceil(1.5) = 2; off-topic never overlaps.
        let selected = overlap_selection(&queries, 3);
        assert_eq!(selected.len(), 2);
        assert!(
            selected
                .iter()
                .all(|id| id.trim_start_matches('q').parse::<usize>().unwrap() % 2 == 0)
        );
        assert_eq!(selected, overlap_selection(&queries, 3));
    }

    #[test]
    fn notes_require_one_fenced_block() {
        let block = r#"{"version":1,"guideline_version":"g1","annotators":[{"id":"h1","kind":"human","evidence":"e1"},{"id":"h2","kind":"human","evidence":"e2"}],"assignment_seed":1,"overlap":"corpus-stratum-ceil30-v1","blinding_evidence":"b","adjudication_rule":"r"}"#;
        let notes = format!("# Notes\n\n```annotation-notes\n{block}\n```\n");
        assert!(parse_notes(notes.as_bytes()).is_some());
        assert!(parse_notes(format!("{notes}{notes}").as_bytes()).is_none());
        assert!(parse_notes(notes.replace("```\n", "").as_bytes()).is_none());
        let same = notes.replace(r#""id":"h2""#, r#""id":"h1""#);
        assert!(parse_notes(same.as_bytes()).is_none());
        let model = notes.replace(
            r#""id":"h2","kind":"human""#,
            r#""id":"h2","kind":"model","model":"m""#,
        );
        assert!(parse_notes(model.as_bytes()).is_none());
        let policy = notes.replace("ceil30-v1", "ceil20-v1");
        assert!(parse_notes(policy.as_bytes()).is_none());
    }

    #[test]
    fn hashes_require_full_ascii_hex() {
        assert!(valid_hex(&"a0".repeat(20), 40));
        assert!(valid_hex(&"AF".repeat(32), 64));
        assert!(!valid_hex("abc123", 40));
        assert!(!valid_hex(&"g".repeat(40), 40));
        assert!(!valid_hex(&"é".repeat(20), 40));
    }

    #[test]
    fn split_assignment_requires_exact_seeded_partition() {
        let queries: Vec<Query> = (0..6)
            .map(|i| Query {
                id: format!("q{i}"),
                text: format!("query {i}"),
                stratum: Stratum::Lookup,
                corpus: "corpus".into(),
                model_drafted: false,
            })
            .collect();
        let expected = split(&queries, 7);
        assert!(check_split_assignment(&queries, 7, &expected).is_empty());
        let mut tampered = expected.clone();
        tampered.dev.push(tampered.test[0].clone());
        tampered.test[1] = "unknown".into();
        let failures = check_split_assignment(&queries, 7, &tampered);
        for gate in [
            "split_query_unknown",
            "split_query_duplicate",
            "split_query_missing",
            "split_not_reproducible",
        ] {
            assert!(failures.iter().any(|f| f.gate == gate), "{gate}");
        }
        let mut tampered = expected;
        std::mem::swap(&mut tampered.dev[0], &mut tampered.test[0]);
        assert_eq!(
            check_split_assignment(&queries, 7, &tampered)[0].gate,
            "split_not_reproducible"
        );
    }
}
