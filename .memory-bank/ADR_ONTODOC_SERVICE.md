# ADR: OntoDoc — Measurable Documentation Graph-RAG MCP Service

Date: 2026-11-08
Revision: 11 (2026-11-11). Revision 2 added donor ideas
(`audit_session-2026-11-08-donor-ideas-review.md`); revision 3 applies the plan
challenge (`audit_session-2026-11-08-plan-challenge.md`): section-level
evaluation unit, frozen chunk budget, raw-score abstention, specified PPR, power
analysis, a smaller confirmatory family, a code freeze, and H6 dropped.
Amendment (2026-11-08): the user requested GPT-6-Astra as the query reviewer.
Revision 4 accepts folder-scoped HTTP MCP endpoints and independent databases
under `docs_share/`, as requested by the user on 2026-11-10. This is an
architecture amendment; implementation requires scoped tasks. The existing
stdio service and evaluation protocol remain unchanged. Revision 5 records
OD-3.1's schema-2 identity, canonical-artifact and current-byte freshness
decisions, and permits source-fragment LF formatting separators. These repairs
require OD-3.2 implementation; diagnostics capabilities remain Proposed.
Revision 6 resolves EVAL-1.5's frozen-artifact location conflict below.
Revision 7 separates transport-independent safety from network exposure and
accepts the bounded store-read/publication contract below, under the user's
review, consolidate and proceed instruction. It does not authorize HTTP exposure.
Revision 8 accepts SAFETY-0's rooted local I/O, exact citation membership,
numerical work/admission limits and publication outcomes below. Implementation
belongs to OD-3.4/3.6/3.5/3.9; HTTP-0 remains owner-blocked.
Revision 9 accepts EVAL-1.6's trusted preparation/provenance handoff and fixed
default-promotion priority below. EVAL-1.3 implements admission; measurement
remains disabled, human annotation and HTTP exposure gates unchanged.
Revision 10 (2026-11-11) records the owner's approval of the round-4 decision
packet: HTTP-0 exposure and numeric contracts (below), the 4096-byte section-ID
exception, a per-result `text_truncated` search field, and unavailable
statistics for empty exploratory groups. Evaluation stays parked on human
inputs (`audit_session-2026-11-11-owner-decisions.md`).
Revision 11 (2026-11-11) makes pruning move a validated complete generation to
`tmp-<id>` before deleting it (round 6; `audit_session-2026-11-11-review-round6.md`).
Owner: OntoDoc maintainers

## Status

Accepted

## Context

The user requires a new MCP service for documentation retrieval in this
repository, written in Rust, whose quality claims are scientifically provable:
pre-registered, measured on held-out data, with uncertainty reported.

Two predecessors exist:

- OntoIndex (`../ontoindex`, TypeScript). Strong snapshot and freshness
  discipline, but retrieval quality is unmeasured: replay metrics measure
  movement against a baseline, the Markdown graph-RAG audit used 8 queries,
  fusion weights and confidence tiers are hand-set, and no relevance ground
  truth exists.
- Ontograph (`ontocode-rs/`, Rust source snapshot). Sound atomic generation
  publishing, but no Markdown parser, no chunking, no ranking and no retrieval
  evaluation. See `audit_session-2026-11-08-prior-attempt-review.md`.

## Options Considered

| Option | Summary | Disposition |
| --- | --- | --- |
| A. Port the ontograph extension | Extract `ext/ontograph`, stub the host crates | Rejected: about 80k lines, 90 host-API imports, docs ranking absent; porting costs more than rewriting |
| B. Tantivy + ONNX embeddings + graph from day one | Full hybrid stack | Rejected for v1: three unmeasured legs, heavy native dependencies, no evidence that any leg helps |
| C. Small standalone service; lexical baseline first; graph expansion as a pre-registered experiment | BM25 over section chunks; Personalized PageRank over the link graph, kept only if a held-out test shows a gain | **Accepted** |

## Decision

1. **One Cargo package** `ontodoc` at the repository root: library
   (`src/lib.rs`) plus binary `ontodoc` (`src/main.rs`). Rust edition 2024,
   toolchain 1.95.0. `ontocode-rs/` is excluded from the build and stays a
   read-only reference.
2. **Corpus:** `.md` and `.markdown` files found with the `ignore` crate
   (honours `.gitignore` and `.ontodocignore`). Reads stay beneath the root and
   never follow symlinks. Files that are too large or not UTF-8 are skipped with
   a recorded diagnostic. Aggregate resource exhaustion, unsupported rooted I/O
   and unsafe/unreadable discovery controls fail the operation, never publish a
   partial index. The accepted local safety contract below specifies the reader.
3. **Parsing:** `pulldown-cmark` 0.13 with source offsets. Sections start at
   ATX and setext headings; YAML front matter is skipped; anchors use GitHub
   slug rules, including `-1`, `-2` suffixes for duplicates.
4. **Sections and chunks:**
   - The **section** (heading to next heading of equal or higher level) is the
     evaluation unit. Its id is derived from the path and the heading-slug path,
     using the schema-2 empty-component rule below, so it does not depend on
     the chunk budget or on line numbers. Qrels,
     pools and metrics are keyed by section id.
   - The **chunk** is the retrieval unit: one per section, split at block
     boundaries when the section exceeds the chunk budget. The budget is
     **frozen at 1,200 bytes** before any pooling and is not tuned in v1.
     Chunk id = section id + ordinal.
   - Both record the repository-relative path, heading path, byte span, line
     span and the SHA-256 of the span bytes.
   - A ranked chunk list becomes a ranked section list by keeping each
     section's first occurrence.
   Each chunk also stores two structural tiers, derived without any model:
   - **L0**: the heading path plus the first sentence, at most 200 bytes.
   - **L1**: the outline of the enclosing section (child headings plus the
     first sentence of each), at most 1,500 bytes.
   L2 is the chunk text itself.

   Each document gets a **structural pseudo-document**: its title, all its
   headings and the first sentence of every section. L0, L1 and pseudo-documents
   contain verbatim source fragments joined only by one LF formatting separator
   between emitted fragments. Selecting substrings may omit heading syntax;
   byte caps shorten fragments only at UTF-8 boundaries with explicit truncation.
   Separators are formatting, not source quotations. No generated words,
   summaries or claims are permitted. L2 remains the original chunk text.
5. **Graph:** nodes are Document, Section and Chunk. Edges:
   - `CONTAINS`: the hierarchy.
   - `NEXT`: reading order.
   - `LINKS_TO`: resolved relative Markdown links and anchors.
   - `MENTIONS`: inline code spans, recorded but not resolved in v1.
   Unresolved links are diagnostics, not edges.
