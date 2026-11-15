# Stdio Auto-Index Plan

Date: 2026-11-15
Revision: 2 (2026-11-15). Revision 2 adds the four PQ tasks from the documentation-quality probe review; the three AI tasks are unchanged.
Status: Closed (2026-11-15). All seven tasks are DONE. See the [audit](../.memory-bank/audit_session-2026-11-15-stdio-auto-index.md).
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (Accepted, revision 20, applied 2026-11-15 by AI-ADR from [ADR_STDIO_AUTO_INDEX.md](ADR_STDIO_AUTO_INDEX.md)) and `AGENTS.md`.
Requested by: owner, who selected option D ("refresh while serving") on 2026-11-15, instructed "update adr", and replied "proceed" to the drafted amendment.

## Goal

A local user starts `ontodoc serve --auto-index` in a working folder and queries its Markdown files without running `ontodoc index` by hand. A tool call that finds no index, or a stale one, rebuilds it through the existing build and publish path, waits, and answers from the new generation.

The flag is off by default and exists for stdio only. No tool, request field, response field, schema version, ranking input or dependency changes. The plan makes no claim about retrieval quality.

## Current Structure

Read from source on 2026-11-15. Line numbers move; function names are the stable reference.

| Symbol | File | Role today |
| --- | --- | --- |
| `Command::Serve` | `src/cli.rs` | Unit variant with no arguments |
| `main`, `serve` | `src/main.rs` | Match `Command::Serve` and call `mcp::serve_stdio(root)` |
| `serve_stdio` | `src/mcp.rs` | Builds `OntoDocServer::new(root)` with one permit |
| `OntoDocServer::blocking` | `src/mcp.rs` | Takes the permit, reads the active generation id, reloads when it changed, runs the handler. All five tools call it |
| `App::index` | `src/cli.rs` | `store::build_rooted` then `Store::publish` |
| `App::current_id`, `App::load_index_for` | `src/cli.rs` | Map a missing generation to `AppError::NoIndex` |
| `App::freshness` | `src/cli.rs` | Private; hashes every admitted file against the manifest |
| `Registry::discover_with` | `src/mcp.rs` | Builds the HTTP servers through `OntoDocServer::build` |

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
  AI-ADR:
    title: Apply the approved revision 20 amendment to the service ADR
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - docs/ADR_STDIO_AUTO_INDEX.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - docs/ADR_STDIO_AUTO_INDEX.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    non_goals:
    - No code, no rewording of the approved text, and no change to another ADR section.
    required_validation:
    - rg -n '[[:blank:]]+$' .memory-bank/ADR_ONTODOC_SERVICE.md .memory-bank/MEMORY.md docs/ADR_STDIO_AUTO_INDEX.md docs/STDIO_AUTO_INDEX_PLAN.md
    - rg -n '^Revision. 20|^## Opt-In Stdio Index Refresh|Revision 20 changes stdio only|rev 20' .memory-bank/ADR_ONTODOC_SERVICE.md .memory-bank/MEMORY.md
    - project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md
    expected_evidence:
    - The five edits listed in docs/ADR_STDIO_AUTO_INDEX.md are applied exactly; the ADR header names revision 20 and the new section sits immediately before the folder-scoped HTTP section.
    - MEMORY.md names revision 20 and stays at or below 100 lines.
    - The draft's status line says applied, and the trailing-blank search prints nothing.
    rollback:
    - Revert only this task's changes; both memory-bank files carry earlier uncommitted edits.
    stop_conditions:
    - The owner changes the wait-for-rebuild choice; the draft is edited first.
    - The ADR text at an edit site no longer matches the draft's context.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15, after approval of docs/ADR_STDIO_AUTO_INDEX.md
        validation_summary: Five edits applied by script with one-match checks at every site; trailing-blank search exit 1 with no output; revision 20 marker search found header line 4, section line 617, delivery boundary line 822 and MEMORY.md line 11; MEMORY.md 46 lines.
      role_results:
        coder-manager: null
    dod_status: pass
  AI-1:
    title: Add serve --auto-index to the stdio server
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: release
    depends_on:
    - AI-ADR
    owner_files:
    - src/cli.rs
    - src/main.rs
    - src/mcp.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/cli.rs
    - src/main.rs
    - src/mcp.rs
    - tests/mcp.rs
    non_goals:
    - No change to src/store.rs, src/http.rs, src/search/, src/eval/, Cargo.toml or Cargo.lock.
    - No new tool, request field, response field, schema version, dependency, thread, timer or watcher.
    - No change to behaviour without the flag, and no edit to an existing test's expectation.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib mcp::'
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-ai1-clippy-20261115 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-ai1-clippy-20261115
    expected_evidence:
    - OntoIndex upstream impact is recorded for OntoDocServer blocking, serve_stdio and Command Serve before the edit, and a HIGH or CRITICAL result is reported to the owner first.
    - With the flag off, every existing test passes unedited.
    - With no index, the first call builds one and answers; after a file edit the next call returns the new text with fresh true and a new generation id.
    - A failed rebuild with an active generation answers with fresh false; with no generation it returns the existing typed error; a second call inside 30 seconds does not rebuild.
    - A corrupt active generation keeps its typed error and is not rebuilt or deleted.
    - The permit is held through the rebuild and a concurrent call returns busy.
    - ontodoc serve-http refuses the flag, and the HTTP registry never enables it.
    - The generation id after an automatic build equals the id from ontodoc index on the same folder.
    - The lane result reports Clippy success.
    rollback:
    - Remove only this task's hunks; src/cli.rs, src/mcp.rs and tests/mcp.rs carry earlier uncommitted edits and are never restored from HEAD.
    stop_conditions:
    - The service ADR is not at revision 20.
    - The change needs an edit outside the write set, a new dependency, or work outside the call's permit.
    - An existing test fails with the flag off.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: OntoIndex upstream impact recorded before the edit with blocking MEDIUM and serve_stdio and Command LOW; cargo test --lib with the mcp module filter exit 0 with 24 passed; cargo test --test mcp exit 0 with 18 passed; cargo test --all-targets exit 0 with lib 240 passed and 4 ignored and every integration target passing; cargo fmt --check exit 0; Clippy lane ontodoc-ai1-clippy-20261115 lane-result exit_code 0 state terminal, submitted after the last source edit with no newer src or tests file.
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
    dod_status: pass
  AI-DOC:
    title: Document the flag and record the outcome
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - AI-1
    owner_files:
    - README.md
    - CHANGELOG.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/audit_session-2026-11-15-stdio-auto-index.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/audit_session-2026-11-15-stdio-auto-index.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    non_goals:
    - No runtime change and no retrieval quality claim.
    required_validation:
    - rg -n '[[:blank:]]+$' README.md CHANGELOG.md AGENTS.md .memory-bank/project_architecture.md .memory-bank/project_plan-current.md .memory-bank/audit_session-2026-11-15-stdio-auto-index.md docs/STDIO_AUTO_INDEX_PLAN.md
    - project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md
    expected_evidence:
    - README explains the flag, the store it writes, the rebuild wait, the failure behaviour and the benchmark-root warning; CHANGELOG has a 0.1.1 entry.
    - AGENTS.md service-boundary wording names the stdio exception and keeps HTTP indexing in the trusted CLI.
    - The architecture router rows for src/mcp.rs and src/cli.rs name the flag.
    - The audit records every command with its exit status, every skipped check and the limits.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Recorded evidence contradicts the documentation.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: README stdio auto-index subsection, CHANGELOG 0.1.1 entry, AGENTS.md stdio exception, architecture router rows and closed plan index row are in place; one README sentence corrected against src/cli.rs so only verify adds a per-call corpus pass; trailing-blank rg over the seven files exit 1 with no output; project_plan_validate status pass with 7 tasks and zero gate counters.
      role_results:
        coder-manager: null
    dod_status: pass
  PQ-1:
    title: Add query guidance to the search tool description and server instructions
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on: []
    owner_files:
    - src/mcp.rs
    allowed_write_set:
    - src/mcp.rs
    non_goals:
    - No change to ranking, tokenizer, defaults, request fields, response fields, schema or dependencies.
    - No default threshold and no numeric cutoff in any text; the probe's 0.3 is a guess from one query.
    - No statement that keyword queries or scope improve results; the text describes how matching works.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib mcp::'
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --test http
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-pq1-clippy-20261115 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-pq1-clippy-20261115
    expected_evidence:
    - OntoIndex upstream impact is recorded for the search tool handler and get_info before the edit.
    - The search description says matching is exact lowercase words with no stemming and no stop-word removal, suggests distinctive terms from the expected text, and names scope as a path-prefix filter.
    - The search description or instructions say abstention_score is always returned, that abstention happens only when the caller passes threshold, and that no validated cutoff exists.
    - The wording follows the texts in the PQ-1 design notes, and every existing test passes unedited.
    - The lane result reports Clippy success.
    rollback:
    - Remove only this task's hunks; src/mcp.rs carries earlier uncommitted edits and is never restored from HEAD.
    stop_conditions:
    - The service ADR fixes the tool description or instruction text, or an existing test asserts it.
    - The change needs an edit outside src/mcp.rs.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: OntoIndex upstream impact LOW with zero dependents for get_info and the search handler; ADR and tests fix no tool text; lib mcp 18 passed exit 0; test mcp 17 passed exit 0; test http 7 passed exit 0; fmt check exit 0; Clippy lane ontodoc-pq1-clippy-20261115 exit 0.
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
    dod_status: pass
  PQ-DOC:
    title: Document query guidance, the abstention score and link diagnostics for operators
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - PQ-1
    owner_files:
    - README.md
    - docs/DEPLOYMENT.md
    - CHANGELOG.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    allowed_write_set:
    - README.md
    - docs/DEPLOYMENT.md
    - CHANGELOG.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    non_goals:
    - No runtime change, no retrieval quality claim, and no recommended threshold value.
    - No edit to files in the ontocode repository; its corpus fixes are listed under Routed Outside This Repository.
    required_validation:
    - rg -n '[[:blank:]]+$' README.md docs/DEPLOYMENT.md CHANGELOG.md docs/STDIO_AUTO_INDEX_PLAN.md
    - rg -n 'abstention_score|OutsideRoot|MissingTarget|threshold' README.md docs/DEPLOYMENT.md
    - project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md
    expected_evidence:
    - The README tool table lists threshold, alpha and beta as search inputs, and the text says abstention_score is always present and any caller cutoff is advisory.
    - The README says the tokenizer does no stemming and removes no stop words, in the same terms as the PQ-1 tool text.
    - The README or deployment guide explains OutsideRoot and MissingTarget, including that a link starting with a slash is resolved from the corpus root, so an absolute filesystem path is reported MissingTarget whether or not that file exists.
    - The deployment guide says link diagnostics are expected when a corpus links to files outside its root, and are not index failures.
    - A short README subsection gives the three authoring conventions from the probe as untested suggestions.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Source read during the task contradicts a sentence to be written.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: README tool table lists threshold alpha and beta; new paragraphs on abstention_score and tokenizer matching; new Link diagnostics and authoring subsections; deployment guide Link diagnostics section; CHANGELOG 0.1.1 entry. Claims checked against tokenize.rs, abstention in search mod.rs, join in graph.rs and the Debug-formatted problem field in store.rs. Trailing-blank search exit 1 with no output; term search exit 0 with hits in README and DEPLOYMENT.
      role_results:
        coder-manager: null
    dod_status: pass
  PQ-SMOKE:
    title: Record the probe questions as a smoke set and rerun them once
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - PQ-1
    owner_files:
    - docs/SMOKE_PROBE.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    allowed_write_set:
    - docs/SMOKE_PROBE.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    non_goals:
    - No file under eval/, no qrels, no use of a benchmark corpus or of any held-out query.
    - No tuning of any parameter on these questions and no quality claim from them.
    - No reindex of the ontocode docs store unless status reports it stale, and no change to the installed binary.
    required_validation:
    - rg -n '[[:blank:]]+$' docs/SMOKE_PROBE.md docs/STDIO_AUTO_INDEX_PLAN.md
    - project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md
    expected_evidence:
    - docs/SMOKE_PROBE.md lists the eight probe questions, the stop-word-only query, and two added questions for each of gaps G1, G2 and G3, each with its expected source written before the rerun.
    - Every question is run once through search with default parameters against /opt/d1/_wrk/ontocode/docs, and the file records generation id, binary version, rank of the expected source or its absence, and abstention_score.
    - All outcomes are reported, failures included, beside the probe's original outcomes.
    - The file states that the set is a smoke check outside the benchmark, that its questions were written with knowledge of the documents, and that it supports no quality claim.
    rollback:
    - Delete docs/SMOKE_PROBE.md and revert this task's plan edits.
    stop_conditions:
    - The corpus generation differs from the probe's and the difference cannot be recorded.
    - Anyone proposes changing a ranking parameter because of these results; that goes to PQ-ARM.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: docs/SMOKE_PROBE.md fixed 15 questions and expected sources before the run, then recorded one CLI run per question with top-k 5 on generation 7bf5830a9b5a, the probe's own. Six of eight answerable questions found the expected source at rank 1; Q6 and A2 missed; S1 scored 1.10 as in the probe; out-of-corpus scores overlapped in-corpus ones. Trailing-blank search exit 1 with no output.
      role_results:
        coder-manager: null
    dod_status: pass
  PQ-ARM:
    title: Draft the tokenizer-variant arm proposal for the owner
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/TOKENIZER_ARM_PROPOSAL.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    allowed_write_set:
    - docs/TOKENIZER_ARM_PROPOSAL.md
    - docs/STDIO_AUTO_INDEX_PLAN.md
    non_goals:
    - No code, no tokenizer change, no evaluation run, no edit under eval/ and no ADR edit.
    - No dense or embedding leg; the ADR defers it.
    - No use of the consumed 0.1.0 test queries and no reading of held-out labels or results.
    required_validation:
    - rg -n '[[:blank:]]+$' docs/TOKENIZER_ARM_PROPOSAL.md docs/STDIO_AUTO_INDEX_PLAN.md
    - project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md
    expected_evidence:
    - The proposal states the hypothesis, the baseline B1, the variants considered (stop-word removal, stemming, both), the metric and the minimum effect, in the form the service ADR uses for a hypothesis.
    - It states the cost from source, namely a new tokenizer version, a changed generation id, a required reindex, and query-side and index-side tokenization kept identical.
    - It states what cannot be claimed today, namely that the 0.1.0 test split is consumed, a confirmatory claim needs fresh test queries, and the ADR sentence on new arms after H2 needs an owner reading because H2 ended without a decision.
    - It covers the abstention score for a query made only of common words, with the formula read from source, as a separate question from ranking.
    - It ends with the decisions the owner must make and marks itself Proposed.
    rollback:
    - Delete docs/TOKENIZER_ARM_PROPOSAL.md and revert this task's plan edits.
    stop_conditions:
    - Writing the proposal would need a held-out label, result or query.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply proceed open tasks on 2026-11-15
        validation_summary: docs/TOKENIZER_ARM_PROPOSAL.md marked Proposed; hypothesis row in the ADR form with baseline B1, variants T1 T2 T3, nDCG@10 and minimum effect 0.05; cost read from TOKENIZER_VERSION and IndexConfig validate in store.rs; abstention bound derived from bm25.rs and abstention in search mod.rs; consumed test split and undecided H2 stated; six owner decisions listed. No code, eval or ADR edit. Trailing-blank search exit 1 with no output.
      role_results:
        coder-manager: null
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| AI-R1 | Owner selection of option D and approval of the amendment | architecture-gate | AI-ADR | ADR header and section search, trailing-blank search |
| AI-R2 | Amendment: opt-in flag, stdio only, rebuild inside the call's permit | implementation | AI-1 | Focused and full suites, Clippy lane |
| AI-R3 | Amendment: failure behaviour, retry interval, no rebuild on load errors | implementation | AI-1 | Regression tests named in the task card |
| AI-R4 | `AGENTS.md`: bound work before spawning it; the permit stays with the work | proof-only | AI-1 | Concurrent-call test returns `busy` during a rebuild |
| AI-R5 | `AGENTS.md`: every changed non-trivial behaviour needs a focused regression check | proof-only | AI-1 | Tests in `tests/mcp.rs` and `src/mcp.rs` |
| AI-R6 | Amendment: operator documentation and service-boundary wording | implementation | AI-DOC | Trailing-blank search, tracker validation |
| PQ-R1 | Probe G1, option A: callers lack guidance on how matching works | implementation | PQ-1, PQ-DOC | Focused suites, Clippy lane, README search |
| PQ-R2 | Probe G2: `abstention_score` and the caller-supplied threshold are undocumented | implementation | PQ-1, PQ-DOC | README search |
| PQ-R3 | Probe G5: link diagnostics are read as defects | implementation | PQ-DOC | README and deployment guide search |
| PQ-R4 | Probe "Measurement": keep a smoke set outside the benchmark and report all results | proof-only | PQ-SMOKE | Recorded run in `docs/SMOKE_PROBE.md` |
| PQ-R5 | Probe G1, option C: stop-word and stemming arm | architecture-gate | PQ-ARM | Owner decision on the proposal |
| PQ-R6 | `AGENTS.md`: ranking changes need dev evidence and pre-registration; no claim without measurement | proof-only | PQ-1, PQ-SMOKE, PQ-ARM | Non-goals in each card |

