# OntoDoc 0.1.0 release plan

Revision 13, 2026-11-13. Status: **73 DONE, 1 BLOCKED; corrected candidate locally validated and packaged, publication blocked**. The [canonical plan](../.memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md) owns task state. Design authority is the [service ADR](../.memory-bank/ADR_ONTODOC_SERVICE.md), revision 16.

## Review conclusion

The service has a clear existing-owner architecture, separate corpus stores, deterministic publication and careful evaluation custody. CORR-1..3 repaired HTTP/stdio response-overflow parity, removed temporary response-sizing buffers and reduced graph/tier scans. CORR-LOCK corrected descriptor retention at lock release with a regression that failed before the fix and passed afterward. Default-parallel tests and local package smoke pass. The original parallel failure's holder remains unidentified; the regression proves the corrected mechanism.

The [GPT review and alternatives](../.memory-bank/audit_session-2026-11-13-review-round9.md) distinguish reproduced defects, source-supported complexity and unmeasured hypotheses. The selected approach keeps the current owners and standard library, adds no dependency, preserves freshness and admission contracts, and uses synthetic regressions before performance measurements.

## Completed evidence

- The sole held-out attempt and saved-score verification passed. H1 passed against an empty exact-substring B0; that is a weak sanity baseline. H2/H5 effects are exploratory because labels were model-judged. Defaults remain lexical, optional weights 0.3, abstention off. All contrasts, system metrics, H3/H4 and limitations are in [EVALUATION_0.1.0.md](EVALUATION_0.1.0.md).
- [Candidate validation and local package smoke](../.memory-bank/audit_session-2026-11-12-release-preparation.md) recorded formatting, Clippy, 281 passing tests plus one ignored helper, a release build, five-tool transport smoke and clean-directory install/index/search. These are retained historical results; this review did not rerun Rust builds.
- [Capacity observations](../.memory-bank/audit_session-2026-11-13-capacity.md) cover both pinned corpora and a near-file-cap corpus at concurrency 1 and 4. There is no latency acceptance target or capacity verdict.
- REVIEW-9 added a read-only package/source verifier: 22 synthetic tests passed; the preserved nine-member archive and 28-file scoped source inventory passed. Annotated-tag validation remains unperformed because no tag was supplied.
- REVIEW-9-DOC corrects deployment/ADR metadata, publishes the complete aggregate evaluation companion and replaces obsolete release steps with this current sequence.

## Preserved identities

| Item | Identity |
| --- | --- |
| Candidate F | `cc116c44a4d7f854bc19c6a4b40d75098a437268` |
| Binary SHA-256 | `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55` |
| Archive SHA-256 | `e53b9453cbf0c85f8f6f5ab7fbee7e45f1fc92933420c9f0800a1d8299f24dd6` |
| Local archive | `tmp/release-010/package/ontodoc-0.1.0-linux-x86_64-gnu.tar.gz` |
| Runtime target | Linux x86_64 GNU; glibc ≥2.39, kernel ≥5.6, libgcc_s |

The archive contains binary, README, CHANGELOG, licence/notice texts, deployment guide and checksums. Its embedded documentation is frozen with its bytes; the new evaluation companion and corrected deployment guide must accompany any publication. The binary retains 312 recorded home-directory path matches. No tag or upload exists.

## Selected next sequence

1. **Safe review fixes complete.** REVIEW-9 and REVIEW-9-DOC closed after focused validation. Archive, measured binary, registration and consumed attempt are preserved.
2. **Corrected candidate authorized.** The owner's subsequent `proceed` accepts a separate corrected candidate with a new binary hash, preserving the measured candidate and stating that evaluation describes the original binary. Restore the accepted ADR parity contract. See [authorization and execution](../.memory-bank/audit_session-2026-11-13-corrected-candidate.md).
3. **CORR-1 complete: HTTP overflow parity.** Existing transport owners share typed cap errors and native request-ID admission. An ID whose refusal cannot fit receives HTTP 413 before dispatch. Forty focused tests, formatting and all-target Clippy passed.
4. **CORR-2 complete: response sizing.** `cli::json_fits` uses fixed storage and preserves late serializer errors, truncation order and complete MCP bounds. Escaping, thresholds and old-reference byte comparisons passed, with formatting and all-target Clippy.
5. **CORR-3 complete: indexing scans.** Graph events are filtered before lookup; validated sorted spans use binary search with a compatibility fallback. Heading ancestry is cached once per section with a stack bounded by public u8 levels. Old-reference graph/tier outputs and canonical artifacts match. Single debug timing observations are recorded without a general speedup claim.
6. **CORR-LOCK and CORR-4 complete: validation and local package.** Default-parallel all-target tests, formatting, all-target Clippy and locked release build exited 0. Separate deterministic binary/source archives, installation checks, 30 synthetic artifact comparisons, five-tool old/new transport parity, cap errors and six large-response comparisons passed. Repeated time/RSS observations are mixed and carry no general speedup claim. Original package, verifier pins and custody are preserved. Failed test/helper attempts remain in the audit.
7. **Finish REL-6 after owner decisions.** Bind destination, package-only versus repository/history exposure and build-path acceptance; verify the selected candidate/tag identities, distribute all-outcomes documentation, then record annotated tag, publication and public install smoke. Publication is still blocked.

