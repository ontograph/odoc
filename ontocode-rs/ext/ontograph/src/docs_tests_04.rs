#[tokio::test]
async fn audit_error_topology_emits_skipped_records_for_ambiguous_or_unsupported_patterns() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        r#"
fn sample() -> Result<(), FunctionCallError> {
    panic!("not an error topology edge");
    return Err(FunctionCallError::RespondToModel("bad".to_string()));
}
"#,
    )
    .expect("write source");

    let result =
        audit_error_topology_result(&workspace, json!({ "action": "error_topology" })).await;

    assert_eq!(
        result.skipped,
        vec![
            AuditErrorTopologySkippedRecord {
                category: "unsupported".to_string(),
                path: "src/lib.rs".to_string(),
                line: 3,
                snippet: "panic!(\"not an error topology edge\");".to_string(),
                reason: "Error-like panic or assertion syntax is not an error topology edge."
                    .to_string(),
            },
            AuditErrorTopologySkippedRecord {
                category: "ambiguous".to_string(),
                path: "src/lib.rs".to_string(),
                line: 4,
                snippet: "return Err(FunctionCallError::RespondToModel(\"bad\".to_string()));"
                    .to_string(),
                reason:
                    "Line matches multiple error-topology categories; this MVP does not infer edge direction."
                        .to_string(),
            },
        ]
    );
    assert!(result.findings.is_empty());
}

