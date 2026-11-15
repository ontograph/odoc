# GraphRAG With References: Implementation Plan

Date: 2026-11-13
Revision: 4, implementation and validation recorded on 2026-11-13
Status: Closed. GR-REG, GR-0, GR-2 and GR-DOC are DONE. Deferred work remains outside this execution.
Authority: service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (Accepted, revision 17), the user's proceed instruction, `AGENTS.md`, and `docs/ADR_DONOR_ADOPTION.md` (revision 5). The evidence/diagnostics ADR remains proposed.
Requested by: user, "consolidate best proposals to implementation plan md file; md - only; read only; graphrag with references", then "review and challenge; update doc".

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: closed
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: null
tasks:
  GR-REG:
    title: Register approved GraphRAG scope and post-0.1.0 release decision
    status: DONE
    classification: architecture-paperwork
    type: tracking
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - docs/GRAPHRAG_REFERENCES_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    allowed_write_set:
    - docs/GRAPHRAG_REFERENCES_PLAN.md
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/project_pending-tasks.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    non_goals:
    - No ranking, schema, evaluation, frozen artifact or publication changes.
    required_validation:
    - git diff --check
    expected_evidence:
    - Strict plan validates; ADR records always-on neighbour citations; 0.1.1 development proceeds independently of REL-6; preserved artifacts and custody unchanged.
    rollback:
    - Revert only this task's edits, preserving pre-existing work.
    stop_conditions:
    - Conflict with accepted service ADR.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User instruction to proceed tasks from this plan, 2026-11-13.
        validation_summary: Strict project_plan_validate PASS four tasks; git diff --check exit 0. ADR17 and audit_session-2026-11-13-graphrag-references.md record always-on citations and independent unreleased 0.1.1 development. No frozen artifact, evaluation or publication change.
    dod_status: pass
  GR-0:
    title: Document current citation coverage and bounded graph neighbours
    status: DONE
    classification: docs/design-only
    type: documentation
    validation_tier: unit-fast
    depends_on:
    - GR-REG
    owner_files:
    - README.md
    allowed_write_set:
    - README.md
    non_goals:
    - No ranking, schema, evaluation, frozen artifact or publication changes.
    required_validation:
    - git diff --check
    expected_evidence:
    - README statements checked against tiers, search, related and graph source and ADR; release verifier README coupling checked.
    rollback:
    - Revert only this task's edits, preserving pre-existing work.
    stop_conditions:
    - Conflict with accepted service ADR.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks instruction, 2026-11-13.
        validation_summary: README checked against src/tiers.rs chunk_tiers/section_l1, src/cli.rs search/related/bound, src/graph.rs EdgeKind/resolve and ADR items 4/8; git diff --check exit 0. scripts/verify_release.py ROOT_FILES/workspace_inventory excludes working README; package member checks use preserved archive bytes. No Rust compilation needed for GR-0.
    dod_status: pass
  GR-2:
    title: Return verifiable citations for chunk and section neighbours
    status: DONE
    classification: implementation-ready
    type: code
    validation_tier: release
    assurance_profile: full
    complexity: complex
    impact_risk: medium
    depends_on:
    - GR-0
    owner_files:
    - src/cli.rs
    - src/mcp.rs
    - tests/mcp.rs
    - tests/http.rs
    - Cargo.toml
    - Cargo.lock
    allowed_write_set:
    - src/cli.rs
    - src/mcp.rs
    - tests/mcp.rs
    - tests/http.rs
    - Cargo.toml
    - Cargo.lock
    non_goals:
    - No ranking, schema, evaluation, frozen artifact or publication changes.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --test http
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    - just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-gr2-clippy-20261113 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
    - git diff --check
    expected_evidence:
    - Unchanged-fixture chunk and parent citations verify valid; mentions omit citation; unknown or wrong-kind endpoints return corrupt; complete stdio and HTTP frames truncate under 128 KiB and have equal responses; real published ordinary corpus edge counts recorded.
    rollback:
    - Revert only this task's edits, preserving pre-existing work.
    stop_conditions:
    - New tool, request field, stored structure or wider response bound required.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks instruction, 2026-11-13.
        validation_summary: Focused CLI 12, MCP 13 and HTTP 7 tests passed; all-targets 293 passed and 3 ignored; all five compiler-lane result.json receipts under /opt/d2/.ontocode/artifacts/compiler-lane/ontodoc-gr2-{cli,mcp,http,all,clippy}-20261113 are terminal exit 0. cargo fmt --check and git diff --check exit 0 after local formatting correction. Independent citation_review found no blocking correctness gaps. Ordinary Rust Book index has 1370 chunks, maximum 132 incident neighbours, 2 chunks above 100. OntoIndex impact LOW; bounded verify_diff passed with expected files and dirty-tree limitations. Exact commands, corpus hash and review limits recorded in the closeout audit by GR-DOC.
    dod_status: pass
  GR-DOC:
    title: Document neighbour citations and unreleased 0.1.1 behavior
    status: DONE
    classification: docs/design-only
    type: documentation
    validation_tier: unit-fast
    depends_on:
    - GR-2
    owner_files:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    allowed_write_set:
    - README.md
    - CHANGELOG.md
    - .memory-bank/project_architecture.md
    - .memory-bank/audit_session-2026-11-13-graphrag-references.md
    non_goals:
    - No runtime or release artifact changes.
    required_validation:
    - git diff --check
    expected_evidence:
    - README and changelog describe verified neighbour citations and limits; audit records exact validation outcomes and corpus counts.
    rollback:
    - Revert only this task edits.
    stop_conditions:
    - Documentation contradicts validated behavior.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed tasks instruction, 2026-11-13.
        validation_summary: README, CHANGELOG and architecture describe the implemented citation contract and limits; execution audit records exact commands, terminal receipts, independent review and independently counted Rust Book edges with index hash. git diff --check exit 0; Python checks of eight Markdown files passed local link paths, fence balance, whitespace, memory bounds and package/lock version 0.1.1. Strict project_plan_validate PASS four tasks before evidence insertion; insertion triggered the conditional-repair classifier, corrected once by clarifying corpus-count wording. Documentation-only changes require no additional Rust compilation. Runtime write set was six files; documentation and manager maintenance stayed within registered owners. GR-5 and REL-6 remain deferred or blocked independently.
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| GR-SCOPE | Proceed with structural GraphRAG references under ADR item 8 | architecture-gate | GR-REG | Strict tracker validation and recorded decision |
| GR-DOC | Explain supporting chunk versus tier text and neighbour limits | implementation | GR-0 | Source and ADR cross-check |
| GR-CITE | Cite chunk/section neighbours, omit unresolved mentions, fail corrupt edges | implementation | GR-2 | Focused unit and transport regressions |
| GR-BOUND | Preserve order, item and complete frame limits and transport parity | compatibility-test | GR-2 | Complete stdio/HTTP oversized-frame regressions |
| GR-RELEASE | Separate 0.1.1 identity; preserve 0.1.0 artifacts and custody | implementation | GR-REG, GR-2, GR-DOC | Package version and scope checks |
| GR-FUSION | Separate optional diagnosis requires its own rule decision | non-goal | GR-REG | No dev or held-out payload access |

