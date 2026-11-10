# GraphRAG Donor Ideas Review

Date: 2026-11-10
Scope: useful ideas from the downloaded Microsoft GraphRAG main snapshot.
Disposition: recommendations only; no implementation, ADR amendment, schema,
ranking, evaluation configuration or task-order change. Independent challenge:
GPT-6-Astra, effective model confirmed by the agent runtime.

## Recommendation

**Borrow evidence and accounting patterns, not GraphRAG's retrieval stack.**
The strongest new idea is tying rendered context to its actual source fragments.
Context-budget accounting is the next useful refinement. Topology and build
diagnostics are operational options, not evidence of retrieval improvement.

| Idea | Smallest useful application | Gate |
| --- | --- | --- |
| Rendered-text/source parity | Clarify supporting-chunk citation semantics; add fragment provenance only for a required consumer | Citation contract; schema/API review for new artifacts/fields |
| Context-selection accounting | Reuse existing assembly state in bounded offline or trusted-CLI diagnostics | Distinct loss states; unchanged delivered context |
| Layer-aware topology | Count all chunks and components with and without `NEXT`, offline | Defined projection; no pruning or ranking changes |
| Operational outcome/detail separation | Preserve freshness totals; add sanitized phase errors only for an observed need | Public response review; publication remains authoritative |

These are recommendation priorities, not task ordering. EVAL-1.3 remains active;
EVAL-2.0 remains blocked. New work requires separately authorized scoped tasks.

## Evidence Boundary

- Snapshot: `tmp/graphrag/`, main revision
  `34b2b67ae4601ad0b47bcd179fbe3f2a23aa0857`, package version 3.3.0,
  MIT licence. Provenance: [source receipt](../tmp/graphrag-source.json).
  The archive has no Git history. All 913 original files were checked against
  [pre-index hashes](../tmp/graphrag-preindex-files.json) during this review.
- Ideas only: no donor code, prompts or data copied; no donor runtime installed
  or executed. `D/` below means `tmp/graphrag/packages/graphrag/graphrag/`,
  relative to the OntoDoc root. File/line citations refer to this pinned snapshot.
- [Index receipt](../tmp/graphrag-index.json) records successful structural
  indexing, not usable current workspace navigation or complete coverage.
  Committed reads lack a commit identity; workspace preparation fails with
  `workspace_build_failed / ENOENT` on a nonexistent nested `ontocode-rs` path.
  **OntoIndex stale/degraded: snapshot workspace preparation fails; source
  inspection is authority.** Unchanged requests/reindexing were not repeated.
- Three read-only specialists reviewed context, graph and pipeline owners;
  GPT-6-Astra independently verified and challenged the shortlist. Findings
  below are source-derived; proposed failure cases were not run.
- Authority remains [ADR revision 4](ADR_ONTODOC_SERVICE.md). Stdio and its five
  tools are implemented. Folder-scoped HTTP is accepted design only:
  `docs_share/<name>/` → `/<name>/mcp`, each with its own store. Graphs, statistics,
  citations and caches remain corpus-local even when local IDs coincide.

## 1. Tie Rendered Context To Its Actual Evidence

GraphRAG constructs the context row and returned source record from the same
values and accepts them together after a budget decision
(`D/query/context_builder/source_context.py:54-79`). Its entity/relationship
merge also retains contributing text-unit IDs
(`D/index/operations/extract_graph/extract_graph.py:104-128`). These record-level
associations establish neither byte verification nor truth of extracted claims.

OntoDoc's L0 includes ancestor headings; L1 includes descendant outline text
(`src/tiers.rs:41-68,82-99`). Search attaches the supporting chunk's citation
regardless of selected tier (`src/cli.rs:523-537`; `src/cite.rs:33-43`). A valid
chunk citation therefore does not verify every rendered L0/L1 fragment. This
does not establish that the citation itself is invalid or that H4 fails.

**Minimum option:** clarify whether a citation identifies the supporting chunk
or certifies all rendered text. Only if exact rendered provenance is required,
extend existing tier/citation owners with bounded source-snapshot fragments.
Reconstruction must distinguish source bytes from inserted separators and retain
UTF-8 cap boundaries. Do not represent a composed tier as one contiguous span.
Persisted fragments require schema review; new public fields need API/ADR
adjudication. Keep section identity, tier text, packing and ranking unchanged.

