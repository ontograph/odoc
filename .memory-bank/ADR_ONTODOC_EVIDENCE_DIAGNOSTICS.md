# ADR: Evidence And Diagnostics For OntoDoc Tools

Date: 2026-11-10
Revision: 2 — contract challenge, 2026-11-10
Owner: OntoDoc maintainers
Requested by: user, "make adr for proposed tools", following the GraphRAG review.

## Status

Proposed

This proposes extensions to existing tools and trusted CLI diagnostics. It does
not approve implementation or supersede the accepted
[service ADR, revision 4](ADR_ONTODOC_SERVICE.md). Acceptance requires the
capability-specific compatibility/resource decisions below and an explicit
service-ADR amendment for the accepted subset. All four capabilities remain
proposed; independent gates permit separately scoped decisions, not implicit
partial acceptance.

EVAL-1.3 remains active; no implementation task is created, completed or reordered.

## Context

The [GraphRAG donor review](audit_session-2026-11-10-graphrag-donor-ideas.md)
identified four useful capabilities: rendered-text provenance, context-selection
accounting, relation-layer topology diagnostics and operational outcome/detail
separation. Source evidence, limitations and the donor pin are recorded there;
no donor code, prompts or data are adopted.

OntoDoc already has the required owners and five MCP tools. L0/L1 may include
ancestor/descendant text outside their supporting chunk, but search always
attaches that chunk's citation (`src/tiers.rs:41-99`; `src/cli.rs:523-537`).
Assembly computes accounting that search discards (`src/assemble.rs:38-65`;
`src/cli.rs:511-520`). Status caps freshness lists without retaining their totals
(`src/cli.rs:659-665`). These observations establish contract questions, not
runtime reproductions, invalid citations or retrieval-quality gains.

One narrow authority conflict must be resolved at acceptance: the service ADR
says every tier byte comes from source; tier construction inserts newline
separators (`src/tiers.rs:178-196`). This proposal distinguishes copied fragments
from explicit formatting. It does not silently override the accepted wording.

## Options Considered

| Option | Disposition |
| --- | --- |
| Keep the current surface and document supporting-chunk semantics | Valid minimum; retains limited diagnostics |
| Extend existing tools selectively; put expensive diagnostics in trusted CLI | **Proposed**; reuse existing owners and immutable generations |
| Add four dedicated MCP tools for provenance, context, topology and build state | Rejected for this proposal; duplicates owners and exposes control-plane work |
| Port GraphRAG's entity/community/report pipeline | Rejected; changes retrieval semantics and introduces unrelated dependencies |

## Proposed Decision

Retain the MCP names `search`, `read`, `related`, `verify` and `status`. No sixth
tool, REST ranker, listener, background diagnostic service or new dependency is
introduced. Extend `src/mcp.rs` → `src/cli.rs::App` → existing tier, assembly,
citation and store owners. Stdio and any later authorized HTTP transport share
the same application contracts.

### Tool And CLI Surface

All additions below are proposed, not available commands or fields today.

| Surface | Proposed addition | Default / boundary |
| --- | --- | --- |
| MCP `search`; CLI `search` | Optional `diagnostics: bool` and `provenance: bool`; CLI flags `--diagnostics`, `--provenance` | Both false; absent response extensions when false; existing `explain` remains score components only |
| MCP `verify`; CLI `verify` | Verify returned source-fragment citations using the existing citation-list shape | No composed-text verdict, new verification state or arbitrary-file capability |
| MCP `status`; CLI `status` | Optional `diagnostics: bool`; exact `added_count`, `removed_count`, `modified_count` when requested | False; totals survive detail truncation |
| Trusted CLI `status` only | `--topology` over its selected immutable generation | No equivalent MCP flag or per-search full-graph scan |
| Trusted CLI `index` only | Sanitized build/publish phase errors if an operational need is demonstrated | No MCP indexing/build-state tool or workflow framework |
| `read`, `related` | No new request fields | Existing behavior retained; supporting-span semantics documented |

Request omission and explicit false must be equivalent. Invalid values for known
fields and limit violations fail explicitly. Preserve the existing unknown-field
policy unless a separately documented compatibility decision changes it; ignored
unknown fields must never enable a mode. Current request types do not declare
`deny_unknown_fields` (`src/cli.rs:159-202`; `src/mcp.rs:34-52`). Test omission,
false, invalid types, legacy unknown fields, tolerant clients and strict schemas.

