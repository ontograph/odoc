use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;
use crate::km_store::KmStore;

// Backlog 93: a failing negative-control case blocks candidate approval.
#[test]
fn eval_negative_control_failure_blocks_candidate_pass() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "expected signal");
    KmStore::new(&store_root).import(&source).expect("import");
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    // A negative control that the package actually contains: it must fail.
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": "Negative control",
            "requiredSubstrings": ["expected signal"],
            "expectPresent": false
        })
        .to_string(),
    )
    .expect("case");

    let result = run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");

    assert_eq!(result.failed_cases, 1);
    assert!(
        !candidate_eval_passed(&result),
        "a failing negative control must block candidate approval"
    );
}

// Backlog 94: usage-log records carry only bounded metadata, never case text
// or secret-shaped content.
#[test]
fn eval_usage_log_records_are_bounded_and_secret_free_per_case() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "expected signal");
    KmStore::new(&store_root).import(&source).expect("import");
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    let secret = "sk-live-usage-log-secret";
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": format!("Authorization: Bearer {secret}"),
            "requiredSubstrings": ["expected signal"],
        })
        .to_string(),
    )
    .expect("case");

    run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");

    let usage = fs::read_to_string(km_root.join("_logs/km_usage.jsonl")).expect("usage log");
    assert!(
        !usage.contains(secret),
        "usage log must not echo case prompt text: {usage}"
    );
    // Exactly one bounded metadata record per eval run.
    assert_eq!(usage.lines().filter(|line| !line.is_empty()).count(), 1);
    let record: serde_json::Value =
        serde_json::from_str(usage.lines().next().expect("record")).expect("json record");
    assert!(record.get("contextBytes").is_some());
    assert!(
        record.get("prompt").is_none(),
        "no prompt body is recorded: {record}"
    );
}

#[test]
fn run_km_eval_writes_accuracy_report_and_usage_log() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "expected signal");
    KmStore::new(&store_root).import(&source).expect("import");
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": "How should this module be used?",
            "requiredSubstrings": ["expected signal"]
        })
        .to_string(),
    )
    .expect("case");

    let result = run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");
    let accuracy = fs::read_to_string(km_root.join("_eval/km_accuracy.jsonl")).expect("accuracy");
    let report = fs::read_to_string(km_root.join("_eval/report.json")).expect("report");
    let usage = fs::read_to_string(km_root.join("_logs/km_usage.jsonl")).expect("usage");

    assert_eq!(result.total_cases, 1);
    assert_eq!(result.passed_cases, 1);
    assert_eq!(result.failed_cases, 0);
    assert_eq!(result.pass_rate, 1.0);
    assert!(accuracy.contains("\"passed\":true"));
    assert!(report.contains("\"passRate\": 1.0"));
    assert!(usage.contains("\"resolvedActive\":true"));
}

#[test]
fn run_km_eval_does_not_write_secret_shaped_case_text() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "ordinary signal");
    KmStore::new(&store_root).import(&source).expect("import");
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": "token sk-secret-1234567890 should not be logged",
            "requiredSubstrings": ["ghp_secretShouldNotAppear"]
        })
        .to_string(),
    )
    .expect("case");

    let result = run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");
    let accuracy = fs::read_to_string(km_root.join("_eval/km_accuracy.jsonl")).expect("accuracy");
    let report = fs::read_to_string(km_root.join("_eval/report.json")).expect("report");
    let usage = fs::read_to_string(km_root.join("_logs/km_usage.jsonl")).expect("usage");

    assert_eq!(result.failed_cases, 1);
    let combined = format!("{accuracy}\n{report}\n{usage}");
    assert!(!combined.contains("sk-secret"));
    assert!(!combined.contains("ghp_secretShouldNotAppear"));
}

#[test]
fn run_km_eval_supports_negative_control_cases() {
    let tempdir = TempDir::new().expect("temp dir");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    let source = write_valid_km(tempdir.path(), "ordinary signal");
    KmStore::new(&store_root).import(&source).expect("import");
    let cases_dir = km_root.join("_eval/suite");
    fs::create_dir_all(&cases_dir).expect("cases dir");
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": "Does the KM mention unrelated material?",
            "requiredSubstrings": ["quantum teleportation protocol"],
            "expectPresent": false
        })
        .to_string(),
    )
    .expect("case");

    let result = run_km_eval(
        "codebase-graph/creating-ontograph",
        &store_root,
        /*cases_dir*/ None,
    )
    .expect("eval");

    assert_eq!(result.total_cases, 1);
    assert_eq!(result.passed_cases, 1);
    assert_eq!(result.failed_cases, 0);
    assert_eq!(result.pass_rate, 1.0);
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
    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )
    .expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n").expect("test");
    km_root
}

fn valid_manifest() -> &'static str {
    "id = \"codebase-graph/creating-ontograph\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
}
