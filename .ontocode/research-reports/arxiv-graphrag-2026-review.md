# Graph RAG on arXiv, 2026: per-paper review

Status: COMPLETE. Reviewed: lines 1-271 of 271. Tier B is not reviewed.

Source set: the 271 tier A papers listed in
[arxiv-graphrag-2026.md](arxiv-graphrag-2026.md). Line numbers below are line
numbers in `tmp/arxiv/graphrag-2026/metadata.jsonl`, which follows the tier A
table order of that index (published date, then arXiv id).

## Summary

Many 2026 papers depart from the best-known graph RAG design: an
entity-relation graph extracted by an LLM, clustered into communities and
summarized. This review holds no sample from earlier years, so it describes
what the 2026 papers do and shows no trend. Four directions recur. Numbers in
brackets are entry numbers in the per-paper section.

1. **Cheaper graphs.** Many papers drop LLM triple extraction, and 63 entries
   call no LLM at all when the index is built. The graph comes from document
   structure and links [19, 129, 184, 200], from a parser [8, 218, 225], from
   co-occurrence of tokens, sentences or training passages
   [150, 186, 215, 238], or from passage similarity [229]. One of these
   still uses an LLM for another indexing step [186].
2. **The graph navigates; the source text is the evidence.** A graph built by
   extraction drops qualifiers, negation and uncertainty, so several papers
   use it only to find passages [17, 36, 137, 170, 214, 226].
3. **Control moves to the query.** Routers, per-query policies and agents
   decide whether and how to use the graph [23, 67, 156, 223, 235, 266].
4. **The claims are being tested.** A body of negative results shows that
   graph RAG does not reliably beat plain retrieval, and nine papers treat
   the graph as an attack surface. In almost every cited comparison the
   plain arm is dense retrieval, not BM25.

Most headline results are the authors' own, on small or sampled test sets,
often scored by an LLM judge. Nothing here was reproduced.

The findings are consolidated into one proposed design in
[graphrag-2026-ideal-architecture.md](graphrag-2026-ideal-architecture.md).

## Counts

The counts were computed from this file by a read-only script and sum to 271.
The LLM counts were then adjusted by hand on 2026-11-15 for three corrected
entries: 19 and 184 (index-time LLM use not stated in the pages read) and 86
(an LLM reranks at query time).

| Kind | Entries |
|---|---|
| method (includes one theory paper, entry 152) | 129 |
| application | 62 |
| tangential | 26 |
| system | 22 |
| benchmark | 21 |
| attack-or-defence | 9 |
| survey | 2 |

| Novelty | Entries | Meaning |
|---|---|---|
| 3 | 27 | rethinks the pipeline or contradicts common practice |
| 2 | 131 | a distinct new mechanism in one stage |
| 1 | 85 | incremental change to one step |
| 0 | 28 | existing tool, standard pipeline, or no graph RAG contribution |

| LLM use | yes | no | not stated or not applicable |
|---|---|---|---|
| when the index is built | 146 | 63 | 62 |
| during retrieval | 151 | 90 | 30 |

31 entries use an LLM in neither stage. Read the "no" column with care: it
includes papers that query an existing curated knowledge graph and some
tangential papers, and many of these methods still need an embedding model.

## Approaches by design choice

The lists give examples from the entries; they are not exhaustive.

### What a node is

| Graph unit | How it works | Examples |
|---|---|---|
| Extracted entities and relations, often with community summaries | The default lineage of Microsoft GraphRAG, LightRAG and HippoRAG | 10, 30, 43, 46, 63, 79, 87, 109, 121, 166, 177, 211 |
| Passages or chunks | Passages are nodes; edges come from similarity, shared entities or supported transitions | 214, 215, 219, 229 |
| Sentences, propositions, atoms | A unit smaller than a chunk carries one fact and is retrieved on its own | 5, 27, 31, 127, 140, 238, 243 |
| Tokens, meanings and mentions | Corpus tokens, meaning nodes or named entities, joined only by co-occurrence | 58, 84, 150, 186 |
| Document structure | Sections, clauses, headings, cross-references and hyperlinks | 19, 80, 89, 94, 128, 129, 149, 162, 184, 200, 255 |
| Hyperedges | One edge joins many entities, or one passage's entities | 13, 32, 85, 102, 127, 138, 157, 167, 235, 259 |
| Hierarchies and trees | Encoding trees, k-core layers, category layers, induced type hierarchies | 6, 25, 33, 46, 94, 141, 231, 271 |
| Causal, event and temporal units | Causal variables, timeline entries, facts with validity intervals | 20, 130, 137, 160, 163, 195, 220, 230 |
| Parsed code and typed engineering models | Syntax trees, power-grid models, piping diagrams, SQL logs | 8, 14, 61, 133, 218, 225, 257 |
| Multimodal evidence | Page regions, images, tables, video and audio segments | 148, 167, 173, 175, 205, 236, 246, 264, 265 |
| No corpus graph | The structure lives in the query, in working state, or in model weights | 28, 149, 207, 213, 228 |

### How the graph is built

| Construction | Examples |
|---|---|
| LLM extraction of triples | most of the 146 entries that call an LLM at index time |
| Deterministic, from structure, parsing or co-occurrence | 8, 58, 84, 85, 129, 150, 215, 218, 219, 225, 229, 238 |
| Existing curated graph, no construction | 16, 35, 49, 70, 179, 216, 233, 256, 266 |
| LLM extraction with a cheaper step swapped in | 37 (graph only over often-cited chunks), 46 (k-core in place of Leiden), 94 (structural entropy in place of LLM clustering), 116 (a GNN decides where the LLM should look), 186 (co-occurrence in place of extracted relations) |
| Construction quality and governance | 111, 120, 123, 166, 191, 197, 246, 260, 271 |

### How retrieval walks the graph

| Retrieval | Examples |
|---|---|
| Personalized PageRank | 17, 23, 31, 79, 85, 122, 148, 150, 157, 168, 186, 209, 215, 238, 257, 259, 265, 270 |
| Other diffusion or spreading activation | 87, 140, 151, 229, 232 |
| Bounded k-hop or breadth-first expansion | 56, 77, 108, 110, 137, 145, 154, 160, 214, 219, 240, 242 |
| Path search and beam search | 7, 10, 75, 78, 92, 144, 179, 204 |
| Subgraph chosen by optimization (Steiner tree, spanning tree, coverage, information bottleneck) | 49, 62, 118, 226, 227, 233, 253, 256 |
| Generated or templated Cypher, SPARQL or SQL | 16, 39, 45, 60, 61, 68, 70, 115, 135, 147, 174, 216, 261, 266 |
| Agent explores with tools | 29, 42, 81, 84, 104, 125, 129, 139, 149, 167, 184, 217, 267 |
| Router or per-query policy | 20, 23, 63, 67, 68, 128, 156, 162, 176, 183, 223, 235 |
| Learned retriever (GNN, graph foundation model, reinforcement learning) | 6, 13, 49, 62, 78, 98, 112, 159, 164, 181, 206, 256, 258 |
| Lexical and dense fusion with the graph as a side signal | 19, 33, 65, 89, 154, 160, 176, 192, 195, 255 |

### Where the evidence comes from and how it reaches the model

| Choice | Examples |
|---|---|
| Graph locates, source text answers | 17, 28, 36, 137, 168, 170, 173, 198, 205, 214, 226 |
| Provenance and admissibility checks | 200, 203, 216, 251, 261 |
| Context rendering and compression | 32, 56, 145, 218, 221, 222 |
| Graph evidence outside the prompt text | 88, 207, 248 |
| Memory and feedback into the graph | 24, 83, 98, 105, 172, 209 |

### Security, evaluation and diagnostics

| Topic | Examples |
|---|---|
| Stealing or reconstructing the graph | 12, 26, 119 |
| Defences against theft and reconstruction | 2, 268 |
| Poisoning and retrieval attacks | 52, 71, 239, 247 |
| Benchmarks and controlled comparisons | 21, 39, 47, 69, 110, 115, 145, 146, 161, 178, 182, 201, 221, 237, 241 |
| Diagnostics, calibration and explanation | 9, 90, 104, 112, 134, 143, 158, 224 |

## Smart and unusual decisions

### The 27 entries with novelty 3

**Doing without the LLM-built entity graph**

| Entry | Paper | Unusual decision | Authors' reason | Evidence and weakest point |
|---|---|---|---|---|
| 8 | 2601.08773, AST-derived graphs for code | The code graph comes from a deterministic Tree-sitter parse; no LLM at index time | Indexing takes 2.81 s against 200.14 s | 15/15 against 13/15 for the LLM-built graph; 15 questions per repository |
| 58 | 2604.16350, LiteSemRAG | Token surface forms are separated from context-specific meaning nodes | Represents polysemy without LLM parsing | Best MRR@10 on three sets with zero LLM tokens; retrieval-only evaluation |
| 150 | 2606.30093, TIGRAG | Nodes are corpus tokens; edges count sliding-window co-occurrence; query tokens seed Personalized PageRank | Replaces extracted entities and relations | Better quality at lower cost is claimed; numbers not in the pages read |
| 107 | 2605.19735, ContextRAG | Topology from fuzzy Formal Concept Analysis; a finite set of seed, soft-join and soft-meet nodes stands in for the lattice | The full lattice is too large to enumerate | 33.6% F1 on 130 tasks; no graph-free baseline on the same metric |
| 229 | 2609.19622, geometric gain graph | Passage edges reward novelty as well as similarity and penalize hubs; one diffusion step replaces Personalized PageRank | Multi-hop walks over-smooth these weights | Average F1 66.49 and 64.86; 1,000 sampled cases per benchmark |
| 28 | 2603.29875, UnWeaver | No graph: entities are a vector index and vote for their source chunks | Keeps source text and avoids graph construction | Leads on two of four datasets; LLM judge |
| 228 | 2609.19417, TrioRAG | No graph for multimodal cross-document questions; three retrievals fused by reciprocal rank fusion | Rank fusion needs no score normalization and no one signal dominates | 1.6-2.3 times faster, recall@10 87.1%; model-curated benchmark |
| 213 | 2608.30468, Hi-Q | The structure grows on the query side: a query unit is split only when evidence fails to resolve it | Avoids unsupported intermediate steps | 52.3 EM and 64.0 F1 on average; sampled questions, some baselines quoted |

**Changing what the graph is for**

| Entry | Paper | Unusual decision | Authors' reason | Evidence and weakest point |
|---|---|---|---|---|
| 170 | 2607.19301, PAGE-RAG | Text is the "evidence floor"; the graph only organizes and navigates | Graph construction omits source detail | 24 of 24 unanswerable questions refused against 4 of 24; 24 cases |
| 137 | 2607.22592, HCG-RAG | A small fixed vocabulary of causal variables; the answer model sees evidence text, never triples or node names | Exhaustive extraction is costly and large | 0.679 against 0.673 with about five times fewer nodes; parity, not a gain |
| 203 | 2608.22062, MedEventGraph-RAG | A hard wall: retrieval scores and controller state may not enter the assessor's evidence bundle | Identical bundles always get the same assessment | Balanced accuracy 78.6, 67.3, 96.8; three protocols are exploratory |
| 205 | 2608.23011, DAGC | Coarse index, fine evidence: similar adjacent chunks merge into index nodes that map back to the originals | The index adapts to redundancy and the evidence is kept | 45-47% of the nodes at about 99% of the performance; one backbone |
| 31 | 2604.20844, AtomicRAG | Atoms are the evidence unit; entity edges carry no relation label | Relation labels are error-prone | Gains on five benchmarks; numbers not in the pages read |
| 259 | 2610.35412, attention-steered hypergraph | Descriptions sit on passage-entity incidences; attention-head scores steer Personalized PageRank | The question can match a description and the reader resolves it in context | +3.5 recall@5 and +7.4 all@5; three corpora |
| 24 | 2602.04926, AutoPrunedRetriever | One symbolic codebook is both the retrieval index and persistent memory | Earlier reasoning is reused | 1.8-10.8 points with up to 96% fewer input tokens; two benchmarks are the authors' own |
| 167 | 2607.12764, EvoGraph-R1 | The agent may edit the hypergraph during retrieval; visual facts hang off an image anchor node | Visual facts stay grounded in their image | F1 68.5 on 2Wiki; four A100 GPUs, cost not read |
| 266 | 2611.06481, ArcRAG | The retriever induces a query-local interface from whatever schema the graph has | One retriever serves graphs built in different ways | +12-39 points over generator-only ablations; 100-question ablations |
| 163 | 2607.10265, TGMS | A bi-temporal graph (valid time and transaction time); operators report evidence completeness | Arithmetic over a truncated result yields a plausible wrong answer | 0.408 against 0.106 for vector RAG; benchmark and workload co-designed |

**Moving the graph out of the prompt text**

| Entry | Paper | Unusual decision | Authors' reason | Evidence and weakest point |
|---|---|---|---|---|
| 207 | 2608.25489, parametric graph memory | One LoRA adapter per entity; no subgraph text in the context | The graph becomes parametric memory | +0.243 exact match on single-valued relations; 150 entities; retrieving substitute adapters by similarity failed |
| 248 | 2610.22818, FuseRAG | Reusable evidence stays in latent memory; only selected facts are written as text | No single text order suits both graph locality and cache reuse | Up to 2.7 times faster time to first token; no per-dataset scores read |
| 232 | 2609.24346, LADDER | Retrieval is triggered by entities detected during parallel diffusion decoding; the entity set only grows | A mention that disappears and returns must not refresh twice | EM 45.2% against 39.6%, 2.79 s against 11.30 s; MuSiQue F1 below the best baseline |
| 218 | 2609.02011, seed-anchored rendering | Fixed render order: seed descriptions and their edges first, then outward by hop | Answer-bearing units survive truncation when the seed-local text fits | 0.450 to 0.970 at 8,000 characters; one topology family, one reader |

**Results that contradict common practice**

| Entry | Paper | Unusual decision | Finding | Weakest point |
|---|---|---|---|---|
| 178 | 2607.26497, BM25 wins at scale | Evidence is held fixed while the corpus grows through 28 sizes | BM25 leads at 601M tokens; graph systems finish fewer size tiers | Graph systems are compared only on tiers they finished |
| 221 | 2609.10237, answer paths | Subgraphs come from gold paths, so serialization is studied without a retriever | Context precision from 1.00 to 0.51 changed F1 by +0.003; removing the answer path caused large losses | Reasoning depth is aliased with the benchmark |
| 69 | 2604.09666, RAGSearch | All backends run under one agent protocol and matched budgets | Agentic search lifts dense RAG, but graph backends keep an advantage on multi-hop tasks | Full tables beyond the pages read |
| 152 | 2606.30247, soft grounding theory | A finite slack keeps unsupported but uncontradicted reasoning alive | Hard grounding cannot handle missing evidence | Proofs only, no experiment |

**Security**

| Entry | Paper | Unusual decision | Authors' reason | Evidence and weakest point |
|---|---|---|---|---|
| 2 | 2601.00274, AURA | The owner adulterates its own graph with false triples that a secret key filters out; nodes are chosen by minimum vertex cover | Vertex cover touches every relation; centrality does not guarantee that | Unauthorized accuracy 5.3%, authorized fidelity 100%; assumes key and tags stay secret |

### Smaller decisions worth noting

These come from entries scored 0 to 2. Each is one well-reasoned choice
inside an otherwise ordinary pipeline.

| Theme | Entry | Decision | Authors' reason |
|---|---|---|---|
| Bounded walks | 17 | Escalate from local expansion to bridge discovery to Personalized PageRank only as needed | Most queries stop early; evidence is then recovered from source text |
| Bounded walks | 57 | A binding propagates to the next hop only when candidate scores are concentrated | Uncertain matches must not become false chains |
| Bounded walks | 151 | A monotonic activation rule | Fixed-depth propagation fits in one database query with no cyclic reinforcement |
| Bounded walks | 144 | A learned heuristic may only reorder work inside a corridor set by exact distances | Search is cut while feasibility guarantees hold |
| Bounded walks | 75 | Reachability is precomputed by hop and entity type | Online traversal is avoided at the cost of space |
| Hub control | 215 | Edges are filtered by a structural profile of their endpoints before PageRank | Blocks diffusion into generic hubs |
| Hub control | 122 | Propagation never starts from high-degree type nodes | Generic types spread noise |
| Hub control | 140 | Entity-passage links are weighted by term frequency in the passage | Discounts incidental mentions |
| Hub control | 55 | Similarity links exist only as temporary edges during one query | Permanent densification causes semantic drift |
| Ranking | 84 | Candidate discovery is separate from ranking; ranking is plain similarity | Composite scores made results worse |
| Ranking | 86 | Rankings are fused, not raw scores | The two scores are on incommensurable scales |
| Ranking | 36 | A quota is reserved for passages localized from graph paths; dense retrieval fills the rest | Balances precision and recall |
| Ranking | 93 | The retriever returns convergence nodes, not paths | Adding the paths lowered accuracy |
| Ranking | 256 | The generator accepts disconnected evidence | Retrieval need not add irrelevant bridge nodes to stay connected |
| Structure | 46 | k-core, not k-truss, for a deterministic hierarchy | Knowledge graphs have few triangles |
| Structure | 80 | `SUPERSEDES` and `REFERS_TO` are explicit edge types | Traversal resolves which version prevails |
| Structure | 195 | A relation supersedes an older one by document order, never by a date the model supplied | Model-supplied dates are unsupported |
| Structure | 160 | A reader-position cutoff returns only evidence up to the requested chapter | Prevents leaking later content |
| Structure | 129 | The graph is only document structure and hyperlinks | No extraction is needed |
| Structure | 270 | Synonym edges use a threshold, not a top-k list | Deleting a neighbor cannot change the remaining links, so deletion is exact |
| Evidence | 200 | A citation passes only if its quoted span matches the source after whitespace normalization | Translation cannot silently alter evidence |
| Evidence | 216 | The embedding fallback may only find anchors; evidence is admitted after a symbolic query executes | Keeps traceability |
| Evidence | 251 | An unknown condition counts as neither support nor contradiction | Keeps partial observability honest |
| Evidence | 261 | Duplicate triples are kept as separate occurrences | Each keeps its own source span |
| Evidence | 81 | Identities join on exact normalized name keys, never by fuzzy matching | Avoids merging borderline identities |
| Cost | 56 | Context is compressed by graph proximity with no LLM call | An 8B model with compression reaches 55.8% on 2Wiki against 48.8% for a plain 70B model |
| Cost | 116 | A GNN predicts that a region is missing something, not which edge | Decides where to spend LLM effort |
| Cost | 131 | Graph queries are deferred until a failure | The mandatory query overhead was the reliability cost |
| Cost | 70 | Prewritten query templates are executed; none is generated at runtime | Prevents hallucinated retrieval steps |
| Evaluation | 219 | Plain RAG and graph RAG get the same candidate budget | Isolates the retrieval effect |
| Evaluation | 111 | Graph fidelity is measured by direct graph access | The usual retrieval path masks construction errors |
| Evaluation | 143 | Uncertainty is measured separately for answer, evidence and retrieval state | Agreement among sampled answers hides stable retrieval errors |
| Evaluation | 227 | A graph built with knowledge of the benchmark is used as a declared, non-deployable upper bound | Bounds what a better graph could achieve; oracle retrieval then shows extraction is the limit |
| Evaluation | 106, 192 | Edges to future papers and own-citation leakage are removed | Prevents the graph from leaking the label |
| Attack | 239 | The attack edits summaries, hierarchy edges and scores added after indexing | One edit affects many queries and no source document changes |
| Defence | 268 | The privacy mechanism includes a null outcome | One mechanism protects both a relation's meaning and its existence |

## Negative and sobering results

| Entry | Paper | Result as stated | Scale |
|---|---|---|---|
| 178 | 2607.26497 | BM25 leads at the full 601M tokens; graph systems finish fewer size tiers | 500 questions |
| 126 | 2606.04646, QO-Bench | GraphRAG recall 3.8% in global mode and 0.9% in local mode under exact tuple matching | 785 questions, one domain |
| 161 | 2608.28607 | Graph-RAG macro-F1 0.663 against 0.830 for a flat LLM | 101 pairs |
| 66 | 2604.04948 | Well-prepared plain RAG 94.1% against 82% for GraphRAG | 50 questions |
| 145 | 2606.25656 | Documents plus one hop reach Hit@1 0.6972 against 0.6422 for a hybrid graph pipeline, with LLM answer generation included; retrieval-only Hit@1 is 0.5596 and document-only 0.4404, and every arm retrieves densely | one dataset |
| 149 | 2606.29399 | An agent browsing the document tree reaches 81.5% against 49.5% for GraphRAG; edge inference costs 2.8 times more with no accuracy gain | 200 questions |
| 45 | 2603.13307 | A fine-tuned model scores 2.92/3 against 2.62/3 for runtime GraphRAG | 100 questions |
| 177 | 2607.24984 | GraphRAG and the vector baseline are statistically indistinguishable overall | 1,500 answers |
| 221 | 2609.10237 | Context precision has no measured effect when the answer path is present | 30,841 trials |
| 243 | 2610.13128 | The authors' own ablation finds graph expansion neutral | three benchmarks |
| 19 | 2604.09868 | Keeping structure helps precision and MRR; neighbour expansion and reranking were ineffective | 800+ questions, own set |
| 132 | 2607.19362 | Plain nearest-neighbour graphs had the strongest overall win rates across constructions | two datasets, LLM judge |
| 110, 182 | 2608.05153, 2608.00705 | GraphRAG context precision is 0.12-0.23, yet it wins on answer-citation F1; the metric choice reverses the ranking | two corpora |
| 21 | 2602.02053 | Naive RAG leads on summary F1 (15.84) on noisy cited web sources | 1,100 questions |
| 106 | 2605.18410 | Prompts with only the target paper often equal or beat prompts with graph context | one journal |
| 51 | 2603.20252 | Adding extracted triplets hurt most hallucination detectors | 755 examples |
| 93 | 2605.06403 | Adding paths to the returned nodes lowered accuracy | two datasets |
| 84 | 2604.17056 | Composite ranking scores made results worse than plain similarity | 519 questions |
| 115 | 2605.27164 | Contrastive alignment of the semantic and symbolic graph views gave no measurable gain | 117 questions |
| 207 | 2608.25489 | Retrieving substitute adapters by embedding or weight similarity failed | 150 entities |
| 224 | 2609.13324 | 20% injected graph defects cut accuracy from 0.93 to 0.39-0.62, and model knowledge can mask defects | one domain |
| 237 | 2609.36700 | Mean F1 falls 11.7% to 16.3% when the same question arrives over several turns | 750 questions |
| 199 | 2608.16096 | The paper's own estimate of GraphRAG indexing cost is $19.49 to $38.99; the range $2.30 to $24.94 per 5.64 MB corpus is quoted from KET-RAG | cost audit |
| 192 | 2609.26237 | Citation-graph features add 0.014 F1 once own-citation leakage is removed | eight seeds |
| 29 | 2602.08400 | Distributed traversal scores 56 against 63 for the centralized system, at under a quarter of the tokens | 89 queries |
| 95, 160, 220 | 2607.16208, 2607.05577, 2609.05139 | The authors report that at least one headline comparison is not statistically significant | p = 0.054 and 0.092 where stated |

Two results point the other way and deserve the same weight: under one agent
protocol, graph backends keep an advantage on multi-hop tasks [69], and
GraphRAG wins every MuSiQue stratum on answer-citation F1 [182].

## Recurring evidence weaknesses

The counts are keyword counts over the Evidence lines of this file, so they
are lower bounds.

| Weakness | Entries | Examples |
|---|---|---|
| An LLM judge scores the answers | at least 26 | 20, 28, 30, 45, 121, 140, 188, 240 |
| Headline numbers were not in the pages read | 21 | 31, 49, 138, 150, 258, 263 |
| A 1,000-item sample replaces the full benchmark | at least 11 | 55, 57, 67, 215, 229, 239 |
| Very small test sets | many | 10 questions [4, 77], 15 [8], 19 [61], 20 [80, 262], 22 [211], 30 [104, 105], 50 [66, 176] |
| The benchmark is the authors' own | many | 1, 19, 24, 101, 128, 163, 222, 252 |
| One domain, corpus or repository | many | 81, 145, 162, 200, 225, 255 |
| Baselines quoted or not rerun | several | 213, 214, 223, 242 |

## Tangential papers

These 26 papers match the tier A selection rule but make no graph RAG
contribution, or use graph RAG only as a baseline or example.

| Entry | arXiv id | What the paper is |
|---|---|---|
| 3 | 2601.11587 | multi-agent urban carbon planning; states it works without GraphRAG |
| 44 | 2603.03175 | formal-verification agents |
| 50 | 2603.08965 | continuous scale selection for graph hierarchies; no retrieval evaluation |
| 53 | 2603.11798 | relational-table document question answering |
| 54 | 2603.13185 | scene graphs from video |
| 66 | 2604.04948 | PDF conversion pipelines for plain RAG |
| 73 | 2604.09682 | safety of O-RAN agents |
| 76 | 2604.05846 | reinforcement learning for graph learning tasks |
| 97 | 2605.10247 | a graph-language model |
| 101 | 2605.14177 | prospection-guided memory retrieval |
| 103 | 2605.14362 | repository file filtering |
| 106 | 2605.18410 | paper impact prediction |
| 113 | 2605.24366 | tables as the retrieval structure |
| 114 | 2605.25480 | an LLM-compiled wiki navigated by an agent |
| 120 | 2605.29168 | ontology repair during graph construction; no retrieval |
| 126 | 2606.04646 | query-operator benchmark |
| 142 | 2606.22180 | distributed graph embedding training |
| 169 | 2607.15516 | prompt caching and compression costs |
| 187 | 2608.09016 | graph pretraining for node ranking |
| 194 | 2608.13706 | claim-level verification for finance |
| 199 | 2608.16096 | licence and cost audit of retrieval benchmarks |
| 202 | 2608.18952 | editable memory for recommendation |
| 212 | 2609.00050 | cloud workflow orchestration |
| 244 | 2610.13296 | hierarchical section retrieval with per-section metadata written by an LLM |
| 252 | 2610.30611 | sequential diagnosis over hypothesis graphs |
| 263 | 2610.37826 | self-contained sub-questions for multi-hop questions |

Five of them still carry useful data points for graph RAG: 66, 106, 126 and
199 (see the negative-results table) and 244 (section retrieval with
per-section metadata).

## Ideas that fit OntoDoc's constraints

This is an idea list only. Per `AGENTS.md`, donor inspection is not evidence
of retrieval improvement, no feature is approved by this report, and any
ranking change needs its own dev-split evidence and preregistration. No paper
code was inspected or copied.

OntoDoc forbids an LLM, an embedding model and network access on the
retrieval path, and requires deterministic and bounded work. Most "LLM-free"
papers above still use embeddings, so only the parts below carry over.

