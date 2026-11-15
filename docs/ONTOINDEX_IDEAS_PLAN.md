# OntoIndex Ideas: Implementation Plan

Date: 2026-11-14
Revision: 3
Status: Completed. All five tasks have evidence recorded in the execution and capacity audits; release and evaluation gates remain unchanged.
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (Accepted, revision 19), `AGENTS.md`, `docs/ADR_DONOR_ADOPTION.md`. The evidence/diagnostics ADR is proposed; this plan accepts nothing from it.
Source: the three candidates in `.ontocode/research-reports/ontoindex-ideas-review.md` (revision 1).
Requested by: user, "make implementation plan from proposed", then "review and challenge, update doc".

## Goal

Turn the three candidates of the OntoIndex ideas review into bounded tasks:

1. Refuse a generation whose graph edges are malformed, at publication and at load, with a tested way to recover.
2. Count what the link resolver does with every link of the two benchmark corpora, from a test-only harness.
3. Measure the per-request freshness walk on synthetic corpora near the configured budgets.

None of the three changes ranking, the stored schema, the tokenizer, the five MCP tools, their request fields or the CLI output. None is evidence that retrieval improves, and the plan claims none.

## Review Of Revision 1

Revision 1 was reviewed by three GPT-6-Luna explorers (store measurement, source trace, link classes), one GPT-6.1-Sol challenger, and coordinator re-reads of `src/store.rs`, `src/graph.rs` and `src/corpus.rs`. Nothing was built or run except a read-only script over the two stored artifacts.

### Factual errors

| Revision 1 said | What the source shows |
| --- | --- |
| A refused active generation "is recovered the way any corrupt active generation is" | `index` does not recover it. The generation id covers the configuration, the file manifest and the skips, so a rebuild has the same id and different bytes, and `publish_inner` refuses with `same-id artifact conflict`. Recovery is manual and was neither tested nor documented |
| The mitigation for a CRITICAL-impact change is that "the new code only adds refusals" | A refusal at load makes all five tools unavailable for that corpus until an operator repairs the store. That is the cost of the change |
| The `Over` case, 60 files plus 4, exceeds the byte budget | A file read reserves `max_file_bytes + 1` bytes before the open and refunds the unused part. With files of 2,096,128 bytes, 64 fit and the 65th is refused |
| `E90`, 90,000 files, sits just under the file and probe budgets | A third budget binds first. `directory_children_fd` charges three times the length of the directory path plus the name for every entry, against 16 MiB. With a root path of 60 bytes the walk stops near 75,000 files |
| `external` means a scheme URL or `mailto:` | `resolve` tests only for `://` anywhere in the destination and for a lowercase `mailto:` prefix. An email autolink, `tel:`, `MAILTO:` and `//host/path` fall into the non-Markdown branch, and `x.md?next=http://y` counts as external |
| `directory` covers a last segment of `.` or `..`, beside an `outside_root` problem | `..` and `dir/` return `Ignored` before `join` runs, so they never produce `OutsideRoot` |
| A copy that reproduces the pinned generation id is calibrated | The id does not cover edges or diagnostics. An equal id says nothing about link handling |
| `App::index` is the only production caller of `build_rooted` | `store::build` wraps it. `App::index` is the only caller outside `src/store.rs` |
| `git diff --check` validates the measurement tasks | `tmp/` is ignored by Git (`.gitignore:4`), so the check does not read those files |
| "The production resolver is the only classifier that cannot disagree with itself", "the work is linear", and two permits as the reason calls are refused | Three assurances with no measurement behind them. They are removed or restated as limits |

### Over-engineering

| Revision 1 | Revision 2 |
| --- | --- |
| Rule E6, canonical edge order, and rule D1, `documents` matching `files` record by record | Both dropped. No reader depends on edge order or on the manifest pairing, and `fixture_edges_match_golden_list` already pins the builder's order. D1 shrinks to "two documents share a path", as its own decision |
| The self-loop clause in E2 | Dropped. No reader was shown to misbehave on a self-loop |
| A permanent `link_outcomes` field in the `index` report with twelve classes, across `src/graph.rs`, `src/store.rs`, `src/cli.rs` and `tests/cli.rs` | A test-only harness in the `src/graph.rs` test module. The question it answers is asked once, for one deferred decision |
| OI-2M as a separate task, working on copies of both corpora | Folded into OI-2. The harness reads a corpus tree and writes nothing, so no copy is needed |
| OI-3 with six cases, five tools and two concurrency levels, gated on OI-1 | Three served cases, three tools and one concurrency-4 run. The dependency on OI-1 is removed |

### Forgotten

- A recovery path for a refused active generation, with a test and operator text. The same gap already exists for duplicate-id refusals.
- The availability trade-off as an owner decision: refuse and stop serving the corpus, or serve on a silently smaller graph.
- The precedent that lets a load refusal ship without a schema bump. ADR revision 17 added the `related` endpoint refusal with no schema change, and `load_rejects_duplicate_ids_even_with_matching_artifact_hashes` already models an older self-consistent artifact refused at load.
- Tests that isolate one rule each. "One tampered artifact per rule" did not say that the earlier checks must still pass, that the reason must match exactly, or that all three load adapters are covered.
- The build command and the binary hash in the measurement card.
- The memory cost of the new lookups at load, which stays unmeasured and is now listed as a limit.

### What held up

