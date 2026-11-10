#[tokio::test]
async fn audit_session_bundle_reports_cap_exceeded() {
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

    let mut findings = Vec::new();
    for i in 0..6 {
        findings.push(json!({
            "finding_id": format!("F-{}", i),
            "path": format!("src/file_{}.rs", i),
            "symbol": "func"
        }));
    }

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "findings": findings
        }),
    )
    .await;

    // Use a blank group key for all so they dedupe together
    let _ = audit_session_dedupe_result(&workspace).await;
    let result = audit_session_bundle_result(&workspace).await;

    // The findings do not have identical fingerprints or symbols (they are identical symbols but different paths).
    // Wait, the dedupe logic uses `symbol` then `write_set` then `fingerprint`. All have symbol="func".
    // So they all dedupe into the same group.

    assert_eq!(result.status, AuditSessionBundleStatus::Pass);
    assert_eq!(result.bundles.len(), 1);
    assert_eq!(result.bundles[0].files_count, 6);
    assert_eq!(result.bundles[0].cap_exceeded, true);
    assert_eq!(
        result.bundles[0].stop_conditions,
        vec!["Max files/LOC cap exceeded.".to_string()]
    );
}

#[tokio::test]
async fn audit_session_bundle_fails_closed_for_stale_or_corrupt_session() {
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

    let missing = audit_session_bundle_result(&workspace).await;
    assert_eq!(missing.status, AuditSessionBundleStatus::Inconclusive);

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
        }),
    )
    .await;

    index_workspace_generation_for_tests(&workspace, "gen-stale");
    let stale_root = workspace_artifact_root(&workspace);
    let stale_session_path = stale_root.root_path().join("audit-session.json");
    let stale_lock_path = stale_root.root_path().join("audit-session.lock.json");
    let stale_session_before =
        std::fs::read(&stale_session_path).expect("read stale session before");
    let stale_lock_before = std::fs::read(&stale_lock_path).expect("read stale lock before");

    let stale = audit_session_bundle_result(&workspace).await;
    assert_eq!(stale.status, AuditSessionBundleStatus::Inconclusive);
    assert_eq!(
        stale_session_before,
        std::fs::read(&stale_session_path).expect("read stale session after")
    );
    assert_eq!(
        stale_lock_before,
        std::fs::read(&stale_lock_path).expect("read stale lock after")
    );

    let corrupt_dir = tempdir().expect("temp dir");
    let corrupt_workspace = corrupt_dir.path().join("workspace");
    std::fs::create_dir_all(corrupt_workspace.join("src")).expect("create src");
    std::fs::create_dir(corrupt_workspace.join(".git")).expect("create git marker");
    std::fs::write(
        corrupt_workspace.join("src/lib.rs"),
        "fn indexed() {}
",
    )
    .expect("write source");
    index_workspace_for_tests(&corrupt_workspace);
    let corrupt_root = workspace_artifact_root(&corrupt_workspace);
    std::fs::create_dir_all(corrupt_root.root_path()).expect("create root");
    let corrupt_session_path = corrupt_root.root_path().join("audit-session.json");
    let corrupt_lock_path = corrupt_root.root_path().join("audit-session.lock.json");
    std::fs::write(&corrupt_session_path, "{not-json").expect("corrupt");
    std::fs::write(
        &corrupt_lock_path,
        serde_json::to_string_pretty(&AuditSessionLock {
            session_id: "session-1".to_string(),
            source_identity: current_source_identity_for_tests(&corrupt_workspace),
        })
        .expect("serialize corrupt lock"),
    )
    .expect("write corrupt lock");
    let corrupt_session_before =
        std::fs::read(&corrupt_session_path).expect("read corrupt session before");
    let corrupt_lock_before = std::fs::read(&corrupt_lock_path).expect("read corrupt lock before");

    let corrupt = audit_session_bundle_result(&corrupt_workspace).await;
    assert_eq!(corrupt.status, AuditSessionBundleStatus::Inconclusive);
    assert_eq!(
        corrupt_session_before,
        std::fs::read(&corrupt_session_path).expect("read corrupt session after")
    );
    assert_eq!(
        corrupt_lock_before,
        std::fs::read(&corrupt_lock_path).expect("read corrupt lock after")
    );
}

async fn audit_session_bundle_result(workspace: &std::path::Path) -> AuditSessionBundleResult {
    let payload = ToolPayload::Function {
        arguments: json!({ "action": "session_bundle" }).to_string(),
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
        .expect("audit session_bundle should succeed");
    serde_json::from_value::<AuditSessionBundleResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_bundle result")
}

#[tokio::test]
async fn audit_worker_scope_review_reports_matching_scope_pass() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "findings": [
                {
                    "finding_id": "F-1",
                    "fingerprint": "scope-pass",
                    "path": "lib.rs",
                    "symbol": "caller"
                },
                {
                    "finding_id": "F-2",
                    "fingerprint": "scope-pass",
                    "path": "lib.rs",
                    "symbol": "callee"
                }
            ]
        }),
    )
    .await;

    let _ = audit_session_dedupe_result(&workspace).await;
    let bundles = audit_session_bundle_result(&workspace).await;
    let bundle_id = bundles.bundles[0].bundle_id.clone();

    let result = audit_worker_scope_review_result(
        &workspace,
        json!({
            "action": "worker_scope_review",
            "bundle_id": bundle_id,
            "scope": "staged",
            "executed_tests": ["cargo test -p lib.rs"]
        }),
    )
    .await;

    assert_eq!(result.status, AuditWorkerScopeStatus::Pass);
}

