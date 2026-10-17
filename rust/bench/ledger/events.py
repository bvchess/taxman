#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""What a ledger run spent its time on, beyond `ledger report`: events.py DIR [CHAIN.jsonl].

From DIR/events.jsonl and DIR/ledger.jsonl (or their gzipped copies): the CPU of the setup (the boards' facts), of the
maps and of the searches; the searches that completed, reached their ceiling or were stopped, and what a stopped
search costs beyond its limit
(its setup, the start incumbent and the dual bound); how often a stopped search's dual bound lowered U, by how much,
and how far below the board's root bound it was; the changes the relations made, by side and rule; and the boards'
numbers of searches.  With CHAIN.jsonl, the results file of `taxman solve` over the same stretch, the chain's seconds
beside it.  Standard library only.
"""
import gzip
import json
import os
import sys
from collections import Counter


def lines(path):
    """The JSON lines of `path`, or of `path`.gz if only that is there."""
    opener = open if os.path.exists(path) else gzip.open
    with opener(path if opener is open else path + ".gz", "rt") as f:
        return [json.loads(line) for line in f if line.strip()]


def median(values):
    values = sorted(values)
    return values[len(values) // 2] if values else None


def main(directory, chain=None):
    events = lines(f"{directory}/events.jsonl")
    ledger = lines(f"{directory}/ledger.jsonl")
    start = next(e for e in events if e["action"] == "start")
    maps = [e for e in events if e["action"] == "init"]
    searches = [e for e in events if e["action"] == "search"]
    end = [e for e in events if e["action"] == "end"][-1]
    print(f"boards {len(ledger)}, proven {end['proven']}, CPU {end['cpu']:.0f} s, wall {end['t']:.0f} s "
          f"({end['reason']})")
    print(f"setup (the boards' facts and problems): {start['cpu']:.1f} CPU s")
    print(f"maps: {len(maps)}, {sum(e['seconds'] for e in maps):.1f} s")
    stopped = [e for e in searches if not e["completed"]]
    capped = [e for e in searches if e["reached_cap"]]
    print(f"searches: {len(searches)} of {len({e['n'] for e in searches})} boards, "
          f"{sum(e['seconds'] for e in searches):.1f} s; completed {len(searches) - len(stopped)} "
          f"({len(capped)} at their ceiling), stopped {len(stopped)}")
    over = [e["seconds"] - e["limit_s"] for e in stopped]
    if over:
        print(f"a stopped search's seconds beyond its limit: median {median(over):.2f}, max {max(over):.2f}")
    root = {e["n"]: e["after"][1] for e in maps}
    lowered = [e["before"][1] - e["after"][1] for e in stopped if e["after"][1] < e["before"][1]]
    print(f"stopped searches whose dual bound lowered U: {len(lowered)}, by {sum(lowered)} in all "
          f"(median {median(lowered)})")
    below = [root[e["n"]] - e["dual"] for e in stopped if e["n"] in root]
    if below:
        print(f"a stopped search's dual bound below its board's root bound: median {median(below)}, max {max(below)}")
    derived = Counter((e["side"], e["rule"]) for e in events if e["action"] == "derive")
    print("changes the relations made:", ", ".join(f"{s} {r} {c}" for (s, r), c in sorted(derived.items())))
    settle = [e for e in events if e["action"] == "settle"]
    if settle:
        print(f"the maps' bounds carried at {settle[0]['t']:.1f} s: {settle[0]['changes']} changes in "
              f"{settle[0]['seconds']:.2f} s")
    if "main_s" in end:
        print(f"the main thread: busy {end['main_s']:.1f} s of {end['t']:.0f} (its longest piece "
              f"{end['main_longest_s']:.2f} s), results waited {end['waited_s']:.1f} s in all")
    workers = start["workers"]
    whole = (end["cpu"] - start["cpu"]) / (end["t"] - start["t"])
    print(f"the run's CPU over its wall time: {whole:.2f} of {workers} workers ({whole / workers:.0%})")
    after_maps = [e for e in events if e["action"] != "derive" and maps and e["t"] >= maps[-1]["t"]]
    if len(after_maps) > 1 and after_maps[-1]["t"] > after_maps[0]["t"]:
        used = (after_maps[-1]["cpu"] - after_maps[0]["cpu"]) / (after_maps[-1]["t"] - after_maps[0]["t"])
        print(f"from the last map's result to the end: {used:.2f} of {workers} ({used / workers:.0%})")
    print("boards by their number of searches:", dict(sorted(Counter(e["attempts"] for e in ledger).items())))
    if chain:
        games = lines(chain)
        total = sum(g["seconds"]["total"] for g in games)
        search = sum(g["seconds"]["search"] for g in games)
        methods = Counter(g["method"] for g in games)
        print(f"the chain: {len(games)} games, {total:.1f} s ({search:.1f} s of search), {dict(methods)}")


if __name__ == "__main__":
    main(*sys.argv[1:])
