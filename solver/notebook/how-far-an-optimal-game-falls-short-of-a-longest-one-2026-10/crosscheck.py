#!/usr/bin/env python3
"""Cross-check the longest-games run without the prover's code.

For every game n of a range, read the recorded game of results/strict.jsonl, replay it by the rules, and compare its
number of moves with the matching number of the board's maximal-factor graph, computed from scratch by Hopcroft-Karp.
Where the two are equal the recorded game is a longest game.  A sample of the replays is repeated with the slow
reference checker of heuristics/core.py.  With --run FILE (the output of the prover's `moves`), the matching numbers
and the move counts are compared with the prover's.

Run from the top of the repository:
    PYTHONPATH=heuristics nice -n 19 python3 PATH/crosscheck.py --lo 2 --hi 4949 --run FILE
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from core import check_sequence, smallest_prime_factors

TOP = Path(__file__).resolve().parents[3]


def load_games(path: Path) -> dict[int, dict]:
    """The first line of each n, which must run 1, 2, 3, ... without a gap."""
    games: dict[int, dict] = {}
    for line in path.read_text().splitlines():
        record = json.loads(line)
        games.setdefault(record["n"], record)
    assert sorted(games) == list(range(1, len(games) + 1)), "the chain has a gap"
    return games


def prime_factors(m: int, spf: list[int]) -> set[int]:
    found = set()
    while m > 1:
        found.add(spf[m])
        m //= spf[m]
    return found


class Board:
    """The divisors and the maximal-factor graph of the board 1..top, of which the board 1..n is a prefix."""

    def __init__(self, top: int):
        spf = smallest_prime_factors(top)
        self.top = top
        self.divs: list[list[int]] = [[] for _ in range(top + 1)]
        for d in range(1, top // 2 + 1):
            for m in range(2 * d, top + 1, d):
                self.divs[m].append(d)
        omega = [0] * (top + 1)
        for m in range(2, top + 1):
            omega[m] = omega[m // spf[m]] + 1
        # The edges join m to m/p for each prime p dividing m.  The side of a number is the parity of its Omega, the
        # number of its prime factors with multiplicity: m and m/p differ by one prime factor.
        self.left = [m for m in range(1, top + 1) if omega[m] % 2 == 0]
        self.adj: list[list[int]] = [[] for _ in range(top + 1)]
        for m in range(2, top + 1):
            for p in prime_factors(m, spf):
                for a, b in ((m, m // p), (m // p, m)):
                    if omega[a] % 2 == 0:
                        self.adj[a].append(b)
        for neighbors in self.adj:
            neighbors.sort()

    def replay(self, moves: list[int]) -> int | None:
        """The score of the moves played in order, or None if one is illegal.  A move is a number in the pot with a
        proper divisor in the pot; it takes the number and all its proper divisors out."""
        pot = bytearray(b"\x01" * (self.top + 1))
        score = 0
        for c in moves:
            if not pot[c]:
                return None
            taxed = [d for d in self.divs[c] if pot[d]]
            if not taxed:
                return None
            for d in taxed:
                pot[d] = 0
            pot[c] = 0
            score += c
        return score

    def matching_number(self, n: int) -> int:
        """The size of a maximum matching of the maximal-factor graph of the board 1..n (Hopcroft-Karp)."""
        left = [u for u in self.left if u <= n]
        adj = {u: [v for v in self.adj[u] if v <= n] for u in left}
        mate = [0] * (n + 1)
        size = 0
        for u in left:
            for v in adj[u]:
                if not mate[v]:
                    mate[u], mate[v] = v, u
                    size += 1
                    break
        while True:
            # Layer the left vertices by their alternating distance from the free ones.
            dist = {u: 0 for u in left if not mate[u]}
            roots = list(dist)
            queue = list(dist)
            reached = False
            for u in queue:
                for v in adj[u]:
                    w = mate[v]
                    if not w:
                        reached = True
                    elif w not in dist:
                        dist[w] = dist[u] + 1
                        queue.append(w)
            if not reached:
                return size
            # Augment along paths that go one layer deeper at each step, depth first.
            at = {u: 0 for u in dist}
            for root in roots:
                path = [root]
                while path:
                    u = path[-1]
                    if at[u] == len(adj[u]):
                        dist[u] = -1
                        path.pop()
                        continue
                    v = adj[u][at[u]]
                    at[u] += 1
                    w = mate[v]
                    if not w:
                        for a in path:
                            b = adj[a][at[a] - 1]
                            mate[a], mate[b] = b, a
                        size += 1
                        break
                    if dist.get(w) == dist[u] + 1:
                        path.append(w)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--lo", type=int, default=2)
    parser.add_argument("--hi", type=int, required=True)
    parser.add_argument("--run", type=Path, help="the prover's `moves` output to compare with")
    parser.add_argument("--sample", type=int, default=97, help="also replay every k-th game with core.check_sequence")
    args = parser.parse_args()
    games = load_games(TOP / "results" / "strict.jsonl")
    board = Board(args.hi)
    run = {}
    if args.run:
        run = {r["n"]: r for r in map(json.loads, args.run.read_text().splitlines())}
    started = time.time()
    illegal, shorter, longer, disagree, sampled = [], [], [], [], 0
    for n in range(args.lo, args.hi + 1):
        game = games[n]
        moves = game["moves"]
        assert len(set(moves)) == len(moves), n
        if board.replay(moves) != game["score"]:
            illegal.append(n)
        nu = board.matching_number(n)
        if len(moves) < nu:
            shorter.append((n, nu - len(moves)))
        if len(moves) > nu:
            longer.append(n)
        if run and (run[n]["nu"], run[n]["recorded_moves"]) != (nu, len(moves)):
            disagree.append(n)
        if (n - args.lo) % args.sample == 0 or n == args.hi:
            assert check_sequence(n, moves) == game["score"], n
            sampled += 1
    print(f"games {args.lo}..{args.hi}: {args.hi - args.lo + 1}")
    print(f"recorded game illegal or at another score: {illegal or 'none'}; also replayed by core.check_sequence: "
          f"{sampled} games")
    print(f"recorded game with more moves than the matching number: {longer or 'none'}")
    print(f"recorded game with fewer moves than the matching number (n, by how many): {shorter or 'none'}")
    if run:
        print(f"disagreements with {args.run.name} on the matching number or the move count: {disagree or 'none'}")
    print(f"{time.time() - started:.0f} s", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
