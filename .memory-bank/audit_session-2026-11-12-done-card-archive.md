# DONE Task-Card Evidence Archive, second batch

Date: 2026-11-12
Authority: user instruction 2026-11-12 "proceed" after review round 8; tracker
at 93% of the 131,072-byte parser ceiling.

Tracker before archival: `4479aabed495a8e35474d003e9fd5b7cae783633583b1391dc24e961d96d75f5`. Same method as
[the first batch](audit_session-2026-11-11-done-card-evidence-archive.md): for
each DONE card, the `expected_evidence` list and the `validation_summary`
scalar are moved here verbatim and replaced by one reference line. Titles,
statuses, dependencies, write sets, required validation, authorization and
manager routing are unchanged. Each SHA-256 covers the UTF-8 bytes of the
original YAML lines. Historical acceptance text, not current-tree validation.

## EVAL-2.0

expected_evidence SHA-256: `d183e7f9fe39b5d2dcbd169631ad987ffc3803ab312c6f8a45cc9d9c41421106`

```yaml
    - Each corpus entry has URL, 40-character commit, licence and local path.
    - Test split has at least 150 answerable and at least 30 off-topic queries (so at least 270 queries overall at the 1:2 dev:test split); strata assigned by the ADR definitions at writing time; each model-drafted query has a substantive GPT-6-Astra review tied to its final text, with provenance and limitations recorded before pooling.
    - Pool judgement count reported; if above 6,000 the pool is regenerated with the pre-registered grid {0.2, 0.4} and both counts are recorded.
    - Combined authoring inputs are preparation-only; config.partitions binds separate dev/test query payloads and the preparation evidence defined by EVAL-1.6. No execution command falls back to combined inputs.
    - The trusted operator supplies manager-owned eval/audit_session-query-approval.md with authentic query-review approval and its exact digest before the first pool/ranking invocation; the data worker neither writes nor approves that audit. Pools-stage preparation has no qrels/annotation evidence and never admits measurement. The manager audit records deterministic full-grid/fallback pool regeneration and manifest hash; supplied flags alone do not establish reviewer independence or chronology.
```

validation_summary SHA-256: `b5d2f12d9c27a558287f8a1358830aad2ff632034ec1ea932335a7dea5dc448c`

```yaml
        validation_summary: 'Local run exit0 (release build): ontodoc eval pool -> 259 answerable queries, 4004 pairs (<=6000, full grid), sha256 fc5c8a81; second pool byte-identical (cmp); check-benchmark --stage pools with gates.json and review pin 94aaeff0 passed with no failures. 341 queries, split seed 20261111 reproducible, test 173 answerable / 55 off-topic. --preparation-out is qrels-stage only in the CLI, so no pools manifest is published; required_validation drops that flag. No labels exist.'
```

## FIX-9

expected_evidence SHA-256: `2651549cf6f5569e131ca75989ad77760e3313cf796777b194f57f15d0ac7144`

```yaml
    - run_reserved takes Option<StagingDir>; held-out stages before reservation, dev stages after its budget checks (pre-FIX-8 order).
    - Privileged-host skip of the read-only-parent test prints SKIPPED.
    - tests/ranking_sanity.rs, ADR discrimination check: oracle beats a seeded shuffle with p < 0.01, self-comparison gives p = 1, B1/B2/B3 find planted sections above shuffle, and B0 cites the first section.
```

validation_summary SHA-256: `58517f790daa6e4387011d6c1ff8603023a9aebd64e3134636c3f4998afa80bb`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 210 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok. Astra P2 (dev staging moved before ranking) and P3 (silent privileged skip) fixed. The first run of ranking_sanity showed that B0 cannot reach planted later sections. This is the ADR-defined baseline behaviour; it is asserted, not changed. Synthetic only.'
```

## FIX-10

expected_evidence SHA-256: `66b4920b26300b8ce80af63a946bd94f6c5774849fca2a440cad92f55c4b949c`

```yaml
    - BM25 empty-document avgdl, PPR iteration-cap and fusion tie/drop/singleton tests with hand computations in comments.
    - Dev profile opt-level 3 for sha2, digest and block-buffer only.
    - README usability notes are documentation-only and recorded in the round-6 audit, outside this card.
