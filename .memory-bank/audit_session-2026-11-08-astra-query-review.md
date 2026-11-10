---
name: Astra query-review amendment
type: audit
---

# Astra Query-Review Amendment

Date: 2026-11-08. User instruction: "use gpt-6-astra as reviewer continue",
after being told EVAL-2.0 needed a human query author or reviewer.

The user chose GPT-6-Astra for query review. The ADR and EVAL-2.0 now allow
model-authored, Astra-reviewed queries with per-query provenance, acceptance
evidence and final-text binding before pooling. This is a prospective change;
no benchmark queries or results existed at the time of amendment. No query has
yet been reviewed or accepted. Do not represent model review as human review.
Model training exposure, correlated author/reviewer errors and uncertain
representativeness limit external validity.

GPT-6-Astra independently reviewed the proposed exception and the existing
protocol. The exception applies only to query review. EVAL-2.1 still requires
two human annotators, blinded judgments and the recorded agreement threshold.

## Source-level integrity findings

- `src/cli.rs::run_eval` dispatches `EvalCommand::Run` to `eval::run::run`
  without calling `check_benchmark`; `src/eval/run.rs::run` checks explicit
  test-split confirmation but does not check the frozen code/config identity.
- `src/eval/gates.rs::check_qrels` accepts an existing judged section with no
  `span_sha256`. Floor, corpus-pin and pool-reproducibility checks can be
  omitted by the gate configuration.
- The standalone `check_frozen` command does not enforce the frozen identity
  on the run path. No empirical run was performed to reproduce these findings.

The ADR requires integrity checks before runs, matching span hashes and refusal
of a dirty or mismatched held-out run. `AGENTS.md` says to stop and report an
ADR/code conflict. EVAL-2.0 remains blocked pending resolution and validation;
no measured run, tuning or held-out evaluation was performed. EVAL-1.2's prior
closeout is historical evidence and does not establish these enforcement paths.

The plan validator passed after the amendment (21 tasks); strict manager
decision: `no_dispatch`, status blocked. No tests or benchmarks ran this turn.
