# Donor implementation review and evidence repair

Date: 2026-11-14
Status: Closed
Authority: User request to challenge completed donor work, consolidate proposals and proceed; service ADR18.

## Goal

Correct misleading closure text and preserve a durable census result without speculative runtime changes.

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
  DR-1:
    title: Correct contradictory donor records and retain verified census evidence
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/DONOR_REVIEW_PLAN.md
    - docs/DONOR_CANDIDATES_PLAN.md
    - docs/ADR_DONOR_ADOPTION.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    - .memory-bank/audit_session-2026-11-14-donor-review.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    allowed_write_set:
    - docs/DONOR_REVIEW_PLAN.md
    - docs/DONOR_CANDIDATES_PLAN.md
    - docs/ADR_DONOR_ADOPTION.md
    - .memory-bank/audit_session-2026-11-14-donor-candidates.md
    - .memory-bank/audit_session-2026-11-14-donor-review.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    non_goals:
    - No Rust, parser, ranking, schema, evaluation or release changes; preserve closed task history.
    required_validation:
    - git diff --check
    - project_plan_validate docs/DONOR_REVIEW_PLAN.md
    - project_plan_validate docs/DONOR_CANDIDATES_PLAN.md
    - Verify census SHA256SUMS and durable result hash; inspect local document references.
    expected_evidence:
    - Audit no longer calls the completed census pending; speculative comparison is clearly separated from observed baseline; durable record includes exact census JSON and its verified hash.
    rollback:
    - Revert only review-owned documentation edits, preserving pre-existing work.
    stop_conditions:
    - Evidence contradicts ADR18 or requires runtime behavior changes.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: Review challenge consolidate and proceed request on 2026-11-14.
        validation_summary: Two GPT Luna source reviews found documentation defects and no runtime blocker. Both plan validators passed; census SHA256SUMS and exact retained JSON hash matched; local references checked; git diff --check exited 0. No Rust edits or builds. Evidence in .memory-bank/audit_session-2026-11-14-donor-review.md.
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| DR-R1 | Challenge completed work and execute the smallest robust fixes | implementation | DR-1 | GPT source reviews, evidence hash and documentation checks |

## Decisions And Non-Goals

The original four-task plan stays closed. GPT review found no concrete runtime blocker. Citation retention, shared fitting behavior and explicit omission counts are strengths; source tests cover the important oversized-hit and citation-only boundaries. Tests were inspected, not rerun in this documentation-only review.

| Option | Benefit and cost | Decision |
| --- | --- | --- |
| Repair records and retain exact census results | Fixes confirmed contradictions and fragile ignored evidence with no runtime cost | Selected |
| Replace the approximate scanner with the production Markdown parser | Could support a future calibrated census; requires a separately scoped measurement tool and fixtures | Defer until link-resolution work has an approved question |
| Rewrite fitting around incremental byte estimates or add persistent graph indexes | Might reduce serialization or scans, but adds invalidation/escaping complexity without a measured bottleneck | Reject for this task; profile synthetic worst cases before proposing it |

Repeated serialization is bounded by 50 results, but bounded work is not a latency measurement. The existing exact serializer remains the safest fit oracle. The approximate link scanner duplicated parser logic and failed calibration; do not extend it or use it to justify edge gains. Preserve its failed outcome. A historical MCP wire-byte fixture remains absent; existing application-byte and transport-boundary tests do not establish historical wire identity.

## Closeout

DR-1 corrected stale summary and closure prose, clarified the unmeasured comparison, and preserved exact census JSON with verified hashes. Documentation checks passed; no runtime or performance claim follows.
