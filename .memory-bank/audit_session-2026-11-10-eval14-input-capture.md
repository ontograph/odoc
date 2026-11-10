# EVAL-1.4: Partial Immutable Input Capture

Date: 2026-11-10
Authority: service ADR revision 5 and the active EVAL-1.4 task card.
Disposition: **OPEN; partial implementation retained, not accepted.** No task
completion, successor selection, measurement enablement or HTTP change.

## Implemented Scope

- `src/eval/run.rs`: physically separate dev/test payload paths; full preparation
  produces opaque, non-deserializable `MechanicalEvidence` only after mechanical
  gates pass. Dev admission selects paths before reading payloads, checks captured
  identity and current freshness/pins, and retains both partition commitments.
- Captured qrels, split assignments, gate policy, pool bytes and `Arc` indexes
  are reused. Pool regeneration compares in-memory bytes. Private captured dev
  computation refuses missing positive judgements. Saved scoring parses the
  exact ranking/abstention bytes whose hashes it checked.
- Payload reads use a single nonblocking/no-follow final-component open and
  descriptor regular-file/size checks. Caps include 16 MiB/payload, 128 MiB/index,
  256 MiB aggregate capture, 100,000 JSONL records, 2,000 queries and 100,000
  ranking jobs. These caps do not establish full admission boundedness.
- `src/store.rs` shares artifact checksum/schema/identity validation between its
  ordinary loader and captured-byte decoder, and shares the freshness comparison.
  `src/corpus.rs` adds optional Markdown-byte and yielded-entry counters, leaving
  the ordinary walk entry point and ignore policy in their existing owner.
- Production changes: `src/corpus.rs`, `src/store.rs`, `src/eval/run.rs`,
  `src/eval/gates.rs`, `src/eval/sweep.rs`. Test changes: `tests/common/mod.rs`,
  `tests/eval_gates.rs`. `src/cli.rs`, `tests/eval_run.rs`, dependencies,
  artifact schema/IDs and ranking algorithms are unchanged.

These APIs do not establish reviewed queries, human annotation, durable custody,
race-safe ancestor containment or multi-user HTTP readiness. Public measurement
refusals remain. No real benchmark queries, labels or results were read or run.

## Review And Acceptance Gap

Requested and effective model: GPT-6-Astra, read-only explorer agents
`eval14_flow` and `eval14_gates`. Neither changed files or ran validation.
Flow review found stale dev admission, lost held-out commitments and a possible
blocking FIFO open; the draft adds admission rechecks, retained commitments and
nonblocking/no-follow payload opens with synthetic regressions. Those repairs
are not a blanket final approval of every changed path.

The final gates review recommends leaving EVAL-1.4 OPEN. Current counters run
after `ignore::Walk` yields (`src/corpus.rs:140`), missing internal work:

- Installed `ignore-0.4.25/src/walk.rs:1057`: ignore filtering precedes the
  public `filter_entry` callback; root entries bypass that callback.
- `walk.rs:1098-1157`: ignored files are skipped internally; even rejected
  directories invoke `add_child` before the loop continues.
- `dir.rs:269` and `gitignore.rs:392`: automatic ignore-file loading reaches
  unrestricted `File::open` and `BufRead::lines` before sufficient accounting.

These locations are in the installed Cargo registry dependency, not donor code.
Review source receipts: `e7d1fe`, `063309`, `db19b2`; resume inspection also
confirmed the callback/read ordering and current corpus call site. There is no
sufficient installed public hook demonstrated by this review.

**Proved limit:** payload/index byte caps, Markdown-read reservations and yielded
entry caps. **Unproved limit:** complete freshness traversal and ignore-file
ingestion. Passing current tests does not close this acceptance gap. A prior
enumeration cannot bind a subsequent mutable traversal.

## Next Scoped Action

Select a shared corpus traversal/dependency seam that accounts for enumeration
before filtering and bounds ignore-file reads, while preserving current ignore
precedence, nesting, negation, symlink treatment and freshness semantics. Scope
the implementation and required parity/exhaustion regressions in the existing
task before editing. Do not duplicate the walker in eval, weaken ignore policy,
introduce a broad custom framework, or claim the HTTP/rooted-reader tasks pass.

This is unresolved implementation/design work, not an asserted external owner
gate: keep the task OPEN with the concrete gap recorded in acceptance and plan
decisions. `known_blockers` remains empty because a substantive entry disables
dispatch readiness; the remaining engineering work requires no invented owner
gate. No formal BLOCKED transition or evidence-backed completion was performed.

## Validation Evidence

