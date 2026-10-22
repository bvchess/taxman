#!/usr/bin/env python3
"""Twin units of two snapshots: units of game B whose promotions and candidates are those of a unit of game A.
Prints counts by depth and gap band, and writes the twin pairs (A id, B id, depth, gap A, gap B) to OUT.
Usage: pairs.py SNAP_A SNAP_B OUT"""
import collections
import json
import sys


def load(d):
    units = []
    for line in open(d + "/units.jsonl"):
        r = json.loads(line)
        if r["kind"] == "unit":
            units.append(r)
    return units


ua, ub = load(sys.argv[1]), load(sys.argv[2])
by_pc = {(u["promoted"], u["cands"]): u for u in ua}
by_p = collections.defaultdict(list)
for u in ua:
    by_p[u["promoted"]].append(u)
pairs = []
same_p = 0
for u in ub:
    if u["promoted"] in by_p:
        same_p += 1
    v = by_pc.get((u["promoted"], u["cands"]))
    if v:
        pairs.append((v, u))
print("units", len(ua), len(ub), "same promotions", same_p, "same promotions and candidates", len(pairs))
bands = collections.Counter()
for v, u in pairs:
    d = "deep" if u["depth"] >= 55 else "shallow"
    g = min(u["gap"] // 300 * 300, 1200)
    bands[(d, g)] += 1
for k in sorted(bands):
    print(k, bands[k])
print("gap equal in", sum(1 for v, u in pairs if v["gap"] == u["gap"]), "of", len(pairs))
with open(sys.argv[3], "w") as out:
    for v, u in pairs:
        out.write(f"{v['id']} {u['id']} {u['depth']} {v['gap']} {u['gap']}\n")
