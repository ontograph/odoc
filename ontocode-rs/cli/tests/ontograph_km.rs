use anyhow::Result;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn non_repo_tempdir() -> tempfile::TempDir {
    for base in [std::env::temp_dir(), std::path::PathBuf::from("/dev/shm")] {
        let Ok(base) = base.canonicalize() else {
            continue;
        };
        if base.ancestors().any(|ancestor| {
            [".git", ".agents", ".ontocode"]
                .iter()
                .any(|marker| ancestor.join(marker).symlink_metadata().is_ok())
        }) {
            continue;
        }
        if let Ok(dir) = tempfile::tempdir_in(base) {
            return dir;
        }
    }
    panic!("no writable temporary directory outside Git/project ancestors");
}

fn codex_command(ontocode_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(ontocode_utils_cargo_bin::cargo_bin("ontocode")?);
    cmd.env("ONTOCODE_HOME", ontocode_home);
    Ok(cmd)
}

fn write_valid_km(
    root: &Path,
    name: &str,
    version: &str,
    claim: &str,
) -> Result<std::path::PathBuf> {
    let km_root = root.join(format!("{name}-{version}"));
    // Citation validation resolves silver:doc against the project scope root.
    fs::write(root.join("doc"), "doc")?;
    fs::create_dir_all(km_root.join("examples"))?;
    fs::create_dir_all(km_root.join("tests"))?;
    fs::write(km_root.join("km.toml"), valid_manifest(name, version))?;
    fs::write(
        km_root.join("facts.jsonl"),
        format!(
            "{{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"{claim}\",\"source\":\"silver:doc\",\"confidence\":0.9}}\n"
        ),
    )?;
    fs::write(
        km_root.join("examples/README.md"),
        "# Example\n\nExample body.\n",
    )?;
    fs::write(km_root.join("tests/README.md"), "# Test\n\nTest body.\n")?;
    Ok(km_root)
}

fn valid_manifest(name: &str, version: &str) -> String {
    format!(
        "id = \"codebase-graph/{name}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}

fn init_git_repo(root: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()?;
    anyhow::ensure!(status.success(), "git init failed");
    Ok(())
}

#[test]
fn draft_extract_command_and_legacy_alias_are_available() -> Result<()> {
    let ontocode_home = TempDir::new()?;

    for command_name in ["draft-extract", "llm-extract"] {
        codex_command(ontocode_home.path())?
            .args(["ontograph", "km", command_name, "--help"])
            .assert()
            .success()
            .stdout(contains("deterministic draft KM package"));
    }

    Ok(())
}

#[test]
fn draft_extract_requires_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = non_repo_tempdir();
    let source = project.path().join("source.md");
    let output = project.path().join("_draft");
    fs::write(&source, "# Source\n")?;

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "draft-extract",
            "--project-root",
            project.path().to_str().unwrap(),
            "--source",
            "bronze:source.md",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("Ontograph KM authoring is disabled"));

    fs::write(
        ontocode_home.path().join("config.toml"),
        "[ontograph.km.llm]\nenabled = true\n",
    )?;

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "draft-extract",
            "--project-root",
            project.path().to_str().unwrap(),
            "--source",
            "bronze:source.md",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("draft:").and(contains("_draft")));

    assert!(output.join("km.toml").exists());
    assert!(output.join("facts.jsonl").exists());

    Ok(())
}

#[test]
fn ontograph_km_librarian_consolidate_rejects_output_outside_project() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = TempDir::new()?;
    init_git_repo(project.path())?;
    let outside = TempDir::new()?;
    let store_root = TempDir::new()?;
    let input = write_valid_km(project.path(), "sample-a", "0.1.0", "bounded claim")?;
    let output = outside.path().join("_draft-consolidated");

    codex_command(ontocode_home.path())?
        .current_dir(project.path())
        .args([
            "ontograph",
            "km",
            "librarian",
            "consolidate",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains(
            "output directory must be inside the canonical project root",
        ));

    assert!(!output.exists());

    Ok(())
}

#[test]
fn ontograph_km_librarian_health_runs_without_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let store_root = TempDir::new()?;

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "librarian",
            "health",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(
            contains("active_modules: 0")
                .and(contains("obsolete_modules: 0"))
                .and(contains("issues: 0")),
        );

    Ok(())
}

#[test]
fn ontograph_km_librarian_catalog_runs_without_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let store_root = TempDir::new()?;

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "librarian",
            "catalog",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("modules: 0"));

    Ok(())
}

