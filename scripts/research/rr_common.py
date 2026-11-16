"""Shared helpers for the retrieval research scripts (ADR_ONTODOC_RETRIEVAL_RESEARCH rev 4).

Dev data only. Ports tokenizer version 1 (src/search/tokenize.rs), Okapi BM25
(src/search/bm25.rs) and SplitMix64 (src/eval/stats.rs); every port is checked
against the pinned binary before any number is reported.
"""
import collections
import hashlib
import json
import math
import os
import subprocess
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RUN = 'tmp/release-010/benchmark/eval-donor-fusion/'
INPUTS = [RUN + 'misses.json', RUN + 'run-candidate/B1.tsv', RUN + 'run-candidate/meta.json',
          'eval/dev-queries.jsonl', 'eval/dev-qrels.jsonl']
HY0_SUMMARY = 'tmp/hybrid-stage0/summary.json'
CORPORA = {'ontoindex': 'tmp/corpora/ontoindex', 'rust-book': 'tmp/corpora/rust-book/src'}
BINARY = 'target/release/ontodoc'
FETCH = 50
CUTOFF = 10
K1, B = 1.2, 0.75
SEED = 20261116
RESAMPLES = 10000
STOPWORDS = frozenset('''a an the and or but if then else as at by for from in into of on onto to
with without about is am are was were be been being do does did have has had i me my we us our
you your he him his she her it its they them their this that these those what which who whom
whose when where why how can'''.split())
VOWELS = frozenset('aeiou')
FORBIDDEN = ('eval/test-', 'eval/queries.jsonl', 'eval/qrels.jsonl', 'eval/pools',
             'audit_session-2026-11-13-held-out')


class GateError(Exception):
    pass


# Rust std char properties for non-ASCII code points, from charprops.rs compiled by the
# pinned toolchain; Python's str methods disagree on some marks and CJK numerals.
_PROPS = {}


def load_char_props(texts):
    code_points = sorted({ord(c) for t in texts for c in t if ord(c) > 127})
    src = os.path.join(ROOT, 'scripts/research/charprops.rs')
    with tempfile.TemporaryDirectory() as d:
        exe = os.path.join(d, 'charprops')
        subprocess.run(['rustc', '-O', '--edition', '2021', src, '-o', exe], check=True, cwd=ROOT,
                       capture_output=True)
        out = subprocess.run([exe], input=''.join(f'{n}\n' for n in code_points).encode(),
                             capture_output=True, check=True).stdout.decode()
    for line in out.splitlines():
        n, a, u, lo, num, low = line.split('\t')
        _PROPS[chr(int(n))] = (a == '1', u == '1', lo == '1', num == '1',
                               ''.join(chr(int(x)) for x in low.split(',')))
    missing = [n for n in code_points if chr(n) not in _PROPS]
    if missing:
        raise GateError(f'char property probe missed {len(missing)} code points')
    return hashlib.sha256(out.encode()).hexdigest()


def _prop(c, i, ascii_fn):
    if ord(c) < 128:
        return ascii_fn(c)
    p = _PROPS.get(c)
    if p is None:
        raise GateError(f'no char property for U+{ord(c):04X}')
    return p[i]


def is_alnum(c):
    return _prop(c, 0, str.isalnum)


def is_upper(c):
    return _prop(c, 1, str.isupper)


def is_lower(c):
    return _prop(c, 2, str.islower)


def is_numeric(c):
    return _prop(c, 3, str.isdigit)


def lower(s):
    return ''.join(c.lower() if ord(c) < 128 else _prop(c, 4, str.lower) for c in s)


def path(p):
    if any(f in p for f in FORBIDDEN):
        raise GateError(f'refused held-out path {p}')
    return os.path.join(ROOT, p)


def sha256(p):
    with open(path(p), 'rb') as f:
        return hashlib.sha256(f.read()).hexdigest()


# --- tokenizer version 1 -------------------------------------------------

def _word_char(c):
    return c == '_' or is_alnum(c)


