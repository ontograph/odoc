# OD-3.5: Input, Verification And Actual-Work Budgets

Date: 2026-11-10
Authority: service ADR revision 8; user proceed; strict manager selected
dependency-ready OD-3.5 for local execution, without a dispatch lease.

## Implemented

- `src/mcp.rs`: a fixed-buffer AsyncRead guard bounds the actual native rmcp
  line collector to 128 KiB including LF/CRLF before JSON decoding. Exact-cap
  EOF is admitted; a detection byte permanently refuses without forwarding it.
  Pending/cancelled reads retain state and buffered following lines. Native
  transport closure is checked for the sanitized refusal; no request ID is invented.
- Borrowed JSON checks count all argument keys/nested strings (64 KiB), then
  per-field sizes, citation/heading counts and canonical hashes before typed
  conversion. Tool-name bytes are excluded. Typed entry points retain guards.
  `src/cli.rs` shares citation preflight and rejects before index loading;
  citation stdin uses a bounded cap+1 read.
- One server-owned fail-fast permit moves into the single blocking closure
  alongside the cache mutation lock. That closure owns CURRENT, reload/build,
  tool execution and structured serialization. Cancellation and runtime shutdown
  cannot free admission while work survives. Failed reload never serves the old
  index as current; panic/poison errors are generic and return the permit.
- `src/cite.rs`: one entire-batch budget precharges source reservations
  (128 MiB), SHA inputs (128 MiB) and SHA calls (1,000,000). Repeated citations,
  original spans and moved candidates all count. Successful reads retain
  content+1 and refund unused reservation; failed reads retain the reservation.
  Exhaustion refuses the operation without partial states. Membership preflight,
  checked offsets, current-byte semantics and the trusted pure helper remain.
- Synthetic regressions cover exact/cap+1, UTF8/unknown fields, native framing,
  buffered overflow tails, all-five-tool admission, zero spawn/CURRENT/load
  counters on refusal, cancellation, runtime shutdown, cold reload contention,
  cache reuse, poison recovery, repeated/midbatch/tiny-hash budget exhaustion.

## Reviews And Scope

Read-only reviewers requested and effectively used GPT-6-Astra:

- `01a1c720-b688-73f2-b973-f56c4f7585cb`, results `/2`, `/3`: citation
  budgeting had no scoped defect. CLI post-conversion validation finding was
  corrected with borrowed JSON preflight; final CLI delta had no new finding.
- `01a1c720-b667-75e0-8010-faa8256ff7e1`, results `/2`, `/3`: corrected
  tool-name accounting, stale exact-cap assertion and missing explicit
  counter/shutdown evidence. Final MCP/shared-helper delta had no scoped blocker.

Workers `01a1c724-8184-7c52-a416-6d4b3a1628c2` (cite only) and
`01a1c724-f577-70f1-aeff-0c62d6b39c5f` (mcp only) completed disjoint writes;
parent integrated the shared CLI/protocol checks. Children ran no compiler/test
commands. All children are closed. Review is source evidence, not runtime proof.

Pre-edit inventory `/tmp/ontodoc-od35-baseline-61y84o0u.json` and final inventory
`/tmp/ontodoc-od35-final-source.json`, receipt `eb69ea`, identify four changed
implementation files: `src/cite.rs`, `src/cli.rs`, `src/mcp.rs`, `tests/mcp.rs`.
`src/store.rs` is unchanged. AGENTS.md also changed during this interval outside
these agents' writes; attribution is unverified. Its OntoIndex counts currently
differ from supplied instructions. Preserve that unrelated delta; it is not
claimed as task output. Final hashes:

```text
src/cite.rs f27e52a0f5787fa4267d49d558ed4e8f64920d32e638cb785db7f581365c5cc3
src/cli.rs 5c661ff3d7a29e0b73ecb50cecb82506c8d207f57ec0f469aa067bc81b7ca82f
src/mcp.rs bc0bfc7f1dbed246372ccfac34cc4a6ea7e169dff72c9007fcb9cc37282383c1
tests/mcp.rs ef3da0de950289c06751a3c6cfcf9fc6b0b594b342fdc24491f344e93f48dc00
```

