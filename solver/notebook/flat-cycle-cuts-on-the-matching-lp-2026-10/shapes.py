#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The shapes of the shortest flat cycles of the maximal-factor graph on 1..N (default 1500), for the entry
solver/notebook/flat-cycle-cuts-on-the-matching-lp-2026-10/README.md.  Standard library only.

usage: shapes.py [N]

Between the numbers with k prime factors (movers) and those with k - 1 (factors), counted with multiplicity, for
every k: counts the pairs of movers that share two maximal factors (a 4-cycle; there are none), and lists the hexagons
and octagons from the factor graph (two factors joined by the mover that lists both) as the solver's `wall cycles`
does.  Each cycle is put in a canonical shape: its movers divided by their gcd X, the primes renamed a, b, c, ... in
order of first appearance in the sorted quotients.  Prints the shapes with their counts.
"""
import sys
from collections import Counter, defaultdict
from itertools import combinations
from math import gcd


def factorize(n):
    out, p = [], 2
    while p * p <= n:
        while n % p == 0:
            out.append(p)
            n //= p
        p += 1
    if n > 1:
        out.append(n)
    return out


def shape(movers):
    x = 0
    for u in movers:
        x = gcd(x, u)
    quotients = sorted(factorize(u // x) for u in movers)
    names = {}
    for q in quotients:
        for p in q:
            names.setdefault(p, "abcdefgh"[len(names)])
    return " ".join(sorted("".join(sorted(names[p] for p in q)) for q in quotients))


def main():
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 1500
    omega = {m: len(factorize(m)) for m in range(2, n + 1)}
    hexagons, octagons, doubled = Counter(), Counter(), 0
    for k in range(2, max(omega.values()) + 1):
        movers = [m for m in range(2, n + 1) if omega[m] == k]
        pair = {}
        adj = defaultdict(list)
        for u in movers:
            factors = sorted({u // p for p in set(factorize(u))})
            for a, b in combinations(factors, 2):
                if (a, b) in pair:
                    doubled += 1
                    continue
                pair[(a, b)] = u
                adj[a].append((b, u))
                adj[b].append((a, u))
        label = lambda a, b: pair.get((min(a, b), max(a, b)))
        for f1 in sorted(adj):
            around = adj[f1]
            for f2, u1 in around:
                for f3, u3 in around:
                    if f1 < f2 < f3:
                        u2 = label(f2, f3)
                        if u2 is not None and len({u1, u2, u3}) == 3:
                            hexagons[shape([u1, u2, u3])] += 1
            wedges = defaultdict(list)
            for f2, u1 in around:
                if f2 > f1:
                    for f3, u2 in adj[f2]:
                        if f3 > f1 and u2 != u1:
                            wedges[f3].append((f2, u1, u2))
            for f3, items in wedges.items():
                for i, (f2, u1, u2) in enumerate(items):
                    for f4, u4, u3 in items[i + 1 :]:
                        if f2 != f4 and len({u1, u2, u3, u4}) == 4:
                            octagons[shape([u1, u2, u3, u4])] += 1
    print(f"n = {n}: {doubled} pairs of movers sharing two factors")
    print(f"hexagons: {sum(hexagons.values())}, by shape: {dict(hexagons)}")
    print(f"octagons: {sum(octagons.values())}, by shape: {dict(octagons)}")


if __name__ == "__main__":
    main()
