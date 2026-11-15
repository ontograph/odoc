# Dev sweep and preregistration, 2026-11-13

EVAL-3.0 completed with two byte-identical dev runs. This session runs the approved dev grid using the original owner-pinned preparation. All paths and commitments are bound by `export-evidence.json`; candidate `cc116c44a4d7f854bc19c6a4b40d75098a437268` and binary SHA-256 `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55` remain unchanged. Preparation SHA-256 is `1e4835cd994a799b136fcf7c095567e43ae9bbaafd800a6bdbc51d69b1cf5125`.

Protocol before execution: canonical `eval sweep ... --split dev --out results/dev-sweep/sweep.json` from the clean candidate. Compare `power.required_answerable_test` against the preparation-bound 165. If greater, stop without selecting configuration, recapturing preparation or freezing. Preserve the result and request the owner's disposition. If the comparison passes, select dev parameters, recapture trusted preparation after relocation, and obtain a second owner pin before registration. No held-out execution is authorized by a passing sweep alone.

## Sweep and power outcome

Threshold-selection rule, fixed before calculation: use only answerable dev IDs (non-off-topic with a positive section qrel) and the existing lexical raw normalized scores in `results/dev-proof/abstention.tsv`. Validate 82 answerable IDs, complete unique score coverage and finite nonnegative scores. Sort ascending and select index floor(n/10), using strict score < threshold as the scorer does. This gives at most 8/82 false abstentions; ties remain included. Require a positive threshold so live no-lexical-match handling agrees. Record the cutoffs and hashes; select only alpha, beta and threshold in external config before trusted recapture. No test labels or scores participate in selection.

`eval sweep` exited 0; canonical Python power comparison exited 0 (150 <= 165). Sweep SHA-256 `d6f7e8524b32a880943698ff14571f27ad9ce6b23f722f0a2db0f71451b6ac17`. Selected alpha 0.5, beta 0.3, 82 answerable dev queries; per-arm required n: H2 49, H5 81, protocol floor 150. All 12 grid rows are retained. These dev selection results do not establish held-out improvement. Threshold selection and trusted recapture remain pending; no freeze or held-out command ran.

## Selected configuration

{
  "alpha": 0.5,
  "beta": 0.3,
  "threshold": 0.32431970510486274,
  "answerable": 82,
  "allowed_false": 8,
  "actual_false": 8,
  "false_rate": 0.0975609756097561,
  "cutoffs": [
    0.32174964195686256,
    0.32431970510486274
  ],
  "scores_sha256": "4aebeb712c08f377c5fa026616ebdd84b9b4c7b7d9698fa00207be17e32e77e9",
  "selected_config_sha256": "6cad185cf079e5f26f995d836c7872497db5c20fb8bb09945b28779c3a77cc80"
}

Trusted recapture is manager-executed under the owner-authorized EVAL-3.1 protocol. It mechanically checks full benchmark integrity and unchanged commitments; no held-out results are inspected. Operator owner identifies the decision owner; the new manifest is pending their explicit pin, and is not yet approved for registration.

## Recapture outcome and owner gate

Canonical qrels-stage `eval check-benchmark` with both operator arguments exited 0. Selected preparation SHA-256 `0d4ef0952e453c5b6e3222723815d68a9b224da32c6fe6b5564a57481f8c8c61`. All prepared input hashes except config are unchanged; corpus/index metadata, partition commitments, counts and policy exactly equal the approved manifest. Verification exit 0; evidence `results/dev-sweep/recapture-validation.json`. Candidate remains clean and binary unchanged. No registration, frozen export or held-out attempt exists.

Pending owner approval of this exact selected preparation hash. EVAL-3.1 remains open; freeze and check-frozen are intentionally not run until that decision. Threshold 0.32431970510486274 yields 8/82 (9.7561%) dev false abstentions. Model-judged results remain exploratory and cannot promote defaults.

## Owner approval and freeze admission

The owner's 2026-11-13 reply "proceed tasks" to the explicit selected-manifest approval request approves SHA-256 `0d4ef0952e453c5b6e3222723815d68a9b224da32c6fe6b5564a57481f8c8c61`. Fresh checks verified this exact manifest, bound selected config, retained binary, clean candidate commit and power (150 required, 165 available). Frozen export, registration, started-attempt and test output paths were absent. Earlier sweep/recapture results are retained evidence, not rerun in this turn.

Freeze execution uses the retained candidate as working directory, external benchmark `/opt/d1/_wrk/ontodoc/tmp/release-010/benchmark/eval`, threshold `0.32431970510486274`, and the newly approved preparation pin. Git-common custody is `/opt/d1/_wrk/ontodoc/.git/ontodoc-eval/`. Preserve every created custody file even on failure. Run freeze, then check-frozen, record exact output/registration hashes before the sole test command.

## Freeze result

Canonical `eval freeze` and `eval check-frozen` both exited 0. Exact registration/export binding and code/lockfile/binary/pin checks passed. Evidence in `results/freeze/validation.json`:

```json
{
  "freeze_exit": 0,
  "check_frozen_exit": 0,
  "registration_sha256": "fafd43de53e61f2b9b1b9055a3736b8148002752864fcbc837e9742c14e07572",
  "frozen_export_sha256": "fcb05142d66537d9bbc8874c1327257513d67855477926dd88cafa7d9c645d74",
  "preparation_sha256": "0d4ef0952e453c5b6e3222723815d68a9b224da32c6fe6b5564a57481f8c8c61",
  "git_commit": "cc116c44a4d7f854bc19c6a4b40d75098a437268",
  "binary_sha256": "af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55",
  "cargo_lock_sha256": "2f5a28ae120bdabe661506d4766d7884fc0ba84e8922c08951cfb15c9fe6e7ac",
  "registration_embeds_exact_export": true
}
```

The fixed h5-then-h2-v1 policy is preparation-bound; model-judged labels authorize exploratory effects only and no default promotion. No held-out attempt existed at this closeout.
