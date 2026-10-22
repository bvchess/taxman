#!/usr/bin/env python3
"""How far each finished unit lies from the notebook's line, against its depth and its candidates.

Usage: residuals.py UNITS RUNS [UNITS RUNS]...

Each pair is a units file and the jsonl lines of `tune solve` made from it.  A run counts if it finished with conflict
branching on (the default tactics) and found nothing better than the incumbent it started from.  Its gap is the unit's
bound less what it needed from that incumbent (`bound − (incumbent + 1 − sum)`), and its residual is log10(steps) less
the notebook's line, 0.511 + 0.356 · gap/100.  Prints the units, then the correlation of the residual with the depth,
the number of candidates and the gap.
"""
import json
import math
import sys


def corr(xs, ys):
    n = len(xs)
    mx, my = sum(xs) / n, sum(ys) / n
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    sxx = sum((x - mx) ** 2 for x in xs)
    syy = sum((y - my) ** 2 for y in ys)
    return sxy / math.sqrt(sxx * syy) if sxx and syy else float("nan")


def main():
    args = sys.argv[1:]
    rows = []
    for units_path, runs_path in zip(args[::2], args[1::2]):
        units = {}
        for line in open(units_path):
            d = json.loads(line)
            if d.get("kind") == "unit":
                units[d["id"]] = d
        seen = set()
        for line in open(runs_path):
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            u = units.get(r["id"])
            key = (r["id"], r["incumbent"])
            if u is None or key in seen or r["tactics"] != "default" or not r["completed"]:
                continue
            if r["best"] != r["incumbent"]:
                continue
            seen.add(key)
            gap = u["bound"] - (r["incumbent"] + 1 - u["sum"])
            if gap <= 0:
                continue
            resid = math.log10(max(r["nodes"], 1)) - (0.511 + 0.356 * gap / 100)
            rows.append((r["id"], u["depth"], u.get("n_cands", u.get("cands", 0)), gap, r["nodes"], resid))
    rows.sort(key=lambda row: -row[5])
    for row in rows:
        print("%-12s depth %3d cands %4d gap %5d steps %9d residual %+.2f" % row)
    print("units", len(rows))
    for name, i in (("depth", 1), ("cands", 2), ("gap", 3)):
        print("correlation of the residual with", name, "%.2f" % corr([r[i] for r in rows], [r[5] for r in rows]))


if __name__ == "__main__":
    main()
