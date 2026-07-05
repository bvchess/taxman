"""Continuation solver: solve Taxman game n by warm-starting from n-1.

The transition anatomy (transitions.py) showed that the optimal solution
of game n sits a tiny, local perturbation away from the optimal solution
of game n-1: keep the provably-optimal upper half, carry the previous
game's lower picks, and a shallow flip/bundle search closes the rest.

This module turns that measurement into a *sequential self-fed solver*.
Games are solved in order; each game is warm-started from the previous
game's OWN produced solution (not from optimal.json).  optimal.json, when
present, is used only to score the result -- never to guide the search.
Feeding the solver its own output is the drift experiment: does the chain
stay pinned to the optimum, or does small per-game error accumulate?

Per game n (given the previous solution's pick set and score):

  1. Seed the exact upper half U*(n) from solve_upper_half (provably
     optimal, always playable).
  2. Carry the previous solution's lower picks (2*m <= n), largest-first,
     skipping any that no longer fit -- the "incumbent".
  3. Certificate: if the incumbent already scores n + score(n-1), it is
     optimal given game n-1 (opt(n) <= n + opt(n-1)); record and stop.
  4. Tier-1: steepest-ascent single flips (the largest playable lower add)
     until none improves.
  5. Tier-2: coupled add/remove bundles over the blocked adds -- the
     measured valley signature (e.g. n=507: add {198,182} remove
     {154,220}, net +6) that no single flip can cross.

Every produced solution is validated by deriving a real-game order from
the matching and replaying it under the true rules (approx.check_sequence)
to reproduce the recorded score.

Usage:
    python3 continuation.py --from 500 --to 1000 [--seed-from-optimal]
                            [--bundle-limit K] [--out PATH]
"""

from __future__ import annotations

import argparse
import itertools
import json
import sys
import time
from math import gcd
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

from approx import check_sequence, divisor_lists
from seteval import SetEval
from taxman_mini import smallest_prime_factors, solve_upper_half
from verify import DEFAULT_OPTIMAL

MINI_DIR = Path(__file__).resolve().parent

TIER1_STEP_CAP = 80
TIER2_REFILL_K = 30


# ---------------------------------------------------------------------------
# real-game ordering (correctness gate C)
# ---------------------------------------------------------------------------

def derive_order(n: int, S: Set[int], match: Dict[int, int]) -> List[int]:
    """A legal real-game order for a playable set, from its matching.

    Topologically sorts the full playability precedence -- "a before b
    whenever a's assigned coupon divides b, OR a divides b" -- exactly the
    relation SetEval's acyclicity check verifies.  (order_for_real_game in
    taxman_mini only encodes the coupon edges, which suffices for the
    upper half where no pick divides another, but a full game also needs
    the pick-divides-pick edges so a claimed number is not swept as tax by
    a larger pick played first.)  A valid, acyclic matching always yields a
    topological order.
    """
    indeg = {c: 0 for c in S}
    succ: Dict[int, List[int]] = {c: [] for c in S}
    for a in S:
        seen: Set[int] = set()
        for start in (match[a], a):
            for b in range(2 * start, n + 1, start):
                if b in S and b != a and b not in seen:
                    seen.add(b)
                    succ[a].append(b)
                    indeg[b] += 1
    ready = [c for c in S if indeg[c] == 0]
    order: List[int] = []
    while ready:
        a = ready.pop()
        order.append(a)
        for b in succ[a]:
            indeg[b] -= 1
            if indeg[b] == 0:
                ready.append(b)
    if len(order) != len(S):
        raise RuntimeError(f"cyclic precedence for game {n}; set unplayable")
    return order


# ---------------------------------------------------------------------------
# tier 1: steepest-ascent single flips
# ---------------------------------------------------------------------------

