#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Search time per game of the run to the wall, against the last cloud run on the same games: an SVG, log y.

plot_search_times.py OUT.svg.  The run's games 1..3440 are in reproof.jsonl.gz beside this script, and its games from
3441 on are the committed chain's own lines (results/strict.jsonl); the last cloud run is the committed lines of
2176..3440, made with the build of 55287ff.  A game is drawn if it was searched (its proof exhausted the search or
reached the ceiling, after at least a millisecond of search)."""
import gzip
import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
STRICT = HERE / "../../../results/strict.jsonl"
OUT = sys.argv[1]


def games(path):
    with (gzip.open(path, "rt") if path.suffix == ".gz" else open(path)) as f:
        return [json.loads(line) for line in f]


def search_seconds(rows, lo, hi):
    out = {}
    for d in rows:
        s = (d.get("seconds") or {}).get("search", 0) or 0
        if lo <= d["n"] <= hi and d.get("proof") in ("exhausted", "cap") and s >= 0.001 and (d.get("nodes") or 0) > 0:
            out[d["n"]] = s
    return out


committed = games(STRICT)
run_rows = games(HERE / "reproof.jsonl.gz") + [d for d in committed if d["n"] > 3440]
last_n = max(d["n"] for d in run_rows)
run = search_seconds(run_rows, 1, last_n)
old = search_seconds(committed, 2176, 3440)

W, H = 1000, 560
L, R, T, B = 80, 30, 70, 60
X0, X1 = 0, math.ceil(last_n / 500) * 500
Y0, Y1 = -3, 4  # decades: 0.001 s to 10,000 s


def x(n):
    return L + (n - X0) / (X1 - X0) * (W - L - R)


def y(s):
    v = min(max(math.log10(s), Y0), Y1)
    return T + (Y1 - v) / (Y1 - Y0) * (H - T - B)


ink, muted, grid = "#1f2328", "#59636e", "#d1d9e0"
blue, gray = "#2f6fdb", "#b4bcc6"
parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" font-family="Helvetica, Arial, sans-serif">',
         f'<rect width="{W}" height="{H}" fill="#ffffff"/>',
         f'<text x="{L}" y="30" font-size="18" fill="{ink}" font-weight="600">'
         f'Search time per game, the run to the wall (1..{last_n})</text>',
         f'<text x="{L}" y="52" font-size="13" fill="{muted}">One c8g.16xlarge, 64 threads; games settled without a '
         f'search (primes, repeats, ceilings reached at the first node) are not shown.</text>']
for d in range(Y0, Y1 + 1):
    yy = y(10 ** d)
    label = {-3: "0.001", -2: "0.01", -1: "0.1"}.get(d, f"{10 ** d:,}")
    parts.append(f'<line x1="{L}" x2="{W - R}" y1="{yy:.1f}" y2="{yy:.1f}" stroke="{grid}" stroke-width="1"/>')
    parts.append(f'<text x="{L - 8}" y="{yy + 4:.1f}" font-size="12" fill="{muted}" text-anchor="end">{label}</text>')
for n in range(X0, X1 + 1, 500):
    xx = x(n)
    parts.append(f'<line x1="{xx:.1f}" x2="{xx:.1f}" y1="{H - B}" y2="{H - B + 5}" stroke="{muted}"/>')
    parts.append(f'<text x="{xx:.1f}" y="{H - B + 20}" font-size="12" fill="{muted}" text-anchor="middle">{n}</text>')
parts.append(f'<line x1="{L}" x2="{W - R}" y1="{H - B}" y2="{H - B}" stroke="{muted}"/>')
parts.append(f'<text x="{(L + W - R) / 2}" y="{H - 15}" font-size="13" fill="{ink}" text-anchor="middle">n</text>')
parts.append(f'<text x="22" y="{(T + H - B) / 2}" font-size="13" fill="{ink}" text-anchor="middle" '
             f'transform="rotate(-90 22 {(T + H - B) / 2})">search seconds (log scale)</text>')
for n, s in sorted(old.items()):
    parts.append(f'<circle cx="{x(n):.1f}" cy="{y(s):.1f}" r="2.2" fill="{gray}"/>')
for n, s in sorted(run.items()):
    parts.append(f'<circle cx="{x(n):.1f}" cy="{y(s):.1f}" r="2.2" fill="{blue}"/>')
lx, ly = L + 20, T + 14
parts.append(f'<rect x="{lx - 10}" y="{ly - 16}" width="330" height="48" fill="#ffffff" stroke="{grid}"/>')
parts.append(f'<circle cx="{lx + 4}" cy="{ly - 3}" r="4" fill="{blue}"/>')
parts.append(f'<text x="{lx + 14}" y="{ly + 1}" font-size="12" fill="{ink}">this run ({len(run)} searches)</text>')
parts.append(f'<circle cx="{lx + 4}" cy="{ly + 17}" r="4" fill="{gray}"/>')
parts.append(f'<text x="{lx + 14}" y="{ly + 21}" font-size="12" fill="{ink}">last cloud run, same games '
             f'({len(old)} searches)</text>')
parts.append("</svg>")
open(OUT, "w").write("\n".join(parts))
big = sorted(run.items(), key=lambda kv: -kv[1])[:5]
print(len(run), "searches; slowest:", [(n, round(s)) for n, s in big])
print("total search hours this run:", round(sum(run.values()) / 3600, 2), " old same games:",
      round(sum(old.values()) / 3600, 2))
