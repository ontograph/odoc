# Completed task cards before release-plan integration

Captured verbatim on 2026-11-12. These cards are historical evidence; the active tracker retains status, dependencies, ownership and closeout state.

## OD-0.0

SHA-256 of original card bytes: `92a0e069b07879393db88de675b92a5f892cef94718cb80794f284c2e1ab0afb`.

```yaml
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
```

## OD-1.0

SHA-256 of original card bytes: `c863958860cc1961f05bfb572d5f52e06a974665d9a99e6052aab0b35b5924dd`.

```yaml
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
```

## OD-1.1

SHA-256 of original card bytes: `eee00cbe5f4ea8539e58ae9f834702df47ffb5c6ecdf21bf1faf0a60b2cee90e`.

```yaml
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
```

## OD-1.2

SHA-256 of original card bytes: `2e39f905e81ef2ed9eb0e339bc94ed7a83af53e522f21e3de9c1f6dc95fb4ce9`.

```yaml
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
```

## OD-1.3

SHA-256 of original card bytes: `08330d94df89900b5ccafef1b191010c5b614829be9892f6cf0f37035850977b`.

```yaml
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
```

## OD-1.4

SHA-256 of original card bytes: `48d9624f9b5b276370c6343cc081f5a38717cf49c4e35c1b39bc16c1d8ad0329`.

```yaml
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
```

## OD-1.5

SHA-256 of original card bytes: `93799ea66f281f735f11450ac5c364f6f634a28916c9f2302f609d2169205de3`.

```yaml
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
```

## OD-1.6

SHA-256 of original card bytes: `3f0b4520cb89298aaf61b561dd8b8d4ded53875b26ba02a6028f2fc725c4f3a1`.

```yaml
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
```

## OD-1.7

SHA-256 of original card bytes: `02e5986fa4b60bb1d168184bbe123326c2446f75b008c021153097adbdd23584`.

```yaml
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
```

## OD-1.8

SHA-256 of original card bytes: `662260482f8c65ddac4495f15318b7b1ebb128bc8ff08462dd4830f83780c6fe`.

```yaml
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
```

## OD-2.0

SHA-256 of original card bytes: `a337ecee5e41176e7ef86a15975fcb6dd9b906dc45b80bee50a18c38869ad544`.

```yaml
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
```

## OD-2.1

SHA-256 of original card bytes: `7046ac6cc6535dc180f9f127a6ce655d53fa80f00259a6ac0e337d91603e2225`.

```yaml
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
```

## EVAL-1.0

SHA-256 of original card bytes: `165aee2a98be7ace4427975a484be6018c8d74515c63ce8cd3cefd289ec2ad00`.

```yaml
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
```

## EVAL-1.1

SHA-256 of original card bytes: `cdb52d0a5e600c08ec534545a6a8805f807ce7c57790d2728e2de60da2e454f6`.

```yaml
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
```

## EVAL-1.2

SHA-256 of original card bytes: `91587807915b8903af0477eb940e44be67a61d960115bfb42c8c975f5c856f59`.

```yaml
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
```

## EVAL-1.3

SHA-256 of original card bytes: `730e157aa9879ae81223c3d4d60ab3eae49a4f78bcb02916d718acbc2631ad30`.

```yaml
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
```

## EVAL-2.0

SHA-256 of original card bytes: `4dd1243fe47304665e5fb26c8dc3e8f6676f3b23472c24f523dfdfd19de6ac99`.

```yaml
  EVAL-2.0:
    title: Given the pinned corpora, when the benchmark is assembled, then `eval/` holds pinned corpus metadata, stratified queries with operational strata and seeded dev/test split, and grid-wide top-10 pools with the judgement count reported
    status: DONE
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
    - CARGO_BUILD_JOBS=4 cargo run --release -- eval check-benchmark eval/config.json --stage pools --gates eval/gates.json --query-review eval/query-review.jsonl --query-review-sha256 "$ONTODOC_QUERY_REVIEW_SHA256"
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#eval-20 (sha256 d183e7f9fe39)'
    rollback:
    - Delete the listed files.
    stop_conditions:
    - No permissively licensed second corpus is available.
    - Queries cannot be written without copying target text.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "1-4" approving the spot-check sample and review digest 94aaeff0 (eval/audit_session-query-approval.md); owner decisions 2026-11-11'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#eval-20 (sha256 b5d2f12d9c27)'
    dod_status: pass
```

