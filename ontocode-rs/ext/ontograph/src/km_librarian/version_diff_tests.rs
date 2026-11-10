use std::fs;
use std::path::Path;
use std::path::PathBuf;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;
use crate::km_librarian::KmLibrarianCatalogRequest;
use crate::km_librarian::run_km_librarian_catalog;
use crate::km_librarian::run_km_librarian_version_diff;
use crate::km_store::KmModuleStatus;

#[test]
fn diff_reports_added_removed_and_changed_facts_and_mappings() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);

    let first = write_km(
        tempdir.path(),
        "sample-a",
        "0.1.0",
        &[
            ("fact-shared", "unchanged claim", 0.9),
            ("fact-changed", "old claim", 0.5),
            ("fact-removed", "dropped claim", 0.7),
        ],
        &[
            ("map-shared", "alpha", "beta", 0.9),
            ("map-changed", "gamma", "delta", 0.4),
            ("map-removed", "theta", "iota", 0.5),
        ],
    );
    let second = write_km(
        tempdir.path(),
        "sample-a",
        "0.2.0",
        &[
            ("fact-shared", "unchanged claim", 0.9),
            ("fact-changed", "new claim", 0.5),
            ("fact-added", "fresh claim", 0.8),
        ],
        &[
            ("map-shared", "alpha", "beta", 0.9),
            ("map-changed", "gamma", "epsilon", 0.4),
            ("map-added", "zeta", "eta", 0.6),
        ],
    );
    store.import(&first).expect("import first");
    store.import(&second).expect("import second");

    let result = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root,
        module_id: "codebase-graph/sample-a".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.2.0".to_string(),
    })
    .expect("version diff");

    assert_eq!(result.module_id, "codebase-graph/sample-a");
    assert_eq!(result.left.version, "0.1.0");
    assert_eq!(result.left.status, KmModuleStatus::Obsolete);
    assert_eq!(result.right.version, "0.2.0");
    assert_eq!(result.right.status, KmModuleStatus::Active);
    assert!(!result.truncated);

    assert_eq!(
        result
            .added_facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect::<Vec<_>>(),
        vec!["fact-added"]
    );
    assert_eq!(
        result
            .removed_facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect::<Vec<_>>(),
        vec!["fact-removed"]
    );
    assert_eq!(
        result
            .changed_facts
            .iter()
            .map(|change| (
                change.fact_id.as_str(),
                change.before.claim.as_str(),
                change.after.claim.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![("fact-changed", "old claim", "new claim")]
    );

    assert_eq!(
        result
            .added_mappings
            .iter()
            .map(|mapping| mapping.mapping_id.as_str())
            .collect::<Vec<_>>(),
        vec!["map-added"]
    );
    assert_eq!(
        result
            .removed_mappings
            .iter()
            .map(|mapping| mapping.mapping_id.as_str())
            .collect::<Vec<_>>(),
        vec!["map-removed"]
    );
    assert_eq!(
        result
            .changed_mappings
            .iter()
            .map(|change| (
                change.mapping_id.as_str(),
                change.before.target_concept.as_str(),
                change.after.target_concept.as_str()
            ))
            .collect::<Vec<_>>(),
        vec![("map-changed", "delta", "epsilon")]
    );
}

#[test]
fn diff_caps_collections_and_reports_truncation_with_deterministic_order() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);

    let added: Vec<(String, String, f64)> = (0..MAX_CHANGED_ITEMS + 10)
        .map(|index| (format!("fact-{index:03}"), format!("claim {index}"), 0.5))
        .collect();
    let added_refs: Vec<(&str, &str, f64)> = added
        .iter()
        .map(|(id, claim, confidence)| (id.as_str(), claim.as_str(), *confidence))
        .collect();

    let first = write_km(
        tempdir.path(),
        "sample-cap",
        "0.1.0",
        &[("fact-keep", "kept claim", 0.9)],
        &[],
    );
    let second = write_km(tempdir.path(), "sample-cap", "0.2.0", &added_refs, &[]);
    store.import(&first).expect("import first");
    store.import(&second).expect("import second");

    let result = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root,
        module_id: "codebase-graph/sample-cap".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.2.0".to_string(),
    })
    .expect("version diff");

    assert!(result.truncated);
    assert_eq!(result.added_facts.len(), MAX_CHANGED_ITEMS);
    // Deterministic order: capped output is the first N stable ids ascending.
    assert_eq!(result.added_facts[0].fact_id, "fact-000");
    assert_eq!(
        result.added_facts[MAX_CHANGED_ITEMS - 1].fact_id,
        format!("fact-{:03}", MAX_CHANGED_ITEMS - 1)
    );
}

