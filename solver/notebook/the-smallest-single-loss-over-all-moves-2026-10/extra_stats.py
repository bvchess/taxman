#!/usr/bin/env python3
"""How the true optimal game differs from the best one-loss swap, over the violations.

Reads violations.tsv beside this script (standard library only, run from anywhere): python3 extra_stats.py
"""

from __future__ import annotations

import ast
import csv
from collections import Counter
from pathlib import Path

with (Path(__file__).resolve().parent / "violations.tsv").open() as tsv:
    rows = list(csv.DictReader(tsv, delimiter="\t"))
cnt = Counter()
for r in rows:
    n, x = int(r["n"]), int(r["x_min"])
    dropped, added = ast.literal_eval(r["dropped"]), ast.literal_eval(r["added"])
    added = [a for a in added if a != n]
    cnt["x_min dropped" if x in dropped else "x_min kept"] += 1
    cnt["dropped - added = %d" % min(len(dropped) - len(added), 3)] += 1
    cnt["largest dropped >= x_min" if max(dropped) >= x else "all dropped < x_min"] += 1
    cnt["sum(dropped) - sum(added) = delta"] += (sum(dropped) - sum(added) == int(r["delta"]))
    cnt["every added < x_min"] += all(a < x for a in added)
    cnt["1-for-1 exchange (one dropped, one added besides n)"] += (len(dropped) == 1 and len(added) == 1)
    cnt["pure drop (nothing added besides n)"] += (not added)
    gap = int(r["gap"])
    cnt["gap<=10"] += gap <= 10
    cnt["gap>100"] += gap > 100
print(len(rows), dict(cnt))
ds = sorted(len(ast.literal_eval(r["dropped"])) for r in rows)
print("moves dropped: median", ds[len(ds) // 2], "max", ds[-1])
print("alternatives:", Counter(r["alternative"][0] for r in rows))
print("D by n range: n<=990:", [int(r["n"]) for r in rows if r["alternative"][0] == "D" and int(r["n"]) <= 990])
print("F+A+B+C+E count n<=990:", Counter(r["alternative"][0] for r in rows if int(r["n"]) <= 990))
print("violations with 425<=n<=780 (non-unique era) and their classes:",
      Counter(r["alternative"][0] for r in rows if 426 <= int(r["n"]) <= 780))
