#!/usr/bin/env python3
"""Every solve of the study, a line each: its folder, setting, unit, gap, steps, CPU seconds, whether it finished, and
its best.  Usage: runs.py DIR... > runs.txt"""
import glob
import json
import os
import sys

gaps = json.load(open('/home/brian/taxman_work/lab/work/choice-2026-10-28/gaps.json'))
print('folder setting unit gap steps cpu_s finished best')
for d in sys.argv[1:]:
    for f in sorted(glob.glob(os.path.join(d, '*.json'))):
        tag, uid = os.path.basename(f)[:-5].split('-', 1)
        r = json.loads(open(f).read().strip().split('\n')[-1])
        print(os.path.basename(d.rstrip('/')), tag, uid, gaps.get(uid, ''), r['nodes'], f"{r['cpu_s']:.2f}",
              int(r['completed']), r['best'])
