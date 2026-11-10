# SAFETY-0: Accepted Local Safety Contracts

Date: 2026-11-10
Authority: user's review/consolidate/proceed instruction; strict manager selected
dependency-ready SAFETY-0 for local documentation execution. Service ADR revision
8 is the decision; this audit is evidence, not implementation or deployment authority.

## Decision And Alternatives

| Boundary | Alternatives challenged | Accepted minimum |
| --- | --- | --- |
| Rooted I/O | check-then-read; component-openat chain; full-relative openat2 | Locked rustix1.1.4 `fs`, retained root, BENEATH/NO_SYMLINKS, no fallback. RawDir preserves enumeration errors. |
| Citation access | any beneath-root path; retained-generation lookup; active constructible tuple | Exact whole-section/chunk membership, all fields/batch checked before reads; old/unknown typed refusal. |
| Expensive work | timeout only; queued workers; fail-fast owned worker | One actual stdio worker/no application queue; explicit input/traversal/hash/construction counters owned through cancellation. |
| Publication | generic error after rename; rollback promise; explicit committed outcome | CURRENT rename is commit; separate durability uncertainty and maintenance failure, never destructive rollback. |

The root/bootstrap and open-time guarantee is explicit: trusted configured parent
prefix, no final root symlink, full-relative descendant opens, no global snapshot,
hard-link or mount-origin guarantee. Root-bound generation reads prevent mixing
a replaced root's generation and the retained root's documents. Originally
admitted citation membership preserves hidden whitelists and does not turn ignore
rules into ACLs or add another policy walk solely for verification.

ADR8's limits are conservative service policies, not measured capacities/RSS/time
bounds. Source/hash work is separate from the larger artifact boundary. Freshness
still hashes current bytes; no TTL/watchers. Construction reuses the existing
document lookup. Preflight and work ceilings reject, not truncate computation.

Publication review caught a missing durability prerequisite: first publication
must sync newly created store namespace entries in their containing parents;
reused candidates need file/directory sync too. These are now precommit checks.
HISTORY/cleanup are bounded and fixed-shape, protect CURRENT and retain the lock.
The trusted CLI administrator protects the store namespace during publication;
this is not hostile-administrator-safe publication. The parallel Locked cause
remains unproven; process/barrier evidence belongs to OD-3.9.

## Source And Review Evidence

Parent source/API inspection: tool receipts `654dcd`, `050b01`, `ea44ab`,
`d95376`, `26e207`, `f5be92`, terminal exit0. The broad installed-API search
yielded a live command; attempted interrupt reported closed stdin. A subsequent
process observation found no matching search process. No successful interrupt
is claimed; bounded exact registry paths supplied the API evidence afterward.

Main source seams: `src/corpus.rs` WalkBudget/bounded_dir/capture_ignore;
`src/cite.rs` verify/verify_bytes/read_beneath; `src/cli.rs` App::verify and
section_citation; `src/mcp.rs` index/blocking/verify; `src/store.rs` build,
freshness/load/publish/record_and_prune; `src/search/mod.rs` SearchIndex::new.
Installed rustix1.1.4 exposes openat2/RawDir; no runtime capability probe ran.

Both requested/effective reviewers were GPT-6-Astra, read-only explorers:

- Rooted reader/membership: agent `01a1c6e5-4a8f-7a02-a2c0-ceccd5e9bbf4`,
  `agent-results/01a1c6e5-4a8f-7a02-a2c0-ceccd5e9bbf4/1` discovery and `/2`
  delta review; final rooted/membership delta has no blockers.
- Work/publication: agent `01a1c6e5-4ad0-7970-86c8-e2cbdfb3035b`,
  `agent-results/01a1c6e5-4ad0-7970-86c8-e2cbdfb3035b/1` discovery; delta
  review `/2` confirmed the namespace durability correction and no remaining
  blockers in its assigned sections. Final clarification makes the trusted
  publication prerequisite explicitly include redirectable ancestors.

Children changed no files and ran no builds/tests. Reviews are static evidence.
Parent's committed OntoIndex semantic query returned unavailable indexed commit
identity. Source inspection is authority; no graph PASS or reindex. Git files
are untracked, so scope uses the pre-edit SHA inventory, not an empty diff.

## Validation And Scope

- `project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md`: PASS,
  39 tasks, no errors after contract/card changes. Advisory line/90%-parser-size
  warnings remain; do not erase authenticated task history to silence them.
- Python document checks: scope/whitespace/fence checker exit0 (`1d8a6b`);
  local references/heading anchors exit0 (`1e11ad`). Final closure checks and
  strict completion evidence are recorded in SAFETY-0's canonical task closeout.
  Final four-document scope/reference/anchor/fence/whitespace check exited0
  (`104a44`); revised plan validation again PASS39, no errors.
- Implementation writes: only service ADR, implementation plan and architecture.
  Authorized manager audit/status routing adds this audit, MEMORY/current/pending.
  Source, tests, Cargo files and evaluation payloads are unchanged by this task.
- Cargo, formatting, Clippy, kernel/race tests, load/performance tests and real
  evaluation were skipped: this task is documentation-only. Historical passing
  tests do not validate these unimplemented contracts.

Next implementation: OD-3.4, then OD-3.6/3.5 and OD-3.9. HTTP-0 stays blocked
on explicit owner exposure/admission choice; no listener, anonymous access,
model-for-human substitution, diagnostics acceptance or retrieval-quality claim.

Strict `complete-local` completed SAFETY-0 and selected OD-3.4, revision
`0aadde42aa7e2b7bdfa259a9a20ccebfeb9a50f7490e89206a54ecc479baac92`.
Counts: 24 DONE, 13 OPEN, 2 BLOCKED /39. Routers updated after that transition;
no task was dispatched or implemented by this closure. Recovery counts: zero
evidence corrections, rejected receipts, disk recoveries or resumed build jobs.
