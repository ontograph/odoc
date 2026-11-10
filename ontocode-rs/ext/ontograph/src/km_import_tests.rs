use std::fs;
use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::*;

#[test]
fn run_km_import_imports_to_project_store_and_logs_progress() {
    let tempdir = TempDir::new().expect("temp dir");
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let result = run_km_import(&source, &store_root).expect("import succeeds");
    let index = fs::read_to_string(store_root.join("index.json")).expect("index exists");
    let log = fs::read_to_string(
        tempdir
            .path()
            .join(".ontocode/ontograph/km/_ops/import/single/progress.jsonl"),
    )
    .expect("log exists");

    assert_eq!(result.id, "codebase-graph/creating-ontograph");
    assert_eq!(result.version, "0.1.0");
    assert_eq!(result.status, "active");
    assert!(Path::new(&result.cache_path).exists());
    assert!(index.contains("\"id\": \"codebase-graph/creating-ontograph\""));
    assert!(log.contains("\"result\":\"active\""));
    assert!(log.contains("\"id\":\"codebase-graph/creating-ontograph\""));
}

#[test]
fn run_km_import_for_project_records_canonical_scope() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    write_cited_source(&project_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    run_km_import_for_project(&source, &store_root, &project_root).expect("import succeeds");

    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(store_root.join("index.json")).expect("index exists"),
    )
    .expect("valid index");
    assert_eq!(
        index["modules"][0]["projectScope"],
        fs::canonicalize(project_root)
            .expect("canonical project")
            .display()
            .to_string()
    );
}

#[test]
fn run_km_import_rejects_invalid_package_without_log() {
    let tempdir = TempDir::new().expect("temp dir");
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");
    fs::write(source.join("facts.jsonl"), "{\n").expect("break facts");

    let error = run_km_import(&source, &store_root).expect_err("import should fail");
    let log_path = tempdir
        .path()
        .join(".ontocode/ontograph/km/_ops/import/single/progress.jsonl");

    assert!(matches!(error, KmImportError::InvalidKm(_)));
    assert!(!store_root.join("index.json").exists());
    assert!(!log_path.exists());
}

#[test]
fn agent_import_succeeds_when_post_activation_log_cannot_be_written() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    write_cited_source(&project_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let km_root = tempdir.path().join(".ontocode/ontograph/km");
    let store_root = km_root.join("_store");
    fs::create_dir_all(&km_root).expect("km root");
    fs::write(km_root.join("_ops"), "blocks log directory").expect("blocking file");

    let result =
        run_km_import_for_agent_candidate(&source, &store_root, &project_root, "thread-1", 5)
            .expect("activation remains successful when only progress logging fails");

    assert_eq!(result.status, "active");
    assert!(store_root.join("index.json").exists());
}

#[test]
fn run_km_import_batch_resumes_without_reimporting_successful_items() {
    let tempdir = TempDir::new().expect("temp dir");
    let first = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/creating-ontograph",
        "0.1.0",
        "bounded claim",
    );
    let second = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/using-ontograph",
        "0.1.0",
        "second claim",
    );
    let batch_dir = tempdir
        .path()
        .join(".ontocode/ontograph/km/_ops/import/batch-1");
    fs::create_dir_all(&batch_dir).expect("batch dir");
    fs::write(
        batch_dir.join("manifest.jsonl"),
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first.display().to_string()).expect("first row"),
            serde_json::json!({ "path": second.display().to_string() })
        ),
    )
    .expect("manifest");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let first_result =
        run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("first batch");
    let second_result =
        run_km_import_batch(&batch_dir, &store_root, /*resume*/ true).expect("resume batch");
    let progress = fs::read_to_string(batch_dir.join("progress.jsonl")).expect("progress");

    assert_eq!(
        first_result,
        KmImportBatchResult {
            batch_dir: batch_dir.display().to_string(),
            imported_count: 2,
            skipped_count: 0,
            failed_count: 0,
            store_root: store_root.display().to_string(),
        }
    );
    assert_eq!(second_result.imported_count, 0);
    assert_eq!(second_result.skipped_count, 2);
    assert_eq!(second_result.failed_count, 0);
    assert_eq!(progress.lines().count(), 2);
}

