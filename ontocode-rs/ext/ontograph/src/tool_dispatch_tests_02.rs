async fn audit_coverage_result(workspace: &Path) -> AuditCoverageResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "coverage" }).to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit coverage should succeed");
    serde_json::from_value::<AuditCoverageResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit coverage result")
}

async fn audit_violations_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditViolationsResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit violations should succeed");
    serde_json::from_value::<AuditViolationsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit violations result")
}

async fn audit_migration_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditMigrationResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit migration should succeed");
    serde_json::from_value::<AuditMigrationResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit migration result")
}

async fn audit_requirements_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditRequirementsResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit requirements should succeed");
    serde_json::from_value::<AuditRequirementsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit requirements result")
}

async fn audit_error_topology_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditErrorTopologyResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit error_topology should succeed");
    serde_json::from_value::<AuditErrorTopologyResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit error_topology result")
}

async fn audit_drift_result(workspace: &Path, arguments: serde_json::Value) -> AuditDriftResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit drift should succeed");
    serde_json::from_value::<AuditDriftResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit drift result")
}

async fn audit_graph_diff_result(
    workspace: &Path,
    arguments: serde_json::Value,
) -> AuditGraphDiffResult {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit graph_diff should succeed");
    serde_json::from_value::<AuditGraphDiffResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit graph_diff result")
}

async fn audit_tool_error(
    workspace: &Path,
    arguments: serde_json::Value,
) -> ontocode_extension_api::FunctionCallError {
    let payload = ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    expect_tool_error(
        install_and_find_named_tool(workspace, AUDIT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload,
            })
            .await,
        "audit tool should fail closed",
    )
}

fn trace_document_item() -> DocsTraceItem {
    DocsTraceItem::Document {
        document_id: "doc-adr".to_string(),
        path: "docs/adr.md".to_string(),
        title: "ADR".to_string(),
        headings: vec![DocsTraceHeading {
            heading_id: "h-design".to_string(),
            title: "Design".to_string(),
        }],
    }
}

fn trace_fact_a_item() -> DocsTraceItem {
    DocsTraceItem::Fact {
        fact_id: "fact-a".to_string(),
        document_id: "doc-adr".to_string(),
        heading_id: Some("h-design".to_string()),
        fact_kind: "requirement".to_string(),
        text: "Alpha fact".to_string(),
        line_start: 3,
        line_end: 3,
        requirement_id: Some("REQ-1".to_string()),
    }
}

#[derive(Default)]
struct DocsIndexFixtureOverrides {
    source_generation_id: Option<String>,
    source_index_content_hash: Option<String>,
    completed_at: Option<serde_json::Value>,
    schema_version: Option<u32>,
    fact_a_kind: Option<String>,
    fact_a_paths: Option<serde_json::Value>,
    fact_a_routes: Option<serde_json::Value>,
    fact_a_symbols: Option<serde_json::Value>,
    links: Option<serde_json::Value>,
}

fn write_docs_index_for_tests(workspace: &Path) {
    write_docs_index_with_overrides_for_tests(workspace, DocsIndexFixtureOverrides::default());
}

fn write_docs_index_with_overrides_for_tests(
    workspace: &Path,
    overrides: DocsIndexFixtureOverrides,
) {
    let root = workspace_artifact_root(workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    let manifest_path = root
        .generation_manifest_path(&generation_id)
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("read manifest"),
    )
    .expect("parse manifest");
    let generation = manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == manifest.active_generation_id)
        .expect("active generation");
    let completed_at = overrides
        .completed_at
        .unwrap_or_else(|| json!(1_725_000_001_i64));
    let fact_a_paths = overrides
        .fact_a_paths
        .unwrap_or_else(|| json!(["src/lib.rs"]));
    let fact_a_routes = overrides
        .fact_a_routes
        .unwrap_or_else(|| json!(["/api/alpha"]));
    let fact_a_symbols = overrides
        .fact_a_symbols
        .unwrap_or_else(|| json!(["indexed"]));
    let fact_a_kind = overrides
        .fact_a_kind
        .unwrap_or_else(|| "requirement".to_string());
    let links = overrides.links.unwrap_or_else(|| json!([]));
    let docs_index = json!({
        "manifest": {
            "schema_version": overrides.schema_version.unwrap_or(1),
            "generator_version": "test-docs-v1",
            "repository_root": manifest.repository.root,
            "source_index_generation_id": overrides
                .source_generation_id
                .as_deref()
                .unwrap_or(&generation.metadata.generation_id),
            "source_index_content_hash": overrides
                .source_index_content_hash
                .as_deref()
                .unwrap_or(&generation.metadata.manifest_hash),
            "created_at_unix_seconds": 1_725_000_000_i64,
            "completed_at_unix_seconds": completed_at,
            "artifact_content_hash": "docs-test-hash"
        },
        "documents": [
            {
                "id": "doc-adr",
                "path": "docs/adr.md",
                "title": "ADR",
                "kind": "markdown",
                "content_hash": "doc-hash",
                "byte_length": 42,
                "headings": [
                    { "id": "h-design", "title": "Design" }
                ],
                "summary": "Docs context summary."
            }
        ],
        "facts": [
            {
                "id": "fact-b",
                "document_id": "doc-adr",
                "heading_id": "h-design",
                "kind": "requirement",
                "text": "Beta fact",
                "line_start": 4,
                "line_end": 4,
                "requirement_id": "REQ-2",
                "routes": [],
                "paths": [],
                "symbols": []
            },
            {
                "id": "fact-a",
                "document_id": "doc-adr",
                "heading_id": "h-design",
                "kind": fact_a_kind,
                "text": "Alpha fact",
                "line_start": 3,
                "line_end": 3,
                "requirement_id": "REQ-1",
                "routes": fact_a_routes,
                "paths": fact_a_paths,
                "symbols": fact_a_symbols
            }
        ],
        "links": links
    });
    let docs_path = root
        .generation_file_path(&generation_id, crate::docs::DOCS_INDEX_FILE)
        .expect("docs-index path");
    std::fs::write(
        docs_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&docs_index).expect("serialize docs-index")
        ),
    )
    .expect("write docs-index");
}

