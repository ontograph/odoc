# DONE Task-Card Validation-Summary Archive

Date: 2026-11-11
Authority: user review/consolidation instruction (round 5); the tracker was at
93% of the 131,072-byte parser ceiling.

Tracker before archival: `ee9a213f2918f537e22ad5e8c173d00c2b49d1d30a1924ee82dae6d5ef2feb2b`. For each DONE card, the closeout
`validation_summary` line is moved here verbatim. In the plan, it becomes one
reference line. `authorized_by`, `authorization_ref`, `dod_status`, statuses,
dependencies, write sets and manager routing are unchanged. Each SHA-256
covers the UTF-8 bytes of the YAML line shown. This is historical validation
evidence, not current-tree validation. It follows the earlier
[expected-evidence archive](audit_session-2026-11-11-done-card-evidence-archive.md).

## OD-0.0

SHA-256: `9547531a405b962410af8f5a7046eb89de2b5ac8174236e214abb18119435626`

```yaml
        validation_summary: 'CARGO_BUILD_JOBS=4 cargo check --all-targets exit0; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. cargo metadata --no-deps: workspace_root repo root, single member ontodoc; ontocode-rs not resolved. Cargo.lock sha256 b25f4fd3b299405343d854b1af6d6093c91169ce347c59e430c589d603548322; rmcp 1.7.0 pulldown-cmark 0.13.4 schemars 1.2.1 sha2 0.10.9 clap 4.6.7 tokio 1.53.1 ignore 0.4.25. .gitignore covers /target/ and /.ontodoc/.'
```

## EVAL-1.3

SHA-256: `b5512fc122f23a2bb64bdbced418dd9ec0c809a19f82a23588f345cd57ad7ef1`

```yaml
        validation_summary: 'Lane eval13-close-a exit0: CARGO_BUILD_JOBS=4 cargo test --lib (196 passed, 1 ignored); cargo test --test eval_gates --test eval_run (20 + 11 passed); cargo test --test cli (8 passed); cargo fmt --check ok; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings ok (no allowance). Slices 1-9 evidence and GPT-6-Astra findings in audit_session-2026-11-11-eval13-progress.md. Synthetic fixtures only; no real benchmark or held-out data.'
```

## OD-3.0

SHA-256: `96e92224ead1a9eedc221b57b3ea073a9edecbaddb70f2f8941a86dca446029a`

```yaml
        validation_summary: 'OD-3.0 only src/store.rs: shared borrowed-ID uniqueness validator runs before publish serialization/writes and after load hash/schema/identity checks. Three new regressions cover independent section/chunk duplicates, empty-slug/preamble collision, matching-hash malformed artifacts and preservation/no creation of stores. CARGO_BUILD_JOBS=4 cargo test --lib store:: exit 0, 12 passed (e8eae0/309805, session21403). cargo fmt --check initially exit1 for two test-layout differences, corrected only those; final exit0 (161f97). CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings via managed job ontodoc-od30-clippy-20261110-01: terminal exit0, receipt275702, result.json under /opt/d2/.ontocode/artifacts/compiler-lane/ontodoc-od30-clippy-20261110-01/. GPT-6-Astra source verifier result agent-results/01a1c610-6d9e-7611-93dc-a0668668d98b/2: no blocking findings. Source review only; no independent test claim. IDs/schema/ranking/evaluation guards unchanged; no quality/performance claim.'
```

## OD-3.1

SHA-256: `a9f5fc57967c93cdf1216ec0e13058eaefa5ace154888f219acdf02cb7193949`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-31; SHA256 ec638b42f2dbac324146b0170375d2dcbd1fff8558b61e43029f4a3e00870f2f. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.2

SHA-256: `104442db0afcea02d9146a91b4192cf9cac0bd237b1d38caf50511dbdb77c5d3`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-32; SHA256 89f6ba636154c30118e813adaa9d6f49796b06b17f6239506295582778f9e690. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.3

SHA-256: `d5a77f8108a00c79318722ba2f9175f27d801642ba9629d2ed401b8cf936d690`

