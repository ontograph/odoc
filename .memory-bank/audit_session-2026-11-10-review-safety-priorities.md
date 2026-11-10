# Fresh GPT Challenge And Safety Priorities

Date: 2026-11-10
Authority: user requested independent GPT challenge, alternatives, one
consolidated plan and execution. Accepted service ADR revision 7 separates
transport-independent safety from exposure and accepts OD-3.8's store bounds.

## Review Evidence And Limits

Three requested/effective GPT-6-Astra read-only explorers reviewed evaluation,
runtime safety and the plan independently. Results:
`agent-results/01a1c6cc-3ab0-7af2-bf8d-53b31d171e0b/1`,
`agent-results/01a1c6cc-6564-73e3-b07c-73291cd6cb32/1` and
`agent-results/01a1c6cc-9645-7df3-b026-2997f538c910/1`.
No child executed Rust validation. No real benchmark queries, qrels or results
were inspected. Historical test receipts do not establish current-tree success.

OntoIndex navigation failed `workspace_build_failed`; explicit committed
context/impact failed because indexed commit identity is unknown. Source
inspection is authority. No reindex or graph PASS. This checkout has no HEAD
and project files are untracked; an initial SHA-256 inventory of 76 owned
source/test/config/document files supports session scope checking, not provenance
or a clean measurement freeze. Empty Git diff is insufficient evidence here.

## Good, Worst And Forgotten

| Assessment | Source finding | Disposition |
| --- | --- | --- |
| Keep | One Rust package, shared App/search path, immutable indexes, five tools, no network/LLM retrieval dependency | No new ranker/service/framework. |
| Keep | Deterministic schema-2 identity, canonical-byte tests, complete outgoing MCP frame/privacy bounds, captured eval bytes and consumed attempts | Preserve DONE history; measure quality separately. |
| Worst ordering | Stdio containment/work/freshness repairs depended on HTTP-0's unrelated deployment choice | SAFETY-0 decides local contracts; HTTP-0 keeps the owner exposure gate. |
| High safety gap | `App::verify` accepts paths without generation membership; `cite::read_beneath` checks then reads; moved-span scanning hashes every candidate | OD-3.4 rooted reader/membership, then OD-3.5 precharged work and closure-owned permits. Exhaustion is never fabricated missing/stale. |
| High I/O gap | `Store::current_id` and `verify_dir` read unrestricted bytes and join unvalidated pointer contents | OD-3.8 bounded reads/serialization in the existing owner. |
| Forgotten commit boundary | `Store::publish_inner` changes CURRENT before fallible sync/history/prune; an error can mean already published | SAFETY-0 defines outcomes; OD-3.9 deterministic regressions, no blind rollback. |
| Evaluation incomplete | CLI combined-input paths and dev-only capture cannot yield correct held-out execution; preparation lacks provenance/count handoff | EVAL-1.6 decision then EVAL-1.3 positive and negative public paths; guards retained. |
| Forgotten eval contract | Run publishes before completed custody; per-corpus metrics lack required exploratory intervals | Explicit EVAL-1.3 acceptance. |
| Avoidable work | `Generation::chunk_text` scans documents per chunk; integrity gates rebuild token sets and section lookups | Reuse existing lookup in OD-3.6 and local precomputation in EVAL-1.3; no global cache or measured speed claim. |
| Plan handoff gaps | Data cards omit physical partitions; final closeout omits HTTP/external report binding; both-pass promotion unspecified | Unfinished card scopes/dependencies corrected; EVAL-1.6 preregisters promotion. |
| Unresolved validation | Earlier parallel pruning test returned Locked; serial passing did not diagnose it | OD-3.9 requires process/lock evidence; no flaky-test label or blind retry. |

The overengineering is repeated custody/review scaffolding before a useful safe
success path, not statistical integrity. Reuse the existing custody owner;
stale wording requesting another registry does not justify another service.
Additional tools, provenance-range frameworks, semantic graph edges, watchers
and TTL freshness stay deferred. Folder HTTP remains requested scope, delivered
after its safety and exposure prerequisites rather than silently abandoned.

## Alternatives And Selected Sequence

