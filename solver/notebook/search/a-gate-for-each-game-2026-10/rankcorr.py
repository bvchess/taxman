#!/usr/bin/env python3
"""Per game, the rank correlation (Spearman) of a unit's log ratio of steps, gate 600 against 1,200, with its gap,
depth, number of candidates and size (its steps at 1,200), over the units of 2,000 steps or more that some gate
finished; and the mean log ratio.  usage: rankcorr.py"""
import json
import math

FOLDERS = {
    "5052": "/home/brian/taxman_work/lab/work/gate-2026-10-28/5052",
    "4995": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4995",
    "4992": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4992",
    "4984": "/home/brian/taxman_work/lab/work/gate-2026-10-28/4984",
    "5054": "/home/brian/taxman_work/lab/work/dyngate-2026-10-28/5054",
}


def rank(values):
    order = sorted(range(len(values)), key=lambda i: values[i])
    ranks = [0] * len(values)
    for k, i in enumerate(order):
        ranks[i] = k
    return ranks


def spearman(a, b):
    ra, rb = rank(a), rank(b)
    n = len(a)
    ma, mb = sum(ra) / n, sum(rb) / n
    cov = sum((x - ma) * (y - mb) for x, y in zip(ra, rb))
    return cov / math.sqrt(sum((x - ma) ** 2 for x in ra) * sum((y - mb) ** 2 for y in rb))


def main():
    for game, folder in FOLDERS.items():
        units = {}
        for line in open(f"{folder}/units-{game}.jsonl"):
            r = json.loads(line)
            if r["kind"] == "unit":
                units[r["id"]] = r
        rows = []
        for i in open(f"{folder}/sample.txt").read().split():
            a = json.load(open(f"{folder}/out/g1200-{i}.json"))
            b = json.load(open(f"{folder}/out/g600-{i}.json"))
            if a["nodes"] < 2000 or (not a["completed"] and not b["completed"]):
                continue
            u = units[i]
            rows.append((math.log(b["nodes"] / a["nodes"]), u["gap"], u["depth"], u["n_cands"], math.log(a["nodes"])))
        lr = [r[0] for r in rows]
        cells = " ".join(f"{name} {spearman(lr, [r[k] for r in rows]):+.2f}"
                         for k, name in ((1, "gap"), (2, "depth"), (3, "cands"), (4, "size")))
        print(f"{game}: {len(rows)} units; rank correlation of log(steps 600/1200) with {cells}; "
              f"mean log ratio {sum(lr) / len(lr):+.2f}")


if __name__ == "__main__":
    main()
