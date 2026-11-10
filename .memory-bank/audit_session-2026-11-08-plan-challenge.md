# Audit Session: Plan Challenge (ADR rev 2, plan 18 tasks)

Date: 2026-11-08
Inputs: `ADR_ONTODOC_SERVICE.md` revision 2 and
`ONTODOC_IMPLEMENTATION_PLAN.md` (18 tasks), both before revision 3.
Method: two independent read-only adversarial reviews plus a main-session pass.
Nothing was built or run. The cargo filter finding was checked against
`cargo test --help`, which accepts a single TESTNAME.
Outcome: ADR revision 3 and a rebuilt plan (21 tasks). Every finding below is
dispositioned.

## Findings and dispositions

| # | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| C1 | Critical | Pass rule "CI excludes 0 and p < 0.05" also passes a significant *degradation* | Fixed: pass needs mean difference > 0, CI lower bound > 0 and Holm-adjusted p < alpha |
| C2 | Critical | Sweeping chunk budget changes chunk ids (the ordinal within a section), so qrels keyed by chunk id break; the generation id also hashed query-time parameters | Fixed: the **section** is the evaluation unit (stable id = path + heading-slug path); chunk budget is frozen at 1,200 and not swept; the generation id hashes only the corpus manifest and index-time config |
| C3 | Critical | Pools built from untuned B2/B3 leave tuned results unjudged, biasing against H2/H5 | Fixed: pool depth 10 over every system at **every grid point**; gate judged@10 ≥ 0.95 per system; condensed-list nDCG' as a sensitivity check |
| C4 | Critical | No power analysis; five Holm tests on strata of 15 queries make a real gain almost undetectable, and failure was reported as a negative | Fixed: confirmatory family reduced to {H2, H5} overall; strata tests exploratory; minimum effect 0.05 nDCG@10, power 0.8; n computed from dev variance; underpowered results reported as **inconclusive**, not negative; floor of 150 answerable test queries |
| H1 | High | The abstention threshold sits on a min-max score whose top value is always ≥ 1−a; max = min divides 0 by 0 | Fixed: abstention uses the raw B1 top BM25 score divided by the query's summed IDF, the same for every mode; degenerate min-max is defined; zero-score candidates are dropped, so a = 0 / beta = 0 reproduce B1 |
| H2 | High | Ranking metrics undefined on off-topic queries (IDCG = 0) and contaminated by abstention | Fixed: H2/H5 run with abstention off, on answerable queries only; off-topic queries are not judged and are used only for H3 |
| H3 | High | B0 is a strawman, yet H1 used the first Holm slot | Fixed: H1 is a sanity gate outside the family; a failure stops the run for a pipeline audit |
| M1 | Medium | Retrieval code could change after the test data exists; only the config hash was checked | Fixed: the frozen config records git commit, `Cargo.lock` hash and binary hash; the held-out run refuses a mismatch or a dirty tree |
| M2 | Medium | No task owned the split and pool generators | Fixed: `eval split` and `eval pool` added to EVAL-1.1 |
| M3 | Medium | `cargo test --lib parse:: chunk::` is invalid (one TESTNAME only) | Fixed: `cargo test --lib -- parse:: chunk::` |
| M4 | Medium | ADR H4 ("every span in a mutated file is stale") contradicted OD-1.5 (valid/moved) | Fixed: states are valid / moved / stale / missing, defined by span bytes, not by file; H4 restated |
| M5 | Medium | PPR edges, weights, dangling nodes and tolerance unspecified; CONTAINS would make PPR measure document locality, not links | Fixed: PPR over chunk nodes using LINKS_TO and NEXT (undirected, weight 1), no CONTAINS (B3 covers hierarchy), dangling mass to seeds, L1 tolerance 1e-9, max 100 iterations |
| M6 | Medium | Recall@10 and MRR undefined on graded labels | Fixed: binary relevance = grade ≥ 1 |
| M7 | Medium | Annotation workload never estimated | Fixed: EVAL-2.0 reports the pool size; above 6,000 judgements it falls back to the pre-registered grid {0.2, 0.4} before labelling |
| M8 | Medium | "multi-hop" and "conceptual" strata had no operational definition | Fixed: defined at query-writing time and never reassigned after system runs |
| M9 | Medium | Three tasks too large (EVAL-1.1, OD-1.4, OD-1.1) | Fixed: split into EVAL-1.1/1.2, OD-1.5/1.6 and OD-1.1/1.2 |
| L1 | Low | Unweighted Cohen kappa on ordinal labels | Fixed: quadratic-weighted kappa with a bootstrap CI, plus label prevalence |
| L2 | Low | H6 compares coverage with L0 counted as coverage, so breadth-first wins by construction | Fixed: H6 dropped; budgeted assembly stays as a feature with no claim |
| L3 | Low | Two corpora but no rule for combining them | Fixed: bootstrap and randomization stratified by corpus; per-corpus results reported |

## Rejected suggestions

- Cutting `scope` and `explain`: kept. They are cheap, carry no claim, and
  `explain` drives the miss attribution.
- Cutting the `moved` citation state: kept with an exact definition (the span
  hash is found exactly once elsewhere in the same file). It resolves M4
  instead of causing it.
- Removing the EVAL-3.2 → OD-2.1 dependency: kept. It is the code-freeze
  ordering that M1 requires.
- Adding a BM25 whole-document baseline: not needed once H1 is only a
  sanity gate.
