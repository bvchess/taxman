"""Core polynomial-time reduction for the upper half of a Taxman game.

The reduction is played with a set of potential selections C and a set of
maximal factors F, where the maximal factor graph is bipartite: every
number is either a selection or a factor, never both.
(See https://github.com/bvchess/taxman/wiki/Taxman-Mini for the wiki's
description of this reduction, "Taxman Mini", and the solve_mini /
optimize_mini procedures implemented below.)

For a regular Taxman game N, the numbers greater than N/2 are exactly the
source nodes of the maximal factor graph (no number in the game is a multiple
of them), and all of their maximal factors are <= N/2.  So

    C = { c : N/2 < c <= N }
    F = union of the maximal factors of the members of C

forms a valid instance of the factor game. The theory tested by this project
is that optimize_mini/solve_mini applied to this game yield exactly the
numbers greater than N/2 in the optimal solution to game N, along with a
valid order in which to select them.

Three well-known ideas underlie this file, wearing number-theory
clothes.  (1) *Modeling*: the game becomes a bipartite graph — picks on
one side, candidate tax payments on the other, an edge where payment
divides pick — so "can everything be paid?" becomes a matching
question.  (2) *Degree-1 peeling*: solve_mini repeatedly makes only
forced moves (a vertex with one live edge has no choice), maintained
with worklists exactly like Kahn's topological sort keeps its
in-degree-zero queue; each edge is touched O(1) times, so it runs in
O(V + E) — and E is tiny, since a number has only as many maximal
factors as distinct primes (average ln ln n ~= 2).  (3) *Topological
order*: order_for_real_game turns the payment assignment into a legal
play sequence by topologically sorting a precedence DAG.  One warning:
solve_mini is NOT equivalent to checking
that a matching exists (Hall's condition).  There are sets where a
perfect payment matching exists but every such matching is impossible
to schedule — solve_mini's forced-move discipline rejects those too.
The smallest example lives at n=21; see the README's ledger.
"""

from __future__ import annotations

from typing import Dict, Iterable, List, Sequence, Set, Tuple


class Infeasible(Exception):
    """Raised by solve_mini when not all members of C can be selected."""


def smallest_prime_factors(limit: int) -> List[int]:
    """Sieve: spf[n] is the smallest prime factor of n, for 0 <= n <= limit.

    The Sieve of Eratosthenes, storing a witness instead of a boolean:
    spf[n] == n exactly when n is prime, and repeatedly dividing n by
    spf[n] factors it in O(log n) with no trial division.  Build cost is
    the classic O(n log log n).
    """
    spf = list(range(limit + 1))
    for p in range(2, int(limit**0.5) + 1):
        if spf[p] == p:  # p is prime
            for multiple in range(p * p, limit + 1, p):
                if spf[multiple] == multiple:
                    spf[multiple] = p
    return spf


def prime_factors(n: int, spf: Sequence[int]) -> Set[int]:
    """The distinct prime factors of n."""
    primes = set()
    while n > 1:
        p = spf[n]
        primes.add(p)
        while n % p == 0:
            n //= p
    return primes


