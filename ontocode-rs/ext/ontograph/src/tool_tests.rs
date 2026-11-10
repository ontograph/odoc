use super::*;

use crate::evidence_planner::EvidenceSupportStatus;
use crate::evidence_planner::PlannerOperationsProvider;
use pretty_assertions::assert_eq;

#[test]
fn planner_provider_expired_request_rejects_all_operations_before_workspace_access() {
    let thread_state = Arc::new(OntographThreadState::default());
    let provider =
        ThreadStateOperationsProvider::with_timeout(&thread_state, /*timeout_ms*/ 0);
    let timeout = Err("Timeout".to_string());

    assert_eq!(provider.search_symbols("anchor", /*limit*/ 1), timeout);
    assert_eq!(provider.docs_context("anchor", /*limit*/ 1), timeout);
    assert_eq!(
        provider.inspect_context("anchor"),
        Err("Timeout".to_string())
    );
    assert_eq!(
        provider.impact_symbol(
            "anchor",
            crate::evidence_planner::TraversalDirection::Upstream,
            /*max_hops*/ 1,
        ),
        timeout
    );
    assert_eq!(provider.procedural_project("anchor", /*hops*/ 1), timeout);
}

#[test]
fn planner_provider_admission_preserves_live_and_default_budget_caps() {
    let thread_state = Arc::new(OntographThreadState::default());
    let unbounded = ThreadStateOperationsProvider::new(&thread_state);
    assert_eq!(unbounded.admit_query(), Ok(MAX_QUERY_TIMEOUT_MS));

    let bounded = ThreadStateOperationsProvider::with_timeout(&thread_state, u64::MAX);
    assert_eq!(bounded.admit_query(), Ok(MAX_QUERY_TIMEOUT_MS));

    let expired = ThreadStateOperationsProvider::with_timeout(&thread_state, /*timeout_ms*/ 0);
    assert_eq!(expired.admit_query(), Err("Timeout".to_string()));
    assert_eq!(expired.query_timeout_ms(), 1);
}

const PLANNER_KM_MANIFEST: &str = "id = \"codebase-graph/planner\"\nversion = \"0.1.0\"\nkind = \"codebase-graph\"\nsummary = \"A bounded KM for planner tests.\"\nsources = [\"silver:doc\"]\nlicenses = [\"MIT\"]\n[provenance]\ncreated_by = \"agent\"\ngenerated_at = \"2026-07-01\"\nsource_commit = \"HEAD\"\n";

const PLANNER_KM_FACTS: &str = "{\"id\":\"fact-1\",\"kind\":\"rule\",\"claim\":\"bounded claim\",\"source\":\"silver:doc\",\"confidence\":0.9}\n";

/// Writes a minimal valid KM package with the supplied procedural transitions.
fn write_planner_km(root: &Path, transitions: &[serde_json::Value]) -> PathBuf {
    let km_root = root.join("planner-km");
    std::fs::create_dir_all(km_root.join("examples")).expect("create examples");
    std::fs::create_dir_all(km_root.join("tests")).expect("create tests");
    std::fs::write(km_root.join("km.toml"), PLANNER_KM_MANIFEST).expect("write manifest");
    std::fs::write(km_root.join("facts.jsonl"), PLANNER_KM_FACTS).expect("write facts");
    std::fs::write(km_root.join("examples/README.md"), "example").expect("write example");
    std::fs::write(km_root.join("tests/README.md"), "test").expect("write test");
    let text = transitions
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(km_root.join("transitions.jsonl"), text).expect("write transitions");
    km_root
}

fn planner_transition(source: &str, target: &str) -> serde_json::Value {
    serde_json::json!({
        "source": source,
        "target": target,
        "relation": "next",
        "condition": "input is complete",
        "guidance": "Proceed to the next stage.",
        "pitfalls": ["Do not skip validation"],
    })
}

/// Builds a workspace whose KM store holds one package with `transitions`.
fn planner_provider_workspace(
    dir: &Path,
    transitions: &[serde_json::Value],
) -> Arc<OntographThreadState> {
    let workspace = dir.join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    // Scoped import validates every declared source against the project scope,
    // so `silver:doc` must resolve to a real file inside the workspace.
    std::fs::write(workspace.join("doc"), "planner source").expect("write km source");
    let source = write_planner_km(dir, transitions);
    let store = KmStore::for_project_path(&workspace).expect("km store");
    store
        .import_for_project(&source, &workspace.display().to_string())
        .expect("import km");
    let thread_state = Arc::new(OntographThreadState::default());
    thread_state.set_current_cwd(workspace);
    thread_state
}

