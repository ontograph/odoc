use std::cell::Cell;
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::KmLlmRecipeError;
use super::KmLlmRecipeRequest;
use super::run_km_llm_recipe;
use crate::indexer::km_gold::KmGoldBuildRequest;
use crate::indexer::km_gold::build_active_km_gold;
use crate::km_import::run_km_import;
use crate::km_llm_client::KmLlmClientConfig;
use crate::km_llm_client::KmLlmClientError;
use crate::km_llm_client::KmLlmModelContext;
use crate::km_llm_client::KmLlmTextModel;

#[test]
fn recipe_author_writes_gold_loadable_recipe_metadata() {
    let root = tempdir().expect("tempdir");
    let package = root.path().join("package");
    write_km(&package);

    let result = run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: package.clone(),
            generated_outputs: vec!["facts.jsonl".to_string(), "mappings.jsonl".to_string()],
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .expect("recipe");

    assert_eq!(result.generated_outputs, 2);
    assert_eq!(result.conflicts, 1);
    let store_root = root.path().join("_store");
    run_km_import(&package, &store_root).expect("import");
    let gold = build_active_km_gold(KmGoldBuildRequest {
        repo_root: root.path().to_path_buf(),
        km_store_root: store_root,
        generation_id: "test-generation".to_string(),
        target_head: "HEAD".to_string(),
        implementation_version: "test".to_string(),
        timestamp_unix_seconds: 1,
    })
    .expect("gold");

    let recipe = gold.modules[0].recipe.as_ref().expect("recipe in gold");
    assert_eq!(
        recipe.generated_outputs,
        vec!["facts.jsonl", "mappings.jsonl"]
    );
    assert_eq!(recipe.conflict_ids, vec!["conflict-1"]);
}

#[test]
fn recipe_author_requires_existing_package() {
    let root = tempdir().expect("tempdir");

    let err = run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: root.path().join("missing"),
            generated_outputs: Vec::new(),
            project_root: root.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmRecipeError::MissingPackage);
}

#[test]
fn recipe_author_rejects_package_outside_project_root() {
    let project = tempdir().expect("project tempdir");
    let external = tempdir().expect("external tempdir");
    let package = external.path().join("package");
    write_km(&package);

    let err = run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: package.clone(),
            generated_outputs: Vec::new(),
            project_root: project.path().to_path_buf(),
        },
        None,
    )
    .unwrap_err();

    assert_eq!(err, KmLlmRecipeError::PackageOutsideProjectRoot);
    assert!(!package.join("recipe.json").exists());
}

struct StubModel {
    summary: Result<String, KmLlmClientError>,
    calls: Cell<usize>,
}

impl KmLlmTextModel for StubModel {
    fn complete(&self, _prompt_text: &str) -> Result<String, KmLlmClientError> {
        self.calls.set(self.calls.get() + 1);
        self.summary.clone()
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
fn recipe_author_uses_model_summary_when_enabled() {
    let root = tempdir().expect("tempdir");
    let package = root.path().join("package");
    write_km(&package);
    let config = enabled_local_config();
    let model = StubModel {
        summary: Ok("Model-authored recipe summary.".to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: package.clone(),
            generated_outputs: vec!["facts.jsonl".to_string(), "mappings.jsonl".to_string()],
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("recipe");

    assert_eq!(model.calls.get(), 1);
    let recipe = fs::read_to_string(package.join("recipe.json")).expect("recipe");
    assert!(recipe.contains("Model-authored recipe summary."));
}

#[test]
fn recipe_author_falls_back_to_deterministic_summary_when_disabled() {
    let root = tempdir().expect("tempdir");
    let package = root.path().join("package");
    write_km(&package);
    let mut config = enabled_local_config();
    config.enabled = false;
    let model = StubModel {
        summary: Ok("Model-authored recipe summary.".to_string()),
        calls: Cell::new(0),
    };

    run_km_llm_recipe(
        &KmLlmRecipeRequest {
            package_dir: package.clone(),
            generated_outputs: vec!["facts.jsonl".to_string(), "mappings.jsonl".to_string()],
            project_root: root.path().to_path_buf(),
        },
        Some(KmLlmModelContext {
            config: &config,
            model: &model,
        }),
    )
    .expect("recipe");

    assert_eq!(model.calls.get(), 0);
    let recipe = fs::read_to_string(package.join("recipe.json")).expect("recipe");
    assert!(recipe.contains("File-only recipe metadata for codebase-graph/recipe-test@0.1.0."));
    assert!(!recipe.contains("Model-authored recipe summary."));
}

fn write_km(root: &Path) {
    fs::create_dir_all(root.join("examples")).expect("examples");
    fs::create_dir_all(root.join("tests")).expect("tests");
    fs::write(root.join("examples/README.md"), "# Example\n").expect("example");
    fs::write(root.join("tests/README.md"), "# Test\n").expect("test");
    fs::write(
        root.join("km.toml"),
        "id = \"codebase-graph/recipe-test\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"Test KM package.\"\nsources = [\"silver:test.md\"]\nlicenses = [\"MIT\"]\n\n[provenance]\ncreated_by = \"test\"\ngenerated_at = \"2026-07-06\"\nsource_commit = \"HEAD\"\n",
    )
    .expect("manifest");
    fs::write(
        root.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"graph_pattern\",\"claim\":\"A graph fact.\",\"source\":\"silver:test.md\",\"confidence\":0.9}\n",
    )
    .expect("facts");
    fs::write(
        root.join("mappings.jsonl"),
        "{\"id\":\"conflict-1\",\"kind\":\"conflict_record\",\"source_concept\":\"a\",\"target_concept\":\"b\",\"relation\":\"conflicts_with\",\"source\":\"silver:test.md\",\"confidence\":0.8,\"resolution\":\"Keep local.\"}\n",
    )
    .expect("mappings");
}
