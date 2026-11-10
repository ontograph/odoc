# OD-3.4: Rooted Reads And Exact Citation Membership

Date: 2026-11-10
Authority: service ADR revision 8; user proceed; strict manager selected OD-3.4
for local implementation. This record authorizes no HTTP or evaluation run.

## Implemented

- `src/corpus.rs`: direct locked rustix 1.1.4 `fs`, retained root descriptor,
  final-root symlink refusal (including trailing slash/dot), full-relative
  openat2 BENEATH/NO_SYMLINKS, checked regular-descriptor reads. Unsupported or
  unsafe resolution refuses without fallback. RawDir uses fixed 64-KiB storage,
  charges dot/terminal/error probes, preserves enumeration failures and resolves
  unknown types through rooted metadata. Each walk gets a fresh directory offset.
- Shared explicit ignore traversal replaces automatic production loading;
  the independent automatic walker remains a test oracle. Controls that disappear
  after preflight refuse without refunding failed work. Existing readable-tree
  policy fixtures pass. Rooted publication is not introduced; writes remain the
  trusted CLI/store boundary selected by the ADR.
- `src/store.rs`, `src/cli.rs`: App eagerly retains its root or setup error;
  CURRENT/meta/artifact, build, freshness and verification use that root. A
  selected generation loads without reading CURRENT a second time. Existing
  pointer/meta/artifact caps and captured decoding remain shared.
- `src/cite.rs`: shared whole-section/chunk constructors and exact full-batch
  membership checks precede target reads. Old/unknown IDs return
  `generation_unavailable`, same-generation nonmembers `invalid_citation`.
  No HISTORY lookup or arbitrary-path verification route remains. Valid members
  preserve valid/moved/ambiguous/stale/missing; only missing/symlink/nonregular
  objects become missing, while I/O/capability/race/cap failures stay typed.
- `src/mcp.rs`: verification uses the shared pinned index and App; cache misses
  load the selected CURRENT ID. Five tools and stdio parity remain unchanged.

Synthetic regressions cover retained-root replacement before first load and
after caching, failed setup without rebinding, publication after pointer
selection, identical tuples across independently bound roots, redirected store
ancestors/caps, every citation field, invalid batch tails before reads, hidden
whitelists, newly ignored members, whole sections, lower tiers and truncated
reads. Primitive checks cover outside-root ancestor moves, symlink/FIFO/socket/
directory leaves, unknown types, post-open replacement/growth/shrink/exact caps,
deleted-directory enumeration and one-shot unsupported/race propagation.

## Review

Both read-only reviewers requested and effectively used GPT-6-Astra:

- Rooted reader: `01a1c6f9-bc93-7000-a420-a771e8677083`, results `/1`
  discovery, `/2` challenge, `/3` final delta. Findings corrected: disappearing
  control refund, ENOTDIR classification, embedded NUL and socket-open handling;
  primitive regression gaps filled. Final result found no remaining concrete
  blockers in the requested deltas.
- App/store/MCP: `01a1c6f9-bd13-7eb3-a739-ad267e053d81`, results `/1`
  discovery, `/2` review, `/3` final delta. Root/pin/adapters/integration evidence
  gaps filled; explicit `--lib cli::` added to execute App unit regressions.
  Final result found no concrete blocker/missing mandatory integration.

Worker `01a1c6fe-982a-7601-a079-3ea287b00177` result `/3` implemented only
`src/corpus.rs` and `Cargo.toml`; parent owned integration and validation.
Children ran no builds/tests. Rejected initial worker packets produced no edits;
correction did not expand ownership. Agents are closed. Static review does not
establish independent runtime validation.

## Executed Validation

Native repository root, Rust 1.95.0, `CARGO_BUILD_JOBS=4`, serial managed lane
under `/opt/d2/.ontocode/artifacts/compiler-lane/`. Every started job was harvested
with the canonical reader; launcher success was never treated as test success.