OntoIndex navigation/impact and final verify_diff refused unknown indexed commit
identity; source inspection is authority. No graph PASS or manual reindex is
claimed. Changes to the shared request boundary were treated as high risk.
The checkout has no initial commit, so an empty Git diff cannot establish scope.
Manager-owned closeout updates audit/plan/status/pending/memory/architecture only.

## Executed Validation

Native repository root, Rust 1.95.0, serial managed compiler lane under
`/opt/d2/.ontocode/artifacts/compiler-lane/`. Each job was harvested by the
canonical result reader; startup was not treated as success. All Cargo commands
use `CARGO_BUILD_JOBS=4`. Commands within each job use serial `&&`.

Job `ontodoc-od35-focused-20261110-a`, receipt `358745`, reharvest `eb69ea`,
terminal **101**:

```sh
CARGO_BUILD_JOBS=4 cargo test --lib cite::
CARGO_BUILD_JOBS=4 cargo test --lib mcp::
CARGO_BUILD_JOBS=4 cargo test --lib cli::
CARGO_BUILD_JOBS=4 cargo test --test mcp
```

Citation13, MCP12 and App5 passed. MCP integration: 10 passed, one failed.
The exact64KiB argument fixture exposed production accounting of tool-name
bytes. Corrected the counter to cover arguments only; no limit was weakened.

Job `ontodoc-od35-validation-20261110-b`, receipt `bcb33f`, terminal **101**,
started with `CARGO_BUILD_JOBS=4 cargo test --lib mcp::`: 11 passed, one failed.
The now-stale unit assertion expected an exact-cap key to refuse. Corrected it
to accept exact cap and reject cap+1. Later cli/MCP/CLI/eval/lint/format commands
in this failed chain were skipped. This was distinct from job a's wire failure.

Job `ontodoc-od35-validation-20261110-c`, receipt `61891b`, terminal **101**:

```sh
CARGO_BUILD_JOBS=4 cargo test --lib cite::
CARGO_BUILD_JOBS=4 cargo test --lib mcp::
CARGO_BUILD_JOBS=4 cargo test --lib cli::
CARGO_BUILD_JOBS=4 cargo test --test mcp
CARGO_BUILD_JOBS=4 cargo test --test cli
CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

All tests passed before Clippy's `int_plus_one` warning in tests/mcp.rs.
Visible integration totals: MCP11, CLI7, gates14/run6. Prefix truncation hides
the unit totals; do not invent them. Replaced the test's `len()+1 <= cap`
comparison with equivalent `len() < cap`. Formatting in this chain was skipped.

Job `ontodoc-od35-lint-20261110-d`, receipt `ac758a`, terminal **0**:

```sh
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

Every command passed. No behavior changed after job c except the equivalent
test comparison and formatter output. Each distinct failure was corrected once;
no repeated failure or rejected manager receipt was retried.

No full-library/workspace suite, real held-out run, HTTP test or performance/RSS
benchmark was executed. Passing counters/unit tests establish neither retrieval
improvement nor global memory/latency/multi-user readiness. Measurement remains
disabled. Admission is one server/process as constructed by the stdio entry
point, not a global limit across arbitrary library-created servers.

## Remaining Boundaries

OD-3.9 owns publication commit outcomes and deterministic investigation of the
historical parallel store Locked failure. HTTP-0 remains owner-gated; evaluation
provenance/annotation gates remain. No schema, ranking, dependency, donor or
evaluation payload changed. Strict plan checks and complete-local control task
closure, not this prose.

## Closeout

Strict `complete-local` completed OD-3.5 and selected OD-3.9. Post-tool revision:
`88bed492e26754d42725de2d6bf13987c48a76ffe4d18f54fad6e8ce50df66ad`.
Counts: 27 DONE, 10 OPEN, 2 BLOCKED /39; manager remains in progress.
An initial call omitted expected_revision and was rejected before mutation;
one corrected call supplied the current hash. No stale revision or role receipt
was bypassed. Plan validation passed all 39 tasks before closure. The tracker is
98% of its parser ceiling; future additions need compact evidence or authorized
archival rather than deleting authenticated history.

Final plan validation passed 39 tasks; strict routing selected dependency-ready
OD-3.9. Reference/whitespace, MEMORY94-line cap, final source identity/scope and
`git diff --check` checks exited 0 (receipt `68a804`). No compiler jobs remain.
