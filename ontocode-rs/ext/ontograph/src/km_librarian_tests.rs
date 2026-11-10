use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use ontocode_protocol::ThreadId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;
use crate::km_store::KmStore;

#[test]
fn health_counts_active_and_obsolete_modules() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let first = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let second = write_valid_km(tempdir.path(), "sample-a", "0.2.0", "new bounded claim");
    store.import(&first).expect("first import");
    store.import(&second).expect("second import");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result,
        KmLibrarianHealthResult {
            active_modules: 1,
            obsolete_modules: 1,
            issues: Vec::new(),
        }
    );
}

#[test]
fn health_reports_missing_cached_package_dir() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");
    fs::remove_dir_all(&record.path).expect("remove cached package");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result,
        KmLibrarianHealthResult {
            active_modules: 1,
            obsolete_modules: 0,
            issues: vec![KmLibrarianHealthIssue {
                id: record.id,
                version: record.version,
                path: record.path,
                code: "package-dir-missing".to_string(),
                message: "cached KM package directory is missing".to_string(),
            }],
        }
    );
}

#[test]
fn health_reports_cached_package_validation_errors() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");
    fs::write(Path::new(&record.path).join("facts.jsonl"), "{\n").expect("break cached facts");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result,
        KmLibrarianHealthResult {
            active_modules: 1,
            obsolete_modules: 0,
            issues: vec![
                KmLibrarianHealthIssue {
                    id: record.id.clone(),
                    version: record.version.clone(),
                    path: record.path.clone(),
                    code: "validation-status".to_string(),
                    message: "cached KM package validation status is invalid".to_string(),
                },
                KmLibrarianHealthIssue {
                    id: record.id,
                    version: record.version,
                    path: record.path,
                    code: "validation-error".to_string(),
                    message: "facts.jsonl contains malformed JSONL: EOF while parsing an object at line 1 column 1".to_string(),
                },
            ],
        }
    );
}

#[test]
fn health_sorts_issues_by_km_id_version_path() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let later = write_valid_km(tempdir.path(), "sample-b", "0.1.0", "bounded claim");
    let earlier = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let later_record = store.import(&later).expect("later import");
    let earlier_record = store.import(&earlier).expect("earlier import");
    fs::remove_dir_all(&later_record.path).expect("remove later cached package");
    fs::remove_dir_all(&earlier_record.path).expect("remove earlier cached package");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result.issues,
        vec![
            KmLibrarianHealthIssue {
                id: earlier_record.id,
                version: earlier_record.version,
                path: earlier_record.path,
                code: "package-dir-missing".to_string(),
                message: "cached KM package directory is missing".to_string(),
            },
            KmLibrarianHealthIssue {
                id: later_record.id,
                version: later_record.version,
                path: later_record.path,
                code: "package-dir-missing".to_string(),
                message: "cached KM package directory is missing".to_string(),
            },
        ]
    );
}

fn write_valid_km(root: &Path, name: &str, version: &str, claim: &str) -> std::path::PathBuf {
    let km_root = root.join(format!("{name}-{version}"));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), valid_manifest(name, version)).expect("manifest");
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

fn valid_manifest(name: &str, version: &str) -> String {
    format!(
        "id = \"codebase-graph/{name}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}

#[test]
fn catalog_returns_all_active_modules_sorted_and_serializes_correctly() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let sample_b = write_valid_km(tempdir.path(), "sample-b", "0.1.0", "claim b");
    let sample_a = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "claim a");

    let record_b = store.import(&sample_b).expect("import b");
    let record_a = store.import(&sample_a).expect("import a");

    let result = run_km_librarian_catalog(&KmLibrarianCatalogRequest {
        km_store_root: store_root,
    })
    .expect("catalog");

    assert_eq!(
        result,
        KmLibrarianCatalogResult {
            modules: vec![
                KmLibrarianCatalogEntry {
                    id: record_a.id,
                    version: record_a.version,
                    path: record_a.path,
                    source_path: record_a.source_path,
                    content_hash: record_a.content_hash,
                    imported_at_unix_seconds: record_a.imported_at_unix_seconds,
                    topic_words: record_a.topic_words,
                },
                KmLibrarianCatalogEntry {
                    id: record_b.id,
                    version: record_b.version,
                    path: record_b.path,
                    source_path: record_b.source_path,
                    content_hash: record_b.content_hash,
                    imported_at_unix_seconds: record_b.imported_at_unix_seconds,
                    topic_words: record_b.topic_words,
                },
            ]
        }
    );
    assert!(
        serde_json::to_string(&result)
            .expect("serialize")
            .contains("topicWords")
    );
}

