#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Franklín and Moniot's characterization of playability against the parity matroids, by brute force.

docs/theory/franklin-moniot.md, has the statements and the proofs this module checks.  A *payment
scheme* of a pick set S pays each member s with a distinct maximal factor of s outside S: a matching of the
maximal-factor graph whose upper ends are exactly S.  per(S, T) counts the schemes whose payments are exactly the set
T, the permanent of S's rows on the columns T.  The section proves

    S is matchable (the relaxation of the F-M upper bound)     iff  per(S, T) >= 1 for some T,
    S is independent (the lemma of Theorem 10)                 iff  per(S, T) is odd for some T,
    S is playable (F-M Theorem 1; THEORY.md, Theorem 1)        iff  per(S, T) == 1 for some T,

so playable => independent => matchable, and an independent set that is not playable is an *odd core*.

* `exhaustive` checks every pick set of the boards up to --max-n.  Playability is decided by the rule alone (some
  order of S is legal); Franklín and Moniot's condition literally (a scheme with no flat alternating cycle in the
  maximal-factor graph); independence twice (the lemma's family, and the two parity matroids of Theorem 10); and
  each is compared with the permanents and with `core.peel`.  It also prints, board by board, the heaviest playable,
  independent and matchable sets: opt(n), the parity bound and the F-M bound.  Below 30 every number has at most two
  distinct primes, so these boards test the graph-like regime only.
* `sampled` makes the same comparisons on random dense sets of the boards 30..--sample-max, grown around members
  with three or more distinct primes, where the hierarchy is strict.
* `odd_cores` searches the boards --core-boards for odd cores of at most --core-size members.  Every odd core has a
  member with three or more distinct primes whose maximal factors are all outside it (THEORY.md), so the search
  enumerates the sets that have one.  The section proves that no board below 45 has an odd core.
* `example` works the n = 21 sets in both languages.

The run of the section (a few minutes):
    python3 -m evaluation.fm_relation
"""

from __future__ import annotations

import argparse
import itertools
import random
import sys
import time
from collections import Counter
from typing import Dict, FrozenSet, Iterable, List, NamedTuple, Optional, Sequence, Set

from core import Infeasible, maximal_factor_lists, peel, smallest_prime_factors
from evaluation.bitpot import bits, divisor_masks, mask_of

Scheme = Dict[int, int]  # member -> its payment


class Board:
    """The numbers 1..n with what the checks need: maximal factors, Ω, divisor masks."""

    def __init__(self, n: int) -> None:
        self.n = n
        self.half = n // 2
        self.mf: List[Set[int]] = [set(fs) for fs in maximal_factor_lists(n)]
        spf = smallest_prime_factors(n)
        self.omega = [0] * (n + 1)
        for c in range(2, n + 1):
            self.omega[c] = self.omega[c // spf[c]] + 1
        self.proper = divisor_masks(n)  # bit d for every proper divisor d of c
        self.closure = [self.proper[c] | 1 << c for c in range(n + 1)]  # ... and c itself


# --- playability by the rule alone --------------------------------------------------------------------------------


def playable_table(board: Board, members: Sequence[int]) -> bytearray:
    """table[mask] = 1 iff the members selected by mask can all be played, in some order.

    After any legal play of a set P the numbers gone from the pot are exactly the divisors of P's members (each move
    removes its pick and the pick's divisors still there), whatever the order.  So P + s is playable iff, for some
    s, P is playable, s is still in the pot afterwards, and s has a proper divisor there: s can be played last.
    (A subset of a playable set is playable: dropping a move only leaves more in the pot.  A mask with an unplayable
    one-smaller subset is therefore unplayable, a shortcut and nothing more.)
    """
    k = len(members)
    removed = [0] * (1 << k)
    table = bytearray(1 << k)
    table[0] = 1
    for mask in range(1, 1 << k):
        low = mask & -mask
        removed[mask] = removed[mask ^ low] | board.closure[members[low.bit_length() - 1]]
        for i in bits(mask):
            rest = mask ^ (1 << i)
            if not table[rest]:
                break
            s = members[i]
            if not removed[rest] >> s & 1 and board.proper[s] & ~removed[rest]:
                table[mask] = 1
                break
    return table


def playable(board: Board, picks: Iterable[int]) -> bool:
    members = sorted(picks)
    return bool(playable_table(board, members)[(1 << len(members)) - 1])


# --- Franklín and Moniot's condition, literally -------------------------------------------------------------------


def schemes(board: Board, picks: FrozenSet[int]) -> List[Scheme]:
    """Every payment scheme of the picks: each pays with a distinct maximal factor of its own outside the picks."""
    options = {s: sorted(board.mf[s] - picks) for s in picks}
    order = sorted(picks, key=lambda s: len(options[s]))
    found: List[Scheme] = []
    pay: Scheme = {}
    used: Set[int] = set()

    def extend(i: int) -> None:
        if i == len(order):
            found.append(dict(pay))
            return
        s = order[i]
        for f in options[s]:
            if f not in used:
                used.add(f)
                pay[s] = f
                extend(i + 1)
                used.discard(f)
                del pay[s]

    extend(0)
    return found


def has_scheme(board: Board, picks: FrozenSet[int]) -> bool:
    """Whether the picks have a payment scheme (Kuhn's augmenting paths)."""
    owner: Dict[int, int] = {}  # factor -> the pick it pays for

    def augment(s: int, seen: Set[int]) -> bool:
        for f in board.mf[s] - picks:
            if f not in seen:
                seen.add(f)
                if f not in owner or augment(owner[f], seen):
                    owner[f] = s
                    return True
        return False

    return all(augment(s, set()) for s in picks)


def has_directed_cycle(succ: Dict[object, List[object]]) -> bool:
    """Kahn's algorithm: a digraph is acyclic iff repeatedly deleting in-degree-0 vertices empties it."""
    indegree: Counter = Counter()
    for v in succ:
        indegree[v] += 0
        for w in succ[v]:
            indegree[w] += 1
    ready = [v for v in indegree if indegree[v] == 0]
    seen = 0
    while ready:
        v = ready.pop()
        seen += 1
        for w in succ.get(v, ()):
            indegree[w] -= 1
            if indegree[w] == 0:
                ready.append(w)
    return seen < len(indegree)


def flat_alternating_cycle(board: Board, pay: Scheme) -> bool:
    """Whether the matching {(pay[s], s)} has a flat alternating cycle in the maximal-factor graph.

    The definition, literally: a cycle of the graph (edges join f and c when c/f is prime) whose vertices lie in two
    adjacent ranks, Ω = k and k + 1, and whose edges alternate between the matching and the rest.  Every vertex of an
    alternating cycle has its matched edge on the cycle, so for ranks (k, k + 1) only the matched pairs with upper
    end of rank k + 1 can carry one, and among them an alternating cycle is a directed cycle when the matched edges
    point down and the others up.
    """
    owner = {f: s for s, f in pay.items()}
    for k in {board.omega[f] for f in owner}:
        tops = [s for s in pay if board.omega[s] == k + 1]
        succ: Dict[object, List[object]] = {}
        for s in tops:
            succ[("top", s)] = [("bottom", pay[s])]
            succ[("bottom", pay[s])] = [("top", t) for t in tops if t != s and pay[s] in board.mf[t]]
        if has_directed_cycle(succ):
            return True
    return False


# --- independence over GF(2) ---------------------------------------------------------------------------------------


def gf2_independent(vectors: Iterable[int]) -> bool:
    """Whether the bitmask vectors are linearly independent over GF(2)."""
    basis: Dict[int, int] = {}  # leading bit -> basis vector
    for v in vectors:
        while v:
            top = v.bit_length() - 1
            if top not in basis:
                basis[top] = v
                break
            v ^= basis[top]
        else:
            return False
    return True


def lemma_independent(board: Board, picks: Iterable[int]) -> bool:
    """The family of Theorem 10's lemma: v(s) for every pick, e_s for every pick s <= n/2."""
    picks = list(picks)
    family = [mask_of(board.mf[s]) for s in picks] + [1 << s for s in picks if s <= board.half]
    return gf2_independent(family)


def parity_independent(board: Board, picks: Iterable[int]) -> bool:
    """Independence in both of Theorem 10's matroids M_0 and M_1 (a coloop is independent of everything)."""
    picks = list(picks)
    for q in (0, 1):
        rows = [mask_of(board.mf[s]) if board.omega[s] % 2 != q else 1 << s
                for s in picks if board.omega[s] % 2 != q or s <= board.half]
        if not gf2_independent(rows):
            return False
    return True


def peel_accepts(board: Board, picks: FrozenSet[int]) -> bool:
    factors = {f for s in picks for f in board.mf[s]} - picks
    try:
        peel(picks, factors, {s: board.mf[s] for s in picks})
    except Infeasible:
        return False
    return True


# --- the checks ----------------------------------------------------------------------------------------------------


class Verdict(NamedTuple):
    playable: bool  # by the rule alone
    fm: bool  # a payment scheme with no flat alternating cycle (F-M Theorem 1's condition)
    unique: bool  # per(S, T) == 1 for some T
    peel: bool  # core.peel accepts
    independent: bool  # the family of Theorem 10's lemma
    odd: bool  # per(S, T) odd for some T
    parity: bool  # independent in both parity matroids
    matchable: bool  # some payment scheme

    def consistent(self) -> bool:
        return (self.playable == self.fm == self.unique == self.peel
                and self.independent == self.odd == self.parity
                and (self.independent or not self.playable) and (self.matchable or not self.independent))


def verdict(board: Board, picks: FrozenSet[int], is_playable: bool) -> Verdict:
    found = schemes(board, picks)
    per = Counter(frozenset(p.values()) for p in found)
    return Verdict(playable=is_playable,
                   fm=any(not flat_alternating_cycle(board, p) for p in found),
                   unique=any(c == 1 for c in per.values()),
                   peel=peel_accepts(board, picks),
                   independent=lemma_independent(board, picks),
                   odd=any(c % 2 for c in per.values()),
                   parity=parity_independent(board, picks),
                   matchable=bool(found))


def tally(counts: Counter, picks: FrozenSet[int], v: Verdict) -> bool:
    """Count a set's verdict; report it if it breaks the hierarchy."""
    counts["sets"] += 1
    counts["playable"] += v.playable
    counts["independent"] += v.independent
    counts["matchable"] += v.matchable
    counts["matchable, dependent"] += v.matchable and not v.independent
    counts["odd cores"] += v.independent and not v.playable
    if not v.consistent():
        print(f"  BROKEN at {sorted(picks)}: {v}")
        return False
    return True


def summary(counts: Counter) -> str:
    return (f"{counts['sets']:,} sets: {counts['playable']:,} playable, {counts['independent']:,} independent, "
            f"{counts['matchable']:,} matchable; {counts['matchable, dependent']:,} matchable but dependent, "
            f"{counts['odd cores']:,} independent but unplayable (odd cores)")


def exhaustive(max_n: int) -> bool:
    """Every pick set of {2..max_n}: the hierarchy, the literal F-M condition, peel, and the three bounds."""
    started = time.time()
    board = Board(max_n)
    members = list(range(2, max_n + 1))
    play = playable_table(board, members)
    counts: Counter = Counter()
    smallest: Optional[List[int]] = None  # the smallest matchable but dependent set
    best = {key: [0] * (max_n + 1) for key in ("playable", "independent", "matchable")}
    ok = True
    for mask in range(1 << len(members)):
        picks = frozenset(members[i] for i in bits(mask))
        v = verdict(board, picks, bool(play[mask]))
        ok &= tally(counts, picks, v)
        if v.matchable and not v.independent:
            key = sorted(picks)
            if smallest is None or (key[-1], len(key), key) < (smallest[-1], len(smallest), smallest):
                smallest = key
        top = max(picks, default=0)
        for name in best:
            if getattr(v, name) and sum(picks) > best[name][top]:
                best[name][top] = sum(picks)
    print(f"every pick set of {{2..{max_n}}} ({time.time() - started:.0f} s): {summary(counts)}")
    print(f"  playable = F-M's condition = (per = 1 for some T) = peel, and independent = (per odd for some T) = "
          f"independent in both parity matroids, on every set: {'yes' if ok else 'NO'}")
    print(f"  the smallest matchable but dependent set: {smallest}")
    print("  board   opt(n)   parity bound   F-M bound   (the heaviest playable, independent, matchable set)")
    running = {name: 0 for name in best}
    for n in range(2, max_n + 1):
        for name in best:
            running[name] = max(running[name], best[name][n])
        print(f"  {n:5}   {running['playable']:6}   {running['independent']:12}   {running['matchable']:9}")
    return ok


def sampled(trials: int, boards: range, max_size: int, seed: int) -> bool:
    """Random dense pick sets on larger boards, where members with three or more primes make the hierarchy strict.

    Each set grows from a random member with at least three maximal factors by adding random numbers that share a
    maximal factor with the set so far and keep it matchable (until it has the drawn size or no such number is left):
    sets that are matchable and core-like, the regime in which the three conditions can differ.
    """
    started = time.time()
    rng = random.Random(seed)
    counts: Counter = Counter()
    ok = True
    for board_n in boards:
        board = Board(board_n)
        hubs = [c for c in range(2, board_n + 1) if len(board.mf[c]) >= 3]
        listers: Dict[int, List[int]] = {}
        for c in range(2, board_n + 1):
            for f in board.mf[c]:
                listers.setdefault(f, []).append(c)
        for _ in range(trials):
            picks = {rng.choice(hubs)}
            size = rng.randint(2, max_size)
            while len(picks) < size:
                neighbours = sorted({c for s in picks for f in board.mf[s] for c in listers[f]} - picks)
                neighbours = [c for c in neighbours if has_scheme(board, frozenset(picks | {c}))]
                if not neighbours:
                    break
                picks.add(rng.choice(neighbours))
            frozen = frozenset(picks)
            ok &= tally(counts, frozen, verdict(board, frozen, playable(board, frozen)))
    print(f"sampled dense sets on the boards {boards.start}..{boards.stop - 1} ({time.time() - started:.0f} s): "
          f"{summary(counts)}; the hierarchy and the equivalences on every set: {'yes' if ok else 'NO'}")
    return ok


# --- odd cores -----------------------------------------------------------------------------------------------------


def odd_cores(n: int, max_size: int) -> List[List[int]]:
    """The minimal odd cores of at most max_size members on the board n: unplayable, independent, and every subset
    playable.  Each has a member h with at least three maximal factors, none of them in the core."""
    board = Board(n)
    hubs = [h for h in range(2, n + 1) if len(board.mf[h]) >= 3]
    found: Set[FrozenSet[int]] = set()
    for hub in hubs:
        others = [c for c in range(2, n + 1) if c != hub and c not in board.mf[hub]]
        for size in range(max_size):
            for rest in itertools.combinations(others, size):
                picks = (hub,) + rest
                if not lemma_independent(board, picks):
                    continue
                members = sorted(picks)
                table = playable_table(board, members)
                full = (1 << len(members)) - 1
                if not table[full] and all(table[full ^ (1 << i)] for i in range(len(members))):
                    found.add(frozenset(picks))
    return sorted((sorted(core) for core in found), key=lambda core: (len(core), core))


def describe(board: Board, picks: Iterable[int]) -> None:
    """Print a set's payment schemes grouped by payment set, with their flat alternating cycles."""
    picks = frozenset(picks)
    found = schemes(board, picks)
    per = Counter(frozenset(p.values()) for p in found)
    print(f"  {sorted(picks)}: playable {playable(board, picks)}, independent {lemma_independent(board, picks)}, "
          f"matchable {bool(found)}")
    for payments, count in sorted(per.items(), key=lambda item: sorted(item[0])):
        flat = sum(flat_alternating_cycle(board, p) for p in found if frozenset(p.values()) == payments)
        print(f"    payment set {sorted(payments)}: per = {count}, {flat} of the {count} schemes with a flat "
              f"alternating cycle")


def example() -> None:
    board = Board(21)
    cycle = (10, 14, 15, 21)
    print("n = 21, the 4-cycle over the primes 2, 3, 5, 7:")
    describe(board, cycle)
    total = 0
    for s in cycle:
        total ^= mask_of(board.mf[s])
    print(f"    v(10) + v(14) + v(15) + v(21) over GF(2): {sorted(bits(total))} (empty: a dependency)")
    for pay in schemes(board, frozenset(cycle)):
        precedence = {a: sorted(b for b in cycle if b != a and b % pay[a] == 0) for a in cycle}
        print(f"    scheme {pay}: precedence {precedence}")
    print("n = 21, the matchable set of the F-M bound (145):")
    describe(board, (10, 12, 14, 15, 16, 18, 19, 20, 21))


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--max-n", type=int, default=22, help="largest board checked exhaustively (default 22)")
    parser.add_argument("--sample-max", type=int, default=120, help="largest board sampled (default 120)")
    parser.add_argument("--samples", type=int, default=200, help="random sets per sampled board (default 200)")
    parser.add_argument("--sample-size", type=int, default=12, help="largest random set (default 12)")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--core-boards", type=int, nargs="*", default=[44, 45],
                        help="boards searched for odd cores (default 44 45)")
    parser.add_argument("--core-size", type=int, default=6, help="largest odd core searched for (default 6)")
    args = parser.parse_args(argv)
    ok = exhaustive(args.max_n)
    ok &= sampled(args.samples, range(30, args.sample_max + 1), args.sample_size, args.seed)
    for n in args.core_boards:
        started = time.time()
        cores = odd_cores(n, args.core_size)
        print(f"board {n}: {len(cores)} minimal odd cores of at most {args.core_size} members "
              f"({time.time() - started:.0f} s)")
        board = Board(n)
        for core in cores:
            describe(board, core)
    example()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