Steps 3–6 are complete. Current receipts are in the [corrected-candidate audit](../.memory-bank/audit_session-2026-11-13-corrected-candidate.md). There is no held-out rerun in this sequence.

Corrected local artifacts are under `tmp/release-010/corrected/`: `ontodoc-0.1.0-corrected-linux-x86_64-gnu.tar.gz`, `ontodoc-0.1.0-corrected-source.tar.gz`, `SHA256SUMS`, `package-manifest.json` and `validation.json`. Binary SHA-256 is `2fc26f1527ce59842e135ef47fd80d950541bd6389fa360c43f987dd8684c2a3`; binary archive SHA-256 is `daccd66812a83aa3082a9a29b38fd7be7a91d63cdaedf9ad367e0bd394b64725`. The captured dirty source inventory defines identity; base HEAD is not a candidate commit. Corrected binary contains 317 `/home/` prefix occurrences; release path review remains open.

## Repeatable verification now

Run the verifier's synthetic tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests -p test_verify_release.py
```

`scripts/verify_release.py` binds the preserved candidate F. Its default source check intentionally fails against the corrected workspace; run that identity check only against the preserved source. Corrected packaging uses its own captured source and hashes. After an annotated tag exists for the preserved candidate, `--tag v0.1.0` checks that identity. No tag check establishes release readiness or reproducible compilation. Do not rerun a create-only packager as a verification command.

## Decisions and stop conditions

| Decision | Selected recommendation / unresolved input |
| --- | --- |
| Post-measurement defect | Authorized: separate corrected candidate, retained original evidence, explicit changed-binary disclosure |
| Destination and exposure | Owner supplies destination and package-only or repository/history scope |
| Embedded build paths | Owner accepts recorded paths or authorizes a separately identified rebuild; stripping changes identity |
| Ingress | Keep the [deployment example](DEPLOYMENT.md) untested until target validation is authorized |
| Capacity | Retain measurements without a pass verdict until an acceptance target exists |

Whole-repository publication also exposes evaluation/judging material, internal plans, snapshot provenance and commit identities; package exclusion does not hide them from a tag. The owner retained the read-only donor snapshot, selected MIT for the root package, local output under `./`, and Linux x86_64 GNU. Licence scopes and provenance remain in [release preparation evidence](../.memory-bank/audit_session-2026-11-12-release-preparation.md).

CORR-1 restores the accepted ADR parity contract. The authorized post-held-out repair changes binary identity; original measurements stay attached to the original binary. Never delete custody, retry the consumed attempt, relabel after results, or describe a corrected binary as the measured artifact.

Historical freeze, preparation and accepted evaluation decisions remain in the [canonical tracker](../.memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md), [release decisions](../.memory-bank/audit_session-2026-11-13-release-decisions.md), [preregistration](../.memory-bank/audit_session-2026-11-13-preregistration.md) and [held-out audit](../.memory-bank/audit_session-2026-11-13-held-out.md). They are evidence, not instructions to rerun the experiment.

## Deferred

Human-label confirmation needs fresh queries and its own protocol. Evidence/diagnostics extensions remain Proposed. New tools, non-loopback binding, built-in authentication, donor editing and ranking/default changes are outside this repair sequence. Compact the canonical tracker before adding more cards: it is near the 128 KiB parser ceiling; preserve all authenticated state and historical evidence.