#[tokio::test]
async fn audit_worker_scope_review_reports_unexpected_changed_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    std::fs::write(workspace.join("other.rs"), "fn other() { let a = 1; }\n")
        .expect("write other source");
    git(&workspace, &["add", "lib.rs", "other.rs"]);
    index_workspace_for_tests(&workspace);

    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "findings": [
                {
                    "finding_id": "F-1",
                    "fingerprint": "scope-fail",
                    "path": "lib.rs",
                    "symbol": "caller"
                },
                {
                    "finding_id": "F-2",
                    "fingerprint": "scope-fail",
                    "path": "lib.rs",
                    "symbol": "callee"
                }
            ]
        }),
    )
    .await;

    let _ = audit_session_dedupe_result(&workspace).await;
    let bundles = audit_session_bundle_result(&workspace).await;
    let bundle_id = bundles.bundles[0].bundle_id.clone();

    let result = audit_worker_scope_review_result(
        &workspace,
        json!({
            "action": "worker_scope_review",
            "bundle_id": bundle_id,
            "scope": "staged",
            "executed_tests": ["cargo test -p lib.rs"]
        }),
    )
    .await;

    assert_eq!(result.status, AuditWorkerScopeStatus::Fail);
    assert_eq!(result.unexpected_changed_files, vec!["other.rs"]);
}

#[tokio::test]
async fn audit_worker_scope_review_fails_closed_for_missing_bundle() {
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

    let result = audit_worker_scope_review_result(
        &workspace,
        json!({
            "action": "worker_scope_review",
            "bundle_id": "invalid-bundle",
            "scope": "unstaged"
        }),
    )
    .await;

    assert_eq!(result.status, AuditWorkerScopeStatus::Inconclusive);
    assert!(result.notes[0].contains("Could not load valid bundle state for worker scope review"));
}

async fn audit_worker_scope_review_result(
    workspace: &std::path::Path,
    arguments: serde_json::Value,
) -> AuditWorkerScopeResult {
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
        .expect("audit worker_scope_review should succeed");
    serde_json::from_value::<AuditWorkerScopeResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit worker_scope_review result")
}

#[tokio::test]
async fn audit_session_review_worker_reports_matching_scope_pass() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let bundle_id = prepare_session_review_worker_bundle(&workspace, false).await;

    let result = audit_session_review_worker_result(
        &workspace,
        json!({
            "action": "session_review_worker",
            "bundle_id": bundle_id,
            "scope": "staged",
            "executed_tests": ["cargo test -p lib.rs"]
        }),
    )
    .await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::SessionReviewWorker
    );
    assert_eq!(result.status, AuditWorkerScopeStatus::Pass);
}

#[tokio::test]
async fn audit_session_review_worker_reports_unexpected_changed_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let bundle_id = prepare_session_review_worker_bundle(&workspace, true).await;

    let result = audit_session_review_worker_result(
        &workspace,
        json!({
            "action": "session_review_worker",
            "bundle_id": bundle_id,
            "scope": "staged",
            "executed_tests": ["cargo test -p lib.rs"]
        }),
    )
    .await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::SessionReviewWorker
    );
    assert_eq!(result.status, AuditWorkerScopeStatus::Fail);
    assert_eq!(result.unexpected_changed_files, vec!["other.rs"]);
}

#[tokio::test]
async fn audit_session_review_worker_reports_missing_required_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let bundle_id = prepare_session_review_worker_bundle(&workspace, false).await;

    let result = audit_session_review_worker_result(
        &workspace,
        json!({
            "action": "session_review_worker",
            "bundle_id": bundle_id,
            "scope": "staged",
            "executed_tests": []
        }),
    )
    .await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::SessionReviewWorker
    );
    assert_eq!(result.status, AuditWorkerScopeStatus::Fail);
    assert_eq!(result.missing_expected_tests, vec!["cargo test -p lib.rs"]);
}

#[tokio::test]
async fn audit_session_review_worker_fails_closed_for_missing_bundle() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_session_review_worker_result(
        &workspace,
        json!({
            "action": "session_review_worker",
            "bundle_id": "invalid-bundle",
            "scope": "staged"
        }),
    )
    .await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::SessionReviewWorker
    );
    assert_eq!(result.status, AuditWorkerScopeStatus::Inconclusive);
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("Could not load valid bundle state"))
    );
}