#[test]
fn ontograph_km_librarian_version_diff_reports_retained_versions() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = non_repo_tempdir();
    let store_root = TempDir::new()?;
    let first = write_valid_km(project.path(), "sample-a", "0.1.0", "old claim")?;
    let second = write_valid_km(project.path(), "sample-a", "0.2.0", "new claim")?;
    fs::write(
        first.join("mappings.jsonl"),
        concat!(
            r#"{"id":"map-shared","kind":"concept_mapping","source_concept":"alpha","target_concept":"beta","relation":"same_as","source":"silver:doc","confidence":0.9}"#,
            "\n",
            r#"{"id":"map-changed","kind":"concept_mapping","source_concept":"gamma","target_concept":"delta","relation":"same_as","source":"silver:doc","confidence":0.4}"#,
            "\n",
            r#"{"id":"map-removed","kind":"concept_mapping","source_concept":"theta","target_concept":"iota","relation":"same_as","source":"silver:doc","confidence":0.5}"#,
            "\n",
        ),
    )?;
    fs::write(
        second.join("mappings.jsonl"),
        concat!(
            r#"{"id":"map-shared","kind":"concept_mapping","source_concept":"alpha","target_concept":"beta","relation":"same_as","source":"silver:doc","confidence":0.9}"#,
            "\n",
            r#"{"id":"map-changed","kind":"concept_mapping","source_concept":"gamma","target_concept":"epsilon","relation":"same_as","source":"silver:doc","confidence":0.4}"#,
            "\n",
            r#"{"id":"map-added","kind":"concept_mapping","source_concept":"zeta","target_concept":"eta","relation":"same_as","source":"silver:doc","confidence":0.6}"#,
            "\n",
        ),
    )?;
    for input in [&first, &second] {
        codex_command(ontocode_home.path())?
            .current_dir(project.path())
            .args([
                "ontograph",
                "km",
                "import",
                input.to_str().unwrap(),
                "--store-root",
                store_root.path().to_str().unwrap(),
            ])
            .assert()
            .success();
    }

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "librarian",
            "catalog",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("codebase-graph/sample-a@0.2.0").and(contains("0.1.0").not()));

    let version_diff_args = [
        "ontograph",
        "km",
        "librarian",
        "version-diff",
        "codebase-graph/sample-a",
        "0.1.0",
        "0.2.0",
        "--store-root",
        store_root.path().to_str().unwrap(),
    ];
    codex_command(ontocode_home.path())?
        .args(version_diff_args)
        .assert()
        .success()
        .stdout(
            contains("module: codebase-graph/sample-a 0.1.0 -> 0.2.0")
                .and(contains("truncated: false"))
                .and(contains("facts: +0 -0 ~1"))
                .and(contains("mappings: +1 -1 ~1")),
        );
    codex_command(ontocode_home.path())?
        .args(version_diff_args)
        .arg("--json")
        .assert()
        .success()
        .stdout(
            contains("\"moduleId\": \"codebase-graph/sample-a\"")
                .and(contains("\"version\": \"0.1.0\""))
                .and(contains("\"version\": \"0.2.0\""))
                .and(contains("\"factId\": \"fact-1\""))
                .and(contains("\"mappingId\": \"map-added\""))
                .and(contains("\"mappingId\": \"map-removed\""))
                .and(contains("\"mappingId\": \"map-changed\""))
                .and(contains("\"truncated\": false")),
        );

    Ok(())
}

#[test]
fn ontograph_km_librarian_query_runs_without_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = non_repo_tempdir();
    let store_root = TempDir::new()?;
    let lower_ranked = write_valid_km(project.path(), "sample-b", "0.1.0", "alpha claim")?;
    let higher_ranked = write_valid_km(project.path(), "sample-a", "0.1.0", "alpha beta claim")?;

    for input in [&lower_ranked, &higher_ranked] {
        codex_command(ontocode_home.path())?
            .current_dir(project.path())
            .args([
                "ontograph",
                "km",
                "import",
                input.to_str().unwrap(),
                "--store-root",
                store_root.path().to_str().unwrap(),
            ])
            .assert()
            .success();
    }

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "librarian",
            "query",
            "alpha beta",
            "--limit",
            "1",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(
            contains("query: alpha beta")
                .and(contains("candidates: 1"))
                .and(contains("truncated: true"))
                .and(contains("candidate: local codebase-graph/sample-a@0.1.0"))
                .and(contains("codebase-graph/sample-b").not()),
        );

    Ok(())
}

#[test]
fn ontograph_km_explicit_store_roots_are_isolated() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = non_repo_tempdir();
    let store_a = TempDir::new()?;
    let store_b = TempDir::new()?;
    let package_a = write_valid_km(
        project.path(),
        "store-a",
        "0.1.0",
        "shared isolation signal",
    )?;
    let package_b = write_valid_km(
        project.path(),
        "store-b",
        "0.1.0",
        "shared isolation signal",
    )?;

    for (package, store_root) in [(&package_a, &store_a), (&package_b, &store_b)] {
        codex_command(ontocode_home.path())?
            .current_dir(project.path())
            .args([
                "ontograph",
                "km",
                "import",
                package.to_str().unwrap(),
                "--store-root",
                store_root.path().to_str().unwrap(),
            ])
            .assert()
            .success();
    }

    for (store_root, expected, excluded) in [
        (&store_a, "codebase-graph/store-a", "codebase-graph/store-b"),
        (&store_b, "codebase-graph/store-b", "codebase-graph/store-a"),
    ] {
        codex_command(ontocode_home.path())?
            .current_dir(project.path())
            .args([
                "ontograph",
                "km",
                "librarian",
                "query",
                "shared isolation",
                "--store-root",
                store_root.path().to_str().unwrap(),
            ])
            .assert()
            .success()
            .stdout(contains(expected).and(contains(excluded).not()));
    }

    Ok(())
}