#[test]
fn diff_fails_closed_on_missing_invalid_and_tampered_versions() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let first = write_km(
        tempdir.path(),
        "sample-a",
        "0.1.0",
        &[("fact-1", "claim", 0.9)],
        &[],
    );
    let second = write_km(
        tempdir.path(),
        "sample-a",
        "0.2.0",
        &[("fact-1", "claim", 0.9)],
        &[],
    );
    store.import(&first).expect("import first");
    let second_record = store.import(&second).expect("import second");

    let missing = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root.clone(),
        module_id: "codebase-graph/sample-a".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "9.9.9".to_string(),
    });
    assert_eq!(
        missing,
        Err(KmLibrarianVersionDiffError::VersionNotFound(
            "codebase-graph/sample-a@9.9.9".to_string()
        ))
    );

    let same_version = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root.clone(),
        module_id: "codebase-graph/sample-a".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.1.0".to_string(),
    });
    assert_eq!(
        same_version,
        Err(KmLibrarianVersionDiffError::InvalidRequest(
            "left and right versions must differ".to_string()
        ))
    );

    let empty_id = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root.clone(),
        module_id: "  ".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.2.0".to_string(),
    });
    assert_eq!(
        empty_id,
        Err(KmLibrarianVersionDiffError::InvalidRequest(
            "module id must not be empty".to_string()
        ))
    );

    // Tampering the cached snapshot must fail closed, not silently diff.
    fs::write(
        Path::new(&second_record.path).join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"tampered\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("tamper facts");

    let tampered = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root,
        module_id: "codebase-graph/sample-a".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.2.0".to_string(),
    });
    assert!(
        matches!(
            tampered,
            Err(KmLibrarianVersionDiffError::InvalidSnapshot(_))
        ),
        "tampered snapshot must fail closed, got {tampered:?}"
    );
}

#[test]
fn obsolete_versions_stay_invisible_to_ordinary_catalog_reads() {
    let tempdir = TempDir::new().expect("temp dir");
    let store_root = tempdir.path().join("km-cache");
    let store = KmStore::new(&store_root);
    let first = write_km(
        tempdir.path(),
        "sample-a",
        "0.1.0",
        &[("fact-old", "old claim", 0.9)],
        &[],
    );
    let second = write_km(
        tempdir.path(),
        "sample-a",
        "0.2.0",
        &[("fact-new", "new claim", 0.9)],
        &[],
    );
    store.import(&first).expect("import first");
    store.import(&second).expect("import second");

    let catalog = run_km_librarian_catalog(&KmLibrarianCatalogRequest {
        km_store_root: store_root.clone(),
    })
    .expect("catalog");

    // Ordinary resolution is unchanged: only the active version is listed.
    assert_eq!(
        catalog
            .modules
            .iter()
            .map(|module| module.version.as_str())
            .collect::<Vec<_>>(),
        vec!["0.2.0"]
    );

    // The obsolete version is reachable only through the explicit report.
    let result = run_km_librarian_version_diff(&KmLibrarianVersionDiffRequest {
        km_store_root: store_root,
        module_id: "codebase-graph/sample-a".to_string(),
        left_version: "0.1.0".to_string(),
        right_version: "0.2.0".to_string(),
    })
    .expect("version diff");

    assert_eq!(result.left.status, KmModuleStatus::Obsolete);
    assert_eq!(
        result
            .removed_facts
            .iter()
            .map(|fact| fact.fact_id.as_str())
            .collect::<Vec<_>>(),
        vec!["fact-old"]
    );
}

fn write_km(
    root: &Path,
    name: &str,
    version: &str,
    facts: &[(&str, &str, f64)],
    mappings: &[(&str, &str, &str, f64)],
) -> PathBuf {
    let km_root = root.join(format!("{name}-{version}"));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(
        km_root.join("km.toml"),
        format!(
            "id = \"codebase-graph/{name}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .expect("manifest");

    let facts_text = facts
        .iter()
        .map(|(id, claim, confidence)| {
            format!(
                "{{\"id\":\"{id}\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":{confidence}}}"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(km_root.join("facts.jsonl"), format!("{facts_text}\n")).expect("facts");

    if !mappings.is_empty() {
        let mappings_text = mappings
            .iter()
            .map(|(id, source_concept, target_concept, confidence)| {
                format!(
                    "{{\"id\":\"{id}\",\"kind\":\"concept_mapping\",\"source_concept\":\"{source_concept}\",\"target_concept\":\"{target_concept}\",\"relation\":\"same_as\",\"source\":\"silver:doc\",\"confidence\":{confidence}}}"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(km_root.join("mappings.jsonl"), format!("{mappings_text}\n")).expect("mappings");
    }

    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )
    .expect("example");
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n").expect("test");
    km_root
}
