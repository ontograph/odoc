# Review round 9 — release correctness, cost and plan simplification

Date: 2026-11-13. User requested a critical review, alternatives, one selected plan and execution, with GPT subagents and OntoIndex navigation. Three GPT-6.1-Sol explorers reviewed runtime, retrieval/indexing and release work; a GPT implementation agent and an independent GPT source challenger handled REVIEW-9. This audit separates current source evidence, live synthetic evidence, retained measurements and unmeasured complexity estimates.

## What went well

- Existing `mcp → cli::App → search` ownership avoids a parallel HTTP ranker. Per-corpus roots/stores and immutable slots keep corpus identity separate; work permits remain with blocking work. Retain these contracts.
- Atomic generation publication, deterministic identifiers/artifacts and current-byte freshness have explicit regression coverage in prior closure audits. This review inspected affected owners; it did not rerun the entire Rust suite or prove every boundary afresh.
- Evaluation custody binds one frozen binary and one consumed attempt. Model-judged results are correctly exploratory, defaults remain unchanged, and the limitations are disclosed in the accepted audit. Preserve those decisions.
- The package copies the measured binary. Current REVIEW-9 validation confirms archive/member hashes and exact recursive source inventory, improving on the create-only package script's narrower comparison.

## Worst and forgotten findings

| Priority | Finding and evidence | Consequence |
| --- | --- | --- |
| Release blocker | `src/http.rs:163` replaces oversized success bodies with plain 500; `src/mcp.rs:303` returns typed `cap_exceeded` with request ID. ADR parity at `ADR_ONTODOC_SERVICE.md:704` requires equivalent responses apart from framing. Live synthetic reproduction below. | Valid requests have inconsistent error semantics; ordinary parity smoke did not cover this. |
| High cost, unmeasured | `src/cli.rs:661` serializes the whole response into a vector after each dropped item. `App::search:830` clones selected L2 text first; `src/chunk.rs:25` permits an oversized paragraph chunk. | Fifty ~2 MiB results can imply ~100 MiB text copies and ~2.5 GiB aggregate serialization allocation across repeated shrinking. This is arithmetic, not peak RSS or a measured production workload. |
| Index scalability | `src/graph.rs:80–83,143` scans chunks for every parser event, before filtering relevant events. `src/tiers.rs:55,107` rescans all preceding sections for each chunk's ancestors. | Quadratic synthetic shapes; 10,000 sibling headings can produce 50,005,000 scan visits. Existing near-file-cap corpus has one section and misses this shape. |
| Deferred resource gap | `src/store.rs:425–452` materializes parsed sections/chunks for all files before heading/count checks. | Invalid inputs can consume avoidable work before rejection. Earlier per-file checks help but do not bound allocations inside one parser call. |
| Reporting gap | Repository README/CHANGELOG defer E1 and per-corpus intervals to an audit absent from the archive. | Package-only readers lack all outcomes and limitations. New standalone evaluation companion fixes the distribution material; preserved archive remains unchanged. |
| Process defect | REL-6 validation invoked the create-only package writer; source matching missed deleted/non-Rust files. | Repeat validation either refuses or gives incomplete identity evidence. Read-only verifier now covers exact inventory and optional annotated tags. |
| Documentation drift | Deployment source said target/capacity pending although packaging substituted newer facts. ADR header said rev15 despite accepted rev16; release plan mixed obsolete instructions with completed steps. | Multiple contradictory sources. Source docs and current plan corrected. |

### Live HTTP reproduction

Runtime explorer used the unchanged frozen binary and a synthetic indexed corpus. `tools/call status` with a 130900-character string request ID produced a 130989-byte HTTP request and a 130990-byte stdio frame, each within 131072 bytes. Stdio returned JSON-RPC error -32603 with `cap_exceeded`, the original ID and a 131026-byte frame. HTTP returned status 500, 31 plain-text bytes (`response exceeds the byte limit`) and lost the ID. Small-ID controls returned matching structured fresh status. In-memory probe and both servers exited 0; tool receipt `40eb8f`. An earlier list-tools probe had the same boundary mismatch (`556408`).

The explorer ran the probe in this review session; the coordinator subsequently inspected both boundary implementations and the ADR. Binary identity remains `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`. No held-out input or product source changed.

OntoIndex search/context/impact were used by reviewers. Workspace preparation sometimes returned busy/untracked coverage limits; committed fallback was labelled and source checked. `bound_response` impact was LOW with two direct dependents (router and test); this does not prove transport correctness. New Python symbols had UNKNOWN impact because indexed coverage was absent. No manual reindex was started.

## Alternatives and selected approaches

