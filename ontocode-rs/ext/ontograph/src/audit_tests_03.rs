#[tokio::test]
async fn audit_migration_reports_missing_source_reference() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_kind: Some("migration_reference".to_string()),
            fact_a_paths: Some(json!(["src/missing.rs"])),
            fact_a_symbols: Some(json!(["missing_symbol"])),
            fact_a_routes: Some(json!(["/api/missing"])),
            ..Default::default()
        },
    );

    let result = audit_migration_result(&workspace, json!({ "action": "migration" })).await;

    assert_eq!(
        result.items,
        vec![AuditMigrationItem {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            fact_kind: "migration_reference".to_string(),
            status: AuditMigrationStatus::MissingSourceReference,
            text: "Alpha fact".to_string(),
            line_start: 3,
            line_end: 3,
            requirement_id: Some("REQ-1".to_string()),
            paths: vec!["src/missing.rs".to_string()],
            symbols: vec!["missing_symbol".to_string()],
            routes: vec!["/api/missing".to_string()],
            missing_paths: vec!["src/missing.rs".to_string()],
            missing_symbols: vec!["missing_symbol".to_string()],
            missing_routes: vec!["/api/missing".to_string()],
        }]
    );
}

#[tokio::test]
async fn audit_migration_skips_unsupported_migration_fact_kind() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_kind: Some("migration_unknown".to_string()),
            ..Default::default()
        },
    );

    let result = audit_migration_result(&workspace, json!({ "action": "migration" })).await;

    assert!(result.items.is_empty());
    assert_eq!(
        result.skipped_references,
        vec![AuditMigrationSkippedReference {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            fact_kind: "migration_unknown".to_string(),
            reason: "unsupported migration fact kind is not a migration reference".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_migration_fails_closed_for_missing_or_stale_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);

    let missing_err = audit_tool_error(&workspace, json!({ "action": "migration" })).await;
    assert!(
        missing_err
            .to_string()
            .contains("docs-index.json is missing")
    );

    let stale_hash = current_source_index_hash_for_tests(&workspace);
    write_drift_workspace_source(&workspace, "indexed_changed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_index_content_hash: Some(stale_hash),
            fact_a_kind: Some("migration".to_string()),
            ..Default::default()
        },
    );

    let stale_err = audit_tool_error(&workspace, json!({ "action": "migration" })).await;
    assert!(
        stale_err
            .to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn audit_migration_fails_closed_when_source_index_is_unavailable() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_kind: Some("migration".to_string()),
            ..Default::default()
        },
    );
    let root = workspace_artifact_root(&workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    std::fs::remove_file(
        root.generation_file_path(&generation_id, crate::manifest::SOURCE_INDEX_FILE)
            .expect("source-index path"),
    )
    .expect("remove source-index");

    let err = audit_tool_error(&workspace, json!({ "action": "migration" })).await;

    assert!(err.to_string().contains("source-index.json is missing"));
}

#[tokio::test]
async fn audit_requirements_reports_explicit_requirement_reference() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-req",
                    "from_document_id": "doc-adr",
                    "target_kind": "requirement_id",
                    "target": "REQ-1"
                }
            ])),
            ..Default::default()
        },
    );

    let result = audit_requirements_result(&workspace, json!({ "action": "requirements" })).await;

    assert_eq!(
        result.items,
        vec![
            AuditRequirementItem {
                document_id: "doc-adr".to_string(),
                fact_id: "fact-a".to_string(),
                fact_kind: "requirement".to_string(),
                requirement_id: "REQ-1".to_string(),
                status: AuditRequirementStatus::Referenced,
                text: "Alpha fact".to_string(),
                line_start: 3,
                line_end: 3,
                paths: vec!["src/lib.rs".to_string()],
                symbols: vec!["indexed".to_string()],
                routes: vec!["/api/alpha".to_string()],
                missing_paths: Vec::new(),
                missing_symbols: Vec::new(),
                missing_routes: Vec::new(),
            },
            AuditRequirementItem {
                document_id: "doc-adr".to_string(),
                fact_id: "fact-b".to_string(),
                fact_kind: "requirement".to_string(),
                requirement_id: "REQ-2".to_string(),
                status: AuditRequirementStatus::Unverifiable,
                text: "Beta fact".to_string(),
                line_start: 4,
                line_end: 4,
                paths: Vec::new(),
                symbols: Vec::new(),
                routes: Vec::new(),
                missing_paths: Vec::new(),
                missing_symbols: Vec::new(),
                missing_routes: Vec::new(),
            },
        ]
    );
    assert_eq!(
        result.links,
        vec![AuditRequirementLink {
            link_id: "link-req".to_string(),
            from_document_id: "doc-adr".to_string(),
            requirement_id: "REQ-1".to_string(),
        }]
    );
    assert_eq!(result.action, crate::audit_build::AuditAction::Requirements);
}

