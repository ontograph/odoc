use super::AuditCouplingFinding;
use super::AuditCouplingResult;
use super::run_coupling_audit;
use crate::audit_build::AuditAction;
use crate::query::BoundedItems;
use crate::query::DiffImpactRequest;
use crate::query::DiffImpactResult;
use crate::query::EvidenceRequest;
use crate::query::EvidenceResult;
use crate::query::GraphSccInputItem;
use crate::query::GraphSccInputRequest;
use crate::query::GraphSccInputResult;
use crate::query::ModuleContextRequest;
use crate::query::ModuleContextResult;
use crate::query::OntographReadQuery;
use crate::query::QueryResult;
use crate::query::RepoMapRequest;
use crate::query::RepoMapResult;
use crate::query::RouteImpactRequest;
use crate::query::RouteImpactResult;
use crate::query::RouteMapRequest;
use crate::query::RouteMapResult;
use crate::query::RouteShapeRequest;
use crate::query::RouteShapeResult;
use crate::query::SourceReferenceSet;
use crate::query::SymbolImpactRequest;
use crate::query::SymbolImpactResult;
use crate::query::SymbolSearchRequest;
use crate::query::SymbolSearchResult;
use crate::query::TestLinkRequest;
use crate::query::TestLinkResult;

struct CouplingQuery {
    edges: Vec<GraphSccInputItem>,
}

impl OntographReadQuery for CouplingQuery {
    fn search_symbols(&self, _request: SymbolSearchRequest) -> QueryResult<SymbolSearchResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn search_repomap(&self, _request: RepoMapRequest) -> QueryResult<RepoMapResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn inspect_module(&self, _request: ModuleContextRequest) -> QueryResult<ModuleContextResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn inspect_evidence(&self, _request: EvidenceRequest) -> QueryResult<EvidenceResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn inspect_shape(&self, _request: RouteShapeRequest) -> QueryResult<RouteShapeResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn impact_symbol(&self, _request: SymbolImpactRequest) -> QueryResult<SymbolImpactResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn impact_route(&self, _request: RouteImpactRequest) -> QueryResult<RouteImpactResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn impact_diff(&self, _request: DiffImpactRequest) -> QueryResult<DiffImpactResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn discover_routes(&self, _request: RouteMapRequest) -> QueryResult<RouteMapResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn graph_scc_inputs(&self, request: GraphSccInputRequest) -> QueryResult<GraphSccInputResult> {
        Ok(GraphSccInputResult {
            edges: BoundedItems::new(self.edges.clone(), request.budget),
        })
    }

    fn test_links(&self, _request: TestLinkRequest) -> QueryResult<TestLinkResult> {
        unreachable!("coupling audit only reads graph SCC inputs")
    }

    fn source_references(&self) -> SourceReferenceSet {
        SourceReferenceSet {
            paths: Default::default(),
            route_paths: Default::default(),
            symbols: Default::default(),
            symbols_by_path: Default::default(),
        }
    }
}

#[test]
fn coupling_uses_explicit_relationship_paths_not_node_id_parsing() {
    let result = run_coupling_audit(
        &CouplingQuery {
            edges: vec![GraphSccInputItem {
                relationship_id: "edge-1".to_string(),
                from_node_id: "opaque-caller-node".to_string(),
                from_symbol: "caller".to_string(),
                from_path: "src/a.rs".to_string(),
                to_node_id: "opaque-callee-node".to_string(),
                to_symbol: "callee".to_string(),
                to_path: "src/b.rs".to_string(),
            }],
        },
        5,
    )
    .expect("coupling audit should use explicit paths");

    assert_eq!(
        result,
        AuditCouplingResult {
            action: AuditAction::Coupling,
            total_pairs: 1,
            findings: vec![AuditCouplingFinding {
                from_path: "src/a.rs".to_string(),
                to_path: "src/b.rs".to_string(),
                edge_count: 1,
            }],
            truncated: false,
            limits: vec![
                "max_items=5".to_string(),
                "max_coupling_rows=100".to_string(),
            ],
            notes: vec![
                "This native slice reports file-pair coupling from existing resolved graph CALLS facts only."
                    .to_string(),
                "Same-file calls and unresolved call facts are intentionally excluded.".to_string(),
                "Rows are sorted by descending edge count, then from_path, then to_path."
                    .to_string(),
            ],
        }
    );
}
