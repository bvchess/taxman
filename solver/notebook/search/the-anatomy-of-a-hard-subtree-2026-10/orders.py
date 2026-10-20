#!/usr/bin/env python3
"""Branching orders for `wall anatomy --prefer FILE`, from a unit's trace (README.md beside this file, question 7).

usage: python3 -I orders.py TRACE_DIR ID KIND [HOME] > FILE

KIND is one of:
  cores    the members of the conflicts the trace's branching steps resolved, the most often resolved first;
  home     the candidates of home HOME, the largest first, then every other candidate, the largest first;
  largest  every candidate, the largest first (strong branching's forced members still come first).
The game's candidates are those of `TRACE_DIR/N.candidates.tsv`, N the game of the unit ID (`N-INDEX`).  Standard
library only.
"""
import sys
from collections import Counter
from pathlib import Path


def main():
    trace, uid, kind = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    game = uid.split("-")[0]
    home = {}
    with open(trace / f"{game}.candidates.tsv") as f:
        next(f)
        for line in f:
            x, h, _ = line.split("\t")
            home[int(x)] = int(h)
    by_value = sorted(home, reverse=True)
    if kind == "largest":
        order = by_value
    elif kind == "home":
        h = int(sys.argv[4])
        order = [x for x in by_value if home[x] == h] + [x for x in by_value if home[x] != h]
    elif kind == "cores":
        with open(trace / f"{uid}.conflicts.tsv") as f:
            next(f)
            listed = [[int(x) for x in line.rstrip("\n").split("\t")[2].split(",") if x] for line in f]
        uses = Counter()
        with open(trace / f"{uid}.steps.tsv") as f:
            names = next(f).rstrip("\n").split("\t")
            col = {n: i for i, n in enumerate(names)}
            last_child = -1
            for line in f:
                v = line.rstrip("\n").split("\t")
                kid = int(v[col["kid"]])
                if kid < 0 and v[col["inh"]] == "1":
                    kid = last_child
                if int(v[col["br"]]) and kid >= 0:
                    for x in listed[kid]:
                        uses[x] += 1
                last_child = int(v[col["ckid"]])
        order = [x for x, _ in sorted(uses.items(), key=lambda kv: (-kv[1], -kv[0]))]
        order += [x for x in by_value if x not in uses]
    else:
        sys.exit(f"unknown kind {kind}: cores, home or largest")
    print(" ".join(map(str, order)))


if __name__ == "__main__":
    main()
