#!/usr/bin/env python3
"""The rule with a second condition: of the violations in violations.tsv up to TOP, which rule games can take one more
number (some y outside the game whose addition keeps it playable), and which cannot.

Run from the top of the repository: PYTHONPATH=heuristics python3 <this folder>/nothing_more.py [TOP]  (default 300).
"""

import sys
from pathlib import Path

from swaplib import Playability, load_games


def main() -> None:
    top = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    rows = [line.rstrip("\n").split("\t") for line in open(Path(__file__).with_name("violations.tsv"))][1:]
    games = load_games(top)
    pl = Playability(top)
    more, stuck = [], []
    for row in rows:
        n, x, rule, opt = int(row[0]), int(row[1]), int(row[2]), int(row[3])
        if n > top:
            continue
        game = set(games[n - 1]["moves"]) - {x} | {n}
        y = next((y for y in range(n, 1, -1) if y not in game and pl.playable(n, game | {y})), None)
        (stuck if y is None else more).append((n, x, rule, opt, y))
    print(f"violations up to {top}: {len(more) + len(stuck)}; one more number fits: {len(more)}; none fits: {len(stuck)}")
    for n, x, rule, opt, _ in stuck:
        print(f"  n={n}: gives up {x}, scores {rule}, opt {opt}, nothing more fits")


if __name__ == "__main__":
    main()
