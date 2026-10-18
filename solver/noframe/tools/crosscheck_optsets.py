#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Cross-checks the Rust optsets collections of optimal pick sets against theory-free exhaustive search.

Each line of an optsets file lists, for one n, the optimal value ("score") and the optimal pick sets the Rust program
found using the playability theorems: a canonical set (the line's "canonical" if present, else the moves of
optimal.json's game) and "others", each given as (canonical - out) | in.  For every game that ended "complete" with
at most 200 sets and n <= --max, heuristics/evaluation/certify.py's `Exact` (a raw-pot-state brute force that
assumes no theory) recomputes the optimum and every optimal whole pick set, and both are compared with the line.  The
run stops at the first n whose brute force exceeds --budget seconds, or at the first disagreement (reported in full).

    crosscheck_optsets.py OPTSETS_FILE [--max N] [--budget SECONDS]
"""

import argparse
import gzip
import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "heuristics"))

from core import divisor_lists  # noqa: E402
from evaluation.bitpot import mask_of  # noqa: E402
from evaluation.certify import WHOLE_SET_CAP, Exact, OutOfTime  # noqa: E402

MAX_SETS = 200  # the program lists at most this many sets, so a line with more is not checkable


def rust_sets(row, optimal_moves):
    """The Rust collection of optimal pick sets for one line, as a set of frozensets."""
    canonical = frozenset(row.get("canonical", optimal_moves))
    sets = {canonical}
    sets.update((canonical - set(other["out"])) | set(other["in"]) for other in row["others"])
    assert len(sets) == row["sets"], f"n={row['n']}: line says {row['sets']} sets but {len(sets)} reconstructed"
    assert all(sum(s) == row["score"] for s in sets), f"n={row['n']}: a reconstructed set does not sum to the score"
    return sets


def show(sets):
    return [sorted(s) for s in sorted(sets, key=sorted)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("optsets_file")
    parser.add_argument("--max", type=int, default=40)
    parser.add_argument("--budget", type=float, default=30.0, help="seconds of brute force per game")
    args = parser.parse_args()

    opener = gzip.open if args.optsets_file.endswith(".gz") else open
    with opener(args.optsets_file, "rt") as f:
        rows = {r["n"]: r for r in map(json.loads, f) if r["n"] <= args.max}
    checkable = [n for n in sorted(rows) if rows[n]["ended"] == "complete" and rows[n]["sets"] <= MAX_SETS]
    optimal = {g["n"]: g["moves"] for g in json.loads((ROOT / "results/optimal.json").read_text())}
    divs = divisor_lists(max(checkable, default=1))
    sys.setrecursionlimit(100_000)

    started = time.monotonic()
    agree = multiple = 0
    verified = []
    for n in checkable:
        row = rows[n]
        solver = Exact(n, divs, time.monotonic() + args.budget)
        full_pot = mask_of(range(1, n + 1))
        try:
            true_opt = solver.best(full_pot)
            brute = solver.optimal_pick_sets(full_pot, cap=WHOLE_SET_CAP)
        except OutOfTime:
            print(f"n={n}: brute force exceeded the {args.budget:.0f}s budget, stopping here")
            break
        rust = rust_sets(row, optimal.get(n, []))
        if true_opt != row["score"] or brute != rust:
            print(f"DISAGREEMENT at n={n}: brute-force optimum {true_opt} vs Rust score {row['score']}")
            if brute is None:
                print(f"  brute-force optimal pick sets: more than {WHOLE_SET_CAP}")
            else:
                print(f"  brute-force optimal pick sets ({len(brute)}): {show(brute)}")
            print(f"  Rust optimal pick sets ({len(rust)}): {show(rust)}")
            return 1
        agree += 1
        multiple += len(rust) > 1
        verified.append(n)

    skipped = len(rows) - len(checkable)
    if not verified:
        print("no game was verified")
        return 1
    print(f"optsets agrees with brute force on {agree} games (n={verified[0]}..{verified[-1]}, score and whole set of "
          f"optimal pick sets), {multiple} with more than one optimal set; {skipped} lines not checkable "
          f"(incomplete or more than {MAX_SETS} sets); {time.monotonic() - started:.1f}s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
