# Memory Index

Active-only project router. Keep at or below 100 lines. Closed plans live under
[`archive/`](archive/MEMORY_ARCHIVE.md) and are not normal task context.

## Authorities

- [Current project status](project_plan-current.md) - in-progress plan and next task.
- [Pending tasks](project_pending-tasks.md) - waiting and blocked work.
- [Architecture](project_architecture.md) - owners and change-home rules.
- [Service ADR](ADR_ONTODOC_SERVICE.md) - folder MCP/schema 2, local safety, custody, trusted preparation (rev 9), HTTP-0 exposure/limits (rev 10), prune via temp (rev 11), schema 3 (rev 12), html links/schema 4 and model-judged exploratory track (rev 13); measurement gated.
- [Evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md) - proposed existing-tool extensions and trusted CLI diagnostics; not accepted or implemented.
- [Agent rules](../AGENTS.md) - repository-wide instructions.

## Active Plans

- [OntoDoc implementation plan](ONTODOC_IMPLEMENTATION_PLAN.md) - 52/58 done (EVAL-2.0 pool, FIX-14 model-judged admission, 2026-11-12); EVAL-2.1 model judging active.

## Recent Evidence

- [EVAL-2.0 preparation](audit_session-2026-11-11-eval20-preparation.md) - owner decisions, FIX-13, query rounds 1-2.
- [Review rounds 6-7](audit_session-2026-11-11-review-round6.md) - FIX-10/11: numeric tests, test speed, prune self-heal (ADR rev 11); FIX-12 table-aware first sentence, schema 3 (rev 12).
- [Review round 5](audit_session-2026-11-11-review-round5.md) - FIX-9 staging order, ranking-sanity check, docs drift, plan archive.
- [Owner decisions](audit_session-2026-11-11-owner-decisions.md) - ten approved; ADR rev 10; HTTP-2 loopback listener, FIX-7/SIMPL-3/FIX-8 DONE.
- [Review round 4](audit_session-2026-11-11-review-round4.md) - FIX-5 pre-reservation refusals, FIX-6 heading cap DONE.
- [Review round 3](audit_session-2026-11-11-review-round3.md) - FIX-3 run-file bound, miss attribution, FIX-4 list caps DONE.
- [Review/simplify consolidation](audit_session-2026-11-11-review-simplify-consolidate.md) - FIX-1, SIMPL-1, HTTP-1, PERF-1 DONE.
- [EVAL-1.3 admission](audit_session-2026-11-11-eval13-progress.md) - DONE: preparation-pinned
  measurement, held-out pre-reservation admission; synthetic only, lane eval13-close-a exit0.
- [Publication outcomes/process locks](audit_session-2026-11-11-od39-publication-outcomes.md) -
  OD-3.9 complete; focused/default-parallel and process checks, Clippy/formatting
  passed; Astra reviewed. Historical parallel Locked cause remains unknown.
- [Input/work budgets](audit_session-2026-11-10-od35-input-work-budgets.md) -
  OD-3.5 complete; native input, preconversion and batch verification limits;
  actual-work cancellation/shutdown ownership. Focused checks and Astra passed.

- [Streaming freshness/admission](audit_session-2026-11-10-od36-streaming-admission.md) -
  OD-3.6 complete; focused and synthetic integration checks, Clippy/formatting
  passed; Astra delta reviews found no blockers. No measured RSS/quality claim.

- [Rooted reads/membership](audit_session-2026-11-10-od34-rooted-membership.md) -
  OD-3.4 complete; required tests, Clippy/formatting and Astra delta review passed;
  OD-3.5/3.6 budgets and OD-3.9 publication are now complete.

- [SAFETY-0 local contracts](audit_session-2026-11-10-safety0-local-contracts.md) -
  ADR8 reader/membership/work/publication decisions accepted; Astra delta review
  and document/plan checks passed. Local implementation complete; HTTP gated.

- [Fresh GPT challenge and safety priorities](audit_session-2026-11-10-review-safety-priorities.md) -
  stdio safety decoupled from HTTP exposure; bounded store repair completed,
  provenance/publication/default-promotion gaps assigned; measurement guarded.

