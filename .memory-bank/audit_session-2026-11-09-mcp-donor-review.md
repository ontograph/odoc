# Audit Session: MCP Donor Ideas Review

Date: 2026-11-09
Scope: user-requested architecture review of `../ontocode/tmp/repo/mcp`.
Independent challenge: GPT-6-Astra, requested and effective model confirmed by
the dispatch receipt for `/root/donor_astra_challenge`.

## Recommendation

Keep OntoDoc's accepted Rust architecture. The donor supplies useful prompts
for future identifier discovery and corpus preparation, but no demonstrated
graph-RAG or retrieval-quality improvement. No implementation, dependency,
MCP surface, ADR, evaluation configuration or plan change is authorized by
this record. EVAL-1.3 remains the active implementation task.

## Evidence And Provenance

The reviewed directory contains eight Python files. `D` below means
`../ontocode/tmp/repo/mcp`; unprefixed `src/` paths refer to OntoDoc.

- No donor-specific licence, README or dependency manifest was found in the
  reviewed MCP tree or immediate `tmp/repo` parent listing. Source-reuse rights,
  upstream origin, dependency versions and donor revision remain unresolved.
  The application's licence-key check is not a source-code licence grant.
- The enclosing Ontocode checkout is at
  `4e74de1f4b38cb7fe7e281e934ae25caed0f34e3`, but `git ls-files tmp/repo/mcp`
  returns no files. That commit does not pin this donor. Do not infer reuse
  permission from the outer checkout's licence. No donor code, prompts or data
  were copied.
- OntoIndex's scoped search under registered repository `codex` returned no
  definitions and degraded-capability warnings. Source inspection is authority;
  an empty indexed result is not evidence that the files lack functionality.
- Runtime behavior, generated MCP schemas and dependency behavior were not
  tested. Findings distinguish source-visible behavior from possible effects.

## Implemented Flows

**Standalone help search:** startup extracts HTML documentation, converts it
to Markdown plus searchable plain text, embeds whole files and writes Chroma
batches. `docsearch(query)` combines substring and vector results, then returns
joined Markdown strings. One-word queries put substring results first; two or
three words put vector results first; four or more use vectors only. Distances
are requested but do not determine fusion or abstention
(`D/helpsearch/main.py:389-528,531-642`).

**Code/metadata service:** startup opens three Chroma collections for metadata,
code and help. Metadata/code use generic 1,000-character, zero-overlap chunks;
help uses its HTML parser. Tools run vector, substring and filename matching,
deduplicate by record ID and concatenate results in a query-dependent order.
Recognized dotted-name prefixes have domain-specific singular/plural rewrites
(`D/codemetadata/vectorindexer/indexer.py:87-95,161-186,188-430`).

The named metadata and code parsers are not used by the active indexing path.
Their dotted object paths and properties are not graph edges. The reviewed
search paths do not traverse a document or code graph
(`D/codemetadata/vectorindexer/metadata_parser.py:190-315`,
`D/codemetadata/vectorindexer/code_parser.py:8-34`,
`D/codemetadata/vectorindexer/indexer.py:188-292,318-379`).

## Ranked Idea Decisions

