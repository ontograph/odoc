# GraphRAG review and repair

Authority: user requested GPT subagents, OntoIndex navigation, challenge of completed work, comparison of solutions, consolidation and execution. [Consolidated plan](../docs/GRAPHRAG_REVIEW_PLAN.md) owns two tasks. Original GraphRAG tasks remain DONE.

## Findings and decisions

- Good: existing citation constructors, bounded borrowed endpoint maps, stable output order, explicit truncation and actual stdio/HTTP frame tests. No schema, ranking or freshness shortcut was introduced.
- Fixed: raw incoming ID matching confused unresolved code targets with real chunk IDs. A legal `code:target.md` containing `# Target` and a separate inline code span `target.md#target~0` generated equal strings in different node domains. `App::related` now matches mentions only at their source; retained outgoing mentions must have the existing `code:` prefix. Stored artifacts and IDs are unchanged.
- Added coverage: legal collision, invalid outgoing mention targets, exact first-100 prefix at 100/101 incident edges, citation presence and repeated serialized equality. Incoming non-mention edges remain supported.
- Simplified direction: retained existing linear scans and bounded temporary maps. Permanent index maps, early-stop counters and shared serialization redesign have no measured need. The pre-existing repeated-pop serializer can do quadratic work in the capped item count; it is not a new defect or measured bottleneck.
- Corrected next-step design: withdrew GR-5's unapproved majority rule, which ignored loss magnitude, relevance grades, ranks and unjudged entries. Descriptive diagnosis with explicit adjudication is the recommendation; no new threshold or data access is approved. Hub counts alone do not justify pagination or schema-5 link spans.
- Corrected prose: the overflow fixture uses long `!` headings, not JSON-escaped headings. Updated stale memory wording that said corrected-candidate repairs still awaited a decision.

## GPT review evidence

All three explorers used GPT-6.1-Sol. Correctness receipt `agent-results/01a1d6d3-4be0-7100-8121-0ded2e08113b/1` identified the two mention issues and the boundary-test gap. Its post-edit receipt `/3` found no blocking findings. Performance receipt `agent-results/01a1d6d3-6ca0-71d0-ad0f-9316e1930a7d/1` recommended keeping the current lookup design. Plan-review receipt `agent-results/01a1d6d3-95af-7d83-a1a9-defeac2ea2b2/1` challenged the GR-5 rule and distinguished historical validation from release readiness. Children changed no files and ran no Rust validation; performance review ran whitespace checking and a serialization-work arithmetic model, not timing.

OntoIndex search/context identified App::related and citation constructors. Impact was LOW with three indexed direct dependents. Safe-edit classified the exported API DANGEROUS and warned of incomplete dirty graph coverage; source confirmed both CLI and MCP callers. The signature is unchanged. review_diff refused unsupported scope all; test_gap could not establish indexed coverage. verify_diff passed file scope with no symbol allowlist and pre-existing dirty differences. Final reviewer query degraded with workspace build failure; source was authoritative. No reindex was started.

## Current validation

Commands ran in `/opt/d1/_wrk/ontodoc`, native Linux, serially, with `CARGO_BUILD_JOBS=4`. Each compiler command used `just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached JOB env CARGO_BUILD_JOBS=4 COMMAND`. Terminal receipts were harvested by `just --justfile /opt/d1/_wrk/ontocode/justfile lane-result JOB --wait 60`; receipt directory `/opt/d2/.ontocode/artifacts/compiler-lane/JOB/` contains `result.json`, `artifact.json`, `output.log`.

| JOB | COMMAND | Terminal exit | Evidence |
| --- | --- | --- | --- |
| ontodoc-grr-cli-20261114 | `cargo test --lib cli::` | 101 | 13 passed, 1 failed: new fixture counted all graph edges, including document→section |
| ontodoc-grr-cli-fixed-20261114 | `cargo test --lib cli::` | 0 | 14 passed after counting only incident edges |
| ontodoc-grr-mcp-20261114 | `cargo test --test mcp` | 0 | 13 passed |
| ontodoc-grr-http-20261114 | `cargo test --test http` | 0 | 7 passed, including full-frame citation overflow parity |
| ontodoc-grr-clippy-20261114 | `cargo clippy --all-targets -- -D warnings` | 0 | No warnings |

`rustfmt --edition 2024 src/cli.rs`, `cargo fmt --check`, and `git diff --check` exited 0. Fresh focused total: 34 passing tests. All-target tests, release build/package verification and performance timing were not rerun: the task uses focused existing-owner and transport checks plus all-target Clippy. The earlier 293-pass/3-ignored result remains historical. No before-fix collision execution was captured; source establishes the prior fault and the new regression passes on the correction. The initial failed boundary assertion is fixture failure, not evidence of the runtime defect.

Validated `src/cli.rs` SHA-256: `26ff922ad7c545c37773ccfd2acca1b33a1b6dcd9867faf040d4e85b0ca05337`. A start/end comparison confirmed unchanged bytes for 12 surrounding source, test, package, ADR and original implementation-plan files. Only `src/cli.rs` changed at runtime; the broad pre-existing Git diff is not this task's diff. No evaluation payload, custody, donor or preserved release artifact was opened or edited for this review. No quality, speed or release-readiness claim follows.

## Process corrections and remaining work

Tracker registration rejected unsupported complexity `medium`, then reported the release-tier mechanical minimum `complex`; applying the explicit minimum validated the two-task plan. GPT worker dispatch first failed packet shape, then the worker refused missing literal packet fields without edits. After one worker follow-up was refused, the coordinator closed it and implemented locally under dispatch_mode none. GPT review remained independent. One test-fixture correction was required; all compiler jobs were harvested. No disk recovery or active child remains.

GR-5 remains pending exact protocol approval; REL-6 remains blocked on destination, exposure and embedded-path decisions. No publication or held-out run occurred. The smaller selected plan fixes demonstrated behavior and documents the rest instead of automatically expanding the feature set.

Final documentation validation: `git diff --check` exit 0; bounded Python checks of ten Markdown files passed local paths, fences and whitespace, MEMORY bounds and unchanged validated source hash (exit 0). Strict plan validation passed. Both GRR tasks completed through `complete-local`; manager is closed with no active task. The removed active router entry is retained verbatim: `[GraphRAG review](../docs/GRAPHRAG_REVIEW_PLAN.md) - GRR-1 typed mention incidence and boundary tests, then GRR-2 documentation; GPT review findings, no evaluation or publication.`
