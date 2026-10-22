#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The units of `tune solve` runs side by side: table.py LABEL=FILE ..."""
import json
import sys

runs = {}
order = []
for arg in sys.argv[1:]:
    label, path = arg.split("=", 1)
    order.append(label)
    for line in open(path):
        if line.strip():
            d = json.loads(line)
            runs.setdefault(d["id"], {})[label] = d
print("| unit | " + " | ".join(f"{label} steps | {label} CPU s" for label in order) + " | best, proved |")
for uid, by in runs.items():
    cells = []
    for label in order:
        d = by.get(label)
        cells += [f"{d['nodes']:,}" if d else "-", f"{d['cpu_s']:.1f}" if d else "-"]
    bests = {(d["best"], d["proved"], d["completed"]) for d in by.values()}
    print(f"| {uid} | " + " | ".join(cells) + f" | {bests} |")
