"""Anatomy of the transition from optimal game n-1 to optimal game n.

Adding the number n to a Taxman pot perturbs the optimal solution.  This
module measures, for each n in a range, how local and how shallow that
perturbation is.  It builds on the same matching theory approx.py's solvent
uses (a set of picks is playable iff a divisor-matching with an acyclic
precedence relation exists), exposed here as an incremental evaluator
(SetEval) so a candidate set can be mutated pick-by-pick and re-tested.

For every analyzed n it reports:

  * churn_lower   how many lower-half (2*m <= n) picks changed between the
                  two optimal games (the symmetric difference size).
  * locality      divisor-graph distance from n of each changed lower pick.
  * incumbent_gap the score lost by adapting game n-1's lower picks onto
                  game n's provably-optimal upper half, and whether that
                  gap carries a clean certificate.
  * geodesic_depth the smallest move size (add / swap-pair / swap-triple)
                  needed to walk the adapted incumbent to game n's lower
                  set, or "blocked" if no strictly-improving move exists.
  * blind_gap     the residual gap after steepest-ascent single-flip hill
                  climbing from the adapted incumbent.

A sanity gate reloads each known optimal set through SetEval first, so a
bug in the evaluator surfaces as evaluator_failures rather than as bogus
transition statistics.

Usage:
    python3 transitions.py [--from 500] [--to 1000] [--optimal PATH]
                           [--sample 1]
"""

from __future__ import annotations

import argparse
import itertools
import json
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple, Union

from approx import divisor_lists, maximal_factor_lists
from seteval import SetEval
from taxman_mini import (
    MiniInfeasible, smallest_prime_factors, solve_mini, solve_upper_half,
)
from verify import DEFAULT_OPTIMAL

MINI_DIR = Path(__file__).resolve().parent


# ---------------------------------------------------------------------------
# delta classification and divisor-graph locality
# ---------------------------------------------------------------------------

def classify_deltas(n: int, prev_set: Set[int], cur_set: Set[int]) -> Dict[str, Any]:
    """Classify the added/removed picks between optimal games n-1 and n.

    Both sets are split at n's own threshold (2*m <= n is "lower").  The
    boundary list holds any migrant at the half-step that only n's parity
    move crosses (m == n/2 when n is even).
    """
    added = cur_set - prev_set
    removed = prev_set - cur_set
    n_in_added = n in added

    changed = added | removed
    boundary = sorted(
        m for m in changed if (2 * m > n - 1) and not (2 * m > n)
    )
    added_lower = sorted(m for m in added if 2 * m <= n)
    removed_lower = sorted(m for m in removed if 2 * m <= n)
    churn_lower = len(added_lower) + len(removed_lower)

    return {
        "added_lower": added_lower,
        "removed_lower": removed_lower,
        "boundary": boundary,
        "churn_lower": churn_lower,
        "n_in_added": n_in_added,
    }


def locality_bfs(
    n: int, changed: Sequence[int], divs: Sequence[List[int]]
) -> Dict[str, int]:
    """Bucket each changed number by its divisor-graph distance from n.

    The undirected graph joins u and v iff one divides the other; it is
    never materialized.  A vertex v's neighbours are its proper divisors
    (divs[v]) plus its multiples up to n.  BFS from n is capped at depth 4.
    """
    dist: Dict[int, int] = {n: 0}
    frontier = [n]
    depth = 0
    while frontier and depth < 4:
        depth += 1
        nxt: List[int] = []
        for v in frontier:
            neighbours = list(divs[v])
            neighbours.extend(range(2 * v, n + 1, v))
            for u in neighbours:
                if u not in dist:
                    dist[u] = depth
                    nxt.append(u)
        frontier = nxt

    buckets = {"1": 0, "2": 0, "3": 0, "4+": 0, "far": 0}
    for x in changed:
        d = dist.get(x)
        if d is None:
            buckets["far"] += 1
        elif d >= 4:
            buckets["4+"] += 1
        else:
            buckets[str(d)] += 1
    return buckets


# ---------------------------------------------------------------------------
# adapted incumbent (protocol step 3)
# ---------------------------------------------------------------------------

