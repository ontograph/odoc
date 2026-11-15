# Release preparation, 2026-11-12

## REL-5, 2026-11-13

REL-3 retry update: the owner replied "proceed tasks" directly after the request for one corrected dispatch retry. This supplies authorization for that scoped correction. The packet now contains all required fields and was accepted as `capacity_script_retry`; no additional model-provider retry or product change was requested. Earlier rejection counts remain historical. Capacity execution must wait for script review and self-check.

Continuation checkpoint: REL-3 protocol registered and task reopened after correcting its read-only performance-role write declaration to a generated validation workspace. Validator passed (129,436 bytes). Discovery verified both corpus pins and a bounded near-file-cap fixture without reading evaluation labels. Luna discovery hit 429, Grok Composer was unavailable, and Sol completed discovery. Measurement-worker dispatch encountered four packet-shape tool rejections (Scenario, Given, Write set), then one child rejection for missing `Non-goals:` and `Stop when:` lines. No worker edits or measurements occurred. Dispatch stopped: no observable progress; retry budget spent. REL-3 stays active/OPEN, with no live child/job. Reopen condition for further dispatch is explicit authorization of one corrected packet retry with all required lines; a bare proceed does not reset the skill's guard. This is a workflow stop, not a product failure.

The accepted target is Linux x86_64 GNU, glibc 2.39+, kernel 5.6+. Clean linked candidate `tmp/release-010/candidate` is commit `cc116c44a4d7f854bc19c6a4b40d75098a437268`. The main checkout's index and unrelated CLAUDE.md edit were preserved. No tag, push or evaluation freeze occurred.

Managed compiler job `ontodoc-rel5-20261113-a` completed with exit 0 in 124.076 seconds. From the candidate, `python3 ../evidence/validate.py` ran these commands sequentially with `CARGO_BUILD_JOBS=4`; all exited 0:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`: 281 passed, zero failed, one ignored.
- `cargo build --release --locked`

`python3 tmp/release-010/evidence/smoke.py` exited 0: the retained executable reports `ontodoc 0.1.0`, indexes a synthetic fixture, initializes and calls status/search/read/related/verify over stdio and HTTP, returns equal structured results and a valid citation, and shuts both processes down successfully. SHA-256 before and after matches `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. Candidate status remained clean after validation.

Evidence files under `tmp/release-010/evidence/`: `candidate.json`, `checks.json`, `check-0.log` through `check-3.log`, `smoke.py`, `smoke.json`, and `binary-inspection.json`. The managed receipt was harvested using `just --justfile /opt/d1/_wrk/ontocode/justfile lane-result ontodoc-rel5-20261113-a --wait 60`, reader exit 0 and nested terminal exit 0. The wrapper provides orchestration only; the donor snapshot was never built.

Host: rustc 1.95.0, glibc 2.39, Linux 6.11.0-29-generic x86_64. Retained binary is 10,597,520 bytes, dynamic and unstripped, with GLIBC_2.39 among its required versions. The bounded home-path scan found 312 matches, including Cargo registry source locations. These remain in the unchanged binary and require consideration when choosing publication exposure. No claim of path scrubbing, minimum-kernel testing, reproducible rebuild bytes, capacity or retrieval improvement follows from this validation.

Strict `complete-local` closed REL-5; canonical counts are 59 DONE / 7 OPEN. REL-3 next requires a concrete capacity protocol. Recovery: one resumed compiler job harvested, no duplicate build; one evidence-patch context mismatch made no write and was corrected. The ignored test was not run; no real held-out inputs were accessed. Capacity, dev reproducibility, sweep, freeze, held-out measurement and release publication remain pending.

## REL-1, 2026-11-13

Owner "proceed tasks" follows the explicit proposal `Copyright (c) 2026 OntoDoc contributors` and is treated as acceptance. Root LICENSE and Cargo package metadata now select MIT. The owner selected keeping the snapshot; its source, NOTICE and COPY_MANIFEST remain unchanged. The original Apache LICENSE is preserved byte-for-byte as LICENSE-APACHE, with the historical manifest mapping explained in README and AGENTS. No ADR change is needed for this retained read-only reference.

Dependency commands (all exit 0, CARGO_BUILD_JOBS=4, no compilation): `cargo metadata --locked --offline --format-version 1`; `cargo tree --locked --offline --target x86_64-unknown-linux-gnu --edges normal --prefix none --format '{p}'`; same tree with edges `normal,build` and `normal,no-proc-macro`; after metadata change, `cargo metadata --locked --offline --no-deps --format-version 1`. THIRD_PARTY_LICENSES.md contains 114 crates: 90 normal-graph runtime candidates and 24 build-only. This is conservative dependency reachability, not proof of linked symbols. Archive licence/notice/copyright texts are included, including Unicode terms and LLVM exception alternatives. Dev-only and other-target crates are excluded.

