"""Independent verification of optimal.json, free of the frame theory.

optimal.json was produced by the frame/mini-game solver, and this
project's upper-half algorithm is built on the same structural theory.
Agreement between them is therefore not, by itself, evidence that
either is correct.  This script certifies as much of optimal.json as
possible using only arguments that share no assumptions with frames,
mini games, or maximal-factor decompositions:

1. Replay: every solution in optimal.json is checked to be a legal
   game whose score matches, making every entry a sound LOWER bound.

2. Brute force: an exact solver over raw pot states - pick any number
   with a surviving divisor, tax sweeps all remaining divisors -
   with memoization on normalized states and a simple sound bound for
   pruning (a pick consumes a distinct extra number, so at most
   floor(|pot|/2) picks, each from the currently-pickable set).  For
   every game it completes, the true optimum is known unconditionally,
   and every optimal solution's set of picks above N/2 is enumerated.

3. Certificate chain: opt(N) <= N + opt(N-1), proved in one paragraph
   with no structural theory (N can never be tax, so an N-game that
   skips N maps to an (N-1)-game).  Whenever the recorded solution
   achieves N + score(N-1), its optimality follows from game N-1's.
   Certification therefore propagates upward from the brute-forced
   base through every such game.

4. Matching bound: opt(N) is at most the maximum-weight matching of
   the divisor graph with edge weight = the larger endpoint (Franklin
   & Moniot 2023; every legal game's picks pair with distinct paid
   divisors).  Wherever that bound equals the recorded score, the game
   is certified outright.  Requires networkx; skipped if unavailable.

Usage:
    python3 independent.py [--budget 30] [--brute-max 64]
                           [--matching-max 300] [--optimal PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, FrozenSet, List, Optional, Set, Tuple

from approx import check_sequence, divisor_lists
from bitpot import bits, divisor_masks, mask_of, multiple_masks, popcount
from taxman_mini import smallest_prime_factors, solve_upper_half
from verify import DEFAULT_OPTIMAL


class OutOfTime(Exception):
    pass


class Exact:
    """Exact taxman solver over raw pot states.  No structural theory."""

    def __init__(self, n: int, divs, deadline: float):
        self.n = n
        self.divs = divs
        self.dmask = divisor_masks(n)
        self.mmask = multiple_masks(n)
        self.deadline = deadline
        self.memo: Dict[int, int] = {}
        self.calls = 0

    def multiples(self, c: int, pot: int) -> bool:
        return self.mmask[c] & pot != 0

    def normalize(self, pot: int) -> int:
        """Drop inert numbers: never pickable and never usable as tax."""
        changed = True
        while changed:
            changed = False
            for c in list(bits(pot)):
                if self.dmask[c] & pot == 0 and self.mmask[c] & pot == 0:
                    pot &= ~(1 << c)
                    changed = True
        return pot

    def bound(self, pot: int) -> int:
        pickable = sorted(
            (c for c in bits(pot) if self.dmask[c] & pot),
            reverse=True,
        )
        return sum(pickable[: min(len(pickable), popcount(pot) // 2)])

    def best(self, pot: int) -> int:
        cached = self.memo.get(pot)
        if cached is not None:
            return cached
        self.calls += 1
        if self.calls % 4096 == 0 and time.monotonic() > self.deadline:
            raise OutOfTime
        result = 0
        moves = sorted(
            (c for c in bits(pot) if self.dmask[c] & pot),
            reverse=True,
        )
        for c in moves:
            tax = self.dmask[c] & pot
            child = self.normalize(pot & ~tax & ~(1 << c))
            if c + self.bound(child) <= result:
                continue
            result = max(result, c + self.best(child))
        self.memo[pot] = result
        return result

    def optimal_upper_sets(
        self, pot: int, cap: int = 64
    ) -> Optional[Set[FrozenSet[int]]]:
        """Every set of >n/2 picks used by some optimal continuation."""
        memo: Dict[int, Optional[Set[FrozenSet[int]]]] = {}

        def walk(state: int) -> Optional[Set[FrozenSet[int]]]:
            if state in memo:
                return memo[state]
            target = self.best(state)
            if target == 0:
                memo[state] = {frozenset()}
                return memo[state]
            found: Set[FrozenSet[int]] = set()
            for c in bits(state):
                tax = self.dmask[c] & state
                if not tax:
                    continue
                child = self.normalize(state & ~tax & ~(1 << c))
                if c + self.best(child) != target:
                    continue
                tails = walk(child)
                if tails is None:
                    memo[state] = None
                    return None
                for tail in tails:
                    found.add(tail | {c} if 2 * c > self.n else tail)
                    if len(found) > cap:
                        memo[state] = None
                        return None
            memo[state] = found
            return found

        return walk(pot)


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--budget", type=float, default=30.0,
                        help="seconds of brute force per game")
    parser.add_argument("--brute-max", type=int, default=64)
    parser.add_argument("--matching-max", type=int, default=300)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    games = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    top = max(games)
    divs = divisor_lists(top)
    spf = smallest_prime_factors(top)

    # --- 1. replay: every entry is a legal lower bound -------------------
    for n, g in games.items():
        assert check_sequence(n, g["moves"]) == g["score"] if g["moves"] \
            else g["score"] == 0
    print(f"replay: all {len(games)} recorded solutions are legal games "
          f"with matching scores (sound lower bounds)")

    # --- 2. brute force --------------------------------------------------
    sys.setrecursionlimit(100_000)
    brute_verified = 0
    upper_confirmed = 0
    upper_unique = 0
    for n in range(1, args.brute_max + 1):
        solver = Exact(n, divs, time.monotonic() + args.budget)
        try:
            true_opt = solver.best(mask_of(range(1, n + 1)))
        except OutOfTime:
            print(f"brute force: n={n} exceeded {args.budget:.0f}s budget, "
                  f"stopping here")
            break
        if true_opt != games[n]["score"]:
            print(f"brute force: DISAGREEMENT at n={n}: "
                  f"true {true_opt} vs recorded {games[n]['score']}")
            continue
        brute_verified = n

        sets = solver.optimal_upper_sets(mask_of(range(1, n + 1)))
        ours = frozenset(solve_upper_half(n, spf)[0])
        if sets is not None:
            if len(sets) == 1:
                upper_unique += 1
            if ours in sets:
                upper_confirmed += 1
            elif sets:
                print(f"brute force: n={n}: our upper set {sorted(ours)} not "
                      f"among optimal upper sets {[sorted(s) for s in sets]}")

    print(f"brute force: optimal.json scores verified exhaustively for "
          f"n=1..{brute_verified} (no theory assumed)")
    print(f"  our upper-half set appears in an optimal solution in "
          f"{upper_confirmed}/{brute_verified}; unique optimal upper set in "
          f"{upper_unique}/{brute_verified}")

    # --- 3. matching upper bound (certifies games outright) ---------------
    matching_tight: Set[int] = set()
    try:
        import networkx as nx
    except ImportError:
        print("matching bound: networkx not installed, skipped")
        nx = None
    if nx is not None:
        started = time.monotonic()
        limit = min(args.matching_max, top)
        for n in range(2, limit + 1):
            graph = nx.Graph()
            for c in range(2, n + 1):
                for d in divs[c]:
                    graph.add_edge(d, c, weight=c)
            matching = nx.max_weight_matching(graph)
            ub = sum(max(a, b) for a, b in matching)
            if ub == games[n]["score"]:
                matching_tight.add(n)
            elif ub < games[n]["score"]:
                print(f"matching bound: IMPOSSIBLE at n={n}: "
                      f"UB {ub} < recorded {games[n]['score']}")
        print(f"matching bound: recomputed independently for n=2..{limit} "
              f"({time.monotonic() - started:.0f}s); tight (score == UB, "
              f"certified outright) in {len(matching_tight)}/{limit - 1} games")

    # --- 4. certificate chain opt(n) <= n + opt(n-1) ----------------------
    certified = {n: n <= brute_verified or n in matching_tight for n in games}
    chain_links = 0
    for n in range(2, top + 1):
        if not certified[n] and certified[n - 1] and \
                games[n]["score"] == n + games[n - 1]["score"]:
            certified[n] = True
            chain_links += 1
    print(f"certificate chain: opt(n) = n + opt(n-1) certifies "
          f"{chain_links} further games given their predecessors")

    total_certified = sum(1 for n in games if certified[n])
    print(f"\nTOTAL independently certified: {total_certified}/{len(games)} "
          f"games of optimal.json")
    uncertified = [n for n in sorted(games) if not certified[n]]
    if uncertified:
        print(f"first uncertified games: {uncertified[:15]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
