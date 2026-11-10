#[tokio::test]
async fn docs_trace_tool_rejects_empty_selector_set() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let err = docs_context_error(&workspace, json!({ "action": "trace" })).await;

    assert!(
        err.to_string()
            .contains("trace requires at least one selector")
    );
}

#[tokio::test]
async fn docs_trace_tool_returns_unresolved_selector_record() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "requirement_id": "REQ-X" }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![DocsTraceItem::UnresolvedSelector {
            selector: "requirement_id".to_string(),
            value: "REQ-X".to_string(),
            reason: "selector did not match any explicit docs-index document, fact, or link"
                .to_string(),
        }]
    );
}

#[tokio::test]
async fn docs_trace_tool_caps_and_truncates_results() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(
        &workspace,
        json!({ "action": "trace", "document_id": "doc-adr", "max_items": 2 }),
    )
    .await;

    assert_eq!(
        result.trace_items,
        vec![trace_document_item(), trace_fact_a_item()]
    );
    assert!(result.truncated);
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_missing_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(err.to_string().contains("docs-index.json is missing"));
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_missing_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(err.to_string().contains("docs-index.json is missing"));
}

#[tokio::test]
async fn docs_readiness_tool_rejects_context_selectors() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let err = docs_context_error(
        &workspace,
        json!({ "action": "readiness", "document_id": "doc-adr" }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("readiness does not accept context selectors")
    );
}

#[tokio::test]
async fn docs_readiness_tool_rejects_context_selectors_before_sidecar_load() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let err = docs_context_error(
        &workspace,
        json!({ "action": "readiness", "document_id": "doc-adr" }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("readiness does not accept context selectors")
    );
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_corrupt_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    let docs_path = root
        .generation_file_path(&generation_id, crate::docs::DOCS_INDEX_FILE)
        .expect("docs path");
    std::fs::write(docs_path, "{not json").expect("write corrupt docs-index");

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(err.to_string().contains("docs-index.json is invalid"));
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_corrupt_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    let docs_path = root
        .generation_file_path(&generation_id, crate::docs::DOCS_INDEX_FILE)
        .expect("docs path");
    std::fs::write(docs_path, "{not json").expect("write corrupt docs-index");

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(err.to_string().contains("docs-index.json is invalid"));
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_dangling_docs_link() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-missing-source",
                    "from_document_id": "missing-doc",
                    "target_kind": "path",
                    "target": "src/lib.rs"
                }
            ])),
            ..Default::default()
        },
    );

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(err.to_string().contains("docs-index.json is invalid"));
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_unsupported_docs_schema() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            schema_version: Some(2),
            ..Default::default()
        },
    );

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("docs-index.json schema is unsupported")
    );
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_unsupported_docs_schema() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            schema_version: Some(2),
            ..Default::default()
        },
    );

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(
        err.to_string()
            .contains("docs-index.json schema is unsupported")
    );
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_unreadable_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    let docs_path = root
        .generation_file_path(&generation_id, crate::docs::DOCS_INDEX_FILE)
        .expect("docs path");
    std::fs::create_dir(docs_path).expect("create unreadable docs-index directory");

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(err.to_string().contains("docs-index.json is unreadable"));
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_mismatched_source_identity() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_generation_id: Some("old-generation".to_string()),
            ..Default::default()
        },
    );

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_mismatched_source_identity() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_generation_id: Some("old-generation".to_string()),
            ..Default::default()
        },
    );

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(
        err.to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn docs_context_tool_rejects_same_count_stale_source_index_hash() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn alpha() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let stale_source_hash = current_source_index_hash_for_tests(&workspace);
    std::fs::write(workspace.join("lib.rs"), "fn bravo() {}\n").expect("rewrite same-count source");
    index_workspace_for_tests(&workspace);
    assert_ne!(
        stale_source_hash,
        current_source_index_hash_for_tests(&workspace)
    );
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_index_content_hash: Some(stale_source_hash),
            ..Default::default()
        },
    );

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(
        err.to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn docs_drift_tool_rejects_same_count_stale_source_index_hash() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    let stale_source_hash = current_source_index_hash_for_tests(&workspace);
    write_drift_workspace_source(&workspace, "indexed_changed");
    index_workspace_for_tests(&workspace);
    assert_ne!(
        stale_source_hash,
        current_source_index_hash_for_tests(&workspace)
    );
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            source_index_content_hash: Some(stale_source_hash),
            ..Default::default()
        },
    );

    let err = docs_context_error(&workspace, json!({ "action": "drift" })).await;

    assert!(
        err.to_string()
            .contains("does not match the active source-index.json identity")
    );
}

#[tokio::test]
async fn docs_context_tool_fails_closed_for_in_progress_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            completed_at: Some(serde_json::Value::Null),
            ..Default::default()
        },
    );

    let err = docs_context_error(
        &workspace,
        json!({ "action": "context", "path": "docs/adr.md" }),
    )
    .await;

    assert!(err.to_string().contains("docs-index.json is in progress"));
}

