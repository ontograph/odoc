# OntoDoc

Standalone Rust MCP service for cited documentation retrieval. It exposes
five read-only tools over **stdio** (one configured corpus) or over a
**loopback HTTP** listener (one endpoint per `docs_share/<folder>/`). Index publication is a trusted CLI action. Retrieval uses local
lexical/graph algorithms; no model, embeddings or outbound service is required
on the retrieval path.

**Status:** experimental. Remote delivery requires operator-managed TLS authentication in front of the loopback listener; production ingress deployment is not yet validated. The registered evaluation below reports exploratory results from model-judged labels. Confirmatory retrieval-quality and multi-user performance claims have not been established.
Evaluation commands that measure, score or freeze results fail closed while
their integrity prerequisites remain unresolved.

## Install And Index

Native Linux with `openat2` support (Linux 5.6 or newer), Rust **1.95.0**, edition 2024. The root toolchain file pins rustfmt
and Clippy. From this repository root:

```sh
CARGO_BUILD_JOBS=4 cargo install --locked --path .
ontodoc --root /absolute/path/to/dedicated-docs index
ontodoc --root /absolute/path/to/dedicated-docs status
ontodoc --root /absolute/path/to/dedicated-docs search "atomic publication" --mode lexical --top-k 5
```

Use a dedicated, trusted documentation directory, not a checkout containing
credentials or unrelated private files. Markdown `.md` and `.markdown` sources
are discovered with `.gitignore` and `.ontodocignore` rules. The default file
limit is 2 MiB; oversized/non-UTF-8 files produce skip diagnostics. Inspect the
index/status output rather than assuming every file was admitted.

The corpus's `.ontodoc/` directory stores atomic generations and a CURRENT
pointer. **Schema 4 requires reindexing older stores** with the trusted
`index` command; load never silently migrates them. Generations omit source
mtimes and use canonical content/configuration/skip identity. The publisher
retains the current generation and two old ones; retention does not give clients
a historical-snapshot API.

## Connect An MCP Client

Configure the client to start the installed binary with an absolute corpus
root. For clients using an `mcpServers` JSON configuration:

```json
{
  "mcpServers": {
    "ontodoc": {
      "command": "/absolute/path/to/ontodoc",
      "args": ["--root", "/absolute/path/to/dedicated-docs", "serve"]
    }
  }
}
```

Use the client's own supported configuration format; the JSON above is not an
OntoDoc configuration file. Run `index` before starting retrieval. `serve`
communicates over stdin/stdout; stdout contains MCP frames only, not a shell
prompt or human-readable search output.

To serve several `docs_share/<name>/` folders today, index each folder and
configure one stdio server per folder; each keeps its own `.ontodoc/` store:

```json
{
  "mcpServers": {
    "docs-guide": {
      "command": "/absolute/path/to/ontodoc",
      "args": ["--root", "/absolute/path/to/docs_share/guide", "serve"]
    },
    "docs-api": {
      "command": "/absolute/path/to/ontodoc",
      "args": ["--root", "/absolute/path/to/docs_share/api", "serve"]
    }
  }
}
```

One stdio connection serves exactly one corpus; there is no cross-folder
retrieval or in-session switching.

### Automatic indexing for local stdio use

`ontodoc --root /absolute/path/to/docs serve --auto-index` lets a single local
user query a working folder without running `index` first. A tool call that
finds no index, or finds the active one stale, builds a new generation through
the same path as `ontodoc index`, waits for it, and answers from it. The
generation ID equals the one `index` would publish for the same files.

- The server writes only `<root>/.ontodoc/`, never your documents. The tools
  stay read-only with respect to the corpus; the server instructions say the
  server maintains its own index store.
- The rebuild runs inside the call. Large folders make that call slow, and the
  cost is not measured. Every rebuild is a full one. A concurrent call returns
  `busy` while it runs.
- If a rebuild fails, the call is answered from the previous generation with
  `fresh: false`, or returns the build's typed error when no generation exists.
  The server waits at least 30 seconds before trying again. Failures go to
  stderr as fixed codes.
