// Except where a fixture names the real provider, every fixture in this module
// drives the planner through deterministic mock providers. Mock-backed
// assertions prove planner mechanism and boundary behavior only; they are not
// real-provider retrieval quality, coverage, or performance claims. The
// real-provider fixture builds a bounded local corpus and reports its own
// evidence source.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use pretty_assertions::assert_eq;

use super::*;

#[test]
fn retrieval_shapes_serialize_and_deserialize() {
    let shapes = vec![
        (RetrievalShape::DirectLookup, "\"direct_lookup\""),
        (RetrievalShape::Neighborhood, "\"neighborhood\""),
        (RetrievalShape::DependencyPath, "\"dependency_path\""),
        (RetrievalShape::Provenance, "\"provenance\""),
        (RetrievalShape::CrossArtifactJoin, "\"cross_artifact_join\""),
        (RetrievalShape::TemporalVersion, "\"temporal_version\""),
    ];

    for (shape, expected_json) in shapes {
        let serialized = serde_json::to_string(&shape).expect("serialize shape");
        assert_eq!(serialized, expected_json);
        let deserialized: RetrievalShape =
            serde_json::from_str(&serialized).expect("deserialize shape");
        assert_eq!(deserialized, shape);
    }
}

#[test]
fn traversal_directions_serialize_and_deserialize() {
    let directions = vec![
        (TraversalDirection::Upstream, "\"upstream\""),
        (TraversalDirection::Downstream, "\"downstream\""),
        (TraversalDirection::Lateral, "\"lateral\""),
        (TraversalDirection::Bidirectional, "\"bidirectional\""),
    ];

    for (direction, expected_json) in directions {
        let serialized = serde_json::to_string(&direction).expect("serialize direction");
        assert_eq!(serialized, expected_json);
        let deserialized: TraversalDirection =
            serde_json::from_str(&serialized).expect("deserialize direction");
        assert_eq!(deserialized, direction);
    }
}

#[test]
fn evidence_support_statuses() {
    let supported = EvidenceSupportStatus::Supported;
    assert_eq!(
        serde_json::to_string(&supported).expect("serialize supported"),
        "\"supported\""
    );

    let blocked = EvidenceSupportStatus::Blocked {
        reason: "tampered artifact hash".to_string(),
    };
    let blocked_json = serde_json::to_string(&blocked).expect("serialize blocked");
    assert!(blocked_json.contains("blocked"));
    assert!(blocked_json.contains("tampered artifact hash"));

    let unresolved = EvidenceSupportStatus::Unresolved {
        reason: "ambiguous symbol anchor".to_string(),
    };
    let unresolved_json = serde_json::to_string(&unresolved).expect("serialize unresolved");
    assert!(unresolved_json.contains("unresolved"));
    assert!(unresolved_json.contains("ambiguous symbol anchor"));
}

#[test]
fn evidence_record_validation_requires_identity() {
    let record = EvidenceRecord {
        id: "".to_string(),
        source_location: Some(SourceLocation {
            path: "src/lib.rs".to_string(),
            start_line: Some(1),
            end_line: Some(10),
        }),
        project_scope: None,
        version_or_hash: Some("sha256-abc".to_string()),
        relation: Some("calls".to_string()),
        retrieval_reason: "lookup".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert_eq!(record.validate(), Err(PlannerModelError::MissingIdentity));

    let whitespace_id = EvidenceRecord {
        id: "   ".to_string(),
        ..record
    };
    assert_eq!(
        whitespace_id.validate(),
        Err(PlannerModelError::MissingIdentity)
    );
}

#[test]
fn supported_evidence_record_requires_provenance() {
    let record = EvidenceRecord {
        id: "Symbol:test_fn".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("defines".to_string()),
        retrieval_reason: "direct match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert!(matches!(
        record.validate(),
        Err(PlannerModelError::MissingProvenance(_))
    ));
}

#[test]
fn supported_evidence_record_with_location_or_hash_passes() {
    let record_loc = EvidenceRecord {
        id: "Symbol:test_fn".to_string(),
        source_location: Some(SourceLocation {
            path: "src/lib.rs".to_string(),
            start_line: Some(5),
            end_line: Some(12),
        }),
        project_scope: Some("ontocode".to_string()),
        version_or_hash: None,
        relation: Some("defines".to_string()),
        retrieval_reason: "direct match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert!(record_loc.validate().is_ok());

    let record_hash = EvidenceRecord {
        id: "KmFact:123".to_string(),
        source_location: None,
        project_scope: Some("ontocode".to_string()),
        version_or_hash: Some("gen-42-hash-xyz".to_string()),
        relation: Some("km_fact".to_string()),
        retrieval_reason: "provenance lookup".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert!(record_hash.validate().is_ok());
}

#[test]
fn source_location_invalid_line_range_fails_closed() {
    let loc = SourceLocation {
        path: "src/main.rs".to_string(),
        start_line: Some(20),
        end_line: Some(10),
    };
    assert_eq!(
        loc.validate(),
        Err(PlannerModelError::InvalidLineRange { start: 20, end: 10 })
    );
}

#[test]
fn retrieval_plan_validation_rejects_empty_anchor() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "   ".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["imports".to_string()],
        project_scope: None,
        budget: PlannerBudget::default(),
    };
    assert!(matches!(
        plan.validate(),
        Err(PlannerModelError::InvalidAnchor(_))
    ));
}

#[test]
fn planner_budget_bounds_enforced() {
    let zero_hops = PlannerBudget {
        max_hops: 0,
        ..Default::default()
    };
    assert!(zero_hops.validate().is_err());

    let excessive_hops = PlannerBudget {
        max_hops: MAX_HOPS_LIMIT + 1,
        ..Default::default()
    };
    assert!(excessive_hops.validate().is_err());

    let zero_records = PlannerBudget {
        max_records: 0,
        ..Default::default()
    };
    assert!(zero_records.validate().is_err());

    let valid_budget = PlannerBudget::default();
    assert!(valid_budget.validate().is_ok());
}

