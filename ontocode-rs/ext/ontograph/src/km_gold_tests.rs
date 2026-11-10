use pretty_assertions::assert_eq;

use super::*;

#[test]
fn builds_deterministic_gold_from_active_imports_and_excludes_obsolete_versions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo_root = dir.path().join("repo");
    let km_store_root = dir.path().join("km-store");
    std::fs::create_dir_all(&repo_root).expect("repo root");

    let legacy = dir.path().join("legacy-km");
    let active = dir.path().join("active-km");
    write_sample_km(&legacy, "demo/graph", "0.1.0", "legacy-fact", false);
    write_sample_km(&active, "demo/graph", "0.2.0", "active-fact", true);

    let store = KmStore::new(&km_store_root);
    store.import(&legacy).expect("import legacy");
    store.import(&active).expect("import active");

    let request = sample_request(&repo_root, &km_store_root, "km-gen-1");
    let first = build_active_km_gold(request.clone()).expect("build gold");
    let second = build_active_km_gold(request).expect("rebuild gold");

    assert_eq!(first, second);
    assert_eq!(first.modules.len(), 1);
    assert_eq!(first.modules[0].id, "demo/graph");
    assert_eq!(first.modules[0].version, "0.2.0");
    assert_eq!(first.modules[0].facts[0].id, "active-fact");

    let loaded = KmGoldStore::load_active_generation(&OntographArtifactRoot::new(&repo_root))
        .expect("load active gold");
    assert_eq!(
        loaded.module_summaries(QueryBudget::default()).items,
        vec![KmGoldModuleSummary {
            id: "demo/graph".to_string(),
            version: "0.2.0".to_string(),
            kind: "codebase-graph".to_string(),
            facts: 1,
            sources: 1,
            mappings: 1,
        }]
    );
}

#[test]
fn reads_fact_source_and_mapping_summaries_from_active_gold_generation() {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo_root = dir.path().join("repo");
    let km_store_root = dir.path().join("km-store");
    std::fs::create_dir_all(&repo_root).expect("repo root");

    let active = dir.path().join("active-km");
    write_sample_km(&active, "demo/graph", "0.2.0", "active-fact", true);

    let store = KmStore::new(&km_store_root);
    store.import(&active).expect("import active");

    build_active_km_gold(sample_request(&repo_root, &km_store_root, "km-gen-2"))
        .expect("build gold");

    let loaded = KmGoldStore::load_active_generation(&OntographArtifactRoot::new(&repo_root))
        .expect("load active gold");

    assert_eq!(
        loaded
            .fact_summaries("demo/graph", QueryBudget::default())
            .items,
        vec![KmGoldFactSummary {
            module_id: "demo/graph".to_string(),
            fact_id: "active-fact".to_string(),
            kind: "architecture_rule".to_string(),
            source: "silver:test-source".to_string(),
            confidence: 0.95,
        }]
    );
    assert_eq!(
        loaded
            .source_summaries("demo/graph", QueryBudget::default())
            .items,
        vec![KmGoldSourceSummary {
            module_id: "demo/graph".to_string(),
            source_id: "source-001".to_string(),
            kind: "test_source".to_string(),
            path: "tmp/test-source.md".to_string(),
            role: "primary".to_string(),
        }]
    );
    assert_eq!(
        loaded
            .mapping_summaries("demo/graph", QueryBudget::default())
            .items,
        vec![KmGoldMappingSummary {
            module_id: "demo/graph".to_string(),
            mapping_id: "mapping-001".to_string(),
            kind: "conflict_record".to_string(),
            relation: "conflicts_with".to_string(),
            source_concept: "donor_runtime".to_string(),
            target_concept: "native_owner".to_string(),
        }]
    );
}

#[test]
fn gold_generation_uses_private_km_paths_without_touching_source_index_pointer() {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo_root = dir.path().join("repo");
    let km_store_root = dir.path().join("km-store");
    std::fs::create_dir_all(&repo_root).expect("repo root");

    let active = dir.path().join("active-km");
    write_sample_km(&active, "demo/graph", "0.2.0", "active-fact", true);

    let store = KmStore::new(&km_store_root);
    store.import(&active).expect("import active");

    build_active_km_gold(sample_request(&repo_root, &km_store_root, "km-gen-3"))
        .expect("build gold");

    let root = OntographArtifactRoot::new(&repo_root);
    assert_eq!(
        std::fs::read_to_string(root.km_gold_current_pointer_path()).expect("km gold pointer"),
        "km-gen-3\n"
    );
    assert!(!root.current_pointer_path().exists());
    assert!(
        root.km_gold_generation_file_path("km-gen-3", KM_GOLD_FILE)
            .expect("gold artifact path")
            .exists()
    );
}

