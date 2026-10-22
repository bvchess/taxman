#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Prime exchanges that are symmetries of a root, and those that nearly are.

usage: nearpairs.py ROOT_INSTANCE.jsonl [MAX_DEFECT]

For primes p < q, the exchange of p and q maps a number p^a q^b r to p^b q^a r.  It is a symmetry of the root
(Theorem 13) when it maps every frame's moves onto its moves, its factors onto its factors and the candidates onto
themselves.  The *defect* of a pair is the numbers of the frames and candidates whose image is not in the same role
(the same frame, as a move or a factor, and a candidate exactly when the number is).  Pairs with no candidate divisible
by pq, p^2 or q^2 only, as the rule needs.
"""

import json
import sys
from collections import Counter


def primes_upto(n):
    sieve = bytearray([1]) * (n + 1)
    sieve[0:2] = b"\x00\x00"
    for i in range(2, int(n ** 0.5) + 1):
        if sieve[i]:
            sieve[i * i::i] = bytearray(len(sieve[i * i::i]))
    return [i for i in range(n + 1) if sieve[i]]


def swap(x, p, q):
    a = b = 0
    r = x
    while r % p == 0:
        r //= p
        a += 1
    while r % q == 0:
        r //= q
        b += 1
    return r * p ** b * q ** a


def main():
    inst = next(json.loads(line) for line in open(sys.argv[1]) if '"instance"' in line)
    max_defect = int(sys.argv[2]) if len(sys.argv) > 2 else 12
    n = inst["n"]
    role = {}
    for h, (moves, factors, _) in enumerate(inst["frames"]):
        for x in moves:
            role[x] = ("move", h)
        for x in factors:
            role[x] = ("factor", h)
    cands = {x for x, _ in inst["cands"]}
    chosen = set(inst["chosen"])
    numbers = set(role) | cands
    ps = [p for p in primes_upto(n // 2) if p >= 5]
    rows = []
    for i, p in enumerate(ps):
        for q in ps[i + 1:]:
            if p * q > n and any(x % (p * q) == 0 for x in cands):
                pass
            if any(x % (p * q) == 0 or x % (p * p) == 0 or x % (q * q) == 0 for x in cands):
                continue
            touched = [x for x in numbers if x % p == 0 or x % q == 0]
            if not touched:
                continue
            defect = []
            for x in touched:
                y = swap(x, p, q)
                if role.get(x) != role.get(y) or ((x in cands) != (y in cands)):
                    defect.append(x)
            if len(defect) <= max_defect:
                in_c = [x for x in touched if x in cands]
                rows.append((len(defect), p, q, len(in_c), sum(1 for x in in_c if x in chosen), sorted(defect)))
    rows.sort()
    exact = [r for r in rows if r[0] == 0]
    print(f"n {n}: {len(exact)} exact symmetric pairs; pairs by defect size: {sorted(Counter(r[0] for r in rows).items())}")
    for d, p, q, nc, nh, defect in rows[:60]:
        print(f"  defect {d:2d}  ({p}, {q})  candidates moved {nc}, in the heaviest set {nh}  defect {defect[:8]}")


if __name__ == "__main__":
    main()
