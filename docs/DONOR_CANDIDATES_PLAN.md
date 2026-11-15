# Donor Candidates C1 To C6: Implementation Plan

Date: 2026-11-14
Revision: 3
Status: Closed. Four authorized tasks completed; ranking candidates remain deferred.
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (Accepted, revision 18), `AGENTS.md`, `docs/ADR_DONOR_ADOPTION.md` (revision 5). The evidence/diagnostics ADR is proposed; this plan accepts nothing from it.
Source: candidates C1 to C6 in `.ontocode/research-reports/similar-projects-comparison.md` (revision 3).
Requested by: user, "make implementation plan for all 6 proposes", then "review and challenge, update doc".

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
  DC-REG:
    title: Accept ADR18 and register authorized citation preservation and census tasks
    status: DONE
    classification: architecture-paperwork
    type: tracking
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/DONOR_CANDIDATES_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    allowed_write_set:
    - docs/DONOR_CANDIDATES_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    non_goals:
    - No ranking, schema, evaluation inputs, reindex or preserved release artifact changes.
    required_validation:
    - git diff --check
    - project_plan_validate docs/DONOR_CANDIDATES_PLAN.md
    expected_evidence:
    - ADR18 records nullable tier, rank-order text fitting and related omitted counts; tracker validates.
    rollback:
    - Revert only this task's changes; preserve pre-existing work.
    stop_conditions:
    - Contradiction with another accepted ADR item.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks from docs/DONOR_CANDIDATES_PLAN.md on 2026-11-14.
        validation_summary: Strict project_plan_validate passed with four tasks; git diff --check exited 0. ADR18 accepts nullable tier and rank-order text fit with unchanged truncated semantics and related omitted count. Registration audit and memory routers updated; runtime implementation pending.
    dod_status: pass
  DC-1:
    title: Keep search citations under text pressure and count omitted related edges
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: release
    depends_on:
    - DC-REG
    owner_files:
    - src/cli.rs
    - src/mcp.rs
    - tests/mcp.rs
    - tests/http.rs
    - tests/cli.rs
    allowed_write_set:
    - src/cli.rs
    - src/mcp.rs
    - tests/mcp.rs
    - tests/http.rs
    - tests/cli.rs
    non_goals:
    - No ranking, schema, evaluation inputs, reindex or preserved release artifact changes.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --test http
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-dc1-clippy-20261114 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - git diff --check
    expected_evidence:
    - First oversized-code regression proves four ranked hits become an empty successful MCP result before implementation; all DC-1 synthetic acceptance cases pass after implementation.
    - Search keeps citations with nullable tier and fits text in rank order; related listed plus omitted equals incident count; fitting responses retain exact bytes; complete protocol bounds and parity pass.
    rollback:
    - Revert only this task's changes; preserve pre-existing work.
    stop_conditions:
    - First-step regression differs from the recorded empty successful MCP result; correct plan before behavior change.
    - Request, stored structure, response limit or another response bounding behavior must change.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks on 2026-11-14.
        validation_summary: Worker implemented the five-file scope. All required commands exited 0 (CLI unit14, CLI integration9, MCP17, HTTP7, all-targets, fmt, managed Clippy and diff check). Main-session review confirms shared rank-order fit and omission accounting. Independent review found no blockers (agent-results/01a1d8f8-bf94-7ab2-a3d8-818f38441911/2); exact-byte regression covers application JSON, with no separate historical MCP wire-byte fixture. Worker evidence agent-results/01a1d8f2-5639-7282-9107-1996104b2de5/1.
    dod_status: pass
  DC-3:
    title: Record descriptive neighbour cuts chunk sizes and ignored-link calibration outcome
    status: DONE
    classification: docs/design-only
    type: inventory
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: unit-fast
    depends_on:
    - DC-REG
    owner_files:
    - tmp/donor-candidates/census/
    - docs/ADR_DONOR_ADOPTION.md
    allowed_write_set:
    - tmp/donor-candidates/census/
    - docs/ADR_DONOR_ADOPTION.md
    non_goals:
    - No ranking, schema, evaluation inputs, reindex or preserved release artifact changes.
    required_validation:
    - python3 tmp/donor-candidates/census/census.py
    - git diff --check
    expected_evidence:
    - Saved script, SHA-256 and output record schema4 fresh store identities, omitted edge kinds, chunk byte thresholds and calibrated link diagnostics or explicit uncalibrated outcome.
    rollback:
    - Revert only this task's changes; preserve pre-existing work.
    stop_conditions:
    - Either store stale or schema other than 4; never reindex.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks on 2026-11-14.
        validation_summary: python3 tmp/donor-candidates/census/census.py and git diff --check exited 0. Saved script/output/SHA256SUMS and Decision3 record fresh schema4 stores, 10 cut links and45 mentions in Rust Book, none in OntoIndex; max chunks10269/33444 bytes. Link scanner uncalibrated (0/17 and9/10 diagnostics), additional edges unavailable, preparation match unconfirmed. No reindex or evaluation payload access.
    dod_status: pass
  DC-DOC:
    title: Document accepted response contracts and census outcome
    status: DONE
    classification: docs/design-only
    type: documentation
    validation_tier: unit-fast
    depends_on:
    - DC-1
    - DC-3
    owner_files:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    non_goals:
    - No runtime or ranking change
    required_validation:
    - git diff --check
    expected_evidence:
    - Public docs describe nullable tier, omitted edges and read prefixes; memory records actual validation and remaining limits.
    rollback:
    - Revert only task changes
    stop_conditions:
    - Evidence contradicts documentation
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Proceed open tasks on 2026-11-14.
        validation_summary: README/changelog describe nullable tier, related omitted counts and read prefixes; architecture and memory updated. References and structure checked; git diff --check exited 0. Audit records validation, baseline correction and census limits.
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| DC-R1 | User instruction to proceed; ADR18 | architecture-gate | DC-REG | Strict tracker and diff checks |
| DC-R2 | Citation preservation and omitted neighbour count | implementation | DC-1 | Synthetic acceptance cases and full required validation |
| DC-R4 | Public contract documentation | implementation | DC-DOC | References and whitespace checks |
| DC-R3 | Descriptive census without evaluation input access | implementation | DC-3 | Saved script, hashes, calibration and store freshness |

