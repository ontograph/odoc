use std::fs;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;
use crate::km_store::KmStore;
use crate::km_store::KmStoreError;
use ontocode_utils_cargo_bin::repo_root;

#[test]
fn valid_sample_km_passes() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.errors, Vec::<String>::new());
    assert_eq!(report.facts, 1);
    assert_eq!(report.examples, 1);
    assert_eq!(report.tests, 1);
}

#[test]
fn every_project_local_km_package_passes_native_validation() {
    let repo_root = repo_root().expect("repo root");
    let km_root = repo_root.join(".ontocode/ontograph/km");
    assert!(
        km_root.is_dir(),
        "project-local KM store missing at {}",
        km_root.display()
    );

    let mut validated = 0usize;
    let mut failures = Vec::new();
    for entry in fs::read_dir(&km_root).expect("read KM store") {
        let entry = entry.expect("KM store entry");
        let path = entry.path();
        if !path.is_dir() || !path.join("km.toml").is_file() {
            continue;
        }
        validated += 1;
        let report = validate_km(&path);
        if report.status != KmValidationStatus::Valid {
            failures.push(format!("{}: {:?}", path.display(), report.errors));
        }
    }

    assert!(
        validated > 0,
        "no KM packages with km.toml found under {}",
        km_root.display()
    );
    assert!(
        failures.is_empty(),
        "invalid KM packages under {}: {}",
        km_root.display(),
        failures.join("; ")
    );
}

#[test]
fn cgraph_donor_patterns_km_passes_native_validation() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_cgraph_donor_patterns_km(tempdir.path());

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.errors, Vec::<String>::new());
    assert_eq!(report.id, "codebase-graph/cgraph-donor-patterns");
    assert_eq!(report.version, "0.1.1");
    assert_eq!(report.facts, 22);
    assert_eq!(report.examples, 1);
    assert_eq!(report.tests, 1);
}

#[test]
fn checked_in_sources_pass_context_aware_validation() {
    let project_root = repo_root().expect("repo root");
    let package = project_root.join(".ontocode/ontograph/km/cgraph-donor-patterns");
    let manifest = load_manifest(&package).expect("donor KM manifest");
    for source in manifest.sources.as_deref().unwrap_or_default() {
        if let Some(path) = source.strip_prefix("bronze:")
            && !project_root.join(path).exists()
        {
            eprintln!("skipping: donor sources not present: {path}");
            return;
        }
    }

    let report = validate_km_for_project(&package, &project_root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn missing_km_toml_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = tempdir.path().join("km");
    fs::create_dir_all(root.join("examples")).expect("examples dir");
    fs::create_dir_all(root.join("tests")).expect("tests dir");
    fs::write(root.join("facts.jsonl"), valid_fact_line()).expect("facts");
    fs::write(root.join("examples/README.md"), "example").expect("example");
    fs::write(root.join("tests/README.md"), "test").expect("test");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(report.errors.iter().any(|error| error.contains("missing")));
}

#[test]
fn invalid_semver_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("km.toml"),
        valid_manifest("not-semver", true, true),
    )
    .expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml version must be strict semver".to_string())
    );
}

#[test]
fn missing_license_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("km.toml"), valid_manifest("0.1.0", false, true)).expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml licenses must be present".to_string())
    );
}

#[test]
fn missing_provenance_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("km.toml"), valid_manifest("0.1.0", true, false)).expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml provenance must be present".to_string())
    );
}

#[test]
fn invalid_provenance_fields_fail_closed() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("km.toml"),
        format!(
            "{}\n[provenance]\ncreated_by = \" \"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n",
            valid_manifest("0.1.0", true, false)
        ),
    )
    .expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml provenance must be present".to_string())
    );
}

