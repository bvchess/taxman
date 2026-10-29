#!/usr/bin/env python3
"""usage: panel.py GAMEDIR BASE SET... -- each setting against BASE on the units of GAMEDIR/sample.txt that every
setting finished: steps and CPU in all, by the units' gap band, the units capped, and the units that differ most."""
import json
import os
import sys

BANDS = [(0, 600), (600, 900), (900, 1200), (1200, 99999)]


def main():
    gamedir, base, *sets = sys.argv[1:]
    sets = [base] + sets
    game = os.path.basename(gamedir.rstrip("/"))
    gaps = {}
    for line in open(f"{gamedir}/units-{game}.jsonl"):
        r = json.loads(line)
        if r["kind"] == "unit":
            gaps[r["id"]] = r["gap"]
    ids = open(f"{gamedir}/sample.txt").read().split()
    res = {}
    for i in ids:
        row = {}
        for s in sets:
            p = f"{gamedir}/out/{s}-{i}.json"
            if not os.path.exists(p):
                break
            j = json.load(open(p))
            row[s] = (j["nodes"], j["cpu_s"], j["completed"])
        else:
            res[i] = row
    print(f"== {game}: {len(res)} of {len(ids)} units finished in all of {', '.join(sets)}")

    def table(sel, label):
        if not sel:
            return
        b_steps = sum(res[i][base][0] for i in sel)
        b_cpu = sum(res[i][base][1] for i in sel)
        cells = []
        for s in sets:
            st = sum(res[i][s][0] for i in sel)
            cpu = sum(res[i][s][1] for i in sel)
            cap = sum(not res[i][s][2] for i in sel)
            cells.append(f"{s} {st / max(b_steps, 1):.3f}/{cpu / max(b_cpu, 1e-9):.3f} c{cap}")
        print(f"  {label:16} n={len(sel):2} base {b_steps:8d} steps {b_cpu:7.0f} s | " + " | ".join(cells))

    print("  (each setting: steps/CPU against the base, c = units capped)")
    table(list(res), "all")
    for lo, hi in BANDS:
        table([i for i in res if lo <= gaps[i] < hi], f"gap {lo}-{hi - 1}")
    print("  per unit (gap: steps k / CPU s, * capped), largest base first:")
    for i in sorted(res, key=lambda i: -res[i][base][0])[:25]:
        cells = " | ".join(
            f"{s}:{res[i][s][0] / 1000:.0f}k/{res[i][s][1]:.0f}{'' if res[i][s][2] else '*'}" for s in sets)
        print(f"    {i:12} {gaps[i]:5} {cells}")


if __name__ == "__main__":
    main()
