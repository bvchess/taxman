#!/usr/bin/env python3
"""collect.py RUNS OUT: every run of RUNS/LABEL/*.jsonl as a line of OUT/runs.jsonl (its label, unit or game, steps,
CPU seconds, whether it completed, its best, the optimum, tactics, gate and node limit), and the study's lines
(PRICESTUDY, STEPTIMERS) of RUNS/LABEL/*.err in OUT/study.txt, each headed by its label and unit."""
import glob
import json
import os
import sys

runs, out_dir = sys.argv[1:3]
FIELDS = ["game", "id", "nodes", "cpu_s", "completed", "best", "opt", "tactics", "gate", "node_limit"]
with open(os.path.join(out_dir, "runs.jsonl"), "w") as out, open(os.path.join(out_dir, "study.txt"), "w") as study:
    for label in sorted(os.listdir(runs)):
        for path in sorted(glob.glob(os.path.join(runs, label, "*.jsonl"))):
            for line in open(path):
                if line.strip():
                    d = json.loads(line)
                    out.write(json.dumps({"label": label, **{k: d.get(k) for k in FIELDS}}) + "\n")
        for path in sorted(glob.glob(os.path.join(runs, label, "*.err"))):
            unit = os.path.basename(path)[: -len(".err")]
            for line in open(path):
                if line.startswith(("PRICESTUDY ", "STEPTIMERS ")):
                    study.write(f"{label} {unit} {line}")
