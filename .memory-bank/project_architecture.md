---
name: OntoDoc Architecture
type: project
---

# OntoDoc Architecture

Authority: [ADR_ONTODOC_SERVICE.md](ADR_ONTODOC_SERVICE.md). Update this file
when an owner, flow, or change-home rule changes. All `src/` owners below
exist (OD-0.0..OD-2.1, EVAL-1.0..EVAL-1.2 done); `eval/` holds only
`corpora.json` so far.

## Flow

`corpus walk → parse (sections) → chunk → tiers → graph → publish generation → load → search (B1/B2/B3, abstention) → assemble (budget) → MCP tools`
`eval (ADR9 target): final-query approval → split → approved grid-wide pools → human qrels/provenance → trusted preparation handoff → selected dev sweep + power → trusted config recapture → freeze (config + code) → one consumed held-out attempt → accepted decisions → separate default promotion/no-op`

[README](../README.md) documents root-package installation, stdio client setup,
five tools, schema-1 reindexing and current safety/evaluation limits. It does not
authorize HTTP delivery or claim a measured retrieval/performance result.

## Accepted Folder-Scoped HTTP Design (Not Implemented)

Implemented 2026-11-11 (heading kept for links): HTTP-1 registry and the
HTTP-2 loopback listener under ADR revision 10 (owner decision: loopback
behind an authenticated TLS ingress). The ADR revision 8 local contracts below
apply to both transports. No new tool, authentication stack or non-loopback
bind is authorized.

ADR revision 4 maps every immediate real subdirectory of `./docs_share/` to a
separate logical MCP service, named after that folder:

| Corpus | Endpoint | Own database/index store |
| --- | --- | --- |
| `docs_share/<name>/` | `/<name>/mcp` | `docs_share/<name>/.ontodoc/` |

One Rust listener routes to one `OntoDocServer`/`App` per folder. Each owns its
corpus root, store, immutable index slot and MCP sessions; clients of that
endpoint share its index. All users may read all exposed corpora. This is
per-corpus separation, not user tenancy or a merged index with path filtering.

Folder discovery is sorted and startup-only; nested folders stay in their
parent corpus. Preserve names with URL-segment encoding; reject ambiguous
routes, traversal and symlinks. Unknown/unindexed endpoints never fall back to
another corpus. Generations, graph traversal, ranking statistics, citations,
verification and reloads remain endpoint-scoped. Registry/cache keys include
corpus identity because generation/section IDs can coincide across stores.

Extend existing owners: `src/mcp.rs` owns endpoint/session routing and per-corpus
server reuse; `src/cli.rs` owns configured roots and trusted per-folder indexing;
`src/store.rs` owns each independent `.ontodoc/` database and atomic publication.
Keep the five tools, stdio support and retrieval algorithms. HTTP-2
(2026-11-11, ADR revision 10) adds `src/http.rs`: loopback-only hyper http1
accept loop → `Router` (connection and in-flight caps, Origin 403, exact
encoded-route lookup, POST only, 128 KiB streaming body, 30 s read deadline)
→ rmcp stateless JSON `StreamableHttpService` per corpus (Host allowlist) in a
tracked task that owns its slot → complete-response bound and generic
non-2xx bodies. Each route reuses the registry's `OntoDocServer`; tool work
takes 4 global then 2 per-corpus permits inside `OntoDocServer::blocking`.
HTTP-1 (2026-11-11) implements `mcp::Registry::discover`: immediate real
subdirectories, sorted, at most 32, URL-unreserved names, symlinks and other
special entries refused, hidden entries and plain files ignored. It holds one
`OntoDocServer` per folder, with an exact lookup and no default; `src/http.rs`
routes to those servers. Detailed lifecycle and acceptance requirements live
in the ADR's **Folder-Scoped HTTP MCP Architecture** section.

## Owners

