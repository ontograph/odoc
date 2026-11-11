# Review Round 6: Challenge, Consolidate, Proceed

Date: 2026-11-11. HEAD at start: `dddc179`.
Authority: user instruction to review and challenge what was planned and done,
consolidate the most robust proposals into one plan and proceed, using
OntoIndex and GPT sub-agents.

Three read-only GPT-6-Astra reviewers covered the parser/tiers, the numeric
scorers and tests, and operations/docs. OntoIndex `impact` was used before
editing. Every finding below was reproduced or confirmed in source.

## Done Well

- Service boundaries hold: stdio and HTTP share one handler set, each folder
  has its own store, and indexing stays in the trusted CLI.
- Measurement guards are intact; no quality claim was made.

## Worst

- The test suite was slow for a bad reason. Unoptimized debug `sha2` spent
  13.8 s hashing the 170 MB test binary in one registration test.
- Edge cases of the numeric scorers had no hand-computed tests: the empty
  document, the PPR iteration cap, and fusion ties, drops and singletons.

## Forgotten

- Two Markdown-structure edge cases (findings 1 and 2) and an interrupted
  prune that cannot recover (finding 3).
- The README lacked a per-folder index loop. It also did not say that folders
  are discovered at startup, that unindexed folders return `no_index`, or how
  to configure a URL client.

## Findings And Decisions

| # | Finding | Decision |
| --- | --- | --- |
| 1 | P2, `parse.rs::top_level_blocks` ignores `SoftBreak`. A multiline setext heading `Alpha\nBeta\n====` becomes `AlphaBeta`, id `#alphabeta`. GitHub renders `<h1>Alpha<br>Beta</h1>` (checked against the GitHub Markdown API). | **Deferred to owner.** Changing this changes section IDs, which are qrel keys. AGENTS.md requires ADR/schema review for identity changes, and GitHub's exact slug for an embedded newline was not established. A trial edit was reverted. Needs an ADR note and either a schema bump or a recorded reindex. |
| 2 | P2, `tiers.rs::first_sentence` uses `Parser::new` without `ENABLE_TABLES`, so a GFM table can become the "first sentence" in L0, L1 and the pseudo-document. | **Deferred to owner.** It changes tier bytes and B3 input, so it is a ranking change that needs dev-split evidence, and evaluation is parked. Fix sketch: share the `parse.rs` options. |
| 3 | P2, `store.rs`: an interrupted prune leaves an incomplete generation directory. `remove_owned` then refuses it, and pruning stays disabled. | **Deferred to owner.** The ADR permits incomplete shapes only for temp directories, so a fix needs an ADR amendment. Publication and reads are unaffected; the only cost is disk retention. |
| 4 | Test speed. | Fixed (FIX-10): `[profile.dev.package.{sha2,digest,block-buffer}] opt-level = 3`. All targets now finish in 17.8 s wall, including compilation. |
| 5 | Missing hand-computed numeric tests. | Fixed (FIX-10). BM25 empty document: N = 2, avgdl = ½, score (22/31)·ln 2. PPR two-node cap: A = 20/37 + (17/37)·0.85^100. Fusion: w = ½ gives all ½ ordered by raw BM25; w = 0 gives [1, ½] and drops zero-BM25 candidates; top-k; a singleton scores 0. Behaviour is unchanged. |
| 6 | README usability. | Fixed (documentation only): index loop, startup discovery, `no_index` until indexed, URL client note. |
| 7 | `tests/http.rs` reads lack deadlines; drain under active work and the `serve-http` binary are untested end to end. | Deferred: optional hardening, with no failure observed. |
| 8 | PPR buffer reuse, `corpus.rs` allocation, splitting `store.rs`. | Rejected: unmeasured, or the file is about 57% tests. |

Overengineering avoided: no new CLI or health surface, no tier or schema churn
without the owner, and no store split.

## Validation (this session)

- `cargo test --all-targets`, exit 0:
  - lib: 213 passed, 1 ignored;
  - cli: 8;
  - eval_gates: 20;
  - eval_run: 14;
  - http: 4;
  - mcp: 12;
  - ranking_sanity: 1.
- `cargo fmt --check`: ok.
- `cargo clippy --all-targets -- -D warnings`: ok.
- `git diff --check`: ok.
- `project_plan_validate`: pass, 54 tasks. FIX-10 was closed through strict
  `reopen` and `complete-local`.
- Synthetic data only.

## Owner Decisions Requested

1. Findings 1 and 2: accept an ADR note and a reindex (or a schema bump) for
   the setext soft-break slug and table-aware first sentence. For finding 2,
   accept it as an index-content fix before the first dev registration.
2. Finding 3: amend the ADR so prune may remove an owned incomplete generation
   directory.
