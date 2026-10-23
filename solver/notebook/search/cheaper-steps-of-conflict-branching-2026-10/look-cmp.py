#!/usr/bin/env python3
"""cmp.py A B: runs/A and runs/B side by side: steps, best, the counters (same or not), CPU (user + sys) and ratio."""
import json
import os
import sys

S = '/tmp/claude-1001/-home-brian-taxman-work-research/ee866c42-a3bc-4d6b-b3f5-6d2038b6659f/scratchpad/runs'


def load(label):
    out = {}
    for f in sorted(os.listdir(f'{S}/{label}')):
        if f.endswith('.jsonl'):
            uid = f[:-6]
            lines = [json.loads(l) for l in open(f'{S}/{label}/{f}') if l.strip()]
            t = open(f'{S}/{label}/{uid}.time').read().split()
            out[uid] = (lines[-1] if lines else None, float(t[0]) + float(t[1]))
    return out


a, b = load(sys.argv[1]), load(sys.argv[2])
ta = tb = 0
for uid in a:
    if uid not in b:
        continue
    (ra, ca), (rb, cb) = a[uid], b[uid]
    ta += ca
    tb += cb
    sa = {k: v for k, v in (ra.get('stats') or {}).items() if not k.startswith('gf2')}
    sb = {k: v for k, v in (rb.get('stats') or {}).items() if not k.startswith('gf2')}
    diff = [k for k in sa if sa[k] != sb.get(k)]
    print(f"{uid}: steps {ra.get('nodes')} {rb.get('nodes')} best {ra.get('best')} {rb.get('best')} "
          f"counters {'same' if not diff else 'DIFFER ' + ','.join(diff)} cpu {ca:.1f} {cb:.1f} ratio {cb / ca:.3f}")
print(f'all: cpu {ta:.1f} {tb:.1f} ratio {tb / ta:.3f}')
