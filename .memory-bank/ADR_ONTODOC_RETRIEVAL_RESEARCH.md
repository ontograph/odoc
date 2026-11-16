# ADR: Retrieval Research Priorities From The Medium RAG Review

Date: 2026-11-16
Revision: 4 (2026-11-16, user: "propose several solutions to do next tasks and fix issues, cherry pick most robust and performance proposals, consolidate it to one plan and proceed it"). Revision 4 records the user's answers that let steps 1 and 2 run, registers the step 1 and step 2 rules and thresholds in [Registration](#registration) before any step 1 code exists, and moves the step 2 and step 3 experiments into dev-only research scripts. [Revision 4 Change](#revision-4-change) lists the proposals adopted and rejected. Revision 3 (user agreed to add a step for depth misses) added step 2, a model-free reordering of the lexical candidates aimed at `truncated` misses, extends step 1 with the descriptors that select it and the `truncated` ceiling, renumbers the later steps and adds decision 8; [Revision 3 Change](#revision-3-change) gives the reason. Revision 1 ranked research topics from the collection comparison alone. Revision 2 (challenge review requested by the user) added the dev miss evidence already recorded for the hybrid proposal, replaces the prose sequence with gated steps, states the constraints that keep every step exploratory, corrects the language scope, cites every collection entry the comparison uses by its original URL and lists the decisions acceptance needs. The [disposition table](#revision-2-challenge-disposition) gives each challenge with its evidence.
Owner: OntoDoc maintainers
Requested by: user, "make adr md file as proposed", after the comparison of the `tmp/medium` collection with OntoDoc. Revision 2 follows "review and challenge .memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md update doc". Revision 3 follows "agree", the user's answer to the proposal to add a step for depth misses.

## Status