fn current_source_index_hash_for_tests(workspace: &Path) -> String {
    current_source_index_metadata_for_tests(workspace).1
}

fn current_source_identity_for_tests(workspace: &Path) -> AuditSessionSourceIdentity {
    let (generation_id, source_index_content_hash) =
        current_source_index_metadata_for_tests(workspace);
    let root = workspace_artifact_root(workspace);
    let manifest_path = root
        .generation_manifest_path(&generation_id)
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("read manifest"),
    )
    .expect("parse manifest");
    let target_head = manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == generation_id)
        .expect("active generation")
        .metadata
        .target_head
        .clone();
    AuditSessionSourceIdentity {
        generation_id,
        source_index_content_hash,
        target_head,
    }
}

fn current_source_index_metadata_for_tests(workspace: &Path) -> (String, String) {
    let root = workspace_artifact_root(workspace);
    let generation_id = std::fs::read_to_string(root.current_pointer_path())
        .expect("current pointer")
        .trim()
        .to_string();
    let manifest_path = root
        .generation_manifest_path(&generation_id)
        .expect("manifest path");
    let manifest = serde_json::from_str::<OntographManifest>(
        &std::fs::read_to_string(manifest_path).expect("read manifest"),
    )
    .expect("parse manifest");
    let metadata = &manifest
        .completed_generations
        .iter()
        .find(|generation| generation.metadata.generation_id == manifest.active_generation_id)
        .expect("active generation")
        .metadata;
    (
        metadata.generation_id.clone(),
        metadata.manifest_hash.clone(),
    )
}

fn write_verify_diff_workspace(workspace: &Path) {
    std::fs::create_dir_all(workspace).expect("create workspace");
    init_git_workspace(workspace);
    rewrite_verify_diff_workspace(workspace, "");
}

fn rewrite_verify_diff_workspace(workspace: &Path, caller_body: &str) {
    std::fs::write(
        workspace.join("lib.rs"),
        format!("fn caller() {{\n    callee();\n    {caller_body}\n}}\nfn callee() {{}}\n"),
    )
    .expect("write verify diff source");
}

fn verify_diff_expected_limits() -> Vec<String> {
    vec![
        "changed files are capped at 200 items".to_string(),
        "changed symbols are capped at 200 items per file".to_string(),
        "expected files, symbols, and tests are capped at 200 items each".to_string(),
        "audit.verify_diff supports scopes `unstaged`, `staged`, `all`, and `compare`".to_string(),
        "audit.verify_diff is read-only and does not discover tests or create tracking records"
            .to_string(),
    ]
}

fn verify_diff_expected_notes() -> Vec<String> {
    vec![
        "audit.verify_diff reuses impact.diff for changed-file and changed-symbol facts."
            .to_string(),
        "Expected tests are compared only against caller-supplied executed_tests; no test discovery is performed."
            .to_string(),
    ]
}

fn test_gap_expected_limits() -> Vec<String> {
    vec![
        "changed files are capped at 200 items".to_string(),
        "changed symbols are capped at 200 items per file".to_string(),
        "executed tests are capped at 200 items".to_string(),
        "symbol and file test expectations are capped at 200 entries each".to_string(),
        "audit.test_gap supports scopes `unstaged`, `staged`, `all`, and `compare`".to_string(),
        "audit.test_gap is read-only and does not discover tests or infer coverage from filenames"
            .to_string(),
    ]
}

fn test_gap_expected_notes() -> Vec<String> {
    vec![
        "audit.test_gap reuses impact.diff for changed-file and changed-symbol facts."
            .to_string(),
        "Expected tests are caller-supplied; audit.test_gap does not discover tests or infer coverage from filenames."
            .to_string(),
    ]
}

fn workspace_artifact_root(workspace: &Path) -> OntographArtifactRoot {
    OntographArtifactRoot::new(workspace)
}

fn init_git_workspace(workspace: &Path) {
    git(workspace, &["init", "-q"]);
    git(
        workspace,
        &["config", "user.email", "ontograph-tests@example.com"],
    );
    git(workspace, &["config", "user.name", "Ontograph Tests"]);
}

