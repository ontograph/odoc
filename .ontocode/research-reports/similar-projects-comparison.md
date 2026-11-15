# Similar projects compared with OntoDoc

Date: 2026-11-14
Revision: 4
Scope: the eight shallow clones under `tmp/similar/` against OntoDoc's working tree (package version 0.1.1, unreleased, commit `ee511dc` plus 53 changed or untracked paths). Requested by: user, "compare again all from ./tmp/similar with ontodoc".

## Method and limits

The donor clones are at the same commits as in revision 3. What changed is OntoDoc, which gained two rounds of work since that revision, and the depth of the donor reading: this round asked every donor the same eleven questions about updates, load integrity, large reads, Markdown handling, tokens, fusion, limits, tools and evaluation.

Eight GPT-6-Luna explorer runs did the reading: OntoDoc's current state, `arabold`, `iwe`, `xerj` ingestion, `xerj` storage, the five small projects, and two fact checks. Explorers ran read-only commands (`rg`, `git`, `sed`). The coordinator ran no command and read files by path and through OntoIndex. Claims are marked:

- **re-read**: the coordinator read the cited file while writing this revision.
- **carried**: the coordinator read it earlier in this review, and the result reached this text through notes. It was not read again.
- **explorer**: reported by an explorer with a file and line reference.
- **claimed**: stated only by the donor's README or docs.

Limits:

