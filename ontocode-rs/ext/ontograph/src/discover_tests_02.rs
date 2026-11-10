#[tokio::test]
async fn audit_test_gap_returns_inconclusive_when_source_index_is_unavailable() {
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
            .notes
            .iter()
            .any(|note| note.contains("failed to load the native source index")),
        "expected source-index limitation note: {:?}",
        result.notes
    );
}

#[tokio::test]
async fn audit_test_gap_requires_base_ref_for_compare_scope() {
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
                        "action": "test_gap",
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
            "ontograph.audit base_ref is required for action `test_gap` with scope `compare`"
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn audit_test_suggestions_suggests_missing_symbol_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_suggestions_result(
        &workspace,
        json!({
            "action": "test_suggestions",
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

    assert_eq!(result.status, AuditTestSuggestionsStatus::Suggested);
    assert_eq!(
        result.suggestions,
        vec![AuditTestSuggestion {
            target_kind: AuditTestSuggestionTargetKind::Symbol,
            target: "caller".to_string(),
            candidate_test_path: "tests/caller_tests.rs".to_string(),
            assertion_shape: "assert expected behavior for changed symbol `caller`".to_string(),
            missing_tests: vec!["just test -p ontocode-ontograph-extension".to_string()],
        }]
    );
}

#[tokio::test]
async fn audit_test_suggestions_suggests_missing_file_test() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_suggestions_result(
        &workspace,
        json!({
            "action": "test_suggestions",
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

    assert_eq!(result.status, AuditTestSuggestionsStatus::Suggested);
    assert_eq!(
        result.suggestions,
        vec![AuditTestSuggestion {
            target_kind: AuditTestSuggestionTargetKind::File,
            target: "lib.rs".to_string(),
            candidate_test_path: "tests/lib_tests.rs".to_string(),
            assertion_shape: "assert expected behavior for changed file `lib.rs`".to_string(),
            missing_tests: vec!["just test -p ontocode-ontograph-extension".to_string()],
        }]
    );
}

#[tokio::test]
async fn audit_test_suggestions_reports_no_gaps_without_suggestions() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_suggestions_result(
        &workspace,
        json!({
            "action": "test_suggestions",
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

    assert_eq!(result.status, AuditTestSuggestionsStatus::NoGaps);
    assert_eq!(result.source_gap_status, AuditTestGapStatus::Pass);
    assert_eq!(result.suggestions, Vec::new());
}

#[tokio::test]
async fn audit_test_suggestions_are_deterministic_and_report_cap() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_verify_diff_workspace(&workspace);
    git(&workspace, &["add", "lib.rs"]);
    index_workspace_for_tests(&workspace);
    rewrite_verify_diff_workspace(&workspace, "let changed = 1;");
    index_workspace_for_tests(&workspace);

    let result = audit_test_suggestions_result(
        &workspace,
        json!({
            "action": "test_suggestions",
            "scope": "unstaged",
            "executed_tests": [],
            "symbol_test_expectations": {
                "caller": ["symbol-test"]
            },
            "file_test_expectations": {
                "lib.rs": ["file-test"]
            },
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(
        result
            .suggestions
            .iter()
            .map(|suggestion| (&suggestion.target_kind, suggestion.target.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (&AuditTestSuggestionTargetKind::Symbol, "caller"),
            (&AuditTestSuggestionTargetKind::File, "lib.rs"),
        ]
    );
    assert!(
        result
            .limits
            .iter()
            .any(|limit| limit == "test suggestions are capped at 200 items")
    );
}

#[tokio::test]
async fn audit_abi_reports_compatible_schema_addition() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let result = audit_abi_result(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": {
                "name": "Source",
                "fields": {
                    "id": { "type": "string", "nullable": false }
                }
            },
            "target_schema": {
                "name": "Target",
                "fields": {
                    "id": { "type": "string", "nullable": false },
                    "displayName": { "type": "string", "nullable": true }
                }
            }
        }),
    )
    .await;

    assert_eq!(result.status, AuditAbiStatus::Compatible);
    assert_eq!(result.source_name, Some("Source".to_string()));
    assert_eq!(result.target_name, Some("Target".to_string()));
    assert_eq!(
        result.findings,
        vec![AuditAbiFinding {
            kind: AuditAbiFindingKind::FieldAdded,
            field: "displayName".to_string(),
            compatible: true,
            source_type: None,
            target_type: Some("string".to_string()),
            source_nullable: None,
            target_nullable: Some(true),
            detail: "field `displayName` was added".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_abi_reports_removed_field_as_incompatible() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let result = audit_abi_result(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": {
                "fields": {
                    "id": { "type": "string" },
                    "name": { "type": "string" }
                }
            },
            "target_schema": {
                "fields": {
                    "id": { "type": "string" }
                }
            }
        }),
    )
    .await;

    assert_eq!(result.status, AuditAbiStatus::Incompatible);
    assert_eq!(
        result.findings,
        vec![AuditAbiFinding {
            kind: AuditAbiFindingKind::FieldRemoved,
            field: "name".to_string(),
            compatible: false,
            source_type: Some("string".to_string()),
            target_type: None,
            source_nullable: Some(false),
            target_nullable: None,
            detail: "field `name` was removed".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_abi_reports_type_and_nullability_mismatch() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let result = audit_abi_result(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": {
                "fields": {
                    "id": { "type": "string", "nullable": true }
                }
            },
            "target_schema": {
                "fields": {
                    "id": { "type": "number", "nullable": false }
                }
            }
        }),
    )
    .await;

    assert_eq!(result.status, AuditAbiStatus::Incompatible);
    assert_eq!(
        result
            .findings
            .iter()
            .map(|finding| &finding.kind)
            .collect::<Vec<_>>(),
        vec![
            &AuditAbiFindingKind::TypeChanged,
            &AuditAbiFindingKind::NullabilityChanged,
        ]
    );
}

#[tokio::test]
async fn audit_abi_fails_closed_for_missing_or_unsupported_input() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let missing = audit_abi_error(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": { "fields": {} }
        }),
    )
    .await;
    assert!(
        missing
            .to_string()
            .contains("target_schema is required for action `abi`"),
        "unexpected error: {missing}"
    );

    let unsupported = audit_abi_error(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": "interface Source { id: string }",
            "target_schema": { "fields": {} }
        }),
    )
    .await;
    assert!(
        unsupported
            .to_string()
            .contains("source_schema must be a normalized JSON schema object"),
        "unexpected error: {unsupported}"
    );

    let malformed = audit_abi_error(
        &workspace,
        json!({
            "action": "abi",
            "source_schema": { "fields": { "id": "string" } },
            "target_schema": { "fields": {} }
        }),
    )
    .await;
    assert!(
        malformed.to_string().contains("source_schema is malformed"),
        "unexpected error: {malformed}"
    );
}

#[tokio::test]
async fn audit_path_verify_passes_when_required_pattern_follows_trigger() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_path_verify_fixture(&workspace, "fn run() {\n    start();\n    finish();\n}\n");

    let result = audit_path_verify_result(
        &workspace,
        json!({
            "action": "path_verify",
            "path": "src/lib.rs",
            "trigger_pattern": "start();",
            "required_patterns": ["finish();"],
            "forbidden_patterns": ["panic!"],
            "window_lines": 5
        }),
    )
    .await;

    assert_eq!(result.status, AuditPathVerifyStatus::Pass);
    assert_eq!(result.trigger_line, Some(2));
    assert_eq!(result.checked_lines, 2);
    assert_eq!(result.findings, Vec::new());
}