Native root package, Rust 1.95.0, `CARGO_BUILD_JOBS=4`. Commands below were
recorded during the interrupted implementation session. Their managed terminal
receipts were re-read successfully on resume using:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 \
  /opt/d1/_wrk/ontocode/scripts/compiler_lane_detached.py \
  --result JOB --wait 60
```

Receipts live under `/opt/d2/.ontocode/artifacts/compiler-lane/`. Reader exit 0
means the receipt was read; command outcomes below are its nested exit codes.
No compiler command was relaunched during documentation closeout.

| Job (`ontodoc-eval14-` prefix) | Recorded command or correction | Command exit |
| --- | --- | --- |
| `check-20261110a` | `CARGO_BUILD_JOBS=4 cargo check --all-targets` | 0 |
| `focused-20261110a` | Incorrectly concatenated Cargo argv; unexpected `cargo` argument, no tests | 1 |
| `focused-20261110b` | Corrected shell sequence refused shared compiler lock; no bypass | 75 |
| `focused-20261110c` | Capture/store unit suites and required evaluation integration suites | 0 |
| `bounded-20261110a` | Focused command sequence below | 0 |
| `lint-20261110a` | `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`; items after test module | 101 |
| `lint-20261110b` | Same Clippy command after moving test module to the end | 0 |

Latest substantive test sequence, with `CARGO_BUILD_JOBS=4`:

```sh
cargo test --lib eval::run::tests &&
cargo test --lib corpus::tests &&
cargo test --lib store::tests &&
cargo test --test eval_gates --test eval_run
```

The interrupted session recorded 7 capture/scoring, 6 corpus, 17 store, 14 gates
and 6 run tests passing. The canonical receipt's bounded tail is truncated: on
resume it confirms terminal exit 0 and the latter four suite counts; it does
not independently display the capture suite count. Earlier focused receipt
displays 6 capture tests before the scoring regression was added. No full-suite
or live-corpus performance claim follows.

Coverage includes inaccessible held-out/combined payloads, post-capture file
deletion, cached index reuse, identity tampering, stale admission, malformed and
oversized inputs, symlink/FIFO refusal, invalid spans/duplicates, saved-byte
scoring tampering, and walker result parity/counter exhaustion. No regression
currently proves the ignore-internal work boundary.

The interrupted session recorded an initial `cargo fmt --check` failure, then
`cargo fmt && cargo fmt --check` exit 0 and `git diff --check` exit 0. Formatting,
comments and a test-name correction followed substantive testing/Clippy; no
substantive logic change followed. On resume exact current source hashes match
the stored post-format inventory. Tests/Clippy were not rerun on unchanged
source; document/plan checks are separate closeout evidence.

### Documentation Closeout Checks

- Parent inline Python link/anchor/fence/whitespace, memory-length and exact-file
  scope check: exit 0, receipt `98374c`; `MEMORY.md` has 70 lines. The same shell
  ran `git diff --check` with exit 0. Source scope is checked by hashes because
  Git does not track these files yet.
- An initial scope-check invocation constructed its allowlist incorrectly and
  flagged every changed file. Correcting the concatenation produced the PASS
  above; no source or allowed scope was changed to satisfy it.
- An initial plan update put the engineering gap in `known_blockers`, which
  made the selected task dispatch-ineligible and failed structural validation.
  The correction records remaining work in acceptance and plan decisions,
  preserving OPEN status and empty dispatch blockers. Validator then passed
  35 tasks with only the existing length advisory; strict manager-next selects
  local EVAL-1.4 execution at revision
  `4668e4eb7388af5d607d2edf89090db8034f458e6c8b7ae23632c0f6354ab9c9`.
  No manager state, completion receipt or role result was fabricated.

## Scope And Remaining Limits

- Plan scope was corrected to permit shared store decoding and optional corpus
  ceilings. Resume validator passed all 35 tasks; existing >1,500-line advisory
  remains. Counts stay 20 DONE, 13 OPEN, 2 BLOCKED.
- No initial Git commit exists. Exact-file SHA-256 inventory, not an empty Git
  diff, establishes the interrupted session's changed-file set. Resume source
  hashes matched that inventory before these documentation updates.
- OntoIndex's previously recorded workspace-preparation/unknown-commit failure
  leaves source inspection authoritative. No unchanged failing query or reindex
  was repeated; no graph-derived risk or scope PASS is claimed.
- All seven known compiler jobs have terminal receipts; resume process inspection
  found no compiler work running. Both completed review agents were closed.
- HTTP-0's exposure/admission decision and EVAL-2.0's evaluation prerequisites
  remain unresolved. Neither is waived by this partial implementation.