6. **Retrieval systems:**
   - **B0**: the prior attempt's behaviour reimplemented as the baseline.
     Case-insensitive substring match over title, headings and the first 1 KiB;
     results in path order.
   - **B1**: Okapi BM25 over chunk text plus its heading path, with literature
     defaults `k1 = 1.2` and `b = 0.75`. The tokenizer lowercases with Unicode,
     splits on non-alphanumerics and also emits camelCase and snake_case parts.
     It does no stemming in v1.
   - **B2**: B1, then Personalized PageRank over chunk nodes.
     - Edges: `LINKS_TO` (projected to chunks: a link from a chunk to a
       section or document targets that section's or document's first chunk)
       and `NEXT`. Both are treated as undirected with weight 1. `CONTAINS` is
       excluded; B3 covers hierarchy.
     - Damping 0.85. Seeds are the top 10 B1 chunks, with teleport weight
       proportional to the BM25 score. Mass from dangling nodes returns to the
       seeds. Power iteration stops at L1 change < 1e-9 or 100 iterations.
     - Candidates: B1 top 50 ∪ PPR top 50.
     - Score: `(1 - a) * mm(bm25) + a * mm(ppr)`, where `mm` is min-max over
       the candidates and `mm` = 0 for every candidate when max = min.
     - `a` is tuned on dev over `{0, 0.1, ..., 0.5}`. At `a = 0` the
       zero-BM25 candidates are dropped, so B2 reproduces B1 exactly.
   - **B3**: hierarchical propagation without a model (OpenViking's removed
     parent-score idea, made testable). The final score is
     `(1 - beta) * mm(bm25_chunk) + beta * mm(bm25_doc)`, where
     `bm25_doc` is the BM25 score of the chunk's document pseudo-document, over
     the same candidate pool as B2. `beta` is tuned on dev over
     `{0, 0.1, ..., 0.5}`; at `beta = 0` the zero-BM25 candidates are dropped,
     so B3 reproduces B1 exactly.
   - **Abstention** is decided **before fusion**, identically for every
     mode. Return no results when `s = top_bm25 / sum(idf(query terms))` is
     below a threshold fixed on dev. Every abstention carries a typed reason:
     `no_lexical_match`, `below_threshold` or `scope_empty`.
   - **Scope filter**: an optional path prefix restricts both the candidates
     and the graph seeds. It is a feature, not a quality claim.
   - **Explain**: on request, each result reports its score components (BM25,
     PPR, doc score, rank before fusion). Debugging only; no quality claim.
     Miss attribution uses the same components.
   - No LLM, embedding or outbound network dependency is on the retrieval path;
     an inbound MCP transport does not change retrieval behavior.
7. **Storage:** generation directories under `.ontodoc/generations/<id>/`, where
   `<id>` is the SHA-256 of the canonical corpus manifest, skip diagnostics and **index-time**
   configuration (chunk budget, tier caps, tokenizer and schema version).
   Query-time parameters (`a`, `beta`, threshold) never change the generation.
   - Write sequence: write to a temp dir, fsync, rename, swap the
     `.ontodoc/CURRENT` pointer, all under `.ontodoc/index.lock`. Keep 2 old
     generations.
   - Every path inside an artifact is repository-relative.
   - The artifact hash is verified on load.
   - Schema 2 removes persisted mtime, canonicalizes skip diagnostics, and
     rejects incompatible schemas rather than migrating. The exact format and
     same-ID conflict policy are specified below; implementation is OD-3.2.
   - Hitting a cap fails publication; a partial index is never published.
   - The BM25 index is rebuilt in memory from the chunks at load.
8. **MCP surface:** implemented over stdio, using `rmcp` 1.7. The accepted HTTP
   extension is specified below. Both use the same five read-only tools:
   - `search`: query, top_k, mode `lexical|graph|hier`, optional `scope`,
     `explain` and `budget_bytes`. With `budget_bytes`, results go through
     **breadth-first context assembly**. Every result first gets L0; leftover
     budget then upgrades results in rank order to L1 and then L2. A tier that
     doesn't fit stays at the previous tier and is never truncated. This is a
     feature with no quality claim. Each result carries `text_truncated`: true
     when its L0/L1 text was shortened by the tier byte cap at indexing (the
     stored flag; L2 is never shortened). The response `truncated` flag keeps
     meaning omitted results (revision 10).
   - `read`: chunk or section by id or path#anchor.
   - `related`: graph neighbours of a chunk.
   - `verify`: re-check citations against current bytes.
   - `status`: generation, counts, freshness and diagnostics.

   Every result carries a citation
   `{generation_id, path, heading_path, byte_start, byte_end, line_start,
   line_end, sha256}`. Responses are bounded by item count and serialized bytes,
   with explicit `truncated` flags. Indexing is a CLI action (`ontodoc index`),
   never an MCP tool.
9. **Freshness:** `status` and every response report `fresh` or `stale`. The
   check compares admitted relative paths and SHA-256 hashes of current file
   bytes with the active manifest on every freshness check. Size or mtime
   equality never substitutes for hashing; changing only mtime is not stale.
   Stale results remain visible but are flagged. `read` serves the indexed
   snapshot; `verify` checks current bytes independently. No mtime cache is
   required by this design; bounded streaming I/O is a separate repair.
10. **Citation states:** after exact active-generation membership admission
    below, `verify` compares the cited span with the current file. The result
    is `valid` when the bytes at the span hash match, `moved`
    when the hash occurs exactly once elsewhere in the same file (the new span
    is returned), `stale` when the hash is absent or ambiguous, and `missing`
    when the admitted target is gone or is now a symlink/nonregular file.
    Admission, resource and I/O errors are typed refusals, not citation states.
11. **Determinism:** the same corpus, configuration and binary produce a
    byte-identical generation for the same canonical discovery outcome (defined
    below), and byte-identical evaluation outputs for the same frozen inputs.
    Filesystem mtimes and root location do not enter canonical bytes. Statistical
    resampling uses an in-crate SplitMix64 generator with a fixed seed.

## Schema 2: Identity And Canonical Artifacts (Accepted, Not Implemented)

Status 2026-11-11: implemented by OD-3.1/OD-3.2; the heading keeps its original
anchor for existing links.

Authority: OD-3.1 under the consolidated review-and-proceed instruction.
OD-3.2 implements this decision in the existing owners; current schema-1 code
is not evidence that this contract is implemented. The
[decision audit](audit_session-2026-11-10-identity-canonical-decision.md) records
alternatives, source evidence and review.

### Section Identity And Link Compatibility

- Keep the document-wide heading-anchor allocator, including collision checks
  for duplicate headings and literal suffixes. Allocate anchors before building
  IDs. Preserve `Section.anchor` and heading text.
- Preamble section ID remains `<relative-path>#`. A heading section ID remains
  `<relative-path>#<ancestor-components-joined-by-/>`, replacing **only an empty
  allocated anchor component** with `@empty`. All nonempty components remain
  unchanged. The slug alphabet after Unicode lowercasing is Unicode
  alphanumeric plus `-` and `_`; it excludes `@`, `/`, `#` and `~`. `@empty`
  is reserved for ID components, not an added Markdown anchor. Future slug
  alphabet changes must preserve that separation or revise the schema.
- Chunk ID remains `<section-id>~<zero-based-ordinal>`. Section IDs depend on
  paths, heading hierarchy and allocated anchors, never chunk budgets, offsets
  or line numbers. Uniqueness checks remain at publication and load.

Examples omit the prefix `a.md#`; each row is a fresh document:

| Source order | Allocated heading anchors | Canonical section-ID suffixes |
| --- | --- | --- |
| preamble; `# !!!` | empty | preamble empty; `@empty` |
| `# Parent`; `## !!!`; `### Child` | `parent`, empty, `child` | `parent`, `parent/@empty`, `parent/@empty/child` |
| `# !!!`; `## ???` | empty, `-1` | `@empty`, `@empty/-1` |
| `# !!!`; `# -1`; `# ???` | empty, `-1`, `-2` | `@empty`, `-1`, `-2` |
| `# a`; `# a`; `# a-1`; `# a` | `a`, `a-1`, `a-1-1`, `a-2` | unchanged |
| `# !!!`; `# @empty`; `# empty` | empty, `empty`, `empty-1` | `@empty`, `empty`, `empty-1` |

Markdown fragments keep matching allocated anchors, and an empty fragment
keeps targeting the document's first chunk. Synthetic `#@empty` is not resolved
as a Markdown anchor. `read` keeps exact chunk/section lookup before the
`path#anchor` alias; `read("a.md#")` selects a preamble if present, otherwise
the existing empty-anchor heading alias. Exact IDs for paths containing `#`
or `~` still work; the existing single-fragment alias limitation for filenames
containing `#` is not extended. No old nested-empty-ID aliases are introduced.

### Canonical Records, Identity And Publication

- Set the stored schema version to **2**; keep tokenizer and ranker versions
  unchanged. `FileEntry` contains relative `path`, captured UTF-8 byte `size`
  and `sha256`; remove `mtime_ns` rather than zeroing it or adding an auxiliary
  cache. Transient filesystem metadata may remain outside the artifact.
