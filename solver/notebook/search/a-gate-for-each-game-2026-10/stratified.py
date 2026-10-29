#!/usr/bin/env python3
"""The trial's own pick on each game's 60 units: 24 at evenly spaced ranks of the gap (as `gate_trial::pick` takes
them from a checkpoint's sample), at each of 10 offsets within a slice, and the gate of least CPU over them among
600, 800, 1,000 and 1,200 (with the margin of 3% against 1,200); its ratio over all 60.  usage: stratified.py"""
import json
import os

FOLDERS = {
    "5052": "/home/brian/taxman_work/lab/work/gate-2026-10-28/5052",
    "5054": "/home/brian/taxman_work/lab/work/dyngate-2026-10-28/5054",
    "4992": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4992",
    "4995": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4995",
    "4984": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4984",
}
GATES = ("g1200", "g1000", "g800", "g600")
K, MARGIN = 24, 0.03


def main():
    for game, folder in FOLDERS.items():
        gaps = {}
        for line in open(f"{folder}/units-{game}.jsonl"):
            r = json.loads(line)
            if r["kind"] == "unit":
                gaps[r["id"]] = r["gap"]
        ids = sorted(open(f"{folder}/sample.txt").read().split(), key=lambda i: gaps[i])
        sets = [s for s in GATES if os.path.exists(f"{folder}/out/{s}-{ids[0]}.json")]
        cpu = {i: {s: json.load(open(f"{folder}/out/{s}-{i}.json"))["cpu_s"] for s in sets} for i in ids}
        full = {s: sum(cpu[i][s] for i in ids) / sum(cpu[i]["g1200"] for i in ids) for s in sets}
        picks = []
        for u in range(10):
            chosen = [ids[int((k + (u + 0.5) / 10) * len(ids) / K)] for k in range(K)]
            tot = {s: sum(cpu[i][s] for i in chosen) for s in sets}
            best = min(sets, key=lambda s: tot[s])
            pick = best if tot[best] <= (1 - MARGIN) * tot["g1200"] else "g1200"
            picks.append(pick)
        mean = sum(full[p] for p in picks) / len(picks)
        print(f"{game}: picks {' '.join(p[1:] for p in picks)}; mean ratio {mean:.3f} (best {min(full.values()):.3f})")


if __name__ == "__main__":
    main()
