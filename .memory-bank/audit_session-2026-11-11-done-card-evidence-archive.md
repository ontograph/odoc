# DONE Task-Card Expected-Evidence Archive

Date: 2026-11-11
Authority: user review/consolidation instruction; tracker at 93% of the 131,072-byte
parser ceiling.

Tracker before archival: `427dd4de2f3977a349ccd13268b3adfc41249f12889ce24eb3acd52a07857586`. For each DONE card, only the
`expected_evidence` list is moved here verbatim. In the plan it is replaced by
one reference line. Titles, statuses, dependencies, write sets, validation,
closeout evidence and manager routing are unchanged. Each SHA-256 covers the
UTF-8 bytes of the original YAML lines. This is historical acceptance text,
not current-tree validation.

## OD-0.0

SHA-256: `524b2be1019ce38aa7fce13e2a4a59eb022d964268fa821a5f9a5461bd84ba1c`

```yaml
    - Package `ontodoc`, edition 2024, toolchain 1.95.0; dependencies limited to pulldown-cmark 0.13, ignore, serde, serde_json, sha2, thiserror, anyhow, clap 4, tokio, rmcp 1.7 (features server, macros, transport-io), schemars 1; dev-dependencies tempfile, pretty_assertions.
    - An empty `[workspace]` table in `Cargo.toml` makes the package its own workspace root, so `ontocode-rs/` is never resolved.
    - '`.gitignore` covers `target/` and `.ontodoc/`.'
```

## OD-1.0

SHA-256: `d764553f835b0faa05489d333b23172d419f2ee6b8309656e3864d0bb7d8b05d`

```yaml
    - Tests prove .gitignore and .ontodocignore rules apply, symlinks are skipped, files above the byte cap and non-UTF-8 files yield diagnostics without failing the walk, and output order is byte-stable.
    - Each file record carries relative slash path, size, mtime and SHA-256.
```

## OD-1.1

SHA-256: `a43869c6d173468bb496f5e12d0c57c3befdc8059127bfe232e2d8f7c646edc3`

```yaml
    - For every section and chunk, `source[byte_start..byte_end]` hashes to the recorded SHA-256 and line spans match a hand-checked fixture.
    - Section ids are identical for chunk budgets 800, 1200 and 2000; inserting a paragraph above a section leaves later section and chunk ids unchanged.
    - Headings inside fenced code (backtick and tilde, longer closing fences) are not sections; front matter is excluded.
    - Sections above the budget split only at block boundaries; no chunk is empty; chunk-to-section mapping is total.
```

## OD-1.2

SHA-256: `09371c03c70e077ab129218c7ac3e4025571c08c3692e12f2ff9898b7ac2a9db`

```yaml
    - Tests assert each tier is a concatenation of source substrings (checked by byte search) and respects its cap at a UTF-8 boundary.
    - First-sentence extraction is defined (up to the first `. `, `? `, `! ` or block end) and tested on fixtures.
```

## OD-1.3

SHA-256: `887ecbfa6575796287f0ca4aadd6cbcd750180519d0a5cf376f10fbf980f00b8`

```yaml
    - Golden edge list for the fixture, sorted and deterministic.
    - '`a.md#anchor` resolves to that section''s first chunk; `a.md` resolves to the document''s first chunk; targets outside the root and missing targets are reported with source span.'
```

## OD-1.4

SHA-256: `dd4b83ddbe46145898efd5d87272d4887189eff9450bbb932202b88e5c9ff47a`

```yaml
    - Generation id equals SHA-256 of corpus manifest plus index-time config; changing a query-time parameter leaves it unchanged; two builds of the same corpus are byte-identical.
    - Tests cover a held lock, stale-lock reclaim, failure before pointer swap leaving the previous generation active, artifact hash mismatch rejected on load, cap overflow publishing nothing, pruning to two old generations, and repository-relative paths only.
    - Freshness lists added, removed and modified paths after an edit.
```

## OD-1.5

SHA-256: `7771b7ef1c830d4e88bc4d63d8150b85ac513c2ade8e5a7ce2142307c5f7d60f`

```yaml
    - BM25 on a three-document toy corpus matches hand-computed scores to 1e-9 with k1 1.2, b 0.75.
    - Abstention score `top_bm25 / sum(idf(query terms))` matches a hand-computed case; reasons `no_lexical_match` and `below_threshold` are produced; abstention can be disabled for evaluation.
    - Tokenizer splits camelCase and snake_case and lowercases Unicode; ties break by chunk id; repeated runs are identical.
