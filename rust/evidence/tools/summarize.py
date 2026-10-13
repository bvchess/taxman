#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Summaries of the evidence campaigns' JSONL files (plain or .gz).

    summarize.py span FILE...     the (♦) span campaign: games, rejections, spanned, violations, by range
    summarize.py star FILE...     the threshold-by-threshold (★) campaign: thresholds and components by method
    summarize.py exact FILE...    the model-free exact cross-check of optimal scores
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
OPTIMAL_JSON = ROOT / "src/main/resources/optimal.json"  # the recorded games n = 1..1000
STRICT_JSONL = ROOT / "rust/results/strict.jsonl"  # the recorded games beyond that (the first line for an n)
OPTIMAL_JSON_LAST_N = 1000
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


@cache
def optimal_json_moves():
    return {g["n"]: g["moves"] for g in json.loads(OPTIMAL_JSON.read_text())}


@cache
def strict_moves():
    first = {}
    for r in lines([str(STRICT_JSONL)]):
        first.setdefault(r["n"], r["moves"])
    return first


def recorded_moves(n):
    """The moves of the recorded game for n (loaded on first use)."""
    return (optimal_json_moves() if n <= OPTIMAL_JSON_LAST_N else strict_moves())[n]


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
    """Whether the canonical set of n - 1 maps to that of n at each chain or prime step, and where the recorded
    game's set stands among the canonically ordered sets."""
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
                print(f"too many sets to list (not analysed below): {compact_ranges(unlisted)}")
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


if __name__ == "__main__":
    {"span": span, "star": star, "exact": exact, "adversary": adversary, "optsets": optsets,
     "moves": moves}[sys.argv[1]](sys.argv[2:])
