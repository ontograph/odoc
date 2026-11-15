# GraphRAG references execution

## Decision

The user instructed: `proceed tasks from docs/GRAPHRAG_REFERENCES_PLAN.md`. This authorizes its recommended structural graph scope and always-on chunk/section neighbour citations. ADR17 records that reading. Development targets unreleased 0.1.1 independently of REL-6; no publication is authorized. The original and corrected 0.1.0 artifacts and consumed evaluation custody remain untouched. GR-5 needs a separate decision on its diagnosis rule and remains deferred.

The proposal now owns a four-task strict tracker: GR-REG registration, GR-0 existing behavior documentation, GR-2 implementation and GR-DOC documentation/closeout. Runtime and documentation ownership are separated for manager validation. GR-2 additionally owns only the package's version fields in Cargo.toml/Cargo.lock so the changed response cannot advertise 0.1.0. The registered Clippy command uses the host compiler-lane launcher from the adjacent Ontocode checkout; it runs against this standalone package and never builds the donor snapshot.

## Evidence

Initial strict validation refused the missing manager block. A subsequent structural check identified role write-scope and Clippy-lane fields; these were corrected. Strict validation then passed with four tasks. No build, measurement or publication has run at registration.

## Implementation and review

GR-2 adds optional serialized neighbour citations in `src/cli.rs::Neighbour` and assembles them in `App::related`. Maps hold only the at-most-100 returned endpoint IDs; chunks and sections are each scanned once. Output follows the original edge order. Existing citation constructors, freshness checks and response bounds are reused. Unresolved or wrong-kind chunk/section endpoints return the existing typed corrupt-generation error; mentions omit citations. The MCP description and package version (0.1.1) match the contract. No schema change or reindex is required.

Synthetic regressions are `related_citations_resolve_by_kind_and_corrupt_edges_fail_closed` in `src/cli.rs`, `related_citations_verify_chunks_and_parent_but_omit_mentions` in `tests/mcp.rs`, and `cited_related_overflow_truncates_identically_over_stdio_and_http` in `tests/http.rs`. The overflow fixture has 48 linked target documents with long punctuation headings (`!`, which needs no JSON escaping). It exceeds the limit before trimming, then checks a nonempty truncated response, actual spawned stdio framing including newline, real HTTP parity and verification of retained citations. Wording corrected on 2026-11-14; separate existing transport tests cover escaped values.

Independent read-only Luna explorer `citation_review` found no blocking correctness gaps in the implementation, MCP description and new tests. Receipt: `agent-results/01a1d679-9b62-7cb0-b4e9-4ad90cf7cfd6/1`. The reviewer ran no tests and changed no files. Its initial OntoIndex request returned `workspace_busy`; committed-mode results flagged dirty files, so source inspection was authoritative. The coordinator's impact checks on `related` and `Neighbour` were LOW. `verify_diff` passed with the six expected runtime/package paths, subject to its 25-symbol bound and the pre-existing dirty tree; it does not establish clean whole-tree scope.

## Validation

All commands ran in the standalone OntoDoc root in this session. Compiler commands ran serially through `just --justfile /opt/d1/_wrk/ontocode/justfile compiler-lane-detached JOB env CARGO_BUILD_JOBS=4 COMMAND`. The launcher belongs to the adjacent host checkout; no donor snapshot was built. Each job has `output.log` and a terminal exit-0 `result.json` under `/opt/d2/.ontocode/artifacts/compiler-lane/JOB/`; those receipts and full-suite summary lines were re-read after handoff.

| Job | Exact COMMAND | Exit | Result |
| --- | --- | --- | --- |
| `ontodoc-gr2-cli-20261113` | `cargo test --lib cli::` | 0 | 12 passed |
| `ontodoc-gr2-mcp-20261113` | `cargo test --test mcp` | 0 | 13 passed |
| `ontodoc-gr2-http-20261113` | `cargo test --test http` | 0 | 7 passed |
| `ontodoc-gr2-all-20261113` | `cargo test --all-targets` | 0 | 293 passed, 3 ignored |
| `ontodoc-gr2-clippy-20261113` | `cargo clippy --all-targets -- -D warnings` | 0 | No warnings |