```yaml
        validation_summary: 'OD-3.3 changes only src/mcp.rs, src/cli.rs, tests/mcp.rs, tests/cli.rs. Native rmcp Transport wrapper counts complete serialized JSON-RPC plus newline; cap_exceeded fallback preserves ID, unanswerable IDs close before dispatch under documented audit policy. Text/structured parity and five tools preserved; escaping-aware truncation, metadata exhaustion refusal, shared generic internal messages and join/setup redaction. CARGO_BUILD_JOBS=4 cargo test --test mcp --test cli --lib terminal exit0: lib85 CLI4 MCP7, managed job ontodoc-od33-regressions-20261110-v5 receipt2a7ba9; includes raw wire/escaped/exact-cap/success+error fallback/EOF-open/cancellation/privacy regressions. Required focused command also exit0 in v2 (CLI3 MCP7) before final additive regressions. CARGO_BUILD_JOBS=4 cargo fmt --check exit0 receipt e4735e; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings terminal exit0 managed ontodoc-od33-clippy-20261110-v1 receipte59e8e. Earlier compile E0277/E0477 corrected; one admission exit75 respected. GPT-6-Astra final source reviewer no blockers, agent-results/01a1c623-83c5-7cc1-876d-5ce0766aa4ab/5; no independent test claim. Audit audit_session-2026-11-10-mcp-wire-bounds.md records every outcome and limitations. Source/hash fallback after OntoIndex preparation failures; no reindex, no commit, no quality/performance/HTTP/inbound-allocation claim.'
```

## OD-3.4

SHA-256: `3f716a93d91b911c379436c736e70f546260bdd586c6276b67848b4e0d6b7ee1`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-34; SHA256 f0c74c9b0db430fc574a4a8f3649fa2da07d3114c0c8dc7815ecae07f61df390. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.5

SHA-256: `7479892e3a840ba4a34df54af778f99465bfdead36ff6ed945a8e4034703e70f`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-35; SHA256 310147b4ee13e752fdef695145918a0638fa784173ed9822547442ac646f3612. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.6

SHA-256: `79955109100ae4c699e360e69a1ba04a208d827b493108ec6eabca58dcbe94f6`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-36; SHA256 dc5dd2ef05512502413a9c9f2619abd0c4f0bc1a3a8cf21a11b7db65cc4ae8f3. Completion and role state unchanged; not current-tree proof.'
```

## EVAL-1.4

SHA-256: `e7fbe946abb4108346872dbc7dae7cd3286991842c2d253744584519060c2585`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#eval-14; SHA256 9d1cc7388aff11c63cf4c901b8de8b22b147f2ba6c0eb008949917cb59cf4b14. Completion and role state unchanged; not current-tree proof.'
```

## EVAL-1.5

SHA-256: `5760454f484114ef23c9d975fb08045075654f9511c1cd116433343d7599796a`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#eval-15; SHA256 8900fcaf867bf64ece26d0cb0c52512af30712109b26164452bf4652ff52492d. Completion and role state unchanged; not current-tree proof.'
```

## HTTP-0

SHA-256: `73304d15dae2bc5adb20154abafcc739610fd3175e1fdd053a026581e24a520e`

```yaml
        validation_summary: 'ADR revision 10 adds "HTTP-0 Exposure And Numeric Contracts": loopback backend behind authenticated trusted ingress; rmcp stateless JSON mode (no sessions/SSE/replay); exact /<folder>/mcp routing; Host allowlist, Origin refused; 128-KiB request/response, 256 concurrent requests, 4 global/2 per-corpus fail-fast work permits owned by work, reloads inside permits; no payload/credential logging; drain on shutdown; stdio parity. Also section IDs 4096, per-result text_truncated, null empty-group stats. Documentation only; project_plan_validate run after edit.'
```

## HTTP-1

SHA-256: `fafa9305239fd6ba8d809e0974ad090e0fc31d77f6d79f5e597ddc22cdccb554`

```yaml
        validation_summary: 'Lane http1-b exit0: cargo test --lib 197 passed 1 ignored; mcp 12, cli 8; fmt ok; clippy -D warnings ok. src/mcp.rs Registry::discover: immediate subdirectories only, sorted, hidden entries and plain files ignored, URL-unreserved single-segment names (else InvalidName), symlinks/special entries NotDirectory, >32 TooMany, missing dir Unavailable; one OntoDocServer/App/store per folder; exact case-sensitive get with no default. Test docs_share_folders_are_independent_corpora: identical local ids in two corpora keep separate text, generations, read/related/status/verify, cross-corpus citation never verifies, empty folder reports no_index. No listener, no auto-index.'