#[test]
fn procedural_project_returns_hop_bounded_km_evidence() {
    let dir = tempfile::tempdir().expect("temp dir");
    let thread_state = planner_provider_workspace(
        dir.path(),
        &[
            planner_transition("review", "verify"),
            planner_transition("verify", "ship"),
        ],
    );
    let provider = ThreadStateOperationsProvider::new(&thread_state);

    let one_hop = provider
        .procedural_project("review", 1)
        .expect("project one hop");
    assert_eq!(one_hop.len(), 1);
    assert_eq!(one_hop[0].metadata.get("hops"), Some(&"1".to_string()));

    // The second hop is reachable only when the caller pays for it, proving the
    // bound is hop depth rather than a flat item multiplier.
    let two_hops = provider
        .procedural_project("review", 2)
        .expect("project two hops");
    assert_eq!(two_hops.len(), 2);
    assert_eq!(two_hops[1].metadata.get("hops"), Some(&"2".to_string()));
}

#[test]
fn procedural_project_carries_km_content_hash_provenance() {
    let dir = tempfile::tempdir().expect("temp dir");
    let thread_state =
        planner_provider_workspace(dir.path(), &[planner_transition("review", "verify")]);
    let provider = ThreadStateOperationsProvider::new(&thread_state);

    let records = provider
        .procedural_project("review", 2)
        .expect("project transitions");

    let record = records.first().expect("one transition record");
    assert_eq!(record.support_status, EvidenceSupportStatus::Supported);
    let hash = record
        .version_or_hash
        .as_ref()
        .expect("supported KM evidence carries an immutable content hash");
    assert!(!hash.trim().is_empty());
    assert_eq!(
        record.metadata.get("km_version"),
        Some(&"0.1.0".to_string())
    );
}

#[test]
fn procedural_project_terminates_on_cyclic_transitions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let thread_state = planner_provider_workspace(
        dir.path(),
        &[
            planner_transition("review", "verify"),
            planner_transition("verify", "review"),
        ],
    );
    let provider = ThreadStateOperationsProvider::new(&thread_state);

    // A cycle must terminate rather than re-expanding the same node forever.
    let records = provider
        .procedural_project("review", 2)
        .expect("project cyclic graph");

    assert!(records.len() <= MAX_PLANNER_PROCEDURAL_RECORDS);
    let ids = records
        .iter()
        .map(|record| record.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), records.len(), "records must be unique");
}

#[test]
fn procedural_project_blocks_tampered_km_cache() {
    let dir = tempfile::tempdir().expect("temp dir");
    let thread_state =
        planner_provider_workspace(dir.path(), &[planner_transition("review", "verify")]);
    let workspace = thread_state.current_repo_root().expect("workspace root");
    let store = KmStore::for_project_path(&workspace).expect("km store");
    let record = store
        .resolve_active("codebase-graph/planner")
        .expect("resolve active")
        .expect("active record");
    let cached_root = store
        .validated_module_root(&record)
        .expect("validated module root");
    // Mutating cached content after import invalidates the recorded hash.
    std::fs::write(cached_root.join("facts.jsonl"), "{\"id\":\"tampered\"}\n")
        .expect("tamper with cache");

    let provider = ThreadStateOperationsProvider::new(&thread_state);
    let records = provider
        .procedural_project("review", 2)
        .expect("project tampered cache");

    let blocked = records
        .iter()
        .find(|record| matches!(record.support_status, EvidenceSupportStatus::Blocked { .. }))
        .expect("tampered cache reports blocked evidence");
    assert_eq!(blocked.version_or_hash, None);
    assert!(
        !records
            .iter()
            .any(|record| record.support_status == EvidenceSupportStatus::Supported),
        "a tampered package must not project supported transitions"
    );
}

#[cfg(unix)]
#[test]
fn read_bounded_utf8_rejects_symlink_target() {
    let dir = tempfile::tempdir().expect("temp dir");
    let target = dir.path().join("target.rs");
    let link = dir.path().join("link.rs");
    std::fs::write(&target, "pub struct Outside;\n").expect("write target");
    std::os::unix::fs::symlink(&target, &link).expect("create symlink");

    let err = read_bounded_utf8(&link).expect_err("symlink open should fail");

    assert!(
        err.to_string()
            .contains("ontograph.explain_module path could not be opened")
    );
}

