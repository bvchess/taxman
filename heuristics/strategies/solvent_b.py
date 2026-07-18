# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""solvent-b: solvent, self-audited for the "self-blocking" mistake.

Background (see README.md's "Solvent" section and
`results/mistake_catalog_1000.json`): base solvent's entire measured loss
against optimal decomposes into persistent "self-blocking" episodes.
Solvent's descending greedy commits each candidate m as its OWN selection
the moment it stays playable -- but m was also, simultaneously, a coupon:
a maximal factor that some kept multiple p*m (a prime p, m's "patron")
could have used as ITS tax payment.  By buying m outright, solvent spends
the very coupon its patron needed, and the patron either drops out or
settles for a worse hand later in the scan.  This is invisible to solvent
in the moment: the scan is descending and one-pass, m's patron p*m was
already decided (kept) before m is reached, and nothing looks backward to
ask "does anyone still need this as currency?"

The dual-use audit set makes the conflict explicit, structurally, with no
hard-coded ranges:

    B = { m in current picks : some OTHER kept pick x has m as one of
          x's maximal factors }
      = { m in current picks : exists a prime p with p*m also kept }

Every m in B is a pick that is *also* a live coupon someone else could
have used -- exactly solvent's self-blocking shape, at any ratio p, not
just the p=2 upper-third band the mistake catalog happens to observe (that
band is a consequence of the mechanism -- 3m already exceeds n, so 2m is
m's only possible patron -- not a defining criterion; see
`_structural_audit_set`'s theory-violation assertion, verified for the
full n=2..1000 sweep).

Victims are invisible to one pass, but pricing them is exactly what a
banned rerun buys: ban m from the coupon pool and rerun solvent from
scratch with m unavailable as a *pick*.  If m's patron needed m as tax,
denying solvent m the same way IT denied the patron either frees the
patron to be paid by something else (the patron's set gets kept, m's own
points are usually recoverable elsewhere in the pool) or the rerun's
answer scores no better and the ban is discarded.  Either way, a full
rerun re-derives the entire descending scan with the ban in force, so
whatever happens below m -- the part solvent's one pass could never see
while deciding m -- is priced correctly this time by construction: it is
just solvent's own machinery, run again.

The audit iterates (cap 6, see MAX_ITERS): each round recomputes B on the
CURRENT pick set, tries every m in B in descending order, and adopts the
first one whose banned rerun strictly beats the current score -- growing
the cumulative banned set by exactly that one m and restarting the next
round from the new pick set.  No improvement in a round is a fixpoint;
bans are cumulative and never released, so a banned m can never resurface
in a later B.  Measured over n=2..1000 (`band_audit.py`,
`band_audit_results.json`): solvent-b holds 99.929% of all optimal points
(337/999 games exact, worst game 99.20%) and recovers about 52% of
solvent's total point loss against optimal -- while never scoring below
solvent (every adoption is a strict improvement, so the floor is solvent
itself).

The cost of asking "what if not m" is what this file buys down.  A naive
implementation reruns solvent_banned -- a full O(n^2 log n) descending
scan -- once per candidate per round: cap 6 rounds, |B| candidates tried
per round (bounded by the pick set size, O(n)), so O(n) reruns of an
O(n^2 log n) strategy is an O(n^3 log n)-class strategy, a full factor of
n above solvent itself (the same "one factor of n" relationship
README.md draws between reference/solvent.py and strategies/solvent.py,
here between rerun-per-audit and this file).

The speedup -- prefix-reuse forking -- removes the "rerun from n every
time" waste without changing what gets computed. Within one audit round,
every candidate trial differs from the plain (unbanned-this-round) scan
by exactly one extra banned number, and the candidates are tried in the
same descending order the scan itself runs in. So the state immediately
above the highest untried candidate is IDENTICAL across every trial in
the round -- it does not depend on which candidate is being tried, only
on the (round-fixed) `banned` set every trial shares. `_audit_pass` walks
the descending scan once per round: at each candidate m it snapshots the
scan's live (selected, owner) state (deep-copied -- `mf` is shared,
read-only, and never copied), forks a trial that skips m and runs the
rest of the scan to completion (`_finish_scan`), and -- if that trial does
not strictly improve -- resumes the SAME scan by playing m normally and
walking on to the next candidate. Everything above the lowest tried
candidate is computed exactly once per round rather than once per
candidate; only the tail below each fork point is repeated, and that tail
shrinks with each candidate tried. This is bit-identical to rerunning
solvent_banned(n, mf, banned | {m}) from scratch for every m (verified
directly, at every fork point, not just adopted ones -- see
evaluation/test_taxman.py's fork-vs-full property test), just without
recomputing the shared prefix.

solvent-b still costs a full factor of n more than plain solvent even with
forking (O(n) rounds-times-candidates of O(n^2 log n) reruns is
Theta(n^3 log n)-class overall) -- forking only removes the *shared-prefix*
waste of a naive rerun-per-candidate implementation, not the fact that
every candidate still needs its own from-that-point-down rescan, so the
speedup over a naive full rerun is real but modest (measured ~2x at
n=500 and n=1000; see README-style benchmarking in
evaluation/test_taxman.py's fork-vs-full test).  Measured against plain
solvent directly, the total cost is ~9x at n=500 and ~59x at n=1000, and
growing -- both the audit set size and the number of audit rounds tend to
grow with n.  Use solvent-b where the extra points matter more than the
extra time.
"""

from __future__ import annotations

from typing import Dict, List, Optional, Sequence, Set, Tuple

from strategies.solvent import (
    _augment,
    _complete_matching,
    _is_acyclic,
    _playable_order,
    _rollback,
    solvent,
)

MAX_ITERS = 6


def _structural_audit_set(pick_set: Set[int], mf: Sequence[List[int]]) -> Set[int]:
    """B = {m in pick_set : some OTHER kept pick x has m as a maximal
    factor of x} -- i.e. m is simultaneously kept AND usable as a kept
    multiple's tax payment.  No n/3, n/2, or any other numeric threshold.
    """
    B: Set[int] = set()
    for x in pick_set:
        for f in mf[x]:
            if f != x and f in pick_set:
                # Defense in depth: f is a maximal factor of x (f = x/p for
                # prime p >= 2), so f <= x/2 -- the audit can never reach
                # into the upper half.  A violation would be evidence
                # against conjecture (star) (no legal game's upper half
                # beats the no-search U*).
                assert 2 * f <= x, f"m={f} > half of kept x={x}: theory violation"
                B.add(f)
    return B


def _make_try_select(
    n: int,
    mf: Sequence[List[int]],
    selected: Set[int],
    owner: Dict[int, int],
):
    """Build solvent's try_select closure over a given (selected, owner)
    pair, so the same accept logic can drive either the trunk scan or a
    forked continuation without duplicating the tier-1/tier-2 decision
    (identical to strategies.solvent.solvent's inner try_select, and to
    band_audit.py's solvent_banned, just parameterized on which state it
    mutates).
    """

    def try_select(m: int) -> bool:
        pre_trail: List[Tuple[int, Optional[int]]] = []
        if m in owner:
            holder = owner.pop(m)
            if _augment(holder, owner, selected, mf, {m}, pre_trail):
                pre_trail.append((m, holder))
            else:
                owner[m] = holder
                return False

        selected.add(m)
        trail: List[Tuple[int, Optional[int]]] = []
        if _augment(m, owner, selected, mf, set(), trail):
            if _is_acyclic(selected, owner, n):
                return True
            _rollback(owner, trail)
            selected.discard(m)
            _rollback(owner, pre_trail)
            matching = _complete_matching(selected | {m}, mf)
            if matching is None:
                return False
            owner_candidate = {f: c for c, f in matching.items()}
            owner.clear()
            owner.update(owner_candidate)
            selected.add(m)
            return True

        selected.discard(m)
        _rollback(owner, pre_trail)
        return False

    return try_select


def _finish_scan(
    n: int,
    mf: Sequence[List[int]],
    banned: Set[int],
    selected: Set[int],
    owner: Dict[int, int],
    start: int,
) -> None:
    """Continue a descending scan from `start` down to 2, skipping
    `banned`, mutating `selected`/`owner` in place.  If `selected`/`owner`
    already reflect a plain-`banned` scan's state through `start + 1`, this
    reproduces the tail of solvent_banned(n, mf, banned) exactly -- the fork
    half of prefix-reuse forking.
    """
    try_select = _make_try_select(n, mf, selected, owner)
    for m in range(start, 1, -1):
        if not mf[m] or m in banned:
            continue
        try_select(m)


def _audit_pass(
    n: int,
    mf: Sequence[List[int]],
    banned: Set[int],
    candidates: List[int],
    current_score: int,
) -> Optional[Tuple[int, Set[int], List[int], int]]:
    """One forward descending scan implementing a full audit round: trial-
    evaluate every m in `candidates` (already sorted descending, matching
    band_audit.py's processing order) as one extra ban on top of the
    round-fixed `banned`, without rerunning the shared prefix per trial.

    Above the highest untried candidate, every trial sees the identical
    (selected, owner) state -- none of them differ from the plain `banned`
    scan yet -- so that work happens exactly once: the trunk scan below
    advances candidate by candidate, and at each one snapshots (deep-copies)
    the live state, forks a trial that skips m and runs the rest of the
    scan to completion (`_finish_scan`), and -- if that trial does not
    strictly beat current_score -- resumes the SAME trunk by playing m
    normally (via the trunk's own try_select) and walking on to the next
    candidate.  Each trial is therefore bit-identical to an independent
    solvent_banned(n, mf, banned | {m}) rerun (the trunk and that rerun
    agree on every candidate above m, since neither has banned it yet); only
    the shared prefix above is computed once instead of once per candidate.

    Returns (m, selected, playable_seq, score) for the first candidate (in
    descending order) whose trial strictly improves on current_score --
    band_audit.py's "adopt the first improvement, then stop" rule -- or
    None if every candidate was tried and none did (a fixpoint this round).
    """
    selected: Set[int] = set()
    owner: Dict[int, int] = {}
    try_select = _make_try_select(n, mf, selected, owner)

    remaining = iter(candidates)
    next_candidate = next(remaining, None)

    for m in range(n, 1, -1):
        if not mf[m] or m in banned:
            continue
        if m == next_candidate:
            fork_selected = set(selected)
            fork_owner = dict(owner)
            _finish_scan(n, mf, banned | {m}, fork_selected, fork_owner, m - 1)
            trial_score = sum(fork_selected)
            if trial_score > current_score:
                match = {c: f for f, c in fork_owner.items() if c in fork_selected}
                trial_seq = _playable_order(fork_selected, match, n)
                return m, fork_selected, trial_seq, trial_score
            next_candidate = next(remaining, None)
        try_select(m)

    return None


def solvent_b(n: int, mf: Sequence[List[int]]) -> List[int]:
    """solvent-b: solvent, audited to a fixpoint against self-blocking.

    Runs base solvent, then repeatedly computes the dual-use audit set of
    the current pick set and tries banning each member (highest first) to
    see if a rerun with that one extra ban scores strictly higher; the
    first improvement is adopted, cumulatively, and the audit restarts from
    the new pick set (cap MAX_ITERS rounds).  See the module docstring for
    the mechanism and `_audit_pass` for the prefix-reuse forking that makes
    each round one scan instead of one rerun per candidate.  Never scores
    below solvent: adoption is always a strict improvement, so a fixpoint
    with zero adoptions returns solvent's own result unchanged.
    """
    current_seq = solvent(n, mf)
    current_set: Set[int] = set(current_seq)
    current_score = sum(current_set)
    banned: Set[int] = set()

    for _ in range(MAX_ITERS):
        B = _structural_audit_set(current_set, mf)
        if not B:
            break
        candidates = sorted(B, reverse=True)
        adopted = _audit_pass(n, mf, banned, candidates, current_score)
        if adopted is None:
            break
        m, new_set, new_seq, new_score = adopted
        banned = banned | {m}
        current_set, current_seq, current_score = new_set, new_seq, new_score

    return current_seq
