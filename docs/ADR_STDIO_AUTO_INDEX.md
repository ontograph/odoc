# ADR Amendment Draft: Opt-In Stdio Index Refresh

Date: 2026-11-15
Status: Approved by the owner on 2026-11-15 and applied to the service ADR as revision 20 on 2026-11-15 by task AI-ADR of [STDIO_AUTO_INDEX_PLAN.md](STDIO_AUTO_INDEX_PLAN.md). Not implemented; AI-1 implements it.
Target: `.memory-bank/ADR_ONTODOC_SERVICE.md`, revision 19 to revision 20.
Requested by: owner, who selected option D ("refresh while serving"), instructed "update adr", and then replied "proceed" to the draft on 2026-11-15.

The service ADR stays the design authority. This file holds the exact text of the amendment because the session that drafted it could not write under `.memory-bank/`. Five attempts were refused on 2026-11-15: the coordinator's own patch, a worker that declined the files as outside its role, a Luna worker whose workspace was mounted read-only, a second Luna worker refused with "current automatic continuation is read-only for the patch target; resume in a writable turn", and a third refused with "project-plan tracking is single-writer: only coder-manager may modify tracking files". No edit landed. Until the edits below are applied there, revision 19 is in force and `serve` has no automatic indexing.

The owner chose option D without naming one of its two behaviours. The draft put the question with a recommendation, that the call waits for the rebuild, and the owner replied "proceed". That reply is recorded here as acceptance of the recommendation. The other behaviour is listed under Alternatives Considered.

## Edits To Apply

Apply all five together.

1. **ADR header, line 4.** Replace the prefix `Revision: 19 (2026-11-14).` with the text below, followed by one space, and keep the rest of the line.

   ```text
   Revision: 20 (2026-11-15). Revision 20 accepts the opt-in stdio index refresh below (`serve --auto-index`), authorized by the owner's selection of option D and the instruction "update adr" on 2026-11-15. It is not implemented and needs an approved plan task.
   ```

2. **ADR Decision item 8.** Replace the line `   never an MCP tool.` with these two lines.

   ```text
      never an MCP tool. Revision 20's opt-in stdio refresh adds no tool; see
      [Opt-In Stdio Index Refresh](#opt-in-stdio-index-refresh-accepted-not-implemented).
   ```