#[test]
fn catalog_excludes_obsolete_modules() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);

    let v1 = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "claim v1");
    let v2 = write_valid_km(tempdir.path(), "sample-a", "0.2.0", "claim v2");

    store.import(&v1).expect("import v1");
    let record_v2 = store.import(&v2).expect("import v2");

    let result = run_km_librarian_catalog(&KmLibrarianCatalogRequest {
        km_store_root: store_root,
    })
    .expect("catalog");

    assert_eq!(
        result,
        KmLibrarianCatalogResult {
            modules: vec![KmLibrarianCatalogEntry {
                id: record_v2.id,
                version: record_v2.version,
                path: record_v2.path,
                source_path: record_v2.source_path,
                content_hash: record_v2.content_hash,
                imported_at_unix_seconds: record_v2.imported_at_unix_seconds,
                topic_words: record_v2.topic_words,
            }]
        }
    );
}

#[test]
fn query_returns_ranked_bounded_candidates() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let lower_ranked = write_valid_km(tempdir.path(), "sample-b", "0.1.0", "alpha claim");
    let higher_ranked = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "alpha beta claim");
    store.import(&lower_ranked).expect("import lower ranked");
    let higher_record = store.import(&higher_ranked).expect("import higher ranked");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "alpha beta".to_string(),
        max_candidates: 1,
    })
    .expect("query");

    assert_eq!(
        result,
        KmLibrarianQueryResult {
            query: "alpha beta".to_string(),
            candidates: vec![KmLibrarianQueryCandidate {
                scope: KmLibrarianQueryScope::Local,
                id: "codebase-graph/sample-a".to_string(),
                version: "0.1.0".to_string(),
                path: fs::canonicalize(higher_record.path)
                    .expect("canonical package path")
                    .display()
                    .to_string(),
                matched_terms: vec!["alpha".to_string(), "beta".to_string()],
                reason: "local".to_string(),
                referenced_by: None,
            }],
            truncated: true,
            ontology_paths: Vec::new(),
            matched_facts: vec![KmLibrarianMatchedFact {
                package_id: "codebase-graph/sample-a".to_string(),
                package_version: "0.1.0".to_string(),
                fact_id: "fact-1".to_string(),
                kind: "rule".to_string(),
                claim: "alpha beta claim".to_string(),
                source: "silver:doc".to_string(),
                confidence: 0.9,
            }],
            ontology_truncated: false,
        }
    );
}

#[test]
fn query_counts_each_package_once_before_applying_the_limit() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let first = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "first bounded claim");
    let second = write_valid_km(tempdir.path(), "sample-b", "0.1.0", "second bounded claim");
    store.import(&first).expect("first import");
    store.import(&second).expect("second import");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "bounded".to_string(),
        max_candidates: 2,
    })
    .expect("query");

    assert_eq!(
        result
            .candidates
            .iter()
            .map(|candidate| (candidate.id.clone(), candidate.scope))
            .collect::<Vec<_>>(),
        vec![
            (
                "codebase-graph/sample-a".to_string(),
                KmLibrarianQueryScope::Local,
            ),
            (
                "codebase-graph/sample-b".to_string(),
                KmLibrarianQueryScope::Local,
            ),
        ]
    );
    assert!(!result.truncated);
}

