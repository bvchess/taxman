"""solvent: matching-based full-game approximation of optimal Taxman play.

The full-game generalization of "take the highest prime": consider
n, n-1, ..., 2 and accept each number if the pick set stays playable.
Per candidate the decision has three outcomes:

  * the incremental augmenting search fails -> reject outright.  By
    Berge's lemma (a matching is maximum iff no augmenting path exists
    from a free vertex), the failure already proves no matching covers
    the candidate; nothing can overturn it.
  * the search succeeds and the updated matching's precedence relation
    is acyclic -> accept.
  * the search succeeds but the matching has gone precedence-cyclic ->
    ask solve_mini, which decides exactly.  This path is load-bearing:
    some sets have perfect matchings, every one of them cyclic (first
    at n=21), and solve_mini correctly refuses those -- it is a
    playability oracle, strictly stronger than a matching test.

A rejection is permanent either way: playable sets are downward-closed,
so a set unplayable now can never become playable by adding more.

The key fact (sharpened by the lifting lemma to maximal factors only):

    A set S of selections can all be played, in some order, if and only
    if there is a matching that assigns each selection a distinct
    maximal factor outside S, such that the precedence relation "a
    before b whenever a's assigned factor divides b, or a divides b" is
    acyclic.

    Playing any topological order works: a selection's assigned factor
    cannot be swept earlier (everything divisible by it comes later), and
    a selection that is itself someone's divisor is played before being
    swept.  Conversely, a real game induces such a matching (any paid
    divisor lifts to a surviving maximal factor) and its play order is a
    linear extension.

Under the hood this is classic bipartite maximum matching, applied
greedily.  _augment is Kuhn's algorithm (the Hungarian-style
alternating DFS); the accept loop is the matroid greedy template --
descending weights, keep what stays independent -- which is why the
picks above n/2 are provably the heaviest upper half any game can hold
(matchable upper sets form a transversal matroid; that optimal games
hold exactly this set is verified for n <= 1000, not proven);
_is_acyclic and _playable_order are Kahn's
topological sort doing cycle detection and schedule construction.  The
one nonstandard ingredient is the interplay between matching and
scheduling: matchability alone is NOT playability, which is exactly why
the cyclic path above must consult solve_mini.
"""

from __future__ import annotations

from typing import Dict, List, Optional, Sequence, Set, Tuple

from core import Infeasible, solve_mini


def _augment(
    v: int,
    owner: Dict[int, int],
    selected: Set[int],
    mf: Sequence[List[int]],
    visited: Set[int],
    trail: List[Tuple[int, Optional[int]]],
) -> bool:
    """Kuhn's augmenting search: find v a candidate factor, reassigning others.

    The textbook alternating-path DFS for bipartite matching: try each of
    v's factors; a free factor ends the path, an owned one recurses on its
    holder to re-route it.  Success extends the matching by one; failure
    (with the full `visited` set explored) proves, via Berge's lemma, that
    no matching covers v alongside the current picks -- callers treat that
    as a conclusive rejection.  `mf` is the candidate-payment pool (maximal
    factors); every owner change is recorded on the trail so a failed
    composite operation can be rolled back exactly, undo-log style.
    """
    for f in reversed(mf[v]):
        if f in selected or f in visited:
            continue
        visited.add(f)
        holder = owner.get(f)
        if holder is None or _augment(holder, owner, selected, mf, visited, trail):
            trail.append((f, holder))
            owner[f] = v
            return True
    return False


def _rollback(owner: Dict[int, int], trail: List[Tuple[int, Optional[int]]]) -> None:
    for f, previous in reversed(trail):
        if previous is None:
            del owner[f]
        else:
            owner[f] = previous


def _complete_matching(
    target: Set[int], mf: Sequence[List[int]]
) -> Optional[Dict[int, int]]:
    """Decide playability of `target` exactly via the factor-game reduction.

    Tier 1's cycle retry only reshuffles the candidate's own coupon; a real
    cycle can require OTHER picks' coupons to move too, which falsely
    vetoes playable candidates (see strategies.seteval.SetEval, which
    proves this out).  This is the complete tier: reduce target to a
    bipartite factor game -- each pick maps to the set of its candidate
    factors (maximal factors) NOT in target -- and let solve_mini decide
    exactly, returning a full pick -> factor matching, or None if no
    assignment covers every pick.
    """
    avail = {c: {d for d in mf[c] if d not in target} for c in target}
    factors: Set[int] = set().union(*avail.values()) if avail else set()
    try:
        _, matching = solve_mini(target, factors, avail)
    except Infeasible:
        return None
    return matching


