# OntoDoc Consolidated Challenge And Repair Plan

Date: 2026-11-10
Authority: user request to challenge delivered/planned work, compare solutions,
consolidate one plan and proceed. Accepted service ADR revision 4 remains design
authority. Evidence/diagnostics ADR revision 2 remains Proposed.

## Review Method And Limits

Three independent read-only GPT-6-Astra subagents reviewed evaluation, service
boundaries, and plan/process. Runtime spawn results reported requested and
effective model `gpt-6-astra` for all three. Result references:

- Evaluation: `agent-results/01a1c610-3f8e-7873-9e31-d1b7b7cabc69/1`.
- Service: `agent-results/01a1c610-6d9e-7611-93dc-a0668668d98b/1`.
- Plan: `agent-results/01a1c610-a04f-7c33-acad-5b042baeb155/1`.
- Independent first-repair review:
  `agent-results/01a1c610-6d9e-7611-93dc-a0668668d98b/2`: no blocking findings;
  source review only, no independent test execution.

OntoIndex workspace search/safe-edit failed `workspace_build_failed`. Committed
search/context/impact failed `indexed commit identity is unknown` (root tool
results and review reports). **OntoIndex stale/degraded: workspace preparation
failed and indexed commit identity is unknown; source inspection is authority.**
No refresh was initiated and no successful graph blast-radius claim is made.

Source inspection, synthetic regression results and historical closeouts are
separate evidence. No real held-out queries, labels or results were opened;
no benchmark, production listener or donor build was run. No measured retrieval
quality or latency/throughput improvement is claimed. The checkout is untracked;
an empty git diff cannot establish write scope. A pre-edit protected-file hash
inventory is used instead.

## What Went Well

- One Rust package and one CLI/MCP application path, five read-only tools, and
  an immutable search index avoid parallel retrieval stacks (`Cargo.toml`,
  `src/mcp.rs`, `src/cli.rs::App`).
- Atomic publication uses a lock, fsync and pointer replacement. Citation
  states, deterministic tie-breaking and hand-computed statistics have focused
  tests. These strengths do not prove the remaining boundary contracts.
- The ADR separates functional features from scientific claims, retains lexical
  default, and requires power, frozen identity, all outcomes and human labels.
- Accepted HTTP architecture reuses per-folder stores and startup discovery,
  avoiding merged databases, tenant ACLs and watchers.

## Findings And Disposition

| Priority | Finding and current source evidence | Selected action |
| --- | --- | --- |
| High | Preamble and empty-slug heading can share section/chunk IDs (`src/parse.rs:81-120`, `src/chunk.rs:38`). Search maps keep the last duplicate while read finds the first (`src/search/mod.rs:143-148`, `src/cli.rs:555`). | OD-3.0 rejects publication/load now; OD-3.1/3.2 decide and repair identity/canonical format. |
| High | Application JSON size is measured before `CallToolResult::structured` and JSON-RPC wrapping (`src/cli.rs:351`, `src/mcp.rs:92`); rmcp 1.7 duplicates structured content into text. Read caps raw text, not escaped JSON (`src/cli.rs:578-592`), and internal errors expose paths (`src/mcp.rs:83-89`). | OD-3.3 tests and bounds complete frames/errors in the existing boundary. |
| High | Verify accepts caller paths without manifest/generation membership (`src/cli.rs:633-644`); metadata checks then ordinary reads do not prevent ancestor replacement or growth (`src/cite.rs:102-122`, `src/corpus.rs:208-237`). | HTTP-0 decides native read primitive/membership; OD-3.4 shares one rooted bounded reader. |
| High | Moved-span search hashes every candidate substring (`src/cite.rs:67-77`): approximately `(N-L+1)*L` hashed bytes. Nominal 2-MiB files and 1-MiB spans imply roughly 1 TiB per miss, a derived work count, not a runtime measurement. `spawn_blocking` has no work-owned admission (`src/mcp.rs:117-197`). | OD-3.5 combines early input limits, aggregate work budget and closure-owned permits; not timeout alone. |
| Medium | Freshness repeatedly walks/hashes and retains document texts (`src/store.rs:365`, `src/corpus.rs:154`); CURRENT/meta/artifact reads allocate before bounds (`src/store.rs::current_id/verify_dir`). | OD-3.6 streams freshness and bounds reads without caching away current-byte semantics. |
| Medium | `FileEntry.mtime_ns` changes serialized bytes while generation ID excludes it (`src/store.rs::FileEntry/build/generation_id`). Existing touch test checks only ID. | OD-3.1/3.2 make an explicit schema decision and compare full bytes with distinct mtimes. |
| High | `ensure_measurement_ready` always refuses (`src/eval/run.rs:471-478`); this is containment, not EVAL-1.3's valid-input success path. | Preserve guards; implement EVAL-1.4 and EVAL-1.5 before EVAL-1.3 integration. |
| High | Combined queries/qrels are eagerly read before dev filtering; preflight and scoring reopen mutable inputs (`src/eval/run.rs:196-247,622-624,1026-1067`, `src/eval/gates.rs:93-105`, `src/eval/sweep.rs:139-155`). | Physically split inputs and capture bounded immutable snapshots once; validate and compute from identical bytes. |
| High | Accepted folder HTTP has no task cards; plan authority says rev3, README describes only the donor snapshot. Evaluation commands contain unsupported shapes; frozen path policy conflicts with ADR. | One updated rev4 tracker; HTTP-0/1/2, OD-3.7 docs, corrected command syntax and explicit freeze-location gate. |

Source line numbers above describe the pre-repair review; symbol names locate
subsequent code. All findings except OD-3.0 remain open, regardless of historical
DONE cards. No historical card is reopened or relabelled as current validation.