- In `src/store.rs`, map skip reasons directly to stable strings: `symlink`,
  `not_utf8`, `non_utf8_path`, `changed`, `io`, or
  `too_large:size=<decimal>;cap=<decimal>`. Do not serialize Debug strings, raw
  I/O messages, absolute roots or OS error prose. Preserve skip multiplicity.
  Keep diagnostic paths lexical and relative: omit `.` components; absolute
  paths or any `..` component become the empty unknown path. Do not resolve
  symlinks, guess basenames or reinterpret literal Linux backslashes. Lossy
  non-UTF-8 diagnostic paths are not source identities.
- Sort admitted files by path bytes and canonical skips by `(path bytes,
  reason bytes)` before hashing and serialization. Generation ID is lowercase
  SHA-256 of compact serde JSON for the ordered tuple
  `(IndexConfig, [(path, sha256), ...], [(skip_path, skip_reason), ...])`.
  Hash the exact canonical collections stored, not a second inventory capture.
  A skip change changes the generation ID even when admitted documents do not.
- The same relative files/bytes, config, binary and canonical discovery outcome
  must produce identical serialized `Generation`, `index.json`, `meta.json` and
  `CURRENT` bytes across different roots and mtimes. Discovery outcome means
  the same admitted files and canonical skip records; a transient I/O failure
  may change it. `meta.json` remains ID/hash/length, with no timestamp.
  `HISTORY` is identical only for identical starting state and publication
  sequence; lock/temp names and filesystem timestamps are not artifact bytes.
- Build, publication and load reject unsupported schema/configuration versions;
  publication also checks the recomputed canonical ID before changing the store.
  Schema-1 generations require trusted CLI reindexing. Rejection does not
  migrate, relabel, delete or reuse them as schema 2; ordinary retention pruning
  after successful publication remains unchanged. A retired `mtime_ns` field
  in a purported schema-2 file record rejects explicitly rather than being ignored.
- Under the existing publication lock, reuse an already verified same-ID
  directory only when **both index and metadata bytes** equal the candidate's
  canonical bytes. A verified mismatch is an explicit corruption/conflict
  refusal before changing that directory, `CURRENT` or `HISTORY`; never silently
  keep or overwrite different valid bytes. Exact-byte republication remains
  idempotent. Missing/corrupt-target recovery and atomic publication retain
  their existing protection of the active generation.

### Evidence And Non-Goals

OD-3.2 must exercise independent stores with explicitly different file mtimes,
compare full canonical bytes, and detect same-size edits even with restored
mtime. Add synthetic tests for the ID examples and cross-budget section IDs,
anchor/read behavior, canonical skip records and order, schema rejection,
skip-dependent generation IDs, idempotence and index/meta conflict preservation.

The chunk budget stays frozen at 1,200 bytes. Graph relation definitions,
link projection, scoring, tie-break rules, default mode and hypothesis family
are unchanged; repaired IDs can change ID-based tie ordering and resulting
rankings for formerly broken inputs, so do not claim cross-schema rank parity.
Schema/binary identity changes do not inherit a frozen evaluation identity.
If pools, qrels or a freeze already bind affected IDs, stop and obtain fresh
preregistration and independently reviewed preparation; never silently remap
labels or reuse a spent test split. No such artifacts are created or read here.
No provenance ranges, diagnostics options/tools, HTTP listener, migration
framework, new dependency or measured improvement is accepted by this amendment.

## Transport-Independent Safety And Store Bounds (Accepted)

Containment, citation membership, input/I/O/work bounds and work-owned admission
apply to stdio as well as HTTP. Their implementation does not depend on the
owner's HTTP deployment choice. SAFETY-0 accepts the contracts below;
HTTP-0 retains network exposure,
authentication, session and network-specific admission decisions. Neither this
amendment nor SAFETY-0 permits a listener or selects remote deployment.

OD-3.8 implements one independent store boundary in `src/store.rs`:

- Bound raw `CURRENT` reads to 128 bytes and metadata to 4 KiB; after bounded
  UTF-8 decoding and surrounding-whitespace trimming, require exactly 64
  lowercase hexadecimal ID characters before constructing a generation path.
- Enforce a fixed 512-MiB serialized artifact ceiling independent of metadata
  or persisted configuration. Publication uses the smaller of that ceiling and
  `max_artifact_bytes`, bounding serialization before filesystem mutation.
  Loading and captured-byte decoding enforce the configured cap as well.
  The 512-MiB ceiling deliberately restricts custom configurations previously
  allowed to exceed the default; schema 2 and canonical bytes remain unchanged.
  Evaluation's stricter capture ceilings remain in force.
- Open final components no-follow/nonblocking, inspect the opened regular-file
  descriptor, and cap actual reads, including growth after metadata inspection.
  A metadata check followed by an unrestricted read is insufficient. This
  boundary does not claim ancestor containment or a resident-memory ceiling.
- Cap, unsafe-file and I/O refusals cannot authorize deleting/replacing an
  existing generation or advancing `CURRENT`/`HISTORY`. Preserve supported
  recovery of an absent or verifiably corrupt ordinary artifact explicitly.

OD-3.8 does not implement the post-rename outcomes below. No schema, ranker,
tool, cache or benchmark claim is accepted by these safety decisions.

### Rooted Local I/O (Accepted, Not Implemented)

Status 2026-11-11: implemented by OD-3.4; the heading keeps its original
anchor for existing links.

Select the already-locked **rustix 1.1.4**, promoted to a direct dependency
with `fs` in OD-3.4. One concrete reader in `src/corpus.rs` owns the retained
corpus-directory descriptor; reuse it rather than adding a filesystem trait or
custom syscall layer. The minimum platform is Linux with usable `openat2`
(introduced in 5.6); kernel version alone does not prove availability.

- The configured root's parent prefix is trusted local setup and may contain
  deployment symlinks. Normalize trailing slash/dot spelling, then open the
  final corpus-root component as a real directory without following a symlink.
  Retain that descriptor for the `App` lifetime. Reject invalid roots explicitly;
  do not canonicalize a final symlink into an accepted corpus directory.
- Resolve every descendant's **complete relative path** from that root with
  `openat2` and `BENEATH | NO_SYMLINKS`. File opens use read-only, close-on-exec,
  no-follow and nonblocking flags; inspect the opened regular-file descriptor
  and cap actual bytes with checked `limit + 1` reads. No absolute, empty,
  parent or non-normal citation path components. No component-chain `openat`,
  check-then-read, ordinary-path or `/proc/self/fd` fallback.
- Unsupported, blocked or transiently unsafe resolution fails closed with a
  bounded typed refusal. In particular ENOSYS, blocked syscalls and EAGAIN do
  not permit fallback or unbounded retries. Existing `cap_exceeded` covers work
  exhaustion; `unsupported_filesystem`, `filesystem_changed` and generic `io`
  distinguish other failures without exposing paths/OS prose to clients.
- Enumerate rooted opened directories with locked rustix `RawDir` and a fixed
  **64-KiB buffer**, preserving enumeration errors. Charge probes before
  advancing, including dot entries, terminal probes and errors; omit `.`/`..`
  from policy/output. Refuse records that cannot fit rather than grow/retry.
  Resolve unknown entry types using rooted no-follow descriptor metadata.
  Child directories, files and controls always restart full-relative lookup
  at the root, never at a movable opened descendant.
- The shared bounded traversal/public `ignore` matchers replace automatic
  production control loading. Preserve sorted discovery, custom-before-git
  precedence across depths, nearer rules within a family, hidden whitelists,
  excluded-parent pruning, BOM/CRLF and existing malformed-rule/UTF-8-prefix
  policy. Parent/global controls, `.git/info/exclude` and `.ignore` stay inactive.
  Missing controls mean no rules; symlink/nonregular/unreadable controls refuse.
  Controls inside pruned directories are never opened. Stable readable trees
  must match the existing ordinary walker's policy; retain that walker only as
  a test oracle. Source skips remain canonical diagnostics, not silent drops.
