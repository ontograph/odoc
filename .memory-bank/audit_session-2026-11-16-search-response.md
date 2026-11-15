# Search Response Changes

Date: 2026-11-16
Plan: [SEARCH_RESPONSE_PLAN.md](../docs/SEARCH_RESPONSE_PLAN.md), revision 1
Authority: owner reply "1 2" on 2026-11-16 to the agent task test report; service ADR revision 21.

No retrieval or answer-quality claim follows from anything below.

## Changes

- `src/mcp.rs` `structured`: a tool result keeps both the text block and `structuredContent` when that complete result fits `MAX_RESPONSE_BYTES`. Otherwise it sends the text block alone, and only then shrinks the value with the existing `Bounded` rules. The text is the same `Value::to_string` serialization rmcp writes, so the two copies cannot differ.
- `src/cli.rs` `SearchResponse` gains `unmatched_terms`, placed after `abstention_score`. `App::search` fills it from `query_terms` and the new `Bm25::contains` (`src/search/bm25.rs`). It ignores `scope` and never changes results.
- The search tool description and the server instructions name both changes.
- `SearchIndex::search`, `Ranked`, `abstention`, ranking, defaults, schema and the tokenizer are unchanged. Generation IDs are unchanged: the agent test corpus kept generation `9775ec695d93…` and `fresh: true` under the new binary.

## Impact

OntoIndex upstream impact before editing: `App.search` MEDIUM, with 5 direct callers (`run`, `EvalCommand.run` and three tests). `SearchResponse` LOW and `structured` LOW. The HIGH symbols `SearchIndex.search`, `Ranked` and `abstention` were not edited.

## Tests

- New `mcp::tests::structured_content_is_dropped_before_text_is_withheld`: a small read result keeps both equal copies; a 70,000-byte text is sent once and untruncated; a 200,000-byte text is sent once and truncated with `truncated: true`. Each result fits the bound.
- New `cli::tests::unmatched_terms_list_query_terms_absent_from_every_chunk`: query order, lowercasing and part splitting, heading terms counted as matched, `scope` independence, and a query with no terms.
- `search_results_carry_the_stored_tier_truncation_flag`: the byte-exact field order now includes `"unmatched_terms":[]`.
- `tests/mcp.rs` and `tests/http.rs`: helpers read the text block and assert parity whenever `structuredContent` is present. `rank_order_text_fit_keeps_later_small_hits` now uses three 45 KB texts: two are returned in full without `structuredContent`, the third is withheld, and later small hits keep L2. Before, the second large text was withheld because the two copies shared one frame.

## Validation

| Command | Result |
| --- | --- |
| `CARGO_BUILD_JOBS=4 cargo test --lib mcp::` | exit 0, 24 passed (before the new tests) |
| `CARGO_BUILD_JOBS=4 cargo test --test mcp --test http` | first run: 5 mcp and 1 http failures that read `structuredContent` unconditionally; helpers updated, then exit 0 with 18 and 7 passed |
| `CARGO_BUILD_JOBS=4 cargo test --lib -- mcp:: unmatched_terms search_results_carry bm25` | exit 0, 31 passed |
| `CARGO_BUILD_JOBS=4 cargo test --all-targets` | exit 0; lib 242 passed and 4 ignored; every integration target passed |
| `cargo fmt --check` | exit 0 |
| Clippy lane `ontodoc-sr1-clippy-20261116`, `cargo clippy --all-targets -- -D warnings` | `lane-result` exit_code 0, state terminal; submitted after the last Rust edit |
| `project_plan_validate docs/SEARCH_RESPONSE_PLAN.md` | pass, 3 tasks, zero gate counters |
| `rg -n '[[:blank:]]+$'` over the changed Markdown files | exit 1, no output |

## Measurement

`/tmp/escprobe.py` ran over stdio against `/opt/d2/ontodoc-agent-task-20261116/corpus`, first with the installed `~/.local/bin/ontodoc` and then with `target/release/ontodoc` built from this tree.

| Query | top_k | Old frame | Old withheld | New frame | New withheld | New `structuredContent` |
| --- | --- | --- | --- | --- | --- | --- |
| change personality soul profile snapshot store | 10 | 39,800 | 0 | 39,844 | 0 | present |
| sandbox | 50 | 130,418 | 22 | 89,266 | 0 | absent |
| permission approval policy | 50 | 130,397 | 24 | 99,859 | 0 | absent |

The 10-result frame grows by 44 bytes, which is the empty `unmatched_terms` field in both copies.

The arm B search arguments from the agent test logs were rerun through the new CLI. `unmatched_terms` was non-empty only for N1 (`kubernetes`, `horizontalpodautoscaler`, `horizontal`, `pod`, `autoscaler`). It was empty for the G1, G2 and N2 searches and for every answerable question. Q2's first search had failed in the test with a clap argument error, and it fails the same way here.

## Limits

- For the ontocode client, which sends only `structuredContent` to the model, a small result costs the model the same tokens as before. The gain is the room for text in large results. When `structuredContent` is dropped, that client sends the content array, which is 4 to 5% larger than the bare JSON in the measured frames.
- Clients other than ontocode, rmcp's test client and the raw wire tests were not checked for the text fallback.
- `unmatched_terms` would have flagged one of the four gap and negative questions. It cannot help when every query word occurs somewhere in the corpus.
- The installed `~/.local/bin/ontodoc` was not replaced. No agent rerun was made.
