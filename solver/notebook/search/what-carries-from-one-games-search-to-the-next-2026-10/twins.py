#!/usr/bin/env python3
"""Units of consecutive snapshots: how close each unit of game B is to a unit of game A (promotion sets), and how
units sit against the games' optima.  Usage: twins.py SNAP_A SNAP_B"""
import collections
import json
import statistics
import sys


def bits(h):
    s = set()
    for k, c in enumerate(h):
        v = int(c, 16)
        for b in range(4):
            if v >> b & 1:
                s.add(4 * k + b)
    return s


def load(d):
    game, units = None, []
    for line in open(d + "/units.jsonl"):
        r = json.loads(line)
        if r["kind"] == "game":
            game = r
        else:
            r["P"] = frozenset(bits(r["promoted"]))
            r["C"] = frozenset(bits(r["cands"]))
            units.append(r)
    game["opt"] = frozenset(bits(game["incumbent"]))
    game["cset"] = frozenset(bits(game["candidates"]))
    return game, units


def share(u, opt):
    return len(u["P"] & opt) / max(1, len(u["P"]))


def main():
    ga, ua = load(sys.argv[1])
    gb, ub = load(sys.argv[2])
    print("games", ga["game"], gb["game"], "units", len(ua), len(ub))
    print("candidates: A-B", sorted(ga["cset"] - gb["cset"]), "B-A", sorted(gb["cset"] - ga["cset"]))
    print("optima: |A|", len(ga["opt"]), "|B|", len(gb["opt"]), "common", len(ga["opt"] & gb["opt"]))
    exact = {u["P"] for u in ua}
    print("B units whose promoted set is exactly an A unit's:", sum(1 for u in ub if u["P"] in exact), "of", len(ub))
    by_depth = collections.defaultdict(list)
    for u in ua:
        by_depth[u["depth"]].append(u)
    hist = collections.Counter()
    for u in ub:
        best = None
        for d in range(u["depth"] - 3, u["depth"] + 4):
            for v in by_depth.get(d, []):
                sd = len(u["P"] ^ v["P"])
                if best is None or sd < best:
                    best = sd
        hist[min(best if best is not None else 99, 20)] += 1
    print("symmetric difference to the nearest A unit (capped 20):", sorted(hist.items()))
    for lo, hi in [(0, 300), (300, 600), (600, 900), (900, 1200), (1200, 9999)]:
        sel = [u for u in ub if lo <= u["gap"] < hi]
        if sel:
            print("gap", lo, hi, "units", len(sel), "depth med", statistics.median(u["depth"] for u in sel),
                  "share in own opt med", round(statistics.median(share(u, gb["opt"]) for u in sel), 3),
                  "in prev opt", round(statistics.median(share(u, ga["opt"]) for u in sel), 3))


main()
