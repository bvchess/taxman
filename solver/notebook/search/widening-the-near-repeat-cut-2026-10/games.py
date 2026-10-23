#!/usr/bin/env python3
"""The whole games of game.sh: a line a run (game, setting, steps, CPU seconds, whether it ended, whether its best is
the known optimum, and for the study cut the nodes it cut and the checks made), and the runs as JSON lines to OUT.

Usage: python3 -I games.py DIR [OUT]"""
import glob
import json
import os
import re
import sys

rows = []
for path in sorted(glob.glob(os.path.join(sys.argv[1], "g*.json"))):
    name = os.path.basename(path)[1:-5]
    game, setting = name.split("-", 1)
    d = json.loads(open(path).read().splitlines()[-1])
    err = open(path + ".err").read() if os.path.exists(path + ".err") else ""
    m = re.search(r"widen: checks (\d+); holds (\d+)", err)
    rows.append({
        "game": int(game), "setting": setting, "steps": d["nodes"], "cpu_s": round(d["cpu_s"], 1),
        "ended": d["completed"], "optimum": d["best"] == d["opt"], "cut": int(m.group(2)) if m else None,
        "checks": int(m.group(1)) if m else None,
    })
rows.sort(key=lambda r: (r["game"], r["setting"]))
for r in rows:
    print(f"{r['game']} {r['setting']:>10} steps {r['steps']:>10,} cpu {r['cpu_s']:>8} ended {r['ended']!s:5} "
          f"optimum {r['optimum']!s:5} cut {r['cut']} checks {r['checks']}")
if len(sys.argv) > 2:
    with open(sys.argv[2], "w") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