#[tokio::test]
async fn audit_requirements_reports_missing_source_reference() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_paths: Some(json!(["src/missing.rs"])),
            fact_a_symbols: Some(json!(["missing_symbol"])),
            fact_a_routes: Some(json!(["/api/missing"])),
            ..Default::default()
        },
    );

    let result = audit_requirements_result(&workspace, json!({ "action": "requirements" })).await;

    assert_eq!(
        result.items[0].status,
        AuditRequirementStatus::MissingSourceReference
    );
    assert_eq!(result.items[0].missing_paths, vec!["src/missing.rs"]);
    assert_eq!(result.items[0].missing_symbols, vec!["missing_symbol"]);
    assert_eq!(result.items[0].missing_routes, vec!["/api/missing"]);
}

#[tokio::test]
async fn audit_requirements_fails_closed_for_missing_or_stale_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);

    let missing_err = audit_tool_error(&workspace, json!({ "action": "requirements" })).await;
    assert!(
        missing_err
            .to_string()
            .contains("docs-index.json is missing")
    );

    let stale_hash = current_source_index_hash_for_tests(&workspace);
    write_drift_workspace_source(&workspace, "indexed_changed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_index_content_hash: Some(stale_hash),
            ..Default::default()
        },
    );
    let stale_err = audit_tool_error(&workspace, json!({ "action": "requirements" })).await;
    assert!(
        stale_err
            .to_string()
            .contains("does not match the active source-index.json identity"),
        "unexpected error: {stale_err}"
    );
}

#[tokio::test]
async fn audit_requirements_fails_closed_when_source_index_is_unavailable() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(&workspace, DocsIndexFixtureOverrides::default());
    let root = workspace_artifact_root(&workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    std::fs::remove_file(
        root.generation_file_path(&generation_id, crate::manifest::SOURCE_INDEX_FILE)
            .expect("source-index path"),
    )
    .expect("remove source-index");

    let err = audit_tool_error(&workspace, json!({ "action": "requirements" })).await;

    assert!(err.to_string().contains("source-index.json is missing"));
}

#[tokio::test]
async fn audit_drift_aggregates_docs_drift_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_paths: Some(json!(["src/missing.rs"])),
            fact_a_symbols: Some(json!(["missing_symbol"])),
            fact_a_routes: Some(json!(["/api/missing"])),
            ..Default::default()
        },
    );

    let result = audit_drift_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(result.action, crate::audit_build::AuditAction::Drift);
    assert_eq!(result.source.status, "ready");
    assert_eq!(result.source.docs_paired, true);
    assert_eq!(result.findings.len(), 3);
    assert!(result.skipped_references.is_empty());
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("No prose similarity"))
    );
}

#[tokio::test]
async fn audit_drift_reports_clean_docs_source_graph() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = audit_drift_result(&workspace, json!({ "action": "drift" })).await;

    assert!(result.findings.is_empty());
    assert!(result.skipped_references.is_empty());
    assert!(!result.truncated);
    assert_eq!(result.source.source_paths, 1);
    assert_eq!(result.source.source_routes, 1);
}

