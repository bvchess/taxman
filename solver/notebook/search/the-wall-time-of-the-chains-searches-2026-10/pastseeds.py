#!/usr/bin/env python3
"""Whether the better of the two seeds' starts reaches each searched game's best, on the chain's games.

Usage: pastseeds.py STRICT SEEDS...

STRICT is the chain's results file, SEEDS the lines of `examples/seeds.rs N N --known STRICT` for searched games (the
game of n − 1 is the chain's, the other seed the bounds store's game of n).  Prints, for each game, the best (the score
less in_the_bag), each seed's start and how far the better start lies below the best, then the counts.
"""
import json
import sys


def main():
    best = {}
    for line in open(sys.argv[1]):
        d = json.loads(line)
        if d.get("in_the_bag") is not None and d.get("completed"):
            best[d["n"]] = d["score"] - d["in_the_bag"]
    rows = []
    for path in sys.argv[2:]:
        for line in open(path):
            r = json.loads(line)
            rows.append(r)
    counts = {"rule": 0, "both": 0, "prev": 0, "store": 0}
    for r in sorted(rows, key=lambda r: r["game"]):
        b = best[r["game"]]
        short = {"rule": b - r["chain"], "both": b - r["both"],
                 "prev": b - r["prev_start"], "store": b - r["other_start"]}
        for k in counts:
            counts[k] += short[k] == 0
        print(r["game"], "best", b, "n-1 start", r["prev_start"], "store start", r["other_start"],
              "below: rule", short["rule"], "both", short["both"])
    print(len(rows), "games; the start reaches the best:",
          ", ".join(f"{k} {v}" for k, v in counts.items()))


if __name__ == "__main__":
    main()
