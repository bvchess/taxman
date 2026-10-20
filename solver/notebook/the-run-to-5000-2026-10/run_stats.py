#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The run to 5000: run_stats.py

Prints the numbers of the entry's README.md.  The run's lines of games 3915..4547 are the committed chain's, byte for
byte, so they are read from results/strict.jsonl; the run's clock is read from logs/ and run/.  It prints:

* what the 633 lines are (the method and proof of each, the build, the games taken from the bounds store);
* the games, searches, nodes and search hours of each stretch of about a hundred games, and the median of its
  exhausted searches (the search seconds of a game are the hours it was in flight in the window, shared with the
  games beside it, so the sum over the games is more than the time);
* the dearest searches, by nodes and by search seconds;
* the machines, the hours of each, the instance-hours and the cost;
* the boards the bounds store has proven past the run's end;
* the game stopped unfinished.

Standard library only; run it from anywhere.
"""
import gzip
import json
import re
import statistics
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIRST, LAST = 3915, 4547
STRETCHES = [(3915, 4000), (4001, 4100), (4101, 4200), (4201, 4300), (4301, 4400), (4401, 4500), (4501, 4547)]
BIG_GAME_NODES = 30_000_000
WAITING_NODES = 100_000


def read_keyed(path):
    """The lines of `path` as key=value pairs."""
    pairs = {}
    for line in path.read_text().splitlines():
        key, _, value = line.partition("=")
        pairs[key] = value
    return pairs


def utc(text):
    return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)


def seconds_of(game):
    return game["seconds"]["search_all_runs"]


def hours(seconds):
    return seconds / 3600


def read_games():
    games = {}
    with open(HERE / "../../../results/strict.jsonl") as f:
        for line in f:
            game = json.loads(line)
            if FIRST <= game["n"] <= LAST:
                games[game["n"]] = game
    assert sorted(games) == list(range(FIRST, LAST + 1)), "the committed chain lacks a game of the run"
    return games


def describe_lines(games):
    lines = list(games.values())
    builds = {(g["build"]["commit"][:7], g["build"]["dirty"], g["threads"], g["tactics"]) for g in lines}
    print(f"games {FIRST}..{LAST}: {len(lines)} lines; build, dirty, threads, tactics: {sorted(builds)}")
    print("options:", {json.dumps(g["options"], sort_keys=True) for g in lines})
    by_kind = {}
    for g in lines:
        by_kind.setdefault((g["method"], g["proof"]), []).append(g)
    for (method, proof), group in sorted(by_kind.items(), key=lambda item: -len(item[1])):
        print(f"  {method:<7} {proof:<12} {len(group):>4}")
    searches = [g for g in lines if g["method"] == "search"]
    print(f"  searches {len(searches)}: {sum(g['proof'] == 'exhausted' for g in searches)} exhausted, "
          f"{sum(g['proof'] == 'cap' for g in searches)} at the ceiling")
    store = [g for g in lines if g["method"] == "known"]
    assumed = {a for g in store for a in g["assumes"]}
    print(f"  from the bounds store: {len(store)} games, which assume statement U at {len(assumed)} boards in all; "
          f"games with any assumption: {sum(bool(g['assumes']) for g in lines)}; "
          f"sources: {sorted({g['known']['source'] for g in store})}")
    resumed = [g["n"] for g in lines if g["invocations"] > 1]
    print(f"  games searched in more than one invocation: {resumed}")


def stretch_table(games):
    print("\nby stretch (search hours are summed over the games, so they add up to more than the time):")
    print(f"{'stretch':>11} {'games':>5} {'store':>5} {'searches':>8} {'G nodes':>8} {'hours':>6} "
          f"{'median exhausted s':>19}")
    for first, last in STRETCHES:
        group = [games[n] for n in range(first, last + 1)]
        searches = [g for g in group if g["method"] == "search"]
        exhausted = [seconds_of(g) for g in searches if g["proof"] == "exhausted"]
        median = f"{statistics.median(exhausted):,.0f}" if exhausted else "-"
        print(f"{first:>5}-{last:<5} {len(group):>5} {sum(g['method'] == 'known' for g in group):>5} "
              f"{len(searches):>8} {sum(g['nodes'] for g in group) / 1e9:>8.2f} "
              f"{hours(sum(seconds_of(g) for g in searches)):>6.1f} {median:>19}")
    searches = [g for g in games.values() if g["method"] == "search"]
    big = [g for g in searches if g["nodes"] >= BIG_GAME_NODES]
    total = sum(g["nodes"] for g in searches)
    print(f"{'all':>11} {len(games):>5} {sum(g['method'] == 'known' for g in games.values()):>5} {len(searches):>8} "
          f"{total / 1e9:>8.2f} {hours(sum(seconds_of(g) for g in searches)):>6.1f}")
    print(f"{len(big)} games of {BIG_GAME_NODES // 1_000_000} million nodes or more took "
          f"{sum(g['nodes'] for g in big) / 1e9:.2f} G of the {total / 1e9:.2f} G nodes")
    return total


def dearest(games):
    searches = list(games.values())
    print("\nthe dearest searches by nodes:")
    for g in sorted(searches, key=lambda g: g["nodes"], reverse=True)[:8]:
        print(f"  {g['n']}: {g['nodes'] / 1e9:.3f} G nodes, {seconds_of(g):,.0f} s ({hours(seconds_of(g)):.2f} h), "
              f"{g['proof']}, invocations {g['invocations']}")
    print("the dearest searches by search seconds:")
    for g in sorted(searches, key=seconds_of, reverse=True)[:8]:
        print(f"  {g['n']}: {seconds_of(g):,.0f} s ({hours(seconds_of(g)):.2f} h), {g['nodes'] / 1e9:.3f} G nodes, "
              f"{g['proof']}")
    over_hour = sum(seconds_of(g) > 3600 for g in searches)
    over_half = sum(seconds_of(g) > 1800 for g in searches)
    print(f"searches over an hour: {over_hour}; over half an hour: {over_half}")
    ceiling = [g for g in searches if g["proof"] == "cap"]
    slow = [(g["n"], round(seconds_of(g)), g["nodes"]) for g in ceiling if seconds_of(g) > 600]
    print(f"proofs at the ceiling: {len(ceiling)}, the most nodes {max(g['nodes'] for g in ceiling):,}; "
          f"in flight over 600 s (n, seconds, nodes): {slow}")
    ticks = progress_ticks()
    print("progress lines (a minute apart) of the longest in flight and the slow proofs at the ceiling, "
          f"those with under {WAITING_NODES:,} nodes (the search waits for the game before it):")
    for n in [g["n"] for g in sorted(searches, key=seconds_of, reverse=True)[:5]] + [s[0] for s in slow]:
        waiting = sum(nodes < WAITING_NODES for nodes in ticks[n])
        print(f"  {n}: {waiting} of {len(ticks[n])}")


def progress_ticks():
    """The node counts of each game's progress lines in logs/chain.err.gz, by n, in order."""
    ticks = {}
    with gzip.open(HERE / "logs/chain.err.gz", "rt") as f:
        for line in f:
            found = re.match(r"^n=(\d+) \S+ nodes (\d+) \((?!stopped|done)", line)
            if found:
                ticks.setdefault(int(found.group(1)), []).append(int(found.group(2)))
    return ticks


