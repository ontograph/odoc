use pretty_assertions::assert_eq;
use std::process::Command;

use crate::manifest::OntographArtifactRoot;
use crate::manifest::OntographManifest;
use crate::manifest::SOURCE_INDEX_FILE;
use crate::query::BoundedItems;
use crate::query::DiffImpactFile;
use crate::query::DiffImpactProcess;
use crate::query::DiffImpactRequest;
use crate::query::DiffImpactResult;
use crate::query::DiffImpactSummary;
use crate::query::DiffImpactSymbol;
use crate::query::EvidenceItem;
use crate::query::EvidenceRequest;
use crate::query::EvidenceResult;
use crate::query::GraphSccInputItem;
use crate::query::GraphSccInputRequest;
use crate::query::GraphSccInputResult;
use crate::query::ImpactItem;
use crate::query::ModuleContextItem;
use crate::query::ModuleContextRequest;
use crate::query::ModuleContextResult;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::QueryError;
use crate::query::RepoMapFileItem;
use crate::query::RepoMapRequest;
use crate::query::RepoMapResult;
use crate::query::SymbolImpactRequest;
use crate::query::SymbolImpactResult;
use crate::query::SymbolSearchItem;
use crate::query::SymbolSearchRequest;
use crate::query::SymbolSearchResult;
use crate::query::UnresolvedEvidenceItem;

use super::*;

#[test]
fn indexes_rust_files_deterministically_and_ignores_private_dirs() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/z.rs"),
        "pub struct Zebra;\n    fn nested() {}\nconst LIMIT: usize = 1;\n",
    );
    write_file(
        dir.path().join("src/a.rs"),
        "pub(crate) enum Mode { A }\ntrait Local {}\nfn helper() {}\n",
    );
    write_file(dir.path().join("README.md"), "# ignored\n");
    write_file(dir.path().join(".git/ignored.rs"), "pub struct Ignored;\n");
    write_file(
        dir.path().join(".ontocode/ontograph/ignored.rs"),
        "pub struct Ignored;\n",
    );
    write_file(
        dir.path().join("target/ignored.rs"),
        "pub struct Ignored;\n",
    );
    write_file(dir.path().join("tmp/ignored.rs"), "pub struct Ignored;\n");
    write_file(
        dir.path().join("third_party/ignored.rs"),
        "pub struct Ignored;\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact,
        SourceIndexArtifact {
            schema_version: 1,
            files: vec![
                IndexedFile {
                    node_id: "file:src/a.rs".to_string(),
                    path: "src/a.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![
                        IndexedSymbol {
                            node_id: "symbol:src/a.rs:1:enum:Mode".to_string(),
                            name: "Mode".to_string(),
                            kind: "enum".to_string(),
                            line: 1,
                        },
                        IndexedSymbol {
                            node_id: "symbol:src/a.rs:2:trait:Local".to_string(),
                            name: "Local".to_string(),
                            kind: "trait".to_string(),
                            line: 2,
                        },
                        IndexedSymbol {
                            node_id: "symbol:src/a.rs:3:fn:helper".to_string(),
                            name: "helper".to_string(),
                            kind: "fn".to_string(),
                            line: 3,
                        },
                    ],
                },
                IndexedFile {
                    node_id: "file:src/z.rs".to_string(),
                    path: "src/z.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![
                        IndexedSymbol {
                            node_id: "symbol:src/z.rs:1:struct:Zebra".to_string(),
                            name: "Zebra".to_string(),
                            kind: "struct".to_string(),
                            line: 1,
                        },
                        IndexedSymbol {
                            node_id: "symbol:src/z.rs:2:fn:nested".to_string(),
                            name: "nested".to_string(),
                            kind: "fn".to_string(),
                            line: 2,
                        },
                        IndexedSymbol {
                            node_id: "symbol:src/z.rs:3:const:LIMIT".to_string(),
                            name: "LIMIT".to_string(),
                            kind: "const".to_string(),
                            line: 3,
                        },
                    ],
                },
            ],
            relationships: Vec::new(),
            processes: Vec::new(),
            routes: Vec::new(),
            unresolved_calls: Vec::new(),
            resource_facts: Vec::new(),
            resource_facts_truncated: false,
        }
    );
}

#[test]
fn indexes_nested_rust_functions_without_indexing_comments_or_strings() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "pub struct Demo;\nimpl Demo {\n    fn method() {\n        helper();\n    }\n}\nmod nested {\n    fn helper() {}\n}\n// fn commented() {}\nconst TEXT: &str = \"fn stringy() {}\";\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.files[0].symbols,
        vec![
            IndexedSymbol {
                node_id: "symbol:lib.rs:1:struct:Demo".to_string(),
                name: "Demo".to_string(),
                kind: "struct".to_string(),
                line: 1,
            },
            IndexedSymbol {
                node_id: "symbol:lib.rs:3:fn:method".to_string(),
                name: "method".to_string(),
                kind: "fn".to_string(),
                line: 3,
            },
            IndexedSymbol {
                node_id: "symbol:lib.rs:7:mod:nested".to_string(),
                name: "nested".to_string(),
                kind: "mod".to_string(),
                line: 7,
            },
            IndexedSymbol {
                node_id: "symbol:lib.rs:8:fn:helper".to_string(),
                name: "helper".to_string(),
                kind: "fn".to_string(),
                line: 8,
            },
            IndexedSymbol {
                node_id: "symbol:lib.rs:11:const:TEXT".to_string(),
                name: "TEXT".to_string(),
                kind: "const".to_string(),
                line: 11,
            },
        ]
    );
}

#[test]
fn writes_source_index_artifact_and_manifest_counts() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "pub mod graph;\ntype NodeId = u64;\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let root = OntographArtifactRoot::new(dir.path());
    let source_index_path = root
        .generation_file_path("gen-1", SOURCE_INDEX_FILE)
        .expect("source index path");
    let written_artifact =
        std::fs::read_to_string(source_index_path).expect("source index artifact");
    assert_eq!(
        serde_json::from_str::<SourceIndexArtifact>(&written_artifact).expect("artifact json"),
        artifact
    );

    let manifest_path = root
        .generation_manifest_path("gen-1")
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest.active_generation_id, "gen-1");
    assert_eq!(manifest.completed_generations[0].counts.files, 1);
    assert_eq!(manifest.completed_generations[0].counts.symbols, 2);
    assert_eq!(manifest.completed_generations[0].counts.relationships, 0);
}

#[test]
fn source_index_error_reports_relative_path_without_publication() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source_path = dir.path().join("src/invalid.rs");
    std::fs::create_dir_all(source_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&source_path, [0xff]).expect("write invalid UTF-8 source");
    let root = OntographArtifactRoot::new(dir.path());

    assert_eq!(
        index_rust_workspace(sample_request(dir.path())),
        Err(RustIndexError::ReadFile("src/invalid.rs".to_string()))
    );
    assert!(
        !root
            .generation_file_path("gen-1", SOURCE_INDEX_FILE)
            .expect("source index path")
            .exists()
    );
    assert!(
        !root
            .generation_manifest_path("gen-1")
            .expect("manifest path")
            .exists()
    );
    assert!(!root.current_pointer_path().exists());
}

#[test]
fn source_index_error_redacts_secret_like_filename() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source_path = dir.path().join("src/token=abcdefghijklmnop.rs");
    std::fs::create_dir_all(source_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&source_path, [0xff]).expect("write invalid UTF-8 source");

    assert_eq!(
        index_rust_workspace(sample_request(dir.path())),
        Err(RustIndexError::ReadFile(
            "src/token=[REDACTED_SECRET]".to_string()
        ))
    );
}

