# EVAL-1.6: Trusted Preparation And Default Promotion

Date: 2026-11-11
Scope: manager-owned documentation/design only; service ADR revision 9.
Status: DONE through strict `complete-local`; EVAL-1.3 selected next.

## Decision And Alternatives

| Option | Decision |
| --- | --- |
| Recompute full gates on every dev command | Rejected: opens held-out and full retrieval-pool payloads, and still does not establish human/reviewer identity or chronology |
| Introduce a signing framework or independent registry service | Rejected: unnecessary new infrastructure; signatures alone would not authenticate annotation or independence |
| Explicit trusted preparation, exact-byte metadata handoff and existing fixed custody | Selected: reuse current CLI/evaluation owners; separate full-data operator preparation from selected-only execution |

The operator approves authentic review/annotation evidence and exact artifact
hashes in an audit. JSON flags, timestamps, reviewer model strings and local
custody are not proof of human identity, independent review or pre-pool chronology.
The decision requires versioned provenance companions, bounded preparation
manifests, both partition commitments and derived counts. It does not claim
these contracts are implemented.

Freeze `h5-then-h2-v1` before held-out access: H5 pass recommends hierarchical
mode/frozen beta; otherwise H2 pass recommends graph/frozen alpha; otherwise
lexical/no-op. Both arms remain reported. This is a fixed preregistered policy
preference, not a latency comparison or another confirmatory test. The current
shared hierarchical path also computes PPR; no algorithm change is authorized. Omitted
threshold remains disabled because H3 is descriptive. EVAL-3.2 recommends only;
new OD-4.1 owns the minimal default implementation/no-op before OD-4.0.

## Source Findings And Ownership

GPT-6-Astra explorer `01a1c74f-6a33-7172-b3ee-b3ffdbb71489`, results `/1`–`/3`,
traced existing owners without editing files or running Rust commands:

- `src/eval/run.rs`: Query/Judgement and EvalConfig (`:88`–`:127`),
  InputIdentity/MechanicalEvidence (`:364`–`:399`), prepare/load_dev (`:494`,
  `:520`) and full-pool open preceding split selection (`:654`–`:661`).
  Mechanical capture lacks durable query-review/annotation admission; dev
  isolation must exclude the full pool and provenance, not just test queries.
- `src/eval/gates.rs:69` and `:359`: existing configuration/full-data gates
  cannot establish annotation provenance or run unchanged on a selected dev view.
- `src/eval/sweep.rs:491`–`:499`, `:552`–`:565`: freeze/check need whole
  preparation commitments distinct from selected digests, plus approved test
  counts without loading test labels.
- `src/cli.rs:1009`–`:1135` and `src/eval/run.rs:1311`: public measurement
  remains universally refused. EVAL-1.3 retains complete synthetic success and
  failure integration, not merely deletion of the guards.
- `src/eval/run.rs:962` and `:1548`–`:1550`: extend existing fixed custody;
  a visible report without completed matching custody is unaccepted, consumes
  the attempt and grants no retry. Reservation precedes held-out payload access.
- `src/cli.rs:31`, `:150`–`:192`, `:337`–`:352`: shared mode/weight omissions;
  `src/mcp.rs:406` consumes SearchRequest, and `src/search/mod.rs:138` takes
  explicit mode. OD-4.1 needs production edits only in `src/cli.rs`, with
  `tests/cli.rs` and `tests/mcp.rs` compatibility checks. No parallel defaults owner.

Research receipts: `d064dc`, `04db79`, `5d3182`, `9fb8db`, `b50781`, `59761f`,
`54fc98`; default-owner followup `63c556`, `55aab5`, `565a42`, `c92bb7`,
`45a121`, all recorded exit 0. These support source inspection, not generated
schema/runtime behavior or current-tree tests. Requested/effective GPT-6-Astra
identity was confirmed by the agent listing; models do not count as humans.

## Tracker Preservation And Corrections

[Verbatim evidence preservation](audit_session-2026-11-11-tracker-evidence-compaction.md)
retains 24 historical DONE validation-summary scalars and exact SHA-256 hashes.
The original tracker was 130,466 bytes. First compaction produced 113,312 bytes;
new contracts grew it to 121,434 bytes, then the second compaction reduced it to
110,286 bytes. Receipt `75a598`, exit 0, restores and compares every DONE card
and manager state against both checkpoints. No authenticated history, leases,
receipts, role results, statuses or dependencies were discarded.

