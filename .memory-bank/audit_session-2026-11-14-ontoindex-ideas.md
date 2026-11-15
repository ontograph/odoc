# OntoIndex Ideas Execution

Date: 2026-11-14
Tracker: `docs/ONTOINDEX_IDEAS_PLAN.md`

## Registration

The user instructed: "proceed open tasks from docs/ONTOINDEX_IDEAS_PLAN.md". Under OI-REG's explicit gate this accepts recommendations 1–6: E1–E5 at publication and load without schema bump; corpus outage on refusal; manual directory-removal recovery conditional on the synthetic proof; D1 duplicate-document-path refusal; a test-only link census; and descriptive capacity measurement without a latency target. ADR19 records the product decisions. All five tasks are now implemented and validated below.

Pre-existing dirty files were observed before work and are preserved. The supplied `@ONTOINDEX.md` reference has no root file in this checkout; the supplied inline OntoIndex instructions and installed navigation skill apply. OntoIndex reports incomplete dirty source coverage; source inspection is required for affected owners. No evaluation query, label or result payload was opened, and no pinned store or preserved release artifact was changed.

Initial strict plan validation passed (5 tasks, no errors). The manager returned `wait` for the pre-authorization state. `reopen` refused because a waiting manager already has an active task; no lease or receipt exists. Registration changes that unleased waiting status to in-progress under the new user authorization. No runtime role results are edited.

Validation and task outcomes are recorded below as they occur.

OI-REG: `git diff --check` exited 0; `project_plan_validate docs/ONTOINDEX_IDEAS_PLAN.md` returned pass (5 tasks, no errors). `wc -lc .memory-bank/MEMORY.md` exited 0: 45 lines, 3523 bytes. Documentation references and the six-file write set were checked. No Rust compilation was required or performed for registration.

## OI-2: Calibrated Link Census

The new code is confined to the `src/graph.rs` test module. The 20-event synthetic fixture states hand-counted classes. Both real-corpus outputs match stored linked-pair/diagnostic counts, 276/17 and 44/10. Decision 3 of `docs/ADR_DONOR_ADOPTION.md` records all event classes and distinct ignored destinations. No candidate edge-gain count or link-resolution change follows.

- `CARGO_BUILD_JOBS=4 cargo test --lib graph::`: exit 0, 8 passed and 2 ignored.
- `env ONTODOC_LINK_CENSUS_ROOT=tmp/corpora/rust-book/src CARGO_BUILD_JOBS=4 cargo test --lib graph::tests::link_outcome_census -- --ignored --nocapture`: exit 0.
- `env ONTODOC_LINK_CENSUS_ROOT=tmp/corpora/ontoindex CARGO_BUILD_JOBS=4 cargo test --lib graph::tests::link_outcome_census -- --ignored --nocapture`: exit 0.
- `cargo fmt --check`, `sha256sum -c tmp/ontoindex-ideas/link-outcomes/SHA256SUMS`, `git diff --check`: exit 0.
- Managed `CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings`: initial `ontodoc-oi2-clippy-20261114` exit 101 (`collapsible_if` in census); corrected locally, focused tests passed; `ontodoc-oi2-clippy-fix-20261114` terminal exit 0. Exact receipts remain in the compiler lane; `tmp/ontoindex-ideas/link-outcomes/validation.json` records checks.

## OI-3: Capacity

The [capacity audit](audit_session-2026-11-14-ontoindex-ideas-capacity.md) records build identity, exact commands/exits, all computed charges and measured outcomes. B16/B60/E50 publish; EP index refuses and Over status/search refuse while verify stays valid. Evidence is descriptive, measured before OI-1 and without a latency target. Both scripts and 37 output files are checksum-bound under `tmp/ontoindex-ideas/capacity/`. No budget or freshness policy changed.

## OI-1: Generation Invariants And Recovery