ADR before code, synthetic fixtures, fixed refusal reasons that carry no corpus text, publication refusal that leaves the store untouched, a measurement with no latency verdict, and no change to freshness or ranking. Both benchmark stores pass every revision 1 rule with zero violations, including the two rules revision 1 had left unchecked, so the stop condition on those stores is not expected to fire.

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
  OI-REG:
    title: Record service ADR revision 19 and register the OntoIndex ideas tasks
    status: DONE
    classification: architecture-paperwork
    type: tracking
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/ONTOINDEX_IDEAS_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas.md
    allowed_write_set:
    - docs/ONTOINDEX_IDEAS_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas.md
    non_goals:
    - No source, ranking, schema, tokenizer, evaluation input, reindex or preserved release artifact change.
    required_validation:
    - git diff --check
    - project_plan_validate docs/ONTOINDEX_IDEAS_PLAN.md
    expected_evidence:
    - ADR revision 19 states the edge invariant, the duplicate document path rule, the refusal policy and the recovery procedure exactly as the owner decided them; tracker validates; registration audit records each decision in Decisions And Non-Goals.
    rollback:
    - Revert only this task's changes; preserve pre-existing work.
    stop_conditions:
    - The amendment contradicts another accepted ADR item.
    - The owner declines the edge invariant; OI-1 is then removed before any code.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/ONTOINDEX_IDEAS_PLAN.md on 2026-11-14; OI-REG accepts recommendations 1–6.
        validation_summary: Strict project_plan_validate passed with five tasks and no errors; git diff --check exited 0. ADR19 records E1–E5, D1, refusal and conditional recovery; audit records all six decisions. Memory is 45 lines and 3523 bytes. Production work remains pending.
    dod_status: pass
  OI-2:
    title: Count link outcomes with a test-only harness and record both benchmark corpora
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on:
    - OI-REG
    owner_files:
    - src/graph.rs
    - tmp/ontoindex-ideas/link-outcomes/
    allowed_write_set:
    - src/graph.rs
    - tmp/ontoindex-ideas/link-outcomes/
    non_goals:
    - No change outside the test module of src/graph.rs, no report or stored field, no link resolution change, no reindex of the pinned stores, no evaluation input access and no decision on the deferred link candidate.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib graph::'
    - env ONTODOC_LINK_CENSUS_ROOT=tmp/corpora/rust-book/src CARGO_BUILD_JOBS=4 cargo test --lib graph::tests::link_outcome_census -- --ignored --nocapture
    - env ONTODOC_LINK_CENSUS_ROOT=tmp/corpora/ontoindex CARGO_BUILD_JOBS=4 cargo test --lib graph::tests::link_outcome_census -- --ignored --nocapture
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-oi2-clippy-fix-20261114 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - sha256sum -c tmp/ontoindex-ideas/link-outcomes/SHA256SUMS
    - git diff --check
    expected_evidence:
    - A synthetic test states a hand-written count per class in a comment and asserts it; the classes sum to the link events.
    - For each corpus the saved output shows distinct linked pairs equal to the stored LINKS_TO count (276 and 44) and problems equal to the stored link diagnostics (17 and 10), or the result is recorded as uncalibrated.
    - No production line of src/graph.rs changes; both outputs, the commands and SHA256SUMS are saved in the new directory.
    rollback:
    - Revert only this task's changes and remove the new directory.
    stop_conditions:
    - The harness needs a change to production code or to resolve.
    - A calibration equality fails; record uncalibrated and stop without adjusting the harness to fit.
    - Any command would write under tmp/corpora.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/ONTOINDEX_IDEAS_PLAN.md on 2026-11-14.
        validation_summary: Test-module census and hand-counted 20-event fixture added. Focused graph tests exited 0 (8 passed, 2 ignored); both ignored census commands exited 0 and matched 276/17 and 44/10. Formatting, SHA256SUMS and whitespace checks exited 0. Initial Clippy exited 101 for collapsible_if; one local correction and focused rerun passed; managed retry ontodoc-oi2-clippy-fix-20261114 exited 0. Evidence is tmp/ontoindex-ideas/link-outcomes/validation.json. Independent Luna read-only challenge passed with source-coverage limits; no production lines changed by this task.
    dod_status: pass
  OI-3:
    title: Measure the freshness walk on synthetic corpora near the configured budgets
    status: DONE
    classification: docs/design-only
    type: inventory
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - OI-REG
    owner_files:
    - tmp/ontoindex-ideas/capacity/
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas-capacity.md
    allowed_write_set:
    - tmp/ontoindex-ideas/capacity/
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas-capacity.md
    non_goals:
    - No source change, freshness change, budget change, permit change, verdict or edit of preserved release evidence under tmp/release-010.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo build --release
    - sha256sum target/release/ontodoc
    - python3 tmp/ontoindex-ideas/capacity/make_corpora.py
    - python3 tmp/ontoindex-ideas/capacity/measure.py --binary target/release/ontodoc --out tmp/ontoindex-ideas/capacity/results
    - sha256sum -c tmp/ontoindex-ideas/capacity/SHA256SUMS
    - git diff --check
    expected_evidence:
    - Saved generator and measurement scripts, their SHA-256, the binary SHA-256 and commit, and raw output for every pre-registered case including the cases that refused.
    - For every case the audit records the byte, entry and path charges computed from the generator's file list beside the observed result, and reports every listed metric with no pass or fail verdict.
    rollback:
    - Remove the new directory and the audit file.
    stop_conditions:
    - A case cannot be indexed; record the refusal as that case's result and never raise a cap.
    - The measurement would need a source change or an edit under tmp/release-010.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/ONTOINDEX_IDEAS_PLAN.md on 2026-11-14.
        validation_summary: Managed release build ontodoc-oi3-build-20261114 exited 0; binary SHA256 recorded. Generator, measurement, self-check, Python AST/whitespace, 39-file checksum verification and git diff --check exited 0. B16/B60/E50 published; EP index exited 1 cap_exceeded, retained as registered result; Over status/search refused and verify remained valid. Audit records complete computed demands, index/CLI RSS and timings, all 256 RPC calls, cumulative server RSS, provenance and limits. No latency verdict or source change.
    dod_status: pass
  OI-1:
    title: Refuse generations with malformed edges at publication and load, with a tested recovery
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: high
    validation_tier: release
    depends_on:
    - OI-REG
    owner_files:
    - src/store.rs
    allowed_write_set:
    - src/store.rs
    non_goals:
    - No schema bump, reindex, ranking change, edge cap, edge order rule, change to the publication protocol, to search graph construction, related, document lookup or unique id validation.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib search::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-oi1-clippy-20261114 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - CARGO_BUILD_JOBS=4 cargo run --quiet -- --root tmp/corpora/rust-book/src status
    - CARGO_BUILD_JOBS=4 cargo run --quiet -- --root tmp/corpora/ontoindex status
    - git diff --check
    expected_evidence:
    - Each accepted rule has a test that changes one field of one record in a published artifact, keeps every earlier check valid, and asserts the exact fixed reason at load; one rule is asserted through all three load adapters.
    - A generation with one injected bad edge is refused by publication on an empty store and on a store with an active generation, and CURRENT, HISTORY, the directories and the bytes are unchanged.
    - A recovery test refuses an active generation at load, shows that republishing is refused and leaves the store unchanged, removes that generation directory, republishes and loads.
    - Valid fixtures, including an empty generation, publish and load with unchanged artifact bytes and generation ids; both benchmark stores report their recorded generation ids through status; the existing related corrupt edge test passes unedited.
    rollback:
    - Revert only this task's changes; no stored data is affected.
    stop_conditions:
    - Either benchmark store is refused by the new check; record which rule fired and return to the owner before any change to the rule or the store.
    - Recovery by removing the generation directory does not work; return to the owner, because the alternative changes the publication protocol.
    - The change needs a schema bump, a source edit outside src/store.rs or a change to an existing test's expectation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User renewed proceed instruction after three worker prompt-shape rejections; execute locally.
        validation_summary: Only src/store.rs changed for OI-1; fixed E1-E5/D1 refusals and four regression tests cover all loaders, publication preservation, edge/duplicate-ID recovery and valid/empty byte roundtrips. Store tests exited 0 (49 passed, 1 ignored), cli 14 and search 25 passed; all-targets, fmt, managed Clippy ontodoc-oi1-clippy-20261114 and whitespace exited 0. Both pinned status commands exited 0, fresh schema4, original 375917fc and 4e546b16 generation IDs. OntoIndex scoped verify_diff PASS with capped symbol coverage; independent read-only review found no material gap. Pre-existing lock changes preserved. Added load cost remains unmeasured.
    dod_status: pass
  OI-DOC:
    title: Document the edge invariant, the link counts and the measured limits
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - OI-1
    - OI-2
    - OI-3
    owner_files:
    - README.md
    - CHANGELOG.md
    - docs/ADR_DONOR_ADOPTION.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - docs/ADR_DONOR_ADOPTION.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-ontoindex-ideas.md
    non_goals:
    - No runtime or ranking change and no retrieval quality claim.
    required_validation:
    - git diff --check
    expected_evidence:
    - README states the refusal of malformed edges with the recovery steps that the OI-1 recovery test proves, and the measured corpus sizes with the budget that bound each; the changelog records the refusal.
    - The link class counts are recorded under Decision 3 of docs/ADR_DONOR_ADOPTION.md; memory records actual validation and remaining limits.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Recorded evidence contradicts the documentation.
    - The package would be released before this task is DONE; a refusal must not ship without its recovery text.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/ONTOINDEX_IDEAS_PLAN.md; documentation closeout follows validated OI-1/OI-2/OI-3.
        validation_summary: README recovery matches tested removal/republish sequence; capacity table and budget refusals match saved results; changelog records unreleased refusal; donor Decision3 has calibrated classes; architecture and memory routers updated. All local documentation links and whitespace passed; MEMORY is 45 lines and 3590 bytes. git diff --check exited 0. No Rust changes in OI-DOC, no release or new quality claim. Execution audit records all task commands, exits, dispatch interruption and residual limits.
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| OI-R1 | User instruction to proceed; service ADR revision 19 | architecture-gate | OI-REG | Strict tracker and whitespace checks |
| OI-R2 | Candidate 1: edge invariant at publication and load, with recovery | implementation | OI-1 | One isolating test per rule, recovery test, unchanged fixture bytes, `status` on both benchmark stores |
| OI-R3 | Candidate 2: link outcome counts from the production resolver | proof-only | OI-2 | Hand-counted fixture, sum equality, calibration against the stored edge and diagnostic counts |
| OI-R4 | Candidate 3: freshness walk near the budgets | proof-only | OI-3 | Saved scripts, hashes, computed budget charges and raw output for every pre-registered case |
| OI-R5 | Public documentation of the outcome, including recovery | implementation | OI-DOC | References and whitespace checks |

