# OD-4.0 evaluation documentation closeout

README now reports the registered experiment, H1, H2/H5 effects and 95% intervals, H3 Wilson intervals, H4 fixture limits, E1/per-corpus audit links and all model-judged/withdrawal/attribution restrictions. AGENTS records consumed custody and the ADR16 diagnostic exception. Architecture records the measured exploratory status and unchanged lexical defaults. Source and candidate binary remain unchanged.

The [held-out audit](audit_session-2026-11-13-held-out.md) and [no-op audit](audit_session-2026-11-13-default-noop.md) bind evidence and decisions. Proposed evidence/diagnostics extensions remain Proposed; no new tool, retrieval behavior, tenancy or authentication was introduced.

`CARGO_BUILD_JOBS=4 cargo test` ran from the main root on 2026-11-13 and exited 0: 281 passed, one ignored subprocess helper, zero failed (218 library, 8 CLI, 24 evaluation gates, 14 evaluation run, 4 HTTP, 12 MCP, 1 ranking-sanity). No doc tests were defined. The ignored helper is exercised explicitly by process-lock tests. These synthetic tests do not rerun the real held-out benchmark. Formatting and managed all-target Clippy passed during OD-4.1; no Rust edits followed.

REL-6 remains pending: local package preparation is authorized; destination, repository/history exposure and publication review remain owner decisions. No tag or public release is claimed.
