---
name: Current project status
type: project
---

# Current Project Status

The [OntoIndex ideas plan](../docs/ONTOINDEX_IDEAS_PLAN.md) completed all five tasks: ADR19 generation checks and tested recovery, calibrated link census, synthetic capacity measurement and operator documentation. [Execution evidence](audit_session-2026-11-14-ontoindex-ideas.md) records passing Rust validation and unchanged pinned store IDs; [capacity evidence](audit_session-2026-11-14-ontoindex-ideas-capacity.md) retains all refusals and limits. Release and evaluation gates remain unchanged.

The [donor follow-up review](../docs/DONOR_REVIEW_PLAN.md) completed DR-1 documentation and evidence repairs after GPT source review found no runtime blocker. Original donor tasks stay closed; ranking, calibration, release and evaluation gates are unchanged.

Updated: 2026-11-16

The [donor candidates plan](../docs/DONOR_CANDIDATES_PLAN.md) completed ADR18, citation-preserving responses, the descriptive census and documentation. All required Rust checks passed; independent review found no blockers. Link scanning is uncalibrated and selected preparation matching unconfirmed. Ranking candidates remain deferred; release and evaluation gates are unchanged.

| Canonical plan | State | Remaining task | Counts |
| --- | --- | --- | --- |
| [RETRIEVAL_RESEARCH_PLAN.md](../docs/RETRIEVAL_RESEARCH_PLAN.md) | closed | None | 6 DONE (RR-0 registration, RR-1 step 1, RR-2 and RR-3 not opened, RR-HY, RR-DOC); step 1 met no threshold; [audit](audit_session-2026-11-16-retrieval-research.md) |
| [HYBRID_RETRIEVAL_PLAN.md](../docs/HYBRID_RETRIEVAL_PLAN.md) | blocked | HY-2, HY-3 wait for ADR decisions | 2 DONE (HY-0 stage 0, HY-1 fusion functions), 2 BLOCKED; [audit](audit_session-2026-11-16-hybrid-stage0.md) |
| Local 0.1.1 release | closed | None | tag `v0.1.1`, installed at `~/.local/opt/ontodoc-0.1.1`; [audit](audit_session-2026-11-16-local-release-011.md) |
| [SEARCH_RESPONSE_PLAN.md](../docs/SEARCH_RESPONSE_PLAN.md) | closed | None | 3 DONE (SR-ADR, SR-1, SR-DOC); [audit](audit_session-2026-11-16-search-response.md) |
| [STDIO_AUTO_INDEX_PLAN.md](../docs/STDIO_AUTO_INDEX_PLAN.md) | closed | None | 7 DONE (AI-ADR, PQ-1, PQ-DOC, PQ-SMOKE, PQ-ARM, AI-1, AI-DOC); [audit](audit_session-2026-11-15-stdio-auto-index.md) |
| [REFACTORING_PLAN.md](../docs/REFACTORING_PLAN.md) | closed | None | 7 DONE (revision 3 added RF-5, RF-CLOSE2) |
| [GRAPHRAG_REVIEW_PLAN.md](../docs/GRAPHRAG_REVIEW_PLAN.md) | closed | None in selected scope | 2 DONE |
| [GRAPHRAG_REFERENCES_PLAN.md](../docs/GRAPHRAG_REFERENCES_PLAN.md) | closed | None in approved scope | 4 DONE |
| [ONTODOC_IMPLEMENTATION_PLAN.md](ONTODOC_IMPLEMENTATION_PLAN.md) | closed | None | 74 DONE; REL-6 closed as a local install by owner decision, no public release; [audit](audit_session-2026-11-15-local-release.md) |

EVAL-3.2 completed the single registered held-out attempt and saved-score verification, both exit 0. H1 passed. With model-judged labels, H2 difference +0.032912 (95% CI [0.016128, 0.049996]) and H5 +0.032709 ([0.012113, 0.053406]) are exploratory and authorize no default promotion. The [held-out audit](audit_session-2026-11-13-held-out.md) records all hypotheses, per-corpus intervals, custody hashes and limitations. The owner approved unavailable held-out miss attribution (ADR16). Custody is consumed; preserve it.

OD-4.1 [records no-op defaults](audit_session-2026-11-13-default-noop.md): lexical, optional weights 0.3, abstention off. OD-4.0 [documents the results](audit_session-2026-11-13-evaluation-closeout.md). Retained candidate validation recorded 281 passing tests with one ignored helper, focused tests, formatting and all-target Clippy. Rust validation was not rerun in review round 9; no Rust source changed.

REL-6 local package preparation passed deterministic archive generation and install/index/search smoke; [release evidence](audit_session-2026-11-12-release-preparation.md#rel-6-2026-11-13) binds the original archive. REVIEW-9's read-only verifier now checks exact archive/member hashes and recursive source/Cargo/toolchain inventory: 22 synthetic tests and real archive verification exited 0. No tag or upload exists.

[Review round 9](audit_session-2026-11-13-review-round9.md) led to the separately identified corrected candidate. CORR-1 restored transport parity; CORR-2 removed sizing buffers; CORR-3 removed graph/tier scans; CORR-LOCK fixed descriptor retention with red/green regression evidence. CORR-4 passed default-parallel all-target tests, formatting, Clippy, locked release build, deterministic packaging and extracted-binary native smoke. The [corrected-candidate audit](audit_session-2026-11-13-corrected-candidate.md) retains failed attempts, exact identities, mixed timing observations and limits. Original parallel lock ownership remains unproven. Artifacts and `validation.json` are under `tmp/release-010/corrected/`; no general performance gain is claimed. The [evaluation companion](../docs/EVALUATION_0.1.0.md) accompanies the package and keeps measurements attached to the original binary.

Earlier completed work: [freeze](audit_session-2026-11-13-preregistration.md), [dev reproducibility](audit_session-2026-11-13-dev-proof.md), [capacity](audit_session-2026-11-13-capacity.md), [licensing/candidate](audit_session-2026-11-12-release-preparation.md). Native target is Linux x86_64 GNU, glibc 2.39+, kernel 5.6+. Deployment ingress remains untested; capacity has no acceptance verdict. The donor snapshot stays read-only. Proposed evidence/diagnostics extensions remain deferred.

See [pending tasks](project_pending-tasks.md) for runtime and publication gates.

Unreleased 0.1.1 adds chunk/section neighbour citations through the existing `related` tool. [GraphRAG closeout](audit_session-2026-11-13-graphrag-references.md) records 293 passing tests, 3 ignored, formatting and Clippy exit 0, independent review and ordinary-corpus cap counts. GR-5 remains deferred; this changes no ranking, schema, defaults or publication decision.
