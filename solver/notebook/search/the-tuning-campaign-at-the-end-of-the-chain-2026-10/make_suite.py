#!/usr/bin/env python3
"""Builds the tuning suite (suite.jsonl) from checkpoints of searches at the end of the chain.  Standard library only.

The steps (README.md has the commands as run):

  1. `tune units GAME CKPT --sample 100000 --out units_GAME.jsonl` writes all the units of a checkpoint, each with the
     bound of its node against what it needs, the `gap` (examples/tune.rs).
  2. `make_suite.py probe` finds subtrees of the sizes the suite wants.  A unit's size cannot be told from its depth or
     its candidates, but a tree grows by about a factor of ten with every 175 of the gap between its root's bound and
     what the root needs, so the units whose gap is in a band (`--gap-lo` to `--gap-hi`) are the ones likely to have
     100,000 to 3,000,000 nodes (and a unit that does not finish costs the whole node limit, which is why not to try
     the others).  The band's units are tried in an order mixing its gaps, each with growing node limits (LADDER or
     `--ladder`, from `--start`) until it finishes, and the node counts and CPU seconds with the default settings are
     what the probe records (`--probes`, one `tune solve` result line each, appended to as they come; a probe that
     stops resumes from the file), until the game has `--per-game` units in range.  It probes the whole games named by
     `--wholes` the same way, with the limits of `--whole-limits`.
  3. `make_suite.py assemble` picks from the probes: the units that finished with LO to HI nodes, spread log-uniformly
     in size and evenly over the games, and the smallest whole games that took WHOLE_LO to WHOLE_HI nodes.
  4. `make_suite.py refresh` puts the default's CPU seconds of a later build, measured by tune.py, into the suite.
  5. `make_suite.py candidates` lists the searched games of the chain that may be small enough for `--wholes`.
"""

import argparse
import collections
import json
import math
import os
import random
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

# python3 -I leaves the script's directory off the path, and tune.py is beside this file.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import tune  # noqa: E402

#: The node limits a unit is tried with in turn, each about three times the last: a unit that finishes at one has cost
#: no more than half as much again as its own size in the failures before it.
LADDER = (20_000, 120_000, 400_000, 1_200_000, 3_000_000)
HERE = Path(__file__).resolve().parent


