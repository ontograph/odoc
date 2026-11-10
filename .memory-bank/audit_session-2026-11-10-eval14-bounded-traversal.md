# EVAL-1.4 Bounded Traversal

Date: 2026-11-10
Authority: service ADR revision 5; active EVAL-1.4 task.
Status: implementation and validation complete; strict task closeout recorded
in the canonical plan. Evaluation remains disabled.

## Selected Seam And Conflict Disposition

Use standard directory enumeration in the existing corpus owner and public
`ignore::gitignore` matchers. Keep the ordinary walker unchanged as the oracle
for successful regular-control fixtures; share source decoding and output
sorting. Do not implement glob semantics or another eval-local corpus policy.

Account before enumeration, including ignored/hidden/non-Markdown entries.
Bound directory collection before sorting and descent before filesystem work.
Capture ignore bytes under the shared byte budget before public `add_line` and
`build`, never automatic ignore-file loaders. Match custom controls across the
ancestor stack before git controls; apply hidden filtering only without a
matching ignore/whitelist rule. Prune excluded parents. No `.git` probing is
needed with the configured `require_git(false)`.

Ceilings: depth 64, ignore file 64 KiB, line 4 KiB, aggregate physical lines
4,096, enumeration 100,000, cumulative path-content accounting 16 MiB, and
1,000,000 matching-work units. A matcher visit costs `max(1, matcher.len())`
before calling it. This is a work counter, not a wall-time guarantee. Exhaustion
and unsafe/unreadable controls fail capture explicitly, never partial success.
Path reservations conservatively include retained basename/result and matcher
provenance/root copies; they do not measure allocator/collection overhead.
Syntax/decoding behavior retains the ordinary matcher's valid-prefix policy.

ADR §2 forbids following symlinks. Installed `ignore` automatic control loading
uses `exists()`/`File::open`, which can follow symlinked controls, contrary to
that authority. For this isolated new capture flow, conform to the ADR by
refusing symlink/nonregular controls before use; do not preserve the violation
for compatibility. Successful parity covers successfully readable regular
controls. Unlike the ordinary loader's ignored control I/O failures, unreadable
controls fail capture explicitly. The ordinary
walker's violation and race-safe ancestor containment remain unresolved service
repairs under OD-3.4/3.6. No ADR amendment, schema change, dependency addition or
global containment acceptance follows.

GPT-6-Astra read-only explorer `bounded_walk_design` reviewed the seam and
explicitly accepted this scoped conformance disposition (result `/2`, agent
`01a1c682-3937-7650-b2db-c0ed29d7ecfd`). Source receipts `339855`, `f2e4e9`,
`35a593` exit 0 establish static findings, not runtime parity. No child changes
or validation. Parent source confirms ADR §2, corpus configuration and installed
control loading. OntoIndex returned `workspace_busy`; committed-symbol impact
also refused unknown indexed commit identity. Source inspection is authority;
no graph risk/scope PASS is claimed. Shared traversal has high impact.

## Planned Acceptance

Synthetic parity cases: custom/git precedence across depths, last-rule wins,
negation, anchored/directory patterns, hidden whitelists, excluded parents,
nested `.git`, inactive parent/global/`.ignore` policies, BOM/CRLF, malformed
rules, invalid UTF-8 prefix and ordinary source skip diagnostics.

Exhaustion cases: ignored-entry floods, oversized/long/many ignore rules,
depth/path/matching budget overflow and symlink/FIFO controls. Controls in pruned
directories must not be opened. Captured eval admission must propagate refusal.
Run focused corpus/capture/store and required eval integration checks, all-target
Clippy and formatting. Measurement guards and human/provenance gates remain.

## Implementation And Review

`src/corpus.rs` now owns bounded directory enumeration and explicit control-byte
capture. Shared source collection/sorting is reused by both paths; the ordinary
automatic ignore traversal is unchanged. Shared Markdown opens use Linux
final-component no-follow/nonblocking flags and check descriptor file type,
preventing substituted FIFOs from blocking before inspection. This does not
prove ancestor containment. `src/eval/run.rs` constructs a shared walk budget
for captured corpus freshness and tests exact refusal propagation.

Synthetic parity covers custom/git precedence, nested negation, direct hidden
files/directories, parent pruning, nested `.git`, inactive parent/`.ignore`
policies, BOM/CRLF, escapes, invalid glob continuation and invalid UTF-8 prefix.
Exhaustion tests cover ignored/hidden entry floods, control byte/line/count caps,
depth/path/matching units and unsafe symlink/FIFO controls. Pruned directories'
oversized controls are not opened. Dev admission rejects oversized and symlink
controls with exact typed errors. Existing capture regressions cover held-out
isolation, identity tampering, stale sources, cached indexes and saved-byte
scoring; see the [partial capture audit](audit_session-2026-11-10-eval14-input-capture.md).