#[test]
fn donor_parity_constant_matches_live_action_enums() {
    use super::DiscoverAction;
    use super::DocsAction;
    use super::ImpactAction;
    use super::InspectAction;
    use super::ManageAction;
    use super::SearchAction;
    use crate::audit_build::AuditAction;

    const DISCOVER_ACTIONS: [DiscoverAction; 6] = [
        DiscoverAction::Groups,
        DiscoverAction::Packs,
        DiscoverAction::Repos,
        DiscoverAction::Routes,
        DiscoverAction::Sync,
        DiscoverAction::Tools,
    ];
    const DOCS_ACTIONS: [DocsAction; 4] = [
        DocsAction::Context,
        DocsAction::Drift,
        DocsAction::Trace,
        DocsAction::Readiness,
    ];
    const MANAGE_ACTIONS: [ManageAction; 3] = [
        ManageAction::Index,
        ManageAction::RouteMap,
        ManageAction::Session,
    ];
    const INSPECT_ACTIONS: [InspectAction; 3] = [
        InspectAction::Context,
        InspectAction::Evidence,
        InspectAction::Shape,
    ];
    const IMPACT_ACTIONS: [ImpactAction; 4] = [
        ImpactAction::Symbol,
        ImpactAction::Batch,
        ImpactAction::Route,
        ImpactAction::Diff,
    ];
    const SEARCH_ACTIONS: [SearchAction; 3] = [
        SearchAction::Semantic,
        SearchAction::Cypher,
        SearchAction::Repomap,
    ];
    const AUDIT_ACTIONS: [AuditAction; 34] = [
        AuditAction::Build,
        AuditAction::Patterns,
        AuditAction::Rerun,
        AuditAction::TechDebt,
        AuditAction::Hotspots,
        AuditAction::DeadCode,
        AuditAction::Cycles,
        AuditAction::Coupling,
        AuditAction::Coverage,
        AuditAction::Report,
        AuditAction::TraceBoundary,
        AuditAction::Violations,
        AuditAction::Migration,
        AuditAction::Requirements,
        AuditAction::Drift,
        AuditAction::GraphDiff,
        AuditAction::Logic,
        AuditAction::SessionStart,
        AuditAction::SessionVerify,
        AuditAction::SessionDedupe,
        AuditAction::SessionBundle,
        AuditAction::ExtractFsm,
        AuditAction::ErrorTopology,
        AuditAction::WorkerScopeReview,
        AuditAction::SessionReviewWorker,
        AuditAction::VerifyDiff,
        AuditAction::TestGap,
        AuditAction::TestSuggestions,
        AuditAction::Abi,
        AuditAction::PathVerify,
        AuditAction::Pressure,
        AuditAction::Concurrency,
        AuditAction::SimulateFault,
        AuditAction::SessionDispatch,
    ];

    fn check_discover(action: DiscoverAction) {
        match action {
            DiscoverAction::Groups => {}
            DiscoverAction::Packs => {}
            DiscoverAction::Repos => {}
            DiscoverAction::Routes => {}
            DiscoverAction::Sync => {}
            DiscoverAction::Tools => {}
        }
    }

    fn check_docs(action: DocsAction) {
        match action {
            DocsAction::Context => {}
            DocsAction::Drift => {}
            DocsAction::Trace => {}
            DocsAction::Readiness => {}
        }
    }

    fn check_manage(action: ManageAction) {
        match action {
            ManageAction::Index => {}
            ManageAction::RouteMap => {}
            ManageAction::Session => {}
        }
    }

    fn check_inspect(action: InspectAction) {
        match action {
            InspectAction::Context => {}
            InspectAction::Evidence => {}
            InspectAction::Shape => {}
        }
    }

    fn check_impact(action: ImpactAction) {
        match action {
            ImpactAction::Symbol => {}
            ImpactAction::Batch => {}
            ImpactAction::Route => {}
            ImpactAction::Diff => {}
        }
    }

    fn check_search(action: SearchAction) {
        match action {
            SearchAction::Semantic => {}
            SearchAction::Cypher => {}
            SearchAction::Repomap => {}
        }
    }

    fn check_audit(action: AuditAction) {
        match action {
            AuditAction::Build => {}
            AuditAction::Patterns => {}
            AuditAction::Rerun => {}
            AuditAction::TechDebt => {}
            AuditAction::Hotspots => {}
            AuditAction::DeadCode => {}
            AuditAction::Cycles => {}
            AuditAction::Coupling => {}
            AuditAction::Coverage => {}
            AuditAction::Report => {}
            AuditAction::TraceBoundary => {}
            AuditAction::Violations => {}
            AuditAction::Migration => {}
            AuditAction::Requirements => {}
            AuditAction::Drift => {}
            AuditAction::GraphDiff => {}
            AuditAction::Logic => {}
            AuditAction::SessionStart => {}
            AuditAction::SessionVerify => {}
            AuditAction::SessionDedupe => {}
            AuditAction::SessionBundle => {}
            AuditAction::ExtractFsm => {}
            AuditAction::ErrorTopology => {}
            AuditAction::WorkerScopeReview => {}
            AuditAction::SessionReviewWorker => {}
            AuditAction::VerifyDiff => {}
            AuditAction::TestGap => {}
            AuditAction::TestSuggestions => {}
            AuditAction::Abi => {}
            AuditAction::PathVerify => {}
            AuditAction::Pressure => {}
            AuditAction::Concurrency => {}
            AuditAction::SimulateFault => {}
            AuditAction::SessionDispatch => {}
        }
    }

    for action in DISCOVER_ACTIONS {
        check_discover(action);
    }
    for action in DOCS_ACTIONS {
        check_docs(action);
    }
    for action in MANAGE_ACTIONS {
        check_manage(action);
    }
    for action in INSPECT_ACTIONS {
        check_inspect(action);
    }
    for action in IMPACT_ACTIONS {
        check_impact(action);
    }
    for action in SEARCH_ACTIONS {
        check_search(action);
    }
    for action in AUDIT_ACTIONS {
        check_audit(action);
    }

    let donor_actions_sum = DISCOVER_ACTIONS.len()
        + DOCS_ACTIONS.len()
        + MANAGE_ACTIONS.len()
        + INSPECT_ACTIONS.len()
        + IMPACT_ACTIONS.len()
        + SEARCH_ACTIONS.len()
        + AUDIT_ACTIONS.len()
        - 1; // ManageAction::Index is the native writer, not a donor action

    assert_eq!(donor_actions_sum, crate::tool::DONOR_ACTIONS_IMPLEMENTED);
}

