#[tokio::test]
async fn audit_concurrency_reports_nested_lock_hint() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_concurrency_fixture(
        &workspace,
        "fn run(a: &Mutex<usize>, b: &Mutex<usize>) {\n    let ga = a.lock().unwrap();\n    let gb = b.lock().unwrap();\n    let _ = (*ga, *gb);\n}\n",
    );

    let result = audit_concurrency_result(&workspace, json!({"action": "concurrency"})).await;

    // Two lock acquisitions plus one nested-lock hint on the second acquisition.
    assert_eq!(result.total_candidates, 3);
    let rules = result
        .findings
        .iter()
        .map(|finding| finding.rule.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        rules,
        vec!["lock_acquisition", "lock_acquisition", "nested_lock_hint"]
    );
    let hint = result
        .findings
        .iter()
        .find(|finding| finding.rule == "nested_lock_hint")
        .expect("nested lock hint");
    assert_eq!(hint.line, 3);
}

#[tokio::test]
async fn audit_concurrency_reports_truncation_when_candidate_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_concurrency_fixture(
        &workspace,
        "fn run(a: &Mutex<usize>, b: &Mutex<usize>) {\n    let ga = a.lock().unwrap();\n    let gb = b.lock().unwrap();\n    let _ = (*ga, *gb);\n}\n",
    );

    let result = audit_concurrency_result(
        &workspace,
        json!({
            "action": "concurrency",
            "max_items": 1
        }),
    )
    .await;

    assert_eq!(result.total_candidates, 3);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.truncated, true);
}

#[tokio::test]
async fn audit_simulate_fault_reports_static_structural_evidence() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_simulate_fault_fixture(
        &workspace,
        "fn run() -> Result<(), Error> {\n    let value = maybe_open();\n    if value.is_err() {\n        return Err(Error);\n    }\n    Ok(())\n}\n",
    );

    let result = audit_simulate_fault_result(
        &workspace,
        json!({
            "action": "simulate_fault",
            "path": "src/lib.rs",
            "target_call": "maybe_open",
            "return_value": "Err(Error)",
            "window_lines": 3
        }),
    )
    .await;

    assert_eq!(result.status, AuditFaultSimulationStatus::Evidence);
    assert_eq!(result.scanned_files, 1);
    assert_eq!(result.target_sites, 1);
    assert_eq!(
        result.findings,
        vec![
            AuditFaultSimulationFinding {
                kind: "assignment".to_string(),
                path: "src/lib.rs".to_string(),
                line: 2,
                target_line: 2,
                target_call: "maybe_open".to_string(),
                modeled_return_value: "Err(Error)".to_string(),
                snippet: "let value = maybe_open();".to_string(),
                reason:
                    "Assignment near target call may store or transform the modeled return value."
                        .to_string(),
            },
            AuditFaultSimulationFinding {
                kind: "branch".to_string(),
                path: "src/lib.rs".to_string(),
                line: 3,
                target_line: 2,
                target_call: "maybe_open".to_string(),
                modeled_return_value: "Err(Error)".to_string(),
                snippet: "if value.is_err() {".to_string(),
                reason: "Branch near target call may depend on or handle the modeled return value."
                    .to_string(),
            },
            AuditFaultSimulationFinding {
                kind: "early_return".to_string(),
                path: "src/lib.rs".to_string(),
                line: 4,
                target_line: 2,
                target_call: "maybe_open".to_string(),
                modeled_return_value: "Err(Error)".to_string(),
                snippet: "return Err(Error);".to_string(),
                reason: "Early return near target call may be a fault propagation point."
                    .to_string(),
            },
        ]
    );
    assert_eq!(result.truncated, false);
}

