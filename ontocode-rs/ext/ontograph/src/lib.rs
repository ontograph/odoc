mod artifact_io;
mod audit_abi;
mod audit_build;
mod audit_concurrency;
mod audit_coupling;
mod audit_coverage;
mod audit_cycles;
mod audit_drift;
mod audit_error_topology;
mod audit_fault_simulation;
mod audit_fsm;
mod audit_graph_diff;
mod audit_logic;
mod audit_migration;
mod audit_path_verify;
mod audit_pressure;
mod audit_report;
mod audit_requirements;
mod audit_session;
mod audit_session_dispatch;
mod audit_tech_debt;
mod audit_test_gap;
mod audit_verify_diff;
mod audit_violations;
mod audit_worker_scope;
mod docs;
mod docs_index;
pub(crate) mod evidence_planner;
mod extension;
mod group_state;
mod indexer;
mod km;
mod km_agent_authoring;
mod km_agent_tools;
mod km_context;
pub mod km_eval;
pub mod km_import;
pub mod km_librarian;
pub mod km_llm_client;
pub mod km_llm_consolidate;
pub mod km_llm_extract;
pub mod km_llm_judge;
pub mod km_llm_lint;
pub mod km_llm_recipe;
mod km_resolve;
mod km_store;
mod lineage_admission;
mod lineage_admission_record;
#[cfg(test)]
mod lineage_collector;
mod lineage_grounding;
mod lineage_km_retrieval;
mod lineage_records;
#[cfg(test)]
mod lineage_replay;
mod lineage_routing;
mod lineage_selection;
#[cfg(test)]
mod lineage_trace_convert;
mod manage_session;
mod manifest;
mod procedural_graph;
mod query;
mod resource_facts;
mod rust_parser;
mod tool;

pub use extension::install;
pub use extension::install_agent_knowledge_surface;
pub use extension::install_agent_knowledge_surface_with_metrics;

/// Donor-parity counts reported by the native extension.
///
/// These are the same constants the `ontograph.discover` tool reports. They are
/// exported so host surfaces do not have to parse the checked-in parity
/// Markdown, which is documentation and can drift or be corrupted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParitySummary {
    pub donor_actions_total: usize,
    pub donor_actions_implemented: usize,
    pub donor_actions_intentional_divergence: usize,
    pub donor_actions_missing: usize,
}

/// Returns the donor-parity counts compiled into this extension.
pub fn parity_summary() -> ParitySummary {
    ParitySummary {
        donor_actions_total: tool::DONOR_ACTIONS_TOTAL,
        donor_actions_implemented: tool::DONOR_ACTIONS_IMPLEMENTED,
        donor_actions_intentional_divergence: tool::DONOR_ACTIONS_INTENTIONAL_DIVERGENCE,
        donor_actions_missing: tool::donor_actions_missing(),
    }
}

/// Returns the fully qualified names of the tools this extension installs.
pub fn implemented_tool_names() -> Vec<String> {
    tool::implemented_tool_summaries()
        .into_iter()
        .map(|summary| summary.full_name)
        .collect()
}

/// Result of one successful source-index rebuild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRebuildOutcome {
    pub generation_id: String,
    pub files: u64,
    pub symbols: u64,
    pub relationships: u64,
    pub nodes: u64,
    pub edges: u64,
    pub limits: Vec<String>,
}

/// Rebuilds the local source index and publishes exactly one generation.
///
/// This is the same writer the `ontograph.manage` `index` action uses. It is
/// exported so host surfaces can trigger a rebuild without reimplementing the
/// indexing or locking rules. The call walks the workspace and blocks; callers
/// on an async runtime must run it on a blocking task.
pub fn rebuild_source_index(repo_root: &std::path::Path) -> Result<IndexRebuildOutcome, String> {
    let (generation_id, counts, limits) =
        tool::rebuild_source_index(repo_root).map_err(|error| match error {
            ontocode_extension_api::FunctionCallError::RespondToModel(message) => message,
            other => other.to_string(),
        })?;
    Ok(IndexRebuildOutcome {
        generation_id,
        files: counts.files,
        symbols: counts.symbols,
        relationships: counts.relationships,
        nodes: counts.nodes,
        edges: counts.edges,
        limits,
    })
}

#[cfg(test)]
mod lineage_session_tests;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