```

validation_summary SHA-256: `ae1a67e2bd05a29bef60e9230d13cd20ab77bdb76cbfab89a67da357c47afe1a`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 213 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1) in 17.8 s wall; fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. Synthetic only.'
```

## FIX-11

expected_evidence SHA-256: `48e0934a40427094092013d25265394132c04022e5847f3ed16261933c0dcafb`

```yaml
    - record_and_prune renames each validated complete pruned generation to tmp-<id> (refusing an occupied name), syncs generations/ and the store dir, then deletes; new PruneMove phase.
    - Regression - failing on the second prune file leaves one file in tmp-<id>; the next publish removes it and prunes to three generations.
    - Fusion shuffled-input raw-BM25 and chunk-id tie tests; HTTP reads bounded by a deadline and accumulated to the header terminator.
```

validation_summary SHA-256: `ab47ca5d36cff919bd4bf423078297371b8cc20b458d7e086070f613fc3e244f`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 214 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. ADR revision 11. Two GPT-6-Astra reviews, no P1; P2 store-dir sync and P3 partial-file coverage fixed. Synthetic only.'
```

## FIX-12

expected_evidence SHA-256: `bf20e290692d6b37704c60480055122bba69e9b11179f42f675dd8a25400ef05`

```yaml
    - One pub(crate) MARKDOWN_OPTIONS const in parse.rs used by parse, tiers first_sentence and graph.
    - Regression - a table before a paragraph yields first sentence "Actual sentence.".
    - SCHEMA_VERSION 3; schema 2 and 4 artifacts refuse as unsupported; CLI status reports 3.
```

validation_summary SHA-256: `f574fa3ae4ac2666bb07e2d0e6f1932c5a0b119033e086a98f3424646ac867b6`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 214 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. All section/chunk-id fixture tests unchanged and green. No dev or held-out data exists or was read.'
```

## FIX-13

expected_evidence SHA-256: `3500e691d969a5b7cdc48d1de5ce53969fb69c9af7a8ff20fcf04f493763a889`

```yaml
    - '`x.html`/`x.htm` (any case, optional anchor) links to an existing `x.md` become LINKS_TO edges; a missing `x.md` stays ignored without diagnostics; a missing anchor on a resolved page is a MissingAnchor diagnostic.'
    - SCHEMA_VERSION 4; schema 3 and 5 artifacts refuse as unsupported; CLI status reports 4.
    - git_state on a `src/` root ignores `src/.ontodoc` but not other untracked files; code identity is unchanged.
```

validation_summary SHA-256: `a899c69ab1bfcd47a837a36e3c7914ae2dd1fa380418b9ce37390673a1941e0f`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 216 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok. Pinned rust-book/src reindexed: 112 docs, 276 LINKS_TO (was 0 from reference .html links), 17 MissingAnchor diagnostics. Synthetic tests only; no dev or held-out data.'
```

## FIX-14

expected_evidence SHA-256: `2b6c7267e81938782404459ae30f8d2a2cf786be7eb9e44532bffa1dbbda3cac`

```yaml
    - Notes without label_provenance keep the exact human-only behaviour; model-judged notes require two valid distinct kind=model parties and model adjudicator; mixed or human-claimed model notes refuse.
    - Preparation manifest serializes label_provenance; human manifests are byte-compatible.
    - A report from a model-judged preparation has label_provenance model-judged and every comparison role exploratory with holm_p and decision null; a synthetic regression proves it.
```

validation_summary SHA-256: `e40b3c7ecac8de0e8447bea9c97eedc51647457f6acdb4387faf2d28df5e37d1`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 217 passed 1 ignored; cli 8, eval_gates 22, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. End-to-end test fails when the downgrade call is removed (mutation check). GPT-6-Astra review: P2 u8 overflow fixed (widened, regression), P2 missing end-to-end test added, P2 unknown provenance now refused in Preparation::verify, P3 H1 holm fixture fixed; claimed P1 sweep confirmatory_family refuted (no such field or promotion code in src/). Synthetic only.'
```
