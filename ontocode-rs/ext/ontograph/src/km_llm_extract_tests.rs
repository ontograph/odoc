use std::fs;

use tempfile::tempdir;

use super::KmLlmExtractError;
use super::KmLlmExtractRequest;
use super::MAX_MODEL_PROMPT_BYTES;
use super::MODEL_INSTRUCTION_HEADROOM_BYTES;
use super::ParsedSource;
use super::assemble_prompt_text;
use super::run_km_llm_extract;
use crate::km_import::run_km_import;
use crate::km_llm_client::DraftFact;
use crate::km_llm_client::DraftMapping;
use crate::km_llm_client::KmLlmClientConfig;
use crate::km_llm_client::KmLlmClientError;
use crate::km_llm_client::KmLlmExtractDraft;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::KmLlmTextModel;
use crate::km_llm_client::MODEL_EXTRACT_INSTRUCTION;

struct StubModel {
    draft: Result<KmLlmExtractDraft, KmLlmClientError>,
}

impl KmLlmTextModel for StubModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.draft.clone().and_then(|draft| {
            serde_json::to_string(&draft).map_err(|err| KmLlmClientError::Parse(err.to_string()))
        })
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

fn model_fact(source: &str, claim: &str) -> DraftFact {
    DraftFact {
        id: "model-fact-1".to_string(),
        kind: "model_fact".to_string(),
        claim: claim.to_string(),
        source: source.to_string(),
        confidence: 0.9,
    }
}

fn model_mapping(source: &str) -> DraftMapping {
    DraftMapping {
        id: "model-mapping-1".to_string(),
        kind: "model_mapping".to_string(),
        source_concept: "docs/source.md".to_string(),
        target_concept: "ontograph_km_fact".to_string(),
        relation: "candidate_source_for".to_string(),
        source: source.to_string(),
        confidence: 0.9,
    }
}

#[test]
fn extractor_emits_importable_draft_package() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    assert!(output_dir.join("facts.jsonl").is_file());
    assert!(output_dir.join("mappings.jsonl").is_file());

    let import_result =
        run_km_import(&output_dir, &root.path().join("_store")).expect("draft imports");
    assert_eq!(import_result.id, "codebase-graph/draft-extract");
}

#[test]
fn extractor_rejects_missing_prefixed_source() {
    let root = tempdir().expect("tempdir");

    let err = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["bronze:missing.md".to_string()],
            output_dir: root.path().join("_draft"),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(
        err,
        KmLlmExtractError::MissingSource("bronze:missing.md".to_string())
    );
}

#[test]
fn extractor_requires_reserved_output_dir() {
    let root = tempdir().expect("tempdir");
    fs::write(root.path().join("source.md"), "# Source\n").expect("source");

    let err = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:source.md".to_string()],
            output_dir: root.path().join("draft"),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmExtractError::OutputDirNotReserved);
}

#[test]
fn extractor_rejects_output_outside_project_root() {
    let project = tempdir().expect("project tempdir");
    let external = tempdir().expect("external tempdir");
    fs::write(project.path().join("source.md"), "# Source\n").expect("source");
    let output_dir = external.path().join("_draft");

    let err = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: project.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmExtractError::OutputDirOutsideProjectRoot);
    assert!(!output_dir.exists());
}

#[test]
fn extractor_preserves_existing_output_directory() {
    let root = tempdir().expect("tempdir");
    fs::write(root.path().join("source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");
    fs::create_dir(&output_dir).expect("output dir");
    fs::write(output_dir.join("sentinel"), "keep").expect("sentinel");

    let err = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmExtractError::OutputDirExists);
    assert_eq!(
        fs::read_to_string(output_dir.join("sentinel")).expect("sentinel remains"),
        "keep"
    );
}

#[test]
fn extractor_prompt_never_exceeds_aggregate_byte_cap() {
    let root = tempdir().expect("tempdir");
    fs::write(
        root.path().join("source.bin"),
        vec![0xff; MAX_MODEL_PROMPT_BYTES * 2],
    )
    .expect("source");

    let prompt = assemble_prompt_text(
        &[ParsedSource {
            raw: "silver:source.bin".to_string(),
            relative_path: "source.bin".to_string(),
        }],
        root.path(),
    )
    .expect("prompt");

    assert_eq!(prompt.text.len(), MAX_MODEL_PROMPT_BYTES);
    assert!(
        !prompt.covers_all_sources,
        "a truncated source is not fully represented"
    );
}

#[test]
fn extractor_cleans_staging_after_generated_package_validation_fails() {
    let root = tempdir().expect("tempdir");
    let source_name = "source\ninvalid.md";
    fs::write(root.path().join(source_name), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let err = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec![format!("silver:{source_name}")],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert!(matches!(err, KmLlmExtractError::InvalidDraft(_)));
    assert!(!output_dir.exists());
    assert!(
        fs::read_dir(root.path())
            .expect("project entries")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .contains(".staging-"))
    );
}

#[test]
fn extractor_writes_model_rows_when_enabled() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/source.md", "model derived claim")],
            mappings: vec![model_mapping("silver:docs/source.md")],
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    assert_eq!(result.mappings, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("model derived claim"));
    assert!(facts.contains("model_fact"));
}

#[test]
fn extractor_falls_back_when_disabled() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let mut config = enabled_local_config();
    config.enabled = false;
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/source.md", "model derived claim")],
            mappings: vec![model_mapping("silver:docs/source.md")],
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
    assert!(!facts.contains("model derived claim"));
}

#[test]
fn extractor_falls_back_for_missing_model_source_rows() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/does-not-exist.md", "hallucinated")],
            mappings: vec![],
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
    assert!(!facts.contains("hallucinated"));
}

