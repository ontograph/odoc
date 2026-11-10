use pretty_assertions::assert_eq;

use super::*;

#[test]
fn query_budget_clamps_to_accepted_adr_limits() {
    assert_eq!(
        QueryBudget::bounded(30_000, 50_000, 1024 * 1024),
        QueryBudget {
            timeout_ms: MAX_QUERY_TIMEOUT_MS,
            max_items: MAX_QUERY_ITEMS,
            max_serialized_bytes: MAX_SERIALIZED_BYTES,
        }
    );
    assert_eq!(
        QueryBudget::bounded(0, 0, 0),
        QueryBudget {
            timeout_ms: 1,
            max_items: 1,
            max_serialized_bytes: 1,
        }
    );
}

#[test]
fn bounded_items_truncates_deterministic_results() {
    let items = vec![
        SymbolSearchItem {
            symbol: "A".to_string(),
            kind: "struct".to_string(),
            path: "src/a.rs".to_string(),
        },
        SymbolSearchItem {
            symbol: "B".to_string(),
            kind: "enum".to_string(),
            path: "src/b.rs".to_string(),
        },
    ];
    let budget = QueryBudget::bounded(100, 1, 256);

    assert_eq!(
        BoundedItems::new(items, budget.clone()),
        BoundedItems {
            items: vec![SymbolSearchItem {
                symbol: "A".to_string(),
                kind: "struct".to_string(),
                path: "src/a.rs".to_string(),
            }],
            truncated: true,
            budget,
        }
    );
}