This is more specific than the earlier
[successful-link evidence proposal](audit_session-2026-11-10-ontoindex-graph-ideas.md).
GraphRAG's many-source association also reinforces that proposal: preserve a
bounded sorted occurrence list behind one deduplicated structural edge only if
an explanation consumer needs it. Repeated links must not increase PPR weight.
Do not create a parallel provenance subsystem or infer semantic entities.

**Future synthetic acceptance:** ancestor/descendant text outside the supporting
chunk; repeated identical sentences; UTF-8 cap cuts; inserted separators; a
source edit outside the chunk that changes its rendered outline. Reconstruct
returned text exactly from snapshot fragments plus declared formatting. Enforce
fragment/publication caps and corpus/generation binding. With diagnostic-only
changes, delivered IDs, order, tiers and text must remain identical.

## 2. Preserve Context Accounting Without Changing Selection

GraphRAG can return candidate text units with `in_context` membership
(`D/query/structured_search/local_search/mixed_context.py:354-375`). OntoDoc
already computes selected tiers, selected text bytes and a below-L2 flag
(`src/assemble.rs:38-65`), but search retains only `.tiers`
(`src/cli.rs:511-520`). Reuse those calculations rather than inventing a second
context selector.

Keep the loss states distinct:

1. Stored L0/L1 text hit its construction cap
   (`src/store.rs:130-133,348-351`).
2. Assembly selected a tier below L2, or omitted a hit that did not fit.
3. Final response bounding removed an entire result
   (`src/cli.rs:351-366`).

Search currently sets its aggregate flag for whole-hit omissions, while each
returned result exposes its chosen tier. Resolve the public meaning of
`truncated` before combining these states; do not blindly OR their booleans or
call this a confirmed ADR violation. Name stages precisely: assembly input is
`ranked.hits`, not necessarily the ranker's full candidate population.

**Minimum option:** bounded offline or trusted-CLI counts for assembly input,
selected tiers, omitted hits, stored-tier cap hits, text bytes and final response
removals. No raw query/document logging. `text_budget_bytes` is not the size of
the complete serialized MCP response.

Adding fields to a bounded response can itself evict additional results. Thus
unchanged ranker output alone does not prove unchanged delivered context. Public
diagnostics need an explicit response-contract decision and complete wire-bound
checks, including metadata and errors. GraphRAG stops at its first oversized row
(`D/query/context_builder/source_context.py:66-67`); retain OntoDoc's existing
skip-and-continue assembly (`src/assemble.rs:49-59`).

**Future synthetic acceptance:** every hit retained below L2; omitted L0;
stored-tier cap hit; small later item after an oversized item; escaped text;
metadata near the response cap. Reconcile pre/post-bound counts and verify
delivered IDs, order, tiers and text, not only scores. Preserve the ADR's fixed
breadth-first L0 → L1 → L2 policy.

## 3. Inspect Topology By Relation Layer, Including Isolates

GraphRAG includes entity-only isolates in its degree population, then can remove
them by degree or largest-component filters
(`D/index/operations/prune_graph.py:29-39,49-58,97-103`). Clustering can select
the largest connected component (`D/index/operations/cluster_graph.py:69-70`;
`D/config/defaults.py:68-73`). Borrow population accounting, not pruning.

OntoDoc already initializes every chunk node and redistributes dangling mass
(`src/search/mod.rs:149-160`; `src/search/ppr.rs:25-39,60-72`). Isolate
preservation is existing behavior, not a new donor feature to implement.

**Minimum option, for a named diagnostic need:** offline bounded counts of all
chunk nodes, isolates and component sizes under undirected `LINKS_TO`, then under
`LINKS_TO+NEXT`. Report chunks incident only to `NEXT`; reading-order chains can
mask a lack of cross-document links. Define first-chunk projection, reversed-edge
deduplication and self-loop handling consistently with the existing graph.
No per-query full-graph statistics, node removal, degree boosts or quality score.

**Future synthetic acceptance:** edgeless corpus; NEXT-only chain; small linked
component beside a larger one; duplicates, reversed links and self-loops.
Hand-count both projections. Preserve searchable nodes, adjacency, scores and
canonical generation bytes for diagnostic-only work.

## 4. Keep Operational Diagnostics Small

GraphRAG separates workflow outcome from aggregate/per-workflow metrics
(`D/index/typing/pipeline_run_result.py:13-22`;
`D/index/typing/stats.py:27-42`). Its runner yields a workflow result before
persisting the associated statistics; an early-stop request also reaches the
completion log (`D/index/run/run_pipeline.py:135-150`). These events are not
proof of durable complete publication.