## Summary

All four authorized tasks are complete. The census recorded a failed link-scanner calibration; completion does not establish edge gains or preparation-generation matching.

| Task | Candidates | Completed outcome | Status |
| --- | --- | --- | --- |
| DC-REG | | Service ADR revision 18 and tracker registration | DONE |
| DC-1 | C1, C2 | Search preserves citations under text pressure; related reports omitted neighbours | DONE |
| DC-3 | C3, feeds C6 and DC-1 | Descriptive neighbour/chunk census; link scanning uncalibrated | DONE |
| DC-DOC | | Public contracts and census limits documented | DONE |

C4 stemming, C5 heading weight and exact phrase, C6 multi-hop traversal and the evaluation arm they share are deferred with reopen conditions. None has evidence behind it yet, and each ranking candidate needs a fresh test split to support a claim.

DC-1 needs no schema change, no reindex and no evaluation. It targets the unreleased 0.1.1.

## Review Of Revision 1

Two GPT-6-Luna explorers checked facts, one GPT-6.1-Sol explorer challenged the design, and the coordinator re-read the owners named under Evidence.

### Factual errors

| # | Revision 1 said | Source shows | Change |
| --- | --- | --- | --- |
| 1 | Revision 2 predicted `cap_exceeded` for an oversized rank-1 chunk | The first runnable regression proves four ranked hits, large first, become an empty successful MCP response. Once the last item is removed, the empty response fits | Corrected in revision 3 before implementation; preserve citations under text pressure |
| 2 | The response limit is 128 KiB | rmcp 1.7.0 `CallToolResult::structured` carries the JSON twice, as text and as structured content (explorer, `rmcp-1.7.0/src/model.rs:2864`). `structured` in `src/mcp.rs` measures that doubled result against 128 KiB, so the inner JSON starts losing items near 64 KiB, less escaping and envelope | All cases below are worked against the MCP form |
| 3 | A result without text can be fetched whole with `read` | `App::read` cuts text at `MAX_RESPONSE_BYTES / 2`, `bound` can halve it again, and `ReadRequest` has only `id`. No tool can deliver a chunk above about 64 KiB in full | Acceptance now expects a truncated prefix. A `read` continuation is a deferred row |
| 4 | At most 100 serialization passes | `structured` runs `serde_json::to_value` and a sizing pass on every loop, after `bound` has already looped | The new rule bounds the work of each pass; see DC-1 |
| 5 | Stdio and HTTP parity covers the output | `cli::run` pretty-prints the response after `bound` measured compact JSON, and nothing bounds that output (explorer) | Recorded as a limit and a deferred row |

