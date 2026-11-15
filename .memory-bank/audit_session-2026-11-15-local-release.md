# Local 0.1.0 Install (REL-6 Closeout)

Date: 2026-11-15

## Decision

The owner was asked seven questions with proposed answers. The reply was "make only local install / approved". That approves the proposed answers and replaces publication with a local install.

1. Release the corrected candidate as 0.1.0. Evaluation results describe the original measured binary; the bundled `EVALUATION_0.1.0.md` says so.
2. and 3. Do not publish. No GitHub repository, tag, release or upload; no package or history exposure.
4. Keep the recorded home build paths: 317 `/home/` byte-prefix occurrences, all Cargo registry and Rust toolchain paths. Use `--remap-path-prefix` from 0.1.1 onwards.
5. Unreleased 0.1.1 work stays out of 0.1.0.
6. One local commit, no push.
7. No tokenizer experiment now.

## Evidence

- `sha256sum -c SHA256SUMS` in `tmp/release-010/corrected/`: both archives OK. The binary archive SHA-256 is `daccd668…4b64725`; the manifest records `published: false` and binary SHA-256 `2fc26f15…69c2a3`.
- The archive was extracted to a unique temporary directory under `~/.local/opt/`. Its internal `SHA256SUMS` check passed. It was then moved to `~/.local/opt/ontodoc-0.1.0/`, a path that did not exist before. The installed binary SHA-256 is `2fc26f1527ce59842e135ef47fd80d950541bd6389fa360c43f987dd8684c2a3`.
- Symlink `~/.local/bin/ontodoc-0.1.0`: `--version` prints `ontodoc 0.1.0`.
- Scratch folder smoke: `index` exit 0, committed. `search "restore backup"` exit 0, fresh, top citation `guide.md`. Stdio `tools/list` returned the five tools `read`, `related`, `search`, `status` and `verify`. The scratch folder was removed.
- `~/.local/bin/ontodoc` stays the unreleased 0.1.1 build with `--auto-index` used by the ontocode MCP entry. The two binaries coexist under different names.
- Local commit `116967a` holds the previously uncommitted completed-plan work, with no tag or push. `.gitignore` now ignores manager `*.integrate.lock` files. No commit hooks are configured.

## Limits

- No public release exists, and nothing was uploaded or tagged.
- The installed 0.1.0 binary is the corrected candidate and was not itself measured by the held-out evaluation.
- No Rust build or test was rerun for this closeout; the package's own validation is in `audit_session-2026-11-13-corrected-candidate.md`.

## Probe Response (2026-11-15)

The owner asked for a reply to the ontocode side about a retest. The reply is `/opt/d1/_wrk/ontocode/docs/review/ONTODOC_TEAM_RESPONSE_AND_RETEST_PROTOCOL_2026_11_15.md`, an uncommitted file in that repo. It answers both probe reports.

- Confirmed against the saved 0.1.0 probe copy: query 6 and query 9 results, the copy's counts, and the same generation `c302af8d…` from 0.1.0 and 0.1.1.
- Corrected two report claims. The live counts differ from the copy's because the live index includes the review files. The four `MissingTarget` targets exist; they are absolute paths.
- Gave a blind retest protocol. A `.ontodocignore` line `review/ONTODOC_*` on a fresh copy indexes 194 of 197 Markdown files, and neither search, for the report filename or for the retest's own words, returns any OntoDoc review file.

There was no OntoDoc code or ranking change, and no plan task was added.

## Agent-Cost Measurement (2026-11-15)

The owner asked how OntoDoc helps an agent and whether that can be measured. The protocol was frozen before the run: `/opt/d2/ontodoc-agent-cost-20261115/PROTOCOL.md`, SHA-256 prefix `223ef1a9`. The results are in `RESULTS.md` beside it. It reuses the ontocode retest's 8 non-blind answerable questions and corpus copy, copied there from `/tmp`.

- Expected document found: `ontodoc search` 5/8, scripted `rg` whole-file reading 4/8.
- Evidence read for all 8 questions: 152,051 bytes against 3,219,580 bytes.
- MCP search: 163 ms on the first call, then a median of 4.9 ms.
- The MCP frame is about 34 KB per 10-result search, because `CallToolResult::structured` duplicates the JSON in `content` and `structuredContent`. This is recorded and nothing was changed.
- There is no quality or general claim, and no end-to-end agent task was run.