## Summary

Execution outcome: OI-REG, OI-2, OI-3, OI-1 and OI-DOC completed in that order. [Execution audit](../.memory-bank/audit_session-2026-11-14-ontoindex-ideas.md) records commands and exit statuses; [capacity audit](../.memory-bank/audit_session-2026-11-14-ontoindex-ideas-capacity.md) records all case outcomes. The task-design prose below preserves the pre-execution rationale and acceptance conditions; statements about unrun work describe that registration state.

| Task | Candidate | Change | Gate |
| --- | --- | --- | --- |
| OI-REG | | Service ADR revision 19; tracker registration | The user's instruction to proceed |
| OI-2 | 2 | Test-only harness in `src/graph.rs`; counts for both benchmark corpora saved under `tmp/ontoindex-ideas/link-outcomes/` | OI-REG |
| OI-3 | 3 | Walk latency on synthetic corpora; no source change | OI-REG, and a session that can run a server |
| OI-1 | 1 | Edge checks in `Generation::validate` and a recovery test; `src/store.rs` only | OI-REG |
| OI-DOC | | README recovery text and measured sizes, changelog, link counts under Decision 3 of `docs/ADR_DONOR_ADOPTION.md`, architecture and memory | OI-1, OI-2, OI-3 |

Recommended order: OI-REG, OI-2, OI-3, OI-1, OI-DOC. OI-2 goes first because it is the cheapest task and touches no production code. OI-3 and OI-1 do not depend on each other. Running OI-3 first leaves numbers on record for the binary without the new check, so one `status` command repeated after OI-1 would show the cost of the check if anyone asks.