## FIX-14

SHA-256 of original card bytes: `e6a78671a03217b470f8866c5498b725d8242b64ce9470a78802e1dfe9933493`.

```yaml
  FIX-14:
    title: Given ADR rev 13, annotation notes may declare label_provenance model-judged with two distinct model judges, preparation records it, and every scored report then carries it with confirmatory comparisons downgraded to exploratory (no Holm, no decision)
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-13
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/gates.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/common/mod.rs
    allowed_write_set:
    - src/eval/gates.rs
    - src/eval/run.rs
    - tests/eval_gates.rs
    - tests/common/mod.rs
    non_goals:
    - Weakening any human-path check; model parties as human; changes to ranking, metrics or decision rules for human labels.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-14 (sha256 2b6c7267e819)'
    rollback:
    - Revert this task's listed changes.
    stop_conditions:
    - Any existing human-path test changes expectation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "1-4" (decision 2: implement model-judged admission) and "continue"; ADR revision 13 owner decision 1b'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-14 (sha256 e40b3c7ecac8)'
    dod_status: pass
```

## FIX-15

SHA-256 of original card bytes: `d733b6792a4ea89c2a4012005b31b0e160a5d8bcc5ec645cc2589394fd89e2e6`.

```yaml
  FIX-15:
    title: Given ADR rev 14-15, config pool_extension deepens the pool for listed answerable queries only, is validated against the whole benchmark at preparation and pooling, and selected dev/test captures never need the other split
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-14
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/eval/run.rs
    - tests/eval_gates.rs
    allowed_write_set:
    - src/eval/run.rs
    - tests/eval_gates.rs
    non_goals:
    - Changing @10 metrics, the base pool, ranking, or any human-path rule.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib eval::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - PoolExtension{depth 11..=30, 1..=64 sorted unique ids}; rank_sections_to keeps the FETCH order and top-10 prefix; only listed queries pool deeper; pool bytes regenerate identically.
    - validate_pool_extension runs on the complete query set (load, full capture, pooling), not per partition; a regression with ids in both splits passes preparation and a dev-only capture, and fails on the old per-partition check.
    rollback:
    - Revert this task's listed changes and remove pool_extension from eval/config.json.
    stop_conditions:
    - Any existing pool or human-path test changes expectation.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-12: "B" (ADR rev 14 pool extension) and "proceed" (review round 8, ADR rev 15)'
        validation_summary: 'Local run exit0: cargo fmt --check; clippy --all-targets -D warnings; cargo test --all-targets 281 passed 0 failed. GPT-6-Astra review P1 (global extension check rejected partitioned preparation) fixed with regression pool_extension_spanning_both_splits_admits_preparation_and_dev_capture, which fails with the old check. Two pool generations byte-identical. Synthetic tests plus the real pool.'
    dod_status: pass
```

## OD-3.0

SHA-256 of original card bytes: `492901f88d5727b6a47ad0ee2dfaf46d84c441c1bf2fea2cb56a161c37349985`.

```yaml
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
```

## OD-3.1

SHA-256 of original card bytes: `ca38b63e26fdd760bb235872ecbc26c61ae7f4f74b925cd2a800ee2eb54abe52`.

```yaml
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
```

## OD-3.2

SHA-256 of original card bytes: `6664b7ab6985d7216e16787525547daf8cd63f71bb83a025c20db3636146be09`.

```yaml
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
```

## OD-3.3

SHA-256 of original card bytes: `a5193210ef028cc54ae482a522780dcc5b3342aeb16eafbcc9d76fd575506f8b`.

```yaml
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
```

