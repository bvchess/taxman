"""Does OneTax get the upper half right?

For each game this script compares the numbers greater than N/2 that
OneTax selects against the (provably optimal) upper half produced by
solve_upper_half, and splits OneTax's loss into an upper-half part and
a lower-half part.  It also tests the obvious hybrid: run OneTax but
forbid it from picking upper-half numbers outside the optimal upper set
(they remain available as tax).

Usage:
    python3 upper_fidelity.py [--max-n 1000] [--optimal PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import List

from approx import (
    check_sequence, divisor_lists, one_tax, one_tax_forced_upper,
    one_tax_oracle,
)
from taxman_mini import smallest_prime_factors, solve_upper_half
from verify import DEFAULT_OPTIMAL


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--max-n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    divs = divisor_lists(args.max_n)
    spf = smallest_prime_factors(args.max_n)

    per_game: List[dict] = []
    games = wrong_upper = 0
    upper_loss_total = lower_loss_total = 0
    hybrid_wins = hybrid_ties = hybrid_losses = 0
    onetax_optimal_wrong_upper = 0
    worst: List[tuple] = []
    scores = {"onetax": 0, "hybrid": 0, "oracle": 0, "opt": 0}

    for n in range(2, args.max_n + 1):
        if n not in optimal or optimal[n]["score"] == 0:
            continue
        games += 1
        opt_score = optimal[n]["score"]
        upper_opt = {m for m in optimal[n]["moves"] if m > n / 2}

        seq: List[int] = []
        onetax_score = one_tax(n, divs, sequence=seq)
        assert check_sequence(n, seq) == onetax_score
        upper_onetax = {m for m in seq if m > n / 2}

        # The provably optimal upper set (verified against optimal.json).
        upper_exact, _ = solve_upper_half(n, spf)
        assert set(upper_exact) == upper_opt

        hybrid_score, hybrid_seq, forced = one_tax_forced_upper(n, divs, spf)
        assert check_sequence(n, hybrid_seq) == hybrid_score
        assert {m for m in hybrid_seq if m > n / 2} == upper_opt

        oracle_score, oracle_seq = one_tax_oracle(n, divs, spf)
        assert check_sequence(n, oracle_seq) == oracle_score
        assert oracle_score >= max(onetax_score, hybrid_score)

        per_game.append({
            "oracle": oracle_score,
            "n": n,
            "opt": opt_score,
            "onetax": onetax_score,
            "hybrid": hybrid_score,
            "upper_opt_sum": sum(upper_opt),
            "upper_onetax_sum": sum(upper_onetax),
            "onetax_missed": len(upper_opt - upper_onetax),
            "hybrid_forced": forced,
        })

        upper_loss = sum(upper_opt) - sum(upper_onetax)
        total_loss = opt_score - onetax_score
        if upper_onetax != upper_opt:
            wrong_upper += 1
            upper_loss_total += upper_loss
            if onetax_score == opt_score:
                onetax_optimal_wrong_upper += 1
            worst.append((upper_loss, n, sorted(upper_opt - upper_onetax),
                          sorted(upper_onetax - upper_opt)))
        lower_loss_total += total_loss - upper_loss

        scores["onetax"] += onetax_score
        scores["hybrid"] += hybrid_score
        scores["oracle"] += oracle_score
        scores["opt"] += opt_score
        if hybrid_score > onetax_score:
            hybrid_wins += 1
        elif hybrid_score == onetax_score:
            hybrid_ties += 1
        else:
            hybrid_losses += 1

    print(f"games examined:                 {games}")
    print(f"OneTax upper half != optimal:   {wrong_upper} games "
          f"({100 * wrong_upper / games:.1f}%)")
    print(f"  ...while still scoring optimal: {onetax_optimal_wrong_upper}")
    print(f"OneTax loss from the upper half: {upper_loss_total} points")
    print(f"OneTax loss from the lower half: {lower_loss_total} points")
    print()
    worst.sort(reverse=True)
    print("largest upper-half losses (points, n, missed, extra):")
    for loss, n, missed, extra in worst[:8]:
        print(f"  {loss:>5} n={n:<5} missed {missed} took {extra}")
    print()
    print("hybrid = OneTax forced to play the optimal upper half (any pick "
          "must leave the remaining upper selections solvable):")
    print(f"  hybrid > onetax in {hybrid_wins} games, "
          f"= in {hybrid_ties}, < in {hybrid_losses}")
    print(f"  total points: onetax {scores['onetax']}, "
          f"hybrid {scores['hybrid']}, optimal {scores['opt']}")
    print(f"  share of optimal: onetax "
          f"{100 * scores['onetax'] / scores['opt']:.3f}%, hybrid "
          f"{100 * scores['hybrid'] / scores['opt']:.3f}%, oracle "
          f"{100 * scores['oracle'] / scores['opt']:.3f}%")

    out = Path(__file__).resolve().parent / "fidelity_results.json"
    out.write_text(json.dumps(per_game, indent=0))
    print(f"per-game results written to {out.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