#[tokio::test]
async fn audit_drift_preserves_skipped_unsupported_references() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-external",
                    "from_document_id": "doc-adr",
                    "target_kind": "external_url",
                    "target": "https://example.test"
                }
            ])),
            ..Default::default()
        },
    );

    let result = audit_drift_result(&workspace, json!({ "action": "drift" })).await;

    assert!(result.findings.is_empty());
    assert_eq!(
        result.skipped_references,
        vec![DocsDriftSkippedReference {
            link_id: "link-external".to_string(),
            target_kind: "external_url".to_string(),
            target: "https://example.test".to_string(),
            reason: "unsupported docs link target kind is not drift".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_drift_fails_closed_for_stale_docs_source_pairing() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    let stale_hash = current_source_index_hash_for_tests(&workspace);
    write_drift_workspace_source(&workspace, "indexed_changed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_index_content_hash: Some(stale_hash),
            ..Default::default()
        },
    );

    let err = audit_tool_error(&workspace, json!({ "action": "drift" })).await;

    assert!(
        err.to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn audit_graph_diff_reports_added_and_removed_graph_ids() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_graph_diff_workspace_source(&workspace, false, true);
    index_workspace_generation_for_tests(&workspace, "base-gen");
    write_graph_diff_workspace_source(&workspace, true, false);
    index_workspace_generation_for_tests(&workspace, "target-gen");

    let result = audit_graph_diff_result(
        &workspace,
        json!({
            "action": "graph_diff",
            "base_generation_id": "base-gen",
            "target_generation_id": "target-gen"
        }),
    )
    .await;

    assert_eq!(result.action, crate::audit_build::AuditAction::GraphDiff);
    assert_eq!(result.base.generation_id, "base-gen");
    assert_eq!(result.target.generation_id, "target-gen");
    assert!(
        result
            .symbol_nodes
            .added
            .iter()
            .any(|node_id| node_id.ends_with(":fn:gamma"))
    );
    assert!(
        result
            .symbol_nodes
            .removed
            .iter()
            .any(|node_id| node_id.ends_with(":fn:beta"))
    );
    assert!(
        result
            .calls
            .added
            .iter()
            .any(|call_id| call_id.contains(":gamma:"))
    );
    assert!(
        result
            .unresolved_calls
            .removed
            .iter()
            .any(|call_id| call_id.contains(":missing_target:"))
    );
    assert_eq!(
        result.process_nodes.added,
        vec!["process:src/lib.rs:alpha:alpha>gamma".to_string()]
    );
    assert_eq!(
        result.process_nodes.removed,
        vec!["process:src/lib.rs:alpha:alpha>beta".to_string()]
    );
    assert_eq!(
        result.route_nodes.added,
        vec!["route:GET:/api/gamma:src/lib.rs:15:gamma".to_string()]
    );
    assert_eq!(
        result.route_nodes.removed,
        vec!["route:GET:/api/beta:src/lib.rs:15:beta".to_string()]
    );
    assert!(!result.truncated);
}

#[tokio::test]
async fn audit_graph_diff_fails_closed_for_legacy_empty_process_or_route_id() {
    for family in ["processes", "routes"] {
        let dir = tempdir().expect("temp dir");
        let workspace = dir.path().join("workspace");
        write_graph_diff_workspace_source(&workspace, false, false);
        index_workspace_generation_for_tests(&workspace, "base-gen");
        write_graph_diff_workspace_source(&workspace, true, false);
        index_workspace_generation_for_tests(&workspace, "target-gen");

        let root = workspace_artifact_root(&workspace);
        let source_index_path = root
            .generation_file_path("base-gen", crate::manifest::SOURCE_INDEX_FILE)
            .expect("source index path");
        let mut source_index = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&source_index_path).expect("read source index"),
        )
        .expect("source index json");
        source_index[family][0]["node_id"] = json!("");
        let source_index_contents = format!(
            "{}\n",
            serde_json::to_string_pretty(&source_index).expect("serialize source index")
        );
        std::fs::write(source_index_path, &source_index_contents).expect("rewrite source index");
        let mut hasher = Sha256::new();
        hasher.update(source_index_contents.as_bytes());
        let source_index_hash = format!("sha256:{:x}", hasher.finalize());
        rewrite_generation_manifest_for_tests(&workspace, "base-gen", |manifest| {
            manifest["completed_generations"][0]["metadata"]["manifest_hash"] =
                json!(source_index_hash);
        });

        let err = audit_tool_error(
            &workspace,
            json!({
                "action": "graph_diff",
                "base_generation_id": "base-gen",
                "target_generation_id": "target-gen"
            }),
        )
        .await;

        assert!(
            err.to_string()
                .contains("generation graph facts are incomplete"),
            "empty {family} node id must fail closed"
        );
    }
}

