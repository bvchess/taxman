#!/usr/bin/env python3
"""Searched games of the chain by the shift of P* from game n - 1, and the share of their steps.
Usage: shifts.py STRICT.jsonl FROM TO"""
import collections
import json
import sys

rows = {}
for line in open(sys.argv[1]):
    d = json.loads(line)
    rows.setdefault(d["n"], []).append(d)
lo, hi = int(sys.argv[2]), int(sys.argv[3])


def best(d):
    ib = d.get("in_the_bag")
    return d["score"] - ib if ib else None


steps = collections.Counter()
count = collections.Counter()
listing = []
for n in range(lo, hi + 1):
    if n not in rows or n - 1 not in rows:
        continue
    d = rows[n][-1]
    if d["method"] != "search" or d["proof"] != "exhausted":
        continue
    p = rows[n - 1][-1]
    b, pb = best(d), best(p)
    nodes = sum(r["nodes"] for r in rows[n])
    if pb is None:
        band = "prev prime"
    else:
        s = abs(b - pb)
        band = "0" if s == 0 else "1-20" if s <= 20 else "21-100" if s <= 100 else "101-500" if s <= 500 else ">500"
    steps[band] += nodes
    count[band] += 1
    listing.append((n, band, nodes, b, pb, (d.get("setup") or {}).get("candidates"),
                    (p.get("setup") or {}).get("candidates")))
tot = sum(steps.values())
for band in ["0", "1-20", "21-100", "101-500", ">500", "prev prime"]:
    print(f"{band:>10} games {count[band]:4d} steps {steps[band]:.3e} share {steps[band] / tot:.3f}")
if len(sys.argv) > 4:
    for row in listing:
        print(*row)
