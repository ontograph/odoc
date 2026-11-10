use std::cell::Cell;
use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::KmLlmJudgeError;
use super::KmLlmJudgeRequest;
use super::run_km_llm_judge;
use crate::km_eval::run_km_eval;
use crate::km_llm_client::KmLlmClientConfig;
use crate::km_llm_client::KmLlmClientError;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::KmLlmTextModel;
use crate::km_store::KmStore;

#[test]
fn judge_writes_separate_advisory_output_without_mutating_accuracy() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "expected signal");
    KmStore::new(&store_root).import(&source).expect("import");
    write_case(&km_root, "case-1", "expected signal", "expected signal");
    run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");
    let accuracy_before =
        fs::read_to_string(km_root.join("_eval/km_accuracy.jsonl")).expect("accuracy");

    let result = run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: km_root.clone(),
            package_dir: None,
        },
        None,
    )
    .expect("judge");

    let accuracy_after =
        fs::read_to_string(km_root.join("_eval/km_accuracy.jsonl")).expect("accuracy");
    let judgments =
        fs::read_to_string(km_root.join("_eval/draft_judgments.jsonl")).expect("judgments");
    assert_eq!(accuracy_after, accuracy_before);
    assert_eq!(result.cases_generated, 0);
    assert_eq!(result.judgments_written, 1);
    assert!(judgments.contains("\"caseId\":\"case-1\""));
    assert!(judgments.contains("\"promptEchoRisk\":true"));
    assert!(judgments.contains("\"advisoryStatus\":\"needs_review_prompt_echo_risk\""));
}

#[test]
fn judge_can_generate_eval_cases_from_package_facts() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    fs::create_dir_all(&km_root).expect("km root");
    let source = write_valid_km(tempdir.path(), "generated signal");
    fs::create_dir_all(km_root.join("_eval")).expect("eval dir");
    fs::write(
        km_root.join("_eval/km_accuracy.jsonl"),
        "{\"caseId\":\"missing\",\"id\":\"codebase-graph/creating-ontograph\",\"version\":\"0.1.0\",\"passed\":false}\n",
    )
    .expect("accuracy");

    let result = run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: km_root.clone(),
            package_dir: Some(source),
        },
        None,
    )
    .expect("judge");

    let generated_case =
        fs::read_to_string(km_root.join("_eval/suite/draft-generated-fact-1.json"))
            .expect("generated case");
    assert_eq!(result.cases_generated, 1);
    assert_eq!(result.judgments_written, 1);
    assert!(generated_case.contains("generated signal"));
}

#[test]
fn judge_requires_existing_km_root() {
    let tempdir = TempDir::new().expect("temp dir");

    let err = run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: tempdir.path().join("missing"),
            package_dir: None,
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmJudgeError::MissingKmRoot);
}

#[test]
fn judge_uses_model_advisory_row_when_enabled() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    write_accuracy(&km_root, true);
    write_case(&km_root, "case-1", "expected signal", "different signal");
    let config = enabled_local_config();
    let model = StubModel {
        response: Ok(serde_json::json!({
            "judgments": [{
                "caseId": "case-1",
                "advisoryScore": 0.25,
                "advisoryStatus": "model_needs_review",
                "advisoryRationale": "Model-authored advisory only."
            }]
        })
        .to_string()),
        calls: Cell::new(0),
    };

    let result = run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: km_root.clone(),
            package_dir: None,
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("judge");

    let accuracy = fs::read_to_string(km_root.join("_eval/km_accuracy.jsonl")).expect("accuracy");
    let judgments =
        fs::read_to_string(km_root.join("_eval/draft_judgments.jsonl")).expect("judgments");
    assert_eq!(model.calls.get(), 1);
    assert_eq!(result.judgments_written, 1);
    assert_eq!(
        accuracy,
        "{\"caseId\":\"case-1\",\"id\":\"codebase-graph/creating-ontograph\",\"version\":\"0.1.0\",\"passed\":true}\n"
    );
    assert!(judgments.contains("\"advisoryScore\":0.25"));
    assert!(judgments.contains("\"advisoryStatus\":\"model_needs_review\""));
    assert!(judgments.contains("\"advisoryRationale\":\"Model-authored advisory only.\""));
    assert!(judgments.contains("\"judge\":\"model-advisory\""));
}

