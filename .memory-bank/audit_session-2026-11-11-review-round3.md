# Review Round 3: Challenge, Simplify, Consolidate (2026-11-11)

Authority: user request "review and challenge what has been planned and done
... consolidate to one plan and proceed it; use ontoindex; use GPT sub-agents".
Reviewers: three independent read-only GPT-6-Astra explorers (product path,
evaluation subsystem, planning/process). The main session re-checked every
finding adopted below against current source before accepting it.

## Done Well (keep)

- Product: `App` is shared by CLI and all five MCP tools. Admission permits are
  owned by the actual blocking work, and requests share an immutable
  `Arc<SearchIndex>` snapshot (`src/mcp.rs` `blocking`). Rooted `openat2` reads
  never follow symlinks (`src/corpus.rs`). Citation membership is checked before
  any document read. Publication is atomic, and outbound MCP frames are bounded.
- Evaluation: the metrics and statistics have hand-computed oracles. The dev
  capture never opens held-out payloads, and the held-out run reserves its single
  attempt before any test read; failures consume it.
- Process: the ADR is the authority, each task has a write set and runnable
  evidence, and model output never stands in for a human annotator.

## Worst (verified defects)

| # | Defect | Evidence | Severity |
| - | ------ | -------- | -------- |
| D1 | Ranking files are produced without any byte budget, but scoring reads them back under a combined 16 MiB cap. A valid benchmark (2,000 queries x 10 lines x up to ~4.4 KB each) can exceed it, so the single held-out attempt is consumed and then scoring fails. | `src/eval/run.rs` `run_reserved` ranking loop; `score_captured` `budget = MAX_INPUT_BYTES` | P1 (burns the test split) |
| D2 | (Reviewer claim, **rejected**) `App::search` ignores `Assembled.truncated`. | Round 2 already rejected this: ADR §8 keeps the response `truncated` flag for omitted results, and each result carries its own `tier`. Reconsidering it without a new argument would flip-flop. | — |
| D3 | `ChunkBoundary` is reported whenever no chunk of the section shares a query term. That is a vocabulary mismatch, not evidence of a chunk boundary. | `src/eval/sweep.rs` `attribute_miss` | P2 (diagnostics overclaim) |
| D4 | `related` collects every neighbour before capping, and `status` clones the full diagnostic arrays before capping. The work is unbounded until after allocation. | `src/cli.rs` `related`, `status` | P3 |
| D5 | The `_scratch` parameter of `check_pool`/`check_benchmark` is unused plumbing. | `src/eval/gates.rs` | P3 (dead code) |

D2 is kept rejected, in line with round 2 (`audit_session-2026-11-11-review-simplify-consolidate.md`, item 2).

## Overengineering / Simplify

- Evaluation (about 7,100 lines, of which about 31% is tests) has never touched
  real data. Preparation, capture, registration and reservation serve distinct
  ADR trust transitions, so do not merge them. They are kept; only the duplicate
  re-checks across `check_registration` / `check_held_out_code` /
  `held_out_preflight` / `check_frozen` may later be passed through as already
  verified identity. That is deferred: it touches the safety-critical held-out
  path and has no user value until real data exists.
- `kappa_from_counts` duplicates `stats::weighted_kappa`, but it serves as the
  cluster-matrix form needed by the bootstrap. Kept.
- Process: the plan is 102 KB, status is copied into four files and has drifted
  (`MEMORY.md` 34/43, `project_pending-tasks.md` HTTP-1 still pending), and the
  tree is uncommitted. Fix: reconcile the status copies, and present one
  decision packet instead of repeated blocked replies.

## Forgotten

- `Registry::discover` and `with_permits` are reachable only from tests. The
  binary still serves one root over stdio. Multi-folder serving needs HTTP-2,
  which is owner-gated by HTTP-0.
- There is no dev signal without humans. A synthetic non-confirmatory smoke
  benchmark was proposed. It is deferred: it is new scope, and the ADR forbids
  presenting it as evidence. The owner can request it explicitly.

## Options Considered

| Option | What | Robustness | Perf/value | Pick |
| ------ | ---- | ---------- | ---------- | ---- |
| A | Fix D1-D5 only, reconcile status, decision packet | High: small, test-backed | Removes the one P1 that would waste the test split | **Yes** |
| B | A plus the eval re-check consolidation | Medium: touches held-out admission | Fewer lines; no user value yet | Deferred |
| C | Replace custody with an operator workflow | Low: weakens the accepted ADR | Largest deletion | Rejected |
| D | Self-authorize loopback HTTP | Violates AGENTS.md (needs an ADR amendment) | High user value | Rejected; offered as decision D1 |
| E | Incremental index / freshness TTL | Weakens ADR §9 current-byte freshness | Highest throughput | Rejected without an ADR change |

## Consolidated Plan

1. **FIX-3 (eval):** D1 - bound ranking output to scoring's budget during
   generation, and refuse before reservation when the worst case cannot fit; D3 -
   report a vocabulary miss as `lexical_miss` rather than `chunk_boundary`
   (ADR stage set unchanged); D5 - drop unused scratch plumbing.
2. **FIX-4 (product):** D4 - cap before collecting/cloning in `related`/`status`.
3. Status reconciliation across `MEMORY.md`, `project_plan-current.md`,
   `project_pending-tasks.md` and `project_architecture.md`.
4. Owner decision packet (D1 HTTP-0, D2 section-ID cap, D3 evaluation inputs,
   D4 baseline commit).

## Outcomes

- **FIX-3 DONE** (strict complete-local, revision `1ed6fc6c…12e4`).
  - `run_file_bound` sums all run files (four systems plus `abstention.tsv`)
    over each query id. It uses the emitted chunk `section_id` width.
  - All run files must fit one 16-MiB payload (ADR, line ~870). Admission
    checks this against the manifest-pinned `splits.test` before reservation.
    Writing checks it cumulatively. `score_captured` keeps a single
    `MAX_INPUT_BYTES` budget.
  - Astra round 2 found P1 (manifest counts are not bound to the split), P2
    (a 5×16-MiB budget) and two P3s. All four are fixed.
  - The Astra delta review found one P2: the bound measured section ids, not
    the emitted chunk `section_id`. Fixed, and the regression now widens the
    chunk field.
  - Residual: pre-reservation ordering is shown by the code order plus the
    bound unit test, not by an end-to-end custody regression.
- **FIX-4 DONE** (revision `55cd5058…4ac9`). Lists are capped lazily before
  cloning, with `take(MAX_LIST_ITEMS + 1)`.
- **Validation** (this session, local, exit 0):
  - focused: `cargo test --lib eval::` 48, `cli::` 7; integration `eval_run`
    13, `eval_gates` 20, `cli` 8, `mcp` 12;
  - full: `cargo test --all-targets` 256 passed, 1 ignored;
  - `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` ok;
  - `project_plan_validate` pass with 46 tasks;
  - all data synthetic.
- **Status copies reconciled:** `MEMORY.md` (99 lines), `project_plan-current.md`,
  `project_pending-tasks.md` and `project_architecture.md`.
- **Manager:** blocked. Every remaining task is gated on the owner or on human
  annotators: HTTP-0, then HTTP-2; EVAL-2.0, then EVAL-2.1/3.x and OD-4.x.