#[test]
fn source_index_error_neutralizes_control_characters() {
    assert_eq!(
        sanitize_read_file_error_path("src/line\nbreak\t.rs"),
        "src/line?break?.rs"
    );
}

#[test]
fn source_index_error_bounds_long_utf8_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let segment = "é".repeat(60);
    let mut source_path = dir.path().to_path_buf();
    for _ in 0..4 {
        source_path.push(&segment);
    }
    source_path.push("invalid.rs");
    std::fs::create_dir_all(source_path.parent().expect("parent")).expect("create parent");
    std::fs::write(&source_path, [0xff]).expect("write invalid UTF-8 source");

    let Err(RustIndexError::ReadFile(path)) = index_rust_workspace(sample_request(dir.path()))
    else {
        panic!("expected read-file error");
    };
    assert!(path.len() <= MAX_READ_FILE_ERROR_PATH_BYTES);
    assert!(path.starts_with(&segment));
    assert!(path.ends_with("..."));
    assert!(!path.contains(dir.path().to_string_lossy().as_ref()));
}

#[test]
fn indexes_same_file_function_call_relationships() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "mod graph;\nfn caller() {\n    callee();\n}\nfn callee() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:lib.rs:2:caller:lib.rs:5:callee:lib.rs:3".to_string(),
            from_node_id: "symbol:lib.rs:2:fn:caller".to_string(),
            from_symbol: "caller".to_string(),
            from_path: "lib.rs".to_string(),
            from_line: 2,
            to_node_id: "symbol:lib.rs:5:fn:callee".to_string(),
            to_symbol: "callee".to_string(),
            to_path: "lib.rs".to_string(),
            to_line: 5,
            kind: "CALLS".to_string(),
            path: "lib.rs".to_string(),
            line: 3,
        }]
    );

    let root = OntographArtifactRoot::new(dir.path());
    let manifest_path = root
        .generation_manifest_path("gen-1")
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest.completed_generations[0].counts.relationships, 1);
    assert_eq!(manifest.completed_generations[0].counts.edges, 1);
}

#[test]
fn indexes_test_function_calls_as_test_relationships() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn target() -> usize {\n    1\n}\n\n#[test]\nfn target_is_tested() {\n    target();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:lib.rs:6:target_is_tested:lib.rs:1:target:lib.rs:7".to_string(),
            from_node_id: "symbol:lib.rs:6:fn:target_is_tested".to_string(),
            from_symbol: "target_is_tested".to_string(),
            from_path: "lib.rs".to_string(),
            from_line: 6,
            to_node_id: "symbol:lib.rs:1:fn:target".to_string(),
            to_symbol: "target".to_string(),
            to_path: "lib.rs".to_string(),
            to_line: 1,
            kind: "TESTS".to_string(),
            path: "lib.rs".to_string(),
            line: 7,
        }]
    );
    assert!(artifact.processes.is_empty());
}

#[test]
fn indexes_cross_file_function_call_relationships() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "mod b;\nfn caller() {\n    helper();\n}\n",
    );
    write_file(dir.path().join("src/b.rs"), "fn helper() {}\n");

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:src/a.rs:2:caller:src/b.rs:1:helper:src/a.rs:3".to_string(),
            from_node_id: "symbol:src/a.rs:2:fn:caller".to_string(),
            from_symbol: "caller".to_string(),
            from_path: "src/a.rs".to_string(),
            from_line: 2,
            to_node_id: "symbol:src/b.rs:1:fn:helper".to_string(),
            to_symbol: "helper".to_string(),
            to_path: "src/b.rs".to_string(),
            to_line: 1,
            kind: "CALLS".to_string(),
            path: "src/a.rs".to_string(),
            line: 3,
        }]
    );
}

#[test]
fn graph_depth_records_stable_node_ids_without_absolute_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "mod b;\nfn caller() {\n    helper();\n}\n",
    );
    write_file(dir.path().join("src/b.rs"), "fn helper() {}\n");

    let (first, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let (second, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace again");

    assert_eq!(first.files, second.files);
    assert_eq!(first.relationships, second.relationships);
    assert_eq!(first.files[0].node_id, "file:src/a.rs".to_string());
    let caller = first.files[0]
        .symbols
        .iter()
        .find(|symbol| symbol.name == "caller")
        .expect("caller symbol");
    assert_eq!(caller.node_id, "symbol:src/a.rs:2:fn:caller".to_string());
    assert_eq!(
        first.relationships[0].relationship_id,
        "call:src/a.rs:2:caller:src/b.rs:1:helper:src/a.rs:3".to_string()
    );
    let serialized = serde_json::to_string(&first).expect("artifact json");
    assert!(!serialized.contains(dir.path().to_string_lossy().as_ref()));
}

#[test]
fn graph_depth_records_unresolved_calls_for_missing_and_ambiguous_targets() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "fn caller() {\n    missing();\n    run();\n}\nfn run() {}\n",
    );
    write_file(dir.path().join("src/b.rs"), "fn run() {}\n");

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.relationships.is_empty());
    assert_eq!(
        artifact.unresolved_calls,
        vec![
            IndexedUnresolvedCall {
                unresolved_call_id: "unresolved-call:src/a.rs:1:caller:missing:src/a.rs:2"
                    .to_string(),
                from_node_id: "symbol:src/a.rs:1:fn:caller".to_string(),
                from_symbol: "caller".to_string(),
                from_path: "src/a.rs".to_string(),
                from_line: 1,
                target_symbol: "missing".to_string(),
                path: "src/a.rs".to_string(),
                line: 2,
                reason: "no_indexed_definition".to_string(),
            },
            IndexedUnresolvedCall {
                unresolved_call_id: "unresolved-call:src/a.rs:1:caller:run:src/a.rs:3".to_string(),
                from_node_id: "symbol:src/a.rs:1:fn:caller".to_string(),
                from_symbol: "caller".to_string(),
                from_path: "src/a.rs".to_string(),
                from_line: 1,
                target_symbol: "run".to_string(),
                path: "src/a.rs".to_string(),
                line: 3,
                reason: "ambiguous_target".to_string(),
            },
        ]
    );
}

#[test]
fn parsed_receiver_and_scoped_calls_remain_ambiguous_when_names_collide() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "struct Type;\nimpl Type {\n    fn run(&self) {}\n}\n\nfn caller() {\n    let client = Type;\n    client.run();\n    Type::run();\n    self.run();\n}\nfn run() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].to_symbol, "run");
    assert_eq!(artifact.relationships[0].line, 9);
    assert_eq!(
        artifact
            .unresolved_calls
            .iter()
            .map(|call| (call.line, call.reason.as_str()))
            .collect::<Vec<_>>(),
        vec![(8, "ambiguous_target"), (10, "ambiguous_target")]
    );
    assert!(artifact.relationships.iter().all(|relationship| {
        relationship.line == 9 && relationship.to_node_id == "symbol:src/a.rs:3:fn:run"
    }));
    assert!(
        artifact
            .unresolved_calls
            .iter()
            .all(|call| { call.line != 8 && call.line != 10 || call.reason == "ambiguous_target" })
    );
}

#[test]
fn parsed_unique_qualified_call_resolves() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "struct Type;\nimpl Type {\n    fn run(&self) {}\n}\n\nfn caller() {\n    Type::run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(artifact.unresolved_calls, Vec::new());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].to_symbol, "run");
    assert_eq!(artifact.relationships[0].line, 7);
}

