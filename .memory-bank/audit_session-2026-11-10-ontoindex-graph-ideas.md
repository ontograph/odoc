# OntoIndex Graph Ideas Review

Date: 2026-11-10
Scope: review `../ontoindex` for ideas for making OntoDoc's documentation graph.
The user's `../ontoidex` is interpreted as that existing sibling repository.
Disposition: recommendations only; no code, ADR, schema, task order or evaluation
configuration changed. Independent challenge: GPT-6-Astra, effective model
confirmed by the agent runtime.

## Decision Summary

**Keep the existing Rust graph and improve its correctness and bounds before
adding relationships.** Borrow evidence-carrying source facts and explicit
coverage reporting. Do not port the donor's graph infrastructure or ranker.
Nothing in this review establishes a documentation-retrieval improvement.

The recommendation order below is not an implementation task order. EVAL-1.3
remains active; new work requires separately authorized, scoped tasks.

## Evidence Boundary

- Donor checkout: `../ontoindex`, HEAD
  `4a34725516f56a9b43790e0b7b8d320c6cced994`; status was clean. In this report,
  `D/` means `../ontoindex/ontoindex/`, relative to the OntoDoc repository root.
- Donor licence is AGPL-3.0. Ideas only: no donor code, prompts or data copied.
- Committed-index navigation reported the matching donor commit. Subsequent
  context queries failed with a full buffer pool. **OntoIndex stale/degraded:
  context lookup exhausted the buffer pool; source inspection is authority.**
  No retry or reindex was used to bypass that failure.
- Three read-only specialists reviewed extraction, retrieval and lifecycle.
  GPT-6-Astra independently challenged the shortlist against current source.
  Tests cited below were inspected, not run. No held-out OntoDoc data was read.
- Authority remains [ADR revision 4](ADR_ONTODOC_SERVICE.md). Each immediate
  `docs_share/<name>/` has its own proposed `/<name>/mcp` endpoint and store.
  All users have equal read access; graphs, statistics and caches never merge
  across corpora. That HTTP architecture is accepted, not implemented.

## What OntoDoc Already Has

`src/graph.rs:69-115,126-198` builds structural edges, parses actual Markdown
links, resolves relative paths and anchors, and records broken-link diagnostics.
`src/search/ppr.rs:3-16,23-39` already constructs deterministic adjacency for the
fixed undirected, weight-one `LINKS_TO` + `NEXT` graph. Atomic generation
publication and request-held index snapshots already exist
(`src/store.rs:425-469`; `src/mcp.rs:125-135`). These are not new donor ideas.

Preserve section identity independently of chunk budgets and keep hashes as
freshness evidence. Donor section IDs include source line numbers, and sidecar
chunk keys incorporate the whole-file hash
(`D/src/core/ingestion/markdown-processor.ts:107`;
`D/src/core/ingestion/enrichment/markdown-document-facts.ts:242-257`). Those
identities are unsuitable replacements for OntoDoc's citation/evaluation unit.

## Prioritized Ideas And Their Limits

### 1. Validate graph identities and invariants

Reject duplicate document/section/chunk identities and malformed relationships
before publication and before constructing search indexes from a loaded artifact.
Check known edge kinds, endpoint types, ownership, chunk-to-section membership,
permitted `CONTAINS` relationships and same-document `NEXT`. The unresolved
`MENTIONS` target is a specific exception, not permission for arbitrary dangling
edges. Artifact hashes alone do not establish graph validity
(`src/store.rs:136-140,484-524`; `src/search/mod.rs:143-160`).

Concrete source-derived risk: a preamble has ID `<path>#`, and a first
punctuation-only heading can also get an empty slug and that ID. Both first
chunks then receive `<path>#~0`. A synthetic example is
`intro\n\n# !!!\n\nbody` (`src/parse.rs:94-115,212-230`;
`src/chunk.rs:36-39`). This was not executed as a reproduction. A validator
rejects the collision; it does not repair the identity contract. Any identity
repair requires ADR/schema compatibility review.

The donor uses ID maps but silently keeps the first conflicting node
(`D/src/core/graph/graph.ts:8-17`). Borrow lookup efficiency, not that policy.
Structural validation is our adaptation recommendation, not a verified donor
feature. Existing owners: `src/parse.rs`, `src/chunk.rs`, `src/store.rs`.

### 2. Bound construction and neighbour retrieval before allocation grows