def _is_acyclic(selected: Set[int], owner: Dict[int, int], n: int) -> bool:
    """Kahn's check of the precedence relation over the selected set."""
    indeg = {c: 0 for c in selected}
    succ: Dict[int, List[int]] = {c: [] for c in selected}
    match = {c: f for f, c in owner.items() if c in selected}
    for a in selected:
        seen = set()
        for start in (match[a], a):
            for b in range(2 * start, n + 1, start):
                if b in selected and b != a and b not in seen:
                    seen.add(b)
                    succ[a].append(b)
                    indeg[b] += 1
    ready = [c for c in selected if indeg[c] == 0]
    done = 0
    while ready:
        a = ready.pop()
        done += 1
        for b in succ[a]:
            indeg[b] -= 1
            if indeg[b] == 0:
                ready.append(b)
    return done == len(selected)


def _playable_order(
    selected: Set[int], match: Dict[int, int], n: int
) -> List[int]:
    """Topologically order selections by the precedence relation.

    The fast path's acceptances are verified acyclic before being
    committed; the complete tier instead trusts the conjecture that
    solve_mini's returned assignment is always schedulable, so it accepts
    unconditionally.  Either way the final matching is expected acyclic,
    and a single Kahn's-algorithm pass orders every element.  A leftover
    cycle would be a conjecture violation, so this raises rather than
    repairing it.
    """
    succ: Dict[int, List[int]] = {c: [] for c in selected}
    indeg: Dict[int, int] = {c: 0 for c in selected}
    for a in selected:
        seen = set()
        for start in (match[a], a):
            for b in range(2 * start, n + 1, start):
                if b in selected and b != a and b not in seen:
                    seen.add(b)
                    succ[a].append(b)
                    indeg[b] += 1
    ready = sorted(c for c in selected if indeg[c] == 0)
    order: List[int] = []
    while ready:
        a = ready.pop()
        order.append(a)
        for b in succ[a]:
            indeg[b] -= 1
            if indeg[b] == 0:
                ready.append(b)
    if len(order) != len(selected):
        raise RuntimeError(
            "conjecture violated: solve_mini produced an unschedulable "
            "assignment (see README, 'cyclic')"
        )
    return order


def solvent(
    n: int,
    mf: Sequence[List[int]],
    rejections: Optional[List[Tuple[int, str]]] = None,
) -> List[int]:
    """Solvent matching approximation of a full game; returns the sequence.

    `mf` is the candidate-payment pool: each pick's maximal factors (f with
    pick/f prime), as built by core.maximal_factor_lists.  This is the
    correct pool for the matching question ("can every pick reserve a
    distinct payment?") -- the lifting lemma proves it matching-equivalent
    to the full proper-divisor pool, and feeding maximal factors instead of
    all divisors was verified bit-identical (set and score) for every n in
    1..1000.
    """
    selected: Set[int] = set()
    owner: Dict[int, int] = {}  # factor -> selection paying it

    def try_select(m: int) -> bool:
        pre_trail: List[Tuple[int, Optional[int]]] = []
        if m in owner:
            # m currently serves as someone's tax; try to re-route that
            # holder onto a different divisor without touching m.
            holder = owner.pop(m)
            if _augment(holder, owner, selected, mf, {m}, pre_trail):
                pre_trail.append((m, holder))  # rollback restores m's owner
            else:
                owner[m] = holder  # undo the pop
                # Conclusive reject (Berge): a failed augmenting search from
                # the free vertex `holder` (avoiding m) proves no matching
                # saturates `selected` while excluding factor m, yet
                # target = selected | {m} requires exactly that.  The
                # complete tier's Infeasible is therefore forced -- skip it.
                if rejections is not None:
                    rejections.append((m, "infeasible"))
                return False

        selected.add(m)  # blocks m from being used as anyone's tax
        trail: List[Tuple[int, Optional[int]]] = []
        if _augment(m, owner, selected, mf, set(), trail):
            if _is_acyclic(selected, owner, n):
                return True  # fast, common-case silent success
            # Augment succeeded but the precedence is cyclic: this is NOT
            # conclusive, so the complete tier must decide.  Here solve_mini
            # is a playability oracle, not a matching oracle -- a set can be
            # matchable yet unschedulable (its every matching cyclic), and
            # solve_mini's forced-peeling structure correctly rejects such
            # sets.  See the n=21 counterexample in the lean-solvent
            # experiment docstring (candidate 10 augments but yields an
            # unplayable 145-point set).  This path is unchanged from the
            # canonical two-tier code.
            _rollback(owner, trail)
            selected.discard(m)
            _rollback(owner, pre_trail)
            matching = _complete_matching(selected | {m}, mf)
            if matching is None:
                if rejections is not None:
                    rejections.append((m, "infeasible"))
                return False
            owner_candidate = {f: c for c, f in matching.items()}
            owner.clear()
            owner.update(owner_candidate)
            selected.add(m)
            return True

        # Conclusive reject (Berge): a failed augmenting search from the free
        # vertex m proves no matching saturates selected | {m}.
        selected.discard(m)
        _rollback(owner, pre_trail)
        if rejections is not None:
            rejections.append((m, "infeasible"))
        return False

    for m in range(n, 1, -1):
        if not mf[m]:
            continue
        try_select(m)

    match = {c: f for f, c in owner.items() if c in selected}
    return _playable_order(selected, match, n)