```

## OD-1.6

SHA-256: `07ccbfec2ff2cf132555a48c0da57d14b859708a9c09515f66dde9e4bf82341d`

```yaml
    - PPR on a four-node graph with one dangling node matches hand-computed power iteration (damping 0.85, dangling mass to seeds, L1 tolerance 1e-9, cap 100).
    - B3 doc scores come from BM25 over document pseudo-documents; a two-document case matches to 1e-9.
    - For a 0 and beta 0 the ranked list equals B1 element for element on every fixture query.
    - Scope with no matches yields abstention reason `scope_empty`.
```

## OD-1.7

SHA-256: `69bf8d1aa46db3fe9fcf8750bc0681a71bf02485ba8ef05a1f20fca80ad682b9`

```yaml
    - Four-state test passes; a span hash occurring twice after the edit is `stale`, not `moved`.
```

## OD-1.8

SHA-256: `1fab700da595a2bc8cf601f128048f9d24ceeb45e3f148104985aee19e87e564`

```yaml
    - Generated rank lists and budgets assert total bytes <= budget, no partial tier, monotone tiers by rank, and identical output across runs.
```

## OD-2.0

SHA-256: `89c0caa5f09f92ada4811f0032c3afd32b78f1d367b397c42760f0c020fceeb3`

```yaml
    - Integration test runs the built binary against a temp copy of the fixture and asserts JSON shape and exit codes.
```

## OD-2.1

SHA-256: `319d39a14add4be141247a2df7a60aa398185d166471ee35af7a5c73780bd919`

```yaml
    - '`tools/list` returns exactly search, read, related, verify, status, all annotated read-only.'
    - '`search` accepts mode lexical|graph|hier, scope, explain and budget_bytes; the default mode is lexical until EVAL-3.2 decides otherwise.'
    - Invalid arguments return MCP errors, not panics; every response stays under the byte cap with `truncated` set when items were dropped.
    - Stdout carries only protocol frames; logs go to stderr.
```

## EVAL-1.0

SHA-256: `8f0a6c125bfdb881affe778593ad663b024c3e1448b53391029bb4d502e7473c`

```yaml
    - Metric tests use worked examples with written-out arithmetic; queries with no relevant item are rejected by the metric API, not scored.
    - Decision function returns pass only for mean > 0, CI lower bound > 0 and Holm p < alpha; a significant negative difference returns negative, never pass; a wide CI returns inconclusive.
    - Identical systems give p 1.0 and a CI containing 0; a constant +0.1 shift gives CI [0.1, 0.1]; outputs are identical across runs for a fixed seed.
    - Holm matches a textbook example; Wilson matches published values for 0/10, 5/10, 10/10; weighted kappa matches a published worked example; power formula matches a hand calculation.
```

## EVAL-1.1

SHA-256: `3fd641dfb81a5a5a6b5a24e36d4f5c62eb35dc7aba4d43c86d1fc5e0089aeb29`

```yaml
    - '`eval split --seed` stratifies by stratum and corpus; `eval pool` emits the top-10 sections of every system at every grid point, deduplicated and sorted.'
    - Ranking metrics run with abstention off on answerable queries; abstention metrics run on off-topic and answerable queries separately.
    - Report records corpus digests, qrels digest, config hash, git commit, binary hash, seed, per-query metrics, judged@10 per system and paired comparisons with decisions.
    - 'Discrimination check on a synthetic fixture: an oracle run beats a shuffled run with p < 0.01; a system compared with itself gives p = 1.'
