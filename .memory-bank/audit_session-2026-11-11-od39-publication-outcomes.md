# OD-3.9: Publication Outcomes And Process Locks

Date: 2026-11-11 (work began 2026-11-10)
Authority: service ADR revision 8; user proceed; strict manager selected
dependency-ready OD-3.9 for local execution without a lease.

## Implemented

- `src/store.rs:668`: `Result<PublishOutcome, StoreError>` distinguishes
  precommit errors from committed durability/maintenance outcomes. CURRENT
  rename is the commit point. A failed subsequent directory sync returns
  uncertain/not_attempted; HISTORY/prune failure returns synced/failed.
  No postcommit error propagation or rollback is added.
- Namespace creation syncs containing parents before commit, including retries
  after mkdir succeeds but its parent sync fails. Reused artifacts sync both
  files, candidate directory and generations directory. Administrator-controlled
  ancestor aliases are accepted; owned store/generations symlinks refuse.
- HISTORY is bounded to 195 bytes and three distinct canonical LF-terminated
  IDs. A shared 100,000-probe cleanup budget is charged before enumeration,
  including skipped entries, EOF and injected errors. Inventories are validated
  before deletion; only expected regular files and empty owned directories are
  removed. No recursive deletion, unknown-tree traversal or CURRENT deletion.
- Active corrupt-candidate repair now refuses. The old repair regression
  deleted/replaced the current generation, conflicting with ADR8; bounded
  inactive repair remains supported. This applies existing authority, not a
  changed format or an implicit ADR amendment.
- `src/cli.rs:443` and `tests/cli.rs:69`: index JSON keeps prior fields and adds
  committed, durability, maintenance, failure_phase and finite warnings.
  Committed maintenance failure exits 0, keeps the new CURRENT/readable data
  and preserves malformed HISTORY. Explicit administrator recovery is tested.
  Output loss still cannot establish noncommit.
- Fault matrices cover precommit phases, namespace creation/retry, exact reuse,
  postcommit sync, HISTORY, cleanup/repair and pruning. CurrentSync failure
  proves maintenance was skipped and HISTORY unchanged.
- Subprocess tests (`src/store.rs:1642–1850`) acquire locks in separate native
  libtest processes, checking PID and actual held-descriptor device/inode in
  lock mode. Normal release and owner death permit reacquisition without
  unlinking. Eight publication barrier kinds check same-store exclusion and
  different-store publication through commit/maintenance. Precommit error,
  uncertain, maintenance-failed and complete returns release before child exit.
  Helper output/line counts, channels and receives are bounded; RAII kills/reaps
  on failure. No sleeps, blind retries, environment mutation or lock changes.

## Historical Failure And Limits

The EVAL-1.5 parallel failure remains **unexplained**. The retained
`ontodoc-eval15-tests-20261110a/output.log:161` records `Locked`, terminal 101;
no contemporaneous PID/inode/descriptor owner evidence identifies its cause.
Later serial success and these new default-parallel/process tests do not prove
a historical fix. Descriptor inheritance remains only a hypothesis.

Publication assumes an administrator protects redirectable store ancestors;
this is not hostile-administrator containment. Injected pre-operation failures
establish phase behavior, not power-loss/crash durability. Process receive
deadlines do not bound kernel filesystem stalls, wait/reap or join wall time.
Publication-mode metadata describes the lock path, not the held descriptor.
No full-library/workspace test, real benchmark, HTTP or performance/RSS claim.
Evaluation remains disabled; schema, ranking and dependencies are unchanged.

## Review And Ownership

Requested/effective GPT-6-Astra read-only reviewers:

- `01a1c737-c253-77d1-92a8-7580188a54f8`, results `/3`–`/5`: required exact
  namespace retry/reuse fault evidence was added; final alias correction and
  production delta had no scoped blockers. Prior `/2` reviewed CLI adapter/tests.
- `01a1c737-96ed-7cc3-a1ce-6d872a74a62b`, result `/3`: process harness and
  ownership/lifetime evidence had no scoped blockers, with the limits above.