def tier1(evaluator: SetEval, n: int) -> Tuple[int, List[int]]:
    """Hill-climb by the single steepest improving add until none exists.

    A single add of x raises the score by exactly +x; a single remove only
    lowers it and so is never strictly improving on its own.  The steepest
    improving add is therefore the largest addable lower non-pick, so we
    sweep descending and take the first acceptance (playable_add rolls
    itself back on failure, so no snapshotting is needed).

    The sweep is restricted to the local candidates -- lower non-picks x
    with gcd(x, n) > 1 -- because the perturbation from adding n to the pot
    lands within divisor-distance 2 of n (the transition anatomy's measured
    locality), so any free add that helps shares a prime factor with n.
    That keeps the convergence sweep (proving no add improves, the common
    and otherwise expensive case) to ~80 candidates instead of ~n/2.
    Returns (flips_taken, cap_hit) with cap_hit unused by callers.
    """
    local = [
        x for x in range(n // 2, 1, -1)
        if evaluator.divs[x] and gcd(x, n) > 1
    ]
    flips = 0
    while flips < TIER1_STEP_CAP:
        moved = False
        for x in local:
            if x in evaluator.S:
                continue
            if evaluator.playable_add(x):
                flips += 1
                moved = True
                break
        if not moved:
            break
    return flips, []


# ---------------------------------------------------------------------------
# tier 2: coupled add/remove bundles over the blocked adds
# ---------------------------------------------------------------------------

def tier2_pass(
    evaluator: SetEval,
    n: int,
    budget: List[int],
) -> bool:
    """Find and apply one improving remove-and-refill bundle; True if found.

    The measured valley signature is a coupled set of lower add/remove
    moves with a small positive net that no single flip can cross (e.g.
    n=500: remove 189, add 210, net +21; n=507: remove {154,220}, add
    {198,182}, net +6; n=520: remove 192, add 240, net +48).  The search is
    anchored on *removals* rather than adds, which is what makes it both
    correct and cheap:

    * There are only ~24 lower picks, so single and pair removals total
      ~C(24,2) ~ 300 combos regardless of how many hundreds of lower
      numbers n admits -- far fewer than iterating candidate adds, and
      independent of n's density.

    * After removing a combo R, the picks that can newly become playable
      are exactly those sharing a prime factor with something in R (removing
      R frees R's coupons and slots, which only matter to numbers built from
      the same primes).  Locality from the transition anatomy holds here as
      the refill set: local blocked adds (gcd(x, n) > 1) that also share a
      factor with R.  Refilling them largest-value first recovers the
      partner adds of the valley (the +240 for -192, the {198,182} for
      {154,220}).

    A combo is kept when the remove-and-refill strictly raises the score;
    removals reaching *through* the matching or the precedence relation are
    both covered, since a real improving refill is only accepted if the
    whole set replays.  Singles are tried before pairs and the first
    improving combo is applied, so each pass makes one correction and tier-1
    re-runs.  `budget[0]` caps combo evaluations per game and is decremented
    in place.
    """
    lower_picks = sorted(p for p in evaluator.S if 2 * p <= n)
    local_blocked = sorted(
        (x for x in range(2, n // 2 + 1)
         if x not in evaluator.S and evaluator.divs[x] and gcd(x, n) > 1),
        reverse=True,
    )
    if not lower_picks or not local_blocked:
        return False

    # For each removable pick, the local blocked adds it shares a factor
    # with: the only adds a refill after removing it can recruit, since
    # removing a pick frees only coupons/slots built from its own primes.
    share: Dict[int, List[int]] = {
        r: [x for x in local_blocked if gcd(x, r) > 1] for r in lower_picks
    }

    base = evaluator.snapshot()
    before = sum(base[0])

    def refill(cands: List[int]) -> None:
        # Descending value is the load-bearing order: a big blocked add that
        # is structurally unplayable simply fails and leaves the state
        # untouched, so the smaller genuine beneficiary later in the list
        # still finds its coupon free.  The full playable_add (with the
        # solve_mini completeness tier) is required -- a valley beneficiary
        # such as 189 after removing {98,210} is not greedily addable, only
        # completely so.
        for x in cands:
            if x not in evaluator.S:
                evaluator.playable_add(x)

    def candidates(removed: Tuple[int, ...], pool: List[int]) -> List[int]:
        # Keep only the strongest candidates by shared-factor size with the
        # removed picks, then order that shortlist by value for the refill.
        # A valley beneficiary shares a *large* gcd with what it displaces
        # (240/192 share 48, 210/189 share 21, 198/154 share 22), whereas
        # the many always-blocked large adds share only a factor of 2 with
        # the removal and never become playable -- shortlisting by gcd drops
        # them so the solve_mini-heavy refill runs on ~K, not ~150,
        # candidates, without losing the beneficiary.
        ranked = sorted(
            pool, key=lambda x: max(gcd(x, r) for r in removed), reverse=True
        )[:TIER2_REFILL_K]
        ranked.sort(reverse=True)
        return ranked

    def try_combo(removed: Tuple[int, ...], pool: List[int]) -> bool:
        if not pool:
            return False
        budget[0] -= 1
        evaluator.restore(base)
        for r in removed:
            evaluator.remove(r)
        refill(candidates(removed, pool))
        return evaluator.score() > before

    # Singles first (one correctly-priced swap), then pairs (the coupled
    # 2-for-2 valley moves).  First improving combo wins the pass.
    for r in lower_picks:
        if budget[0] <= 0:
            break
        if try_combo((r,), share[r]):
            return True
    for r1, r2 in itertools.combinations(lower_picks, 2):
        if budget[0] <= 0:
            break
        merged = share[r1] + [x for x in share[r2] if gcd(x, r1) <= 1]
        if try_combo((r1, r2), merged):
            return True

    evaluator.restore(base)
    return False


# ---------------------------------------------------------------------------
# per-game solve
# ---------------------------------------------------------------------------

def solve_game(
    n: int,
    spf: Sequence[int],
    divs: Sequence[List[int]],
    prev_set: Set[int],
    prev_score: Optional[int],
    bundle_limit: int,
    optimal: Dict[int, Dict[str, Any]],
) -> Dict[str, Any]:
    """Solve game n warm-started from the previous solution; return a record."""
    t0 = time.monotonic()
    evaluator = SetEval(n, divs)

    # Step 1: seed the provably-optimal, always-playable upper half.
    upper_seq, _ = solve_upper_half(n, spf)
    for c in sorted(upper_seq, reverse=True):
        if not evaluator.playable_add(c):
            raise RuntimeError(f"upper-half selection {c} failed at n={n}")

    # Step 2: carry the previous solution's lower picks, skipping failures.
    for x in sorted((m for m in prev_set if 2 * m <= n), reverse=True):
        evaluator.playable_add(x)

    incumbent_score = evaluator.score()
    certified = False
    tier = 0
    flips = 0

    # Step 3: certificate.
    if prev_score is not None and incumbent_score == n + prev_score:
        certified = True
    else:
        # Steps 4-5: interleave tier-1 hill climbing and tier-2 bundles.
        budget = [bundle_limit]
        while budget[0] > 0:
            f, _failed_adds = tier1(evaluator, n)
            flips += f
            if f > 0:
                tier = max(tier, 1)
            if tier2_pass(evaluator, n, budget):
                tier = max(tier, 2)
            else:
                break

    our_score = evaluator.score()

    # Gate C: validate by replaying a derived real-game order.
    order = derive_order(n, evaluator.S, evaluator.match)
    replay_score = check_sequence(n, order)
    assert replay_score == our_score == sum(evaluator.S), (
        f"replay mismatch at n={n}: replay={replay_score} "
        f"score={our_score} sum={sum(evaluator.S)}"
    )

    prev_lower = {m for m in prev_set if 2 * m <= n}
    our_lower = {m for m in evaluator.S if 2 * m <= n}
    churn = len(prev_lower ^ our_lower)

    record: Dict[str, Any] = {
        "n": n,
        "score": our_score,
        "certified": certified,
        "tier": tier,
        "flips": flips,
        "churn_from_prev": churn,
        "time_s": round(time.monotonic() - t0, 4),
        "_set": sorted(evaluator.S),  # carried forward; stripped before output
    }
    if n in optimal:
        opt = optimal[n]["score"]
        record["opt"] = opt
        record["gap"] = opt - our_score
    return record


# ---------------------------------------------------------------------------
# driver
# ---------------------------------------------------------------------------

def run(
    from_n: int,
    to_n: int,
    seed_from_optimal: bool,
    bundle_limit: int,
    optimal: Dict[int, Dict[str, Any]],
    spf: Sequence[int],
    divs: Sequence[List[int]],
) -> List[Dict[str, Any]]:
    """Solve games from_n..to_n sequentially, each fed the previous result."""
    if seed_from_optimal:
        if (from_n - 1) not in optimal:
            raise SystemExit(
                f"--seed-from-optimal needs optimal.json to cover n={from_n - 1}"
            )
        prev_set: Set[int] = set(optimal[from_n - 1]["moves"])
        prev_score: Optional[int] = optimal[from_n - 1]["score"]
    else:
        prev_set = set()
        prev_score = None

    records: List[Dict[str, Any]] = []
    started = time.monotonic()
    for i, n in enumerate(range(from_n, to_n + 1), 1):
        rec = solve_game(n, spf, divs, prev_set, prev_score, bundle_limit, optimal)
        prev_set = set(rec.pop("_set"))
        prev_score = rec["score"]
        records.append(rec)
        if i % 50 == 0:
            print(f"...through n={n} ({time.monotonic() - started:.0f}s)",
                  file=sys.stderr)
    return records


# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------

def print_summary(
    records: List[Dict[str, Any]], elapsed: float, seeded: bool
) -> None:
    total = len(records)
    print("=" * 64)
    print(f"CONTINUATION SOLVER: {total} games "
          f"(seed = {'optimal.json' if seeded else 'cold/empty'})")
    print("=" * 64)
    if total == 0:
        return

    scored = [r for r in records if "gap" in r]

    # Exact-match rate + gap distribution vs optimal.json.
    if scored:
        exact = sum(1 for r in scored if r["gap"] == 0)
        gaps = [r["gap"] for r in scored]
        g0 = sum(1 for g in gaps if g == 0)
        g1 = sum(1 for g in gaps if 1 <= g <= 20)
        g2 = sum(1 for g in gaps if 21 <= g <= 100)
        g3 = sum(1 for g in gaps if g > 100)
        neg = sum(1 for g in gaps if g < 0)
        print(f"\nExact matches vs optimal: {exact}/{len(scored)} "
              f"({100 * exact / len(scored):.1f}%)")
        print("Gap distribution (opt - ours):")
        print(f"  ==0: {g0}   1-20: {g1}   21-100: {g2}   >100: {g3}"
              + (f"   <0(beats opt!): {neg}" if neg else ""))
        print(f"  mean gap: {sum(gaps) / len(gaps):.2f}   max gap: {max(gaps)}")

        # MEAN GAP BY 100-BAND -- the drift/accumulation curve (headline).
        print("\n*** MEAN GAP BY 100-BAND (drift curve, headline) ***")
        bands: Dict[int, List[int]] = {}
        for r in scored:
            bands.setdefault((r["n"] // 100) * 100, []).append(r["gap"])
        for lo in sorted(bands):
            gs = bands[lo]
            exact_b = sum(1 for g in gs if g == 0)
            print(f"  {lo:4d}-{lo + 99:<4d}: mean={sum(gs) / len(gs):7.2f}  "
                  f"max={max(gs):5d}  exact={exact_b}/{len(gs)}")
    else:
        print("\n(no optimal.json coverage in this range; gaps unavailable)")

    # Certificate rate.
    cert = sum(1 for r in records if r["certified"])
    print(f"\nCertificate rate: {cert}/{total} "
          f"({100 * cert / total:.1f}%)")

    # Tier usage histogram.
    tiers = {0: 0, 1: 0, 2: 0}
    for r in records:
        tiers[r["tier"]] += 1
    print("Tier usage (deepest tier producing the solution):")
    print(f"  tier 0 (insertion/cert): {tiers[0]}   "
          f"tier 1 (flips): {tiers[1]}   tier 2 (bundles): {tiers[2]}")

    # Churn and timing.
    churn = [r["churn_from_prev"] for r in records]
    times = [r["time_s"] for r in records]
    print(f"\nChurn from prev (lower symmetric diff): "
          f"mean={sum(churn) / total:.2f} max={max(churn)}")
    print(f"Per-game time: mean={sum(times) / total:.3f}s "
          f"max={max(times):.3f}s")
    print(f"Total runtime: {elapsed:.1f}s")


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--from", dest="from_n", type=int, default=500)
    parser.add_argument("--to", dest="to_n", type=int, default=1000)
    parser.add_argument("--seed-from-optimal", action="store_true",
                        help="start the chain from optimal.json's (from-1) "
                             "solution (the standard experiment)")
    parser.add_argument("--bundle-limit", type=int, default=2000,
                        help="max tier-2 bundle evaluations per game")
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--out", type=Path,
                        default=MINI_DIR / "continuation_results.json")
    args = parser.parse_args(argv)

    sys.setrecursionlimit(100_000)
    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    spf = smallest_prime_factors(args.to_n)
    divs = divisor_lists(args.to_n)

    started = time.monotonic()
    records = run(args.from_n, args.to_n, args.seed_from_optimal,
                  args.bundle_limit, optimal, spf, divs)
    elapsed = time.monotonic() - started

    args.out.write_text(json.dumps(records, separators=(",", ":")))
    print_summary(records, elapsed, args.seed_from_optimal)
    print(f"\n{args.out}: {len(records)} records, "
          f"{args.out.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
