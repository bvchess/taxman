"""Comparison runner: score every strategy against known optimal solutions.

Plays solvent, solvent-b, cascade, and Moniot's onetax/maxturn heuristics on
every game covered by optimal.json, and reports how each compares -- both
against the known optimum and against Robert Moniot's own strategy-comparison
table (https://www.dsm.fordham.edu/~moniot/taxman-strategies-comparison.html).

NOTE on cost: solvent-b reruns solvent's O(n^2 log n) scan up to
MAX_ITERS * |audit set| times per game (prefix-reuse forking removes the
"recompute the shared prefix per candidate" waste, but every candidate
still needs its own rescan below the fork point) -- measured ~9x solvent's
own cost at n=500 and ~59x at n=1000, growing with n.  There is no cost
guard here (none of the other strategies have one either); a full
--max-n 1000 sweep including solvent-b will take noticeably longer than
without it.

Usage:
    python3 -m evaluation.scoreboard [--max-n 1000] [--optimal PATH] [--moniot PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Sequence

from core import check_sequence, divisor_lists, maximal_factor_lists
from evaluation.verify import DEFAULT_OPTIMAL
from strategies.cascade import cascade
from strategies.maxturn import max_turn
from strategies.onetax import one_tax
from strategies.solvent import solvent
from strategies.solvent_b import solvent_b

DEFAULT_MONIOT = Path(__file__).resolve().parent.parent / "results" / "moniot_table.json"

STRATEGIES = ("solvent", "solvent_b", "cascade", "onetax", "maxturn")
DISPLAY_NAME = {"solvent_b": "solvent-b"}  # dict keys must be identifiers; display differs


def play_all(
    n: int, divs: Sequence[List[int]], mf: Sequence[List[int]]
) -> Dict[str, int]:
    return {
        "solvent": check_sequence(n, solvent(n, mf)),
        "solvent_b": check_sequence(n, solvent_b(n, mf)),
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
        label = DISPLAY_NAME.get(name, name)
        print(f"{label:<9} mean {sum(pcts) / len(pcts):6.2f}%   "
              f"min {min(pcts):6.2f}%   optimal in {exact}/{len(pcts)} games")

    print()
    header = f"{'n':>5} {'optimal':>8}"
    for name in STRATEGIES:
        header += f" {DISPLAY_NAME.get(name, name):>9} {'%':>6}"
    print(header)
    for n in (21, 50, 100, 128, 250, 500, 750, 1000):
        if n not in scored:
            continue
        r = scored[n]
        line = f"{n:>5} {r['opt']:>8}"
        for name in STRATEGIES:
            line += f" {r[name]:>9} {100 * r[name] / r['opt']:>6.2f}"
        print(line)

    out = Path(__file__).resolve().parent.parent / "strategies_out.json"
    out.write_text(json.dumps(results, indent=0))
    print(f"\nper-game results written to {out.name} "
          f"({time.monotonic() - started:.0f}s total)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