## OI-REG: Record The Decision

- **Gate:** the user's instruction to proceed with this plan. Proceeding accepts the recommendations under Decisions And Non-Goals unless the owner states otherwise.
- **Who:** the main manager session, within OI-REG's declared documentation write set.
- **Content of service ADR revision 19:**
  - Publication and load refuse a generation that breaks any accepted rule below, as `corrupt`. The rules restate what the builder produces. The stored schema stays 4, and a valid generation keeps its bytes and its id.
  - Why no schema bump: revision 17 added the `related` endpoint refusal without one, and duplicate ids are already refused at load in an older self-consistent artifact.
  - Refusal policy: a refused generation stops all five tools for that corpus until an operator repairs the store. The alternative, serving on a graph with edges silently dropped, is rejected.
  - Recovery: `index` cannot replace a refused active generation, because the rebuilt generation has the same id. The operator removes that generation directory and runs `index`. OI-1 tests this before the text ships.
- **Not in the ADR:** OI-2 and OI-3 change no product behaviour and need no ADR text. OI-REG registers them only.
- **Validation:** `git diff --check`; strict plan validation of the tracker.
- **Clippy command:** the plan validator requires Clippy to run through the managed compiler lane, so OI-1 and OI-2 use the `compiler-lane-detached` form that DC-1 used. The lane ids carry today's date; OI-REG renames them if the tasks run on another day.

## OI-2: Count Link Outcomes (Candidate 2)

- **Owner:** the test module of `src/graph.rs`. No production line changes.
- **Why:** the deferred decision on extensionless and directory links has no number. The DC-3 census tried a separate regex scanner and failed calibration. This harness calls the production `resolve` and checks its totals against the production `build`.
- **Why test-only:** the count answers one question for one decision. A permanent report field would be product surface with no reader after that decision. The field is listed under Deferred.

### What is uncounted today

`resolve` returns a target chunk, a problem or `Ignored`. `Ignored` covers four branches: a destination that contains `://` or starts with `mailto:`, a destination that is neither Markdown nor `.html`/`.htm`, an `.html` link with no sibling `.md`, and a target with no matching chunk. `build` also skips a link that lies in no chunk and drops a link to its own chunk. Only problems are recorded.

### Harness

1. One `#[ignore]` test, `link_outcome_census`, reads a corpus root from `ONTODOC_LINK_CENSUS_ROOT`. It walks, parses and chunks the way `fixture_graph` does, with `WalkConfig::default()` and `CHUNK_BUDGET`, and writes nothing.
2. It calls `build` once. It then visits every `Tag::Link` event itself, finds the chunk with `chunk_at`, calls `resolve`, and puts the link in exactly one class.
3. One private classifier function in the test module does the placing. A second, ordinary test covers it on a synthetic document.
4. Output is counts only, on stderr. No destination, path or heading is printed.

### Classes

The classifier decides in this order: the chunk lookup, then the value `resolve` returned, then the destination text for an ignored link. The text tests repeat the three string tests of `resolve` after the same fragment and query split and the same percent decoding, so an ignored link lands in the class of the branch that ignored it.

| Class | Definition |
| --- | --- |
| `outside_chunk` | No chunk covers the link; `build` skips it before resolution |
| `linked` | `resolve` returns another chunk. Distinct `(from, to)` pairs are also counted |
| `same_chunk` | `resolve` returns the linking chunk |
| `problem` | `resolve` returns a problem; split into `missing_target`, `missing_anchor` and `outside_root` |
| `external` | Ignored; the destination contains `://` or starts with `mailto:` |
| `ignored_markdown` | Ignored; the path is empty or ends in `.md` or `.markdown`, so the target has no chunk |
| `ignored_html` | Ignored; the path ends in `.html` or `.htm`, with no sibling `.md` or a target with no chunk |
| `not_a_path` | Ignored; an email autolink, a destination that starts with a URI scheme and `:`, or one that starts with `//` |
| `directory` | Ignored; the decoded path ends with `/`, or its last segment is `.` or `..` |
| `no_extension` | Ignored; the last path segment has no `.` |
| `other_extension` | Any other ignored destination |

Only the last three classes bear on the deferred decision. The others exist so that every link is accounted for and so that `tel:` and email links do not inflate `no_extension`. The last three also report the number of distinct destinations, because fifty links to one directory and fifty links to fifty directories lead to different decisions.

### Self-checks

- The classes sum to the link events.
- Distinct `linked` pairs equal the `LINKS_TO` edges of `build`, and `problem` equals its diagnostics.

### Corpus counts

