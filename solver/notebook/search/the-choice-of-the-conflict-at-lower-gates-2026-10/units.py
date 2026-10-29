#!/usr/bin/env python3
"""Steps of each unit under settings, the largest units first.  Usage: units.py DIRS GAME NAME..."""
import collections
import glob
import json
import os
import sys

dirs, game, names = sys.argv[1], sys.argv[2], sys.argv[3:]
gaps = json.load(open('/home/brian/taxman_work/lab/work/choice-2026-10-28/gaps.json'))
res = collections.defaultdict(dict)
for f in [f for d in dirs.split(',') for f in glob.glob(os.path.join(d, '*.json'))]:
    tag, uid = os.path.basename(f)[:-5].split('-', 1)
    if uid.startswith(game) and tag in names:
        r = json.loads(open(f).read().strip().split('\n')[-1])
        res[uid][tag] = r['nodes']
print(f"{'unit':12} {'gap':>5} " + ' '.join(f'{n[:9]:>9}' for n in names))
for uid in sorted(res, key=lambda u: -max(res[u].values())):
    print(f"{uid:12} {gaps[uid]:5} " + ' '.join(f"{res[uid].get(n, 0):9}" for n in names))