### Design defects

| # | Finding | Change |
| --- | --- | --- |
| 6 | Clearing text from the lowest rank upward can remove every text. Worked cases are in DC-1 | Replaced by rank-order fit |
| 7 | `tier: "none"` had no representation. `SearchResult.tier` is the three-variant `TierLevel`, and the plan forbade touching `src/assemble.rs` | `SearchResult.tier` becomes `Option<TierLevel>`, serialized as `null`. The assembly enum is unchanged |
| 8 | The plan widened `truncated`, reversing accepted item 8 ("keeps meaning omitted results", revision 10), and called a row of a proposed ADR "taken" | `truncated` keeps its meaning. Withheld text is signalled per result. Revision 18 shrinks accordingly |
| 9 | DC-1 and DC-2 edited the same fields and the same method, and DC-2 invalidated DC-1's budget behaviour | One task |
| 10 | `omitted` on `search` would be 0 in almost every response once text is withheld before results are removed | Dropped; `related` keeps it |
| 11 | The census repeated known numbers, counted link syntax by filesystem existence, and used a "5% more" rule with no denominator | Narrowed; counts unique resolvable edges against indexed documents; no automatic rule |
| 12 | The stage 0 rules for stemming and heading weight do not discriminate. "Any shared stem" fires on incidental morphology in a long section, and heading overlap against the top-10 mean ignores score margins. Miss records hold no rank (`query_id`, `system`, `section_id`, `stage`) | Removed. An honest diagnostic needs rank changes from an in-memory variant, which is code |
| 13 | DC-ARM had no traced write set; DC-6 specified filters, depth and hop fields without a consumer | Both reduced to deferred rows |
| 14 | Server instructions (`get_info`), tool doc comments, release handling and the CLI were missing | Added to DC-1 and Release |
| 15 | Summary, option tables, decisions, execution and limits repeated the task cards | Shortened |

### Kept from revision 1

Proposed status with the ADR first; synthetic boundary fixtures; citation preservation as the goal; one rule for CLI and MCP; stdio and HTTP parity checks; no reindex for response-only work; a failing regression test as the first step; scripts and hashes saved beside census output; no reuse of the consumed test split.

## DC-REG: Record The Decision

- **Gate:** the user's instruction to proceed with this plan.
- **Who:** manager session; completed on 2026-11-14 with ADR18, strict tracker validation and whitespace checks.
- **Write set:** this file (tracker block), `.memory-bank/ADR_ONTODOC_SERVICE.md`, `.memory-bank/MEMORY.md`, `.memory-bank/project_plan-current.md`, `.memory-bank/project_pending-tasks.md`, `.memory-bank/project_architecture.md` after DC-1, one new `audit_session-*` file.
- **Content of service ADR revision 18, decision item 8:**
  - A `search` result may carry its citation and no text: `tier` is `null`, `text` is empty and `text_truncated` is false. This is the cap signal for withheld text.
  - Texts are kept in rank order. A text that does not fit in the complete response is withheld and later ranks are still tried.
  - Whole results are omitted only when the citations alone exceed the bound. `truncated` keeps its revision 10 meaning.
  - `related` carries `omitted`, the number of incident edges it did not list.
- **Validation:** `git diff --check`; strict plan validation of the registered tracker.
- **Stop:** the amendment contradicts another accepted item, or the owner wants `truncated` set for withheld text. The second case returns here for a revised flag rule before any code.

## DC-1: Keep Every Citation, Withhold Text That Does Not Fit (C1, C2)

