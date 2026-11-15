# OntoIndex ideas reviewed against OntoDoc

Date: 2026-11-14
Revision: 1
Scope: the Markdown documentation side and the MCP service layer of `../ontoindex`, compared with OntoDoc's working tree (package 0.1.1, schema 4, service ADR revision 18). Requested by: user, "review and challenge idea from ../ontoindex, provide comparation table".

## Outcome

OntoIndex has little left to give OntoDoc. Of the donor ideas examined, twelve are rejected, eight describe something OntoDoc already does as well or better, and three are worth recording as candidates. None is authorized by this report.

The three candidates, in the order recommended:

1. Validate graph edges when a generation is published and loaded.
2. Count link outcomes inside the production resolver, as a trusted CLI diagnostic.
3. Measure the per-request freshness walk on a corpus near the configured budgets.

Three stay deferred with sharper conditions: a continuation for `read`, indexing front-matter text, and joined identifier tokens. Three are dropped: a cap on query terms, stored evidence for successful links, and synthetic citation-completeness checks.

## Method and limits

Thirteen explorer runs read source and stores. Twelve were GPT-6-Luna: five on the donor (retrieval, grounding, extraction, service, and a commit-range and callers check) and seven on OntoDoc (current state, source facts, an ADR and validation lookup, two store-size measurements and two edge measurements, the first of which failed on a wrong path and produced nothing). One GPT-6.1-Sol run attacked the draft verdicts. The coordinator then re-read the OntoDoc owners behind every candidate. Claims are marked:

- **re-read**: the coordinator read the cited source in this session.
- **explorer**: reported by an explorer with a file reference, not re-read.
- **inferred**: derived by reading code, not observed by running it.

Limits:

- The coordinator ran no command. Nothing was built, tested, linted or benchmarked, and `git diff --check` was not run on this file. Explorers ran read-only `rg`, `sed`, `git` and `python3` commands that wrote nothing.
- Donor source was read by explorers from disk. The coordinator saw only four donor functions, through the OntoIndex graph of that repository. An earlier explorer reported that graph as indexed at `4a34725`; the files concerned are unchanged between that commit and HEAD (explorer).
- OntoIndex is AGPL-3.0. No donor code, prompt or data was copied, and none may be.
- No dev or held-out query, label, pool or result was opened. Store measurements read `index.json` of the two benchmark stores only.
- Both stores are benchmark corpora and may not resemble a served `docs_share` folder.
- Donor inspection is no evidence of a retrieval improvement. No row below claims one.

## Donor identity

| Item | Value | Source |
| --- | --- | --- |
| Checkout | `/opt/d1/_wrk/ontoindex`, HEAD `fdc2566896a13f351a67f5aee11ebf411c3a8c1a`, clean | explorer |
| Licence | AGPL-3.0 (root `LICENSE` and `ontoindex/LICENSE`) | explorer |
| Distance from the 2026-11-10 review | One commit after `4a34725` (2026-11-09 to 2026-11-11) | explorer |
| Changes in that commit, within the six paths examined | `ontoindex/src/mcp/local/auto-refresh.ts` only, 38 insertions and 8 deletions | explorer |

The ranking, grounding, inline-context, retrieval-policy, response-paging and evidence-ledger files all existed at `4a34725`. The donor's documentation side has therefore not changed since `.memory-bank/audit_session-2026-11-10-ontoindex-graph-ideas.md`. This review covers donor areas that audit did not read: the grounding check, the service layer and the ranker's evidence.

## Comparison

Donor cells are explorer-reported unless marked. OntoDoc cells are re-read unless marked.

