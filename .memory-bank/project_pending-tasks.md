---
name: Pending tasks
type: project
---

# Pending Tasks

Updated: 2026-11-17. The [0.1.2 release](../docs/RELEASE_0.1.2_PLAN.md) is complete:
commit `8e94a3d`, annotated tag `v0.1.2`, private `er77/ontodoc` GitHub release,
verified download and default local install. No release work remains in the
authorized scope; [audit](audit_session-2026-11-17-release-012.md). Publication
blockers below describe the historical 0.1.0 candidate. Public exposure remains
undecided.

The [refactoring plan](../docs/REFACTORING_PLAN.md) is closed with 5 DONE: research-document corrections, `section_ranking` removal and the pinned `eval misses` move. [Audit](audit_session-2026-11-15-refactoring.md) records commands and limits. No ranking, schema or default changed.

[OntoIndex ideas](../docs/ONTOINDEX_IDEAS_PLAN.md) has no remaining tasks in the authorized scope. ADR19 refusal/recovery, calibrated census, capacity measurement and documentation are complete; [evidence](audit_session-2026-11-14-ontoindex-ideas.md) records validation. Additional load cost, new link-resolution rules and a capacity acceptance target remain deferred; release and evaluation gates are unchanged.

The [donor follow-up review](../docs/DONOR_REVIEW_PLAN.md) completed DR-1 documentation and evidence repairs after GPT source review found no runtime blocker. Original donor tasks stay closed; ranking, calibration, release and evaluation gates are unchanged.

[Donor candidates](../docs/DONOR_CANDIDATES_PLAN.md) completed its four authorized tasks. Ranking and link-resolution candidates remain deferred; the census scanner was uncalibrated and no edge-gain claim follows. No ranking experiment, test-split access or reindex is authorized.

The [GraphRAG review](../docs/GRAPHRAG_REVIEW_PLAN.md) is closed: GRR-1 and GRR-2 DONE. Typed mention lookup is corrected and covered by fresh focused tests; the recommended GR-5 descriptive protocol still requires approval before payload access. See [review evidence](audit_session-2026-11-14-graphrag-review.md). The original completed tracker remains closed.

The [GraphRAG references plan](../docs/GRAPHRAG_REFERENCES_PLAN.md) is closed with 4 DONE; ADR17 and unreleased 0.1.1 neighbour citations are implemented and documented. [Closeout evidence](audit_session-2026-11-13-graphrag-references.md) records validation and limits. GR-5 remains deferred pending its diagnosis-rule decision; REL-6 publication gates remain unchanged.

Updated: 2026-11-15. The [canonical plan](ONTODOC_IMPLEMENTATION_PLAN.md) is closed with 74 DONE. REL-6 closed as a local install only by owner decision, with no public release, tag or upload ([local release audit](audit_session-2026-11-15-local-release.md)). Earlier status as of 2026-11-13 follows. CORR-0..4 and CORR-LOCK complete; corrected binary/source archives and validation receipts are under `tmp/release-010/corrected/`. Default-parallel tests, Clippy, formatting, build and native smoke pass. Original measured artifacts/custody remain unchanged. Publication destination, exposure and embedded-path decisions remain.

- REL-6 original package is preserved under `tmp/release-010/package/`; archive/install smoke evidence remains. The corrected package passed separate source identity, deterministic archive, extracted install/index/search and synthetic transport/artifact checks. See [corrected-candidate evidence](audit_session-2026-11-13-corrected-candidate.md).
- [Corrected candidate closure](audit_session-2026-11-13-corrected-candidate.md) records transport parity, response allocation, graph/tier scans, lock lifetime and local validation/package. Tracker descriptions were compacted with exact archive and state comparison. Current validation is recorded; final task is publication only.
- REVIEW-9-DOC completed release/deployment corrections and the [all-outcomes companion](../docs/EVALUATION_0.1.0.md). Distribute that companion and corrected deployment guide alongside the preserved archive; its embedded documentation has not changed.
- Publication requires a destination and exposure decision: package only, or repository with history. The latter also exposes eval/judging data, internal plans, snapshot provenance and commit identities. The unchanged binary contains recorded home build paths; owner acceptance remains needed. No remote, tag or upload was created. Source identity against the eventual tag and public install smoke remain unperformed.
- Evaluation is complete. Preserve registration and the completed sole attempt in shared Git-common custody. [Accepted audit](audit_session-2026-11-13-held-out.md) reports model-judged exploratory effects only; defaults remain lexical with abstention off. No retry, relabeling or post-test tuning is authorized; future human-label confirmation needs fresh test queries.
- Native target: Linux x86_64 GNU, glibc 2.39+, kernel 5.6+. No ingress installation is planned; the deployment example remains untested. [Capacity observations](audit_session-2026-11-13-capacity.md) have no latency acceptance target.
- Proposed evidence/diagnostics extensions and additional retrieval changes remain deferred. No new tool, default promotion, built-in authentication or donor editing is authorized.
