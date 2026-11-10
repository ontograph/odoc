//! Breadth-first budgeted context assembly (a feature; no quality claim).
//!
//! Pass 1 gives every result its L0 in rank order while it fits. Pass 2
//! upgrades results in rank order to L1, and pass 3 to L2 (the chunk text).
//! An upgrade replaces the previous tier and is applied only if the total
//! still fits; otherwise the result keeps its previous tier. Tiers are never
//! truncated. Size is the sum of the bytes of the chosen tier texts.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TierLevel {
    L0,
    L1,
    L2,
}

/// The three tiers of one ranked result.
#[derive(Debug, Clone, Copy)]
pub struct TierTexts<'a> {
    pub l0: &'a str,
    pub l1: &'a str,
    pub l2: &'a str,
}

impl<'a> TierTexts<'a> {
    fn get(&self, level: TierLevel) -> &'a str {
        match level {
            TierLevel::L0 => self.l0,
            TierLevel::L1 => self.l1,
            TierLevel::L2 => self.l2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assembled {
    /// One entry per input result, in rank order; `None` if even L0 didn't fit.
    pub tiers: Vec<Option<TierLevel>>,
    pub total_bytes: usize,
    /// True when some result is below L2.
    pub truncated: bool,
}

pub fn assemble(results: &[TierTexts<'_>], budget: usize) -> Assembled {
    let mut tiers: Vec<Option<TierLevel>> = vec![None; results.len()];
    let mut total = 0usize;
    for level in [TierLevel::L0, TierLevel::L1, TierLevel::L2] {
        for (result, tier) in results.iter().zip(tiers.iter_mut()) {
            let current = tier.map_or(0, |t| result.get(t).len());
            if level != TierLevel::L0 && tier.is_none() {
                continue;
            }
            let next = total - current + result.get(level).len();
            if next <= budget {
                total = next;
                *tier = Some(level);
            }
        }
    }
    Assembled {
        truncated: tiers.iter().any(|t| *t != Some(TierLevel::L2)),
        tiers,
        total_bytes: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn texts(sizes: &[(usize, usize, usize)]) -> Vec<(String, String, String)> {
        sizes
            .iter()
            .map(|&(a, b, c)| ("0".repeat(a), "1".repeat(b), "2".repeat(c)))
            .collect()
    }

    fn view(owned: &[(String, String, String)]) -> Vec<TierTexts<'_>> {
        owned
            .iter()
            .map(|(l0, l1, l2)| TierTexts { l0, l1, l2 })
            .collect()
    }

    #[test]
    fn breadth_first_then_upgrades_in_rank_order() {
        let owned = texts(&[(10, 30, 100), (10, 30, 100), (10, 30, 100)]);
        let r = view(&owned);
        // L0 for all: 30. L1 for #1: 50, #2: 70, #3: 90. L2 #1 would be 160 > 100.
        let got = assemble(&r, 100);
        assert_eq!(got.tiers, vec![Some(TierLevel::L1); 3]);
        assert_eq!(got.total_bytes, 90);
        assert!(got.truncated);
        // Budget 25: only two L0s fit; the third gets nothing.
        let got = assemble(&r, 25);
        assert_eq!(
            got.tiers,
            vec![Some(TierLevel::L0), Some(TierLevel::L0), None]
        );
        // A non-fitting upgrade keeps the prior tier but later ones still apply.
        let owned = texts(&[(10, 500, 600), (10, 20, 30)]);
        let got = assemble(&view(&owned), 60);
        assert_eq!(got.tiers, vec![Some(TierLevel::L0), Some(TierLevel::L2)]);
        assert_eq!(got.total_bytes, 40);
    }

    #[test]
    fn generated_cases_respect_budget_and_are_maximal() {
        // SplitMix64 with a fixed seed for reproducible cases.
        let mut state: u64 = 0x5eed;
        let mut next = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        for _ in 0..500 {
            let n = (next() % 8) as usize;
            let sizes: Vec<(usize, usize, usize)> = (0..n)
                .map(|_| {
                    let a = (next() % 50) as usize;
                    let b = a + (next() % 200) as usize;
                    (a, b, b + (next() % 1000) as usize)
                })
                .collect();
            let owned = texts(&sizes);
            let r = view(&owned);
            let budget = (next() % 2000) as usize;
            let got = assemble(&r, budget);
            assert!(got.total_bytes <= budget);
            let sum: usize = got
                .tiers
                .iter()
                .zip(&r)
                .filter_map(|(t, x)| t.map(|t| x.get(t).len()))
                .sum();
            assert_eq!(sum, got.total_bytes);
            assert_eq!(assemble(&r, budget), got);
            // Maximal: tier sizes are non-decreasing here, so the total only
            // grows and no result could still step up one tier at the end.
            for (t, x) in got.tiers.iter().zip(&r) {
                let (cur, up) = match t {
                    None => (0, Some(TierLevel::L0)),
                    Some(TierLevel::L0) => (x.l0.len(), Some(TierLevel::L1)),
                    Some(TierLevel::L1) => (x.l1.len(), Some(TierLevel::L2)),
                    Some(TierLevel::L2) => (x.l2.len(), None),
                };
                if let Some(up) = up {
                    assert!(got.total_bytes - cur + x.get(up).len() > budget);
                }
            }
        }
    }

    #[test]
    fn tiers_are_monotone_by_rank_for_equal_sizes() {
        let owned = texts(&[(10, 40, 90); 5]);
        let r = view(&owned);
        for budget in 0..600 {
            let got = assemble(&r, budget);
            assert!(got.tiers.windows(2).all(|w| w[0] >= w[1]), "{budget}");
        }
    }
}
