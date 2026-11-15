# OntoDoc Refactoring Plan

Date: 2026-11-15
Revision: 3
Status: Revision 2's five tasks are DONE (closed 2026-11-15; see the [audit](../.memory-bank/audit_session-2026-11-15-refactoring.md)). Revision 3 follows a post-closeout review and adds RF-5 and RF-CLOSE2. Statements in the revision 1 and 2 review sections and in Limits describe the tree before implementation and are kept as history.
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (Accepted, revision 19) and `AGENTS.md`. This plan changes neither.
Source: the audit of `.ontocode/research-reports/arxiv-graphrag-2026-review.md` against the OntoDoc source, delivered on 2026-11-15, and the review of revision 1 of this plan on the same day.
Requested by: user, "based on that report provide refactoring plan for ontodoc", then "review and challenge docs/REFACTORING_PLAN.md, update doc".

## Goal

The audit found no paper result that supports a change to OntoDoc's ranking. The review of revision 1 then found that most of its code tasks were cosmetic, and that one of them changed the behaviour of public constructors while the plan said nothing changed. Revision 2 keeps the three pieces of work that have a concrete reason:

1. Delete one function that nothing calls, and correct one comment in `src/search/mod.rs` that describes a pair of indices without saying when the pair matters.
2. Move the miss-collection loop from the CLI module to the evaluation module that already owns miss attribution, after a test pins its output.
3. Correct the ten statements the audit found wrong in the review file, and the three the architecture document inherited.

No task changes a ranking, a default, the stored schema, the tokenizer, the five MCP tools, their fields or the output of any CLI command. RF-2 removes one `pub` function from the library crate; that is a change to the crate's surface and is listed as owner decision 2. No task is evidence that retrieval improves, and the plan claims none.

## Review Of Revision 1

Revision 1 was checked against source before this rewrite. Two GPT-6-Luna explorers quoted `rg` and `sed` output for the search, store, evaluation and CLI claims, a third did the same for the Git facts, and a GPT-6.1-Sol challenger argued against each task. The coordinator read `src/search/mod.rs` in full, including its test module. A fourth explorer re-ran the line lookups on the working tree before this revision was written.

### Factual errors

| Revision 1 said | What the source shows | Change in revision 2 |
| --- | --- | --- |
| Every task keeps behaviour byte-identical, and the first/last document pair that RF-4 removes is unreachable | `SearchIndex::new` (`src/search/mod.rs:202`) and `try_new` (`:209`) are `pub` and do not call `Generation::validate`. `store::build` (`src/store.rs:453`) returns without calling it. Two tests build a duplicate path on purpose (`:515`, `:591`). The pair is unreachable only for a generation that passed validation | RF-4 is withdrawn. RF-2 corrects the comment instead |
| The callers of `search` include three unit tests | At least seven test functions in `src/search/mod.rs` call it | No task edits `search`, so the claim is gone |
| Existing tests check single hits and statistical properties only | The test module asserts multi-hit order, full `Ranked` equality with explain for three modes on four queries against an independently built index, and zero-weight order over 7 queries, 3 `top_k` values and 2 corpora (`:765`) | The case for a golden is weaker than stated. RF-0 is withdrawn |
| More than 50 chunks matching one term makes both the cut at 50 and the admission of PageRank-only chunks happen | It guarantees the cut only. A PageRank-only pool member needs a chunk with zero BM25 linked to a seed, and the proposed self-check could have been met by the small `tests/fixtures/graph` corpus alone | Recorded in the deferred design |
| RF-0's case list reaches `NoLexicalMatch` with an absent term | `abstention` returns `NoLexicalMatch` only when a threshold is set (`src/search/mod.rs:410`) | Recorded in the deferred design |
| RF-3: whether `score`, `Bench::qrels` and `Bench::indexes` share one error type was not checked | All three return `EvalError` (`src/eval/run.rs:2860`, `:386`, `:414`), and `AppError` converts it at `src/cli.rs:469` | RF-3 names the return type; no new variant |
| RF-3: whether a test asserts the output of `eval misses` was not checked | `tests/eval_run.rs:320` runs the command and discards the JSON. No test asserts a `Miss` value | RF-3 starts with a characterisation assertion |
| `git diff --check` validates the documentation tasks | `docs/` and `.ontocode/` are untracked (`git ls-files` returns nothing for them). `git diff --check` does not read untracked files | Those tasks use an `rg` search for trailing blanks |
| The Clippy command line is the evidence | `compiler-lane-detached` only starts the lane (`/opt/d1/_wrk/ontocode/justfile:177`). The result comes from `lane-result` (`:187`) | Each code task lists both commands |
| The baseline is the commit and the list of uncommitted source files | File names do not identify the content of a dirty tree | RF-REG records SHA-256 of the five files the code tasks touch |

### Over-engineering

