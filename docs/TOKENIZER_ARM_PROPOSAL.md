# Proposal: Tokenizer Variant Arm

Date: 2026-11-15
Status: Proposed. Nothing is decided, built or measured. The owner decides whether any part proceeds.
Task: PQ-ARM in [STDIO_AUTO_INDEX_PLAN.md](STDIO_AUTO_INDEX_PLAN.md).
Source of the idea: option C of `/opt/d1/_wrk/ontocode/docs/review/ONTODOC_DOC_QUALITY_PROBE_2026_11_15.md`.
Authority it would need: an amendment to the service ADR `.memory-bank/ADR_ONTODOC_SERVICE.md` (revision 20) and a new pre-registration.

## Why

The probe and its rerun in [SMOKE_PROBE.md](SMOKE_PROBE.md) found two misses where the question used a different inflection from the target text: Q6 (`recovered` against `recover`) and A2 (`permissions`, `configured` against `permission`, `unconfigured`). They also found a query of common words only (S1) ranking a persona file first with an abstention score of 1.10. That is three cases from a smoke set written by people who knew the documents. It justifies writing a hypothesis down. It is no evidence that a variant helps.

## Current Behaviour, From Source

- `src/search/tokenize.rs` (tokenizer version 1): words are runs of alphanumerics and `_`; compound words also emit their `_` and camelCase parts; everything is lowercased. No stemming, no stop words. The same `tokenize` serves indexing and `query_terms`.
- `src/search/bm25.rs`: `idf(t) = ln(1 + (N - n_t + 0.5) / (n_t + 0.5))`, `k1 = 1.2`, `b = 0.75`.
- `src/search/mod.rs`, `abstention`: score `s = top_bm25 / Σ idf(query terms)`, or 0 when nothing matches.
- `src/store.rs`: `TOKENIZER_VERSION = 1` is stored in `IndexConfig`; load refuses any other version with "run `ontodoc index`". The service ADR (lines 196–197 at revision 20) puts the tokenizer in the index-time configuration that defines the generation id.

## The Two Questions

They are different and should be decided separately.

### R. Ranking: does a tokenizer variant raise nDCG@10 over B1?

Variants, each a new index-time system against the lexical baseline B1:

| Variant | Change | Expected effect | Risk |
| --- | --- | --- | --- |
| T1 | Remove a fixed English stop-word list on both sides | Common words stop matching | Queries that are mostly stop words become empty; phrases such as "how to" lose words; code identifiers such as `is` or `it` disappear |
| T2 | Light suffix stemming (a fixed plural and `-ed`/`-ing` rule set) | `recovered` matches `recover` | Over-merging; identifiers and paths are stemmed too unless excluded |
| T3 | T1 and T2 together | Both | Both |

Any variant needs either code written against std or a new dependency (none for stemming exists in `Cargo.toml` today); both need a task that names it. The stop-word list and stemming rules are data and must be fixed in the pre-registration, versioned, and identical for index and query.

Hypothesis in the ADR's form:

| ID | Role | Claim | Test |
| --- | --- | --- | --- |
| H-T (one per variant kept) | Confirmatory | T-variant beats B1 | Paired bootstrap (10,000) and two-sided paired randomization (10,000) on nDCG@10, answerable test queries; Holm over the variants kept; minimum effect Δ nDCG@10 = 0.05; power and `n` from dev as in the ADR's Power section |

Cost from source: a new `TOKENIZER_VERSION`, a new generation id for every corpus, a required trusted reindex everywhere, and BM25 statistics that change for every query. Every existing store is refused until reindexed, so the variant cannot ship as an option beside version 1 without a second tokenizer path in the index.

### A. Abstention: why does a common-word query score high?

From the formula above, each term adds at most `idf · (k1 + 1) = 2.2 · idf` to a document's BM25, reached at high term frequency in a short document. So `s` approaches the share of the query's IDF that the best document covers, scaled by that saturation. A document that contains every query word several times scores high whatever those words' IDF is, and common words are the ones most often found together. Removing stop words (T1) would change this as a side effect. A different abstention statistic would change it directly, but H3 is descriptive and the ADR (lines 1217–1219 at revision 20) does not let any result turn abstention on by default. This question needs its own hypothesis if the owner wants one; it is not part of R.

## What Cannot Be Claimed Today

- The 0.1.0 test split is consumed (`AGENTS.md`, held-out audit of 2026-11-13). A confirmatory claim needs fresh test queries and, on the human track, human annotation.
- The ADR says a dense leg "can be proposed after H2 resolves" (line 1231 at revision 20). It says nothing that explicitly gates a tokenizer arm, and H2 ended exploratory with no decision. Whether a new arm may be registered now is an owner reading.
- Dev tuning may use only the dev split. The smoke set in `SMOKE_PROBE.md` is not dev data and must not be used to tune a list or a rule.
- The ontocode docs corpus is in neither benchmark corpus, so no benchmark result would speak for it directly.

## Decisions For The Owner

1. Whether to pursue R at all, and if so with which variants (T1, T2, T3).
2. Whether registering a new arm now is allowed while H2 stands undecided.
3. Whether the variant would replace tokenizer version 1 (with a reindex everywhere) or ship beside it (a second tokenizer path).
4. Whether to write std-only rules or to approve a named dependency.
5. Whether A gets its own descriptive hypothesis, or stays as documented behaviour (README, "Every search response carries `abstention_score`").
6. Which labels: the human/confirmatory track or a new exploratory model-judged track, which authorizes no promotion.

Until these are answered, nothing in this file is a task.
