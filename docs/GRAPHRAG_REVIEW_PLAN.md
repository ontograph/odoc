# GraphRAG reference review and repairs

Date: 2026-11-14
Status: Closed; GRR-1 and GRR-2 DONE
Authority: User request to review, challenge, choose robust proposals and proceed; service ADR17 and existing typed graph semantics.

## Goal

Keep unresolved mention IDs distinct from chunk endpoints during neighbour lookup, prove boundary behavior, and correct the deferred diagnosis proposal without adding retrieval features.

## Review and choices

Three GPT-6.1-Sol explorers reviewed correctness, performance and plan coverage using OntoIndex then current source. The graph omits dirty-source coverage, so source is authoritative. Existing citation constructors, bounded endpoint maps, explicit truncation and transport parity tests are sound. No measured speed improvement follows from their structure.

| Finding | Options | Selected action |
| --- | --- | --- |
| A legal `code:target.md#target~0` chunk ID can equal an unresolved inline-code target; raw incoming matching reports a false chunk relation | Reject colon filenames; change stored IDs/schema; distinguish edge target kinds during lookup | Kind-aware incidence in `App::related`, no identity or schema change |
| Outgoing malformed MENTIONS targets bypass endpoint checks | Leave unchecked; validate entire graph at load; validate the retained outgoing mention's existing `code:` syntax | Local prefix guard; valid lexical collisions remain unresolved mentions |
| First-100 exact prefix has only generic iterator coverage | Leave generic test; add actual related boundary test; add pagination | One related regression for 100/101 items, order, citation and truncation |
| Proposed GR-5 majority rule ignores loss magnitude, rank, grades and unjudged replacements | Keep arbitrary majority gate; invent weighted threshold; descriptive diagnosis with explicit adjudication | Withdraw automatic closure proposal; require a separately approved descriptive protocol before dev payload access |
| Lookup scans and response sizing could be optimized | Retain bounded implementation; add early-stop state; retain permanent lookup/adjacency maps | Retain implementation; no measured need for more state or shared serialization redesign |
| Documentation overstates escaped-heading fixture and memory contains stale pending wording | Preserve ambiguity; correct reference prose; rewrite historical evidence | Correct prose with dated audit; retain original execution facts |