OntoDoc reports generation counts after `publish` succeeds
(`src/cli.rs:423-434`). Status retains totals for skipped files/link diagnostics,
but caps added/removed/modified lists without their original counts
(`src/cli.rs:327-342,659-665`).

**Minimum option:** exact freshness totals before list truncation are smaller
than new pipeline infrastructure. If debugging shows a need, add fixed sanitized
build/publish phase and terminal error/outcome information in existing
`src/store.rs`/`src/cli.rs` owners. Public fields require contract review. Keep
timing/machine metadata outside canonical generation bytes, and logs on stderr.
No callback registry, workflow framework, generic cache or sidecar subsystem.

**Future synthetic acceptance:** more changed paths than the detail cap; totals
survive response trimming; failure before publication; publication failure;
previous `CURRENT` preserved; no premature published-success report. Test
complete response bounds and deterministic artifacts if implementation is later
authorized.

## Keep, Defer And Reject

- **Keep full immutable rebuilds.** Existing content/configuration identity is
  appropriate (`src/store.rs:222-229,365-392`). Donor delta detection compares
  titles, missing changed bytes under unchanged titles; the reviewed loader
  returns new inputs, not its computed deletions
  (`D/index/update/incremental_index.py:46-58`;
  `D/index/workflows/load_update_documents.py:61-62`). Relationship updates
  average old/delta weights (`D/index/update/relationships.py:46-62`), warning
  against assuming batch-independent equivalence. The previous graph audit
  already defers incremental work; require full-rebuild canonical equivalence
  across edits, additions, deletions, renames, repaired targets and batch splits
  if measurements later justify it.
- **Defer semantic communities and global synthesis.** Noun phrase equality and
  within-unit co-occurrence do not establish a relation or resolve homonyms
  (`D/index/operations/build_noun_graph/build_noun_graph.py:67-86,114-141`).
  Community artifacts select intra-community relationship evidence, not every
  member mention or boundary relationship
  (`D/index/workflows/create_communities.py:113-155`). A future experiment needs
  explicit identity, evidence/omission and isolate semantics before a ranker.
  It is a separate research arm, not an alteration to current H2/H5.
- **Do not treat FastGraphRAG as model-free.** Its workflow still includes
  community reports and text embeddings (`D/index/workflows/factory.py:63-72`);
  report creation obtains a completion model
  (`D/index/workflows/create_community_reports_text.py:51-58`). No LLM,
  embedding, DRIFT/local-global router or generated report path is adopted.
- **Keep fixed B2/B3 and assembly.** New edges, weights, diversity, ordering,
  packing or fusion require separately approved dev experiments and
  preregistration. Structural headings/tiers and atomic stores already exist;
  do not relabel them as new GraphRAG inventions.
- **Keep the existing HTTP gates.** Donor row derivation creates every task
  before semaphore admission (`D/index/utils/derive_from_rows.py:100-112`).
  This is not bounded queued work or cancellation ownership. Total/per-corpus
  admission, actual-work permit lifetime and corpus isolation remain existing
  requirements for scoped HTTP delivery, not new benefits of this review.

## Validation And Scope

Source inspection and donor hash verification support this report, not runtime
failure or retrieval-quality claims. No build, Rust/donor tests, fmt/clippy,
benchmark, performance experiment or held-out evaluation ran; none is required
for this documentation-only review. Synthetic checks above are future acceptance
criteria, not executed results.

Documentation validation checks relative links, source-reference ranges,
structure, whitespace, the memory-router line cap, original donor hashes and
the 76-file pre-edit project baseline. `git diff --check` is supplementary:
the project files are untracked, so an empty diff cannot establish write scope.
The inline `python3 -` documentation/integrity check passed, exit 0 (receipt
`5c7427`): 24 relative links, 31 source references, a 54-line memory router,
76 protected baseline files and 913 original donor hashes. Only `MEMORY.md`
changed among baseline files; this audit is the sole new file in the reviewed
project trees. `git diff --check` also exited 0 in the same command receipt.
GPT-6-Astra's delta review accepted the report without material corrections
(`agent-results/01a1c5c5-68ee-7b12-a1f1-b25c205079ed/2`); it did not independently
rerun the coordinator's hash or write-scope checks.

Changed files: this audit and its link in `MEMORY.md` only. Source, donor files,
ADR, implementation plan, architecture router, agent instructions and evaluation
artifacts remain unchanged. No quality, throughput or cost improvement is claimed.
