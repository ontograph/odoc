//! Serde and JSON-schema types for the Ontograph tool surface.
//!
//! Split out of `tool.rs` (OGRC-07), which held both these declarations and
//! the `ToolExecutor` implementations. Only declarations live here; the tool
//! structs keep their private fields next to the impls that read them.

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::docs::DocsReadinessSummary;
use crate::query::DocsContextItem;
use crate::query::DocsDriftItem;
use crate::query::DocsDriftSkippedReference;
use crate::query::DocsTraceItem;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExplainModuleArgs {
    pub path: String,
    #[serde(default)]
    pub max_symbols: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiscoverArgs {
    pub action: DiscoverAction,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocsArgs {
    pub action: DocsAction,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub heading_id: Option<String>,
    #[serde(default)]
    pub requirement_id: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct InspectArgs {
    pub action: InspectAction,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub targets: Option<Vec<String>>,
    #[serde(default)]
    pub include_snippet: Option<bool>,
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KmContextArgs {
    pub id: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// `summary` returns only the KM id, version and summary; the default `full` returns the whole bounded context.
    #[serde(default)]
    pub detail: Option<KmContextDetail>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum KmContextDetail {
    Full,
    Summary,
}

#[cfg(test)]
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProceduralContextArgs {
    /// Active Knowledge Module id that declares the procedural transitions.
    pub id: String,
    /// Procedure node to localize within that module's transition graph.
    pub node: String,
    #[serde(default)]
    pub max_hops: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManageArgs {
    pub action: ManageAction,

    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImpactArgs {
    pub action: ImpactAction,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub symbols: Option<Vec<String>>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub base_ref: Option<String>,
    #[serde(default)]
    pub include_paths: Option<Vec<String>>,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchArgs {
    pub action: SearchAction,
    pub query: String,
    #[serde(default)]
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DiscoverAction {
    Groups,
    Packs,
    Repos,
    Routes,
    Sync,
    Tools,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DocsAction {
    Context,
    Drift,
    Trace,
    Readiness,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ManageAction {
    Index,
    RouteMap,
    Session,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InspectAction {
    Context,
    Evidence,
    Shape,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ImpactAction {
    Symbol,
    Batch,
    Route,
    Diff,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SearchAction {
    Semantic,
    Cypher,
    Repomap,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImplementedToolSummary {
    pub full_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DiscoverResult {
    pub namespace: String,
    pub groups: Vec<DiscoveredGroupSummary>,
    pub packs: Vec<DiscoveredPackSummary>,
    pub tools: Vec<ImplementedToolSummary>,
    pub repos: Vec<DiscoveredRepoSummary>,
    pub routes: Vec<DiscoveredRouteSummary>,
    pub routes_truncated: bool,
    pub parity_matrix_path: String,
    pub donor_actions_total: usize,
    pub donor_actions_implemented: usize,
    pub donor_actions_intentional_divergence: usize,
    pub donor_actions_missing: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DiscoveredGroupSummary {
    pub name: String,
    pub repo_roots: Vec<String>,
    pub artifact_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DiscoveredPackSummary {
    pub name: String,
    pub description: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DiscoveredRepoSummary {
    pub root: String,
    pub vcs: String,
    pub remote_url: Option<String>,
    pub active_generation_id: String,
    pub target_head: String,
    pub files: u64,
    pub symbols: u64,
    pub relationships: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DiscoveredRouteSummary {
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsResult {
    pub action: DocsAction,
    pub items: Vec<DocsContextItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drift_items: Vec<DocsDriftItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_drift_references: Vec<DocsDriftSkippedReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trace_items: Vec<DocsTraceItem>,
    pub readiness: Option<DocsReadinessSummary>,
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ExplainModuleResult {
    pub path: String,
    pub language: Option<String>,
    pub line_count: usize,
    pub top_level_symbols: Vec<String>,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectContextResult {
    pub path: String,
    pub language: Option<String>,
    pub symbols: Vec<InspectSymbolSummary>,
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectSymbolSummary {
    pub symbol: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectEvidenceResult {
    pub evidence: Vec<InspectEvidenceItem>,
    pub unresolved: Vec<InspectUnresolvedTarget>,
    pub evidence_truncated: bool,
    pub unresolved_truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectShapeResult {
    pub routes: Vec<InspectShapeRouteSummary>,
    pub routes_truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectShapeRouteSummary {
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectEvidenceItem {
    pub target: String,
    pub kind: String,
    pub path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub snippet: Option<String>,
    pub symbol: Option<String>,
    pub symbol_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct InspectUnresolvedTarget {
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum InspectResult {
    Context(InspectContextResult),
    Evidence(InspectEvidenceResult),
    Shape(InspectShapeResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct KmContextResult {
    pub id: String,
    pub version: String,
    pub context: String,
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ProceduralContextTransition {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub condition: String,
    pub guidance: String,
    pub pitfalls: Vec<String>,
    pub hops: usize,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ProceduralContextProvenance {
    pub created_by: String,
    pub generated_at: String,
    pub source_commit: String,
}

#[cfg(test)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ProceduralContextResult {
    pub id: String,
    pub version: String,
    /// Present only when the requested node was localized in the graph.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_node: Option<String>,
    pub transitions: Vec<ProceduralContextTransition>,
    /// Present only when localization failed and the caller must fall back.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub provenance: ProceduralContextProvenance,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]

pub(crate) struct ManageResult {
    pub action: ManageAction,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<crate::manifest::GraphCountSummary>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<crate::audit_session::AuditSessionRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<crate::audit_session::AuditSessionLock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_source_identity: Option<crate::audit_session::AuditSessionSourceIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_path: Option<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub routes: Vec<DiscoveredRouteSummary>,
    pub routes_truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactSymbolResult {
    pub symbol: String,
    pub upstream: Vec<ImpactSymbolSummary>,
    pub downstream: Vec<ImpactSymbolSummary>,
    pub upstream_truncated: bool,
    pub downstream_truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactBatchResult {
    pub results: Vec<ImpactBatchItemResult>,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactBatchItemResult {
    pub symbol: String,
    pub upstream: Vec<ImpactSymbolSummary>,
    pub downstream: Vec<ImpactSymbolSummary>,
    pub upstream_truncated: bool,
    pub downstream_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactRouteResult {
    pub results: Vec<ImpactRouteItemResult>,
    pub routes_truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactRouteItemResult {
    pub route: DiscoveredRouteSummary,
    pub upstream: Vec<ImpactSymbolSummary>,
    pub downstream: Vec<ImpactSymbolSummary>,
    pub upstream_truncated: bool,
    pub downstream_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactSymbolSummary {
    pub symbol: String,
    pub path: String,
    pub relationship: String,
    /// 1-based line of the call that proves this relationship. The call sits in
    /// the caller's file: `path` for upstream items, the queried symbol's file
    /// for downstream items.
    pub site_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactDiffResult {
    pub summary: ImpactDiffSummary,
    pub changed_files: Vec<ImpactDiffFileResult>,
    pub affected_processes: Vec<ImpactDiffProcessResult>,
    pub changed_files_truncated: bool,
    pub affected_processes_truncated: bool,
    pub warnings: Vec<String>,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactDiffSummary {
    pub changed_files: usize,
    pub changed_symbols: usize,
    pub affected_processes: usize,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactDiffFileResult {
    pub path: String,
    pub changed_symbols: Vec<ImpactDiffSymbolResult>,
    pub changed_symbols_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactDiffSymbolResult {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct ImpactDiffProcessResult {
    pub name: String,
    pub entry_symbol: String,
    pub path: String,
    pub changed_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub(crate) enum ImpactResult {
    Symbol(ImpactSymbolResult),
    Batch(ImpactBatchResult),
    Route(ImpactRouteResult),
    Diff(ImpactDiffResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct SearchResult {
    pub action: SearchAction,
    pub query: String,
    pub matches: Vec<SearchSymbolSummary>,
    pub files: Vec<SearchRepomapFileSummary>,
    pub truncated: bool,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct SearchSymbolSummary {
    pub symbol: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct SearchRepomapFileSummary {
    pub path: String,
    pub language: String,
    pub symbols: Vec<SearchSymbolSummary>,
}
