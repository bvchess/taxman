#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""What a longer climb limit would cost, from a ledger run's events: climb_limit.py DIR.

From DIR/events.jsonl (or its gzipped copy): the searches that began with the board below proven, and, for each limit,
the proofs a climb of that limit on every searched board would have made and what it would have cost.  A board the run
finished by a search costs its total search seconds there (at most the limit); a board it did not finish costs the
limit.  That is an upper bound on a climb's cost, since the run's searches had not always the game and the exact
ceiling of a proven board below them.  Then the run's own cost per proof by exhausted search over its CPU hours, the
price a climb has to beat.  Standard library only.
"""
import gzip
import json
import os
import sys
from collections import defaultdict


def lines(path):
    """The JSON lines of `path`, or of `path`.gz if only that is there."""
    opener = open if os.path.exists(path) else gzip.open
    with opener(path if opener is open else path + ".gz", "rt") as f:
        return [json.loads(line) for line in f if line.strip()]


def main(directory):
    events = lines(f"{directory}/events.jsonl")
    proven_at = {e["n"]: e["t"] for e in events if e["action"] == "proven"}
    by_board = defaultdict(list)
    for e in events:
        if e["action"] == "search":
            by_board[e["n"]].append(e)

    never = float("inf")
    began_below = [
        e for runs in by_board.values() for e in runs if proven_at.get(e["n"] - 1, never) <= e["t"] - e["seconds"]
    ]
    done = [e for e in began_below if e["completed"] or e["reached_cap"]]
    hours = sum(e["seconds"] for e in began_below) / 3600
    print(f"searches that began with the board below proven: {len(began_below)}, {hours:.2f} core-hours, "
          f"{len(done)} completed")

    total = {n: sum(e["seconds"] for e in runs) for n, runs in by_board.items()}
    finished = sorted(total[n] for n, runs in by_board.items() if any(e["completed"] or e["reached_cap"] for e in runs))
    unfinished = len(by_board) - len(finished)
    print(f"boards searched {len(by_board)}, finished by a search {len(finished)}, not {unfinished}")
    print("limit  proofs  core-hours  core-s/proof  core-s per proof added since the limit before")
    previous = None
    for limit in (10, 20, 30, 45, 60, 90, 120, 180, 300, 600):
        proofs = sum(1 for t in finished if t <= limit)
        cost = sum(min(limit, t) for t in finished) + limit * unfinished
        row = f"{limit:>5} {proofs:>7} {cost / 3600:>11.2f} {cost / max(proofs, 1):>13.0f}"
        if previous:
            row += f" {(cost - previous[1]) / max(proofs - previous[0], 1):>12.0f}"
        print(row)
        previous = (proofs, cost)

    cpu = sorted(e["cpu"] for e in events if e["action"] == "proven" and e["upper_rule"] == "exhausted")
    end = events[-1]["cpu"] / 3600
    print(f"the run's own proofs by an exhausted search ({len(cpu)}), core-seconds each over its CPU hours:")
    for first in range(0, int(end), 8):
        count = sum(1 for c in cpu if first * 3600 <= c < (first + 8) * 3600)
        each = 8 * 3600 / max(count, 1)
        print(f"  {first:>3}-{first + 8:<3} core-hours: {count:>3} proofs, {each:>5.0f} core-s each")


if __name__ == "__main__":
    main(sys.argv[1])
