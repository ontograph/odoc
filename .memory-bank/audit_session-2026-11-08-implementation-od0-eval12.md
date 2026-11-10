---
name: Implementation session OD-0.0 to EVAL-1.2
type: audit
---

# Implementation Session 2026-11-08: OD-0.0 to EVAL-1.2

Authority: [ONTODOC_IMPLEMENTATION_PLAN.md](ONTODOC_IMPLEMENTATION_PLAN.md),
[ADR_ONTODOC_SERVICE.md](ADR_ONTODOC_SERVICE.md) revision 3. Authorization: user
instruction "proceed tasks" (2026-11-08). Every task was executed in the main
session (`dispatch_mode: none`) and closed with `manager_loop_advance
action=complete-local`. Each task's `validation_summary` in the plan holds its
per-task evidence.

## Closed tasks

OD-0.0, OD-1.0, OD-1.1, OD-1.2, OD-1.3, OD-1.4, OD-1.5, OD-1.6, OD-1.7, OD-1.8,
OD-2.0, OD-2.1, EVAL-1.0, EVAL-1.1, EVAL-1.2: 15 of 21 task cards.

## Final gate run (after EVAL-1.2 fix)

| Command | Exit | Result |
| --- | --- | --- |
| `cargo fmt --check` | 0 | clean |
| `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings` | 0 | clean after removing a needless lifetime in `src/eval/sweep.rs` `split_queries` |
| `CARGO_BUILD_JOBS=4 cargo test` | 0 | 74 lib, 2 cli, 10 eval_gates, 3 eval_run, 4 mcp passed |
| `CARGO_BUILD_JOBS=4 cargo test --test eval_gates` | 0 | 10 passed |
| scan of non-test `src/eval/{gates,sweep,mod}.rs`, `src/cli.rs` for `unwrap`/`expect`/`println` | - | none found |

## Deviations from the plan text (accepted, recorded)

- B1 candidate pool for fusion is `max(50, top_k)` rather than a fixed 50, so
  large `top_k` requests are never truncated by the pool (`src/search/mod.rs`).
- Freshness compares full content hashes of the current walk with the
  generation; there is no size/mtime fast path. Correct but slower on large
  corpora (`src/store.rs::freshness`).
- Citation state `moved` requires exactly one matching span hash elsewhere;
  a duplicated span is reported `stale`, not `moved` (`src/cite.rs`).
- `CitationInput` in `src/mcp.rs` mirrors `cite::Citation` with a
  `JsonSchema` derive because `src/cite.rs` was outside OD-2.1's write set.

## EVAL-2.0 blocked

- Done inside the write set: `eval/corpora.json` pins
  - ontoindex `d535fe888a88694c524636bed8a7c409594184cb`, AGPL-3.0-or-later
    (local evaluation only, not redistributed), 325 Markdown files;
  - rust-lang/book `2771df165f092a4fbb9f46b5b2e7450f19010adb`, MIT OR
    Apache-2.0, cloned clean to `/opt/d1/_wrk/rust-book`, 112 Markdown files
    under `src/`.
- Blocker 1: the ADR requires human-written or human-reviewed queries
  (>= 270 total). No human reviewer is available in this autonomous session,
  so no queries were authored or drafted.
- Blocker 2: indexing writes `<root>/.ontodoc` inside the pinned checkouts,
  outside this workspace; needs user approval (or indexing of copies).
- Plan correction: the original `required_validation` passed a directory
  (`eval check-benchmark eval/`), which exits 1 with "Is a directory". It now
  reads `eval check-benchmark eval/config.json --stage pools --gates
  eval/gates.json`; `eval/config.json`, `eval/gates.json` and
  `eval/splits.json` were added to the write set.
- Manager state: `status: blocked`, `active_next_task: null`, reopen gate
  recorded in the plan. EVAL-2.1 (two human annotators, kappa >= 0.6) is the
  next human-gated step after EVAL-2.0.
