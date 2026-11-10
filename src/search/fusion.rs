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

    #[test]
    fn min_max_is_zero_when_degenerate() {
        assert_eq!(min_max(&[2.0, 2.0, 2.0]), vec![0.0, 0.0, 0.0]);
        assert_eq!(min_max(&[]), Vec::<f64>::new());
        assert_eq!(min_max(&[1.0, 3.0, 2.0]), vec![0.0, 1.0, 0.5]);
    }
}
