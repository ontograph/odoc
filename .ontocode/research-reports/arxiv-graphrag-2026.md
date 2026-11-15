# arXiv graph RAG papers submitted in 2026: index and download script

Listing date: 2026-11-14. The list covers arXiv entries whose `published` date falls between 2026-01-01 and the listing date.

All 271 tier A PDFs were downloaded on 2026-11-14 to `tmp/arxiv/graphrag-2026/pdf/` (627 MB, ignored by Git) with the script below. Tier B PDFs were not fetched.

| Check | Result |
|---|---|
| PDFs present | 271, each starting with `%PDF` and larger than 10,000 bytes |
| Checksums | `SHA256SUMS` rebuilt after the last file landed; `sha256sum -c` passes |
| Metadata | `metadata.jsonl` 271 lines, `candidates-kg-rag.jsonl` 158 lines |
| Exception | `2608.01269` (ACE-GraphRAG, tier A row 183): the v2 PDF and the unversioned URL return HTTP 404, so `pdf/2608.01269v1.pdf` was fetched by hand instead |
| Run | One interrupted run (3 papers), then a detached rerun that kept those 3 and fetched 267; 1 failure, the exception above |

`failures.jsonl` still lists the v2 failure from the script run, and the script as written does not fall back to an earlier version.

## Result

| Set | Papers | Meaning |
|---|---|---|
| Union of all queries | 433 | Every 2026 entry returned by the seven queries, deduplicated by arXiv id |
| Tier A | 271 | Title or abstract names graph RAG explicitly; these are the papers the script downloads |
| Tier B | 158 | Title or abstract combines "knowledge graph" with RAG, without a tier A phrase; downloaded only with `--tier-b` |
| Dropped | 4 | Matched a query in arXiv's full-record search, but neither rule matched title plus abstract: `2602.15898`, `2605.31506`, `2607.24288`, `2609.14616` |

| Month | Tier A | Tier B |
|---|---|---|
| 2026-01 | 19 | 14 |
| 2026-02 | 22 | 17 |
| 2026-03 | 26 | 14 |
| 2026-04 | 23 | 25 |
| 2026-05 | 33 | 14 |
| 2026-06 | 31 | 20 |
| 2026-07 | 27 | 12 |
| 2026-08 | 33 | 15 |
| 2026-09 | 24 | 14 |
| 2026-10 | 27 | 11 |
| 2026-11 | 6 | 2 |

Tier A id-set digest: `f616fae3e0de48f497615046c8014f6570eba1d0bf63b668fb74714337ab730d`. Tier B id-set digest: `be3f43f02e797d10d1eae3968ab489cbe92998864668a3dea9c2ec286f4ae1e3`. Each digest is the SHA-256 of the sorted arXiv ids joined by newlines.

## Method

Endpoint `https://export.arxiv.org/api/query`, 200 results per page, sorted by submission date ascending, 3 seconds between requests. Every query is ANDed with `submittedDate:[202601010000 TO 202612312359]`, and an entry is kept only when its `published` field starts with `2026`. When a paper appears in several queries, the highest version is kept.

| # | Query | totalResults |
|---|---|---|
| 1 | `all:graphrag` | 168 |
| 2 | `all:"graph rag"` | 59 |
| 3 | `all:"graph retrieval augmented generation"` | 46 |
| 4 | `all:"graph based retrieval augmented generation"` | 49 |
| 5 | `all:"graph enhanced retrieval augmented generation"` | 3 |
| 6 | `all:"graph based rag"` | 38 |
| 7 | `all:"knowledge graph" AND (all:"retrieval augmented generation" OR all:rag)` | 273 |

Classification runs on title plus abstract, lowercased, with whitespace collapsed. `MOD` stands for `(based|enhanced|augmented|structured|guided|aware|driven)` and `END` for end of text or any character outside `[a-z0-9_]`.

| Tier A rule | Pattern | Papers matched first by this rule |
|---|---|---|
| 1 | `graph[ -]?rag` then `END` | 218 |
| 2 | `graph[ -]+MOD?[ -]*retrieval[ -]+augmented` | 30 |
| 3 | `graph[ -]+MOD[ -]+rag` then `END` | 22 |
| 4 | `retrieval[ -]+augmented generation (over|on|with|using) (knowledge )?graphs?` | 1 |

Tier B is every remaining paper that matches `knowledge[ -]+graphs?` and also `retrieval[ -]+augmented` or the standalone word `rag`.

## Evidence and limits

- The listing program ran five times on 2026-11-14, each time with 8 API requests and 0 retries. All five runs reported the same per-query totals, the same union, tier and dropped counts, and the same two digests.
- The tables below were copied from the output of those runs: tier A in three slices (rows 1 to 90, 91 to 180, 181 to 271) and tier B in two (1 to 80, 81 to 158). Row numbers are contiguous, and the rows per month in both tables equal the month counts the program printed.
- The id columns of both tables were read back from this file and hashed: tier A gives 271 ids, 271 distinct, and the tier A digest above; tier B gives 158 ids, 158 distinct, and the tier B digest above.
- `fetch.py` below was extracted from this file with the `awk` command shown and compiles (198 lines). Its query and classification functions were then run against the API without writing files and returned union 433, tier A 271, tier B 158 and the recorded tier A digest.
- One PDF URL was requested twice with the script's user agent, `https://arxiv.org/pdf/2601.00274v1`: HTTP 200, `application/pdf`, first bytes `%PDF`, 8,172,415 bytes. The body was discarded.
- The download loop ran once end to end, including resume after the interrupted first run. The backoff on HTTP 429 and 503 never triggered. PDFs were checked for the `%PDF` header, size and checksum, not opened or read.
- Matching uses title and abstract only. A paper that says "graph RAG" only in its body is absent. A paper that mentions graph RAG in passing is present: tier A contains entries whose main subject is something else, and nobody reviewed the list for topical relevance.
- Papers are not deduplicated by title. Tier B rows 94 and 121 carry the same title under two ids.
- Rows are ordered by the API's `published` date, then by id. For some entries that date is earlier than the month encoded in the id; the table shows what the API returned.
- Titles are as returned by the API, with `|` replaced by `/`. LaTeX escapes in titles, such as the ampersand in tier A row 211, may render differently in Markdown.
- The licence of each paper was not checked. arXiv papers carry different licences, so treat the PDFs as local research copies.
- arXiv keeps accepting 2026 submissions until the end of the year. A later run returns a longer list and a different digest.

## Download in a writable shell

`/tmp/` is ignored by Git in this repository, so the output stays out of the tree. Run from the repository root:

~~~bash
mkdir -p tmp/arxiv/graphrag-2026
awk '/^```python$/{f=1;next} /^```$/{f=0} f' .ontocode/research-reports/arxiv-graphrag-2026.md > tmp/arxiv/graphrag-2026/fetch.py
python3 -u tmp/arxiv/graphrag-2026/fetch.py tmp/arxiv/graphrag-2026 2>&1 | tee tmp/arxiv/graphrag-2026/run.log
~~~

The run needs at least 3 seconds per paper, so about 15 minutes for tier A before transfer time. Rerunning the same command resumes: existing PDFs are kept. `--list-only` writes the metadata without fetching PDFs, and `--tier-b` also fetches the 158 tier B papers.

| Output under `tmp/arxiv/graphrag-2026/` | Content |
|---|---|
| `pdf/<id>v<version>.pdf` | One file per paper, from `https://arxiv.org/pdf/<id>v<version>` |
| `metadata.jsonl` | Tier A records: id, version, published date, category, title, abstract, matching rule |
| `candidates-kg-rag.jsonl` | Tier B records in the same shape |
| `queries.log` | Per-query totals, counts and the tier A digest of that run |
| `failures.jsonl` | Papers whose PDF could not be fetched or failed validation; empty on a clean run |
| `SHA256SUMS` | Checksums of every PDF present |

Checks after the run:

~~~bash
cd tmp/arxiv/graphrag-2026
sha256sum -c --quiet SHA256SUMS
ls pdf | wc -l
wc -l metadata.jsonl failures.jsonl
~~~

A complete run exits with status 0, leaves `failures.jsonl` empty, and has as many PDFs as `metadata.jsonl` has lines. The script exits with status 1 when any PDF failed.

### `fetch.py`

