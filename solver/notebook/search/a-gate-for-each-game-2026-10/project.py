#!/usr/bin/env python3
"""The projected total CPU of each gate policy over a panel of games, weighted by each game's cost (its steps in the
chain's profile; for 5054, still running, the steps of its twin-sized games), against the gate of 1,200.

A policy's ratio on a game is the CPU of its 60 sampled units (gate-2026-10-28 and dyngate-2026-10-28) under the gate
the policy gives, against g1200's; the gate trial's is the mean ratio of the gate a test of K units picks (random K of
the 60, the gate of least CPU over them, 2,000 draws), applied to the steps after the trial (AFTER) and charged its
cost (the test's CPU, as a share of the game's).

usage: project.py"""
import json
import os
import random

GATE = "/home/brian/taxman_work/lab/work/gate-2026-10-28"
DYN = "/home/brian/taxman_work/lab/work/dyngate-2026-10-28"
# Game: (units folder, steps of the whole game).  5054 runs still: its cost is taken as 5052's, its size since the
# first checkpoints has followed 5049's and 5052's.  The twins (4983 of 4984, 5049 of 5052) are counted as their twin.
GAMES = {
    "5052": (f"{GATE}/5052", 4.09e9),
    "4995": (f"{GATE}/4995", 0.98e9),
    "4992": (f"{GATE}/4992", 1.95e9),
    "4984": (f"{GATE}/4984", 3.87e9),
    "5054": (f"{DYN}/5054", 4.09e9),
}
TWINS = {"4983": ("4984", 8.77e9), "5049": ("5052", 4.05e9)}
AFTER = 50e6
STEP_S = 0.45e-3  # CPU seconds a step near n = 5000 (the units' own mean)
K = 24
TRIED = ("g1200", "g1000", "g800", "g600")


def load(folder, sets):
    ids = open(f"{folder}/sample.txt").read().split()
    rows = []
    for i in ids:
        r = {}
        for s in sets:
            p = f"{folder}/out/{s}-{i}.json"
            if os.path.exists(p):
                j = json.load(open(p))
                r[s] = j["cpu_s"]
        if len(r) == len(sets):
            rows.append(r)
    return rows


def main():
    rng = random.Random(1)
    ratios, trial = {}, {}
    for game, (folder, _) in GAMES.items():
        first = open(f"{folder}/sample.txt").read().split()[0]
        sets = [s for s in ("g1200", "g1000", "g800", "g600") if os.path.exists(f"{folder}/out/{s}-{first}.json")]
        rows = load(folder, sets)
        base = sum(r["g1200"] for r in rows)
        ratios[game] = {s: sum(r[s] for r in rows) / base for s in sets}
        tried = [s for s in TRIED if s in sets]
        mean, cost = 0.0, 0.0
        for _ in range(2000):
            sub = rng.sample(rows, K)
            pick = min(tried, key=lambda s: sum(r[s] for r in sub))
            mean += ratios[game][pick]
            cost += sum(r[s] for r in sub for s in tried)
        trial[game] = (mean / 2000, cost / 2000)
    panel = dict(GAMES)
    for twin, (of, steps) in TWINS.items():
        panel[twin] = (None, steps)
        ratios[twin] = ratios[of]
        trial[twin] = trial[of]
    total = sum(steps for _, steps in panel.values())
    gates = " ".join(f"{s:>6}" for s in ("g1000", "g800", "g600"))
    print(f"game  steps(G) weight {gates}  trial(K=24)  trial cost")
    for game, (_, steps) in panel.items():
        cells = " ".join(f"{ratios[game].get(s, float('nan')):6.3f}" for s in ("g1000", "g800", "g600"))
        r, c = trial[game]
        cpu = steps * STEP_S
        eff = (AFTER + (steps - AFTER) * r + c / STEP_S) / steps
        print(f"{game:5} {steps / 1e9:8.2f} {steps / total:6.3f} {cells}  {r:6.3f} -> {eff:6.3f}  "
              f"{c / 3600:5.2f} h of {cpu / 3600:6.0f} h")
    for name, games in (("panel of 5", list(GAMES)), ("with the twins", list(panel))):
        tot = sum(panel[g][1] for g in games)
        line = []
        for s in ("g1000", "g800", "g600"):
            line.append(f"{s} {sum(panel[g][1] * ratios[g].get(s, 1.0) for g in games) / tot:.3f}")
        eff = sum(AFTER + (panel[g][1] - AFTER) * trial[g][0] for g in games)
        cost = sum(trial[g][1] / STEP_S for g in games)
        line.append(f"trial {(eff + cost) / tot:.3f}")
        print(f"{name:15} (cost-weighted, against g1200): " + ", ".join(line))


if __name__ == "__main__":
    main()
