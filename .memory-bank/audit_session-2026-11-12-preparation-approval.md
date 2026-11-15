# Preparation approval closeout

The owner's 2026-11-12 reply "1 2 3" accepts item 1 of the preceding next-step proposal: approval of `eval/preparation.json`, SHA-256 `1e4835cd994a799b136fcf7c095567e43ae9bbaafd800a6bdbc51d69b1cf5125`. The decision is recorded in `eval/audit_session-preparation.md`. It supplies no copyright identity or snapshot disposition.

An inline Python check using `hashlib.sha256` and `json` exited 0: the exact approved manifest, nine input hashes and four dev/test partition hashes match. Payloads were hashed without inspecting their labels; aggregate manifest metadata confirms qrels stage, model-judged provenance, kappa above 0.6 and 165 positive answerable test queries. No payload, manifest, corpus or custody file changed.

The qrels-stage command and judging evidence in the preparation audit are historical. The required create-new preparation command was not repeated against its existing pinned output. No build, Rust test, benchmark, freeze or held-out attempt ran in this approval closeout. Current checks establish byte continuity, not new judgement provenance or a current-tree runtime result.

`git diff --check` exited 0 and `project_plan_validate` returned PASS before closure. The strict `complete-local` transition completed EVAL-2.1 at revision `87bb2e08e34a9487a62bda9cd703c4b58b434e5e7a2eaf6415d438d530211b45`. The manager then returned `no_dispatch`, blocked on REL-1. Counts: 57 DONE, 9 OPEN, 66 total.

Recovery: one `reopen` request was rejected because the waiting manager already had an active task. No lease or receipt existed; the cleared owner gate was recorded and `manager_loop_next` authorized local execution. No repeated failed request, rejected receipt or running job remains.

Next inputs: copyright holder/year and snapshot disposition consistent with the ADR. The host binary target must be confirmed before REL-5. Candidate validation and capacity precede dev evaluation; a second manifest pin follows selected configuration. All model-judged results remain exploratory.
