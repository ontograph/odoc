use pretty_assertions::assert_eq;

use super::*;
use crate::indexer::RustIndexRequest;
use crate::indexer::index_rust_workspace;
use crate::manifest::SOURCE_INDEX_FILE;

/// Builds a two-file fixture: one Rust file that supplies index targets, and
/// two Markdown files that exercise headings, an explicit requirement marker, a
/// resolved link, and an unresolved reference.
fn fixture_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn indexed_symbol() {}\n",
    )
    .expect("write rust");
    std::fs::write(
        dir.path().join("design.md"),
        "# Design\n\n## Uses `src/lib.rs` and `indexed_symbol`\n\n## Mentions `not_in_index`\n",
    )
    .expect("write design");
    std::fs::write(
        dir.path().join("requirements.md"),
        "# Requirements\n\nRequirement: REQ-2 must hold\n\n## Requirement: REQ-1 must hold\n\n```\nRequirement: REQ-3 must be ignored\n# not a heading\n```\n",
    )
    .expect("write requirements");
    dir
}

fn build_fixture_index(dir: &tempfile::TempDir) -> DocsIndexArtifact {
    let request = RustIndexRequest {
        repo_root: dir.path().to_path_buf(),
        artifact_root: OntographArtifactRoot::new(dir.path()),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: "0.0.0-test".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    };
    let (_artifact, _limits) = index_rust_workspace(request).expect("index workspace");
    let root = OntographArtifactRoot::new(dir.path());
    let path = root
        .generation_file_path("gen-1", DOCS_INDEX_FILE)
        .expect("docs index path");
    let contents = std::fs::read_to_string(path).expect("read docs index");
    serde_json::from_str::<DocsIndexArtifact>(&contents).expect("deserialize docs index")
}

#[test]
fn indexing_publishes_a_docs_index_the_loader_accepts() {
    let dir = fixture_workspace();
    let artifact = build_fixture_index(&dir);

    // The loader rejects an artifact whose completion timestamp is absent, and
    // rejects one whose identity does not match the active source generation.
    assert_eq!(artifact.manifest.schema_version, DOCS_SCHEMA_VERSION);
    assert_eq!(
        artifact.manifest.completed_at_unix_seconds,
        Some(1_725_000_000)
    );
    assert_eq!(artifact.manifest.source_index_generation_id, "gen-1");
    assert!(
        artifact
            .manifest
            .source_index_content_hash
            .starts_with("sha256:")
    );

    let paths = artifact
        .documents
        .iter()
        .map(|document| document.path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec!["design.md".to_string(), "requirements.md".to_string()]
    );
}

#[test]
fn links_record_resolved_targets_and_skip_unresolved_references() {
    let dir = fixture_workspace();
    let artifact = build_fixture_index(&dir);

    let targets = artifact
        .links
        .iter()
        .map(|link| (link.target_kind.clone(), link.target.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        targets,
        vec![
            ("path".to_string(), "src/lib.rs".to_string()),
            ("symbol".to_string(), "indexed_symbol".to_string()),
        ]
    );
    // `not_in_index` resolves against nothing, so it is skipped rather than
    // guessed into a fabricated link.
    assert!(
        !artifact
            .links
            .iter()
            .any(|link| link.target == "not_in_index")
    );
}

#[test]
fn requirement_ids_are_recorded_only_from_the_explicit_marker() {
    let dir = fixture_workspace();
    let artifact = build_fixture_index(&dir);

    let requirement_ids = artifact
        .facts
        .iter()
        .filter_map(|fact| fact.requirement_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        requirement_ids,
        vec!["REQ-2".to_string(), "REQ-1".to_string()]
    );

    let line_requirement = artifact
        .facts
        .iter()
        .find(|fact| fact.requirement_id.as_deref() == Some("REQ-2"))
        .expect("line requirement");
    assert_eq!(line_requirement.heading_id, None);
    assert_eq!(line_requirement.line_start, 3);
    assert_eq!(line_requirement.line_end, 3);
    assert_eq!(line_requirement.text, "Requirement: REQ-2 must hold");
}

#[test]
fn headings_inside_fenced_blocks_are_not_recorded() {
    let dir = fixture_workspace();
    let artifact = build_fixture_index(&dir);

    let requirements_document = artifact
        .documents
        .iter()
        .find(|document| document.path == "requirements.md")
        .expect("requirements document");
    let titles = requirements_document
        .headings
        .iter()
        .map(|heading| heading.title.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        titles,
        vec![
            "Requirements".to_string(),
            "Requirement: REQ-1 must hold".to_string(),
        ]
    );
}

#[test]
fn artifact_round_trips_through_the_loader_field_set() {
    let dir = fixture_workspace();
    let artifact = build_fixture_index(&dir);

    // `deny_unknown_fields` on every artifact struct means a Serialize/
    // Deserialize mismatch fails here rather than silently at load time.
    let serialized = serde_json::to_string(&artifact).expect("serialize");
    let round_tripped =
        serde_json::from_str::<DocsIndexArtifact>(&serialized).expect("deserialize");
    assert_eq!(round_tripped, artifact);
}

#[test]
fn document_cap_skips_sidecar_and_reports_limit() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn indexed_symbol() {}\n",
    )
    .expect("write rust");
    for index in 0..=MAX_DOCS_DOCUMENTS {
        std::fs::write(dir.path().join(format!("doc_{index}.md")), "# D\n")
            .expect("write markdown");
    }

    let request = RustIndexRequest {
        repo_root: dir.path().to_path_buf(),
        artifact_root: OntographArtifactRoot::new(dir.path()),
        generation_id: "gen-cap".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: "0.0.0-test".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    };
    let (_artifact, limits) = index_rust_workspace(request).expect("index workspace");
    let root = OntographArtifactRoot::new(dir.path());

    assert!(
        root.generation_file_path("gen-cap", SOURCE_INDEX_FILE)
            .expect("source index path")
            .exists()
    );
    assert!(
        root.generation_manifest_path("gen-cap")
            .expect("manifest path")
            .exists()
    );
    assert!(
        !root
            .generation_file_path("gen-cap", DOCS_INDEX_FILE)
            .expect("docs index path")
            .exists()
    );
    assert_eq!(limits.len(), 1);
    assert!(limits[0].contains(&MAX_DOCS_DOCUMENTS.to_string()));
}

