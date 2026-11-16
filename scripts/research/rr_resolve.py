"""Resolve one routed step of ADR_ONTODOC_RETRIEVAL_RESEARCH rev 4 from step 1's recorded route.

Usage: rr_resolve.py --self-test | --step {2,3} --out PATH. A step that step 1 did not open is
recorded as not opened; an opened step needs its own diagnostic script, which this revision lacks.
"""
import argparse
import json
import sys

import rr_common as rc

STEP1 = 'tmp/retrieval-research/step1.json'


def resolve(route, step):
    opened = route['step2'] is not None if step == 2 else bool(route['step3'])
    return {'step': step, 'opened': opened, 'candidate': route['step2'] if step == 2 else None}


def self_test():
    r = {'step2': None, 'step3': False, 'step4_signal': False, 'none': True}
    assert resolve(r, 2) == {'step': 2, 'opened': False, 'candidate': None}
    assert resolve(r, 3)['opened'] is False
    r = {'step2': 'H', 'step3': True, 'step4_signal': False, 'none': False}
    assert resolve(r, 2) == {'step': 2, 'opened': True, 'candidate': 'H'}
    assert resolve(r, 3)['opened'] is True
    print('self-test ok')


def main(step, out):
    s1 = json.load(open(rc.path(STEP1)))
    if not all(s1['gates'].values()):
        raise rc.GateError('step 1 gates did not all pass')
    rec = resolve(s1['route'], step)
    rec.update({'step1_sha256': rc.sha256(STEP1), 'adr': s1['adr'], 'route': s1['route']})
    if rec['opened']:
        raise rc.GateError(f'step {step} opened; its diagnostic script is required')
    with open(rc.path(out), 'w', encoding='utf-8') as f:
        json.dump(rec, f, indent=1, sort_keys=True); f.write('\n')
    json.dump(rec, sys.stdout, indent=1, sort_keys=True); print()


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--self-test', action='store_true')
    ap.add_argument('--step', type=int, choices=[2, 3])
    ap.add_argument('--out')
    a = ap.parse_args()
    if a.self_test:
        self_test()
    elif a.step and a.out:
        try:
            main(a.step, a.out)
        except rc.GateError as e:
            print(f'GATE FAILED: {e}', file=sys.stderr); sys.exit(2)
    else:
        ap.error('--self-test, or --step and --out, required')
