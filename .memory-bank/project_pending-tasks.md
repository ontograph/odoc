---
name: Pending tasks
type: project
---

# Pending Tasks

Updated: 2026-11-11. The structured
[canonical plan](ONTODOC_IMPLEMENTATION_PLAN.md) controls status and ordering;
this file routes blockers, not a second queue.

## Current (2026-11-11)

- Service work is complete: HTTP-2 loopback listener, FIX-7, SIMPL-3, FIX-8 and
  FIX-9 (round 5) DONE ([owner decisions](audit_session-2026-11-11-owner-decisions.md)).
- EVAL-2.0 (BLOCKED) is parked by the owner pending pinned corpora, a trusted
  operator and two human annotators; EVAL-2.1 → EVAL-3.x → OD-4.1 → OD-4.0
  follow it. Delivery item outside code: the operator's authenticated TLS
  ingress in front of `serve-http`.
- Entries below are history.

## Active Service Repair

- SAFETY-0 and OD-3.4 are DONE: ADR8 local contracts and
  [rooted read/membership implementation](audit_session-2026-11-10-od34-rooted-membership.md).
  OD-3.6 is also DONE ([streaming/admission audit](audit_session-2026-11-10-od36-streaming-admission.md));
  OD-3.5 input/verification/actual-work budgets are DONE ([closure audit](audit_session-2026-11-10-od35-input-work-budgets.md)).
  OD-3.9 is DONE ([publication/lock audit](audit_session-2026-11-11-od39-publication-outcomes.md)).
  EVAL-1.3 is DONE. HTTP-0 still needs the owner exposure/admission choice; completed
  local work is not multi-user readiness or measured capacity.

- OD-3.8 bounded store reads/publication is DONE: final managed job d passed
  store23 plus integration32, formatting and Clippy, exit0. Initial admission
  denials and the corrected Clippy expression remain recorded; no lock bypass.
  [Current review/audit](audit_session-2026-11-10-review-safety-priorities.md)
  records exact results, source-review limits and the next local safety sequence.

- OD-3.0 is DONE: duplicate section/chunk publication and load reject. Store
  tests (12 passed), formatting and managed all-target Clippy exited 0. Existing
  malformed generations reject, not migrate; empty-slug IDs landed in OD-3.1/3.2.
- OD-3.3 is DONE: complete native MCP frame/error bounds and generic internal
  messages. Exact escaped IDs preserved or connection rejected before dispatch;
  85 library + 4 CLI + 7 MCP tests, formatting and all-target Clippy exit 0.
- OD-3.1 is DONE: service ADR revision 5 accepts ID-only `@empty`, canonical skip
  records/identity, no persisted mtime, same-ID byte conflicts and unchanged
  current-byte freshness. Document checks passed; no implementation claimed.
- OD-3.2 is DONE: schema 2, unique empty-slug IDs, canonical bytes and strict
  version/ID/conflict rejection; 92 library + 5 CLI tests, formatting and Clippy
  passed. [Audit](audit_session-2026-11-10-schema2-implementation.md) records Astra review and limits.
- HTTP-0 is BLOCKED: owner exposure/admission choice remains unsupplied. Pick
  local loopback-only use or authenticated trusted-ingress remote delivery and
  accept the admission/session trust boundary. [Review](audit_session-2026-11-10-http0-authority-gate.md)
  records proposals only. HTTP-1 (registry, no listener) is DONE; HTTP-2 depends on HTTP-0;
  no listener or automatic indexing. Local safety decisions no longer depend on
  HTTP-0: OD-3.8 → SAFETY-0 → OD-3.4 → OD-3.6 → OD-3.5 → OD-3.9 are DONE.
  Publication/process-lock tests pass; the historical parallel Locked cause
  remains unknown rather than claimed fixed.
- OD-3.7 is DONE: service-use and agent documentation now matches the implemented
  stdio/schema-2 service and its unresolved safety/HTTP/evaluation limits.

## Evaluation: Guarded, Not Complete

- EVAL-1.6 is DONE through strict complete-local: trusted preparation/provenance
  handoff and deterministic default-promotion policy before EVAL-1.3.
  ADR9 decision is documented and Astra-reviewed; historical tracker evidence is
  compacted with exact scalar/hash preservation. EVAL-1.3 later DONE.
- EVAL-1.4 is DONE: immutable physical split capture, current freshness/pins and
  shared indexes; pre-filter traversal and explicit ignore-input budgets close
  the earlier gap. Final 139 tests and Clippy/formatting passed; Astra reviewed.
  Unsafe/unreadable controls refuse; OD-3.4 now supplies shared rooted reads,
  and OD-3.6 now supplies streaming/load admission.
  [Closure](audit_session-2026-11-10-eval14-bounded-traversal.md).
- EVAL-1.5 is DONE: ADR revision6 resolves fixed common-dir custody/external
  exports and audits; exact completed metadata/report binds saved-run hashes.
  [Audit](audit_session-2026-11-10-eval15-attempt-identity.md) records Astra review and validation.
- EVAL-1.3 is DONE: preparation-pinned measurement commands, selected-only
  capture, held-out admission before reservation and synthetic end-to-end
  dev-then-one-held-out success. [Audit](audit_session-2026-11-11-eval13-progress.md).
- Dev loading must not open held-out query/label payloads. Validate and compute
  from identical captured bytes; full preparation has a different evidence scope.
  ADR9 also excludes combined/full-pool/full-provenance payloads from ordinary
  dev run/sweep/freeze/check/misses/score; trusted preparation recaptures and
  reapproves dev-selected config before freeze. Metadata/hash pins alone do not
  authenticate humans, reviewers or pre-pool chronology.
- Reuse audit preregistration and an exclusive durable attempt record; a new
  registry service is not required. A mutable local record is not external custody.
- EVAL-3.x external benchmark/binary/custody/audit commands are rebound under
  ADR6; EVAL-1.3 must validate them before measurement. Registration is one-shot,
  with no delete/reset workflow; superseding custody needs fresh approval.
- EVAL-2.0 remains BLOCKED on the trusted operator: pinned permissive corpora,
  manager-owned query-approval audit before pooling. At least 270 reviewed stratified queries,
  including >=150 answerable and >=30 off-topic held-out queries, are still needed.
  GPT-6-Astra query review is authorized; model review is never human annotation.
- EVAL-2.1 requires two human annotators and annotation provenance; no substitution
  with agents. Subsequent dev/power/freeze/held-out tasks retain every ADR gate.
- EVAL-3.2 recommends only under preregistered h5-then-h2-v1; new OD-4.1
  implements the audited default or records no-op before OD-4.0. Omitted
  threshold stays disabled, explicit modes/weights remain supported, and no
  mutable evaluation artifact enters retrieval.
- Baseline commits exist (2026-11-11, see `git log`). A clean, matching commit
  and binary identity at freeze time remain a prerequisite the operator
  supplies; they are not manufactured during review.

## Deferred Decisions

- Evidence/diagnostics ADR remains Proposed. No new fields, tools, persistent
  provenance, topology command or publication-workflow API is accepted here.
- No ranking change, donor-code adoption or retrieval-quality claim is justified
  by this source review. Service safety tests use only synthetic fixtures.
