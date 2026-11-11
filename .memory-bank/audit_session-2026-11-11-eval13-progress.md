# Audit Session 2026-11-11: EVAL-1.3 Progress (Slice 1 Of Several)

Task: EVAL-1.3 (OPEN, not complete). Tracker revision at start:
`6b6e432ffdcb704d52cfc27f7801820d1fc3f71304d298bd6a81c49bb85d4969`; ADR 9
`797c27aa…07e8f`. Universal refusals (`ensure_measurement_ready`) are kept.

## Slice 1 Done: Query Review Before Ranking (ADR9 "Trusted Approval")

- `src/eval/gates.rs`: version-1 `query-review.jsonl` parser
  (`deny_unknown_fields`), `check_query_review(queries, bytes, pin)` →
  `QueryApproval`. Lowercase 64-hex out-of-band pin; exact `Query.text`
  digest, corpus/stratum/model_drafted match; final decision accept;
  distinct author/reviewer id and evidence; GPT-6-Astra reviewer for model
  drafts (model never human); linked predecessor history ending at the
  accepted final text; first author fixes drafting origin; 128-B ids,
  4-KiB rationale/evidence, 16-MiB bytes, 100k entries; missing, duplicate
  and unknown records refuse.
- `check_pool`/`check_benchmark` take `Option<&QueryApproval>`; absence is a
  `query_review_missing`/`query_review_mismatch` gate failure checked before
  indexes load. `run::pool`/`pool_captured` require coverage before ranking.
  `Bench::prepare` takes review path + pin. `preflight` passes `None`
  (fails closed; execution is still refused earlier).
- CLI: `eval pool --query-review --query-review-sha256` (required);
  `check-benchmark` takes both optionally (missing → gate failure).
- Tests: synthetic GPT-6-Astra review fixture; 15 tamper/refusal cases plus
  reject→human revise→Astra accept success; pools/qrels stage without review.

Review: GPT-6-Astra explorer (read-only) found four P2s: allocation/byte
bounds, an over-strict revise-only history rule, a per-revision origin rule
and an unbounded query_id/corpus. It also flagged two weak tests. All were
fixed before the final run. Not fixed: review bytes are not yet charged to
the shared capture budget, which slice 3 (preparation manifest) owns.

## Validation (this session, compiler lane)

- Job `eval13-s1-test4`: `cargo test --test eval_gates --test eval_run`
  passed 16 + 6; `cargo test --lib eval::` passed 35; `cargo fmt --check`
  passed; `cargo clippy --all-targets -- -D warnings` exited 101, solely on
  `clippy::unused_unit` at `src/store.rs:745`. That file is outside the
  EVAL-1.3 write set and unchanged since the baseline.
- Job `eval13-s1-clippydiag`: Clippy with only `-A clippy::unused_unit`
  exited 0, so the changed files carry no other lint failures.

## Remaining Slices

## Slice 2 Done: Human Annotation Companion (ADR9)

- `src/eval/gates.rs`: version-1 `annotation.jsonl` records and one fenced
  `annotation-notes` block in `annotation.md` (`deny_unknown_fields`);
  exactly two independent human annotators; model drafts need a named
  human's `draft_review`; human adjudicator for `final`; final grade and
  span hash must equal the qrel; every qrel pair annotated exactly once.
  Overlap is `corpus-stratum-ceil30-v1` (SplitMix64, `assignment_seed`).
  Quadratic-weighted kappa uses raw pre-adjudication grades. The CI is a
  10k query-cluster bootstrap seeded by `config.seed`, with a 0.6 floor.
  `GateReport.agreement` reports the result. Off-topic qrels are refused
  (`off_topic_judged`). Any undefined bootstrap resample refuses
  (`agreement_ci_undefined`): no conditional CI is reported.
- `run.rs`: `TrustedInputs`, `Bench::prepare(path, &TrustedInputs)`.
  `preflight` passes empty provenance (fails closed). CLI
  `check-benchmark --annotation --annotation-notes` (paired).
- Tests: worked-example kappa with a seeded cluster bootstrap (zero and about
  25% undefined cases), stratified ceil-30% overlap and the single-fence rule.
  Integration: 72-query overlap with kappa 1.0, `annotation_missing`, 9
  tamper cases, a reviewed-draft success, a no-overlap refusal and off-topic
  labels.

