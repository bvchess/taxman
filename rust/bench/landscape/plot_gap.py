#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The standalone gap (quick upper bound less quick lower bound) of every game 2..8191, and the gap to the optimum where
it is known: an SVG with a linear y axis, per-game dots and rolling medians.  plot_gap.py OUT.svg

The games are those of low.jsonl.gz (2..2175, the quick bounds with their known optima), cal.jsonl (2176..3440) and
map.jsonl.gz (3441..8191), all beside this script; a game in two of them is taken from the first."""
import gzip
import json
import statistics as st
import sys
from pathlib import Path

LAND = Path(__file__).resolve().parent
OUT = sys.argv[1]
HALF = 50  # the rolling median takes 101 games


def lines(path):
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rt") as f:
        return [json.loads(x) for x in f if x.strip()]


games = {}
for path in (LAND / "low.jsonl.gz", LAND / "cal.jsonl", LAND / "map.jsonl.gz"):
    for d in lines(path):
        if d.get("gap") is not None:
            games.setdefault(d["n"], d)
ns = sorted(games)
gap = {n: games[n]["gap"] for n in ns}
to_opt = {n: games[n]["upper"] - games[n]["opt"] for n in ns if games[n].get("opt") is not None}


def rolling(values):
    keys = sorted(values)
    out = []
    for i, n in enumerate(keys):
        if i % 10:
            continue
        window = [values[k] for k in keys[max(0, i - HALF):i + HALF + 1]]
        out.append((n, st.median(window)))
    return out


W, H = 1100, 600
L, R, T, B = 80, 30, 92, 60
X1, Y1 = 8200, 10000


def x(n):
    return L + n / X1 * (W - L - R)


def y(v):
    return T + (Y1 - min(v, Y1)) / Y1 * (H - T - B)


ink, muted, grid = "#1f2328", "#59636e", "#d1d9e0"
blue, orange, shade = "#2f6fdb", "#c4622d", "#eef1f4"
parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" font-family="Helvetica, Arial, sans-serif">',
         f'<rect width="{W}" height="{H}" fill="#ffffff"/>',
         f'<text x="{L}" y="30" font-size="18" fill="{ink}" font-weight="600">The gap between the quick bounds, '
         f'games 2 to 8191</text>',
         f'<text x="{L}" y="52" font-size="13" fill="{muted}">Quick upper bound (GF(2) at the root) less quick lower '
         f'bound (the heuristic start game), each game on its own, no chain.</text>',
         f'<text x="{L}" y="70" font-size="13" fill="{muted}">A larger gap means a more expensive search.  Dots: one '
         f'game each; lines: medians of 101 games.</text>']
# The quiet stretches of the chain, and the next dips of the map.
for lo, hi, label in ((1600, 2099, "quiet"), (2600, 2999, "quiet"), (3600, 4199, "next dip?"),
                      (4600, 4799, "dip?")):
    parts.append(f'<rect x="{x(lo):.1f}" y="{T}" width="{x(hi) - x(lo):.1f}" height="{H - T - B}" fill="{shade}"/>')
    parts.append(f'<text x="{(x(lo) + x(hi)) / 2:.1f}" y="{T + 14}" font-size="11" fill="{muted}" '
                 f'text-anchor="middle">{label}</text>')
for v in range(0, Y1 + 1, 2000):
    yy = y(v)
    parts.append(f'<line x1="{L}" x2="{W - R}" y1="{yy:.1f}" y2="{yy:.1f}" stroke="{grid}" stroke-width="1"/>')
    parts.append(f'<text x="{L - 8}" y="{yy + 4:.1f}" font-size="12" fill="{muted}" text-anchor="end">{v:,}</text>')
for n in range(0, X1 + 1, 500):
    xx = x(n)
    parts.append(f'<line x1="{xx:.1f}" x2="{xx:.1f}" y1="{H - B}" y2="{H - B + 5}" stroke="{muted}"/>')
    if n % 1000 == 0:
        parts.append(f'<text x="{xx:.1f}" y="{H - B + 20}" font-size="12" fill="{muted}" '
                     f'text-anchor="middle">{n}</text>')
parts.append(f'<line x1="{L}" x2="{W - R}" y1="{H - B}" y2="{H - B}" stroke="{muted}"/>')
parts.append(f'<text x="{(L + W - R) / 2}" y="{H - 15}" font-size="13" fill="{ink}" text-anchor="middle">n</text>')
parts.append(f'<text x="22" y="{(T + H - B) / 2}" font-size="13" fill="{ink}" text-anchor="middle" '
             f'transform="rotate(-90 22 {(T + H - B) / 2})">gap (points of score)</text>')
for n in ns:
    parts.append(f'<circle cx="{x(n):.1f}" cy="{y(gap[n]):.1f}" r="1.3" fill="{blue}" fill-opacity="0.22"/>')
# Where the proven chain ends.
xc = x(3440)
parts.append(f'<line x1="{xc:.1f}" x2="{xc:.1f}" y1="{T + 22}" y2="{H - B}" stroke="{ink}" stroke-width="1" '
             f'stroke-dasharray="4 3"/>')
parts.append(f'<text x="{xc + 5:.1f}" y="{T + 34}" font-size="11" fill="{ink}">proven chain ends, 3440</text>')
for series, color in ((rolling(gap), blue), (rolling(to_opt), orange)):
    pts = " ".join(f"{x(n):.1f},{y(v):.1f}" for n, v in series)
    parts.append(f'<polyline points="{pts}" fill="none" stroke="#ffffff" stroke-width="4" stroke-linejoin="round"/>')
    parts.append(f'<polyline points="{pts}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round"/>')
lx, ly = L + 20, T + 52
parts.append(f'<rect x="{lx - 10}" y="{ly - 16}" width="390" height="48" fill="#ffffff" stroke="{grid}"/>')
parts.append(f'<line x1="{lx - 2}" x2="{lx + 12}" y1="{ly - 4}" y2="{ly - 4}" stroke="{blue}" stroke-width="2"/>')
parts.append(f'<text x="{lx + 18}" y="{ly}" font-size="12" fill="{ink}">gap between the quick bounds '
             f'({len(gap):,} games)</text>')
parts.append(f'<line x1="{lx - 2}" x2="{lx + 12}" y1="{ly + 16}" y2="{ly + 16}" stroke="{orange}" stroke-width="2"/>')
parts.append(f'<text x="{lx + 18}" y="{ly + 20}" font-size="12" fill="{ink}">quick upper bound less the proven '
             f'optimum (to 3440)</text>')
parts.append("</svg>")
open(OUT, "w").write("\n".join(parts))
print(len(gap), "games;", len(to_opt), "with a known optimum;", sum(v > Y1 for v in gap.values()), "above", Y1)
