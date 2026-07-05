"""How much of an optimal Taxman game comes from the lower half?

Reads the known optimal solutions and reports, per range of N, how many
selections are at or below N/2 and what share of the optimal score they
carry.  This bounds how well "solve the upper half perfectly, ignore the
rest" can do, and quantifies the part of the game that remains hard.

Usage:
    python3 halves.py [--optimal PATH]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import List

from verify import DEFAULT_OPTIMAL


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    games = json.loads(args.optimal.read_text())
    rows = []
    for g in games:
        n, score, moves = g["n"], g["score"], g["moves"]
        if score == 0:
            continue
        lower = [m for m in moves if m <= n / 2]
        rows.append((n, score, len(moves), len(lower), sum(lower)))

    print(f"{'N range':>12} {'lower-half selections':>22} {'share of moves':>15} "
          f"{'share of points':>16} {'max share':>10}")
    for a, b in ((2, 100), (101, 300), (301, 600), (601, 1000)):
        part = [r for r in rows if a <= r[0] <= b]
        if not part:
            continue
        sel = sum(r[3] for r in part) / len(part)
        moves = 100 * sum(r[3] for r in part) / sum(r[2] for r in part)
        points = [100 * r[4] / r[1] for r in part]
        print(f"{f'{a}-{b}':>12} {sel:>22.1f} {moves:>14.1f}% "
              f"{sum(points) / len(points):>15.2f}% {max(points):>9.2f}%")

    worst = max(rows, key=lambda r: r[4] / r[1])
    print(f"\nlargest lower-half share: N={worst[0]} with "
          f"{100 * worst[4] / worst[1]:.2f}% of the optimal score")
    for n in (100, 500, 1000):
        r = next((x for x in rows if x[0] == n), None)
        if r:
            print(f"N={n}: {r[3]} of {r[2]} selections are <= N/2, "
                  f"worth {r[4]} of {r[1]} points ({100 * r[4] / r[1]:.2f}%)")
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())
