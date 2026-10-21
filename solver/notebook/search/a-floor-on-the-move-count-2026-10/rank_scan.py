#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Whether the floor `promotions + 1` cuts a game's root: whether the recorded optimal game has as many promotions as a
search can make at all.

usage: python3 -I rank_scan.py WALL [--count 60] [--seed 7] [--lo 1000] [--hi 4547]

For a sample of the games of the chain that were searched, runs `wall images --root --count --nodes 3` with the floor
(TAXMAN_FLOOR) one above the number of promotions of the game's recorded optimal game, and prints, from the line, whether
the floor applied at the root (`applies`) and why it cut: no common independent set of that many members (`no_set`,
the intersection's rank is the promotions'), or more promotions than the root can add (`over_cap`).
"""
import argparse
import json
import os
import random
import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
KNOWN = REPO / "results" / "strict.jsonl"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("wall")
    parser.add_argument("--count", type=int, default=60)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--lo", type=int, default=1000)
    parser.add_argument("--hi", type=int, default=4547)
    args = parser.parse_args()
    rows = {row["n"]: row for row in map(json.loads, KNOWN.read_text().splitlines())}
    searched = [n for n in range(args.lo, args.hi + 1) if rows[n]["method"] == "search"]
    sample = sorted(random.Random(args.seed).sample(searched, args.count))
    print("n promotions applies no_set over_cap cuts")
    cut = 0
    for n in sample:
        promotions = len(rows[n]["promotions"])
        env = dict(os.environ, TAXMAN_FLOOR=str(promotions + 1))
        command = [args.wall, "images", "--known", str(KNOWN), "--root", "--count", "--nodes", "3", str(n)]
        done = subprocess.run(command, env=env, capture_output=True, text=True, check=True)
        line = json.loads(done.stdout.strip().splitlines()[-1])
        print(n, promotions, line["floor_applies"], line["floor_no_set"], line["floor_over_cap"], line["floor_cuts"])
        cut += line["floor_cuts"] > 0
    print(f"the root is cut at promotions + 1 in {cut} of {len(sample)} games")


if __name__ == "__main__":
    main()
