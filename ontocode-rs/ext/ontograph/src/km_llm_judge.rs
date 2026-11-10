use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde::Serialize;

use crate::km::load_facts;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::run_model_text;

const EVAL_SUITE_DIR: &str = "_eval/suite";
const EVAL_RESULTS: &str = "_eval/km_accuracy.jsonl";
const DRAFT_JUDGMENTS: &str = "_eval/draft_judgments.jsonl";
const MAX_MODEL_TEXT_CHARS: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmLlmJudgeRequest {
    pub km_root: PathBuf,
    pub package_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmLlmJudgeResult {
    pub cases_generated: usize,
    pub judgments_written: usize,
    pub judgments_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KmLlmJudgeError {
    MissingKmRoot,
    ReadFacts,
    WriteCase,
    MissingAccuracy,
    ReadAccuracy,
    ReadCases,
    InvalidAccuracy(String),
    InvalidCase(String),
    WriteJudgment,
}

pub fn run_km_llm_judge(
    request: &KmLlmJudgeRequest,
    model_ctx: Option<KmLlmModelContext<'_>>,
) -> Result<KmLlmJudgeResult, KmLlmJudgeError> {
    if !request.km_root.is_dir() {
        return Err(KmLlmJudgeError::MissingKmRoot);
    }
    let cases_generated = match request.package_dir.as_deref() {
        Some(package_dir) => generate_cases(package_dir, &request.km_root.join(EVAL_SUITE_DIR))?,
        None => 0,
    };
    let cases = read_cases(&request.km_root.join(EVAL_SUITE_DIR))?;
    let accuracy_path = request.km_root.join(EVAL_RESULTS);
    let mut judgments = read_accuracy(&accuracy_path)?
        .into_iter()
        .map(|result| {
            let case = cases.get(&result.case_id);
            build_judgment(result, case)
        })
        .collect::<Vec<_>>();
    if let Some(ctx) = model_ctx {
        let prompt = assemble_judge_prompt(&judgments, &cases);
        if let Some(output) = run_model_text(ctx.config, &prompt, ctx.model) {
            apply_model_judgments(&mut judgments, &output);
        }
    }
    let judgments_path = request.km_root.join(DRAFT_JUDGMENTS);
    append_judgments(&judgments_path, &judgments)?;
    Ok(KmLlmJudgeResult {
        cases_generated,
        judgments_written: judgments.len(),
        judgments_path: judgments_path.display().to_string(),
    })
}

impl std::fmt::Display for KmLlmJudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKmRoot => write!(f, "KM root directory is missing"),
            Self::ReadFacts => write!(f, "failed to read KM package facts"),
            Self::WriteCase => write!(f, "failed to write generated eval case"),
            Self::MissingAccuracy => write!(f, "KM accuracy results are missing"),
            Self::ReadAccuracy => write!(f, "failed to read KM accuracy results"),
            Self::ReadCases => write!(f, "failed to read KM eval cases"),
            Self::InvalidAccuracy(error) => write!(f, "invalid KM accuracy row: {error}"),
            Self::InvalidCase(error) => write!(f, "invalid KM eval case: {error}"),
            Self::WriteJudgment => write!(f, "failed to write draft judgment rows"),
        }
    }
}

