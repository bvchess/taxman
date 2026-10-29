#!/usr/bin/env python3
"""Steps and CPU of settings against a reference, by game and gap band, over the units solved under all of them.
Usage: bands.py OUT REF NAME..."""
import collections
import glob
import json
import math
import os
import sys

out, ref, names = sys.argv[1], sys.argv[2], sys.argv[3:]
gaps = json.load(open('/home/brian/taxman_work/lab/work/choice-2026-10-28/gaps.json'))
res = collections.defaultdict(dict)
files = [f for d in out.split(',') for f in glob.glob(os.path.join(d, '*.json'))]
for f in files:
    tag, uid = os.path.basename(f)[:-5].split('-', 1)
    r = json.loads(open(f).read().strip().split('\n')[-1])
    res[uid][tag] = (r['nodes'], r['cpu_s'], r['completed'], r['best'])
BANDS = [(300, 600), (600, 900), (900, 1200), (1200, 9999)]
for game in ('4984', '4995', '5052'):
    for name in names:
        rows = []
        tot = collections.defaultdict(lambda: [0, 0, 0.0, 0.0, 0])
        logs = []
        for uid, t in res.items():
            if not uid.startswith(game) or ref not in t or name not in t:
                continue
            g = gaps[uid]
            band = next(b for b in BANDS if b[0] <= g < b[1])
            a, b = t[ref], t[name]
            for key in (band, 'all'):
                x = tot[key]
                x[0] += a[0]
                x[1] += b[0]
                x[2] += a[1]
                x[3] += b[1]
                x[4] += 1
            logs.append(math.log(b[0] / a[0]))
            if a[3] != b[3]:
                print('  best differs', uid, a[3], b[3])
        if not logs:
            continue
        parts = []
        for key in BANDS + ['all']:
            if key in tot:
                x = tot[key]
                label = 'all' if key == 'all' else f'{key[0]}-'
                parts.append(f"{label}({x[4]}) st {x[1] / x[0]:.3f} cpu {x[3] / x[2]:.3f}")
        allx = tot['all']
        print(f"{game} {name:>8} vs {ref}: units {allx[4]} steps {allx[0]}->{allx[1]} cpu {allx[2]:.0f}->{allx[3]:.0f} | "
              + ' | '.join(parts) + f" | gm steps {math.exp(sum(logs) / len(logs)):.3f}")