Store worker `01a1c73c-95c0-75b2-b366-d3706ccb97d9` owned only `src/store.rs`
until handoff; parent added process tests/shared CLI integration and corrections.
Children ran no compiler/tests/formatter; the worker's `git diff --check` exit0
is not meaningful scope proof for this untracked file. Dispatch packet shape
corrections preceded execution; these were not manager receipt rejections.
All three children are closed. Reviews are source-only, not runtime evidence.

## Validation

Native root package, Rust 1.95.0. Managed serial compiler jobs live under
`/opt/d2/.ontocode/artifacts/compiler-lane/`; canonical result reader receipts
are terminal authority. No overlapping compiler job was found before launches.

Job `ontodoc-od39-20261110a`, receipt `acd684`, terminal **101**:
first store command failed compilation because the private path-aware test
closure needed an explicit `&Path` annotation. Corrected once.

Job `ontodoc-od39-20261110b`, receipt `a363af`, terminal **101**:
store tests compiled; 9 passed, 33 failed, 1 helper ignored. Namespace recursion
rejected the host's root-owned `/tmp` symlink. Source/host receipt `9f45a5`
established `/tmp -> /opt/d1/tmp`. Corrected trusted ancestor handling while
retaining owned-directory refusal, with an explicit synthetic regression.
All subsequent commands in those two failed chains were skipped.

Job `ontodoc-od39-20261110c`, receipt `69cfcc`, terminal **101**:

```sh
CARGO_BUILD_JOBS=4 cargo test --lib store::
CARGO_BUILD_JOBS=4 cargo test --lib cli::
CARGO_BUILD_JOBS=4 cargo test --test cli
CARGO_BUILD_JOBS=4 cargo test --test mcp --test eval_gates --test eval_run
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

Every test command exited **0** before Clippy failed: store43 passed/1 helper
ignored (executed explicitly by process tests); CLI units6, CLI integration8,
MCP11, gates14 and run6 passed. Tail truncation was resolved by reading only
test summary lines from the retained log (receipt `c393d3`). Store used normal
default-parallel execution, not suite serialization. Clippy found manual prefix
stripping and a collapsible conditional; equivalent local rewrites corrected
both once. The formatting commands in that chain were skipped.

Job `ontodoc-od39-20261110d`, receipt `299a88`, terminal **0**:

```sh
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

All passed; no behavioral edits followed test job c, only those equivalent
style fixes and formatting. No repeated unchanged failure, serial-only lock
workaround, rejected role receipt, stale revision repair or disk recovery.

OntoIndex navigation/impact and final verify_diff refused unknown indexed
commit identity (`299a88` outer call for final audit); source is authority.
No graph PASS or manual reindex. Publication treated as HIGH-risk boundary.
The checkout has no initial commit: empty Git diff does not prove scope.
Baseline `/tmp/ontodoc-od39-baseline-uah1krha.json` and receipt `332996`
show exactly three implementation files changed before manager documentation:

```text
src/store.rs 4225bce4bc4a1f02ea4ecd6eac76a4aa0ec7938b97110f9ee7eaeac739b1c1f4
src/cli.rs fb4bcacf7099d3a958317ec4994907bce9721f51dfb5b8750a5f0961db009661
tests/cli.rs 3d5634a6143a02faa40894ec646f33692d43a513bd6965cd7bd7710ac30605cf
```

Manager closeout is limited to audit/plan/status/pending/memory/architecture.
Strict complete-local, not this prose, controls completion and successor choice.

## Closeout

Strict `complete-local` completed OD-3.9 and selected dependency-ready EVAL-1.6;
revision `b08b83e60c44466698776a4b2b15726083ce91b9ae73cb78b96284197d47759c`.
Counts: 28 DONE, 9 OPEN, 2 BLOCKED /39. Plan validation passed before completion;
reference/fence/whitespace/memory-cap/precloseout scope checks and diff-check
exit0 (`129e00`). No manager rejection or stale revision occurred.
Status/pending/memory/architecture now route to EVAL-1.6. The tracker has fewer
than 1 KiB remaining below its 128-KiB parser ceiling; compact verbose evidence
into exact-linked audit records without deleting authenticated state before
adding further task evidence. HTTP and evaluation external gates are unchanged.
