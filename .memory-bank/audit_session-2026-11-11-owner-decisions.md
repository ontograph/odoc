# Audit Session 2026-11-11: Owner Decisions (Round-4 Packet)

The owner's words were "approved and continue". They replied to the
ten-question decision packet given after round 4. Every recommended answer
was approved:

1. HTTP-0 exposure: a loopback backend behind an authenticated TLS ingress
   that cannot be bypassed. Anonymous and direct public listeners stay
   unauthorized.
2. Numeric limits, recorded in ADR revision 10 as policies, not measured
   capacity:
   - 32 corpora;
   - 128-KiB request and response frames;
   - 256 concurrent requests;
   - 4 global and 2 per-corpus work permits, with reloads counted inside
     them;
   - existing load limits; a 2-MiB file cap.
3. Native clients only. Any request carrying `Origin` is refused. A browser
   allowlist needs a later ADR change.
4. No logging of queries, documents, session IDs or credentials. Internal
   errors stay generic.
5. Section IDs may be up to 4,096 bytes. Other evaluation identifiers stay at
   128. Code already used 4,096 (`gates.rs:579`); revision 10 makes the ADR
   text match.
6. Evaluation is parked until the owner supplies pinned corpora, a named
   operator and two human annotators. No synthetic smoke benchmark unless
   requested.
7. A per-result `text_truncated` search field, filled from the stored
   L0/L1 flags.
8. Empty exploratory per-corpus groups (`n = 0`) report `null`
   statistics.
9. Share the kappa implementation now. Defer the identity-check
   consolidation until after the held-out run.
10. Create the private staging directory before the held-out reservation.

## Design Refinement (Within The Approval)

Item 2 listed 256 sessions in total and 32 per corpus. To bound sessions
with the least code, HTTP-2 uses rmcp stateless JSON mode: there are no
server sessions, SSE streams or replay caches. So session hijack, reuse,
retention and replay bounds are vacuous rather than separately enforced.
The 256 figure bounds concurrent requests instead. Stateless mode drops
server-to-client notifications. OntoDoc uses none. Cancellation is
connection drop; blocking work keeps its permit until it stops.

## Resulting Tasks

- ADR revision 10 (items 1–5, 7, 8). HTTP-0 closes as accepted.
- HTTP-2: the listener.
- FIX-7: `text_truncated` field and `null` empty-group statistics.
- SIMPL-3: shared kappa.
- FIX-8: staging directory before reservation.
- EVAL-2.0: blocked, with the parked reason recorded.

## Outcomes (2026-11-11)

All closed through strict `complete-local`; synthetic data only.

| Task | Result |
| --- | --- |
| HTTP-0 | ADR revision 10 accepted (reopen + complete-local). |
| HTTP-2 | `src/http.rs` + `serve-http`; `tests/http.rs` (4) and `http::` units (4). |
| FIX-7 | `SearchResult.text_truncated`; `Comparison` statistics `null` when `n = 0`. |
| SIMPL-3 | `stats::kappa_from_counts` is the single kappa; gates duplicate removed. |
| FIX-8 | Staging is created before `Bench::load_test_reserved`. Regression: a read-only parent refuses with no attempt consumed; it failed under the old order (temporary revert). |
| EVAL-2.0 | Still BLOCKED; `known_blockers` records the owner park. |

GPT-6-Astra reviewed HTTP-2 (read-only). Each finding was verified in the
rmcp 1.7 and hyper 1.8 sources, then fixed:

- **P1: a client disconnect leaked rmcp per-request tasks.** In stateless
  mode, the oneshot transport stops only after the response is sent, so
  dropping the receiver left the service and its waiter running. Fix: each
  rmcp call runs in a `TaskTracker` task that owns the in-flight slot. The
  task always completes, and shutdown waits for it. Test:
  `dropped_request_still_finishes_and_releases_its_slot`.
- **P2: idle or slow connections were not admitted.** Fix: a connection
  semaphore of 256 closes an accepted socket at once when it is full. A
  30-second deadline covers the header read (hyper `TokioTimer`) and the
  body read (408). Test: `idle_connections_count_against_admission`.
- **P2: rmcp error bodies echoed parser and header details.** Fix: every
  non-2xx rmcp response keeps its status but gets the body
  `request refused`. Tests cover a malformed body and an unknown
  `MCP-Protocol-Version`.

Final validation, run in this session:

- `cargo test --all-targets`, exit 0:
  - lib: 210 passed, 1 ignored;
  - cli: 8;
  - eval_gates: 20;
  - eval_run: 14;
  - http: 4;
  - mcp: 12.
- `cargo fmt --check`: ok.
- `cargo clippy --all-targets -- -D warnings`: ok.
- `project_plan_validate`: pass, 52 tasks.

Limits:

- The HTTP numbers are policies, not measured capacity.
- The ingress (TLS plus authentication) is an operator delivery item. It
  is not tested here.

Refinement within the approval: item 2 set no read deadline. The 30-second
deadline is the smallest fixed bound that keeps the admitted connections
finite under slowloris, and it is recorded in the ADR revision 10 limits
table. Changing it is a configuration decision; no option exists yet.
