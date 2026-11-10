# OD-3.3: MCP complete-frame bounds and privacy

Date: 2026-11-10
Status: Implementation and required validation complete; strict closeout below.

## Boundary Decision

Keep locked rmcp 1.7.0's native stdio codec. Wrap its server transport to
measure serialized JSON-RPC messages including the newline against 128 KiB.
An oversized reply becomes a fixed `cap_exceeded` error with the original ID.
Admit a request only if that complete refusal, including its unchanged escaped
ID, fits. Otherwise close before dispatch, without a partial response or a
substituted, shortened or omitted ID. This is an intentional service
compatibility limit, not a claim that MCP forbids large IDs. OD-3.3 authorizes
this documented boundary decision; no tool/schema/transport-stack change.

The native codec's fixed, ID-less parse error is already below the cap and
must retain recovery behavior. This does not bound native inbound line
allocation, blocking-work lifetime or freshness I/O; those remain separate
tasks. Native tracing in an embedding host is not an audited logging surface.

## Navigation

OntoIndex workspace preparation failed repeatedly; indexed commit identity is
unknown. No further unchanged retries or reindex. Current source and locked
SDK inspection are authoritative for this task. The protocol boundary is high
risk. A GPT-6-Astra read-only review confirmed native Transport extension points
and the need for pre-dispatch oversized-ID admission.

## Implementation And Review

- `src/mcp.rs`: native transport wrapper with capped serialization counter;
  measured text/structured parity with existing truncatable responses; generic
  join, initialization and session errors. No codec, listener or tool added.
- `src/cli.rs`: shared public error messages, escaping-aware read bounds and
  typed refusal when indivisible metadata cannot fit; evaluation guards intact.
- `tests/mcp.rs`, `tests/cli.rs`: actual binary/native-codec wire regressions,
  exact-limit escaped IDs, errors, five tools, parse recovery and privacy.
- GPT-6-Astra source review found the transport conversion and test-only future
  lifetime errors; both were corrected. Two wire coverage gaps and its suggested
  cancellation test were added. Final source review reports no remaining
  blocker: `agent-results/01a1c623-83c5-7cc1-876d-5ce0766aa4ab/5`.
  The reviewer changed no files and ran no validation; runtime results are the
  parent receipts, not independent reviewer test evidence.

## Validation Receipts

Managed jobs use `/opt/d1/_wrk/ontocode/scripts/compiler_lane_detached.py`
from the OntoDoc package root, not the donor snapshot. Each terminal outcome was
harvested with `--result <job> --wait 60`; launcher success is not validation.
Receipts live under `$ONTOCODE_HOME/artifacts/compiler-lane/<job>/`.

| Job | Command (all Cargo uses `CARGO_BUILD_JOBS=4`) | Terminal result |
| --- | --- | --- |
| `ontodoc-od33-focused-20261110-v1` | `cargo test --test mcp --test cli` | 101: compile E0277; native stdio tuple is IntoTransport, not Transport |
| `ontodoc-od33-focused-20261110-v2` | `cargo test --test mcp --test cli` | 0: CLI 3, MCP 7 passed after explicit native AsyncRwTransport conversion |
| `ontodoc-od33-regressions-20261110-v3` | `cargo test --test mcp --test cli --lib` | 75: shared compiler-lane artifact lock held by another session; compilation did not start |
| `ontodoc-od33-regressions-20261110-v4` | `cargo test --test mcp --test cli --lib` | 101: test fake returned a borrowed async future; E0477 required a static send future |
| `ontodoc-od33-regressions-20261110-v5` | `cargo test --test mcp --test cli --lib` | 0: 85 library, 4 CLI, 7 MCP tests passed after returning an owned ready future |
| `ontodoc-od33-clippy-20261110-v1` | `cargo clippy --all-targets -- -D warnings` | 0; no warnings |

`CARGO_BUILD_JOBS=4 cargo fmt --check`: exit 0 (`e4735e`). Terminal managed
receipt readers: `ee3dae`, `359e57`, `ee3789`, `730b57`, `2a7ba9`, `e59e8e`
in job-table order. The v5 output tail was truncated; retained summary includes
all three terminal test counts. Earlier v1 failure was truncated to its E0277
diagnostic; no missing log content is treated as validation evidence.

Preflight found no compiler or target-lock holder in OntoDoc. A concurrent
sibling session caused v3 admission refusal; no override or overlapping OntoDoc
command was used. After the sibling process exited, v4 obtained admission.
All jobs were harvested. No donor build or held-out evaluation ran. Other
integration suites and benchmarks were not run; they are not OD-3.3 gates.

## Scope And Limits

Content-hash comparison against the pre-edit 68-file inventory is used because
this checkout has no initial commit and its files are untracked. Production
and regression changes are confined to the four allowed files; memory-bank
evidence/routing changes are separate. Cargo manifests/lock, agent rules, ADRs,
store/identity/ranking/evaluation sources and fixtures are unchanged. No commit
was made. OntoIndex diff verification remains unavailable as described above;
it is not represented as passed.

Tests establish bounded actual native-codec output, explicit truncation or
typed refusal, exact-ID preservation, independent connection closure,
cancellation-sticky rejection, privacy and ordinary five-tool compatibility.
They do not establish HTTP readiness, inbound allocation bounds, throughput,
retrieval quality or completion of other resource-safety tasks.

## Closeout

Strict `complete-local` completed OD-3.3 from the recorded main-session evidence
and selected dependency-ready OD-3.1. Post-closeout tracking revision:
`acecdacda2c4a4f2474bb465d7adf01e58dcbc0ec5e64be2cf082ebd08d5322a`.
Plan counts: 17 DONE, 17 OPEN, 1 BLOCKED (35 total). OD-3.1 is a documentation
decision task; no identity or canonical-artifact code change is included here.
