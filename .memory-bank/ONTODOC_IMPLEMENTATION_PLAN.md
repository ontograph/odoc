# OntoDoc Implementation Plan

Date: 2026-11-10
Status: HTTP delivery done; EVAL-2.0 parked by owner (2026-11-11) pending human inputs; see manager_loop
Authority: `ADR_ONTODOC_SERVICE.md` (Accepted, revision 10); user requirement of
2026-11-08: new Rust MCP service for documentation in this repository, with
scientifically provable improvements. Reviews: `audit_session-2026-11-08-prior-attempt-review.md`,
`audit_session-2026-11-08-donor-ideas-review.md`, `audit_session-2026-11-08-plan-challenge.md`.

## Goal

Deliver the five-tool stdio service with explicit correctness/resource repairs,
then the accepted independent folder-scoped HTTP endpoints after their exposure
gate. Service repairs and operational documentation do not wait for annotation.
A single frozen, adequately
powered held-out run records H1 (sanity gate), H2 and H5 (confirmatory, Holm)
as pass, negative or inconclusive, with H3, H4 and E1 reported, whatever the
outcome.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: blocked
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: reopen after at least one unfinished task becomes dependency-ready and has no substantive known_blockers
tasks:
  OD-0.0:
    title: Given an empty package, when `cargo check --all-targets` runs at the repository root, then the `ontodoc` lib and bin compile and `ontocode-rs/` is not built
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on: []
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitignore
    - src/lib.rs
    - src/main.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - rust-toolchain.toml
    - .gitignore
    - src/lib.rs
    - src/main.rs
    non_goals:
    - Any retrieval, storage or MCP logic.
    - Modifying anything under `ontocode-rs/`.
    - A multi-crate workspace.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo check --all-targets
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-00 (sha256 524b2be1019c)'
    rollback:
    - Delete the listed files.
    stop_conditions:
    - A dependency version is unavailable offline and network fetch fails.
    - Cargo resolves `ontocode-rs/Cargo.toml` as part of the build.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-00 (sha256 9547531a405b)'
    dod_status: pass
  OD-1.0:
    title: Given a fixture tree with ignored, oversized, non-UTF-8 and symlinked files, when the corpus walk runs, then only eligible `.md`/`.markdown` files are returned in sorted relative-path order and each skip is a typed diagnostic
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-0.0
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/corpus.rs
    - src/lib.rs
    - tests/fixtures/corpus/
    allowed_write_set:
    - src/corpus.rs
    - src/lib.rs
    - tests/fixtures/corpus/
    non_goals:
    - Parsing Markdown.
    - Following symlinks or reading outside the root.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- corpus::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-10 (sha256 d764553f835b)'
    rollback:
    - Revert `src/corpus.rs` and the module line in `src/lib.rs`.
    stop_conditions:
    - Beneath-root reads need platform code beyond std on Linux.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-10; verbatim scalar SHA-256 84105a9a30bb4cb0df0230b1973e65af6e0cae529c1917caa2ac7362c7f4ab00.
    dod_status: pass
  OD-1.1:
    title: Given Markdown fixtures with ATX and setext headings, front matter, nested fences and duplicate headings, when parsed, then sections and chunks carry GitHub slugs, byte and line spans, span SHA-256, budget-independent section ids and chunk ids of section id plus ordinal
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.0
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/parse.rs
    - src/chunk.rs
    - src/lib.rs
    - tests/fixtures/markdown/
    allowed_write_set:
    - src/parse.rs
    - src/chunk.rs
    - src/lib.rs
    - tests/fixtures/markdown/
    non_goals:
    - Tiers and pseudo-documents (OD-1.2).
    - Resolving links (OD-1.3).
    - MDX or HTML rendering.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- parse:: chunk::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-11 (sha256 a43869c6d173)'
    rollback:
    - Revert the two modules and the module lines in `src/lib.rs`.
    stop_conditions:
    - pulldown-cmark offsets cannot express a required span.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-11; verbatim scalar SHA-256 7e8cf4743772b284dd61cbc6d29142bd086989f826e44f8479a2251bba1e9ff4.
    dod_status: pass
  OD-1.2:
    title: Given parsed sections, when tiers are derived, then each chunk has L0 at most 200 bytes and L1 at most 1,500 bytes, each document has a structural pseudo-document, and every tier byte is copied from the source
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/tiers.rs
    - src/lib.rs
    allowed_write_set:
    - src/tiers.rs
    - src/lib.rs
    non_goals:
    - Generated or model-written summaries.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- tiers::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-12 (sha256 09371c03c70e)'
    rollback:
    - Revert `src/tiers.rs`.
    stop_conditions:
    - A tier cannot be built without rewriting source text.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-12; verbatim scalar SHA-256 33b3afd538ffab1bf054f5c6c3b521a8a7ff9c59bd76b07c185df4f6724c2513.
    dod_status: pass
  OD-1.3:
    title: Given a multi-file fixture with relative links, anchors, broken links and inline code spans, when the graph is built, then CONTAINS, NEXT, LINKS_TO (projected to first chunks) and MENTIONS edges match the golden edge list and broken links are diagnostics, not edges
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/graph.rs
    - src/lib.rs
    - tests/fixtures/graph/
    allowed_write_set:
    - src/graph.rs
    - src/lib.rs
    - tests/fixtures/graph/
    non_goals:
    - Resolving MENTIONS to code symbols.
    - External URL fetching.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- graph::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-13 (sha256 887ecbfa6575)'
    rollback:
    - Revert `src/graph.rs`.
    stop_conditions:
    - Link resolution needs repository knowledge beyond the corpus file set.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-13; verbatim scalar SHA-256 a50ec0a8de1969b38b6a7f6551d7891ce2eddcb0116af70919cc61c61648acad.
    dod_status: pass
  OD-1.4:
    title: Given a built corpus, when a generation is published, interrupted, reloaded with a corrupted artifact, or checked after a file edit, then publication is atomic, corruption is rejected, overflow publishes nothing, only index-time config affects the generation id, and freshness reports stale paths
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.2
    - OD-1.3
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/store.rs
    - src/lib.rs
    allowed_write_set:
    - src/store.rs
    - src/lib.rs
    non_goals:
    - Incremental re-indexing.
    - Non-Linux publication fallbacks.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- store::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-14 (sha256 dd4b83ddbe46)'
    rollback:
    - Revert `src/store.rs`; delete `.ontodoc/` in fixtures.
    stop_conditions:
    - Atomic rename semantics differ on the target filesystem.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-14; verbatim scalar SHA-256 e9f76e2b54ca87413b60d0cef10cf33e87212ccb4f23f4295a042ee9417d382c.
    dod_status: pass
  OD-1.5:
    title: Given an indexed fixture, when the tokenizer, B0, B1 and abstention run, then B0 returns prior-attempt substring order, B1 BM25 scores match hand-computed values to 1e-9, chunk rankings map to section rankings by first occurrence, and abstention on normalised raw BM25 returns empty with a typed reason
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.4
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/search/mod.rs
    - src/search/tokenize.rs
    - src/search/baseline.rs
    - src/search/bm25.rs
    - src/lib.rs
    allowed_write_set:
    - src/search/mod.rs
    - src/search/tokenize.rs
    - src/search/baseline.rs
    - src/search/bm25.rs
    - src/lib.rs
    non_goals:
    - PPR, B2, B3, scope and explain (OD-1.6).
    - Embeddings, rerankers, query rewriting or LLM calls.
    - Tuning any parameter.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- search::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-15 (sha256 7771b7ef1c83)'
    rollback:
    - Revert the listed search files.
    stop_conditions:
    - B0 behaviour cannot be reimplemented from the audit description without the host crates.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-15; verbatim scalar SHA-256 7fa3923af3e5b88ea6d71e81ae92b354d4567bff2f9c8bba1b5ca43922d57bf1.
    dod_status: pass
  OD-1.6:
    title: Given an indexed fixture, when B2 and B3 run, then PPR over undirected LINKS_TO and NEXT matches hand-computed values to 1e-6, a 0 and beta 0 reproduce B1 exactly, degenerate min-max yields 0, scope restricts candidates and seeds, and explain lists score components
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.5
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/search/ppr.rs
    - src/search/fusion.rs
    - src/search/mod.rs
    allowed_write_set:
    - src/search/ppr.rs
    - src/search/fusion.rs
    - src/search/mod.rs
    non_goals:
    - CONTAINS edges in PPR.
    - Combined B2+B3 fusion.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- search::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-16 (sha256 07ccbfec2ff2)'
    rollback:
    - Revert the listed search files.
    stop_conditions:
    - B2 or B3 at zero weight cannot reproduce B1 without changing B1.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-16; verbatim scalar SHA-256 475fed0a400d0a760f17408dbfb8cce0958de0b6fc5aed58b6dad9aaf94e7acb.
    dod_status: pass
  OD-1.7:
    title: Given citations from a generation, when the cited file is unchanged, shifted by an edit outside the span, edited inside the span, or deleted, then verify returns valid, moved with the new span, stale and missing respectively
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.4
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cite.rs
    - src/lib.rs
    allowed_write_set:
    - src/cite.rs
    - src/lib.rs
    non_goals:
    - Answer-level claim grounding.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- cite::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-17 (sha256 69bf8d1aa46d)'
    rollback:
    - Revert `src/cite.rs`.
    stop_conditions:
    - A citation state cannot be decided from the stored span and current bytes alone.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-17; verbatim scalar SHA-256 66064a5bec70a22e25901c0e531529b0d34af51a24977a94b9ab2db6fc4b254f.
    dod_status: pass
  OD-1.8:
    title: Given ranked results and a byte budget, when context assembly runs, then every result first receives L0, leftover budget upgrades results in rank order to L1 then L2, an upgrade that does not fit keeps the prior tier, and output never exceeds the budget or truncates a tier
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.5
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/assemble.rs
    - src/lib.rs
    allowed_write_set:
    - src/assemble.rs
    - src/lib.rs
    non_goals:
    - Session ledgers or de-duplication across calls.
    - Any quality hypothesis or comparison arm.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- assemble::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-18 (sha256 1fab700da595)'
    rollback:
    - Revert `src/assemble.rs`.
    stop_conditions:
    - A tier for a chunk is missing from the generation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-18; verbatim scalar SHA-256 739cfa451ac96888960d0ebf718da124cb442ae0e6d3f1b4ffb6116cfb9eee91.
    dod_status: pass
  OD-2.0:
    title: Given a fixture repository, when `ontodoc index`, `search`, `read`, `verify` and `status` run, then each exits 0 with deterministic JSON output and a missing index exits non-zero with a typed message
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.6
    - OD-1.7
    type: code
    validation_tier: integration
    impact_risk: low
    owner_files:
    - src/main.rs
    - src/cli.rs
    - src/lib.rs
    - tests/cli.rs
    allowed_write_set:
    - src/main.rs
    - src/cli.rs
    - src/lib.rs
    - tests/cli.rs
    non_goals:
    - The `eval` subcommands (EVAL-1.1, EVAL-1.2).
    - Shell completion or config files.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-20 (sha256 89c0caa5f09f)'
    rollback:
    - Revert the listed files.
    stop_conditions:
    - CLI output cannot be made deterministic without changing core types owned by earlier tasks.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-20; verbatim scalar SHA-256 3a0eaba1c31d909404d5dbd9aadd9198c9d77977fe704b25680bf79435d86978.
    dod_status: pass
  OD-2.1:
    title: Given a running `ontodoc serve` on an in-process duplex transport, when a client lists and calls the five tools, then each returns schema-valid bounded JSON with citations and freshness, and oversized results set `truncated`
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.0
    - OD-1.8
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - src/lib.rs
    - tests/mcp.rs
    - Cargo.toml
    - Cargo.lock
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - src/lib.rs
    - tests/mcp.rs
    - Cargo.toml
    - Cargo.lock
    non_goals:
    - HTTP transport, auth, write or index tools.
    - MCP resources and prompts.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-21 (sha256 319d39a14add)'
    rollback:
    - Revert the listed files.
    stop_conditions:
    - rmcp 1.7 lacks a duplex/in-memory transport usable from tests.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#od-21; verbatim scalar SHA-256 9e8e0b763c3909e92d31cfeb71214732f56daae12c24a2cb76ceecc1f611ac8a.
    dod_status: pass
  EVAL-1.0:
    title: Given hand-computed reference cases, when nDCG@k, Recall@k, MRR@k, condensed nDCG', corpus-stratified paired bootstrap and randomization, Holm, Wilson, quadratic-weighted kappa and the power formula run, then each matches the reference within 1e-9 or the stated Monte Carlo tolerance
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-0.0
    type: code
    validation_tier: unit-fast
    impact_risk: medium
    owner_files:
    - src/eval/mod.rs
    - src/eval/metrics.rs
    - src/eval/stats.rs
    - src/lib.rs
    allowed_write_set:
    - src/eval/mod.rs
    - src/eval/metrics.rs
    - src/eval/stats.rs
    - src/lib.rs
    non_goals:
    - Statistics crates; implement with std and an in-crate SplitMix64.
    - Reading run files (EVAL-1.1).
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib -- eval::'
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-10 (sha256 8f0a6c125bfd)'
    rollback:
    - Revert the listed eval files.
    stop_conditions:
    - A hand-computed reference value disagrees with the implementation; stop and record the discrepancy instead of adjusting the test.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-10; verbatim scalar SHA-256 0744b5ad8005c7ed8e71bd1ae9a3a2d318568272006a0191bcf4159c504feac3.
    dod_status: pass
  EVAL-1.1:
    title: Given fixture queries, qrels and a config, when `ontodoc eval split`, `pool`, `run` and `score` execute, then splits and pools regenerate byte for byte, run writes TREC-style section rankings and a report with identity fields, score re-scores saved runs identically, and the test split is refused without `--confirm-test-split`
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.0
    - OD-2.0
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    allowed_write_set:
    - src/eval/run.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    non_goals:
    - Integrity gates, sweep and power (EVAL-1.2).
    - Authoring the real benchmark (EVAL-2.0).
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-11 (sha256 3fd641dfb81a)'
    rollback:
    - Revert the listed files.
    stop_conditions:
    - Section ids are not stable across generations of the same pinned corpus.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-11; verbatim scalar SHA-256 54077fb704190a1ea71e65e608ee733655c0d688f81400cf529a7b9799787214.
    dod_status: pass
  EVAL-1.2:
    title: Given fixture benchmarks with planted defects, when `ontodoc eval check-benchmark` and `eval sweep` run, then every integrity gate fails on its planted defect, sweep refuses non-dev splits, misses are attributed to one stage each, and the frozen config is refused on a dirty tree or hash mismatch
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.1
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_gates.rs
    - tests/fixtures/eval_gates/
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - src/cli.rs
    - tests/eval_gates.rs
    - tests/fixtures/eval_gates/
    non_goals:
    - Real benchmark data.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_gates
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-12 (sha256 4bbfa343da08)'
    rollback:
    - Revert the listed files.
    stop_conditions:
    - A gate cannot be checked from repository files and the pinned generations alone.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-08: proceed tasks'
        validation_summary: See audit_session-2026-11-11-tracker-evidence-compaction.md#eval-12; verbatim scalar SHA-256 3948218febdce9fdf51c8578f763e817329dc74a25aaa46ad16e7fdae53dbfbc.
    dod_status: pass
  EVAL-1.3:
    title: Given missing or invalid benchmark evidence, when a measurement or held-out scoring is requested, the evaluation entry point refuses before publishing results; valid evidence and matching frozen identity permit the measured operation
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    - EVAL-1.4
    - EVAL-1.5
    - EVAL-1.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    - src/store.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - src/eval/mod.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    - tests/fixtures/eval/
    - src/store.rs
    non_goals:
    - Real benchmark data, annotation or held-out measurement.
    - Model-only relevance labels counted as human judgments.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-13 (sha256 9b367418be9b)'
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - Correctness requires weakening an ADR integrity, freeze or annotation requirement.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed instructions 2026-11-11 (EVAL-1.3 slices 1-9); src/store.rs unused_unit one-line fix added to write set after repeated approval request and user proceed.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-13 (sha256 b5512fc122f2)'
    dod_status: pass
  EVAL-2.0:
    title: Given the pinned corpora, when the benchmark is assembled, then `eval/` holds pinned corpus metadata, stratified queries with operational strata and seeded dev/test split, and grid-wide top-10 pools with the judgement count reported
    status: BLOCKED
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    - OD-1.6
    - EVAL-1.3
    type: test
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - eval/corpora.json
    - eval/config.json
    - eval/gates.json
    - eval/splits.json
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    - eval/pools/
    - eval/preparation.json
    allowed_write_set:
    - eval/corpora.json
    - eval/config.json
    - eval/gates.json
    - eval/splits.json
    - eval/queries.jsonl
    - eval/query-review.jsonl
    - eval/dev-queries.jsonl
    - eval/test-queries.jsonl
    - eval/pools/
    - eval/preparation.json
    non_goals:
    - Relevance judgements (EVAL-2.1).
    - Link-derived silver qrels (rejected in the ADR).
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo run --release -- eval check-benchmark eval/config.json --stage pools --gates eval/gates.json --query-review eval/query-review.jsonl --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256" --preparation-out eval/preparation.json
    expected_evidence:
    - Each corpus entry has URL, 40-character commit, licence and local path.
    - Test split has at least 150 answerable and at least 30 off-topic queries (so at least 270 queries overall at the 1:2 dev:test split); strata assigned by the ADR definitions at writing time; each model-drafted query has a substantive GPT-6-Astra review tied to its final text, with provenance and limitations recorded before pooling.
    - Pool judgement count reported; if above 6,000 the pool is regenerated with the pre-registered grid {0.2, 0.4} and both counts are recorded.
    - Combined authoring inputs are preparation-only; config.partitions binds separate dev/test query payloads and the preparation evidence defined by EVAL-1.6. No execution command falls back to combined inputs.
    - The trusted operator supplies manager-owned eval/audit_session-query-approval.md with authentic query-review approval and its exact digest before the first pool/ranking invocation; the data worker neither writes nor approves that audit. Pools-stage preparation has no qrels/annotation evidence and never admits measurement. The manager audit records deterministic full-grid/fallback pool regeneration and manifest hash; supplied flags alone do not establish reviewer independence or chronology.
    rollback:
    - Delete the listed files.
    stop_conditions:
    - No permissively licensed second corpus is available.
    - Queries cannot be written without copying target text.
    known_blockers:
    - 'Parked by owner 2026-11-11 pending corpora, a trusted operator and two human annotators (audit_session-2026-11-11-owner-decisions.md). EVAL-1.3 DONE 2026-11-11. Remaining owner gate: real benchmark assembly needs the trusted operator (human) to select pinned permissive corpora, supply the manager-owned query-approval audit and digest before the first pool run, and commit human annotators for EVAL-2.1. GPT-6-Astra review is authorized for queries only and never counts as human annotation; generic proceed does not resolve this gate.'
  EVAL-2.1:
    title: Given approved pools, when two humans independently label every pair for the predeclared overlap of at least 30% of answerable queries, then qrels and provenance pass qrels-stage preparation with raw-grade agreement and trusted approval
    status: OPEN
    classification: proof-only
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-2.0
    type: inventory
    validation_tier: integration
    impact_risk: high
    owner_files:
    - eval/qrels.jsonl
    - eval/annotation.md
    - eval/annotation.jsonl
    - eval/preparation.json
    - eval/audit_session-preparation.md
    - eval/dev-qrels.jsonl
    - eval/test-qrels.jsonl
    allowed_write_set:
    - eval/qrels.jsonl
    - eval/annotation.md
    - eval/annotation.jsonl
    - eval/preparation.json
    - eval/audit_session-preparation.md
    - eval/dev-qrels.jsonl
    - eval/test-qrels.jsonl
    non_goals:
    - Model-only labels counted as human judgements.
    - Judging off-topic queries.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo run --release -- eval check-benchmark eval/config.json --stage qrels --gates eval/gates.json --query-review eval/query-review.jsonl --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256" --annotation eval/annotation.jsonl --annotation-notes eval/annotation.md --preparation-out eval/preparation.json
    expected_evidence:
    - Every pooled answerable query/section pair has a label; annotators saw no system identity or score.
    - '`eval/annotation.md` records the guideline, pseudonymous annotator ids, weighted kappa with 95% CI, prevalence and the adjudication rule.'
    - Preparation exports separate dev-qrels.jsonl and test-qrels.jsonl committed by config.partitions and EVAL-1.6's exact-byte admission record; combined qrels remain preparation-only.
    - ADR9 companion records bind each qrel to exact section/span, independent raw human grades, model-draft human review and adjudication; notes record blinded assignments and verified pseudonymous human identities. Derive at least 30% answerable-query overlap, quadratic-weighted kappa >=0.6, seeded query-cluster CI and prevalence. Undefined agreement or fewer than two overlap queries refuses. Operator audits the exact qrels-stage manifest hash and pins it for execution; no model is a human.
    rollback:
    - Delete the listed files.
    stop_conditions:
    - No second human annotator is available; record the blocker instead of substituting a model.
    - Weighted kappa below 0.6; revise the guideline and relabel before any run.
    known_blockers: []
  EVAL-3.0:
    title: Given the dev split, when one `ontodoc eval run` of B0 and B1 executes, then `report.json` exists with every identity field, judged@10 for both systems, and a second run is byte-identical
    status: OPEN
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-2.1
    type: measurement
    validation_tier: integration
    impact_risk: low
    owner_files:
    - $ONTODOC_BENCH/results/dev-proof/
    - $ONTODOC_BENCH/results/dev-proof-repeat/
    - $ONTODOC_BENCH/audit_session-dev-proof.md
    allowed_write_set:
    - $ONTODOC_BENCH/results/dev-proof/
    - $ONTODOC_BENCH/results/dev-proof-repeat/
    - $ONTODOC_BENCH/audit_session-dev-proof.md
    non_goals:
    - The test split.
    - Parameter tuning.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-proof"'
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-proof-repeat"'
    - diff -r "$ONTODOC_BENCH/results/dev-proof" "$ONTODOC_BENCH/results/dev-proof-repeat"
    expected_evidence:
    - '`$ONTODOC_BENCH/results/dev-proof/report.json` exists and a separately named rerun produces byte-identical artifacts; existing output directories refuse overwrite.'
    - Trusted preparation has already exported/recaptured and approved the canonical external snapshot; audit binds preparation hash, complete benchmark/partition commitments and matching committed code/binary. Dev proof loads selected dev payloads only, never the full pool/provenance/test payloads. No scientific improvement follows from this instrument proof.
    rollback:
    - Preserve the recorded proof; delete only an explicitly owned disposable dev rerun.
    stop_conditions:
    - Report identity fields are missing or rerun output differs.
    - Canonical external benchmark snapshot and frozen binary paths are not pinned in the audit; EVAL-1.3 success/admission proof is missing.
    - Clean committed code, matching audited binary and preparation/provenance handoff are unavailable; HEAD exists (b6e3f3e) but the tree is dirty; freezing needs a clean committed checkpoint supplied by the owner.
    known_blockers: []
  EVAL-3.1:
    title: Given the dev split only, when PPR alpha, B3 beta and the abstention threshold are swept and the power calculation runs, then the external frozen export and fixed registration bind selected values, power, commit and hashes before test access
    status: OPEN
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.0
    type: measurement
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - $ONTODOC_BENCH/config.json
    - $ONTODOC_BENCH/preparation.json
    - $ONTODOC_BENCH/frozen-config.toml
    - $ONTODOC_BENCH/results/dev-sweep/
    - $ONTODOC_GIT_COMMON/ontodoc-eval/registration.json
    - $ONTODOC_BENCH/audit_session-preregistration.md
    allowed_write_set:
    - $ONTODOC_BENCH/config.json
    - $ONTODOC_BENCH/preparation.json
    - $ONTODOC_BENCH/frozen-config.toml
    - $ONTODOC_BENCH/results/dev-sweep/
    - $ONTODOC_GIT_COMMON/ontodoc-eval/registration.json
    - $ONTODOC_BENCH/audit_session-preregistration.md
    non_goals:
    - Any test-split evaluation.
    - Sweeping the chunk budget or any grid beyond the ADR.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval sweep "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split dev --out "$ONTODOC_BENCH/results/dev-sweep/sweep.json"'
    - '"$ONTODOC_FROZEN_BIN" eval check-benchmark "$ONTODOC_BENCH/config.json" --stage qrels --gates "$ONTODOC_BENCH/gates.json" --query-review "$ONTODOC_BENCH/query-review.jsonl" --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256" --annotation "$ONTODOC_BENCH/annotation.jsonl" --annotation-notes "$ONTODOC_BENCH/annotation.md" --preparation-out "$ONTODOC_BENCH/preparation.json"'
    - '"$ONTODOC_FROZEN_BIN" eval freeze "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --threshold "$ONTODOC_DEV_THRESHOLD" --repo . --out "$ONTODOC_BENCH/frozen-config.toml"'
    - '"$ONTODOC_FROZEN_BIN" eval check-frozen "$ONTODOC_BENCH/config.json" "$ONTODOC_BENCH/frozen-config.toml" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --repo .'
    expected_evidence:
    - Grid a {0, 0.1, ..., 0.5}, beta {0, 0.1, ..., 0.5} (or the recorded fallback {0.2, 0.4}); threshold set for at most 10% false abstention on answerable dev queries.
    - Sweep using the original approved preparation; select only dev parameters in config.json. Then run the listed qrels-stage check explicitly as trusted-operator preparation, not ordinary dev; verify unchanged query/review/label/split bytes and reapprove the resulting manifest in the external audit. Update ONTODOC_PREPARATION_SHA256 from that approval before freeze/check. Ordinary dev never opens held-out data to refresh approval. Record exact config, complete partition/index commitments, both selected digests and h5-then-h2-v1 policy; never tune/rewrite held-out payloads.
    - required n = max(n_H2, n_H5, 150) is at most the preparation-bound test answerable count; exact frozen export and registration SHA-256 hashes are recorded in an audit session before EVAL-3.2.
    rollback:
    - Preserve the write-once registration and frozen evidence. Superseding custody requires a fresh approved experiment and separately scoped transition; never delete it to retry.
    stop_conditions:
    - Any code path reads test-split qrels during the sweep.
    - Required n exceeds the answerable test queries; write and label more test queries (reopen EVAL-2.0/2.1) before freezing.
    known_blockers:
    - ADR revision 6 resolves artifact location. EVAL-1.3 (DONE) proved synthetic captured freeze/check and attempt success paths; real runs still need EVAL-2.x data. ONTODOC_DEV_THRESHOLD must bind the dev selection, never a guess; external benchmark/custody/binary paths must be pinned and preregistered.
  EVAL-3.2:
    title: Given approved frozen identity, when the sole held-out run executes, then accepted custody binds every reported hypothesis and the h5-then-h2-v1 recommendation without changing application defaults
    status: OPEN
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.1
    - OD-2.1
    type: measurement
    validation_tier: integration
    impact_risk: high
    owner_files:
    - $ONTODOC_BENCH/results/test-v1/
    - $ONTODOC_GIT_COMMON/ontodoc-eval/attempt-started.json
    - $ONTODOC_GIT_COMMON/ontodoc-eval/attempt-result.json
    - $ONTODOC_BENCH/audit_session-held-out.md
    allowed_write_set:
    - $ONTODOC_BENCH/results/test-v1/
    - $ONTODOC_GIT_COMMON/ontodoc-eval/attempt-started.json
    - $ONTODOC_GIT_COMMON/ontodoc-eval/attempt-result.json
    - $ONTODOC_BENCH/audit_session-held-out.md
    non_goals:
    - Re-running the test split with a changed config.
    - Reporting only passing hypotheses.
    required_validation:
    - '"$ONTODOC_FROZEN_BIN" eval run "$ONTODOC_BENCH/config.json" --preparation "$ONTODOC_BENCH/preparation.json" --preparation-sha256 "$ONTODOC_PREPARATION_SHA256" --split test --confirm-test-split --out "$ONTODOC_BENCH/results/test-v1"'
    expected_evidence:
    - Identity fields match the frozen config; judged@10 >= 0.95 for every system; condensed nDCG' reported.
    - Pooled and per-corpus results, per-stage miss attribution, and an audit session recording the outcome and marking the test split burned if any later tuning uses its errors.
    - Report H1, both H2/H5 Holm/CI decisions, H3/H4/E1 and all limitations, regardless of gain. Freeze h5-then-h2-v1 before access; H5 pass recommends hier/frozen beta, else H2 pass graph/frozen alpha, else lexical/no-op. Omitted threshold stays disabled. This task writes no application defaults; OD-4.1 alone implements an accepted recommendation.
    - Audit binds canonical external snapshot, preparation approval, frozen export/registration and started/completed hashes. Reserve before test payload access; every failure consumes the attempt. Visible output without matching completed custody is unaccepted, including publication/completion failure; preserve and audit it, never retry/reset.
    rollback:
    - None for the run itself; a superseding run requires a new pre-registered configuration, a fresh test split and its own audit record.
    stop_conditions:
    - Frozen config hash, commit or binary hash mismatch, or a dirty tree.
    - H1 fails; stop and audit the pipeline before interpreting H2 or H5.
    known_blockers: []
  OD-4.1:
    title: Given an accepted audited held-out report, apply only its predeclared h5-then-h2-v1 default recommendation with CLI/MCP parity, or record an explicit no-op when neither arm passes
    status: OPEN
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.2
    type: code
    validation_tier: release
    impact_risk: medium
    owner_files:
    - src/cli.rs
    - tests/cli.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/cli.rs
    - tests/cli.rs
    - tests/mcp.rs
    non_goals:
    - Ranking algorithm changes, mutable evaluation reads on the retrieval path, or promotion by largest observed held-out gain.
    - Default abstention, new MCP fields/tools or changes to explicit per-request mode/weight semantics.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp
    - cargo fmt --check
    - just --justfile ../ontocode/justfile compiler-lane-detached "$ONTODOC_PROMOTION_JOB" sh -c "CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings"
    expected_evidence:
    - Verify the accepted completed report, publication, frozen identity and audited recommendation. Apply H5-first mode plus frozen beta, else H2 mode plus frozen alpha, else record no code change through manager-owned closeout. Invalid or H1-failing output authorizes no promotion.
    - Shared SearchMode/SearchRequest in src/cli.rs owns clap and serde defaulting and omitted weights; src/mcp.rs consumes it and src/search/mod.rs requires explicit mode, so neither needs production edits. Preserve explicit requests and zero weights; absent threshold remains None. Generated MCP schema, CLI and serde omissions agree, and implicit ranking matches explicit approved parameters on synthetic fixtures.
    - Manager-owned audit records the exact recommendation and default constants, or the no-op reason; no second held-out run or post-result tuning. No-op may use unchanged compatibility checks and needs no source edit.
    - Run the no-cd external compiler-lane wrapper from this package root only, with a manager-assigned unique absent ONTODOC_PROMOTION_JOB. Harvest its terminal result with `just --justfile ../ontocode/justfile lane-result "$ONTODOC_PROMOTION_JOB"`; startup exit0 is not lint success. Never build the donor snapshot.
    rollback:
    - Revert only default/test edits without altering frozen code, custody or benchmark artifacts.
    stop_conditions:
    - Missing or mismatched accepted custody, audit, preregistered policy or frozen parameters.
    - Promotion needs an algorithm/schema/threshold-default change outside this card.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
        coder-worker-test-challenger: null
  OD-4.0:
    title: Given the held-out results, when the closeout is written, then README, AGENTS.md and memory-bank files describe install, MCP client setup, tool contracts and each hypothesis outcome with its CI and a link to the report
    status: OPEN
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-3.2
    - OD-4.1
    - HTTP-2
    - OD-3.9
    type: documentation
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - README.md
    - AGENTS.md
    - .memory-bank/
    allowed_write_set:
    - README.md
    - AGENTS.md
    - .memory-bank/
    non_goals:
    - Claims beyond the recorded intervals or for inconclusive arms.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test
    expected_evidence:
    - README states each H2/H5 outcome as pass, negative or inconclusive with its CI and links the pinned external held-out report/audit identity under ADR6.
    - Memory bank updated; this plan closed.
    rollback:
    - Revert the documentation edits.
    stop_conditions:
    - Held-out report is missing or its identity fields do not match the frozen config.
    known_blockers: []
  OD-3.0:
    title: Given duplicate section or chunk IDs, publication refuses before any store mutation and loading rejects the malformed generation
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-1.1
    - OD-1.4
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    allowed_write_set:
    - src/store.rs
    non_goals:
    - Changing ID syntax, schema, graph/ranking algorithms or evaluation guards.
    - Full artifact semantic validation, path/read caps, HTTP or filesystem containment repairs.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-30 (sha256 484829166e8d)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - The repair requires a new identity format or disk schema.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: review and challenge, consolidate one plan and proceed; GPT subagents requested.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-30 (sha256 96e92224ead1)'
    dod_status: pass
  OD-3.1:
    title: Given canonical-byte and empty-slug conflicts, record an explicit compatible identity/schema decision before changing stored artifacts
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    non_goals:
    - Code changes, listener startup, benchmark execution or implicit acceptance of diagnostics.
    required_validation:
    - project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-31 (sha256 7d01421ae495)'
    rollback:
    - Revert only this decision's document changes.
    stop_conditions:
    - An identity change would relabel an already frozen benchmark without a fresh preregistration.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed with consolidated review-and-repair plan; OD-3.1 selected by strict manager routing.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-31 (sha256 a9f5fc57967c)'
    dod_status: pass
  OD-3.2:
    title: Given identical source bytes and different mtimes, canonical artifacts match byte for byte and distinct Markdown sections have distinct IDs
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/parse.rs
    - src/store.rs
    - src/graph.rs
    - src/tiers.rs
    - src/chunk.rs
    - tests/cli.rs
    allowed_write_set:
    - src/parse.rs
    - src/store.rs
    - src/graph.rs
    - src/tiers.rs
    - src/chunk.rs
    - tests/cli.rs
    non_goals:
    - Ranking tuning, new graph relations, real labels or benchmark regeneration.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --lib
    - CARGO_BUILD_JOBS=4 cargo test --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-32 (sha256 9c63cde6aa1f)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - OD-3.1 has not resolved the format and invalidation decisions.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed with active repair tasks; GPT-6-Astra review retained.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-32 (sha256 104442db0afc)'
    dod_status: pass
  OD-3.3:
    title: Given escaped payloads and internal failures, every complete MCP frame fits the wire cap and exposes no private internal paths
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - tests/mcp.rs
    - tests/cli.rs
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - tests/mcp.rs
    - tests/cli.rs
    non_goals:
    - New tools, HTTP transport or a second response pipeline.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test mcp --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-33 (sha256 2962a75db3e0)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - The locked transport cannot enforce a complete-frame cap without a documented compatibility decision.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; consolidated repair plan, GPT subagent review retained.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-33 (sha256 d5a77f8108a0)'
    dod_status: pass
  OD-3.4:
    title: Given fabricated citations or filesystem replacements, verification reads only current bytes of authorized corpus documents within descriptor-rooted byte bounds
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    - SAFETY-0
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - src/corpus.rs
    - src/cite.rs
    - src/store.rs
    - src/cli.rs
    - src/mcp.rs
    - tests/cli.rs
    - tests/mcp.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - src/corpus.rs
    - src/cite.rs
    - src/store.rs
    - src/cli.rs
    - src/mcp.rs
    - tests/cli.rs
    - tests/mcp.rs
    non_goals:
    - Per-user ACLs, cached freshness substituted for verification, or arbitrary-file access.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib corpus::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cite::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp
    - CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-34 (sha256 6c952f4be76b)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Implementation requires weakening ADR8 root binding, membership, policy parity or unsupported-filesystem refusal.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.4 local implementation, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-34 (sha256 3f716a93d91b)'
    dod_status: pass
  OD-3.5:
    title: Given costly or cancelled requests, input/work budgets reject early and admission remains owned by the actual blocking work until it stops
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.4
    - OD-3.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cite.rs
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/cite.rs
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    non_goals:
    - Timeout-only containment, silent semantic weakening, or new ranking methods.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cite::'
    - CARGO_BUILD_JOBS=4 cargo test --test mcp
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-35 (sha256 4263a38ddbb4)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Implementation requires a network/session boundary or weakening ADR8 actual-work ownership/local limits.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.5 local execution, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-35 (sha256 7479892e3a84)'
    dod_status: pass
  OD-3.6:
    title: Given freshness scans or malformed store files, current-byte checks retain bounded memory and loaders reject excessive reads before allocation
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.4
    - OD-3.8
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/corpus.rs
    - src/store.rs
    - src/cli.rs
    - src/search/mod.rs
    - tests/cli.rs
    allowed_write_set:
    - src/corpus.rs
    - src/store.rs
    - src/cli.rs
    - src/search/mod.rs
    - tests/cli.rs
    non_goals:
    - Watcher infrastructure, TTL freshness or per-request index rebuilds.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib search::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib corpus::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-36 (sha256 b662568e0292)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - An optimization changes current-byte freshness semantics or requires an unapproved disk-format change.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager selected dependency-ready OD-3.6 local implementation, dispatch_mode none, no lease.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-36 (sha256 79955109100a)'
    dod_status: pass
  EVAL-1.4:
    title: Given dev-only execution, split-isolated immutable inputs validate and compute from the same bounded bytes without opening held-out payloads
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.2
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/corpus.rs
    - src/cli.rs
    - src/store.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/corpus.rs
    - src/cli.rs
    - src/store.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - External audit registry service, real benchmark preparation/labels or measurement enablement.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-14 (sha256 10419b329610)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Dev execution must open held-out records or provenance is replaced by self-asserted evidence.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; continue selected EVAL-1.4 and its recorded traversal seam.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-14 (sha256 e7fbe946abb4)'
    dod_status: pass
  EVAL-1.5:
    title: Given frozen registration and a held-out attempt, the exclusive durable attempt and exact saved-run bytes remain bound to the approved identity
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.4
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    - src/eval/gates.rs
    - src/eval/sweep.rs
    - tests/common/
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - Real held-out execution, model annotations counted as human, or claims that mutable local receipts provide external independent custody.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-15 (sha256 416994b20cb2)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Authoritative preregistration/attempt location or scored-byte identity is unresolved.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed with active EVAL-1.5; its explicit ADR conflict-resolution authority and manager documentation scope recorded in Decisions And Non-Goals.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-15 (sha256 5760454f4841)'
    dod_status: pass
  HTTP-0:
    title: Given accepted folder-scoped HTTP, decide exposure and numerical resource/containment contracts before implementing network delivery
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-2.1
    - SAFETY-0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/project_architecture.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    non_goals:
    - Code changes, listener startup, benchmark execution or implicit acceptance of diagnostics.
    required_validation:
    - project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    expected_evidence:
    - Record loopback versus authenticated trusted ingress deployment; origin/host/DNS-rebinding controls, session binding and error/log privacy.
    - Set numeric network/frame, corpus-count, session, aggregate multi-corpus and global work/load/memory budgets; cancellation permits belong to actual work.
    - Reuse SAFETY-0 containment, reader, membership and local budgets without weakening them; locked rmcp remains the only transport stack.
    - Folder encoding, unknown/unindexed readiness and no-fallback rules remain unchanged; diagnostics and retrieval hypotheses stay out of scope.
    rollback:
    - Revert only this decision's document changes.
    stop_conditions:
    - Network exposure requires an authentication or deployment choice not supplied by the service owner.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (round-4 decision packet); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-0 (sha256 73304d15dae2)'
    dod_status: pass
  HTTP-1:
    title: Given docs_share folders, startup produces bounded sorted independent corpus endpoints and stores without cross-corpus fallback
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-1
    - OD-3.2
    - OD-3.4
    - OD-3.6
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/mcp.rs
    - src/cli.rs
    - src/store.rs
    - tests/mcp.rs
    non_goals:
    - Listener startup, live discovery, per-user tenancy, merged indexes or new tools.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test mcp --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#http-1 (sha256 f598494d7abc)'
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Folder encoding, corpus-cap or no-fallback rules in the accepted folder ADR are insufficient; listener, exposure, auth and session decisions stay in HTTP-0/HTTP-2.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; HTTP-1 split from the HTTP-0 exposure gate in audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-1 (sha256 fafa9305239f)'
    dod_status: pass
  HTTP-2:
    title: Given configured HTTP delivery, locked rmcp serves endpoint-bound sessions with stdio parity and bounded actual work
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-0
    - HTTP-1
    - PERF-1
    - OD-3.3
    - OD-3.5
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - Cargo.toml
    - Cargo.lock
    - src/main.rs
    - src/cli.rs
    - src/mcp.rs
    - src/http.rs
    - src/lib.rs
    - tests/mcp.rs
    - tests/cli.rs
    - tests/http.rs
    allowed_write_set:
    - Cargo.toml
    - Cargo.lock
    - src/main.rs
    - src/cli.rs
    - src/mcp.rs
    - src/http.rs
    - src/lib.rs
    - tests/mcp.rs
    - tests/cli.rs
    - tests/http.rs
    non_goals:
    - New MCP tools, retrieval-network dependencies, anonymous exposure or scientific quality claims.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test mcp --test cli --test http
    - 'CARGO_BUILD_JOBS=4 cargo test --lib http:': null
    - 'CARGO_BUILD_JOBS=4 cargo test --lib mcp:': null
    expected_evidence:
    - Enable only the approved locked rmcp HTTP feature and minimal required transport dependencies; reuse the five stdio handlers, no REST ranker.
    - Synthetic listener tests prove endpoint/session isolation, same-corpus stdio parity, complete wire bounds, origin/host/admission rejection, cancellation ownership and controlled drain.
    - Publication, corruption or reload in one corpus never replaces a sibling index; no production listener is exposed by tests.
    rollback:
    - Revert only this task's changes; preserve published source inputs and prior closeouts.
    stop_conditions:
    - Exposure/resource contracts are unresolved or transport parity requires weakening isolation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (all ten owner recommendations); audit_session-2026-11-11-owner-decisions.md; ADR revision 10 HTTP-0 section'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#http-2 (sha256 8822733dd088)'
    dod_status: pass
  OD-3.7:
    title: Given the current service, operational documentation accurately describes installation, five tools and limitations without waiting for retrieval-quality results
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.0
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - README.md
    - AGENTS.md
    - .memory-bank/project_architecture.md
    non_goals:
    - Code changes, listener startup, benchmark execution or implicit acceptance of diagnostics.
    required_validation:
    - project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-37 (sha256 c687263f03e9)'
    rollback:
    - Revert only this decision's document changes.
    stop_conditions:
    - Documentation would claim an unimplemented feature or a measured result without evidence.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: proceed; dependency-ready operational docs after HTTP-0 authority stop.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-37 (sha256 6886f29ec17c)'
    dod_status: pass
  OD-3.8:
    title: Given malformed or oversized store controls and artifacts, bounded readers and serialization refuse before unbounded reads or pointer changes
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.2
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    allowed_write_set:
    - src/store.rs
    non_goals:
    - Descriptor-rooted containment, freshness streaming, total-memory admission, publication commit-point redesign or ranking changes.
    - Real benchmark measurements, HTTP exposure, dependencies, schema migration or new public configuration knobs.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp --test eval_gates --test eval_run
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-38 (sha256 64e5907a8589)'
    rollback:
    - Revert only this task's changes; retain prior evidence and published source data.
    stop_conditions:
    - A byte-bound repair requires changing canonical schema or silently accepting unsafe target replacement.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User instruction 2026-11-10: GPT challenge, consolidate robust performance-oriented plan and proceed; ADR7 and OD-3.8 selected by strict manager.'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-38 (sha256 86df1804dea9)'
    dod_status: pass
  OD-3.9:
    title: Given failures around CURRENT publication, the recorded commit-point contract distinguishes uncommitted failure from committed durability or maintenance failure
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SAFETY-0
    - OD-3.8
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/store.rs
    - src/cli.rs
    - tests/cli.rs
    allowed_write_set:
    - src/store.rs
    - src/cli.rs
    - tests/cli.rs
    non_goals:
    - New store format, general transaction framework, background janitor, implicit rollback or changing retrieval ranking.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#od-39 (sha256 14ef58bb7b32)'
    rollback:
    - Revert only this task's changes; retain prior evidence and published source data.
    stop_conditions:
    - Implementing ADR8 outcomes requires a format/transaction-framework change or reproducing the parallel lock failure requires unrelated global test changes.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict manager OD-3.9 local execution, ADR8.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#od-39 (sha256 59c80667c64a)'
    dod_status: pass
  SAFETY-0:
    title: Given stdio safety gaps, record rooted-reader, membership, work budgets and publication outcomes independently of HTTP deployment
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - OD-3.2
    - OD-3.3
    - OD-3.8
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    non_goals:
    - Code changes, listener exposure, real benchmark preparation/measurement or implicit acceptance of diagnostics.
    required_validation:
    - project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#safety-0 (sha256 a48f83b362d3)'
    rollback:
    - Revert only this task's changes; retain prior evidence and published source data.
    stop_conditions:
    - The decision requires a network exposure choice or substituting models for required human annotators.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User review/challenge/consolidate/proceed instruction; strict manager selected dependency-ready SAFETY-0, local coder-manager documentation execution.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#safety-0 (sha256 6f2d09c4ba9d)'
    dod_status: pass
  EVAL-1.6:
    title: Given incomplete provenance admission, record the trusted preparation handoff and a deterministic default-promotion rule before public evaluation integration
    status: DONE
    classification: architecture-paperwork
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.4
    - EVAL-1.5
    type: documentation
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_SERVICE.md
    - .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    - .memory-bank/project_architecture.md
    non_goals:
    - Code changes, listener exposure, real benchmark preparation/measurement or implicit acceptance of diagnostics.
    required_validation:
    - project_plan_validate .memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#eval-16 (sha256 72776c22ae42)'
    rollback:
    - Revert only this task's changes; retain prior evidence and published source data.
    stop_conditions:
    - The decision requires a network exposure choice or substituting models for required human annotators.
    known_blockers: []
    closeout:
      role_results:
        coder-manager: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: User proceed; strict EVAL-1.6 documentation scope under review/consolidate/proceed instruction.
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#eval-16 (sha256 9a46b1ce4357)'
    dod_status: pass
  FIX-1:
    title: Given verified review defects, moved citations report byte-correct lines, dev resamples are bounded and preparation inputs obey layout isolation
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - EVAL-1.3
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/cite.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/common/
    allowed_write_set:
    - src/cite.rs
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/common/
    non_goals:
    - Ranking, schema, freshness-semantics or ADR changes; real benchmark data.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --lib && CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run --test cli --test mcp
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#fix-1 (sha256 ebecbfa5d886)'
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - A fix requires weakening an ADR integrity or freshness requirement.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-1 (sha256 b985679ee993)'
    dod_status: pass
  SIMPL-1:
    title: Given duplicated evaluation code, freeze computes its dev sweep once, the unreachable check-benchmark run branch is removed and gates reuse the stats quantile, with unchanged behaviour
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-1
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/eval/stats.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/eval/stats.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/eval_run.rs
    non_goals:
    - Removing custody, reservation, preparation binding or any ADR safeguard; frozen export format changes.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval:: && CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run --test cli'
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#simpl-1 (sha256 26fa695d6cd4)'
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - A simplification changes any gate outcome, report byte or refusal order.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-1 (sha256 d518d2e86b14)'
    dod_status: pass
  FIX-2:
    title: Given any accepted corpus name, the frozen config TOML written by freeze parses back to the identical config
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-1
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/sweep.rs
    allowed_write_set:
    - src/eval/sweep.rs
    non_goals:
    - Byte changes for ordinary names; new fields; frozen version bump; real benchmark data.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --lib eval::sweep
    - cargo fmt --check
    expected_evidence:
    - Round-trip regression over control-character, quote, backslash, DEL, non-ASCII and ' = ' corpus names; existing round-trip test unchanged.
    rollback:
    - Revert this task's src/eval/sweep.rs changes.
    stop_conditions:
    - The fix changes the written bytes for ordinary ASCII names.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: proceed (after open-item list offering deferred review fixes); audit_session-2026-11-11-review-simplify-consolidate.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-2 (sha256 9b5ad8ff6c6b)'
    dod_status: pass
  FIX-3:
    title: Given an admitted benchmark, ranking output fits the scoring read budget before the held-out attempt is reserved, vocabulary misses are not attributed to chunk boundaries, and unused scratch plumbing is removed
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-2
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/cli.rs
    - tests/eval_gates.rs
    allowed_write_set:
    - src/eval/run.rs
    - src/eval/sweep.rs
    - src/eval/gates.rs
    - src/cli.rs
    - tests/eval_gates.rs
    non_goals:
    - Weakening custody, reservation, freeze or any ADR integrity rule; new miss stages; real benchmark data.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval:: && CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run --test cli'
    - cargo fmt --check
    expected_evidence:
    - Regression showing an oversized ranking worst case is refused before reservation and generation enforces the same budget.
    - Regression showing a zero-overlap section is attributed lexical_miss, not chunk_boundary; ADR stage set unchanged.
    - check_pool/check_benchmark lose the unused scratch parameter; existing gate tests pass.
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - A fix requires changing the ADR stage set, scoring budget semantics or held-out custody.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed; audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-3 (sha256 c2328a5e8e8d)'
    dod_status: pass
  FIX-4:
    title: Given large graphs or diagnostics, related and status cap neighbours and diagnostic lists before collecting or cloning, with unchanged responses
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-3
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cli.rs
    allowed_write_set:
    - src/cli.rs
    non_goals:
    - Changing response shape, ranking, freshness or the search truncated contract.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli:: && CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp'
    - cargo fmt --check
    expected_evidence:
    - related collects at most MAX_LIST_ITEMS+1 neighbours; status clones at most MAX_LIST_ITEMS diagnostics; truncated flags and response bytes unchanged.
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - Capping changes any response byte for inputs at or below the cap.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed; audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-4 (sha256 76d06c453ef6)'
    dod_status: pass
  SIMPL-2:
    title: Given a held-out run or score, the registered freeze is read and verified once and passed to the later frozen check, with unchanged refusals
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-4
    type: code
    validation_tier: integration
    impact_risk: low
    owner_files:
    - src/eval/run.rs
    allowed_write_set:
    - src/eval/run.rs
    non_goals:
    - Removing any identity, code, custody or power check; reordering checks before payload reads; changing CLI check-frozen.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval:: && CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run --test cli'
    - cargo fmt --check
    expected_evidence:
    - run_reserved and score call check_held_out_code once; held_out_preflight takes the verified FrozenConfig; all existing refusal tests pass unchanged.
    rollback:
    - Revert this task's src/eval/run.rs changes.
    stop_conditions:
    - Any refusal test changes outcome or a check moves after a payload read.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: proceed after stated default (baseline commit plus re-check consolidation); audit_session-2026-11-11-review-round3.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-2 (sha256 eb35c9c13502)'
    dod_status: pass
  FIX-5:
    title: Given TSV-unsafe query ids or an unusable output parent, held-out admission refuses before the attempt is reserved
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-2
    type: code
    validation_tier: integration
    impact_risk: medium
    owner_files:
    - src/eval/run.rs
    allowed_write_set:
    - src/eval/run.rs
    non_goals:
    - Writability probes; weakening custody; reading held-out payloads before reservation; real benchmark data.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval:: && CARGO_BUILD_JOBS=4 cargo test --test eval_gates --test eval_run --test cli'
    - cargo fmt --check
    expected_evidence:
    - validate_queries and the pinned split ids refuse tab/CR/LF ids; regression.
    - external_path requires a directory parent and admission refuses non-NotFound metadata errors; regression.
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - A fix requires reading a held-out payload before reservation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 4); audit_session-2026-11-11-review-round4.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-5 (sha256 a374f24668ca)'
    dod_status: pass
  FIX-6:
    title: Given any published generation, every returned citation is verifiable and path#anchor lookup works for paths containing
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-5
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/store.rs
    - src/cli.rs
    allowed_write_set:
    - src/store.rs
    - src/cli.rs
    non_goals:
    - Changing citation shape, ID scheme, schema version or MCP response fields.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store:: && CARGO_BUILD_JOBS=4 cargo test --lib cli:: && CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp'
    - cargo fmt --check
    expected_evidence:
    - build refuses (CapExceeded) a heading path above the citation input bound (64 headings / 16 KiB); regression.
    - find_section splits path#anchor at the last
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - The cap requires a schema version bump or alters existing valid generations.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 4); audit_session-2026-11-11-review-round4.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-6 (sha256 41710c33dac2)'
    dod_status: pass
  FIX-7:
    title: Given search results and evaluation reports, per-result text truncation is explicit and empty exploratory groups report null statistics
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-2
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/cli.rs
    - src/eval/run.rs
    allowed_write_set:
    - src/cli.rs
    - src/eval/run.rs
    non_goals:
    - Changing ranking, assembly order, schema version or the confirmatory decision rule.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib cli::'
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp --test eval_run
    - cargo fmt --check
    expected_evidence:
    - SearchResult.text_truncated from stored l0_truncated/l1_truncated (L2 false); regression search_results_carry_the_stored_tier_truncation_flag.
    - Comparison mean/ci_lo/ci_hi/p are Option and null when n = 0; Holm uses 1 for a missing p; decision Inconclusive without an interval; per_corpus_comparisons test asserts nulls.
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - Null statistics would change a confirmatory decision with n > 0.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-7 (sha256 019cf794c91b)'
    dod_status: pass
  SIMPL-3:
    title: Given rater count matrices, one quadratic-weighted kappa implementation serves agreement gates and statistics
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-7
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/gates.rs
    - src/eval/stats.rs
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/stats.rs
    non_goals:
    - Consolidating identity checks (deferred by owner) or changing agreement thresholds.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - cargo fmt --check
    expected_evidence:
    - stats::kappa_from_counts (square matrix, k >= 2) is the only kappa formula; weighted_kappa builds counts and delegates; gates.rs duplicate removed.
    - Worked examples unchanged: 10/13, identical raters 1, undefined None (stats and gates tests).
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - A worked-example value changes.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#simpl-3 (sha256 b6e2cc4192b0)'
    dod_status: pass
  FIX-8:
    title: Given a held-out run, the private staging directory exists before the sole attempt is reserved
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - SIMPL-3
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/run.rs
    - tests/eval_run.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_run.rs
    non_goals:
    - Reading held-out payloads before reservation or changing StagingDir semantics.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test eval_run
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - cargo fmt --check
    expected_evidence:
    - run_held_out and run create StagingDir before Bench::load_test_reserved / run_reserved; run_reserved takes the stage.
    - Regression unwritable_output_parent_refuses_before_reservation: read-only parent refuses, no attempt-started.json, no output; fails with the old ordering (checked by temporary revert).
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - Staging before reservation would read held-out payloads.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: approved and continue (owner decisions 7, 8, 9, 10); audit_session-2026-11-11-owner-decisions.md'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#fix-8 (sha256 3bd39fe7017b)'
    dod_status: pass
  FIX-9:
    title: Given round-5 review, dev runs stage after validation and an ADR discrimination check exercises the real rankers on planted synthetic sections
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-8
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/ranking_sanity.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_run.rs
    - tests/ranking_sanity.rs
    non_goals:
    - Quality claims, tuning, new CLI commands, real data or held-out guard changes.
    required_validation:
    - CARGO_BUILD_JOBS=4 cargo test --test ranking_sanity --test eval_run
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - cargo fmt --check
    expected_evidence:
    - run_reserved takes Option<StagingDir>; held-out stages before reservation, dev stages after its budget checks (pre-FIX-8 order).
    - Privileged-host skip of the read-only-parent test prints SKIPPED.
    - tests/ranking_sanity.rs, ADR discrimination check: oracle beats a seeded shuffle with p < 0.01, self-comparison gives p = 1, B1/B2/B3 find planted sections above shuffle, and B0 cites the first section.
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - The diagnostic would need real benchmark data or a held-out path.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 5); audit_session-2026-11-11-review-round5.md'
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 210 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok. Astra P2 (dev staging moved before ranking) and P3 (silent privileged skip) fixed. The first run of ranking_sanity showed that B0 cannot reach planted later sections. This is the ADR-defined baseline behaviour; it is asserted, not changed. Synthetic only.'
    dod_status: pass
  FIX-10:
    title: Given round-6 review, numeric scorers have hand-computed edge tests, hash crates build optimized in dev
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-9
    type: code
    validation_tier: unit-fast
    impact_risk: low
    owner_files:
    - Cargo.toml
    - src/search/bm25.rs
    - src/search/ppr.rs
    - src/search/fusion.rs
    allowed_write_set:
    - Cargo.toml
    - src/search/bm25.rs
    - src/search/ppr.rs
    - src/search/fusion.rs
    non_goals:
    - Parser, tier, identity, schema or ranking changes; store pruning recovery.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib search::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - BM25 empty-document avgdl, PPR iteration-cap and fusion tie/drop/singleton tests with hand computations in comments.
    - Dev profile opt-level 3 for sha2, digest and block-buffer only.
    - README usability notes are documentation-only and recorded in the round-6 audit, outside this card.
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - A test would require changing scorer behaviour.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 6); audit_session-2026-11-11-review-round6.md'
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 213 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1) in 17.8 s wall; fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. Synthetic only.'
    dod_status: pass
  FIX-11:
    title: Given round-7 review, an interrupted prune no longer disables pruning, fusion tie-break tests discriminate, and HTTP socket tests have deadlines
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-10
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/store.rs
    - src/search/fusion.rs
    - src/search/bm25.rs
    - tests/http.rs
    allowed_write_set:
    - src/store.rs
    - src/search/fusion.rs
    - src/search/bm25.rs
    - tests/http.rs
    non_goals:
    - Parser, tier, identity, schema or ranking changes; recovery of already-existing incomplete generation dirs.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib store::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - record_and_prune renames each validated complete pruned generation to tmp-<id> (refusing an occupied name), syncs generations/ and the store dir, then deletes; new PruneMove phase.
    - Regression - failing on the second prune file leaves one file in tmp-<id>; the next publish removes it and prunes to three generations.
    - Fusion shuffled-input raw-BM25 and chunk-id tie tests; HTTP reads bounded by a deadline and accumulated to the header terminator.
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - The fix would relax complete-shape validation of generations/.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate and proceed (round 7); audit_session-2026-11-11-review-round6.md#round-7'
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 214 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. ADR revision 11. Two GPT-6-Astra reviews, no P1; P2 store-dir sync and P3 partial-file coverage fixed. Synthetic only.'
    dod_status: pass
  FIX-12:
    title: Given owner approval, the first-sentence rule parses with the sectioning GFM extensions so tables are never first sentences, and the stored schema becomes 3
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-11
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/parse.rs
    - src/tiers.rs
    - src/graph.rs
    - src/store.rs
    - tests/cli.rs
    allowed_write_set:
    - src/parse.rs
    - src/tiers.rs
    - src/graph.rs
    - src/store.rs
    - tests/cli.rs
    non_goals:
    - Section or chunk identity changes, ranking parameter changes, tuning, migration of old stores.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib tiers:: parse:: graph:: store::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - One pub(crate) MARKDOWN_OPTIONS const in parse.rs used by parse, tiers first_sentence and graph.
    - Regression - a table before a paragraph yields first sentence "Actual sentence.".
    - SCHEMA_VERSION 3; schema 2 and 4 artifacts refuse as unsupported; CLI status reports 3.
    rollback:
    - Revert this task's listed changes; schema-3 stores then need a reindex under schema 2.
    stop_conditions:
    - Section or chunk ids change on any existing fixture.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: "approved" (round-7 option 2, table-aware first sentence); ADR revision 12'
        validation_summary: 'Local run exit0: cargo test --all-targets (lib 214 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12, ranking_sanity 1); fmt --check ok; clippy --all-targets -D warnings ok; git diff --check ok. All section/chunk-id fixture tests unchanged and green. No dev or held-out data exists or was read.'
    dod_status: pass
  PERF-1:
    title: Given concurrent MCP clients, requests snapshot the shared immutable index under a short lock and run with bounded configurable permits owned by the actual work
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - HTTP-1
    type: code
    validation_tier: integration
    impact_risk: high
    owner_files:
    - src/mcp.rs
    - tests/mcp.rs
    allowed_write_set:
    - src/mcp.rs
    - tests/mcp.rs
    non_goals:
    - Listener startup, freshness caching or TTLs, unbounded queues, ranking changes.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib mcp:: && CARGO_BUILD_JOBS=4 cargo test --test mcp'
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-11-done-card-evidence-archive.md#perf-1 (sha256 9664c6309097)'
    rollback:
    - Revert this task's listed source and test changes.
    stop_conditions:
    - Concurrency requires changing ADR admission or freshness contracts.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: review/challenge/consolidate one plan and proceed; use GPT reviewers'
        validation_summary: 'Archived verbatim: audit_session-2026-11-11-done-card-validation-archive.md#perf-1 (sha256 bb9e833843f6)'
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| REQ-001 | User: Rust project | implementation | OD-0.0 | `cargo check --all-targets` |
| REQ-002 | User: new MCP service for docs | implementation | OD-2.1 | `cargo test --test mcp` |
| REQ-003 | ADR §2–4: safe corpus walk, CommonMark parsing, budget-independent section ids, chunks | implementation | OD-1.0, OD-1.1 | corpus and parse unit tests |
| REQ-004 | ADR §4: structural tiers and document pseudo-documents | implementation | OD-1.2 | tiers unit tests |
| REQ-005 | ADR §5: document link graph | implementation | OD-1.3 | graph golden edge list |
| REQ-006 | ADR §7: atomic verified generations, index-time-only generation id | implementation | OD-1.4 | store unit tests |
| REQ-007 | ADR §6: B0, B1, pre-fusion abstention, no LLM on path | implementation | OD-1.5 | search unit tests |
| REQ-008 | ADR §6: specified PPR (B2), B3, scope, explain | implementation | OD-1.6 | search unit tests |
| REQ-009 | ADR §10, H4: four citation states | implementation | OD-1.7, OD-2.1 | cite and MCP tests |
| REQ-010 | ADR §8: budgeted assembly, feature without claim | implementation | OD-1.8, OD-2.1 | assemble and MCP tests |
| REQ-011 | ADR §9 and §11: freshness and determinism | implementation | OD-1.4, EVAL-1.1 | store tests; byte-identical reruns |
| REQ-012 | User: scientifically provable improvements; ADR metrics, decision rule, power | implementation | EVAL-1.0, EVAL-1.1, EVAL-1.2 | stats unit tests; discrimination check; planted-defect gates |
| REQ-013 | ADR benchmark data, grid-wide pooling, annotation | proof-only | EVAL-2.0, EVAL-2.1 | `eval check-benchmark` |
| REQ-014 | ADR H1 gate, H2/H5 confirmatory, H3, E1; code freeze; burned split | proof-only | EVAL-3.0, EVAL-3.1, EVAL-3.2 | recorded reports and audit sessions |
| REQ-015 | User: review prior attempt; reuse list applied | implementation | OD-1.0, OD-1.4, OD-1.7 | prior-attempt audit; store and cite tests |
| REQ-016 | User: donor ideas (OpenViking, pharma) | implementation | OD-1.2, OD-1.6, OD-1.8, EVAL-1.2 | donor-ideas audit; unit and gate tests |
| REQ-017 | Plan challenge findings C1–C4, H1–H3, M1–M9, L1–L3 | implementation | EVAL-1.0, EVAL-1.2, OD-1.1, OD-1.5, OD-1.6 | plan-challenge audit dispositions |
| REQ-018 | Embeddings, KM, LineageRAG, code graph, H6 excluded from v1 | non-goal | OD-1.5, OD-1.8 | task non_goals; ADR Consequences |
| REQ-019 | User: AGENTS.md and memory bank | implementation | OD-4.0 | files present; updated at closeout |
| REQ-020 | Review S5: unique published section/chunk IDs | implementation | OD-3.0 | duplicate collection rejection; unchanged store; malformed load |
| REQ-021 | Review S6: canonical bytes and identity collisions | architecture-gate | OD-3.1, OD-3.2 | accepted schema decision; mtime-independent full artifact bytes |
| REQ-022 | Review S1: full-wire bounds and internal-error privacy | implementation | OD-3.3 | actual JSON-RPC frame tests, including escaped text and errors |
| REQ-023 | Review S2: corpus membership and race-safe bounded reads | implementation | SAFETY-0, OD-3.4 | descriptor replacement/growth and fabricated-citation tests |
| REQ-024 | Review S3: bounded actual work and cancellation ownership | implementation | SAFETY-0, OD-3.5 | work counters and closure-owned admission barrier tests |
| REQ-025 | Review S4: bounded freshness/load I/O without semantic weakening | implementation | OD-3.8, SAFETY-0, OD-3.6 | pre-read caps; same-size edit freshness |
| REQ-026 | Evaluation isolation, checked-byte identity and success paths | implementation | EVAL-1.4, EVAL-1.5, EVAL-1.3 | synthetic inaccessible held-out inputs, tampering, attempt and success/failure tests |
| REQ-027 | Accepted rev4: each docs_share child has endpoint and own store | implementation | HTTP-0, HTTP-1, HTTP-2 | independent stores/session bindings and stdio parity |
| REQ-028 | Usable operational documentation independent of claims | implementation | OD-3.7, OD-4.0 | current contracts first; scientific results only after EVAL-3.2 |
| REQ-029 | Proposed evidence/diagnostics capabilities | architecture-gate | OD-4.0 | deferred and unaccepted; separate service-ADR amendment and task scope required |
| REQ-030 | Current challenge: stdio safety independent of network exposure | architecture-gate | SAFETY-0, HTTP-0 | explicit local contracts; owner exposure gate retained |
| REQ-031 | Current challenge: bounded store controls/artifacts/publication | implementation | OD-3.8 | actual-byte, configured/global cap and preservation regressions |
| REQ-032 | Current challenge: publication outcomes and lock evidence | implementation | SAFETY-0, OD-3.9 | deterministic commit-boundary failure tests |
| REQ-033 | ADR9 trusted provenance, cross-process preparation and custody acceptance | architecture-gate | EVAL-1.6, EVAL-1.3, EVAL-2.0, EVAL-2.1 | pre-pool approval, raw human agreement, selected-only execution and synthetic failure paths |
| REQ-034 | ADR9 deterministic default promotion, distinct from measurement | implementation | EVAL-1.6, EVAL-3.2, OD-4.1, OD-4.0 | frozen H5-first policy; audited implementation/no-op; CLI/MCP parity |
| REQ-035 | 2026-11-11 review: verified cite/search/eval defects | implementation | FIX-1 | focused regression per defect |
| REQ-036 | 2026-11-11 review: remove eval duplication without weakening safeguards | implementation | SIMPL-1 | unchanged gate/report/refusal tests |
| REQ-037 | User: many concurrent users on the shared corpus | implementation | PERF-1, HTTP-2 | overlapping admitted requests, bounded permits |
| REQ-038 | Deferred review item: frozen TOML round-trips arbitrary corpus names | implementation | FIX-2 | escaped-name round-trip regression |
| REQ-039 | 2026-11-11 round-3 review: verified eval output-budget, miss-attribution and dead-plumbing defects | implementation | FIX-3 | pre-reservation budget and attribution regressions |
| REQ-040 | 2026-11-11 round-3 review: bounded related/status collection | implementation | FIX-4 | unchanged-response regression |
| REQ-041 | 2026-11-11 round-3 deferred simplification: single held-out freeze verification per command | implementation | SIMPL-2 | unchanged refusal tests |
| REQ-042 | 2026-11-11 round-4 review: held-out pre-reservation refusals for TSV-unsafe ids and unusable output parents | implementation | FIX-5 | refusal regressions |
| REQ-043 | 2026-11-11 round-4 review: verifiable citations (heading bound at publication) | implementation | FIX-6 | cap and lookup regressions |
| REQ-044 | Owner decisions 7-8 (rev10): per-result text_truncated; null statistics for n = 0 groups | implementation | FIX-7 | flag and null regressions |
| REQ-045 | Owner decision 9: one shared kappa implementation | implementation | SIMPL-3 | unchanged worked examples |
| REQ-046 | Owner decision 10: staging before held-out reservation | implementation | FIX-8 | unwritable-parent refusal regression |
| REQ-047 | Round-5 review: dev staging order; ADR discrimination check on planted synthetic sections | implementation | FIX-9 | ranking_sanity and staging regressions |
| REQ-048 | Round-6 review: hand-computed scorer edge tests; dev hash-crate optimization | implementation | FIX-10 | search:: numeric tests |
| REQ-049 | Round-7 review: interrupted prune recovery via temp namespace; discriminating fusion ties; bounded HTTP test reads | implementation | FIX-11 | store prune regression |
| REQ-050 | Owner-approved table-aware first sentence; schema 3 | implementation | FIX-12 | tiers first_sentence regression |

