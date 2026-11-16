//! Dense candidates and reciprocal rank fusion for the offline hybrid
//! experiment (`.memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md`). Nothing here
//! is reachable from [`super::Mode`], the CLI or MCP.
//!
//! Vectors are precomputed outside the binary; this module only validates
//! them and computes exact cosine top-k and rank fusion. Similarity sums in
//! index order in `f64`, so equal inputs give equal bits.

use std::collections::{BTreeMap, BTreeSet};

use crate::store::ChunkRecord;

use super::Hit;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DenseError {
    #[error("vector dimension must be positive")]
    ZeroDimension,
    #[error("vector matrix size overflows")]
    SizeOverflow,
    #[error("expected {expected} vector rows or values, found {found}")]
    Length { expected: usize, found: usize },
    #[error("vector {0} has a non-finite value")]
    NonFinite(usize),
    #[error("vector {0} has zero norm")]
    ZeroNorm(usize),
    #[error("query has dimension {found}, expected {expected}")]
    QueryDimension { expected: usize, found: usize },
    #[error("query vector has a non-finite value")]
    QueryNonFinite,
    #[error("query vector has zero norm")]
    QueryZeroNorm,
}

/// Unit-normalized chunk vectors, one row per chunk in generation order.
#[derive(Debug, Clone, PartialEq)]
pub struct Vectors {
    dim: usize,
    rows: Vec<f64>,
}

/// Explicit L2 normalization in `f64`; `None` for a zero norm.
fn normalized(values: &[f32]) -> Option<Vec<f64>> {
    let norm = values
        .iter()
        .map(|&v| f64::from(v) * f64::from(v))
        .sum::<f64>()
        .sqrt();
    (norm > 0.0 && norm.is_finite()).then(|| values.iter().map(|&v| f64::from(v) / norm).collect())
}

impl Vectors {
    /// Validates `rows × dim` values before allocating and normalizes each row.
    pub fn new(dim: usize, rows: usize, values: &[f32]) -> Result<Self, DenseError> {
        if dim == 0 {
            return Err(DenseError::ZeroDimension);
        }
        let expected = rows.checked_mul(dim).ok_or(DenseError::SizeOverflow)?;
        if values.len() != expected {
            return Err(DenseError::Length {
                expected,
                found: values.len(),
            });
        }
        let mut out = Vec::with_capacity(expected);
        for (row, chunk) in values.chunks_exact(dim).enumerate() {
            if chunk.iter().any(|v| !v.is_finite()) {
                return Err(DenseError::NonFinite(row));
            }
            out.extend(normalized(chunk).ok_or(DenseError::ZeroNorm(row))?);
        }
        Ok(Self { dim, rows: out })
    }

    pub fn len(&self) -> usize {
        self.rows.len() / self.dim
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Exact cosine top-k over in-scope chunks: cosine descending, ties by
    /// chunk id. Scope applies before selection, and every in-scope chunk is
    /// eligible whatever BM25 returned. `chunks` must be parallel to the rows.
    pub fn top_k(
        &self,
        query: &[f32],
        chunks: &[ChunkRecord],
        scope: Option<&str>,
        top_k: usize,
    ) -> Result<Vec<Hit>, DenseError> {
        if query.len() != self.dim {
            return Err(DenseError::QueryDimension {
                expected: self.dim,
                found: query.len(),
            });
        }
        if chunks.len() != self.len() {
            return Err(DenseError::Length {
                expected: self.len(),
                found: chunks.len(),
            });
        }
        if query.iter().any(|v| !v.is_finite()) {
            return Err(DenseError::QueryNonFinite);
        }
        let q = normalized(query).ok_or(DenseError::QueryZeroNorm)?;
        let mut hits: Vec<Hit> = self
            .rows
            .chunks_exact(self.dim)
            .enumerate()
            .filter(|(i, _)| scope.is_none_or(|s| chunks[*i].path.starts_with(s)))
            .map(|(chunk, row)| Hit {
                chunk,
                score: row.iter().zip(&q).map(|(a, b)| a * b).sum(),
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| chunks[a.chunk].id.cmp(&chunks[b.chunk].id))
        });
        hits.truncate(top_k);
        Ok(hits)
    }
}