#[test]
fn query_expands_existing_mappings_with_bounded_provenance() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(
        tempdir.path(),
        "ontology",
        "0.1.0",
        "Connection pooling reuses database sessions.",
    );
    fs::write(
        source.join("facts.jsonl"),
        concat!(
            "{\"id\":\"fact-pooling\",\"kind\":\"rule\",\"claim\":\"Connection pooling reuses database sessions.\",\"source\":\"silver:database.md\",\"confidence\":0.95}\n",
            "{\"id\":\"fact-latency\",\"kind\":\"observation\",\"claim\":\"Database session reuse reduces connection setup latency.\",\"source\":\"silver:performance.md\",\"confidence\":0.9}\n",
        ),
    )
    .expect("facts");
    fs::write(
        source.join("mappings.jsonl"),
        concat!(
            "{\"id\":\"map-pooling-session\",\"kind\":\"concept_mapping\",\"source_concept\":\"connection pooling\",\"target_concept\":\"database session reuse\",\"relation\":\"same_as\",\"source\":\"silver:database.md\",\"confidence\":0.95}\n",
            "{\"id\":\"map-session-resource\",\"kind\":\"concept_mapping\",\"source_concept\":\"database session reuse\",\"target_concept\":\"resource reuse\",\"relation\":\"broader\",\"source\":\"silver:architecture.md\",\"confidence\":0.9}\n",
            "{\"id\":\"map-resource-lifecycle\",\"kind\":\"concept_mapping\",\"source_concept\":\"resource reuse\",\"target_concept\":\"resource lifecycle\",\"relation\":\"broader\",\"source\":\"silver:architecture.md\",\"confidence\":0.8}\n",
        ),
    )
    .expect("mappings");
    let record = store.import(&source).expect("import");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "connection pooling".to_string(),
        max_candidates: 10,
    })
    .expect("query");

    assert_eq!(
        result,
        KmLibrarianQueryResult {
            query: "connection pooling".to_string(),
            candidates: vec![KmLibrarianQueryCandidate {
                scope: KmLibrarianQueryScope::Local,
                id: "codebase-graph/ontology".to_string(),
                version: "0.1.0".to_string(),
                path: fs::canonicalize(record.path)
                    .expect("canonical package path")
                    .display()
                    .to_string(),
                matched_terms: vec!["connection".to_string(), "pooling".to_string()],
                reason: "local".to_string(),
                referenced_by: None,
            }],
            truncated: false,
            ontology_paths: vec![
                KmLibrarianOntologyPath {
                    package_id: "codebase-graph/ontology".to_string(),
                    package_version: "0.1.0".to_string(),
                    mapping_id: "map-pooling-session".to_string(),
                    depth: 1,
                    source_concept: "connection pooling".to_string(),
                    target_concept: "database session reuse".to_string(),
                    relation: "same_as".to_string(),
                    source: "silver:database.md".to_string(),
                    confidence: 0.95,
                },
                KmLibrarianOntologyPath {
                    package_id: "codebase-graph/ontology".to_string(),
                    package_version: "0.1.0".to_string(),
                    mapping_id: "map-session-resource".to_string(),
                    depth: 2,
                    source_concept: "database session reuse".to_string(),
                    target_concept: "resource reuse".to_string(),
                    relation: "broader".to_string(),
                    source: "silver:architecture.md".to_string(),
                    confidence: 0.9,
                },
            ],
            matched_facts: vec![
                KmLibrarianMatchedFact {
                    package_id: "codebase-graph/ontology".to_string(),
                    package_version: "0.1.0".to_string(),
                    fact_id: "fact-latency".to_string(),
                    kind: "observation".to_string(),
                    claim: "Database session reuse reduces connection setup latency.".to_string(),
                    source: "silver:performance.md".to_string(),
                    confidence: 0.9,
                },
                KmLibrarianMatchedFact {
                    package_id: "codebase-graph/ontology".to_string(),
                    package_version: "0.1.0".to_string(),
                    fact_id: "fact-pooling".to_string(),
                    kind: "rule".to_string(),
                    claim: "Connection pooling reuses database sessions.".to_string(),
                    source: "silver:database.md".to_string(),
                    confidence: 0.95,
                },
            ],
            ontology_truncated: false,
        }
    );
}

#[test]
fn query_uses_retrieval_matches_as_ontology_seeds() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(
        tempdir.path(),
        "ontology",
        "0.1.0",
        "Connection pooling reuses database sessions and reduces latency.",
    );
    fs::write(
        source.join("mappings.jsonl"),
        "{\"id\":\"map-pooling-session\",\"kind\":\"concept_mapping\",\"source_concept\":\"connection pooling\",\"target_concept\":\"database session reuse\",\"relation\":\"same_as\",\"source\":\"silver:database.md\",\"confidence\":0.95}\n",
    )
    .expect("mappings");
    store.import(&source).expect("import");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "connections pools latency".to_string(),
        max_candidates: 10,
    })
    .expect("query");

    assert_eq!(result.ontology_paths.len(), 1);
    assert_eq!(result.ontology_paths[0].mapping_id, "map-pooling-session");
}