| Idea | Decision | OntoDoc owner and adoption condition |
| --- | --- | --- |
| Filename/path-derived discovery context | **Most promising new experiment; defer.** The donor derives help titles from paths and also matches filenames (`D/codemetadata/vectorindexer/help_parser.py:30-46,66-72`; `indexer.py:331-343`). | B1 currently indexes headings and chunk text, not filenames (`src/search/mod.rs:125-130`). If dev queries establish filename-only misses, compare an explicit path-context arm against unchanged B1 in `src/search/`. Keep derived context out of quoted evidence. |
| Exact identifiers and bounded aliases | **Defer; unproved ranking benefit.** Borrow the distinction between identifier discovery and descriptive questions, not the word-count routing. The donor rewrites known, case-sensitive prefixes before a dot, rather than implementing general stemming or an exact identifier index (`D/codemetadata/vectorindexer/indexer.py:381-415`). | `src/search/tokenize.rs:9-29` already splits camelCase/snake_case identifiers and dots, but does not preserve a complete dotted token or expand aliases. A later arm must retain the original query, bound expansions, handle ambiguity and explain the match. `read(path#anchor)` resolves a known location; it does not discover an unknown identifier (`src/cli.rs:552-567,679-688`). |
| Searchable text separate from displayed evidence | **Keep the principle; existing Markdown capability, not equivalent HTML support.** Donor HTML parsing produces plain text and Markdown (`D/codemetadata/vectorindexer/help_parser.py:62-74`). | OntoDoc tokenizes source headings/chunks and returns source-derived tiers with citations (`src/search/mod.rs:125-130`; `src/cli.rs:499-538`). Retain this boundary; no new representation layer is needed for v1. |
| Offline HTML normalization | **Defer until an HTML corpus is required.** This expands corpus coverage, not necessarily ranking quality. | An explicitly approved, deterministic offline preparation step could emit local Markdown for the existing `src/corpus.rs` → `src/parse.rs` flow. No donor runtime or second index backend. Pin converter/configuration, record input/output hashes, preserve headings/code/link labels and destinations, and report lossy conversion. Citations would identify generated Markdown; original-HTML spans need a separate mapping contract. |
| Retrieval provenance | **Already substantially covered; extend only for an approved new arm.** Donor records `search_type` (`D/codemetadata/vectorindexer/indexer.py:318-379`). | Existing `explain` exposes score components and pre-fusion BM25 rank (`src/search/mod.rs:84-91,216-225,274-286`; `src/cli.rs:173-176,269`). Match provenance would belong here, not in a separate MCP tool. |
| Separate metadata/code/help tools | **Reject for v1.** These are donor corpus categories, not evidence that OntoDoc needs three retrieval services. | Keep the five-tool contract in `src/mcp.rs:140-210`. Path-prefix scope already exists (`src/search/mod.rs:185-194`), but is not an entity-type facet. New facets or tools require a concrete need, ADR amendment and compatibility tests. |
| Index/configuration compatibility | **Keep existing owner; reject donor relabeling.** | OntoDoc hashes index configuration and content into a generation and verifies schema/artifact identity on load (`src/store.rs:222-229,484-523`). Renaming an embedding model attribute does not establish compatible vectors (`D/helpsearch/main.py:695-733`). |

## Approaches Not To Adopt

1. **Direct active-collection writes and count-only readiness.** Nonempty
   collections can suppress reindexing after source edits/deletions; indexing
   batches are written without generation publication. Logged batch failures
   can leave a subset available. Preserve OntoDoc's locked, synced generation
   publication (`D/codemetadata/vectorindexer/indexer.py:161-186,213-239,266-292`;
   `D/helpsearch/main.py:470-528,788-797`; `src/store.rs:430-469`).
2. **An uncapped hybrid union presented as `limit` results.** Codemetadata
   combines up to three separately limited lists without a final cap, allowing
   up to `3 × limit` distinct records. Standalone helpsearch caps candidate
   counts, but returns whole Markdown without a response-byte cap. Batch size
   is not a file, corpus or response bound. Keep final item/serialized-byte
   limits and explicit truncation (`D/codemetadata/vectorindexer/indexer.py:318-379`;
   `D/helpsearch/main.py:574-642`; `src/cli.rs:24-26,345-362,464-467`).
3. **Unversioned model switching or runtime embedding fallback.** Query
   embeddings can call a configured API; codemetadata's API request hardcodes
   a model rather than using its saved configured model. Vector search happens
   before exact matching, so embedding failure prevents that fallback. In
   standalone helpsearch, changing the API selector may affect later requests,
   but changing a local model-name attribute does not demonstrate reloading
   weights. These are compatibility/availability risks, not reproduced failures
   (`D/codemetadata/vectorindexer/indexer.py:29-68,128-159,318-343`;
   `D/helpsearch/main.py:280-290,695-733`). V1 has no embeddings; a dense arm
   can only follow the ADR's later hypothesis gate
   (`ADR_ONTODOC_SERVICE.md:280-281`).
