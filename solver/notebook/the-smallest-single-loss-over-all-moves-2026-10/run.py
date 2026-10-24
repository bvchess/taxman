#!/usr/bin/env python3
"""The one-loss rule over n = 2..4955: for each n, is moves(n-1) + {n} playable (exact), else the smallest x whose
removal makes it playable, else none.

Run from the top of the repository:

    PYTHONPATH=heuristics nice -n 19 python3 PATH/run.py --out OUT.jsonl [--lo N] [--hi N] [--step K] [--also N ...]
        [--procs P] [--mode core|brute|both]

Mode core (the default) tries only the moves of one minimal unplayable subset, brute tries every x upward, and both
runs the two and asserts that they agree.
"""

from __future__ import annotations

import argparse
import json
import multiprocessing
import time
from pathlib import Path

from swaplib import TOP, Playability, load_games, one_loss_brute, one_loss_core

PL = None
GAMES = None
ARGS = None


def init() -> None:
    global PL
    PL = Playability(TOP)


def solve(n: int) -> dict:
    prev, cur = GAMES[n - 1], GAMES[n]
    started = time.monotonic()
    calls0 = PL.calls
    total = set(prev["moves"]) | {n}
    row = {"n": n, "opt_prev": prev["score"], "opt": cur["score"]}
    if PL.playable(n, total):
        row["kind"] = "exact"
        order = PL.order(n, total)
        assert order is not None and PL.replay(n, order) and sum(order) == prev["score"] + n
        row["swap_score"] = prev["score"] + n
    else:
        x = one_loss_brute(PL, n, prev["moves"]) if ARGS.mode in ("brute", "both") else None
        y = one_loss_core(PL, n, prev["moves"]) if ARGS.mode in ("core", "both") else None
        if ARGS.mode == "both":
            assert x == y, (n, x, y)
        x = x if ARGS.mode in ("brute", "both") else y
        if x is None:
            row["kind"] = "none"
        else:
            row["kind"] = "one_loss"
            row["x_min"] = x
            row["swap_score"] = prev["score"] + n - x
            order = PL.order(n, total - {x})
            assert order is not None and PL.replay(n, order) and sum(order) == row["swap_score"]
    row["peel_calls"] = PL.calls - calls0
    row["seconds"] = round(time.monotonic() - started, 3)
    return row


def main() -> None:
    global GAMES, ARGS
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("brute", "core", "both"), default="core")
    parser.add_argument("--lo", type=int, default=2)
    parser.add_argument("--hi", type=int, default=TOP)
    parser.add_argument("--step", type=int, default=1, help="take every step-th n of lo..hi")
    parser.add_argument("--also", type=int, nargs="*", default=[], help="and these n")
    parser.add_argument("--procs", type=int, default=1)
    parser.add_argument("--out", type=Path, required=True)
    ARGS = parser.parse_args()
    GAMES = load_games(TOP)
    ns = sorted(set(range(ARGS.lo, ARGS.hi + 1, ARGS.step)) | set(ARGS.also))
    started = time.monotonic()
    with ARGS.out.open("w") as out:
        if ARGS.procs > 1:
            with multiprocessing.get_context("fork").Pool(ARGS.procs, initializer=init) as pool:
                for row in pool.imap(solve, reversed(ns), chunksize=4):
                    out.write(json.dumps(row) + "\n")
                    out.flush()
        else:
            init()
            for n in ns:
                out.write(json.dumps(solve(n)) + "\n")
                out.flush()
    print(f"{len(ns)} n in {time.monotonic() - started:.0f} s")


if __name__ == "__main__":
    main()
