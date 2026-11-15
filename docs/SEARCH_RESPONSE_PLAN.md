# Search Response Plan

Date: 2026-11-16
Revision: 1
Status: Closed (2026-11-16). All three tasks are DONE. See the [audit](../.memory-bank/audit_session-2026-11-16-search-response.md).
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` revision 21 (item 8) and `AGENTS.md`.
Requested by: owner reply "1 2" on 2026-11-16, selecting proposals 1 (smaller MCP responses) and 2 (show when nothing matches) from the agent task test report.

## Goal

1. MCP search results currently carry their JSON twice, once as text and once as `structuredContent`. When both copies would exceed the 128 KiB frame, send the JSON once so that fewer search texts are withheld.
2. Tell agents which query words occur nowhere in the corpus, without adding a threshold or changing ranking.

This plan makes no claim about retrieval or answer quality.

## Findings Behind The Design

- `rmcp`'s `CallToolResult::structured` writes the same JSON as a text block and as `structuredContent`. A 10-result search frame was 39,800 bytes: 19,500 structured and 20,215 for the content array.
- The ontocode client sends only `structuredContent` to the model when it is present (`as_function_call_output_payload` in `ontocode-rs/protocol/src/models.rs`). For that client, the duplicate costs no model tokens. Its cost is frame room: at `top_k: 50` two searches withheld 22 and 24 of 50 texts over MCP, against none over the CLI.
- The MCP specification says a tool that returns structured content should also return the serialized JSON as text. Dropping the text would break text-only clients; dropping `structuredContent` only when both copies cannot fit keeps one complete copy in every result.
- `abstention_score` overlapped between answerable and unanswerable questions in the agent test, and tuning on those 12 questions is forbidden. `unmatched_terms` needs no threshold. In that test, it was non-empty for one search (N1) out of all arm B searches.

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
  SR-ADR:
    title: Record the response changes as service ADR revision 21
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
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    non_goals:
    - No change to another ADR section, threshold policy or evaluation rule.
    required_validation:
    - rg -n '[[:blank:]]+$' .memory-bank/ADR_ONTODOC_SERVICE.md .memory-bank/MEMORY.md
    - rg -n 'Revision. 21' .memory-bank/ADR_ONTODOC_SERVICE.md
    expected_evidence:
    - The ADR header names revision 21 and the owner reply; item 8 states the text-first result rule and the unmatched_terms field.
    rollback:
    - Revert only this task's hunks; MEMORY.md carries another session's uncommitted line.
    stop_conditions:
    - The ADR text at the edit site no longer matches.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply 1 2 on 2026-11-16
        validation_summary: Header revision 21 and two item 8 paragraphs added; trailing-blank search exit 1 with no output; revision search found the header and both paragraphs.
      role_results:
        coder-manager: null
    dod_status: pass
  SR-1:
    title: Send oversized MCP results once as text and add unmatched_terms to search
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: release
    depends_on:
    - SR-ADR
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - src/search/bm25.rs
    - tests/mcp.rs
    - tests/http.rs
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - src/search/bm25.rs
    - tests/mcp.rs
    - tests/http.rs
    non_goals:
    - No change to SearchIndex search, Ranked, abstention, ranking, defaults, schema, tokenizer or dependencies.
    - No threshold or numeric cutoff on unmatched_terms, and no withheld result because of it.
    - No outputSchema and no new tool or request field.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib mcp::'
    - CARGO_BUILD_JOBS=4 cargo test --test mcp --test http
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-sr1-clippy-20261116 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-sr1-clippy-20261116
    expected_evidence:
    - OntoIndex upstream impact for App search, SearchResponse and structured is recorded before the edit; the HIGH symbols SearchIndex search, Ranked and abstention are not edited.
    - A unit test shows both copies for a small result, text only with untruncated text when only one copy fits, and the existing truncation when one copy cannot fit.
    - Wire tests over stdio and HTTP accept a result without structuredContent and assert parity whenever it is present.
    - A unit test covers unmatched_terms order, headings, scope independence and an empty query.
    - The byte-exact search field-order test includes unmatched_terms.
    - The lane result reports Clippy success.
    rollback:
    - Revert only this task's hunks.
    stop_conditions:
    - The change needs an edit to a HIGH-risk ranking symbol or outside the write set.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply 1 2 on 2026-11-16
        validation_summary: Impact App search MEDIUM, SearchResponse LOW, structured LOW; all-targets exit 0 with lib 242 passed 4 ignored and every integration target passing; fmt check exit 0; Clippy lane ontodoc-sr1-clippy-20261116 exit_code 0 state terminal, submitted after the last source edit. Per-command detail is in the audit.
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
    dod_status: pass
  SR-DOC:
    title: Document the result rule and unmatched_terms and record the outcome
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - SR-1
    owner_files:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/audit_session-2026-11-16-search-response.md
    - docs/SEARCH_RESPONSE_PLAN.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/audit_session-2026-11-16-search-response.md
    - docs/SEARCH_RESPONSE_PLAN.md
    non_goals:
    - No runtime change and no retrieval quality claim.
    required_validation:
    - rg -n '[[:blank:]]+$' README.md CHANGELOG.md .memory-bank/project_architecture.md .memory-bank/project_plan-current.md .memory-bank/audit_session-2026-11-16-search-response.md docs/SEARCH_RESPONSE_PLAN.md
    - project_plan_validate docs/SEARCH_RESPONSE_PLAN.md
    expected_evidence:
    - README and CHANGELOG state the client fallback rule and what unmatched_terms does and does not mean.
    - The audit records each command with its exit status, the before/after frame measurements and the limits.
    rollback:
    - Revert only this task's changes.
    stop_conditions:
    - Recorded evidence contradicts the documentation.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: owner
        authorization_ref: owner reply 1 2 on 2026-11-16
        validation_summary: README paragraphs, CHANGELOG 0.1.1 entry, architecture router note, plan index row and audit written; trailing-blank search exit 1 with no output; plan validation result recorded in the audit.
      role_results:
        coder-manager: null
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| SR-R1 | Owner reply "1 2": smaller MCP responses | implementation | SR-ADR, SR-1 | Unit and wire tests, frame measurement |
| SR-R2 | Owner reply "1 2": show agents when nothing matches | implementation | SR-ADR, SR-1 | Unit test, rerun of arm B queries |
| SR-R3 | `AGENTS.md`: bounded complete responses and stdio/HTTP parity | proof-only | SR-1 | `tests/mcp.rs`, `tests/http.rs` |
| SR-R4 | `AGENTS.md`: no ranking change or threshold without dev evidence | proof-only | SR-1 | Non-goals; HIGH symbols untouched |
| SR-R5 | Operator documentation | implementation | SR-DOC | Trailing-blank search, plan validation |

## Limits

- The text-only fallback depends on clients reading the text block when `structuredContent` is absent. The ontocode client does this. Other clients were not tested.
- `unmatched_terms` reports only words absent from the whole corpus. In the agent test it was non-empty for one search out of all arm B searches. It cannot flag a question whose words all occur somewhere in the corpus.
- No agent rerun was made, so this plan has no evidence on answer correctness, calls or bytes read by agents.
