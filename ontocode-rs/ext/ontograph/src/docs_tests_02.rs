#[tokio::test]
async fn audit_build_reports_bounded_default_residue_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir_all(workspace.join("docs")).expect("create docs");
    std::fs::create_dir_all(workspace.join("tmp")).expect("create tmp");
    std::fs::create_dir_all(workspace.join("target")).expect("create target");
    std::fs::create_dir_all(workspace.join(".memory-bank")).expect("create memory-bank");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn sample() {\n    // TODO: keep me\n}\n",
    )
    .expect("write source");
    std::fs::write(workspace.join("docs/guide.md"), "TODO: ignored doc note\n")
        .expect("write docs");
    std::fs::write(
        workspace.join("tmp/ignored.rs"),
        "// TODO: ignored tmp residue\n",
    )
    .expect("write tmp");
    std::fs::write(
        workspace.join("target/generated.rs"),
        "// TODO: ignored target residue\n",
    )
    .expect("write target");
    std::fs::write(
        workspace.join(".memory-bank/note.md"),
        "TODO: ignored memory-bank note\n",
    )
    .expect("write memory-bank");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "build", "max_items": 5 }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit build should scan the workspace");

    let result = serde_json::from_value::<AuditBuildResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditBuildResult {
            action: crate::audit_build::AuditAction::Build,
            forbidden_domains: vec![
                "TODO:".to_string(),
                "FIXME:".to_string(),
                "XXX:".to_string(),
                "HACK:".to_string(),
                "todo!(".to_string(),
                "unimplemented!(".to_string(),
                "dbg!(".to_string(),
            ],
            scanned_files: 1,
            findings: vec![AuditBuildFinding {
                domain: "TODO:".to_string(),
                path: "src/lib.rs".to_string(),
                line: 2,
                snippet: "// TODO: keep me".to_string(),
            }],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "max_forbidden_domains=32".to_string(),
            ],
            notes: vec![
                "This first native slice scans repo-owned source/config files only.".to_string(),
                "Scratch and build-noise roots are excluded: .git, .ontocode/ontograph, .ontoindex, node_modules, target, third_party, tmp.".to_string(),
                "Default domains also skip doc/test-heavy roots: .memory-bank, docs, test, tests, fixtures, __fixtures__, __tests__.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_patterns_reports_bounded_default_code_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir_all(workspace.join("docs")).expect("create docs");
    std::fs::create_dir_all(workspace.join("tests")).expect("create tests");
    std::fs::create_dir_all(workspace.join("tmp")).expect("create tmp");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn sample() {\n    let value = maybe().unwrap();\n    todo!(\"later\");\n}\n",
    )
    .expect("write source");
    std::fs::write(
        workspace.join("tests/ignored.rs"),
        "fn test_only() {\n    panic!(\"ignored\");\n}\n",
    )
    .expect("write tests");
    std::fs::write(
        workspace.join("docs/guide.md"),
        "unwrap() in docs is ignored\n",
    )
    .expect("write docs");
    std::fs::write(
        workspace.join("tmp/ignored.rs"),
        "fn ignored() { unimplemented!(); }\n",
    )
    .expect("write tmp");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "patterns", "max_items": 2 }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit patterns should scan code files");

    let result = serde_json::from_value::<AuditPatternResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditPatternResult {
            action: crate::audit_build::AuditAction::Patterns,
            patterns: vec![
                "todo!(".to_string(),
                "unimplemented!(".to_string(),
                ".unwrap()".to_string(),
                ".expect(".to_string(),
                "panic!(".to_string(),
                "unsafe {".to_string(),
            ],
            scanned_files: 1,
            findings: vec![
                AuditPatternFinding {
                    pattern: ".unwrap()".to_string(),
                    reason: "Unchecked failure path; verify the call cannot fail or handle the error explicitly.".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 2,
                    snippet: "let value = maybe().unwrap();".to_string(),
                },
                AuditPatternFinding {
                    pattern: "todo!(".to_string(),
                    reason: "Deferred implementation marker; confirm the path is intentionally incomplete.".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 3,
                    snippet: "todo!(\"later\");".to_string(),
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=2".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "max_patterns=32".to_string(),
            ],
            notes: vec![
                "This native slice scans repo-owned code files only.".to_string(),
                "Scratch and build-noise roots are excluded: .git, .ontocode/ontograph, .ontoindex, node_modules, target, third_party, tmp.".to_string(),
                "Default patterns also skip doc/test-heavy roots: .memory-bank, docs, test, tests, fixtures, __fixtures__, __tests__.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_rerun_rechecks_saved_findings_against_workspace_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir_all(workspace.join("audit")).expect("create audit");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn sample() {\n    todo!(\"moved\");\n    todo!(\"keep\");\n}\n",
    )
    .expect("write source");
    std::fs::write(
        workspace.join("audit/findings.json"),
        serde_json::to_string_pretty(&json!({
            "findings": [
                {
                    "path": "src/lib.rs",
                    "line": 3,
                    "snippet": "todo!(\"keep\");"
                },
                {
                    "file": "src/lib.rs",
                    "line": 1,
                    "snippet": "todo!(\"moved\");"
                },
                {
                    "path": "src/lib.rs",
                    "line": 4,
                    "snippet": "todo!(\"fixed\");"
                },
                {
                    "path": "src/missing.rs",
                    "line": 1,
                    "snippet": "todo!(\"missing\");"
                },
                {
                    "line": 2,
                    "snippet": "todo!(\"invalid\");"
                }
            ]
        }))
        .expect("serialize findings"),
    )
    .expect("write audit file");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "rerun",
            "audit_file": "audit/findings.json",
            "max_items": 10
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, AUDIT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("audit rerun should recheck saved findings");

    let result = serde_json::from_value::<AuditRerunResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditRerunResult {
            action: crate::audit_build::AuditAction::Rerun,
            audit_file: "audit/findings.json".to_string(),
            total: 5,
            still_open: 1,
            moved: 1,
            fixed: 1,
            missing_file: 1,
            invalid: 1,
            findings: vec![
                AuditRerunFinding {
                    path: String::new(),
                    pattern: None,
                    recorded_line: Some(2),
                    status: AuditRerunStatus::Invalid,
                    current_line: None,
                    detail: Some("missing file path".to_string()),
                },
                AuditRerunFinding {
                    path: "src/lib.rs".to_string(),
                    pattern: None,
                    recorded_line: Some(1),
                    status: AuditRerunStatus::Moved,
                    current_line: Some(2),
                    detail: None,
                },
                AuditRerunFinding {
                    path: "src/lib.rs".to_string(),
                    pattern: None,
                    recorded_line: Some(3),
                    status: AuditRerunStatus::StillOpen,
                    current_line: Some(3),
                    detail: None,
                },
                AuditRerunFinding {
                    path: "src/lib.rs".to_string(),
                    pattern: None,
                    recorded_line: Some(4),
                    status: AuditRerunStatus::Fixed,
                    current_line: None,
                    detail: None,
                },
                AuditRerunFinding {
                    path: "src/missing.rs".to_string(),
                    pattern: None,
                    recorded_line: Some(1),
                    status: AuditRerunStatus::MissingFile,
                    current_line: None,
                    detail: None,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=10".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
            ],
            notes: vec![
                "This native slice rechecks one saved audit findings file against the active workspace only."
                    .to_string(),
                "Saved findings may use either native `path` or donor-style `file` keys."
                    .to_string(),
                format!(
                    "Both the saved audit file and matched workspace files are capped at {MAX_FILE_BYTES} bytes."
                ),
            ],
        }
    );
}

#[tokio::test]
async fn audit_session_start_creates_lock_and_reads_back_session() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let source_identity = current_source_identity_for_tests(&workspace);

    let result = audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "session_id": "session-1",
            "created_at_unix_seconds": 1_725_000_100_i64,
            "findings": [
                {
                    "finding_id": "F-1",
                    "title": "Missing guard",
                    "path": "src/lib.rs",
                    "line": 1,
                    "severity": "medium",
                    "status": "open"
                }
            ]
        }),
    )
    .await;
    let root = workspace_artifact_root(&workspace);
    let expected_session = AuditSessionRecord {
        session_id: "session-1".to_string(),
        created_at_unix_seconds: 1_725_000_100,
        status: AuditSessionRecordStatus::Open,
        source_identity: source_identity.clone(),
        findings: vec![AuditSessionFindingSummary {
            fingerprint: None,
            finding_id: "F-1".to_string(),
            title: "Missing guard".to_string(),
            symbol: None,
            write_set: Vec::new(),
            path: Some("src/lib.rs".to_string()),
            line: Some(1),
            severity: Some("medium".to_string()),
            status: "open".to_string(),
        }],
    };
    let expected_lock = AuditSessionLock {
        session_id: "session-1".to_string(),
        source_identity,
    };

    assert_eq!(
        result,
        AuditSessionStartResult {
            action: crate::audit_build::AuditAction::SessionStart,
            status: AuditSessionStartStatus::Created,
            session: expected_session.clone(),
            lock: expected_lock.clone(),
            session_path: root
                .root_path()
                .join("audit-session.json")
                .display()
                .to_string(),
            lock_path: root
                .root_path()
                .join("audit-session.lock.json")
                .display()
                .to_string(),
            notes: vec![
                "Created one local Ontograph audit session and lock under the existing artifact root."
                    .to_string(),
                "This slice does not dispatch workers, schedule bundles, plan prompts, or expose host/alpha surfaces."
                    .to_string(),
            ],
        }
    );
    assert_eq!(
        serde_json::from_str::<AuditSessionRecord>(
            &std::fs::read_to_string(root.root_path().join("audit-session.json"))
                .expect("read session")
        )
        .expect("parse session"),
        expected_session
    );
    assert_eq!(
        serde_json::from_str::<AuditSessionLock>(
            &std::fs::read_to_string(root.root_path().join("audit-session.lock.json"))
                .expect("read lock")
        )
        .expect("parse lock"),
        expected_lock
    );
}