| Idea | Source entries | Why it fits | Caution from the papers |
|---|---|---|---|
| A graph of document structure and resolved links, with no extraction | 19, 80, 129, 184, 200 | OntoDoc already builds this graph at index time from headings and resolved links (`src/graph.rs`) | Structure helped precision, but neighbour expansion did not [19]; `AGENTS.md` forbids deriving qrels from links |
| Hub control before or during Personalized PageRank | 122, 140, 215, 229 | OntoDoc's PageRank already divides each chunk's mass by its degree (`src/search/ppr.rs`); no paper tests more than that on a lexical arm | Each was tuned on question-answering sets with embeddings in the loop |
| Fixed-depth, monotonic propagation | 151 | OntoDoc's PageRank stops at 100 iterations or a change below 1e-9 | One reproduction-based comparison |
| Rank fusion, not score fusion; discovery separate from ranking | 84, 86, 228 | OntoDoc fuses min-max scaled scores (`src/search/fusion.rs`); no paper compares the two rules on OntoDoc's arms | Composite scores hurt in [84] |
| Deterministic render order that protects seed-local evidence under a byte budget | 218, 221 | OntoDoc's assembly is already rank-ordered and budgeted, with explicit truncation | One topology family [218] |
| Completeness reported with every bounded operator | 163 | OntoDoc's bounded responses already carry truncation flags (`README.md`, truncation contract) | Co-designed benchmark |
| Exact-quote citation checks against current bytes | 200 | OntoDoc's `verify` already compares an exact byte hash of the current file | One statute |
| Equal candidate budgets across compared arms; no label leakage through edges | 106, 192, 219 | Protocol discipline | None |
| Treat auxiliary index structures as an attack surface | 239, 247 | Supports keeping index writes in the trusted control plane | Entry 239 proposes defences as well, and its attack needs write access to the store; 247 is an attack paper |
| A token co-occurrence graph as a lexical alternative to entity graphs | 150 | OntoDoc has no such graph; this stays an idea and fits no existing mechanism | Numbers not in the pages read; query-time LLM use is reported |
| Keep BM25 as the baseline to beat | 178 | BM25 led at the largest corpus size | Graph systems were compared only on tiers they finished |

## Method and limits

- Each paper was read by a `gpt-6-luna` explorer sub-agent from the PDF with
  `pdftotext` (pages 1-10, pages 11-16 when the method section was cut off).
  The coordinator transcribed the entries; it did not re-read the PDFs.
- Results are the authors' claims. Nothing was reproduced.
- Novelty scores are a model's judgement against the scale below, not a
  citation or peer-review measure.
- Tier B (158 papers) is not reviewed.
- A read-only script check of this file found 271 entries numbered 1 to 271
  with no gap or duplicate, each with the six bullets in order, and each
  arXiv id equal to the id on the same line of the metadata file. The check
  ran before the consolidated sections were added and was not repeated.
- The same check found the PDF at the metadata version for 270 of 271 papers.
  The exception is `2608.01269`: the metadata names v2 and only
  `2608.01269v1.pdf` is present, so entry 183 was read as v1.
- Four explorers reported "fallback" files for `2609.01617`, `2607.00508`,
  `2607.20506`, `2608.25952`, `2610.26432` and `2610.33277`. The check shows
  that for each of these the one PDF present is the metadata version, and no
  id has two PDFs. Those papers were therefore read at the intended version.
- The coordinator changed some explorer labels, to score the graph RAG
  contribution only. Novelty: 54 (3 to 0), 66 (3 to 1), 145 (1 to 2),
  155 (2 to 1), 159 (3 to 2), 160 (3 to 2), 217 (2 to 1). Kind: 221 (method to
  benchmark), 228 (attack-or-defence to method), 260 (tangential to method).
  Entries 1-10 were scored by the coordinator.
- The groupings in the consolidated sections come from the coordinator's
  reading of the entries. They are examples, not complete lists.
- Paper text was treated as untrusted data.

## Entry legend

- **Kind**: method, system, benchmark, attack-or-defence, survey, application,
  tangential (graph RAG is only mentioned or used as a baseline).
- **LLM**: whether an LLM is called when the index is built, and whether one is
  called during retrieval. Generating the final answer does not count.
- **Decision**: the most non-obvious design choice, with the authors' reason.
- **Novelty**: 0 = existing tool or standard pipeline; 1 = incremental change
  to one step; 2 = a distinct new mechanism in one stage; 3 = rethinks the
  pipeline or contradicts common practice.
- **Evidence**: datasets and headline result as stated, then the weakest point.

## Per-paper entries

Entries 1-10 came from the first slice in an older format; the coordinator
assigned their novelty scores from the returned text.

### 1. 2601.00216 — SR-RAG: sports rehabilitation graph RAG
- Kind: method
- Approach: LLM-extracted knowledge graph with a PICO schema (population, intervention, comparison, outcome) for sports rehabilitation. Retrieval generates a PICO-guided hypothetical document, then ranks with ColBERT and a cross-encoder.
- LLM: index = yes; query retrieval = yes.
- Decision: Bayesian Evidence Tier Reranking learns a bias per evidence grade from pairwise preferences, so no preset weights are needed (p. 4).
- Novelty: 1 — domain schema plus a learned evidence-grade prior.
- Evidence: own benchmark of 1,637 QA pairs; R@10 0.812, faithfulness 0.819 (Table 2). Caveat: own benchmark only.

### 2. 2601.00274 — AURA: adulterating a knowledge graph against theft
- Kind: attack-or-defence
- Approach: The owner injects plausible false triples and nodes into a proprietary graph, so a stolen copy answers wrongly. The owner filters them at query time through encrypted metadata tags and a secret key.
- LLM: index = yes; query retrieval = no.
- Decision: nodes to adulterate are chosen by minimum vertex cover (exact ILP up to about 150k nodes, heuristic above), so every relation is covered, which centrality-based choice does not guarantee (p. 5).
- Novelty: 3 — degrades the graph on purpose as IP protection.
- Evidence: MetaQA, WebQSP, FB15k-237, HotpotQA; unauthorized accuracy 5.3%, authorized fidelity 100%, 80.2% of adulterants survive sanitization (abstract). Caveat: threat model assumes the key and tags stay secret.

### 3. 2601.11587 — Multi-agent urban carbon planning
- Kind: tangential
- Approach: Four planning agents over plain text RAG; the paper states it works "without GraphRAG".
- LLM: index = not stated; query retrieval = not stated.
- Decision: none: standard pipeline.
- Novelty: 0 — false positive of the tier A selection rule.
- Evidence: not relevant to graph RAG. Caveat: none seen.

### 4. 2601.02814 — CausalAgent: evidence-linked causal graphs
- Kind: application
- Approach: LightRAG index over dementia and exercise abstracts. The agent must retrieve before making any causal claim and emits causal DAGs whose edges link to evidence.
- LLM: index = yes; query retrieval = yes.
- Decision: an evidence-first protocol forbids causal statements without a prior retrieval call (p. 2).
- Novelty: 1 — protocol on top of an existing graph index.
- Evidence: 234 abstracts, 10 questions; 95% accuracy against 34% for the baseline. Caveat: ten test questions.

### 5. 2601.03014 — SentGraph: sentence graph with discourse edges
- Kind: method
- Approach: Three-layer graph of topic entities, core sentences and supplementary sentences. Edges are Rhetorical Structure Theory discourse relations plus cross-document topic bridges, built offline by an LLM. Online: anchor selection, then breadth-first path expansion.
- LLM: index = yes; query retrieval = not stated.
- Decision: the three layers avoid comparing all sentence pairs (p. 3).
- Novelty: 2 — sentences as nodes, discourse relations as edges.
- Evidence: HotpotQA EM 48.80, 2WikiMultiHopQA 42.00, MuSiQue 26.80 (Table 1). Caveat: index cost not reported in the pages read.

### 6. 2601.04945 — T-Retriever: entropy-built encoding tree
- Kind: method
- Approach: Builds an encoding tree over a textual graph by top-down partitioning. An LLM summarizes tree nodes, query embeddings select tree nodes, and the induced subgraphs pass through a GNN to the LLM.
- LLM: index = yes; query retrieval = no.
- Decision: partitions minimize a joint structural and semantic entropy, replacing bottom-up hierarchies with fixed quotas per level (p. 2).
- Novelty: 2 — information-theoretic objective for the hierarchy.
- Evidence: SceneGraphs 0.8507, WebQSP 0.7715, BookGraphs 0.7579 (Table 1). Caveat: needs GNN training.

### 7. 2601.07192 — Relink: query-specific evidence graph built on the fly
- Kind: method
- Approach: Combines an LLM-extracted graph with a "latent relation pool" of co-occurring entity pairs and their source sentences. Beam search with a trained ranker and an LLM assembles an evidence graph per query.
- LLM: index = yes; query retrieval = yes.
- Decision: explicit facts and latent co-occurrence relations are ranked in one space, so query utility decides, not prior graph membership (p. 2).
- Novelty: 2 — defers relation commitment to query time.
- Evidence: five QA sets; average +5.4% EM and +5.2% F1. Caveat: query-time LLM cost not reported in the pages read.

### 8. 2601.08773 — AST-derived graphs for code RAG
- Kind: benchmark
- Approach: Deterministic Tree-sitter graph with Java types as nodes and `injects`, `extends`, `implements` edges, compared against an LLM-extracted graph and no graph.
- LLM: index = no; query retrieval = not stated.
- Decision: bidirectional traversal plus expansion from an interface to its consumers (p. 7).
- Novelty: 3 — no LLM at index time, and it wins.
- Evidence: Shopizer, 15/15 questions against 13/15 for the LLM graph and 6/15 without a graph; indexing 2.81 s against 200.14 s. Caveat: 15 questions per repository.

### 9. 2601.09241 — Ca2KG: causal calibration of KG-RAG confidence
- Kind: method
- Approach: Calibrates answer confidence with counterfactual prompt interventions (degraded path quality, flawed reasoning). A panel LLM re-scores answers under each intervention.
- LLM: index = not stated; query retrieval = yes.
- Decision: a "Causal Calibration Index" combines mean support and stability across interventions (p. 5).
- Novelty: 2 — confidence from stability under counterfactual retrieval.
- Evidence: MetaQA, WebQSP; lower ECE and Brier score (section 5.1.1). Caveat: several extra LLM calls per answer.

### 10. 2601.11144 — Deep GraphRAG: hierarchical beam search
- Kind: method
- Approach: LLM-extracted graph, weighted Louvain, three-level community hierarchy. Top-down beam search runs from communities to entities.
- LLM: index = yes; query retrieval = yes.
- Decision: an entity description is concatenated with its parent community representation before scoring (p. 2); a 1.5B integrator model trained with "DW-GRPO" reaches over 94% of the 72B result.
- Novelty: 1 — beam search over the usual community hierarchy.
- Evidence: NQ EM 44.69%, HotpotQA 45.44% (Table 1). Caveat: none seen.

### 11. 2602.00020 — Adaptive exercise generation
- Kind: application
- Approach: Auto-HKG uses LLMs to build a hierarchical graph from exercises and textbooks, with question, category, concept and method nodes joined by relations such as prerequisites. CG-RAG traverses prerequisite, peer or successor concepts according to learner mastery, then ranks and generates exercises.
- LLM: index = yes; query retrieval = yes.
- Decision: mastery thresholds choose between prerequisite, peer and successor concepts, so retrieval follows the learner's state (p. 3).
- Novelty: 1 — learner-state routing on a conventional graph RAG pipeline.
- Evidence: 1,007 math exercises; 75% coarse-category and 90% sampled concept extraction accuracy; mastery rose to 40.2% after three rounds (sections 3.1-3.3). Caveat: no controlled comparison reported.

### 12. 2601.14662 — AGEA: agentic graph extraction attack
- Kind: attack-or-defence
- Approach: Attacks a black-box GraphRAG system to reconstruct its entity-relation graph. Regex extraction proposes candidate nodes and edges, an LLM filters them, and novelty-guided exploration alternates with hub-focused exploitation (pp. 3-4).
- LLM: index = yes; query retrieval = yes.
- Decision: novelty is computed before filtering, so denoising does not suppress the exploration signal (p. 4).
- Novelty: 2 — budgeted graph reconstruction as an adaptive attack.
- Evidence: medical, agriculture and novel graphs on Microsoft GraphRAG and LightRAG; at 1,000 queries AGEA leads Table 1 in leakage and precision (p. 6). Caveat: limited to the tested systems and corpora.

### 13. 2601.17755 — Progress-aware hypergraph RAG
- Kind: method
- Approach: LLM-extracted hyperedges, each joining a set of entities, represent higher-order relations (p. 3). Retrieval reranks hyperedges by query relevance and local-versus-global incidence; training rewards each step for added coverage of answer-guided target hyperpaths (pp. 3-5).
- LLM: index = yes; query retrieval = no.
- Decision: shortest answer-guided hyperpaths give step-level supervision, rewarding retrieval progress and not only final answers (p. 5).
- Novelty: 2 — ranking and policy rewards aligned to hyperpath progress.
- Evidence: 2WikiMultiHopQA, HotpotQA, MuSiQue, NQ; average F1 59.11 against 49.89 and 63.64 against 55.94 for the strongest baseline with 3B and 7B models (Table 1). Caveat: four benchmarks, two model sizes.

### 14. 2601.18381 — Devito code translation agent
- Kind: application
- Approach: Builds a Devito graph from parsed and segmented documents and code, with relations such as MENTIONS, CALLS, INHERITS, RELATED TO and PART OF (pp. 2-3). It detects Leiden communities and retrieves by community search, concept expansion, full-text search and semantic similarity (pp. 3-4).
- LLM: index = no; query retrieval = yes.
- Decision: each node keeps links to its eight nearest embedding neighbours, which sparsifies the graph while retaining semantic links (p. 3).
- Novelty: 1 — standard graph retrieval stages for code translation.
- Evidence: 12,793 nodes, 62,362 relations, 70 communities; four-agent execution reached 5.75x throughput (pp. 3, 5). Caveat: no controlled retrieval comparison in the pages read.

### 15. 2601.18579 — GraFine: retrieval refinement on corpus graphs
- Kind: method
- Approach: Works on corpus graphs whose nodes hold text and whose edges are given relations (p. 2). It alternates semantic-guided expansion, which combines query similarity with structural signals, with graph-aware smoothing and reranking (pp. 4-5).
- LLM: index = not stated; query retrieval = no.
- Decision: cross-encoder relevance is treated as a noisy signal on the graph and smoothed over it before pruning (p. 5).
- Novelty: 2 — reranker scores smoothed over graph topology.
- Evidence: reference networks and text-rich knowledge graphs; average 9.9% Recall@10 gain and generation win rates above 55% (abstract). Caveat: benchmark sizes and table values not in the pages read.

### 16. 2603.28773 — UltRAG: universal knowledge graph RAG
- Kind: method
- Approach: Queries an existing knowledge graph of entity nodes and typed relation edges; it does not build one (p. 3). An LLM writes a structured query, a neural executor retrieves answer candidates, and an LLM arbitrates among them (pp. 4-5).
- LLM: index = no; query retrieval = yes.
- Decision: a neural executor replaces symbolic execution, because LLM-written queries and incomplete graphs are noisy and symbolic execution is brittle to that noise (p. 4).
- Novelty: 2 — neural query executor as an LLM tool.
- Evidence: GTSQA and KGQAGen-10K; F1 89.29 and 88.82 (Table 1, p. 8). Caveat: controlled subgraphs guarantee the evidence is present.

### 17. 2601.21162 — A2RAG: adaptive agentic graph retrieval
- Kind: method
- Approach: Assumes an offline entity-relation graph built from text, with a map from each entity to its source chunks (p. 2). Retrieval escalates from local expansion to bridge discovery to personalized PageRank, then maps the selected graph regions back to source text (pp. 4-5).
- LLM: index = not stated; query retrieval = yes.
- Decision: the graph is only a navigation map; evidence is recovered from source text, because graph extraction can drop qualifiers (pp. 2-3).
- Novelty: 2 — bounded escalation plus provenance-text recovery.
- Evidence: HotpotQA and 2WikiMultiHopQA; Recall@2 gains of 9.9 and 11.8 points and about 50% fewer tokens and lower latency than iterative baselines (pp. 1-2). Caveat: sampled subsets.

### 18. 2602.16715 — Graph RAG for design structure matrix generation
- Kind: application
- Approach: Tests GraphRAG as one way to generate design structure matrices for cyber-physical systems, with a conventional graph of extracted entities, relations and community summaries (pp. 5-6). The retrieval walk is not specified.
- LLM: index = yes; query retrieval = not stated.
- Decision: not stated.
- Novelty: 0 — applies graph RAG to a new task.
- Evidence: power screwdriver and CubeSat cases; headline results not in the pages read. Caveat: two case studies.

### 19. 2604.09868 — Graph retrieval for telecom standards
- Kind: method
- Approach: Nodes are document sections ("InfoUnits") with title and body; edges encode section hierarchy and citation references (pp. 2-3). Retrieval combines BM25, dense ranking and rank fusion, with optional neighbour expansion and reranking (pp. 3-4).
- LLM: index = not stated in pages 1-12; query retrieval = no.
- Decision: hierarchy and citations are represented directly, because standards are structured and cross-referential (pp. 1-2).
- Novelty: 1 — lightweight structural signals for standards.
- Evidence: own ETSI question set with 800+ valid questions; keeping structure improves precision and MRR, while expansion and reranking were ineffective (section III). Caveat: synthesized benchmark from one standards series.

### 20. 2602.01858 — SOPRAG: multi-view graph experts for procedures
- Kind: method
- Approach: Builds entity, causal and flow graphs linked to "Procedure Cards"; units are extracted entities, system states and ordered procedure steps (pp. 3-4). An LLM router weights the three experts by query intent, and retrieved flow steps guide generation (p. 5).
- LLM: index = yes; query retrieval = yes.
- Decision: Procedure Cards prune candidates before expert retrieval, as operators first scan procedure titles (p. 4).
- Novelty: 2 — separate entity, causal and flow graphs.
- Evidence: four industrial domains, 1,000 queries; best retrieval scores (Table 2) and procedure quality up to 1.00 for Data Center (Table 3). Caveat: agent-generated benchmark, LLM judge.

### 21. 2602.02053 — WildGraphBench
- Kind: benchmark
- Approach: Builds a benchmark from Wikipedia statements and the pages they cite; the benchmark defines no graph of its own. It evaluates existing GraphRAG systems.
- LLM: index = yes; query retrieval = not stated.
- Decision: the noisy cited reference pages are the corpus, to test retrieval and aggregation "in the wild" (p. 3).
- Novelty: 1 — benchmark with heterogeneous cited sources.
- Evidence: 1,100 questions; Microsoft GraphRAG global search scores 47.64 multi-fact accuracy, while naive RAG leads summary F1 at 15.84 (Table 2). Caveat: Wikipedia-derived labels and LLM judging.

### 22. 2602.02635 — Tobacco pest and disease reasoning
- Kind: application
- Approach: Directed knowledge graph from agricultural literature, guidelines and expert resources, with disease, symptom, pesticide and field-operation nodes and typed edges. TransE initializes embeddings and a graph convolutional network aggregates neighbours for subgraph retrieval.
- LLM: index = not stated; query retrieval = no.
- Decision: a two-layer GCN refines TransE embeddings so nodes carry multi-hop context (pp. 2-3).
- Novelty: 1 — TransE and GCN in a domain pipeline.
- Evidence: tobacco QA; 90.1% accuracy against 75.2% for the plain LLM (Table 1). Caveat: dataset size and splits not stated.

### 23. 2602.03578 — Adaptive routing between RAG and GraphRAG
- Kind: method
- Approach: Graph of noun-phrase and passage nodes joined by OpenIE relation, occurrence and synonymy edges. Query complexity features route each query to dense RAG, graph retrieval with personalized PageRank, or weighted rank fusion.
- LLM: index = yes; query retrieval = yes.
- Decision: syntactic features of the query predict when graph retrieval should beat plain retrieval; borderline queries go to fusion (pp. 3-4).
- Novelty: 2 — syntax-based three-way routing.
- Evidence: two single-hop and two multi-hop QA sets; result table not in the pages read. Caveat: headline figures unavailable.

### 24. 2602.04926 — AutoPrunedRetriever: persistent reasoning graphs
- Kind: method
- Approach: Encodes corpus facts and query-derived questions and answers as entity-relation edge sequences in one shared codebook. It retrieves reasoning paths by semantic and structural matching, then merges aliases and prunes redundant structure for reuse.
- LLM: index = yes; query retrieval = yes.
- Decision: one symbolic codebook serves as both retrieval index and persistent memory, so earlier reasoning is reused (pp. 2-3).
- Novelty: 3 — retrieval index unified with persistent query reasoning.
- Evidence: five complex reasoning benchmarks; 1.8-10.8 accuracy points over the strongest baseline with up to 96% fewer input tokens (Table 1). Caveat: two benchmarks built by the authors.

### 25. 2602.05143 — CausalRAG2: hierarchical causal graphs
- Kind: method
- Approach: Hierarchical graph partitioned into modules and joined by LLM-assessed "causal gates". Retrieval seeds relevant nodes, expands over structural, hierarchical and gate edges, then an LLM filters the subgraph for causal paths.
- LLM: index = yes; query retrieval = yes.
- Decision: sparse causal gates between distant modules cross module boundaries without fully connecting the graph (p. 4).
- Novelty: 2 — causal gates bridge modules.
- Evidence: five HolisQA domains and five QA datasets (Tables 3-4). Caveat: HolisQA is new, and evaluation uses an LLM.

### 26. 2602.06495 — GRASP: graph reconstruction attack
- Kind: attack-or-defence
- Approach: Targets GraphRAG systems that return entity-relation subgraphs. Multi-turn queries reconstruct a target entity's one-hop subgraph from returned relation records.
- LLM: index = not stated; query retrieval = yes.
- Decision: extraction is framed as a relation-processing task with per-record identifiers, which keeps edge identity and reduces hallucinated combinations (p. 5).
- Novelty: 2 — instance-grounded subgraph reconstruction under safeguards.
- Evidence: two knowledge graphs, four models, several frameworks; up to 82.9 reconstruction F1. Caveat: targeted one-hop extraction in selected deployments.

### 27. 2603.03296 — PlugMem: agent memory module
- Kind: application
- Approach: Turns episodic traces into semantic propositions and procedural prescriptions with provenance links. Retrieval traverses concept or intent nodes to reach knowledge units. GraphRAG appears as related work only.
- LLM: index = yes; query retrieval = yes.
- Decision: propositions and prescriptions, not entities or chunks, are the core units of the memory graph (pp. 2, 5-6).
- Novelty: 2 — memory organized around actionable knowledge units.
- Evidence: LongMemEval, HotpotQA, WebArena; improvements over task-agnostic and task-specific baselines (section 4). Caveat: three settings.

### 28. 2603.29875 — UnWeaver: entity-based retrieval without a graph
- Kind: method
- Approach: An LLM extracts entity descriptions from chunks, groups equivalent entities and links each entity to its source chunks. No explicit graph is built. Retrieval ranks entity embeddings, and entities vote for the original chunks.
- LLM: index = yes; query retrieval = no.
- Decision: source chunks are recovered through shared entities, which keeps source text and avoids graph construction (pp. 1-3).
- Novelty: 3 — vector retrieval over entities rivals graph pipelines without traversal.
- Evidence: leads on COVID-QA and eManual; HippoRAG2 leads on Tech-QA and MuSiQue (Table 1). Caveat: four datasets, LLM judge.

### 29. 2602.08400 — SCOUT-RAG: distributed traversal
- Kind: system
- Approach: Each domain is a separate private knowledge graph. Agents estimate domain relevance, choose local or global traversal, then expand depth or breadth according to answer quality and the remaining budget.
- LLM: index = not stated; query retrieval = yes.
- Decision: training-free relevance estimation and iterative quality feedback decide which domains to call (pp. 2-4).
- Novelty: 2 — distributed traversal as a budgeted multi-agent process.
- Evidence: 89 multi-domain queries over 1-40 countries; overall score 56 against 63 for centralized DRIFT, with 159,169 against 693,731 tokens (Table 1). Caveat: 89 queries, GPT-4o judge; lower quality than centralized.

### 30. 2602.08545 — DA-RAG: dynamic community search
- Kind: method
- Approach: Semantic chunk nodes, per-chunk entity and relation nodes, and an entity similarity layer. Query-guided attributed community search first selects chunks, then retrieves cohesive communities in the entity and similarity layers.
- LLM: index = yes; query retrieval = yes.
- Decision: semantic chunks act as the hierarchy, replacing offline graph clustering (p. 3).
- Novelty: 2 — attributed community search at query time.
- Evidence: Agriculture, Mixed, News Articles; win rates often above 90% (Table 1); up to 37% lower index time and 41% lower token overhead (abstract). Caveat: LLM-generated questions and LLM-judged win rates.

### 31. 2604.20844 — AtomicRAG: atom-entity graphs
- Kind: method
- Approach: Targets noisy chunks and inflexible triples. LLMs extract factual "atoms", entities and triples; the graph links atoms to entities, and entities to each other through unlabeled co-occurrence edges. Query decomposition and personalized PageRank retrieve candidate atoms, which are filtered before generation.
- LLM: index = yes; query retrieval = yes.
- Decision: entity edges carry no relation label, which removes the dependence on error-prone relation labels (p. 3).
- Novelty: 3 — independently retrievable knowledge atoms replace chunks as evidence.
- Evidence: improvements on five public benchmarks (abstract). Caveat: numeric results not in the pages read.

### 32. 2602.14470 — HyperRAG: n-ary facts over hypergraphs
- Kind: method
- Approach: Represents n-ary facts as hyperedges joining entities, to avoid fragmented binary facts and long reasoning paths. HyperRetriever learns query-conditioned chains; HyperMemory uses the LLM's parametric memory to guide beam search.
- LLM: index = not stated; query retrieval = yes.
- Decision: the context budget is split 50% hyperedges, 30% entities, 20% source chunks, chosen by ablation (p. 6).
- Novelty: 2 — retrieval adapted to n-ary hypergraph facts.
- Evidence: WikiTopics, HotpotQA, MuSiQue, 2WikiMultiHopQA; average gains of 2.95% MRR and 1.23% Hits@10 (abstract). Caveat: baseline and significance details not in the pages read.

### 33. 2602.15313 — Mnemis: dual-route hierarchical memory
- Kind: method
- Approach: For memory questions that need global coverage. LLMs extract episodes, entities and factual edges into a base graph; another LLM organizes entities into overlapping category layers. Retrieval runs embedding and BM25 search beside top-down category selection, then merges and reranks.
- LLM: index = yes; query retrieval = yes.
- Decision: a child node may belong to several categories, so retrieval can follow different semantic facets (p. 4).
- Novelty: 2 — top-down global selection beside similarity search.
- Evidence: 93.9 on LoCoMo and 91.6 on LongMemEval-S with GPT-4.1-mini (abstract). Caveat: long-term memory benchmarks only.

