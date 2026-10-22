#!/usr/bin/env python3
"""Pick twin pairs across gap and depth bands, seeded, and print jobs for unit.sh: SNAP ID SETTING.
Usage: pick_twins.py PAIRS SNAP_A SNAP_B"""
import random
import sys

rng = random.Random(7)
pairs = [line.split() for line in open(sys.argv[1])]
want = {("deep", 600): 4, ("deep", 900): 4, ("deep", 1200): 3,
        ("shallow", 300): 3, ("shallow", 600): 4, ("shallow", 900): 4, ("shallow", 1200): 2}
chosen = []
for (kind, lo), k in sorted(want.items()):
    hi = 9999 if lo == 1200 else lo + 300
    sel = [p for p in pairs if (int(p[2]) >= 55) == (kind == "deep") and lo <= int(p[4]) < hi]
    chosen += rng.sample(sel, min(k, len(sel)))
for a, b, depth, ga, gb in chosen:
    settings = ["on", "p4096_55"] if int(depth) >= 55 else ["on"]
    for s in settings:
        print(sys.argv[2], a, s)
        print(sys.argv[3], b, s)
