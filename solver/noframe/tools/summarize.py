#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Summaries of the prover's campaigns' JSONL files (plain or .gz).

    summarize.py span FILE...     the (♦) span campaign: games, rejections, spanned, violations, by range
    summarize.py star FILE...     the threshold-by-threshold (★) campaign: thresholds and components by method
    summarize.py exact FILE...    the proofs of optimal scores (a later file overrides), and how many searches start
                                  from the roles of F4 (`fix_low` 1, which `exact` does unless told not to)
    summarize.py check A B FILE...  FILEs prove every n in 2..B, with the recorded scores in A..B
    summarize.py order A B        the games A..B, dearest first by the solver's nodes (the order prover.sh runs them)
    summarize.py cost [--band N] FILE...  its cost curve: the searches made with the parity bound from scratch, by band
    summarize.py sources B_FILE D_FILE  study E6: the search with the sources of layer 3 and up free (exact, from the
                                  roles of F4) against the same search with the whole upper part fixed to U*
                                  (--fix-upper 1)
    summarize.py optsets FILE...  every optimal pick set per game (bin/optsets.rs): tables, exchanges, canonical sets
    summarize.py moves FILE...    the longest games against the optimal ones (bin/moves.rs)
"""

import gzip
import json
import sys
from collections import Counter
from functools import cache
from itertools import combinations
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
COMMITTED = ROOT / "results/optimal.jsonl.gz"  # the committed chain: the recorded games, a result line each
UPPER_MODES = {
    "all": "every optimal set",
    "fixed": "optimal sets whose upper part is U*",
    "other": "optimal sets whose upper part is not U*",
}


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


def exact_proofs(rows):
    """Which games are proven optimal without the frame model: n is proven when its Franklín–Moniot bound or its
    parity bound equals the recorded score, or the search proved it (if it used the segment cuts, only when every
    smaller game is proven), or its score is at its ceiling (`chain`), meets the prime formula (`prime`) or meets the
    semiprime formula (`semiprime`, J3 of docs/theory/arrival.md) and n - 1 is proven.  Returns (proven, how,
    first_gap): whether each n in 1..max(rows) is proven, how the proven ones were (counts by kind), and the first n
    that is not (None if all are).

    Only `"search":"proven"` counts as a proof.  A line with `"search":"upper_fixed"` (`exact --fix-upper 1`, the
    frame model's problem with the whole upper part fixed to U*) is a measurement and never a proof.  A search with
    `fix_low` 1 started from the roles of F4 (the bottom-layers theorem, docs/theory/frame-model.md), so its proof
    rests on that theorem too (`on_theorem_b`)."""
    proven = {1: True}
    how = Counter()
    first_gap = None
    for n in range(2, max(rows) + 1):
        r = rows.get(n)
        ok = False
        if r is not None:
            if r["fm_bound"] == r["score"]:
                ok, kind = True, "Franklín–Moniot bound"
            elif r.get("parity_bound") == r["score"]:
                ok, kind = True, "parity bound"
            elif r["search"] == "proven" and (not r.get("segments") or all(proven.get(m) for m in range(2, n))):
                ok, kind = True, "search"
            elif r["relation"] in ("chain", "prime", "semiprime") and proven.get(n - 1):
                ok, kind = True, r["relation"]
        proven[n] = ok
        if ok:
            how[kind] += 1
        elif first_gap is None:
            first_gap = n
    return proven, how, first_gap


def on_theorem_b(rows):
    """What the proofs of `rows` rest on beyond the model-independent bounds: the number of searches among them that
    started from the roles of F4 (`fix_low` 1), and the number of all searches that proved."""
    searches = [r for r in rows.values() if r["search"] == "proven"]
    return sum(1 for r in searches if r.get("fix_low")), len(searches)


def exact(paths):
    """The proofs of optimal scores (`exact_proofs`); a later file's line for a game replaces an earlier file's."""
    rows = {r["n"]: r for r in lines(paths) if not r.get("fix_upper")}  # fix_upper lines are measurements
    if not rows:
        print("no lines to summarize (lines with fix_upper are measurements, not proofs, and are ignored)")
        return
    proven, how, first_gap = exact_proofs(rows)
    done = sorted(rows)
    print(f"games recorded: {len(rows)} ({done[0]}..{done[-1]}); proven: {sum(proven.values()) - 1}; "
          f"by {dict(how)}")
    from_b, searches = on_theorem_b(rows)
    print(f"searches that proved: {searches}, of them {from_b} from the roles of F4 (fix_low)")
    print(f"every n in 2..{(first_gap or max(rows) + 1) - 1} proven; first unproven: {first_gap}")
    if first_gap:
        beyond = [n for n in range(first_gap, max(rows) + 1) if proven.get(n)]
        open_ = [n for n in range(first_gap, max(rows) + 1) if not proven.get(n)]
        print(f"beyond it, proven: {compact_ranges(beyond)}; not (yet): {compact_ranges(open_)}")
    searched = [r for r in rows.values() if r["search"] in ("proven", "budget")]
    print(f"searches: {len(searched)}, total {sum(r['secs'] for r in searched) / 3600:.1f} h, "
          f"most nodes {max((r['nodes'], r['n']) for r in searched)}")
    last = max(searched, key=lambda r: r['n'])
    print(f"Franklín–Moniot-bound gap at the largest searched n ({last['n']}): {last['fm_bound'] - last['score']}")
    big = sorted(searched, key=lambda r: -r["secs"])[:5]
    print("slowest: " + ", ".join(f"n={r['n']} {r['secs']:.0f}s {r['nodes']} nodes" for r in big))
    resumed = [r for r in searched if r.get("parity") and r.get("matching_nodes")]
    if resumed:
        print("resumed with the parity bound: " + ", ".join(
            f"n={r['n']} +{r['nodes'] - r['matching_nodes']} nodes" for r in sorted(resumed, key=lambda r: r["n"])))


def exact_cost(args):
    """cost [--band N] FILE...: the cost curve of the searches made with the parity bound from scratch (no nodes
    without it), by band of n (50 wide unless N says): how many, nodes and seconds, milliseconds a node (total seconds
    over total nodes), the root's parity gap; and the least-squares slopes of log10 nodes and seconds.  Lines with
    `fix_upper` (measurements of the frame model's problem) are ignored.  A search from the roles of F4
    (`fix_low` 1) takes about half the time a node of the plain root's: do not mix them in one table."""
    import math
    import statistics
    band, paths = (int(args[1]), args[2:]) if args[:1] == ["--band"] else (50, args)
    rows = sorted((r for r in lines(paths) if r.get("parity") and r["search"] == "proven"
                   and not r.get("matching_nodes") and not r.get("fix_upper")), key=lambda r: r["n"])
    hours = sum(r['secs'] for r in rows) / 3600
    print(f"searches with the parity bound from scratch: {len(rows)}, total {hours:.2f} h")
    print(f"{'n':>11} {'games':>5} {'gap':>6} {'median nodes':>12} {'max nodes':>10} {'median s':>9} {'max s':>8} "
          f"{'total nodes':>12} {'total s':>8} {'ms/node':>8}")
    for lo in range(rows[0]["n"] // band * band, rows[-1]["n"] + 1, band):
        sel = [r for r in rows if lo <= r["n"] < lo + band]
        if not sel:
            continue
        gaps = [r["parity_bound"] - r["score"] for r in sel]
        nodes = [r["nodes"] for r in sel]
        secs = [r["secs"] for r in sel]
        print(f"{lo:>5}-{lo + band - 1:<5} {len(sel):>5} {statistics.mean(gaps):>6.0f} "
              f"{statistics.median(nodes):>12.0f} {max(nodes):>10} {statistics.median(secs):>9.1f} {max(secs):>8.1f} "
              f"{sum(nodes):>12} {sum(secs):>8.0f} {1000 * sum(secs) / max(sum(nodes), 1):>8.2f}")

    def slope(xs, ys):
        mx, my = statistics.mean(xs), statistics.mean(ys)
        sxx = sum((x - mx) ** 2 for x in xs)
        b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx
        resid = sum((y - my - b * (x - mx)) ** 2 for x, y in zip(xs, ys))
        se = math.sqrt(resid / (len(xs) - 2) / sxx)
        r2 = 1 - resid / sum((y - my) ** 2 for y in ys)
        return b, se, r2

    for lo_n in (rows[0]["n"], 600, 900):
        sel = [r for r in rows if r["n"] >= lo_n]
        if len(sel) < 3:
            continue
        xs = [r["n"] for r in sel]
        for key in ("nodes", "secs"):
            b, se, r2 = slope(xs, [math.log10(max(r[key], 1e-3)) for r in sel])
            print(f"from n = {lo_n}: log10 {key} grows {100 * b:.3f} ± {100 * se:.3f} per 100 in n (R² {r2:.2f})")
        b, se, r2 = slope([r["parity_bound"] - r["score"] for r in sel], [math.log10(max(r["nodes"], 1)) for r in sel])
        print(f"from n = {lo_n}: log10 nodes grows {100 * b:.3f} ± {100 * se:.3f} per 100 of the root's parity gap "
              f"(R² {r2:.2f})")


# The criterion of study E6, fixed before the measurement: the integrated search, which leaves the sources of layer 3
# and up free, is worth building in full if it is not much dearer than the search with the whole upper part fixed.
E6_RATIO = 1.2  # B/D at most this on ...
E6_SHARE = 0.9  # ... at least this share of the games,
E6_WORST = 2.0  # B/D never above this, and
E6_BRANCHING = 0.05  # B's nodes branching on a source of layer 3 or up at most this share of all B's nodes.


def exact_sources(args):
    """sources B_FILE D_FILE: study E6, what the sources of layer 3 and up cost.  B_FILE holds the searches from the
    roles of F4 (`exact`, whose default is `--fix-low 1`: `fix_low` 1, no `fix_upper`, `"search":"proven"`, so each is
    a proof of opt(n) <= V with the sources of layer 3 and up free); D_FILE the same searches with the whole upper
    part fixed to U* (`--fix-upper 1`: `fix_upper` 1, `"search":"upper_fixed"`, which is the frame model's problem on
    the whole board and proves nothing).  For each game with a finished search in both files, a row: n, the root's
    parity gap (`parity_bound - score`), the nodes of B and of D, B/D, the seconds of B and of D, and B's counters
    `branched_on_sources` (nodes that branched on a free source of layer 3 or up) and `barred_upper` (nodes whose roles
    bar a member of U*_{>=3}) as percentages of B's nodes.  The games finished in only one file are listed.  Then
    the criterion of study E6, printed part by part with its figure and PASS or FAIL:

        B/D at most 1.2 on at least 90% of the games, B/D never above 2, and the nodes of B branching on a source of
        layer 3 or up at most 5% of all B's nodes (summed over the games).

    The last line is the verdict: PASS (build the integrated search's stages 2-3) or FAIL (build stage 1 only and
    measure stage 2 on two games first).  Exits 1, with no verdict, if no game is finished in both files."""
    b_path, d_path = args
    b_rows = {r["n"]: r for r in lines([b_path]) if r.get("fix_low") and not r.get("fix_upper")
              and r["search"] == "proven"}
    d_rows = {r["n"]: r for r in lines([d_path]) if r.get("fix_upper") and r["search"] == "upper_fixed"}
    both = sorted(b_rows.keys() & d_rows.keys())
    if not both:
        print(f"no game has a finished search in both {b_path} (B) and {d_path} (D)")
        sys.exit(1)
    print(f"{'n':>5} {'gap':>6} {'B nodes':>10} {'D nodes':>10} {'B/D':>6} {'B secs':>9} {'D secs':>9} "
          f"{'B branching':>11} {'B barring':>10}")
    ratios = []
    for n in both:
        b, d = b_rows[n], d_rows[n]
        ratios.append(b["nodes"] / d["nodes"])
        print(f"{n:>5} {b['parity_bound'] - b['score']:>6} {b['nodes']:>10} {d['nodes']:>10} {ratios[-1]:>6.2f} "
              f"{b['secs']:>9.1f} {d['secs']:>9.1f} {100 * b['branched_on_sources'] / b['nodes']:>10.2f}% "
              f"{100 * b['barred_upper'] / b['nodes']:>9.2f}%")
    only_b, only_d = sorted(b_rows.keys() - d_rows.keys()), sorted(d_rows.keys() - b_rows.keys())
    print(f"finished in B only: {compact_ranges(only_b)}; in D only: {compact_ranges(only_d)}")
    close = sum(r <= E6_RATIO for r in ratios)
    worst = max(ratios)
    branching = sum(b_rows[n]["branched_on_sources"] for n in both) / sum(b_rows[n]["nodes"] for n in both)
    parts = [
        (f"B/D <= {E6_RATIO} on at least {100 * E6_SHARE:.0f}% of the games", close >= E6_SHARE * len(both),
         f"{close} of {len(both)} games ({100 * close / len(both):.1f}%)"),
        (f"B/D never above {E6_WORST}", worst <= E6_WORST, f"largest {worst:.2f} (n={both[ratios.index(worst)]})"),
        (f"B's nodes branching on a source of layer >= 3 at most {100 * E6_BRANCHING:.0f}% of B's nodes",
         branching <= E6_BRANCHING, f"{100 * branching:.2f}%"),
    ]
    for what, ok, figure in parts:
        print(f"{'PASS' if ok else 'FAIL'}  {what}: {figure}")
    if all(ok for _, ok, _ in parts):
        print("study E6: PASS (build the integrated search's stages 2-3)")
    else:
        print("study E6: FAIL (build stage 1 only and measure stage 2 on two games first)")


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


@cache
def recorded_games():
    """The result line of each n of the committed chain (loaded on first use)."""
    return {r["n"]: r for r in lines([str(COMMITTED)])}


def recorded_moves(n):
    """The moves of the recorded game for n."""
    return recorded_games()[n]["moves"]


def prime_factors(m):
    factors, p = [], 2
    while p * p <= m:
        while m % p == 0:
            factors.append(p)
            m //= p
        p += 1
    return factors + [m] if m > 1 else factors


def previous_prime(n):
    p = n - 1
    while prime_factors(p) != [p]:
        p -= 1
    return p


def semiprime_exchange(n):
    """x of the semiprime formula (J3, docs/theory/arrival.md) at an odd n with two prime factors, or None if n is not
    such a number or closes no cycle with U*_2(n - 1), the numbers of layer 2 in U*(n - 1).  The graph has the primes
    up to n / 2 and the ground, 0, for vertices; pq is the edge p-q and p^2 the edge p-0.  U*(n - 1) is read from the
    recorded game of n - 1, which plays it above (n - 1) / 2 (`recorded_ustar` in src/upper.rs checks that for every
    game the prover proves); x is the smallest edge on the path between the ends of n."""
    def ends(m):
        factors = prime_factors(m)
        return (factors[0], factors[1] if factors[0] != factors[1] else 0) if len(factors) == 2 else None

    if n % 2 == 0 or ends(n) is None:
        return None
    edges_at = {}
    for m in recorded_moves(n - 1):
        if m > (n - 1) // 2 and ends(m) is not None:
            a, b = ends(m)
            edges_at.setdefault(a, []).append((b, m))
            edges_at.setdefault(b, []).append((a, m))
    start, goal = ends(n)
    reached, stack = {start}, [(start, [])]
    while stack:
        vertex, path = stack.pop()
        if vertex == goal:
            return min(path)
        for other, m in edges_at.get(vertex, []):
            if other not in reached:
                reached.add(other)
                stack.append((other, path + [m]))
    return None


def factorized(m):
    """168 as '168=2^3*3*7' (a prime is just itself)."""
    factors = prime_factors(m)
    if len(factors) < 2:
        return str(m)
    return f"{m}=" + "*".join(f"{p}^{e}" if e > 1 else str(p) for p, e in sorted(Counter(factors).items()))


def compact_ranges(ns):
    """Integers as '3..5, 9, 12..13'."""
    runs = []
    for n in sorted(ns):
        if runs and n == runs[-1][1] + 1:
            runs[-1][1] = n
        else:
            runs.append([n, n])
    return ", ".join(str(a) if a == b else f"{a}..{b}" for a, b in runs) or "none"


def optsets_by_mode(paths):
    """{upper mode: {n: line}}; a later file's line for the same mode and n replaces an earlier one."""
    modes = {}
    for r in lines(paths):
        modes.setdefault(r.get("upper", "all"), {})[r["n"]] = r
    return modes


def canonical_order(sets):
    """Sets by their members in decreasing order, lexicographically, the largest (the canonical one) first."""
    return sorted(sets, key=lambda s: sorted(s, reverse=True), reverse=True)


def listed_sets(r):
    """The optimal sets of a complete optsets line as frozensets, canonical first; None if it does not list them all."""
    if r["ended"] != "complete" or r["sets"] > len(r["others"]) + 1:
        return None
    if r["sets"] == 0:
        return []
    canonical = frozenset(r["canonical"] if "canonical" in r else recorded_moves(r["n"]))
    sets = [canonical] + [(canonical - set(o["out"])) | set(o["in"]) for o in r["others"]]
    assert len(set(sets)) == len(sets) == r["sets"], f"n={r['n']}: the line's sets do not add up"
    assert all(sum(s) == r["score"] for s in sets), f"n={r['n']}: a reconstructed set does not sum to the score"
    ordered = canonical_order(sets)
    assert ordered[0] == canonical, f"n={r['n']}: the canonical set is not the largest"
    return ordered


def optsets_bucket(n):
    """The first n of the table row holding n: 2..424, 425..500, then hundreds from 501."""
    return 2 if n < 425 else 425 if n <= 500 else 501 + (n - 501) // 100 * 100


def optsets_table(rows):
    def summary(label, sel):
        done = [r for r in sel if r["ended"] == "complete"]
        by_count = Counter(min(r["sets"], 3) for r in done)
        print(f"{label:>12} {len(sel):>6} {len(done):>8} {by_count[0]:>6} {by_count[1]:>6} {by_count[2]:>6} "
              f"{by_count[3]:>6} {len(sel) - len(done):>12} {sum(r['nodes'] for r in sel):>15,} "
              f"{max(r['secs'] for r in sel):>9.1f}")

    print(f"{'range':>12} {'games':>6} {'complete':>8} {'0 sets':>6} {'1 set':>6} {'2 sets':>6} {'3+ sets':>6} "
          f"{'budget/limit':>12} {'nodes':>15} {'max secs':>9}")
    groups = {}
    for r in rows:
        groups.setdefault(optsets_bucket(r["n"]), []).append(r)
    for _, sel in sorted(groups.items()):
        summary(f"{sel[0]['n']}..{sel[-1]['n']}", sel)
    summary("total", rows)


def optsets_classes(rows, mode):
    """Which complete games have only U* upper parts, a single upper part, distinct lower parts; the first n with
    several sets (for 'other', how many complete games have no such set at all)."""
    done = [r for r in rows if r["ended"] == "complete"]
    several = [r["n"] for r in done if r["sets"] > 1]
    first = f"; first n with more than one set: {min(several)}" if several else "; no game has more than one set"
    if mode == "other":
        print(f"complete games with no such set: {sum(r['sets'] == 0 for r in done)} of {len(done)}{first}")
        return
    greedy = sum(r["upper_is_greedy"] == r["sets"] for r in done)
    one_upper = sum(r["upper_parts"] <= 1 for r in done)
    distinct_lower = sum(r["lower_parts"] == r["sets"] for r in done)
    print(f"of {len(done)} complete games: every set has upper part U* in {greedy}, all sets share one upper part in "
          f"{one_upper}, lower parts all distinct (lower_parts == sets) in {distinct_lower}{first}")


def optsets_incomplete(rows):
    ended = Counter(r["ended"] for r in rows if r["ended"] != "complete")
    print("incomplete games: " + ("; ".join(
        f"{how} {compact_ranges(r['n'] for r in rows if r['ended'] == how)}" for how in sorted(ended)) or "none"))


def exchange(a, b):
    """What the optimal sets a and b trade: the two disjoint sides (each ascending, the one with the larger
    numbers first), which have equal sums."""
    sides = sorted((tuple(sorted(a - b)), tuple(sorted(b - a))), key=lambda side: side[::-1], reverse=True)
    assert sum(sides[0]) == sum(sides[1]), f"an exchange between equal-sum sets has unequal sides: {sides}"
    return tuple(sides)


def optsets_exchanges(rows, sets_of):
    """Every exchange that is new at its n (not between two sets of n - 1), and whether all sets of a game have the
    same size."""
    exchanges = {n: {exchange(a, b) for a, b in combinations(sets, 2)} for n, sets in sets_of.items()}
    uneven = [n for n, sets in sets_of.items() if len({len(s) for s in sets}) > 1]
    print("all sets of a game have the same size: "
          + ("yes" if not uneven else f"NO, exceptions {compact_ranges(uneven)}") + f" ({len(sets_of)} games)")
    ratios = [(max(left[-1], right[-1]) / n, n) for n, trades in exchanges.items() for left, right in trades]
    print(f"{len(set().union(*exchanges.values()))} distinct exchanges"
          + (f"; the largest number traded is {max(ratios)[0]:.3f} n (n={max(ratios)[1]})" if ratios else ""))
    for n in sorted(exchanges):
        before = exchanges.get(n - 1)
        new = sorted(exchanges[n] - (before or set()))
        if new:
            note = "" if before is not None else " (n-1 not listed)"
            print(f"n={n} {rows[n]['relation']}, {len(sets_of[n])} sets{note}")
        for left, right in new:
            sides = " <-> ".join("{" + ", ".join(map(factorized, side)) + "}" for side in (left, right))
            print(f"  {sides}  sum {sum(left)}, {len(left)}<->{len(right)}, "
                  f"max/n {max(left[-1], right[-1]) / n:.3f}")


def optsets_canonical(rows, sets_of):
    """Whether the canonical set of n - 1 maps to that of n at each step to a game at its ceiling (`chain`) or to a
    prime (`prime`), and where the recorded game's set stands among the canonically ordered sets."""
    kept, steps, broken = 0, 0, []
    for n in sorted(sets_of):
        before = sets_of.get(n - 1)
        relation = rows[n]["relation"]
        if relation not in ("chain", "prime") or before is None or len(before) < 2:
            continue
        mapped = (before[0] - ({previous_prime(n)} if relation == "prime" else set())) | {n}
        steps += 1
        if mapped == sets_of[n][0]:
            kept += 1
        else:
            where = f"rank {sets_of[n].index(mapped)} of {len(sets_of[n])}" if mapped in sets_of[n] else "not optimal"
            broken.append(f"{n} ({relation}, mapped set {where})")
    print(f"canonical survives {kept} of {steps} chain/prime steps from a multi-set game"
          + (f"; exceptions: {', '.join(broken)}" if broken else ""))
    ranks = {}
    for n, sets in sets_of.items():
        if len(sets) > 1:
            recorded = frozenset(recorded_moves(n))
            assert recorded in sets, f"n={n}: the recorded game is not among the listed sets"
            ranks[n] = sets.index(recorded)
    off = {n: rank for n, rank in sorted(ranks.items()) if rank}
    print(f"recorded = canonical in {len(ranks) - len(off)} of {len(ranks)} multi-set games"
          + (f"; exceptions: {', '.join(f'{n} (rank {rank})' for n, rank in off.items())}" if off else ""))


def optsets(paths):
    modes = optsets_by_mode(paths)
    for mode, meaning in UPPER_MODES.items():
        if mode not in modes:
            continue
        rows = [modes[mode][n] for n in sorted(modes[mode])]
        print(f"== upper {mode} ({meaning}): {len(rows)} games, n = {rows[0]['n']}..{rows[-1]['n']}")
        optsets_table(rows)
        optsets_classes(rows, mode)
        optsets_incomplete(rows)
        if mode == "all":
            listed = {r["n"]: listed_sets(r) for r in rows if r["ended"] == "complete"}
            unlisted = [n for n, sets in listed.items() if sets is None]
            if unlisted:
                print(f"too many sets to list (not analyzed below): {compact_ranges(unlisted)}")
            sets_of = {n: sets for n, sets in listed.items() if sets}
            print("-- exchanges")
            optsets_exchanges(modes[mode], sets_of)
            print("-- canonical sets")
            optsets_canonical(modes[mode], sets_of)


def moves(paths):
    """How M(n), the most moves of a game of n, was settled, and the games whose recorded optimal game is shorter."""
    rows = {r["n"]: r for r in lines(paths)}
    ns = sorted(rows)
    how = Counter(r["how"] for r in rows.values())
    undecided = [n for n in ns if rows[n]["most"] is None]
    print(f"games: {len(ns)} ({ns[0]}..{ns[-1]}); M(n) settled by {dict(how)}; undecided: {compact_ranges(undecided)}")
    over = [n for n in ns if rows[n]["most"] is not None and rows[n]["nu"] > rows[n]["most"]]
    print(f"matching number above M(n): {compact_ranges(over)}")
    gaps = Counter(r["gap"] for r in rows.values() if r["most"] is not None)
    print(f"M(n) minus the recorded game's moves: {dict(sorted(gaps.items()))}")
    for n in ns:
        r = rows[n]
        if not r.get("gap"):
            continue
        heaviest = ""
        if "heaviest_sum" in r:
            heaviest = (f"; the heaviest longest game scores {r['heaviest_sum']} ({r['score'] - r['heaviest_sum']} "
                        f"less): {r['heaviest_out']} out, {r['heaviest_in']} in")
        print(f"n={n}: the recorded game has {r['recorded_moves']} moves ({r['recorded_upper']} above n/2), "
              f"M(n) = {r['most']} ({r['how']}){heaviest}")


def recorded_scores():
    """The recorded optimum V(n) of each n, from the committed chain."""
    return {n: r["score"] for n, r in recorded_games().items()}


def exact_order(args):
    """order A B: the games A..B, comma separated, the ones `exact` will search the longest first.  A game's cost is
    predicted by the nodes of the solver's search that settled it in the committed chain (the source game's for a
    repeated search) times 10^((n - 1300)/1000): over 34 games of 1302-2124 `exact` took 0.13 to 13 times as many
    nodes (the geometric mean 0.97 at 1300-1500 and 3.8 at 1800-2124, the scatter 0.28 decades), because the parity
    bound is the weaker bound and its gap grows faster than the solver's.  It is a guide to the order, not an estimate.
    Games at their ceiling (`chain`) or meeting the prime formula (`prime`) or the semiprime formula (`semiprime`),
    which `exact` does not search, go last."""
    lo, hi = int(args[0]), int(args[1])
    rows = recorded_games()

    def solver_nodes(n):
        r = rows[n]
        while r["proof"] == "same_search":
            r = rows[r["same_search"]["n"]]
        return r["nodes"]

    def searched(n):
        score, prev = rows[n]["score"], rows[n - 1]["score"]
        prime = n > 2 and prime_factors(n) == [n] and score == n + prev - previous_prime(n)
        if score == n + prev or prime:
            return False
        x = semiprime_exchange(n)
        return x is None or score != prev + n - x

    cost = {n: solver_nodes(n) * 10 ** ((n - 1300) / 1000) if searched(n) else -1 for n in range(lo, hi + 1)}
    print(",".join(str(n) for n in sorted(cost, key=lambda n: (-cost[n], -n))))


def exact_check(args):
    """check A B FILE...: the files (a later file's line for a game replaces an earlier file's) prove opt(n) for every
    n in 2..B (`exact_proofs`: so the files must reach back to 2, as data/exact_parity_2_1300.jsonl does), have a
    line for every n in A..B, and each line's score is the recorded optimum.  Exits 1 if any of that fails."""
    lo, hi = int(args[0]), int(args[1])
    rows = {r["n"]: r for r in lines(args[2:])}
    scores = recorded_scores()
    missing = [n for n in range(lo, hi + 1) if n not in rows]
    wrong = [n for n in range(lo, hi + 1) if n in rows and rows[n]["score"] != scores.get(n)]
    proven, how, _ = exact_proofs({n: r for n, r in rows.items() if n <= hi})
    unproven = [n for n in range(2, hi + 1) if not proven.get(n)]
    in_range = Counter(rows[n]["search"] for n in range(lo, hi + 1) if n in rows)
    print(f"games {lo}..{hi}: {hi - lo + 1 - len(missing)} of {hi - lo + 1} have a line; searches: "
          f"{dict(in_range)}; proven by {dict(how)}")
    print(f"without a line: {compact_ranges(missing) or 'none'}; score not the recorded optimum: "
          f"{compact_ranges(wrong) or 'none'}; not proven in 2..{hi}: {compact_ranges(unproven) or 'none'}")
    from_b, searches = on_theorem_b({n: r for n, r in rows.items() if n <= hi})
    print(f"searches that proved: {searches}, of them {from_b} from the roles of F4 (fix_low)")
    ok = not (missing or wrong or unproven)
    print(f"{'OK: opt(n) is proven equal to the recorded score for every n in 2..' + str(hi) if ok else 'FAILED'}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    {"span": span, "star": star, "exact": exact, "check": exact_check, "order": exact_order, "cost": exact_cost,
     "sources": exact_sources, "adversary": adversary, "optsets": optsets, "moves": moves}[sys.argv[1]](sys.argv[2:])
