#[tokio::test]
async fn audit_session_verify_reports_current_session() {
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
            "session_id": "session-1",
            "created_at_unix_seconds": 1_725_000_100_i64
        }),
    )
    .await;

    let result = audit_session_verify_result(&workspace).await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::SessionVerify
    );
    assert_eq!(result.status, AuditSessionVerifyStatus::Current);
    assert_eq!(
        result.session.as_ref().expect("session").session_id,
        "session-1"
    );
    assert_eq!(
        result.lock.as_ref().expect("lock").source_identity,
        result.active_source_identity
    );
}

#[tokio::test]
async fn manage_session_reports_absent_state_without_mutation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let result = manage_session_result(&workspace).await;
    let session_path = result.session_path.as_deref().expect("session path");
    let lock_path = result.lock_path.as_deref().expect("lock path");

    assert!(result.session.is_none());
    assert!(result.lock.is_none());
    assert!(!session_path.is_empty());
    assert!(!lock_path.is_empty());
    assert_eq!(
        result.notes,
        vec![
            "Read-only inspection of the native session model without mutating active lock state."
                .to_string()
        ]
    );
    assert!(!Path::new(session_path).exists());
    assert!(!Path::new(lock_path).exists());
}

#[tokio::test]
async fn manage_session_reads_existing_session_without_mutation() {
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
            "session_id": "session-1",
            "created_at_unix_seconds": 1_725_000_100_i64
        }),
    )
    .await;
    let root = workspace_artifact_root(&workspace);
    let session_path = root.root_path().join("audit-session.json");
    let lock_path = root.root_path().join("audit-session.lock.json");
    let session_before = std::fs::read_to_string(&session_path).expect("read session before");
    let lock_before = std::fs::read_to_string(&lock_path).expect("read lock before");

    let result = manage_session_result(&workspace).await;

    assert_eq!(
        result.session.as_ref().expect("session").session_id,
        "session-1"
    );
    assert_eq!(result.lock.as_ref().expect("lock").session_id, "session-1");
    assert_eq!(
        std::fs::read_to_string(&session_path).expect("read session after"),
        session_before
    );
    assert_eq!(
        std::fs::read_to_string(&lock_path).expect("read lock after"),
        lock_before
    );
}

#[tokio::test]
async fn audit_session_verify_reports_stale_session_without_mutation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index first workspace");
    audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "session_id": "session-1",
            "created_at_unix_seconds": 1_725_000_100_i64
        }),
    )
    .await;
    let root = workspace_artifact_root(&workspace);
    let original_session = std::fs::read_to_string(root.root_path().join("audit-session.json"))
        .expect("read original session");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-2".to_string(),
        target_head: "def456".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_001,
    })
    .expect("index second workspace");

    let result = audit_session_verify_result(&workspace).await;

    assert_eq!(result.status, AuditSessionVerifyStatus::StaleSession);
    assert_eq!(
        result
            .lock
            .as_ref()
            .expect("lock")
            .source_identity
            .generation_id,
        "gen-1"
    );
    assert_eq!(
        std::fs::read_to_string(root.root_path().join("audit-session.json")).expect("read session"),
        original_session
    );
}

#[tokio::test]
async fn audit_session_verify_reports_missing_and_incomplete_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let missing = audit_session_verify_result(&workspace).await;
    assert_eq!(missing.status, AuditSessionVerifyStatus::Missing);
    assert!(missing.session.is_none());
    assert!(missing.lock.is_none());

    let root = workspace_artifact_root(&workspace);
    std::fs::create_dir_all(root.root_path()).expect("create root");
    std::fs::write(
        root.root_path().join("audit-session.lock.json"),
        serde_json::to_string_pretty(&AuditSessionLock {
            session_id: "session-1".to_string(),
            source_identity: current_source_identity_for_tests(&workspace),
        })
        .expect("serialize lock"),
    )
    .expect("write lock only");

    let incomplete = audit_session_verify_result(&workspace).await;
    assert_eq!(incomplete.status, AuditSessionVerifyStatus::Incomplete);
    assert!(incomplete.session.is_none());
    assert!(incomplete.lock.is_none());
}