def _split_parts(word):
    parts, cur = [], []
    for i, c in enumerate(word):
        if c == '_':
            if cur:
                parts.append(lower(''.join(cur))); cur = []
            continue
        if i > 0 and is_upper(c):
            prev = word[i - 1]
            nxt_lower = i + 1 < len(word) and is_lower(word[i + 1])
            if is_lower(prev) or is_numeric(prev) or (is_upper(prev) and nxt_lower):
                if cur:
                    parts.append(lower(''.join(cur))); cur = []
        cur.append(c)
    if cur:
        parts.append(lower(''.join(cur)))
    return parts


def words(text):
    out, cur = [], []
    for c in text:
        if _word_char(c):
            cur.append(c)
        else:
            out.append(''.join(cur)); cur = []
    out.append(''.join(cur))
    return out


def tokenize_tagged(text):
    """(token, eligible) pairs; eligible marks an unsplit ASCII-letter word of length >= 3."""
    out = []
    for word in words(text):
        parts = _split_parts(word)
        if len(parts) > 1:
            out.append((lower(word), False))
            out.extend((p, False) for p in parts)
        elif parts:
            p = parts[0]
            out.append((p, len(word) >= 3 and word.isascii() and word.isalpha()))
    return out


def tokenize(text):
    return [t for t, _ in tokenize_tagged(text)]


def query_terms(query):
    seen, out = set(), []
    for t in tokenize(query):
        if t not in seen:
            seen.add(t); out.append(t)
    return out


# --- BM25 ---------------------------------------------------------------

class Bm25:
    def __init__(self, docs):
        self.postings = collections.defaultdict(list)
        self.doc_len = []
        for d, tokens in enumerate(docs):
            self.doc_len.append(len(tokens))
            for term, n in collections.Counter(tokens).items():
                self.postings[term].append((d, n))
        self.avgdl = sum(self.doc_len) / len(self.doc_len) if self.doc_len else 0.0

    def idf(self, term):
        n = len(self.postings.get(term, ()))
        return math.log(1.0 + (len(self.doc_len) - n + 0.5) / (n + 0.5))

    def scores(self, terms):
        s = [0.0] * len(self.doc_len)
        for term in terms:
            post = self.postings.get(term)
            if not post:
                continue
            idf = self.idf(term)
            for d, tf in post:
                ratio = self.doc_len[d] / self.avgdl if self.avgdl > 0 else 1.0
                s[d] += idf * tf * (K1 + 1.0) / (tf + K1 * (1.0 - B + B * ratio))
        return s


def rank(scores, chunk_ids, top_k):
    hits = [(i, sc) for i, sc in enumerate(scores) if sc > 0.0]
    hits.sort(key=lambda h: (-h[1], chunk_ids[h[0]]))
    return hits[:top_k]


def project(hits, chunk_section, depth):
    out, seen = [], set()
    for i, sc in hits:
        if len(out) == depth:
            break
        s = chunk_section[i]
        if s not in seen:
            seen.add(s); out.append((s, sc))
    return out


# --- corpora ------------------------------------------------------------

