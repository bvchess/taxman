# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Verify the upper-half theory against known optimal Taxman solutions.

Theory: for every game N there is a polynomial-time algorithm (optimize_mini
followed by solve_mini on the upper-half factor game, see
https://github.com/bvchess/taxman/wiki/Taxman-Mini) that produces exactly
the numbers greater than N/2 in the optimal solution, along with an order
in which those numbers can all be selected.

For each game this script checks:

  1. Set match: the selections chosen by optimize_mini equal the set of
     numbers greater than N/2 in the known optimal solution.
  2. Playable: replaying solve_mini's sequence as a real taxman game over
     the pot optC + rF, where each selection removes every remaining divisor
     from the pot, pays at least one factor of tax per selection and selects
     every number in optC.
  3. Order match (informational only): whether solve_mini's order equals the
     relative order of the >N/2 numbers in the known optimal solution.
     Optimal orderings are not unique, so a difference is not a failure.

Usage:
    python3 -m evaluation.verify [--max-n 1000] [--optimal PATH] [-v]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import List, Sequence, Set

from core import smallest_prime_factors, solve_upper_half

DEFAULT_OPTIMAL = Path(__file__).resolve().parent.parent.parent / "src/main/resources/optimal.json"


def replay(sequence: Sequence[int], pot: Set[int]) -> bool:
    """Play the sequence as a real taxman game over the pot.

    Every selection must pay at least one factor of tax; tax removes every
    remaining divisor of the selection from the pot.
    """
    pot = set(pot)
    for c in sequence:
        if c not in pot:
            return False
        tax = {d for d in pot if d != c and c % d == 0}
        if not tax:
            return False
        pot -= tax
        pot.remove(c)
    return True


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--max-n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("-v", "--verbose", action="store_true",
                        help="print every game, not just failures")
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    spf = smallest_prime_factors(args.max_n)

    games = 0
    set_failures: List[int] = []
    replay_failures: List[int] = []
    order_differences: List[int] = []
    started = time.monotonic()

    for n in range(1, args.max_n + 1):
        if n not in optimal:
            print(f"n={n}: no known optimal solution, stopping")
            break
        games += 1

        expected_order = [m for m in optimal[n]["moves"] if m > n / 2]
        expected_set = set(expected_order)

        sequence, tax_pool = solve_upper_half(n, spf)
        got_set = set(sequence)

        ok_set = got_set == expected_set
        ok_replay = replay(sequence, got_set | tax_pool)
        same_order = sequence == expected_order

        if not ok_set:
            set_failures.append(n)
            print(f"n={n}: SET MISMATCH")
            print(f"  expected {sorted(expected_set)}")
            print(f"  got      {sorted(got_set)}")
            print(f"  missing {sorted(expected_set - got_set)}, "
                  f"extra {sorted(got_set - expected_set)}")
        if not ok_replay:
            replay_failures.append(n)
            print(f"n={n}: SEQUENCE NOT PLAYABLE: {sequence} "
                  f"with factors {sorted(tax_pool)}")
        if not same_order:
            order_differences.append(n)
        if args.verbose:
            print(f"n={n}: set={'ok' if ok_set else 'FAIL'} "
                  f"playable={'ok' if ok_replay else 'FAIL'} "
                  f"order={'same' if same_order else 'differs'} {sequence}")

    elapsed = time.monotonic() - started
    print()
    print(f"games checked:        {games} (in {elapsed:.1f}s)")
    print(f"set matches:          {games - len(set_failures)}/{games}")
    print(f"playable sequences:   {games - len(replay_failures)}/{games}")
    print(f"identical order:      {games - len(order_differences)}/{games} "
          f"(informational; optimal orderings are not unique)")
    if set_failures:
        print(f"set failures at:      {set_failures}")
    if replay_failures:
        print(f"replay failures at:   {replay_failures}")

    return 1 if (set_failures or replay_failures) else 0


if __name__ == "__main__":
    sys.exit(main())