#[test]
fn parsed_nested_function_keeps_its_own_call_owner() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "fn outer() {\n    fn inner() {\n        target();\n    }\n    inner();\n}\nfn target() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.files.iter().any(|file| {
        file.path == "src/a.rs"
            && file
                .symbols
                .iter()
                .any(|symbol| symbol.name == "inner" && symbol.line == 2)
    }));
    assert!(artifact.relationships.iter().any(|relationship| {
        relationship.from_symbol == "inner"
            && relationship.from_line == 2
            && relationship.to_symbol == "target"
    }));
    assert!(artifact.relationships.iter().any(|relationship| {
        relationship.from_symbol == "outer"
            && relationship.from_line == 1
            && relationship.to_symbol == "inner"
    }));
}

#[test]
fn parsed_recursion_produces_self_edge() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "fn recur() {\n    recur();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].from_symbol, "recur");
    assert_eq!(artifact.relationships[0].to_symbol, "recur");
}

#[test]
fn parsed_import_resolves_unique_cross_file_function() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("src/helper.rs"), "pub fn run() {}\n");
    write_file(
        dir.path().join("src/main.rs"),
        "use crate::helper::run;\nfn caller() {\n    run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].from_symbol, "caller");
    assert_eq!(artifact.relationships[0].to_symbol, "run");
    assert_eq!(artifact.relationships[0].to_path, "src/helper.rs");
}

#[test]
fn parsed_unmapped_import_does_not_hide_same_file_definition() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/main.rs"),
        "use external::run;\nfn caller() {\n    run();\n}\nfn run() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].from_symbol, "caller");
    assert_eq!(artifact.relationships[0].to_symbol, "run");
    assert_eq!(artifact.relationships[0].to_path, "src/main.rs");
}

#[test]
fn parsed_renamed_import_resolves_the_imported_definition() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("src/helper.rs"), "pub fn actual() {}\n");
    write_file(
        dir.path().join("src/main.rs"),
        "use crate::helper::actual as run;\nfn caller() {\n    run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].to_symbol, "actual");
    assert_eq!(artifact.relationships[0].to_path, "src/helper.rs");
}

#[test]
fn parsed_relative_self_import_resolves_from_the_importing_module() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("src/pkg/helper.rs"), "pub fn run() {}\n");
    write_file(
        dir.path().join("src/pkg/mod.rs"),
        "use self::helper::run;\nfn caller() {\n    run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].to_path, "src/pkg/helper.rs");
}

#[test]
fn parsed_relative_super_import_resolves_from_the_parent_module() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("src/helper.rs"), "pub fn run() {}\n");
    write_file(
        dir.path().join("src/pkg/child.rs"),
        "use super::helper::run;\nfn caller() {\n    run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.unresolved_calls.is_empty());
    assert_eq!(artifact.relationships.len(), 1);
    assert_eq!(artifact.relationships[0].to_path, "src/helper.rs");
}

#[test]
fn parsed_multiple_resolvable_imports_keep_same_name_ambiguous() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("src/a.rs"), "pub fn run() {}\n");
    write_file(dir.path().join("src/b.rs"), "pub fn run() {}\n");
    write_file(
        dir.path().join("src/main.rs"),
        "use crate::a::run;\nuse crate::b::run;\nfn caller() {\n    run();\n}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert!(artifact.relationships.is_empty());
    assert_eq!(artifact.unresolved_calls.len(), 1);
    assert_eq!(artifact.unresolved_calls[0].reason, "ambiguous_target");
}

#[test]
fn unparsable_rust_file_uses_line_scanner_fallback() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "fn caller() {\n    callee();\n}\nfn callee( {\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:src/a.rs:1:caller:src/a.rs:4:callee:src/a.rs:2".to_string(),
            from_node_id: "symbol:src/a.rs:1:fn:caller".to_string(),
            from_symbol: "caller".to_string(),
            from_path: "src/a.rs".to_string(),
            from_line: 1,
            to_node_id: "symbol:src/a.rs:4:fn:callee".to_string(),
            to_symbol: "callee".to_string(),
            to_path: "src/a.rs".to_string(),
            to_line: 4,
            kind: "CALLS".to_string(),
            path: "src/a.rs".to_string(),
            line: 2,
        }]
    );
}

#[test]
fn active_source_index_store_reports_bounded_graph_helpers() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 1, 2048);
    let caller_node_id = "symbol:lib.rs:1:fn:caller".to_string();
    let callee_node_id = "symbol:lib.rs:4:fn:callee".to_string();
    assert_eq!(
        store.graph_scc_inputs(GraphSccInputRequest {
            budget: budget.clone(),
        }),
        Ok(GraphSccInputResult {
            edges: BoundedItems {
                items: vec![GraphSccInputItem {
                    relationship_id: "call:lib.rs:1:caller:lib.rs:4:callee:lib.rs:2".to_string(),
                    from_node_id: caller_node_id,
                    from_symbol: "caller".to_string(),
                    from_path: "lib.rs".to_string(),
                    to_node_id: callee_node_id,
                    to_symbol: "callee".to_string(),
                    to_path: "lib.rs".to_string(),
                }],
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn indexes_same_file_call_relationships_as_processes() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    middle();\n}\nfn middle() {\n    callee();\n}\nfn callee() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.processes,
        vec![
            IndexedProcess {
                node_id: "process:lib.rs:caller:caller>middle".to_string(),
                name: "caller -> middle".to_string(),
                entry_symbol: "caller".to_string(),
                path: "lib.rs".to_string(),
                steps: vec!["caller".to_string(), "middle".to_string()],
            },
            IndexedProcess {
                node_id: "process:lib.rs:middle:middle>callee".to_string(),
                name: "middle -> callee".to_string(),
                entry_symbol: "middle".to_string(),
                path: "lib.rs".to_string(),
                steps: vec!["middle".to_string(), "callee".to_string()],
            },
        ]
    );
}

#[test]
fn indexes_direct_axum_routes() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("server.rs"),
        "use axum::routing::{get, post};\nuse axum::Router;\nfn app() -> Router {\n    Router::new()\n        .route(\"/readyz\", get(readiness_handler))\n        .route(\"/api/users/:id/\", post(create_user))\n}\nfn readiness_handler() {}\nfn create_user() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.routes,
        vec![
            IndexedRoute {
                node_id: "route:GET:/readyz:server.rs:5:readiness_handler".to_string(),
                method: "GET".to_string(),
                route_path: "/readyz".to_string(),
                handler_symbol: "readiness_handler".to_string(),
                path: "server.rs".to_string(),
                line: 5,
            },
            IndexedRoute {
                node_id: "route:POST:/api/users/{param}:server.rs:6:create_user".to_string(),
                method: "POST".to_string(),
                route_path: "/api/users/{param}".to_string(),
                handler_symbol: "create_user".to_string(),
                path: "server.rs".to_string(),
                line: 6,
            },
        ]
    );

    let root = OntographArtifactRoot::new(dir.path());
    let manifest_path = root
        .generation_manifest_path("gen-1")
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("manifest"),
    )
    .expect("manifest json");
    assert_eq!(manifest.completed_generations[0].counts.nodes, 6);
}

#[test]
fn source_index_artifact_accepts_older_route_free_json() {
    let artifact = serde_json::from_str::<SourceIndexArtifact>(
        r#"{"schema_version":1,"files":[],"relationships":[],"processes":[]}"#,
    )
    .expect("legacy artifact json");

    assert_eq!(
        artifact,
        SourceIndexArtifact {
            schema_version: 1,
            files: Vec::new(),
            relationships: Vec::new(),
            processes: Vec::new(),
            routes: Vec::new(),
            unresolved_calls: Vec::new(),
            resource_facts: Vec::new(),
            resource_facts_truncated: false,
        }
    );
}

