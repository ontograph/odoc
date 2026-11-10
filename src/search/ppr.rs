//! Personalized PageRank over chunk nodes (B2 graph signal).
//!
//! The graph is simple and undirected: every `LINKS_TO` or `NEXT` edge adds
//! one weight-1 neighbour pair; parallel edges and self-loops collapse.
//! Iteration: `r' = (1-d)·p + d·W·r + d·dangling(r)·p`, where `p` is the
//! normalised seed vector and `dangling(r)` is the mass on degree-0 nodes.
//! Starts at `r = p`; stops when the L1 change is below [`TOLERANCE`] or after
//! [`MAX_ITERATIONS`].

use std::collections::BTreeSet;

pub const DAMPING: f64 = 0.85;
pub const TOLERANCE: f64 = 1e-9;
pub const MAX_ITERATIONS: usize = 100;
/// Number of top B1 chunks used as seeds.
pub const SEEDS: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkGraph {
    adj: Vec<Vec<usize>>,
}

impl ChunkGraph {
    /// `n` nodes; `edges` are endpoint pairs in either direction.
    pub fn new(n: usize, edges: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let pairs: BTreeSet<(usize, usize)> = edges
            .into_iter()
            .filter(|(a, b)| a != b && *a < n && *b < n)
            .map(|(a, b)| (a.min(b), a.max(b)))
            .collect();
        let mut adj = vec![Vec::new(); n];
        for (a, b) in pairs {
            adj[a].push(b);
            adj[b].push(a);
        }
        for list in &mut adj {
            list.sort_unstable();
        }
        Self { adj }
    }

    /// PPR scores for `seeds` (node, positive weight); all zero without seeds.
    pub fn ppr(&self, seeds: &[(usize, f64)]) -> Vec<f64> {
        let n = self.adj.len();
        let total: f64 = seeds.iter().map(|(_, w)| w.max(0.0)).sum();
        if total <= 0.0 {
            return vec![0.0; n];
        }
        let p: Vec<(usize, f64)> = seeds
            .iter()
            .filter(|(v, w)| *v < n && *w > 0.0)
            .map(|&(v, w)| (v, w / total))
            .collect();
        let mut r = vec![0.0; n];
        for &(v, w) in &p {
            r[v] += w;
        }
        for _ in 0..MAX_ITERATIONS {
            let mut next = vec![0.0; n];
            let mut dangling = 0.0;
            for (u, neighbours) in self.adj.iter().enumerate() {
                if neighbours.is_empty() {
                    dangling += r[u];
                } else {
                    let share = DAMPING * r[u] / neighbours.len() as f64;
                    for &v in neighbours {
                        next[v] += share;
                    }
                }
            }
            for &(v, w) in &p {
                next[v] += (1.0 - DAMPING) * w + DAMPING * dangling * w;
            }
            let delta: f64 = next.iter().zip(&r).map(|(a, b)| (a - b).abs()).sum();
            r = next;
            if delta < TOLERANCE {
                break;
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_node_graph_with_dangling_node_matches_hand_computation() {
        // Path A(0)–B(1)–C(2); D(3) isolated (dangling). Seeds A and D, equal
        // weight: p = (1/2, 0, 0, 1/2), d = 0.85. Fixed point:
        //   w = (1-d)/2 + d·w/2           => w = (1-d)/(2-d)      = 0.13043478260869568
        //   x = (1-d)/2 + d·y/2 + d·w/2   => x = w + d·y/2        = 0.300235017626322
        //   y = d·(x/1 + z/1)             => y = d·w/(1-d²)       = 0.39952996474735597
        //   z = d·y/2                                             = 0.16980023501762628
        let graph = ChunkGraph::new(4, [(0, 1), (2, 1), (1, 0), (1, 1)]);
        let r = graph.ppr(&[(0, 2.0), (3, 2.0)]);
        let want = [
            0.300235017626322,
            0.39952996474735597,
            0.16980023501762628,
            0.13043478260869568,
        ];
        for (got, want) in r.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{got} vs {want}");
        }
        assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn no_seeds_gives_zero_scores() {
        let graph = ChunkGraph::new(2, [(0, 1)]);
        assert_eq!(graph.ppr(&[]), vec![0.0, 0.0]);
        assert_eq!(graph.ppr(&[(0, 0.0)]), vec![0.0, 0.0]);
    }
}