- `App`'s generation reads (CURRENT, metadata, artifact), document verification,
  build and freshness must bind the **same retained corpus root**. Store-owned
  readers keep the existing byte caps. A root rename/replacement must not mix
  one root's generation with another root's documents. Path-based public
  wrappers may open their own operation-scoped root for CLI/eval compatibility;
  no shared retrieval path may silently rebind an existing `App` to a pathname.
  Trusted store publication remains in `src/store.rs`; this is not a publication
  filesystem redesign.

Containment holds at each successful lookup; subsequent reads use that opened
object even after rename. It is not an atomic corpus/policy snapshot or a claim
that an object stays reachable under the root throughout the read. Hard links
and mount crossings remain allowed; no inode-origin authentication or
`NO_XDEV` restriction is selected. Current-byte freshness remains mandatory.

### Exact Citation Membership (Accepted, Not Implemented)

Status 2026-11-11: implemented by OD-3.4; the heading keeps its original
anchor for existing links.

Pin the bound corpus's active generation once per request. Before any requested
document read, validate the **entire** admitted citation batch: every field must
equal a constructible `for_chunk` or whole-section citation from that generation;
the exact path must occur in both its file manifest and document records, and
the nonempty span must belong to the stored document. Share constructors/checks
in `src/cite.rs`; do not retain an unchecked public path-verification route.
No normalization, fuzzy match or arbitrary caller-selected subspan is accepted.

- No active generation: existing `no_index`. A different generation ID, whether
  unknown, old or retained-but-inactive: `generation_unavailable`. Do not consult
  HISTORY or load a caller-named generation. A same-ID nonmember: `invalid_citation`.
  All refuse before target reads. Concurrent publication affects later requests,
  not the pinned request.
- Search L0/L1/L2 citations describe the full chunk; truncated `read` text retains
  the whole chunk/section citation. Whole-section spans may include descendants
  and exceed one chunk budget. Empty documents have no constructible citation.
- Membership means originally admitted in the pinned generation. Ignore rules
  are discovery policy, not ACLs: a newly ignored file can still verify until
  reindex. Freshness separately walks current policy and hashes current bytes.
  Excluded/unindexed paths cannot become a verification oracle; explicitly
  whitelisted hidden files remain valid members.
- For a valid member, absent/symlink/nonregular target preserves `missing`;
  unsupported resolution, race refusal, read errors and work exhaustion are
  typed errors, never fabricated `missing`/`stale`. Readable current bytes retain
  valid/moved/stale semantics, including unique relocation and ambiguity.
- Identical citation tuples in different corpora cannot prove issuance origin.
  Accept only if independently constructible in the receiving corpus, reading
  exclusively its own generation/root. No new citation fields, issuance registry,
  cross-store fallback or retained-generation verification service is selected.

### Local Request And Actual-Work Ceilings (Accepted, Not Implemented)

Status 2026-11-11: implemented by OD-3.5/OD-3.6; the heading keeps its original
anchor for existing links.

These are conservative admission policies, **not measured capacity, wall-time
or resident-memory guarantees**. They are process-local service limits outside
persisted `IndexConfig`: schema/canonical identity and ranking inputs do not
change. Stricter caller/evaluation caps still apply. Custom generations exceeding
these local load limits refuse explicitly; they are not migrated or truncated.
Network/session/global multi-corpus composition remains HTTP-0.

| Boundary | Ceiling | Implementing task |
| --- | --- | --- |
| Raw inbound stdio frame, including LF/CRLF | 128 KiB, enforced before collection/JSON decoding | OD-3.5 |
| Aggregate decoded argument string bytes | 64 KiB | OD-3.5 |
| Query, scope, citation path | 4 KiB each | OD-3.5 |
| Read/related ID | 16 KiB | OD-3.5 |
| Citation vector; heading vector | 100 citations; 64 headings / 16 KiB per citation | OD-3.5 |
| Generation/hash fields | Exactly 64 lowercase hex bytes | OD-3.4/3.5 |
| Source file read | min(configured cap, 2 MiB), plus one detection byte | OD-3.4/3.6 |
| Source/control read work per build/freshness/verification operation | 128 MiB aggregate, including detection/reservation bytes | OD-3.6/3.5 |
| Traversal | 100,000 pre-filter probes; depth 64; 16 MiB accounted path content | OD-3.6 |
| Ignore controls | 64 KiB/file, 4 KiB/line, 4,096 aggregate physical lines, 1,000,000 matching units | OD-3.6 |
| Verification SHA work | 128 MiB aggregate SHA input and 1,000,000 SHA calls | OD-3.5 |
| Load bytes | CURRENT 128 bytes; metadata 4 KiB; artifact min(configured cap, 512 MiB) | OD-3.8/3.6 |
| Load collection preflight | files/documents 100,000 each; sections/chunks 500,000 each; lower configured limits apply | OD-3.6 |
| Search-index construction | 512 MiB checked aggregate actual heading/span/pseudo-document tokenizer input | OD-3.6 |
| Actual blocking retrieval work per stdio process | 1 active operation; 0 application waiting queue | OD-3.5 |

The source-work allowance is separate from the artifact-read/checksum allowance;
a 512-MiB artifact need not fit a 128-MiB source scan. Precharge before reads,
enumeration, matching, hashing, copying or tokenization. Source/control reads
reserve checked cap+1; only successful unused reservations may be refunded,
and failed reads consume theirs. Fixed enumeration buffering does not authorize
unbudgeted copying/filtering of its contents. Detection bytes count as work,
not admitted content. Bound file/skip and manifest storage by charged probes
and paths; freshness streams and drops document text instead of collecting it.
Exhaustion fails the whole operation with `cap_exceeded`, never partial publication,
fabricated freshness or partial verification states. Existing response truncation
remains explicit and complete outbound frames remain bounded to 128 KiB.

Validate strings/vector counts before citation conversion, index loading or
spawning. Refuse >100 input citations rather than truncating requested work.
The stdio guard must cover the actual rmcp `read_until` input path before its
growing line allocation; setting an unused codec limit is insufficient. Oversized
raw frames terminate the connection with a bounded reason before decode; do not
invent an ID. Inner decoded/argument checks do not prove the raw-frame contract.

Allocation-light load preflight counts every encountered array element, including
duplicate JSON keys, before typed collection/lookup construction. No edge cap is
invented: artifact bytes bound serialized edges/diagnostics coarsely, not their
decoded RSS. Before BM25 tokenization, check actual referenced text lengths to
reject overlapping-span amplification; reuse the existing document lookup for
both validation/construction instead of repeated linear scans. Preserve exact
token/rank inputs for accepted generations; no ranking parameter changes.

Verification charges every original-span and relocation candidate SHA before
work, including repeated citations and failed matches. Both byte and call limits
apply across the batch; a one-byte window must not permit millions of tiny
hashes outside accounting. Checked offset arithmetic cannot panic/overflow.

Acquire one fail-fast permit **before CURRENT reads, reload, verification,
freshness or any blocking spawn**. Move it into the actual blocking operation,
including generation load/rebuild and tool execution. All five handlers use that
boundary; no nested permit acquisition or detached reload lane. Busy admission
returns `busy`, not an unbounded queue. Cancellation/shutdown cannot release the
permit or index mutation lock while work survives; cooperative checkpoints may
stop work, but ownership ends only when it actually stops. Retain one shared
immutable active index; a failed reload does not serve the old index as current.
The single-worker stdio choice bounds overlap, not total RSS; HTTP-0 must settle
network, multi-corpus and resident-memory reservations before network delivery.

### Publication Commit And Outcomes (Accepted, Not Implemented)

Status 2026-11-11: implemented by OD-3.9; the heading keeps its original
anchor for existing links.

