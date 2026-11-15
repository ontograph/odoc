# Refactoring Plan Registration

Date: 2026-11-15
Plan: [REFACTORING_PLAN.md](../docs/REFACTORING_PLAN.md), revision 2

The owner instructed on 2026-11-15: "proceed open tasks from docs/REFACTORING_PLAN.md". The plan states, "Proceeding with this plan accepts the recommendations below unless the owner states otherwise." This records acceptance of all five recommendations:

1. The tracker stays in `docs/REFACTORING_PLAN.md`.
2. Delete `section_ranking`.
3. Keep the first/last `DocumentMap` pair and correct the comment only.
4. RF-0 golden and RF-1 stage extraction are withdrawn.
5. No changelog entry; the audit records the removal.

Baseline HEAD: `ee511dc0f1b2300266fee23e7012463d9d3e2f11`

Baseline file SHA-256 hashes:

| File | SHA-256 |
| --- | --- |
| `src/search/mod.rs` | `03a73eee8fbadb4faf5f3169d873011af4272a40d2aabf4413a627d3fc1c791b` |
| `src/cli.rs` | `d30789db08dafaf600e80d45369b2ebc7125ab9057382aa2bb1efcda428f747c` |
| `src/eval/sweep.rs` | `7d3dcf3b7480b94b53b69bef6dad6b30b1ccb9119c80c4f59d6681a6849d81c1` |
| `src/eval/run.rs` | `717214e26ae841414851acfd0ff71a2407c0dc6c06ed91b238113b43cb4be80e` |
| `tests/eval_run.rs` | `9f2b8525ce125fe265a4b90c8d931fef7326c37ea068a7e95a111998cb2312eb` |

Baseline dirty status for these files: ` M src/cli.rs`. `src/cli.rs` already differed from HEAD before any RF task.

## Task log

### RF-REG

Commands and exit statuses:

- `git rev-parse HEAD` — exit 0.
- `sha256sum src/search/mod.rs src/cli.rs src/eval/sweep.rs src/eval/run.rs tests/eval_run.rs` — exit 0.
- `git status --short -- src/search/mod.rs src/cli.rs src/eval/sweep.rs src/eval/run.rs tests/eval_run.rs` — exit 0.
- Tracking files were not updated: MEMORY.md, project_plan-current.md and project_pending-tasks.md are reserved for coder-manager, which is unavailable in this session; the owner or a coder-manager applies them.
- `rg -n '[[:blank:]]+$' .memory-bank/audit_session-2026-11-15-refactoring.md` — exit 1, no match (corrected at RF-CLOSE; it was recorded as pending).

### RF-DOC

- The coordinator applied the ten review-file corrections, eight ideas-table rewordings and three architecture-document rows through apply_patch.
- A Luna explorer ran the trailing-blank search on both files: exit 1, no match. 271 entries in sequence with six bullets each.
- No copy was taken before editing; rollback reapplies the old text quoted in the plan tables.

### RF-2

- OntoIndex upstream impact on `section_ranking`: 0 callers, risk LOW.
- Removed public function: `ontodoc::search::section_ranking` and its only use, the unit test `chunk_hits_map_to_sections_by_first_occurrence`. No changelog entry, per decision 5.
- `rg -n section_ranking src tests` — exit 1, no match (re-run at RF-CLOSE, exit 1).
- `cargo fmt --check` — exit 0 (worker-reported).
- Clippy lane `ontodoc-rf2-clippy-20261115`, `cargo clippy --all-targets -- -D warnings` — terminal, exit 0.
- `CARGO_BUILD_JOBS=4 cargo test --lib search::` — 24 passed, exit 0 (explorer run).

### RF-3

- OntoIndex upstream impact on `misses` in `src/cli.rs`: LOW, one caller `run_eval`.
- `Miss` and `misses` moved to `src/eval/sweep.rs` (`pub(crate)`, returning `EvalError`); `FETCH` became `pub(super)` in `src/eval/run.rs` and replaced the literal `50` in `attribute_miss`.
- `tests/eval_run.rs` asserts the full `eval misses` JSON: 75 entries (38 `a-`, 37 `b-`), all stage `truncated`, system B0, two sections. The RF-3 record said 71; RF-CLOSE counted 75 distinct IDs with no duplicates and corrected the figure.
- Worker-reported, not independently re-run by the verifying explorer: `cargo test --test eval_run` 14 passed; `cargo test --all-targets` lib 232 passed and 4 ignored, cli 9, eval_gates 24, eval_run 14, http 7, mcp 17, ranking_sanity 1; `cargo test --lib eval::` 51 passed.
- Explorer-verified: `cargo fmt --check` exit 0; Clippy lane `ontodoc-rf3-clippy-20261115` terminal exit 0; five SHA-256 values listed below.

