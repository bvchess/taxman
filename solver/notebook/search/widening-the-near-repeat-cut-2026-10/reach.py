#!/usr/bin/env python3
"""The reach of the near-repeat cut at a snapshot's units, from the lines of `widen units M N` (one a unit).

Each unit is weighed by the notebook's line, log10 steps = 0.511 + 0.356 * gap / 100 (a unit of gap 0 or less
counts one step).  Prints, for each mode, the share of the units and of the line where the check holds structurally,
the reasons it fails weighed by the line, and the numbers that break it most often.

Usage: python3 -I reach.py FILE [FILE ...]"""
import collections
import json
import sys

MODES = ["0", "1", "15", "16", "31"]


def weight(gap):
    return 10 ** (0.511 + 0.356 * gap / 100) if gap > 0 else 1.0


for path in sys.argv[1:]:
    units = [json.loads(line) for line in open(path) if line.startswith("{")]
    if not units:
        print(path, "no units")
        continue
    total = sum(weight(u["gap"]) for u in units)
    print(f"{path}: {len(units)} units, line {total:.3g} steps, bound ok: {units[0]['bound_ok']}")
    for mode in [m for m in MODES if "m" + m in units[0]]:
        holds = [u for u in units if u["m" + mode] == "holds"]
        w = sum(weight(u["gap"]) for u in holds)
        reasons = collections.Counter()
        for u in units:
            reasons[u["m" + mode]] += weight(u["gap"])
        why = collections.Counter()
        for u in units:
            if u["m" + mode] != "holds":
                for x in u["why" + mode].split()[:3]:
                    why[x] += weight(u["gap"])
        top = ", ".join(f"{x} {c / total:.0%}" for x, c in why.most_common(8))
        shares = ", ".join(f"{r} {c / total:.1%}" for r, c in reasons.most_common())
        print(f"  mode {mode:>2}: holds at {len(holds)} units ({len(holds) / len(units):.1%}), "
              f"{w / total:.1%} of the line; {shares}")
        print(f"           breaking numbers (share of the line): {top}")