### 34. 2602.16650 — Polymer literature scholar
- Kind: application
- Approach: Compares paragraph-vector retrieval with graph retrieval over 1,028 papers on polyhydroxyalkanoates. The graph uses canonicalized entities and relations for multi-hop retrieval.
- LLM: index = yes; query retrieval = not stated.
- Decision: paragraphs are the evidence unit, to keep experimental context (p. 6).
- Novelty: 1 — domain comparison of vector and graph pipelines.
- Evidence: GraphRAG has higher precision, VectorRAG broader recall (abstract). Caveat: headline comparison is qualitative in the pages read.

### 35. 2602.17949 — CUICurate: clinical concept set curation
- Kind: application
- Approach: Automates curation of UMLS concept sets. Concept identifiers are nodes and UMLS semantic relationships are edges; embedded seed concepts are expanded through child, parent and synonym links, then filtered and classified by an LLM.
- LLM: index = no; query retrieval = yes.
- Decision: semantic seeds are combined with graph expansion, because relevant concepts may sit outside the target's hierarchy (Methods).
- Novelty: 1 — graph expansion plus LLM curation for concept sets.
- Evidence: GPT-5 kept at least 95% of definitive gold-standard concepts across five targets (abstract). Caveat: five target concepts.

### 36. 2602.20926 — HELP: HyperNode expansion and evidence localization
- Kind: method
- Approach: Targets costly traversal and noisy summaries. OpenIE extracts triples; connected triples are grouped step by step into HyperNodes, pruned by query embedding similarity, and mapped back to source passages through a triple-to-passage index. Dense retrieval fills the remaining context slots.
- LLM: index = no; query retrieval = no.
- Decision: a primary quota is reserved for passages localized from paths, and dense retrieval backfills, to balance precision and recall (p. 5).
- Novelty: 2 — expanded paths localized directly to source passages.
- Evidence: NQ, PopQA, MuSiQue, 2Wiki, HotpotQA, LV-Eval (Table 1); up to 28.8x speedup (abstract). Caveat: some baselines are reproduced.

### 37. 2602.22584 — Reinforced co-adaptation for advertising QA
- Kind: application
- Approach: Extracts entities and relations from high-citation chunks into a graph, partitions it into communities, and routes complex queries to graph traversal beside hybrid retrieval. Reinforcement learning trains the generator against evidence-based rewards.
- LLM: index = yes; query retrieval = yes.
- Decision: the graph covers only frequently cited chunks, which cuts traversal cost while keeping useful evidence (p. 2).
- Novelty: 1 — standard GraphRAG with evidence-constrained RL.
- Evidence: 72% lower hallucination rate (Table 1); two-week A/B test with 28.6% higher like rate and 92.7% lower URL hallucination. Caveat: internal dataset and online test.

### 38. 2602.23719 — SAGE-LLM for UAV decisions
- Kind: application
- Approach: Adds graph retrieval to an LLM-based UAV decision system. The graph organizes scene, strategy and control knowledge in a star-hierarchical structure and supplies planning and obstacle-avoidance LLMs, whose semantic actions are checked by a fuzzy control barrier function.
- LLM: index = not stated; query retrieval = yes.
- Decision: semantic actions are modelled as directional cones, which allows formal safety checks of discrete LLM instructions (p. 5).
- Novelty: 1 — hierarchical graph retrieval inside a safety-control system.
- Evidence: near-zero collisions and zero-shot generalization in pursuit-evasion scenarios (abstract). Caveat: quantitative results not in the pages read.

### 39. 2603.05529 — NGDBench: neural graph data management
- Kind: benchmark
- Approach: Benchmarks neural querying over noisy, evolving organizational data. Structured and extracted unstructured sources are unified as labeled property graphs, pairing a clean latent graph with a perturbed observed graph. GraphRAG and Text-to-Cypher systems answer analytical queries and handle updates.
- LLM: index = yes; query retrieval = yes.
- Decision: both the clean and the perturbed graph are kept, so performance is measured against known imperfections (p. 3).
- Novelty: 2 — full Cypher and dynamic operations over noisy graphs.
- Evidence: five domains (Table 2); tested systems are noise-sensitive and struggle with state tracking (abstract). Caveat: benchmark-only evidence.

### 40. 2603.00460 — MED-COPILOT: guideline GraphRAG plus patient retrieval
- Kind: application
- Approach: Guidelines are segmented into provenance-linked text units; LLM extraction builds a graph of clinical concepts and relations with community summaries. A separate hybrid keyword and semantic route retrieves similar patient cases.
- LLM: index = yes; query retrieval = yes.
- Decision: segmentation keeps the guideline hierarchy, to retain conditional and escalation logic (p. 3).
- Novelty: 1 — guideline GraphRAG combined with similar-patient retrieval.
- Evidence: gains over LLM-only and RAG baselines on note completion, MedQA and clinical MMLU (Table 1, p. 5). Caveat: no gold-standard patient-similarity evaluation.

### 41. 2603.19276 — Short-answer grading with GraphRAG
- Kind: application
- Approach: Compares Microsoft GraphRAG and HippoRAG for retrieving grading evidence. HippoRAG's personalized PageRank follows graph links to find bridging rubric concepts. Graph construction is not described in the pages read.
- LLM: index = not stated; query retrieval = no.
- Decision: graph traversal retrieves an intermediate "Key Concepts" bridge even when the answer and rubric share few words (section 6.2).
- Novelty: 1 — two existing systems applied to grading.
- Evidence: NGSS Task 2 accuracy 84.8% with HippoRAG against 4.3% without retrieval (section 6.1). Caveat: task-specific comparison.

### 42. 2603.01410 — GraphScout: agentic graph reasoning
- Kind: method
- Approach: Trains models to explore knowledge graphs through a code interpreter and a node retriever. A strong "scout" LLM explores from sampled seed nodes to produce question-answer pairs with evidence clues; a smaller model is then post-trained to retrieve evidence by multi-turn tool use.
- LLM: index = yes; query retrieval = yes.
- Decision: exploration behaviour is learned from automatically generated graph questions and clues, not from hand-written guidance (section 3.3).
- Novelty: 2 — a model post-trained for free-form graph exploration.
- Evidence: GRBench, five domains; 16.7% average QwenScore gain with fewer inference tokens (Table 1). Caveat: one benchmark.

### 43. 2604.20859 — KGiRAG: iterative GraphRAG
- Kind: method
- Approach: Assumes a knowledge graph organized into communities. Query-time entity extraction and semantic search retrieve graph text, entities, relations and community summaries; if answer quality misses thresholds, the context is widened and the loop repeats.
- LLM: index = yes; query retrieval = not stated.
- Decision: refinement is capped at four iterations, which the authors found sufficient (section 3, p. 3).
- Novelty: 1 — answer-feedback iterations on GraphRAG.
- Evidence: HotpotQA; higher semantic quality and relevance than a single-shot baseline (abstract). Caveat: one dataset, no magnitudes in the pages read.

### 44. 2603.03175 — Saarthi: formal verification agents
- Kind: tangential
- Approach: GraphRAG is one of several retrieval techniques that supply technical knowledge to a multi-agent formal-verification framework. The graph is not described.
- LLM: index = not stated; query retrieval = not stated.
- Decision: not stated.
- Novelty: 0 — GraphRAG is a minor component.
- Evidence: CVDP benchmark; 70% higher assertion accuracy and 50% fewer iterations to coverage closure (abstract). Caveat: gains cannot be attributed to GraphRAG.

### 45. 2603.13307 — GraphRAG for river and sediment standards
- Kind: application
- Approach: Neo4j property graph of Japanese river and sediment-control standards with 200 nodes and 268 relations. Query time uses specialized Cypher queries and reranking.
- LLM: index = yes; query retrieval = no.
- Decision: the graph is also used to generate QA pairs for QLoRA fine-tuning, and the fine-tuned model beat runtime GraphRAG (section 7.1, p. 8).
- Novelty: 1 — a negative comparison for runtime graph retrieval on a small domain.
- Evidence: 100 questions; GraphRAG 2.62/3 against QLoRA 2.92/3 (Table 5). Caveat: 100 questions, one domain, LLM judge.

### 46. 2603.05207 — Core-based hierarchies for GraphRAG
- Kind: method
- Approach: Replaces Leiden communities with a deterministic k-core hierarchy over a graph of entities or claims. Size-bounded clusters that preserve connectivity are summarized for global questions, and retrieval uses those summaries.
- LLM: index = yes; query retrieval = no.
- Decision: k-core is chosen over k-truss, because knowledge graphs have few triangles and k-truss would discard most of the graph (section 3).
- Novelty: 2 — deterministic hierarchy in place of Leiden.
- Evidence: financial transcripts, news, podcasts; better comprehensiveness and diversity with fewer tokens (abstract). Caveat: author-selected datasets, LLM judges.

### 47. 2603.05698 — Robustness comparison of RAG and GraphRAG
- Kind: benchmark
- Approach: Compares the RGB benchmark's RAG baseline with customized Microsoft GraphRAG on noise, information integration, negative rejection and counterfactual robustness.
- LLM: index = yes; query retrieval = no.
- Decision: GraphRAG is customized separately for each of the four tasks, which the authors list as a limitation (section V).
- Novelty: 1 — task-specific GraphRAG customizations evaluated.
- Evidence: RGB; gains over the baseline, mostly for GPT-3.5 (section VI). Caveat: negative rejection stays below 50%.

### 48. 2603.06290 — EpisTwin: personal AI over a user graph
- Kind: system
- Approach: Multimodal LLMs extract triples from personal data into a user-centric graph; Leiden communities become summary nodes linked to their members. Agentic retrieval combines graph operations with GraphRAG and can re-ground image-linked entities in the raw image.
- LLM: index = yes; query retrieval = yes.
- Decision: visual refinements last only for the current reasoning session, so query-specific detail does not alter the curated graph (section 5.2).
- Novelty: 2 — query-driven visual re-grounding.
- Evidence: PersonalQA-71-100, 71 synthetic objects and 100 questions (section 6). Caveat: synthetic benchmark, LLM judge.

### 49. 2603.07179 — Minimal reasoning subgraphs
- Kind: method
- Approach: A pretrained graph foundation model retrieves query-specific entity subgraphs from knowledge graphs; an information-bottleneck selector keeps sufficient evidence while cutting size and disconnected structure. Relational paths are scored and passed to the LLM.
- LLM: index = no; query retrieval = no.
- Decision: the query-conditioned "core set" is chosen by a label-free information-bottleneck objective that trades evidence sufficiency against structural minimality (section 4).
- Novelty: 2 — minimal-subgraph selection on a graph foundation model.
- Evidence: multi-hop QA benchmarks; state of the art claimed (abstract). Caveat: result tables not in the pages read.

### 50. 2603.08965 — Semantic level of detail
- Kind: tangential
- Approach: Applies heat-kernel diffusion over the Laplacian of a nearest-neighbour graph of Poincaré embeddings to find continuous abstraction scales. GraphRAG is context only.
- LLM: index = no; query retrieval = no.
- Decision: scale boundaries are detected from spectral structure, replacing a hand-tuned Leiden resolution parameter (section 1).
- Novelty: 2 — continuous scale selection for graph hierarchies (not evaluated in a retrieval system).
- Evidence: synthetic block model ARI 1.00 at high signal-to-noise (Table 3); WordNet Kendall tau 0.79 for detected boundaries (Table 2). Caveat: no retrieval evaluation.

### 51. 2603.20252 — FinReflectKG: financial hallucination benchmark
- Kind: benchmark
- Approach: Evaluates hallucination detectors on financial QA, with text chunks and with extracted subject-relation-object triplets. It compares detection with and without triplets; it specifies no retrieval method.
- LLM: index = yes; query retrieval = not stated.
- Decision: triplets were left unverified on purpose, to keep naturally occurring extraction errors (p. 4).
- Novelty: 1 — isolates detector sensitivity to noisy triplets.
- Evidence: 755 annotated SEC 10-K examples; clean-condition F1 of 0.82-0.86 (Table 3), and triplets hurt most detectors (Table 4). Caveat: one annotator per example, one corpus.

### 52. 2603.11501 — KEPo: knowledge evolution poisoning
- Kind: attack-or-defence
- Approach: Poisons a GraphRAG corpus by fabricating a temporally ordered path from an original fact to a malicious target fact, so the poisoned facts join existing communities and retrieval surfaces them.
- LLM: index = yes; query retrieval = yes.
- Decision: the poisoned claim sits at the end of a fabricated "knowledge evolution" path, which lowers its mismatch with the graph and improves integration (pp. 3-4).
- Novelty: 2 — poisoning through fabricated chronological paths.
- Evidence: GraphRAG-Bench, Graph-Medical, Graph-Story; state-of-the-art attack success claimed (abstract). Caveat: headline rates not in the pages read.

### 53. 2603.11798 — DocSage: schema-aware multi-document QA
- Kind: tangential
- Approach: DocSage structures documents as relational tables; GraphRAG is only a baseline.
- LLM: index = yes; query retrieval = yes.
- Decision: the system asks clarification questions to refine a query-specific schema (p. 3).
- Novelty: 1 — dynamic relational schema; no graph RAG contribution.
- Evidence: MEBench and Loong; 89.2% MEBench accuracy against 62.0% for GPT-4o with RAG (Table 1, p. 6). Caveat: results are for a non-graph system.

### 54. 2603.13185 — World scene graphs from monocular video
- Kind: tangential
- Approach: Persistent scene graphs with objects as nodes and object relations as edges, including occluded objects. Graph-RAG-based vision-language methods are baselines.
- LLM: index = not stated; query retrieval = not stated.
- Decision: objects stay in a persistent world state while unobserved, to model object permanence (pp. 2-3).
- Novelty: 0 for graph RAG (the explorer scored the scene-graph idea itself 3).
- Evidence: ActionGenome4D; PredCls R@50 99.76 (Table 2, p. 10). Caveat: Graph-RAG results not in the pages read.

### 55. 2603.14006 — INSES: reasoning beyond explicit edges
- Kind: method
- Approach: For noisy, incomplete knowledge graphs. An LLM navigates by selecting relevant neighbouring triples, and embedding similarity expands to semantically similar nodes. The property graph has entity nodes, relation edges, attributes and node embeddings.
- LLM: index = not stated; query retrieval = yes.
- Decision: similarity links are added only during a query's search as temporary virtual edges, which avoids permanent densification and its semantic drift (p. 3).
- Novelty: 2 — LLM pruning plus query-time semantic expansion.
- Evidence: 1,000 sampled queries each from MuSiQue, 2Wiki and HotpotQA; best overall EM and LLM-judge scores with a router (Table 2). Caveat: sampled subsets, no cost figures.

### 56. 2603.14045 — The reasoning bottleneck in graph RAG
- Kind: method
- Approach: Improves answer reasoning after graph retrieval, on KET-RAG's index (keyword-chunk graph, LLM-extracted entity graph, text chunks). It adds SPARQL-style question decomposition and context compression by breadth-first search from question entities.
- LLM: index = yes; query retrieval = yes.
- Decision: context is compressed to about 4,000 tokens by graph proximity and evidence type, with no LLM call in compression (pp. 3-4).
- Novelty: 2 — graph traversal as LLM-free context compression.
- Evidence: 500 questions each from HotpotQA, MuSiQue, 2Wiki; an 8B model with decomposition and compression reaches 55.8% on 2Wiki against 48.8% for a plain 70B model (Table 3). Caveat: effects vary by system and model size.

### 57. 2603.14828 — CS-RAG: robust retrieval with imperfect graphs
- Kind: method
- Approach: Targets retrieval drift and unsupported hop continuation. It plans ordered atomic relation constraints, retrieves triples aligned with anchor and relation, propagates bindings only when candidate scores are concentrated, and retrieves text for unresolved constraints.
- LLM: index = yes; query retrieval = yes.
- Decision: an effective candidate count acts as a sufficiency check before a binding propagates, so uncertain matches do not become false chains (pp. 3-4).
- Novelty: 2 — gated hop propagation with text fallback.
- Evidence: 1,000 queries each from 2Wiki, HotpotQA, MuSiQue; strong averages and low variance across graph builders (Table 1). Caveat: controlled-issue results shown only as figures.

### 58. 2604.16350 — LiteSemRAG: LLM-free semantic graph retrieval
- Kind: method
- Approach: Heterogeneous graph of document, chunk, surface-token and contextual-meaning nodes; edges join documents to chunks, chunks to meanings and meanings to tokens. Retrieval scores co-occurring meaning nodes, then recovers isolated meanings by propagating weight within a token family.
- LLM: index = no; query retrieval = no.
- Decision: token surface forms are separated from context-specific meaning nodes, which represents polysemy without LLM parsing (pp. 5-6).
- Novelty: 3 — LLM-free indexing and retrieval with explicit meaning nodes.
- Evidence: HotpotQA, SciFact, FEVER; best MRR@10 on all three (Table 1) and zero LLM tokens (Table 3). Caveat: retrieval-only evaluation.

### 59. 2603.20309 — BubbleRAG: evidence retrieval for black-box graphs
- Kind: method
- Approach: For graphs whose schema is unknown. It builds a graph from LLM-extracted triples with textual edges, groups candidate node or edge anchors per query concept, expands connected candidate evidence graphs, ranks them and applies LLM-guided expansion.
- LLM: index = yes; query retrieval = yes.
- Decision: each edge is enriched with the combined head-relation-tail text, so query anchors can match relation meaning as well as entities (p. 3).
- Novelty: 2 — retrieval as selection of an informative connected subgraph.
- Evidence: HotpotQA, MuSiQue, 2Wiki; average F1 63.02 and accuracy 66.63 with an 8B model (Table 2). Caveat: authors' own validation setup.

### 60. 2603.22340 — Graph RAG at scale: LPG against RDF
- Kind: application
- Approach: Converts JSON fund records to RDF triples or to a designed labeled property graph, then retrieves by SPARQL traversal or by schema-aware text-to-Cypher.
- LLM: index = yes; query retrieval = yes.
- Decision: frequent attributes become reusable nodes and relationships in the property graph, to allow deterministic traversals and reduce text-to-Cypher mistakes (pp. 11-12).
- Novelty: 1 — RDF and property-graph pipelines on nested fund data.
- Evidence: 200 fund questions; scores 172.5 (RDF) and 185.5 (LPG) against 116 for agentic RAG (Table 1). Caveat: small single-domain set, manual scoring.

### 61. 2603.22528 — ChatP&ID: engineering diagrams as graphs
- Kind: application
- Approach: Nodes are DEXPI elements of a piping and instrumentation diagram; compositional and reference attributes become typed edges, data attributes become node properties. Retrieval offers global and local vector search, path traversal and generated Cypher.
- LLM: index = yes; query retrieval = yes.
- Decision: three abstraction levels (complete, process, conceptual) are kept, so topology is not lost in one condensation step (p. 6).
- Novelty: 1 — existing retrieval methods applied to diagrams.
- Evidence: 18% accuracy gain over raw images and 85% lower token cost than direct file ingestion (abstract). Caveat: one diagram, 19 questions.

### 62. 2603.23554 — MixDemo for textual graph QA
- Kind: method
- Approach: Nodes and edges hold natural-language text; Sentence-BERT scores them against the query and a prize-collecting Steiner tree builds a connected subgraph. A query-conditioned graph encoder weights evidence, and clustered demonstrations are chosen by a mixture-of-experts mechanism.
- LLM: index = no; query retrieval = yes.
- Decision: message passing is conditioned on the query, to suppress irrelevant nearby nodes (section 3.3, p. 3).
- Novelty: 2 — query-conditioned graph encoder as evidence filter.
- Evidence: ExplaGraphs 87.31%, SceneGraphs 82.32%, WebQSP Hit@1 71.36%, above G-Retriever (Table 2). Caveat: three benchmarks, modest gains.

### 63. 2603.25152 — OMD-GraphRAG enhancements
- Kind: method
- Approach: LLM extraction creates typed triples under an expert schema; clustering adds attribute similarity, boundary neighbours and pattern-constrained multi-hop groups. Retrieval fuses entity-neighbourhood traversal with community-report search and reranking.
- LLM: index = yes; query retrieval = no.
- Decision: a lightweight heuristic weights graph against community retrieval by query complexity, avoiding an LLM intent call per query (section 3.3.2).
- Novelty: 1 — schema extraction, clustering and two-channel retrieval combined.
- Evidence: MultiHop-RAG average F1 78.92% against 69.71% for LightRAG (Table 1). Caveat: one benchmark, single run, no significance test.

### 64. 2604.20869 — OncoBrain: oncology planning evaluation
- Kind: application
- Approach: A cancer graph links diseases, stages, biomarkers, drugs, regimens, toxicities and trials; construction and retrieval mechanics are not in the pages read. The platform adds treatment-plan memory and safety checks.
- LLM: index = not stated; query retrieval = yes.
- Decision: a clinician review step stays in the loop, with sources, reasoning and safety scores exposed (Methods).
- Novelty: 1 — domain retrieval plus plan corpus and safety layer.
- Evidence: guideline-alignment means of 4.60, 4.56 and 4.70 across three clinician groups, 173 case reviews (Table 1). Caveat: mostly synthetic cases.

### 65. 2604.19776 — South African tuberculosis domain model
- Kind: application
- Approach: Biomedical named-entity recognition extracts entities, and embedding-based linking builds the graph. Retrieval combines BM25, dense FAISS search, graph-semantic retrieval and neighbouring-document expansion.
- LLM: index = no; query retrieval = no.
- Decision: BioMistral is fine-tuned with QLoRA, because the dataset is small and full fine-tuning is infeasible (section 3.4).
- Novelty: 1 — domain fine-tuning with hybrid graph retrieval.
- Evidence: average accuracy 69.70 and factuality 77.70 for the fine-tuned model with GraphRAG (Table 3). Caveat: no tuberculosis specialist took part.

### 66. 2604.04948 — PDF conversion pipelines for RAG
- Kind: tangential
- Approach: Compares PDF conversion, cleaning, chunking and metadata pipelines for ordinary RAG. An exploratory GraphRAG arm used LLMGraphTransformer triples in Neo4j.
- LLM: index = yes; query retrieval = not stated.
- Decision: hierarchical chunks carry a breadcrumb of their section path, so retrieved passages keep document structure (section 2.1.2, p. 6).
- Novelty: 1 for graph RAG — a negative result: GraphRAG lost to well-prepared plain RAG.
- Evidence: 94.1% for the best plain setup against 82% for GraphRAG (abstract). Caveat: 50 questions, one Portuguese administrative corpus.

### 67. 2604.16401 — GraphRAG-Router: cost-aware routing
- Kind: method
- Approach: A router coordinates several GraphRAG systems and several generator models. It picks the retrieval system first, then the generator, and may repeat retrieval over rounds.
- LLM: index = no; query retrieval = yes.
- Decision: choosing retriever before generator reduces the staged choice to O(|G| + |L|) (section 4.1).
- Novelty: 2 — learned multi-round routing over retrievers and generators.
- Evidence: best average EM on six QA datasets (Table 1); nearly 30% less large-model use (section 5.4). Caveat: 1,000 sampled items per dataset.

### 68. 2604.00556 — HabitatAgent: housing consultation
- Kind: application
- Approach: Graph of properties, regions, transit, schools and districts with relations such as LOCATED_IN and NEAR_SUBWAY, from 5,000 listings. Dense retrieval proposes candidates, then query constraints drive Cypher filtering.
- LLM: index = no; query retrieval = yes.
- Decision: the router that decides on graph retrieval penalizes a missed complex query more than an unnecessary graph call (section 3.2, p. 7).
- Novelty: 1 — adaptive graph retrieval in a verified multi-agent workflow.
- Evidence: 0.95 accuracy on 300 queries and on 60 complex queries (Tables 1-2). Caveat: proprietary data, small set.

### 69. 2604.09666 — RAGSearch: agentic search over retrieval backends
- Kind: benchmark
- Approach: Benchmarks dense retrieval and five existing graph backends under one agent protocol, where agents query the chosen backend iteratively.
- LLM: index = not stated; query retrieval = yes.
- Decision: backends are compared under a shared agent protocol and matched retrieval budgets (section 1).
- Novelty: 3 — asks whether agentic search removes the need for a graph.
- Evidence: six QA datasets; agentic search lifts dense RAG, but graph backends keep an advantage on multi-hop tasks (abstract, Table 1). Caveat: full tables beyond the pages read.

### 70. 2604.02545 — Competency questions as story plans
- Kind: method
- Approach: The Live Aid knowledge graph holds typed RDF triples from MusicBrainz, Wikidata and curated material. For each beat of a narrative plan, fixed SPARQL templates retrieve facts, while a Graph-RAG arm expands a typed, hop-limited neighbourhood around bound entities.
- LLM: index = no; query retrieval = no.
- Decision: prewritten SPARQL templates are executed, and no query is generated at runtime, to prevent hallucinated retrieval steps (section 4.2, p. 11).
- Novelty: 2 — competency questions become an executable retrieval plan.
- Evidence: symbolic retrieval has the highest support; Graph-RAG has global cohesion 1 (Tables 2-3). Caveat: one graph, automated metrics.

### 71. 2604.02954 — LogicPoison: logical attacks on GraphRAG
- Kind: attack-or-defence
- Approach: Attacks the graph's logical connections by swapping entities in the corpus while keeping their types. It targets frequent hubs and query-specific bridge entities, so the built graph reroutes reasoning paths.
- LLM: index = yes; query retrieval = not stated.
- Decision: global hubs are combined with query-specific bridges, so the attack damages both broad connectivity and the relevant paths (pp. 3-4).
- Novelty: 2 — type-preserving swaps corrupt topology, not content.
- Evidence: HotpotQA, 2Wiki, MuSiQue; higher attack success than PoisonedRAG across tested systems (Table 1). Caveat: 500 sampled queries per dataset.

### 72. 2604.03174 — Beyond the parameters: contextual enrichment survey
- Kind: survey
- Approach: Describes GraphRAG as triple extraction, often with community summaries from clustering, and local or global graph retrieval at query time.
- LLM: index = yes; query retrieval = not stated.
- Decision: prompting, RAG, GraphRAG and CausalRAG are ordered as rising context structure, trading indexing complexity for reasoning faithfulness (p. 3).
- Novelty: 1 — unified taxonomy with a claim-audit framework.
- Evidence: Table 2 synthesizes cited studies and says they are not head-to-head. Caveat: heterogeneous cited protocols.