GPT-6-Astra review (read-only) found three issues. Fixed: off-topic labels
were admitted (medium), the undefined-resample policy was undeclared and
the unit-test comment was wrong. Open items, recorded and not silently
changed:
(a) annotation/qrel `section_id` allows 4096 bytes but ADR9 bounds ids to 128
    bytes; path-derived section ids may need an explicit ADR exception;
(b) review/annotation bytes and entries are not charged to the shared capture
    budget and are parsed before counting (slice 3);
(c) `assignment_seed`/notes are not yet bound by an audited hash, so a
    different seed could select another overlap subset (slice 3 must bind
    annotation + notes hashes in `preparation.json`).

Validation: job `eval13-s2-e`: eval_gates passed 19, eval_run passed 6,
`--lib eval::` passed 38, and `cargo fmt --check` passed. Clippy with
`-A clippy::unused_unit` passed. Plain Clippy exited 101, solely on
`src/store.rs:745` `unused_unit`.

## Slice 3 Done: Preparation Manifest Emit/Verify (ADR9 "Exact-Byte Handoff")

- `run.rs`: `Preparation` v1 (qrels stage only, `deny_unknown_fields`),
  binding operator/evidence; raw SHA-256 of config, gates, splits, combined
  queries/qrels, full pool, query-review, annotation and notes (so
  `assignment_seed` is now hash-bound); sorted corpora with pinned commit,
  generation id, index and meta hashes; dev/test partition hashes plus a
  domain-tagged ordered commitment distinct from selected digests; counts
  derived from captured records (split/corpus/stratum, positive answerable,
  pool pairs, reviewed/model-drafted, agreement); policy
  `h5-then-h2-v1`, seed, grids and gate-report hash. 1-MiB cap.
  `publish` is create-new + fsync. `verify` requires lowercase pin equal to
  raw-byte SHA-256 and rejects unknown version/stage/field/hash shapes and
  a broken commitment.
- `Bench::prepare` returns `Preparation`; combined authoring files must
  equal the exact duplicate-free union of both partitions. Review,
  annotation and notes now consume the shared 256-MiB byte and 100k
  record budgets (`parse_jsonl_within`; history/grade entries charged).
- `load_dev` is crate-private and checks the manifest's config/gates/
  splits/pool, corpora and dev partition; public cross-process admission is
  `Bench::load_dev_pinned(config, manifest, pin)`.
- CLI: `check-benchmark --stage qrels --preparation-out PATH --operator
  --operator-evidence` publishes only after all gates pass and prints the
  manifest SHA-256 and commitment.
- Tests: manifest binding, counts, tamper/version/pin refusals and pinned dev
  admission with held-out, combined and provenance files removed; union
  mismatch/duplicate and operator refusals; nested budget charging; CLI
  publish-once, no publication on gate failure, operator required.

GPT-6-Astra review (interrupted after first findings, read-only):
fixed — public `load_dev(&Preparation)` let external callers admit a
deserialized, unpinned manifest (now crate-private; tests use the pinned
path). Open, owned by later slices: (d) `capture` still reads the full pool
on selected dev (slice 4); (e) per-line typed allocation precedes the
nested-entry charge (bounded by the 32-KiB line cap, not by the budget);
qrels-stage pool parsing in `check_benchmark` is not charged (slice 4/8).

Validation: job `eval13-s3-e`: eval_gates 20, eval_run 6, `--lib eval::` 41
passed; fmt ok; Clippy with `-A clippy::unused_unit` ok; plain Clippy 101
solely at `src/store.rs:745`.

## Slice 4 Done: Selected Dev Capture Without Full Pool

- `run.rs::capture`: the full pool is read only on full preparation
  (`selected == None`); `CapturedInputs.pool` is `Option`. Selected dev
  never opens, stats or hashes it (combined files were already never read).
  `load_dev` no longer compares a locally computed pool hash; it adopts the
  verified manifest's committed pool hash into the capture identity, so
  registration still binds it. `pool_bytes` on a selected capture refuses.
- Tests: the cross-process dev admission tests now replace `pool.jsonl`,
  query-review, annotation and notes (plus held-out and combined files) with
  unreadable directories and still admit; pool tampering is no longer a dev
  refusal (the manifest binds it).

Validation: job `eval13-s4-b`: eval_gates 20, eval_run 6, `--lib eval::` 41
passed; fmt ok; Clippy with `-A clippy::unused_unit` ok (plain Clippy blocker
at `src/store.rs:745` unchanged).