fn sample_request(
    repo_root: &std::path::Path,
    km_store_root: &std::path::Path,
    generation_id: &str,
) -> KmGoldBuildRequest {
    KmGoldBuildRequest {
        repo_root: repo_root.to_path_buf(),
        km_store_root: km_store_root.to_path_buf(),
        generation_id: generation_id.to_string(),
        target_head: "head-1".to_string(),
        implementation_version: "0.0.0-test".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    }
}

fn write_sample_km(
    root: &std::path::Path,
    id: &str,
    version: &str,
    fact_id: &str,
    include_recipe: bool,
) {
    std::fs::create_dir_all(root.join("examples")).expect("create examples");
    std::fs::create_dir_all(root.join("tests")).expect("create tests");
    std::fs::write(
        root.join("km.toml"),
        format!(
            "id = \"{id}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"Test KM\"\nsources = [\"silver:test-source\"]\nlicenses = [\"MIT\"]\n\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .expect("write manifest");
    std::fs::write(
        root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"{fact_id}\",\"kind\":\"architecture_rule\",\"claim\":\"gold fact\",\"source\":\"silver:test-source\",\"confidence\":0.95}}\n"
        ),
    )
    .expect("write facts");
    std::fs::write(root.join("examples/README.md"), "example\n").expect("write example");
    std::fs::write(root.join("tests/README.md"), "test\n").expect("write test");
    std::fs::write(
        root.join("sources.jsonl"),
        "{\"id\":\"source-001\",\"kind\":\"test_source\",\"path\":\"tmp/test-source.md\",\"role\":\"primary\"}\n",
    )
    .expect("write sources");
    std::fs::write(
        root.join("mappings.jsonl"),
        "{\"id\":\"mapping-001\",\"kind\":\"conflict_record\",\"source_concept\":\"donor_runtime\",\"target_concept\":\"native_owner\",\"relation\":\"conflicts_with\",\"source\":\"silver:test-source\",\"confidence\":0.99,\"resolution\":\"reject donor runtime\"}\n",
    )
    .expect("write mappings");
    if include_recipe {
        std::fs::write(
            root.join("recipe.json"),
            "{\n  \"id\": \"recipe-001\",\n  \"kind\": \"km_composition_recipe\",\n  \"summary\": \"recipe\",\n  \"composition\": {\n    \"generated_outputs\": [\"facts.jsonl\", \"mappings.jsonl\"],\n    \"conflicts\": [{\"id\": \"conflict-001\"}]\n  }\n}\n",
        )
        .expect("write recipe");
    }
}

// Backlog 95: withdrawal marks a record obsolete, so gold generation drops it.
// Status alone was covered before; this pins the withdrawal path explicitly.
#[test]
fn gold_generation_excludes_withdrawn_records() {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo_root = dir.path().join("repo");
    let km_store_root = dir.path().join("km-store");
    std::fs::create_dir_all(&repo_root).expect("repo root");

    let kept = dir.path().join("kept-km");
    let withdrawn = dir.path().join("withdrawn-km");
    write_sample_km(&kept, "demo/kept", "0.1.0", "kept-fact", true);
    write_sample_km(
        &withdrawn,
        "demo/withdrawn",
        "0.1.0",
        "withdrawn-fact",
        true,
    );

    let store = KmStore::new(&km_store_root);
    store.import(&kept).expect("import kept");
    store.import(&withdrawn).expect("import withdrawn");
    store
        .withdraw("demo/withdrawn", "0.1.0", "retracted".to_string())
        .expect("withdraw");

    let gold = build_active_km_gold(sample_request(&repo_root, &km_store_root, "km-gen-1"))
        .expect("build gold");

    assert_eq!(gold.modules.len(), 1);
    assert_eq!(gold.modules[0].id, "demo/kept");
}

// Backlog 96: after a version supersession, regenerated gold carries only the
// new version's facts.
#[test]
fn gold_regeneration_after_version_supersession_drops_old_facts() {
    let dir = tempfile::tempdir().expect("temp dir");
    let repo_root = dir.path().join("repo");
    let km_store_root = dir.path().join("km-store");
    std::fs::create_dir_all(&repo_root).expect("repo root");

    let first = dir.path().join("first-km");
    write_sample_km(&first, "demo/graph", "0.1.0", "old-fact", true);
    let store = KmStore::new(&km_store_root);
    store.import(&first).expect("import first");

    let before = build_active_km_gold(sample_request(&repo_root, &km_store_root, "km-gen-1"))
        .expect("build gold");
    assert_eq!(before.modules[0].facts[0].id, "old-fact");

    let second = dir.path().join("second-km");
    write_sample_km(&second, "demo/graph", "0.2.0", "new-fact", true);
    store.import(&second).expect("import second");

    let after = build_active_km_gold(sample_request(&repo_root, &km_store_root, "km-gen-2"))
        .expect("rebuild gold");

    assert_eq!(after.modules.len(), 1);
    assert_eq!(after.modules[0].version, "0.2.0");
    assert_eq!(after.modules[0].facts[0].id, "new-fact");
    assert!(
        !after
            .modules
            .iter()
            .flat_map(|module| module.facts.iter())
            .any(|fact| fact.id == "old-fact"),
        "superseded facts must not survive regeneration"
    );
}

#[test]
fn active_gold_rejects_oversized_pointer_and_artifact() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = OntographArtifactRoot::new(dir.path());
    std::fs::create_dir_all(root.km_gold_root_path()).expect("gold root");

    std::fs::write(
        root.km_gold_current_pointer_path(),
        "g".repeat(MAX_KM_GOLD_POINTER_BYTES + 1),
    )
    .expect("oversized pointer");
    assert_eq!(
        KmGoldStore::load_active_generation(&root).expect_err("pointer must be bounded"),
        KmGoldLoadError::InvalidCurrentPointer
    );

    std::fs::write(root.km_gold_current_pointer_path(), "g1\n").expect("pointer");
    let generation_dir = root.km_gold_generation_dir("g1").expect("generation path");
    std::fs::create_dir_all(&generation_dir).expect("generation dir");
    std::fs::write(
        root.km_gold_generation_file_path("g1", KM_GOLD_FILE)
            .expect("artifact path"),
        vec![b' '; MAX_KM_GOLD_ARTIFACT_BYTES + 1],
    )
    .expect("oversized artifact");
    assert_eq!(
        KmGoldStore::load_active_generation(&root).expect_err("artifact must be bounded"),
        KmGoldLoadError::ReadArtifact
    );
}