def maximal_factors(n: int, spf: Sequence[int]) -> Set[int]:
    """The maximal factors of n: every f such that n/f is prime.

    If n is prime, its only maximal factor is 1.  1 itself has none.
    """
    return {n // p for p in prime_factors(n, spf)}


def solve_mini(
    selections: Iterable[int],
    factors: Iterable[int],
    mf: Dict[int, Set[int]],
) -> Tuple[List[int], Dict[int, int]]:
    """Order the selections of a factor game so every one can be taken.

    Implements the recursive procedure from the wiki iteratively:

        solve_mini(C, F):
            if C is empty: return []
            if some c in C has only a single factor f in F:
                return [c] + solve_mini(C - {c}, F - {f})
            if some f in F is a maximal factor of only one c in C:
                return solve_mini(C - {c}, F - {f}) + [c]
            ERROR: cannot select all members of C using F

    Returns (sequence, matching) where matching maps each selection to the
    factor it pays as tax.  Raises Infeasible if no ordering exists.

    Implementation note (why this is fast): the naive reading rescans
    every member at every level, O(|C| * E).  Instead, `single_factor`
    and `single_comp` are worklists of vertices whose live degree just
    became 1 -- the same pattern as Kahn's topological sort maintaining
    its in-degree-zero queue.  Each edge is deleted exactly once inside
    remove_pair, doing O(1) work plus possibly enqueueing a neighbor, so
    the whole run is O(V + E).  With E = sum of distinct-prime counts
    (average ~2 per member), this is by far the cheapest operation in
    the project -- linear in a very sparse graph.

    Semantics note (easy to get wrong): failure here does NOT simply
    mean "no perfect matching exists".  Some sets have perfect payment
    matchings, every one of which is precedence-cyclic and hence
    unplayable; the forced-move discipline rejects those too.  Both
    directions are theorems (THEORY.md: "The schedulability theorem",
    "The completeness theorem"): with maximal-factor pools, success
    here is exactly playability.  See the n=21 example.
    """
    c_set = set(selections)
    f_set = set(factors)

    # Live bipartite adjacency between selections and factors.
    fac_of = {c: mf[c] & f_set for c in c_set}
    comps_of: Dict[int, Set[int]] = {f: set() for f in f_set}
    for c in c_set:
        for f in fac_of[c]:
            comps_of[f].add(c)

    single_factor = {c for c in c_set if len(fac_of[c]) == 1}
    single_comp = {f for f in f_set if len(comps_of[f]) == 1}

    front: List[int] = []
    back: List[int] = []  # discovered order; played in reverse at the end
    matching: Dict[int, int] = {}

    def remove_pair(c: int, f: int) -> None:
        c_set.discard(c)
        f_set.discard(f)
        single_factor.discard(c)
        single_comp.discard(f)
        for other_f in fac_of.pop(c) - {f}:
            comps = comps_of[other_f]
            comps.discard(c)
            if len(comps) == 1:
                single_comp.add(other_f)
        for other_c in comps_of.pop(f) - {c}:
            facs = fac_of[other_c]
            facs.discard(f)
            if len(facs) == 1:
                single_factor.add(other_c)
            elif not facs:
                raise Infeasible(f"{other_c} has no remaining factor")

    while c_set:
        while single_factor:
            c = single_factor.pop()
            if not fac_of[c]:
                raise Infeasible(f"{c} has no remaining factor")
            (f,) = fac_of[c]
            front.append(c)
            matching[c] = f
            remove_pair(c, f)
        if not c_set:
            break
        if single_comp:
            f = single_comp.pop()
            if f not in f_set or len(comps_of[f]) != 1:
                continue  # stale entry
            (c,) = comps_of[f]
            back.append(c)
            matching[c] = f
            remove_pair(c, f)
        else:
            raise Infeasible(
                f"cannot select all of {sorted(c_set)} using {sorted(f_set)}"
            )

    return front + back[::-1], matching


def optimize_mini(
    selections: Iterable[int],
    factors: Iterable[int],
    mf: Dict[int, Set[int]],
) -> Tuple[Set[int], Set[int]]:
    """The optimal subset of selections, taken greedily from largest to smallest.

    Implements optimize_mini from the wiki.  Returns (optC, rF): the optimal
    selections and the factors they will use as tax.
    """
    f_set = set(factors)
    opt_c: Set[int] = set()
    r_f: Set[int] = set()

    for c in sorted(selections, reverse=True):
        opt_c2 = opt_c | {c}
        r_f2 = r_f | (mf[c] & f_set)
        try:
            solve_mini(opt_c2, r_f2, mf)
        except Infeasible:
            continue
        opt_c = opt_c2
        r_f = r_f2

    return opt_c, r_f


def order_for_real_game(matching: Dict[int, int]) -> List[int]:
    """Order matched selections so the sequence is playable in a real game.

    solve_mini's order only accounts for maximal factors, but in a real
    taxman game a selection sweeps ALL of its remaining divisors from the
    pot.  The order must therefore ensure that whenever a's assigned factor
    divides b, a is played before b: a consumes its factor before b would
    sweep it, and conversely b cannot rob a of its factor.  (This is the
    role of solving frames front-to-back in the wiki's walkthrough.)

    Any topological order of that precedence works, and one always
    exists: by the schedulability theorem (THEORY.md, "The schedulability
    theorem"), an assignment produced by solve_mini's peeling can never
    have a cyclic precedence -- the prime-counting potential confines
    any would-be cycle to pool edges, and the peel rules refuse those
    configurations outright.

    Algorithmically this is Kahn's topological sort verbatim: build the
    precedence DAG, repeatedly emit a vertex with no unplayed
    predecessor.  (For the full game, where selections can divide each
    other, the precedence needs a second edge type -- see
    strategies.solvent._playable_order; the theorem covers both.)
    """
    order: List[int] = []
    blockers: Dict[int, int] = {c: 0 for c in matching}  # unplayed predecessors
    successors: Dict[int, List[int]] = {c: [] for c in matching}
    for a, f in matching.items():
        for b in matching:
            if b != a and b % f == 0:
                successors[a].append(b)
                blockers[b] += 1

    ready = sorted((c for c, k in blockers.items() if k == 0), reverse=True)
    while ready:
        c = ready.pop()
        order.append(c)
        for b in successors[c]:
            blockers[b] -= 1
            if blockers[b] == 0:
                ready.append(b)
        ready.sort(reverse=True)

    if len(order) != len(matching):
        raise Infeasible("cycle in selection ordering constraints")
    return order


def upper_half_game(n: int, spf: Sequence[int]) -> Tuple[Set[int], Set[int], Dict[int, Set[int]]]:
    """The factor game formed by the upper half of Taxman game n.

    Returns (C, F, mf) where C is every number greater than n/2, F is the
    union of their maximal factors, and mf maps each member of C to its
    maximal factors.
    """
    c_set = set(range(n // 2 + 1, n + 1))
    mf = {c: maximal_factors(c, spf) for c in c_set}
    f_set = set().union(*mf.values()) if mf else set()
    return c_set, f_set, mf


def solve_upper_half(n: int, spf: Sequence[int]) -> Tuple[List[int], Set[int]]:
    """Selections greater than n/2 in an optimal game n, in playable order.

    Returns (sequence, tax_pool): the ordered optimal upper-half selections
    and the set of factors available to pay their tax.
    """
    c_set, f_set, mf = upper_half_game(n, spf)
    opt_c, r_f = optimize_mini(c_set, f_set, mf)
    _, matching = solve_mini(opt_c, r_f, mf)
    return order_for_real_game(matching), r_f


# ---------------------------------------------------------------------------
# whole-game helpers shared by every strategy and evaluation script
# ---------------------------------------------------------------------------

def divisor_lists(n: int) -> List[List[int]]:
    """divs[m] is every proper divisor of m (1 <= divisor < m), for m <= n."""
    divs: List[List[int]] = [[] for _ in range(n + 1)]
    for d in range(1, n // 2 + 1):
        for m in range(2 * d, n + 1, d):
            divs[m].append(d)
    return divs


def maximal_factor_lists(n: int) -> List[List[int]]:
    """mf[m] is every maximal factor of m (f with m/f prime), ascending.

    Same shape and index contract as divisor_lists (indices 0 and 1 are
    empty).  This is the candidate-payment pool for the matching machinery:
    the lifting lemma guarantees any surviving proper divisor of a pick
    lifts to a surviving maximal factor outside the set, so restricting the
    pool to maximal factors is matching-equivalent to using all proper
    divisors (confirmed bit-identical for every n in 1..1000).
    """
    spf = smallest_prime_factors(n)
    mf: List[List[int]] = [[] for _ in range(n + 1)]
    for c in range(2, n + 1):
        mf[c] = sorted(maximal_factors(c, spf))
    return mf


def check_sequence(n: int, sequence: Sequence[int]) -> int:
    """Score a sequence while verifying it is a legal game of Taxman n."""
    pot = set(range(1, n + 1))
    score = 0
    for c in sequence:
        tax = {d for d in pot if d != c and c % d == 0}
        if c not in pot or not tax:
            raise RuntimeError(f"illegal sequence for game {n} at {c}")
        pot -= tax
        pot.remove(c)
        score += c
    return score
