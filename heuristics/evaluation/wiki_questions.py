#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""The two open questions of the wiki's Questions page, measured on the committed games and by exhaustive search.

solver/notebook/the-wikis-open-questions-2026-10/README.md has the method, the results and the proofs; this
module computes every number there.  The games read are optimal.json (1..1300) and the committed chain
results/strict.jsonl (its games 1..2175, TOP, which the study was made on; the file has more now).

* `tax`: how many numbers the taxman takes for one move (its *tax*).  Every committed game, the moves that
  pay three, the shapes a tax can have (a check of the entry's theorem), a reordering of every game that has a
  move paying three into one that pays at most two, and, by exhaustive search over every optimal game of the
  boards 2..--exhaustive, the least and the greatest largest tax an optimal game can have.
* `adapt`: the wiki's procedure for reusing the game of n - 1 (Reusing-a-previous-solution: play the game of n - 1,
  select n just before the move that would leave it no divisor, then the rest of the moves that are still legal),
  applied to the committed games; delta(n) = n + opt(n - 1) - opt(n) and what it decides; the Java's set-level reuse
  (keep the previous moves up to n/2, take the upper half of n's game) that the wiki's 70% counts; the patterns
  by n's factorization; and, by exhaustive search over every optimal game of n - 1 for n up to --exhaustive, how
  much the outcome depends on which game of n - 1 is adapted.

Usage (from heuristics/):
    python3 -m evaluation.wiki_questions tax [--exhaustive 40]
    python3 -m evaluation.wiki_questions adapt [--exhaustive 40]
"""

from __future__ import annotations

import argparse
import json
import math
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set, Tuple

from core import Infeasible, divisor_lists, maximal_factors, peel, prime_factors, smallest_prime_factors
from evaluation.bitpot import bits, mask_of, popcount
from evaluation.certify import Exact
from evaluation.verify import DEFAULT_OPTIMAL
from strategies.solvent import _playable_order

DEFAULT_CHAIN = DEFAULT_OPTIMAL.parent / "strict.jsonl"
TOP = 2175
TABLE_TOP = 1300  # the last game of optimal.json that the study reads; the file goes on with the chain's games


# --------------------------------------------------------------------------------------------------------------------
# shared


def load_games(optimal: Path, chain: Path) -> Tuple[Dict[int, List[int]], Dict[int, dict]]:
    """optimal.json's moves by n, to TABLE_TOP, and the chain's completed lines by n, to TOP."""
    table = {g["n"]: g["moves"] for g in json.loads(optimal.read_text()) if g["n"] <= TABLE_TOP}
    lines = {}
    for text in chain.read_text().splitlines():
        line = json.loads(text)
        if line.get("completed") and line["n"] <= TOP:
            lines[line["n"]] = line
    return table, lines


def taxes(n: int, moves: Sequence[int], divs: Sequence[List[int]]) -> Optional[List[List[int]]]:
    """The tax of each move of a game of n, or None if the moves are not a legal game."""
    pot = bytearray([1]) * (n + 1)
    out = []
    for c in moves:
        tax = [d for d in divs[c] if pot[d]]
        if not pot[c] or not tax:
            return None
        pot[c] = 0
        for d in tax:
            pot[d] = 0
        out.append(tax)
    return out


def playable_order(n: int, selected: Set[int], spf: Sequence[int]) -> Optional[List[int]]:
    """A legal order of a set of moves on the board 1..n, or None if none exists.

    The playability theorem (P2) and the peel theorem (P4), docs/theory/playability.md, say when one exists.
    """
    mf = {c: maximal_factors(c, spf) for c in selected}
    try:
        _, match = peel(selected, set(range(1, n + 1)) - selected, mf)
    except Infeasible:
        return None
    return _playable_order(set(selected), match, n)


def tax_shape(c: int, tax: Sequence[int], spf: Sequence[int]) -> bool:
    """The theorem's shapes: some of c's maximal factors c/p, or c/p and c/p^2 (with or without the first)."""
    mf = maximal_factors(c, spf)
    if set(tax) <= mf:
        return True
    return any(set(tax) <= {c // p, c // (p * p)} for p in prime_factors(c, spf) if c % (p * p) == 0)


# --------------------------------------------------------------------------------------------------------------------
# tax


def excess(tx: List[List[int]]) -> int:
    return sum(max(0, len(t) - 2) for t in tx)


def repair(n: int, moves: List[int], divs: Sequence[List[int]]) -> Optional[List[int]]:
    """Reorder a game so that no move pays three or more, one step at a time: a later multiple of a number of the
    tax to just before the move, or the move to just after it.  None if that gets stuck."""
    tx = taxes(n, moves, divs)
    while excess(tx):
        i = next(k for k, t in enumerate(tx) if len(t) > 2)
        step = None
        for j in range(i + 1, len(moves)):
            if not any(moves[j] % a == 0 for a in tx[i]):
                continue
            for trial in (moves[:i] + [moves[j]] + moves[i:j] + moves[j + 1:],
                          moves[:i] + moves[i + 1:j + 1] + [moves[i]] + moves[j + 1:]):
                t2 = taxes(n, trial, divs)
                if t2 is not None and excess(t2) < excess(tx):
                    step = (trial, t2)
                    break
            if step:
                break
        if step is None:
            return None
        moves, tx = step
    return moves


def tax_range(solver: Exact, pot: int, memo: Dict[int, Tuple[int, int]]) -> Tuple[int, int]:
    """(least, greatest) over the optimal continuations from pot of the largest tax a move pays."""
    hit = memo.get(pot)
    if hit is not None:
        return hit
    target = solver.best(pot)
    lo, hi = (0, 0) if target == 0 else (sys.maxsize, -1)
    for c in bits(pot) if target else ():
        tax = solver.dmask[c] & pot
        if not tax:
            continue
        child = solver.normalize(pot & ~tax & ~(1 << c))
        if c + solver.best(child) == target:
            clo, chi = tax_range(solver, child, memo)
            lo, hi = min(lo, max(popcount(tax), clo)), max(hi, max(popcount(tax), chi))
    memo[pot] = (lo, hi)
    return lo, hi


def heaviest_tax_game(solver: Exact, pot: int, memo: Dict[int, Tuple[int, int]]) -> List[Tuple[int, List[int]]]:
    """An optimal game from pot whose largest tax is the greatest any optimal game has, with its taxes."""
    want = tax_range(solver, pot, memo)[1]
    out = []
    target = solver.best(pot)
    while target:
        for c in sorted(bits(pot), reverse=True):
            tax = solver.dmask[c] & pot
            child = solver.normalize(pot & ~tax & ~(1 << c))
            if not tax or c + solver.best(child) != target:
                continue
            if want < 0 or max(popcount(tax), tax_range(solver, child, memo)[1]) == want:
                if popcount(tax) == want:
                    want = -1
                out.append((c, list(bits(tax))))
                pot, target = child, target - c
                break
    return out


def cmd_tax(args: argparse.Namespace) -> None:
    divs = divisor_lists(TOP)
    spf = smallest_prime_factors(TOP)
    table, lines = load_games(args.optimal, args.chain)
    files = (("optimal.json", table), ("strict.jsonl", {n: line["moves"] for n, line in lines.items()}))
    for name, games in files:
        sizes, worst, three, shapes = Counter(), Counter(), [], 0
        for n, moves in sorted(games.items()):
            tx = taxes(n, moves, divs)
            sizes.update(len(t) for t in tx)
            worst[max((len(t) for t in tx), default=0)] += 1
            three += [(n, c, t) for c, t in zip(moves, tx) if len(t) >= 3]
            shapes += sum(not tax_shape(c, t, spf) for c, t in zip(moves, tx))
        print(f"{name}: {len(games)} games, {sum(sizes.values())} moves; moves by tax "
              f"{sorted(sizes.items())}; games by largest tax {sorted(worst.items())}; taxes outside the theorem's "
              f"shapes: {shapes}")
        distinct = {}
        for n, c, t in three:
            distinct.setdefault((c, tuple(t)), []).append(n)
        for (c, t), ns in distinct.items():
            print(f"  {c} pays {list(t)} in the games of {ns[0]}..{ns[-1]} ({len(ns)} games)")

    repaired, stuck = 0, []
    for n, line in sorted(lines.items()):
        if excess(taxes(n, line["moves"], divs)):
            fixed = repair(n, line["moves"], divs)
            if fixed is None:
                stuck.append(n)
                continue
            assert sorted(fixed) == sorted(line["moves"]) and max(len(t) for t in taxes(n, fixed, divs)) <= 2
            repaired += 1
    print(f"reordered to pay at most two: {repaired} games; stuck: {stuck}")

    most_forced, two_payers = Counter(), {}
    for n, line in sorted(lines.items()):
        chosen = set(line["moves"])
        takers = Counter(d for s in chosen for d in divs[s] if d not in chosen)
        most_forced[max((sum(takers[d] == 1 for d in divs[s] if d not in chosen) for s in chosen), default=0)] += 1
        two_payers[n] = len(takers) - len(chosen)
    print(f"games by the most numbers one move alone can take: {sorted(most_forced.items())}")
    print("moves paying two in any order that pays at most two (taxed numbers less moves), every 250: "
          + ", ".join(f"{n}: {two_payers[n]}" for n in range(250, TOP + 1, 250)))

    sys.setrecursionlimit(100_000)
    first = None
    for n in range(2, args.exhaustive + 1):
        started = time.monotonic()
        solver = Exact(n, divs, started + 3600)
        root = solver.normalize(mask_of(range(1, n + 1)))
        memo: Dict[int, Tuple[int, int]] = {}
        lo, hi = tax_range(solver, root, memo)
        print(f"  exhaustive n={n}: opt {solver.best(root)}, largest tax of an optimal game: least {lo}, greatest {hi}"
              f" ({time.monotonic() - started:.1f} s)", flush=True)
        if hi >= 3 and first is None:
            first = (n, heaviest_tax_game(solver, root, memo))
    if first:
        print(f"first board with an optimal game paying three: {first[0]}: {first[1]}")


# --------------------------------------------------------------------------------------------------------------------
# adapt


def adapt(n: int, prev: Sequence[int], divs: Sequence[List[int]]) -> Tuple[List[int], int, Optional[int]]:
    """The wiki's procedure on a game of n - 1: (the game of n, the move it had to skip or 0, the move n was played
    before, or None if n came last)."""
    pot = bytearray([1]) * (n + 1)
    out, skipped, before = [], 0, None

    def play(c: int) -> bool:
        tax = [d for d in divs[c] if pot[d]]
        if not pot[c] or not tax:
            return False
        pot[c] = 0
        for d in tax:
            pot[d] = 0
        out.append(c)
        return True

    for m in prev:
        if before is None and all(d == m or d in divs[m] for d in divs[n] if pot[d]):
            before = m
            assert play(n)
        if not play(m):
            assert not skipped, "the procedure skips at most one move"
            skipped = m
    if before is None:
        assert play(n)
    return out, skipped, before


def prime_list(n: int, spf: Sequence[int]) -> List[int]:
    """The prime factors of n with multiplicity, ascending."""
    primes = []
    while n > 1:
        primes.append(spf[n])
        n //= spf[n]
    return primes


def kind(n: int, spf: Sequence[int]) -> str:
    """The classes of n the entry's table uses."""
    primes = prime_list(n, spf)
    if len(primes) == 1:
        return "prime"
    if n % 2 == 0:
        return "2p" if len(primes) == 2 else "even, other"
    return "odd pq" if len(primes) == 2 and primes[0] != primes[1] else "odd, other"