rmcp and rmcp-macros omit licence files in their archives; their common upstream LICENSE was retrieved at packaged commit 3529c3675ff64db805bd947ca6ece6090809e43d (SHA-256 0382b0057770ca05e9c350a50aa3b1c1fea84da0bc81d723bf00b9aa841be58a). sse-stream declares MIT/Apache-2.0 but has no licence file in its archive or the complete upstream tree at packaged commit 4d2d2e01a6b1155eb52743164d62574e340ffa96. Apache-2.0 is selected and its standard terms supplied from apache.org; no copyright attribution is invented. The attempted upstream LICENSE request returned 404; tree discovery and standard-text retrieval succeeded. Source URLs and hashes accompany texts.

Validation found an initial tool-output truncation in the generated notices, repaired by bounded reads. A subsequent byte comparison isolated CRLF normalization (20 CRLF sequences); normalized text is complete, and the formatting distinction from recorded original-byte hashes is disclosed. All COPY_MANIFEST hashes matched, mapping only LICENSE to LICENSE-APACHE. Root Cargo metadata parses as MIT. git diff --check passed. No Rust source changed; compilation/tests/Clippy and release evaluation were not run.

Read-only Sol explorer compared seven donor/current pairs covering query budgets, citations and metric records. Donor query.rs:113/265 versus src/cli.rs:655 and src/mcp.rs:247/265/303; lineage_admission.rs:113, lineage_grounding.rs:265 and docs.rs:245 versus src/cite.rs:86/177/237; lineage_replay.rs:135/183/202 versus src/eval/run.rs:1713/2594/2643 and metrics.rs:51. Two Python lexical/line comparisons exited 0: no shared comments/string literals of at least 50 characters, at most two meaningful identical lines, and longest token matches 5–30 tokens of generic syntax/serde boilerplate. No substantive copied text identified within this scope; this does not establish legal clearance or exclude copying elsewhere. OntoIndex committed index matched HEAD; dirty coverage was incomplete, so source governed. Luna dispatch failed with 429 and Grok Composer was unavailable; neither was retried. Sol completed without edits or benchmark access.

Authority: user instruction, “proceed tasks from docs/RELEASE_0.1.0_PLAN.md”. This authorizes reversible preparation and independent documentation. It does not supply a copyright identity, publication destination, snapshot disposition, binary target, manifest pin or release fallback decision.

## REL-0

Registered REL-0 through REL-6 in the canonical plan, with independent documentation first and owner-gated candidate work thereafter. EVAL-3.0 now depends on candidate validation and capacity measurement. The original 53 completed task cards are archived verbatim with individual SHA-256 hashes; active status, dependencies, owner/write sets and original closeout bytes remain. No completed task was reopened.

`src/cli.rs:1240` rejects `--gates` during preparation capture and requires operator fields. EVAL-3.1 now writes a new `preparation-selected.json`, waits for its second owner pin and checks sweep power against the current 165 answerable test queries before freeze. EVAL-3.x requires the linked clean worktree at F, external benchmark paths and shared Git-common custody. No evaluation payload was opened or run.

Initial plan validation passed at 59 tasks. Integration validation caught YAML flow-style closeout fields unsupported by the strict tracker and an unwrapped Clippy command. Corrected by retaining original closeout blocks verbatim and declaring the managed compiler-lane command. Final structural validation passed at 66 tasks. The tracker remains above its advisory size/line threshold but below its hard 131,072-byte limit.

Current manifest bytes hash to `1e4835cd994a799b136fcf7c095567e43ae9bbaafd800a6bdbc51d69b1cf5125` (`sha256sum eval/preparation.json`, exit 0). This verifies bytes only; owner approval remains pending in `eval/audit_session-preparation.md`.

## REL-2

Corrected README search defaults and unmeasured capacity, exposed hidden corpus directories, native Linux/openat2 requirement, and stdio input bounds. AGENTS now states current schema 4. Architecture now describes stateless HTTP, current exposure approval, actual folder-name handling and exploratory evaluation. Replaced the stale pending list with current blockers; its HEAD contents remain verbatim in `archive/release-010-pending-history.md`.

Source evidence: `src/mcp.rs:550–636` registry discovery; `src/cli.rs:227–269` defaults; `src/http.rs:65–149` stateless routing and limits; `src/http.rs:245` HTTP/1.1 transport; ADR revision 10 at `ADR_ONTODOC_SERVICE.md:662`; schema and frame constants in store/MCP. The explorer verified these sources, including rmcp 1.7 protocol headers, without execution. No behavior/ADR conflict was identified in this bounded review.

Validation: `git diff --check`, exit 0. Inline Python checked changed-document relative file references, current-status assertions and exact preservation of the previous pending router, exit 0. No Rust compilation or runtime claim.

