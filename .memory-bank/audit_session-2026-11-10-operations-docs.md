# OD-3.7: Operational Documentation

Date: 2026-11-10
Authority: [OD-3.7 task](ONTODOC_IMPLEMENTATION_PLAN.md), accepted
[service ADR revision 5](ADR_ONTODOC_SERVICE.md), user instruction to proceed.

## Changes

- README describes the root Rust package, installation, trusted CLI indexing,
  schema-1 reindexing, stdio client setup and all five tools/CLI equivalents.
  Prior-attempt provenance remains in its own read-only snapshot section.
- Current limitations are explicit: disabled measurement/scoring/freezing,
  no HTTP implementation or exposure grant, unresolved rooted I/O/membership
  and aggregate work/allocation/cancellation, no quality/capacity claim.
- AGENTS states the implemented schema-2 rules and HTTP-0 owner authority gate;
  its folder routing is labelled design, not runtime. Existing scientific,
  privacy, validation and donor boundaries are preserved.
- Architecture points operations to README. No runtime, dependencies, donor
  snapshot or evaluation payload changed. HTTP-0 remains blocked; independent
  documentation does not accept the proposals in that review.

## Validation Scope

Parent documentation check `2791b8`, exit 0: local links and heading anchors,
fence balance, whitespace, parseable client JSON, command/tool names and flag
shapes checked against existing CLI/MCP/Cargo source, memory-router line bound;
`git diff --check` exit 0. Required plan validator passed 35 tasks with no errors
and only the existing length warning. No install command or documented example
was executed: command shapes are source-checked, not an installation smoke.

SHA-256 comparison against the pre-task inventory reports only README, AGENTS,
architecture and the plan's predeclared local-closeout scaffold changed before
audit/router closeout. No source/test/Cargo files changed. Hash scope is required
because the checkout is untracked with no initial commit; Git diff alone is
not evidence. The first combined patch rejected stale AGENTS context atomically;
source was reread and the corrected patch applied. It changed no partial files.

No Rust build, test, formatting or Clippy in this documentation task. OD-3.2's
earlier Rust validation remains its own recorded result, not a new docs check.
No network listener, benchmark or held-out query/label access. OntoIndex's
unchanged preparation/commit-identity failure uses the recorded source fallback;
no repeated failing query or reindex.

Requested/effective GPT-6-Astra read-only reviewer:
`agent-results/01a1c656-a8f5-78b1-a8fb-4585c3d3094f/1`, no blocking factual,
security or compatibility findings. Its scoped `git diff --check` exited 0
(`b79541`); no Rust validation or installation smoke, no files changed. Final
main-session link/structure/scope and plan checks cover closeout/router edits.