| Revision 1 | Why it is more than the problem needs | Revision 2 |
| --- | --- | --- |
| RF-1: extract `scope_mask`, `lexical_only` and `candidate_pool` from `search` | The rule that B2 and B3 share one pool is already stated in the module comment (`src/search/mod.rs:12`), and both modes already fall through one block (`:349`). The extraction moves code on the path of every request and every evaluation run for a readability gain only | Withdrawn. RF-2 adds one comment line above the pool block |
| RF-0: a golden for three arms on two corpora | Its job was to gate RF-1. With RF-1 and RF-4 withdrawn no search code moves, and the golden does not cover the output RF-3 moves | Withdrawn; design kept under Deferred |
| RF-4: shrink a private map and rewrite two tests | Small gain, and it changes what two tests assert | Withdrawn; comment corrected in RF-2 |
| `cargo test --all-targets` and the golden in every task | Each small task repeated the full suite | Focused commands per task, one full run at the end of RF-3 |
| A dependency chain RF-1, RF-2, RF-3, RF-4 | `max_in_flight: 1` already serialises the work. Only one edge has a reason | RF-3 depends on RF-2 so the one full run covers both code changes |
| `FETCH` visible in the whole crate | `sweep.rs` is a sibling of `run.rs` under `eval` | `pub(super)` |
| A changelog entry for the removed function | `Cargo.toml:8` sets `publish = false` and no caller exists | Recorded in the audit; owner decision 5 |

### Forgotten

- A check that pins the output of `eval misses` before the code moves. `tests/eval_run.rs` was not in RF-3's write set.
- `misses` takes the first `CUTOFF` entries of a ranking (`src/cli.rs:1560`) that `rank_sections` already cuts at `CUTOFF` (`src/eval/run.rs:1581`). The `take` is redundant and can go during the move.
- `src/cli.rs` has uncommitted edits from other work. Rollback for RF-3 has to remove this plan's hunks only.
- The architecture router rows the tasks touch: `.memory-bank/project_architecture.md:85` (search module), `:88` (CLI subcommands) and `:94` (miss attribution in `src/eval/{gates,sweep}.rs`).

### What held up

- No ranking, schema, tool or field change, and the list of non-goals.
- RF-2: `section_ranking` has one definition (`src/search/mod.rs:435`) and one use, in the unit test at `:678`.
- RF-3's direction: `src/eval/sweep.rs` already owns `MissStage` (`:236`) and `attribute_miss` (`:253`), whose only caller is `misses`.
- RF-DOC, unchanged.
- The "considered and not selected" list.

## What The Audit Implies

| Audit finding | Consequence for this plan |
| --- | --- |
| OntoDoc already has the structure and link graph, degree-normalised PageRank with a 100-iteration cap, rank-ordered budgeted assembly, `verify` with an exact byte hash, and the BM25 baseline | No new mechanism |
| No paper gives evidence about OntoDoc's B2 or B3 arms | No ranking, fusion or damping change, and no task edits `SearchIndex::search` |
| "Equal candidate budgets across compared arms" [106, 192, 219] holds as protocol discipline | The shared pool is already one block. RF-2 adds a comment that names the rule; RF-3 replaces an unnamed `50` with `FETCH` |
| Eight of the twelve "ideas that fit" rows need rewording and row 8 should go | RF-DOC |
| Ten statements in the review file are wrong, and three were copied into the architecture document | RF-DOC |

## Current Structure

Line numbers are from the working tree on 2026-11-15 at HEAD `ee511dc0f1b2300266fee23e7012463d9d3e2f11`. `src/search/` and `src/eval/` show no difference from HEAD; `src/cli.rs` does.

