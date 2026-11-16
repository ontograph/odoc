"""Step 1 of ADR_ONTODOC_RETRIEVAL_RESEARCH rev 4: descriptive split of the B1 dev misses.

Usage: rr_step1.py --self-test | --out PATH. Reads dev inputs only; every parity gate in the
ADR's Registration must pass before any share is computed.
"""
import argparse
import collections
import hashlib
import json
import sys

import rr_common as rc

THRESHOLDS = {'step2': 0.50, 'step3': 0.25, 'step4': 0.50}
DESCRIPTOR_ORDER = ['H', 'C', 'N']


def content_terms(query):
    tagged = rc.tokenize_tagged(query)
    eligible = {t for t, e in tagged if e}
    q = rc.query_terms(query)
    return q, [t for t in q if t not in rc.STOPWORDS], eligible


def section_vocab(corpus, section):
    toks, elig = set(), set()
    for i in corpus.by_section.get(section, []):
        for t, e in corpus.head_tagged[i] + corpus.body_tagged[i]:
            toks.add(t)
            if e:
                elig.add(t)
    return toks, elig


def lexical_group(q_all, q_c, q_elig, toks, elig):
    """Registered order: shared content term, then surface-form pair, then none."""
    if any(t in toks for t in q_c):
        return 'shared_term_below_50', []
    pairs = sorted({(t, u) for t in q_c if t in q_elig for u in elig if rc.surface_pair(t, u)})
    if pairs:
        return 'surface_form_variant', [list(p) for p in pairs]
    return 'no_shared_content_term', []


def descriptors(corpus, section, q_c, hits):
    qc = set(q_c)
    head = set(rc.tokenize('\n'.join(corpus.section_heading.get(section, []))))
    mine = [i for i, _ in hits if corpus.section[i] == section]
    best = set(corpus.tokens[mine[0]]) if mine else set()
    return {'H': int(bool(qc & head)), 'C': len(qc & best), 'N': len(mine)}


def favours(value, comparators):
    return value > rc.median(comparators)


def truncated_ceiling_query(qrel, top10, truncated_sections):
    g = [qrel.get(s, 0) for s in top10] + [qrel[s] for s in truncated_sections]
    return rc.dcg(sorted(g, reverse=True)[:rc.CUTOFF]) / rc.dcg(sorted(qrel.values(), reverse=True)[:rc.CUTOFF])


def route(shares):
    out = {'step2': None, 'step3': False, 'step4_signal': False}
    best = None
    for d in DESCRIPTOR_ORDER:
        s = shares['depth'][d]['share']
        if s is not None and s >= THRESHOLDS['step2'] and (best is None or s > best[1]):
            best = (d, s)
    out['step2'] = best[0] if best else None
    sf = shares['lexical']['surface_form_variant']['share']
    ns = shares['lexical']['no_shared_content_term']['share']
    out['step3'] = sf is not None and sf >= THRESHOLDS['step3']
    out['step4_signal'] = ns is not None and ns >= THRESHOLDS['step4']
    out['none'] = out['step2'] is None and not out['step3'] and not out['step4_signal']
    return out


