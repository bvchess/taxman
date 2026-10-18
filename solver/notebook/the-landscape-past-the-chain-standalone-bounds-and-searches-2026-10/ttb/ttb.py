#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Time to the best game against the time to the proof: standalone one-thread searches of 60 games of 1301..2175
(one.sh; a ladder line a game in N.txt).

usage: ./ttb.py
"""
import re
import statistics as st
from pathlib import Path

HERE = Path(__file__).resolve().parent
# `search 1.33 s nodes 10632 ... best 70636 opt 70636 ... | start 70440, found at 0.80 s, 9000 steps`, or, when the
# local search's start was already the best game, `| start incumbent 70636`.
LINE = re.compile(r"n=(\d+) T=1 (\w+)\s+search\s+([\d.]+) s nodes\s+(\d+).*?best (\d+) opt (\d+).*?\| "
                  r"(?:start (\d+), found at ([\d.]+) s, (\d+) steps|start incumbent (\d+))")

rows = []
for path in HERE.glob("*.txt"):
    if path.name == "games.txt":
        continue
    n, state, seconds, nodes, best, opt, start, found, steps, start_best = LINE.search(path.read_text()).groups()
    rows.append(dict(n=int(n), done=state == "DONE", seconds=float(seconds), nodes=int(nodes), best=int(best),
                     opt=int(opt), start=int(start or start_best), found=float(found) if found else None))
rows.sort(key=lambda r: r["seconds"])
searched = [r for r in rows if r["done"] and r["found"] is not None]
print(f"{len(rows)} games, {sum(r['done'] for r in rows)} finished in 300 s (the longest {rows[-1]['seconds']:.1f} s); "
      f"reached the optimum: {sum(r['best'] == r['opt'] for r in rows)}; the start was already the optimum in "
      f"{sum(r['found'] is None for r in rows)}")
share = [r["found"] / r["seconds"] for r in searched]
print(f"share of the time spent before the best game was found, over the {len(searched)} games whose search improved "
      f"on the start: median {st.median(share):.2f}, p10 {sorted(share)[len(share) // 10]:.2f}, "
      f"p90 {sorted(share)[9 * len(share) // 10]:.2f}")
print(f"{'n':>5} {'total s':>8} {'found s':>8} {'share':>6} {'nodes':>10} {'start below opt':>16}")
for r in rows:
    found, part = ("start", "") if r["found"] is None else (f"{r['found']:.2f}", f"{r['found'] / r['seconds']:.2f}")
    print(f"{r['n']:>5} {r['seconds']:8.2f} {found:>8} {part:>6} {r['nodes']:>10} {r['opt'] - r['start']:>16}")
