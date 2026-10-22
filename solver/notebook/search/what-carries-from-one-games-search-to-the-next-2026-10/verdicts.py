#!/usr/bin/env python3
"""Probe verdicts of deep units and how they agree between units close in their promotions.
A unit's verdict: off ended it within the probe's budget (the probe run took at most BUDGET + 1,024 steps and completed).
Usage: verdicts.py SNAPDIR RESULTS_DIR SETTING BUDGET"""
import glob
import itertools
import json
import os
import sys


def bits(h):
    return frozenset(4 * k + b for k, c in enumerate(h) for b in range(4) if int(c, 16) >> b & 1)


snap, rdir, setting, budget = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
units = {}
for line in open(snap + "/units.jsonl"):
    r = json.loads(line)
    if r["kind"] == "unit":
        units[r["id"]] = r
rows = []
for f in glob.glob(f"{rdir}/*-{setting}.json"):
    uid = os.path.basename(f)[: -len(f"-{setting}.json")]
    try:
        d = json.loads(open(f).read())
    except ValueError:
        continue
    u = units[uid]
    won = d["completed"] and d["nodes"] <= budget + 1024
    rows.append((uid, u["depth"], u["gap"], won, bits(u["promoted"]), bits(u["cands"])))
print(len(rows), "units,", sum(r[3] for r in rows), "ended by off within the budget")
pairs = []
for a, b in itertools.combinations(rows, 2):
    sd = len(a[4] ^ b[4])
    pairs.append((sd, a[3] == b[3], a[0], b[0]))
pairs.sort()
for lo, hi in [(0, 4), (4, 10), (10, 20), (20, 40), (40, 999)]:
    sel = [p for p in pairs if lo <= p[0] < hi]
    if sel:
        print(f"promotions differing by {lo}-{hi - 1}: {len(sel)} pairs, verdicts agree in {sum(p[1] for p in sel)}")
for p in pairs[:12]:
    print(*p)