```

## EVAL-1.2

SHA-256: `4bbfa343da089c542e46ecf05984bd59286282615e2fd7d395fce16f5ab50902`

```yaml
    - 'Planted defects each fail: judged section missing, span hash mismatch, dev/test token Jaccard >= 0.8, non-reproducible pool, judged@10 < 0.95, corpus checkout dirty or at the wrong commit, stratum or answerable-count floor not met.'
    - Each miss is attributed to exactly one of not_indexed, chunk_boundary, lexical_miss, fusion_displaced, truncated.
    - '`eval sweep --split dev` writes the grid table, the per-arm dev sd_diff and required n; `eval freeze` writes `frozen-config.toml` with parameters, power inputs, commit, Cargo.lock hash and binary hash.'
    - A held-out run with a dirty tree, different commit or different binary hash is refused.
    - '`check-benchmark --stage pools|qrels|run` selects which gates apply; each stage is covered by a planted-defect test.'
```

## EVAL-1.3

SHA-256: `9b367418be9b230bf149b1d34c3b6ee6bf4bbb28e2d85f78be43a22a3a0226f0`

```yaml
    - Measurement and tuning entry points fail closed for missing required gate settings, bad corpus pins, split integrity failures, unreproducible pools, missing or mismatched judged span hashes, and other failed preflight checks; errors do not publish usable results.
    - Held-out measurement and scoring require an unchanged frozen configuration, sufficient power and test count, clean matching code identity and matching benchmark inputs before accessing held-out results.
    - Judged-at-10 is checked before a report is accepted; missing or mismatched evidence is covered by focused regression tests.
    - Tests exercise both the public library paths and CLI where applicable, using isolated fixtures, and avoid any real benchmark test split.
    - Validate preparation-bound query-review decisions, final-text digests/history and review-before-pooling ordering plus human annotation/blinding/agreement records under EVAL-1.6's trusted-operator contract; missing, mismatched or model-only human evidence refuses.
    - Implement ADR9 version-1 query-review/annotation companions and bounded pools/qrels preparation manifests in existing owners. `pool` and pools-stage regeneration check trusted query approval before ranking; qrels-stage preparation recomputes all full-data gates, exact parsed partition unions, provenance/counts and raw-byte identities. Out-of-band review/preparation hash pins bind operator-audited approval, not self-authenticating JSON flags.
    - Positive synthetic CLI/library coverage includes selected dev/test capture, preparation-bound counts and full benchmark commitments, freeze/check, run, sweep, misses and saved scoring; dev execution never reads combined or held-out payloads.
    - Dev run/sweep/freeze/check/misses/score open no combined queries/qrels, full pool, full query-review/annotation or held-out payloads, including stat/hash and regeneration. Both partition commitments and approved counts survive separate CLI processes; preparation identity differs from selected digests. Test requires confirmation and reserves/fsyncs the sole started attempt before test payload access. Frozen/registration/metadata format changes refuse old versions without a reset path; no index schema change.
    - Completion-record or output-publication failures leave no accepted usable report and retain a consumed held-out attempt; test deterministic failure injection.
    - Reports include exploratory per-corpus confidence intervals without adding to the confirmatory family. Precompute borrowed section lookups and query tokens once in integrity gates, preserving exact gate semantics and failure order.
```

## OD-3.0

SHA-256: `484829166e8d7e6123061cdbd7de96e34b12312d619defb288248fdf1264c521`

```yaml
    - One shared store invariant checks raw section and chunk collections independently before publication serialization and after load verification; rejection is StoreError::Corrupt.
    - Synthetic preamble/punctuation-heading collisions and independently injected duplicate sections/chunks reject; first failure creates no store, and an existing CURRENT, HISTORY and generation bytes remain unchanged.
    - A self-consistent persisted artifact with duplicate IDs is rejected on load even when metadata hashes match; ordinary unique generations still roundtrip.
    - Existing section/chunk IDs and schema remain unchanged; affected Markdown fails explicitly pending OD-3.1/OD-3.2.
```

## OD-3.1

SHA-256: `7d01421ae495ad6e8d02def1ab1e0835a062bd6f7b58c07460ce33fe28a88a6a`

```yaml
    - Decide how preambles and empty heading slugs remain distinct and section IDs remain budget-independent, including nested and literal-suffix collisions.
    - Decide how volatile mtime leaves canonical artifact content without weakening current-byte freshness; enumerate schema bump and explicit invalidation of incompatible generations.
    - Resolve copied-fragment versus formatting-separator wording without accepting any diagnostics capability; unchanged graph/ranking semantics and hypothesis family are explicit.