| # | Aspect | OntoIndex, documentation side | OntoDoc | Verdict |
| --- | --- | --- | --- | --- |
| 1 | Lexical scoring | BM25, `k1` 1.2, `b` 0.75; heading path and excerpt scored as one field; every candidate chunk is tokenized on each query (re-read, `scoreMarkdownChunks`) | The same formula and constants; heading path and chunk text as one field; the index is built once at load | Equal. Nothing to take |
| 2 | Tokens | camelCase split, a fixed English stop-word list, at most 32 distinct query tokens, no stemming | camelCase and snake_case split keeping the whole word; no stop words, no stemming; the query is bounded at 4,096 bytes | Keep OntoDoc's. Stop words rejected, term cap dropped |
| 3 | Path prior | Scores of archive, audit, plan, research-report and release-note paths are multiplied by 0.5 | None | Reject |
| 4 | Link signal | Chunks of documents linked from the top chunk gain 0.25 × the top score. PageRank output is attached as metadata and does not change order | Personalized PageRank over `LINKS_TO` and `NEXT` changes order in the opt-in `graph` mode | OntoDoc ahead. Reject the bonus |
| 5 | Evidence for ranking | A small synthetic recall test and a repository-specific audit; no comparison against a plain lexical baseline | A registered dev and held-out protocol; the one held-out attempt is complete and left the defaults unchanged (`.memory-bank/MEMORY.md`) | OntoDoc ahead |
| 6 | Unit identity | The chunk key includes the whole-file hash, so any edit changes every key in the file; legacy section ids include the heading line number | `<path>#<heading path>~<ordinal>`; section ids do not depend on the chunk budget | OntoDoc ahead |
| 7 | Link resolution | Raw `href` and label are stored; a target is filled only for a literal `.md` suffix; no relative-path or anchor resolution | Relative paths, anchors, percent-decoding and `.html` to `.md`; a miss is a diagnostic (`src/graph.rs`, `resolve`) | OntoDoc ahead |
| 8 | Link accounting | Every link is kept as a fact | Only broken links are counted. Resolved, external and ignored links leave no count | Candidate 2 |
| 9 | Graph integrity | Id maps keep the first of two conflicting nodes silently (2026-11-10 audit) | Unique section and chunk ids are checked at publish and load. Edges are unchecked; search drops a bad edge silently and `related` refuses it | Candidate 1 |
| 10 | Checking an answer | Path and symbol citations are extracted from answer text and compared with a list of known paths. No file bytes are read; there is no span or hash (re-read in part, `checkAnswerGrounding`) | A citation carries a byte span and SHA-256; `verify` reads current bytes and returns `valid`, `moved`, `stale` or `missing` | OntoDoc ahead. Reject as a tool |
| 11 | Front matter | Owner, service, status, kind, tags and tracker state become facts | Skipped entirely: in no section, chunk or index, and not returned by `read` (`src/parse.rs`, `top_level_blocks`) | Deferred; evidence is weak |
| 12 | Semantic facts | Requirements, API specifications, mentions and concept clusters; resolution, trace and drift need the code graph | `MENTIONS` edges stay unresolved | Reject; outside the ADR |
| 13 | Large responses | The full serialized response is stored on disk for 15 minutes (64 entries, 64 MiB); pages are byte slices of the JSON text; a logical budget drops detail fields first | Each complete response is bounded at 128 KiB; `search` withholds text and keeps citations; `read` returns a prefix and has no continuation | Reject stored pages. A stateless offset stays deferred |
| 14 | Freshness | The source manifest is checked at most once per five minutes per repository; a stale index triggers a background job | `search`, `read`, `related` and `status` walk and hash the corpus on every call and report `fresh` in each response | Reject the throttle. Candidate 3 |
| 15 | Indexing | Durable jobs with deduplication and cancellation; refresh on query | Trusted CLI only; a full rebuild, about 0.11 s on the benchmark corpora (capacity audit); atomic publication | Reject |
| 16 | Admission | No global query limit or overload reply was found; a request timeout may be one hour | Fail-fast permits, 4 global and 2 per corpus, with a `busy` refusal (ADR) | OntoDoc ahead |
| 17 | Logging | Query text is written to daily JSONL files by default | No query or document text in logs (`AGENTS.md`) | Reject |
| 18 | Self-description | `diagnose`, `tool_contract` and a capability envelope naming next tools | `status`, plus the package version in the MCP server info (`src/mcp.rs`, `get_info`) | Reject |
| 19 | Models | Embeddings optional and off by default; none on this path | None anywhere on the retrieval path | Equal |