#[tokio::test]
async fn audit_session_verify_reports_corrupt_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    std::fs::create_dir_all(root.root_path()).expect("create root");
    std::fs::write(root.root_path().join("audit-session.json"), "{not-json")
        .expect("write corrupt session");
    std::fs::write(
        root.root_path().join("audit-session.lock.json"),
        serde_json::to_string_pretty(&AuditSessionLock {
            session_id: "session-1".to_string(),
            source_identity: current_source_identity_for_tests(&workspace),
        })
        .expect("serialize lock"),
    )
    .expect("write lock");

    let result = audit_session_verify_result(&workspace).await;

    assert_eq!(result.status, AuditSessionVerifyStatus::Corrupt);
    assert!(result.session.is_none());
    assert!(result.lock.is_none());
}

#[tokio::test]
async fn audit_session_verify_reports_mismatched_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    std::fs::create_dir_all(root.root_path()).expect("create root");
    let source_identity = current_source_identity_for_tests(&workspace);
    std::fs::write(
        root.root_path().join("audit-session.json"),
        serde_json::to_string_pretty(&AuditSessionRecord {
            session_id: "session-1".to_string(),
            created_at_unix_seconds: 1_725_000_100,
            status: AuditSessionRecordStatus::Open,
            source_identity: source_identity.clone(),
            findings: Vec::new(),
        })
        .expect("serialize session"),
    )
    .expect("write session");
    std::fs::write(
        root.root_path().join("audit-session.lock.json"),
        serde_json::to_string_pretty(&AuditSessionLock {
            session_id: "session-2".to_string(),
            source_identity,
        })
        .expect("serialize lock"),
    )
    .expect("write lock");

    let result = audit_session_verify_result(&workspace).await;

    assert_eq!(result.status, AuditSessionVerifyStatus::Mismatched);
    assert_eq!(
        result.session.as_ref().expect("session").session_id,
        "session-1"
    );
    assert_eq!(result.lock.as_ref().expect("lock").session_id, "session-2");
}

#[tokio::test]
async fn audit_session_dedupe_groups_exact_fingerprints() {
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
            "session_id": "session-1",
            "findings": [
                { "finding_id": "F-1", "title": "One", "fingerprint": "fp-1" },
                { "finding_id": "F-2", "title": "Two", "fingerprint": "fp-1" },
                { "finding_id": "F-3", "title": "Three", "fingerprint": "fp-2" }
            ]
        }),
    )
    .await;

    let result = audit_session_dedupe_result(&workspace).await;
    let session_id = result.session_id.clone();

    assert_eq!(
        result,
        AuditSessionDedupeResult {
            action: crate::audit_build::AuditAction::SessionDedupe,
            status: AuditSessionDedupeStatus::Pass,
            session_id,
            groups: vec![
                AuditSessionDedupeGroup {
                    key_kind: AuditSessionDedupeKeyKind::Fingerprint,
                    key: "fp-1".to_string(),
                    finding_ids: vec!["F-1".to_string(), "F-2".to_string()],
                },
                AuditSessionDedupeGroup {
                    key_kind: AuditSessionDedupeKeyKind::Fingerprint,
                    key: "fp-2".to_string(),
                    finding_ids: vec!["F-3".to_string()],
                },
            ],
            ungrouped_limitations: Vec::new(),
            notes: vec![
                "Grouped existing local audit-session findings by fingerprint, then symbol, then write_set.".to_string(),
                "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_session_dedupe_falls_back_to_symbol_then_write_set() {
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
            "session_id": "session-1",
            "findings": [
                { "finding_id": "F-1", "title": "One", "symbol": "render" },
                { "finding_id": "F-2", "title": "Two", "symbol": "render" },
                { "finding_id": "F-3", "title": "Three", "write_set": ["src/a.rs", "src/b.rs"] },
                { "finding_id": "F-4", "title": "Four", "write_set": ["src/b.rs", "src/a.rs"] }
            ]
        }),
    )
    .await;

    let result = audit_session_dedupe_result(&workspace).await;

    assert_eq!(
        result.groups,
        vec![
            AuditSessionDedupeGroup {
                key_kind: AuditSessionDedupeKeyKind::Symbol,
                key: "render".to_string(),
                finding_ids: vec!["F-1".to_string(), "F-2".to_string()],
            },
            AuditSessionDedupeGroup {
                key_kind: AuditSessionDedupeKeyKind::WriteSet,
                key: "src/a.rs\nsrc/b.rs".to_string(),
                finding_ids: vec!["F-3".to_string(), "F-4".to_string()],
            },
        ]
    );
}

