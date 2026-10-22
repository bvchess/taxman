#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""What a fresh search of 4875 from 538080 would cost: project.py UNITS RUNS... [--min-gap G] [--steps-done N]

UNITS is `tune units`'s file of a checkpoint (the gap of each unit), RUNS the `tune solve` lines of some of its units
with one setting.  The completed runs give a line log10 steps = a + b · gap/100 (least squares, on the units of gap at
least G, default 600) and the CPU a step; every unit of the file is then counted at its own steps if it was run to the
end, else at the line's prediction (at least the steps it took, if it was stopped).  The spread is the line's residual
standard deviation, carried to the sum by drawing each unit's residual (1,000 draws, seeded).
"""

import json
import math
import random
import statistics
import sys


def main():
    args = sys.argv[1:]
    min_gap, done = 600, 0
    if "--min-gap" in args:
        i = args.index("--min-gap")
        min_gap = int(args[i + 1])
        del args[i:i + 2]
    if "--steps-done" in args:
        i = args.index("--steps-done")
        done = int(args[i + 1])
        del args[i:i + 2]
    units = [json.loads(line) for line in open(args[0]) if line.strip()]
    units = [u for u in units if u.get("kind") == "unit"]
    gap = {u["id"]: u["gap"] for u in units}
    runs = {}
    for path in args[1:]:
        for line in open(path):
            if line.strip():
                d = json.loads(line)
                if d["id"] in gap:
                    runs[d["id"]] = d
    fit = [(gap[i], math.log10(d["nodes"])) for i, d in runs.items() if d["completed"] and gap[i] >= min_gap]
    xs, ys = [g / 100 for g, _ in fit], [y for _, y in fit]
    mx, my = statistics.mean(xs), statistics.mean(ys)
    b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
    a = my - b * mx
    res = [y - (a + b * x) for x, y in zip(xs, ys)]
    sd = statistics.stdev(res) if len(res) > 2 else 0.0
    steps = sum(d["nodes"] for d in runs.values() if d["completed"])
    cpu = sum(d["cpu_s"] for d in runs.values() if d["completed"])
    per_step = cpu / steps
    print(f"fit on {len(fit)} completed units of gap >= {min_gap}: log10 steps = {a:.3f} + {b:.3f} * gap/100, "
          f"residual sd {sd:.2f}; {per_step * 1e3:.3f} ms a step over the completed runs")
    rng = random.Random(1)
    known, predicted = 0.0, []
    for u in units:
        if u["gap"] <= 0:
            continue
        d = runs.get(u["id"])
        if d and d["completed"]:
            known += d["nodes"]
        else:
            floor = d["nodes"] if d else 0
            predicted.append((u["gap"], floor))
    def total(shift):
        return known + sum(max(floor, 10 ** (a + b * g / 100 + shift())) for g, floor in predicted)
    central = total(lambda: 0.0)
    draws = sorted(total(lambda: rng.gauss(0, sd)) for _ in range(1000))
    big = sorted(predicted, reverse=True)[:5]
    print(f"units run to the end: {sum(1 for d in runs.values() if d['completed'])}, their steps {known:,.0f}; "
          f"predicted: {len(predicted)} units (the largest gaps {[g for g, _ in big]})")
    for label, s in [("central", central), ("p10", draws[100]), ("median", draws[500]), ("p90", draws[900])]:
        s += done
        print(f"  {label}: {s:.3e} steps, {s * per_step / 3600:.1f} CPU hours at {per_step * 1e3:.3f} ms a step")


if __name__ == "__main__":
    main()