### Rendered-Text Provenance

The existing `citation` remains the supporting chunk/section citation. It does
not certify all text composed into an L0/L1 tier. The proposed optional
`provenance` object for each search result contains a format version, selected
tier, rendered UTF-8 byte count, rendered SHA-256 and an ordered `parts` list:

- A source part contains a canonical citation in the existing shape. Its exact
  half-open byte span and hash identify the emitted source fragment in the
  selected generation. `heading_path` belongs to the fragment's source section,
  not necessarily the supporting chunk; line bounds/hash describe the clipped
  emitted span. Positions are captured while constructing tiers, never recovered
  later by matching potentially repeated text.
- A separator part has a fixed `newline` kind. It represents the inserted
  formatting byte, not source evidence. Arbitrary generated text is forbidden.
- The server replays parts against its pinned snapshot and reproduces `text`
  exactly, including UTF-8-safe cap cuts. L2 uses its chunk span. Existing L0/L1
  fragments remain in the result's document; no cross-document expansion.

Clients can walk the returned text with a byte cursor: each source part consumes
`byte_end - byte_start` bytes whose hash must match its citation; a newline part
consumes exactly `\n`. Require valid ranges, UTF-8 boundaries and exact text
exhaustion. No duplicate output-offset fields or copied fragment text is needed.
This checks consistency of returned text/metadata, not an independent snapshot
or authenticity proof. Existing `read` selects the active generation and may
return only a prefix (`src/cli.rs:552-591`); `verify` returns states, not source
bytes. No arbitrary-span or retained-snapshot read API is proposed. Independent
snapshot reconstruction requires the client to possess that snapshot separately.

Persisted fragment ranges change the on-disk schema: bump its version, include
the new semantics/limits in generation identity and reject incompatible
generations. Existing section/chunk identities, chunk budget and tier text stay
unchanged. Capture ranges in the tier owner; review persistent size/load cost
with synthetic fixtures before freezing representation. Missing or excessive
evidence is a typed failure; never present partial provenance as exact. No change
to graph edge weights or occurrences is authorized by this ADR.

`verify` checks each source part against current bytes using the existing
valid/moved/stale/missing meanings. The current 100-citation processing/result
cap is not admission control: the MCP input is collected before processing
(`src/cli.rs:633-644`; `src/mcp.rs:195-197`). Propose an admission-bounded
100-citation request ceiling with staged admission: cap raw frame/input bytes
before decoding, cap collection/field bytes while decoding, then cap total work
before blocking execution. Do not promise rejection before all allocation.
Clients explicitly batch larger lists. All parts are required to assess source
coverage; separators are checked by reconstruction. Strict rejection replaces
current over-limit truncation and requires compatibility tests. Even 100
citations need aggregate limits on distinct generations, loaded artifact bytes,
distinct files, read bytes, buffered bytes and relocation/hash work. Current
relocation hashes every candidate window (`src/cite.rs:66-82`); counting results
alone does not bound work. Budget exhaustion is a typed resource error, not
`missing`, `stale`, or a partial verification success.

For the proposed verification contract, validate all generation/citation
membership before current-file reads; any admission or membership failure
rejects the request. This is a compatibility change to `verify`, not a new mode
or a claim that current verification already enforces membership.
Use one bounded request-local read buffer per distinct path for all its citations,
and retain no cross-request cache. This avoids repeated reads; it does not prove
an atomic document/filesystem snapshot. Actual work retains its admission permit
until it stops, including after cancellation of its async waiter.
Snapshot reconstruction and current-byte checks are separate operations. Even
if every source part verifies, independent checks across batches do not establish
an atomic current-file view or that moved fragments form the same current tier.
No aggregate "current rendered text is valid" claim is made.

Before any current-file read, verification must validate membership in the
endpoint's own generation: supporting chunk, supporting section, or exact stored
fragment. Membership is equality of the complete canonical citation record,
including heading path, line bounds and hash; any matching canonical record
suffices when a fragment is reused by several chunks. Do not identify source
ownership by heading text alone. Pin immutable validated generation data for
the actual operation, not merely an ID or directory path. An unavailable
retained generation is an explicit generation error, not missing/stale and not
permission to substitute `CURRENT`. Validate generation-ID syntax and perform
bounded race-safe reads of the endpoint's own store; never join an unchecked ID
into a path. Retained-generation access must remain bounded by the store's
retention policy, not grow a new archive or cache. Pruning may remove disk files,
but cannot invalidate a request that already acquired its complete validated
snapshot. Failure during acquisition is explicit, never a partial pin. Later
requests/batches acquire independently and may return `generation_unavailable`;
batching provides no lease or shared current-byte observation. Bound mixed-
generation requests rather than retaining an unbounded set of generations.

