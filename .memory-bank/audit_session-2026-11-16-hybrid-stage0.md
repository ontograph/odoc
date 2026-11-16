# Audit: Hybrid Retrieval Stage 0 And Fusion Primitives

Date: 2026-11-16
Request: user, "implement .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md".
Plan: [HYBRID_RETRIEVAL_PLAN.md](../docs/HYBRID_RETRIEVAL_PLAN.md). ADR: [hybrid retrieval](ADR_ONTODOC_HYBRID_RETRIEVAL.md) revision 2, still Proposed.

## Scope

The ADR stages work behind owner decisions. Two parts needed none: stage 0, which the ADR defines over existing dev evidence, and the pure cosine and fusion functions in the `src/search/` owner row. Stages 1 to 4 need a selected model, tokenizer and embedding tool with licences, a label track, fresh reviewed queries with a cross-language stratum, caps, a custody transition and a service-ADR amendment. They stay blocked in the plan.

## Stage 0 (HY-0)

Inputs are the clean-candidate dev run recorded in `docs/ADR_DONOR_ADOPTION.md` and its `eval misses` output; no new run was made. `python3 tmp/hybrid-stage0/stage0.py` exited 0 and wrote `tmp/hybrid-stage0/summary.json` (sha256 `ea844cf2…`; script `f1998274…`). It read only `eval/dev-queries.jsonl` (`c0911db7…`), `eval/dev-qrels.jsonl` (`036c47cc…`), `run-candidate/meta.json` (`1b2ffb6e…`), `run-candidate/B1.tsv` (`4b45fdf5…`) and `misses.json` (`0a07e0c4…`) under `tmp/release-010/benchmark/eval-donor-fusion/`.

| Measure (B1, 82 answerable dev queries) | Value |
| --- | --- |
| Miss pairs by stage | 78 `truncated`, 21 `lexical_miss`, 0 `chunk_boundary` |
| Queries with any relevant section outside the top 10 | 48/82 = 0.585, Wilson 95% [0.477, 0.686] |
| Queries with a `lexical_miss` section | 18/82 = 0.220, Wilson 95% [0.144, 0.321] |
| `lexical_miss` queries by stratum | multi-hop 8, conceptual 6, lookup 4 |
| `lexical_miss` queries by corpus | rust-book 14, ontoindex 4 |
| B1 mean nDCG@10 | 0.7247 |
| Oracle gain, reorder the current top 10 only | +0.1289 |
| Oracle gain, insert every `lexical_miss` section at its ideal rank | +0.1630 |
| Oracle gain, recover every miss | +0.2753 |
| Dev queries with a non-ASCII character | 0 |

The ADR's need bound is `p ≥ 0.05`. The lexical-miss share alone has a lower Wilson bound of 0.144, so stage 0 does not stop the work on the miss-share criterion. The oracle gains are ceilings: they place every recovered section at its ideal rank, which no real leg does.

The ADR's second stage 0 question is unanswered. The dev queries contain no non-ASCII text and no cross-language stratum, so the existing benchmark says nothing about cross-language need. That needs owner decision 2.

Limits: the qrels were pooled from B0–B3, so `p` is a lower bound; the labels are model-judged, so this is exploratory evidence; `lexical_miss` includes sections that share a term but rank below 50, so it is no pure vocabulary-mismatch count. No need threshold was registered before this measurement, and none is set here.

## Fusion Primitives (HY-1)

`src/search/dense.rs` adds `Vectors` (validated, explicitly normalized rows; exact cosine top-k with scope before selection and chunk-id ties) and `rrf` (one-based ranks, first occurrence per list, cut to depth). It is declared in `src/search/mod.rs` and has no caller: no `Mode`, CLI, MCP, eval arm or store change. OntoIndex rated `rank_sections_to` HIGH upstream (12 dependents, 4 processes), so it was not edited.

Commands, all from `/opt/d1/_wrk/ontodoc`:

- `CARGO_BUILD_JOBS=4 cargo test --lib search::dense::` exit 0, 5 passed.
- `CARGO_BUILD_JOBS=4 cargo test --lib search::` exit 0, 29 passed.
- `cargo fmt --check` exit 1 on first run (line wrapping in the new tests), then exit 0 after `rustfmt --edition 2024 src/search/dense.rs`.
- `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings` exit 0.
- Lane `ontodoc-hy1-clippy-20261116`: launcher exit 0; `lane-result` exit 0, nested `exit_code` 0, state terminal.
- `project_plan_validate docs/HYBRID_RETRIEVAL_PLAN.md` passed after three repairs (task complexity, quoting `::` in YAML, Clippy through the lane).

The full test suite was not run; the change adds one unreferenced module. The tests check arithmetic written out in comments and establish no retrieval-quality claim.

## Not Done

No model was downloaded or run, no vector produced, no query written, no test-split file opened and no custody, config, query, qrels, default or schema touched.
