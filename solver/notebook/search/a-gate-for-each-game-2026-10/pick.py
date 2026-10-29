#!/usr/bin/env python3
"""usage: pick.py GAMEDIR [N] -- N units (default 60) of gap >= 300 at evenly spaced ranks of the gap, written to
GAMEDIR/sample.txt largest gap first (the method of gate-2026-10-28/pick.py)."""
import json
import os
import sys

gamedir = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 60
game = os.path.basename(gamedir.rstrip("/"))
units = []
for line in open(f"{gamedir}/units-{game}.jsonl"):
    x = json.loads(line)
    if x["kind"] == "unit" and x["gap"] >= 300:
        units.append((x["gap"], x["index"], x["id"]))
units.sort()
picked = [units[int((k + 0.5) * len(units) / n)] for k in range(n)]
with open(f"{gamedir}/sample.txt", "w") as f:
    for _, _, i in reversed(picked):
        f.write(i + "\n")
print(f"{game}: {len(units)} units of gap >= 300; picked {len(picked)}: gaps {picked[0][0]} to {picked[-1][0]}")
