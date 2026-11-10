use std::cell::Cell;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::KmLlmConsolidateError;
use super::KmLlmConsolidateRequest;
use super::run_km_llm_consolidate;
use crate::km_import::run_km_import;
use crate::km_llm_client::DraftMapping;
use crate::km_llm_client::KmLlmClientConfig;
use crate::km_llm_client::KmLlmClientError;
use crate::km_llm_client::KmLlmExtractDraft;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::KmLlmTextModel;

#[test]
fn consolidator_emits_importable_package_with_conflict_records() {
    let root = tempdir().expect("tempdir");
    let left = root.path().join("left");
    let right = root.path().join("right");
    write_km(
        &left,
        "left/source.md",
        "supports",
        "Left source says a graph is useful.",
    );
    write_km(
        &right,
        "right/source.md",
        "conflicts_with",
        "Left source says a graph is useful.",
    );
    let output_dir = root.path().join("_merged");

    let result = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![left, right],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .expect("consolidate");

    // The two inputs share a claim but cite different canonical sources, so the
    // Consolidation Contract keeps them as separate facts.
    assert_eq!(result.facts, 2);
    assert_eq!(result.conflicts, 1);
    let mappings = fs::read_to_string(output_dir.join("mappings.jsonl")).expect("mappings");
    assert!(mappings.contains("\"kind\":\"conflict_record\""));

    let import_result =
        run_km_import(&output_dir, &root.path().join("_store")).expect("merged imports");
    assert_eq!(import_result.id, "codebase-graph/draft-consolidated");
}

#[test]
fn consolidator_requires_input_packages() {
    let root = tempdir().expect("tempdir");

    let err = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: Vec::new(),
            output_dir: root.path().join("_merged"),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmConsolidateError::EmptyInputs);
}

#[test]
fn consolidator_requires_reserved_output_dir() {
    let root = tempdir().expect("tempdir");
    let input = root.path().join("input");
    write_km(&input, "source.md", "supports", "A graph fact.");

    let err = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![input],
            output_dir: root.path().join("merged"),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmConsolidateError::OutputDirNotReserved);
}

#[test]
fn consolidator_rejects_output_outside_project_root() {
    let project = tempdir().expect("project tempdir");
    let external = tempdir().expect("external tempdir");
    let input = project.path().join("input");
    write_km(&input, "source.md", "supports", "A graph fact.");
    let output_dir = external.path().join("_merged");

    let err = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![input],
            output_dir: output_dir.clone(),
            project_root: project.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmConsolidateError::OutputDirOutsideProjectRoot);
    assert!(!output_dir.exists());
}

#[test]
fn consolidator_preserves_existing_output_directory() {
    let root = tempdir().expect("tempdir");
    let input = root.path().join("input");
    write_km(&input, "source.md", "supports", "A graph fact.");
    let output_dir = root.path().join("_merged");
    fs::create_dir(&output_dir).expect("output dir");
    fs::write(output_dir.join("sentinel"), "keep").expect("sentinel");

    let err = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![input],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmConsolidateError::OutputDirExists);
    assert_eq!(
        fs::read_to_string(output_dir.join("sentinel")).expect("sentinel remains"),
        "keep"
    );
}

fn write_km(root: &Path, source: &str, relation: &str, claim: &str) {
    fs::create_dir_all(root.join("examples")).expect("examples");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::write(root.join("examples/README.md"), "# Example\n").expect("example");
    fs::write(root.join("tests/README.md"), "# Test\n").expect("test");
    fs::write(
        root.join("km.toml"),
        format!(
            "id = \"codebase-graph/test-{}\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Test KM package.\"\nsources = [\"bronze:{source}\"]\nlicenses = [\"MIT\"]\n\n[provenance]\ncreated_by = \"test\"\ngenerated_at = \"2026-07-06\"\nsource_commit = \"HEAD\"\n",
            relation.replace('_', "-")
        ),
    )
    .expect("manifest");
    fs::write(
        root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"graph_pattern\",\"claim\":\"{claim}\",\"source\":\"bronze:{source}\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    fs::write(
        root.join("mappings.jsonl"),
        format!(
            "{{\"id\":\"mapping-1\",\"kind\":\"concept_mapping\",\"source_concept\":\"raw graph\",\"target_concept\":\"km graph\",\"relation\":\"{relation}\",\"source\":\"bronze:{source}\",\"confidence\":0.8}}\n"
        ),
    )
    .expect("mappings");
}

