#!/usr/bin/env python3
"""The twin units' results side by side: for each pair (A id, B id) of PAIRS and each setting, the steps of both
(a trailing + when the CPU limit stopped it).  Usage: twin_table.py PAIRS RESULTS_DIR"""
import json
import os
import sys

pairs = [line.split() for line in open(sys.argv[1])]
rdir = sys.argv[2]


def res(uid, s):
    f = f"{rdir}/{uid}-{s}.json"
    if not os.path.exists(f):
        return None
    d = json.load(open(f))
    return d["nodes"], d["completed"]


def fmt(r):
    return "-" if r is None else f"{r[0]}{'' if r[1] else '+'}"


rows = []
for a, b, depth, ga, gb in pairs:
    ra = {s: res(a, s) for s in ("on", "p4096_55")}
    rb = {s: res(b, s) for s in ("on", "p4096_55")}
    if ra["on"] is None and rb["on"] is None:
        continue
    rows.append((int(depth), int(gb), a, b, ra, rb))
rows.sort()
same = total = 0
for depth, gap, a, b, ra, rb in rows:
    print(depth, gap, a, b, "on", fmt(ra["on"]), fmt(rb["on"]), "probe", fmt(ra["p4096_55"]), fmt(rb["p4096_55"]))
    if ra["on"] and rb["on"] and ra["on"][1] and rb["on"][1]:
        total += 1
        same += ra["on"][0] == rb["on"][0]
print("both completed on:", total, "with equal steps:", same)
