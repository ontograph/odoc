# OntoDoc 0.1.0 evaluation results

This report reproduces the accepted aggregate evaluation summary from 2026-11-13 and accompanies both local candidates. Results apply only to the original candidate `cc116c44a4d7f854bc19c6a4b40d75098a437268`, binary SHA-256 `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. Its archive is unchanged. The separately identified corrected candidate repairs transport overflow and indexing/response costs; it has synthetic validation, not a new held-out evaluation. Its source and binary identities are recorded in its package manifest.

B0 is the exact-substring baseline, B1 lexical BM25, B2 graph propagation and B3 document-hierarchy blending. H1 compares B1 with B0; H2 compares B2 with B1; H5 compares B3 with B1. E1 examines multi-hop B2 and conceptual B3 subsets.

## All reported contrasts

All intervals below are 95% paired bootstrap intervals (10,000 seeded resamples). H1 is the only decision. H2/H5 and E1 use model-judged labels, are exploratory, and have no Holm p, decision or promotion. Raw unadjusted p-values remain in the immutable JSON; they do not authorize confirmatory claims.

| Contrast | n | Mean difference | 95% CI | Role / decision |
| --- | ---: | ---: | --- | --- |
| H1 (ndcg10) | 165 | 0.719047 | [0.685364, 0.752125] | sanity / pass |
| H2 (ndcg10) | 165 | 0.032912 | [0.016128, 0.049996] | exploratory / none |
| H5 (ndcg10) | 165 | 0.032709 | [0.012113, 0.053406] | exploratory / none |
| E1-B2-multi-hop-recall (recall10) | 37 | 0.049710 | [0.000739, 0.100323] | exploratory / none |
| E1-B2-multi-hop-mrr (mrr10) | 37 | 0.003604 | [-0.045946, 0.048649] | exploratory / none |
| E1-B3-conceptual-recall (recall10) | 56 | 0.086497 | [0.039911, 0.135848] | exploratory / none |
| E1-B3-conceptual-mrr (mrr10) | 56 | 0.023661 | [-0.011012, 0.061607] | exploratory / none |
| ontoindex/H1 (ndcg10) | 74 | 0.702887 | [0.649913, 0.753992] | exploratory / none |
| ontoindex/H2 (ndcg10) | 74 | 0.036261 | [0.006176, 0.066958] | exploratory / none |
| ontoindex/H5 (ndcg10) | 74 | 0.009950 | [-0.022846, 0.042940] | exploratory / none |
| ontoindex/E1-B2-multi-hop-recall (recall10) | 11 | 0.022727 | [0.000000, 0.068182] | exploratory / none |
| ontoindex/E1-B2-multi-hop-mrr (mrr10) | 11 | 0.045455 | [0.000000, 0.136364] | exploratory / none |
| ontoindex/E1-B3-conceptual-recall (recall10) | 28 | 0.083136 | [0.011872, 0.161086] | exploratory / none |
| ontoindex/E1-B3-conceptual-mrr (mrr10) | 28 | 0.004464 | [-0.048214, 0.055960] | exploratory / none |
| rust-book/H1 (ndcg10) | 91 | 0.732188 | [0.687707, 0.774357] | exploratory / none |
| rust-book/H2 (ndcg10) | 91 | 0.030189 | [0.012337, 0.047533] | exploratory / none |
| rust-book/H5 (ndcg10) | 91 | 0.051216 | [0.025125, 0.077582] | exploratory / none |
| rust-book/E1-B2-multi-hop-recall (recall10) | 26 | 0.061126 | [-0.006733, 0.131044] | exploratory / none |
| rust-book/E1-B2-multi-hop-mrr (mrr10) | 26 | -0.014103 | [-0.076923, 0.034615] | exploratory / none |
| rust-book/E1-B3-conceptual-recall (recall10) | 28 | 0.089858 | [0.033508, 0.153675] | exploratory / none |
| rust-book/E1-B3-conceptual-mrr (mrr10) | 28 | 0.042857 | [-0.001786, 0.098214] | exploratory / none |

## System metrics

Ranking metrics use abstention off. Condensed nDCG equals nDCG for every reported system/corpus; judged@10 is 1.0 throughout. B0 returned no rows: its judged coverage is vacuous and H1 is a weak comparison against an empty exact-substring baseline.

| Corpus | System | nDCG@10 / condensed | Recall@10 | MRR@10 |
| --- | --- | ---: | ---: | ---: |
| pooled | B0 | 0.000000 | 0.000000 | 0.000000 |
| ontoindex | B0 | 0.000000 | 0.000000 | 0.000000 |
| rust-book | B0 | 0.000000 | 0.000000 | 0.000000 |
| pooled | B1 | 0.719047 | 0.784456 | 0.850462 |
| ontoindex | B1 | 0.702887 | 0.768772 | 0.830405 |
| rust-book | B1 | 0.732188 | 0.797211 | 0.866771 |
| pooled | B2@0.5 | 0.751959 | 0.829938 | 0.867912 |
| ontoindex | B2@0.5 | 0.739148 | 0.821288 | 0.846659 |
| rust-book | B2@0.5 | 0.762377 | 0.836972 | 0.885195 |
| pooled | B3@0.3 | 0.751756 | 0.827091 | 0.861582 |
| ontoindex | B3@0.3 | 0.712837 | 0.785514 | 0.839114 |
| rust-book | B3@0.3 | 0.783404 | 0.860901 | 0.879853 |

## H3 and H4

H3 uses the dev-selected threshold 0.32431970510486274. Correct abstention was 42/55 = 0.763636, Wilson 95% CI [0.636516, 0.856333]. False abstention was 13/165 = 0.078788, Wilson 95% CI [0.046620, 0.130123]. This is descriptive evidence; abstention remains disabled by default.

H4 has deterministic fixture evidence in the accepted audit: 14 citation unit tests and 12 MCP integration tests passed, covering four citation states, bounded cited fresh responses and tier verification. Candidate smoke also verified a returned citation over stdio and HTTP. These are historical checks, not rerun for this report, and do not verify every benchmark citation. Ranking TSVs contain section rankings, not citation objects.

## Limitations and defaults

All 220 test queries were model-drafted: 165 answerable and 55 off-topic. Labels were model-judged, with possible shared drafting/judging and lexical biases. Results are exploratory; there is no human-label confirmation, Holm decision or default promotion. H1's passing comparison against an empty B0 does not establish general retrieval quality.

Twelve unanswered queries were withdrawn before measurement after no pooled system found a relevant section, including after depth-30 extension: `oi-cn-020/021/044`, `oi-lk-039/040`, and `oi-mh-019/023/025/028/029/049/050`. Withdrawal raises absolute scores; changed split membership can change signs and dev selection. The immutable harness report lacks withdrawal metadata; this companion supplies that disclosure.

Held-out per-stage miss attribution was unavailable under an explicit owner-approved exception. The single attempt is consumed. Its test queries cannot support a later human-label confirmatory claim; fresh queries are required. No held-out rerun, relabeling or tuning occurred during this review.

Defaults remain lexical BM25, optional graph/hierarchy weights 0.3 and abstention off. The measured graph arm used 0.5; this does not promote that weight. Capacity observations have no acceptance target and deployment ingress remains untested. The corrected candidate restores HTTP overflow parity; the original measured binary retains that defect. Publication remains pending destination and exposure decisions.

Provenance: the existing aggregate summary in `.memory-bank/audit_session-2026-11-13-held-out.md`. No raw held-out queries, labels or result files were read to produce this companion.