| Job | Terminal outcome |
| --- | --- |
| `ontodoc-od34-focused-20261110-a` | exit 101: test used `Errno::ACCES`; corrected to `ACCESS`. Receipt `9e2866`. |
| `ontodoc-od34-focused-20261110-b` | exit 101: corpus 18/19; expected lexical `/tmp` instead of trusted-parent canonical root. Corrected fixture. Receipt `2b9718`. |
| `ontodoc-od34-focused-20261110-c` | exit 101: preceding corpus/cite/store/CLI passed, store 24 and CLI 6 visible; MCP 8/9, lower-tier fixture had identical L0/L1. Added child heading, preserving real tier semantics. Prefix truncated. Receipt `f99d2b`. |
| `ontodoc-od34-validation-20261110-d` | exit 101: corpus 20/21; added ancestor-move fixture used lexical path instead of reader logical root. Corrected fixture, no production weakening. Receipt `62c738`. |
| `ontodoc-od34-validation-20261110-e` | All commands below through App tests passed; Clippy exit 101 on three test-only clone-to-slice lints. Corrected to `slice::from_ref`. Prefix truncated; no invented test totals. Receipt `3ff89b`. |
| `ontodoc-od34-lint-20261110-f` | Clippy, formatting and formatting check exit 0. Receipt `caf03e`. |

Job e, exact serial command prefix (each completed with exit 0 before Clippy):

```sh
CARGO_BUILD_JOBS=4 cargo test --lib corpus::
CARGO_BUILD_JOBS=4 cargo test --lib cite::
CARGO_BUILD_JOBS=4 cargo test --lib store::
CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp
CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
CARGO_BUILD_JOBS=4 cargo test --lib cli::
```

Job f, exact successful chain:

```sh
CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings
cargo fmt
cargo fmt --check
```

The last semantic edits before job e were a canonical-path fixture correction.
After e, only the three test-only slice expressions and formatter output changed.
No complete library/workspace suite, throughput/RSS benchmark, real held-out
evaluation, hostile-administrator publication proof or HTTP test was run.
Historical parallel store Locked failure remains OD-3.9; a passing focused run
does not establish its cause or repair.

## Scope And Limits

Pre-edit SHA inventory `/tmp/ontodoc-od34-baseline-6qe8s6q1.json` confirms nine
implementation files changed, exactly OD-3.4's allowed set: Cargo.toml/lock,
corpus/cite/store/cli/mcp owners and CLI/MCP integrations. Receipt `62d87a` binds
final source hashes. Authorized manager updates add this audit and update plan,
status/pending/memory and architecture; no donor/eval payload/ranker/schema edits.
Checkout has no initial commit; an empty Git diff cannot prove scope.

OntoIndex workspace preparation failed (`workspace_busy`); corrected committed
impact failed unknown indexed commit identity. Source inspection is authority;
no repeat unchanged failure, manual reindex or graph PASS. Shared-root changes
were treated as high risk before editing. Document/reference/whitespace and
strict plan checks are required for closeout; terminal manager result controls
completion, not this narrative.

OD-3.6 still owns streaming freshness and allocation-light load/construction
admission; OD-3.5 owns raw/decoded input, aggregate verification work and actual
work/cancellation ownership. Current-byte semantics remain, not TTL freshness.
HTTP-0 remains owner-gated, measurement disabled; no performance/quality claim.

## Closeout

Strict `complete-local` completed OD-3.4 with OD-3.6 selected next. Post-tool
tracking revision: `6972f11a6f61c9e46e09f58c0b9df4ce930aff35dddd400a15da9949fb0f0774`.
Counts: 25 DONE, 12 OPEN, 2 BLOCKED / 39; manager remains in progress.
Document link/fence/whitespace/scope checks and `git diff --check` exited 0,
receipt `6f21ab`; strict plan validation passed 39 tasks with no errors before
completion. Parser-size warning remains (94%); no authenticated history removed.
