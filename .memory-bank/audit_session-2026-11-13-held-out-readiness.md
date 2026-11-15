# Held-out readiness decision

EVAL-3.1 is complete. The owner approved proceeding with the stated diagnostic limitation on 2026-11-13. This is a source/readiness review, not a benchmark run or a retrieval-quality result.

## Verified state

On 2026-11-13 a Python identity check exited 0: the retained binary, selected preparation, lockfile, registration and frozen export match `tmp/release-010/benchmark/eval/results/freeze/validation.json`; the registration embeds the exact export bytes; candidate HEAD is `cc116c44a4d7f854bc19c6a4b40d75098a437268` with an empty `git status --porcelain`. Neither `attempt-started.json`, `attempt-result.json` nor `results/test-v1` exists. No evaluation, build or lint command was rerun during this readiness review.

Freeze and check-frozen exit-0 evidence is retained in the [preregistration audit](audit_session-2026-11-13-preregistration.md). The strict manager completed EVAL-3.1 and selected EVAL-3.2, revision `781646931b21d5bde98a27f3d9cc913eba5bfb947c8792f1cce36ab1615d3f2c`.

## Required evidence gap

The EVAL-3.2 card requires pooled/per-corpus results and per-stage miss attribution. Explorer `freeze_packet` traced the frozen candidate's CLI and evaluator without accessing real test payloads. `src/cli.rs:1377` dispatches `EvalCommand::Misses` through dev-selected `pinned(...)`; it has no test confirmation argument. `src/eval/run.rs:2882` rejects a saved test run unless its capture is test-selected. The main session confirmed these source branches. Therefore the required held-out miss-attribution command is unavailable. The explorer's earlier absence observations for registration/export were superseded by the successful freeze and current identity checks.

The run command and later `eval score ... --confirm-test-split` are supported. This gap concerns completion of the task's required diagnostic evidence. It does not justify removing the test-access refusal, rebuilding the frozen binary, changing registration, or consuming the attempt without settling the evidence requirement.

## Proposed decision

Recommended: approve EVAL-3.2 with held-out per-stage miss attribution explicitly unavailable. Keep every existing integrity, H1, identity and once-only gate; report all supported hypotheses and per-corpus metrics with the model-judged exploratory restrictions. Amend only the task's diagnostic acceptance after approval, then write the external pre-attempt audit and run once.

Alternative: retain the full diagnostic requirement and separately authorize implementation plus a superseding experiment. Existing custody must remain intact; this is not permission to delete registration, reuse an attempt, or invent a replacement protocol.

The owner's subsequent "proceed tasks" accepts the explicit recommendation and approval question: proceed with held-out per-stage miss attribution unavailable. EVAL-3.2's diagnostic evidence requirement is amended accordingly. All integrity, identity, statistical and once-only gates remain. OD-4.1, OD-4.0 and REL-6 retain their dependency gates.
