#!/usr/bin/env python3
"""The searches of the chain from its progress lines: steps, own search, rate, start and best, efficiency and tail.

Usage: timeline.py CHAIN_ERR [MINUTES]

CHAIN_ERR is the chain's stderr (`~/taxman-work/logs/chain.err`), with a progress line a minute for each game in
flight.  A game's last invocation is the run of lines after the last fall of its step count; its own search starts at
the last line whose steps are still within 1,000 of the first (the time it waited, suspended, behind an older game).
Prints a line a game: the steps and seconds of its own search, the rate, the best of its first and last lines, the
least efficiency and the largest tail of its lines.  With MINUTES, also the rate in the first MINUTES of its own
search and in the rest.
"""
import re
import sys

PATTERN = re.compile(r"^n=(\d+) (?:(\d+)h)?(\d+)m(\d+)s nodes (\d+) \(([^)]*)\) best (\d+) queue (\d+)"
                     r".*?eff ([\d.]+)%.*?tail ([\d.]+)%")


def main():
    split = float(sys.argv[2]) * 60 if len(sys.argv) > 2 else None
    games = {}
    for line in open(sys.argv[1]):
        m = PATTERN.match(line)
        if not m:
            continue
        seconds = int(m[2] or 0) * 3600 + int(m[3]) * 60 + int(m[4])
        games.setdefault(int(m[1]), []).append(
            (seconds, int(m[5]), m[6], int(m[7]), int(m[8]), float(m[9]), float(m[10])))
    for n, rows in sorted(games.items()):
        start = 0
        for i in range(1, len(rows)):
            if rows[i][1] < rows[i - 1][1]:
                start = i
        rows = rows[start:]
        began = max([r[0] for r in rows if r[1] <= rows[0][1] + 1000] + [rows[0][0]])
        last = rows[-1]
        own = last[0] - began
        steps = last[1] - rows[0][1]
        text = "%d steps %d own %ds rate %.1fk/s start %d best %d (%s) eff_min %.1f tail_max %.1f" % (
            n, steps, own, steps / max(own, 1) / 1000, rows[0][3], last[3], last[2],
            min(r[5] for r in rows), max(r[6] for r in rows))
        if split is not None:
            mid = [r for r in rows if r[0] <= began + split]
            if mid and own > split:
                first = (mid[-1][1] - rows[0][1]) / max(mid[-1][0] - began, 1)
                rest = (last[1] - mid[-1][1]) / max(last[0] - mid[-1][0], 1)
                text += " first %.1fk/s rest %.1fk/s" % (first / 1000, rest / 1000)
        print(text)


if __name__ == "__main__":
    main()
