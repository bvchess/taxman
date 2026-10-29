#!/usr/bin/env python3
"""The large units, gate 600 against 1,200 (settings G600 and G1200): each unit's steps, and the totals over the
units that at least one gate finished, a unit stopped at the limit counted at the limit (so a gate that stopped counts
less than its tree; the ratio leans toward the gate that stopped).  usage: large.py"""
import json
import os

D = "/home/brian/taxman_work/lab/work/dyngate-2026-10-28"
SETS = {
    "5052, to 1 million": ("5052", f"{D}/ids-5052-big.txt"),
    "5054, to 1 million": ("5054", f"{D}/ids-5054-big.txt"),
    "5054, high gap, to 500,000": ("5054", f"{D}/ids-5054-high.txt"),
    "4984, to 1 million": ("4984", f"{D}/ids-4984-big.txt"),
}


def main():
    for name, (game, idfile) in SETS.items():
        gaps = {}
        for line in open(f"{D}/{game}/units-{game}.jsonl"):
            r = json.loads(line)
            if r["kind"] == "unit":
                gaps[r["id"]] = r["gap"]
        rows, both, gains, losses = [], 0, 0, 0
        for i in open(idfile).read().split():
            r = {}
            for s in ("G1200", "G600"):
                p = f"{D}/{game}/out/{s}-{i}.json"
                if os.path.exists(p):
                    j = json.load(open(p))
                    r[s] = (j["nodes"], j["completed"])
            if len(r) < 2:
                continue
            if not r["G1200"][1] and not r["G600"][1]:
                both += 1
                continue
            rows.append((i, gaps[i], r["G1200"], r["G600"]))
            gains += r["G600"][0] < r["G1200"][0]
            losses += r["G600"][0] > r["G1200"][0]
        a = sum(x[2][0] for x in rows)
        b = sum(x[3][0] for x in rows)
        print(f"== {name}: {len(rows)} units finished by a gate ({gains} smaller at 600, {losses} larger), {both} by "
              f"neither; steps {a} at 1,200, {b} at 600: {b / a if a else float('nan'):.2f}")
        for i, gap, x, y in rows:
            fmt = lambda v: f"{v[0] / 1000:.0f}k{'' if v[1] else '+'}"
            print(f"   {i:11} gap {gap:5}: {fmt(x):>6} -> {fmt(y):>6}")


if __name__ == "__main__":
    main()