#[test]
fn source_index_artifact_bounds_resource_facts_and_marks_truncation() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut content = String::from("fn worker() {\n");
    for _ in 0..(MAX_RESOURCE_FACTS + 1) {
        content.push_str("    open();\n");
    }
    content.push_str("}\n");
    write_file(dir.path().join("lib.rs"), &content);

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(artifact.resource_facts.len(), MAX_RESOURCE_FACTS);
    assert!(artifact.resource_facts_truncated);
}

#[test]
fn relationship_index_ignores_comments_strings_and_duplicate_calls() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee(); callee();\n    // callee();\n    let text = \"callee()\";\n}\nfn callee() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:lib.rs:1:caller:lib.rs:6:callee:lib.rs:2".to_string(),
            from_node_id: "symbol:lib.rs:1:fn:caller".to_string(),
            from_symbol: "caller".to_string(),
            from_path: "lib.rs".to_string(),
            from_line: 1,
            to_node_id: "symbol:lib.rs:6:fn:callee".to_string(),
            to_symbol: "callee".to_string(),
            to_path: "lib.rs".to_string(),
            to_line: 6,
            kind: "CALLS".to_string(),
            path: "lib.rs".to_string(),
            line: 2,
        }]
    );
}

#[test]
fn relationship_index_keeps_multiline_function_signatures_until_body_opens() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller(\n    really_long_parameter_name: usize,\n)\nwhere\n    usize: Copy,\n{\n    callee();\n}\nfn callee() {}\n",
    );

    let (artifact, _limits) =
        index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    assert_eq!(
        artifact.relationships,
        vec![IndexedRelationship {
            relationship_id: "call:lib.rs:1:caller:lib.rs:9:callee:lib.rs:7".to_string(),
            from_node_id: "symbol:lib.rs:1:fn:caller".to_string(),
            from_symbol: "caller".to_string(),
            from_path: "lib.rs".to_string(),
            from_line: 1,
            to_node_id: "symbol:lib.rs:9:fn:callee".to_string(),
            to_symbol: "callee".to_string(),
            to_path: "lib.rs".to_string(),
            to_line: 9,
            kind: "CALLS".to_string(),
            path: "lib.rs".to_string(),
            line: 7,
        }]
    );
}