/// Reciprocal rank fusion with one-based ranks:
/// `score(c) = [c in B]/(k + rank_B(c)) + [c in D]/(k + rank_D(c))`.
/// A chunk absent from a list contributes zero, and a repeated chunk counts at
/// its first rank only. Raw scores are ignored. The result sorts by fused
/// score descending, ties by chunk id, and is cut to `depth` so that section
/// projection sees at most `depth` chunks.
pub fn rrf(bm25: &[Hit], dense: &[Hit], k: u32, chunks: &[ChunkRecord], depth: usize) -> Vec<Hit> {
    let mut scores: BTreeMap<usize, f64> = BTreeMap::new();
    for list in [bm25, dense] {
        let mut seen = BTreeSet::new();
        for (i, hit) in list.iter().enumerate() {
            if seen.insert(hit.chunk) {
                *scores.entry(hit.chunk).or_insert(0.0) += 1.0 / (f64::from(k) + (i + 1) as f64);
            }
        }
    }
    let mut hits: Vec<Hit> = scores
        .into_iter()
        .map(|(chunk, score)| Hit { chunk, score })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| chunks[a.chunk].id.cmp(&chunks[b.chunk].id))
    });
    hits.truncate(depth);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SpanRecord;

    /// Chunk ids `c000`, `c001`, ...; even chunks in `a.md`, odd in `b.md`.
    fn chunks(n: usize) -> Vec<ChunkRecord> {
        (0..n)
            .map(|i| ChunkRecord {
                id: format!("c{i:03}"),
                section_id: format!("s{i}"),
                ordinal: 0,
                path: if i % 2 == 0 { "a.md" } else { "b.md" }.into(),
                heading_path: Vec::new(),
                span: SpanRecord {
                    byte_start: 0,
                    byte_end: 0,
                    line_start: 1,
                    line_end: 1,
                },
                sha256: String::new(),
                l0: String::new(),
                l0_truncated: false,
                l1: String::new(),
                l1_truncated: false,
            })
            .collect()
    }

    fn hits(ids: impl IntoIterator<Item = usize>) -> Vec<Hit> {
        ids.into_iter()
            .map(|chunk| Hit { chunk, score: 1.0 })
            .collect()
    }

    fn ids(hits: &[Hit]) -> Vec<usize> {
        hits.iter().map(|h| h.chunk).collect()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn rrf_k60_agreement_always_beats_a_single_list() {
        // B = chunks 0..=49; D = chunks 50..=98, then chunk 49 at rank 50.
        // Chunk 49, ranks 50 and 50: 1/110 + 1/110 = 0.018182.
        // Best single-list chunk, rank 1: 1/61 = 0.016393.
        let c = chunks(100);
        let fused = rrf(&hits(0..50), &hits((50..99).chain([49])), 60, &c, 50);
        assert_eq!(fused[0].chunk, 49);
        assert!(close(fused[0].score, 2.0 / 110.0));
        assert!(close(fused[1].score, 1.0 / 61.0));
        // The union holds 99 chunks; the fused list is cut to depth 50.
        assert_eq!(fused.len(), 50);
    }

    #[test]
    fn rrf_k10_agreement_wins_only_below_rank_12() {
        let c = chunks(40);
        // Chunk 30 at rank 11 in both lists: 2/21 = 0.095238, above the
        // single-list rank 1 score 1/11 = 0.090909 of chunks 0 and 10.
        let mut b = hits(0..10);
        b.extend(hits([30]));
        let mut d = hits(10..20);
        d.extend(hits([30]));
        let fused = rrf(&b, &d, 10, &c, 50);
        assert_eq!(fused[0].chunk, 30);
        assert!(close(fused[0].score, 2.0 / 21.0));
        // Chunk 31 at rank 12 in both: 2/22 = 1/11 exactly (halving is exact
        // in binary), tying chunks 0 and 11; ids order c000, c011, c031.
        let mut b = hits(0..11);
        b.extend(hits([31]));
        let mut d = hits(11..22);
        d.extend(hits([31]));
        let fused = rrf(&b, &d, 10, &c, 50);
        assert_eq!(ids(&fused[..3]), [0, 11, 31]);
        assert_eq!(fused[0].score, fused[2].score);
    }

    #[test]
    fn rrf_with_an_empty_leg_keeps_the_other_order() {
        let c = chunks(5);
        // BM25 returned nothing: dense-only chunks stay eligible.
        // Rank 3 scores 1/63 = 0.015873.
        let fused = rrf(&[], &hits([3, 1, 4]), 60, &c, 50);
        assert_eq!(ids(&fused), [3, 1, 4]);
        assert!(close(fused[2].score, 1.0 / 63.0));
        let fused = rrf(&hits([2, 0]), &[], 60, &c, 1);
        assert_eq!(ids(&fused), [2]);
        // A repeated chunk counts once per list, at its first rank: 1/61.
        let fused = rrf(&hits([2, 2]), &[], 60, &c, 50);
        assert_eq!(ids(&fused), [2]);
        assert!(close(fused[0].score, 1.0 / 61.0));
    }

    #[test]
    fn cosine_top_k_normalizes_ranks_and_scopes_before_selection() {
        let c = chunks(4);
        // Rows [3,4] -> [0.6,0.8]; [0,2] -> [0,1]; [1,0]; [2,0] -> [1,0].
        let v = Vectors::new(2, 4, &[3.0, 4.0, 0.0, 2.0, 1.0, 0.0, 2.0, 0.0]).unwrap();
        // Query [5,0] -> [1,0]: cosines 0.6, 0, 1, 1; chunks 2 and 3 tie by id.
        let top = v.top_k(&[5.0, 0.0], &c, None, 3).unwrap();
        assert_eq!(ids(&top), [2, 3, 0]);
        assert!(close(top[0].score, 1.0));
        assert!(close(top[2].score, 0.6));
        // Only b.md (chunks 1 and 3) is eligible: chunk 3 is not displaced by
        // the higher-ranked out-of-scope chunk 2.
        let top = v.top_k(&[5.0, 0.0], &c, Some("b.md"), 1).unwrap();
        assert_eq!(ids(&top), [3]);
        assert!(v.top_k(&[5.0, 0.0], &c, Some("z/"), 10).unwrap().is_empty());
        // f32::MAX squared is finite in f64, so a huge row normalizes to 1.
        let big = Vectors::new(1, 1, &[f32::MAX]).unwrap();
        assert!(close(
            big.top_k(&[1.0], &chunks(1), None, 1).unwrap()[0].score,
            1.0
        ));
    }

    #[test]
    fn malformed_vectors_and_queries_are_refused() {
        assert_eq!(Vectors::new(0, 1, &[]), Err(DenseError::ZeroDimension));
        assert_eq!(
            Vectors::new(usize::MAX, 2, &[]),
            Err(DenseError::SizeOverflow)
        );
        let length = Err(DenseError::Length {
            expected: 4,
            found: 3,
        });
        assert_eq!(Vectors::new(2, 2, &[1.0, 0.0, 1.0]), length);
        let nan = Vectors::new(2, 2, &[1.0, 0.0, f32::NAN, 1.0]);
        assert_eq!(nan, Err(DenseError::NonFinite(1)));
        let zero = Vectors::new(2, 2, &[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(zero, Err(DenseError::ZeroNorm(1)));
        let c = chunks(1);
        let v = Vectors::new(2, 1, &[1.0, 0.0]).unwrap();
        let dim = Err(DenseError::QueryDimension {
            expected: 2,
            found: 1,
        });
        assert_eq!(v.top_k(&[1.0], &c, None, 1), dim);
        let inf = v.top_k(&[f32::INFINITY, 0.0], &c, None, 1);
        assert_eq!(inf, Err(DenseError::QueryNonFinite));
        assert_eq!(
            v.top_k(&[0.0, 0.0], &c, None, 1),
            Err(DenseError::QueryZeroNorm)
        );
        // Chunk metadata that is not parallel to the rows is refused.
        let extra = Err(DenseError::Length {
            expected: 1,
            found: 2,
        });
        assert_eq!(v.top_k(&[1.0, 0.0], &chunks(2), None, 1), extra);
    }
}