- A corrupt or refused generation is never rebuilt or removed; use the recovery
  procedure below.
- With the flag, `verify` also hashes the corpus on every call. The other
  tools reuse the freshness check they already make.
- `serve-http` does not accept the flag. Do not use it on a pinned benchmark
  corpus or a store bound by an evaluation registration.

### HTTP (loopback, behind an authenticated ingress)

The [deployment guide](docs/DEPLOYMENT.md) provides an explicitly untested systemd/nginx example and target-host acceptance checks.

Index each folder first, then start the listener:

```sh
for d in /absolute/path/to/docs_share/*/; do ontodoc --root "$d" index; done
ontodoc serve-http --docs-share /absolute/path/to/docs_share --listen 127.0.0.1:8080
```

Each immediate folder is served at `http://127.0.0.1:8080/<percent-encoded
name>/mcp` with its own `.ontodoc/` store; endpoints are printed to stderr.
The transport is MCP Streamable HTTP in stateless JSON mode: `POST` only, one
JSON-RPC message per request, no sessions or SSE (`GET`/`DELETE` → 405).

- Folders are discovered once at startup: a new folder needs a restart. A
  folder without an index is still served; its tools return `no_index` until
  `ontodoc --root <folder> index` publishes one (no restart needed). Plain
  files are ignored; a symlink or special entry in `docs_share` refuses startup. Real `.git` and `.ontodoc` directories are skipped; other hidden directories are exposed as corpora. Folder names may contain spaces or UTF-8 and use canonical percent encoding in endpoint URLs.
