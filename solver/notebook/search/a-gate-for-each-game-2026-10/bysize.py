#!/usr/bin/env python3
"""The steps of the gate of 600 (and 800) against 1,200 by the size of a unit's subtree (its steps at 1,200), for the
units every setting completed, per game: does the lower gate lose more in larger subtrees?"""
import json
import os

FOLDERS = {
    "5052": "/home/brian/taxman_work/lab/work/gate-2026-10-28/5052",
    "4995": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4995",
    "4992": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4992",
    "4984": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4984",
    "5054": "/home/brian/taxman_work/lab/work/dyngate-2026-10-28/5054",
}
BINS = [(1000, 5000), (5000, 20000), (20000, 60000), (60000, 151000)]


def main():
    for game, folder in FOLDERS.items():
        ids = open(f"{folder}/sample.txt").read().split()
        rows = []
        for i in ids:
            r = {}
            for s in ("g1200", "g800", "g600"):
                p = f"{folder}/out/{s}-{i}.json"
                if os.path.exists(p):
                    j = json.load(open(p))
                    r[s] = (j["nodes"], j["completed"])
            if len(r) == 3:
                rows.append(r)
        cells = []
        for lo, hi in BINS:
            sel = [r for r in rows if lo <= r["g1200"][0] < hi and r["g1200"][1] and r["g600"][1] and r["g800"][1]]
            if not sel:
                cells.append(f"{lo // 1000}k-{hi // 1000}k: -")
                continue
            b = sum(r["g1200"][0] for r in sel)
            cells.append(f"{lo // 1000}k-{hi // 1000}k n={len(sel):2} g800 {sum(r['g800'][0] for r in sel) / b:.2f}"
                         f" g600 {sum(r['g600'][0] for r in sel) / b:.2f}")
        capped = [r for r in rows if not r["g1200"][1]]
        cap_done = sum(1 for r in capped if r["g600"][1])
        print(f"{game}: " + " | ".join(cells) + f" | capped at 1200: {len(capped)}, of them g600 completes {cap_done}")


if __name__ == "__main__":
    main()