The worst defect is typed-node confusion in a legal corpus, not citation construction. Forgotten coverage is collision behavior and the actual related item-cap boundary. Runtime overengineering is not evident; process repetition and speculative next features are the larger simplification opportunities. Two tasks suffice here.

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
  GRR-1:
    title: Separate unresolved mention targets from chunk incidence and prove item bounds
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on: []
    owner_files:
    - src/cli.rs
    allowed_write_set:
    - src/cli.rs
    non_goals:
    - No schema
    - ranking
    - request
    - freshness
    - stored graph or response-bound changes.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli:': null
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --test http
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-grr-clippy-20261114 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - git diff --check
    expected_evidence:
    - Legal code-prefixed file and matching inline code retain outgoing mention without citation and do not create incoming chunk neighbour; malformed outgoing mention fails corrupt.
    - Actual related results at 100 and 101 incident edges preserve exact prefix, citations and truncation; focused CLI and transport tests pass.
    rollback:
    - Revert only this task changes while preserving pre-existing dirty work.
    stop_conditions:
    - Stored ID change or ADR conflict required.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User request to select robust proposals and proceed, 2026-11-14.
        validation_summary: Fresh CLI14, MCP13, HTTP7 all passed exit0; cargo fmt --check, git diff --check and all-target Clippy exit0. Compiler receipts ontodoc-grr-{cli-fixed,mcp,http,clippy}-20261114 harvested with lane-result. Initial CLI exit101 was a fixture counting all graph edges instead of incident edges; one correction passed. GPT correctness review receipt agent-results/01a1d6d3-4be0-7100-8121-0ded2e08113b/3 has no blockers. OntoIndex impact LOW, exported API warning reviewed; verify_diff PASS limited to file scope. Twelve pre-existing source/test/package/authority hashes unchanged. Runtime file src/cli.rs hash 26ff922ad7c545c37773ccfd2acca1b33a1b6dcd9867faf040d4e85b0ca05337.
    dod_status: pass
  GRR-2:
    title: Record review decisions and withdraw unsupported automatic diagnosis closure
    status: DONE
    classification: docs/design-only
    type: documentation
    validation_tier: unit-fast
    depends_on:
    - GRR-1
    owner_files:
    - README.md
    - CHANGELOG.md
    - docs/GRAPHRAG_REFERENCES_PLAN.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-14-graphrag-review.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - docs/GRAPHRAG_REFERENCES_PLAN.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-14-graphrag-review.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    non_goals:
    - No evaluation payload reads
    - release publication or runtime changes.
    required_validation:
    - git diff --check
    expected_evidence:
    - Review findings and chosen/rejected options recorded; docs match typed incidence; exact validation outcomes and limits recorded; local links and strict plan validate.
    rollback:
    - Revert only this task documentation edits.
    stop_conditions:
    - Unapproved evaluation or publication decision required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User requested review consolidation and execution, 2026-11-14.
        validation_summary: README/changelog/architecture document typed mention incidence; unapproved GR-5 automatic majority closure withdrawn, descriptive protocol remains gated. Review audit records GPT findings/options, exact fresh commands/outcomes and limitations. git diff --check exit0; Python local-path/fence/whitespace checks on ten Markdown files, MEMORY bounds and validated src/cli.rs hash pass exit0. Strict project_plan_validate PASS two tasks. No new evaluation, release or performance claim.
    dod_status: pass
```

## Required Traceability

| Requirement ID | Requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| GRR-KIND | Existing graph MENTIONS targets are unresolved code nodes | implementation | GRR-1 | Legal collision and malformed-target regressions |
| GRR-BOUND | Preserve first-100 order and explicit truncation | compatibility-test | GRR-1 | Actual related boundary and transport tests |
| GRR-REVIEW | Compare and execute robust minimal proposals | implementation | GRR-2 | Audit, docs and strict validation |
| GRR-DEFER | No unapproved measurements or new public contracts | non-goal | GRR-2 | No evaluation payload access; GR-5 and REL-6 gates retained |

## Execution and limits

Manager maintenance may update this tracker and `.memory-bank/MEMORY.md`, `project_plan-current.md`, `project_pending-tasks.md` as task state changes. The prior four-task tracker remains closed. Compiler jobs run serially through the host launcher in this standalone checkout: `ontodoc-grr-cli-20261114`, `ontodoc-grr-cli-fixed-20261114` (one fixture correction), `ontodoc-grr-mcp-20261114`, `ontodoc-grr-http-20261114`, `ontodoc-grr-clippy-20261114`. No donor build or held-out access is authorized.

OntoIndex impact on `related` is LOW; safe-edit flags its exported API as DANGEROUS and graph coverage incomplete. Its signature stays unchanged. Source callers are CLI dispatch and MCP related handler. The audit review_diff facade refused scope all; test_gap found no indexed test evidence, so existing source tests and fresh command results govern validation.

GR-1 global topology checks, pagination, schema-5 link spans and permanent lookup indexes remain deferred. GR-5 needs an agreed descriptive protocol; REL-6 still needs destination/exposure/build-path decisions. No new performance or quality claim is authorized.

## Closeout

Both tasks completed through evidence-bound complete-local transitions. Fresh validation: CLI 14, MCP 13 and HTTP 7 tests passed; formatting, Clippy and whitespace checks exit 0. GPT post-edit review found no blockers. [Audit and exact receipts](../.memory-bank/audit_session-2026-11-14-graphrag-review.md) records the fixture correction, scope, rejected alternatives and remaining external gates.