#[tokio::test]
async fn audit_path_verify_fails_when_forbidden_pattern_follows_trigger() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_path_verify_fixture(&workspace, "fn run() {\n    start();\n    panic!();\n}\n");

    let result = audit_path_verify_result(
        &workspace,
        json!({
            "action": "path_verify",
            "path": "src/lib.rs",
            "trigger_pattern": "start();",
            "forbidden_patterns": ["panic!"],
            "window_lines": 5
        }),
    )
    .await;

    assert_eq!(result.status, AuditPathVerifyStatus::Fail);
    assert_eq!(
        result.findings,
        vec![AuditPathVerifyFinding {
            kind: AuditPathVerifyFindingKind::ForbiddenPatternPresent,
            pattern: "panic!".to_string(),
            line: Some(3),
            detail: "forbidden pattern `panic!` was found after trigger within the bounded window"
                .to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_path_verify_reports_missing_trigger_as_inconclusive() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_path_verify_fixture(&workspace, "fn run() {\n    finish();\n}\n");

    let result = audit_path_verify_result(
        &workspace,
        json!({
            "action": "path_verify",
            "path": "src/lib.rs",
            "trigger_pattern": "start();",
            "required_patterns": ["finish();"],
            "window_lines": 5
        }),
    )
    .await;

    assert_eq!(result.status, AuditPathVerifyStatus::Inconclusive);
    assert_eq!(result.trigger_line, None);
    assert_eq!(result.checked_lines, 0);
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("Trigger pattern was not found"))
    );
}

