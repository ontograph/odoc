# OntoDoc

Standalone Rust MCP service for cited documentation retrieval. It exposes
five read-only tools over **stdio** (one configured corpus) or over a
**loopback HTTP** listener (one endpoint per `docs_share/<folder>/`). Index publication is a trusted CLI action. Retrieval uses local
lexical/graph algorithms; no model, embeddings or outbound service is required
on the retrieval path.

**Status:** experimental, not ready for untrusted clients or remote exposure.
Retrieval-quality and multi-user performance claims have not been established.
Evaluation commands that measure, score or freeze results fail closed while
their integrity prerequisites remain unresolved.

## Install And Index

Native Linux, Rust **1.95.0**, edition 2024. The root toolchain file pins rustfmt
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
pointer. **Schema 2 requires reindexing schema-1 stores** with the trusted
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

### HTTP (loopback, behind an authenticated ingress)

Index each folder first (`ontodoc --root docs_share/<name> index`), then:

```sh
ontodoc serve-http --docs-share /absolute/path/to/docs_share --listen 127.0.0.1:8080
```

Each immediate folder is served at `http://127.0.0.1:8080/<percent-encoded
name>/mcp` with its own `.ontodoc/` store; endpoints are printed to stderr.
The transport is MCP Streamable HTTP in stateless JSON mode: `POST` only, one
JSON-RPC message per request, no sessions or SSE (`GET`/`DELETE` → 405).

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

| MCP tool | Main input | Behavior | Equivalent CLI |
| --- | --- | --- | --- |
| `search` | `query`, optional `mode`, `top_k`, `scope`, `explain`, `budget_bytes` | Ranked chunks with section IDs, citations and freshness; modes `lexical`, `graph`, `hier` | `search QUERY --mode lexical --top-k 5` |
| `read` | `id` | Snapshot chunk/section by exact ID or existing `path#anchor` alias | `read 'guide.md#installation'` |
| `related` | `chunk_id` | Graph neighbours within this corpus | `related 'guide.md#installation~0'` |
| `verify` | `citations` array | Recheck cited spans against current file bytes | `verify JSON`, or `verify -` to read JSON from stdin |
| `status` | empty object | Active generation, counts, freshness and diagnostics | `status` |

All CLI commands accept `--root`. CLI `verify` accepts one citation object or
an array; copy complete citation objects returned by search/read, not arbitrary
paths. There are no MCP write/index tools. Search `top_k` is limited to 1–50;
list results carry explicit truncation. Complete stdio response frames are
limited to 128 KiB, including wrappers and newline; oversized responses are
truncated where supported or refused. This does not bound inbound allocation.

`scope` filters search paths; it is not authorization. Scores/fusion weights are
provisional, not confidence estimates or measured relevance guarantees. With
`budget_bytes`, search selects structural L0/L1 or full-text L2 tiers; it does
not generate summaries. Exact section IDs depend on allocated heading hierarchy,
not chunk budget. Preambles use `path#`; empty heading components use `@empty`
only in canonical IDs. Markdown anchors remain unchanged, and `#@empty` is not
a new Markdown anchor. Chunk IDs append `~<ordinal>`.

`read` serves the indexed snapshot. Freshness hashes current admitted file bytes;
mtime equality is not a shortcut. Stale results remain visible with flags.
`verify` distinguishes valid, uniquely moved, stale/ambiguous and missing spans
against current bytes; it does not refresh the snapshot. A publication between
calls can change the active generation; cross-call snapshot pinning is absent.

## Safety And Delivery Limits

- Corpus reads use `openat2` beneath the retained root without following
  symlinks, and `verify` accepts only citations that are members of the active
  generation. This does not protect against a hostile store or corpus
  administrator.
- Request inputs, freshness/verification work, store reads and blocking work
  have fixed ceilings, and work permits stay with the work until it stops.
  These are policies, not measured capacity. Every `search`, `read` and
  `status` rehashes the corpus to report freshness, so cost grows with corpus
  size (capped at 128 MiB read per call); avoid tight `status` polling.
- HTTP limits are conservative policies, not measured capacity. Equal document
  access is not anonymous access: never expose the listener without the
  authenticating ingress.
- Evidence/provenance/topology diagnostic extensions remain Proposed. No new
  tools or measured quality gain follows from donor ideas or passing tests.

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

The harness requires physically split immutable inputs, reviewed queries, human
annotation, a frozen clean commit/binary identity and a pre-registered powered
held-out run. Those prerequisites are incomplete. Do not remove evaluation
refusals, inspect held-out labels during development or claim retrieval gains
from unit tests. [Pending tasks](.memory-bank/project_pending-tasks.md) records
the integrity and external annotation gates.

## Prior-Attempt Source Snapshot

Copied from the Ontocode working tree on 2026-11-06. Original repository-relative
paths are preserved. `COPY_MANIFEST.json` records the source commit and SHA-256
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
