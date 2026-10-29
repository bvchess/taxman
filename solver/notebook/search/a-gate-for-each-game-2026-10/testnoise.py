#!/usr/bin/env python3
"""How well a small in-game test picks a game's gate: from the gate re-check's 60 units a game (gates 400 to 1,200),
draw K units at random, pick the gate whose CPU (or steps) over them is least, and score the pick by the CPU ratio of
that gate against g1200 over all 60 units.  Also the test's cost in CPU against the game's 60-unit g1200 CPU.

usage: testnoise.py [GAMEDIR ...]  (default: the four games of gate-2026-10-28)"""
import json
import os
import random
import sys

D = "/home/brian/taxman_work/lab/work/gate-2026-10-28"
SETS = ["g1200", "g1000", "g800", "g600", "g400"]


def load(gamedir, sets):
    game = os.path.basename(gamedir.rstrip("/"))
    ids = open(f"{gamedir}/sample.txt").read().split()
    rows = []
    for i in ids:
        r = {}
        for s in sets:
            p = f"{gamedir}/out/{s}-{i}.json"
            if not os.path.exists(p):
                break
            j = json.load(open(p))
            r[s] = (j["nodes"], j["cpu_s"])
        else:
            rows.append(r)
    return game, rows


def main():
    dirs = sys.argv[1:] or [f"{D}/{g}" for g in ("5052", "4995", "4992", "4984")]
    rng = random.Random(1)
    for gd in dirs:
        first = open(f"{gd}/sample.txt").read().split()[0]
        sets = [s for s in SETS if os.path.exists(f"{gd}/out/{s}-{first}.json")]
        game, rows = load(gd, sets)
        tot = {s: sum(r[s][1] for r in rows) for s in sets}
        full = {s: tot[s] / tot["g1200"] for s in sets}
        best = min(full.values())
        print(f"== {game}: {len(rows)} units; CPU against g1200: " + " ".join(f"{s} {full[s]:.3f}" for s in sets))
        for k in (10, 20, 30, 60):
            for key, name in ((1, "cpu"), (0, "steps")):
                picks = {}
                score = 0.0
                trials = 2000
                for _ in range(trials):
                    sub = rng.sample(rows, k) if k < len(rows) else rows
                    s = min(sets, key=lambda s: sum(r[s][key] for r in sub))
                    picks[s] = picks.get(s, 0) + 1
                    score += full[s]
                cost = k / len(rows) * sum(tot.values()) / tot["g1200"]
                print(f"  K={k:2} by {name:5}: mean ratio of the pick {score / trials:.3f} (best {best:.3f}); picks "
                      + " ".join(f"{s}:{picks.get(s, 0) / trials:.2f}" for s in sets)
                      + f"; test cost {cost:.2f} x the 60 units' g1200 CPU")


if __name__ == "__main__":
    main()