// `codebase_repos` is optional, so packages authored before the field stay valid.
#[test]
fn absent_codebase_repos_passes() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn populated_codebase_repos_passes() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("km.toml"),
        format!(
            "{}\n[[codebase_repos]]\nname = \"kimi-cli\"\nurl = \"https://github.com/MoonshotAI/kimi-cli.git\"\ncommit = \"6bb74f8c7f455f0d627c88110e6f3136717ee5e9\"\n",
            valid_manifest("0.1.0", true, true)
        ),
    )
    .expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.errors, Vec::<String>::new());
}

#[test]
fn codebase_repo_missing_commit_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("km.toml"),
        format!(
            "{}\n[[codebase_repos]]\nname = \"kimi-cli\"\nurl = \"https://github.com/MoonshotAI/kimi-cli.git\"\ncommit = \"  \"\n",
            valid_manifest("0.1.0", true, true)
        ),
    )
    .expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml codebase_repos entries require name, url, and commit".to_string())
    );
}

#[test]
fn codebase_repos_over_cap_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let mut manifest = valid_manifest("0.1.0", true, true);
    for index in 0..21 {
        manifest.push_str(&format!(
            "\n[[codebase_repos]]\nname = \"donor-{index}\"\nurl = \"https://example.invalid/donor-{index}.git\"\ncommit = \"abc123\"\n"
        ));
    }
    fs::write(root.join("km.toml"), manifest).expect("manifest");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml codebase_repos exceeds the v1 cap".to_string())
    );
}

#[test]
fn malformed_jsonl_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("facts.jsonl"), "{\n").expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("malformed JSONL"))
    );
}

#[test]
fn fact_without_source_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"ok\",\"source\":\"\",\"confidence\":0.9}\n",
    )
    .expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"fact is missing source".to_string())
    );
}

#[test]
fn context_free_validation_does_not_guess_a_source_root() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
}

#[test]
fn over_cap_claim_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let long_claim = "x".repeat(MAX_FACT_CLAIM_CHARS + 1);
    fs::write(
        root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{long_claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"fact claim exceeds the v1 cap".to_string())
    );
}

#[test]
fn missing_examples_fail() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::remove_dir_all(root.join("examples")).expect("remove examples");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(report.errors.contains(&"examples must exist".to_string()));
}

#[test]
fn missing_tests_fail() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::remove_dir_all(root.join("tests")).expect("remove tests");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(report.errors.contains(&"tests must exist".to_string()));
}

fn write_valid_km(root: &std::path::Path) -> std::path::PathBuf {
    let km_root = root.join("sample-km");
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), valid_manifest("0.1.0", true, true)).expect("manifest");
    fs::write(km_root.join("facts.jsonl"), valid_fact_line()).expect("facts");
    fs::write(km_root.join("examples/README.md"), "example").expect("example");
    fs::write(km_root.join("tests/README.md"), "test").expect("test");
    km_root
}

#[test]
fn oversized_package_is_rejected_by_budget() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    // Write an example larger than the 4 MiB total-byte budget.
    fs::write(root.join("examples/big.bin"), vec![0u8; 5 * 1024 * 1024]).expect("big example");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"package exceeds the total byte budget".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

#[test]
fn deep_package_tree_is_rejected_by_budget() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    // Build a directory chain deeper than the depth budget.
    let mut deep = root.join("examples");
    for index in 0..40 {
        deep = deep.join(format!("d{index}"));
    }
    fs::create_dir_all(&deep).expect("deep tree");
    fs::write(deep.join("leaf.md"), "leaf").expect("leaf");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"package directory tree is too deep".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

#[cfg(unix)]
#[test]
fn symlinked_package_entry_is_rejected_by_budget() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    // Point a symlink at an out-of-package target.
    let outside = tempdir.path().join("outside.txt");
    fs::write(&outside, "secret").expect("outside target");
    std::os::unix::fs::symlink(&outside, root.join("examples/link.md")).expect("symlink");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"package contains a symlinked entry".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