#[tokio::test]
async fn audit_simulate_fault_rejects_invalid_path() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_simulate_fault_fixture(&workspace, "fn run() {}\n");

    let error = audit_simulate_fault_error(
        &workspace,
        json!({
            "action": "simulate_fault",
            "path": "../src/lib.rs",
            "target_call": "run",
            "return_value": "Err"
        }),
    )
    .await;

    assert!(
        error
            .to_string()
            .contains("path must be one repo-relative Rust file without traversal"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn audit_simulate_fault_reports_missing_target_as_inconclusive() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_simulate_fault_fixture(&workspace, "fn run() {\n    present();\n}\n");

    let result = audit_simulate_fault_result(
        &workspace,
        json!({
            "action": "simulate_fault",
            "target_call": "missing",
            "return_value": "None"
        }),
    )
    .await;

    assert_eq!(result.status, AuditFaultSimulationStatus::Inconclusive);
    assert_eq!(result.target_sites, 0);
    assert_eq!(result.findings, Vec::new());
    assert_eq!(
        result.skipped,
        vec![AuditFaultSimulationSkippedRecord {
            category: "inconclusive".to_string(),
            path: None,
            line: None,
            snippet: None,
            reason: "Target call was not found; no static fault window was evaluated.".to_string(),
        }]
    );
}

#[tokio::test]
async fn audit_simulate_fault_reports_truncation_when_candidate_cap_is_hit() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    write_simulate_fault_fixture(
        &workspace,
        "fn run() {\n    let value = maybe_open();\n    if value.is_err() {}\n    let other = value;\n}\n",
    );

    let result = audit_simulate_fault_result(
        &workspace,
        json!({
            "action": "simulate_fault",
            "target_call": "maybe_open",
            "return_value": "Err",
            "max_items": 1
        }),
    )
    .await;

    assert_eq!(result.total_candidates, 3);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.truncated, true);
}

#[tokio::test]
async fn search_semantic_tool_reads_active_source_index_symbols() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn call_alpha() {}\nfn call_beta() {}\nfn other() {}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "semantic",
            "query": "call",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("search should read active source index");

    let result = serde_json::from_value::<SearchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        SearchResult {
            action: SearchAction::Semantic,
            query: "call".to_string(),
            matches: vec![SearchSymbolSummary {
                symbol: "call_alpha".to_string(),
                kind: "fn".to_string(),
                path: "lib.rs".to_string(),
            }],
            files: Vec::new(),
            truncated: true,
            limits: vec![
                "search matches are capped at 200 items".to_string(),
                "search.semantic currently uses native source-index symbol matching only"
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn search_cypher_tool_translates_typed_symbol_query() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn call_alpha() {}\nfn other() {}\n",
    )
    .expect("write source");
    index_workspace_for_tests(&workspace);

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "cypher",
            "query": "MATCH (s:Symbol) WHERE s.name CONTAINS 'call' RETURN s",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("typed cypher query should translate");

    let result = serde_json::from_value::<SearchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        SearchResult {
            action: SearchAction::Cypher,
            query: "MATCH (s:Symbol) WHERE s.name CONTAINS 'call' RETURN s".to_string(),
            matches: vec![SearchSymbolSummary {
                symbol: "call_alpha".to_string(),
                kind: "fn".to_string(),
                path: "lib.rs".to_string(),
            }],
            files: Vec::new(),
            truncated: false,
            limits: vec![
                "search matches are capped at 200 items".to_string(),
                "search.cypher translates only to native source-index symbol matching".to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn search_cypher_tool_translates_typed_file_query() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/alpha.rs"), "fn alpha_entry() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: test_target_head(&workspace),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "cypher",
            "query": "MATCH (f:File) WHERE f.path CONTAINS 'alpha.rs' RETURN f"
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("typed file cypher query should translate");

    let result = serde_json::from_value::<SearchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        SearchResult {
            action: SearchAction::Cypher,
            query: "MATCH (f:File) WHERE f.path CONTAINS 'alpha.rs' RETURN f".to_string(),
            matches: Vec::new(),
            files: vec![SearchRepomapFileSummary {
                path: "src/alpha.rs".to_string(),
                language: "rust".to_string(),
                symbols: vec![SearchSymbolSummary {
                    symbol: "alpha_entry".to_string(),
                    kind: "fn".to_string(),
                    path: "src/alpha.rs".to_string(),
                }],
            }],
            truncated: false,
            limits: vec![
                "repomap files are capped at 200 items".to_string(),
                "repomap symbols per file are capped at 200 items".to_string(),
                "search.cypher translates only to native source-index file and symbol facts"
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn search_cypher_tool_rejects_raw_or_write_query_before_store_access() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "cypher",
            "query": "MATCH (s:Symbol) RETURN s; DELETE s"
        })
        .to_string(),
    };
    let error = match install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload,
        })
        .await
    {
        Ok(_) => panic!("raw or write cypher query should be rejected"),
        Err(error) => error,
    };

    assert!(
        error
            .to_string()
            .contains("raw or write queries are rejected")
    );
}