- [EVAL-1.5 custody closure 2026-11-10](audit_session-2026-11-10-eval15-attempt-identity.md) -
  fixed registration/consumed attempts and exact-byte saved scoring; Astra-reviewed;
  serial library112, integration, protocol, Clippy/formatting passed. Measurement disabled.

- [EVAL-1.4 capture/traversal closure 2026-11-10](audit_session-2026-11-10-eval14-bounded-traversal.md) -
  split-isolated input capture, shared decoding and pre-filter traversal budgets;
  139 tests, Clippy and formatting passed; Astra reviewed. Measurement disabled.
- [Operational docs 2026-11-10](audit_session-2026-11-10-operations-docs.md) -
  README/agent guidance describes current stdio/schema/safety limits; Astra and document checks passed.
- [Schema-2 implementation 2026-11-10](audit_session-2026-11-10-schema2-implementation.md) -
  OD-3.2 identity/canonical artifacts, Astra review, 92 library + 5 CLI tests and Clippy passed.
- [Identity/canonical decision 2026-11-10](audit_session-2026-11-10-identity-canonical-decision.md) -
  OD-3.1 accepted schema 2, preserved anchors/current-byte freshness; implemented by OD-3.2.
- [MCP wire bounds 2026-11-10](audit_session-2026-11-10-mcp-wire-bounds.md) -
  OD-3.3 complete-frame cap/privacy, exact-ID compatibility limit and validation.
- [Consolidated GPT review 2026-11-10](audit_session-2026-11-10-consolidated-review.md) -
  selected robust repairs, simplified evaluation approach, first publication/load fix and current validation.
- [GraphRAG donor ideas 2026-11-10](audit_session-2026-11-10-graphrag-donor-ideas.md) -
  Astra-challenged proposals: rendered-source provenance, context accounting and topology diagnostics.
- [OntoIndex graph ideas 2026-11-10](audit_session-2026-11-10-ontoindex-graph-ideas.md) -
  Astra-challenged recommendations: identity, bounds and evidence before semantic edges.
- [Folder-scoped MCP decision 2026-11-10](audit_session-2026-11-10-folder-mcp-architecture.md) -
  one named endpoint and independent database per `docs_share/` child; design only.
- [Prior-attempt review 2026-11-08](audit_session-2026-11-08-prior-attempt-review.md) -
  why `ontocode-rs/` is reference-only and which patterns are reused.
- [Donor ideas review 2026-11-08](audit_session-2026-11-08-donor-ideas-review.md) -
  OpenViking and pharma-ltm-assistant ideas adopted and rejected (ADR rev 2).
- [MCP donor review 2026-11-09](audit_session-2026-11-09-mcp-donor-review.md) -
  Astra-challenged ideas; identifier/path discovery and HTML preparation deferred.
- [Shared HTTP challenge 2026-11-09](audit_session-2026-11-09-http-service-challenge.md) -
  optional single-access-domain HTTP recommended; security/work bounds gate adoption.
- [Shared-corpus clarification 2026-11-09](audit_session-2026-11-09-shared-corpus-clarification.md) -
  equal reader access; single-corpus assumption superseded by ADR rev 4.
- [Implementation session 2026-11-08](audit_session-2026-11-08-implementation-od0-eval12.md) -
  OD-0.0..EVAL-1.2 closeouts, commands, exit codes, deviations, EVAL-2.0 blocker.
- [Astra query-review amendment 2026-11-08](audit_session-2026-11-08-astra-query-review.md) -
  query-review exception, remaining human annotation gate and integrity findings.

## Standing Facts

- Language: Rust 1.95.0, edition 2024, one package `ontodoc` at the repo root.
- Accepted HTTP design: `docs_share/<name>/` → `/<name>/mcp`, own `.ontodoc/`
  database per folder; equal access, no cross-corpus retrieval. Loopback `serve-http` behind an authenticated ingress.
- `ontocode-rs/` is a read-only source snapshot (`COPY_MANIFEST.json`); it is
  not buildable and must never be part of the Cargo build.
- Quality claims require the ADR protocol: section-level qrels, powered held-out
  split, frozen config + code freeze, paired bootstrap + randomization, Holm over
  {H2, H5}; outcomes are pass, negative or inconclusive.
