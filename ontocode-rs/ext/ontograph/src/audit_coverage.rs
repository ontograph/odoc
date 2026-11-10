use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::query::MAX_QUERY_ITEMS;
use crate::query::MAX_QUERY_TIMEOUT_MS;
use crate::query::MAX_SERIALIZED_BYTES;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;
use crate::query::TestLinkRequest;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AuditCoverageStatus {
    Pass,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCoverageResult {
    pub action: AuditAction,
    pub status: AuditCoverageStatus,
    pub summary: AuditCoverageSummary,
    pub linked_targets: Vec<AuditCoverageLink>,
    pub unlinked_symbols: Vec<AuditCoverageSymbol>,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCoverageSummary {
    pub linked_targets: usize,
    pub unlinked_symbols: usize,
    pub test_links_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCoverageLink {
    pub test_symbol: String,
    pub test_path: String,
    pub target_symbol: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCoverageSymbol {
    pub symbol: String,
    pub path: String,
}

pub(crate) fn inconclusive_coverage_result(note: String) -> AuditCoverageResult {
    AuditCoverageResult {
        action: AuditAction::Coverage,
        status: AuditCoverageStatus::Inconclusive,
        summary: AuditCoverageSummary {
            linked_targets: 0,
            unlinked_symbols: 0,
            test_links_truncated: false,
        },
        linked_targets: Vec::new(),
        unlinked_symbols: Vec::new(),
        limits: coverage_limits(),
        notes: vec![note],
    }
}

pub(crate) fn run_coverage_audit(
    store: &impl OntographReadQuery,
    max_items: usize,
) -> AuditCoverageResult {
    let links = match store.test_links(TestLinkRequest {
        budget: QueryBudget::bounded(MAX_QUERY_TIMEOUT_MS, max_items, MAX_SERIALIZED_BYTES),
    }) {
        Ok(result) => result.links,
        Err(error) => return inconclusive_coverage_result(format!("{error:?}")),
    };
    let references = store.source_references();
    let linked_targets = links
        .items
        .into_iter()
        .map(|link| AuditCoverageLink {
            test_symbol: link.test_symbol,
            test_path: link.test_path,
            target_symbol: link.target_symbol,
            target_path: link.target_path,
        })
        .collect::<Vec<_>>();
    let linked_target_keys = linked_targets
        .iter()
        .map(|link| (link.target_path.clone(), link.target_symbol.clone()))
        .collect::<std::collections::BTreeSet<_>>();
    let unlinked_symbols = references
        .symbols_by_path
        .iter()
        .flat_map(|(path, symbols)| {
            symbols
                .iter()
                .filter(|&symbol| !linked_target_keys.contains(&(path.clone(), symbol.clone())))
                .map(|symbol| AuditCoverageSymbol {
                    symbol: symbol.clone(),
                    path: path.clone(),
                })
        })
        .take(max_items)
        .collect::<Vec<_>>();

    AuditCoverageResult {
        action: AuditAction::Coverage,
        status: AuditCoverageStatus::Pass,
        summary: AuditCoverageSummary {
            linked_targets: linked_targets.len(),
            unlinked_symbols: unlinked_symbols.len(),
            test_links_truncated: links.truncated,
        },
        linked_targets,
        unlinked_symbols,
        limits: coverage_limits(),
        notes: vec![
            "audit.coverage reads only explicit TESTS relationships from the native source index."
                .to_string(),
            "No filename-only inference, coverage percentage, or external coverage tool is used."
                .to_string(),
        ],
    }
}

fn coverage_limits() -> Vec<String> {
    vec![
        format!("audit.coverage result lists are capped at {MAX_QUERY_ITEMS} items"),
        "audit.coverage reports explicit test links and unlinked indexed symbols only".to_string(),
    ]
}
