# Donor adoption review and consolidated next steps

Date: 2026-11-13. Scope: donor ADR and its measurement prerequisites, with a documentary cross-check of release status. Two GPT-6-Sol explorers reviewed independent execution and planning questions. This is a source/document review, not a current build, benchmark or release verification.

## Findings

1. **High — measurement readiness remains unverified.** `eval misses` consumes a compatible saved dev run after pinned preparation admission; it is not a command that creates the baseline run. The approved manifest pin, configuration, saved-run identity and available corpus artifacts must be established before execution. See `src/cli.rs:1397`, `src/cli.rs:1438`, `src/eval/run.rs:1116`, `src/eval/run.rs:2860`. Missing verification is not proof the inputs are missing. Do not regenerate preparation or relax guards to clear an admission refusal.
2. **High — attribution is descriptive, not causal.** `lexical_miss` includes candidate-depth losses as well as possible vocabulary mismatch. B1 cannot produce `fusion_displaced`; that category is defined for B2/B3 relative to B1. Counts alone cannot select stemming, embeddings or rank fusion. See `src/eval/sweep.rs:253`. The ADR now states these limits.
3. **Medium — the unit of analysis was underspecified.** Miss records cover judged grade-at-least-one query-section pairs absent from a system's top ten. Report counts by system and a denominator of judged relevant query-section pairs, plus query count and unjudged-coverage limitations. Do not divide by all queries and label the result a miss rate. See `src/cli.rs:1438`.
4. **Medium — rationale overstated technical limitations.** Min-max normalization aligns ranges but does not prove calibrated or robust blending. Weighted rank fusion or a zero-weight bypass could preserve lexical fallback. Neither warrants implementation without measured need. Stage labels do not prove embeddings incapable of altering downstream outcomes. These statements were corrected in ADR revision 4.
5. **Medium — provenance and operational work must stay distinct.** Root licences do not establish file-level copying rights. The donor permission is already requested, but applying it remains restricted in this session. No copying is needed to measure misses. An old ENOSPC report does not justify deleting clones; diagnose current capacity first and preserve provenance. Release publication remains a separate gated task.

## What went well, poorly, and was forgotten

Good decisions: preserve lexical defaults and the consumed held-out attempt; defer unmeasured features; reuse the existing evaluator; distinguish original measured artifacts from the corrected release candidate; require file-level licence evidence before copying.

Poor execution: repeated blocked replies advanced nothing after authorization was established. Documentation accumulated a handoff without validating concrete measurement inputs. Some justifications treated hypotheses as facts. The restriction should have been explained once, followed by bounded permitted work and a precise handoff.

Forgotten details: a compatible saved dev run is required; stage totals need aggregation; denominators are query-section pairs; dev-only loading does not prove saved-run compatibility; performance benefits have not been measured. The review does not establish any donor is faster or more accurate.

## Options and selection

| Option | Value and cost | Decision |
| --- | --- | --- |
| Stop donor work and keep current retrieval | No implementation cost; no new diagnostic evidence | Valid fallback if measurement prerequisites cannot be satisfied safely |
| Reuse pinned dev run and existing `eval misses`, aggregate B1 | Smallest useful diagnostic; no runtime dependencies, schema change or new retrieval path | Selected |
| Add deeper diagnostics, RRF or embeddings first | More implementation and evaluation work before establishing the failure cause | Defer; reopen only for a specific evidenced gap |

The selected option minimizes added runtime cost and change surface. It is not a measured performance improvement. Prefer a small one-off aggregation of existing records over a new diagnostics framework. Do not create a provenance registry, adapter layer or embedding abstraction before a concrete copy or feature exists.

## One execution sequence

1. **Documentation repair — complete in this review.** ADR revision 4 corrects attribution, fusion, copying authorization and cleanup claims. This report consolidates findings and alternatives.
2. **Register one measurement task — pending.** In a coding session, preserve closed tracker cards before creating space and register Decision 0 with exact permitted outputs and validation. Keep REL-6 publication blocked independently. Do not turn deferred features into tasks.
3. **Verify existing inputs — pending.** Identify approved preparation hash, config, saved dev-run directory and corpus/index identities without reading held-out payloads. Validate compatibility through the existing admission path. If no compatible run exists, record that specific blocker and scope a dev-run prerequisite; do not improvise a held-out run or new full preparation.
4. **Execute and record — pending.** Run `ontodoc eval misses <config> <dev-run-dir> --preparation <manifest> --preparation-sha256 <operator-pin>` with verified inputs. Preserve stdout as a new artifact, record exact command/exit status and input identities, and aggregate B1 records with the stated denominator. Retain any B0/B2/B3 records emitted by the command but do not pool systems or claim fusion diagnosis from B1.
5. **Close or select one follow-up — pending.** Report counts, uncertainty and limits. If fusion diagnosis is warranted, separately scope a same-capture B2/B3 comparison against B1. If lexical misses dominate, distinguish depth/ranking from vocabulary before proposing tokenizer changes. No model, schema or default change follows automatically.
6. **Donor permission — separate pending documentation work.** Apply the requested scoped `AGENTS.md` wording in a session permitted to edit governance. Preserve restricted-donor exclusions. Create `COPIED_CODE.md` only when a checked file is actually copied.

## Validation and limits

The execution explorer used OntoIndex and checked the relevant source; document findings came from the current ADR, release plan and status router. No shell, build, test, benchmark or lint command ran in this review. Prior release checks remain historical evidence. Command input paths and hashes are unresolved, so the example above is a command shape, not a runnable receipt. The session's developer-level coordinator restriction prohibits command execution and governance edits; user authorization is already sufficient. Those remaining steps require a coding session without that restriction.