#[tokio::test]
async fn docs_readiness_tool_fails_closed_for_in_progress_docs_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            completed_at: Some(serde_json::Value::Null),
            ..Default::default()
        },
    );

    let err = docs_context_error(&workspace, json!({ "action": "readiness" })).await;

    assert!(err.to_string().contains("docs-index.json is in progress"));
}

#[tokio::test]
async fn docs_drift_tool_reports_missing_source_path() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_paths: Some(json!(["./src/missing.rs"])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(
        result.drift_items,
        vec![DocsDriftItem::MissingSourcePath {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            path: "src/missing.rs".to_string(),
        }]
    );
    assert!(result.skipped_drift_references.is_empty());
}

#[tokio::test]
async fn docs_drift_tool_reports_missing_symbol_without_fuzzy_match() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed_near_match");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_symbols: Some(json!(["indexed"])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(
        result.drift_items,
        vec![DocsDriftItem::MissingSymbol {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            symbol: "indexed".to_string(),
            path: Some("src/lib.rs".to_string()),
        }]
    );
}

#[tokio::test]
async fn docs_drift_tool_reports_missing_route() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            fact_a_routes: Some(json!(["/api/missing"])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(
        result.drift_items,
        vec![DocsDriftItem::MissingRoute {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            route: "/api/missing".to_string(),
        }]
    );
}

#[tokio::test]
async fn docs_drift_tool_reports_stale_document_link_target() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-doc",
                    "from_document_id": "doc-adr",
                    "target_kind": "document_id",
                    "target": "missing-doc"
                }
            ])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(
        result.drift_items,
        vec![DocsDriftItem::StaleDocumentLink {
            link_id: "link-doc".to_string(),
            from_document_id: "doc-adr".to_string(),
            target_document_id: "missing-doc".to_string(),
            missing_document_id: "missing-doc".to_string(),
            missing_endpoint: "target_document_id".to_string(),
        }]
    );
}

#[tokio::test]
async fn docs_drift_tool_reports_stale_document_link_source() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_with_overrides_for_tests(
        &workspace,
        DocsIndexFixtureOverrides {
            links: Some(json!([
                {
                    "id": "link-doc",
                    "from_document_id": "missing-doc",
                    "target_kind": "document_id",
                    "target": "doc-adr"
                }
            ])),
            ..Default::default()
        },
    );

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(
        result.drift_items,
        vec![DocsDriftItem::StaleDocumentLink {
            link_id: "link-doc".to_string(),
            from_document_id: "missing-doc".to_string(),
            target_document_id: "doc-adr".to_string(),
            missing_document_id: "missing-doc".to_string(),
            missing_endpoint: "from_document_id".to_string(),
        }]
    );
}

#[tokio::test]
async fn docs_drift_tool_skips_unsupported_link_kind() {
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

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert!(result.drift_items.is_empty());
    assert_eq!(
        result.skipped_drift_references,
        vec![DocsDriftSkippedReference {
            link_id: "link-external".to_string(),
            target_kind: "external_url".to_string(),
            target: "https://example.test".to_string(),
            reason: "unsupported docs link target kind is not drift".to_string(),
        }]
    );
}

#[tokio::test]
async fn docs_drift_tool_reports_no_drift_for_clean_sidecar() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_drift_workspace_source(&workspace, "indexed");
    index_workspace_for_tests(&workspace);
    write_docs_index_for_tests(&workspace);

    let result = docs_tool_result(&workspace, json!({ "action": "drift" })).await;

    assert_eq!(result.action, DocsAction::Drift);
    assert!(result.drift_items.is_empty());
    assert!(result.skipped_drift_references.is_empty());
    assert!(!result.truncated);
}

#[tokio::test]
async fn docs_drift_tool_fails_closed_for_missing_source_index() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let err = docs_context_error(&workspace, json!({ "action": "drift" })).await;

    assert!(err.to_string().contains("source index"));
}

#[tokio::test]
async fn audit_migration_reports_implemented_reference() {
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

    let result = audit_migration_result(&workspace, json!({ "action": "migration" })).await;

    assert_eq!(
        result.items,
        vec![AuditMigrationItem {
            document_id: "doc-adr".to_string(),
            fact_id: "fact-a".to_string(),
            fact_kind: "migration".to_string(),
            status: AuditMigrationStatus::Implemented,
            text: "Alpha fact".to_string(),
            line_start: 3,
            line_end: 3,
            requirement_id: Some("REQ-1".to_string()),
            paths: vec!["src/lib.rs".to_string()],
            symbols: vec!["indexed".to_string()],
            routes: vec!["/api/alpha".to_string()],
            missing_paths: Vec::new(),
            missing_symbols: Vec::new(),
            missing_routes: Vec::new(),
        }]
    );
    assert!(result.skipped_references.is_empty());
    assert_eq!(result.action, crate::audit_build::AuditAction::Migration);
}