| Issue | Alternatives | Pick and reason |
| --- | --- | --- |
| HTTP conflict | Keep and explicitly amend ADR; globally shrink request limits; repair error parity in existing transport owners | Repair parity in a separately identified candidate. An exception weakens the stated contract; a smaller global limit rejects valid traffic and needs another policy decision. |
| Repeated serialization | Allocate full vectors repeatedly; capped serde writer; incremental response construction | Capped writer first, reusing existing `fits_json` approach. Small behavior-preserving repair; it does not remove all cloning or repeated traversal. Measure residual cost before incremental construction. |
| Graph lookup | Linear scan; binary search; interval tree | Filter events then binary search proven sorted/nonoverlapping spans. No new tree/dependency. Preserve edge and diagnostic ordering. |
| Heading ancestry | Reverse scan; early break on level1; per-section heading stack | Stack with at most six heading levels. Early break still scans quadratic flat level2 documents. Preserve preambles, skipped levels and exact tier bytes. |
| Late caps | Leave as is; per-file/cumulative rejection; streaming parser redesign | Per-file checks as a separate bounded follow-up. Streaming redesign needs evidence and a larger correctness review. |
| Package verification | Rerun writer; rebuild binary; read-only pinned verifier | Read-only verifier, implemented. It preserves measured bytes and verifies exact source membership. |
| Missing outcomes | Repack original; link private audit; standalone companion | Standalone sanitized companion, implemented; distribute next to preserved archive, or include in a separately identified corrected package. |
| Plan complexity | Add another orchestration layer; continuously recheck blocked publication; compact current plan and retain exact reopen gate | Current plan rewritten around remaining actions. No unchanged blocked reruns. Canonical tracker needs careful compaction before new cards, retaining receipt/lease/history state. |

Do not add caches that replace current-byte freshness with TTL/mtime, another REST ranker, generalized serializers, interval-tree dependencies or new tools. The apparent `top_k.max(50)` mismatch is not a current defect: production maximum and evaluation FETCH are both 50. No change selected there.

The bureaucracy is a genuine cost: the canonical tracker approached 128 KiB, while obsolete freeze instructions and documentation substitutions obscured present state. Keep one authoritative tracker, one short release sequence and immutable evidence links. Do not weaken custody or validation to reduce line count.

## Evidence quality challenge

The evaluation reports 21 contrasts including zero-crossing and negative effects; the public companion now carries all of them plus system metrics, H3/H4 and withdrawals. B0 returned no rows, so H1 is a weak sanity result. All 220 test queries were model-drafted and labels model-judged; shared biases remain. Twelve unanswered queries were withdrawn before measurement, raising absolute scores and altering membership. Miss attribution was unavailable under the accepted exception. None supports a confirmatory claim or default promotion.

Capacity observations establish measurements on recorded shapes and hardware, not multi-user readiness or an SLA. Ingress authentication/TLS/encoded routing/drain has not been exercised end to end. Preserve these limits. Performance proposals above require synthetic measurements and semantic/canonical-byte parity before claiming improvement.

## Work performed

REVIEW-9 added only `scripts/verify_release.py` and `tests/test_verify_release.py`: bounded gzip/tar inspection without extraction, pinned archive/binary/member checks, receipt/checksum agreement, exact source inventory including non-Rust files, and optional literal annotated-tag comparison. It uses Python standard library and Git, does not execute the binary or mutate artifacts, and requires a trusted quiescent workspace. No concurrent-filesystem containment claim is made.

Independent source challenge found no material defect under that scope. The challenger did not run tests; its role was read-only. Implementation agent validation in this session:

| Command | Exit | Result / receipt |
| --- | ---: | --- |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_verify_release.py` | 0 | 22 tests; `636c9f` |
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify_release.py` | 0 | Nine members and 28 source files; `75c713` |
| `git diff --check` | 0 | `e4305d` |

Tag check skipped because no tag was supplied. Rust builds/tests, ingress installation, archive regeneration, held-out rerun and publication were not performed. Historical checks are not restated as current validation.

REVIEW-9-DOC adds [EVALUATION_0.1.0.md](../docs/EVALUATION_0.1.0.md) from the existing aggregate audit only, corrects deployment/ADR metadata and updates current tracking. Product owners/flows remain unchanged. The ADR change corrects its revision header only. Existing dirty work, including CLAUDE.md and the read-only snapshot, is preserved.

Documentation validation: independent GPT source challenge found no material issue and confirmed all 21 contrasts and 12 system rows. Coordinator checked local links/anchors, Markdown fences and exact aggregate-section equality with an inline Python check (exit 0); `git diff --check` exited 0. Strict plan validation passed 68 tasks, with the retained warning that the canonical tracker is near its 131072-byte ceiling. No additional runtime validation is claimed.

## Consolidated sequence and stop

The [release plan](../docs/RELEASE_0.1.0_PLAN.md#selected-next-sequence) is the single selected sequence: finish safe review repairs → explicit corrected-candidate decision → transport parity → allocation reduction → graph/tier scans → candidate validation and synthetic performance evidence → destination/exposure decision and publication. Each runtime step needs a scoped canonical card after the decision; none is marked implemented.

AGENTS.md explicitly says to report an ADR/source conflict and stop affected implementation until resolved. The release plan also requires an owner decision for a code fix after the held-out run because binary identity changes. Recommended resolution is a separate corrected candidate, original measurement preserved, no held-out retry and explicit notes distinguishing the two binaries. REL-6 remains blocked on that decision plus destination/exposure/build-path acceptance.