def read_jsonl(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


# ---------------------------------------------------------------------------------------------------------
# Units


Game = collections.namedtuple("Game", "header units path")


def load_units(paths):
    """{game: Game} (the game line, the units in checkpoint order, the file) from the files `tune units` wrote."""
    games = {}
    for path in paths:
        rows = read_jsonl(path)
        header = next(r for r in rows if r["kind"] == "game")
        games[header["game"]] = Game(header, [r for r in rows if r["kind"] == "unit"], path)
    return games


def candidate_order(units, gap_lo, gap_hi, seed, bands=4):
    """The units whose gap is from `gap_lo` to `gap_hi`, in the order to try them: shuffled by the seed within `bands`
    equal bands of the gap, and taken from the bands in turn, so that the first few have the whole range of sizes."""
    pool = [u for u in units if u.get("gap") is not None and gap_lo <= u["gap"] <= gap_hi]
    random.Random(seed).shuffle(pool)
    width = (gap_hi - gap_lo + 1) / bands
    groups = [[u for u in pool if int((u["gap"] - gap_lo) / width) == b] for b in range(bands)]
    order = []
    while any(groups):
        order += [group.pop() for group in groups if group]
    return order


# ---------------------------------------------------------------------------------------------------------
# Probing


class Probes:
    """The result lines of `tune solve` with the default settings for the units and games tried, in a jsonl file."""

    def __init__(self, path):
        self.path, self.lock, self.records = Path(path), threading.Lock(), collections.defaultdict(list)
        if self.path.exists():
            for record in read_jsonl(self.path):
                self.records[record["id"]].append(record)

    def solved(self, id):
        """The record of `id` that finished, if any."""
        return next((r for r in self.records[id] if r["completed"]), None)

    def failed_limit(self, id):
        """The largest node limit `id` did not finish within (0 if it was not tried)."""
        return max((r["node_limit"] for r in self.records[id] if not r["completed"]), default=0)

    def add(self, record):
        with self.lock:
            self.records[record["id"]].append(record)
            with open(self.path, "a") as out:
                out.write(json.dumps(record) + "\n")

    def add_runs(self, path):
        """Takes the lines of `wall images` in `path` that ran the default search (mode `count`) as solves too, in
        memory: they record the same steps, CPU seconds and best sum, from the example `wall` in place of `tune`."""
        for line in read_jsonl(path):
            if line.get("mode") == "count" and line["tactics"] == "default":
                self.records[line["id"]].append({
                    "id": line["id"], "game": line["n"], "nodes": line["nodes"], "cpu_s": line["cpu_s"],
                    "wall_s": line.get("wall_s", 0.0), "completed": line["completed"], "best": line["best"],
                    "node_limit": None})


class Prober:
    """Solves problems with the default settings and a node limit through `tune solve`, keeping the results."""

    def __init__(self, args, probes):
        self.args, self.probes = args, probes
        self.ladder = tuple(getattr(args, "ladder", LADDER))

    def wait_for_load(self):
        """Waits while the machine's load average over one minute is over `--max-load` (0: never)."""
        limit = getattr(self.args, "max_load", 0)
        while limit and os.getloadavg()[0] > limit:
            time.sleep(30)

    def solve(self, problem, limit, units):
        a = self.args
        self.wait_for_load()
        command = tune.solve_command(a.bin, a.known, units, problem, node_limit=limit, nice=a.nice)
        record = tune.solve(command)
        self.probes.add(record)
        done = "finished" if record["completed"] else "stopped"
        print(f"  {problem['id']}: {done} at {record['nodes']} nodes, {record['cpu_s']:.1f} s of CPU", flush=True)
        return record

    def unit(self, unit, units, start):
        """The record of the unit's finished solve, tried with the node limits of the ladder from `start` on (and above
        any it failed within already), or None if it did not finish within the last."""
        found = self.probes.solved(unit["id"])
        for limit in self.ladder:
            if found or limit < start or limit <= self.probes.failed_limit(unit["id"]):
                continue
            found = self.solve({"kind": "unit", "game": unit["game"], "id": unit["id"]}, limit, units)
            found = found if found["completed"] else None
        return found

    def game(self, units, order, path, target, lo, hi, start, max_failures=3):
        """Probes the units of `order` until `target` units of the game have finished with `lo` to `hi` nodes, or
        `max_failures` have not finished within the last limit of the ladder (each costs it); returns how many have."""
        def in_range(unit):
            found = self.probes.solved(unit["id"])
            return found is not None and lo <= found["nodes"] <= hi

        have, failures = sum(in_range(u) for u in units), 0
        for unit in order:
            if have >= target or failures >= max_failures:
                break
            if not self.probes.solved(unit["id"]) and self.probes.failed_limit(unit["id"]) < self.ladder[-1]:
                found = self.unit(unit, path, start)
                have += found is not None and in_range(unit)
                failures += found is None
        return have

    def whole(self, game, limits=None):
        """Probes the whole game with the node limits in turn until one finishes it (by default the last limit of the
        ladder alone: it is one problem), skipping the limits it failed within already."""
        problem = {"kind": "whole", "game": game, "id": f"{game}-root"}
        for limit in limits or (self.ladder[-1],):
            if not self.probes.solved(problem["id"]) and limit > self.probes.failed_limit(problem["id"]):
                self.solve(problem, limit, None)


def probe_command(args):
    games = load_units(args.units)
    prober = Prober(args, Probes(args.probes))
    jobs = [(f"game {game}, whole", lambda game=game: prober.whole(game, args.whole_limits)) for game in args.wholes]
    for game, found in sorted(games.items()):
        order = candidate_order(found.units, args.gap_lo, args.gap_hi, args.seed)
        if args.ids:
            by_id = {u["id"]: u for u in found.units}
            order = [by_id[id] for id in args.ids if id in by_id]
        jobs.append((f"game {game}: {len(order)} units in the band of gaps",
                     lambda found=found, order=order: prober.game(found.units, order, found.path, args.per_game,
                                                                  args.lo, args.hi, args.start, args.max_failures)))
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        futures = [(label, pool.submit(job)) for label, job in jobs]
        for label, future in futures:
            result = future.result()
            print(f"done: {label}" + (f", {result} in range" if result is not None else ""), flush=True)


# ---------------------------------------------------------------------------------------------------------
# Assembling the suite


def select(candidates, count, lo, hi, bins=8):
    """Picks `count` of the candidates, (game, id, nodes) triples, spread log-uniformly in `nodes` between `lo` and `hi`
    (the same number from each of `bins` equal ranges of log size, where the candidates allow) and evenly over the games
    (the game with the fewest picks gives the next, the candidate farthest in size from the bin's picks so far)."""
    span = math.log(hi / lo)
    ranges = [[] for _ in range(bins)]
    for game, id, nodes in candidates:
        if lo <= nodes <= hi:
            ranges[min(bins - 1, int(bins * math.log(nodes / lo) / span))].append((game, id, nodes))
    picked, per_game = [[] for _ in range(bins)], collections.Counter()
    while sum(map(len, picked)) < count and any(ranges):
        # The range with the fewest picks (the earlier on a tie) that has a candidate left.
        b = min((i for i in range(bins) if ranges[i]), key=lambda i: (len(picked[i]), i))
        game = min({c[0] for c in ranges[b]}, key=lambda g: (per_game[g], g))
        center = lo * math.exp(span * (b + 0.5) / bins)

        def apart(candidate):
            logs = [abs(math.log(candidate[2] / other[2])) for other in picked[b]]
            return (min(logs, default=math.inf), -abs(math.log(candidate[2] / center)))

        best = max((c for c in ranges[b] if c[0] == game), key=apart)
        ranges[b].remove(best)
        picked[b].append(best)
        per_game[game] += 1
    return sorted((c for p in picked for c in p), key=lambda c: (c[0], c[2]))


def assemble_command(args):
    games = load_units(args.units)
    probes = Probes(args.probes)
    for runs in args.runs:
        probes.add_runs(runs)
    sizes = []
    for game, found in games.items():
        sizes += [(game, u["id"], probes.solved(u["id"])["nodes"]) for u in found.units if probes.solved(u["id"])]
    chosen = select(sizes, args.count, args.lo, args.hi)
    wholes = sorted(
        (probes.solved(f"{n}-root") for n in args.wholes if probes.solved(f"{n}-root")), key=lambda r: r["nodes"]
    )
    wholes = [r for r in wholes if args.whole_lo <= r["nodes"] <= args.whole_hi][: args.whole_count]

    def default(record):
        return {k: record[k] for k in ("nodes", "cpu_s", "wall_s", "best", "completed")}

    commit = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=HERE, check=False)
    lines = [{"kind": "suite", "commit": commit.stdout.strip(), "lo": args.lo, "hi": args.hi, "units": len(chosen),
              "wholes": len(wholes)}]
    for game in sorted({g for g, _, _ in chosen}):
        lines.append(games[game].header)
        by_id = {u["id"]: u for u in games[game].units}
        for _, id, _ in (c for c in chosen if c[0] == game):
            lines.append({**by_id[id], "default": default(probes.solved(id))})
    for record in wholes:
        gap = {"gap": args.whole_gaps[record["game"]]} if record["game"] in args.whole_gaps else {}
        lines.append({"kind": "whole", "game": record["game"], "id": record["id"], **gap,
                      "default": default(record)})
    Path(args.out).write_text("".join(json.dumps(line) + "\n" for line in lines))
    per_game = collections.Counter(g for g, _, _ in chosen)
    print(f"{len(chosen)} units ({dict(sorted(per_game.items()))}) and {len(wholes)} whole games in {args.out}")
    print("unit sizes, nodes:", [n for _, _, n in chosen])
    print("whole games:", [(r["game"], r["nodes"]) for r in wholes])