- **Inputs:** the Markdown trees under `tmp/corpora/rust-book/src` and `tmp/corpora/ontoindex`. No query, label, pool, run or result file. The walk skips the hidden `.ontodoc` directory, and nothing is indexed or published.
- **Calibration:** the stored generations hold 276 and 44 `LINKS_TO` edges and 17 and 10 link diagnostics, all `MissingAnchor` for the Rust Book and all `MissingTarget` for the OntoIndex corpus. The harness must reproduce those four numbers. A mismatch means the harness or its configuration differs from the build that made the store; the result is then recorded as uncalibrated and the task stops.
- **Saved:** both outputs, the commands and `SHA256SUMS` under `tmp/ontoindex-ideas/link-outcomes/`. OI-DOC copies the class counts under Decision 3 of `docs/ADR_DONOR_ADOPTION.md`, beside the DC-3 result they replace. The plan validator keeps that file out of a worker task's write set.
- **Reference point:** the DC-3 census approximated the ignored destinations of the OntoIndex corpus as 52 directory, 12 without extension and 1 other. That scanner was uncalibrated.
- **Limit:** both are benchmark corpora and may not resemble a served `docs_share` folder. The counts decide nothing alone; the owner reads them and decides whether the deferred link candidate is worth a schema bump.

### Acceptance

- The synthetic test holds at least one link of every class, states the expected count of each in a comment and asserts it.
- `fixture_edges_match_golden_list`, `broken_links_are_diagnostics_with_spans` and `html_links_resolve_to_existing_markdown_sources_only` pass unedited, and the diff touches only the test module.

## OI-3: Measure The Freshness Walk Near The Budgets (Candidate 3)

The measurement is descriptive. It carries no pass or fail rule, because the ADR promises no wall time.

- **Gate:** OI-REG, and a session that can run a server.
- **Why:** `search`, `read`, `related` and `status` walk the corpus and hash every admitted file on each call. The only measured corpora are small: about 18 ms at p50 on 3.6 MB, against 0.24 ms for `verify`, which does not walk. The budgets allow 128 MiB of file bytes, 100,000 directory probes and 16 MiB of path charge per walk, and nothing has been measured near any of them. How admission behaves when a walk is long was not measured either.
- **Write set:** a new directory `tmp/ontoindex-ideas/capacity/` holding a corpus generator, the measurement script, their SHA-256 and all output; one audit file.
- **Script:** `tmp/release-010/evidence/capacity/measure.py` hard-codes its binary, commit, corpus list and output directory, and it is preserved release evidence. Copy it into the new directory and give the copy two arguments, the binary and the output directory. The original stays byte-identical (SHA-256 `ea00cd29…`).

### Walk accounting the cases are derived from

| Budget | Limit | Charge |
| --- | --- | --- |
| Bytes | 128 MiB | A file read reserves `max_file_bytes + 1` (2,097,153) before the open and keeps the file length plus one byte afterwards. The refusal depends on the reservation, so it comes earlier than the sum of file sizes suggests |
| Entries | 100,000 | One per directory entry returned, including `.` and `..`, plus one probe at the end of each directory |
| Paths | 16 MiB | Three times the length of the directory path plus one plus the name, for every entry. The directory path includes the corpus root, so the file ceiling depends on where the corpus is mounted |

The byte and path rows were re-read in `collect_rooted_file` and `directory_children_fd`. Whether the root is held in canonical absolute form was not traced, so the script records the root it passed and the length of its real path. `IndexConfig` has its own `max_files`, `max_chunks` and `max_file_bytes`; the script records their values for the measured commit, because one of them may bind before a walk budget does.

### Pre-registered cases

The generator is deterministic: fixed seed, fixed file names, short paragraphs under headings, no links.

| Case | Shape | What runs | Purpose |
| --- | --- | --- | --- |
| B16 | 16 files of 2,096,128 bytes, about 32 MiB | Index, serve, measure | Byte ladder, first step |
| B60 | 60 files of 2,096,128 bytes, about 120 MiB | Index, serve, measure | Near the byte budget. 64 such files fit and the 65th is refused |
| E50 | 50,000 files of about 1 KiB in 50 directories | Index, serve, measure | Many small files. The path charge is about 150,000 times the root length plus 14, which fits only when the root path is at most about 97 bytes |
| EP | Files of 64 bytes in directories of 1,000, with 1,000 more files than the path budget admits for the recorded root | `index` only | Tests the computed file ceiling. Expects a refusal near 75,000 files for a root of 60 bytes |
| Over | The largest published byte case, with files of the same size added after publication until the tree holds 66; the added files are removed when the case ends | Serve and measure, no indexing | The walking tools are expected to refuse. Records the error each tool returns and the time to refusal |

B60 is estimated at about 105,000 chunks and an artifact above 300 MiB, under the 512 MiB cap. That estimate was never run. If a case cannot be indexed, the refusal is the result for that case.

### Recorded for every case

- Machine, commit, tree state, binary SHA-256, the corpus root and the length of its real path.
- The byte, entry and path charges computed from the generator's own file list. A refusal is never used alone to infer which budget fired.
- Index wall time, artifact bytes, chunk count, peak resident memory of `index`.
- Wall time and peak resident memory of one CLI `status` run, which loads the generation and walks once in a fresh process.
- Peak resident memory of the server when the run ends.
- For `status`, `search` and `verify` over MCP: p50, p95 and maximum latency at concurrency 1 over 20 batches, as in the 2026-11-13 capacity audit. `verify` is the control. `search` also runs once at concurrency 4, with the count of `busy` refusals.

`read` and `related` are left out. They take the same walk as `status`, and their own work depends on the query and the section, which a synthetic corpus does not represent.

### What each result informs

| Result | Decision it informs |
| --- | --- |
| Walk time grows with corpus bytes and stays small beside a client's timeout | None. README gains the measured sizes |
| Walk time is long enough that concurrent calls are refused | The owner chooses between a documented corpus size ceiling, a permit change and an ADR question on freshness cost |
| B60 cannot be published | The practical ceiling is the artifact, chunk or tokenizer cap, below the walk budget. Record the largest case that publishes |
| EP is refused where the path charge predicts | README states the file ceiling as a function of the corpus root length. Whether the charge should include the root is a separate question for the owner |