### 73. 2604.09682 — Decision-theoretic safety in O-RAN agents
- Kind: tangential
- Approach: Evaluates persona-driven agents for O-RAN tasks, with RAG and GraphRAG as two retrieval conditions. The graph is not described.
- LLM: index = not stated; query retrieval = yes.
- Decision: evaluation is split into normative, prescriptive and behavioural parts (pp. 1, 5).
- Novelty: 0 — GraphRAG is an evaluated condition.
- Evidence: 486 persona configurations on two challenges (abstract). Caveat: GraphRAG-specific results not in the pages read.

### 74. 2604.16422 — Biomedical knowledge: pretraining against GraphRAG
- Kind: application
- Approach: Builds a UMLS graph of biomedical concepts and relations in Neo4j. Model2vec retrieves relevant graph text, and traversal expands it into a subgraph of triples for the LLM.
- LLM: index = not stated; query retrieval = no.
- Decision: one UMLS snapshot feeds both pretraining and GraphRAG, so the two access modes are compared on the same knowledge (p. 2).
- Novelty: 1 — controlled comparison of access modes.
- Evidence: PubMedQA 51.2 to 54.4 and BioASQ 75.0 to 80.0 accuracy (Table 5). Caveat: two datasets.

### 75. 2605.18760 — DotRAG: retrieval-time reasoning along paths
- Kind: method
- Approach: Builds an entity-relation graph, an LLM-written graph overview, and a reachability map from bounded breadth-first search, indexed by hop and entity type. At query time an LLM ranks trajectories the graph supports, and progressively constrained semantic searches retrieve and rank paths.
- LLM: index = yes; query retrieval = yes.
- Decision: reachability is precomputed, which constrains searches and avoids repeated online traversal at the cost of space (p. 4).
- Novelty: 2 — reachability index by hop and type.
- Evidence: MetaQA 2-hop recall 97.78% and F1 51.27%; 12.2x lower latency than HopRAG (abstract). Caveat: benchmarks lack annotated gold paths.

### 76. 2604.05846 — AgentGL: reinforcement learning for graph learning
- Kind: tangential
- Approach: Trains an LLM agent to use neighbourhood, hop-constrained and global search tools on text-attributed graphs for node classification and link prediction. GraphRAG is a baseline.
- LLM: index = not stated; query retrieval = yes.
- Decision: a search-constrained thinking phase reduces redundant retrieval (p. 7).
- Novelty: 0 — GraphRAG is a baseline.
- Evidence: seven graph datasets (Table 1). Caveat: graph-learning tasks, not retrieval.

### 77. 2604.06279 — Plasma GraphRAG for gyrokinetic simulations
- Kind: application
- Approach: Typed graph of parameter, device and source nodes; edges encode co-mention, definition links, physical coupling and table rows. Similarity and query-entity matches seed a d-hop expansion, and the subgraph is linearized for generation.
- LLM: index = yes; query retrieval = no.
- Decision: expert validation stays in the semi-automated extraction pipeline, for physical consistency and standard symbols (p. 4).
- Novelty: 1 — domain schema and parameter scoring.
- Evidence: ten questions; better than vanilla RAG and 35.25% fewer hallucinations (Figure 3). Caveat: ten questions, heuristic metrics.

### 78. 2605.18765 — STAR: semantic-tuned, tail-adaptive retrieval
- Kind: method
- Approach: Retrieves entity-relation paths from an existing knowledge graph. A cross-attention retriever scores query-path pairs, and beam search retrieves the paths.
- LLM: index = not stated; query retrieval = no.
- Decision: training paths are weighted by rarity with class-wise scaling, to learn long-tail paths (p. 6).
- Novelty: 2 — joint query-path scoring with tail-adaptive training.
- Evidence: WebQSP, CWQ, GrailQA; average gains of 1.8% in retrieval and 2.2% in QA (abstract). Caveat: small gains; per-dataset breakdown not read.

### 79. 2604.10426 — CodaRAG: associative evidence retrieval
- Kind: method
- Approach: Builds an entity-relation graph, induces entity types, and merges fragmented entities with embedding similarity plus an LLM judge. Retrieval combines local semantic expansion, query-personalized PageRank and structural embeddings; an LLM filters interference.
- LLM: index = yes; query retrieval = yes.
- Decision: three association pathways run together and the "hyper-associative" noise is pruned afterwards (pp. 5-6).
- Novelty: 2 — multi-path association with interference filtering.
- Evidence: GraphRAG-Bench medical and novel subsets; 7-10% retrieval-recall and 3-11% generation-accuracy gains over HippoRAG 2 (Table 1). Caveat: sampled questions, LLM-based evaluation.

### 80. 2604.14220 — Knowledge graph RAG for enterprise documents
- Kind: system
- Approach: Document and clause nodes carry timestamp, version and text; `SUPERSEDES`, `REFERS_TO` and `CONTAINS` edges model precedence, citation and hierarchy. A crawler follows references breadth-first or depth-first and aggregates the clauses.
- LLM: index = not stated; query retrieval = yes.
- Decision: supersession and reference are explicit edge types, so traversal resolves which version prevails and what a clause depends on (pp. 3-4).
- Novelty: 2 — version-aware recursive citation traversal.
- Evidence: Code of Federal Regulations benchmark; 19/20 correct or complete against 5/20 for vector RAG (Table 1). Caveat: 20 questions, keyword-overlap scoring.

### 81. 2605.18770 — Agentic GraphRAG for commercial registries
- Kind: application
- Approach: Neo4j graph built from registry records, with Company, Person, Event and NameHub nodes joined by event, role and name edges. An agent uses routed graph tools and bounded reflection to gather evidence.
- LLM: index = yes; query retrieval = yes.
- Decision: identities are joined on exact normalized name keys, never by fuzzy matching, to avoid merging borderline identities (p. 8).
- Novelty: 1 — controlled agentic retrieval on a multilingual registry.
- Evidence: 60 hand-curated questions; correctness 0.83 against 0.26 for the strongest flat baseline (section 7). Caveat: one registry, small benchmark.

### 82. 2606.00029 — TCAR-Gen: temporal graph retrieval
- Kind: method
- Approach: Case graph with Case, Person, Evidence, Location and Chunk nodes joined by typed relations and temporal markers. Query seeds drive bounded expansion; a temporal GNN, hybrid chunk scoring and path scoring rank evidence.
- LLM: index = not stated; query retrieval = no.
- Decision: paths are scored by evidence support, temporal consistency, graph coherence and model confidence, so competing evidence branches can be compared (p. 9).
- Novelty: 1 — query-conditioned temporal scoring with branching.
- Evidence: Victorian Crime Diaries; Recall@5 0.3738, above named baselines (abstract). Caveat: one corpus, graph-derived queries.

### 83. 2604.15676 — EvoRAG: feedback backpropagation into the graph
- Kind: method
- Approach: Entity-relation graph of triplets extracted from text. Retrieval forms reasoning paths; feedback on the response is assigned to paths and propagated to per-triplet contribution scores, which steer later retrieval.
- LLM: index = yes; query retrieval = yes.
- Decision: paths are the bridge from response feedback to triplets, because a triplet's utility depends on its path and query (p. 3).
- Novelty: 2 — response feedback becomes reusable triplet scores.
- Evidence: 7.34% reasoning-accuracy gain over KG-RAG frameworks (abstract). Caveat: feedback mostly comes from LLM judgments.

### 84. 2604.17056 — RLM-on-KG: LLM traversal of a mention graph
- Kind: method
- Approach: Deterministically builds a bipartite RDF mention graph of documents, chunks and named entities; co-mention edges join entities that share a chunk. At query time an LLM explores the graph and collects candidate chunks, which are reranked by vector similarity.
- LLM: index = no; query retrieval = yes.
- Decision: candidate discovery is separated from ranking; ranking is pure vector similarity, because composite scores made results worse (p. 9).
- Novelty: 2 — query-time LLM navigation with no LLM at index time.
- Evidence: GraphRAG-Bench Novel, 519 questions; +4.37 F1 over a heuristic and +2.42 over GraphRAG local search (abstract). Caveat: the GraphRAG arm is a controlled local variant.

### 85. 2604.17458 — EHRAG: hybrid hypergraph retrieval
- Kind: method
- Approach: Entity nodes with sentence-level structural hyperedges and semantic hyperedges from embedding clusters. Query anchors diffuse activation through both; passage scores combine dense similarity with activated entities, and personalized PageRank refines them.
- LLM: index = no; query retrieval = no.
- Decision: semantic hyperedges link similar entities that sit in otherwise disconnected contexts (p. 3).
- Novelty: 2 — clustered semantic hyperedges in an LLM-free graph.
- Evidence: four datasets; up to 6.9% LLM-accuracy gain over LinearRAG on 2WikiMultiHop (Table 1). Caveat: gains vary by dataset and metric.

### 86. 2604.19128 — GraphRAG-IRL: personalized recommendation
- Kind: application
- Approach: Graph of item, category and concept nodes with item-category, item-concept and item co-occurrence edges. Individual and community preference features feed inverse-reinforcement-learning pre-ranking; an LLM reranks a short list.
- LLM: index = no; query retrieval = yes (an LLM reranks the short list).
- Decision: rankings are fused, not raw scores, because the learned scores and the LLM's ordinal output are on incommensurable scales (p. 5).
- Novelty: 1 — graph features in a recommendation pipeline.
- Evidence: MovieLens and KuaiRand; NDCG@10 up 15.7% and 16.6% over supervised baselines (Table 2). Caveat: 608 test users on MovieLens.

### 87. 2605.18775 — QAFD-RAG: query-aware flow diffusion
- Kind: method
- Approach: LLM-extracted entity-relation graph. Query keywords seed a diffusion whose edge weights combine node-node similarity with how well each endpoint aligns with the query; node scores select the subgraph.
- LLM: index = yes; query retrieval = yes.
- Decision: edge weights are recomputed during traversal, so connected but irrelevant regions receive less flow (p. 4).
- Novelty: 2 — query-aware diffusion with stated retrieval guarantees.
- Evidence: QA, multi-hop QA, summarization and text-to-SQL; gains over graph RAG baselines (Table 1). Caveat: several scores use GPT-4o as judge.

### 88. 2604.20487 — Knowledge capsules and key-value injection
- Kind: method
- Approach: Extracted facts become subject-relation-object "capsules" with provenance ids; each is a graph edge. Retrieval traverses the graph, scores evidence sentences and selects capsules for injection into model attention.
- LLM: index = yes; query retrieval = not stated.
- Decision: provenance stays outside the injected key-value tensors, so raw sentences can still ground the answer (p. 9).
- Novelty: 0 for graph RAG — the contribution is the injection mechanism.
- Evidence: above RAG and GraphRAG on QA benchmarks (abstract). Caveat: results not in the pages read.

### 89. 2604.23585 — ComplianceNLP: regulatory gap detection
- Kind: application
- Approach: Regulatory graph of provisions, obligation nodes and cross-reference edges. Hybrid dense and sparse retrieval finds passages, then graph proximity reranks them for obligation extraction and gap analysis.
- LLM: index = no; query retrieval = no.
- Decision: new provisions are excluded from graph reranking until validated, and retrieval falls back to embeddings during that window (p. 3).
- Novelty: 1 — cross-reference graph as a reranking signal.
- Evidence: GapBench, 423 pairs; gap-detection F1 87.7, and removing graph reranking costs 4.6 F1 (Table 2). Caveat: benchmark from one institution.

### 90. 2604.24623 — XGRAG: graph-native explanations
- Kind: method
- Approach: On a GraphRAG backbone, it deduplicates entities in the retrieved subgraph, perturbs nodes or edges, and scores each by the semantic change in the answer.
- LLM: index = yes; query retrieval = no.
- Decision: the highest-degree entity represents each duplicate cluster, which keeps the subgraph's central node (p. 4).
- Novelty: 2 — answer sensitivity through graph-level counterfactuals.
- Evidence: NarrativeQA, FairyTaleQA, TriviaQA; node perturbation F1 0.62 against 0.54 for word-level RAG-Ex (Table 2). Caveat: attribution ground truth is derived from answer similarity.

### 91. 2605.03476 — CuraView: medical hallucination detection
- Kind: application
- Approach: Builds a patient-specific graph from multi-table health records, with patient, diagnosis, medication, lab, vital-sign, symptom, procedure and department entities. The detector retrieves sentence-level evidence and assigns grades E1-E4.
- LLM: index = yes; query retrieval = yes.
- Decision: domain-customized extraction prompts reduce entity count and improve connectivity (p. 4).
- Novelty: 1 — patient-specific GraphRAG for verification.
- Evidence: Discharge-Me subset, 50 held-out patients, 1,103 sentences; E4 F1 0.831 (Table 1). Caveat: small held-out patient set.

### 92. 2605.05643 — TGS-RAG: bidirectional text-graph synergy
- Kind: method
- Approach: LLM-extracted graph linked both ways to source chunks. Semantic beam search finds graph paths; visited nodes vote to rerank text, and cues in the text can restore graph paths that the beam pruned.
- LLM: index = yes; query retrieval = yes.
- Decision: pruned nodes are kept as deferred memory, so a path can be restored without another database query (p. 3).
- Novelty: 2 — two-way reranking and path resurrection.
- Evidence: MuSiQue and HotpotQA; strict hit rate 34.84% and 62.00%, judge accuracy 41.37% and 79.99% (Table 1). Caveat: LLM-judged answers.

### 93. 2605.06403 — GATHER: convergence-centric retrieval
- Kind: method
- Approach: Cell-centric graph linking genes, functions, pathways and cell types, assembled from 20+ databases and seven ontologies. Traversal from many source genes ranks candidate nodes by weighted gene support and hop distance; no LLM is used until the final reasoning.
- LLM: index = not stated; query retrieval = no.
- Decision: the retriever returns convergence nodes, not paths; adding the paths lowered accuracy (p. 5).
- Novelty: 2 — multi-source convergence as the retrieval target.
- Evidence: Immune and Lung datasets; exact match 27.45% and 59.64% (Table 2). Caveat: depends on graph coverage; no expert validation.

### 94. 2605.08997 — SEM-RAG for telecom standards
- Kind: method
- Approach: Compiles paragraphs, table cells, headers, predicates, formulas and operators into typed graph primitives with provenance. Structural entropy minimization builds a hierarchy, and query routing selects subgraphs.
- LLM: index = yes; query retrieval = yes.
- Decision: structural entropy replaces LLM clustering for the hierarchy, to cut indexing cost (pp. 2, 6).
- Novelty: 2 — tables and formulas compiled into graph primitives.
- Evidence: TeleQnA 94.1% and ORAN-Bench-13K 93.8% accuracy (abstract). Caveat: matched baseline details not in the abstract.

### 95. 2607.16208 — ColGraphRAG: late-interaction retrieval
- Kind: method
- Approach: On a prebuilt multimodal evidence graph of text, tables, images, entities and provenance, it replaces pooled image similarity with MaxSim over query tokens and image regions, then expands the selected evidence graph.
- LLM: index = not stated; query retrieval = no.
- Decision: only the ranking of graph-linked images changes; graph and downstream pipeline stay fixed, to isolate the effect (p. 2).
- Novelty: 1 — late-interaction reranking of visual candidates.
- Evidence: MultimodalQA and ViDoRe v3; better point estimates (Tables 1-2). Caveat: the authors disclaim statistical significance.

### 96. 2605.09505 — EpiGraph: epilepsy knowledge graph and benchmark
- Kind: benchmark
- Approach: Combines literature and seven clinical resources into a graph of genes, phenotypes, syndromes, treatments and outcomes with evidence-grounded typed relations. Graph-RAG retrieves disease-centred subgraphs and reasoning paths.
- LLM: index = yes; query retrieval = yes.
- Decision: rule-based and LLM relation extraction are combined, then triplets are normalized and validated against a schema of 1,370 types (pp. 3-4).
- Novelty: 1 — domain graph with a multi-task benchmark.
- Evidence: EpiBench, five tasks; largest gains of 30-41% in pharmacogenomic reasoning (abstract). Caveat: no per-task table in the pages read.

### 97. 2605.10247 — GTLM: unified text and graph reasoning
- Kind: tangential
- Approach: A graph-language model that adds topology-derived attention biases between node text tokens. It has no retrieval pipeline; GraphRAG-style systems are comparisons.
- LLM: index = not stated; query retrieval = not stated.
- Decision: position ids reset per node and prefix attention is bidirectional, for permutation equivariance over node order (p. 3).
- Novelty: 0 — not a graph RAG contribution.
- Evidence: WebQSP F1 77.9 against 69.8 for GNN-RAG (Table 3). Caveat: different readers and retrieval setups.

### 98. 2605.12061 — SAGE: self-evolving graph memory
- Kind: method
- Approach: A writer builds graph memory incrementally from interactions; a trained graph foundation model reads it and returns feedback to the writer. Nodes and edges encode entities, episodes, documents, aliases, temporal constraints and cross-fragment relations.
- LLM: index = yes; query retrieval = no.
- Decision: calibration on the current graph is combined with transferable structural schema prompts, so retrieval adapts as the graph changes (p. 6).
- Novelty: 2 — reader feedback evolves what the writer stores.
- Evidence: LongMemEval; 7.4-9.6 points over native memory backends after one evolution round (abstract). Caveat: per-backend detail not in the abstract.

### 99. 2605.13481 — PersonalAI 2.0: search planning
- Kind: method
- Approach: Stores object, thesis and episodic vertices with relations and separate dense and sparse embeddings. An LLM extracts entities, matches vertices and generates clue queries step by step to guide traversal.
- LLM: index = yes; query retrieval = yes.
- Decision: the three plan-enhancement operations are evaluated separately, because preprocessing can hurt; the authors advise against enabling all three (pp. 2, 5).
- Novelty: 1 — planned iterative traversal.
- Evidence: five QA benchmarks; 18-point average judge-score gain (abstract). Caveat: some analyses use 100 questions per benchmark.

### 100. 2605.14034 — SoVA: social value alignment
- Kind: application
- Approach: Applies GraphRAG to texts on social principles and values. Community answers are scored at query time, the top k are kept, and summarized instructions guide the LLM.
- LLM: index = yes; query retrieval = yes.
- Decision: only top-level community summaries are used, to avoid redundant abstraction (p. 4).
- Novelty: 1 — retrieval of prescriptive principles.
- Evidence: DailyDilemmas; expected-behaviour ratios of 95.71% (Maslow) and 94.51% (Plutchik) (Table 5). Caveat: targets are theory-defined.

### 101. 2605.14177 — Thinking ahead: prospection-guided retrieval
- Kind: tangential
- Approach: Uses simulated future steps as retrieval probes to find long-term memories that a direct query misses. GraphRAG is a comparison baseline; the memory store's graph is not described.
- LLM: index = yes; query retrieval = yes.
- Decision: simulate, retrieve and refine run in a loop, so retrieved facts sharpen later probes (p. 5).
- Novelty: 0 for graph RAG — a retrieval policy, not a graph method.
- Evidence: MemoryQuest; recall 0.723 against 0.256 for the strongest baseline (Table 1). Caveat: authors' benchmark, sampled subsets.

### 102. 2605.14259 — HEAR: hypergraph enterprise agentic reasoner
- Kind: system
- Approach: Base graph nodes are data interfaces with binary links; hyperedges bind node subsets to business rules and procedures. The LLM retrieves hyperedges, then inspects adjacency or runs bounded path discovery to decide which data to fetch.
- LLM: index = not stated; query retrieval = yes.
- Decision: data stays virtualized behind graph nodes; millions of table rows are not instantiated in the hypergraph (pp. 3-4).
- Novelty: 2 — graph over data interfaces and rules, not over data.
- Evidence: supply-chain evaluation, up to 94.7% accuracy (abstract). Caveat: authors' own tasks.

### 103. 2605.14362 — Correctness-aware repository filtering
- Kind: tangential
- Approach: Filters repository files by size before tokenization to cut context bloat. GraphRAG is only a comparison.
- LLM: index = no; query retrieval = no.
- Decision: filesystem size metadata is read before any file content, which makes filtering nearly free (p. 2).
- Novelty: 0 — not a graph RAG contribution.
- Evidence: 10 repositories; 79.6% mean token reduction and 72% file-level accuracy over 18 tasks (abstract). Caveat: 18 tasks.

### 104. 2605.15109 — Why neighbourhoods matter
- Kind: benchmark
- Approach: Tests whether the evidence an agentic GraphRAG system cites accounts for its answer. Entities and relations come from evidence triples plus distractor text; agents search and traverse iteratively.
- LLM: index = yes; query retrieval = yes.
- Decision: entity removal is compared with text masking, which separates lost evidence from lost traversal structure (pp. 3-4).
- Novelty: 2 — citation sufficiency tested by trajectory ablation.
- Evidence: 30 selected 2WikiMultiHopQA questions; removing visited-but-uncited nodes also changes accuracy (Table 2). Caveat: 30 questions.

### 105. 2605.16676 — MetaKGEnrich: metacognitive graph enrichment
- Kind: application
- Approach: Seeds a GraphRAG graph from a web snippet, scores nodes with seven graph metrics, has an LLM write questions about weak nodes, and ingests the retrieved snippets.
- LLM: index = yes; query retrieval = yes.
- Decision: nodes at or below the median of each metric are flagged, an adaptive threshold (p. 3).
- Novelty: 1 — graph metrics as a signal of knowledge gaps.
- Evidence: improved 26/30 NQ, 25/30 MS MARCO and 24/30 HotpotQA questions. Caveat: 30 questions per dataset, LLM judge.

### 106. 2605.18410 — From Node2Vec to GPT-based GraphRAG
- Kind: tangential
- Approach: Predicts paper impact. Papers are nodes; citation and textual-similarity edges respect temporal order. Node2Vec embeddings are compared with GPT prompts that include graph neighbourhoods.
- LLM: index = not stated; query retrieval = yes.
- Decision: edges to future papers are excluded, to prevent temporal leakage (p. 8).
- Novelty: 0 — GraphRAG is one arm of an impact-prediction study.
- Evidence: prompts with only the target paper often equal or beat prompts with graph context; peak AUC about 0.87 (abstract). Caveat: one journal dataset.

### 107. 2605.19735 — ContextRAG: extraction-free graph construction
- Kind: method
- Approach: Avoids LLM entity and relation extraction. It clusters chunk embeddings and applies fuzzy Formal Concept Analysis; chunk-derived concepts and soft join and meet operations induce the graph. Retrieval uses that structure, with sequential query rewrites.
- LLM: index = yes; query retrieval = yes.
- Decision: a finite set of seed, soft-join and soft-meet nodes stands in for the full fuzzy concept lattice, which would be too large to enumerate (p. 9).
- Novelty: 3 — topology from concept lattices, not from extracted triples.
- Evidence: 130 UltraDomain tasks; 33.6% overall F1 and 36.8% multi-hop F1 (abstract). Caveat: no same-metric baseline without a graph.

### 108. 2606.00062 — Graph-augmented financial sentiment analysis
- Kind: application
- Approach: Ticker nodes connect to news chunks; directed INFLUENCES edges between tickers carry sentiment intensity. Dense retrieval finds seed chunks, then traversal expands to neighbouring tickers and reranks their chunks.
- LLM: index = no; query retrieval = no.
- Decision: traversal follows only edges with intensity above 0.5, balancing breadth and precision (p. 5).
- Novelty: 1 — thresholded sentiment-weighted two-hop traversal.
- Evidence: 100 queries; +6.4% entity recall and +11.7% answer relevancy (Tables I-II). Caveat: the ablation uses 20 questions.

### 109. 2605.20815 — GraphRAG on consumer hardware
- Kind: system
- Approach: Evaluates Microsoft-style GraphRAG for health-record schema retrieval with local models: extraction from schema chunks, Leiden communities, hierarchical summaries, local and global search.
- LLM: index = yes; query retrieval = yes.
- Decision: local search is compared with global summarization, because global search may exceed a small model's synthesis capacity (pp. 6-7).
- Novelty: 1 — model and mode trade-offs on local hardware.
- Evidence: eight-document subset; Qwen 2.5 scores 3.3/5 in local search, and Phi-4-mini fails to build the index (Table 4). Caveat: curated subset.

### 110. 2608.05153 — Universal pathologies, conditional consequences
- Kind: benchmark
- Approach: Compares RAG architectures on typed DO-178C requirements and MuSiQue paragraph chains. Requirements or paragraphs are nodes with typed links or REFERENCES edges; the GraphRAG arm takes vector seeds and walks up to two hops. Corpus, embedder and judge are varied.
- LLM: index = no; query retrieval = yes.
- Decision: the same 300 query-pipeline-repeat tuples are pinned across embedders, to measure judge consistency (p. 3).
- Novelty: 2 — robustness across corpus, embedder and judge.
- Evidence: GraphRAG citation precision of 0.12-0.23 across settings (Table 2). Caveat: one anonymized requirements corpus.

### 111. 2605.21974 — Format-constraint coupling in graph construction
- Kind: benchmark
- Approach: Studies how the serialization of statistical tables and the schema choice affect graph construction. The graph holds subject-time-value facts; deterministic topology classification and schema induction guide extraction, and fact coverage checks that triples are bound correctly within two hops.
- LLM: index = yes; query retrieval = no.
- Decision: fidelity is measured by direct graph access, because the usual retrieval path masks construction errors (p. 8).
- Novelty: 2 — shows that input format and schema interact, with a fidelity benchmark.
- Evidence: six-dataset factorial with interaction up to +1.180; CSVFidelity-Bench has 1,892 facts (section 4.3). Caveat: bounded datasets; some sections lost in extraction.

### 112. 2605.21994 — Ex-GraphRAG: interpretable evidence routing
- Kind: method
- Approach: Replaces the opaque message-passing encoder of a GNN-based GraphRAG with an additive one (M-GNAN), so each retrieved node's contribution can be read off exactly.
- LLM: index = not stated; query retrieval = not stated.
- Decision: an additive encoder gives exact node attribution, where post-hoc explanation only approximates it (pp. 2, 4).
- Novelty: 2 — exact attribution by construction.
- Evidence: STaRK-Prime; matches the black-box encoder, and pruning bridge nodes cuts multi-hop Hits@1 by up to 28% (Table 2). Caveat: one dataset; the detailed audit covers one query.

### 113. 2605.24366 — Structure-aware RAG from noisy data
- Kind: tangential
- Approach: Uses tables as the intermediate representation for noisy conversational data: it generates and validates table values, retrieves tables densely and expands evidence through table lookups. GraphRAG is a baseline.
- LLM: index = yes; query retrieval = no.
- Decision: unsupported or structurally invalid table values become NULL (p. 5).
- Novelty: 0 — the proposed retrieval uses tables, not a graph.
- Evidence: MSDialog and CSDialog; recall 0.94 and 0.53 (Table 1). Caveat: two datasets, LLM judge.