def machines():
    """The machines of wall-node.log and the solver's invocations of chain.log."""
    node = (HERE / "logs/wall-node.log").read_text().splitlines()
    stamp = re.compile(r"^(\S+Z) (.*)$")
    starts, chains, ends = [], [], []
    for line in node:
        when, text = stamp.match(line).groups()
        if text.startswith("wall-node on"):
            starts.append((utc(when), text.split()[2].rstrip(":")))
        elif text.startswith("chain.sh 8191"):
            chains.append(utc(when))
        elif text.startswith("chain.sh exited"):
            ends.append((utc(when), text))
    chain_log = (HERE / "logs/chain.log").read_text()
    done = [int(x) for x in re.findall(r"games done so far: (\d+)", chain_log)]
    solver_starts = [utc(x) for x in re.findall(r"^(\S+Z) starting the solver", chain_log, re.M)]
    exits = re.findall(r"^(\S+Z) the solver exited with status \d+ after (\d+)s", chain_log, re.M)
    solver_ends = [utc(when) for when, _ in exits]
    solver_seconds = [int(seconds) for _, seconds in exits]
    print("\nthe machines (wall-node.log, chain.log):")
    for i, ((start, host), chain, (end, text)) in enumerate(zip(starts, chains, ends)):
        setup_minutes = (chain - start).total_seconds() / 60
        done_at_end = done[i + 1] if i + 1 < len(done) else LAST
        print(f"  {i + 1}. {host}: node script {start:%m-%d %H:%M:%S}Z, setup {setup_minutes:.1f} min, "
              f"solver {solver_starts[i]:%m-%d %H:%M:%S}Z to {solver_ends[i]:%m-%d %H:%M:%S}Z, "
              f"games done at its end {done_at_end}, solver seconds {solver_seconds[i]:,} "
              f"({hours(solver_seconds[i]):.2f} h); {text.split(': ', 1)[1]}")
    for i in range(1, len(starts)):
        gap = (starts[i][0] - ends[i - 1][0]).total_seconds() / 60
        print(f"  gap between {i} and {i + 1}: {gap:.1f} minutes")
    print(f"  solver seconds in all: {sum(solver_seconds):,} ({hours(sum(solver_seconds)):.2f} h) in "
          f"{len(solver_seconds)} invocations")
    return sum(solver_seconds)