#[tokio::test]
async fn audit_session_start_reports_stale_existing_lock_without_overwrite() {
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

    let result = audit_session_start_result(
        &workspace,
        json!({
            "action": "session_start",
            "session_id": "session-2",
            "created_at_unix_seconds": 1_725_000_200_i64
        }),
    )
    .await;

    assert_eq!(result.status, AuditSessionStartStatus::StaleSession);
    assert_eq!(result.session.session_id, "session-1".to_string());
    assert_eq!(
        result.session.status,
        AuditSessionRecordStatus::StaleSession
    );
    assert_eq!(
        result.lock.source_identity.generation_id,
        "gen-1".to_string()
    );
    assert_eq!(
        std::fs::read_to_string(root.root_path().join("audit-session.json"))
            .expect("read stale session"),
        original_session
    );
}

#[tokio::test]
async fn audit_session_start_fails_closed_for_corrupt_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    std::fs::create_dir_all(root.root_path()).expect("create ontograph root");
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

    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "session_start",
            "session_id": "session-2"
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("found corrupt audit-session.json"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_session_start_fails_closed_for_incomplete_local_state_without_overwrite() {
    enum ExistingFile {
        Session,
        Lock,
    }

    for existing_file in [ExistingFile::Session, ExistingFile::Lock] {
        let dir = tempdir().expect("temp dir");
        let workspace = dir.path().join("workspace");
        std::fs::create_dir_all(workspace.join("src")).expect("create src");
        std::fs::create_dir(workspace.join(".git")).expect("create git marker");
        std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
        index_workspace_for_tests(&workspace);
        let root = workspace_artifact_root(&workspace);
        std::fs::create_dir_all(root.root_path()).expect("create ontograph root");
        let source_identity = current_source_identity_for_tests(&workspace);
        let session_path = root.root_path().join("audit-session.json");
        let lock_path = root.root_path().join("audit-session.lock.json");
        let (existing_path, missing_path, original) = match existing_file {
            ExistingFile::Session => (
                &session_path,
                &lock_path,
                serde_json::to_string_pretty(&AuditSessionRecord {
                    session_id: "session-1".to_string(),
                    created_at_unix_seconds: 1_725_000_100,
                    status: AuditSessionRecordStatus::Open,
                    source_identity: source_identity.clone(),
                    findings: Vec::new(),
                })
                .expect("serialize session"),
            ),
            ExistingFile::Lock => (
                &lock_path,
                &session_path,
                serde_json::to_string_pretty(&AuditSessionLock {
                    session_id: "session-1".to_string(),
                    source_identity,
                })
                .expect("serialize lock"),
            ),
        };
        std::fs::write(existing_path, &original).expect("write partial audit session state");

        let error = audit_tool_error(
            &workspace,
            json!({
                "action": "session_start",
                "session_id": "session-2"
            }),
        )
        .await;

        assert!(
            error
                .to_string()
                .contains("found incomplete local audit session state"),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(existing_path).expect("read existing partial state"),
            original
        );
        assert!(!missing_path.exists());
    }
}

#[tokio::test]
async fn audit_session_start_fails_closed_for_mismatched_valid_state() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    let root = workspace_artifact_root(&workspace);
    let session = AuditSessionRecord {
        session_id: "session-1".to_string(),
        created_at_unix_seconds: 1_725_000_100,
        status: AuditSessionRecordStatus::Open,
        source_identity: current_source_identity_for_tests(&workspace),
        findings: Vec::new(),
    };
    let lock = AuditSessionLock {
        session_id: "session-1".to_string(),
        source_identity: AuditSessionSourceIdentity {
            generation_id: "other-generation".to_string(),
            source_index_content_hash: "other-hash".to_string(),
            target_head: "other-head".to_string(),
        },
    };
    std::fs::write(
        root.root_path().join("audit-session.json"),
        serde_json::to_string_pretty(&session).expect("serialize session"),
    )
    .expect("write session");
    std::fs::write(
        root.root_path().join("audit-session.lock.json"),
        serde_json::to_string_pretty(&lock).expect("serialize lock"),
    )
    .expect("write lock");
    let original_session = std::fs::read_to_string(root.root_path().join("audit-session.json"))
        .expect("read original session");
    let original_lock = std::fs::read_to_string(root.root_path().join("audit-session.lock.json"))
        .expect("read original lock");

    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "session_start",
            "session_id": "session-2"
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("found mismatched audit session source identity"),
        "unexpected error: {error}"
    );
    assert_eq!(
        std::fs::read_to_string(root.root_path().join("audit-session.json")).expect("read session"),
        original_session
    );
    assert_eq!(
        std::fs::read_to_string(root.root_path().join("audit-session.lock.json"))
            .expect("read lock"),
        original_lock
    );
}