```

## OD-3.2

SHA-256: `9c63cde6aa1fa0b9d3202cfb233c1a51f3f570ee522284de28e370614ec0ddc8`

```yaml
    - Implement only the recorded identity/schema decision; old schemas reject explicitly.
    - 'Follow service ADR schema 2: encode only empty allocated ID components as @empty; preserve Markdown anchors and read precedence. Test exact IDs for delimiter-bearing paths without extending aliases.'
    - Canonicalize skip paths/reasons in the store owner, include sorted skips in generation identity, omit persisted mtime and use captured-byte size. Verify root-independent diagnostics and skip-dependent IDs.
    - Explicitly set different mtimes across two synthetic roots; compare full generation and published artifact bytes, not only generation IDs.
    - Compare index.json, meta.json and CURRENT from independent stores; compare HISTORY only under identical initial state/sequence. Same-size edits with restored mtime must remain stale.
    - Reject incompatible build/publish/load versions, retired schema-2 mtime fields and mismatched candidate IDs. Reuse only exact index/meta bytes for a same-ID target; conflicts preserve target, CURRENT and HISTORY.
    - Exercise preamble, punctuation-only, nested empty-slug, duplicate heading and literal-suffix cases; IDs are unique and stable across chunk budgets.
```

## OD-3.3

SHA-256: `2962a75db3e0488715684f3e4817eb0c388f5561d7493a921f19c932ba72ff27`

```yaml
    - Measure actual serialized JSON-RPC responses, including rmcp text/structured duplication, request IDs, wrappers and errors; enforce the existing 128-KiB cap.
    - Control-character-heavy read/search payloads and long malformed inputs either truncate explicitly or yield a bounded typed error; all five tool contracts stay compatible.
    - Client-facing internal errors contain stable error kinds and generic messages, not sentinel filesystem paths or raw OS errors.
```

## OD-3.4

SHA-256: `6c952f4be76bfe0007d75908c960096b9953bc62f2d7b88bfbdf6e70e862bcc2`

```yaml
    - 'Implement ADR8 rooted local I/O: promote locked rustix1.1.4 fs; retain the App corpus descriptor; use full-relative openat2 BENEATH|NO_SYMLINKS for descendants and fixed64KiB RawDir enumeration. Unsupported/blocked/racing resolution refuses without fallback. Trusted parent bootstrap rejects final root symlinks including trailing-slash spelling.'
    - Root-bound CURRENT/meta/artifact reads, build, freshness and verification share corpus identity. src/store.rs changes are limited to this reader/caller binding and existing byte caps, not publication redesign. Root replacement cannot mix a generation and documents from different roots.
    - Entire batches validate exact constructible whole-chunk/section fields and manifest/document membership before target reads. Different/old generation yields generation_unavailable, same-generation nonmember invalid_citation; no HISTORY lookup or unindexed-path oracle. Preserve whitelisted hidden paths and originally-admitted membership without a current-policy rewalk.
    - Shared rooted traversal preserves stable readable-tree ignore parity; unsafe/unreadable controls refuse. Ordinary automatic walker is only a test oracle. Existing eval integration remains guarded; no real corpus/labels opened.
    - Barrier/injected tests cover root/ancestor/leaf replacement, growth, symlink/FIFO controls, unknown entry types, deleted-directory enumeration, unsupported resolution, each citation field, L0/L1 and truncated reads, whole sections and identical IDs across roots. Preserve valid/moved/ambiguous/stale/missing; work/I/O refusals never become citation states.
```

## OD-3.5

SHA-256: `4263a38ddbb40e1f59a63653c0c1dc63a02d7bf4b35654b1fc5a3b2416fcf10e`

```yaml
    - Enforce ADR8 raw128KiB stdio line ceiling on the actual native input path before allocation/decode, including LF/CRLF. Exact-cap/cap+1 malformed and unterminated frames prove the guard, not an unused codec setting; keep complete outbound bounds and five-tool parity.
    - Before conversion/load/spawn enforce aggregate decoded strings64KiB, query/scope/path4KiB, IDs16KiB, citations100, headings64/16KiB per citation and canonical64-byte hashes/IDs. Over100 citations refuse rather than truncate; tests prove zero load/spawn on refusal.
    - Verification shares aggregate source-read128MiB, candidate-hash128MiB and1000000 SHA-call budgets across the entire batch. Precharge original-span and moved-window checks including repeated citations; checked offsets and midbatch/tiny-window exhaustion return cap_exceeded, never partial or incorrect states.
    - One process-local actual worker and zero application waiting queue; acquire fail-fast before CURRENT I/O/load/rebuild/freshness/verify/spawn and transfer ownership into the closure. All five handlers share it; no detached reload or nested acquisition. Busy is typed; cancelled waiters cannot release admission or mutation locks while work survives.
    - Barrier/counter-based synthetic tests prove cancellation, shutdown/work ownership, cold reload contention and caps without sleeps or latency/RSS claims. Preserve immutable shared-index reuse and explicit refusal on failed reload.