- **Gate:** DC-REG.
- **Owner file:** `src/cli.rs`.
- **Write set:** `src/cli.rs`; `src/mcp.rs` (`structured`, tool doc comments, `get_info` instructions); `tests/mcp.rs`; `tests/http.rs`; `tests/cli.rs`. DC-DOC owns `README.md` and `CHANGELOG.md`.
- **First step:** completed before implementation. `oversized_first_hit_preserves_citations` uses an 84,000-byte indivisible code block and three small sections; direct ranking returns four hits with the large chunk first. MCP returns an empty successful response, so the desired four-result assertion fails. The earlier `cap_exceeded` prediction was incorrect and is withdrawn.

### Behaviour

1. **Never copy text that cannot fit.** A hit whose selected text is longer than `MAX_RESPONSE_BYTES / 2` is returned without text. `App::read` already cuts at that length, so no constant is added. Today `App::search` copies every selected text before bounding.
2. **Budget.** Under `budget_bytes`, a hit that the assembler gives no tier is returned without text. It is skipped today and sets `truncated`.
3. **Rank-order fit on overflow.** When the complete response does not fit, take every text out, then walk the results in rank order and put a text back only if the complete response still fits. A text that does not fit stays out and the walk continues.
4. **Last resort.** If the response does not fit with no text at all, remove trailing results as today and set `truncated`.
5. **`related`.** `omitted` counts incident edges absent from `neighbours`, whether cut by the 100-item cap or by the byte bound. Removal order is unchanged.

A result without text has `tier: null`, `text: ""`, `text_truncated: false` and its usual citation. A response that fits is untouched, and a `search` response gains no field.

`bound` and `structured` each measure a different serialized form, so step 3 must run in both against the form each one measures. They share one shrink step, and `SearchResponse` supplies the rank-order fit. The other four responses keep their `pop_item` behaviour.

### Why rank-order fit

The following design estimates assume about 0.5 KiB of metadata per result, no `budget_bytes` and the MCP form, which carries each text twice. These exact scenarios were not run and are not acceptance evidence. The observed baseline is the four-hit 84,000-byte fixture above, which returned an empty successful result. The 62 KiB chunk sits just under the step 1 threshold, so only the overflow rule decides it.

| Case | Pre-change estimate (unmeasured) | Lowest rank first (revision 1) | Largest text first | Rank-order fit |
| --- | --- | --- | --- | --- |
| 80 KiB chunk at rank 1, ten 1 KiB hits | Tail removal can empty the result | Eleven citations, no text | Rank 1 without text, ten with | Rank 1 without text, ten with |
| 62 KiB chunk at rank 1, ten 1 KiB hits | Rank 1 alone, or error | Eleven citations, no text | Rank 1 without text, ten with | Rank 1 without text, ten with |
| 80 KiB chunk at rank 3 | Ranks 1 and 2 | Text on ranks 1 and 2 only | Only rank 3 without text | Only rank 3 without text |
| 50 ordinary chunks | Top ranks, tail dropped | Top ranks keep text | The largest texts go first, rank 1 included | Top ranks keep text |

Lowest rank first fails when one large text sits high in the ranking. Largest first fails the ordinary case: it can strip rank 1 to keep rank 50, which is worse than today for the result that matters most. Rank-order fit has no failing case in the table.

It also bounds the work. After one full pass that detects the overflow, over at most 50 texts of 64 KiB, each trial serializes the texts already kept, which fit, plus one candidate of at most 64 KiB. Clearing from a full response serializes everything on every pass. The pass count stays at most one per result plus the removals of step 4.

The cost is that `bound` and `structured` call a shared step that receives their fit check, where they call `pop_item` today.

**Fallback** if that change turns out larger than one shared step: keep step 1 and clear text from the lowest rank upward inside `pop_item`. Record the 62 KiB case above as a known gap.

### Excluded

A request field; an implicit default budget; stepping an oversized result down to L1 or L0; per-cause counters; `omitted` on `search`; a change to `src/assemble.rs`, `verify` or `status`; a bound on CLI output.

### Acceptance, on synthetic fixtures

