"""Executable specification of the README's "Greedy, final form" pseudocode.

Written for clarity, not speed; the fast bit-identical implementation
(incremental matching + Kuhn augmenting paths) lives in approx.py's greedy().
"""


class Infeasible(Exception):
    """Raised when solve_mini cannot pay every member from the factor pool."""


def proper_divisors(c):
    return [d for d in range(1, c) if c % d == 0]


def solve_mini(members, factors, factors_of):
    if not members:
        return [], {}

    for c in members:
        remaining = factors_of[c] & factors
        if len(remaining) == 1:
            (f,) = remaining
            seq, pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return [c] + seq, pay

    for f in factors:
        payers = [c for c in members if f in factors_of[c]]
        if len(payers) == 1:
            c = payers[0]
            seq, pay = solve_mini(members - {c}, factors - {f}, factors_of)
            pay[c] = f
            return seq + [c], pay

    raise Infeasible(f"cannot select every member of {members} using {factors}")


def playable(s_set):
    factors = set()
    for s in s_set:
        factors |= set(proper_divisors(s))
    factors -= s_set

    factors_of = {s: set(proper_divisors(s)) & factors for s in s_set}

    try:
        _, pay = solve_mini(s_set, factors, factors_of)
    except Infeasible:
        return None
    return pay


def ordered(s_set, pay):
    remaining = set(s_set)
    placed = []
    while remaining:
        for a in remaining:
            blocked = any(
                b != a
                and ((pay.get(b) is not None and a % pay[b] == 0) or a % b == 0)
                for b in remaining
            )
            if not blocked:
                placed.append(a)
                remaining.discard(a)
                break
        else:
            raise RuntimeError(
                f"schedulability conjecture violated: no valid ordering "
                f"exists for {remaining}"
            )
    return placed


def greedy(n):
    s = set()
    for c in range(n, 1, -1):
        if playable(s | {c}) is not None:
            s.add(c)
    pay = playable(s)
    return ordered(s, pay)


if __name__ == "__main__":
    import sys

    sys.setrecursionlimit(10000)
    result = greedy(int(sys.argv[1]))
    print(result)
    print(sum(result))