- Clients that accept a URL-based MCP server configure one entry per folder,
  pointing at the ingress, for example
  `{"mcpServers": {"docs-guide": {"url": "https://docs.example/guide/mcp"}}}`
  (use the client's own format and authentication settings).

- The listener refuses non-loopback addresses. Remote users must reach it
  through an operator-run TLS ingress that authenticates every request;
  OntoDoc has no authentication. The ingress must rewrite the upstream
  `Host` header to the listen authority (for example `127.0.0.1:8080`); any
  other `Host` is refused with 403.
- Requests carrying `Origin` are refused (403): native MCP clients only.
- Limits: 32 corpora; 128 KiB request and complete response bodies (413 /
  generic 500); 256 admitted connections and 256 in-flight requests, excess
  refused (503 or closed), never queued; 30 s to send headers and body; 4
  global and 2 per-corpus blocking-work permits (`busy` when exhausted).
- Error bodies are generic; queries, documents and credentials are not logged.
- Ctrl-C/SIGTERM stops accepting and drains in-flight requests.

## Tools And CLI

Search retains citations when text does not fit: `tier: null`, empty `text` and `text_truncated: false` signal withholding by the budget or response bound. Selected text above 64 KiB is withheld in CLI and MCP. On overflow, texts are tried in rank order, continuing past those that do not fit. Only omitted whole results set `truncated: true`. Clients must accept nullable tiers. `read` may return a prefix with `truncated: true`; it has no continuation parameter.

`related.omitted` counts incident edges excluded by the item or byte cap. Add it to `neighbours.length` to obtain the incident-edge count.

| MCP tool | Main input | Behavior | Equivalent CLI |
| --- | --- | --- | --- |
| `search` | `query`, optional `mode`, `top_k`, `scope`, `explain`, `budget_bytes`, `threshold`, `alpha`, `beta` | Ranked chunks with section IDs, citations and freshness; modes `lexical`, `graph`, `hier` | `search QUERY --mode lexical --top-k 5` |
| `read` | `id` | Snapshot chunk/section by exact ID or existing `path#anchor` alias | `read 'guide.md#installation'` |
| `related` | `chunk_id` | Graph neighbours within this corpus, with citations for chunks and parent sections | `related 'guide.md#installation~0'` |
| `verify` | `citations` array | Recheck cited spans against current file bytes | `verify JSON`, or `verify -` to read JSON from stdin |
| `status` | empty object | Active generation, counts, freshness and diagnostics | `status` |

Without `budget_bytes`, retained search text is exactly its cited chunk span. With a budget, L0 and L1 contain verbatim source fragments joined by newlines: ancestor headings or enclosing-section outline text may lie outside that span. The citation still identifies the supporting chunk; it does not cover every displayed tier fragment.

`related` lists at most 100 neighbours in stored edge order: hierarchy (`CONTAINS`), reading order (`NEXT`), links (`LINKS_TO`), then unresolved code mentions (`MENTIONS`). The response byte limit can shorten the list further; omitted neighbours set `truncated: true`. A link to a section or document targets its first chunk. To find links into a section, call `related` on that section's chunk `~0`; incoming links do not appear on its later chunks.

Chunk and parent-section neighbours carry a `citation` accepted by `verify`; unresolved code mentions omit that field. Missing or wrong-kind chunk/section endpoints fail with a `corrupt` error. Verification checks endpoint bytes against the active generation and current files; it does not certify the relation itself. Read the source chunk to inspect a link. There is no paging.

Code mentions are outgoing references to unresolved `code:<text>` nodes. They do not become incoming chunk neighbours when their text happens to equal a chunk ID. A retained outgoing mention with a target outside that format fails with `corrupt`.

All CLI commands accept `--root`. CLI `verify` accepts one citation object or
an array; copy complete citation objects returned by search/read/related, not arbitrary
paths. There are no MCP write/index tools. Search `top_k` is limited to 1–50;
list results carry explicit truncation. Complete stdio response frames are
limited to 128 KiB, including wrappers and newline; oversized responses are
truncated where supported or refused. Inbound stdio frames have a separate 128 KiB line limit.

The default search mode is lexical BM25 with `top_k=10` and abstention disabled. `graph` and `hier` are opt-in; omitted fusion weights remain `0.3`. The model-judged exploratory evaluation does not authorize promoting either mode or changing these defaults.

Every search response also carries `unmatched_terms`: the query terms, after the tokenization described below, that occur in no chunk heading or text anywhere in the corpus, in query order. It ignores `scope`, uses no threshold and never withholds results. When it names the key words of a question, the corpus may not cover that question, or it uses other words. It is not a relevance score, and an empty list does not mean the answer is present.

Every MCP tool result carries its JSON as one text content block. When both copies fit the 128 KiB frame, `structuredContent` repeats the same JSON. When they do not, `structuredContent` is omitted before any search text is withheld or any result is truncated, so clients must read the text block when `structuredContent` is absent. No `outputSchema` is published.

Every search response carries `abstention_score`: the top BM25 score divided by the summed IDF of the query terms, or 0 when nothing matches. Results are withheld (`abstained`) only when the caller passes `threshold`. No validated cutoff exists for general corpora, so any cutoff a caller applies is advisory. Common words have low IDF, so a query made of them can still score high.

Matching uses exact lowercase words, with `snake_case` and camelCase words also split into parts. There is no stemming and no stop-word removal: `recovered` does not match `recover`, and words such as `what` or `how` count like any other term. Prefer distinctive words the expected text is likely to contain, and pass `scope` when the path prefix is known. The MCP tool description says the same.

`scope` filters search paths; it is not authorization. Scores/fusion weights are
provisional, not confidence estimates or measured relevance guarantees. With
`budget_bytes`, search selects structural L0/L1 or full-text L2 tiers; it does
not generate summaries. Exact section IDs depend on allocated heading hierarchy,
not chunk budget. Preambles use `path#`; empty heading components use `@empty`
only in canonical IDs. Markdown anchors remain unchanged, and `#@empty` is not
a new Markdown anchor. Chunk IDs append `~<ordinal>`.

### Link diagnostics

`index` reports a `link_diagnostics` count and `status` lists the diagnostics (capped, with `truncated`). Each names the source `path`, the link `target` and a `problem`:

- `MissingTarget`: the link names a `.md` or `.markdown` path that is not an admitted document of this corpus. A link starting with `/` is resolved from the corpus root, so an absolute filesystem path such as `/home/me/docs/x.md` is reported here whether or not that file exists.
- `MissingAnchor`: the target document exists but has no heading with that anchor.
- `OutsideRoot`: a relative link climbs above the corpus root, for example `../../other/x.md`.

Diagnostics never block publication and never become graph edges. A corpus that links to files outside its own folder will always report some; they describe the corpus, not an index failure.

### Writing documents that search well

These are untested suggestions from a hand-run probe (2026-11-15), not measured guidance:

- Open each decision record or plan with one plain line naming the questions it answers.
- Start a section that a later amendment changed with `Amended by:` and a link, so the original text is not read as current.
- Use links relative to the document, not absolute filesystem paths.

`read` serves the indexed snapshot. Freshness hashes current admitted file bytes;
mtime equality is not a shortcut. Stale results remain visible with flags.
`verify` distinguishes valid, uniquely moved, stale/ambiguous and missing spans
against current bytes; it does not refresh the snapshot. A publication between
calls can change the active generation; cross-call snapshot pinning is absent.

## Safety And Delivery Limits

- Publication and load refuse duplicate section/chunk IDs, duplicate document paths and malformed graph edges. `NEXT` and `LINKS_TO` require chunk endpoints; `NEXT` stays within a document. `CONTAINS` requires document-to-section, section-to-section or section-to-chunk endpoints. `MENTIONS` requires a chunk source and an unresolved `code:` target. Unknown edge kinds refuse. A refused active generation makes its corpus unavailable until repaired; schema remains 4.
- To recover from one of these generation refusals, stop the affected service and trusted indexing processes. Identify the refused generation ID in that corpus's `.ontodoc/CURRENT`, then remove **only** `.ontodoc/generations/<id>` for that ID. Keep `CURRENT` and `HISTORY`. Run `ontodoc --root <corpus> index`, then `ontodoc --root <corpus> status`; restart only after successful load and the expected freshness result. Reindexing before removing the directory can return `same-id artifact conflict`. The [recovery regression](src/store.rs) proves directory removal, republish and load for malformed edges and duplicate IDs. Preserve any evidence needed before removal; do not apply this procedure to pinned evaluation stores.
- Corpus reads use `openat2` beneath the retained root without following
  symlinks, and `verify` accepts only citations that are members of the active
  generation. This does not protect against a hostile store or corpus
  administrator.
- Request inputs, freshness/verification work, store reads and blocking work
  have fixed ceilings, and work permits stay with the work until it stops.
  These are policies, not measured capacity. Every `search`, `read`,
  `related` and `status` rehashes the corpus to report freshness, so cost grows with corpus
  size (capped at 128 MiB read per call); avoid tight `status` polling.
- HTTP limits are conservative policies, not measured capacity. Equal document
  access is not anonymous access: never expose the listener without the
  authenticating ingress.
- Evidence/provenance/topology diagnostic extensions remain Proposed. No new
  tools or measured quality gain follows from donor ideas or passing tests.
- A [local capacity baseline](.memory-bank/audit_session-2026-11-13-capacity.md) records both benchmark corpora and one file just below 2 MiB. Across three fresh-store runs each, command elapsed time was 0.064–0.114 seconds and index peak RSS 15,876–21,156 KiB. HTTP search p95 was 1.8–19.0 ms at concurrency 1 and 2.4–19.1 ms at concurrency 4; overload refusals are reported separately. These short, warm-cache samples have no acceptance target and do not establish sustained capacity. Index aggregate caps are checked after per-file parsing and do not establish an early parser-memory ceiling.

The [synthetic capacity measurement](.memory-bank/audit_session-2026-11-14-ontoindex-ideas-capacity.md) used a separate 0.1.1 release build before the additional generation validation. Each latency below is p95 over 20 sequential HTTP calls; caches were not cleared.

| Corpus | Files / source bytes | Status p95 | Search p95 | Verify p95 |
| --- | --- | ---: | ---: | ---: |
| B16 | 16 / 33,538,048 | 22.95 ms | 31.45 ms | 3.57 ms |
| B60 | 60 / 125,767,680 | 77.84 ms | 117.90 ms | 2.10 ms |
| E50 | 50,000 / 51,200,000 | 325.09 ms | 305.01 ms | 3.20 ms |

All three published within the configured budgets. One four-search batch per corpus returned two successes and two `busy` refusals. The 65,059-file EP case refused indexing with `cap_exceeded`; its computed path demand was 17,038,890 bytes against 16,777,216. Path charge includes the absolute root and filenames, so there is no location-independent file ceiling. At EP's 71-byte root and generated filename shape, the computed ceiling was 64,059 files. Expanding B60 to 66 files without reindexing exceeded the 128 MiB byte reservation budget: status/search refused, while verification remained valid. Server peak RSS across all corpora was 580,816 KiB. The audit contains index/CLI timings, all outcomes and charge arithmetic. These repetitive synthetic fixtures establish neither a recommended size limit nor sustained capacity; the new load-check cost remains unmeasured.

See the [active plan](.memory-bank/ONTODOC_IMPLEMENTATION_PLAN.md),
[service ADR](.memory-bank/ADR_ONTODOC_SERVICE.md),
[owner decisions](.memory-bank/audit_session-2026-11-11-owner-decisions.md)
and [architecture owners](.memory-bank/project_architecture.md).

## Development And Evaluation

Read [AGENTS.md](AGENTS.md) and [the memory router](.memory-bank/MEMORY.md)
before changing code. Use the task's focused validation and `CARGO_BUILD_JOBS=4`;
compiler jobs are serialized through the managed lane. Rust changes require
formatting and all-target Clippy before completion. Documentation-only tasks
do not compile Rust. Do not build the donor snapshot.

The confirmatory harness requires physically split immutable inputs, reviewed
queries, human annotation, a frozen clean commit/binary identity and a
pre-registered powered held-out run; human annotation is not done. The current
track uses model-judged labels and is exploratory only (no Holm decision, no
promotion). The registered run completed once on 2026-11-13. Do not remove evaluation
refusals, inspect held-out labels during development or claim retrieval gains
from unit tests. [Pending tasks](.memory-bank/project_pending-tasks.md) records
the integrity and external annotation gates.

### Registered exploratory results

The [held-out audit](.memory-bank/audit_session-2026-11-13-held-out.md) records all pooled and per-corpus contrasts, confidence intervals, custody hashes and limitations. The local immutable [report](tmp/release-010/benchmark/eval/results/test-v1/report.json) has SHA-256 `006af6e5fcae05a9693edcc254adf7381d5622dbeae211264578f32d1a9691e0`; it is retained locally under ignored `tmp/` and is not included in a source checkout. Candidate `cc116c44a4d7f854bc19c6a4b40d75098a437268`, binary SHA-256 `af5fd69e216cb8a8e81ba495588f50d8274725388d27d61d1050fef3a2b83b55`, was measured on 165 answerable and 55 off-topic queries across OntoIndex and the Rust Book.

| Contrast | Mean nDCG@10 difference | Bootstrap 95% CI | Interpretation |
| --- | ---: | --- | --- |
| H1: lexical B1 versus B0 | +0.719047 | [0.685364, 0.752125] | Sanity gate passed; B0 returned no results |
| H2: graph B2, alpha 0.5, versus B1 | +0.032912 | [0.016128, 0.049996] | Exploratory, model-judged labels; no decision |
| H5: hierarchy B3, beta 0.3, versus B1 | +0.032709 | [0.012113, 0.053406] | Exploratory, model-judged labels; no decision |

Pooled nDCG@10 was 0.719047 for lexical, 0.751959 for graph and 0.751756 for hierarchy; condensed nDCG was identical. Judged@10 was 1.0 for all systems, vacuously so for empty B0. H3's dev-selected threshold gave correct abstention 42/55 (76.36%, Wilson 95% CI [63.65%, 85.63%]) and false abstention 13/165 (7.88%, [4.66%, 13.01%]); abstention stays disabled by default. H4 fixture tests cover citation validity and all four states; they do not establish exhaustive validity for every benchmark result. E1 and per-corpus intervals are reported in the audit, including intervals crossing zero.

All test queries were model-drafted, and labels were model-judged; shared model and lexical biases limit interpretation. Twelve answerable queries with no relevant pooled section after the depth-30 extension were withdrawn before measurement: `oi-cn-020/021/044`, `oi-lk-039/040`, `oi-mh-019/023/025/028/029/049/050`. Withdrawal raises absolute scores and split recomputation can change results. Held-out per-stage miss attribution is unavailable, an owner-approved limitation. The sole attempt is consumed; a future human-label confirmatory claim needs fresh test queries. The [default closeout](.memory-bank/audit_session-2026-11-13-default-noop.md) records no application changes.

## Prior-Attempt Source Snapshot

OntoDoc's standalone root package is licensed under the MIT licence in [LICENSE](LICENSE), copyright (c) 2026 OntoDoc contributors. Dependency licence texts and notices are collected in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

The retained `ontocode-rs/` snapshot remains separately licensed under [LICENSE-APACHE](LICENSE-APACHE), with its original [NOTICE](NOTICE). Neither is relicensed by the root MIT licence. Benchmark corpora retain their own licences.

Copied from the Ontocode working tree on 2026-11-06. Original repository-relative
paths are preserved except that the copied root `LICENSE` now resides at `LICENSE-APACHE`, byte-for-byte unchanged. `COPY_MANIFEST.json` is the original historical record (its `LICENSE` entry refers to those Apache bytes), recording the source commit and SHA-256
hash of each copied file; the snapshot uses working-tree contents, which may
include uncommitted edits rather than only committed bytes.

### Snapshot Contents

- `ontocode-rs/ext/ontograph/`: complete owning crate, including LineageRAG,
  Knowledge Modules, shared graph helpers, unit tests, and fixtures.
- `ontocode-rs/ext/extension-api/`: extension interfaces used by the owner.
- Selected CLI, TUI, app-server, feature, configuration, and core files:
  registration, commands, lifecycle integration, and regression tests.
- Workspace manifest, lockfile, toolchain declaration, crate manifests,
  `LICENSE`, and `NOTICE`.

### Snapshot Entry Points

- `ontocode-rs/ext/ontograph/src/lib.rs`: module inventory.
- `ontocode-rs/ext/ontograph/src/extension.rs`: host contributors and registration.
- `ontocode-rs/ext/ontograph/src/lineage_admission.rs`: lineage admission.
- `ontocode-rs/ext/ontograph/src/lineage_km_retrieval.rs`: KM retrieval integration.
- `ontocode-rs/ext/ontograph/src/km.rs`: KM types and constraints.
- `ontocode-rs/cli/src/km_cmd.rs`: KM CLI operations.

### Snapshot Boundaries

This is a source snapshot, not an independently buildable workspace or MCP
service. Copied manifests still reference Ontocode workspace crates not included
here (including the rest of core, protocol, tools, telemetry, secrets, utilities,
and test support). Shared host files contain unrelated functionality because
they were copied intact, without rewriting code.

Runtime KM stores and packages, session logs, credentials, build outputs, Git
history, and project tracking documents are excluded. Tests and fixtures inside
the owning source crates are included. No KM import, activation, or migration
was performed. Source files in Ontocode were not modified.

The copy record reports all 198 source copies were compared byte-for-byte and
hashed at acquisition. That historical result is not validation of the current
Rust service. The snapshot is **read-only**, excluded from the root build and
never a dependency. Reuse only approved ideas rewritten against std/public
crates; never import or build these files. See the
[prior-attempt review](.memory-bank/audit_session-2026-11-08-prior-attempt-review.md).