#[tokio::test]
async fn audit_extract_fsm_reports_enum_assignments_and_transition_candidates() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "enum Phase {\n    Idle,\n    Running,\n    Done,\n}\n\nstruct Machine { state: Phase }\n\nimpl Machine {\n    fn tick(&mut self) {\n        if matches!(self.state, Phase::Idle) {\n            self.state = Phase::Running;\n        }\n        match self.state {\n            Phase::Running => { self.state = Phase::Done; }\n            Phase::Done => {}\n        }\n    }\n}\n",
    )
    .expect("write source");

    let result = audit_fsm_result(
        &workspace,
        json!({
            "action": "extract_fsm",
            "max_items": 10
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditFsmResult {
            action: crate::audit_build::AuditAction::ExtractFsm,
            scanned_files: 1,
            total_candidates: 6,
            findings: vec![
                AuditFsmFinding {
                    kind: "enum_state".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 2,
                    snippet: "Idle,".to_string(),
                    state: "Phase::Idle".to_string(),
                    source_state: None,
                    target_state: None,
                    variable: None,
                    reason: "State-like enum variant declared explicitly.".to_string(),
                },
                AuditFsmFinding {
                    kind: "enum_state".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 3,
                    snippet: "Running,".to_string(),
                    state: "Phase::Running".to_string(),
                    source_state: None,
                    target_state: None,
                    variable: None,
                    reason: "State-like enum variant declared explicitly.".to_string(),
                },
                AuditFsmFinding {
                    kind: "enum_state".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 4,
                    snippet: "Done,".to_string(),
                    state: "Phase::Done".to_string(),
                    source_state: None,
                    target_state: None,
                    variable: None,
                    reason: "State-like enum variant declared explicitly.".to_string(),
                },
                AuditFsmFinding {
                    kind: "transition_candidate".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 12,
                    snippet: "self.state = Phase::Running;".to_string(),
                    state: "Phase::Running".to_string(),
                    source_state: Some("Phase::Idle".to_string()),
                    target_state: Some("Phase::Running".to_string()),
                    variable: None,
                    reason: "A guard on one explicit state is followed by an explicit state assignment within 8 lines.".to_string(),
                },
                AuditFsmFinding {
                    kind: "state_assignment".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 12,
                    snippet: "self.state = Phase::Running;".to_string(),
                    state: "Phase::Running".to_string(),
                    source_state: None,
                    target_state: Some("Phase::Running".to_string()),
                    variable: Some("self.state".to_string()),
                    reason: "State-like variable is assigned an explicit enum variant."
                        .to_string(),
                },
                AuditFsmFinding {
                    kind: "transition_candidate".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 15,
                    snippet: "Phase::Running => { self.state = Phase::Done; }".to_string(),
                    state: "Phase::Done".to_string(),
                    source_state: Some("Phase::Running".to_string()),
                    target_state: Some("Phase::Done".to_string()),
                    variable: None,
                    reason:
                        "A match arm maps one explicit state to one explicit state assignment."
                            .to_string(),
                },
            ],
            skipped: Vec::new(),
            truncated: false,
            limits: vec![
                "max_items=10".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "lookahead_lines=8".to_string(),
            ],
            notes: vec![
                "This native slice scans Rust source files only; docs, graph facts, lifecycle state, and donor runtime semantics are intentionally out of scope.".to_string(),
                "Findings are shallow syntax evidence for state-like enums, explicit state assignments, and branch-to-assignment transition candidates only.".to_string(),
                "Unsupported or ambiguous state updates are emitted as skipped records instead of being inferred.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_extract_fsm_reports_no_findings_for_non_fsm_fixture() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "enum Color {\n    Red,\n    Blue,\n}\n\nfn add(left: usize, right: usize) -> usize {\n    left + right\n}\n",
    )
    .expect("write source");

    let result = audit_fsm_result(
        &workspace,
        json!({
            "action": "extract_fsm",
            "max_items": 5
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditFsmResult {
            action: crate::audit_build::AuditAction::ExtractFsm,
            scanned_files: 1,
            total_candidates: 0,
            findings: Vec::new(),
            skipped: Vec::new(),
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "lookahead_lines=8".to_string(),
            ],
            notes: vec![
                "This native slice scans Rust source files only; docs, graph facts, lifecycle state, and donor runtime semantics are intentionally out of scope.".to_string(),
                "Findings are shallow syntax evidence for state-like enums, explicit state assignments, and branch-to-assignment transition candidates only.".to_string(),
                "Unsupported or ambiguous state updates are emitted as skipped records instead of being inferred.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_extract_fsm_reports_truncation_when_candidate_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "enum MachineState {\n    Idle,\n    Running,\n    Paused,\n    Done,\n}\n",
    )
    .expect("write source");

    let result = audit_fsm_result(
        &workspace,
        json!({
            "action": "extract_fsm",
            "max_items": 2
        }),
    )
    .await;

    assert_eq!(result.scanned_files, 1);
    assert_eq!(result.total_candidates, 4);
    assert_eq!(
        result.findings,
        vec![
            AuditFsmFinding {
                kind: "enum_state".to_string(),
                path: "src/lib.rs".to_string(),
                line: 2,
                snippet: "Idle,".to_string(),
                state: "MachineState::Idle".to_string(),
                source_state: None,
                target_state: None,
                variable: None,
                reason: "State-like enum variant declared explicitly.".to_string(),
            },
            AuditFsmFinding {
                kind: "enum_state".to_string(),
                path: "src/lib.rs".to_string(),
                line: 3,
                snippet: "Running,".to_string(),
                state: "MachineState::Running".to_string(),
                source_state: None,
                target_state: None,
                variable: None,
                reason: "State-like enum variant declared explicitly.".to_string(),
            },
        ]
    );
    assert_eq!(result.skipped, Vec::new());
    assert!(result.truncated);
}

#[tokio::test]
async fn audit_extract_fsm_emits_skipped_records_for_ambiguous_or_unsupported_updates() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "struct Machine { state: Phase }\n\nimpl Machine {\n    fn choose(&mut self, ready: bool) {\n        self.state = if ready { Phase::Ready } else { Phase::Failed };\n        self.state = compute_next();\n    }\n}\n",
    )
    .expect("write source");

    let result = audit_fsm_result(
        &workspace,
        json!({
            "action": "extract_fsm",
            "max_items": 5
        }),
    )
    .await;

    assert_eq!(
        result.skipped,
        vec![
            AuditFsmSkippedRecord {
                category: "ambiguous".to_string(),
                path: "src/lib.rs".to_string(),
                line: 5,
                snippet: "self.state = if ready { Phase::Ready } else { Phase::Failed };"
                    .to_string(),
                reason: "State-like assignment mentions multiple enum variants.".to_string(),
            },
            AuditFsmSkippedRecord {
                category: "unsupported".to_string(),
                path: "src/lib.rs".to_string(),
                line: 6,
                snippet: "self.state = compute_next();".to_string(),
                reason: "State-like assignment target is not an explicit enum variant.".to_string(),
            },
        ]
    );
    assert_eq!(result.findings, Vec::new());
    assert_eq!(result.total_candidates, 2);
    assert!(!result.truncated);
}

#[tokio::test]
async fn audit_logic_reports_bounded_rust_logic_findings() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "use std::fs::File;\nuse std::process::Command;\n\nfn risky(path: &std::path::Path, lock: &std::sync::Mutex<()>) {\n    if path.exists() {\n        let _file = File::open(path).unwrap();\n    }\n    let _guard = lock.lock().unwrap();\n    Command::new(\"git\").status().unwrap();\n}\n",
    )
    .expect("write source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "logic",
            "max_items": 5
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
        .expect("audit logic should scan Rust source");

    let result = serde_json::from_value::<AuditLogicResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditLogicResult {
            action: crate::audit_build::AuditAction::Logic,
            scanned_files: 1,
            total_candidates: 3,
            findings: vec![
                AuditLogicFinding {
                    category: "toctou".to_string(),
                    rule: "check_then_file_use_window".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 6,
                    snippet: "let _file = File::open(path).unwrap();".to_string(),
                    reason: "File use follows a prior existence or metadata check within 6 lines; verify the path cannot change between check and use.".to_string(),
                },
                AuditLogicFinding {
                    category: "resource-leaks".to_string(),
                    rule: "rust_resource_acquisition_review".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 6,
                    snippet: "let _file = File::open(path).unwrap();".to_string(),
                    reason: "Resource acquisition is present; verify ownership, handoff, or cleanup because this MVP does not prove resource lifetime.".to_string(),
                },
                AuditLogicFinding {
                    category: "concurrency".to_string(),
                    rule: "blocking_work_after_lock".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 9,
                    snippet: "Command::new(\"git\").status().unwrap();".to_string(),
                    reason: "Blocking I/O, sleep, or process work follows a visible lock acquisition within 6 lines; verify it does not run under the lock guard.".to_string(),
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "lookahead_lines=6".to_string(),
            ],
            notes: vec![
                "This native slice scans Rust source files only; docs, facts, lifecycle state, and graph semantics are intentionally out of scope.".to_string(),
                "Implemented MVP categories: resource-leaks, toctou, concurrency.".to_string(),
                "Deferred donor categories for later slices: fork-safety and signals.".to_string(),
                "Findings are static evidence only and do not change audit lifecycle status.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_tech_debt_ranks_rust_functions_by_structure_and_callers() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn risky(alpha: usize, beta: usize, gamma: usize, delta: usize) -> usize {\n    let mut total = alpha + beta;\n    total += gamma;\n    total += delta;\n    total += helper();\n    total += helper_two();\n    total\n}\n\nfn helper() -> usize { 1 }\nfn helper_two() -> usize { 1 }\n\npub fn caller_one() -> usize {\n    risky(1, 2, 3, 4)\n}\n\npub fn caller_two() -> usize {\n    risky(4, 3, 2, 1)\n}\n",
    )
    .expect("write source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "tech_debt",
            "max_items": 3,
            "min_lines": 2,
            "since": "6 months ago"
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
        .expect("audit tech_debt should rank Rust functions");

    let result = serde_json::from_value::<AuditTechDebtResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditTechDebtResult {
            action: crate::audit_build::AuditAction::TechDebt,
            since: "6 months ago".to_string(),
            min_lines: 2,
            scanned_files: 1,
            total_candidates: 3,
            findings: vec![
                AuditTechDebtFinding {
                    symbol: "risky".to_string(),
                    kind: "fn".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 1,
                    line_count: 8,
                    parameter_count: 4,
                    caller_count: 2,
                    commit_count: 0,
                    score_millis: 2800,
                },
                AuditTechDebtFinding {
                    symbol: "caller_one".to_string(),
                    kind: "fn".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 13,
                    line_count: 3,
                    parameter_count: 0,
                    caller_count: 0,
                    commit_count: 0,
                    score_millis: 150,
                },
                AuditTechDebtFinding {
                    symbol: "caller_two".to_string(),
                    kind: "fn".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 17,
                    line_count: 3,
                    parameter_count: 0,
                    caller_count: 0,
                    commit_count: 0,
                    score_millis: 150,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=3".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "max_since_chars=120".to_string(),
            ],
            notes: vec![
                "This native slice ranks Rust functions only; other languages and docs are intentionally out of scope."
                    .to_string(),
                "Caller counts reuse the existing unique-name Rust call heuristic, so duplicate function names stay conservative."
                    .to_string(),
                "Scores are scaled by 1000 after combining line count, call-site count, parameter count, and git churn."
                    .to_string(),
                "Git churn was unavailable for this workspace view, so scores fell back to structural signals only."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_hotspots_ranks_rust_files_by_aggregated_structure() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/hot.rs"),
        "pub fn risky(alpha: usize, beta: usize, gamma: usize, delta: usize) -> usize {\n    let mut total = alpha + beta;\n    total += gamma;\n    total += delta;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total\n}\n\npub fn caller_one() -> usize {\n    let mut total = 0;\n    total += risky(1, 2, 3, 4);\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total\n}\n",
    )
    .expect("write hot source");
    std::fs::write(
        workspace.join("src/cold.rs"),
        "pub fn steady() -> usize {\n    let mut total = 0;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    total\n}\n",
    )
    .expect("write cold source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "hotspots",
            "max_items": 2,
            "since": "6 months ago"
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
        .expect("audit hotspots should rank Rust files");

    let result = serde_json::from_value::<AuditHotspotsResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditHotspotsResult {
            action: crate::audit_build::AuditAction::Hotspots,
            since: "6 months ago".to_string(),
            scanned_files: 2,
            total_candidate_files: 2,
            findings: vec![
                AuditHotspotFinding {
                    path: "src/hot.rs".to_string(),
                    function_count: 2,
                    total_line_count: 23,
                    total_caller_count: 1,
                    commit_count: 0,
                    score_millis: 3350,
                },
                AuditHotspotFinding {
                    path: "src/cold.rs".to_string(),
                    function_count: 1,
                    total_line_count: 12,
                    total_caller_count: 0,
                    commit_count: 0,
                    score_millis: 600,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=2".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
                "default_min_function_lines=10".to_string(),
                "max_since_chars=120".to_string(),
            ],
            notes: vec![
                "This native slice ranks Rust files only; other languages and docs are intentionally out of scope."
                    .to_string(),
                "File hotspots sum the same bounded structural signals already used by audit.tech_debt."
                    .to_string(),
                "Only functions meeting the default minimum line threshold contribute to hotspot ranking."
                    .to_string(),
                "Git churn was unavailable for this workspace view, so hotspot scores fell back to structural signals only."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_dead_code_reports_only_unique_private_functions_without_callers() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn entry() -> usize {\n    used_helper()\n}\n\nfn used_helper() -> usize {\n    1\n}\n\nfn long_dead() -> usize {\n    let base = 1;\n    let extra = 2;\n    base + extra\n}\n\nfn dup() -> usize {\n    1\n}\n\npub fn exported() -> usize {\n    9\n}\n",
    )
    .expect("write lib source");
    std::fs::write(
        workspace.join("src/other.rs"),
        "fn short_dead() -> usize {\n    7\n}\n\nfn dup() -> usize {\n    2\n}\n",
    )
    .expect("write other source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "dead_code",
            "max_items": 5
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
        .expect("audit dead_code should report bounded private orphan functions");

    let result = serde_json::from_value::<AuditDeadCodeResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        AuditDeadCodeResult {
            action: crate::audit_build::AuditAction::DeadCode,
            scanned_files: 2,
            total_candidates: 2,
            findings: vec![
                AuditDeadCodeFinding {
                    symbol: "long_dead".to_string(),
                    kind: "fn".to_string(),
                    path: "src/lib.rs".to_string(),
                    line: 9,
                    line_count: 5,
                },
                AuditDeadCodeFinding {
                    symbol: "short_dead".to_string(),
                    kind: "fn".to_string(),
                    path: "src/other.rs".to_string(),
                    line: 1,
                    line_count: 3,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                format!("max_file_bytes={MAX_FILE_BYTES}"),
            ],
            notes: vec![
                "This native slice reports Rust functions only; other languages and docs are intentionally out of scope."
                    .to_string(),
                "Only unique-name private functions with zero observed callers under the current bounded Rust call heuristic are reported."
                    .to_string(),
                "Public functions and duplicate function names are excluded to avoid overclaiming reachability."
                    .to_string(),
            ],
        }
    );
}