#[test]
fn read_query_trait_returns_typed_search_inspect_and_impact_results() {
    let store = FakeReadStore;
    let budget = QueryBudget::bounded(100, 1, 512);

    assert_eq!(
        store.search_symbols(SymbolSearchRequest {
            query: "Demo".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolSearchResult {
            matches: BoundedItems {
                items: vec![SymbolSearchItem {
                    symbol: "Demo".to_string(),
                    kind: "struct".to_string(),
                    path: "src/demo.rs".to_string(),
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.search_repomap(RepoMapRequest {
            query: "Demo".to_string(),
            budget: budget.clone(),
        }),
        Ok(RepoMapResult {
            files: BoundedItems {
                items: vec![RepoMapFileItem {
                    path: "src/demo.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![SymbolSearchItem {
                        symbol: "Demo".to_string(),
                        kind: "struct".to_string(),
                        path: "src/demo.rs".to_string(),
                    }],
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.inspect_module(ModuleContextRequest {
            path: "src/demo.rs".to_string(),
            budget: budget.clone(),
        }),
        Ok(ModuleContextResult {
            module: ModuleContextItem {
                path: "src/demo.rs".to_string(),
                language: Some("rust".to_string()),
                symbols: BoundedItems {
                    items: vec![SymbolSearchItem {
                        symbol: "Demo".to_string(),
                        kind: "struct".to_string(),
                        path: "src/demo.rs".to_string(),
                    }],
                    truncated: false,
                    budget: budget.clone(),
                },
            },
        })
    );
    assert_eq!(
        store.inspect_evidence(EvidenceRequest {
            targets: vec![
                "src/demo.rs:3".to_string(),
                "Demo".to_string(),
                "Missing".to_string(),
            ],
            include_snippet: true,
            context_lines: 2,
            budget: budget.clone(),
        }),
        Ok(EvidenceResult {
            evidence: BoundedItems {
                items: vec![EvidenceItem {
                    target: "src/demo.rs:3".to_string(),
                    kind: "line".to_string(),
                    path: "src/demo.rs".to_string(),
                    start_line: Some(3),
                    end_line: Some(3),
                    snippet: Some("pub struct Demo;".to_string()),
                    symbol: None,
                    symbol_kind: None,
                }],
                truncated: true,
                budget: budget.clone(),
            },
            unresolved: BoundedItems {
                items: vec![UnresolvedEvidenceItem {
                    target: "Missing".to_string(),
                    reason: "missing in fake store".to_string(),
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.inspect_shape(RouteShapeRequest {
            route: Some("/demo".to_string()),
            path: Some("src/demo.rs".to_string()),
            budget: budget.clone(),
        }),
        Ok(RouteShapeResult {
            routes: BoundedItems {
                items: vec![RouteShapeItem {
                    method: "GET".to_string(),
                    route_path: "/demo".to_string(),
                    handler_symbol: "demo_handler".to_string(),
                    path: "src/demo.rs".to_string(),
                    line: 7,
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.impact_symbol(SymbolImpactRequest {
            symbol: "Demo".to_string(),
            budget: budget.clone(),
        }),
        Ok(SymbolImpactResult {
            upstream: BoundedItems {
                items: vec![ImpactItem {
                    symbol: "caller".to_string(),
                    path: "src/caller.rs".to_string(),
                    relationship: "CALLS".to_string(),
                    site_line: 2,
                }],
                truncated: false,
                budget: budget.clone(),
            },
            downstream: BoundedItems {
                items: Vec::new(),
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.impact_route(RouteImpactRequest {
            route: Some("/demo".to_string()),
            path: Some("src/demo.rs".to_string()),
            budget: budget.clone(),
        }),
        Ok(RouteImpactResult {
            routes: BoundedItems {
                items: vec![RouteImpactItem {
                    method: "GET".to_string(),
                    route_path: "/demo".to_string(),
                    handler_symbol: "demo_handler".to_string(),
                    path: "src/demo.rs".to_string(),
                    line: 7,
                    upstream: BoundedItems {
                        items: vec![ImpactItem {
                            symbol: "caller".to_string(),
                            path: "src/caller.rs".to_string(),
                            relationship: "CALLS".to_string(),
                            site_line: 2,
                        }],
                        truncated: false,
                        budget: budget.clone(),
                    },
                    downstream: BoundedItems {
                        items: Vec::new(),
                        truncated: false,
                        budget: budget.clone(),
                    },
                }],
                truncated: false,
                budget: budget.clone(),
            },
        })
    );
    assert_eq!(
        store.impact_diff(DiffImpactRequest {
            scope: "unstaged".to_string(),
            base_ref: None,
            include_paths: vec!["src".to_string()],
            budget: budget.clone(),
        }),
        Ok(DiffImpactResult {
            summary: DiffImpactSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            changed_files: BoundedItems {
                items: vec![DiffImpactFile {
                    path: "src/demo.rs".to_string(),
                    changed_symbols: BoundedItems {
                        items: vec![DiffImpactSymbol {
                            symbol: "Demo".to_string(),
                            kind: "struct".to_string(),
                            path: "src/demo.rs".to_string(),
                            line: 1,
                        }],
                        truncated: false,
                        budget: budget.clone(),
                    },
                }],
                truncated: false,
                budget: budget.clone(),
            },
            affected_processes: BoundedItems {
                items: vec![DiffImpactProcess {
                    name: "caller -> Demo".to_string(),
                    entry_symbol: "caller".to_string(),
                    path: "src/demo.rs".to_string(),
                    changed_steps: vec!["Demo".to_string()],
                }],
                truncated: false,
                budget: budget.clone(),
            },
            warnings: Vec::new(),
        })
    );
    assert_eq!(
        store.discover_routes(RouteMapRequest {
            route: Some("/demo".to_string()),
            path: None,
            budget: budget.clone(),
        }),
        Ok(RouteMapResult {
            routes: BoundedItems {
                items: vec![RouteMapItem {
                    method: "GET".to_string(),
                    route_path: "/demo".to_string(),
                    handler_symbol: "demo_handler".to_string(),
                    path: "src/demo.rs".to_string(),
                    line: 7,
                }],
                truncated: false,
                budget,
            },
        })
    );
}

struct FakeReadStore;

impl OntographReadQuery for FakeReadStore {
    fn search_symbols(&self, request: SymbolSearchRequest) -> QueryResult<SymbolSearchResult> {
        Ok(SymbolSearchResult {
            matches: BoundedItems::new(
                vec![SymbolSearchItem {
                    symbol: request.query,
                    kind: "struct".to_string(),
                    path: "src/demo.rs".to_string(),
                }],
                request.budget,
            ),
        })
    }

    fn search_repomap(&self, request: RepoMapRequest) -> QueryResult<RepoMapResult> {
        Ok(RepoMapResult {
            files: BoundedItems::new(
                vec![RepoMapFileItem {
                    path: "src/demo.rs".to_string(),
                    language: "rust".to_string(),
                    symbols: vec![SymbolSearchItem {
                        symbol: request.query,
                        kind: "struct".to_string(),
                        path: "src/demo.rs".to_string(),
                    }],
                }],
                request.budget,
            ),
        })
    }

    fn inspect_module(&self, request: ModuleContextRequest) -> QueryResult<ModuleContextResult> {
        Ok(ModuleContextResult {
            module: ModuleContextItem {
                path: request.path.clone(),
                language: Some("rust".to_string()),
                symbols: BoundedItems::new(
                    vec![SymbolSearchItem {
                        symbol: "Demo".to_string(),
                        kind: "struct".to_string(),
                        path: request.path,
                    }],
                    request.budget,
                ),
            },
        })
    }

    fn inspect_evidence(&self, request: EvidenceRequest) -> QueryResult<EvidenceResult> {
        Ok(EvidenceResult {
            evidence: BoundedItems::new(
                vec![
                    EvidenceItem {
                        target: "src/demo.rs:3".to_string(),
                        kind: "line".to_string(),
                        path: "src/demo.rs".to_string(),
                        start_line: Some(3),
                        end_line: Some(3),
                        snippet: request
                            .include_snippet
                            .then(|| "pub struct Demo;".to_string()),
                        symbol: None,
                        symbol_kind: None,
                    },
                    EvidenceItem {
                        target: "Demo".to_string(),
                        kind: "symbol".to_string(),
                        path: "src/demo.rs".to_string(),
                        start_line: Some(1),
                        end_line: Some(1),
                        snippet: request
                            .include_snippet
                            .then(|| "pub struct Demo;".to_string()),
                        symbol: Some("Demo".to_string()),
                        symbol_kind: Some("struct".to_string()),
                    },
                ],
                request.budget.clone(),
            ),
            unresolved: BoundedItems::new(
                vec![UnresolvedEvidenceItem {
                    target: "Missing".to_string(),
                    reason: "missing in fake store".to_string(),
                }],
                request.budget,
            ),
        })
    }

    fn inspect_shape(&self, request: RouteShapeRequest) -> QueryResult<RouteShapeResult> {
        Ok(RouteShapeResult {
            routes: BoundedItems::new(
                vec![RouteShapeItem {
                    method: "GET".to_string(),
                    route_path: request.route.unwrap_or_else(|| "/demo".to_string()),
                    handler_symbol: "demo_handler".to_string(),
                    path: request.path.unwrap_or_else(|| "src/demo.rs".to_string()),
                    line: 7,
                }],
                request.budget,
            ),
        })
    }

    fn impact_symbol(&self, request: SymbolImpactRequest) -> QueryResult<SymbolImpactResult> {
        Ok(SymbolImpactResult {
            upstream: BoundedItems::new(
                vec![ImpactItem {
                    symbol: "caller".to_string(),
                    path: "src/caller.rs".to_string(),
                    relationship: "CALLS".to_string(),
                    site_line: 2,
                }],
                request.budget.clone(),
            ),
            downstream: BoundedItems::new(Vec::new(), request.budget),
        })
    }

    fn impact_route(&self, request: RouteImpactRequest) -> QueryResult<RouteImpactResult> {
        Ok(RouteImpactResult {
            routes: BoundedItems::new(
                vec![RouteImpactItem {
                    method: "GET".to_string(),
                    route_path: request.route.unwrap_or_else(|| "/demo".to_string()),
                    handler_symbol: "demo_handler".to_string(),
                    path: request.path.unwrap_or_else(|| "src/demo.rs".to_string()),
                    line: 7,
                    upstream: BoundedItems::new(
                        vec![ImpactItem {
                            symbol: "caller".to_string(),
                            path: "src/caller.rs".to_string(),
                            relationship: "CALLS".to_string(),
                            site_line: 2,
                        }],
                        request.budget.clone(),
                    ),
                    downstream: BoundedItems::new(Vec::new(), request.budget.clone()),
                }],
                request.budget,
            ),
        })
    }

    fn impact_diff(&self, request: DiffImpactRequest) -> QueryResult<DiffImpactResult> {
        let changed_symbols = BoundedItems::new(
            vec![DiffImpactSymbol {
                symbol: "Demo".to_string(),
                kind: "struct".to_string(),
                path: "src/demo.rs".to_string(),
                line: 1,
            }],
            request.budget.clone(),
        );
        Ok(DiffImpactResult {
            summary: DiffImpactSummary {
                changed_files: 1,
                changed_symbols: 1,
                affected_processes: 1,
                risk_level: "medium".to_string(),
            },
            changed_files: BoundedItems::new(
                vec![DiffImpactFile {
                    path: "src/demo.rs".to_string(),
                    changed_symbols,
                }],
                request.budget.clone(),
            ),
            affected_processes: BoundedItems::new(
                vec![DiffImpactProcess {
                    name: "caller -> Demo".to_string(),
                    entry_symbol: "caller".to_string(),
                    path: "src/demo.rs".to_string(),
                    changed_steps: vec!["Demo".to_string()],
                }],
                request.budget,
            ),
            warnings: Vec::new(),
        })
    }

    fn discover_routes(&self, request: RouteMapRequest) -> QueryResult<RouteMapResult> {
        Ok(RouteMapResult {
            routes: BoundedItems::new(
                vec![RouteMapItem {
                    method: "GET".to_string(),
                    route_path: request.route.unwrap_or_else(|| "/demo".to_string()),
                    handler_symbol: "demo_handler".to_string(),
                    path: "src/demo.rs".to_string(),
                    line: 7,
                }],
                request.budget,
            ),
        })
    }

    fn graph_scc_inputs(&self, request: GraphSccInputRequest) -> QueryResult<GraphSccInputResult> {
        Ok(GraphSccInputResult {
            edges: BoundedItems::new(Vec::new(), request.budget),
        })
    }

    fn test_links(&self, request: TestLinkRequest) -> QueryResult<TestLinkResult> {
        Ok(TestLinkResult {
            links: BoundedItems::new(
                vec![TestLinkItem {
                    relationship_id: "relationship:test".to_string(),
                    test_symbol: "test_demo".to_string(),
                    test_path: "src/demo.rs".to_string(),
                    target_symbol: "Demo".to_string(),
                    target_path: "src/demo.rs".to_string(),
                }],
                request.budget,
            ),
        })
    }

    fn source_references(&self) -> SourceReferenceSet {
        SourceReferenceSet {
            paths: ["src/demo.rs".to_string()].into(),
            route_paths: ["/demo".to_string()].into(),
            symbols: ["Demo".to_string(), "demo_handler".to_string()].into(),
            symbols_by_path: [(
                "src/demo.rs".to_string(),
                ["Demo".to_string(), "demo_handler".to_string()].into(),
            )]
            .into(),
        }
    }
}
