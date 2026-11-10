# OD-3.6: Streaming Freshness And Load Admission

Date: 2026-11-10
Authority: service ADR revision 8; user proceed; strict manager selected
dependency-ready OD-3.6 for local execution, with no dispatch lease.

## Implemented

- `src/corpus.rs`: one accepted-file visitor owns traversal/policy. Build's
  collecting wrapper remains; freshness no longer retains source texts across
  files. Each visit clamps source/file limits and its accessible byte/probe
  window, restoring excluded shared allowance on success and error. Source and
  control reads reserve cap+1 before reading, retain content+1 after success,
  and retain the full reservation after failure. Shared path/line/matching
  counters and accepted-file skip semantics remain intact.
- `src/store.rs`: freshness keeps only path/size/hash, globally sorts that
  manifest and preserves added/removed/modified comparisons, including same-size
  changes. Shared verified decoding performs allocation-light raw collection
  counting before typed records. All encountered duplicate collection keys
  count toward configured/global caps; excess elements refuse before decoding
  their contents. Existing configuration, schema, checksum and artifact bounds
  remain in force for normal, rooted and captured loaders.
- `src/search/mod.rs`: checked actual heading/LF/span/pseudo-document input
  admission precedes joins, tokenization and BM25 work. Overlapping references
  count repeatedly; arithmetic overflow refuses. One borrowed document map is
  reused for validation/construction, preserving first-document span text and
  last-document hierarchy mapping on previously accepted duplicate paths.
- `src/cli.rs`: service construction uses `SearchIndex::try_new`. Shared decoder
  validation also covers evaluation's unchanged captured-decode constructor
  path. Public infallible `new` remains an explicitly trusted compatibility API
  for manually constructed generations; universal library admission is not claimed.
- Inline and CLI regressions cover policy/manifest parity, callback sequencing,
  reservation-before-read injection, residual/error budgets, exact/cap+1 raw
  collection counts, duplicate keys, key ordering, load adapters, tokenizer
  bytes/overlap/overflow, exact old/new BM25 and B1/B2/B3 parity, sanitized CLI
  refusal and unchanged CURRENT.

## Review And Scope

Read-only reviewers requested and effectively used GPT-6-Astra:

- `01a1c712-2482-7690-bd25-15af775068be`, result `/2`: corpus/freshness
  delta, no scoped blocker. Callback testing proves sequencing; text-release
  assurance also depends on ownership/source inspection, not measured RSS.
- `01a1c711-fa9d-7810-bcbf-ddcc171dd202`, results `/2` and `/3`:
  decoder/construction and final fixture correction, no remaining source blocker.

Workers `01a1c715-7bc9-7481-8985-09978ed89991` and
`01a1c715-1b2e-70e2-8b88-24aac2544cbf`, result `/1` each, wrote only
corpus.rs and search/mod.rs respectively. Parent integrated and validated.
Children ran no compiler/test/formatter commands; all agents are closed.

Pre-edit inventory `/tmp/ontodoc-od36-baseline-0b__bsyd.json` compared with final
source confirms exactly the five allowed implementation paths: corpus/store/cli,
search/mod.rs and tests/cli.rs. Receipt `fcc612` records their hashes and
`/tmp/ontodoc-od36-final-source.json`. No dependencies, schema, ranking parameters,
donor files, HTTP or evaluation payloads changed. Manager-owned closeout adds
this audit and updates plan/status/pending/memory/architecture only.

OntoIndex committed navigation/impact refused unknown indexed commit identity;
source inspection is authority. No unchanged query retry, manual reindex or
graph PASS is claimed. Shared freshness/load changes were treated as high risk.
The checkout has no initial commit; empty Git diff is not scope proof.

## Executed Validation

Native repository root, Rust 1.95.0, serial managed compiler lane under
`/opt/d2/.ontocode/artifacts/compiler-lane/`. All Cargo commands use
`CARGO_BUILD_JOBS=4`. Both jobs were harvested through the canonical result
reader; launcher success was not treated as validation success.

Job `ontodoc-od36-focused-20261110-a`, receipt `05a5a8`, terminal exit **101**:

```sh
CARGO_BUILD_JOBS=4 cargo test --lib search::
CARGO_BUILD_JOBS=4 cargo test --lib corpus::
CARGO_BUILD_JOBS=4 cargo test --lib store::
CARGO_BUILD_JOBS=4 cargo test --test cli
```

Search and corpus passed before the store fixture failed (corpus 28 visible;
prefix truncated, no invented search total). Store: 26 passed, one failed.
The new adapter fixture serialized a sorted JSON Value, encountering documents
before files and correctly refusing the document count instead of the expected
file count. Correction serializes a Generation clone to preserve files-first
field order; production behavior is unchanged. The final CLI command was skipped
by this failed chain, not reported as passing.

Job `ontodoc-od36-validation-20261110-b`, receipt `77b3dd`, terminal exit **0**:

```sh
CARGO_BUILD_JOBS=4 cargo test --lib store::
CARGO_BUILD_JOBS=4 cargo test --test cli
CARGO_BUILD_JOBS=4 cargo test --lib cli::
CARGO_BUILD_JOBS=4 cargo test --test mcp
CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

Every command passed in the serial && chain. Visible totals: CLI7, App4, MCP9,
gates14, run6; the store prefix was truncated. After job a, only the fixture
correction and formatter output changed. No full-library/workspace suite, real
held-out run, latency/throughput/RSS benchmark or HTTP validation was executed.
Passing counters/parity tests do not establish retrieval improvement, decoded
memory accounting or hostile-administrator containment. Measurement remains disabled.

## Remaining Boundaries

OD-3.5 owns raw/decoded input, aggregate verification hashing and actual-work
admission/cancellation; OD-3.9 owns publication commit outcomes and deterministic
investigation of historical parallel store Locked failures. Neither is repaired
by these results. HTTP-0 remains owner-gated. No TTL/watcher substitutes for
current-byte freshness, new tool or network listener was introduced.

Strict plan/document checks and complete-local control closure, not this prose.

## Closeout

Strict `complete-local` completed OD-3.6 with OD-3.5 selected next. Post-tool
tracking revision: `2c63d51f6ddeccc5906e76f5439865f90f9a3925f1b55cf0b770b15ca112840b`.
Counts: 26 DONE, 11 OPEN, 2 BLOCKED / 39; manager remains in progress.
Plan validation passed 39 tasks with no errors; document/reference/whitespace
checks, final source identity and `git diff --check` exited 0, receipt `5490bd`.
The plan is near its parser ceiling (96%); future tracking additions need
bounded evidence or authorized archival, never deletion of authenticated history.
