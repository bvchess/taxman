#!/usr/bin/env python3
"""Planning for rust/tools/cloud/sweep.sh, which solves a stretch of board sizes in blocks of consecutive games, each
block on one thread, and then goes over the games again so that each sees the game before it whichever block made it.
Standard library only; it shares no code with the Rust crate.

usage:
  sweep_plan.py blocks FIRST LAST BLOCKS GAME_SECONDS
      Prints the blocks, "A B" a line, that split FIRST..LAST into at most BLOCKS blocks of consecutive games whose
      expected work is about equal.  The work of a game is its setup, which grows steeply with n (about 1 second at
      2200 on one thread, 3 at 3000, 9 at 4000 and 48 at 6000, interpolated between and extended past those with the
      nearest exponent), and the limit on its search, GAME_SECONDS: a search is stopped at its limit, so that is the
      most it costs.

  sweep_plan.py excerpt RESULTS A B OUT
      Writes to OUT the part of the results file RESULTS that `taxman solve A-B --keep-going --retry-improved --out
      OUT` needs, and prints "LINES TODO": the number of lines written (the solver appends after them) and the number
      of games of A..B that it will search again first (none: nothing to go over, and the job is not worth starting;
      see `excerpt` for which).  `solve` opens its --out file with the checks of `taxman verify-results`, so the excerpt must hold
      whatever each of its lines rests on: all the lines of A..B, the completed games of the 8 board sizes before A
      (the games a repeat can repeat) and the best unfinished search of A - 1 (its seed), and, for every completed line
      taken, the game of n - 1 it names (the completed game, or the unfinished search whose score and moves it
      records) and the game it repeats, and so on.  The lines are copied as they are, in their order in RESULTS.

  sweep_plan.py passes RESULTS PLAN OUTDIR
      `excerpt` for every block of the plan file PLAN ("A B" a line, as `blocks` prints it) that has a game to search
      again, from one reading of RESULTS: writes OUTDIR/job-NNN.jsonl (NNN the block's number, from 1) and
      OUTDIR/job-NNN.count (the number of lines written), and prints "NNN A B LINES TODO" for each (NNN three digits).
"""
import argparse
import json
import math
import os
import sys

# Seconds of setup of one game on one thread, measured on a Graviton4 core (rust/tools/cloud/RUNBOOK.md, section 5).
SETUP_SECONDS = [(2200, 1.0), (3000, 3.0), (4000, 9.0), (6000, 48.0)]
SAME_SEARCH_WINDOW = 8  # solver::SAME_SEARCH_WINDOW: how far back a game looks for a search to repeat


def setup_seconds(n):
    """The setup of game n, by a power law between the measured points and beyond them from the nearest two."""
    points = SETUP_SECONDS
    segment = len(points) - 2
    for k in range(len(points) - 2):
        if n < points[k + 1][0]:
            segment = k
            break
    (n0, s0), (n1, s1) = points[segment], points[segment + 1]
    return s0 * (n / n0) ** (math.log(s1 / s0) / math.log(n1 / n0))


def game_seconds(n, limit):
    """What a game can cost: its setup and the limit of its search."""
    return setup_seconds(n) + limit


def split_blocks(first, last, blocks, limit):
    """[(a, b)] blocks of consecutive games of first..last, each about the same work; a game is in the block that its
    midpoint (in the running total of the work) falls in."""
    games = last - first + 1
    blocks = max(1, min(blocks, games))
    costs = [game_seconds(n, limit) for n in range(first, last + 1)]
    per_block = sum(costs) / blocks
    out = []
    done = 0.0
    for n, cost in zip(range(first, last + 1), costs):
        k = min(blocks - 1, int((done + cost / 2) / per_block))
        if out and out[-1][2] == k:
            out[-1][1] = n
        else:
            out.append([n, n, k])
        done += cost
    return [(a, b) for a, b, _ in out]


def digest(moves):
    """64-bit FNV-1a of the moves written in decimal and separated by commas, as 16 hex digits."""
    h = 0xCBF29CE484222325
    for byte in ",".join(map(str, moves)).encode():
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def read_results(path):
    """([text of each line kept], {n: completed summary}, {n: [unfinished summaries]}); the lines are numbered from 0
    in the list, and a blank or torn last line is dropped.  A summary is the dict of the fields the plan uses."""
    texts, completed, unfinished = [], {}, {}
    with open(path, encoding="utf-8") as f:
        raw = f.read().split("\n")
    for text in raw:
        if not text.strip():
            continue
        try:
            row = json.loads(text)
        except json.JSONDecodeError:
            break  # a torn last line (or a bad one, which the referees will name)
        number = len(texts)
        texts.append(text + "\n")
        summary = {"line": number, "n": row["n"], "prev": row.get("prev")}
        if row.get("completed"):
            summary["score"] = row["score"]
            summary["same"] = (row.get("same_search") or {}).get("n")
            completed[row["n"]] = summary
        else:
            best = row.get("best_so_far")
            if best is None:
                continue  # a line with no game in it: nothing to seed from, and nothing to keep
            summary["best"] = best
            summary["digest"] = digest(row["moves"])
            unfinished.setdefault(row["n"], []).append(summary)
    return texts, completed, unfinished


