#!/usr/bin/env python3
"""Which move a game gives up when it differs from the game before by one move and the promotions are the same.

For every n of the proven games whose recorded optimal game is the recorded game of n - 1 with n added and one move x
dropped, the promotions unchanged (the swaps of scan_swaps.py), it compares x with the *shadow* of n: n with each prime
factor replaced by the prime just below it, with multiplicity (so the shadow of a prime is the prime before it).  For
even n, whose factor 2 has no prime below it, it compares x with n/2.  It prints the counts and every exception.

Run from the top of the repository: python3 solver/notebook/search/the-corner-of-a-near-repeat-2026-10/shadow.py
"""

import json
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
CHAIN_FILE = Path.home() / "taxman_work" / "chain-data" / "results" / "strict.jsonl"


def factorization(n):
    factors, p = [], 2
    while p * p <= n:
        while n % p == 0:
            factors.append(p)
            n //= p
        p += 1
    if n > 1:
        factors.append(n)
    return factors


def prime_below(p):
    q = p - 1
    while len(factorization(q)) != 1:
        q -= 1
    return q


def shadow(n):
    product = 1
    for p in factorization(n):
        product *= prime_below(p)
    return product


def load_games():
    games = {}
    for path in (REPO / "results" / "strict.jsonl", CHAIN_FILE):
        if not path.exists():
            continue
        for line in open(path):
            game = json.loads(line)
            n = game.get("n")
            if game.get("completed") and game.get("moves") and n not in games:
                games[n] = (frozenset(game["moves"]), frozenset(game.get("promotions") or []), game.get("method"))
    return games


def main():
    games = load_games()
    counts, exceptions = Counter(), []
    for n in sorted(games):
        if n - 1 not in games:
            continue
        (before, promoted_before, _), (after, promoted_after, method) = games[n - 1], games[n]
        gone, new = before - after, after - before
        if new != {n} or len(gone) != 1 or promoted_before != promoted_after:
            continue
        x = next(iter(gone))
        searched = "searched" if method == "search" else "not searched"
        if n % 2:
            kind = "prime n, x the prime before" if len(factorization(n)) == 1 else "odd composite n, x its shadow"
            ok = x == shadow(n)
        else:
            kind, ok = "even n, x = n/2", 2 * x == n
        counts[(kind if ok else "exception", searched)] += 1
        if not ok:
            exceptions.append((n, x, factorization(n), factorization(x)))
    for (kind, searched), count in sorted(counts.items()):
        print(f"{kind:32} {searched:13} {count:5}")
    print("even n:", sorted(n for n in games if n % 2 == 0 and n - 1 in games
                            and games[n][0] - games[n - 1][0] == {n} and len(games[n - 1][0] - games[n][0]) == 1
                            and games[n][1] == games[n - 1][1]))
    print("exceptions:", exceptions if exceptions else "none")


if __name__ == "__main__":
    main()
