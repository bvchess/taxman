#!/usr/bin/env python3
"""Emulates a gate set by a unit's own gap: units of gap under TOP take setting LOW's result, the others REF's; per game
the total CPU and steps against REF, and the panel's geometric mean.  Usage: toprule.py DIRS REF LOW TOP"""
import collections
import glob
import json
import math
import os
import sys

dirs, ref, low, top = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
gaps = json.load(open('/home/brian/taxman_work/lab/work/choice-2026-10-28/gaps.json'))
res = collections.defaultdict(dict)
for f in [f for d in dirs.split(',') for f in glob.glob(os.path.join(d, '*.json'))]:
    tag, uid = os.path.basename(f)[:-5].split('-', 1)
    r = json.loads(open(f).read().strip().split('\n')[-1])
    res[uid][tag] = (r['nodes'], r['cpu_s'])
ratios, parts = [], []
for game in ('4984', '4995', '5052'):
    a = b = sa = sb = 0
    for uid, t in res.items():
        if uid.startswith(game) and ref in t and low in t:
            pick = t[low] if gaps[uid] < top else t[ref]
            a += t[ref][1]
            b += pick[1]
            sa += t[ref][0]
            sb += pick[0]
    ratios.append(b / a)
    parts.append(f"{game} cpu {b / a:.3f} steps {sb / sa:.3f}")
print(f"{low} under {top}, else {ref}: " + ' | '.join(parts)
      + f" | panel {math.exp(sum(map(math.log, ratios)) / len(ratios)):.3f}")
