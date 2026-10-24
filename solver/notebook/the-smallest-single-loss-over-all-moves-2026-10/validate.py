#!/usr/bin/env python3
"""Every recorded game plays, known unplayable sets fail, and peel agrees with a brute-force search on small boards.

Run from the top of the repository: PYTHONPATH=heuristics nice -n 19 python3 PATH/validate.py
"""

from __future__ import annotations

import random
import sys
from functools import lru_cache
from itertools import combinations

from swaplib import TOP, Playability, load_games


def brute_playable(n: int, selected: frozenset, divs) -> bool:
    """Search every order of the set on the board 1..n (pot as a frozenset), with memo."""
    @lru_cache(maxsize=None)
    def go(rest: frozenset, pot: frozenset) -> bool:
        if not rest:
            return True
        for c in rest:
            if c in pot:
                tax = [d for d in divs[c] if d in pot]
                if tax and go(rest - {c}, pot - set(tax) - {c}):
                    return True
        return False
    return go(selected, frozenset(range(1, n + 1)))


def main() -> int:
    pl = Playability(TOP)
    games = load_games()
    bad = 0
    for n, line in sorted(games.items()):
        moves = line["moves"]
        order = pl.order(n, set(moves))
        ok = (order is not None and pl.replay(n, order) and sorted(order) == sorted(moves) and pl.replay(n, moves)
              and sum(moves) == line["score"] and len(set(moves)) == len(moves) and pl.playable(n, set(moves)))
        if not ok:
            bad += 1
            print("RECORDED GAME FAILS", n)
    print(f"recorded games n = 1..{max(games)}: {len(games)} checked, {bad} fail (peel playable, peel's order replays "
          f"legally, the recorded order replays legally, score = sum of moves)")

    known = [
        ("2 and 3 on 3 (two primes, one 1)", 3, {2, 3}),
        ("2 and 4 on 4 (2 needs 1; 4 needs 2 or 1)", 4, {2, 4}),
        ("prime 7 and prime 5 on 10", 10, {5, 7}),
        ("{6, 10, 15} on 15 (matchable but cyclic)", 15, {6, 10, 15}),
        ("n=21's 145-sum set", 21, {10, 12, 14, 15, 16, 18, 19, 20, 21}),
        ("1 as a move", 5, {1, 4}),
        ("a move above the board", 5, {6}),
        ("recorded game of 100 plus a second prime (89 beside its prime 97)", 100, set(games[100]["moves"]) | {89}),
    ]
    for name, n, sel in known:
        res = pl.playable(n, sel)
        small = max(sel) <= 21 and 1 not in sel and max(sel) <= n
        brute = brute_playable(n, frozenset(sel), pl.divs) if small else None
        print(f"  {name}: peel {res}" + (f", brute force {brute}" if brute is not None else ""))
        assert not res
        assert brute in (None, False)

    # peel against a search over all orders: every subset on the boards up to 13, random subsets on boards 14..24
    rng = random.Random(1)
    checked = disagree = playable_count = 0
    for n in range(2, 14):
        for k in range(0, n):
            for sel in combinations(range(2, n + 1), k):
                s = set(sel)
                a, b = pl.playable(n, s), brute_playable(n, frozenset(s), pl.divs)
                playable_count += a
                checked += 1
                if a != b:
                    disagree += 1
                    print("DISAGREE", n, sorted(s), a, b)
    for n in range(14, 25):
        for _ in range(1500):
            s = {c for c in range(2, n + 1) if rng.random() < rng.choice((0.3, 0.45, 0.6))}
            a, b = pl.playable(n, s), brute_playable(n, frozenset(s), pl.divs)
            playable_count += a
            checked += 1
            if a != b:
                disagree += 1
                print("DISAGREE", n, sorted(s), a, b)
    print(f"peel vs brute-force search over orders: {checked} sets ({playable_count} playable), {disagree} disagree")
    return 1 if bad or disagree else 0


if __name__ == "__main__":
    sys.exit(main())
