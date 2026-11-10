# Audit Session: Donor Ideas Review (OpenViking, pharma-ltm-assistant)

Date: 2026-11-08
Scope: read-only review of two donors for ideas to adopt in OntoDoc. Nothing was
built or run. **Licences:** OpenViking is AGPL-3.0 and pharma-ltm-assistant has
no licence. Only ideas are taken; no code, prompts or data are copied.

## Donor 1: OpenViking (`../ontocode/tmp/OpenViking`, HEAD `3e0c9dc`)

- **Model.** One `viking://` virtual filesystem holds knowledge, memory and
  skills. Each directory has L0 (`.abstract.md`, about 100 tokens) and L1
  (`.overview.md`, about 2k tokens), both written bottom-up by an LLM at index
  time. L2 is the original content (`docs/en/concepts/03-context-layers.md:7-14`).
  Retrieval and memory are in Python; the Rust code is only a filesystem layer
  (`crates/ragfs`) and a CLI.
- **Retrieval today.** One global dense/sparse vector search, optionally
  reranked over 2 × limit candidates, then a score threshold
  (`openviking/retrieve/hierarchical_retriever.py:64-65`). The docs say
  explicitly: "retrieval does not navigate directories recursively" and
  "directory priority queues, parent-to-child score propagation, and
  convergence rounds are removed" (`docs/en/concepts/07-retrieval.md:82,104`).
  An earlier version used `final = 0.5·embedding + 0.5·parent` with a fixed
  alpha. It was removed without any published comparison.
- **Context assembly.** Every candidate first gets its default tier; leftover
  budget then deepens the best ones; a tier that doesn't fit falls back one
  level instead of being truncated
  (`openviking/retrieve/context_assembler/budget.py:3-9`).
- **Evaluation.** `benchmark/RAG` measures "Recall" by substring or token-overlap
  match on 12–90 queries per dataset, with an LLM judge for answers. It has no
  baselines, no confidence intervals, no significance tests and no nDCG. The
  LoCoMo headline gains are LLM-judged figures with no confidence intervals.

## Donor 2: pharma-ltm-assistant (`../ontoindex/tmp/pharma-ltm-assistant`)

- **Domain.** Pharmacy long-term memory. LLMs only at the edges (classify,
  extract); the state machine, validation, retrieval and safety checks are
  deterministic code. It has no document retrieval.
- **Evaluation discipline** (the main value):
  - Gold extractions are replayed through the whole pipeline at 100%
    (`tests/test_devcheck.py`, `tests/test_golden.py:41`).
  - A dev/held-out leakage test fails on any near-duplicate text
    (`tests/test_golden.py:73-84`).
  - Generated data is checked to be byte-reproducible (`tests/test_golden.py:125`).
  - Every error is assigned to the pipeline stage that caused it
    (`evaluation.py:488-580`).
  - Saved outputs are re-scored offline (`scripts/replay_report.py:25`).
  - The README states openly when the held-out set has been spent by tuning on
    its errors.
  - States are typed: "known absent", "unknown" and "conflict" are separate
    (`retrieval.py:60,319-327`).
  - The post-answer check is deterministic code
    (`safety.py:858-885`). OntoIndex already adopted it as answer grounding
    (`../ontoindex/.memory-bank/DONOR_LTM_ANSWER_GROUNDING_PROJECT_PLAN.md`).
- **Weaknesses.** Point estimates on 16–97 scenarios with no confidence
  intervals; fuzzy thresholds used as grounding proof; regex answer checks
  that were edited after results were seen; a golden set drafted by a model.

## Adopted (see ADR Revision 2)

| # | Idea | Donor | Lands in | Claim type |
| --- | --- | --- | --- | --- |
| A1 | Hierarchical score propagation without an LLM: blend a chunk's BM25 score with the BM25 score of its document's structural pseudo-document, beta tuned on dev | OpenViking (removed, untested there) | B3, OD-1.4 | Confirmatory H5 |
| A2 | Structural L0/L1 tiers with no LLM (heading path + first sentence; section outline) | OpenViking L0/L1 | OD-1.1, OD-1.6 | none (feature) |
| A3 | Breadth-first budgeted context assembly with tier fallback instead of truncation | OpenViking `budget.py` | OD-1.6, `search` | Exploratory H6 |
| A4 | Path-prefix scope filter on `search` | OpenViking target directories | OD-1.4, OD-2.1 | none (feature) |
| A5 | `explain` returns per-result score components | OpenViking (documented, not built) | OD-1.4, OD-2.1 | none (debug) |
| A6 | Threshold applies to the final fused score; typed abstention reasons | OpenViking rerank rules; pharma typed states | OD-1.4 | H3 |
| A7 | Qrels self-check: every labelled chunk exists and its span hash matches | pharma devcheck | EVAL-1.1 | gate |
| A8 | Dev/test leakage check on near-duplicate queries | pharma `test_golden` | EVAL-1.1 | gate |
| A9 | Byte-reproducible pools and benchmark files | pharma `test_golden` | EVAL-1.1, EVAL-2.0 | gate |
| A10 | Per-miss error attribution by pipeline stage | pharma `evaluation.py` | EVAL-1.1 | report |
| A11 | Offline re-scoring of saved run files | pharma `replay_report.py` | EVAL-1.1 (`eval score`) | tooling |
| A12 | Burned-split rule | pharma README | ADR rules | protocol |

## Rejected

- LLM-written L0/L1 summaries, LLM intent expansion and LLM rerank. They break
  determinism and the rule against model calls on the retrieval path. A frozen
  index-time LLM summary arm may be proposed later as a separate hypothesis.
- "Hotness" or recency signals in ranking: usage-dependent, so offline results
  can't be reproduced. Clock-based forgetting: freshness follows generations.
- Substring or token-overlap "recall" and LLM-judge metrics as primary
  evidence: qrels with nDCG stay primary.
- Fuzzy-match thresholds as citation proof: exact byte spans + SHA-256 stay.
- Write tools, memory/skill stores, ACL tools, a `viking://` URI scheme and
  pluggable storage backends: out of scope for a read-only docs service.
- Fixed query-type → section routing tables: domain-specific.

## Note (plan revision 3, 2026-11-08)

The task ids above refer to plan revision 2. After the renumbering in revision 3:

- A1, A4, A5 → OD-1.6.
- A2 → OD-1.2.
- A3 → OD-1.8; H6 was dropped and assembly has no quality claim.
- A6 → OD-1.5.
- A7–A10 → EVAL-1.2.
- A11 → EVAL-1.1.

See `audit_session-2026-11-08-plan-challenge.md`.
