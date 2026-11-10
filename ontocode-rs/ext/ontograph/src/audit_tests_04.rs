#[tokio::test]
async fn explain_module_tool_resolves_repo_root_relative_path_from_nested_workspace_cwd() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    let nested = workspace.join("src/nested");
    std::fs::create_dir_all(&nested).expect("create nested workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/sample.rs"),
        "pub struct Demo;\nfn helper() {}\n",
    )
    .expect("write source");

    let tool = install_and_find_tool(&nested).await;
    let payload = ToolPayload::Function {
        arguments: json!({
            "path": "src/sample.rs",
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
        .expect("tool should resolve repo-root-relative path");

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
            line_count: 2,
            top_level_symbols: vec!["Demo".to_string(), "helper".to_string()],
            limits: vec![
                format!("file reads are capped at {MAX_FILE_BYTES} bytes"),
                "Rust top_level_symbols use syn when the file parses; malformed Rust and non-Rust files use bounded line-based extraction".to_string()
            ],
        }
    );
}

#[tokio::test]
async fn discover_packs_reports_static_native_audit_catalog() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::write(workspace.join("lib.rs"), "fn indexed() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: "abc123".to_string(),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "packs" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover packs should report the static native catalog");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: Vec::new(),
            packs: vec![
                DiscoveredPackSummary {
                    name: "residue_domains".to_string(),
                    description:
                        "Static native pack for bounded forbidden-domain residue scans."
                            .to_string(),
                    actions: vec!["audit.build".to_string()],
                },
                DiscoveredPackSummary {
                    name: "pattern_checks".to_string(),
                    description: "Static native pack for bounded source-pattern scans."
                        .to_string(),
                    actions: vec!["audit.patterns".to_string()],
                },
                DiscoveredPackSummary {
                    name: "saved_finding_reruns".to_string(),
                    description:
                        "Static native pack for rerunning saved audit findings against the current workspace state."
                            .to_string(),
                    actions: vec!["audit.rerun".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_tech_debt".to_string(),
                    description:
                        "Static native pack for bounded Rust structural tech-debt ranking."
                            .to_string(),
                    actions: vec!["audit.tech_debt".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_hotspots".to_string(),
                    description: "Static native pack for bounded Rust file hotspot ranking."
                        .to_string(),
                    actions: vec!["audit.hotspots".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_dead_code".to_string(),
                    description: "Static native pack for bounded Rust private-orphan scans."
                        .to_string(),
                    actions: vec!["audit.dead_code".to_string()],
                },
                DiscoveredPackSummary {
                    name: "graph_cycles".to_string(),
                    description: "Static native pack for bounded graph cycle reports.".to_string(),
                    actions: vec!["audit.cycles".to_string()],
                },
                DiscoveredPackSummary {
                    name: "graph_coupling".to_string(),
                    description: "Static native pack for bounded graph coupling reports."
                        .to_string(),
                    actions: vec!["audit.coupling".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_report".to_string(),
                    description:
                        "Static native pack for bounded source-index and audit-surface reporting."
                            .to_string(),
                    actions: vec!["audit.report".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_violations".to_string(),
                    description:
                        "Static native pack for caller-supplied boundary-rule checks.".to_string(),
                    actions: vec!["audit.violations".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_migration".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar migration reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.migration".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_requirements".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar requirement reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.requirements".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_drift".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar drift aggregation. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.drift".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_graph_diff".to_string(),
                    description:
                        "Static native pack for completed source-index graph generation diffs."
                            .to_string(),
                    actions: vec!["audit.graph_diff".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_logic".to_string(),
                    description:
                        "Static native pack for bounded Rust systems-logic scans.".to_string(),
                    actions: vec!["audit.logic".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_start".to_string(),
                    description:
                        "Static native pack for local audit session lock and readback state."
                            .to_string(),
                    actions: vec!["audit.session_start".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_verify".to_string(),
                    description:
                        "Static native pack for read-only audit session and lock verification."
                            .to_string(),
                    actions: vec!["audit.session_verify".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_dedupe".to_string(),
                    description:
                        "Static native pack for read-only audit-session finding grouping."
                            .to_string(),
                    actions: vec!["audit.session_dedupe".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_bundle".to_string(),
                    description:
                        "Static native pack for projecting grouped audit findings into bundles."
                            .to_string(),
                    actions: vec!["audit.session_bundle".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_verify_diff".to_string(),
                    description:
                        "Static native pack for read-only expected-vs-actual diff verification."
                            .to_string(),
                    actions: vec!["audit.verify_diff".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_worker_scope_review".to_string(),
                    description:
                        "Static native pack for reviewing diff scope against bundle targets."
                            .to_string(),
                    actions: vec!["audit.worker_scope_review".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_test_gap".to_string(),
                    description:
                        "Static native pack for caller-supplied changed-symbol test-gap checks."
                            .to_string(),
                    actions: vec!["audit.test_gap".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_test_suggestions".to_string(),
                    description:
                        "Static native pack for deterministic suggestions from explicit test-gap evidence."
                            .to_string(),
                    actions: vec!["audit.test_suggestions".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_abi".to_string(),
                    description:
                        "Static native pack for normalized JSON schema ABI comparisons."
                            .to_string(),
                    actions: vec!["audit.abi".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_path_verify".to_string(),
                    description:
                        "Static native pack for bounded source-window path verification."
                            .to_string(),
                    actions: vec!["audit.path_verify".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_pressure".to_string(),
                    description:
                        "Static native pack for shallow quota and global side-effect pressure scans."
                            .to_string(),
                    actions: vec!["audit.pressure".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_concurrency".to_string(),
                    description:
                        "Static native pack for shallow lock-acquisition and nested-lock hint scans."
                            .to_string(),
                    actions: vec!["audit.concurrency".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_simulate_fault".to_string(),
                    description:
                        "Static native pack for shallow Rust fault-window simulation scans."
                            .to_string(),
                    actions: vec!["audit.simulate_fault".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_dispatch".to_string(),
                    description:
                        "Static native pack for read-only prompt-packet rendering from one audit bundle."
                            .to_string(),
                    actions: vec!["audit.session_dispatch".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_review_worker".to_string(),
                    description:
                        "Static native pack for reviewing worker changes against one audit bundle."
                            .to_string(),
                    actions: vec!["audit.session_review_worker".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_coverage".to_string(),
                    description:
                        "Static native pack for explicit source-index test-link coverage."
                            .to_string(),
                    actions: vec!["audit.coverage".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_fsm".to_string(),
                    description:
                        "Static native pack for bounded Rust state-machine extraction."
                            .to_string(),
                    actions: vec!["audit.extract_fsm".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_error_topology".to_string(),
                    description:
                        "Static native pack for bounded Rust error-topology scans.".to_string(),
                    actions: vec!["audit.error_topology".to_string()],
                },
            ],
            tools: Vec::new(),
            repos: Vec::new(),
            routes: Vec::new(),
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only static native Ontograph pack descriptors are listed.".to_string(),
                "No donor manifests, suites, execution plans, or model-pack metadata are used."
                    .to_string(),
                "Docs-backed actions require a docs-index.json sidecar, which ontograph.manage action `index` publishes alongside the source index."
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn discover_packs_reports_static_native_audit_catalog_without_artifact() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");

    let payload = ToolPayload::Function {
        arguments: json!({ "action": "packs" }).to_string(),
    };
    let output = install_and_find_named_tool(&workspace, DISCOVER_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, DISCOVER_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(2048),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("discover packs should report the static native catalog without an artifact");

    let result = serde_json::from_value::<DiscoverResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        DiscoverResult {
            namespace: "ontograph".to_string(),
            groups: Vec::new(),
            packs: vec![
                DiscoveredPackSummary {
                    name: "residue_domains".to_string(),
                    description:
                        "Static native pack for bounded forbidden-domain residue scans."
                            .to_string(),
                    actions: vec!["audit.build".to_string()],
                },
                DiscoveredPackSummary {
                    name: "pattern_checks".to_string(),
                    description: "Static native pack for bounded source-pattern scans."
                        .to_string(),
                    actions: vec!["audit.patterns".to_string()],
                },
                DiscoveredPackSummary {
                    name: "saved_finding_reruns".to_string(),
                    description:
                        "Static native pack for rerunning saved audit findings against the current workspace state."
                            .to_string(),
                    actions: vec!["audit.rerun".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_tech_debt".to_string(),
                    description:
                        "Static native pack for bounded Rust structural tech-debt ranking."
                            .to_string(),
                    actions: vec!["audit.tech_debt".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_hotspots".to_string(),
                    description: "Static native pack for bounded Rust file hotspot ranking."
                        .to_string(),
                    actions: vec!["audit.hotspots".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_dead_code".to_string(),
                    description: "Static native pack for bounded Rust private-orphan scans."
                        .to_string(),
                    actions: vec!["audit.dead_code".to_string()],
                },
                DiscoveredPackSummary {
                    name: "graph_cycles".to_string(),
                    description: "Static native pack for bounded graph cycle reports.".to_string(),
                    actions: vec!["audit.cycles".to_string()],
                },
                DiscoveredPackSummary {
                    name: "graph_coupling".to_string(),
                    description: "Static native pack for bounded graph coupling reports."
                        .to_string(),
                    actions: vec!["audit.coupling".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_report".to_string(),
                    description:
                        "Static native pack for bounded source-index and audit-surface reporting."
                            .to_string(),
                    actions: vec!["audit.report".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_violations".to_string(),
                    description:
                        "Static native pack for caller-supplied boundary-rule checks.".to_string(),
                    actions: vec!["audit.violations".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_migration".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar migration reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.migration".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_requirements".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar requirement reference reports. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.requirements".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_drift".to_string(),
                    description:
                        "Static native pack for explicit docs-sidecar drift aggregation. Requires a docs-index.json sidecar, published by ontograph.manage action `index`."
                            .to_string(),
                    actions: vec!["audit.drift".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_graph_diff".to_string(),
                    description:
                        "Static native pack for completed source-index graph generation diffs."
                            .to_string(),
                    actions: vec!["audit.graph_diff".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_logic".to_string(),
                    description:
                        "Static native pack for bounded Rust systems-logic scans.".to_string(),
                    actions: vec!["audit.logic".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_start".to_string(),
                    description:
                        "Static native pack for local audit session lock and readback state."
                            .to_string(),
                    actions: vec!["audit.session_start".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_verify".to_string(),
                    description:
                        "Static native pack for read-only audit session and lock verification."
                            .to_string(),
                    actions: vec!["audit.session_verify".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_dedupe".to_string(),
                    description:
                        "Static native pack for read-only audit-session finding grouping."
                            .to_string(),
                    actions: vec!["audit.session_dedupe".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_bundle".to_string(),
                    description:
                        "Static native pack for projecting grouped audit findings into bundles."
                            .to_string(),
                    actions: vec!["audit.session_bundle".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_verify_diff".to_string(),
                    description:
                        "Static native pack for read-only expected-vs-actual diff verification."
                            .to_string(),
                    actions: vec!["audit.verify_diff".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_worker_scope_review".to_string(),
                    description:
                        "Static native pack for reviewing diff scope against bundle targets."
                            .to_string(),
                    actions: vec!["audit.worker_scope_review".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_test_gap".to_string(),
                    description:
                        "Static native pack for caller-supplied changed-symbol test-gap checks."
                            .to_string(),
                    actions: vec!["audit.test_gap".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_test_suggestions".to_string(),
                    description:
                        "Static native pack for deterministic suggestions from explicit test-gap evidence."
                            .to_string(),
                    actions: vec!["audit.test_suggestions".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_abi".to_string(),
                    description:
                        "Static native pack for normalized JSON schema ABI comparisons."
                            .to_string(),
                    actions: vec!["audit.abi".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_path_verify".to_string(),
                    description:
                        "Static native pack for bounded source-window path verification."
                            .to_string(),
                    actions: vec!["audit.path_verify".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_pressure".to_string(),
                    description:
                        "Static native pack for shallow quota and global side-effect pressure scans."
                            .to_string(),
                    actions: vec!["audit.pressure".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_concurrency".to_string(),
                    description:
                        "Static native pack for shallow lock-acquisition and nested-lock hint scans."
                            .to_string(),
                    actions: vec!["audit.concurrency".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_simulate_fault".to_string(),
                    description:
                        "Static native pack for shallow Rust fault-window simulation scans."
                            .to_string(),
                    actions: vec!["audit.simulate_fault".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_dispatch".to_string(),
                    description:
                        "Static native pack for read-only prompt-packet rendering from one audit bundle."
                            .to_string(),
                    actions: vec!["audit.session_dispatch".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_session_review_worker".to_string(),
                    description:
                        "Static native pack for reviewing worker changes against one audit bundle."
                            .to_string(),
                    actions: vec!["audit.session_review_worker".to_string()],
                },
                DiscoveredPackSummary {
                    name: "audit_coverage".to_string(),
                    description:
                        "Static native pack for explicit source-index test-link coverage."
                            .to_string(),
                    actions: vec!["audit.coverage".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_fsm".to_string(),
                    description:
                        "Static native pack for bounded Rust state-machine extraction."
                            .to_string(),
                    actions: vec!["audit.extract_fsm".to_string()],
                },
                DiscoveredPackSummary {
                    name: "rust_error_topology".to_string(),
                    description:
                        "Static native pack for bounded Rust error-topology scans.".to_string(),
                    actions: vec!["audit.error_topology".to_string()],
                },
            ],
            tools: Vec::new(),
            repos: Vec::new(),
            routes: Vec::new(),
            routes_truncated: false,
            parity_matrix_path: ".memory-bank/knowledge-hub/ONTOGRAPH_DONOR_PARITY_MATRIX.md"
                .to_string(),
            donor_actions_total: 62,
            donor_actions_implemented: 56,
            donor_actions_intentional_divergence: 0,
            donor_actions_missing: 5,
            notes: vec![
                "Only static native Ontograph pack descriptors are listed.".to_string(),
                "No donor manifests, suites, execution plans, or model-pack metadata are used."
                    .to_string(),
                "Docs-backed actions require a docs-index.json sidecar, which ontograph.manage action `index` publishes alongside the source index."
                    .to_string(),
            ],
        }
    );
}