| Area | Where | What is there now |
| --- | --- | --- |
| Candidate pool | `src/search/mod.rs:349` to `:359` | B1 top `max(50, top_k)` plus the in-scope PageRank top 50, built inline once for `Mode::Graph` and `Mode::Hier` |
| Dead function | `section_ranking`, `src/search/mod.rs:435` | Section ids by first occurrence. Used only by the unit test `chunk_hits_map_to_sections_by_first_occurrence` (`:678`). The evaluation uses `rank_sections_to` (`src/eval/run.rs:1586`), which also carries scores and a depth |
| Document lookup | `DocumentMap`, `src/search/mod.rs:38` | A first and a last document index per path. The comment at `:36` calls this historical and does not say when the two differ |
| Duplicate paths | `Generation::validate`, `src/store.rs:289`; message at `:344` | Refused for every stored generation. The public constructors `SearchIndex::new` and `try_new` do not run this check |
| Miss collection | `Miss` and `misses`, `src/cli.rs:1529` and `:1537` | A loop over queries, four systems and labels inside the CLI module. It returns `AppError` only because its three fallible calls convert from `EvalError` |
| Fetch depth | `FETCH`, `src/eval/run.rs:32`; literal at `src/eval/sweep.rs:299` | `attribute_miss` writes `Params::new(Mode::Lexical, 50)` where it means the evaluation fetch depth |
| Miss test | `tests/eval_run.rs:320` | Runs `eval misses` on a dev fixture run and checks only that it succeeds |

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: closed
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: null
tasks:
  RF-REG:
    title: Register the refactoring tasks, the owner decisions and the file baseline
    status: DONE
    classification: architecture-paperwork
    type: tracking
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    allowed_write_set:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    non_goals:
    - No source, ranking, schema, tokenizer, evaluation input, ADR or preserved release artifact change, and no edit to the memory-bank tracking files, which the runtime reserves for a coder-manager.
    required_validation:
    - rg -n '[[:blank:]]+$' docs/REFACTORING_PLAN.md .memory-bank/audit_session-2026-11-15-refactoring.md
    - project_plan_validate docs/REFACTORING_PLAN.md
    expected_evidence:
    - The audit records each owner decision from Decisions And Non-Goals, the HEAD commit, and the SHA-256 of src/search/mod.rs, src/cli.rs, src/eval/sweep.rs, src/eval/run.rs and tests/eval_run.rs as they stand before any task edits them.
    - The trailing-blank search prints nothing and the tracker validates. Routing from MEMORY.md, project_plan-current.md and project_pending-tasks.md moves to RF-CLOSE.
    rollback:
    - Revert only this task's changes; preserve pre-existing work.
    stop_conditions:
    - The owner declines the plan; every later task is then removed before any code.
    - The owner changes a decision; the affected task card is rewritten before it starts.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/REFACTORING_PLAN.md on 2026-11-15; RF-REG accepts recommendations 1-5.
        validation_summary: The Luna worker ran git rev-parse HEAD (exit 0, ee511dc0f1b2300266fee23e7012463d9d3e2f11), sha256sum of the five task files (exit 0) and git status (exit 0, only src/cli.rs modified), and wrote .memory-bank/audit_session-2026-11-15-refactoring.md with the five decisions and hashes; the trailing-blank search on it exited 1 with no match. Strict project_plan_validate passed. MEMORY.md, project_plan-current.md and project_pending-tasks.md were not updated because the runtime reserves them for a coder-manager, which is unavailable; that routing moves to RF-CLOSE.
    dod_status: pass
  RF-DOC:
    title: Correct the review file and the architecture document
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RF-REG
    owner_files:
    - .ontocode/research-reports/arxiv-graphrag-2026-review.md
    - .ontocode/research-reports/graphrag-2026-ideal-architecture.md
    allowed_write_set:
    - .ontocode/research-reports/arxiv-graphrag-2026-review.md
    - .ontocode/research-reports/graphrag-2026-ideal-architecture.md
    non_goals:
    - No new paper reading, no new idea row, no change to an entry the audit did not name, and no retrieval quality claim.
    required_validation:
    - rg -n '[[:blank:]]+$' .ontocode/research-reports/arxiv-graphrag-2026-review.md .ontocode/research-reports/graphrag-2026-ideal-architecture.md
    expected_evidence:
    - Each row of the RF-DOC tables is applied or marked as declined with a reason; the entry count stays 271 and the six bullets per entry stay in order.
    - The trailing-blank search prints nothing.
    rollback:
    - Restore the two files from the copies taken before the task; both are untracked, so Git cannot restore them.
    stop_conditions:
    - A correction needs a page of a paper that the audit did not read.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/REFACTORING_PLAN.md on 2026-11-15.
        validation_summary: The coordinator applied all ten review-file rows, the eight ideas-table rewordings and the three architecture-document rows through apply_patch. Entries 19 and 184 now read index = not stated in pages 1-12, entry 86 reads query retrieval = yes, and the derived LLM counts were adjusted to match (index 146/63/62, retrieval 151/90/30, neither 31) in both files, which the RF-DOC tables implied without listing. A Luna explorer ran the checks. The trailing-blank search on both files exited 1 with no match. 271 numbered entries run in sequence 1 to 271, and every entry has its six bullets in order. A script count over the entries gives 146 index-time and 151 query-time yes values. No retired phrase remains in the review file. The remaining 32 and 148 matches are entry numbers or rows of the architecture document's historical revision record. No copy was taken before editing, so rollback means reapplying the old text from these tables and the revision 1 wording quoted in them.
    dod_status: pass
  RF-2:
    title: Remove the unused search::section_ranking and correct two comments
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on:
    - RF-REG
    owner_files:
    - src/search/mod.rs
    allowed_write_set:
    - src/search/mod.rs
    non_goals:
    - No change to SearchIndex::search statements, DocumentMap, a constructor, a constant, rank_sections, rank_sections_to or any other test.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib search::'
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-rf2-clippy-20261115 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-rf2-clippy-20261115
    - rg -n section_ranking src tests
    expected_evidence:
    - Before the edit the source search shows the definition and one unit test; after it the search prints nothing.
    - The diff holds one deleted function, one deleted test or assertion, and comment lines only; no executable statement outside them changes.
    - The lane result reports Clippy success for the job; the start command alone is not evidence.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - The search or the compiler finds another caller; keep the function and close the task as not needed.
    - The owner did not accept decision 2.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/REFACTORING_PLAN.md on 2026-11-15; decision 2 accepted in RF-REG.
        validation_summary: OntoIndex upstream impact on section_ranking reported 0 callers, risk LOW. A Luna worker confirmed the baseline SHA-256 03a73eee8fbadb4faf5f3169d873011af4272a40d2aabf4413a627d3fc1c791b. Its rg before the edit showed the definition at src/search/mod.rs line 435 and the test use at line 697. It then deleted section_ranking and the test chunk_hits_map_to_sections_by_first_occurrence, rewrote the two-line DocumentMap comment as four lines, and added one comment line above the pool. It reported cargo fmt --check exit 0. A separate Luna explorer then confirmed the result. The git diff hunks show only those edits. rg -n section_ranking src tests exited 1 with no match. The lane result for job ontodoc-rf2-clippy-20261115 is terminal with exit code 0 for cargo clippy --all-targets -- -D warnings. sha256sum after the edit is af85d29b8464d551183ac9b7b1e286003c6a2fb4bd48649e2f06c417c80335cc. One explorer run of the focused search test command (cargo test --lib with the search filter, CARGO_BUILD_JOBS=4) printed 24 passed, 0 failed, 0 ignored, 212 filtered out, and cargo exit status 0. Two result messages from each sub-agent disagreed on test counts, diffstat and one hash. The coordinator records only the figures above, which match the diff hunks and the test tail. The full suite runs once at the end of RF-3.
    dod_status: pass
  RF-3:
    title: Pin the eval misses output, then move miss collection to eval::sweep
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: release
    depends_on:
    - RF-2
    owner_files:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    non_goals:
    - No change to an admission refusal, to the order of checks in run_eval, to a Miss field name or order, to FETCH's value, to a fixture, or to any evaluation payload; no evaluation command is run on real benchmark inputs.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_run
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-rf3-clippy-20261115 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-rf3-clippy-20261115
    - rg -n 'Mode::Lexical, 50' src/eval/sweep.rs
    expected_evidence:
    - Step 1, on source with no RF-3 edit under src/, tests/eval_run.rs asserts the full JSON of eval misses for the dev fixture run, the output is not empty, and the test passes.
    - Step 2, Miss and misses live in src/eval/sweep.rs with the same field names, order and loop; the Misses arm of run_eval calls the moved function after the same refusals; the step 1 assertion passes unedited.
    - attribute_miss uses FETCH, FETCH keeps the value 50, and the literal search prints nothing.
    - The full suite passes once after both code tasks, and the lane result reports Clippy success.
    rollback:
    - Remove only this task's hunks. src/cli.rs carried uncommitted edits before the task; compare against the SHA-256 recorded by RF-REG and never restore the file from HEAD.
    stop_conditions:
    - The fixture run yields no misses, so step 1 cannot characterise the output; report to the owner before any move.
    - The step 1 assertion changes after the move; revert step 2 and report the first differing value.
    - The move needs a new error variant, a change to a refusal, or an edit to another test's expectation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/REFACTORING_PLAN.md on 2026-11-15.
        validation_summary: OntoIndex upstream impact on misses in src/cli.rs is LOW with one direct caller, run_eval. The first worker packet named the wrong test and the worker changed nothing; the four hashes still matched the RF-REG baseline. In the retry the Luna worker captured the eval misses JSON in admitted_protocol_runs_dev_then_one_held_out_attempt and asserted it in full. The output is 75 entries (corrected at RF-CLOSE from a stated 71; 38 a- and 37 b- IDs, no duplicates), all stage truncated and system B0, over two sections. The worker reports that the assertion passed on unmoved source before step 2; no separate check exists for that ordering. Step 2 moved Miss and misses to src/eval/sweep.rs as pub(crate), returning EvalError. It made FETCH pub(super) and used it in attribute_miss, dropped the redundant take of CUTOFF, and reduced the Misses arm in src/cli.rs to one call. A separate Luna explorer then checked the result. rg for the literal Mode Lexical 50 in src/eval/sweep.rs finds nothing, and Miss and misses exist only in src/eval/sweep.rs. The eval_run suite passed 14 of 14. cargo test --all-targets passed every target (lib 232 passed, 4 ignored; cli 9, eval_gates 24, eval_run 14, http 7, mcp 17, ranking_sanity 1; 0 failed). cargo fmt --check exited 0. The lane result for job ontodoc-rf3-clippy-20261115 is terminal with exit code 0. New SHA-256 values are src/cli.rs cbc43a0ea0eae5be0eafd44e3f0914a8c7d2129ca26651b351c0418bf0ec474f, src/eval/sweep.rs 3b4c8210c605a7b757325cfae7289d3d902cfea94a7e517bd1f0bfb4c03995db, src/eval/run.rs fbc6c9559ba438a40d21b5fa138ea9deea0c6dd6b0577734ec4e61dfeaee477d and tests/eval_run.rs 7944c415afc8d328c4f37f8f9bf6f9ac94bbe016b41b386dd77ff1b624f8fcdc. The pinned output exercises only the truncated stage for B0, so the other stages and systems in the loop are not characterised by it.
    dod_status: pass
  RF-CLOSE:
    title: Record the outcome in the architecture router and the audit
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RF-3
    - RF-DOC
    owner_files:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    allowed_write_set:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    non_goals:
    - No runtime change, no changelog entry unless the owner chose it in decision 5, and no retrieval quality claim.
    required_validation:
    - rg -n '[[:blank:]]+$' docs/REFACTORING_PLAN.md .memory-bank/project_architecture.md .memory-bank/MEMORY.md .memory-bank/project_plan-current.md .memory-bank/project_pending-tasks.md .memory-bank/audit_session-2026-11-15-refactoring.md
    - project_plan_validate docs/REFACTORING_PLAN.md
    expected_evidence:
    - The router row for src/eval/{gates,sweep}.rs names miss collection beside miss attribution; the rows for src/search/mod.rs and src/cli.rs were re-read and are still true.
    - The audit records every command with its exit status, the removed public function, every skipped check and every task closed as not needed.
    - The trailing-blank search prints nothing and MEMORY.md stays at or below 100 lines.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Recorded evidence contradicts the documentation.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/REFACTORING_PLAN.md on 2026-11-15.
        validation_summary: The coordinator re-ran CARGO_BUILD_JOBS=4 cargo test --test eval_run (14 passed, exit 0) and sha256sum of the five task files, which match the RF-3 values. A count of the pinned misses list found 75 distinct IDs and no duplicate, so the RF-3 figure of 71 was corrected. The src/eval/{gates,sweep}.rs router row now names miss collection; the src/search/mod.rs and src/cli.rs rows were re-read against source and remain true. The audit records commands with exit statuses for RF-REG through RF-CLOSE, the removed section_ranking function without a changelog entry, the withdrawn RF-0, RF-1 and RF-4, skipped checks and limits. MEMORY.md (46 lines), project_plan-current.md and project_pending-tasks.md route to the closed plan. The trailing-blank search over the six files exited 1 with no match and strict project_plan_validate passed.
    dod_status: pass
  RF-5:
    title: Pin the remaining miss stages and correct the Truncated doc comment
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on:
    - RF-CLOSE
    owner_files:
    - src/eval/sweep.rs
    - src/cli.rs
    allowed_write_set:
    - src/eval/sweep.rs
    - src/cli.rs
    non_goals:
    - No change to attribute_miss statements, MissStage variants or their order, visibility, FETCH, CUTOFF, any fixture or the pinned eval misses test.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::sweep::'
    - CARGO_BUILD_JOBS=4 cargo test --test eval_run
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-rf5-clippy-20261115 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-rf5-clippy-20261115
    expected_evidence:
    - The existing unit test in src/eval/sweep.rs also asserts FusionDisplaced for B2 and B3 and Truncated for B0 on the same matching section of its tempdir corpus; every MissStage variant is now asserted at least once.
    - The Truncated doc comment states the fallback rule the code applies (BM25 retrieves the section within FETCH chunks and no earlier stage applied), which includes sections already in B1's top 10 when the system is B0.
    - The Misses arm in run_eval uses the local pretty helper like every other eval arm, producing the same bytes.
    - The tests and fmt exit 0, and the lane result reports Clippy success for the job.
    rollback:
    - Remove only this task's hunks; src/cli.rs carries unrelated uncommitted edits.
    stop_conditions:
    - Asserting a stage requires changing attribute_miss or a fixture.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Review and challenge, consolidate to one plan and proceed it, 2026-11-15.
        validation_summary: OntoIndex upstream impact on attribute_miss is LOW with one direct caller, misses; on run_eval in src/cli.rs LOW with callers run and EvalCommand.run. A GPT worker extended the sweep unit test, renamed it attribute_miss_reports_every_stage, and asserted FusionDisplaced for B2(0.3) and B3(0.3) and Truncated for B0 on section a.md#parent/child, so every MissStage variant is now asserted. It replaced the Truncated doc comment with the fallback rule and switched the Misses arm in run_eval to the local pretty helper. The coordinator read the diff and re-ran the gates. CARGO_BUILD_JOBS=4 cargo test --lib with the eval sweep filter gave 5 passed, exit 0. CARGO_BUILD_JOBS=4 cargo test --test eval_run gave 14 passed, exit 0, so the pinned misses output is unchanged. cargo fmt --check exited 0. The lane result for job ontodoc-rf5-clippy-20261115 is terminal with exit code 0.
    dod_status: pass
  RF-CLOSE2:
    title: Record revision 3 in the audit
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RF-5
    owner_files:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    - .memory-bank/project_plan-current.md
    allowed_write_set:
    - docs/REFACTORING_PLAN.md
    - .memory-bank/audit_session-2026-11-15-refactoring.md
    - .memory-bank/project_plan-current.md
    non_goals:
    - No runtime change and no retrieval quality claim.
    required_validation:
    - rg -n '[[:blank:]]+$' docs/REFACTORING_PLAN.md .memory-bank/audit_session-2026-11-15-refactoring.md .memory-bank/project_plan-current.md
    - project_plan_validate docs/REFACTORING_PLAN.md
    expected_evidence:
    - The audit records the review findings, RF-5 commands with exit statuses, and the declined proposals with reasons.
    - The trailing-blank search prints nothing.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Recorded evidence contradicts the documentation.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Review and challenge, consolidate to one plan and proceed it, 2026-11-15.
        validation_summary: The audit gains a revision 3 section with the review findings (done well, done worst, forgotten, overengineering), the declined proposals with reasons, and RF-5 commands with exit statuses, plus the skipped full-suite re-run. project_plan-current.md shows 7 DONE. The trailing-blank search over the three files exited 1 with no match and strict project_plan_validate passed.
    dod_status: pass
