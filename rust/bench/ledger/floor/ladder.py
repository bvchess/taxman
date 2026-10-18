#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The ladder of floor steps (examples/ledger/cost.rs) laid on the 14 games measured under floors
(bench/target/target.jsonl): what the steps and the proof cost together against the proof alone, and how well the line
of the steps so far forecasts the proof.  Standard library only.

usage: ladder.py [STEP [PROOF_STEPS]]      STEP the points of a step at the median slope (default 150), PROOF_STEPS the
                                           steps the gap must exceed for another step to be taken (default 2)

For each game the steps start at the root's bound (U = the root's bound) and go down by the cost-equal step of the
board's line (STEP at the median slope, fewer on a steeper line) until what is left of the gap is within PROOF_STEPS
steps; each step costs what the measured floors give at that distance (log10 of the nodes interpolated between the
measured floors), and the proof costs the measured full proof.  The line is fitted as the ledger fits it: from the
steps of 100 nodes or more, its slope pulled to the median slope as a pair of points 141 apart on the median line would pull it.
"""
import json
import math
import statistics
import sys
from pathlib import Path

PRIOR_SLOPE, PRIOR_INTERCEPT = 0.0068, 0.5  # cost.rs: PRIOR
SLOPES = (0.003, 0.012)
PULL = 100.0 ** 2
MIN_NODES = 100


def interpolated(runs, distance, root):
    """log10 of the nodes of a search under the floor `distance` below the root's bound."""
    points = sorted((root - r["floor"], math.log10(max(r["nodes"], 1))) for r in runs)
    for (d0, y0), (d1, y1) in zip(points, points[1:]):
        if d0 <= distance <= d1:
            return y0 + (y1 - y0) * (distance - d0) / (d1 - d0)
    if distance < points[0][0]:
        return points[0][1] - PRIOR_SLOPE * (points[0][0] - distance)
    return points[-1][1]


def fit(points):
    """The line (intercept, slope) through the points (distance, log10 nodes), as cost.rs fits it."""
    if not points:
        return PRIOR_INTERCEPT, PRIOR_SLOPE
    n = len(points)
    mx, my = sum(x for x, _ in points) / n, sum(y for _, y in points) / n
    sxx = sum((x - mx) ** 2 for x, _ in points)
    sxy = sum((x - mx) * (y - my) for x, y in points)
    slope = min(max((sxy + PULL * PRIOR_SLOPE) / (sxx + PULL), SLOPES[0]), SLOPES[1])
    return my - slope * mx, slope


def main():
    step_points = int(sys.argv[1]) if len(sys.argv) > 1 else 150
    proof_steps = float(sys.argv[2]) if len(sys.argv) > 2 else 2
    path = Path(__file__).resolve().parents[2] / "target" / "target.jsonl"
    games = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
    rows = []
    for game in games:
        root, runs, gap = game["root"], game["runs"], game["root"] - game["opt"]
        proof = next(r["nodes"] for r in runs if r["proof"])
        depth, spent, points, errors = 0, 0.0, [], []
        while True:
            intercept, slope = fit(points)
            errors.append((len(points), intercept + slope * gap - math.log10(proof)))
            step = round(step_points * PRIOR_SLOPE / slope)
            if gap - depth <= step * proof_steps:
                break
            depth += step
            y = interpolated(runs, depth, root)
            spent += 10 ** y
            if 10 ** y >= MIN_NODES:
                points.append((depth, y))
        rows.append((game["n"], gap, proof, (spent + proof) / proof, errors))
    print(f"steps of {step_points} points, the proof when the gap is within {proof_steps:g} steps")
    print("      n   gap     proof nodes  steps + proof over proof   steps taken")
    for n, gap, proof, over, errors in rows:
        print(f"{n:>7} {gap:>5} {proof:>14}  {over:>10.2f}   {len(errors) - 1}")
    print(f"steps and proof over the proof: median {statistics.median(r[3] for r in rows):.2f}, "
          f"by nodes {sum(r[3] * r[2] for r in rows) / sum(r[2] for r in rows):.2f}")
    print("forecast of the proof (log10 of nodes, forecast less actual) from the line of the steps so far:")
    for k in range(6):
        errors = [e for r in rows for kk, e in r[4] if kk == k]
        if errors:
            print(f"  {k} points of 100 nodes or more: {len(errors):>2} forecasts, median {statistics.median(errors):+.2f}, "
                  f"median absolute {statistics.median(abs(e) for e in errors):.2f}")


main()