## Challenge

Three weaknesses of the donor matter more than any single feature.

**The ranker has no baseline.** The path prior and the link bonus were never compared with plain BM25 on the same queries. The prior also encodes one repository's folder habits: in a documentation corpus, a plan or a release note is often the answer. OntoDoc should not import a constant whose only support is that it exists.

**"Answer verification" checks names, never content.** The grounding check confirms that a cited path is known. A file whose text changed completely still grounds. OntoDoc's `verify` already answers the stronger question, and the donor's idea reduces, inside OntoDoc, to a client habit: carry the citation objects that `search` returned and pass them to `verify`. That needs no server change.

**Most of the documentation features lean on a code graph.** Mention resolution, requirement trace, API drift and four of the six retrieval policies resolve Markdown against code symbols. OntoDoc serves folders of documents with no code beside them, by design.

The independent challenger disagreed with four draft rejections and corrected the reasoning behind two more. Each was weighed:

| Draft rejection | Challenger's objection | Resolution |
| --- | --- | --- |
| Link bonus, "PageRank already covers it" | The mechanisms differ: the donor boosts every scored chunk of a linked document, OntoDoc walks chunk-level edges | Accepted. The rejection stands on the missing measurement |
| Stop words, "IDF already damps them" | Damping is not removal; common words still add score and length | Accepted. The rejection stands because it is a ranking change with no evidence, and one donor in the earlier comparison removed stop words after they dropped the token `GET` |
| Semantic facts need a code graph | Requirement ids and API routes can be extracted from documents alone | Partly accepted. Cross-code trace and drift stay rejected. Lookup by identifier is the tokenizer question under Deferred, and an extracted field has no named consumer |
| Stored response pages | A stateless cursor bound to the index identity is worth keeping | Agreed, and already recorded as the `read` continuation under Deferred |
| `status` suffices | `status` shows no build identity or limits | Not accepted as a candidate. The version is already in the MCP server info, and no client has asked for machine-readable limits |
| Answer grounding | A client can extract citations and call `verify` | Agreed that this is client work. OntoDoc changes nothing |

## Candidates

### 1. Validate edges at publication and load

- **Today.** `Generation::validate` checks the configuration, unique section and chunk ids, manifest order and the generation id (explorer quote of `src/store.rs:289-310`; `validate_unique_ids` re-read). It runs at publish and at load and never looks at `edges`. `SearchIndex::from_indexes` builds the PageRank graph with a `filter_map` that drops a `LINKS_TO` or `NEXT` edge whose endpoint is not a chunk id, without a trace (re-read, `src/search/mod.rs`). `App::related` returns a typed corrupt error for the same edge (re-read, `src/cli.rs`). ADR revision 17 states that rule for `related` only.
- **Measured.** Both benchmark stores pass a strict check: every `LINKS_TO` and `NEXT` edge joins two chunks, no `NEXT` edge crosses documents, every `CONTAINS` endpoint resolves, and there are no self-loops, duplicate edges, duplicate ids or collisions between id kinds (explorer; 8,817 and 7,555 edges).
- **Value.** The artifact hash is verified on load, so an invalid edge can only come from a builder defect or a hand-made store. The check turns that into a refused publication. Without it, graph mode ranks on a smaller graph and says nothing. It changes nothing for a valid generation.
- **Smallest form.** One pass in `Generation::validate`: a known kind; chunk endpoints for `LINKS_TO` and `NEXT`; the same document for `NEXT`; a `code:` target for `MENTIONS`; the three permitted `CONTAINS` shapes. Add a duplicate-document-path check beside it.
- **Gate.** A service ADR amendment that states the invariant, because a generation that loads today could be refused. No schema bump and no reindex.
- **Honest size.** No defect has been observed. This is a guard, and the one remaining part of the 2026-11-10 audit's first idea.