### RF-CLOSE

- `CARGO_BUILD_JOBS=4 cargo test --test eval_run` — 14 passed, exit 0 (coordinator run, 2026-11-15).
- `sha256sum` of the five task files — exit 0: `src/cli.rs` `cbc43a0ea0eae5be0eafd44e3f0914a8c7d2129ca26651b351c0418bf0ec474f`, `src/eval/sweep.rs` `3b4c8210c605a7b757325cfae7289d3d902cfea94a7e517bd1f0bfb4c03995db`, `src/eval/run.rs` `fbc6c9559ba438a40d21b5fa138ea9deea0c6dd6b0577734ec4e61dfeaee477d`, `tests/eval_run.rs` `7944c415afc8d328c4f37f8f9bf6f9ac94bbe016b41b386dd77ff1b624f8fcdc`, `src/search/mod.rs` `af85d29b8464d551183ac9b7b1e286003c6a2fb4bd48649e2f06c417c80335cc`.
- Router: the `src/eval/{gates,sweep}.rs` row now names miss collection. The `src/search/mod.rs` row (admission, first/last document map, trusted constructor at line 202) and the `src/cli.rs` row (`eval misses` subcommand) were re-read and remain true.
- Closed as not needed: RF-0 golden, RF-1 stage extraction, RF-4 document-map shrink (withdrawn in plan revision 2).
- Skipped: full `--all-targets` test and Clippy re-run at RF-CLOSE (no Rust changed after RF-3).

### Limits

- That the misses assertion passed on unmoved source before step 2 rests only on the worker's attestation.
- The pin covers only system B0 and stage `truncated`; other stages and systems are not characterised.
- Worker result messages disagreed on counts, diffstat and one hash, one of which was fabricated; only figures matching diffs, hashes or test tails are recorded.
- The verifying explorer did not re-run the RF-3 test suites; the coordinator re-ran them after closeout (below).
- No retrieval quality claim follows from this plan.

### Post-closeout re-run (coordinator, 2026-11-15)

- `CARGO_BUILD_JOBS=4 cargo test --all-targets` — exit 0: lib 232 passed and 4 ignored, main 0, cli 9, eval_gates 24, eval_run 14, http 7, mcp 17, ranking_sanity 1; 0 failed. This matches the worker-reported RF-3 figures.
- `cargo fmt --check` — exit 0.
- `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings` — exit 0.

## Revision 3 (post-closeout review)

A GPT-6-Astra read-only explorer and a coordinator source read reviewed the closed work.

- Done well: the `misses` move preserves behaviour (`rank_sections` truncates to `CUTOFF`; qrels are nested `BTreeMap`s), and withdrawing RF-0, RF-1 and RF-4 avoided moving search code for readability alone.
- Done worst: worker reports were unreliable (a miscount of 71 for 75, one fabricated hash), so every figure needed re-verification.
- Forgotten: `FusionDisplaced` and `Truncated` had no unit assertion; the pinned `eval misses` output exercises only B0 and `truncated`. The `Truncated` comment ("just below the cutoff") misdescribed the fallback rule. The `Misses` arm bypassed the local `pretty` helper. The plan header still read as unstarted.
- Overengineering: none new. The 75-entry pin stays because counts and samples would lose membership and order checks.
- Declined, with reasons in the plan: replacing the pin, narrowing `attribute_miss`/`MissStage` visibility (a public-surface change needing an owner decision), a richer `eval_run` fixture (other assertions depend on it), rewriting historical references, reopening RF-0, RF-1 or RF-4.

### RF-5

- OntoIndex upstream impact: `attribute_miss` LOW (one caller, `misses`); `run_eval` in `src/cli.rs` LOW (callers `run`, `EvalCommand.run`).
- A GPT worker (gpt-6.1-sol) edited `src/eval/sweep.rs` and `src/cli.rs`: the test is renamed `attribute_miss_reports_every_stage` and asserts `FusionDisplaced` for B2(0.3) and B3(0.3) and `Truncated` for B0 on `a.md#parent/child`, so every `MissStage` variant is asserted; the `Truncated` comment now states the fallback rule; the `Misses` arm uses `pretty`.
- Coordinator re-runs: `CARGO_BUILD_JOBS=4 cargo test --lib eval::sweep::` 5 passed, exit 0; `CARGO_BUILD_JOBS=4 cargo test --test eval_run` 14 passed, exit 0 (pinned misses unchanged); `cargo fmt --check` exit 0; Clippy lane `ontodoc-rf5-clippy-20261115` terminal, exit 0.
- Skipped: full `--all-targets` test re-run (the change touches one unit test, one comment and one equivalent call; the affected suites ran).
- No retrieval quality claim.