#[test]
fn evidence_subgraph_enforces_max_records_and_deduplicates() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Upstream,
        relation_types: vec!["callers".to_string()],
        project_scope: Some("proj-a".to_string()),
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 2,
            max_artifacts: 5,
            max_context_bytes: 10_000,
            timeout_ms: 5000,
        },
    };

    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");

    let r1 = EvidenceRecord {
        id: "rec-1".to_string(),
        source_location: Some(SourceLocation {
            path: "a.rs".to_string(),
            start_line: Some(1),
            end_line: Some(5),
        }),
        project_scope: Some("proj-a".to_string()),
        version_or_hash: Some("h1".to_string()),
        relation: Some("calls".to_string()),
        retrieval_reason: "caller".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert!(subgraph.add_record(r1.clone()).is_ok());
    // Deduplication: re-adding same ID succeeds without incrementing count
    assert!(subgraph.add_record(r1).is_ok());
    assert_eq!(subgraph.records.len(), 1);

    let r2 = EvidenceRecord {
        id: "rec-2".to_string(),
        source_location: Some(SourceLocation {
            path: "b.rs".to_string(),
            start_line: Some(1),
            end_line: Some(5),
        }),
        project_scope: Some("proj-a".to_string()),
        version_or_hash: Some("h2".to_string()),
        relation: Some("calls".to_string()),
        retrieval_reason: "caller".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    assert!(subgraph.add_record(r2).is_ok());
    assert_eq!(subgraph.records.len(), 2);

    // Third record exceeds max_records (2)
    let r3 = EvidenceRecord {
        id: "rec-3".to_string(),
        source_location: Some(SourceLocation {
            path: "c.rs".to_string(),
            start_line: Some(1),
            end_line: Some(5),
        }),
        project_scope: Some("proj-a".to_string()),
        version_or_hash: Some("h3".to_string()),
        relation: Some("calls".to_string()),
        retrieval_reason: "caller".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    let res = subgraph.add_record(r3);
    assert!(matches!(res, Err(PlannerModelError::BudgetExceeded(_))));
    assert!(subgraph.truncated);
}

#[test]
fn evidence_subgraph_enforces_scope_isolation() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "Symbol:alpha".to_string(),
        direction: TraversalDirection::Lateral,
        relation_types: vec![],
        project_scope: Some("scope-expected".to_string()),
        budget: PlannerBudget::default(),
    };

    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");

    let wrong_scope = EvidenceRecord {
        id: "rec-wrong".to_string(),
        source_location: Some(SourceLocation {
            path: "wrong.rs".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: Some("scope-forbidden".to_string()),
        version_or_hash: Some("h1".to_string()),
        relation: None,
        retrieval_reason: "lookup".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };
    let res = subgraph.add_record(wrong_scope);
    assert!(matches!(res, Err(PlannerModelError::ScopeMismatch { .. })));
}

#[test]
fn blocked_records_fail_closed_with_explicit_reason_without_source_location() {
    let blocked_record = EvidenceRecord {
        id: "Artifact:stale_doc".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("doc_ref".to_string()),
        retrieval_reason: "docs check".to_string(),
        support_status: EvidenceSupportStatus::Blocked {
            reason: "SourceIdentityMismatch: content hash diverged from index".to_string(),
        },
        metadata: BTreeMap::new(),
    };
    // Blocked record validation succeeds because support_status is Blocked (it explicitly preserves failure reason)
    assert!(blocked_record.validate().is_ok());

    let plan = RetrievalPlan {
        shape: RetrievalShape::Provenance,
        anchor: "Artifact:stale_doc".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };
    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");
    assert!(subgraph.add_record(blocked_record).is_ok());
    assert_eq!(subgraph.records.len(), 1);
}

#[derive(Clone)]
struct MockOperationsProvider {
    call_order: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    operation_delay: Option<std::time::Duration>,
    symbols_result: Result<Vec<EvidenceRecord>, String>,
    docs_result: Result<Vec<EvidenceRecord>, String>,
    inspect_result: Result<Option<EvidenceRecord>, String>,
    impact_result: Result<Vec<EvidenceRecord>, String>,
    procedural_result: Result<Vec<EvidenceRecord>, String>,
}

impl MockOperationsProvider {
    fn new() -> Self {
        Self {
            call_order: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            operation_delay: None,
            symbols_result: Ok(Vec::new()),
            docs_result: Ok(Vec::new()),
            inspect_result: Ok(None),
            impact_result: Ok(Vec::new()),
            procedural_result: Ok(Vec::new()),
        }
    }

    fn record_call(&self, name: &str) {
        self.call_order.lock().unwrap().push(name.to_string());
        if let Some(delay) = self.operation_delay {
            std::thread::sleep(delay);
        }
    }
}

impl PlannerOperationsProvider for MockOperationsProvider {
    fn search_symbols(&self, _query: &str, _limit: usize) -> Result<Vec<EvidenceRecord>, String> {
        self.record_call("search_symbols");
        self.symbols_result.clone()
    }

    fn docs_context(&self, _query: &str, _limit: usize) -> Result<Vec<EvidenceRecord>, String> {
        self.record_call("docs_context");
        self.docs_result.clone()
    }

    fn inspect_context(&self, _symbol: &str) -> Result<Option<EvidenceRecord>, String> {
        self.record_call("inspect_context");
        self.inspect_result.clone()
    }

    fn impact_symbol(
        &self,
        _symbol: &str,
        _direction: TraversalDirection,
        _max_hops: usize,
    ) -> Result<Vec<EvidenceRecord>, String> {
        self.record_call("impact_symbol");
        self.impact_result.clone()
    }

    fn procedural_project(
        &self,
        _anchor: &str,
        _hops: usize,
    ) -> Result<Vec<EvidenceRecord>, String> {
        self.record_call("procedural_project");
        self.procedural_result.clone()
    }
}

fn traversal_record(id: &str, relation: Option<&str>) -> EvidenceRecord {
    EvidenceRecord {
        id: id.to_string(),
        source_location: Some(SourceLocation {
            path: format!("src/{id}.rs"),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: Some(format!("hash-{id}")),
        relation: relation.map(str::to_string),
        retrieval_reason: "bounded traversal fixture".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::from([(String::from("hops"), String::from("1"))]),
    }
}

#[derive(Clone)]
struct CapacityTrackingProvider {
    inner: MockOperationsProvider,
    impact_max_records: std::sync::Arc<std::sync::Mutex<Option<usize>>>,
    procedural_max_records: std::sync::Arc<std::sync::Mutex<Option<usize>>>,
}

impl CapacityTrackingProvider {
    fn new(inner: MockOperationsProvider) -> Self {
        Self {
            inner,
            impact_max_records: std::sync::Arc::new(std::sync::Mutex::new(None)),
            procedural_max_records: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl PlannerOperationsProvider for CapacityTrackingProvider {
    fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<EvidenceRecord>, String> {
        self.inner.search_symbols(query, limit)
    }

    fn docs_context(&self, query: &str, limit: usize) -> Result<Vec<EvidenceRecord>, String> {
        self.inner.docs_context(query, limit)
    }

    fn inspect_context(&self, symbol: &str) -> Result<Option<EvidenceRecord>, String> {
        self.inner.inspect_context(symbol)
    }

    fn impact_symbol(
        &self,
        symbol: &str,
        direction: TraversalDirection,
        max_hops: usize,
    ) -> Result<Vec<EvidenceRecord>, String> {
        self.inner.impact_symbol(symbol, direction, max_hops)
    }

    fn procedural_project(&self, anchor: &str, hops: usize) -> Result<Vec<EvidenceRecord>, String> {
        self.inner.procedural_project(anchor, hops)
    }

    fn impact_symbol_bounded(
        &self,
        symbol: &str,
        direction: TraversalDirection,
        max_hops: usize,
        max_records: usize,
    ) -> Result<TraversalResult, String> {
        *self.impact_max_records.lock().unwrap() = Some(max_records);
        Ok(TraversalResult {
            records: self.inner.impact_symbol(symbol, direction, max_hops)?,
            truncated: false,
        })
    }

    fn procedural_project_bounded(
        &self,
        anchor: &str,
        hops: usize,
        max_records: usize,
    ) -> Result<TraversalResult, String> {
        *self.procedural_max_records.lock().unwrap() = Some(max_records);
        Ok(TraversalResult {
            records: self.inner.procedural_project(anchor, hops)?,
            truncated: false,
        })
    }
}

#[test]
fn engine_passes_remaining_record_capacity_to_bounded_overrides() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![
        traversal_record("Symbol:seed-a", None),
        traversal_record("Symbol:seed-b", None),
    ]);
    mock.impact_result = Ok(vec![traversal_record("Impact:kept", Some("calls"))]);
    mock.procedural_result = Ok(vec![traversal_record("Proc:kept", Some("uses"))]);

    let capacity = CapacityTrackingProvider::new(mock);
    let engine = PlannerEngine::new(&capacity);

    let neighborhood = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::Neighborhood,
            anchor: "Symbol:seed-a".to_string(),
            direction: TraversalDirection::Downstream,
            relation_types: vec![],
            project_scope: None,
            budget: PlannerBudget {
                max_records: 5,
                ..PlannerBudget::default()
            },
        })
        .expect("neighborhood execute");
    assert!(neighborhood.records.len() >= 2);
    assert_eq!(
        *capacity.impact_max_records.lock().unwrap(),
        Some(3),
        "impact must receive remaining capacity after textual seeds"
    );

    let join = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::CrossArtifactJoin,
            anchor: "Symbol:seed-a".to_string(),
            direction: TraversalDirection::Bidirectional,
            relation_types: vec![],
            project_scope: None,
            budget: PlannerBudget {
                max_records: 4,
                ..PlannerBudget::default()
            },
        })
        .expect("cross-artifact execute");
    assert!(join.records.len() >= 2);
    assert_eq!(
        *capacity.procedural_max_records.lock().unwrap(),
        Some(2),
        "procedural projection must receive remaining capacity after textual seeds"
    );
}

#[test]
fn default_bounded_traversal_methods_handle_zero_exact_and_overflow() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![
        traversal_record("Impact:one", Some("calls")),
        traversal_record("Impact:two", Some("calls")),
    ]);
    mock.procedural_result = Ok(vec![
        traversal_record("Proc:one", Some("uses")),
        traversal_record("Proc:two", Some("uses")),
        traversal_record("Proc:three", Some("uses")),
    ]);

    let zero = mock
        .impact_symbol_bounded("anchor", TraversalDirection::Downstream, 2, 0)
        .expect("zero-capacity impact");
    assert!(zero.truncated);
    assert!(zero.records.is_empty());

    let exact = mock
        .impact_symbol_bounded("anchor", TraversalDirection::Downstream, 2, 2)
        .expect("exact-capacity impact");
    assert!(!exact.truncated);
    assert_eq!(exact.records.len(), 2);

    let overflow = mock
        .procedural_project_bounded("anchor", 2, 1)
        .expect("overflow procedural");
    assert!(overflow.truncated);
    assert_eq!(overflow.records.len(), 1);
    assert_eq!(overflow.records[0].id, "Proc:one");
}

#[test]
fn bounded_truncation_survives_when_relation_filter_drops_all_records() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![
        traversal_record("Impact:imports-a", Some("imports")),
        traversal_record("Impact:imports-b", Some("imports")),
        traversal_record("Impact:imports-c", Some("imports")),
    ]);

    let engine = PlannerEngine::new(&mock);
    let subgraph = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::Neighborhood,
            anchor: "Symbol:root".to_string(),
            direction: TraversalDirection::Downstream,
            relation_types: vec!["calls".to_string()],
            project_scope: None,
            budget: PlannerBudget {
                max_records: 2,
                ..PlannerBudget::default()
            },
        })
        .expect("filtered truncated traversal");

    assert!(
        subgraph.truncated,
        "provider overflow must remain visible after every returned relation is filtered out"
    );
    assert!(
        subgraph.records.iter().any(|record| {
            record.id == "Blocked:RelationFilter:Symbol:root"
                && matches!(
                    &record.support_status,
                    EvidenceSupportStatus::Blocked { .. }
                )
        }),
        "relation-filter emptiness must still be reported as blocked evidence"
    );
}

#[test]
fn textual_symbol_and_docs_search_execute_before_graph_traversal() {
    let mock = MockOperationsProvider::new();
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "MyStruct".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let result = engine.execute(plan);
    assert!(result.is_ok());

    let calls = mock.call_order.lock().unwrap().clone();
    assert!(calls.len() >= 4);
    assert_eq!(calls[0], "search_symbols");
    assert_eq!(calls[1], "docs_context");
    assert_eq!(calls[2], "inspect_context");
    assert_eq!(calls[3], "impact_symbol");
}

#[test]
fn direct_lookup_stops_adaptively_without_graph_traversal() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "MyStruct".to_string(),
        source_location: Some(SourceLocation {
            path: "src/lib.rs".to_string(),
            start_line: Some(1),
            end_line: Some(10),
        }),
        project_scope: None,
        version_or_hash: Some("hash1".to_string()),
        relation: None,
        retrieval_reason: "exact match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "MyStruct".to_string(),
        direction: TraversalDirection::Upstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute direct lookup");
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(subgraph.records[0].id, "MyStruct");

    let calls = mock.call_order.lock().unwrap().clone();
    assert_eq!(calls, vec!["search_symbols", "docs_context"]);
}

#[test]
fn direct_lookup_unresolved_anchor_recorded_explicitly() {
    let mock = MockOperationsProvider::new();
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "NonExistentSymbol".to_string(),
        direction: TraversalDirection::Upstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute direct lookup");
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(subgraph.records[0].id, "Unresolved:NonExistentSymbol");
    assert!(matches!(
        subgraph.records[0].support_status,
        EvidenceSupportStatus::Unresolved { .. }
    ));
}

#[test]
fn stale_or_failed_operation_recorded_as_blocked_evidence() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result =
        Err("QueryError::StaleArtifact: index commit differs from HEAD".to_string());
    mock.docs_result = Err("DocsIndexLoadError::SourceIdentityMismatch".to_string());
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "AnySymbol".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute plan with blocked ops");
    assert_eq!(subgraph.records.len(), 2);
    assert!(matches!(
        subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { .. }
    ));
    assert!(matches!(
        subgraph.records[1].support_status,
        EvidenceSupportStatus::Blocked { .. }
    ));
}