Only the new validation call/method, HashMap import and four regression tests plus their helpers were added in `src/store.rs`. Existing lock repairs were preserved. `Generation::validate_edges_and_documents` enforces E1–E5/D1 through publication and all load adapters. All 13 one-field malformed-artifact cases repair metadata hashes/length and assert exact reasons through ordinary, rooted and captured loaders. Publication refusal preserves CURRENT, HISTORY, generation directories and artifact/meta bytes. Recovery proves same-id conflict, then removal of only the refused generation directory, republish and load for both malformed edges and duplicate chunk IDs. Valid/empty fixtures retain canonical bytes and IDs; existing test expectations were unchanged.

OntoIndex impact was CRITICAL: 43 upstream symbols, seven flows and five modules. Direct callers are publication and verified decoding. Dirty graph coverage is incomplete; source was checked. Scoped `verify_diff` passed, but capped at 25 symbols and had no expected-symbol assertion; it does not establish test coverage. Read-only Luna review found no material issue; tests below were run by the coordinator.

| Command | Terminal outcome |
| --- | --- |
| `CARGO_BUILD_JOBS=4 cargo test --lib store::` | exit 0; 49 passed, 1 ignored helper |
| `CARGO_BUILD_JOBS=4 cargo test --lib cli::` | exit 0; 14 passed |
| `CARGO_BUILD_JOBS=4 cargo test --lib search::` | exit 0; 25 passed |
| `CARGO_BUILD_JOBS=4 cargo test --all-targets` | exit 0; output capture was truncated, so no aggregate count is claimed |
| `cargo fmt --check` | exit 0 |
| `just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached ontodoc-oi1-clippy-20261114 env CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings` | terminal command exit 0; 4.068 s; harvested with `lane-result ontodoc-oi1-clippy-20261114 --wait 60` |
| `CARGO_BUILD_JOBS=4 cargo run --quiet -- --root tmp/corpora/rust-book/src status` | exit 0; fresh schema 4, generation `375917fc2e8cd756e785fdc118e6f5d12f4967c69963f89469de1b010d38ca95` |
| `CARGO_BUILD_JOBS=4 cargo run --quiet -- --root tmp/corpora/ontoindex status` | exit 0; fresh schema 4, generation `4e546b16f9fd4533f597a435487d1ba7175070d33992138f4c4a49889464446b` |
| `git diff --check` | exit 0 |

Three worker dispatches were rejected before an agent started (missing Scenario, Given, then Write set fields). The no-progress guard stopped that turn. The user renewed the same proceed instruction, explicitly resuming the task; implementation ran locally. No worker lease or result was fabricated. One OI-2 Clippy correction was required; no OI-1 compiler retry, disk recovery or publication-protocol change was needed.

## OI-DOC And Limits

README now describes the refused-store outage and tested recovery, plus the capacity case sizes, latency and binding budgets. Changelog records unreleased 0.1.1 behavior; donor Decision 3 contains calibrated counts; architecture records the store owner. Status routers retain independent release/evaluation gates. The source binary used for capacity predates OI-1; its added validation time/memory remains unmeasured. No test-split payload, pinned store or preserved release artifact was changed; no release, version bump or retrieval-quality claim is made.

Prior active-memory route retained verbatim here before replacing it with the implemented standing fact:

> - [OntoIndex ideas](../docs/ONTOINDEX_IDEAS_PLAN.md) - authorized; OI-REG registers ADR19, followed by link census, capacity measurement, invariant implementation and documentation. [Evidence](audit_session-2026-11-14-ontoindex-ideas.md).

OI-DOC validation: local Markdown links, trailing whitespace and MEMORY bounds passed (Python check exit 0; MEMORY 45 lines / 3590 bytes). `git diff --check` exited 0. No Rust compilation was needed for documentation. One closeout patch failed context verification without writing; a corrected patch applied after reading current context. Strict tracker validation passed all five tasks with no errors. `complete-local` accepted OI-DOC; final `manager_loop_next` returned `no_dispatch`, no active task and no eligible task. All five tasks are DONE; no open task remains in this plan.