### 114. 2605.25480 — Retrieval as reasoning via an LLM-compiled wiki
- Kind: tangential
- Approach: An agent searches, reads and follows links over wiki pages that an LLM compiled from the corpus; pages and bidirectional links are the navigable structure. GraphRAG is a baseline.
- LLM: index = yes; query retrieval = yes.
- Decision: a persistent "Error Book" accumulates compilation-error constraints and drives repairs across batches (pp. 3-5).
- Novelty: 1 — pages and links in place of entities and relations.
- Evidence: 500 questions each on HotpotQA, MuSiQue and 2Wiki; 2.0-8.1 F1 above the listed GraphRAG baseline (Table 1). Caveat: one answer model.

### 115. 2605.27164 — Query symbolically or retrieve semantically?
- Kind: benchmark
- Approach: Presents DualGraph and the SpecsQA benchmark for semi-structured QA. A textual graph serves semantic retrieval by entity-to-chunk voting; a symbolic graph of normalized triples serves LLM-generated SPARQL; a router or fallback joins them.
- LLM: index = yes; query retrieval = yes.
- Decision: the two graph views stay separate, after a learned contrastive alignment of the two showed no measurable gain (p. 5).
- Novelty: 2 — paired semantic and symbolic views with fallback.
- Evidence: SpecsQA, 117 hand-written questions; list-match F1 0.372 against 0.290 for Wikontic (Tables 1-2). Caveat: 117 questions from one website snapshot.

### 116. 2605.28004 — CrossAug: cross-chunk graph augmentation
- Kind: method
- Approach: Targets relations whose evidence spans chunks and is therefore missed by per-chunk extraction. A GNN scores sampled subgraphs for "missingness", and an LLM proposes evidence-cited triples only for the selected regions, offline.
- LLM: index = yes; query retrieval = no.
- Decision: the GNN predicts that a subgraph is missing something, not which edge, which matches the goal of deciding where to spend LLM effort (p. 4).
- Novelty: 2 — self-supervised topology scoring directs completion.
- Evidence: HippoRAG2 with CrossAug gains 1.33 F1 on MuSiQue and 1.09 on LiteraryQA (Table 2). Caveat: small gains; sampled questions.

### 117. 2605.28120 — LegalGraphRAG: multi-agent legal reasoning
- Kind: application
- Approach: Hierarchical graph with ontology, fact and rule layers; case and article relations support semantic, community and charge-anchored retrieval. A Researcher retrieves, an Auditor checks applicability against source documents, and an Adjudicator synthesizes.
- LLM: index = yes; query retrieval = yes.
- Decision: each article carries a diagnostic checklist of legal conditions, which semantic matching may miss (pp. 4-5).
- Novelty: 1 — legal hierarchy with verification agents.
- Evidence: CAIL2018 and CMDL; charge accuracy and F1 averages of 37.2/58.3 and 37.6/51.1 (Table 2). Caveat: two benchmarks, one backbone.

### 118. 2605.28175 — Mixture-of-experts KG-RAG for recommendation
- Kind: method
- Approach: Entities and triples are embedded into node and triple indexes. An agent chooses between no retrieval, triple retrieval, k-hop expansion, or a connected subgraph ranked by personalized PageRank.
- LLM: index = no; query retrieval = yes.
- Decision: for the connected subgraph, edges are scored by query-relation similarity and Kruskal's minimum spanning tree picks a low-cost connecting tree (p. 8).
- Novelty: 1 — query-routed retrieval granularity.
- Evidence: real-world datasets; headline numbers not in the pages read. Caveat: results unavailable.

### 119. 2605.28645 — GraphSteal: structural knowledge stealing
- Kind: attack-or-defence
- Approach: Black-box reconstruction of the hidden graph behind a Graph RAG system. Depth-wise heuristic search expands evidence around an entity for attributes; breadth-wise diffusion spreads across relation neighbourhoods to infer topology.
- LLM: index = not stated; query retrieval = yes.
- Decision: breadth-wise diffusion recovers global topology and avoids the error accumulation of one deep chain (pp. 2, 5).
- Novelty: 2 — topology reconstruction by adaptive traversal.
- Evidence: generic and healthcare scenarios; over 90% recovery claimed (abstract). Caveat: recovery falls as the graph grows.

### 120. 2605.29168 — Better later than sooner: ontology-grounded graph construction
- Kind: tangential
- Approach: Builds ontology-consistent symbolic graphs of triples and qualifiers. OAK extracts and canonicalizes facts; MEND flags ontology violations and applies targeted LLM corrections, then validates.
- LLM: index = yes; query retrieval = no.
- Decision: correction is deferred until after extraction, which avoids repeated LLM calls per fact (section 4).
- Novelty: 1 — post-extraction ontology repair (construction only, no retrieval).
- Evidence: HotpotQA and MuSiQue; valid triples rise to 96.8% and 98.4% (Table 1). Caveat: consistency figures reported on HotpotQA.

### 121. 2605.31010 — MoG: mixture of experts for GraphRAG
- Kind: method
- Approach: An LLM extracts entity nodes and triple edges; fuzzy clustering creates overlapping "expert" graphs, and semantic, structural and mixed hubs hold general knowledge. Hub clues route retrieval to selected experts, followed by a second activation round.
- LLM: index = yes; query retrieval = yes.
- Decision: expert membership overlaps, because one entity plays several roles (p. 3).
- Novelty: 2 — sparse activation of overlapping subgraphs.
- Evidence: MuSiQue LLM-accuracy 66.0 against 51.4 for the strongest baseline (Table 1). Caveat: LLM-based accuracy.

### 122. 2606.00610 — MemGraphRAG: memory-based multi-agent construction
- Kind: method
- Approach: Agents share a global memory while building schema, fact and passage layers, to avoid fragmented graphs from local extraction. Retrieval filters memory candidates, initializes node weights and runs personalized PageRank.
- LLM: index = yes; query retrieval = no.
- Decision: propagation does not start from high-degree type nodes, because generic types spread noise (p. 7).
- Novelty: 1 — shared-memory multi-agent construction.
- Evidence: HotpotQA, 2Wiki, MuSiQue and two GraphRAG-Bench subsets; overall LLM-accuracy 59.25 (Table 1). Caveat: LLM-based accuracy.

### 123. 2606.01394 — UniD3: KG-RAG for drug-disease discovery
- Kind: application
- Approach: Paper-level extraction finds entities and relations; a second graph-level pass consolidates drug, disease, gene, target and treatment nodes. KG-RAG retrieves graph evidence for dataset generation and QA.
- LLM: index = yes; query retrieval = yes.
- Decision: a second consolidation pass connects the fragmented per-paper graphs (Methods).
- Novelty: 1 — two-stage extraction for biomedical graphs.
- Evidence: 157,849 PubMed articles; external F1 0.82-0.87 and clinician AUROC 0.90 (abstract). Caveat: clinician review scope not stated.

### 124. 2606.03460 — From 3D perception to underground mine safety
- Kind: application
- Approach: Scene-graph nodes are detected objects and anomalies with geometry, confidence and uncertainty; proximity edges hold spatial relations. Identity matching links consecutive graphs into a temporal graph, and GraphRAG retrieves summaries of past graphs.
- LLM: index = not stated; query retrieval = yes.
- Decision: GraphRAG is triggered only when short-term context is insufficient (p. 9).
- Novelty: 1 — graph memory in layered safety reasoning.
- Evidence: 115 hazard scenarios; coverage rises from 57% with rule checks to 93% with memory (abstract). Caveat: includes simulated scenarios.

### 125. 2607.22571 — SCAIR: schema-conditioned enterprise graph reasoning
- Kind: method
- Approach: For agentic KG-RAG on dense, constrained enterprise graphs. It adds schema-conditioned planning, schema-aware traversal and controlled propagation of topic entities to iterative reasoning.
- LLM: index = no; query retrieval = yes.
- Decision: structural priors are injected at inference time, so schema changes need no retraining (p. 2).
- Novelty: 1 — schema-aware control of iterative retrieval.
- Evidence: a manufacturing configuration-database benchmark; beats evaluated baselines (abstract). Caveat: numbers not in the pages read.

### 126. 2606.04646 — QO-Bench: query-operator retrieval benchmark
- Kind: tangential
- Approach: Diagnoses whether RAG systems keep typed event tuples intact and can execute query operators. GraphRAG is one evaluated baseline.
- LLM: index = not stated; query retrieval = not stated.
- Decision: answers are scored by exact tuple matching, with no LLM judge (abstract).
- Novelty: 0 for graph RAG — but a strong negative data point for it.
- Evidence: 22,984 articles, 614 events, 785 questions; GraphRAG recall 3.8% in global mode and 0.9% in local mode (Table 4). Caveat: one corporate-event domain.

### 127. 2606.28351 — HyperSU: semantic-unit hypergraph RAG
- Kind: method
- Approach: Targets hallucinated hyperedges and noisy expansion. Source-grounded semantic units are segmented by an entity-aware minimum description length criterion; each unit is a hyperedge over its co-mentioned entities. Clue-guided forward and backward expansion finds multi-hop evidence.
- LLM: index = no; query retrieval = yes.
- Decision: units found by backward retrieval only verify and boost forward evidence, so dense seeds cannot dominate the ranking (p. 5).
- Novelty: 2 — hyperedges are spans of source text, chosen by MDL.
- Evidence: GraphRAG-Bench average accuracy 68.7 against 63.2 for the strongest baseline (abstract). Caveat: one benchmark family.

### 128. 2608.12335 — HC-RAG: financial filing evidence retrieval
- Kind: method
- Approach: Typed graph of document, section, text-unit, table-unit and metadata nodes. Retrieval follows document-section-unit paths and routes evidence by query intent.
- LLM: index = not stated; query retrieval = no.
- Decision: text and table evidence are routed by four intents, because question types need different evidence (p. 2).
- Novelty: 1 — intent-aware routing over filing structure.
- Evidence: DocFinQA +6.6 F1 over RAPTOR; Multi-Doc-2025 60.2 F1, 10.9 above GraphRAG (Table 3). Caveat: Multi-Doc-2025 is the authors' own.

### 129. 2606.05901 — Simple GraphRAG for complex QA
- Kind: application
- Approach: Wikipedia article, section, paragraph and chunk nodes joined by containment, link, redirect and sequence edges; chunks and titles are embedded. An agent calls a vector-search tool and a graph-query tool.
- LLM: index = no; query retrieval = yes.
- Decision: the graph is only the document structure and hyperlinks, with no extraction; the August 2025 snapshot was chosen because it covers 1,207 MoNaCo questions (section 3.1).
- Novelty: 1 — structure-and-links graph as agent tools.
- Evidence: MoNaCo; higher factual precision and recall and roughly half the hallucinations (abstract). Caveat: 108 unanswerable questions dropped.

### 130. 2606.06044 — IA-RAG: interval-algebra temporal retrieval
- Kind: method
- Approach: For events with duration and uncertain boundaries. Interval event units form a hierarchical "Thematic Forest", and edges encode Allen interval relations. Retrieval traverses under temporal constraints, and subgraph tightening refines incomplete intervals.
- LLM: index = yes; query retrieval = yes.
- Decision: traversal direction (forward, backward or both) is chosen from the seed event's relation to the target window, which avoids needless expansion (p. 8).
- Novelty: 2 — Allen relations as edges, constraint-guided traversal.
- Evidence: TimeQA 61.72, TempReason 80.21, ComplexTR 65.95 accuracy (Table 2). Caveat: trails DyG-RAG on TempReason.

### 131. 2606.07850 — PDE-Agents: graph-aided finite-element simulations
- Kind: application
- Approach: The graph stores material, run, failure and reference-chunk nodes with typed links; curated materials and run history are indexed with embeddings and HNSW. Retrieval returns the three most similar successful runs as a warm start, with graph queries available on demand.
- LLM: index = not stated; query retrieval = no.
- Decision: graph queries are deferred until a failure or an unknown material; the authors attribute the reliability gain to dropping the mandatory query overhead (pp. 9-13).
- Novelty: 2 — warm start first, graph only on failure.
- Evidence: 50-task ablation; 100% success and physics score 0.933 against 0.853 with the graph off (Table 4). Caveat: small controlled task set.

### 132. 2607.19362 — GraphContainer: unified Graph RAG platform
- Kind: system
- Approach: Maps graphs from incompatible GraphRAG systems into one in-memory representation of nodes, edges, attributes and embeddings; a recorder captures the nodes and edges retrieved at each step. Retrieval methods are pluggable (vector, one-hop, FastInsight).
- LLM: index = no; query retrieval = not stated.
- Decision: each retrieval step is recorded as an incremental subgraph, which saves storage while keeping inspection context (p. 2).
- Novelty: 2 — common representation with replayable retrieval traces.
- Evidence: BSARD and SciFact; FastInsight beat one-hop on every construction, and plain k-NN graphs had the strongest overall win rates (Table 3). Caveat: LLM-as-judge comparisons.

### 133. 2606.08944 — LongRTL: graph-guided RTL optimization
- Kind: application
- Approach: For hardware designs longer than the context window. AST nodes are code constructs; a GCN scores AST similarity to templates, and tree dynamic programming partitions the design into matching subtrees. Similar exemplars are retrieved per subtree.
- LLM: index = no; query retrieval = no.
- Decision: partitions must be connected, disjoint subtrees that cover the whole AST, so each is a usable code unit (pp. 3-4).
- Novelty: 1 — AST graph retrieval applied to RTL.
- Evidence: 14 single-module designs; 100% equivalence against under 50% for baselines (Table III). Caveat: 14 designs.

### 134. 2606.10177 — VArify: visual verification for GraphRAG
- Kind: system
- Approach: An interface for food-science researchers to inspect and repair GraphRAG errors over FoodAtlas (food, chemical and disease entities with provenance). LLM-written subquestions go to graph queries or to a plain LLM answer; the interface shows mappings, retrieved relations and evidence.
- LLM: index = not stated; query retrieval = yes.
- Decision: evidence is shown before the answer, to counter the answer-first reading seen in the formative study (pp. 5-6).
- Novelty: 2 — answer, retrieval and provenance linked for inspection.
- Evidence: formative study with 5 participants and a use study with 6 researchers. Caveat: small qualitative studies.

### 135. 2608.00006 — RAG for small-business misinformation reduction
- Kind: application
- Approach: Evaluates vector and graph RAG for small-business cybersecurity and business questions. An LLM extracts query intent, forms Cypher queries against Neo4j and retrieves facts.
- LLM: index = not stated; query retrieval = yes.
- Decision: when a graph query returns nothing, constraints are relaxed, then broad traversal is used (p. 6).
- Novelty: 1 — graph against vector comparison for one audience.
- Evidence: improved quality and fewer hallucinations claimed (abstract). Caveat: no figures in the pages read.

### 136. 2606.11562 — GraphInfer-Bench: benchmarking graph inference
- Kind: benchmark
- Approach: Tests open-ended inference over node neighbourhoods, which the authors separate from lookup and retrieval. It samples two-hop ego-subgraphs from six text-attributed graphs.
- LLM: index = no; query retrieval = no.
- Decision: tasks are built so that no answer can be read from one node or one traversal (pp. 1-3).
- Novelty: 0 for graph RAG — retrieval is explicitly excluded.
- Evidence: 42,000 samples, six graphs, five tasks; plain GNNs matched or beat the strongest LLM-based row (Table 6). Caveat: benchmark-specific scoring.

### 137. 2607.22592 — HCG-RAG: schema-constrained causal graphs
- Kind: method
- Approach: Targets the cost and size of exhaustive entity-relation extraction. Nodes are typed causal variables, edges are directed causal links, and every edge maps to supporting chunks; an LLM-written domain specification constrains construction. Retrieval matches the query to variables, runs bounded breadth-first search, ranks "outcome cones" and returns the linked evidence.
- LLM: index = yes; query retrieval = no.
- Decision: the answer model receives evidence text, never triples or node names; the graph is only a structured evidence filter (pp. 2, 6).
- Novelty: 3 — a small fixed vocabulary of causal variables replaces the entity graph.
- Evidence: medical set correctness 0.679 against 0.673 for LightRAG, with about five times fewer nodes (Table 1). Caveat: parity on quality; 375 epilepsy questions.

### 138. 2607.22597 — HyCE-RAG: hypergraph evidence chains
- Kind: method
- Approach: LLM extraction builds hyperedges that join the entities of one evidence unit, keeping source chunks and incidence links. Query entities seed a query-aware hypergraph; confidence propagates over entity-hyperedge incidence, and evidence units are assembled into chains.
- LLM: index = yes; query retrieval = yes.
- Decision: low-confidence or conflicting paths are kept apart from strong evidence and still shown, so the answer model sees alternatives (p. 8).
- Novelty: 2 — confidence propagation and chain assembly on hypergraphs.
- Evidence: HotpotQA, 2Wiki, MuSiQue and two GraphRAG-Bench subsets; gains claimed (abstract). Caveat: figures not in the pages read.

### 139. 2606.16409 — PathRouter: training for retrieval quality
- Kind: method
- Approach: Agentic GraphRAG training usually rewards a correct answer even when retrieval took a shortcut. PathRouter scores retrieved passages by overlap with gold evidence and routes policy updates by four outcome categories, with teacher guidance for evidence-poor trajectories.
- LLM: index = not stated; query retrieval = yes.
- Decision: the teacher's KL term applies only to reasoning and search-query tokens, never to answer tokens, to prevent answer imitation (pp. 2-4).
- Novelty: 2 — reward depends on answer and evidence jointly.
- Evidence: six QA benchmarks; 7B average F1 from 57.82 to 62.74 over Graph-R1 (Table 1). Caveat: evidence overlap is a proxy for path quality.

### 140. 2606.17856 — FlowRAG: frequency-aware graph flow
- Kind: method
- Approach: Graph of passage, summary, sentence and entity nodes. Retrieval activates entities from matching summaries and sentences, then propagates weighted flow to extract reasoning paths.
- LLM: index = yes; query retrieval = no.
- Decision: entity-passage links are weighted by the entity's term frequency within the passage, which discounts incidental mentions (p. 5).
- Novelty: 2 — term-frequency edge weights with two-granularity activation.
- Evidence: four benchmarks; average GPT-accuracy 58.89 against 57.17 for LinearRAG (Table 1). Caveat: small gain; sampled questions, LLM judge.

### 141. 2606.18075 — HyGRAG: unified context and relation-aware GraphRAG
- Kind: method
- Approach: Chunk and entity nodes share one graph, with edges for shared entities, extracted relations and entity-to-chunk membership. Both node types are clustered into one hierarchy; retrieval returns summaries, chunks, entities and triples.
- LLM: index = yes; query retrieval = yes.
- Decision: chunks and entities are clustered together and summarized together, which fuses context with relations that other systems retrieve separately (p. 2).
- Novelty: 2 — joint hierarchy over chunks and entities.
- Evidence: PopQA, QuALITY, MultiHop-RAG, MuSiQue, HotpotQA; +6.2% factual and +9.7% multi-hop accuracy (Table 1). Caveat: some baselines hit the two-day runtime limit.

### 142. 2606.22180 — FeLoG: distributed graph embedding with feedback
- Kind: tangential
- Approach: A distributed graph-embedding training system; feedback steers sampling toward undertrained nodes. GraphRAG is a case study.
- LLM: index = no; query retrieval = no.
- Decision: neighbour consistency is the quality feedback signal, because comparing historical embeddings costs O(|V|·d) memory and can falsely signal convergence (p. 6).
- Novelty: 0 — an embedding system, not a graph RAG contribution.
- Evidence: seven graphs; 27.9x average speedup and 53.1% less communication (abstract). Caveat: GraphRAG is only a case study.

### 143. 2606.22728 — Diagnosing retrieval-state lock-in
- Kind: method
- Approach: Diagnoses cases where repeated samples reach the same empty or wrong retrieval state and so return the same wrong answer. The KG-RAG system anchors on entities and expands local neighbourhoods.
- LLM: index = yes; query retrieval = yes.
- Decision: uncertainty is measured separately for answer, evidence and retrieval state, because agreement among sampled answers hides stable retrieval errors (p. 3).
- Novelty: 2 — three separate uncertainty signals along the pipeline.
- Evidence: six QA snapshots; 91.9% pooled precision at 7.7% coverage for the conjunctive acceptance rule (abstract). Caveat: low coverage.

### 144. 2606.23108 — Distance-indexed neural A* for medical GraphRAG
- Kind: method
- Approach: Retrieves simple paths in a directed, typed medical graph with provenance. Exact distances from pruned landmark labeling bound the feasible paths, and a neural A* heuristic orders expansions inside that corridor.
- LLM: index = no; query retrieval = no.
- Decision: the learned heuristic may only reorder work inside the exact-distance corridor, which cuts search while keeping feasibility guarantees (p. 5).
- Novelty: 2 — exact distance index bounds a learned search.
- Evidence: fertility queries on a graph of about 700K nodes; 0.74 recall at 1.18 s p95, 6.3% audited hallucinations (pp. 8-9). Caveat: one application, test size not stated.

### 145. 2606.25656 — Is GraphRAG needed? Context optimization
- Kind: benchmark
- Approach: Compares nine RAG scenarios over entity documents and a predefined knowledge graph: query entity extraction, k-hop paths, serialized subgraphs, with or without vector-retrieved documents.
- LLM: index = yes; query retrieval = yes.
- Decision: multiple relations between the same two entities are grouped to avoid repeating names, and graph and document context are deduplicated (p. 4).
- Novelty: 2 — a negative result: simple documents plus one hop beat the hybrid pipeline.
- Evidence: STaRK-Prime; documents plus one-hop relations reach Hit@1 0.6972 against 0.6422 for a hybrid GraphRAG scenario (Table 1). The 0.6972 includes LLM answer generation; retrieval-only Hit@1 is 0.5596 and document-only 0.4404, and every arm retrieves densely. Caveat: one precision-medicine dataset.

### 146. 2606.26458 — MKG-RAG-Bench: multimodal graph retrieval benchmark
- Kind: benchmark
- Approach: Retrieval evaluation over existing multimodal knowledge graphs, with aligned queries and triplet targets. It evaluates text, fusion, captioning and reranking retrievers and proposes no traversal method.
- LLM: index = no; query retrieval = no.
- Decision: queries mask the relation or the tail but keep the head entity as an anchor, which removes entity-disambiguation ambiguity (p. 4).
- Novelty: 2 — isolates the retrieval step for multimodal graphs.
- Evidence: MarKG and MedMKG; fusion reaches 51.70 NDCG@5 in the general all-modal setting (Table 3). Caveat: test queries are LLM-synthesized from triplets.

### 147. 2606.28011 — LLM agents for fault-tolerant control
- Kind: application
- Approach: Plant knowledge follows an ontology of structure, function, behaviour, monitoring and faults. Agents use SPARQL templates to retrieve subgraphs that connect equipment, states, faults, guards, actuators and limits.
- LLM: index = not stated; query retrieval = no.
- Decision: the LLM only proposes candidate plans; graph search, control and safety validation belong to dedicated components (p. 5).
- Novelty: 1 — ontology grounding for control agents.
- Evidence: simulated mixing module and stirred-tank reactor; valid recovery decisions within latency budgets (abstract). Caveat: no numbers in the pages read.

### 148. 2606.28780 — KG4VD: multimodal graph RAG for visual documents
- Kind: method
- Approach: Page-grounded graphs link extracted facts to visual or text regions of a page; canonical matching joins pages into a document graph. Retrieval anchors on relevant page images, then runs query-adaptive personalized PageRank.
- LLM: index = yes; query retrieval = yes.
- Decision: page images are retrieved first and serve as the PageRank entry points, because retrieval over the full graph adds noise (p. 5).
- Novelty: 2 — page-anchored personalized PageRank.
- Evidence: MMLongBench-Doc 44.52% overall accuracy, ahead of listed baselines (Table 1). Caveat: the authors' DLVQA set has 525 instances.

### 149. 2606.29399 — Planning over nuclear regulatory documents
- Kind: method
- Approach: The document tree is the environment. The agent browses, reads or runs BM25 search, and maintains a dynamic graph of the evidence sections collected so far and the relations inferred between them. Nothing is indexed offline.
- LLM: index = no; query retrieval = yes.
- Decision: optional edge inference is kept for traceability although it costs 2.8x and does not improve accuracy (abstract).
- Novelty: 2 — the graph is working state of one query, not an index.
- Evidence: 200 NuScale safety-report questions; 81.5% accuracy against 43.5% for PageIndex and 49.5% for GraphRAG (Table 4). Caveat: 98% of judgment questions have the answer "Yes".

### 150. 2606.30093 — TIGRAG: token co-occurrence graphs
- Kind: method
- Approach: Weighted undirected graph whose nodes are corpus tokens and whose edges count sliding-window co-occurrence inside chunks. Query tokens seed personalized PageRank; chunks are weighted by the expanded token scores, reranked, and expanded again with entities from retrieved chunks.
- LLM: index = no; query retrieval = yes.
- Decision: sentence-window chunking is preferred to attention-based segmentation, which costs more computation (p. 6).
- Novelty: 3 — tokens and co-occurrence counts replace extracted entities and relations.
- Evidence: HotpotQA, 2Wiki, MuSiQue; better retrieval and QA with lower indexing time, latency and prompt size (abstract). Caveat: numbers not in the pages read.

### 151. 2606.30133 — Query-aware spreading activation
- Kind: method
- Approach: Graph of typed entity nodes, binary relation edges and document or paragraph nodes joined by mentions, extracted by an LLM. Retrieval seeds the question's entities and runs three rounds of activation, gated by each candidate's cosine similarity to the query.
- LLM: index = yes; query retrieval = yes.
- Decision: a monotonic activation rule prevents cyclic reinforcement, so fixed-depth propagation fits in one Cypher query (pp. 4-6).
- Novelty: 2 — query-gated, fixed-depth propagation inside the database.
- Evidence: MuSiQue 41.68 F1 and 2Wiki 55.00 F1 on 1,000 questions each (Table III). Caveat: competitor numbers are reproductions.

### 152. 2606.30247 — Grounding under incomplete graph evidence
- Kind: method (theory)
- Approach: A theory of grounding LLM reasoning trajectories against an incomplete graph. Entities are anchors, typed relations are compatibility maps, paths are compositions. It prescribes no construction or retrieval walk.
- LLM: index = not stated; query retrieval = not stated.
- Decision: a finite slack lets unsupported but uncontradicted trajectories keep probability; hard conditioning is the infinite-penalty limit (pp. 8-10).
- Novelty: 3 — argues that hard grounding cannot handle missing evidence.
- Evidence: propositions and proofs only. Caveat: no empirical evaluation.

