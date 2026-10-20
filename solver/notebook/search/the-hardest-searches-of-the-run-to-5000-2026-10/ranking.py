#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Ranks the searches of a stretch of the committed chain by their nodes, and lists the smallest.

Standard library only.

usage: python3 -I ranking.py top [--from 3915] [--to 4547] [--count 20] [--strict FILE]
       python3 -I ranking.py smallest [--from 3915] [--to 4547] [--lo 1000000] [--count 12] [--strict FILE]

`top` prints the COUNT games of FROM..TO whose search took the most nodes, each with its nodes, its wall hours (the
`search_all_runs` seconds of the line, on the threads of its invocations) and its invocations, and then the share of
the stretch's nodes carried by the dearest 5, 10, 20 and 40 games.  `smallest` prints the searched games of the stretch
with at least LO nodes, from the fewest up: the candidates for whole games to solve on one thread.  A searched game is a
line with the method `search` that did not stop at the ceiling (`reached_cap`).
"""
import argparse
import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
STRICT = REPO / "results" / "strict.jsonl"
SHARES = (5, 10, 20, 40)


def searches(strict, first, last):
    """The searched games of first..last, as dicts of n, nodes, hours, threads and invocations."""
    found = []
    for line in Path(strict).read_text().splitlines():
        row = json.loads(line)
        if first <= row["n"] <= last and row["method"] == "search" and not row["reached_cap"]:
            found.append({"n": row["n"], "nodes": row["nodes"], "hours": row["seconds"]["search_all_runs"] / 3600,
                          "threads": row["threads"], "invocations": row["invocations"]})
    return found


def stretch_nodes(strict, first, last):
    """The nodes of all the lines of first..last."""
    rows = (json.loads(line) for line in Path(strict).read_text().splitlines())
    return sum(row["nodes"] for row in rows if first <= row["n"] <= last)


def top(args):
    found = sorted(searches(args.strict, args.first, args.last), key=lambda g: -g["nodes"])
    total = stretch_nodes(args.strict, args.first, args.last)
    print(f"The searches of {args.first}..{args.last}: {len(found)} games searched, {total / 1e9:.2f} G nodes in all, "
          f"{sum(g['hours'] for g in found):.1f} wall hours.\n")
    print("| rank | game | nodes | wall hours | invocations |")
    print("|---|---|---|---|---|")
    for rank, g in enumerate(found[: args.count], 1):
        print(f"| {rank} | {g['n']} | {g['nodes']:,} | {g['hours']:.2f} | {g['invocations']} |")
    print("\n| dearest games | nodes | share of the stretch's nodes |")
    print("|---|---|---|")
    for k in SHARES:
        nodes = sum(g["nodes"] for g in found[:k])
        print(f"| {k} | {nodes:,} | {100 * nodes / total:.1f}% |")


def smallest(args):
    found = [g for g in searches(args.strict, args.first, args.last) if g["nodes"] >= args.lo]
    print("| game | nodes | wall hours | invocations |")
    print("|---|---|---|---|")
    for g in sorted(found, key=lambda g: g["nodes"])[: args.count]:
        print(f"| {g['n']} | {g['nodes']:,} | {g['hours']:.3f} | {g['invocations']} |")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("top", "smallest"):
        s = sub.add_parser(name)
        s.add_argument("--from", dest="first", type=int, default=3915)
        s.add_argument("--to", dest="last", type=int, default=4547)
        s.add_argument("--count", type=int, default=20 if name == "top" else 12)
        s.add_argument("--strict", default=STRICT, type=Path)
    sub.choices["smallest"].add_argument("--lo", type=int, default=1_000_000)
    args = parser.parse_args()
    {"top": top, "smallest": smallest}[args.command](args)


if __name__ == "__main__":
    main()
