#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Games 3899..5000: proven, open, and how much searching remains.  where_chart.py LEDGER WALL_RESULTS OUT.html

LEDGER is the survey's ledger.jsonl (3900..4999); WALL_RESULTS the chain's results file (results/strict.jsonl; the
games up to its last are drawn as the chain's).  For every open game: its gap (U - L); for every distinct search left
(one a repeat class), two estimates of its remaining proof: the floor-step line (its own floor steps, fitted as the
ledger's cost.rs fits them) and the chain's model (log10 core-seconds = 1.89 + gap/553, fitted to the chain's searched
games of 3441..3899)."""
import html
import json
import math
import statistics as st
import sys

ledger_path, wall_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
MED = 0.0068
chain_end = 0
with open(wall_path) as f:
    for line in f:
        chain_end = max(chain_end, json.loads(line)["n"])
L = {}
with open(ledger_path) as f:
    for line in f:
        d = json.loads(line)
        L[d["n"]] = d
FIRST, LAST = 3899, 5000


def floor_line(e):
    root, lo = e["root"], e["lower"]["value"]
    steps = e.get("steps") or []
    pts = [(root - s["floor"], math.log10(s["nodes"])) for s in steps if s["nodes"] >= 100]
    rates = [s["nodes"] / s["seconds"] for s in steps if s["nodes"] >= 1000 and s["seconds"] > 0]
    rate = max(rates) if rates else 3000.0
    if len(pts) >= 2:
        mx, my = st.mean(p[0] for p in pts), st.mean(p[1] for p in pts)
        w = 141 ** 2 / 2
        b = (sum((p[0] - mx) * (p[1] - my) for p in pts) + w * MED) / (sum((p[0] - mx) ** 2 for p in pts) + w)
        a = my - b * mx
    elif steps:
        x0, y0 = max((root - s["floor"], math.log10(max(s["nodes"], 1))) for s in steps)
        b, a = MED, y0 - MED * x0
    else:
        return None
    return 10 ** (a + b * (root - lo)) / rate / 3600


status, gaps, searches = {}, {}, []
seen = set()
for n in range(FIRST, LAST + 1):
    if n <= chain_end:
        status[n] = "chain"
        continue
    e = L.get(n)
    if e is None:
        status[n] = "none"
        continue
    lo, up = e["lower"]["value"], e["upper"]["value"]
    if lo == up:
        status[n] = "ledger"
        continue
    status[n] = "open"
    gaps[n] = up - lo
    c = e.get("class") or n
    if c in seen or e.get("root") is None:
        continue
    seen.add(c)
    fl = floor_line(e) if e.get("steps") else None
    model = 10 ** (1.89 + (e["root"] - lo) / 553) / 3600
    searches.append((n, fl, model, e.get("search_s", 0) / 3600))

W, H, Lm, R = 1000, 520, 70, 20
x = lambda n: Lm + (n - FIRST) / (LAST + 1 - FIRST) * (W - Lm - R)
bw = x(FIRST + 1) - x(FIRST)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="games {FIRST} to {LAST}">']
# strip: status of every game
rows_y = {"chain": 22, "ledger": 34}
for n in range(FIRST, LAST + 1):
    if status[n] in rows_y:
        cls = "chainc" if status[n] == "chain" else "s1"
        svg.append(f'<rect class="{cls}" x="{x(n):.2f}" y="{rows_y[status[n]]}" width="{bw:.2f}" height="10">'
                   f'<title>{n}: proven by the {status[n]}</title></rect>')
svg.append(f'<text class="lab" x="{Lm - 6}" y="{rows_y["chain"] + 9}" text-anchor="end">chain</text>')
svg.append(f'<text class="lab" x="{Lm - 6}" y="{rows_y["ledger"] + 9}" text-anchor="end">ledger</text>')
svg.append(f'<text class="lab" x="{Lm}" y="14">proven games, by who proved them '
           f'(a gap in both rows is an open game)</text>')
# panel A: gap of the open games
top_a, bot_a = 70, 230
ga = lambda v: bot_a - min(v, 3000) / 3000 * (bot_a - top_a)
for v in (0, 1000, 2000, 3000):
    svg.append(f'<line class="gridl" x1="{Lm}" x2="{W - R}" y1="{ga(v):.1f}" y2="{ga(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{Lm - 6}" y="{ga(v) + 4:.1f}" text-anchor="end">{v:,}</text>')
svg.append(f'<text class="lab" x="{Lm}" y="{top_a - 8}">open games: upper bound minus best game (points)</text>')
for n, g in gaps.items():
    svg.append(f'<rect class="s1" x="{x(n):.2f}" y="{ga(g):.1f}" width="{bw:.2f}" height="{bot_a - ga(g):.1f}">'
               f'<title>{n}: gap {g:,}</title></rect>')
# panel B: the two estimates of each remaining search, one-thread hours, log scale
top_b, bot_b = 280, 480
lo_h, hi_h = 1e-3, 1e8
gb = lambda h: bot_b - ((math.log10(min(max(h, lo_h), hi_h)) - math.log10(lo_h))
                        / (math.log10(hi_h) - math.log10(lo_h)) * (bot_b - top_b))
for p in range(-3, 9, 2):
    v = 10 ** p
    label = {-3: "4 s", -1: "6 min", 1: "10 h", 3: "6 weeks", 5: "11 years", 7: "1,100 years"}[p]
    svg.append(f'<line class="gridl" x1="{Lm}" x2="{W - R}" y1="{gb(v):.1f}" y2="{gb(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{Lm - 6}" y="{gb(v) + 4:.1f}" text-anchor="end">{label}</text>')
svg.append(f'<text class="lab" x="{Lm}" y="{top_b - 8}">each search left (one a repeat class): its remaining '
           f'proof on one thread, log scale</text>')
for n, fl, model, spent in searches:
    tip = (f"{n}: chain model {model:.3g} h" + (f", floor-step line {fl:.3g} h" if fl else ", no floor steps yet")
           + f"; already searched {spent:.1f} h (one thread)")
    if fl:
        svg.append(f'<circle class="s1" cx="{x(n):.1f}" cy="{gb(fl):.1f}" r="2.6">'
                   f'<title>{html.escape(tip)}</title></circle>')
    shape = "s2" if fl else "hollow"
    svg.append(f'<rect class="{shape}" x="{x(n) - 2.6:.1f}" y="{gb(model) - 2.6:.1f}" width="5.2" height="5.2"><title>'
               f'{html.escape(tip)}</title></rect>')
for n in range(3900, LAST + 1, 100):
    svg.append(f'<text class="lab" x="{x(n):.1f}" y="{H - 18}" text-anchor="middle">{n}</text>')
svg.append("</svg>")

rows, totals = [], [0.0, 0.0]
for b in range(3900, 5000, 100):
    ss = [s for s in searches if b <= s[0] < b + 100]
    og = [g for n, g in gaps.items() if b <= n < b + 100]
    pr = sum(1 for n in range(b, b + 100) if status.get(n) in ("chain", "ledger"))
    fl = sum(s[1] for s in ss if s[1])
    md = sum(s[2] for s in ss)
    totals[0] += fl
    totals[1] += md
    rows.append(f"<tr><td>{b}–{b + 99}</td><td>{pr}</td><td>{len(og)}</td><td>{len(ss)}</td>"
                f"<td>{st.median(og) if og else 0:,.0f}</td><td>{fl:,.0f}</td><td>{md:,.0f}</td></tr>")
cheap = sorted(searches, key=lambda s: s[2])
cum = []
for k in (50, 100, 150, 200, 250, len(cheap)):
    if k <= len(cheap):
        cum.append(f"<tr><td>the cheapest {k}</td><td>{sum(s[2] for s in cheap[:k]):,.0f}</td></tr>")
n_proven = sum(1 for v in status.values() if v in ("chain", "ledger"))
page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Games 3899 to 5000</title>
<style>
:root {{ --bg:#fcfcfb; --ink:#0b0b0b; --muted:#52514e; --grid:#e3e2de; --chain:#9a9890; --s1:#2a78d6; --s2:#eb6834;
  --s3:#1baf7a; }}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7;
  --grid:#353533; --chain:#7d7b74; --s1:#3987e5; --s2:#d95926; --s3:#199e70; }} }}
:root[data-theme="dark"] {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7; --grid:#353533; --chain:#7d7b74; --s1:#3987e5;
  --s2:#d95926; --s3:#199e70; }}
body {{ background:var(--bg); color:var(--ink); font:15px/1.45 -apple-system, Helvetica, Arial, sans-serif;
  margin:0 auto; max-width:1040px; padding:24px 16px 48px; }}
h1 {{ font-size:22px; margin:0 0 6px; }} h2 {{ font-size:17px; margin:28px 0 4px; }} p {{ color:var(--muted); margin:4px 0 10px; }}
svg {{ width:100%; height:auto; display:block; }}
.lab {{ fill:var(--muted); font-size:12px; }} .gridl {{ stroke:var(--grid); }} .grid {{ fill:var(--grid); }}
.chainc {{ fill:var(--chain); }} .hollow {{ fill:none; stroke:var(--s2); stroke-width:1.5; }} .s1 {{ fill:var(--s1); }} .s2 {{ fill:var(--s2); }} .s3 {{ fill:var(--s3); }}
.keys {{ display:flex; gap:16px; flex-wrap:wrap; font-size:13px; color:var(--muted); margin:6px 0; }}
.sw {{ display:inline-block; width:10px; height:10px; border-radius:2px; margin-right:6px; vertical-align:-1px; }}
table {{ border-collapse:collapse; font-size:14px; margin:8px 0; width:100%; }}
td, th {{ border-bottom:1px solid var(--grid); padding:5px 8px; text-align:right; }}
td:first-child, th:first-child {{ text-align:left; }} th {{ color:var(--muted); font-weight:600; }}
.wrap {{ overflow-x:auto; }}
</style></head><body>
<h1>Games {FIRST}–{LAST}: where we stand</h1>
<p>{n_proven} of {LAST - FIRST + 1} games proven: the chain to {chain_end}, the ledger the rest; {len(gaps)} open, which
come down to {len(searches)} distinct searches (games posing the same search are proven together, primes follow the
game below).  Hover any mark for its numbers.</p>
<div class="keys"><span><i class="sw" style="background:var(--chain)"></i>proven by the chain (top row)</span><span><i
class="sw" style="background:var(--s1)"></i>proven by the ledger (second row)</span></div>
{"".join(svg)}
<div class="keys"><span><i class="sw" style="background:var(--s1);border-radius:50%"></i>circle: forecast from the game's own
floor steps</span><span><i class="sw" style="background:var(--s2)"></i>square: forecast from the chain's cost against gap
(3441–3899)</span><span><i class="sw" style="border:1.5px solid var(--s2)"></i>hollow square: the same, for a game with
no floor steps yet (most were already searched for hours by stages 1 and 2)</span></div>
<p>Two estimates of each remaining proof, both rough: the floor-step lines extrapolate a game's first steps over a long
distance (on 3900 the line said about 4 billion nodes; the chain proved it with 0.44 billion, with the game below proven),
and the chain's model is a loose fit (r = 0.51).  Read them as orders of magnitude.</p>
<h2>By stretch</h2>
<div class="wrap"><table><tr><th>games</th><th>proven</th><th>open</th><th>searches left</th><th>median gap</th>
<th>one-thread hours, floor lines</th><th>one-thread hours, chain model</th></tr>{"".join(rows)}
<tr><td><b>total</b></td><td></td><td></td><td></td><td></td><td><b>{totals[0]:,.0f}</b></td><td><b>{totals[1]:,.0f}</b></td></tr>
</table></div>
<h2>The cheapest searches first (chain model)</h2>
<div class="wrap"><table><tr><th>searches</th><th>one-thread hours</th></tr>{"".join(cum)}</table></div>
</body></html>
"""
with open(out_path, "w") as f:
    f.write(page)
print(n_proven, len(gaps), len(searches), [round(t) for t in totals])