All paths are interpreted solely within the endpoint's bound corpus. Membership
is not authentication of a citation's origin: identical complete citation records
can exist in two corpora. Such identity must never trigger reads from the other
store. Preserve section citations from `read`, not only chunk citations.

### Context Selection And Loss Accounting

Reuse `Assembled`; do not add another selector or copy the donor's stop-on-first-
oversized-row policy. Preserve skip-and-continue breadth-first L0 → L1 → L2.
The optional `diagnostics` object distinguishes:

- Stored-tier cap hits among delivered results. These flags describe tier
  construction, not the request's assembly decision.
- Assembly-input hits, included hits, context-budget omissions and selected
  tiers before response bounding. Input means `ranked.hits`, not every ranking
  candidate. A below-L2 selection is a tier choice, not a measured relevance loss.
- Delivered hits, final response omissions, delivered tier counts and delivered
  text bytes after response bounding.
- Requested text budget, or null when absent. Text bytes exclude JSON escaping,
  citations, metadata, protocol wrapping and errors; they are not wire bytes.

Required identities: input = included + context-budget omissions;
included = delivered + final response omissions. Tier counts sum to their
respective included/delivered totals. Per-request flags describe only the
selected tier, not an unused capped L0/L1 when L2 was delivered.

Keep the existing search-level `truncated` meaning: an entire hit was omitted
by assembly or response bounds. Do not OR it with below-L2 selection or stored
tier-cap flags. Expose those separately. This clarification needs acceptance
because the service ADR currently specifies explicit flags without these details.

### Response Bounds And No Silent Delivery Drift

Finalize one base response using the existing ranking/assembly choices and the
approved complete-wire bounding policy. Capture one generation, one freshness
observation and one request ID/serializer policy; do not rerun an ordinary search
to compare against enrichment. Compute final counters from this finalized base,
then serialize requested additions once under the same policy. No recursive
trimming/reselection to make diagnostic counters fit. Optional metadata must
not silently evict more base results, change their order/tier/text, or omit
required evidence. If the complete response plus requested metadata will not
fit, return a bounded `metadata_limit` error with no partial success payload.
Clients can retry without metadata or with a smaller requested `top_k`.

This preserves content of successful enriched responses, not availability:
an opt-in request can fail where its ordinary form succeeds. Default requests
must retain the baseline delivered result tuple and omit the new fields. Here
the baseline is the same compatible generation and accepted wire policy. Across
a provenance schema rebuild, generation IDs, citation generation IDs and schema
version deliberately change (`src/store.rs:223-229`); require unchanged ordered
chunk/section IDs, scores, tiers and text, not identical old-schema responses.
Within the rebuilt generation, absent and explicit-false requests are identical.
Safety fixes needed to bound currently oversized responses are separate
prerequisites; their compatibility effects must be documented rather than hidden
as diagnostics.

The complete-wire cap includes serialized JSON-RPC/MCP wrapping, structured/text
representations, request IDs and errors. Acceptance must specify bounded IDs
and malformed-input/error framing; limiting tool arguments alone is insufficient.
If mandatory fields alone exceed the cap, fail rather than return oversized JSON.
Apply one shared policy with exact encoder-specific accounting in existing
application/transport owners, not competing inner/outer truncators. CLI limits
include pretty JSON plus the output newline (`src/cli.rs:717-741`;
`src/main.rs:12-19`); MCP limits include its actual serialized message. Bound
serialization allocations with capped/counting output before materializing a
large response; serializing an unbounded value just to measure it is insufficient.
Final counters describe the actually delivered base, not an intermediate list.

### Offline Topology

Trusted CLI `status --topology` uses one pinned generation and reports two
projections: undirected `LINKS_TO`, and undirected `LINKS_TO+NEXT`. Both include
every chunk node, including isolates. Reuse the existing first-chunk projection;
deduplicate reversed/repeated endpoint pairs and ignore self-loops as PPR does.

