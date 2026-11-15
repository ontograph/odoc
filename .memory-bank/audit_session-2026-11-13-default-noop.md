# OD-4.1 default no-op

The accepted [held-out audit](audit_session-2026-11-13-held-out.md) binds report SHA-256 `006af6e5fcae05a9693edcc254adf7381d5622dbeae211264578f32d1a9691e0` and completed custody. H1 passed; H2/H5 have model-judged labels and exploratory roles, null decisions and null Holm p. ADR13 and the predeclared h5-then-h2-v1 policy authorize no promotion on this track. OD-4.1 therefore changes no application source or tests.

Default mode stays lexical; alpha and beta remain 0.3; omitted threshold remains None. Explicit requests retain existing behavior. Read-only explorer verified `src/cli.rs`, `src/mcp.rs`, `src/cite.rs`, `src/search/mod.rs`, `tests/cli.rs` and `tests/mcp.rs` are byte-identical to candidate F (`cc116c44a4d7f854bc19c6a4b40d75098a437268`) and its checkout. This verifies an unchanged contract, not new comprehensive parity coverage.

## Validation in this session

Commands ran from the main root package, with no overlapping compiler work:

- `CARGO_BUILD_JOBS=4 cargo test --lib cli::`: exit 0, eight passed.
- `CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp`: exit 0, eight CLI and 12 MCP tests passed.
- `cargo fmt --check`: exit 0.
- `just --justfile ../ontocode/justfile compiler-lane-detached ontodoc-010-noop-20261113-final sh -c 'CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings'`: launcher exit 0.
- `just --justfile ../ontocode/justfile lane-result ontodoc-010-noop-20261113-final --wait 60`: exit 0; terminal result exit 0. Receipt: `/opt/d2/.ontocode/artifacts/compiler-lane/ontodoc-010-noop-20261113-final/result.json`.

No held-out rerun, tuning, relabeling or frozen-binary rebuild occurred. Existing tests exercise default CLI/MCP use, schema presence and citations; they do not independently exhaust every omitted-weight schema combination. No new behavior was introduced that needs new tests.