def adaptation_counts(solver: Exact, n: int, delta: int) -> Tuple[int, int]:
    """Over every optimal game of n - 1 (solver is the board n - 1): how many, and on how many the wiki's procedure
    is optimal for n (it is when the move it skips is worth delta, or it skips none and delta is 0)."""
    divn = mask_of(d for d in range(1, n) if n % d == 0)
    games: Dict[int, int] = {}
    good: Dict[int, int] = {}

    def optimal_moves(pot: int):
        target = solver.best(pot)
        for c in bits(pot) if target else ():
            tax = solver.dmask[c] & pot
            raw = pot & ~tax & ~(1 << c)
            child = solver.normalize(raw)
            if tax and c + solver.best(child) == target:
                yield c, tax, raw, child

    def count(pot: int) -> int:
        if pot not in games:
            games[pot] = sum(count(child) for _, _, _, child in optimal_moves(pot)) or 1
        return games[pot]

    def rec(pot: int) -> int:
        if pot in good:
            return good[pot]
        total, live, end = 0, divn & pot, True
        for c, tax, raw, child in optimal_moves(pot):
            end = False
            if live & ~(tax | 1 << c) == 0:  # c would leave n no divisor: n goes first, and c may lose its tax
                skipped = c if live >> c & 1 or tax & ~live == 0 else 0
                total += count(child) if skipped == delta else 0
            elif divn & raw & ~child:  # a divisor of n is left that nothing can take: n comes last
                total += count(child) if delta == 0 else 0
            else:
                total += rec(child)
        good[pot] = int(delta == 0) if end else total
        return good[pot]

    root = solver.normalize(mask_of(range(1, n)))
    return count(root), rec(root)