| Problem | Alternatives | Selected |
| --- | --- | --- |
| Safety delivery | Wait for HTTP; repair everything together; small store boundary first | Independently testable store repair, then local decisions and safety tasks. |
| Containment | Check/canonicalize then read; new filesystem framework; supported rooted native primitive | SAFETY-0 selects minimal fail-closed native reader. OD-3.8 claims only leaf/read-byte bounds. |
| Freshness/load work | TTL/watchers; repeated text-bearing walk; streaming hashes/existing lookups | Streaming + precharged caps, unchanged current-byte semantics. |
| Evaluation | Lift guards; registry daemon; trusted preparation plus existing capture/custody | Exact-byte handoff and synthetic public success/refusal proof; two humans still required. |
| Publication | Blind retry/rollback; pretend every error is precommit; explicit committed outcomes | Separate commit/durability/maintenance states with deterministic failures. |

One canonical plan: **OD-3.8 → SAFETY-0 → OD-3.4 → OD-3.6 → OD-3.5**;
OD-3.9 follows its publication decision. **EVAL-1.6 → EVAL-1.3** precedes query
preparation, two-human annotation and dev/frozen held-out work. HTTP-0 requires
explicit owner loopback or authenticated-ingress choice; HTTP-1/2 retain safety
dependencies. OD-4.0 joins HTTP and scientific completion. Four bounded tasks
added, 39 total; no previous DONE task reopened. Plan-length warning remains
advisory; bulk archival is not a prerequisite for repair.

## Execution And Validation

OD-3.8 implementation/validation in progress; not yet closed by this document.
Initial managed job `ontodoc-review-store-20261110a` returned terminal **75**,
artifact lock, receipt `384040`; no compiler success. Owner PID finished before
corrected launch. Job `ontodoc-review-store-20261110b` ran formatting and all
required tests successfully (store23, CLI5, MCP7, gates14, run6), then Clippy
failed on `unused_unit`, terminal **101**, receipt `dc7837` (tail truncated).
Removed only that redundant expression; attempted final check
`ontodoc-review-store-20261110c` was denied by a different shared compiler owner,
terminal **75**, receipt `bf764e`. Final validation is pending; no lock override
or donor build. Scope inventory receipt `e3737f` shows only `src/store.rs` plus
the seven authorized memory/ADR/plan documents changed; no removed files.
The second compiler owner finished (bounded wait receipt `ad5864`); final
managed job `ontodoc-review-store-20261110d` then completed **exit 0**, canonical
reader receipt `a0830e`. Exact commands:

- `cargo fmt && cargo fmt --check`: exit 0.
- `CARGO_BUILD_JOBS=4 cargo test --lib store::`: exit 0, **23 passed**.
- `CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp --test eval_gates --test eval_run`:
  exit 0, **5 + 7 + 14 + 6 passed**.
- `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`: exit 0.

Terminal authority:
`/opt/d2/.ontocode/artifacts/compiler-lane/ontodoc-review-store-20261110d/result.json`.
Reader tail is truncated at the prefix; terminal result, all test totals and
Clippy completion remain visible, and complete diagnostics are in `output.log`.
Six new same-module regressions cover the new store boundary. Exact 512-MiB
serialization is not exercised: tests cover small configured exact/cap+1 writer
boundaries, sparse oversized files and global limit checks. No full-library
suite, benchmark, containment, memory-budget or HTTP validation was run.

Astra source/delta results `agent-results/01a1c6cc-6564-73e3-b07c-73291cd6cb32/2`
and `/3` report no source blockers; plan delta `/root/challenge_plan` result `/3`
finds no dependency/ADR conflicts. Those reviews executed no Rust checks.
Final rustfmt-only source hash:
`4055ad1393a355fd5a2211207b535b4d06ac405e25dc0eb5c62159c2878584d6`.
Document links/anchors/fences/whitespace/router cap and `git diff --check`
passed, receipt `fa7b05`; plan validator PASS39, existing length warning only.

Strict `complete-local` closed **OD-3.8** and selected **SAFETY-0**. Final state:
**23 DONE, 14 OPEN, 2 BLOCKED / 39**. All 22 inherited DONE cards were retained.
Only `src/store.rs` and seven authorized memory/ADR/plan documents changed;
no dependency, donor, ranker, other test file or real evaluation payload changed.
No commit, retrieval-quality or measured performance claim. The old parallel
Locked defect remains unproven/unrepaired despite this focused suite passing.
Final OntoIndex `verify_diff` also failed unknown indexed commit identity after
correcting its unsupported committed-read argument; no graph scope PASS.
