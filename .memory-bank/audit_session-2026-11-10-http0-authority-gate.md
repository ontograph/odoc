# HTTP-0: Exposure Choice Required

Date: 2026-11-10
Authority: [HTTP-0 stop condition](ONTODOC_IMPLEMENTATION_PLAN.md) and accepted
[folder HTTP architecture](ADR_ONTODOC_SERVICE.md#folder-scoped-http-mcp-architecture-accepted-not-implemented).
Status: Blocked; recommendations below are not accepted contracts.

## Owner Decision

The owner requested shared, folder-scoped MCP access but has not selected the
network admission/deployment boundary. Equal document access does not authorize
anonymous access. Generic instruction to proceed does not clear HTTP-0's
explicit stop condition. Do not move the unresolved decision into HTTP-2.

| Choice | Consequences |
| --- | --- |
| Local loopback-only HTTP | Explicit trusted-local-user model; no remote service. Loopback is not authentication against other local processes. |
| Authenticated trusted-ingress remote delivery | Recommended for the requested shared service: loopback backend, authenticated TLS ingress with no bypass, principal-bound sessions. Owner must accept the boundary; actual ingress/authentication configuration remains a delivery acceptance gate. |
| Anonymous or direct public listener | Not authorized; do not infer this from shared documents. |

Reopen when the owner explicitly chooses the first or second model and accepts
its admission/session boundary. No listener, dependency or runtime changed.
The ADR remains revision 5; no proposed controls below are silently accepted.
Independent work remains open. The manager selects dependency-ready OD-3.7
operational documentation next; HTTP-0 is not a plan-wide deadlock.

## Source-Backed Recommendations To Finalize After The Choice

- Reuse locked rmcp Streamable HTTP and existing five handlers. Stateful mode
  preserves notifications/cancellation; the installed stateless path ignores
  notifications. Define exact Host/authority and Origin handling, including
  forwarded-header spoofing, for all POST/GET/DELETE paths. A native-client-only
  profile could reject all present Origins; browser access needs an explicit
  allowlist. Loopback and library defaults alone do not prove rebinding safety.
- Bind sessions to corpus and admitted principal, checking revocation on every
  method. Native session IDs alone supply neither identity nor authorization.
  Reserve quotas before session creation and clean up malformed initialization.
  rmcp's native body collection needs a streaming limit before collection.
- Consider initial limits of 32 corpora, 128-KiB request/complete JSON-RPC frame,
  256 global/32 per-corpus sessions, four global/two per-corpus actual jobs,
  one reload included inside the global work limit and fail-fast admission.
  These are proposals, not measured capacity. Also bound encoded SSE, connections,
  control paths, channels, completed-event replay, slow streams and shutdown.
  The default 60-second request-channel retention has no count ceiling; merely
  bounding sessions is insufficient. Do not add an unbounded queue/replay cache.
- Preserve the actual current **2-MiB/file** limit. Existing 512-MiB artifacts
  across 32 corpora allow 16 GiB of serialized artifacts before decoded indexes,
  postings and reload overlap. Select smaller server ceilings and separate
  decoded/construction/scratch/retired-generation reservations; serialized-byte
  caps are not resident-memory caps. Numeric budgets remain undecided, not an
  implementation-ready performance promise.
- Charge entry/file/hash-read and verification candidate-window/hash-byte work
  before operations. A timeout does not stop blocking work; permits remain in
  that work. Exhaustion is a typed refusal, never fabricated freshness or a
  `missing`/`stale` citation result. Keep current-byte semantics, not TTL caching.
- Native reader candidate: already-locked rustix 1.1.4 with `fs`, full relative
  `openat2` beneath a trusted retained corpus descriptor, `BENEATH|NO_SYMLINKS`,
  nonblocking no-follow open, regular-file descriptor check and bounded cap+1
  reads. Fail closed if required resolution is unsupported/blocked. A bare
  component-openat chain leaves an ancestor-rename gap; a leaf-reader repair
  alone does not secure path-based ignore traversal. No new dependency approved.
- Citation candidate: before target reads, pin the bound active generation and
  require an exact constructible section/chunk citation. Unknown/old generation
  is a typed refusal, not `missing`; this compatibility restriction requires ADR
  acceptance. Identical cross-corpus citations cannot prove origin under current
  fields but may only resolve against the endpoint's own generation/files.
  No issuance registry, new fields or retained-snapshot service is selected.
- Disable dependency payload/session-ID logging: rmcp emits request/results at
  debug and some notifications/session IDs at info. Preserve generic internal
  errors; ingress logging must not store queries, documents or credentials.

## Evidence And Limits

Both requested/effective reviewers were GPT-6-Astra, read-only:

- Native/API review: `agent-results/01a1c64e-0d65-7f30-9ed1-2c3bbe7d012a/1`;
  source/API receipts `91e062`, `99fec7`, `df9237`, `e039ab`, `54c74c`, exit 0.
  Covers corpus/citation/store callers, locked rustix and current limits.
- Authority/transport review:
  `agent-results/01a1c64e-89a5-75c3-a3cd-7b9873d3dc72/1`; source receipts
  `a41273`, `163a66`, `93e13e`, `744bb4`, exit 0. Covers ADR/task stop,
  locked rmcp session/body/logging behavior and memory arithmetic.

Reviewers changed no files and ran no validation commands. Parent records the
authority stop, then validates plan structure, links, whitespace and hash scope.
OntoIndex's unchanged recorded preparation/commit failure uses source fallback;
no retry/reindex. No Rust build, race/kernel probe, HTTP interoperability, load
test, performance/retrieval claim or real evaluation occurred in HTTP-0.

The first plan check rejected a plan-wide blocked state while independent
OD-3.7 and EVAL-1.4 remained ready. Corrected only routing/status to select
OD-3.7 and retain HTTP-0's explicit blocker; no implementation gate was bypassed.