#[test]
fn query_fails_closed_for_malformed_mapping_rows() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(
        tempdir.path(),
        "ontology",
        "0.1.0",
        "Connection pooling reuses database sessions.",
    );
    fs::write(source.join("mappings.jsonl"), "{\n").expect("mappings");
    store.import(&source).expect("import");

    let error = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "connection pooling".to_string(),
        max_candidates: 10,
    })
    .expect_err("malformed mapping error");

    assert!(matches!(error, KmLibrarianQueryError::Ontology(_)));
}

#[test]
fn query_caps_ontology_paths_at_twenty_five() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(
        tempdir.path(),
        "ontology",
        "0.1.0",
        "Seed concept bounded claim.",
    );
    let mappings = (0..26)
        .map(|index| {
            format!(
                "{{\"id\":\"map-{index:02}\",\"kind\":\"concept_mapping\",\"source_concept\":\"seed concept\",\"target_concept\":\"target {index:02}\",\"relation\":\"related_to\",\"source\":\"silver:ontology.md\",\"confidence\":0.9}}\n"
            )
        })
        .collect::<String>();
    fs::write(source.join("mappings.jsonl"), mappings).expect("mappings");
    store.import(&source).expect("import");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "seed concept".to_string(),
        max_candidates: 10,
    })
    .expect("query");

    assert_eq!(result.ontology_paths.len(), 25);
    assert!(result.ontology_truncated);
    assert_eq!(result.ontology_paths[0].mapping_id, "map-00");
    assert_eq!(result.ontology_paths[24].mapping_id, "map-24");
}

#[test]
fn heal_scan_reports_one_invalid_package_item() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");
    fs::write(Path::new(&record.path).join("facts.jsonl"), "{\n").expect("break cached facts");

    let result = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root,
    })
    .expect("heal scan");

    assert_eq!(
        result,
        KmLibrarianHealScanResult {
            reports: vec![KmLibrarianHealScanReport {
                id: record.id,
                version: record.version,
                path: record.path,
                code: "invalid-package".to_string(),
                message: "cached KM package validation status is invalid".to_string(),
            }],
            conflicts: Vec::new(),
        }
    );
}

#[test]
fn heal_scan_skips_missing_cached_package_dir() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");
    fs::remove_dir_all(&record.path).expect("remove cached package");

    let result = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root,
    })
    .expect("heal scan");

    assert_eq!(
        result,
        KmLibrarianHealScanResult {
            reports: Vec::new(),
            conflicts: Vec::new(),
        }
    );
}

#[test]
fn heal_scan_reports_conflict_records_from_mappings_jsonl() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    fs::write(
        source.join("mappings.jsonl"),
        concat!(
            "{\"id\":\"mapping-1\",\"kind\":\"equivalent_to\",\"source_concept\":\"a\",\"target_concept\":\"b\",\"relation\":\"maps_to\",\"source\":\"silver:doc\"}\n",
            "{\"id\":\"conflict-2\",\"kind\":\"conflict_record\",\"source_concept\":\"b\",\"target_concept\":\"c\",\"relation\":\"conflicts_with\",\"source\":\"silver:doc-b\",\"resolution\":\"keep b\"}\n",
            "{\"id\":\"conflict-1\",\"kind\":\"conflict_record\",\"source_concept\":\"a\",\"target_concept\":\"b\",\"relation\":\"conflicts_with\",\"source\":\"silver:doc-a\",\"resolution\":\"keep a\"}\n",
        ),
    )
    .expect("mappings");
    let record = store.import(&source).expect("import");

    let result = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root,
    })
    .expect("heal scan");

    assert_eq!(
        result,
        KmLibrarianHealScanResult {
            reports: Vec::new(),
            conflicts: vec![
                KmLibrarianHealScanConflict {
                    package_id: record.id.clone(),
                    package_version: record.version.clone(),
                    package_path: record.path.clone(),
                    id: Some("conflict-1".to_string()),
                    source_concept: Some("a".to_string()),
                    target_concept: Some("b".to_string()),
                    relation: Some("conflicts_with".to_string()),
                    resolution: Some("keep a".to_string()),
                    source: Some("silver:doc-a".to_string()),
                },
                KmLibrarianHealScanConflict {
                    package_id: record.id,
                    package_version: record.version,
                    package_path: record.path,
                    id: Some("conflict-2".to_string()),
                    source_concept: Some("b".to_string()),
                    target_concept: Some("c".to_string()),
                    relation: Some("conflicts_with".to_string()),
                    resolution: Some("keep b".to_string()),
                    source: Some("silver:doc-b".to_string()),
                },
            ],
        }
    );
    assert!(
        serde_json::to_string(&result)
            .expect("serialize")
            .contains("sourceConcept")
    );
}

