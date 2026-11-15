# Release capacity baseline, 2026-11-13

## Preserved preregistered protocol

- REL-3 protocol registered 2026-11-13 before execution: evidence/capacity below tmp/release-010; same retained REL-5 binary. Copy pinned tmp/corpora/ontoindex (d535fe888a88694c524636bed8a7c409594184cb) and tmp/corpora/rust-book/src (2771df165f092a4fbb9f46b5b2e7450f19010adb) to capacity/docs_share, excluding only .git/.ontodoc and rejecting symlinks. Third corpus has one 2,097,151-byte Markdown file (2MiB minus one; near file-byte cap only), header 12 bytes, 2047 1024-byte paragraphs and final 1011-byte paragraph. Run three fresh index samples per corpus using GNU time wall/RSS, separate stores per repeat. Serve first indexed copies over loopback HTTP; fixed synthetic queries documentation/rust/capacityneedle, lexical top_k=1. For each corpus/tool, warm once then 20 synchronized batches at concurrency 1 and 4; fresh HTTP connections, 60s timeout, no retries, success p50/p95 nearest-rank and separate busy/error counts/latencies. Resolve read/related/citation from setup search. Additional 10 synchronized c16 status batches on near-cap corpus record busy. Record complete response bounds, fresh results, corpus sizes/counts, host, script hash and unchanged binary hash; preserve raw measurements, no acceptance verdict without owner latency target. No evaluation labels or ranking claims. Run registered measure.py commands only after self-check validates quantiles and error classification.

## Results