## Slice 5 Done: Test Capture After Attempt Reservation

- `run.rs`: `capture`/`load_selected` handle either selected split
  (selected digests are that partition's raw hashes). `load_dev` delegates.
  `Bench::load_test_reserved(config, preparation, custody, out)` first
  checks the registration binds this manifest's partitions and pool (reads
  only custody, no held-out data, consumes nothing), then `start_attempt`
  (create-new + fsync), then captures the test partition; a capture
  failure writes a `failed` result, consuming the attempt.
  `run_held_out(config, preparation, pin, confirm, out)` requires
  confirmation, the universal guard, then reservation-first capture and
  the existing `run_reserved` body (failure after reservation consumes).
  `run` keeps its previous behaviour via `run_reserved`.
- Tests: unbound registration refuses with no attempt file; an unreadable
  test qrels file after reservation leaves a `failed`, non-retryable
  attempt; a bound readable capture holds exactly the test partition
  (180 queries; raw test hashes) and a second reservation is refused; dev,
  combined, pool and provenance payloads are unreadable throughout.
  `run_held_out` refuses unconfirmed, and confirmed under the guard.

Validation: job `eval13-s5-b` (before the last refusal test): `--lib eval::`
42, eval_gates 20, eval_run 6 passed; fmt and Clippy (`-A unused_unit`) ok.

GPT-6-Astra review of slices 4-5 (read-only, complete):
fixed — reservation preceded identity checks: `load_test_reserved` now also
requires the registration's config/gates/splits/pool/index hashes to equal
the manifest's before reserving (test: matching partitions, different
config → refused, no attempt file). Validation job `eval13-s5-d`:
`--lib eval::` 43, eval_gates 20, eval_run 6 passed; fmt, Clippy
(`-A unused_unit`) ok.
Open, assigned:
(f) High: layout isolation. Selected capture still runs pin `git status`
    (repository-wide `:/`) and the freshness walk; if benchmark payloads sit
    inside the corpus root or its enclosing checkout, git/the walker can stat
    them. Preparation must refuse overlapping layouts (slice 9 admission).
(g) Frozen code/export and power preflight still run after reservation in
    `run_reserved`; must move before `start_attempt` when slice 9 enables
    admission. Currently unreachable (universal guard).
(h) `run_reserved`/`score_captured` require a dev capture and registration
    compares dev digests against test digests, so held-out computation cannot
    yet succeed (slice 6 registration/frozen format + slice 9).
(i) Registration does not bind the whole preparation hash (slice 6).

## Slice 6 Done: Versioned Freeze, Registration and Run Metadata

Fixes findings h and i.

- `FrozenConfig` v2 (`FROZEN_VERSION`, `src/eval/sweep.rs`): adds
  `version`, `preparation_sha256` and the dev/test queries and qrels digests
  from the manifest. These replace the combined `queries_sha256`/`qrels_sha256`.
  The struct is `deny_unknown_fields`; `from_toml` refuses any other version,
  so v1 files are refused, not migrated.
- `FrozenConfig::preparation_problems(bench)` checks the manifest hash, both
  committed partitions and the captured split's actual digests.
  `check_registration` and `check_frozen` use it, so a freeze made from a dev
  capture now binds a later test capture (h).
- `Registration` v2 (`REGISTRATION_VERSION`, `src/eval/run.rs`) binds
  `preparation_sha256`. `load_test_reserved` compares it before
  `start_attempt`, so no test payload is read and no attempt is consumed on a
  mismatch (i).
- `RunMeta` v2 (`RUN_META_VERSION`, `deny_unknown_fields`) records `version`
  and `preparation_sha256`; `score_captured` refuses other versions or hashes.
- `require_dev_capture` is replaced by `require_prepared_capture(split)` and
  `preparation_binding()`. Both require a selected, manifest-admitted capture
  of the named split.
- `Preparation` holds the verified pin, or the digest of its own `to_bytes`
  when freshly prepared, as a non-serialized `sha256`. It does not recompute
  the digest by reserialization: the Astra review (P2) showed that default
  serde_json floats need not round-trip byte-identically.
- Tests added:
  - test-capture digest binding and refusal of a different manifest hash
    before reservation;
  - whole-manifest hash binding in `check_registration`;
  - v1/v3/unknown-field frozen TOML refusals;
  - RunMeta version and hash refusals;
  - typed-mutation semantic refusals for `Preparation::verify` (Astra P3);
  - minified-bytes admission keeps the raw-byte digest.
