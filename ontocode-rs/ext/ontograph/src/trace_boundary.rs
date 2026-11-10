use std::time::Duration;
use std::time::Instant;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::indexer::SourceIndexArtifact;
use crate::resource_facts::IndexedResourceFact;

const DEFAULT_MAX_ITEMS: usize = 20;
const DEFAULT_MAX_INPUT_FACTS: usize = 256;
const DEFAULT_MAX_SERIALIZED_BYTES: usize = 64 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TraceBoundaryFindingStatus {
    Observed,
    Violation,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct TraceBoundaryFinding {
    pub resource_fact_id: String,
    pub operation: String,
    pub path: String,
    pub line: usize,
    pub owner_symbol: Option<String>,
    pub status: TraceBoundaryFindingStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct TraceBoundaryResult {
    pub action: String,
    pub input_facts: usize,
    pub handoff_facts: usize,
    pub observed: usize,
    pub violations: usize,
    pub inconclusive: bool,
    pub timed_out: bool,
    pub findings: Vec<TraceBoundaryFinding>,
    pub truncated: bool,
    pub serialized_bytes: usize,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TraceBoundaryBudget {
    pub max_items: usize,
    pub max_input_facts: usize,
    pub max_serialized_bytes: usize,
    pub timeout: Duration,
}

impl Default for TraceBoundaryBudget {
    fn default() -> Self {
        Self {
            max_items: DEFAULT_MAX_ITEMS,
            max_input_facts: DEFAULT_MAX_INPUT_FACTS,
            max_serialized_bytes: DEFAULT_MAX_SERIALIZED_BYTES,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

pub(crate) fn run_trace_boundary_report(
    artifact: &SourceIndexArtifact,
    budget: TraceBoundaryBudget,
) -> TraceBoundaryResult {
    let input_facts = artifact.resource_facts.len();
    let limits = vec![
        format!("max_items={}", budget.max_items),
        format!("max_input_facts={}", budget.max_input_facts),
        format!("max_serialized_bytes={}", budget.max_serialized_bytes),
        format!("timeout_ms={}", budget.timeout.as_millis()),
    ];
    let notes = vec![
        "reads explicit IndexedResourceFact records only".to_string(),
        "syntactic boundary evidence does not prove resource identity, ownership, lifetime, or runtime behavior".to_string(),
    ];

    if artifact.resource_facts_truncated || input_facts > budget.max_input_facts {
        return TraceBoundaryResult {
            action: "trace_boundary".to_string(),
            input_facts,
            handoff_facts: artifact
                .resource_facts
                .iter()
                .filter(|fact| fact.kind == "handoff")
                .count(),
            observed: 0,
            violations: 0,
            inconclusive: true,
            timed_out: false,
            findings: Vec::new(),
            truncated: true,
            serialized_bytes: 0,
            limits,
            notes,
        };
    }

    let deadline = Instant::now() + budget.timeout;
    let mut findings = Vec::new();
    let mut observed = 0;
    let mut violations = 0;
    let mut inconclusive = false;
    let mut serialized_bytes: usize = 0;
    let mut truncated = false;
    let mut timed_out = false;

    for fact in artifact
        .resource_facts
        .iter()
        .filter(|fact| fact.kind == "handoff")
    {
        if Instant::now() >= deadline {
            timed_out = true;
            inconclusive = true;
            truncated = true;
            break;
        }
        if findings.len() >= budget.max_items {
            inconclusive = true;
            truncated = true;
            break;
        }

        let finding = classify_handoff_fact(fact);
        let finding_bytes = serde_json::to_vec(&finding).map_or(usize::MAX, |bytes| bytes.len());
        if serialized_bytes.saturating_add(finding_bytes) > budget.max_serialized_bytes {
            inconclusive = true;
            truncated = true;
            break;
        }
        serialized_bytes += finding_bytes;
        match finding.status {
            TraceBoundaryFindingStatus::Observed => observed += 1,
            TraceBoundaryFindingStatus::Violation => violations += 1,
            TraceBoundaryFindingStatus::Inconclusive => inconclusive = true,
        }
        findings.push(finding);
    }

    TraceBoundaryResult {
        action: "trace_boundary".to_string(),
        input_facts,
        handoff_facts: artifact
            .resource_facts
            .iter()
            .filter(|fact| fact.kind == "handoff")
            .count(),
        observed,
        violations,
        inconclusive,
        timed_out,
        findings,
        truncated,
        serialized_bytes,
        limits,
        notes,
    }
}

fn classify_handoff_fact(fact: &IndexedResourceFact) -> TraceBoundaryFinding {
    let (status, detail) = match fact.status.as_str() {
        "candidate" if fact.owner_node_id.is_some() && fact.owner_symbol.is_some() => (
            TraceBoundaryFindingStatus::Observed,
            "explicit syntactic boundary marker with an indexed owner; semantic transfer remains unproven".to_string(),
        ),
        "candidate" => (
            TraceBoundaryFindingStatus::Inconclusive,
            "boundary marker has no proven enclosing owner".to_string(),
        ),
        "inconclusive" => (
            TraceBoundaryFindingStatus::Inconclusive,
            fact.reason
                .clone()
                .unwrap_or_else(|| "producer marked boundary evidence inconclusive".to_string()),
        ),
        other => (
            TraceBoundaryFindingStatus::Inconclusive,
            format!("resource fact uses unsupported producer status `{other}`"),
        ),
    };

    TraceBoundaryFinding {
        resource_fact_id: fact.resource_fact_id.clone(),
        operation: fact.operation.clone(),
        path: fact.path.clone(),
        line: fact.line,
        owner_symbol: fact.owner_symbol.clone(),
        status,
        detail,
    }
}

#[cfg(test)]
#[path = "trace_boundary_tests.rs"]
mod tests;