#[test]
fn active_source_index_store_supports_bounded_search_module_context_and_impact() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\nfn called_other() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 1, 512);

    assert_eq!(
        store.search_symbols(SymbolSearchRequest {
            query: "call".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolSearchResult {
            matches: BoundedItems {
                items: vec![SymbolSearchItem {
                    symbol: "caller".to_string(),
                    kind: "fn".to_string(),
                    path: "lib.rs".to_string(),
                }],
                truncated: true,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.inspect_module(ModuleContextRequest {
            path: "lib.rs".to_string(),
            budget: budget.clone(),
        }),
        Ok(ModuleContextResult {
            module: ModuleContextItem {
                path: "lib.rs".to_string(),
                language: Some("rust".to_string()),
                symbols: BoundedItems {
                    items: vec![SymbolSearchItem {
                        symbol: "caller".to_string(),
                        kind: "fn".to_string(),
                        path: "lib.rs".to_string(),
                    }],
                    truncated: true,
                    budget: budget.clone(),
                },
            },
        })
    );
    assert_eq!(
        store.search_repomap(RepoMapRequest {
            query: "lib".to_string(),
            budget: budget.clone(),
        }),
        Ok(RepoMapResult {
            files: BoundedItems {
                items: vec![RepoMapFileItem {
                    path: "lib.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![SymbolSearchItem {
                        symbol: "caller".to_string(),
                        kind: "fn".to_string(),
                        path: "lib.rs".to_string(),
                    }],
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec![
                "lib.rs:2".to_string(),
                "callee".to_string(),
                "missing.rs".to_string(),
            ],
            include_snippet: true,
            context_lines: 1,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "lib.rs:2".to_string(),
                    kind: "line".to_string(),
                    path: "lib.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(3),
                    snippet: Some("fn caller() {\n    callee();\n}".to_string()),
                    symbol: None,
                    symbol_kind: None,
                }],
                truncated: true,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: vec![UnresolvedEvidenceItem {
                    target: "missing.rs".to_string(),
                    reason: "no matching indexed line, file, or symbol found".to_string(),
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "callee".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolImpactResult {
            upstream: BoundedItems {
                items: vec![ImpactItem {
                    symbol: "caller".to_string(),
                    path: "lib.rs".to_string(),
                    relationship: "CALLS".to_string(),
                    site_line: 2,
                }],
                truncated: false,
                budget: budget.clone(),
            },
            downstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_includes_multiline_rust_symbol_body() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = "pub fn five() {\n    let a = 1;\n    let b = 2;\n    a + b;\n}\n";
    write_file(dir.path().join("src/lib.rs"), source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::default();

    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec!["five".to_string()],
            include_snippet: true,
            context_lines: 0,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "five".to_string(),
                    kind: "symbol".to_string(),
                    path: "src/lib.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(5),
                    snippet: Some(source.trim_end().to_string()),
                    symbol: Some("five".to_string()),
                    symbol_kind: Some("fn".to_string()),
                }],
                truncated: false,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_batches_many_symbols_in_one_rust_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut source = String::new();
    let mut targets = Vec::new();
    let mut items = Vec::new();
    for i in 0..20 {
        let symbol = format!("f{i}");
        let function = format!("pub fn f{i}() {{\n    let a = {i};\n    let b = a + 1;\n}}\n");
        let start = 1 + i * 5;
        let end = start + 3;
        source.push_str(&function);
        source.push('\n');
        targets.push(symbol.clone());
        items.push(EvidenceItem {
            target: symbol.clone(),
            kind: "symbol".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: Some(start),
            end_line: Some(end),
            snippet: Some(function.trim_end().to_string()),
            symbol: Some(symbol),
            symbol_kind: Some("fn".to_string()),
        });
    }
    write_file(dir.path().join("src/lib.rs"), &source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(
        /*timeout_ms*/ 5_000,
        /*max_items*/ 200,
        /*max_serialized_bytes*/ 1024 * 1024,
    );

    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets,
            include_snippet: true,
            context_lines: 0,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items,
                truncated: false,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_without_snippet_keeps_symbol_line_and_skips_widening() {
    let dir = tempfile::tempdir().expect("temp dir");
    let source = "pub fn five() {\n    let a = 1;\n    let b = 2;\n    a + b;\n}\n";
    write_file(dir.path().join("src/lib.rs"), source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(
        /*timeout_ms*/ 5_000,
        /*max_items*/ 200,
        /*max_serialized_bytes*/ 1024 * 1024,
    );

    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec!["five".to_string()],
            include_snippet: false,
            context_lines: 0,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "five".to_string(),
                    kind: "symbol".to_string(),
                    path: "src/lib.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(1),
                    snippet: None,
                    symbol: Some("five".to_string()),
                    symbol_kind: Some("fn".to_string()),
                }],
                truncated: false,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_caps_long_rust_item_at_eighty_lines() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut source = String::from("pub fn long() {\n");
    for i in 0..98 {
        source.push_str(&format!("    let line_{i} = {i};\n"));
    }
    source.push_str("}\n");
    write_file(dir.path().join("src/lib.rs"), &source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(
        /*timeout_ms*/ 5_000,
        /*max_items*/ 200,
        /*max_serialized_bytes*/ 1024 * 1024,
    );
    let expected_snippet = source.lines().take(80).collect::<Vec<_>>().join("\n");

    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec!["long".to_string()],
            include_snippet: true,
            context_lines: 0,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "long".to_string(),
                    kind: "symbol".to_string(),
                    path: "src/lib.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(80),
                    snippet: Some(expected_snippet),
                    symbol: Some("long".to_string()),
                    symbol_kind: Some("fn".to_string()),
                }],
                truncated: false,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_matches_symbol_kind_when_file_changed_after_indexing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let initial_source = "pub fn Widget() {\n    let a = 1;\n    let b = 2;\n    a + b;\n}\n";
    write_file(dir.path().join("src/lib.rs"), initial_source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let source = "struct Widget { value: u8 } pub fn Widget() {\n    let a = 1;\n    let b = 2;\n    a + b;\n}\n";
    write_file(dir.path().join("src/lib.rs"), source);
    let budget = QueryBudget::bounded(
        /*timeout_ms*/ 5_000,
        /*max_items*/ 200,
        /*max_serialized_bytes*/ 1024 * 1024,
    );

    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec!["Widget".to_string()],
            include_snippet: true,
            context_lines: 0,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "Widget".to_string(),
                    kind: "symbol".to_string(),
                    path: "src/lib.rs".to_string(),
                    start_line: Some(1),
                    end_line: Some(5),
                    snippet: Some(source.trim_end().to_string()),
                    symbol: Some("Widget".to_string()),
                    symbol_kind: Some("fn".to_string()),
                }],
                truncated: false,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn inspect_evidence_stays_bounded_when_file_no_longer_parses_after_indexing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let initial_source = "pub fn Widget() {\n    let a = 1;\n    let b = 2;\n    a + b;\n}\n";
    write_file(dir.path().join("src/lib.rs"), initial_source);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    write_file(dir.path().join("src/lib.rs"), "pub fn Widget( {{{ ]] \n");
    let budget = QueryBudget::bounded(
        /*timeout_ms*/ 5_000,
        /*max_items*/ 200,
        /*max_serialized_bytes*/ 1024 * 1024,
    );

    let result = store
        .inspect_evidence(EvidenceRequest {
            targets: vec!["Widget".to_string()],
            include_snippet: true,
            context_lines: 0,
            budget,
        })
        .expect("evidence request must not fail on an unparseable file");

    assert!(result.evidence.items.len() <= 1);
    for item in &result.evidence.items {
        let (start, end) = (item.start_line.unwrap_or(1), item.end_line.unwrap_or(1));
        assert!(
            end >= start && end - start < 80,
            "snippet window is bounded"
        );
    }
}

#[test]
fn active_source_index_store_keeps_relevant_repomap_files_before_truncation() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("a-target.rs"), "fn helper() {}\n");
    write_file(dir.path().join("z.rs"), "fn target() {}\n");
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");

    assert_eq!(
        store.search_repomap(RepoMapRequest {
            query: "target".to_string(),
            budget: QueryBudget::bounded(100, 1, 2048),
        }),
        Ok(RepoMapResult {
            files: BoundedItems {
                items: vec![RepoMapFileItem {
                    path: "z.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![SymbolSearchItem {
                        symbol: "target".to_string(),
                        kind: "fn".to_string(),
                        path: "z.rs".to_string(),
                    }],
                }],
                truncated: true,
                budget: QueryBudget::bounded(100, 1, 2048),
            },
        })
    );
}

#[test]
fn active_source_index_store_bounded_repomap_matches_full_ranking() {
    let dir = tempfile::tempdir().expect("temp dir");
    for path in ["z.rs", "b.rs", "a.rs", "helper.rs"] {
        write_file(
            dir.path().join(path),
            "fn helper_z() {}\nfn helper_a() {}\nfn helper() {}\nfn helper_m() {}\n",
        );
    }
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let mut store =
        SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
            .expect("load active source index");
    // File ties must use path order, not scan order.
    store.artifact.files.reverse();
    let query = SearchQuery::new("helper");
    let mut expected_files = store
        .artifact
        .files
        .iter()
        .map(|file| {
            let mut symbols = file
                .symbols
                .iter()
                .map(|symbol| SymbolSearchItem {
                    symbol: symbol.name.clone(),
                    kind: symbol.kind.clone(),
                    path: file.path.clone(),
                })
                .collect::<Vec<_>>();
            symbols.sort_by_key(|symbol| query.score_symbol(&symbol.symbol, &symbol.path));
            RepoMapFileItem {
                path: file.path.clone(),
                language: file.language.clone(),
                symbols,
            }
        })
        .collect::<Vec<_>>();
    expected_files.sort_by(|left, right| left.path.cmp(&right.path));
    for limit in 0..=5 {
        for max_serialized_bytes in [1, 300, 4096] {
            let budget = QueryBudget {
                timeout_ms: 1000,
                max_items: limit,
                max_serialized_bytes,
            };
            let mut candidates = expected_files.clone();
            for file in &mut candidates {
                file.symbols.truncate(limit);
            }
            let expected =
                bounded_items(candidates, budget.clone()).map(|files| RepoMapResult { files });
            assert_eq!(
                store.search_repomap(RepoMapRequest {
                    query: "helper".to_string(),
                    budget,
                }),
                expected
            );
        }
    }
}

#[test]
fn impact_capacity_preserves_order_and_truncation() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn target() {}\nfn first() {\n target();\n}\nfn second() {\n target();\n}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load index");
    let full = store
        .impact_symbol(SymbolImpactRequest {
            symbol: "target".to_string(),
            budget: QueryBudget::default(),
        })
        .expect("full impact");
    assert_eq!(full.upstream.items.len(), 2);
    for limit in [0, 1, 2, 3] {
        let budget = QueryBudget::bounded(
            /*timeout_ms*/ 1000, limit, /*max_serialized_bytes*/ 4096,
        );
        let expected = SymbolImpactResult {
            upstream: bounded_items(full.upstream.items.clone(), budget.clone())
                .expect("bounded upstream"),
            downstream: bounded_items(full.downstream.items.clone(), budget.clone())
                .expect("bounded downstream"),
        };
        assert_eq!(
            store.impact_symbol(SymbolImpactRequest {
                symbol: "target".to_string(),
                budget,
            }),
            Ok(expected)
        );
    }
}

#[test]
fn active_source_index_store_fails_closed_for_ambiguous_function_names() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("src/a.rs"),
        "mod nested {\n    pub fn run() {}\n}\n",
    );
    write_file(
        dir.path().join("src/b.rs"),
        "fn caller() {\n    run();\n}\npub fn run() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");

    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "run".to_string(),
            budget: QueryBudget::bounded(100, 10, 1024),
        }),
        Err(QueryError::AmbiguousArtifact)
    );
}

