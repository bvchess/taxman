#!/usr/bin/env python3
"""The panel's measure: per game, the total CPU and steps of a setting against a reference over the units solved under
both, and the geometric mean of the per-game total-CPU ratios.  Usage: panel.py DIRS REF NAME... (DIRS: folders,
comma-separated)"""
import collections
import glob
import json
import math
import os
import sys

dirs, ref, names = sys.argv[1], sys.argv[2], sys.argv[3:]
res = collections.defaultdict(dict)
for f in [f for d in dirs.split(',') for f in glob.glob(os.path.join(d, '*.json'))]:
    tag, uid = os.path.basename(f)[:-5].split('-', 1)
    r = json.loads(open(f).read().strip().split('\n')[-1])
    res[uid][tag] = (r['nodes'], r['cpu_s'], r['completed'])
for name in names:
    ratios = []
    parts = []
    for game in ('4984', '4995', '5052'):
        a = b = sa = sb = n = 0
        for uid, t in res.items():
            if uid.startswith(game) and ref in t and name in t:
                a += t[ref][1]
                b += t[name][1]
                sa += t[ref][0]
                sb += t[name][0]
                n += 1
        if n:
            ratios.append(b / a)
            parts.append(f"{game} ({n}) cpu {b / a:.3f} steps {sb / sa:.3f}")
    if ratios:
        gm = math.exp(sum(map(math.log, ratios)) / len(ratios))
        print(f"{name:>12} vs {ref}: " + ' | '.join(parts) + f" | panel {gm:.3f}")
