# Audit Session: Code Review And Agent Guidance

Date: 2026-11-09
Scope: user-requested codebase review and `AGENTS.md` update. No implementation,
ADR, plan, task-state or evaluation-input changes. All findings are source
inspection, not executed reproductions or measured capacity results.

## Findings

| Priority | Finding and trigger | Evidence and consequence |
| --- | --- | --- |
| P1 | Preamble and punctuation-only heading collide. `Intro.\n\n# !!!\n\nBody.\n` produces two sections named `a.md#` and first chunks named `a.md#~0`. | `src/parse.rs:94-111,225-230`; `src/chunk.rs:36-39`. ID maps overwrite entries (`src/tiers.rs:44-54`; `src/search/mod.rs:143-159`), confusing tiers, graph edges and section-level evaluation. Require uniqueness; identity changes need ADR/schema review. |
| P1 | Response caps do not cover complete encoded results. Control characters in section text can expand beyond the raw-text allowance. | `src/cli.rs:351-358,578-592`; `src/mcp.rs:92-95`. List bounding also returns an oversized fixed envelope when nothing can be removed. `tests/mcp.rs:187-190` checks inner search JSON only. This conflicts with the ADR serialized-byte promise (`ADR_ONTODOC_SERVICE.md:143-146`). |
| P1 | Citation verification accepts arbitrary beneath-root paths, independent of the published corpus; symlink/size checks precede a separate path read. | `src/cli.rs:633-644`; `src/cite.rs:58-61,102-122`. Ignored/non-Markdown files expose existence/hash-match information; concurrent filesystem mutation can invalidate containment/size checks. Equal corpus access does not authorize every checkout file. No race exploit was attempted. |
| P1 before remote exposure | Blocking work has no admission/cooperative cancellation; ordinary requests also read/hash the corpus and verification can hash every overlapping window. | `src/mcp.rs:125-136,191-200`; `src/cli.rs:496,582,626,649`; `src/store.rs:365-372`; `src/corpus.rs:154-167`; `src/cite.rs:66-83`. HTTP alone would multiply existing work, not establish capacity. Detailed remedies remain in the HTTP challenge; retain freshness semantics until authority is reconciled. |
| P2 | Canonical artifacts include mtime although generation identity omits it. Touch-only changes retain the ID but vary serialized bytes. | `src/store.rs:88-93,223-229,322-326`. This conflicts with byte-identical generations (`ADR_ONTODOC_SERVICE.md:157-159`). The test compares artifacts before touching, then IDs only afterward (`src/store.rs:637-654`). |
| P2 | Many short headings induce quadratic index construction within file/chunk caps. | Graph events linearly search chunks (`src/graph.rs:83-84,146-150`); each tier scans preceding sections (`src/tiers.rs:55,104-112`). Output caps do not bound earlier CPU work. Complexity is source-derived; runtime was not measured. |
| P2 | Older evaluation success-path tests contradict deliberate refusal guards. | `src/eval/run.rs:474-478,584-586,930-932`; `src/eval/sweep.rs:133-141`; `tests/eval_run.rs:95-103`; `tests/eval_gates.rs:253-275`. Preserve guards and align tests within the authorized evaluation task; no test failure was executed here. |
| Readiness gap | Combined qrels and incomplete process evidence still block benchmark readiness. | `Bench::qrels` loads all labels (`src/eval/run.rs:230-247`). Reachable `check-frozen` reads them and held-out answerability (`src/cli.rs:848-860`; `src/eval/sweep.rs:537-556`). Gate configuration does not certify query review or human annotation (`src/eval/gates.rs:69-78`; `src/eval/run.rs:341-356`; `ADR_ONTODOC_SERVICE.md:241-245,259-264`). Do not use these paths with real held-out data during ordinary development. |

## Guidance Decisions

Keep one Rust package, existing retrieval/store owners and atomic publication.
The implemented service remains five read-only stdio tools. The selected future
deployment is one corpus with equal document access, without tenant routing or
per-user indexes; HTTP still needs an approved ADR amendment and scoped tasks.

`AGENTS.md` now distinguishes requirements from current guarantees and reviews
from implementation authorization. It requires complete-response and work
bounds, actual work ownership across cancellation, corpus-membership checks,
unique IDs and canonical-byte determinism checks. It preserves donor boundaries,
adds the unresolved-licence MCP donor and requires failure-path regression
checks. Documentation-only validation avoids unrelated Rust compilation;
Rust task gates remain unchanged. No new dependency or abstraction is proposed.

Independent GPT-6-Astra review caught two wording problems: chunk-budget
independence belongs to section IDs, and CI width is not an ADR outcome rule.
The final text separates section stability from ID uniqueness, uses the exact
H2/H5 decision rule and prohibits an underpowered held-out run. Evaluation
refusals, review-before-pooling, human annotation and burned-split rules remain.

## Evidence And Limits

- OntoDoc is absent from the OntoIndex registry; source supplied evidence.
- Read-only reviewers: `/root/retrieval_source`, `/root/eval_source`, and
  `/root/agents_astra`. Astra's dispatch confirms requested/effective model
  `gpt-6-astra`. They changed no files and executed no validation commands.
- Source evidence receipts: parent `448293`, `996eed`, `c59454`, `76f2db`,
  `b93a96`; retrieval reviewer `7a9e23`, `e00c96`, `fa0c96`, `aaaa59`, `eee82f`;
  evaluation reviewer `8c9419`, `ffa73f`, `ab209a`, `f16c49`, `a3ce0b`;
  Astra `557da9`, `44e65a`, `0d57c3`, `71bcac`, `23c208`, `c49ee6`.
  Source-inspection commands exited **0**; this is not runtime validation.
- Two initial fast-review dispatches failed with provider HTTP 429 before any
  tool use; successful replacement reviewers supplied the evidence above.
- Parent `AGENTS.md` SHA-256 before editing:
  `2e62b5bc996713cffc7d85e005e4affd203601d875ed5983e33ceed248babfa3`.
- No builds, Rust tests, format, Clippy, benchmark runs, real held-out data
  access, network probes or load tests were performed. No code defect is fixed
  by these instructions. No measured agent-effectiveness claim or scaffold A/B
  evaluation is made; that requires its own frozen tasks and acceptance record.
- The project is largely untracked, so a Git diff alone does not establish the
  edit boundary.

## Validation

- Inline `python3` documentation/reference/hash validation exited **0**
  (`07df8a`): all three edited documents passed title, fence, whitespace and
  reference checks; 16 relative links and 30 source-line references resolved;
  the memory router remains below 100 lines. Retained evaluation/security/build
  safeguards and section-only ID stability were checked explicitly.
- The same check compared 54 pre-edit file hashes: 52 protected files remain
  unchanged, including runtime, tests, evaluation inputs, ADR, plan and status.
  Existing edited files are only `AGENTS.md` and `.memory-bank/MEMORY.md`; this
  audit is the sole new document. Protected source/input inventories are
  unchanged. The AGENTS diff was inspected from saved pre-edit bytes because
  the file is untracked.
- `git diff --check` exited **0** (`20c8c5`); its untracked-file limitation is
  covered by the direct content checks above.
- Astra's final delta review (`f3ef54`, source inspection exit **0**) confirmed
  both requested wording fixes and found no additional issue in that delta.
- Rust tests, formatting, Clippy and scientific evaluation were skipped for
  this documentation-only change; no implementation task is closed.
