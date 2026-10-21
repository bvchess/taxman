#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The check that the promotions on a unit's path from its game's root count toward the floor's `p`.

usage: python3 -I check_units.py WALL [--nodes 300]

For every unit of the hardest searches' suite-hard.jsonl, runs `wall images --suite ... --ids ID --count` for a few
steps with the floor at 1 (which never cuts, since a unit's node has promoted at least one number) and reads the
probe's `floor_first_promoted` and `floor_first_cap` (src/search/floor.rs): the promotions and the cap that the first
bound check, the unit's own node's, was given.  A unit's `promoted` set is the numbers promoted on its path from the
game's root (the suite's `depth` is their count), and the searcher's node starts from it, so the first check's
promotions must equal the unit's `depth`, and its cap must be at most the game's `max_total` less `depth`.  Prints a
line a unit and the count of units that differ.
"""
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
SUITE = HERE.parent / "the-hardest-searches-of-the-run-to-5000-2026-10" / "suite-hard.jsonl"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("wall")
    parser.add_argument("--nodes", type=int, default=300)
    args = parser.parse_args()
    rows = [json.loads(line) for line in SUITE.read_text().splitlines()]
    max_total = {row["game"]: row["max_total"] for row in rows if row["kind"] == "game"}
    env = {**os.environ, "TAXMAN_FLOOR": "1"}
    bad = 0
    for unit in (row for row in rows if row["kind"] == "unit"):
        command = [args.wall, "images", "--known", str(REPO / "results" / "strict.jsonl"), "--suite", str(SUITE),
                   "--ids", unit["id"], "--count", "--nodes", str(args.nodes), str(unit["game"])]
        done = subprocess.run(command, env=env, capture_output=True, text=True, check=True)
        line = json.loads(done.stdout.strip().splitlines()[-1])
        first, cap = line["floor_first_promoted"], line["floor_first_cap"]
        ok = first == unit["depth"] and cap <= max_total[unit["game"]] - unit["depth"]
        bad += not ok
        print(f"{unit['id']}: depth {unit['depth']}, first check's promotions {first}, cap {cap} "
              f"(max_total {max_total[unit['game']]}) {'ok' if ok else 'DIFFERENT'}")
    print("all equal" if not bad else f"{bad} differ")
    sys.exit(bool(bad))


if __name__ == "__main__":
    main()