#[test]
fn record_budget_exhaustion_returns_bounded_truncated_subgraph() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![
        EvidenceRecord {
            id: "Symbol:first".to_string(),
            source_location: Some(SourceLocation {
                path: "src/first.rs".to_string(),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: None,
            retrieval_reason: "first match".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Symbol:second".to_string(),
            source_location: Some(SourceLocation {
                path: "src/second.rs".to_string(),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: None,
            retrieval_reason: "second match".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:first".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            max_records: 1,
            ..PlannerBudget::default()
        },
    };

    let subgraph = engine
        .execute(plan)
        .expect("record-budget exhaustion returns a bounded incomplete result");
    assert!(
        subgraph.truncated,
        "an exhausted record budget must mark the subgraph truncated"
    );
    assert_eq!(
        subgraph.records.len(),
        1,
        "records collected before exhaustion must survive"
    );
}

#[test]
fn artifact_budget_exhaustion_surfaces_planner_error() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            max_records: 3,
            max_artifacts: 1,
            max_context_bytes: 10_000,
            ..PlannerBudget::default()
        },
    };
    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");
    let record = |id: &str, path: &str| EvidenceRecord {
        id: id.to_string(),
        source_location: Some(SourceLocation {
            path: path.to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: None,
        retrieval_reason: "artifact budget boundary".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };

    subgraph
        .add_record(record("record-1", "src/one.rs"))
        .expect("first artifact fits");
    let result = subgraph.add_record(record("record-2", "src/two.rs"));
    assert!(matches!(
        result,
        Err(PlannerModelError::BudgetExceeded(message))
            if message.contains("max_artifacts limit of 1 reached")
    ));
    assert!(subgraph.truncated);
}

#[test]
fn artifact_budget_distinguishes_versions_of_the_same_path() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            max_records: 3,
            max_artifacts: 1,
            max_context_bytes: 10_000,
            ..PlannerBudget::default()
        },
    };
    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");
    let record = |id: &str, version_or_hash: &str| EvidenceRecord {
        id: id.to_string(),
        source_location: Some(SourceLocation {
            path: "src/lib.rs".to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: Some(version_or_hash.to_string()),
        relation: None,
        retrieval_reason: "same path, distinct immutable artifact".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };

    subgraph
        .add_record(record("record-v1", "sha256:v1"))
        .expect("first artifact version fits");
    let result = subgraph.add_record(record("record-v2", "sha256:v2"));

    assert!(matches!(
        result,
        Err(PlannerModelError::BudgetExceeded(message))
            if message.contains("max_artifacts limit of 1 reached")
    ));
    assert_eq!(subgraph.records.len(), 1);
    assert!(subgraph.truncated);
}

#[test]
fn traversal_hop_budget_exhaustion_returns_bounded_truncated_subgraph() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![
        EvidenceRecord {
            id: "Impact:hop-1".to_string(),
            source_location: Some(SourceLocation {
                path: "src/hop-1.rs".to_string(),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 1".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::from([(String::from("hops"), String::from("1"))]),
        },
        EvidenceRecord {
            id: "Impact:hop-2".to_string(),
            source_location: Some(SourceLocation {
                path: "src/hop-2.rs".to_string(),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 2".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::from([(String::from("hops"), String::from("2"))]),
        },
        EvidenceRecord {
            id: "Impact:hop-3".to_string(),
            source_location: Some(SourceLocation {
                path: "src/hop-3.rs".to_string(),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 3".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::from([(String::from("hops"), String::from("3"))]),
        },
    ]);
    let engine = PlannerEngine::new(&mock);
    let plan = RetrievalPlan {
        shape: RetrievalShape::DependencyPath,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 10,
            max_artifacts: 10,
            max_context_bytes: 10_000,
            timeout_ms: 5_000,
        },
    };

    let subgraph = engine
        .execute(plan)
        .expect("hop-budget exhaustion returns a bounded incomplete result");
    assert!(
        subgraph.truncated,
        "a record beyond the hop budget must mark the subgraph truncated"
    );
    assert!(
        subgraph.records.iter().all(|record| record
            .metadata
            .get("hops")
            .is_none_or(|hops| hops.parse::<usize>().is_ok_and(|hops| hops <= 2))),
        "no emitted record may exceed the hop budget"
    );
}

#[test]
fn traversal_hop_budget_keeps_fanout_records_at_same_hop() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok((1..=3)
        .map(|index| EvidenceRecord {
            id: format!("Impact:fanout-{index}"),
            source_location: Some(SourceLocation {
                path: format!("src/fanout-{index}.rs"),
                start_line: Some(1),
                end_line: Some(2),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 1 fan-out".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::from([(String::from("hops"), String::from("1"))]),
        })
        .collect());
    let engine = PlannerEngine::new(&mock);
    let plan = RetrievalPlan {
        shape: RetrievalShape::DependencyPath,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget {
            max_hops: 1,
            max_records: 10,
            max_artifacts: 10,
            max_context_bytes: 10_000,
            timeout_ms: 5_000,
        },
    };

    let subgraph = engine.execute(plan).expect("fan-out at one hop fits");
    assert_eq!(subgraph.records.len(), 3);
}

#[test]
fn context_budget_exhaustion_surfaces_planner_error() {
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            max_records: 2,
            max_artifacts: 2,
            max_context_bytes: 1,
            ..PlannerBudget::default()
        },
    };
    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");
    let record = EvidenceRecord {
        id: "record-1".to_string(),
        source_location: Some(SourceLocation {
            path: "src/one.rs".to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: None,
        retrieval_reason: "context budget boundary".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };

    let result = subgraph.add_record(record);
    assert!(matches!(
        result,
        Err(PlannerModelError::BudgetExceeded(message))
            if message.contains("max_context_bytes limit of 1 exceeded")
    ));
    assert!(subgraph.truncated);
    assert!(subgraph.records.is_empty());
}

#[test]
fn scope_mismatch_is_preserved_as_blocked_evidence_during_execution() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:wrong-scope".to_string(),
        source_location: Some(SourceLocation {
            path: "src/wrong.rs".to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: Some("scope-forbidden".to_string()),
        version_or_hash: None,
        relation: None,
        retrieval_reason: "scope boundary".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: Some("scope-expected".to_string()),
        budget: PlannerBudget::default(),
    };

    let subgraph = engine
        .execute(plan)
        .expect("scope mismatch becomes evidence");
    assert_eq!(subgraph.records.len(), 1);
    assert!(matches!(
        &subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { reason }
            if reason.contains("project scope mismatch")
    ));
}

#[test]
fn timeout_after_provider_preserves_evidence_and_names_undispatched_operation() {
    let mut mock = MockOperationsProvider::new();
    mock.operation_delay = Some(std::time::Duration::from_millis(5));
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:slow".to_string(),
        source_location: Some(SourceLocation {
            path: "src/slow.rs".to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: None,
        retrieval_reason: "slow symbol search".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);
    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "Symbol:slow".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            timeout_ms: 1,
            ..PlannerBudget::default()
        },
    };

    let subgraph = engine
        .execute(plan)
        .expect("deadline returns a bounded incomplete result");
    assert!(subgraph.truncated);
    assert_eq!(
        subgraph
            .records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Symbol:slow", "Blocked:Timeout:docs_context:Symbol:slow"]
    );
    assert_eq!(
        *mock.call_order.lock().unwrap(),
        vec!["search_symbols".to_string()]
    );
    assert_eq!(
        subgraph.termination,
        Some(TerminationMetadata {
            operation: AllowedOperation::DocsContext,
            phase: TerminationPhase::After,
            reason: TerminationReason::Timeout,
        })
    );
}

#[test]
fn provider_timeout_becomes_blocked_evidence_without_later_dispatch() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Err("query timeout".to_string());
    let engine = PlannerEngine::new(&mock);
    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:store-timeout".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine
        .execute(plan)
        .expect("provider timeout becomes bounded evidence");

    assert!(subgraph.truncated);
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(
        subgraph.records[0].id,
        "Error:SearchSymbols:Symbol:store-timeout"
    );
    assert!(matches!(
        &subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { reason } if reason == "query timeout"
    ));
    assert_eq!(
        *mock.call_order.lock().unwrap(),
        vec!["search_symbols".to_string()]
    );
    assert_eq!(
        subgraph.termination,
        Some(TerminationMetadata {
            operation: AllowedOperation::SearchSymbols,
            phase: TerminationPhase::During,
            reason: TerminationReason::Timeout,
        })
    );
}

#[test]
fn timeout_is_checked_after_inspect_provider_operation() {
    let mut mock = MockOperationsProvider::new();
    mock.operation_delay = Some(std::time::Duration::from_millis(5));
    mock.inspect_result = Ok(Some(EvidenceRecord {
        id: "Symbol:slow-inspect".to_string(),
        source_location: Some(SourceLocation {
            path: "src/slow.rs".to_string(),
            start_line: Some(1),
            end_line: Some(2),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: None,
        retrieval_reason: "slow inspect".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }));
    let engine =
        PlannerEngine::with_allowlist(&mock, BTreeSet::from([AllowedOperation::InspectContext]));
    let plan = RetrievalPlan {
        shape: RetrievalShape::Provenance,
        anchor: "Symbol:slow-inspect".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget {
            timeout_ms: 1,
            ..PlannerBudget::default()
        },
    };

    let subgraph = engine
        .execute(plan)
        .expect("deadline returns a bounded incomplete result");
    assert!(subgraph.truncated);
    assert_eq!(subgraph.records[0].id, "Symbol:slow-inspect");
    assert!(
        subgraph
            .records
            .iter()
            .any(|record| { record.id == "Blocked:Timeout:inspect_context:Symbol:slow-inspect" })
    );
    assert_eq!(
        subgraph.termination,
        Some(TerminationMetadata {
            operation: AllowedOperation::InspectContext,
            phase: TerminationPhase::After,
            reason: TerminationReason::Timeout,
        })
    );
}

#[test]
fn termination_metadata_is_finite_bounded_and_legacy_compatible() {
    let operations = [
        AllowedOperation::SearchSymbols,
        AllowedOperation::DocsContext,
        AllowedOperation::InspectContext,
        AllowedOperation::ImpactSymbol,
        AllowedOperation::ProceduralProject,
    ];
    let phases = [
        TerminationPhase::Before,
        TerminationPhase::After,
        TerminationPhase::During,
    ];
    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "Symbol:legacy".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };
    let mut complete = EvidenceSubgraph::new(plan).expect("complete subgraph");
    complete.records.push(EvidenceRecord {
        id: "Symbol:legacy".to_string(),
        source_location: Some(SourceLocation {
            path: "src/legacy.rs".to_string(),
            start_line: Some(1),
            end_line: Some(1),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: None,
        retrieval_reason: "legacy shape".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    });
    #[derive(serde::Serialize)]
    struct LegacyEvidenceSubgraph<'a> {
        plan: &'a RetrievalPlan,
        records: &'a [EvidenceRecord],
        truncated: bool,
    }
    let legacy_json = serde_json::to_string(&LegacyEvidenceSubgraph {
        plan: &complete.plan,
        records: &complete.records,
        truncated: complete.truncated,
    })
    .expect("serialize explicit legacy shape");
    let decoded_legacy: EvidenceSubgraph =
        serde_json::from_str(&legacy_json).expect("deserialize legacy shape");
    assert_eq!(decoded_legacy, complete);
    assert_eq!(serde_json::to_string(&complete).unwrap(), legacy_json);

    for operation in operations {
        for phase in phases {
            let mut terminated = complete.clone();
            terminated.termination = Some(TerminationMetadata {
                operation,
                phase,
                reason: TerminationReason::Timeout,
            });
            let terminated_json =
                serde_json::to_string(&terminated).expect("serialize termination metadata");
            assert!(
                terminated_json.len() - legacy_json.len() <= 256,
                "termination metadata exceeded allowance: {terminated_json}"
            );
            let decoded: EvidenceSubgraph =
                serde_json::from_str(&terminated_json).expect("deserialize termination metadata");
            assert_eq!(decoded, terminated);
        }
    }
}

fn timeout_test_subgraph(budget: PlannerBudget) -> EvidenceSubgraph {
    EvidenceSubgraph::new(RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:cap".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget,
    })
    .expect("valid timeout test plan")
}

fn timeout_test_record() -> EvidenceRecord {
    EvidenceRecord {
        id: "Symbol:cap".to_string(),
        source_location: Some(SourceLocation {
            path: "src/cap.rs".to_string(),
            start_line: Some(1),
            end_line: Some(1),
        }),
        project_scope: None,
        version_or_hash: Some("cap-hash".to_string()),
        relation: None,
        retrieval_reason: "cap fixture".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }
}

#[test]
fn timeout_metadata_survives_record_artifact_and_byte_saturation() {
    let record = timeout_test_record();
    let record_len = serde_json::to_string(&record).unwrap().len();

    let mut record_capped = timeout_test_subgraph(PlannerBudget {
        max_records: 1,
        ..PlannerBudget::default()
    });
    record_capped
        .add_record(record.clone())
        .expect("record cap fixture");
    let records_before = record_capped.records.clone();
    let bytes_before = record_capped.total_serialized_bytes();
    add_timeout_record(&mut record_capped, "docs_context", 10, "before")
        .expect("record cap keeps timeout metadata");
    assert_eq!(record_capped.records, records_before);
    assert_eq!(record_capped.total_serialized_bytes(), bytes_before);
    assert_eq!(
        record_capped.termination.unwrap().operation,
        AllowedOperation::DocsContext
    );

    let mut artifact_capped = timeout_test_subgraph(PlannerBudget {
        max_artifacts: 1,
        ..PlannerBudget::default()
    });
    artifact_capped
        .add_record(record.clone())
        .expect("artifact cap fixture");
    let records_before = artifact_capped.records.clone();
    let bytes_before = artifact_capped.total_serialized_bytes();
    add_timeout_record(&mut artifact_capped, "inspect_context", 10, "after")
        .expect("artifact cap keeps timeout metadata");
    assert_eq!(artifact_capped.records, records_before);
    assert_eq!(artifact_capped.total_serialized_bytes(), bytes_before);
    assert_eq!(
        artifact_capped.termination.unwrap().operation,
        AllowedOperation::InspectContext
    );

    let mut byte_capped = timeout_test_subgraph(PlannerBudget {
        max_context_bytes: record_len,
        ..PlannerBudget::default()
    });
    byte_capped
        .add_record(record)
        .expect("exact byte cap fixture");
    let records_before = byte_capped.records.clone();
    let bytes_before = byte_capped.total_serialized_bytes();
    assert_eq!(bytes_before, byte_capped.plan.budget.max_context_bytes);
    add_timeout_record(&mut byte_capped, "impact_symbol", 10, "after")
        .expect("byte cap keeps timeout metadata");
    assert_eq!(byte_capped.records, records_before);
    assert_eq!(byte_capped.total_serialized_bytes(), bytes_before);
    assert_eq!(
        byte_capped.termination.unwrap().operation,
        AllowedOperation::ImpactSymbol
    );
}

#[test]
fn one_byte_context_budget_is_valid_and_timeout_metadata_is_still_visible() {
    let mut subgraph = timeout_test_subgraph(PlannerBudget {
        max_context_bytes: 1,
        ..PlannerBudget::default()
    });
    add_timeout_record(&mut subgraph, "procedural_project", 10, "before")
        .expect("one byte budget remains valid");
    assert!(subgraph.records.is_empty());
    assert_eq!(
        subgraph.termination,
        Some(TerminationMetadata {
            operation: AllowedOperation::ProceduralProject,
            phase: TerminationPhase::Before,
            reason: TerminationReason::Timeout,
        })
    );
}

#[test]
fn relation_filter_excludes_supported_traversal_records_outside_requested_relations() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![
        EvidenceRecord {
            id: "Impact:kept".to_string(),
            source_location: Some(SourceLocation {
                path: "src/kept.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "requested relation".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Impact:filtered".to_string(),
            source_location: Some(SourceLocation {
                path: "src/filtered.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("imports".to_string()),
            retrieval_reason: "unrequested relation".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Impact:unlabeled".to_string(),
            source_location: Some(SourceLocation {
                path: "src/unlabeled.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: None,
            retrieval_reason: "no relation label".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DependencyPath,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("relation filter executes");
    let ids = subgraph
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec!["Impact:kept"],
        "only supported evidence carrying a requested relation may be emitted"
    );
}

#[test]
fn relation_filter_preserves_blocked_traversal_evidence() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![EvidenceRecord {
        id: "Blocked:ProceduralHopLimit:root".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("procedural_hop_limit".to_string()),
        retrieval_reason: "depth reduced".to_string(),
        support_status: EvidenceSupportStatus::Blocked {
            reason: "procedural traversal depth reduced to 2 hop(s)".to_string(),
        },
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DependencyPath,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("blocked evidence survives");
    assert_eq!(subgraph.records.len(), 1);
    assert!(
        matches!(
            subgraph.records[0].support_status,
            EvidenceSupportStatus::Blocked { .. }
        ),
        "a relation filter must not hide boundary evidence"
    );
}

/// The source index emits uppercase relationship kinds such as `CALLS`, while
/// callers naturally request `calls`. Exact-equality matching would silently
/// return nothing for every real impact query.
#[test]
fn relation_filter_matches_source_index_casing() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![EvidenceRecord {
        id: "Impact:UppercaseKind".to_string(),
        source_location: Some(SourceLocation {
            path: "src/caller.rs".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: None,
        // Exactly what `indexer.rs` writes for a non-test call edge.
        relation: Some("CALLS".to_string()),
        retrieval_reason: "index relationship kind".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let subgraph = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::DependencyPath,
            anchor: "Symbol:root".to_string(),
            direction: TraversalDirection::Downstream,
            relation_types: vec!["calls".to_string()],
            project_scope: None,
            budget: PlannerBudget::default(),
        })
        .expect("relation filter executes");

    assert_eq!(
        subgraph
            .records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Impact:UppercaseKind"],
        "a lowercase request must match the index's uppercase relationship kind"
    );
}

/// A filter that removes everything must be distinguishable from genuinely
/// absent evidence, otherwise a mis-spelled relation looks like a clean answer.
#[test]
fn relation_filter_reports_when_it_removes_every_traversal_record() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![EvidenceRecord {
        id: "Impact:OtherRelation".to_string(),
        source_location: Some(SourceLocation {
            path: "src/other.rs".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: None,
        relation: Some("TESTS".to_string()),
        retrieval_reason: "test edge".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let subgraph = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::DependencyPath,
            anchor: "Symbol:root".to_string(),
            direction: TraversalDirection::Downstream,
            relation_types: vec!["imports".to_string()],
            project_scope: None,
            budget: PlannerBudget::default(),
        })
        .expect("an empty filter result is evidence, not an error");

    let report = subgraph
        .records
        .iter()
        .find(|record| record.id == "Blocked:RelationFilter:Symbol:root")
        .expect("a fully filtered traversal must be reported");
    assert!(
        matches!(
            &report.support_status,
            EvidenceSupportStatus::Blocked { reason }
                if reason.contains("imports") && reason.contains("TESTS")
        ),
        "the report must name both the requested and the observed relations"
    );
    assert_eq!(
        report.metadata.get("observed_relations"),
        Some(&"TESTS".to_string()),
        "callers must be able to read the observed relation vocabulary"
    );
}

/// A filter that keeps some evidence is working as requested, so it must not
/// add a drop report.
#[test]
fn relation_filter_does_not_report_when_some_evidence_survives() {
    let mut mock = MockOperationsProvider::new();
    mock.impact_result = Ok(vec![
        EvidenceRecord {
            id: "Impact:Kept".to_string(),
            source_location: Some(SourceLocation {
                path: "src/kept.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("CALLS".to_string()),
            retrieval_reason: "kept".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Impact:Dropped".to_string(),
            source_location: Some(SourceLocation {
                path: "src/dropped.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("TESTS".to_string()),
            retrieval_reason: "dropped".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let subgraph = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::DependencyPath,
            anchor: "Symbol:root".to_string(),
            direction: TraversalDirection::Downstream,
            relation_types: vec!["calls".to_string()],
            project_scope: None,
            budget: PlannerBudget::default(),
        })
        .expect("partial filtering executes");

    assert!(
        !subgraph
            .records
            .iter()
            .any(|record| record.id.starts_with("Blocked:RelationFilter:")),
        "a partially satisfied filter must not report a total drop"
    );
}

#[test]
fn restricted_allowlist_skips_operations_outside_the_allowlist() {
    let mock = MockOperationsProvider::new();
    let engine = PlannerEngine::with_allowlist(
        &mock,
        BTreeSet::from([
            AllowedOperation::SearchSymbols,
            AllowedOperation::DocsContext,
        ]),
    );

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Symbol:root".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    engine.execute(plan).expect("restricted allowlist executes");
    let calls = mock.call_order.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec!["search_symbols", "docs_context"],
        "operations outside the allowlist must never reach the provider"
    );
}

#[test]
fn inspect_context_error_is_recorded_as_blocked_evidence() {
    let mut mock = MockOperationsProvider::new();
    mock.inspect_result = Err("InspectError::StaleArtifact".to_string());
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Provenance,
        anchor: "Symbol:stale".to_string(),
        direction: TraversalDirection::Upstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("inspect failure is evidence");
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(subgraph.records[0].id, "Error:InspectContext:Symbol:stale");
    assert!(matches!(
        subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { .. }
    ));
}

#[test]
fn evidence_planner_tool_exposure_and_spec() {
    let thread_state = std::sync::Arc::new(crate::tool::OntographThreadState::default());
    let tool = crate::tool::EvidencePlannerTool::new(thread_state);

    use ontocode_extension_api::ToolExecutor;
    use ontocode_extension_api::ToolName;
    use ontocode_extension_api::ToolSpec;
    assert_eq!(
        tool.tool_name(),
        ToolName::namespaced(
            crate::tool::ONTOGRAPH_NAMESPACE,
            crate::tool::EVIDENCE_PLANNER_TOOL_NAME
        )
    );
    assert_eq!(
        tool.exposure(),
        ontocode_tools::ToolExposure::DirectModelOnly
    );
    assert!(tool.supports_parallel_tool_calls());

    let ToolSpec::Namespace(namespace) = tool.spec() else {
        panic!("expected namespace tool spec");
    };
    assert_eq!(namespace.name, crate::tool::ONTOGRAPH_NAMESPACE);
    assert_eq!(namespace.tools.len(), 1);

    let schema = serde_json::to_string(&namespace.tools[0]).expect("serialize tool spec");
    for field in [
        "max_hops",
        "max_records",
        "max_artifacts",
        "max_context_bytes",
        "timeout_ms",
    ] {
        assert!(schema.contains(field), "missing budget field {field}");
    }

    let args: crate::tool::EvidencePlannerArgs = serde_json::from_value(serde_json::json!({
        "shape": "direct_lookup",
        "anchor": "Symbol:root",
        "max_hops": MAX_HOPS_LIMIT,
        "max_records": MAX_RECORDS_LIMIT,
        "max_artifacts": MAX_ARTIFACTS_LIMIT,
        "max_context_bytes": MAX_CONTEXT_BYTES_LIMIT,
        "timeout_ms": MAX_TIMEOUT_MS_LIMIT
    }))
    .expect("all supported budget controls deserialize");
    let budget = PlannerBudget {
        max_hops: args.max_hops.expect("max_hops"),
        max_records: args.max_records.expect("max_records"),
        max_artifacts: args.max_artifacts.expect("max_artifacts"),
        max_context_bytes: args.max_context_bytes.expect("max_context_bytes"),
        timeout_ms: args.timeout_ms.expect("timeout_ms"),
    };
    assert!(budget.validate().is_ok());
}

#[test]
fn surface_scoped_ontograph_extension_includes_evidence_planner_tool() {
    use ontocode_extension_api::ExtensionData;
    use ontocode_extension_api::ToolContributor;
    use ontocode_extension_api::ToolName;

    let extension = crate::extension::SurfaceScopedOntographExtension::default();
    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("thread");
    let tools = extension.tools(&session_store, &thread_store);
    assert!(tools.iter().any(|t| t.tool_name()
        == ToolName::namespaced(
            crate::tool::ONTOGRAPH_NAMESPACE,
            crate::tool::EVIDENCE_PLANNER_TOOL_NAME
        )));
}

#[test]
fn evaluation_corpus_exact_lookup() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:MyTarget".to_string(),
        source_location: Some(SourceLocation {
            path: "src/target.rs".to_string(),
            start_line: Some(15),
            end_line: Some(30),
        }),
        project_scope: Some("ontocode".to_string()),
        version_or_hash: Some("sha-exact-1".to_string()),
        relation: Some("symbol_match".to_string()),
        retrieval_reason: "exact match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "MyTarget".to_string(),
        direction: TraversalDirection::Bidirectional,
        relation_types: vec![],
        project_scope: Some("ontocode".to_string()),
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute exact lookup");
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(subgraph.records[0].id, "Symbol:MyTarget");
    assert!(matches!(
        subgraph.records[0].support_status,
        EvidenceSupportStatus::Supported
    ));
    assert!(subgraph.records[0].source_location.is_some());
}

#[test]
fn evaluation_corpus_multi_hop_dependency() {
    let mut mock = MockOperationsProvider::new();
    mock.inspect_result = Ok(Some(EvidenceRecord {
        id: "Symbol:RootCaller".to_string(),
        source_location: Some(SourceLocation {
            path: "src/caller.rs".to_string(),
            start_line: Some(10),
            end_line: Some(25),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: Some("root".to_string()),
        retrieval_reason: "inspect root".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }));
    mock.impact_result = Ok(vec![
        EvidenceRecord {
            id: "Impact:CalleeHop1".to_string(),
            source_location: Some(SourceLocation {
                path: "src/hop1.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 1".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Impact:CalleeHop2".to_string(),
            source_location: Some(SourceLocation {
                path: "src/hop2.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("calls".to_string()),
            retrieval_reason: "hop 2".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DependencyPath,
        anchor: "RootCaller".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec!["calls".to_string()],
        project_scope: None,
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 10,
            max_artifacts: 5,
            max_context_bytes: 10_000,
            timeout_ms: 5000,
        },
    };

    let subgraph = engine.execute(plan).expect("execute dependency path");
    assert_eq!(subgraph.records.len(), 3);
    assert_eq!(subgraph.records[0].id, "Symbol:RootCaller");
    assert_eq!(subgraph.records[1].id, "Impact:CalleeHop1");
    assert_eq!(subgraph.records[2].id, "Impact:CalleeHop2");
}

#[test]
fn evaluation_corpus_cross_artifact_join() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:CoreEngine".to_string(),
        source_location: Some(SourceLocation {
            path: "src/engine.rs".to_string(),
            start_line: Some(1),
            end_line: Some(50),
        }),
        project_scope: None,
        version_or_hash: None,
        relation: Some("source".to_string()),
        retrieval_reason: "source match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    mock.docs_result = Ok(vec![EvidenceRecord {
        id: "Doc:docs/architecture.md".to_string(),
        source_location: Some(SourceLocation {
            path: "docs/architecture.md".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: Some("doc-sha256".to_string()),
        relation: Some("specification".to_string()),
        retrieval_reason: "docs join".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    mock.procedural_result = Ok(vec![EvidenceRecord {
        id: "KmTransition:EngineBoot".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: Some("km-fact-1".to_string()),
        relation: Some("transition".to_string()),
        retrieval_reason: "km join".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::CrossArtifactJoin,
        anchor: "CoreEngine".to_string(),
        direction: TraversalDirection::Bidirectional,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute cross artifact join");
    assert_eq!(subgraph.records.len(), 3);
    assert!(subgraph.records.iter().any(|r| r.id.starts_with("Symbol:")));
    assert!(subgraph.records.iter().any(|r| r.id.starts_with("Doc:")));
    assert!(
        subgraph
            .records
            .iter()
            .any(|r| r.id.starts_with("KmTransition:"))
    );
}

#[test]
fn evaluation_corpus_deterministic_repeatability() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![
        EvidenceRecord {
            id: "Symbol:A".to_string(),
            source_location: Some(SourceLocation {
                path: "src/a.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: Some("hA".to_string()),
            relation: None,
            retrieval_reason: "A".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Symbol:B".to_string(),
            source_location: Some(SourceLocation {
                path: "src/b.rs".to_string(),
                start_line: None,
                end_line: None,
            }),
            project_scope: None,
            version_or_hash: Some("hB".to_string()),
            relation: None,
            retrieval_reason: "B".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Provenance,
        anchor: "Symbol:A".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let run1 = engine.execute(plan.clone()).expect("run 1");
    let run2 = engine.execute(plan).expect("run 2");

    assert_eq!(run1.records, run2.records);
    assert_eq!(run1.total_serialized_bytes(), run2.total_serialized_bytes());
}

#[test]
fn evaluation_corpus_scope_isolation() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:OtherProject".to_string(),
        source_location: Some(SourceLocation {
            path: "other/src/lib.rs".to_string(),
            start_line: Some(1),
            end_line: Some(8),
        }),
        project_scope: Some("other-project".to_string()),
        version_or_hash: Some("other-project-sha".to_string()),
        relation: Some("symbol_match".to_string()),
        retrieval_reason: "cross-project candidate".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "SharedSymbol".to_string(),
        direction: TraversalDirection::Lateral,
        relation_types: vec![],
        project_scope: Some("ontocode".to_string()),
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute scope-isolated lookup");
    assert_eq!(subgraph.records.len(), 1);
    assert_eq!(
        subgraph.records[0].id,
        "Blocked:AddRecord:Symbol:OtherProject"
    );
    assert_eq!(
        subgraph.records[0].project_scope.as_deref(),
        Some("ontocode")
    );
    assert!(matches!(
        &subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { reason } if reason.contains("project scope mismatch")
    ));
}

#[test]
fn evaluation_corpus_stale_and_tampered_artifacts() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Err("SourceIdentityMismatch: index commit is stale".to_string());
    mock.docs_result = Err("SourceIdentityMismatch: artifact hash was tampered".to_string());
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Neighborhood,
        anchor: "Artifact:review".to_string(),
        direction: TraversalDirection::Downstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine
        .execute(plan)
        .expect("execute stale-artifact fixture");
    assert_eq!(subgraph.records.len(), 2);
    assert_eq!(
        subgraph.records[0].id,
        "Error:SearchSymbols:Artifact:review"
    );
    assert_eq!(subgraph.records[1].id, "Error:DocsContext:Artifact:review");
    assert!(matches!(
        &subgraph.records[0].support_status,
        EvidenceSupportStatus::Blocked { reason } if reason.contains("stale")
    ));
    assert!(matches!(
        &subgraph.records[1].support_status,
        EvidenceSupportStatus::Blocked { reason } if reason.contains("tampered")
    ));
}

#[test]
fn evaluation_corpus_unresolved_and_conflicts() {
    let mut mock = MockOperationsProvider::new();
    mock.docs_result = Ok(vec![EvidenceRecord {
        id: "Conflict:AmbiguousAnchor".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("conflict".to_string()),
        retrieval_reason: "conflicting document anchors".to_string(),
        support_status: EvidenceSupportStatus::Unresolved {
            reason: "conflicting evidence: two document anchors disagree".to_string(),
        },
        metadata: BTreeMap::new(),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::DirectLookup,
        anchor: "MissingAnchor".to_string(),
        direction: TraversalDirection::Upstream,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute unresolved fixture");
    assert_eq!(subgraph.records.len(), 2);
    assert_eq!(subgraph.records[0].id, "Conflict:AmbiguousAnchor");
    assert_eq!(subgraph.records[1].id, "Unresolved:MissingAnchor");
    assert!(matches!(
        &subgraph.records[0].support_status,
        EvidenceSupportStatus::Unresolved { reason } if reason.contains("conflicting evidence")
    ));
    assert!(matches!(
        &subgraph.records[1].support_status,
        EvidenceSupportStatus::Unresolved { reason } if reason.contains("was not found")
    ));
}

#[test]
fn evaluation_corpus_provenance_completeness() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![
        EvidenceRecord {
            id: "Symbol:WithLocation".to_string(),
            source_location: Some(SourceLocation {
                path: "src/located.rs".to_string(),
                start_line: Some(4),
                end_line: Some(9),
            }),
            project_scope: None,
            version_or_hash: None,
            relation: Some("symbol_match".to_string()),
            retrieval_reason: "source location fixture".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Artifact:WithHash".to_string(),
            source_location: None,
            project_scope: None,
            version_or_hash: Some("sha256:immutable-artifact".to_string()),
            relation: Some("artifact_match".to_string()),
            retrieval_reason: "immutable hash fixture".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
        EvidenceRecord {
            id: "Symbol:MissingProvenance".to_string(),
            source_location: None,
            project_scope: None,
            version_or_hash: None,
            relation: Some("symbol_match".to_string()),
            retrieval_reason: "incomplete provenance fixture".to_string(),
            support_status: EvidenceSupportStatus::Supported,
            metadata: BTreeMap::new(),
        },
    ]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::Provenance,
        anchor: "EvidenceSet".to_string(),
        direction: TraversalDirection::Lateral,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute provenance fixture");
    let supported = subgraph
        .records
        .iter()
        .filter(|record| matches!(record.support_status, EvidenceSupportStatus::Supported))
        .collect::<Vec<_>>();
    assert_eq!(supported.len(), 2);
    assert!(
        supported
            .iter()
            .all(|record| { record.source_location.is_some() || record.version_or_hash.is_some() })
    );

    let blocked = subgraph
        .records
        .iter()
        .find(|record| record.id == "Blocked:AddRecord:Symbol:MissingProvenance")
        .expect("missing provenance is represented as blocked evidence");
    assert!(matches!(
        &blocked.support_status,
        EvidenceSupportStatus::Blocked { reason } if reason.contains("missing provenance")
    ));
}

#[test]
fn supported_record_without_immutable_identity_is_rejected() {
    let record = EvidenceRecord {
        id: "KmTransition:pkg@1.0.0:a->b".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("next".to_string()),
        retrieval_reason: "procedural transition".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    };

    assert!(matches!(
        record.validate(),
        Err(PlannerModelError::MissingProvenance(message))
            if message.contains("requires source location or version/hash")
    ));

    let with_hash = EvidenceRecord {
        version_or_hash: Some("sha256:abc".to_string()),
        ..record
    };
    assert!(with_hash.validate().is_ok());
}

#[test]
fn blocked_record_from_tampered_km_cache_needs_no_provenance() {
    // A record the KM store refused to validate carries no trustworthy hash, so
    // it must still be representable as explicit blocked evidence.
    let record = EvidenceRecord {
        id: "Km:codebase-graph/sample@0.1.0".to_string(),
        source_location: None,
        project_scope: None,
        version_or_hash: None,
        relation: Some("knowledge_module".to_string()),
        retrieval_reason: "knowledge module `codebase-graph/sample` failed cache validation"
            .to_string(),
        support_status: EvidenceSupportStatus::Blocked {
            reason: "KM index is invalid".to_string(),
        },
        metadata: BTreeMap::new(),
    };

    assert!(record.validate().is_ok());

    let plan = RetrievalPlan {
        shape: RetrievalShape::CrossArtifactJoin,
        anchor: "review".to_string(),
        direction: TraversalDirection::Lateral,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };
    let mut subgraph = EvidenceSubgraph::new(plan).expect("new subgraph");
    subgraph.add_record(record).expect("add blocked record");
    assert_eq!(subgraph.records.len(), 1);
}

#[test]
fn cross_artifact_join_returns_source_docs_graph_and_km_evidence() {
    let mut mock = MockOperationsProvider::new();
    mock.symbols_result = Ok(vec![EvidenceRecord {
        id: "Symbol:ReviewEngine".to_string(),
        source_location: Some(SourceLocation {
            path: "src/review.rs".to_string(),
            start_line: Some(10),
            end_line: Some(40),
        }),
        project_scope: None,
        version_or_hash: Some("commit-sha".to_string()),
        relation: Some("symbol_match".to_string()),
        retrieval_reason: "textual match".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    mock.docs_result = Ok(vec![EvidenceRecord {
        id: "Doc:docs/review.md".to_string(),
        source_location: Some(SourceLocation {
            path: "docs/review.md".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: Some("commit-sha".to_string()),
        relation: Some("document".to_string()),
        retrieval_reason: "docs excerpt".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }]);
    mock.inspect_result = Ok(Some(EvidenceRecord {
        id: "Symbol:ReviewEngine:context".to_string(),
        source_location: Some(SourceLocation {
            path: "src/review.rs".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: Some("commit-sha".to_string()),
        relation: Some("inspected_symbol".to_string()),
        retrieval_reason: "context inspection".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::new(),
    }));
    mock.procedural_result = Ok(vec![EvidenceRecord {
        id: "KmTransition:codebase-graph/sample@0.1.0:review->verify".to_string(),
        source_location: Some(SourceLocation {
            path: "/cache/codebase-graph/sample/0.1.0".to_string(),
            start_line: None,
            end_line: None,
        }),
        project_scope: None,
        version_or_hash: Some("km-content-hash".to_string()),
        relation: Some("next".to_string()),
        retrieval_reason: "procedural transition 1 hop(s) from `review`".to_string(),
        support_status: EvidenceSupportStatus::Supported,
        metadata: BTreeMap::from([("hops".to_string(), "1".to_string())]),
    }]);
    let engine = PlannerEngine::new(&mock);

    let plan = RetrievalPlan {
        shape: RetrievalShape::CrossArtifactJoin,
        anchor: "review".to_string(),
        direction: TraversalDirection::Lateral,
        relation_types: vec![],
        project_scope: None,
        budget: PlannerBudget::default(),
    };

    let subgraph = engine.execute(plan).expect("execute cross artifact join");

    let ids = subgraph
        .records
        .iter()
        .map(|record| record.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "Symbol:ReviewEngine",
            "Doc:docs/review.md",
            "Symbol:ReviewEngine:context",
            "KmTransition:codebase-graph/sample@0.1.0:review->verify",
        ]
    );
    assert!(
        subgraph
            .records
            .iter()
            .all(|record| record.version_or_hash.is_some())
    );

    let calls = mock.call_order.lock().unwrap().clone();
    assert_eq!(
        calls,
        vec![
            "search_symbols",
            "docs_context",
            "inspect_context",
            "procedural_project",
        ]
    );
}

fn real_provider_test_workspace() -> (
    tempfile::TempDir,
    std::sync::Arc<crate::tool::OntographThreadState>,
) {
    const KM_MANIFEST: &str = r#"id = "codebase-graph/evidence-planner"
version = "0.1.0"
kind = "codebase-graph"
summary = "A bounded real-provider corpus for evidence planner evaluation."
sources = ["silver:doc"]
licenses = ["MIT"]
[provenance]
created_by = "evidence-planner-tests"
generated_at = "2026-07-01"
source_commit = "TEST"
"#;
    const KM_FACTS: &str = "{\"id\":\"planner-fact\",\"kind\":\"rule\",\"claim\":\"real provider evidence is bounded\",\"source\":\"silver:doc\",\"confidence\":0.9}\n";

    let dir = tempfile::tempdir().expect("temp dir");
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join("src")).expect("create src");
    std::fs::create_dir_all(workspace.join("docs")).expect("create docs");
    std::fs::create_dir(workspace.join(".git")).expect("create git marker");
    std::fs::write(
        workspace.join("src/review.rs"),
        concat!(
            "pub fn review_engine() { review_alpha(); review_beta(); }\n",
            "fn review_alpha() { review_cycle(); }\n",
            "fn review_beta() { review_leaf(); }\n",
            "fn review_cycle() { review_engine(); }\n",
            "fn review_leaf() {}\n",
            "fn review_helper() {}\n",
        ),
    )
    .expect("write source corpus");
    std::fs::write(
        workspace.join("docs/review.md"),
        "# Review Engine\n\nThe review_engine uses bounded evidence and provenance.\n",
    )
    .expect("write docs corpus");
    std::fs::write(workspace.join("doc"), "KM source for the review engine.\n")
        .expect("write KM source");

    let committed_head = crate::indexer::resolve_target_head(&workspace);
    let target_head = format!(
        "{committed_head}-{}",
        crate::indexer::rust_workspace_state_key(&workspace, &committed_head)
            .expect("workspace fingerprint")
    );
    crate::indexer::index_rust_workspace(crate::indexer::RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: crate::manifest::OntographArtifactRoot::new(&workspace),
        generation_id: target_head.clone(),
        target_head,
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_000,
    })
    .expect("publish source and docs indexes");

    let km_root = dir.path().join("evidence-planner-km");
    std::fs::create_dir_all(km_root.join("examples")).expect("create KM examples");
    std::fs::create_dir_all(km_root.join("tests")).expect("create KM tests");
    std::fs::write(km_root.join("km.toml"), KM_MANIFEST).expect("write KM manifest");
    std::fs::write(km_root.join("facts.jsonl"), KM_FACTS).expect("write KM facts");
    std::fs::write(
        km_root.join("transitions.jsonl"),
        serde_json::json!({
            "source": "review_engine",
            "target": "verify",
            "relation": "next",
            "condition": "evidence is complete",
            "guidance": "Verify the grounded evidence.",
            "pitfalls": ["Do not infer unsupported facts"]
        })
        .to_string(),
    )
    .expect("write KM transitions");
    std::fs::write(km_root.join("examples/README.md"), "example").expect("write KM example");
    std::fs::write(km_root.join("tests/README.md"), "test").expect("write KM test");
    let store = crate::km_store::KmStore::for_project_path(&workspace).expect("KM store");
    store
        .import_for_project(&km_root, &workspace.display().to_string())
        .expect("import KM package");

    let thread_state = std::sync::Arc::new(crate::tool::OntographThreadState::default());
    thread_state.set_current_cwd(workspace);
    (dir, thread_state)
}

#[test]
fn real_provider_impact_capacity_preserves_exact_boundary_and_legacy() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    let legacy = provider
        .impact_symbol("review_engine", TraversalDirection::Downstream, 2)
        .expect("legacy impact");
    assert_eq!(legacy.len(), 4);
    for capacity in [0, 1, 4, 5] {
        let result = provider
            .impact_symbol_bounded("review_engine", TraversalDirection::Downstream, 2, capacity)
            .expect("bounded impact");
        assert_eq!(
            result,
            TraversalResult {
                records: legacy.iter().take(capacity).cloned().collect(),
                truncated: capacity < legacy.len(),
            }
        );
    }
    let empty = provider
        .impact_symbol_bounded("review_helper", TraversalDirection::Downstream, 2, 0)
        .expect("empty impact");
    assert_eq!(
        empty,
        TraversalResult {
            records: vec![],
            truncated: false
        }
    );
}

#[test]
fn real_provider_impact_traversal_handles_fan_out_cycles_and_directions() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);

    let cases = [
        (
            TraversalDirection::Upstream,
            vec!["Impact:review_cycle", "Impact:review_alpha"],
        ),
        (
            TraversalDirection::Downstream,
            vec![
                "Impact:review_alpha",
                "Impact:review_beta",
                "Impact:review_cycle",
                "Impact:review_leaf",
            ],
        ),
        (
            TraversalDirection::Bidirectional,
            vec![
                "Impact:review_cycle",
                "Impact:review_alpha",
                "Impact:review_beta",
                "Impact:review_leaf",
            ],
        ),
        (
            TraversalDirection::Lateral,
            vec![
                "Impact:review_cycle",
                "Impact:review_alpha",
                "Impact:review_beta",
                "Impact:review_leaf",
            ],
        ),
    ];

    for (direction, expected_ids) in cases {
        let result = provider
            .impact_symbol_bounded("review_engine", direction, 3, 8)
            .expect("real-provider traversal");
        assert_eq!(
            result
                .records
                .iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert!(!result.truncated);
        assert_eq!(
            result
                .records
                .iter()
                .filter(|record| record.id == "Impact:review_engine")
                .count(),
            0,
            "the cycle must not re-admit the anchor"
        );
    }
}

#[test]
fn real_provider_impact_caps_each_store_query_and_rejects_exhausted_callers() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::with_timeout(&thread_state, 250);

    crate::indexer::reset_impact_symbol_test_observation();
    let result = provider
        .impact_symbol_bounded("review_engine", TraversalDirection::Downstream, 3, 4)
        .expect("bounded real-provider impact");
    assert_eq!(result.records.len(), 4);
    assert!(!result.truncated);

    let observations = crate::indexer::impact_symbol_test_observation();
    assert_eq!(
        observations
            .iter()
            .map(|(symbol, max_items, _)| (symbol.as_str(), *max_items))
            .collect::<Vec<_>>(),
        vec![
            ("review_engine", 5),
            ("review_alpha", 3),
            ("review_beta", 2),
            ("review_cycle", 1),
            ("review_leaf", 1),
        ],
        "every frontier request must use remaining capacity plus one overflow candidate"
    );
    assert!(
        observations
            .iter()
            .all(|(_, max_items, timeout_ms)| *max_items <= 5 && (1..=250).contains(timeout_ms)),
        "nested store requests must retain the request-derived item and time caps"
    );

    let exhausted = crate::tool::ThreadStateOperationsProvider::with_timeout(&thread_state, 0);
    crate::indexer::reset_impact_symbol_test_observation();
    assert_eq!(
        exhausted.impact_symbol_bounded("review_engine", TraversalDirection::Downstream, 3, 4,),
        Err("Timeout".to_string())
    );
    assert!(
        crate::indexer::impact_symbol_test_observation().is_empty(),
        "an exhausted caller must not enter the source index store"
    );
}

#[test]
fn real_provider_impact_fanout_cycles_and_directions_are_bounded() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let workspace = thread_state.current_repo_root().expect("current repo root");
    let calls = (0..24)
        .map(|index| format!("leaf_{index:02}();"))
        .collect::<String>();
    let leaves = (0..24)
        .map(|index| format!("fn leaf_{index:02}() {{ hub(); }}\n"))
        .collect::<String>();
    std::fs::write(
        workspace.join("src/fanout.rs"),
        format!("fn hub() {{ {calls} }}\n{leaves}"),
    )
    .expect("write cyclic fanout corpus");
    let committed_head = crate::indexer::resolve_target_head(&workspace);
    let target_head = format!(
        "{committed_head}-{}",
        crate::indexer::rust_workspace_state_key(&workspace, &committed_head)
            .expect("workspace fingerprint")
    );
    crate::indexer::index_rust_workspace(crate::indexer::RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: crate::manifest::OntographArtifactRoot::new(&workspace),
        generation_id: target_head.clone(),
        target_head,
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_001,
    })
    .expect("publish fanout source index");
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    for direction in [
        TraversalDirection::Upstream,
        TraversalDirection::Downstream,
        TraversalDirection::Bidirectional,
        TraversalDirection::Lateral,
    ] {
        for capacity in [0, 1, 20, 23, 24, 25] {
            let result = provider
                .impact_symbol_bounded("hub", direction, /*max_hops*/ 3, capacity)
                .expect("bounded cyclic impact");
            assert_eq!(result.records.len(), capacity.min(24));
            assert_eq!(result.truncated, capacity < 24);
            assert_eq!(
                result
                    .records
                    .iter()
                    .map(|record| record.id.clone())
                    .collect::<std::collections::BTreeSet<_>>(),
                (0..capacity.min(24))
                    .map(|index| format!("Impact:leaf_{index:02}"))
                    .collect()
            );
            assert!(
                result
                    .records
                    .iter()
                    .all(|record| record.metadata.get("hops").map(String::as_str) == Some("1"))
            );
            assert_eq!(
                result,
                provider
                    .impact_symbol_bounded("hub", direction, /*max_hops*/ 3, capacity)
                    .expect("repeat impact")
            );
        }
    }
}

#[test]
fn real_provider_timeout_stops_scan_without_abandoned_work() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let workspace = thread_state.current_repo_root().expect("current repo root");
    let symbol_count = crate::indexer::QUERY_DEADLINE_CHECK_STRIDE + 1;
    let contents = (0..symbol_count)
        .map(|index| format!("fn deadline_symbol_{index}() {{}}\n"))
        .collect::<String>();
    std::fs::write(workspace.join("src/deadline.rs"), contents).expect("write deadline corpus");

    let committed_head = crate::indexer::resolve_target_head(&workspace);
    let target_head = format!(
        "{committed_head}-{}",
        crate::indexer::rust_workspace_state_key(&workspace, &committed_head)
            .expect("workspace fingerprint")
    );
    crate::indexer::index_rust_workspace(crate::indexer::RustIndexRequest {
        repo_root: workspace.clone(),
        artifact_root: crate::manifest::OntographArtifactRoot::new(&workspace),
        generation_id: target_head.clone(),
        target_head,
        implementation_version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp_unix_seconds: 1_725_000_001,
    })
    .expect("publish deadline source index");

    // Inject expiry at the scan checkpoint without racing provider admission.
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    crate::indexer::force_query_scan_timeout_after_items(1);
    let result = provider.search_symbols("missing", symbol_count);
    let scanned_at_return = crate::indexer::query_scan_test_items();
    crate::indexer::clear_query_scan_timeout();

    assert_eq!(result, Err("Timeout".to_string()));
    assert_eq!(
        scanned_at_return,
        crate::indexer::QUERY_DEADLINE_CHECK_STRIDE
    );
    assert!(
        scanned_at_return < symbol_count + 2,
        "the provider must return before the source artifact scan completes"
    );
    std::thread::yield_now();
    assert_eq!(
        crate::indexer::query_scan_test_items(),
        scanned_at_return,
        "request-owned scan state must not mutate after the provider returns"
    );
}

#[test]
fn evaluation_corpus_real_provider_reports_recall_rejection_provenance_determinism_and_bounds() {
    // Evidence source: the native source index, docs index, and KM store are
    // built in this test and consumed through ThreadStateOperationsProvider.
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    let engine = PlannerEngine::new(&provider);
    let plan = RetrievalPlan {
        shape: RetrievalShape::CrossArtifactJoin,
        anchor: "review_engine".to_string(),
        direction: TraversalDirection::Bidirectional,
        relation_types: Vec::new(),
        project_scope: Some(
            thread_state
                .current_repo_root()
                .expect("workspace scope")
                .display()
                .to_string(),
        ),
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 8,
            max_artifacts: 8,
            max_context_bytes: 8 * 1024,
            timeout_ms: 5_000,
        },
    };

    let first = engine.execute(plan.clone()).expect("real-provider run");
    let second = engine.execute(plan).expect("repeat real-provider run");

    let has_source = first
        .records
        .iter()
        .any(|record| record.id.starts_with("Symbol:"));
    let has_docs = first
        .records
        .iter()
        .any(|record| record.id.starts_with("Doc:") || record.id.starts_with("DocFact:"));
    let has_km = first
        .records
        .iter()
        .any(|record| record.id.starts_with("KmTransition:"));
    assert!(has_source, "recall missed source-index evidence");
    assert!(has_docs, "recall missed docs-index evidence");
    assert!(has_km, "recall missed KM-store evidence");

    assert!(
        first.records.iter().all(|record| {
            matches!(record.support_status, EvidenceSupportStatus::Supported)
                && (record.source_location.is_some() || record.version_or_hash.is_some())
        }),
        "supported real-provider evidence must carry provenance"
    );
    assert_eq!(
        first, second,
        "real-provider retrieval must be deterministic"
    );
    assert!(!first.truncated, "real-provider corpus must fit its budget");
    assert!(first.records.len() <= 8);
    assert!(first.total_serialized_bytes() <= 8 * 1024);
    let artifact_keys = first
        .records
        .iter()
        .map(|record| {
            record
                .source_location
                .as_ref()
                .map(|location| format!("path:{}", location.path))
                .or_else(|| {
                    record
                        .version_or_hash
                        .as_ref()
                        .map(|hash| format!("hash:{hash}"))
                })
                .expect("supported real-provider record identity")
        })
        .collect::<BTreeSet<_>>();
    assert!(artifact_keys.len() <= 8, "artifact bound exceeded");

    let rejected = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::CrossArtifactJoin,
            anchor: "review_engine".to_string(),
            direction: TraversalDirection::Bidirectional,
            relation_types: Vec::new(),
            project_scope: Some("wrong-project-scope".to_string()),
            budget: PlannerBudget::default(),
        })
        .expect("scope rejection is represented as evidence");
    assert!(!rejected.records.is_empty(), "rejection must be observable");
    assert!(rejected.records.iter().all(|record| {
        matches!(
            &record.support_status,
            EvidenceSupportStatus::Blocked { reason } if reason.contains("project scope mismatch")
        )
    }));
}

/// The planner budget advertises up to five hops, but the procedural
/// projection clamps traversal to its own lower cap. The reduced depth must
/// reach the caller as blocked evidence instead of silently shrinking the
/// traversal.
#[test]
fn real_provider_reports_reduced_procedural_hop_depth_as_blocked_evidence() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);

    let requested_hops = crate::procedural_graph::MAX_REQUESTED_HOPS + 1;
    let records = provider
        .procedural_project("review_engine", requested_hops)
        .expect("procedural projection runs against the real KM store");
    let limit_record = records
        .iter()
        .find(|record| record.id == "Blocked:ProceduralHopLimit:review_engine")
        .expect("a clamped hop request must be reported as evidence");
    assert!(
        matches!(
            &limit_record.support_status,
            EvidenceSupportStatus::Blocked { reason }
                if reason.contains("depth reduced")
        ),
        "the reduced traversal depth must be explicit, not silent"
    );
    assert_eq!(
        limit_record.metadata.get("effective_hops"),
        Some(&crate::procedural_graph::MAX_REQUESTED_HOPS.to_string()),
        "callers must be able to read the effective traversal depth"
    );

    let within_cap = provider
        .procedural_project("review_engine", crate::procedural_graph::MAX_REQUESTED_HOPS)
        .expect("projection within the cap runs");
    assert!(
        !within_cap
            .iter()
            .any(|record| record.id.starts_with("Blocked:ProceduralHopLimit:")),
        "a request within the cap must not report a reduced depth"
    );
}

#[test]
fn real_provider_procedural_project_bounded_zero_exact_overflow_and_legacy() {
    let (dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    let workspace = thread_state.current_repo_root().expect("workspace");

    let zero = provider
        .procedural_project_bounded("review_engine", 1, 0)
        .expect("zero-capacity projection");
    assert!(
        zero.truncated,
        "omitted reachable evidence must prove overflow"
    );
    assert!(zero.records.is_empty());

    let exact = provider
        .procedural_project_bounded("review_engine", 1, 1)
        .expect("exact-capacity projection");
    assert!(
        !exact.truncated,
        "exact capacity with no omitted evidence is complete"
    );
    assert_eq!(exact.records.len(), 1);
    assert!(exact.records[0].id.starts_with("KmTransition:"));

    // A second eligible module with the same anchor proves overflow after the
    // first retained transition fills capacity.
    let overflow_km_root = dir.path().join("evidence-planner-overflow-km");
    std::fs::create_dir_all(overflow_km_root.join("examples")).expect("overflow examples");
    std::fs::create_dir_all(overflow_km_root.join("tests")).expect("overflow tests");
    std::fs::write(
        overflow_km_root.join("km.toml"),
        r#"id = "codebase-graph/evidence-planner-overflow"
version = "0.1.0"
kind = "codebase-graph"
summary = "Second eligible module used to prove bounded overflow."
sources = ["silver:doc"]
licenses = ["MIT"]
[provenance]
created_by = "evidence-planner-tests"
generated_at = "2026-07-01"
source_commit = "TEST"
"#,
    )
    .expect("write overflow KM manifest");
    std::fs::write(
        overflow_km_root.join("facts.jsonl"),
        serde_json::json!({
            "id": "overflow-fact",
            "kind": "rule",
            "claim": "overflow module is eligible",
            "source": "silver:doc",
            "confidence": 0.9
        })
        .to_string(),
    )
    .expect("write overflow KM facts");
    std::fs::write(
        overflow_km_root.join("transitions.jsonl"),
        serde_json::json!({
            "source": "review_engine",
            "target": "publish",
            "relation": "next",
            "condition": "verification passed",
            "guidance": "Publish the verified evidence.",
            "pitfalls": ["Do not publish blocked evidence"]
        })
        .to_string(),
    )
    .expect("write overflow KM transitions");
    std::fs::write(overflow_km_root.join("examples/README.md"), "example")
        .expect("write overflow example");
    std::fs::write(overflow_km_root.join("tests/README.md"), "test").expect("write overflow test");
    let store = crate::km_store::KmStore::for_project_path(&workspace).expect("KM store");
    store
        .import_for_project(&overflow_km_root, &workspace.display().to_string())
        .expect("import overflow KM package");

    let overflow = provider
        .procedural_project_bounded("review_engine", 1, 1)
        .expect("overflow projection");
    assert!(
        overflow.truncated,
        "a later eligible module omitted at full capacity must prove overflow"
    );
    assert_eq!(overflow.records.len(), 1);
    assert!(overflow.records[0].id.starts_with("KmTransition:"));

    let legacy = provider
        .procedural_project("review_engine", 1)
        .expect("legacy projection");
    let bounded_default = provider
        .procedural_project_bounded("review_engine", 1, 32)
        .expect("bounded default-cap projection");
    assert_eq!(
        legacy, bounded_default.records,
        "legacy procedural_project must delegate to the bounded override"
    );
    assert!(
        !bounded_default.truncated,
        "default-cap capacity fits both eligible transitions without omission"
    );
}

#[test]
fn real_provider_procedural_project_bounded_hop_limit_at_zero_capacity() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    let requested_hops = crate::procedural_graph::MAX_REQUESTED_HOPS + 1;

    let zero = provider
        .procedural_project_bounded("review_engine", requested_hops, 0)
        .expect("zero-capacity hop-limit projection");
    assert!(
        zero.truncated,
        "hop-limit blocked evidence proves overflow at zero capacity"
    );
    assert!(zero.records.is_empty());

    let retained = provider
        .procedural_project_bounded("review_engine", requested_hops, 1)
        .expect("capacity retains hop-limit metadata");
    let limit_record = retained
        .records
        .iter()
        .find(|record| record.id == "Blocked:ProceduralHopLimit:review_engine")
        .expect("hop-limit evidence must be retained when capacity allows");
    assert_eq!(
        limit_record.metadata.get("effective_hops"),
        Some(&crate::procedural_graph::MAX_REQUESTED_HOPS.to_string()),
        "callers must still read the effective traversal depth"
    );
    assert!(
        retained.truncated,
        "retaining only the hop-limit record must omit the reachable transition"
    );
}

/// Measures the lexical retrieval limit that the evaluation decision depends
/// on. The anchor shares no lexical token with its intended target, so a miss
/// here is evidence about retrieval capability rather than fixture wiring.
#[test]
fn real_provider_non_lexical_anchor_is_reported_as_unresolved() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    let engine = PlannerEngine::new(&provider);

    let lexical = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::DirectLookup,
            anchor: "review_engine".to_string(),
            direction: TraversalDirection::Bidirectional,
            relation_types: Vec::new(),
            project_scope: None,
            budget: PlannerBudget::default(),
        })
        .expect("lexical anchor resolves");
    assert!(
        lexical
            .records
            .iter()
            .any(|record| record.id.starts_with("Symbol:")),
        "the lexical control case must resolve its target"
    );

    let paraphrased = engine
        .execute(RetrievalPlan {
            shape: RetrievalShape::DirectLookup,
            anchor: "appraisal machinery".to_string(),
            direction: TraversalDirection::Bidirectional,
            relation_types: Vec::new(),
            project_scope: None,
            budget: PlannerBudget::default(),
        })
        .expect("a retrieval miss is returned as evidence, not an error");
    assert!(
        paraphrased.records.iter().any(|record| matches!(
            &record.support_status,
            EvidenceSupportStatus::Unresolved { .. }
        )),
        "a vocabulary-mismatched anchor must be reported as unresolved"
    );
    assert!(
        !paraphrased
            .records
            .iter()
            .any(|record| record.id.starts_with("Symbol:")),
        "lexical retrieval cannot reach a target that shares no token with the anchor"
    );
}

/// Store queries enforce `QueryBudget::timeout_ms` themselves, so the provider
/// must pass the planner's remaining budget rather than the fixed per-query
/// maximum. Without that, a caller's `timeout_ms` is discarded entirely.
#[test]
fn real_provider_passes_remaining_budget_to_store_queries() {
    let (_dir, thread_state) = real_provider_test_workspace();

    // A provider with no planner deadline keeps the previous fixed bound.
    let unbounded = crate::tool::ThreadStateOperationsProvider::new(&thread_state);
    assert_eq!(
        unbounded.query_timeout_ms(),
        crate::query::MAX_QUERY_TIMEOUT_MS,
        "an unbounded provider must preserve the per-query maximum"
    );
    crate::indexer::reset_source_search_test_observation();
    unbounded
        .search_symbols("review_engine", 1)
        .expect("unbounded real-provider search");
    assert_eq!(
        crate::indexer::source_search_test_observation(),
        (1, Some(crate::query::MAX_QUERY_TIMEOUT_MS)),
        "an unbounded provider must issue one store request with the per-query maximum"
    );

    // A caller asking for less than the per-query maximum must actually get
    // less, otherwise one traversal can outlive the whole planner budget.
    let bounded = crate::tool::ThreadStateOperationsProvider::with_timeout(&thread_state, 250);
    crate::indexer::reset_source_search_test_observation();
    bounded
        .search_symbols("review_engine", 1)
        .expect("bounded real-provider search");
    let (request_count, timeout_ms) = crate::indexer::source_search_test_observation();
    assert_eq!(
        request_count, 1,
        "the provider must issue one store request"
    );
    let timeout_ms = timeout_ms.expect("the store must observe the query budget");
    assert!(
        (1..=250).contains(&timeout_ms),
        "store queries must be bounded by the caller's remaining budget, got {timeout_ms}ms"
    );
    assert!(
        timeout_ms < crate::query::MAX_QUERY_TIMEOUT_MS,
        "a 250ms request must not be widened to the {}ms per-query maximum",
        crate::query::MAX_QUERY_TIMEOUT_MS
    );

    // Admission must reject an already-exhausted caller before a store request.
    let exhausted = crate::tool::ThreadStateOperationsProvider::with_timeout(&thread_state, 0);
    crate::indexer::reset_source_search_test_observation();
    assert_eq!(
        exhausted.search_symbols("review_engine", 1),
        Err("Timeout".to_string())
    );
    assert_eq!(
        crate::indexer::source_search_test_observation(),
        (0, None),
        "an exhausted caller must not enter SourceIndexStore"
    );
}

/// Repeated execution under an identical plan must produce identical evidence.
/// Determinism is the property the evaluation report claims, so it is asserted
/// against the real provider rather than a mock.
#[test]
fn real_provider_retrieval_is_stable_across_repeated_runs() {
    let (_dir, thread_state) = real_provider_test_workspace();
    let provider = crate::tool::ThreadStateOperationsProvider::with_timeout(&thread_state, 5_000);
    let engine = PlannerEngine::new(&provider);
    let plan = RetrievalPlan {
        shape: RetrievalShape::CrossArtifactJoin,
        anchor: "review_engine".to_string(),
        direction: TraversalDirection::Bidirectional,
        relation_types: Vec::new(),
        project_scope: None,
        budget: PlannerBudget {
            max_hops: 2,
            max_records: 8,
            max_artifacts: 8,
            max_context_bytes: 8 * 1024,
            timeout_ms: 5_000,
        },
    };

    let runs = (0..3)
        .map(|_| engine.execute(plan.clone()).expect("bounded run"))
        .collect::<Vec<_>>();
    assert!(
        runs.windows(2).all(|pair| pair[0] == pair[1]),
        "repeated retrieval under an identical plan must be deterministic"
    );
    assert!(
        runs[0].total_serialized_bytes() <= 8 * 1024,
        "every run must respect the context budget"
    );
}

/// Drives the registered tool through `handle()` so the real dispatch path is
/// covered: argument parsing, scope derivation from thread state, budget
/// plumbing, and serialization. The engine-level fixtures above bypass all of
/// that by calling `PlannerEngine::execute` directly.
#[tokio::test]
async fn evidence_planner_tool_handle_derives_scope_and_returns_bounded_result() {
    use ontocode_extension_api::ToolExecutor;
    use ontocode_extension_api::ToolName;

    let (_dir, thread_state) = real_provider_test_workspace();
    let expected_scope = thread_state
        .current_repo_root()
        .expect("workspace resolves a repo root")
        .display()
        .to_string();
    let tool = crate::tool::EvidencePlannerTool::new(thread_state);

    let payload = ontocode_extension_api::ToolPayload::Function {
        arguments: serde_json::json!({
            "shape": "cross_artifact_join",
            "anchor": "review_engine",
            "max_records": 1,
        })
        .to_string(),
    };
    let call = ontocode_extension_api::ToolCall {
        turn_id: "turn-1".to_string(),
        call_id: "call-1".to_string(),
        tool_name: ToolName::namespaced(
            crate::tool::ONTOGRAPH_NAMESPACE,
            crate::tool::EVIDENCE_PLANNER_TOOL_NAME,
        ),
        model: "gpt-test".to_string(),
        truncation_policy: ontocode_utils_output_truncation::TruncationPolicy::Bytes(64 * 1024),
        conversation_history: ontocode_extension_api::ConversationHistory::default(),
        turn_item_emitter: std::sync::Arc::new(ontocode_extension_api::NoopTurnItemEmitter),
        payload: payload.clone(),
    };

    let output = tool
        .handle(call)
        .await
        .expect("an exhausted record budget must return a bounded result, not a tool error");
    let value = output
        .post_tool_use_response("call-1", &payload)
        .expect("json response");
    let subgraph = &value["subgraph"];

    assert_eq!(
        subgraph["truncated"], true,
        "max_records=1 must surface truncation through the real tool path"
    );
    let records = subgraph["records"]
        .as_array()
        .expect("subgraph carries records");
    assert_eq!(
        records.len(),
        1,
        "evidence collected before exhaustion must reach the caller"
    );
    assert_eq!(
        records[0]["project_scope"],
        serde_json::Value::String(expected_scope),
        "handle() must derive project scope from thread state"
    );
}