def self_test():
    rc.load_char_props(['Straße ÜBER', '一', '\u0902'])
    # Rust: U+4E00 is not numeric (Lo) and U+0902 is alphabetic; Python's str methods disagree.
    assert not rc.is_numeric('一') and '一'.isnumeric()
    assert rc.is_alnum('\u0902') and not '\u0902'.isalnum()
    toks = rc.tokenize('getUserName snake_case HTTPServer, Straße ÜBER v2Api x')
    assert toks == ['getusername', 'get', 'user', 'name', 'snake_case', 'snake', 'case', 'httpserver',
                    'http', 'server', 'straße', 'über', 'v2api', 'v2', 'api', 'x'], toks
    assert rc.tokenize('__init__ --- _x') == ['init', 'x']
    assert rc.tokenize('  !? ') == []
    assert rc.query_terms('b a B a_c') == ['b', 'a', 'a_c', 'c']
    tagged = dict(rc.tokenize_tagged('getUser users v2 ab Cache'))
    assert tagged == {'getuser': False, 'get': False, 'user': False, 'users': True, 'v2': False,
                      'ab': False, 'cache': True}, tagged
    # BM25 toy from src/search/bm25.rs: d0 "a b a", d1 "b c", d2 "c c c d".
    bm = rc.Bm25([['a', 'b', 'a'], ['b', 'c'], ['c', 'c', 'c', 'd']])
    for got, want in zip(bm.scores(['a', 'c']), [1.3486402228911236, 0.5442147286003255, 0.6893386562270789]):
        assert abs(got - want) < 1e-12, (got, want)
    # d0 "", d1 "a": (22/31) ln 2.
    import math
    assert abs(rc.Bm25([[], ['a']]).scores(['a'])[1] - 22 / 31 * math.log(2)) < 1e-12
    assert rc.rank([0.0, 2.0, 2.0, 1.0], ['d', 'c', 'b', 'a'], 2) == [(2, 2.0), (1, 2.0)]
    assert rc.project([(0, 3.0), (1, 2.0), (2, 1.0)], ['s1', 's1', 's2'], 10) == [('s1', 3.0), ('s2', 1.0)]
    for a, b, want in [('index', 'indexes', True), ('config', 'configs', True), ('query', 'queries', True),
                       ('cache', 'cached', True), ('cache', 'caching', True), ('map', 'mapped', True),
                       ('run', 'running', True), ('box', 'boxes', True), ('class', 'classes', True),
                       ('class', 'classs', False), ('index', 'index', False), ('day', 'daies', False),
                       ('build', 'builder', False), ('use', 'usage', False), ('lock', 'locked', True)]:
        assert rc.surface_pair(a, b) is want, (a, b)
    assert rc.fold_key_map(['cache', 'cached', 'caches', 'caching']) == {
        'cached': 'cache', 'caches': 'cache', 'caching': 'cache'}
    q, qc, qe = content_terms('How do indexes work')
    assert q == ['how', 'do', 'indexes', 'work'] and qc == ['indexes', 'work'] and 'indexes' in qe
    assert lexical_group(q, qc, qe, {'work', 'the'}, {'work'})[0] == 'shared_term_below_50'
    assert lexical_group(q, qc, qe, {'index', 'how'}, {'index'}) == ('surface_form_variant', [['indexes', 'index']])
    assert lexical_group(q, qc, qe, {'how', 'zebra'}, {'zebra'})[0] == 'no_shared_content_term'
    q2, qc2, qe2 = content_terms('getIndexes')
    assert lexical_group(q2, qc2, qe2, {'index'}, {'index'})[0] == 'no_shared_content_term'
    assert rc.median([1, 3, 2]) == 2 and rc.median([1, 2, 3, 4]) == 2.5
    assert favours(3, [1, 2, 3]) and not favours(2, [1, 2, 3])
    # nDCG: qrel a:2 b:1 c:0, ranking a c b -> (2 + 0 + 1/log2 4) / (2 + 1/log2 3) = 0.9502344167898356.
    assert abs(rc.ndcg({'a': 2, 'b': 1, 'c': 0}, ['a', 'c', 'b']) - 0.9502344167898356) < 1e-12
    assert rc.condensed_ndcg({'a': 2, 'b': 1}, ['x', 'a', 'b']) == 1.0
    assert rc.judged_at({'a': 1}, ['a', 'x']) == 0.5
    assert abs(truncated_ceiling_query({'a': 2, 'b': 1}, ['b', 'x'], ['a']) - 1.0) < 1e-12
    rng = rc.SplitMix64(0)
    assert rng.next_u64() == 0xe220a8397b1dcdaf and rng.next_u64() == 0x6e789e6aa1b965f4
    assert abs(rc.quantile([0.0, 1.0, 2.0, 3.0], 0.5) - 1.5) < 1e-12
    sh = rc.cluster_share([(1, 2), (1, 2)], resamples=50)
    assert sh['share'] == 0.5 and sh['cluster95'] == [0.5, 0.5]
    shares = {'depth': {'H': {'share': 0.5}, 'C': {'share': 0.6}, 'N': {'share': 0.6}},
              'lexical': {'surface_form_variant': {'share': 0.2}, 'no_shared_content_term': {'share': 0.5}}}
    r = route(shares)
    assert r == {'step2': 'C', 'step3': False, 'step4_signal': True, 'none': False}, r
    shares['depth'] = {'H': {'share': 0.49}, 'C': {'share': None}, 'N': {'share': 0.1}}
    shares['lexical']['no_shared_content_term']['share'] = 0.1
    assert route(shares)['none']
    print('self-test ok')