#[test]
fn document_collection_retains_only_one_overflow_sentinel() {
    let dir = tempfile::tempdir().expect("tempdir");
    for index in 0..=MAX_DOCS_DOCUMENTS + 100 {
        std::fs::write(dir.path().join(format!("doc_{index}.md")), "# D\n")
            .expect("write markdown");
    }

    let files = collect_markdown_files(dir.path()).expect("collect markdown files");

    assert_eq!(files.len(), MAX_DOCS_DOCUMENTS + 1);
}

#[test]
fn oversized_markdown_skips_sidecar_before_extraction() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("large.md"),
        vec![b'#'; MAX_DOCS_FILE_BYTES + 1],
    )
    .expect("write oversized markdown");
    let root = OntographArtifactRoot::new(dir.path());

    let request = DocsIndexRequest {
        repo_root: dir.path(),
        artifact_root: &root,
        generation_id: "gen-large",
        source_index_content_hash: "sha256:test",
        implementation_version: "0.0.0-test",
        timestamp_unix_seconds: 1_725_000_000,
    };
    let limits = build_and_publish_docs_index(
        request,
        &SourceIndexArtifact {
            schema_version: 1,
            files: Vec::new(),
            relationships: Vec::new(),
            processes: Vec::new(),
            routes: Vec::new(),
            unresolved_calls: Vec::new(),
            resource_facts: Vec::new(),
            resource_facts_truncated: false,
        },
    )
    .expect("index oversized markdown");

    assert_eq!(limits.len(), 1);
    assert!(limits[0].contains("MAX_DOCS_FILE_BYTES"));
    assert!(
        !root
            .generation_file_path("gen-large", DOCS_INDEX_FILE)
            .expect("docs index path")
            .exists()
    );
}

#[test]
fn capped_json_writer_never_retains_bytes_over_limit() {
    let mut writer = CappedJsonWriter::new(3);
    std::io::Write::write_all(&mut writer, b"abcd").expect("write capped bytes");

    assert!(writer.exceeded());
    assert_eq!(writer.into_bytes(), b"abc");
}

#[test]
fn producer_skip_set_excludes_node_modules_ontocode_and_archive() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn indexed_symbol() {}\n",
    )
    .expect("write rust");
    std::fs::write(dir.path().join("README.md"), "# Readme\n").expect("write readme");
    std::fs::create_dir_all(dir.path().join("node_modules")).expect("mkdir node_modules");
    std::fs::write(dir.path().join("node_modules/x.md"), "# X\n").expect("write node");
    std::fs::create_dir_all(dir.path().join(".ontocode")).expect("mkdir ontocode");
    std::fs::write(dir.path().join(".ontocode/y.md"), "# Y\n").expect("write ontocode");
    std::fs::create_dir_all(dir.path().join(".memory-bank/archive")).expect("mkdir archive");
    std::fs::write(dir.path().join(".memory-bank/archive/z.md"), "# Z\n").expect("write archive");
    std::fs::create_dir_all(dir.path().join("nested/archive")).expect("mkdir nested archive");
    std::fs::write(dir.path().join("nested/archive/kept.md"), "# Kept\n").expect("write kept");

    let request = RustIndexRequest {
        repo_root: dir.path().to_path_buf(),
        artifact_root: OntographArtifactRoot::new(dir.path()),
        generation_id: "gen-skip".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: "0.0.0-test".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    };
    let (_artifact, _limits) = index_rust_workspace(request).expect("index workspace");
    let root = OntographArtifactRoot::new(dir.path());
    let contents = std::fs::read_to_string(
        root.generation_file_path("gen-skip", DOCS_INDEX_FILE)
            .expect("docs index path"),
    )
    .expect("read docs index");
    let artifact = serde_json::from_str::<DocsIndexArtifact>(&contents).expect("docs index json");
    let mut paths = artifact
        .documents
        .into_iter()
        .map(|document| document.path)
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(
        paths,
        vec![
            "README.md".to_string(),
            "nested/archive/kept.md".to_string()
        ]
    );
}
