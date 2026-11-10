//! Statistics for the pre-registered decisions, with std only.
//!
//! - [`SplitMix64`]: the project's seeded generator.
//! - [`paired_bootstrap`]: percentile CI of the mean paired difference,
//!   resampling queries within each corpus (stratum).
//! - [`randomization_test`]: two-sided paired sign-flip test.
//! - [`holm`], [`wilson`], [`weighted_kappa`], [`required_n`], [`decide`].

/// SplitMix64 (Steele, Lea, Flood 2014).
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (Lemire multiply-shift); `n` must be positive.
    pub fn below(&mut self, n: usize) -> usize {
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub mean: f64,
    pub lo: f64,
    pub hi: f64,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Type-7 (linear interpolation) quantile of sorted values.
fn quantile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let h = (sorted.len() - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

/// Percentile 95% CI of the pooled mean difference. `strata` holds the
/// per-query differences of each corpus; each is resampled with replacement
/// at its own size.
pub fn paired_bootstrap(strata: &[&[f64]], resamples: usize, seed: u64) -> Interval {
    let all: Vec<f64> = strata.iter().flat_map(|s| s.iter().copied()).collect();
    let mut rng = SplitMix64::new(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut sum = 0.0;
        for stratum in strata {
            for _ in 0..stratum.len() {
                sum += stratum[rng.below(stratum.len())];
            }
        }
        means.push(if all.is_empty() {
            0.0
        } else {
            sum / all.len() as f64
        });
    }
    means.sort_by(f64::total_cmp);
    Interval {
        mean: mean(&all),
        lo: quantile(&means, 0.025),
        hi: quantile(&means, 0.975),
    }
}

/// Two-sided paired randomization (sign-flip) p-value with the +1
/// correction: `(1 + #{|mean*| >= |mean|}) / (1 + resamples)`. Sign flips
/// stay within each query, so corpus stratification is preserved.
pub fn randomization_test(diffs: &[f64], resamples: usize, seed: u64) -> f64 {
    let observed = mean(diffs).abs();
    let mut rng = SplitMix64::new(seed);
    let mut extreme = 0usize;
    for _ in 0..resamples {
        let sum: f64 = diffs
            .iter()
            .map(|d| if rng.next_u64() >> 63 == 1 { -d } else { *d })
            .sum();
        let permuted = if diffs.is_empty() {
            0.0
        } else {
            (sum / diffs.len() as f64).abs()
        };
        if permuted >= observed - 1e-12 {
            extreme += 1;
        }
    }
    (1 + extreme) as f64 / (1 + resamples) as f64
}

/// Holm step-down adjusted p-values, in input order.
pub fn holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| p[a].total_cmp(&p[b]).then(a.cmp(&b)));
    let mut adjusted = vec![0.0; m];
    let mut running: f64 = 0.0;
    for (rank, &i) in order.iter().enumerate() {
        running = running.max(((m - rank) as f64 * p[i]).min(1.0));
        adjusted[i] = running;
    }
    adjusted
}

