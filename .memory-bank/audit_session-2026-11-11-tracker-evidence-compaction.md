# Historical Tracker Evidence: Verbatim Preservation

Date: 2026-11-11
Authority: EVAL-1.6 selected local manager documentation work; active tracker parser ceiling.

Original tracker SHA-256: `b08b83e60c44466698776a4b2b15726083ce91b9ae73cb78b96284197d47759c`.
Only the long DONE main-session validation summaries below are replaced by
exact links. Authorization fields, role results, dispatch/receipt/attempt state,
task cards, statuses, dependencies and manager routing remain unchanged.
These are historical receipts, not current-tree validation. Original decoded
YAML scalar values are retained verbatim below; SHA-256 covers UTF-8 scalar bytes.

Verification: receipt `1af195` (exit 0) restored the first ten scalars and
confirmed otherwise identical parsed tracker state; 130,466 → 113,312 bytes.
Receipt `75a598` (exit 0) verifies all 24 replacements against both saved
checkpoints: exact UTF-8 scalar/hash preservation, unchanged DONE cards after
restoration and identical manager state. The second checkpoint reduces
121,434 → 110,286 bytes while retaining EVAL-1.6's new task contracts. The
existing >1,500-line advisory remains; no authenticated state was deleted.

## Second Compaction Checkpoint

Tracker before this continuation: `2dc52eb024528a990c557482954edce6eef3446ebceb769b22211161c8e874c1`.
EVAL-1.6 task-card additions are retained; only the following historical DONE
summary scalars are replaced, with parsed state otherwise identical.

## OD-1.0

Scalar SHA-256: `84105a9a30bb4cb0df0230b1973e65af6e0cae529c1917caa2ac7362c7f4ab00`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- corpus:: exit0 (5 passed); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Tests cover .gitignore (no git repo) and nested .ontodocignore, hidden dirs, file and dir symlinks skipped as typed diagnostics, byte cap and non-UTF-8 skipped without failing, max_files cap error, byte-sorted output identical across runs, record fields rel slash path/size/mtime_ns/sha256 (hand-checked hash). Fixtures generated in tempdirs because symlinks and invalid UTF-8 are not portable as checked-in files. Parent, global and .git/info/exclude rules disabled so the corpus depends only on the root.
```

## OD-1.1

Scalar SHA-256: `7e8cf4743772b284dd61cbc6d29142bd086989f826e44f8479a2251bba1e9ff4`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- parse:: chunk:: exit0 (10 passed); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Fixture tests/fixtures/markdown/sample.md: hand-checked line spans for 7 sections (preamble, ATX, setext, duplicate install/install-1), front matter excluded, headings in backtick, tilde and 4-backtick fences not sections. Section and chunk span bytes hash to recorded SHA-256. Section ids identical for budgets 800/1200/2000; inserting a paragraph keeps section and chunk ids. Oversized section splits only at block boundaries (lens 46/39/39/80 at budget 50), no empty chunk, chunk-to-section mapping total. Design: section extent includes children; chunks pack only own blocks (no double indexing); preamble is level-0 section id `<path>#`; chunk id `<section id>~<ordinal>`; CHUNK_BUDGET=1200.
```

## OD-1.2

Scalar SHA-256: `33b3afd538ffab1bf054f5c6c3b521a8a7ff9c59bd76b07c185df4f6724c2513`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- tiers:: exit0 (5 passed); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Tiers are newline-joined source substrings (heading text without ATX/setext markers, first sentences); every tier line found in source by byte search; L0<=200, L1<=1500, pseudo-doc<=16 KiB, cut at UTF-8 boundary with truncated flag (tested at boundaries 190 and 197). L0 = heading path + first sentence of chunk; L1 = section heading/first sentence + descendant outline; pseudo-doc = all headings and first sentences. First sentence = first paragraph block up to first . ? ! followed by whitespace, else block end; code blocks excluded. Hand-written outline asserted on sample fixture.
```

## OD-1.3