#[test]
fn active_source_index_store_returns_empty_impact_for_unique_non_function_symbol() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "pub struct Demo;\nfn helper() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 10, 1024);

    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "Demo".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolImpactResult {
            upstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget: budget.clone(),
            },
            downstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn impact_items_report_each_call_site_line_as_edge_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee();\n    let _ = 1;\n    callee();\n}\nfn callee() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 10, 4096);
    let call_site = |line| ImpactItem {
        symbol: String::new(),
        path: "lib.rs".to_string(),
        relationship: "CALLS".to_string(),
        site_line: line,
    };

    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "callee".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolImpactResult {
            upstream: BoundedItems {
                items: vec![
                    ImpactItem {
                        symbol: "caller".to_string(),
                        ..call_site(2)
                    },
                    ImpactItem {
                        symbol: "caller".to_string(),
                        ..call_site(4)
                    },
                ],
                truncated: false,
                budget: budget.clone(),
            },
            downstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "caller".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolImpactResult {
            upstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget: budget.clone(),
            },
            downstream: BoundedItems {
                items: vec![
                    ImpactItem {
                        symbol: "callee".to_string(),
                        ..call_site(2)
                    },
                    ImpactItem {
                        symbol: "callee".to_string(),
                        ..call_site(4)
                    },
                ],
                truncated: false,
                budget,
            },
        })
    );
}

#[test]
fn active_source_index_store_reports_diff_impact_for_changed_owned_symbols() {
    let dir = tempfile::tempdir().expect("temp dir");
    init_git_workspace(dir.path());
    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    );
    git(dir.path(), &["add", "lib.rs"]);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    write_file(
        dir.path().join("lib.rs"),
        "fn caller() {\n    callee();\n    let changed = 1;\n}\nfn callee() {}\n",
    );

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 10, 4096);

    assert_eq!(
        store.impact_diff(DiffImpactRequest {
            scope: "unstaged".to_string(),
            base_ref: None,
            include_paths: Vec::new(),
            budget: budget.clone(),
        }),
        Ok(DiffImpactResult {
            summary: DiffImpactSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            changed_files: BoundedItems {
                items: vec![DiffImpactFile {
                    path: "lib.rs".to_string(),
                    changed_symbols: BoundedItems {
                        items: vec![DiffImpactSymbol {
                            symbol: "caller".to_string(),
                            kind: "fn".to_string(),
                            path: "lib.rs".to_string(),
                            line: 1,
                        }],
                        truncated: false,
                        budget: budget.clone(),
                    },
                }],
                truncated: false,
                budget: budget.clone(),
            },
            affected_processes: BoundedItems {
                items: vec![DiffImpactProcess {
                    name: "caller -> callee".to_string(),
                    entry_symbol: "caller".to_string(),
                    path: "lib.rs".to_string(),
                    changed_steps: vec!["caller".to_string()],
                }],
                truncated: false,
                budget,
            },
            warnings: Vec::new(),
        })
    );
}

#[test]
fn active_source_index_store_filters_diff_impact_to_include_paths() {
    let dir = tempfile::tempdir().expect("temp dir");
    init_git_workspace(dir.path());
    write_file(dir.path().join("src/lib.rs"), "fn keep() {}\n");
    write_file(dir.path().join("tests/demo.rs"), "fn skip() {}\n");
    git(dir.path(), &["add", "src/lib.rs", "tests/demo.rs"]);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    write_file(
        dir.path().join("src/lib.rs"),
        "fn keep() {\n    let changed = 1;\n}\n",
    );
    write_file(
        dir.path().join("tests/demo.rs"),
        "fn skip() {\n    let changed = 1;\n}\n",
    );

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 10, 4096);
    let result = store
        .impact_diff(DiffImpactRequest {
            scope: "unstaged".to_string(),
            base_ref: None,
            include_paths: vec!["src".to_string()],
            budget,
        })
        .expect("diff impact");

    assert_eq!(result.summary.changed_files, 1);
    assert_eq!(result.changed_files.items[0].path, "src/lib.rs".to_string());
    assert_eq!(
        result.warnings,
        vec!["Omitted 1 changed path(s) outside include_paths: tests/demo.rs".to_string()]
    );
}

#[test]
fn active_source_index_store_ranks_search_matches_before_truncating() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn helper_extra() {}\nfn helper() {}\nfn call_helper() {}\nfn other() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 2, 512);

    assert_eq!(
        store.search_symbols(SymbolSearchRequest {
            query: "helper".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolSearchResult {
            matches: BoundedItems {
                items: vec![
                    SymbolSearchItem {
                        symbol: "helper".to_string(),
                        kind: "fn".to_string(),
                        path: "lib.rs".to_string(),
                    },
                    SymbolSearchItem {
                        symbol: "helper_extra".to_string(),
                        kind: "fn".to_string(),
                        path: "lib.rs".to_string(),
                    },
                ],
                truncated: true,
                budget,
            },
        })
    );
}

#[test]
fn active_source_index_store_bounded_search_preserves_stable_ties() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn helper_z() {}\nfn helper_a() {}\nfn helper() {}\nfn helper_m() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let full = store
        .search_symbols(SymbolSearchRequest {
            query: "helper".to_string(),
            budget: QueryBudget::default(),
        })
        .expect("full search");
    let mut expected_symbols = store
        .artifact
        .files
        .iter()
        .flat_map(|file| file.symbols.iter().map(|symbol| symbol.name.clone()))
        .collect::<Vec<_>>();
    expected_symbols.sort_by_key(|name| usize::from(name != "helper"));
    assert_eq!(
        full.matches
            .items
            .iter()
            .map(|item| item.symbol.clone())
            .collect::<Vec<_>>(),
        expected_symbols
    );
    for limit in 1..=5 {
        let budget = QueryBudget::bounded(100, limit, 4096);
        let mut expected_items = full.matches.items.clone();
        expected_items.truncate(limit);
        assert_eq!(
            store.search_symbols(SymbolSearchRequest {
                query: "helper".to_string(),
                budget: budget.clone(),
            }),
            Ok(SymbolSearchResult {
                matches: BoundedItems {
                    items: expected_items,
                    truncated: full.matches.items.len() > limit,
                    budget,
                },
            })
        );
    }
}

#[test]
fn active_source_index_store_interrupts_empty_file_scan_at_deadline_stride() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("lib.rs"), "fn helper() {}\n");
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let mut store =
        SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
            .expect("load active source index");
    let mut empty_file = store.artifact.files[0].clone();
    empty_file.symbols.clear();
    store.artifact.files = vec![empty_file; QUERY_DEADLINE_CHECK_STRIDE + 1];
    force_query_scan_timeout_after_items(1);
    let result = store.search_symbols(SymbolSearchRequest {
        query: "missing".to_string(),
        budget: QueryBudget::default(),
    });
    let scanned = query_scan_test_items();
    clear_query_scan_timeout();
    assert_eq!(result, Err(QueryError::Timeout));
    assert_eq!(scanned, QUERY_DEADLINE_CHECK_STRIDE);
}

#[test]
fn active_source_index_store_interrupts_symbol_scan_at_deadline_stride() {
    let dir = tempfile::tempdir().expect("temp dir");
    let symbol_count = QUERY_DEADLINE_CHECK_STRIDE + 1;
    let contents = (0..symbol_count)
        .map(|index| format!("fn symbol_{index}() {{}}\n"))
        .collect::<String>();
    write_file(dir.path().join("lib.rs"), &contents);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");

    force_query_scan_timeout_after_items(1);
    let result = store.search_symbols(SymbolSearchRequest {
        query: "missing".to_string(),
        budget: QueryBudget::default(),
    });
    let scanned = query_scan_test_items();
    clear_query_scan_timeout();

    assert_eq!(result, Err(QueryError::Timeout));
    assert_eq!(scanned, QUERY_DEADLINE_CHECK_STRIDE);
    assert!(
        scanned < symbol_count,
        "the scan must stop before the artifact ends"
    );
}

