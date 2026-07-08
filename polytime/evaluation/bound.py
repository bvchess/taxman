"""The Franklin & Moniot upper bound on the optimal Taxman score.

Franklin, A. F. and Moniot, R. K., "The difficulty of beating the
Taxman", Discrete Applied Mathematics 339, 166-171, 2023: for game N,
build a graph on vertices 1..N with an edge (d, c) of weight c for
every c in 2..N and every d in maximal_factors(c) - note maximal
factors, not all divisors.  The bound is the weight of a maximum-weight
matching of that graph (every legal game's picks pair with distinct
paid factors, so no game can score above this).

This maximal-factor-edge bound is much sharper than the naive
all-divisors matching bound (see `certify.py`): at n=1000 the gap
to the true optimum is 1168 points here, versus 3038 for the
all-divisors version.

Measured against the known optimal solutions in optimal.json, the
bound is tight - equal to the true optimum - in 45 games, all with
N <= 122 (the last tight game is N=122).  Over N=500..1000 the optimum
averages 99.71% of the bound and never falls below 99.57%.

Usage:
    python3 -m evaluation.bound 21 128 1000
    python3 -m evaluation.bound --max-n 1000 --out bounds.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional, Sequence

from core import maximal_factors, smallest_prime_factors

try:
    import networkx as nx
except ImportError:
    nx = None

DEFAULT_OPTIMAL = Path(__file__).resolve().parent.parent.parent / "src/main/resources/optimal.json"


def fm_bound(n: int, spf: Optional[Sequence[int]] = None) -> int:
    """The Franklin & Moniot upper bound on the optimal score of game n."""
    if nx is None:
        raise RuntimeError("networkx is required for fm_bound(); pip install networkx")
    if spf is None:
        spf = smallest_prime_factors(n)

    graph = nx.Graph()
    for c in range(2, n + 1):
        for d in maximal_factors(c, spf):
            graph.add_edge(d, c, weight=c)
    matching = nx.max_weight_matching(graph)
    return sum(max(a, b) for a, b in matching)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("games", type=int, nargs="*",
                        help="one or more game sizes to bound")
    parser.add_argument("--max-n", type=int,
                        help="also sweep n=2..MAX_N")
    parser.add_argument("--out", type=Path,
                        help="write the sweep as a JSON object {n: bound}")
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    if nx is None:
        print("fm_bound requires networkx: pip install networkx", file=sys.stderr)
        return 1

    ns = sorted(set(args.games) | set(range(2, args.max_n + 1) if args.max_n else ()))
    if not ns:
        parser.error("give one or more game sizes, or --max-n")

    optimal: Dict[int, dict] = {}
    if args.optimal.exists():
        optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    else:
        print(f"(no optimal solutions found at {args.optimal}, skipping comparison)")

    spf = smallest_prime_factors(max(ns))
    bounds: Dict[int, int] = {}
    for n in ns:
        bound = fm_bound(n, spf)
        bounds[n] = bound
        if n in optimal:
            score = optimal[n]["score"]
            gap = bound - score
            pct = 100 * score / bound if bound else 100.0
            print(f"n={n}: bound={bound} optimal={score} gap={gap} "
                  f"({pct:.2f}% of bound)")
        else:
            print(f"n={n}: bound={bound}")

    if args.out:
        args.out.write_text(json.dumps(bounds, indent=2) + "\n")
        print(f"wrote {len(bounds)} bounds to {args.out}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
