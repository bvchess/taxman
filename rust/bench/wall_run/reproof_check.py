#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The run to the wall against the committed chain: reproof_check.py

The run solved games 1..3440 again before it went on; reproof.jsonl.gz holds its lines of those games, whole.  Its
lines of 3441..3914 are the committed file's, byte for byte, so they are not repeated here.  This prints the games whose score, proof or method differ from
results/strict.jsonl's (none), and, by range, the search seconds and nodes of the run against the committed lines
(1..2175 is the first run, 2176..3440 the second, with the older build; 3441..3914 the run itself), and the run's
longest games.  Standard library only.
"""
import gzip
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
RANGES = [(1, 2175), (2176, 3440), (3441, 3914)]


def search_seconds(game):
    return game["seconds"]["search_all_runs"]


with gzip.open(HERE / "reproof.jsonl.gz", "rt") as f:
    run = {game["n"]: game for game in map(json.loads, f)}
with open(HERE / "../../results/strict.jsonl") as f:
    committed = {game["n"]: game for game in map(json.loads, f)}
# the run's own lines of 3441..3914 are the committed file's
run.update({n: game for n, game in committed.items() if n > 3440})

compared = range(1, 3441)
different = [n for n in compared if any(run[n][key] != committed[n][key] for key in ("score", "proof", "method"))]
print(f"games 1..3440: {len(compared)} compared, {len(different)} differ in score, proof or method {different[:10]}")
print(f"{'games':>11}  {'run: hours':>10} {'G nodes':>8}   {'committed: hours':>16} {'G nodes':>8}"
      f"  (search seconds, summed)")
for first, last in RANGES:
    ns = range(first, last + 1)
    print(f"{first:>5}-{last:<5}  {sum(search_seconds(run[n]) for n in ns) / 3600:>10.2f} "
          f"{sum(run[n]['nodes'] for n in ns) / 1e9:>8.2f}   "
          f"{sum(search_seconds(committed[n]) for n in ns) / 3600:>16.2f} "
          f"{sum(committed[n]['nodes'] for n in ns) / 1e9:>8.2f}")
print("the longest games of the run:")
for game in sorted(run.values(), key=search_seconds, reverse=True)[:6]:
    print(f"  {game['n']}: {search_seconds(game):,.0f} s ({search_seconds(game) / 3600:.2f} h), "
          f"{game['nodes'] / 1e9:.3f} G nodes")
