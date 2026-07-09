"""SetEval: an incremental playability evaluator for Taxman pick sets.

A set of Taxman picks is playable iff there is a matching assigning each
pick a distinct proper divisor still in the pot such that the precedence
relation "a before b whenever a's assigned divisor divides b, or a divides
b" is acyclic (any topological order is then a legal game).  SetEval keeps
such a matching incrementally: a candidate set can be mutated pick-by-pick
(playable_add / remove) and re-tested cheaply, with augmenting-path
rollback so a rejected mutation fully restores prior state.

This class was extracted verbatim from transitions.py (which now imports
it) so the continuation solver and the transition anatomy share one
validated evaluator.  It mirrors strategies.solvent's matching machinery and
falls back to core.solve_mini for a complete playability decision when the
fast, incomplete solvent tier rejects an add.
"""

from __future__ import annotations

from typing import Dict, List, Optional, Sequence, Set, Tuple

from core import Infeasible, solve_mini


# ---------------------------------------------------------------------------
# SetEval: incremental playability evaluator (a stateful port of solvent)
# ---------------------------------------------------------------------------

class SetEval:
    """A mutable set of Taxman picks with an incremental playability test.

    Mirrors strategies.solvent's matching machinery over the maximal-factor
    candidate pool (`mf`): `owner` maps each candidate factor currently
    spent as tax to the pick holding it, and `match` is the inverse
    restricted to the current picks.  The pool is maximal factors rather
    than all proper divisors because the lifting lemma makes the two
    matching-equivalent (verified bit-identical for every n in 1..1000).
    `match` is not threaded
    through Kuhn's recursion; it is rebuilt by one dict inversion after
    every committed mutation, which keeps it immune to augmenting-path
    rollback and costs only O(|S|).
    """

    def __init__(self, n: int, mf: Sequence[List[int]]) -> None:
        self.n = n
        # shared, read-only candidate-payment pool: mf[m] = m's maximal
        # factors (f with m/f prime), ascending.  The lifting lemma makes
        # this matching-equivalent to the full proper-divisor pool, so the
        # augmenting search and solve_mini reduction below only need it.
        self.mf = mf
        self.S: Set[int] = set()
        self.owner: Dict[int, int] = {}  # divisor -> pick paying it as tax
        self.match: Dict[int, int] = {}  # pick -> its own coupon
        # Set by _solvent_add: True when the fast tier failed on an
        # augmenting search (Berge-conclusive -- no matching covers S | {x});
        # False when it failed only because every matching found was cyclic
        # (the complete tier must then decide).
        self._fail_conclusive = False

    # -- Kuhn's augmenting search, ported from strategies._augment ------------
    def _augment(
        self,
        v: int,
        visited: Set[int],
        trail: List[Tuple[int, Optional[int]]],
    ) -> bool:
        for f in reversed(self.mf[v]):
            if f in self.S or f in visited:
                continue
            visited.add(f)
            holder = self.owner.get(f)
            if holder is None or self._augment(holder, visited, trail):
                trail.append((f, holder))
                self.owner[f] = v
                return True
        return False

    def _rollback(self, trail: List[Tuple[int, Optional[int]]]) -> None:
        for f, previous in reversed(trail):
            if previous is None:
                del self.owner[f]
            else:
                self.owner[f] = previous

    def _is_acyclic(self) -> bool:
        """Kahn's check of the precedence relation over the current picks."""
        selected = self.S
        indeg = {c: 0 for c in selected}
        succ: Dict[int, List[int]] = {c: [] for c in selected}
        match = {c: f for f, c in self.owner.items() if c in selected}
        for a in selected:
            seen = set()
            for start in (match[a], a):
                for b in range(2 * start, self.n + 1, start):
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

    def _rebuild_match(self) -> None:
        self.match = {c: f for f, c in self.owner.items() if c in self.S}

    def playable_add(self, x: int) -> bool:
        """Try to add x to the set; return whether the set stays playable.

        Two-tier test.  The fast tier is a direct port of strategies.solvent's
        try_select (pop x if it holds someone's tax, add x, augment, retry
        with each of x's own divisors forced on a precedence cycle); it is
        sound but not complete -- its single-pick coupon reshuffling can
        falsely reject a member of a genuinely playable set when breaking
        the cycle needs OTHER picks reassigned too (empirically ~9 picks
        per game around n=500, e.g. 230/225/220 which sit inside the
        provably-playable optimal set).  So when the fast tier rejects, a
        complete tier runs: solve_mini decides the bipartite selectability
        of S + {x} exactly (raising Infeasible iff no assignment covers
        every pick) and its canonical order is confirmed acyclic under the
        full real-game precedence.  Only a rejection by BOTH tiers is a
        real "unplayable".  Any failure fully restores prior state.
        """
        if x in self.S:
            return True
        if self._solvent_add(x):
            return True
        if self._fail_conclusive:
            # Berge-conclusive fast-tier failure (a failed augmenting search
            # from a free vertex): no matching covers S | {x}, so the
            # complete tier's Infeasible is forced -- skip it.
            return False
        # Cyclic-only fast-tier failure: solve_mini here is a playability
        # oracle (a set can be matchable yet unschedulable; see the n=21
        # counterexample in the lean-solvent experiment), so the complete
        # tier must decide.
        target = self.S | {x}
        matching = self._complete_matching(target)
        if matching is None:
            return False
        self.S = target
        self.owner = {f: c for c, f in matching.items()}
        self.match = dict(matching)
        return True

    def _solvent_add(self, x: int) -> bool:
        """Fast, sound-but-incomplete add: strategies.solvent's try_select.

        Sets self._fail_conclusive on failure: True when an augmenting
        search failed (Berge-conclusive), False when only a precedence cycle
        remained (the complete tier must decide).
        """
        # A failed Kuhn search leaves the matching untouched, so only
        # successful augments need their trails rolled back.
        self._fail_conclusive = False
        pre_trail: List[Tuple[int, Optional[int]]] = []
        if x in self.owner:
            # x currently serves as someone's tax; try to rematch them
            # without letting them reclaim x.
            holder = self.owner.pop(x)
            if not self._augment(holder, {x}, pre_trail):
                self.owner[x] = holder
                self._fail_conclusive = True  # Berge: re-route failure
                return False
            pre_trail.append((x, holder))  # a rollback restores x's owner
        self.S.add(x)  # blocks x from being used as anyone's tax below

        trail: List[Tuple[int, Optional[int]]] = []
        if not self._augment(x, set(), trail):
            self.S.discard(x)  # no matching covers x: permanent reject
            self._rollback(pre_trail)
            self._fail_conclusive = True  # Berge: augment failure from x
            return False
        if self._is_acyclic():
            self._rebuild_match()
            return True
        self._rollback(trail)

        # The matching creates a precedence cycle; retry with each of x's
        # candidate factors forced in turn, since a different assignment for
        # x can route the precedence differently.
        for f in reversed(self.mf[x]):
            if f in self.S:
                continue
            trail = []
            if self._augment(x, set(self.mf[x]) - {f}, trail):
                if self._is_acyclic():
                    self._rebuild_match()
                    return True
                self._rollback(trail)
        self.S.discard(x)
        self._rollback(pre_trail)
        # cyclic-only failure: NOT conclusive (flag stays False)
        return False

    def _complete_matching(
        self, target: Set[int]
    ) -> Optional[Dict[int, int]]:
        """Decide playability of `target` exactly, returning a matching.

        Reduces the pick set to a bipartite factor game (each pick to
        its proper divisors that lie outside the set) and lets solve_mini
        find an assignment or prove none exists.  A returned assignment is
        confirmed acyclic under the full real-game precedence (both the
        coupon edges and the pick-divides-pick edges) before acceptance;
        None means no playable assignment (Infeasible or a cyclic one).
        """
        avail = {c: {d for d in self.mf[c] if d not in target} for c in target}
        factors: Set[int] = set().union(*avail.values()) if avail else set()
        try:
            _, matching = solve_mini(target, factors, avail)
        except Infeasible:
            return None
        if not self._matching_acyclic(target, matching):
            return None
        return matching

    def _matching_acyclic(
        self, target: Set[int], matching: Dict[int, int]
    ) -> bool:
        """Kahn's check of the precedence induced by a full matching."""
        indeg = {c: 0 for c in target}
        succ: Dict[int, List[int]] = {c: [] for c in target}
        for a in target:
            seen = set()
            for start in (matching[a], a):
                for b in range(2 * start, self.n + 1, start):
                    if b in target and b != a and b not in seen:
                        seen.add(b)
                        succ[a].append(b)
                        indeg[b] += 1
        ready = [c for c in target if indeg[c] == 0]
        done = 0
        while ready:
            a = ready.pop()
            done += 1
            for b in succ[a]:
                indeg[b] -= 1
                if indeg[b] == 0:
                    ready.append(b)
        return done == len(target)

    def remove(self, x: int) -> None:
        """Remove pick x.  A subset of a playable set is always playable."""
        if x not in self.S:
            return
        self.S.discard(x)
        f = self.match.pop(x, None)
        if f is not None:
            del self.owner[f]

    def score(self) -> int:
        return sum(self.S)

    def snapshot(self) -> Tuple[frozenset, dict, dict]:
        return (frozenset(self.S), dict(self.owner), dict(self.match))

    def restore(self, snap: Tuple[frozenset, dict, dict]) -> None:
        self.S = set(snap[0])
        self.owner = dict(snap[1])
        self.match = dict(snap[2])

    def clone(self) -> "SetEval":
        """A new evaluator sharing n/mf by reference, with copied state."""
        other = SetEval(self.n, self.mf)
        other.S = set(self.S)
        other.owner = dict(self.owner)
        other.match = dict(self.match)
        return other