```

## HTTP-2

SHA-256: `8822733dd0884ca9147a1a0fe3da8a1b7b9719096acbf6c0df62db7f7c15a332`

```yaml
        validation_summary: 'Local run exit0: cargo test --all-targets all suites ok (lib 208 passed 1 ignored before the review fixes); after fixes cargo test --lib http:: 4 passed, --test http 4 passed; fmt --check ok; clippy --all-targets -D warnings ok. src/http.rs: loopback-only serve, rmcp 1.7 stateless JSON, exact /<encoded folder>/mcp routing, Origin 403, Host allowlist, POST only, 128 KiB streaming request bound and complete response bound, 256 connection plus 256 in-flight fail-fast admission, 30 s header/body read deadline, rmcp call detached in a tracked task owning its slot (disconnect cannot leak rmcp oneshot tasks), rmcp non-2xx bodies replaced by a generic body, drain on Ctrl-C/SIGTERM. mcp.rs: global 4 then per-corpus 2 permits fail-fast, moved into spawn_blocking. GPT-6-Astra review: P1 disconnect leak, P2 connection admission/slowloris, P2 rmcp error bodies, all fixed with regressions. Synthetic only.'
```

## OD-3.7

SHA-256: `6886f29ec17c9ae7419abbe1b8298e0c82e96c489ce71140d230ffb049b2cc6b`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-37; SHA256 945dbf116e5aabf61b765bcc1eba0ca9310c97f0f6892716554caeb7aeda8b9b. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.8

SHA-256: `86df1804dea9595df8fc71c022580c05a3de6a3dda11230570e2a2644f283a5a`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#od-38; SHA256 4ad736e47fa811d29f112927ffc0bafe32c563d077119c31a060b6574970fbd1. Completion and role state unchanged; not current-tree proof.'
```

## OD-3.9

SHA-256: `59c80667c64a75dd06addd23d09020622681f803dcb11909fa0575c2ffb76080`

```yaml
        validation_summary: 'Audit audit_session-2026-11-11-od39-publication-outcomes.md binds three allowed implementation files and final hashes. CURRENT rename commits; explicit outcomes/CLI exit0 warnings; namespace/reuse syncs; HISTORY195 and shared precharged100000 cleanup; fixed-shape inventory/current protection. Phase faults and subprocess PID/inode ownership, same/different-store, release/death/full maintenance barriers. Historical parallel Locked cause UNKNOWN, not claimed fixed. Managed c test commands exit0: cargo test --lib store:: 43pass/1 helper ignored, --lib cli::6, --test cli8, --test mcp --test eval_gates --test eval_run 11/14/6 (all CARGO_BUILD_JOBS=4). Job c terminal101 Clippy style findings corrected once; d terminal0 CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings; cargo fmt; cargo fmt --check, receipt299a88. Prior a compile101 annotation and b store101 trusted /tmp alias regression corrected with focused coverage; no repeated unchanged failure. Effective GPT6Astra publication results3-5/process3 no scoped blockers after corrections; source-only. Worker store-only, parent integration; all children closed. Baseline scope332996 only store/cli/testscli; no schema/ranking/dependency/eval/HTTP changes. OntoIndex unknown commit identity, source authority/no graph PASS. No full suite/heldout/performance/RSS claim. No lease or role receipt rejection; strict complete-local required.'
```

## SAFETY-0

SHA-256: `6f2d09c4ba9d2805c263515c0c640fc8872993cd33eb94e69b3ff4dcd565de0f`

```yaml
        validation_summary: 'Historical validation preserved verbatim: audit_session-2026-11-11-tracker-evidence-compaction.md#safety-0; SHA256 cdba0e23a704266f07d8bf65d58c3db2cd4887953749ed6ba0c4a65171bf119b. Completion and role state unchanged; not current-tree proof.'
```

## EVAL-1.6

SHA-256: `9a46b1ce435735b01c620c036020c516d49545b8f73d46ad9b268734fdc94ee3`

```yaml
        validation_summary: ADR9 and future task cards documented; audit_session-2026-11-11-eval16-preparation-decision.md records choices, gaps and evidence. Versioned query-review/human annotation, trusted exact-byte preparation/both-partition counts, selected-only dev access, attempt-before-test-read and completed-custody/published-byte acceptance. Fixed h5-then-h2-v1 recommendation; separate OD-4.1 minimal default/no-op, absent threshold disabled. Effective GPT-6-Astra results4/6 found then verified correction of two document blockers (PPR rationale, exact manager audit authority), receipts b7d213/aa7596 exit0; source-only. Plan PASS40 no errors, line advisory only. Python eight-document references/anchors/fences/whitespace/scope, MEMORY100, protected source/tests/build/eval hashes, all24 compacted scalars/DONE cards preserved and git diff --check exit0 receiptc3a3cf. No Rust/tests/benchmark/labels/HTTP work, no guard removal or performance claim. OntoIndex unknown committed identity; source authority. No lease, receipt rejection or repeated unchanged failure. Strict complete-local required.
```

