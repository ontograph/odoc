# Architecture Challenge: Shared HTTP OntoDoc

Date: 2026-11-09
Authority: user request to challenge standalone HTTP deployment for many users.
Disposition: design recommendation only; existing ADR and implementation plan
remain authoritative and unchanged. Independent reviewer: GPT-6-Astra
(`/root/http_security_astra`; requested/effective model confirmed at dispatch).

## Verdict

An optional HTTP MCP transport is a sound direction for remote users sharing
one authorized corpus. HTTP supplies connectivity and enables shared index
memory; it does not establish request capacity, fairness or tenant isolation.
OntoDoc is already a standalone Rust package. This proposal changes its network
and trust boundaries, not its retrieval architecture.

Retain stdio. Reuse the existing application and ranking owners. Initially
deploy one service/process per access domain, not per human. Defer a pooled
multi-tenant platform until measured deployment/memory costs justify its
authorization complexity. Do not expose the current process unchanged.

The ADR specifies stdio (`ADR_ONTODOC_SERVICE.md:131-147`) and the implemented
MCP task excludes HTTP/auth (`ONTODOC_IMPLEMENTATION_PLAN.md:510-512`). HTTP
therefore requires an approved ADR amendment and bounded implementation tasks.
This challenge neither changes task order nor completes EVAL-1.3. The wording
against retrieval-path network calls must distinguish an inbound HTTP transport
from outbound retrieval dependencies; the proposal adds no model, embedding or
external lookup to ranking.

## Challenge The Workload First

“Many users” does not specify simultaneous work. Define corpus bytes/chunks/
edges, arrival rate, active versus idle connections, search/read/verify mix,
ranking modes, update rate, hardware and latency/error/memory targets. A small
number of expensive graph/verification calls can cost more than many idle
connections. Mean active work is approximately arrival rate times mean service
time; measure saturation and tails rather than inferring capacity from that
relation or from Rust/Tokio.

The decisive trust distinction is:

- **Shared corpus, identical access:** simplest useful HTTP deployment.
- **Separate private corpora/access domains:** separate process, root and OS
  identity first; authenticated routing cannot accept arbitrary client roots.
- **Mixed permissions inside one corpus:** a new authorization-aware retrieval
  design, not a path-filter feature. Defer rather than retrofit final filtering.

## Source-Grounded Blockers

| Severity | Finding | Consequence for shared HTTP |
| --- | --- | --- |
| High | `search`, `read`, `related` and `status` invoke freshness; `store::freshness` walks the corpus, reading, hashing and retaining all eligible text (`src/cli.rs:496,582,626,649`; `src/store.rs:365-372`; `src/corpus.rs:154-167`). | Concurrent ordinary calls multiply corpus-wide disk/CPU work and temporary corpus memory. HTTP alone cannot fix this. |
| High | A failed citation-span check triggers SHA-256 at every possible same-length window (`src/cite.rs:66-83`), up to 100 citations (`src/cli.rs:633-644`). | Hash work is proportional to `(B-L+1)*L` per citation. With a 2 MiB file and a 1 MiB span, the theoretical full scan hashes about 1 TiB of overlapping input. This is an algebraic worst case, not a measured exploit or latency result. Bound work without silently weakening moved/stale semantics. |
| High | Handlers start `spawn_blocking` work without application admission or cooperative cancellation (`src/mcp.rs:125-136,191-200`). | Disconnects/timeouts need not end started work. Request count, work count and work ownership must remain bounded even during cancellation storms. |
| High | `verify` accepts caller paths without checking published corpus membership (`src/cli.rs:633-644`; `src/cite.rs:58-61,102-122`). | Ignored/non-Markdown files beneath the root can yield existence or hash-match information. A shared readable index does not authorize every file beneath its checkout. |
| High | `scope` is a caller-controlled prefix. BM25 statistics and graph are global; PPR traverses the full graph before in-scope result filtering (`src/search/mod.rs:125-160,185-208,241-259`). | Scope is not authorization. Hidden material may affect scores and ordering; `read`, `related`, `verify` and `status` also need independent authorization. |
| High | Citation reads check symlinks/size, then separately read the file (`src/cite.rs:102-122`). | Concurrent hostile filesystem mutation can invalidate the check. The unconditional no-symlink/bounded-read guarantee needs correction before exposure to that threat model; no race exploit was executed. |
| High | `read` limits raw text, not serialized JSON (`src/cli.rs:578-592`); list bounding stops when no more items can be removed (`src/cli.rs:351-358`). | Escaping/metadata can exceed the claimed byte cap. Bounds must cover complete encoded MCP/HTTP responses, inputs, intermediate work and error results, not just displayed text. |
| Medium | A shared mutex is held during index loading/rebuilding (`src/mcp.rs:107-122`). | New arrivals wait for reload; failure returns errors although the previous slot remains. Decide explicitly whether a last-good generation may remain available. |
| Medium | Each call selects the current index; `read` and `related` take no expected generation (`src/mcp.rs:22-31,107-136`). | Search then read can cross a publication boundary. Per-request snapshot safety is not multi-call snapshot consistency. |