### 153. 2607.00052 — AGE: adaptive graph embedding
- Kind: method
- Approach: Targets the mismatch between graph and text embeddings when a frozen LLM answers over a retrieved subgraph. A learned sampler picks key nodes, and a JEPA-style encoder-decoder predicts the embeddings of auxiliary nodes.
- LLM: index = not stated; query retrieval = no.
- Decision: auxiliary nodes are masked, not key nodes, because key nodes carry the dominant context and are hard to predict (pp. 3, 6-9).
- Novelty: 2 — learned key-node sampling for graph tokens.
- Evidence: ExplaGraphs, SceneGraphs, WebQSP, CWQ named. Caveat: results beyond the pages read.

### 154. 2609.01617 — DocuSearch: hybrid enterprise retrieval
- Kind: system
- Approach: Telecom document search. Chunk relations sit in a plain edge table; vector and BM25 hits seed one-hop neighbour expansion, then weighted reciprocal rank fusion, reranking and MMR select evidence.
- LLM: index = not stated; query retrieval = yes.
- Decision: every chunk gets its own context-sufficiency and groundedness check; if none passes, expanded contexts are merged (pp. 2-3).
- Novelty: 1 — per-chunk groundedness checks in hybrid retrieval.
- Evidence: internal corpus, 120 annotated queries; P@10 0.69, R@10 0.79, grounding 89.6% (Tables III-IV). Caveat: internal corpus, small query set.

### 155. 2607.00508 — PlanRAG: logical query trees
- Kind: method
- Approach: For exploratory queries. A logical query tree has atomic-query nodes and semantic-dependency edges, built from LLM relationship judgments and dynamic programming over a multi-dimensional cost model. Execution retrieves and generates up the tree. Graph RAG is a comparison.
- LLM: index = no; query retrieval = yes.
- Decision: unrelated query pairs are classified first and pruned before dynamic programming, which reduces LLM calls (p. 4).
- Novelty: 1 for graph RAG — the graph is a query plan, not an index.
- Evidence: new WikiWeb-ERP dataset; results beyond the pages read. Caveat: authors' dataset only.

### 156. 2609.02894 — R²Adapter: routing and rewriting
- Kind: method
- Approach: A lightweight router sends each query to vanilla RAG or to graph RAG, both existing systems. Uncertain queries routed to the graph are rewritten.
- LLM: index = not stated; query retrieval = no.
- Decision: only uncertain graph-routed queries are rewritten, so well-formed queries are left alone (pp. 2, 8).
- Novelty: 2 — learned routing plus selective rewriting.
- Evidence: HotpotQA, MuSiQue, 2Wiki; up to 59% less graph usage at comparable accuracy. Caveat: result pages extracted poorly.

### 157. 2607.20506 — Hypergraph fact extraction and retrieval
- Kind: method
- Approach: Targets incomplete LLM-extracted hypergraphs and shallow one-hop retrieval. Self-consistency improves extraction, and personalized PageRank propagates query relevance through the hypergraph to rank chunks.
- LLM: index = yes; query retrieval = not stated.
- Decision: PageRank uses global connectivity and attenuates noise with topological distance (pp. 1-2).
- Novelty: 1 — PageRank and self-consistency added to HyperGraphRAG.
- Evidence: isolated hyperedges fall from 62% to 20% on Fiction. Caveat: headline table not in the pages read.

### 158. 2607.03447 — TRIAGE: Graph-RAG instrumentation
- Kind: system
- Approach: Instruments typed, directed triples across implementation, expert validation and retrieval use, to locate where an automated graph RAG pipeline fails. It defines metrics and a diagnostic chain, not a new algorithm.
- LLM: index = yes; query retrieval = not stated.
- Decision: metrics form a chain of necessary conditions; failure is located at the first broken link, which maps to a remedy for that stage (pp. 3-4).
- Novelty: 2 — stage-localized diagnostics that need no gold labels.
- Evidence: framework with proof of concept. Caveat: empirical detail not read.

### 159. 2607.19398 — HyGRL: adaptive hybrid graph reasoning
- Kind: method
- Approach: Text chunks act as relay nodes in a heterogeneous text and knowledge graph, to bridge a sparse knowledge graph. A learned policy induces structure adaptively, trained by imitation and then preference-based reinforcement learning.
- LLM: index = yes; query retrieval = no.
- Decision: a small learned MLP policy replaces online LLM exploration; reported latency is 1.82 s on HotpotQA (pp. 6-7).
- Novelty: 2 — chunks as relay nodes, with a cheap learned traversal policy.
- Evidence: 2Wiki supporting-fact F1 44.7 (Table 4). Caveat: expert preference check covers 100 pairs.

### 160. 2607.05577 — Narrative world model for fiction
- Kind: application
- Approach: Stores typed, chapter-scoped narrative records and a temporal graph whose facts carry validity intervals. BM25, vector retrieval and one-hop graph expansion assemble evidence.
- LLM: index = yes; query retrieval = no.
- Decision: a causal chapter cutoff returns only evidence up to the requested chapter, which prevents leaking later plot (pp. 2-4).
- Novelty: 2 — validity intervals and a reader-position cutoff.
- Evidence: private 176-item multi-hop set 0.898 against 0.574 for Graphiti; public 110-item subset 0.709 against 0.582 (Tables 1-2). Caveat: public subset underpowered, p = 0.054.

### 161. 2608.28607 — RegDivergence-101 benchmark
- Kind: benchmark
- Approach: FDA and EMA requirement pairs labelled AGREE, DIVERGE or directional SILENT. The Graph-RAG baseline turns each requirement into an obligation tuple and classifies aligned pairs.
- LLM: index = yes; query retrieval = no.
- Decision: SILENT is directional, because one regulator's silence differs from the other's (p. 3).
- Novelty: 0 for graph RAG — but a negative data point.
- Evidence: 101 pairs; Graph-RAG macro-F1 0.663 against 0.830 for a flat LLM (Table 1). Caveat: pilot scale, one LLM family.

### 162. 2607.08269 — PolyUQuest: structure-aware web RAG
- Kind: system
- Approach: Graph of webpage, evidence-block, entity and topic nodes joined by hyperlinks, containment, mentions and semantic associations, built from crawled HTML, DOM blocks and LLM-extracted entities. Retrieval routes a query to block search, linked-page expansion or entity-relation expansion.
- LLM: index = yes; query retrieval = yes.
- Decision: a rules-first router handles clear query patterns and calls an LLM only for the rest, to keep common-case latency low (p. 3).
- Novelty: 2 — hyperlinks, DOM structure and entities in one graph.
- Evidence: 300 questions on one university website; correctness 0.644, coverage 0.649, faithfulness 0.921 (Table 1). Caveat: one website.

### 163. 2607.10265 — TGMS: bi-temporal graph memory system
- Kind: system
- Approach: Nodes and edges with stable identity carry valid-time and transaction-time versions; an append-only event log builds the bi-temporal property graph. An LLM planner composes 13 typed temporal operators that execute deterministically, with trace-based claim checks.
- LLM: index = not stated; query retrieval = yes.
- Decision: operators report evidence completeness, because arithmetic over a truncated result still yields a plausible wrong answer (p. 5).
- Novelty: 3 — "what was believed when" becomes queryable.
- Evidence: CollegeMsg frozen test 0.408 accuracy against 0.106 for vector RAG; correction probes 0.897 against zero for latest-state baselines (Table 4). Caveat: benchmark and workload were co-designed.

### 164. 2607.11159 — NGM-RAG: neural graph matching
- Kind: method
- Approach: LLMs build a corpus graph and a query graph with textual node and edge descriptions. Node matches combine name edit similarity, BM25 and GNN representations; adaptive weighted ranking selects context.
- LLM: index = yes; query retrieval = yes.
- Decision: parameter-free LightGCN serves homogeneous graphs and supervised GINE serves heterogeneous ones (p. 4).
- Novelty: 2 — retrieval as matching a query graph to the corpus graph.
- Evidence: HotpotQA F1 0.441 with Llama 3.1 8B (Table 1). Caveat: long-context questions are LLM-generated.

### 165. 2607.11464 — FAIR GraphRAG with digital objects
- Kind: application
- Approach: Each node is a FAIR Digital Object holding data, metadata and a persistent identifier. LLMs help create schemas and extract content, and an LLM writes database queries at retrieval time.
- LLM: index = yes; query retrieval = yes.
- Decision: a property graph is preferred to RDF, because properties carry metadata and the structure matches JSON-like LLM output (p. 4).
- Novelty: 1 — FAIR metadata objects as nodes.
- Evidence: eight-patient RNA-sequencing dataset (section IV). Caveat: first 80 rows, no clinical expert evaluation.

### 166. 2607.11683 — RAGU: multi-step GraphRAG engine
- Kind: system
- Approach: Two-stage extraction of typed entities and relations, then DBSCAN-backed deduplication, summarization and Leiden communities. Retrieval is entity vector search expanded to relations and chunks, or community summaries.
- LLM: index = yes; query retrieval = yes.
- Decision: relation extraction may only use entity names validated in the first stage, which removes entity-relation mismatches (p. 2).
- Novelty: 2 — extraction separated from consolidation.
- Evidence: GraphRAG-Bench Medical; evidence recall 0.84 against at most 0.76, while HippoRAG 2 leads factoid correctness (Tables 1-2). Caveat: one benchmark domain.

### 167. 2607.12764 — EvoGraph-R1: self-evolving hypergraphs
- Kind: method
- Approach: Textual n-ary relations and image-grounded visual hyperedges are fused by entity resolution. An agent retrieves graph evidence, searches the web, edits the hypergraph or answers; a reinforcement-learning policy picks the action from quality and cost rewards.
- LLM: index = yes; query retrieval = yes.
- Decision: visual facts hang off an image anchor node, so they stay grounded in their source image (p. 4).
- Novelty: 3 — the agent edits the graph during retrieval.
- Evidence: F1 68.5 on 2Wiki, 65.4 on HotpotQA, 56.8 on NQ (Table 1). Caveat: four A100 GPUs; cost not read.

### 168. 2607.14661 — SmartRAG: mobile graph memory
- Kind: system
- Approach: On-device QA with evolving knowledge. Nodes are entities and paragraphs; extracted triplets are linked to their source paragraph before they are materialized. Retrieval uses plan-guided lexical and dense search, traversal with personalized PageRank and a full-document fallback.
- LLM: index = yes; query retrieval = yes.
- Decision: the paragraph is the atomic unit for both writing and retrieval, which ties every fact to source text (p. 5).
- Novelty: 2 — continual entity-type learning in a three-layer mobile graph.
- Evidence: TriviaQA, NQ, HotpotQA, MultiHopQA; a quantized 1.7B model matches or beats larger standalone models on several sets (Table 1). Caveat: two smartphones profiled.

### 169. 2607.15516 — Cache-aware prompt compression
- Kind: tangential
- Approach: Studies prompt caching and compression costs. A knowledge-graph RAG pipeline over code is one cost case study.
- LLM: index = not stated; query retrieval = not stated.
- Decision: compressed document context is kept independent of the query, so it stays reusable in the cached prefix (p. 10).
- Novelty: 0 — graph RAG is a cost workload.
- Evidence: 9.3x against cache-all on FastAPI and 2.4x on httpx (abstract). Caveat: two codebases.

### 170. 2607.19301 — PAGE-RAG: evidence-grounded retrieval
- Kind: method
- Approach: For long-document QA. Entity, concept, event or claim nodes with semantic and path edges, communities and summaries, all linked back to source passages. Query-adaptive routing combines text retrieval, graph neighbourhoods, paths and summaries under evidence budgets.
- LLM: index = yes; query retrieval = yes.
- Decision: text is the "evidence floor" and the graph only organizes and navigates it, because graph construction omits source detail (pp. 3, 8).
- Novelty: 3 — the graph is declared a lossy projection, never evidence.
- Evidence: two book-length datasets; all 24 unanswerable questions correctly refused, against 4/24 without the evidence constraint (p. 3). Caveat: headline is 24 refusal cases.

### 171. 2607.22319 — From naive RAG to agentic RAG
- Kind: survey
- Approach: A vision paper on knowledge-grounded data integration. It describes GraphRAG graphs built from task metadata by LLM extraction with communities and summaries, external knowledge graphs, and agentic retrieval.
- LLM: index = yes; query retrieval = yes.
- Decision: external knowledge graphs are recommended for broader factual coverage than small self-built graphs (p. 5).
- Novelty: 0 — review of existing approaches.
- Evidence: none. Caveat: no empirical evaluation.

### 172. 2607.23278 — Co-E: co-evolving graph and text memory
- Kind: method
- Approach: The graph holds canonical knowledge-base triples plus open-vocabulary triples extracted from text, joined by entity linking. Each reasoning step retrieves text and graph evidence, then consolidates, adds, prunes and re-injects facts into a shared working memory.
- LLM: index = yes; query retrieval = yes.
- Decision: synchronization runs in both directions, because either one-way flow loses the feedback between evidence types (p. 9, Table 3).
- Novelty: 2 — synchronized graph and text working memory.
- Evidence: six QA benchmarks; 72.6 EM on 2Wiki and 70.0 on Bamboogle with tree search (Table 2). Caveat: comparisons mix backbones and training regimes.

### 173. 2607.24861 — HVM-GraphRAG for complex documents
- Kind: method
- Approach: Extracts entity facts from text, tables and images, reconciles conflicts with document-wide context, then groups entities into concepts joined by relation edges. Retrieval scores concept anchors, propagates over query-relevant relations and maps concepts straight to supporting chunks.
- LLM: index = yes; query retrieval = yes.
- Decision: retrieval runs on the compact concept graph and jumps directly to chunks, which avoids walking the dense entity graph (pp. 4-5).
- Novelty: 2 — concept-level index for multimodal evidence.
- Evidence: MMLongBench, M3DocVQA, Qasper; top results across settings (Table 2). Caveat: no cost figures in the pages read.

### 174. 2607.24475 — Historical documents through query inference
- Kind: application
- Approach: Graph of people, places, marriages and record fields from annotated historical records. An LLM writes Cypher queries and can expand selected values with word-spotting alternatives before execution.
- LLM: index = no; query retrieval = yes.
- Decision: query values are expanded with visually similar transcriptions, so the query tolerates OCR and transcription noise; automatic graph construction is out of scope (p. 8).
- Novelty: 1 — word-spotting expansion for noisy graph queries.
- Evidence: Esposalles collection, 100 questions; values not in the pages read. Caveat: one collection, small set.

### 175. 2607.24554 — DeCoRAG: semantic-aware cropping
- Kind: method
- Approach: A vision-language model extracts triples from selected regions of complex document pages to build a multimodal graph. A global semantic anchor guides region pruning and cropping; retrieval combines propagation over concept relations with direct chunk matching.
- LLM: index = yes; query retrieval = yes.
- Decision: the model describes the whole page first and only then crops and extracts local structure, which reduces attention drift and wasted visual tokens (pp. 2-3).
- Novelty: 2 — a page-level anchor steers cropping and extraction.
- Evidence: SPIQA, SlideVQA, PaperTab, FetaTab, DocVQA; up to 12.5 points better semantic pass rate and 40.8% fewer construction tokens. Caveat: gains vary by benchmark.

### 176. 2607.24663 — APS-RAG for facility operations
- Kind: system
- Approach: Dense, sparse and graph channels over operational knowledge. The LLM-extracted Neo4j graph has Fault, Component, System and LogEntry nodes with cause, resolution and system relations. Adaptive reciprocal rank fusion merges channels, and a corrective agent can retrieve again.
- LLM: index = yes; query retrieval = yes.
- Decision: fusion weights are set by hand per query intent, favouring the graph for causal, multi-hop and troubleshooting queries (p. 5).
- Novelty: 1 — hybrid retrieval with intent-dependent fusion.
- Evidence: APS-Bench, 50 questions; strict vital-nugget recall 70.3% against 63.8% for BM25 (abstract). Caveat: 50 questions.

### 177. 2607.24984 — Knowledge graphs for earthquake catalogs
- Kind: application
- Approach: Applies schema-free GraphRAG to raw tabular earthquake catalogs: LLM-extracted event entities and relations, community summaries, local and global search. Prompt changes target observed fabrication modes.
- LLM: index = yes; query retrieval = yes.
- Decision: no hand-designed ontology or per-catalog extraction rules are used (pp. 1-2).
- Novelty: 1 — standard GraphRAG moved to tabular catalogs, with an honest null result.
- Evidence: three catalogs, 1,500 answers; mechanism questions reach 2.85/3 after fixes (Table 2). Caveat: overall, GraphRAG and the vector baseline are statistically indistinguishable.

### 178. 2607.26497 — BM25 wins at scale
- Kind: benchmark
- Approach: A controlled study of retrieval paradigms across 28 nested corpus sizes. Graph methods extract entities and relations from chunks; scalable variants are included.
- LLM: index = yes; query retrieval = yes.
- Decision: the question-relevant and adversarial evidence is held fixed while the corpus grows, which isolates the effect of scale (p. 2).
- Novelty: 3 — negative result: graph construction stops scaling and trails BM25.
- Evidence: EnterpriseRAG-Bench, 500 questions; BM25 leads at the full 601M tokens, and graph systems complete fewer size tiers (pp. 1-2). Caveat: graph systems are compared only on tiers they finished.

### 179. 2607.27136 — KAMR: knowledge-aligned multi-hop retrieval
- Kind: method
- Approach: Retrieves triplets from an existing knowledge graph. Pair-level matching picks anchor triplets aligned with the query; element-level matching scores connected candidates, which are added step by step through graph neighbourhoods.
- LLM: index = no; query retrieval = no.
- Decision: anchors and connected triplets get different matching objectives, because global matching misses facts that complete a path but align weakly with the query (pp. 2-3).
- Novelty: 2 — separate objectives for anchor and bridging facts.
- Evidence: PathQuestion and CWQ; up to 99.50% triplet recall and 99.16% path recall on PathQuestion (Table 1). Caveat: extra expansion can lower CWQ answer accuracy.

### 180. 2607.27918 — OptGraph for evolutionary optimization
- Kind: application
- Approach: Typed graph of problem types, variants, descriptions, models, code, errors and fix hints, built from resources and execution traces. It ranks variant nodes, expands their neighbourhoods and, after a validation failure, retrieves error and fix nodes.
- LLM: index = yes; query retrieval = yes.
- Decision: a different model validates the output, to reduce same-model self-verification bias (p. 7).
- Novelty: 1 — graph memory of errors and fixes for optimization modelling.
- Evidence: six optimization datasets; 75.4% average exact accuracy, 8.9 points above the cited prior best (Table I). Caveat: different models for generation and validation.

### 181. 2607.28397 — GLM-RAG: graph language models for RAG
- Kind: method
- Approach: Entity graph from LLM-based named-entity recognition and relation extraction. A graph language model scores the entities of a local subgraph from structure and text together, and the ranked entities map to documents.
- LLM: index = yes; query retrieval = yes.
- Decision: the question is fused with the subgraph in the model's first attention layer, so retrieval reads graph and text semantics jointly (p. 5).
- Novelty: 2 — a graph language model replaces the GNN retriever.
- Evidence: HotpotQA, 2Wiki, MuSiQue and 11 out-of-domain sets; best on the medical and computer-science GraphRAG-Bench subsets (Table 2). Caveat: one run per experiment.

### 182. 2608.00705 — Triple-robustness analysis for multi-hop traceability
- Kind: benchmark
- Approach: Compares five retrieval pipelines on a typed requirements graph and a Wikipedia paragraph graph. The GraphRAG arm seeds a walk with vector hits and follows up to two typed-edge hops.
- LLM: index = not stated; query retrieval = yes.
- Decision: citation quality is measured on the citations in the answer as well as on the retrieved context, because the synthesizer cites selectively (pp. 2, 4).
- Novelty: 2 — the choice of citation metric reverses the architecture ranking.
- Evidence: GraphRAG wins every MuSiQue stratum on answer-citation F1 while its context precision is 0.12-0.23 (Tables 1-2). Caveat: synthetic requirements corpus.

### 183. 2608.01269 — ACE-GraphRAG: agentic context engineering
- Kind: method
- Approach: An inference-time policy on an existing hierarchical GraphRAG index. LLM-rewritten "depth" and "breadth" queries fetch supplementary evidence, merged with provenance and abstraction level kept. Read as v1.
- LLM: index = not stated; query retrieval = yes.
- Decision: a lightweight LLM classifier picks the context policy from the query's topology, because no fixed policy suits all queries (pp. 4-5).
- Novelty: 2 — context assembly adapted to query topology.
- Evidence: +4 EM on HotpotQA and 2Wiki; preferred on four UltraDomain subsets (Table 2). Caveat: 100 queries per QA dataset.

### 184. 2608.01565 — DocNavRAG: document-structured graph RAG
- Kind: method
- Approach: Graph from the document hierarchy, with paragraph and structural nodes; edges for hierarchy, reading order, citations, entity overlap, semantic similarity and community membership. An LLM agent locates, navigates, expands and fetches nodes while tracking what evidence it still needs.
- LLM: index = not stated in pages 1-12; query retrieval = yes.
- Decision: the native document hierarchy is kept and only lightweight lateral links are added, in place of costly semantic graph extraction (pp. 2, 4).
- Novelty: 2 — a stateful agent walks document structure and cross-links.
- Evidence: four long-document and multi-document QA benchmarks; best answer quality and context sufficiency (Table 3). Caveat: two benchmarks have few documents.

### 185. 2608.01732 — X-KGRank: graph RAG for recommendation explanations
- Kind: application
- Approach: MovieLens graph of user, movie and genre nodes with RATED, HAS GENRE and CO-RATED edges. LightGCN ranks items; for long-tail items a graph path grounds the LLM's explanation.
- LLM: index = no; query retrieval = yes.
- Decision: graph paths are retrieved only for long-tail items, since the LLM likely already knows popular ones (p. 5).
- Novelty: 1 — popularity decides when to retrieve.
- Evidence: MovieLens-1M; NDCG@10 0.2956 against 0.2525 for popularity (Table II). Caveat: explanations judged on 16 cases.

### 186. 2608.07994 — VDGR-RAG: unified enterprise knowledge retrieval
- Kind: system
- Approach: Hierarchical graph of directory, chunk and entity nodes with containment, chunk-entity and entity co-occurrence edges. Retrieval combines vector search, table-of-contents navigation and personalized PageRank, with directory backtracking and LLM reflection.
- LLM: index = yes; query retrieval = yes.
- Decision: entity edges are sentence-level co-occurrence, not LLM-extracted relations, which cuts cost and extraction noise (p. 7).
- Novelty: 1 — established retrieval paths plus directory routing.
- Evidence: four wireless-product domains; gains claimed (abstract). Caveat: tables beyond the extracted text.

### 187. 2608.09016 — PreGress: ranking-native graph pretraining
- Kind: tangential
- Approach: General node ranking on graphs: a pretrained GNN predicts degree and reconstructs attributes, and task prompts adapt it to ranking criteria. Graph RAG is an application example.
- LLM: index = no; query retrieval = no.
- Decision: degree prediction is the pretraining task, because the label is free and continuous (p. 3).
- Novelty: 0 — not a graph RAG contribution.
- Evidence: six public graphs and two recommendation sets. Caveat: no graph-RAG-specific results stated.

### 188. 2608.09214 — AkasicDB: unified vector-graph-relational DBMS
- Kind: system
- Approach: One database execution framework runs vector search, graph traversal and relational filtering in a single query plan ("Omni RAG").
- LLM: index = not stated; query retrieval = yes.
- Decision: the three modalities share one plan, so the top-k limit can prune work early and shrink intermediate results (pp. 2-3).
- Novelty: 2 — the query optimizer, not application code, fuses the modalities.
- Evidence: four UltraDomain datasets; wins 55-64% against vector RAG with the lowest retrieval time on each (Table 1). Caveat: demo paper, LLM judge.

### 189. 2608.09698 — VeriForge: mixed-initiative writing support
- Kind: application
- Approach: Domain graph from uploaded texts, with entities and inferred relations linked to source passages. Hybrid retrieval anchors on an entity and expands to neighbours.
- LLM: index = yes; query retrieval = yes.
- Decision: the tool offers source-linked facts as raw material, never finished prose, to preserve the author's agency and trust (pp. 2, 5).
- Novelty: 1 — provenance-aware graph RAG for writers.
- Evidence: interviews with 9 writers and a within-subjects study with 12. Caveat: small user study.

### 190. 2608.11277 — Graph-guided RAG for automotive root-cause analysis
- Kind: application
- Approach: Graph of signal-to-fault mappings, propagation relations and diagnostic rules. Features from time-series recordings retrieve similar past cases by cosine similarity; candidate faults are scored with graph evidence.
- LLM: index = no; query retrieval = yes.
- Decision: primary and direct signal evidence outweighs propagated signals, so downstream effects support the root cause without dominating it (p. 3).
- Novelty: 2 — time-series evidence with graph-constrained case retrieval.
- Evidence: two hardware-in-the-loop case studies; best top-1 accuracy 90% and 94% (abstract). Caveat: subset size not stated.

### 191. 2608.28642 — MAGG: governed knowledge graph construction
- Kind: method
- Approach: The graph holds entities, candidate triples and admitted triples, each with domain ownership, evidence provenance, a governance decision and an audit record. Domain experts (agents) review proposed triples; QA routes a question to the relevant domain subgraphs.
- LLM: index = yes; query retrieval = yes.
- Decision: one entity may have several domain owners, which allows cross-domain governance (p. 3).
- Novelty: 2 — an admission step with ownership decides what enters the graph.
- Evidence: SciERC strict F1 0.156 against 0.106 for flat insertion (Table 2); MuSiQue EM 0.51 against 0.42 for GraphRAG (Table 5). Caveat: QA on a 100-question slice.

### 192. 2609.26237 — ABAI: multi-stage legal case retrieval
- Kind: method
- Approach: BM25 context windows, neural reranking, Leiden community and graph-attention features, then LightGBM. The document graph joins cases through shared extracted legal entities, plus embedding nearest-neighbour edges.
- LLM: index = no; query retrieval = no.
- Decision: the attention graph mixes two edge sources, with entity-overlap edges weighted at 30% (p. 4).
- Novelty: 1 — graph features in a conventional ranking pipeline.
- Evidence: COLIEE 2026 test F1 0.177; citation-graph features add 0.014 F1 over eight seeds once own-citation leakage is removed (section 5). Caveat: small effect; test labels unavailable for diagnosis.

### 193. 2608.13050 — GraphRAG for threat hunting plans
- Kind: application
- Approach: Compares Microsoft GraphRAG with vector-only RAG for turning cyber threat reports into hunting plans, in local and global search modes.
- LLM: index = yes; query retrieval = yes.
- Decision: both local and global retrieval are tested, since they give different contexts to the same prompt (Table 2).
- Novelty: 0 — evaluates an existing system.
- Evidence: nine reports; in one case GraphRAG detections stay at 100% after indicators are rotated, against 29% for naive RAG (abstract). Caveat: nine reports, LLM judge.