| Path | Role | Task |
| --- | --- | --- |
| `Cargo.toml` | single package `ontodoc` (lib + bin) | OD-0.0 |
| `src/corpus.rs` | retained-root openat2 reader, fixed-buffer RawDir traversal, shared accepted-file visitor/collecting wrapper, explicit ignore and residual work budgets; automatic walker is test-only | OD-1.0, EVAL-1.4, OD-3.4/3.6 |
| `src/parse.rs`, `src/chunk.rs` | pulldown-cmark sections (evaluation unit, budget-independent ids), chunks (section id + ordinal), spans, hashes; `MARKDOWN_OPTIONS` shared with tiers and graph | OD-1.1, FIX-12 |
| `src/tiers.rs` | structural L0/L1 tiers and document pseudo-documents, copied from source; table-aware first sentence (schema 3) | OD-1.2, FIX-12 |
| `src/graph.rs` | CONTAINS / NEXT / LINKS_TO (projected to first chunks) / MENTIONS edges | OD-1.3 |
| `src/store.rs` | generations, lock, pointer, shared artifact/captured-byte validation with raw collection preflight; streaming sorted freshness manifest; prune moves to `tmp-<id>` before deletion | OD-1.4, EVAL-1.4, OD-3.6, FIX-11 |
| `src/search/{tokenize,baseline,bm25}.rs` | tokenizer, B0, B1, pre-fusion abstention on normalised raw BM25 | OD-1.5 |
| `src/search/{ppr,fusion}.rs` | PPR over LINKS_TO+NEXT (B2), doc-score blend (B3), scope, explain | OD-1.6 |
| `src/search/mod.rs` | checked aggregate tokenizer-input admission and shared first-text/last-hierarchy document map; trusted compatibility constructor retained | OD-1.6, OD-3.6 |
| `src/assemble.rs` | breadth-first budgeted context assembly (L0 → L1 → L2), no quality claim | OD-1.8 |
| `src/cite.rs` | exact active-generation chunk/section membership before rooted current-byte verification (valid / moved / stale / missing) | OD-1.7, OD-3.4 |
| `src/cli.rs`, `src/main.rs` | application layer (`App`, bounded responses, `AppError::kind()`) shared by CLI and MCP; clap subcommands index/search/read/related/verify/status/serve/serve-http (runtime in `main.rs`) and `eval split\|pool\|run\|score\|check-benchmark\|sweep\|freeze\|check-frozen\|misses` | OD-2.0, EVAL-1.1, EVAL-1.2 |
| `src/mcp.rs` | rmcp stdio server, five read-only tools, native input/output bounds, preconversion admission, closure-owned bounded permits (optional shared global, then per-corpus) over a snapshot cache, `docs_share` folder registry | OD-2.1, OD-3.3, OD-3.5, HTTP-1, PERF-1, HTTP-2 |
| `src/http.rs` | loopback Streamable HTTP listener: routing, admission, body/response bounds, drain | HTTP-2 |
| `tests/ranking_sanity.rs` | ADR discrimination check on planted synthetic sections (oracle vs shuffle, self p = 1, B1–B3 vs shuffle); sanity only, no quality claim | FIX-9 |
| `src/eval/{metrics,stats}.rs` | metrics, decision rule, bootstrap, randomization, Holm, Wilson, the single weighted-kappa implementation (`kappa_from_counts`), power | EVAL-1.0 |
| `src/eval/run.rs` | split/pool/run/score/reports; split-specific capture and opaque mechanical evidence, bounded payload/index bytes and shared cached indexes | EVAL-1.1, EVAL-1.4 |
| `src/eval/{gates,sweep}.rs` | integrity gates, in-memory pool comparison, captured dev computation, miss attribution, freeze/code-freeze checks; public measurement disabled | EVAL-1.2, EVAL-1.4 |
| `eval/` | corpora pins, queries, pools, qrels, frozen config, results | EVAL-2.x, EVAL-3.x |
| `ontocode-rs/` | read-only prior-attempt snapshot; never built | — |

## Change-home rules