#[test]
fn run_km_import_batch_records_same_version_conflict_as_failed() {
    let tempdir = TempDir::new().expect("temp dir");
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let batch_dir = tempdir
        .path()
        .join(".ontocode/ontograph/km/_ops/import/batch-1");
    fs::create_dir_all(&batch_dir).expect("batch dir");
    fs::write(
        batch_dir.join("manifest.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({ "sourcePath": source.display().to_string() })
        ),
    )
    .expect("manifest");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("first import");
    fs::write(
        source.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"changed claim\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("change package");
    let result = run_km_import_batch(&batch_dir, &store_root, /*resume*/ true).expect("conflict");
    let progress = fs::read_to_string(batch_dir.join("progress.jsonl")).expect("progress");

    assert_eq!(result.imported_count, 0);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(progress.contains("\"result\":\"failed\""));
    assert!(progress.contains("KM version content conflict"));
}

#[test]
fn run_km_import_batch_for_project_records_canonical_scope() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_root = tempdir.path().join("project");
    write_cited_source(&project_root);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let batch_dir = tempdir
        .path()
        .join(".ontocode/ontograph/km/_ops/import/batch-1");
    fs::create_dir_all(&batch_dir).expect("batch dir");
    fs::write(
        batch_dir.join("manifest.jsonl"),
        format!("{}\n", serde_json::json!({ "path": source })),
    )
    .expect("manifest");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    run_km_import_batch_for_project(
        &batch_dir,
        &store_root,
        /*resume*/ false,
        &project_root,
    )
    .expect("batch import");

    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(store_root.join("index.json")).expect("index exists"),
    )
    .expect("valid index");
    assert_eq!(
        index["modules"][0]["projectScope"],
        fs::canonicalize(project_root)
            .expect("canonical project")
            .display()
            .to_string()
    );
}

#[test]
fn run_km_import_batch_resume_does_not_bypass_project_scope_conflicts() {
    let tempdir = TempDir::new().expect("temp dir");
    let project_a = tempdir.path().join("project-a");
    let project_b = tempdir.path().join("project-b");
    write_cited_source(&project_a);
    write_cited_source(&project_b);
    let source = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let batch_dir = tempdir
        .path()
        .join(".ontocode/ontograph/km/_ops/import/batch-1");
    fs::create_dir_all(&batch_dir).expect("batch dir");
    fs::write(
        batch_dir.join("manifest.jsonl"),
        format!("{}\n", serde_json::json!({ "path": source })),
    )
    .expect("manifest");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    run_km_import_batch_for_project(&batch_dir, &store_root, /*resume*/ false, &project_a)
        .expect("first batch import");
    let result =
        run_km_import_batch_for_project(&batch_dir, &store_root, /*resume*/ true, &project_b)
            .expect("conflict is recorded in batch progress");
    let progress = fs::read_to_string(batch_dir.join("progress.jsonl")).expect("progress");
    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(store_root.join("index.json")).expect("index exists"),
    )
    .expect("valid index");

    assert_eq!(result.imported_count, 0);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.failed_count, 1);
    assert!(progress.contains("KM project scope conflict"));
    assert_eq!(
        index["modules"][0]["projectScope"],
        fs::canonicalize(project_a)
            .expect("canonical project A")
            .display()
            .to_string()
    );
}

#[test]
fn truncate_error_preserves_utf8_boundaries() {
    let error = KmImportError::InvalidKm(vec!["\u{20ac}".repeat(200)]);

    let message = truncate_error(&error);

    assert!(message.ends_with("..."));
    assert!(message.len() <= 515);
}