- The first-step fixture succeeds: rank 1 has `tier: null`, later results keep their L2 text, `truncated` is false.
- The same chunk at rank 3: every rank is present and only rank 3 lacks text.
- Two chunks of about 40 KiB at ranks 1 and 2 over MCP: rank 1 keeps its text, rank 2 does not, and the small results below keep theirs.
- `top_k` 50 of ordinary chunks over MCP: all citations are present, rank 1 has text, `truncated` is false.
- Citations alone over the bound, forced by long heading paths: trailing results are removed and `truncated` is true.
- A budget below the first L0: the result is present without text.
- A citation from a result without text passes `verify` as `valid`. `read` returns the text, or a prefix with `truncated: true` for a chunk above what `read` can carry.
- A `search` response that fits today, has no text above 64 KiB and skips no hit under a budget is byte-identical after the change.
- The existing MCP overflow tests assert `truncated: true` and a non-empty list for a large `search` (explorer, `tests/mcp.rs`). They are rewritten to assert withheld text, and the citation-overflow case above takes over the `truncated` assertion.
- `related`: `neighbours.len() + omitted` equals the incident edge count at 100, 101 and 150 edges, and when long heading paths force the byte bound.
- Boundary checks with text full of quotes, backslashes and newlines, with long metadata and with a long request ID: each complete frame is within the limit or is a typed `cap_exceeded`. Successful stdio and HTTP payloads are equal. The transport can still refuse a frame at the boundary, so success there is not promised.
- `bounded_responses_keep_native_truncation_bytes_and_order` still shows that `related` removes the same neighbours in the same order.
- Tool doc comments, `get_info` instructions, `README.md` and `CHANGELOG.md` describe `tier: null`, `related.omitted`, and that `read` may return a prefix.

### Validation, stop and rollback

- **Required validation:** `CARGO_BUILD_JOBS=4 cargo test --lib cli::`, `CARGO_BUILD_JOBS=4 cargo test --test cli`, `CARGO_BUILD_JOBS=4 cargo test --test mcp`, `CARGO_BUILD_JOBS=4 cargo test --test http`, `CARGO_BUILD_JOBS=4 cargo test --all-targets`, `cargo fmt --check`, `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`, `git diff --check`.
- **Stop:** the change needs a request field, a stored structure, a different response limit, or a behaviour change in another response's bounding.
- **Rollback:** revert the task. No stored data is affected.

### What changes for a client

- `tier` can be `null`. No output schema is published, but a client that parses `tier` as one of three strings must handle it. The string is asserted at `tests/mcp.rs:281` and `tests/cli.rs:175` (explorer).
- A budget that left a hit out and set `truncated` now returns that hit without text, with `truncated` false.
- The CLI prints text up to 128 KiB today. After step 1 it withholds text above 64 KiB, as MCP does.
- `related` adds about 15 bytes, so a response exactly at the limit can lose one more neighbour.
- Evaluation is unaffected: it calls `SearchIndex::search` directly (explorer, `src/eval/run.rs`, `src/eval/sweep.rs`).

## DC-3: Census Of Neighbour Cuts, Chunk Sizes And Ignored Links

The census is descriptive. It opens no task by rule; the owner reads the numbers and decides.

- **Gate:** owner agreement, and a session that can run commands. `status` takes no lock and writes nothing to a store (explorer, `src/cli.rs`, `src/store.rs`).
- **Inputs:** the stores under `tmp/corpora/rust-book/src/.ontodoc` and `tmp/corpora/ontoindex/.ontodoc`, and their Markdown files. No query, label, pool, run or result file.
- **Write set:** a new directory `tmp/donor-candidates/census/` holding each script, its SHA-256 and its output; the result under Decision 3 of `docs/ADR_DONOR_ADOPTION.md`.
- **Already known, not repeated:** the Rust Book store has 1,370 chunks, at most 132 neighbours and 2 chunks above 100 (`.memory-bank/audit_session-2026-11-13-graphrag-references.md`, with its script). Decision 3 already records fragment, `.html` and wiki-link observations.
- **Method:**
  1. Run `ontodoc --root <corpus root> status` on each store. Record the generation, schema version, `fresh` and `link_diagnostic_count`. Record whether the `ontoindex` generation matches the approved dev preparation; that match is unconfirmed.
  2. For each chunk with more than 100 incident edges, replay the order `App::related` lists them in. Record the count by kind and how many `CONTAINS`, `NEXT` and `LINKS_TO` edges fall after position 100.
  3. From chunk spans, record the largest chunk and how many exceed 32 KiB, 64 KiB and 128 KiB. This says how often DC-1 withholds text and how often `read` cannot return a chunk whole.
  4. Count ignored link destinations by class: no extension, directory, `.html` with no sibling `.md`, other extension. For each class, count the unique additional edges a rule would create, testing targets against the indexed documents. Calibrate the script first: it must reproduce the stored `link_diagnostics` counts per problem exactly, since diagnostics are recorded once per link occurrence. If it does not, report step 4 as uncalibrated.