The first 40-task validator pass attempt rejected an audit Markdown path in a
data worker's write set and OD-4.1's unmanaged/integration-tier all-target lint.
The corrected plan preserves manager ownership of approval paperwork and uses
a managed release-tier lint with terminal receipt harvest. Subsequent validation
passed 40 tasks with no errors. The >1,500-line advisory remains.

GPT-6-Astra delta review result `/4` identified two material blockers: a claim
that hierarchy avoids PPR (contradicted by `src/search/mod.rs:339`–`:362`), and
missing exact manager authority for the required query-approval audit. Both are
corrected: the fixed H5-first table has no performance rationale, and Decisions
grants only the manager the exact audit path during EVAL-2.0. The data worker
cannot write or approve that audit. Review receipts `13a877`, `b510e5`, `2cb978`,
`80e0e4`, `23f1a9` recorded exit 0; review is source-only, not runtime validation.
Result `/6` verifies the actual two corrected passages, with no remaining
blockers in that scope; receipts `b7d213` and `aa7596`, exit 0. Result `/5` had
acknowledged the parent-reported first correction without rereading; `/6` is
the source-verified correction closure, not a full independent runtime audit.

A patch-assembly command produced truncated JSON and failed to decode before
any tool write. The corrected bounded patch applied once; subsequent scalar
and state comparison passed. This was not lost work, a Rust failure or a
rejected manager receipt.

## Validation And Limits

- Strict `complete-local` accepted revision
  `692a922ad7197e537106e047cb9d44a9ab1b43ec3a99dd7f2ad945e6ec60a51d`
  and produced `6b6e432ffdcb704d52cfc27f7801820d1fc3f71304d298bd6a81c49bb85d4969`.
  Next strict decision selects dependency-ready EVAL-1.3. Counts: 29 DONE,
  9 OPEN, 2 BLOCKED of 40. No code or benchmark task was completed by this decision.
- Final pre-close plan validation: PASS40, zero errors; only the existing
  >1,500-line advisory. Receipt `c3a3cf`, terminal exit 0, checks all eight changed
  documents, local references/anchors/fences/whitespace, MEMORY exactly 100 lines,
  task linkage and baseline/protected scope; all 24 scalar replacements and
  historical DONE cards are preserved. `git diff --check` also exits 0. Six
  existing memory documents and two new audits are the complete write scope.
- Recovery accounting: no active lease, rejected manager receipt, disk recovery
  or background job resumption. One structural-validator correction after the
  three reported errors; two concrete Astra findings corrected once and verified.
  The truncated patch-assembly failure made no repository write. No unchanged
  blocked review or validation retry occurred.
- Startup strict manager decision: execute-local EVAL-1.6, no lease; plan
  validator PASS39. ADR9/task alignment adds OD-4.1, with OD-4.0 dependent on it.
- Receipt `2a06c8`, terminal exit 0: Python checks of local Markdown references,
  anchors, fences, whitespace, task linkage, MEMORY line cap, baseline write
  scope and resumed payload hashes; `git diff --check` exit 0 in that chain.
- Baseline `/tmp/ontodoc-eval16-baseline-ufvtaljl.json` has 81 entries. Source,
  tests, Cargo/lock/toolchain and unrelated documents remain unchanged. A resume
  inventory binds the only existing eval file, `corpora.json`, plus build inputs;
  no queries, labels, pool, rankings or real results were opened.
- This checkout has no initial commit. Empty `git diff` is not scope proof;
  byte inventories and exact tracker comparison are used instead.
- OntoIndex's earlier committed query failed because indexed commit identity
  was unknown. Source inspection is authority; no unchanged retry, reindex or
  graph-backed PASS is claimed.
- No Cargo build/test/fmt/Clippy, binary/schema execution, benchmark, network
  listener or real annotation work ran for this documentation task. HTTP-0's
  owner choice, two-human annotation and clean committed frozen identity remain
  required later. No retrieval, latency, capacity or RSS improvement is claimed.
