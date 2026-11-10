# EVAL-1.5: Frozen Registration And Attempt Identity

Date: 2026-11-10
Status: DONE through strict complete-local; EVAL-1.3 selected next.
Authority: EVAL-1.5 explicitly requires resolution of the frozen-location conflict.

## Decision Before Implementation

ADR revision 6 selects one write-once slot under Git's common directory.
Compared alternatives: keeping `eval/frozen-config.toml` in the matching clean
commit is self-referential; ignoring generated in-tree files needs dirty-state
exceptions. An external export alone leaves attempt custody relocatable with
the benchmark. A fixed Git-common-dir slot reuses local exclusive/fsync files,
shares custody across worktrees, and requires no daemon, dependency or command.
Only one registration is supported; superseding it is outside this task.

Pre-change source: `src/cli.rs` freeze rejects in-checkout output;
`src/eval/run.rs` reads the freeze beside the benchmark and keys only a started
marker there.
Saved TSV scoring already hashes and parses identical captured bytes, but its
editable metadata lacks authoritative completion binding. Full public success
paths remain EVAL-1.3 work, including provenance and held-out capture admission.

OntoIndex stale/degraded: workspace build failed; committed impact refused
unknown indexed commit identity. Source is authority; no graph-risk PASS.
Requested/effective GPT-6-Astra explorer `eval15_trace`
(agent `01a1c694-91e5-76c1-bae8-387b503d5aa5`) challenged the options before
implementation. Its result `/1` recommends the fixed single slot; `/2` found
external-audit/config-selection/reproducibility-scope issues, all addressed.
Result `/4` found a metadata reread between generation and scoring. Applied its
minimal fix: pass constructed metadata directly to `score_captured` and delete
the sole-use `score_inner`; the regression replaces the staged metadata file
without changing the scored captured value. Result `/5` confirms that finding
closed, with no blockers in the requested correction; static review only.
No real held-out labels, queries or results were inspected.

Manager documentation scope is recorded separately from the coder write set
in plan Decisions And Non-Goals; its structured validator rejects manager docs
in worker write sets. An initial attempted mixed scope failed validation, was
corrected once, and the separated scope passed. It covers this decision, ADR,
routers and downstream command bindings. No active dispatch lease exists; task status remains OPEN.

## Validation

Native root package, Rust 1.95, `CARGO_BUILD_JOBS=4`; no donor build or real eval.
All jobs below are managed direct children under
`/opt/d2/.ontocode/artifacts/compiler-lane/`, harvested through the canonical
`compiler_lane_detached.py --result JOB --wait 60` reader.

- `ontodoc-eval15-tests-20261110a`: required `cargo test --test eval_gates
  --test eval_run` passed; following `cargo test --lib` exited 101 (110 pass,
  2 fail). New custody fixture compared a canonical path to a `/tmp` symlink
  spelling; fixed by canonicalizing its base. Unchanged store pruning returned
  `Locked` in parallel. No store source was changed; serial rerun passed, so
  this is recorded as a parallel-sensitive unresolved validation limitation,
  not proof of a repaired store root cause. Receipt `b787eb`, tail truncated.
- `ontodoc-eval15-final-20261110a`: `cargo fmt && cargo fmt --check && cargo
  test --test eval_gates --test eval_run && cargo test --lib -- --test-threads=1
  && cargo clippy --all-targets -- -D warnings`, exit 0. Library: 112 passed;
  all earlier steps passed by the chained command. Receipt `404d69`, prefix
  truncated; integration counts are not inferred from that tail.
- `ontodoc-eval15-protocol-20261110a`: `cargo test --test cli --test mcp`,
  exit 0: 5 CLI + 7 MCP. Receipt `10215d`, untruncated.
- `ontodoc-eval15-api-20261110a`: after extending the same synthetic test with
  real clean-Git code identity, `cargo fmt && cargo fmt --check && cargo test
  --lib eval::run::tests:: -- --test-threads=1 && cargo clippy --all-targets
  -- -D warnings`, exit 0: 13 focused tests. Receipt `130140`, untruncated.

Five new unit regressions cover linked-worktree resolution, concurrent exclusive
start, interruption/failed/partial occupancy, exact completed metadata/report
and registration commitments, unsafe/oversized custody, captured identity and
raw export mismatch. Real registration/check-export APIs run against synthetic
code and benchmark bytes; public guards stay in force. Integration assertions
also confirm disabled commands do not create custody files. This is not a full
held-out success path; EVAL-1.3 must connect selected test capture, preparation
counts, provenance/admission and all callers before lifting any refusal.

Implementation worker `eval15_impl` changed only run.rs; parent changed CLI,
integration assertions and authorized manager docs. No dependency, ranking,
MCP transport, ID/schema, donor or real benchmark artifact changed.

- Final reviewed-source job `ontodoc-eval15-reviewed-20261110a`: `cargo fmt &&
  cargo fmt --check && cargo test --test eval_gates --test eval_run && cargo test
  --lib -- --test-threads=1 && cargo clippy --all-targets -- -D warnings`, exit 0,
  library112. Receipt `c2908f` (reader session48774; prefix truncated). No further
  code edits. Required integration suites passed in this same chained run.

Strict `complete-local` completed EVAL-1.5 and selected EVAL-1.3, revision
`88f81eb955069af7536b8c2704acd6a4de8ba92313470b91d2db5509747b3116`.
Source SHA-256: run.rs `a50c2a66f019cbc3ca85a72a1fba212223c3398159031175bf4b7d88ebe8896b`;
cli.rs `c24d405e3bff6e3db300b8e87b739ce72522e8d0d5748fc3deb033e2440d41eb`.
OntoIndex verify_diff also refused unknown indexed commit identity; no graph
verification claimed. No initial commit exists, so exact source inventories
provide scope evidence instead of an empty Git diff. All five compiler jobs
were harvested terminal; children closed.