Use counted insertion, bounded evidence/diagnostic accumulation and indexed
lookups in existing owners. Donor maps expose counts and allocation-free
iterators (`D/src/core/graph/graph.ts:94-110`), but impose no cardinality caps.
OntoDoc scans chunks for every parser event and collects all `related` neighbours
before capping (`src/graph.rs:83-87,146-150`; `src/cli.rs:604-623`). Upstream
parsing/chunking and serialized artifacts also need pre-allocation bounds;
their current limits are checked after collection/serialization
(`src/store.rs:241-255,430-436`).

First consider a capped iterator for `related`; it bounds returned accumulation,
not the full edge scan. Add a typed incident-edge index only if measurements
justify its memory cost. PPR adjacency cannot replace it: adjacency discards
relation kinds, direction and multiplicity. Keep B2 unchanged; replacing full
PPR with a capped neighbourhood is a new ranking intervention. Index-cap
exhaustion must fail publication, never silently publish a partial graph.

### 3. Preserve successful-link evidence for a named consumer

The donor's link facts retain destination text, label and source lines
(`D/src/core/ingestion/enrichment/markdown-document-facts.ts:75-83`). Its
producer only splits the destination into path/fragment; that is source evidence,
not proof of canonical target resolution
(`D/src/core/ingestion/enrichment/markdown-sidecar-producer.ts:903-923`).

OntoDoc keeps source spans for failed links but successful edges retain only
kind/from/to (`src/graph.rs:39-65,91-103`; `src/store.rs:136-148`). For an
approved graph-audit or explanation use case, retain bounded occurrences with
source-snapshot span, original destination and the actually resolved target.
Keep these facts separate from deduplicated ranking edges: repeated references
must not silently become extra PPR weight.

Diagnostic-only evidence is the smaller option when offline auditing is the
only consumer. Persistent facts or new response fields require schema/API
review. Recording a canonical document/section target must not silently change
the ADR's first-chunk projection. Existing owners: `src/graph.rs`, `src/store.rs`;
`src/search/mod.rs` and `src/cli.rs` only for approved explanation consumers.
Seed/candidate-origin traces can explain inclusion, not all PPR mass flows.

### 4. Report coverage separately from identity, freshness and quality

Reuse existing generation metadata, skipped files and link diagnostics. Add
bounded counts only for a defined operational need: supported occurrences,
resolved references, unsupported/external links, broken references, self-links
and resolved targets without a chunk projection. These are different states;
edge count is not link occurrence count or a coverage percentage
(`src/graph.rs:93-103,185-198`).

The donor separates source/configuration identity from coverage and records
analyzer/scope/ignore contracts
(`D/src/core/indexing/source-manifest.ts:31-43`). Apply that distinction without
adding a second manifest subsystem. Parser/graph semantic changes must invalidate
incompatible artifacts via the approved schema/configuration contract. Preserve
current-byte freshness checks; more metadata must not weaken them. No coverage
or edge-confidence number is a probability of answer correctness.

### 5. Preserve whole-corpus generations; defer incremental mutation

The donor removes incident edges in one pass and reports surviving affected
sources (`D/src/core/graph/graph.ts:48-76`). This is useful only when indexing
measurements justify incremental complexity. Changed-file-only resolution misses
previously broken links repaired by a newly added target. If authorized later,
compare against full rebuilds and initially re-resolve links corpus-wide.
Keep complete immutable publication in `src/store.rs`; do not import database
pools or disk-reader pins for the current in-memory request snapshot.

### 6. Treat semantic edges as a later experiment

For poorly linked corpora, exact requirement/API identifiers with evidence and
explicit ambiguous/unresolved states are a more defensible research candidate
than merging generic headings or shared owners. They are still outside v1's
unresolved-`MENTIONS` boundary and need ADR/schema authorization before use.
Donor concept clusters retain reasons/source keys but transitively union shared
signals, allowing generic labels to connect unrelated material
(`D/src/core/ingestion/enrichment/markdown-concept-clusters.ts:42-46,123-143`).

Direction, relation weights, concepts, communities and alternative fusion methods
need separately approved dev experiments and preregistration. Do not mutate
fixed B2/B3 or the H2/H5 confirmatory family. No LLM, embeddings or new graph
database is required to implement the correctness ideas above.

## Donor Behaviours Not To Adopt

- **Regex links and guessed mentions.** The basic Markdown processor strips
  anchors, creates file-level links and fans a matching code name out to every
  candidate (`D/src/core/ingestion/markdown-processor.ts:145-190,196-215`). Keep
  OntoDoc's Markdown parser, actual destination resolution and unresolved mentions.
