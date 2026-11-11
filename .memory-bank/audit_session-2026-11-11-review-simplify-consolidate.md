# Review, Challenge And Consolidation (2026-11-11)

Authority: user request "review and challenge what has been planned and done
... consolidate to one plan and proceed; use OntoIndex; use GPT sub-agents".
Reviewers: three independent read-only GPT-6-Astra explorers. They covered:
- the eval subsystem;
- the product/retrieval path;
- planning and process.

The main session verified every finding cited below against source before
adopting it. No benchmark, held-out or label data was read.

## Done Well (Keep)

- Strict task cards with recorded validation. Honest refusal to claim retrieval
  quality. Synthetic-only fixtures.
- The statistics core (`src/eval/stats.rs`) is compact and checked against
  worked examples. This covers the stratified paired bootstrap, sign-flip
  randomization, Holm and the decision rule.
- Physical split isolation and binding before reads. Exact-byte saved scoring.
  Consumed-attempt custody. These are ADR6/ADR9 requirements, not ceremony.
- Safety on the stdio path: rooted reads, bounded responses, publication
  outcomes and fail-fast admission.

## Worst / Over-Engineered

- **Process outweighs product.** The plan is 1,923 lines for 40 tasks; the ADR
  is 979 lines and also serves as a decision log; there are 30 audit files.
  Several turns produced zero progress while the owner-choice gate stayed
  shut. Status is spread over five documents that contradict each other:
  - plan Decisions says "EVAL-1.3 remains incomplete";
  - ADR headings still say "Not Implemented" for sections that are
    implemented;
  - pending-tasks and EVAL-3.0 claim "no initial commit", but HEAD is
    `b6e3f3e`.
- **Eval orchestration duplication.** `run.rs` is 3,053 production lines.
  Over-engineered items:
  - freeze runs the dev sweep twice (`cli.rs` Freeze, then `sweep::freeze`
    recomputes it to compare);
  - frozen, code and registration checks are repeated across
    `check_registration`, `held_out_preflight`, `held_out_admission`,
    `run_reserved` and `check_frozen`;
  - `gates::kappa_from_counts`/`percentile` duplicate
    `stats::weighted_kappa`/`quantile`;
  - the CLI `check-benchmark --stage run`/`--run` branch is dead;
  - TOML is hand-rolled.

  Estimated recoverable: 300–500 production LOC, with no safeguard dropped.
- **store.rs/corpus.rs.** Legacy non-rooted entry points kept for test oracles
  (`freshness`, `load`, `build`) double the surface. They are kept and frozen;
  removal is low value now.

## Forgotten / Stalled

- **The product goal has no code.** That goal is a multi-user HTTP service with
  one endpoint per `docs_share/` folder. The folder registry (HTTP-1) excludes
  listener startup, yet depends on the whole of HTTP-0, so it was blocked for
  no technical reason.
- **Concurrency.** `OntoDocServer::blocking` holds the index-slot mutex across
  the whole request, with a semaphore of 1 (`src/mcp.rs` ~348–390). HTTP
  cannot parallelize by raising permits alone.
- **Per-request cost.** Every search/read/related/status runs a full
  current-byte corpus freshness walk (`App::freshness` → `freshness_rooted`).
  That is O(corpus bytes) per call. Weakening it needs an ADR decision; it is
  not a silent change.
- The evidence-diagnostics ADR is still Proposed with no decision task. The
  GraphRAG donor ideas are not mapped to any task.

## Verified Bugs (Adopted)

1. **`src/cite.rs` `verify_bytes_with`: wrong moved line numbers.** Moved-span
   line numbers come from `String::from_utf8_lossy(bytes)` but are indexed
   with original byte offsets. Invalid UTF-8 before the span shifts them, so
   the line numbers are wrong.
2. **Rejected on re-check: `App::search` `truncated`.** ADR §8 says a tier that
   does not fit "stays at the previous tier and is never truncated", and each
   result reports its `tier`. A downgrade is therefore intended behaviour, not
   silent truncation, so the flag stays reserved for omitted results.
3. **`src/eval/run.rs` `validate_config`: `resamples` unbounded.** It does not
   bound `resamples`, so admitted dev scoring can allocate and loop without
   limit; zero gives meaningless CIs. Only freeze and held-out runs enforce
   10,000.
4. **`src/eval/run.rs` `prepare`: layout gap.** Combined queries/qrels and the
   trusted review/annotation/notes paths are read without `check_layout`.
   Placed inside a corpus checkout, they could later be inspected by
   freshness or `git status` during dev runs.

Recorded but deferred (each needs an ADR or format decision):
- [FIXED by FIX-2, lane fix2-b] The frozen TOML codec did not round-trip arbitrary corpus names. It
  writes Rust debug escapes and reads them back with a JSON decoder.
- The 128-byte identifier limit conflicts with section IDs up to 4,096 bytes.
- A failed held-out H1 leaves no unaccepted diagnostic artifacts.
- The `ChunkBoundary` miss attribution is heuristic.
- The 6,000-pair pool fallback is not enforced.

## Options For The Next Work

| Option | Robustness | Performance | Decision |
| --- | --- | --- | --- |
| A. Keep waiting for owner gates | n/a | n/a | Rejected: zero progress |
| B. Fix verified bugs, then delete small duplications | High (focused tests) | Neutral or better | **Adopt first** |
| C. Large eval refactor (merge admission checks, captured-object split) | Medium risk | Some speedup | Defer; only after a real benchmark exists |
| D. HTTP-1 folder registry now, stdio-testable, no listener | High | Neutral | **Adopt**: split HTTP-1's dependency on HTTP-0 |
| E. Concurrent read path: drop the mutex during the request, `Arc` snapshot, N permits | High if ownership is kept | Large under HTTP | **Adopt after D**: needed before any listener |
| F. Loopback listener default for HTTP-0 | Medium: still an exposure choice | — | Keep owner-gated, but propose loopback as the recommended default |
| G. Freshness cache or TTL | Weakens the current-byte guarantee | Large | Rejected without an ADR amendment |

