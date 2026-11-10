//! Per-query ranking metrics over section ids.
//!
//! Gains are the qrels grades (0, 1, 2) used linearly with a `log2(rank + 1)`
//! discount. "Relevant" means grade >= 1. A query without any relevant
//! section is rejected with [`MetricError::NoRelevant`]: it is not answerable
//! and must not be averaged into ranking metrics.

use std::collections::{BTreeMap, HashSet};

/// Judgements of one query: section id → grade. Absent ids are unjudged.
pub type Qrels = BTreeMap<String, u8>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MetricError {
    #[error("query has no relevant section")]
    NoRelevant,
    #[error("ranking repeats section {0}")]
    Duplicate(String),
}

fn check(qrels: &Qrels, ranking: &[&str]) -> Result<(), MetricError> {
    if !qrels.values().any(|&g| g >= 1) {
        return Err(MetricError::NoRelevant);
    }
    let mut seen = HashSet::new();
    for id in ranking {
        if !seen.insert(*id) {
            return Err(MetricError::Duplicate((*id).to_string()));
        }
    }
    Ok(())
}

fn dcg(gains: impl Iterator<Item = u8>) -> f64 {
    gains
        .enumerate()
        .map(|(i, g)| f64::from(g) / ((i + 2) as f64).log2())
        .sum()
}

fn ideal(qrels: &Qrels, k: usize) -> f64 {
    let mut grades: Vec<u8> = qrels.values().copied().collect();
    grades.sort_unstable_by(|a, b| b.cmp(a));
    dcg(grades.into_iter().take(k))
}

fn grade(qrels: &Qrels, id: &str) -> u8 {
    qrels.get(id).copied().unwrap_or(0)
}

/// nDCG@k; unjudged sections count as grade 0.
pub fn ndcg_at(qrels: &Qrels, ranking: &[&str], k: usize) -> Result<f64, MetricError> {
    check(qrels, ranking)?;
    let gains = ranking.iter().take(k).map(|id| grade(qrels, id));
    Ok(dcg(gains) / ideal(qrels, k))
}

/// Condensed nDCG'@k: unjudged sections are removed before cutting at k.
pub fn condensed_ndcg_at(qrels: &Qrels, ranking: &[&str], k: usize) -> Result<f64, MetricError> {
    check(qrels, ranking)?;
    let judged: Vec<&str> = ranking
        .iter()
        .copied()
        .filter(|id| qrels.contains_key(*id))
        .collect();
    ndcg_at(qrels, &judged, k)
}

/// Fraction of relevant sections retrieved in the top k.
pub fn recall_at(qrels: &Qrels, ranking: &[&str], k: usize) -> Result<f64, MetricError> {
    check(qrels, ranking)?;
    let relevant = qrels.values().filter(|&&g| g >= 1).count();
    let found = ranking
        .iter()
        .take(k)
        .filter(|id| grade(qrels, id) >= 1)
        .count();
    Ok(found as f64 / relevant as f64)
}

/// Reciprocal rank of the first relevant section in the top k, else 0.
pub fn mrr_at(qrels: &Qrels, ranking: &[&str], k: usize) -> Result<f64, MetricError> {
    check(qrels, ranking)?;
    Ok(ranking
        .iter()
        .take(k)
        .position(|id| grade(qrels, id) >= 1)
        .map_or(0.0, |i| 1.0 / (i + 1) as f64))
}

/// Fraction of the top k that is judged (pool completeness).
pub fn judged_at(qrels: &Qrels, ranking: &[&str], k: usize) -> f64 {
    let top: Vec<&&str> = ranking.iter().take(k).collect();
    if top.is_empty() {
        return 1.0;
    }
    top.iter().filter(|id| qrels.contains_key(***id)).count() as f64 / top.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qrels() -> Qrels {
        [("s1", 2), ("s2", 1), ("s3", 0), ("s4", 2)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    const RANKING: [&str; 4] = ["s3", "s1", "x", "s2"];

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} vs {b}");
    }

    #[test]
    fn ndcg_worked_example() {
        // DCG  = 0/log2 2 + 2/log2 3 + 0/log2 4 + 1/log2 5
        //      = 0 + 1.2618595071429148 + 0 + 0.43067655807339306 = 1.6925360652163079
        // IDCG = 2/1 + 2/log2 3 + 1/log2 4 + 0/log2 5
        //      = 2 + 1.2618595071429148 + 0.5 = 3.761859507142915
        // nDCG@10 = 1.6925360652163079 / 3.761859507142915 = 0.4499200626718163
        close(ndcg_at(&qrels(), &RANKING, 10).unwrap(), 0.4499200626718163);
        // nDCG@2: DCG = 1.2618595071429148, IDCG = 2 + 1.2618595071429148
        //       = 0.38685280723454163
        close(ndcg_at(&qrels(), &RANKING, 2).unwrap(), 0.38685280723454163);
    }

    #[test]
    fn condensed_ndcg_drops_unjudged() {
        // Condensed list s3, s1, s2: DCG = 0 + 2/log2 3 + 1/log2 4 = 1.7618595071429148
        // / IDCG 3.761859507142915 = 0.4683480347412084
        close(
            condensed_ndcg_at(&qrels(), &RANKING, 10).unwrap(),
            0.4683480347412084,
        );
    }

    #[test]
    fn recall_mrr_and_judged() {
        // Relevant: s1, s2, s4. Top 10 has s1, s2 -> 2/3. Top 2 has s1 -> 1/3.
        close(recall_at(&qrels(), &RANKING, 10).unwrap(), 2.0 / 3.0);
        close(recall_at(&qrels(), &RANKING, 2).unwrap(), 1.0 / 3.0);
        // First relevant at rank 2.
        close(mrr_at(&qrels(), &RANKING, 10).unwrap(), 0.5);
        close(mrr_at(&qrels(), &RANKING, 1).unwrap(), 0.0);
        // 3 of 4 judged ("x" is not).
        close(judged_at(&qrels(), &RANKING, 10), 0.75);
        close(judged_at(&qrels(), &[], 10), 1.0);
    }

    #[test]
    fn unanswerable_queries_and_duplicates_are_rejected() {
        let none: Qrels = [("s3".to_string(), 0)].into_iter().collect();
        assert_eq!(ndcg_at(&none, &["s3"], 10), Err(MetricError::NoRelevant));
        assert_eq!(recall_at(&none, &[], 10), Err(MetricError::NoRelevant));
        assert_eq!(mrr_at(&Qrels::new(), &[], 10), Err(MetricError::NoRelevant));
        assert_eq!(
            ndcg_at(&qrels(), &["s1", "s1"], 10),
            Err(MetricError::Duplicate("s1".into()))
        );
    }

    #[test]
    fn perfect_and_empty_rankings() {
        close(ndcg_at(&qrels(), &["s1", "s4", "s2"], 10).unwrap(), 1.0);
        close(ndcg_at(&qrels(), &[], 10).unwrap(), 0.0);
    }
}
