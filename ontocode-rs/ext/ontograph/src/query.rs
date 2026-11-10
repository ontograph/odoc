#[cfg(test)]
use std::cell::Cell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Duration;
use std::time::Instant;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

pub(crate) const MAX_QUERY_TIMEOUT_MS: u64 = 5_000;
pub(crate) const MAX_QUERY_ITEMS: usize = 200;
pub(crate) const MAX_SERIALIZED_BYTES: usize = 64 * 1024;

pub(crate) const QUERY_DEADLINE_CHECK_STRIDE: usize = 64;

#[cfg(test)]
thread_local! {
    static QUERY_SCAN_TEST_EXPIRE_AFTER_ITEMS: Cell<Option<usize>> = const { Cell::new(None) };
    static QUERY_SCAN_TEST_ITEMS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn force_query_scan_timeout_after_items(after_items: usize) {
    QUERY_SCAN_TEST_EXPIRE_AFTER_ITEMS.with(|value| value.set(Some(after_items)));
    QUERY_SCAN_TEST_ITEMS.with(|value| value.set(0));
}

#[cfg(test)]
pub(crate) fn query_scan_test_items() -> usize {
    QUERY_SCAN_TEST_ITEMS.with(Cell::get)
}

#[cfg(test)]
pub(crate) fn clear_query_scan_timeout() {
    QUERY_SCAN_TEST_EXPIRE_AFTER_ITEMS.with(|value| value.set(None));
}

pub(crate) fn query_scan_step() {
    #[cfg(test)]
    QUERY_SCAN_TEST_ITEMS.with(|value| value.set(value.get() + 1));
}

pub(crate) fn check_query_scan_deadline(deadline: QueryDeadline) -> QueryResult<()> {
    #[cfg(test)]
    if QUERY_SCAN_TEST_EXPIRE_AFTER_ITEMS.with(|after_items| {
        after_items
            .get()
            .is_some_and(|after_items| query_scan_test_items() >= after_items)
    }) {
        return Err(QueryError::Timeout);
    }
    deadline.check()
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QueryDeadline {
    started_at: Instant,
    timeout: Duration,
}

impl QueryDeadline {
    pub(crate) fn start(timeout_ms: u64) -> Self {
        Self {
            started_at: Instant::now(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub(crate) fn check(self) -> QueryResult<()> {
        if self.started_at.elapsed() > self.timeout {
            Err(QueryError::Timeout)
        } else {
            Ok(())
        }
    }
}

/// Read-only Ontograph query boundary used by future native tool handlers.
///
/// Implementations must read only completed artifact generations and must
/// enforce the budget carried by each typed request.
pub(crate) trait OntographReadQuery {
    fn search_symbols(&self, request: SymbolSearchRequest) -> QueryResult<SymbolSearchResult>;

    fn search_repomap(&self, request: RepoMapRequest) -> QueryResult<RepoMapResult>;

    fn inspect_module(&self, request: ModuleContextRequest) -> QueryResult<ModuleContextResult>;

    fn inspect_evidence(&self, request: EvidenceRequest) -> QueryResult<EvidenceResult>;

    fn inspect_shape(&self, request: RouteShapeRequest) -> QueryResult<RouteShapeResult>;

    fn impact_symbol(&self, request: SymbolImpactRequest) -> QueryResult<SymbolImpactResult>;

    fn impact_route(&self, request: RouteImpactRequest) -> QueryResult<RouteImpactResult>;

    fn impact_diff(&self, request: DiffImpactRequest) -> QueryResult<DiffImpactResult>;

    fn discover_routes(&self, request: RouteMapRequest) -> QueryResult<RouteMapResult>;

    fn graph_scc_inputs(&self, request: GraphSccInputRequest) -> QueryResult<GraphSccInputResult>;

    fn test_links(&self, request: TestLinkRequest) -> QueryResult<TestLinkResult>;

    fn source_references(&self) -> SourceReferenceSet;
}

pub(crate) type QueryResult<T> = Result<T, QueryError>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct QueryBudget {
    pub timeout_ms: u64,
    pub max_items: usize,
    pub max_serialized_bytes: usize,
}

impl QueryBudget {
    pub(crate) fn bounded(timeout_ms: u64, max_items: usize, max_serialized_bytes: usize) -> Self {
        Self {
            timeout_ms: timeout_ms.clamp(1, MAX_QUERY_TIMEOUT_MS),
            max_items: max_items.clamp(1, MAX_QUERY_ITEMS),
            max_serialized_bytes: max_serialized_bytes.clamp(1, MAX_SERIALIZED_BYTES),
        }
    }
}

impl Default for QueryBudget {
    fn default() -> Self {
        Self {
            timeout_ms: MAX_QUERY_TIMEOUT_MS,
            max_items: MAX_QUERY_ITEMS,
            max_serialized_bytes: MAX_SERIALIZED_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SymbolSearchRequest {
    pub query: String,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RepoMapRequest {
    pub query: String,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ModuleContextRequest {
    pub path: String,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EvidenceRequest {
    pub targets: Vec<String>,
    pub include_snippet: bool,
    pub context_lines: usize,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteShapeRequest {
    pub route: Option<String>,
    pub path: Option<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SymbolImpactRequest {
    pub symbol: String,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactRequest {
    pub scope: String,
    pub base_ref: Option<String>,
    pub include_paths: Vec<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteImpactRequest {
    pub route: Option<String>,
    pub path: Option<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteMapRequest {
    pub route: Option<String>,
    pub path: Option<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct GraphSccInputRequest {
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TestLinkRequest {
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocsContextRequest {
    pub document_id: Option<String>,
    pub path: Option<String>,
    pub heading_id: Option<String>,
    pub requirement_id: Option<String>,
    pub route: Option<String>,
    pub symbol: Option<String>,
    pub text: Option<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocsTraceRequest {
    pub document_id: Option<String>,
    pub requirement_id: Option<String>,
    pub route: Option<String>,
    pub path: Option<String>,
    pub symbol: Option<String>,
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DocsDriftRequest {
    pub budget: QueryBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct SourceReferenceSet {
    pub paths: BTreeSet<String>,
    pub route_paths: BTreeSet<String>,
    pub symbols: BTreeSet<String>,
    pub symbols_by_path: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct BoundedItems<T> {
    pub items: Vec<T>,
    pub truncated: bool,
    pub budget: QueryBudget,
}

impl<T> BoundedItems<T> {
    #[cfg(test)]
    pub(crate) fn new(mut items: Vec<T>, budget: QueryBudget) -> Self {
        let truncated = items.len() > budget.max_items;
        items.truncate(budget.max_items);
        Self {
            items,
            truncated,
            budget,
        }
    }
}

pub(crate) fn bounded_serialized_items<T>(
    mut items: Vec<T>,
    budget: QueryBudget,
) -> QueryResult<BoundedItems<T>>
where
    T: Serialize,
{
    let mut serialized_bytes = 2;
    let mut bounded = Vec::new();
    let mut truncated = items.len() > budget.max_items;
    for item in items.drain(..) {
        if bounded.len() == budget.max_items {
            truncated = true;
            break;
        }
        let item_bytes = serde_json::to_vec(&item)
            .map_err(|_| QueryError::CorruptArtifact)?
            .len();
        let separator_bytes = usize::from(!bounded.is_empty());
        if serialized_bytes + separator_bytes + item_bytes > budget.max_serialized_bytes {
            if bounded.is_empty() {
                return Err(QueryError::OverBudget);
            }
            truncated = true;
            break;
        }
        serialized_bytes += separator_bytes + item_bytes;
        bounded.push(item);
    }
    Ok(BoundedItems {
        items: bounded,
        truncated,
        budget,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SymbolSearchResult {
    pub matches: BoundedItems<SymbolSearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RepoMapResult {
    pub files: BoundedItems<RepoMapFileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RepoMapFileItem {
    pub path: String,
    pub language: String,
    pub symbols: Vec<SymbolSearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SymbolSearchItem {
    pub symbol: String,
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ModuleContextResult {
    pub module: ModuleContextItem,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EvidenceResult {
    pub evidence: BoundedItems<EvidenceItem>,
    pub unresolved: BoundedItems<UnresolvedEvidenceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteShapeResult {
    pub routes: BoundedItems<RouteShapeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ModuleContextItem {
    pub path: String,
    pub language: Option<String>,
    pub symbols: BoundedItems<SymbolSearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct EvidenceItem {
    pub target: String,
    pub kind: String,
    pub path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub snippet: Option<String>,
    pub symbol: Option<String>,
    pub symbol_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct UnresolvedEvidenceItem {
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteShapeItem {
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct SymbolImpactResult {
    pub upstream: BoundedItems<ImpactItem>,
    pub downstream: BoundedItems<ImpactItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ImpactItem {
    pub symbol: String,
    pub path: String,
    pub relationship: String,
    /// 1-based line of the call that proves this relationship. The call sits in
    /// the caller's file: `path` for upstream items, the queried symbol's file
    /// for downstream items.
    pub site_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactResult {
    pub summary: DiffImpactSummary,
    pub changed_files: BoundedItems<DiffImpactFile>,
    pub affected_processes: BoundedItems<DiffImpactProcess>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactSummary {
    pub changed_files: usize,
    pub changed_symbols: usize,
    pub affected_processes: usize,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactFile {
    pub path: String,
    pub changed_symbols: BoundedItems<DiffImpactSymbol>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactSymbol {
    pub symbol: String,
    pub kind: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DiffImpactProcess {
    pub name: String,
    pub entry_symbol: String,
    pub path: String,
    pub changed_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteImpactResult {
    pub routes: BoundedItems<RouteImpactItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteImpactItem {
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
    pub upstream: BoundedItems<ImpactItem>,
    pub downstream: BoundedItems<ImpactItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteMapResult {
    pub routes: BoundedItems<RouteMapItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct GraphSccInputResult {
    pub edges: BoundedItems<GraphSccInputItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TestLinkResult {
    pub links: BoundedItems<TestLinkItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct GraphSccInputItem {
    pub relationship_id: String,
    pub from_node_id: String,
    pub from_symbol: String,
    pub from_path: String,
    pub to_node_id: String,
    pub to_symbol: String,
    pub to_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct TestLinkItem {
    pub relationship_id: String,
    pub test_symbol: String,
    pub test_path: String,
    pub target_symbol: String,
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsContextResult {
    pub items: BoundedItems<DocsContextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsTraceResult {
    pub items: BoundedItems<DocsTraceItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsDriftResult {
    pub items: BoundedItems<DocsDriftItem>,
    pub skipped: BoundedItems<DocsDriftSkippedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsMigrationResult {
    pub items: BoundedItems<DocsMigrationItem>,
    pub skipped: BoundedItems<DocsMigrationSkippedReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsRequirementResult {
    pub items: BoundedItems<DocsRequirementItem>,
    pub links: BoundedItems<DocsRequirementLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DocsContextItem {
    Document {
        document_id: String,
        path: String,
        title: String,
        document_kind: String,
        excerpt: String,
    },
    Fact {
        fact_id: String,
        document_id: String,
        heading_id: Option<String>,
        fact_kind: String,
        text: String,
        line_start: usize,
        line_end: usize,
        requirement_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DocsTraceItem {
    Document {
        document_id: String,
        path: String,
        title: String,
        headings: Vec<DocsTraceHeading>,
    },
    Fact {
        fact_id: String,
        document_id: String,
        heading_id: Option<String>,
        fact_kind: String,
        text: String,
        line_start: usize,
        line_end: usize,
        requirement_id: Option<String>,
    },
    Link {
        link_id: String,
        from_document_id: String,
        to_document_id: Option<String>,
        target_kind: String,
        target: String,
        requirement_id: Option<String>,
        route: Option<String>,
        path: Option<String>,
        symbol: Option<String>,
    },
    UnresolvedSelector {
        selector: String,
        value: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsTraceHeading {
    pub heading_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DocsDriftItem {
    MissingSourcePath {
        document_id: String,
        fact_id: String,
        path: String,
    },
    MissingSymbol {
        document_id: String,
        fact_id: String,
        symbol: String,
        path: Option<String>,
    },
    MissingRoute {
        document_id: String,
        fact_id: String,
        route: String,
    },
    StaleDocumentLink {
        link_id: String,
        from_document_id: String,
        target_document_id: String,
        missing_document_id: String,
        missing_endpoint: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsDriftSkippedReference {
    pub link_id: String,
    pub target_kind: String,
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsMigrationItem {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub requirement_id: Option<String>,
    pub paths: Vec<String>,
    pub symbols: Vec<String>,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsMigrationSkippedReference {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsRequirementItem {
    pub document_id: String,
    pub fact_id: String,
    pub fact_kind: String,
    pub requirement_id: String,
    pub text: String,
    pub line_start: usize,
    pub line_end: usize,
    pub paths: Vec<String>,
    pub symbols: Vec<String>,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct DocsRequirementLink {
    pub link_id: String,
    pub from_document_id: String,
    pub requirement_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RouteMapItem {
    pub method: String,
    pub route_path: String,
    pub handler_symbol: String,
    pub path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum QueryError {
    MissingArtifact,
    AmbiguousArtifact,
    StaleArtifact,
    CorruptArtifact,
    Timeout,
    OverBudget,
    ExternalCommand,
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