#[test]
fn judge_falls_back_to_deterministic_row_when_model_disabled() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    write_accuracy(&km_root, true);
    write_case(&km_root, "case-1", "expected signal", "different signal");
    let mut config = enabled_local_config();
    config.enabled = false;
    let model = StubModel {
        response: Ok(serde_json::json!({
            "judgments": [{
                "caseId": "case-1",
                "advisoryScore": 0.25,
                "advisoryStatus": "model_needs_review"
            }]
        })
        .to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: km_root.clone(),
            package_dir: None,
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("judge");

    let judgments =
        fs::read_to_string(km_root.join("_eval/draft_judgments.jsonl")).expect("judgments");
    assert_eq!(model.calls.get(), 0);
    assert!(judgments.contains("\"advisoryScore\":1.0"));
    assert!(judgments.contains("\"advisoryStatus\":\"accepted_by_deterministic_eval\""));
    assert!(judgments.contains("\"judge\":\"offline-advisory-precheck\""));
    assert!(!judgments.contains("model_needs_review"));
}

#[test]
fn judge_falls_back_to_deterministic_row_when_model_response_is_invalid() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    write_accuracy(&km_root, false);
    let config = enabled_local_config();
    let model = StubModel {
        response: Ok("not json".to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_judge(
        &KmLlmJudgeRequest {
            km_root: km_root.clone(),
            package_dir: None,
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("judge");

    let judgments =
        fs::read_to_string(km_root.join("_eval/draft_judgments.jsonl")).expect("judgments");
    assert_eq!(model.calls.get(), 1);
    assert!(judgments.contains("\"advisoryScore\":0.0"));
    assert!(judgments.contains("\"advisoryStatus\":\"failed_deterministic_eval\""));
    assert!(judgments.contains("\"judge\":\"offline-advisory-precheck\""));
}

fn write_case(km_root: &Path, id: &str, prompt: &str, required: &str) {
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    fs::write(
        cases_dir.join(format!("{id}.json")),
        serde_json::json!({
            "id": id,
            "prompt": prompt,
            "requiredSubstrings": [required]
        })
        .to_string(),
    )
    .expect("case");
}

fn write_accuracy(km_root: &Path, passed: bool) {
    fs::create_dir_all(km_root.join("_eval")).expect("eval dir");
    fs::write(
        km_root.join("_eval/km_accuracy.jsonl"),
        format!(
            "{{\"caseId\":\"case-1\",\"id\":\"codebase-graph/creating-ontograph\",\"version\":\"0.1.0\",\"passed\":{passed}}}\n"
        ),
    )
    .expect("accuracy");
}

struct StubModel {
    response: Result<String, KmLlmClientError>,
    calls: Cell<usize>,
}

impl KmLlmTextModel for StubModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.calls.set(self.calls.get() + 1);
        self.response.clone()
    }
}

fn enabled_local_config() -> KmLlmClientConfig {
    KmLlmClientConfig {
        enabled: true,
        offline_only: true,
        provider_is_local: true,
        max_input_tokens: None,
        model_slug: "test-model".to_string(),
        provider_id: "local".to_string(),
        project_root: None,
    }
}

fn write_valid_km(root: &Path, claim: &str) -> std::path::PathBuf {
    let km_root = root.join("sample-km");
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), valid_manifest()).expect("manifest");
    fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    fs::write(km_root.join("examples/README.md"), "# Example\n").expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n").expect("test");
    km_root
}

fn valid_manifest() -> &'static str {
    "id = \"codebase-graph/creating-ontograph\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
}

// Backlog 91: deterministic case generation. The same package facts must yield
// byte-identical generated eval cases across independent runs, so an eval
// suite cannot drift under a fixed input.
#[test]
fn judge_case_generation_is_deterministic_for_fixed_facts() {
    let generate = || {
        let tempdir = TempDir::new().expect("temp dir");
        let km_root = tempdir.path().join(".ontocode/ontograph/km");
        fs::create_dir_all(km_root.join("_eval")).expect("eval dir");
        let source = write_valid_km(tempdir.path(), "generated signal");
        fs::write(
            km_root.join("_eval/km_accuracy.jsonl"),
            "{\"caseId\":\"missing\",\"id\":\"codebase-graph/creating-ontograph\",\"version\":\"0.1.0\",\"passed\":false}\n",
        )
        .expect("accuracy");

        let result = run_km_llm_judge(
            &KmLlmJudgeRequest {
                km_root: km_root.clone(),
                package_dir: Some(source),
            },
            None,
        )
        .expect("judge");
        let case = fs::read_to_string(km_root.join("_eval/suite/draft-generated-fact-1.json"))
            .expect("generated case");
        (result.cases_generated, case)
    };

    let (first_count, first_case) = generate();
    let (second_count, second_case) = generate();

    assert_eq!(first_count, second_count);
    assert_eq!(
        first_case, second_case,
        "generated eval cases must be byte-identical for fixed facts"
    );
}
