#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""What drives the slope of the cost curve: the tables of
solver/notebook/search/what-drives-the-slope-2026-10/README.md.

Reads the recorded chain (nodes on 4 threads, proofs), slope_roots.jsonl (`slope roots`: the root of every non-prime
game 880-1300 and of the games past 1300 that bands.jsonl solved) and slope_proofs.jsonl (`slope prove`: one-thread
searches with the incumbent at the optimum plus a margin delta; delta = 0 for every searched game, more margins for
some), joins them by n and prints markdown.  The commands that made the data files are in that entry.  Pure
Python (no numpy); `./slope.py > slope_output.md`.

The recorded chain is results/strict.jsonl as of commit 2487671 (STRICT_COMMIT), read from git: the 1..1300 chain of
the older build, whose node counts that entry quotes.  The file was replaced by the GF(2) build's chain at 0133afe
and 0b5a136, and with that one the script stops at its check of the caps of the games past 1300.  `--strict PATH`
reads another file instead.
"""
import argparse
import json
import math
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
STRICT_FILE = "rust/results/strict.jsonl"  # where the file was at STRICT_COMMIT, before the layout of solver/
# The chain whose node counts the section quotes; the file was replaced by the GF(2) build's chain at 0133afe/0b5a136.
STRICT_COMMIT = "2487671c38c051958fee9b8e2f0e0edd07715878"


def parse(text):
    return [json.loads(line) for line in text.splitlines() if line.strip()]


def load(path):
    return parse(path.read_text())


def recorded_chain(path):
    """The lines of the recorded chain: the file `path` if given, else STRICT_FILE as of STRICT_COMMIT, from git."""
    if path:
        return load(path)
    try:
        shown = subprocess.run(["git", "show", f"{STRICT_COMMIT}:{STRICT_FILE}"], cwd=REPO, check=True,
                               capture_output=True, text=True)
    except (OSError, subprocess.CalledProcessError) as error:
        detail = error.stderr.strip() if isinstance(error, subprocess.CalledProcessError) else error
        sys.exit(f"slope.py: cannot read {STRICT_FILE} as of commit {STRICT_COMMIT} with git ({detail}); "
                 "run it inside the repository's checkout with that commit, or pass --strict PATH")
    return parse(shown.stdout)


def solve(a, b):
    """Solves a x = b (a small square matrix) by Gauss-Jordan elimination with partial pivoting."""
    k = len(b)
    m = [row[:] + [b[i]] for i, row in enumerate(a)]
    for c in range(k):
        p = max(range(c, k), key=lambda r: abs(m[r][c]))
        m[c], m[p] = m[p], m[c]
        for r in range(k):
            if r != c:
                f = m[r][c] / m[c][c]
                m[r] = [x - f * y for x, y in zip(m[r], m[c])]
    return [m[i][k] / m[i][i] for i in range(k)]


class Fit:
    """Least squares of y on the columns xs and an intercept."""

    def __init__(self, xs, y):
        cols = [[1.0] * len(y)] + [[float(x) for x in col] for col in xs]
        k, n = len(cols), len(y)
        xtx = [[sum(a * b for a, b in zip(cols[i], cols[j])) for j in range(k)] for i in range(k)]
        self.beta = solve(xtx, [sum(a * b for a, b in zip(cols[i], y)) for i in range(k)])
        self.res = [yi - sum(self.beta[j] * cols[j][i] for j in range(k)) for i, yi in enumerate(y)]
        ss = sum(r * r for r in self.res)
        mean = sum(y) / n
        self.r2 = 1 - ss / sum((v - mean) ** 2 for v in y)
        self.sd = math.sqrt(ss / (n - k))
        inv = [solve(xtx, [1.0 if r == c else 0.0 for r in range(k)]) for c in range(k)]
        self.se = [self.sd * math.sqrt(inv[j][j]) for j in range(k)]

    def slope(self, i, scale=1, digits=5):
        return f"{scale * self.beta[i]:.{digits}f} ± {scale * self.se[i]:.{digits}f}"

    def predict(self, *x):
        return self.beta[0] + sum(b * v for b, v in zip(self.beta[1:], x))


def corr(a, b):
    ma, mb = sum(a) / len(a), sum(b) / len(b)
    sab = sum((x - ma) * (y - mb) for x, y in zip(a, b))
    return sab / math.sqrt(sum((x - ma) ** 2 for x in a) * sum((y - mb) ** 2 for y in b))


def table(header, rows):
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    for row in rows:
        print("| " + " | ".join(str(c) for c in row) + " |")
    print()


def gaps(r):
    """The variables of a game measured at its root (a line of slope_roots.jsonl)."""
    opt = r["opt"]
    return {
        "n": r["n"],
        "candidates": r["candidates"],
        "max_total": r["max_total"],
        "matching gap": r["matching"]["bound"] - opt,
        "refined gap (3 levels)": r["refined"][-1] - opt,
        "Lagrangian gap": r["lagrange"]["bound"] - opt,
        "undecided (Lagrangian)": r["lagrange"]["undecided"],
        "cap gap": None if r["cap"] is None else r["cap"] - opt,
        "greedy gap": opt - r["greedy"],
        "start gap": opt - r["start"],
    }


def main(argv):
    parser = argparse.ArgumentParser(description="The tables of the entry in README.md.")
    parser.add_argument("--strict", type=Path,
                        help=f"the recorded chain (default: {STRICT_FILE} as of {STRICT_COMMIT[:7]}, from git)")
    strict = {r["n"]: r for r in recorded_chain(parser.parse_args(argv).strict)}
    roots = {r["n"]: r for r in load(HERE / "slope_roots.jsonl")}
    proofs = defaultdict(dict)
    depths = {}
    for r in load(HERE / "slope_proofs.jsonl"):
        assert r["completed"], r
        proofs[r["n"]][r["delta"]] = r["nodes"]
        if r["steps_by_depth"]:
            depths[(r["n"], r["delta"])] = r["steps_by_depth"]
    v = {n: gaps(r) for n, r in roots.items()}
    for n, s in strict.items():
        if n in roots:
            assert roots[n]["opt"] == s["score"] - s["in_the_bag"] and s["setup"].get("cap") in (None, roots[n]["cap"])

    searched = sorted(n for n, s in strict.items() if s["proof"] == "exhausted" and n in roots)
    beyond = sorted(n for n in roots if n not in strict)
    assert all(0 in proofs[n] for n in searched), "a searched game without its proof-only count"
    chain = {n: math.log10(strict[n]["nodes"]) for n in searched}
    proof = {n: math.log10(c[0]) for n, c in proofs.items() if 0 in c}
    names = [k for k in v[searched[0]] if k != "n"]
    names.insert(0, "n")
    scans = sorted(n for n in proofs if sum(1 for d in proofs[n] if d > 0) >= 4)

    print(f"# What drives the slope: the tables\n\nGenerated by `./slope.py`.  {len(searched)} games of 880-1300 "
          f"searched to exhaustion in `results/strict.jsonl`; {len(roots)} non-prime games measured at the root "
          f"({len(beyond)} of them past 1300, from `bands.jsonl`); margin scans of {len(scans)} games.  'chain' = "
          "the recorded node count (4 threads, from the chain's start incumbent), 'proof' = one thread, incumbent at "
          "the optimum (delta = 0).  Gaps are root bounds minus the optimum promotion sum P*.  Logarithms are "
          "base 10.\n")

    print("## 1. log10(nodes) against one variable (the searched games)\n")
    rows = []
    for name in names:
        x = [v[n][name] for n in searched]
        row = [name]
        for y in ([chain[n] for n in searched], [proof[n] for n in searched]):
            f = Fit([x], y)
            row += [f.slope(1), f"{f.r2:.2f}", f"{f.sd:.2f}"]
        rows.append(row)
    table(["variable", "chain: slope per unit", "R²", "resid. sd", "proof: slope per unit", "R²", "resid. sd"], rows)

    y = [proof[n] for n in searched]
    print("## 2. log10(proof nodes) against two variables\n")
    pairs = [("n", "matching gap"), ("n", "Lagrangian gap"), ("candidates", "matching gap"),
             ("candidates", "Lagrangian gap"), ("max_total", "Lagrangian gap"), ("matching gap", "Lagrangian gap"),
             ("n", "candidates")]
    rows = []
    for a, b in pairs:
        f = Fit([[v[n][a] for n in searched], [v[n][b] for n in searched]], y)
        rows.append([f"{a} + {b}", f"{f.slope(1)} (t {f.beta[1] / f.se[1]:.1f})",
                     f"{f.slope(2)} (t {f.beta[2] / f.se[2]:.1f})", f"{f.r2:.2f}", f"{f.sd:.2f}"])
    table(["variables", "first: slope", "second: slope", "R²", "resid. sd"], rows)

    print("Residuals of one fit against another variable:\n")
    rows = []
    for first, other in [("n", "Lagrangian gap"), ("Lagrangian gap", "n"), ("n", "matching gap"),
                         ("matching gap", "n"), ("Lagrangian gap", "candidates")]:
        res = Fit([[v[n][first] for n in searched]], y).res
        x = [v[n][other] for n in searched]
        f = Fit([x], res)
        rows.append([f"log10(nodes) ~ {first}", other, f"{corr(res, x):.2f}", f.slope(1), f"{f.r2:.2f}"])
    table(["fit", "residuals against", "correlation", "slope", "R²"], rows)

    print("## 3. The gaps against n\n")
    every = sorted(n for n in roots if n in strict)
    rows = []
    for label, ns in [(f"all {len(every)} non-prime games", every), (f"the {len(searched)} searched", searched)]:
        for name in ("matching gap", "refined gap (3 levels)", "Lagrangian gap", "cap gap", "greedy gap"):
            pts = [(n, v[n][name]) for n in ns if v[n][name] is not None]
            f = Fit([[n for n, _ in pts]], [g for _, g in pts])
            rows.append([label, name, f.slope(1, 100, 0), f"{f.r2:.2f}", f"{f.sd:.0f}"])
    table(["games", "gap", "per 100 n", "R²", "resid. sd"], rows)

    print("By bands of n (searched games; means; past 1300 the games of `bands.jsonl`, all searched here):\n")
    rows = []
    for lo, hi in [(lo, lo + 49) for lo in range(851, 1300, 50)] + [(1301, 1310), (1350, 1352)]:
        ns = [n for n in searched + beyond if lo <= n <= hi]
        if not ns:
            continue
        m = lambda f: sum(f(n) for n in ns) / len(ns)  # noqa: E731
        pr = [proof[n] for n in ns if n in proof]
        rows.append([f"{lo}-{hi}", len(ns), f"{m(lambda n: v[n]['candidates']):.0f}",
                     f"{m(lambda n: v[n]['matching gap']):.0f}", f"{m(lambda n: v[n]['Lagrangian gap']):.0f}",
                     f"{m(lambda n: v[n]['undecided (Lagrangian)']):.0f}",
                     f"{sum(pr) / len(pr):.2f} ({len(pr)})" if pr else "-",
                     f"{sum(chain[n] for n in ns if n in chain) / len(ns):.2f}" if ns[0] in chain else "-"])
    table(["n", "games", "candidates", "matching gap", "Lagrangian gap", "undecided", "log10 proof nodes (games)",
           "log10 chain nodes"], rows)

    print("## 4. Margin scans: every bound lowered by delta (the incumbent at P* + delta), one thread\n")
    rows, points = [], []
    margins = sorted({d for n in scans for d in proofs[n] if d >= 0})
    for n in scans:
        lg = {d: math.log10(max(c, 1)) for d, c in sorted(proofs[n].items()) if d >= 0}
        near = [(d, y) for d, y in lg.items() if d <= 200]
        f = Fit([[d for d, _ in near]], [y for _, y in near])
        # The margin at which the tree is down to 100 nodes, interpolated in log10(nodes).
        ds = sorted(lg)
        d100 = next((d0 + (lg[d0] - 2) / (lg[d0] - lg[d1]) * (d1 - d0) for d0, d1 in zip(ds, ds[1:])
                     if lg[d0] >= 2 > lg[d1]), None)
        rows.append([n, v[n]["matching gap"], v[n]["Lagrangian gap"],
                     " ".join(f"{lg[d]:.2f}" if d in lg else "-" for d in margins), f"{-f.beta[1]:.4f}",
                     "-" if d100 is None else f"{d100:.0f}"])
        points += [(n, d, y) for d, y in lg.items() if y >= 1.5]
    table(["n", "matching gap", "Lagrangian gap", "log10 N at the margins " + ", ".join(str(d) for d in margins),
           "decades per unit of margin (0-200)", "margin at 100 nodes"], rows)

    print("One fit over the scan points with at least 32 nodes: log10 N = a + c (gap - delta) [+ d n]:\n")
    rows = []
    for name in ("matching gap", "Lagrangian gap"):
        for limit in (200, 10**9):
            pts = [p for p in points if p[1] <= limit]
            yy = [p[2] for p in pts]
            eff = [v[p[0]][name] - p[1] for p in pts]
            for xs, label in [([eff], "gap - delta"), ([eff, [p[0] for p in pts]], "gap - delta, n")]:
                f = Fit(xs, yy)
                coef = ", ".join(f.slope(i) for i in range(1, len(f.beta)))
                rows.append([name, "all" if limit > 10**6 else f"delta <= {limit}", label, coef, f"{f.r2:.2f}",
                             f"{f.sd:.2f}", len(yy)])
    table(["gap", "margins", "variables", "slopes", "R²", "resid. sd", "points"], rows)

    print("The same remaining Lagrangian gap (gap - delta within 12 of the target), in games of any size:\n")
    rows = []
    for target in (100, 200, 300, 400):
        hits = sorted((p[0], p[1], p[2]) for p in points if abs(v[p[0]]["Lagrangian gap"] - p[1] - target) <= 12)
        hits += sorted((n, 0, proof[n]) for n in searched if abs(v[n]["Lagrangian gap"] - target) <= 12
                       and n not in scans)
        ys = [h[2] for h in hits]
        rows.append([target, len(hits), f"{min(ys):.2f}-{max(ys):.2f}",
                     ", ".join(f"{n}{'' if d == 0 else f' (delta {d})'}: {y:.2f}" for n, d, y in hits
                               if d > 0 or n in scans)])
    table(["remaining gap", "points", "log10 N range", "scanned games among them (log10 N)"], rows)

    print("## 5. Matched comparisons (proof nodes)\n")
    rows = []
    for label, close, far, key_close, key_far in [
        ("similar Lagrangian gap (within 15), n at least 100 apart", 15, 100, "Lagrangian gap", "n"),
        ("similar n (within 30), Lagrangian gap at least 100 apart", 30, 100, "n", "Lagrangian gap"),
        ("similar matching gap (within 25), n at least 100 apart", 25, 100, "matching gap", "n"),
    ]:
        slopes = []
        for i, a in enumerate(searched):
            for b in searched[i + 1:]:
                if abs(v[a][key_close] - v[b][key_close]) <= close and abs(v[a][key_far] - v[b][key_far]) >= far:
                    slopes.append((proof[b] - proof[a]) / (v[b][key_far] - v[a][key_far]))
        mean = sum(slopes) / len(slopes)
        sd = math.sqrt(sum((s - mean) ** 2 for s in slopes) / (len(slopes) - 1))
        rows.append([label, len(slopes), f"{100 * mean:.2f}", f"{100 * sd:.2f}"])
    table(["pairs", "count", "mean change of log10 nodes per 100 of the other", "sd"], rows)

    print("## 6. Past 1300, out of sample: the fits of the searched games against the games of `bands.jsonl`\n")
    models = [("Lagrangian gap", ["Lagrangian gap"]), ("matching gap", ["matching gap"]),
              ("both gaps", ["matching gap", "Lagrangian gap"]), ("Lagrangian gap and n", ["Lagrangian gap", "n"])]
    fits = [Fit([[v[n][k] for n in searched] for k in keys], y) for _, keys in models]
    near = [p for p in points if p[1] <= 200]
    scan_fit = Fit([[v[p[0]]["Lagrangian gap"] - p[1] for p in near]], [p[2] for p in near])
    rows = []
    for n in beyond:
        for d, c in sorted(proofs.get(n, {}).items()):
            g = v[n]["Lagrangian gap"]
            pred = [f"{f.predict(*(v[n][k] for k in keys)):.2f}" if d == 0 else "-"
                    for f, (_, keys) in zip(fits, models)]
            rows.append([n, v[n]["matching gap"], g, d, f"{math.log10(c):.2f}", *pred,
                         f"{scan_fit.predict(g - d):.2f}"])
    table(["n", "matching gap", "Lagrangian gap", "delta", "log10 N"] + [f"predicted by {m}" for m, _ in models]
          + ["by the scan fit (gap - delta, delta <= 200)"], rows)
    print("(Residual sd of those fits on the searched games: "
          + ", ".join(f"{f.sd:.2f}" for f in fits + [scan_fit])
          + ".  The games without a proof count are measured at the root only.)\n")

    print("## 7. Starting below the optimum\n")
    at_opt = [n for n in searched if v[n]["start gap"] == 0]
    below = [n for n in searched if v[n]["start gap"] > 0]
    rows = []
    for label, ns in [("start = optimum", at_opt), ("start below the optimum", below)]:
        ratio = [chain[n] - proof[n] for n in ns]
        rows.append([label, len(ns), f"{10 ** (sum(ratio) / len(ratio)):.2f}",
                     f"{sum(v[n]['start gap'] for n in ns) / len(ns):.0f}"])
    table(["searched games, by the chain's start incumbent", "count",
           "chain nodes / proof nodes (geometric mean)", "mean start gap"], rows)
    below_scans = sorted(n for n in proofs if any(d < 0 for d in proofs[n]))
    deltas = sorted({d for n in below_scans for d in proofs[n] if d <= 0}, reverse=True)
    rows = [[n] + [f"{proofs[n][d] / proofs[n][0]:.2f}" for d in deltas] for n in below_scans]
    print("Nodes with the incumbent starting below the optimum (a value, no set), relative to the proof:\n")
    table(["n"] + [f"delta {d}" for d in deltas], rows)

    print("## 8. Where the nodes are: steps by the number of promotions of the node\n")
    rows = []
    for (n, d), steps in sorted(depths.items()):
        total = sum(steps)
        mean = sum(i * c for i, c in enumerate(steps)) / total
        peak = max(range(len(steps)), key=lambda i: steps[i])
        rows.append([n, d, total, f"{mean:.1f}", peak, len(steps) - 1, v[n]["max_total"],
                     f"{(math.log10(steps[peak]) - math.log10(steps[5])) / (peak - 5):.3f}"])
    table(["n", "delta", "steps", "mean depth", "busiest depth", "deepest", "max_total",
           "decades per level from depth 5 to the busiest"], rows)

    print("## 9. The root's nogoods in one refit of the search (`fit`: 50 steps from zero multipliers)\n")
    rows = []
    for lo, hi in [(lo, lo + 49) for lo in range(851, 1300, 50)] + [(1301, 1352)]:
        rs = [roots[n] for n in sorted(roots) if lo <= n <= hi]
        m = lambda f: sum(f(r) for r in rs) / len(rs)  # noqa: E731
        rows.append([f"{lo}-{hi}", len(rs), f"{m(lambda r: r['lagrange']['nogoods']):.0f}",
                     f"{m(lambda r: r['matching']['bound'] - r['opt']):.0f}",
                     f"{m(lambda r: r['lagrange']['bound'] - r['opt']):.0f}",
                     f"{m(lambda r: r['lagrange']['refit'] - r['opt']):.0f}"])
    table(["n", "games", "root nogoods", "matching gap", "Lagrangian gap (converged)", "gap after one refit"], rows)


if __name__ == "__main__":
    main(sys.argv[1:])
