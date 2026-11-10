use std::collections::BTreeMap;
use std::collections::BTreeSet;

use ontocode_extension_api::FunctionCallError;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use crate::audit_build::AuditAction;
use crate::query::GraphSccInputRequest;
use crate::query::OntographReadQuery;
use crate::query::QueryBudget;

const MAX_CYCLE_SCCS: usize = 50;
const MAX_CYCLE_NODES: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCycleFinding {
    pub nodes: Vec<String>,
    pub node_count: usize,
    pub edge_count: usize,
    pub truncated_nodes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub(crate) struct AuditCyclesResult {
    pub action: AuditAction,
    pub total_cycles: usize,
    pub findings: Vec<AuditCycleFinding>,
    pub truncated: bool,
    pub limits: Vec<String>,
    pub notes: Vec<String>,
}

pub(crate) fn run_cycles_audit(
    query: &impl OntographReadQuery,
    max_items: usize,
) -> Result<AuditCyclesResult, FunctionCallError> {
    let max_items = max_items.clamp(1, MAX_CYCLE_SCCS);
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
                "ontograph.audit failed to read graph cycle input facts".to_string(),
            )
        })?
        .edges;
    if edges.truncated {
        return Err(FunctionCallError::RespondToModel(
            "ontograph.audit cycle input facts are truncated; refusing to report partial graph cycles"
                .to_string(),
        ));
    }

    let graph = DirectedGraph::from_edges(edges.items);
    let mut findings = graph
        .strongly_connected_components()
        .into_iter()
        .filter_map(|mut nodes| {
            nodes.sort();
            let edge_count = graph.edge_count_inside(&nodes);
            if nodes.len() > 1 || edge_count > 0 {
                let truncated_nodes = nodes.len() > MAX_CYCLE_NODES;
                let node_count = nodes.len();
                nodes.truncate(MAX_CYCLE_NODES);
                Some(AuditCycleFinding {
                    nodes,
                    node_count,
                    edge_count,
                    truncated_nodes,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    findings.sort_by(|left, right| {
        left.nodes
            .first()
            .cmp(&right.nodes.first())
            .then(left.node_count.cmp(&right.node_count))
    });
    let total_cycles = findings.len();
    let truncated = findings.len() > max_items;
    findings.truncate(max_items);

    Ok(AuditCyclesResult {
        action: AuditAction::Cycles,
        total_cycles,
        findings,
        truncated,
        limits: vec![
            format!("max_items={max_items}"),
            format!("max_cycle_sccs={MAX_CYCLE_SCCS}"),
            format!("max_nodes_per_cycle={MAX_CYCLE_NODES}"),
        ],
        notes: vec![
            "This native slice reports SCCs from existing resolved graph CALLS facts only."
                .to_string(),
            "Unresolved call facts are intentionally not guessed into cycle edges.".to_string(),
            "Only SCCs with more than one node or a self-loop are reported.".to_string(),
        ],
    })
}

#[derive(Debug, Default)]
struct DirectedGraph {
    edges: BTreeMap<String, BTreeSet<String>>,
    nodes: BTreeSet<String>,
}

impl DirectedGraph {
    fn from_edges(edges: Vec<crate::query::GraphSccInputItem>) -> Self {
        let mut graph = Self::default();
        for edge in edges {
            graph.nodes.insert(edge.from_node_id.clone());
            graph.nodes.insert(edge.to_node_id.clone());
            graph
                .edges
                .entry(edge.from_node_id)
                .or_default()
                .insert(edge.to_node_id);
        }
        graph
    }

    fn strongly_connected_components(&self) -> Vec<Vec<String>> {
        let mut tarjan = Tarjan::new(self);
        for node in &self.nodes {
            if !tarjan.indices.contains_key(node) {
                tarjan.visit(node.clone());
            }
        }
        tarjan.components
    }

    fn edge_count_inside(&self, nodes: &[String]) -> usize {
        let node_set = nodes.iter().collect::<BTreeSet<_>>();
        nodes
            .iter()
            .filter_map(|node| self.edges.get(node))
            .map(|targets| {
                targets
                    .iter()
                    .filter(|target| node_set.contains(target))
                    .count()
            })
            .sum()
    }
}

struct Tarjan<'a> {
    graph: &'a DirectedGraph,
    index: usize,
    indices: BTreeMap<String, usize>,
    lowlinks: BTreeMap<String, usize>,
    stack: Vec<String>,
    on_stack: BTreeSet<String>,
    components: Vec<Vec<String>>,
}

impl<'a> Tarjan<'a> {
    fn new(graph: &'a DirectedGraph) -> Self {
        Self {
            graph,
            index: 0,
            indices: BTreeMap::new(),
            lowlinks: BTreeMap::new(),
            stack: Vec::new(),
            on_stack: BTreeSet::new(),
            components: Vec::new(),
        }
    }

    fn visit(&mut self, node: String) {
        self.indices.insert(node.clone(), self.index);
        self.lowlinks.insert(node.clone(), self.index);
        self.index += 1;
        self.stack.push(node.clone());
        self.on_stack.insert(node.clone());

        if let Some(targets) = self.graph.edges.get(&node) {
            for target in targets {
                if !self.indices.contains_key(target) {
                    self.visit(target.clone());
                    let target_lowlink = self.lowlinks[target];
                    let node_lowlink = self.lowlinks[&node];
                    self.lowlinks
                        .insert(node.clone(), node_lowlink.min(target_lowlink));
                } else if self.on_stack.contains(target) {
                    let target_index = self.indices[target];
                    let node_lowlink = self.lowlinks[&node];
                    self.lowlinks
                        .insert(node.clone(), node_lowlink.min(target_index));
                }
            }
        }

        if self.lowlinks[&node] == self.indices[&node] {
            let mut component = Vec::new();
            while let Some(member) = self.stack.pop() {
                self.on_stack.remove(&member);
                component.push(member.clone());
                if member == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}
