# Dev reproducibility proof, 2026-11-13

Before execution: same-byte export and complete preparation commitments recorded in `export-evidence.json`. Owner-approved preparation pin `1e4835cd994a799b136fcf7c095567e43ae9bbaafd800a6bdbc51d69b1cf5125`.

Candidate `/opt/d1/_wrk/ontodoc/tmp/release-010/candidate`, commit `cc116c44a4d7f854bc19c6a4b40d75098a437268`, clean; Git common directory `/opt/d1/_wrk/ontodoc/.git`. Binary `/opt/d1/_wrk/ontodoc/tmp/release-010/candidate/target/release/ontodoc`, SHA-256 `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. Benchmark `/opt/d1/_wrk/ontodoc/tmp/release-010/benchmark/eval`, outside candidate and corpus checkouts.

Dev commands load selected dev inputs only. Complete benchmark and partition commitments are preserved from preparation; no held-out ranking or label inspection. Original approval admits same-byte dev export. ADR9 relocation recapture is mandatory before registration.

Protocol: canonical EVAL-3.0 dev command twice into absent `results/dev-proof` and `results/dev-proof-repeat`, then `diff -r`. Check identity and judged@10 fields. No tuning or quality claim.

Export recovery: initial script exited 1 after copying eval, because plain Git status counted the required untracked stores. Corrected guard exactly matches corpus admission (all untracked files, excluding only corpus-root .ontodoc). Continued partial export without overwriting; both exported pin/status/index/meta and selected hashes verified.

## Outcome

Both canonical `eval run ... --split dev` commands exited 0 from the clean candidate; `diff -r results/dev-proof results/dev-proof-repeat` exited 0. Seven artifacts match byte-for-byte. `report.json` and `meta.json` agree; commit, clean flag, binary, preparation, config, selected payloads, splits, generation IDs and run hashes verified. Judged@10 is 1.0 for B0/B1/B2@0/B3@0. B0.tsv is empty, so its judged coverage is vacuous; no retrieval-quality conclusion follows.

A third invocation targeting the existing first output refused with exit 1; all original hashes stayed unchanged. Exact argv, error and artifact hashes are in `dev-proof-validation.json`. Candidate remained clean and binary unchanged. No Rust build/test, test-split execution, registration or custody write occurred.