Proposed, with steps 1 and 2 accepted for dev-only exploratory execution on 2026-11-16 under the answers in [Registration](#registration). Steps 3 to 5 stay proposed.

Step 1 ran on 2026-11-16 and met no routing threshold ([Step 1 Result](#step-1-result)). The current system stays, and steps 2 to 4 were recorded as not opened.

This record orders retrieval research topics and defers the ones with no demonstrated need. Beyond steps 1 and 2 as registered, it authorizes no implementation, dependency, model acquisition, measurement, tokenizer or schema change, default promotion or task completion. The [service ADR](ADR_ONTODOC_SERVICE.md) remains authoritative.

Three proposed records own the detailed designs, and this record changes none of them. The [hybrid retrieval ADR](ADR_ONTODOC_HYBRID_RETRIEVAL.md) owns the dense and fusion experiment. The [agentic retrieval ADR](ADR_ONTODOC_AGENTIC_RETRIEVAL.md) owns the client policy. The [evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md) owns context-selection accounting and rendered-text provenance. This record owns the collection comparison, the order across those proposals and the disposition of topics none of them covers.

## Context

### What The Collection Supports

The collection under `tmp/medium` was requested for 2025-11-16 through 2026-11-16. Its `manifest.json` lists 30 posts.

| Property | Value |
| --- | --- |
| Coverage | 2 author-feed texts, whose completeness against the original pages is unverified, and 28 excerpts of 35 to 184 characters |
| Dates | 2026-06-29 through 2026-11-16. 22 posts are from November 2026 and 10 of those from the collection day. The first seven months of the requested window are empty |
| Authors | 26. Both longer texts are by one author, who describes M1 as article 9 of a 20-article learning series |
| Feeds | The 30 posts come from four tag feeds and one author feed. 12 of 20 tag feeds and 39 author feeds failed with HTTP 429, 51 failures in total |

The manifest's `authors_queried` value is 39, equal to the number of failed author feeds, although a further author feed supplied both longer texts. At least 40 author feeds were requested, so that count is unreliable.

The collection is a topic inventory. No entry supplies a reproducible measurement, and no disposition below rests on an article's claim. M5's title announces a measurement of graph RAG; its 168-character excerpt contains none.

The two longer texts are tutorials. M1 says that similar text is not necessarily the answer, that stored embeddings may need updating when documents change and that a small collection may need no dedicated vector database. M2 separates retrieval quality from answer faithfulness and correctness, defines Precision@K, Recall@K and MRR, names the Ragas framework while noting that some of its metrics need further models or reference data, and recommends unanswerable questions in a test set.

### What Dev Evidence Already Shows

Stage 0 of the [hybrid plan](../docs/HYBRID_RETRIEVAL_PLAN.md) measured BM25 misses on the existing dev split on 2026-11-16. The [stage-0 audit](audit_session-2026-11-16-hybrid-stage0.md) records the inputs, hashes and limits. The labels are model-judged and pooled from B0 to B3, so the figures are exploratory and the miss shares are lower bounds.

| Measure (B1, 82 answerable dev queries) | Value |
| --- | --- |
| Miss pairs by stage | 78 `truncated`, 21 `lexical_miss`, 0 `chunk_boundary`; 99 in total |
| Queries with a relevant section outside the top ten | 48/82 = 0.585, Wilson 95% [0.477, 0.686] |
| Queries with a `lexical_miss` section | 18/82 = 0.220, Wilson 95% [0.144, 0.321] |
| Oracle nDCG@10 gain over the B1 mean of 0.7247 | +0.1289 reordering the current top ten; +0.1630 inserting every `lexical_miss` section at its ideal rank; +0.2753 recovering every miss |

The stage names come from `src/eval/sweep.rs::attribute_miss`. Read with that source, the table supports six statements:

1. 78 of 99 miss pairs (78/99 = 0.788) are `truncated`: BM25 returns a chunk of the section within 50 chunks, and the section is outside the top ten. These are ordering misses among candidates the lexical leg already has. A new candidate source does not address them directly.
2. `lexical_miss` (21/99 = 0.212) merges two cases: the section shares no token with the query, or it scores above zero and ranks below 50. The tokenizer removes no stop words and BM25 idf is always positive (`src/search/tokenize.rs`, `src/search/bm25.rs`), so a section sharing one function word with the query has a positive score. For natural-language queries the second case may dominate. That is an inference from source and is unmeasured. The stage therefore measures neither vocabulary mismatch nor inflection.
3. `chunk_boundary` is 0. No dev miss has its matching terms outside every chunk of the section, so dev evidence gives no reason to change chunking. The stage says nothing about text withheld at delivery.
4. A query with more than ten judged-relevant sections has misses under every ranker. The audit does not report how many of the 48 queries are in that position, so the `truncated` count overstates what reordering can fix by an unknown amount.
5. The oracle gains are ceilings: each places sections at their ideal ranks, which no real ranker does. The third equals 1 − 0.7247, a perfect ranking. Reordering the current top ten has a ceiling of +0.1289, close to the +0.1630 ceiling for the `lexical_miss` sections, so ordering is a research target of about the same size as candidate recovery.
6. The audit reports no ceiling for recovering the `truncated` sections alone.

The audit names two corpora, `rust-book` and `ontoindex`, and reports no non-ASCII character in any dev query. No need threshold was registered before stage 0 ran.

### Constraints That Bind Every Step

- **Labels.** The service ADR records "Owner decision 2026-11-11: no human annotators are available" and "human qrels remain the only route to a confirmatory claim". Until that changes, every comparison below yields effects and intervals with no decision and no default change. Whether any step can change the product depends on the label track before it depends on the technique.
- **Dense leg.** The service ADR's Consequences state: "A dense leg can be proposed after H2 resolves, as a new pre-registered hypothesis under the same protocol." H2 has a model-judged exploratory interval and no decision. How that condition is met is hybrid ADR decision 3 and is open.
- **Models.** Service ADR decision 6 keeps LLM, embedding and outbound-network dependencies off the retrieval path, and its Consequences reject "LLM intent expansion and LLM rerank" as irreproducible. Neither sentence names a learned reranker that is not an LLM. This record treats one as covered until the owner says otherwise.
- **Tokenizer.** `IndexConfig::tokenizer_version` feeds the generation ID (`src/store.rs`). A normalization change bumps that version, which alters every generation ID, the BM25 statistics, `unmatched_terms` and the abstention score, and needs a trusted reindex of every corpus. The pinned benchmark stores get new generations, so the change needs a new evaluation preparation. `Cargo.toml` has no stemming dependency.
- **Pools.** The existing qrels were pooled from B0 to B3. A section that only a new arm retrieves is unjudged and scores as nonrelevant, which biases the existing qrels against every new arm. A new arm needs a pool extension and new judgments before a dev comparison means anything.
- **Dev reuse.** The dev split has informed the B2 and B3 weights and stage 0. Each further look adapts to it. A claim about a new arm needs fresh queries, as the hybrid ADR already requires.
- **Language.** The user wrote "propose answeres ( 1- NO )" on 2026-11-16, read in this session as: the real workload has no cross-language queries. No repository file records that answer. If it stands, it removes the cross-language stratum and the cross-language motivation from the dense proposal. It does not say which language the real corpora and queries use. A stemmer is language-specific, and an embedding model must cover the corpus language. The stage-0 figures come from ASCII queries, and the audit names only the `rust-book` and `ontoindex` corpora, so they say nothing about a corpus in a more inflected language.

## Consolidated Comparison

Source IDs refer to [Sources And Evidence Limits](#sources-and-evidence-limits). "Text" marks one of the two longer texts; every other entry is a title and an excerpt.

| Approach or tools | Collection evidence | OntoDoc today | Disposition |
| --- | --- | --- | --- |
| Structural chunking, parent-child retrieval | M1 (text): chunk size and overlap affect quality. M3: title names parent-child search | Sections and chunks follow Markdown structure; the 1,200-byte chunk budget is frozen in v1; L0, L1 and L2 tiers are source fragments; assembly is budgeted | No chunker change: dev `chunk_boundary` is 0. Text lost at delivery is a separate question that the evidence/diagnostics ADR owns |
| BM25 and keyword search | M1 (text): keyword search, alone or with vectors, can be enough | BM25 is the default. The tokenizer lowercases and splits identifiers; it has no stemming and no stop words | Baseline and control. Stemming is this record's own candidate; no collection entry mentions it. Step 3 |
| Embeddings, dense retrieval, hybrid fusion | M1 (text): the mechanism, and that similar is not the same as correct. M7: excerpt lists embeddings and vector search | No dense leg. Exact-cosine and rank-fusion functions exist in `src/search/dense.rs` with no caller | Step 4, under the hybrid ADR |
| FAISS, Pinecone, vector databases | M4: title. M1 (text): a dedicated vector database matters for larger collections | None | No backend. The hybrid ADR uses exact search over a bounded matrix. An approximate index or a hosted service needs a measured scale problem first |
| Reranking | M7: excerpt. M2 (text): named as one option after evaluation finds a retrieval problem | B2 and B3 reorder a lexically seeded pool without a model | No model reranker: the service ADR blocks it. B2 and B3 were measured in the registered experiment with exploratory outcomes. Step 1 describes the ordering misses, and step 2 tests one declared model-free reordering of the lexical candidates with its own dev evidence |
| Entity GraphRAG and knowledge graphs; Neo4j, PostgreSQL, LM Studio | M5, M11, M12, M13: titles and excerpts | The graph holds documents, sections and chunks. Ranking uses `LINKS_TO` and `NEXT` edges only. No entity is extracted | Deferred. Reopen on an approved set of relationship questions that B1 and B2 both miss. Model-based entity extraction would add a model to indexing, with its own errors and provenance |
| Temporal graphs and ontology layers | M6, M14: titles and excerpts | Current-byte freshness and `verify`. No validity interval for a fact | Deferred. Reopen on a stated as-of workload |
| Query rewriting and multi-query retrieval | M3, M16, M17: titles and excerpts; M17 describes rewriting by an LLM | Search returns `unmatched_terms`; the server instructions advise rephrasing when results miss | Step 5, under the agentic ADR. Model-written rewriting waits for that ADR's open decision |
| Adaptive and agentic retrieval; LangGraph, MongoDB, PostgreSQL | M8, M13, M15: titles and excerpts | Five read-only tools. The service generates no answer | Step 5. Framework and storage choices belong to the client |
| Quote and citation checking | M9: excerpt describes a quote check that catches made-up citations | Citations carry span hashes; `verify` reports valid, moved, stale or missing; `read` returns the indexed text | No server change. A client can match its quotation against `read` output today. `verify` establishes byte correspondence and says nothing about support for an answer |
| Document permissions | M10: excerpt describes a two-user permission test | One store per corpus. Every service user may read every exposed corpus | No change. Document ACLs need a changed requirement, and `scope` is a search filter |
| Evaluation; Ragas, Precision@K, Recall@K, MRR, faithfulness | M2 (text) | Section-keyed graded qrels, nDCG@10, an off-topic stratum, once-only custody | No adoption. Answer-level metrics need an answering client and belong to the agentic ADR's outcome. A Ragas metric that calls a model is a model-judged label |
| PDF ingestion, local or cloud generation, fine-tuning | M18, M19, M20: titles and excerpts | Markdown corpora only. No generation | Outside the service boundary. No step |
| Production failure modes | M21: title only | Bounds, admission and freshness are service contracts | No evidence to act on |

## Proposed Decision

Step 1 routes the work. Each later step starts only when step 1's share for the miss class it targets meets its registered threshold, and each stops on its own condition. Steps 2 to 4 each test one change against B1, in the order below, so that no two effects are confounded. Each step needs its own acceptance; the [plan](../docs/RETRIEVAL_RESEARCH_PLAN.md) holds the tasks.

| Step | Question | Needs | Stop when |
| --- | --- | --- | --- |
| 0. Dev miss share | How often does BM25 miss a judged-relevant section? | Done under the hybrid plan as HY-0 | Not stopped on the hybrid ADR's `p` bound |
| 1. Split the existing misses | What do `lexical_miss` and `truncated` consist of? | The existing dev run outputs, a declared content-term rule, surface-form rule and set of depth descriptors, and thresholds registered before the step runs. No model, label, dependency or crate change | No threshold is met: the current system stays and steps 2 to 4 are not started |
| 2. Reorder the lexical candidates | Does one declared, deterministic reordering of B1's top 50 chunks move `truncated` sections into the top ten? | Step 1's depth descriptor at its threshold, naming one candidate. A diagnostic against the existing qrels as registered; a claim needs new judgments. No model, dependency, tokenizer or schema change | The dev difference is at or below 0, or the arm removes more judged-relevant sections from the top ten than it adds |
| 3. Lexical normalization arm | Does a declared normalization recover surface-form misses without merging distinct identifiers or versions? | Step 1's surface-form share at its threshold. The experiment uses the registered endings in a dev-only script; a promotion needs the corpus language, a stemmer source, a tokenizer version change with reindex and new judgments | The dev difference is at or below 0, or the identifier and version guardrail fails |
| 4. Dense and hybrid | The hybrid ADR's stages 1 to 4, unchanged | Step 1's share of misses with no shared content term at its threshold, plus every hybrid ADR decision | As the hybrid ADR states |
| 5. Client policy | The agentic ADR's stages, unchanged | That ADR's open decision recorded. Its stage 0 then needs no model and no label, and is independent of steps 1 to 4 | As the agentic ADR states |

Step 1 has three parts. The first assigns every `lexical_miss` pair to one of three groups: no shared content term, a shared surface-form variant, or a shared term ranked below 50. The second assigns every `truncated` pair to a query with more than ten judged-relevant sections or to the remainder, and reports the rank band of each. The third describes each `truncated` pair in the remainder by three depth descriptors, compared with the sections ranked in the top ten: whether its query terms occur in its heading path, how many of the query's distinct terms its best chunk matches, and how many of its chunks score above zero. It also reports the oracle nDCG@10 ceiling for recovering the `truncated` sections alone, which the stage-0 audit omits (statement 6). All three parts are descriptive, read dev data only and add no ranking arm.

Step 2 tests the one reordering that the step 1 descriptor meeting its threshold names. B1 scores a chunk's heading path and text as one bag of tokens (`SearchIndex::indexes` in `src/search/mod.rs`), and the harness ranks a section by its first chunk among B1's top 50 (`rank` in `src/eval/run.rs`). The candidates follow from those two facts: heading terms scored as a separate field when missed sections match in their headings, a factor for the share of distinct query terms matched when missed sections match more of them than the sections above, or a section score aggregated over its chunks when missed sections have several matching chunks. The arm keeps B1's candidate set, so it cannot recover a `lexical_miss` section, and its ceiling is step 1's `truncated` ceiling. It is reachable from the evaluation harness only and leaves B1 and every search mode unchanged. A second candidate needs its own run after the first ends.

The order follows cost and what the label constraint leaves open. Step 1 and the agentic ADR's stage 0 need no labels and no model. Step 2 needs no dependency, tokenizer change or reindex, and targets the largest miss class, 78 of 99 pairs. Step 3 is cheaper than step 4 in models and licences and dearer in store churn, because it changes every generation ID. This record proposes that hybrid stage 1 wait for step 1's result. The hybrid ADR's own gates are unchanged.

Under model-judged labels, steps 2 to 4 can end only with exploratory intervals. None can change a default.

## Decisions Required At Acceptance

1. Whether this stays a separate record or its comparison and order move into the hybrid ADR.
2. The language or languages of the real corpora and queries, and confirmation that "1- NO" answered the cross-language workload question.
3. The label track, which is also hybrid ADR decision 4: human annotators, or exploration that cannot reach a default.
4. Whether step 1 may run on the existing dev outputs, with its content-term rule, surface-form rule, depth descriptors and thresholds.
5. Whether the rule against models on the retrieval path covers a learned reranker that is not an LLM.
6. For step 3: the stemmer source and licence, the languages, acceptance of a tokenizer version change with a reindex of every corpus, and whether the change needs a service-ADR amendment.
7. The deferrals in the comparison table and their reopen conditions.
8. For step 2: acceptance of an evaluation-only arm, and whether promoting it later needs a service-ADR amendment, because it would change the hit order of every search mode that builds on B1.

## Alternatives And Consequences

| Option | Disposition |
| --- | --- |
| Keep the current system and stop | Valid, and the outcome when step 1 meets no threshold |
| Adopt a stack from the collection: vector database, embeddings, reranker and entity graph together | Rejected. The effects would be confounded, query-time embeddings and a model reranker conflict with the service ADR, and the collection holds no supporting measurement |
| Start the dense experiment now | Not stopped by the hybrid ADR's stage-0 bound, and blocked on that ADR's open decisions. It spends model selection, licences, vectors and new labels before anyone knows what `lexical_miss` consists of |
| Split the existing misses first | **Proposed** |
| Reorder the lexical candidates before adding a candidate source | **Proposed** as step 2, gated on step 1. It targets the largest miss class at the lowest cost; its ceiling is unknown until step 1 reports it |

Step 1 costs one script and one short audit, and it can end or reorder steps 2 to 4. It adds a further look at a dev split that the B2 and B3 weights and stage 0 have already used.

Two follow-ups sit outside this record. The hybrid ADR and plan still list the cross-language question as open and still specify a cross-language stratum, so they need a revision once decision 2 is answered. The collection lives under `tmp/`, which `.gitignore` excludes; publishing this record needs the collection kept somewhere durable, or the original URLs below accepted as the reference.

## Registration

Registered 2026-11-16, before any step 1 script was written or run. The rules below may change only by a new dated revision made before the step they govern runs; a change after a run needs a new run under a new registration.

### Answers Recorded

| Decision | Answer | Source |
| --- | --- | --- |
| 1 | Separate record | Existing ownership; no merge requested |
| 2 | No cross-language workload. Corpus and query language stay unstated; the dev queries contain no non-ASCII character (HY-0), which proves neither English nor a language mix | User, "propose answeres ( 1- NO )", 2026-11-16 |
| 3 | Model-judged exploratory track. No result here can reach a decision or a default | Service ADR owner decision 2026-11-11, no human annotators |
| 4 | Step 1 may run on the existing dev outputs with the rules below | User, "consolidate it to one plan and proceed it", 2026-11-16 |
| 5 | A learned reranker that is not an LLM is treated as covered by the no-model rule | Conservative default; no task depends on it |
| 6 | Deferred until step 3's measurement routes to a promotion proposal. The step 3 experiment needs no stemmer dependency because it uses the registered surface-form rule | This revision |
| 7 | Deferrals and reopen conditions kept as listed in the comparison table | No change requested |
| 8 | Step 2 runs as a dev-only research script that permutes B1's own candidates. No Rust ranking code changes; a Rust port and the service-ADR question arise only with a promotion proposal | User, "proceed it", 2026-11-16 |

Prior exposure: the aggregate B1 miss counts and oracle gains in [What Dev Evidence Already Shows](#what-dev-evidence-already-shows) were seen before this registration and informed the choice of steps. No token, per-pair or per-query content of any miss was seen. The thresholds are prospective routing rules for the dev split, which the B2 and B3 weights and HY-0 have already used. They are policy choices, not significance tests.

### Step 1 Inputs And Parity Gates

The inputs are HY-0's five files with the hashes in `tmp/hybrid-stage0/summary.json`, the pinned stores of both corpora at the generation IDs in the dev run's `meta.json`, and the current `target/release/ontodoc`, whose sha256 is recorded. No other evaluation file is opened. The script stops before any classification when any gate fails:

1. Every input hash equals HY-0's, and each store's `CURRENT` names the pinned generation, before and after the run.
2. For every answerable dev query, `ontodoc search --mode lexical --top-k 50 --explain` returns `truncated: false`, and the first-occurrence projection of its chunks to ten sections equals that query's rows of `B1.tsv` in order, with scores equal within 1e-9 relative.
3. A Python port of tokenizer version 1 (`src/search/tokenize.rs`) and of BM25 (`src/search/bm25.rs`, k1 1.2, b 0.75, heading path then chunk text per chunk) reproduces the binary's 50 chunks for every query, in order, with scores equal within 1e-9 relative.
4. The B1 miss pairs recomputed from those rankings and the dev qrels equal the B1 records of `misses.json`, stage by stage: `truncated` exactly when a chunk of the section is among the 50.

A self-test over synthetic fixtures with hand-computed expected values runs before the real inputs are read.

### Step 1 Rules

Query terms `Q` are the unique tokens of the query. Content terms `Q_c` are `Q` minus this list: a an the and or but if then else as at by for from in into of on onto to with without about is am are was were be been being do does did have has had i me my we us our you your he him his she her it its they them their this that these those what which who whom whose when where why how can. A section's tokens are the tokens of its chunks as indexed: heading path, then chunk text.

Each `lexical_miss` pair goes to the first group that applies:

1. Shared term ranked below 50: a term of `Q_c` occurs in the section's tokens.
2. Shared surface-form variant: a term of `Q_c` and a section token form a registered surface-form pair.
3. No shared content term: otherwise. A flag records whether the section shares only terms outside `Q_c`.

A surface-form pair is two different eligible tokens where one is the other plus one registered ending. An eligible token comes from a word the tokenizer did not split, consists of ASCII letters only and is at least three letters long. Tokens with digits, underscores or camelCase parts never fold, so identifiers and version strings stay distinct. With `x` the shorter token, the endings are: `x` and `x + s` when `x` does not end in `s`; `x` and `x + es` when `x` ends in `s`, `x`, `z`, `ch` or `sh`; `x` ending in a consonant then `y`, and `x` with that `y` replaced by `ies`; `x` and `x + ed` or `x + ing`; `x` ending in `e`, and `x` without that `e` plus `ed` or `ing`; `x` ending in a single consonant after a vowel, and `x` plus that consonant doubled plus `ed` or `ing`. Vowels are a, e, i, o and u; every other ASCII letter is a consonant. One step only; no chain of endings.

Each `truncated` pair goes to queries with more than ten judged-relevant sections, or to the remainder. Its rank band uses its section rank in B1's first-occurrence order over the 50 chunks: 11 to 20, or 21 to 50.

Each remainder pair gets three descriptors, and so does each of its query's B1 top-ten sections:

- `H`: 1 when a term of `Q_c` occurs in the tokens of the section's own heading path, else 0.
- `C`: the number of distinct `Q_c` terms in the tokens of the section's highest-ranked chunk among the 50.
- `N`: the number of the section's chunks among the 50.

A descriptor favours the missed section when its value is strictly greater than the median of that descriptor over the query's top-ten sections.

The truncated-only ceiling is HY-0's oracle with the `truncated` sections added to the top ten and the `lexical_miss` sections left out. It includes reordering of the existing top ten.

Shares are reported over pairs with a 95% query-cluster bootstrap interval (10,000 resamples of queries, SplitMix64 seed 20261116) and, for reference, a Wilson interval.

### Routing

The point estimate decides. Denominators are all `lexical_miss` pairs for steps 3 and 4, and all remainder `truncated` pairs for step 2. A zero denominator routes nothing.

| Route | Share | Threshold |
| --- | --- | --- |
| Step 2 opens | Pairs favoured by `H`, by `C` or by `N` | At least 0.50 for one descriptor. The candidate is the descriptor with the largest share; ties go to `H`, then `C`, then `N` |
| Step 3 opens | Shared surface-form variant | At least 0.25 |
| Step 4 signal | No shared content term | At least 0.50. Recorded in the hybrid plan; the hybrid ADR's own decisions still gate HY-2 |

When no row meets its threshold, the current system stays and steps 2 to 4 do not start.

### Step 2 Arm

The arm B1R takes exactly B1's 50 scored chunks and permutes them; it adds and removes nothing. Each chunk `c` with B1 score `s` gets a new score `R`, and chunks sort by `R` descending with ties by chunk ID. Sections are then projected to ten by first occurrence, as for B1. Only the candidate step 1 names runs; its constant is fixed here:

| Descriptor | Candidate | `R(c)` |
| --- | --- | --- |
| `H` | Heading field | `s` plus BM25 of `Q` over heading-path tokens alone, with an index over every chunk's heading path, k1 1.2, b 0.75 and weight 1 |
| `C` | Coverage | `s` times the share of `Q_c` terms in the chunk's tokens; `Q` stands in when `Q_c` is empty |
| `N` | Section aggregation | The sum of the B1 scores of the 50 chunks in the same section, added in chunk-rank order |

The comparison uses the existing dev qrels only, so it is a diagnostic. It reports the paired mean nDCG@10 difference with a 95% query bootstrap interval (10,000 resamples, seed 20261116), condensed nDCG@10 for both systems, judged@10 for both, the known relevant sections B1R moves into the top ten and the judged-relevant sections it moves out. Step 2 stops when the mean difference is at or below 0, or when it moves out more than it moves in. When judged@10 for B1R is below 0.95, the result is recorded as underdetermined: sections it promotes are unjudged, and a claim needs new judgments under the service ADR's dual-judge model track, which has no task here.

## Step 1 Result

The [audit](audit_session-2026-11-16-retrieval-research.md) holds commands, hashes and limits. Every parity gate passed for the 82 answerable dev queries.

| Share | Value | Threshold |
| --- | --- | --- |
| Best depth descriptor, `H` (heading match) | 22/66 = 0.333, query-cluster 95% [0.206, 0.471]; `C` 0.136, `N` 0.197 | 0.50 for step 2 |
| Surface-form variant | 0/21 = 0.000 | 0.25 for step 3 |
| No shared content term | 1/21 = 0.048, [0.000, 0.167] | 0.50 for the step 4 signal |

20 of the 21 `lexical_miss` pairs share a content term with the query and rank below 50. The truncated-only oracle gain is +0.2441 of the +0.2753 all-miss ceiling, so depth misses hold most of the headroom, but no registered descriptor separates them from the top ten. Another depth hypothesis needs a new dated registration before it runs. The results are exploratory.

## Revision 4 Change

Three read-only GPT reviews proposed alternatives for step 1, for the step 2 arm and for the plan's gates. Adopted:

- Step 1 runs as a Python script over the pinned binary's rankings and the stored generation, gated on exact parity with `B1.tsv`, `misses.json` and the binary's top 50. This is cheaper than a new Rust subcommand and touches no crate code. A Python reimplementation without the parity gates was rejected.
- The step 2 arm runs as a research script over B1's captured candidates. Adding a `System` variant was rejected: it touches the evaluation dispatcher, which OntoIndex rates HIGH risk, and the four-system lists the held-out machinery checks. A dev-only CLI subcommand was rejected because it adds a CLI surface.
- Step 2 and step 3 compare against the existing qrels as a diagnostic with judged@10 and condensed nDCG reported. Reusing `config.pool_extension` was rejected: the service ADR permits it only for unanswered queries chosen before rankings were seen.
- Step 3's experiment folds the registered surface-form endings inside the parity-checked Python BM25. This needs no stemmer dependency, no tokenizer version bump and no reindex until a promotion is proposed.
- Pair shares carry a query-cluster interval, because pairs from one query are correlated.
- Truncated pairs have no band beyond 50: by definition their section is among the 50 chunks.

## Revision 3 Change

Revision 2's steps 2 and 3 both add or change candidates, so they address only `lexical_miss`: 21 of 99 dev miss pairs, with an oracle ceiling of +0.1630. Its own statements 1 and 5 show that 78 pairs are `truncated` and that ordering is a research target of about the same size, yet no step acted on them; the reranking row deferred them to an unnamed proposal. Revision 3 makes that proposal step 2 and adds the step 1 descriptors that choose its candidate. It changes no constraint, and the step stays exploratory under model-judged labels.

## Revision 2 Challenge Disposition

| Challenge to revision 1 | Evidence | Change |
| --- | --- | --- |
| Its first step asked for a miss diagnosis that already existed | The stage-0 audit of the same day reports miss pairs by stage, with intervals and oracle gains | The dev evidence section; the order is built on it |
| It put candidate recall first | 78/99 dev miss pairs are `truncated`: the candidate is present and ranked out of the top ten. The reorder ceiling of +0.1289 is close to the `lexical_miss` ceiling of +0.1630 | Statements 1 and 5 and the reranking row; no claim that a dense leg addresses the largest miss class |
| "If dev misses show inflection failures" could not be evaluated | `attribute_miss` merges zero overlap and rank below 50 in `LexicalMiss`; the tokenizer keeps stop words | Step 1 defines the split |
| Normalization was presented as a cheap control | `tokenizer_version` feeds the generation ID; `Cargo.toml` has no stemmer; the qrels were pooled from B0 to B3 | Costs listed under the tokenizer and pool constraints; step 2 gated behind step 1 |
| It ruled out a model's language coverage as a selection criterion | No cross-language queries does not settle the corpus language; a stemmer and an embedding model both depend on it | Language constraint rewritten; decision 2 |
| The cross-language answer was stated as settled | No repository file records it, and the hybrid ADR and plan still list the question as open | The answer is quoted as written and confirmed under decision 2 |
| The annotator constraint was missing | Service ADR, model-judged track, owner decision of 2026-11-11 | Label constraint; the stated limit on steps 2 and 3; decision 3 |
| The service ADR's condition for a dense leg was missing | "A dense leg can be proposed after H2 resolves" | Dense-leg constraint |
| Context loss was a step here although another record owns it | The evidence/diagnostics ADR proposes context-selection accounting and rendered-text provenance | That ADR is linked, and the step is removed |
| The relationship step repeated a completed contrast | "Compare existing document-link traversal against ordinary retrieval" is B2 against B1, which the registered experiment ran once | Step removed; a reopen condition replaces it |
| Sources were not durable or complete | 12 links pointed into the git-ignored `tmp/`; rows cited "collection entries" without identifiers; Ragas was omitted; the FAISS and Pinecone descriptions came from no collection entry | Original URLs, M1 to M21, an evaluation row with Ragas, and the unsourced sentence removed |
| The collection's bias was described without numbers | Manifest dates, authors and feed failures | The property table |
| The record decided nothing | No stop condition and no decision list; several paragraphs restated protocol rules that the service ADR and `AGENTS.md` already hold | Gated steps, decisions required, and the restated rules removed |

## Sources And Evidence Limits

The URLs come from the manifest. None was fetched for this revision. The local capture of each entry is the file in `tmp/medium` whose name ends with the same identifier as the URL.

- M1, text: [What I Learned About Vector Databases While Building My First RAG Application](https://medium.com/@kamechi098/what-i-learned-about-vector-databases-while-building-my-first-rag-application-2a169c1ba564)
- M2, text: [How to Evaluate RAG Applications: Measuring Accuracy, Relevance and Reliability](https://medium.com/@kamechi098/how-to-evaluate-rag-applications-measuring-accuracy-relevance-and-reliability-5cc72c70459c)
- M3: [Advanced RAG Explained: Query Rewriting, Multi-Query Retrieval, Parent-Child Search, and Contextual…](https://medium.com/ai-engineering-compass/advanced-rag-explained-query-rewriting-multi-query-retrieval-parent-child-search-and-contextual-5f9e087e2878)
- M4: [FAISS vs Pinecone: Which Vector Database Should You Choose for RAG?](https://medium.com/@souvikrajsingh02/faiss-vs-pinecone-which-vector-database-should-you-choose-for-rag-a9c172c49b85)
- M5: [Does Graph RAG Actually Help? I Measured It on Postgres and TypeScript](https://medium.com/@kun742711819/does-graph-rag-actually-help-i-measured-it-on-postgres-and-typescript-e2bb4caacc83)
- M6: [Inside a temporal Graph RAG](https://medium.com/@cabreirajm/inside-a-temporal-graph-rag-81b9f5efcc1e)
- M7: [Your RAG System Is Finding the Wrong Documents. Here's Why.](https://medium.com/codetodeploy/your-rag-system-is-finding-the-wrong-documents-heres-why-b75385bde50d)
- M8: [MongoDB vs PostgreSQL for a LangGraph RAG Agent: How to Choose](https://medium.com/@boilershelf/mongodb-vs-postgresql-for-a-langgraph-rag-agent-how-to-choose-a605bfef5833)
- M9: [Your RAG Agent Cites Sources. Check That the Quotes Are Real.](https://medium.com/@satyamyadavaiml/your-rag-agent-cites-sources-check-that-the-quotes-are-real-5df48530baf8)
- M10: [Your AI Found the Right Document. Should It Have Been Allowed To?](https://medium.com/@emilyhustlenyc/your-ai-found-the-right-document-should-it-have-been-allowed-to-345cd7952d3e)
- M11: [How to Build a Personalized E-Commerce Recommendation System Using GraphRAG, Neo4j, and Python](https://medium.com/@mukherjee.arindam92/how-to-build-a-personalized-e-commerce-recommendation-system-using-graphrag-neo4j-and-python-f85f579fea5e)
- M12: [Notlardan Bilgi Grafiğine: Neo4j ve LM Studio ile GraphRAG Denemem](https://medium.com/@ygt.ersoy01_50518/notlardan-bilgi-grafi%C4%9Fine-neo4j-ve-lm-studio-ile-graphrag-denemem-54d461d4ff5a)
- M13: [Agentic GraphRAG and an explorer for your own stack](https://medium.com/neo4j/agentic-graphrag-and-an-explorer-for-your-own-stack-7fd528f1de23)
- M14: [Your Vector Database Doesn’t Understand Your Business. NVIDIA Wants to Fix That.](https://medium.com/data-science-collective/your-vector-database-doesnt-understand-your-business-nvidia-wants-to-fix-that-8dad7940b533)
- M15: [Naive vs. Adaptive RAG: Working Python Examples for a Small Business](https://medium.com/@dilipnswamy/naive-vs-adaptive-rag-working-python-examples-for-a-small-business-4c39fbe77b9c)
- M16: [Multi query retrival in rag](https://medium.com/@jhaaayush677/multi-query-retrival-in-rag-6bddf7d958c2)
- M17: [Query rewriting in rag](https://medium.com/@jhaaayush677/query-rewriting-in-rag-bc8c47769018)
- M18: [From PDFs to AI Answers: Building My First RAG System with Python](https://medium.com/@annareddyhimajareddy/from-pdfs-to-ai-answers-building-my-first-rag-system-with-python-87561bca71bb)
- M19: [Production-Ready RAG: Run Local or Cloud LLMs — An Advanced Engineering Guide](https://medium.com/@dilipnswamy/production-ready-rag-run-local-or-cloud-llms-an-advanced-engineering-guide-497e46a28ad4)
- M20: [Understanding RAG and Fine-Tuning for Local AI Systems](https://medium.com/@bitmaster.sg/understanding-rag-and-fine-tuning-for-local-ai-systems-5b3fb08365d6)
- M21: [Production Architecture and Failure Modes: Building a RAG System That Doesn’t Fall Over](https://anilpise7.medium.com/production-architecture-and-failure-modes-building-a-rag-system-that-doesnt-fall-over-e569bc89f469)

The manifest lists nine further entries. No row depends on them.

This revision read the manifest, both longer texts and the 28 excerpts as captured; the stage-0 audit and the hybrid plan; the hybrid and agentic ADRs; the opening sections of the evidence/diagnostics ADR; and the service ADR's revision header, model-judged track and Consequences. In current source it read `search` in `src/search/mod.rs`, `src/search/tokenize.rs`, `src/search/bm25.rs`, the head of `src/search/ppr.rs` and `src/search/dense.rs`, `MissStage`, `misses` and `attribute_miss` in `src/eval/sweep.rs`, `IndexConfig` in `src/store.rs`, `Cargo.toml` and `.gitignore`. Statements about `verify`, `read`, the server instructions and the tool surface come from the agentic ADR and `project_architecture.md`; `src/cli.rs`, `src/mcp.rs` and `src/cite.rs` were not opened. Revision 3 also read `SearchIndex::indexes` in `src/search/mod.rs`, `rank` and `System` in `src/eval/run.rs`, and the per-system stage counts in the dev run's `misses.json`. A search of `src/` and `tests/` found one reference to `dense` outside `src/search/dense.rs`: the `pub mod dense;` declaration in `src/search/mod.rs`.

It also read `project_plan-current.md`, which summarizes the held-out H2 and H5 intervals. No held-out query, label, ranking or result file and no held-out audit was opened, and no proposal here uses those intervals. No dev query or qrel file was opened; the dev figures are the stage-0 audit's aggregates.

No build, test, lint or benchmark ran for this revision. A script checked the final newline, the single title, trailing whitespace, local link targets and anchors, and column counts in every table; `git diff --check` exited 0.
