# Audit Session: Prior-Attempt Review (ontograph snapshot)

Date: 2026-11-08
Subject: `ontocode-rs/` source snapshot (see `../COPY_MANIFEST.json`, source head
`1e0598cb`). Read-only review; nothing was built or run. The snapshot is not a
standalone workspace: 90 imports of `ontocode_extension_api`, plus
`ontocode_tools`, `ontocode_protocol`, `ontocode_core`, `ontocode_features`,
`ontocode_secrets`, `ontocode_otel` and `ontocode_utils_*`.

Paths below are relative to `ontocode-rs/ext/ontograph/src/`.

## Scale

80 production modules, about 80.8k lines including tests. Docs retrieval is a
small part; most of the code is the Rust code graph (`indexer.rs`,
`rust_parser.rs`), about 25 `audit_*` modules, Knowledge Modules (`km_*`) and
LineageRAG (`lineage_*`). `tool.rs` tracks 62 "donor actions" for parity with
the TypeScript OntoIndex.

## Docs pipeline as built

- Walk: `*.md` only, fixed skip list, no `.gitignore` (`docs_index.rs:537-586`).
- Parse: hand-written line scanner, not CommonMark. ATX headings only; any
  fence line toggles a fence regardless of fence character or length;
  four-space-indented headings accepted; setext headings, front matter,
  `.markdown`/`.mdx` not handled (`docs_index.rs:345,446-469`).
- No chunking: a document gets a title, its heading list and the first 1 KiB of
  raw Markdown as `summary` (`docs_index.rs:497`). Body text is never indexed.
- Ranking: none. Text queries are a single ASCII-lowercase substring test
  (`docs.rs:772-797`); results are sorted by path/id keys (`docs.rs:204,221`).
- IDs: `sha256(prefix:value)` truncated to 16 hex characters; heading and fact
  ids include the line number, so an edit above changes every id below
  (`docs_index.rs:358,373`).
- Publication: per-file atomic writes, a `current-generation` pointer switched
  last, a hard-link lock with stale reclaim, pruning to 2 generations
  (`manifest.rs:14-23,275,368-385`; `indexer.rs:478-531,2417-2436`). Sound.
- Defects: a cap overflow silently skips the docs sidecar while the generation
  still publishes (`docs_index.rs:79-111`); `artifact_content_hash` is written
  but never checked on load (`docs_index.rs:137` vs `docs.rs:116-150`); one
  non-UTF-8 `.md` fails the whole index build; absolute `repository_root` in the
  manifest; `trace`/`drift` have no deadline checks and run O(facts x docs) scans.
- Dead branches: `migration*` facts and `document_id`/`requirement` link kinds
  are never produced, so the matching drift/trace code never runs
  (`docs.rs:310,368,440,511`).

## Knowledge Modules and LineageRAG

- KM = folder package: `km.toml` (semver, kind, provenance), `facts.jsonl`,
  `transitions.jsonl`, `mappings.jsonl`, examples, tests; versioned store with
  content hashes and a single active version (`km.rs`, `km_store.rs:263-311`).
- LineageRAG: wording-based "confidence" (constants 9500/1500/7000,
  `lineage_routing.rs:25-29`) gates an attempt; every unique word of 3+
  characters becomes a "demand" (`lineage_km_retrieval.rs:285`); retrieval is
  exact term overlap with no IDF (`km_resolve.rs:260-283`); greedy coverage
  selection capped at 8 passages/4096 tokens (`lineage_selection.rs`);
  admission re-reads spans and rejects stale ones (`lineage_admission.rs:103`);
  grounding checks quoted spans after the answer (`lineage_grounding.rs:222`).
- Evaluation: `km_eval` is a substring render test that ignores the query for
  retrieval (`km_eval.rs:70`); `km_llm_judge` builds cases from the module's
  own claims (circular, `km_llm_judge.rs:150-169`); `km_gold` is a snapshot, not
  gold data; `lineage_replay` has real metric shapes but is `#[cfg(test)]`-only
  with an 8-case corpus (`lib.rs:54-55`). No CIs, significance tests, splits or
  inter-annotator agreement anywhere.

## Verdict

Do not port the extension. Build a new standalone service and reuse only these
patterns, rewritten against std and public crates:

| Reuse | From | Change |
| --- | --- | --- |
| Beneath-root regular-file reads, no symlink follow | `artifact_io.rs` | port nearly as-is |
| Generation dir + pointer + lock + prune | `manifest.rs`, `indexer.rs:478-531` | relative paths, verify artifact hash on load, fail publication on cap overflow |
| Query budget/deadline, bounded serialization | `query.rs:113-299` | drop thread-local test hooks |
| Span + content-hash citation, stale-span rejection | `lineage_admission.rs`, `lineage_grounding.rs`, `docs.rs` drift | byte spans, not line-only |
| Metric record shape, corpus digest, `provider_backed` flag | `lineage_replay.rs` | real qrels, IR metrics, bootstrap CIs |

Replace: Markdown parsing (`pulldown-cmark` with offsets), chunking, ranking
(BM25 + experimental graph expansion), evaluation (qrels + IR metrics + paired
statistics) and host integration (`rmcp` stdio server).

Out of scope for v1: Rust code graph, `audit_*`, KM authoring, LLM
extract/judge/consolidate, LineageRAG routing heuristics, evidence planner,
greedy demand-coverage selection (reconsider only after a measured gain).
