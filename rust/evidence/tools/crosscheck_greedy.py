#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Cross-checks the Rust greedy (the span campaign's kept set) against the Python reference.

For each game in the span file whose n is selected, U* is recomputed with heuristics/core.py's optimize_mini
(peel-based descending greedy, sharing no code with the Rust crate) and its size and sum are compared with the
file's `kept` and `kept_sum`.  For n <= 1000 the sum is also compared with the upper half of the optimal game in
optimal.json.

    crosscheck_greedy.py SPAN_FILE [--max N] [--every K]
"""

import argparse
import gzip
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "heuristics"))

from core import optimize_mini, smallest_prime_factors, upper_half_game  # noqa: E402


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("span_file")
    parser.add_argument("--max", type=int, default=1500)
    parser.add_argument("--every", type=int, default=1)
    args = parser.parse_args()
    opener = gzip.open if args.span_file.endswith(".gz") else open
    with opener(args.span_file, "rt") as f:
        rows = {r["n"]: r for r in map(json.loads, f) if r["n"] <= args.max}
    optimal = {g["n"]: g for g in json.loads((ROOT / "src/main/resources/optimal.json").read_text())}
    spf = smallest_prime_factors(args.max)
    checked = with_optimal = 0
    for n in sorted(rows):
        if n % args.every and n != max(rows):
            continue
        c_set, f_set, mf = upper_half_game(n, spf)
        kept, _ = optimize_mini(c_set, f_set, mf)
        row = rows[n]
        assert (len(kept), sum(kept)) == (row["kept"], row["kept_sum"]), (n, len(kept), sum(kept), row)
        checked += 1
        if n in optimal:
            upper = sum(m for m in optimal[n]["moves"] if 2 * m > n)
            assert upper == sum(kept), (n, upper, sum(kept))
            with_optimal += 1
    print(f"greedy agrees with core.optimize_mini on {checked} games (n <= {args.max}); "
          f"with the upper half of optimal.json's game on {with_optimal}")


if __name__ == "__main__":
    main()