## Summary

| Task | Change | Files | Gate |
| --- | --- | --- | --- |
| AI-ADR | Apply the five approved edits | service ADR, `MEMORY.md` | A session that may write tracking files |
| AI-1 | The flag, the refresh step and its tests | `src/cli.rs`, `src/main.rs`, `src/mcp.rs`, `tests/mcp.rs` | AI-ADR |
| AI-DOC | README, CHANGELOG, `AGENTS.md`, router and audit | documentation | AI-1 |
| PQ-1 | Query and abstention guidance in the tool text | `src/mcp.rs` | none |
| PQ-DOC | Operator documentation for guidance, abstention score and link diagnostics | README, deployment guide, CHANGELOG | PQ-1 |
| PQ-SMOKE | Smoke set recorded and rerun once | `docs/SMOKE_PROBE.md` | PQ-1 |
| PQ-ARM | Tokenizer-variant arm proposal | `docs/TOKENIZER_ARM_PROPOSAL.md` | none; the arm itself waits on the owner |

AI-1 and PQ-1 both edit `src/mcp.rs`, and AI-DOC and PQ-DOC both edit `README.md` and `CHANGELOG.md`. The loop runs one task at a time, so each starts from the other's result.

## Probe Review (2026-11-15)

Source: `/opt/d1/_wrk/ontocode/docs/review/ONTODOC_DOC_QUALITY_PROBE_2026_11_15.md` (96 lines), a hand-run probe of 8 questions against the 0.1.1 service over the ontocode `docs` folder. The report says it supports no quality claim, and this plan keeps that reading.

