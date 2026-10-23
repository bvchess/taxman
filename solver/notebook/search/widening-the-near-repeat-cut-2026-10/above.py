#!/usr/bin/env python3
"""The units of a snapshot that hold a promotion set worth more than game M's P*, from the lines of `widen greedy M
N`: no reference whose bound is at most P*_M can cut them.  Each unit is weighed by the notebook's line, log10 steps =
0.511 + 0.356 * gap / 100.  Prints the share of the units and of the line, the same for the units whose greedy
completion reaches N's P*, and for the units where a greedy completion is a promotion set of N and not of M: no
argument that the node's completions are sets of M can cut those.

Usage: python3 -I above.py FILE [FILE ...]"""
import json
import sys


def weight(gap):
    return 10 ** (0.511 + 0.356 * gap / 100) if gap > 0 else 1.0


for path in sys.argv[1:]:
    units = [json.loads(line) for line in open(path) if line.startswith("{")]
    total = sum(weight(u["gap"]) for u in units)
    above = [u for u in units if u["above_m"]]
    at_n = [u for u in units if u["greedy"] >= u["pstar_n"]]
    sm, sn = units[0]["pstar_m"], units[0]["pstar_n"]
    print(f"{path}: P*_M {sm}, P*_N {sn} ({sn - sm:+d}); {len(units)} units")
    rows = [("above P*_M", above), ("at P*_N", at_n)]
    if "not_in_m" in units[0]:
        rows.append(("no set of M", [u for u in units if u["not_in_m"]]))
    for name, chosen in rows:
        w = sum(weight(u["gap"]) for u in chosen)
        print(f"  greedy completion {name}: {len(chosen)} units ({len(chosen) / len(units):.1%}), "
              f"{w / total:.1%} of the line")
