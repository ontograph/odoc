use std::collections::BTreeMap;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::query::GraphSccInputRequest;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;

const MAX_COUPLING_ROWS: usize = 100;

#[cfg(test)]
#[path = "audit_coupling_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCouplingFinding {
    pub from_path: String,
    pub to_path: String,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCouplingResult {
    pub action: AuditAction,
    pub total_pairs: usize,
    pub findings: Vec<AuditCouplingFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_coupling_audit(
    query: &impl OntographReadQuery,
    max_items: usize,
) -> Result<AuditCouplingResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_COUPLING_ROWS);
    let edges = query
        .graph_scc_inputs(GraphSccInputRequest {
            budget: QueryBudget::bounded(
                QueryBudget::default().timeout_ms,
                crate::query::MAX_QUERY_ITEMS,
                QueryBudget::default().max_serialized_bytes,
            ),
        })
        .map_err(|_| {
            FunctionCallError::RespondToModel(
                "ontograph.audit failed to read graph coupling input facts".to_string(),
            )
        })?
        .edges;
    if edges.truncated {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit coupling input facts are truncated; refusing to report partial graph coupling"
                .to_string(),
        ));
    }

    let mut counts = BTreeMap::<(String, String), usize>::new();
    for edge in edges.items {
        if edge.from_path == edge.to_path {
            continue;
        }
        *counts.entry((edge.from_path, edge.to_path)).or_default() += 1;
    }

    let mut findings = counts
        .into_iter()
        .map(|((from_path, to_path), edge_count)| AuditCouplingFinding {
            from_path,
            to_path,
            edge_count,
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| {
        right
            .edge_count
            .cmp(&left.edge_count)
            .then(left.from_path.cmp(&right.from_path))
            .then(left.to_path.cmp(&right.to_path))
    });
    let total_pairs = findings.len();
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditCouplingResult {
        action: AuditAction::Coupling,
        total_pairs,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_coupling_rows={MAX_COUPLING_ROWS}"),
        ],
        notes: vec![
            "This native slice reports file-pair coupling from existing resolved graph CALLS facts only."
                .to_string(),
            "Same-file calls and unresolved call facts are intentionally excluded.".to_string(),
            "Rows are sorted by descending edge count, then from_path, then to_path.".to_string(),
        ],
    })
}
