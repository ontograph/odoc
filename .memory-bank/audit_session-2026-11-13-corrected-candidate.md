# Corrected candidate authorization and execution

The user replied `proceed` after the recommendation to preserve the measured binary and create a separate corrected candidate. This authorizes restoring the existing ADR transport-parity contract, the selected bounded performance repairs, validation and local packaging under `./`. It does not authorize a held-out rerun or publication to an unspecified destination. No semantic ADR exception is needed: implementation will conform to the accepted contract.

The original candidate F, archive, evaluation registration and consumed attempt remain unchanged. New source/binary/package identities must be recorded separately. Existing evaluation results describe the original measured binary; they are not measurements of the corrected candidate. Public destination, repository/history exposure and embedded build-path decisions remain open.

## Preparation

CORR-0 preserved the exact pre-compaction tracker in `archive/corrected-candidate-preparation-cards.md`, then replaced verbose completed-card descriptions with references. A structural Python comparison verified all status, dependencies, owner/write sets, closeout and runtime fields unchanged (exit 0, receipt `0f05e0`). Strict plan validation passed; tracker shrank by 11,473 bytes. One missing-field/active-task validation correction was made before work. No receipt or completed task was recreated.

## Execution evidence

GPT source exploration uses OntoIndex plus current source. Only synthetic inputs are allowed for regressions and performance checks. Native root-package Cargo commands use four jobs; the external compiler-lane wrapper provides orchestration only and never builds the donor snapshot.

### CORR-1

HTTP uses the native rmcp request parser and shared exact-ID cap error. Its complete-body limit reserves the same newline byte as stdio. IDs whose refusal cannot fit receive 413 before dispatch. Existing tracked-task permit ownership and generic transport errors remain. GPT source challenge found no production defect; native notification handling was verified by runtime tests. OntoIndex omitted dirty files, so current source was authoritative.

Compiler receipts under `/opt/d2/.ontocode/artifacts/compiler-lane/`: `ontodoc-corr1-validation-20261113-a` exited 101 (new test assumed 400; native notification returned 202). Corrected native classification is 202 or 415. `ontodoc-corr1-validation-20261113-b` passed `CARGO_BUILD_JOBS=4 cargo test --test http` (6) and `--test mcp` (12), then exited 101 compiling a new unit fixture with an unsupported integer conversion. Explicit `NumberOrString::Number` corrected it. `ontodoc-corr1-validation-20261113-c` exited 0: `CARGO_BUILD_JOBS=4 cargo test --lib mcp::` (17), `--lib http::` (5), `cargo fmt --check`, and `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`. Two distinct fixture corrections; no repeated unresolved failure. Production files: `src/http.rs`, `src/mcp.rs`; regressions: `tests/http.rs`, `tests/mcp.rs`. No held-out run, publication or original artifact edit.

### CORR-3

Filtered graph events use binary search after once-per-document span validation; arbitrary unsorted/overlapping inputs retain first-match linear behavior. Heading paths use a stack once per section, bounded by the public u8 level domain. Complete old-reference graph and tier outputs match, including malformed public input compatibility. Parent reviewed the GPT patch; OntoIndex committed impact LOW had unverified provenance.

`ontodoc-corr3-validation-20261113-a` exited 0: `CARGO_BUILD_JOBS=4 cargo test --lib graph::` (7), `--lib tiers::` (6), `--lib synthetic_scan_timing -- --ignored --nocapture` (2), `--lib store::tests::canonical_bytes_match_independent_roots_and_explicitly_different_mtimes` (1), `cargo fmt --check`, and four-job all-target Clippy with `-D warnings`. Single debug timing observations on 1,000 flat headings: graph old 23.149 ms / new 10.433 ms; tiers old 7.466 ms / new 5.386 ms. These are smoke observations without a performance acceptance verdict. Only `src/graph.rs` and `src/tiers.rs` changed.

### CORR-2

