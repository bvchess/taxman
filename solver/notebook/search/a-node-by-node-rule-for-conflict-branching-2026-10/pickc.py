#!/usr/bin/env python3
"""Picks fresh validation units of 4983 and 4984 (none picked before), by gap band, into ids-GAME-c.ids."""
import json
import random

W = '/home/brian/taxman_work/lab/work/branch-2026-10-27/'
random.seed(101)
used = set()
for f in ['ids-4983-a.txt', 'ids-4983-b.txt', 'ids-4984-extra.txt', 'ids-4984-b.txt']:
    used |= set(line.split()[0] for line in open(W + f))
used |= set(open('/home/brian/taxman_work/lab/work/recheck4984/sample.txt').read().split())
for game, path in ((4983, '/home/brian/taxman_work/lab/work/hardgames/forecast/units/live4983.jsonl'),
                   (4984, '/home/brian/taxman_work/lab/work/patch/units-4984.jsonl')):
    units = [json.loads(line) for line in open(path)]
    units = [u for u in units if u.get('kind') == 'unit' and u['id'] not in used]
    out = []
    for lo, hi, k in [(1150, 1300, 12), (1300, 1450, 12), (1450, 1600, 10), (1600, 1700, 6)]:
        c = [u for u in units if lo <= u['gap'] < hi]
        out += [(u['gap'], u['id']) for u in random.sample(c, k)]
    with open(W + 'ids-%d-c.ids' % game, 'w') as f:
        for g, i in sorted(out):
            f.write(i + '\n')
    print(game, len(out))