For each projection report nodes, unique edges, isolates, component count,
largest component size and a bounded largest-component-size sample, sorted
descending. Empty graphs have zero components/largest size; singleton nodes
count as components. Also report chunks incident to `NEXT` but not `LINKS_TO`.
No per-component raw text or paths are needed. Sample truncation is explicit;
totals remain exact. Counts describe topology, never relevance or answer quality.

Bound node/edge/work cardinalities before allocation or traversal. Traversal is
O(nodes + edges) after projection; canonical endpoint lookup, deduplication,
string bytes and bounded sample selection also count toward the total work
budget. Do not claim linear overall work from traversal alone. Exceeding a
computational cap fails explicitly, rather than returning estimates as exact
totals. Compute offline on demand; do not persist a second graph/cache or rebuild
the search index. No pruning, degree boost, new relation, ranking change or
cross-corpus aggregation.

Combined status/topology output uses that same generation for its ID, counts,
projections and freshness comparison, with a single freshness observation.
No second load of `CURRENT` midway through the command. Files can change while
freshness is checked; the observation is not an atomic live-corpus snapshot.
Projection work must finish within its budget before exact totals are returned;
incomplete computation is a typed error, not an exact-looking partial report.

### Operational Detail And Publication

Compute exact freshness totals before trimming detail lists; preserve totals
when final serialization removes samples. Reuse existing freshness checks without
weakening current-byte semantics or doing an extra corpus walk for metadata.

Optional build/publish diagnostics stay within existing CLI/store owners. Fixed
phase names and typed sanitized error categories are sufficient. Pre-commit
failure preserves the previous `CURRENT`. Pointer replacement is the commit
boundary; a later error does not imply rollback. Current publication swaps the
pointer before directory sync/cleanup (`src/store.rs:468-470`). Define post-swap
outcomes before implementation: uncertain durability requires reconciliation;
established commit with cleanup failure is committed with a warning. An error
alone cannot establish the active generation or justify published-success.
Logs on stderr must contain no raw queries, document text, credentials or
sensitive internal errors. Timings/machine data never enter canonical generations.
No workflow registry, callback system or durable event sidecar is proposed.

## Resource And Compatibility Gates Before Acceptance

Each row is an independent decision boundary, not a new task or authorization.
The accepted service ADR and shared safety rules still apply to every row. Do
not make small diagnostics depend on unrelated provenance/publication redesign.

| Capability | Additional gates for its scoped acceptance |
| --- | --- |
| Status freshness totals | Optional schema/default compatibility, one existing freshness capture, exact encoder bounds; no fragment schema or retained-generation loader needed |
| Search accounting | Once-finalized base, exact pre/post-bound counters, opt-in metadata-limit behavior and complete response bounds; no fragment persistence needed |
| Rendered provenance and fragment verification | Canonical fragment schema/format, measured synthetic storage/load cost, snapshot pin/membership, staged admission and aggregate verification budgets |
| Trusted topology | Defined projection and exact totals, single-generation status coherence, bounded preprocessing/traversal/output; no publication outcome API needed |
| Optional CLI phase outcomes | Demonstrated operational need and explicit pre/post-commit outcome semantics; no provenance schema or MCP build-state surface |

The detailed gates below apply only to capabilities using the named facility;
they do not create dependencies between otherwise independent rows.

- Freeze numerical ceilings for provenance parts per tier/request/generation,
  serialized provenance bytes, offline diagnostic output, topology nodes/edges,
  and total computation/allocation. Reuse the existing 50-hit, 100-citation and
  128 KiB response constants where applicable, but define how the complete-wire
  limit includes framing (`src/cli.rs:24-26`). This ADR is not implementation-ready
  while new ceilings and their bounded-error behavior remain unspecified.
- Generation membership, unique identities, bounded race-safe filesystem access
  and complete response/error bounds are prerequisites, not proven capabilities
  of current `verify` or `bound`. Admission for blocking work must cover generation
  loads, freshness and verification until the actual work ends.
- Review and freeze only the relevant optional schemas, strict-client behavior,
  schema invalidation and service-ADR amendments. Keep index provenance identity
  separate from wire `format_version`: copied-range/renderer semantic changes
  invalidate incompatible generations; wire field layout and transport-only
  limits do not enter generation identity. Defaults avoid per-result provenance
  construction/serialization, but shared reload/deserialization of persisted
  ranges still has a cost. Measure it; do not claim zero default-request overhead.
  No flag may change implicit rankings, evidence selection or packing.
