# Review Round 5: Challenge, Consolidate, Proceed

Date: 2026-11-11. HEAD at start: `729fcb9`.
Authority: user instruction to review and challenge what was planned and done,
cherry-pick the most robust proposals, consolidate them into one plan, proceed,
and use OntoIndex and GPT sub-agents.

Three read-only GPT-6-Astra reviewers covered separate scopes: the service,
evaluation, and process/docs. OntoIndex was refreshed (job `8dc0875b`) and used
for navigation; every finding below was confirmed in source.

## Done Well

- The five-tool stdio and HTTP transports share one set of `OntoDocServer`
  handlers. Each folder has its own store. Indexing stays in the trusted CLI.
- Rmcp's JSON-RPC tool errors arrive as HTTP 200, so typed `busy` and
  `no_index` errors keep stdio parity. Only transport refusals get generic
  bodies.
- No quality claims were made. Measurement guards were kept intact.

## Worst

- Operational truth was buried in historical status prose. Stale "not
  implemented" statements remained in the ADR, the architecture file,
  plan-current and README, which still listed already-implemented safety work
  as unresolved.
- The plan sat at 93% of the parser ceiling.
- 7,200 evaluation lines have never measured anything. That is accepted while
  evaluation is parked.

## Forgotten

- The ADR discrimination check (oracle beats a shuffle, a self-comparison
  gives p = 1) had not been exercised against the real rankers.
- The README did not give working ingress `Host` guidance.

## Findings And Decisions

| # | Finding | Decision |
| --- | --- | --- |
| 1 | P2: FIX-8 moved dev staging before dev ranking. | Fixed (FIX-9): `run_reserved` takes `Option<StagingDir>`. Held-out runs stage before reservation; dev runs stage after their budget checks. |
| 2 | P3: the read-only-parent test passed silently on privileged hosts. | Fixed: it prints `SKIPPED`. A regular-file parent was tried instead and rejected: FIX-5 admission already refuses it, so it does not exercise the FIX-8 order. |
| 3 | The discrimination check was missing. | Added `tests/ranking_sanity.rs`, which uses 24 planted synthetic documents. The oracle beats a seeded shuffle (p < 0.01), the self-comparison gives p = 1, and B1, B2 and B3 each beat the shuffle. It is test-only, so it adds no CLI surface and touches no held-out path. |
| 4 | The diagnostic's first run showed that B0 returns each document's first chunk, so it never reaches a planted later section. | This is the ADR §6 baseline behaviour, kept and asserted. Section-level H1 (B1 beats B0) is therefore easy to pass on such data; this is noted for the owner, and no change was made. |
| 5 | P3: the README said "forward Host unchanged". | Rewritten: the ingress must rewrite `Host` to the listen authority. A `--public-host` option was rejected because it changes the ADR10 loopback-authority contract. |
| 6 | Every search, read and status call rehashes the whole corpus (capped at 128 MiB). | Kept: the response-level `fresh` flag requires it, and AGENTS.md forbids weakening freshness. Documented in README. A TTL or mtime cache would need an ADR change. |
| 7 | The in-flight semaphore looked redundant with the connection cap. | Kept: detached calls outlive their connections. |
| 8 | Per-request `OntoDocServer` and `ToolRouter` clones. | Deferred: unmeasured and secondary. |
| 9 | Evaluation simplification options: freeze, split `run.rs`, or feature-gate. | Freeze while parked. Splitting moves 2,000–3,000 lines with no evidence gain. |
| 10 | Plan size. | Archived the 31 DONE `validation_summary` lines verbatim, with hashes, in `audit_session-2026-11-11-done-card-validation-archive.md`. The plan went from 121,807 to 108,341 bytes; all 31 hashes round-trip. |
| 11 | Docs drift. | ADR HTTP status and Delivery Boundary, `project_architecture.md`, `project_plan-current.md` and README safety limits were corrected. Headings that serve as link anchors were kept. |

Rejected as overengineering for this round:

- Collapsing `project_pending-tasks.md` (about 7 KB saved; the memory-bank
  convention expects the file).
- A health endpoint (no ADR authority).
- A CLI `eval diagnostic` command (a test gives the same signal with no new
  surface).

## Validation (this session)

- `cargo test --all-targets`, exit 0:
  - lib: 210 passed, 1 ignored;
  - cli: 8;
  - eval_gates: 20;
  - eval_run: 14;
  - http: 4;
  - mcp: 12;
  - ranking_sanity: 1.
- `cargo fmt --check`: ok.
- `cargo clippy --all-targets -- -D warnings`: ok.
- `project_plan_validate`: pass, 53 tasks.
- FIX-9 was closed through strict `complete-local`.
- Synthetic data only.