def cost():
    config = read_keyed(HERE / "run/config")
    ledger = read_keyed(HERE / "run/ledger")
    ended = read_keyed(HERE / "run/ended")
    price = float(config["PRICE"])
    spent = float(ledger["hours"])
    print(f"\nthe run (run/config, run/ledger, run/ended): commit {config['COMMIT'][:7]}, {config['INSTANCE_TYPE']}, "
          f"window {config['WINDOW']}, extra arguments '{config['EXTRA_ARGS']}', "
          f"release tests {config['RELEASE_TESTS']}")
    print(f"  budget {config['BUDGET_HOURS']} instance-hours; ledger start {utc_from_epoch(ledger['start'])}, "
          f"ended {utc_from_epoch(ended['time'])} ({ended['reason']})")
    print(f"  instance-hours {spent:.4f} at ${price}: ${spent * price:.2f}; the approved 36 hours "
          f"${36 * price:.2f}, the 40.5 of the end ${40.5 * price:.2f}")
    return spent


def store_ahead():
    """The boards the bounds store has proven (best lower bound equal to best upper bound), past the run's end."""
    lower, upper = {}, {}
    with open(HERE / "../../../results/bounds.jsonl") as f:
        for line in f:
            bound = json.loads(line)
            n, value = bound["n"], bound["value"]
            if bound["side"] == "lower":
                lower[n] = max(lower.get(n, value), value)
            else:
                upper[n] = min(upper.get(n, value), value)
    proven = sorted(n for n in lower if n in upper and lower[n] == upper[n])
    used = [n for n in proven if FIRST <= n <= LAST]
    ahead = [n for n in proven if LAST < n <= 5000]
    print(f"\nthe bounds store (results/bounds.jsonl as it is when this runs): {len(proven)} proven boards, "
          f"the last {proven[-1]}; {len(used)} in {FIRST}..{LAST}; {len(ahead)} in {LAST + 1}..5000 "
          f"(first {ahead[0]}), so {5000 - LAST - len(ahead)} of the {5000 - LAST} games to 5000 are not in it")


def utc_from_epoch(text):
    return datetime.fromtimestamp(int(text), timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def stopped_game():
    with gzip.open(HERE / "logs/chain.out.gz", "rt") as f:
        lines = f.read().splitlines()
    incomplete = [line for line in lines if "incomplete" in line]
    print("\nthe game stopped unfinished (chain.out):")
    for line in incomplete:
        print(" ", line)
    with gzip.open(HERE / "logs/chain.err.gz", "rt") as f:
        stops = [line for line in f.read().splitlines() if "(stopped)" in line]
    for line in stops:
        print("  chain.err:", line.split(" | ")[0])


def main():
    games = read_games()
    describe_lines(games)
    total_nodes = stretch_table(games)
    dearest(games)
    solver_seconds = machines()
    cost()
    store_ahead()
    stopped_game()
    print(f"\nnodes of the lines per solver second: {total_nodes / solver_seconds / 1e3:.0f} thousand, so a billion "
          f"nodes cost {1e9 * solver_seconds / total_nodes / 3600:.2f} solver hours (the stopped game's nodes and the "
          f"work redone after a resume are not in it)")


main()