A throttle, an mtime cache or any other weakening of freshness stays rejected. Changing the walk is an ADR decision taken after the numbers exist.

## OI-1: Refuse Malformed Edges (Candidate 1)

- **Owner:** `Generation::validate` in `src/store.rs`. It runs in `publish_inner` and in `decode_verified_generation`, which every load path reaches, including the evaluation capture path.
- **Impact warning:** OntoIndex rates a change to `Generation::validate` **CRITICAL**: 43 upstream symbols, 7 execution flows and 5 modules, because every publication and every load passes through it. A new refusal at load takes a corpus out of service. Both real stores were checked offline against every rule below and pass, and both are loaded with the new binary before the task closes.
- **How often it runs:** the server caches the loaded index per corpus by generation id (`src/mcp.rs`), so load validation runs once per process and once per generation change. It does not run per request.

### What is unchecked today

`Generation::validate` checks the configuration, unique section and chunk ids, manifest order and the generation id. It never reads `edges` or `documents`. `SearchIndex::from_indexes` drops a `LINKS_TO` or `NEXT` edge with an unknown endpoint without a trace, so graph mode would rank on a smaller graph. `App::related` refuses the same edge with a `corrupt` error at request time. Nothing in `src/search/` reads `CONTAINS` or `MENTIONS`.

### Rules

| Rule | A generation is refused when | Reader it protects |
| --- | --- | --- |
| E1 | An edge kind is not `CONTAINS`, `NEXT`, `LINKS_TO` or `MENTIONS` | Every reader that matches on kind |
| E2 | A `LINKS_TO` or `NEXT` edge has an endpoint that is not a chunk id | Graph ranking, which drops the edge silently; `related`, which fails at request time |
| E3 | A `NEXT` edge joins chunks with different `path` values | Graph ranking and `related` |
| E4 | A `MENTIONS` edge does not go from a chunk id to a target that starts with `code:` | `related` |
| E5 | A `CONTAINS` edge is none of: document path to section, section to section, section to chunk | `related` |
| D1 | Two `documents` records share a path | `document_map` in `src/search/mod.rs`, reported to pick different records for text and for hierarchy scoring |

E1 to E5 are one decision. D1 is a second, separable decision: it is not an edge rule, and its reader claim comes from an explorer and was not re-read.

### Behaviour

1. One new private method, called from `Generation::validate` after `validate_unique_ids`, so the lookups can assume unique ids.
2. One pass over the edges. Lookups are built once from borrowed ids: chunk id to `path`, the set of section ids and the set of document paths.
3. Each refusal is `StoreError::Corrupt` with a fixed reason string per rule. The reason carries no id, path, heading or code text, because ids are built from corpus text and `AGENTS.md` keeps document text out of logs and errors.
4. The pass visits each edge once, and the lookups hold one entry per chunk, section and document, all bounded by the artifact cap. They are a second set of hash tables beside those of `validate_unique_ids`. Their time and memory at load are not measured by this plan.

### Recovery

A generation that is refused at load and is the active one cannot be replaced by `index`. The rebuilt generation has the same id. `verify_dir` accepts the stored directory because its hash and length agree, the bytes differ, and `publish_inner` returns `same-id artifact conflict`. Had `verify_dir` failed instead, the answer would be `cannot repair active generation`. Either way the store stays as it is.

The procedure is to remove `.ontodoc/generations/<id>` by hand and run `index`. Nothing has run this yet, so OI-1 proves it in a test before README describes it. If it does not work, the task stops: the alternative is to let publication replace a refused active generation, which changes the publication protocol and needs its own ADR decision.

The README text lands in OI-DOC, because the plan validator keeps `README.md` out of a worker task's write set. The package is 0.1.1 unreleased, and OI-DOC stops if a release would come before it, so the refusal does not reach an operator without its recovery steps.

The same procedure applies to the existing duplicate-id refusal, which has no documented recovery today. The README paragraph covers both.

### Excluded

A schema bump or reindex; a cap on edge count; an edge order or duplicate edge rule; a self-loop rule; any change to `validate_unique_ids`, `publish_inner`, `SearchIndex::from_indexes`, `ChunkGraph::new`, `App::related` or `document_map`; checks that a chunk's `section_id` agrees with its `CONTAINS` edge or that record paths name a document.

### Acceptance, on synthetic fixtures

- **One rule per test case.** Each case takes a published valid artifact, changes one field of one record with `replace_artifact`, which keeps the hash and length consistent, and asserts that `load` returns `corrupt` with exactly that rule's reason. The change must leave every earlier check valid, so the asserted rule is the one that fired. The model is `load_rejects_duplicate_ids_even_with_matching_artifact_hashes`.
- **Branches.** E2 is covered for `from` and for `to`, and for `LINKS_TO` and for `NEXT`. E5 is covered for an unknown source, for chunk to section, and for document to chunk.
- **Adapters.** One rule is asserted through the ordinary, rooted and captured loaders, following `all_load_adapters_apply_preflight_before_typed_records`.
- **Publication.** A built generation with one injected bad edge is refused by `publish` on an empty store and on a store with an active generation. `CURRENT`, `HISTORY`, the directories and the bytes are unchanged. The model is `duplicate_ids_refuse_publication_without_mutating_the_store`.
- **Recovery.** A test makes the active generation refusable, asserts the load refusal, asserts that publishing the rebuilt generation is refused and leaves the store unchanged, removes the generation directory, publishes again and loads.
- **Valid input.** A fixture with all four edge kinds, including an unresolved `MENTIONS` target, publishes and loads. So does an empty generation. The existing round-trip and canonical-bytes tests pass unedited.
- **Existing tests.** `related_citations_resolve_by_kind_and_corrupt_edges_fail_closed` passes unedited; it injects its bad edges into the loaded index in memory, after the load check has run. `bounded_construction_matches_old_first_text_last_document_and_rankings` passes unedited; it builds its two same-path documents in memory and never publishes them.
- **Real stores.** `status` from the new build reports generation `375917fc…` for the Rust Book store and `4e546b16…` for the OntoIndex corpus store. `status` takes no lock and writes nothing to a store.