#[test]
fn active_source_index_store_interrupts_repomap_scan_at_deadline_stride() {
    let dir = tempfile::tempdir().expect("temp dir");
    let symbol_count = QUERY_DEADLINE_CHECK_STRIDE + 1;
    let contents = (0..symbol_count)
        .map(|index| format!("fn symbol_{index}() {{}}\n"))
        .collect::<String>();
    write_file(dir.path().join("lib.rs"), &contents);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");

    force_query_scan_timeout_after_items(1);
    let result = store.search_repomap(RepoMapRequest {
        query: "missing".to_string(),
        budget: QueryBudget::default(),
    });
    let scanned = query_scan_test_items();
    clear_query_scan_timeout();

    assert_eq!(result, Err(QueryError::Timeout));
    assert_eq!(scanned, QUERY_DEADLINE_CHECK_STRIDE);
    assert!(
        scanned < symbol_count + 1,
        "the scan must stop before the artifact ends"
    );
}

#[test]
fn active_source_index_store_interrupts_impact_scan_at_deadline_stride() {
    let dir = tempfile::tempdir().expect("temp dir");
    let relationship_count = QUERY_DEADLINE_CHECK_STRIDE + 1;
    let contents = std::iter::once("fn target() {}\n".to_string())
        .chain(
            (0..relationship_count)
                .map(|index| format!("fn caller_{index}() {{\n    target();\n}}\n")),
        )
        .collect::<String>();
    write_file(dir.path().join("lib.rs"), &contents);
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let symbol_location_scan_count = relationship_count + 2;

    force_query_scan_timeout_after_items(symbol_location_scan_count + 1);
    let result = store.impact_symbol(SymbolImpactRequest {
        symbol: "target".to_string(),
        budget: QueryBudget::default(),
    });
    let scanned = query_scan_test_items();
    clear_query_scan_timeout();

    assert_eq!(result, Err(QueryError::Timeout));
    assert_eq!(
        scanned,
        symbol_location_scan_count + QUERY_DEADLINE_CHECK_STRIDE
    );
    assert!(
        scanned < symbol_location_scan_count + relationship_count,
        "the impact scan must stop before the relationship artifact ends"
    );
}

#[test]
fn active_source_index_store_enforces_serialized_byte_budget() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "fn alpha() {}\nfn alphabet() {}\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 20, 70);

    let result = store
        .search_symbols(SymbolSearchRequest {
            query: "alpha".to_string(),
            budget: budget.clone(),
        })
        .expect("search should stay under budget");

    assert_eq!(result.matches.items.len(), 1);
    assert!(result.matches.truncated);
    assert_eq!(result.matches.budget, budget);
}

#[test]
fn inspect_evidence_redacts_secret_like_snippets() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(
        dir.path().join("lib.rs"),
        "const AUTH: &str = \"Authorization: Bearer sample1234567890token\";\nconst API_KEY: &str = \"api_key = sk-sample1234567890\";\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");

    let store = SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
        .expect("load active source index");
    let budget = QueryBudget::bounded(100, 10, 2048);

    // Behavioral assertion instead of exact snippet bytes: repo-wide secret
    // redaction passes kept rewriting the literal expectation in this file.
    // The contract is that no secret value survives and a marker is present.
    let result = store
        .inspect_evidence(EvidenceRequest {
            targets: vec!["lib.rs:1".to_string(), "lib.rs".to_string()],
            include_snippet: true,
            context_lines: 1,
            budget,
        })
        .expect("evidence request succeeds");

    assert_eq!(result.evidence.items.len(), 2);
    assert!(result.unresolved.items.is_empty());
    for item in &result.evidence.items {
        let snippet = item.snippet.as_deref().expect("snippet present");
        assert!(
            !snippet.contains("sample1234567890"),
            "secret value must not survive redaction: {snippet}"
        );
        assert!(
            snippet.contains("REDACTED"),
            "redaction marker must be present: {snippet}"
        );
    }
}

#[test]
fn active_source_index_store_fails_closed_without_current_pointer() {
    let dir = tempfile::tempdir().expect("temp dir");

    assert_eq!(
        SourceIndexStore::load_active_generation(&OntographArtifactRoot::new(dir.path()))
            .unwrap_err(),
        SourceIndexLoadError::MissingCurrentPointer
    );
}

#[test]
fn dirty_rust_file_metadata_changes_workspace_state_key() {
    let dir = tempfile::tempdir().expect("temp dir");
    init_git_workspace(dir.path());
    write_file(dir.path().join("lib.rs"), "fn value() { let _ = 1; }\n");
    git(dir.path(), &["add", "lib.rs"]);
    git(dir.path(), &["commit", "-qm", "initial"]);
    let head = resolve_target_head(dir.path());

    write_file(dir.path().join("lib.rs"), "fn value() { let _ = 2; }\n");
    let first = rust_workspace_state_key(dir.path(), &head).expect("first dirty state");
    let first_status = Command::new("git")
        .args(["status", "--porcelain=v1", "--", "*.rs"])
        .current_dir(dir.path())
        .output()
        .expect("first status");
    write_file(dir.path().join("lib.rs"), "fn value() { let _ = 22222; }\n");
    let second = rust_workspace_state_key(dir.path(), &head).expect("second dirty state");
    let second_status = Command::new("git")
        .args(["status", "--porcelain=v1", "--", "*.rs"])
        .current_dir(dir.path())
        .output()
        .expect("second status");

    assert_eq!(first_status.stdout, second_status.stdout);
    assert_ne!(first, second);
}

#[test]
fn fresh_versioned_workspace_state_key_does_not_walk_all_rust_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    init_git_workspace(dir.path());
    write_file(dir.path().join("lib.rs"), "fn ready() {}\n");
    git(dir.path(), &["add", "lib.rs"]);
    git(dir.path(), &["commit", "-qm", "initial"]);
    let head = resolve_target_head(dir.path());
    reset_full_workspace_walk_count();

    rust_workspace_state_key(dir.path(), &head).expect("fresh workspace key");

    assert_eq!(full_workspace_walk_count(), 0);
}

#[test]
fn non_numeric_index_lock_is_not_reclaimed() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("lib.rs"), "fn ready() {}\n");
    let root = OntographArtifactRoot::new(dir.path());
    std::fs::create_dir_all(root.root_path()).expect("artifact root");
    std::fs::write(root.index_rebuild_lock_path(), "unknown-owner\n").expect("lock");

    assert_eq!(
        index_rust_workspace(sample_request(dir.path())),
        Err(RustIndexError::RebuildInProgress)
    );
    assert_eq!(
        std::fs::read_to_string(root.index_rebuild_lock_path()).expect("retained lock"),
        "unknown-owner\n"
    );
}

#[test]
fn concurrent_index_rebuilds_publish_one_generation() {
    let dir = tempfile::tempdir().expect("temp dir");
    for index in 0..512 {
        write_file(
            dir.path().join(format!("src/file_{index}.rs")),
            &format!("fn function_{index}() {{}}\n"),
        );
    }
    let repo_root = dir.path().to_path_buf();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    // Spawn every thread before the barrier releases them.
    #[allow(clippy::needless_collect)]
    let handles = (0..2)
        .map(|_| {
            let repo_root = repo_root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                index_rust_workspace(sample_request(&repo_root))
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("rebuild thread"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(RustIndexError::RebuildInProgress)))
            .count(),
        1
    );
    let root = OntographArtifactRoot::new(dir.path());
    let generations = std::fs::read_dir(root.root_path().join("generations"))
        .expect("generations")
        .count();
    assert_eq!(generations, 1);
    SourceIndexStore::load_active_generation(&root).expect("uncorrupted active generation");
}