The commit point is a successful atomic rename of `CURRENT.tmp` onto `CURRENT`
under the store lock. The candidate artifact/directory must already be synced.
Reused candidates require the same file/directory sync precondition; a valid
checksum is not evidence of durability. Before commit, sync namespace entries
for newly created store directories in their containing parents, including the
corpus root when `.ontodoc` is first created. Failure of those syncs is precommit.
Keep `Result<PublishOutcome, StoreError>` in the existing store owner: `Err`
means precommit failure; `Ok` reports the committed generation ID plus durability,
maintenance status and a bounded failure-phase code when applicable.

| Result | Required behavior |
| --- | --- |
| Precommit failure | CURRENT unchanged; previously valid active data preserved. Temp/orphan candidate files may remain. |
| Committed; durability uncertain | Rename succeeded but store-directory sync failed. Report `durability=uncertain`, `maintenance=not_attempted`; skip HISTORY/pruning. |
| Committed; maintenance failed | CURRENT directory sync succeeded; HISTORY or cleanup failed. Report `durability=synced`, `maintenance=failed`; preserve CURRENT and valid current data. |
| Committed; complete | `durability=synced`, `maintenance=complete`; retain current plus two old generations after ordinary successful maintenance. |

OD-3.9 adds publication fields to the existing CLI index report. Every committed
outcome is explicit successful JSON/exit 0 with its warnings; do not turn a
postcommit warning into an ordinary precommit error or promise rollback. The ID
was active at the commit point under the lock, not necessarily forever after
release. Process interruption or stdout failure may hide a successful commit;
missing output is not evidence of noncommit. Inspect CURRENT before recovery;
no blind destructive retry or rollback protocol is added.

Bound HISTORY to **195 bytes** (three distinct canonical IDs with LF), reject
malformed entries, and bound cleanup to **100,000 precharged enumeration probes**.
Missing HISTORY starts empty. Sync the store directory after HISTORY rename and
generation directory changes after pruning; phase failures keep the above
classification. Validate bounded cleanup inventory before deletion; remove only
expected owned generation/temp shapes (`index.json`, `meta.json`, then empty
directory; incomplete temp shapes may contain a subset of these regular files),
never recursively traverse unknown trees. Pruning first renames a validated
complete generation to its `tmp-<id>` name (refusing an occupied name) and
syncs `generations/`, so an interrupted deletion leaves only an incomplete
temp shape for the next temp cleanup (revision 11; no rule relaxed). Unexpected nested/symlink
contents or cap/error refuse cleanup. Explicitly protect CURRENT even if HISTORY
is damaged; a precommit cleanup refusal cannot damage active data. Retain the
lock through sync/maintenance. No janitor, transaction framework or disk format
change is selected. Exact syscall fault injection and barrier/process lock tests
belong to OD-3.9; the earlier parallel `Locked` cause remains unproven.
The trusted CLI administrator must protect the corpus-root/store namespace and
its redirectable ancestors from concurrent untrusted mutation throughout
publication/cleanup. Corpus-reader containment does
not claim race-safe publication against a hostile store administrator.

## Folder-Scoped HTTP MCP Architecture (Accepted, Not Implemented)

Status 2026-11-11: implemented. HTTP-1 is the startup folder registry
(`src/mcp.rs`), and HTTP-2 is the loopback listener (`src/http.rs`) under the
revision-10 contracts below. The heading keeps its anchor.

Authority: user requirement of 2026-11-10: each subfolder of `./docs_share`
must be a separate endpoint named after that folder, with its own database,
behaving as a separate MCP service. This replaces the earlier single-corpus
deployment assumption in the shared-corpus clarification. All service users
retain equal read access to the exposed corpora; folders are corpus boundaries,
not private tenants or user-specific indexes.

### Endpoint And Database Mapping

- Discover immediate real subdirectories of `./docs_share/` in sorted order
  at service startup. Each becomes one independent logical MCP server at
  `/<folder-name>/mcp` on a shared HTTP listener. For example:

  | Corpus root | MCP endpoint | Independent database/index store |
  | --- | --- | --- |
  | `docs_share/rust-book/` | `/rust-book/mcp` | `docs_share/rust-book/.ontodoc/` |
  | `docs_share/product-guide/` | `/product-guide/mcp` | `docs_share/product-guide/.ontodoc/` |

- The folder name is a case-sensitive endpoint key, encoded losslessly as one
  URL path segment. Do not lowercase, slugify or silently alias names. Reject
  ambiguous/colliding routes, encoded path separators and traversal attempts
  with explicit errors; never turn a request path into an arbitrary filesystem
  root. Symlink entries are rejected with diagnostics, never followed.
- Nested directories belong to their immediate parent's corpus; they do not
  create additional endpoints. Files directly inside `docs_share/` are not an
  implicit corpus. An empty folder still has its endpoint and reports an
  unindexed state until a valid generation exists; it never falls back to a
  sibling's index. Missing `docs_share/` is an explicit configuration error.
- "Own database" means a physically separate existing generation store, not
  rows or a path filter inside a merged database. Each folder owns its
  `.ontodoc/CURRENT`, generations and publication lock. Reuse `src/store.rs`;
  this requirement does not introduce SQLite or another database engine.

### Runtime Ownership And Isolation

```text
HTTP listener
  /rust-book/mcp     → OntoDocServer(root=docs_share/rust-book)
                      → App → rust-book/.ontodoc → immutable SearchIndex
  /product-guide/mcp → OntoDocServer(root=docs_share/product-guide)
                      → App → product-guide/.ontodoc → immutable SearchIndex

Trusted CLI publisher → one selected folder's atomic generation store
Local stdio client   → one explicitly configured corpus root, as today
```

- Start with one Rust process and the locked `rmcp` Streamable HTTP transport.
  Construct one `OntoDocServer`/`App` and shared index slot per folder; clone
  that server for its clients, not a new database/index per connection. Reuse
  the same five read-only tools and ranking code, not a second retrieval stack.
- Every tool, including `verify` and `status`, resolves only within its bound
  corpus. Keep chunks, graph edges, BM25 statistics, generations and reloads
  separate. No merged graph, cross-folder search or aggregate `/mcp` endpoint
  is implied. A missing ID/path never triggers a search in another corpus.
- IDs, citations and paths relative to each corpus root are endpoint-scoped.
  Identical generation IDs or section IDs across folders do not make the stores interchangeable;
  registries/caches must key by corpus identity as well as generation. Keep
  each MCP session bound to its endpoint; a session cannot select a different
  corpus through tool arguments or a reused session ID.
- Index publication remains a trusted CLI/control-plane action per folder;
  no MCP write/index tools. A missing, corrupt or reloading index affects its
  endpoint's readiness, not routing to a default corpus. Directory discovery
  occurs at startup; additions/removals/renames require a controlled restart
  and drain of the old sessions. Live discovery/watchers are not required.
- Shared document access does not imply anonymous network access or shared
  queries/sessions. The HTTP admission/authentication decision and existing
  containment, full-wire-size, cancellation and work-bound blockers remain
  prerequisites for exposure. Bound active corpora and total/per-corpus work;
  reject an exceeded cap explicitly rather than silently omitting folders.

### HTTP-0 Exposure And Numeric Contracts (Accepted, Revision 10)

Owner decision 2026-11-11: **loopback backend behind authenticated trusted
ingress.** The process binds only a loopback address. Remote users reach it
through an operator-run TLS ingress that authenticates every request and
cannot be bypassed; ingress configuration is a delivery acceptance item, not
OntoDoc code. Anonymous or direct public listeners remain unauthorized.

- Transport: locked `rmcp` 1.7 `transport-streamable-http-server` in
  **stateless JSON mode** (`stateful_mode=false`, `json_response=true`): one
  POST carries one JSON-RPC message and receives one bounded JSON response.
  No server sessions, SSE streams, replay cache or session store exist, so
  session hijack/reuse, retained-channel and replay bounds are vacuous. GET and
  DELETE return 405. Each request is bound to its URL's corpus by routing; no
  tool argument selects a corpus. Server-initiated notifications are not used.