## FIX-1

SHA-256: `b985679ee993df87392e91cb24435d20f5b8aad5822dcf6a15733b44cf9997b2`

```yaml
        validation_summary: 'Lane fix1-a exit0: cargo test --lib 197 passed 1 ignored; eval_gates 20, eval_run 13, cli 8, mcp 11 passed; cargo fmt --check ok; cargo clippy --all-targets -D warnings ok. New tests moved_lines_count_raw_bytes_after_invalid_utf8 (old lossy code gives line 1, not 3), preparation_inputs_inside_corpus_checkout_are_refused, out_of_range_resamples_refuse_at_admission. Search truncated item rejected on re-check (ADR 8 tier downgrade is intended). Synthetic only.'
```

## SIMPL-1

SHA-256: `d518d2e86b1491280786f306bb6b627da5c32baed1e6e91f28875ab0811be2bc`

```yaml
        validation_summary: 'Lane simpl1-c exit0 (simpl1-a/b exit101 on unused run binding, fixed): cargo test --lib 197 passed 1 ignored; eval_gates 20, eval_run 13, cli 8, mcp 11; fmt ok; clippy -D warnings ok. freeze(bench, threshold, code) computes the dev sweep once (CLI no longer precomputes and compares); unreachable check-benchmark saved-run scoring removed; gates percentile delegates to stats::quantile. stats::weighted_kappa kept (worked-example oracle for gates kappa). Synthetic only.'
```

## FIX-2

SHA-256: `9b5ad8ff6c6b93d941b9b8c9a9ad69aeaac3635c10f21ac78ce7cee9d4dab05d`

```yaml
        validation_summary: 'Lane fix2-b exit0 (fix2-a exit1 on fmt only): cargo test --lib eval::sweep 4 passed; cargo fmt --check ok; cargo clippy --all-targets -D warnings ok. toml_string escapes quote, backslash and control chars (incl. DEL, C1) as JSON/TOML-compatible escapes, other chars verbatim; generation_ids keys are parsed as a leading JSON string so names containing " = " work. New test frozen_config_roundtrips_unusual_corpus_names; existing roundtrip test unchanged; ordinary names keep historical bytes. Synthetic only.'
```

## FIX-3

SHA-256: `c2328a5e8e8d14214ceaebf7a531157b4c8754e3ee76c97e4411b60a8ce78733`

```yaml
        validation_summary: 'Local run exit0: cargo test --lib eval:: 48 passed; eval_run 13, eval_gates 20, cli 8 passed; cargo test --all-targets 256 passed 1 ignored; fmt --check ok; clippy --all-targets -D warnings ok. run_file_bound sums all run files (4 systems + abstention) per pinned query id using the emitted chunk section_id width, checked against the single 16-MiB payload in held_out_admission (manifest-pinned splits.test, before reservation) and cumulatively while writing; score_captured budget back to MAX_INPUT_BYTES. attribute_miss gives chunk_boundary only when the section span contains a query term. scratch param removed from check_pool/check_benchmark. Astra delta reviews: P1/P2/P3 fixed; chunk-vs-section width P2 fixed. Residual: admission ordering covered by code order plus bound unit test, not an end-to-end custody test. Synthetic only.'
```

## FIX-4

SHA-256: `76d06c453ef6dea65ab76d8ab168e6373a3f91eec4866874e5000e7a87431c8b`

```yaml
        validation_summary: 'Local run exit0: cargo test --lib cli:: 7 passed; cli 8, mcp 12 passed; cargo test --all-targets 256 passed 1 ignored; fmt --check ok; clippy --all-targets -D warnings ok. capped_iter takes at most MAX_LIST_ITEMS+1 lazily (clone after take) for related neighbours and status skipped/link_diagnostics; truncated semantics and responses at or below the cap unchanged. New test list_caps_take_at_most_one_extra_item_and_flag_truncation. Synthetic only.'
```

## SIMPL-2

SHA-256: `eb35c9c135024ec8d71f5fcc39d6133e27c93e63a060c48c758be212dba20709`

```yaml
        validation_summary: 'Local run exit0: cargo test --lib eval:: 48 passed; eval_gates 20, eval_run 13, cli 8 passed; fmt --check ok; clippy --all-targets -D warnings ok. run_reserved and score read/verify the registered freeze once (check_held_out_code) and pass it to held_out_preflight, which now only runs check_frozen; check order unchanged, no check removed; CLI check-frozen unchanged. Synthetic only.'
```

