#!/usr/bin/env python3
"""Compare panels of units of the 4895 snapshot: runs/LABEL/ID.jsonl.  Usage: cmp.py BASE LABEL...

For each label, per unit: steps and CPU s ('>' a unit stopped by its limit); totals over the units every label
finished, as steps and CPU, against BASE; and the units split at depth 40 (shallow, deep)."""
import glob
import json
import os
import sys

R = "/home/brian/taxman-lab/work/research-cb/runs"
SNAP = os.environ.get("SNAP", "/home/brian/taxman-lab/snap/4895-20261022T123603Z") + "/units.jsonl"
units = {}
for line in open(SNAP):
    r = json.loads(line)
    if r.get("kind") == "unit":
        units[r["id"]] = r


def load(label):
    out = {}
    for f in glob.glob(f"{R}/{label}/*.jsonl"):
        if os.path.getsize(f) == 0:
            continue
        r = json.load(open(f))
        out[r["id"]] = r
    return out


labels = sys.argv[1:]
runs = {lab: load(lab) for lab in labels}
ids = sorted(set.intersection(*(set(r) for r in runs.values())), key=lambda i: units[i]["index"])
print(f"{'unit':>10} {'depth':>5} {'gap':>5} " + " ".join(f"{lab:>16}" for lab in labels))
for i in ids:
    u = units[i]
    cells = []
    for lab in labels:
        r = runs[lab][i]
        mark = "" if r["completed"] else ">"
        cells.append(f"{mark}{r['nodes']:>8} {r['cpu_s']:6.1f}")
    print(f"{i:>10} {u['depth']:5d} {u['gap']:5d} " + " ".join(f"{c:>16}" for c in cells))
done = [i for i in ids if all(runs[lab][i]["completed"] for lab in labels)]
print(f"{len(done)} of {len(ids)} units finished by every label")
base = labels[0]
for part, sel in (("all", done), ("depth<40", [i for i in done if units[i]["depth"] < 40]),
                  ("depth>=40", [i for i in done if units[i]["depth"] >= 40])):
    if not sel:
        continue
    b_steps = sum(runs[base][i]["nodes"] for i in sel)
    b_cpu = sum(runs[base][i]["cpu_s"] for i in sel)
    line = [f"{part} ({len(sel)} units): {base} {b_steps} steps {b_cpu:.1f} s"]
    for lab in labels[1:]:
        s = sum(runs[lab][i]["nodes"] for i in sel)
        c = sum(runs[lab][i]["cpu_s"] for i in sel)
        line.append(f"{lab} {s/b_steps:.3f} steps {c/b_cpu:.3f} cpu")
    print("; ".join(line))
for lab in labels:
    if any(runs[lab][i]["best"] != runs[lab][i]["incumbent"] and runs[lab][i]["completed"] for i in ids):
        print(f"NOTE {lab}: a unit found a better best")