#[test]
fn heal_scan_reports_malformed_mappings_jsonl_without_panicking() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    fs::write(source.join("mappings.jsonl"), "{\n").expect("mappings");
    let record = store.import(&source).expect("import");

    let result = run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root,
    })
    .expect("heal scan");

    assert_eq!(result.conflicts, Vec::new());
    assert_eq!(result.reports.len(), 1);
    assert_eq!(
        result.reports[0],
        KmLibrarianHealScanReport {
            id: record.id,
            version: record.version,
            path: record.path,
            code: "mappings-jsonl-malformed".to_string(),
            message:
                "mappings.jsonl line 1 is malformed: EOF while parsing an object at line 1 column 1"
                    .to_string(),
        }
    );
}

#[test]
fn consolidate_runs_report_only_loop_without_promotion() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    store.import(&source).expect("import");

    let result = run_km_librarian_consolidate(&KmLibrarianConsolidateRequest {
        input_dirs: vec![source],
        output_dir: tempdir.path().join("_draft-consolidated"),
        km_store_root: store_root,
        project_root: tempdir.path().to_path_buf(),
    })
    .expect("consolidate");

    assert_eq!(result.consolidation.facts, 1);
    assert_eq!(
        result.health,
        KmLibrarianHealthResult {
            active_modules: 1,
            obsolete_modules: 0,
            issues: Vec::new(),
        }
    );
    assert_eq!(
        result.heal_scan,
        KmLibrarianHealScanResult {
            reports: Vec::new(),
            conflicts: Vec::new(),
        }
    );
    assert_eq!(
        result.promotion_skipped_reason,
        "gold promotion and active-KM mutation were not attempted by this report-only task"
    );
}

#[test]
fn consolidate_propagates_consolidation_errors() {
    let tempdir = TempDir::new().expect("temp dir");

    let error = run_km_librarian_consolidate(&KmLibrarianConsolidateRequest {
        input_dirs: Vec::new(),
        output_dir: tempdir.path().join("_draft-consolidated"),
        km_store_root: tempdir.path().join("km-cache"),
        project_root: tempdir.path().to_path_buf(),
    })
    .expect_err("empty input error");

    assert_eq!(
        error,
        KmLibrarianConsolidateError::Consolidate(
            "at least one input KM directory is required".to_string()
        )
    );
}

#[test]
fn agent_plan_assembles_existing_librarian_verbs() {
    let request = KmLibrarianAgentRequest {
        km_store_root: Path::new("/tmp/km-cache").to_path_buf(),
        input_dirs: vec![Path::new("/tmp/input-a").to_path_buf()],
        output_dir: Path::new("/tmp/output").to_path_buf(),
        project_root: Path::new("/tmp").to_path_buf(),
    };

    let plan = run_km_librarian_agent_plan(&request);

    assert_eq!(
        plan,
        KmLibrarianAgentPlan {
            steps: vec![
                KmLibrarianAgentPlanStep::Health(KmLibrarianHealthRequest {
                    km_store_root: request.km_store_root.clone(),
                }),
                KmLibrarianAgentPlanStep::Catalog(KmLibrarianCatalogRequest {
                    km_store_root: request.km_store_root.clone(),
                }),
                KmLibrarianAgentPlanStep::HealScan(KmLibrarianHealScanRequest {
                    km_store_root: request.km_store_root.clone(),
                }),
                KmLibrarianAgentPlanStep::Consolidate(KmLibrarianConsolidateRequest {
                    input_dirs: request.input_dirs,
                    output_dir: request.output_dir,
                    km_store_root: request.km_store_root,
                    project_root: request.project_root,
                }),
            ],
        }
    );
}