#[tokio::test]
async fn audit_session_dedupe_open_findings_only_and_status_limitations() {
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
            "session_id": "session-1",
            "findings": [
                { "finding_id": "F-1", "title": "Open", "fingerprint": "fp-1", "status": "open" },
                { "finding_id": "F-2", "title": "Closed", "fingerprint": "fp-1", "status": "closed" },
                { "finding_id": "F-3", "title": "Open", "fingerprint": "fp-2", "status": "Open" },
                {
                    "finding_id": "F-4",
                    "title": "Unknown",
                    "fingerprint": "fp-2",
                    "status": "fixed"
                }
            ]
        }),
    )
    .await;

    let result = audit_session_dedupe_result(&workspace).await;
    let session_id = result.session_id.clone();

    assert_eq!(
        result,
        AuditSessionDedupeResult {
            action: crate::audit_build::AuditAction::SessionDedupe,
            status: AuditSessionDedupeStatus::Pass,
            session_id,
            groups: vec![
                AuditSessionDedupeGroup {
                    key_kind: AuditSessionDedupeKeyKind::Fingerprint,
                    key: "fp-1".to_string(),
                    finding_ids: vec!["F-1".to_string()],
                },
                AuditSessionDedupeGroup {
                    key_kind: AuditSessionDedupeKeyKind::Fingerprint,
                    key: "fp-2".to_string(),
                    finding_ids: vec!["F-3".to_string()],
                },
            ],
            ungrouped_limitations: vec![
                AuditSessionDedupeLimitation {
                    finding_id: "F-2".to_string(),
                    reason: "finding status is not open: closed".to_string(),
                },
                AuditSessionDedupeLimitation {
                    finding_id: "F-4".to_string(),
                    reason: "finding status is not open: fixed".to_string(),
                },
            ],
            notes: vec![
                "Grouped existing local audit-session findings by fingerprint, then symbol, then write_set.".to_string(),
                "No session state was modified; no bundle, dispatch, scheduler, planner, or worker runtime was created.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_session_dedupe_reports_ungrouped_limitations() {
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
            "session_id": "session-1",
            "findings": [{ "finding_id": "F-1", "title": "No grouping fields" }]
        }),
    )
    .await;

    let result = audit_session_dedupe_result(&workspace).await;

    assert_eq!(
        result.ungrouped_limitations,
        vec![AuditSessionDedupeLimitation {
            finding_id: "F-1".to_string(),
            reason: "missing fingerprint, symbol, and write_set grouping fields".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_session_dedupe_fails_closed_for_stale_or_corrupt_session() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index first workspace");
    audit_session_start_result(
        &workspace,
        json!({ "action": "session_start", "session_id": "session-1" }),
    )
    .await;
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-2".to_string(),
        target_head: "def456".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_001,
    })
    .expect("index second workspace");

    let root = workspace_artifact_root(&workspace);
    let stale_session_path = root.root_path().join("audit-session.json");
    let stale_lock_path = root.root_path().join("audit-session.lock.json");
    let stale_session_before =
        std::fs::read(&stale_session_path).expect("read stale session before");
    let stale_lock_before = std::fs::read(&stale_lock_path).expect("read stale lock before");

    let stale = audit_session_dedupe_result(&workspace).await;
    assert_eq!(stale.status, AuditSessionDedupeStatus::Inconclusive);
    assert!(
        stale
            .notes
            .iter()
            .any(|note| note.contains("does not match the active source-index identity"))
    );
    assert_eq!(
        stale_session_before,
        std::fs::read(&stale_session_path).expect("read stale session after")
    );
    assert_eq!(
        stale_lock_before,
        std::fs::read(&stale_lock_path).expect("read stale lock after")
    );

    let corrupt_workspace = dir.path().join("corrupt-workspace");
    std::fs::create_dir_all(corrupt_workspace.join("src")).expect("create corrupt src");
    std::fs::create_dir(corrupt_workspace.join(".git")).expect("create corrupt git marker");
    std::fs::write(corrupt_workspace.join("src/lib.rs"), "fn indexed() {}\n")
        .expect("write corrupt source");
    index_workspace_for_tests(&corrupt_workspace);
    let corrupt_root = workspace_artifact_root(&corrupt_workspace);
    std::fs::create_dir_all(corrupt_root.root_path()).expect("create corrupt root");
    let corrupt_session_path = corrupt_root.root_path().join("audit-session.json");
    let corrupt_lock_path = corrupt_root.root_path().join("audit-session.lock.json");
    std::fs::write(&corrupt_session_path, "{not-json").expect("write corrupt session");
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

    let corrupt = audit_session_dedupe_result(&corrupt_workspace).await;
    assert_eq!(corrupt.status, AuditSessionDedupeStatus::Inconclusive);
    assert!(
        corrupt
            .notes
            .iter()
            .any(|note| note.contains("Could not read or parse audit-session.json"))
    );
    assert_eq!(
        corrupt_session_before,
        std::fs::read(&corrupt_session_path).expect("read corrupt session after")
    );
    assert_eq!(
        corrupt_lock_before,
        std::fs::read(&corrupt_lock_path).expect("read corrupt lock after")
    );
}