def main(out_path):
    inp = rc.load_inputs()
    binary_sha = rc.sha256(rc.BINARY)
    corpora = {n: rc.Corpus(n, g) for n, g in sorted(inp['gens'].items())}
    gates = {'input_hashes_match_hy0': True, 'current_pinned_before': True}
    queries, qrels, b1 = inp['queries'], inp['qrels'], inp['b1']
    hits_by_q = {}
    for qid in inp['answerable']:
        q = queries[qid]; c = corpora[q['corpus']]
        hits = c.b1(q['text'])
        resp = rc.binary_top50(c.root, q['text'])
        bin_hits = [(r['chunk_id'], r['score']) for r in resp['results']]
        if resp['generation_id'] != c.gen_id:
            raise rc.GateError(f'{qid}: binary generation {resp["generation_id"]}')
        if len(bin_hits) != len(hits) or any(c.ids[i] != cid or not rc.close(sc, bsc)
                                             for (i, sc), (cid, bsc) in zip(hits, bin_hits)):
            raise rc.GateError(f'{qid}: Python BM25 differs from the binary top 50')
        top10 = rc.project(hits, c.section, rc.CUTOFF)
        want = b1.get(qid, [])
        if [s for s, _ in top10] != [s for _, s, _ in want] or any(
                not rc.close(a, w) for (_, a), (_, _, w) in zip(top10, want)):
            raise rc.GateError(f'{qid}: projection differs from B1.tsv')
        hits_by_q[qid] = hits
    gates['binary_top50_parity'] = True
    gates['python_bm25_parity'] = True
    gates['b1_tsv_parity'] = True

    pairs = []
    for qid in inp['answerable']:
        q = queries[qid]; c = corpora[q['corpus']]; hits = hits_by_q[qid]
        top10 = [s for s, _ in rc.project(hits, c.section, rc.CUTOFF)]
        in50 = {c.section[i] for i, _ in hits}
        terms = rc.query_terms(q['text'])
        scores = None
        for s, g in sorted(qrels[qid].items()):
            if g < 1 or s in top10:
                continue
            own = c.by_section.get(s)
            if not own:
                stage = 'not_indexed'
            elif s in in50:
                stage = 'truncated'
            else:
                if scores is None:
                    scores = c.bm25.scores(terms)
                stage = 'lexical_miss'
                if all(scores[i] <= 0.0 for i in own):
                    toks, _ = section_vocab(c, s)
                    if any(t in toks for t in terms):
                        stage = 'chunk_boundary_candidate'
            pairs.append({'query_id': qid, 'section_id': s, 'stage': stage})
    want = sorted((m['query_id'], m['section_id'], m['stage']) for m in inp['misses'])
    got = sorted((p['query_id'], p['section_id'], p['stage']) for p in pairs)
    if got != want:
        raise rc.GateError(f'recomputed B1 misses differ from misses.json ({len(got)} vs {len(want)})')
    gates['misses_json_parity'] = True

    lex_clusters = collections.defaultdict(collections.Counter)
    depth_clusters = collections.defaultdict(collections.Counter)
    trunc_groups = collections.Counter(); bands = collections.Counter()
    for p in pairs:
        qid = p['query_id']; q = queries[qid]; c = corpora[q['corpus']]; hits = hits_by_q[qid]
        q_all, q_c, q_elig = content_terms(q['text'])
        if p['stage'] == 'lexical_miss':
            toks, elig = section_vocab(c, p['section_id'])
            group, sf = lexical_group(q_all, q_c, q_elig, toks, elig)
            p['group'] = group; p['surface_pairs'] = sf
            p['stopword_only_overlap'] = group == 'no_shared_content_term' and any(t in toks for t in q_all)
            lex_clusters[qid][group] += 1; lex_clusters[qid]['_n'] += 1
        elif p['stage'] == 'truncated':
            ranked = [s for s, _ in rc.project(hits, c.section, rc.FETCH)]
            srank = ranked.index(p['section_id']) + 1
            p['section_rank'] = srank
            p['band'] = '11-20' if srank <= 20 else '21-50'
            n_rel = sum(1 for g in qrels[qid].values() if g >= 1)
            p['group'] = 'more_than_ten_relevant' if n_rel > rc.CUTOFF else 'remainder'
            trunc_groups[p['group']] += 1; bands[(p['group'], p['band'])] += 1
            if p['group'] == 'remainder':
                top10 = [s for s, _ in rc.project(hits, c.section, rc.CUTOFF)]
                d = descriptors(c, p['section_id'], q_c, hits)
                comp = {k: [descriptors(c, s, q_c, hits)[k] for s in top10] for k in DESCRIPTOR_ORDER}
                p['descriptors'] = d
                p['top10_median'] = {k: rc.median(comp[k]) for k in DESCRIPTOR_ORDER}
                p['favours'] = {k: favours(d[k], comp[k]) for k in DESCRIPTOR_ORDER}
                depth_clusters[qid]['_n'] += 1
                for k in DESCRIPTOR_ORDER:
                    depth_clusters[qid][k] += int(p['favours'][k])
    lex_groups = ['shared_term_below_50', 'surface_form_variant', 'no_shared_content_term']
    shares = {
        'lexical': {g: rc.cluster_share([(c[g], c['_n']) for _, c in sorted(lex_clusters.items())])
                    for g in lex_groups},
        'depth': {k: rc.cluster_share([(c[k], c['_n']) for _, c in sorted(depth_clusters.items())])
                  for k in DESCRIPTOR_ORDER},
    }
    base, trunc, full = [], [], []
    stage_of = {(p['query_id'], p['section_id']): p['stage'] for p in pairs}
    for qid in inp['answerable']:
        c = corpora[queries[qid]['corpus']]
        top10 = [s for s, _ in rc.project(hits_by_q[qid], c.section, rc.CUTOFF)]
        tr = [s for s in qrels[qid] if stage_of.get((qid, s)) == 'truncated']
        lx = [s for s in qrels[qid] if stage_of.get((qid, s)) == 'lexical_miss']
        base.append(rc.ndcg(qrels[qid], top10))
        trunc.append(truncated_ceiling_query(qrels[qid], top10, tr))
        full.append(truncated_ceiling_query(qrels[qid], top10, tr + lx))
    mean = lambda v: sum(v) / len(v)
    hy0 = inp['hy0']
    if not rc.close(mean(base), hy0['b1_ndcg10']) or not rc.close(mean(full) - mean(base), hy0['oracle_gain_recover_all_misses']):
        raise rc.GateError('ceiling recomputation differs from HY-0')
    gates['hy0_ceiling_parity'] = True
    for n, c in corpora.items():
        if open(rc.path(c.root + '/.ontodoc/CURRENT')).read().strip() != c.gen_id:
            raise rc.GateError(f'{n}: CURRENT moved during the run')
    gates['current_pinned_after'] = True
    stage_counts = collections.Counter(p['stage'] for p in pairs)
    lex_counts = collections.Counter(p['group'] for p in pairs if p['stage'] == 'lexical_miss')
    out = {
        'adr': 'ADR_ONTODOC_RETRIEVAL_RESEARCH revision 4, Registration',
        'label_track': 'model-judged exploratory; no decision or default change',
        'inputs_sha256': inp['sha'],
        'hy0_summary_sha256': rc.sha256(rc.HY0_SUMMARY),
        'binary_sha256': binary_sha,
        'generations': {n: {'generation_id': c.gen_id, 'artifact_sha256': c.artifact_sha256} for n, c in corpora.items()},
        'stopwords_sha256': hashlib.sha256(' '.join(sorted(rc.STOPWORDS)).encode()).hexdigest(),
        'seed': rc.SEED, 'resamples': rc.RESAMPLES,
        'gates': gates,
        'answerable_dev_queries': len(inp['answerable']),
        'b1_miss_pairs_by_stage': dict(sorted(stage_counts.items())),
        'lexical_miss_groups': {g: lex_counts.get(g, 0) for g in lex_groups},
        'lexical_stopword_only_overlap': sum(1 for p in pairs if p.get('stopword_only_overlap')),
        'truncated_groups': dict(sorted(trunc_groups.items())),
        'truncated_bands': {f'{g}/{b}': n for (g, b), n in sorted(bands.items())},
        'shares': shares,
        'b1_ndcg10': mean(base),
        'oracle_gain_truncated_only': mean(trunc) - mean(base),
        'oracle_gain_recover_all_misses': mean(full) - mean(base),
        'thresholds': THRESHOLDS,
        'route': route(shares),
        'pairs': pairs,
    }
    with open(rc.path(out_path), 'w', encoding='utf-8') as f:
        json.dump(out, f, indent=1, sort_keys=True, ensure_ascii=False); f.write('\n')
    summary = {k: v for k, v in out.items() if k != 'pairs'}
    json.dump(summary, sys.stdout, indent=1, sort_keys=True); print()


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--self-test', action='store_true')
    ap.add_argument('--out')
    a = ap.parse_args()
    if a.self_test:
        self_test()
    elif a.out:
        try:
            main(a.out)
        except rc.GateError as e:
            print(f'GATE FAILED: {e}', file=sys.stderr); sys.exit(2)
    else:
        ap.error('--self-test or --out required')