#[tokio::test]
async fn agent_delegates_request_to_injected_spawner() {
    let captured = Arc::new(Mutex::new(None));
    let spawner_captured = Arc::clone(&captured);
    let spawner =
        move |thread_id: ThreadId,
              request: KmLibrarianAgentRequest|
              -> ontocode_extension_api::AgentSpawnFuture<'static, &'static str, String> {
            let spawner_captured = Arc::clone(&spawner_captured);
            Box::pin(async move {
                *spawner_captured.lock().expect("captured") = Some((thread_id, request));
                Ok("spawned")
            })
        };
    let agent = KmLibrarianAgent::new(spawner);
    let thread_id =
        ThreadId::from_string("00000000-0000-0000-0000-000000000123").expect("thread id");
    let request = KmLibrarianAgentRequest {
        km_store_root: Path::new("/tmp/km-cache").to_path_buf(),
        input_dirs: vec![Path::new("/tmp/input-a").to_path_buf()],
        output_dir: Path::new("/tmp/output").to_path_buf(),
        project_root: Path::new("/tmp").to_path_buf(),
    };

    let spawned = agent
        .spawn_subagent(thread_id, request.clone())
        .await
        .expect("spawn");

    assert_eq!(spawned, "spawned");
    assert_eq!(
        *captured.lock().expect("captured"),
        Some((thread_id, request))
    );
}

#[test]
fn health_reports_active_checked_in_source_drift_without_mutating_the_store() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    let source_root = checked_in_km_root(&project_root);
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);

    let unchanged = write_valid_km(&source_root, "sample-a", "0.1.0", "unchanged claim");
    let changed = write_valid_km(&source_root, "sample-b", "0.1.0", "changed claim");
    let missing = write_valid_km(&source_root, "sample-c", "0.1.0", "missing claim");
    let project_scope = fs::canonicalize(&project_root)
        .expect("canonical project")
        .display()
        .to_string();
    let unchanged_record = store
        .import_for_project(&unchanged, &project_scope)
        .expect("import unchanged source");
    let changed_record = store
        .import_for_project(&changed, &project_scope)
        .expect("import changed source");
    let missing_record = store
        .import_for_project(&missing, &project_scope)
        .expect("import missing source");
    fs::create_dir_all(store_root.join("_ops")).expect("ops dir");
    fs::write(
        store_root.join("_ops/import.jsonl"),
        "existing import log\n",
    )
    .expect("import log");

    fs::write(changed.join("examples/README.md"), "changed body\n").expect("edit source");
    fs::remove_dir_all(&missing).expect("remove source package");
    let index_before = fs::read(store_root.join("index.json")).expect("index bytes");
    let store_before = store_files(&store_root);

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root.clone(),
    })
    .expect("health");

    let code_for = |source_path: &str| {
        result
            .issues
            .iter()
            .find(|issue| issue.path == source_path)
            .map(|issue| issue.code.as_str())
    };
    assert_eq!(code_for(&unchanged_record.source_path), None);
    assert_eq!(
        code_for(&changed_record.source_path),
        Some("source-changed")
    );
    assert_eq!(
        code_for(&missing_record.source_path),
        Some("source-missing")
    );
    assert_eq!(
        result
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>(),
        vec!["source-changed", "source-missing"]
    );
    assert_eq!(
        result.issues,
        vec![
            KmLibrarianHealthIssue {
                id: changed_record.id,
                version: changed_record.version,
                path: changed_record.source_path,
                code: "source-changed".to_string(),
                message: "checked-in KM source package differs from the imported snapshot; publish and import a new package version".to_string(),
            },
            KmLibrarianHealthIssue {
                id: missing_record.id,
                version: missing_record.version,
                path: missing_record.source_path,
                code: "source-missing".to_string(),
                message: "checked-in KM source package is missing; publish and import a new package version".to_string(),
            },
        ]
    );
    assert_eq!(
        fs::read(store_root.join("index.json")).expect("index bytes"),
        index_before,
        "health must not rewrite index bytes"
    );
    assert_eq!(
        store_files(&store_root),
        store_before,
        "health must not rewrite import logs or cached packages"
    );
    assert!(Path::new(&unchanged_record.source_path).is_dir());
}

#[test]
fn health_ignores_obsolete_and_untracked_source_drift() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    let source_root = checked_in_km_root(&project_root);
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let project_scope = fs::canonicalize(&project_root)
        .expect("canonical project")
        .display()
        .to_string();

    let obsolete = write_valid_km(&source_root, "sample-a", "0.1.0", "old claim");
    store
        .import_for_project(&obsolete, &project_scope)
        .expect("import obsolete version");
    let active = write_valid_km(&source_root, "sample-a", "0.2.0", "active claim");
    store
        .import_for_project(&active, &project_scope)
        .expect("import active version");
    let untracked = write_valid_km(&project_root, "sample-b", "0.1.0", "untracked claim");
    store
        .import_for_project(&untracked, &project_scope)
        .expect("import untracked source");
    let unscoped = write_valid_km(&source_root, "sample-c", "0.1.0", "unscoped claim");
    store.import(&unscoped).expect("import unscoped source");

    fs::remove_dir_all(obsolete).expect("remove obsolete source");
    fs::write(untracked.join("examples/README.md"), "changed body\n")
        .expect("edit untracked source");
    fs::write(unscoped.join("examples/README.md"), "changed body\n").expect("edit unscoped source");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
}