## Consolidated Plan (Cherry-Picked)

1. **FIX-1:** cite moved-line bytes, eval `resamples` bound, prepare layout
   coverage. Each gets a focused
   regression test.
2. **SIMPL-1:**
   - freeze computes the sweep once;
   - remove the dead `check-benchmark --run`/`--stage run` branch;
   - gates reuse the `stats` quantile and kappa.

   Behaviour is unchanged and the existing tests stay green.
3. **DOC-1:** fix stale status and ADR markers; archive DONE cards into an
   audit file so the active plan becomes short.
4. **HTTP-1:** `docs_share` folder registry with per-folder `App`/store/index
   slots and no listener. Its dependency on HTTP-0 is split: the listener,
   exposure, auth and session work stay in HTTP-0/HTTP-2.
5. **PERF-1:** concurrent read admission. Snapshot `Arc<SearchIndex>` under a
   short lock, keep the permit owned by the work, and make N permits
   configurable. Freshness semantics are unchanged.
6. HTTP-0 stays owner-gated; the recommendation is loopback plus an
   authenticated reverse proxy. EVAL-2.x stays owner/human-gated.

## Execution Outcome

All four tasks were closed through strict `complete-local`.

| Task | Lane (exit 0) | Result |
| --- | --- | --- |
| FIX-1 | fix1-a | raw-byte moved-citation lines; resamples 1..=10000 at config admission; trusted/combined preparation paths are layout-checked inside `capture`, before freshness/git (moved there after Astra review) |
| SIMPL-1 | simpl1-c | `freeze(bench, threshold, code)` sweeps once; unreachable saved-run scoring branch deleted; `gates::percentile` delegates to `stats::quantile` |
| HTTP-1 | http1-b, then perf1-c | `mcp::Registry::discover`: sorted immediate directories, at most 32 corpora and at most 4,096 entries; names kept verbatim, refused only if they cannot be one lossless segment; symlinks/special entries refuse; only `.git`/`.ontodoc` and plain files skipped; exact lookup with no fallback |
| PERF-1 | perf1-c | mutex limited to the CURRENT check, reload and `Arc` snapshot; `with_permits(1..=64)`, stdio stays at 1; permits are owned by the work; a tool panic no longer poisons the cache |

Final lane perf1-c:
- `cargo test --lib`: 199 passed, 1 ignored
- mcp 12, cli 8, eval_gates 20, eval_run 13
- `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`: ok

Failed intermediate lanes:
- simpl1-a/b exited 101 on an unused binding.
- perf1-a exited 101: the join-failure test asserted the old poisoned-cache
  behaviour.

GPT-6-Astra delta review: no P1 findings. Every P2/P3 finding was fixed in
perf1-c:
- registry naming, hidden-entry handling, classification order and entry
  bound;
- preparation layout check now precedes corpus inspection;
- reload-concurrency test added;
- the barrier wait is now bounded.

The fixes were not re-reviewed.

Plan size: the `expected_evidence` lists of all 34 DONE cards were moved
verbatim, with hashes, to
`audit_session-2026-11-11-done-card-evidence-archive.md`. The tracker went
from 122,502 to 100,263 bytes. Parsed state is otherwise identical, and this
was checked by script.

Still owner-gated:
- HTTP-0: exposure/auth choice. Recommendation: loopback plus an
  authenticated reverse proxy.
- EVAL-2.0: corpora, query approval and human annotators.

Not done, and not claimed:
- HTTP listener;
- freshness caching (needs an ADR);
- the larger eval admission consolidation;
- frozen TOML round-trip of unusual corpus names;
- 128-byte section ID conflict;
- retained failed-H1 diagnostics.

## Astra re-review of perf1-c fixes (2026-11-11)

- Reviewer: GPT-6-Astra explorer, read-only. Production code in `blocking()`,
  `Registry::discover` and `capture(.., trusted)`: no findings.
- P2 (tests only): the concurrency tests in `src/mcp.rs` waited inside the
  blocking worker with no limit (`Barrier::wait`, `recv`), so a regression
  would hang runtime teardown instead of failing. Fixed by moving the limit
  into the worker: an atomic rendezvous with a 10 s deadline, and
  `recv_timeout(10 s)`.
- Lane `perf1-d`: `cargo test --lib mcp::` passed 16, `cargo test --test mcp`
  passed 12, `cargo fmt --check` ok, and `cargo clippy --all-targets -D warnings`
  ok. Exit 0.

## FIX-2: frozen TOML round-trip of corpus names (2026-11-11)

- Defect: `to_toml` wrote Rust `{:?}` escapes (`\u{1}`), which the JSON-based
  reader refused, and a name containing ` = ` split at the wrong place.
- Fix (in `src/eval/sweep.rs`): `toml_string` writes escapes valid in both TOML
  and JSON. The parser reads the key as a leading JSON string. Ordinary names
  produce the same bytes as before, and the version was not bumped.
- Lane `fix2-b` (exit 0): `cargo test --lib eval::sweep` passed 4, and fmt and
  clippy passed. Closed through strict `complete-local`.

## Full validation after FIX-2 (2026-11-11)

Lane `full-1` exit 0: `cargo test --all-targets` (lib 200 passed 1 ignored;
cli 8, eval_gates 20, eval_run 13, mcp 12), `cargo fmt --check` ok,
`cargo clippy --all-targets -D warnings` ok. Plan still blocked on owner
gates HTTP-0 and EVAL-2.0 and on the section-ID ADR conflict.
