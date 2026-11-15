# ADR: Adopting Ideas And Code From Similar Projects

Date: 2026-11-13
Revision: 5 — authorized execution and donor policy, 2026-11-13
Owner: OntoDoc maintainers
Requested by: user, "make adr md file for 1-4; should we reuse embedder; allow in agents.md to copy and adopt code"; revision 2 follows "review and challenge".

## Status

Decision 0 measured; remaining dispositions unchanged.

This ADR records one measurement (done), two deferred ideas, one existing practice, an answer to the embedder question and the donor-code rule now in `AGENTS.md`. The user authorized proceeding with these tasks on 2026-11-13. That authorizes the Decision 0 measurement and its prerequisites; deferred features remain deferred. The Decision 0 dev measurement has run (see [Execution Handoff](#execution-handoff)). The accepted [service ADR](../.memory-bank/ADR_ONTODOC_SERVICE.md), revision 16, remains unchanged.

`AGENTS.md` now implements the authorized donor permission in [Copying Rules](#copying-rules). No donor source was copied.

## Context

Eight similar projects were shallow-cloned into `tmp/similar/` and indexed with OntoIndex on 2026-11-13. No donor was built or run, so their speed and quality figures are their own claims.

OntoDoc retrieves with BM25 by default and offers optional link-graph (PPR) and heading-hierarchy fusion at weight 0.3. It uses no embedding, LLM or network on the retrieval path. Its one held-out attempt is consumed, and its H2/H5 effects are exploratory because the labels were model-judged.

Evidence read:

| Source | Files read |
| --- | --- |
| `arabold/docs-mcp-server` | `LICENSE` (MIT), `README.md`, `docs/guides/benchmarking.md` |
| `Ayush-yadav11/quillrag` | `LICENSE` (MIT), `Cargo.toml`, `README.md`, `src/search.rs` |
| `iwe-org/iwe` | `LICENSE-APACHE` (Apache-2.0), `README.md` |
| `xerj-org/xerj` | root `LICENSE` (Apache-2.0), `README.md`, `engine/crates/xerj-ai/src/embedder.rs`, `local.rs` |
| OntoDoc | `src/search/fusion.rs`, `src/search/tokenize.rs`, `src/eval/sweep.rs` (`attribute_miss`), header of `THIRD_PARTY_LICENSES.md`, `NOTICE` |

Only root licence files were read. No donor's subdirectories were checked for other licences.

## Options Considered

| Option | Disposition |
| --- | --- |
| Adopt nothing | Valid minimum; costs nothing and loses nothing measured |
| Measure dev-split misses first, then decide each idea on that evidence | **Done** (Decision 0 result below) |
| Add rank fusion and an embedder now | Rejected; no measured need and no test split to prove a gain |
| Bundle a neural model as `quillrag` does | Rejected for now; see the embedder section |

## Decision 0: Measure dev-split misses first

Every idea below claims to fix a kind of miss, and none has a measured size. OntoDoc already has the instrument: `attribute_miss` in `src/eval/sweep.rs`, reached through `eval misses`, assigns each missed relevant section one stage.

Authorized: run it on the **dev** split for the lexical default and record the stage counts in an audit file.

- `lexical_miss` is an upper bound on vocabulary mismatch. It also counts sections that share a query term but rank below the fetch depth of 50.
- Stage labels describe the existing pipeline; they do not establish which intervention would fix a miss. In particular, `lexical_miss` does not prove a vocabulary problem, and an alternate ranking could change downstream displacement or cutoff outcomes.
- Never run this on the test split. Held-out miss attribution is unavailable by the service ADR revision 16 exception.

Gate: B1 stage counts identify where further diagnosis may be useful. They cannot alone justify an embedder or reopen Decision 2; fusion diagnosis requires separately scoped B2/B3 evidence.

## Decision 1: Deterministic metrics decide, judged rubrics only report

Source idea: `arabold/docs-mcp-server` lets deterministic metrics (MRR, Recall@k, nDCG@k, Hit@k) gate regressions and reports LLM-judged rubrics without ever failing a run.

OntoDoc already works this way: its decisions use deterministic section-level metrics, and model-judged labels are exploratory under service ADR revision 13. This decision records the practice and adds no work.

The donor's 59-query dataset is not adopted. It targets other sites, and OntoDoc's qrels are keyed by section id in its own pinned corpora.

Disposition: no change.

## Decision 2: Reciprocal rank fusion

Source idea: `quillrag` fuses two rankings with `score(d) = Σ 1/(60 + rank(d))` in `rrf_fuse`.

Disposition: **deferred**, not proposed for implementation. Reasons:

- **Unproven benefit.** OntoDoc already normalises both score lists with `min_max`. That handles scale differences but does not establish calibration or robustness to score distributions. Rank fusion remains an alternative to measure only if a concrete blend problem is found.
- **Compatibility needs design.** The donor's unweighted rank fusion is not a drop-in replacement for `fuse`, which reproduces lexical order exactly at weight 0. Weighted rank fusion or an explicit zero-weight bypass could preserve this guarantee, but neither is needed without evidence for the change.
- **Tie handling is undefined.** The donor gives no rule for ranking tied scores. OntoDoc would need a fixed one, such as BM25 descending then chunk id, as `fuse` uses today.
- **Real cost.** A new mode touches `src/search/`, the dev sweep and the frozen configuration. It is more than the fusion function.
- **No hypothesis.** A new arm needs a named baseline, a minimum effect and a fresh pre-registered test split. It cannot join the consumed H2/H5 family.

Reopen only with dev evidence that the weighted blend is the problem, for example many `fusion_displaced` misses for the graph or hierarchy modes. If reopened, rewrite it: the donor function sorts a `HashMap` by score alone, which leaves tied results in arbitrary order.

Decision rule for the dev fusion check (DA-3 in `docs/DONOR_ADOPTION_PLAN.md`), fixed on 2026-11-13 before the run. Run dev at the selected weights, B2 alpha 0.5 and B3 beta 0.3, and count relevant pairs from B1's top 10 that each mode drops. A pair is displaced when it is absent from that mode's miss records but present in B1's, so a section that B1 also missed is never counted. Decision 2 reopens if either mode displaces 21 or more pairs, which is 5% of the 414 relevant dev pairs. Otherwise it stays deferred. Pairs a mode recovers from B1's misses and the tool's own `fusion_displaced` label are reported alongside but do not change the decision.

The rule sentence above is wrong: "absent from that mode's miss records but present in B1's" describes recovered pairs. The intended and applied definition, consistent with "pairs from B1's top 10 that each mode drops", is present in the mode's misses and absent from B1's. Recovered pairs are the reverse.

Result (DA-3, 2026-11-13, dev split only): **the rule fired; Decision 2 reopens for diagnosis.**

| System | Misses of 414 | Displaced vs B1 (queries) | Recovered vs B1 | Stages |
| --- | --- | --- | --- | --- |
| B1 | 99 | — | — | 78 truncated, 21 lexical_miss |
| B2 alpha 0.5 | 88 | 15 (11) | 26 | 15 fusion_displaced, 61 truncated, 12 lexical_miss |
| B3 beta 0.3 | 69 | **32 (24)** | 62 | 32 fusion_displaced, 19 truncated, 18 lexical_miss |

B3 crosses the 21-pair threshold and B2 does not. The tool's `fusion_displaced` counts equal the displaced counts. Both modes are net positive against B1: B2 by 11 pairs and B3 by 30. Fusion therefore helps on average and still drops a real set of sections B1 ranked well. That matches the retained dev sweep, where both arms beat B1 on nDCG@10.

What reopening means: the next step is to diagnose the 32 B3 displacements, for example whether the doc-score blend promotes sibling sections of one document over the relevant one. It does not authorize rank fusion. The donor's unweighted reciprocal rank fusion would not reduce these displacements by itself, and every compatibility, tie-break and preregistration point above still applies. No ranking change, default change or held-out access follows.

Run identity: clean candidate `cc116c44…` (`git_dirty` false), binary `af5fd69e…`, config `6cad185c…`, owner-approved selected preparation pin `0d4ef095…`. Both `eval run --split dev` and `eval misses` exited 0 with empty stderr. Outputs are in `tmp/release-010/benchmark/eval-donor-fusion/`: `run-candidate/` (meta `1b2ffb6e…`), `misses.json` (`0a07e0c4…`) and `summary.json` (`cd286c50…`). Its B1 ranking is byte-identical to the earlier `dev-proof` B1 (`4b45fdf5…`). An earlier attempt from the main checkout is kept as `run/`, but recorded `git_dirty` true, so it was not used. Its rankings equal `run-candidate/` and only metadata differs.

## Decision 3: Link-graph comparison with `iwe`

OI-2's calibrated production-resolver census (2026-11-14) supersedes the approximate ignored-link counts below. Distinct linked pairs equal the stored `LINKS_TO` counts (276 / 44), and problems equal the stored diagnostics (17 / 10). The synthetic 20-event regression checks the classifier; outputs, commands and hashes are under `tmp/ontoindex-ideas/link-outcomes/`.

| Link-event class | Rust Book | OntoIndex |
| --- | ---: | ---: |
| Linked | 276 | 44 |
| Missing anchor | 17 | 0 |
| Missing target | 0 | 10 |
| External | 103 | 175 |
| Ignored HTML | 33 | 0 |
| Directory | 0 | 52 |
| No extension | 0 | 10 |
| Other extension | 0 | 3 |
| Total | 429 | 294 |

`outside_chunk`, `same_chunk`, `outside_root`, `ignored_markdown` and `not_a_path` were zero for both corpora. OntoIndex has 18 distinct directory destinations, two without extensions and two with other extensions; Rust Book has none in these classes. Counts do not establish how many additional unique edges a candidate rule would produce. Ignored-link classification repeats resolver string tests and must track future resolver changes. Benchmark corpora may differ from served folders. Link-resolution changes remain deferred; no schema change, reindex or retrieval-quality claim follows. See [execution evidence](../.memory-bank/audit_session-2026-11-14-ontoindex-ideas.md).

Descriptive census (DC-3, 2026-11-14): both existing stores are fresh schema 4. Rust Book's two over-cap chunks lose 10 LINKS_TO and 45 MENTIONS edges after position 100; no CONTAINS or NEXT edges are cut. OntoIndex has no chunk above 100 incident edges. Largest chunks are 10,269 bytes and 33,444 bytes respectively; neither corpus has a chunk above 64 KiB. Only OntoIndex has a chunk above 32 KiB (one). These benchmark corpora do not establish typical served-corpus behavior.

The approximate inline-link scanner failed calibration: Rust Book reproduced 0 of 17 MissingAnchor diagnostics; OntoIndex reproduced 9 of 10 MissingTarget diagnostics. Step 4 is explicitly uncalibrated and additional unique-edge counts are unavailable. No link-resolution change is justified. The selected approved preparation-generation match remains unconfirmed because the available preparation pin differs from the selected pin. No store was reindexed and no query, label, pool or result payload was read.

Census script, output and SHA-256 manifest are local ignored artifacts under `tmp/donor-candidates/census/`. The [review audit](../.memory-bank/audit_session-2026-11-14-donor-review.md) retains the exact result and verified hashes; the local script and corpus stores are still needed to reproduce the measurement. `python3 tmp/donor-candidates/census/census.py` exited 0 in the original execution; the explicit calibration failure is a permitted limited outcome, not evidence of edge gains.

Only the `iwe` README was read. It describes notes that link to each other and can belong to several topics; it does not show how links are resolved, and the multi-topic feature may concern hierarchy, not links.

Disposition: **deferred**. No gap in `src/graph.rs` is established. A read-only review of `iwe`'s link resolution is optional and has no claim attached. Any later change to edge construction needs a schema bump, and graph edges must never feed qrels.

Review result (DA-4, 2026-11-13, read-only, `iwe` commit `b733afe95ca6dd96ac7205e696bb920975253f18`; cited files under `crates/liwe/`, Apache-2.0 by the root and crate `LICENSE-APACHE`, no file headers; nothing copied). No gap in `src/graph.rs` was found. Both sides resolve relative and `../` links, ignore external URLs, and keep code text out of link edges. Each remaining difference favours OntoDoc or is a deliberate choice:

- Fragment-only `#heading` links: OntoDoc resolves them within the document (`src/graph.rs:182-185`, `214-227`); `iwe` drops them. The corpora contain 9.
- `.html` targets: OntoDoc maps them to an existing sibling `.md` (`src/graph.rs:197-211`); `iwe` ignores them. The corpora contain 16; how many resolve was not checked.
- Wiki `[[links]]`: only `iwe` parses them, and the corpora contain none.
- Missing targets yield diagnostics and no edge in OntoDoc; duplicate edges are deduplicated (`src/graph.rs:73-75`, `111-116`).

Disposition stays **deferred** with no follow-up. Counts are literal `rg` occurrences over `*.md` in the two pinned corpora, not resolved edges.

## Embedder: should OntoDoc reuse one?

Recommendation: **no, not now.** This section replaces revision 1's Decision 4, an opt-in embedder rule for a feature this ADR advises against building.

| Candidate | Cost | What it adds |
| --- | --- | --- |
| Stemming or word-variant matching in `src/search/tokenize.rs` | Tokenizer version bump and reindex; no dependency | Matches word variants. The tokenizer has no stemming today |
| `xerj` feature hashing (`local.rs`) | Small dependency-free function; schema bump to store vectors | Sub-word overlap through character trigrams. The donor's source calls it lexical, not semantic |
| `quillrag` MiniLM through `candle` | Four new dependencies; a binary of about 105 MB by the donor's figure; schema bump | Paraphrase matching |
| No change (current) | Nothing | Nothing |

Reasons:

1. **The need is unmeasured.** Decision 0 comes first.
2. **The cheapest fix is not an embedder.** If misses come from word variants, a tokenizer change addresses them inside the existing owner and keeps BM25 explainable. Feature hashing mostly re-measures the same word overlap.
3. **The neural donor has no quality evidence.** `quillrag`'s README says relevance is "not yet benchmarked", it is verified at 1,000 chunks, and its dense search is a single-threaded linear scan.
4. **It breaks standing rules.** A neural model conflicts with the no-embedding rule and the no-new-dependency rule. Whether `candle` output is bit-identical across CPUs is unverified, so artifact determinism is at risk.
5. **No test split.** Any vector arm needs dev evidence and a fresh pre-registered test split.

Conditions if this is ever reopened:

- Off by default, behind a cargo feature, with no download at query time and a service-ADR amendment first.
- The embedder's identity (algorithm, version, dimensions) is part of the generation identity. `xerj`'s source warns that changing the hash without changing the identity silently mixes vector spaces.
- The licence of any bundled model weights is checked separately. `quillrag`'s MiniLM weights were not checked.

## Copying Rules

The user asked for `AGENTS.md` to permit copying donor code. None of the decisions above needs it: every idea here is smaller as a rewrite. The wording below is offered because it was requested.

Applied wording for the `## Donor Code` section of `AGENTS.md`:

```markdown
## Donor Code

`../ontoindex`, `../ontocode/tmp/OpenViking` (AGPL-3.0),
`../ontoindex/tmp/pharma-ltm-assistant` (no licence) and
`../ontocode/tmp/repo/mcp` (source licence/provenance unresolved) are idea sources only.
Never copy their code, prompts or data into this repository.

Code may be copied and adapted from a donor file that is MIT or Apache-2.0
licensed (owner decision 2026-11-13). The clones under `tmp/similar/` are
candidates. Before copying, check the licence that applies to that file: the
root licence file, any licence file in its directory, and any header in the
file. A README badge is not evidence. For every copy:

- Keep the donor's copyright and licence notice. For Apache-2.0, also carry
  applicable attribution from `NOTICE` when supplied and state in changed files that they were changed.
- Record donor, commit, files and licence in `COPIED_CODE.md` at the
  repository root, and mark the origin in a comment beside the code. Do not
  use `THIRD_PARTY_LICENSES.md`; it is a generated dependency inventory.
- Review the package licence statement and `NOTICE` when the first copy
  lands, because the package then carries third-party source.
- Copy code only. Never copy donor prompts, benchmark queries, labels, model
  weights or other data without a separate licence check and owner approval.
- Copied code still meets every rule in this file: existing owner module, no
  new dependency unless the task names it, determinism, bounds, no
  `unwrap`/`expect`, and a focused regression test.
- Copying does not approve a feature. An ADR and a plan task still decide what
  is built; see `docs/ADR_DONOR_ADOPTION.md`.

Donors with AGPL, no licence or unresolved provenance stay idea sources only.
Adopted ideas are listed in the donor-review audits linked from
`.memory-bank/MEMORY.md`. Donor inspection is not evidence of retrieval
improvement; new ranking arms need their own approved dev evaluation and
preregistration, not a changed confirmatory family.
```

`COPIED_CODE.md` does not exist yet; create it with the first copy.

## Consequences

- No runtime, schema, default or evaluation change follows from this ADR.
- The only authorized measurement was Decision 0, a dev-split measurement; its result is in [Execution Handoff](#execution-handoff).
- An earlier OntoIndex query on the `iwe` clone reportedly failed with `ENOSPC`. Its cause and current disk state are unverified. Diagnose capacity before an operation that needs it; do not delete donor checkouts as an assumed remedy. Preserve any commit and licence evidence needed for provenance.
- This file sits under `docs/` because `.memory-bank/` was not writable in the authoring session. Move it beside the other ADRs and link it from `.memory-bank/MEMORY.md` if wanted.

## Execution Handoff

Result (dev split only, exit 0): `ontodoc eval misses config.json dev-run --preparation preparation.json --preparation-sha256 1e4835cd994a799b136fcf7c095567e43ae9bbaafd800a6bdbc51d69b1cf5125`, run from `tmp/release-010/benchmark/eval-donor-dev/` with the retained approved `eval/config.json` (SHA-256 `b65c79a5…`, equal to the preparation and saved run), the clean candidate binary (SHA-256 `af5fd69e…`) and the saved `dev-proof` run. Outputs `tmp/release-010/benchmark/donor-misses.json` (SHA-256 `7b495cae…`) and `donor-b1-summary.json` (`190ab4a1…`). The later frozen export (`6cad185c…`, alpha 0.5, beta 0.3) was not used.

Of 109 dev queries and 414 relevant (grade >= 1) pairs, B1 missed 99 from its top 10: 78 `truncated` (40 queries) and 21 `lexical_miss` (18 queries); `not_indexed`, `chunk_boundary` and `fusion_displaced` are 0. The saved run has alpha and beta 0, so B2 and B3 equal B1 and the run cannot show fusion displacement. The lexical-miss share (5.1%) is an upper bound on vocabulary mismatch. It is not evidence for an embedder and does not reopen Decision 2. If pursued, a tokenizer word-variant change is the cheapest candidate and needs its own dev evaluation and preregistration. No test split, pool or full preparation was opened, and no custody record was written.

Follow-up diagnostic (read-only, dev queries and the pinned corpus checkouts, no code or config change): the 21 `lexical_miss` pairs span 18 queries (two sections could not be located by heading slug). In all 19 located sections, the section text shares at least one exact term with its query, so none is a zero-overlap vocabulary mismatch. For these 19 sections, a crude suffix-stripping stemmer (`ing`, `ies`, `ed`, `es`, `s`, `ly`) finds no section whose only overlap is a word variant. Stemming therefore would not have recovered any of them; the misses are sections whose shared terms are common words or that rank below the BM25 fetch depth of 50. 10 of the 19 share only terms that occur in at least 20% of corpus files. The tokenizer word-variant candidate is therefore not supported by this data and is dropped unless new dev evidence appears. Limits: heading-slug section extraction is approximate and includes child sections, the stemmer is crude, and this does not measure ranking effects of stemming on the other 315 found pairs.

Command shape:

```text
ontodoc eval misses <config> <dev-run-dir> --preparation <manifest> --preparation-sha256 <operator-pin>
```

`src/cli.rs:170` declares the arguments; `src/cli.rs:1397` loads the pinned dev capture; `src/cli.rs:1434` scores the saved run and emits individual miss records for B0/B1/B2/B3. Aggregate the B1 records for Decision 0; the command does not directly return stage totals or restrict its output to B1. `src/eval/run.rs:1116` and `src/eval/run.rs:1148` select dev paths without a combined/test fallback. `src/eval/sweep.rs:245` restricts `fusion_displaced` to B2/B3 when B1 retrieved the section in its top ten. The paths, pin and saved-run compatibility above were verified by the run.

## Revision History

- Revision 5: applied the authorized donor policy in `AGENTS.md` and recorded the Decision 0 dev measurement (B1: 78 truncated, 21 lexical_miss of 414 pairs).

- Revision 4: challenged categorical embedding and fusion claims, clarified that donor permission is already requested, removed speculative cleanup, and consolidated the next steps in [the review report](../.ontocode/research-reports/donor-adoption-review.md). No measurement or runtime change was made.

- Revision 3: recorded authorization to proceed with Decision 0, current tracker validation and execution prerequisites; retained deferred feature decisions and distinguished lexical attribution from fusion diagnosis. Measurement remains unexecuted.

- Revision 2: added Decision 0 using the existing `attribute_miss` tool; deferred rank fusion and the link-graph review; reduced the benchmark split to a record of existing practice; replaced Decision 4 with conditions in the embedder section and added a tokenizer alternative; moved copy records from `THIRD_PARTY_LICENSES.md` to `COPIED_CODE.md`; added Apache-2.0 change notices and per-file licence checks; added an options table; corrected the count of licence files read.
- Revision 1: initial proposal of four adoptions.
