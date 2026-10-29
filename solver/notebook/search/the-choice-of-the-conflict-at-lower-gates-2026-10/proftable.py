#!/usr/bin/env python3
"""The profile of the choice as a markdown table: per game and setting, the choices a step, the choice's share of the
steps' cycles and its parts, and the CPU of a choice.  Usage: proftable.py OUT NAME..."""
import collections
import glob
import json
import os
import sys

out, names = sys.argv[1], sys.argv[2:]
cyc = collections.defaultdict(collections.Counter)
cnt = collections.defaultdict(collections.Counter)
tot = collections.defaultdict(lambda: [0, 0.0])
for f in glob.glob(os.path.join(out, '*.json')):
    tag, uid = os.path.basename(f)[:-5].split('-', 1)
    if tag not in names:
        continue
    key = (tag, uid.split('-')[0])
    r = json.loads(open(f).read().strip().split('\n')[-1])
    tot[key][0] += r['nodes']
    tot[key][1] += r['cpu_s']
    for line in open(f[:-5] + '.err'):
        if line.startswith('PROF'):
            _, i, c, n = line.split()
            cyc[key][int(i)] += int(c)
            cnt[key][int(i)] += int(n)
PARTS = [(3, 'exact prices'), (7, 'ordered conflicts'), (5, 'split estimates'), (1, 'conflict members'), (9, 'look'),
         (6, 'table conflicts')]
print('| game | setting | steps | CPU s | choices a step | choice, % of a step | '
      + ' | '.join(p[1] for p in PARTS) + ' | rest | us a choice |')
print('|' + '---|' * (8 + len(PARTS)))
for game in ('4984', '4995', '5052'):
    for name in names:
        key = (name, game)
        if key not in tot:
            continue
        c, n = cyc[key], cnt[key]
        step = c[64]
        share = 100 * c[0] / step
        parts = [100 * c[i] / step for i, _ in PARTS]
        rest = share - sum(parts)
        print(f"| {game} | {name} | {tot[key][0]:,} | {tot[key][1]:,.0f} | {n[0] / tot[key][0]:.3f} | {share:.1f} | "
              + ' | '.join(f'{p:.1f}' for p in parts) + f" | {rest:.1f} | {c[0] / max(n[0], 1) / 2200:,.0f} |")
