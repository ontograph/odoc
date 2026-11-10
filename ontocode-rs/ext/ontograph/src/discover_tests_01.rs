#[tokio::test]
async fn impact_route_tool_fails_closed_for_ambiguous_in_file_handler_names() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"use axum::routing::get;

fn build_router() -> axum::Router {
    axum::Router::new()
        .route("/run", get(run))
}

async fn run() {}

mod nested {
    pub async fn run() {}
}
"#,
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(2048),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "route",
                        "route": "/run"
                    })
                    .to_string(),
                },
            })
            .await,
        "impact route should fail closed for ambiguous handler names in one file",
    );

    assert!(
        err.to_string()
            .contains("matched route handler is ambiguous within source-index.json"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn impact_diff_tool_reads_changed_symbols_from_git_diff() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    init_git_workspace(&workspace);
    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n}\nfn callee() {}\n",
    )
    .expect("write source");
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);

    std::fs::write(
        workspace.join("lib.rs"),
        "fn caller() {\n    callee();\n    let changed = 1;\n}\nfn callee() {}\n",
    )
    .expect("rewrite source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "diff",
            "scope": "unstaged",
            "max_items": 10
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(4096),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("impact diff should read active source index");

    let result = serde_json::from_value::<ImpactDiffResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        ImpactDiffResult {
            summary: ImpactDiffSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            changed_files: vec![ImpactDiffFileResult {
                path: "lib.rs".to_string(),
                changed_symbols: vec![ImpactDiffSymbolResult {
                    symbol: "caller".to_string(),
                    kind: "fn".to_string(),
                    path: "lib.rs".to_string(),
                    line: 1,
                }],
                changed_symbols_truncated: false,
            }],
            affected_processes: vec![ImpactDiffProcessResult {
                name: "caller -> callee".to_string(),
                entry_symbol: "caller".to_string(),
                path: "lib.rs".to_string(),
                changed_steps: vec!["caller".to_string()],
            }],
            changed_files_truncated: false,
            affected_processes_truncated: false,
            warnings: Vec::new(),
            limits: vec![
                "changed files are capped at 200 items".to_string(),
                "changed symbols are capped at 200 items per file".to_string(),
                "affected processes are capped at 200 items".to_string(),
                "impact.diff supports scopes `unstaged`, `staged`, `all`, and `compare`"
                    .to_string(),
                "impact.diff reads changed files from git diff and changed symbols from the active native source-index artifact"
                    .to_string(),
                "The native Rust index resolves unqualified calls and some path-qualified calls; receiver dispatch, cross-file imports, macros, and non-Rust files remain bounded or unresolved."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn impact_diff_tool_requires_base_ref_for_compare_scope() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    init_git_workspace(&workspace);
    std::fs::write(workspace.join("lib.rs"), "fn caller() {}\n").expect("write source");
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, IMPACT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, IMPACT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(4096),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "diff",
                        "scope": "compare"
                    })
                    .to_string(),
                },
            })
            .await,
        "compare scope without base_ref should fail closed",
    );

    assert!(
        err.to_string().contains(
            "ontograph.impact base_ref is required for action `diff` with scope `compare`"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn audit_verify_diff_passes_when_expected_diff_and_tests_match() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": ["caller"],
            "expected_tests": ["just test -p ontocode-ontograph-extension"],
            "executed_tests": ["just test -p ontocode-ontograph-extension"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditVerifyDiffResult {
            action: crate::audit_build::AuditAction::VerifyDiff,
            status: AuditVerifyDiffStatus::Pass,
            summary: AuditVerifyDiffSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            missing_expected_files: Vec::new(),
            unexpected_changed_files: Vec::new(),
            missing_expected_symbols: Vec::new(),
            unexpected_changed_symbols: Vec::new(),
            missing_expected_tests: Vec::new(),
            changed_files_truncated: false,
            changed_symbols_truncated: false,
            warnings: Vec::new(),
            limits: verify_diff_expected_limits(),
            notes: verify_diff_expected_notes(),
        }
    );
}

#[tokio::test]
async fn audit_verify_diff_reports_missing_expected_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs", "other.rs"],
            "expected_symbols": ["caller"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Fail);
    assert_eq!(result.missing_expected_files, vec!["other.rs".to_string()]);
    assert_eq!(result.unexpected_changed_files, Vec::<String>::new());
}

#[tokio::test]
async fn audit_verify_diff_reports_unexpected_changed_file() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    std::fs::write(workspace.join("extra.rs"), "fn extra() {}\n").expect("write extra source");
    git(&workspace, &["add", "lib.rs", "extra.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    std::fs::write(
        workspace.join("extra.rs"),
        "fn extra() {\n    let changed = 2;\n}\n",
    )
    .expect("rewrite extra source");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": ["caller", "extra"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Fail);
    assert_eq!(
        result.unexpected_changed_files,
        vec!["extra.rs".to_string()]
    );
}

#[tokio::test]
async fn audit_verify_diff_reports_missing_expected_symbol() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": ["caller", "callee"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Fail);
    assert_eq!(result.missing_expected_symbols, vec!["callee".to_string()]);
    assert_eq!(result.unexpected_changed_symbols, Vec::<String>::new());
}

#[tokio::test]
async fn audit_verify_diff_reports_unexpected_changed_symbol() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": [],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Fail);
    assert_eq!(
        result.unexpected_changed_symbols,
        vec!["caller".to_string()]
    );
}

