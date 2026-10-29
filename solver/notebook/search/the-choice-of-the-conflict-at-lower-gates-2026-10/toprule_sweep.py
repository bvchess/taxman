#!/usr/bin/env python3
"""toprule.py on the gate sweep of 2026-10-28 (lab/work/gate-2026-10-28/GAME/out): units of gap under TOP take setting
LOW's result, the others REF's; per game the total CPU and steps against REF.  Usage: toprule_sweep.py REF LOW TOP"""
import glob
import json
import math
import os
import sys

ref, low, top = sys.argv[1], sys.argv[2], int(sys.argv[3])
root = '/home/brian/taxman_work/lab/work/gate-2026-10-28'
ratios, parts = [], []
for game in ('4984', '4992', '4995', '5052'):
    gaps = {}
    for line in open(f'{root}/{game}/units-{game}.jsonl'):
        r = json.loads(line)
        if r.get('kind') == 'unit':
            gaps[r['id']] = r['gap']
    res = {}
    for f in glob.glob(f'{root}/{game}/out/*.json'):
        tag, uid = os.path.basename(f)[:-5].split('-', 1)
        r = json.loads(open(f).read().strip().split('\n')[-1])
        res.setdefault(uid, {})[tag] = (r['nodes'], r['cpu_s'])
    a = b = sa = sb = 0
    for uid, t in res.items():
        if ref in t and low in t:
            pick = t[low] if gaps[uid] < top else t[ref]
            a += t[ref][1]
            b += pick[1]
            sa += t[ref][0]
            sb += pick[0]
    ratios.append(b / a)
    parts.append(f"{game} cpu {b / a:.3f} steps {sb / sa:.3f}")
print(f"{low} under {top}, else {ref}: " + ' | '.join(parts)
      + f" | panel {math.exp(sum(map(math.log, ratios)) / len(ratios)):.3f}")