```

## OD-3.6

SHA-256: `b662568e02925cade9ffc9de4c839aca2f47274128a4a3842bb7141325d60e45`

```yaml
    - Stream freshness manifest/hash comparisons without retaining all document texts; preserve added/removed/modified semantics including same-size edits.
    - Apply ADR8 shared build/freshness ceilings before work; source2MiB, source/control aggregate128MiB, prefilter probes100000, depth64, accounted paths16MiB, ignore64KiB/line4KiB/lines4096/matching1000000. Preserve stricter eval reservations and rooted policy parity; cap exhaustion fails rather than publishes partial or fabricates freshness.
    - Reuse OD-3.8 CURRENT128/meta4096/artifact512MiB boundaries with configured lower caps. Allocation-light preflight counts all encountered files/documents100000 and sections/chunks500000 before typed collections, including duplicate JSON keys; explicit local admission, no schema migration or silent drop.
    - Checked aggregate actual heading/span/pseudo-document tokenizer input512MiB refuses overlap amplification before BM25 work. Move the existing document lookup before construction; preserve exact accepted text/token/ranking order with synthetic parity.
    - Instrumented readers/probes and lowered synthetic preflight budgets prove exact/cap+1 refusal before the next read/allocation/hash operation, ignored-entry floods and equivalent current-byte freshness. No throughput/RSS guarantee follows; artifact bytes are not resident-memory accounting.
```

## EVAL-1.4

SHA-256: `10419b329610c972d0454317fbad469e0e7bdcf34cd71a9fea18081fa1661ecf`

```yaml
    - 'Freshness scope correction: reuse the shared corpus owner and store freshness comparison, but acceptance requires pre-filter traversal accounting and bounded ignore-file ingestion as well as Markdown reads. Yielded-entry counters alone are insufficient. Preserve ignore/freshness semantics; do not claim the rooted-reader/HTTP OD-3.4/3.6 contracts.'
    - 'Selected seam: corpus-owned bounded std directory enumeration with public ignore GitignoreBuilder/add_line/matched; ordinary ignore walker remains the regular-control compatibility oracle. Separate custom/git ancestor stacks, custom family first; share Markdown/output handling. No new dependency or eval-local ignore policy.'
    - 'Opt-in ceilings: depth64, ignore file64KiB, line4KiB, aggregate physical lines4096, pre-filter enumeration100000, conservatively accounted path-content bytes16MiB and matching-work1000000 units; ignore reads share capture byte reservations. Typed refusal on exhausted bounds or unsafe/unreadable controls; these are not allocator-overhead or CPU-time guarantees.'
    - 'ADR conflict disposition: ordinary ignore walker may follow symlinked controls contrary to ADR2. The new bounded path conforms by refusing symlink/nonregular controls; parity covers eligible regular controls only. Ordinary traversal/ancestor containment remains unresolved under OD-3.4/3.6, not accepted by this task. See audit_session-2026-11-10-eval14-bounded-traversal.md.'
    - 'Source-trace scope correction: reuse a store-owned captured-byte decoder for bounded eval index capture; no duplicated schema/checksum validator, public store-loader change or HTTP budget acceptance. In-process non-deserializable mechanical prevalidation is not query-review/human annotation provenance; persistent handoff remains gated by EVAL-1.5/1.3.'
    - Separate full preparation from split-specific execution; physically separate dev/test query and label inputs before dev loading, rather than filtering combined records.
    - Capture bounded input bytes/digests and parsed indexes once; validation, tuning and scoring use the captured values or fail explicitly rather than reopening mutable replacements.
    - Dev-only synthetic processing succeeds when held-out payload paths are inaccessible; required prevalidated cross-split integrity evidence remains identity-bound and fail-closed.
    - Keep public measurement refusals until EVAL-1.3 integration; do not read real held-out data or change scientific gates.