def cmd_adapt(args: argparse.Namespace) -> None:
    divs = divisor_lists(TOP)
    spf = smallest_prime_factors(TOP)
    table, lines = load_games(args.optimal, args.chain)
    opt = {n: line["score"] for n, line in lines.items()}

    for name, games, tops in (("optimal.json", table, (1000, 1300)), ("strict.jsonl", None, (1000, TOP))):
        for top in tops:
            how = Counter()
            for n in range(2, top + 1):
                moves, skipped, _ = adapt(n, games[n - 1] if games else lines[n - 1]["moves"], divs)
                ok = sum(moves) == opt[n]
                how["fails" if not ok else "lucky" if not skipped else "prime" if spf[n] == n else "general"] += 1
            print(f"wiki's procedure on {name}'s games, n = 2..{top}: optimal {top - 1 - how['fails']} of {top - 1} "
                  f"({100 * (top - 1 - how['fails']) / (top - 1):.1f}%): {dict(how)}")
    differ = [n for n in range(2, 1302) if (sum(adapt(n, table[n - 1], divs)[0]) == opt[n])
              != (sum(adapt(n, lines[n - 1]["moves"], divs)[0]) == opt[n])]
    print(f"n of 2..1301 where the two files' games of n - 1 disagree: {differ}")

    rows = []
    for n in range(2, TOP + 1):
        line, prev = lines[n], lines[n - 1]
        moves = adapt(n, prev["moves"], divs)[0]
        chosen = set(prev["moves"])
        delta = n + opt[n - 1] - opt[n]
        taken = {d for s in chosen for d in divs[s]}
        row = dict(n=n, delta=delta, kind=kind(n, spf), ok=sum(moves) == opt[n], proof=line["proof"],
                   swept_divisor=any(d not in chosen and d not in taken for d in divs[n]), half_chosen=n // 2 in chosen)
        if delta == 0 and not row["ok"]:
            order = playable_order(n, chosen | {n}, spf)
            if order is not None:  # the entry's theorem: n's game less n adapts
                again = adapt(n, [c for c in order if c != n], divs)[0]
                assert sum(again) == opt[n], n
                row["reordered"] = True
        upper = {c for c in line["moves"] if 2 * c > n}
        reuse = {c for c in chosen if 2 * c <= n} | upper
        order = playable_order(n, reuse, spf) if sum(reuse) == opt[n] else None
        row["java"] = order is not None and taxes(n, order, divs) is not None
        row["cofactor"] = n // max(prime_factors(n, spf))
        row["omega"] = len(prime_list(n, spf))
        digests = [(game.get("setup") or {}).get("digest") for game in (line, prev)]
        row["same_problem"] = digests[0] is not None and digests[0] == digests[1]
        rows.append(row)

    composite = [r for r in rows if r["kind"] != "prime"]
    zero = [r for r in composite if r["delta"] == 0]
    print(f"composite n of 2..{TOP}: {len(composite)}; delta = 0: {len(zero)} (the procedure optimal on the recorded "
          f"game {sum(r['ok'] for r in zero)}, of them with a divisor of n swept at the end of the game of n - 1 "
          f"{sum(r['ok'] and r['swept_divisor'] for r in zero)}; optimal after reordering the game of n - 1 "
          f"{sum(r.get('reordered', False) for r in zero)}; the recorded moves of n - 1 plus n unplayable "
          f"{[r['n'] for r in zero if not r['ok'] and not r.get('reordered')]})")
    pos = [r for r in composite if r["delta"] > 0]
    coprime = [r for r in pos if math.gcd(r["delta"], r["n"]) == 1]
    shared = Counter()
    for r in pos:
        n, delta, chosen = r["n"], r["delta"], set(lines[r["n"] - 1]["moves"])
        if math.gcd(delta, n) == 1:
            continue
        if r["ok"]:
            shared["optimal on the recorded game"] += 1
        elif delta not in chosen:
            shared["delta not a move of the recorded game"] += 1
        elif n % delta and not any(n % f == 0 for f in maximal_factors(delta, spf) - chosen):
            shared["no maximal factor of delta outside the moves divides n"] += 1
        else:
            shared["not settled"] += 1
    print(f"  delta > 0: {len(pos)}; coprime to n (the procedure fails on every optimal game of n - 1) "
          f"{len(coprime)}; sharing a factor {len(pos) - len(coprime)}: {dict(shared)}; the successes "
          f"{[(r['n'], r['delta']) for r in pos if r['ok']]}")

    for top in (1000, 1300, TOP):
        some = [r for r in rows if r["n"] <= top]
        java = sum(r["java"] or r["kind"] == "prime" for r in some)
        print(f"the Java's set-level reuse (primes, and keeping n - 1's moves up to n/2), n = 2..{top}: optimal "
              f"{java} of {len(some)} ({100 * java / len(some):.1f}%)")

    tab: Dict[str, Counter] = defaultdict(Counter)
    for r in rows:
        keys = [("class", r["kind"]), ("proof", r["proof"]), ("cofactor n / largest prime", min(r["cofactor"], 13)),
                ("prime factors with multiplicity (4: four or more), parity", min(r["omega"], 4), r["n"] % 2),
                ("search problem that of n - 1", r["same_problem"])]
        if r["n"] % 2 == 0 and r["kind"] != "prime":
            keys.append(("even n: n/2 a move of the game of n - 1", r["half_chosen"]))
        for name, *value in keys:
            value = tuple(value)
            tab[name][(value, "games")] += 1
            tab[name][(value, "wiki")] += r["ok"]
            tab[name][(value, "java")] += r["java"] or r["kind"] == "prime"
            tab[name][(value, "delta 0")] += r["delta"] == 0
    for name, c in tab.items():
        print(f"by {name} (games: wiki's procedure, Java's reuse, delta = 0):")
        for value in sorted({v for v, _ in c}, key=str):
            print(f"  {value}: {c[value, 'games']}: {c[value, 'wiki']}, {c[value, 'java']}, {c[value, 'delta 0']}")

    primes = [p for p in range(2, TOP + 1) if spf[p] == p]
    below = dict(zip(primes[1:], primes))
    semi = Counter()
    for r in rows:
        if r["kind"] == "odd pq":
            shifted = math.prod(below[p] for p in prime_factors(r["n"], spf))
            semi[(2 * shifted > r["n"], r["delta"] == shifted)] += 1
            if r["delta"] != shifted:
                semi[("not", r["n"], r["delta"])] += 1
    print("odd pq, by (p'q' > pq/2, delta = p'q'), p' and q' the primes just below p and q: "
          f"{sorted(semi.items(), key=str)}")

    sys.setrecursionlimit(100_000)
    prev_solver = None
    for n in range(2, args.exhaustive + 1):
        started = time.monotonic()
        solver = Exact(n, divs, started + 3600)
        best = solver.best(solver.normalize(mask_of(range(1, n + 1))))
        if prev_solver is not None:
            delta = n + prev_solver[1] - best
            total, good = adaptation_counts(prev_solver[0], n, delta)
            print(f"  exhaustive n={n}: delta {delta}; optimal games of n - 1: {total}, on which the procedure is "
                  f"optimal: {good} ({100 * good / total:.1f}%) ({time.monotonic() - started:.1f} s)", flush=True)
        prev_solver = (solver, best)


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--chain", type=Path, default=DEFAULT_CHAIN)
    sub = parser.add_subparsers(dest="command", required=True)
    for name, run in (("tax", cmd_tax), ("adapt", cmd_adapt)):
        command = sub.add_parser(name)
        command.add_argument("--exhaustive", type=int, default=40, help="search every optimal game up to this board")
        command.set_defaults(run=run)
    args = parser.parse_args(argv)
    args.run(args)
    return 0


if __name__ == "__main__":
    sys.exit(main())
