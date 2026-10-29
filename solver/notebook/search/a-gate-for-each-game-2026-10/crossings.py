#!/usr/bin/env python3
"""For each game's profile: when the open units' gap quantiles first fall under a level, and the share of the game's
steps after that point.  Usage: crossings.py PROFILE..."""
import json
import sys

QUANTS = ["q0", "q10", "q25", "q50", "q75", "q90", "max"]
LEVELS = [1400, 1200, 1000, 800]


def read(path):
    opens, end, start_inc = [], None, None
    with open(path) as f:
        for line in f:
            if '"kind":"node"' in line[:20]:
                continue
            r = json.loads(line)
            k = r.get("kind")
            if k == "open" and r.get("gaps"):
                opens.append((r["nodes"], r["gaps"], r["units"]))
            elif k == "end":
                end = r
            elif k == "start" and start_inc is None:
                start_inc = r
    return opens, end


def main():
    rows = []
    with open(sys.argv[1]) as f:
        paths = f.read().split()
    for path in paths:
        opens, end = read(path)
        n = path.split("/n")[-1].split(".")[0]
        if not opens:
            continue
        total = end["nodes"] if end else opens[-1][0]
        done = end is not None and end.get("completed")
        out = {"n": n, "total": total, "done": done, "first": opens[0][1], "cross": {}}
        for qi in (6, 5, 3):
            for lev in LEVELS:
                at = next((nodes for nodes, g, _ in opens if g[qi] < lev), None)
                out["cross"][(QUANTS[qi], lev)] = None if at is None else 1 - at / total
        rows.append(out)
    hdr = "game   steps(G) done  first-gaps(q0..max)                  " + " ".join(
        f"{q}<{lev}" for q in ("max", "q90", "q50") for lev in LEVELS)
    print(hdr)
    for r in rows:
        cells = []
        for q in ("max", "q90", "q50"):
            for lev in LEVELS:
                v = r["cross"][(q, lev)]
                cells.append(f"{'-' if v is None else f'{v:.2f}':>{len(q) + 5}}")
        print(f"{r['n']:5} {r['total'] / 1e9:8.2f} {str(r['done'])[0]:>4}  {str(r['first']):36} " + " ".join(cells))


if __name__ == "__main__":
    main()