fn procedural_context_call(payload: ontocode_extension_api::ToolPayload) -> ToolCall {
    ToolCall {
        turn_id: "turn-1".to_string(),
        call_id: "call-1".to_string(),
        tool_name: ToolName::namespaced(ONTOGRAPH_NAMESPACE, PROCEDURAL_CONTEXT_TOOL_NAME),
        model: "gpt-test".to_string(),
        truncation_policy: ontocode_utils_output_truncation::TruncationPolicy::Bytes(1024),
        conversation_history: ontocode_extension_api::ConversationHistory::default(),
        turn_item_emitter: Arc::new(ontocode_extension_api::NoopTurnItemEmitter),
        payload,
    }
}

fn procedural_transition(source: &str, target: &str, guidance: &str) -> serde_json::Value {
    serde_json::json!({
        "source": source,
        "target": target,
        "relation": "next",
        "condition": "input is complete",
        "guidance": guidance,
        "pitfalls": ["Do not skip validation"],
    })
}

fn write_valid_km_for_procedural_context_test(root: &Path, version: &str) -> std::path::PathBuf {
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

fn write_procedural_km(root: &Path, transitions: &[serde_json::Value]) -> std::path::PathBuf {
    let source = write_valid_km_for_procedural_context_test(root, "0.1.0");
    let text = transitions
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(source.join("transitions.jsonl"), text).expect("write transitions");
    source
}

async fn procedural_context_result(
    store: KmStore,
    arguments: serde_json::Value,
) -> ProceduralContextResult {
    let tool = ProceduralContextTool::new_with_store(store);
    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: arguments.to_string(),
    };
    let output = tool
        .handle(procedural_context_call(payload.clone()))
        .await
        .expect("procedural context should resolve");
    let response = output
        .post_tool_use_response("call-1", &payload)
        .expect("json response");
    serde_json::from_value::<ProceduralContextResult>(response).expect("deserialize result")
}

#[tokio::test]
async fn procedural_context_tool_returns_bounded_localized_transitions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_procedural_km(
        dir.path(),
        &[
            procedural_transition("inspect", "validate", "Validate input"),
            procedural_transition("validate", "report", "Report result"),
            procedural_transition("report", "archive", "Archive review"),
        ],
    );
    store.import(&source).expect("import KM");

    let result = procedural_context_result(
        store,
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate"
        }),
    )
    .await;

    assert_eq!(result.active_node.as_deref(), Some("validate"));
    assert_eq!(result.fallback_reason, None);
    assert_eq!(
        result
            .transitions
            .iter()
            .map(|transition| (
                transition.source.clone(),
                transition.target.clone(),
                transition.hops
            ))
            .collect::<Vec<_>>(),
        vec![
            ("validate".to_string(), "report".to_string(), 1),
            ("report".to_string(), "archive".to_string(), 2)
        ]
    );
    assert!(!result.provenance.source_commit.is_empty());
}

