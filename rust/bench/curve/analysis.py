#!/usr/bin/env python3
"""Step 5: calibration, per-band statistics, log-linear fits and chain-cost projections from the raw JSONL files.

Reads chain.jsonl (strict chain 1001-1231), calib.jsonl (standalone, same games), bands.jsonl (standalone beyond 1231),
scaling.jsonl and recheck.jsonl, and prints markdown tables for the README.  Usage: analysis.py
"""
import argparse
import math
import statistics

from common import HERE, SEARCH_LIMIT, is_prime, load_jsonl, non_primes_from

# AWS c9g.48xlarge, 192 cores: price per hour and the speed-up over this machine's 8 threads.
PRICES = {"spot": 3.33, "on-demand": 8.35}
SPEEDUPS = {"optimistic (19x)": 19.0, "pessimistic (10x)": 10.0}

# Fits use the bands that start at or above each of these; MAIN_FROM is the one projected in full.  Below 1200 the
# games are cheap whatever n is (a few candidates), so those bands say little about the growth.
FIT_STARTS = (1150, 1200, 1250)
MAIN_FROM = 1200


def quantile(values, q):
    """Linear interpolation between order statistics (numpy's default)."""
    v = sorted(values)
    pos = q * (len(v) - 1)
    lo, hi = math.floor(pos), math.ceil(pos)
    return v[lo] + (v[hi] - v[lo]) * (pos - lo)


def fit(points):
    """Least squares y = a + b x; returns (a, b, residual standard deviation, mean x, standard error of b)."""
    n = len(points)
    mx = sum(x for x, _ in points) / n
    my = sum(y for _, y in points) / n
    sxx = sum((x - mx) ** 2 for x, _ in points)
    b = sum((x - mx) * (y - my) for x, y in points) / sxx
    a = my - b * mx
    resid = math.sqrt(sum((y - a - b * x) ** 2 for x, y in points) / max(n - 2, 1))
    return a, b, resid, mx, resid / math.sqrt(sxx)


def fmt_time(s):
    return f"{s:.1f}" if s < 100 else f"{s:.0f}"