- Lane: `eval13-s6-a` exited 101 on Clippy `type_complexity` (fixed with the
  `PartitionPairs` alias); `eval13-s6-b` exited 0. `eval13-s6-c` exited 0
  after the Astra fixes, running the same command: `lib eval::` 43 passed,
  eval_gates 20 passed, eval_run 6 passed, fmt ok, and Clippy ok with
  `-A clippy::unused_unit` (the `store.rs:745` blocker).
- GPT-6-Astra read-only review, recorded findings:
  - P2: canonical reserialization check: fixed as above.
  - P3: masked semantic tests and missing RunMeta negatives: fixed.
  - Confirmed: no missing hash or digest check, h fixed, and the manifest hash
    is compared before any test payload read.
  - Deferred to slice 9: `check_frozen` counts test answerables from the
    selected capture, which is zero for a dev capture at freeze time.
    Code/export/power checks still run after reservation (g).

## Slice 7 Done: Publish After Completion, Failure Injection

- `run_reserved` ends in `publish_run` (`src/eval/run.rs`). It writes
  `report.json` to the private stage, publishes by fsync and rename, rereads
  the published `meta.json` and `report.json` and compares them with the
  computed bytes. Only then does it record `completed` custody.
- `consume_on_error` writes a `failed` terminal record for any held-out
  error. It never replaces an occupied slot. Instead it creates
  `attempt-voided.json` (create-new), which makes `verify_completed` refuse.
  This closes the Astra P1 gap: a readable `completed` record whose fsync
  failed could otherwise have been accepted.
- Residual risk: if writing the failed record and writing the void marker
  both fail, the error reports both. That is a double filesystem fault, and
  the run still returns Err.
- New test `publication_or_completion_failure_leaves_consumed_unaccepted_attempt`
  injects failures deterministically through filesystem states:
  - an output directory created after staging: publication fails, the attempt
    is recorded `failed` and there is no report;
  - a partial terminal record already in the slot: the output is visible, but
    the record is not replaced and is never accepted;
  - a completed record followed by an injected error: the record is voided and
    refused;
  - success binds the reread bytes;
  - a dev run publishes without custody records.
  No retry is possible after any of these.
- Lane: `eval13-s7-a` exited 0 before the P1 fix and `eval13-s7-b` exited 0
  after it. Both ran: `lib eval::` 44 passed, eval_gates 20 passed, eval_run
  6 passed, fmt ok, and Clippy ok with `-A clippy::unused_unit`.
- GPT-6-Astra review:
  - P1, completed record with a failed sync: fixed with the void marker.
  - Not covered, deferred to slice 9's end-to-end tests: post-rename sync
    failure (output visible, then recorded `failed`) and the public run/score
    path, which is still refused by the universal guard.

## Slice 8 Done: Per-Corpus Exploratory CIs, Precomputed Gate Lookups

- `Report.per_corpus_comparisons` (`src/eval/run.rs`) is built by
  `per_corpus_comparisons`. It recomputes every pre-registered spec (H1, H2,
  H5 and the four E1 contrasts) within each corpus, keeping each stratum
  filter. Output is in sorted corpus order with ids `<corpus>/<spec>`.
- These rows are exploratory only: bootstrap CI and raw p, `holm_p` and
  `decision` always None. Holm and decisions still cover only the pooled
  H2/H5 family.
- `src/eval/gates.rs`:
  - `check_qrels` builds a borrowed `(corpus, section_id)` map once; the
    first section with an id wins, as the former `find` did.
  - `check_leakage` tokenizes each query once; the last duplicate id wins, as
    the former `collect` did.
  - `jaccard` delegates to `token_set`/`jaccard_sets`.
  - Failure order is unchanged.
- New test `per_corpus_comparisons_do_not_mix_corpora_or_join_family` uses two
  corpora with hand-computed effects: a/H2 mean 0.5 with CI [0.5, 0.5], b/H2
  mean -0.25, pooled 0.25. It also checks that the stratum filter is kept,
  that no row joins the family, and deterministic serialization. The scoring
  test asserts seven per-corpus rows for its single corpus.
