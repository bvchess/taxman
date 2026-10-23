#!/usr/bin/env python3
"""Pairs of consecutive games that differ by one swapped move, and the searched ones among them.

A pair (n - 1, n) is a *swap* when the best games of n - 1 and n promote the same set of numbers and the moves of n
are the moves of n - 1 with one older move replaced by n.  The scan reads the chain's files in the order given, keeps
the first completed line of each n, and prints every pair whose game n was settled by a search, with the old move,
the new move n, their factorizations and the node counts of both games (a dash and the method for a game that was not
searched).

    scan_swaps.py [STRICT.jsonl ...]

With no argument it reads results/strict.jsonl of this repository, then the chain's file of the games not yet
committed (~/taxman_work/chain-data/results/strict.jsonl).
"""

import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
CHAIN_FILE = Path.home() / "taxman_work" / "chain-data" / "results" / "strict.jsonl"
DEFAULT_FILES = [REPO / "results" / "strict.jsonl", CHAIN_FILE]


def factorization(n):
    """The prime factorization of n as text, such as '2*11*113'."""
    primes = []
    p = 2
    while p * p <= n:
        while n % p == 0:
            primes.append(p)
            n //= p
        p += 1
    if n > 1 or not primes:
        primes.append(n)
    return "*".join(str(p) for p in primes)


def first_completed(files):
    """The first completed line of each n over the files, in order."""
    games = {}
    for path in files:
        with open(path) as lines:
            for line in lines:
                game = json.loads(line)
                if game.get("completed") and game["n"] not in games:
                    games[game["n"]] = game
    return games


def swaps(games):
    """Yield (n, old move, game n - 1, game n) for each pair that is a swap."""
    for n in sorted(games):
        before, game = games.get(n - 1), games[n]
        if before is None or set(before["promotions"]) != set(game["promotions"]):
            continue
        old, new = set(before["moves"]), set(game["moves"])
        if new - old == {n} and len(old - new) == 1:
            yield n, (old - new).pop(), before, game


def nodes_text(game):
    """The node count of a searched game, or a dash and the method of one that was not."""
    return f"{game['nodes']:,}" if game["method"] == "search" else f"- ({game['method']})"


def is_kind_of_4953(n, old):
    """Whether a move 2*11*p gives way to n = 3*13*q."""
    return old % 22 == 0 and n % 39 == 0


def best_sum(game):
    """The best promotion sum of a game."""
    return game["score"] - game["in_the_bag"]


def is_near_repeat(before, game):
    """Whether a game poses the problem of the game before it but for a swap: the same candidates, max_total, P*."""
    if before.get("setup") is None or game.get("setup") is None:
        return False
    return (before["setup"]["candidates"] == game["setup"]["candidates"]
            and before["setup"]["max_total"] == game["setup"]["max_total"]
            and best_sum(before) == best_sum(game))


def searched_nodes(games, low, high):
    """The nodes of the searched games from low to high."""
    return sum(game["nodes"] for n, game in games.items() if low <= n <= high and game["method"] == "search")


def main():
    files = [Path(arg) for arg in sys.argv[1:]] or DEFAULT_FILES
    games = first_completed(files)
    pairs = list(swaps(games))
    searched = [pair for pair in pairs if pair[3]["method"] == "search"]
    both = [pair for pair in searched if pair[2]["method"] == "search"]
    last = max(games)
    print(f"games {min(games)}..{last}; swaps {len(pairs)}; searched {len(searched)}; "
          f"both games searched {len(both)}")
    print("methods of game n over all swaps:", dict(Counter(pair[3]["method"] for pair in pairs)))
    kind = [pair for pair in searched if is_kind_of_4953(pair[0], pair[1])]
    print(f"searched swaps of the kind of 4953 (a move 2*11*p gives way to 3*13*q, marked *): {len(kind)}")
    swap_nodes = sum(pair[3]["nodes"] for pair in searched)
    total = searched_nodes(games, 1, last)
    print(f"nodes of the games n of the searched swaps: {swap_nodes:,} of {total:,} searched "
          f"({swap_nodes / total:.1%})")
    low = 4850
    window = searched_nodes(games, low, last)
    near = [n for n in range(low, last + 1)
            if games[n]["method"] == "search" and is_near_repeat(games[n - 1], games[n])]
    near_nodes = sum(games[n]["nodes"] for n in near)
    print(f"near repeats from {low} to {last}: {near}, {near_nodes:,} of {window:,} searched nodes "
          f"({near_nodes / window:.1%})")
    print()
    print(f"{'n-1':>5} {'old move':>8} {'= factors':<12} {'n':>5} {'= factors':<12} "
          f"{'nodes of n-1':>20} {'nodes of n':>14}")
    for n, old, before, game in searched:
        mark = " *" if is_kind_of_4953(n, old) else ""
        print(f"{n - 1:>5} {old:>8} {factorization(old):<12} {n:>5} {factorization(n):<12} "
              f"{nodes_text(before):>20} {nodes_text(game):>14}{mark}")


if __name__ == "__main__":
    main()