def band_stats(rows, timeout):
    """Statistics of one band's standalone runs (rows: records with n, status, search_s, nodes)."""
    times = [r["search_s"] if r["status"] == "DONE" else max(r["search_s"], timeout) for r in rows]
    timeouts = sum(r["status"] != "DONE" for r in rows)
    k = len(times)
    # A statistic is censored (only a lower bound) when it depends on the value of a timed-out game.
    censored = {
        "median": timeouts * 2 >= k + (k % 2 == 0) and timeouts > 0 and sorted(times)[(k - 1) // 2] >= timeout,
        "p75": timeouts > 0 and quantile(times, 0.75) >= timeout,
        "max": timeouts > 0,
        "mean": timeouts > 0,
    }
    return {
        "ns": [r["n"] for r in rows],
        "k": k,
        "timeouts": timeouts,
        "median": statistics.median(times),
        "p75": quantile(times, 0.75),
        "max": max(times),
        "mean": sum(times) / k,
        "nodes_median": statistics.median(r["nodes"] for r in rows),
        "nodes_max": max(r["nodes"] for r in rows),
        # Share of the search before its final incumbent was found (0 when the start incumbent was already optimal).
        "found_share": statistics.median((r["found_s"] or 0.0) / max(r["search_s"], 1e-9) for r in rows),
        "censored": censored,
    }


def calibration(chain, calib):
    """Standalone / chained cost per game class; returns the ratios used by the projection."""
    chain_by_n = {r["n"]: r for r in chain}
    rows = []
    for r in calib:
        c = chain_by_n[r["n"]]
        assert r["status"] == "DONE" and r["best"] == r["opt"], r
        rows.append({"n": r["n"], "chain_s": c["seconds"]["total"], "chain_nodes": c["nodes"],
                     "alone_s": r["search_s"] + r["prep_s"], "alone_nodes": r["nodes"]})

    def klass(row):
        if row["chain_nodes"] == 0:
            return "cap reached (0 nodes)"
        return "small search (chain < 5 s)" if row["chain_s"] < 5 else "big search (chain >= 5 s)"

    print("### Calibration: standalone (no seed, no cap) against chained, per class of the chained game\n")
    print("| class | games | chain s (sum) | standalone s (sum) | standalone/chain (sums) | median per-game ratio "
          "| min-max per-game | nodes standalone/chain (sums) |")
    print("|---|---|---|---|---|---|---|---|")
    groups = {}
    for row in rows:
        groups.setdefault(klass(row), []).append(row)
    groups["all non-prime games"] = rows
    groups["big searches, n >= 1150"] = [r for r in rows if klass(r).startswith("big") and r["n"] >= 1150]
    for name in ["cap reached (0 nodes)", "small search (chain < 5 s)", "big search (chain >= 5 s)",
                 "big searches, n >= 1150", "all non-prime games"]:
        g = groups[name]
        cs = sum(r["chain_s"] for r in g)
        als = sum(r["alone_s"] for r in g)
        ratios = [r["alone_s"] / r["chain_s"] for r in g]
        cn, an = sum(r["chain_nodes"] for r in g), sum(r["alone_nodes"] for r in g)
        node_ratio = f"{an / cn:.2f}" if cn else "n/a (chain 0)"
        print(f"| {name} | {len(g)} | {cs:.1f} | {als:.1f} | {als / cs:.2f} | {statistics.median(ratios):.2f} "
              f"| {min(ratios):.2f}-{max(ratios):.1f} | {node_ratio} |")
    prime_chain = sum(r["seconds"]["total"] for r in chain if r["method"] == "prime")
    print(f"\nChain total {sum(r['seconds']['total'] for r in chain):.0f} s over {len(chain)} games; the "
          f"{sum(r['method'] == 'prime' for r in chain)} primes cost {prime_chain:.2f} s together.\n")
    print("Largest games, chained against standalone (s): " + ", ".join(
        f"{r['n']}: {r['chain_s']:.1f} vs {r['alone_s']:.1f}"
        for r in sorted(rows, key=lambda r: -r["alone_s"])[:8]) + "\n")
    big = groups["big searches, n >= 1150"]
    window = [r for r in rows if r["n"] >= 1150]
    return {
        "ratio_big": sum(r["chain_s"] for r in big) / sum(r["alone_s"] for r in big),
        "ratio_window": sum(r["chain_s"] for r in window) / sum(r["alone_s"] for r in window),
        "ratio_all": sum(r["chain_s"] for r in rows) / sum(r["alone_s"] for r in rows),
    }


def bands_table(calib, bands, timeout):
    """Per-band table (bands of the first 8 non-primes from a start; the calibration supplies those below 1250)."""
    by_n = {r["n"]: r for r in calib}
    by_n.update({r["n"]: r for r in bands})
    specs = [(s, 8) for s in (1001, 1050, 1100, 1150, 1200, 1250, 1300)] + [(1350, 3)]
    out = []
    print("### Standalone cost per band (8 threads, no seed, no cap; the first non-primes from the band start)\n")
    print("| band start | games (n) | median s | p75 s | max s | mean s | timeouts | median nodes (M) | max nodes (M) "
          "| median share of time before the final incumbent |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for start, count in specs:
        ns = non_primes_from(start, count)
        rows = [by_n[n] for n in ns if n in by_n]
        if len(rows) < count:
            continue
        st = band_stats(rows, timeout)
        st["start"] = start
        st["x"] = sum(ns) / len(ns)
        out.append(st)
        print(f"| {start} | {ns[0]}-{ns[-1]} ({len(ns)}) | {fmt_time(st['median'])} | {fmt_time(st['p75'])} "
              f"| {fmt_time(st['max'])} | {fmt_time(st['mean'])} | {st['timeouts']} | {st['nodes_median'] / 1e6:.1f} "
              f"| {st['nodes_max'] / 1e6:.1f} | {100 * st['found_share']:.0f}% |")
    print()
    return out


def fits(band_rows, starts):
    """Fits of log10(time) against n for the band medians, means and maxima, over the bands that start at or
    above each of `starts`; a censored statistic (one that depends on a timed-out game) is left out."""
    print("### Fits: log10(seconds) = a + b n over band statistics (x = mean n of the band's games)\n")
    print("| statistic | bands used | b (per n) | s.e. of b | x10 per dn | a | residual sd (log10) "
          "| seconds at 1231 | at 1300 | at 1400 |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    result = []
    for start in starts:
        label = f"bands from {start}"
        for stat in ("median", "mean", "max"):
            pts = [(b["x"], math.log10(b[stat])) for b in band_rows if b["start"] >= start
                   and not b["censored"].get(stat, False)]
            if len(pts) < 3:
                continue
            a, slope, sd, xc, se = fit(pts)
            result.append((f"{stat}, {label}", (a, slope, sd, xc, se)))
            print(f"| {stat}, {label} | {len(pts)} | {slope:.5f} | {se:.5f} | {1 / slope:.0f} | {a:.2f} | {sd:.2f} "
                  f"| {10 ** (a + slope * 1231):.0f} | {10 ** (a + slope * 1300):.0f} "
                  f"| {10 ** (a + slope * 1400):.0f} |")
    print()
    return result


def cumulative(params, ratio, target, extra_slope=0.0):
    """Seconds on this machine (8 threads) to extend the chain from 1231 to `target`: the sum over the non-primes
    of ratio x 10^(a + b n); `extra_slope` moves the slope while keeping the line through the band centroid."""
    a, slope, _, xc = params
    slope += extra_slope
    yc = a + params[1] * xc
    return sum(ratio * 10 ** (yc + slope * (n - xc)) for n in range(1232, target + 1) if not is_prime(n))


def project(params, ratio, label):
    """Table of the cumulative chain cost to N on this machine and on 192 cores, for one fit."""
    a, slope, _, xc = params
    print(f"#### {label}\n")
    print("| N | game N standalone (s) | cumulative, this machine (8 thr.) | 192 cores, 19x | spot $ | on-demand $ "
          "| 192 cores, 10x | spot $ | on-demand $ |")
    print("|---|---|---|---|---|---|---|---|---|")
    for target in (1300, 1400, 1450, 1500, 1550, 1600, 1700, 1800, 2000):
        total = cumulative(params, ratio, target)
        cells = [f"{10 ** (a + slope * target):.3g}", human(total)]
        for speed in SPEEDUPS.values():
            hours = total / speed / 3600
            cells += [human(total / speed), f"{hours * PRICES['spot']:.3g}", f"{hours * PRICES['on-demand']:.3g}"]
        print(f"| {target} | " + " | ".join(cells) + " |")
    print()


def reached(params, ratio, budget_s, extra_slope=0.0):
    """The largest N to which the chain can be extended within `budget_s` seconds on this machine (8 threads)."""
    a, slope, _, xc = params
    slope += extra_slope
    yc = a + params[1] * xc
    total, n = 0.0, 1231
    while True:
        n += 1
        if not is_prime(n):
            total += ratio * 10 ** (yc + slope * (n - xc))
        if total > budget_s:
            return n - 1


def reach_table(all_params, ratio):
    """How far a 192-core machine gets in a time or money budget: the main fit, and its slope +- 1 s.e."""
    params = next(p for label, p in all_params if label == f"mean, bands from {MAIN_FROM}")
    se = params[4]
    budgets = [("1 day", 86400), ("1 week", 7 * 86400), ("30 days", 30 * 86400), ("1 year", 365 * 86400)]
    budgets += [(f"${d:,} spot", d / PRICES["spot"] * 3600) for d in (100, 1000, 10000, 100000)]
    budgets += [(f"${d:,} on-demand", d / PRICES["on-demand"] * 3600) for d in (1000, 10000)]
    print("#### How far the chain gets: the largest N reached within the budget (mean fit from "
          f"{MAIN_FROM}; central slope, and in brackets the range for the slope +1 s.e. to -1 s.e.)\n")
    print("| budget | 192 cores, 19x | 192 cores, 10x | this machine (8 thr.) |")
    print("|---|---|---|---|")
    for name, seconds in budgets:
        cells = []
        for speed in (19.0, 10.0, 1.0):
            central = reached(params[:4], ratio, seconds * speed)
            low = reached(params[:4], ratio, seconds * speed, -se)
            high = reached(params[:4], ratio, seconds * speed, se)
            cells.append(f"{central} ({high}-{low})")
        print(f"| {name} | " + " | ".join(cells) + " |")
    print()


def sensitivity(all_params, ratio):
    """Cumulative time on this machine (8 threads) to N for every fit, and the main fit with its slope +- 1 s.e."""
    print("#### Sensitivity: cumulative chain cost on this machine (8 threads) for each fit\n")
    targets = (1400, 1500, 1600)
    print("| fit | slope b | s.e. of b | " + " | ".join(f"to {t}" for t in targets) + " |")
    print("|---|---|---|" + "---|" * len(targets))
    for label, params in all_params:
        a, slope, sd, xc, se = params
        row = [f"{slope:.5f}", f"{se:.5f}"] + [human(cumulative(params[:4], ratio, t)) for t in targets]
        print(f"| {label} | " + " | ".join(row) + " |")
        if label == f"mean, bands from {MAIN_FROM}":
            for sign, name in ((-1, "low"), (1, "high")):
                row = [f"{slope + sign * se:.5f}", ""] + [
                    human(cumulative(params[:4], ratio, t, sign * se)) for t in targets]
                print(f"| {label}, slope {name} (+-1 s.e.) | " + " | ".join(row) + " |")
    print()


def human(seconds):
    for unit, size in (("years", 365 * 86400), ("days", 86400), ("h", 3600), ("min", 60)):
        if seconds >= size:
            return f"{seconds / size:.3g} {unit}"
    return f"{seconds:.3g} s"


def scaling(path):
    rows = sorted(load_jsonl(path), key=lambda r: r["threads"])
    if not rows:
        return
    base = rows[0]["search_s"] * rows[0]["threads"]
    print("### Thread scaling, proof only (seeded with the game's own optimum)\n")
    print("| n | threads | search s | speed-up over 1 thread | nodes (M) | k nodes/s per thread | "
          "speed-up of nodes/s | effective parallelism % |")
    print("|---|---|---|---|---|---|---|---|")
    per_thread = None
    for r in rows:
        rate = r["nodes"] / r["search_s"] / r["threads"] / 1e3
        per_thread = per_thread or rate
        print(f"| {r['n']} | {r['threads']} | {r['search_s']:.1f} | {base / r['search_s']:.2f} "
              f"| {r['nodes'] / 1e6:.2f} | {rate:.1f} | {rate * r['threads'] / per_thread:.2f} | {r['eff_pct']} |")
    print()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    args = ap.parse_args()
    chain = load_jsonl(HERE / "chain.jsonl")
    calib = load_jsonl(HERE / "calib.jsonl")
    bands = load_jsonl(HERE / "bands.jsonl")
    ratios = calibration(chain, calib)
    print("Ratios chain/standalone used below: " + ", ".join(f"{k} {v:.2f}" for k, v in ratios.items()) + "\n")
    band_rows = bands_table(calib, bands, SEARCH_LIMIT)
    all_params = fits(band_rows, FIT_STARTS)
    ratio = ratios["ratio_window"]
    print(f"### Projection of the cumulative chain cost from 1231 to N (chain cost per non-prime game = {ratio:.2f} "
          "x the standalone fit)\n")
    for label, params in all_params:
        if label.endswith(f"bands from {MAIN_FROM}"):
            project(params[:4], ratio, f"{label}")
    sensitivity(all_params, ratio)
    reach_table(all_params, ratio)
    scaling(HERE / "scaling.jsonl")
    recheck = load_jsonl(HERE / "recheck.jsonl")
    first = {r["n"]: r for r in calib + bands}
    print("### Repeat runs (same game, later in the day with Zoom running)\n")
    print("| n | first run s | nodes (M) | k nodes/s | repeat s | nodes (M) | k nodes/s |")
    print("|---|---|---|---|---|---|---|")
    for r in recheck:
        f = first[r["n"]]
        print(f"| {r['n']} | {f['search_s']:.1f} | {f['nodes'] / 1e6:.1f} | {f['knps']:.0f} | {r['search_s']:.1f} "
              f"| {r['nodes'] / 1e6:.1f} | {r['knps']:.0f} |")


if __name__ == "__main__":
    main()