- **Calling optional PPR metadata the document ranker.** The reviewed Markdown
  path scores/caps chunks first and then optionally attaches PPR metadata
  (`D/src/mcp/local/local-backend.ts:714-733,771-788`). Its PPR adapter reads
  `fact.chunkKey` before testing link facts, whose producer supplies
  `fromChunkKey`; normal produced links therefore appear to miss that branch.
  Even that branch creates a link-token node, not a resolved destination-chunk
  edge (`D/src/mcp/local/local-backend.ts:910-934`). Source finding only, not a
  runtime reproduction or a claim about every donor retrieval flow.
- **Assuming a visited-node cap bounds total work.** The PPR helper maps all
  input nodes, builds all outgoing edges and stores skips before slicing the
  preview (`D/src/core/ingestion/enrichment/markdown-document-ppr.ts:117-118,145,218,249-265`).
- **Filename-based authority as universal relevance.** Active Markdown scoring
  discounts plan/audit paths and adds a one-hop bonus to already-scored target
  chunks (`D/src/core/ingestion/enrichment/markdown-query-ranking.ts:60-69,120,147-169`).
  Plans and audit history can be the exact answer; relevance needs measurement.

## Options

| Option | Recommendation | Evidence needed |
| --- | --- | --- |
| Retain current graph | Immediate outcome of this review; no implementation authorization | Existing correctness findings remain open |
| Strengthen existing graph | Preferred future option for validated identity, bounds or provenance needs | Focused synthetic regressions and operational measurements |
| Add inferred entity/concept graph | Defer; more edges can amplify false relationships | Approved semantics, independently reviewed relevance labels, separate experiment |

For every option, construct/query one graph per corpus endpoint. A future
evidence layer would feed the existing fixed chunk projection and ranker, not
create a parallel retrieval stack or cross-folder graph.

## Acceptance Checks For Future Authorized Work

- Identity/integrity: synthetic preamble + punctuation heading; duplicate IDs;
  wrong-type/missing endpoints; invalid ownership/`CONTAINS`; cross-document
  `NEXT`; valid unresolved mentions. Invalid publication preserves `CURRENT`;
  hash-consistent but structurally invalid artifacts fail before query indexing.
- Evidence: repeated/self/reference-style links, encoded paths, fragments and
  empty targets; exact spans against stored snapshot bytes; deterministic
  occurrence ordering; unchanged projected edges and synthetic B1/B2/B3 results.
- Bounds: cap-minus-one/cap/cap-plus-one for facts, edges, diagnostics and bytes;
  dense parser events and high-degree neighbours; typed failure before excessive
  accumulation; no partial publication; complete wrapped response limits.
- Compatibility: schema rejection; canonical artifact-byte equality across
  source-mtime-only changes; no weakened current-byte freshness; corpus/generation
  identity isolation, including matching local IDs in different corpora.
- Incremental work, if later approved: canonical-byte equality with full rebuild
  after additions/deletions/renames/anchor edits and broken-link repair.
- Ranking work, if later approved: follow the ADR's dev-only tuning and frozen,
  powered held-out protocol. More nodes, unit tests or narrower CIs alone are
  not evidence of H2/H5 success. The donor synthetic recall test requires six
  rank-one answers; it is no controlled graph-versus-lexical comparison
  (`D/test/unit/markdown-query-ranking-recall.test.ts:75-106`).

## Validation And Scope

Source/identity reads exited 0. The session's inline `python3 -` documentation
check passed (exit 0): relative links, source-reference ranges, structure,
whitespace, the 52-line memory router and 74 baseline file hashes. Its first
attempt caught an overlong citation range; that range was corrected and the
check rerun with `set -e`. `git diff --check` exited 0, supplementary only because
the project files are untracked. The baseline check found only `MEMORY.md`
changed among existing protected files; this audit is new.

Donor freshness navigation succeeded; later context lookup failed as disclosed
above. No builds, Rust tests, fmt/clippy, donor tests or retrieval benchmarks
were executed: this is a documentation-only review. No controlled
documentation-graph gain was established by this bounded inspection.

Changed files: this audit and its link in `MEMORY.md` only. Source, ADR,
implementation plan, architecture router, agent instructions and evaluation
artifacts remain unchanged. This report is neither an ADR amendment nor a task
closeout, and its recommendations do not authorize implementation.
