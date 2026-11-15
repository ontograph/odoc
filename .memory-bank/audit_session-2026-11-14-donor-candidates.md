# Donor candidate execution

Date: 2026-11-14

## DC-REG

The user's instruction to proceed authorizes the plan's recommended nullable search tier, related omission count, shared CLI text withholding and descriptive census. ADR18 records those contracts before implementation. Deferred ranking candidates remain deferred.

The tracker uses local execution with explicit role scaffolds. Public README/changelog work is assigned to DC-DOC because strict validation excludes manager paperwork from code-worker scope. Existing working-tree changes are preserved.

Registration validation: strict project-plan validation passed with four tasks and no errors.

## DC-1 baseline

`CARGO_BUILD_JOBS=4 cargo test --test mcp oversized_first_hit_preserves_citations -- --exact` exited 101: the expected four results were zero. Added direct ranking assertions passed (four hits, large chunk first), confirming response trimming. One diagnostic attempt used nonexistent `Params::default` and failed compilation; corrected to `Params::new`. The baseline rerun reproduced zero results. Revision 3 corrected the mistaken cap-error premise before production edits. The 84,000-byte fixture now passes.

## Closure

DC-1 changed only `src/cli.rs`, `src/mcp.rs`, `tests/cli.rs`, `tests/mcp.rs` and `tests/http.rs`. Worker validation commands all exited 0: `CARGO_BUILD_JOBS=4 cargo test --lib cli::` (14), `--test cli` (9), `--test mcp` (17), `--test http` (7), `--all-targets`, `cargo fmt --check`, `git diff --check`, and the registered managed `cargo clippy --all-targets -- -D warnings` lane (terminal receipt exit 0). Independent source review found no blockers; application JSON has an exact-byte regression, while historical MCP wire bytes have no separate fixture. OntoIndex scope verification passed with its 25-symbol cap and dirty-coverage limits. Existing dirty work was preserved.

DC-3 saved its script, output and hashes under `tmp/donor-candidates/census/`; command exited 0. Rust Book cuts 10 link edges and 45 mentions beyond 100; OntoIndex cuts none. Maximum chunk sizes are 10,269 and 33,444 bytes. Link calibration failed (0/17 and 9/10 diagnostics), so no added-edge counts or claims follow. Selected preparation matching remains unconfirmed. No reindex or evaluation payload access occurred.

DC-DOC updated README, changelog and architecture with nullable tier, omitted counts and read-prefix limits. References and structure were checked; `git diff --check` exited 0. No additional Rust build was needed after documentation-only changes.

Both read-only `target/debug/ontodoc --root <root> status` checks exited 0: Rust Book generation `375917fc2e8cd756e785fdc118e6f5d12f4967c69963f89469de1b010d38ca95` and OntoIndex `4e546b16f9fd4533f597a435487d1ba7175070d33992138f4c4a49889464446b` are schema 4 and fresh, with 17 and 10 link diagnostics. These checks were prerequisites for the completed descriptive census above; preparation-generation matching remains unconfirmed. The [follow-up review](audit_session-2026-11-14-donor-review.md) preserves the exact census result and its verified hash.