### What was checked against source

| Report statement | Check | Result |
| --- | --- | --- |
| The tokenizer has no stemming and no stop words | `src/search/tokenize.rs`, module comment | Confirmed |
| `abstention_score` is always returned | `SearchResponse` in `src/cli.rs` has the field with no skip attribute | Confirmed |
| Abstention happens only with `threshold` | `SearchRequest.threshold` is optional, default absent; ADR lines 1137–1139 at revision 19, 1217–1219 at revision 20 | Confirmed |
| Tokenizer settings are index-time and change the generation id | ADR lines 196–197 | Confirmed |
| A dense leg is deferred | ADR lines 1151–1152 at revision 19, 1231–1232 at revision 20 | Confirmed |
| Four `MissingTarget` links point at files that "do not exist" | `join` in `src/graph.rs` | Not established. A link starting with `/` is resolved from the corpus root, so an absolute filesystem path is reported `MissingTarget` whether or not the file exists |
| The 8 outcomes, scores and the 40 diagnostics | none | Not rerun in this session |

The README does not mention `threshold`, `abstention_score`, `OutsideRoot` or `MissingTarget` outside the evaluation results (search on 2026-11-15). No test or snapshot asserts the tool description or instruction text.

### Challenges

- **Sample:** one failing question (query 6) and one stop-word query carry G1. The author wrote the questions after reading the targets. This justifies guidance text and a proposal, not a ranking change.
- **Option A wording:** the report's "send two or three keyword queries" rests on that one case. PQ-1 states how matching works and leaves the choice to the caller.
- **Stop-word query and abstention:** the score is top BM25 divided by the summed idf of the query terms (comment on `SearchRequest.threshold`). Common words have low idf, so the ratio can be high. That is a property of the formula and belongs in PQ-ARM, not in a default threshold.
- **Option C timing:** the report says "after H2". H2 ended as exploratory with no decision, and the test split is consumed. Whether a new arm may be registered now is an owner reading of the ADR.
- **G3 `related` idea:** untested in the probe. No task; the smoke set adds two G3 questions first.

