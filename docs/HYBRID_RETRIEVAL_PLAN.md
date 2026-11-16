# Hybrid Retrieval Plan

Date: 2026-11-16
Status: Blocked. HY-0 and HY-1 are DONE; HY-2 and HY-3 wait for owner decisions recorded in the [ADR](../.memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md#decisions-required-at-acceptance).
Authority: proposed [hybrid retrieval ADR](../.memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md) revision 2, the [service ADR](../.memory-bank/ADR_ONTODOC_SERVICE.md) and `AGENTS.md`.
Requested by: user, "implement .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md", 2026-11-16.

## Goal

Carry the ADR's staged experiment as far as it can go without an owner decision: measure stage 0 from existing dev evidence, and add the pure cosine and rank-fusion functions that later stages reuse, unreachable from any search mode, CLI command or MCP tool.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: blocked
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: The owner answers ADR decisions 4, 5, 6 and 9 (label track, model and embedding tool with licences, over-limit policy, caps and artifact directory), with the retrieval research step 1 result in view; then reopen HY-2. Decision 2 is answered (no cross-language workload).
tasks:
  HY-0:
    title: Stage 0 reports the B1 dev miss share by stage from the pinned dev run, with Wilson intervals and oracle gain bounds
    status: DONE
    classification: proof-only
    type: measurement
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - tmp/hybrid-stage0/stage0.py
    - tmp/hybrid-stage0/summary.json
    allowed_write_set:
    - tmp/hybrid-stage0/stage0.py
    - tmp/hybrid-stage0/summary.json
    non_goals:
    - No new eval run, no test-split payload, label or result access, and no change to queries, qrels, config or custody.
    - No registered need threshold is set after seeing the numbers.
    required_validation:
    - python3 tmp/hybrid-stage0/stage0.py
    expected_evidence:
    - summary.json binds the input hashes of the dev run (meta 1b2ffb6e, B1.tsv 4b45fdf5, misses.json 0a07e0c4) and the dev queries and qrels, and reports p, its Wilson interval and the oracle gains.
    rollback:
    - Delete tmp/hybrid-stage0/.
    stop_conditions:
    - An input hash differs from the recorded dev run, or a test-split file would be read.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: implement .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md, 2026-11-16
        validation_summary: python3 tmp/hybrid-stage0/stage0.py exit 0; summary.json sha256 ea844cf2; B1 lexical-miss query share 18/82 = 0.220, Wilson 95% [0.144, 0.321]; details in the audit.
      role_results:
        coder-performance: null
        coder-manager: null
    dod_status: pass
  HY-1:
    title: Pure exact-cosine top-k and one-based RRF pass hand-computed checks and stay unreachable from Mode, CLI and MCP
    status: DONE
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: release
    depends_on: []
    owner_files:
    - src/search/dense.rs
    - src/search/mod.rs
    allowed_write_set:
    - src/search/dense.rs
    - src/search/mod.rs
    non_goals:
    - No change to SearchIndex search, Mode, Params, abstention, fusion::fuse, ranking, defaults, schema or dependencies.
    - No vector file format, loader, inference or eval arm.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib search::'
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-hy1-clippy-20261116 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-hy1-clippy-20261116
    expected_evidence:
    - Tests cover k = 60 agreement over rank 1 (2/110 against 1/61), k = 10 agreement at rank 11 and the exact tie at rank 12, an empty BM25 leg, repeated chunks, cut to depth, cosine ties by chunk id, scope before top-k, and refusal of zero dimension, size overflow, wrong length, non-finite and zero-norm rows and queries.
    rollback:
    - Delete src/search/dense.rs and its module line.
    stop_conditions:
    - The change needs an edit to an existing ranking symbol.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: implement .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md, 2026-11-16
        validation_summary: 'cargo test --lib search:: 29 passed exit 0 (5 new); cargo fmt --check exit 0; cargo clippy --all-targets -D warnings exit 0.'
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
    dod_status: pass
  HY-2:
    title: Stage 1 token census and a hash-bound vector bundle loader refuse identity mismatch, missing, duplicate or extra vectors and cap overruns
    status: BLOCKED
    classification: implementation-ready
    type: code
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: release
    depends_on:
    - HY-0
    - HY-1
    owner_files:
    - src/eval/run.rs
    - src/eval/sweep.rs
    allowed_write_set:
    - src/eval/run.rs
    - src/eval/sweep.rs
    non_goals:
    - No inference in the binary, no model download without the owner's selection, no production store or schema change.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-hy2-clippy-20261116 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-hy2-clippy-20261116
    expected_evidence:
    - A census lists token lengths per candidate tokenizer; focused tests cover bundle identity mismatch, cross-corpus isolation, missing, duplicate and extra vectors, dense_truncated counts, an over-limit query and cap refusal.
    rollback:
    - Revert only this task's hunks.
    stop_conditions:
    - The model, tokenizer, embedding tool or licences are unselected, or caps and the artifact directory are unset.
    known_blockers:
    - ADR decisions 5, 6 and 9 are open; no multilingual model or tokenizer is selected or installed.
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
  HY-3:
    title: Stage 2 dev comparison of X-BM25, X-DENSE and X-HYBRID-k for k in 10 and 60 on fresh reviewed queries
    status: BLOCKED
    classification: proof-only
    type: measurement
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    validation_tier: unit-fast
    depends_on:
    - HY-2
    owner_files:
    - src/eval/run.rs
    allowed_write_set:
    - src/eval/run.rs
    non_goals:
    - No held-out access, custody change, default change or confirmatory claim.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    expected_evidence:
    - A dev report with per-stratum effects and intervals, agreed-chunk and displacement counts, and the required test sample size.
    rollback:
    - Keep the report; revert only code hunks.
    stop_conditions:
    - The dev mean difference is at or below 0, or the required sample exceeds funded annotation.
    known_blockers:
    - ADR decision 4 is open; fresh reviewed queries and pooled labels do not exist. The cross-language stratum was withdrawn under decision 2 (ADR revision 3).
    closeout:
      role_results:
        coder-performance: null
        coder-manager: null
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| HY-R1 | ADR stage 0: miss share `p` by stage on the existing dev split | proof-only | HY-0 | `summary.json` and the audit |
| HY-R2 | ADR owner table: pure cosine top-k and rank fusion, unreachable from Mode, CLI and MCP | implementation | HY-1 | `cargo test --lib search::` |
| HY-R3 | ADR required checks: RRF arithmetic, ties, empty leg, depth cut, scope before top-k, malformed vectors | implementation | HY-1 | `search::dense::tests` |
| HY-R4 | ADR stage 1: token census, pinned vectors, bundle identity, `dense_truncated`, caps | blocked | HY-2 | Decisions 5, 6, 9 |
| HY-R5 | ADR stage 2: dev comparison and power | blocked | HY-3 | Decisions 2, 4 |
| HY-R6 | ADR stages 3 and 4: held-out custody transition and service-ADR amendment | non-goal | HY-3 | Decisions 3, 10 |

## Closeout

HY-0 and HY-1 are done; the [audit](../.memory-bank/audit_session-2026-11-16-hybrid-stage0.md) records commands, hashes and limits. The ADR stays Proposed. No ranking, default, schema, dependency or evaluation record changed.

On 2026-11-16 the [retrieval research audit](../.memory-bank/audit_session-2026-11-16-retrieval-research.md) split the B1 `lexical_miss` pairs: 1 of 21 (0.048, query-cluster 95% [0.000, 0.167]) shares no content term with its query, below that ADR's registered step 4 signal of 0.50. HY-2 and HY-3 keep their status and gates; the result is recorded for the owner's decisions. ADR revision 3 removed the cross-language stratum under decision 2.