def build_adapted_incumbent(
    n: int,
    spf: Sequence[int],
    mf: Sequence[List[int]],
    prev_set: Set[int],
) -> Tuple[SetEval, int]:
    """Adapt game n-1's lower picks onto game n's optimal upper half.

    The upper half (provably optimal, from solve_upper_half) is loaded
    largest-first and must all fit.  Then game n-1's lower picks (by n's
    threshold) are added largest-first; the count that no longer fit is
    returned as adapt_dropped.
    """
    upper_seq, _tax_pool = solve_upper_half(n, spf)
    evaluator = SetEval(n, mf)
    for c in sorted(upper_seq, reverse=True):
        if not evaluator.playable_add(c):
            raise RuntimeError(
                f"upper-half selection {c} failed to load in game {n}"
            )

    lower_prev = sorted((x for x in prev_set if 2 * x <= n), reverse=True)
    adapt_dropped = 0
    for x in lower_prev:
        if not evaluator.playable_add(x):
            adapt_dropped += 1
    return evaluator, adapt_dropped


# ---------------------------------------------------------------------------
# geodesic test (protocol step 4)
# ---------------------------------------------------------------------------

def geodesic_test(
    evaluator: SetEval, n: int, cur_set: Set[int]
) -> Union[int, str]:
    """Walk the adapted incumbent to game n's lower set by minimal moves.

    D records the outstanding differences (add-role and remove-role lower
    numbers).  Each outer iteration takes the smallest strictly-improving
    move (single add, then swap-pair, then swap-triple) and restarts.  A
    single remove can never strictly improve on its own (it only drops a
    positive value), so removes are only attempted as part of a pair or
    triple; see the comment in the singles pass.  Returns the max move
    size used (0 if D started empty) or "blocked".
    """
    incumbent_lower = {s for s in evaluator.S if 2 * s <= n}
    target_lower = {s for s in cur_set if 2 * s <= n}
    D: Dict[int, str] = {}
    for x in target_lower - incumbent_lower:
        D[x] = "add"
    for x in incumbent_lower - target_lower:
        D[x] = "remove"

    if not D:
        return 0

    max_depth = 0
    while D:
        elems = sorted(D, reverse=True)

        # Singles.  A successful add of x (x not in S) strictly raises the
        # score by exactly x, so success alone is strictly improving.  A
        # lone remove only ever lowers the score (x is a positive member),
        # so it is never strictly improving by itself and is skipped here;
        # a remove is useful only paired with a larger add (see Pairs).
        moved = False
        for x in elems:
            if D[x] != "add":
                continue
            if evaluator.playable_add(x):
                del D[x]
                max_depth = max(max_depth, 1)
                moved = True
                break
        if moved:
            continue

        # Pairs.  Apply removes first, then adds (largest add first).  A
        # pair of two removes can never raise the score, so the strict
        # comparison rejects it naturally; no special-casing needed.
        for a, b in itertools.combinations(elems, 2):
            snap = evaluator.snapshot()
            combo = [a, b]
            removes = [x for x in combo if D[x] == "remove"]
            adds = sorted((x for x in combo if D[x] == "add"), reverse=True)
            for x in removes:
                evaluator.remove(x)
            ok = True
            for x in adds:
                if not evaluator.playable_add(x):
                    ok = False
                    break
            if ok and evaluator.score() > sum(snap[0]):
                del D[a]
                del D[b]
                max_depth = max(max_depth, 2)
                moved = True
                break
            evaluator.restore(snap)
        if moved:
            continue

        # Triples.
        for a, b, c in itertools.combinations(elems, 3):
            snap = evaluator.snapshot()
            combo = [a, b, c]
            removes = [x for x in combo if D[x] == "remove"]
            adds = sorted((x for x in combo if D[x] == "add"), reverse=True)
            for x in removes:
                evaluator.remove(x)
            ok = True
            for x in adds:
                if not evaluator.playable_add(x):
                    ok = False
                    break
            if ok and evaluator.score() > sum(snap[0]):
                del D[a]
                del D[b]
                del D[c]
                max_depth = max(max_depth, 3)
                moved = True
                break
            evaluator.restore(snap)
        if moved:
            continue

        return "blocked"

    return max_depth


# ---------------------------------------------------------------------------
# blind climb (protocol step 5)
# ---------------------------------------------------------------------------