def refresh_command(args):
    """Rewrites the CPU and wall seconds of each problem's `default` in the suite from the default's results in a
    results file of tune.py (which ran the whole suite with the default alone, `--concurrency` jobs at a time): after a
    change that makes the search faster without changing its trees, which the nodes and best sums must show.  With
    `--new-trees`, after a change of the default that changes the trees on purpose (a tactic's default): the nodes are
    rewritten too, and the best sums must still be the suite's, since a sound setting changes the tree, not the answer.
    """
    lines = read_jsonl(args.suite)
    done = tune.read_results(args.results)
    total = 0.0
    for line in lines:
        if line.get("kind") not in tune.PROBLEM_KINDS:
            continue
        found = done.get((tune.DEFAULT, line["id"]))
        if found is None:
            raise SystemExit(f"{args.results}: no result of the default on {line['id']}")
        result, old = found["result"], line["default"]
        same_trees = result["nodes"] == old["nodes"]
        if not result["completed"] or result["best"] != old["best"] or not (same_trees or args.new_trees):
            raise SystemExit(f"{line['id']}: the default now takes {result['nodes']} nodes and finds {result['best']}, "
                             f"not the suite's {old['nodes']} and {old['best']}: other trees, so a new suite "
                             "(or --new-trees, if the best sum is the suite's and the trees changed on purpose)")
        line["default"] = {**old, "nodes": result["nodes"], "cpu_s": result["cpu_s"], "wall_s": result["wall_s"]}
        total += result["cpu_s"]
    lines[0].update(default_cpu_s=round(total, 1), concurrency=args.concurrency)
    lines[0]["commit"] = args.commit or lines[0]["commit"]
    Path(args.suite).write_text("".join(json.dumps(line) + "\n" for line in lines))
    print(f"{args.suite}: the default's CPU seconds from {args.results}, {total:.0f} s in all")