#[tokio::test]
async fn procedural_context_tool_reports_unknown_node_fallback() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_procedural_km(
        dir.path(),
        &[procedural_transition(
            "inspect",
            "validate",
            "Validate input",
        )],
    );
    store.import(&source).expect("import KM");

    let result = procedural_context_result(
        store,
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "missing-node"
        }),
    )
    .await;

    assert_eq!(result.active_node, None);
    assert!(result.transitions.is_empty());
    assert_eq!(
        result.fallback_reason.as_deref(),
        Some("the requested node is not a declared procedure in this Knowledge Module")
    );
}

#[tokio::test]
async fn procedural_context_tool_reports_empty_graph_fallback() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_valid_km_for_procedural_context_test(dir.path(), "0.1.0");
    store.import(&source).expect("import KM");

    let result = procedural_context_result(
        store,
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate"
        }),
    )
    .await;

    assert_eq!(result.active_node, None);
    assert!(result.transitions.is_empty());
    assert_eq!(
        result.fallback_reason.as_deref(),
        Some("the active Knowledge Module declares no transitions")
    );
}

#[tokio::test]
async fn procedural_context_tool_fails_closed_without_active_km() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let tool = ProceduralContextTool::new_with_store(store);
    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: serde_json::json!({
            "id": "codebase-graph/missing",
            "node": "inspect"
        })
        .to_string(),
    };

    let message = match tool.handle(procedural_context_call(payload)).await {
        Ok(_) => panic!("an unavailable active KM must fail closed"),
        Err(err) => format!("{err:?}"),
    };
    assert!(
        message.contains("has no active KM for id `codebase-graph/missing`"),
        "{message}"
    );
}

#[tokio::test]
async fn procedural_context_tool_rejects_empty_node() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let tool = ProceduralContextTool::new_with_store(store);
    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "   "
        })
        .to_string(),
    };

    let result = tool.handle(procedural_context_call(payload)).await;

    let message = match result {
        Ok(_) => panic!("empty node must fail closed"),
        Err(err) => err.to_string(),
    };
    assert!(message.contains("node must not be empty"), "{message}");
}

#[tokio::test]
async fn procedural_context_tool_rejects_empty_id() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let tool = ProceduralContextTool::new_with_store(store);
    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: serde_json::json!({
            "id": "   ",
            "node": "validate"
        })
        .to_string(),
    };

    let result = tool.handle(procedural_context_call(payload)).await;

    let message = match result {
        Ok(_) => panic!("empty id must fail closed"),
        Err(err) => err.to_string(),
    };
    assert!(message.contains("id must not be empty"), "{message}");
}

#[tokio::test]
async fn procedural_context_tool_fails_closed_on_malformed_transitions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_procedural_km(
        dir.path(),
        &[procedural_transition(
            "inspect",
            "validate",
            "Validate input",
        )],
    );
    store.import(&source).expect("import KM");
    let activated = store
        .resolve_active("codebase-graph/creating-ontograph")
        .expect("resolve active")
        .expect("active record");
    let root = store
        .validated_module_root(&activated)
        .expect("validated module root");
    std::fs::write(root.join("transitions.jsonl"), "{ not valid json\n")
        .expect("write malformed transitions");

    let tool = ProceduralContextTool::new_with_store(store);
    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate"
        })
        .to_string(),
    };

    let message = match tool.handle(procedural_context_call(payload)).await {
        Ok(_) => panic!("malformed transitions must fail closed"),
        Err(err) => format!("{err:?}"),
    };
    assert!(message.contains("failed to read the KM cache"), "{message}");
}