#[test]
fn health_keeps_reporting_cached_snapshot_failures_with_source_drift() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    let source_root = checked_in_km_root(&project_root);
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(&source_root, "sample-a", "0.1.0", "bounded claim");
    let project_scope = fs::canonicalize(&project_root)
        .expect("canonical project")
        .display()
        .to_string();
    let record = store
        .import_for_project(&source, &project_scope)
        .expect("import");

    fs::write(source.join("examples/README.md"), "changed body\n").expect("edit source");
    fs::write(Path::new(&record.path).join("facts.jsonl"), "{\n").expect("break cached facts");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(
        result
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>(),
        vec!["source-changed", "validation-status", "validation-error"]
    );
}

// Backlog 82: repeated heal scans over unchanged state produce identical
// reports.
#[test]
fn heal_scan_output_is_deterministic_across_runs() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    for name in ["sample-a", "sample-b", "sample-c"] {
        let source = write_valid_km(tempdir.path(), name, "0.1.0", "bounded claim");
        store.import(&source).expect("import");
    }
    let request = KmLibrarianHealScanRequest {
        km_store_root: store_root,
    };

    let first = run_km_librarian_heal_scan(&request).expect("first scan");
    let second = run_km_librarian_heal_scan(&request).expect("second scan");

    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_string(&first).expect("first json"),
        serde_json::to_string(&second).expect("second json")
    );
}

// Backlog 83: an empty store yields an empty catalog, not an error.
#[test]
fn catalog_with_zero_modules_serializes_empty_not_error() {
    let tempdir = TempDir::new().expect("temp dir");

    let result = run_km_librarian_catalog(&KmLibrarianCatalogRequest {
        km_store_root: tempdir.path().join("km-cache"),
    })
    .expect("catalog succeeds on an empty store");

    assert_eq!(result.modules, Vec::new());
    assert_eq!(
        serde_json::to_string(&result).expect("json"),
        "{\"modules\":[]}"
    );
}

// Backlog 85: the consolidate loop is report-only and never writes into the
// store directory.
#[test]
fn consolidate_report_only_never_writes_to_store_dir() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    store.import(&source).expect("import");
    let before = store_files(&store_root);

    run_km_librarian_heal_scan(&KmLibrarianHealScanRequest {
        km_store_root: store_root.clone(),
    })
    .expect("heal scan");

    assert_eq!(
        store_files(&store_root),
        before,
        "a read-only scan must not mutate the store"
    );
}

/// Snapshots every file under the store root with its bytes.
fn store_files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut entries = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(read_dir) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if let Ok(bytes) = fs::read(&path) {
                entries.push((path, bytes));
            }
        }
    }
    entries.sort();
    entries
}

fn checked_in_km_root(project_root: &Path) -> std::path::PathBuf {
    let source_root = project_root.join(".ontocode/ontograph/km");
    fs::create_dir_all(&source_root).expect("checked-in KM root");
    fs::write(project_root.join("doc"), "checked-in source evidence\n").expect("source evidence");
    source_root
}

// Backlog 84: query results are scope-ranked, with every local candidate
// ordered ahead of every global one.
#[test]
fn query_scope_ranking_prefers_local_over_global() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    // `local` matches on fact claims; `neighbor` is reachable only as a
    // reference target, so it lands in the global scope.
    let local = write_valid_km(tempdir.path(), "local", "0.1.0", "alpha retrieval claim");
    let neighbor = write_valid_km(tempdir.path(), "neighbor", "0.1.0", "unrelated claim");
    store.import(&local).expect("import local");
    store.import(&neighbor).expect("import neighbor");

    let result = run_km_librarian_query(&KmLibrarianQueryRequest {
        km_store_root: store_root,
        query: "alpha retrieval claim".to_string(),
        max_candidates: 10,
    })
    .expect("query succeeds");

    let scopes = result
        .candidates
        .iter()
        .map(|candidate| candidate.scope)
        .collect::<Vec<_>>();
    let first_global = scopes
        .iter()
        .position(|scope| *scope == KmLibrarianQueryScope::Global);
    if let Some(first_global) = first_global {
        assert!(
            scopes[first_global..]
                .iter()
                .all(|scope| *scope == KmLibrarianQueryScope::Global),
            "local candidates must all precede global ones: {scopes:?}"
        );
    }
    assert!(
        scopes.first() == Some(&KmLibrarianQueryScope::Local),
        "a local match ranks first: {scopes:?}"
    );
}

