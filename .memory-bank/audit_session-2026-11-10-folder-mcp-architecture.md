# Decision: Folder-Scoped MCP Endpoints

Date: 2026-11-10
Authority: user request to make each `./docs_share` subfolder a named endpoint
with its own database, behaving as a separate MCP service.

## Accepted Design

ADR revision 4 specifies `docs_share/<name>/` → `/<name>/mcp`, with each corpus
owning its existing `docs_share/<name>/.ontodoc/` generation database. One Rust
HTTP listener routes to separate logical MCP servers. Each has its own root,
application, shared index slot and sessions; clients reuse that corpus's server.
No merged database, graph or ranking statistics and no cross-corpus fallback.

All users may read every exposed corpus; this separates documentation
collections, not private tenants. Paths/IDs/citations are endpoint-scoped and
registry/cache keys include corpus identity. Names are preserved through URL
segment encoding; traversal, ambiguous routes and symlinks are rejected.
Discovery is startup-only; nested folders belong to their parent corpus.
Indexing remains a trusted per-folder CLI action with atomic publication.

The ADR supersedes the earlier single-corpus deployment assumption. Existing
security/work-bound blockers and stdio compatibility remain. The evaluation
protocol and active task order are unchanged. `docs_share/` is absent and HTTP
is not implemented; no directory/database creation or runtime change was made.

## Review And Evidence

- Existing owners support this design: `src/cli.rs:416-420` selects one root's
  `.ontodoc/` store; `src/mcp.rs:73-104` shares one server's application/index
  through `Arc`s. This is source evidence, not an HTTP runtime test.
- GPT-6-Astra (`/root/folder_endpoints_astra`, requested/effective model confirmed
  at dispatch) found no must-fix architecture issue. Its source-read receipts
  `cfcd45`, `d6a488`, `213e97` exited **0**. It changed no files and executed no
  builds, tests, lint, benchmarks or held-out data access.
- Parent source inspection `2ccd4f` exited **0**. OntoIndex navigation returned
  `workspace_busy`; the reviewer received `workspace_build_failed`. Source
  inspection supplied authority; no reindex was started by an agent.
- Edited existing files: service ADR, architecture router, `AGENTS.md` service
  boundary and memory index. This decision is the only new document. The
  implementation plan, runtime, dependencies, tests and evaluation inputs are
  outside this change. The repository is untracked, so direct content/hash
  checks are needed in addition to `git diff --check`.
- The initial protected-file check exited **1** (`701896`): `CLAUDE.md` changed
  concurrently, and inspection (`bae615`) found the reviewed OntoIndex block
  in `AGENTS.md` replaced by generated guidance. A query-triggered background
  refresh is a possible cause, not verified process attribution. Restored the
  saved reviewed AGENTS block; left the unrelated `CLAUDE.md` change intact.
  Final scope checks distinguish that external change from this task's edits.

## Validation Scope

Documentation validation checks relative links, structure, whitespace, the
memory size cap, consistent endpoint/store examples and protected-file hashes.
Corrected inline `python3` validation and `git diff --check` exited **0**
(`404e97`): five documents, 18 relative links, matching endpoint/store mappings
and 68 unchanged protected files. Four existing docs and this new decision are
the task's write set; the externally changed `CLAUDE.md` is excluded explicitly.
Runtime, Rust formatting/Clippy/tests and scientific evaluation are skipped for
this architecture-only change. No capacity, HTTP readiness or quality gain is
claimed. Scoped implementation tasks must deliver the ADR acceptance evidence.