Two ADR conflicts are material: response byte bounds and citation reads do not
fully implement the unconditional promises in `ADR_ONTODOC_SERVICE.md:47-50,145-146`.
Do not silently choose implementation behavior over that authority. Freshness
also differs from the metadata-first wording at ADR lines 148-150, but full
hashing is explicitly accepted in
`audit_session-2026-11-08-implementation-od0-eval12.md:30-36`. Reconcile those
records before choosing a remote-service freshness policy; caching a check
would change its meaning unless its observation age/guarantee is explicit.

## Deployment Options

| Option | Appropriate use | Main trade-off | Recommendation |
| --- | --- | --- | --- |
| Local stdio per workspace | Local/private working trees and offline use | Repeated index memory across processes; no central remote access | Keep supported. |
| Shared HTTP process for one access domain | Team/public documentation where every authorized reader has identical access | Central failure/overload boundary; needs HTTP security and resource controls | Preferred first HTTP deployment, after blockers are addressed. |
| HTTP process per access domain | Different customers or private corpora | More memory/processes and routing operations; OS boundaries remain understandable | Prefer initially over pooled private tenants. |
| Pooled multi-tenant service | Proven scale/operating-cost need for many access domains | Principal-aware corpus selection, graph/statistics isolation, caches, quotas, lifecycle and revocation | Defer; no evidence currently justifies the complexity. |

## Minimum Sufficient HTTP Architecture

```text
Remote MCP clients
  → approved TLS/authentication boundary
  → private rmcp Streamable HTTP listener + bounded admission
  → existing OntoDocServer / App
  → shared immutable Arc<SearchIndex> for one authorized corpus

Separate trusted index publisher → verified generation → atomic pointer swap
Local clients → existing stdio transport → same application/ranking behavior
```

The already installed, locked `rmcp` 1.7 source has native Streamable HTTP
support; OntoDoc enables only `transport-io` today (`Cargo.toml:26`). Reuse that
transport rather than write a second REST retrieval stack. Clone one server
into the transport factory: its `Arc<App>` and shared index slot already support
this (`src/mcp.rs:73-104`). Constructing a new server for every session/request
would duplicate index loading and BM25 state. No global ranking mutex is needed:
`SearchIndex` queries borrow immutable state and allocate private work buffers
(`src/search/mod.rs:114-169,185-208`).

The TLS/auth boundary can use approved deployment infrastructure; do not make a
new donor daemon or checkout a runtime dependency. Backend access must be
restricted so authentication cannot be bypassed. For remote-client compatibility,
verify the chosen protocol revision's authorization/discovery requirements;
a generic login proxy is not automatically a compatible MCP authorization
implementation. Bind any session to its authenticated principal/access domain;
a session ID is not authorization. Reject spoofed identity headers and define
Origin/Host policy, token expiry and logout/reconnect behavior. These are design
requirements, not claims of verified current protocol compliance.

Stateless JSON request/response may simplify operations if required clients and
tool behavior permit it. The installed rmcp stateless path ignores notifications;
do not assume stateful cancellation/notification semantics survive unchanged.
Choose transport/session mode through interoperability tests, not preference.

Bound connections/sessions, request bodies and fields, global/principal active
work, queue length/wait time and expensive tool budgets. Move an admission
permit with blocking work so a cancelled awaiting request cannot free capacity
while computation continues. Include verification, freshness and reloads, not
only search. Limit work even when no response will be sent; use cooperative
cancellation or explicitly bounded completion. Do not add Redis, a broker,
distributed search, ranking caches or an orchestrator without a measured need.

