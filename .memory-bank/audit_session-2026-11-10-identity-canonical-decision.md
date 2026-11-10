# OD-3.1: Identity And Canonical-Artifact Decision

Date: 2026-11-10
Status: Decision and document validation complete; strict closeout below.
Authority: consolidated review-and-proceed instruction; OD-3.1 task card.

## Evidence And Conflict Resolution

Current source inspected, not executed:

- `src/parse.rs:80-120,212-232`: preamble and first empty heading share an ID;
  document-wide anchor allocation already handles duplicate/literal suffixes.
  `src/chunk.rs:25-40` appends ordinals after parsing, independently of section
  identity. `src/graph.rs:167-207` resolves allocated anchors, while
  `src/cli.rs:715-726` prefers exact IDs over aliases.
- `src/store.rs:90-95,243-250,342-347`: serialized mtime changes artifact bytes
  without changing generation ID. `src/store.rs:294-301` serializes Debug skip
  reasons that may carry raw I/O text from `src/corpus.rs:118-125`; current IDs
  exclude skip records. These conflict with canonical bytes and relative paths.
- `src/store.rs:385-425` hashes current bytes without consulting stored mtime;
  the ADR's metadata-first description was inaccurate. Revision 5 explicitly
  selects current-byte hashing, without introducing a cache or weakening verify.
- `src/store.rs:450-490,529-547`: verified same-ID targets are reused without
  comparing candidate bytes. Same-store republish cannot prove determinism.
- `src/tiers.rs:178-196` inserts LF separators. Revision 5 expressly permits
  these formatting bytes while retaining verbatim fragments and no model text.

OntoIndex stale/degraded: workspace preparation failed and indexed commit
identity is unknown; source inspection is authority. Those repeated failures
were recorded in the preceding active repair audit; no unchanged retry,
reindex or graph-validation success is claimed here.

## Alternatives And Selected Minimum

| Question | Alternatives | Decision |
| --- | --- | --- |
| Empty heading identity | Global ordinals; normal slug sentinel plus a second allocator; reserved ID-only component | `@empty` only for an empty allocated anchor; ordinary IDs/anchors unchanged |
| Mtime/canonical bytes | Zero persisted mtime; sidecar cache; omit field | Remove field in schema 2; captured text length for size; current-byte freshness |
| Error-bearing generations | Scope determinism only to no-skip corpora; persist volatile OS text; canonical diagnostics in existing store owner | Stable variant strings and lexical relative/unknown paths; sorted records enter ID |
| Same-ID bytes | Silently retain old bytes; overwrite valid target; reject conflict | Exact index/meta bytes may be reused; verified mismatch refuses before pointer/history changes |
| Tier source wording | Ban existing separators; accept broad generated summaries; narrow formatting exception | One LF between copied fragments only; no provenance capability accepted |

Canonical skips retain multiplicity and coarse reason, but discard raw OS
messages. Adding/reclassifying a skip now changes generation identity even when
admitted documents are unchanged. This is deliberate schema-2 compatibility
behavior. Hash, field, link/read and migration details and worked identity cases
are normative in the service ADR; the task card names their future regressions.

Two read-only GPT-6-Astra explorers challenged independent flows. Runtime
spawn responses report requested/effective model `gpt-6-astra` for both:

- Identity: `agent-results/01a1c638-6a41-7cb2-aa1c-4b2242b5dbe2/2` supports
  reserved-component uniqueness, Unicode filtering and unchanged alias scope.
- Canonical artifacts: `agent-results/01a1c638-9a4e-77d2-97bf-dba4eb6f6600/2`
  identified path-normalization, metadata-byte comparison and explicit version
  versus field rejection requirements. The recorded contract includes them.
- Final identity document review:
  `agent-results/01a1c638-6a41-7cb2-aa1c-4b2242b5dbe2/3`: no blockers.
- Final canonical document review:
  `agent-results/01a1c638-9a4e-77d2-97bf-dba4eb6f6600/3`: clarified that
  schema rejection does not delete old generations, while ordinary retention
  after successful publication remains unchanged. No other blocker reported.

These are source reviews, not independent runtime validation. The first spawn
request was rejected because full-history forks cannot override role/model;
the corrected requests used no history fork and started successfully. An
initial document patch had a context mismatch and applied no changes; it was
corrected against current text. No tool/model failure is retried unchanged.

## Evaluation And Scope

`eval/` filename-only inspection through two directory levels found only
`corpora.json` (receipt `e7adb5`); no payloads were loaded. The
plan still has blocked data preparation and OPEN freeze/run tasks. Any affected
external frozen benchmark must stop OD-3.2 pending fresh preregistration; the
decision never authorizes relabeling or reuse of a spent split. Frozen chunk
budget, graph relations, score functions, default mode and hypotheses stay
unchanged; repaired ID-based ties are not promised identical rankings.

No code, donor files, dependencies, corpus, eval fixtures or benchmark data were
changed. Required validation is document-only: project-plan validator, local
references, whitespace, structure and write-scope hashes. No Rust tests, fmt,
Clippy, benchmarks or listener are run or claimed in this documentation task.
Diagnostics ADR remains Proposed; its historical context is not new authority.

## Document Validation

- `project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md`: PASS,
  35 tasks, no errors; warning that the plan exceeds 1,500 lines. Executed via
  the native project-plan validator after the ADR/task-card changes.
- Python document self-check: exit 0, receipt `78dba3`; local links and target
  heading anchors resolve, fences balance, whitespace and final newlines pass,
  accepted service revision and Proposed diagnostics status agree, conflicting
  old tier/freshness wording is absent.
- `git diff --check`: exit 0 in the same receipt; since files are untracked,
  this is supplemented by explicit document checks and the pre-edit content
  hash inventory. It does not prove scope on its own.
- Both reviewer assignments were read-only; neither ran validation commands
  nor changed files. No compiler job was started. Historical test evidence is
  not a result for this session.

## Closeout

Strict `complete-local` completed OD-3.1 from the recorded main-session evidence
and selected dependency-ready OD-3.2. Plan: 18 DONE, 16 OPEN, 1 BLOCKED (35).
The pre-edit 69-file content inventory confirms only the ADR, architecture,
plan and new decision audit changed before closeout; routing/status updates
followed it. All source/tests/fixtures, manifests, agent rules and the Proposed
diagnostics ADR are unchanged. No commit was created.
