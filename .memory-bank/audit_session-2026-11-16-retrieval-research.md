# Audit: Retrieval Research Step 1 And Routing

Date: 2026-11-16
Request: user, "propose several solutions to do next tasks and fix issues, cherry pick most robust and performance proposals, consolidate it to one plan and proceed it", with OntoIndex for code navigation and GPT sub-agents.
Plan: [RETRIEVAL_RESEARCH_PLAN.md](../docs/RETRIEVAL_RESEARCH_PLAN.md) revision 3. ADR: [retrieval research](ADR_ONTODOC_RETRIEVAL_RESEARCH.md) revision 4, steps 1 and 2 accepted for dev-only exploratory execution; the rest stays Proposed.

## Proposals And Selection

Three read-only GPT sub-agents reviewed the plan: GPT-6-Astra for step 1 design, GPT-6.1-Sol for the step 2 arm, and GPT-6-Astra for a challenge of the plan's gates. They used OntoIndex context and impact on `attribute_miss`, `misses`, `SearchIndex::search`, `rank_sections_to`, `System`, `Params` and `pool`, then checked current source, because OntoIndex reported `src/search/mod.rs` and `src/search/dense.rs` as dirty and absent from the graph. No sub-agent edited a file or opened a held-out file.

| Issue raised | Choice | Rejected alternative |
| --- | --- | --- |
| Step 1 needs ranks to 50 and token overlap that `misses.json` and `B1.tsv` lack | Python over the pinned binary and stored generation, behind parity gates | A new Rust subcommand (crate change for a one-off diagnosis); a Python BM25 with no parity check |
| Step 2 arm placement | Research script that permutes B1's captured top 50 | A `System` variant (OntoIndex rates `rank_sections_to` HIGH, 8 nodes; the held-out lists enumerate four systems); a dev CLI subcommand |
| New judgments for new arms | Diagnostic on existing qrels with judged@10 and condensed nDCG | Reusing `config.pool_extension`, which service ADR revision 14 limits to unanswered queries chosen before rankings were seen |
| Stemming experiment touches the global tokenizer version | Registered endings inside the parity-checked Python BM25 | A `TOKENIZER_VERSION` bump with reindex before any evidence |
| Sequential steps deadlock when a step is skipped | Each step resolves as measured or not opened | A plain dependency chain |
| Pair shares treat pairs from one query as independent | Query-cluster bootstrap beside Wilson | Wilson alone |
| Truncated pairs listed a "below 50" band | Bands 11–20 and 21–50 only | — |

The rules, gates and thresholds were registered in ADR revision 4 before `scripts/research/` existed.

## Step 1 (RR-1)

Scripts: `scripts/research/rr_common.py` (sha256 `f25d6bef…`), `scripts/research/rr_step1.py` (`4f395ba0…`) and `scripts/research/charprops.rs` (`cb3e29b9…`). The last prints Rust standard-library character properties for the pinned toolchain, rustc 1.95.0, because Python's `str` methods disagree with Rust on ten code points in the corpora, such as U+4E00 (numeric in Python, not in Rust) and U+0902 (alphanumeric in Rust, not in Python).

Commands, run from the repository root:

| Command | Exit |
| --- | --- |
| `python3 scripts/research/rr_step1.py --self-test` | 0 |
| `python3 scripts/research/rr_step1.py --out tmp/retrieval-research/step1.json`, twice | 0; outputs byte-identical, sha256 `33a67b6b…` |

The first real run stopped at the BM25 parity gate on one query: the Python port used Python character properties. After the switch to Rust's, the run passed.

Inputs were HY-0's five files with unchanged hashes, `target/release/ontodoc` (0.1.1, sha256 `400e6fb5…`), and the stores at generation `4e546b16…` (ontoindex) and `375917fc…` (rust-book). Every gate passed: input hashes, `CURRENT` before and after, binary top 50 against the Python BM25 for all 82 answerable queries, projection against `B1.tsv`, recomputed misses against `misses.json`, and the HY-0 ceiling.

| Measure (B1, 82 answerable dev queries) | Value |
| --- | --- |
| `lexical_miss` pairs | 21: 20 shared term ranked below 50, 0 surface-form variant, 1 no shared content term (its only overlap is a stopword) |
| No shared content term | 1/21 = 0.048, cluster 95% [0.000, 0.167] |
| Surface-form variant | 0/21 = 0.000, cluster 95% [0.000, 0.000], Wilson [0.000, 0.155] |
| `truncated` pairs | 78: 12 in queries with more than ten relevant sections, 66 remainder |
| Remainder rank bands | 32 at section rank 11–20, 34 at 21–50 |
| `H` favours the missed section | 22/66 = 0.333, cluster 95% [0.206, 0.471] |
| `C` favours | 9/66 = 0.136, cluster 95% [0.049, 0.243] |
| `N` favours | 13/66 = 0.197, cluster 95% [0.105, 0.313] |
| Oracle gain, truncated sections only (includes reordering the top ten) | +0.2441 |
| Oracle gain, every miss (HY-0 parity) | +0.2753 |

Route: no registered threshold is met. Step 2 needs a descriptor share of at least 0.50 (best `H`, 0.333), step 3 needs at least 0.25 surface-form share (0.000), and the step 4 signal needs at least 0.50 no-shared-term share (0.048). Per the ADR, the current system stays and steps 2 to 4 do not start.

## Steps 2 And 3 (RR-2, RR-3)

`python3 scripts/research/rr_resolve.py --self-test` exited 0. `--step 2` and `--step 3` each exited 0 and recorded the step as not opened, bound to step 1's sha256: `tmp/retrieval-research/step2.json` (`4880af2e…`) and `step3.json` (`3364a805…`). No arm ran, and no B1R or folding code exists.

## Reading

Nearly every vocabulary miss shares a content term with the query. The miss happens because the section ranks below 50, not because the words differ, so neither stemming nor dense retrieval has dev evidence of a target. The depth misses carry most of the headroom (+0.2441 of +0.2753), but none of the three registered descriptors separates them from the top ten in a majority of pairs, so none of the registered reorderings has a stated reason to work. A different depth hypothesis needs a new registration. The results are exploratory under model-judged labels, the dev split has been reused, and the qrels were pooled from B0 to B3.

## Limits

No Rust source, store, tokenizer, evaluation record, query, label or custody file changed, and no held-out file was opened. `scripts/research/rr_common.py` refuses held-out paths. Outputs under `tmp/` are ignored by Git; the hashes above identify them, and the tracked scripts regenerate them from the pinned inputs. No Rust build, test or Clippy run was needed or claimed.
