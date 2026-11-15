# OntoDoc Implementation Plan

Date: 2026-11-10
Status: 67 DONE; REL-6 blocked on HTTP parity/candidate and publication decisions; see manager_loop
Authority: `ADR_ONTODOC_SERVICE.md` (Accepted, revision 16); user requirement of
2026-11-08: new Rust MCP service for documentation in this repository, with
scientifically provable improvements. Reviews: `audit_session-2026-11-08-prior-attempt-review.md`,
`audit_session-2026-11-08-donor-ideas-review.md`, `audit_session-2026-11-08-plan-challenge.md`.

## Goal

Deliver the five-tool stdio service with explicit correctness/resource repairs,
then the accepted independent folder-scoped HTTP endpoints after their exposure
gate. Service repairs and operational documentation do not wait for annotation.
A single frozen, adequately
powered held-out run records H1 (sanity gate), H2 and H5 (confirmatory, Holm)
as pass, negative or inconclusive, with H3, H4 and E1 reported, whatever the
outcome. The current track uses model-judged labels and is exploratory only
(ADR revisions 13-15); confirmatory outcomes need human labels.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: in-progress
  selection_policy: active_next_task-first
  active_next_task: CORR-0
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: Owner resolves HTTP parity ADR conflict and authorizes a separate corrected candidate or explicit ADR exception; register scoped repair cards before runtime edits. REL-6 additionally needs validated candidate, destination/exposure and build-path decisions.
tasks:
  CORR-0:
    title: Preserve completed cards and register approved corrected-candidate work
    status: OPEN
    classification: architecture-paperwork
    type: documentation
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - .memory-bank/
    - docs/
    allowed_write_set:
    - .memory-bank/
    - docs/
    non_goals:
    - No runtime, frozen artifact, custody or publication changes.
    required_validation:
    - git diff --check
    expected_evidence:
    - Archived cards retain exact bytes; runtime state preserved; strict plan validates; owner proceed decision recorded.
    rollback:
    - Restore this task changes only.
    stop_conditions:
    - Historical state would be lost.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
  OD-0.0:
    title: OD-0.0 completed; original task card in archive/release-010-completed-cards.md#od-00
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on: []
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitignore
    - src/lib.rs
    - src/main.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitignore
    - src/lib.rs
    - src/main.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-00 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-00 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-00 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-00 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-00 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-00 (sha256 9547531a405b)'
    dod_status: pass
  OD-1.0:
    title: OD-1.0 completed; original task card in archive/release-010-completed-cards.md#od-10
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-0.0
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/corpus.rs
    - src/lib.rs
    - tests/fixtures/corpus/
    allowed_write_set:
    - src/corpus.rs
    - src/lib.rs
    - tests/fixtures/corpus/
    non_goals:
    - See archive/release-010-completed-cards.md#od-10 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-10 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-10 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-10 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-10 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-10; verbatim scalar SHA-256 84105a9a30bb4cb0df0230b1973e65af6e0cae529c1917caa2ac7362c7f4ab00.
    dod_status: pass
  OD-1.1:
    title: OD-1.1 completed; original task card in archive/release-010-completed-cards.md#od-11
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.0
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/parse.rs
    - src/chunk.rs
    - src/lib.rs
    - tests/fixtures/markdown/
    allowed_write_set:
    - src/parse.rs
    - src/chunk.rs
    - src/lib.rs
    - tests/fixtures/markdown/
    non_goals:
    - See archive/release-010-completed-cards.md#od-11 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-11 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-11 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-11 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-11 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-11; verbatim scalar SHA-256 7e8cf4743772b284dd61cbc6d29142bd086989f826e44f8479a2251bba1e9ff4.
    dod_status: pass
  OD-1.2:
    title: OD-1.2 completed; original task card in archive/release-010-completed-cards.md#od-12
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/tiers.rs
    - src/lib.rs
    allowed_write_set:
    - src/tiers.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-12 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-12 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-12 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-12 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-12 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-12; verbatim scalar SHA-256 33b3afd538ffab1bf054f5c6c3b521a8a7ff9c59bd76b07c185df4f6724c2513.
    dod_status: pass
  OD-1.3:
    title: OD-1.3 completed; original task card in archive/release-010-completed-cards.md#od-13
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/graph.rs
    - src/lib.rs
    - tests/fixtures/graph/
    allowed_write_set:
    - src/graph.rs
    - src/lib.rs
    - tests/fixtures/graph/
    non_goals:
    - See archive/release-010-completed-cards.md#od-13 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-13 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-13 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-13 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-13 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-13; verbatim scalar SHA-256 a50ec0a8de1969b38b6a7f6551d7891ce2eddcb0116af70919cc61c61648acad.
    dod_status: pass
  OD-1.4:
    title: OD-1.4 completed; original task card in archive/release-010-completed-cards.md#od-14
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.2
    - OD-1.3
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/store.rs
    - src/lib.rs
    allowed_write_set:
    - src/store.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-14 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-14 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-14 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-14 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-14 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-14; verbatim scalar SHA-256 e9f76e2b54ca87413b60d0cef10cf33e87212ccb4f23f4295a042ee9417d382c.
    dod_status: pass
  OD-1.5:
    title: OD-1.5 completed; original task card in archive/release-010-completed-cards.md#od-15
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.4
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/search/mod.rs
    - src/search/tokenize.rs
    - src/search/baseline.rs
    - src/search/bm25.rs
    - src/lib.rs
    allowed_write_set:
    - src/search/mod.rs
    - src/search/tokenize.rs
    - src/search/baseline.rs
    - src/search/bm25.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-15 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-15 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-15 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-15 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-15 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-15; verbatim scalar SHA-256 7fa3923af3e5b88ea6d71e81ae92b354d4567bff2f9c8bba1b5ca43922d57bf1.
    dod_status: pass
  OD-1.6:
    title: OD-1.6 completed; original task card in archive/release-010-completed-cards.md#od-16
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.5
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/search/ppr.rs
    - src/search/fusion.rs
    - src/search/mod.rs
    allowed_write_set:
    - src/search/ppr.rs
    - src/search/fusion.rs
    - src/search/mod.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-16 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-16 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-16 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-16 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-16 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-16; verbatim scalar SHA-256 475fed0a400d0a760f17408dbfb8cce0958de0b6fc5aed58b6dad9aaf94e7acb.
    dod_status: pass
  OD-1.7:
    title: OD-1.7 completed; original task card in archive/release-010-completed-cards.md#od-17
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.4
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cite.rs
    - src/lib.rs
    allowed_write_set:
    - src/cite.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-17 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-17 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-17 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-17 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-17 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-17; verbatim scalar SHA-256 66064a5bec70a22e25901c0e531529b0d34af51a24977a94b9ab2db6fc4b254f.
    dod_status: pass
  OD-1.8:
    title: OD-1.8 completed; original task card in archive/release-010-completed-cards.md#od-18
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.5
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/assemble.rs
    - src/lib.rs
    allowed_write_set:
    - src/assemble.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-18 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-18 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-18 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-18 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-18 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-18; verbatim scalar SHA-256 739cfa451ac96888960d0ebf718da124cb442ae0e6d3f1b4ffb6116cfb9eee91.
    dod_status: pass
  OD-2.0:
    title: OD-2.0 completed; original task card in archive/release-010-completed-cards.md#od-20
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.6
    - OD-1.7
    type: code
    validation_tier: integration
    impact_risk: low
    owner_files:
    - src/main.rs
    - src/cli.rs
    - src/lib.rs
    - tests/cli.rs
    allowed_write_set:
    - src/main.rs
    - src/cli.rs
    - src/lib.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-20 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-20 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-20 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-20 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-20 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-20; verbatim scalar SHA-256 3a0eaba1c31d909404d5dbd9aadd9198c9d77977fe704b25680bf79435d86978.
    dod_status: pass
  OD-2.1:
    title: OD-2.1 completed; original task card in archive/release-010-completed-cards.md#od-21
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.0
    - OD-1.8
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - src/lib.rs
    - tests/mcp.rs
    - Cargo.toml
    - Cargo.lock
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - src/lib.rs
    - tests/mcp.rs
    - Cargo.toml
    - Cargo.lock
    non_goals:
    - See archive/release-010-completed-cards.md#od-21 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-21 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-21 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-21 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-21 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-21; verbatim scalar SHA-256 9e8e0b763c3909e92d31cfeb71214732f56daae12c24a2cb76ceecc1f611ac8a.
    dod_status: pass
  EVAL-1.0:
    title: EVAL-1.0 completed; original task card in archive/release-010-completed-cards.md#eval-10
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-0.0
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/eval/mod.rs
    - src/eval/metrics.rs
    - src/eval/stats.rs
    - src/lib.rs
    allowed_write_set:
    - src/eval/mod.rs
    - src/eval/metrics.rs
    - src/eval/stats.rs
    - src/lib.rs
    non_goals:
    - See archive/release-010-completed-cards.md#eval-10 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-10 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-10 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-10 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-10 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-10; verbatim scalar SHA-256 0744b5ad8005c7ed8e71bd1ae9a3a2d318568272006a0191bcf4159c504feac3.
    dod_status: pass
  EVAL-1.1:
    title: EVAL-1.1 completed; original task card in archive/release-010-completed-cards.md#eval-11
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.0
    - OD-2.0
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    allowed_write_set:
    - src/eval/run.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    non_goals:
    - See archive/release-010-completed-cards.md#eval-11 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-11 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-11 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-11 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-11 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-11; verbatim scalar SHA-256 54077fb704190a1ea71e65e608ee733655c0d688f81400cf529a7b9799787214.
    dod_status: pass
  EVAL-1.2:
    title: EVAL-1.2 completed; original task card in archive/release-010-completed-cards.md#eval-12
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.1
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_gates.rs
    - tests/fixtures/eval_gates/
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_gates.rs
    - tests/fixtures/eval_gates/
    non_goals:
    - See archive/release-010-completed-cards.md#eval-12 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-12 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-12 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-12 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-12 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-12; verbatim scalar SHA-256 3948218febdce9fdf51c8578f763e817329dc74a25aaa46ad16e7fdae53dbfbc.
    dod_status: pass
  EVAL-1.3:
    title: EVAL-1.3 completed; original task card in archive/release-010-completed-cards.md#eval-13
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    - EVAL-1.4
    - EVAL-1.5
    - EVAL-1.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    - src/store.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    - src/store.rs
    non_goals:
    - See archive/release-010-completed-cards.md#eval-13 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-13 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-13 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-13 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-13 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed instructions 2026-11-11 (EVAL-1.3 slices 1-9); src/store.rs unused_unit one-line fix added to write set after repeated approval request and user proceed.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-13 (sha256 b5512fc122f2)'
    dod_status: pass
  EVAL-2.0:
    title: EVAL-2.0 completed; original task card in archive/release-010-completed-cards.md#eval-20
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    - OD-1.6
    - EVAL-1.3
    type: test
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - eval/corpora.json
    - eval/config.json
    - eval/gates.json
    - eval/splits.json
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    - eval/pools/
    - eval/preparation.json
    allowed_write_set:
    - eval/corpora.json
    - eval/config.json
    - eval/gates.json
    - eval/splits.json
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    - eval/pools/
    - eval/preparation.json
    non_goals:
    - See archive/release-010-completed-cards.md#eval-20 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-20 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-20 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-20 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-20 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "1-4" approving the spot-check sample and review digest 94aaeff0 (eval/audit_session-query-approval.md); owner decisions 2026-11-11'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#eval-20 (sha256 b5d2f12d9c27)'
    dod_status: pass
  EVAL-2.1:
    title: Given approved pools and FIX-14, when two independent model judges (GPT-6-Astra, claude-opus-5-5 high) blindly grade every pooled pair, then model-judged qrels and provenance pass qrels-stage preparation with agreement and trusted operator approval (ADR rev 13, exploratory only)
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-2.0
    - FIX-14
    - FIX-15
    type: test
    validation_tier: integration
    impact_risk: high
    owner_files:
    - eval/qrels.jsonl
    - eval/annotation.jsonl
    - eval/preparation.json
    - eval/dev-qrels.jsonl
    - eval/test-qrels.jsonl
    - eval/judging/
    - eval/config.json
    - eval/pools/pool.jsonl
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/splits.json
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    allowed_write_set:
    - eval/qrels.jsonl
    - eval/annotation.jsonl
    - eval/preparation.json
    - eval/dev-qrels.jsonl
    - eval/test-qrels.jsonl
    - eval/judging/
    - eval/config.json
    - eval/pools/pool.jsonl
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/splits.json
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    non_goals:
    - Model labels counted or described as human judgements.
    - Judging off-topic queries.
    - A judge seeing system identity, scores, ranks or the other judge's grade.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo run --release -- eval check-benchmark eval/config.json --stage qrels --query-review eval/query-review.jsonl --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256" --annotation eval/annotation.jsonl --annotation-notes eval/annotation.md --preparation-out eval/preparation.json --operator "$OPERATOR" --operator-evidence eval/audit_session-preparation.md
    expected_evidence:
    - Every pooled answerable pair has two raw model grades from blinded judge sessions shown only guideline, query and section text in a seeded order; final grade floor((a+b)/2).
    - annotation.md declares label_provenance model-judged, both model ids, guideline version, blinding and the adjudication rule; agreement kappa >= 0.6 with seeded query-cluster CI and prevalence.
    - Preparation records label_provenance model-judged; separate dev/test qrels; operator audits and pins the manifest hash.
    rollback:
    - Delete the listed files.
    stop_conditions:
    - Weighted kappa below 0.6; revise the guideline and regrade before any run.
    - A judge session saw rankings, scores or the other judge's grades.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "1 2 3" accepts preparation approval item 1 and continued release work.'
        validation_summary: 'Approval-only closeout: inline Python SHA-256 checks exit 0 for the exact approved manifest, nine bound inputs and four partition files; git diff --check exit 0. Historical qrels-stage preparation exit 0, judging provenance and agreement are preserved in eval/audit_session-preparation.md and eval/judging/. Required preparation command was not rerun: its create-new output already exists and is now pinned. No current build/test/measurement claim; no labels inspected. All payloads unchanged.'
    dod_status: pass
  FIX-14:
    title: FIX-14 completed; original task card in archive/release-010-completed-cards.md#fix-14
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-13
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/gates.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/common/mod.rs
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/common/mod.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-14 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-14 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-14 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-14 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-14 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "1-4" (decision 2: implement model-judged admission) and "continue"; ADR revision 13 owner decision 1b'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-14 (sha256 e40b3c7ecac8)'
    dod_status: pass
  FIX-15:
    title: FIX-15 completed; original task card in archive/release-010-completed-cards.md#fix-15
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-14
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/run.rs
    - tests/eval_gates.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_gates.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-15 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-15 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-15 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-15 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-15 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "B" (ADR rev 14 pool extension) and "proceed" (review round 8, ADR rev 15)'
        validation_summary: 'Local run exit0: cargo fmt --check; clippy --all-targets -D warnings; cargo test --all-targets 281 passed 0 failed. GPT-6-Astra review P1 (global extension check rejected partitioned preparation) fixed with regression pool_extension_spanning_both_splits_admits_preparation_and_dev_capture, which fails with the old check. Two pool generations byte-identical. Synthetic tests plus the real pool.'
    dod_status: pass
  EVAL-3.0:
    title: Given the dev split, when one `ontodoc eval run` of B0 and B1 executes, then `report.json` exists with every identity field, judged@10 for both systems, and a second run is byte-identical
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-2.1
    - REL-3
    - REL-5
    type: measurement
    validation_tier: integration
    impact_risk: low
    owner_files:
    - tmp/release-010/candidate/target/release/ontodoc
    allowed_write_set: []
    validation_workspace:
      root: tmp/release-010
      inputs:
      - candidate/target/release/ontodoc
      generated_outputs:
      - benchmark/
    non_goals:
    - The test split.
    - Parameter tuning.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-proof"'
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-proof-repeat"'
    - diff -r "$ONTODOC_BENCH/results/dev-proof" "$ONTODOC_BENCH/results/dev-proof-repeat"
    expected_evidence:
    - '`$ONTODOC_BENCH/results/dev-proof/report.json` exists and a separately named rerun produces byte-identical artifacts; existing output directories refuse overwrite.'
    - Trusted preparation has already exported/recaptured and approved the canonical external snapshot; audit binds preparation hash, complete benchmark/partition commitments and matching committed code/binary. Dev proof loads selected dev payloads only, never the full pool/provenance/test payloads. No scientific improvement follows from this instrument proof.
    - Run every command from a clean linked Git worktree at candidate commit F; record its absolute path, shared Git common directory and exact binary SHA-256. Benchmark and output paths are outside the checkout. Main-checkout documentation may advance independently; no separate clone or custody reset.
    rollback:
    - Preserve the recorded proof; delete only an explicitly owned disposable dev rerun.
    stop_conditions:
    - Report identity fields are missing or rerun output differs.
    - Canonical external benchmark snapshot and frozen binary paths are not pinned in the audit; EVAL-1.3 success/admission proof is missing.
    - Clean candidate worktree, matching audited binary and approved external preparation are unavailable.
    known_blockers: []
    closeout:
      role_results:
        coder-performance: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User 2026-11-13 proceed tasks; same-byte external export.
        validation_summary: Both dev runs and diff -r exit 0; seven byte-identical files, all metadata/hash identities checked, judged10=1 for four arms (B0 empty). Overwrite probe exit 1 with original hashes unchanged. External benchmark/eval/{audit_session-dev-proof.md,dev-proof-validation.json}; mirror audit_session-2026-11-13-dev-proof.md. No held-out access or custody writes.
    dod_status: pass
  EVAL-3.1:
    title: Given the dev split only, when PPR alpha, B3 beta and the abstention threshold are swept and the power calculation runs, then the external frozen export and fixed registration bind selected values, power, commit and hashes before test access
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.0
    type: measurement
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - tmp/release-010/candidate/target/release/ontodoc
    allowed_write_set: []
    validation_workspace:
      root: .
      inputs:
      - tmp/release-010/candidate/target/release/ontodoc
      - tmp/release-010/benchmark/eval/preparation-selected.json
      - tmp/release-010/benchmark/eval/config.json
      generated_outputs:
      - tmp/release-010/benchmark/eval/frozen-config.toml
      - tmp/release-010/benchmark/eval/results/freeze/
      - tmp/release-010/benchmark/eval/audit_session-preregistration.md
      - .git/ontodoc-eval/
    non_goals:
    - Any test-split evaluation.
    - Sweeping the chunk budget or any grid beyond the ADR.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval sweep "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-sweep/sweep.json"'
    - 'python3 -c ''import json, os; from pathlib import Path; n=json.loads((Path(os.environ["ONTODOC_BENCH"])/"results/dev-sweep/sweep.json").read_text())["power"]["required_answerable_test"]; assert isinstance(n, int) and 150 <= n <= 165, f"STOP before freeze: required answerable test count {n}"; print(n)'' '
    - '"$ONTODOC_FROZEN_BIN" eval check-benchmark "$ONTODOC_BENCH/config.json" --stage qrels --query-review "$ONTODOC_BENCH/query-review.jsonl" --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256" --annotation "$ONTODOC_BENCH/annotation.jsonl" --annotation-notes "$ONTODOC_BENCH/annotation.md" --preparation-out "$ONTODOC_BENCH/preparation-selected.json" --operator "$OPERATOR" --operator-evidence "$ONTODOC_BENCH/audit_session-preregistration.md"'
    - '"$ONTODOC_FROZEN_BIN" eval freeze "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation-selected.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --threshold "$ONTODOC_DEV_THRESHOLD" --repo . --out "$ONTODOC_BENCH/frozen-config.toml"'
    - '"$ONTODOC_FROZEN_BIN" eval check-frozen "$ONTODOC_BENCH/config.json" "$ONTODOC_BENCH/frozen-config.toml" --preparation "$ONTODOC_BENCH/preparation-selected.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --repo .'
    expected_evidence:
    - Grid a {0, 0.1, ..., 0.5}, beta {0, 0.1, ..., 0.5} (or the recorded fallback {0.2, 0.4}); threshold set for at most 10% false abstention on answerable dev queries.
    - Sweep using the original approved preparation; select only dev parameters in config.json. Then run the listed qrels-stage check explicitly as trusted-operator preparation, not ordinary dev; verify unchanged query/review/label/split bytes and reapprove the resulting manifest in the external audit. Update ONTODOC_PREPARATION_SHA256 from that approval before freeze/check. Ordinary dev never opens held-out data to refresh approval. Record exact config, complete partition/index commitments, both selected digests and h5-then-h2-v1 policy; never tune/rewrite held-out payloads.
    - required n = max(n_H2, n_H5, 150) is at most the preparation-bound test answerable count; exact frozen export and registration SHA-256 hashes are recorded in an audit session before EVAL-3.2.
    - Run every command from a clean linked Git worktree at candidate commit F; record its absolute path, shared Git common directory and exact binary SHA-256. Benchmark and output paths are outside the checkout. Main-checkout documentation may advance independently; no separate clone or custody reset.
    - Before freeze, the explicit power comparison succeeds against the currently pinned 165 answerable test queries. After selected config recapture, stop until the owner pins preparation-selected.json; ONTODOC_PREPARATION_SHA256 then means that second pin. The original preparation remains intact. Export path is absent and its external parent exists.
    rollback:
    - Preserve the write-once registration and frozen evidence. Superseding custody requires a fresh approved experiment and separately scoped transition; never delete it to retry.
    stop_conditions:
    - Any code path reads test-split qrels during the sweep.
    - Required n exceeds the answerable test queries; write and label more test queries (reopen EVAL-2.0/2.1) before freezing.
    - 'Second owner pin missing, recapture would overwrite a manifest, export parent missing, or power exceeds 165: stop before registration. Fallback needs owner decision; never delete custody to retry.'
    known_blockers: []
    closeout:
      role_results:
        coder-performance: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks approves selected pin 0d4ef095...f8c8c61 on 2026-11-13.
        validation_summary: Retained sweep/power/recapture exit0; this turn pin/identity checks, freeze and check-frozen exit0. Required150<=165, exact registration/export binding. Full hashes and commands in audit_session-2026-11-13-preregistration.md and external results/freeze/validation.json. No held-out attempt yet.
    dod_status: pass
  EVAL-3.2:
    title: Given approved frozen identity, when the sole held-out run executes, then accepted custody binds every reported hypothesis and the h5-then-h2-v1 recommendation without changing application defaults
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.1
    - OD-2.1
    type: measurement
    validation_tier: integration
    impact_risk: high
    owner_files:
    - tmp/release-010/candidate/target/release/ontodoc
    allowed_write_set: []
    validation_workspace:
      root: .
      inputs:
      - tmp/release-010/candidate/target/release/ontodoc
      - tmp/release-010/benchmark/eval/preparation-selected.json
      - tmp/release-010/benchmark/eval/frozen-config.toml
      - .git/ontodoc-eval/registration.json
      generated_outputs:
      - tmp/release-010/benchmark/eval/results/test-v1/
      - tmp/release-010/benchmark/eval/results/held-out-validation/
      - tmp/release-010/benchmark/eval/audit_session-held-out.md
      - .git/ontodoc-eval/attempt-started.json
      - .git/ontodoc-eval/attempt-result.json
    non_goals:
    - Re-running the test split with a changed config.
    - Reporting only passing hypotheses.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation-selected.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split test --confirm-test-split --out "$ONTODOC_BENCH/results/test-v1"'
    expected_evidence:
    - Identity fields match the frozen config; judged@10 >= 0.95 for every system; condensed nDCG' reported.
    - Pooled and per-corpus results and an audit recording the outcome and burned-split restrictions. Owner approved unavailable held-out per-stage miss attribution on 2026-11-13; record this limitation explicitly (audit_session-2026-11-13-held-out-readiness.md).
    - Report H1, both H2/H5 Holm/CI decisions, H3/H4/E1 and all limitations, regardless of gain. Freeze h5-then-h2-v1 before access; H5 pass recommends hier/frozen beta, else H2 pass graph/frozen alpha, else lexical/no-op. Under model-judged labels (ADR rev 13) H2/H5 report effects and CIs with role exploratory, no Holm p, no decision and no recommendation; H1 keeps its decision; the report lists the rev 15 withdrawn queries. Omitted threshold stays disabled. This task writes no application defaults; OD-4.1 alone implements an accepted recommendation.
    - Audit binds canonical external snapshot, preparation approval, frozen export/registration and started/completed hashes. Reserve before test payload access; every failure consumes the attempt. Visible output without matching completed custody is unaccepted, including publication/completion failure; preserve and audit it, never retry/reset.
    - Run every command from a clean linked Git worktree at candidate commit F; record its absolute path, shared Git common directory and exact binary SHA-256. Benchmark and output paths are outside the checkout. Main-checkout documentation may advance independently; no separate clone or custody reset.
    rollback:
    - None for the run itself; a superseding run requires a new pre-registered configuration, a fresh test split and its own audit record.
    stop_conditions:
    - Frozen config hash, commit or binary hash mismatch, or a dirty tree.
    - H1 fails; stop and audit the pipeline before interpreting H2 or H5.
    known_blockers: []
    closeout:
      role_results:
        coder-performance: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks approves held-out diagnostic limitation on 2026-11-13.
        validation_summary: EVAL-3.2 sole run and saved score exit0; completed custody and all publication hashes verified, score equals report. H1 pass, judged10=1; model-judged H2/H5 exploratory without promotion. Citation14/MCP12 tests exit0. Full outcomes, CIs, withdrawal list and limitations in audit_session-2026-11-13-held-out.md; external acceptance.json binds hashes.
    dod_status: pass
  OD-4.1:
    title: Given an accepted audited held-out report, apply only its predeclared h5-then-h2-v1 default recommendation with CLI/MCP parity, or record an explicit no-op when neither arm passes
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.2
    type: code
    validation_tier: release
    impact_risk: medium
    owner_files:
    - src/cli.rs
    - tests/cli.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/cli.rs
    - tests/cli.rs
    - tests/mcp.rs
    non_goals:
    - Ranking algorithm changes, mutable evaluation reads on the retrieval path, or promotion by largest observed held-out gain.
    - Default abstention, new MCP fields/tools or changes to explicit per-request mode/weight semantics.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp
    - cargo fmt --check
    - just --justfile ../ontocode/justfile compiler-lane-detached "$ONTODOC_PROMOTION_JOB" sh -c "CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings"
    expected_evidence:
    - 'Verify the accepted completed report, publication, frozen identity and audited recommendation. Apply H5-first mode plus frozen beta, else H2 mode plus frozen alpha, else record no code change through manager-owned closeout. Invalid or H1-failing output authorizes no promotion. A model-judged (exploratory) report authorizes no promotion: record the no-op.'
    - Shared SearchMode/SearchRequest in src/cli.rs owns clap and serde defaulting and omitted weights; src/mcp.rs consumes it and src/search/mod.rs requires explicit mode, so neither needs production edits. Preserve explicit requests and zero weights; absent threshold remains None. Generated MCP schema, CLI and serde omissions agree, and implicit ranking matches explicit approved parameters on synthetic fixtures.
    - Manager-owned audit records the exact recommendation and default constants, or the no-op reason; no second held-out run or post-result tuning. No-op may use unchanged compatibility checks and needs no source edit.
    - Run the no-cd external compiler-lane wrapper from this package root only, with a manager-assigned unique absent ONTODOC_PROMOTION_JOB. Harvest its terminal result with `just --justfile ../ontocode/justfile lane-result "$ONTODOC_PROMOTION_JOB"`; startup exit0 is not lint success. Never build the donor snapshot.
    rollback:
    - Revert only default/test edits without altering frozen code, custody or benchmark artifacts.
    stop_conditions:
    - Missing or mismatched accepted custody, audit, preregistered policy or frozen parameters.
    - Promotion needs an algorithm/schema/threshold-default change outside this card.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks; ADR13 model-judged report requires explicit no-op.
        validation_summary: No source/test changes; accepted custody and exploratory null decisions verified. CLI8/MCP12 and cli-unit8 passed; fmt exit0; managed Clippy terminal exit0 (ontodoc-010-noop-20261113-final). Source bytes match F. See audit_session-2026-11-13-default-noop.md.
    dod_status: pass
  OD-4.0:
    title: Given the held-out results, when the closeout is written, then README, AGENTS.md and memory-bank files describe install, MCP client setup, tool contracts and each hypothesis outcome with its CI and a link to the report
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.2
    - OD-4.1
    - HTTP-2
    - OD-3.9
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - README.md
    - AGENTS.md
    - .memory-bank/
    allowed_write_set:
    - README.md
    - AGENTS.md
    - .memory-bank/
    non_goals:
    - Claims beyond the recorded intervals or for inconclusive arms.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test
    expected_evidence:
    - README states each H2/H5 outcome as pass, negative or inconclusive with its CI and links the pinned external held-out report/audit identity under ADR6. Model-judged runs state effects and CIs as exploratory, say \"model-judged labels\", and claim no outcome.
    - Memory bank updated; evaluation closeout recorded; release tasks keep this plan open until REL-6 completes.
    rollback:
    - Revert the documentation edits.
    stop_conditions:
    - Held-out report is missing or its identity fields do not match the frozen config.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks authorizes evaluation documentation closeout.
        validation_summary: README/AGENTS/architecture report accepted exploratory outcomes, CIs and limitations; audit_session-2026-11-13-evaluation-closeout.md. Main-root cargo test exit0, 281 passed/one ignored; prior same-session fmt and managed Clippy exit0. No Rust changes or held-out rerun.
    dod_status: pass
  OD-3.0:
    title: OD-3.0 completed; original task card in archive/release-010-completed-cards.md#od-30
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    - OD-1.4
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    allowed_write_set:
    - src/store.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-30 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-30 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-30 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-30 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-30 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: review and challenge, consolidate one plan and proceed; GPT subagents requested.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-30 (sha256 96e92224ead1)'
    dod_status: pass
  OD-3.1:
    title: OD-3.1 completed; original task card in archive/release-010-completed-cards.md#od-31
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    non_goals:
    - See archive/release-010-completed-cards.md#od-31 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-31 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-31 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-31 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-31 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed with consolidated review-and-repair plan; OD-3.1 selected by strict manager routing.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-31 (sha256 a9f5fc57967c)'
    dod_status: pass
  OD-3.2:
    title: OD-3.2 completed; original task card in archive/release-010-completed-cards.md#od-32
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/parse.rs
    - src/store.rs
    - src/graph.rs
    - src/tiers.rs
    - src/chunk.rs
    - tests/cli.rs
    allowed_write_set:
    - src/parse.rs
    - src/store.rs
    - src/graph.rs
    - src/tiers.rs
    - src/chunk.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-32 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-32 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-32 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-32 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-32 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed with active repair tasks; GPT-6-Astra review retained.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-32 (sha256 104442db0afc)'
    dod_status: pass
  OD-3.3:
    title: OD-3.3 completed; original task card in archive/release-010-completed-cards.md#od-33
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - tests/mcp.rs
    - tests/cli.rs
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - tests/mcp.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-33 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-33 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-33 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-33 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-33 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; consolidated repair plan, GPT subagent review retained.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-33 (sha256 d5a77f8108a0)'
    dod_status: pass
  OD-3.4:
    title: OD-3.4 completed; original task card in archive/release-010-completed-cards.md#od-34
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    - SAFETY-0
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - src/corpus.rs
    - src/cite.rs
    - src/store.rs
    - src/cli.rs
    - src/mcp.rs
    - tests/cli.rs
    - tests/mcp.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - src/corpus.rs
    - src/cite.rs
    - src/store.rs
    - src/cli.rs
    - src/mcp.rs
    - tests/cli.rs
    - tests/mcp.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-34 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-34 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-34 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-34 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-34 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.4 local implementation, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-34 (sha256 3f716a93d91b)'
    dod_status: pass
  OD-3.5:
    title: OD-3.5 completed; original task card in archive/release-010-completed-cards.md#od-35
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.4
    - OD-3.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cite.rs
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/cite.rs
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-35 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-35 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-35 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-35 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-35 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.5 local execution, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-35 (sha256 7479892e3a84)'
    dod_status: pass
  OD-3.6:
    title: OD-3.6 completed; original task card in archive/release-010-completed-cards.md#od-36
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.4
    - OD-3.8
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/corpus.rs
    - src/store.rs
    - src/cli.rs
    - src/search/mod.rs
    - tests/cli.rs
    allowed_write_set:
    - src/corpus.rs
    - src/store.rs
    - src/cli.rs
    - src/search/mod.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-36 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-36 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-36 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-36 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-36 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.6 local implementation, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-36 (sha256 79955109100a)'
    dod_status: pass
  EVAL-1.4:
    title: EVAL-1.4 completed; original task card in archive/release-010-completed-cards.md#eval-14
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/corpus.rs
    - src/cli.rs
    - src/store.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/corpus.rs
    - src/cli.rs
    - src/store.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#eval-14 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-14 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-14 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-14 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-14 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; continue selected EVAL-1.4 and its recorded traversal seam.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-14 (sha256 e7fbe946abb4)'
    dod_status: pass
  EVAL-1.5:
    title: EVAL-1.5 completed; original task card in archive/release-010-completed-cards.md#eval-15
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.4
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#eval-15 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-15 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-15 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-15 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-15 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed with active EVAL-1.5; its explicit ADR conflict-resolution authority and manager documentation scope recorded in Decisions And Non-Goals.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-15 (sha256 5760454f4841)'
    dod_status: pass
  HTTP-0:
    title: HTTP-0 completed; original task card in archive/release-010-completed-cards.md#http-0
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    - SAFETY-0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    non_goals:
    - See archive/release-010-completed-cards.md#http-0 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#http-0 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#http-0 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#http-0 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#http-0 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (round-4 decision packet); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-0 (sha256 73304d15dae2)'
    dod_status: pass
  HTTP-1:
    title: HTTP-1 completed; original task card in archive/release-010-completed-cards.md#http-1
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-1
    - OD-3.2
    - OD-3.4
    - OD-3.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    non_goals:
    - See archive/release-010-completed-cards.md#http-1 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#http-1 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#http-1 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#http-1 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#http-1 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; HTTP-1 split from the HTTP-0 exposure gate in audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-1 (sha256 fafa9305239f)'
    dod_status: pass
  HTTP-2:
    title: HTTP-2 completed; original task card in archive/release-010-completed-cards.md#http-2
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-0
    - HTTP-1
    - PERF-1
    - OD-3.3
    - OD-3.5
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - src/main.rs
    - src/cli.rs
    - src/mcp.rs
    - src/http.rs
    - src/lib.rs
    - tests/mcp.rs
    - tests/cli.rs
    - tests/http.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - src/main.rs
    - src/cli.rs
    - src/mcp.rs
    - src/http.rs
    - src/lib.rs
    - tests/mcp.rs
    - tests/cli.rs
    - tests/http.rs
    non_goals:
    - See archive/release-010-completed-cards.md#http-2 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#http-2 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#http-2 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#http-2 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#http-2 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (all ten owner recommendations); audit_session-2026-11-11-owner-decisions.md; ADR revision 10 HTTP-0 section'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-2 (sha256 8822733dd088)'
    dod_status: pass
  OD-3.7:
    title: OD-3.7 completed; original task card in archive/release-010-completed-cards.md#od-37
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    non_goals:
    - See archive/release-010-completed-cards.md#od-37 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-37 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-37 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-37 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-37 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; dependency-ready operational docs after HTTP-0 authority stop.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-37 (sha256 6886f29ec17c)'
    dod_status: pass
  OD-3.8:
    title: OD-3.8 completed; original task card in archive/release-010-completed-cards.md#od-38
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.2
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    allowed_write_set:
    - src/store.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-38 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-38 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-38 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-38 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-38 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: GPT challenge, consolidate robust performance-oriented plan and proceed; ADR7 and OD-3.8 selected by strict manager.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-38 (sha256 86df1804dea9)'
    dod_status: pass
  OD-3.9:
    title: OD-3.9 completed; original task card in archive/release-010-completed-cards.md#od-39
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SAFETY-0
    - OD-3.8
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    - src/cli.rs
    - tests/cli.rs
    allowed_write_set:
    - src/store.rs
    - src/cli.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#od-39 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#od-39 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#od-39 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#od-39 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#od-39 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager OD-3.9 local execution, ADR8.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-39 (sha256 59c80667c64a)'
    dod_status: pass
  SAFETY-0:
    title: SAFETY-0 completed; original task card in archive/release-010-completed-cards.md#safety-0
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.2
    - OD-3.3
    - OD-3.8
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    non_goals:
    - See archive/release-010-completed-cards.md#safety-0 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#safety-0 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#safety-0 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#safety-0 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#safety-0 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User review/challenge/consolidate/proceed instruction; strict manager selected dependency-ready SAFETY-0, local coder-manager documentation execution.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#safety-0 (sha256 6f2d09c4ba9d)'
    dod_status: pass
  EVAL-1.6:
    title: EVAL-1.6 completed; original task card in archive/release-010-completed-cards.md#eval-16
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.4
    - EVAL-1.5
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    non_goals:
    - See archive/release-010-completed-cards.md#eval-16 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#eval-16 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#eval-16 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#eval-16 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#eval-16 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict EVAL-1.6 documentation scope under review/consolidate/proceed instruction.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-16 (sha256 9a46b1ce4357)'
    dod_status: pass
  FIX-1:
    title: FIX-1 completed; original task card in archive/release-010-completed-cards.md#fix-1
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.3
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/cite.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/common/
    allowed_write_set:
    - src/cite.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/common/
    non_goals:
    - See archive/release-010-completed-cards.md#fix-1 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-1 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-1 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-1 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-1 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-1 (sha256 b985679ee993)'
    dod_status: pass
  SIMPL-1:
    title: SIMPL-1 completed; original task card in archive/release-010-completed-cards.md#simpl-1
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-1
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/eval/stats.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/eval/stats.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#simpl-1 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#simpl-1 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#simpl-1 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#simpl-1 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#simpl-1 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-1 (sha256 d518d2e86b14)'
    dod_status: pass
  FIX-2:
    title: FIX-2 completed; original task card in archive/release-010-completed-cards.md#fix-2
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/sweep.rs
    allowed_write_set:
    - src/eval/sweep.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-2 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-2 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-2 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-2 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-2 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: proceed (after open-item list offering deferred review fixes); audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-2 (sha256 9b5ad8ff6c6b)'
    dod_status: pass
  FIX-3:
    title: FIX-3 completed; original task card in archive/release-010-completed-cards.md#fix-3
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-2
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/cli.rs
    - tests/eval_gates.rs
    allowed_write_set:
    - src/eval/run.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/cli.rs
    - tests/eval_gates.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-3 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-3 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-3 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-3 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-3 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed; audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-3 (sha256 c2328a5e8e8d)'
    dod_status: pass
  FIX-4:
    title: FIX-4 completed; original task card in archive/release-010-completed-cards.md#fix-4
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-3
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cli.rs
    allowed_write_set:
    - src/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-4 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-4 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-4 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-4 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-4 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed; audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-4 (sha256 76d06c453ef6)'
    dod_status: pass
  SIMPL-2:
    title: SIMPL-2 completed; original task card in archive/release-010-completed-cards.md#simpl-2
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-4
    type: code
    validation_tier: integration
    impact_risk: low
    owner_files:
    - src/eval/run.rs
    allowed_write_set:
    - src/eval/run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#simpl-2 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#simpl-2 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#simpl-2 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#simpl-2 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#simpl-2 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: proceed after stated default (baseline commit plus re-check consolidation); audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-2 (sha256 eb35c9c13502)'
    dod_status: pass
  FIX-5:
    title: FIX-5 completed; original task card in archive/release-010-completed-cards.md#fix-5
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-2
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    allowed_write_set:
    - src/eval/run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-5 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-5 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-5 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-5 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-5 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 4); audit_session-2026-11-11-review-round4.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-5 (sha256 a374f24668ca)'
    dod_status: pass
  FIX-6:
    title: FIX-6 completed; original task card in archive/release-010-completed-cards.md#fix-6
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-5
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/store.rs
    - src/cli.rs
    allowed_write_set:
    - src/store.rs
    - src/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-6 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-6 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-6 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-6 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-6 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 4); audit_session-2026-11-11-review-round4.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-6 (sha256 41710c33dac2)'
    dod_status: pass
  FIX-7:
    title: FIX-7 completed; original task card in archive/release-010-completed-cards.md#fix-7
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-2
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-7 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-7 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-7 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-7 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-7 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-7 (sha256 019cf794c91b)'
    dod_status: pass
  SIMPL-3:
    title: SIMPL-3 completed; original task card in archive/release-010-completed-cards.md#simpl-3
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-7
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/gates.rs
    - src/eval/stats.rs
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/stats.rs
    non_goals:
    - See archive/release-010-completed-cards.md#simpl-3 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#simpl-3 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#simpl-3 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#simpl-3 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#simpl-3 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-3 (sha256 b6e2cc4192b0)'
    dod_status: pass
  FIX-8:
    title: FIX-8 completed; original task card in archive/release-010-completed-cards.md#fix-8
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-3
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/run.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_run.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-8 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-8 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-8 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-8 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-8 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-8 (sha256 3bd39fe7017b)'
    dod_status: pass
  FIX-9:
    title: FIX-9 completed; original task card in archive/release-010-completed-cards.md#fix-9
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-8
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/ranking_sanity.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/ranking_sanity.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-9 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-9 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-9 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-9 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-9 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 5); audit_session-2026-11-11-review-round5.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-9 (sha256 58517f790daa)'
    dod_status: pass
  FIX-10:
    title: FIX-10 completed; original task card in archive/release-010-completed-cards.md#fix-10
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-9
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - Cargo.toml
    - src/search/bm25.rs
    - src/search/ppr.rs
    - src/search/fusion.rs
    allowed_write_set:
    - Cargo.toml
    - src/search/bm25.rs
    - src/search/ppr.rs
    - src/search/fusion.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-10 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-10 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-10 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-10 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-10 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 6); audit_session-2026-11-11-review-round6.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-10 (sha256 ae1a67e2bd05)'
    dod_status: pass
  FIX-11:
    title: FIX-11 completed; original task card in archive/release-010-completed-cards.md#fix-11
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-10
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/store.rs
    - src/search/fusion.rs
    - src/search/bm25.rs
    - tests/http.rs
    allowed_write_set:
    - src/store.rs
    - src/search/fusion.rs
    - src/search/bm25.rs
    - tests/http.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-11 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-11 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-11 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-11 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-11 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 7); audit_session-2026-11-11-review-round6.md#round-7'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-11 (sha256 ab47ca5d36cf)'
    dod_status: pass
  FIX-12:
    title: FIX-12 completed; original task card in archive/release-010-completed-cards.md#fix-12
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-11
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/parse.rs
    - src/tiers.rs
    - src/graph.rs
    - src/store.rs
    - tests/cli.rs
    allowed_write_set:
    - src/parse.rs
    - src/tiers.rs
    - src/graph.rs
    - src/store.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-12 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-12 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-12 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-12 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-12 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: "approved" (round-7 option 2, table-aware first sentence); ADR revision 12'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-12 (sha256 f574fa3ae4ac)'
    dod_status: pass
  FIX-13:
    title: FIX-13 completed; original task card in archive/release-010-completed-cards.md#fix-13
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-12
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/graph.rs
    - src/store.rs
    - src/eval/gates.rs
    - tests/cli.rs
    allowed_write_set:
    - src/graph.rs
    - src/store.rs
    - src/eval/gates.rs
    - tests/cli.rs
    non_goals:
    - See archive/release-010-completed-cards.md#fix-13 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#fix-13 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#fix-13 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#fix-13 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#fix-13 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: "1 b ... 2 i 3 ok" (html link mapping, rust-book/src root, model-judged exploratory track); ADR revision 13'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-13 (sha256 a899c69ab1bf)'
    dod_status: pass
  PERF-1:
    title: PERF-1 completed; original task card in archive/release-010-completed-cards.md#perf-1
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/mcp.rs
    - tests/mcp.rs
    non_goals:
    - See archive/release-010-completed-cards.md#perf-1 (verbatim card).
    required_validation:
    - See archive/release-010-completed-cards.md#perf-1 (verbatim card).
    expected_evidence:
    - See archive/release-010-completed-cards.md#perf-1 (verbatim card).
    rollback:
    - See archive/release-010-completed-cards.md#perf-1 (verbatim card).
    stop_conditions:
    - See archive/release-010-completed-cards.md#perf-1 (verbatim card).
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; use GPT reviewers'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#perf-1 (sha256 bb9e833843f6)'
    dod_status: pass
  REL-0:
    title: Given the release instruction, register scoped release cards and correct eval command sequencing while preserving completed-task evidence and owner gates
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on: []
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - docs/RELEASE_0.1.0_PLAN.md
    - .memory-bank/archive/release-010-completed-cards.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    allowed_write_set:
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - docs/RELEASE_0.1.0_PLAN.md
    - .memory-bank/archive/release-010-completed-cards.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - git diff --check
    expected_evidence:
    - All 53 original DONE cards are archived byte-for-byte with hashes; active status, dependencies and complete closeout records are unchanged.
    - Canonical plan validation passes; eval recapture omits --gates, uses operator arguments and a new manifest, power comparison precedes freeze, and linked-worktree identity is explicit.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: proceed tasks from docs/RELEASE_0.1.0_PLAN.md'
        validation_summary: git diff --check exit 0; Python archive/hash/state check exit 0 for all 53 DONE cards; project_plan_validate PASS (66 tasks). See audit_session-2026-11-12-release-preparation.md. No Rust or evaluation execution.
    dod_status: pass
  REL-1:
    title: Given owner licence and snapshot decisions, prepare MIT root metadata and a complete linked-crate licence inventory before candidate freeze
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - REL-0
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - LICENSE
    - NOTICE
    - Cargo.toml
    - README.md
    - THIRD_PARTY_LICENSES.md
    - LICENSE-APACHE
    - AGENTS.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    allowed_write_set:
    - LICENSE
    - NOTICE
    - Cargo.toml
    - README.md
    - THIRD_PARTY_LICENSES.md
    - LICENSE-APACHE
    - AGENTS.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - git diff --check
    expected_evidence:
    - Owner-approved copyright line; root licence and Cargo metadata agree; linked dependency licence texts inventoried with build-only crates distinguished.
    - Prior reuse query-budget, citation and metric patterns compared for copied text; snapshot disposition resolved in a separately scoped ADR/task amendment if needed.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-13: keep snapshot; proceed tasks after proposed Copyright (c) 2026 OntoDoc contributors.'
        validation_summary: 'REL-1 audit in audit_session-2026-11-12-release-preparation.md: offline locked Cargo metadata/tree exit 0; git diff --check exit 0; manifest hash continuity and normalized full licence completeness checks exit 0; 114 crates, 90 runtime candidates and 24 build-only. Bounded source reuse comparison completed with no substantive copied text found. No Rust source changes or compilation. LICENSE-APACHE preserves original bytes; NOTICE and snapshot unchanged.'
    dod_status: pass
  REL-2:
    title: Given source-checked service contracts, correct stale architecture and pending documentation and label ranking and capacity unmeasured
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - REL-0
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    - .memory-bank/archive/release-010-pending-history.md
    allowed_write_set:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    - .memory-bank/archive/release-010-pending-history.md
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - git diff --check
    expected_evidence:
    - Current docs match source for hidden corpus directories, schema 4, HTTP-0 completion, lexical defaults and model-judged exploratory limits.
    - Pending router contains current work only; removed historical text remains verbatim in archive/release-010-pending-history.md.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: proceed tasks from docs/RELEASE_0.1.0_PLAN.md'
        validation_summary: git diff --check exit 0; inline Python document-link/status/history checks exit 0; source-backed registry/defaults/HTTP/schema review. See audit_session-2026-11-12-release-preparation.md#rel-2. Rust checks skipped for docs-only task.
    dod_status: pass
  REL-4:
    title: Given the accepted ingress contract, document a systemd service and TLS-authenticated nginx proxy with correct Host and encoded routing, explicitly labelled untested
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - REL-2
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - docs/DEPLOYMENT.md
    - README.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    allowed_write_set:
    - docs/DEPLOYMENT.md
    - README.md
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - git diff --check
    expected_evidence:
    - Guide includes trusted indexing, account/file permissions, TLS authentication on every request, fixed upstream Host, URI preservation, Origin refusal, protocol headers and graceful drain.
    - No service installation or remote exposure; missing nginx/runtime tests are explicitly disclosed.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: proceed tasks from docs/RELEASE_0.1.0_PLAN.md'
        validation_summary: git diff --check exit 0; inline Python guide-reference/fence/INI/ingress-contract checks exit 0; project_plan_validate PASS. Guide explicitly untested; nginx/systemd runtime checks skipped. See audit_session-2026-11-12-release-preparation.md#rel-4.
    dod_status: pass
  REL-5:
    title: Given an approved binary target and release changes, validate a clean linked candidate worktree and retain one hashed release binary
    status: DONE
    classification: proof-only
    assurance_profile: full
    complexity: complex
    depends_on:
    - REL-1
    - REL-4
    type: proof
    validation_tier: release
    impact_risk: low
    owner_files:
    - tmp/release-010/candidate/Cargo.toml
    - tmp/release-010/candidate/src/
    - tmp/release-010/candidate/tests/
    allowed_write_set: []
    validation_workspace:
      root: tmp/release-010
      inputs:
      - candidate/Cargo.toml
      - candidate/Cargo.lock
      - candidate/src/
      - candidate/tests/
      generated_outputs:
      - candidate/target/
      - evidence/
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - cargo fmt --check
    - just --justfile "$ONTODOC_LANE_JUSTFILE" compiler-lane-detached "$ONTODOC_RELEASE_JOB" sh -c "CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings"
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - CARGO_BUILD_JOBS=4 cargo build --release --locked
    expected_evidence:
    - Every command and exit code recorded from clean worktree F; all five tools exercised over stdio and HTTP; --version and home-path binary scan recorded.
    - Binary SHA-256, toolchain, glibc and kernel recorded; executable retained unchanged for measurement and packaging.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-auditor: null
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User 2026-11-13 proceed tasks; Linux x86_64 GNU target accepted.
        validation_summary: 'REL-5: clean F cc116c44a4d7f854bc19c6a4b40d75098a437268; compiler lane ontodoc-rel5-20261113-a terminal exit 0; fmt, Clippy, all-target tests (281 passed, 1 ignored), locked release build exit 0. tmp/release-010/evidence/{checks,smoke,binary-inspection}.json records five-tool stdio/HTTP parity, version, host and path scan (312 home-path matches). Binary SHA-256 af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55 retained unchanged. No held-out execution or publication.'
    dod_status: pass
  REL-3:
    title: Given the retained candidate binary, record index time/RSS and HTTP per-tool p50/p95 at concurrency 1 and 4 plus busy behavior on both benchmark corpora and a near-cap corpus
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - REL-5
    type: measurement
    validation_tier: release
    impact_risk: low
    owner_files:
    - tmp/release-010/candidate/target/release/ontodoc
    allowed_write_set: []
    validation_workspace:
      root: tmp/release-010
      inputs:
      - candidate/target/release/ontodoc
      generated_outputs:
      - evidence/capacity/
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - python3 tmp/release-010/evidence/capacity/measure.py --self-check
    - python3 tmp/release-010/evidence/capacity/measure.py
    - sha256sum tmp/release-010/candidate/target/release/ontodoc
    expected_evidence:
    - Audit records host, corpus sizes, repetitions, workload, all five tool latencies at both concurrency levels, index elapsed time/peak RSS and observed busy refusal.
    - Measurement protocol and runnable command packet registered before running; no capacity pass verdict without owner latency target.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    known_blockers: []
    closeout:
      role_results:
        coder-performance: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User 2026-11-13 proceed tasks; corrected packet retry accepted.
        validation_summary: 'REL-3 audit_session-2026-11-13-capacity.md; measure.py --self-check and full invocation exit 0. Nine index runs exit 0; 1660 HTTP samples: 1008 success, 652 busy, zero other errors; five tools/c1/c4 on both pins plus near-file-cap corpus. Final binary/candidate identity verified; no capacity acceptance verdict. Raw result.json and observations.jsonl under tmp/release-010/evidence/capacity/.'
    dod_status: pass
  REVIEW-9:
    title: Verify the preserved release archive and complete source inventory without rebuilding
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on: []
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - scripts/verify_release.py
    - tests/test_verify_release.py
    allowed_write_set:
    - scripts/verify_release.py
    - tests/test_verify_release.py
    non_goals:
    - No src, Cargo, donor, package, frozen checkout or evaluation custody changes; no publication, runtime fixes or performance claims.
    - Main session owns documentation and tracking; implementation child owns only the two Python files.
    required_validation:
    - PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_verify_release.py
    - PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify_release.py
    - git diff --check
    expected_evidence:
    - Read-only verifier checks archive and member identities, exact source file sets and bytes against F, and optional annotated-tag identity; missing tag is explicitly skipped.
    - Synthetic tests reject corrupted or unsafe members, source additions/deletions/changes, symlinks, lightweight tags and mismatched annotated tags.
    rollback:
    - Revert only REVIEW-9 edits; preserve all prior changes, archive bytes, source candidate and custody.
    stop_conditions:
    - Any production-code or frozen-artifact edit, unresolved ADR conflict or publication action is required.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User 2026-11-13 review, consolidate and proceed with GPT subagents.
        validation_summary: 'GPT implementation and independent source challenge reviewed; no material finding. Unittest: 22 pass, exit 0 (636c9f); verifier: 9 members and 28 source files pass, exit 0 (75c713); git diff --check exit 0 (e4305d). Tag skipped; no Rust/build/eval/publication. Parent inspected final source; challenge was source-only.'
    dod_status: pass
  REVIEW-9-DOC:
    title: Consolidate GPT challenge findings and correct release documentation against current evidence
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - REVIEW-9
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - docs/RELEASE_0.1.0_PLAN.md
    - docs/DEPLOYMENT.md
    - docs/EVALUATION_0.1.0.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/audit_session-2026-11-13-review-round9.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    allowed_write_set:
    - docs/RELEASE_0.1.0_PLAN.md
    - docs/DEPLOYMENT.md
    - docs/EVALUATION_0.1.0.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/audit_session-2026-11-13-review-round9.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    non_goals:
    - No runtime changes, publication, performance claims or held-out rerun; preserve frozen bytes and history.
    required_validation:
    - git diff --check
    expected_evidence:
    - Review cites strengths, defects, alternatives and selected next actions; docs and REL-6 gates match evidence, with references and strict plan validation checked.
    rollback:
    - Revert only new review documentation edits.
    stop_conditions:
    - A semantic ADR change, frozen-artifact edit or publication is required.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User 2026-11-13 review, consolidate and proceed.
        validation_summary: Round9 audit and consolidated release plan complete; all-outcomes companion independently checked by GPT. Inline Python links/anchors/fences and exact aggregate tables exit 0; git diff --check exit 0; strict plan validation PASS, 68 tasks. No Rust/artifact/eval changes; runtime blocked by recorded ADR/candidate decision.
    dod_status: pass
  REL-6:
    title: Given accepted evaluation closeout and release inputs, package the exact measured binary, verify installation, then tag and publish to the owner-selected destination
    status: BLOCKED
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-4.0
    - REL-2
    - REL-3
    - REL-4
    - REL-5
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - CHANGELOG.md
    - tmp/release-010/package/
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    allowed_write_set:
    - CHANGELOG.md
    - tmp/release-010/package/
    - .memory-bank/audit_session-2026-11-12-release-preparation.md
    non_goals:
    - No retrieval changes, held-out access, publication or owner decisions beyond this card.
    required_validation:
    - git diff --check
    - PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify_release.py
    expected_evidence:
    - Package contains binary, README, MIT licence, dependency licence texts, deployment guide and SHA256SUMS; binary hash equals accepted held-out report.
    - Selected candidate and annotated tag have exact source/Cargo/toolchain inventory and bytes; install smoke, tag and publication receipts recorded; distribute EVALUATION_0.1.0.md with all outcomes. Current verifier pins preserved F only; corrected candidate needs separate identities/validation.
    rollback:
    - Revert only this task’s new changes; preserve pre-existing edits, historical evidence and evaluation custody.
    stop_conditions:
    - An unresolved owner decision, ADR conflict or write outside the declared scope is required.
    - Stop affected runtime implementation/publication at the confirmed HTTP parity ADR conflict; owner must resolve corrected-candidate identity, destination/exposure and embedded paths. Preserve measured binary and consumed custody.
    known_blockers:
    - HTTP large-ID overflow returns plain 500 versus stdio typed cap error, conflicting with ADR parity; see audit_session-2026-11-13-review-round9.md. Original package verified; corrected-candidate, publication destination/exposure and embedded-path decisions outstanding. No tag/upload.
    closeout:
      role_results:
        coder-manager: null
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| REQ-001 | User: Rust project | implementation | OD-0.0 | `cargo check --all-targets` |
| REQ-002 | User: new MCP service for docs | implementation | OD-2.1 | `cargo test --test mcp` |
| REQ-003 | ADR §2–4: safe corpus walk, CommonMark parsing, budget-independent section ids, chunks | implementation | OD-1.0, OD-1.1 | corpus and parse unit tests |
| REQ-004 | ADR §4: structural tiers and document pseudo-documents | implementation | OD-1.2 | tiers unit tests |
| REQ-005 | ADR §5: document link graph | implementation | OD-1.3 | graph golden edge list |
| REQ-006 | ADR §7: atomic verified generations, index-time-only generation id | implementation | OD-1.4 | store unit tests |
| REQ-007 | ADR §6: B0, B1, pre-fusion abstention, no LLM on path | implementation | OD-1.5 | search unit tests |
| REQ-008 | ADR §6: specified PPR (B2), B3, scope, explain | implementation | OD-1.6 | search unit tests |
| REQ-009 | ADR §10, H4: four citation states | implementation | OD-1.7, OD-2.1 | cite and MCP tests |
| REQ-010 | ADR §8: budgeted assembly, feature without claim | implementation | OD-1.8, OD-2.1 | assemble and MCP tests |
| REQ-011 | ADR §9 and §11: freshness and determinism | implementation | OD-1.4, EVAL-1.1 | store tests; byte-identical reruns |
| REQ-012 | User: scientifically provable improvements; ADR metrics, decision rule, power | implementation | EVAL-1.0, EVAL-1.1, EVAL-1.2 | stats unit tests; discrimination check; planted-defect gates |
| REQ-013 | ADR benchmark data, grid-wide pooling, annotation | proof-only | EVAL-2.0, EVAL-2.1 | `eval check-benchmark` |
| REQ-014 | ADR H1 gate, H2/H5 confirmatory, H3, E1; code freeze; burned split | proof-only | EVAL-3.0, EVAL-3.1, EVAL-3.2 | recorded reports and audit sessions |
| REQ-015 | User: review prior attempt; reuse list applied | implementation | OD-1.0, OD-1.4, OD-1.7 | prior-attempt audit; store and cite tests |
| REQ-016 | User: donor ideas (OpenViking, pharma) | implementation | OD-1.2, OD-1.6, OD-1.8, EVAL-1.2 | donor-ideas audit; unit and gate tests |
| REQ-017 | Plan challenge findings C1–C4, H1–H3, M1–M9, L1–L3 | implementation | EVAL-1.0, EVAL-1.2, OD-1.1, OD-1.5, OD-1.6 | plan-challenge audit dispositions |
| REQ-018 | Embeddings, KM, LineageRAG, code graph, H6 excluded from v1 | non-goal | OD-1.5, OD-1.8 | task non_goals; ADR Consequences |
| REQ-019 | User: AGENTS.md and memory bank | implementation | OD-4.0 | files present; updated at closeout |
| REQ-020 | Review S5: unique published section/chunk IDs | implementation | OD-3.0 | duplicate collection rejection; unchanged store; malformed load |
| REQ-021 | Review S6: canonical bytes and identity collisions | architecture-gate | OD-3.1, OD-3.2 | accepted schema decision; mtime-independent full artifact bytes |
| REQ-022 | Review S1: full-wire bounds and internal-error privacy | implementation | OD-3.3 | actual JSON-RPC frame tests, including escaped text and errors |
| REQ-023 | Review S2: corpus membership and race-safe bounded reads | implementation | SAFETY-0, OD-3.4 | descriptor replacement/growth and fabricated-citation tests |
| REQ-024 | Review S3: bounded actual work and cancellation ownership | implementation | SAFETY-0, OD-3.5 | work counters and closure-owned admission barrier tests |
| REQ-025 | Review S4: bounded freshness/load I/O without semantic weakening | implementation | OD-3.8, SAFETY-0, OD-3.6 | pre-read caps; same-size edit freshness |
| REQ-026 | Evaluation isolation, checked-byte identity and success paths | implementation | EVAL-1.4, EVAL-1.5, EVAL-1.3 | synthetic inaccessible held-out inputs, tampering, attempt and success/failure tests |
| REQ-027 | Accepted rev4: each docs_share child has endpoint and own store | implementation | HTTP-0, HTTP-1, HTTP-2 | independent stores/session bindings and stdio parity |
| REQ-028 | Usable operational documentation independent of claims | implementation | OD-3.7, OD-4.0 | current contracts first; scientific results only after EVAL-3.2 |
| REQ-029 | Proposed evidence/diagnostics capabilities | architecture-gate | OD-4.0 | deferred and unaccepted; separate service-ADR amendment and task scope required |
| REQ-030 | Current challenge: stdio safety independent of network exposure | architecture-gate | SAFETY-0, HTTP-0 | explicit local contracts; owner exposure gate retained |
| REQ-031 | Current challenge: bounded store controls/artifacts/publication | implementation | OD-3.8 | actual-byte, configured/global cap and preservation regressions |
| REQ-032 | Current challenge: publication outcomes and lock evidence | implementation | SAFETY-0, OD-3.9 | deterministic commit-boundary failure tests |
| REQ-033 | ADR9 trusted provenance, cross-process preparation and custody acceptance | architecture-gate | EVAL-1.6, EVAL-1.3, EVAL-2.0, EVAL-2.1 | pre-pool approval, raw human agreement, selected-only execution and synthetic failure paths |
| REQ-034 | ADR9 deterministic default promotion, distinct from measurement | implementation | EVAL-1.6, EVAL-3.2, OD-4.1, OD-4.0 | frozen H5-first policy; audited implementation/no-op; CLI/MCP parity |
| REQ-035 | 2026-11-11 review: verified cite/search/eval defects | implementation | FIX-1 | focused regression per defect |
| REQ-036 | 2026-11-11 review: remove eval duplication without weakening safeguards | implementation | SIMPL-1 | unchanged gate/report/refusal tests |
| REQ-037 | User: many concurrent users on the shared corpus | implementation | PERF-1, HTTP-2 | overlapping admitted requests, bounded permits |
| REQ-038 | Deferred review item: frozen TOML round-trips arbitrary corpus names | implementation | FIX-2 | escaped-name round-trip regression |
| REQ-039 | 2026-11-11 round-3 review: verified eval output-budget, miss-attribution and dead-plumbing defects | implementation | FIX-3 | pre-reservation budget and attribution regressions |
| REQ-040 | 2026-11-11 round-3 review: bounded related/status collection | implementation | FIX-4 | unchanged-response regression |
| REQ-041 | 2026-11-11 round-3 deferred simplification: single held-out freeze verification per command | implementation | SIMPL-2 | unchanged refusal tests |
| REQ-042 | 2026-11-11 round-4 review: held-out pre-reservation refusals for TSV-unsafe ids and unusable output parents | implementation | FIX-5 | refusal regressions |
| REQ-043 | 2026-11-11 round-4 review: verifiable citations (heading bound at publication) | implementation | FIX-6 | cap and lookup regressions |
| REQ-044 | Owner decisions 7-8 (rev10): per-result text_truncated; null statistics for n = 0 groups | implementation | FIX-7 | flag and null regressions |
| REQ-045 | Owner decision 9: one shared kappa implementation | implementation | SIMPL-3 | unchanged worked examples |
| REQ-046 | Owner decision 10: staging before held-out reservation | implementation | FIX-8 | unwritable-parent refusal regression |
| REQ-047 | Round-5 review: dev staging order; ADR discrimination check on planted synthetic sections | implementation | FIX-9 | ranking_sanity and staging regressions |
| REQ-048 | Round-6 review: hand-computed scorer edge tests; dev hash-crate optimization | implementation | FIX-10 | search:: numeric tests |
| REQ-049 | Round-7 review: interrupted prune recovery via temp namespace; discriminating fusion ties; bounded HTTP test reads | implementation | FIX-11 | store prune regression |
| REQ-050 | Owner-approved table-aware first sentence; schema 3 | implementation | FIX-12 | tiers first_sentence regression |
| REQ-051 | Owner-approved `.html`→`.md` link resolution (schema 4); subdirectory corpus-root pin exclusion | implementation | FIX-13 | graph html-link and gates subdirectory-root regressions |
| REQ-052 | ADR rev 13 model-judged exploratory labels: admission, provenance, exploratory-only reporting | implementation | FIX-14, EVAL-2.1 | gates/run model-judged regressions |
| REQ-053 | ADR rev 14-15 pool extension and one-time withdrawal of still-unanswered queries | implementation | FIX-15, EVAL-2.1 | pool-extension regressions; qrels gate |

