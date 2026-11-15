# OntoIndex Ideas Capacity Measurement

Date: 2026-11-14
Authority: OI-3 in `docs/ONTOINDEX_IDEAS_PLAN.md`, authorized by the user's proceed instruction.

## Protocol Before Measurement

Run the registered B16, B60, E50 and EP indexing cases, then measure published cases and Over. Preserve refused cases as results. Measure `status`, `search` and `verify` over loopback HTTP MCP for 20 sequential batches, and one batch of four concurrent searches. Record fresh-process CLI status separately. No latency target or pass/fail capacity verdict is assigned. No benchmark query, label or result payload is used.

The generator records deterministic bytes and file lists. Accounting includes directory dot entries and terminal probes; paths include the canonical absolute root, directory entries charged three times, and missing `.gitignore` / `.ontodocignore` capture probes. The `.ontodoc` root entry adds a charge after indexing but is not descended. Byte accounting reports final charge and peak reservation, since each Markdown read reserves 2,097,153 bytes before refund. Source references: `RootedCorpus::open`, `directory_children_fd`, `capture_ignore_after_preflight`, `collect_rooted_file` in `src/corpus.rs`; default `IndexConfig` in `src/store.rs`.

The preserved capacity script is reused only by adaptation in `tmp/ontoindex-ideas/capacity/`. Its original bytes, all pinned benchmark stores, preserved release packages and consumed evaluation custody remain unchanged. The release binary is built from the current dirty tree, and commit, tree state and binary SHA-256 accompany results; these measurements are not the historical frozen binary's results.

## Execution

Managed release build job `ontodoc-oi3-build-20261114`, command `CARGO_BUILD_JOBS=4 cargo build --release`, completed with exit 0 (24.059 seconds in the terminal receipt). `sha256sum target/release/ontodoc` exited 0: `216633bf37cbac9500a26bc0e4daabbec5e1fb65ba73bc186361faf6462fe300`.

The preserved script SHA-256 is `ea00cd296af9918b3f847c1bcb84586a4cbf3a2c86d0941c71f434ced71f28fe`.

## Completed Observations

Generator and measurement commands from OI-3 both exited 0. The release build predates OI-1; the additional invariant load cost is not measured. Host: Linux-6.11.0-29-generic-x86_64-with-glibc2.39. Commit `ee511dc0f1b2300266fee23e7012463d9d3e2f11`; dirty state and tree hash are saved in `results/logs/`. Scripts, manifest, raw command logs, complete bounded RPC responses and outcome summaries are under `tmp/ontoindex-ideas/capacity/`; SHA256SUMS binds them.

### Computed Walk Charges

These are complete-walk demands from the file inventory, not observed work before refusal. Bytes include retained length-plus-one and the largest pending reservation. Path charge includes absent ignore-file probes. After indexing, the `.ontodoc` entry adds one entry and 243 path bytes for the three published roots. Root lengths are 72 bytes (B16/B60/E50) and 71 bytes (EP).

| Case | Files | File bytes | Retained bytes | Peak reservation bytes | Entries | Path bytes | Observation |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| B16 | 16 | 33538048 | 33538064 | 33539088 | 19 | 4601 | published, fresh status |
| B60 | 60 | 125767680 | 125767740 | 125768764 | 63 | 15557 | published, fresh status |
| E50 | 50000 | 51200000 | 51250000 | 53346128 | 50203 | 13245017 | published, fresh status |
| EP | 65059 | 4163776 | 4228835 | 6325923 | 65326 | 17038890 | index exit 1, cap_exceeded |
| Over | 66 | 138344448 | 138344514 | 138345538 | 70 | 17294 | status/search cap_exceeded; verify valid |

Limits: walk bytes 134,217,728; entries 100,000; paths 16,777,216; files 100,000; chunks 500,000; artifact 536,870,912 bytes; tokenizer input 536,870,912 bytes. EP's computed path ceiling is 64059 files; the fixture adds 1,000. Its path demand exceeds the cap while other demands remain below their limits. The public refusal is generic `cap_exceeded`; it does not name the budget. Over crosses the byte reservation budget on file 65. Its six added files were removed, and the server exited 0.

### Index And Fresh-Process CLI

