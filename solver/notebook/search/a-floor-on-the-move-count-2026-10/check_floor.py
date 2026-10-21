#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The data under the floor probe, and its checks (solver/frame/examples/floor_plain.rs is the Rust half).

usage: python3 -I check_floor.py moves
           the steps of results/strict.jsonl by the change in len(moves)
       python3 -I check_floor.py plain PLAIN_BIN N...
           for each N: len(moves) - len(promotions) against the length of the game with no promotions, which
           PLAIN_BIN (the example floor_plain) prints
       python3 -I check_floor.py floor PLAIN_BIN N...
           for each N: the floor F = len(moves(N - 1)) - len(plain(N)), and len(promotions(N)) against it
       python3 -I check_floor.py every PLAIN_BIN
           both checks for every game of the chain, in a line each for the summary
"""
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
STRICT = REPO / "results" / "strict.jsonl"


def chain():
    """The chain's lines by n."""
    return {row["n"]: row for row in map(json.loads, STRICT.read_text().splitlines())}


def plain_lengths(binary, games):
    """The length of the game with no promotions, by n, from the example floor_plain."""
    out = subprocess.run([binary, *map(str, games)], capture_output=True, text=True, check=True).stdout
    return {int(n): int(length) for n, length, _ in (line.split() for line in out.splitlines())}


def moves():
    rows = chain()
    steps = {"shrinks": 0, "same": 0, "grows by one": 0, "grows by more": 0}
    ceiling = {"same": 0, "grows by one": 0, "other": 0}
    below = {"same": 0, "grows by one": 0, "other": 0}
    for n in range(2, max(rows) + 1):
        d = len(rows[n]["moves"]) - len(rows[n - 1]["moves"])
        key = "shrinks" if d < 0 else "same" if d == 0 else "grows by one" if d == 1 else "grows by more"
        steps[key] += 1
        at_ceiling = rows[n]["score"] == n + rows[n - 1]["score"]
        group = ceiling if at_ceiling else below
        group[key if key in group else "other"] += 1
    print(f"each step n - 1 to n, n = 2..{max(rows)} ({sum(steps.values())} steps):", steps)
    print("at the ceiling (score = n + score(n - 1)):", ceiling)
    print("below the ceiling:", below)


def plain(binary, games):
    rows = chain()
    lengths = plain_lengths(binary, games)
    bad = 0
    for n in games:
        row = rows[n]
        have = len(row["moves"]) - len(row["promotions"])
        scores = row["in_the_bag"] is None or row["in_the_bag"] + sum(row["promotions"]) == row["score"]
        ok = have == lengths[n] and scores
        bad += not ok
        print(f"n={n} method={row['method']} moves={len(row['moves'])} promotions={len(row['promotions'])} "
              f"moves-promotions={have} plain={lengths[n]} {'ok' if ok else 'DIFFERENT'}")
    print("all equal" if not bad else f"{bad} differ")


def floor(binary, games):
    rows = chain()
    lengths = plain_lengths(binary, games)
    for n in games:
        floor_f = len(rows[n - 1]["moves"]) - lengths[n]
        have = len(rows[n]["promotions"])
        print(f"n={n} moves(n-1)={len(rows[n - 1]['moves'])} plain(n)={lengths[n]} F={floor_f} "
              f"promotions(n)={have} {'ok' if have >= floor_f else 'BELOW THE FLOOR'}")


def every(binary):
    rows = chain()
    games = sorted(rows)[1:]
    lengths = plain_lengths(binary, games)
    differ = [n for n in games if len(rows[n]["moves"]) - len(rows[n]["promotions"]) != lengths[n]]
    print(f"games 2..{games[-1]}: moves - promotions is the length of the game with no promotions in "
          f"{len(games) - len(differ)} of {len(games)}")
    floors = {n: len(rows[n - 1]["moves"]) - lengths[n] for n in games[1:]}
    below = [n for n in floors if len(rows[n]["promotions"]) < floors[n]]
    equal = [n for n in floors if len(rows[n]["promotions"]) == floors[n]]
    print(f"games 3..{games[-1]}: promotions below the floor F in {len(below)} of {len(floors)}; "
          f"equal to F in {len(equal)}; one above F in "
          f"{sum(len(rows[n]['promotions']) == floors[n] + 1 for n in floors)}")


if __name__ == "__main__":
    command, rest = sys.argv[1], sys.argv[2:]
    if command == "moves":
        moves()
    elif command == "plain":
        plain(rest[0], [int(n) for n in rest[1:]])
    elif command == "floor":
        floor(rest[0], [int(n) for n in rest[1:]])
    elif command == "every":
        every(rest[0])
    else:
        sys.exit(__doc__)