#[tokio::test]
async fn search_cypher_tool_rejects_unsupported_syntax_before_store_access() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");

    let mut queries = vec![
        "MATCH (s:Symbol) WHERE s.name CONTAINS '' RETURN s".to_string(),
        "MATCH (s:Symbol) WHERE s.name CONTAINS 'call\\'alpha' RETURN s".to_string(),
        "MATCH (s:Symbol) WHERE s.name CONTAINS 'call' RETURN s; MATCH (f:File) RETURN f"
            .to_string(),
        "MATCH (s:Symbol) WHERE s.name CONTAINS 'call' RETURN s.name".to_string(),
        "MATCH (s:Symbol) WHERE s.name CONTAINS 'call\n' RETURN s".to_string(),
    ];
    queries.push(format!(
        "MATCH (s:Symbol) WHERE s.name CONTAINS '{}' RETURN s",
        "x".repeat(8 * 1024)
    ));

    for (index, query) in queries.into_iter().enumerate() {
        let payload = ToolPayload::Function {
            arguments: json!({"action": "cypher", "query": query}).to_string(),
        };
        let error = match install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
            .await
            .handle(ToolCall {
                turn_id: "turn-1".to_string(),
                call_id: format!("call-{index}"),
                tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
                model: "gpt-test".to_string(),
                truncation_policy: TruncationPolicy::Bytes(1024),
                conversation_history: ConversationHistory::default(),
                turn_item_emitter: Arc::new(NoopTurnItemEmitter),
                payload,
            })
            .await
        {
            Ok(_) => panic!("unsupported typed cypher query should be rejected: {query}"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("ontograph.search cypher"));
    }
}

#[tokio::test]
async fn search_semantic_tool_fails_closed_when_source_index_is_missing() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("lib.rs"),
        "fn call_alpha() {}\nfn call_beta() {}\nfn other() {}\n",
    )
    .expect("write source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "semantic",
            "query": "call",
            "max_items": 1
        })
        .to_string(),
    };
    let error = match install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
    {
        Ok(_) => panic!("search should fail closed without an active source index"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("ontograph.manage"));
    assert!(error.to_string().contains("index"));
    assert!(
        !workspace_artifact_root(&workspace)
            .current_pointer_path()
            .exists()
    );
    assert!(!workspace.join(".ontograph/current-generation").exists());
}

#[tokio::test]
async fn search_semantic_tool_fails_closed_when_source_index_is_stale() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("lib.rs"), "fn stale_symbol() {}\n").expect("write source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "stale-head".to_string(),
        target_head: "stale-head".to_string(),
        implementation_version: "stale-version".to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index stale workspace");
    std::fs::write(workspace.join("lib.rs"), "fn fresh_symbol() {}\n").expect("rewrite source");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "semantic",
            "query": "fresh",
            "max_items": 1
        })
        .to_string(),
    };
    let generation_root = workspace_artifact_root(&workspace).generations_root();
    let generations_before = std::fs::read_dir(&generation_root)
        .expect("read generations before")
        .count();
    let error = match install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
    {
        Ok(_) => panic!("search should fail closed for a stale source index"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("ontograph.manage"));
    assert!(error.to_string().contains("index"));
    assert_eq!(
        std::fs::read_dir(generation_root)
            .expect("read generations after")
            .count(),
        generations_before
    );
}