`cli::json_fits` uses a fixed-storage writer and continues serialization after overflow to preserve genuine late serde errors. CLI truncation order and bytes match the old allocating reference; MCP reuses the helper with its existing false-on-error behavior. Parent inspected the GPT implementation and regressions. Committed OntoIndex impact reported MEDIUM for `bound`, LOW for MCP; unverified provenance makes current source authoritative.

`ontodoc-corr2-validation-20261113-a` exited 0: `CARGO_BUILD_JOBS=4 cargo test --lib cli::` (11), `--lib mcp::` (18), `cargo fmt --check`, and `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`. Thresholds, Unicode/control escapes, errors before/after overflow including custom FileTooLarge, paragraph truncation and indivisible metadata passed. Only `src/cli.rs` and `src/mcp.rs` changed. Temporary serialization buffers are removed; no measured speedup is claimed.

### CORR-4 validation interruption

`ontodoc-corr4-tests-20261113-a`: `CARGO_BUILD_JOBS=4 cargo test --all-targets` exited 101. Library results: 224 passed, 1 failed, 3 ignored; integration tests were not reached. `store::tests::process_publication_holds_lock_through_commit_and_maintenance` failed at `src/store.rs:1833`: publication to its separate temporary corpus returned `Locked` while the main corpus was deliberately locked. No unexpected-holder identity was captured.

`ontodoc-corr4-lock-proof-20261113-a`: `CARGO_BUILD_JOBS=4 cargo test --lib store::tests::process_publication_holds_lock_through_commit_and_maintenance -- --exact --nocapture --test-threads=1` exited 0 (one test). `ontodoc-corr4-tests-serial-20261113-a`: `CARGO_BUILD_JOBS=4 cargo test --all-targets -- --test-threads=1` exited 0; the terminal receipt was harvested in this session. Serial success does not replace the failed required parallel check. The final candidate Clippy lane, release build and real packaging have not run.

GPT read-only diagnosis traced publication through `Store::publish_inner` to `Store::lock`. Graph/tiers run before locking; scheduling influence is possible, but a regression in those repairs is unproven. A concurrently spawned process retaining an inherited open-file-description lock is a hypothesis supported by Linux descriptor semantics, not an identified owner. CORR-4 packaging pauses under its validation-failure stop condition. A separately scoped store correction requires a deterministic regression and preservation of fail-fast contention and lock ownership through maintenance; serializing the suite alone is not a repair.

Package/smoke helper scripts were prepared under `tmp/release-010/corrected/`; no corrected binary/archive or source capture exists yet. The package helper's six synthetic selftests passed in the GPT worker receipt `65bd84`. Those tests validate the helper, not the candidate. Binary smoke remains unexecuted.

### CORR-LOCK prerequisite

The existing authorization to fix reviewed issues covers a separately registered `src/store.rs` repair. Alternatives considered: serialize tests (does not fix lock lifetime), retry contention (changes fail-fast semantics), or private explicit-unlock guard (selected). OntoIndex reported HIGH upstream impact around publication; the user was warned before edits. Current source confirms one production acquisition whose scope spans commit and maintenance. A guard changes release only, with best-effort unlock and close fallback; it adds no API, dependency, retry or schema change.

Before production edits, the retained-duplicate regression ran via `ontodoc-corrlock-red-20261113-a`: `CARGO_BUILD_JOBS=4 cargo test --lib store::tests::held_lock_blocks_and_released_lock_is_reclaimed -- --exact` exited 101 at successor acquisition, returning `Locked` after logical-owner drop. This deterministically demonstrates the descriptor-lifetime mechanism, without identifying the holder from the earlier parallel failure. One task-routing correction and one YAML command-quoting correction were made while registering the prerequisite; no authenticated state was replaced.

