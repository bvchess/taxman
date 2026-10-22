#!/usr/bin/env python3
"""How far the bounds store's games lie below each searched game's best, from 3915 on.

Usage: storegap.py BOUNDS STRICT

BOUNDS is results/bounds.jsonl (the store), STRICT the chain's results file.  For each game of STRICT that a search
proved and the store holds a lower bound with its promotions for, prints the shortfall (the game's score less the
store's best lower score), the search's steps and its build, and a count of the games by shortfall.
"""
import collections
import json
import sys


def main():
    bounds, strict = sys.argv[1], sys.argv[2]
    low = {}
    for line in open(bounds):
        d = json.loads(line)
        if d["side"] == "lower" and "promotions" in d:
            low[d["n"]] = max(low.get(d["n"], 0), d["value"])
    last = {}
    for line in open(strict):
        d = json.loads(line)
        last[d["n"]] = d
    counts = collections.Counter()
    for n, d in sorted(last.items()):
        if n < 3915 or d.get("method") != "search" or n not in low:
            continue
        short = d["score"] - low[n]
        counts[0 if short == 0 else 1 if short < 50 else 2 if short < 150 else 3] += 1
        if short >= 50:
            print(n, "short", short, "steps", d["nodes"], "build", d["build"]["commit"][:7])
    print("searched games with a store game, by shortfall 0, 1-49, 50-149, 150 and more:",
          [counts[i] for i in range(4)])


if __name__ == "__main__":
    main()
