//! Score fusion for B2 and B3 over a shared candidate pool.
//!
//! `score = (1 - w)·mm(bm25) + w·mm(secondary)`, where `mm` is min-max over
//! the pool and is 0 everywhere when max = min. At `w = 0`, candidates with
//! zero BM25 are dropped. Ties break by raw BM25 descending, then chunk id, so
//! `w = 0` reproduces the B1 order exactly.

use crate::store::ChunkRecord;

use super::Hit;

/// Min-max scaling; all zeros when the values are empty or constant.
pub fn min_max(values: &[f64]) -> Vec<f64> {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if values.is_empty() || max <= min {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| (v - min) / (max - min)).collect()
}

/// Fuses `bm25` and `secondary` (both parallel to `pool`) and ranks the pool.
pub fn fuse(
    pool: &[usize],
    bm25: &[f64],
    secondary: &[f64],
    weight: f64,
    chunks: &[ChunkRecord],
    top_k: usize,
) -> Vec<Hit> {
    let mm_bm25 = min_max(bm25);
    let mm_secondary = min_max(secondary);
    let mut scored: Vec<(Hit, f64)> = pool
        .iter()
        .enumerate()
        .filter(|&(i, _)| weight != 0.0 || bm25[i] > 0.0)
        .map(|(i, &chunk)| {
            let score = (1.0 - weight) * mm_bm25[i] + weight * mm_secondary[i];
            (Hit { chunk, score }, bm25[i])
        })
        .collect();
    scored.sort_by(|(a, a_bm25), (b, b_bm25)| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| b_bm25.total_cmp(a_bm25))
            .then_with(|| chunks[a.chunk].id.cmp(&chunks[b.chunk].id))
    });
    scored.truncate(top_k);
    scored.into_iter().map(|(hit, _)| hit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SpanRecord;

    fn chunks(n: usize) -> Vec<ChunkRecord> {
        (0..n)
            .map(|i| ChunkRecord {
                id: format!("c{i}"),
                section_id: format!("s{i}"),
                ordinal: 0,
                path: "a.md".into(),
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

    fn ranked(hits: &[Hit]) -> Vec<(usize, f64)> {
        hits.iter().map(|h| (h.chunk, h.score)).collect()
    }

    #[test]
    fn min_max_is_zero_when_degenerate() {
        assert_eq!(min_max(&[2.0, 2.0, 2.0]), vec![0.0, 0.0, 0.0]);
        assert_eq!(min_max(&[]), Vec::<f64>::new());
        assert_eq!(min_max(&[1.0, 3.0, 2.0]), vec![0.0, 1.0, 0.5]);
    }

    #[test]
    fn fused_scores_match_hand_computation() {
        let chunks = chunks(3);
        let pool = [0, 1, 2];
        let (bm25, secondary) = ([4.0, 2.0, 0.0], [0.0, 0.5, 1.0]);
        // mm(bm25) = [1, 1/2, 0], mm(secondary) = [0, 1/2, 1].
        // w = 1/2: every score is (1/2)·mm_b + (1/2)·mm_s = 1/2; the tie
        // breaks by raw BM25 descending, so the order is 0, 1, 2.
        let hits = fuse(&pool, &bm25, &secondary, 0.5, &chunks, 10);
        assert_eq!(ranked(&hits), vec![(0, 0.5), (1, 0.5), (2, 0.5)]);
        // w = 0: score = mm_b = [1, 1/2]; chunk 2 has zero BM25 and is dropped.
        let hits = fuse(&pool, &bm25, &secondary, 0.0, &chunks, 10);
        assert_eq!(ranked(&hits), vec![(0, 1.0), (1, 0.5)]);
        // top_k truncates after ranking.
        let hits = fuse(&pool, &bm25, &secondary, 0.5, &chunks, 1);
        assert_eq!(ranked(&hits), vec![(0, 0.5)]);
        // A singleton pool is degenerate for min-max: score 0.
        let hits = fuse(&[1], &[3.0], &[2.0], 0.5, &chunks, 10);
        assert_eq!(ranked(&hits), vec![(1, 0.0)]);
    }
}