### Routed Outside This Repository

These fixes are edits to the ontocode documentation, which this plan does not own. They need a task in the ontocode repository.

| Probe item | Edit |
| --- | --- |
| G1, option B | One opening line per ADR and plan naming the questions it answers |
| G3 | `Amended by: <link>` at the start of a section an amendment changed |
| G4, query 8 | Write a local `docs/sandbox.md` or delete the one-line stub |
| G4, query 5 | Add a "why" line to the ADR status block; check `adr/0018` |
| G5 | Replace the four absolute-path links with repository-relative ones or remove them |

### Not adopted

- **Option D, dense leg:** deferred by the ADR.
- **A default threshold of 0.3:** the report itself calls the value a guess.

## PQ-1: Design Notes

The worker uses these texts. They may be shortened to fit line width but not changed in meaning.

- **Search description, added sentences:** "Matching is on exact lowercase words; there is no stemming and no stop-word removal, so prefer distinctive terms that the document is likely to contain. `scope` restricts results to paths with that prefix. `abstention_score` is always returned; results are withheld only when `threshold` is passed, and no validated cutoff exists."
- **Server instructions, added sentence:** "Search matches exact words without stemming; rephrase with terms from the expected text when results miss."
- The existing sentences stay. AI-1 adds its own sentence to the instructions when the flag is set; whichever task runs second keeps the other's text.
- The description is part of the `tools/list` response. The worker confirms that response stays inside the 128 KiB frame limit through the existing tests and adds no new test for wording.