/// Writes a batch manifest listing the given package paths, one row per line.
fn write_batch_manifest(batch_dir: &Path, sources: &[&Path]) {
    fs::create_dir_all(batch_dir).expect("batch dir");
    let rows = sources
        .iter()
        .map(|source| serde_json::to_string(&source.display().to_string()).expect("manifest row"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(batch_dir.join("manifest.jsonl"), format!("{rows}\n")).expect("manifest");
}

// Backlog 73: resume is keyed by package content hash, so moving a package
// directory does not cause a reimport.
#[test]
fn batch_resume_skips_by_content_hash_not_by_path() {
    let tempdir = TempDir::new().expect("temp dir");
    let original = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    write_batch_manifest(&batch_dir, &[original.as_path()]);
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let first = run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("first");
    assert_eq!(first.imported_count, 1);

    // Move the package elsewhere and repoint the manifest at the new path.
    let moved = tempdir.path().join("relocated-km");
    fs::rename(&original, &moved).expect("relocate package");
    write_batch_manifest(&batch_dir, &[moved.as_path()]);

    let resumed = run_km_import_batch(&batch_dir, &store_root, /*resume*/ true).expect("resume");

    assert_eq!(resumed.imported_count, 0);
    assert_eq!(resumed.skipped_count, 1, "identical content is skipped");
}

// Backlog 74: one invalid package fails its own row without halting the batch.
#[test]
fn batch_with_one_invalid_package_continues_and_records_failure() {
    let tempdir = TempDir::new().expect("temp dir");
    let good = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/good",
        "0.1.0",
        "bounded claim",
    );
    let broken = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/broken",
        "0.1.0",
        "bounded claim",
    );
    fs::write(broken.join("km.toml"), "not = valid = toml").expect("break manifest");
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    write_batch_manifest(&batch_dir, &[broken.as_path(), good.as_path()]);
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let result = run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("batch");

    assert_eq!(result.failed_count, 1);
    assert_eq!(
        result.imported_count, 1,
        "a failed row must not halt the batch"
    );
}

// Backlog 75: the progress log is append-only across a resume.
#[test]
fn batch_progress_log_is_append_only_across_resume() {
    let tempdir = TempDir::new().expect("temp dir");
    let first = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/one",
        "0.1.0",
        "bounded claim",
    );
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    write_batch_manifest(&batch_dir, &[first.as_path()]);
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("first");
    let after_first = fs::read_to_string(batch_dir.join("progress.jsonl")).expect("progress");

    let second = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/two",
        "0.1.0",
        "second claim",
    );
    write_batch_manifest(&batch_dir, &[first.as_path(), second.as_path()]);
    run_km_import_batch(&batch_dir, &store_root, /*resume*/ true).expect("resume");
    let after_resume = fs::read_to_string(batch_dir.join("progress.jsonl")).expect("progress");

    assert!(
        after_resume.starts_with(&after_first),
        "resume must append, never rewrite earlier progress"
    );
    assert!(after_resume.len() > after_first.len());
}

// Backlog 76: a project-scope conflict marks its row failed and the batch
// keeps going.
#[test]
fn batch_for_project_scope_conflict_marks_item_failed_without_halting() {
    let tempdir = TempDir::new().expect("temp dir");
    let conflicting = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/scoped",
        "0.1.0",
        "bounded claim",
    );
    let other = write_valid_km_with_id(
        tempdir.path(),
        "codebase-graph/other",
        "0.1.0",
        "bounded claim",
    );
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");
    let first_scope = tempdir.path().join("project-a");
    let second_scope = tempdir.path().join("project-b");
    write_cited_source(&first_scope);
    write_cited_source(&second_scope);

    // Claim the package under the first scope.
    let claim_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b0");
    write_batch_manifest(&claim_dir, &[conflicting.as_path()]);
    run_km_import_batch_for_project(&claim_dir, &store_root, /*resume*/ false, &first_scope)
        .expect("claim batch");

    // Re-import the same package under a different scope, alongside a fresh one.
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    write_batch_manifest(&batch_dir, &[conflicting.as_path(), other.as_path()]);
    let result = run_km_import_batch_for_project(
        &batch_dir,
        &store_root,
        /*resume*/ false,
        &second_scope,
    )
    .expect("second batch");

    assert_eq!(result.failed_count, 1);
    assert_eq!(result.imported_count, 1);
}