#[tokio::test]
async fn procedural_context_tool_hides_another_threads_agent_authored_record() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join(".git")).expect("git dir");
    let store = KmStore::for_project_path(dir.path()).expect("project store");
    let source = write_procedural_km(
        dir.path(),
        &[procedural_transition(
            "inspect",
            "validate",
            "Validate input",
        )],
    );
    store
        .import_with_origin(
            &source,
            KmIndexOrigin::AgentThread {
                thread_id: "thread-author".to_string(),
            },
        )
        .expect("import KM");

    let call = || {
        procedural_context_call(ontocode_extension_api::ToolPayload::Function {
            arguments: serde_json::json!({
                "id": "codebase-graph/creating-ontograph",
                "node": "inspect"
            })
            .to_string(),
        })
    };

    ProceduralContextTool::new_with_store_for_thread(store.clone(), "thread-author".to_string())
        .handle(call())
        .await
        .expect("the authoring thread reads its own record");

    let message =
        match ProceduralContextTool::new_with_store_for_thread(store, "thread-other".to_string())
            .handle(call())
            .await
        {
            Ok(_) => panic!("another thread must not read this record"),
            Err(err) => format!("{err:?}"),
        };
    assert!(message.contains("has no active KM"), "{message}");
}

#[tokio::test]
async fn procedural_context_tool_honors_requested_hop_limit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    let source = write_procedural_km(
        dir.path(),
        &[
            procedural_transition("inspect", "validate", "Validate input"),
            procedural_transition("validate", "report", "Report result"),
            procedural_transition("report", "archive", "Archive review"),
        ],
    );
    store.import(&source).expect("import KM");

    let single_hop = procedural_context_result(
        store.clone(),
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate",
            "max_hops": 1
        }),
    )
    .await;
    assert_eq!(
        single_hop
            .transitions
            .iter()
            .map(|transition| transition.hops)
            .collect::<Vec<_>>(),
        vec![1]
    );

    let zero_hops = procedural_context_result(
        store.clone(),
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate",
            "max_hops": 0
        }),
    )
    .await;
    assert_eq!(zero_hops.active_node.as_deref(), Some("validate"));
    assert!(zero_hops.transitions.is_empty());
    assert_eq!(zero_hops.fallback_reason, None);

    let over_cap = procedural_context_result(
        store,
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate",
            "max_hops": 99
        }),
    )
    .await;
    assert_eq!(
        over_cap
            .transitions
            .iter()
            .map(|transition| transition.hops)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(
        over_cap
            .transitions
            .iter()
            .all(|transition| transition.hops <= MAX_PROCEDURAL_CONTEXT_HOPS)
    );
}

#[tokio::test]
async fn procedural_context_tool_caps_returned_transitions() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = KmStore::new(dir.path().join("km-cache"));
    // Every edge in this fan-out is one hop from the active node, so only the
    // response cap can bound the projection.
    let transitions = (0..MAX_PROCEDURAL_CONTEXT_TRANSITIONS + 8)
        .map(|index| {
            procedural_transition("validate", &format!("report-{index:03}"), "Report result")
        })
        .collect::<Vec<_>>();
    let source = write_procedural_km(dir.path(), &transitions);
    store.import(&source).expect("import KM");

    let result = procedural_context_result(
        store,
        serde_json::json!({
            "id": "codebase-graph/creating-ontograph",
            "node": "validate"
        }),
    )
    .await;

    assert_eq!(result.active_node.as_deref(), Some("validate"));
    assert_eq!(result.transitions.len(), MAX_PROCEDURAL_CONTEXT_TRANSITIONS);
    assert!(
        result.limits.contains(&format!(
            "transitions are capped at {MAX_PROCEDURAL_CONTEXT_TRANSITIONS}"
        )),
        "{:?}",
        result.limits
    );
}

/// PG-R4 withholds host registration: publishing this verb on the embedded host
/// changes an asserted host tool contract and needs a separate accepted ADR.
#[test]
fn procedural_context_tool_is_not_registered_on_the_embedded_host() {
    use ontocode_extension_api::ExtensionData;
    use ontocode_extension_api::ToolContributor;

    let procedural_context =
        ToolName::namespaced(ONTOGRAPH_NAMESPACE, PROCEDURAL_CONTEXT_TOOL_NAME);
    let registered = crate::extension::SurfaceScopedOntographExtension::default()
        .tools(
            &ExtensionData::new("session"),
            &ExtensionData::new("thread"),
        )
        .iter()
        .map(|tool| tool.tool_name())
        .collect::<Vec<_>>();

    assert!(!registered.contains(&procedural_context), "{registered:?}");
    // The sibling KM verb is registered, so the assertion above proves the
    // surface was populated rather than empty.
    assert!(
        registered.contains(&ToolName::namespaced(
            ONTOGRAPH_NAMESPACE,
            KM_CONTEXT_TOOL_NAME
        )),
        "{registered:?}"
    );
}