`cargo fmt --check` initially exited 1 for new CLI formatting. `rustfmt --edition 2024 src/cli.rs` corrected it; the repeated check exited 0 before the final all-target and Clippy jobs. `git diff --check` exited 0. Full-suite counts: 226 library, 8 CLI, 24 evaluation-gate, 14 evaluation-run, 7 HTTP, 13 MCP and 1 ranking-sanity tests. Ignored tests are the store subprocess helper and synthetic graph/tier timing checks. Evaluation tests use synthetic fixtures; they are not a new benchmark measurement. No additional compiler run is needed for GR-DOC's documentation-only changes.

## Ordinary corpus cap observation

Read-only counting of the published Rust Book documentation index found 1,370 chunks, maximum 132 incident neighbours, and 2 chunks above 100. Generation: `375917fc2e8cd756e785fdc118e6f5d12f4967c69963f89469de1b010d38ca95`. `index.json` SHA-256: `b88c3f33aec4547ecc252a8cb7d255e8f943010cb70c13f3832cca29b7aca7ce`. The explorer's result was independently reproduced by the coordinator with the following command, exit 0. Each incident edge is counted once even for a self-edge, matching `related`. No queries, labels or result payloads were read.

```bash
python3 - <<'PY'
import json, pathlib, hashlib, collections
p = pathlib.Path('tmp/corpora/rust-book/src/.ontodoc')
gid = (p / 'CURRENT').read_text().strip()
ip = p / 'generations' / gid / 'index.json'
d = json.loads(ip.read_text())
ids = {c['id'] for c in d['chunks']}
n = collections.Counter()
for e in d['edges']:
    for x in {e['from'], e['to']} & ids:
        n[x] += 1
print(dict(generation_id=gid,
           index_sha256=hashlib.sha256(ip.read_bytes()).hexdigest(),
           total_chunks=len(ids),
           max_neighbour_count=max(n.values(), default=0),
           chunks_over_100=sum(v > 100 for v in n.values())))
PY
```

The cap has observable consequences; no paging or request-filter expansion was implemented. Citations identify endpoints, not proof of the relation. Links target first chunks, and mentions remain unresolved. This change establishes no retrieval-quality improvement.

## Documentation and scope

GR-0 documents tier text versus supporting-chunk citations and graph order/limits. GR-DOC updates README, changelog and the existing architecture owner, with tracker and memory routing maintenance. The release verifier's working-tree inventory excludes README; preserved package members are checked from archive bytes. This session did not modify release archives, frozen binaries, evaluation payloads or shared custody. No fresh full release-archive verification was run for this task. Pre-existing dirty files remain; the unrelated implementation tracker and REL-6 publication gates were not changed by these tasks. GR-5 remains deferred pending its separate diagnosis-rule decision.

Documentation validation: `git diff --check` exited 0. A bounded Python check of the eight changed documentation/manager files passed local Markdown link paths, paired fenced blocks, trailing whitespace, MEMORY's 100-line/4096-byte bounds and matching package/lock version 0.1.1 (exit 0). `project_plan_validate` passed all four tasks with no errors or warnings. Reference checks do not validate external links or historical line numbers. The manager completed GR-REG, GR-0 and GR-2 through `complete-local`; GR-DOC uses the same evidence-bound transition. No rejected runtime receipts or disk recoveries were needed; initial tracker registration corrections and the formatting correction are recorded above.

Final GR-DOC completion initially hit a validator classification error: inserting the word `reproduced` for corpus counting triggered conditional-repair branch requirements. The tool rejected the write atomically. Rewording that evidence as independent corpus counting restored strict validation, and one corrective `complete-local` retry succeeded. All four tasks are DONE and the manager is closed with no active task. Active-memory routing was synchronized after closure; the removed GraphRAG routing entry is preserved here verbatim: `[GraphRAG references](../docs/GRAPHRAG_REFERENCES_PLAN.md) - unreleased 0.1.1 neighbour citations implemented; GR-DOC closeout active; release artifacts and custody preserved.`