class Corpus:
    def __init__(self, name, gen_id):
        self.name, self.root = name, CORPORA[name]
        cur = open(path(self.root + '/.ontodoc/CURRENT')).read().strip()
        if cur != gen_id:
            raise GateError(f'{name}: CURRENT {cur} != pinned {gen_id}')
        self.gen_id = gen_id
        art = f'{self.root}/.ontodoc/generations/{gen_id}/index.json'
        meta = json.load(open(path(f'{self.root}/.ontodoc/generations/{gen_id}/meta.json')))
        self.artifact_sha256 = sha256(art)
        if meta['artifact_sha256'] != self.artifact_sha256 or meta['generation_id'] != gen_id:
            raise GateError(f'{name}: artifact hash mismatch')
        g = json.load(open(path(art), encoding='utf-8'))
        if g['config']['tokenizer_version'] != 1:
            raise GateError(f'{name}: tokenizer_version {g["config"]["tokenizer_version"]}')
        self.char_props_sha256 = load_char_props(
            [d['text'] for d in g['documents']] + ['\n'.join(c['heading_path']) for c in g['chunks']])
        docs = {}
        for d in g['documents']:
            docs.setdefault(d['path'], d['text'].encode('utf-8'))
        self.chunks = g['chunks']
        self.ids = [c['id'] for c in self.chunks]
        self.section = [c['section_id'] for c in self.chunks]
        self.section_heading = {s['id']: s['heading_path'] for s in g['sections']}
        self.head_tagged, self.body_tagged = [], []
        for c in self.chunks:
            raw = docs.get(c['path'], b'')[c['span']['byte_start']:c['span']['byte_end']]
            self.head_tagged.append(tokenize_tagged('\n'.join(c['heading_path'])))
            self.body_tagged.append(tokenize_tagged(raw.decode('utf-8')))
        self.tokens = [[t for t, _ in h] + [t for t, _ in b]
                       for h, b in zip(self.head_tagged, self.body_tagged)]
        self.bm25 = Bm25(self.tokens)
        self.by_section = collections.defaultdict(list)
        for i, s in enumerate(self.section):
            self.by_section[s].append(i)
        self._heading_bm25 = None

    def heading_bm25(self):
        if self._heading_bm25 is None:
            self._heading_bm25 = Bm25([[t for t, _ in h] for h in self.head_tagged])
        return self._heading_bm25

    def b1(self, query):
        return rank(self.bm25.scores(query_terms(query)), self.ids, FETCH)


def binary_top50(root, query):
    out = subprocess.run([path(BINARY), 'search', '--root', path(root), '--mode', 'lexical',
                          '--top-k', str(FETCH), '--explain', query],
                         capture_output=True, check=True)
    d = json.loads(out.stdout)
    if d.get('truncated') or not d.get('fresh'):
        raise GateError('binary response truncated or stale')
    return d


# --- inputs -------------------------------------------------------------

def load_inputs():
    hy0 = json.load(open(path(HY0_SUMMARY)))
    sha = {p: sha256(p) for p in INPUTS}
    if sha != hy0['inputs_sha256']:
        raise GateError('input hashes differ from HY-0')
    queries = {}
    for line in open(path('eval/dev-queries.jsonl'), encoding='utf-8'):
        q = json.loads(line); queries[q['id']] = q
    qrels = collections.defaultdict(dict)
    for line in open(path('eval/dev-qrels.jsonl'), encoding='utf-8'):
        r = json.loads(line); qrels[r['query_id']][r['section_id']] = r['grade']
    b1 = collections.defaultdict(list)
    for line in open(path(RUN + 'run-candidate/B1.tsv'), encoding='utf-8'):
        q, _, s, rk, sc, _ = line.rstrip('\n').split('\t'); b1[q].append((int(rk), s, float(sc)))
    for q in b1:
        b1[q].sort()
    misses = [m for m in json.load(open(path(RUN + 'misses.json'))) if m['system'] == 'B1']
    meta = json.load(open(path(RUN + 'run-candidate/meta.json')))
    gens = {c['name']: c['generation_id'] for c in meta['corpora']}
    answerable = sorted(q for q in queries if any(g >= 1 for g in qrels[q].values()))
    return dict(sha=sha, hy0=hy0, queries=queries, qrels=qrels, b1=b1, misses=misses,
                gens=gens, answerable=answerable)


# --- metrics and statistics ---------------------------------------------

def dcg(gains):
    return sum(g / math.log2(i + 2) for i, g in enumerate(gains))


def ndcg(qrel, ranking, k=CUTOFF):
    ideal = dcg(sorted(qrel.values(), reverse=True)[:k])
    return dcg([qrel.get(s, 0) for s in ranking[:k]]) / ideal


def condensed_ndcg(qrel, ranking, k=CUTOFF):
    return ndcg(qrel, [s for s in ranking if s in qrel], k)


def judged_at(qrel, ranking, k=CUTOFF):
    top = ranking[:k]
    return sum(1 for s in top if s in qrel) / len(top) if top else 1.0


