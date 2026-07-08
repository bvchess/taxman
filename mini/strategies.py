"""Approximating a full optimal Taxman game in (roughly) O(n^2) time.

The upper-half theory (see verify.py) shows that the selections greater
than N/2 in an optimal game can be found in polynomial time.  This module
pushes the same ideas through the whole game and compares the results
against heuristics from Robert Moniot's strategy comparison
(https://www.dsm.fordham.edu/~moniot/taxman-strategies-comparison.html).

The key fact (a generalization of the ordering argument in verify.py):

    A set S of selections can all be played, in some order, if and only
    if there is a matching that assigns each selection a distinct divisor
    still in the game, such that the precedence relation "a before b
    whenever a's assigned divisor divides b, or a divides b" is acyclic.

    Playing any topological order works: a selection's assigned divisor
    cannot be swept earlier (everything divisible by it comes later), and
    a selection that is itself someone's divisor is played before being
    swept.  Conversely, a real game induces such a matching (pick one
    paid divisor per selection) and its play order is a linear extension.

Strategies implemented here:

solvent
    The full-game generalization of "take the highest prime": consider
    n, n-1, ..., 2 and accept each number if the pick set stays playable.
    Playability is decided two-tier: a cheap incremental augmenting search
    (Kuhn) plus acyclicity check tries to extend the matching in place,
    and on any failure the complete tier (an exact bipartite reduction via
    solve_mini) decides accept/reject exactly.  The complete tier trusts
    the conjecture that solve_mini's returned assignment is always
    schedulable (zero counterexamples across all runs), so any matching it
    returns is accepted unconditionally; the sole rejection reason is now
    "infeasible" (solve_mini found no assignment at all).  A complete-tier
    rejection is permanent: playable sets are downward-closed, so a set
    that cannot be matched now can never be matched after more elements
    are added.

cascade
    The pure upper-half machinery applied band by band: play the numbers
    above half the remaining maximum optimally with optimize_mini and
    solve_mini, sweep, repeat on what is left.  Shows how much of the
    game the verified theory captures on its own, with no promotions.

onetax (Moniot)
    Pick the largest number with exactly one divisor left in play, with
    Moniot's refinement: if that pick would strand a multiple m (leaving
    m with no divisors while m divides nothing left in play), pick m
    instead.

maxturn (Carmony & Holliday, 1993)
    Pick the number maximizing the player's take minus the taxman's take
    for the turn.

Usage:
    python3 strategies.py [--max-n 1000] [--optimal PATH] [--moniot PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

from taxman_mini import (
    MiniInfeasible, maximal_factors, optimize_mini, smallest_prime_factors,
    solve_mini,
)
from verify import DEFAULT_OPTIMAL

DEFAULT_MONIOT = Path(__file__).resolve().parent / "results" / "moniot_table.json"


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


# ---------------------------------------------------------------------------
# solvent: matching-based full-game approximation
# ---------------------------------------------------------------------------

def _augment(
    v: int,
    owner: Dict[int, int],
    selected: Set[int],
    mf: Sequence[List[int]],
    visited: Set[int],
    trail: List[Tuple[int, Optional[int]]],
) -> bool:
    """Kuhn's augmenting search: find v a candidate factor, reassigning others.

    `mf` is the candidate-payment pool (maximal factors); every owner change
    is recorded on the trail so it can be rolled back.
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
    """Decide playability of `target` exactly via the Taxman Mini reduction.

    Tier 1's cycle retry only reshuffles the candidate's own coupon; a real
    cycle can require OTHER picks' coupons to move too, which falsely
    vetoes playable candidates (see transitions.py's SetEval, which proves
    this out).  This is the complete tier: reduce target to a bipartite
    Taxman Mini game -- each pick maps to the set of its candidate factors
    (maximal factors) NOT in target -- and let solve_mini decide exactly,
    returning a full pick -> factor matching, or None if no assignment
    covers every pick.
    """
    avail = {c: {d for d in mf[c] if d not in target} for c in target}
    factors: Set[int] = set().union(*avail.values()) if avail else set()
    try:
        _, matching = solve_mini(target, factors, avail)
    except MiniInfeasible:
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
    pick/f prime), as built by maximal_factor_lists.  This is the correct
    pool for the matching question ("can every pick reserve a distinct
    payment?") -- the lifting lemma proves it matching-equivalent to the
    full proper-divisor pool, and feeding maximal factors instead of all
    divisors was verified bit-identical (set and score) for every n in
    1..1000.
    """
    selected: Set[int] = set()
    owner: Dict[int, int] = {}  # factor -> selection paying it

    def try_select(m: int) -> bool:
        # STEP 1 -- fast path: an opportunistic incremental extension of
        # the current matching.  Any failure here rolls back fully and
        # falls through to the complete tier; nothing is rejected yet.
        pre_trail: List[Tuple[int, Optional[int]]] = []
        if m in owner:
            # m currently serves as someone's tax; try to re-route that
            # holder onto a different divisor without touching m.
            holder = owner.pop(m)
            if _augment(holder, owner, selected, mf, {m}, pre_trail):
                pre_trail.append((m, holder))  # rollback restores m's owner
            else:
                owner[m] = holder  # undo the pop; fall through to step 2
                pre_trail = []

        if len(pre_trail) or m not in owner:
            selected.add(m)  # blocks m from being used as anyone's tax
            trail: List[Tuple[int, Optional[int]]] = []
            if _augment(m, owner, selected, mf, set(), trail):
                if _is_acyclic(selected, owner, n):
                    return True  # fast, common-case silent success
                _rollback(owner, trail)
            selected.discard(m)
            _rollback(owner, pre_trail)

        # STEP 2 -- complete tier: the unconditional decider.  `selected`
        # does not contain m here (the fast path rolled back fully), so
        # pass a fresh `selected | {m}` without mutating shared state.
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

    for m in range(n, 1, -1):
        if not mf[m]:
            continue
        try_select(m)

    match = {c: f for f, c in owner.items() if c in selected}
    return _playable_order(selected, match, n)


# ---------------------------------------------------------------------------
# cascade: the verified upper-half theory applied band by band
# ---------------------------------------------------------------------------

def cascade(n: int, divs: Sequence[List[int]]) -> List[int]:
    """Approximate a full game with repeated upper-half mini games.

    Unlike solvent, cascade uses TRUE proper divisors (`divs`) as its
    per-band edge pool, NOT the maximal-factor pool.  It replays
    solve_mini's returned ORDER directly under real-game (true-divisor)
    sweeping, so the order must be valid against every divisor a pick
    sweeps -- a stronger property than the matching-feasibility the lifting
    lemma equates the two pools on.  With a maximal-factor pool solve_mini
    can emit an order that strands a pick whose only surviving payment is a
    non-maximal shared divisor swept early (e.g. n=5: playing 4 before 5
    sweeps their shared divisor 1).  The sweep replay and end-of-band
    liveness check need true divisors for the same reason.
    """
    pot: Set[int] = set(range(1, n + 1))
    sequence: List[int] = []
    band_top = n

    while band_top >= 1 and pot:
        # Candidates are source nodes: nothing in the pot is a multiple.
        c_set = {
            c for c in pot
            if 2 * c > band_top
            and not any(m in pot for m in range(2 * c, n + 1, c))
        }
        band_top //= 2
        if not c_set:
            continue
        edges: Dict[int, Set[int]] = {
            c: {d for d in divs[c] if d in pot} for c in c_set
        }
        f_set: Set[int] = set().union(*edges.values())

        opt_c, r_f = optimize_mini(c_set, f_set, edges)
        order, _ = solve_mini(opt_c, r_f, edges)

        for c in order:
            tax = [d for d in divs[c] if d in pot]
            if not tax:
                raise RuntimeError(f"illegal move {c} in game {n}")
            pot.difference_update(tax)
            pot.discard(c)
            sequence.append(c)

        # Unselected candidates with no divisor left can never be played;
        # ones that still have a divisor get another chance in later bands.
        pot -= {c for c in c_set - opt_c if not any(d in pot for d in divs[c])}

    return sequence


# ---------------------------------------------------------------------------
# Moniot's heuristics
# ---------------------------------------------------------------------------

def one_tax(n: int, divs: Sequence[List[int]], refined: bool = True) -> int:
    """Moniot's OneTax heuristic; returns the player's score.

    Picks the largest number with exactly one divisor left in play, with
    Moniot's refinement: if that pick would strand a multiple m (leaving
    m with no divisors while m divides nothing left in play), pick m
    instead.
    """
    in_pot = bytearray(n + 1)
    for i in range(1, n + 1):
        in_pot[i] = 1
    count = [sum(1 for d in divs[m] if in_pot[d]) for m in range(n + 1)]

    def remove(x: int) -> None:
        in_pot[x] = 0
        for m in range(2 * x, n + 1, x):
            count[m] -= 1

    score = 0
    while True:
        pick = 0
        for c in range(n, 1, -1):
            if in_pot[c] and count[c] == 1:
                pick = c
                break
        if not pick:
            break

        if pick and count[pick] == 1 and refined:
            d = next(x for x in divs[pick] if in_pot[x])
            rescue = 0
            for m in range(2 * pick, n + 1, pick):
                if not in_pot[m]:
                    continue
                stranded = all(x in (pick, d) for x in divs[m] if in_pot[x])
                useless = not any(in_pot[k] for k in range(2 * m, n + 1, m))
                if stranded and useless:
                    rescue = max(rescue, m)
            if rescue:
                pick = rescue

        tax = [x for x in divs[pick] if in_pot[x]]
        remove(pick)
        for x in tax:
            remove(x)
        score += pick
    return score


def max_turn(n: int, divs: Sequence[List[int]]) -> int:
    """Carmony & Holliday's MaxTurn heuristic; returns the player's score."""
    in_pot = bytearray(n + 1)
    for i in range(1, n + 1):
        in_pot[i] = 1
    count = [len(divs[m]) for m in range(n + 1)]
    dsum = [sum(divs[m]) for m in range(n + 1)]

    def remove(x: int) -> None:
        in_pot[x] = 0
        for m in range(2 * x, n + 1, x):
            count[m] -= 1
            dsum[m] -= x

    score = 0
    while True:
        pick, best = 0, None
        for c in range(n, 1, -1):  # ties go to the larger number
            if in_pot[c] and count[c] > 0:
                value = c - dsum[c]
                if best is None or value > best:
                    pick, best = c, value
        if not pick:
            break
        tax = [x for x in divs[pick] if in_pot[x]]
        remove(pick)
        for x in tax:
            remove(x)
        score += pick
    return score


# ---------------------------------------------------------------------------
# scoring and reporting
# ---------------------------------------------------------------------------

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


STRATEGIES = ("solvent", "cascade", "onetax", "maxturn")


def play_all(
    n: int, divs: Sequence[List[int]], mf: Sequence[List[int]]
) -> Dict[str, int]:
    return {
        "solvent": check_sequence(n, solvent(n, mf)),
        "cascade": check_sequence(n, cascade(n, divs)),
        "onetax": one_tax(n, divs),
        "maxturn": max_turn(n, divs),
    }


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--max-n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--moniot", type=Path, default=DEFAULT_MONIOT)
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    moniot = json.loads(args.moniot.read_text())
    moniot_rows = {int(k): v for k, v in moniot["rows"].items()}

    started = time.monotonic()
    divs = divisor_lists(args.max_n)
    mf = maximal_factor_lists(args.max_n)
    results: Dict[int, Dict[str, int]] = {}

    sys.setrecursionlimit(100_000)
    for n in range(1, args.max_n + 1):
        if n not in optimal:
            break
        results[n] = play_all(n, divs, mf)
        results[n]["opt"] = optimal[n]["score"]
        if n % 100 == 0:
            print(f"...through n={n} ({time.monotonic() - started:.0f}s)",
                  file=sys.stderr)

    # Sanity: compare our OneTax/MaxTurn implementations with Moniot's table.
    for column, mine in (("OneTax", "onetax"), ("MaxTurn", "maxturn")):
        idx = moniot["header"].index(column) - 1
        diffs = [n for n, r in results.items()
                 if n in moniot_rows and moniot_rows[n][idx] != r[mine]]
        span = sum(1 for n in results if n in moniot_rows)
        print(f"{column} matches Moniot's table in {span - len(diffs)}/{span} "
              f"games{f' (differs at {diffs[:10]})' if diffs else ''}")

    print()
    scored = {n: r for n, r in results.items() if r["opt"] > 0}
    for name in STRATEGIES:
        pcts = [100 * r[name] / r["opt"] for r in scored.values()]
        exact = sum(1 for r in scored.values() if r[name] == r["opt"])
        print(f"{name:<8} mean {sum(pcts) / len(pcts):6.2f}%   "
              f"min {min(pcts):6.2f}%   optimal in {exact}/{len(pcts)} games")

    print()
    header = f"{'n':>5} {'optimal':>8}"
    for name in STRATEGIES:
        header += f" {name:>8} {'%':>6}"
    print(header)
    for n in (21, 50, 100, 128, 250, 500, 750, 1000):
        if n not in scored:
            continue
        r = scored[n]
        line = f"{n:>5} {r['opt']:>8}"
        for name in STRATEGIES:
            line += f" {r[name]:>8} {100 * r[name] / r['opt']:>6.2f}"
        print(line)

    out = Path(__file__).resolve().parent / "strategies_out.json"
    out.write_text(json.dumps(results, indent=0))
    print(f"\nper-game results written to {out.name} "
          f"({time.monotonic() - started:.0f}s total)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
