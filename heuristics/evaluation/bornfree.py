# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""The Franklín & Moniot "born-free" matching: a lower bound with no matching algorithm.

Franklín, A. F. and Moniot R. K.  The difficulty of beating the
Taxman.  Discrete Applied Mathematics, 339, 166-171, (2023).
Preprint: arXiv:2211.00461, https://arxiv.org/abs/2211.00461

Theorem 1 of that paper identifies the optimal Taxman score with the
maximum-weight matching of the maximal-factor graph (see bound.py)
*restricted to matchings free of flat alternating cycles* -- a
structural constraint that ordinary maximum-weight matching knows
nothing about (bound.py's unconstrained matching can, and does,
include such cycles, which is why it is only an upper bound).

Theorem 3 sidesteps that constraint entirely by constructing a
matching that is free of flat alternating cycles *by construction*,
hence "born free": no matching algorithm runs at all, just two nested
descending loops.

    For each prime p <= N, from largest to smallest:
        For each x with p*x <= N, from largest to smallest:
            If x and p*x are both still unused, match them:
                the player picks p*x, the taxman collects x.

The descending prime order is the load-bearing one: Theorem 3's proof
turns on the fact that a flat alternating cycle would have required a
pair the algorithm must already have taken at a larger prime.  The
descending order over x is the paper's, and is kept for fidelity, but
it makes no difference to the result -- it yields the identical
matching for every n from 2 to 400.  Reversing the primes, by
contrast, is a real change and usually a loss.

The resulting score is a lower bound on the true optimum: unlike
bound.fm_bound's unconstrained matching, every pick set this returns
is realizable as an actual game.  Theorem 4 proves that the restricted
(p_max=5) variant below already exceeds half the pot for all N >= 847.
The paper also states in passing that the strategy wins for every
N > 3; that is false at exactly N=7 and N=13 -- see
test_born_free_beats_half_pot_except_two_small_exceptions.

This module never has to order the picks itself: born_free_matching
returns a {pick: tax} dict in exactly the shape core.order_for_real_game
already knows how to schedule (Kahn's topological sort over the
precedence the matching implies), so born_free_game just calls it.

Usage:
    python3 -m evaluation.bornfree 21 100 1000
    python3 -m evaluation.bornfree --max-n 1000 --out bornfree.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Dict, List, Optional

from core import order_for_real_game, smallest_prime_factors
from evaluation.bound import DEFAULT_OPTIMAL


def born_free_matching(n: int, p_max: Optional[int] = None) -> Dict[int, int]:
    """The born-free matching for game n, as {pick: tax}.

    Franklin & Moniot, Theorem 3: for each prime p <= n (or <= p_max, if
    given) in descending order, for each x with p*x <= n in descending
    order of x, take the pair (x, p*x) if both are still unused -- the
    player picks p*x and pays tax x. Free of flat alternating cycles by
    construction, so (unlike bound.fm_bound's unconstrained matching)
    every pick set this returns is realizable as an actual game; see
    born_free_game.
    """
    if n < 1:
        return {}
    spf = smallest_prime_factors(n)
    primes = [p for p in range(2, n + 1) if spf[p] == p]
    if p_max is not None:
        primes = [p for p in primes if p <= p_max]

    used = [False] * (n + 1)
    matching: Dict[int, int] = {}
    for p in sorted(primes, reverse=True):
        for x in range(n // p, 0, -1):
            y = p * x
            if not used[x] and not used[y]:
                used[x] = True
                used[y] = True
                matching[y] = x
    return matching


def born_free(n: int, p_max: Optional[int] = None) -> int:
    """The born-free score for game n: the sum of the matching's picks."""
    return sum(born_free_matching(n, p_max).keys())


def born_free_game(n: int, p_max: Optional[int] = None) -> List[int]:
    """A legal move sequence realizing the born-free matching for game n."""
    return order_for_real_game(born_free_matching(n, p_max))


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("games", type=int, nargs="*",
                        help="one or more game sizes to score")
    parser.add_argument("--max-n", type=int,
                        help="also sweep n=2..MAX_N")
    parser.add_argument("--p-max", type=int,
                        help="restrict to primes <= P_MAX (the paper's Theorem 4 variant)")
    parser.add_argument("--out", type=Path,
                        help="write the sweep as a JSON object {n: score}")
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    ns = sorted(set(args.games) | set(range(2, args.max_n + 1) if args.max_n else ()))
    if not ns:
        parser.error("give one or more game sizes, or --max-n")

    optimal: Dict[int, dict] = {}
    if args.optimal.exists():
        optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    else:
        print(f"(no optimal solutions found at {args.optimal}, skipping comparison)")

    scores: Dict[int, int] = {}
    for n in ns:
        score = born_free(n, args.p_max)
        scores[n] = score
        pot = n * (n + 1) // 2
        pot_pct = 100 * score / pot if pot else 100.0
        if n in optimal:
            opt_score = optimal[n]["score"]
            pct = 100 * score / opt_score if opt_score else 100.0
            print(f"n={n}: score={score} optimal={opt_score} "
                  f"({pct:.2f}% of optimal, {pot_pct:.2f}% of pot)")
        else:
            print(f"n={n}: score={score} ({pot_pct:.2f}% of pot)")

    if args.out:
        args.out.write_text(json.dumps(scores, indent=2) + "\n")
        print(f"wrote {len(scores)} scores to {args.out}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