- Routing: `/<percent-encoded folder>/mcp` exact match against the HTTP-1
  registry; anything else is 404 with a generic body. No default corpus.
- Host/Origin: `Host` must match the configured loopback authority (rmcp
  `allowed_hosts`). Native-client profile: any request carrying `Origin` is
  refused 403 before routing or body reads. A browser allowlist needs a later
  ADR change. Forwarded headers are ignored.
- Numeric limits (conservative policies, not measured capacity):

  | Boundary | Limit |
  | --- | --- |
  | Corpora | 32 (HTTP-1 `MAX_CORPORA`) |
  | Request body | 128 KiB, enforced while streaming, before JSON decode; `Content-Length` above it refuses 413 without reading |
  | Complete response body | 128 KiB, existing `MAX_RESPONSE_BYTES` envelope rule |
  | Concurrent in-flight HTTP requests (connections admitted) | 256 global each; excess requests refused 503 and excess connections closed, fail-fast, no queue |
  | Request read deadline | 30 s for the request head and for the body (408) |
  | Actual blocking tool work | 4 global and 2 per corpus permits, acquired fail-fast before CURRENT reads/reload/spawn and owned by the work until it stops; `busy` on exhaustion |
  | Reloads | inside the same permits; one shared immutable index per corpus |
  | Load/artifact | existing local limits; operators keep artifacts small (no new resident-memory guarantee) |
  | Source file | 2 MiB, unchanged |

- Privacy: no logging of queries, documents, session IDs, credentials or
  request bodies; rmcp `tracing` output is not installed by OntoDoc (no
  subscriber). Error bodies stay generic (rmcp non-2xx bodies are replaced). Stdout stays free of protocol output
  in HTTP mode; diagnostics go to stderr.
- Shutdown: Ctrl-C/SIGTERM stops accepting, then drains in-flight requests;
  blocking work keeps its permit until it actually ends.
- Parity: the five handlers are the same `OntoDocServer` methods used by stdio;
  responses for the same corpus and request are identical apart from transport
  framing.

### Delivery Boundary

HTTP-2 (DONE 2026-11-11) implements the contracts above; `tests/http.rs`
holds the synthetic checks. Still outside this ADR: automatic indexing,
database creation by the listener, authentication in OntoDoc, sessions/SSE
and non-loopback binds. `docs_share/` does not exist in this checkout.

The acceptance checks were to prove with small synthetic corpora
that each folder has its own endpoint/store, all five tools stay within that
folder, identical local IDs do not mix stores, session reuse cannot switch
endpoints, and publication/failure in one corpus does not replace another's
index. Include name encoding/traversal/symlink rejection and same-corpus
stdio/HTTP parity. These are functional/operational checks, not evidence for
retrieval-quality hypotheses; the pre-registered benchmark is unchanged.

## Pre-Registered Hypotheses

### Metrics

- Evaluation unit: section. Gains are 0/1/2 from the qrels; "relevant" means
  grade ≥ 1 for Recall and MRR.
- Primary: nDCG@10 (graded gain, log2 discount). Secondary: Recall@10, MRR@10.
- Ranking metrics are computed with **abstention off**, on **answerable** test
  queries (at least one section of grade ≥ 1), and then averaged per query.
  Off-topic queries are not judged; they are used only for H3.
- Two corpora: bootstrap and randomization resample within each corpus
  (stratified). Results are reported pooled and per corpus.
  An exploratory group with `n = 0` reports `mean`, CI bounds and `p` as
  unavailable (`null`), never fabricated zeros (revision 10).

### Hypotheses

| ID | Role | Claim | Test |
| --- | --- | --- | --- |
| H1 | Sanity gate | B1 beats B0 | Paired bootstrap (10,000) + two-sided paired randomization (10,000) on nDCG@10; alpha 0.05, no family. A failure stops the run for a pipeline audit |
| H2 | Confirmatory | B2 beats B1 | Same tests on nDCG@10, answerable test queries |
| H5 | Confirmatory | B3 beats B1 | Same tests on nDCG@10, answerable test queries |
| H3 | Descriptive | Abstention behaves as tuned | Correct-abstention rate on off-topic and false-abstention rate on answerable test queries, each with a Wilson 95% CI; no pass/fail |
| H4 | Deterministic | Citations are verifiable | Tests: 100% of returned citations are `valid` against their generation; the four citation states behave as defined in §10 |
| E1 | Exploratory | B2 on `multi-hop`, B3 on `conceptual`, Recall@10, MRR@10, per corpus | Reported with CIs, no decision |

**Decision rule** for H2 and H5 (Holm over the family {H2, H5},
alpha = 0.05). An arm **passes** only when all three hold:

- the mean difference is above 0;
- the lower bound of the bootstrap 95% CI is above 0;
- the Holm-adjusted randomization p is below 0.05.

An arm that doesn't pass is **negative** when the upper CI bound is below the
minimum effect, and **inconclusive** otherwise. Only a pass can recommend a
default-mode change, under the fixed promotion policy below; lexical stays
the default until the separately scoped promotion task validates that change.

### Power

- Minimum effect of interest: Δ nDCG@10 = 0.05. Power target 0.8 at
  alpha = 0.025 (the first Holm step).
- Required n = `((z(0.9875) + z(0.8)) * sd_diff / 0.05)^2`, where `sd_diff` is
  the dev standard deviation of the per-query difference for each arm. The
  test split must have `max(n_H2, n_H5, 150)` answerable queries.
- The calculation and its inputs are recorded in the frozen config before
  test-split access. If the test split is too small, more queries are written
  before the run. The run never goes ahead underpowered.

### Protocol rules

- **Burned split.** Once anything has been tuned on the errors of a test run,
  that split is spent. Later claims need a fresh, independently written test
  split, and the audit record says so.
- **Code freeze and attempt custody (revision 6).** Under EVAL-1.5's explicit
  conflict-resolution authority, use one write-once registration at
  `<git-common-dir>/ontodoc-eval/registration.json`, shared by linked worktrees.
  It stores the exact frozen TOML bytes, parameters/power, commit, `Cargo.lock`
  and binary hashes, and captured benchmark/index/partition commitments.
  `freeze --out` exports identical bytes outside the code checkout; the export
  is not registration authority. The held-out run refuses a dirty tree or any
  identity/export mismatch. No tracked-file/dirty-tree exception is introduced.
  Before held-out computation, exclusively create and fsync `attempt-started.json`
  in that same directory. A separate exclusive `attempt-result.json` records
  failure or completion; completion binds exact saved metadata/report bytes,
  whose metadata binds every ranking/abstention file. Scoring parses the same
  bytes it verifies. Interruption leaves a consumed started attempt; changing
  output/benchmark directories, weights or export bytes grants no second one.
  One registration is supported, not a configurable/keyed registry or daemon.
  A subsequent experiment needs fresh approved preregistration/split and a
  separately scoped custody transition; never delete these files to retry.
  These mutable local files do not prove independent custody, annotation or
  query-review provenance. External `audit_session-*` preregistration records
  exact registration/export hashes before test access; started/result hashes
  and every outcome are recorded afterward. These audit files live under the
  canonical external benchmark directory during the freeze; mirror them to
  `.memory-bank/` only after measurement so audit writes do not dirty the frozen
  checkout. EVAL-1.3 retains admission and end-to-end validation, and
  public measurement remains disabled until its complete protocol passes.
- The test split is evaluated once per frozen configuration. Every run is
  recorded in an external `audit_session-*` file, whether it passes or fails,
  and mirrored to the memory bank after measurement.
- Each report records the corpus digests, qrels digest, configuration hash,
  commit, binary hash and seed.