#[test]
fn extractor_falls_back_when_model_rows_violate_native_bounds() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let mut invalid = model_fact("silver:docs/source.md", "model derived claim");
    invalid.id.clear();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![invalid],
            mappings: Vec::new(),
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
    assert!(!facts.contains("model derived claim"));
}

#[test]
fn extractor_falls_back_when_model_returns_no_facts() {
    let root = tempdir().expect("tempdir");
    fs::write(root.path().join("source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");
    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: Vec::new(),
            mappings: Vec::new(),
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
}

#[test]
fn extractor_falls_back_when_model_row_source_escapes_project_root() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:../secret.md", "model derived claim")],
            mappings: Vec::new(),
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
    assert!(!facts.contains("model derived claim"));
}

#[test]
fn extractor_canonicalizes_model_row_sources() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/./source.md", "model derived claim")],
            mappings: Vec::new(),
        }),
    };

    run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("\"source\":\"silver:docs/source.md\""));
    assert!(!facts.contains("docs/./source.md"));
}

/// Record a model that must never be consulted: any call is the failure.
struct ForbiddenModel {
    called: std::cell::Cell<bool>,
}

impl KmLlmTextModel for ForbiddenModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.called.set(true);
        Err(KmLlmClientError::Parse(
            "model must not be consulted under partial source coverage".to_string(),
        ))
    }
}

// ADR Bounded Authoring Contract: a model-derived draft is accepted only when
// every requested source is fully represented in the bounded prompt. A source
// larger than the aggregate cap is truncated into the prompt, so the model must
// not be consulted at all and the deterministic scaffold is what lands on disk.
#[test]
fn extract_falls_back_when_any_single_source_exceeds_cap() {
    let tempdir = tempdir().expect("temp dir");
    let project_root = tempdir.path().to_path_buf();
    fs::create_dir_all(project_root.join("docs")).expect("docs dir");
    fs::write(
        project_root.join("docs/source.md"),
        "s".repeat(MAX_MODEL_PROMPT_BYTES + 4096),
    )
    .expect("oversized source");
    let output_dir = project_root.join("_draft");
    let config = enabled_local_config();
    let model = ForbiddenModel {
        called: std::cell::Cell::new(false),
    };

    run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: project_root.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert!(
        !model.called.get(),
        "partial coverage must skip the model call entirely"
    );
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(
        facts.contains("draft_source_reference"),
        "the deterministic fallback wrote the package"
    );

    let prompt = assemble_prompt_text(
        &[ParsedSource {
            raw: "silver:docs/source.md".to_string(),
            relative_path: "docs/source.md".to_string(),
        }],
        &fs::canonicalize(&project_root).expect("canonical root"),
    )
    .expect("prompt assembles");
    assert!(
        !prompt.covers_all_sources,
        "an oversized source is not fully represented"
    );
}

// The complement: a small source is fully represented, so the model path stays
// reachable and its draft is what lands on disk.
#[test]
fn extract_uses_model_draft_when_every_source_is_fully_represented() {
    let tempdir = tempdir().expect("temp dir");
    let project_root = tempdir.path().to_path_buf();
    fs::create_dir_all(project_root.join("docs")).expect("docs dir");
    fs::write(project_root.join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = project_root.join("_draft");
    let config = enabled_local_config();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/source.md", "Fully covered claim.")],
            mappings: vec![model_mapping("silver:docs/source.md")],
        }),
    };

    run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: project_root.clone(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Fully covered claim."));

    let prompt = assemble_prompt_text(
        &[ParsedSource {
            raw: "silver:docs/source.md".to_string(),
            relative_path: "docs/source.md".to_string(),
        }],
        &fs::canonicalize(&project_root).expect("canonical root"),
    )
    .expect("prompt assembles");
    assert!(prompt.covers_all_sources);
}

// The coverage reserve must track the client's real instruction header instead
// of restating its length. If the instruction is reworded, this stays exact and
// no coordinated edit is needed.
#[allow(clippy::assertions_on_constants)]
#[test]
fn instruction_headroom_is_derived_from_the_client_instruction() {
    assert_eq!(
        MODEL_INSTRUCTION_HEADROOM_BYTES,
        MODEL_EXTRACT_INSTRUCTION.len()
    );
    assert!(MODEL_INSTRUCTION_HEADROOM_BYTES > 0);
}

#[test]
fn extractor_keeps_valid_model_rows_when_one_row_is_invalid() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let mut invalid = model_fact("silver:docs/source.md", "invalid model claim");
    invalid.id.clear();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![
                model_fact("silver:docs/source.md", "model derived claim"),
                invalid,
            ],
            mappings: Vec::new(),
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.facts, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("model derived claim"));
    assert!(!facts.contains("invalid model claim"));
    assert!(!facts.contains("Draft extractor selected"));
}

#[test]
fn extractor_falls_back_when_a_model_mapping_is_invalid() {
    let root = tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("docs")).expect("docs dir");
    fs::write(root.path().join("docs/source.md"), "# Source\n").expect("source");
    let output_dir = root.path().join("_draft");

    let config = enabled_local_config();
    let mut invalid = model_mapping("silver:docs/source.md");
    invalid.relation.clear();
    let model = StubModel {
        draft: Ok(KmLlmExtractDraft {
            facts: vec![model_fact("silver:docs/source.md", "model derived claim")],
            mappings: vec![invalid],
        }),
    };

    let result = run_km_llm_extract(
        &KmLlmExtractRequest {
            sources: vec!["silver:docs/source.md".to_string()],
            output_dir: output_dir.clone(),
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("extract");

    assert_eq!(result.mappings, 1);
    let facts = fs::read_to_string(output_dir.join("facts.jsonl")).expect("facts");
    assert!(facts.contains("Draft extractor selected"));
    assert!(!facts.contains("model derived claim"));
}
