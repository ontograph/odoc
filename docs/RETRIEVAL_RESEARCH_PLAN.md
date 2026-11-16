# Retrieval Research Plan

Date: 2026-11-16
Status: Closed. Revision 3 (2026-11-16) consolidated the plan after three GPT design reviews and ran it: step 1 met no registered threshold, so steps 2 and 3 closed as not opened.
Authority: proposed [retrieval research ADR](../.memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md) revision 4 and its [Registration](../.memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md#registration), the [service ADR](../.memory-bank/ADR_ONTODOC_SERVICE.md) and `AGENTS.md`. The [hybrid retrieval ADR](../.memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md) and [plan](HYBRID_RETRIEVAL_PLAN.md) keep ownership of step 4; the [agentic retrieval ADR](../.memory-bank/ADR_ONTODOC_AGENTIC_RETRIEVAL.md) keeps ownership of step 5.
Requested by: user, "from proposed maek implementation plan md file", 2026-11-16. Revision 2 followed "agree" (add a depth-miss step). Revision 3 follows "propose several solutions to do next tasks and fix issues, cherry pick most robust and performance proposals, consolidate it to one plan and proceed it".

## Goal

Run the ADR's registered step 1 on the existing B1 dev misses, then run only the steps it routes to, each as a dev-only diagnostic that changes no ranking code, store, tokenizer, default or evaluation record. Every result is exploratory under model-judged labels.

## Manager Tracking

```yaml
manager_loop:
  authority: true
  status: closed
  selection_policy: active_next_task-first
  active_next_task: null
  max_in_flight: 1
  dispatch_mode: none
  reopen_gate: null
tasks:
  RR-0:
    title: The ADR registers the step 1 rules, parity gates, routing thresholds and the step 2 arm formulas, with recorded answers to decisions 1 to 8, before any step 1 script exists
    status: DONE
    classification: architecture-paperwork
    type: design-gate
    assurance_profile: full
    complexity: standard
    impact_risk: low
    validation_tier: unit-fast
    depends_on: []
    owner_files:
    - .memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md
    allowed_write_set:
    - .memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md
    non_goals:
    - No script, measurement or look at miss tokens, pairs or per-query content.
    required_validation:
    - git diff --check
    - rg -n '^## Registration|^### Routing|^### Step 2 Arm' .memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md
    expected_evidence:
    - ADR revision 4 holds the answers table, prior-exposure statement, parity gates, content-term list, surface-form endings, depth descriptors, routing thresholds with the tie rule, and the three step 2 formulas, dated 2026-11-16, and scripts/research/ does not yet exist.
    rollback:
    - Revert the revision 4 hunks of the ADR.
    stop_conditions:
    - A threshold depends on any step 1 output.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: ADR revision 4 written before scripts/research/ existed; git diff --check exit 0; registration headings present.
      role_results:
        coder-manager: null
    dod_status: pass
  RR-1:
    title: Step 1 passes every parity gate, then assigns every B1 lexical_miss and truncated pair to its registered group with descriptors, shares, intervals and the route
    status: DONE
    classification: proof-only
    type: measurement
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RR-0
    owner_files:
    - scripts/research/rr_common.py
    - scripts/research/rr_step1.py
    - scripts/research/charprops.rs
    - tmp/retrieval-research/step1.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    allowed_write_set:
    - scripts/research/rr_common.py
    - scripts/research/rr_step1.py
    - scripts/research/charprops.rs
    - tmp/retrieval-research/step1.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    non_goals:
    - No ranking arm, Rust crate change, store write, model, label or dependency.
    - No held-out file is opened, including eval/test-*, eval/queries.jsonl, eval/qrels.jsonl, eval/pools and the held-out audit.
    - No rule or threshold edit after the run.
    required_validation:
    - python3 scripts/research/rr_step1.py --self-test
    - python3 scripts/research/rr_step1.py --out tmp/retrieval-research/step1.json
    - git diff --check
    expected_evidence:
    - The self-test checks tokenizer splitting with Rust character properties, BM25 against the hand-computed values in src/search/bm25.rs, the surface-form endings and exclusions, the three lexical groups, the descriptors, the ceiling, SplitMix64 and the routing tie rule on synthetic fixtures.
    - step1.json binds every input hash, the binary sha256 and both generation IDs, records each parity gate as passed, and gives group counts that sum to the 21 lexical_miss and 78 truncated pairs, shares with query-cluster and Wilson intervals, the truncated-only ceiling and the registered route.
    rollback:
    - Keep the scripts and audit; delete tmp/retrieval-research/step1.json.
    stop_conditions:
    - A parity gate fails; record which gate and stop.
    - A held-out file would be read.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: 'self-test exit 0; two runs exit 0, byte-identical step1.json sha256 33a67b6b; all 8 gates pass; lexical 20/0/1 of 21; best depth share H 0.333; route none; git diff --check exit 0.'
      role_results:
        coder-performance: null
        coder-manager: null
    dod_status: pass
  RR-2:
    title: Step 2 resolves from step 1's route as either not opened, or a B1R diagnostic of the registered candidate whose script is written under a plan revision
    status: DONE
    classification: proof-only
    type: measurement
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RR-1
    owner_files:
    - scripts/research/rr_resolve.py
    - tmp/retrieval-research/step2.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    allowed_write_set:
    - scripts/research/rr_resolve.py
    - tmp/retrieval-research/step2.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    non_goals:
    - No Rust change, System variant, CLI surface, pool extension, new judgment or default change.
    - Only the candidate step 1 named; no second candidate or constant change after the run.
    required_validation:
    - python3 scripts/research/rr_resolve.py --self-test
    - python3 scripts/research/rr_resolve.py --step 2 --out tmp/retrieval-research/step2.json
    - git diff --check
    expected_evidence:
    - Not-opened branch, when the step 1 route names no candidate. step2.json records opened false, bound to step 1's sha256, and the audit says so.
    - Opened branch, when the route names a candidate. The resolver refuses with exit 2, and a plan revision adds the diagnostic script with the registered statistics before the task can close.
    rollback:
    - Delete tmp/retrieval-research/step2.json.
    stop_conditions:
    - Step 1 gates did not all pass.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: 'Not-opened branch: resolver self-test exit 0; --step 2 exit 0, step2.json sha256 4880af2e, opened false; git diff --check exit 0.'
      role_results:
        coder-performance: null
        coder-manager: null
    dod_status: pass
  RR-3:
    title: Step 3 resolves from step 1's route as either not opened, or a surface-form diagnostic whose script is written under a plan revision
    status: DONE
    classification: proof-only
    type: measurement
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RR-2
    owner_files:
    - tmp/retrieval-research/step3.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    allowed_write_set:
    - tmp/retrieval-research/step3.json
    - .memory-bank/audit_session-2026-11-16-retrieval-research.md
    non_goals:
    - No tokenizer, TOKENIZER_VERSION, store, dependency or reindex change; a promotion needs ADR decision 6 and its own task.
    required_validation:
    - python3 scripts/research/rr_resolve.py --step 3 --out tmp/retrieval-research/step3.json
    - git diff --check
    expected_evidence:
    - Not-opened branch, when the surface-form share is below 0.25. step3.json records opened false, bound to step 1's sha256.
    - Opened branch. The resolver refuses with exit 2, and a plan revision adds the folding diagnostic with the identifier and version guardrail before the task can close.
    rollback:
    - Delete tmp/retrieval-research/step3.json.
    stop_conditions:
    - RR-2 is not DONE, so each arm is compared with B1 alone.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: 'Not-opened branch: --step 3 exit 0, step3.json sha256 3364a805, opened false; git diff --check exit 0.'
      role_results:
        coder-performance: null
        coder-manager: null
    dod_status: pass
  RR-HY:
    title: The hybrid plan records step 1's no-shared-term share, and the hybrid ADR and plan withdraw the cross-language stratum per decision 2
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RR-1
    owner_files:
    - docs/HYBRID_RETRIEVAL_PLAN.md
    - .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md
    allowed_write_set:
    - docs/HYBRID_RETRIEVAL_PLAN.md
    - .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md
    non_goals:
    - No change to HY task status or to the hybrid ADR's other decisions.
    required_validation:
    - git diff --check
    - rg -n 'cross-language' .memory-bank/ADR_ONTODOC_HYBRID_RETRIEVAL.md docs/HYBRID_RETRIEVAL_PLAN.md
    expected_evidence:
    - The hybrid plan Closeout cites the audit and the no-shared-term share; every remaining cross-language mention is the stage-0 history or a dated note citing decision 2.
    rollback:
    - Revert only this task's hunks.
    stop_conditions:
    - RR-1 has no recorded result.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: Hybrid ADR revision 3 and plan Closeout updated; project_plan_validate on the hybrid plan pass; git diff --check exit 0.
      role_results:
        coder-manager: null
    dod_status: pass
  RR-DOC:
    title: MEMORY.md links the research ADR, this plan and the audit within 100 lines and 4,096 bytes, and the plan table shows final counts
    status: DONE
    classification: docs/design-only
    type: documentation
    assurance_profile: full
    complexity: complex
    impact_risk: low
    validation_tier: unit-fast
    depends_on:
    - RR-3
    - RR-HY
    owner_files:
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/archive/MEMORY_ARCHIVE.md
    allowed_write_set:
    - .memory-bank/MEMORY.md
    - .memory-bank/project_plan-current.md
    - .memory-bank/archive/MEMORY_ARCHIVE.md
    non_goals:
    - No change to plan or ADR content.
    required_validation:
    - git diff --check
    - wc -lc .memory-bank/MEMORY.md
    expected_evidence:
    - MEMORY.md is at most 100 lines and 4,096 bytes; any line moved out appears verbatim in archive/MEMORY_ARCHIVE.md.
    rollback:
    - Revert only this task's hunks.
    stop_conditions:
    - Byte headroom cannot be made without dropping an active entry.
    known_blockers: []
    closeout:
      main_session_evidence:
        authorized_by: user
        authorization_ref: consolidate it to one plan and proceed it, 2026-11-16
        validation_summary: 'MEMORY.md 44 lines, 3,839 bytes; three closed review lines moved verbatim to archive/MEMORY_ARCHIVE.md with ../ link prefixes; plan table row closed; git diff --check exit 0.'
      role_results:
        coder-manager: null
    dod_status: pass
```

## Required Traceability

| Requirement ID | ADR/user requirement | Disposition | Task IDs | Validation |
| --- | --- | --- | --- | --- |
| RR-R1 | Registration of rules, gates and thresholds before step 1 runs | architecture-gate | RR-0 | ADR revision 4 |
| RR-R2 | Step 1 parity with `B1.tsv`, `misses.json` and the binary's top 50 before classification | proof-only | RR-1 | `step1.json` gate records |
| RR-R3 | Step 1 lexical groups, truncated groups, rank bands, descriptors and ceiling | proof-only | RR-1 | `rr_step1.py --self-test`, `step1.json` |
| RR-R4 | Routing by registered thresholds; no threshold met stops steps 2 to 4 | proof-only | RR-1 | Audit route statement |
| RR-R5 | Step 2 B1R diagnostic, permutation only, existing qrels, judged@10 reported | proof-only | RR-2 | `rr_step2.py --self-test`, `step2.json` |
| RR-R6 | Step 3 surface-form diagnostic with identifier and version guardrail | proof-only | RR-3 | `rr_step3.py --self-test`, `step3.json` |
| RR-R7 | Step 4 waits for RR-1's no-shared-term share plus every hybrid ADR decision | blocked | RR-HY | Hybrid plan Closeout |
| RR-R8 | Decision 2: no cross-language workload reflected in the hybrid records | implementation | RR-HY | `rg cross-language` |
| RR-R9 | Step 5 client policy owned by the agentic ADR | non-goal | RR-0 | Agentic ADR |
| RR-R10 | Promotion of any arm, a stemmer dependency, tokenizer bump, new judgments or a Rust port | non-goal | RR-0 | Needs a new ADR decision and task |
| RR-R11 | Deferred topics: entity GraphRAG, temporal graphs, vector database, model reranker, PDF ingestion | non-goal | RR-0 | ADR comparison table |
| RR-R12 | Memory router and plan table current | implementation | RR-DOC | `wc -lc` |

## Decisions And Non-Goals

Three read-only GPT reviews compared alternatives; the ADR's [Revision 4 Change](../.memory-bank/ADR_ONTODOC_RETRIEVAL_RESEARCH.md#revision-4-change) lists what was adopted and rejected. The plan keeps the choices that change no shipped code and that check their own fidelity. One shared Python module ports tokenizer version 1 and BM25 and must reproduce the pinned binary's top 50 and `B1.tsv` exactly before any number is reported; the binary supplies the rankings, and the stored generation supplies headings and chunk text. Steps 2 and 3 reuse that module, so both arms run against the same verified baseline in one process, with no reindex and no Rust build.

Each branch ends with a DONE record, so a skipped step does not block its successors. In the reproduced branch, RR-1's share meets the registered threshold and the task runs its diagnostic. In the not-reproduced branch, the share is below the threshold and the task records the route as not opened, with no script run. RR-3 depends on RR-2 so the two arms are never measured together.

Scripts live under `scripts/research/`, which Git tracks, so the method survives; outputs live under `tmp/retrieval-research/`, and the audit records their hashes and headline numbers. Every comparison uses the existing dev qrels and reports judged@10 and condensed nDCG beside nDCG@10, because sections a new arm promotes are unjudged. New judgments, a Rust port, a stemmer dependency and any default change need a later ADR decision.

## Closeout

All six tasks are DONE. ADR revision 4 registered the rules before `scripts/research/` existed. Step 1 passed every parity gate and met no threshold: 20 of 21 `lexical_miss` pairs share a content term with the query, none is a surface-form variant, one shares no content term, and the best depth descriptor favours 0.333 of remainder `truncated` pairs against the 0.50 threshold. Steps 2 and 3 were recorded as not opened. The [audit](../.memory-bank/audit_session-2026-11-16-retrieval-research.md) holds commands, hashes and limits. No Rust source, store, tokenizer, default or evaluation record changed. Any further depth hypothesis needs a new dated registration.