impl std::error::Error for KmLlmJudgeError {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccuracyRow {
    case_id: String,
    id: String,
    version: String,
    passed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalCase {
    id: String,
    prompt: String,
    required_substrings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JudgmentRow {
    case_id: String,
    id: String,
    version: String,
    deterministic_passed: bool,
    advisory_score: f64,
    advisory_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    advisory_rationale: Option<String>,
    prompt_echo_risk: bool,
    judge: String,
    judged_at_unix_seconds: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelJudgeResponse {
    judgments: Vec<ModelJudgment>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelJudgment {
    case_id: String,
    advisory_score: Option<f64>,
    advisory_status: Option<String>,
    advisory_rationale: Option<String>,
}

fn generate_cases(package_dir: &Path, cases_dir: &Path) -> Result<usize, KmLlmJudgeError> {
    let facts = load_facts(package_dir).map_err(|_| KmLlmJudgeError::ReadFacts)?;
    fs::create_dir_all(cases_dir).map_err(|_| KmLlmJudgeError::WriteCase)?;
    let mut count = 0;
    for fact in facts.into_iter().take(20) {
        let case = EvalCase {
            id: format!("draft-generated-{}", fact.id),
            prompt: format!("When should `{}` knowledge be used?", fact.kind),
            required_substrings: vec![fact.claim],
        };
        let contents =
            serde_json::to_string_pretty(&case).map_err(|_| KmLlmJudgeError::WriteCase)?;
        fs::write(
            cases_dir.join(format!("{}.json", case.id)),
            format!("{contents}\n"),
        )
        .map_err(|_| KmLlmJudgeError::WriteCase)?;
        count += 1;
    }
    Ok(count)
}

fn read_cases(cases_dir: &Path) -> Result<BTreeMap<String, EvalCase>, KmLlmJudgeError> {
    if !cases_dir.exists() {
        return Ok(BTreeMap::new());
    }
    let entries = fs::read_dir(cases_dir).map_err(|_| KmLlmJudgeError::ReadCases)?;
    let mut cases = BTreeMap::new();
    for entry in entries {
        let path = entry.map_err(|_| KmLlmJudgeError::ReadCases)?.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|_| KmLlmJudgeError::ReadCases)?;
        let case = serde_json::from_str::<EvalCase>(&raw)
            .map_err(|error| KmLlmJudgeError::InvalidCase(error.to_string()))?;
        cases.insert(case.id.clone(), case);
    }
    Ok(cases)
}

fn read_accuracy(path: &Path) -> Result<Vec<AccuracyRow>, KmLlmJudgeError> {
    let contents = fs::read_to_string(path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            KmLlmJudgeError::MissingAccuracy
        } else {
            KmLlmJudgeError::ReadAccuracy
        }
    })?;
    let mut rows = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        rows.push(
            serde_json::from_str::<AccuracyRow>(line)
                .map_err(|error| KmLlmJudgeError::InvalidAccuracy(error.to_string()))?,
        );
    }
    Ok(rows)
}

fn build_judgment(result: AccuracyRow, case: Option<&EvalCase>) -> JudgmentRow {
    let prompt_echo_risk = case.is_some_and(|case| {
        case.required_substrings
            .iter()
            .any(|required| case.prompt.contains(required))
    });
    let advisory_score = match (result.passed, prompt_echo_risk) {
        (true, false) => 1.0,
        (true, true) => 0.5,
        (false, _) => 0.0,
    };
    let advisory_status = match (result.passed, prompt_echo_risk) {
        (true, false) => "accepted_by_deterministic_eval",
        (true, true) => "needs_review_prompt_echo_risk",
        (false, _) => "failed_deterministic_eval",
    }
    .to_string();
    JudgmentRow {
        case_id: result.case_id,
        id: result.id,
        version: result.version,
        deterministic_passed: result.passed,
        advisory_score,
        advisory_status,
        advisory_rationale: None,
        prompt_echo_risk,
        judge: "offline-advisory-precheck".to_string(),
        judged_at_unix_seconds: now_unix_seconds(),
    }
}

fn assemble_judge_prompt(judgments: &[JudgmentRow], cases: &BTreeMap<String, EvalCase>) -> String {
    let rows = judgments
        .iter()
        .map(|judgment| {
            let case = cases.get(&judgment.case_id);
            serde_json::json!({
                "caseId": judgment.case_id,
                "id": judgment.id,
                "version": judgment.version,
                "deterministicPassed": judgment.deterministic_passed,
                "offlineAdvisoryStatus": judgment.advisory_status,
                "promptEchoRisk": judgment.prompt_echo_risk,
                "prompt": case.map(|case| case.prompt.as_str()),
                "requiredSubstrings": case.map(|case| &case.required_substrings),
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Review these KM eval results as advisory-only judge metadata. Return only JSON shaped as {{\"judgments\":[{{\"caseId\":\"...\",\"advisoryScore\":0.0,\"advisoryStatus\":\"...\",\"advisoryRationale\":\"...\"}}]}}. Do not include provider, model, secret, endpoint, or credential details.\n\n{rows}\n"
    )
}

fn apply_model_judgments(judgments: &mut [JudgmentRow], output: &str) {
    let Ok(response) = serde_json::from_str::<ModelJudgeResponse>(output) else {
        return;
    };
    let by_case = response
        .judgments
        .into_iter()
        .map(|judgment| (judgment.case_id.clone(), judgment))
        .collect::<BTreeMap<_, _>>();
    for judgment in judgments {
        let Some(model_judgment) = by_case.get(&judgment.case_id) else {
            continue;
        };
        let mut applied = false;
        if let Some(score) = model_judgment
            .advisory_score
            .filter(|score| score.is_finite())
        {
            judgment.advisory_score = score.clamp(0.0, 1.0);
            applied = true;
        }
        if let Some(status) = model_judgment
            .advisory_status
            .as_deref()
            .and_then(normalize_model_text)
        {
            judgment.advisory_status = status;
            applied = true;
        }
        if let Some(rationale) = model_judgment
            .advisory_rationale
            .as_deref()
            .and_then(normalize_model_text)
        {
            judgment.advisory_rationale = Some(rationale);
            applied = true;
        }
        if applied {
            judgment.judge = "model-advisory".to_string();
        }
    }
}

fn normalize_model_text(value: &str) -> Option<String> {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.is_empty() || value.chars().count() > MAX_MODEL_TEXT_CHARS {
        return None;
    }
    Some(value)
}

fn append_judgments(path: &Path, judgments: &[JudgmentRow]) -> Result<(), KmLlmJudgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| KmLlmJudgeError::WriteJudgment)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| KmLlmJudgeError::WriteJudgment)?;
    for judgment in judgments {
        let line = serde_json::to_string(judgment).map_err(|_| KmLlmJudgeError::WriteJudgment)?;
        writeln!(file, "{line}").map_err(|_| KmLlmJudgeError::WriteJudgment)?;
    }
    Ok(())
}

fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
#[path = "km_llm_judge_tests.rs"]
mod tests;