async fn prepare_session_review_worker_bundle(
    workspace: &std::path::Path,
    include_unexpected_file: bool,
) -> String {
    write_verify_diff_workspace(workspace);
    git(workspace, &["add", "lib.rs"]);
    rewrite_verify_diff_workspace(workspace, "let changed = 1;");
    if include_unexpected_file {
        std::fs::write(workspace.join("other.rs"), "fn other() { let a = 1; }\n")
            .expect("write other source");
        git(workspace, &["add", "lib.rs", "other.rs"]);
    } else {
        git(workspace, &["add", "lib.rs"]);
    }
    index_workspace_for_tests(workspace);

    audit_session_start_result(
        workspace,
        json!({
            "action": "session_start",
            "findings": [
                {
                    "finding_id": "F-1",
                    "fingerprint": "session-review",
                    "path": "lib.rs",
                    "symbol": "caller"
                },
                {
                    "finding_id": "F-2",
                    "fingerprint": "session-review",
                    "path": "lib.rs",
                    "symbol": "callee"
                }
            ]
        }),
    )
    .await;
    let _ = audit_session_dedupe_result(workspace).await;
    let bundles = audit_session_bundle_result(workspace).await;
    bundles.bundles[0].bundle_id.clone()
}

async fn audit_session_review_worker_result(
    workspace: &std::path::Path,
    arguments: serde_json::Value,
) -> AuditWorkerScopeResult {
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
        .expect("audit session_review_worker should succeed");
    serde_json::from_value::<AuditWorkerScopeResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_review_worker result")
}

#[tokio::test]
async fn audit_session_dispatch_renders_prompt_packet_from_one_bundle() {
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
                { "finding_id": "F-1", "path": "src/lib.rs", "symbol": "indexed" },
                { "finding_id": "F-2", "path": "src/lib.rs", "symbol": "indexed" }
            ]
        }),
    )
    .await;
    let _ = audit_session_dedupe_result(&workspace).await;
    let bundles = audit_session_bundle_result(&workspace).await;
    let bundle_id = bundles.bundles[0].bundle_id.clone();

    let result = audit_session_dispatch_result(
        &workspace,
        json!({ "action": "session_dispatch", "bundle_id": bundle_id.clone() }),
    )
    .await;

    assert_eq!(result.status, AuditSessionDispatchStatus::Ready);
    assert_eq!(result.bundle_id, bundle_id);
    assert!(
        result
            .prompt_packet
            .contains(&format!("bundle_id: {bundle_id}"))
    );
    assert!(result.prompt_packet.contains("owner_files: src/lib.rs"));
    assert!(result.prompt_packet.contains("target_symbols: indexed"));
    assert_eq!(result.prompt_truncated, false);
}

#[tokio::test]
async fn audit_session_dispatch_fails_closed_for_missing_bundle() {
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
                { "finding_id": "F-1", "path": "src/lib.rs", "symbol": "indexed" }
            ]
        }),
    )
    .await;
    let _ = audit_session_dedupe_result(&workspace).await;
    let _ = audit_session_bundle_result(&workspace).await;

    let result = audit_session_dispatch_result(
        &workspace,
        json!({ "action": "session_dispatch", "bundle_id": "invalid-bundle" }),
    )
    .await;

    assert_eq!(result.status, AuditSessionDispatchStatus::Inconclusive);
    assert!(result.prompt_packet.is_empty());
    assert!(result.notes[0].contains("not found in active audit session"));
}

#[tokio::test]
async fn audit_session_dispatch_truncates_oversized_prompt_packet() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    // Many distinct files/symbols sharing one fingerprint collapse into a
    // single large bundle so the rendered packet exceeds the cap.
    let mut findings = Vec::new();
    for i in 0..200 {
        std::fs::write(
            workspace.join(format!("src/f{i}.rs")),
            format!("fn symbol_{i}() {{}}\n"),
        )
        .expect("write source");
        findings.push(json!({
            "finding_id": format!("F-{i}"),
            "path": format!("src/f{i}.rs"),
            "symbol": format!("symbol_{i}"),
            "fingerprint": "shared-fp"
        }));
    }
    index_workspace_for_tests(&workspace);

    audit_session_start_result(
        &workspace,
        json!({ "action": "session_start", "findings": findings }),
    )
    .await;
    let _ = audit_session_dedupe_result(&workspace).await;
    let bundles = audit_session_bundle_result(&workspace).await;
    let bundle_id = bundles.bundles[0].bundle_id.clone();

    let result = audit_session_dispatch_result(
        &workspace,
        json!({ "action": "session_dispatch", "bundle_id": bundle_id }),
    )
    .await;

    assert_eq!(result.status, AuditSessionDispatchStatus::Ready);
    assert_eq!(result.prompt_truncated, true);
    assert!(result.prompt_packet.chars().count() <= 4000);
}

async fn audit_session_dispatch_result(
    workspace: &std::path::Path,
    arguments: serde_json::Value,
) -> AuditSessionDispatchResult {
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
            truncation_policy: TruncationPolicy::Bytes(8192),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit session_dispatch should succeed");
    serde_json::from_value::<AuditSessionDispatchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize audit session_dispatch result")
}
