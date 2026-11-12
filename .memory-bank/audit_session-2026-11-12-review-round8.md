# Review round 8: challenge of EVAL-2.1 work, consolidated plan, execution (2026-11-12)

Authority: user "review and challenge … consolidate it to one plan and proceed
it", then "proceed" (taken as approval of the recommended withdrawal, ADR
revision 15). Three GPT-6-Astra read-only reviewers: evaluation protocol,
simplification, service/performance.

## Findings

Done well: rules fixed before grading (rev 13-14); both judges graded every
pair; deterministic pool extension; the extension review caught a real P1.
Service (reviewer-reported): work-owned permits, complete-frame bounds, rooted
reads, one index slot per corpus.

Done worst:

1. Code and benchmark inputs changed without a plan card (now FIX-15).
2. Nothing committed; labels untracked; judge outputs only in git-ignored
   `tmp/judge/` (now `eval/judging/` with hashes).
3. The first extension code had a P1 its own tests could not see.
4. Option B: 507 pairs judged twice rescued 4 of 16 queries; a one-packet
   pilot would have shown about 1% positives first.
5. EVAL-3.2, OD-4.1 and OD-4.0 still demanded Holm decisions and promotion,
   contradicting rev 13 (cards now aligned).
6. Plan at 93% of the parser ceiling; `MEMORY.md` at its cap with closed
   history (both compacted).

Forgotten: judging provenance (now hashed); kappa on the final pool (now
computed); stale README/architecture schema statements (fixed); service
capacity has no measurement; `build_rooted` (`src/store.rs`) parses and
chunks every file before checking heading and chunk caps (late refusal, input
bytes bounded; source-confirmed).

Over-engineering: keep every evaluation safeguard (the exploratory track
still runs through freeze and once-only custody). Simplify records, not code.
One dead check to drop when next touching `run.rs`: the id/corpus loop in
`Bench::load` repeats `validate_queries`. Do not split `run.rs` mid-benchmark.

## Options and choices

| Question | Options | Chosen |
| --- | --- | --- |
| 12 unlabelled queries | withdraw and disclose; audit and pool again; stop for humans | withdraw (owner "proceed") |
| Judging evidence | commit outputs and hashes; manifest only; leave in `tmp/` | commit outputs, hash packets |
| Simplification | wait; archive records; cut eval code | archive records |
| Service | measure first; settle diagnostics ADR; freeze | measure first (PERF-2, not yet planned) |

Withdrawal reasoning: a query with no relevant section scores 0 for every
system, so over a fixed query set dropping it rescales paired means by
n/(n-12). The recomputed split changes partition membership, so per-split
means and signs are not invariant (review round 8 finding). Withdrawal raises
absolute scores, which reports must disclose. The
protocol reviewer rated it the highest selection-bias option and a human
rebuild the most defensible; no humans are available (owner, 2026-11-11).

## Executed

- ADR revision 15; FIX-15 card DONE; EVAL-2.1 write set widened; downstream
  cards aligned; DONE-card evidence of seven cards archived verbatim
  ([archive](audit_session-2026-11-12-done-card-archive.md), exact restore
  checked): plan 122,484 → about 120.6 KB, validator PASS.
- Withdrawal, split, pool, qrels and qrels-stage preparation: see
  [`eval/audit_session-preparation.md`](../eval/audit_session-preparation.md).
  Kappa 0.846 [0.820, 0.871]; test answerable 165; manifest `1e4835cd…`.
- `MEMORY.md` 100 → 37 lines and `project_plan-current.md` history moved
  verbatim to `archive/`.
- Validation: `cargo fmt --check` exit 0, `CARGO_BUILD_JOBS=4 cargo clippy
  --all-targets -- -D warnings` exit 0, `cargo test --all-targets` 281
  passed, 0 failed.

## Next

1. Owner pins the preparation manifest hash; close EVAL-2.1.
2. EVAL-3.0: two dev runs of B0/B1 from a clean committed checkout, byte
   compare.
3. Service track, independent of labels: a PERF-2 latency/RSS baseline and
   an early per-file cap check in `build_rooted` (FIX-16). Neither is in the
   plan yet; adding them needs room the plan does not have (about 10 KB left).

## Limits

Queries model-drafted and model-reviewed; drafter is judge `j2`; results
exploratory. Reviewer findings on the service were not rechecked except the
`build_rooted` cap order. OntoIndex did not cover the two dirty source files.