| Case | Index exit | Index seconds | Index peak RSS KiB | Artifact bytes | Chunks | CLI status exit | CLI seconds | CLI peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| B16 | 0 | 0.824 | 143284 | 52413210 | 28480 | 0 | 0.566 | 119764 |
| B60 | 0 | 1.983 | 494876 | 196548454 | 106800 | 0 | 2.030 | 433980 |
| E50 | 0 | 1.468 | 395608 | 108100394 | 50000 | 0 | 1.325 | 261948 |
| EP | 1 | 0.316 | 27912 | N/A | N/A | N/A | N/A | N/A |
| Over | N/A | N/A | N/A | reused B60 | reused B60 | 1 | 2.025 | 433732 |

EP was index-only and refused before publication; serving and CLI generation metrics are N/A. Over was not reindexed.

### MCP Latencies

Milliseconds, nearest-rank quantiles. Each concurrency-1 row is 20 sequential requests; each concurrency-4 search row is one batch of four. Rows separate outcomes; zero-count outcomes have no latency. Every other outcome count (including HTTP 503) was zero. Raw JSON-RPC cap refusals use code -32603 with `data.error=cap_exceeded`; busy uses `data.error=busy`.

| Case | Tool | Concurrency | Outcome | Count | p50 ms | p95 ms | Max ms |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| B16 | status | 1 | success | 20 | 20.724 | 22.948 | 23.306 |
| B16 | search | 1 | success | 20 | 26.666 | 31.446 | 31.481 |
| B16 | search | 4 | busy | 2 | 3.143 | 3.158 | 3.158 |
| B16 | search | 4 | success | 2 | 30.385 | 36.049 | 36.049 |
| B16 | verify | 1 | success | 20 | 0.874 | 3.572 | 4.822 |
| B60 | status | 1 | success | 20 | 71.985 | 77.841 | 79.429 |
| B60 | search | 1 | success | 20 | 108.853 | 117.901 | 127.488 |
| B60 | search | 4 | busy | 2 | 1.107 | 1.157 | 1.157 |
| B60 | search | 4 | success | 2 | 97.636 | 101.430 | 101.430 |
| B60 | verify | 1 | success | 20 | 0.658 | 2.099 | 3.955 |
| E50 | status | 1 | success | 20 | 291.096 | 325.088 | 335.542 |
| E50 | search | 1 | success | 20 | 291.615 | 305.015 | 306.499 |
| E50 | search | 4 | busy | 2 | 0.531 | 0.572 | 0.572 |
| E50 | search | 4 | success | 2 | 286.719 | 286.979 | 286.979 |
| E50 | verify | 1 | success | 20 | 0.391 | 3.199 | 3.704 |
| Over | status | 1 | cap_exceeded | 20 | 73.266 | 82.116 | 84.294 |
| Over | search | 1 | cap_exceeded | 20 | 104.740 | 117.445 | 119.988 |
| Over | search | 4 | busy | 2 | 0.774 | 0.848 | 0.848 |
| Over | search | 4 | cap_exceeded | 2 | 90.910 | 91.063 | 91.063 |
| Over | verify | 1 | success | 20 | 0.285 | 0.445 | 0.589 |

Server high-water RSS is cumulative across corpora in one process: 122,572 KiB after B16, 505,240 after B60, 580,816 after E50 and at end. These are not isolated per-corpus memory costs. No cache dropping was performed; the fresh CLI process does not imply cold filesystem cache. Synthetic repetitive paragraphs, one lexical query and one citation per corpus do not represent production workloads. No latency target, capacity verdict, retrieval-quality claim or recommended corpus ceiling follows. Root length and filename shape affect the path ceiling.

### Validation And Preservation

The measurement recorded unchanged binary and original-script hashes at completion. Evidence preserves every index refusal and all 256 measured RPC calls. The generator/measurement self-check covers hand-computed charges, fixture identity, quantiles and outcome classification; command exits and final checksum verification are recorded below. No source, budget, freshness policy, pinned store, preserved release artifact or evaluation input was changed by OI-3.

- `python3 tmp/ontoindex-ideas/capacity/make_corpora.py`: exit 0.
- `python3 tmp/ontoindex-ideas/capacity/measure.py --binary target/release/ontodoc --out tmp/ontoindex-ideas/capacity/results`: exit 0.
- `python3 tmp/ontoindex-ideas/capacity/measure.py --self-check`: exit 0; includes generator arithmetic/inventory checks.
- Python AST and trailing-whitespace checks on both scripts: exit 0.
- `sha256sum -c tmp/ontoindex-ideas/capacity/SHA256SUMS`: exit 0, all 39 evidence files matched.
- `git diff --check`: exit 0; ignored Python files checked separately above.