## Goal

When OntoDoc reports a graph neighbour that is a chunk or a section, it returns a citation for it that `verify` accepts. The corpus stays Markdown only, the service stays read only, and no model or network joins the retrieval path.

The original document-only request constrained proposal work. The user's subsequent proceed instruction authorizes the task write sets above.

Revision 1 promised a checkable reference for "every graph relation". That was too strong; [Limits After GR-2](#limits-after-gr-2) lists what stays uncovered.

## What The Term Means Here

OntoDoc has no generation step; the client model writes the answer. "GraphRAG with references" means four existing tools working together:

1. `search` in `graph` mode ranks chunks by BM25 fused with Personalized PageRank over the link graph. Every hit carries a citation.
2. `related` walks one step from a chunk along typed edges.
3. `read` returns cited text.
4. `verify` checks a citation against current file bytes.

All four steps now return or check citations for chunk and section endpoints. Unresolved code mentions have no citation.

It does not mean Microsoft GraphRAG's pipeline. Entities, community reports, model-written summaries, embeddings and global search are in [Non-Goals](#non-goals).

## Current State

Current owners are listed below; historical review references later in this document describe the proposal baseline. The working tree includes pre-existing uncommitted work.

| Capability | Today |
| --- | --- |
| Graph edges | `CONTAINS`, `NEXT`, `LINKS_TO`, `MENTIONS`, deduplicated and stored in that order (`src/graph.rs:35`, `src/graph.rs:72`) |
| Link targets | A link to a section or document becomes an edge to the first chunk of that target (`src/graph.rs:223`). A code span becomes `MENTIONS` to an unresolved `code:<text>` id (`src/graph.rs:13`) |
| Graph ranking | PPR over an undirected weight-1 graph of `LINKS_TO` and `NEXT` (`src/search/ppr.rs:12`, `src/search/mod.rs:251`) |
| Search references | Every result has a chunk citation. Under `budget_bytes`, L0 and L1 can include heading and outline text outside the cited span (`src/tiers.rs::chunk_tiers`); README explains the supporting-chunk boundary |
| Neighbour references | `src/cli.rs::App::related` returns `kind`, `direction`, `id` and citations for chunk/section neighbours. Mentions omit citations. At most 100 neighbours, further bounded by serialized bytes, with `truncated` and no total or pagination |
| Verification | A citation must equal a stored chunk or section citation of the active generation (`src/cite.rs:86`). Any other span is refused |
| Tool contract | Structured JSON without a published output schema (`src/mcp.rs`). Synthetic tests verify cited endpoints, mention omission, corrupt endpoints, complete-frame bounds and stdio/HTTP parity (`src/cli.rs`, `tests/mcp.rs`, `tests/http.rs`) |

## Review Of Revision 1

Three read-only explorers checked revision 1: one against source line by line, one against the evaluation code, one as an independent challenger. The coordinator re-read the disputed source.

What held: the source references, the dev figures quoted from the donor ADR, the non-goals, and the central idea that neighbours should carry citations.

| Finding | Evidence | Change |
| --- | --- | --- |
| The goal overstated the result | Mentions have no target to cite; the 100-neighbour cap has no continuation; links attach to a target's first chunk | Goal narrowed; limits listed |
| GR-1 (edge validation) was not a prerequisite | `related` can refuse an id it cannot resolve. No malformed artifact has been observed. The proposed rules also missed `NEXT` adjacency and `CONTAINS` parentage | Deferred |
| GR-3 (topology) does not concern references, and its card was understated | CLI `status` takes no flags (`src/cli.rs:61`); the search graph is private (`src/search/mod.rs:190`); the evidence ADR requires more fields and frozen ceilings | Deferred |
| GR-4 (link spans, schema 5) adds a locator that `verify` cannot check | `src/cite.rs:86`. After GR-2 the source chunk's citation already places a link within one chunk. No test establishes the parser range for reference-style links | Deferred |
| GR-5 (fusion diagnosis) could not run as written | A saved run holds the top 10 sections per query and system (`src/eval/run.rs:30`, `src/eval/run.rs:2454`). A miss record holds no replacement (`src/cli.rs:1430`). Nothing exposes PPR seeds or neighbours | Rewritten around what saved files hold; moved to a separate track |
| The reason given for ordering GR-5 before GR-4 was wrong | Analysing a saved run needs no binary. A schema change does block a new dev run under the old preparation, because the generation id hashes the index config (`src/store.rs:387`) and dev admission compares generation ids (`src/eval/run.rs:1216`) | Corrected |
| GR-6 was a card without content | It named three possible ranking changes and chose none | Withdrawn; conditions kept in one paragraph |
| "Most lexical misses are ordering misses" was ambiguous | The 78 pairs are B1's `truncated` stage, not its `lexical_miss` stage | Reworded |
| The gate for GR-2 was misjudged | The accepted service ADR, decision item 8, already says "Every result carries a citation" | Gate restated as a reading the owner confirms |
| Release and version handling was missing | Package version is 0.1.0; `CHANGELOG.md:3` has no unreleased section; publication task REL-6 is blocked | New section |
| The `related` tool description and wire shape were not covered | Description above the handler, `src/mcp.rs:449` | Added to GR-2 |
| The plan was long for its content | Six cards where two are needed | Cut |

Execution records use the session date, 2026-11-13.

## Plan

GR-0 and GR-2 deliver the requested behavior, with GR-REG recording authorization and GR-DOC recording final documentation. No task changes the stored index, schema, ranking or defaults, or needs a reindex or evaluation.

### GR-0: Document what references cover

- **Gate:** satisfied by the user's proceed instruction. It describes current behaviour.
- **Write set:** `README.md`.
- **Content,** beside the tool table (`README.md:118`):
  - Without `budget_bytes`, result text is exactly the cited span. With it, L0 and L1 text adds ancestor headings and outline text joined by newlines, and the citation still identifies only the supporting chunk.
  - `related` lists at most 100 neighbours in stored edge order: hierarchy, reading order, links, then code mentions.
  - A link to a section or document appears on that target's first chunk. To see what links to a section, call `related` on its chunk `~0`.
- **Check:** each sentence matches `src/tiers.rs:41`, `src/cli.rs:831`, `src/cli.rs:944` and `src/graph.rs:223`. The first point agrees with the service ADR, which defines tiers as verbatim fragments joined by one line feed.
- **Stop:** a sentence would contradict the service ADR. Report it; do not reword the ADR.

### GR-2: Neighbours carry citations

- **Gate:** ADR17 records always-on neighbour citations and separate unreleased 0.1.1 development under the user's proceed instruction.
- **Owner file:** `src/cli.rs`.
- **Write set:** `src/cli.rs`, `src/mcp.rs` (tool description only), `tests/mcp.rs`, `tests/http.rs`, and package version fields in `Cargo.toml`/`Cargo.lock`. README/changelog updates belong to GR-DOC.
- **Behaviour:**
  - `Neighbour` (`src/cli.rs:607`) gains an optional `citation`, serialized only when present, as `SearchResult.explain` already is (`src/cli.rs:565`).
  - The edge kind decides. A `NEXT` or `LINKS_TO` neighbour is a chunk (`Citation::for_chunk`, `src/cite.rs:45`). A `CONTAINS` neighbour of a chunk is its section (`Citation::for_section`, `src/cite.rs:31`). A `MENTIONS` neighbour gets none.
  - An id that does not resolve to the expected kind is a corrupt generation. Return a typed error; never return the neighbour without its citation.
  - Resolve the listed ids, at most 100, in one pass over chunks and sections. The request stays linear in generation size, as the existing existence check is (`src/cli.rs:944`).
  - Order, the 100-item cap and the 128 KiB bound are unchanged. When bytes run out, `bound` drops trailing neighbours, which are mentions first and links next (`src/cli.rs:727`).
  - The request is unchanged (`src/mcp.rs:40`). The tool description at `src/mcp.rs:449` says that chunk and section neighbours carry citations.
- **Excluded:** neighbour text, tier text, per-neighbour staleness, a total count, paging.
- **Acceptance, on synthetic fixtures:**
  - Every returned citation passes `verify` as `valid` on an unchanged fixture, for a chunk neighbour and for the parent section.
  - A mention neighbour has no `citation` key.
  - A chunk whose cited neighbours exceed 128 KiB returns a complete frame with `truncated: true` over stdio and HTTP. Long heading paths force this.
  - Stdio and HTTP responses are identical.
  - A generation with an edge to an unknown id makes `related` fail with the typed error.
- **Required validation:** `CARGO_BUILD_JOBS=4 cargo test --lib cli::`, `CARGO_BUILD_JOBS=4 cargo test --test mcp`, `CARGO_BUILD_JOBS=4 cargo test --test http`, `CARGO_BUILD_JOBS=4 cargo test --all-targets`, `cargo fmt --check`, `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`, `git diff --check`.
- **Evidence to report:** from the `edges` of one published index of an ordinary documentation folder, the largest neighbour count of any chunk and how many chunks exceed 100. This shows whether the cap matters on a real corpus. It reads no queries, labels or results.
- **Stop:** the change needs a new tool, a request field, a stored structure or a wider response bound.
- **Rollback:** revert the task. No stored data is affected.

Expected size, estimated and not measured: a citation is two 64-character hashes, a path, a heading path and four numbers, a few hundred bytes with ordinary headings. 100 neighbours then fit well inside 128 KiB. A heading path may reach 16 KiB (`MAX_CITATION_HEADING_BYTES` in `src/store.rs`), in which case the byte bound trims the list.

### How `related` returns citations

| Option | Effect | Verdict |
| --- | --- | --- |
| Always include `citation` | One code path and one response shape. No published output schema exists to break. Responses grow | **Recommended** |
| Opt-in request flag | Default bytes unchanged. Adds a request field, a second shape and tests for absent, false and true | Only if the owner requires byte-identical defaults |
| Leave as is | One `read` per neighbour, up to 100 calls | Rejected |

### Limits After GR-2

- **Mentions.** A `MENTIONS` neighbour is a code string with no target, so it has no citation.
- **Hubs.** Neighbours beyond the first 100 are not returned and cannot be requested. The smallest fix would be a kind filter or an offset on the request, which is a contract change. Decide it on the neighbour counts GR-2 reports.
- **Granularity.** A link neighbour's citation identifies the first chunk of the linked section or document. The section a link names is that chunk's heading path.
- **What is verified.** `verify` confirms that each endpoint's bytes are unchanged. That a link exists is shown by reading the source chunk; no tool certifies the relation separately.
- **Later chunks.** Links into a section are listed only on its first chunk.

## Deferred Work

Each item needs its own decision and task card. None is needed for the goal.

| Item | Why deferred | Reopen when | If reopened |
| --- | --- | --- | --- |
| GR-1: validate edges at publish and load | No malformed artifact has been observed, and the artifact hash is already checked on load. GR-2 refuses unresolved ids itself | A malformed generation is found, or an integrity requirement is stated | Check endpoints and kinds first. Search silently skips a ranking edge whose endpoint is unknown (`src/search/mod.rs:254`), which is the strongest reason to do it. A complete check of `CONTAINS` and `NEXT` recomputes them from sections and chunks and compares |
| GR-3: isolate and component counts on CLI `status` | Operator diagnostics; no bearing on references | The evidence ADR's topology row is accepted | Follow that ADR's field list and freeze its ceilings. Write set includes `src/search/mod.rs`, since the graph is private |
| GR-4: persist link spans (schema 5) | A span is not a citation `verify` accepts. It costs a reindex of every corpus, a new dev preparation and unmeasured artifact bytes | A named consumer must locate a link more precisely than its chunk | First establish by test what the parser range covers for reference-style links. Freeze caps before coding. Add HTTP parity tests and a changelog entry |
| Tier provenance parts, search accounting, `status` totals | Proposed in the evidence ADR; they do not concern the graph | That ADR is accepted row by row | Stay in that ADR |
| Exact-identifier or concept edges | A new relation kind is a schema and ranking change | An ADR and a fresh evaluation are approved | Separate plan |

## Separate Track: Fusion Diagnosis

This concerns ranking, not references. It has no dependency on GR-0 or GR-2 in either direction, and it is the only open question the measured evidence raises.

Dev-split figures, quoted from `docs/ADR_DONOR_ADOPTION.md` (Decision 2) and not re-run:

| System | Relevant pairs missed from top 10, of 414 | Displaced from B1's top 10 (queries) | Recovered |
| --- | --- | --- | --- |
| B1 lexical | 99 (78 truncated, 21 lexical_miss) | n/a | n/a |
| B2 graph, alpha 0.5 | 88 | 15 (11) | 26 |
| B3 hierarchy, beta 0.3 | 69 | 32 (24) | 62 |

Most of B1's misses are ordering misses: 78 of 99 are sections BM25 finds within depth 50 and ranks below 10. Both fusion modes recover more pairs than they displace. B3 displaces 32, over the 21-pair threshold fixed before that run, which reopened rank fusion for diagnosis only.

### GR-5: Are the displacements losses?

- **Gate:** authorized as diagnosis by the donor ADR. An exact descriptive protocol still needs owner agreement before dev payloads are read; the 2026-11-14 review withdraws the former majority-based automatic closure proposal.
- **Inputs:** the saved run named in that ADR, `tmp/release-010/benchmark/eval-donor-fusion/run-candidate/`, and the dev queries and labels admitted by its preparation pin. No binary is needed; a schema or code change does not affect this analysis.
- **Method:** for each query with a displaced pair, 24 for B3 and 11 for B2, compare the top 10 sections under B1 and under the mode:
  - count judged-relevant sections in each list;
  - list the sections that entered, each marked relevant, non-relevant or unjudged;
  - mark whether an entering section belongs to the same document as the displaced one, using the indexed path. The donor ADR's own hypothesis is that the document score promotes sibling sections.
- **Recommended protocol, still requiring approval in the donor ADR:** report per-query gains, ties and losses together with loss magnitude, relevance grades, rank changes, unjudged entering sections and same-document replacements. Treat unjudged separately. Do not automatically close Decision 2 from the fraction of queries losing relevant sections: a minority can contain most of the loss. Adjudicate the descriptive evidence explicitly; no replacement threshold or ranking change is selected here.
- **Write set:** `docs/ADR_DONOR_ADOPTION.md` and a new directory under `tmp/release-010/benchmark/`. Save the script and its hash beside the output; the earlier word-variant check did not, and lacks a reproducible script.
- **Stop:** dev queries or labels cannot be read without opening a combined or test-split file; any code, configuration or custody change is needed.
- **Limit:** the labels are model-judged, so the result guides diagnosis and supports no quality claim.

A ranking change, if one is ever proposed, needs a named cause from GR-5, a service-ADR task, exact reproduction of lexical order at weight 0 with the tie order `fuse` uses (`src/search/fusion.rs:23`), hand-computed tests, dev-only tuning under a new approved preparation, and a fresh pre-registered test split before any improvement claim. Revision 1's GR-6 card is withdrawn.

## Release And Version

- The package and lockfile version are 0.1.1; the server reports that package version. `CHANGELOG.md` records the unreleased neighbour-citation contract.
- The 0.1.0 candidate is preserved by identity: `scripts/verify_release.py` pins its commit, binary and archive hashes. Publication (REL-6) is blocked on owner decisions.
- GR-2 changes a tool response. A future 0.1.1 release needs its own source/package identity; this execution creates no release commit, archive or publication. The preserved packages, measured binary and evaluation custody were outside every write set.
- GR-0 and GR-DOC edit the working README. `scripts/verify_release.py` excludes it from `ROOT_FILES` / workspace inventory; package-member checks read the preserved archive bytes. This was checked during GR-0.

## Execution Decisions

The user's proceed instruction accepts the recommended structural graph scope and always-on citations, recorded in ADR17. Use this small tracker independently of the release tracker; target unreleased 0.1.1 without waiting for REL-6. Preserve both 0.1.0 packages and custody. GR-5 remains a separate optional track pending a decision on its rule; no evaluation payloads are opened by these tasks.

## Non-Goals

- No embedding, language model or outbound network on the retrieval path.
- No entity, concept or community graph, and no generated summaries.
- No sixth MCP tool, no write or index tool over MCP, no session or streaming transport.
- No input format beyond `.md` and `.markdown`.
- No cross-corpus graph, statistics or cache.
- No incremental index update.
- No schema change, default promotion or rerun of the consumed held-out attempt, and no qrels derived from links.
- No donor code. The GraphRAG snapshot is MIT and could be copied under the `AGENTS.md` rule, but it is Python built around model calls. The idea taken from it is that a returned context row and its source record are built together.

## Limits Of This Plan

- Focused and all-target Rust tests passed, along with formatting and Clippy; the [execution audit](../.memory-bank/audit_session-2026-11-13-graphrag-references.md) records commands, terminal receipts and limits. No real benchmark or held-out measurement ran.
- References into `src/eval/run.rs` and `tests/` come from explorers and were not all re-read by the coordinator.
- Dev figures are quoted from the donor ADR and were not recomputed.
- GR-0 and GR-2 improve what accompanies a result. Neither is evidence that retrieval finds better results.
- The ordinary-heading response-size expectation remains an estimate. A synthetic long-heading regression proves bounded truncation and transport parity. Rust Book index counting found a maximum of 132 neighbours and 2 of 1370 chunks above 100; no wider paging contract is authorized.

## References

OntoDoc documents:

- `.memory-bank/ADR_ONTODOC_SERVICE.md`, revision 17: decision items 4 (tiers), 5 (graph), 6 (retrieval systems), 8 (MCP surface and neighbour citations), 10 (citation states).
- `.memory-bank/ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md`, revision 2: provenance, accounting, topology and their gates.
- `.memory-bank/audit_session-2026-11-10-graphrag-donor-ideas.md`: evidence patterns; rejected GraphRAG components.
- `.memory-bank/audit_session-2026-11-10-ontoindex-graph-ideas.md`: graph invariants, bounded neighbours, link evidence.
- `docs/ADR_DONOR_ADOPTION.md`, revision 5, and `docs/DONOR_ADOPTION_PLAN.md`: dev miss attribution, fusion displacement, `iwe` comparison.
- `.ontocode/research-reports/similar-projects-comparison.md`: the eight similar projects.
- `.memory-bank/project_pending-tasks.md`: release state and REL-6.

OntoDoc source owners:

- `src/cli.rs`, `src/mcp.rs`: tool contracts, response bounds, tool descriptions.
- `src/cite.rs`: citations and verification.
- `src/graph.rs`: edge construction and link resolution.
- `src/store.rs`: schema, generation identity and validation.
- `src/search/ppr.rs`, `src/search/fusion.rs`, `src/search/mod.rs`: graph signal, blend, candidate pool.
- `src/tiers.rs`: tier text.
- `src/eval/run.rs`, `src/eval/sweep.rs`: saved runs, dev admission, miss attribution.

Donor references, ideas only:

- Microsoft GraphRAG 3.3.0, MIT, revision `34b2b67ae4601ad0b47bcd179fbe3f2a23aa0857`, under `tmp/graphrag/packages/graphrag/graphrag/`: `query/context_builder/source_context.py:54-79`.
- `arabold/docs-mcp-server`, MIT: `src/store/DocumentStore.ts`, weighted rank fusion with a tie order. Reported by an explorer, not re-read.
- `iwe-org/iwe`, Apache-2.0, commit `b733afe95ca6dd96ac7205e696bb920975253f18`: `crates/liwe/src/model/key_index.rs`. Its graph does not enter ranking.
- `../ontoindex`, AGPL-3.0: ideas only, never code.

## Revision History

- Review follow-up 2026-11-14: withdrew the unapproved GR-5 majority rule; [GRAPHRAG_REVIEW_PLAN.md](GRAPHRAG_REVIEW_PLAN.md) owns the typed-mention lookup repair and current validation. Original four task statuses remain closed.
- Revision 4: implemented citations for unreleased 0.1.1; recorded validation, corpus cap evidence and documentation closeout.
- Revision 3: registered four execution tasks and ADR17 under the user's proceed instruction; kept GR-5 deferred.
- Revision 2: narrowed the goal; reduced the plan to GR-0 and GR-2; deferred GR-1, GR-3 and GR-4 with reopening conditions; rewrote GR-5 around saved-run contents and moved it to a separate track; withdrew GR-6; added release and version constraints and the limits that remain after GR-2.
- Revision 1: six tasks, GR-0 to GR-5, and an undesigned GR-6.
