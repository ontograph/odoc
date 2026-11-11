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

## Round 7 (same day): Challenge Of Round 6

A GPT-6-Astra reviewer re-checked round 6; a second reviewer checked the fix.
OntoIndex `impact` on `record_and_prune` reported HIGH (36 nodes, all within
`publish_inner`, its callers and the store tests); all of them ran green.

| # | Finding | Decision |
| --- | --- | --- |
| R1 | Finding 1's premise was wrong. github-slugger takes the heading's `textContent`. `<br>` contributes no text and the slugger strips LF, so GitHub's anchor is `alphabeta`, as OntoDoc already produces. | **Closed, no bug.** Section IDs are correct. Only the human-readable `heading_path` loses the space, which is cosmetic and left unchanged. |
| R2 | Finding 3 did not need a relaxed rule. | Fixed (FIX-11, ADR revision 11). Prune renames each validated complete generation to `tmp-<id>` (refusing an occupied name), syncs `generations/` and the store directory, then deletes. An interrupted deletion leaves an incomplete temp shape, which the existing temp cleanup removes on the next publish. A regression test fails on the second file and checks recovery to three generations. Already-existing incomplete generation directories still refuse; that is unchanged. |
| R3 | P3: the fusion tie test could not detect a missing BM25 comparator, because input, id and BM25 order coincided. | Fixed: a shuffled-input test where only raw BM25 decides, and an all-equal test where only chunk id decides. |
| R4 | P2: `tests/http.rs` had an unbounded `read_to_end` and assumed a single-read status line. | Fixed: a 10 s deadline, and reads accumulate to `\r\n\r\n`, capped at 8 KiB. |
| R5 | Finding 2 (tables chosen as the first sentence). | Deferral confirmed: the fix changes tier and B3 bytes. Still an owner decision. |

Round-7 overengineering avoided: no recovery janitor for legacy incomplete
directories, no batch rename-then-delete restructuring (a single loop under the
held lock is sufficient), and no cosmetic heading-text change.

Validation (round 7):
- `cargo test --all-targets`, exit 0: lib 214 passed, 1 ignored; cli 8,
  eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `git diff --check`: ok.
- `project_plan_validate`: pass, 55 tasks. FIX-11 was closed through strict
  `reopen` and `complete-local`.

## Owner Approval: Table-Aware First Sentence (FIX-12)

The owner replied "approved" to round-7 option 2.
- `src/parse.rs` now has a single `MARKDOWN_OPTIONS` const. Sectioning, tier
  `first_sentence` and graph extraction all use it. This also removed a
  duplicate options list in `graph.rs`.
- A table before a paragraph now yields the paragraph's first sentence.
- `SCHEMA_VERSION` is 3 (ADR revision 12), so older stores refuse to load
  until a trusted `ontodoc index` rebuilds them.
- Section and chunk IDs are unchanged, and every ID fixture test stayed green.
- No dev or held-out measurement exists, so no evaluation result is affected.

Validation: `cargo test --all-targets` exit 0 (lib 214 passed, 1 ignored;
integration targets unchanged); fmt, clippy `-D warnings` and `git diff --check`
ok; `project_plan_validate` pass, 56 tasks.