fn write_cgraph_donor_patterns_km(root: &std::path::Path) -> std::path::PathBuf {
    let km_root = root.join("cgraph-donor-patterns");
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(
        km_root.join("km.toml"),
        include_str!("../../../../.ontocode/ontograph/km/cgraph-donor-patterns/km.toml"),
    )
    .expect("manifest");
    fs::write(
        km_root.join("facts.jsonl"),
        include_str!("../../../../.ontocode/ontograph/km/cgraph-donor-patterns/facts.jsonl"),
    )
    .expect("facts");
    fs::write(
        km_root.join("examples/README.md"),
        include_str!("../../../../.ontocode/ontograph/km/cgraph-donor-patterns/examples/README.md"),
    )
    .expect("example");
    fs::write(
        km_root.join("tests/README.md"),
        include_str!("../../../../.ontocode/ontograph/km/cgraph-donor-patterns/tests/README.md"),
    )
    .expect("test");
    km_root
}

fn valid_manifest(version: &str, include_license: bool, include_provenance: bool) -> String {
    let licenses = if include_license {
        "licenses = [\"MIT\"]\n"
    } else {
        "licenses = []\n"
    };
    let provenance = if include_provenance {
        "[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    } else {
        ""
    };
    format!(
        "id = \"codebase-graph/creating-ontograph\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\n{licenses}\n{provenance}"
    )
}

fn valid_fact_line() -> &'static str {
    "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":0.9}\n"
}

fn valid_transition_line() -> &'static str {
    "{\"source\":\"inspect\",\"target\":\"validate\",\"relation\":\"next\",\"condition\":\"when inspection is complete\",\"guidance\":\"validate the observed state\",\"pitfalls\":[\"do not skip validation\"]}\n"
}

#[test]
fn valid_procedural_transitions_pass_native_validation() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("transitions.jsonl"), valid_transition_line()).expect("transitions");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(load_transitions(&root).expect("load transitions").len(), 1);
}

#[test]
fn active_cache_activation_rejects_invalid_procedural_transitions() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("transitions.jsonl"), valid_transition_line()).expect("transitions");
    let mut record = store.import(&root).expect("import succeeds");

    fs::write(
        std::path::Path::new(&record.path).join("transitions.jsonl"),
        "{\"source\":\"\",\"target\":\"validate\",\"relation\":\"next\",\"condition\":\"ok\",\"guidance\":\"ok\"}\n",
    )
    .expect("tamper cached transitions");
    // Keep the content hash internally consistent so this assertion exercises
    // the transition activation seam rather than the earlier tamper check.
    record.content_hash =
        crate::km_store::hash_directory(std::path::Path::new(&record.path)).expect("rehash cache");

    assert_eq!(
        store.validated_module_root(&record),
        Err(KmStoreError::InvalidIndex)
    );
}

#[test]
fn active_cache_activation_accepts_valid_procedural_transitions() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("transitions.jsonl"), valid_transition_line()).expect("transitions");
    let record = store.import(&root).expect("import succeeds");

    let activated = store
        .validated_module_root(&record)
        .expect("valid procedural cache activates");

    assert_eq!(
        activated,
        fs::canonicalize(&record.path).expect("canonical cached snapshot")
    );
    assert_eq!(
        load_transitions(&activated)
            .expect("validated procedural transitions load")
            .len(),
        1
    );
}

#[cfg(unix)]
#[test]
fn active_cache_activation_rejects_symlinked_procedural_entries() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("transitions.jsonl"), valid_transition_line()).expect("transitions");
    let record = store.import(&root).expect("import succeeds");

    let cached_transitions = std::path::Path::new(&record.path).join("transitions.jsonl");
    fs::remove_file(&cached_transitions).expect("remove cached transitions");
    let outside = tempdir.path().join("outside-transitions.jsonl");
    fs::write(&outside, valid_transition_line()).expect("outside transitions");
    std::os::unix::fs::symlink(&outside, &cached_transitions).expect("symlink transitions");

    assert_eq!(
        store.validated_module_root(&record),
        Err(KmStoreError::InvalidIndex)
    );
}

