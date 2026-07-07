"""Portfolio baseline and research on where the forced-upper hybrid loses.

Reads the per-game outputs of approx.py (approx_results.json) and
upper_fidelity.py (fidelity_results.json) and reports:

1. Portfolio: play every strategy, keep the best game.  This is the
   real baseline any smarter single strategy has to beat.
2. Forensics on the games where plain OneTax beats the forced-upper
   hybrid: where they live, how big the margins are, and what
   distinguishes them (upper numbers OneTax missed, forced stalls,
   upper gain vs lower deficit).
3. A tax-count census of the known optimal games: how many optimal
   moves pay 1, 2, 3+ divisors of tax, and how many points ride on
   multi-tax moves - the ceiling for any "pay two taxes when frozen"
   extension of OneTax.

Usage:
    python3 portfolio.py [--max-n 1000] [--optimal PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List

from verify import DEFAULT_OPTIMAL

HERE = Path(__file__).resolve().parent


def pct(a: int, b: int) -> str:
    return f"{100 * a / b:.3f}%"


def report_portfolio(rows: List[dict], solvent: Dict[int, int]) -> None:
    names = ("onetax", "hybrid", "oracle", "solvent", "duo", "quad")
    total = {name: 0 for name in names + ("opt",)}
    exact = {name: 0 for name in names}
    argmax = {"onetax": 0, "hybrid": 0, "oracle": 0, "solvent": 0, "tie": 0}
    have_oracle = "oracle" in rows[0]
    for r in rows:
        g = solvent[r["n"]]
        o = r.get("oracle", 0)
        scores = {"onetax": r["onetax"], "hybrid": r["hybrid"],
                  "oracle": o, "solvent": g,
                  "duo": max(r["onetax"], r["hybrid"]),
                  "quad": max(r["onetax"], r["hybrid"], o, g)}
        total["opt"] += r["opt"]
        for name in names:
            total[name] += scores[name]
            if scores[name] == r["opt"]:
                exact[name] += 1
        winners = [name for name in ("onetax", "hybrid", "oracle", "solvent")
                   if scores[name] == scores["quad"]]
        argmax[winners[0] if len(winners) == 1 else "tie"] += 1

    print("=== 1. Portfolio baseline (per-game best) ===")
    for name in names:
        if name == "oracle" and not have_oracle:
            continue
        label = {"duo": "max(onetax, hybrid)",
                 "quad": "max(all four)"}.get(name, name)
        print(f"  {label:<28} {pct(total[name], total['opt']):>8}   "
              f"exactly optimal in {exact[name]}/{len(rows)}")
    print(f"  sole best strategy: onetax {argmax['onetax']}, "
          f"hybrid {argmax['hybrid']}, oracle {argmax['oracle']}, "
          f"solvent {argmax['solvent']}, tie {argmax['tie']}")
    print()


def report_losing_games(rows: List[dict]) -> None:
    losing = [r for r in rows if r["hybrid"] < r["onetax"]]
    winning = [r for r in rows if r["hybrid"] > r["onetax"]]

    print("=== 2. Games where plain OneTax beats the forced hybrid ===")
    print(f"  losing games: {len(losing)}, winning: {len(winning)}, "
          f"ties: {len(rows) - len(losing) - len(winning)}")

    print(f"  {'N range':>10} {'lose':>5} {'win':>5} {'tie':>5} "
          f"{'avg margin when losing':>23} {'when winning':>13}")
    for a, b in ((2, 100), (101, 250), (251, 500), (501, 750), (751, 1000)):
        seg = [r for r in rows if a <= r["n"] <= b]
        seg_l = [r for r in seg if r["hybrid"] < r["onetax"]]
        seg_w = [r for r in seg if r["hybrid"] > r["onetax"]]
        ml = (sum(r["onetax"] - r["hybrid"] for r in seg_l) / len(seg_l)
              if seg_l else 0)
        mw = (sum(r["hybrid"] - r["onetax"] for r in seg_w) / len(seg_w)
              if seg_w else 0)
        print(f"  {f'{a}-{b}':>10} {len(seg_l):>5} {len(seg_w):>5} "
              f"{len(seg) - len(seg_l) - len(seg_w):>5} {ml:>23.1f} {mw:>13.1f}")

    def upper_gain(r: dict) -> int:
        return r["upper_opt_sum"] - r["upper_onetax_sum"]

    def lower_delta(r: dict) -> int:
        onetax_lower = r["onetax"] - r["upper_onetax_sum"]
        hybrid_lower = r["hybrid"] - r["upper_opt_sum"]
        return onetax_lower - hybrid_lower

    for name, seg in (("losing", losing), ("winning", winning)):
        if not seg:
            continue
        ug = sum(upper_gain(r) for r in seg) / len(seg)
        ld = sum(lower_delta(r) for r in seg) / len(seg)
        fc = sum(r["hybrid_forced"] for r in seg) / len(seg)
        miss = sum(r["onetax_missed"] for r in seg) / len(seg)
        print(f"  {name}: avg upper gain {ug:.1f}, avg lower given up "
              f"{ld:.1f}, avg forced stalls {fc:.1f}, "
              f"avg upper numbers OneTax missed {miss:.1f}")

    print("  hybrid outcome by upper numbers OneTax missed:")
    for lo, hi in ((0, 0), (1, 2), (3, 5), (6, 9), (10, 99)):
        seg = [r for r in rows if lo <= r["onetax_missed"] <= hi]
        if not seg:
            continue
        w = sum(1 for r in seg if r["hybrid"] > r["onetax"])
        l = sum(1 for r in seg if r["hybrid"] < r["onetax"])
        net = sum(r["hybrid"] - r["onetax"] for r in seg)
        label = f"{lo}" if lo == hi else f"{lo}-{hi if hi < 99 else '+'}"
        print(f"    missed {label:>4}: {len(seg):>3} games, hybrid wins {w:>3},"
              f" loses {l:>3}, net {net:+d} points")
    print()


def report_tax_census(optimal: List[dict], max_n: int) -> None:
    by_taxes = {1: 0, 2: 0, 3: 0}
    points = {1: 0, 2: 0, 3: 0}
    upper_multi = lower_multi = 0
    games_with_multi = 0
    worst: List[tuple] = []

    for g in optimal:
        n = g["n"]
        if n > max_n or g["score"] == 0:
            continue
        pot = set(range(1, n + 1))
        multi_here = 0
        for c in g["moves"]:
            tax = {d for d in pot if d != c and c % d == 0}
            pot -= tax
            pot.discard(c)
            k = min(len(tax), 3)
            by_taxes[k] += 1
            points[k] += c
            if k >= 2:
                multi_here += c - sum(tax)
                if c > n / 2:
                    upper_multi += 1
                else:
                    lower_multi += 1
        if multi_here:
            games_with_multi += 1
            worst.append((multi_here, n))

    moves = sum(by_taxes.values())
    print("=== 3. Tax census of the known optimal games ===")
    for k, label in ((1, "exactly 1 tax"), (2, "exactly 2 taxes"),
                     (3, "3 or more taxes")):
        print(f"  moves paying {label:<16} {by_taxes[k]:>7} "
              f"({100 * by_taxes[k] / moves:.2f}% of moves, "
              f"{points[k]} points)")
    print(f"  games containing a multi-tax move: {games_with_multi}")
    print(f"  multi-tax moves above/below N/2: {upper_multi}/{lower_multi}")
    worst.sort(reverse=True)
    print(f"  largest net value on multi-tax moves in one game: "
          f"{[f'n={n}: {v:+d}' for v, n in worst[:5]]}")


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--max-n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    rows = [r for r in json.loads((HERE / "fidelity_results.json").read_text())
            if r["n"] <= args.max_n]
    solvent = {int(n): v["solvent"] for n, v in
               json.loads((HERE / "approx_results.json").read_text()).items()}
    optimal = json.loads(args.optimal.read_text())

    report_portfolio(rows, solvent)
    report_losing_games(rows)
    report_tax_census(optimal, args.max_n)
    return 0


if __name__ == "__main__":
    sys.exit(main())
