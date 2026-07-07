"""Performance measurements for every solver and strategy in this project.

Times each component at a range of game sizes (best of `--repeat` runs),
reports an empirical scaling exponent k (time ~ n^k) fit between the
smallest and largest sample, and profiles the most expensive strategy to
show where its time goes.

Usage:
    python3 bench.py [--sizes 125,250,500,1000] [--repeat 3]
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

from approx import (
    cascade, divisor_lists, solvent, max_turn, one_tax, one_tax_forced_upper,
    one_tax_oracle,
)
from taxman_mini import smallest_prime_factors, solve_upper_half


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
    spf = smallest_prime_factors(top)

    subjects = [
        ("onetax", lambda n: one_tax(n, divs)),
        ("maxturn", lambda n: max_turn(n, divs)),
        ("upper half (solve_upper_half)", lambda n: solve_upper_half(n, spf)),
        ("cascade", lambda n: cascade(n, divs)),
        ("solvent", lambda n: solvent(n, divs)),
        ("hybrid (forced upper)", lambda n: one_tax_forced_upper(n, divs, spf)),
        ("oracle (fork)", lambda n: one_tax_oracle(n, divs, spf)),
    ]

    header = f"{'per-game seconds':<31}" + "".join(f"{f'n={s}':>10}" for s in sizes)
    print(header + f"{'~n^k':>7}")
    for name, fn in subjects:
        times = [best_time(lambda s=s: fn(s), args.repeat) for s in sizes]
        k = (math.log(times[-1] / times[0]) / math.log(sizes[-1] / sizes[0])
             if times[0] > 0 else float("nan"))
        row = f"{name:<31}" + "".join(f"{t:>10.4f}" for t in times)
        print(row + f"{k:>7.2f}")

    print(f"\nprofile of one_tax_oracle(n={top}), top functions by cumulative time:")
    profiler = cProfile.Profile()
    profiler.enable()
    one_tax_oracle(top, divs, spf)
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
