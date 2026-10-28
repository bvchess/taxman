#!/usr/bin/env python3
"""Per unit and setting: the steps, CPU, the phase shares and ms a timed step (PHASES lines), and the counters of the
result line.  Usage: phases.py DIR"""
import glob
import json
import os
import re
import sys

for f in sorted(glob.glob(os.path.join(sys.argv[1], '*.json'))):
    r = json.loads(open(f).read().strip().split('\n')[-1])
    err = f[:-5] + '.err'
    ph = [l for l in open(err) if l.startswith('PHASES')]
    line = '%-28s %7d %6.1f %.3f ms/step' % (os.path.basename(f)[:-5], r['nodes'], r['cpu_s'], 1000 * r['cpu_s'] / r['nodes'])
    if ph:
        kv = dict(re.findall(r'(\w+)=(\d+)', ph[-1]))
        steps = int(kv.pop('steps'))
        tot = sum(int(v) for v in kv.values())
        line += ' | timed %.3f ms/step: ' % (tot / steps / 1e6) + ' '.join(
            '%s %.3f' % (k[:5], int(v) / steps / 1e6) for k, v in kv.items() if int(v) > 0)
    s = r['stats']
    line += ' | solves %d aug %d strong %d/%d' % (s['gf2_solves'], s['gf2_augmentations'], s['gf2_strong_solves'],
                                                 s['gf2_strong_augmentations'])
    print(line)