/// Wilson score interval for `k` successes out of `n` at the given `z`.
pub fn wilson(k: usize, n: usize, z: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let n = n as f64;
    let p = k as f64 / n;
    let z2 = z * z;
    let denom = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denom;
    let half = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denom;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

/// Quadratic-weighted Cohen's kappa for two raters on grades `0..categories`.
/// Returns `None` when undefined (no pairs, or no expected disagreement).
pub fn weighted_kappa(a: &[u8], b: &[u8], categories: usize) -> Option<f64> {
    if a.len() != b.len() || a.is_empty() || categories < 2 {
        return None;
    }
    let k = categories;
    let mut observed = vec![vec![0.0; k]; k];
    for (&x, &y) in a.iter().zip(b) {
        let (x, y) = (usize::from(x), usize::from(y));
        if x >= k || y >= k {
            return None;
        }
        observed[x][y] += 1.0;
    }
    let n = a.len() as f64;
    let rows: Vec<f64> = observed.iter().map(|r| r.iter().sum()).collect();
    let cols: Vec<f64> = (0..k)
        .map(|j| observed.iter().map(|r| r[j]).sum())
        .collect();
    let weight = |i: usize, j: usize| ((i as f64 - j as f64) / (k - 1) as f64).powi(2);
    let (mut num, mut den) = (0.0, 0.0);
    for i in 0..k {
        for j in 0..k {
            num += weight(i, j) * observed[i][j];
            den += weight(i, j) * rows[i] * cols[j] / n;
        }
    }
    (den > 0.0).then(|| 1.0 - num / den)
}

/// Inverse standard normal CDF (Acklam's rational approximation, relative
/// error below 1.15e-9).
pub fn z_quantile(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    const LOW: f64 = 0.02425;
    if !(0.0..=1.0).contains(&p) || p.is_nan() {
        return f64::NAN;
    }
    if p == 0.0 {
        return f64::NEG_INFINITY;
    }
    if p == 1.0 {
        return f64::INFINITY;
    }
    let tail = |q: f64| {
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    if p < LOW {
        tail((-2.0 * p.ln()).sqrt())
    } else if p > 1.0 - LOW {
        -tail((-2.0 * (1.0 - p).ln()).sqrt())
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

/// Required answerable queries: `((z(1 - alpha/2) + z(power)) * sd / effect)^2`,
/// rounded up. The ADR fixes alpha 0.025 (first Holm step), power 0.8 and
/// effect 0.05.
pub fn required_n(sd_diff: f64, effect: f64, alpha: f64, power: f64) -> usize {
    let z = z_quantile(1.0 - alpha / 2.0) + z_quantile(power);
    ((z * sd_diff / effect).powi(2)).ceil() as usize
}

/// Sample standard deviation (n - 1).
pub fn sample_sd(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let var = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    var.sqrt()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Pass,
    Negative,
    Inconclusive,
}

/// ADR decision rule for a confirmatory arm.
pub fn decide(ci: Interval, holm_p: f64, alpha: f64, min_effect: f64) -> Decision {
    if ci.mean > 0.0 && ci.lo > 0.0 && holm_p < alpha {
        Decision::Pass
    } else if ci.hi < min_effect {
        Decision::Negative
    } else {
        Decision::Inconclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    #[test]
    fn splitmix64_reference_sequence() {
        // Reference outputs for seed 0 from the published SplitMix64 algorithm.
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        for _ in 0..100 {
            let x = a.below(10);
            assert!(x < 10);
            assert_eq!(x, b.below(10));
        }
    }

    #[test]
    fn identical_systems_give_p_one_and_ci_containing_zero() {
        let diffs = vec![0.0; 40];
        assert_eq!(randomization_test(&diffs, 10_000, 1), 1.0);
        let ci = paired_bootstrap(&[&diffs], 10_000, 1);
        assert!(ci.lo <= 0.0 && ci.hi >= 0.0);
    }

    #[test]
    fn constant_shift_gives_degenerate_ci() {
        let a = vec![0.1; 30];
        let b = vec![0.1; 20];
        let ci = paired_bootstrap(&[&a, &b], 2_000, 3);
        close(ci.mean, 0.1, 1e-12);
        close(ci.lo, 0.1, 1e-12);
        close(ci.hi, 0.1, 1e-12);
        // All sign flips of a constant are at most as extreme: only the
        // all-positive flip ties, expected count 10000 / 2^50 ~ 0.
        let p = randomization_test(&[a, b].concat(), 10_000, 3);
        close(p, 1.0 / 10_001.0, 1e-12);
    }

    #[test]
    fn outputs_are_identical_for_a_fixed_seed() {
        let diffs: Vec<f64> = (0..50)
            .map(|i| ((i * 37) % 11) as f64 / 10.0 - 0.4)
            .collect();
        let (a, b) = diffs.split_at(20);
        assert_eq!(
            paired_bootstrap(&[a, b], 5_000, 42),
            paired_bootstrap(&[a, b], 5_000, 42)
        );
        assert_eq!(
            randomization_test(&diffs, 5_000, 42),
            randomization_test(&diffs, 5_000, 42)
        );
    }

    #[test]
    fn discrimination_oracle_beats_shuffle() {
        // Oracle wins by ~0.3 on 60 synthetic queries with noise.
        let mut rng = SplitMix64::new(9);
        let diffs: Vec<f64> = (0..60)
            .map(|_| 0.3 + (rng.below(1000) as f64 / 1000.0 - 0.5) * 0.4)
            .collect();
        assert!(randomization_test(&diffs, 10_000, 11) < 0.01);
        let ci = paired_bootstrap(&[&diffs], 10_000, 11);
        assert!(ci.lo > 0.2 && ci.hi < 0.4, "{ci:?}");
    }

    #[test]
    fn bootstrap_ci_matches_normal_approximation() {
        // Monte Carlo tolerance: diffs with mean 0.05 and sd ~0.1 over n = 100.
        // Normal CI half-width = 1.96 * sd / sqrt(n); bootstrap agrees within 0.005.
        let diffs: Vec<f64> = (0..100)
            .map(|i| 0.05 + if i % 2 == 0 { 0.1 } else { -0.1 })
            .collect();
        let sd = sample_sd(&diffs);
        let half = 1.959_963_984_540_054 * sd * (99.0f64 / 100.0).sqrt() / 10.0;
        let ci = paired_bootstrap(&[&diffs], 10_000, 5);
        close(ci.mean, 0.05, 1e-12);
        close(ci.lo, 0.05 - half, 0.005);
        close(ci.hi, 0.05 + half, 0.005);
    }

    #[test]
    fn holm_textbook_example() {
        // p = (0.01, 0.04, 0.03, 0.005), m = 4. Sorted: 0.005·4 = 0.02,
        // 0.01·3 = 0.03, 0.03·2 = 0.06, 0.04·1 = 0.04 -> monotone max 0.06.
        let adjusted = holm(&[0.01, 0.04, 0.03, 0.005]);
        let want = [0.03, 0.06, 0.06, 0.02];
        for (a, w) in adjusted.iter().zip(want) {
            close(*a, w, 1e-12);
        }
        assert_eq!(holm(&[0.6, 0.9]), vec![1.0, 1.0]);
    }

    #[test]
    fn wilson_published_values() {
        // z = 1.959963984540054. Wilson score 95% intervals, matching the values
        // tabulated by Newcombe (1998, Stat Med 17:857) to 4 dp:
        // 0/10 -> [0, 0.2775], 5/10 -> [0.2366, 0.7634], 10/10 -> [0.7225, 1].
        // Full-precision values from the closed form (p + z²/2n ± z·sqrt(p(1-p)/n + z²/4n²)) / (1 + z²/n).
        let z = 1.959_963_984_540_054;
        let (lo, hi) = wilson(0, 10, z);
        close(lo, 0.0, 1e-9);
        close(hi, 0.277_532_799_862_889_15, 1e-9);
        let (lo, hi) = wilson(5, 10, z);
        close(lo, 0.236_593_090_512_564_05, 1e-9);
        close(hi, 0.763_406_909_487_436, 1e-9);
        let (lo, hi) = wilson(10, 10, z);
        close(lo, 0.722_467_200_137_110_9, 1e-9);
        close(hi, 1.0, 1e-9);
    }

    #[test]
    fn quadratic_weighted_kappa_worked_example() {
        // Rater pairs give O = [[3,0,0],[1,2,1],[0,1,2]], rows (3,4,3), cols (4,3,3),
        // n = 10, w(i,j) = (i-j)^2 / 4.
        // Observed: 0.25·(1 + 1 + 1) = 0.75.
        // Expected: 0.25·(0.9 + 1.6 + 1.2 + 0.9) + 1·(0.9 + 1.2) = 1.15 + 2.1 = 3.25.
        // kappa = 1 - 0.75 / 3.25 = 10/13 = 0.7692307692307692.
        let a = [0, 1, 2, 2, 1, 0, 2, 1, 1, 0];
        let b = [0, 1, 2, 1, 1, 0, 2, 2, 0, 0];
        close(weighted_kappa(&a, &b, 3).unwrap(), 10.0 / 13.0, 1e-12);
        close(weighted_kappa(&a, &a, 3).unwrap(), 1.0, 1e-12);
        assert_eq!(weighted_kappa(&[1, 1], &[1, 1], 3), None);
        assert_eq!(weighted_kappa(&[0], &[3], 3), None);
    }

    #[test]
    fn z_quantiles_and_power() {
        close(z_quantile(0.975), 1.959_963_984_540_054, 1e-8);
        close(z_quantile(0.9875), 2.241_402_727_604_946_4, 1e-8);
        close(z_quantile(0.8), 0.841_621_233_572_914_4, 1e-8);
        close(z_quantile(0.5), 0.0, 1e-15);
        close(z_quantile(1e-10), -6.361_340_902_404_056, 1e-7);
        // n = ((2.2414027276 + 0.8416212336) · 0.2 / 0.05)^2
        //   = (3.0830239612 · 4)^2 = 12.3320958448^2 = 152.0805879 -> 153.
        assert_eq!(required_n(0.2, 0.05, 0.025, 0.8), 153);
        // sd 0.15: (3.0830239612 · 3)^2 = 85.5453307 -> 86.
        assert_eq!(required_n(0.15, 0.05, 0.025, 0.8), 86);
        close(
            sample_sd(&[1.0, 2.0, 3.0, 4.0]),
            (5.0f64 / 3.0).sqrt(),
            1e-12,
        );
    }

    #[test]
    fn decision_rule() {
        let ci = |mean, lo, hi| Interval { mean, lo, hi };
        assert_eq!(
            decide(ci(0.06, 0.02, 0.10), 0.01, 0.05, 0.05),
            Decision::Pass
        );
        // Significant negative difference is never a pass.
        assert_eq!(
            decide(ci(-0.08, -0.12, -0.04), 0.001, 0.05, 0.05),
            Decision::Negative
        );
        // Wide CI: inconclusive.
        assert_eq!(
            decide(ci(0.03, -0.05, 0.11), 0.2, 0.05, 0.05),
            Decision::Inconclusive
        );
        // Positive CI but Holm p too large: not a pass.
        assert_eq!(
            decide(ci(0.06, 0.01, 0.11), 0.06, 0.05, 0.05),
            Decision::Inconclusive
        );
        // Tight CI below the minimum effect: negative even if positive.
        assert_eq!(
            decide(ci(0.01, 0.005, 0.02), 0.2, 0.05, 0.05),
            Decision::Negative
        );
    }
}
