use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use ontocode_extension_api::ContextualUserFragment;
use serde::Deserialize;
use serde::Serialize;

use crate::km_context::render_km_context;
use crate::km_resolve::KmResolveError;
use crate::km_resolve::resolve_active_km;
use crate::km_store::KmStore;

const EVAL_SUITE_DIR: &str = "_eval/suite";
const EVAL_RESULTS: &str = "_eval/km_accuracy.jsonl";
const EVAL_REPORT: &str = "_eval/report.json";
const USAGE_LOG: &str = "_logs/km_usage.jsonl";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KmEvalResult {
    pub id: String,
    pub total_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub pass_rate: f64,
    pub report_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmEvalError {
    MissingKmRoot,
    MissingCases,
    ReadCases,
    InvalidCase(String),
    ResolveKm,
    MissingActiveKm,
    WriteResult,
    WriteReport,
    WriteUsage,
}

pub fn run_km_eval(
    id: &str,
    store_root: &Path,
    cases_dir: Option<&Path>,
) -> Result<KmEvalResult, KmEvalError> {
    let Some(km_root) = store_root.parent() else {
        return Err(KmEvalError::MissingKmRoot);
    };
    let cases_dir = cases_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(|| km_root.join(EVAL_SUITE_DIR));
    let cases = read_cases(&cases_dir)?;
    let store = KmStore::new(store_root);
    let resolve_started = Instant::now();
    let resolved = resolve_active_km(&store, id)
        .map_err(|_| KmEvalError::ResolveKm)?
        .ok_or(KmEvalError::MissingActiveKm)?;
    let resolve_ms = resolve_started.elapsed().as_millis();

    let mut passed_cases = 0;
    for case in &cases {
        let render_started = Instant::now();
        let fragment = render_km_context(&resolved, &case.prompt);
        let context = fragment.render();
        let render_ms = render_started.elapsed().as_millis();
        let matches_required = case
            .required_substrings
            .iter()
            .all(|required| context.contains(required));
        let passed = if case.expect_present {
            matches_required
        } else {
            !matches_required
        };
        if passed {
            passed_cases += 1;
        }
        append_eval_result(
            &km_root.join(EVAL_RESULTS),
            &CaseResult {
                case_id: case.id.clone(),
                id: resolved.id.clone(),
                version: resolved.version.clone(),
                passed,
                context_bytes: context.len(),
                render_duration_ms: render_ms,
                evaluated_at_unix_seconds: now_unix_seconds(),
            },
        )?;
        append_usage(
            &km_root.join(USAGE_LOG),
            &UsageRecord {
                id: resolved.id.clone(),
                version: resolved.version.clone(),
                resolved_active: true,
                resolve_duration_ms: resolve_ms,
                render_duration_ms: render_ms,
                rendered_fact_count: resolved.facts.len(),
                rendered_example_count: resolved.examples.len(),
                context_bytes: context.len(),
                truncated: context.contains("Truncation:"),
                recorded_at_unix_seconds: now_unix_seconds(),
            },
        )?;
    }

    let total_cases = cases.len();
    let failed_cases = total_cases - passed_cases;
    let result = KmEvalResult {
        id: resolved.id,
        total_cases,
        passed_cases,
        failed_cases,
        pass_rate: if total_cases == 0 {
            0.0
        } else {
            passed_cases as f64 / total_cases as f64
        },
        report_path: km_root.join(EVAL_REPORT).display().to_string(),
    };
    write_report(&result)?;
    Ok(result)
}

/// A candidate passes only when the suite ran at least one case and every case
/// passed. An empty suite fails closed rather than counting as a pass.
#[cfg(test)]
pub(crate) fn candidate_eval_passed(result: &KmEvalResult) -> bool {
    if result.total_cases == 0 {
        return false;
    }
    result.failed_cases == 0
}

impl std::fmt::Display for KmEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKmRoot => write!(f, "KM store root must have a parent directory"),
            Self::MissingCases => write!(f, "KM eval cases directory is missing"),
            Self::ReadCases => write!(f, "failed to read KM eval cases"),
            Self::InvalidCase(error) => write!(f, "invalid KM eval case: {error}"),
            Self::ResolveKm => write!(f, "failed to resolve active KM"),
            Self::MissingActiveKm => write!(f, "active KM was not found"),
            Self::WriteResult => write!(f, "failed to write KM eval result"),
            Self::WriteReport => write!(f, "failed to write KM eval report"),
            Self::WriteUsage => write!(f, "failed to write KM usage telemetry"),
        }
    }
}

impl std::error::Error for KmEvalError {}

impl From<KmResolveError> for KmEvalError {
    fn from(_: KmResolveError) -> Self {
        Self::ResolveKm
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvalCase {
    id: String,
    prompt: String,
    required_substrings: Vec<String>,
    #[serde(default = "default_expect_present")]
    expect_present: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseResult {
    case_id: String,
    id: String,
    version: String,
    passed: bool,
    context_bytes: usize,
    render_duration_ms: u128,
    evaluated_at_unix_seconds: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageRecord {
    id: String,
    version: String,
    resolved_active: bool,
    resolve_duration_ms: u128,
    render_duration_ms: u128,
    rendered_fact_count: usize,
    rendered_example_count: usize,
    context_bytes: usize,
    truncated: bool,
    recorded_at_unix_seconds: i64,
}

fn default_expect_present() -> bool {
    true
}

fn read_cases(cases_dir: &Path) -> Result<Vec<EvalCase>, KmEvalError> {
    let entries = fs::read_dir(cases_dir).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            KmEvalError::MissingCases
        } else {
            KmEvalError::ReadCases
        }
    })?;
    let mut paths = entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| KmEvalError::ReadCases)?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut cases = Vec::new();
    for path in paths {
        let raw = fs::read_to_string(&path).map_err(|_| KmEvalError::ReadCases)?;
        let case = serde_json::from_str::<EvalCase>(&raw)
            .map_err(|error| KmEvalError::InvalidCase(error.to_string()))?;
        if case.id.trim().is_empty() || case.required_substrings.is_empty() {
            return Err(KmEvalError::InvalidCase(
                "case id and requiredSubstrings are required".to_string(),
            ));
        }
        cases.push(case);
    }
    Ok(cases)
}

fn append_eval_result(path: &Path, result: &CaseResult) -> Result<(), KmEvalError> {
    let line = serde_json::to_string(result).map_err(|_| KmEvalError::WriteResult)?;
    append_line(path, &line).map_err(|_| KmEvalError::WriteResult)
}

fn append_usage(path: &Path, result: &UsageRecord) -> Result<(), KmEvalError> {
    let line = serde_json::to_string(result).map_err(|_| KmEvalError::WriteUsage)?;
    append_line(path, &line).map_err(|_| KmEvalError::WriteUsage)
}

fn write_report(result: &KmEvalResult) -> Result<(), KmEvalError> {
    let path = PathBuf::from(&result.report_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| KmEvalError::WriteReport)?;
    }
    let contents = serde_json::to_string_pretty(result).map_err(|_| KmEvalError::WriteReport)?;
    fs::write(path, format!("{contents}\n")).map_err(|_| KmEvalError::WriteReport)
}

fn append_line(path: &Path, line: &str) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
#[path = "km_eval_tests.rs"]
mod tests;