def candidates_command(args):
    """The searched games of FROM-TO (3000-3440 by default) whose search in the chain took the fewest nodes, from LO
    up."""
    rows = []
    for r in read_jsonl(args.strict):
        if args.first <= r["n"] <= args.last and r["method"] == "search" and not r["reached_cap"] \
                and r["nodes"] >= args.lo:
            rows.append((r["nodes"], r["n"]))
    print(",".join(str(n) for _, n in sorted(rows)[: args.count]))


# ---------------------------------------------------------------------------------------------------------


def limits(text):
    """The increasing node limits of `LIMIT,LIMIT,...`."""
    found = [int(x) for x in text.split(",") if x]
    if not found or found != sorted(set(found)):
        raise argparse.ArgumentTypeError(f"{text!r}: node limits must be increasing")
    return found


def whole_gaps(text):
    """{game: gap} from `GAME=GAP,...`."""
    return {int(game): int(gap) for game, gap in (item.split("=") for item in text.split(",") if item)}


def parse(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = p.add_subparsers(dest="command", required=True)
    for name in ("probe", "assemble"):
        s = sub.add_parser(name)
        s.add_argument("--units", nargs="+", required=True, help="the units files of `tune units`, one a game")
        s.add_argument("--probes", required=True, help="the probes file (appended to)")
        s.add_argument("--wholes", type=lambda t: [int(x) for x in t.split(",") if x], default=[],
                       help="the whole games to probe (probe) or to choose from (assemble), comma-separated")
        s.add_argument("--lo", type=int, default=100_000, help="fewest nodes of a unit (default 100,000)")
        s.add_argument("--hi", type=int, default=3_000_000, help="most nodes of a unit (default 3,000,000)")
    probe = sub.choices["probe"]
    probe.add_argument("--gap-lo", type=int, default=740, help="smallest gap of a unit to try (default 740)")
    probe.add_argument("--gap-hi", type=int, default=1000, help="largest gap of a unit to try (default 1000)")
    probe.add_argument("--ids", type=lambda t: [x for x in t.split(",") if x], default=[],
                       help="try these units, in this order, instead of the band's (the units of a stack, say, "
                            "from the deepest up, whose sizes grow)")
    probe.add_argument("--per-game", type=int, default=12, help="units in range wanted from each game (default 12)")
    probe.add_argument("--max-failures", type=int, default=3, help="units not finishing, per game, to give up at (3)")
    probe.add_argument("--start", type=int, default=120_000, help="first node limit tried on a unit (default 120,000)")
    probe.add_argument("--seed", type=int, default=1)
    probe.add_argument("--ladder", type=limits, default=list(LADDER),
                       help="the node limits a unit is tried with in turn, comma-separated (default the ladder)")
    probe.add_argument("--whole-limits", type=limits, default=None,
                       help="the node limits to try a whole game with, in turn, comma-separated (default: the last "
                            "limit of the ladder, 3,000,000)")
    probe.add_argument("--workers", type=int, default=2)
    probe.add_argument("--nice", type=int, default=0)
    probe.add_argument("--max-load", type=float, default=0,
                       help="start a solve only when the one-minute load average is at most this (default 0: any)")
    probe.add_argument("--bin", default=tune.CRATE / "target" / "release" / "examples" / "tune", type=Path)
    probe.add_argument("--known", default=tune.REPO / "results" / "strict.jsonl", type=Path)
    assemble = sub.choices["assemble"]
    assemble.add_argument("--out", required=True)
    assemble.add_argument("--count", type=int, default=40, help="units to keep (default 40)")
    assemble.add_argument("--whole-count", type=int, default=6, help="whole games to keep (default 6)")
    assemble.add_argument("--whole-lo", type=int, default=500_000, help="fewest nodes of a whole game (default 500k)")
    assemble.add_argument("--whole-hi", type=int, default=3_000_000, help="most nodes of a whole game (default 3M)")
    assemble.add_argument("--runs", action="append", default=[],
                          help="a file of `wall images` lines whose default searches (mode count) count as probes too")
    assemble.add_argument("--whole-gaps", type=whole_gaps, default={},
                          help="GAME=GAP,... the root gaps of whole games, recorded in their lines")
    refresh = sub.add_parser("refresh")
    refresh.add_argument("--suite", required=True, help="the suite file, rewritten")
    refresh.add_argument("--results", required=True, help="a results file of tune.py with the default on every problem")
    refresh.add_argument("--concurrency", type=int, required=True, help="jobs that ran at once, to record")
    refresh.add_argument("--commit", help="the commit of the build that measured, to record (default: the suite's)")
    refresh.add_argument("--new-trees", action="store_true",
                         help="accept other nodes than the suite's (the default's trees changed on purpose), if the "
                              "best sums are the suite's, and record them")
    cand = sub.add_parser("candidates")
    cand.add_argument("--strict", default=tune.REPO / "results" / "strict.jsonl", type=Path)
    cand.add_argument("--from", dest="first", type=int, default=3000, help="first game to list (default 3000)")
    cand.add_argument("--to", dest="last", type=int, default=3440, help="last game to list (default 3440)")
    cand.add_argument("--lo", type=int, default=300_000, help="fewest nodes in the chain (default 300,000)")
    cand.add_argument("--count", type=int, default=14, help="games to list (default 14)")
    return p.parse_args(argv)


def main(argv=None):
    args = parse(argv)
    {"probe": probe_command, "assemble": assemble_command, "refresh": refresh_command,
     "candidates": candidates_command}[args.command](args)


if __name__ == "__main__":
    main()
