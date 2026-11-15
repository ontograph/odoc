# Sole held-out attempt — 2026-11-13

## Pre-attempt authorization

The owner approved selected preparation SHA-256 `0d4ef0952e453c5b6e3222723815d68a9b224da32c6fe6b5564a57481f8c8c61`, then explicitly approved proceeding with unavailable held-out per-stage miss attribution by replying "proceed tasks" to that scope question. All other EVAL-3.2 requirements remain. Labels are model-judged; effects and intervals are exploratory, with no Holm decisions or default promotion. The revision-15 withdrawal of 12 unanswered queries limits the benchmark. The model-judged test split cannot support a later human-label confirmatory claim; fresh queries are required.

Working directory: `/opt/d1/_wrk/ontodoc/tmp/release-010/candidate`.
External benchmark: `/opt/d1/_wrk/ontodoc/tmp/release-010/benchmark/eval`.
Shared Git-common custody: `/opt/d1/_wrk/ontodoc/.git/ontodoc-eval/`.
Candidate commit: `cc116c44a4d7f854bc19c6a4b40d75098a437268`.
Binary SHA-256: `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`.
Cargo.lock SHA-256: `2f5a28ae120bdabe661506d4766d7884fc0ba84e8922c08951cfb15c9fe6e7ac`.
Registration SHA-256: `fafd43de53e61f2b9b1b9055a3736b8148002752864fcbc837e9742c14e07572`.
Frozen export SHA-256: `fcb05142d66537d9bbc8874c1327257513d67855477926dd88cafa7d9c645d74`.

Power floor is 150 versus 165 available answerable test queries. Preflight will verify exact identities, clean candidate, absent attempt/output paths and external placement immediately before running. Disk check reports 179 GiB available; process inspection found no overlapping compiler or evaluation process. The registered binary performs its own mechanical integrity/admission checks and reserves the sole attempt before test payload access. Any consumed failure is retained and never retried.

## Commands

From the candidate working directory with `ONTODOC_BENCH` and `ONTODOC_FROZEN_BIN` set to the absolute paths above and `ONTODOC_PREPARATION_SHA256` set to the approved pin:

```sh
"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation-selected.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split test --confirm-test-split --out "$ONTODOC_BENCH/results/test-v1"
```

If completed custody is accepted, verify the saved artifacts with:

```sh
"$ONTODOC_FROZEN_BIN" eval score "$ONTODOC_BENCH/config.json" "$ONTODOC_BENCH/results/test-v1" --confirm-test-split --preparation "$ONTODOC_BENCH/preparation-selected.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256"
```

Logs and command receipts go under `results/held-out-validation/`; benchmark publication goes to `results/test-v1/`. H1 must pass before H2/H5 interpretation. Preserve registration and attempt files for every outcome. No tuning, relabeling, rerun or binary rebuild is authorized.

## Accepted outcome

The sole `eval run` and saved `eval score --confirm-test-split` both exited 0. Saved score equals the published report; completed custody binds exact meta/report bytes and every run TSV hash was verified. H1 passed before interpreting H2/H5. Command receipts and hashes are in `results/held-out-validation/`. No attempt was retried.

```json
{
  "score_exit": 0,
  "score_equals_published_report": true,
  "published_files_verified": true,
  "hashes": {
    ".git/ontodoc-eval/registration.json": "fafd43de53e61f2b9b1b9055a3736b8148002752864fcbc837e9742c14e07572",
    ".git/ontodoc-eval/attempt-started.json": "83d782be495eaa83706b30909ae7a828335d91a1b1442a3ff92e2c3983cdaf94",
    ".git/ontodoc-eval/attempt-result.json": "6339ea3a3ee45e3ad1e394604fe0c6b2e6904e16112c35c9763062ecb0898f4f",
    "tmp/release-010/benchmark/eval/results/test-v1/meta.json": "74cad341ceee406240545004847e1f2418be93f4d3da568e0199df7b932c1ef7",
    "tmp/release-010/benchmark/eval/results/test-v1/report.json": "006af6e5fcae05a9693edcc254adf7381d5622dbeae211264578f32d1a9691e0"
  }
}
```

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

H3 uses the dev-selected threshold 0.32431970510486274; it is descriptive and remains disabled by default.
- correct_abstention: 42/55 = 0.763636, Wilson 95% CI [0.636516, 0.856333].
- false_abstention: 13/165 = 0.078788, Wilson 95% CI [0.046620, 0.130123].

H4 is deterministic fixture evidence: `CARGO_BUILD_JOBS=4 cargo test --lib cite::` exited 0 (14 passed, including four citation states), and `CARGO_BUILD_JOBS=4 cargo test --test mcp` exited 0 (12 passed, including bounded cited fresh responses and tier verification). Both ran on the main checkout in this session. The retained candidate smoke also previously verified a returned citation over stdio and HTTP. These checks do not constitute exhaustive verification of every benchmark citation; the ranking TSVs contain section rankings, not citation objects.

## Limits and split disposition

The report covers 165 answerable and 55 off-topic test queries; all 220 were model-drafted. Model-judged labels share potential drafting/judging and lexical biases. No human-label confirmatory result, Holm decision or default promotion follows. No later tuning has occurred; under ADR13 this model-judged test split is already unavailable for a future human-label confirmatory claim. Any later tuning on its errors must additionally be recorded as burning it.

ADR15 withdrew `oi-cn-020`, `oi-cn-021`, `oi-cn-044`, `oi-lk-039`, `oi-lk-040`, `oi-mh-019`, `oi-mh-023`, `oi-mh-025`, `oi-mh-028`, `oi-mh-029`, `oi-mh-049`, `oi-mh-050` before measurement: no pooled system found a relevant section for these queries even after depth-30 extension. Withdrawal raises absolute scores; recomputing the split changes membership and can change signs and dev selection. The immutable harness JSON lacks withdrawal metadata; this bound audit is its required disclosure companion, not a mutation of the accepted report.

Held-out per-stage miss attribution is unavailable, accepted explicitly by the owner and documented in ADR revision 16. The frozen binary and registration were preserved. Capacity observations have no acceptance target; deployment ingress remains untested.

Recommendation: none. OD-4.1 must record the model-judged no-op; lexical mode, default weights 0.3 and absent threshold remain unchanged.