#[tokio::test]
async fn search_repomap_tool_reads_active_source_index_files() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/alpha.rs"),
        "fn alpha_entry() {}\nfn alpha_helper() {}\n",
    )
    .expect("write alpha source");
    std::fs::write(workspace.join("src/beta.rs"), "fn beta_entry() {}\n")
        .expect("write beta source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: test_target_head(&workspace),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "repomap",
            "query": "alpha",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("repomap should read active source index");

    let result = serde_json::from_value::<SearchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        SearchResult {
            action: SearchAction::Repomap,
            query: "alpha".to_string(),
            matches: Vec::new(),
            files: vec![SearchRepomapFileSummary {
                path: "src/alpha.rs".to_string(),
                language: "rust".to_string(),
                symbols: vec![SearchSymbolSummary {
                    symbol: "alpha_entry".to_string(),
                    kind: "fn".to_string(),
                    path: "src/alpha.rs".to_string(),
                }],
            }],
            truncated: false,
            limits: vec![
                "repomap files are capped at 200 items".to_string(),
                "repomap symbols per file are capped at 200 items".to_string(),
                "search.repomap currently uses native source-index file and symbol facts only"
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn search_repomap_tool_keeps_relevant_files_before_truncation() {
    let dir = tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create workspace");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(workspace.join("src/a-target.rs"), "fn helper() {}\n")
        .expect("write path-only source");
    std::fs::write(workspace.join("src/z.rs"), "fn target() {}\n")
        .expect("write exact-match source");
    index_rust_workspace(RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: workspace_artifact_root(&workspace),
        generation_id: "gen-1".to_string(),
        target_head: test_target_head(&workspace),
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("index workspace");

    let payload = ToolPayload::Function {
        arguments: json!({
            "action": "repomap",
            "query": "target",
            "max_items": 1
        })
        .to_string(),
    };
    let output = install_and_find_named_tool(&workspace, SEARCH_TOOL_NAME)
        .await
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, SEARCH_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("repomap should keep the exact symbol match");

    let result = serde_json::from_value::<SearchResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(
        result,
        SearchResult {
            action: SearchAction::Repomap,
            query: "target".to_string(),
            matches: Vec::new(),
            files: vec![SearchRepomapFileSummary {
                path: "src/z.rs".to_string(),
                language: "rust".to_string(),
                symbols: vec![SearchSymbolSummary {
                    symbol: "target".to_string(),
                    kind: "fn".to_string(),
                    path: "src/z.rs".to_string(),
                }],
            }],
            truncated: true,
            limits: vec![
                "repomap files are capped at 200 items".to_string(),
                "repomap symbols per file are capped at 200 items".to_string(),
                "search.repomap currently uses native source-index file and symbol facts only"
                    .to_string(),
            ],
        }
    );
}

#[tokio::test]
async fn km_context_tool_reads_active_imported_km() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_valid_km_for_tool_test(dir.path(), "0.1.0");
    store.import(&source).expect("import KM");
    let tool = KmContextTool::new_with_store(store);
    let payload = ToolPayload::Function {
        arguments: json!({
            "id": "codebase-graph/creating-ontograph",
            "reason": "matched code graph task"
        })
        .to_string(),
    };

    let output = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("KM context should resolve");

    let response = output
        .post_tool_use_response("call-1", &payload)
        .expect("json response");
    let mut response_keys = response
        .as_object()
        .expect("object response")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    response_keys.sort_unstable();
    assert_eq!(
        response_keys,
        vec!["context", "id", "limits", "truncated", "version"]
    );
    let result = serde_json::from_value::<KmContextResult>(response).expect("deserialize result");

    assert_eq!(result.id, "codebase-graph/creating-ontograph");
    assert_eq!(result.version, "0.1.0");
    assert!(result.context.contains("Knowledge Module:"));
    assert!(result.context.contains("matched code graph task"));
    assert!(!result.truncated);
}

#[tokio::test]
async fn km_context_tool_summary_detail_returns_summary_only() {
    let dir = tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_valid_km_for_tool_test(dir.path(), "0.1.0");
    store.import(&source).expect("import KM");
    let tool = KmContextTool::new_with_store(store);
    let payload = ToolPayload::Function {
        arguments: json!({
            "id": "codebase-graph/creating-ontograph",
            "detail": "summary"
        })
        .to_string(),
    };

    let output = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("KM summary should resolve");

    let response = output
        .post_tool_use_response("call-1", &payload)
        .expect("json response");
    let result = serde_json::from_value::<KmContextResult>(response).expect("deserialize result");

    assert_eq!(result.id, "codebase-graph/creating-ontograph");
    assert_eq!(result.version, "0.1.0");
    assert!(result.context.contains("Summary:"));
    assert!(!result.context.contains("Facts:"));
    assert!(!result.context.contains("Reason:"));
    assert!(!result.truncated);
    assert!(result.limits.iter().all(|limit| !limit.contains("facts")));
}

#[tokio::test]
async fn km_context_tool_uses_the_thread_project_store() {
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let nested = dir.path().join("nested/deeper");
    std::fs::create_dir_all(&nested).expect("nested dir");
    let store = KmStore::for_project_path(&nested).expect("project store");
    let source = write_valid_km_for_tool_test(dir.path(), "0.1.0");
    store.import(&source).expect("import KM");
    let thread_state = Arc::new(OntographThreadState::default());
    thread_state.set_current_cwd(nested);
    let tool = KmContextTool::new(thread_state);
    let payload = ToolPayload::Function {
        arguments: json!({ "id": "codebase-graph/creating-ontograph" }).to_string(),
    };

    let output = tool
        .handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: payload.clone(),
        })
        .await
        .expect("KM context should resolve from the project store");

    let result = serde_json::from_value::<KmContextResult>(
        output
            .post_tool_use_response("call-1", &payload)
            .expect("json response"),
    )
    .expect("deserialize result");

    assert_eq!(result.id, "codebase-graph/creating-ontograph");
    assert_eq!(result.version, "0.1.0");
}

#[tokio::test]
async fn km_context_tool_hides_another_threads_agent_authored_record() {
    // Agent-authored records belong to the thread that wrote them, so a second
    // thread must not be able to read one by naming its id.
    let dir = tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let store = KmStore::for_project_path(dir.path()).expect("project store");
    let source = write_valid_km_for_tool_test(dir.path(), "0.1.0");
    store
        .import_with_origin(
            &source,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-author".to_string(),
            },
        )
        .expect("import KM");

    let km_context_call = || ToolCall {
        turn_id: "turn-1".to_string(),
        call_id: "call-1".to_string(),
        tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
        model: "gpt-test".to_string(),
        truncation_policy: TruncationPolicy::Bytes(1024),
        conversation_history: ConversationHistory::default(),
        turn_item_emitter: Arc::new(NoopTurnItemEmitter),
        payload: ToolPayload::Function {
            arguments: json!({ "id": "codebase-graph/creating-ontograph" }).to_string(),
        },
    };

    KmContextTool::new_with_store_for_thread(store.clone(), "thread-author".to_string())
        .handle(km_context_call())
        .await
        .expect("the authoring thread reads its own record");

    let error = expect_tool_error(
        KmContextTool::new_with_store_for_thread(store, "thread-other".to_string())
            .handle(km_context_call())
            .await,
        "another thread must not read this record",
    );
    assert!(format!("{error:?}").contains("has no active KM"));
}
#[tokio::test]
async fn km_context_tool_rejects_non_git_directories() {
    // /dev/shm has no ancestor `.git`, unlike a tempdir under an ambient /tmp/.git.
    let dir = tempfile::TempDir::new_in("/dev/shm").expect("temp dir");
    let thread_state = Arc::new(OntographThreadState::default());
    thread_state.set_current_cwd(dir.path().to_path_buf());
    let tool = KmContextTool::new(thread_state);

    let error = expect_tool_error(
        tool.handle(ToolCall {
            turn_id: "turn-1".to_string(),
            call_id: "call-1".to_string(),
            tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, KM_CONTEXT_TOOL_NAME),
            model: "gpt-test".to_string(),
            truncation_policy: TruncationPolicy::Bytes(1024),
            conversation_history: ConversationHistory::default(),
            turn_item_emitter: Arc::new(NoopTurnItemEmitter),
            payload: ToolPayload::Function {
                arguments: json!({ "id": "codebase-graph/creating-ontograph" }).to_string(),
            },
        })
        .await,
        "non-Git workspace should not resolve a project KM store",
    );

    assert!(
        error
            .to_string()
            .contains("workspace context is unavailable for this turn")
    );
}
