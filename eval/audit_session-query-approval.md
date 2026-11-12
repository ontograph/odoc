# Query Approval (EVAL-2.0)

- Date: 2026-11-12
- Trusted operator: repository owner (the user), per owner decision 3 of
  2026-11-11 (`.memory-bank/audit_session-2026-11-11-eval20-preparation.md`).
- Approval: owner reply "1-4" on 2026-11-12 to the handoff that requested the
  spot-check of `tmp/eval-draft/spot-check-sample.md` (24 queries, seed
  20261111) and approval of the review digest below. Recorded by the EVAL-2.0
  manager; the manager did not approve on the operator's behalf.
- Approved bytes: `eval/query-review.jsonl`, SHA-256
  `94aaeff00a1be76fbb64d3cf86529df97aaa7327dea01e175c98c261c1e44e3e`,
  covering the 341 final queries of `eval/queries.jsonl`
  (`a64ec4bb14bdb912d5f07a1f915124234ef4552a94bc03fb4b55110d099d65df`).
- Provenance: all queries drafted by claude-opus-5-5 and reviewed by
  GPT-6-Astra (two rounds) by direct corpus reading; no retrieval ranking or
  score existed before this approval. Model-authored and model-reviewed; no
  human authored or reviewed the queries beyond the operator spot-check.
- Limits: these flags and this file do not authenticate reviewer independence
  or chronology by themselves.

## Pool Evidence

(appended after `ontodoc eval pool`)
- 2026-11-12: `ontodoc eval pool eval/config.json --out eval/pools/pool.jsonl
  --query-review eval/query-review.jsonl --query-review-sha256 94aaeff0…`
  (release build of commit 21bafa3 plus this approval file), exit 0:
  259 answerable queries, 4,004 pairs, SHA-256
  `fc5c8a81440a5ee53e61f7ec0b99147a96fec06cec940b65841896a570d6693e`.
  Full grid `{0,0.1,...,0.5}` for B2 and B3 (4,004 ≤ 6,000; no fallback).
- A second invocation to `/tmp/pool2.jsonl` was byte-identical (`cmp`).
- `ontodoc eval check-benchmark eval/config.json --stage pools --gates
  eval/gates.json --query-review eval/query-review.jsonl
  --query-review-sha256 94aaeff0…` exit 0, `passed: true`, no failures.
- No `preparation.json`: the CLI publishes only at stage qrels, which needs
  labels (EVAL-2.1 under ADR revision 13).