```

## EVAL-1.5

SHA-256: `416994b20cb279d9108c5755813277817b70b18c0f52b0a5605fd854bae87e5b`

```yaml
    - Reuse existing audit preregistration and create_new/fsync attempt mechanisms; choose one authoritative attempt location independent of disposable output paths, without adding a registry daemon.
    - 'Implement the recorded ADR revision 6 location resolution: Git-common-dir write-once registration/attempt custody and external exact-byte frozen export; rebind EVAL-3.x commands, audit paths and outputs before use.'
    - Bind frozen/config/code/benchmark identity and saved-run bytes; failure consumes the attempt and started/failed/completed outcomes remain auditable.
    - Changing output directory cannot create another attempt; altered ranking/freeze bytes reject, and scoring parses exactly the bytes whose digests were verified.
    - All tests are synthetic; public guards stay until EVAL-1.3 demonstrates the complete success and failure paths.
```

## HTTP-1

SHA-256: `f598494d7abc70548cbc688f0f3c261e1b9682159a495edee612c9a93646f68c`

```yaml
    - Use existing per-folder App/Store/shared immutable index slots; discover only immediate real subdirectories and never auto-index.
    - Preserve URL-segment names without aliases; reject encoded separators, traversal, symlinks, collisions and corpus-cap excess explicitly.
    - Two synthetic corpora with identical local IDs keep all five tools, citations, statistics, generations and readiness separate; missing index never falls back.
```

## OD-3.7

SHA-256: `c687263f03e926e5de9f0b2ea8edd8846cfb11df1d3b8b42b7f40bca68f098a8`

```yaml
    - README describes the Rust service and stdio setup; donor snapshot provenance stays in its own section with read-only boundaries.
    - Document disabled evaluation, unimplemented HTTP and known service-safety blockers accurately, without measured quality/performance claims.
    - Check local links, whitespace and command shapes against existing owners; no Rust build for documentation alone.
```

## OD-3.8

SHA-256: `64e5907a85892b14318ccc65560095d05b696c85709457a531e078e1ce0e41b2`

```yaml
    - CURRENT raw bytes <=128, metadata <=4096 and global artifact ceiling <=512 MiB are enforced by bounded reads before whole-file allocation/hash/parse; configured artifact limits are checked after bounded header decoding before generation acceptance. IDs are exactly 64 lowercase hexadecimal characters after bounded UTF-8 trim before joining paths.
    - Final-component no-follow/nonblocking opens and descriptor regular-file checks refuse symlinks/FIFOs/nonregular files; actual reads cap at limit+1 and reject growth. Do not claim ancestor containment.
    - Publication serialization obeys min(configured cap,512 MiB) before writes; normal and captured decoding enforce the same global and configured limits without schema/canonical byte changes.
    - Existing target cap/unsafe-file/I/O refusal preserves target/CURRENT/HISTORY; missing or ordinary verified-corrupt target recovery stays explicitly supported.
    - Synthetic tests cover exact-cap/cap+1 read and serialization, growth after metadata, invalid/traversal/whitespace pointers, unsafe files, captured decoding and publication preservation. Existing roundtrip, schema, conflict, CLI/MCP and eval tests remain green.