4. **Markdown strings as sufficient evidence.** The donor's help output omits
   source spans, hashes, generation identity and freshness. Codemetadata exposes
   record metadata, but not verifiable citation spans. Keep `Citation`,
   `verify`, generation freshness and structured responses
   (`D/helpsearch/main.py:591-619,641-642`;
   `D/codemetadata/vectorindexer/indexer.py:350-357`;
   `src/cite.rs:20-47,50-84`; `src/cli.rs:273-293`).
5. **Lossy normalization and implicit exposure.** Standalone helpsearch removes
   entire anchor elements, including their labels, from searchable text; query
   punctuation is also removed. Recall effects need measurement. The reviewed
   HTTP servers bind to `0.0.0.0` without request authentication configured in
   those files; ordinary file opens can follow file symlinks. These are
   source-visible risks, not demonstrated exploits. Keep local stdio and
   beneath-root corpus reads. Donor stdout logging is not evidence of broken
   HTTP framing, but must not be transplanted into stdio
   (`D/helpsearch/main.py:52-58,433-458,557-559,783-797`;
   `D/codemetadata/main.py:159-165`;
   `D/codemetadata/vectorindexer/indexer.py:188-210`; `src/mcp.rs:226-234`).

The donor's separate indexing CLI also has a static call/signature mismatch:
`D/codemetadata/index_repository.py:21` supplies one argument, while
`D/codemetadata/vectorindexer/indexer.py:165` requires three excluding `self`.
This was confirmed with AST inspection, not by executing the CLI.

## Scientific And Implementation Gate

No controlled baseline, section qrels, confidence intervals or held-out quality
result was established in this review. Vector-plus-substring search is not a
graph intervention. The donor does not prove any improvement over OntoDoc.

If later authorized, compare unchanged B1 separately with path context, exact
identifier discovery and bounded aliases; do not conflate their effects with
HTML corpus expansion. Use dev queries to diagnose misses and measure both
targeted lookup gains and unrelated-query regressions. Keep stable section IDs,
the frozen chunk budget, deterministic tie-breaking, citations and bounds.
Any confirmatory claim needs separate preregistration, an appropriate multiplicity
and power treatment, and sealed held-out inputs under the ADR. Do not append
these arms to the current confirmatory family or change the default from donor
inspection. This review does not resolve EVAL-1.3's benchmark-integrity work.

Astra's challenge specifically corrected three overstatements: Markdown
tokenization is not full HTML/plain-text support; known-ID reads are not
identifier discovery; and donor path-derived discovery context is not already
part of OntoDoc B1. Those corrections are incorporated above.

## Validation And Scope

- Static Python AST check: all eight donor files parse; the indexing CLI's
  one-versus-three argument mismatch is confirmed. `python3` exited **0**,
  parent receipt `4f21e0`. No donor module was imported or executed.
- Codemetadata test/evaluation-marker probe: `rg` exited **1**, no matches in
  the assigned directory, receipt `29445b`. This is bounded absence evidence,
  not proof that no external tests or results exist.
- Independent GPT-6-Astra source-review receipts: `30d60e`, `c4b1ac`, `91ebf6`,
  `667dc3`, `e95fa5`, `59c4f1`, `9a99bc`, each exited **0**. The review is
  architecture evidence, not human annotation or a benchmark result.
- Documentation/scope check: `python3` exited **0**, receipt `9fcfd9`:
  51 protected runtime/ADR/plan/input files unchanged, no new implementation
  files, 38 explicit source references resolve, Markdown checks pass and
  `MEMORY.md` remains within 100 lines. Reference resolution does not prove
  that a cited passage supports its interpretation.
- No donor execution, dependency installation, MCP handshake, Rust build/test,
  formatting, Clippy or evaluation run was performed. Runtime checks were
  skipped because this is a documentation-only architecture review. No clean
  build, test suite, benchmark or production-readiness claim is made.
- Written artifacts: this audit and its link in `MEMORY.md`. Rust sources,
  fixtures, evaluation inputs, ADR, implementation plan and task status remain
  unchanged.
