#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Every game of the ledger past 4300: where its bounds started and where they are now.  past_chart.py LEDGER OUT.html

The optimum is not known here, so the zero line is the best game the ledger holds (the optimum, for a proven game).
Up: how far the upper bound is above it, at the start (gray, the root's GF(2) bound) and now (blue, open games only).
Down: how far the start game (the standalone start incumbent) was below it.  The quick bounds are the landscape's map
(solver/notebook/search/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/map.jsonl.gz), the same the
ledger's maps compute."""
import gzip
import html
import json
import statistics as st
import sys
from pathlib import Path

LANDSCAPE = "the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10"
MAP = Path(__file__).resolve().parents[2] / LANDSCAPE / "map.jsonl.gz"
ledger_path, out_path = sys.argv[1], sys.argv[2]

quick = {}
with gzip.open(MAP, "rt") as f:
    for line in f:
        d = json.loads(line)
        if d.get("upper") is not None:
            quick[d["n"]] = (d["lower"], d["upper"])
now = {}
with open(ledger_path) as f:
    for line in f:
        d = json.loads(line)
        if d.get("lower") and d.get("upper"):
            now[d["n"]] = (d["lower"]["value"], d["upper"]["value"])
ns = sorted(n for n in now if n in quick)
first, last = ns[0], ns[-1]
rows = []
for n in ns:
    (ql, qu), (lo, up) = quick[n], now[n]
    rows.append((n, qu - lo, lo - ql, up - lo, lo == up))
proven = [r for r in rows if r[4]]
open_ = [r for r in rows if not r[4]]

W, H, L, R, T, B = 1000, 470, 70, 20, 40, 64
LIM = 2500
mid = T + (H - T - B) * 0.62
up_scale = (mid - T) / LIM
down_lim = LIM * (H - B - mid) / (mid - T)


def x(n):
    return L + (n - first) / (last + 1 - first) * (W - L - R)


def yu(v):
    return mid - min(v, LIM) * up_scale


def yd(v):
    return mid + min(v, down_lim) * up_scale


w = x(first + 1) - x(first)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="every game past {first}: its bounds at the start and now">']
for v in range(0, LIM + 1, 500):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{yu(v):.1f}" y2="{yu(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{yu(v) + 4:.1f}" text-anchor="end">{v:,}</text>')
for v in range(500, int(down_lim) + 1, 500):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{yd(v):.1f}" y2="{yd(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{yd(v) + 4:.1f}" text-anchor="end">{v:,}</text>')
svg.append(f'<text class="axis" x="{L + 4}" y="{T - 16}">↑ upper bound, above the best game we hold</text>')
for n, qa, qb, ea, done in rows:
    state = "proven: the best game is the optimum" if done else f"open: upper bound {ea:,} above the best game"
    tip = f"{n}, {state}; at the start the upper bound was {qa:,} above it and the start game {qb:,} below it"
    bars = (f'<rect class="startbar" x="{x(n):.2f}" y="{yu(qa):.1f}" width="{w:.2f}" height="{mid - yu(qa):.1f}"/>'
            f'<rect class="startbar" x="{x(n):.2f}" y="{mid:.1f}" width="{w:.2f}" height="{yd(qb) - mid:.1f}"/>')
    if not done:
        bars += f'<rect class="s1" x="{x(n):.2f}" y="{yu(ea):.1f}" width="{w:.2f}" height="{mid - yu(ea):.1f}"/>'
    svg.append(f"<g><title>{html.escape(tip)}</title>{bars}</g>")
svg.append(f'<line class="tick" x1="{L}" x2="{W - R}" y1="{mid:.1f}" y2="{mid:.1f}"/>')
svg.append(f'<text class="lab" x="{W - R}" y="{mid - 5:.1f}" text-anchor="end">the best game we hold</text>')
for n in range((first // 100 + 1) * 100, last + 1, 100):
    svg.append(f'<line class="tick" x1="{x(n):.1f}" x2="{x(n):.1f}" y1="{H - B}" y2="{H - B + 4}"/>')
    svg.append(f'<text class="lab" x="{x(n):.1f}" y="{H - B + 17}" text-anchor="middle">{n}</text>')
svg.append(f'<text class="axis" x="{L + 4}" y="{H - 10}">↓ start game, below the best game we hold</text>')
svg.append("</svg>")

med = lambda v: f"{st.median(v):,.0f}" if v else "–"
note = (f"{len(proven)} of {len(rows)} games are proven (no blue bar).  The open games' upper bounds came down from a "
        f"median {med([r[1] for r in open_])} to {med([r[3] for r in open_])} above the best game we hold; the start "
        f"games were "
        f"a median {med([r[2] for r in rows])} below it, so the finder and the searches did most of their work on the "
        f"lower side.  At the start the proven games' quick bounds were a median {med([r[1] + r[2] for r in proven])} "
        f"apart, the open games' {med([r[1] + r[2] for r in open_])}.")

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ledger past {first}</title>
<style>
:root {{ --bg:#fcfcfb; --ink:#0b0b0b; --muted:#52514e; --grid:#e3e2de; --chain:#9a9890; --s1:#2a78d6; }}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7;
  --grid:#353533; --chain:#7d7b74; --s1:#3987e5; }} }}
:root[data-theme="dark"] {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7; --grid:#353533; --chain:#7d7b74; --s1:#3987e5; }}
body {{ background:var(--bg); color:var(--ink); font:15px/1.45 -apple-system, Helvetica, Arial, sans-serif;
  margin:0 auto; max-width:1040px; padding:24px 16px 48px; }}
h1 {{ font-size:22px; margin:0 0 6px; }} p {{ color:var(--muted); margin:4px 0 10px; }}
svg {{ width:100%; height:auto; display:block; }}
.grid {{ stroke:var(--grid); }} .tick {{ stroke:var(--muted); }} .lab {{ fill:var(--muted); font-size:12px; }}
.axis {{ fill:var(--ink); font-size:13px; }} .s1 {{ fill:var(--s1); }} .startbar {{ fill:var(--chain); opacity:0.55; }}
.keys {{ display:flex; gap:16px; flex-wrap:wrap; font-size:13px; color:var(--muted); margin:6px 0; }}
.sw {{ display:inline-block; width:10px; height:10px; border-radius:2px; margin-right:6px; vertical-align:-1px; }}
.sw.s1 {{ background:var(--s1); }} .sw.start {{ background:var(--chain); opacity:0.55; }}
</style></head><body>
<h1>Every game of {first}–{last}: where its bounds started, and where they are now</h1>
<p>The ledger past 4300, stages 1 and 2.  The optimum is not known here, so the zero line is the best game the ledger
holds, which is the optimum for a proven game.  Up: how far the upper bound is above it; down: how far the start game
was below it.  Hover a bar for its numbers.  {html.escape(note)}</p>
<div class="keys"><span class="key"><i class="sw start"></i>at the start (the quick bounds)</span><span class="key"><i
class="sw s1"></i>now (open games only)</span></div>
{"".join(svg)}
</body></html>
"""
with open(out_path, "w") as f:
    f.write(page)
print(note)