### 194. 2608.13706 — CLAIR-Fin: claim-level verification
- Kind: tangential
- Approach: A typed claim ledger links claims to text spans, table cells, derived metrics, chart regions and constraint checks. Graph-RAG is a baseline.
- LLM: index = not stated; query retrieval = yes.
- Decision: evidence weights depend on claim type: exact numbers favour tables, causal claims favour prose (p. 5).
- Novelty: 0 — Graph-RAG is a baseline.
- Evidence: 500 questions; faithfulness 0.889 against 0.832 for Graph-RAG (Table 2). Caveat: one annual-report dataset.

### 195. 2608.24921 — Bi-temporal graph RAG in PostgreSQL
- Kind: system
- Approach: Embedded chunks, canonical entities, relation edges and community summaries all live in PostgreSQL. Retrieval fuses bounded traversal, relation-embedding similarity and lexical search; each fact's temporal validity reaches answer synthesis.
- LLM: index = yes; query retrieval = yes.
- Decision: a relation supersedes an older one by document order, never by a date the model supplied, because such dates are unsupported (p. 3).
- Novelty: 2 — bi-temporal validity carried to synthesis, in a plain relational store.
- Evidence: LongMemEval 94.0% against 71.2% for Zep; the temporal-grounding ablation moves temporal reasoning from 0.496 to 0.881 (sections 1, 8.4). Caveat: one run, one protocol.

### 196. 2608.15056 — GraphLoom: multimodal evidence routing
- Kind: method
- Approach: Builds a per-instance multimodal graph from grounded scene descriptions, extracted triples, visual relations and commonsense facts. Bounded, query-conditioned subgraph retrieval feeds selected graph slots into a frozen decoder.
- LLM: index = yes; query retrieval = yes.
- Decision: at most one extra interleaved retrieval round is allowed, which improves multi-hop coverage with bounded cost (p. 4).
- Novelty: 2 — reliability-calibrated graph slots compete with decoder context.
- Evidence: ScienceQA, MultiModalQA, OK-VQA; gains claimed (abstract). Caveat: no numbers in the pages read.

### 197. 2608.15919 — Noesis: bidirectional graph RAG
- Kind: system
- Approach: Builds the graph by reading a document forward with graph-feedback context, then reconnects backward by node degree. At query time it routes across knowledge bases by graph-derived fingerprints and finds cross-base links by chunk and node similarity.
- LLM: index = yes; query retrieval = yes.
- Decision: graph writes are serialized, so concurrent entity resolution cannot split entities that should merge (p. 6).
- Novelty: 2 — forward read plus backward reconnection for cross-section relations.
- Evidence: HotpotQA, 1,000 questions; 59.5 EM and 74.7 F1, reported as +27.8 EM over GraphRAG (abstract). Caveat: edge validation covers one 193-page document.

### 198. 2608.16004 — LineageRAG: source grounding
- Kind: method
- Approach: Decomposes a question into evidence "demands" and retrieves passages per demand over the corpus graph, keeping each candidate's demand. It selects complementary passages and links each supported demand to a verbatim source span.
- LLM: index = not stated; query retrieval = yes.
- Decision: demand identities persist through retrieval; later queries are not rewritten with earlier answers, which keeps demand-to-passage provenance independent (p. 4).
- Novelty: 2 — provenance per information need, end to end.
- Evidence: HotpotQA, 2Wiki, MuSiQue; average gains over GraphRAG baselines of 3.51 R@5, 5.96 EM and 5.22 F1 (abstract). Caveat: higher query-time token use.

### 199. 2608.16096 — Commercial costs in retrieval benchmarks
- Kind: tangential
- Approach: Audits embedding licences and indexing costs in multi-hop retrieval benchmarks and measures thirteen embedders on MuSiQue. GraphRAG appears only in the cost audit.
- LLM: index = no; query retrieval = no.
- Decision: exact brute-force cosine search removes approximate-index recall loss as a confound (p. 6).
- Novelty: 0 for graph RAG — useful cost data.
- Evidence: the paper's own estimate of GraphRAG indexing cost is $19.49 to $38.99; the range $2.30 to $24.94 per 5.64 MB corpus in section 4.2 is quoted from KET-RAG. Caveat: relevance-seeded sample, not a systematic review.

### 200. 2609.27814 — LabourCrew: statutory QA
- Kind: application
- Approach: "StatuteGraph" indexes chapters, sections, provisos and cross-references in place of fixed-size spans. Retrieval plans searches over that structure; an evidence ledger restricts agents to retrieved text, and a calibrated gate decides whether to release an answer.
- LLM: index = not stated; query retrieval = yes.
- Decision: a citation passes only if its quoted span matches source text after whitespace normalization, so translation cannot silently alter evidence (p. 5).
- Novelty: 1 — statute structure plus exact-quote verification.
- Evidence: LabourActQA, 500 Bangla questions; false-accept rate 0.081 at target 0.10; answer relevancy 0.862 against 0.815 for Graph-RAG (Table 1). Caveat: one statute.

### 201. 2608.18489 — MissDiag: typed robustness evaluation
- Kind: benchmark
- Approach: Diagnoses how missing evidence hurts KGQA and KG-RAG systems. It turns benchmark support graphs into labeled directed graphs and removes edges by four structural rules.
- LLM: index = not stated; query retrieval = not stated.
- Decision: questions and gold answers stay fixed while support edges change, which allows paired attribution of each drop (p. 3).
- Novelty: 2 — typed missingness interventions separate causes of degradation.
- Evidence: KGQAGen-10k development split, 1,050 paired examples; removing answer-adjacent edges hurts most (Table 1). Caveat: development split only.

### 202. 2608.18952 — rEDMRec: editable experience memory
- Kind: tangential
- Approach: Editable memory for recommendation; GraphRAG is a baseline.
- LLM: index = yes; query retrieval = no.
- Decision: teacher reasoning is stored in four separately editable channels, so ranking never calls the teacher (pp. 1, 5).
- Novelty: 0 — GraphRAG is a baseline.
- Evidence: ML-1M, Amazon Beauty, Steam; HR@1 gains over GraphRAG on most backbones (abstract). Caveat: two backbones are exceptions.

### 203. 2608.22062 — MedEventGraph-RAG: evidence-admissible verification
- Kind: system
- Approach: Verifies relations among patient-specific event occurrences that are linked to source rows, note spans, timestamps and trajectories. Recorded relations and deterministic derivations form the graph. Retrieval expands candidate event chains broadly; a separate assessor uses only evidence that passes patient, scope, occurrence and source checks.
- LLM: index = not stated; query retrieval = yes.
- Decision: retrieval scores and controller state may not enter the assessor's evidence bundle, so identical bundles always get the same assessment (p. 4).
- Novelty: 3 — discovery and admissible evidence are separate stages with a hard wall between them.
- Evidence: i2b2, n2c2, MIMIC-IV, LUNGUAGE; balanced accuracy 78.6, 67.3 and 96.8 on three pairwise protocols (Table 1). Caveat: three protocols are flagged exploratory.

### 204. 2608.22688 — FashionKG-RAG
- Kind: method
- Approach: A fashion graph of triples from textbooks, DBpedia and reviewed LLM expansion. Retrieval takes paths from query entities, ranks them against a simplified "query skeleton", then an LLM agent scrutinizes candidates against the full query.
- LLM: index = yes; query retrieval = yes.
- Decision: the simplified query drives recall and the original query drives the final check, so constraints are not lost (p. 6).
- Novelty: 1 — two-stage path reranking.
- Evidence: curated fashion QA set; better than non-RAG and KG-RAG baselines (abstract). Caveat: numbers not in the pages read.

### 205. 2608.23011 — DAGC: coarse indexing, fine evidence
- Kind: method
- Approach: For video. Visually similar neighbouring chunks are merged into coarse graph nodes that keep a mapping to the original chunks. Graph retrieval locates candidate regions on the coarse graph, which are expanded back to fine chunks for answering.
- LLM: index = yes; query retrieval = not stated.
- Decision: only adjacent chunks above a similarity threshold merge, up to a maximum span, so the index adapts to local redundancy and original evidence is kept (p. 5).
- Novelty: 3 — index granularity is decoupled from evidence granularity.
- Evidence: MLVU, VideoMME, LongVideoBench; 45-47% of the nodes with about 99% of the performance and 1.3-1.7x speedup (Table 1). Caveat: one graph RAG backbone.

### 206. 2608.25123 — SelfGraphRAG: synthetic QA supervision
- Kind: method
- Approach: LLM-extracted, canonicalized entity-relation graph. An LLM writes multi-hop and neighbourhood questions from graph substructures, and those train a graph retrieval model that selects query-relevant subgraphs.
- LLM: index = yes; query retrieval = no.
- Decision: synthetic multi-hop questions stop at two-hop paths, to bound enumeration cost (p. 8).
- Novelty: 2 — the graph generates the supervision for its own retriever.
- Evidence: MoreHopQA; accuracy 4.61 and F1 12.59 against 3.85 and 9.24 for GraphRAG (Table 1). Caveat: very low absolute scores; same corpus for training and test.

### 207. 2608.25489 — Parametric knowledge graph memory
- Kind: method
- Approach: Converts each entity's k-hop neighbourhood into text and trains one LoRA adapter per entity. At query time adapters are selected and their weights injected. Retrieving substitute adapters by embedding or weight-geometry similarity was tested and failed.
- LLM: index = yes; query retrieval = no.
- Decision: one adapter per entity makes the graph parametric memory, with no subgraph text in the context (p. 3).
- Novelty: 3 — the graph is stored in weights; a reported negative result on similarity retrieval.
- Evidence: MetaQA, 150 single-film entities; +0.243 exact match on single-valued relations (Table 1). Caveat: 150 entities.

### 208. 2608.25952 — Spatial knowledge graph agents for livability
- Kind: application
- Approach: Spatial graph of residents, households, residences, facilities, categories and road hubs. Retrieval follows household members to their residence and then to nearby facilities ranked by approximate walking time; the LLM generates daily schedules from that context.
- LLM: index = no; query retrieval = no.
- Decision: at most ten facilities per category within 3 km are retrieved, which bounds the schedule context (p. 9).
- Novelty: 1 — household-centred spatial graph retrieval.
- Evidence: Shenzhen prototype (abstract). Caveat: one neighbourhood, simulated interviews.

### 209. 2608.25960 — LivingRAG: reusing graph RAG experience
- Kind: system
- Approach: Graph of passage, sentence and entity nodes built with lightweight entity extraction; the base system propagates query activation and ranks passages with personalized PageRank. Verified activation maps from past queries bias later retrieval, and stored reasoning summaries scaffold generation.
- LLM: index = no; query retrieval = no.
- Decision: activation-map reuse (for retrieval) and reasoning-summary reuse (for generation) are separate stores, because they transfer different signals (pp. 4-5).
- Novelty: 2 — quality-gated reuse of past retrieval activations.
- Evidence: 2Wiki, HotpotQA, MuSiQue, WixQA; best overall accuracy and 22.7% fewer completion tokens than LinearRAG (Tables 2-3). Caveat: WixQA compared with LinearRAG only.

### 210. 2608.25986 — Multi-granularity context for multimodal graph RAG
- Kind: method
- Approach: Builds a multimodal graph from document text and images and fuses the text graph with the image graph. The contribution adds local (reference-based) and global (document) context during image-graph construction and fusion.
- LLM: index = yes; query retrieval = not stated.
- Decision: image-to-graph construction gets local plus global context, but fusion gets only local context, to keep entity alignment focused (p. 7).
- Novelty: 2 — textual context of two granularities during construction.
- Evidence: vision-centric subset of MMLongBench-Doc; improvement across methods (abstract). Caveat: selected subset, no numbers in the pages read.

### 211. 2608.28766 — Hybrid GraphRAG for climate Q&A
- Kind: application
- Approach: LLM-extracted entities and relations, Leiden communities with bottom-up summaries. Retrieval combines vector hits with community summaries and reranks them with a cross-encoder.
- LLM: index = yes; query retrieval = no.
- Decision: vector hits and community summaries get equal weight in reranking, to mix local and global information (p. 3).
- Novelty: 1 — hybrid retrieval with cross-encoder reranking.
- Evidence: 22 questions on five articles; 160% higher contextual relevance and 177% higher contextual recall than classical RAG (Fig. 5). Caveat: 22 questions, LLM judges.

### 212. 2609.00050 — Agentic cloud workflow engineering
- Kind: tangential
- Approach: Cloud workflow orchestration. Its graph is a workflow graph of states, transitions, dependencies and verification gates; graph-based retrieval is mentioned as one possible workflow.
- LLM: index = not stated; query retrieval = not stated.
- Decision: none: standard pipeline.
- Novelty: 0 — GraphRAG is a listed application domain.
- Evidence: 140 tasks, 14 domains, six conditions (section 4.2). Caveat: no GraphRAG-specific evaluation.

### 213. 2608.30468 — Hi-Q: hierarchical query refinement
- Kind: method
- Approach: Builds no corpus graph. It builds a query tree: nodes are query units, and dependency-preserving binary splits are edges. Retrieval tests whether the evidence resolves each unit, and only unresolved nodes are expanded.
- LLM: index = no; query retrieval = yes.
- Decision: a query unit is refined only when retrieved evidence fails to resolve it, which avoids unsupported intermediate steps and needless decomposition (pp. 2-3).
- Novelty: 3 — the structure grows on the query side, driven by evidence.
- Evidence: MuSiQue, HotpotQA, 2Wiki; average 52.3 EM and 64.0 F1 in full-corpus retrieval (Table 1). Caveat: sampled questions; several baseline numbers quoted from prior work.

### 214. 2609.29695 — EvLink: evidence linking
- Kind: method
- Approach: Targets graph paths that are reachable but not supported by evidence. Passages are nodes; an edge joins two passages through a source-grounded extracted relation or endpoint alignment, with the supporting fact attached. Bounded breadth-first search finds bridge passages, then evidence-need mining and noisy-OR coverage select a compact set.
- LLM: index = yes; query retrieval = yes.
- Decision: passages stay the retrieval unit and every link must carry textual support (pp. 2-3).
- Novelty: 2 — passage graph whose edges are supported transitions.
- Evidence: three multi-hop and two simple QA benchmarks; average gains of 2.4 R@5, 1.9 EM and 2.4 F1 over leading GraphRAG baselines (Table 1). Caveat: two baseline results taken from the original papers.

### 215. 2609.00513 — ISO-RAG: isoperimetric filtering
- Kind: method
- Approach: Targets semantic drift from broad diffusion. Passages are nodes, joined when they co-occur in training instances. It takes dense seeds, builds a local graph from a dense pool plus k-hop neighbourhoods, filters edges by node-wise isoperimetric profiles, then runs deterministic personalized PageRank.
- LLM: index = no; query retrieval = no.
- Decision: edges are filtered before PageRank by the structural compatibility of their endpoints, which blocks diffusion into generic hubs (p. 4).
- Novelty: 2 — topology-based edge pruning before local PageRank.
- Evidence: HotpotQA, 2Wiki, MuSiQue; average gains of 10.0% retrieval recall and 4.3% exact match (Tables 1-2). Caveat: edges come from training-instance co-occurrence; 1,000 validation items per dataset.

### 216. 2609.00604 — NeuroGraph threat reasoning
- Kind: application
- Approach: Multi-hop cyber-threat reasoning over a curated graph of assets, vulnerabilities, weaknesses and attack techniques. An LLM writes ontology-aware Cypher; if symbolic retrieval fails, embeddings find anchors for another Cypher query.
- LLM: index = no; query retrieval = yes.
- Decision: the embedding fallback may only locate anchors; evidence is admitted only after an ontology-compliant Cypher query executes, which keeps symbolic traceability (pp. 2-3).
- Novelty: 2 — symbolic first, embeddings only to find entry points.
- Evidence: CTI-RCM 2024 and 2021 and CTI-ATE; accuracy 87.60%, 90.40% and 77.15% (Table 4). Caveat: assumes a curated graph.

### 217. 2609.00761 — Agent-enhanced academic graph RAG
- Kind: method
- Approach: Author, Paper and Venue nodes with authorship and venue edges. An LLM classifies the query to choose a traversal, a reranking agent expands the subgraph when evidence is insufficient, and a verification agent checks answers against graph facts.
- LLM: index = no; query retrieval = yes.
- Decision: at most two adaptive expansions, to bound latency and keep the subgraph compact (p. 3).
- Novelty: 1 — query-aware traversal with sufficiency control (the explorer scored 2).
- Evidence: OpenAlex 76.68% and DBLP 73.43% accuracy (Table 1). Caveat: 400 generated queries per graph, answers extracted from the same graph.

### 218. 2609.02011 — Seed-anchored graph rendering
- Kind: method
- Approach: Targets answer evidence that a fixed context budget cuts off. Nodes and relations come directly from typed power-grid models (CIM/CGMES), with no extraction. The subgraph is rendered deterministically: query-seed descriptions and their incident edges first, then outward by hop distance.
- LLM: index = no; query retrieval = no.
- Decision: locality is enforced at the rendering boundary; if the seed-local text fits the budget, the answer-bearing units are guaranteed to survive truncation (pp. 3-4).
- Novelty: 3 — a guarantee about serialization order, not about retrieval.
- Evidence: 100 fresh items of one grid family; accuracy from 0.450 to 0.970 at 8,000 characters (section 6.2). Caveat: one topology family, one 7B reader.

### 219. 2609.03857 — GRASP: multi-topic essay grading
- Kind: application
- Approach: Reference-answer nodes are linked when SBERT cosine similarity is at least 0.42; edges at 0.70 or more are "strong". Retrieval takes the top cosine matches as seeds and expands one hop over strong edges; Hungarian assignment maps references to answer segments.
- LLM: index = no; query retrieval = no.
- Decision: plain RAG and graph RAG get the same candidate budget, k = n + 2, which isolates the retrieval effect (p. 8).
- Novelty: 1 — one-hop expansion on a similarity graph.
- Evidence: synthetic essays from SciEntsBank; complete gold-node retrieval 62.5% against 25.6% at n = 3 (section 5.2). Caveat: 79 reference nodes, synthetic essays.

### 220. 2609.05139 — Spatio-temporal GraphRAG
- Kind: method
- Approach: For narrative QA where relations change over time and depend on place. Edges are extracted relations with evidence spans, two time coordinates and spatial attributes. The query's time and place constraints select a slice of the graph.
- LLM: index = yes; query retrieval = yes.
- Decision: both a chapter coordinate and a narrative-event coordinate are stored, because chapter boundaries alone mislocate events that span chapters (p. 9).
- Novelty: 2 — time- and place-scoped graph slices.
- Evidence: Red-Chamber-QA, 120 held-out questions; 0.733 against 0.675 for the window baseline, p = 0.092 (abstract). Caveat: the difference is not significant.

### 221. 2609.10237 — Answer paths and grounding in graph question answering
- Kind: benchmark
- Approach: A controlled study of how triple selection, serialization, order and instructions change graph RAG answers. Each question's subgraph comes from its gold SPARQL path; no retriever is evaluated.
- LLM: index = no; query retrieval = no.
- Decision: keep the answer path in the context. Removing it caused large accuracy losses, while replacing the distractor triples did not (pp. 1-2).
- Novelty: 3 — retrieval precision has no measured effect as long as the answer path stays in.
- Evidence: 16 experiments, 30,841 trials, six models; lowering context precision from 1.00 to 0.51 changed F1 by +0.003 (Table IV). Caveat: reasoning depth is aliased with the benchmark.

### 222. 2609.10239 — LiteRAG: cost-efficient graph retrieval
- Kind: method
- Approach: Reuses an existing Microsoft GraphRAG index. Semantic and lexical anchors seed a bounded expansion with adaptive thresholds and hub penalties, which produces compact reasoning chains for the prompt.
- LLM: index = yes; query retrieval = no.
- Decision: change only retrieval and context construction and leave indexing alone, so the comparison isolates those stages (p. 5).
- Novelty: 2 — adaptive exploration plus reasoning-chain context.
- Evidence: DistComp quality 0.798 at over 100 times lower latency and over 99% lower cost than GraphRAG Global and DRIFT; UltraDomain quality 0.801 with about 14 times fewer tokens than LinearRAG (Table 1). Caveat: DistComp is the authors' benchmark; the ablation uses 32 queries.

### 223. 2609.11065 — MOSAIC: per-query graph retrieval policies
- Kind: method
- Approach: An LLM analyzer emits a validated per-query policy that sets seeds, traversal, stopping and evidence selection. The graph, scoring, grounding and answer generator are shared across queries.
- LLM: index = yes; query retrieval = yes.
- Decision: one analyzer sets all four stages jointly, and its output is validated against bounds before use (pp. 2-3).
- Novelty: 2 — joint per-query control of seeds, traversal, stopping and selection.
- Evidence: GraphRAG-Bench correctness 76.97 on Medical and 64.33 on Novel; Medical evidence recall 95.1 (Tables 4-5). Caveat: the baselines were not rerun in the same implementation.

### 224. 2609.13324 — Graph RAG data integrity diagnostics
- Kind: method
- Approach: A diagnostic pipeline that separates three error sources: retrieval reasoning, defects in the graph data and Cypher generation errors. The graph is a spatial and temporal ecological knowledge graph; its schema is not stated in the pages read.
- LLM: index = not stated; query retrieval = yes.
- Decision: isolate one error source at a time by holding the rest of the pipeline fixed (p. 3).
- Novelty: 2 — shows that model knowledge can mask graph defects.
- Evidence: ecological graph of Southeastern Tibet; injecting 20% defects cut accuracy from 0.93 to 0.39-0.62 (Conclusion). Caveat: one domain and a judge model; the explorer reported garbled figure text in this PDF.

### 225. 2609.12464 — Hierarchical graph RAG for code migration
- Kind: application
- Approach: Tree-sitter turns the abstract syntax tree into a property graph with inheritance, call and import edges. The architecture skeleton is serialized into a context cache.
- LLM: index = no; query retrieval = no.
- Decision: translate parents before children, so child code can use the generated parent implementations (p. 2).
- Novelty: 1 — syntax-tree hierarchy applied to migration context.
- Evidence: spring-petclinic-genai; API hallucinations fell from 56.4% to 16.2%, CodeBLEU 90.6% against 91.1% (section 6.2). Caveat: one repository.

### 226. 2609.18099 — EffiRAG and the cost of structure
- Kind: method
- Approach: LLM-extracted entity and relation records link to their source chunks. Vector similarity retrieves records and chunks, then a budgeted coverage selection picks the context.
- LLM: index = yes; query retrieval = no.
- Decision: the graph only locates evidence; the source chunks stay the answer evidence, because relation records drop details such as negation and uncertainty (p. 3).
- Novelty: 1 — graph records as handles to source chunks, with ingestion and query cost priced.
- Evidence: UltraDomain; preferred over LightRAG-hybrid on 93 of 120 queries; total cost $0.408 against $0.952 (Table 2). Caveat: 120 model-generated queries and an LLM judge.

### 227. 2609.18317 — Knowledge graphs for cultural question answering
- Kind: application
- Approach: KGGen extracts triples from Spanish Wikipedia articles. G-Retriever selects a query subgraph with a prize-collecting Steiner tree and encodes it for the LLM.
- LLM: index = yes; query retrieval = no.
- Decision: a graph built with knowledge of the benchmark serves as an upper bound, and is declared non-deployable because it uses the evaluation questions (Appendix B.2).
- Novelty: 1 — automatic graph construction tested on a cultural domain.
- Evidence: LatamQA; 72% error reduction over the base model; oracle retrieval shows extraction is the limit (Table 6). Caveat: one article supports each question.

### 228. 2609.19417 — TrioRAG: multimodal retrieval without a graph
- Kind: method
- Approach: No graph. A shared multi-vector index holds page text and images. Three retrievals (question text, anchor image, and a query enhanced by a vision-language model) are fused by reciprocal rank fusion.
- LLM: index = no; query retrieval = yes.
- Decision: reciprocal rank fusion needs no score normalization and limits how far any one signal can dominate when the signals are unevenly reliable (p. 3).
- Novelty: 3 — competitive cross-document retrieval with no graph construction.
- Evidence: three benchmarks; inference 1.6-2.3 times faster; on AutoQA image recall alone was 19.3% and the combined signals reached 87.1% recall@10 (Table 6). Caveat: AutoQA is model-curated; human validation found that 46% of questions strictly need more than one document.

### 229. 2609.19622 — Geometric gain graph for multi-hop retrieval
- Kind: method
- Approach: A document-only graph with passages as nodes. The edge weight is a cosine-angle product that combines similarity with novelty, with a degree penalty for hubs. A filtered seed set gets one step of controlled diffusion.
- LLM: index = no; query retrieval = yes.
- Decision: one diffusion step replaces Personalized PageRank, because multi-hop walks over-smooth these edge weights (p. 7).
- Novelty: 3 — an LLM-free passage topology that rewards novelty as well as similarity.
- Evidence: MuSiQue, 2WikiMultiHopQA, HotpotQA; average F1 66.49 and 64.86 under two embedding models (Table 1). Caveat: 1,000 sampled cases per benchmark.

### 230. 2609.20754 — RAFT: stateful troubleshooting retrieval
- Kind: method
- Approach: Each troubleshooting case is a directed chain of timeline entries. Retrieval matches the state of the active case to entries and returns the parent trajectory. An optional graph links similar cases.
- LLM: index = yes; query retrieval = no.
- Decision: retrieve at the level of one timeline entry and return the surrounding case, so a match reflects the current stage of the investigation (pp. 2-3).
- Novelty: 2 — intermediate case states as the retrieval unit.
- Evidence: synthetic benchmark Case Hit 0.842-0.888 (Table 2); Jira +10.5 to +17.3 points over plain RAG (Table 4). Caveat: 30 Jira groups and no confidence intervals.

### 231. 2609.35810 — TRACE: structure enhancement for oncology
- Kind: method
- Approach: A concept tree and a relation graph cover the same oncology concepts. Both are built from oncology text and refined using model loss. Lexical and endpoint matching, weighted by edge confidence, selects tree paths and relations.
- LLM: index = yes; query retrieval = no.
- Decision: support for a descendant also raises support for its ancestors, so retrieved paths keep their clinical context (p. 3).
- Novelty: 2 — offline updates of edge confidence from model loss.
- Evidence: ten oncology classification tasks and MedQuAD CancerGov; zero-shot macro accuracy 0.748 against 0.728 for the base model (Table 1). Caveat: no clinical expert checked the evidence.

### 232. 2609.24346 — LADDER: graph retrieval during diffusion decoding
- Kind: method
- Approach: An entity-document graph. A diffusion language model decodes in parallel; each newly detected entity triggers propagation by a graph foundation model and a top-K document retrieval that guides the later denoising steps.
- LLM: index = not stated; query retrieval = no.
- Decision: the entity set only grows, so a speculative mention that disappears and comes back does not trigger a repeated refresh (p. 5).
- Novelty: 3 — retrieval is triggered by events inside parallel decoding, not run before it.
- Evidence: HotpotQA, 2WikiMultiHopQA, MuSiQue; average exact match 45.2% against 39.6%, latency 2.79 against 11.30 seconds per question (Table 2). Caveat: MuSiQue F1 stays below the strongest baseline.

