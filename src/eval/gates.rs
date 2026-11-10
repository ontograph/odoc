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

use crate::search::tokenize::tokenize;

use super::run::{Bench, EvalError, Query, Report, Splits, Stratum, pool_captured, split};

pub const LEAK_JACCARD: f64 = 0.8;
pub const MIN_JUDGED: f64 = 0.95;

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
    let a: BTreeSet<String> = tokenize(a).into_iter().collect();
    let b: BTreeSet<String> = tokenize(b).into_iter().collect();
    let union = a.union(&b).count();
    if union == 0 {
        return 1.0;
    }
    a.intersection(&b).count() as f64 / union as f64
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
    let mut failures = bench.freshness_failures()?;
    for j in judgements {
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
        let section = indexes
            .get(*corpus)
            .and_then(|ix| ix.generation.sections.iter().find(|s| s.id == j.section_id));
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
    let text: BTreeMap<&str, &str> = bench
        .queries
        .iter()
        .map(|q| (q.id.as_str(), q.text.as_str()))
        .collect();
    let mut failures = check_split_assignment(&bench.queries, bench.config.seed, &splits);
    for t in &splits.test {
        for d in &splits.dev {
            let (Some(a), Some(b)) = (text.get(t.as_str()), text.get(d.as_str())) else {
                continue;
            };
            let j = jaccard(a, b);
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
pub fn check_pool(
    bench: &Bench,
    expected: &Path,
    _scratch: &Path,
) -> Result<Vec<GateFailure>, EvalError> {
    let indexes = bench.indexes()?;
    let (regenerated, _, _) = pool_captured(bench, &indexes)?;
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
    // `.ontodoc/` holds generated indexes and is not part of the pin.
    let mut status_args = vec![
        "status",
        "--porcelain",
        "--untracked-files=all",
        "--ignore-submodules=none",
        "--",
        ":/",
    ];
    if exclude_index {
        status_args.push(":(top,exclude).ontodoc");
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
}

/// Runs every gate that applies at `stage`. `report` is required for `run`.
pub fn check_benchmark(
    bench: &Bench,
    gates: &GateConfig,
    stage: Stage,
    report: Option<&Report>,
    scratch: &Path,
) -> Result<GateReport, EvalError> {
    let mut failures = Vec::new();
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
        failures.extend(check_pool(bench, &bench.dir.join(pool_file), scratch)?);
    } else {
        failures.push(fail("pool_missing", "expected pool is required".into()));
    }
    if stage >= Stage::Qrels && !failures.is_empty() {
        return Ok(GateReport {
            stage,
            passed: false,
            failures,
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_on_normalised_tokens() {
        // {install, with, cargo} vs {install, with, cargo, now}: 3/4.
        assert!((jaccard("Install with cargo", "install WITH cargo now") - 0.75).abs() < 1e-12);
        assert_eq!(jaccard("a b", "a b"), 1.0);
        assert_eq!(jaccard("a", "b"), 0.0);
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
