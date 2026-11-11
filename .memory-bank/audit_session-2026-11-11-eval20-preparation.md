# Audit Session 2026-11-11: EVAL-2.0 Label-Free Preparation And FIX-13

## Owner Decisions (2026-11-11)

1. Corpora stay pinned: ontoindex `d535fe888a88…`, rust-book `2771df165f09…`.
2. Fix `local_path` in `eval/corpora.json` (pending; operator clones live in
   gitignored `tmp/corpora/`).
3. The user is the trusted operator.
4. Claude (claude-opus-5-5) drafts queries; GPT-6-Astra reviews them before
   any pooling; the user spot-checks a sample.
5. No human annotators. Option (b) approved: model-judged exploratory track
   with two judges, GPT-6-Astra and claude-opus-5-5 high (ADR revision 13).
   It never yields human qrels or confirmatory decisions.
6. With no annotators, evaluation stays parked for confirmatory claims.
7. Label-free preparation proceeds and stops before pooling.
8. Link option (i): `.html` → sibling `.md` (FIX-13, schema 4).
9. rust-book corpus root is `src/` (older editions excluded).

## FIX-13 (commit d07e17f)

- `src/graph.rs::resolve`: a relative `x.html`/`x.htm` link resolves to an
  existing sibling `x.md`; missing `.md` stays ignored (no diagnostic).
- `src/eval/gates.rs::git_state_impl`: pin exclusion `:(exclude).ontodoc` is
  relative to the corpus root, so `rust-book/src/.ontodoc` is not dirt. Code
  identity still counts it; other untracked files stay dirty.
- `SCHEMA_VERSION` 4; stores need a trusted reindex.
- Validation (this session): `CARGO_BUILD_JOBS=4 cargo test` all targets
  exit 0 (lib 216 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4,
  mcp 12, ranking_sanity 1); `cargo fmt --check` and `cargo clippy
  --all-targets -- -D warnings` exit 0. Synthetic fixtures only.
- Reindex of pinned `rust-book/src`: 112 docs, 547 sections, 1,370 chunks,
  276 LINKS_TO edges (0 before for reference `.html` links), 17 MissingAnchor
  diagnostics. ontoindex: 44 LINKS_TO, 10 diagnostics.

## Query Round 1 (No Retrieval Used)

310 model-drafted queries; four independent GPT-6-Astra reviews by direct
corpus reading. Accepted/rejected:

| Corpus | lookup | conceptual | multi-hop | off-topic |
| --- | --- | --- | --- | --- |
| ontoindex | 52/3 | 31/14 | 11/19 | 13/12 |
| rust-book | 51/4 | 34/11 | 6/24 | 19/6 |

Main rejects: multi-hop answerable from one section; conceptual answerable
from one section; off-topic partly answered; false premises. Accepted totals
(185 answerable, 32 off-topic) are below the floors (~225 / ~45 overall).

## Round 2 (Done, No Retrieval Used)

Fresh claude-opus-5-5 (high) drafters revised the 93 rejects and added new
multi-hop/off-topic queries. A first ontoindex drafter was closed by root
before writing output (operator error) and was replaced by a fresh drafter.
Root dropped five drafter-flagged duplicates/overlaps (oi-mh-031/032/047,
oi-ot-034/047) without rewording. New GPT-6-Astra reviews:

| Corpus | lookup | conceptual | multi-hop | off-topic |
| --- | --- | --- | --- | --- |
| ontoindex r2 | 3/0 | 14/0 | 12/26 | 30/2 |
| rust-book r2 | 4/0 | 8/1 | 33/11 | 20/0 |

Final accepted set: 341 queries (all model-drafted, GPT-6-Astra reviewed):
ontoindex 55/45/23/43, rust-book 55/42/39/39 (lookup/conceptual/multi-hop/
off-topic). ontoindex has few resolvable links (44 LINKS_TO), so its
multi-hop stratum is the thinnest.

## Assembled `eval/` (Label-Free)

- `queries.jsonl` 341; `query-review.jsonl` version-1 records with chained
  round history (drafter and reviewer model parties, distinct evidence),
  SHA-256 `94aaeff00a1be76fbb64d3cf86529df97aaa7327dea01e175c98c261c1e44e3e`.
- `splits.json` via `ontodoc eval split --seed 20261111`: dev 113, test 228,
  byte-reproducible (cmp). Test: 173 answerable (lookup 74, conceptual 58,
  multi-hop 41), 55 off-topic; floors 150/30/15 met.
- `dev-queries.jsonl`, `test-queries.jsonl`; `config.json` (grid
  `{0,0.1,...,0.5}`, seed 20261111, 10,000 resamples; alpha/beta/threshold are
  placeholders until the dev sweep freezes them); `gates.json` (floors, pins,
  `pools/pool.jsonl`); `corpora.json` local paths under gitignored
  `tmp/corpora/`, rust-book root `src`.
- Gate run (release build, this session): `ontodoc eval check-benchmark
  eval/config.json --stage pools --gates <gates without pool> --query-review
  eval/query-review.jsonl --query-review-sha256 94aaeff0…` reported only
  `pool_missing` (exit 1, expected): query review, floors, dev/test leakage and
  both corpus pins passed. No pool, ranking or preparation was produced.
- Operator spot-check sample: `tmp/eval-draft/spot-check-sample.md` (seeded).

## Open

- Model-judged label admission is not implemented: gates require
  `PartyKind::Human` (gates.rs annotation checks). It needs its own task before
  EVAL-2.1 labelling.
- Pooling needs the operator's `eval/audit_session-query-approval.md`
  (spot-check result, approval of review digest `94aaeff0…`) before
  `ontodoc eval pool`.
- Limitation: drafter and one judge are the same model family
  (claude-opus-5-5); queries are model-authored and model-reviewed.
