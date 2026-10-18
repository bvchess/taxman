#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The prep run against the first full ledger run: compare_page.py OUT.html

Three runs of 2176..3440 with the truth hidden: the first full run (64 workers, no climbs, no finder, nothing given),
and the prep run's two ledgers side by side (32 workers each, climbs both ways, given 1..2175), A with the finder and B
without.  Games proven against core-hours, then the climbs and the finder, then stretch by stretch.  It reads the three
runs' events.jsonl.gz, which are kept beside this script and in ../cloud-20261017/."""
import gzip
import html
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUNS = [("first run: no climbs, no finder", HERE.parent / "cloud-20261017/run", "chain"),
        ("prep B: climbs", HERE / "b/run", "s2"),
        ("prep A: climbs and the finder", HERE / "a/run", "s1")]
FIRST, LAST = 2176, 3440


def esc(s):
    return html.escape(str(s), quote=True)


def events(run):
    with gzip.open(run / "events.jsonl.gz", "rt") as f:
        return [json.loads(x) for x in f if x.strip()]


curves, climbs, finder = [], [], []
for label, run, cls in RUNS:
    ev = events(run)
    seen, points = set(), []
    for e in ev:
        if e["action"] == "proven" and FIRST <= e["n"] <= LAST and e["n"] not in seen:
            seen.add(e["n"])
            points.append(e["cpu"] / 3600)
    end = max(e["cpu"] for e in ev) / 3600
    curves.append((label, cls, sorted(points), end))
    for way in ("up", "down"):
        c = [e for e in ev if e["action"] == "search" and e.get("job") == "climb" and e.get("way") == way]
        if c:
            proved = [e for e in c if e.get("outcome") == "proved"]
            climbs.append((label, way, len(c), len(proved), sum(e["seconds"] for e in c) / 3600))
    runs_ = [e for e in ev if e["action"] in ("init", "search") and (e.get("lns_s") or 0) > 0]
    if runs_:
        better = sum(1 for e in runs_ if (e.get("lns_found") or 0) > (e.get("lns_from") or 0))
        finder.append((label, len(runs_), better, sum(e["lns_s"] for e in runs_) / 3600))

W, H, L, R, T, B = 900, 400, 70, 30, 20, 50
XMAX, YMAX = 48, LAST - FIRST + 1


def x(v):
    return L + v / XMAX * (W - L - R)


def y(v):
    return H - B - v / YMAX * (H - T - B)


svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="games proven against core-hours, three runs">']
for v in range(0, YMAX + 1, 250):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{y(v):.1f}" y2="{y(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end">{v:,}</text>')
for v in range(0, XMAX + 1, 6):
    svg.append(f'<line class="tick" x1="{x(v):.1f}" x2="{x(v):.1f}" y1="{H - B}" y2="{H - B + 4}"/>')
    svg.append(f'<text class="lab" x="{x(v):.1f}" y="{H - B + 17}" text-anchor="middle">{v}</text>')
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{H - 10}" text-anchor="middle">core-hours</text>')
svg.append(f'<text class="axis" x="16" y="{(T + H - B) / 2}" text-anchor="middle" '
           f'transform="rotate(-90 16 {(T + H - B) / 2})">games proven (of {YMAX:,})</text>')
for label, cls, pts, end in curves:
    path = [(0, 0)] + [(h, i + 1) for i, h in enumerate(pts)] + [(end, len(pts))]
    svg.append(f'<polyline class="line {cls}l" points="' + " ".join(f"{x(h):.1f},{y(k):.1f}" for h, k in path) + '"/>')
    svg.append(f'<text class="lab strong" x="{x(end) + 5:.1f}" y="{y(len(pts)) + 4:.1f}">{len(pts):,}</text>')
    for mark in (6, 12, 24, 48):
        if mark <= end + 1e-9:
            k = sum(1 for h in pts if h <= mark)
            svg.append(f'<circle class="hit" cx="{x(mark):.1f}" cy="{y(k):.1f}" r="7">'
                       f'<title>{esc(label)}: {k:,} proven by {mark} core-hours</title></circle>')
svg.append("</svg>")
chart = "\n".join(svg)


def at(pts, h):
    return sum(1 for v in pts if v <= h)


rows = "".join(f"<tr><td>{esc(label)}</td>" + "".join(f"<td>{at(pts, h):,}</td>" if h <= end + 1e-9 else "<td>–</td>"
                                                     for h in (3, 6, 12, 24, 48)) + "</tr>"
               for label, cls, pts, end in curves)
climb_rows = "".join(f"<tr><td>{esc(label)}</td><td>{way}</td><td>{n}</td><td>{p} ({100 * p / n:.0f}%)</td>"
                     f"<td>{h:.1f}</td><td>{3600 * h / max(p, 1):.0f}</td></tr>" for label, way, n, p, h in climbs)
finder_rows = "".join(f"<tr><td>{esc(label)}</td><td>{n:,}</td><td>{b:,}</td><td>{h:.2f}</td></tr>"
                      for label, n, b, h in finder)
keys = "".join(f'<span class="key"><i class="sw {cls}"></i>{esc(label)}</span>' for label, cls, _, _ in curves)

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ledger prep run</title>
<style>
:root {{ --bg:#fcfcfb; --ink:#0b0b0b; --muted:#52514e; --grid:#e3e2de; --chain:#9a9890; --s1:#2a78d6; --s2:#eb6834; }}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7;
  --grid:#353533; --chain:#7d7b74; --s1:#3987e5; --s2:#d95926; }} }}
:root[data-theme="dark"] {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7; --grid:#353533; --chain:#7d7b74;
  --s1:#3987e5; --s2:#d95926; }}
body {{ background:var(--bg); color:var(--ink); font:15px/1.45 -apple-system, Helvetica, Arial, sans-serif;
  margin:0 auto; max-width:940px; padding:24px 16px 48px; }}
h1 {{ font-size:22px; margin:0 0 4px; }} h2 {{ font-size:17px; margin:30px 0 4px; }}
p {{ color:var(--muted); margin:4px 0 10px; }}
svg {{ width:100%; height:auto; display:block; }}
.grid {{ stroke:var(--grid); }} .tick {{ stroke:var(--muted); }} .lab {{ fill:var(--muted); font-size:12px; }}
.axis {{ fill:var(--ink); font-size:13px; }} .strong {{ fill:var(--ink); }} .hit {{ fill:transparent; }}
.line {{ fill:none; stroke-width:2; stroke-linejoin:round; }} .chainl {{ stroke:var(--chain); }} .s1l {{ stroke:var(--s1); }}
.s2l {{ stroke:var(--s2); }}
.keys {{ display:flex; gap:16px; flex-wrap:wrap; font-size:13px; color:var(--muted); margin:6px 0; }}
.sw {{ display:inline-block; width:10px; height:10px; border-radius:2px; margin-right:6px; vertical-align:-1px; }}
.sw.chain {{ background:var(--chain); }} .sw.s1 {{ background:var(--s1); }} .sw.s2 {{ background:var(--s2); }}
table {{ border-collapse:collapse; font-size:14px; margin:8px 0; width:100%; }}
td, th {{ border-bottom:1px solid var(--grid); padding:5px 8px; text-align:right; }}
td:first-child, th:first-child {{ text-align:left; }} th {{ color:var(--muted); font-weight:600; }}
.wrap {{ overflow-x:auto; }}
</style></head><body>
<h1>The ledger's prep run on 2176–3440</h1>
<p>Three runs with the answers hidden, every proven game at the chain's optimum and no bound on the wrong side in any.
The prep run's two ledgers shared one machine (32 workers and 24 core-hours each) and were given the proven games up to
2175, as a run past 3440 would be given the chain's.</p>
<h2>Games proven against core-hours</h2>
<div class="keys">{keys}</div>
{chart}
<div class="wrap"><table><tr><th>games proven by</th><th>3</th><th>6</th><th>12</th><th>24</th><th>48 core-hours</th></tr>
{rows}</table></div>
<h2>The climbs</h2>
<p>A climb is a search of a game next to a proven one, from that game, for at most 60 seconds of one thread.</p>
<div class="wrap"><table><tr><th>run</th><th>way</th><th>climbs</th><th>proved their game</th><th>core-hours</th>
<th>CPU seconds a proof</th></tr>{climb_rows}</table></div>
<h2>The finder</h2>
<div class="wrap"><table><tr><th>run</th><th>runs</th><th>found a better game</th><th>core-hours</th></tr>{finder_rows}
</table></div>
</body></html>
"""
with open(sys.argv[1], "w") as f:
    f.write(page)
print([(label, len(pts)) for label, _, pts, _ in curves], climbs, finder)
