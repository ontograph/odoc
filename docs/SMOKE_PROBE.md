# Documentation Smoke Probe

Date: 2026-11-15
Status: Questions and expected sources fixed before the rerun; run once on 2026-11-15. Results below.
Task: PQ-SMOKE in [STDIO_AUTO_INDEX_PLAN.md](STDIO_AUTO_INDEX_PLAN.md).
Source: `/opt/d1/_wrk/ontocode/docs/review/ONTODOC_DOC_QUALITY_PROBE_2026_11_15.md` (questions 1–8 and the stop-word query).

This is a smoke check outside the benchmark. It uses the ontocode `docs` folder, which is neither benchmark corpus, and no held-out query. The questions were written by people who had read the documents, so they favour the tool. The expected sources are one reader's judgment, not qrels. The set supports no quality claim, and no parameter is tuned on it.

## Questions

Q1–Q8 and S1 are the probe's. A1–A6 are new, two for each of gaps G1, G2 and G3, and were written for this file before any of them was run.

| ID | Gap | Query | Expected source |
| --- | --- | --- | --- |
| Q1 | — | Which ADR requires one transport owner for out-of-process callers? | `adr/0007` |
| Q2 | — | Why was a defined permission profile ignored? | `adr/0007` |
| Q3 | G3 | Where are soul profiles stored and activated? | `adr/0004` |
| Q4 | — | Can a sub-agent use a different provider than its parent? | `adr/0005` |
| Q5 | G4 | Why was LineageRAG removed? | `adr/0018` |
| Q6 | G1 | What happens to command output that is too large, and how is it recovered? | `adr/0015` |
| Q7 | G2 | Kubernetes horizontal pod autoscaling | none in the corpus |
| Q8 | G4 | Sandbox modes read-only, workspace-write, danger-full-access | none locally |
| S1 | G1 | what is it that how too is | none; stop words only |
| A1 | G1 | How is spending limited before a session spends anything? | `adr/0008` |
| A2 | G1 | Which permissions apply when nothing is configured at startup? | `adr/0016` |
| A3 | G2 | Postgres autovacuum tuning for large tables | none in the corpus |
| A4 | G2 | React useEffect cleanup on unmount | none in the corpus |
| A5 | G3 | Was the librarian chain removed? | `adr/0018`, section "Amendment 2026-11-07" |
| A6 | G3 | What did the soul activation scope reduction keep? | `adr/0004`, section "Amendment 1" |

## Results

Run once on 2026-11-15 through `ontodoc --root /opt/d1/_wrk/ontocode/docs search QUERY --top-k 5`, with lexical mode and no `threshold`, `scope` or fusion weight. The binary was the installed `ontodoc 0.1.1` (`~/.local/bin/ontodoc`), built from this working tree before PQ-1; PQ-1 changed only tool text, so search behaviour is the same. The CLI calls the same `App::search` as the MCP tool. The generation was `7bf5830a9b5aa2f390cf59b35e539382e5a78160cd800ad0058b842018a67ba4`, the probe's own. `fresh` was false because the probe report was added to the folder after indexing; no indexed file was modified or removed. The store was not reindexed.

Rank is the first result whose path starts with the expected source. The probe column is copied from the report.

| ID | Rank | Abstention score | Top result | Probe outcome (score) |
| --- | ---: | ---: | --- | --- |
| Q1 | 1 | 1.33 | `adr/0007` Status and scope | Pass, rank 1 (1.15) |
| Q2 | 1 | 0.65 | `adr/0007` Problem and evidence | Pass, rank 1 (0.79) |
| Q3 | 1 | 0.69 | `adr/0004` Amendment 1 | Partial: storage path at rank 4 (0.52) |
| Q4 | 1 | 1.06 | `adr/0005` | Pass, rank 1 (0.89) |
| Q5 | 1 | 1.13 | `adr/0018` Amendment 2026-11-07 | Partial: right documents, no technical reason (1.13) |
| Q6 | absent | 0.49 | `personas/claude-fable.md` | Fail (0.49) |
| Q7 | no results | 0.00 | none | 5 irrelevant results (0.19) |
| Q8 | n/a | 0.76 | `adr/0016` Decision | No local answer (0.40–0.67) |
| S1 | n/a | 1.10 | `personas/gpt-6-astra.md` Voice | `personas/gpt-6-astra.md` at rank 1 (1.10) |
| A1 | 1 | 0.71 | `adr/0008` | new |
| A2 | absent | 0.64 | `ONTOCODE_UPSTREAM_GAP_ISSUES.md` ONT-104 | new |
| A3 | n/a | 0.36 | `review/LINEAGERAG_CODING_ARCHITECTURE_CHALLENGE_2026_11_06.md` | new |
| A4 | n/a | 0.24 | `plans/LEAN_CTX_TECHNIQUE_ADOPTION_PROJECT_PLAN.md` | new |
| A5 | 1 | 1.77 | `adr/0018` Amendment 2026-11-07 | new |
| A6 | 1 | 1.14 | `adr/0004` Amendment 1 | new |

### Reading

- Q6 and S1 reproduce the probe exactly. Q6 still misses `adr/0015`, and S1, a query of common words only, still scores 1.10, higher than eight of the answered questions here.
- A2 is a new miss of the same kind as Q6. Its words (`permissions`, `configured`, `startup`) differ from the title of `adr/0016` ("Unconfigured startup permission defaults") by inflection: `permissions` against `permission`, `configured` against `unconfigured`.
- A3 and A4, both out of the corpus, scored 0.36 and 0.24 and returned five results each. Q8 has no local answer and scored 0.76. In this run, out-of-corpus and in-corpus scores overlap: A3 at 0.36 is close to Q6 at 0.49, and Q8 is above Q2. This matches the report's advice not to ship a default threshold.
- Q3, Q5, A5 and A6 rank an amendment section first. The rank column counts documents, so Q3 shows rank 1 where the probe, judging the specific storage-path passage, called it partial. Neither run checked which chunk holds that passage.
- Q1, Q2, Q3, Q4 and Q8 give different scores from the probe on the same generation, and Q7 returned nothing where the probe returned five results. The report's table gives paraphrased questions, not exact query strings, so the wording here probably differs from the probe's. The score depends only on the query and the corpus, not on `top_k`.

### Limits

- One run, 15 questions, one reader's expected sources. No relevance judgment protocol was followed.
- The questions were written by people who knew the documents.
- The MCP transport, `graph` and `hier` modes, `scope`, `threshold` and the `related` tool were not exercised.
- The run script was a temporary file outside the repository (`/tmp/ontodoc_smoke_run.py`); it calls the CLI once per question and reads `results[].citation.path`, `generation_id`, `fresh` and `abstention_score`.