## OD-3.4

SHA-256 of original card bytes: `d9d088eef3e1734b5c77b44ea535f80afaa8eb98c00248695968d524873df2fa`.

```yaml
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
```

## OD-3.5

SHA-256 of original card bytes: `84cb26ae40785c0ac05f41f6bf6dd9831d6fbd05fd208653d7a64ff209d06bbc`.

```yaml
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
```

## OD-3.6

SHA-256 of original card bytes: `bf51624c64732cf8d37ebaf83c75931489a3e0f33b7c484c8e194d26e8dc66d0`.

```yaml
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
```

## EVAL-1.4

SHA-256 of original card bytes: `b6771e268cafaa8304838f28282a147cc437d4e221a4e55936c98bc203c2886b`.

```yaml
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
```

## EVAL-1.5

SHA-256 of original card bytes: `053f5df9690e3847d254dbeaa00cc9677e6cb3cd225b84f385fc2190121cb823`.

```yaml
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
```

## HTTP-0

SHA-256 of original card bytes: `26a3cc8e0bedd090389f2efe0895d1d7901c9fb2359d73a0b1aa2a8caf0aa7c5`.

```yaml
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
```

## HTTP-1

SHA-256 of original card bytes: `700fb970df3240ae0fa6f1948d8a4e41fefd6a76f68f3e305718af971c534e4b`.

```yaml
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
```

## HTTP-2

SHA-256 of original card bytes: `408acf21c1fb13ba9919896aabaa0b9dcd9f907e1fc2ed59f198905ff8b8ac50`.

```yaml
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
```

## OD-3.7

SHA-256 of original card bytes: `00c8d70585b90ba5b540b164bf789eea9ec6046c9761d4828c325bd62d64a59e`.

```yaml
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
```

## OD-3.8

SHA-256 of original card bytes: `01a5ecfea640ab76489e48c7046a935005136e1e6c39f5599c6515f879b8ea54`.

```yaml
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
```

## OD-3.9

SHA-256 of original card bytes: `e3fb46b3dae6fb6f450e3aa33409187b2837e7103e10596f6b03110edbcfb4a3`.

```yaml
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
```

## SAFETY-0

SHA-256 of original card bytes: `6fe7712dad09ba71ba56a18f27a98949d30c8d2f9d01dc64d989e5428eae21f2`.

```yaml
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
```

## EVAL-1.6

SHA-256 of original card bytes: `75953df9c6b2a5df638038b2784652891a4bc1ff12b11b7d2b7c970e86be8f5c`.

```yaml
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
```

## FIX-1

SHA-256 of original card bytes: `c614f5f909d0fc187b43efe2ca943cd67a4a7c0549bb2f07ede50cdaacbdd1a2`.

```yaml
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
```

## SIMPL-1

SHA-256 of original card bytes: `dc0aadcdf0bae27bd9d5a2adc49be44da97e2b77ffba9aa531ed7654589a90a6`.

```yaml
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
```

## FIX-2

SHA-256 of original card bytes: `142802f8481a9775a9ec069046a99eca219ae9ff9096e2d07e2e12d8328c6801`.

```yaml
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
```

## FIX-3

SHA-256 of original card bytes: `ede12cf4136e14817e2066c6944fb570f6e6a8902625fe96b98dff9666e733b1`.

```yaml
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
```

## FIX-4

SHA-256 of original card bytes: `3466624835d25e43d2fbcf35e7637044f279b20fa16ed41a8a5c27b57de59b9d`.

```yaml
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
```

## SIMPL-2

SHA-256 of original card bytes: `92e58f00ac4796f35bef7f6baecd491ed7488943ecad31855125a6a28540c665`.

```yaml
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
```

## FIX-5

SHA-256 of original card bytes: `5ee535facd59f76acda30bf379c44e2deb23cd5829cfe5dbd954d6c589fbae58`.

```yaml
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
```

## FIX-6

SHA-256 of original card bytes: `497bd2e094caa318923943bc1616a7e5a8b4282cebfd76f3255a27e5f22f3842`.