## REL-4

Added `docs/DEPLOYMENT.md` and a README link. The guide contains an unprivileged read-only systemd service, explicit trusted indexing, nginx TLS plus Basic authentication on every route, fixed upstream Host, preserved escaped URI and protocol headers, unchanged Origin refusal, work limits and graceful shutdown tradeoffs. All installation and end-to-end acceptance commands are instructions for the target, not commands run here. The guide is explicitly untested; no ingress was installed.

Validation: `git diff --check`, exit 0; inline Python guide-reference, Markdown-fence, INI parse and explicit ingress-contract checks, exit 0. `project_plan_validate` PASS, 66 tasks. `nginx -t`, `systemd-analyze verify`, installed-service checks and end-to-end acceptance are skipped because no deployment was provisioned; INI parsing does not establish systemd compatibility.

## Scope and limits

Initial HEAD: `ee511dc0f1b2300266fee23e7012463d9d3e2f11`. Pre-existing changes: `CLAUDE.md` modified, `docs/RELEASE_0.1.0_PLAN.md` untracked. The CLAUDE edit is preserved. No source, Cargo metadata, licence, donor snapshot, evaluation data or custody edits are authorized by the documentation tasks.

OntoIndex context for `src/cli.rs::run_eval` and read-only explorer context for HTTP were checked against source. The explorer reported current committed index at HEAD; dirty documentation is outside its graph coverage. Luna delegation failed with HTTP 429 before any tool call; Grok Composer was unavailable; Sol completed read-only verification. No child changed files.

Rust compilation, format, lint, benchmark, freeze and held-out checks are skipped for this documentation-only work. No server, ingress, tag or release was installed or published.

## Manager outcome and validation corrections

Strict `complete-local` transitions completed REL-0, REL-2 and REL-4. Final manager state is `waiting`, active task EVAL-2.1; 56 DONE and 10 OPEN. Reopen requires the exact preparation pin; REL-1 separately needs the copyright holder/year and snapshot disposition. Full evaluation, candidate validation, capacity measurement and publication are incomplete.

Validation corrections retained: a generated YAML alias collision was caught before repository writes; strict tracker parsing required original block-form closeout fields; an existing unquoted colon in OD-4.1 evidence needed scalar quoting for manager serialization; REL-4's attempted explicit successor REL-1 was refused because REL-1 has blockers, so completion used automatic successor selection. No rejected transition landed or was bypassed. The initial OntoIndex scope check treated directory names as literal expected files and failed; its corrected invocation lists each exact expected path. No compiler, custody or attempt recovery occurred.

Final structural validation: `project_plan_validate` PASS at 66 tasks, with advisory size/line warnings (tracker remains below 131,072 bytes). `manager_loop_next` returns `wait`/`no-dispatch` with the explicit owner gate. Existing archive/state proof and documentation checks are recorded above; these support documentation completion only.

Final OntoIndex `verify_diff` PASS with exact file paths; symbol scope was not checked and untracked documents were checked separately. Final `git diff --check` and inline Python counts/gate/original-DONE-state/file-reference/write-scope validation exited 0. The first filesystem scope check detected `.memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md.integrate.lock`, created by the manager transitions; the corrected check includes that tool-owned artifact and preserves it. No source, eval payload or donor snapshot changed. Tracker size after closeout is 124,609 bytes. No commit was created.
# REL-6 (2026-11-13)

Local reversible release preparation completed after accepted EVAL-3.2, OD-4.1 no-op and OD-4.0 closeout. CHANGELOG records all main hypothesis outcomes and restrictions. Publication remains blocked on destination, repository/history exposure and acceptance of embedded build paths; no tag, commit or upload was performed.

`python3 tmp/release-010/package/build-package.py` exited 0. It created `tmp/release-010/package/ontodoc-0.1.0-linux-x86_64-gnu.tar.gz`, SHA-256 `e53b9453cbf0c85f8f6f5ab7fbee7e45f1fc92933420c9f0800a1d8299f24dd6`, with external and internal SHA256SUMS. Binary SHA-256 exactly matches accepted report `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. All current Rust source/Cargo bytes match candidate F. Two in-memory archive generations were byte-identical. Package contains the binary, standalone installation README derived from source, CHANGELOG, MIT and Apache licences, NOTICE, dependency licence texts and an untested deployment guide updated to the selected target. Unshipped audit/source links are labelled references; no donor code or benchmark payload is included. Licence text is copied exactly.

The archive was extracted into a unique disposable directory, internal checksums verified, `--version` checked, a synthetic corpus indexed and lexical search returned a hit; all commands exited 0. Owned staging was removed on exit. `tmp/release-010/package/validation.json` retains commands, outputs, file hashes and assertions. The script refuses overwriting an existing archive. These local checks do not establish tag/source identity for a future tag or a public installation test.

