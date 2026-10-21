#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Measurement 1 as a table: the finder alone, from the start a search begins at, for each game.

usage: finder_alone.py FINDER_ALONE.jsonl     (the lines `examples/finder.rs` printed, all games in one file)

For each game it prints the start's distance below the target (the game's final best), then for each budget the number
of the 3 seeds that reached the target, the seconds at which each did, and the best of the seeds that did not.
"""
import json
import sys

starts, runs = {}, {}
for line in open(sys.argv[1]):
    row = json.loads(line)
    if "seconds" not in row:
        starts[row["game"]] = row
    else:
        runs.setdefault(row["game"], []).append(row)

print("| game | target | start below | 5 s: seeds that reached it (seconds) | 60 s: seeds that reached it (seconds) |")
print("|---|---|---|---|---|")
total = {5.0: [0, 0], 60.0: [0, 0]}
for game in sorted(runs):
    cells = []
    first = runs[game][0]
    for budget in (5.0, 60.0):
        rows = [r for r in runs[game] if r["seconds"] == budget]
        hit = [r for r in rows if r["reached"]]
        total[budget][0] += len(hit)
        total[budget][1] += len(rows)
        seconds = ", ".join(f"{r['found_s']:.1f}" for r in hit)
        miss = [r for r in rows if not r["reached"]]
        below = f"; the others {min(r['target'] - r['best'] for r in miss)} short at best" if miss else ""
        cells.append(f"{len(hit)} of {len(rows)}" + (f" ({seconds})" if hit else "") + below)
    print(f"| {game} | {first['target']} | {first['target'] - first['start']} | {cells[0]} | {cells[1]} |")
for budget, (hit, runs_) in total.items():
    print(f"all games, {budget:.0f} s: {hit} of {runs_} runs reached the target")