### Validation, stop and rollback

- **Required validation:** the commands in the tracker.
- **Stop:** a benchmark store is refused, recovery by directory removal fails, or the change needs a source edit outside `src/store.rs`. Do not reindex the benchmark stores: the approved dev preparation pins their identities.
- **Rollback:** revert the task. No stored data changes.

### Alternatives considered

| Option | Covers | Outcome |
| --- | --- | --- |
| Check in `Generation::validate`, at publication and load | Every tool, old artifacts and new ones | Chosen. It follows the `validate_unique_ids` precedent and needs one source file |
| Check in `SearchIndex::from_indexes` | Search only | Rejected. `related` and the evaluation capture path stay unprotected, and the check would sit in the ranking owner |
| Check at publication only | New generations | Rejected. An existing or hand-made store is never checked, and the builder cannot produce a violation, so the check would guard nothing real |
| A builder test only | The builder | Rejected. It protects nothing at run time |
| Let publication replace a refused active generation | Recovery without operator action | Kept as the fallback if directory removal fails. It changes `publish_inner` |

## OI-DOC: Documentation

- **Gate:** OI-1, OI-2 and OI-3.
- **Content:**
  - README states that a generation with malformed edges or duplicate ids is refused, and gives the recovery steps exactly as the OI-1 recovery test runs them.
  - README gains the measured corpus sizes with their latencies and the budget that bound each case.
  - The 0.1.1 section of `CHANGELOG.md` records the refusal and makes no retrieval-quality claim.
  - Decision 3 of `docs/ADR_DONOR_ADOPTION.md` gains the link class counts from OI-2, or the statement that they are uncalibrated.
  - `project_architecture.md` records the edge invariant under the store owner.
- **Stop:** a release of the package before this task is DONE.
- **Release:** the package is 0.1.1 unreleased. Nothing here bumps a version. Both preserved 0.1.0 packages, the measured binary and the evaluation custody stay untouched.

## Deferred

| Item | Why deferred | Reopen when | If reopened |
| --- | --- | --- | --- |
| `link_outcomes` in the `index` report | No reader beyond the one decision OI-2 serves | An operator needs link counts for a served corpus | Counters in `graph::build`, returned through `build_rooted`; an ADR note on the report |
| Edge order and duplicate edge rule | No reader depends on either; the golden list pins the builder's order | A reader starts to rely on sorted or unique edges | One more rule in the same method |
| Load cost of the edge check | Load runs once per process and per generation change | A corpus near the artifact cap is served, or startup time becomes a complaint | One CLI `status` run on the largest OI-3 case, before and after OI-1 |
| `read` continuation | A request field, so an ADR amendment. What `read` returns over MCP for a large section is inferred as about 32 KiB and was never run | A served corpus has sections above that size. The OntoIndex corpus has one of 107,222 bytes | First step: one characterization test in `tests/mcp.rs` with a section of about 100,000 bytes, asserting the bytes returned. Then an `offset` on the request and a `next_offset` computed after the final shrink |
| Front-matter text | Front matter lies outside every section, so a hit has no span to cite; making it citable is a schema bump and a ranking change | A real `docs_share` folder has titles that exist only in front matter | Schema and ADR review |
| Joined identifier tokens (`REQ-12`, `--max-items`) | Tokenizer version 2: every generation id changes and BM25 length normalization shifts | The ranking round that `docs/DONOR_CANDIDATES_PLAN.md` defers behind a fresh test split | One registered family with stemming and heading weight |
| Count of ignored links a candidate rule would resolve | No class counts yet | OI-2 shows a class large enough to matter | A second pass in the same harness |

## Decisions And Non-Goals

### Decisions needed

| # | Decision | Recommendation |
| --- | --- | --- |
| 1 | Accept rules E1 to E5 as a publication and load invariant with no schema bump | Yes. Both benchmark stores pass them, the builder cannot produce a violation, and revision 17 and the duplicate-id refusal are the precedent |
| 2 | When a stored generation breaks a rule, refuse it and stop serving that corpus until an operator repairs the store | Yes. The alternative is ranking on a graph with edges silently missing. The cost is an outage for that corpus, and no malformed edge has ever been observed |
| 3 | Accept manual recovery: remove the generation directory, then `index` | Yes, if the OI-1 recovery test passes. Otherwise the task returns to the owner |
| 4 | Accept D1, no two documents with one path | Yes, as a rule separate from the edges. Declining it leaves E1 to E5 intact |
| 5 | Count links with a test-only harness and add nothing to the `index` report | Yes. The report field stays deferred |
| 6 | Approve OI-3 as a measurement with no latency target | Yes. A target can be set once numbers exist |

### Non-goals

- No new MCP tool, request field, session or transport, and no new field in any CLI report.
- No change to ranking, defaults, the chunk budget, the tokenizer, the schema or the response limit.
- No weakening of freshness and no change to a budget or permit count.
- No change to the publication protocol.
- No embedding, language model or outbound network on the retrieval path.
- No read of dev or test queries, labels or results, and no rerun of the consumed held-out attempt.
- No donor code. `../ontoindex` is AGPL-3.0 and remains an idea source only.
- No edit of preserved 0.1.0 artifacts, including everything under `tmp/release-010/`, and no reindex of the pinned stores under `tmp/corpora/`.