fn git(workspace: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn write_valid_km_for_tool_test(root: &Path, version: &str) -> std::path::PathBuf {
    let km_root = root.join(format!("sample-km-{version}"));
    std::fs::create_dir_all(km_root.join("examples")).expect("create examples");
    std::fs::create_dir_all(km_root.join("tests")).expect("create tests");
    std::fs::write(
        km_root.join("km.toml"),
        format!(
            "id = \"codebase-graph/creating-ontograph\"\nversion = \"{version}\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM for tool tests.\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n"
        ),
    )
    .expect("write manifest");
    std::fs::write(
        km_root.join("facts.jsonl"),
        "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":0.9}\n",
    )
    .expect("write facts");
    std::fs::write(km_root.join("examples/README.md"), "example").expect("write example");
    std::fs::write(km_root.join("tests/README.md"), "test").expect("write test");
    km_root
}

fn test_target_head(workspace: &Path) -> String {
    let committed_head = crate::indexer::resolve_target_head(workspace);
    format!(
        "{committed_head}-{}",
        rust_workspace_state_key(workspace, &committed_head).expect("workspace fingerprint")
    )
}

#[tokio::test]
async fn audit_session_bundle_projects_deduped_groups() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn indexed() {}
",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "findings": [
                {
                    "finding_id": "F-1",
                    "path": "src/lib.rs",
                    "symbol": "indexed"
                },
                {
                    "finding_id": "F-2",
                    "path": "src/lib.rs",
                    "symbol": "indexed"
                }
            ]
        }),
    )
    .await;

    let dedupe = audit_session_dedupe_result(&workspace).await;
    assert_eq!(dedupe.status, AuditSessionDedupeStatus::Pass);

    let result = audit_session_bundle_result(&workspace).await;

    assert_eq!(result.status, AuditSessionBundleStatus::Pass);
    assert_eq!(result.bundles.len(), 1);
    assert_eq!(result.bundles[0].finding_ids, vec!["F-1", "F-2"]);
    assert_eq!(result.bundles[0].owner_files, vec!["src/lib.rs"]);
    assert_eq!(result.bundles[0].target_symbols, vec!["indexed"]);
    assert_eq!(result.bundles[0].files_count, 1);
    assert_eq!(result.bundles[0].loc_estimate, 100);
    assert_eq!(result.bundles[0].cap_exceeded, false);
}

#[tokio::test]
async fn audit_session_bundle_filters_non_open_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "findings": [
                { "finding_id": "F-1", "path": "src/lib.rs", "symbol": "indexed", "status": "open" },
                {
                    "finding_id": "F-2",
                    "path": "src/lib.rs",
                    "symbol": "indexed",
                    "status": "closed"
                },
                {
                    "finding_id": "F-3",
                    "path": "src/lib.rs",
                    "symbol": "indexed",
                    "status": "resolved"
                }
            ]
        }),
    )
    .await;

    let dedupe = audit_session_dedupe_result(&workspace).await;
    let dedupe_session_id = dedupe.session_id.clone();
    assert_eq!(
        dedupe,
        AuditSessionDedupeResult {
            action: crate::audit_build::AuditAction::SessionDedupe,
            status: AuditSessionDedupeStatus::Pass,
            session_id: dedupe_session_id.clone(),
            groups: vec![AuditSessionDedupeGroup {
                key_kind: AuditSessionDedupeKeyKind::Symbol,
                key: "indexed".to_string(),
                finding_ids: vec!["F-1".to_string()],
            }],
            ungrouped_limitations: vec![
                AuditSessionDedupeLimitation {
                    finding_id: "F-2".to_string(),
                    reason: "finding status is not open: closed".to_string(),
                },
                AuditSessionDedupeLimitation {
                    finding_id: "F-3".to_string(),
                    reason: "finding status is not open: resolved".to_string(),
                },
            ],
            notes: vec![
                "Grouped existing local audit-session findings by fingerprint, then symbol, then write_set.".to_string(),
                "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created.".to_string(),
            ],
        }
    );

    let result = audit_session_bundle_result(&workspace).await;
    assert_eq!(
        result,
        AuditSessionBundleResult {
            action: crate::audit_build::AuditAction::SessionBundle,
            status: AuditSessionBundleStatus::Pass,
            session_id: dedupe_session_id,
            bundles: vec![crate::audit_session::AuditSessionBundleItem {
                bundle_id: "bundle-1".to_string(),
                finding_ids: vec!["F-1".to_string()],
                owner_files: vec!["src/lib.rs".to_string()],
                target_symbols: vec!["indexed".to_string()],
                required_tests: vec!["cargo test -p src".to_string()],
                stop_conditions: Vec::new(),
                files_count: 1,
                loc_estimate: 100,
                cap_exceeded: false,
            }],
            notes: vec![
                "Projected deduped findings into read-only bundles.".to_string(),
                "No session state was modified; no dispatch, scheduler, planner, or worker runtime was created.".to_string(),
            ],
        }
    );
}
