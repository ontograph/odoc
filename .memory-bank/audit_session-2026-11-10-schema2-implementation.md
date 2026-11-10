# OD-3.2: Schema-2 Identity And Canonical Artifacts

Date: 2026-11-10
Authority: [service ADR revision 5](ADR_ONTODOC_SERVICE.md#schema-2-identity-and-canonical-artifacts-accepted-not-implemented),
[task OD-3.2](ONTODOC_IMPLEMENTATION_PLAN.md), user instruction to proceed.

## Implemented

- `src/parse.rs` replaces only empty allocated hierarchical ID components with
  `@empty`. Preamble IDs, allocated Markdown anchors, headings and chunk ordinal
  suffixes remain unchanged. No migration or new fragment aliases.
- `src/store.rs` writes schema 2 with captured-byte size and no persisted mtime.
  Canonical skip paths/reasons are sorted, preserve multiplicity and enter the
  compact-JSON generation hash. Independent roots and explicit different mtimes
  produce identical Generation, index, metadata, CURRENT and first-publication
  HISTORY bytes. HISTORY equality is conditional on identical initial sequence.
- Build/publish/load reject incompatible versions. Load rejects retired mtime
  fields; publish/load reject false canonical IDs. Publication verifies exact
  index AND metadata bytes for existing valid same-ID targets; conflicts preserve
  target, CURRENT and HISTORY. Existing corruption recovery/retention stays.
- Freshness still hashes current bytes: touching mtimes is fresh; same-size edits
  with restored mtimes are stale. Verification semantics and tiers are unchanged.
- Tests in parser/store/chunk/graph/CLI cover preambles, empty/nested/duplicate
  slugs, literal suffix collisions, Unicode, delimiter-bearing filenames,
  cross-budget identity, fragment/read compatibility, skips and refusals.

## Validation And Review

All commands ran in the root package with `CARGO_BUILD_JOBS=4`.
Managed launcher: sibling `scripts/compiler_lane_detached.py`, because this
standalone repository has no Justfile. Only this package was compiled.

| Command | Terminal result | Receipt |
| --- | --- | --- |
| `cargo test --lib --test cli` | exit 75, shared lane occupied; no tests ran | managed job `ontodoc-od32-schema2-tests-20261110-a`, harvest `e1d761` |
| `cargo test --lib --test cli` | exit 0: 92 library, 5 CLI passed | managed job `ontodoc-od32-schema2-tests-20261110-b`, harvest `1eae39` |
| `cargo fmt --check` | exit 0 | `5f2ecf` |
| `cargo clippy --all-targets -- -D warnings` | exit 0 | managed job `ontodoc-od32-schema2-clippy-20261110-a`, harvest `4d70dd` |

The lock refusal was respected. The existing owner's terminal receipt showed
completion before the corrected launch. Every owned job was harvested; no code
correction was necessary after tests/lint. Managed receipts live under
`$ONTOCODE_HOME/artifacts/compiler-lane/<job>/`.

Requested/effective reviewer **GPT-6-Astra**:
`agent-results/01a1c647-bc73-7330-b3e0-fb633c3aa3a0/1` found no correctness blocker
in parser/store. Its two suggested checks (mtime-only freshness and a real
skip-dependent build ID) were added before validation. Delta review `/2` found
no blockers or regression gaps in the final additions. Reviewer changed no
files and ran no Rust validation; passing results above are main-session proof.

## Scope And Limits

- Pre-edit 70-file SHA-256 inventory versus post-validation inventory reports
  exactly `src/parse.rs`, `src/store.rs`, `src/chunk.rs`, `src/graph.rs`,
  `tests/cli.rs` changed; no removed files. `src/tiers.rs` needed no change.
  Authorized closeout adds this audit and memory/architecture/tracker updates.
- Checkout has no initial commit and is untracked. Git diff alone cannot prove
  scope. OntoIndex remained degraded after recorded preparation/commit-identity
  failures; source tracing/hash inventory are the declared fallback. No repeated
  failing query or reindex was attempted; no graph-derived impact claim.
- Synthetic tests only; no real held-out payload, benchmark execution, retrieval
  improvement, performance measurement or cross-schema ranking-parity claim.
  Evaluation guards, ranking code, dependencies, donors and HTTP remain unchanged.
- Schema 1 requires trusted CLI reindexing. Current safety work still needs
  HTTP-0 decisions followed by OD-3.4/3.5/3.6; this closeout proves neither rooted
  I/O race safety nor multi-user HTTP readiness.