#[tokio::test]
async fn audit_verify_diff_reports_missing_expected_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": ["caller"],
            "expected_tests": ["just test -p ontocode-ontograph-extension"],
            "executed_tests": [],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Fail);
    assert_eq!(
        result.missing_expected_tests,
        vec!["just test -p ontocode-ontograph-extension".to_string()]
    );
}

#[tokio::test]
async fn audit_verify_diff_reports_untracked_unmapped_paths_as_inconclusive() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    std::fs::write(workspace.join("untracked.txt"), "untracked notes\n")
        .expect("write untracked source");
    git(&workspace, &["add", "-N", "untracked.txt"]);

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": [],
            "expected_symbols": [],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Inconclusive);
    assert_eq!(result.summary.changed_files, 0);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("not present in the active Rust source index")),
        "expected unmapped path warning: {:?}",
        result.warnings
    );
}

#[tokio::test]
async fn audit_verify_diff_returns_inconclusive_when_source_index_is_unavailable() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    let (generation_id, _) = current_source_index_metadata_for_tests(&workspace);
    let source_index_path = workspace_artifact_root(&workspace)
        .generation_file_path(&generation_id, crate::manifest::SOURCE_INDEX_FILE)
        .expect("source-index path");
    std::fs::write(source_index_path, "{not json").expect("corrupt source-index");

    let result = audit_verify_diff_result(
        &workspace,
        json!({
            "action": "verify_diff",
            "scope": "unstaged",
            "expected_files": ["lib.rs"],
            "expected_symbols": ["caller"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditVerifyDiffStatus::Inconclusive);
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("failed to load the native source index")),
        "expected source-index limitation note: {:?}",
        result.notes
    );
}

#[tokio::test]
async fn audit_verify_diff_requires_base_ref_for_compare_scope() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);

    let err = expect_tool_error(
        install_and_find_named_tool(&workspace, AUDIT_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, AUDIT_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(4096),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload: ToolPayload::Function {
                    arguments: json!({
                        "action": "verify_diff",
                        "scope": "compare"
                    })
                    .to_string(),
                },
            })
            .await,
        "compare scope without base_ref should fail closed",
    );

    assert!(
        err.to_string().contains(
            "ontograph.audit base_ref is required for action `verify_diff` with scope `compare`"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn audit_test_gap_passes_when_expected_tests_were_executed() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_gap_result(
        &workspace,
        json!({
            "action": "test_gap",
            "scope": "unstaged",
            "executed_tests": ["just test -p ontocode-ontograph-extension"],
            "symbol_test_expectations": {
                "caller": ["just test -p ontocode-ontograph-extension"]
            },
            "file_test_expectations": {
                "lib.rs": ["just test -p ontocode-ontograph-extension"]
            },
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditTestGapResult {
            action: crate::audit_build::AuditAction::TestGap,
            status: AuditTestGapStatus::Pass,
            summary: AuditTestGapSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            missing_symbol_tests: Vec::new(),
            missing_file_tests: Vec::new(),
            symbols_without_expectations: Vec::new(),
            files_without_expectations: Vec::new(),
            changed_files_truncated: false,
            changed_symbols_truncated: false,
            warnings: Vec::new(),
            limits: test_gap_expected_limits(),
            notes: test_gap_expected_notes(),
        }
    );
}

#[tokio::test]
async fn audit_test_gap_reports_missing_symbol_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_gap_result(
        &workspace,
        json!({
            "action": "test_gap",
            "scope": "unstaged",
            "executed_tests": [],
            "symbol_test_expectations": {
                "caller": ["just test -p ontocode-ontograph-extension"]
            },
            "file_test_expectations": {
                "lib.rs": []
            },
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditTestGapStatus::Fail);
    assert_eq!(
        result.missing_symbol_tests,
        vec![AuditTestGapMissingSymbolTests {
            symbol: "caller".to_string(),
            missing_tests: vec!["just test -p ontocode-ontograph-extension".to_string()],
        }]
    );
}

#[tokio::test]
async fn audit_test_gap_reports_missing_file_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_gap_result(
        &workspace,
        json!({
            "action": "test_gap",
            "scope": "unstaged",
            "executed_tests": [],
            "symbol_test_expectations": {
                "caller": []
            },
            "file_test_expectations": {
                "lib.rs": ["just test -p ontocode-ontograph-extension"]
            },
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditTestGapStatus::Fail);
    assert_eq!(
        result.missing_file_tests,
        vec![AuditTestGapMissingFileTests {
            path: "lib.rs".to_string(),
            missing_tests: vec!["just test -p ontocode-ontograph-extension".to_string()],
        }]
    );
}

#[tokio::test]
async fn audit_test_gap_reports_changed_items_without_expectations() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_gap_result(
        &workspace,
        json!({
            "action": "test_gap",
            "scope": "unstaged",
            "executed_tests": ["just test -p ontocode-ontograph-extension"],
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditTestGapStatus::Fail);
    assert_eq!(
        result.symbols_without_expectations,
        vec!["caller".to_string()]
    );
    assert_eq!(
        result.files_without_expectations,
        vec!["lib.rs".to_string()]
    );
}

#[tokio::test]
async fn audit_test_gap_reports_unmapped_paths_as_inconclusive() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    std::fs::write(workspace.join("untracked.txt"), "untracked notes\n")
        .expect("write untracked source");
    git(&workspace, &["add", "-N", "untracked.txt"]);

    let result = audit_test_gap_result(
        &workspace,
        json!({
            "action": "test_gap",
            "scope": "unstaged",
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(result.status, AuditTestGapStatus::Inconclusive);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("not present in the active Rust source index")),
        "expected unmapped path warning: {:?}",
        result.warnings
    );
}
