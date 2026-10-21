#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Measurement 2 as a table: whole games solved on one thread with the finder off and on.

usage: whole_games.py DIR GAME...     (DIR holds GAME-off.jsonl, GAME-on.jsonl, their .err and .time files, which
                                       `taxman solve GAME --use-prev --force-search --threads 1 [--finder-after 100000]
                                       --out GAME-MODE.jsonl` under `/usr/bin/time -l` made)

For each game it prints a row a run: the steps, the search's seconds, the CPU seconds of the process (user and system,
the finder's thread included), the score, the incumbent of the first progress line (two minutes in) and what the finder
did.
"""
import json
import re
import sys

D = sys.argv[1].rstrip("/") + "/"
print("| game | finder | steps | search s | CPU s | score | incumbent at 2 min | finder: at steps, from, best, accepted, found at |")
print("|---|---|---|---|---|---|---|---|")
for game in sys.argv[2:]:
    rows = {}
    for mode in ("off", "on"):
        line = json.loads(open(f"{D}{game}-{mode}.jsonl").readline())
        time = open(f"{D}{game}-{mode}.time").read()
        user = float(re.search(r"([\d.]+)\s+user", time).group(1))
        system = float(re.search(r"([\d.]+)\s+sys", time).group(1))
        first = next(re.search(r" best (\d+)", l).group(1) for l in open(f"{D}{game}-{mode}.err") if " nodes " in l)
        finder = line["engine"].get("finder")
        what = (f"{finder['at_nodes']:,}, {finder['from']}, {finder['best']}, {finder['improved']}, "
                f"{finder['found_seconds']:.1f} s" if finder else "")
        rows[mode] = (line["nodes"], line["seconds"]["search"], user + system, line["score"])
        print(f"| {game} | {mode} | {line['nodes']:,} | {line['seconds']['search']:.0f} | {user + system:.0f} | "
              f"{line['score']} | {first} | {what} |")
    off, on = rows["off"], rows["on"]
    print(f"|  | on / off | {on[0] / off[0]:.2f} | {on[1] / off[1]:.2f} | {on[2] / off[2]:.2f} | "
          f"{'equal' if on[3] == off[3] else 'DIFFER'} | | |")