#[test]
fn rebuilds_prune_to_retained_generation_count() {
    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("lib.rs"), "fn indexed() {}\n");

    for (generation_id, timestamp) in [
        ("gen-1", 1_725_000_001),
        ("gen-2", 1_725_000_002),
        ("gen-3", 1_725_000_003),
    ] {
        let mut request = sample_request(dir.path());
        request.generation_id = generation_id.to_string();
        request.timestamp_unix_seconds = timestamp;
        index_rust_workspace(request).expect("index workspace");
    }
    let root = OntographArtifactRoot::new(dir.path());
    assert_eq!(
        std::fs::read_dir(root.generations_root())
            .expect("generations")
            .count(),
        3
    );

    let mut request = sample_request(dir.path());
    request.generation_id = "gen-4".to_string();
    request.timestamp_unix_seconds = 1_725_000_004;
    index_rust_workspace(request).expect("index workspace");
    assert_eq!(
        std::fs::read_dir(root.generations_root())
            .expect("generations")
            .count(),
        3
    );
    assert!(
        !root
            .generation_dir("gen-1")
            .expect("generation path")
            .exists()
    );
    assert!(
        root.generation_dir("gen-4")
            .expect("generation path")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn prune_failure_does_not_fail_publish() {
    use std::os::unix::fs::PermissionsExt;

    // A read-only parent directory does not stop root from unlinking children,
    // so this failure cannot be provoked when the suite runs as root.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }

    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("lib.rs"), "fn indexed() {}\n");
    for (generation_id, timestamp) in [
        ("gen-1", 1_725_000_001),
        ("gen-2", 1_725_000_002),
        ("gen-3", 1_725_000_003),
    ] {
        let mut request = sample_request(dir.path());
        request.generation_id = generation_id.to_string();
        request.timestamp_unix_seconds = timestamp;
        index_rust_workspace(request).expect("index workspace");
    }

    let old_generation = OntographArtifactRoot::new(dir.path())
        .generation_dir("gen-1")
        .expect("generation path");
    write_file(old_generation.join("keep.txt"), "keep");
    std::fs::set_permissions(&old_generation, std::fs::Permissions::from_mode(0o555))
        .expect("make old generation read-only");

    let mut request = sample_request(dir.path());
    request.generation_id = "gen-4".to_string();
    request.timestamp_unix_seconds = 1_725_000_004;
    let (_, limits) = index_rust_workspace(request).expect("publish despite prune failure");

    std::fs::set_permissions(&old_generation, std::fs::Permissions::from_mode(0o755))
        .expect("restore generation permissions");
    assert!(
        limits
            .iter()
            .any(|warning| warning.contains("failed to prune generation gen-1")),
        "limits: {limits:?}"
    );
    assert_eq!(
        std::fs::read_to_string(OntographArtifactRoot::new(dir.path()).current_pointer_path())
            .expect("current pointer"),
        "gen-4\n"
    );
}

#[test]
fn docs_context_bounded_ranking_preserves_duplicate_and_fact_order_semantics() {
    use crate::docs::DOCS_INDEX_FILE;
    use crate::docs::DocsIndexArtifact;
    use crate::docs::DocsIndexStore;
    use crate::query::DocsContextItem;
    use crate::query::DocsContextRequest;
    use crate::query::DocsContextResult;

    let dir = tempfile::tempdir().expect("temp dir");
    write_file(dir.path().join("lib.rs"), "fn helper() {}\n");
    write_file(
        dir.path().join("README.md"),
        "# Guide\nRequirement: REQ-1\n",
    );
    index_rust_workspace(sample_request(dir.path())).expect("index workspace");
    let root = OntographArtifactRoot::new(dir.path());
    let path = root
        .generation_file_path("gen-1", DOCS_INDEX_FILE)
        .expect("docs path");
    let mut artifact: DocsIndexArtifact =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("docs artifact"))
            .expect("parse docs artifact");
    let mut first = artifact.documents[0].clone();
    first.id = "doc".to_string();
    first.path = "z.md".to_string();
    let mut last = first.clone();
    last.path = "a.md".to_string();
    last.title = "last duplicate".to_string();
    artifact.documents = vec![first, last.clone()];
    let mut before = artifact.facts[0].clone();
    before.id = "before".to_string();
    before.document_id = "doc".to_string();
    before.text = "not selected".to_string();
    before.requirement_id = None;
    let mut selected = before.clone();
    selected.id = "selected".to_string();
    selected.text = "needle".to_string();
    let mut duplicate = selected.clone();
    duplicate.text = "later duplicate".to_string();
    let mut after = before.clone();
    after.id = "after".to_string();
    artifact.facts = vec![before, selected.clone(), duplicate, after.clone()];
    artifact.links.clear();
    std::fs::write(
        &path,
        serde_json::to_vec(&artifact).expect("serialize docs"),
    )
    .expect("write docs fixture");
    let store = DocsIndexStore::load_active_generation(&root).expect("load docs");
    let expected = vec![
        DocsContextItem::Document {
            document_id: last.id,
            path: last.path,
            title: last.title,
            document_kind: last.kind,
            excerpt: last.summary,
        },
        DocsContextItem::Fact {
            fact_id: after.id,
            document_id: after.document_id,
            heading_id: after.heading_id,
            fact_kind: after.kind,
            text: after.text,
            line_start: after.line_start,
            line_end: after.line_end,
            requirement_id: after.requirement_id,
        },
        DocsContextItem::Fact {
            fact_id: selected.id,
            document_id: selected.document_id,
            heading_id: selected.heading_id,
            fact_kind: selected.kind,
            text: selected.text,
            line_start: selected.line_start,
            line_end: selected.line_end,
            requirement_id: selected.requirement_id,
        },
    ];
    for max_items in 0..=4 {
        for max_serialized_bytes in [1, 300, 4096] {
            let budget = QueryBudget {
                max_items,
                max_serialized_bytes,
                ..QueryBudget::default()
            };
            assert_eq!(
                store.context(DocsContextRequest {
                    document_id: None,
                    path: None,
                    heading_id: None,
                    requirement_id: None,
                    route: None,
                    symbol: None,
                    text: Some("needle".to_string()),
                    budget: budget.clone(),
                }),
                bounded_serialized_items(expected.clone(), budget)
                    .map(|items| DocsContextResult { items })
            );
        }
    }
    // Exercise admission, both artifact scans, and both candidate projections.
    for after_items in [0, 1, 3, 7, 8, 9] {
        force_query_scan_timeout_after_items(after_items);
        let result = store.context(DocsContextRequest {
            document_id: None,
            path: None,
            heading_id: None,
            requirement_id: None,
            route: None,
            symbol: None,
            text: Some("needle".to_string()),
            budget: QueryBudget::default(),
        });
        let scanned = query_scan_test_items();
        clear_query_scan_timeout();
        assert_eq!(result, Err(QueryError::Timeout));
        assert_eq!(scanned, after_items);
        assert_eq!(query_scan_test_items(), scanned);
    }
}

fn sample_request(repo_root: &std::path::Path) -> RustIndexRequest {
    RustIndexRequest {
        repo_root: repo_root.to_path_buf(),
        artifact_root: OntographArtifactRoot::new(repo_root),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: "0.0.0-test".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    }
}

fn write_file(path: std::path::PathBuf, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    std::fs::write(path, contents).expect("write file");
}

fn init_git_workspace(repo_root: &std::path::Path) {
    git(repo_root, &["init", "-q"]);
    git(
        repo_root,
        &["config", "user.email", "ontograph-tests@example.com"],
    );
    git(repo_root, &["config", "user.name", "Ontograph Tests"]);
}

fn git(repo_root: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}