#[test]
fn ontograph_km_explicit_store_batch_import_works_outside_git() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let workspace = non_repo_tempdir();
    let store_root = TempDir::new()?;
    let package = write_valid_km(
        workspace.path(),
        "batch-store",
        "0.1.0",
        "batch custom store signal",
    )?;
    let batch_dir = workspace.path().join("batch");
    fs::create_dir(&batch_dir)?;
    fs::write(
        batch_dir.join("manifest.jsonl"),
        format!(
            "{}\n",
            serde_json::to_string(&package.display().to_string())?
        ),
    )?;

    codex_command(ontocode_home.path())?
        .current_dir(workspace.path())
        .args([
            "ontograph",
            "km",
            "import-batch",
            batch_dir.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("imported: 1"));

    codex_command(ontocode_home.path())?
        .current_dir(workspace.path())
        .args([
            "ontograph",
            "km",
            "librarian",
            "query",
            "batch custom store",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("codebase-graph/batch-store"));

    Ok(())
}

#[test]
fn ontograph_km_default_store_is_stable_across_project_subdirectories() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = TempDir::new()?;
    init_git_repo(project.path())?;
    let nested = project.path().join("nested");
    fs::create_dir(&nested)?;
    let package = write_valid_km(
        project.path(),
        "project-store",
        "0.1.0",
        "stable project store signal",
    )?;

    codex_command(ontocode_home.path())?
        .current_dir(&nested)
        .args(["ontograph", "km", "import", package.to_str().unwrap()])
        .assert()
        .success();

    codex_command(ontocode_home.path())?
        .current_dir(project.path())
        .args([
            "ontograph",
            "km",
            "librarian",
            "query",
            "stable project store",
        ])
        .assert()
        .success()
        .stdout(contains("codebase-graph/project-store"));

    assert!(
        project
            .path()
            .join(".ontocode/ontograph/km/_store/index.json")
            .is_file()
    );
    assert!(!nested.join(".ontocode/ontograph/km/_store").exists());

    Ok(())
}

#[test]
fn ontograph_km_librarian_heal_scan_runs_without_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let store_root = TempDir::new()?;

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "librarian",
            "heal-scan",
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("reports: 0").and(contains("conflicts: 0")));

    Ok(())
}

#[test]
fn ontograph_km_eval_supports_negative_control_cases() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = non_repo_tempdir();
    let store_root = TempDir::new()?;
    let input = write_valid_km(project.path(), "sample-eval", "0.1.0", "ordinary signal")?;
    let cases_dir = project.path().join("cases");
    fs::create_dir_all(&cases_dir)?;
    fs::write(
        cases_dir.join("case-1.json"),
        serde_json::json!({
            "id": "case-1",
            "prompt": "Does the KM mention unrelated material?",
            "requiredSubstrings": ["quantum teleportation protocol"],
            "expectPresent": false
        })
        .to_string(),
    )?;

    codex_command(ontocode_home.path())?
        .current_dir(project.path())
        .args([
            "ontograph",
            "km",
            "import",
            input.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    codex_command(ontocode_home.path())?
        .args([
            "ontograph",
            "km",
            "eval",
            "codebase-graph/sample-eval",
            "--cases",
            cases_dir.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(
            contains("\"totalCases\": 1")
                .and(contains("\"passedCases\": 1"))
                .and(contains("\"failedCases\": 0"))
                .and(contains("\"passRate\": 1.0")),
        );

    Ok(())
}

#[test]
fn ontograph_km_librarian_consolidate_runs_report_only_without_enabled_config() -> Result<()> {
    let ontocode_home = TempDir::new()?;
    let project = TempDir::new()?;
    init_git_repo(project.path())?;
    fs::create_dir(project.path().join("nested"))?;
    let store_root = TempDir::new()?;
    let input = write_valid_km(project.path(), "sample-a", "0.1.0", "bounded claim")?;
    let output = project.path().join("_draft-consolidated");

    codex_command(ontocode_home.path())?
        .current_dir(project.path())
        .args([
            "ontograph",
            "km",
            "import",
            input.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    codex_command(ontocode_home.path())?
        .current_dir(project.path().join("nested"))
        .args([
            "ontograph",
            "km",
            "librarian",
            "consolidate",
            "--input",
            input.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--store-root",
            store_root.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(
            contains("draft:")
                .and(contains("facts: 1"))
                .and(contains("health_active_modules: 1"))
                .and(contains("heal_reports: 0"))
                .and(contains(
                    "promotion: gold promotion and active-KM mutation were not attempted",
                )),
        );

    assert!(output.join("km.toml").exists());
    assert!(output.join("facts.jsonl").exists());

    Ok(())
}
