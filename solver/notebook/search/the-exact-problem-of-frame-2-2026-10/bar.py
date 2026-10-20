#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The bar: the node's proof (the search exact in every frame) and the search exact in frame 2, in steps and CPU
seconds, at the bound study's nodes, by band of gap.  Reads the bound study's local.jsonl.gz.

python3 bar.py local.jsonl.gz
"""

import gzip
import json
import statistics
import sys


def main():
    rows = []
    with gzip.open(sys.argv[1], "rt") as f:
        for line in f:
            d = json.loads(line)
            if d.get("kind") != "node":
                continue
            gap = d["bound"] - d["need"] + 1
            h = {e[0]: e for e in d["hybrid"]}
            proof = h.get("all")
            two = h.get("2")
            rows.append((d["n"], d["id"], d["node"], gap, d["subtree"], proof, two))
    bands = [(0, 399), (400, 1000), (1001, 10 ** 6)]
    print("| gap | nodes | proof steps median (range) | proof CPU s median (range) | frame 2 exact CPU s median |")
    print("|---|---|---|---|---|")
    for lo, hi in bands:
        sel = [r for r in rows if lo <= r[3] <= hi and r[5] is not None and r[5][2]]
        if not sel:
            continue
        steps = [r[5][3] for r in sel]
        cpu = [r[5][5] for r in sel]
        two = [r[6][5] for r in sel if r[6] is not None]
        print(f"| {lo}-{hi} | {len(sel)} | {statistics.median(steps):.0f} ({min(steps)}-{max(steps)}) |"
              f" {statistics.median(cpu):.3f} ({min(cpu):.3f}-{max(cpu):.3f}) | {statistics.median(two):.3f} |")
    for r in rows:
        print(r[0], r[1], r[2], r[3], r[4], r[5][3] if r[5] else None, r[5][5] if r[5] else None,
              r[6][3] if r[6] else None, r[6][5] if r[6] else None)


if __name__ == "__main__":
    main()
