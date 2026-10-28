#!/usr/bin/env python3
"""Collects unit results (NAME-ID.json files of tune solve) from out folders into one table on stdout:
setting, id, gap, steps, cpu_s, completed.  Usage: collect.py DIR... (each DIR holds the .json files; the gaps come from
the units files named in GAMES)."""
import glob
import json
import os
import re
import sys

GAMES = {
    4983: '/home/brian/taxman_work/lab/work/hardgames/forecast/units/live4983.jsonl',
    4984: '/home/brian/taxman_work/lab/work/patch/units-4984.jsonl',
    4991: '/home/brian/taxman_work/lab/work/patch4991/units-4991.jsonl',
    4992: '/home/brian/taxman_work/lab/work/patch4992/units-4992.jsonl',
    4995: '/home/brian/taxman_work/lab/work/patch4995/units-4995.jsonl',
}


def gaps():
    out = {}
    for path in GAMES.values():
        for line in open(path):
            d = json.loads(line)
            if d.get('kind') == 'unit':
                out[d['id']] = d['gap']
    return out


def main():
    gap = gaps()
    for d in sys.argv[1:]:
        for f in sorted(glob.glob(os.path.join(d, '*.json'))):
            m = re.match(r'(.*?)-(\d{4}-\d+(?:\.\d+)*)(?:-(\d+))?\.json$', os.path.basename(f))
            if not m:
                continue
            text = open(f).read().strip()
            if not text:
                continue
            r = json.loads(text.split('\n')[-1])
            tag = m.group(1) + ('' if m.group(3) is None else '#' + m.group(3))
            print(d, tag, r['id'], gap.get(r['id'], -1), r['nodes'], round(r['cpu_s'], 3), int(r['completed']), r['best'])


main()