### 2. Count link outcomes in the production resolver

- **Today.** `resolve` in `src/graph.rs` returns one of three results: a target chunk, a problem, or `Ignored` (re-read). `Ignored` covers four different cases: a URL with a scheme, a non-Markdown extension, an `.html` link with no sibling `.md`, and a target document that has no chunk. `build` also drops a link to its own chunk and a link that lies in no chunk. Only problems are counted.
- **Why.** The DC-3 census tried to count ignored links with a separate regex scanner. It failed calibration (0 of 17 and 9 of 10 stored diagnostics reproduced), so the deferred decision on extensionless and directory links still has no number (`.memory-bank/audit_session-2026-11-14-donor-review.md`).
- **Smallest form.** Count each outcome where the resolver already decides it. Expose the counts through a trusted CLI diagnostic, which the proposed evidence ADR already lists as CLI topology diagnostics. Storing the counts in the generation would be a schema bump; whether they can be recomputed from the stored document text without one was not traced (inferred as likely, since `index.json` holds each document's text).
- **Gate.** The owner wants to decide the deferred link-resolution candidate. It is the only consumer.

### 3. Measure the freshness walk near the budgets

- **Today.** Four of the five tools walk the corpus and hash every admitted file on each call. On the OntoIndex benchmark corpus they take about 18 ms at p50, against 0.24 ms for `verify`, which does not walk. In synchronized bursts of four calls, 40 of 80 calls to each walking tool were refused as `busy`, against 9 of 80 for `verify` (`.memory-bank/audit_session-2026-11-13-capacity.md`).
- **Gap.** That corpus has 325 Markdown candidates and 3.6 MB. The budgets allow 128 MiB and 100,000 probes per walk. Nothing has been measured near either limit: the "near-cap" corpus in the capacity audit is one 2 MiB file, which approaches the per-file limit only. The ADR makes no wall-time promise (explorer).
- **Why it matters.** Admission is fail-fast with two permits per corpus, so the longer a walk holds a permit, the more calls are refused.
- **Form.** A synthetic corpus near the aggregate budgets and the existing capacity script. No code change.
- **What it must not become.** The donor's five-minute throttle would report results as fresh that are not. The ADR requires no mtime cache (explorer), and `AGENTS.md` forbids weakening freshness to save per-request work. Any change to the walk is an ADR decision made after the number exists.

## Deferred, with corrected conditions

### `read` continuation

`App::read` cuts text at 65,536 bytes and `ReadResponse` shrinks by halving its text (re-read, `src/cli.rs`). The MCP path measures the result as rmcp carries it, which holds the JSON twice (explorer, recorded in `docs/DONOR_CANDIDATES_PLAN.md`), against the same 128 KiB (re-read, `src/mcp.rs`, `structured`). A full 65,536-byte prefix therefore cannot fit, and the text is halved. Over MCP, `read` on anything larger than roughly 60 KiB, less for text with many quotes and newlines, probably returns half of the prefix, about 32 KiB. This is inferred from the code and was not run.

Measured sizes (explorer, two matching runs): the OntoIndex corpus has one section above 64 KiB (`ontoindex/CHANGELOG.md#changelog`, 107,222 bytes) and four above 32 KiB; the Rust Book has two above 32 KiB and none above 64 KiB. The largest chunk is 33,444 bytes, which should fit.

The plan's reopen condition reads "chunks above 64 KiB in a served corpus". It should name sections as well, and it should use the size `read` delivers over MCP, established by one test. A workaround exists for sections: read `<section id>~0` and follow `NEXT` through `related`. No tool lists a section's chunks, and the server instructions do not describe the workaround.

If reopened, the smallest form is one `offset` field on the request and a `next_offset` computed after the final shrink. The response already carries `generation_id`, so a client can detect an index change by comparing two responses, with no server state. That keeps the donor's good idea, a cursor that an index change invalidates, and drops its stored pages. It is a request field, so it needs an ADR amendment.

### Front-matter text

OntoDoc cannot find a document by text that appears only in its front matter. In the two stores, 23 of 118 OntoIndex-corpus documents have front matter: `description:` in 22, `title:` in 1 and `tags:` in 1. The Rust Book has none (explorer, two matching runs). No document was found with a front-matter title and no level-1 heading (explorer, one run).

Front matter lies outside every section, so a hit on it has no span to cite. Making it citable changes section spans and hashes, which is a schema bump and a ranking change. Reopen when a real `docs_share` folder shows titles that exist only in front matter.

### Joined identifier tokens

`REQ-12` becomes `req` and `12`; `--max-items` becomes `max` and `items`; `v1.95.0` becomes `v1`, `95` and `0` (re-read, `src/search/tokenize.rs`). A query for the identifier still matches through its parts, so the loss is precision, and no miss has been attributed to it. The OntoIndex corpus has 40 distinct identifiers of the first shape in 23 documents and 37 distinct multi-word flags in 31 documents; the Rust Book has 2 and 3 (explorer, one run, approximate patterns).

Emitting the joined token is tokenizer version 2: every generation id changes, every store and the dev preparation are rebuilt, and longer documents shift BM25 length normalization for unrelated queries. It belongs with the stemming and heading-weight round that `docs/DONOR_CANDIDATES_PLAN.md` already defers behind a fresh test split.

## Dropped

- **Cap on distinct query terms.** The 4,096-byte query limit already bounds the terms at about 2,000 (inferred from the tokenizer) and with them the scoring work. A second limit adds a refusal with no measured slow query behind it.
- **Stored evidence for successful links** (label, destination, span). No consumer is named, as in the 2026-11-10 audit.
- **Synthetic citation-completeness checks** (donor `passive-retrieval-quality.ts`). The challenger named this as a missed idea. One explorer read it, and no gap in OntoDoc's own citation tests was shown.

## Rejected

| Donor idea | Reason |
| --- | --- |
| Path prior of 0.5 | Corpus-specific and unmeasured; plans and audits can be the answer |
| Link bonus from the top chunk | Unmeasured; a ranking change needs dev evidence and a registration |
| English stop-word list | Ranking change with no evidence; English only |
| Concept clusters and knowledge report | Generic labels merge unrelated material; the report only flags the damage afterwards |
| Mention resolution, requirement trace, API drift, retrieval policies | Need a code graph; outside the ADR |
| Server-stored response pages | Server state between calls conflicts with the stateless HTTP listener; a byte slice of JSON is not parseable alone |
| Throttled freshness | Weakens the accepted freshness contract |
| Durable jobs and refresh on query | Indexing stays in the trusted CLI |
| Query logging | Forbidden by `AGENTS.md` |
| `diagnose`, `tool_contract`, capability envelope | Five tools by ADR; the version is already exposed |
| Incremental indexing | A full rebuild takes about 0.11 s on the benchmark corpora; reopen on a measured need |
| Answer grounding as a tool | Weaker than `verify`; a sixth tool |

## Status of the 2026-11-10 ideas

| 2026-11-10 idea | State now | Source |
| --- | --- | --- |
| 1. Validate graph identities and invariants | Unique ids are checked and empty id components use `@empty` (schema 2). Edges are unchecked | re-read; `AGENTS.md` |
| 2. Bound construction and neighbour retrieval | `related` counts incident edges, lists at most 100 and looks up only the listed endpoints; `chunk_at` uses a binary search when chunks are ordered | re-read |
| 3. Successful-link evidence | Open, no consumer | |
| 4. Coverage separate from identity and freshness | `status` reports `link_diagnostic_count`; resolved and ignored links are uncounted | re-read |
| 5. Whole-corpus generations, incremental deferred | Unchanged | |
| 6. Semantic edges as a later experiment | Unchanged | |

## Corrections

- **`docs/DONOR_CANDIDATES_PLAN.md`, Deferred, `read` continuation.** The reopen condition names chunks and 64 KiB. Sections are the affected unit, and the MCP limit is probably near 32 KiB for large reads; see above. The plan is closed and was not edited.
- **Stored donor pages and index changes.** The page validation checks expiry, tool, request hash and a token over the stored result (re-read, `validateStored`). The module also extracts repository and graph-generation identity from a response (explorer; `extractIdentity` re-read in part). Whether a continuation call compares that identity was not established. The rejection does not depend on it.
- **One explorer result was discarded.** A second result arrived under the name of the store-measurement explorer with different totals (for example 2,223 sections where the first reported 1,072). A fresh measurement against the same `index.json` hashes reproduced the first result, and each store holds exactly one generation. The second result is not used.

## Evidence

Store identities (explorer, three runs agree): Rust Book generation `375917fc…`, `index.json` SHA-256 `b88c3f33…`, 112 documents, 547 sections, 1,370 chunks, 8,817 edges; OntoIndex corpus generation `4e546b16…`, SHA-256 `a5c77c58…`, 118 documents, 1,072 sections, 1,275 chunks, 7,555 edges.

Coordinator re-reads in OntoDoc:

- `src/search/mod.rs`: `SearchIndex::from_indexes`, `SearchIndex::search`.
- `src/store.rs`: `validate_unique_ids`, `StoreError`, `generation_id`.
- `src/graph.rs`: module comment, `build`, `structure_edges`, `chunk_at`, `resolve`.
- `src/search/tokenize.rs`: the whole file.
- `src/parse.rs`: section extents, `MARKDOWN_OPTIONS`, `top_level_blocks`.
- `src/cli.rs`: `App::read`, `App::related`, `App::verify_index`, `App::status`, `Bounded` and its five implementations, `bound`.
- `src/mcp.rs`: `structured`, `get_info`.
- `.memory-bank/ADR_ONTODOC_SERVICE.md`: decision items 5 to 7 and the work-budget table.
- `.memory-bank/audit_session-2026-11-13-capacity.md`, `audit_session-2026-11-14-donor-review.md`, `audit_session-2026-11-10-ontoindex-graph-ideas.md`, `ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md` (first part), `docs/DONOR_CANDIDATES_PLAN.md`.

Coordinator re-reads in the donor, through the OntoIndex graph: the opening of `scoreMarkdownChunks` and of `checkAnswerGrounding`, all of `validateStored`, the opening of `extractIdentity`.

Explorer-reported donor files, under `ontoindex/src/`: `core/ingestion/enrichment/markdown-query-ranking.ts`, `markdown-document-ppr.ts`, `markdown-answer-grounding.ts`, `markdown-answer-verify.ts`, `docs-inline-context.ts`, `docs-retrieval-policies.ts`, `markdown-sidecar-producer.ts`, `markdown-document-facts.ts`, `markdown-concept-clusters.ts`, `passive-retrieval-quality.ts`; `core/ingestion/markdown-processor.ts`; `mcp/shared/response-pages.ts`, `response-limits.ts`, `tool-registry.ts`; `mcp/local/local-backend.ts`, `auto-refresh.ts`; `core/runtime/evidence-read-ledger.ts`; `cli/docs.ts`; the query-log module.

## Not verified

- Any donor runtime behaviour, speed or test result.
- The size `read` delivers over MCP for a section above 62 KiB.
- Freshness-walk time on any corpus larger than the two benchmark corpora.
- Whether a real `docs_share` folder has front-matter-only titles, identifier-heavy text or sections above 64 KiB.
- Whether link outcome counts can be recomputed from a stored generation without a schema change.
- The identifier and flag counts, which come from one run with approximate patterns.
- Line numbers in donor files, which are the explorers'.