def blind_climb(
    evaluator: SetEval, n: int, opt_score: int
) -> Tuple[int, int]:
    """Steepest-ascent single-flip hill climbing over the lower half.

    A single add of x raises the score by exactly +x; a single remove only
    lowers it.  So the steepest strictly-improving single flip is simply
    the largest lower non-pick x that playable_add accepts -- removes never
    help, and playable_add already rolls itself back on failure, so trying
    candidates largest-first and taking the first success needs no extra
    snapshotting.  Runs for at most 60 steps.
    """
    blind_steps = 0
    for _ in range(60):
        moved = False
        for x in range(n // 2, 0, -1):
            if x in evaluator.S:
                continue
            if evaluator.playable_add(x):
                blind_steps += 1
                moved = True
                break
        if not moved:
            break
    blind_gap = opt_score - evaluator.score()
    return blind_gap, blind_steps


# ---------------------------------------------------------------------------
# sanity gate (protocol step 6)
# ---------------------------------------------------------------------------

def sanity_reload(
    n: int,
    opt_score: int,
    opt_moves: Sequence[int],
    mf: Sequence[List[int]],
) -> bool:
    """Reload a known optimal set through SetEval largest-first.

    playable_add's own forced-retry loop is the required repair mechanism,
    so a plain add of each move (largest first) already exercises it.
    Returns False (never raises) if any add fails or the score mismatches.
    """
    evaluator = SetEval(n, mf)
    for x in sorted(opt_moves, reverse=True):
        if not evaluator.playable_add(x):
            return False
    return evaluator.score() == opt_score


# ---------------------------------------------------------------------------
# main / CLI / per-transition orchestration
# ---------------------------------------------------------------------------

def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--from", dest="from_n", type=int, default=500)
    parser.add_argument("--to", dest="to_n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--sample", type=int, default=1)
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    sys.setrecursionlimit(100_000)
    divs = divisor_lists(args.to_n)
    mf = maximal_factor_lists(args.to_n)
    spf = smallest_prime_factors(args.to_n)

    transitions: List[Dict[str, Any]] = []
    evaluator_failures: List[int] = []
    started = time.monotonic()
    analyzed = 0

    for n in range(args.from_n, args.to_n + 1, args.sample):
        if n not in optimal or (n - 1) not in optimal:
            print(f"n={n}: no known optimal solution, stopping", file=sys.stderr)
            break
        analyzed += 1

        opt_score_n = optimal[n]["score"]
        opt_score_prev = optimal[n - 1]["score"]

        if not sanity_reload(n, opt_score_n, optimal[n]["moves"], mf):
            evaluator_failures.append(n)

        prev_set = set(optimal[n - 1]["moves"])
        cur_set = set(optimal[n]["moves"])

        deltas = classify_deltas(n, prev_set, cur_set)
        changed = deltas["added_lower"] + deltas["removed_lower"]
        locality = locality_bfs(n, changed, divs)

        evaluator, adapt_dropped = build_adapted_incumbent(
            n, spf, mf, prev_set
        )
        incumbent_gap = opt_score_n - evaluator.score()
        if incumbent_gap == 0:
            certificate: Optional[str] = "gap0"
        elif evaluator.score() == n + opt_score_prev:
            certificate = "n_plus_prev"
        else:
            certificate = None

        geodesic_depth = geodesic_test(evaluator.clone(), n, cur_set)
        blind_gap, blind_steps = blind_climb(
            evaluator.clone(), n, opt_score_n
        )

        transitions.append({
            "n": n,
            "churn_lower": deltas["churn_lower"],
            "added_lower": deltas["added_lower"],
            "removed_lower": deltas["removed_lower"],
            "boundary": deltas["boundary"],
            "adapt_dropped": adapt_dropped,
            "incumbent_gap": incumbent_gap,
            "certificate": certificate,
            "locality": locality,
            "geodesic_depth": geodesic_depth,
            "blind_gap": blind_gap,
            "blind_steps": blind_steps,
        })

        if analyzed % 50 == 0:
            print(f"...through n={n} ({time.monotonic() - started:.0f}s)",
                  file=sys.stderr)

    out = MINI_DIR / "transitions.json"
    out.write_text(json.dumps(transitions, separators=(",", ":")))

    print_summary(transitions, evaluator_failures)
    print(f"\n{out.name}: {len(transitions)} records, {out.stat().st_size} "
          f"bytes ({time.monotonic() - started:.0f}s total)")
    return 0


def print_summary(
    transitions: List[Dict[str, Any]], evaluator_failures: List[int]
) -> None:
    total = len(transitions)

    print("=" * 60)
    print(f"EVALUATOR FAILURES: {len(evaluator_failures)} (expected 0)")
    if evaluator_failures:
        print(f"  offending n: {evaluator_failures}")
    print("=" * 60)
    if total == 0:
        return

    # Certificate rate.
    cert_counts = {"gap0": 0, "n_plus_prev": 0, None: 0}
    for t in transitions:
        cert_counts[t["certificate"]] += 1
    certified = cert_counts["gap0"] + cert_counts["n_plus_prev"]
    print("\nCertificate rate:")
    print(f"  certified: {certified}/{total} ({100 * certified / total:.1f}%)")
    print(f"    gap0:        {cert_counts['gap0']}")
    print(f"    n_plus_prev: {cert_counts['n_plus_prev']}")
    print(f"    none:        {cert_counts[None]}")

    # Incumbent gap distribution.
    gaps = [t["incumbent_gap"] for t in transitions]
    g0 = sum(1 for g in gaps if g == 0)
    g1 = sum(1 for g in gaps if 1 <= g <= 50)
    g2 = sum(1 for g in gaps if 51 <= g <= 200)
    g3 = sum(1 for g in gaps if g > 200)
    print("\nIncumbent gap distribution:")
    print(f"  ==0: {g0}   1-50: {g1}   51-200: {g2}   >200: {g3}")
    print(f"  mean gap: {sum(gaps) / total:.2f}")

    # Churn distribution.
    churn = [t["churn_lower"] for t in transitions]
    c0 = sum(1 for c in churn if c == 0)
    c1 = sum(1 for c in churn if c == 1)
    c2 = sum(1 for c in churn if 2 <= c <= 5)
    c3 = sum(1 for c in churn if 6 <= c <= 15)
    c4 = sum(1 for c in churn if c >= 16)
    print("\nChurn (lower) distribution:")
    print(f"  min={min(churn)} mean={sum(churn) / total:.2f} max={max(churn)}")
    print(f"  0: {c0}   1: {c1}   2-5: {c2}   6-15: {c3}   16+: {c4}")

    # Locality histogram.
    loc_totals = {"1": 0, "2": 0, "3": 0, "4+": 0, "far": 0}
    for t in transitions:
        for k in loc_totals:
            loc_totals[k] += t["locality"][k]
    total_changed = sum(loc_totals.values())
    print("\nLocality histogram (changed lower numbers by divisor-distance "
          "from n):")
    print(f"  1: {loc_totals['1']}   2: {loc_totals['2']}   "
          f"3: {loc_totals['3']}   4+: {loc_totals['4+']}   "
          f"far: {loc_totals['far']}")
    if total_changed:
        near = loc_totals["1"] + loc_totals["2"]
        print(f"  within distance <=2 of n: {near}/{total_changed} "
              f"({100 * near / total_changed:.1f}%)")

    # Geodesic depth histogram (headline).
    geo_counts: Dict[Union[int, str], int] = {0: 0, 1: 0, 2: 0, 3: 0,
                                               "blocked": 0}
    for t in transitions:
        geo_counts[t["geodesic_depth"]] += 1
    print("\n*** GEODESIC DEPTH HISTOGRAM (headline) ***")
    print(f"  0: {geo_counts[0]}   1: {geo_counts[1]}   2: {geo_counts[2]}   "
          f"3: {geo_counts[3]}   blocked: {geo_counts['blocked']}")

    # Blind climb.
    blind_zero = sum(1 for t in transitions if t["blind_gap"] == 0)
    nonzero = [t["blind_gap"] for t in transitions if t["blind_gap"] != 0]
    print("\nBlind climb:")
    print(f"  blind_gap==0: {blind_zero}/{total} "
          f"({100 * blind_zero / total:.1f}%)")
    if nonzero:
        print(f"  mean residual gap (nonzero cases): "
              f"{sum(nonzero) / len(nonzero):.2f}")

    # adapt_dropped distribution.
    drop = [t["adapt_dropped"] for t in transitions]
    d0 = sum(1 for d in drop if d == 0)
    d1 = sum(1 for d in drop if d == 1)
    d2 = sum(1 for d in drop if d == 2)
    d3 = sum(1 for d in drop if d >= 3)
    print("\nadapt_dropped distribution:")
    print(f"  0: {d0}   1: {d1}   2: {d2}   3+: {d3}")


if __name__ == "__main__":
    sys.exit(main())