#[test]
fn malformed_transition_fails_closed() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("transitions.jsonl"), "{\n").expect("transitions");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("transitions.jsonl contains malformed JSONL"))
    );
}

#[test]
fn transition_missing_endpoint_and_oversized_attribute_fail() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let oversized = "x".repeat(MAX_TRANSITION_ATTRIBUTE_CHARS + 1);
    fs::write(
        root.join("transitions.jsonl"),
        format!("{{\"source\":\"\",\"target\":\"validate\",\"relation\":\"next\",\"condition\":\"ok\",\"guidance\":\"{oversized}\"}}\n"),
    ).expect("transitions");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"transition requires source and target".to_string())
    );
    assert!(
        report
            .errors
            .contains(&"transition attribute exceeds the v1 cap".to_string())
    );
}

#[test]
fn direct_transition_loader_rejects_boundary_violations() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let oversized = "x".repeat(MAX_TRANSITION_ATTRIBUTE_CHARS + 1);
    fs::write(
        root.join("transitions.jsonl"),
        format!("{{\"source\":\"inspect\",\"target\":\"validate\",\"relation\":\"next\",\"condition\":\"ok\",\"guidance\":\"{oversized}\"}}\n"),
    )
    .expect("transitions");

    let error = load_transitions(&root).expect_err("invalid transition must not load");

    assert_eq!(error, "transition attribute exceeds the v1 cap");

    let valid = valid_transition_line();
    fs::write(
        root.join("transitions.jsonl"),
        valid.repeat(MAX_TRANSITION_COUNT + 1),
    )
    .expect("transitions over cap");
    assert_eq!(
        load_transitions(&root).expect_err("transition count must be bounded"),
        "transitions.jsonl exceeds the v1 transition cap"
    );
}

fn transition_line(source: &str, target: &str) -> String {
    format!(
        "{{\"source\":\"{source}\",\"target\":\"{target}\",\"relation\":\"next\",\"condition\":\"when inspection is complete\",\"guidance\":\"validate the observed state\"}}\n"
    )
}

// PG-R1: the endpoint cap is the one transition boundary with no proof in
// either path. The report-collecting validator and the fail-fast loader must
// place it at the same character count.
#[test]
fn transition_endpoint_at_exact_cap_passes_and_one_over_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());

    let at_cap = "e".repeat(MAX_TRANSITION_ENDPOINT_CHARS);
    fs::write(
        root.join("transitions.jsonl"),
        transition_line(&at_cap, "validate"),
    )
    .expect("transitions at cap");
    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);
    assert_eq!(
        load_transitions(&root)
            .expect("at-cap endpoint loads")
            .len(),
        1
    );

    let over_cap = "e".repeat(MAX_TRANSITION_ENDPOINT_CHARS + 1);
    fs::write(
        root.join("transitions.jsonl"),
        transition_line("inspect", &over_cap),
    )
    .expect("transitions over cap");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"transition endpoint exceeds the v1 cap".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
    assert_eq!(
        load_transitions(&root).expect_err("over-cap endpoint must not load"),
        "transition endpoint exceeds the v1 cap"
    );
}

// PG-R1: the loader already proves the count cap, but the validator reaches it
// through a separate counter. Both must admit exactly MAX_TRANSITION_COUNT.
#[test]
fn transitions_at_exact_count_cap_pass_validation_and_one_over_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let at_cap = valid_transition_line().repeat(MAX_TRANSITION_COUNT);

    fs::write(root.join("transitions.jsonl"), &at_cap).expect("transitions at cap");
    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);
    assert_eq!(
        load_transitions(&root)
            .expect("at-cap transitions load")
            .len(),
        MAX_TRANSITION_COUNT
    );

    fs::write(
        root.join("transitions.jsonl"),
        format!("{at_cap}{}", valid_transition_line()),
    )
    .expect("transitions over cap");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"transitions.jsonl exceeds the v1 transition cap".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

