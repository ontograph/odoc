//! Dev sweep, power, miss attribution and the code freeze.
//!
//! `sweep` runs on the dev split only: every grid point of B2 (`alpha`) and
//! B3 (`beta`) against B1, the per-arm standard deviation of the per-query
//! nDCG@10 difference at the best grid point, and the required test size.
//! `freeze` writes `frozen-config.toml` with the chosen parameters, the power
//! inputs and the code identity (commit, `Cargo.lock` hash, binary hash).
//! `check_frozen` refuses a held-out run on a dirty tree or any mismatch.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::corpus::sha256_hex;
use crate::search::{Mode, Params, SearchIndex};

use super::metrics::{self, Qrels};
use super::run::{
    Bench, CUTOFF, EvalError, MIN_EFFECT, Query, SplitName, Stratum, System, rank_sections,
};
use super::stats::{required_n, sample_sd};

pub const POWER: f64 = 0.8;
/// First Holm step for two confirmatory arms.
pub const POWER_ALPHA: f64 = 0.025;
pub const MIN_TEST_ANSWERABLE: usize = 150;
pub const REQUIRED_RESAMPLES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GridPoint {
    pub system: String,
    pub weight: f64,
    pub ndcg10: f64,
    pub mean_diff_vs_b1: f64,
    pub sd_diff_vs_b1: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PowerInputs {
    pub alpha: f64,
    pub power: f64,
    pub min_effect: f64,
    pub sd_diff_h2: f64,
    pub sd_diff_h5: f64,
    pub n_h2: usize,
    pub n_h5: usize,
    pub required_answerable_test: usize,
}

pub(crate) fn validate_power(power: &PowerInputs) -> Result<(), EvalError> {
    if power.alpha != POWER_ALPHA
        || power.power != POWER
        || power.min_effect != MIN_EFFECT
        || [power.sd_diff_h2, power.sd_diff_h5]
            .iter()
            .any(|sd| !sd.is_finite() || *sd < 0.0)
    {
        return Err(EvalError::Invalid("invalid frozen power inputs".into()));
    }
    let n_h2 = required_n(power.sd_diff_h2, MIN_EFFECT, POWER_ALPHA, POWER);
    let n_h5 = required_n(power.sd_diff_h5, MIN_EFFECT, POWER_ALPHA, POWER);
    if n_h2 == usize::MAX
        || n_h5 == usize::MAX
        || power.n_h2 != n_h2
        || power.n_h5 != n_h5
        || power.required_answerable_test != n_h2.max(n_h5).max(MIN_TEST_ANSWERABLE)
    {
        return Err(EvalError::Invalid(
            "frozen power sample sizes differ from the recorded calculation".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SweepReport {
    pub dev_answerable: usize,
    pub b1_ndcg10: f64,
    pub grid: Vec<GridPoint>,
    pub best_alpha: f64,
    pub best_beta: f64,
    pub power: PowerInputs,
}

fn ndcg_per_query(
    indexes: &BTreeMap<String, SearchIndex>,
    queries: &[&Query],
    qrels: &BTreeMap<String, Qrels>,
    system: System,
) -> Result<Vec<f64>, EvalError> {
    let mut judged = 0.0;
    let scores = queries
        .iter()
        .map(|q| {
            let qr = qrels.get(&q.id).ok_or_else(|| {
                EvalError::Invalid(format!("missing qrels for dev query {}", q.id))
            })?;
            let index = indexes.get(&q.corpus).ok_or_else(|| {
                EvalError::Invalid(format!("missing index for corpus {}", q.corpus))
            })?;
            let ranking = rank_sections(index, system, &q.text);
            let ids: Vec<&str> = ranking.iter().map(|(s, _)| s.as_str()).collect();
            judged += metrics::judged_at(qr, &ids, CUTOFF);
            metrics::ndcg_at(qr, &ids, CUTOFF)
                .map_err(|error| EvalError::Invalid(format!("dev query {}: {error}", q.id)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if queries.is_empty() || judged / (queries.len() as f64) < 0.95 {
        return Err(EvalError::Invalid(format!(
            "{} dev judged@10 below 0.95",
            system.name()
        )));
    }
    Ok(scores)
}

/// Dev-only sweep. Refuses any other split.
pub fn sweep(bench: &Bench, split: SplitName) -> Result<SweepReport, EvalError> {
    if split != SplitName::Dev {
        return Err(EvalError::Invalid(
            "sweep runs on the dev split only".into(),
        ));
    }
    sweep_captured(bench)
}

pub(crate) fn sweep_captured(bench: &Bench) -> Result<SweepReport, EvalError> {
    bench.require_prepared_capture(SplitName::Dev)?;
    for grid in [&bench.config.alpha_grid, &bench.config.beta_grid] {
        if grid.is_empty()
            || grid
                .iter()
                .any(|weight| !weight.is_finite() || !(0.0..=1.0).contains(weight))
        {
            return Err(EvalError::Invalid("invalid or empty dev sweep grid".into()));
        }
    }
    let indexes = bench.indexes()?;
    let (qrels, _) = bench.qrels()?;
    let queries: Vec<&Query> = bench
        .queries
        .iter()
        .filter(|q| q.stratum != Stratum::OffTopic)
        .collect();
    for q in &queries {
        if !qrels
            .get(&q.id)
            .is_some_and(|r| r.values().any(|&g| g >= 1))
        {
            return Err(EvalError::Invalid(format!(
                "missing positive judgement for dev query {}",
                q.id
            )));
        }
    }
    if queries.len() < 2 {
        return Err(EvalError::Invalid(
            "power estimation requires at least two answerable dev queries".into(),
        ));
    }
    let b1 = ndcg_per_query(&indexes, &queries, &qrels, System::B1)?;
    let mean = |v: &[f64]| {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let mut grid = Vec::new();
    let points = bench
        .config
        .alpha_grid
        .iter()
        .map(|&a| ("B2", a, System::B2(a)))
        .chain(
            bench
                .config
                .beta_grid
                .iter()
                .map(|&b| ("B3", b, System::B3(b))),
        );
    for (name, weight, system) in points {
        let scores = ndcg_per_query(&indexes, &queries, &qrels, system)?;
        let diffs: Vec<f64> = scores.iter().zip(&b1).map(|(s, b)| s - b).collect();
        grid.push(GridPoint {
            system: name.into(),
            weight,
            ndcg10: mean(&scores),
            mean_diff_vs_b1: mean(&diffs),
            sd_diff_vs_b1: sample_sd(&diffs),
        });
    }
    // Best grid point per arm: highest mean nDCG@10, ties to the smaller weight.
    let best = |arm: &str| {
        grid.iter()
            .filter(|g| g.system == arm)
            .fold(None::<&GridPoint>, |acc, g| match acc {
                Some(a)
                    if a.ndcg10 > g.ndcg10 || (a.ndcg10 == g.ndcg10 && a.weight <= g.weight) =>
                {
                    Some(a)
                }
                _ => Some(g),
            })
    };
    let (b2, b3) = (best("B2"), best("B3"));
    let sd_h2 = b2.map_or(0.0, |g| g.sd_diff_vs_b1);
    let sd_h5 = b3.map_or(0.0, |g| g.sd_diff_vs_b1);
    let n_h2 = required_n(sd_h2, MIN_EFFECT, POWER_ALPHA, POWER);
    let n_h5 = required_n(sd_h5, MIN_EFFECT, POWER_ALPHA, POWER);
    Ok(SweepReport {
        dev_answerable: queries.len(),
        b1_ndcg10: mean(&b1),
        best_alpha: b2.map_or(0.0, |g| g.weight),
        best_beta: b3.map_or(0.0, |g| g.weight),
        grid,
        power: PowerInputs {
            alpha: POWER_ALPHA,
            power: POWER,
            min_effect: MIN_EFFECT,
            sd_diff_h2: sd_h2,
            sd_diff_h5: sd_h5,
            n_h2,
            n_h5,
            required_answerable_test: n_h2.max(n_h5).max(MIN_TEST_ANSWERABLE),
        },
    })
}

/// Pipeline stage responsible for one judged-relevant section missing from
/// a system's top 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissStage {
    /// The section is not in the generation.
    NotIndexed,
    /// The section's text matches query terms, but none of its chunks does:
    /// the terms fall outside every chunk span (heading-only or split
    /// across a boundary). Not reported for zero vocabulary overlap.
    ChunkBoundary,
    /// BM25 ranks none of its chunks within the fetch depth, including when
    /// the section shares no query term at all (vocabulary mismatch).
    LexicalMiss,
    /// BM25 alone has it in the top 10, but fusion pushed it out.
    FusionDisplaced,
    /// BM25 finds it, just below the cutoff.
    Truncated,
}

/// Attributes a missed relevant section to exactly one stage.
pub fn attribute_miss(
    index: &SearchIndex,
    query: &str,
    section_id: &str,
    system: System,
) -> MissStage {
    let g = &index.generation;
    if !g.sections.iter().any(|s| s.id == section_id) {
        return MissStage::NotIndexed;
    }
    let terms = crate::search::tokenize::query_terms(query);
    let scores = index.bm25().scores(&terms);
    let own: Vec<usize> = g
        .chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| c.section_id == section_id)
        .map(|(i, _)| i)
        .collect();
    if own.iter().all(|&i| scores[i] <= 0.0) {
        // Zero chunk score is a boundary miss only when the section's own
        // text (spanning all its chunks) still contains a query term.
        let section_text = g
            .sections
            .iter()
            .find(|s| s.id == section_id)
            .and_then(|s| {
                g.documents
                    .iter()
                    .find(|d| d.path == s.path)?
                    .text
                    .get(s.span.byte_start..s.span.byte_end)
            })
            .unwrap_or("");
        let section_terms = crate::search::tokenize::tokenize(section_text);
        return if terms.iter().any(|t| section_terms.contains(t)) {
            MissStage::ChunkBoundary
        } else {
            MissStage::LexicalMiss
        };
    }
    let b1 = rank_sections(index, System::B1, query);
    let in_b1 = b1.iter().any(|(s, _)| s == section_id);
    if in_b1 && matches!(system, System::B2(_) | System::B3(_)) {
        return MissStage::FusionDisplaced;
    }
    let deep = index.search(query, &Params::new(Mode::Lexical, 50));
    let found = deep
        .hits
        .iter()
        .any(|h| g.chunks[h.chunk].section_id == section_id);
    if found {
        MissStage::Truncated
    } else {
        MissStage::LexicalMiss
    }
}

/// Version 2 binds the preparation manifest and both partitions; older
/// frozen files are refused, never migrated.
pub const FROZEN_VERSION: u32 = 2;

/// Frozen evaluation configuration (TOML).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrozenConfig {
    pub version: u32,
    /// SHA-256 of the admitted preparation manifest.
    pub preparation_sha256: String,
    pub alpha: f64,
    pub beta: f64,
    pub threshold: f64,
    pub seed: u64,
    pub resamples: usize,
    pub config_sha256: String,
    /// Exact partition payload digests from the manifest.
    pub dev_queries_sha256: String,
    pub dev_qrels_sha256: String,
    pub test_queries_sha256: String,
    pub test_qrels_sha256: String,
    pub splits_sha256: String,
    pub git_commit: String,
    pub cargo_lock_sha256: String,
    pub binary_sha256: String,
    pub generation_ids: BTreeMap<String, String>,
    pub power: PowerInputs,
}

/// A quoted string valid as both a TOML basic string and JSON: `"`, `\` and
/// control characters (including DEL) are escaped; everything else is
/// written verbatim, so ordinary names keep their historical bytes.
fn toml_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl FrozenConfig {
    /// Deterministic TOML: scalars, power inputs and sorted corpus generations.
    pub fn to_toml(&self) -> String {
        let p = &self.power;
        let mut text = format!(
            "# Frozen evaluation configuration. Do not edit after test-split access.\n\
             version = {}\npreparation_sha256 = \"{}\"\n\
             alpha = {:?}\nbeta = {:?}\nthreshold = {:?}\nseed = {}\nresamples = {}\n\
             config_sha256 = \"{}\"\ndev_queries_sha256 = \"{}\"\ndev_qrels_sha256 = \"{}\"\n\
             test_queries_sha256 = \"{}\"\ntest_qrels_sha256 = \"{}\"\n\
             splits_sha256 = \"{}\"\ngit_commit = \"{}\"\ncargo_lock_sha256 = \"{}\"\n\
             binary_sha256 = \"{}\"\n\n[power]\nalpha = {:?}\npower = {:?}\nmin_effect = {:?}\n\
             sd_diff_h2 = {:?}\nsd_diff_h5 = {:?}\nn_h2 = {}\nn_h5 = {}\n\
             required_answerable_test = {}\n",
            self.version,
            self.preparation_sha256,
            self.alpha,
            self.beta,
            self.threshold,
            self.seed,
            self.resamples,
            self.config_sha256,
            self.dev_queries_sha256,
            self.dev_qrels_sha256,
            self.test_queries_sha256,
            self.test_qrels_sha256,
            self.splits_sha256,
            self.git_commit,
            self.cargo_lock_sha256,
            self.binary_sha256,
            p.alpha,
            p.power,
            p.min_effect,
            p.sd_diff_h2,
            p.sd_diff_h5,
            p.n_h2,
            p.n_h5,
            p.required_answerable_test,
        );
        text.push_str("\n[generation_ids]\n");
        for (corpus, generation) in &self.generation_ids {
            text.push_str(&format!(
                "{} = {}\n",
                toml_string(corpus),
                toml_string(generation)
            ));
        }
        text
    }

    /// Parses the exact shape written by [`FrozenConfig::to_toml`].
    pub fn from_toml(text: &str) -> Result<Self, EvalError> {
        let mut top = serde_json::Map::new();
        let mut power = serde_json::Map::new();
        let mut generations = serde_json::Map::new();
        let mut saw_generations = false;
        let mut section = "";
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[power]" || line == "[generation_ids]" {
                section = line;
                saw_generations |= line == "[generation_ids]";
                continue;
            }
            let bad = || EvalError::Parse {
                path: PathBuf::from("frozen-config.toml"),
                line: i + 1,
                message: format!("unsupported line {raw:?}"),
            };
            let table = match section {
                "[power]" => &mut power,
                "[generation_ids]" => &mut generations,
                _ => &mut top,
            };
            // Corpus keys are quoted strings that may themselves contain " = ".
            let (key, value) = if section == "[generation_ids]" {
                let mut keys = serde_json::Deserializer::from_str(line).into_iter::<String>();
                let key = keys.next().ok_or_else(bad)?.map_err(|_| bad())?;
                let rest = line.get(keys.byte_offset()..).ok_or_else(bad)?;
                (key, rest.strip_prefix(" = ").ok_or_else(bad)?)
            } else {
                let (key, value) = line.split_once(" = ").ok_or_else(bad)?;
                (key.to_string(), value)
            };
            let value: serde_json::Value = serde_json::from_str(value).map_err(|_| bad())?;
            if table.insert(key, value).is_some() {
                return Err(bad());
            }
        }
        top.insert("power".into(), serde_json::Value::Object(power));
        if !saw_generations {
            return Err(EvalError::Invalid(
                "missing frozen corpus generations".into(),
            ));
        }
        top.insert(
            "generation_ids".into(),
            serde_json::Value::Object(generations),
        );
        let frozen: Self = serde_json::from_value(serde_json::Value::Object(top))?;
        if frozen.version != FROZEN_VERSION {
            return Err(EvalError::Invalid(
                "unsupported frozen config version".into(),
            ));
        }
        Ok(frozen)
    }

    /// Mismatches between this freeze and the bench's manifest-admitted
    /// capture: manifest hash, both partitions and the selected split's
    /// captured digests. Empty when bound.
    pub(crate) fn preparation_problems(&self, bench: &Bench) -> Result<Vec<String>, EvalError> {
        let Some((split, sha, partitions)) = bench.preparation_binding() else {
            return Ok(vec![
                "benchmark capture lacks an admitted preparation".into(),
            ]);
        };
        let mut problems = Vec::new();
        if self.version != FROZEN_VERSION {
            problems.push("frozen config version unsupported".into());
        }
        if self.preparation_sha256 != sha {
            problems.push("preparation manifest differs".into());
        }
        let frozen = [
            (&self.dev_queries_sha256, &self.dev_qrels_sha256),
            (&self.test_queries_sha256, &self.test_qrels_sha256),
        ];
        for (name, (want, have)) in ["dev", "test"].iter().zip(frozen.iter().zip(partitions)) {
            if (want.0, want.1) != (&have.0, &have.1) {
                problems.push(format!("{name} partition digests differ"));
            }
        }
        let k = usize::from(split == SplitName::Test);
        if (&bench.queries_sha256, &bench.qrels()?.1) != (&partitions[k].0, &partitions[k].1) {
            problems.push("captured partition differs from the manifest".into());
        }
        Ok(problems)
    }
}

/// Code identity checked by the freeze.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeIdentity {
    pub git_commit: String,
    pub dirty: bool,
    pub cargo_lock_sha256: String,
    pub binary_sha256: String,
}

impl CodeIdentity {
    /// Identity of the repository at `repo` and the running binary.
    pub fn current(repo: &Path) -> Result<Self, EvalError> {
        let (git_commit, dirty) = super::gates::code_git_state(repo).ok_or_else(|| {
            EvalError::Invalid(format!("{} is not a git checkout", repo.display()))
        })?;
        let lock = repo.join("Cargo.lock");
        let lock_bytes = fs::read(&lock).map_err(|source| EvalError::Io { path: lock, source })?;
        let exe = std::env::current_exe().map_err(|source| EvalError::Io {
            path: PathBuf::from("current_exe"),
            source,
        })?;
        let bin = fs::read(&exe).map_err(|source| EvalError::Io { path: exe, source })?;
        Ok(Self {
            git_commit,
            dirty,
            cargo_lock_sha256: sha256_hex(&lock_bytes),
            binary_sha256: sha256_hex(&bin),
        })
    }
}

/// Builds the frozen config from a dev sweep and the code identity.
/// Runs the dev sweep once and freezes its selection.
pub fn freeze(
    bench: &Bench,
    threshold: f64,
    code: &CodeIdentity,
) -> Result<FrozenConfig, EvalError> {
    bench.require_prepared_capture(SplitName::Dev)?;
    if bench.config.resamples != REQUIRED_RESAMPLES {
        return Err(EvalError::Invalid(
            "freezing requires exactly 10000 resamples".into(),
        ));
    }
    if code.dirty {
        return Err(EvalError::Invalid("refusing to freeze a dirty tree".into()));
    }
    let sweep = &self::sweep(bench, SplitName::Dev)?;
    validate_power(&sweep.power)?;
    if sweep.dev_answerable < 2 {
        return Err(EvalError::Invalid(
            "freezing power requires at least two answerable dev queries".into(),
        ));
    }
    if (
        bench.config.alpha,
        bench.config.beta,
        bench.config.threshold,
    ) != (sweep.best_alpha, sweep.best_beta, threshold)
        || ![sweep.best_alpha, sweep.best_beta]
            .iter()
            .all(|weight| weight.is_finite() && (0.0..=1.0).contains(weight))
        || !threshold.is_finite()
        || threshold < 0.0
    {
        return Err(EvalError::Invalid(
            "benchmark config must contain the selected alpha, beta and threshold before freezing"
                .into(),
        ));
    }
    let generation_ids = generation_ids(bench)?;
    let (_, splits_sha256) = bench.splits()?;
    let (_, preparation_sha256, [dev, test]) = bench
        .preparation_binding()
        .ok_or_else(|| EvalError::Invalid("freezing requires an admitted preparation".into()))?;
    Ok(FrozenConfig {
        version: FROZEN_VERSION,
        preparation_sha256: preparation_sha256.into(),
        dev_queries_sha256: dev.0.clone(),
        dev_qrels_sha256: dev.1.clone(),
        test_queries_sha256: test.0.clone(),
        test_qrels_sha256: test.1.clone(),
        alpha: sweep.best_alpha,
        beta: sweep.best_beta,
        threshold,
        seed: bench.config.seed,
        resamples: bench.config.resamples,
        config_sha256: bench.config_sha256.clone(),
        splits_sha256,
        git_commit: code.git_commit.clone(),
        cargo_lock_sha256: code.cargo_lock_sha256.clone(),
        binary_sha256: code.binary_sha256.clone(),
        generation_ids,
        power: sweep.power.clone(),
    })
}

/// Refuses a held-out run unless code and benchmark match the freeze. The
/// benchmark config must carry the frozen alpha, beta and threshold.
pub fn check_frozen(
    frozen: &FrozenConfig,
    bench: &Bench,
    code: &CodeIdentity,
) -> Result<(), EvalError> {
    let answerable = bench
        .prepared_test_answerable()
        .ok_or_else(super::run::admission_error)?;
    if frozen.resamples != REQUIRED_RESAMPLES || bench.config.resamples != REQUIRED_RESAMPLES {
        return Err(EvalError::Invalid(
            "frozen config requires exactly 10000 resamples".into(),
        ));
    }
    validate_power(&frozen.power)?;
    let mut problems = Vec::new();
    if code.dirty {
        problems.push("working tree is dirty".to_string());
    }
    let mut same = |what: &str, a: &str, b: &str| {
        if a != b {
            problems.push(format!("{what} differs (frozen {a}, now {b})"));
        }
    };
    same("git commit", &frozen.git_commit, &code.git_commit);
    same(
        "Cargo.lock",
        &frozen.cargo_lock_sha256,
        &code.cargo_lock_sha256,
    );
    same("binary", &frozen.binary_sha256, &code.binary_sha256);
    same(
        "benchmark config",
        &frozen.config_sha256,
        &bench.config_sha256,
    );
    let (_, splits) = bench.splits()?;
    same("splits", &frozen.splits_sha256, &splits);
    problems.extend(frozen.preparation_problems(bench)?);
    if generation_ids(bench)? != frozen.generation_ids {
        problems.push("corpus generations differ from the frozen config".into());
    }
    if answerable < frozen.power.required_answerable_test {
        problems.push(format!(
            "answerable test queries {answerable} below required {}",
            frozen.power.required_answerable_test
        ));
    }
    let c = &bench.config;
    if (c.alpha, c.beta, c.threshold, c.seed, c.resamples)
        != (
            frozen.alpha,
            frozen.beta,
            frozen.threshold,
            frozen.seed,
            frozen.resamples,
        )
    {
        problems.push("run parameters differ from the frozen config".into());
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(EvalError::Invalid(format!(
            "frozen config refused: {}",
            problems.join("; ")
        )))
    }
}

fn generation_ids(bench: &Bench) -> Result<BTreeMap<String, String>, EvalError> {
    Ok(bench
        .indexes()?
        .iter()
        .map(|(corpus, index)| (corpus.clone(), index.generation.generation_id.clone()))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen() -> FrozenConfig {
        let n_h2 = required_n(0.2, MIN_EFFECT, POWER_ALPHA, POWER);
        let n_h5 = required_n(0.15, MIN_EFFECT, POWER_ALPHA, POWER);
        FrozenConfig {
            version: FROZEN_VERSION,
            preparation_sha256: "p".into(),
            alpha: 0.2,
            beta: 0.2,
            threshold: 0.3,
            seed: 1,
            resamples: 10,
            config_sha256: "c".into(),
            dev_queries_sha256: "dq".into(),
            dev_qrels_sha256: "dr".into(),
            test_queries_sha256: "tq".into(),
            test_qrels_sha256: "tr".into(),
            splits_sha256: "s".into(),
            git_commit: "abc".into(),
            cargo_lock_sha256: "lock".into(),
            binary_sha256: "bin".into(),
            generation_ids: BTreeMap::from([("corpus".into(), "generation".into())]),
            power: PowerInputs {
                alpha: POWER_ALPHA,
                power: POWER,
                min_effect: MIN_EFFECT,
                sd_diff_h2: 0.2,
                sd_diff_h5: 0.15,
                n_h2,
                n_h5,
                required_answerable_test: n_h2.max(n_h5).max(MIN_TEST_ANSWERABLE),
            },
        }
    }

    #[test]
    fn frozen_config_roundtrips_through_toml() {
        let f = frozen();
        let text = f.to_toml();
        assert!(text.contains("alpha = 0.2\n"));
        assert!(text.contains("[power]\n"));
        assert_eq!(FrozenConfig::from_toml(&text).unwrap(), f);
        assert!(FrozenConfig::from_toml("alpha: 1").is_err());
        assert!(FrozenConfig::from_toml(text.split("[generation_ids]").next().unwrap()).is_err());
        // Version-1 files (no version, combined digests) and unknown
        // versions are refused rather than migrated.
        let v1 = text
            .replace("version = 2\n", "")
            .replace("preparation_sha256 = \"p\"\n", "")
            .replace("dev_queries_sha256 = \"dq\"\n", "queries_sha256 = \"q\"\n");
        assert!(FrozenConfig::from_toml(&v1).is_err());
        let v3 = text.replace("version = 2\n", "version = 3\n");
        assert!(FrozenConfig::from_toml(&v3).is_err());
        let extra = text.replace("version = 2\n", "version = 2\nqueries_sha256 = \"q\"\n");
        assert!(FrozenConfig::from_toml(&extra).is_err());
    }

    #[test]
    fn frozen_config_roundtrips_unusual_corpus_names() {
        // Old codec wrote Rust debug escapes (`\u{1}`, `\u{200b}`), which the
        // JSON reader refused; a key containing " = " split at the wrong place.
        let names = [
            "a\u{1}b",
            "q\"uote",
            "back\\slash",
            "del\u{7f}",
            "zero\u{200b}width",
            "кириллица",
            "k = v",
            "tab\there\nnl",
            "c1\u{85}",
        ];
        let mut f = frozen();
        f.generation_ids = names
            .iter()
            .map(|n| ((*n).to_string(), format!("gen {n}")))
            .collect();
        let text = f.to_toml();
        assert_eq!(FrozenConfig::from_toml(&text).unwrap(), f);
        // Ordinary names keep their historical bytes.
        assert!(
            frozen()
                .to_toml()
                .ends_with("\"corpus\" = \"generation\"\n")
        );
        assert_eq!(toml_string("del\u{7f}"), "\"del\\u007F\"");
        // A key followed by anything but " = " is refused.
        let broken = text.replace("\"k = v\" = ", "\"k = v\" =");
        assert!(FrozenConfig::from_toml(&broken).is_err());
    }

    #[test]
    fn zero_overlap_is_a_lexical_miss_and_child_only_terms_are_a_chunk_boundary() {
        let dir = tempfile::tempdir().unwrap();
        // `# Parent` owns only its intro block; its span also covers `## Child`.
        std::fs::write(
            dir.path().join("a.md"),
            "# Parent\n\nintro words\n\n## Child\n\nzebra stripes\n",
        )
        .unwrap();
        let generation =
            crate::store::build(dir.path(), &crate::store::IndexConfig::default()).unwrap();
        let index = SearchIndex::new(generation);
        // No query term anywhere in the section: vocabulary mismatch.
        assert_eq!(
            attribute_miss(&index, "unrelated", "a.md#parent", System::B1),
            MissStage::LexicalMiss
        );
        // The term is in the parent's span, but only in the child's chunk.
        assert_eq!(
            attribute_miss(&index, "zebra", "a.md#parent", System::B1),
            MissStage::ChunkBoundary
        );
        assert_eq!(
            attribute_miss(&index, "zebra", "missing.md#x", System::B1),
            MissStage::NotIndexed
        );
    }

    #[test]
    fn power_rejects_invalid_inputs_and_inconsistent_sample_sizes() {
        let valid = frozen().power;
        assert!(validate_power(&valid).is_ok());
        for sd in [f64::NAN, f64::INFINITY, -0.1, f64::MAX] {
            let mut invalid = valid.clone();
            invalid.sd_diff_h2 = sd;
            assert!(validate_power(&invalid).is_err());
        }
        let mut invalid = valid.clone();
        invalid.alpha = 0.05;
        assert!(validate_power(&invalid).is_err());
        let mut invalid = valid.clone();
        invalid.n_h2 += 1;
        assert!(validate_power(&invalid).is_err());
        let mut invalid = valid;
        invalid.required_answerable_test = MIN_TEST_ANSWERABLE - 1;
        assert!(validate_power(&invalid).is_err());
    }

    #[test]
    fn dev_metrics_refuse_missing_qrels_instead_of_dropping_queries() {
        let query = Query {
            id: "dev".into(),
            text: "query".into(),
            corpus: "corpus".into(),
            stratum: Stratum::Lookup,
            model_drafted: false,
        };
        assert!(ndcg_per_query(&BTreeMap::new(), &[&query], &BTreeMap::new(), System::B1).is_err());
    }
}