## Alternatives And Picks

| Problem | Alternatives challenged | Selected robust minimum |
| --- | --- | --- |
| Publication ambiguity | Silently deduplicate; redesign IDs immediately; reject ambiguous generations | Reject before writes and on load, then separately approve format changes. No schema bump for the validator. |
| Evaluation availability | Delete guards; build an independent registry service; immutable split snapshots plus durable exclusive attempt | Snapshots plus existing audit/create-new/fsync mechanisms. A registry daemon is not required by ADR. A mutable local receipt alone is not independent custody. |
| Validation/measurement I/O | Reread mutable files repeatedly; lock external writers; capture once | Parse/digest the same bounded owned bytes. Locks alone cannot control unchecked external writers. |
| Request CPU/cancellation | Timeout; semaphore alone; explicit work budget plus actual-work-owned permit | Combined budget and permit; deterministic counter/barrier checks rather than timing assertions. |
| Freshness performance | TTL cache/watchers; repeated text-bearing walk; streaming current-byte inventory | Streaming current hashes with aggregate caps; defer caches/watchers and measure before claiming performance. |
| Delivery order | All service work behind human annotation; second plan; one tracker with independent dependency branches | One tracker, one active task, independent service/evaluation/HTTP prerequisites. |
| Donor features | Entity/community/LLM pipeline; four new tools; reuse existing owners only | Defer proposed diagnostics and semantic graph expansion; preserve five tools and current hypothesis family. |

The main overengineering is duplicated mutable-input validation and treating a
new registry as mandatory; the main underengineering is security/work bounds.
Statistical integrity is necessary, not a simplification target. Repeated
approval loops and audits without executable repair cards were process failures;
the present explicit review-and-proceed grant is acted on without a second ask.

## Consolidated Sequence

1. OD-3.0: uniqueness refusal at publish/load, synthetic store-preservation checks.
2. Prefer OD-3.3 full-wire/privacy repair next; HTTP-0 resolves exposure, native
   reader, membership and numerical budgets. No listener startup is authorized.
3. OD-3.1/3.2 identity/canonical schema decision and implementation; OD-3.4/3.5/3.6
   containment, real-work admission and bounded freshness/load I/O. OD-3.7
   documents usable stdio independently of scientific outcomes.
4. EVAL-1.4 isolated immutable inputs, EVAL-1.5 frozen attempt/run identity,
   EVAL-1.3 integration with positive and negative synthetic evidence. Resolve
   in-checkout frozen-path versus external-artifact refusal before execution.
5. HTTP-1/2 implement folder isolation then locked rmcp HTTP after prerequisites.
   Equal corpus access does not authorize anonymous exposure.
6. EVAL-2.x queries/review and two-human labels; EVAL-3.x dev proof, tuning,
   powered freeze and once-only held-out run; OD-4.0 scientific closeout.

## First Repair Evidence

Only `src/store.rs` is authorized for OD-3.0 code changes. One borrowed-ID
HashSet validates section and chunk namespaces independently, before publication
serialization/filesystem work and after load hash/schema/identity checks.
No new dependency, ID format, schema, ranking, tool or evaluation change.

- `CARGO_BUILD_JOBS=4 cargo test --lib store::`: exit 0, **12 passed**,
  receipts `e8eae0` / terminal `309805` (session 21403). Three new regressions
  cover independent duplicate kinds, real preamble/empty-slug collision and
  self-consistent malformed persisted artifacts. Existing roundtrip passes.
- `cargo fmt --check`: first exit 1 (`743d80`), two test-layout differences;
  corrected only those lines. Second exit 0 (`161f97`).
- `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`: terminal
  exit 0, managed job `ontodoc-od30-clippy-20261110-01`, startup `47ff21`,
  validated terminal receipt `275702` (no truncated tail). Launcher/reader:
  `PYTHONDONTWRITEBYTECODE=1 python3 /opt/d1/_wrk/ontocode/scripts/compiler_lane_detached.py`
  with launch arguments `ontodoc-od30-clippy-20261110-01 -- --fail-fast cargo clippy --all-targets -- -D warnings`
  and harvest arguments `--result ontodoc-od30-clippy-20261110-01 --wait 60`.
  Terminal authority is
  `/opt/d2/.ontocode/artifacts/compiler-lane/ontodoc-od30-clippy-20261110-01/result.json`.
  The existing generic launcher inherits OntoDoc cwd but its admission lock and
  process scan cover the sibling checkout. Independent OntoDoc process/artifact
  admission checks found no active writer (`332c59`); no lock override, build
  command import, donor build, new runner or dependency was added.
- Full integration suite, HTTP runtime, benchmarks and held-out evaluation were
  not run. Focused library tests plus all-target lint do not establish those
  contracts or multi-user readiness.
- Initial revised-plan validation rejected integration-tier whole-target Clippy
  command wiring. Removed it from focused task validation, not from the
  repository closeout requirement; corrected plan validation **PASS, 35 tasks**.

Ordinary Markdown with an empty heading slug after a preamble now fails safely.
It remains unindexable until the separately gated identity repair. Existing
malformed stored generations are rejected, not migrated or silently repaired.

Strict `manager_loop_advance(action=complete-local)` completed OD-3.0 and selected
OD-3.3 next. Final counts: 16 DONE, 18 OPEN, 1 BLOCKED, 35 total. The original
15 DONE closeouts are preserved. Only `src/store.rs` and six planning/memory
documents changed; protected-source hashes show no edits to evaluation, ranking,
transport, manifests or AGENTS.md. No commit was created.
