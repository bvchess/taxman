#!/usr/bin/env python3
"""Units of a snapshot stratified by depth band and by the members of the incumbent they have left out (neither
promoted nor candidates), gap in [LO, HI); prints unit.sh jobs (SNAP ID SETTING) for each setting.
Usage: pick_excl.py SNAPNAME LO HI PER_CELL SETTING..."""
import json
import random
import sys

snapname, lo, hi, per = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
settings = sys.argv[5:]


def bits(h):
    return frozenset(4 * k + b for k, c in enumerate(h) for b in range(4) if int(c, 16) >> b & 1)


units, game = [], None
for line in open(f"/home/brian/taxman-lab/snap/{snapname}/units.jsonl"):
    r = json.loads(line)
    if r["kind"] == "game":
        game = r
    else:
        units.append(r)
opt = bits(game["incumbent"])
cells = {}
for u in units:
    if not lo <= u["gap"] < hi or u["depth"] < 40:
        continue
    x = len(opt - bits(u["promoted"]) - bits(u["cands"]))
    d = min(u["depth"] // 20 * 20, 80)
    cells.setdefault((d, min(max(x, 3), 6)), []).append(u["id"])
rng = random.Random(11)
for key in sorted(cells):
    for uid in rng.sample(cells[key], min(per, len(cells[key]))):
        for s in settings:
            print(snapname, uid, s)