```yaml
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
```

## FIX-7

SHA-256 of original card bytes: `7b277f0b9c44b5a3d4ffc69042891fecdaa565d00e17d1907328110a85cd6395`.

```yaml
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
```

## SIMPL-3

SHA-256 of original card bytes: `bf0462be32e1104556bd8d4274fd07115b65a2050af7c9ca73ee974abeae86f8`.

```yaml
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
```

## FIX-8

SHA-256 of original card bytes: `12360aca44a434f745f5d7af63ce0bf9460b7986523837af615252450810cfea`.

```yaml
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
```

## FIX-9

SHA-256 of original card bytes: `d2723cd43c79b48c04bcbe3f3d7da7fbc1391981124fe6d5442b99baf03139a9`.

```yaml
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
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-9 (sha256 2651549cf6f5)'
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
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-9 (sha256 58517f790daa)'
    dod_status: pass
```

## FIX-10

SHA-256 of original card bytes: `ba67c0ce9012c39d5d231e5754bccb90c0b86de5aa06defadcabea278b070278`.

```yaml
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
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-10 (sha256 66b4920b2630)'
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
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-10 (sha256 ae1a67e2bd05)'
    dod_status: pass
```

## FIX-11

SHA-256 of original card bytes: `8c2de2bed3c2ebcd1b300421f9d130c2249066e2b263c61a5b46d0b427f36011`.

```yaml
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
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-11 (sha256 48e0934a4042)'
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
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-11 (sha256 ab47ca5d36cf)'
    dod_status: pass
```

## FIX-12

SHA-256 of original card bytes: `695549d3bf66c82b3ca00bfc6ed9a4d6f374dfac898bad4986fe9ae0c68bdc3d`.

```yaml
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
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-12 (sha256 bf20e290692d)'
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
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-12 (sha256 f574fa3ae4ac)'
    dod_status: pass
```

## FIX-13

SHA-256 of original card bytes: `ca4f4ff2258a0859567a50256f739a85f513fc4ee6bd4d45051253399b024b32`.

```yaml
  FIX-13:
    title: Given owner approval, relative `.html` links resolve to an existing sibling `.md` source (schema 4) and a corpus rooted in a checkout subdirectory excludes only its own store from the pin dirty check
    status: DONE
    classification: implementation-ready
    assurance_profile: full
    complexity: complex
    depends_on:
    - FIX-12
    type: code
    validation_tier: unit-fast
    impact_risk: high
    owner_files:
    - src/graph.rs
    - src/store.rs
    - src/eval/gates.rs
    - tests/cli.rs
    allowed_write_set:
    - src/graph.rs
    - src/store.rs
    - src/eval/gates.rs
    - tests/cli.rs
    non_goals:
    - Section or chunk identity changes, ranking parameter changes, tuning, migration of old stores, absolute-URL link resolution.
    required_validation:
    - 'CARGO_BUILD_JOBS=4 cargo test --lib graph:: store:: eval::gates::'
    - CARGO_BUILD_JOBS=4 cargo test --all-targets
    - cargo fmt --check
    expected_evidence:
    - 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-13 (sha256 3500e691d969)'
    rollback:
    - Revert this task's listed changes; schema-4 stores then need a reindex.
    stop_conditions:
    - Section or chunk ids change on any existing fixture.
    known_blockers: []
    closeout:
      role_results:
        coder-worker: null
      main_session_evidence:
        authorized_by: user
        authorization_ref: 'User 2026-11-11: "1 b ... 2 i 3 ok" (html link mapping, rust-book/src root, model-judged exploratory track); ADR revision 13'
        validation_summary: 'Archived verbatim: audit_session-2026-11-12-done-card-archive.md#fix-13 (sha256 a899c69ab1bf)'
    dod_status: pass
```

## PERF-1

SHA-256 of original card bytes: `4a35cbcd98c93cb7fc9596555a20db4c5a537b291c0899d5de47cbf925c72ddd`.

```yaml
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
