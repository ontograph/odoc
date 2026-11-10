#[tokio::test]
async fn audit_cycles_reports_sccs_and_self_loops_from_graph_facts() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    beta()\n}\n\npub fn beta() -> usize {\n    alpha()\n}\n\npub fn self_loop() -> usize {\n    self_loop()\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_cycles_result(
        &workspace,
        json!({
            "action": "cycles",
            "max_items": 5
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditCyclesResult {
            action: crate::audit_build::AuditAction::Cycles,
            total_cycles: 2,
            findings: vec![
                AuditCycleFinding {
                    nodes: vec![
                        "symbol:src/lib.rs:1:fn:alpha".to_string(),
                        "symbol:src/lib.rs:5:fn:beta".to_string(),
                    ],
                    node_count: 2,
                    edge_count: 2,
                    truncated_nodes: false,
                },
                AuditCycleFinding {
                    nodes: vec!["symbol:src/lib.rs:9:fn:self_loop".to_string()],
                    node_count: 1,
                    edge_count: 1,
                    truncated_nodes: false,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                "max_cycle_sccs=50".to_string(),
                "max_nodes_per_cycle=20".to_string(),
            ],
            notes: vec![
                "This native slice reports SCCs from existing resolved graph CALLS facts only."
                    .to_string(),
                "Unresolved call facts are intentionally not guessed into cycle edges.".to_string(),
                "Only SCCs with more than one node or a self-loop are reported.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_cycles_reports_truncation_when_cycle_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    beta()\n}\n\npub fn beta() -> usize {\n    alpha()\n}\n\npub fn self_loop() -> usize {\n    self_loop()\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_cycles_result(
        &workspace,
        json!({
            "action": "cycles",
            "max_items": 1
        }),
    )
    .await;

    assert_eq!(result.total_cycles, 2);
    assert_eq!(result.findings.len(), 1);
    assert!(result.truncated);
}

#[tokio::test]
async fn audit_cycles_reports_no_findings_for_acyclic_graph() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "pub fn alpha() -> usize {\n    beta()\n}\n\npub fn beta() -> usize {\n    1\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_cycles_result(
        &workspace,
        json!({
            "action": "cycles",
            "max_items": 5
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditCyclesResult {
            action: crate::audit_build::AuditAction::Cycles,
            total_cycles: 0,
            findings: Vec::new(),
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                "max_cycle_sccs=50".to_string(),
                "max_nodes_per_cycle=20".to_string(),
            ],
            notes: vec![
                "This native slice reports SCCs from existing resolved graph CALLS facts only."
                    .to_string(),
                "Unresolved call facts are intentionally not guessed into cycle edges.".to_string(),
                "Only SCCs with more than one node or a self-loop are reported.".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_cycles_fails_closed_when_scc_input_edges_are_truncated() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    let mut source = String::new();
    for index in 0..200 {
        source.push_str(&format!(
            "pub fn caller_{index}() -> usize {{\n    callee_{index}()\n}}\n\npub fn callee_{index}() -> usize {{\n    1\n}}\n\n"
        ));
    }
    source.push_str(
        "pub fn cycle_a() -> usize {\n    cycle_b()\n}\n\npub fn cycle_b() -> usize {\n    cycle_a()\n}\n",
    );
    std::fs::write(workspace.join("src/lib.rs"), source).expect("write source");
    index_workspace_for_tests(&workspace);

    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "cycles",
            "max_items": 5
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("cycle input facts are truncated"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_coupling_reports_cross_file_file_pairs() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/a.rs"),
        "pub fn a_one() -> usize {\n    b_one()\n}\n\npub fn a_two() -> usize {\n    b_two()\n}\n",
    )
    .expect("write a source");
    std::fs::write(
        workspace.join("src/b.rs"),
        "pub fn b_one() -> usize {\n    1\n}\n\npub fn b_two() -> usize {\n    2\n}\n\npub fn b_three() -> usize {\n    c_one()\n}\n",
    )
    .expect("write b source");
    std::fs::write(
        workspace.join("src/c.rs"),
        "pub fn c_one() -> usize {\n    1\n}\n",
    )
    .expect("write c source");
    index_workspace_for_tests(&workspace);

    let result = audit_coupling_result(
        &workspace,
        json!({
            "action": "coupling",
            "max_items": 5
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditCouplingResult {
            action: crate::audit_build::AuditAction::Coupling,
            total_pairs: 2,
            findings: vec![
                AuditCouplingFinding {
                    from_path: "src/a.rs".to_string(),
                    to_path: "src/b.rs".to_string(),
                    edge_count: 2,
                },
                AuditCouplingFinding {
                    from_path: "src/b.rs".to_string(),
                    to_path: "src/c.rs".to_string(),
                    edge_count: 1,
                },
            ],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                "max_coupling_rows=100".to_string(),
            ],
            notes: vec![
                "This native slice reports file-pair coupling from existing resolved graph CALLS facts only."
                    .to_string(),
                "Same-file calls and unresolved call facts are intentionally excluded.".to_string(),
                "Rows are sorted by descending edge count, then from_path, then to_path."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_coupling_reports_output_truncation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/a.rs"),
        "pub fn call_b() -> usize {\n    b()\n}\n\npub fn call_c() -> usize {\n    c()\n}\n",
    )
    .expect("write a source");
    std::fs::write(workspace.join("src/b.rs"), "pub fn b() -> usize { 1 }\n")
        .expect("write b source");
    std::fs::write(workspace.join("src/c.rs"), "pub fn c() -> usize { 1 }\n")
        .expect("write c source");
    index_workspace_for_tests(&workspace);

    let result = audit_coupling_result(
        &workspace,
        json!({
            "action": "coupling",
            "max_items": 1
        }),
    )
    .await;

    assert_eq!(result.total_pairs, 2);
    assert_eq!(result.findings.len(), 1);
    assert!(result.truncated);
}

#[tokio::test]
async fn audit_coupling_fails_closed_when_input_edges_are_truncated() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    let mut source = String::new();
    for index in 0..201 {
        source.push_str(&format!(
            "pub fn caller_{index}() -> usize {{\n    callee_{index}()\n}}\n\npub fn callee_{index}() -> usize {{\n    1\n}}\n\n"
        ));
    }
    std::fs::write(workspace.join("src/lib.rs"), source).expect("write source");
    index_workspace_for_tests(&workspace);

    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "coupling",
            "max_items": 5
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("coupling input facts are truncated"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_violations_reports_forbidden_cross_prefix_calls() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src/ui")).expect("create ui");
    std::fs::create_dir_all(workspace.join("src/db")).expect("create db");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/ui/view.rs"),
        "pub fn render() -> usize {\n    query()\n}\n",
    )
    .expect("write ui");
    std::fs::write(
        workspace.join("src/db/store.rs"),
        "pub fn query() -> usize {\n    1\n}\n",
    )
    .expect("write db");
    index_workspace_for_tests(&workspace);

    let result = audit_violations_result(
        &workspace,
        json!({
            "action": "violations",
            "max_items": 5,
            "rules": [{
                "rule_id": "ui-must-not-call-db",
                "from_path_prefix": "src/ui/",
                "forbidden_to_path_prefix": "src/db/",
                "severity": "high"
            }]
        }),
    )
    .await;

    assert_eq!(
        result,
        AuditViolationsResult {
            action: crate::audit_build::AuditAction::Violations,
            findings: vec![AuditViolationFinding {
                rule_id: "ui-must-not-call-db".to_string(),
                severity: Some("high".to_string()),
                from_path: "src/ui/view.rs".to_string(),
                to_path: "src/db/store.rs".to_string(),
                from_symbol: "render".to_string(),
                to_symbol: "query".to_string(),
                relationship_id:
                    "call:src/ui/view.rs:1:render:src/db/store.rs:1:query:src/ui/view.rs:2"
                        .to_string(),
            }],
            skipped_rules: Vec::new(),
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                "max_violation_rows=100".to_string(),
                "max_rules=32".to_string(),
            ],
            notes: vec![
                "This native slice checks caller-supplied path-prefix rules over resolved graph CALLS facts only."
                    .to_string(),
                "Unresolved calls are not guessed into boundary violations.".to_string(),
                "Rules are local request data only; no repo config, DSL, or semantic owner inference is used."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn audit_violations_reports_no_findings_for_allowed_or_unresolved_calls() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src/ui")).expect("create ui");
    std::fs::create_dir_all(workspace.join("src/db")).expect("create db");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/ui/view.rs"),
        "pub fn render() -> usize {\n    missing()\n}\n",
    )
    .expect("write ui");
    std::fs::write(
        workspace.join("src/db/store.rs"),
        "pub fn query() -> usize {\n    1\n}\n",
    )
    .expect("write db");
    index_workspace_for_tests(&workspace);

    let result = audit_violations_result(
        &workspace,
        json!({
            "action": "violations",
            "rules": [{
                "rule_id": "ui-must-not-call-db",
                "from_path_prefix": "src/ui/",
                "forbidden_to_path_prefix": "src/db/"
            }]
        }),
    )
    .await;

    assert_eq!(result.findings, Vec::new());
    assert_eq!(result.skipped_rules, Vec::new());
}

#[tokio::test]
async fn audit_violations_skips_invalid_rules_and_fails_closed_when_too_many_rules() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "pub fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_violations_result(
        &workspace,
        json!({
            "action": "violations",
            "rules": [
                {
                    "rule_id": "",
                    "from_path_prefix": "src/ui/",
                    "forbidden_to_path_prefix": "src/db/"
                },
                {
                    "rule_id": "missing-from",
                    "from_path_prefix": "",
                    "forbidden_to_path_prefix": "src/db/"
                }
            ]
        }),
    )
    .await;

    assert_eq!(
        result.skipped_rules,
        vec![
            AuditViolationSkippedRule {
                rule_id: None,
                reason: "missing rule_id".to_string(),
            },
            AuditViolationSkippedRule {
                rule_id: Some("missing-from".to_string()),
                reason: "missing from_path_prefix".to_string(),
            },
        ]
    );

    let too_many_rules = (0..33)
        .map(|index| {
            json!({
                "rule_id": format!("rule-{index}"),
                "from_path_prefix": "src/a/",
                "forbidden_to_path_prefix": "src/b/"
            })
        })
        .collect::<Vec<_>>();
    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "violations",
            "rules": too_many_rules
        }),
    )
    .await;
    assert!(
        error.to_string().contains("exceed the maximum of 32"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_violations_fails_closed_when_input_edges_are_truncated() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    let mut source = String::new();
    for index in 0..201 {
        source.push_str(&format!(
            "pub fn caller_{index}() -> usize {{\n    callee_{index}()\n}}\n\npub fn callee_{index}() -> usize {{\n    1\n}}\n\n"
        ));
    }
    std::fs::write(workspace.join("src/lib.rs"), source).expect("write source");
    index_workspace_for_tests(&workspace);

    let error = audit_tool_error(
        &workspace,
        json!({
            "action": "violations",
            "rules": [{
                "rule_id": "no-src",
                "from_path_prefix": "src/",
                "forbidden_to_path_prefix": "src/"
            }]
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("violation input facts are truncated"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_coverage_reports_explicit_test_links_without_filename_inference() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn target() -> usize {\n    1\n}\n\nfn untested() -> usize {\n    2\n}\n\n#[test]\nfn target_is_tested() {\n    target();\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_coverage_result(&workspace).await;

    assert_eq!(result.status, AuditCoverageStatus::Pass);
    assert_eq!(result.summary.linked_targets, 1);
    assert_eq!(result.summary.test_links_truncated, false);
    assert_eq!(result.linked_targets[0].test_symbol, "target_is_tested");
    assert_eq!(result.linked_targets[0].target_symbol, "target");
    assert!(
        result
            .unlinked_symbols
            .iter()
            .any(|symbol| symbol.symbol == "untested" && symbol.path == "src/lib.rs")
    );
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("explicit TESTS relationships"))
    );
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("No filename-only inference"))
    );
}

#[tokio::test]
async fn audit_report_reads_active_source_index_and_static_audit_surface() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn caller() -> usize {\n    callee()\n}\n\nfn callee() -> usize {\n    1\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_report_result(&workspace).await;

    assert_eq!(result.action, crate::audit_build::AuditAction::Report);
    assert_eq!(result.source_index.ready, true);
    assert_eq!(
        result.source_index.status,
        AuditReportSourceIndexStatus::Ready
    );
    assert_eq!(result.source_index.files, 1);
    assert_eq!(result.source_index.symbols, 2);
    assert_eq!(result.source_index.relationships, 1);
    assert_eq!(result.source_index.processes, 1);
    assert_eq!(result.graph.files, result.source_index.files);
    assert_eq!(result.graph.symbols, result.source_index.symbols);
    assert_eq!(
        result.graph.relationships,
        result.source_index.relationships
    );
    assert_eq!(result.graph.routes, result.source_index.routes);
    assert_eq!(
        result.graph.unresolved_calls,
        result.source_index.unresolved_calls
    );
    assert_eq!(result.graph.processes, result.source_index.processes);
    assert!(
        result
            .audit_surface
            .implemented_native_audit_actions
            .contains(&"audit.report".to_string())
    );
    assert_eq!(
        result.audit_surface.missing_audit_actions,
        vec![
            "audit.resource_trace".to_string(),
            "audit.taint".to_string(),
        ]
    );
    assert_eq!(result.audit_surface.donor_actions_total, 62);
    assert_eq!(result.audit_surface.donor_actions_implemented, 56);
    assert_eq!(result.audit_surface.donor_actions_missing, 5);
    assert_eq!(
        result.notes,
        vec![
            "audit.report does not build, refresh, or mutate source-index artifacts".to_string(),
            "missing source-index state is reported as readiness only".to_string(),
            "stale or corrupt source-index state fails closed through existing source-index readiness checks".to_string(),
            "the native source index currently analyzes Rust source only".to_string(),
            "call relationships use a conservative lexical unique-name heuristic".to_string(),
            "unresolved or ambiguous calls remain explicit and must not be treated as proven absent dependencies".to_string(),
            "source inspection remains authority for safety-critical conclusions".to_string(),
        ]
    );
}

#[tokio::test]
async fn audit_trace_boundary_dispatches_public_bounded_report() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn boundary() {\n    SCM_RIGHTS;\n}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let result = audit_trace_boundary_result(&workspace).await;

    assert_eq!(result.action, "trace_boundary");
    assert_eq!(result.handoff_facts, 1);
    assert_eq!(result.observed, 1);
    assert_eq!(result.violations, 0);
    assert!(!result.inconclusive);
    assert!(!result.timed_out);
    assert!(!result.findings.is_empty());
    assert!(
        result
            .notes
            .iter()
            .any(|note| note.contains("IndexedResourceFact"))
    );
    assert!(result.limits.iter().any(|limit| limit == "max_items=20"));
}

#[tokio::test]
async fn audit_trace_boundary_fails_closed_without_source_index() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::write(workspace.join("src/lib.rs"), "fn boundary() {}\n").expect("write source");
    let artifact_root = workspace_artifact_root(&workspace);

    let error = audit_tool_error(&workspace, json!({ "action": "trace_boundary" })).await;

    assert!(
        error
            .to_string()
            .contains("active source index is unavailable"),
        "unexpected error: {error}"
    );
    assert!(!artifact_root.current_pointer_path().exists());
}

#[tokio::test]
async fn audit_trace_boundary_rejects_stale_source_index() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/lib.rs"), "fn boundary() {}\n").expect("write source");
    index_workspace_for_tests(&workspace);
    std::fs::write(
        workspace.join("src/lib.rs"),
        "fn boundary() {}\nfn changed() {}\n",
    )
    .expect("modify source");

    let error = audit_tool_error(&workspace, json!({ "action": "trace_boundary" })).await;

    assert!(
        error
            .to_string()
            .contains("source index is stale for the current workspace state"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_report_reports_missing_source_index_without_mutation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::write(workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    let artifact_root = workspace_artifact_root(&workspace);

    let result = audit_report_result(&workspace).await;

    assert_eq!(result.source_index.ready, false);
    assert_eq!(
        result.source_index.status,
        AuditReportSourceIndexStatus::Missing
    );
    assert_eq!(result.source_index.files, 0);
    assert_eq!(result.graph.files, 0);
    assert!(!artifact_root.current_pointer_path().exists());
}

#[tokio::test]
async fn audit_report_fails_closed_for_stale_or_corrupt_source_index() {
    let dir = tempdir().expect("temp dir");
    let stale_workspace = dir.path().join("stale");
    std::fs::create_dir_all(stale_workspace.join("src")).expect("create stale src");
    std::fs::write(stale_workspace.join("src/lib.rs"), "fn indexed() {}\n").expect("write source");
    index_workspace_for_tests(&stale_workspace);
    std::fs::write(
        stale_workspace.join("src/lib.rs"),
        "fn indexed() {}\nfn changed() {}\n",
    )
    .expect("modify source");

    let stale_error = audit_tool_error(&stale_workspace, json!({ "action": "report" })).await;
    assert!(
        stale_error
            .to_string()
            .contains("source index is stale for the current workspace state"),
        "unexpected error: {stale_error}"
    );

    let corrupt_workspace = dir.path().join("corrupt");
    std::fs::create_dir_all(corrupt_workspace.join("src")).expect("create corrupt src");
    std::fs::write(corrupt_workspace.join("src/lib.rs"), "fn indexed() {}\n")
        .expect("write source");
    index_workspace_for_tests(&corrupt_workspace);
    let artifact_root = workspace_artifact_root(&corrupt_workspace);
    let generation_id =
        std::fs::read_to_string(artifact_root.current_pointer_path()).expect("current generation");
    let source_index_path = artifact_root
        .generation_file_path(generation_id.trim(), "source-index.json")
        .expect("source-index path");
    std::fs::write(source_index_path, "{ not json").expect("corrupt source index");

    let corrupt_error = audit_tool_error(&corrupt_workspace, json!({ "action": "report" })).await;
    assert!(
        corrupt_error
            .to_string()
            .contains("failed to load the native source index"),
        "unexpected error: {corrupt_error}"
    );
}
