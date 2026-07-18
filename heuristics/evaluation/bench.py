# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Performance measurements for every solver and strategy in this project.

Times each component at a range of game sizes (best of `--repeat` runs),
reports an empirical scaling exponent k (time ~ n^k) fit between the
smallest and largest sample, and profiles the most expensive strategy to
show where its time goes.

Usage:
    python3 -m evaluation.bench [--sizes 125,250,500,1000] [--repeat 3]
"""

from __future__ import annotations

import argparse
import cProfile
import io
import math
import pstats
import sys
import time
from typing import Callable, List

from core import (
    divisor_lists, maximal_factor_lists, smallest_prime_factors,
    solve_upper_half,
)
from strategies.cascade import cascade
from strategies.maxturn import max_turn
from strategies.onetax import one_tax
from strategies.solvent import solvent


def best_time(fn: Callable[[], object], repeat: int) -> float:
    best = math.inf
    for _ in range(repeat):
        started = time.perf_counter()
        fn()
        best = min(best, time.perf_counter() - started)
    return best


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sizes", default="125,250,500,1000")
    parser.add_argument("--repeat", type=int, default=3)
    args = parser.parse_args(argv)
    sizes = [int(s) for s in args.sizes.split(",")]

    top = max(sizes)
    divs = divisor_lists(top)
    mf = maximal_factor_lists(top)
    spf = smallest_prime_factors(top)

    subjects = [
        ("onetax", lambda n: one_tax(n, divs)),
        ("maxturn", lambda n: max_turn(n, divs)),
        ("upper half (solve_upper_half)", lambda n: solve_upper_half(n, spf)),
        ("cascade", lambda n: cascade(n, divs)),
        ("solvent", lambda n: solvent(n, mf)),
    ]

    header = f"{'per-game seconds':<31}" + "".join(f"{f'n={s}':>10}" for s in sizes)
    print(header + f"{'~n^k':>7}")
    for name, fn in subjects:
        times = [best_time(lambda s=s: fn(s), args.repeat) for s in sizes]
        k = (math.log(times[-1] / times[0]) / math.log(sizes[-1] / sizes[0])
             if times[0] > 0 else float("nan"))
        row = f"{name:<31}" + "".join(f"{t:>10.4f}" for t in times)
        print(row + f"{k:>7.2f}")

    print(f"\nprofile of solvent(n={top}), top functions by cumulative time:")
    profiler = cProfile.Profile()
    profiler.enable()
    solvent(top, mf)
    profiler.disable()
    out = io.StringIO()
    stats = pstats.Stats(profiler, stream=out)
    stats.sort_stats("cumulative").print_stats(12)
    for line in out.getvalue().splitlines():
        if line.strip() and ("ncalls" in line or "/" in line or "{" in line):
            print("  " + line.strip()[:110])
    return 0


if __name__ == "__main__":
    sys.exit(main())
