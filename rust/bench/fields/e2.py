#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of E2, the multi-field bound at the roots past 2175:
rust/docs/experiments/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10.md, section "E2".

Reads, next to this script unless an option says otherwise: roots.jsonl (E1: arm f1cores at 66 roots of 900-2175),
roots_e2.jsonl (E2: arms gf2, f1cores and f1cores16 certified at the roots past 2175), search_e1roots.jsonl (search
mode, 60 s, at E1's converged roots) and budget60_e1roots.jsonl, budget60_e2roots.jsonl (the main arm certified with a
budget of 60 s at E1's converged roots and at E2's), all written by `examples/fields.rs`; the share, the convergence
rule (a bracket within 1% of the root's GF(2) gap) and the trend are `fields.py`'s.  The criteria are the advisor's,
fixed in advance (design.md, S4, E2): killed if the mean share over the roots from 2200 on that converge is below 0.25,
or the slope over 900 to the last root is -2 points per 100 n or worse with t <= -2; if more than 30% of the roots from
2200 on do not converge, the mean over all of them decides (their shares are lower bounds on their LPs'); the limit of
16 core-derived signings a frame is adopted if it gains 0.05 of the gap or more at the dear roots; search mode serves if
it reaches E1's certified bound within 5% of the gap at 80% of E1's converged roots.  `./e2.py > e2_output.md`.
"""
import argparse
import statistics as st
import sys
from pathlib import Path

from fields import HERE, MAIN, converged, fmt, is_open, load, mean, median, of, pct, ratio, share, table, trend

PAST = 2200  # the roots of E2 start here
E2_UNCONVERGED_MAX = 0.30  # more unconverged than this: decide on all the roots
E2_KILL_MEAN = 0.25
E2_KILL_SLOPE, E2_KILL_T = -2.0, -2.0
CAP_GAIN = 0.05  # the limit of 16 is adopted at a gain of this share of the gap at the dear roots
SEARCH_WITHIN, SEARCH_AT_LEAST = 0.05, 0.80
CAP16 = "f1cores16"
DEAR = 5  # the searched games with the largest gaps, from the chain's results (see the README)


def by_key(rows):
    return {(r["n"], r["arm"]): r for r in rows}


def gap(r):
    return r["gf2"] - r["opt"]


def mean_or_dash(values, digits=3):
    return fmt(mean(values), digits)


def slope_cells(rows):
    tr = trend(rows)
    return ["-", "-"] if tr is None else [f"{tr.slope:+.2f} ± {tr.se:.2f}", f"{tr.t:+.1f}"]


def per_root(e2, e1_search):
    print("## 1. The roots past 2175\n")
    print("Certified, `f1cores`, 600 s a root.  Gap: GF(2) bound less P*.  Share: the fraction of the gap the bound "
          "closes.  `*`: the bracket did not close to 1% of the gap (the share is a lower bound on the LP's).  "
          "Bracket: the final family's lowest L less its certified lower bound (`-`: none was certified in the "
          "budget).  L - master: that lowest L less the master's value, an "
          "estimate of the LP that is not certified.  Matroids: extra matroids per frame (frame 0 first).\n")
    main = sorted((r for r in e2 if r["arm"] == MAIN and r["mode"] == "certified"), key=lambda r: r["n"])
    out = []
    for r in main:
        cell = fmt(share(r)) + ("" if converged(r) else "*") if is_open(r) else "-"
        by_master = fmt((r["gf2"] - r["master"]) / gap(r)) if is_open(r) else "-"
        bracket = fmt(r["bracket"], 1) if r["lb"] > 0 else "-"
        out.append([r["n"], gap(r), r.get("max_rank"), r["bound"], cell, bracket,
                    fmt(r["l_best"] - r["master"], 1), by_master, r["stop"], f"{r['ms'] / 1e3:.0f}", r["rounds"],
                    r["pricings_mi"], f"{r['cores_assigned']}/{r['cores_found']}",
                    "".join(f"{m} " for m in r["matroids"]).strip().replace(" ", ",")])
    table(["n", "gap", "max rank", "bound", "share", "bracket", "L - master", "share by master", "stop", "s", "rounds",
           "pricings", "cores assigned/found", "matroids"], out)
    return main


def criteria(e1, main):
    print("## 2. The kill criteria\n")
    past = [r for r in main if r["n"] >= PAST and is_open(r)]
    done = [r for r in past if converged(r)]
    unconverged = ratio(len(past) - len(done), len(past))
    print(f"* roots from {PAST} on with a defined share: {len(past)}, converged within 1% of the gap: {len(done)} "
          f"({pct(ratio(len(done), len(past)))}); unconverged {pct(unconverged)} (the rule changes above "
          f"{100 * E2_UNCONVERGED_MAX:.0f}%)")
    m_done, m_all = mean(share(r) for r in done), mean(share(r) for r in past)
    print(f"* K1: mean share over the converged roots from {PAST}: {fmt(m_done)}; over all of them (lower bounds on "
          f"their LPs'): {fmt(m_all)} (killed below {E2_KILL_MEAN})")
    e1_main = [r for r in e1 if r["arm"] == MAIN and is_open(r)]
    sets = {
        f"converged roots, 900-last (E1's {sum(converged(r) for r in e1_main)} and E2's {len(done)})":
            [r for r in e1_main if converged(r)] + done,
        f"all roots, 900-last (E1's {len(e1_main)} and E2's {len(past)})": e1_main + past,
        f"converged roots from {PAST}": done,
        f"all roots from {PAST}": past,
        "E1's converged roots from 1700 (E1's K2)": [r for r in e1_main if converged(r) and r["n"] >= 1700],
    }
    out = []
    verdict_slope = None
    for name, rows in sets.items():
        tr = trend(rows)
        out.append([name, len(rows), fmt(mean(share(r) for r in rows))] + slope_cells(rows))
        if verdict_slope is None:
            verdict_slope = tr
    print("\nSlope of the share against n (points per 100 n):\n")
    table(["roots", "n roots", "mean share", "slope ± se", "t"], out)
    decide_on_all = unconverged is not None and unconverged > E2_UNCONVERGED_MAX
    mean_used = m_all if decide_on_all else m_done
    tr = trend(list(sets.values())[1 if decide_on_all else 0])
    k1 = mean_used is not None and mean_used < E2_KILL_MEAN
    k2 = tr is not None and tr.slope <= E2_KILL_SLOPE and tr.t <= E2_KILL_T
    basis = "all roots (more than 30% unconverged)" if decide_on_all else "the converged roots"
    hit = " and ".join(name for name, h in (("K1", k1), ("K2", k2)) if h)
    print(f"**Verdict** (on {basis}): mean share {fmt(mean_used)}, slope "
          f"{'-' if tr is None else f'{tr.slope:+.2f} ± {tr.se:.2f} (t {tr.t:+.1f})'}: "
          f"{'killed by ' + hit if hit else 'not killed'}\n")


def bands(main):
    print("## 3. By band of 200 n\n")
    groups = {}
    for r in main:
        if is_open(r):
            groups.setdefault(r["n"] // 200 * 200, []).append(r)
    out = []
    for lo in sorted(groups):
        rs = groups[lo]
        out.append([f"{lo}-{lo + 199}", len(rs), sum(converged(r) for r in rs), mean_or_dash(share(r) for r in rs),
                    mean_or_dash(share(r) for r in rs if converged(r)), fmt(mean(gap(r) for r in rs), 0),
                    fmt(mean(r["ms"] / 1e3 for r in rs), 0), fmt(mean(r["pricings_mi"] for r in rs), 0)])
    table(["n", "roots", "converged", "mean share, all", "mean share, converged", "mean gap", "mean s", "pricings"],
          out)


GAP_BANDS = [(0, 200), (200, 400), (400, 600), (600, 800), (800, 1000), (1000, 1250), (1250, 1500)]


def ols(rows, columns):
    """Least squares of the share on `columns` (functions of a row) and a constant: (coefficients, standard errors)."""
    x = [[1.0] + [c(r) for c in columns] for r in rows]
    y = [share(r) for r in rows]
    k = len(x[0])
    xtx = [[sum(row[i] * row[j] for row in x) for j in range(k)] for i in range(k)]
    inv = [[float(i == j) for j in range(k)] for i in range(k)]
    a = [xtx[i][:] + inv[i] for i in range(k)]
    for i in range(k):  # Gauss-Jordan
        p = max(range(i, k), key=lambda r: abs(a[r][i]))
        a[i], a[p] = a[p], a[i]
        a[i] = [v / a[i][i] for v in a[i]]
        for r in range(k):
            if r != i:
                f = a[r][i]
                a[r] = [v - f * w for v, w in zip(a[r], a[i])]
    inv = [row[k:] for row in a]
    beta = [sum(inv[i][j] * sum(row[j] * yy for row, yy in zip(x, y)) for j in range(k)) for i in range(k)]
    resid = [yy - sum(b * v for b, v in zip(beta, row)) for row, yy in zip(x, y)]
    sigma2 = sum(e * e for e in resid) / (len(y) - k)
    return beta, [(sigma2 * inv[i][i]) ** 0.5 for i in range(k)]


def by_gap(e1, main):
    print("## 4. The share against the gap (after the fact, not one of the design's criteria)\n")
    rows = [r for r in e1 if r["arm"] == MAIN and is_open(r)] + [r for r in main if is_open(r)]
    print(f"`{MAIN}` at E1's and E2's roots together ({len(rows)}, converged or not: the shares are valid bounds), by "
          "the root's GF(2) gap.\n")
    out = []
    for lo, hi in GAP_BANDS:
        rs = [r for r in rows if lo <= gap(r) < hi]
        if rs:
            out.append([f"{lo}-{hi - 1}", len(rs), fmt(mean(share(r) for r in rs)), fmt(median(share(r) for r in rs)),
                        f"{min(r['n'] for r in rs)}-{max(r['n'] for r in rs)}"])
    table(["gap", "roots", "mean share", "median share", "n"], out)
    beta, se = ols(rows, [lambda r: r["n"] / 100, lambda r: gap(r) / 100])
    print("Least squares, share = a + b (n / 100) + c (gap / 100) over those roots: "
          f"a = {beta[0]:.3f} ± {se[0]:.3f}, b = {beta[1]:+.4f} ± {se[1]:.4f} (n), "
          f"c = {beta[2]:+.4f} ± {se[2]:.4f} (gap); the standard errors take the roots as independent, which they "
          "are not quite (neighbouring games share their structure).\n")


def cap(e2, dear):
    print("## 5. The limit of 16 core-derived signings a frame against 8\n")
    pairs = {}
    for r in e2:
        if r["arm"] in (MAIN, CAP16) and r["mode"] == "certified":
            pairs.setdefault(r["n"], {})[r["arm"]] = r
    both = {n: d for n, d in sorted(pairs.items()) if MAIN in d and CAP16 in d and is_open(d[MAIN])}
    if not both:
        print("No root has both arms yet.\n")
        return
    out, gains, dear_gains = [], [], []
    for n, d in both.items():
        a, b = d[MAIN], d[CAP16]
        g = (a["bound"] - b["bound"]) / gap(a)
        gains.append(g)
        if n in dear:
            dear_gains.append(g)
        out.append([n, gap(a), a["bound"], b["bound"], fmt(share(a)) + ("" if converged(a) else "*"),
                    fmt(share(b)) + ("" if converged(b) else "*"), fmt(g), "dear" if n in dear else "",
                    f"{a['ms'] / 1e3:.0f} / {b['ms'] / 1e3:.0f}", f"{a['cores_found']} / {b['cores_found']}",
                    f"{a['cores_assigned']} / {b['cores_assigned']}"])
    table(["n", "gap", "bound, 8", "bound, 16", "share, 8", "share, 16", "gain (share of the gap)", "", "s",
           "cores found", "assigned"], out)
    print(f"Mean gain of the limit of 16 over 8: {fmt(mean(gains))} of the gap over the {len(gains)} roots; at the "
          f"{len(dear_gains)} dear roots {fmt(mean(dear_gains))} (adopted at {CAP_GAIN} or more at the dear roots): "
          f"{'adopt 16' if dear_gains and mean(dear_gains) >= CAP_GAIN else 'keep 8'}.  Roots where 16 is worse: "
          f"{sum(g < 0 for g in gains)}.\n")


def deficits(e1_main, rows):
    """(run, E1's run, deficit) for each of `rows` (runs at E1's roots) at a root where E1 converged: the deficit is
    the bound the run reached less E1's, as a share of the root's gap."""
    out = []
    for r in sorted(rows, key=lambda r: r["n"]):
        base = e1_main.get(r["n"])
        if base is not None and is_open(base) and converged(base):
            out.append((r, base, (r["bound"] - base["bound"]) / gap(base)))
    return out


def summary_line(name, found):
    within = sum(d <= SEARCH_WITHIN for _, _, d in found)
    ds = [d for _, _, d in found]
    frac = ratio(within, len(found))
    print(f"* {name}: within {int(100 * SEARCH_WITHIN)}% of the gap of E1's certified bound at "
          f"{of(within, len(found))} of E1's converged roots; the bound is above it by {fmt(mean(ds))} of the gap on "
          f"average, median {fmt(median(ds))}, worst {fmt(max(ds, default=None))}; "
          f"{int(100 * SEARCH_AT_LEAST)}% needed: {'yes' if frac is not None and frac >= SEARCH_AT_LEAST else 'no'}")


def search(e1, found, budget):
    print("## 6. Search mode, and a short certified budget, against E1's certified bounds\n")
    if not found and not budget:
        print("No search-mode roots yet.\n")
        return
    e1_main = {r["n"]: r for r in e1 if r["arm"] == MAIN}
    s_rows = deficits(e1_main, [r for r in found if r["mode"] == "search"])
    b_rows = deficits(e1_main, [r for r in budget if r["mode"] == "certified"])
    b_by = {r["n"]: (r, d) for r, _, d in b_rows}
    print("Bounds at E1's converged roots (E1: certified, 120 s budget, run to convergence; search: no exact "
          "checks, 60 s; 60 s: certified with a budget of 60 s).  Deficit: the bound less E1's, as a share of the "
          "root's gap.\n")
    out = []
    for r, base, d in s_rows:
        b = b_by.get(r["n"])
        out.append([r["n"], gap(base), base["bound"], r["bound"], fmt(d), b[0]["bound"] if b else "-",
                    fmt(b[1]) if b else "-", f"{base['ms'] / 1e3:.0f}", f"{r['ms'] / 1e3:.1f}",
                    f"{b[0]['ms'] / 1e3:.0f}" if b else "-", r["pricings_mi"], base["pricings_mi"]])
    table(["n", "gap", "bound, E1", "bound, search", "deficit, search", "bound, 60 s", "deficit, 60 s", "s, E1",
           "s, search", "s, 60 s", "pricings, search", "pricings, E1"], out)
    if s_rows:
        summary_line("search mode (60 s budget, ends when the separation finds no new core)", s_rows)
    if b_rows:
        summary_line("certified, 60 s budget", b_rows)
    for label, rows in (("search", s_rows), ("60 s", b_rows)):
        if not rows:
            continue
        groups = {}
        for r, base, d in rows:
            groups.setdefault(r["n"] // 500 * 500, []).append(d)
        cells = ", ".join(f"{lo}-{lo + 499}: {fmt(mean(v))}" for lo, v in sorted(groups.items()))
        print(f"* mean deficit by band of 500 n, {label}: {cells}")
    print()


def frontier_budget(main, budget):
    print("## 7. A budget of 60 s against 600 s at the roots past 2175\n")
    short = {r["n"]: r for r in budget if r["arm"] == MAIN and r["mode"] == "certified"}
    if not short:
        print("No 60 s runs at these roots yet.\n")
        return
    out, ds = [], []
    for r in main:
        s = short.get(r["n"])
        if s is None or not is_open(r):
            continue
        d = (s["bound"] - r["bound"]) / gap(r)
        ds.append(d)
        out.append([r["n"], gap(r), r["bound"], s["bound"], fmt(d), fmt(share(r)) + ("" if converged(r) else "*"),
                    fmt(share(s)) + ("" if converged(s) else "*"), f"{r['ms'] / 1e3:.0f}", f"{s['ms'] / 1e3:.0f}"])
    table(["n", "gap", "bound, 600 s", "bound, 60 s", "deficit (share of gap)", "share, 600 s", "share, 60 s",
           "s, 600 s", "s, 60 s"], out)
    within = sum(d <= SEARCH_WITHIN for d in ds)
    print(f"With 60 s the bound is above the 600 s run's by {fmt(mean(ds))} of the gap on average (median "
          f"{fmt(median(ds))}, worst {fmt(max(ds))}); within {int(100 * SEARCH_WITHIN)}% of the gap at "
          f"{of(within, len(ds))} of the roots.\n")


def signings(main):
    print("## 8. Signings and cost at the roots past 2175\n")
    out = []
    for r in main:
        frames_with = [h for h, m in enumerate(r["matroids"]) if m > 0]
        per = [m - (1 if h in (1, 2, 3) else 0) for h, m in enumerate(r["matroids"])]
        out.append([r["n"], ",".join(f"{h}:{m}" for h, m in enumerate(r["matroids"]) if m), max(per, default=0),
                    r["cores_found"], r["cores_assigned"], r["cores_skipped"], r["columns"], r["cuts"],
                    f"{r['ms_mi'] / max(r['pricings_mi'], 1):.2f}", f"{r['ms_simplex'] / 1e3:.0f}",
                    f"{r['ms'] / 1e3:.0f}", r["checks_exact"], r["pivots_exact"]])
    table(["n", "matroids (frame:count)", "most core-derived on a frame", "cores found", "assigned", "unplaced",
           "columns", "cuts", "ms per pricing (MI)", "s simplex", "s total", "exact checks", "pivots, exact"], out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    parser.add_argument("--e1", type=Path, default=HERE / "roots.jsonl")
    parser.add_argument("--e2", type=Path, default=HERE / "roots_e2.jsonl")
    parser.add_argument("--search", type=Path, default=HERE / "search_e1roots.jsonl")
    parser.add_argument("--budget", type=Path, default=HERE / "budget60_e1roots.jsonl")
    parser.add_argument("--budget-e2", type=Path, default=HERE / "budget60_e2roots.jsonl")
    parser.add_argument("--dear", type=int, nargs="*", default=[3300, 3333, 3335, 3336, 3176],
                        help="the dear roots: the searched games past 2175 with the largest GF(2) gaps")
    args = parser.parse_args()
    key = lambda r: (r.get("n"), r.get("arm"), r.get("mode", "certified"))  # noqa: E731
    e1 = load(args.e1, key) or []
    e2 = load(args.e2, key) or []
    found = load(args.search, key) or []
    budget = load(args.budget, key) or []
    for r in e1:
        r.setdefault("mode", "certified")
    print(f"# E2: past 2175\n\nE1: `{args.e1.name}`, {len(e1)} records; E2: `{args.e2.name}`, {len(e2)}; "
          f"search: `{args.search.name}`, {len(found)}.\n")
    if not e2:
        print("No E2 roots.")
        return
    main_rows = per_root(e2, found)
    criteria(e1, main_rows)
    bands(main_rows)
    by_gap(e1, main_rows)
    cap(e2, set(args.dear))
    search(e1, found, budget)
    frontier_budget(main_rows, load(args.budget_e2, key) or [])
    signings(main_rows)


if __name__ == "__main__":
    main()
