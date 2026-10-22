#!/usr/bin/env python3
"""Summarize the looks of the rule `look` (TAXMAN_CB_TRACE_LOOKS lines): how often the child looked at is cut, by the
conflict's size, the cheapest member's price, the depth and the ratio of the conflict's weight less c's to the best's."""
import re
import sys
from collections import Counter

for path in sys.argv[1:]:
    rows = []
    for line in open(path):
        if line.startswith("LOOK "):
            d = dict(kv.split("=") for kv in line.split()[1:])
            rows.append(d)
    print(f"== {path}: {len(rows)} looks, {sum(r['cut'] == 'true' for r in rows)} cut")

    def table(name, key):
        n, c = Counter(), Counter()
        for r in rows:
            k = key(r)
            n[k] += 1
            c[k] += r["cut"] == "true"
        print(f"  by {name}: " + ", ".join(f"{k}: {c[k]}/{n[k]}" for k in sorted(n)))

    table("size", lambda r: min(int(r["k"]), 6))
    table("cheapest price", lambda r: next(b for b in (0, 10, 30, 100, 300, 10**9) if float(r["pc"]) <= b))
    table("depth", lambda r: int(r["d"]) // 20 * 20)
    table("gap", lambda r: int(r["gap"]) // 200 * 200)
    table("eff/best", lambda r: round(float(r["eff"]) / float(r["best"]), 1))
