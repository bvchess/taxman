#!/usr/bin/env python3
"""The tables of a sweep of nogood policies, from the lines run.sh kept.

usage: summarize.py RESULTS [--games=G,G,...] [--policies=P,P,...]     (from rust/bench/nogoods; markdown on stdout)

RESULTS has one line per run, `GAME<TAB>POLICY<TAB>the ladder's line`.  The policies are in the order of their first
appearance, or as --policies lists them (and the games as --games does); `default` is the one the others are compared
with.  Totals are over the games that every policy listed finished (a run that reached the ladder's time limit says
OPEN, and its game is left out of every total).  The tables: nodes by
game and policy; each policy's nodes against the default's by game, with the geometric mean and the range; the totals
of nodes, CPU seconds and gigacycles against the default, with the CPU time of a node; and the counters of the nogood
tables, summed over the games (the largest table: the largest of any).
"""
import math
import re
import sys
from collections import OrderedDict

RUN = re.compile(r"^n=(?P<n>\d+) T=1 (?P<status>\S+)\s+search\s+(?P<wall>[\d.]+) s nodes\s+(?P<nodes>\d+) ")
CPU = re.compile(r"cpu (?P<cpu>[\d.]+) s (?P<cycles>[\d.?]+) Gcycles")
NOGOODS = re.compile(
    r"nogoods learned (?P<learned>\d+) lookups (?P<lookups>\d+) hit (?P<hit>[\d.]+)% ruled out (?P<ruled>\d+) "
    r"forgot (?P<forgot>\d+) \(table (?P<table>\d+) kept (?P<kept>\d+)\) evicted (?P<evicted>\d+) "
    r"largest (?P<largest>\d+)"
)
BASE = "default"


def parse(path):
    """{policy: {game: run}} in order of appearance, a run being a dict of its figures."""
    runs = OrderedDict()
    with open(path) as f:
        for line in f:
            game, policy, text = line.rstrip("\n").split("\t", 2)
            head, cpu, counters = RUN.match(text), CPU.search(text), NOGOODS.search(text)
            if not (head and cpu and counters):
                raise SystemExit(f"cannot read the line of {game} under {policy}: {text}")
            run = {
                "done": head["status"] in ("DONE", "CAPPED"),
                "nodes": int(head["nodes"]),
                "wall": float(head["wall"]),
                "cpu": float(cpu["cpu"]),
                "gcycles": float(cpu["cycles"]) if cpu["cycles"] != "?" else float("nan"),
            }
            run.update({key: float(value) for key, value in counters.groupdict().items()})
            runs.setdefault(policy, OrderedDict())[int(game)] = run
    return runs


def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out += ["| " + " | ".join(str(cell) for cell in row) + " |" for row in rows]
    return "\n".join(out)


def percent(ratio):
    return f"{100 * (ratio - 1):+.1f}%"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    options = dict(a[2:].split("=", 1) for a in sys.argv[1:] if a.startswith("--") and "=" in a)
    if len(args) != 1 or set(options) - {"games", "policies"}:
        raise SystemExit(__doc__)
    runs = parse(args[0])
    if "policies" in options:
        runs = OrderedDict((p, runs[p]) for p in options["policies"].split(",") if p in runs)
    if BASE not in runs:
        raise SystemExit("no run of the default policy")
    games = [int(g) for g in options["games"].split(",")] if "games" in options else list(runs[BASE])
    complete = [g for g in games if all(g in r and r[g]["done"] for r in runs.values())]
    left_out = [g for g in games if g not in complete]
    print(f"Games in the totals: {', '.join(map(str, complete))}" + (f"; left out: {left_out}" if left_out else ""))
    print()

    def total(policy, key):
        return sum(runs[policy][g][key] for g in complete)

    def nodes_of(policy, game):
        run = runs[policy].get(game)
        return None if run is None else run["nodes"] / runs[BASE][game]["nodes"]

    rows = []
    for policy, by_game in runs.items():
        cells = [f"`{policy}`"]
        for g in games:
            run = by_game.get(g)
            cells.append("-" if run is None else f"{run['nodes']:,}" + ("" if run["done"] else "*"))
        rows.append(cells)
    print(table(["policy"] + [str(g) for g in games], rows))
    print()

    rows = []
    for policy in runs:
        ratios = [nodes_of(policy, g) for g in complete]
        logs = [math.log(r) for r in ratios]
        mean = sum(logs) / len(logs)
        # The standard error of the mean of the log ratios (the games are the sample), as a percentage.
        error = 100 * math.sqrt(sum((x - mean) ** 2 for x in logs) / max(len(logs) - 1, 1) / len(logs))
        rows.append([f"`{policy}`"] + [percent(r) for r in ratios]
                    + [percent(math.exp(mean)), f"{error:.1f}", f"{percent(min(ratios))} to {percent(max(ratios))}"])
    print(table(["policy"] + [str(g) for g in complete] + ["geometric mean", "± (s.e.)", "range"], rows))
    print()

    rows = []
    for policy in runs:
        nodes, cpu, cycles = total(policy, "nodes"), total(policy, "cpu"), total(policy, "gcycles")
        rows.append([
            f"`{policy}`",
            f"{nodes:,}",
            percent(nodes / total(BASE, "nodes")),
            f"{cpu:.0f}",
            percent(cpu / total(BASE, "cpu")),
            f"{cycles:.0f}",
            percent(cycles / total(BASE, "gcycles")),
            f"{1e6 * cpu / nodes:.0f}",
        ])
    print(table(["policy", "nodes", "vs default", "CPU s", "vs default", "Gcycles", "vs default", "CPU µs a node"], rows))
    print()

    rows = []
    for policy in runs:
        by = {key: total(policy, key) for key in ("learned", "lookups", "ruled", "forgot", "evicted")}
        hits = sum(runs[policy][g]["lookups"] * runs[policy][g]["hit"] / 100 for g in complete)

        def mean(key):
            return sum(runs[policy][g][key] * runs[policy][g]["forgot"] for g in complete) / max(by["forgot"], 1)

        rows.append([
            f"`{policy}`",
            f"{by['learned']:,.0f}",
            f"{by['lookups']:,.0f}",
            f"{100 * hits / max(by['lookups'], 1):.1f}%",
            f"{by['ruled']:,.0f}",
            f"{by['forgot']:,.0f}",
            f"{mean('table'):.0f}",
            f"{mean('kept'):.0f}",
            f"{by['evicted']:,.0f}",
            f"{max(runs[policy][g]['largest'] for g in complete):.0f}",
        ])
    print(table(["policy", "learned", "lookups", "answered", "ruled out", "forgettings", "table at one", "kept",
                 "evicted", "largest"], rows))


if __name__ == "__main__":
    main()
