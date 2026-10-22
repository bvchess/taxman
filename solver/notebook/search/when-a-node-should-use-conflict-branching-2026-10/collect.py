#!/usr/bin/env python3
"""Collect every run of runs/LABEL/ID.jsonl into one jsonl: the label, then the run's id, steps, CPU s, whether it
completed, its best and the known optimum."""
import glob
import json
import os
import sys

R = sys.argv[1] if len(sys.argv) > 1 else "/home/brian/taxman-lab/work/research-cb/runs"
for f in sorted(glob.glob(f"{R}/*/*.jsonl")):
    if os.path.getsize(f) == 0:
        continue
    r = json.load(open(f))
    out = {"label": os.path.basename(os.path.dirname(f))}
    for k in ("id", "nodes", "cpu_s", "completed", "best", "opt", "tactics"):
        out[k] = r.get(k)
    out["cpu_s"] = round(out["cpu_s"], 1)
    print(json.dumps(out))
