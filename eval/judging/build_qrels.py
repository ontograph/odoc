"""Merge two model judges into qrels/annotation (ADR rev 13 floor rule).

Deterministic; no grading logic. Packets (query + section text) are rebuilt by
build_packets.py into tmp/judge/ (git-ignored: they copy corpus text); judge
outputs are committed in eval/judging/out-*. Grades are emitted only for pairs
in the current pool: ADR rev 15 withdrew 12 queries, whose judged pairs stay
in the outputs but leave the benchmark.
"""
import glob, hashlib, json, sys
from collections import Counter

J = 'tmp/judge'
O = 'eval/judging'
pool = [json.loads(l) for l in open('eval/pools/pool.jsonl')]
span, packets = {}, []
for f in sorted(glob.glob(f'{J}/packet-*.jsonl')):
    packets.append(f.split('/')[-1])
    for l in open(f):
        r = json.loads(l)
        span[r['pair']] = r['span_sha256']
g = {}
for m in ('astra', 'claude'):
    g[m] = {}
    for pk in packets:
        inp = [json.loads(l)['pair'] for l in open(f'{J}/{pk}')]
        out = [json.loads(l) for l in open(f'{O}/out-{m}/{pk}')]
        assert [o['pair'] for o in out] == inp, (m, pk)
        for o in out:
            assert set(o) == {'pair', 'grade', 'why'} and o['grade'] in (0, 1, 2), (m, pk, o)
            g[m][o['pair']] = o['grade']
keys = [f"{p['query_id']}|{p['section_id']}" for p in pool]
assert len(keys) == len(set(keys))
assert set(g['astra']) == set(g['claude']) == set(span), 'judges and packets disagree'
missing = sorted(set(keys) - set(span))
assert not missing, f'pool pairs never judged: {missing[:5]}'
splits = json.load(open('eval/splits.json'))
dev, test = set(splits['dev']), set(splits['test'])
qrels, ann, dq, tq = [], [], [], []
for k in sorted(keys):
    q, s = k.split('|', 1)
    a, b = g['astra'][k], g['claude'][k]
    f = (a + b) // 2
    rec = {'query_id': q, 'section_id': s, 'grade': f, 'span_sha256': span[k]}
    qrels.append(rec)
    (dq if q in dev else tq if q in test else sys.exit('split ' + q)).append(rec)
    ann.append({'version': 1, 'query_id': q, 'section_id': s, 'span_sha256': span[k],
                'grades': [{'annotator': 'j1', 'grade': a, 'evidence': f'eval/judging/out-astra#{k}'},
                           {'annotator': 'j2', 'grade': b, 'evidence': f'eval/judging/out-claude#{k}'}],
                'final': {'grade': f,
                          'adjudicator': {'id': 'floor-rule', 'kind': 'model', 'model': 'floor-mean-v1',
                                          'evidence': 'ADR rev 13 floor((a+b)/2)'},
                          'evidence': 'floor-mean-v1'}})


def write(path, rows):
    with open(path, 'w') as fh:
        fh.write(''.join(json.dumps(r, ensure_ascii=False, separators=(',', ':')) + '\n' for r in rows))


write('eval/qrels.jsonl', qrels)
write('eval/dev-qrels.jsonl', dq)
write('eval/test-qrels.jsonl', tq)
write('eval/annotation.jsonl', ann)
print('pairs', len(qrels), 'dev', len(dq), 'test', len(tq), 'unused judged pairs', len(set(span) - set(keys)),
      'final', sorted(Counter(r['grade'] for r in qrels).items()))
rel = {r['query_id'] for r in qrels if r['grade'] >= 1}
print('answerable without grade>=1:', sorted({r['query_id'] for r in qrels} - rel))
for p in ('eval/qrels.jsonl', 'eval/annotation.jsonl', 'eval/dev-qrels.jsonl', 'eval/test-qrels.jsonl'):
    print(p, hashlib.sha256(open(p, 'rb').read()).hexdigest())