## Decisions And Non-Goals

- ADR9 accepts [trusted preparation and promotion](ADR_ONTODOC_SERVICE.md#trusted-evaluation-preparation-and-promotion-accepted-not-implemented).
  EVAL-1.3 implements bounded provenance/admission with synthetic proof; EVAL-2.x
  supplies trusted review/human evidence. EVAL-3.2 recommends only; OD-4.1 applies
  H5-first defaults or records no-op before final OD-4.0. No ranking, code,
  real benchmark access, listener or guard removal occurs in EVAL-1.6.

- EVAL-2.0's manager, separately from the data worker, may create/update exactly
  `eval/audit_session-query-approval.md` to record authentic final-query approval
  and its hash before the first ranking invocation, then append pool/preparation
  evidence. This manager-only paperwork authority does not enter the worker's
  allowed write set or permit self-approval, fabricated reviewer provenance or
  ranking before approval. Missing trusted operator evidence stops pooling.

- SAFETY-0 accepts ADR8 local rooted-reader/membership/work/publication contracts;
  [decision evidence](audit_session-2026-11-10-safety0-local-contracts.md).
  Implement OD-3.4 → OD-3.6 → OD-3.5, then OD-3.9; EVAL-1.6 remains independent.
  HTTP-0 still needs the owner's exposure/admission choice. No code, listener,
  schema/ranking change or measurement is authorized by this documentation closure.

- Current consolidation: [fresh GPT challenge](audit_session-2026-11-10-review-safety-priorities.md). Preserve all 22 DONE cards and existing receipts. Select OD-3.8, then SAFETY-0 → OD-3.4 → OD-3.6 → OD-3.5, with OD-3.9 after its decision; EVAL-1.6 → EVAL-1.3 completes independent evaluation admission. HTTP-0 remains owner-blocked. This explicitly supersedes the earlier HTTP-dependent safety ordering, not its deployment gate.

- EVAL-1.5's explicit authority-conflict resolution permits manager-owned
  documentation only in `ADR_ONTODOC_SERVICE.md`, this plan (including EVAL-3.x
  command bindings), `audit_session-2026-11-10-eval15-attempt-identity.md`,
  `MEMORY.md`, `project_architecture.md`, `project_plan-current.md`,
  `project_pending-tasks.md` (all under `.memory-bank/`) and `AGENTS.md`.
  This is separate from the unchanged coder-worker implementation write set;
  manager closeout/status changes still use the strict manager loop.

- 2026-11-11 [review/simplify consolidation](audit_session-2026-11-11-review-simplify-consolidate.md):
  FIX-1 → SIMPL-1 → HTTP-1 (folder registry, no listener; split from the HTTP-0
  exposure gate) → PERF-1 (concurrent admission) → HTTP-2 (still needs HTTP-0).
  EVAL-2.x stays owner/human-gated.
- Review decision: [2026-11-10 consolidated challenge](audit_session-2026-11-10-consolidated-review.md).
  Preserve all 15 historical DONE cards; those receipts do not validate today's tree.
  Execute OD-3.0 first, then prefer wire/privacy repairs and bounded I/O/work.
- EVAL-1.3 is DONE (2026-11-11); measurement admits only a pinned trusted preparation.
  A separate audit registry service is not required by the accepted ADR.
- EVAL-1.4's [bounded capture seam](audit_session-2026-11-10-eval14-bounded-traversal.md)
  uses corpus-owned pre-filter enumeration and explicit ignore bytes with public
  matchers; successfully readable regular-control parity is tested. Unsafe or
  unreadable controls refuse. This conforms to ADR2 without accepting the
  ordinary automatic walker's symlink conflict or claiming rooted HTTP I/O.
  Preserve the [partial-progress audit](audit_session-2026-11-10-eval14-input-capture.md)
  as the record of the earlier unaccepted draft and validation corrections.
- Before EVAL-3.x execution, rebind each required command to the implemented CLI
  after EVAL-1.3; existing historical command shapes are not executable evidence.
  ADR revision 6 uses a canonical external `ONTODOC_BENCH` snapshot, the exact
  audited `ONTODOC_FROZEN_BIN` and Git-resolved absolute `ONTODOC_GIT_COMMON`.
  Pin paths, corpus and held-out snapshot bytes before the dev proof. Dev-only
  parameter selection may update config.json under EVAL-3.1; recapture/revalidate
  and register that final config before any held-out access. External
  `audit_session-*` files record preregistration (registration/export hashes)
  and outcomes (started/result hashes); mirror them to memory only afterward.
  Run from the clean frozen checkout without rebuilding between freeze and
  held-out execution.
  These variable-bound commands are future protocols, not measurements run here.
- HTTP follows HTTP-0 then HTTP-1/2 and the service-safety dependencies, not the
  human-label chain. No listener is implemented or exposed by this plan revision.
- Diagnostics remain Proposed; this consolidation does not accept any capability.
  OD-4.0 records its final disposition, never implements it implicitly.
- OD-3.1 records [service ADR revision 5](ADR_ONTODOC_SERVICE.md#schema-2-identity-and-canonical-artifacts-accepted-not-implemented):
  schema 2 reserves `@empty` in IDs only, canonicalizes skip records and removes
  persisted mtime; fresh checks remain current-byte hashes. OD-3.2 implements
  these decisions; no code or benchmark identity changes occur in OD-3.1.
  LF formatting between copied tier fragments is permitted, without accepting
  provenance/diagnostics capabilities or changing the hypothesis family.

- One package with a library and a binary; split into crates only when a second
  consumer exists.
- `ontocode-rs/` is read-only reference material; nothing imports from it.
- No container, Windows or macOS build work.
- Task ids were renumbered in revision 3 before any task started; no state was lost.
- The section is the evaluation unit; the chunk budget is frozen at 1,200 bytes
  and is not tuned in v1.
- EVAL-2.1 needs two humans. A model may draft queries, but its output is
  marked and never counted as a human label.
- Parallel work: EVAL-1.0 depends only on OD-0.0; OD-1.2 and OD-1.3 both
  depend only on OD-1.1; OD-1.7 and OD-1.8 can run beside OD-1.6.
- Donor ideas are adopted as ideas only; licences (AGPL-3.0, none) forbid
  copying code or data.

## Closeout