REL-3 uses clean candidate `cc116c44a4d7f854bc19c6a4b40d75098a437268`, retained binary SHA-256 `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. Script SHA-256 `ea00cd296af9918b3f847c1bcb84586a4cbf3a2c86d0941c71f434ced71f28fe`. Protocol was registered in the canonical plan before execution. Generated evidence is under `tmp/release-010/evidence/capacity/`: `measure.py`, `result.json`, `observations.jsonl`, separate index logs and GNU time RSS records.

`python3 tmp/release-010/evidence/capacity/measure.py --self-check` and `python3 tmp/release-010/evidence/capacity/measure.py` both exited 0. Nine index subprocesses and HTTP shutdown exited 0; final candidate, binary and original tracked-status checks passed. No Rust source changes or compiler checks were needed. Earlier REL-5 results are historical evidence, not rerun here.

Host: AMD Ryzen 9 9950X (8 CPUs reported to Python), Linux 6.11.0-29-generic x86_64, glibc 2.39, Python 3.12.3, rustc 1.95.0. Pinned corpus copies preserve ignore files and exclude only `.git`/`.ontodoc`; originals were unchanged. OntoIndex pin `d535fe888a88694c524636bed8a7c409594184cb`; Rust Book pin `2771df165f092a4fbb9f46b5b2e7450f19010adb`, root `src/`. Inventory has 325 Markdown candidates/3,617,877 bytes for OntoIndex, of which 118 documents are admitted by ignore policy; Rust Book admits 112 documents/1,221,077 Markdown bytes. The synthetic corpus has one 2,097,151-byte file, one section, 2,048 chunks and 4,096 edges. It approaches the file-byte cap only.

Three fresh-store index repetitions per corpus; wall time includes process startup and Python timeout-wait polling overhead (visible quantization), so these are coarse command elapsed measurements, not precise engine timing. OS caches were uncontrolled. Peak RSS is GNU time's process maximum, not HTTP server memory.

| Corpus | Index wall seconds, 3 runs | Peak RSS KiB, 3 runs |
| --- | --- | --- |
| ontoindex | 0.114, 0.114, 0.114 | 15876, 16080, 15916 |
| rust-book | 0.114, 0.114, 0.114 | 17000, 17124, 17092 |
| near-cap | 0.064, 0.064, 0.064 | 21156, 21016, 21096 |

HTTP uses fresh connections, a fixed lexical top-1 query per corpus, one warm call per tool, then 20 synchronized batches at concurrency 1 and 4. Queries are `documentation`, `rust`, `capacityneedle`; read/related/verify use the setup hit and returned citation. Timing includes connection setup and parsing. Percentiles are nearest-rank over successful calls only; raw refusals and separate refusal percentiles/rates remain in result.json. No retries. Extra near-cap status load uses ten batches of 16.

| Corpus | Tool | Concurrency | Success | Busy | Success p50 ms | Success p95 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| ontoindex | status | 1 | 20 | 0 | 18.063 | 18.964 |
| ontoindex | status | 4 | 40 | 40 | 17.782 | 18.536 |
| ontoindex | search | 1 | 20 | 0 | 18.064 | 19.022 |
| ontoindex | search | 4 | 40 | 40 | 17.724 | 19.093 |
| ontoindex | read | 1 | 20 | 0 | 18.069 | 19.880 |
| ontoindex | read | 4 | 40 | 40 | 17.955 | 20.278 |
| ontoindex | related | 1 | 20 | 0 | 18.239 | 19.423 |
| ontoindex | related | 4 | 40 | 40 | 17.576 | 18.816 |
| ontoindex | verify | 1 | 20 | 0 | 0.238 | 0.524 |
| ontoindex | verify | 4 | 71 | 9 | 0.453 | 0.607 |
| rust-book | status | 1 | 20 | 0 | 1.499 | 1.694 |
| rust-book | status | 4 | 40 | 40 | 1.529 | 2.147 |
| rust-book | search | 1 | 20 | 0 | 1.412 | 1.813 |
| rust-book | search | 4 | 42 | 38 | 1.584 | 2.672 |
| rust-book | read | 1 | 20 | 0 | 1.333 | 1.817 |
| rust-book | read | 4 | 42 | 38 | 1.460 | 2.771 |
| rust-book | related | 1 | 20 | 0 | 1.302 | 2.183 |
| rust-book | related | 4 | 42 | 38 | 1.454 | 2.356 |
| rust-book | verify | 1 | 20 | 0 | 0.210 | 0.453 |
| rust-book | verify | 4 | 65 | 15 | 0.434 | 0.566 |
| near-cap | status | 1 | 20 | 0 | 1.403 | 2.504 |
| near-cap | status | 4 | 44 | 36 | 1.492 | 3.238 |
| near-cap | search | 1 | 20 | 0 | 1.670 | 2.700 |
| near-cap | search | 4 | 41 | 39 | 1.565 | 2.420 |
| near-cap | read | 1 | 20 | 0 | 1.484 | 2.279 |
| near-cap | read | 4 | 43 | 37 | 1.503 | 2.424 |
| near-cap | related | 1 | 20 | 0 | 1.341 | 1.754 |
| near-cap | related | 4 | 40 | 40 | 1.368 | 2.459 |
| near-cap | verify | 1 | 20 | 0 | 0.423 | 0.857 |
| near-cap | verify | 4 | 41 | 39 | 0.592 | 1.445 |
| near-cap | status | 16 | 37 | 123 | 3.034 | 4.626 |

All 1,660 measured calls classified: 1,008 successes, 652 JSON-RPC busy refusals, zero HTTP 503 and zero other errors. Successful replies met the complete response-byte bound, freshness checks and valid-citation checks as applicable. Busy observations exercise per-corpus overload; they do not prove exhaustion of the global-work, connection or HTTP-request caps. No latency acceptance verdict, sustained-load guarantee, confidence interval or retrieval-quality claim is assigned. Small successful sample counts, one fixed query/citation, warm caches and synchronized bursts limit generalization.

Recovery: the owner's proceed instruction authorized the corrected packet retry. The worker created the script; local review corrected warm-call argument passing and process-group timeout cleanup before measurement. Self-check passed, then the single full capacity invocation completed. No measurement retry, frozen registration, held-out attempt or custody change occurred. Next is EVAL-3.0's approved external snapshot and two dev-proof runs.