- **Stop:** a store is stale or is not schema 4. Do not reindex these stores; the approved dev preparation pins generation identities.
- **Limits:** both are benchmark corpora and may not resemble a `docs_share` folder. Step 4 approximates the parser; images are skipped by the graph builder and reference links arrive as ordinary link events (explorer, `src/graph.rs`).

## Deferred

| Item | Why deferred | Reopen when | If reopened |
| --- | --- | --- | --- |
| C3 link resolution for extensionless and directory links | No count of edges gained | DC-3 step 4 is calibrated and the owner judges the gain worth a schema bump | Schema bump, reindex of every store and a graph-mode ranking change, as schema 4 was |
| C4 stemming | The donor ADR dropped it provisionally; the stage 0 rule could not tell help from noise | A fresh test split is funded and an evaluation arm exists | Emit the stem in addition to the whole word. Name the language; the tokenizer is Unicode-generic. Promotion needs tokenizer version 2 and a reindex |
| C5 heading weight | Same | Same | An integer repeat of heading-path tokens in the in-memory index; the value 1 reproduces B1 exactly. No stored field |
| C5 exact phrase | Postings hold a document and a term frequency, no positions (explorer, `src/search/bm25.rs`) | Heading weight has been measured first | Positions cost memory; this is the costlier variant |
| Evaluation arm for lexical variants | No candidate has evidence, and `src/eval/run.rs` is not traced for a write set | A fresh test split is funded for a ranking round | See the facts below |
| C6 `kinds` filter or `depth` on `related` | A client can already call `related` on a returned id | A named client workflow needs it. A cut link in DC-3 step 2 is not enough alone | Request field, so an ADR amendment |
| `read` continuation | A request field and a contract change | DC-3 step 3 finds chunks above 64 KiB in a served corpus | An offset on `ReadRequest`, by ADR amendment |
| Bound on CLI output | Present before this plan; the CLI is the trusted control plane | The owner wants printed output bounded | Measure the printed form in `cli::run` |
| `omitted` on `search` | Nearly always 0 after DC-1 | A client needs the count | One field and one increment |

Facts for the evaluation arm, from explorers:

- The systems are a closed set, B0 to B3, and run metadata enforces their identities and order (`src/eval/run.rs`). The sweep varies only the alpha and beta grids.
- The configuration hash covers the exact bytes of the config file, so a new field needs a new owner-approved preparation pin.
- `score` refuses a system below 0.95 judged@10 (`src/eval/gates.rs`). A new arm returns sections the pool never judged, so it probably needs a pool extension and new labels.
- No hook builds a second index over a generation today; `SearchIndex::indexes` hardcodes the tokenizer. An in-memory variant needs no tokenizer version bump, because the BM25 index is rebuilt at load.
- A claim needs a fresh test split and a new registration. Any surviving variant and any fusion change from GR-5 should share one registered family.

## Release

- The package and lock already read 0.1.1 unreleased (explorer); `CHANGELOG.md` has a 0.1.1 section. DC-1 adds to it and bumps nothing.
- Both preserved 0.1.0 packages, the measured binary and the evaluation custody stay untouched.
- `scripts/verify_release.py` does not depend on response fields (explorer).
- The changelog entry states the client-visible changes listed under DC-1 and makes no retrieval-quality claim.