#[test]
fn gold_optional_metadata_rejects_oversized_jsonl_and_recipe() {
    let dir = tempfile::tempdir().expect("temp dir");
    let oversized = vec![b' '; MAX_KM_GOLD_OPTIONAL_JSON_BYTES + 1];
    let jsonl_path = dir.path().join("sources.jsonl");
    let recipe_path = dir.path().join("recipe.json");
    std::fs::write(&jsonl_path, &oversized).expect("oversized jsonl");
    std::fs::write(&recipe_path, oversized).expect("oversized recipe");

    assert_eq!(
        load_optional_jsonl::<KmGoldSource>(&jsonl_path)
            .expect_err("jsonl must be bounded")
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        load_optional_recipe(&recipe_path)
            .expect_err("recipe must be bounded")
            .kind(),
        std::io::ErrorKind::InvalidData
    );
}

#[test]
fn gold_file_tree_rejects_entry_depth_and_byte_overflow() {
    let dir = tempfile::tempdir().expect("temp dir");

    let entry_root = dir.path().join("entry-limit/examples");
    std::fs::create_dir_all(&entry_root).expect("entry root");
    for index in 0..=MAX_KM_GOLD_FILE_ENTRIES {
        std::fs::write(entry_root.join(format!("file-{index}")), b"").expect("entry-limit file");
    }
    assert!(collect_relative_files(&dir.path().join("entry-limit"), "examples").is_err());

    let depth_root = dir.path().join("depth-limit");
    let mut nested = depth_root.join("examples");
    for index in 0..=MAX_KM_GOLD_FILE_DEPTH {
        nested.push(format!("level-{index}"));
    }
    std::fs::create_dir_all(&nested).expect("deep tree");
    std::fs::write(nested.join("file"), b"x").expect("deep file");
    assert!(collect_relative_files(&depth_root, "examples").is_err());

    let bytes_root = dir.path().join("byte-limit");
    let bytes_dir = bytes_root.join("examples");
    std::fs::create_dir_all(&bytes_dir).expect("byte root");
    std::fs::write(
        bytes_dir.join("large"),
        vec![b'x'; (MAX_KM_GOLD_REFERENCED_BYTES + 1) as usize],
    )
    .expect("large referenced file");
    assert!(collect_relative_files(&bytes_root, "examples").is_err());
}