```python
#!/usr/bin/env python3
# Usage: fetch.py [--list-only] [--tier-b] [OUT_DIR]
import hashlib, http.client, json, os, re, sys, time
import urllib.error, urllib.parse, urllib.request
import xml.etree.ElementTree as ET

UA = "ontodoc-research-fetch/1.0 (local research; single connection)"
API = "https://export.arxiv.org/api/query"
PDF = "https://arxiv.org/pdf/"
DATE = "submittedDate:[202601010000 TO 202612312359]"
RECORDED_TIER_A_SHA256 = "f616fae3e0de48f497615046c8014f6570eba1d0bf63b668fb74714337ab730d"


def ph(s):
    return 'all:"' + s + '"'


QUERIES = [
    "all:graphrag",
    ph("graph rag"),
    ph("graph retrieval augmented generation"),
    ph("graph based retrieval augmented generation"),
    ph("graph enhanced retrieval augmented generation"),
    ph("graph based rag"),
    ph("knowledge graph") + " AND (" + ph("retrieval augmented generation") + " OR all:rag)",
]
MOD = "(based|enhanced|augmented|structured|guided|aware|driven)"
END = "($|[^a-z0-9_])"
RULES = [
    "graph[ -]?rag" + END,
    "graph[ -]+" + MOD + "?[ -]*retrieval[ -]+augmented",
    "graph[ -]+" + MOD + "[ -]+rag" + END,
    "retrieval[ -]+augmented generation (over|on|with|using) (knowledge )?graphs?",
]
A = "{http://www.w3.org/2005/Atom}"
O = "{http://a9.com/-/spec/opensearch/1.1/}"
X = "{http://arxiv.org/schemas/atom}"


def fetch(url, timeout):
    delay = 60
    while True:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": UA})
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return r.read()
        except urllib.error.HTTPError as e:
            if e.code not in (429, 503) or delay > 960:
                raise
            print("http", e.code, "sleeping", delay, flush=True)
            time.sleep(delay)
            delay *= 2


def text_of(node):
    return " ".join((node.text or "").split()) if node is not None else ""


def tier(title, summary):
    t = " ".join((title + " " + summary).lower().split())
    for i, r in enumerate(RULES, 1):
        if re.search(r, t):
            return "A", i
    if re.search("knowledge[ -]+graphs?", t) and (
        re.search("retrieval[ -]+augmented", t) or re.search("(^|[^a-z0-9_])rag" + END, t)
    ):
        return "B", 0
    return "D", 0


def run_query(n, q, papers, log):
    full = "(" + q + ") AND " + DATE
    start, total, got = 0, None, 0
    while total is None or start < total:
        params = {
            "search_query": full,
            "start": start,
            "max_results": 200,
            "sortBy": "submittedDate",
            "sortOrder": "ascending",
        }
        url = API + "?" + urllib.parse.urlencode(params)
        entries = []
        for wait in (3, 10, 20, 40, 80, 160):
            time.sleep(wait)
            root = ET.fromstring(fetch(url, 60))
            total = int(root.find(O + "totalResults").text)
            entries = root.findall(A + "entry")
            if entries or start >= total:
                break
        if not entries and start < total:
            sys.exit("query " + str(n) + " incomplete at start " + str(start) + " of " + str(total))
        for e in entries:
            raw = text_of(e.find(A + "id"))
            if "/abs/" not in raw:
                sys.exit("API error entry: " + raw + " " + text_of(e.find(A + "summary")))
            raw = raw.rsplit("/abs/", 1)[1]
            m = re.match("(.+?)v([0-9]+)$", raw)
            pid, ver = (m.group(1), int(m.group(2))) if m else (raw, 1)
            pub = text_of(e.find(A + "published"))
            got += 1
            if not pub.startswith("2026"):
                continue
            if pid in papers and papers[pid]["version"] >= ver:
                continue
            pc = e.find(X + "primary_category")
            title, summary = text_of(e.find(A + "title")), text_of(e.find(A + "summary"))
            kind, rule = tier(title, summary)
            papers[pid] = {
                "id": pid,
                "version": ver,
                "published": pub[:10],
                "category": pc.get("term") if pc is not None else "",
                "title": title,
                "summary": summary,
                "tier": kind,
                "rule": rule,
            }
        start += 200
    log("QUERY " + str(n) + " | " + q + " | totalResults " + str(total) + " | entries " + str(got))


def download(rec, pdf_dir):
    stem = rec["id"].replace("/", "_") + "v" + str(rec["version"])
    path = os.path.join(pdf_dir, stem + ".pdf")
    if os.path.exists(path):
        return "kept", None
    time.sleep(3)
    try:
        body = fetch(PDF + rec["id"] + "v" + str(rec["version"]), 180)
    except (urllib.error.URLError, http.client.HTTPException, OSError) as e:
        return "failed", repr(e)
    if body[:4] != b"%PDF" or len(body) <= 10000:
        return "failed", "not a PDF: " + str(len(body)) + " bytes"
    with open(path + ".part", "wb") as f:
        f.write(body)
        f.flush()
        os.fsync(f.fileno())
    os.replace(path + ".part", path)
    return "fetched", None


def write_jsonl(path, records):
    with open(path, "w", encoding="utf-8") as f:
        for rec in records:
            f.write(json.dumps(rec, ensure_ascii=False, sort_keys=True) + "\n")


def main():
    flags = {a for a in sys.argv[1:] if a.startswith("--")}
    dirs = [a for a in sys.argv[1:] if not a.startswith("--")]
    out = dirs[0] if dirs else "tmp/arxiv/graphrag-2026"
    pdf_dir = os.path.join(out, "pdf")
    os.makedirs(pdf_dir, exist_ok=True)
    lines = []

    def log(line):
        lines.append(line)
        print(line, flush=True)

    papers = {}
    for n, q in enumerate(QUERIES, 1):
        run_query(n, q, papers, log)
    order = sorted(papers.values(), key=lambda r: (r["published"], r["id"]))
    tier_a = [r for r in order if r["tier"] == "A"]
    tier_b = [r for r in order if r["tier"] == "B"]
    digest = hashlib.sha256("\n".join(sorted(r["id"] for r in tier_a)).encode("utf-8")).hexdigest()
    log("UNION " + str(len(order)) + " TIER_A " + str(len(tier_a)) + " TIER_B " + str(len(tier_b)))
    same = "matches" if digest == RECORDED_TIER_A_SHA256 else "differs from"
    log("TIER_A_SHA256 " + digest + " " + same + " the 2026-11-14 listing")
    write_jsonl(os.path.join(out, "metadata.jsonl"), tier_a)
    write_jsonl(os.path.join(out, "candidates-kg-rag.jsonl"), tier_b)
    with open(os.path.join(out, "queries.log"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    if "--list-only" in flags:
        return 0

    wanted = tier_a + (tier_b if "--tier-b" in flags else [])
    failures = []
    counts = {"kept": 0, "fetched": 0, "failed": 0}
    for i, rec in enumerate(wanted, 1):
        state, error = download(rec, pdf_dir)
        counts[state] += 1
        print(i, "/", len(wanted), rec["id"], state, error or "", flush=True)
        if error:
            failures.append({"id": rec["id"], "version": rec["version"], "error": error})
    write_jsonl(os.path.join(out, "failures.jsonl"), failures)
    with open(os.path.join(out, "SHA256SUMS"), "w", encoding="utf-8") as f:
        for name in sorted(os.listdir(pdf_dir)):
            if name.endswith(".pdf"):
                with open(os.path.join(pdf_dir, name), "rb") as p:
                    f.write(hashlib.sha256(p.read()).hexdigest() + "  pdf/" + name + "\n")
    print("DONE wanted", len(wanted), "kept", counts["kept"], "fetched", counts["fetched"], "failed", counts["failed"], flush=True)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
```

## Tier A: explicit graph RAG (271)

Abstract page: `https://arxiv.org/abs/<id>`. PDF: `https://arxiv.org/pdf/<id><version>`.