// PG-R1: procedural records are validated alongside the manifest, not instead
// of it, so well-formed transitions never mask a provenance failure.
#[test]
fn valid_transitions_do_not_mask_invalid_provenance() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("km.toml"), valid_manifest("0.1.0", true, false)).expect("manifest");
    fs::write(root.join("transitions.jsonl"), valid_transition_line()).expect("transitions");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"km.toml provenance must be present".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
    assert!(
        !report
            .errors
            .iter()
            .any(|error| error.starts_with("transition")),
        "unexpected transition errors: {:?}",
        report.errors
    );
}

// PG-R1 (decision: pin current behavior): transitions.jsonl is optional to
// `validate_km` but required by `load_transitions`. `validated_module_root`
// bridges the two with an `is_file()` guard, so the asymmetry stays safe only
// while that guard runs in front of the loader.
#[test]
fn transitions_are_optional_to_validation_but_required_by_the_loader() {
    let tempdir = TempDir::new().expect("temp dir");
    let store = KmStore::new(tempdir.path().join("km-cache"));
    let root = write_valid_km(tempdir.path());
    assert!(!root.join("transitions.jsonl").exists());

    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);
    assert!(
        load_transitions(&root)
            .expect_err("loader requires the file")
            .starts_with("missing ")
    );

    let record = store.import(&root).expect("import succeeds");
    store
        .validated_module_root(&record)
        .expect("package without transitions activates");
}

// PG-R1: an oversized transitions.jsonl is refused by the shared JSONL byte
// cap before any line is parsed, so the per-line caps are never the only bound.
#[test]
fn oversized_transitions_file_is_rejected_by_the_byte_cap() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let mut oversized = String::new();
    while oversized.len() <= MAX_FACTS_JSONL_BYTES as usize {
        oversized.push_str(valid_transition_line());
    }
    fs::write(root.join("transitions.jsonl"), &oversized).expect("oversized transitions");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report.errors.iter().any(|error| {
            error.starts_with("failed to read transitions.jsonl")
                && error.ends_with("exceeds the v1 size cap")
        }),
        "unexpected errors: {:?}",
        report.errors
    );
}

// Backlog 15: the km.toml byte cap is inclusive at the boundary.
#[test]
fn km_toml_at_exact_byte_cap_passes_and_one_over_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let manifest_path = root.join("km.toml");
    let base = fs::read_to_string(&manifest_path).expect("manifest readable");
    let cap = 64 * 1024;
    let padding = cap - base.len() - 2;
    let padded = format!("{base}#{}\n", "p".repeat(padding));
    assert_eq!(padded.len(), cap);

    fs::write(&manifest_path, &padded).expect("write padded manifest");
    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);

    fs::write(&manifest_path, format!("{padded}#")).expect("write oversized manifest");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.ends_with("exceeds the v1 size cap")),
        "unexpected errors: {:?}",
        report.errors
    );
}

// Backlog 16: exactly 200 facts pass; the 201st is rejected.
#[test]
fn facts_jsonl_at_exact_fact_cap_passes_and_201st_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    let fact = |index: usize| {
        format!(
            "{{\"id\":\"fact-{index}\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        )
    };
    let at_cap = (0..200).map(fact).collect::<String>();

    fs::write(root.join("facts.jsonl"), &at_cap).expect("facts at cap");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.facts, 200);

    fs::write(root.join("facts.jsonl"), format!("{at_cap}{}", fact(200))).expect("facts over cap");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"facts.jsonl exceeds the v1 fact cap".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