## FIX-5

SHA-256: `a374f24668ca48ed72f1bf7ec1570e9c213be4abc1efb09930eef3950ef786ac`

```yaml
        validation_summary: 'Local run exit0: cargo test --lib eval:: 48 passed; eval_gates 20, eval_run 13, cli 8 passed; fmt --check ok; clippy --all-targets -D warnings ok. validate_queries refuses empty/tab/CR/LF ids via check_tsv_field; held_out_run_bound runs check_run_ids on manifest-pinned splits.test before reservation; external_path requires a directory parent; held_out_admission refuses non-NotFound symlink_metadata errors. Regressions in run_file_bound_refuses_unscorable_output_before_writing and partial_custody_and_symlinks_are_consumed_or_refused. Residual: an unwritable directory still fails only at staging (no writability probe, by non-goal). Synthetic only.'
```

## FIX-6

SHA-256: `41710c33dac2724000c140add1a393710542564e049e01d0019bb49504056ece`

```yaml
        validation_summary: 'Local run exit0: cargo test --lib store:: 44 passed 1 ignored; cli:: 7; cli 8, mcp 12 passed; cargo test --all-targets 257 passed 1 ignored (before the chunk-path extension; store/cli/mcp rerun after); fmt --check ok; clippy --all-targets -D warnings ok. build_rooted refuses CapExceeded("citation heading bytes") for any section or chunk heading_path above 64 headings / 16 KiB, sharing MAX_CITATION_HEADINGS/MAX_CITATION_HEADING_BYTES with validate_citation_fields; build-only, no schema bump, existing generations still load. Regression heading_paths_beyond_the_citation_bound_fail_publication (16 KiB ok, +1 refused). Proposed last-# lookup rejected: ADR lines 238-240 keep that limitation and tests/cli.rs caught it. Astra delta: no findings plus one P3 (chunk paths), fixed. Synthetic only.'
```

## FIX-7

SHA-256: `019cf794c91be76789d59ba6b54d3021a5ede60a3818b1c600e171a64fe9283a`

```yaml
        validation_summary: 'text_truncated added (ADR rev10 section 8); Comparison.stats() maps n = 0 to null (ADR rev10 statistics line). Local run exit0 this session: cargo test --all-targets (lib 210 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12); cargo fmt --check ok; clippy --all-targets -D warnings ok. Synthetic only.'
```

## SIMPL-3

SHA-256: `b6e2cc4192b04a54e3d6cb2709e71f5c73890d146225ae383640610ccb165355`

```yaml
        validation_summary: 'Net -10 lines; gates cluster_kappa and stats quadratic_weighted_kappa worked examples pass unchanged. Local run exit0 this session: cargo test --all-targets (lib 210 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12); cargo fmt --check ok; clippy --all-targets -D warnings ok. Synthetic only.'
```

## FIX-8

SHA-256: `3bd39fe7017b8d4e747d28e5f775a59ff7dfb681d9cba623049c4f17bbcf8ea9`

```yaml
        validation_summary: 'Staging moved before reservation; StagingDir::create semantics unchanged; dev runs also stage before ranking. Local run exit0 this session: cargo test --all-targets (lib 210 passed 1 ignored; cli 8, eval_gates 20, eval_run 14, http 4, mcp 12); cargo fmt --check ok; clippy --all-targets -D warnings ok. Synthetic only.'
```

## PERF-1

SHA-256: `bb9e833843f6889064cecf0a9edd2803e5034c46628b57ae7fd89fd4f756ab28`

```yaml
        validation_summary: 'Lane perf1-c exit0 (perf1-a exit101: join-failure test assumed old cache poisoning; updated): cargo test --lib 199 passed 1 ignored; mcp 12, cli 8, eval_gates 20, eval_run 13; fmt ok; clippy -D warnings ok. blocking() holds the index mutex only for CURRENT check/reload/Arc clone; permits move into spawn_blocking; with_permits(1..=64), new() = 1. Tests: admitted_calls_overlap_on_one_snapshot_and_excess_is_busy (30 s bounded barrier, one load, busy before work), in_flight_snapshot_finishes_while_later_calls_see_new_generation, cancellation/shutdown keep permits. GPT-6-Astra review: no P1; P2 registry naming/hidden/file-order/entry bound, prepare layout-before-capture, missing reload-concurrency test, P3 unbounded barrier, all fixed in this lane (Registry lossless names, .git/.ontodoc only skipped, MAX_ENTRIES 4096; capture(trusted) checks layout before freshness/git).'
```
