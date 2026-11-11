# Audit Session 2026-11-11: Review Round 4

Request: review and challenge what was planned and done; propose options;
cherry-pick the most robust and performant; consolidate into one plan and
carry it out. GPT-6-Astra ran three read-only reviews (product, evaluation and
process). Every finding below was re-checked in source at `3ce1ea0` before it
was accepted.

## Done Well

- The local safety work (OD-3.x) and the held-out custody design hold up.
- Numeric code hand-checks: Holm `[.03,.06,.06,.02]`; kappa 10/13; power at
  SD 0.2 gives 153 (ADR, lines 679–683).
- Determinism: iteration is sorted and ties break by id. Rounds 2–4 found no
  ordering defect.

## Worst

- About 7,100 lines of evaluation code have never measured anything. Two of
  its verified defects could still burn the sole held-out attempt (E1, E2
  below).
- The docs repeat status in several places, so copies go stale (see S1).

## Forgotten

- `docs_share` folders are usable today only as one stdio process per folder:
  `ontodoc --root docs_share/<name> serve`. Nothing documents this. A
  multiplexing stdio mode would contradict ADR lines 598–613.
- ADR lines 723–725 require a discrimination check. It exists
  (`stats::discrimination_oracle_beats_shuffle` and
  `tests/eval_run.rs:231`).

## Findings

| ID | Finding | Verdict | Disposition |
| --- | --- | --- | --- |
| E1 | P2: split/query ids with tab/CR/LF pass validation. `check_tsv_field` refuses them only while ranking output is written, after reservation. | Confirmed (`run.rs:798`, `:1668`) | FIX-5 |
| E2 | P2: an output parent that is not a directory, or a `symlink_metadata` error other than not-found, passes admission. Staging then fails after reservation. | Confirmed (`run.rs:1808`, `:2091`, `:2170`) | FIX-5 (directory and error kind). A writability probe is out of scope; residual recorded. |
| E3 | P3: empty per-corpus exploratory groups report `mean=0, CI=[0,0], p=1` with `n=0`. | Confirmed; exploratory only, and `n=0` is explicit | Deferred: the fix changes the report schema. |
| P1 | P2: `search` can return a citation whose headings exceed the 16-KiB citation input bound (ADR line 446), so `verify` refuses it. | Confirmed (`cli.rs:329`; no cap at build) | FIX-6: publication refuses heading paths above the bound, per AGENTS' "index caps fail publication". |
| P2 | P2: tier text shortened by L0/L1 byte caps carries no per-result flag (ADR line 97). | New argument (stored `l0_truncated`/`l1_truncated` are never exposed). Earlier rounds rejected only the response-level flag. | Deferred to the owner: an additive per-result field changes the MCP response shape and needs ADR review. |
| P3 | P3: `read("a#b.md#child")` splits at the first `#`. | **Rejected**: ADR lines 238–240 accept this limitation explicitly ("not extended"). `tests/cli.rs::schema_two_exact_ids_precede_unchanged_anchor_aliases` caught the attempted change, which was reverted. | — |
| X1 | Reviewer claimed P2: `related` runs an unbounded depth-3 BFS. | **Rejected**: `related` is a single filter pass over the bounded edge list. There is no depth parameter. | — |
| S1 | Stale docs: plan header says "In progress"; `project_plan-current.md` lists HTTP-0 before HTTP-1; `project_pending-tasks.md` says empty-slug needs OD-3.1/3.2 and HEAD is `b6e3f3e`; AGENTS.md line 46 says discovery is not implemented. | Confirmed | Review-scope docs |

## Options

| Option | What | Robustness | Value | Pick |
| --- | --- | --- | --- | --- |
| A | E1 and E2 pre-reservation refusals | High: refusals only, before reservation | Protects the sole attempt | **Yes** (FIX-5) |
| B | P1 publication cap (P3 rejected per ADR) | High: small, test-backed | Every returned citation verifies | **Yes** (FIX-6) |
| C | Fix stale docs; document per-folder stdio use | High: docs only | Folders usable today without HTTP | **Yes** (review-scope docs) |
| D | Share identity comparisons across run.rs and sweep.rs (60–100 lines) | Medium: held-out timing | Fewer lines, no user value | Deferred |
| E | Share kappa between gates and stats (20–30 lines) | Low risk | Small | Deferred |
| F | Compact the memory bank (MEMORY.md to 40 lines or fewer; archive older audits) | Medium: reference churn | Less drift | Deferred; S1 fixes the drift itself |
| G | Per-result `text_truncated`; E3 nullable statistics | Schema changes | Medium | Owner/ADR |
| H | Multiplexing stdio `--docs-share` | Contradicts ADR lines 598–613 | High | Rejected without an ADR change |

## Consolidated Plan

1. FIX-5: E1 and E2 (`src/eval/run.rs`).
2. FIX-6: P1 (`src/store.rs`, `src/cli.rs`).
3. Review-scope documentation (no plan task; the validator does not admit docs-only cards): S1 and per-folder stdio usage.

## Outcomes

- **FIX-5 DONE** (strict complete-local).
  - `validate_queries` and `check_run_ids` refuse TSV-unsafe ids. The
    `check_run_ids` call runs on the pinned `splits.test` before reservation.
  - `external_path` now requires a directory parent.
  - Admission refuses `symlink_metadata` errors other than not-found.
  - Residual: an unwritable directory still fails at staging. A writability
    probe is a non-goal.
- **FIX-6 DONE** (revision `03756f88…b64b`).
  - Publication refuses a section or chunk heading path beyond the bound
    `verify` accepts (64 headings, 16 KiB). The constants are shared.
  - The check is build-only, so no schema bump: existing generations still load.
  - P3 was **rejected** mid-task. ADR lines 238–240 keep the `#`-filename alias
    limitation on purpose, and `tests/cli.rs` caught the attempted change.
    The change was reverted.
- **Astra delta review:** no findings except one P3. The cap should also cover
  chunk heading paths, which are what citations emit. Fixed.
- **Review-scope documentation:**
  - README now documents one stdio server per `docs_share` folder.
  - AGENTS.md line 46: discovery exists; no route or listener is implemented.
  - Stale `project_pending-tasks.md` and `project_plan-current.md` lines fixed.
  - Plan header now reads "Blocked on owner gates".
- **Validation**, local:
  - `cargo test --all-targets`: 257 passed, 1 ignored.
  - After the chunk-path extension: `store::` 44, `cli` 8 and `mcp` 12 passed.
  - fmt and clippy `-D warnings`: ok.
  - Plan validation: pass, 49 tasks.
- **Rejected:** X1, the "unbounded BFS" in `related`; no such code exists.
- **Deferred:** E3 nullable statistics and a per-result `text_truncated`
  field (both change the schema, so they need owner/ADR review); options D, E
  and F.
- The manager is blocked again on HTTP-0 and EVAL-2.0.
