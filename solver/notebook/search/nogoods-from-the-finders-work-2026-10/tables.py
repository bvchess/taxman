#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of the nogoods experiment: the share of the root's GF(2) gap that a Lagrangian bound over the nogoods of
the finder, of the sound search and of both closes, on eight games of known optimum at 10 and at 120 CPU seconds, and
the bounds of four open games against the best upper bound of `results/bounds.jsonl`.

Reads known-10s.jsonl, known-120s.jsonl, open-10s.jsonl, open-120s.jsonl (and plain-known-*.jsonl, the first pass) in
this directory and ../../../../results/bounds.jsonl.
usage: ./tables.py > tables.txt
"""
import json
import statistics as st
from pathlib import Path

HERE = Path(__file__).resolve().parent
BOUNDS = HERE.parents[3] / "results" / "bounds.jsonl"
POOLS = ("finder", "search", "both")


def lines(name):
    path = HERE / name
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()] if path.exists() else []


def best_bounds(n):
    """The best lower and upper bound of board n in the store (value, rule), or None."""
    low = up = None
    for line in BOUNDS.read_text().splitlines():
        d = json.loads(line)
        if d["n"] != n:
            continue
        if d["side"] == "lower" and (low is None or d["value"] > low[0]):
            low = (d["value"], d["rule"])
        if d["side"] == "upper" and (up is None or d["value"] < up[0]):
            up = (d["value"], d["rule"])
    return low, up


def plain():
    """The first pass, with plain subgradient steps (no momentum): the mean shares, to set beside the final fits."""
    print("\nTHE FIRST PASS, plain subgradient steps (plain-known-*.jsonl): mean share of the root gap closed, K / L")
    for cpu in (10, 120):
        rows = lines(f"plain-known-{cpu}s.jsonl")
        if rows:
            print(f"  {cpu:3d} s: " + ", ".join(
                f"{p} {st.mean(d[p + '_bound_share'] for d in rows):.3f} / {st.mean(d[p + '_long_share'] for d in rows):.3f}"
                for p in POOLS) + f"; pools (median) " + ", ".join(
                f"{p} {st.median(d[p + '_pool'] for d in rows):.0f}" for p in POOLS))


def known(cpu):
    rows = sorted(lines(f"known-{cpu}s.jsonl"), key=lambda d: d["upper"] - d["opt"])
    if not rows:
        return
    print(f"\nKNOWN GAMES, finder and search {cpu} CPU s: share of the root gap (upper - opt) closed, by the bound")
    print("after 200 steps (K) and after 2000 (L)")
    head = f"  {'n':>5} {'gap':>5} {'cands':>5} | " + " | ".join(f"{p:^17}" for p in POOLS)
    print(head)
    print(f"  {'':>5} {'':>5} {'':>5} | " + " | ".join(f"{'pool':>5} {'K':>5} {'L':>5}" for _ in POOLS))
    for d in rows:
        cells = " | ".join(
            f"{d[p + '_pool']:5d} {d[p + '_bound_share']:5.3f} {d[p + '_long_share']:5.3f}" for p in POOLS)
        print(f"  {d['n']:5d} {d['upper'] - d['opt']:5d} {d['candidates']:5d} | {cells}")
    cells = " | ".join(
        f"{st.median(d[p + '_pool'] for d in rows):5.0f} "
        f"{st.mean(d[p + '_bound_share'] for d in rows):5.3f} {st.mean(d[p + '_long_share'] for d in rows):5.3f}"
        for p in POOLS)
    print(f"  {'mean':>5} {'':>5} {'':>5} | {cells}   (the pool column: median)")
    print("  by the bound in gap units (upper - bound, summed over the games): "
          + ", ".join(f"{p} K {sum(d['upper'] - d[p + '_bound'] for d in rows)} "
                      f"L {sum(d['upper'] - d[p + '_long'] for d in rows)}" for p in POOLS)
          + f"; the gaps sum to {sum(d['upper'] - d['opt'] for d in rows)}")
    print(f"  work: finder {st.mean(d['finder_units'] for d in rows):.0f} units, "
          f"{st.mean(d['finder_nodes'] for d in rows):.0f} steps, learned {st.mean(d['finder_learned'] for d in rows):.0f} "
          f"nogoods, {st.mean(d['finder_pool'] for d in rows):.0f} distinct; search "
          f"{st.mean(d['search_nodes'] for d in rows):.0f} steps, learned {st.mean(d['search_learned'] for d in rows):.0f}, "
          f"{st.mean(d['search_pool'] for d in rows):.0f} distinct; both {st.mean(d['both_pool'] for d in rows):.0f}")
    print("  the bounds' seconds (K steps, L steps), mean: "
          + ", ".join(f"{p} {st.mean(d[p + '_bound_s'] for d in rows):.2f}, {st.mean(d[p + '_long_s'] for d in rows):.2f}"
                      for p in POOLS)
          + f"; the root bound {1e3 * st.mean(d['base_s'] for d in rows):.1f} ms")
    print(f"  finder game vs optimum: {sum(d['finder'] == d['opt'] for d in rows)} of {len(rows)} reach it; control: "
          f"{sum(d['search'] == d['opt'] for d in rows)}; nogoods that promote: {sum(d['promotable'] for d in rows)}; "
          f"fits closed (proved the incumbent optimal): {sum(d[p + '_closed_at'] is not None for d in rows for p in POOLS)}")


def open_games(cpu):
    rows = sorted(lines(f"open-{cpu}s.jsonl"), key=lambda d: d["n"])
    if not rows:
        return
    print(f"\nOPEN GAMES, finder and search {cpu} CPU s: bounds as scores, against the best upper bound of the store")
    for d in rows:
        n = d["n"]
        low, up = best_bounds(n)
        store = f"store lower {low[0]} ({low[1]}), upper {up[0]} ({up[1]})" if low and up else "not in the store"
        print(f"  {n}: root GF(2) bound {d['upper']}, incumbent {d['incumbent']} (start {d['start']}, finder "
              f"{d['finder']}, search {d['search']}), gap {d['upper'] - d['incumbent']}; {store}")
        lower = max(d["incumbent"], low[0] if low else 0)
        for p in POOLS:
            parts = []
            for suffix, label in (("bound", "K"), ("long", "L")):
                bound = d[f"{p}_{suffix}"]
                beats = "" if not up else (f", {up[0] - bound} below the store's upper" if bound < up[0]
                                           else f", {bound - up[0]} above the store's upper")
                parts.append(f"{label} {bound} ({d['upper'] - bound} off the root{beats}; {bound - lower} over the lower)")
            print(f"      {p:6} pool {d[p + '_pool']:6d}: " + "; ".join(parts))
        print(f"      work: finder {d['finder_units']} units, {d['finder_nodes']} steps, learned {d['finder_learned']}; "
              f"search {d['search_nodes']} steps, learned {d['search_learned']}; start {d['start_cpu_s']:.1f} s, "
              f"bounds K {d['both_bound_s']:.1f} s L {d['both_long_s']:.1f} s")


if __name__ == "__main__":
    for cpu in (10, 120):
        known(cpu)
    for cpu in (10, 120):
        open_games(cpu)
    plain()
