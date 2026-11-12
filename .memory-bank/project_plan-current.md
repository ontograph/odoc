---
name: Current project status
type: project
---

# Current Project Status

Updated: 2026-11-12

| Canonical plan | State | Active task | Counts |
| --- | --- | --- | --- |
| [ONTODOC_IMPLEMENTATION_PLAN.md](ONTODOC_IMPLEMENTATION_PLAN.md) | in-progress | EVAL-2.1 (qrels prepared, owner pin pending) | 53 DONE, 6 OPEN; 59 total |

- 2026-11-12 review round 8 ([audit](audit_session-2026-11-12-review-round8.md)):
  owner approved withdrawing the 12 answerable queries still unlabelled after
  the depth-30 extension (ADR rev 15). 329 queries, split recomputed, pool
  3,938 pairs; qrels gate passed, kappa 0.846 [0.820, 0.871], test answerable
  165. FIX-15 (pool extension, Astra P1 fixed) DONE. Judging record committed
  in `eval/judging/`. Plan cards EVAL-3.2/OD-4.1/OD-4.0 aligned with rev 13.
  Next: owner pins `eval/preparation.json` (1e4835cd…), close EVAL-2.1, then
  EVAL-3.0 dev runs. Older entries: [archive](archive/project_plan-current-2026-11-12.md).

- 2026-11-12 EVAL-2.0 DONE: operator approved review digest 94aaeff0; full-grid
  pool 4,004 pairs (fc5c8a81), reproducible; pools gates pass. FIX-14 DONE:
  model-judged notes/records admitted only under ADR rev 13 rules; reports
  carry `label_provenance` and downgrade H2/H5 to exploratory (Astra review
  P2s fixed). Next: EVAL-2.1 blinded judging by GPT-6-Astra and
  claude-opus-5-5 high.

- 2026-11-11 EVAL-2.0 preparation ([audit](audit_session-2026-11-11-eval20-preparation.md)):
  owner decisions; FIX-13 DONE (`.html` links, `src/` corpus root, schema 4,
  ADR rev 13 model-judged exploratory track). 341 queries assembled; label-free
  gates pass. Pooling waits for the operator's query-approval audit.

See [pending tasks](project_pending-tasks.md) for gates and external dependencies.
