#!/usr/bin/env python3
"""Totals of a run of run.py by setting and game: units, steps, CPU, and the scratch counters (PROF lines of the .err
files) as shares of the steps' cycles.  Usage: summ.py OUT [IDS]"""
import collections
import glob
import json
import os
import sys

out = sys.argv[1]
only = None
if len(sys.argv) > 2:
    only = {line.split()[0] for line in open(sys.argv[2]) if line.strip()}
NAMES = {
    0: 'choice', 1: 'conflict_members', 2: 'memo.take', 3: 'prices_excluding', 4: 'inherit_pricing',
    5: 'price map (split)', 6: 'table conflicts', 7: 'ordered conflicts', 9: 'look', 10: 'nogood insert',
    40: '  ordered: assert peel', 41: '  ordered: all', 50: '  pricing: all', 51: '  pricing: resolve',
    52: '  pricing: losses+reach', 53: '  pricing: carry check', 54: '  pricing: searches', 55: '  pricing: carried',
    58: '  pricing: certify', 60: 'bound (evaluate)', 61: 'conflict (seek)', 62: 'lagrange', 63: 'strong branching',
    64: 'step', 70: 'strong: certified_excluding',
}
COUNTS = {11: 'chose other than K', 14: 'members', 15: 'exact members', 30: 'priced members', 31: 'ordered made',
          32: 'ordered kept', 56: 'found within reach', 57: 'not found'}
runs = collections.defaultdict(lambda: {'n': 0, 'steps': 0, 'cpu': 0.0, 'done': 0, 'cyc': collections.Counter(),
                                        'cnt': collections.Counter()})
for f in glob.glob(os.path.join(out, '*.json')):
    name = os.path.basename(f)[:-5]
    tag, uid = name.split('-', 1)
    if only and uid not in only:
        continue
    r = json.loads(open(f).read().strip().split('\n')[-1])
    key = (tag, uid.split('-')[0])
    s = runs[key]
    s['n'] += 1
    s['steps'] += r['nodes']
    s['cpu'] += r['cpu_s']
    s['done'] += r['completed']
    try:
        for line in open(f[:-5] + '.err'):
            if line.startswith('PROF'):
                _, i, c, n = line.split()
                s['cyc'][int(i)] += int(c)
                s['cnt'][int(i)] += int(n)
    except FileNotFoundError:
        pass
for key in sorted(runs):
    s = runs[key]
    step = s['cyc'][64] or 1
    print(f"{key[0]:>10} {key[1]}: units {s['n']} finished {s['done']} steps {s['steps']} cpu {s['cpu']:.1f} s "
          f"({1000 * s['cpu'] / max(s['steps'], 1):.3f} ms a step); choices {s['cnt'][0]} "
          f"({s['cnt'][0] / max(s['steps'], 1):.3f} a step)")
    for i, label in NAMES.items():
        if s['cnt'][i]:
            print(f"    {label:28} {100 * s['cyc'][i] / step:6.2f}%  calls {s['cnt'][i]:>9}  "
                  f"{s['cyc'][i] / s['cnt'][i] / 2200:9.1f} us a call")
    print('    ' + '; '.join(f"{label} {s['cyc'][i]}" for i, label in COUNTS.items() if s['cnt'][i]))