### 233. 2609.27009 — LEGO: legal graph RAG with structured reasoning
- Kind: method
- Approach: An expert-built Civil Code graph with rule and article nodes, rule-article support edges and normative edges between provisions. Greedy retrieval picks articles for legal rules not yet covered, then prompts Provision-Fact-Conclusion reasoning.
- LLM: index = no; query retrieval = no.
- Decision: the greedy selection penalizes articles that cover an already covered rule, which gives a compact provision subgraph (p. 3).
- Novelty: 2 — coverage-based retrieval over an expert normative graph.
- Evidence: three civil law benchmarks; Qwen3-8B reaches 40.53% exact match (Table 1). Caveat: paired tests do not show superiority over GPT-5 or DeepSeek-V3.

### 234. 2609.29083 — Physical Internet literature review with GraphRAG
- Kind: application
- Approach: A Neo4j graph built with GPT-4o mini from conference proceedings and two roadmaps. Local search returns graph items close to the query; global search uses community summaries.
- LLM: index = yes; query retrieval = not stated.
- Decision: a hybrid mode combines local and global search (p. 4).
- Novelty: 1 — existing GraphRAG applied to a new literature domain.
- Evidence: IPIC proceedings and two roadmaps; hybrid search has the highest reported scores (Table 6). Caveat: no comparison with official data or expert opinion.

### 235. 2609.29282 — Asymmetric Dynamic Routing
- Kind: method
- Approach: A two-layer hypergraph: factual entities and relations below, abstract insight nodes above, linked by mappings. A classifier routes each query to local fact retrieval, bottom-up diffusion or top-down insight retrieval grounded back in facts.
- LLM: index = not stated; query retrieval = yes.
- Decision: a low-confidence routing decision falls back to the middle path (p. 3).
- Novelty: 2 — intent decides the traversal direction across layers.
- Evidence: five domain corpora; 48.7% fewer prompt tokens and 45.3% lower latency than always using the complex path, with a higher score (Table 4). Caveat: judge-based results.

### 236. 2609.35304 — Modality-aware multimodal graph RAG
- Kind: method
- Approach: A document graph with content nodes labelled by modality and fact edges. Each edge records the chunks and modalities that support it, so retrieval can filter edges by modality before ranking chunks.
- LLM: index = not stated; query retrieval = no.
- Decision: keep a fact edge when its provenance intersects the selected modalities, then use only the supporting chunks from those modalities (p. 3).
- Novelty: 2 — modality provenance on each edge.
- Evidence: MMLongBench-Doc and LongDocURL, 1,051 two-modality questions, five models; tables and text help most, and combinations are often redundant (sections 6-7). Caveat: single-modality questions are excluded.

### 237. 2609.36700 — Multi-turn degradation in RAG
- Kind: benchmark
- Approach: Eight systems, including graph retrievers built with LLM open information extraction, answer the same questions delivered as a full query, as concatenated history and as separate conversation shards.
- LLM: index = yes; query retrieval = yes.
- Decision: shards follow the question's compositional structure, so later turns can refer to entities from earlier turns (p. 4).
- Novelty: 1 — matched multi-turn evaluation of graph and other retrievers.
- Evidence: 750 questions from five multi-hop sets; mean F1 falls 11.7% with history queries and 16.3% with current-turn queries (Table 2). Caveat: the conversations are simulated.

### 238. 2609.37661 — NexusRAG: dual-path propagation
- Kind: method
- Approach: An entity-sentence-passage graph with no relation extraction. Entity neighbor weights come from co-occurrence and semantic similarity. Query-guided semantic propagation finds a frontier, structural propagation expands it, and Personalized PageRank ranks passages.
- LLM: index = no; query retrieval = no.
- Decision: the same sparse neighbor prior drives both propagation and passage initialization (pp. 5-6).
- Novelty: 2 — two propagation paths without relation extraction.
- Evidence: HotpotQA, 2WikiMultiHopQA, MuSiQue and Medical GraphRAG-Bench; evidence recall improves in all four categories (Table 2). Caveat: the medical result uses a benchmark subset.

### 239. 2610.02373 — Hop-Decayed Influence attack
- Kind: attack-or-defence
- Approach: The attack leaves source facts alone and edits the auxiliary structures added after indexing: summaries, hierarchy edges and scores. Influence is propagated from query entities over graph hops to choose what to modify.
- LLM: index = yes; query retrieval = no.
- Decision: target auxiliary structures, because one edit there affects many queries and no source document changes (pp. 3-4).
- Novelty: 2 — auxiliary index structures as an attack surface.
- Evidence: HotpotQA and 2WikiMultiHopQA on Microsoft GraphRAG and HippoRAG 2; attack success 88-94%, one setting modifies 0.016% of structures (Tables 1-2). Caveat: 1,000 sampled cases per benchmark.

### 240. 2610.09026 — BEACON-SP: clinical suicide-risk graph RAG
- Kind: application
- Approach: One graph per patient with diagnoses, medications, risk factors and events as nodes and typed relations. Retrieval finds semantic seed nodes, then runs a bounded breadth-first expansion inside that patient's graph.
- LLM: index = not stated; query retrieval = no.
- Decision: graphs are stored under patient-specific identifiers so evidence cannot cross patients (p. 6).
- Novelty: 2 — ontology-guided per-patient graph retrieval.
- Evidence: 1,500 queries over 100 patients; graph RAG preferred in 76.4% of criterion comparisons (Table II). Caveat: synthetic records and an LLM judge.

### 241. 2610.09360 — TopoGraphRAG-Bench: multimodal evidence topologies
- Kind: benchmark
- Approach: A benchmark of how well retrievers recover the topology of evidence across text, figures and tables. LLM-extracted entities and fuzzy matching link evidence units that are grounded in page layout.
- LLM: index = yes; query retrieval = not stated.
- Decision: questions are built bottom-up from evidence units, then tested counterfactually to confirm that each modality and unit is necessary (pp. 3-5).
- Novelty: 2 — topology coverage as the measured quantity.
- Evidence: 2,024 questions over 201 documents; best answer accuracy 0.535 (MegaRAG), best context recall 0.7215 (RAG-Anything) (Table 2). Caveat: model-judged metrics.

### 242. 2610.13059 — CliQ-RRG: clinical graph retrieval for radiology reports
- Kind: application
- Approach: A graph of anatomy, findings and diseases with typed medical relations. Report similarity and predicted disease labels rank seed entities; a bounded breadth-first search and ranked relation paths follow.
- LLM: index = not stated; query retrieval = no.
- Decision: seeds combine report similarity with predicted disease labels (p. 4).
- Novelty: 1 — disease-aware seeding for radiology.
- Evidence: MIMIC-CXR and IU X-Ray; BLEU-4 0.185 to 0.283 and clinical F1 0.463 to 0.610 over the base model (Table 3). Caveat: some baselines are published numbers, others reproduced.

### 243. 2610.13128 — KnowForge: specialized agents for document question answering
- Kind: system
- Approach: Atomic knowledge points linked by five typed relations (same topic, extends, contrasts, supports, prerequisite). Hybrid BM25 and dense retrieval picks eight anchors and expands one hop.
- LLM: index = yes; query retrieval = yes.
- Decision: search and reading are separate agents with disjoint tools, because a single agent kept searching in place of reading (pp. 3-4).
- Novelty: 0 — the authors' own ablation finds graph expansion neutral (p. 8).
- Evidence: M3DocVQA, MMLongBench-Doc, MultiHop-RAG; M3DocVQA F1 47.5 against 40.4 one-shot (Table 2). Caveat: most figures are single runs.

### 244. 2610.13296 — PRIME: structured document retrieval
- Kind: tangential
- Approach: Not a graph method. The index is restructured into hierarchical sections with metadata that an LLM writes per section; retrieval uses metadata embeddings, BM25 and a cross-encoder. Graph RAG appears only in related work.
- LLM: index = yes; query retrieval = no.
- Decision: per-section metadata extraction cuts embedding cost while keeping structure (pp. 3-4).
- Novelty: 0 — graph RAG is related-work context only.
- Evidence: QASPER, BRIGHT, NFCorpus; 45.8 times less embedding computation and 22.4 times fewer vectors than structure-aware chunking (Table 2). Caveat: the paper documents omission and contradiction risks in its restructuring step.

### 245. 2610.14968 — SEEK: knowledge graph retrieval for secure code
- Kind: method
- Approach: A graph of CWE weakness nodes with taxonomy edges, to which curated CVE fix examples are attached. A planner predicts weakness classes, graph scoping selects their neighborhoods, and deterministic facet ranking picks examples.
- LLM: index = yes; query retrieval = yes.
- Decision: store root cause, vulnerable snippet, fix pattern and fixed snippet, not functional descriptions, which retrieve examples that are irrelevant to security (p. 6).
- Novelty: 2 — weakness-scoped retrieval that reaches unseen weakness classes.
- Evidence: CodeGuard+ and CyberSecEval; SecurePass@1 77.45%, 85.2% on unseen weakness classes (abstract). Caveat: selected benchmarks and models.

### 246. 2610.19341 — MMKG-R1: quality-driven multimodal graph construction
- Kind: method
- Approach: A multimodal LLM is trained to extract typed entities and triples from text, images and tables separately. A fixed assembler creates fact nodes linked to arguments and source evidence.
- LLM: index = yes; query retrieval = not stated.
- Decision: the training reward includes source fidelity and graph readiness, because a correct answer does not show that the graph is good (pp. 1-2).
- Novelty: 2 — graph quality as a direct training reward.
- Evidence: MMQA, MMDocRAG, MMLongBench-Doc; improves every listed retrieval block over base GraphRAG (Table 1). Caveat: test subsets exclude text-only questions.

### 247. 2610.19549 — GraphRAG-LTM: attacks on evidence retrieval
- Kind: attack-or-defence
- Approach: Two attack layers on graphs of chunks and extracted triples. Query edits demote the seed chunks; a stronger attacker also deletes bridge triples in the query's subgraph, which breaks propagation across chunks.
- LLM: index = yes; query retrieval = yes.
- Decision: bridge deletion is paired with the same adversarial query, which isolates the effect of the graph edit (p. 2).
- Novelty: 2 — layered attack on seeds and bridges.
- Evidence: HotpotQA-Distractor, MuSiQue, 2WikiMultiHopQA; answer F1 falls from 55.8 to 23.7 in one HotpotQA setting (Table 1). Caveat: one benchmark uses a 1,000-question subset.

### 248. 2610.22818 — FuseRAG: graph evidence in latent memory
- Kind: method
- Approach: A greedy selector grows a query subgraph by query relevance, relation relevance and novelty, and stops on score or budget. Graph message passing encodes the structural context into latent memory.
- LLM: index = not stated; query retrieval = no.
- Decision: reusable evidence stays in latent memory and only selected facts are written as text, which avoids searching for a text order that suits both graph locality and key-value cache reuse (pp. 1-3).
- Novelty: 3 — moves graph evidence from prompt text into attention memory.
- Evidence: MuSiQue, HotpotQA, 2WikiMultiHopQA; up to 2.7 and 2.2 times faster time to first token with better answers (abstract). Caveat: no per-dataset scores in the pages read.

### 249. 2610.23559 — CognitiveCanvas: interactive graph memory
- Kind: application
- Approach: Dialogue is turned into Goal, Fact-triple and exact Code artifacts with semantic relations across sessions. Retrieval is described as multi-hop traversal; the scoring is not stated.
- LLM: index = yes; query retrieval = yes.
- Decision: code is kept verbatim as its own artifact, so compression of the dialogue cannot damage syntax (p. 2).
- Novelty: 1 — conversation memory schema with spatial steering.
- Evidence: within-subject study with 24 practitioners; multi-hop Recall@3 89.6% against 52.1% for flat RAG (Table I). Caveat: small study; several features are tested together.

### 250. 2610.26432 — E-Layout: graph retrieval for circuit restructuring
- Kind: application
- Approach: Layout cones and candidate logic cones are graphs. Two graph-transformer encoders score candidates by cosine similarity, and a decoder generates gate sizes and locations.
- LLM: index = no; query retrieval = no.
- Decision: e-graphs enumerate equivalent logic, and graph retrieval picks among them, because ordinary extraction costs cannot express layout timing (pp. 4-5).
- Novelty: 1 — circuit-specific retrieval schema.
- Evidence: nine designs; 6% better total negative slack and 2% better worst negative slack on average with 1% less area than a commercial flow (Table 3). Caveat: one commercial flow as the reference.

### 251. 2610.28216 — Engineering graph RAG for wind recovery
- Kind: application
- Approach: A graph of building scenarios, evidence, engineering concepts and recovery information with typed relations from damage to consequences and actions. Retrieval ranks compatible scenarios and returns supported engineering paths.
- LLM: index = yes; query retrieval = no.
- Decision: an unknown or concealed condition counts as neither support nor contradiction, which keeps partial observability honest (pp. 8-12).
- Novelty: 2 — provenance and contradiction controls for engineering inference.
- Evidence: 581 scenarios; better agreement than two baselines (abstract). Caveat: low-rise buildings hit by tornadoes only.

### 252. 2610.30611 — Structure-grounded uncertainty reduction
- Kind: tangential
- Approach: Sequential diagnosis over candidate hypothesis graphs with observation, localization, diagnosis and treatment layers. Partial graph alignment ranks candidates, and an information-gain proxy picks the next observation. Graph RAG is a baseline.
- LLM: index = not stated; query retrieval = not stated.
- Decision: alignment is partial, so incomplete observations need not match a whole candidate graph (p. 6).
- Novelty: 0 — graph RAG is only a baseline.
- Evidence: benchmark derived from PrimeKG; Hit@3 92.5%, 9.4 points above the strongest graph RAG baseline (Table 1). Caveat: the authors built the benchmark.

### 253. 2610.30710 — Graph-based over-representation analysis with RAG
- Kind: application
- Approach: Term nodes link to gene nodes by membership, and shared genes link terms across databases. A graph-based enrichment score ranks terms, then prize-collecting retrieval selects a compact connected evidence subgraph.
- LLM: index = no; query retrieval = no.
- Decision: rank terms inside each database before retrieval, because raw p-values are biased by database size and granularity (section IV).
- Novelty: 2 — prize-collecting retrieval anchored on genes.
- Evidence: 8 and 42 queries; Pearson r 0.997 with standard analysis, median 37 times faster, faithfulness 0.90 (Table II). Caveat: the LLM judge is calibrated on small human-rated sets.

### 254. 2610.30728 — Federated graph RAG
- Kind: method
- Approach: Clients keep private local graphs. Client prototypes form a routing graph on the server. Clients return abstracted evidence, and the server composes facts over an interaction graph.
- LLM: index = not stated; query retrieval = no.
- Decision: share encoded evidence abstractions in place of raw local subgraphs, to limit disclosure (pp. 4-5).
- Novelty: 2 — federated routing and cross-client composition.
- Evidence: HotpotQA, MuSiQue, 2WikiMultiHopQA; HotpotQA recall@5 44.7 and exact match 64.0 (Table 1). Caveat: the federated partitions are simulated.

### 255. 2610.33277 — GEMS-RAG for engineering standards
- Kind: application
- Approach: Nodes are sections, chunks, figures, tables and traffic-control devices; edges are containment, citation, mention and definition. Dense, sparse and visual retrieval combine with graph proximity and expansion.
- LLM: index = yes; query retrieval = no.
- Decision: visual evidence is gated per query, because unneeded images lower precision (Methodology).
- Novelty: 1 — fine-grained visual entities and rule-aware retrieval.
- Evidence: MUTCD-150; recall@5 86.7-88.0%, over 10% better overall score than baselines (Table 2). Caveat: one standards corpus, 150 questions.

### 256. 2610.33511 — TaLenT: learnable graph retriever
- Kind: method
- Approach: Over a text-attributed knowledge graph, a prize-collecting Steiner tree proposes candidate subgraphs. A fine-tuned LLM retriever then selects the evidence nodes, trained with supervision weighted by the gold answer.
- LLM: index = no; query retrieval = yes.
- Decision: the generator accepts disconnected evidence, so retrieval need not add irrelevant bridge nodes just to stay connected (p. 5).
- Novelty: 2 — gold-answer supervision for an LLM graph retriever.
- Evidence: ExplaGraphs accuracy 91.79%, WebQSP Hit@1 76.97%, CWQ Hit@1 55.71% (Table 1). Caveat: needs fine-tuning on gold answers.

### 257. 2610.33946 — Schema-free text-to-SQL from query logs
- Kind: application
- Approach: A graph is built from historical SQL logs. Table nodes carry metadata and business entities; join edges are weighted by frequency. Retrieval matches question entities to seed tables, walks join paths and ranks tables by entity match, history and Personalized PageRank.
- LLM: index = yes; query retrieval = yes.
- Decision: entity matching ranks above embedding similarity, because business terms differ from cryptic schema names (p. 4).
- Novelty: 1 — query-log structure as the graph.
- Evidence: BEAVER 95.7% executable SQL against 60.8% for embedding RAG; internal expert preference 70% against 10% (Tables 1, 3). Caveat: 35 internal questions.

### 258. 2610.34866 — UniGFM-RAG: end-to-end graph retrieval
- Kind: method
- Approach: A graph foundation model scores document nodes in a document-entity graph. A differentiable bridge injects structural prefixes into the LLM prompt, so the answer loss trains retrieval.
- LLM: index = yes; query retrieval = no.
- Decision: independent sigmoid weights replace competitive normalization, so each document keeps its own relevance (p. 5).
- Novelty: 2 — differentiable graph prefixes train retrieval and generation together.
- Evidence: three multi-hop sets and three out-of-domain benchmarks; consistent gains (abstract). Caveat: no headline number in the pages read.

### 259. 2610.35412 — Attention-steered hypergraph retrieval
- Kind: method
- Approach: Passages are vertices and entities are hyperedges. An LLM writes a description for each passage-entity incidence. At query time, scores from the LLM's addressing attention heads set the restart distribution and transition weights of Personalized PageRank.
- LLM: index = yes; query retrieval = yes.
- Decision: descriptions sit on passage-entity incidences, not on entity-entity relations, so the question can match a description and the reader resolves it in context (p. 4).
- Novelty: 3 — replaces the entity-relation graph with a description-labelled hypergraph.
- Evidence: MuSiQue, 2WikiMultiHopQA, HotpotQA; +3.5 recall@5 and +7.4 all@5 over the strongest baseline (abstract). Caveat: three benchmark corpora.

### 260. 2610.36650 — TACO-Graph: graph construction optimizer
- Kind: method
- Approach: Candidate node types, edge types and text attributes define graph variants. A small-sample evaluation scores variants, and a decision tree predicts their relative quality. Graph RAG retrieval is one of the evaluated tasks, beside node classification.
- LLM: index = no; query retrieval = no.
- Decision: proxy scores on representative subsets train interpretable trees, which avoids evaluating every construction on the full data (pp. 1, 4).
- Novelty: 2 — learns which graph construction suits a task.
- Evidence: five datasets; mean Spearman correlation above 0.8 for graph RAG retrieval (Table 3). Caveat: the graph RAG questions are generated and filtered with RAGAS.

### 261. 2610.37237 — GraphRAGProv: provenance for graph RAG
- Kind: method
- Approach: An RDF graph of extracted statements, each grounded in a source span. An LLM writes SPARQL, and query execution returns answers with provenance polynomials that point to the spans.
- LLM: index = yes; query retrieval = yes.
- Decision: duplicate triples are kept as separate occurrences, so each keeps its own source evidence (p. 2).
- Novelty: 2 — provenance polynomials from answer to source text.
- Evidence: 2WikiMultiHopQA and MuSiQue, 400 cases; 311 complete traces (Table 1). Caveat: uses supervised passages and reasoning evidence.

### 262. 2610.37728 — Knowledge graph metadata enrichment for enterprise RAG
- Kind: application
- Approach: A graph of companies, people, filings and subsidiaries with typed edges that carry the filing accession number. An LLM writes the first hop; deterministic graph expansion and text fetches follow.
- LLM: index = no; query retrieval = yes.
- Decision: people are resolved to stable identifiers before any text is read, which prevents mixing people with the same surname (section 7).
- Novelty: 1 — metadata graph expansion compared with SQL and vector retrieval.
- Evidence: SEC EDGAR; 16 of 20 questions solved on all attempts against 10 for SQL and 3 for vector (abstract). Caveat: 20 questions, one model.

### 263. 2610.37826 — DeFT-RAG: self-contained multi-hop questions
- Kind: tangential
- Approach: Not a graph method. Iterative sub-question generation over passage retrieval, with question-answer pairs kept in a table and cache. Graph RAG is a baseline.
- LLM: index = no; query retrieval = yes.
- Decision: every sub-question is self-contained, which avoids planning, dependency and rewriting errors (p. 3).
- Novelty: 0 — graph RAG is only a baseline.
- Evidence: MuSiQue, 2WikiMultiHopQA, HotpotQA; best accuracy among 13 baselines (abstract). Caveat: no result values in the pages read.

### 264. 2610.40971 — FieldGuideRAG: image-grounded species recognition
- Kind: method
- Approach: Species nodes store photographs and facts, with sibling, geographic-range and fact edges. SigLIP proposes candidates, a vision-language model checks each candidate photograph, and the scores are fused with range evidence.
- LLM: index = no; query retrieval = yes.
- Decision: candidates are verified one pair at a time against node photographs, not all at once in a single multi-image prompt (p. 3).
- Novelty: 2 — graph retrieval grounded in node photographs.
- Evidence: CUB-200, Wikidata, four iNaturalist domains; range edges add 7-19 points on iNaturalist; fact answers reach 89.5% (Tables 2, 8). Caveat: species datasets only.

### 265. 2610.41390 — AudioHelix: hierarchical dual-graph audio retrieval
- Kind: method
- Approach: Chunk, unit and concept nodes with inclusion and similarity edges, for long audio. Query relevance propagates from concept nodes; Personalized PageRank then ranks chunks and acoustic units.
- LLM: index = yes; query retrieval = no.
- Decision: text and acoustic features stay separate and meet only at the concept level, which avoids naive cross-modal matching (p. 2).
- Novelty: 2 — a hierarchy that joins acoustic and textual evidence.
- Evidence: TraceAV-Bench, MMAU-Pro, AudioMarathon; 11.1 times lower latency than full audio for Audio Flamingo 3 (Table 3). Caveat: three benchmarks.

### 266. 2611.06481 — ArcRAG: portable graph retrieval
- Kind: method
- Approach: The retriever inspects the schema of whatever graph it is given and induces a query-local interface of nearby relation descriptors and paths. An LLM plans and runs Cypher traversals, refines bad results and re-explores when evidence is thin.
- LLM: index = no; query retrieval = yes.
- Decision: the interface is induced at query time, with no offline relation canonicalization, so one retriever serves graphs built in different ways (pp. 5-6).
- Novelty: 3 — separates retrieval design from graph construction.
- Evidence: MuSiQue, HotpotQA, TriviaQA; exploration adds 12-39 accuracy points over generator-only ablations (Table 13). Caveat: ablations use 100 questions; cross-system results are not controlled.

### 267. 2611.09414 — Typed-centrality grounding for graph RAG
- Kind: method
- Approach: Over a person-object-location-event graph, nodes are partitioned by type and centralities are computed inside each type's subgraph. These measures are given to the agent beside textual similarity.
- LLM: index = no; query retrieval = yes.
- Decision: when a type has no edges to itself, two-hop bridge relations between same-type entities are projected into direct edges (p. 3).
- Novelty: 1 — type-specific centrality added to graph context.
- Evidence: POLE dataset; changed selections for two example prompts (section VII). Caveat: two examples and no question answering benchmark.

### 268. 2611.09658 — RelGuard: differentially private graph RAG
- Kind: attack-or-defence
- Approach: Documents are sanitized before indexing. Triples are extracted, candidate predicates are generated, and an exponential mechanism either substitutes the relation or removes the link. Graph RAG then indexes the sanitized text.
- LLM: index = yes; query retrieval = no.
- Decision: the perturbation includes a null outcome, so one mechanism protects both the meaning of a relation and its existence (p. 3).
- Novelty: 2 — relation-level differential privacy for graph RAG.
- Evidence: three systems, two datasets, two attacks; relation-recovery F1 falls 26.98-44.35% against the strongest baseline, answer F1 falls 2.98-8.97% (abstract). Caveat: the threat model fixes the entity endpoints.

### 269. 2611.10906 — SCorPion: controllable conversational strategies
- Kind: application
- Approach: LLM-extracted entities and relations form a graph; communities yield condition-skill-demonstration summaries. A ranker selects strategies for the query, and explicit control factors reweight category scores.
- LLM: index = yes; query retrieval = yes.
- Decision: only top-level communities produce strategy summaries, to avoid redundant abstraction (p. 5).
- Novelty: 1 — ranked strategies in place of community summaries.
- Evidence: DailyDialog, ESConv, EmpatheticDialogues and others; higher BLEU-2 and ROUGE-L than baselines on several tasks (Tables 1-3). Caveat: mixed automatic metrics and human ratings.

### 270. 2611.11010 — Lightfoot: exact model-free deletion
- Kind: system
- Approach: Passage-level extraction creates entity and passage nodes, facts with provenance and synonym edges. Retrieval seeds Personalized PageRank from matched facts and passages, then fuses its ranking with a cross-encoder.
- LLM: index = yes; query retrieval = no.
- Decision: synonym edges use a similarity threshold, not a top-k list, so deleting a neighbor cannot change the remaining links (p. 3).
- Novelty: 2 — derived graph state that is exactly rebuildable after deletion.
- Evidence: MuSiQue, 2WikiMultiHopQA, HotpotQA; the store after deletion matches a fresh index; +7.7 HotpotQA exact match over HippoRAG 2 (Table 2). Caveat: deletion exactness is checked for Lightfoot only.

### 271. 2611.15467 — AutoOntic: automatic ontology induction
- Kind: method
- Approach: Induces canonical types, a type hierarchy and domain and range constraints for relations from LLM-extracted typed triples. Hierarchy placement uses relation-argument profiles and local LLM checks, then constrains entity merging.
- LLM: index = yes; query retrieval = no.
- Decision: types are processed from general to specific, and a later type may re-parent existing nodes, which corrects earlier placements (p. 3).
- Novelty: 2 — induces and incrementally repairs a type hierarchy.
- Evidence: Text2KGBench and MINE-1; over 90% type alignment; MINE-1 retrieval accuracy 85.87%, 88% with a Wikidata seed (abstract, Table 2). Caveat: ontology alignment partly uses an LLM judge.