#[tokio::test]
async fn audit_path_verify_respects_bounded_window() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_path_verify_fixture(
        &workspace,
        "fn run() {\n    start();\n    unrelated();\n    finish();\n}\n",
    );

    let result = audit_path_verify_result(
        &workspace,
        json!({
            "action": "path_verify",
            "path": "src/lib.rs",
            "trigger_pattern": "start();",
            "required_patterns": ["finish();"],
            "window_lines": 1
        }),
    )
    .await;

    assert_eq!(result.status, AuditPathVerifyStatus::Fail);
    assert_eq!(result.checked_lines, 1);
    assert!(result.limits.iter().any(|limit| limit == "window_lines=1"));
}

#[tokio::test]
async fn audit_path_verify_rejects_absolute_or_traversal_paths() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_path_verify_fixture(&workspace, "fn run() {}\n");

    let error = audit_path_verify_error(
        &workspace,
        json!({
            "action": "path_verify",
            "path": "../src/lib.rs",
            "trigger_pattern": "run",
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("path must be a repo-relative file path without traversal"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_pressure_reports_quota_and_global_side_effect_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_pressure_fixture(
        &workspace,
        "fn run(active_count: usize, max_active: usize) {\n    if active_count > max_active {}\n    std::env::set_var(\"A\", \"B\");\n}\n",
    );

    let result = audit_pressure_result(&workspace, json!({"action": "pressure"})).await;

    assert_eq!(result.findings.len(), 2);
    assert_eq!(
        result.findings[0],
        AuditPressureFinding {
            category: "quota".to_string(),
            rule: "quota_or_capacity_bound".to_string(),
            path: "src/lib.rs".to_string(),
            line: 2,
            snippet: "if active_count > max_active {}".to_string(),
            reason:
                "Quota-like max/limit/capacity check near count-like state; verify global pressure impact."
                    .to_string(),
        }
    );
    assert_eq!(result.findings[1].category, "global_side_effect");
}

#[tokio::test]
async fn audit_pressure_reports_no_findings_for_plain_rust() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_pressure_fixture(
        &workspace,
        "fn run(value: usize) -> usize {\n    value + 1\n}\n",
    );

    let result = audit_pressure_result(&workspace, json!({"action": "pressure"})).await;

    assert_eq!(result.total_candidates, 0);
    assert_eq!(result.findings, Vec::new());
    assert_eq!(result.skipped, Vec::new());
    assert_eq!(result.truncated, false);
}

#[tokio::test]
async fn audit_pressure_reports_truncation_when_candidate_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_pressure_fixture(
        &workspace,
        "fn run(active_count: usize, max_active: usize, pending_count: usize, max_pending: usize) {\n    if active_count > max_active {}\n    if pending_count > max_pending {}\n}\n",
    );

    let result = audit_pressure_result(
        &workspace,
        json!({
            "action": "pressure",
            "max_items": 1
        }),
    )
    .await;

    assert_eq!(result.total_candidates, 2);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.truncated, true);
}

#[tokio::test]
async fn audit_pressure_skips_ambiguous_quota_patterns() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_pressure_fixture(&workspace, "fn run() {\n    let capacity_hint = 2;\n}\n");

    let result = audit_pressure_result(&workspace, json!({"action": "pressure"})).await;

    assert_eq!(result.findings, Vec::new());
    assert_eq!(
        result.skipped,
        vec![AuditPressureSkippedRecord {
            category: "quota".to_string(),
            path: "src/lib.rs".to_string(),
            line: 2,
            snippet: "let capacity_hint = 2;".to_string(),
            reason:
                "Quota-like wording lacks an explicit bound or count-like state in this shallow scan."
                    .to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_concurrency_reports_lock_acquisition_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_concurrency_fixture(
        &workspace,
        "fn run(state: &Mutex<usize>) {\n    let guard = state.lock().unwrap();\n    let _ = *guard;\n}\n",
    );

    let result = audit_concurrency_result(&workspace, json!({"action": "concurrency"})).await;

    assert_eq!(result.total_candidates, 1);
    assert_eq!(
        result.findings,
        vec![AuditConcurrencyFinding {
            category: "lock_scope".to_string(),
            rule: "lock_acquisition".to_string(),
            path: "src/lib.rs".to_string(),
            line: 2,
            snippet: "let guard = state.lock().unwrap();".to_string(),
            reason:
                "Explicit lock acquisition; verify the guard scope stays bounded and does not hold across await or blocking work."
                    .to_string(),
        }]
    );
    assert_eq!(result.truncated, false);
}

#[tokio::test]
async fn audit_concurrency_reports_no_findings_for_plain_rust() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_concurrency_fixture(
        &workspace,
        "fn run(value: usize) -> usize {\n    value + 1\n}\n",
    );

    let result = audit_concurrency_result(&workspace, json!({"action": "concurrency"})).await;

    assert_eq!(result.total_candidates, 0);
    assert_eq!(result.findings, Vec::new());
    assert_eq!(result.truncated, false);
}
