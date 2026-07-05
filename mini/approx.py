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

greedy
    The full-game generalization of "take the highest prime": consider
    n, n-1, ..., 2 and accept each number if an augmenting path can add
    it to the matching (if no augmenting path exists, no matching covers
    it - a permanent, well-founded rejection).  Rare precedence cycles
    are repaired at the end by dropping the smallest involved selection.
    Runs in O(n) augmenting searches of O(E) each, E = sum of divisor
    counts = O(n log n).

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
    python3 approx.py [--max-n 1000] [--optimal PATH] [--moniot PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

from taxman_mini import optimize_mini, solve_mini
from verify import DEFAULT_OPTIMAL

DEFAULT_MONIOT = Path(__file__).resolve().parent / "moniot_table.json"


def divisor_lists(n: int) -> List[List[int]]:
    """divs[m] is every proper divisor of m (1 <= divisor < m), for m <= n."""
    divs: List[List[int]] = [[] for _ in range(n + 1)]
    for d in range(1, n // 2 + 1):
        for m in range(2 * d, n + 1, d):
            divs[m].append(d)
    return divs


# ---------------------------------------------------------------------------
# greedy: matching-based full-game approximation
# ---------------------------------------------------------------------------

def _augment(
    v: int,
    owner: Dict[int, int],
    selected: Set[int],
    divs: Sequence[List[int]],
    visited: Set[int],
    trail: List[Tuple[int, Optional[int]]],
) -> bool:
    """Kuhn's augmenting search: find v a divisor, reassigning others.

    Every owner change is recorded on the trail so it can be rolled back.
    """
    for f in reversed(divs[v]):
        if f in selected or f in visited:
            continue
        visited.add(f)
        holder = owner.get(f)
        if holder is None or _augment(holder, owner, selected, divs, visited, trail):
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
) -> Tuple[List[int], List[int]]:
    """Topologically order selections by the precedence relation.

    Returns (order, dropped): if the precedence graph has cycles, the
    smallest selection in each cycle is dropped until it is acyclic.
    """
    dropped: List[int] = []
    while True:
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
        if len(order) == len(selected):
            return order, dropped
        loser = min(c for c in selected if indeg[c] > 0)
        selected.discard(loser)
        del match[loser]
        dropped.append(loser)


def greedy(n: int, divs: Sequence[List[int]]) -> List[int]:
    """Greedy matching approximation of a full game; returns the sequence."""
    selected: Set[int] = set()
    owner: Dict[int, int] = {}  # divisor -> selection paying it

    def try_select(m: int) -> bool:
        # A failed Kuhn search leaves the matching untouched, so only
        # successful augments need their trails rolled back.
        pre_trail: List[Tuple[int, Optional[int]]] = []
        if m in owner:
            # m currently serves as someone's tax; try to rematch them.
            holder = owner.pop(m)
            if not _augment(holder, owner, selected, divs, {m}, pre_trail):
                owner[m] = holder
                return False
            pre_trail.append((m, holder))  # a rollback restores m's owner
        selected.add(m)  # blocks m from being used as anyone's tax below

        trail: List[Tuple[int, Optional[int]]] = []
        if not _augment(m, owner, selected, divs, set(), trail):
            selected.discard(m)  # no matching covers m: permanent reject
            _rollback(owner, pre_trail)
            return False
        if _is_acyclic(selected, owner, n):
            return True
        _rollback(owner, trail)

        # The matching creates a precedence cycle; retry with each of m's
        # divisors forced in turn, since a different assignment for m can
        # route the precedence differently.
        for f in reversed(divs[m]):
            if f in selected:
                continue
            trail = []
            if _augment(m, owner, selected, divs, set(divs[m]) - {f}, trail):
                if _is_acyclic(selected, owner, n):
                    return True
                _rollback(owner, trail)
        selected.discard(m)
        _rollback(owner, pre_trail)
        return False

    retry_later: List[int] = []
    for m in range(n, 1, -1):
        if not divs[m]:
            continue
        if not try_select(m):
            retry_later.append(m)
    for m in retry_later:
        try_select(m)

    match = {c: f for f, c in owner.items() if c in selected}
    order, dropped = _playable_order(selected, match, n)
    if dropped:
        raise RuntimeError(f"unexpected precedence cycle in game {n}")
    return order


# ---------------------------------------------------------------------------
# cascade: the verified upper-half theory applied band by band
# ---------------------------------------------------------------------------

def cascade(n: int, divs: Sequence[List[int]]) -> List[int]:
    """Approximate a full game with repeated upper-half mini games."""
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
    """Moniot's OneTax heuristic; returns the player's score."""
    in_pot = bytearray(n + 1)
    for i in range(1, n + 1):
        in_pot[i] = 1
    count = [len(divs[m]) for m in range(n + 1)]

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

        if refined:
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


STRATEGIES = ("greedy", "cascade", "onetax", "maxturn")


def play_all(n: int, divs: Sequence[List[int]]) -> Dict[str, int]:
    return {
        "greedy": check_sequence(n, greedy(n, divs)),
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
    results: Dict[int, Dict[str, int]] = {}

    sys.setrecursionlimit(100_000)
    for n in range(1, args.max_n + 1):
        if n not in optimal:
            break
        results[n] = play_all(n, divs)
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

    out = Path(__file__).resolve().parent / "approx_results.json"
    out.write_text(json.dumps(results, indent=0))
    print(f"\nper-game results written to {out.name} "
          f"({time.monotonic() - started:.0f}s total)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
