#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of the target experiment from target.jsonl (the lines of `wall target`): what a search under a floor costs
against the full proof, at each share of the gap, and whether the log of its nodes is linear in the distance proven
down from the root's bound.

usage (from rust/): ./bench/target/tables.py [FILE] > bench/target/tables.txt
"""
import json
import math
import statistics
import sys
from pathlib import Path

from run import MAIN

HERE = Path(__file__).resolve().parent
QUESTION = (0.25, 0.5, 0.75)
# Budgets, as shares of the full proof's nodes, for the table of how far down they prove.
BUDGETS = (1e-3, 1e-2, 0.1, 0.3)
# The same in nodes, for the table of how far down from the root's bound (in promotion sum) they prove.
STEPS = (1e3, 1e4, 1e5, 1e6)
# A search of fewer nodes than this is a handful of steps at the top of the tree: the count has a floor there (the
# searches of two nodes), and the fits leave them out.
FLAT = 100
# The chain's cost model, log10(nodes) = A + G / B for a game of root gap G
# (docs/experiments/the-gap-past-2175-2026-10.md).
MODEL_A, MODEL_B = 2.67, 247


def load(path):
    """The lines of the file, one per game, by board size."""
    return {d["n"]: d for d in map(json.loads, path.read_text().splitlines()) if "runs" in d}


def run_at(game, share):
    """The run of `game` at `share` of the gap (1 is the full proof), if there is one."""
    return next((r for r in game["runs"] if abs(r["fraction"] - share) < 1e-9), None)


def ratio(game, share, key):
    """The run's `key` (nodes or cpu_s) over the full proof's, if both completed."""
    run, proof = run_at(game, share), run_at(game, 1.0)
    if run is None or not (run["completed"] and proof["completed"]):
        return None
    return run[key] / proof[key]


def points_of(game, key=lambda r: math.log10(r["nodes"])):
    """The (share, key of the run) points of the completed runs of at least FLAT nodes, by share."""
    return sorted((r["fraction"], key(r)) for r in game["runs"] if r["completed"] and r["nodes"] >= FLAT)


def fit(points):
    """Least squares of y on x: slope, r squared, and the largest residual."""
    n = len(points)
    mx, my = sum(x for x, _ in points) / n, sum(y for _, y in points) / n
    sxx = sum((x - mx) ** 2 for x, _ in points)
    sxy = sum((x - mx) * (y - my) for x, y in points)
    syy = sum((y - my) ** 2 for _, y in points)
    slope = sxy / sxx
    worst = max(abs(y - (my + slope * (x - mx))) for x, y in points)
    return slope, (sxy * sxy / (sxx * syy) if syy else 1.0), worst


def crossing(points, level):
    """The x at which the line through the (x, y) points, in order of x, first reaches `level`, interpolating between
    neighbours; None if it does not."""
    for (x0, y0), (x1, y1) in zip(points, points[1:]):
        if y0 <= level <= y1 and y1 > y0:
            return x0 + (x1 - x0) * (level - y0) / (y1 - y0)
    return None


def spread(values):
    """The median, minimum and maximum of a list, as text."""
    if not values:
        return "none"
    return f"{statistics.median(values):.4g} ({min(values):.4g} - {max(values):.4g})"


def summary(games, ns):
    print(f"{len(ns)} games: {' '.join(map(str, ns))}")
    capped = [(n, r["fraction"]) for n in ns for r in games[n]["runs"] if not r["completed"]]
    print(f"runs that the CPU cap stopped: {capped if capped else 'none'}")
    proofs = [run_at(games[n], 1.0) for n in ns]
    all_cpu = sum(r["cpu_s"] for n in ns for r in games[n]["runs"])
    print(f"the full proofs: {sum(p['cpu_s'] for p in proofs):.0f} CPU s, {sum(p['nodes'] for p in proofs)} nodes; "
          f"all runs: {all_cpu:.0f} CPU s\n")


def per_game(games, ns):
    print("Per game: the gap (root's bound less the optimum), the full proof, and the floor searches at the three")
    print("shares as a fraction of the full proof's nodes and CPU seconds\n")
    head = f"{'n':>5} {'gap':>5} {'proof nodes':>11} {'proof cpu':>9}"
    for share in QUESTION:
        head += f" | {share:>4} nodes   cpu "
    print(head)
    for n in ns:
        g, proof = games[n], run_at(games[n], 1.0)
        row = f"{n:>5} {g['gap']:>5} {proof['nodes']:>11} {proof['cpu_s']:>9.1f}"
        for share in QUESTION:
            nodes, cpu = ratio(g, share, "nodes"), ratio(g, share, "cpu_s")
            row += " | " + ("   capped        " if nodes is None else f"{nodes:>10.2e} {cpu:>8.2e}")
        print(row)


def by_share(games, ns, shares):
    print("\nBy share of the gap proven down: median (min - max) of the floor search's cost over the full proof's\n")
    print(f"{'share':>6} {'games':>5}  {'nodes':<34} {'CPU':<34} {'all games together: nodes, CPU':>12}")
    for share in shares:
        usable = [n for n in ns if ratio(games[n], share, "nodes") is not None]
        together = {}
        for key in ("nodes", "cpu_s"):
            floors = sum(run_at(games[n], share)[key] for n in usable)
            together[key] = floors / sum(run_at(games[n], 1.0)[key] for n in usable)
        nodes = spread([ratio(games[n], share, "nodes") for n in usable])
        cpu = spread([ratio(games[n], share, "cpu_s") for n in usable])
        mark = " <" if share in QUESTION else ""
        print(f"{share:>6} {len(usable):>5}  {nodes:<34} {cpu:<34} "
              f"{together['nodes']:>9.3g} {together['cpu_s']:>8.3g}{mark}")
    main_games = [n for n in ns if n in MAIN]
    print(f"\n(the same for the {len(main_games)} main games, whose standalone searches took 286 to 1450 s: "
          f"{' '.join(map(str, main_games))})\n")
    for share in QUESTION:
        usable = [n for n in main_games if ratio(games[n], share, "nodes") is not None]
        nodes = spread([ratio(games[n], share, "nodes") for n in usable])
        cpu = spread([ratio(games[n], share, "cpu_s") for n in usable])
        print(f"{share:>6} {len(usable):>5}  {nodes:<34} {cpu:<34}")


def linearity(games, ns, cols):
    print("\nThe log of the nodes against the distance proven down: log10(nodes) at each share of the gap, and the")
    print("least-squares line through the floors of at least 100 nodes and the full proof (share 1) of each game\n")
    print(f"{'n':>5} " + " ".join(f"{c:>6}" for c in cols) + f" | {'decades per':>11} {'r2':>6} {'worst':>6}")
    print(f"{'':>5} " + " ".join(f"{'':>6}" for _ in cols) + f" | {'100 of gap':>11} {'':>6} {'resid':>6}")
    fits = []
    for n in ns:
        points = points_of(games[n])
        slope, r2, worst = fit(points)
        per_100 = slope / games[n]["gap"] * 100
        fits.append((per_100, r2, worst))
        cells = {round(x, 6): y for x, y in points}
        row = " ".join(f"{cells[c]:>6.2f}" if round(c, 6) in cells else f"{'-':>6}" for c in cols)
        print(f"{n:>5} {row} | {per_100:>11.3f} {r2:>6.4f} {worst:>6.3f}")
    per = [f[0] for f in fits]
    print(f"\nslope, decades of nodes per 100 of gap proven down: median {statistics.median(per):.3f} "
          f"(min {min(per):.3f}, max {max(per):.3f}); r2 median {statistics.median(f[1] for f in fits):.4f}, "
          f"worst residual over all games {max(f[2] for f in fits):.3f} decades")
    # The slope of each step between two neighbouring runs (the last is from the highest floor to the full proof).
    steps = []
    for n in ns:
        g = games[n]
        for lo, hi in zip(cols, cols[1:]):
            a, b = run_at(g, lo), run_at(g, hi)
            if a["completed"] and b["completed"] and a["nodes"] >= FLAT:
                steps.append((lo, math.log10(b["nodes"] / a["nodes"]) / ((hi - lo) * g["gap"]) * 100))
    print(f"every step between two neighbouring runs, in decades per 100 of gap: "
          f"{spread([slope for _, slope in steps])} ({len(steps)} steps)")
    for lo, hi in ((0, 0.25), (0.25, 0.5), (0.5, 0.75), (0.75, 1.0)):
        part = [slope for start, slope in steps if lo <= start < hi]
        if part:
            print(f"  steps starting at shares {lo}-{hi}: {spread(part)} ({len(part)} steps)")


def against_the_model(games, ns):
    print(f"\nAgainst the chain's cost model (log10 of the nodes of a game's proof is {MODEL_A} + G/{MODEL_B} for its "
          f"root gap G, sd 0.47):\nlog10(nodes) less the model at the gap k*G, median (min - max)\n")
    for share in QUESTION + (1.0,):
        diffs = [math.log10(run_at(games[n], share)["nodes"]) - (MODEL_A + share * games[n]["gap"] / MODEL_B)
                 for n in ns if run_at(games[n], share)["completed"]]
        print(f"{share:>6}: {statistics.median(diffs):6.2f} ({min(diffs):.2f} - {max(diffs):.2f}) decades")


def budgets(games, ns):
    print("\nHow far down a budget proves: the share of the gap at which a game's floor search costs the budget, as a")
    print("share of its full proof's nodes (interpolating log10 of the nodes between neighbouring runs)\n")
    for budget in BUDGETS:
        reached = []
        for n in ns:
            proof = run_at(games[n], 1.0)["nodes"]
            share = crossing(points_of(games[n], lambda r: math.log10(r["nodes"] / proof)), math.log10(budget))
            if share is not None:
                reached.append(share)
        print(f"{budget:>8g} of the proof's nodes: {spread(reached)} ({len(reached)} of {len(ns)} games reach it)")

    print("\nHow far down a budget in nodes proves, from the root's bound: the distance (in promotion sum) and the")
    print("share of the gap at which a game's floor search costs that many nodes\n")
    median_gap = statistics.median(games[n]["gap"] for n in ns)
    for nodes in STEPS:
        shares = [crossing(points_of(games[n]), math.log10(nodes)) for n in ns]
        shares = [(share, share * games[n]["gap"]) for share, n in zip(shares, ns) if share is not None]
        print(f"{nodes:>8g} nodes: distance {spread([d for _, d in shares])} of the median gap {median_gap:.0f}, "
              f"share {spread([s for s, _ in shares])} ({len(shares)} of {len(ns)} games reach it)")


def main():
    games = load(Path(sys.argv[1]) if len(sys.argv) > 1 else HERE / "target.jsonl")
    ns = sorted(games)
    shares = sorted({r["fraction"] for g in games.values() for r in g["runs"] if not r["proof"]})
    summary(games, ns)
    per_game(games, ns)
    by_share(games, ns, shares)
    linearity(games, ns, shares + [1.0])
    against_the_model(games, ns)
    budgets(games, ns)


if __name__ == "__main__":
    main()