Separate quick snapshot acquisition from single-flight reload coordination.
Preload/verify before readiness, publish only complete generations, and bound
old/new overlap in memory. Define failed reload, rollback, graceful drain and
stale-serving policy. An expected-generation precondition with a typed retry
is smaller than indefinite snapshot retention; bounded pins are optional if
clients actually need them. `verify` checks current bytes, not merely snapshot
bytes, and must retain that distinction.

Authorization/revocation is independent of freshness. The ADR allows stale
results, including stored text for deleted files (`src/cli.rs:569-590`);
that is not permission revocation. Deny revoked access across every tool, old
snapshot and any future cache before further disclosure. For a first shared
deployment, use a dedicated corpus-only root with a least-privilege service
identity, not a broad developer checkout containing unrelated files.

## Evidence Required Before Calling It Multi-User Ready

- Same-generation parity across stdio/HTTP for all five tools, ranked IDs,
  scores and citations; shared factory clones load one index rather than one
  per user. Preserve ranking/defaults and CLI-only indexing.
- Unauthorized requests, direct backend access, cross-user session reuse and
  forbidden paths fail without leaking document/query contents. Mixed-ACL
  deployment, if ever adopted, needs forbidden-document invariance checks for
  graph influence, scores, hierarchy, counts, diagnostics and verification.
- Oversized/malformed requests, escaped JSON, oversized fixed fields, hostile
  citation spans, symlink mutation and cancellation storms remain within limits.
  Invalid requests return typed errors rather than panics or abandoned work.
- Generation changes between tools, in-flight publication, failed reload,
  revocation and shutdown/drain have explicit tested behavior.
- Representative open-loop load tests use synthetic or dedicated operational
  corpora, not the protected quality split. Exercise mixed tools and modes,
  warm/cold/reload cases, bursts, slow clients and sustained saturation. Record
  p50/p95/p99 latency, completed and rejected requests, queue wait, CPU, RSS,
  disk reads and work remaining after cancellation. Set capacity from observed
  targets on declared hardware; do not promise a user/QPS count from inspection.
- Logs/metrics omit raw queries, documents, tokens and internal filesystem
  paths. Existing internal error mapping can expose error strings
  (`src/mcp.rs:83-95`); review the remote error boundary.

Operational scalability is separate from retrieval science. HTTP can preserve
rankings without improving nDCG; speed/availability claims need their own frozen
load scenario and uncertainty/repetitions. Do not change the current held-out
quality protocol or claim its hypotheses passed because transport tests pass.

## Review Evidence And Limits

Source navigation fell back to files because OntoIndex's repository registry
has no OntoDoc entry. Both read-only reviewers made no edits and ran no builds,
tests, load tests, network probes or evaluation. Runtime conclusions above are
source-derived risks, not reproduced exploitation or measured capacity.

- Runtime reviewer receipts `b315ca`, `2dd112`, `5041f8`, `b75a2c`, `ac2725`,
  `dea048`, `5ac2fd`, `e9a59b`: terminal exit **0**, source inspection only.
- GPT-6-Astra receipts `63004e`, `2a029b`, `24dc6c`, `f05f3e`: terminal exit
  **0**, source inspection only. Its additional challenge identified arbitrary
  verification paths, revocation/freshness differences and global graph leakage.
- Native dependency evidence is the local Cargo registry's `rmcp-1.7.0`:
  `rmcp-1.7.0/Cargo.toml:150-158` (HTTP feature),
  `rmcp-1.7.0/src/transport/streamable_http_server/tower.rs:580-599,1106-1115,1174-1231`
  (factories/session modes), `rmcp-1.7.0/src/model.rs:2864-2867` (structured results also
  emitted as text), `rmcp-1.7.0/src/service.rs:955-991` (cancellation context). This is
  installed source evidence, not an executed HTTP handshake.
- Written artifacts: this challenge and its memory-router link only. Source,
  ADR, implementation plan, evaluation data and task state remain unchanged.
- Documentation validation: the first reference check exited **1** because it
  resolved dependency-relative references against OntoDoc. Qualifying those
  references with `rmcp-1.7.0/` corrected the ambiguity. The corrected `python3`
  check exited **0** (`03bd69`): 51 protected source/authority/input files are
  unchanged, 22 source/dependency references resolve, memory links/Markdown
  checks pass and the citation-window arithmetic holds. This validates document
  structure and scope, not the proposed service's security or capacity.