struct ConsolidateStub {
    draft: KmLlmExtractDraft,
    calls: Cell<usize>,
}

impl ConsolidateStub {
    fn new(mappings: Vec<DraftMapping>) -> Self {
        Self {
            draft: KmLlmExtractDraft {
                facts: Vec::new(),
                mappings,
            },
            calls: Cell::new(0),
        }
    }
}

impl KmLlmTextModel for ConsolidateStub {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.calls.set(self.calls.get() + 1);
        serde_json::to_string(&self.draft).map_err(|err| KmLlmClientError::Parse(err.to_string()))
    }
}

fn enabled_config() -> KmLlmClientConfig {
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

fn model_mapping(source_concept: &str, target_concept: &str, relation: &str) -> DraftMapping {
    DraftMapping {
        id: "model-1".to_string(),
        kind: "concept_mapping".to_string(),
        source_concept: source_concept.to_string(),
        target_concept: target_concept.to_string(),
        relation: relation.to_string(),
        source: "km:test".to_string(),
        confidence: 0.9,
    }
}

fn conflicting_inputs(root: &Path) -> (KmLlmConsolidateRequest, std::path::PathBuf) {
    let left = root.join("left");
    let right = root.join("right");
    write_km(&left, "left/source.md", "supports", "A graph fact.");
    write_km(&right, "right/source.md", "refutes", "A graph fact.");
    let output_dir = root.join("_merged");
    (
        KmLlmConsolidateRequest {
            input_dirs: vec![left, right],
            output_dir: output_dir.clone(),
            project_root: root.to_path_buf(),
        },
        output_dir,
    )
}

#[test]
fn consolidator_uses_model_to_resolve_conflicts_when_enabled() {
    let root = tempdir().expect("tempdir");
    let (request, output_dir) = conflicting_inputs(root.path());
    let stub = ConsolidateStub::new(vec![model_mapping("raw graph", "km graph", "refutes")]);
    let cfg = enabled_config();

    let result = run_km_llm_consolidate(
        &request,
        Some(KmLlmModelContext {
            config: &cfg,
            model: &stub,
        }),
    )
    .expect("consolidate");

    assert_eq!(stub.calls.get(), 1);
    assert_eq!(result.conflicts, 0);
    let mappings = fs::read_to_string(output_dir.join("mappings.jsonl")).expect("mappings");
    assert!(!mappings.contains("\"kind\":\"conflict_record\""));
    assert!(mappings.contains("\"relation\":\"refutes\""));
}

#[test]
fn consolidator_ignores_model_rows_for_unknown_endpoints() {
    let root = tempdir().expect("tempdir");
    let (request, output_dir) = conflicting_inputs(root.path());
    let stub = ConsolidateStub::new(vec![model_mapping("other a", "other b", "supports")]);
    let cfg = enabled_config();

    let result = run_km_llm_consolidate(
        &request,
        Some(KmLlmModelContext {
            config: &cfg,
            model: &stub,
        }),
    )
    .expect("consolidate");

    assert_eq!(stub.calls.get(), 1);
    assert_eq!(result.conflicts, 1);
    let mappings = fs::read_to_string(output_dir.join("mappings.jsonl")).expect("mappings");
    assert!(mappings.contains("\"kind\":\"conflict_record\""));
}

#[test]
fn consolidator_disabled_config_keeps_deterministic_conflict() {
    let root = tempdir().expect("tempdir");
    let (request, _output_dir) = conflicting_inputs(root.path());
    let stub = ConsolidateStub::new(vec![model_mapping("raw graph", "km graph", "refutes")]);
    let mut cfg = enabled_config();
    cfg.enabled = false;

    let result = run_km_llm_consolidate(
        &request,
        Some(KmLlmModelContext {
            config: &cfg,
            model: &stub,
        }),
    )
    .expect("consolidate");

    assert_eq!(stub.calls.get(), 0);
    assert_eq!(result.conflicts, 1);
}

#[test]
fn consolidator_ignores_invalid_model_resolution_rows() {
    let root = tempdir().expect("tempdir");
    let (request, output_dir) = conflicting_inputs(root.path());
    let stub = ConsolidateStub::new(vec![model_mapping("raw graph", "km graph", "")]);
    let cfg = enabled_config();

    let result = run_km_llm_consolidate(
        &request,
        Some(KmLlmModelContext {
            config: &cfg,
            model: &stub,
        }),
    )
    .expect("consolidate");

    assert_eq!(stub.calls.get(), 1);
    assert_eq!(result.conflicts, 1);
    let mappings = fs::read_to_string(output_dir.join("mappings.jsonl")).expect("mappings");
    assert!(mappings.contains("\"kind\":\"conflict_record\""));
}

#[test]
fn consolidator_rejects_model_relation_outside_deterministic_candidates() {
    let root = tempdir().expect("tempdir");
    let (request, output_dir) = conflicting_inputs(root.path());
    let stub = ConsolidateStub::new(vec![model_mapping(
        "raw graph",
        "km graph",
        "invented_relation",
    )]);
    let cfg = enabled_config();

    let result = run_km_llm_consolidate(
        &request,
        Some(KmLlmModelContext {
            config: &cfg,
            model: &stub,
        }),
    )
    .expect("consolidate");

    assert_eq!(result.conflicts, 1);
    let mappings = fs::read_to_string(output_dir.join("mappings.jsonl")).expect("mappings");
    assert!(mappings.contains("\"kind\":\"conflict_record\""));
    assert!(!mappings.contains("invented_relation"));
}

// Backlog 89: an exact repeat of the same claim from the same source collapses
// to one fact (ADR Consolidation Contract).
#[test]
fn consolidate_collapses_exact_repeats_from_same_source() {
    let root = tempdir().expect("tempdir");
    let left = root.path().join("left");
    let right = root.path().join("right");
    // Same claim, same canonical source, differing only in package identity.
    write_km(
        &left,
        "shared/source.md",
        "supports",
        "A graph is useful for retrieval.",
    );
    write_km(
        &right,
        "shared/source.md",
        "refines",
        "A graph is useful for retrieval.",
    );
    let output_dir = root.path().join("_merged");

    let result = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![left, right],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .expect("consolidate");

    assert_eq!(result.facts, 1, "exact repeats collapse to one fact");
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert_eq!(facts.lines().filter(|line| !line.is_empty()).count(), 1);
}

// Backlog 88: the ADR Consolidation Contract requires fact identity to include
// the canonical source, so identical claims from *different* sources stay
// separate.
#[test]
fn consolidate_keeps_identical_claims_from_different_sources_separate() {
    let root = tempdir().expect("tempdir");
    let left = root.path().join("left");
    let right = root.path().join("right");
    write_km(
        &left,
        "left/source.md",
        "supports",
        "A graph is useful for retrieval.",
    );
    write_km(
        &right,
        "right/source.md",
        "refines",
        "A graph is useful for retrieval.",
    );
    let output_dir = root.path().join("_merged");

    let result = run_km_llm_consolidate(
        &KmLlmConsolidateRequest {
            input_dirs: vec![left, right],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .expect("consolidate");

    assert_eq!(
        result.facts, 2,
        "distinct canonical sources preserve separate facts"
    );
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    let kept_sources = facts
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| line.contains("right/source.md"))
        .count();
    assert_eq!(
        kept_sources, 1,
        "the second source is preserved as its own fact"
    );
}
