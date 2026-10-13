#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Summaries of the evidence campaigns' JSONL files (plain or .gz).

    summarize.py span FILE...     the (♦) span campaign: games, rejections, spanned, violations, by range
    summarize.py star FILE...     the threshold-by-threshold (★) campaign: thresholds and components by method
    summarize.py exact FILE...    the model-free exact cross-check of optimal scores
"""

import gzip
import json
import sys
from collections import Counter


def lines(paths):
    for path in paths:
        opener = gzip.open if path.endswith(".gz") else open
        with opener(path, "rt") as f:
            for line in f:
                if line.strip():
                    yield json.loads(line)


def span(paths):
    rows = sorted(lines(paths), key=lambda r: r["n"])
    blocks = Counter()
    for r in rows:
        assert r["spanned"] + len(r["violations"]) == r["rejections"], r["n"]
    edges = [2, 1001, 10001, 50001, 100001, 10 ** 12]
    print(f"{'games':>16} {'count':>7} {'rejections':>12} {'spanned':>12} {'violations':>10} "
          f"{'max comp':>9} {'max s':>7}")
    for lo, hi in zip(edges, edges[1:]):
        sel = [r for r in rows if lo <= r["n"] < hi]
        if not sel:
            continue
        label = f"{sel[0]['n']}..{sel[-1]['n']}"
        print(f"{label:>16} {len(sel):>7} {sum(r['rejections'] for r in sel):>12} "
              f"{sum(r['spanned'] for r in sel):>12} {sum(len(r['violations']) for r in sel):>10} "
              f"{max(r['largest_residual_component'] for r in sel):>9} {max(r['secs'] for r in sel):>7.2f}")
        blocks[label] = len(sel)
    bad = [r["n"] for r in rows if r["violations"]]
    print(f"total: {len(rows)} games, {sum(r['rejections'] for r in rows)} rejections, "
          f"{sum(len(r['violations']) for r in rows)} violations {bad[:20]}")


def star(paths):
    rows = sorted(lines(paths), key=lambda r: r["n"])
    thresholds = Counter()
    components = Counter()
    for r in rows:
        thresholds.update(r["thresholds_by_method"])
        components.update(r["components_by_method"])
    print(f"games {rows[0]['n']}..{rows[-1]['n']}: {len(rows)}")
    print(f"rejecting components: {sum(r['rejecting_components'] for r in rows)}; hardest method needed: "
          f"{dict((k, v) for k, v in components.items() if v)}")
    print(f"thresholds: {sum(r['rejections'] for r in rows)}; by method: "
          f"{dict((k, v) for k, v in thresholds.items() if v)}")
    print(f"largest P: {max(r['largest_p'] for r in rows)}, largest g: {max(r['max_g'] for r in rows)}")


def exact(paths):
    """Which games are proven optimal without the frame model: n is proven when its matching bound equals the
    recorded score, or the search proved it (if it used the segment cuts, only when every smaller game is
    proven), or its score meets the chain or prime relation and n - 1 is proven."""
    rows = {r["n"]: r for r in lines(paths)}
    proven = {1: True}
    how = Counter()
    first_gap = None
    for n in range(2, max(rows) + 1):
        r = rows.get(n)
        ok = False
        if r is not None:
            if r["fm_bound"] == r["score"]:
                ok, kind = True, "matching bound"
            elif r["search"] == "proven" and (not r.get("segments") or all(proven.get(m) for m in range(2, n))):
                ok, kind = True, "search"
            elif r["relation"] in ("chain", "prime") and proven.get(n - 1):
                ok, kind = True, r["relation"]
        proven[n] = ok
        if ok:
            how[kind] += 1
        elif first_gap is None:
            first_gap = n
    done = sorted(rows)
    print(f"games recorded: {len(rows)} ({done[0]}..{done[-1]}); proven: {sum(proven.values()) - 1}; "
          f"by {dict(how)}")
    print(f"every n in 2..{(first_gap or max(rows) + 1) - 1} proven; first unproven: {first_gap}")
    if first_gap:
        beyond = [n for n in range(first_gap, max(rows) + 1) if proven.get(n)]
        open_ = [n for n in range(first_gap, max(rows) + 1) if not proven.get(n)]
        print(f"beyond it, proven: {beyond}; not (yet): {open_}")
    searched = [r for r in rows.values() if r["search"] in ("proven", "budget")]
    print(f"searches: {len(searched)}, total {sum(r['secs'] for r in searched) / 3600:.1f} h, "
          f"most nodes {max((r['nodes'], r['n']) for r in searched)}")
    last = max(searched, key=lambda r: r['n'])
    print(f"matching-bound gap at the largest searched n ({last['n']}): {last['fm_bound'] - last['score']}")
    big = sorted(searched, key=lambda r: -r["secs"])[:5]
    print("slowest: " + ", ".join(f"n={r['n']} {r['secs']:.0f}s {r['nodes']} nodes" for r in big))


def adversary(paths):
    for path in paths:
        rows = list(lines([path]))
        if not rows:
            continue
        diamond = sum(1 for r in rows if r["violations"])
        star = Counter()
        for r in rows:
            star.update(r.get("star", {}))
        beaten = sum(1 for r in rows if r.get("star", {}).get("counterexample", 0))
        undecided = sum(1 for r in rows if r.get("star", {}).get("undecided", 0))
        print(f"{path}: {len(rows)} instances (size {rows[0]['size']}, eps {rows[0]['eps']}); "
              f"(♦) fails in {diamond}, (★) fails in {beaten}, undecided {undecided}; "
              f"rejections {sum(r['rejections'] for r in rows)}, violating thresholds "
              f"{sum(r['violations'] for r in rows)}; threshold verdicts {dict((k, v) for k, v in star.items() if v)}")


if __name__ == "__main__":
    {"span": span, "star": star, "exact": exact, "adversary": adversary}[sys.argv[1]](sys.argv[2:])