```

## Review Of Revision 2 (post-closeout)

A GPT-6-Astra read-only review and a coordinator source read on 2026-11-15 checked the closed work.

| Finding | Evidence | Disposition |
| --- | --- | --- |
| The move of `misses` preserves behaviour | `rank_sections` truncates to `CUTOFF` (`src/eval/run.rs:1581`); qrels are nested `BTreeMap`s (`src/eval/metrics.rs:11`, `src/eval/run.rs:386`) | None |
| The pinned test is exact but narrow: 75 entries, all B0 and `truncated` | The fixture's answerable queries contain a token absent from every document, so the whole-query substring baseline B0 never matches, while BM25 finds the one relevant section per corpus; fusion weights are zero | Keep the pin. Cover the other stages in a unit test (RF-5) |
| `FusionDisplaced` and `Truncated` have no unit assertion | Only `LexicalMiss`, `ChunkBoundary` and `NotIndexed` are asserted (`src/eval/sweep.rs:822`) | RF-5 |
| The `Truncated` comment says "just below the cutoff", but the code is a fallback that also covers sections in B1's top 10 when B0 missed | `src/eval/sweep.rs:249`, `:345` | RF-5 corrects the comment |
| The `Misses` arm calls `serde_json::to_string_pretty` directly; every other eval arm uses the local `pretty` helper | `src/cli.rs:1287`, `:1513` | RF-5, same bytes |
| The plan header and Limits still described an unstarted plan | Lines 5 and 458 to 469 of revision 2 | Header corrected in revision 3; Limits kept as history |

Proposals considered and declined:

- Replace the 75-entry pin with per-system counts and samples. That would lose membership and order checks; the pin is cheap and stable.
- Narrow `attribute_miss` and `MissStage` to `pub(crate)`. This changes the public library surface without a user, and would need an owner decision like decision 2.
- Build a richer `eval_run` fixture so `misses` exercises all stages end to end. It would change fixtures that other assertions depend on; the unit test gives the same stage coverage at lower cost.
- Rewrite historical references to `section_ranking` in other closed plans. They are records of past state.
- Reopen RF-0, RF-1 or RF-4. Their withdrawal reasons still hold.

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| RF-R1 | User request for a refactoring plan from the audit, and for its review | architecture-gate | RF-REG | Strict tracker and trailing-blank search |
| RF-R2 | `AGENTS.md`: smallest correct change in the existing owner module; no speculative abstraction | implementation | RF-2, RF-3 | Focused suites, source search for the removed name and the replaced literal |
| RF-R3 | `AGENTS.md`: every changed non-trivial behaviour needs a focused runnable regression check | proof-only | RF-3 | The `eval misses` assertion passes before and after the move, unedited |
| RF-R4 | Audit: wrong statements in the two research documents | implementation | RF-DOC | Entry count and bullet order, trailing-blank search |
| RF-R5 | `AGENTS.md`: update the architecture router when an owner or flow changes | implementation | RF-CLOSE | Router rows re-read against source |
| RF-R6 | `AGENTS.md`: never claim a result that was not run; report the command and exit status | proof-only | RF-2, RF-3, RF-CLOSE | Lane result for Clippy; audit lists commands, statuses and skipped checks |

## Summary

| Task | Change | Files | Gate |
| --- | --- | --- | --- |
| RF-REG | Register the tasks, the owner decisions and the file hashes | tracker and memory | The user's instruction to proceed |
| RF-DOC | Thirteen corrections in two research documents | `.ontocode/research-reports/` | RF-REG |
| RF-2 | Delete `section_ranking` and its test; correct the `DocumentMap` comment; add one comment line above the pool block | `src/search/mod.rs` | RF-REG and owner decision 2 |
| RF-3 | Assert the `eval misses` output, then move `Miss` and `misses` to `eval::sweep` and use `FETCH` for the literal `50` | `src/cli.rs`, `src/eval/sweep.rs`, `src/eval/run.rs`, `tests/eval_run.rs` | RF-2; a non-empty fixture output |
| RF-CLOSE | Architecture router and audit | documentation | RF-3 and RF-DOC |

Order: RF-REG, RF-DOC, RF-2, RF-3, RF-CLOSE. RF-DOC touches no code and can run at any point after RF-REG. RF-3 follows RF-2 only so that its one full test run covers both code changes. The ids RF-0, RF-1 and RF-4 are withdrawn and are not reused.

## RF-REG: Registration And Baseline

- **Decisions:** the audit records the owner's answer to each row of Decisions And Non-Goals. A changed answer rewrites the affected card before that task starts.
- **Baseline:** the HEAD commit and the SHA-256 of the five files the code tasks touch. `src/cli.rs` already differs from HEAD, so the hashes are what lets a later reader tell this plan's hunks from the earlier ones.
- **Who:** a worker or the owner. The research coordinator that wrote this plan cannot edit `.memory-bank/`.

## RF-DOC: Corrections To The Research Documents

Review file, `.ontocode/research-reports/arxiv-graphrag-2026-review.md`. The first three rows were confirmed against the whole PDF; rows four and five by script; the rest are wording that the cited text does not support.

| Location | Current text | Correction |
| --- | --- | --- |
| Entry 86 | "LLM: index = no; query retrieval = no" | An LLM reranks the short list at query time: query retrieval = yes |
| Entry 145 | Hit@1 0.6972 against 0.6422 | 0.6972 includes LLM answer generation. Retrieval-only is 0.5596 and document-only is 0.4404, and every arm is dense |
| Entry 199 | "$2.30 to $24.94 per 5.64 MB corpus (section 4.2)" | That range is quoted from KET-RAG. The paper's own estimate is $19.49 to $38.99 |
| Summary, "still use an LLM for another indexing step [19, 184, 186]" | Lists 19 and 184 | No index-time LLM call was found for 19 or 184 in pages 1 to 12. Limit the list to 186, and align the "index = yes" lines of entries 19 and 184 |
| Negative-results note, "These come from entries scored 1 or 2" | The table cites entry 106 | Entry 106 has novelty 0; state the range as 0 to 2 |
| Entry 244 and the tangential table | "deterministic metadata" | An LLM is called per section at index time; drop "deterministic" |
| Ideas table, entries 239 and 247 | "Attack papers, not defences" | Entry 239 proposes defences, and its attack needs write access to the store |
| Summary, "The 2026 papers move away from the design that defined graph RAG in 2024-2025" | A trend across years | The review holds no earlier-year sample. State what the 2026 papers do |
| Summary, "graph RAG does not reliably beat plain retrieval" | "Plain" unqualified | In almost every cited comparison the plain arm is dense retrieval; say so |
| Ideas table, row "Threshold edges, not top-k edges" [270] | An idea that fits | Remove the row. OntoDoc derives no similarity edges, so there is nothing to threshold |

Eight other rows of the ideas table are reworded so each states what OntoDoc has today. The worker drafts the text from the audit and stays within the facts below. The rows on equal candidate budgets, the index as an attack surface and BM25 as the baseline keep their idea text.

| Ideas row | What OntoDoc has today |
| --- | --- |
| Structure and link graph with no extraction [19, 80, 129, 184, 200] | Built at index time from headings and resolved links (`src/graph.rs`) |
| Hub control around Personalized PageRank [122, 140, 215, 229] | Degree-normalised PageRank in `src/search/ppr.rs`; no paper tests more than that on a lexical arm |
| Fixed-depth, monotonic propagation [151] | PageRank stops at 100 iterations or a change below 1e-9 |
| Rank fusion in place of score fusion [84, 86, 228] | Min-max score fusion in `src/search/fusion.rs`; no paper compares the two on OntoDoc's arms |
| Render order under a byte budget [218, 221] | Assembly is rank-ordered and budgeted, with explicit truncation |
| Completeness reported with bounded operators [163] | Truncation flags on bounded responses (`README.md`, truncation contract) |
| Exact-quote citation checks [200] | `verify` compares an exact byte hash of the current file |
| Token co-occurrence graph [150] | None. The row stays an idea with its caution and claims no fit to an existing mechanism |

Architecture document, `.ontocode/research-reports/graphrag-2026-ideal-architecture.md`.

| Location | Current text | Correction |
| --- | --- | --- |
| Line 15 | "[19] ranks densely and calls an LLM at index time" | Remove the index-time LLM claim |
| P6 at line 73, and line 281 | [163] narrowed to "temporal operators" | Restore the paper's scope: completeness reported with every bounded operator |
| Line 227 | "$2.30 to $24.94 per 5.64 MB corpus [199]" | The same attribution as entry 199 |

Both files are untracked, so the worker copies them aside before editing; that copy is the rollback.

## RF-2: Search Module Cleanup

- **Delete:** `section_ranking` (`src/search/mod.rs:435`) and the assertion that uses it in `chunk_hits_map_to_sections_by_first_occurrence` (`:678`). If nothing else is left in that test, the test goes too. `rank_sections_to` stays the one implementation of "keep each section's first occurrence"; it carries the score and the depth cut that the evaluation needs.
- **Comment at `:36`:** today it reads "Chunk text historically selects the first matching document, while hierarchy scoring selects the last." The corrected text adds the missing condition: the two indices differ only when two documents share a path, `Generation::validate` refuses that for every stored generation (`src/store.rs:344`), and `SearchIndex::new` and `try_new` accept input that has not passed it. The map and both tests that build a duplicate path stay as they are.
- **Comment above `:349`:** one line saying that `Mode::Graph` and `Mode::Hier` fuse over this one pool, as the ADR requires (`.memory-bank/ADR_ONTODOC_SERVICE.md:181`). This is what remains of RF-1.
- **Before editing:** OntoIndex `impact` on `section_ranking`, then the source search. An empty graph result is not evidence; the search and the `--all-targets` Clippy build are.
- **Public surface:** `section_ranking` is `pub`. The package is not published and no caller was found, so the audit records the removal and no changelog entry is planned (decision 5).

## RF-3: Miss Collection In The Evaluation Module

- **Why:** `misses` (`src/cli.rs:1537`) scores a run, loads labels and indexes, ranks sections for four systems and calls `eval::sweep::attribute_miss` for each missed label. It is evaluation logic with no CLI concern in it, and its callee has no other caller.
- **Step 1, pin the output:** `tests/eval_run.rs:320` already runs `eval misses` on a dev fixture run. The worker captures that JSON on source with no RF-3 edit under `src/` and asserts it in full. The fixture is synthetic; no real query, label or result is read. If the output is an empty array the assertion proves nothing about the loop, and the task stops there.
- **Step 2, move:**
  - `Miss` and `misses` go to `src/eval/sweep.rs` as `pub(crate)`, next to `attribute_miss`. `misses` returns `Result<Vec<Miss>, EvalError>`; `run_eval` keeps its `?`, which converts through `AppError::Eval` (`src/cli.rs:469`).
  - `FETCH` (`src/eval/run.rs:32`) becomes `pub(super)` and replaces the literal in `Params::new(Mode::Lexical, 50)` at `src/eval/sweep.rs:299`. That call asks whether the section lies within the depth the evaluation fetches, which is what `FETCH` names.
  - The `.take(CUTOFF)` in the loop goes, because `rank_sections` returns at most `CUTOFF` sections.
- **Not merged:** `CANDIDATES`, `FETCH` and `MAX_TOP_K` stay three constants. They are the pool size per signal, the evaluation fetch depth and the service request cap. They are equal today and nothing requires them to stay equal.
- **Guards:** the admission refusals at the top of `run_eval`, including the arm that lists `Misses` (`src/cli.rs:1272`), and their order do not change.
- **Dirty file:** `src/cli.rs` holds uncommitted edits from other work. The worker keeps them and limits its hunks to the `Miss` struct, the `misses` function and the call in the `Misses` arm.

## RF-CLOSE: Router And Audit

- **Router:** the row for `src/eval/{gates,sweep}.rs` (`.memory-bank/project_architecture.md:94`) gains miss collection. The row for `src/cli.rs` (`:88`) still lists the `eval misses` subcommand, which the CLI still dispatches. The row for `src/search/mod.rs` (`:85`) describes the first-text and last-hierarchy map, which RF-2 leaves in place. Both are re-read and left alone if still true.
- **Audit:** every command with its exit status, the lane results, the removed function, and anything skipped.

## Deferred: Golden And Stage Extraction

RF-0 and RF-1 are withdrawn, not rejected for all time. If the owner later wants `search` split into named stages, a golden must land first, with these corrections to the revision 1 design:

- **Pool witness:** the generated corpus needs a chunk with zero BM25 for the query that is linked to a seed, and the self-check must find a hit with no `bm25_rank` on that corpus, not on `tests/fixtures/graph`.
- **Abstention:** the absent-term case sets a threshold, since `NoLexicalMatch` is returned only then.
- **Capture:** the golden is read with `include_str!` and compared line by line, as `fixture_edges_match_golden_list` does in `src/graph.rs`. The procedure that produces the first golden is written down before the worker starts.
- **Baseline:** file hashes, as RF-REG records them.
- **Gate:** the extraction touches no test and the golden passes unedited.

RF-4 would need the owner to accept that `SearchIndex::new` and `try_new` behave differently on a duplicate path, and two rewritten tests.

## Decisions And Non-Goals

Proceeding with this plan accepts the recommendations below unless the owner states otherwise.

| # | Decision | Recommendation | Alternative |
| --- | --- | --- | --- |
| 1 | Where the tracker lives | This file, as `docs/ONTOINDEX_IDEAS_PLAN.md` does. `.memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md` is near the 128 KiB parser ceiling | Cards in the implementation plan, after archiving closed ones |
| 2 | RF-2: delete `section_ranking` or reuse it | Delete | Change its signature to carry scores and a depth, and call it from `rank_sections_to`; more code for the same result |
| 3 | The first/last document pair | Keep it and correct the comment | Collapse to one index per path; public constructor behaviour on duplicate paths changes and two tests are rewritten |
| 4 | Golden and stage extraction | Withdraw both | Restore RF-0 with the corrected design, then RF-1 |
| 5 | Changelog entry for the removed function | None; the audit records it | One line under 0.1.1, with `CHANGELOG.md` added to RF-CLOSE's write set |

Considered and not selected:

- **Renaming `zero_weight_fusion_reproduces_b1_exactly` (`src/search/mod.rs:765`):** the test compares ids, and fused scores are normalised, so "exactly" overstates it. Nothing depends on the name.
- **Splitting `run_eval` (`src/cli.rs`, about 280 lines):** it is one `match` with fail-closed admission refusals at the top. Splitting it moves guards for a cosmetic gain.
- **Splitting `src/eval/run.rs` (4,836 lines) or `src/store.rs` (3,854 lines):** about half of each is tests. A split is churn across files that hold custody and publication code, with no behaviour to gain.
- **Removing `Assembled.truncated` and `Assembled.total_bytes`:** no production reader was found, but both are tested and small.
- **Removing the second tokenizer-input check:** `decode_verified_generation` calls `search::validate_input`, and the service then calls `SearchIndex::try_new`, which checks again. One pass is redundant on the service load path. `try_new` also receives unvalidated `build` output, so the check in the constructor is its own boundary, and no measurement shows the cost matters.
- **Performance edits:** the linear `pool.contains`, the linear `position` lookup for `bm25_rank`, the per-call document scan in `Generation::chunk_text`, and running PageRank when the weight is 0. Each is bounded by 50 or by the corpus caps, and none has a measurement behind it. Skipping PageRank at weight 0 would also change the pool.
- **One name for `truncated` and `omitted` across tools:** that is a response contract. It needs an ADR amendment and a named client workflow.

Not justified by the audit, and outside this plan:

- Reciprocal rank fusion, hub control, a damping change, threshold edges, a token co-occurrence graph or a seed-first render order.
- Any new field, tool, transport or dependency.
- Any change to the B3 pool, to a default, to the schema or to the tokenizer.
- Any access to evaluation payloads, any reuse of the held-out queries, and any edit under `tmp/release-010/` or `eval/`.

Each item of the first group is a ranking or contract change. Under `AGENTS.md` it needs its own ADR text, dev-split evidence and a fresh preregistered split, and the held-out split of the 0.1.0 experiment is consumed.

## Limits

- Nothing was built, tested or linted for this plan or its review. Every statement about current code comes from reading source, from `rg` and `sed` output quoted by explorers, and from the OntoIndex graph on 2026-11-15.
- Whether the dev fixture run in `tests/eval_run.rs` yields any miss is not known. RF-3 finds out in step 1 and stops if it yields none.
- The working tree has 53 changed paths, among them `src/{cli,graph,http,mcp,store,tiers}.rs`, `tests/{cli,http,mcp}.rs`, `Cargo.toml` and `Cargo.lock`. This plan was written against that tree, not against a release, and line numbers in `src/cli.rs` are the least stable in this file. Function names are the stable reference.
- OntoIndex reported uncommitted source files that its committed graph does not cover. Workers read source before editing and treat an empty graph result as no evidence.
- Whether the corpus walk can emit one relative path twice was not established. It does not affect any task, since revision 2 keeps the first/last pair.
- The lines of `.memory-bank/ADR_ONTODOC_SERVICE.md` and of the two research documents cited here were read for revision 1 and not re-read for revision 2.
- The field values of the tracker are those the validator accepted. The `lane-result` recipe was read in the justfile; it was not run.
- The source changes belong to the unreleased 0.1.1 line. The corrected 0.1.0 candidate under `tmp/release-010/corrected/` is a captured source inventory and is not rebuilt or edited by any task here.
- The plan adds no evidence about retrieval quality in either direction.
- The audit read some papers only to page 12. The RF-DOC rows that depend on that are marked in their text.