| # | arXiv id | Version | Published | Category | Title |
|---|---|---|---|---|---|
| 1 | 2601.00216 | v2 | 2026-01-01 | cs.CL | From Evidence-Based Medicine to Knowledge Graph: Retrieval-Augmented Generation for Sports Rehabilitation and a Domain Benchmark |
| 2 | 2601.00274 | v1 | 2026-01-01 | cs.CR | Making Theft Useless: Adulteration-Based Protection of Proprietary Knowledge Graphs in GraphRAG Systems |
| 3 | 2601.11587 | v1 | 2026-01-03 | cs.CY | Evidence-Grounded Multi-Agent Planning Support for Urban Carbon Governance via RAG |
| 4 | 2601.02814 | v1 | 2026-01-06 | cs.AI | Causal-Enhanced AI Agents for Medical Research Screening |
| 5 | 2601.03014 | v3 | 2026-01-06 | cs.CL | SentGraph: Hierarchical Sentence Graph for Multi-hop Retrieval-Augmented Question Answering |
| 6 | 2601.04945 | v1 | 2026-01-08 | cs.AI | T-Retriever: Tree-based Hierarchical Retrieval Augmented Generation for Textual Graphs |
| 7 | 2601.07192 | v1 | 2026-01-12 | cs.CL | Relink: Constructing Query-Driven Evidence Graph On-the-Fly for GraphRAG |
| 8 | 2601.08773 | v1 | 2026-01-13 | cs.SE | Reliable Graph-RAG for Codebases: AST-Derived Graphs vs LLM-Extracted Knowledge Graphs |
| 9 | 2601.09241 | v2 | 2026-01-14 | cs.CL | When to Trust: A Causality-Aware Calibration Framework for Accurate Knowledge Graph Retrieval-Augmented Generation |
| 10 | 2601.11144 | v3 | 2026-01-16 | cs.IR | Deep GraphRAG: A Balanced Approach to Hierarchical Retrieval and Adaptive Integration |
| 11 | 2602.00020 | v2 | 2026-01-16 | cs.CY | Beyond Static Question Banks: Dynamic Knowledge Expansion via LLM-Automated Graph Construction and Adaptive Generation |
| 12 | 2601.14662 | v2 | 2026-01-21 | cs.AI | Query-Efficient Agentic Graph Extraction Attacks on GraphRAG Systems |
| 13 | 2601.17755 | v3 | 2026-01-25 | cs.CL | HyperGraphPro: Progress-Aware Reinforcement Learning for Structure-Guided Hypergraph RAG |
| 14 | 2601.18381 | v2 | 2026-01-26 | cs.AI | AI Agent for Reverse-Engineering Legacy Finite-Difference Code and Translating to Devito |
| 15 | 2601.18579 | v2 | 2026-01-26 | cs.IR | GraFine: Retrieval-Time Refinement for Efficient Graph RAG over Corpus Graphs |
| 16 | 2603.28773 | v2 | 2026-01-28 | cs.IR | UltRAG: a Universal Simple Scalable Recipe for Knowledge Graph RAG |
| 17 | 2601.21162 | v1 | 2026-01-29 | cs.IR | A2RAG: Adaptive Agentic Graph Retrieval for Cost-Aware and Reliable Reasoning |
| 18 | 2602.16715 | v2 | 2026-01-30 | cs.AI | Retrieval Augmented (Knowledge Graph), and Large Language Model-Driven Design Structure Matrix (DSM) Generation of Cyber-Physical Systems |
| 19 | 2604.09868 | v1 | 2026-01-31 | cs.IR | Exploring Structural Complexity in Normative RAG with Graph-based approaches: A case study on the ETSI Standards |
| 20 | 2602.01858 | v1 | 2026-02-02 | cs.AI | SOPRAG: Multi-view Graph Experts Retrieval for Industrial Standard Operating Procedures |
| 21 | 2602.02053 | v2 | 2026-02-02 | cs.CL | WildGraphBench: Benchmarking GraphRAG with Wild-Source Corpora |
| 22 | 2602.02635 | v1 | 2026-02-02 | cs.CL | Graph-Augmented Reasoning with Large Language Models for Tobacco Pest and Disease Management |
| 23 | 2602.03578 | v1 | 2026-02-03 | cs.CL | Use Graph When It Needs: Efficiently and Adaptively Integrating Retrieval-Augmented Generation with Graphs |
| 24 | 2602.04926 | v2 | 2026-02-04 | cs.DB | AutoPrunedRetriever: Persistent Reasoning Graphs for Efficient and Extensible Retrieval-Augmented Generation |
| 25 | 2602.05143 | v2 | 2026-02-04 | cs.AI | CausalRAG2: Hierarchical Causal Knowledge Graph Design for RAG |
| 26 | 2602.06495 | v1 | 2026-02-06 | cs.CR | Graphs Don't Stay Secret: Practical Subgraph Reconstruction Attacks on Defended Graph RAG |
| 27 | 2603.03296 | v1 | 2026-02-06 | cs.CL | PlugMem: A Task-Agnostic Plugin Memory Module for LLM Agents |
| 28 | 2603.29875 | v3 | 2026-02-06 | cs.IR | UnWeaving the knots of GraphRAG -- turns out VectorRAG is almost enough |
| 29 | 2602.08400 | v1 | 2026-02-09 | cs.AI | SCOUT-RAG: Scalable and Cost-Efficient Unifying Traversal for Agentic Graph-RAG over Distributed Domains |
| 30 | 2602.08545 | v1 | 2026-02-09 | cs.IR | DA-RAG: Dynamic Attributed Community Search for Retrieval-Augmented Generation |
| 31 | 2604.20844 | v1 | 2026-02-10 | cs.IR | AtomicRAG: Atom-Entity Graphs for Retrieval-Augmented Generation |
| 32 | 2602.14470 | v1 | 2026-02-16 | cs.CL | HyperRAG: Reasoning N-ary Facts over Hypergraphs for Retrieval Augmented Generation |
| 33 | 2602.15313 | v2 | 2026-02-17 | cs.CL | Mnemis: Dual-Route Retrieval on Hierarchical Graphs for Long-Term LLM Memory |
| 34 | 2602.16650 | v1 | 2026-02-18 | cs.CE | Retrieval Augmented Generation of Literature-derived Polymer Knowledge: The Example of a Biodegradable Polymer Expert System |
| 35 | 2602.17949 | v2 | 2026-02-20 | cs.CL | CUICurate: A GraphRAG-based Framework for Automated Clinical Concept Curation for NLP applications |
| 36 | 2602.20926 | v1 | 2026-02-24 | cs.AI | HELP: HyperNode Expansion and Logical Path-Guided Evidence Localization for Accurate and Efficient GraphRAG |
| 37 | 2602.22584 | v1 | 2026-02-26 | cs.CL | Towards Faithful Industrial RAG: A Reinforced Co-adaptation Framework for Advertising QA |
| 38 | 2602.23719 | v1 | 2026-02-27 | cs.RO | SAGE-LLM: Towards Safe and Generalizable LLM Controller with Fuzzy-CBF Verification and Graph-Structured Knowledge Retrieval for UAV Decision |
| 39 | 2603.05529 | v2 | 2026-02-27 | cs.DB | NGDBench: Towards Neural Graph Data Management |
| 40 | 2603.00460 | v1 | 2026-02-28 | cs.AI | MED-COPILOT: A Medical Assistant Powered by GraphRAG and Similar Patient Case Retrieval |
| 41 | 2603.19276 | v1 | 2026-02-28 | cs.CL | From Flat to Structural: Enhancing Automated Short Answer Grading with GraphRAG |
| 42 | 2603.01410 | v2 | 2026-03-02 | cs.AI | GraphScout: Empowering Large Language Models with Intrinsic Exploration Ability for Agentic Graph Reasoning |
| 43 | 2604.20859 | v1 | 2026-03-02 | cs.IR | KGiRAG: An Iterative GraphRAG Approach for Responding Sensemaking Queries |
| 44 | 2603.03175 | v1 | 2026-03-03 | cs.AI | Saarthi for AGI: Towards Domain-Specific General Intelligence for Formal Verification |
| 45 | 2603.13307 | v1 | 2026-03-03 | cs.IR | Suppressing Domain-Specific Hallucination in Construction LLMs: A Knowledge Graph Foundation for GraphRAG and QLoRA on River and Sediment Control Technical Standards |
| 46 | 2603.05207 | v2 | 2026-03-05 | cs.IR | Core-based Hierarchies for Efficient GraphRAG |
| 47 | 2603.05698 | v2 | 2026-03-05 | cs.CL | Towards Robust Retrieval-Augmented Generation Based on Knowledge Graph: A Comparative Analysis |
| 48 | 2603.06290 | v1 | 2026-03-06 | cs.AI | The EpisTwin: A Knowledge Graph-Grounded Neuro-Symbolic Architecture for Personal AI |
| 49 | 2603.07179 | v1 | 2026-03-07 | cs.IR | Retrieving Minimal and Sufficient Reasoning Subgraphs with Graph Foundation Models for Path-aware GraphRAG |
| 50 | 2603.08965 | v2 | 2026-03-09 | cs.LG | Semantic Level of Detail for Knowledge Graphs: Discovering Abstraction Boundaries via Spectral Heat Diffusion |
| 51 | 2603.20252 | v1 | 2026-03-11 | cs.CL | FinReflectKG -- HalluBench: GraphRAG Hallucination Benchmark for Financial Question Answering Systems |
| 52 | 2603.11501 | v2 | 2026-03-12 | cs.LG | KEPo: Knowledge Evolution Poison on Graph-based Retrieval-Augmented Generation |
| 53 | 2603.11798 | v1 | 2026-03-12 | cs.AI | DocSage: An Information Structuring Agent for Multi-Doc Multi-Entity Question Answering |
| 54 | 2603.13185 | v1 | 2026-03-13 | cs.CV | Towards Spatio-Temporal World Scene Graph Generation from Monocular Videos |
| 55 | 2603.14006 | v2 | 2026-03-14 | cs.CL | Beyond Explicit Edges: Robust Reasoning over Noisy and Sparse Knowledge Graphs |
| 56 | 2603.14045 | v2 | 2026-03-14 | cs.IR | The Reasoning Bottleneck in Graph-RAG: Structured Prompting and Context Compression for Multi-Hop QA |
| 57 | 2603.14828 | v3 | 2026-03-16 | cs.IR | Don't Worry About Quality: Robust Multi-Hop Retrieval despite Issues in Knowledge Graphs |
| 58 | 2604.16350 | v1 | 2026-03-16 | cs.IR | LiteSemRAG: Lightweight LLM-Free Semantic-Aware Graph Retrieval for Robust RAG |
| 59 | 2603.20309 | v1 | 2026-03-19 | cs.IR | BubbleRAG: Evidence-Driven Retrieval-Augmented Generation for Black-Box Knowledge Graphs |
| 60 | 2603.22340 | v1 | 2026-03-21 | cs.IR | Graphs RAG at Scale: Beyond Retrieval-Augmented Generation With Labeled Property Graphs and Resource Description Framework for Complex and Unknown Search Spaces |
| 61 | 2603.22528 | v1 | 2026-03-23 | cs.IR | GraphRAG for Engineering Diagrams: ChatP&ID Enables LLM Interaction with P&IDs |
| 62 | 2603.23554 | v1 | 2026-03-23 | cs.IR | Mixture of Demonstrations for Textual Graph Understanding and Question Answering |
| 63 | 2603.25152 | v3 | 2026-03-26 | cs.AI | OMD-GraphRAG: Enhancing GraphRAG with Ontology-Guided Extraction, Multi-Dimensional Clustering and Dual-Channel Fusion |
| 64 | 2604.20869 | v1 | 2026-03-27 | cs.CY | Clinical Reasoning AI for Oncology Treatment Planning: A Multi-Specialty Case-Based Evaluation |
| 65 | 2604.19776 | v1 | 2026-03-28 | cs.CL | Development and Preliminary Evaluation of a Domain-Specific Large Language Model for Tuberculosis Care in South Africa |
| 66 | 2604.04948 | v2 | 2026-03-30 | cs.IR | From PDF to RAG-Ready: Evaluating Document Conversion Frameworks for Domain-Specific Question Answering |
| 67 | 2604.16401 | v1 | 2026-03-31 | cs.IR | GraphRAG-Router: Learning Cost-Efficient Routing over GraphRAGs and LLMs with Reinforcement Learning |
| 68 | 2604.00556 | v1 | 2026-04-01 | cs.LG | HabitatAgent: An End-to-End Multi-Agent System for Housing Consultation |
| 69 | 2604.09666 | v1 | 2026-04-01 | cs.IR | Do We Still Need GraphRAG? Benchmarking RAG and GraphRAG for Agentic Search Systems |
| 70 | 2604.02545 | v1 | 2026-04-02 | cs.AI | Competency Questions as Executable Plans: a Controlled RAG Architecture for Cultural Heritage Storytelling |
| 71 | 2604.02954 | v1 | 2026-04-03 | cs.CL | LogicPoison: Logical Attacks on Graph Retrieval-Augmented Generation |
| 72 | 2604.03174 | v1 | 2026-04-03 | cs.CL | Beyond the Parameters: A Technical Survey of Contextual Enrichment in Large Language Models: From In-Context Prompting to Causal Retrieval-Augmented Generation |
| 73 | 2604.09682 | v1 | 2026-04-03 | cs.NI | Decision-Theoretic Safety Assessment of Persona-Driven Multi-Agent Systems in O-RAN |
| 74 | 2604.16422 | v1 | 2026-04-03 | cs.CL | Injecting Structured Biomedical Knowledge into Language Models: Continual Pretraining vs. GraphRAG |
| 75 | 2605.18760 | v2 | 2026-04-06 | cs.IR | DOTRAG: Retrieval-Time Reasoning Along Paths |
| 76 | 2604.05846 | v2 | 2026-04-07 | cs.CL | AgentGL: Towards Agentic Graph Learning with LLMs via Reinforcement Learning |
| 77 | 2604.06279 | v1 | 2026-04-07 | physics.plasm-ph | Plasma GraphRAG: Physics-Grounded Parameter Selection for Gyrokinetic Simulations |
| 78 | 2605.18765 | v1 | 2026-04-11 | cs.IR | STAR: Semantic-Tuned and Tail-Adaptive Retriever for Graph-Augmented Generation |
| 79 | 2604.10426 | v1 | 2026-04-12 | cs.CL | CodaRAG: Connecting the Dots with Associativity Inspired by Complementary Learning |
| 80 | 2604.14220 | v1 | 2026-04-14 | cs.IR | Knowledge Graph RAG: Agentic Crawling and Graph Construction in Enterprise Documents |
| 81 | 2605.18770 | v3 | 2026-04-15 | cs.IR | Agentic Graph Retrieval-Augmented Generation for Auditable Commercial Registry Analysis |
| 82 | 2606.00029 | v1 | 2026-04-15 | cs.CL | TCAR-Gen: Temporal Graph Retrieval with Evidence Fusion for Knowledge-Grounded Generation |
| 83 | 2604.15676 | v2 | 2026-04-17 | cs.DB | EvoRAG: Making Knowledge Graph-based RAG Automatically Evolve through Feedback Backpropagation |
| 84 | 2604.17056 | v1 | 2026-04-18 | cs.IR | RLM-on-KG: Heuristics First, LLMs When Needed: Adaptive Retrieval Control over Mention Graphs for Scattered Evidence |
| 85 | 2604.17458 | v2 | 2026-04-19 | cs.AI | EHRAG: Bridging Semantic Gaps in Lightweight GraphRAG via Hybrid Hypergraph Construction and Retrieval |
| 86 | 2604.19128 | v1 | 2026-04-21 | cs.IR | GraphRAG-IRL: Personalized Recommendation with Graph-Grounded Inverse Reinforcement Learning and LLM Re-ranking |
| 87 | 2605.18775 | v1 | 2026-04-21 | cs.IR | Query-Aware Flow Diffusion for Graph-Based RAG with Retrieval Guarantees |
| 88 | 2604.20487 | v2 | 2026-04-22 | cs.CL | Knowledge Capsules: Structured Nonparametric Memory Units for LLMs |
| 89 | 2604.23585 | v1 | 2026-04-26 | cs.CL | ComplianceNLP: Knowledge-Graph-Augmented RAG for Multi-Framework Regulatory Gap Detection |
| 90 | 2604.24623 | v1 | 2026-04-27 | cs.AI | XGRAG: A Graph-Native Framework for Explaining KG-based Retrieval-Augmented Generation |
| 91 | 2605.03476 | v1 | 2026-05-05 | cs.CL | CuraView: A Multi-Agent Framework for Medical Hallucination Detection with GraphRAG-Enhanced Knowledge Verification |
| 92 | 2605.05643 | v1 | 2026-05-07 | cs.AI | Text-Graph Synergy: A Bidirectional Verification and Completion Framework for RAG |
| 93 | 2605.06403 | v1 | 2026-05-07 | cs.CL | GATHER: Convergence-Centric Hyper-Entity Retrieval for Zero-Shot Cell-Type Annotation |
| 94 | 2605.08997 | v1 | 2026-05-09 | eess.SP | SEM-RAG: Structure-Preserving Multimodal Graph Compilation and Entropy-Guided Retrieval for Telecommunication Standards |
| 95 | 2607.16208 | v1 | 2026-05-09 | cs.AI | ColGraphRAG: Late-Interaction Evidence Retrieval for Multimodal GraphRAG |
| 96 | 2605.09505 | v2 | 2026-05-10 | cs.AI | EpiGraph: Building Generalists for Evidence-Intensive Epilepsy Reasoning in the Wild |
| 97 | 2605.10247 | v2 | 2026-05-11 | cs.LG | Teaching LLMs to See Graphs: Unifying Text and Structural Reasoning |
| 98 | 2605.12061 | v2 | 2026-05-12 | cs.AI | SAGE: A Self-Evolving Agentic Graph-Memory Engine for Structure-Aware Associative Memory |
| 99 | 2605.13481 | v5 | 2026-05-13 | cs.CL | PersonalAI 2.0: Enhancing knowledge graph traversal/retrieval with planning mechanism for Personalized LLM Agents |
| 100 | 2605.14034 | v1 | 2026-05-13 | cs.AI | From Descriptive to Prescriptive: Uncover the Social Value Alignment of LLM-based Agents |
| 101 | 2605.14177 | v1 | 2026-05-13 | cs.IR | Thinking Ahead: Prospection-Guided Retrieval of Memory with Language Models |
| 102 | 2605.14259 | v3 | 2026-05-14 | cs.AI | Hypergraph Enterprise Agentic Reasoner over Heterogeneous Business Systems |
| 103 | 2605.14362 | v1 | 2026-05-14 | cs.SE | Correctness-Aware Repository Filtering Under Maximum Effective Context Window Constraints |
| 104 | 2605.15109 | v1 | 2026-05-14 | cs.AI | Why Neighborhoods Matter: Traversal Context and Provenance in Agentic GraphRAG |
| 105 | 2605.16676 | v1 | 2026-05-15 | cs.AI | Enhancing Metacognitive AI: Knowledge-Graph Population with Graph-Theoretic LLM Enrichment |
| 106 | 2605.18410 | v1 | 2026-05-18 | cs.DL | From Node2Vec to GPT-based GraphRAG: scientific impact prediction across graph and language models |
| 107 | 2605.19735 | v1 | 2026-05-19 | cs.CL | ContextRAG: Extraction-Free Hierarchical Graph Construction for Retrieval-Augmented Generation |
| 108 | 2606.00062 | v1 | 2026-05-19 | cs.CL | Graph-Augmented Retrieval for Cross-Entity Financial Sentiment Analysis: A Comparative Study |
| 109 | 2605.20815 | v1 | 2026-05-20 | cs.CL | GraphRAG on Consumer Hardware: Benchmarking Local LLMs for Healthcare EHR Schema Retrieval |
| 110 | 2608.05153 | v1 | 2026-05-20 | cs.CL | Universal Pathologies, Conditional Consequences: A Triple-Robustness Analysis of RAG for Multi-Hop Traceability |
| 111 | 2605.21974 | v1 | 2026-05-21 | cs.CL | Format-Constraint Coupling in Knowledge Graph Construction from Statistical Tables |
| 112 | 2605.21994 | v1 | 2026-05-21 | cs.LG | Ex-GraphRAG: Interpretable Evidence Routing for Graph-Augmented LLMs |
| 113 | 2605.24366 | v1 | 2026-05-23 | cs.CL | Structure-Aware RAG: Structured Retrieval Augmented Generation from Noisy Data for Conversational Agents |
| 114 | 2605.25480 | v2 | 2026-05-25 | cs.CL | Retrieval as Reasoning: Self-Evolving Agent-Native Retrieval via LLM-Wiki |
| 115 | 2605.27164 | v1 | 2026-05-26 | cs.AI | Query Symbolically or Retrieve Semantically? A Dataset and Method for Semi-Structured Question Answering |
| 116 | 2605.28004 | v1 | 2026-05-27 | cs.CL | Beyond Chunk-Local Extraction: Cross-Chunk Graph Augmentation for GraphRAG |
| 117 | 2605.28120 | v1 | 2026-05-27 | cs.CL | LegalGraphRAG: Multi-Agent Graph Retrieval-Augmented Generation for Reliable Legal Reasoning |
| 118 | 2605.28175 | v2 | 2026-05-27 | cs.IR | Mixture-of-Experts Knowledge Graph Retrieval-Augmented Generation for Multi-Agent LLM-based Recommendation |
| 119 | 2605.28645 | v1 | 2026-05-27 | cs.CR | GraphSteal: Structural Knowledge Stealing from Graph RAG via Traversal Reconstruction |
| 120 | 2605.29168 | v2 | 2026-05-27 | cs.AI | Better Later Than Sooner: Neuro-Symbolic Knowledge Graph Construction via Ontology-grounded Post-extraction Correction |
| 121 | 2605.31010 | v1 | 2026-05-29 | cs.CL | MoG: Mixture of Experts for Graph-based Retrieval-Augmented Generation |
| 122 | 2606.00610 | v1 | 2026-05-30 | cs.IR | MemGraphRAG: Memory-based Multi-Agent System for Graph Retrieval-Augmented Generation |
| 123 | 2606.01394 | v1 | 2026-05-31 | cs.CL | UniD$^3$: A Knowledge Graph-Enhanced RAG Framework for Drug-Disease Discovery and Reasoning |
| 124 | 2606.03460 | v1 | 2026-06-02 | cs.CV | From 3D Perception to Safety Reasoning: A Graph-Based Framework for Real-Time Underground Mine Monitoring |
| 125 | 2607.22571 | v1 | 2026-06-02 | cs.AI | SCAIR: Schema-Conditioned Agentic Iterative Reasoning for Enterprise Knowledge Graphs |
| 126 | 2606.04646 | v2 | 2026-06-03 | cs.CL | QO-Bench: Diagnosing Query-Operator-Preserving Retrieval over Typed Event Tuples |
| 127 | 2606.28351 | v3 | 2026-06-03 | cs.IR | HyperSU: Corpus-Driven Semantic-Unit Hypergraph for Retrieval-Augmented Generation |
| 128 | 2608.12335 | v1 | 2026-06-03 | cs.CL | HC-RAG: Evidence-Centric Retrieval-Augmented Generation over Heterogeneous Financial Filings |
| 129 | 2606.05901 | v2 | 2026-06-04 | cs.CL | Reducing Hallucinations in Complex Question Answering using Simple Graph-based Retrieval-Augmented Generation (long version) |
| 130 | 2606.06044 | v1 | 2026-06-04 | cs.CL | IA-RAG: Interval-Algebra-Driven Temporal Reasoning for Dynamic Knowledge Retrieval |
| 131 | 2606.07850 | v2 | 2026-06-05 | physics.comp-ph | PDE-Agents: An LLM-Orchestrated Multi-Agent Framework for Automated Finite Element Simulations with Knowledge Graph-Augmented Reasoning |
| 132 | 2607.19362 | v1 | 2026-06-05 | cs.AI | GraphContainer: A Unified Platform for Comparing and Debugging Graph RAG Methods |
| 133 | 2606.08944 | v1 | 2026-06-08 | cs.AR | LongRTL: Graph-Similarity-Guided LLM-driven Long Context RTL Optimization |
| 134 | 2606.10177 | v2 | 2026-06-08 | cs.HC | VArify: A Visual Analytics System for Verifying Knowledge Enhanced Large Language Model Responses in Food Science |
| 135 | 2608.00006 | v1 | 2026-06-08 | cs.AI | Enhancing LLMs with Context-Specific Knowledge for Mitigating Misinformation in SMEs: A RAG-based Modeling and Analysis |
| 136 | 2606.11562 | v1 | 2026-06-10 | cs.LG | GraphInfer-Bench: Benchmarking LLM's Inference Capability on Graphs |
| 137 | 2607.22592 | v1 | 2026-06-10 | cs.AI | Structure Over Scale: Schema-Constrained Causal Graphs for RAG |
| 138 | 2607.22597 | v1 | 2026-06-12 | cs.AI | HyCE-RAG: Hypergraph Chain-of-Evidence Retrieval-Augmented Generation for Explainable Multi-hop Question Answering |
| 139 | 2606.16409 | v1 | 2026-06-15 | cs.CL | PathRouter: Aligning Rewards with Retrieval Quality in Agentic Graph Retrieval-Augmented Generation |
| 140 | 2606.17856 | v1 | 2026-06-16 | cs.AI | FlowRAG: Synergizing Explicit Reasoning via Frequency-Aware Multi-Granularity Graph Flow |
| 141 | 2606.18075 | v1 | 2026-06-16 | cs.AI | A Unified Framework for Context-Aware and Relation-Aware Graph Retrieval-Augmented Generation |
| 142 | 2606.22180 | v3 | 2026-06-20 | cs.DC | FeLoG: Scalable and Efficient Distributed Graph Embedding with Feedback Loop Mechanism |
| 143 | 2606.22728 | v1 | 2026-06-22 | cs.CL | When Confidence Takes the Wrong Path: Diagnosing Retrieval-State Lock-In in RAG |
| 144 | 2606.23108 | v1 | 2026-06-22 | cs.AI | TTFT-Aware Graph Chain-of-Thought:Distance-Indexed Neural A* for Low-Hallucination Multi-Hop Medical Reasoning |
| 145 | 2606.25656 | v1 | 2026-06-24 | cs.CL | Is GraphRAG Needed? From Basic RAG to Graph-/Agentic Solutions with Context Optimization |
| 146 | 2606.26458 | v1 | 2026-06-24 | cs.AI | MKG-RAG-Bench: Benchmarking Retrieval in Multimodal Knowledge Graph-Augmented Generation |
| 147 | 2606.28011 | v1 | 2026-06-26 | eess.SY | From Detection to Action: Using LLM Agents for Fault-Tolerant Control |
| 148 | 2606.28780 | v2 | 2026-06-27 | cs.IR | Multimodal Graph RAG for Long-range Visually Rich Document Understanding |
| 149 | 2606.29399 | v1 | 2026-06-28 | cs.AI | LLM-Guided Planning for Multi-hop Reasoning over Multimodal Nuclear Regulatory Documents |
| 150 | 2606.30093 | v1 | 2026-06-29 | cs.CL | Efficient Retrieval-Augmented Generation via Token Co-occurrence Graphs |
| 151 | 2606.30133 | v1 | 2026-06-29 | cs.LG | Query-Aware Spreading Activation for Multi-Hop Retrieval over Knowledge Graphs |
| 152 | 2606.30247 | v1 | 2026-06-29 | cs.CL | Grounding LLM Reasoning under Incomplete Graph Evidence |
| 153 | 2607.00052 | v1 | 2026-06-30 | cs.IR | AGE: Adaptive-masking for Graph Embedding in Graph Retrieval-Augmented Generation |
| 154 | 2609.01617 | v1 | 2026-06-30 | cs.IR | Hybrid Retrieval-Augmented Generation with Knowledge Graph Expansion, RRF Fusion, and Per-Chunk Grounded Evaluation for Enterprise Document Search |
| 155 | 2607.00508 | v2 | 2026-07-01 | cs.IR | When RAG Meets Query Planning: Logical Query Trees for Resolving Exploratory Reasoning Problems |
| 156 | 2609.02894 | v1 | 2026-07-01 | cs.CL | R$^{2}$Adapter: A Routing and Rewriting Adapter for Efficient Hybrid RAG |
| 157 | 2607.20506 | v1 | 2026-07-02 | cs.AI | Optimizing Hypergraph-Based RAG: Toward Better Fact Extraction and Chunk Retrieval |
| 158 | 2607.03447 | v1 | 2026-07-03 | cs.IR | TRIAGE: Trustworthy Retrieval Instrumentation And Graph Evaluation |
| 159 | 2607.19398 | v1 | 2026-07-04 | cs.AI | HyGRL: Adaptive Hybrid Graph Reasoning for Multi-Entity Questions |
| 160 | 2607.05577 | v1 | 2026-07-06 | cs.AI | Narrative World Model: Narratology-Grounded Writer Memory for Long-Form Fiction |
| 161 | 2608.28607 | v1 | 2026-07-08 | cs.AI | RegDivergence-101: An LLM Benchmark for Cross-Jurisdiction Regulatory Contradiction Detection in Life Sciences |
| 162 | 2607.08269 | v2 | 2026-07-09 | cs.AI | PolyUQuest: Verifiable Structure-Aware Web RAG over Heterogeneous Graphs |
| 163 | 2607.10265 | v2 | 2026-07-11 | cs.DB | TGMS: An Agent-Native Bi-Temporal Graph Management System |
| 164 | 2607.11159 | v1 | 2026-07-13 | cs.IR | NGM-RAG: Neural Graph Matching based Retrieval-Augmented Generation |
| 165 | 2607.11464 | v1 | 2026-07-13 | cs.IR | FAIR GraphRAG: A Retrieval-Augmented Generation Approach for Semantic Data Analysis |
| 166 | 2607.11683 | v1 | 2026-07-13 | cs.CL | RAGU: A Multi-Step GraphRAG Engine with a Compact Domain-Adapted LLM |
| 167 | 2607.12764 | v1 | 2026-07-14 | cs.CV | EvoGraph-R1: Self-Evolving Multimodal Knowledge Hypergraphs for Agentic Retrieval |
| 168 | 2607.14661 | v2 | 2026-07-16 | cs.AI | SmartRAG: Native Graph-Based RAG for Mobile Device |
| 169 | 2607.15516 | v1 | 2026-07-17 | cs.LG | Cache-Aware Prompt Compression:A Two-Tier Cost Model for LLM API Caching |
| 170 | 2607.19301 | v1 | 2026-07-21 | cs.IR | PAGE-RAG: Evidence-Grounded Adaptive Graph Retrieval for Long-Document Question Answering |
| 171 | 2607.22319 | v1 | 2026-07-24 | cs.DB | Towards Trustworthy and Cost-Efficient Data Integration: From Naïve RAG to Agentic RAG |
| 172 | 2607.23278 | v1 | 2026-07-25 | cs.CL | Co-Evolving Graph and Text Memory for Training-Free Multi-Hop Question Answering |
| 173 | 2607.24861 | v1 | 2026-07-26 | cs.IR | HVM-GraphRAG: Holistic-View Multimodal Graph Retrieval-Augmented Generation on Complex Document |
| 174 | 2607.24475 | v1 | 2026-07-27 | cs.IR | Robust Interpretation of Historical Documents in Knowledge Graphs Through Query Inference and Execution |
| 175 | 2607.24554 | v1 | 2026-07-27 | cs.IR | DeCoRAG: Cognitive Decoupling and Semantic-Aware Cropping for Complex Document Understanding |
| 176 | 2607.24663 | v1 | 2026-07-27 | physics.acc-ph | A corrective agentic hybrid RAG and an operations-grounded evaluation for a scientific facility |
| 177 | 2607.24984 | v2 | 2026-07-27 | physics.geo-ph | Automatic Knowledge Graph Construction and Query for Earthquake Catalogs |
| 178 | 2607.26497 | v3 | 2026-07-29 | cs.CL | BM25 Wins at Scale: A Scaling Study of Retrieval-Augmented Generation Paradigms |
| 179 | 2607.27136 | v1 | 2026-07-29 | cs.IR | KAMR: Grounding Generation via Knowledge-Aligned Multi-hop Retrieval |
| 180 | 2607.27918 | v1 | 2026-07-30 | math.OC | OptGraph: Large Language Models Enhanced Evolutionary Optimization Via Graph Retrieval-Augmented Generation |
| 181 | 2607.28397 | v2 | 2026-07-30 | cs.AI | GLM-RAG: Graph Language Models for Graph-Based Retrieval-Augmented Generation |
| 182 | 2608.00705 | v1 | 2026-08-01 | cs.IR | A Triple-Robustness Analysis of Retrieval-Augmented Generation for Multi-Hop Requirements Traceability |
| 183 | 2608.01269 | v2 | 2026-08-02 | cs.CL | ACE-GraphRAG: Agentic Context Engineering for Hierarchical GraphRAG |
| 184 | 2608.01565 | v1 | 2026-08-03 | cs.CL | DocNavRAG: Document-Structured Graph RAG with Stateful Evidence Construction for Complex Document Question Answering |
| 185 | 2608.01732 | v1 | 2026-08-03 | cs.IR | X-KGRank: A Knowledge Graph RAG Framework for Explainable Recommendations via Pattern Mining and LLM Re-Ranking |
| 186 | 2608.07994 | v2 | 2026-08-08 | cs.AI | VDGR-RAG: Vectors, Directories, Graphs, and Reflection Are All You Need for Unified Reasoning over Hierarchical Enterprise Knowledge |
| 187 | 2608.09016 | v1 | 2026-08-10 | cs.IR | PreGress: Ranking-Native Pre-training and Prompting for Graph Node Ranking |
| 188 | 2608.09214 | v1 | 2026-08-10 | cs.DB | AkasicDB: Demonstrating Omni RAG with a Unified Vector-Graph-Relational DBMS |
| 189 | 2608.09698 | v1 | 2026-08-10 | cs.HC | VeriForge: Mitigating Latent Knowledge Gaps in Narrative Drafting via Mixed-Initiative Scaffolding |
| 190 | 2608.11277 | v1 | 2026-08-11 | cs.CR | Knowledge-Graph-Guided Retrieval-Augmented LLMs for Explainable Root Cause Analysis in Automotive HiL Validation |
| 191 | 2608.28642 | v1 | 2026-08-12 | cs.AI | From Extraction to Governed Memory: Multi-Agent Knowledge Graph Construction with Domain-Expert Review |
| 192 | 2609.26237 | v1 | 2026-08-12 | cs.IR | ABAI at COLIEE 2026 Task 1: Multi-Stage Retrieval with GraphRAG-Enhanced Meta-Learning, and a Post-Hoc Study of the Cross-Validation-to-Test Gap |
| 193 | 2608.13050 | v1 | 2026-08-13 | cs.CR | Operationalizing Cyber Threat Intelligence with GraphRAG |
| 194 | 2608.13706 | v3 | 2026-08-13 | cs.CL | CLAIR-Fin: An Adversarial Multi-Agent Framework for Claim-Level Verification and Adaptive Debate in Cross-Modal Financial QA |
| 195 | 2608.24921 | v3 | 2026-08-14 | cs.AI | post-graph-rag: A PostgreSQL-Native Bi-Temporal Graph RAG Engine with Temporal Grounding at Synthesis |
| 196 | 2608.15056 | v1 | 2026-08-15 | cs.AI | GraphLoom: Reliability-Calibrated Graph Evidence Routing for Multimodal KG-RAG |
| 197 | 2608.15919 | v1 | 2026-08-16 | cs.IR | Noesis: Bidirectional Graph-RAG with Adaptive Parallelism and Cross-Knowledge-Base Semantic Discovery |
| 198 | 2608.16004 | v1 | 2026-08-17 | cs.IR | LineageRAG: Harnessing GraphRAG by Constructing Evidence Lineages with Source Grounding |
| 199 | 2608.16096 | v1 | 2026-08-17 | cs.IR | The Commercial Tax: Rent-vs-Own Blind Spots in Multi-Hop Retrieval Benchmarks |
| 200 | 2609.27814 | v1 | 2026-08-18 | cs.CL | LabourCrew: A Multi-Agent RAG Framework for Trustworthy Adversarial Deliberation and Statutory Reasoning over Labour Law |
| 201 | 2608.18489 | v1 | 2026-08-19 | cs.CL | MissDiag: Diagnostic Evaluation of Incomplete-Knowledge Robustness in KGQA and KG-RAG |
| 202 | 2608.18952 | v1 | 2026-08-19 | cs.IR | rEDMRec: Distilling Large Language Model Reasoning into an Editable Experience Memory for Recommendation |
| 203 | 2608.22062 | v1 | 2026-08-22 | cs.AI | Search Broadly, Seek Evidence on Both Sides, Decide Narrowly: Evidence-Admissible GraphRAG for Longitudinal Clinical Event Verification |
| 204 | 2608.22688 | v1 | 2026-08-24 | cs.IR | FashionKG-RAG: Knowledge Graph-Enhanced Retrieval-Augmented Generation for Fashion Question Answering |
| 205 | 2608.23011 | v1 | 2026-08-24 | cs.CV | Coarse Indexing, Fine Evidence: Decoupling Temporal Granularity in Long-Video RAG |
| 206 | 2608.25123 | v1 | 2026-08-25 | cs.CL | SelfGraphRAG: Bridging the Supervision Gap in Graph-Based RAG with Synthetic QA Generation |
| 207 | 2608.25489 | v4 | 2026-08-26 | cs.LG | A Storage-Retrieval Gap in Parametric Knowledge Graph Memory |
| 208 | 2608.25952 | v2 | 2026-08-26 | cs.CY | Spatial-Knowledge-Graph-Grounded LLM Agents for Neighborhood Livability Evaluation |
| 209 | 2608.25960 | v1 | 2026-08-26 | cs.AI | LivingRAG: Augmenting Graph RAG with Experience |
| 210 | 2608.25986 | v1 | 2026-08-26 | cs.AI | Multi-Granularity Context-Enhanced RAG over Multimodal Knowledge Graphs |
| 211 | 2608.28766 | v1 | 2026-08-28 | cs.SE | Beyond Vector Search: Comparing Classical RAG with Hybrid GraphRAG for Climate Science Q\&A |
| 212 | 2609.00050 | v2 | 2026-08-30 | cs.SE | Towards Agentic Cloud Engineering: Graph and Loop Engineering with a Zero-Trust Agent Harness |
| 213 | 2608.30468 | v1 | 2026-08-31 | cs.CL | Hi-Q: Hierarchical Evidence-guided Query Refinement for Multi-Hop Question Answering |
| 214 | 2609.29695 | v2 | 2026-08-31 | cs.IR | EvLink: Source-Grounded Evidence Linking for Graph RAG |
| 215 | 2609.00513 | v1 | 2026-09-01 | cs.AI | ISO-RAG: Isoperimetric Noise Control for Retrieval-Augmented Generation |
| 216 | 2609.00604 | v2 | 2026-09-01 | cs.CR | NeuroGraph: An AI Graph-Driven Neuro-Symbolic Framework for Explainable Threat Reasoning in Advanced Manufacturing |
| 217 | 2609.00761 | v1 | 2026-09-01 | cs.SI | Agent-Enhanced Heterogeneous Graph RAG for Academic Question Answering |
| 218 | 2609.02011 | v1 | 2026-09-02 | eess.SY | Seed-Anchored Budget-Bounded Graph Rendering for Question Answering on Industry-Standard Power-Grid Information and Exchange Models |
| 219 | 2609.03857 | v1 | 2026-09-03 | cs.IR | GRASP: Graph-Retrieval Automated Scoring Pipeline for Label-Free Multi-Topic Essay Grading |
| 220 | 2609.05139 | v1 | 2026-09-04 | cs.CL | NS-ST-GraphRAG: Neuro-Symbolic Spatio-Temporal GraphRAG for Literary Knowledge Processing |
| 221 | 2609.10237 | v1 | 2026-09-09 | cs.CL | The Answer Path and the Grounding Instruction in LLM Question Answering over Knowledge Graphs |
| 222 | 2609.10239 | v1 | 2026-09-09 | cs.IR | LiteRAG: Cost-Efficient Graph-Based Retrieval-Augmented Generation |
| 223 | 2609.11065 | v1 | 2026-09-10 | cs.AI | MOSAIC: Query-Aware Exploration Policy Adaptation for GraphRAG |
| 224 | 2609.13324 | v1 | 2026-09-10 | cs.IR | Decoupling Error Attribution in Cloud-Native Graph-RAG: A Data Integrity Diagnostic Framework |
| 225 | 2609.12464 | v1 | 2026-09-11 | cs.AI | Beyond Vector Similarity: Hierarchical Context-Aware Graph RAG vs Standard RAG in Enterprise Code Migration |
| 226 | 2609.18099 | v1 | 2026-09-16 | cs.AI | When Is Graph Structure Worth Its Cost? The Case for Structure Pricing in Retrieval-Augmented Generation |
| 227 | 2609.18317 | v2 | 2026-09-16 | cs.CL | Knowledge-Graph Based Augmentation versus Retrieval Augmented Generation for Cultural-Related Question Answering |
| 228 | 2609.19417 | v1 | 2026-09-16 | cs.CL | Less Is More: Graph-free Multimodal RAG via Multi-signal Late Fusion |
| 229 | 2609.19622 | v2 | 2026-09-17 | cs.IR | Geometric Gain Graph: Zero-Token Graph Construction for Multi-Hop RAG |
| 230 | 2609.20754 | v1 | 2026-09-17 | cs.AI | RAFT: A Stateful Retrieval-Augmented Framework for Troubleshooting Agents |
| 231 | 2609.35810 | v1 | 2026-09-20 | cs.CL | TRACE: Deployable Tree-Relational Structure Enhancement for Oncology LLMs |
| 232 | 2609.24346 | v1 | 2026-09-21 | cs.AI | LADDER: Graph-Guided Diffusion Language Models for Efficient Multi-Hop Reasoning |
| 233 | 2609.27009 | v2 | 2026-09-22 | cs.CL | LEGO: Synergizing Expert GraphRAG and Expert Chain-of-Thought for Legal Reasoning |
| 234 | 2609.29083 | v1 | 2026-09-24 | cs.SI | Advancing the Physical Internet with GraphRAG: A New Way to Review and Integrate Existing Research |
| 235 | 2609.29282 | v2 | 2026-09-24 | cs.IR | Asymmetric Dynamic Routing: Balancing Reasoning Depth and Computational Efficiency in Hypergraph RAG |
| 236 | 2609.35304 | v1 | 2026-09-28 | cs.IR | Signal or Noise? Modality Contribution and Cooperation in Multimodal GraphRAG |
| 237 | 2609.36700 | v1 | 2026-09-29 | cs.CL | Lost in Conversation or Lost in Translation? Diagnosing Multi-Turn Degradation in RAG |
| 238 | 2609.37661 | v1 | 2026-09-29 | cs.CL | Corpus-Guided Dual-Path Propagation for Graph Retrieval-Augmented Generation |
| 239 | 2610.02373 | v1 | 2026-10-01 | cs.CR | Hop-Decayed Influence: New Vulnerabilities of Structural Auxiliary Indexing in GraphRAG Pipelines with LLM |
| 240 | 2610.09026 | v1 | 2026-10-06 | cs.AI | BEACON-SP: Ontology-Grounded GraphRAG Framework for Clinical Suicide Risk Assessment |
| 241 | 2610.09360 | v1 | 2026-10-07 | cs.AI | TopoGraphRAG-Bench: Evaluating Multimodal GraphRAG on Layout-Grounded Evidence Reasoning |
| 242 | 2610.13059 | v1 | 2026-10-09 | cs.CL | CliQ-RRG: Clinical-Knowledge Guided Disease-aware Visual-Textual Alignment for QA-Style Radiology Report Generation |
| 243 | 2610.13128 | v1 | 2026-10-09 | cs.AI | Locate, Judge, Read: Role-Specialized Tool Agents over a Knowledge-Point Index for Open-Domain Multimodal Document QA |
| 244 | 2610.13296 | v1 | 2026-10-09 | cs.IR | PRIME: Pre-Retrieval Index Modeling Enhancement for Structured Document Retrieval-Real |
| 245 | 2610.14968 | v1 | 2026-10-10 | cs.CR | SEEK: Agentic Knowledge Graph Retrieval for Secure Code Generation |
| 246 | 2610.19341 | v1 | 2026-10-14 | cs.CV | MMKG-R1: Constructing Multimodal Knowledge Graphs via Reinforcement Learning |
| 247 | 2610.19549 | v1 | 2026-10-14 | cs.CL | GraphRAG-LTM: A Layered Threat Modeling Framework for Attacks on Graph Retrieval-Augmented Generation |
| 248 | 2610.22818 | v1 | 2026-10-16 | cs.DB | FuseRAG: Fusing Structure and Evidence into Inference for Cost-Efficient Graph-based RAG |
| 249 | 2610.23559 | v1 | 2026-10-16 | cs.SE | An Interactive GraphRAG Workspace for Reifying, Auditing, and Steering Conversational AI Memories |
| 250 | 2610.26432 | v1 | 2026-10-19 | cs.AR | E-Layout: Layout-Stage Logic Restructuring via RL-Optimized Equality-Saturated Graph Retrieval-Augmented Generation |
| 251 | 2610.28216 | v1 | 2026-10-20 | physics.app-ph | A Multimodal Engineering GraphRAG Framework for Post-Wind-Event Building Assessment and Recovery Inference |
| 252 | 2610.30611 | v1 | 2026-10-22 | cs.AI | Better Reasoning, Not Just Bigger Models: A Structure-Grounded Framework for Efficient and Reliable Inference |
| 253 | 2610.30710 | v1 | 2026-10-22 | cs.IR | Automated Enrichment Analysis with GraphRAG: Efficient Multi-Database ORA Extraction and Biological Interpretation |
| 254 | 2610.30728 | v1 | 2026-10-22 | cs.LG | Federated Graph Retrieval-Augmented Generation |
| 255 | 2610.33277 | v1 | 2026-10-23 | cs.AI | GEMS-RAG: Graph-grounded Engineering Multimodal Standards Retrieval-Augmented Generation |
| 256 | 2610.33511 | v1 | 2026-10-24 | cs.CL | Towards a Learnable Retriever for Textual Graph Question Answering |
| 257 | 2610.33946 | v1 | 2026-10-24 | cs.CL | Schema-Free Text-to-SQL: Learning Enterprise Database Structure from Query Logs |
| 258 | 2610.34866 | v1 | 2026-10-26 | cs.AI | UniGFM-RAG: Unify Graph Foundation Models and Large Language Models for End-to-End Retrieval and Reasoning |
| 259 | 2610.35412 | v1 | 2026-10-26 | cs.IR | Read the Route: Attention-Steered Hypergraph Retrieval for Multi-hop RAG |
| 260 | 2610.36650 | v1 | 2026-10-27 | cs.LG | TACO-Graph: Automated Construction of Text-Atributed Graphs |
| 261 | 2610.37237 | v1 | 2026-10-27 | cs.AI | GraphRAGProv: GraphRAG Meets How-Provenance |
| 262 | 2610.37728 | v1 | 2026-10-27 | cs.IR | Beyond Similarity Search: Knowledge Graph Metadata Enrichment for Enterprise Retrieval-Augmented Generation |
| 263 | 2610.37826 | v2 | 2026-10-27 | cs.CL | DeFT-RAG: Decomposing Multi-Hop Questions into Self-Contained Sub-Questions with Reusable Answers |
| 264 | 2610.40971 | v1 | 2026-10-29 | cs.CV | FieldGuideRAG: Image-Grounded GraphRAG for Fine-Grained Recognition with Vision-Language Models |
| 265 | 2610.41390 | v1 | 2026-10-30 | cs.SD | Hierarchical Dual-Graph Retrieval for Long-form Audio Reasoning |
| 266 | 2611.06481 | v1 | 2026-11-04 | cs.CL | ArcRAG: Agentic Compositional Retrieval over Knowledge Graphs |
| 267 | 2611.09414 | v1 | 2026-11-06 | cs.AI | Typed-Centrality Grounding for Agentic GraphRAG: A Symbolic Approach for Agentic Reasoning |
| 268 | 2611.09658 | v1 | 2026-11-06 | cs.CR | Differentially Private GraphRAG: Protecting Relation Connectivity and Semantics |
| 269 | 2611.10906 | v1 | 2026-11-07 | cs.CL | SCorPion: A Strategic, Controllable and Personified Conversational Companion |
| 270 | 2611.11010 | v1 | 2026-11-07 | cs.AI | Lightfoot: Graph Retrieval-Augmented Generation with Exact, Model-Free Document Deletion |
| 271 | 2611.15467 | v1 | 2026-11-11 | cs.CL | AutoOntic: Automatic Ontology Induction from Text for Knowledge Graph Construction |