- **Discrimination check** (before the held-out run): on synthetic data, an
  oracle run beats a shuffled run with p < 0.01, and a system compared with
  itself gives p = 1.

## Benchmark Data

- **Corpora.** `../ontoindex` Markdown pinned at commit `d535fe8`, plus one
  public documentation corpus with a permissive licence. The default candidate
  is `rust-lang/book`; it is pinned by URL, commit and licence in `eval/corpora.json`.
- **Queries.**
  - Write them from the user's information need, without copying target
    text. Assign each stratum at writing time and never change it after system
    runs:
    - `lookup`: the answer is in one section.
    - `conceptual`: needs an explanation spread over one document.
    - `multi-hop`: needs at least two sections, at least one of them reachable
      from the other only by a link.
    - `off-topic`: the corpus has no answer.
  - Assign splits by stratified random sampling with a fixed seed: one third
    dev, two thirds test.
  - At least 150 answerable test queries, raised further by the power
    calculation, and at least 30 off-topic test queries.
  - For EVAL-2.0, model-drafted queries may be accepted after a substantive
    independent GPT-6-Astra review in place of human query review. The review
    must precede pooling and access to retrieval rankings and scores. Record
    each final query's author, reviewer, decision, stratum evidence and final
    text digest, with revision history. A model reviewer is not a human.
    Describe the resulting benchmark as model-authored and model-reviewed:
    correlated model errors, training exposure and unknown resemblance to
    real user needs limit generalization. Do not imply equivalence to a
    human-authored benchmark.
- **Judgements.**
  - Graded 0–2, stored as JSONL.
  - **Pools** are the top 10 sections from every system at every grid point
    of the dev sweep (B0, B1, B2 for each `a`, B3 for each `beta`), on both
    splits, so tuned systems surface no unjudged sections.
  - If the pool exceeds 6,000 judgements, the grid falls back to the
    pre-registered `{0.2, 0.4}` before labelling.
  - Gate: judged@10 ≥ 0.95 for every system in every reported run. Condensed-list
    nDCG' (unjudged items removed) is reported as a sensitivity check.
  - A second annotator labels at least 30% of the queries.
    Quadratic-weighted kappa with a bootstrap CI is reported, together with the
    label prevalence.
  - A model-drafted label must be marked as such and reviewed by a human.
    Model output never counts as a human annotator. EVAL-2.1 retains its two
    human annotators, blinding and agreement requirements.
- **Integrity gates** (each must pass before any run):
  - Every judged section id exists in the pinned generation, and its stored
    span hash matches.
  - No test query is a near duplicate of a dev query (normalised token Jaccard
    ≥ 0.8 fails).
  - Pools and the benchmark files regenerate byte for byte.
  - Every miss in a report is assigned one stage: `not_indexed`,
    `chunk_boundary`, `lexical_miss`, `fusion_displaced` or `truncated`.
    Abstention is off for ranking metrics.
- **Rejected data source:** link-derived silver qrels (link text as the query,
  link target as the relevant item). They leak the graph signal into the
  labels and would bias H2.

## Trusted Evaluation Preparation And Promotion (Accepted, Not Implemented)

Status 2026-11-11: implemented by EVAL-1.3 (synthetic evidence; OD-4.1 promotion pending); the heading keeps its original
anchor for existing links.

Authority: EVAL-1.6 under the review/consolidate/proceed instruction. Extend
existing `src/eval/{run,gates,sweep}.rs` and `src/cli.rs`; no new service,
signature framework, registry or relevance-label source. Query and judgement
records retain their existing shapes. The provenance companions and admission
manifest specified here are new contracts, not existing parser capabilities.
`MechanicalEvidence` remains in-process mechanical evidence, not serializable
review approval. EVAL-1.3 must prove complete synthetic public success/refusal
paths before replacing universal measurement refusals with validated admission.

### Trusted Approval And Query Review

Preparation is an explicit trusted-operator operation permitted to inspect
both splits. Ordinary development/dev execution must not invoke it implicitly.
Approval means an operator has examined authentic review/annotation evidence
and pinned its exact hash in the external preparation audit; a matching hash,
`approved=true`, local JSON, model name or timestamp alone proves neither
human identity, independent review nor chronology. Do not invent signatures
or claim independent custody. Missing/unverifiable evidence refuses admission.

`query-review.jsonl`, already named by EVAL-2.0, has one version-1 record per
final query, keyed by `query_id`, with:

- `final_text_sha256`: SHA-256 of the exact UTF-8 `Query.text` bytes, without
  normalization; matching `corpus`, `stratum`, `model_drafted` and stratum
  evidence/references. Review is of the final text, not a discarded draft.
- Author and reviewer objects: stable pseudonymous `id`, `kind` (`human` or
  `model`), model identifier when applicable and an evidence reference. A
  model-drafted query requires independent GPT-6-Astra review. Independence
  means a separate review interaction, not author self-approval or merely a
  second label; record its provenance and limitations.
- `decision` (`accept`, `revise`, `reject`), substantive rationale, and ordered
  revision history containing each text digest, author, reviewer, decision and
  predecessor digest. Only a final accepted record admits that query; missing,
  duplicate, unknown or mismatched IDs/digests refuse. Strata are assigned at
  writing time; later review cannot relabel strata after retrieval exposure.
- Evidence of approval before any pool generation or access to rankings/scores
  for those queries. Operator audit links the accepted query/review hashes to
  the initial pool invocation; file timestamps or post-pool receipts alone are
  insufficient. Post-ranking text changes require fresh independent query
  preparation, not backdating acceptance.

`eval pool` and preparation-stage pool regeneration validate this approval
**before ranking**. The future CLI takes `--query-review PATH` and
`--query-review-sha256 SHA`, whose hash comes from that trusted audit, not from
blindly hashing arbitrary current input. A stage-pools success may emit a
`stage=pools` preparation manifest after the pool is produced; it records
pre-ranking query approval but is never sufficient for execution/labels.
Split authoring itself is not retrieval exposure. Preserve model-authored and
model-reviewed benchmark limitations in every accepted report.

### Human Annotation Companion

Keep EVAL-2.1's `annotation.md` for guidelines, blinding protocol, agreement,
adjudication and operator evidence. Add `annotation.jsonl`: version-1 records
keyed by `(query_id, section_id)` with `span_sha256`, independent raw grades
(0–2), pseudonymous annotator IDs and `kind=human`, model-draft origin where
applicable, explicit human review of each drafted label, and final grade plus
adjudicator/evidence reference. Final grade/hash must exactly match the qrel.
No link-derived labels, model-only human identities, absent raw grades,
duplicate pair records or undocumented adjudication are admitted.

The notes bind a versioned guideline, two distinct verified human identities,
an assignment seed and a predeclared corpus/stratum-stratified overlap selection,
plus evidence that annotators saw query/source sections but no system identity,
scores, ranking positions or other annotator's grade before independent grading.
The second human labels **all pooled pairs for at least 30% of answerable
queries**, not merely 30% of pairs or a post-hoc convenient subset. Off-topic
queries have no relevance labels and are excluded from that denominator.
Derive overlap coverage from raw records and assignments. Report quadratic-
weighted kappa on pre-adjudication overlapping pairs, grade prevalence and a
95% bootstrap CI using 10,000 seeded query-cluster resamples. Undefined kappa,
fewer than two overlap queries or point kappa below 0.6 refuses preparation;
revise guidelines and relabel before any run. Never fabricate a CI from final
adjudicated labels. These checks do not authenticate the human by themselves.

### Exact-Byte Preparation Handoff