```

## OD-3.9

SHA-256: `14ef58bb7b328cb1eb87364b9451fd7e21d600114ea70347cacb989a2dd5b399`

```yaml
    - Implement ADR8 Result<PublishOutcome,StoreError>; only pre-CURRENT-rename failures return Err. Postrename sync failure is committed/durability_uncertain/maintenance_not_attempted; HISTORY/prune failures after sync are committed/synced/maintenance_failed. CLI index reports explicit additive fields/warnings with exit0 for committed outcomes; output loss never proves noncommit.
    - Deterministic injected failures before rename and at every sync/HISTORY/prune phase assert pointer/data protection, accurate failure-phase outcomes and no rollback. First publication syncs newly created store namespace entries in containing parents; reused candidates satisfy file/directory sync preconditions too. Precommit may leave orphan/temp files; successful maintenance keeps current plus two old generations.
    - HISTORY195-byte canonical three-ID cap and precharged cleanup100000-probe cap apply before parsing/deletion. Validate owned fixed-shape inventories; no recursive unknown-tree deletion, no current-generation deletion even with malformed HISTORY; sync HISTORY rename and generation pruning. Refusals preserve valid active data and retain the lock through maintenance.
    - Investigate the recorded parallel Locked failure with deterministic process/lock ownership evidence; do not label serial-only passing or blind retries a fix. Keep any unproven cause explicit.
    - Same/different-store barrier/process tests cover lock ownership, release/owner death and full publication lifetime without sleeps. Do not infer the earlier failure cause from a plausible descriptor-inheritance hypothesis.
```

## SAFETY-0

SHA-256: `a48f83b362d306c79039397b3ba7d001dce57d3a2628fcf8004a76668a201952`

```yaml
    - Select the minimal supported Linux rooted primitive/dependency and fail-closed unsupported-kernel behavior; retain current-byte freshness and corpus-policy parity.
    - Accept exact constructible generation/citation membership and unknown/old-generation typed refusal semantics before implementation; no arbitrary beneath-root verification oracle.
    - Set local request, verification hash-work, corpus traversal/freshness and actual-work admission ceilings with typed exhaustion and cancellation ownership; separate all network/session/global multi-corpus limits for HTTP-0.
    - Define CURRENT commit point and distinguish precommit failure, committed-but-durability-uncertain and postcommit maintenance failure before OD-3.9; do not promise impossible atomic rollback across filesystem errors.
    - Keep HTTP-0 owner-blocked; no listener, anonymous access, model annotations or benchmark-quality claim follows from local safety decisions.
```

## EVAL-1.6

SHA-256: `72776c22ae42ebb896d3dc7f8f6dbd266d2c949939d4a73df4b12820e0ee4f70`

```yaml
    - Specify existing query-review/annotation artifact shapes and trusted approval boundary, exact-byte identities, review-before-pooling chronology, human-annotation/blinding/agreement records and derived preparation counts.
    - Bind complete benchmark and both partition commitments across CLI processes while dev execution reads no held-out or combined payload; local custody is not independent reviewer or human identity proof.
    - Record a deterministic default-promotion policy for both-pass H2/H5 before held-out access and a separate scoped implementation task if promotion changes application defaults; never choose the biggest observed held-out gain.
    - Define clean matching committed code/binary, canonical external snapshot/audit handoff and completed-custody-before-accepted-report failure states. No real labels/results are read or evaluation guard lifted.
```

## FIX-1

SHA-256: `ebecbfa5d88604281b610cb76d57397d0b3e2030c5bab786472ff8fd9777f047`

```yaml
    - cite moved-span lines are computed on raw bytes; a test with invalid UTF-8 before the span gives correct lines.
    - validate_config refuses resamples outside 1..=10000 before payload reads; a test refuses 0 and 10001.
    - prepare applies check_layout to combined queries/qrels and trusted review/annotation/notes paths; a test places one inside a corpus and refuses.
```

## SIMPL-1

SHA-256: `26fa695d6cd4e683d316d1541f1c67cdf57d8c3f58cedf57961159273bf910bb`

```yaml
    - freeze takes no caller-supplied sweep and runs the dev sweep once; freeze/check-frozen tests pass unchanged.
    - check-benchmark keeps its --run/stage-run refusals but has no unreachable saved-run scoring branch; refusal tests pass unchanged.
    - gates agreement CI uses the single stats quantile; worked-example kappa/CI tests unchanged.
```

## PERF-1

SHA-256: `9664c63090976b67a9c1d218fad046d5b5310a20caa62e2b12be8c8e178945a3`

```yaml
    - The index mutex is held only for the CURRENT comparison/reload and the Arc clone, not across tool work or serialization.
    - Permits default to 1 for stdio, are bounded and configurable, and stay owned by the blocking work until it stops; busy beyond permits is a typed refusal.
    - Tests show two admitted requests overlap, a reload swaps generations for later requests only, and a failed reload never serves the old index as current.
```