Scalar SHA-256: `a50ec0a8de1969b38b6a7f6551d7891ce2eddcb0116af70919cc61c61648acad`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- graph:: exit0 (4 passed); full lib 24 passed; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Golden tests/fixtures/graph/edges.golden (30 edges) written by hand before the run and matched: CONTAINS doc->sections->children->chunks, NEXT per document, LINKS_TO projected to first chunk of target section/document (relative, ./, ../, percent-encoded paths, title attribute, duplicate-anchor gamma-1), MENTIONS code:<text>; self-links, images, external URLs, non-Markdown targets and links/code inside fences produce no edges. Diagnostics with path+line span: MissingAnchor (#nope), MissingTarget (missing.md), OutsideRoot (../outside.md). Build is deterministic (sorted, deduplicated).
```

## OD-1.4

Scalar SHA-256: `e9f76e2b54ca87413b60d0cef10cf33e87212ccb4f23f4295a042ee9417d382c`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- store:: exit0 (9 passed); full lib 33 passed; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0; no unwrap/expect outside tests. Generation id = sha256(canonical JSON of IndexConfig + [(path, sha256)] manifest); query-time params are not inputs; mtime-only touch keeps id; chunk_budget change moves it; two builds serialize byte-identically. Tests: held lock -> Locked, leftover lock file reclaimed (std File::try_lock, released with process); injected failure before CURRENT swap keeps previous generation; corrupted index.json byte -> Corrupt on load; max_chunks overflow fails build and max_artifact_bytes overflow publishes nothing (no new dir, CURRENT unchanged); 5 publishes keep current + 2 old; artifact contains no absolute path; freshness lists added/removed/modified. Store stays inside allowed write set via private record types (corpus/parse/chunk/tiers/graph untouched). Freshness compares hashes after a full walk; size+mtime fast path deferred (ADR item 9 permits hashing changed files; correctness unaffected).
```

## OD-1.5

Scalar SHA-256: `7fa3923af3e5b88ea6d71e81ae92b354d4567bff2f9c8bba1b5ca43922d57bf1`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- search:: exit0 (10 passed); full lib 43 passed; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. BM25 (k1 1.2, b 0.75, idf ln(1+(N-n+.5)/(n+.5))) on toy d0='a b a', d1='b c', d2='c c c d', query a c: 1.3486402228911236 / 0.5442147286003255 / 0.6893386562270789, hand-derived in test comments and cross-checked with an independent python3 one-liner, asserted to 1e-9. Abstention s=top/sum idf=0.9295627631437958 asserted; NoLexicalMatch, BelowThreshold, and threshold None disables abstention. Tokenizer: Unicode lowercase, camelCase/acronym/digit and snake_case parts plus whole word, no stemming. Ties by chunk id; repeat runs equal; section ranking keeps first occurrence. B0 reimplements prior docs.rs Text matcher (whole query ASCII-lowercased substring of title/headings/first 1 KiB, path order) mapped to each document's first chunk.
```

## OD-1.6

Scalar SHA-256: `475fed0a400d0a760f17408dbfb8cce0958de0b6fc5aed58b6dad9aaf94e7acb`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- search:: exit0 (17 passed); full lib 50 passed; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. PPR path A-B-C plus isolated D, seeds A,D: closed-form fixed point 0.300235017626322/0.39952996474735597/0.16980023501762628/0.13043478260869568 derived in test comment, matched to 1e-6 (undirected, deduped, self-loops dropped, d 0.85, dangling mass to seeds, tol 1e-9, cap 100). B3 doc BM25 over pseudo-docs, two-doc case 0.2696520503843362/0.1654051236893609 to 1e-9. alpha 0 and beta 0 equal B1 element-for-element for 7 queries x top_k 1/3/10 on two fixtures (zero-BM25 dropped, ties by raw BM25 then chunk id). min-max degenerate gives 0. Scope prefix zeroes BM25 and PPR outside scope; no in-scope chunk gives scope_empty. Explain lists bm25, ppr, doc_bm25, B1 rank and score; graph mode surfaces a linked zero-BM25 chunk. Abstention identical in all modes. Note: B1 pool is max(50, top_k) so top_k above 50 still reproduces B1.
```

## OD-1.7

Scalar SHA-256: `66064a5bec70a22e25901c0e531529b0d34af51a24977a94b9ab2db6fc4b254f`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- cite:: exit0 (3 passed); full lib 56 passed; cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Four states on one citation: unchanged valid; paragraph inserted above gives moved with new span (+7 bytes, lines 7-9); edit inside span stale; deleted file missing. Span hash occurring twice after edit is stale, not moved. Symlinked, ../ and absolute paths are missing (reads beneath root, no symlink following, size-capped). Citation carries generation_id, path, heading_path, byte and line span, sha256. Resolves plan/ADR citation conflict per ADR item 10 (moved requires unique hash match).
```

## OD-1.8

Scalar SHA-256: `739cfa451ac96888960d0ebf718da124cb442ae0e6d3f1b4ffb6116cfb9eee91`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- assemble:: exit0 (3 passed); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Breadth-first: L0 for all in rank order, then L1, then L2 upgrades in rank order; non-fitting upgrade keeps prior tier and later results may still upgrade; tiers never truncated. 500 SplitMix64-seeded generated cases: total <= budget, total equals sum of chosen tiers, identical output on rerun, and the result is maximal (no single tier step still fits). Equal-size results give tiers monotone non-increasing by rank for every budget 0..600. truncated flag set when any result is below L2.
```

## OD-2.0

Scalar SHA-256: `3a0eaba1c31d909404d5dbd9aadd9198c9d77977fe704b25680bf79435d86978`.

```text
CARGO_BUILD_JOBS=4 cargo test --test cli exit0 (2 passed); CARGO_BUILD_JOBS=4 cargo test exit0 (56 lib + 2 cli); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Binary runs against temp copy of tests/fixtures/graph: index (4 docs, 8 chunks, 3 link diagnostics, same 64-hex generation id in a second copy), status fresh, search graph+explain byte-identical across runs with citation and bm25_rank, abstention no_lexical_match, budget_bytes 40 respected, read by path#anchor (section) and chunk id, related lists LINKS_TO out-neighbours, verify valid then moved (line 3) after an edit, status/search report stale b.md. Missing index on status/search/read/related exits 1 with stderr JSON error no_index; not_found and invalid_argument (top_k 0, alpha 2, bad JSON) are typed. src/cli.rs is the shared application layer (App, typed bounded responses, MAX_RESPONSE_BYTES 128 KiB, truncated flags) for OD-2.1 to reuse.
```

## OD-2.1

Scalar SHA-256: `9e8e0b763c3909e92d31cfeb71214732f56daae12c24a2cb76ceecc1f611ac8a`.

```text
CARGO_BUILD_JOBS=4 cargo test --test mcp exit0 (4 passed); CARGO_BUILD_JOBS=4 cargo test exit0 (56 lib + 2 cli + 4 mcp); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0; no unwrap/expect/print to stdout in non-test code. rmcp 1.7 tool_router over tokio duplex: tools/list returns exactly read, related, search, status, verify, all read_only_hint true with object input schemas; search schema has query, mode, top_k, scope, explain, budget_bytes; default mode lexical. Tools return structured JSON with citations (generation_id, sha256) and fresh flag; edit reported via status modified and verify stale without reindex. Invalid args (top_k 0, alpha 2, bogus mode, missing query, unknown id/chunk, malformed citation) return MCP errors. 60-section corpus search top_k 50 sets truncated with serialized size <= 128 KiB. Real `ontodoc serve` stdio session: every stdout line is a JSON-RPC frame, no extra output after stdin closes, exit 0. Index reloads when CURRENT changes. Cargo.toml adds dev-only rmcp client feature and tokio io-util; Cargo.lock sha256 d91f1c8ed5e8617f113b7f757cab11f756b5ccd747c711bba309d0fe3097a069. `serve` subcommand added to cli.rs/main.rs.
```

## EVAL-1.0

Scalar SHA-256: `0744b5ad8005c7ed8e71bd1ae9a3a2d318568272006a0191bcf4159c504feac3`.

```text
CARGO_BUILD_JOBS=4 cargo test --lib -- eval:: exit0 (16 passed); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. std-only. Metrics with written-out arithmetic, independently cross-checked by python3: nDCG@10 0.4499200626718163, nDCG@2 0.38685280723454163, condensed nDCG' 0.4683480347412084, Recall 2/3, MRR 0.5, judged@10 0.75; no-relevant queries and duplicate ids rejected (MetricError). SplitMix64 seed-0 outputs 0xe220a8397b1dcdaf, 0x6e789e6aa1b965f4 match reference. Stratified paired bootstrap (percentile, type-7) and sign-flip randomization (+1 corrected): identical systems p 1.0 and CI contains 0; constant +0.1 CI [0.1,0.1]; fixed seed reproducible; oracle vs noise p<0.01; bootstrap CI within 0.005 of normal approximation. Holm textbook (0.01,0.04,0.03,0.005)->(0.03,0.06,0.06,0.02). Wilson 0/10,5/10,10/10 match closed form and Newcombe 1998 tabulation. Quadratic-weighted kappa worked example 10/13. z quantile (Acklam) within 1e-8; required_n sd 0.2 -> 153, sd 0.15 -> 86 (hand calc). decide(): pass only mean>0, lo>0, holm p<alpha; significant negative -> negative; wide CI -> inconclusive.
```

## EVAL-1.1

Scalar SHA-256: `54077fb704190a1ea71e65e608ee733655c0d688f81400cf529a7b9799787214`.

```text
CARGO_BUILD_JOBS=4 cargo test --test eval_run exit0 (3 passed); CARGO_BUILD_JOBS=4 cargo test exit0 (72 lib + 2 cli + 3 eval_run + 4 mcp); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. src/eval/run.rs + `ontodoc eval split|pool|run|score`. Split stratified by (corpus, stratum), SplitMix64 seeded, (n+1)/3 dev, byte-identical on rerun. Pool = top-10 sections of B0, B1, B2 per alpha_grid, B3 per beta_grid for non-off-topic queries, sorted/deduped JSONL, byte-identical. Run writes TREC-style tsv per system, abstention.tsv, meta.json with config/queries/qrels/splits sha256, corpus generation ids, git commit+dirty, binary sha256, seed; test split refused (typed error test_split_refused) without --confirm-test-split for run and score. Ranking metrics abstention off on answerable queries; H3 correct/false abstention Wilson rates on off-topic/answerable. Report: per-query metrics, per-system/corpus/stratum means, judged@10, H1 (sanity), H2/H5 (Holm over {H2,H5}) with decisions, E1 exploratory without decision. Score re-scores saved run identically; dev runs byte-identical. Discrimination: oracle vs shuffle p<0.01, self p=1. Fixture sanity: B1 nDCG 0.962 vs B0 0.0 (H1 pass).
```

## EVAL-1.2

Scalar SHA-256: `3948218febdce9fdf51c8578f763e817329dc74a25aaa46ad16e7fdae53dbfbc`.

```text
CARGO_BUILD_JOBS=4 cargo test --test eval_gates exit0 (10 passed); CARGO_BUILD_JOBS=4 cargo test exit0 (74 lib + 2 cli + 10 eval_gates + 3 eval_run + 4 mcp); cargo fmt --check exit0; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0 (after fixing needless_lifetimes in src/eval/sweep.rs split_queries); no unwrap/expect/println in non-test code of src/eval/gates.rs, src/eval/sweep.rs, src/eval/mod.rs, src/cli.rs. src/eval/gates.rs + src/eval/sweep.rs + `ontodoc eval check-benchmark|sweep|freeze|check-frozen|misses`. Planted defects each fire exactly their gate: judged_section_missing, span_hash_mismatch, dev_test_leak (token Jaccard >= 0.8), pool_not_reproducible, judged_below_floor (judged@10 < 0.95), corpus_wrong_commit, corpus_dirty, corpus_not_a_checkout, answerable_floor, off_topic_floor, stratum_floor; --stage pools|qrels|run selects gates, run without --run gives run_report_missing; clean fixture passes pools and qrels. Sweep refuses test split, writes grid, per-arm dev sd_diff and required n (floor 150). Misses attributed to exactly one of not_indexed, chunk_boundary, lexical_miss, fusion_displaced, truncated. Freeze refuses dirty tree, frozen-config.toml records params, [power], commit, Cargo.lock sha256, binary sha256; check-frozen refuses dirty tree, different commit/Cargo.lock and different binary hash (typed error gates_failed).
```

## OD-3.1

Scalar SHA-256: `ec638b42f2dbac324146b0170375d2dcbd1fff8558b61e43029f4a3e00870f2f`.

```text
Documentation only in approved ADR, architecture and plan, plus authorized memory audit/router updates. Accepted service ADR revision5 specifies schema2: ID-only @empty for empty allocated heading components, preserved anchors/read precedence/chunk budget, no migration aliases; omit persisted mtime, captured-byte size, canonical skip paths/reasons sorted and included in identity, exact index/meta same-ID reuse or conflict refusal, incompatible versions and retired field rejection. Current-byte freshness and verify preserved; single LF joins copied fragments, no diagnostics capabilities accepted. OD-3.2 evidence records synthetic edge cases, independent-store full bytes with explicit mtimes, restored-mtime stale edits, diagnostic/ID/schema/conflict tests. Two requested/effective GPT-6-Astra source reviewers: identity result agent-results/01a1c638-6a41-7cb2-aa1c-4b2242b5dbe2/3 no blockers; canonical result agent-results/01a1c638-9a4e-77d2-97bf-dba4eb6f6600/3 retention ambiguity corrected to preserve ordinary pruning. project_plan_validate PASS35 tasks no errors; known plan-length warning. Python links/anchors/fences/whitespace/status checks exit0 and git diff --check exit0 receipt78dba3; hash inventory substitutes for empty untracked diff. Audit audit_session-2026-11-10-identity-canonical-decision.md records alternatives and evidence. No Rust validation/code/dependency/eval-data change; no retrieval/performance claim. OntoIndex unchanged degraded state uses recorded source fallback without retry/reindex.
```

## OD-3.2

Scalar SHA-256: `89f6ba636154c30118e813adaa9d6f49796b06b17f6239506295582778f9e690`.

```text
OD-3.2 implements accepted ADR revision5 schema2 in src/parse.rs and src/store.rs, with synthetic regressions also in src/chunk.rs, src/graph.rs and tests/cli.rs. Empty allocated ID components use @empty only; anchors, exact read precedence, delimiter filenames, preambles, ordinal chunk IDs and cross-budget identities preserved. Persisted mtime removed, captured-byte size and canonical sorted multiplicity-preserving skips included in generation hash. Build/publish/load version, retired-field and recomputed-ID refusals; same-ID index AND metadata bytes must match or preserve target/CURRENT/HISTORY on conflict. Independent roots with explicitly different mtimes match Generation/index/meta/CURRENT/HISTORY bytes under identical initial publication; mtime-only fresh and restored-mtime equal-size edits stale. Required combined CARGO_BUILD_JOBS=4 cargo test --lib --test cli terminal exit0: library92 CLI5, managed ontodoc-od32-schema2-tests-20261110-b receipt1eae39; earlier managed -a exit75 shared lane lock respected and owner completion established before retry. CARGO_BUILD_JOBS=4 cargo fmt --check exit0 receipt5f2ecf; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings terminal exit0 managed ontodoc-od32-schema2-clippy-20261110-a receipt4d70dd. Requested/effective GPT-6-Astra source reviewer agent-results/01a1c647-bc73-7330-b3e0-fb633c3aa3a0/1 and /2 no blockers; suggested mtime/skip checks incorporated before validation, no independent execution claim. Pre-edit 70-file SHA inventory compared after validation changed exactly five allowed code files and removed none; untracked/no-commit checkout makes empty git diff insufficient. Authorized memory/architecture audit/router updates record current status. Audit audit_session-2026-11-10-schema2-implementation.md records outcomes and limitations. OntoIndex unchanged degraded preparation/commit identity uses source fallback; no retry/reindex. No deps/donor/eval/ranker/HTTP changes or quality/performance claim.
```

## OD-3.4

Scalar SHA-256: `f0c74c9b0db430fc574a4a8f3649fa2da07d3114c0c8dc7815ecae07f61df390`.

```text
ADR8 retained-root rustix1.1.4 fs openat2/RawDir, shared bounded policy traversal, App root-bound CURRENT/meta/artifact/build/freshness and selected-ID load; full-batch exact chunk/whole-section membership before targets, generation_unavailable/invalid_citation and typed filesystem failures implemented. Valid/moved/ambiguous/stale/missing preserved; no arbitrary-path/HISTORY verification. Synthetic root replacement before/after load, pin/publication, identical roots, adapter caps/redirects, hidden/ignored membership, every field, lower-tier/truncated sections, growth/shrink/unknown/nonregular/deleted-directory/unsupported/race checks. Requested/effective GPT6Astra explorers 01a1c6f9-bc93-7000-a420-a771e8677083 and 01a1c6f9-bd13-7eb3-a739-ad267e053d81 final result3: no concrete scoped blockers after findings corrected; static only, children no builds. Worker01a1c6fe-982a-7601-a079-3ea287b00177 result3 wrote corpus/Cargo only. All agents closed. Managed job ontodoc-od34-validation-20261110-e receipt3ff89b: cargo test --lib corpus::, cite::, store::, --test cli --test mcp, --test eval_gates --test eval_run, --lib cli:: each exit0 by serial chain; gates14/run6/App4 visible, prefix truncated no invented totals; subsequent Clippy101 three test-only clone lints corrected. Managed lint-20261110-f receiptcaf03e: CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings && cargo fmt && cargo fmt --check terminal exit0. Earlier compile/fixture failures and corrections, all managed receipts and exact commands in audit_session-2026-11-10-od34-rooted-membership.md. Baseline /tmp/ontodoc-od34-baseline-6qe8s6q1.json scope/hash receipt62d87a confirms exactly nine allowed implementation paths; doc/link/fence/whitespace/memory-cap and scope check plus git diff --check exit0 receipt6f21ab. No initial commit, empty diff not scope proof. OntoIndex workspace_busy/unknown committed identity: source authority, no graph PASS/reindex. No ranking/schema/publication redesign/donor/real heldout/HTTP changes; no full-library/quality/performance/hostile-admin proof. OD3.6 streaming/load/construction, OD3.5 input/hash/admission and OD3.9 publication/parallel Locked cause remain open. Strict complete-local required; no receipt/evidence rejection or active lease.
```

## OD-3.5

Scalar SHA-256: `310147b4ee13e752fdef695145918a0638fa784173ed9822547442ac646f3612`.

```text
ADR8 raw128KiB native input guard, borrowed decoded64KiB/per-field admission before typed conversion, shared verification read128MiB/SHA128MiB/1000000-call budgets and one closure-owned actual worker/no queue implemented. Counter/barrier tests cover zero work on refusal, all five tools, cancellation, runtime shutdown, cold reload, poison/permit recovery and immutable cache reuse. Four implementation paths changed (cite/cli/mcp and tests/mcp); store unchanged. Source scope receipt eb69ea records final hashes and preserved unattributed AGENTS delta; no Git baseline exists. Managed job a receipt358745, reharvest eb69ea, terminal101: cite13/MCP12/App5 passed, wire exact64KiB failed because tool name was counted. Corrected to argument-only accounting. Job b receipt bcb33f terminal101: stale exact-cap key test expected refusal; corrected exact acceptance/cap+1 refusal. Job c receipt61891b terminal101: serial cite/mcp/cli units, MCP11, CLI7, gates14/run6 passed; Clippy then flagged equivalent int_plus_one test assertion. Prefix truncated, no invented unit totals. Corrected comparison; job d receipt ac758a terminal0: CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings then cargo fmt and cargo fmt --check. Exact commands/failures/limits in audit_session-2026-11-10-od35-input-work-budgets.md. Requested/effective GPT6Astra reviewers 01a1c720-b688-73f2-b973-f56c4f7585cb result3 (CLI) and 01a1c720-b667-75e0-8010-faa8256ff7e1 result3 (final MCP/helper delta) found no scoped blockers; static only, all children closed. OntoIndex unknown committed identity, final verify_diff refused; source authority, no graph PASS. No dependency/schema/ranking/donor/heldout/HTTP changes, no full-library/performance/RSS/quality claim. Distinct failures each corrected once, no repeated same failure, rejected receipt or active lease. Strict complete-local required.
```

## OD-3.6

Scalar SHA-256: `dc5dd2ef05512502413a9c9f2619abd0c4f0bc1a3a8cf21a11b7db65cc4ae8f3`.

```text
ADR8 shared visitor/collecting build wrapper, text-free globally sorted freshness manifest, local/residual source-control budgets and cap+1 detection accounting implemented; raw collection preflight counts duplicate keys before typed records; checked actual heading/span/pseudo-document tokenizer bytes before BM25 with shared first-text/last-hierarchy map. App try_new and shared captured decoder cover service/eval; trusted public new compatibility limit retained. Exactly five allowed implementation files changed, baseline/final source hash scope receipt fcc612. Synthetic policy/current-byte/ordering/reservation/error/duplicate-key/adapters/tokenizer/overlap/overflow and exact BM25/B1/B2/B3 parity checks. Managed focused job ontodoc-od36-focused-20261110-a receipt05a5a8: search:: and corpus:: exit0 (corpus28 visible), store26 pass/1 fixture fail exit101; sorted Value encountered documents before files. Fixture changed to struct serialization, no production weakening. Managed validation-20261110-b receipt77b3dd serial chain: CARGO_BUILD_JOBS=4 cargo test --lib store::, --test cli, --lib cli::, --test mcp, --test eval_gates --test eval_run, cargo clippy --all-targets -- -D warnings, then cargo fmt and cargo fmt --check all exit0. CLI7/App4/MCP9/gates14/run6 visible, prefix truncated no invented totals. Requested/effective GPT6Astra explorers 01a1c712-2482-7690-bd25-15af775068be result2 and 01a1c711-fa9d-7810-bcbf-ddcc171dd202 results2/3 found no scoped blockers; static only. Corpus/search workers completed disjoint files, children no builds; all agents closed. Audit audit_session-2026-11-10-od36-streaming-admission.md records exact commands/failure/limits. OntoIndex unknown committed identity; source authority, no graph PASS/reindex. No schema/ranking/dependency/donor/heldout/HTTP changes; no full-library/performance/RSS/quality claim. OD3.5 input/hash/actual-work and OD3.9 publication/Locked cause remain open. No active lease, evidence rejection or repeated failure retry; strict complete-local required.
```

## EVAL-1.4

Scalar SHA-256: `9d1cc7388aff11c63cf4c901b8de8b22b147f2ba6c0eb008949917cb59cf4b14`.

```text
EVAL-1.4 complete scoped capture implementation: physically separate dev/test files, opaque identity-bound mechanical evidence, shared captured store decoder and Arc indexes, admitted corpus freshness/pins, in-memory pool checks, exact saved-byte scoring, no public measurement enablement. This continuation fixes ignored-entry/ignore-file budget escape in src/corpus.rs with pre-filter std enumeration and public ignore matchers, shared source handling and final nofollow/nonblock opens; src/eval/run.rs shares budget and tests typed control refusal. Existing ADR prohibits symlink controls; bounded path refuses unsafe/unreadable controls, preserving successfully readable regular-control parity. Ordinary walker conflict/ancestor containment remain OD-3.4/3.6; no HTTP safety acceptance. Requested/effective GPT-6-Astra agent01a1c682-3937-7650-b2db-c0ed29d7ecfd results1/2 select and approve scoped disposition; result3 corrections applied; result4 no remaining source-level delta blockers, static review only. Managed traversal-final-20261110a receipt52ceca command CARGO_BUILD_JOBS=4 cargo test --lib && cargo test --test eval_gates --test eval_run && cargo clippy --all-targets -- -D warnings exit0:107lib+14gates+6run; traversal-protocol-20261110a receiptbf9ebe cargo test --test cli --test mcp exit0:5cli+7mcp. 139 total final tests. Earlier focused traversal/integration jobs exit0. Initial fmt check exit1; cargo fmt && cargo fmt --check plus git diff --check exit0 receiptf8eddf after final compiler jobs, no substantive change or test rerun after formatting. Hash source corpus2c79e893013a33dc555995706630f8c946d396ddcaf99bf76589473776876247 run8346655e8bf57bede3fcf96c2b53f33362cf9bec87f840cfc21d3dca043cb314. No real eval labels/results, donor, ranking, dependencies/schema, HTTP or claim changes. OntoIndex workspace_busy/unknown committed identity; source authority, no graph risk PASS. Audits audit_session-2026-11-10-eval14-input-capture.md and audit_session-2026-11-10-eval14-bounded-traversal.md retain earlier failures/limits. No dispatch lease; local null role scaffold preserved; strict complete-local required before status advance.
```

## EVAL-1.5

Scalar SHA-256: `8900fcaf867bf64ece26d0cb0c52512af30712109b26164452bf4652ff52492d`.

```text
ADR revision6 before code selects one write-once Git-common-dir/ontodoc-eval registration and consumed started/result files, external exact-byte export/audits; EVAL3 commands, output scopes and dev-selection recapture rebound. run.rs binds capture/gates/pool/index/both-partition commitments plus code/freeze identity; exclusive fsync start before computation; failed/partial/interrupted remain consumed; completed exact meta/report hashes bind saved ranking hashes. score parses verified buffers; Astra P2 eliminated by deleting score_inner and directly passing constructed meta to score_captured. CLI register/check wiring and synthetic no-custody side-effect asserts, no public guard lifted. Worker01a1c69c-0432-7fc1-905f-20fdf9e99287 result2 source implementation only; parent owns validation. Requested/effective GPT-6-Astra agent01a1c694-91e5-76c1-bae8-387b503d5aa5 results1/2 design/docs findings addressed, result4 P2 addressed, result5 correction closed no blockers, static review. Managed reviewed-20261110a receiptc2908f exact cargo fmt && cargo fmt --check && cargo test --test eval_gates --test eval_run && cargo test --lib -- --test-threads=1 && cargo clippy --all-targets -- -D warnings exit0; library112, integration passed by command chain; prefix truncated, no fabricated integration counts. Protocol job receipt10215d exit0 5CLI+7MCP. API job receipt130140 exit0 13run unit tests plus fmt/Clippy; exact registered API synthetic success and raw export/code/capture tampering. Initial tests job receiptb787eb exit101 fixture canonical /tmp error fixed plus unchanged parallel store Locked; serial112 rerun passes, no store repair claim. Docs links/fences/whitespace/memory-cap and plan validation pass. Source hashes cli c24d405e3bff6e3db300b8e87b739ce72522e8d0d5748fc3deb033e2440d41eb run a50c2a66f019cbc3ca85a72a1fba212223c3398159031175bf4b7d88ebe8896b. Four actual source/test owners only; untracked checkout no initial commit so git diff insufficient; exact baseline source hashes compared. Audit audit_session-2026-11-10-eval15-attempt-identity.md. OntoIndex workspace build failed, impact/verify unknown committed identity, source authority no graph PASS. No real heldout payloads/results, claims, dependency/ranking/schema/ID/HTTP/donor changes. Local mutable custody not independent provenance. EVAL1.3 retains test capture/admission, preparation counts, provenance and full success/failure caller integration; measurement universally refuses. No active lease; strict complete-local required.
```

## OD-3.7

Scalar SHA-256: `945dbf116e5aabf61b765bcc1eba0ca9310c97f0f6892716554caeb7aeda8b9b`.

```text
OD-3.7 docs only in README.md, AGENTS.md and project_architecture.md plus authorized audit/router/tracker updates. Root Rust installation, stdio client config, five tools and CLI shapes checked against Cargo/toolchain/CLI/MCP owners; schema2 and trusted schema1 reindex, snapshot/freshness/verification differences, bounded outbound versus unbounded inbound, unresolved I/O/membership/work/cancellation and disabled measurement clearly documented. Donor provenance retained as separate read-only nonbuildable snapshot section; HTTP design explicitly unimplemented and HTTP-0 owner exposure/admission gate remains blocked. Parent Python document checker and git diff --check exit0 receipt2791b8: local links/heading anchors/fences/whitespace, parseable client JSON, command/tool/flag source checks, memory<=100 lines. Plan validator PASS35 no errors, existing length warning only. Requested/effective GPT-6-Astra reviewer agent-results/01a1c656-a8f5-78b1-a8fb-4585c3d3094f/1 no blocking factual/security/compatibility findings; scoped diff check exit0 receiptb79541 and no Rust/runtime claim. SHA scope inventory verifies only approved docs/scaffold before authorized closeout, runtime/tests/dependencies/eval payloads unchanged; untracked no-commit checkout makes empty git diff insufficient. Audit audit_session-2026-11-10-operations-docs.md records exact evidence and limitations. No Rust build/test/fmt/Clippy, install smoke, listener, real evaluation, reindex or quality/performance claim in this doc task.
```

## OD-3.8

Scalar SHA-256: `4ad736e47fa811d29f112927ffc0bafe32c563d077119c31a060b6574970fbd1`.

```text
OD-3.8 code only src/store.rs: bounded raw CURRENT128/meta4096/artifact512MiB, canonical lowerhex ID before joins, nofollow/nonblock regular descriptor opens and cap+1 actual reads, bounded publication serde writer, shared decoder configured/global checks and conservative target refusal preservation. Six new synthetic tests; schema2/ranking/eval guards unchanged. Managed job ontodoc-review-store-20261110d result.json terminal exit0 canonical reader a0830e: cargo fmt && cargo fmt --check; CARGO_BUILD_JOBS=4 cargo test --lib store:: 23 passed; CARGO_BUILD_JOBS=4 cargo test --test cli --test mcp --test eval_gates --test eval_run CLI5 MCP7 gates14 run6 passed; CARGO_BUILD_JOBS=4 cargo clippy --all-targets -- -D warnings exit0. Receipt tail truncated, terminal success and all test totals visible; full output retained. Initial a/c exit75 lock refusals respected, owners finished before retries; b tests passed then Clippy unused_unit exit101 fixed once. Final source SHA4055ad1393a355fd5a2211207b535b4d06ac405e25dc0eb5c62159c2878584d6. GPT6Astra source/delta reviewers /challenge_service results /2,/3 no blockers (pre-final-format hash17bd); formatting only after /3. GPT plan delta /challenge_plan /3 no dependency/ADR conflicts. Plan PASS39, doc links/anchors/fences/whitespace/router cap + diff-check exit0 fa7b05. Hash inventory e3737f only src/store.rs and seven authorized memory/ADR/plan documents changed; no deps/tests-other/eval/ranker changes. OntoIndex workspace build and unknown committed identity failures mean source authority, no graph PASS or reindex. Audit audit_session-2026-11-10-review-safety-priorities.md records scope/limits and all failures. Full library suite, real benchmark, exact512MiB serialization, resident-memory bounds, ancestor containment, HTTP and lock-defect repair not claimed.
```

## SAFETY-0

Scalar SHA-256: `cdba0e23a704266f07d8bf65d58c3db2cd4887953749ed6ba0c4a65171bf119b`.

```text
Accepted ADR8 concrete rustix1.1.4 fs retained-root openat2/RawDir reader, root-bound generation/doc reads and exact active whole-section/chunk membership; old/unknown generation and nonmember refuse before target access. Numeric local raw/decoded input, traversal/freshness, load/construction, verification SHA and one actual worker/noqueue ceilings with closure-owned cancellation admission; no RSS/latency claim. CURRENT rename is commit with explicit durability/maintenance outcomes, namespace/reused-candidate sync preconditions and bounded HISTORY/fixed-shape cleanup. Four dependent cards and architecture updated in allowed scope. Requested/effective GPT6Astra explorers 01a1c6e5-4a8f-7a02-a2c0-ceccd5e9bbf4 and 01a1c6e5-4ad0-7970-86c8-e2cbdfb3035b discovery result1/delta result2: no remaining assigned blockers; static only, no child writes/tests. Namespace durability finding corrected before final review; trusted publication ancestors explicitly clarified. project_plan_validate PASS39 no errors after substantive changes; line/parser-size warnings retained. Python reference/anchor/fence/whitespace/hash-scope check terminal exit0 receipt104a44, earlier1d8a6b/1e11ad exit0. Scope baseline /tmp/ontodoc-safety0-baseline-87ra7rc6.json confirms only ADR/plan/architecture plus authorized manager audit/status changes; code/tests/Cargo/eval payloads unchanged. Audit audit_session-2026-11-10-safety0-local-contracts.md. OntoIndex committed query unavailable indexed identity: source authority, no reindex/graph PASS. No Rust build/fmt/Clippy/kernel/race/benchmark run because docs-only; no implemented safety, publication lock fix or quality claim. HTTP0 remains owner-blocked and EVAL2.0 guarded; no listener or model-for-human substitution. No active leases, evidence corrections or rejected receipts; strict completion required.
```
