# ADR: Hybrid Lexical And Dense Retrieval For OntoDoc

Date: 2026-11-16
Revision: 3 (2026-11-16). Revision 3 records the answer to decision 2, no cross-language workload, and the retrieval research step 1 result for the miss class a dense leg would target; [Revision 3 Note](#revision-3-note) gives both. Revision 2 (2026-11-16, challenge review requested by the user). Revision 1 proposed three arms at one fusion setting with one bundled evaluation. Revision 2 adds a need gate before any model work, moves embedding inference out of the measured run, records what the fusion constants do to candidates found by one leg only, replaces the fail-on-oversize rule with flagged truncation, states the annotation prerequisite and defers delivered-text measurement. The [disposition table](#revision-2-challenge-disposition) lists each challenge with its evidence.
Owner: OntoDoc maintainers
Requested by: user, "make adr for hybrid proposals", following the engineering-RAG article review and discussion of vector search.

## Status

Proposed.

This document proposes a staged offline comparison of BM25, multilingual dense retrieval and their rank fusion, and records the conditions for considering production integration. It authorizes no implementation, model acquisition, corpus access, measurement, schema change, new public mode or default promotion. The [service ADR](ADR_ONTODOC_SERVICE.md) remains authoritative; existing tasks and their status are unchanged.

Acceptance must distinguish the offline experiment from production integration, and each needs scoped tasks. The service ADR excludes embedding dependencies from the retrieval path and permits a dense leg only as a new preregistered hypothesis after the original graph experiment resolves. Its completed model-judged evaluation supplies no hybrid evidence. The [decisions required at acceptance](#decisions-required-at-acceptance) are unresolved; none is a choice for an implementer to make.

## Context

The [engineering-RAG review](../docs/HABR_RAG_REVIEW.md) separates source fidelity, retrieval, context delivery and applicability. Vector search addresses retrieval failures caused by vocabulary or language mismatch. It cannot reconstruct corrupted table cells, establish that a component specification applies to a product, or certify a generated answer.

Current source gives a concrete reason to investigate a dense candidate source, and shows what any comparison must match:

- `src/search/tokenize.rs` lowercases Unicode alphanumeric runs and splits identifiers. It has no stemming, stop words, translation or semantic matching. An inflected form is a different token from its base form.
- `src/search/mod.rs::SearchIndex::search` seeds the graph from the top ten positive BM25 matches. With no lexical hit and no threshold, there are no seeds and the candidate pool stays empty. With a threshold, abstention returns `no_lexical_match` before expansion. Hierarchical scoring uses the same pool. A dense score applied only to that pool cannot address the gap.
- `src/eval/run.rs::rank_sections_to` takes 50 ranked chunks (`FETCH`) and keeps each section's first occurrence. It calls `SearchIndex::search` directly; response assembly is outside the evaluation path.
- Service ADR revision 21 makes search return `unmatched_terms`: the query terms that occur in no indexed chunk of the corpus. A client can already see which of its terms the corpus lacks. The field is no relevance or abstention signal.
- `src/assemble.rs` and `src/cli.rs` control what ranked evidence reaches the client. More candidate recall does not imply that the necessary source text fits in the delivered response.
- Exact identifiers, versions, quantities and negation matter in technical documentation. Embeddings may rank a similar but inapplicable passage highly.

These observations establish a mechanism. They do not establish how often it occurs or that a dense leg helps. The only recorded evidence about retrieval failure is service ADR revision 15: 12 answerable queries were withdrawn because no pooled system surfaced a relevant section at depth 30. Whether the corpus answers those queries is unknown.

## Options Considered

| Option | Benefit | Limitation | Proposed disposition |
| --- | --- | --- | --- |
| Retain BM25 and the existing graph modes | No new inference, storage or model lifecycle | Vocabulary and language gaps remain | Production baseline and experimental control |
| Let the client reformulate, using `unmatched_terms` | Available today with no service change; a model-driven client can translate or rephrase and search again | Depends on the client; several calls per question; outside the service's reproducible evaluation | Record as the existing alternative; state in every report that single-query metrics do not measure it |
| Add stemming to the tokenizer | Closes inflection gaps within one language without a model | Does nothing across languages; changes the tokenizer version and every generation ID; can merge distinct identifiers | Separate ranking proposal with its own dev evidence; consider first if stage 0 shows an inflection gap |
| Add a curated glossary or deterministic query expansion | Small intervention for a finite known terminology gap | Needs maintenance; ambiguous expansions can hurt precision | Consider if stage 0 shows a small bounded gap; a comparison arm must be registered before use |
| Replace BM25 with dense retrieval | Finds passages with little lexical overlap | Can lose exact identifiers and retrieve plausible wrong versions | Diagnostic arm only; no replacement proposal |
| Retrieve BM25 and dense candidates independently, then fuse ranks | Tests complementary retrieval while keeping a lexical contribution | More candidates, inference cost and rank displacement | Preferred experiment, after the need gate |
| Add ANN storage, reranking, graph fusion and new chunking together | Several possible improvements | Confounded effects and a much larger operational surface | Deferred; no vector database or ANN index in the first experiment |

## Proposed Decision

### Staged Gates

The work stops at the first stage whose stop condition holds. Stop conditions and their thresholds are registered before the stage's measurement.

| Stage | Question | New inputs | Stop when |
| --- | --- | --- | --- |
| 0. Need | How often does BM25 miss a judged-relevant section, and does a real workload include cross-language queries? | None: the existing dev split and `eval misses` | The registered need threshold is not met and the owner states no cross-language workload |
| 1. Preparation | Can every input be embedded within the model limit, and can the vectors be pinned? | Token census, pinned model and embedding tool, frozen vectors | Licences, limits or identities cannot be pinned |
| 2. Dev | Does a hybrid help on fresh dev queries, and how many test queries does power need? | Fresh reviewed queries and pooled labels | The dev mean difference is at or below 0, or the required sample exceeds the annotation the owner will fund |
| 3. Held-out | One registered attempt | New custody and registration | Any integrity gate fails |
| 4. Production | Separate service-ADR amendment | Evidence from stage 3 | Any production gate below is unmet |

### Stage 0: Need Before Any Model Work

Stage 0 uses existing tooling and the existing dev split only. It reports the share `p` of answerable dev queries that have a judged-relevant section outside the BM25 top ten, by attributed miss stage (`lexical_miss`, `chunk_boundary`, `truncated`).

`p` bounds what candidate recovery can gain. A query's nDCG@10 gain is at most 1, so recovering every missed section raises mean nDCG@10 by at most `p`. If `p` is below 0.05, recovery alone cannot reach the 0.05 minimum effect. A dense leg can also reorder sections BM25 already returns; that gain is not bounded by `p`.

Three limits apply. The existing qrels were pooled from B0–B3 only, so a relevant section that only a dense model would surface was never judged and `p` is a lower bound. The existing benchmark has no cross-language stratum (its strata are lookup, conceptual, multi-hop and off-topic), so stage 0 says nothing about cross-language need. Service logs keep no raw queries, so no usage data exists; a cross-language workload has to be an owner statement about real corpora and users.

### Experiment Boundary

Use a trusted offline evaluation path over explicitly approved, pinned Markdown corpora. Reuse current sections, chunks, heading paths, citations and BM25. The frozen 1,200-byte chunk budget, tokenizer, source text, tier construction and graph semantics stay unchanged. PPR and document-score blending stay out of the hybrid arms so that dense retrieval is the only new signal.

Embedding inference is not part of the measured run. A pinned external tool produces document vectors and query vectors in the trusted preparation step. That step reads the queries of both splits, as the service ADR already allows preparation to do, and produces no rankings. The measured run loads hash-bound vectors and computes similarity and fusion. The evaluation binary therefore needs no inference dependency, and its output is deterministic for the frozen vectors. Inference time is measured separately as a preparation observation.

One multilingual model is used. Multilingual E5 is a candidate family; no artifact is selected and no licence is approved. The prototype performs exact similarity search over a bounded per-corpus vector matrix. It adds no database dependency, remote inference, outbound network access, MCP tool, HTTP endpoint or public search-mode value.

### Arms And Fusion

These names identify proposed experiment arms. They are not CLI options and do not replace historical hypothesis IDs.

| Arm | Candidate construction | Ordering |
| --- | --- | --- |
| `X-BM25` | Positive-score BM25, top 50 chunks | BM25 score descending, chunk ID tie-break |
| `X-DENSE` | Exact cosine, top 50 chunks of the same corpus | Cosine descending, chunk ID tie-break |
| `X-HYBRID-k` | Union of the two top-50 lists, at most 100 chunk IDs, truncated to the top 50 fused chunks | Reciprocal rank fusion with constant `k`, chunk ID tie-break |

Ranks are one-based:

```text
RRF_k(c) = [c in B] / (k + rank_B(c))
         + [c in D] / (k + rank_D(c))
```

An absent chunk contributes zero. A chunk in both lists occurs once and receives both terms. Raw BM25 and cosine scores are never added, read as probabilities or weighted per query. A dense-only chunk stays eligible when BM25 returns nothing.

Every arm passes at most 50 ranked chunks into the existing first-occurrence section projection, as `rank_sections_to` does for the current systems. Section metrics are computed at ten. A query with fewer than ten distinct sections is recorded as such; its depth is not changed.

Fusion is at chunk level because the service returns chunks. When the two legs find different chunks of one section, that section earns no agreement credit. The dev report counts the queries where this happens.

### What The Fusion Constants Do

With `k = 60` and depth 50, every chunk found by both legs outranks every chunk found by one leg. The highest score a single-list chunk can have is rank 1, and the lowest score a double-list chunk can have is rank 50 in both:

```text
single, rank 1:          1 / 61            = 0.016393
double, ranks 50 and 50: 1 / 110 + 1 / 110 = 0.018182
```

This holds whenever depth < k + 2. Under revision 1's single setting, a relevant chunk that only the dense leg finds ranks below every chunk both legs list. That is the case the experiment exists to test: BM25 matched the query's incidental shared tokens, and the dense leg alone found the answer. The same property pushes a BM25-only exact-identifier hit below all agreed chunks. When the agreed chunks cover ten or more sections, a section found by one leg only cannot reach the top ten.

With `k = 10`, a single-list rank 1 scores 1 / 11 = 0.090909, and a double-list chunk at ranks 50 and 50 scores 2 / 60 = 0.033333. A chunk at the same rank `a` in both lists outranks a single-list rank 1 only when `a` is below 12.

The dev grid is therefore `k` in {10, 60}. Both settings are pooled on both splits. Dev selects the higher mean nDCG@10, and an exact tie selects 60. Only the selected setting enters the confirmatory contrast. The dev report gives, per stratum, the number of agreed chunks per query and the number of BM25 top-ten sections each setting displaces. Depth 50 and these two constants are experimental choices and no measured optimum. Any other value, model or preprocessing is a new declared dev configuration with its own pool, logged before evaluation. Nothing is selected on held-out results.

The hybrid searches two lists and each single arm searches one. Reports state this extra candidate and compute budget.

### Embedding Input And Similarity

Embed the chunk text and heading path that BM25 indexes, in one documented serialization with the model's required query and document prefixes. Freeze those bytes, the prefixes, tokenizer version, pooling, normalization, numeric precision and similarity function before measurement. Vectors from different models or preprocessing versions are never mixed.

A block larger than the chunk budget becomes one oversized chunk (`src/chunk.rs`), so a corpus with a large table or code block contains inputs above a 512-token model limit. Revision 1 failed the whole configuration on the first such input, which would stop the experiment on such a corpus. Revision 2 replaces that rule:

- Stage 1 starts with a token census: the token length of every serialized chunk input under each candidate tokenizer, with the count and byte share above the limit. The census needs no inference and is recorded before a model is selected.
- An over-limit chunk is embedded from its leading tokens up to the limit and carries a `dense_truncated` flag. The count is reported, and a miss whose relevant chunk is flagged gets its own attribution stage.
- An over-limit query fails that query's dense leg with a recorded reason and stays in every denominator.
- No chunk or query is removed to make the model fit. Windowing or pooling over a long chunk is a different representation and a separate comparison.

Vectors must be finite, of the declared dimension and of nonzero norm. They are normalized explicitly. A malformed, missing, duplicate or extra chunk-to-vector mapping refuses the run. Similarity sums in index order at a fixed precision, so equal inputs give equal bytes.

The frozen vector bytes are the artifact of record. Preparation regenerates them once in the pinned environment and records whether the bytes match, and if they differ, the largest absolute difference and whether any top-50 list changes. A model is installed only in an explicitly authorized preparation step, with no automatic download and no remote code execution.

### Abstention And Failure Semantics

Ranking comparisons run with abstention off, as in the existing evaluation contract.

A query with no lexical match is either a question the corpus answers in other words or another language, or a question the corpus does not answer. BM25 cannot tell these apart, and today a threshold returns the typed `no_lexical_match` for both. A dense leg returns ten neighbours for both. Adopting a hybrid therefore gives up that typed refusal unless a new signal replaces it, and this experiment selects none. The BM25 threshold is no calibrated hybrid confidence, and an RRF score depends on ranks and list membership.

With abstention off, the dense and hybrid arms return results for every unanswerable query by construction. The unanswerable queries serve one descriptive report: the distribution of top-1 cosine for answerable and unanswerable queries, with no threshold chosen. A future abstention policy needs its own dev calibration, a frozen policy and evaluation of false abstention and false acceptance.

A missing model, corrupt vector file, exceeded cap or failed inference is a typed failure. A hybrid run never continues as BM25 under the hybrid label. All failures and refusal counts are recorded. A production fallback would need an explicit visible contract and its own evidence.

### Scope, Identity And Resource Bounds

Each arm uses the same corpus. Where a path scope applies, it is applied before top-k selection in each leg; filtering after a global search discards in-scope candidates. Candidate pools and ranking statistics are never shared across corpus endpoints.

Evaluation vectors are stored outside production `.ontodoc/` stores and outside source checkouts, in an approved experiment directory. The bundle binds the corpus snapshot, generation and artifact hashes, sorted chunk IDs and input hashes, model and tokenizer hashes, the embedding tool identity and the full configuration. Corpus identity is part of every cache key. A mismatched or incomplete bundle refuses the run. Production schema 4 is unchanged by the experiment.

Acceptance needs numeric limits for corpus chunks, vector dimension and bytes, model and tokenizer file bytes, input bytes and tokens, batch size, threads and resident memory on named native Linux hardware. Exact scoring costs O(N × d), and a float32 matrix alone costs 4 × N × d bytes: 100,000 vectors of dimension 768 need 307,200,000 bytes before model weights, BM25, metadata or workspaces. That figure illustrates the formula and is no capacity target.

Size products and input admission are checked before allocation. A cap hit fails the measurement with explicit evidence. No unbounded queue is introduced and no smaller corpus is silently sampled.

## Evaluation Proposal

### Questions, Labels And Integrity

Use a fresh reviewed query set. Strata are assigned at writing time and never changed after a system run: exact identifier, paraphrase, same-language, multi-section, wrong-version distractor and unanswerable. Revision 3 removed the cross-language stratum under decision 2.

No usage data exists to weight the strata. The aggregate is the mean over the registered stratified sample, and every report says it is conditional on that stipulated mixture and gives each stratum separately. An oversampled stratum is disclosed with its share.

Paraphrases, translations and questions about one information need form a group, and a group stays in one split. Translations evade the token-overlap leakage check, so grouping is reviewed in addition to it. Grouping, stratification and the independent resampling unit are declared before outcomes. The consumed held-out questions are not reused, and their errors do not inform this design.

Final queries and their review provenance are approved before pooling or any exposure to rankings. Labels are section-keyed grades 0–2. A confirmatory claim needs two human annotators and the service ADR's blinding, overlap, agreement and adjudication rules. Model-drafted queries need the required independent review, and no model counts as a human annotator.

The service ADR records an owner decision of 2026-11-11 that no human annotators are available. Until that changes, this experiment can only run on the model-judged exploratory track, which gives effects and intervals, no decision and no route to production, and whose held-out run burns its split for a later human-label claim. Acceptance has to say which track is intended.

Every registered arm and both fusion settings contribute to the judgment pools on both splits. BM25-only pools would leave dense-retrieved sections unjudged. The judged@10 floor and the condensed-list sensitivity check are kept. Pooling of both splits happens only in trusted preparation; ordinary dev code loads dev data directly. Pool depth and the handling of unjudged candidates are frozen before scoring. Recall at depth 50 needs correspondingly deep judgments; top-ten pools do not establish it.

For multi-section questions, reviewers may register alternative minimum sufficient evidence sets, independently of rankings and link topology. Each set lists section IDs. The set is recovered when all its sections are in the top ten. This annotation does not replace section qrels or change their keys.

### Outcomes And Claims

| Outcome | Proposed treatment |
| --- | --- |
| Section nDCG@10, selected `X-HYBRID-k` minus `X-BM25`, answerable test queries | Sole confirmatory superiority contrast, subject to human labels and a new preregistration |
| The same difference on the exact-identifier stratum | Non-inferiority guardrail: the lower bound of the bootstrap 95% CI must be above −δ, with δ registered |
| Dense-only versus BM25; hybrid versus dense-only; the unselected fusion setting | Exploratory effects and intervals |
| Recall@10, MRR@10, sufficient-set recovery in the top ten | Secondary diagnostics; disclose pool completeness |
| Wrong-version distractors | Error types reported separately with intervals |
| Unanswerable queries | Top-1 cosine distributions only; a returned neighbour is neither an answer nor a correct refusal |
| Agreed chunks per query, displaced BM25 top-ten sections, `dense_truncated` count | Mechanism diagnostics per stratum |
| Brute-force scoring time, vector bytes, separately measured inference time | Descriptive, with corpus size, dimension and hardware; no production capacity claim |

The primary contrast uses seeded paired bootstrap and paired randomization with 10,000 replicates, stratified by corpus, resampling and sign-flipping whole groups. Pass requires a mean difference above 0, a bootstrap 95% lower bound above 0 and a two-sided p below 0.05. A nonpassing result is negative only when its upper interval bound is below the registered minimum effect, and inconclusive otherwise. If another confirmatory hypothesis is added, the family, the Holm correction and the power are frozen before test access. The historical H2 and H5 family and its results are unchanged.

A primary pass does not authorize promotion. The guardrail must also pass, and an underpowered guardrail leaves promotion unresolved: an interval that includes −δ is no evidence of no regression. Model-judged results stay exploratory whatever the effect. Retrieval metrics support no claim about generated answers.

Every miss gets one stage, in this order: absent from the index; relevant chunk flagged `dense_truncated` and missed by both legs; missed by both legs; found by a leg and displaced by fusion; beyond the 50-chunk cut. The existing `attribute_miss` covers the current systems only, and the earlier experiment's unavailable-attribution exception does not carry over. All registered outcomes, failures and strata are reported.

Delivered-text sufficiency under a byte budget is deferred. The evaluation path ranks sections and does not call response assembly, so measuring delivered text needs a second harness and a second label type. It is a separate proposal.

### Sample Size And Annotation Cost

The minimum effect of interest stays at Δ nDCG@10 = 0.05 unless the owner registers another value. With one confirmatory contrast, power 0.8 and two-sided alpha 0.05:

```text
n = ((z(0.975) + z(0.8)) × sd_diff / 0.05)²
  = (2.801585 × sd_diff / 0.05)²
  = 7.849 × (sd_diff / 0.05)²

sd_diff = 0.20 gives n = 126
sd_diff = 0.30 gives n = 283
```

`sd_diff` is the dev standard deviation of the per-group difference, and the two values above are illustrations. `n` counts independent groups. The floor of 150 answerable and 30 unanswerable test queries stays and is raised as the calculation requires. The guardrail uses the same formula with δ in place of 0.05 and a true difference of 0, so at δ = 0.05 the exact-identifier stratum alone needs as many groups as the primary contrast.

Four ranked lists are pooled per query (BM25, dense and two fusion settings), so a query contributes at most 40 sections. At the floor, 150 test and 75 dev answerable queries give at most 9,000 pairs before the second annotator's share. Overlap between lists lowers the count; a larger required `n` raises it. This labour is the largest cost of the experiment and is stated to the owner before stage 2.

### Threats To Validity

- A public corpus may have been in the embedding model's training data. The model report does not allow this to be checked. Results are given per corpus, and a public-corpus gain is not generalized to private documentation.
- Model-drafted queries and translated queries may resemble the model's training distribution more than real user questions.
- The metrics describe one query per question. A model-driven client that reformulates after seeing `unmatched_terms` may already recover some lexical misses, so single-query gains can overstate the benefit to such a client.
- A corpus set in one language cannot represent documentation written in several languages.

### Freeze And Custody

The completed registration and attempt under Git-common custody are immutable. Current tooling supports one registration; a new output directory or exported TOML grants no second attempt. A separately approved custody transition is required before any new held-out preparation or run, preserving all prior records and refusal guards.

The freeze covers code, binary, dependencies, corpus, reviewed queries, labels, model and tokenizer hashes, the embedding tool identity, vector bytes for documents and queries, the selected fusion setting, statistics, guardrail margin and seeds. Preregistration and measurement audits stay outside the frozen checkout and are mirrored after measurement. An interrupted held-out run consumes its attempt. Tuning on held-out errors burns the split.

## Existing Owners And Production Gates

This table identifies reuse points. It grants no write scope and no API.

| Owner | Responsibility if separately approved |
| --- | --- |
| `src/search/` | Pure cosine top-k and rank-fusion functions, unreachable from `Mode`, CLI and MCP during the experiment; reuse current BM25 |
| `src/eval/{run,gates,sweep,metrics,stats}.rs` | Vector-bundle loading and validation, offline arms, pools, metrics, stage attribution and the existing statistics; new custody support needs its own accepted design |
| `src/store.rs` | Any future persisted embedding schema, atomic publication and complete-generation validation |
| `src/cli.rs`, `src/mcp.rs` | Any future explicit mode, admission and error surface through the shared application path |

Production integration requires all of the following:

- A service-ADR amendment permitting local query-embedding inference and defining model acquisition and licensing. Precomputed document vectors do not remove query-time inference, and the experiment's precomputed query vectors do not exist in production.
- Deterministic inference on the deployment hardware, or an accepted statement of how nondeterministic query vectors relate to the determinism rule.
- An explicit request, default, abstention and failure contract with stdio and HTTP parity and complete-response accounting. The five tools stay. An explicit hybrid request and a changed omitted-mode default are separate decisions, and the historical promotion policy cannot select this mode.
- A persistent representation with a schema version. Vectors in production generations enter generation identity, incompatible generations are rejected, and publication stays atomic through `src/store.rs`.
- Numeric admission limits, cancellation and work ownership, and measured capacity on the deployment hardware including model startup and corpus reloads.
- A human-labelled held-out pass with the guardrail met. Promotion is a separately authorized task with a lexical rollback path.

The proposed [evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md) is independent of this one.

## Revision 3 Note

Decision 2 is answered: the workload has no cross-language queries, so the cross-language stratum and the cross-language motivation for a multilingual model are withdrawn. A model's coverage of the corpus language still matters.

The [retrieval research ADR](ADR_ONTODOC_RETRIEVAL_RESEARCH.md) step 1 split the 21 B1 dev `lexical_miss` pairs on 2026-11-16 ([audit](audit_session-2026-11-16-retrieval-research.md)). One pair, 1/21 = 0.048 with a query-cluster 95% interval of [0.000, 0.167], shares no content term with its query; 20 share a content term and rank below 50. That ADR's registered step 4 signal needs at least 0.50, so the miss class a dense leg would target has no dev evidence. Stage 0 here is unchanged and the decisions below still gate stage 1; that evidence is part of what decision 3 and the label-track decision weigh.

## Decisions Required At Acceptance

1. Whether stage 0 may run on the existing dev split, and the need threshold that stops the work.
2. Answered 2026-11-16: no cross-language workload (user, "propose answeres ( 1- NO )"). The corpus and query language stay unstated.
3. How the service ADR's "after H2 resolves" condition is met, given that H2 has an exploratory model-judged interval and no decision.
4. The label track: human annotators for a confirmatory claim, or model-judged exploration that cannot lead to production.
5. The model, tokenizer, their licences, the external embedding tool and its pinned environment.
6. The over-limit policy; flagged leading-token truncation is proposed.
7. The fusion grid; `k` in {10, 60} is proposed.
8. The guardrail margin δ and the size of the exact-identifier stratum.
9. Numeric caps and the experiment artifact directory.
10. The custody transition for a second registration.

## Required Checks Before Measurement

Focused runnable checks establish: dense-only candidate admission when BM25 returns nothing; one-based RRF arithmetic for overlap, a missing leg and both constants, including the two written-out cases above; stable ties; truncation of the fused list to 50 before section projection; scope before top-k; cross-corpus isolation; bundle identity mismatch; nonfinite, zero-norm, missing, duplicate and extra vectors; the `dense_truncated` flag and count; an over-limit query; and cap refusal. Numeric checks use expected values written out in comments. A language-mismatch fixture records what the selected model returns. Passing fixtures establish no quality claim.

Dev measurement additionally needs authentic query and label preparation and the integrity gates. Held-out measurement additionally needs the approved custody transition, frozen identities, power and the registered guardrail. This ADR creates no implementation plan and reopens no task.

## Consequences

Stage 0 can end the work before any model, label or dependency cost if BM25 misses are rare. If the work proceeds, it shows whether a multilingual dense leg retrieves useful sections that BM25 misses and whether a fusion setting keeps exact technical hits. It may show that a glossary, stemming or client reformulation covers the gap.

The experiment needs no inference dependency in the Rust package. Its costs are annotation, model licensing and supply-chain review, an external embedding tool, vector storage and a second custody design. ANN, further models, rerankers, query rewriting, table-row segmentation, equipment graphs, delivered-text measurement and generated answers stay separate hypotheses.

## Revision 2 Challenge Disposition

| Challenge to revision 1 | Evidence | Change |
| --- | --- | --- |
| No evidence of need before buying a model and labels | No stop condition existed; `eval misses` and the dev split already exist | Stage 0 with the bound gain ≤ `p`, and its three limits |
| The fusion setting defeats the motivating case | 2 / 110 = 0.018182 is above 1 / 61 = 0.016393, so agreed chunks always outrank single-leg chunks | Property documented; dev grid `k` in {10, 60}; displacement diagnostics |
| Failing on the first over-limit input stops a corpus with one large block | `src/chunk.rs` keeps a block larger than the budget as one chunk | Token census first; flagged truncation, counted and attributed |
| The confirmatory path assumes annotators the owner recorded as unavailable | Service ADR, model-judged track, owner decision of 2026-11-11 | Stated in the label section and as decision 4 |
| Inference inside the measured run adds a dependency and nondeterminism | All queries of an offline experiment are known in advance | Vectors for documents and queries are frozen in preparation; the binary needs no inference dependency |
| Hybrid input to section projection was unspecified | `rank_sections_to` takes 50 chunks; the union holds up to 100 | The fused list is truncated to 50 for every arm |
| Workload weights cannot be derived | Service logs keep no raw queries | Stratified sample with a stipulated mixture, disclosed as such |
| Unanswerable queries had no measurable outcome | Dense and hybrid return neighbours by construction with abstention off | One descriptive cosine report; the lost `no_lexical_match` refusal is stated |
| The regression guardrail was undefined | Revision 1 named tolerances without a test | Non-inferiority bound with margin δ and its sample size |
| Delivered-text sufficiency bundled a second experiment | The evaluation path does not call response assembly | Reduced to top-ten set recovery; delivered text deferred |
| Cheaper alternatives were missing | The tokenizer has no stemming; revision 21 returns `unmatched_terms` | Two options added |
| Validity threats were not listed | Public corpus, model-drafted queries, single-query metric | Section added |

## References And Evidence Limits

- [Engineering-RAG article review](../docs/HABR_RAG_REVIEW.md): motivation and synthetic cases.
- [Service ADR](ADR_ONTODOC_SERVICE.md): retrieval, transport, bounds, evaluation and custody authority, read at revision 21.
- [BEIR](https://arxiv.org/abs/2104.08663): heterogeneous retrieval evaluation in which BM25 was a robust baseline; it predicts nothing about current models or OntoDoc.
- [Multilingual E5 technical report](https://arxiv.org/abs/2402.05672): a candidate model family, with no selection, licence approval or OntoDoc quality claim.

This revision read the current `src/search/mod.rs`, `tokenize.rs`, `ppr.rs`, `src/chunk.rs`, `src/eval/run.rs` and `src/eval/sweep.rs` at HEAD `83caad9`. `rank_sections_to` takes `FETCH` = 50 chunks and keeps each section's first occurrence; `attribute_miss` attributes misses for the B0–B3 systems only; `eval misses` exists in `src/cli.rs`. The arithmetic was done by hand and is written out where used. No build, test, model download, embedding run or benchmark run was executed for this revision, and no held-out query, label or result was read. The checks run were `git diff --check` and a script checking final newline, trailing whitespace, code fences, the single title, local link targets and anchors.
