#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The gap past 2175: solver/notebook/search/the-gap-past-2175-2026-10/README.md.

The root GF(2) gap of a game (the root's bound less the chain's optimum) against the size of the tree that the
EC2 chain made of it.  Inputs (this directory unless a path says otherwise):

* gaps_chain.jsonl: one line per searched game past 2175 of a snapshot of the EC2 chain (a completed search whose
  proof is `exhausted` or `cap`): the root's bound, optimum and gap from `examples/wall.rs nodes --nodes 1` over the
  snapshot, and the chain's own nodes, seconds, proof and threads (`./gap.py join` makes it).
* anatomy_chain.jsonl: the root's conflicts and cores of those games, from `examples/matroid.rs` (`lagrange --limit 0`
  and `cores`; `./gap.py join-anatomy` makes it).
* resolved_1301_2175.jsonl: the committed chain's searched games of 1301-2175 solved again by the build of the
  EC2 chain, one thread, in order, with their roots' gaps (`./gap.py join-resolved` makes it).
* ../../results/strict.jsonl and ../gf2/roots.jsonl: the committed chain to CHAIN_END = 2175 and its roots (the gaps of
  1301-2175, the committed chain's own nodes).  The chain was that long when `gap_output.md` was made; the file has
  grown since, and the tables read only its games to CHAIN_END (the EC2 chain's games begin past it).
* ../cloud/c8g48/games.jsonl: the scaling run's per-game nodes at windows of 1, 4 and 16 (the speculation).
* ../cloud/c8g.chain-2176.jsonl: the Graviton calibration's chain, 2176-2276 (the build against the EC2 chain's).
* setup_c8g.jsonl: the fixed costs of a game on Graviton4.

    ./gap.py games SNAPSHOT                           the searched games past 2175, to pass to `wall` and `matroid`
    ./gap.py join SNAPSHOT RAW > gaps_chain.jsonl     the data file, from a snapshot and wall's output
    ./gap.py join-anatomy SNAPSHOT LAGRANGE CORES > anatomy_chain.jsonl
    ./gap.py join-resolved RESULTS > resolved_1301_2175.jsonl
    ./gap.py > gap_output.md                          every table
    ./gap.py bands trend cost                         only the named ones

The tables: bands (the root gaps by band, against 1301-2175), trend (the gap's growth with n, C5), window (the
speculation and thread effects on a game's nodes), build (the committed chain's build against the EC2 chain's on
the same games), cost (the cost model refitted and tested out of sample), runs (the residuals' structure and the dear
games), anatomy (what at the root explains the residuals, A4), projection (a game at 4000, 5000 and 6000 and what 24
hours reach).  Pure Python; the inputs are found relative to this file.
"""
import collections
import json
import math
import random
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
STRICT = REPO / "results" / "strict.jsonl"
ROOTS = HERE.parent / "the-gf2-bound-in-the-search-2026-10" / "roots.jsonl"
SCALING = HERE.parents[1] / "scaling-to-192-threads-2026-10" / "c8g48" / "games.jsonl"
SETUP = HERE / "setup_c8g.jsonl"
DATA = HERE / "gaps_chain.jsonl"
ANATOMY = HERE / "anatomy_chain.jsonl"
RESOLVED = HERE / "resolved_1301_2175.jsonl"
CALIBRATION = REPO / "solver" / "frame" / "testdata" / "c8g.chain-2176.jsonl"

CHAIN_END = 2175  # the last game of the committed chain that the tables use (strict.jsonl runs on past it)
FIRST = CHAIN_END + 1  # the first game past it, where the EC2 chain's games begin
LAST = 3312  # the last searched game of the snapshot
MIN_NODES = 50  # the fits take the exhausted searches of at least this many nodes, as AGENDA section 7.1 does
P = 0.3  # the exponent of n in the scale of the gap, G / n^P (the cost table says why)


# ---------------------------------------------------------------------------------------------------------
# Least squares and resampling

def inverse(a):
    n = len(a)
    m = [row[:] + [1.0 if i == j else 0.0 for j in range(n)] for i, row in enumerate(a)]
    for c in range(n):
        p = max(range(c, n), key=lambda r: abs(m[r][c]))
        m[c], m[p] = m[p], m[c]
        d = m[c][c]
        m[c] = [x / d for x in m[c]]
        for r in range(n):
            if r != c and m[r][c]:
                f = m[r][c]
                m[r] = [x - f * y for x, y in zip(m[r], m[c])]
    return [row[n:] for row in m]


def ols(columns, y):
    """Least squares of y on the columns (a constant column included by the caller): coefficients, standard errors
    (classical, and heteroscedasticity-robust HC1), R2, residuals and their standard deviation."""
    n, k = len(y), len(columns)
    xtx = [[sum(a * b for a, b in zip(columns[i], columns[j])) for j in range(k)] for i in range(k)]
    inv = inverse(xtx)
    xty = [sum(a * b for a, b in zip(columns[i], y)) for i in range(k)]
    beta = [sum(inv[i][j] * xty[j] for j in range(k)) for i in range(k)]
    resid = [y[t] - sum(beta[j] * columns[j][t] for j in range(k)) for t in range(n)]
    rss = sum(e * e for e in resid)
    mean_y = sum(y) / n
    tss = sum((v - mean_y) ** 2 for v in y)
    s2 = rss / max(1, n - k)
    meat = [[sum(columns[i][t] * columns[j][t] * resid[t] ** 2 for t in range(n)) for j in range(k)] for i in range(k)]
    cov = [[sum(inv[i][a] * meat[a][b] * inv[b][j] for a in range(k) for b in range(k)) * n / max(1, n - k)
            for j in range(k)] for i in range(k)]
    return {"coef": beta, "se": [math.sqrt(s2 * inv[i][i]) for i in range(k)],
            "se_robust": [math.sqrt(max(0.0, cov[i][i])) for i in range(k)], "r2": 1 - rss / tss if tss else 0.0,
            "resid": resid, "sd": math.sqrt(s2), "n": n}


def line(xs, ys):
    """Least squares of ys on xs: (intercept, slope, fit)."""
    fit = ols([[1.0] * len(xs), list(xs)], list(ys))
    return fit["coef"][0], fit["coef"][1], fit


def mean(v):
    return sum(v) / len(v)


def sd(v):
    m = mean(v)
    return math.sqrt(sum((x - m) ** 2 for x in v) / max(1, len(v) - 1))


def median(v):
    return quantile(v, 0.5)


def quantile(v, q):
    v = sorted(v)
    pos = q * (len(v) - 1)
    lo = int(math.floor(pos))
    hi = min(lo + 1, len(v) - 1)
    return v[lo] + (v[hi] - v[lo]) * (pos - lo)


def autocorrelation(e, lag):
    m = mean(e)
    den = sum((x - m) ** 2 for x in e)
    return sum((e[i] - m) * (e[i + lag] - m) for i in range(len(e) - lag)) / den if den else 0.0


def correlation(x, y):
    mx, my = mean(x), mean(y)
    num = sum((a - mx) * (b - my) for a, b in zip(x, y))
    den = math.sqrt(sum((a - mx) ** 2 for a in x) * sum((b - my) ** 2 for b in y))
    return num / den if den else 0.0


def block_indices(n, block, rng):
    """A moving-block bootstrap sample of the indices 0..n: blocks of `block` consecutive ones."""
    idx = []
    while len(idx) < n:
        start = rng.randrange(0, max(1, n - block + 1))
        idx.extend(range(start, min(n, start + block)))
    return idx[:n]


def block_bootstrap(n, statistic, draws=2000, block=15, seed=1):
    rng = random.Random(seed)
    return [statistic(block_indices(n, block, rng)) for _ in range(draws)]


def interval(values, low=0.025, high=0.975):
    return quantile(values, low), quantile(values, high)


# ---------------------------------------------------------------------------------------------------------
# The data

def read_jsonl(path):
    with open(path) as f:
        return [json.loads(text) for text in f if text.strip()]


def completed(path):
    """The completed games of a results file, by board size (the last line of a game wins)."""
    return {r["n"]: r for r in read_jsonl(path) if r.get("completed")}


def committed():
    """The completed games of the committed chain, 1..CHAIN_END."""
    return {n: r for n, r in completed(STRICT).items() if n <= CHAIN_END}


def games_of(snapshot):
    """The board sizes past 2175 whose line is a completed search, on one line, for `wall` and `matroid`."""
    chain = completed(snapshot)
    print(" ".join(str(n) for n in sorted(chain) if n >= FIRST and chain[n]["method"] == "search"))


def join(snapshot, raw):
    """The data file: wall's `nodes --nodes 1` lines of the searched games, with the chain's own columns."""
    chain = completed(snapshot)
    for g in read_jsonl(raw):
        c = chain[g["n"]]
        out = {
            "n": g["n"], "gap": g["gap_opt"], "root_bound": g["root_bound"], "opt": g["opt"], "start": g["start"],
            "proof": c["proof"], "nodes": c["nodes"], "search_s": round(c["seconds"]["search_all_runs"], 3),
            "threads": c["threads"], "invocations": c["invocations"], "candidates": g["candidates"],
            "max_total": g["max_total"], "root_chosen": g["root_chosen"], "root_forced": g["root_forced"],
            "elements": g["elements"], "ranks": g["ranks"], "frame_moves": g["frame_moves"],
            "frame_factors": g["frame_factors"], "frame_freedoms": g["frame_freedoms"],
        }
        print(json.dumps(out, separators=(",", ":")))


def join_anatomy(snapshot, lagrange, cores):
    """The anatomy file: the sizes of the conflicts of the root's heaviest set (`matroid lagrange --limit 0`) and the
    cores it runs into (`matroid cores`), per game."""
    chain = completed(snapshot)
    first = {g["n"]: g for g in read_jsonl(lagrange)}
    core = {g["n"]: g for g in read_jsonl(cores)}
    for n in sorted(first):
        if n not in core or n not in chain:
            continue
        out = {"n": n, "conflicts": first[n]["first_conflicts"], "cores": core[n]["cores"], "ranks": core[n]["ranks"],
               "gf2": first[n]["gf2"], "opt": first[n]["opt"]}
        print(json.dumps(out, separators=(",", ":")))


def join_resolved(results):
    """The resolved file: each searched game of the committed chain's 1301-2175 as the build of the EC2 chain
    solved it again (`taxman solve 1301-2175 --threads 1 --out RESULTS`, in order), with its root gap from
    ../gf2/roots.jsonl and the committed chain's own nodes, for the same game."""
    strict = committed()
    roots = {r["n"]: r for r in read_jsonl(ROOTS)}
    again = completed(results)
    for n in sorted(strict):
        r = strict[n]
        if n > 1300 and r["method"] == "search" and n in roots and n in again:
            s = again[n]
            out = {"n": n, "gap": roots[n]["gf2"] - roots[n]["opt"], "method": s["method"], "proof": s["proof"],
                   "nodes": s["nodes"], "search_s": round(s["seconds"]["search_all_runs"], 3), "threads": s["threads"],
                   "score": s["score"], "committed_proof": r["proof"], "committed_nodes": r["nodes"],
                   "committed_score": r["score"]}
            print(json.dumps(out, separators=(",", ":")))


def load_games():
    return sorted(read_jsonl(DATA), key=lambda r: r["n"])


def baseline():
    """The searched games of 1301-2175 of the committed chain with their roots: n, gap, nodes, proof."""
    strict = committed()
    roots = {r["n"]: r for r in read_jsonl(ROOTS)}
    out = []
    for n in sorted(strict):
        r = strict[n]
        if n > 1300 and r["method"] == "search" and n in roots:
            out.append({"n": n, "gap": roots[n]["gf2"] - roots[n]["opt"], "nodes": r["nodes"], "proof": r["proof"]})
    return out


def resolved():
    """The committed chain's searched games of 1301-2175 as the EC2 chain's build solved them again."""
    return sorted(read_jsonl(RESOLVED), key=lambda r: r["n"]) if RESOLVED.exists() else []


def exhausted(rows):
    return [r for r in rows if r["proof"] == "exhausted" and r["nodes"] >= MIN_NODES]


def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out.extend("| " + " | ".join(str(c) for c in row) + " |" for row in rows)
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# bands: the root gaps by band

def band_row(label, rows):
    gaps = [r["gap"] for r in rows]
    ratios = [1000 * r["gap"] / r["n"] for r in rows]
    return [label, len(rows), f"{mean(gaps):.0f}", f"{median(gaps):.0f}", f"{sd(gaps):.0f}", min(gaps), max(gaps),
            f"{mean(ratios):.0f}"]


def bands(games):
    """Gap by band over the searched games of the committed chain (200 to a band) and of the EC2 chain (100)."""
    header = ["band", "searched games", "mean gap", "median", "sd", "min", "max", "mean 1000 G/n"]
    rows = []
    old = baseline()
    for lo in range(1301, 2101, 200):
        part = [r for r in old if lo <= r["n"] < lo + 200]
        rows.append(band_row(f"{lo}-{lo + 199} (committed)", part))
    for lo in range(2101, games[-1]["n"] + 1, 100):
        part = [r for r in old + games if lo <= r["n"] < lo + 100]
        rows.append(band_row(f"{lo}-{lo + 99}" + (" (committed to 2175)" if lo == 2101 else ""), part))
    rows.append(band_row(f"{FIRST}-{LAST}, all", games))
    return table(header, rows)


# ---------------------------------------------------------------------------------------------------------
# window: what the speculation and the threads do to a game's nodes

def scaling_runs():
    """The scaling run's searches by run name and board size (exhausted, of at least 1000 nodes)."""
    runs = collections.defaultdict(dict)
    for g in read_jsonl(SCALING):
        if g["method"] == "search" and g["proof"] == "exhausted" and g["nodes"] >= 1000:
            runs[g["run"]][g["n"]] = g["nodes"]
    return runs


def log_ratios(a, b):
    """log10 of a's nodes over b's, for the games in both."""
    return [math.log10(a[n] / b[n]) for n in sorted(set(a) & set(b))]


def window_of(name):
    """The window of a scaling run, from its name (`..._W16_...`), or None."""
    m = re.search(r"_W(\d+)(?:_|$)", name)
    return int(m.group(1)) if m else (1 if name.startswith("n2277") else None)


def window(games):
    chain = {r["n"]: r["nodes"] for r in games if r["proof"] == "exhausted" and r["nodes"] >= 1000}
    runs = scaling_runs()
    header = ["run", "window", "games", "mean log10 of its nodes over the chain's", "sd", "geometric mean ratio",
              "sum of nodes over the chain's"]
    rows = []
    per_game = collections.defaultdict(lambda: collections.defaultdict(list))
    for name in sorted(runs, key=lambda s: (window_of(s) or 0, s)):
        w = window_of(name)
        both = sorted(set(runs[name]) & set(chain))
        if w is None or len(both) < 5:
            continue
        lr = log_ratios(runs[name], chain)
        for n in both:
            per_game[w][n].append(math.log10(runs[name][n] / chain[n]))
        total = sum(runs[name][n] for n in both) / sum(chain[n] for n in both)
        rows.append([name, w, len(lr), f"{mean(lr):+.3f}", f"{sd(lr):.3f}", f"{10 ** mean(lr):.2f}", f"{total:.2f}"])
    out = [table(header, rows), "",
           "By window, each game once (its mean over the runs of that window, so that the games the runs share count "
           "once):", ""]
    summary = []
    for w, games_of_w in sorted(per_game.items()):
        v = [mean(x) for x in games_of_w.values()]
        early = [mean(x) for n, x in games_of_w.items() if n <= 2276]
        late = [mean(x) for n, x in games_of_w.items() if n > 2276]
        summary.append([w, len(v), f"{mean(v):+.3f}", f"{sd(v) / math.sqrt(len(v)):.3f}", f"{sd(v):.3f}",
                        f"{10 ** mean(v):.2f}", f"{mean(early):+.3f} ({len(early)})" if early else "-",
                        f"{mean(late):+.3f} ({len(late)})" if late else "-"])
    out.append(table(["window", "games", "mean log10", "se", "sd", "ratio", "games of 2226-2276", "of 2277-2336"],
                     summary))
    out.append("")
    t64, t192 = runs.get("i8b_T64_W1", {}), runs.get("i8b_T192_W1", {})
    lr = log_ratios(t192, t64)
    out.append(f"Threads at a window of 1, 192 against 64 (2226..2276, {len(lr)} games): {mean(lr):+.3f} decades, "
               f"sd {sd(lr):.3f}.")
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# build: the committed chain's build against the EC2 chain's, on the same games

def build(games):
    again = resolved()
    out = [f"The committed chain's {len(again)} searched games of 1301-2175 solved again by the EC2 chain's "
           f"build, in order on one thread: {sum(1 for r in again if r['score'] != r['committed_score'])} scores "
           f"differ from the committed chain's, {sum(1 for r in again if r['proof'] != r['committed_proof'])} "
           "proofs.", ""]
    both = [r for r in again if r["proof"] == "exhausted" and r["committed_proof"] == "exhausted"
            and min(r["nodes"], r["committed_nodes"]) >= 1000]
    header = ["games", "searches of 1000 nodes or more", "mean log10 of the new nodes over the committed", "sd",
              "ratio of the sums", "correlation with the gap", "with n"]
    rows = []
    for lo, hi in ((1301, 1500), (1501, 1700), (1701, 1900), (1901, 2100), (2101, 2175), (1301, 2175)):
        part = [r for r in both if lo <= r["n"] <= hi]
        if len(part) < 5:
            continue
        lr = [math.log10(r["nodes"] / r["committed_nodes"]) for r in part]
        rows.append([f"{lo}-{hi}", len(part), f"{mean(lr):+.3f}", f"{sd(lr):.3f}",
                     f"{sum(r['nodes'] for r in part) / sum(r['committed_nodes'] for r in part):.2f}",
                     f"{correlation([r['gap'] for r in part], lr):+.2f}",
                     f"{correlation([r['n'] for r in part], lr):+.2f}"])
    out += [table(header, rows), ""]
    calibration = {r["n"]: r for r in completed(CALIBRATION).values()
                   if r["method"] == "search" and r["proof"] == "exhausted"}
    chain = {r["n"]: r for r in games if r["proof"] == "exhausted"}
    lr = [math.log10(chain[n]["nodes"] / calibration[n]["nodes"]) for n in sorted(calibration)
          if n in chain and min(chain[n]["nodes"], calibration[n]["nodes"]) >= 1000]
    out.append(f"The EC2 chain (64 threads, a window of 16) against the Graviton calibration's chain (64 threads, "
               f"in order, the build of 407645f) on its {len(lr)} searches of 2176-2276: {mean(lr):+.3f} decades "
               f"(se {sd(lr) / math.sqrt(len(lr)):.3f}, sd {sd(lr):.3f}); less the window's {-SPECULATION:.2f}, "
               f"{mean(lr) + SPECULATION:+.3f} for the build.")
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# trend: the gap's growth with n (C5)

def slope_row(label, rows, per=100):
    ns = [r["n"] for r in rows]
    gaps = [r["gap"] for r in rows]
    a, b, fit = line(ns, gaps)

    def stat(idx):
        return line([ns[i] for i in idx], [gaps[i] for i in idx])[1] * per

    short = interval(block_bootstrap(len(ns), stat, block=15))
    long = interval(block_bootstrap(len(ns), stat, block=100)) if len(ns) >= 200 else None
    return [label, len(rows), f"{b * per:.1f}", f"{fit['se_robust'][1] * per:.1f}", f"[{short[0]:.0f}, {short[1]:.0f}]",
            f"[{long[0]:.0f}, {long[1]:.0f}]" if long else "-", f"{fit['r2']:.2f}", f"{fit['sd']:.0f}",
            f"{a + b * ns[0]:.0f}", f"{a + b * ns[-1]:.0f}"]


def trend(games):
    old = baseline()
    header = ["fit of the gap on n", "games", "slope per 100 n", "robust se", "bootstrap 95%, blocks of 15 games",
              "of 100 games", "R²", "sd", "fitted at the first", "at the last"]
    ex = [r for r in games if r["proof"] == "exhausted"]
    rows = [
        slope_row("1301-2175 searched (committed chain)", old),
        slope_row("1301-2150", [r for r in old if r["n"] <= 2150]),
        slope_row(f"{FIRST}-{LAST}, all searched", games),
        slope_row(f"{FIRST}-{LAST}, exhausted", ex),
        slope_row("2176-2275", [r for r in games if r["n"] < 2276]),
        slope_row("2276-3312", [r for r in games if r["n"] >= 2276]),
        slope_row("2176-2800", [r for r in games if r["n"] <= 2800]),
        slope_row("2800-3312", [r for r in games if r["n"] > 2800]),
        slope_row("1301-3312, both", old + games),
    ]
    out = [table(header, rows), ""]
    allr = old + games
    every = sorted(allr, key=lambda r: r["n"])

    def stat_all(idx):
        return line([every[i]["n"] for i in idx], [every[i]["gap"] for i in idx])[1] * 100

    wide = interval(block_bootstrap(len(every), stat_all, block=200))
    out.append(f"The waves of the gap are 100 games and more long, so the blocks of 15 games understate the "
               f"uncertainty; 1301-3312 with blocks of 200 games (about 450 n): [{wide[0]:.0f}, {wide[1]:.0f}] "
               f"per 100 n.")
    out.append("")
    ns = [r["n"] / 1000 for r in allr]
    gaps = [r["gap"] for r in allr]
    quad = ols([[1.0] * len(ns), ns, [x * x for x in ns]], gaps)
    out.append(
        f"Quadratic in n/1000 over 1301-3312: G = {quad['coef'][0]:.0f} {quad['coef'][1]:+.0f} x "
        f"{quad['coef'][2]:+.0f} x², t of the square term {quad['coef'][2] / quad['se_robust'][2]:.1f} (robust), "
        f"R² {quad['r2']:.2f}.")
    logs = ols([[1.0] * len(ns), [math.log(1000 * x) for x in ns]], [math.log(g) for g in gaps])
    out.append(f"Power law over 1301-3312: G ∝ n^{logs['coef'][1]:.2f} ± {logs['se_robust'][1]:.2f}, "
               f"R² {logs['r2']:.2f}.")
    for label, part in (("1301-2175", old), ("2176-3312", games), ("2276-3312", [r for r in games if r["n"] >= 2276]),
                        ("1301-3312", allr)):
        ratio = ols([[1.0] * len(part), [r["n"] for r in part]], [1000 * r["gap"] / r["n"] for r in part])
        average = mean([1000 * r["gap"] / r["n"] for r in part])
        out.append(f"1000 G/n over {label}: {ratio['coef'][0]:.0f} {ratio['coef'][1] * 100:+.1f} per 100 n "
                   f"(robust se {ratio['se_robust'][1] * 100:.1f}), mean {average:.0f}.")
    # the upper envelope: the dear games are the high gaps
    header = ["n", "searched games", "mean G", "80th percentile", "90th percentile", "maximum"]
    env = []
    for lo in range(1301, 3313, 400):
        part = [r["gap"] for r in old + games if lo <= r["n"] < lo + 400]
        env.append([f"{lo}-{lo + 399}", len(part), f"{mean(part):.0f}", f"{quantile(part, 0.8):.0f}",
                    f"{quantile(part, 0.9):.0f}", max(part)])
    out.extend(["", table(header, env)])
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# arithmetic: do the waves of the gap follow the arithmetic of n?

def distinct_primes(limit):
    """For each m up to limit: the number of its distinct prime factors."""
    omega = [0] * (limit + 1)
    for p in range(2, limit + 1):
        if omega[p] == 0:
            for m in range(p, limit + 1, p):
                omega[m] += 1
    return omega


def blocked_fit(rows, features, folds=10):
    """In-sample R² and R² held out a tenth of the games at a time (consecutive) of the gap on the features of n."""
    x = [[1.0] + features(r["n"]) for r in rows]
    y = [r["gap"] for r in rows]
    columns = [[row[j] for row in x] for j in range(len(x[0]))]
    fit = ols(columns, y)
    mean_y = mean(y)
    sse = 0.0
    for k in range(folds):
        test = set(range(k * len(rows) // folds, (k + 1) * len(rows) // folds))
        train = [i for i in range(len(rows)) if i not in test]
        part = ols([[x[i][j] for i in train] for j in range(len(x[0]))], [y[i] for i in train])
        sse += sum((y[i] - sum(c * v for c, v in zip(part["coef"], x[i]))) ** 2 for i in test)
    return fit, 1 - sse / sum((v - mean_y) ** 2 for v in y)


def arithmetic(games):
    """The gap on counts of numbers by tier: tier k holds the numbers m with exactly k multiples on the board
    (n/(k+1) < m <= n/k).  The frames of the board are tiers; the odd cores are made of movers with three or more
    distinct primes."""
    limit = 7100
    omega = distinct_primes(limit)
    primes = [0] * (limit + 1)
    three = [0] * (limit + 1)
    for m in range(1, limit + 1):
        primes[m] = primes[m - 1] + (1 if omega[m] == 1 and all(m % q for q in range(2, int(m ** 0.5) + 1)) else 0)
        three[m] = three[m - 1] + (1 if omega[m] >= 3 else 0)

    def by_tier(prefix, tiers):
        return lambda n: [prefix[n // k] - prefix[n // (k + 1)] for k in tiers]

    families = (("n alone", lambda n: [n / 1000]), ("primes in tiers 1-9", by_tier(primes, range(1, 10))),
                ("numbers of three or more distinct primes in tiers 1-7", by_tier(three, range(1, 8))))
    every = sorted(baseline() + games, key=lambda r: r["n"])
    header = ["features of n"] + [f"{label}: R² in sample, held out" for label in ("1301-3312", "2176-3312")]
    rows = []
    for name, features in families:
        cells = [name]
        for part in (every, [r for r in every if r["n"] >= FIRST]):
            fit, held_out = blocked_fit(part, features)
            cells.append(f"{fit['r2']:.2f}, {held_out:.2f} (sd {fit['sd']:.0f})")
        rows.append(cells)
    return ("The gap of every searched game on counts of numbers by tier (a linear model; held out a tenth of the "
            "games at a time, consecutive):\n\n" + table(header, rows))


# ---------------------------------------------------------------------------------------------------------
# cost: the model log10 nodes = a + b G / n^p refitted and tested out of sample

SPECULATION = -0.04  # decades: a window of 1 or 4 against the chain's 16 (the window table: -0.04 +- 0.03 by game)
ONE_THREAD = -0.02  # decades: one thread's tree against 64 threads' (the ladder: 951 thousand nodes, 1.00 million)
RUN = 0.04  # decades: a run on 64 threads with a window of 4 against one thread in order (0.02 + 0.02 to 0.03)


def pooled(games):
    """Every exhausted search of 1301-3312 as {n, gap, y}: y the log10 of its nodes on the EC2 chain's build, on
    one thread in order: the committed chain's games as that build solved them again, and the EC2 chain's own
    (64 threads, a window of 16) less the speculation and the threads."""
    out = [{"n": r["n"], "gap": r["gap"], "y": math.log10(r["nodes"]), "new": False} for r in exhausted(resolved())]
    out += [{"n": r["n"], "gap": r["gap"], "y": math.log10(r["nodes"]) + SPECULATION + ONE_THREAD, "new": True}
            for r in exhausted(games)]
    return out


def scaled(r, p):
    return r["gap"] / r["n"] ** p


def fit_scaled(rows, p, extra=None):
    columns = [[1.0] * len(rows), [scaled(r, p) for r in rows]]
    if extra:
        columns.append([extra(r) for r in rows])
    return ols(columns, [r["y"] for r in rows])


def fit_cost(rows):
    """log10 nodes on the gap alone, from the games' own nodes: coefficients, c = G per decade, R², sd."""
    fit = ols([[1.0] * len(rows), [r["gap"] for r in rows]], [math.log10(r["nodes"]) for r in rows])
    fit["c"] = 1 / fit["coef"][1]
    fit["c_se_robust"] = fit["c"] ** 2 * fit["se_robust"][1]
    return fit


def cost_row(label, rows):
    fit = fit_cost(rows)
    return [label, len(rows), f"{fit['coef'][0]:.2f} ± {fit['se'][0]:.2f}",
            f"{fit['c']:.0f} ± {fit['c_se_robust']:.0f}", f"{fit['r2']:.2f}", f"{fit['sd']:.2f}"], fit


def cost(games):
    old = exhausted(baseline())
    new = exhausted(games)
    out = ["The gap alone, from the games' own nodes (the EC2 chain's as it ran: 64 threads, a window of 16):", ""]
    header = ["fit of log10 nodes = a + G/c", "searches", "a", "c (gap per decade)", "R²", "residual sd"]
    rows = []
    base_cells, base = cost_row("1301-2175, committed chain (8 threads, the build of feba6d0)", old)
    again_cells, again = cost_row("1301-2175, the same games on the EC2 build (1 thread, in order)",
                                  exhausted(resolved()))
    chain_cells, chain = cost_row("2176-3312, the EC2 chain", new)
    rows += [base_cells, again_cells, chain_cells]
    rows.append([f"2176-3312 on one thread in order (a less {-SPECULATION - ONE_THREAD:.2f})", len(new),
                 f"{chain['coef'][0] + SPECULATION + ONE_THREAD:.2f} ± {chain['se'][0]:.2f}",
                 f"{chain['c']:.0f} ± {chain['c_se_robust']:.0f}", f"{chain['r2']:.2f}", f"{chain['sd']:.2f}"])
    for lo, hi in ((2176, 2400), (2401, 2600), (2601, 3000), (3001, 3312)):
        rows.append(cost_row(f"{lo}-{hi}", [r for r in new if lo <= r["n"] <= hi])[0])
    out += [table(header, rows), ""]

    pool = pooled(games)
    ranges = (("1301-2175", [r for r in pool if not r["new"]]), ("2176-3312", [r for r in pool if r["new"]]),
              ("1301-3312", pool))
    out += ["Which scale for the gap: R² of log10 nodes (all on the EC2 build, in order) on G / n^p, and the "
            "step between the two chains that a model leaves (the coefficient of a 0/1 variable for the EC2 "
            "chain's games added to the pooled fit; a model that holds across 2175 leaves none):", ""]
    header = ["p"] + [f"R² {label} ({len(rows)})" for label, rows in ranges] + ["step at 2176 (se)"]
    rows = []
    dummy = [1.0 if r["new"] else 0.0 for r in pool]
    for p in (0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.8, 1.0, 1.2):
        step = ols([[1.0] * len(pool), [scaled(r, p) for r in pool], dummy], [r["y"] for r in pool])
        rows.append([p] + [f"{fit_scaled(part, p)['r2']:.3f}" for _, part in ranges] +
                    [f"{step['coef'][2]:+.2f} ({step['se_robust'][2]:.2f})"])
    out += [table(header, rows), ""]
    header = ["model, 1301-3312", "R²", "residual sd"]
    rows = []
    for label, columns in (
            ("G", lambda r: [r["gap"]]), ("G and n", lambda r: [r["gap"], r["n"] / 1000]),
            ("G and G/n", lambda r: [r["gap"], 1000 * r["gap"] / r["n"]]), (f"G / n^{P}", lambda r: [scaled(r, P)]),
            ("G / n", lambda r: [scaled(r, 1.0)])):
        cols = list(zip(*[columns(r) for r in pool]))
        fit = ols([[1.0] * len(pool)] + [list(c) for c in cols], [r["y"] for r in pool])
        rows.append([label, f"{fit['r2']:.3f}", f"{fit['sd']:.2f}"])
    out += [table(header, rows), ""]
    for p in (0.0, P):
        for label, part in ranges:
            f = fit_scaled(part, p)
            decade = 1 / f["coef"][1]
            name = f"G / n^{p}" if p else "G"
            out.append(f"{name} on {label}: a = {f['coef'][0]:.3f} ± {f['se'][0]:.3f}, b = {f['coef'][1]:.4g} ± "
                       f"{f['se_robust'][1]:.2g} (robust), residual sd {f['sd']:.2f}; one decade per {decade:.1f} "
                       f"units of {name}, which is {decade * 3000 ** p:.0f} of gap at n = 3000, "
                       f"{decade * 4000 ** p:.0f} at 4000 and {decade * 6000 ** p:.0f} at 6000.")
    for label, part in ranges[1:]:
        s = [scaled(r, P) for r in part]
        ones = [1.0] * len(part)
        y = [r["y"] for r in part]
        square = ols([ones, s, [x * x for x in s]], y)
        with_n = ols([ones, s, [r["n"] / 1000 for r in part]], y)
        out.append(f"Beyond G / n^{P} on {label}: its square adds {square['coef'][2]:.2e} ± "
                   f"{square['se_robust'][2]:.1e} (residual sd {square['sd']:.3f} against "
                   f"{fit_scaled(part, P)['sd']:.3f}); n/1000 adds {with_n['coef'][2]:+.3f} ± "
                   f"{with_n['se_robust'][2]:.3f} decades.  The scaled gap of the games ranges from {min(s):.1f} to "
                   f"{max(s):.1f}.")
    out.append("")

    out += ["Out of sample: a model fitted to the games of 1301..origin, the bias (mean error in decades, the actual "
            "less the predicted) / rmse over the later games, and the worst bias of the five origins:", ""]
    models = [(f"G / n^{p}" if p else "G", lambda r, p=p: [scaled(r, p)]) for p in (0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 1.0)]
    models.append(("G and n", lambda r: [r["gap"], r["n"] / 1000]))
    origins = (2175, 2400, 2600, 2800, 3000)
    header = ["model"] + [f"fitted to 1301..{origin}" for origin in origins] + ["worst bias"]
    rows = []
    for label, columns in models:
        cells, worst = [label], 0.0
        for origin in origins:
            train = [r for r in pool if r["n"] <= origin]
            test = [r for r in pool if r["n"] > origin]
            cols = list(zip(*[columns(r) for r in train]))
            fit = ols([[1.0] * len(train)] + [list(c) for c in cols], [r["y"] for r in train])
            err = [r["y"] - sum(c * v for c, v in zip(fit["coef"], [1.0] + columns(r))) for r in test]
            cells.append(f"{mean(err):+.2f} / {math.sqrt(mean([e * e for e in err])):.2f}")
            worst = max(worst, abs(mean(err)))
        rows.append(cells + [f"{worst:.2f}"])
    out += [table(header, rows), ""]

    a0, c0 = again["coef"][0], again["c"]
    out.append(f"The model of 1301-2175 on the EC2 build (a = {a0:.2f}, c = {c0:.0f}) against the EC2 chain's "
               f"searches (less {-SPECULATION - ONE_THREAD:.2f} for the window and the threads), the mean residual in "
               f"decades:")
    header = ["searches", "n", "mean residual", "se", "sd", "median"]
    rows = []
    for label, lo, hi in (("2176-2276", 2176, 2276), ("2277-2600", 2277, 2600), ("2601-3312", 2601, 3312),
                          ("2176-3312", 2176, 3312)):
        res = [r["y"] - (a0 + r["gap"] / c0) for r in pool if r["new"] and lo <= r["n"] <= hi]
        rows.append([label, len(res), f"{mean(res):+.2f}", f"{sd(res) / math.sqrt(len(res)):.2f}", f"{sd(res):.2f}",
                     f"{median(res):+.2f}"])
    out += ["", table(header, rows)]
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# runs: the structure of the residuals, and the dear games

def sign_runs(e):
    """Wald-Wolfowitz: the runs of signs about the median against what independence gives: runs, expected, z."""
    m = median(e)
    s = [x > m for x in e if x != m]
    n1, n2 = sum(s), len(s) - sum(s)
    runs = 1 + sum(1 for i in range(1, len(s)) if s[i] != s[i - 1])
    mu = 2 * n1 * n2 / (n1 + n2) + 1
    var = (mu - 1) * (mu - 2) / (n1 + n2 - 1)
    return runs, mu, (runs - mu) / math.sqrt(var)


def residual_series(games, p):
    """The exhausted searches of the EC2 chain in order of n, and the model G / n^p fitted to them: rows, fit,
    residuals."""
    rows = [r for r in pooled(games) if r["new"]]
    fit = fit_scaled(rows, p)
    return rows, fit, fit["resid"]


def runs(games):
    out = []
    summary = []
    for p, label in ((0.0, "G alone"), (P, f"G / n^{P}")):
        rows, fit, e = residual_series(games, p)
        r, mu, z = sign_runs(e)
        blocks = {k: sd([mean(e[i:i + k]) for i in range(0, len(e) - k + 1, k)]) ** 2 / (sd(e) ** 2 / k)
                  for k in (5, 20)}
        summary.append([label, len(e), f"{sd(e):.2f}"] + [f"{autocorrelation(e, k):+.2f}" for k in (1, 2, 5, 10, 20)] +
                       [f"{r} of {mu:.0f}", f"{z:+.1f}", f"{blocks[5]:.1f}", f"{blocks[20]:.1f}"])
    out.append(table(["model", "searches", "residual sd", "autocorrelation at lag 1", "2", "5", "10", "20",
                      "runs of signs, expected", "z", "variance of 5-block means over independence's",
                      "20-block means"], summary))
    out.append("")
    rows, fit, e = residual_series(games, P)
    header = ["games", "searches", "mean G", "mean residual", "sd of the residual", "largest residual (n)"]
    table_rows = []
    for lo in range(FIRST, LAST + 1, 150):
        part = [(r["n"], x) for r, x in zip(rows, e) if lo <= r["n"] < lo + 150]
        gs = [r["gap"] for r in rows if lo <= r["n"] < lo + 150]
        if part:
            top = max(part, key=lambda t: t[1])
            table_rows.append([f"{lo}-{lo + 149}", len(part), f"{mean(gs):.0f}", f"{mean([x for _, x in part]):+.2f}",
                               f"{sd([x for _, x in part]):.2f}", f"{top[1]:+.2f} ({top[0]})"])
    out += [f"The residuals of the model G / n^{P} (a = {fit['coef'][0]:.3f}, b = {fit['coef'][1]:.4f}) by block of "
            "n:", "", table(header, table_rows), ""]
    # the dear games, by the model G / n^P and by G alone
    _, fit0, e0 = residual_series(games, 0.0)
    by_n = {r["n"]: (x, x0) for r, x, x0 in zip(rows, e, e0)}
    nodes = {r["n"]: r for r in exhausted(games)}
    dear = sorted(nodes.values(), key=lambda r: -r["nodes"])[:18]
    header = ["n", "nodes (millions)", "gap", "start below opt", "model G / n^p: nodes (millions)", "residual",
              "model G: nodes (millions)", "residual"]
    table_rows = []
    for r in sorted(dear, key=lambda r: r["n"]):
        x, x0 = by_n[r["n"]]
        y = math.log10(r["nodes"]) + SPECULATION + ONE_THREAD
        table_rows.append([r["n"], f"{r['nodes'] / 1e6:.1f}", r["gap"], r["opt"] - r["start"],
                           f"{10 ** (y - x) / 1e6:.2f}", f"{x:+.2f}", f"{10 ** (y - x0) / 1e6:.2f}", f"{x0:+.2f}"])
    out += ["The 18 dearest searches, and what the models predict for them (the models' nodes are for one thread in "
            "order, the game's own are the chain's, and the residuals are against the models):", "",
            table(header, table_rows)]
    return "\n".join(out)


# ---------------------------------------------------------------------------------------------------------
# anatomy: what at the root explains the residuals (A4)

def root_features(r, anatomy):
    """The root's features of one game: sizes from the data file, conflicts and cores from the anatomy file."""
    f = {
        "candidates": r["candidates"], "max_total": r["max_total"], "root_chosen": r["root_chosen"],
        "forced at the root": r["root_forced"], "start below the optimum": r["opt"] - r["start"],
        "freedoms of frames 0-2": sum(r["frame_freedoms"][:3]), "freedoms, all frames": sum(r["frame_freedoms"]),
        "n": r["n"],
    }
    a = anatomy.get(r["n"])
    if a:
        sizes = a["conflicts"]
        cores = a["cores"]
        f.update({
            "conflicts in the heaviest set": len(sizes),
            "members in the first conflict": sizes[0] if sizes else 0,
            "mean conflict size": mean(sizes) if sizes else 0,
            "largest conflict": max(sizes) if sizes else 0,
            "members in all conflicts": sum(sizes),
            "frames holding a core": len(cores),
            "movers in cores": sum(c["movers"] for c in cores),
            "promoted in cores": sum(c["promoted"] for c in cores),
            "wide movers in cores": sum(c["wide"] for c in cores),
            "factors in cores": sum(c["factors"] for c in cores),
            "movers in the cores of frames 1-3": sum(c["movers"] for c in cores if 1 <= c["frame"] <= 3),
        })
    return f


def anatomy(games):
    anatomy_rows = {a["n"]: a for a in read_jsonl(ANATOMY)} if ANATOMY.exists() else {}
    new = exhausted(games)
    names = list(root_features(new[0], anatomy_rows))
    columns = {name: [] for name in names}
    for r in new:
        f = root_features(r, anatomy_rows)
        for name in names:
            columns[name].append(f[name])
    rows, fit, e = residual_series(games, P)
    _, fit0, e0 = residual_series(games, 0.0)
    out = [f"{len(new)} exhausted searches with the anatomy.  'Share' is the R² of a residual on the feature: the "
           f"share of the residual's variance that it explains, for the residual of the model on the gap alone (R² "
           f"{fit0['r2']:.2f}, sd {fit0['sd']:.2f}) and of the model G / n^{P} (R² {fit['r2']:.2f}, sd "
           f"{fit['sd']:.2f}); 'with G' is the R² of log10 nodes on G and the feature, 'x G' the same with the "
           "feature times G (the cost per unit of gap) as well.", ""]
    header = ["feature", "mean", "sd", "share of the residual of G", "share of the residual of G / n^p", "R² with G",
              "R² with G and x G", "t of the feature with G"]
    table_rows = []
    y = [r["y"] for r in rows]
    gap = [r["gap"] for r in rows]
    ones = [1.0] * len(rows)
    for name in names:
        x = columns[name]
        if sd(x) == 0:
            continue
        with_g = ols([ones, gap, x], y)
        inter = ols([ones, gap, x, [a * b for a, b in zip(gap, x)]], y)
        table_rows.append([name, f"{mean(x):.1f}", f"{sd(x):.1f}", f"{correlation(x, e0) ** 2:.2f}",
                           f"{correlation(x, e) ** 2:.2f}", f"{with_g['r2']:.2f}", f"{inter['r2']:.2f}",
                           f"{with_g['coef'][2] / with_g['se_robust'][2]:+.1f}"])
    table_rows.sort(key=lambda row: -float(row[3]))
    out.append(table(header, table_rows))
    # the multivariate: the residual on all the anatomy features at once
    keep = [columns[name] for name in names if sd(columns[name]) > 0 and name != "n"]
    out.append("")
    out.append(f"All {len(keep)} features at once (without n) explain, in sample, "
               f"{ols([ones] + keep, e)['r2']:.2f} of the residual of G / n^{P} and "
               f"{ols([ones] + keep, e0)['r2']:.2f} of the residual of G alone; held out, a stretch of 40 consecutive "
               f"searches at a time (the features fitted to the others), {blocked_r2(keep, e):.2f} and "
               f"{blocked_r2(keep, e0):.2f}.")
    return "\n".join(out)


def blocked_r2(features, y, folds=10):
    """R² of y predicted from the features by least squares, each fold of consecutive games predicted from the rest."""
    n = len(y)
    mean_y = mean(y)
    sse = 0.0
    for k in range(folds):
        test = set(range(k * n // folds, (k + 1) * n // folds))
        train = [i for i in range(n) if i not in test]
        fit = ols([[1.0] * len(train)] + [[col[i] for i in train] for col in features], [y[i] for i in train])
        for i in test:
            predicted = fit["coef"][0] + sum(c * col[i] for c, col in zip(fit["coef"][1:], features))
            sse += (y[i] - predicted) ** 2
    return 1 - sse / sum((v - mean_y) ** 2 for v in y)


# ---------------------------------------------------------------------------------------------------------
# projection: a game at 4000, 5000 and 6000, and what 24 hours reach

CHAIN_HOURS = 6.4  # wall time of 2176..3314 on 64 threads, as reported with the snapshot
THREADS = 64
NODE_COST_GROWTH = 0.02  # decades per 100 n of the cost of a node: the calibration below reproduces the ladder and 4000
CHAIN_GAMES = 1137  # games 2176..3312 of the snapshot
DRAWS = 300
HORIZON = 720  # hours: a simulated chain stops here
BLOCK = 100  # consecutive searches in a resampled stretch (about 280 n, the length of a wave)
SCENARIOS = (("trend", "the linear fit of 2176-3312"), ("ratio", "G proportional to n"),
             ("steep", "+57 per 100 n from the fit at 3312"))


def node_cost(games):
    """Microseconds of one thread a node at n = 3000 (u0), such that the chain's nodes fill its hours on 64 threads."""
    # (the nodes of every searched game of the data file, and of the games it lacks: negligible)
    weighted = sum(r["nodes"] * 10 ** (NODE_COST_GROWTH * (r["n"] - 3000) / 100) for r in games)
    return CHAIN_HOURS * 3600 * THREADS * 1e6 / weighted


def setup_fit():
    """Seconds of one thread for a game's serial work besides its search, by n: the board, moves and replays of every
    game and half the unseeded local search of a searched game (the chain seeds it): log-linear in n."""
    rows = [r for r in read_jsonl(SETUP) if not r["prime"]]
    ns = [r["n"] for r in rows]
    common = [math.log10(r["board_s"] + r["moves_s"] + r["replay_s"]) for r in rows]
    search = [math.log10(r["zero_search_s"]) for r in rows]
    return line(ns, common), line(ns, search)


class Model:
    """One draw of the projection: a stretch of the observed searches resampled in blocks of consecutive ones, the
    cost model G / n^p fitted to it, and from it the waves of the gap (its distance from the data's linear trend, in
    order) and the cost model's residuals, which the projected games walk through in order; the gap's trend is the
    scenario's."""

    def __init__(self, new, scenario, p, rng):
        trend_a, trend_b, _ = line([r["n"] for r in new], [r["gap"] for r in new])
        idx = block_indices(len(new), BLOCK, rng)
        part = [new[i] for i in idx]
        self.p = p
        self.waves = [r["gap"] - (trend_a + trend_b * r["n"]) for r in part]
        cost_fit = ols([[1.0] * len(part), [r["gap"] / r["n"] ** p for r in part]],
                       [math.log10(r["nodes"]) + SPECULATION + ONE_THREAD for r in part])
        self.a, self.b = cost_fit["coef"]
        self.e = cost_fit["resid"]
        self.pos = rng.randrange(len(part))
        rho = mean([r["gap"] / r["n"] for r in new])
        at_last = trend_a + trend_b * LAST
        self.gap = {"trend": lambda n: trend_a + trend_b * n, "ratio": lambda n: rho * n,
                    "steep": lambda n: at_last + 0.57 * (n - LAST)}[scenario]

    def nodes_at(self, n, i):
        """Nodes of an exhausted search at n with the i-th wave and residual, on 64 threads with a window of 4."""
        gap = max(0.0, self.gap(n) + self.waves[i])
        return 10 ** (self.a + self.b * gap / n ** self.p + self.e[i] + RUN)

    def search_nodes(self, n):
        """Nodes of the next exhausted search at n: the next wave and residual of the stretch, in order."""
        nodes = self.nodes_at(n, self.pos)
        self.pos = (self.pos + 1) % len(self.e)
        return nodes


def simulate(model, share, u0, setup, until_hours, first=LAST + 1, last=9000):
    """Seconds on the 64 threads at the end of each game from `first` on, to `until_hours` or board `last`."""
    rng = random.Random(model.pos)
    (ca, cb, _), (sa, sb, _) = setup
    total = 0.0
    ends = {}
    n = first
    while total < until_hours * 3600 and n <= last:
        seconds = 10 ** (ca + cb * n)
        if rng.random() < share:
            seconds += 10 ** (sa + sb * n) / 2
            seconds += model.search_nodes(n) * u0 * 10 ** (NODE_COST_GROWTH * (n - 3000) / 100) / (THREADS * 1e6)
        total += seconds
        ends[n] = total
        n += 1
    return ends


def projection(games):
    new = exhausted(games)
    share = len(new) / CHAIN_GAMES
    u0 = node_cost(games)
    setup = setup_fit()
    nodes_all = sum(r["nodes"] for r in games) / 1e9
    out = [f"The chain's searches: {len(new)} exhausted of {CHAIN_GAMES} games ({share:.2f}), {nodes_all:.2f} billion "
           f"nodes in all, in {CHAIN_HOURS} hours on {THREADS} threads.  With a node costing u(n) = u0 x "
           f"10^({NODE_COST_GROWTH} (n - 3000) / 100), those hours fix u0 = {u0:.0f} thread-microseconds, so u(2184) = "
           f"{u0 * 10 ** (NODE_COST_GROWTH * (2184 - 3000) / 100):.0f} (the ladder's proof of 2184 on 64 threads: "
           f"166 µs a step, plus the window's idle time) and u(4000) = {u0 * 10 ** (NODE_COST_GROWTH * 10):.0f} (the "
           f"standalone search of 4000 on 192 threads: 417).  The serial work besides the search (the board, moves and "
           f"replays of every game, and half an unseeded local search for a searched one) is "
           f"{10 ** (setup[0][0] + setup[0][1] * 4000):.2f} + {share:.2f} x "
           f"{10 ** (setup[1][0] + setup[1][1] * 4000) / 2:.2f} s a game at 4000, whatever the threads.", ""]
    header = ["games", "exhausted searches", "nodes (millions)", "hours of the 64 threads in the search"]
    rows = []
    for lo in range(FIRST - 1, LAST, 100):
        part = [r for r in games if lo < r["n"] <= lo + 100]
        thread_us = sum(r["nodes"] * u0 * 10 ** (NODE_COST_GROWTH * (r["n"] - 3000) / 100) for r in part)
        hours = thread_us / (THREADS * 3.6e9)
        rows.append([f"{lo + 1}-{lo + 100}", len(exhausted(part)), f"{sum(r['nodes'] for r in part) / 1e6:.0f}",
                     f"{hours:.2f}"])
    out += ["The chain's own hundreds of games (the nodes it made, at that cost of a node):", "",
            table(header, rows), ""]
    sizes = (4000, 5000, 6000)
    combos = [(P, "trend"), (P, "ratio"), (P, "steep"), (0.0, "trend"), (0.0, "ratio"), (0.0, "steep"),
              (0.5, "trend"), (0.5, "ratio")]
    nodes_rows, pace_rows, reach_rows, validation = [], [], [], []
    for p, scenario in combos:
        rng = random.Random(7)
        models = [Model(new, scenario, p, rng) for _ in range(DRAWS)]
        label = ("the gap alone" if p == 0 else f"G / n^{p}") + f", {scenario}"
        gaps = {n: median([m.gap(n) for m in models]) for n in sizes}
        nodes = {n: [m.nodes_at(n, i) for m in models for i in range(len(m.e))] for n in sizes}
        nodes_rows.append([label] + [f"{gaps[n]:.0f}" for n in sizes] +
                          [span([v / 1e6 for v in nodes[n]], 1) for n in sizes])
        per100 = {n: [] for n in sizes}
        to = {n: [] for n in sizes}
        reach24, reach_from_start, over_the_chain = [], [], []
        for m in models:
            ends = simulate(m, share, u0, setup, HORIZON)
            for n in sizes:
                if n + 100 in ends:
                    per100[n].append((ends[n + 100] - ends[n]) / 3600)
                to[n].append(ends[n] / 3600 if n in ends else float("inf"))
            reach24.append(max([k for k, t in ends.items() if t <= 24 * 3600] or [LAST]))
            reach_from_start.append(max([k for k, t in ends.items() if t <= (24 - CHAIN_HOURS) * 3600] or [LAST]))
            if scenario != "steep":  # its trend starts at the last game
                m.pos = random.Random(len(over_the_chain)).randrange(len(m.e))
                over_the_chain.append(simulate(m, share, u0, setup, HORIZON, first=FIRST, last=LAST)[LAST] / 3600)
        pace_rows.append([label] + [f"{mean(per100[n]):.2f}; {span(per100[n], 2)}" if len(per100[n]) >= 0.8 * DRAWS
                                    else "past the horizon" for n in sizes])
        reach_rows.append([label] + [span(to[n], 0, HORIZON) for n in sizes] +
                          [span(reach_from_start, 0), span(reach24, 0)])
        if over_the_chain:
            validation.append([label, span(over_the_chain, 1)])
    out += ["A searched game's nodes at n, on 64 threads with a window of 4 (millions; the median over the draws, then "
            "the median and the 10th and 90th percentile over all the draws' waves and residuals):", "",
            table(["model, gap scenario", "gap at 4000", "5000", "6000", "nodes at 4000", "5000", "6000"],
                  nodes_rows), "",
            "Hours of the 64 threads for 100 consecutive games (the mean over the draws and starting points; the "
            "median [10th, 90th percentile]):", "",
            table(["model, gap scenario", "at 4000", "5000", "6000"], pace_rows), "",
            f"Hours from 3313 to the board (median [10th, 90th percentile]; more than {HORIZON} hours is shown as "
            f">{HORIZON}), and the board reached in 24 hours of the whole chain from 2176 (the 17.6 hours past 3313 "
            "that remain) and in 24 hours past 3313:", "",
            table(["model, gap scenario", "hours to 4000", "5000", "6000", "reached, 24 hours from 2176",
                   "reached, 24 hours past 3313"], reach_rows), "",
            f"The same models over the chain's own games, 2176..{LAST} (hours; the chain took {CHAIN_HOURS}, which "
            "fixes the cost of a node):", "",
            table(["model, gap scenario", "hours, median [10th, 90th percentile]"], validation)]
    return "\n".join(out)


def span(values, digits=1, cap=None):
    """The median and the 10th and 90th percentiles of the values, as text; a value of `cap` or more as >cap."""
    def text(v):
        return f">{cap}" if cap is not None and v >= cap else f"{v:.{digits}f}"

    q = [quantile([min(v, 1e9) for v in values], x) for x in (0.5, 0.1, 0.9)]
    return f"{text(q[0])} [{text(q[1])}, {text(q[2])}]"


# ---------------------------------------------------------------------------------------------------------

SECTIONS = {
    "bands": ("The root gaps by band", bands),
    "trend": ("The gap's growth with n", trend),
    "arithmetic": ("Do the waves of the gap follow the arithmetic of n?", arithmetic),
    "window": ("The speculation and the threads in a game's nodes", window),
    "build": ("The committed chain's build against the EC2 chain's", build),
    "cost": ("The cost model refitted", cost),
    "runs": ("The structure of the residuals, and the dear games", runs),
    "anatomy": ("What at the root explains the residuals", anatomy),
    "projection": ("A game at 4000, 5000 and 6000, and what 24 hours reach", projection),
}


def main(argv):
    if argv and argv[0] == "games":
        return games_of(argv[1])
    if argv and argv[0] == "join":
        return join(argv[1], argv[2])
    if argv and argv[0] == "join-anatomy":
        return join_anatomy(argv[1], argv[2], argv[3])
    if argv and argv[0] == "join-resolved":
        return join_resolved(argv[1])
    names = argv or list(SECTIONS)
    unknown = [name for name in names if name not in SECTIONS]
    if unknown:
        sys.exit(f"unknown table {unknown[0]}: {', '.join(SECTIONS)}")
    games = load_games()
    for name in names:
        title, function = SECTIONS[name]
        print(f"### {title}\n")
        print(function(games))
        print()


if __name__ == "__main__":
    main(sys.argv[1:])
