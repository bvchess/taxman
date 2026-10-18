#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The ledger's experiment as one HTML page: report_page.py RUN_DIR OUT.html

RUN_DIR holds a ledger run's events.jsonl and ledger.jsonl (or their gzipped copies).  The truth is the committed chain
(results/strict.jsonl), the chain's cost per game is the run to the wall's search time (the current build on the same
machine type: solver/notebook/the-run-to-the-wall-2026-10/reproof.jsonl.gz), and the quick bounds of every game
are the landscape's (solver/notebook/the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/
cal.jsonl)."""
import gzip
import html
import json
import math
import sys
from pathlib import Path

NOTEBOOK = Path(__file__).resolve().parents[2]
REPO = NOTEBOOK.parents[1]  # the repository
WALL_RUN = NOTEBOOK / "the-run-to-the-wall-2026-10/reproof.jsonl.gz"
CHAIN_CORE_HOURS = 280.0  # the run to the wall's 2176..3440: about 4.4 hours of 64 threads

run_dir, out_path = sys.argv[1], sys.argv[2]


def jsonl(path):
    """The JSON lines of `path`, or of `path`.gz if only that is there."""
    path = Path(path)
    if not path.exists():
        path = Path(f"{path}.gz")
    with (gzip.open(path, "rt") if path.suffix == ".gz" else open(path)) as f:
        return [json.loads(x) for x in f if x.strip()]


events = jsonl(f"{run_dir}/events.jsonl")
ledger = {d["n"]: d for d in jsonl(f"{run_dir}/ledger.jsonl")}
start = next(e for e in events if e["action"] == "start")
first, last = start["first"], start["last"]
ns = list(range(first, last + 1))
truth = {d["n"]: d for d in jsonl(f"{REPO}/results/strict.jsonl") if first <= d["n"] <= last}
wall = {d["n"]: d for d in jsonl(WALL_RUN) if first <= d["n"] <= last}
LANDSCAPE = NOTEBOOK / "the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10"
quick = {d["n"]: d for d in jsonl(LANDSCAPE / "cal.jsonl") if first <= d["n"] <= last}

# What proved each game: the rule of the upper bound that met the lower one.
METHODS = [("search", "its own search"), ("repeat", "the same search as another game"),
           ("prime", "the prime formula"), ("ceiling", "a neighbour's bound")]
METHOD_OF = {"exhausted": "search", "dual": "search", "repeat": "repeat", "prime": "prime", "ceiling": "ceiling",
             "monotone": "ceiling"}
CHAIN_METHOD = {"exhausted": "search", "same_search": "repeat", "prime_formula": "prime", "cap": "ceiling",
                "trivial": "search"}
proven = {}
for e in events:
    if e["action"] == "proven" and e["n"] not in proven:
        proven[e["n"]] = (e["cpu"] / 3600, METHOD_OF.get(e["upper_rule"], "search"), e["value"], e["upper_rule"])
cpu_end = max(e["cpu"] for e in events) / 3600


def bounds(n):
    d = ledger.get(n, {})
    lo, up = d.get("lower") or {}, d.get("upper") or {}
    return lo.get("value"), up.get("value")


violations = []
for n in ns:
    opt = truth[n]["score"]
    lo, up = bounds(n)
    if (lo is not None and lo > opt) or (up is not None and up < opt):
        violations.append(n)
    if n in proven and proven[n][2] != opt:
        violations.append(n)
exact = sum(proven[n][2] == truth[n]["score"] for n in proven)

# The chain's cost per game: its share of the stretch's core-hours, in proportion to its search time.
secs = {n: (wall[n].get("seconds") or {}).get("search", 0) or 0 for n in ns}
scale = CHAIN_CORE_HOURS / sum(secs.values())
chain_curve, acc = [], 0.0
for i, n in enumerate(ns):
    acc += secs[n] * scale
    chain_curve.append((acc, i + 1))
ledger_curve = sorted(v[0] for v in proven.values())
chain_at_end = max((k for h, k in chain_curve if h <= max(e["cpu"] for e in events) / 3600), default=0)

# The same games both ways: what each spent on the games the ledger proved, and on those it left open.
ledger_spent = {}
for e in events:
    if e["action"] in ("search", "init"):
        ledger_spent[e["n"]] = ledger_spent.get(e["n"], 0.0) + e["seconds"] / 3600
open_ns = [n for n in ns if n not in proven]
groups = [(f"the {len(proven):,} games the ledger proved", [n for n in ns if n in proven]),
          (f"the {len(open_ns):,} it left open", open_ns)]
same_games = [(label, sum(secs[n] for n in group) * scale, sum(ledger_spent.get(n, 0.0) for n in group))
              for label, group in groups]

# --- drawing -------------------------------------------------------------------------------------------------------
COLORS = {"search": "s1", "repeat": "s2", "prime": "s3", "ceiling": "s4"}
parts = []


def esc(s):
    return html.escape(str(s), quote=True)


def log_scale(lo, hi, a, b):
    la, lb = math.log10(lo), math.log10(hi)
    return lambda v: a + (math.log10(min(max(v, lo), hi)) - la) / (lb - la) * (b - a)


def lin_scale(lo, hi, a, b):
    return lambda v: a + (v - lo) / (hi - lo) * (b - a)


def axis_x_n(parts, x, y0, W):
    for n in range(2200, last + 1, 100):
        parts.append(f'<line class="tick" x1="{x(n):.1f}" x2="{x(n):.1f}" y1="{y0}" y2="{y0 + 4}"/>')
        if n % 200 == 0:
            parts.append(f'<text class="lab" x="{x(n):.1f}" y="{y0 + 17}" text-anchor="middle">{n}</text>')


def decades(parts, y, lo, hi, x0, x1, fmt):
    d = math.floor(math.log10(lo))
    while 10 ** d <= hi:
        v = 10 ** d
        if v >= lo:
            parts.append(f'<line class="grid" x1="{x0}" x2="{x1}" y1="{y(v):.1f}" y2="{y(v):.1f}"/>')
            parts.append(f'<text class="lab" x="{x0 - 6}" y="{y(v) + 4:.1f}" text-anchor="end">{fmt(v)}</text>')
        d += 1


def fmt_h(v):
    return f"{v:g}"


# 2. the curve
W, H, L, R, T, B = 900, 380, 70, 20, 20, 50
x = log_scale(0.01, 300, L, W - R)
y = lin_scale(0, len(ns), H - B, T)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="games proven against core-hours">']
for v in range(0, len(ns) + 1, 250):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{y(v):.1f}" y2="{y(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end">{v:,}</text>')
for v in (0.01, 0.1, 1, 10, 100):
    svg.append(f'<line class="tick" x1="{x(v):.1f}" x2="{x(v):.1f}" y1="{H - B}" y2="{H - B + 4}"/>')
    svg.append(f'<text class="lab" x="{x(v):.1f}" y="{H - B + 17}" text-anchor="middle">{v:g}</text>')
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{H - 10}" text-anchor="middle">core-hours (log scale)</text>')
svg.append(f'<text class="axis" x="16" y="{(T + H - B) / 2}" text-anchor="middle" '
           f'transform="rotate(-90 16 {(T + H - B) / 2})">games proven</text>')
pts = " ".join(f"{x(max(h, 0.01)):.1f},{y(k):.1f}" for h, k in chain_curve if h >= 0.01)
svg.append(f'<polyline class="line chain" points="{pts}"/>')
lpts = [(0.01, 0)] + [(h, i + 1) for i, h in enumerate(ledger_curve)] + [(cpu_end, len(ledger_curve))]
svg.append('<polyline class="line s1l" points="' + " ".join(f"{x(max(h, 0.01)):.1f},{y(k):.1f}" for h, k in lpts)
           + '"/>')
svg.append(f'<text class="lab strong" x="{x(cpu_end) + 6:.1f}" y="{y(len(ledger_curve)) + 4:.1f}">ledger: '
           f'{len(ledger_curve):,} in {cpu_end:.1f} core-hours</text>')
svg.append(f'<text class="lab" x="{x(CHAIN_CORE_HOURS) - 4:.1f}" y="{y(len(ns)) - 6:.1f}" text-anchor="end">chain: '
           f'{len(ns):,} in about {CHAIN_CORE_HOURS:.0f}</text>')
for k in (len(ns) // 4, len(ns) // 2, 3 * len(ns) // 4):
    hc = next(h for h, c in chain_curve if c >= k)
    hl = ledger_curve[k - 1] if len(ledger_curve) >= k else None
    tip = f"{k:,} games: chain {hc:.2f} core-hours" + (f", ledger {hl:.2f}" if hl else ", ledger not reached")
    svg.append(f'<circle class="hit" cx="{x(max(hc, 0.01)):.1f}" cy="{y(k):.1f}" r="7">'
               f'<title>{esc(tip)}</title></circle>')
svg.append("</svg>")
curve_svg = "\n".join(svg)

# 3. the map: the chain's cost on top, the ledger's proofs below
W, H, L, R = 900, 470, 70, 20
top = (20, 170)
bot = (215, 420)
x = lin_scale(first, last, L, W - R)
yt = log_scale(0.01, 10000, top[1], top[0])
yb = log_scale(0.01, 50, bot[1], bot[0] + 22)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="where the ledger proved games, and where the chain spent">']
decades(svg, yt, 0.01, 10000, L, W - R, lambda v: f"{v:g}")
svg.append(f'<text class="axis" x="16" y="{sum(top) / 2}" text-anchor="middle" '
           f'transform="rotate(-90 16 {sum(top) / 2})">chain s</text>')
for n in ns:
    if secs[n] > 0:
        svg.append(f'<circle class="dot chainfill" cx="{x(n):.1f}" cy="{yt(secs[n]):.1f}" r="1.8">'
                   f'<title>{n}: the chain searched {secs[n]:.2f} s on 64 threads ({esc(truth[n]["proof"])})</title>'
                   f'</circle>')
svg.append(f'<text class="lab" x="{L}" y="{top[0] - 6}">the chain: search seconds per game on 64 threads (log scale); '
           f'games with no search are left out</text>')
decades(svg, yb, 0.01, 50, L, W - R, fmt_h)
svg.append(f'<text class="axis" x="16" y="{(bot[0] + 22 + bot[1]) / 2}" text-anchor="middle" '
           f'transform="rotate(-90 16 {(bot[0] + 22 + bot[1]) / 2})">core-hours when proven</text>')
svg.append(f'<text class="lab" x="{L}" y="{bot[0] - 6}">the ledger: when each game was proven, and what proved it '
           f'(log scale); the top row is the games left open</text>')
for n in ns:
    if n in proven:
        h, m, v, rule = proven[n]
        svg.append(f'<circle class="dot {COLORS[m]}" cx="{x(n):.1f}" cy="{yb(h):.1f}" r="2.2"><title>{n}: proven at '
                   f'{h:.2f} core-hours by {esc(dict(METHODS)[m])}, score {v:,}</title></circle>')
    else:
        lo, up = bounds(n)
        gap = f"{up - lo:,}" if lo is not None and up is not None else "?"
        svg.append(f'<rect class="open" x="{x(n) - 0.6:.1f}" y="{bot[0] + 4}" width="1.4" height="12">'
                   f'<title>{n}: open, U − L = {gap}</title></rect>')
svg.append(f'<text class="lab" x="{W - R}" y="{bot[0] - 6}" text-anchor="end">open: {len(ns) - len(proven):,}</text>')
axis_x_n(svg, x, bot[1], W)
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{H - 10}" text-anchor="middle">n</text>')
svg.append("</svg>")
map_svg = "\n".join(svg)

# 4. how far each game's bounds are from the optimum, at the start (gray, the quick bounds) and at the end (blue,
# where the run left them), every game in order of n: a proven game has no blue bar.
every = []
for n in ns:
    lo, up = bounds(n)
    o, q = truth[n]["score"], quick.get(n, {})
    if lo is not None and up is not None and "upper" in q:
        every.append((n, q["upper"] - o, o - q["lower"], up - o, o - lo))
rows = [r for r in every if r[0] not in proven]
W, H, L, R, T, B = 900, 440, 70, 20, 34, 60
LIM = 2500
mid = (T + H - B) / 2
y = lin_scale(-LIM, LIM, H - B, T)
x = lin_scale(first, last + 1, L, W - R)
w = x(first + 1) - x(first)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="for every game, how far its bounds are from the optimum">']
for v in range(-2000, 2001, 1000):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{y(v):.1f}" y2="{y(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end">{abs(v):,}</text>')
svg.append(f'<text class="axis" x="{L + 4}" y="{T - 14}">↑ upper bound, above the optimum</text>')
svg.append(f'<text class="axis" x="{L + 4}" y="{H - B + 40}">↓ best game, below the optimum</text>')
for n, qa, qb, ea, eb in every:
    xx = x(n)
    state = "proven" if n in proven else f"open: upper bound {ea:,} above the optimum, best game {eb:,} below it"
    tip = f"{n}, {state}; at the start, upper bound {qa:,} above the optimum and best game {qb:,} below it"
    bars = (f'<rect class="startbar" x="{xx:.2f}" y="{y(min(qa, LIM)):.1f}" width="{w:.2f}" '
            f'height="{mid - y(min(qa, LIM)):.1f}"/>'
            f'<rect class="startbar" x="{xx:.2f}" y="{mid:.1f}" width="{w:.2f}" '
            f'height="{y(-min(qb, LIM)) - mid:.1f}"/>')
    if n not in proven:
        bars += (f'<rect class="s1" x="{xx:.2f}" y="{y(min(ea, LIM)):.1f}" width="{w:.2f}" '
                 f'height="{mid - y(min(ea, LIM)):.1f}"/>'
                 f'<rect class="s1" x="{xx:.2f}" y="{mid:.1f}" width="{w:.2f}" '
                 f'height="{y(-min(eb, LIM)) - mid:.1f}"/>')
    svg.append(f'<g><title>{esc(tip)}</title>{bars}</g>')
svg.append(f'<line class="tick" x1="{L}" x2="{W - R}" y1="{mid:.1f}" y2="{mid:.1f}"/>')
svg.append(f'<text class="lab" x="{W - R}" y="{mid - 5:.1f}" text-anchor="end">the optimum</text>')
for n in range(2200, last + 1, 200):
    svg.append(f'<line class="tick" x1="{x(n):.1f}" x2="{x(n):.1f}" y1="{H - B}" y2="{H - B + 4}"/>')
    svg.append(f'<text class="lab" x="{x(n):.1f}" y="{H - B + 17}" text-anchor="middle">{n}</text>')
svg.append("</svg>")
left_svg = "\n".join(svg)
proven_rows = [r for r in every if r[0] in proven]


def median(v):
    v = sorted(v)
    return v[len(v) // 2] if v else 0


start_gap = [qa + qb for _, qa, qb, _, _ in rows]
end_gap = [ea + eb for _, _, _, ea, eb in rows]
closed = [1 - e / s_ if s_ else 0 for s_, e in zip(start_gap, end_gap)]
up_frac = [1 - ea / qa if qa else 1 for _, qa, _, ea, _ in rows]
lo_frac = [1 - eb / qb if qb else 1 for _, _, qb, _, eb in rows]
up_moved = [qa - ea for _, qa, _, ea, _ in rows]
lo_moved = [qb - eb for _, _, qb, _, eb in rows]
left_note = (f"For the {len(rows):,} open games the gap between the bounds shrank from a median "
             f"{median(start_gap):,} to {median(end_gap):,}: a median {100 * median(closed):.0f}% of each game's gap "
             f"closed.  Both bounds moved by similar amounts (medians {median(lo_moved):,} for the best game, {median(up_moved):,} for the upper "
             f"bound), but the best game started much closer: it is now a median {median([r[4] for r in rows]):,} "
             f"below the optimum ({100 * median(lo_frac):.0f}% closer; the optimum itself in "
             f"{sum(r[4] == 0 for r in rows)} games), while the upper bound is still "
             f"{median([r[3] for r in rows]):,} above it ({100 * median(up_frac):.0f}% closer).")

start_note = (f"At the start the proven games had a median gap of "
              f"{median([qa + qb for _, qa, qb, _, _ in proven_rows]):,} "
              f"between their quick bounds, the open ones {median(start_gap):,}.")

# The share of each open game's gap that the run closed, as a histogram.
W, H, L, R, T, B = 900, 230, 70, 20, 20, 45
bins = [0] * 10
for c in closed:
    bins[min(max(int(c * 10), 0), 9)] += 1
x = lin_scale(0, 10, L, W - R)
y = lin_scale(0, max(bins) * 1.2, H - B, T)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="share of each open game\'s gap closed">']
for v in range(0, int(max(bins) * 1.1) + 1, 25):
    svg.append(f'<line class="grid" x1="{L}" x2="{W - R}" y1="{y(v):.1f}" y2="{y(v):.1f}"/>')
    svg.append(f'<text class="lab" x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end">{v}</text>')
for i, k in enumerate(bins):
    svg.append(f'<rect class="s1" x="{x(i) + 1:.1f}" y="{y(k):.1f}" width="{x(i + 1) - x(i) - 2:.1f}" '
               f'height="{y(0) - y(k):.1f}" rx="3"><title>{k} games closed {10 * i}-{10 * i + 10}% of their gap'
               f'</title></rect>')
    svg.append(f'<text class="lab" x="{(x(i) + x(i + 1)) / 2:.1f}" y="{y(k) - 5:.1f}" text-anchor="middle">{k}</text>')
for i in range(11):
    svg.append(f'<text class="lab" x="{x(i):.1f}" y="{H - B + 16}" text-anchor="middle">{10 * i}%</text>')
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{H - 8}" text-anchor="middle">share of the gap between the '
           f'bounds closed by the run</text>')
svg.append(f'<text class="axis" x="16" y="{(T + H - B) / 2}" text-anchor="middle" '
           f'transform="rotate(-90 16 {(T + H - B) / 2})">open games</text>')
svg.append("</svg>")
closed_svg = "\n".join(svg)

# The same games both ways, as grouped bars.
W, H, L, R = 900, 200, 230, 90
x = lin_scale(0, CHAIN_CORE_HOURS, L, W - R)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" '
       f'aria-label="core-hours spent on the same games by the chain and the ledger">']
for v in range(0, int(CHAIN_CORE_HOURS) + 1, 50):
    svg.append(f'<line class="grid" x1="{x(v):.1f}" x2="{x(v):.1f}" y1="10" y2="{H - 30}"/>')
    svg.append(f'<text class="lab" x="{x(v):.1f}" y="{H - 14}" text-anchor="middle">{v}</text>')
for i, (label, chain_h, ledger_h) in enumerate(same_games):
    y0 = 18 + i * 82
    svg.append(f'<text class="axis" x="{L - 10}" y="{y0 + 28}" text-anchor="end">{esc(label)}</text>')
    for j, (who, h, cls) in enumerate((("chain", chain_h, "chainfill"), ("ledger", ledger_h, "s1"))):
        yy = y0 + j * 28
        done = "" if who == "chain" or i == 0 else ", none finished"
        svg.append(f'<rect class="{cls}" x="{L}" y="{yy}" width="{max(x(h) - L, 2):.1f}" height="22" rx="3">'
                   f'<title>{who}: {h:.1f} core-hours on {esc(label)}{done}</title></rect>')
        svg.append(f'<text class="lab" x="{max(x(h), L + 2) + 6:.1f}" y="{yy + 15}">{who} {h:.1f}{done}</text>')
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{H}" text-anchor="middle">core-hours</text>')
svg.append("</svg>")
same_svg = "\n".join(svg)

# 5. how the games were proven: the ledger's proven games against the chain's whole stretch
ledger_mix = {m: sum(v[1] == m for v in proven.values()) for m, _ in METHODS}
chain_mix = {m: 0 for m, _ in METHODS}
for n in ns:
    chain_mix[CHAIN_METHOD.get(truth[n]["proof"], "search")] += 1
W, H, L, R = 900, 170, 70, 20
x = lin_scale(0, len(ns), L, W - R)
svg = [f'<svg viewBox="0 0 {W} {H}" role="img" aria-label="what proved the games, ledger and chain">']
for row, (name, mix) in enumerate((("ledger", ledger_mix), ("chain", chain_mix))):
    yy, acc = 20 + row * 50, 0
    svg.append(f'<text class="lab" x="{L - 6}" y="{yy + 20}" text-anchor="end">{name}</text>')
    for m, label in METHODS:
        k = mix[m]
        if k:
            svg.append(f'<rect class="{COLORS[m]}" x="{x(acc) + 1:.1f}" y="{yy}" '
                       f'width="{max(x(acc + k) - x(acc) - 2, 1):.1f}" '
                       f'height="30" rx="3"><title>{name}: {k:,} by {esc(label)}</title></rect>')
        acc += k
for v in range(0, len(ns) + 1, 250):
    svg.append(f'<text class="lab" x="{x(v):.1f}" y="{132}" text-anchor="middle">{v:,}</text>')
svg.append(f'<text class="axis" x="{(L + W - R) / 2}" y="{158}" text-anchor="middle">games</text>')
svg.append("</svg>")
mix_svg = "\n".join(svg)

legend = "".join(f'<span class="key"><i class="sw {COLORS[m]}"></i>{esc(label)}</span>' for m, label in METHODS)
tiles = [
    (f"{len(proven):,} of {len(ns):,}", f"games proven ({100 * len(proven) / len(ns):.0f}%)"),
    (f"{cpu_end:.1f}", f"core-hours, against the chain's about {CHAIN_CORE_HOURS:.0f}; the chain had proven "
                       f"{chain_at_end:,} games by then"),
    (f"{exact:,} of {len(proven):,}",
     "proven at the chain's optimum; " + ("no bound on the wrong side" if not violations
                                         else f"{len(violations)} VIOLATIONS: {violations[:10]}")),
]
tile_html = "".join(f'<div class="tile"><div class="big">{esc(a)}</div><div class="small">{esc(b)}</div></div>'
                    for a, b in tiles)

page = f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Ledger experiment results</title>
<style>
:root {{ --bg:#fcfcfb; --ink:#0b0b0b; --muted:#52514e; --grid:#e3e2de; --chain:#9a9890; --open:#b9b7af;
  --s1:#2a78d6; --s2:#eb6834; --s3:#1baf7a; --s4:#eda100; }}
@media (prefers-color-scheme: dark) {{ :root:not([data-theme="light"]) {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7;
  --grid:#353533; --chain:#7d7b74; --open:#5c5a55; --s1:#3987e5; --s2:#d95926; --s3:#199e70; --s4:#c98500; }} }}
:root[data-theme="dark"] {{ --bg:#1a1a19; --ink:#ffffff; --muted:#c3c2b7; --grid:#353533; --chain:#7d7b74;
  --open:#5c5a55; --s1:#3987e5; --s2:#d95926; --s3:#199e70; --s4:#c98500; }}
body {{ background:var(--bg); color:var(--ink); font:15px/1.45 -apple-system, Helvetica, Arial, sans-serif;
  margin:0 auto; max-width:940px; padding:24px 16px 48px; }}
h1 {{ font-size:22px; margin:0 0 4px; }} h2 {{ font-size:17px; margin:34px 0 4px; }}
p {{ color:var(--muted); margin:4px 0 10px; }}
.tiles {{ display:flex; gap:12px; flex-wrap:wrap; margin:18px 0 6px; }}
.tile {{ flex:1 1 200px; border:1px solid var(--grid); border-radius:8px; padding:12px 14px; }}
.big {{ font-size:26px; font-weight:600; }} .small {{ color:var(--muted); font-size:13px; }}
svg {{ width:100%; height:auto; display:block; }}
.grid {{ stroke:var(--grid); stroke-width:1; }} .tick {{ stroke:var(--muted); stroke-width:1; }}
.lab {{ fill:var(--muted); font-size:12px; }} .axis {{ fill:var(--ink); font-size:13px; }} .strong {{ fill:var(--ink); }}
.line {{ fill:none; stroke-width:2; stroke-linejoin:round; }} .chain {{ stroke:var(--chain); }} .s1l {{ stroke:var(--s1); }}
.diag {{ stroke:var(--muted); stroke-dasharray:4 3; }} .startbar {{ fill:var(--chain); opacity:0.55; }} .hit {{ fill:transparent; }}
.chainfill {{ fill:var(--chain); }} .open {{ fill:var(--open); }}
.s1 {{ fill:var(--s1); }} .s2 {{ fill:var(--s2); }} .s3 {{ fill:var(--s3); }} .s4 {{ fill:var(--s4); }}
.keys {{ display:flex; gap:16px; flex-wrap:wrap; font-size:13px; color:var(--muted); margin:6px 0; }}
.sw {{ display:inline-block; width:10px; height:10px; border-radius:2px; margin-right:6px; vertical-align:-1px; }}
.sw.s1 {{ background:var(--s1); }} .sw.s2 {{ background:var(--s2); }} .sw.s3 {{ background:var(--s3); }}
.sw.s4 {{ background:var(--s4); }} .sw.startsw {{ background:var(--chain); opacity:0.55; }} .sw.chainsw {{ background:var(--chain); }}
.two {{ display:grid; grid-template-columns:1fr 1fr; gap:20px; }}
@media (max-width:720px) {{ .two {{ grid-template-columns:1fr; }} }}
</style></head><body>
<h1>The ledger on {first}–{last}, with the answers hidden</h1>
<p>Lower and upper bounds for every game at once, tightened by short one-thread searches and the proven relations
between games, effort going where a proof looks cheapest.  One c8g.16xlarge, 64 searches at a time.  Hover any mark
for its numbers.</p>
<div class="tiles">{tile_html}</div>
<h2>With holes allowed: games proven against core-hours</h2>
<p>This is the comparison for the goal of filling in as many games as possible for a budget, holes allowed.  The
ledger takes the cheap games first by design, and the chain cannot skip any.  It says nothing about the cost of a whole
range: the next chart does.  The chain's cost per game is its share of about {CHAIN_CORE_HOURS:.0f} core-hours in
proportion to its search time (its games overlapped on 64 threads).</p>
<div class="keys"><span class="key"><i class="sw s1"></i>the ledger</span><span class="key"><i class="sw chainsw"></i>the
chain, in order of n</span></div>
{curve_svg}
<h2>The same games, both ways</h2>
<p>What each spent on the games the ledger proved and on those it left open.  On the games it proved, the ledger paid
about twice what the chain paid, because the chain had each game's neighbour below already proven; the chain spent
nearly all of its cost on the games the ledger left open.  The ledger's advantage is reach, not price: it proved those
games without first proving everything below them.</p>
{same_svg}
<h2>Where the ledger proved games, and where the chain's cost was</h2>
<div class="keys">{legend}</div>
{map_svg}
<h2>What is left open: how much the bounds tightened</h2>
<p>{left_note}</p>
{closed_svg}
<h2>Every game: where its bounds started, and where the run left them</h2>
<p>In order of n: how far each game's upper bound is above the optimum, and how far its best game is below it.  Gray
is where the quick bounds started; blue is where the run left an open game.  A game with no blue bar was proven.
{start_note}</p>
<div class="keys"><span class="key"><i class="sw startsw"></i>at the start (the quick bounds)</span><span
class="key"><i class="sw s1"></i>at the end of the run (open games only)</span></div>
{left_svg}
<h2>What proved the games</h2>
<p>The ledger's proven games, against how the chain proved the whole stretch.</p>
<div class="keys">{legend}</div>
{mix_svg}
</body></html>
"""
with open(out_path, "w") as f:
    f.write(page)
print(f"{len(proven)} proven of {len(ns)}, {cpu_end:.2f} core-hours, {same_games}; {left_note}; "
      f"violations {violations[:10]}")
