#!/usr/bin/env python3
"""cmp.py RUNS LABEL_A LABEL_B: the units both labels ran, side by side: steps, best, whether the trees (and, from the
runs' folders, the counters) are the same, CPU seconds and their ratio B/A, and the totals.  RUNS is runs.jsonl, or
the folder of the runs (RUNS/LABEL/*.jsonl, the result lines of tune)."""
import glob
import json
import os
import sys

runs, a, b = sys.argv[1:4]


def load(label):
    out = {}
    if os.path.isfile(runs):
        for line in open(runs):
            d = json.loads(line)
            if d["label"] == label:
                out[d["id"]] = d
        return out
    for f in glob.glob(os.path.join(runs, label, "*.jsonl")):
        for line in open(f):
            line = line.strip()
            if line:
                d = json.loads(line)
                out[d["id"]] = d
    return out


A, B = load(a), load(b)
ta = tb = 0.0
print(f"{'unit':12} {'steps A':>9} {'steps B':>9} {'same':>5} {'best':>8} {'cpu A':>8} {'cpu B':>8} {'B/A':>6} "
      f"{'us/step A':>9} {'us/step B':>9}")
for u in sorted(set(A) & set(B)):
    x, y = A[u], B[u]
    same = x["nodes"] == y["nodes"] and x["best"] == y["best"] and x.get("stats") == y.get("stats")
    ok = x["best"] == y["best"] and (x["opt"] is None or x["best"] == x["opt"])
    ta += x["cpu_s"]
    tb += y["cpu_s"]
    print(f"{u:12} {x['nodes']:9d} {y['nodes']:9d} {str(same):>5} {('ok' if ok else 'BAD'):>8} {x['cpu_s']:8.1f} "
          f"{y['cpu_s']:8.1f} {y['cpu_s'] / x['cpu_s']:6.3f} {1e6 * x['cpu_s'] / x['nodes']:9.0f} "
          f"{1e6 * y['cpu_s'] / y['nodes']:9.0f}")
print(f"{'total':12} {'':9} {'':9} {'':5} {'':8} {ta:8.1f} {tb:8.1f} {tb / max(ta, 1e-9):6.3f}")