// Backlog 17: the claim cap counts characters, not bytes.
#[test]
fn fact_claim_at_exactly_500_chars_passes() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    // 500 two-byte characters: 1000 bytes but exactly at the char cap.
    let claim = "\u{e9}".repeat(500);
    fs::write(
        root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);

    let over = "\u{e9}".repeat(501);
    fs::write(
        root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{over}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )
    .expect("facts");
    let report = validate_km(&root);
    assert!(
        report
            .errors
            .contains(&"fact claim exceeds the v1 cap".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

// Backlog 18: JSON cannot encode NaN or Infinity, so non-finite confidence is
// rejected at the parse layer as malformed JSONL. Pinned so a serde or schema
// change cannot silently start admitting non-finite confidence.
#[test]
fn confidence_nan_and_infinity_are_rejected() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    for literal in ["NaN", "Infinity", "-Infinity"] {
        fs::write(
            root.join("facts.jsonl"),
            format!(
                "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":{literal}}}\n"
            ),
        )
        .expect("facts");
        let report = validate_km(&root);
        assert_eq!(report.status, KmValidationStatus::Invalid, "{literal}");
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.starts_with("facts.jsonl contains malformed JSONL")),
            "unexpected errors for {literal}: {:?}",
            report.errors
        );
    }
}

// Backlog 19 (decision: pin current behavior): CRLF line endings parse because
// the carriage return is JSON whitespace, and the fact content is unaffected.
#[test]
fn crlf_facts_jsonl_lines_parse_or_fail_deterministically() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":0.9}\r\n",
    )
    .expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.facts, 1);
}

// Backlog 20 (decision: pin current behavior): duplicate fact ids are not
// validated today, so a package carrying them still passes. Flagging them
// would be a behavior change requiring its own decision.
#[test]
fn duplicate_fact_ids_are_currently_accepted() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(
        root.join("facts.jsonl"),
        format!("{}{}", valid_fact_line(), valid_fact_line()),
    )
    .expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Valid);
    assert_eq!(report.facts, 2);
}

// Backlog 21: dot segments, parent segments, leading slashes, and empty
// segments are rejected; a backslash inside a segment is an opaque character.
#[test]
fn id_with_dot_or_empty_segments_is_rejected() {
    for rejected in ["a/./b", "a/../b", "/a/b", "a//b", "", "."] {
        assert!(!is_path_like_id(rejected), "{rejected} must be rejected");
    }
    // Pinned current behavior: backslash is not a segment separator.
    assert!(is_path_like_id("a\\b"));
}

