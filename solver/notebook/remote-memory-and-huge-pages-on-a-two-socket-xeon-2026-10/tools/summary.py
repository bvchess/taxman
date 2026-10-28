#!/usr/bin/env python3
"""Summarizes solves.jsonl: for each run (a folder of paired solves) and its two arms, the CPU of the second arm over
the first's, as the ratio of the sums, the geometric mean of the unit ratios and their range, over all reps and by
rep; and any pair whose steps or best differ.  Usage: summary.py solves.jsonl"""
import collections
import json
import math
import sys

# The arms of each run: the one compared against first.
ARMS = {'pen1': ('local', 'remote'), 'thpA': ('base', 'thpmain'), 'thpB': ('base', 'thp'), 'thpC': ('base', 'arena1')}

solves = collections.defaultdict(dict)
for line in open(sys.argv[1]):
    s = json.loads(line)
    solves[s['run']][(s['tag'], s['id'], s['rep'])] = s


def report(name, pairs):
    sa, sb = sum(a['cpu_s'] for a, _ in pairs), sum(b['cpu_s'] for _, b in pairs)
    ratios = [b['cpu_s'] / a['cpu_s'] for a, b in pairs]
    gm = math.exp(sum(math.log(x) for x in ratios) / len(ratios))
    print(f"  {name:6} pairs {len(pairs):3}  CPU {sb / sa:.4f}  geometric mean {gm:.4f}  "
          f"range {min(ratios):.3f}..{max(ratios):.3f}  ({sa:.1f} s against {sb:.1f} s)")


for run, (ta, tb) in ARMS.items():
    got = solves[run]
    pairs = [(a, got[(tb, uid, rep)]) for (tag, uid, rep), a in sorted(got.items())
             if tag == ta and (tb, uid, rep) in got]
    if not pairs:
        continue
    print(f"{run}: {tb} over {ta}")
    for a, b in pairs:
        if (a['nodes'], a['best']) != (b['nodes'], b['best']):
            print(f"  differ: {a['id']} rep {a['rep']}")
    report('all', pairs)
    for rep in sorted({a['rep'] for a, _ in pairs}):
        report(f'rep {rep}', [(a, b) for a, b in pairs if a['rep'] == rep])