class SplitMix64:
    M = (1 << 64) - 1

    def __init__(self, seed):
        self.s = seed & self.M

    def next_u64(self):
        self.s = (self.s + 0x9E3779B97F4A7C15) & self.M
        z = self.s
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & self.M
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & self.M
        return z ^ (z >> 31)

    def below(self, n):
        return (self.next_u64() * n) >> 64


def quantile(sorted_vals, p):
    if not sorted_vals:
        return 0.0
    h = (len(sorted_vals) - 1) * p
    lo, hi = math.floor(h), math.ceil(h)
    return sorted_vals[lo] + (h - lo) * (sorted_vals[hi] - sorted_vals[lo])


def bootstrap_mean(values, resamples=RESAMPLES, seed=SEED):
    """Percentile 95% interval of a mean, resampling the given units."""
    rng, n, means = SplitMix64(seed), len(values), []
    if n == 0:
        return [0.0, 0.0, 0.0]
    for _ in range(resamples):
        means.append(sum(values[rng.below(n)] for _ in range(n)) / n)
    means.sort()
    return [sum(values) / n, quantile(means, 0.025), quantile(means, 0.975)]


def cluster_share(clusters, resamples=RESAMPLES, seed=SEED):
    """Pair share with a query-cluster bootstrap; clusters are (hits, total) per query."""
    clusters = [c for c in clusters if c[1] > 0]
    k, n = sum(c[0] for c in clusters), sum(c[1] for c in clusters)
    if n == 0:
        return {'k': 0, 'n': 0, 'share': None, 'cluster95': None, 'wilson95': None}
    rng, m, shares = SplitMix64(seed), len(clusters), []
    for _ in range(resamples):
        kk = nn = 0
        for _ in range(m):
            a, b = clusters[rng.below(m)]; kk += a; nn += b
        shares.append(kk / nn)
    shares.sort()
    return {'k': k, 'n': n, 'share': k / n, 'cluster95': [quantile(shares, 0.025), quantile(shares, 0.975)],
            'wilson95': wilson(k, n)}


def wilson(k, n, z=1.959963984540054):
    p = k / n; d = 1 + z * z / n; c = p + z * z / (2 * n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return [max(0.0, (c - h) / d), min(1.0, (c + h) / d)]


def median(vals):
    v = sorted(vals); n = len(v)
    return (v[(n - 1) // 2] + v[n // 2]) / 2


def close(a, b, rel=1e-9):
    return abs(a - b) <= rel * max(1.0, abs(a), abs(b))


# --- surface forms ------------------------------------------------------

def _variants(x):
    """Registered one-step forms of base x (x eligible)."""
    out = set()
    if not x.endswith('s'):
        out.add(x + 's')
    if x.endswith(('s', 'x', 'z', 'ch', 'sh')):
        out.add(x + 'es')
    if len(x) >= 2 and x[-1] == 'y' and x[-2] not in VOWELS:
        out.add(x[:-1] + 'ies')
    out.add(x + 'ed'); out.add(x + 'ing')
    if x.endswith('e'):
        out.add(x[:-1] + 'ed'); out.add(x[:-1] + 'ing')
    if len(x) >= 2 and x[-1] not in VOWELS and x[-2] in VOWELS:
        out.add(x + x[-1] + 'ed'); out.add(x + x[-1] + 'ing')
    return out


def surface_pair(a, b):
    """True when eligible tokens a != b differ by one registered ending."""
    if a == b:
        return False
    x, y = (a, b) if len(a) <= len(b) else (b, a)
    return y in _variants(x)


def fold_key_map(vocab):
    """Map each eligible token to the shortest eligible token it pairs with (one step, deterministic)."""
    vocab = sorted(vocab)
    present = set(vocab)
    fold = {}
    for x in vocab:
        for v in sorted(_variants(x)):
            if v in present and v != x:
                cur = fold.get(v)
                if cur is None or (len(x), x) < (len(cur), cur):
                    fold[v] = x
    return fold