// Backlog 77: an empty manifest succeeds with zero counted items.
#[test]
fn empty_batch_manifest_succeeds_with_zero_items() {
    let tempdir = TempDir::new().expect("temp dir");
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    fs::create_dir_all(&batch_dir).expect("batch dir");
    fs::write(batch_dir.join("manifest.jsonl"), "\n\n").expect("blank manifest");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let result = run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("batch");

    assert_eq!(result.imported_count, 0);
    assert_eq!(result.skipped_count, 0);
    assert_eq!(result.failed_count, 0);
}

// Backlog 78: the batch is manifest-driven, so unrelated files sitting in the
// batch directory are never treated as packages.
#[test]
fn batch_dir_with_non_manifest_entries_ignores_them_deterministically() {
    let tempdir = TempDir::new().expect("temp dir");
    let listed = write_valid_km(tempdir.path(), "0.1.0", "bounded claim");
    let batch_dir = tempdir.path().join(".ontocode/ontograph/km/_ops/import/b1");
    write_batch_manifest(&batch_dir, &[listed.as_path()]);
    // Junk beside the manifest, including a package-shaped directory.
    fs::write(batch_dir.join("notes.txt"), "scratch").expect("junk file");
    let unlisted = batch_dir.join("unlisted-km");
    fs::create_dir_all(unlisted.join("examples")).expect("unlisted dir");
    let store_root = tempdir.path().join(".ontocode/ontograph/km/_store");

    let result = run_km_import_batch(&batch_dir, &store_root, /*resume*/ false).expect("batch");

    assert_eq!(result.imported_count, 1);
    assert_eq!(result.failed_count, 0);
}

// Backlog 79: truncation lands exactly on the cap with multibyte input.
#[test]
fn truncate_error_caps_at_limit_with_multibyte_input() {
    // Three-byte characters cannot end on byte 512, so the helper backs up to
    // the nearest boundary at 510 and appends the ellipsis.
    let error = KmImportError::InvalidKm(vec!["\u{754c}".repeat(400)]);

    let message = truncate_error(&error);

    assert!(message.ends_with("..."));
    assert!(std::str::from_utf8(message.as_bytes()).is_ok());
    let body = message.trim_end_matches("...");
    assert!(body.len() <= 512);
    assert!(body.len() >= 510, "truncation stays near the cap");
}

fn write_valid_km(root: &Path, version: &str, claim: &str) -> std::path::PathBuf {
    write_valid_km_with_id(root, "codebase-graph/creating-ontograph", version, claim)
}

fn write_valid_km_with_id(root: &Path, id: &str, version: &str, claim: &str) -> std::path::PathBuf {
    let km_root = root.join(format!("{}-{version}", id.replace('/', "-")));
    fs::create_dir_all(km_root.join("examples")).expect("examples dir");
    fs::create_dir_all(km_root.join("tests")).expect("tests dir");
    fs::write(km_root.join("km.toml"), valid_manifest(id, version)).expect("manifest");
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

/// Creates the `doc` file that fixture packages cite as `silver:doc`.
///
/// Citation validation resolves the source against the project scope, so the
/// cited path has to exist under that root for a scoped import to succeed.
fn write_cited_source(project_root: &Path) {
    fs::create_dir_all(project_root).expect("project root");
    fs::write(project_root.join("doc"), "doc").expect("cited source");
}

fn valid_manifest(id: &str, version: &str) -> String {
    format!(
        "id = \"{id}\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
    )
}