#[tokio::test]
async fn audit_graph_diff_fails_closed_for_generation_identity_mismatch() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_graph_diff_workspace_source(&workspace, false, false);
    index_workspace_generation_for_tests(&workspace, "base-gen");
    write_graph_diff_workspace_source(&workspace, true, false);
    index_workspace_generation_for_tests(&workspace, "target-gen");
    rewrite_generation_manifest_for_tests(&workspace, "base-gen", |manifest| {
        manifest["repository"]["root"] = json!("/different/repo");
    });

    let err = audit_tool_error(
        &workspace,
        json!({
            "action": "graph_diff",
            "base_generation_id": "base-gen",
            "target_generation_id": "target-gen"
        }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("generation repository identity mismatch")
    );
}

#[tokio::test]
async fn audit_graph_diff_fails_closed_for_missing_or_corrupt_generation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_graph_diff_workspace_source(&workspace, false, false);
    index_workspace_generation_for_tests(&workspace, "target-gen");

    let missing_err = audit_tool_error(
        &workspace,
        json!({
            "action": "graph_diff",
            "base_generation_id": "missing-gen",
            "target_generation_id": "target-gen"
        }),
    )
    .await;
    assert!(
        missing_err
            .to_string()
            .contains("base_generation_id generation is missing")
    );

    write_graph_diff_workspace_source(&workspace, true, false);
    index_workspace_generation_for_tests(&workspace, "base-gen");
    let root = workspace_artifact_root(&workspace);
    let source_index_path = root
        .generation_file_path("base-gen", crate::manifest::SOURCE_INDEX_FILE)
        .expect("source index path");
    std::fs::write(source_index_path, "{ not json").expect("corrupt source index");

    let corrupt_err = audit_tool_error(
        &workspace,
        json!({
            "action": "graph_diff",
            "base_generation_id": "base-gen",
            "target_generation_id": "target-gen"
        }),
    )
    .await;
    assert!(
        corrupt_err
            .to_string()
            .contains("base_generation_id generation is corrupt")
    );
}

#[tokio::test]
async fn audit_graph_diff_reports_output_truncation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_graph_diff_truncation_source(&workspace, 1);
    index_workspace_generation_for_tests(&workspace, "base-gen");
    write_graph_diff_truncation_source(&workspace, 5);
    index_workspace_generation_for_tests(&workspace, "target-gen");

    let result = audit_graph_diff_result(
        &workspace,
        json!({
            "action": "graph_diff",
            "base_generation_id": "base-gen",
            "target_generation_id": "target-gen",
            "max_items": 1
        }),
    )
    .await;

    assert!(result.truncated);
    assert!(result.symbol_nodes.added_truncated);
    assert_eq!(result.symbol_nodes.added.len(), 1);
    assert!(result.process_nodes.added_truncated);
    assert_eq!(result.process_nodes.added.len(), 1);
    assert!(result.route_nodes.added_truncated);
    assert_eq!(result.route_nodes.added.len(), 1);
}

#[tokio::test]
async fn explain_module_tool_reads_relative_rust_file_against_turn_cwd() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::write(
        workspace.join("src/sample.rs"),
        "pub struct Demo;\nfn helper() {}\nimpl Demo {\n    fn nested() {}\n}\npub(crate) enum Mode { A }\n",
    )
    .expect("write source");

    let tool = install_and_find_tool(&workspace).await;
    let payload = ToolPayload::Function {
        arguments: json!({
            "path": "./src/sample.rs",
            "max_symbols": 8
        })
        .to_string(),
    };

    let output = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, EXPLAIN_MODULE_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("tool should explain module");

    let result = serde_json::from_value::<ExplainModuleResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ExplainModuleResult {
            path: "src/sample.rs".to_string(),
            language: Some("rust".to_string()),
            line_count: 6,
            top_level_symbols: vec![
                "Demo".to_string(),
                "helper".to_string(),
                "Mode".to_string()
            ],
            limits: vec![
                format!("file reads are capped at {MAX_FILE_BYTES} bytes"),
                "Rust top_level_symbols use syn when the file parses; malformed Rust and non-Rust files use bounded line-based extraction".to_string()
            ],
        }
    );
}
