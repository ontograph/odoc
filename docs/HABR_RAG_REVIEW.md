# Review: Engineering RAG, Evidence And Applicability

Reviewed: 2026-11-16.
Source: [«Как мы тестировали RAG: нашёл документ, но всё равно ошибся»](https://habr.com/ru/companies/psm_unlim/articles/1106096/), ПСМ, published 2026-11-10; measurements dated 2026-09-12.
Scope: article critique and candidate ideas for OntoDoc. This review authorizes no implementation, ADR change, task completion, ranking change or evaluation run.

## Assessment

The article provides useful failure cases and acknowledges that its automated assessment is not expert acceptance. Its strongest contribution is the distinction between finding a document, extracting its contents and establishing that those contents apply to the requested equipment. Its reported improvement is exploratory evidence with unresolved measurement and attribution questions.

Borrow the failure taxonomy, preservation of context around facts, and requirement to retrieve every source needed to justify an answer. Treat an equipment graph, a query validator and table-row indexing as hypotheses requiring separate evidence. The article does not establish that any particular implementation would improve OntoDoc.

## What The Article Establishes

The authors report 44 documents, 27,007 fragments and 188 evaluation questions. They changed table representation and increased the supplied context from five to ten fragments. Reported answer accuracy rose from 77.7% to 84%, document hit rate at ten from 94.1% to 96.3%, and inclusion of all reference numbers from 70.8% to 78.3%. They report 26 improved answers, 14 regressions and 19 erroneous reference answers discovered during a selective expert check.

These are author-reported observations. The article does not supply the raw paired outcomes, full source documents, reference set, judge configuration or reproducible evaluation artifacts. I read the article and inspected one illustrated conversation; that screenshot shows a corrected answer after an image was supplied, but does not expose the original table or the intermediate OCR/index records needed to independently establish the failure's cause.

## Challenges

### 1. The Measured Gain Is Suggestive

If all 26 improvements are incorrect-to-correct transitions and all 14 regressions are correct-to-incorrect transitions on the same 188 questions under unchanged labels, the net gain is 12/188, or 6.38 percentage points. An exact two-sided McNemar test reduces to a binomial test over the 40 discordant pairs:

```python
from math import comb
p = 2 * sum(comb(40, k) for k in range(15)) / 2**40
# 0.0806904677519924
```

That does not meet a conventional two-sided 5% threshold. It also does not prove there is no improvement. The calculation is conditional: the prose does not fully specify binary transitions, unchanged labels, independence across questions or the number of configurations tried. Questions clustered around the same documents or facts further limit inference. This is a descriptive check of published aggregates, not an independently replicated benchmark.

An inferred 158/188 success count gives a Wilson 95% interval of approximately 78.1–88.6%, under the usual independent-Bernoulli assumptions. That interval excludes uncertainty from erroneous references, model judging and selection during tuning.

### 2. Label Repair And Repeated Testing Need Separate Records

The 19 bad reference answers are a valuable admission. The article leaves unclear how many references experts reviewed and whether the before/after percentages use the original or corrected references. Without that information, one cannot estimate the reference error rate or attribute changes entirely to the system.

Version the labels, record adjudication reasons against sources, and rescore both saved system outputs against the same corrected labels. Human review should be blind to system identity where feasible. A second model can provide automated assessment, but its independence from the answering model does not establish correctness or calibrated agreement with engineers.

Reusing the 188 cases after fixing their failures is useful regression testing. It cannot establish generalization to unseen questions. Keep that suite and use a separately prepared, untouched evaluation for a new quality claim. OntoDoc's consumed held-out attempt must remain untouched.

### 3. Two Simultaneous Changes Do Not Identify The Cause

Row-based table representation and increasing context from five to ten fragments both changed. The aggregate result does not establish how much either change contributed, or whether one compensates for the other. The later attribution of improved numerical accuracy specifically to row indexing is stronger than the described experiment supports.

A future dev experiment can compare the four combinations of old/new representation and small/large context. Report both the deployed configurations and a comparison at matched context-byte budgets: chunk counts cease to be comparable when chunk size changes. Hold the corpus, generator, prompt and assessment procedure fixed, and retain latency, bytes and regressions alongside accuracy. This is a proposal for a new experiment, not permission to retune the existing frozen experiment.

### 4. Retrieval Metrics And Number Matching Miss The Central Failure

A document appearing in the first ten results does not show that the required table row, column heading, unit, qualification or second supporting document reached the model. Likewise, an answer can contain every expected number while attaching one to the wrong component, reversing a limit or adding an unsupported claim.

Separate source extraction, evidence retrieval, context delivery, applicability and answer generation. For derived answers, record a human-authored minimum sufficient evidence set: every section needed to support the conclusion, including configuration and limitations. Measure whether a complete accepted set reaches the delivered context. Multiple alternative sufficient sets may exist. A single relevant-section hit or ordinary Recall@k can conceal a missing necessary premise.

Answer assessment also needs incorrect-extra-claim rate, units and operating-condition checks, and severity. An unnecessary refusal and an unsafe equipment recommendation should remain visible separately even when both count as an incorrect answer.

### 5. Different Numbers Are Not Automatically Contradictions

The station's 1150 kW and the engine's 1280 kW could describe electrical output versus shaft output, different ratings, conditions or configurations. The article does not provide enough source context to decide. Maintenance intervals can also differ by service conditions, component version or an intentionally stricter assembled-product requirement.

Compare facts only after aligning object, configuration, parameter, unit, operating condition, revision and applicable authority. An explicit supersession or approved product-specific instruction can settle a difference; sending every unequal pair to an engineer creates false alarms. When that authority is absent, show the distinct scopes and unresolved question instead of asserting a verified contradiction.

The same care applies to the tank example. The documented pipeline-fed configuration may have no onboard tank. Natural-gas equipment as a class is not incapable of receiving gas from an external storage installation. That does not establish compatibility for this station. A useful response identifies the documented configuration and requests confirmation of any proposed alternative supply.

### 6. A Graph Provides Navigation; Applicability Still Needs Evidence

Connecting a station to an engine manual can retrieve relevant background while increasing the risk of transferring component limits to the assembled product. The authors recognize this and state that applicability handling is still being tested.

The relationship needs provenance and configuration scope. A link alone is insufficient evidence that a component's operating limits are guaranteed for the complete station. Preserve the source of the relationship and the source of each limit, including exceptions. A generic disclaimer after an unqualified numerical recommendation does not repair the recommendation.

The proposed pre-search query validator also has a dependency: its equipment profile must already be correct, complete and current. A missing profile field is not a disproved premise. Prefer retrieving authoritative profile evidence before rejecting a question; keep unsupported, contradicted and configuration-unknown premises distinct. This belongs in the consuming assistant unless a separately approved retrieval-service contract says otherwise.

### 7. Row Indexing Does Not Repair OCR

Preserving a table's headers is useful after extraction is correct. Repackaging a corrupted value as a standalone row can make the error easier to retrieve. Grid geometry validation cannot by itself validate digits, units, decimal separators or footnote applicability.

Retain provenance to the original page/cells in an upstream conversion system and check extraction fidelity independently. A row needs the relevant table title, multi-level headers, units, row labels and exceptions. OntoDoc currently consumes Markdown; verifying the Markdown bytes cannot certify that they faithfully reproduce a PDF or spreadsheet.

## Ideas Worth Taking Into OntoDoc

Current-source findings below come from a bounded read-only inspection. OntoIndex returned stale indexed evidence and was used for navigation; present source supplied the implementation findings. The [service ADR](../.memory-bank/ADR_ONTODOC_SERVICE.md) remains design authority.

| Candidate | Existing capability and owner | Useful next evidence and boundary |
| --- | --- | --- |
| Preserve the context around numerical facts | [chunk.rs](../src/chunk.rs) packs parsed blocks; a large table remains one oversized block/chunk. [tiers.rs](../src/tiers.rs) skips tables when selecting the first paragraph sentence. This is not row indexing. | Synthetic Markdown cases covering headers, units, footnotes and large tables under bounded delivery. Observe both indexing and returned text. Any segmentation change needs identity/schema and retrieval review; the 1,200-byte v1 budget remains frozen. |
| Retrieve all necessary premises | [graph.rs](../src/graph.rs) records structural/link relations; [App::related](../src/cli.rs) returns cited neighbors. Those relations do not encode equipment applicability. | Use hand-authored synthetic Markdown links and evidence sets to test discovery and delivery of all required supporting sections. Keep new entity types, applicability inference and relation semantics as separate design proposals. |
| Distinguish retrieval failure from lost context | [assemble.rs](../src/assemble.rs) chooses L0/L1/L2 under a byte budget; [cli.rs](../src/cli.rs) and [mcp.rs](../src/mcp.rs) bound responses and retain citations when text is withheld. | Measure evidence present in the corpus, ranked candidates and final delivered text separately. Reuse the proposed context-selection accounting capability in the [evidence/diagnostics ADR](../.memory-bank/ADR_ONTODOC_EVIDENCE_DIAGNOSTICS.md); it remains unaccepted. |
| Make verification semantics explicit | [cite.rs](../src/cite.rs) checks citation membership and current source bytes. Validity does not establish factual truth, applicability or coverage of every byte in a composed tier. | An assistant should fetch the cited section where necessary and assess support for its claim. Exact rendered-text provenance is already a proposed capability; avoid duplicating it or implying it exists. |
| Diagnose language and premise failures separately | [tokenize.rs](../src/search/tokenize.rs) and [search/mod.rs](../src/search/mod.rs) provide lexical retrieval and lexical abstention. They provide no RU-to-EN translation or premise judgment. | Synthetic RU-query/EN-document cases can characterize lexical limitations, including model-number and glossary bridges. New semantic retrieval requires new design/evaluation authority. False-premise detection and answer-level conflict handling belong to a separately evaluated client. |

The strongest first candidate is a small synthetic evidence-delivery suite. It should distinguish a missing row, a separated qualification, a missing second source, a wrong-version distractor, budget-withheld text and a language mismatch. Outcomes should describe observable retrieval/delivery behavior; they must not label a valid citation as a correct engineering answer.

This suite would characterize a need before selecting a representation or ranking change. Production measurements would require a new approved evaluation, section-keyed labels and reference review. Do not copy the article's engineering examples into benchmark data or derive relevance judgments from document links.

## Review Evidence And Limits

- Article HTML and full body were read on 2026-11-16. One linked conversation image was inspected. Full source documents, OCR traces, raw outcomes and assessment artifacts were unavailable from the article.
- A read-only explorer inspected the current parsing, chunking, tiers, graph, related, citation and search owners. It changed no files and ran no tests. OntoIndex reported indexed commit `ee511dc0` versus HEAD `aafcb8d7` at the initial query and a background refresh; stale graph results are not current-tree validation.
- The Python standard-library calculation of the paired exact p-value and Wilson intervals exited 0. These are calculations from reported aggregates, not reproduced RAG results.
- Only this review document was added. No Rust compilation, benchmark run or held-out query/label/result inspection was performed; those checks are outside this documentation review.
- Documentation validation: `git diff --check` exited 0. An inline `python3` check of the new file's title, final newline, trailing whitespace, code fences, scope, evidence limits and 12 local links exited 0. Its first version exited 1 because it counted a Python comment inside a fenced block as a Markdown heading; excluding fenced code corrected the checker. `git diff --no-index --check /dev/null docs/HABR_RAG_REVIEW.md` emitted no whitespace diagnostics and exited 1 for the new-file difference. `git status --short` exited 0 and showed this document as the sole change.