| REQ-054 | User 2026-11-12: proceed docs/RELEASE_0.1.0_PLAN.md | implementation | REL-0, REL-1, REL-2, REL-3, REL-4, REL-5, REL-6 | Scoped release validation and retained candidate identity |
| REQ-055 | User 2026-11-13: challenge completed/planned work with GPT subagents, consolidate and proceed | implementation | REVIEW-9, REVIEW-9-DOC | Synthetic verifier failure tests, existing artifact verification and evidence-backed review |

## Decisions And Non-Goals

Historical decisions preserved verbatim in [release-010-plan-decisions.md](archive/release-010-plan-decisions.md). Current design authority is ADR revision 16; held-out miss attribution is unavailable for the registered experiment by explicit owner approval. All other gates remain.

REL-6 local preparation is authorized independently of publication: write CHANGELOG.md and an exact-byte binary package under tmp/release-010/package/ using build-package.py; retain deterministic archive, SHA256SUMS and clean-directory install/index/search smoke evidence. Package includes a standalone README derived from repository text, CHANGELOG, licences/notices and DEPLOYMENT; links to unshipped source/audits become labelled source references, and the selected target/capacity facts supersede stale guide prose. Exclude raw benchmark data and donor sources. Disposable staging is created uniquely and cleaned on exit. No tag, commit or upload occurs until the destination/exposure decision; REL-6 cannot close before those receipts.

## Closeout
