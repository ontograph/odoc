use super::*;
use crate::indexer::SourceIndexArtifact;

fn artifact(facts: Vec<IndexedResourceFact>) -> SourceIndexArtifact {
    SourceIndexArtifact {
        schema_version: 1,
        files: Vec::new(),
        relationships: Vec::new(),
        processes: Vec::new(),
        routes: Vec::new(),
        unresolved_calls: Vec::new(),
        resource_facts: facts,
        resource_facts_truncated: false,
    }
}

fn handoff(status: &str, owner: bool) -> IndexedResourceFact {
    IndexedResourceFact {
        resource_fact_id: "resource:src/lib.rs:4:SCM_RIGHTS".to_string(),
        kind: "handoff".to_string(),
        operation: "SCM_RIGHTS".to_string(),
        path: "src/lib.rs".to_string(),
        line: 4,
        owner_node_id: owner.then(|| "symbol:src/lib.rs:1:fn:send".to_string()),
        owner_symbol: owner.then(|| "send".to_string()),
        owner_path: owner.then(|| "src/lib.rs".to_string()),
        owner_line: owner.then_some(1),
        status: status.to_string(),
        reason: None,
    }
}

#[test]
fn reports_unknown_producer_status_as_inconclusive() {
    let result = run_trace_boundary_report(
        &artifact(vec![handoff("unexpected", true)]),
        TraceBoundaryBudget::default(),
    );

    assert_eq!(result.violations, 0);
    assert_eq!(
        result.findings[0].status,
        TraceBoundaryFindingStatus::Inconclusive
    );
    assert!(result.inconclusive);
}

#[test]
fn reports_clean_indexed_boundary_as_observed_without_semantic_claim() {
    let result = run_trace_boundary_report(
        &artifact(vec![handoff("candidate", true)]),
        TraceBoundaryBudget::default(),
    );

    assert_eq!(result.observed, 1);
    assert_eq!(result.violations, 0);
    assert!(!result.inconclusive);
    assert!(
        result.findings[0]
            .detail
            .contains("semantic transfer remains unproven")
    );
}

#[test]
fn marks_truncated_artifact_inconclusive() {
    let mut source = artifact(vec![handoff("candidate", true)]);
    source.resource_facts_truncated = true;

    let result = run_trace_boundary_report(&source, TraceBoundaryBudget::default());

    assert!(result.inconclusive);
    assert!(result.truncated);
    assert!(result.findings.is_empty());
}

#[test]
fn enforces_item_and_serialized_byte_caps() {
    let source = artifact(vec![handoff("candidate", true), handoff("candidate", true)]);
    let item_capped = run_trace_boundary_report(
        &source,
        TraceBoundaryBudget {
            max_items: 1,
            ..TraceBoundaryBudget::default()
        },
    );
    assert_eq!(item_capped.findings.len(), 1);
    assert!(item_capped.truncated);
    assert!(item_capped.inconclusive);

    let byte_capped = run_trace_boundary_report(
        &source,
        TraceBoundaryBudget {
            max_serialized_bytes: 1,
            ..TraceBoundaryBudget::default()
        },
    );
    assert!(byte_capped.findings.is_empty());
    assert!(byte_capped.truncated);
    assert!(byte_capped.inconclusive);
    assert_eq!(byte_capped.serialized_bytes, 0);
}

#[test]
fn enforces_input_and_timeout_caps_as_inconclusive() {
    let source = artifact(vec![handoff("candidate", true), handoff("candidate", true)]);
    let input_capped = run_trace_boundary_report(
        &source,
        TraceBoundaryBudget {
            max_input_facts: 1,
            ..TraceBoundaryBudget::default()
        },
    );
    assert!(input_capped.inconclusive);
    assert!(input_capped.findings.is_empty());

    let timed_out = run_trace_boundary_report(
        &artifact(vec![handoff("candidate", true)]),
        TraceBoundaryBudget {
            timeout: Duration::ZERO,
            ..TraceBoundaryBudget::default()
        },
    );
    assert!(timed_out.timed_out);
    assert!(timed_out.inconclusive);
}