## AI-1: Design Notes

These notes fix the shape so the worker does not design. They follow the amendment and add nothing to it.

- **Flag:** `Command::Serve` becomes a struct variant with one boolean `auto_index`. `main` reads it and passes it to `serve_stdio`. `serve-http` has no such argument, so clap refuses it there.
- **Server state:** `OntoDocServer` gains one optional field holding the time of the last failed rebuild. It is `None` unless the flag is set, and `Registry::discover_with` never sets it.
- **Refresh step:** inside the `spawn_blocking` closure of `blocking`, after the permits move in and before the existing index-slot block:
  1. If `App::current_id` returns `NoIndex`, rebuild.
  2. Otherwise load through the existing slot logic; a load error returns as today.
  3. If the loaded generation is stale, rebuild and reload through the same slot logic.
- **Rebuild:** one call to `App::index`. On error, record the monotonic time and continue with the generation already loaded, or return the error when there is none. Skip the rebuild while the last failure is under 30 seconds old.
- **Staleness:** `App::freshness` is private. Add one small `pub(crate)` method on `App` that returns whether a generation is fresh, and call it from the refresh step.
- **Logging:** one fixed code per failure to stderr. No path, query or operating-system text.
- **Instructions:** `get_info` adds one sentence when the flag is set, saying the server maintains its own index store.

Open point for the worker to confirm from source, not to decide: whether the handler can reuse the refresh step's freshness result without changing `App::search`, `App::read` or `App::status` signatures. If it cannot, the handler repeats the check, which the amendment allows only when a rebuild happened. In that case stop and report, because a second full hash per call on a fresh index would exceed the amendment.

## Limits

- Nothing was built, tested or measured for this plan.
- OntoIndex impact was not run on the three symbols AI-1 edits. `blocking` has five direct callers, the tool handlers, by a source read on 2026-11-15.
- The working tree has many uncommitted changes, among them `src/cli.rs`, `src/mcp.rs` and `tests/mcp.rs`. The plan was written against that tree.
- Every rebuild is a full one. Its cost on a large folder is not measured, and this plan does not measure it.
- The retry interval uses a monotonic clock in process memory. Its test needs a way to observe "no second rebuild" without sleeping 30 seconds; the worker reports the approach it used.
- The probe's outcomes were not rerun for revision 2. PQ-SMOKE is the first rerun.
- The ADR line numbers quoted in the probe review came from an explorer's read on 2026-11-15; the tokenizer comment, the `join` function, `SearchRequest` and `SearchResponse` were read directly.