Reuse `eval check-benchmark` stages in a trusted preparation context. EVAL-1.3
adds `--query-review`, `--query-review-sha256`, `--annotation PATH`,
`--annotation-notes PATH` and `--preparation-out PATH` as applicable. Stage pools
requires query approval before regeneration; stage qrels additionally requires
human provenance and every existing full-data integrity gate. Gates never
accept a caller's JSON `passed` field in place of recomputation. Capture each
input once within existing 16-MiB-per-payload / 256-MiB-total / 100,000-record
limits; query count remains 2,000. Provenance counts consume the shared record
budget, including history/grade entries; reject before excess allocations.
Bound identifiers to 128 UTF-8 bytes, except section IDs (path plus heading
chain), which are bounded to 4,096 bytes (revision 10); rationale/evidence fields to 4 KiB;
the manifest is capped at 1 MiB. Cap exhaustion is a refusal, not omitted evidence.

Publish one versioned `preparation.json` only after its required checks pass:

| Field group | Required binding |
| --- | --- |
| Status | `version=1`, `stage=pools` or `qrels`, approval operator/evidence reference and query-approval hash; only qrels admits execution |
| Input identity | Exact SHA-256 of config, gates, splits, combined queries, full pool and query-review; qrels stage additionally binds combined qrels and annotation companion/notes |
| Corpora | Sorted corpus name, clean pinned commit, generation ID and captured index/meta hashes; configured source policies/freshness still checked at execution |
| Partitions | Separate dev/test query and qrel hashes; a domain-tagged deterministic commitment to the ordered pair of partition hashes, distinct from selected execution digests; qrels absent at pools stage |
| Counts | Per split/corpus/stratum query counts, off-topic counts, pool pairs and reviewed/model-drafted counts; qrels stage additionally derives positive-qrel answerable counts, human double-labelled query coverage and kappa/CI/prevalence |
| Policy | `promotion_policy=h5-then-h2-v1`, seed, full grid or approved fallback and stage gate evidence hashes |

Counts derive from captured accepted records, never trusted supplied totals.
At pools stage, qrels and human-annotation fields are explicitly absent, not
invented empty approvals; query floors use declared answerable strata, not
positive-label counts. At qrels stage every non-off-topic query needs at least one grade >=1; no
off-topic qrels. Floors (150 answerable/30 off-topic test and existing stratum
floors) plus dev-derived power remain mandatory. Combined authoring files must
equal the union of the two physical partitions by exact parsed records and
assignment, with no unknown/omitted/duplicate query or judgement; hash all raw
files separately, never substitute reserialized digests for raw-file identity.
Pool bytes must regenerate deterministically across the full predeclared grid.

The operator approves the resulting exact manifest hash in the external audit.
Execution uses `--preparation PATH --preparation-sha256 SHA`; the pin is supplied
out-of-band from that audit, avoiding a config↔manifest hash cycle. Manifest
publication or read failure produces no approval. Local approval is a trusted
handoff, not tamper-proof storage. Reject unknown versions/fields, malformed
hashes, path aliases and mismatched stage/identity. Reuse bounded no-follow
payload reads; do not claim hostile-administrator containment.

Each execution process verifies the approved manifest bytes, config/gates/splits,
selected queries/qrels and current corpus/index/pin identity. Dev commands must
not open/stat/hash combined inputs, held-out queries/qrels, full pool, full
query-review or full annotation payloads, nor regenerate their rankings and
filter afterward. They may read committed hash strings and aggregate counts;
these do not expose query text, pairs or labels. Full-data floors, leakage,
pool reproducibility and provenance belong to approved preparation, not an
unchanged call to full `check_benchmark` on a partial dev dataset. If a selected
pool is needed, preparation exports and binds it separately; otherwise do not
load any pool payload at execution. Test capture is similarly selected and
requires explicit `--confirm-test-split` before held-out payload access.

Extend the existing frozen configuration, fixed registration and run metadata
to bind `preparation_sha256`, full benchmark/partition commitments, selected
payload digests and the promotion policy. Do not compare a dev hash to a test
hash as if it were complete benchmark identity. Dev freeze/check use approved
preparation-derived test counts for power without loading test labels. Trusted
preparation must recapture/reapprove a changed dev-selected config before freeze;
ordinary dev never opens held-out data to refresh its own approval. Changing
queries/reviews/labels/strata is not parameter recapture. Freeze retains all
ADR6 clean committed code, lockfile, binary, export and custody requirements.

### Frozen Snapshot, Attempt And Acceptance Boundary

Before registration, the trusted operator exports one canonical external
benchmark snapshot, resolves relative corpus/partition paths consistently and
records its manifest/payload commitments. A relocation or config-byte change
requires preparation recapture **before** registration; then do not rebuild,
rewrite or copy files into the frozen code checkout. Audit and output paths
stay outside it. Preregistration binds approval, preparation, registration,
frozen export, commit, lockfile and binary hashes before the held-out command.
The existing common-dir registration is authoritative, not the export or an
audit string. No reset, replacement registration or second attempt is added.

After explicit confirmation and approval/identity/power preflight, exclusively
create and fsync the started attempt **before test payload/ranking access**.
Capture only the test partition and revalidate its commitments; mismatches,
computation failures and interruption consume the attempt. No preflight may
read test payloads to decide whether to reserve the attempt.

Stage output privately; fsync data and metadata/report bytes and bind them in
the completed result. Public success/accepted saved scoring requires the exact
completed custody record **and** matching published bytes at its canonical
external path. Filesystem publication and custody are not one atomic transaction:
publish-first/completion-failed leaves visible but **unaccepted** files; a
completed record followed by publication failure likewise has no accepted
report. Neither case may emit a successful report or free/retry the attempt.
Failure writing the result leaves the started record consumed. Saved scoring
checks completed custody before exposing held-out results and parses the same
verified buffers; external outcome audit records failures as well as successes.

### Fixed Default-Promotion Policy

Freeze `h5-then-h2-v1` before test access. H1 and every integrity/custody gate
must pass; then evaluate both H2/H5 under the unchanged Holm/CI decision rule:

| H5 | H2 | Recommended default |
| --- | --- | --- |
| pass | any | B3 / hierarchical, with frozen beta |
| not pass | pass | B2 / graph, with frozen alpha |
| not pass | not pass | Keep B1 / lexical; no default edit |

The both-pass choice is a fixed preregistered preference, not the larger
held-out gain, a measured performance claim or an additional hypothesis.
The current shared fusion implementation also computes PPR in hierarchical
mode; this decision promises no algorithm removal or latency advantage.
Report both arms and all descriptive/exploratory
outcomes. Invalid/incomplete/H1-failing runs authorize no promotion.
Abstention remains disabled when threshold is omitted. The dev-selected frozen
threshold remains an explicit option and H3's descriptive setting; neither a
ranking pass nor H3 authorizes turning it on by default. Explicit modes and
weights keep their semantics, including zero weight.

EVAL-3.2 records only the recommendation. Separately scoped OD-4.1 verifies the
completed audited report/frozen identity and implements the deterministic
mode/parameter defaults, or documents a no-op for no passing arm. Preserve
explicit per-request modes/weights and CLI/MCP/serde parity; no run-time loading
of mutable evaluation results on the retrieval path. No application code edit
is authorized by EVAL-1.6 or EVAL-3.2, and no default change during the frozen run.

## Consequences

- v1 has no embeddings. A dense leg can be proposed after H2 resolves, as a new
  pre-registered hypothesis under the same protocol.
- Budgeted context assembly ships as a feature with no claim. The former H6
  was dropped because counting L0 as coverage makes breadth-first win by
  construction.
- Model-written L0/L1 summaries (OpenViking's approach) can be proposed later
  as an index-time arm against the structural tiers, frozen and versioned. They
  never go on the query path.
- Usage-based "hotness", recency decay, LLM intent expansion and LLM rerank are
  rejected: they make offline results irreproducible.
- Knowledge Modules, LineageRAG routing, code-graph indexing and audit tools are
  not ported. They reopen only with a measured need.
- The statistics code (metrics, bootstrap, randomization, Holm, Wilson, kappa)
  lives in `src/eval/` and is tested against hand-computed values.
