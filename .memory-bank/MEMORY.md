# Memory Index

Active-only project router. Keep at or below 100 lines. Closed plans live under
[`archive/`](archive/MEMORY_ARCHIVE.md) and are not normal task context.

## Authorities

- [Current project status](project_plan-current.md) - in-progress plan and next task.
- [Pending tasks](project_pending-tasks.md) - waiting and blocked work.
- [Architecture](project_architecture.md) - owners and change-home rules.
- [Service ADR](ADR_ONTODOC_SERVICE.md) - accepted service contracts through revision 21 (implemented).
- [Evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md) - proposed existing-tool extensions and trusted CLI diagnostics; not accepted or implemented.
- [Hybrid retrieval](ADR_ONTODOC_HYBRID_RETRIEVAL.md) - proposed rev 3; [plan](../docs/HYBRID_RETRIEVAL_PLAN.md) blocked after stage 0.
- [Retrieval research](ADR_ONTODOC_RETRIEVAL_RESEARCH.md) - rev 4; [plan](../docs/RETRIEVAL_RESEARCH_PLAN.md) closed: step 1 met no threshold; [audit](audit_session-2026-11-16-retrieval-research.md).
- [Agentic retrieval](ADR_ONTODOC_AGENTIC_RETRIEVAL.md) - proposed bounded client policy and evaluation; no implementation approval.
- [Agent rules](../AGENTS.md) - repository-wide instructions.

## Active Plans

- [OntoDoc implementation plan](ONTODOC_IMPLEMENTATION_PLAN.md) - closed, 74 done. REL-6 closed 2026-11-15 as a local install of the corrected 0.1.0 (`~/.local/opt/ontodoc-0.1.0`, symlink `ontodoc-0.1.0`), with no public release; [local release audit](audit_session-2026-11-15-local-release.md). Original artifacts and custody are preserved.
- [0.1.2 release plan](../docs/RELEASE_0.1.2_PLAN.md) - authorized build, local install, commit and GitHub publication to private `er77/ontodoc`; in progress. Historical 0.1.0 evidence is preserved.
- [Refactoring plan](../docs/REFACTORING_PLAN.md) - closed, 5 DONE: `section_ranking` removed, `eval misses` pinned and moved to `eval::sweep`; [audit](audit_session-2026-11-15-refactoring.md).


## Recent Evidence

- [GPT review round 9](audit_session-2026-11-13-review-round9.md) - historical HTTP overflow finding and 22 verifier tests; corrected-candidate repairs completed, publication gates remain.
- [Held-out audit](audit_session-2026-11-13-held-out.md) - sole attempt accepted; H1 pass, H2/H5 exploratory intervals; defaults unchanged; custody consumed.
- [Preparation approval](audit_session-2026-11-12-preparation-approval.md) - owner pin accepted; unchanged manifest and bound inputs checked; EVAL-2.1 closed.
- [Review round 8 and preparation](audit_session-2026-11-12-review-round8.md) - ADR rev 14-15 pool extension then withdrawal of 12 unanswered queries; historical qrels gate passed (kappa 0.846).
- [EVAL-2.0 preparation](audit_session-2026-11-11-eval20-preparation.md) - owner decisions, FIX-13, query rounds 1-2.
- [Owner decisions](audit_session-2026-11-11-owner-decisions.md) - ten approved; ADR rev 10; HTTP-2 loopback listener, FIX-7/SIMPL-3/FIX-8 DONE.

## Standing Facts

- ADR19 generation invariants and operator recovery are implemented; [execution evidence](audit_session-2026-11-14-ontoindex-ideas.md) and [capacity observations](audit_session-2026-11-14-ontoindex-ideas-capacity.md) retain validation and limits. No retrieval-quality claim or size recommendation follows.
- Language: Rust 1.95.0, edition 2024, one package `ontodoc` at the repo root.
- Accepted HTTP design: `docs_share/<name>/` → `/<name>/mcp`, own `.ontodoc/`
  database per folder; equal access, no cross-corpus retrieval. Loopback `serve-http` behind an authenticated ingress.
- `ontocode-rs/` is a read-only source snapshot (`COPY_MANIFEST.json`); it is
  not buildable and must never be part of the Cargo build.
- Quality claims require the ADR protocol: section-level qrels, powered held-out
  split, frozen config + code freeze, paired bootstrap + randomization, Holm over
  {H2, H5}; outcomes are pass, negative or inconclusive.
