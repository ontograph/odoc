# Trusted Context course: review, challenge and ideas for OntoDoc

Date: 2026-11-14. Subject: the landing page `https://im-center.ru/ai-trust-context-course`, «Практический курс по работе с AI-ответами».

Evidence base: the visible text of that one page, fetched by an explorer over HTTP (status 200 on the non-`www` host; the `www.` host answers 403 to non-browser clients), plus the linked `research-analytics` page. OntoDoc facts come from source and documents read in this session. Nothing was built, tested or benchmarked, and no evaluation payload was opened. This report is the only file written.

## What the course teaches

A two-evening course (6 hours live, 4 to 6 hours on the participant's own case, a 15-minute defence, 39 000 ₽) on the "Trusted Context Method" (TCM). Authors: Станислав Карамушко (method) and Артём Варкулевич (founder of Onto, the tool used in the lab).

The method reduces to five statements:

1. Three questions precede any use of an AI answer: can it be answered from the available context, can the answer be proved, and does anyone have the authority to act on it.
2. A conclusion is backed by a chain Source → Evidence → Claim with provenance, and the step from evidence to claim ("Reasoning") is checked separately.
3. «Найти ≠ обосновать ≠ проверить»: finding a passage, grounding a claim in it and verifying the reasoning are three different things. A source link alone shows neither which evidence supports the conclusion nor why the inference is allowed.
4. «Trusted Context ≠ True Context»: traceable context is not thereby correct. Unknowns and contradictions are recorded, not filled in.
5. The outcome is a decision state: ACT, REQUEST EVIDENCE, ESCALATE or STOP, with a named next action. A justified non-answer is a valid result.

The page's seven steps (FRAME, MODEL, BUILD, GROUND, CONNECT, VERIFY, DECIDE) are a procedure around those statements. BUILD means a hand-made schema in Onto; CONNECT means giving the model access to it, with MCP named as the later access layer.

## Challenge

**The validation claim has no public evidence.** The page says «Метод проходил контролируемую валидацию с результатом PASS WITH FINDINGS» and that the findings shaped version 1.1. Neither the landing page nor the linked research page says who ran it, on which cases, how many, against what comparison, or what the pass criteria and findings were. The page itself adds that this is not a claim of universally proven effectiveness. By OntoDoc's own rule that quality claims need pre-registration and measurement, this sentence carries no weight.

**The white paper was not read.** The 15-page paper «TRUST CONTEXT ДЛЯ AI-РЕШЕНИЙ» is offered through a form that asks for a name and a contact. The form markup exposes the post-submission download address, but the form was not submitted and that address was not opened. The paper may contain the validation design; this review cannot say.

**Answer A versus Answer B is a demonstration, not a comparison.** The participant gets an answer, then curates context by hand, then gets a second answer and judges it. One case, the same person builds the context and grades the result, and nothing separates the effect of the method from the effect of reading the sources carefully. It teaches the idea; it measures nothing.

**The page does not say who performs VERIFY.** "Check whether the AI invented the inferential rule" is ordinary critical reading if a person does it, and a model grading its own reasoning if the model does it. The page text does not settle which.

**The lab depends on a hand-built ontology in a co-instructor's product.** The page says the method is assessed and not the tool, but the worked path runs through Onto, and its founder co-teaches. A schema modelled by hand for two or three claims says nothing about cost at corpus scale.

**The content is established practice under new labels.** Claim, Evidence and Reasoning correspond to claim, grounds and warrant in Toulmin's argument model; the Source → Evidence chain is provenance tracking; the four decision states are an escalation policy. This is my reading, not something the page states. It lowers the novelty of the method, not the usefulness of the checklist.

**Minor credibility signals.** The explorer reported that the page gives two different dates for the last group and two different group sizes (up to 10, and 5), next to a "5 places left" marker. These were not re-checked.

## How it maps to OntoDoc

OntoDoc is a retrieval service with no model on its path. It produces passages and citations; it never forms a claim. Of the course's chain it owns exactly one link, Source → Evidence.

| Course element | OntoDoc today | Disposition |
| --- | --- | --- |
| Source → Evidence with provenance | Every result carries a citation with path, heading path, byte span, line span and SHA-256 (service ADR, decision 8) | Already present |
| Evidence still matches its source | `verify` returns `valid`, `moved`, `stale` or `missing` against current bytes (`src/cite.rs`); every response reports freshness | Already present |
| Answerability, justified non-answer | `abstention_score` is returned on every search; a threshold enables typed abstention (`src/search/mod.rs:402`). Default is off by owner decision; the held-out run reported 42/55 correct and 13/165 false abstentions as descriptive figures | Already present; default unchanged |
| Keep unknowns visible | `stale`, `truncated`, `text_truncated`, `omitted`, skipped files and link diagnostics are explicit fields | Already present |
| Which bytes back the displayed text | With `budget_bytes`, L0 and L1 text can lie outside the cited span (`README.md:132`). The rendered-text provenance proposal in `ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md` covers this and is not accepted | Already proposed; the course adds no new argument |
| Find ≠ ground ≠ verify | Not stated anywhere a client reads | **Gap; see below** |
| Evidence → Claim, Reasoning, contradictions | Needs judgement about meaning | Out of scope: no model on the retrieval path |
| Decision authority, decision states | Organisational policy of the caller | Out of scope |
| A/B answers with and without context | Would be an end-to-end answer evaluation with a model judge | Out of scope; no evaluation is authorised |

## The one idea worth taking

**Say in client-facing text what a `valid` citation does not establish.**

The tool is named `verify`, and the server instructions tell the client to "check quotes with `verify`" (`src/mcp.rs:533-544`). The clients are language models, and they read tool descriptions as guidance. An explorer searched `README.md`, `docs/DEPLOYMENT.md`, `docs/EVALUATION_0.1.0.md` and the service ADR for a sentence saying that a valid citation does not show the passage supports the caller's statement or that the passage is correct, and found none. The existing sentences describe the byte comparison precisely (`README.md:129`, `:159`; service ADR decision 10) and one says verification "does not certify the relation itself" for graph neighbours (`README.md:136`). None draws the line the course draws.

Proposed sentence, to be adapted per place:

> `verify` reports whether the cited bytes are unchanged in the current file. A `valid` result does not show that the passage supports a statement made from it, or that the passage is correct.

Places: the Tools section of `README.md`, the `verify` tool description and the `get_info` instructions in `src/mcp.rs`.

What this is and is not:

- It restates decision 10 of the service ADR in terms of what follows from it. It changes no behaviour, schema, ranking or default, so no ADR amendment is needed.
- It is justified by the absence of the statement, not by an observed failure. Nobody has measured whether clients over-read `valid`.
- The `src/mcp.rs` part is a Rust change. It needs a plan task with a write set, `cargo fmt --check`, Clippy and `cargo test --test mcp`, and it belongs to the unreleased 0.1.1 line, not to the packaged 0.1.0 candidate. An explorer found no test asserting on the instructions or tool description text in `src/mcp.rs`, `tests/mcp.rs` or `tests/http.rs`; other test files were not searched.
- The `README.md` part is documentation only.

A second sentence could travel with it: with abstention disabled, a search that matches nothing returns empty `results` with `abstained: null` and `abstention_score: 0`. That is what `abstention()` does when no threshold is set (`src/search/mod.rs:402-415`), and a test pins `abstained` to none for that case. The ADR does not describe this case. In lexical mode the empty list follows from the code read here; for `graph` and `hier` it depends on PPR with no seeds, which was not read.

## Considered and not taken

- **Report `no_lexical_match` when abstention is disabled.** It would make the non-answer typed in the default configuration, matching `scope_empty`, which is typed regardless of threshold. Not taken: an empty result list already tells the client the same thing, the current behaviour is deliberate and pinned by a test, and OntoIndex rates a change to `abstention` as HIGH risk (nine dependents across three modules, the evaluation run among them). An explorer found that evaluation code reads the score and not the reason, so outputs would probably not change, but that is an inference.
- **Claim or contradiction checking inside OntoDoc.** Needs a model; the service rules forbid one on the retrieval path.
- **A typed Source/Evidence/Claim schema.** OntoDoc stores no claims. The course's schema is the client's artefact.
- **Accepting rendered-text provenance on the strength of this course.** The proposal stands or falls on its own gates; a course page is not evidence for it.

## Limits

- One landing page, visible text only, read once through an explorer's HTML-to-text extraction. Hidden or script-rendered content could have been missed.
- The white paper and any validation report were not read.
- Line numbers quoted from the service ADR and README come partly from an explorer's lookup; the ADR abstention passage and the README sentences were also read directly, the ADR statement that abstention stays disabled when the threshold is omitted was not.
- OntoIndex reported partial semantic coverage and a dirty working tree during this session; source statements here rest on the files as read, not on the graph.
