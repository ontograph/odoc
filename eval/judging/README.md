# EVAL-2.1 model judging record

Model-judged labels, exploratory only (ADR revisions 13-15). Judges:
`j1` GPT-6-Astra and `j2` claude-opus-5-5 (reasoning high), one fresh agent
per packet, each given only `BRIEF.txt`, `GUIDELINE.md` and one packet.

| File | Role |
| --- | --- |
| `GUIDELINE.md`, `BRIEF.txt` | Exact instructions the judges read (copied unchanged from `tmp/judge/`, where they ran; paths inside them refer to that location) |
| `build_packets.py` | Builds blinded packets (query and section text only, seed 20261112) into `tmp/judge/` |
| `out-astra/`, `out-claude/` | Raw grades, one line per packet line: `pair`, `grade`, `why` |
| `build_qrels.py` | Deterministic merge: `floor((a+b)/2)`, writes `eval/qrels.jsonl`, the two partition qrels and `eval/annotation.jsonl` |
| `packets.sha256` | Hashes of the packets the judges read (packets are not committed: they copy corpus text) |
| `outputs.sha256` | Hashes of the instructions, scripts and raw outputs |

Packets 00-40 cover the first 4,004-pair pool; `x0`-`x5` cover the 507 pairs
added by the depth-30 extension (ADR rev 14). All 4,511 pairs were graded by
both judges. ADR rev 15 withdrew 12 queries afterwards, so 573 judged pairs are
not in the current 3,938-pair pool; `build_qrels.py` keeps only pool pairs.

Check: `cd tmp/judge && sha256sum -c ../../eval/judging/packets.sha256` and
`cd eval/judging && sha256sum -c outputs.sha256`; then run
`python3 eval/judging/build_qrels.py` from the repository root.
`build_packets.py` built packets 00-40. The x-packets were split from
`tmp/judge/new-pairs.jsonl` (the pool-extension pairs; not committed, sha256
listed in `packets.sha256`) without a committed script, so they are verified
by hash, not regenerated.

Limits: these are local records, not independent custody. Session isolation
rests on how agents were dispatched, not on any receipt in these files. The
drafting model of the queries is also judge `j2`.