// Backlog 22: exactly 2000 package entries pass; the 2001st is rejected.
#[test]
fn package_entry_count_boundary_at_exactly_2000_passes() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    // The valid package has 6 entries. Filler lives outside examples/tests so
    // the per-directory file caps do not interfere with the budget boundary.
    let filler = root.join("filler");
    fs::create_dir_all(&filler).expect("filler dir");
    for index in 0..1993 {
        fs::write(filler.join(format!("f{index}")), "x").expect("filler file");
    }

    assert_eq!(validate_km(&root).status, KmValidationStatus::Valid);

    fs::write(filler.join("f1993"), "x").expect("one over");
    let report = validate_km(&root);
    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .contains(&"package has too many entries".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

// OGRC-06: generated-case proof that every ingestion budget rejects with its own
// message. One table drives all four families so a new budget cannot be added
// without a rejection case, and each case is built from the real constant rather
// than a hardcoded literal.
#[test]
#[allow(clippy::type_complexity)]
fn every_package_budget_family_rejects_with_its_own_message() {
    // (label, mutation, expected error)
    let cases: Vec<(&str, fn(&std::path::Path), &str)> = vec![
        (
            "total bytes",
            |root| {
                let oversized = vec![b'x'; (MAX_PACKAGE_TOTAL_BYTES as usize) + 1];
                fs::write(root.join("examples/big.md"), oversized).expect("oversized example");
            },
            "package exceeds the total byte budget",
        ),
        (
            "depth",
            |root| {
                let mut deep = root.to_path_buf();
                for index in 0..=MAX_PACKAGE_DEPTH {
                    deep = deep.join(format!("d{index}"));
                }
                fs::create_dir_all(&deep).expect("deep chain");
            },
            "package directory tree is too deep",
        ),
        (
            "entries",
            |root| {
                let filler = root.join("filler");
                fs::create_dir_all(&filler).expect("filler dir");
                // The valid package plus this filler crosses MAX_PACKAGE_ENTRIES.
                for index in 0..=MAX_PACKAGE_ENTRIES {
                    fs::write(filler.join(format!("f{index}")), "x").expect("filler file");
                }
            },
            "package has too many entries",
        ),
        (
            "symlink",
            |root| {
                let outside = root.join("..").join("outside.md");
                fs::write(&outside, "outside").expect("outside file");
                std::os::unix::fs::symlink(&outside, root.join("examples/link.md"))
                    .expect("symlink");
            },
            "package contains a symlinked entry",
        ),
    ];

    for (label, mutate, expected) in cases {
        let tempdir = TempDir::new().expect("temp dir");
        let root = write_valid_km(tempdir.path());
        // The unmutated package must pass, so a rejection below is caused by the
        // mutation and not by an unrelated defect in the fixture.
        assert_eq!(
            validate_km(&root).status,
            KmValidationStatus::Valid,
            "{label}: baseline package must be valid"
        );

        mutate(&root);

        let report = validate_km(&root);
        assert_eq!(
            report.status,
            KmValidationStatus::Invalid,
            "{label}: mutated package must be rejected"
        );
        assert!(
            report.errors.contains(&expected.to_string()),
            "{label}: expected `{expected}`, got {:?}",
            report.errors
        );
    }
}

// OGRC-06: fact sources are rejected for absolute, traversal, and Windows-style
// paths on every platform, so a package authored on Windows cannot smuggle a
// path that escapes the project root here.
#[test]
fn cross_platform_source_paths_are_rejected_uniformly() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path();

    let rejected = [
        // Absolute POSIX.
        "silver:/etc/passwd",
        // Parent traversal, plain and nested.
        "silver:../outside.md",
        "silver:docs/../../outside.md",
        // Windows drive prefix and UNC.
        "silver:C:/Windows/system.ini",
        "silver:\\\\server\\share\\file.md",
        // Backslash separator anywhere.
        "silver:docs\\nested\\file.md",
        // Empty and prefix-only.
        "silver:",
        "silver:   ",
        // Unknown prefix.
        "gold:docs/file.md",
        // Missing prefix entirely.
        "docs/file.md",
    ];

    for source in rejected {
        assert_eq!(
            canonical_source_path(project_root, source),
            None,
            "{source} must be rejected"
        );
    }
}

// Backlog 23: an examples tree holding only an empty subdirectory counts zero
// files and fails the at-least-one-file rule.
#[test]
fn empty_examples_subdirectory_counts_zero_files_and_fails() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::remove_file(root.join("examples/README.md")).expect("remove example");
    fs::create_dir_all(root.join("examples/nested")).expect("empty nested dir");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert_eq!(report.examples, 0);
    assert!(
        report
            .errors
            .contains(&"examples must contain at least one file".to_string()),
        "unexpected errors: {:?}",
        report.errors
    );
}

// Backlog 24: invalid UTF-8 in facts.jsonl fails with a read error, no panic.
#[test]
fn non_utf8_facts_file_fails_with_read_error_not_panic() {
    let tempdir = TempDir::new().expect("temp dir");
    let root = write_valid_km(tempdir.path());
    fs::write(root.join("facts.jsonl"), [0xff, 0xfe, 0x00, 0x41]).expect("facts");

    let report = validate_km(&root);

    assert_eq!(report.status, KmValidationStatus::Invalid);
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.starts_with("failed to read")),
        "unexpected errors: {:?}",
        report.errors
    );
}
