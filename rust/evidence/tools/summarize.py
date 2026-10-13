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
    print(f"{'games':>16} {'count':>7} {'rejections':>12} {'spanned':>12} {'violations':>10} {'max comp':>9} {'max s':>7}")
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
    rows = sorted(lines(paths), key=lambda r: r["n"])
    verdicts = Counter(r["verdict"] for r in rows)
    print(f"games {rows[0]['n']}..{rows[-1]['n']}: {len(rows)}; verdicts {dict(verdicts)}")
    print(f"how proven: {dict(Counter(r['how'] for r in rows))}")
    print(f"total seconds {sum(r['secs'] for r in rows):.0f}, max {max(r['secs'] for r in rows):.1f}")


if __name__ == "__main__":
    {"span": span, "star": star, "exact": exact}[sys.argv[1]](sys.argv[2:])
