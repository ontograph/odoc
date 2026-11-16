# ADR: Bounded Agent-Side Retrieval And Evidence Tracking

Date: 2026-11-16
Revision: 2
Owner: OntoDoc maintainers
Requested by: user, "make adr for new Ideas from habr", following the Selectel article review. Revision 2 follows the user's instruction to review and challenge revision 1.

## Status

Proposed.

This document proposes a bounded retrieval policy for the consuming agent and a staged evaluation of that policy. It authorizes no implementation, model acquisition, measurement, production configuration change or task completion. The [service ADR](ADR_ONTODOC_SERVICE.md) remains authoritative. Acceptance of the design, of an implementation scope and of each evaluation stage are separate decisions.

Revision 2 checks revision 1 against current source. It records an unresolved conflict with the service ADR, corrects two recovery rules, adds the server work a multi-call client causes, names what this repository can own, and replaces the single registered experiment with staged gates. [Challenge Findings](#challenge-findings) lists each change with its evidence.

## Open Decision: Model-Driven Query Reformulation

The service ADR states in its Consequences that "LLM intent expansion and LLM rerank are rejected: they make offline results irreproducible", and in Decision 6 that no LLM is on the retrieval path. Revision 1 read the first statement as a rule about location and moved reformulation into the client. The stated reason is reproducibility, and it applies to any measured system that contains a model, wherever the model runs.

The proposal needs an explicit owner decision before acceptance. Two outcomes are possible:

- The rejection covers the OntoDoc service and its registered retrieval benchmark. A client policy may then be studied. Its results describe one client with one model and are never OntoDoc retrieval-quality claims.
- The rejection covers every workflow OntoDoc endorses. Then only the deterministic parts of this proposal remain: failure-specific use of `read`, `related` and `verify`, and reformulation rules that need no model.

Until that decision is recorded, the model-driven arms below describe a possible experiment and nothing more. Neither outcome permits a model call inside the `ontodoc` crate.

## Context And Source Assessment

The source is Selectel's [«Архитектура RAG 2.0: от простого векторного поиска к Agentic RAG»](https://habr.com/ru/companies/selectel/articles/1106740/). Its useful proposal is to inspect retrieval results, identify missing evidence and choose a bounded next action. Its architecture diagrams and examples do not establish a retrieval-quality gain.

The review found four reasons to treat this as an experiment:

- Context search remains fallible. Vocabulary mismatch, missing documents, wrong versions, stale indexes and withheld text require different recovery actions.
- Joint training and runtime orchestration are distinct. The [original RAG paper](https://arxiv.org/html/2005.11401v4) already jointly trained a query encoder and generator. The article's displayed Contextual AI benchmark compares model and pipeline combinations; it does not compare one system with and without an agent loop.
- The Kubernetes example changes an unspecified startup failure into a `CrashLoopBackOff` search without establishing that observed state. Reformulation can strengthen an unsupported premise.
- Repeated model judgment does not establish factual support. [Research on intrinsic self-correction](https://arxiv.org/abs/2310.01798) found limitations in the studied models without external feedback; that result neither guarantees nor rules out improvement from new tool evidence.

The article body and benchmark image were inspected in the preceding review. The linked Contextual AI page did not load completely, so its experimental method was not audited. No article benchmark, prompt or example is adopted as OntoDoc evaluation data.

The [engineering-RAG review](../docs/HABR_RAG_REVIEW.md) concerns a different Habr article; its distinction between retrieval, delivered evidence and applicability is compatible with this proposal. The [hybrid retrieval ADR](ADR_ONTODOC_HYBRID_RETRIEVAL.md) proposes a separate lexical and dense experiment that must stay out of the first agent-policy comparison. Response extensions remain owned by the [evidence/diagnostics ADR](ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md).

## What This Repository Can Own

The consuming agent is external. No client repository or owner exists. Inside this repository the proposal can reach three places:

- Client guidance in documentation. This is the only deliverable the proposal asks for before measurement.
- The instruction string and tool descriptions that `src/mcp.rs` sends to every client (`OntoDocServer::get_info` and the tool router). They already tell clients to rephrase "with terms from the expected text when results miss" and explain `unmatched_terms`, withheld text and truncated reads. Changing them changes the behaviour of every connected model. That is a server change needing its own task, impact review and evidence, and this ADR does not request it.
- Deterministic scoring of captured traces under `src/eval/`, only if a later task names it. Code that calls a model stays outside the `ontodoc` crate.

Because the server already advises rephrasing, a compliant client today is closer to the fixed or adaptive arm than to a single search. `A-SINGLE` below is a constructed control and does not describe current client behaviour.

## Existing Capability And Limits

The flow is `src/mcp.rs::OntoDocServer` → `src/cli.rs::App` → the existing search, assembly and citation owners. The client policy needs no new server module or tool.

| Existing behavior | Client interpretation |
| --- | --- |
| `App::search` returns ranked chunks with section IDs, citations, a response `fresh` flag, a per-result `stale` flag and `unmatched_terms` | `stale` marks a result whose file changed or disappeared after indexing. `unmatched_terms` ignores `scope`; an empty list does not prove in-scope coverage or answerability. |
| Without `budget_bytes`, results start at L2, the full chunk text. Search returns `tier: null` with empty text for three reasons: the `budget_bytes` budget, the complete-response bound, or chunk text above 64 KiB (`MAX_RESPONSE_BYTES / 2`) | `read` recovers the first two. It cannot recover the third, because `read` cuts at the same 64 KiB. L0 and L1 are structural extracts and can omit a qualification while `text_truncated` is false. |
| `App::read` accepts a chunk ID, a section ID or a corpus-relative `path#anchor`, and returns at most a 64 KiB prefix of the indexed snapshot with `truncated` | A truncated section continues through its chunks. The remainder of a single chunk above 64 KiB is unreachable through any tool. Reading again never refreshes current bytes. |
| `App::related` accepts a chunk ID only, returns at most 100 neighbours with citations and no text, and reports `omitted` | `NEXT` joins consecutive chunks of one document and so gives reading order. `LINKS_TO` targets are already resolved. Each neighbour costs a further `read`. An edge suggests material to inspect and does not establish support. Unresolved `MENTIONS` carry no citation. |
| `verify` accepts at most 100 citations and admits the whole batch before reading any file (`cite::verify_with`) | One citation from another generation, or one that is not an exact indexed chunk or section citation, refuses the entire batch with a typed error and returns no states. An edited citation, including one adjusted from a `moved` result, is refused. |
| Admitted `verify` reports `valid`, `moved`, `stale` or `missing` per citation and reads only the cited files | `valid` and `moved` establish byte correspondence with the indexed span. They do not establish factual truth or support for an answer. |
| `abstention_score` is a normalized lexical score; no cutoff is validated | Neither this score nor a model's self-reported confidence is a probability of correctness. |
| MCP omits `structuredContent` when both copies would exceed the response bound | Read the JSON text block when structured output is absent, and count the result once. |
| `OntoDocServer::blocking` admits one call at a time on stdio and, on HTTP, 4 calls globally and 2 per corpus. Excess calls fail fast as `busy`; there is no queue | Sequential calls are required on stdio. On a shared listener `busy` is a routine transient and says nothing about the corpus. |
| `search`, `read`, `related` and `status` each compare current corpus bytes with the manifest by SHA-256 on every call | Every call in a loop repeats that corpus-wide work. The service ADR forbids replacing it with size or mtime checks. |

Per-call maxima come from `src/cli.rs`: a 128 KiB response, `top_k` up to 50 (default 10), 64 KiB of `read` text, 100 neighbours, 100 citations, 4,096 bytes each for query and scope, and 64 KiB of arguments. These bound one response. Twelve calls can still return 1,572,864 bytes, so they are no context budget for a multi-call client.

These contracts are specified in the service ADR's MCP surface, freshness and citation sections and implemented in [cli.rs](../src/cli.rs), [mcp.rs](../src/mcp.rs), [cite.rs](../src/cite.rs), [search/mod.rs](../src/search/mod.rs) and [graph.rs](../src/graph.rs). Source inspection establishes capability and says nothing about answer quality.

## Options Considered

| Option | Benefit | Limitation | Proposed disposition |
| --- | --- | --- | --- |
| One search followed by bounded reads | Small, predictable workflow | May leave a terminology or dependency gap | Constructed control |
| Deterministic reformulation: inflected forms and identifier splitting, no model | Reproducible; targets exact-word matching without stemming | Covers surface-form mismatch only | No arm in the first comparison; reformulation-type attribution decides whether to propose it |
| Original query plus reformulations written before any retrieval result | Tests whether more retrieval alone helps | Does not adapt to an observed failure | Required control |
| One client controller choosing bounded follow-up actions | Can target a named missing premise | Adds model errors, latency, server work and variability | Experiment, subject to the open decision |
| Server-side planner, agent memory and new SQL/API tools | Broader automation | Changes the service boundary and trust model; no demonstrated need | Outside this proposal |
| Separate planning, execution and checking agents | More independent roles in principle | More cost and correlated model errors; no evidence requiring the split | Deferred |

## Proposed Decision

### Client Ownership And Trust Boundary

Use one consuming-agent controller with request-local state. It calls OntoDoc's existing `search`, `read`, `related` and `verify` tools, and `status` only to list changed paths when a response reports `fresh: false`. OntoDoc continues returning bounded evidence; the client owns interpretation and answer generation. No planner, embedding model, outbound model request, persistent agent memory or new dependency is added to the Rust service.

Each request is bound to one corpus endpoint and its caller-supplied scope. Follow-up actions keep that binding. A missing answer does not authorize switching corpus, widening scope, reading arbitrary files, invoking SQL, executing commands or changing documents. `scope` remains a search filter and grants no authorization. Trusted indexing and the opt-in stdio refresh keep their contracts; HTTP never indexes.

Documents and tool results are untrusted data. Instructions embedded in retrieved text cannot alter the tool allowlist, budgets, endpoint or task. An adaptive loop does let retrieved text influence the next query. Within one read-only corpus that every service user may already read, the consequence is a steered answer or an exhausted budget. Exfiltration risk comes from the client's other tools and its model provider, which the client implementation scope must resolve. This ADR supplies no authorization to send corpus content to a provider.

### Evidence-Guided Loop

1. Preserve the user's question and its explicit identifiers, versions, constraints and scope. Record which prerequisites are observed and which are unknown. A hypothesis never becomes an observation by being searched for.
2. Run the initial lexical search with an explicit `top_k` and `budget_bytes`. Record source identity, delivered text, freshness flags and omissions before deciding that retrieval failed.
3. For each material conclusion, name the inspected passage that supports it or the premise that is missing. Choose a follow-up only when it addresses a named gap.
4. Apply the recovery mapping below. Preserve exact identifiers and negation. Mark new terminology as a search hypothesis unless the question or inspected evidence supplies it.
5. Judge support and applicability separately from byte validity. Before answering, send one `verify` batch containing the citations of the passages the answer relies on, all from the current generation.
6. Answer when the required evidence has been inspected and no material gap remains. Otherwise return a bounded partial answer or ask for the missing observation. Stop when the next action repeats completed work, adds no evidence or exceeds the budget.

"Enough evidence" is a client judgment to be evaluated against reviewed references. It is no server confidence field and no correctness guarantee. A general troubleshooting page cannot establish an incident's observed state; an agent without live diagnostic evidence must say so.

### Failure-Specific Recovery

| Observation | Permitted client response | Required limit |
| --- | --- | --- |
| No useful lexical result | Check `unmatched_terms` and surface form first: matching is on exact lowercase words with identifier splitting and no stemming. Try an inflected or split form before a synonym or a decomposition | Corpus absence and vocabulary mismatch remain alternative explanations; no invented diagnosis |
| Citation retained but text withheld by budget or response bound, or a qualification absent from L0/L1 | `read` the chunk | Text withheld because the chunk exceeds 64 KiB stays incomplete after `read`; report that |
| `read` of a section returns `truncated: true` | Read its chunks in order, using chunk IDs from search results or `NEXT` neighbours. A section span includes descendant sections, whose chunks carry their own section IDs | Every chunk read is charged. A single chunk above 64 KiB stays incomplete |
| A linked dependency or neighbouring explanation may supply a named premise | `related` on the chunk, then `read` a cited neighbour | An edge alone is no support; positive `omitted` prevents an exhaustive-graph claim |
| Two passages appear inconsistent | Compare object, version, configuration, units, conditions and authority | Separate different scopes from an unresolved contradiction; never choose by rank alone |
| A result is flagged `stale`, or the response reports `fresh: false` | Mark claims from the affected paths unresolved for currentness; `status` lists changed paths within its bound | `read` serves the indexed snapshot. The client issues no indexing command |
| `generation_id` differs from an earlier response | Drop citations of the earlier generation from the final `verify` batch and reacquire the needed passages within the original budget, or report unresolved freshness | One earlier-generation citation refuses the whole batch. With stdio `--auto-index` the client's own call can trigger the rebuild |
| `busy` | Repeat the same call after a fixed delay, a fixed small number of times, each attempt charged to the budget | Not evidence that documents are absent |
| Cap, invalid-argument, not-found, corrupt-generation or oversize refusal | Record the failure and stop that action | Deterministic; never retried unchanged or reported as "no relevant documents" |

### Request-Local Evidence Record

Keep a bounded record of material claims, open questions, attempted actions and inspected passages. A passage record holds the corpus endpoint, generation, chunk or section ID, the exact returned citation, the delivered text coverage, the freshness flags and the verification outcome. A claim references the passage actually inspected and records its applicable version and conditions, and whether it is supported, in conflict or open. These are client records. They add no MCP field and no stored artifact.

A citation span is no proof of delivery: a whole-section citation attached to a truncated read does not show that a later sentence was inspected. Keep the exact citation as returned, because `verify` admits only exact indexed citations.

Deduplicate by corpus, generation, source ID and delivered coverage. The same section with fuller text is progress; the same prefix again is not. A support decision does not carry over to a new generation. The existing API has no cross-call snapshot transaction.

State lasts for one request. Production telemetry holds bounded counts, reason codes, durations and stop outcomes, and no raw questions, documents, credentials or model reasoning. An approved evaluation capture may retain the bounded inputs, outputs and action trace it needs, under its own data-handling policy. Generated answers never become a persistent factual memory.

### Bounded Work And Termination

The first comparison allows at most three search calls per question: the original and up to two reformulations. This is an experimental ceiling and neither a measured optimum nor a production default. One tool call is outstanding at a time. Every attempt, failure, `busy` retry and generation-recovery action draws on the original request budget; a rewritten query does not reset it.

Before a stage starts, fix numeric limits for total tool calls, model calls, per-call and aggregate tokens, received response bytes, retained evidence bytes, record counts, `busy` retries and delay, elapsed time and monetary cost. Limits are never raised within a stage. Reserve the final `verify` batch and the answer before spending on another retrieval, and admit the next call against its configured maximum response size.

Stop with a recorded reason: sufficient inspected evidence, missing user observation, unresolved support, conflict or freshness, no progress, operational failure or exhausted budget. Return the supported portion and the material limitation. A passage or qualification dropped to fit memory takes its claim's "supported" status with it.

A deadline stops new work and requests cancellation through the transport. It does not prove that admitted server or model work stopped, so that work stays charged until it is accounted for, and no replacement work starts outside the budget.

## Evaluation Proposal

### Stages

Each stage needs its own acceptance, and each can end the proposal.

- **Stage 0, contracts.** No model. Confirm from existing tests, or from synthetic tests under a scoped task, every server contract in the capability table that the policy relies on. Write the client guidance. The deterministic guidance can ship as documentation with no quality claim on an owner decision, as budgeted context assembly did.
- **Stage 1, dev pilot.** Exploratory, on new dev questions. Its purpose is to estimate effect size, variance, cost and server work, and to decide whether Stage 2 is worth its labelling cost. It supports no adoption claim.
- **Stage 2, registered comparison.** Runs only if Stage 1 shows a difference between `A-ADAPTIVE` and `A-FIXED` at least as large as the minimum useful effect fixed beforehand, and the required number of reviewed questions is obtainable.

Stage 2 is the expensive step. The service ADR's confirmatory track requires human labels, and the completed 0.1.0 evaluation used model-judged labels and stayed exploratory (service ADR revisions 13–15). Answer-level review of several arms per question costs more than section relevance labels did. If human review is unavailable again, Stage 2 can only be exploratory and cannot support an adoption claim.

### Controlled Arms

Keep the same pinned corpus, lexical retriever, tokenizer, chunk budget, response assembly, model and version, answer instructions, source-checking policy and allowed tools. Hold graph and hier ranking and the proposed dense experiment out of this comparison. `related` stays available under the same rules in every retrieval arm.

| Arm | Policy | Role |
| --- | --- | --- |
| `A-CLOSED` | No tools; the model answers from the question alone | Diagnostic: how much the model answers without the corpus |
| `A-SINGLE` | One original-query search, then bounded reading and the final `verify` batch | Constructed control |
| `A-FIXED` | Original query plus two reformulations that the same model writes from the question alone, in one call, before any tool result | Control for a larger search allowance |
| `A-ADAPTIVE` | Original query plus zero to two searches chosen after tool results, each tied to a recorded gap | Candidate |

The retrieval arms share total resource ceilings, read and `related` rules and the answer budget. Actual usage differs between arms and is reported. `A-ADAPTIVE` against `A-FIXED` is the primary contrast, because it alone tests adaptation; `A-ADAPTIVE` against `A-SINGLE` tests the whole package, extra work included. Any further ablation is declared on dev before registration.

`A-CLOSED` guards validity. The service ADR names `rust-book/src` as a pinned benchmark corpus, and a current model may have seen that text in training. If `A-CLOSED` scores close to the retrieval arms, the workload cannot separate them and supports no decision.

### Measures

Use new, reviewed questions covering simple facts, surface-form and terminology mismatch, multi-section requirements, missing observations, absent answers, wrong-version distractors, apparent contradictions, withheld text and source changes. Synthetic cases establish contracts and give no estimate of real-user quality. Do not copy the article's questions or reuse the consumed held-out set.

Keep section-keyed relevance labels. For questions needing several premises, reviewers specify one or more minimum sufficient evidence sets and the passages and qualifications required from each section. Reference evidence is authored from source content and never derived from graph links. Group paraphrases and related information needs before splitting.

- **Primary outcome:** on answerable questions, the rate of answers that are correct and whose every material claim is supported by passages delivered to the model, judged against the reference evidence sets by reviewers blind to the arm.
- **Guardrails:** the rate of unsupported or incorrect material claims on answerable questions, and the false-answer rate on unanswerable questions. Each must stay within a registered margin of the control. Abstentions are reported separately, so universal abstention cannot pass.
- **Evidence measures:** complete evidence-set delivery, relevant-section recall under the declared judgment coverage, missed qualifications and wrong-version conclusions. Candidate recall, delivered evidence and answer correctness are separate; byte verification never counts as a correct answer.
- **Reformulation attribution:** classify every reformulation that recovered needed evidence as an inflected or split form, a synonym, a decomposition or an added hypothesis.
- **Cost and server work:** latency p50 and p95, tokens, model calls, delivered bytes, monetary cost, tool calls per question by tool, and `busy` refusals at a declared concurrency. Each `search`, `read`, `related` and `status` call is one corpus-wide freshness comparison on the server.

Break out prespecified question categories with uncertainty. Cap hits and failed calls stay in the denominator under the frozen failure rubric.

### Reading The Outcome

| Registered result | Consequence |
| --- | --- |
| `A-ADAPTIVE` beats `A-FIXED` on the primary outcome and all guardrails hold | The adaptive policy is the candidate for client guidance |
| `A-ADAPTIVE` and `A-FIXED` are not separated and both beat `A-SINGLE` | The fixed schedule is the candidate: it needs no response-dependent controller. The adaptive controller is not adopted |
| Neither beats `A-SINGLE` | Current guidance stays; the result is recorded as negative or inconclusive under the registered rule |
| Recovered evidence is mostly inflected or split forms | The indicated remedy is a tokenizer experiment under the service ADR. This evidence supports no agent-policy adoption |
| `A-CLOSED` is close to the retrieval arms | Invalid workload; no decision |

A positive result authorizes no server ranking change, embedding integration, instruction-string change or production default.

### Registration And Custody

Tune prompts, stop policy and numeric budgets on dev only. Before held-out access, freeze all arms, configurations, model identifiers and sampling parameters, exact corpus and query and reference bytes, code and binary identities, the capture policy for model and tool calls, seeds and any repetitions. Model sampling can remain nondeterministic at nominal temperature zero. Declare whether measurement uses one captured run or a fixed repetition schedule, and separate deterministic replay of recorded tool interactions from rerunning live inference. Never rerun selectively.

A new registration specifies the primary contrast, minimum useful effect, power, paired uncertainty estimation, multiple-comparison handling and guardrail margins before results. It does not reuse the H2/H5 identifiers, the nDCG effect size or that decision family. Human-confirmatory claims keep the service ADR's query-review and annotation provenance and integrity requirements. Model-judged results need an explicit exploratory authorization and promote nothing.

Preserve the completed registration and attempt records under the Git common directory. The custody implementation supports one registration, so a new experiment needs a separately authorized custody transition and scoped tooling. Do not delete, reset or reinterpret old records, bypass clean-tree or identity gates, or load held-out payloads in a dev command. Record every held-out attempt externally, failures and interruptions included, and mirror records only after measurement.

## Challenge Findings

Findings come from reading the named symbols in current source and the service ADR on 2026-11-16. No build, test or measurement ran.

1. **Conflict with the service ADR was understated.** The rejection of LLM intent expansion is justified by irreproducibility, which a client-side model shares. Revision 2 adds the open decision and stops presenting the model-driven arms as already permitted.
2. **Withheld text is not always recoverable with `read`.** `App::search` withholds chunk text above 64 KiB and `App::read` cuts at the same bound. The capability and recovery tables now separate the three causes.
3. **A truncated section read is not a dead end.** `graph::structure_edges` links consecutive chunks of a document with `NEXT`, and chunk IDs are returned by `search` and `related`. Revision 1 told the client to report incomplete evidence instead.
4. **`verify` batch semantics were missing.** `cite::verify_with` refuses the whole batch for one citation from another generation or one non-member citation. The loop now sends one batch from the current generation and handles a generation change explicitly.
5. **Per-call verification duplicated existing flags.** Search and read responses already carry `fresh` and per-result `stale`. One final batch replaces verification before every currentness statement.
6. **Stopping on the first `busy` made the client fragile.** Admission fails fast with no queue, so `busy` is routine on a shared listener. Revision 2 allows bounded, charged retries and forbids retrying deterministic refusals.
7. **Server work was omitted.** Each `search`, `read`, `related` and `status` call repeats the corpus-wide freshness comparison, so a loop multiplies it. It is now a capability row, a measure and a stated cost.
8. **The proposal had no home in this repository.** No client owner exists, and `OntoDocServer::get_info` already advises rephrasing. Revision 2 names the three places the proposal can reach and marks `A-SINGLE` as a constructed control.
9. **The evaluation could not support its own decision.** Revision 1 left the author of the fixed reformulations undefined, used a composite primary outcome that an abstention mix could move, gave no rule for adaptive and fixed arms that do not separate, ignored answers from model memory on a public corpus, ignored the absence of stemming as a cheaper explanation, and assumed human labels that the last evaluation did not obtain. Revision 2 defines the arms, orders the outcomes, adds `A-CLOSED`, the attribution measure, the outcome table and the stage gates.

## Acceptance Prerequisites And Consequences

Before any acceptance, record the owner decision on model-driven reformulation. Before implementation acceptance, name the external client owner and exact write scope, choose the model, provider and data-handling policy, set every numeric work limit, and specify cancellation and generation-change behaviour. If an API extension becomes necessary, route it through the evidence/diagnostics proposal and amend the service ADR explicitly.

Before measurement acceptance, approve the workload, references, statistical and decision protocol, power, cost limits and custody transition. Synthetic checks cover constraint preservation, repeated-prefix termination, fuller text from the same section, chunk-by-chunk continuation of a truncated section, an oversized chunk, omitted graph neighbours, missing observations, conflicting versions, prompt injection in documents, a generation change before the final `verify` batch, `busy` retries and budget exhaustion. They establish implementation behaviour only.

The expected benefit is targeted recovery using capabilities OntoDoc already exposes. The costs are extra client work, repeated corpus-wide freshness work on the server and more `busy` refusals for other users of a shared listener, nondeterministic orchestration, more failure states and an evaluation whose labelling cost exceeds the last one. A fixed schedule may perform as well, and deterministic guidance may capture most of the value. The accepted service architecture, schema, retrieval defaults and project task order are unchanged.
