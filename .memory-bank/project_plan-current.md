---
name: Current project status
type: project
---

# Current Project Status

Updated: 2026-11-11

| Canonical plan | State | Active task | Counts |
| --- | --- | --- | --- |
| [ONTODOC_IMPLEMENTATION_PLAN.md](ONTODOC_IMPLEMENTATION_PLAN.md) | in-progress | EVAL-2.1 (model-judged labels, ADR rev 13) | 52 DONE, 6 OPEN; 58 total |

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

- 2026-11-11 review round 7 ([audit](audit_session-2026-11-11-review-round6.md#round-7-same-day-challenge-of-round-6)):
  FIX-11 DONE: prune moves to `tmp-<id>` before deleting (ADR rev 11), so an
  interrupted prune self-heals; slug finding closed as no bug; fusion tie and
  HTTP read-deadline tests. FIX-12 (owner-approved): table-aware first sentence,
  shared `MARKDOWN_OPTIONS`, schema 3 (ADR rev 12); reindex existing stores.

- 2026-11-11 review round 6 ([audit](audit_session-2026-11-11-review-round6.md)):
  FIX-10 DONE: hand-computed BM25/PPR/fusion edge tests, optimized dev hash
  crates (suite ~18 s). Parser soft-break slug, table-aware first sentence and
  prune recovery deferred to the owner (identity/ranking/ADR).

- 2026-11-11 review round 5 ([audit](audit_session-2026-11-11-review-round5.md)):
  FIX-9 DONE. Dev staging was restored after validation, and the ADR
  discrimination check is `tests/ranking_sanity.rs`. Docs drift was fixed. DONE
  validation summaries were archived, taking the plan from 122 KB to 108 KB.
  Evaluation code is frozen while parked.

- 2026-11-11 owner packet approved ([decisions](audit_session-2026-11-11-owner-decisions.md)):
  ADR revision 10; HTTP-0, HTTP-2, FIX-7, SIMPL-3, FIX-8 DONE via strict
  complete-local. HTTP-2: loopback `serve-http`, stateless JSON rmcp per
  `docs_share` folder, Origin/Host/size/admission refusals; Astra P1
  (disconnect leak) and two P2 (connection admission/read deadline, rmcp error
  bodies) fixed with regressions. FIX-7 `text_truncated` and null n = 0
  statistics; SIMPL-3 one kappa; FIX-8 staging before reservation. All-targets
  (lib 210 passed 1 ignored; cli 8, gates 20, run 14, http 4, mcp 12), fmt,
  Clippy exit0. Remaining work is EVAL-2.0 onward, parked on human inputs.

- 2026-11-11 review round 3 ([audit](audit_session-2026-11-11-review-round3.md)):
  FIX-3 (all run files sized against one 16-MiB scoring payload from pinned
  test ids before reservation; zero-overlap misses are lexical_miss; unused
  scratch removed) and FIX-4 (lazy cap+1 list collection) DONE via strict
  complete-local. All-targets 256 passed 1 ignored, fmt, Clippy exit0.
  Remaining tasks are owner/human-gated (HTTP-0, EVAL-2.0).

- 2026-11-11 review consolidation ([audit](audit_session-2026-11-11-review-simplify-consolidate.md)):
  four tasks DONE through strict complete-local.
  - FIX-1: raw-byte moved-citation lines, dev resamples bounded to 1..=10000,
    preparation layout checked before corpus inspection.
  - SIMPL-1: freeze sweeps once; dead saved-run branch removed; one shared
    quantile.
  - HTTP-1: `mcp::Registry` over `docs_share` with no listener.
  - PERF-1: index lock covers only reload/snapshot; `with_permits(1..=64)`.

  Final lane perf1-c exit0 (lib 199, mcp 12, cli 8, gates 20, run 13, fmt,
  Clippy). The 34 DONE `expected_evidence` lists were archived verbatim, taking
  the plan from 122 KB to 100 KB. Next: HTTP-0 owner choice, then HTTP-2
  listener; EVAL-2.x stays owner/human-gated.

- EVAL-1.3 is DONE through strict complete-local: every measurement command
  requires `--preparation`/`--preparation-sha256`; held-out run/score admit before
  reservation; selected capture binds config/splits/gates before payload reads;
  layout refuses benchmark payloads inside corpus checkouts. Lane eval13-close-a
  exit0 (lib 196, gates 20, run 11, cli 8, fmt, Clippy -D warnings). Synthetic only;
  [audit](audit_session-2026-11-11-eval13-progress.md). Manager now blocked on owner gates.

- EVAL-1.6 is DONE through strict complete-local: ADR9 query-review/human provenance, trusted preparation
  and whole-vs-selected commitments, attempt acceptance and H5-first promotion.
  New OD-4.1 separates default implementation/no-op from EVAL-3.2 measurement;
  omitted threshold stays disabled. Astra's two document findings corrected;
  [decision audit](audit_session-2026-11-11-eval16-preparation-decision.md).
  Historical tracker scalars are preserved verbatim in a linked audit; no code,
  labels or measurement guards changed. Plan PASS40; eight-document/scope and
  historical-state checks plus git diff --check exit0. (EVAL-1.3 later DONE.)

- OD-3.9 is DONE through strict `complete-local`: CURRENT commit outcomes,
  finite CLI warnings, bounded HISTORY/fixed-shape cleanup and current protection.
  Default-parallel store43 plus CLI units6/integration8, MCP11 and synthetic
  gates14/run6 passed; final Clippy/formatting exit0. Astra found no scoped
  blockers after corrections. Process barriers prove ownership/release/lifetime,
  not the historical parallel `Locked` cause, which remains unknown.
  [Closure audit](audit_session-2026-11-11-od39-publication-outcomes.md).
  EVAL-1.6 is complete; tracker evidence compaction preserves authenticated
  history and keeps the active file below its parser warning threshold.

- OD-3.5 is DONE through strict `complete-local`: raw native input and borrowed
  argument limits precede conversion/work; batch verification precharges source
  and SHA budgets. One actual worker owns admission/cache mutation through
  cancellation and runtime shutdown. Required citation/MCP and selected App/CLI/
  synthetic evaluation tests passed; final all-target Clippy and formatting exit0.
  GPT-6-Astra delta review found no scoped blockers. Three distinct failures and
  corrections, external AGENTS delta, truncated unit totals and scope limits are
  in the [closure audit](audit_session-2026-11-10-od35-input-work-budgets.md).

- OD-3.6 is DONE through strict `complete-local`: shared accepted-file traversal,
  text-free sorted freshness manifests, local/residual reservations and detection
  charges; allocation-light collection preflight and checked actual tokenizer
  input before BM25. Required focused tests and synthetic CLI/MCP/evaluation
  regressions, all-target Clippy and formatting passed. Two GPT-6-Astra source
  reviewers found no scoped blockers. One fixture ordering failure was corrected;
  [closure audit](audit_session-2026-11-10-od36-streaming-admission.md) records
  exact commands/receipts and trusted constructor/performance limits.

- OD-3.4 is DONE through strict `complete-local`: retained-root openat2/RawDir,
  root-bound generation/current-byte reads and exact active chunk/section
  membership before target access. Required focused/integration tests and App
  regressions passed; final Clippy/formatting exit 0. Two GPT-6-Astra delta
  reviewers found no remaining scoped blockers. Earlier compile/fixture/lint
  failures and limits are in the [closure audit](audit_session-2026-11-10-od34-rooted-membership.md).
  Its OD-3.6 successor is now complete; no HTTP, measurement or full-library/performance claim.

- SAFETY-0 is DONE through strict `complete-local`: ADR8 accepts rooted I/O,
  active-generation citation membership, numerical local budgets, work-owned
  admission and commit-point outcomes. Two GPT-6-Astra delta reviews and document/
  plan checks passed. [Decision audit](audit_session-2026-11-10-safety0-local-contracts.md).
  That decision changed no code/tests or HTTP exposure; OD-3.4 is now implemented.

Three GPT-6-Astra reviews produced the
[consolidated review and decisions](audit_session-2026-11-10-consolidated-review.md).
The 15 historical closeouts remain unchanged; they are not current-tree proof.

- OD-3.0 is DONE through strict `complete-local`; code and three regressions
  are in `src/store.rs` only. Store tests: 12 passed, exit 0; formatting: exit 0;
  managed all-target Clippy: exit 0. IDs/schema and evaluation guards unchanged.
- OD-3.3 is DONE through strict `complete-local`: native complete-frame bounds,
  escaping-aware truncation/refusal and generic internal errors. Tests: 85
  library + 4 CLI + 7 MCP passed; formatting and all-target Clippy exit 0.
  See [wire-bound evidence and limits](audit_session-2026-11-10-mcp-wire-bounds.md).
- OD-3.1 is DONE through strict `complete-local`: accepted service ADR revision 5
  records schema-2 IDs/canonical bytes, preserved anchors/current-byte freshness
  and the narrow tier LF exception. Astra challenged both flows; document/plan
  checks passed. No code or Rust validation in this documentation task.
- OD-3.2 is DONE: schema-2 IDs preserve Markdown anchors; canonical records omit
  mtime and include stable skips in identity. Legacy schemas require reindexing;
  same-ID index/meta conflicts refuse without changing target/pointers/history.
  Tests: 92 library + 5 CLI passed; formatting and all-target Clippy exit 0.
  Astra source review found no blockers. See [implementation evidence](audit_session-2026-11-10-schema2-implementation.md).
- HTTP-0 is BLOCKED by its explicit authority stop: the owner must choose local
  loopback-only HTTP or authenticated trusted-ingress remote delivery and accept
  the admission/session trust boundary. [Review and proposed constraints](audit_session-2026-11-10-http0-authority-gate.md)
  are not acceptance or permission to expose a listener. No HTTP code changed.
- OD-3.7 is DONE: README covers installation, stdio client configuration, five
  tools, reindexing and unresolved safety/evaluation/HTTP limits; donor snapshot
  remains read-only. Astra and document checks passed, no Rust execution.
  [Audit](audit_session-2026-11-10-operations-docs.md).
- EVAL-1.4 is DONE through strict `complete-local`: split-specific captures,
  identity-bound mechanical evidence, cached indexes and exact-byte scoring;
  corpus-owned pre-filter enumeration and bounded explicit ignore ingestion
  close the earlier ignore-internal budget gap. Final 107 library, 14 gates,
  6 run, 5 CLI and 7 MCP tests passed; Clippy/formatting exit 0. Astra found no
  remaining source-level traversal blocker. Successfully readable regular
  controls preserve policy; unsafe/unreadable controls refuse. Ordinary walker
  symlink/ancestor limitations remain service-repair work.
  [Closure](audit_session-2026-11-10-eval14-bounded-traversal.md).
- EVAL-1.5 is DONE through strict `complete-local`: ADR revision6 fixed Git
  common-dir registration/consumed attempts, exact-byte external export and
  completed meta/report binding. Astra correction removed metadata reread.
  Required integration, serial library112, formatting/Clippy and CLI5/MCP7
  passed; initial fixture failure corrected, parallel store `Locked` remains
  a recorded validation limitation. [Audit](audit_session-2026-11-10-eval15-attempt-identity.md).
- Current review completed OD-3.8 bounded store reads/publication, SAFETY-0
  local contracts, OD-3.4 rooted membership and OD-3.6 freshness/load work;
  OD-3.5 actual-work admission and OD-3.9 publication outcomes are complete.
  [Fresh GPT challenge](audit_session-2026-11-10-review-safety-priorities.md) records
  alternatives, forgotten failures and execution; no quality/performance claim.
- OD-3.8 is DONE through strict complete-local: final managed job d passed
  store23, CLI5/MCP7/gates14/run6, formatting and all-target Clippy, exit0.
  Astra source/delta reviews found no blockers. Initial lock refusals and one
  corrected Clippy expression remain recorded; no full-library/HTTP claim.
- EVAL-1.6 records provenance/preparation/default-promotion admission, then
  EVAL-1.3 completes all public success/failure paths. Guards stay intact.
- Local safety no longer depends on HTTP exposure or annotation; HTTP-0 stays
  owner-blocked and composes accepted local contracts with network/session limits.
- Evaluation: blocked EVAL-2.0 (owner: corpora, query approval, annotators) → EVAL-2.1 →
  EVAL-3.0/3.1/3.2 → OD-4.1 → OD-4.0. Public measurement remains disabled.
- Folder HTTP: HTTP-0, HTTP-1, HTTP-2 DONE (loopback listener behind an
  authenticated ingress). Diagnostics remain Proposed.

See [pending tasks](project_pending-tasks.md) for gates and external dependencies.