## Limits Recorded Before Execution

Current limits: added invariant load cost remains unmeasured; capacity has no acceptance target; benchmark and synthetic corpora do not establish typical production behavior. OI-1 proved recovery and both pinned loads; OI-2 calibrated its census; OI-3 ran every case and traced canonical-root accounting. The following records the original uncertainties before those tasks ran.

- Nothing was built, tested, linted or measured for this plan, apart from a read-only script over the two stored artifacts. Every size, chunk count and latency for OI-3 is an estimate.
- No malformed edge has ever been observed. OI-1 is a guard against a builder defect or a hand-made store.
- The recovery procedure is derived from reading `publish_inner`. Whether publication succeeds after the generation directory is removed while `CURRENT` and `HISTORY` still name it has not been run.
- The claim about `document_map` behind D1 comes from an explorer and was not re-read.
- The time and memory that the new lookups add at load are unknown, and no task measures them.
- The byte and path arithmetic for OI-3 follows the source, and no case has been run. Whether the corpus root enters the path charge in canonical absolute form was not traced.
- The OI-2 classifier repeats three string tests of `resolve`. If `resolve` changes, the classifier must change with it; the two calibration equalities do not detect a wrong split among ignored links.
- The OntoDoc graph in OntoIndex is indexed at commit `ee511dc` of 2026-11-12. Line numbers from it can be off against the working tree; symbols are the reference.
- Both benchmark corpora may not resemble a served `docs_share` folder, and the synthetic OI-3 corpora resemble neither.

## Evidence

Re-read by the coordinator while writing revision 2:

- `src/graph.rs`: the module comment, `EdgeKind`, `Edge`, `Graph`, `build`, `structure_edges`, `ordered_chunks`, `chunk_at`, `resolve`, `join`, `percent_decode`, the test reference `old_build`, `synthetic_scan_timing` and `fixture_graph`.
- `src/corpus.rs`: `WalkBudget`, `with_local_limits`, `charge`, `bounded_dir`, `directory_children_fd`, `collect_rooted_file`, `capture_ignore_after_preflight`.
- `src/store.rs`, through OntoIndex: `build_rooted`, including the `generation_id(config, files, skipped)` call, and `publish_inner` up to the repair branch.

Read by the coordinator earlier in the review and carried over:

- `src/store.rs`: `verify_dir`, `Generation::validate`, `validate_unique_ids`, `load_id_rooted`, and the tests `load_rejects_duplicate_ids_even_with_matching_artifact_hashes`, `duplicate_ids_refuse_publication_without_mutating_the_store` and `all_load_adapters_apply_preflight_before_typed_records`.
- The header of `.memory-bank/ADR_ONTODOC_SERVICE.md`, `.ontocode/research-reports/ontoindex-ideas-review.md`, `docs/DONOR_CANDIDATES_PLAN.md`.
- OntoIndex impact, upstream: `Generation::validate` CRITICAL (43 symbols, 7 flows); `decode_verified_generation` CRITICAL (41 symbols, 8 flows, reaching all five MCP tools and the evaluation capture path).

Reported by explorers and not re-read:

- The store measurement. Rust Book: generation `375917fc…`, 112 documents, 547 sections, 1,370 chunks, 8,817 edges (1,917 `CONTAINS`, 1,258 `NEXT`, 276 `LINKS_TO`, 5,366 `MENTIONS`), 17 link diagnostics. OntoIndex corpus: generation `4e546b16…`, 118 documents, 1,072 sections, 1,275 chunks, 7,555 edges (2,347, 1,157, 44 and 4,007 in the same order), 10 link diagnostics. Both show zero violations of E1 to E5, of edge order, of duplicate edges and of the document manifest pairing.
- CLI `status` reaches `Generation::validate` through `load_index`, `load_id_rooted` and `decode_verified_generation`, takes no lock and writes nothing.
- The server caches the loaded index per corpus by generation id (`src/mcp.rs`, about lines 402 to 411).
- Nothing in `src/search/` reads `CONTAINS` or `MENTIONS`. `SearchIndex::try_new` validates tokenizer input bounds only.
- ADR revision 17 added the `related` endpoint refusal with no schema change. No README, docs or ADR text describes recovery from a refused generation, and no test covers it.
- `MARKDOWN_OPTIONS` enables YAML metadata, tables, footnotes, strikethrough and task lists. Link events come from inline links, resolved reference links and URI and email autolinks. An email autolink arrives without `mailto:`. Images are not link events.
- The capacity script's hard-coded inputs and its hash, `.gitignore:4`, the `compiler-lane-detached` recipe, and the plan template's tracker fields.

## Closeout

The plan closes when every task is DONE, the two audit files record each command with its exit status, and `MEMORY.md` routes to them. A declined decision removes its task or its rule before closeout and is recorded in the registration audit with the owner's reason.

## Revision History

- Revision 3, 2026-11-14: user authorized execution; OI-REG records ADR19 and registers all six accepted recommendations.

- Revision 2, 2026-11-14: review and challenge of revision 1. Corrected the recovery trace and made a tested recovery part of OI-1. Dropped edge order, the manifest pairing and the self-loop clause. Replaced the `index` report field with a test-only harness and folded OI-2M into OI-2. Re-derived the OI-3 cases from the walk accounting, added the path budget, cut the tools and concurrency levels, and removed the dependency of OI-3 on OI-1. Added the build and binary hash steps to the measurement card.
- Revision 1, 2026-11-14: task cards OI-REG, OI-1, OI-2, OI-2M, OI-3 and OI-DOC from the three candidates of the OntoIndex ideas review.