## Decisions Needed

| # | Decision | Recommendation |
| --- | --- | --- |
| 1 | Accept `search` results without text (`tier: null`), with `truncated` unchanged. `AGENTS.md` asks for a typed error or an explicit flag when a cap is hit; `tier: null` on the result is that flag | Yes |
| 2 | Accept `omitted` on `related` only | Yes |
| 3 | Accept that the CLI withholds text above 64 KiB that it prints today | Yes; one rule for every transport |
| 4 | Approve the DC-3 census as a descriptive measurement | Yes |
| 5 | Fund a fresh test split for a ranking round | Not now; nothing above needs it |

## Non-Goals

- No new MCP tool, request field, session or transport.
- No change to ranking, defaults, the chunk budget, the schema or the response limit.
- No embedding, language model or outbound network on the retrieval path.
- No read of dev or test queries, labels or results, and no rerun of the consumed held-out attempt.
- No donor code. DC-1 is smaller as a change in its existing owner.

## Execution

- DC-REG, DC-1 and DC-3 are DONE; DC-DOC records final documentation and closeout.
- DC-1 is ready for one bounded worker after DC-REG: the owner file, write set, behaviour, tests and commands are stated. It is one task because every part touches the same bounding code.
- DC-3 is independent of DC-1 and can run before it. Its step 3 tells how much DC-1 matters on these corpora.

## Limits Of This Plan

- DC-1 synthetic validation passed and DC-3 ran; the closeout audit records commands and limitations. The worked table remains illustrative.
- The 64 KiB figure is approximate. The text copy inside the MCP result is escaped again, so the real point is lower and varies with content.
- How often responses overflow in practice is unknown. No request log exists.
- A text withheld by `bound` is not reconsidered by `structured`. This needs a compact response above 128 KiB, so at least two texts of tens of KiB.
- No task here is evidence that retrieval improves.

## Evidence

Re-read by the coordinator for this revision:

- `src/cli.rs`: `SearchResult`, `SearchResponse`, `RelatedResponse`, the `Bounded` trait and its implementations, `json_fits`, `bound`, `capped_iter`, `App::search`, `App::read`, `App::related`. OntoIndex places `Bounded` at lines 657 to 660 and the `SearchResponse` `pop_item` at 703 to 705.
- `src/mcp.rs`: `structured` (lines 246 to 261), `fits_json`, `cap_error`, `BoundedTransport`, the five tool handlers and their doc comments, `get_info` (lines 525 to 533).
- Service ADR decision items 8 to 11; `CHANGELOG.md` 0.1.1.

Reported by explorers and not re-read:

- The rmcp 1.7.0 constructor and the `Cargo.lock` pin.
- Test names and assertions in `src/cli.rs`, `tests/mcp.rs`, `tests/http.rs` and `tests/cli.rs`; that no test compares exact response JSON.
- `cli::run` and `src/main.rs` output; the absence of a published output schema.
- Edge ordering and diagnostics in `src/graph.rs`; `status` write safety in `src/store.rs`; the store paths and generation ids.
- Miss and run record fields, the judged gate, preparation hashing and `SearchIndex::indexes`.
- The package and lock version, and `scripts/verify_release.py`.

## Revision History

- Revision 3, 2026-11-14: user authorized execution; ADR18 and strict tracker registered. Runtime regression corrected the baseline from cap error to empty successful response. Public documentation is assigned to DC-DOC to satisfy worker write-scope rules.

- Revision 2, 2026-11-14: review by three explorers and coordinator re-reads. Corrected the oversized-chunk outcome, the effective MCP limit and the `read` limit. Merged DC-1, DC-2 and DC-DOC. Replaced lowest-rank-first clearing with rank-order fit. Kept the meaning of `truncated`; `tier` becomes nullable. Dropped `omitted` on `search`. Narrowed DC-3 and removed its automatic rules. Reduced DC-4, DC-5, DC-6 and DC-ARM to deferred rows. Added release handling.
- Revision 1, 2026-11-14: task cards DC-REG, DC-1 to DC-6, DC-ARM and DC-DOC; four corrections to the candidate table of the comparison report.