- Nothing was built, run or benchmarked. Donor numbers are the donors' own checked-in results. OntoDoc numbers come from its own audits. No speed or quality ranking between projects follows from this report.
- `xerj` has about 3,350 files. Only parts of `xerj-autoindex`, `xerj-storage`, `xerj-fts`, `xerj-mcp`, `xerj-engine/src/index.rs` and `graph.rs` were read. Its rows cover the files under [Evidence](#evidence) and nothing else.
- The OntoIndex graph of OntoDoc is indexed at `ee511dc` and does not include the uncommitted edits. OntoDoc facts were taken from files on disk.
- One explorer reported a different hash for `git rev-parse HEAD`. The file `.git/refs/heads/master` (re-read) and a second explorer's `git log` both give `ee511dc`, with no later commit. This report uses `ee511dc`. That explorer's other answers were checked where a conclusion rests on them: the `read` cap was re-read in `App::read`, its versions agree with `README.md`, `CHANGELOG.md` and the tokenizer module comment, and its quote of `resolve` matches the earlier coordinator read. Its gate list and its search for a `title` key stand on its word alone.
- No dev or held-out query, label or result was opened. No evaluation ran.
- "Not found" in a donor row means the explorer's searches found nothing. It is not proof of absence.

## Main findings

1. **Two of the six candidates from revision 3 are implemented and one is measured.** `search` now keeps the citation when text does not fit, `related` reports how many neighbours it left out, and a calibrated census counts what the link resolver does with every link in the two benchmark corpora. The three ranking candidates stay deferred behind a fresh test split.
2. **In the code that was read, only OntoDoc checks that the records of a stored index are consistent with each other, and only OntoDoc documents a recovery that a test covers.** `xerj` checks segment checksums and structure; `quillrag` checks a schema number; the rest check nothing that was found.
3. **Freshness is where OntoDoc pays.** It rehashes the corpus on every `search`, `read`, `related` and `status`. Measured `status` p95 was 23 ms for 16 files of about 2 MiB and 325 ms for 50,000 files of 1 KiB, and a corpus above the 128 MiB read budget is refused. No donor compares content at query time; those that signal a stale index use modification times, which OntoDoc's contract forbids as a shortcut. No donor shows a way to keep content-hash freshness for less.
4. **Three gaps remain where donors do more without a model:** continuing a read past the size cap (`arabold`), reading a title from YAML front matter (`arabold`, `iwe`, `xerj`), and resolving extensionless links (`iwe`). Each needs an ADR decision. They are listed under [Candidates](#candidates) with their gates.
5. **No donor resolves a link to a directory.** That is the largest class of links OntoDoc ignores in the OntoIndex corpus: 52 link events to 18 destinations.
6. **Revision 3 needed eight corrections,** listed under [Corrections](#corrections). The largest: `iwe` does order by link count when there is no text query, `arabold` detects a changed local file by modification time alone, and its authentication is off by default.

## OntoDoc today

| Aspect | State | Source |
| --- | --- | --- |
| Package | Rust 1.95, edition 2024, version 0.1.1 unreleased, MIT; schema 4, tokenizer 1; service ADR revision 19 | `Cargo.toml:3`, `src/store.rs:38-39` (explorer); `CHANGELOG.md` (re-read) |
| Input | `.md` and `.markdown`; 2 MiB per file, 100,000 files, depth 64; symlinks never followed; hidden entries skipped; `.gitignore` and `.ontodocignore` honoured | `README.md` (re-read); `src/corpus.rs` (carried) |
| Unit | Heading section, packed into chunks at block boundaries; 1,200 bytes, no overlap | `src/chunk.rs` (carried) |
| Front matter | Excluded from sections, chunks and the BM25 input; no `title:` key is read anywhere in `src/` | `src/parse.rs` module comment (re-read); `rg` over `src/` (explorer) |
| Ids | `<path>#<heading path>~<ordinal>`; section ids do not depend on the chunk budget | `src/parse.rs` module comment (re-read) |
| Store | One generation, written to a temp directory, renamed, then `CURRENT` swapped; `index` is always a full rebuild | `README.md` (re-read); `src/store.rs` (carried) |
| Load check | Publication and every load refuse duplicate section or chunk ids, duplicate document paths, unknown edge kinds, wrong endpoint kinds and a `NEXT` edge that crosses documents | `README.md`, `CHANGELOG.md` (re-read) |
| Recovery | Stop the service, remove only `.ontodoc/generations/<id>`, run `index`, then `status`; covered by a regression test | `README.md` (re-read) |
| Lexical | BM25, `k1` 1.2, `b` 0.75; heading path and chunk text as one field | `src/search/bm25.rs` (carried) |
| Tokens | Runs of alphanumerics and `_`; a word that splits at `_` or camelCase emits the whole word and then its parts; no stemming, no stop words | `src/search/tokenize.rs` (re-read) |
| Second signal | Personalized PageRank over `LINKS_TO` and `NEXT` edges, or BM25 of the document pseudo-document; opt-in, default weight 0.3 | `src/search/` (carried) |
| Citation | Generation id, path, heading path, byte and line span, SHA-256 of the span | `src/cite.rs` (carried) |
| Verification | `valid`, `moved`, `stale`, `missing`, against current bytes; only citations of the active generation are admitted | `README.md` (re-read) |
| `search` text | When the budget or the response bound cannot hold a text, the hit stays with `tier: null` and empty text; texts are tried in rank order; text above 64 KiB is withheld | `README.md`, `CHANGELOG.md` (re-read) |
| `read` | Request has `id` only; text is cut at 64 KiB (`MAX_RESPONSE_BYTES / 2`) with `truncated`; whether the MCP frame bound shortens it further has not been run; no continuation | `App::read` in `src/cli.rs` (re-read); `ReadRequest` in `src/mcp.rs:33-37` (explorer) |
| `related` | One hop, at most 100 neighbours in stored edge order, with citations; `omitted` counts the incident edges left out | `App::related` in `src/cli.rs` (re-read) |
| Links | External when the destination contains `://` or starts with `mailto:`; `.md` and `.markdown` resolve relative to the linking file; `.html` and `.htm` resolve to a sibling `.md`; every other destination is ignored; path and fragment are percent-decoded; a fragment resolves to the section's first chunk; no basename fallback | `resolve` in `src/graph.rs:174-228` (explorer, quoted source) |
| Bounds | Complete response frame at most 128 KiB; `top_k` 1 to 50; 4 global and 2 per-corpus work permits | `README.md` (re-read) |
| Freshness | Every `search`, `read`, `related` and `status` rehashes the corpus, up to 128 MiB read per call; `verify` reads only the cited files | `README.md` (re-read) |
| MCP | Five read-only tools over stdio and loopback HTTP; no authentication in process | `README.md` (re-read) |
| Evaluation gates | Query review, qrels, leakage, floors, pool, pins, judged@10, benchmark; no check against a stored dev baseline | `src/eval/gates.rs` (explorer) |

Measured since revision 3, from OntoDoc's own audits (re-read):

| Corpus | Files | Source bytes | `status` p95 | `search` p95 | `verify` p95 |
| --- | ---: | ---: | ---: | ---: | ---: |
| B16 | 16 | 33,538,048 | 22.9 ms | 31.4 ms | 3.6 ms |
| B60 | 60 | 125,767,680 | 77.8 ms | 117.9 ms | 2.1 ms |
| E50 | 50,000 | 51,200,000 | 325.1 ms | 305.0 ms | 3.2 ms |

Each figure is 20 sequential loopback HTTP calls on synthetic repetitive text with warm caches. The build predates the new load check. No latency target exists. A 65,059-file corpus was refused at indexing because its path bytes exceeded 16 MiB, and a 66-file corpus of 138,344,448 bytes was refused by `status` and `search` while `verify` kept working. Source: `.memory-bank/audit_session-2026-11-14-ontoindex-ideas-capacity.md`.

Link census of the two benchmark corpora, per link event (`docs/ADR_DONOR_ADOPTION.md`, Decision 3, re-read):

| Class | Rust Book | OntoIndex corpus |
| --- | ---: | ---: |
| Linked | 276 | 44 |
| Missing anchor | 17 | 0 |
| Missing target | 0 | 10 |
| External | 103 | 175 |
| Ignored `.html` | 33 | 0 |
| Directory | 0 | 52 (18 destinations) |
| No extension | 0 | 10 (2 destinations) |
| Other extension | 0 | 3 (2 destinations) |

The census does not say how many new edges a rule for an ignored class would produce.

## Comparison

### Identity and licence

| Project | Commit | Stack | Licence found | Copy status under `AGENTS.md` |
| --- | --- | --- | --- | --- |
| `arabold/docs-mcp-server` | `d2bdb2f8` | TypeScript, Node 22 | MIT at root and in `package.json` | Eligible |
| `iwe-org/iwe` | `b733afe9` | Rust 2021, five crates | Apache-2.0 at root and in three crates | Eligible |
| `xerj-org/xerj` | `50e4ffde` | Rust, 20 workspace members | Apache-2.0 with a root `NOTICE`; README says the Query-1 crates in release binaries are proprietary, and none was found in the tree | Eligible, with `NOTICE` attribution |
| `Ayush-yadav11/quillrag` | `59af4706` | Rust 2021 | MIT; bundled OCR models are CC BY-SA 4.0 (`assets/ocr/NOTICE.md`) | Code eligible; models and weights are data and are not |
| `Michaol/RustRAG` | `885e23c4` | Rust 2024 | MIT OR Apache-2.0, both files present | Eligible |
| `Govcraft/rust-docs-mcp-server` | `7ee11e78` | Rust 2024 | MIT text with a placeholder copyright holder; no `license` field in `Cargo.toml` | Unclear; idea source until the owner checks |
| `MicrosoftDocs/mcp` | `f0a9478a` | TypeScript CLI | `LICENSE-CODE` is MIT; `LICENSE` is CC BY 4.0 for content | CLI code eligible; skills and docs are not |
| `AayushAlokGit/DocumentationRetrievalMCPServer` | `12db55c8` | Python | No licence file anywhere in the tree | Idea source only |

Commits are from `git rev-parse` in each clone (explorer). Licence rows are explorer-reported. No source file with a licence header was found in the files the explorers inspected. Nothing was copied, and no candidate below needs a copy.

### Corpus, unit and updates

| Project | Input | Unit and size | Ids | How a changed file is detected | Visibility during an update |
| --- | --- | --- | --- | --- | --- |
| OntoDoc | Local Markdown | Heading section, then 1,200-byte chunks, no overlap | Readable, path-based, stable across reindex | Full rebuild on `index`; SHA-256 of every admitted file on each call | Atomic pointer swap; a partial generation is never visible |
| `arabold` | Web crawl and local files, several formats; 10 MiB cap for PDF and Office files | H1 to H3 sections; minimum 500, preferred 1,500, maximum 5,000 characters | Autoincrement rows; a refresh re-inserts a page's chunks, so ids change | Local files: an ETag that is the MD5 of the modification time; no content hash | One SQLite transaction per page, so a crawl is visible page by page |
| `iwe` | Local Markdown or Djot; no per-file or file-count cap found | Whole document with a tree of sections and blocks | Document key is its path; node ids are renumbered on every load | File watcher | In-memory graph; no persisted index |
| `xerj` | Any folder; whole-file parsing capped at 64 MiB (`MAX_WHOLE_FILE`), and a comment in the same file names a 16 MiB read cap for plain text | Markdown and prose: sections of about 2,048 characters with 200 overlap, at most 4,096 per file | Hash of dataset, content key (first 64 KiB plus size) and locator; the path is not in the id | Full-content digest per file when it reindexes; unchanged files skipped; deletes reconciled; optional `--watch` | Temp file, fsync, rename, directory fsync, then an atomic swap |
| `quillrag` | Local text and code, optional PDF and OCR; 8 MiB; follows symlinks | Paragraphs of about 1,000 characters, 120 overlap | Per-document ordinal | Size and modification time first; when those differ, content hash and modification time | `redb` commit, then a Tantivy rebuild; two stores with no common commit |
| `RustRAG` | Local documents and Tree-sitter code | Paragraph, then sentence | Autoincrement | Modification time | Not established |
| `Govcraft` | rustdoc HTML of one crate, generated per run | One page section | None | Not applicable | Not applicable |
| `DocumentationRetrievalMCPServer` | `.md`, `.txt`, `.docx`, `.pptx` (claimed) | Character windows, 4,000 with 200 overlap | Stored ids | Path, size and modification time in a JSON tracker | Not established |
| `MicrosoftDocs/mcp` | Hosted service; the repository holds a client | Not in the repository | n/a | n/a | n/a |

Re-read: `DEFAULT_CONFIG` in `arabold`'s `src/utils/config.ts`, `FileFetcher.ts`, `xerj`'s `extract/mod.rs` constants and `ids.rs`, and the change loop of `index_directory_with` in `quillrag`'s `src/indexer.rs`. The rest is explorer-reported or carried from revision 3.

An id that leaves out the path, as in `xerj`, survives a rename and changes on every edit of the first 64 KiB. OntoDoc's id does the reverse. For citations that a person reads and a later `verify` checks, the path-based id is the better fit.

### Markdown handling

| Project | YAML front matter | Headings | Links between documents |
| --- | --- | --- | --- |
| OntoDoc | Excluded from every indexed unit; title never read | Define the section; heading path is indexed with the text and returned in the citation | `LINKS_TO` edges to a chunk; fragment to a section's first chunk; `.html` to a sibling `.md`; directory, extensionless and other destinations ignored |
| `arabold` | `title` read, falling back to the first H1; the block stays in the indexed text (explorer) | H1 to H3 split; heading path kept | Used for crawl discovery only |
| `iwe` | Parsed into a mapping, usable in filters, sort and projection; a configurable key supplies the title (explorer; `sort_by_frontmatter` re-read) | Tree of sections | Reference and inclusion edges between documents; an extensionless destination is a document link; a fragment is kept on the link but the edge ends at the document |
| `xerj` | `title:` taken from the block when it parses as a mapping, else the first non-blank line; the block stays in the body | Stored as a `headings` array per section (explorer) | Nine detectors; a Markdown link is tried relative to the directory, then to the root; a miss is counted |
| `RustRAG` | A line-oriented parser extracts known fields; whether a title is among them was not established (explorer) | Not established | Not established |
| `DocumentationRetrievalMCPServer` | Parsed with `python-frontmatter`; raw text on a parse failure; use of a title not established (explorer) | Used as keywords (explorer) | None found |
| `MicrosoftDocs/mcp` CLI | n/a | `fetch --section` cuts one ATX heading section with its children | n/a |
| `quillrag`, `Govcraft` | None found | Not used | None |

Re-read: `is_document_url` and `has_uri_scheme` in `iwe`'s `crates/liwe/src/model.rs`, `prose_title` in `xerj`'s `extract/txt.rs`, the head of `resolve_local` in `xerj`'s `detect/mod.rs`, and `cli/src/utils/markdown.ts` in `MicrosoftDocs/mcp`.

Link classification compared:

| Destination | OntoDoc | `iwe` | `xerj` |
| --- | --- | --- | --- |
| `guide.md` | Resolved relative to the linking file | Document link | Directory-relative, then root-relative |
| `guide` (no extension) | Ignored | Document link | Same lookup as any path; no special case |
| `dir/` | Ignored | Not a document link | No special case found |
| `page.html` | Sibling `page.md` when it exists | Not a document link | No special case found |
| `#section` | Section in the same document | Not a document link | Empty target |
| `tel:123`, `MailTo:x` | Internal path, then ignored for lack of a Markdown extension | External: any scheme of two or more characters | External: any `:` before the first `/` |
| `x.md?next=http://y` | External, because the `://` test runs before the query is cut | No scheme by its test, so not external | Not established |
| Percent-encoded path | Decoded | Decoded | Not decoded, by a stated limit |

### Ranking

| Project | Lexical | Analysis | Second signal | Fusion | Ties |
| --- | --- | --- | --- | --- | --- |
| OntoDoc | Own BM25, one field | No stemming, no stop words, identifier split that keeps the whole word | Link-graph PPR, or document BM25 | Weighted min-max blend over the top 50 of each leg | Fused score, BM25, id |
| `arabold` | FTS5 `bm25` with column weights content 10, title 1, url 5, path 1 | `porter unicode61`; 33 stop words dropped from unquoted terms; exact phrase OR each term | Vector, from a provider | Weighted reciprocal rank, `w/(60 + rank)`, weights 1 and 1 | Score, URL, chunk position |
| `iwe` | `bm25` crate over title and body of each document | Stemming and stop words for one configured language | Fuzzy match over key and title | Reciprocal rank, unweighted | Score, then key |
| `xerj` | BM25 with OntoDoc's formula and constants; field boosts | `standard` by default: Unicode words, lowercase; also `english`, `stemmer`, `code` | HNSW over feature-hash or neural vectors | Reciprocal rank, or a min-max blend | Score, sequence number, id |
| `quillrag` | Tantivy BM25 | English stop list on the query; no stemming found | MiniLM cosine, linear scan | Reciprocal rank, `1/(60 + rank)` | Fused score only |
| `RustRAG` | None; vector search | n/a | `sqlite-vec` | None | Distance |
| `Govcraft` | None | n/a | OpenAI embedding; the best document goes to an LLM | None | First seen |
| `DocumentationRetrievalMCPServer` | None active | n/a | ChromaDB vectors | None | None found |

This table is carried from revision 3, where the `arabold`, `iwe` and `xerj` analyzer rows were re-read. `arabold`'s search weights and over-fetch factor were re-read again in `DEFAULT_CONFIG`.

Graph signals. OntoDoc is the only project that uses a link-graph score to rank the answers to a text query. `iwe` does use link structure when there is no text query and no sort: `order_by_rank` in `crates/diwe/src/find.rs` (re-read) orders documents by inbound reference and inclusion edges, then by key. With a text query it calls `search_query::ranked` and uses no graph score. No graph score was found in the `xerj` search path that was read.

How identifiers become tokens:

| Input | OntoDoc (re-read) | `arabold`, FTS5 (explorer) | `xerj` `code` analyzer (explorer) |
| --- | --- | --- | --- |
| `REQ-12` | `req`, `12` | `req`, `12` | Inferred, not tested |
| `--max-items` | `max`, `items` | Not established | Not established |
| `foo_bar` | `foo_bar`, `foo`, `bar` | `foo`, `bar` | Whole token and its parts; a test covers `id_to_fieldnorm` |
| `HttpServer` | `httpserver`, `http`, `server` | One token | Whole token and camelCase parts; a test covers `getHTTPResponse` |

OntoDoc keeps a joined token for `_` and camelCase and none for hyphenated identifiers such as `REQ-12`.

### References and freshness

| Project | Locator in a hit | Content hash | Check against current source | Freshness signal | Cost bound of the signal |
| --- | --- | --- | --- | --- | --- |
| OntoDoc | Path, heading path, byte and line span | SHA-256 of the span | `verify`, four states | Corpus rehashed per call; per-result `stale` | 128 MiB read per call; refused above that |
| `arabold` | Page URL and title | None in output | None found | None in a result; refresh compares modification time | None at query time |
| `iwe` | Document key, optional fragment | None | None found | File watcher reload | n/a |
| `xerj` | `_index`, `_id`, path, line | None found | None per passage | `index_stale` hint for the whole corpus | 4,096 stats and 100 ms; 2 s grace; silent when a cap is hit without a proven change |
| `quillrag` | Path, chunk index, optional page | None | None found | None | n/a |
| `RustRAG` | Document, position, chunk id | None | None found | None | n/a |
| `Govcraft` | None; the reply is generated text | None | None | None | n/a |
| `DocumentationRetrievalMCPServer` | Stored id and metadata | None | None found | None | n/a |

Re-read: the module comment and constants of `xerj`'s `engine/crates/xerj-autoindex/src/staleness.rs`, and `arabold`'s `FileFetcher.ts`.

`xerj`'s hint and OntoDoc's rehash answer different questions. The hint says "a file under the indexed folder is newer than the last run", from modification times, and gives up quietly when the folder is large or the disk slow. OntoDoc's answer is exact for every admitted file and has a hard ceiling in place of a quiet failure. A same-size edit that keeps its modification time escapes every donor's query-time signal and is caught by OntoDoc. `xerj` would catch it at its next reindex, since it compares content digests there; `quillrag`, `RustRAG`, `arabold` and the Python project would not.

### Integrity of the stored index

| Project | Checked when the store is opened | On failure | Recovery |
| --- | --- | --- | --- |
| OntoDoc | Schema and tokenizer version, artifact hashes, unique ids and document paths, edge kinds and endpoints, generation id | Typed `corrupt` error; the corpus is unavailable | Documented and tested: remove the generation directory, reindex |
| `xerj` | Segment size, magic, whole-file CRC, header and section table; snapshot parse; a data-directory marker newer than the binary (explorer) | The open error propagates | Restore a backup or a `.tmp.*` sibling |
| `quillrag` | Schema number equality, checked at the start of an index pass; a check on the search path was not established | `index` stops with "incompatible store schema; rebuild into a new data directory" (re-read) | Rebuild elsewhere |
| `arabold` | Migrations only; no refusal of a newer schema found (explorer) | Not established | None found |
| `RustRAG` | None found; tables are created when absent | Not established | None found |
| `DocumentationRetrievalMCPServer` | A tracker file that fails to parse is treated as empty | Silent | Everything is processed again |
| `MicrosoftDocs/mcp` CLI | A cache with another schema number or malformed JSON is treated as empty | Silent | The cache refills |
| `iwe` | No persisted index | n/a | n/a |

`xerj` proves the bytes are the bytes that were written. OntoDoc also proves that the records make sense together. Neither check covers the other's ground: no posting-order or id audit was established in `xerj`, and OntoDoc relies on hashes without a per-segment CRC.

### MCP surface and bounds

| Project | Tools | Writes through MCP | Transport and default bind | Authentication | Output bound | Reading a unit larger than the bound |
| --- | --- | --- | --- | --- | --- | --- |
| OntoDoc | 5 | None | stdio; loopback stateless HTTP | None in process; an authenticating ingress is required | 128 KiB complete frame; `truncated`, `tier: null`, `omitted` | A prefix of at most 64 KiB with `truncated`; no continuation |
| `arabold` | Search, read, list and version tools, plus scrape, refresh and job tools | Yes, unless read-only is set | stdio; stateless Streamable HTTP; legacy SSE when authentication is off; `127.0.0.1:6280` | Off by default; when on it covers the MCP endpoint only (explorer) | Search limit 1 to 100; no byte cap found; search assembly adds a parent, siblings and children with no size bound | `read_page` continues by `startChunk` and returns `totalChunks` and `nextChunk`; the first chunk is always included |
| `iwe` | 14 | Yes, 9 tools | stdio; Streamable HTTP on `127.0.0.1:8000` | None found | Optional document and token caps, unlimited when omitted; a truncation note | Token caps with `emitted`, `matched` and a hint |
| `xerj` | 24 | Yes | stdio proxy to the HTTP engine | TLS and API key on by default | 8,000 UAX #29 words by default; accounting block | No document-fetch tool found; `xerj_map` pages by offset and limit |
| `quillrag` | `rag_search` and `rag_index` found this round; revision 3 counted 4 | Yes: index, and clear by revision 3 | stdio | None found | `top_k` 1 to 25; no byte bound found | None |
| `RustRAG` | 7 | Yes, 4 tools | stdio; Streamable HTTP on `127.0.0.1` | None found | None found | None found |
| `Govcraft` | 1 | No; each call uses a remote embedding and chat API | stdio (claimed) | None | None found | None |
| `DocumentationRetrievalMCPServer` | 5 active `chromadb_*` tools (revision 3) | None in the active list (revision 3) | stdio | None found | `max_content_length`; one handler appends a marker | No paging |
| `MicrosoftDocs/mcp` | 3 hosted tools | n/a | Streamable HTTP (claimed) | No key (claimed) | `maxTokenBudget` (claimed) | The CLI cuts at `--max-chars` with no marker |

Re-read: `getPageContent` in `arabold`'s `src/store/DocumentManagementService.ts`, its `auth.enabled: false`, host and port defaults, the assembly limits in `DEFAULT_CONFIG`, the head of `xerj`'s `token_budget.rs`, and `truncateOutput` in the Microsoft CLI.

### Quality evidence

| Project | What exists | Labels | Split or registration | Run automatically |
| --- | --- | --- | --- | --- |
| OntoDoc | One registered held-out run: 165 answerable and 55 off-topic queries, section-level labels, bootstrap intervals; H2 +0.0329 and H5 +0.0327 nDCG@10 against lexical | Model-judged, so exploratory | Dev and test split; write-once registration | No; the test split runs once |
| `arabold` | A passage-level harness that fails on a drop above 0.02 in MRR, Hit@5 or nDCG@10 against a hash-bound baseline; a page-level harness with LLM-judged rubrics | The passage set is model-written and marked `status: draft` | None found | The page-level harness runs weekly and on manual dispatch, never on a push or pull request; no workflow was found that invokes the passage-level harness |
| `xerj` | BEIR SciFact, NFCorpus and FiQA-2018 tables with scripts; a paired stemming comparison | Public BEIR labels | BEIR test queries; tuning protocol not established | Not established |
| `iwe` | Runtime benches on generated documents | n/a | n/a | n/a |
| `quillrag` | A `quillbench` command; README says relevance is not yet benchmarked | None found | n/a | n/a |
| Others | Nothing found | n/a | n/a | n/a |

Re-read: `arabold`'s `.github/workflows/eval.yml` and the `evaluate:*` scripts in `package.json`. The donors' numbers are in revision 3 and are unchanged.

## Where OntoDoc is ahead

- **References.** It alone returns a hash and byte span with every hit and neighbour, and alone can report `valid`, `moved`, `stale` or `missing` against current bytes. Since 0.1.1 it also keeps the citation when the text does not fit.
- **Freshness at query time.** It alone compares content on every call. `xerj`'s hint and the refresh checks of `arabold`, `quillrag`, `RustRAG` and the Python project use modification times, or size and modification time.
- **Stored-index integrity.** In the code that was read, it alone checks that the records of a stored index are consistent with each other, and it alone documents a recovery procedure that a test covers.
- **Service boundary.** It alone exposes no write tool and bounds the complete response frame in bytes. `arabold`'s search assembly has no size bound, and `iwe`'s caps are unlimited when omitted.
- **Graph in query ranking.** It alone feeds link structure into the score of a text query.
- **Identity.** Its ids are readable and survive a reindex.
- **Determinism.** Every sort read here ends in a unique key. `quillrag` leaves fused ties without a second key.
- **Evaluation protocol.** It alone separates tuning data from reporting data and registers the run in advance.

## Where OntoDoc is behind

- **Cost of freshness.** A call on a 50,000-file synthetic corpus took about 0.3 s, and a corpus whose read charge exceeds the 128 MiB budget cannot be served. Donors pay almost nothing because they check less.
- **Large units.** A section above the `read` cap cannot be read whole in one call or in several. `docs/ONTOINDEX_IDEAS_PLAN.md` records one section of 107,222 bytes in the OntoIndex corpus; the script behind that figure was not located. A client can still read that section's chunks one by one, since the largest chunk in either corpus is 33,444 bytes.
- **Front matter.** Three donors read a title from it and two more parse it. OntoDoc drops the block, so a document whose only title is in front matter has none.
- **Links without a Markdown extension.** Directory and extensionless destinations produce no edge.
- **Regression guard for ranking work.** `arabold` has a stored baseline with a tolerance. OntoDoc has integrity gates and no comparison with an earlier dev result.
- **Hyphenated identifiers.** `REQ-12` has no joined token.

## Candidates

Status of the candidates from revision 3 (`docs/DONOR_CANDIDATES_PLAN.md`, closed, and `docs/ADR_DONOR_ADOPTION.md`, re-read):

| # | Technique | Status |
| --- | --- | --- |
| C1 | Report how much was left out | Done for `related` (`omitted`). Dropped for `search`, where it is nearly always 0 after C2 |
| C2 | Keep the citation when the text does not fit | Done: `tier: null`, empty text, rank-order fit |
| C3 | Count unresolved links before changing resolution | Measured by the calibrated census above. Any resolution change is still deferred |
| C4 | Stemming | Deferred behind a fresh test split |
| C5 | Heading weight, exact phrase | Deferred behind a fresh test split |
| C6 | Multi-hop traversal | Deferred; a client can call `related` on a returned id |

Open candidates from this round. Each needs no model and no network. None is approved by this report.

| # | Technique | Donor evidence | OntoDoc today | Cost | Gate | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| N1 | Continue a `read` past the cap | `arabold` `getPageContent`: `startChunk`, `nextChunk`, `totalChunks`; every call makes progress | 64 KiB prefix, no continuation | One request field and one response field in `src/cli.rs` and `src/mcp.rs`; no stored data | Service ADR amendment, since it adds a request field. Already a deferred row in two plans | Strongest fit. The need is one known section; reading by chunk id is a workaround |
| N2 | Read `title:` from front matter | `arabold`, `iwe` and `xerj` read a title; `RustRAG` and `DocumentationRetrievalMCPServer` parse the block | Block excluded; no title read | Schema bump if stored; a ranking change if it enters the BM25 field | Schema and ADR review. First step is a count of files in a real `docs_share` folder whose only title is in front matter | Most common donor practice OntoDoc lacks. Need unmeasured |
| N3 | Resolve an extensionless link to `<dest>.md` | `iwe` treats it as a document link | Ignored; 10 events to 2 destinations in the OntoIndex corpus, 0 in the Rust Book | A branch in `resolve`; schema bump; reindex of every store; graph-mode ranking changes | Count the unique edges gained first; then schema and ADR review | Small measured class. Not worth a schema bump alone |
| N4 | Resolve a directory link to its index document | None. `arabold` only normalises index URLs while crawling | Ignored; 52 events to 18 destinations in the OntoIndex corpus | As N3, plus a rule for which file a directory means | As N3 | No donor precedent. The largest ignored class, so it should be counted together with N3 |
| N5 | Fail a dev run that drops below a stored baseline | `arabold`: 0.02 tolerance, baseline bound to corpus, query-set and config hashes | Integrity gates only | A check in `src/eval/`, dev split only | Evaluation integrity rules; no ranking work is open, so it has no user yet | Add when a ranking round opens, together with the deferred evaluation arm |
| N6 | Joined token for hyphenated identifiers | `xerj` `code` analyzer keeps the whole token and its parts | `REQ-12` gives `req`, `12` | Tokenizer version 2; every generation id changes | The same registered family as C4 and C5 | Deferred with them |
| N7 | Grammar-based scheme test | `iwe` and `xerj` | `://` or lowercase `mailto:` | A few lines in `resolve`; schema bump only if an edge changes | The census found no affected link in either corpus | Low value; fold into N3 if that is ever done |

Rejected, with the reason:

- **Modification-time staleness in place of the per-call rehash** (`xerj`, `quillrag`, `arabold`, `RustRAG`). It weakens freshness, which `AGENTS.md` forbids, and it misses a same-size edit that keeps its time. The measured cost above is the price of the stronger answer.
- **Incremental indexing** (`xerj`, `quillrag`). The slowest synthetic case, 126 MB of source, indexed in about 2 s. No need is measured.
- **Ids without the path** (`xerj`). They break readable, stable citations.
- **Budgets in tokens** (`iwe`, `xerj`). The byte bound is exact for the transport; a token count depends on a tokenizer the client may not share.
- **Overlapping passages** (`xerj`, `quillrag`). They blur citation spans and make section-level labels ambiguous.
- **Taking the first candidate when a wiki link is ambiguous** (`iwe`). It invents edges without a report.

On fusion. Nothing in this round changes revision 3's reading. Decision 2 of the donor ADR still waits on a diagnosis of the 32 pairs that hierarchy mode displaces. No ranking change is authorized.

## Corrections

To revision 3 of this report:

1. "OntoDoc is the only project whose ranking uses a link graph" was too broad. `iwe` orders a listing by inbound link count when there is no text query and no sort. No donor uses a graph score for a text query.
2. The `xerj` Markdown unit was "not established". It is sections of about 2,048 characters with 200 overlap, at most 4,096 per file.
3. `arabold`'s authentication default was "not captured". It is off, and when on it covers only the MCP endpoint. Its default bind is `127.0.0.1:6280`.
4. `arabold` stores an `etag` per page, as revision 3 said, but for a local file that value is a hash of the modification time. It is not a content hash.
5. Revision 3 did not say where `arabold`'s benchmarks run. The page-level harness (`tests/search-eval`) runs weekly and on manual dispatch; its workflow says a regression can fail the step and does not block a merge. No workflow was found that invokes the passage-level harness with the 0.02 gate.
6. `quillrag` change detection and its schema check were not described. It skips on size and modification time, hashes otherwise, and refuses a store with another schema number.
7. The OntoDoc column predated the second 0.1.1 round. `search` no longer leaves out a hit whose text does not fit, `related` has `omitted`, `read` returns a prefix at 64 KiB, and load refuses malformed edges.
8. Four "Not verified" items are now answered: `arabold`'s authentication and update atomicity, `iwe`'s handling of fragments and `.html`, the absence of a regression threshold in OntoDoc, and the link-diagnostic counts.

To other documents, none of which was edited:

- `docs/ADR_DONOR_ADOPTION.md`, Decision 3, review result DA-4, says no gap in `src/graph.rs` was found and lists the differences from `iwe`. It omits one: `iwe` treats an extensionless destination as a document link, and OntoDoc ignores it. The later census counts 10 such link events in the OntoIndex corpus.
- The same review says both sides ignore external URLs. Their tests differ: OntoDoc looks for `://` or `mailto:`, and `iwe` accepts any scheme of two or more characters. The census found no link in either corpus where this changes an edge.

## Evidence

Files behind the tables. "re-read" marks a coordinator read for this revision.

- OntoDoc:
  - `src/search/tokenize.rs`, whole file; `src/parse.rs` module comment; `App::read` and `App::related` in `src/cli.rs`; `README.md`; `CHANGELOG.md`; `.git/refs/heads/master` (re-read).
  - `docs/ADR_DONOR_ADOPTION.md`; the headers and deferred tables of `docs/DONOR_CANDIDATES_PLAN.md` and `docs/ONTOINDEX_IDEAS_PLAN.md`; `.memory-bank/audit_session-2026-11-14-ontoindex-ideas.md` and `-capacity.md` (re-read).
  - `resolve` in `src/graph.rs:174-228`, `ReadRequest` in `src/mcp.rs:33-37`, `MAX_RESPONSE_BYTES` at `src/cli.rs:25`, the gate list in `src/eval/gates.rs`, versions and the dirty-path count (explorer).
- `arabold`:
  - `src/utils/config.ts` `DEFAULT_CONFIG`; `src/scraper/fetcher/FileFetcher.ts`; `getPageContent` in `src/store/DocumentManagementService.ts`; `.github/workflows/eval.yml`; `package.json` scripts (re-read).
  - `src/store/DocumentStore.ts`, the migrations and `tests/search-accuracy/` (carried from revision 3).
  - Per-page transaction, front matter through `gray-matter`, FTS5 tokens, authentication scope, the 30,000-character MCP default for `read_page` (explorer).
- `iwe`:
  - `crates/diwe/src/find.rs` (`find`, `order_by_rank`, `node_rank`, `sort_by_frontmatter`, `FindOutput`); `is_document_url` and `has_uri_scheme` in `crates/liwe/src/model.rs` (re-read).
  - `crates/diwe/src/search.rs`, `search_query.rs`, `crates/liwe/src/model/key_index.rs`, `crates/iwec/src/lib.rs` (carried).
  - Front-matter title key, per-request index build, tool count, truncation fields, walk limits (explorer).
- `xerj`:
  - `engine/crates/xerj-autoindex/src/extract/mod.rs` constants `SECTION_CHARS`, `SECTION_OVERLAP`, `MAX_RECORDS_PER_FILE`, `MAX_WHOLE_FILE`; `ids.rs`; `staleness.rs` module comment and constants; `prose_title` in `extract/txt.rs`; the head of `resolve_local` in `detect/mod.rs`; the head of `engine/crates/xerj-mcp/src/token_budget.rs` (re-read).
  - `engine/crates/xerj-fts/src/analyzer.rs`, `detect/mdlink.rs`, `default_detectors`, the BEIR tables (carried).
  - Segment checks, write sequence, change digest, `code` analyzer tests, the tie-order comment in `xerj-engine/src/index.rs`, the MCP tool list (explorer).
- `quillrag`: `index_one_with` and `index_directory_with` in `src/indexer.rs` (re-read); `src/search.rs`, `src/store.rs`, `src/server.rs`, `src/extract.rs`, `assets/ocr/NOTICE.md` (explorer).
- `RustRAG`: `src/indexer/core.rs`, `src/db/`, `src/frontmatter.rs`, `src/mcp/` (explorer).
- `Govcraft`: `src/doc_loader.rs`, `src/server.rs`, `Cargo.toml` (explorer).
- `DocumentationRetrievalMCPServer`: `src/document_upload/`, `src/mcp_server/` (explorer).
- `MicrosoftDocs/mcp`: `cli/src/utils/markdown.ts` (re-read); `cli/src/commands/fetch.ts`, `cli/src/mcp/`, `ThirdPartyNotices.md` (explorer).

## Not verified

- Most of `xerj`: its search path beyond the parts listed, whether any ranking code reads link edges, its `code` analyzer on hyphenated identifiers, and the body of its tie-order test.
- How many bytes OntoDoc's MCP frame returns for a section above 64 KiB. `App::read` cuts at 64 KiB and the frame bound can cut again; `docs/ONTOINDEX_IDEAS_PLAN.md` infers about 32 KiB, and nobody has run it. The origin of the 107,222-byte figure is also unknown.
- How many files in a real `docs_share` folder carry a title only in front matter. This decides N2.
- How many unique edges N3 and N4 would add. The census counts link events, not edges.
- The time and memory that the new load check adds. The capacity build predates it.
- `quillrag`'s `unchanged_by_stat` body and its MCP tool count, where this round's explorer and revision 3 disagree; `RustRAG`'s handling of deleted files; `arabold`'s behaviour when a database is newer than the code.
- Licence headers in donor source files beyond the explorers' searches.
- Any donor's runtime behaviour, speed or the reproducibility of its numbers.

## Revision history

- Revision 4, 2026-11-14: OntoDoc column updated for the second 0.1.1 round and for the capacity and link measurements; new tables for updates, Markdown handling, link classification, identifier tokens and stored-index integrity; status of C1 to C6; candidates N1 to N7; eight corrections to revision 3.
- Revision 3, 2026-11-14: fresh source review of all eight donors against the 0.1.1 working tree; six comparison tables; candidates C1 to C6 with gates; nine corrections to revision 2.
- Revision 2, 2026-11-14: source-level comparison replacing the README-based one.
- Revision 1, 2026-11-13: comparison from READMEs.