3. **ADR, new section.** Insert the section under [Section Text](#section-text) immediately before the heading `## Folder-Scoped HTTP MCP Architecture (Accepted, Not Implemented)`. In the ADR its top heading is level 2 and its sub-headings are level 3.

4. **ADR, "Delivery Boundary".** Replace the sentence that starts `Still outside this ADR: automatic indexing,` with the text below.

   ```text
   Still outside this ADR for the HTTP listener:
   automatic indexing, database creation by the listener, authentication in
   OntoDoc, sessions/SSE and non-loopback binds. Revision 20 changes stdio only.
   ```

5. **`.memory-bank/MEMORY.md`, Service ADR line.** Replace it with the line below.

   ```text
   - [Service ADR](ADR_ONTODOC_SERVICE.md) - folder MCP, safety/custody, schema 4, exploratory evaluation exceptions, citations, accepted generation invariants; opt-in stdio index refresh accepted, not implemented (rev 20).
   ```

After applying, check that a search for trailing blanks in both files prints nothing and that `MEMORY.md` stays at or below 100 lines.

## Section Text

The text from the next heading down to the end of "Limits" is the section to insert. Headings are shown one level deeper here than they will be in the ADR.

### Opt-In Stdio Index Refresh (Accepted, Not Implemented)

Authority: on 2026-11-15 the owner selected option D ("refresh while serving") from five options for local use, then instructed "update adr". The owner did not choose between the two refresh behaviours offered. This section records the recommended one: the call waits for the rebuild. Changing that choice needs an edit to this section before implementation starts. No code exists, and implementation needs an approved plan task.

Purpose: a single local user starts the stdio server in a working folder and queries its Markdown files without running `ontodoc index` by hand.

#### Decision

- `ontodoc serve` gains the opt-in flag `--auto-index`. It is off by default, and without it the server behaves exactly as before.
- The flag exists for stdio only. `serve-http` does not accept it; the HTTP rules on index publication and the delivery boundary below are unchanged.
- With the flag, each admitted tool call does the following inside its existing work permit, before its handler runs:
  1. If the corpus has no active generation, build and publish one.
  2. If the active generation loads and the item 9 freshness check reports it stale, build and publish a new one, then reload.
  3. Otherwise do nothing extra.
- Build and publication are the existing CLI path (`App::index`, `store::build_rooted`, `Store::publish`) with the same caps, lock and outcome rules. There is no incremental build and no second writer.
- The call waits for the rebuild and answers from the new generation. Its `fresh` value still comes from the item 9 comparison.
- A call makes at most one staleness check before its handler and at most one rebuild. The handler may reuse that check's result only when no rebuild happened.
- No tool, request field, response field, schema version, ranking input or dependency is added. Generation bytes for a folder equal those of `ontodoc index` on the same folder.

#### Failure Behaviour

| Case | Result |
| --- | --- |
| Rebuild fails and an active generation exists | The call is answered from that generation with `fresh: false` and the existing per-result `stale` flags |
| Rebuild fails and no generation exists | The call returns the build's existing typed error |
| Publication commits with uncertain durability or failed maintenance | The new generation is served; the outcome code goes to stderr |
| The store lock is held by another publisher | Treated as a failed rebuild; a later call sees whatever that publisher committed |

After a failed rebuild the server waits at least 30 seconds before trying again. The interval uses a monotonic clock held in process memory and never enters an index, a response or an evaluation output. Failures are logged to stderr as fixed codes, without paths, queries or operating-system text.

#### What Does Not Trigger A Rebuild

A refused or corrupt active generation (revision 19), an unsupported schema, an unsafe or unsupported filesystem, and every other load error keep their typed errors and their operator recovery. The server never deletes, repairs or replaces a generation it could not load.

#### Bounds And Work Ownership

- The rebuild runs under the call's own permit, as the accepted rule on actual blocking work requires. No detached lane, background thread or timer is added.
- The stdio server admits one operation at a time, so a rebuild makes concurrent calls return `busy`.
- A client timeout does not stop the rebuild. The work keeps its permit until it ends, and a later call finds the published generation.
- The existing build limits apply unchanged: 100,000 probes, 2 MiB per file, 128 MiB of source work and the artifact ceiling. Exceeding one fails the rebuild; nothing partial is published.
- These are admission policies, not measured capacity or latency guarantees.

#### Trust And Safety

- A local user starts the stdio process for a folder they chose. That user is the trusted operator, and the flag is their standing instruction to index that folder. Tool arguments never select a root or carry an indexing option.
- Writes go only to `<root>/.ontodoc/` through `src/store.rs`. Corpus documents are never written.
- The five tools keep `read_only_hint`: it describes the corpus documents. With the flag set, the server instructions must state that the server maintains its own index store.
- Operators must not use the flag on a pinned benchmark corpus root or on any store bound by an evaluation registration. Evaluation commands do not go through `serve`.

#### Alternatives Considered

| Alternative | Disposition |
| --- | --- |
| Answer from the stale index and refresh in the background | Rejected: it needs work outside the call's permit, and an agent would read outdated text |
| File watcher | Rejected: a new dependency or a polling thread, while the per-call item 9 check already sees every change |
| Overlay index for changed files | Rejected: a second index and a merge step for no stated need |
| Wrapper script that indexes, then serves | Not selected by the owner; it remains usable and needs no code |
| Index store outside the corpus folder | Rejected: it changes store location and corpus identity |

`../ontoindex` was reviewed read-only on 2026-11-15 by a GPT-6-Astra explorer as an idea source; no code, prompt or data was copied. The idea adopted is one rebuild at a time, started by a query that finds the index stale. Immutable generations, the atomic pointer swap, a pinned snapshot per call and stale labels were already present here. The review found that the donor's background refresh swallows scheduling errors, that its overlay capture has no file or byte cap, and that an earlier report of out-of-memory failures was not reproduced.

#### Required Evidence For The Implementing Task

- With the flag off, existing tests pass unedited and responses are unchanged.
- With no index, the first call builds one and answers.
- After a file edit, the next call returns the new text with `fresh: true` and a new generation ID.
- A failed rebuild gives the stale-flagged answer or the typed error above, and the retry interval prevents a rebuild on every call.
- A corrupt generation is not rebuilt.
- The permit is held through the rebuild, and a concurrent call returns `busy`.
- `serve-http` rejects the flag.
- The generation bytes equal those of `ontodoc index` for the same folder.
- The same task updates the operator documentation and the service-boundary wording in `AGENTS.md`.

#### Limits

- Every rebuild is a full one. Its cost on a large folder is not measured.
- The staleness check hashes every admitted file on every call. `search`, `read` and `status` already did this; `related` and `verify` now do too.
- This section adds no evidence about retrieval quality.

## Evidence Behind This Draft

- OntoDoc source read on 2026-11-15: `OntoDocServer::blocking` in `src/mcp.rs` takes a fail-fast permit, compares the active generation ID on every call and reloads when it changed; `App::index`, `App::search` and `App::read` in `src/cli.rs` own the build and the per-call freshness check; `src/corpus.rs` holds the walk limits quoted above.
- `../ontoindex` source read by a GPT-6-Astra explorer on the same day. It did not build or run anything, and it did not reproduce the failures it cites from an earlier report.
- Nothing was built, tested or measured for this draft.
