#!/usr/bin/env python3
"""Gathers the paired solves of the folders DIR... into one jsonl line a solve on stdout: the folder (`run`), the arm
(`tag`), the unit's `id`, the `rep`, and the solve's `cpu_s`, `nodes` (steps), `best` and `completed`.
Usage: collect.py DIR... > solves.jsonl"""
import glob
import json
import os
import sys

for d in sys.argv[1:]:
    for f in sorted(glob.glob(os.path.join(d, '*.json'))):
        tag, rest = os.path.basename(f)[:-5].split('-', 1)
        uid, rep = rest.rsplit('-', 1)
        s = json.loads(open(f).read().strip().split('\n')[-1])
        row = {'run': os.path.basename(os.path.normpath(d)), 'tag': tag, 'id': uid, 'rep': int(rep),
               'cpu_s': round(s['cpu_s'], 6), 'nodes': s['nodes'], 'best': s['best'], 'completed': s['completed']}
        print(json.dumps(row))