#[tokio::test]
async fn audit_lifecycle_forbidden_followup_actions_are_not_registered() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    for action in ["resource_trace", "taint"] {
        let error = audit_tool_error(
            &workspace,
            json!({
                "action": action
            }),
        )
        .await;

        assert!(
            error
                .to_string()
                .contains("invalid ontograph.audit arguments"),
            "unexpected error for {action}: {error}"
        );
    }
}

#[tokio::test]
async fn audit_error_topology_reports_explicit_error_flow_candidates() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"
fn sample(result: Result<(), String>) -> Result<(), String> {
    let error = anyhow!("bad");
    if result.is_err() {
        let _ = result.map_err(|err| err.to_string());
    }
    return Err(error.to_string());
}
"#,
    )
    .expect("write source");

    let result =
        audit_error_topology_result(&workspace, json!({ "action": "error_topology" })).await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::ErrorTopology
    );
    assert_eq!(result.scanned_files, 1);
    assert_eq!(result.total_candidates, 4);
    assert_eq!(
        result
            .findings
            .iter()
            .map(|finding| finding.category.as_str())
            .collect::<Vec<_>>(),
        vec!["source", "check", "swallowed", "return"]
    );
    assert!(result.skipped.is_empty());
}

#[tokio::test]
async fn audit_error_topology_reports_no_findings_for_plain_rust() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn sample() -> usize {\n    let value = 1;\n    value\n}\n",
    )
    .expect("write source");

    let result =
        audit_error_topology_result(&workspace, json!({ "action": "error_topology" })).await;

    assert_eq!(
        result.action,
        crate::audit_build::AuditAction::ErrorTopology
    );
    assert_eq!(result.scanned_files, 1);
    assert_eq!(result.total_candidates, 0);
    assert!(result.findings.is_empty());
    assert!(result.skipped.is_empty());
    assert!(!result.truncated);
    assert_eq!(
        result.limits,
        vec![
            "max_items=20".to_string(),
            format!("max_file_bytes={MAX_FILE_BYTES}"),
        ]
    );
}

#[tokio::test]
async fn audit_error_topology_reports_truncation_when_candidate_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn sample(result: Result<(), String>) -> Result<(), String> {\n    let error = anyhow!(\"bad\");\n    if result.is_err() {}\n    return Err(error.to_string());\n}\n",
    )
    .expect("write source");

    let result = audit_error_topology_result(
        &workspace,
        json!({ "action": "error_topology", "max_items": 1 }),
    )
    .await;

    assert_eq!(result.total_candidates, 3);
    assert_eq!(result.findings.len() + result.skipped.len(), 1);
    assert!(result.truncated);
}