## Tier B: knowledge-graph RAG candidates (158)

| # | arXiv id | Version | Published | Category | Title |
|---|---|---|---|---|---|
| 1 | 2601.01844 | v1 | 2026-01-05 | cs.AI | Clinical Knowledge Graph Construction and Evaluation with Multi-LLMs via Retrieval-Augmented Generation |
| 2 | 2601.15298 | v2 | 2026-01-06 | cs.CL | Embedding Retrofitting: Data Engineering for better RAG |
| 3 | 2601.04568 | v2 | 2026-01-08 | cs.AI | Neurosymbolic Retrievers for Retrieval-augmented Generation |
| 4 | 2601.04764 | v1 | 2026-01-08 | cs.AI | Orion-RAG: Path-Aligned Hybrid Retrieval for Graphless Data |
| 5 | 2601.04859 | v1 | 2026-01-08 | cs.CL | A Navigational Approach for Comprehensive RAG via Traversal over Proposition Graphs |
| 6 | 2601.06519 | v2 | 2026-01-10 | cs.CL | MedRAGChecker: Claim-Level Verification for Biomedical Retrieval-Augmented Generation |
| 7 | 2601.08109 | v1 | 2026-01-13 | cs.DB | CSQL: Mapping Documents into Causal Databases |
| 8 | 2601.08739 | v2 | 2026-01-13 | cs.CL | Ctrl-G: Controlling Structural and Interaction Exposure in Knowledge Graph Augmented LLM Reasoning |
| 9 | 2601.11825 | v1 | 2026-01-16 | cs.AI | AI Co-Scientist for Knowledge Synthesis in Medical Contexts: A Proof of Concept |
| 10 | 2601.15429 | v1 | 2026-01-21 | cs.CL | Domain-Specific Knowledge Graphs in RAG-Enhanced Healthcare LLMs |
| 11 | 2601.15678 | v2 | 2026-01-22 | cs.CR | Connect the Dots: Knowledge Graph-Guided Crawler Attack on Retrieval-Augmented Generation Systems |
| 12 | 2601.19225 | v2 | 2026-01-27 | cs.CL | RPO-RAG: Aligning Small LLMs with Relation-aware Preference Optimization for Knowledge Graph Question Answering |
| 13 | 2603.26670 | v1 | 2026-01-27 | cs.IR | SRAG: RAG with Structured Data Improves Vector Retrieval |
| 14 | 2601.20810 | v1 | 2026-01-28 | cs.SE | Context-Augmented Code Generation Using Programming Knowledge Graphs |
| 15 | 2604.16312 | v1 | 2026-02-01 | cs.IR | FlexStructRAG: Flexible Structure-Aware Multi-Granular Relational Retrieval for RAG |
| 16 | 2602.01965 | v1 | 2026-02-02 | cs.CL | Breaking the Static Graph: Context-Aware Traversal for Robust Retrieval-Augmented Generation |
| 17 | 2602.03315 | v2 | 2026-02-03 | cs.AI | Memora: A Harmonic Memory Representation Balancing Abstraction and Specificity |
| 18 | 2602.05512 | v2 | 2026-02-05 | cs.CL | A Human-in-the-Loop, LLM-Centered Architecture for Knowledge-Graph Question Answering |
| 19 | 2602.05708 | v1 | 2026-02-05 | cs.DB | Cost-Efficient RAG for Entity Matching with LLMs: A Blocking-based Batch Processing Framework |
| 20 | 2602.08668 | v3 | 2026-02-09 | cs.CR | Retrieval Pivot Attacks in Hybrid RAG: Measuring and Mitigating Amplified Leakage from Vector Seeds to Graph Expansion |
| 21 | 2602.09229 | v3 | 2026-02-09 | cs.LG | When Does Embedding Magnitude Matter? A Cross-Task Functional-Symmetry Framework |
| 22 | 2602.10748 | v1 | 2026-02-11 | cs.DB | Benchmarking Large Language Models for Knowledge Graph Validation |
| 23 | 2602.15895 | v2 | 2026-02-11 | cs.CL | Understand Then Memory: A Cognitive Gist-Driven RAG Framework with Global Semantic Diffusion |
| 24 | 2604.20848 | v1 | 2026-02-11 | cs.IR | MATRAG: Multi-Agent Transparent Retrieval-Augmented Generation for Explainable Recommendations |
| 25 | 2602.15553 | v1 | 2026-02-17 | cs.AI | RUVA: Personalized Transparent On-Device Graph Reasoning |
| 26 | 2602.17529 | v1 | 2026-02-19 | cs.AI | Enhancing Large Language Models (LLMs) for Telecom using Dynamic Knowledge Graphs and Explainable Retrieval-Augmented Generation |
| 27 | 2602.20135 | v1 | 2026-02-23 | cs.CL | KNIGHT: Knowledge Graph-Driven Multiple-Choice Question Generation with Adaptive Hardness Calibration |
| 28 | 2603.00122 | v1 | 2026-02-23 | cs.CV | NovaLAD: A Fast, CPU-Optimized Document Extraction Pipeline for Generative AI and Data Intelligence |
| 29 | 2602.22828 | v1 | 2026-02-26 | cs.CL | TCM-DiffRAG: Personalized Syndrome Differentiation Reasoning Method for Traditional Chinese Medicine based on Knowledge Graph and Chain of Thought |
| 30 | 2603.00267 | v1 | 2026-02-27 | cs.AI | Multi-Sourced, Multi-Agent Evidence Retrieval for Fact-Checking |
| 31 | 2603.04452 | v1 | 2026-02-27 | cs.CL | A unified foundational framework for knowledge injection and evaluation of Large Language Models in Combustion Science |
| 32 | 2603.03180 | v1 | 2026-03-03 | cs.SE | Type-Aware Retrieval-Augmented Generation with Dependency Closure for Solver-Executable Industrial Optimization Modeling |
| 33 | 2603.15642 | v1 | 2026-03-03 | cs.AI | CraniMem: Cranial Inspired Gated and Bounded Memory for Agentic Systems |
| 34 | 2603.05129 | v1 | 2026-03-05 | cs.AI | MedCoRAG: Interpretable Hepatology Diagnosis via Hybrid Evidence Retrieval and Multispecialty Consensus |
| 35 | 2603.23512 | v1 | 2026-03-05 | cs.CL | S-Path-RAG: Semantic-Aware Shortest-Path Retrieval Augmented Generation for Multi-Hop Knowledge Graph Question Answering |
| 36 | 2603.10700 | v1 | 2026-03-11 | cs.IR | Structured Linked Data as a Memory Layer for Agent-Orchestrated Retrieval |
| 37 | 2603.11223 | v1 | 2026-03-11 | cs.CL | MDER-DR: Multi-Hop Question Answering with Entity-Centric Summaries |
| 38 | 2603.12458 | v1 | 2026-03-12 | cs.CL | Shattering the Shortcut: A Topology-Regularized Benchmark for Multi-hop Medical Reasoning in LLMs |
| 39 | 2603.20321 | v1 | 2026-03-19 | q-bio.MN | GIP-RAG: An Evidence-Grounded Retrieval-Augmented Framework for Interpretable Gene Interaction and Pathway Impact Analysis |
| 40 | 2603.20094 | v1 | 2026-03-20 | cs.IR | LLM-Enhanced Semantic Data Integration of Electronic Component Qualifications in the Aerospace Domain |
| 41 | 2603.22633 | v1 | 2026-03-23 | cs.AI | Graph-Aware Late Chunking for Retrieval-Augmented Generation in Biomedical Literature |
| 42 | 2603.24925 | v3 | 2026-03-26 | cs.LG | GraphER: An Efficient Graph-Based Enrichment and Reranking Method for Retrieval-Augmented Generation |
| 43 | 2603.24940 | v1 | 2026-03-26 | cs.PL | Evaluating adaptive and generative AI-based feedback and recommendations in a knowledge-graph-integrated programming learning system |
| 44 | 2603.27768 | v1 | 2026-03-29 | cs.CL | TailNLG: A Multilingual Benchmark Addressing Verbalization of Long-Tail Entities |
| 45 | 2603.27910 | v2 | 2026-03-29 | cs.AI | GAAMA: Graph Augmented Associative Memory for Agents |
| 46 | 2604.00687 | v1 | 2026-04-01 | cs.SE | SCPatcher: Automated Smart Contract Code Repair via Retrieval-Augmented Generation and Knowledge Graph |
| 47 | 2604.01610 | v1 | 2026-04-02 | cs.AI | GraphWalk: Enabling Reasoning in Large Language Models through Tool-Based Graph Navigation |
| 48 | 2604.03656 | v1 | 2026-04-04 | cs.AI | Beyond Retrieval: Modeling Confidence Decay and Deterministic Agentic Platforms in Generative Engine Optimization |
| 49 | 2604.04969 | v2 | 2026-04-04 | cs.IR | MG$^2$-RAG: Multi-Granularity Graph for Multimodal Retrieval-Augmented Generation |
| 50 | 2604.04190 | v2 | 2026-04-05 | cs.AI | Schema-Aware Planning and Hybrid Knowledge Toolset for Reliable Knowledge Graph Triple Verification |
| 51 | 2604.04359 | v1 | 2026-04-06 | cs.CL | GROUNDEDKG-RAG: Grounded Knowledge Graph Index for Long-document Question Answering |
| 52 | 2604.04372 | v1 | 2026-04-06 | cs.CV | Graph-to-Frame RAG: Visual-Space Knowledge Fusion for Training-Free and Auditable Video Reasoning |
| 53 | 2604.04565 | v1 | 2026-04-06 | cs.CL | PassiveQA: A Three-Action Framework for Epistemically Calibrated Question Answering via Supervised Finetuning |
| 54 | 2605.27377 | v2 | 2026-04-09 | cs.CL | Enhancing LLM Medical Coding with Structured External Knowledge |
| 55 | 2604.13101 | v1 | 2026-04-10 | cs.SE | Building Trust in the Skies: A Knowledge-Grounded LLM-based Framework for Aviation Safety |
| 56 | 2605.18763 | v1 | 2026-04-10 | cs.IR | Query-Conditioned Graph Retrieval for Contextualized LLM Reasoning in Personalized Wearable Data |
| 57 | 2604.11209 | v1 | 2026-04-13 | cs.CL | Exploring Knowledge Conflicts for Faithful LLM Reasoning: Benchmark and Method |
| 58 | 2604.11419 | v1 | 2026-04-13 | cs.AI | Beyond RAG for Cyber Threat Intelligence: A Systematic Evaluation of Graph-Based and Agentic Retrieval |
| 59 | 2604.14576 | v1 | 2026-04-16 | cs.AI | Enhancing Mental Health Counseling Support in Bangladesh using Culturally-Grounded Knowledge |
| 60 | 2604.15225 | v1 | 2026-04-16 | cs.HC | UrbanClipAtlas: A Visual Analytics Framework for Event and Scene Retrieval in Urban Videos |
| 61 | 2607.14095 | v1 | 2026-04-16 | cs.AI | HG-RAG: Hierarchy-Guided Retrieval-Augmented Generation for Structured Knowledge Graphs |
| 62 | 2604.17114 | v2 | 2026-04-18 | cs.CL | The Provenance Gap in Clinical AI: Evidence-Traceable Temporal Knowledge Graphs for Rare Disease Reasoning |
| 63 | 2604.18478 | v1 | 2026-04-20 | cs.AI | WorldDB: A Vector Graph-of-Worlds Memory Engine with Ontology-Aware Write-Time Reconciliation |
| 64 | 2606.11198 | v1 | 2026-04-21 | cs.CL | The Structural Attention Tax: How Retrieval Format Hijacks In-Context Learning Independent of Content |
| 65 | 2606.15656 | v1 | 2026-04-21 | cs.AI | Overcoming the Impedance Mismatch: A Theoretical Roadmap for Fusing Foundation Models and Knowledge Graphs |
| 66 | 2604.20598 | v1 | 2026-04-22 | cs.IR | Self-Aware Vector Embeddings for Retrieval-Augmented Generation: A Neuroscience-Inspired Framework for Temporal, Confidence-Weighted, and Relational Knowledge |
| 67 | 2604.20666 | v1 | 2026-04-22 | cs.CL | ORPHEAS: A Cross-Lingual Greek-English Embedding Model for Retrieval-Augmented Generation |
| 68 | 2604.20795 | v1 | 2026-04-22 | cs.AI | Automatic Ontology Construction Using LLMs as an External Layer of Memory, Verification, and Planning for Hybrid Intelligent Systems |
| 69 | 2604.22282 | v2 | 2026-04-24 | cs.CL | STEM: Structure-Tracing Evidence Mining for Knowledge Graphs-Driven Retrieval-Augmented Generation |
| 70 | 2604.26176 | v4 | 2026-04-28 | cs.DB | CacheRAG: A Semantic Caching System for Retrieval-Augmented Generation in Knowledge Graph Question Answering |
| 71 | 2605.00943 | v1 | 2026-05-01 | cs.RO | ARIS: Agentic and Relationship Intelligence System for Social Robots |
| 72 | 2605.04643 | v1 | 2026-05-06 | cs.CL | Graph-Augmented LLMs for Swiss MP Ideology Prediction |
| 73 | 2606.00050 | v1 | 2026-05-07 | cs.AI | Grokers: Bottom-Up Inductive Comprehension and Write-Time Intelligence over Typed Knowledge Graphs |
| 74 | 2605.10529 | v1 | 2026-05-11 | cs.AI | PrimeKG-CL: A Continual Graph Learning Benchmark on Evolving Biomedical Knowledge Graphs |
| 75 | 2605.11143 | v1 | 2026-05-11 | cs.CL | ClinicalBench: Stress-Testing Assertion-Aware Retrieval for Cross-Admission Clinical QA on MIMIC-IV |
| 76 | 2605.14665 | v2 | 2026-05-14 | cs.AI | Falkor-IRAC: Graph-Constrained Generation for Verified Legal Reasoning in Indian Judicial AI |
| 77 | 2605.17072 | v1 | 2026-05-16 | cs.AI | RAGA: Reading-And-Graph-building-Agent for Autonomous Knowledge Graph Construction and Retrieval-Augmented Generation |
| 78 | 2605.18570 | v1 | 2026-05-18 | cs.AI | Query-Conditioned Knowledge Alignment for Reliable Cross-System Medical Reasoning |
| 79 | 2605.19229 | v1 | 2026-05-19 | cs.AI | Can Large Language Models Revolutionize Survey Research? Experiments with Disaster Preparedness Responses |
| 80 | 2605.22734 | v1 | 2026-05-21 | cs.CL | ChronoMedKG: A Temporally-Grounded Biomedical Knowledge Graph and Benchmark for Clinical Reasoning |
| 81 | 2605.26305 | v2 | 2026-05-25 | cs.AI | Experiments in Agentic AI for Science |
| 82 | 2605.29602 | v1 | 2026-05-28 | cs.CV | CogniVerse: Revolutionizing Multi-Modal Retrieval-Augmented Generation with Cognitive Reflection and Geometric Reasoning |
| 83 | 2605.29742 | v2 | 2026-05-28 | cs.AI | Citation-Closure Retrieval and Per-Rule Attribution for Real-World Regulatory Compliance Question Answering |
| 84 | 2605.30966 | v1 | 2026-05-29 | cs.IR | Reading Between the Citations: A Typed Claim Network for Scientific Literature |
| 85 | 2606.01613 | v2 | 2026-06-01 | cs.IR | TechRAG: Evidence-Gated Multimodal Agentic RAG for Technical Literature Reasoning |
| 86 | 2607.19359 | v1 | 2026-06-01 | cs.AI | Profile-Graph Memory for LLM Agents: Implicit Cross-Entity Traversal through Narrative Profiles |
| 87 | 2606.03374 | v2 | 2026-06-02 | cs.RO | eMEM: A Hybrid Spatio-Temporal Memory System For Embodied Agents |
| 88 | 2607.18270 | v1 | 2026-06-02 | cs.AI | Trajectory-Aware Clinical Risk Prediction via Severity-Grounded Knowledge Graphs and Retrieval-Augmented Generation |
| 89 | 2606.06003 | v1 | 2026-06-04 | cs.AI | Beyond Vector Similarity: A Structural Analysis of Graph-Augmented Retrieval for Industrial Knowledge Graphs |
| 90 | 2606.06942 | v1 | 2026-06-05 | cs.CL | Didact: A Cross-Domain Capability Discovery System for Defence |
| 91 | 2606.08921 | v1 | 2026-06-08 | cs.LG | Generalized Rank-based Evaluation for Knowledge Graph Completion: Perspectives, Framework, and Analyses |
| 92 | 2606.15778 | v1 | 2026-06-14 | cs.CL | DYNA : Dynamic Episodic Memory Networks for Augmenting Large Language Models with Temporal Knowledge Graphs in Continuous Learning |
| 93 | 2606.15838 | v1 | 2026-06-14 | cs.IR | Intelligent Multimodal Retrieval and Reasoning for Geospatial Knowledge Discovery on the I-GUIDE Platform |
| 94 | 2606.15971 | v2 | 2026-06-14 | cs.CL | SAG: SQL-Retrieval Augmented Generation with Query-Time Dynamic Hyperedges |
| 95 | 2606.20041 | v2 | 2026-06-18 | econ.GN | AI Economist Agent: An Agentic Framework for Evidence-Based Economic and Financial Analysis with RAG, Knowledge Graphs, and Large Language Models |
| 96 | 2606.21151 | v1 | 2026-06-19 | cs.SE | Context-Aware Generative AI for Automated Telecom Test Script Generation |
| 97 | 2606.22419 | v3 | 2026-06-21 | cs.CL | Knowledge-Graph Grounding Helps LLMs Only for Out-of-Training Knowledge: A Controlled Study on Clinical Question Answering |
| 98 | 2606.22692 | v1 | 2026-06-21 | cs.AI | VISTA Architect: A graph database-oriented health AI system demonstrated in multidisciplinary tumor boards |
| 99 | 2606.22992 | v1 | 2026-06-22 | cs.CL | Predicate Importance Estimation and Decoupled Rationale-Score Distillation for Entity Alignment |
| 100 | 2606.23997 | v1 | 2026-06-22 | cs.IR | ChartWalker: Benchmarking the Cross-Chart RAG Task with Hierarchical Knowledge Graphs |
| 101 | 2606.25622 | v1 | 2026-06-24 | cs.CR | Probabilistic Agents in Deterministic Audits: Evaluating Multi-Agent Systems for Automated Audits Based on the German IT-Grundschutz |
| 102 | 2606.28447 | v2 | 2026-06-26 | cs.IR | SemFlowRAG: Directed Semantic Flow from Abstraction to Evidence for Complex Reasoning |
| 103 | 2607.22652 | v1 | 2026-06-26 | cs.AI | KG2Code: Bridging Knowledge Graphs and Large Language Models via Executable Code for Question Answering |
| 104 | 2609.01618 | v1 | 2026-06-30 | cs.IR | Multi-Agent Retrieval-Augmented Generation for Efficient Cloud Knowledge Base Search in Telecom SNOC Environment |
| 105 | 2607.03028 | v1 | 2026-07-03 | cs.IR | HETERQA: Benchmarking Record Retrieval over Multiple Heterogeneous Sources |
| 106 | 2607.03233 | v1 | 2026-07-03 | cs.CR | Agentic and Generative AI for Open-Source Intelligence and Cyber Investigations: Taxonomy, Evaluation, Challenges, and Future Directions |
| 107 | 2607.20517 | v1 | 2026-07-07 | cs.LG | Multimodal CoLRAG-TF: Triple-Filtered Retrieval for Complex PDFs |
| 108 | 2607.09092 | v1 | 2026-07-10 | cs.CL | AgentKGV: Agentic LLM-RAG Framework with Two-Stage Training for the Fact Verification of Knowledge Graphs |
| 109 | 2607.10151 | v1 | 2026-07-11 | cs.IR | MC-RAG System: A Structure-Driven RAG System for Multi-Constraint Queries |
| 110 | 2607.20560 | v1 | 2026-07-17 | cs.LG | Chronofy: A Temporal-Logical Decay Architecture for Information Validity in Time-Aware Retrieval-Augmented Generation |
| 111 | 2607.16604 | v2 | 2026-07-18 | eess.IV | Controlled Evaluation of Graph and Multimodal Augmentation in RAG for Document Question Answering |
| 112 | 2607.22781 | v1 | 2026-07-24 | cs.LG | What Softmax Throws Away: Mass-Aware Attention for Evidence Accumulation |
| 113 | 2607.24838 | v1 | 2026-07-24 | cs.IR | MedJudgeRAG: Option-Wise Evidence Judgment with Dynamic Knowledge Graphs for Medical MCQA |
| 114 | 2607.25959 | v3 | 2026-07-28 | cs.CL | Detecting Knowledge Inconsistencies Across Text, Tables, and Knowledge Graphs |
| 115 | 2607.27748 | v1 | 2026-07-30 | cs.IR | A Structured Knowledge Infrastructure for Domain-Specific Data Asset Discovery |
| 116 | 2607.28942 | v3 | 2026-07-31 | cs.AI | NeSyFS: A Neuro-symbolic Fast-Slow Thinking Framework for LLM Agent under Partial Observability |
| 117 | 2608.00922 | v1 | 2026-08-02 | cs.DC | From Cloud to Crowd: Democratizing LLM Service with Decentralized Edge Collaboration for RAG |
| 118 | 2609.35780 | v1 | 2026-08-04 | cs.IR | TSG Suggester: Tree-Structured Knowledge-Graph Retrieval for Troubleshooting Guide Recommendation in Cloud Incident Management |
| 119 | 2608.08935 | v1 | 2026-08-09 | cs.AI | Integrated Multimodal AI System for Retrieval-Augmented Reasoning, Object Sensing, and Damage Analysis |
| 120 | 2608.09779 | v1 | 2026-08-10 | cs.CL | KGCaRe: Explainable Complex Conditional Question Answering using Automatic Knowledge Graph Construction and Context Retrieval with LLMs |
| 121 | 2608.12129 | v2 | 2026-08-12 | cs.CL | SAG: SQL-Retrieval Augmented Generation with Query-Time Dynamic Hyperedges |
| 122 | 2608.12304 | v1 | 2026-08-12 | cs.AI | Constructing Dynamic Master Logic Models as Knowledge Graphs for Complex System Diagnostics Using Retrieval-Augmented Large Language Models |
| 123 | 2608.17536 | v2 | 2026-08-18 | cs.CL | CoAL-RAG: A Complexity-Aware Legal Retrieval-Augmented Generation Method |
| 124 | 2611.08498 | v1 | 2026-08-19 | cs.IR | Improving Contextual Retrieval for Long Documents using Semantic Chunking and Hybrid Retrieval Strategies |
| 125 | 2608.22634 | v1 | 2026-08-23 | cs.CL | GeoRisk-RAG: A Hierarchy-Aware Risk Framework for Improving RAG Reliability through Selective Answering |
| 126 | 2608.26836 | v1 | 2026-08-27 | cs.AI | SymbolLKG: Towards Verifiable Logical Reasoning via Logical Knowledge Graph and Symbolic Solvers |
| 127 | 2608.28978 | v1 | 2026-08-29 | cs.AI | Selective Forgetting: A Graph-Based Memory Framework for Long-Term LLM Agents |
| 128 | 2608.31137 | v1 | 2026-08-31 | cs.AI | OntoAligner-Ensemble: Voting-Based Fusion across Heterogeneous Ontology Alignment Techniques |
| 129 | 2609.00479 | v1 | 2026-08-31 | cs.AI | EGT-KG: Evidence-Grounded Typed KG Retrieval for Practical Scientific QA with Small Language Models |
| 130 | 2609.01653 | v1 | 2026-08-31 | q-bio.OT | Enhancing Clinical Decision Support and Differential Diagnosis with Knowledge Graphs, and Retrieval Augmented Generation in Generative AI |
| 131 | 2609.05518 | v1 | 2026-08-31 | cs.CV | CrossModalQA: A Cross-modal and Multi-hop Benchmark for Multimodal Retrieval-augmented Generation |
| 132 | 2609.00629 | v1 | 2026-09-01 | cs.CL | ExpArt-KG: Artwork Image Description Generation through Iterative Exploration of Knowledge Graphs |
| 133 | 2609.00763 | v1 | 2026-09-01 | cs.AI | Automated Tree Knowledge Graph Construction using Ontology Expansion and Retrieval from Vietnamese History Textbooks |
| 134 | 2609.00858 | v2 | 2026-09-01 | cs.AI | Verifiable Disaster Storylines and Causal Knowledge Graphs: A Citation-Grounded Pipeline from Heterogeneous Humanitarian Sources |
| 135 | 2609.04377 | v1 | 2026-09-03 | cs.AI | Corporate Language Model (CLM): Transforming Tacit and Fragmented Enterprise Knowledge into a Sovereign, Auditable, and Executable Corporate Intelligence Layer |
| 136 | 2609.04511 | v1 | 2026-09-03 | cs.CL | LentEx: Generalizable Latent Entity Extraction via Synthetic Data and Instruction-Tuned LLMs |
| 137 | 2609.05339 | v1 | 2026-09-04 | cs.AI | Does Your Agent's Memory Survive a Model Upgrade? A Controlled Study of Memory Portability |
| 138 | 2609.08790 | v1 | 2026-09-08 | cs.CR | Evidence-Grounded Retrieval for Investigation Hunt Lead Generation from CTI Reports |
| 139 | 2609.11493 | v2 | 2026-09-10 | cs.AI | From Document Silos to Process Intelligence: A Multi-Layer Knowledge Graph for CMC Process Development |
| 140 | 2609.11569 | v1 | 2026-09-10 | cs.AI | Enabling Knowledge Graph Understanding at Scale with the EXplore Your Graphs ENgine (EXYGEN) |
| 141 | 2609.12791 | v1 | 2026-09-11 | cs.CL | Cognition on Graph: Navigating Massive Knowledge Space via Cognitive Cycles and Bidirectional Graph-Text Synergy |
| 142 | 2609.18248 | v1 | 2026-09-16 | cs.IR | Quanta: A Self-Contained Python Library for Hybrid Retrieval over Quantised Embeddings, Lexical Indexes, and Knowledge Graphs |
| 143 | 2609.24236 | v1 | 2026-09-21 | cs.DB | Efficient Dense Vector Search within Knowledge Graph Content Embeddings |
| 144 | 2609.34145 | v1 | 2026-09-28 | cs.RO | Beyond Retrieval Relevance: Scene-Grounded Risk Entailment for Vision-Language Driving |
| 145 | 2609.35193 | v1 | 2026-09-28 | cs.LG | ConRAG: Lightweight inference of multi-hop relations |
| 146 | 2610.01767 | v1 | 2026-10-01 | cs.CL | A Matryoshka Hierarchical RAG for Efficient Multi-Hop Question Answering |
| 147 | 2610.08894 | v1 | 2026-10-06 | cs.IR | Trustworthy Domain-Specific AI for Structured Knowledge Retrieval and Reasoning |
| 148 | 2610.13695 | v2 | 2026-10-09 | cs.CL | Verifiable Retrieval for Legal Reasoning through Structure-driven Knowledge Graph |
| 149 | 2610.14123 | v1 | 2026-10-09 | cond-mat.mtrl-sci | Hierarchical Multi-Agent Reasoning Systems for Knowledge-Grounded Material Substitution |
| 150 | 2610.15463 | v2 | 2026-10-11 | cs.AI | MemeLogos: Knowledge-Driven Retrieval-Augmented Reasoning for Harmful Meme Detection |
| 151 | 2610.22017 | v1 | 2026-10-15 | cs.AI | CARA: Causally Guided Retrieval Agent for Question Answering |
| 152 | 2610.24390 | v1 | 2026-10-17 | cs.AI | Trust the Edge, Doubt the Arrow: Auditing Validation Signals for Text-Mined Clinical Causal Knowledge Graphs |
| 153 | 2610.28176 | v2 | 2026-10-20 | cs.CL | Temporal Evidence-Parameter Co-Alignment for LLM Reasoning over Temporal Knowledge Graphs |
| 154 | 2610.29933 | v1 | 2026-10-21 | cs.CL | Graph Topology Meets Energy-Driven Pathfinding: Preserving Contextual Consistency in Retrieval-Augmented Generation |
| 155 | 2610.38366 | v1 | 2026-10-28 | cs.CL | TCC-RAG: Temporal Causal Chain-Guided RAG for Temporal Knowledge Graph Question Answering |
| 156 | 2610.39230 | v1 | 2026-10-29 | cs.AI | Beyond LLM Wrappers: Context Engineering and the Three-Engine Architecture for Reliable Financial AI |
| 157 | 2611.10477 | v1 | 2026-11-07 | cs.AI | PatternBloom: Empowering Agentic RAG with Externalized RL-Distilled Graph Patterns |
| 158 | 2611.14263 | v1 | 2026-11-10 | cs.LG | Chain of Physics: Supporting Fault Diagnosis with Engineering-Inspired Large Language Model Reasoning |