def best_unfinished(lines):
    """The best of the unfinished searches of a game: the highest score, the later of equals."""
    return max(lines, key=lambda u: (u["best"], u["line"]))


def built_on(line):
    """(score, proven) of the game of n - 1 an unfinished line was built on, or None if it had none."""
    prev = line["prev"]
    if prev is None:
        return None
    return prev["score"], prev["proof"] is not None or prev["source"] == "optimal"


def improved(last, on):
    """Whether an unfinished search that was built on `on` (see built_on) has more to give now that the game before it
    is `last` ((score, proven) or None): the rule of `taxman solve --retry-improved` (solver/window.rs, `improved`)."""
    if last is None:
        return False
    if on is None:
        return True
    if on[1]:
        return False
    return last[1] or last[0] > on[0]


def state_of(n, completed, unfinished):
    """(score, proven) of the game of n in the results, as the next game sees it: the completed game, else the best
    unfinished search; None if the results have neither."""
    if n in completed:
        return completed[n]["score"], True
    if n in unfinished:
        return best_unfinished(unfinished[n])["best"], False
    return None


def excerpt(texts, completed, unfinished, a, b):
    """(the numbers of the lines to take, the number of games of a..b that the solver searches again first: those with
    only unfinished searches whose game before it, as the results hold it, has more to give than the latest search of
    the game was built on).  The games that follow one it searches again may have more to give by then too, which only the
    solver sees as it goes."""
    take = set()
    for n in range(max(1, a - SAME_SEARCH_WINDOW), a):
        if n in completed:
            take.add(completed[n]["line"])
    if a - 1 not in completed and a - 1 in unfinished:
        take.add(best_unfinished(unfinished[a - 1])["line"])
    todo = 0
    for n in range(a, b + 1):
        if n in completed:
            take.add(completed[n]["line"])
        for u in unfinished.get(n, []):
            take.add(u["line"])
        if n not in completed and n in unfinished:
            # What the game had is what its latest attempt had, not the attempt that scored best.
            on = built_on(max(unfinished[n], key=lambda u: u["line"]))
            if improved(state_of(n - 1, completed, unfinished), on):
                todo += 1
    by_line = {s["line"]: s for s in completed.values()}
    queue = [s for s in (by_line[i] for i in take if i in by_line)]
    while queue:
        s = queue.pop()
        wanted = []
        prev = s["prev"]
        if prev is not None and prev["source"] in ("run", "results"):
            if prev["proof"] is not None:
                if s["n"] - 1 in completed:
                    wanted.append(completed[s["n"] - 1])
            else:
                for u in unfinished.get(s["n"] - 1, []):
                    if u["line"] < s["line"] and u["best"] == prev["score"] and u["digest"] == prev["moves_digest"]:
                        wanted.append(u)
                        break
        if s.get("same") is not None and s["same"] in completed:
            wanted.append(completed[s["same"]])
        for w in wanted:
            if w["line"] not in take:
                take.add(w["line"])
                if w["line"] in by_line:
                    queue.append(w)
    return sorted(take), todo


def read_plan(path):
    """[(a, b)] of a plan file: "A B" a line, as `blocks` prints it."""
    with open(path, encoding="utf-8") as f:
        return [tuple(map(int, line.split())) for line in f if line.strip()]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="command", required=True)
    p = sub.add_parser("blocks")
    for name in ("first", "last", "blocks"):
        p.add_argument(name, type=int)
    p.add_argument("game_seconds", type=float)
    p = sub.add_parser("excerpt")
    p.add_argument("results")
    p.add_argument("first", type=int)
    p.add_argument("last", type=int)
    p.add_argument("out")
    p = sub.add_parser("passes")
    p.add_argument("results")
    p.add_argument("plan")
    p.add_argument("outdir")
    args = ap.parse_args()
    if args.command == "blocks":
        if not 1 <= args.first <= args.last or args.blocks < 1:
            ap.error("blocks needs 1 <= FIRST <= LAST and BLOCKS >= 1")
        for a, b in split_blocks(args.first, args.last, args.blocks, args.game_seconds):
            print(a, b)
        return 0
    texts, completed, unfinished = read_results(args.results)
    if args.command == "passes":
        for index, (a, b) in enumerate(read_plan(args.plan), 1):
            take, todo = excerpt(texts, completed, unfinished, a, b)
            if todo == 0:
                continue
            path = os.path.join(args.outdir, f"job-{index:03d}.jsonl")
            with open(path, "w", encoding="utf-8") as out:
                out.writelines(texts[i] for i in take)
            with open(path[: -len(".jsonl")] + ".count", "w", encoding="utf-8") as out:
                out.write(f"{len(take)}\n")
            print(f"{index:03d}", a, b, len(take), todo)
        return 0
    take, todo = excerpt(texts, completed, unfinished, args.first, args.last)
    with open(args.out, "w", encoding="utf-8") as out:
        out.writelines(texts[i] for i in take)
    print(len(take), todo)
    return 0


if __name__ == "__main__":
    sys.exit(main())
