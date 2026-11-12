# Memory Index

Active-only project router. Keep at or below 100 lines. Closed plans live under
[`archive/`](archive/MEMORY_ARCHIVE.md) and are not normal task context.

## Authorities

- [Current project status](project_plan-current.md) - in-progress plan and next task.
- [Pending tasks](project_pending-tasks.md) - waiting and blocked work.
- [Architecture](project_architecture.md) - owners and change-home rules.
- [Service ADR](ADR_ONTODOC_SERVICE.md) - folder MCP/schema 2, local safety, custody, trusted preparation (rev 9), HTTP-0 exposure/limits (rev 10), prune via temp (rev 11), schema 3 (rev 12), html links/schema 4 and model-judged exploratory track (rev 13), pool extension (rev 14), withdrawal (rev 15); measurement gated.
- [Evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md) - proposed existing-tool extensions and trusted CLI diagnostics; not accepted or implemented.
- [Agent rules](../AGENTS.md) - repository-wide instructions.

## Active Plans

- [OntoDoc implementation plan](ONTODOC_IMPLEMENTATION_PLAN.md) - 53/59 done (FIX-15 pool extension); EVAL-2.1 qrels prepared, waits for the owner's manifest pin.

## Recent Evidence

- [Review round 8 and preparation](audit_session-2026-11-12-review-round8.md) - ADR rev 14-15 pool extension then withdrawal of 12 unanswered queries; qrels gate passed (kappa 0.846), manifest awaits the owner pin.
- [EVAL-2.0 preparation](audit_session-2026-11-11-eval20-preparation.md) - owner decisions, FIX-13, query rounds 1-2.
- [Review rounds 6-7](audit_session-2026-11-11-review-round6.md) - FIX-10/11: numeric tests, test speed, prune self-heal (ADR rev 11); FIX-12 table-aware first sentence, schema 3 (rev 12).
- [Review round 5](audit_session-2026-11-11-review-round5.md) - FIX-9 staging order, ranking-sanity check, docs drift, plan archive.
- [Owner decisions](audit_session-2026-11-11-owner-decisions.md) - ten approved; ADR rev 10; HTTP-2 loopback listener, FIX-7/SIMPL-3/FIX-8 DONE.
- [Review round 4](audit_session-2026-11-11-review-round4.md) - FIX-5 pre-reservation refusals, FIX-6 heading cap DONE.

## Standing Facts

- Language: Rust 1.95.0, edition 2024, one package `ontodoc` at the repo root.
- Accepted HTTP design: `docs_share/<name>/` → `/<name>/mcp`, own `.ontodoc/`
  database per folder; equal access, no cross-corpus retrieval. Loopback `serve-http` behind an authenticated ingress.
- `ontocode-rs/` is a read-only source snapshot (`COPY_MANIFEST.json`); it is
  not buildable and must never be part of the Cargo build.
- Quality claims require the ADR protocol: section-level qrels, powered held-out
  split, frozen config + code freeze, paired bootstrap + randomization, Holm over
  {H2, H5}; outcomes are pass, negative or inconclusive.
