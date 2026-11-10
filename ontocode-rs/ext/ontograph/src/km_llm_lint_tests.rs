use std::cell::Cell;
use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::KmLlmLintError;
use super::KmLlmLintRequest;
use super::run_km_llm_lint;
use crate::km_llm_client::KmLlmClientConfig;
use crate::km_llm_client::KmLlmClientError;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::KmLlmTextModel;

#[test]
fn lint_writes_advisory_report_after_health_check() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), facts_with_findings());

    let result = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        None,
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(result.findings, 3);
    assert!(result.health_checked);
    assert!(report.contains("Deterministic KM health check passed"));
    assert!(report.contains("weak-confidence"));
    assert!(report.contains("weak-citation-shape"));
    assert!(report.contains("duplicate-claim"));
}

#[test]
fn lint_does_not_write_report_when_health_check_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), "");

    let err = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        None,
    )
    .unwrap_err();

    assert!(matches!(err, KmLlmLintError::HealthFailed(_)));
    assert!(!package.join("okf/draft_lint.md").exists());
}

#[test]
fn lint_requires_existing_package() {
    let tempdir = TempDir::new().expect("temp dir");

    let err = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: tempdir.path().join("missing"),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmLintError::MissingPackage);
}

struct StubModel {
    notes: Result<String, KmLlmClientError>,
    calls: Cell<usize>,
}

impl KmLlmTextModel for StubModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.calls.set(self.calls.get() + 1);
        self.notes.clone()
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

#[test]
fn lint_appends_model_notes_when_enabled() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), facts_with_findings());
    let config = enabled_local_config();
    let model = StubModel {
        notes: Ok("Model advisory: review weak citations.".to_string()),
        calls: Cell::new(0),
    };

    let result = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(model.calls.get(), 1);
    assert_eq!(result.findings, 3);
    assert!(report.contains("## Model Advisory Notes"));
    assert!(report.contains("Model advisory: review weak citations."));
}

#[test]
fn lint_falls_back_to_deterministic_report_when_disabled() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), facts_with_findings());
    let mut config = enabled_local_config();
    config.enabled = false;
    let model = StubModel {
        notes: Ok("Model advisory: review weak citations.".to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(model.calls.get(), 0);
    assert!(!report.contains("## Model Advisory Notes"));
    assert!(report.contains("weak-confidence"));
}

#[test]
fn lint_ignores_invalid_model_notes() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), facts_with_findings());
    let config = enabled_local_config();
    let model = StubModel {
        notes: Err(KmLlmClientError::ModelCall("boom".to_string())),
        calls: Cell::new(0),
    };

    run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(model.calls.get(), 1);
    assert!(!report.contains("## Model Advisory Notes"));
}

#[test]
fn lint_skips_model_call_when_no_findings() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), clean_facts());
    let config = enabled_local_config();
    let model = StubModel {
        notes: Ok("Should not be used.".to_string()),
        calls: Cell::new(0),
    };

    let result = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(result.findings, 0);
    assert_eq!(model.calls.get(), 0);
    assert!(report.contains("No advisory findings."));
    assert!(!report.contains("## Model Advisory Notes"));
}

#[test]
fn lint_falls_back_when_input_cap_exceeded() {
    let tempdir = TempDir::new().expect("temp dir");
    let package = write_valid_km(tempdir.path(), facts_with_findings());
    let mut config = enabled_local_config();
    config.max_input_tokens = Some(1);
    let model = StubModel {
        notes: Ok("Model advisory: capped.".to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("lint");

    let report = fs::read_to_string(package.join("okf/draft_lint.md")).expect("report");
    assert_eq!(model.calls.get(), 0);
    assert!(!report.contains("## Model Advisory Notes"));
    assert!(report.contains("weak-confidence"));
}

fn clean_facts() -> &'static str {
    "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"Use graph facts.\",\"source\":\"silver:doc\",\"confidence\":0.9}\n"
}

fn facts_with_findings() -> &'static str {
    "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"Use graph facts.\",\"source\":\"silver:doc\",\"confidence\":0.9}\n{\"id\":\"fact-2\",\"kind\":\"rule\",\"claim\":\"Use graph facts.\",\"source\":\"local-doc\",\"confidence\":0.5}\n"
}

fn write_valid_km(root: &Path, facts: &str) -> std::path::PathBuf {
    let km_root = root.join("sample-km");
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::create_dir_all(km_root.join("okf")).expect("okf dir");
    fs::write(km_root.join("km.toml"), valid_manifest()).expect("manifest");
    fs::write(km_root.join("facts.jsonl"), facts).expect("facts");
    fs::write(km_root.join("examples/README.md"), "# Example\n").expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n").expect("test");
    km_root
}

fn valid_manifest() -> &'static str {
    "id = \"codebase-graph/creating-ontograph\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
}

// Backlog 92: the lint report is written to disk and may be read back by an
// agent, so secret-shaped fact content must not survive into it verbatim.
#[test]
fn lint_report_does_not_echo_secret_shaped_fact_content() {
    let tempdir = TempDir::new().expect("temp dir");
    let secret = "sk-live-super-secret-token-value";
    let package = write_valid_km(
        tempdir.path(),
        &format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"Authorization: Bearer {secret}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    );

    let result = run_km_llm_lint(
        &KmLlmLintRequest {
            package_dir: package,
        },
        None,
    )
    .expect("lint");

    let report = fs::read_to_string(&result.report_path).expect("report readable");
    assert!(
        !report.contains(secret),
        "lint report must not echo secret-shaped fact content: {report}"
    );
}