- Define publication outcome/error semantics across the pointer-swap boundary;
  the existing undifferentiated result is insufficient for precise phase reports.
  Do not claim that every failure rolls back or publish a diagnostic success
  before the corresponding commit/durability state is established.
- Retain corpus-local stores and sessions, equal reader access, stdio parity,
  trusted CLI publication and the accepted HTTP exposure gates. No new transport,
  user tenancy, arbitrary-path read or index publication capability is implied.

## Acceptance Evidence For Future Implementation

| Contract | Required synthetic evidence |
| --- | --- |
| Provenance | Server snapshot reconstruction plus client cursor/hash consistency; ancestor/descendant fragments outside read prefixes; repeated sentences/newlines/UTF-8 cuts; changed source outside chunk does not masquerade as whole-tier verification |
| Verification | Exact chunk/section/fragment records; altered heading/line/span/hash rejection; raw-frame, field, count, mixed-generation and work limits; generation pin survives pruning but later batches may fail; symlink swaps, oversized files and excessive relocation scanning give explicit errors |
| Accounting | All hits below L2; omitted L0; capped selected/unused tiers; later small hit after an oversized one; zero hits; pre/post-bound count identities |
| Wire compatibility | Same-generation absent/false identity, legacy unknown fields, same-capture enriched success; declared cross-schema identity changes; metadata-limit error with no partial payload; escaped text, metadata, IDs/errors and actual MCP/pretty CLI framing at caps |
| Topology | Hand-count edgeless/singleton, NEXT-only, disconnected components, reversed/duplicate links/self-loops; publish CURRENT during combined status command without mixing generations; unchanged search scores/artifacts |
| Operations | Freshness totals above sample caps; pre-commit failure preserves old CURRENT; post-swap sync/cleanup failures report uncertainty or established commit accurately; successful outcomes match the published generation; sanitized diagnostics |
| Determinism/schema | Stable canonical bytes across source-mtime changes; deliberate schema rejection and full rebuild; unchanged section IDs, chunk budget, ranker output and fixed assembly |

Implementation needs scoped task cards with exact write sets, runnable commands,
cap-boundary tests and evidence; this ADR supplies no task completion. No real
held-out queries/labels/results are needed for these checks. Any change to B2/B3,
H4 interpretation, evaluation outputs or frozen artifacts requires separate
adjudication under the service ADR. Preserve code/configuration freeze checks
and existing held-out refusals even when rankings are unchanged; a schema or
binary change does not inherit an earlier frozen identity. No benchmark claim
follows from these tests.

## Consequences And Non-Goals

Clients can request inspectable evidence/accounting without a larger MCP tool
catalog. Defaults stay small; opt-in metadata can explicitly fail at its cap.
Provenance increases index size and needs a schema rebuild; its cost is unmeasured.
Offline diagnostics avoid adding graph scans to shared-user request latency.

No LLM, embeddings, semantic community reports, DRIFT/global synthesis, new graph
database, incremental mutation, altered chunk budget or generic pipeline framework
is adopted. The donor review establishes useful design ideas, not superiority,
throughput, lower cost or statistically proven retrieval improvement.

## Revision 2 Challenge Disposition

Independent GPT-6-Astra challenge and a source-only contract trace identified
the following gaps in revision 1. These are document corrections, not implemented
fixes or closure of the remaining acceptance gates.

| Finding | Disposition |
| --- | --- |
| Span hashes did not specify client evidence inspection or canonical membership | Define server reconstruction, client cursor/hash checks and full-record equality; no new snapshot read API |
| Count limits and generation IDs did not bound actual verification or retention races | Staged byte/collection/work admission; immutable request pins, explicit acquisition errors and no cross-batch lease |
| No-drift baseline was ambiguous across reloads, wrappers and schema rebuilds | Finalize one base under one capture; exact encoder bounds; allow only declared cross-schema identity changes |
| Proposed unknown-field rejection and zero-cost defaults were unjustified | Preserve legacy policy; distinguish default serialization work from common artifact-load cost |
| Global gates coupled small diagnostics to schema/publication changes | Capability-specific acceptance boundaries; all capabilities remain proposed |
| Combined status/topology could mix generations or imply atomic freshness | One pinned generation/freshness observation; fail incomplete topology instead of reporting partial exact totals |