- `Generation::validate_unique_ids` in `src/store.rs` rejects ambiguous raw
  section/chunk collections before publication writes and before loaded
  generations reach search lookup maps. Malformed generations reject rather
  than deduplicate. The implemented schema-2 repair is
  specified in [service ADR revision 5](ADR_ONTODOC_SERVICE.md#schema-2-identity-and-canonical-artifacts-accepted-not-implemented);
  OD-3.2 validation is recorded in the [implementation audit](audit_session-2026-11-10-schema2-implementation.md):
  - `src/parse.rs` encodes an empty allocated anchor as `@empty` only in section
    ID components; preamble `path#`, Markdown anchors and chunk ordinal suffixes
    are preserved. `src/graph.rs` keeps anchor resolution and projection;
    existing `App::read` exact-ID precedence needs no owner change.
  - `src/store.rs` removes persisted mtime, stores captured-byte size,
    canonicalizes/sorts skip records and includes them in generation identity.
    Verified same-ID reuse requires identical index/meta bytes, otherwise
    refusal preserves the target/pointer/history. Schema 1 requires reindexing.
  - Current-byte SHA-256 freshness remains unchanged; no mtime cache. Tiers
    remain verbatim fragments with single-LF formatting separators, not generated
    content. No diagnostic fields, provenance or ranking algorithm is added.
- `src/mcp.rs` bounds complete native-codec output (128 KiB including newline),
  replacing oversized replies with a typed cap error and unchanged request ID.
  IDs unable to fit that refusal close the connection before dispatch. Existing
  CLI response truncation is reused for text/structured parity; `src/cli.rs`
  owns shared public error redaction and escaped read bounds. See the
  [OD-3.3 compatibility decision](audit_session-2026-11-10-mcp-wire-bounds.md).
- ADR revision 7 separates local safety from HTTP exposure. `src/store.rs`
  owns OD-3.8 pointer/meta/artifact bounds and bounded publication serialization;
  shared captured decoding retains evaluation's stricter caps. Limits: CURRENT
  128 bytes, metadata 4 KiB, serialized artifact 512 MiB and configured cap.
  Final-component no-follow/nonblocking protection is not ancestor containment
  or a resident-memory guarantee. ADR8 accepts the remaining reader/membership,
  local budgets and publication outcomes below; OD-3.4 implements rooted reads
  and membership; OD-3.5/3.6 implement local budgets; OD-3.9 implements
  publication outcomes and bounded maintenance.
  HTTP-0 remains owner-gated.
- Native inbound collection has its own fixed-buffer 128-KiB line guard before
  rmcp decode, separate from complete output bounds. Borrowed JSON admission
  counts argument keys/nested strings and per-field limits before typed conversion;
  `src/cli.rs` shares citation preflight. These limits do not prove HTTP readiness.
- Implemented ADR8 reader/membership homes (OD-3.4): `src/corpus.rs` owns one concrete
  retained-root rustix openat2/RawDir reader and shared policy traversal;
  `src/store.rs` owns root-bound generation reads and caller propagation, avoiding
  root-replacement generation/document mixing. `src/cite.rs` owns shared whole
  section/chunk construction and exact membership; `App` pins one bound active
  generation and rejects old/unknown/nonmember citations before target reads.
  Originally admitted membership is not a new current-policy ACL check.
  Full-relative descriptor reads include CURRENT/meta/artifact; MCP loads the
  selected pointer once and verification shares its cached index. Unsupported
  resolution/races refuse without fallback. Root replacement, ignore policy,
  whole-section/lower-tier/truncated citations and error regressions are recorded
  in the [OD-3.4 audit](audit_session-2026-11-10-od34-rooted-membership.md).
- Implemented OD-3.6 streams current-byte freshness with ADR8 counters and allocation-light
  load preflight; `src/search/mod.rs` reuses its document map before tokenization
  with a checked input-byte ceiling. OD-3.5 guards actual stdio input before
  native line collection and admits one actual blocking operation/no queue,
  including load/rebuild/freshness/verify. Closure-owned permits survive
  waiter cancellation and runtime shutdown while work remains. PERF-1
  (2026-11-11) holds the index mutex only for the CURRENT check, reload and
  `Arc` snapshot. Tools run on the snapshot, so `with_permits(1..=64)` calls
  overlap; stdio keeps one permit. Excess calls are `busy`, and a tool panic
  no longer poisons the cache.
  `src/cite.rs` precharges whole-batch read/SHA byte/call budgets, including repeated
  citations and relocation; failures return no partial states. See the
  [OD-3.5 audit](audit_session-2026-11-10-od35-input-work-budgets.md).
  No TTL, parallel retrieval stack or memory
  reservation framework is selected; counters do not prove RSS/latency bounds.
- Implemented OD-3.9 keeps publication in `src/store.rs`: CURRENT rename is commit; explicit
  committed outcomes distinguish uncertain durability from failed maintenance.
  `src/cli.rs` adds those fields to the index report; precommit errors alone mean
  noncommit. Bounded HISTORY/fixed-shape cleanup never rolls back or deletes
  current data. Namespace/reused-candidate syncs precede commit; cleanup has a
  shared precharged probe budget and complete fixed-shape inventories. Locks
  span maintenance. Process ownership/release/death/barrier tests pass; the
  historical parallel `Locked` cause remains unknown. See the
  [publication audit](audit_session-2026-11-11-od39-publication-outcomes.md).
- EVAL-1.4 is implemented: `Bench::prepare` captures both physical
  partitions and runs mechanical gates; opaque `MechanicalEvidence` binds exact
  input/index identities. `Bench::load_dev` selects dev paths before payload
  access, retains both partition commitments and checks current freshness/pins.
  Captured qrels, split/pool/gate values and `Arc` indexes feed computation
  without mutable rereads; saved scoring parses the bytes it hash-checks.
  This is in-process mechanical evidence, not annotation provenance. EVAL-1.5
  adds fixed local custody below; EVAL-1.3 admits measurement only through a
  pinned trusted preparation (synthetic success paths proven; no real benchmark).
- ADR revision 6 assigns the sole write-once registration and started/result
  records to `src/eval/run.rs`, under Git's resolved common directory at
  `ontodoc-eval/`. Registration binds exact frozen TOML and captured identities;
  the external frozen file is a byte-matching export. Completion binds saved
  metadata/report bytes; metadata binds each ranking/abstention file parsed from
  its captured buffer. Linked worktrees share custody. This is a trusted local
  CLI boundary, not immutable external custody or annotation provenance.
  External preregistration/outcome audits are mirrored into memory only after
  measurement to preserve the clean frozen commit. No reset or replacement
  registration is supported; interruptions and failures consume the attempt.
- `src/store.rs` owns the shared schema/checksum/identity decoder for already
  captured index bytes and the freshness comparison. `src/corpus.rs` owns the
  shared bounded adapter: charge before rooted RawDir enumeration, capture
  ignore bytes explicitly and use public dependency matchers. Custom controls
  across depths precede git controls; hidden filtering respects whitelists.
  Depth, line, rule-input, matching and path-content counters bound work as well
  as Markdown/control read reservations. Ordinary automatic traversal remains
  the successful readable-control compatibility oracle; unsafe/unreadable
  controls refuse capture, never silently weaken freshness.
  See [capture/traversal closure](audit_session-2026-11-10-eval14-bounded-traversal.md).
  Final-component no-follow/nonblocking eval payload opens do not establish
  descriptor-rooted corpus containment or HTTP work admission. No external
  registry service is an accepted requirement.
- The ordinary automatic walker is a test-only readable-policy oracle. Production
  corpus traversal uses the retained-root reader and refuses unsafe controls.
  OD-3.6 shares that traversal between collecting builds and streaming freshness;
  it does not reintroduce automatic control loading or claim HTTP readiness.

- Accepted ADR9 evaluation homes (implemented, synthetic evidence only): extend existing
  `src/eval/{run,gates,sweep}.rs` and `src/cli.rs` for query-review and human
  annotation companions plus bounded `preparation.json`. Trusted full preparation
  validates approval before pool ranking, raw human agreement, full-data gates,
  exact-byte identities, both partitions and derived counts. Local JSON/hash
  matching does not authenticate reviewers, humans or chronology; the operator
  approves exact hashes in an external audit. No signature or registry service.
  Selected dev execution must not open/stat/hash combined/test/full pool or
  provenance payloads; only approved metadata counts/commitments and selected
  bytes. Implemented by EVAL-1.3: measurement CLI commands take
  `--preparation`/`--preparation-sha256`; `Bench::load_selected` binds config/
  splits/gates hashes before partition reads and never opens the pool;
  `check_layout` refuses payloads inside corpus checkouts; `held_out_admission`
  (registration, export, manifest, power, code, freshness) precedes reservation;
  `score_held_out` requires completed custody.
  Frozen/registration/run metadata bind whole preparation and selected identities
  separately. Confirmation/preflight reserves the sole attempt before test reads;
  accepted reporting/scoring requires completed custody and published byte parity.
- Default promotion is separate from measurement: EVAL-3.2 records the frozen
  `h5-then-h2-v1` recommendation; OD-4.1 applies H5/frozen beta, else H2/frozen
  alpha, else no-op. `src/cli.rs` owns mode/weight omission for clap and serde;
  MCP consumes that request, so only CLI production defaults plus `tests/cli.rs`
  and `tests/mcp.rs` are in scope. Explicit requests remain supported and omitted
  threshold stays disabled; H3 is descriptive. Never load mutable evaluation
  artifacts on retrieval or change defaults during the frozen run.
- Ranking algorithms live in `src/search/`, and change only with dev evidence.
  Promotion needs the approved preregistered held-out result; a different policy
  or new ranking arm requires new preregistration, not post-hoc result selection.
- New MCP tools need an ADR amendment. Write or index tools are not exposed over MCP.
- On-disk artifact schema changes bump the schema version and invalidate old
  generations; they don't migrate silently.