GPT-6-Astra source challenge `/3` requested conservative path-copy accounting,
a truly malformed glob and direct hidden-file coverage, and explicit unreadable
control refusal documentation. Those corrections were applied. Final delta
review `/4` found no remaining source-level blocker; it ran no validation and
does not approve HTTP, ancestor containment or public measurement. Parent
validation, not the review, establishes the test outcomes below. The reviewer
used receipts `cbdd16` (exit 1: no matching test search) and `12fdee` (exit 0:
located the owner-local test); neither is a test execution claim.

## Validation

Native root package, Rust 1.95.0, `CARGO_BUILD_JOBS=4`. Managed jobs under
`/opt/d2/.ontocode/artifacts/compiler-lane/` were harvested through
`python3 -B /opt/d1/_wrk/ontocode/scripts/compiler_lane_detached.py --result JOB --wait 60`.
Each terminal reader exited 0; command outcomes are the nested exit codes:

| Job (`ontodoc-eval14-` prefix) | Commands | Outcome |
| --- | --- | --- |
| `traversal-tests-20261110a` | `cargo test --lib corpus::tests` | exit 0; 11 initial corpus tests |
| `traversal-integration-20261110a` | `cargo test --lib corpus::tests && cargo test --lib eval::run::tests && cargo test --lib store::tests && cargo test --test eval_gates --test eval_run` | exit 0; bounded tail displays 8 capture, 17 store, 14 gates, 6 run tests; initial corpus summary truncated |
| `traversal-final-20261110a` | `cargo test --lib && cargo test --test eval_gates --test eval_run && cargo clippy --all-targets -- -D warnings` | exit 0; 107 library, 14 gates, 6 run tests and Clippy; summaries visible despite truncated prefix |
| `traversal-protocol-20261110a` | `cargo test --test cli --test mcp` | exit 0; 5 CLI, 7 MCP tests; untruncated |

Final coverage: **139 tests passed**, no failures. Canonical reader receipts
`52ceca` and `bf9ebe` substantiate the final suites/lint. No benchmark, installation
smoke, HTTP or multi-user performance measurement was run.

An initial `cargo fmt --check` exited 1 for formatting differences. After final
tests/Clippy, `cargo fmt && cargo fmt --check` and `git diff --check` exited 0
(receipt `f8eddf`). Only formatting followed those suites; no tests were rerun
after it. Post-format source SHA-256:

- `src/corpus.rs`: `2c79e893013a33dc555995706630f8c946d396ddcaf99bf76589473776876247`.
- `src/eval/run.rs`: `8346655e8bf57bede3fcf96c2b53f33362cf9bec87f840cfc21d3dca043cb314`.

## Acceptance And Limits

The prior audit's ignore-internal traversal gap is closed for opt-in eval
capture by the explicit corpus-owned adapter. Payload/index bytes are captured
and identity-checked once per admission; consumers use those values. Mechanical
evidence is in-process only; no annotation provenance or durable freeze/attempt
custody is claimed. EVAL-1.5 and EVAL-1.3 retain those gates.

Only `src/corpus.rs` and `src/eval/run.rs` were edited as code during this continuation;
earlier EVAL-1.4 store/gates/sweep/test changes remain. Dependencies, IDs/schema,
ranking, CLI/MCP owners and donor/real evaluation data are unchanged. The
checkout has no initial commit, so file hashes rather than an empty Git diff
establish scope. No commit or reindex was created.

Final scope inspection also observed an external `AGENTS.md` hash change from
`3932beb26dc784b8918f79ce5f649620d6656b92d2c4d670cd94380387063e53` to
`8448d88d5e95c56ae2a5dadfdb26d9ad2efd9ce64bad6e10b41ae5e51de111ec`.
Its generated OntoIndex block now advertises 11,323 symbols and 27,216 relations.
Neither parent nor reviewer edited that file; origin is not independently
established and the change is preserved, not claimed as task-owned work. The
initial whole-inventory scope check flagged this difference (receipt `ca1deb`);
the corrected check separates this observed external change from owned writes.

Numerical counters bound logical work/input sizes, not filesystem latency or
allocator overhead. Ordinary control-file symlink behavior, descriptor-rooted
ancestor containment, service-wide admission/cancellation, store/freshness
streaming and inbound HTTP bounds remain OD-3.4/3.5/3.6/HTTP work. Public
measurement refusals, HTTP-0 owner gate and EVAL-2.0 annotation prerequisites
remain intact. No retrieval-quality or throughput improvement is asserted.

Strict `complete-local` completed EVAL-1.4 and selected EVAL-1.5 at revision
`ca58d128ad3382dd4c29e2b2f6d1dbdde7709eecbb0c8e4f0438ed918403600f`.
Counts: 21 DONE, 12 OPEN, 2 BLOCKED of 35; manager remains in-progress. Earlier
partial audit is preserved as history, not current task status.
