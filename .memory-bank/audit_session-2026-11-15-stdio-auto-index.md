# Stdio Auto-Index And Probe Follow-Up

Date: 2026-11-15
Plan: [STDIO_AUTO_INDEX_PLAN.md](../docs/STDIO_AUTO_INDEX_PLAN.md), revision 2
Authority: service ADR revision 20 (applied by AI-ADR in this session) and the owner's "proceed open tasks" on 2026-11-15.

No retrieval-quality claim follows from anything below.

## AI-ADR

The five edits in [ADR_STDIO_AUTO_INDEX.md](../docs/ADR_STDIO_AUTO_INDEX.md) were applied by a script that required exactly one match at each site. The ADR header names revision 20. The new section sits immediately before "Folder-Scoped HTTP MCP Architecture". `MEMORY.md` names revision 20 and has 46 lines.

- `rg -n '[[:blank:]]+$'` on the four files: exit 1, no output.
- The revision 20 marker search found ADR lines 4, 617 and 822 and `MEMORY.md` line 11: exit 0.

## PQ-1

The search tool description and the server instructions in `src/mcp.rs` now say that matching is exact lowercase words without stemming or stop-word removal, that `scope` is a path-prefix filter, and that `abstention_score` is always returned while abstention needs a caller `threshold` with no validated cutoff.

- OntoIndex upstream impact: `get_info` and the search handler LOW, 0 dependents.
- `cargo test --lib mcp::`: 18 passed, exit 0. `cargo test --test mcp`: 17 passed, exit 0. `cargo test --test http`: 7 passed, exit 0. `cargo fmt --check`: exit 0.
- Clippy lane `ontodoc-pq1-clippy-20261115`: exit 0.

## PQ-DOC

README now lists `threshold`, `alpha` and `beta` as search inputs and documents `abstention_score`, the tokenizer, the three link diagnostic kinds and three authoring suggestions. The deployment guide has a "Link diagnostics" section. CHANGELOG has a 0.1.1 entry. Each statement was checked against `tokenize.rs`, `abstention` in `search/mod.rs`, `join` in `graph.rs` and the Debug-formatted `problem` in `store.rs`.

- Trailing-blank search: exit 1, no output. Term search: exit 0.

## PQ-SMOKE

[SMOKE_PROBE.md](../docs/SMOKE_PROBE.md) fixed 15 questions and their expected sources before one CLI run per question against `/opt/d1/_wrk/ontocode/docs`. The installed `ontodoc 0.1.1` was used, and the generation was the probe's own, `7bf5830a…`. Six of eight answerable questions found the expected document at rank 1; Q6 and A2 missed. The stop-word query scored 1.10, as in the probe, and the scores of out-of-corpus queries overlapped those of in-corpus ones. Every outcome is recorded there.

## PQ-ARM

[TOKENIZER_ARM_PROPOSAL.md](../docs/TOKENIZER_ARM_PROPOSAL.md) is Proposed. It holds a hypothesis row against B1, three variants, the cost from source, a separate abstention question and six owner decisions. No code or evaluation file changed.

## AI-1

`ontodoc serve --auto-index` is implemented in `src/cli.rs`, `src/main.rs` and `src/mcp.rs`, with tests in `src/mcp.rs`, `src/cli.rs` and `tests/mcp.rs`.

- **Flag:** `Command::Serve` is a struct variant with `auto_index`. `serve-http` has no such argument, so clap rejects it there.
- **Refresh step:** it lives inside the `spawn_blocking` closure of `OntoDocServer::blocking`, under the call's permits. With no generation it rebuilds. Otherwise it loads, runs one freshness check, and rebuilds when the index is stale and no failure happened in the last 30 seconds.
- **Load errors:** they return as before and are never rebuilt.
- **Hashing once per call:** when no rebuild happens, the check's result is stored as a one-shot hint on `App`, keyed by generation ID. The handler consumes it, so a fresh or failed-rebuild call hashes the corpus once. After a successful rebuild the handler hashes again; the amendment allows that.
- **Rebuild and logging:** `AutoIndex::rebuild` calls `App::index` and records a monotonic failure time in memory. It writes fixed JSON codes to stderr for a failure, uncertain durability or failed maintenance.
- **Instructions:** `get_info` adds one sentence when the flag is set.
- **HTTP:** `Registry::discover_with` never sets the flag.

OntoIndex upstream impact before the edit:

- `OntoDocServer::blocking` was MEDIUM: 5 direct callers, the five tool handlers, plus 1 test.
- `serve_stdio` and `Command` were LOW, with 0 dependents in the graph, which predates the working tree.
- No HIGH or CRITICAL result.

New tests:

- `auto_index_builds_missing_index_then_refreshes_after_edit`: first call builds, the ID equals the CLI `index` ID, a fresh call does not rebuild and leaves no hint, and an edit yields new text, `fresh: true` and a new ID.
- `auto_index_failure_serves_stale_and_waits_before_retrying`: a held store lock makes the rebuild fail. The call answers from the old generation with `fresh: false`, and the next call does not rebuild. With the failure time moved back past the interval, it rebuilds.
- `auto_index_without_generation_returns_the_build_error`: `locked`, then `no_index` inside the interval, with one rebuild.
- `auto_index_never_rebuilds_or_removes_an_unloadable_generation`: corrupt `meta.json` keeps its error with zero rebuilds, and the bytes stay unchanged.
- `auto_index_rebuild_keeps_the_permit_and_refuses_concurrent_calls`: while a rebuild is held open, no permit is free and a second call gets `busy`.
- `auto_index_is_stdio_only_and_named_in_instructions`: the registry server has no auto-index, the instruction sentence appears only with the flag, and without the flag a missing index stays `no_index` with no `.ontodoc/` created.
- `auto_index_flag_parses_for_serve_only` and `freshness_hint_is_used_once_for_its_generation_only` in `src/cli.rs`.
- `stdio_auto_index_builds_on_first_call_and_writes_only_frames` in `tests/mcp.rs`: the built binary over a real stdio pipe, with only frames on stdout and the generation ID equal to `index`.

The retry test does not sleep. It moves the recorded failure instant back by `RETRY_AFTER` plus 1 second through the test-only field.

Commands:

- `cargo test --lib mcp::`: 24 passed, exit 0.
- `cargo test --test mcp`: 18 passed, exit 0. The first run hung in the new stdio test, because the send closure did not own stdin and the server never saw end of input. The process was killed, exit 101, the test was fixed with `move`, and the rerun passed.
- `cargo test --all-targets`: exit 0. lib 240 passed and 4 ignored, cli 9, eval_gates 24, eval_run 14, http 7, mcp 18, ranking_sanity 1.
- `cargo fmt --check`: exit 0.
- Clippy lane `ontodoc-ai1-clippy-20261115` (`cargo clippy --all-targets -- -D warnings`): `lane-result` reports `exit_code` 0, `state` terminal, elapsed 1,105 s, most of it queued behind another session's lane owner. Submitted 18:35:47, after the last AI-1 source edit at 18:35:10; no file under `src/`, `tests/` or the Cargo manifests is newer than the submission. AI-1 closed DONE.

Existing tests passed unedited.

## AI-DOC

The README gains an "Automatic indexing for local stdio use" subsection. It covers the flag, the store it writes, the in-call rebuild, failure behaviour, refusal of corrupt generations, the per-call `verify` cost and the benchmark-root warning. CHANGELOG 0.1.1 has the entry. AGENTS.md names the stdio exception, and HTTP never indexes. The architecture router rows for `src/cli.rs` and `src/mcp.rs` name the flag. The plan index row is closed.

While checking the draft against `src/cli.rs`, one README sentence was corrected. The draft said `related` also hashes the corpus with the flag. In fact `related` already checks freshness, and only `verify`, which reads current bytes, adds a per-call corpus pass.

Commands:

- The trailing-blank `rg` over the seven AI-DOC files: exit 1, no output.
- `project_plan_validate docs/STDIO_AUTO_INDEX_PLAN.md`: status pass, 7 tasks, every quality-gate counter 0.

No Rust compilation was run for this documentation task.

## Limits

- Rebuild cost on a large folder is not measured.
- The stdio pipe test covers one first-call build. Stale rebuilds and failures are covered in-process.
- The OntoIndex graph was indexed at `ee511dc` and does not include the working-tree changes.
- Not run: the HTTP transport with the flag (it cannot be set there), any benchmark, and a release build. At plan closeout the installed binary was not replaced; see the addendum below.

## Addendum: Local Install (2026-11-15, after closeout)

The owner's later "proceed open tasks" covered the one reversible open item. That item was deploying the new build to the local ontocode MCP entry, which serves `/opt/d1/_wrk/ontocode/docs`. That folder is not a benchmark corpus; `eval/corpora.json` names `ontoindex` and `rust-book`.

- Lane `ontodoc-autoindex-release-20261115` (`cargo build --release --locked`): exit 0, terminal.
- The previous binary is kept as `~/.local/bin/ontodoc.bak-20261115-pre-autoindex`, SHA-256 prefix `c58fd460b2c4d3e1`. The installed binary equals `target/release/ontodoc`, SHA-256 prefix `518c30563149f964`.
- Installed binary: `serve --help` lists `--auto-index`; `serve-http --auto-index` is rejected as an unexpected argument.
- Scratch folder: the first stdio `search` built an index and answered `fresh: true` with generation `5ca57b14…`. A later `ontodoc index` on the same folder published the same generation ID. Only `.ontodoc/` was added beside the document.
- The ontocode MCP config (`/opt/d2/.ontocode/config.toml`, backup `config.toml.bak-20261115-pre-autoindex`) now passes `serve --auto-index`. The stale generation `7bf5830a…`, which lacked two newly added review files, was rebuilt on the first stdio `status` call into `4a286d7e…`. It holds 195 documents, `fresh: true`, with empty stderr. CLI `status` then reported the same generation, fresh.

Already-running ontocode sessions keep the old server process until they restart. Rollback: restore both backups.