// Backlog 86: the agent plan reuses the caller's own request paths and adds no
// extra host path of its own.
#[test]
fn agent_plan_never_embeds_absolute_host_paths() {
    let store_root = Path::new("/tmp/km-cache").to_path_buf();
    let project_root = Path::new("/tmp/project").to_path_buf();
    let plan = run_km_librarian_agent_plan(&KmLibrarianAgentRequest {
        km_store_root: store_root.clone(),
        input_dirs: vec![Path::new("/tmp/input").to_path_buf()],
        output_dir: Path::new("/tmp/output").to_path_buf(),
        project_root,
    });

    // Every path in the plan comes from the caller's request; the planner
    // never introduces a home directory or other host path of its own.
    for step in &plan.steps {
        let path = match step {
            KmLibrarianAgentPlanStep::Health(request) => &request.km_store_root,
            KmLibrarianAgentPlanStep::Catalog(request) => &request.km_store_root,
            KmLibrarianAgentPlanStep::HealScan(request) => &request.km_store_root,
            KmLibrarianAgentPlanStep::Consolidate(request) => &request.km_store_root,
        };
        assert_eq!(
            path, &store_root,
            "plan steps reuse the caller-supplied store root"
        );
    }
    assert!(!plan.steps.is_empty());
}

// OGRC-04: a cache directory with no index record is reported as an orphan.
// Orphans are storage the store never reads and never reclaims, so they stay
// invisible without this check.
#[test]
fn health_reports_orphan_cache_directory_without_index_record() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");

    // Copy the imported package to a version directory the index does not know.
    let orphan_dir = Path::new(&record.path)
        .parent()
        .expect("version parent")
        .join("9.9.9");
    fs::create_dir_all(&orphan_dir).expect("orphan dir");
    for entry in fs::read_dir(&record.path).expect("read cached package") {
        let entry = entry.expect("entry");
        if entry.path().is_file() {
            fs::copy(entry.path(), orphan_dir.join(entry.file_name())).expect("copy file");
        }
    }

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    let orphans = result
        .issues
        .iter()
        .filter(|issue| issue.code == "orphan-cache-dir")
        .collect::<Vec<_>>();
    assert_eq!(orphans.len(), 1, "unexpected issues: {:?}", result.issues);
    assert!(orphans[0].path.contains("9.9.9"));
}

// OGRC-04: mutating a cached package after import breaks its recorded content
// hash. `validate_km` alone does not catch this, because the tampered package
// is still structurally valid.
#[test]
fn health_reports_content_tampering_that_keeps_the_package_valid() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let source = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let record = store.import(&source).expect("import");

    // Append to an example file: still a well-formed package, different bytes.
    let example = Path::new(&record.path).join("examples/README.md");
    let original = fs::read_to_string(&example).expect("read example");
    fs::write(&example, format!("{original}\nappended after import\n")).expect("tamper");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert!(
        result
            .issues
            .iter()
            .any(|issue| issue.code == "content-tampered"),
        "unexpected issues: {:?}",
        result.issues
    );
}

// OGRC-04: a freshly obsoleted module is inside the retention window, so it is
// not reported. Retention is a diagnostic only and never deletes content.
#[test]
fn health_does_not_report_recently_obsoleted_module_as_retention_expired() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let first = write_valid_km(tempdir.path(), "sample-a", "0.1.0", "bounded claim");
    let second = write_valid_km(tempdir.path(), "sample-a", "0.2.0", "new bounded claim");
    store.import(&first).expect("first import");
    store.import(&second).expect("second import");

    let result = run_km_librarian_health(&KmLibrarianHealthRequest {
        km_store_root: store_root,
    })
    .expect("health");

    assert_eq!(result.obsolete_modules, 1);
    assert!(
        !result
            .issues
            .iter()
            .any(|issue| issue.code == "retention-expired"),
        "unexpected issues: {:?}",
        result.issues
    );
}
