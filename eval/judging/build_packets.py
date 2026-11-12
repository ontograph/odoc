"""Blinded judge packets: query text + section text only; seeded order; no ranks, systems or scores."""
import json, random, hashlib
ROOTS={'ontoindex':'tmp/corpora/ontoindex','rust-book':'tmp/corpora/rust-book/src'}
MAXC=10**9
q={json.loads(l)['id']:json.loads(l) for l in open('eval/queries.jsonl')}
sec={}
for c,r in ROOTS.items():
    cur=open(r+'/.ontodoc/CURRENT').read().strip()
    idx=json.load(open(f'{r}/.ontodoc/generations/{cur}/index.json'))
    for s in idx['sections']: sec[(c,s['id'])]=(r,s)
pairs=[json.loads(l) for l in open('eval/pools/pool.jsonl')]
items=[]
for p in pairs:
    qq=q[p['query_id']]; r,s=sec[(qq['corpus'],p['section_id'])]
    raw=open(f"{r}/{s['path']}",'rb').read()[s['span']['byte_start']:s['span']['byte_end']]
    assert hashlib.sha256(raw).hexdigest()==s['sha256'], p
    t=raw.decode('utf-8','replace'); trunc=len(t)>MAXC
    items.append({'pair':f"{p['query_id']}|{p['section_id']}",'query':qq['text'],
        'section_heading':' > '.join(s['heading_path']),'section_text':t[:MAXC],'section_text_truncated':trunc,'span_sha256':s['sha256']})
random.Random(20261112).shuffle(items)
B=100
for k in range(0,len(items),B):
    open(f'tmp/judge/packet-{k//B:02d}.jsonl','w').write(''.join(json.dumps(i,ensure_ascii=False)+'\n' for i in items[k:k+B]))
print(len(items), (len(items)+B-1)//B, sum(i['section_text_truncated'] for i in items))