`ontodoc-corrlock-green-20261113-a` exited 0 for the exact regression, `CARGO_BUILD_JOBS=4 cargo test --lib store::`, `CARGO_BUILD_JOBS=4 cargo test --all-targets` with default parallelism, and `cargo fmt --check`. `ontodoc-corrlock-clippy-20261113-a` exited 0 for all-target Clippy with `-D warnings`; `git diff --check` exited 0. Parent reviewed the patch and a separate GPT reviewer found no actionable defect. CORR-LOCK closed through the strict manager; CORR-4 resumed. Only `src/store.rs` changed. Earlier failed receipts remain evidence; original incident attribution remains unresolved.

### Corrected package

Source capture (`build-corrected.py capture --root . --out tmp/release-010/corrected`) exited 0, manifest SHA-256 `7bb1ee28ccf8cf4a1e0466d97060ea4c7f79c0d2386981362cffd6ca51669619`. Its exact inventory includes source, tests, Cargo and toolchain; base HEAD is not the candidate identity. `ontodoc-corrected-clippy-20261113-a` exited 0 for `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`. `ontodoc-corrected-release-20261113-a` exited 0 for `CARGO_BUILD_JOBS=4 cargo build --release --locked`. Parent reran six package-helper tests (exit 0). Source capture verification passed again after packaging.

`build-corrected.py package --root . --out tmp/release-010/corrected` exited 0 (receipt `56b15d`): deterministic archive bytes compared twice, exact member checks, extracted-binary hash, checksums, version and synthetic install/index/search passed; disposable staging removed. Corrected binary SHA-256 `2fc26f1527ce59842e135ef47fd80d950541bd6389fa360c43f987dd8684c2a3`; binary archive `daccd66812a83aa3082a9a29b38fd7be7a91d63cdaedf9ad367e0bd394b64725`; source archive `b9b60300c3a34f09d50e9d98951e036a660fa1cf1767a4c320bb3a4cbe4bb6d6`. Archive member metadata/hashes and installation commands are in `package-manifest.json` and `install-smoke.json` alongside the archives.

Native smoke used only the hash-verified binary extracted from the archive. First attempt `smoke/` exited 1 due to the helper's local `http` name shadowing its module; direct `HTTPConnection` import corrected it. Second attempt `smoke-retry/` exited 1 because the large fixture assumed a search hit despite response trimming; the direct known-section read corrected that distinct harness assumption. Both retained reports confirm 30 exact artifact comparisons and cleanup; the second additionally completed five-tool old/new transport parity and exact ASCII/escaped cap errors. Failed runs are retained, not represented as complete passes.

Original binary/archive hashes still match their preserved identities. All three Git-common custody hashes match the accepted held-out audit; only filenames and digests were read, with no evaluation rerun. Corrected binary contains 317 `/home/` byte-prefix occurrences, a disclosure count rather than a leak-free verdict. Publication awaits destination/exposure and embedded-path decisions. OntoIndex file-scope verification passed after correcting an omitted `expectedFiles` argument; scan capped at 25 symbols and symbol-level scope was not checked. Current source and runtime receipts remain authoritative.

Final `smoke-final/` exited 0 (receipt `4a7b75`): 30 exact CURRENT/meta/index comparisons across five synthetic corpora, old/new binaries and three repetitions; all five tools match across both transports; corrected ASCII/escaped cap errors match; six full-section truncated responses match. All 42 subprocess statuses are recorded. HTTP wrappers were deliberately terminated with SIGTERM (exit -15), not asserted as graceful service exits. Both inner corpus staging and outer extracted-binary staging were removed; children reaped. `validation.json` binds package, source, smoke and helper hashes.

Three fresh-process repetitions, median wall seconds old/new: flat1000 0.04/0.03; nested1000 0.04/0.04; dense3000 0.07/0.05; large indexing 0.04/0.04; small sample 0.02/0.03; large-response 0.12/0.13. Dense3000 median RSS 14,684/14,956 KiB; large-response 13,772/13,668 KiB. Complete raw repetitions are in `timing-summary.json` and GNU time files. Resolution is 0.01 seconds with startup included; mixed results and n=3 support no general speedup or capacity verdict. No artifact equivalence or quality evidence was inferred from timings.