- Lane runs:
  - `eval13-s8-a` exited 0 (H1/H2/H5 only).
  - `eval13-s8-b` exited 101 on Clippy `type_complexity`, fixed with the
    `MetricFn` alias.
  - `eval13-s8-c` exited 0: `lib eval::` 45 passed, eval_gates 20 passed,
    eval_run 6 passed, fmt ok, and Clippy ok with `-A clippy::unused_unit`.
- GPT-6-Astra review:
  - P2: E1 contrasts were missing from the per-corpus rows. Fixed.
  - P2: a single-corpus test could not detect corpus mixing. Fixed with the
    two-corpus test.
  - Confirmed: gate semantics and failure order preserved, no family
    contamination, determinism, no consumer breakage, and bounds covered by
    existing admission caps.
  - Not covered: duplicate-id precedence tests. Loaders already reject
    duplicates.

## Remaining Slices (After Slice 8)

- 9: DONE (see Slice 9 Done). Was: validated admission, covering layout isolation (f), pre-reservation
  preflight (g), answerable-test counts from the manifest, CLI wiring,
  post-rename sync failure and end-to-end tests.

Blocker: `src/store.rs:745` Clippy `unused_unit` needs an owner task or user
approval before EVAL-1.3 can close.

## Slice 9 Done: Validated Admission

- Every measurement command (run, score, sweep, freeze, check-frozen, misses)
  needs `--preparation` and `--preparation-sha256`. Without them it refuses
  before any read. `check-benchmark` at stage run or with `--run` always
  refuses. `run --split test` routes to `run_held_out`, and
  `score --confirm-test-split` routes to `score_held_out`. `run()` and plain
  `score` refuse the test split.
- Selected captures (`load_selected`) bind the config, splits and gates
  bytes to the manifest's `inputs` hashes before any partition payload is
  read. A config that swaps the dev and test paths refuses without reading
  held-out bytes.
- `check_layout` refuses when the benchmark directory, or the parent of any
  payload, sits inside a corpus root or its git toplevel. This covers
  partitions, splits, gates and the preparation-time pool. Parents are
  canonicalized, so symlinked ancestors resolve. A checkout extent that
  cannot be determined also refuses.
- `held_out_admission` runs before reservation. It checks:
  - registration and checkout separation;
  - that the output path is external and does not yet exist;
  - that the frozen export bytes equal the registration;
  - the preparation, partitions, config, splits and generation ids;
  - resamples, and the manifest's answerable-test count against power;
  - code identity;
  - corpus freshness and pins, via `load_selected(Dev)`.
- `score_held_out` verifies the pin, checks that the registration's
  preparation hash matches, runs `verify_completed`, and then scores.
- Tests:
  - `admitted_protocol_runs_dev_then_one_held_out_attempt` runs end to end,
    with held-out and combined files hidden during dev work. It ends with one
    completed attempt and no retry.
  - `benchmark_inside_corpus_checkout_is_refused`
  - `partition_inside_corpus_checkout_is_refused`
  - `swapped_partitions_refuse_before_reading_payloads`
  - `stale_corpus_refuses_held_out_before_reservation`
- Lane runs:
  - `eval13-s9-c` and `eval13-s9-d` exited 0.
  - `eval13-s9-e` exited 101: the config hash was checked before layout,
    which masked the layout refusal. The check order was fixed.
  - `eval13-s9-f` exited 0: lib `eval::` 45 passed, eval_gates 20, eval_run
    11, cli 8; fmt ok; Clippy ok with `-A clippy::unused_unit`.
- GPT-6-Astra review of part 9:
  - P1: freshness and pins were not checked before reservation. Fixed.
  - P2: the splits and pool paths were not checked. Fixed.
  - Config-hash-before-payload: fixed.
  - Nested or symlinked partitions inside a corpus checkout: fixed.
  - A failed `rev-parse` failed open: it now refuses.
  - Stale test names: fixed.
  - The fixes have not been re-reviewed.
- Residual limits:
  - Local custody is not an independent audit.
  - Synthetic fixtures only; no real benchmark was run.
  - A sync failure after rename is not injected.
  - Layout checks canonicalize at capture time; this is not race-safe
    against concurrent filesystem changes.

Closeout blocker: plain Clippy (`-D warnings`, without the allowance) still
fails only on `clippy::unused_unit` at `src/store.rs:745`, which is outside
the EVAL-1.3 write set. EVAL-1.3 stays OPEN until that line is approved or
owned.
