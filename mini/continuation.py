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

Certificates. Three record-level labels certify a score as provably (or,
for the third, conjecturally) optimal given the previous game's score,
without re-solving game n from scratch:

  * "exact" -- the incumbent (or final) score equals n + prev_score, i.e.
    prepending n to the previous game's own solution is already optimal
    (opt(n) <= n + opt(n-1) always; equality closes the gap).
  * "prime" -- the "prime sacrifice" identity from the wiki page "Reusing a
    previous solution" (https://github.com/bvchess/taxman/wiki/Reusing-a-
    previous-solution, sections "If N is prime" and "Generalizing"),
    validated 167/167 against optimal.json: for prime N,
    opt(N) = N + opt(N-1) - p_hat, where p_hat is the largest prime < N.
    Sketch: 1 divides every number, so it is consumed by any game's first
    move; a prime's only proper divisor is 1, so any solution containing a
    prime plays it first and contains at most one prime. Taking prime N as
    a selection therefore forces sacrificing whatever prime the previous
    solution played (p_hat, the largest prime below N -- largest because
    swapping any smaller prime up to p_hat into that slot is always
    feasible and only improves the score). Conversely, prepending N to an
    optimal (N-1) solution with its prime removed is always legal, so the
    bound is tight.
  * "upper-delta" -- CONJECTURE-GRADE, no theorem behind it. Let U*(k) be
    the sum of solve_upper_half(k, spf)'s returned sequence (the provably
    optimal upper half of game k) and d_upper(n) = U*(n) - U*(n-1). The
    certificate fires when score == prev_score + d_upper(n), or -- for
    even n -- score == prev_score + d_upper(n) + n//2, the extra term
    covering the boundary-crosser n/2: on an even game, n/2 sits exactly on
    the upper/lower boundary of game n-1 (excluded, since the upper half is
    strictly greater than (n-1)/2) but can be re-picked as a *lower* number
    of game n once n/2 <= n/2, so a legal transition may carry it across as
    a lower pick in addition to the upper-half delta. Validated (scratchpad
    eviction_cert_check.py) as zero-harmful over all 998 cold-chain
    transitions at n<=1000 (gap never increases when it fires), and the
    underlying identity holds on 70.4% of ground-truth optimal.json
    transitions -- strong empirical support, but unlike "exact" and
    "prime" it has no correctness proof, so it is kept as a separate,
    lower-confidence label.

  "exact" and "prime" are, empirically, special cases of the upper-delta
  identity (zero eviction from the upper half, and prime-evicts-prime,
  respectively) but are kept as their own labels because they are proven;
  "upper-delta" is the catch-all for the conjectured, unproven remainder.
  All three certificates are label-only: they are computed from the
  *final* settled score after all search tiers and the solvent re-anchor,
  and never gate or skip the search itself (see solve_game), with the sole
  exception of the lucky "exact" early exit described there.

Usage:
    python3 continuation.py --from 500 --to 1000 [--seed-from-optimal]
                            [--bundle-limit K] [--out PATH] [--resume]

--out is checkpointed every 5 games (and at the end) with each record's
"_set" and "_upper_sum" retained, so a container restart mid-run can be
recovered from with --resume: it loads the checkpoint, verifies it holds
consecutive games starting at --from, and continues from where it left
off. The final output
file has "_set" and "_upper_sum" stripped from every record, same as a
non-resumed run.
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

from approx import check_sequence, maximal_factor_lists, solvent
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
        if evaluator.mf[x] and gcd(x, n) > 1
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
         if x not in evaluator.S and evaluator.mf[x] and gcd(x, n) > 1),
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
    mf: Sequence[List[int]],
    prev_set: Set[int],
    prev_score: Optional[int],
    prev_upper_sum: Optional[int],
    bundle_limit: int,
    reanchor_solvent: bool,
    optimal: Dict[int, Dict[str, Any]],
    largest_prime_below: Sequence[Optional[int]],
) -> Dict[str, Any]:
    """Solve game n warm-started from the previous solution; return a record."""
    t0 = time.monotonic()
    evaluator = SetEval(n, mf)

    # Step 1: seed the provably-optimal, always-playable upper half. Its sum
    # is U*(n), reused below (rather than recomputed) for the "upper-delta"
    # certificate.
    upper_seq, _ = solve_upper_half(n, spf)
    upper_sum = sum(upper_seq)
    for c in sorted(upper_seq, reverse=True):
        if not evaluator.playable_add(c):
            raise RuntimeError(f"upper-half selection {c} failed at n={n}")

    # Step 2: carry the previous solution's lower picks, skipping failures.
    for x in sorted((m for m in prev_set if 2 * m <= n), reverse=True):
        evaluator.playable_add(x)

    incumbent_score = evaluator.score()
    lucky_exit = False
    tier = 0
    flips = 0

    # Step 3: lucky certificate -- an early exit that skips the search
    # entirely. This is the ONLY certificate allowed to change search
    # behavior; see the "prime" certificate below for why it stays label-only.
    if prev_score is not None and incumbent_score == n + prev_score:
        lucky_exit = True
    else:
        # Step 4: tier-1 hill climbing always runs to quiescence first,
        # independent of the bundle budget -- `--bundle-limit 0` must mean
        # "flips yes, bundles no", not "no local search at all".
        f, _failed_adds = tier1(evaluator, n)
        flips += f
        if f > 0:
            tier = max(tier, 1)

        # Step 5: interleave tier-2 bundles with tier-1 re-convergence,
        # bounded by the bundle budget. bundle_limit=0 means this loop
        # never executes.
        budget = [bundle_limit]
        while budget[0] > 0:
            if tier2_pass(evaluator, n, budget):
                tier = max(tier, 2)
                f, _failed_adds = tier1(evaluator, n)
                flips += f
                if f > 0:
                    tier = max(tier, 1)
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

    source = "chain"
    output_set = evaluator.S
    if reanchor_solvent:
        solvent_seq = solvent(n, mf)
        solvent_score = check_sequence(n, solvent_seq)  # validates solvent's own sequence
        if solvent_score > our_score:
            solvent_set = set(solvent_seq)
            assert solvent_score == sum(solvent_set)
            our_score = solvent_score
            output_set = solvent_set
            source = "solvent"

    prev_lower = {m for m in prev_set if 2 * m <= n}
    our_lower = {m for m in output_set if 2 * m <= n}
    churn = len(prev_lower ^ our_lower)

    # Certificates, computed on the FINAL settled score (after tiers and the
    # solvent re-anchor adoption above) -- all three are label-only postmortem
    # checks, never a search cap. In a cold chain prev_score may itself be
    # suboptimal, so n + prev_score (- p_hat) computed from a suboptimal
    # prev is not a valid upper bound on opt(n); cutting search on it could
    # forfeit points. Labeling only after the fact costs nothing and can
    # never be wrong. (lucky_exit above is the one exception: it short-
    # circuits the search, but only on the same "score == n + prev_score"
    # condition re-checked here, so it always ends up labeled "exact".)
    #
    # Precedence: "exact", else "prime", else "upper-delta" -- the first two
    # are proven identities; "upper-delta" (see module docstring) is
    # CONJECTURE-GRADE, validated zero-harmful over all cold-chain
    # transitions at n<=1000 and holding on 70.4% of ground-truth
    # optimal.json transitions, but with no correctness proof.
    certificate: Optional[str] = None
    if prev_score is not None and our_score == n + prev_score:
        certificate = "exact"
    elif (
        prev_score is not None
        and n >= 2
        and spf[n] == n  # n is prime
        and largest_prime_below[n] is not None
        and our_score == n + prev_score - largest_prime_below[n]
    ):
        certificate = "prime"
    elif prev_score is not None and prev_upper_sum is not None:
        d_upper = upper_sum - prev_upper_sum
        if our_score == prev_score + d_upper:
            certificate = "upper-delta"
        elif n % 2 == 0 and our_score == prev_score + d_upper + n // 2:
            certificate = "upper-delta"
    certified = certificate is not None

    record: Dict[str, Any] = {
        "n": n,
        "score": our_score,
        "certified": certified,
        "certificate": certificate,
        "tier": tier,
        "flips": flips,
        "churn_from_prev": churn,
        "source": source,
        "time_s": round(time.monotonic() - t0, 4),
        "_set": sorted(output_set),  # carried forward; stripped before output
        "_upper_sum": upper_sum,  # U*(n); carried forward, stripped before output
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
    reanchor_solvent: bool,
    optimal: Dict[int, Dict[str, Any]],
    spf: Sequence[int],
    mf: Sequence[List[int]],
    out_path: Optional[Path] = None,
    initial_records: Optional[List[Dict[str, Any]]] = None,
) -> List[Dict[str, Any]]:
    """Solve games from_n..to_n sequentially, each fed the previous result.

    If out_path is given, the growing records list (with each record's
    "_set" still attached) is checkpointed to out_path every 5 games and
    after the final game, so an interrupted run leaves a resumable file
    behind. If initial_records is given and non-empty (a checkpoint loaded
    by main), the loop resumes right after its last game instead of
    starting at from_n: prev_set/prev_score are seeded from that last
    record and it is extended in place -- seed_from_optimal is ignored in
    that case, since the checkpoint seed wins.
    """
    # Prime-sacrifice certificate lookup: largest_prime_below[n] = the
    # largest prime q < n, or None if none exists. Cheap O(range) pass over
    # spf, computed once here where spf is already in scope.
    limit = len(spf) - 1
    largest_prime_below: List[Optional[int]] = [None] * (limit + 1)
    last_prime: Optional[int] = None
    for k in range(limit + 1):
        largest_prime_below[k] = last_prime
        if k >= 2 and spf[k] == k:
            last_prime = k

    if initial_records:
        records: List[Dict[str, Any]] = list(initial_records)
        last = records[-1]
        prev_set: Set[int] = set(last["_set"])
        prev_score: Optional[int] = last["score"]
        # "_upper_sum" is carried in checkpoints the same way "_set" is; fall
        # back to recomputing U*(last n) for checkpoints written before this
        # field existed.
        if "_upper_sum" in last:
            prev_upper_sum: Optional[int] = last["_upper_sum"]
        else:
            prev_upper_sum = sum(solve_upper_half(last["n"], spf)[0])
        start_n = last["n"] + 1
    else:
        records = []
        start_n = from_n
        if seed_from_optimal:
            if (from_n - 1) not in optimal:
                raise SystemExit(
                    f"--seed-from-optimal needs optimal.json to cover n={from_n - 1}"
                )
            prev_set = set(optimal[from_n - 1]["moves"])
            prev_score = optimal[from_n - 1]["score"]
        else:
            prev_set = set()
            prev_score = None
        # U*(from_n - 1), the upper-delta certificate's starting point.
        # solve_upper_half handles n<=0 gracefully (empty selection, sum 0),
        # so this naturally gives U*(1) = 0 and U*(2) = 2 with no special
        # casing -- covers --from 2 (prev game n=1 has U*=0) for free.
        prev_upper_sum = sum(solve_upper_half(from_n - 1, spf)[0])

    started = time.monotonic()
    for i, n in enumerate(range(start_n, to_n + 1), 1):
        rec = solve_game(n, spf, mf, prev_set, prev_score, prev_upper_sum,
                          bundle_limit, reanchor_solvent, optimal,
                          largest_prime_below)
        prev_set = set(rec["_set"])
        prev_score = rec["score"]
        prev_upper_sum = rec["_upper_sum"]
        records.append(rec)
        if i % 50 == 0:
            print(f"...through n={n} ({time.monotonic() - started:.0f}s)",
                  file=sys.stderr)
        if out_path is not None and (i % 5 == 0 or n == to_n):
            out_path.write_text(json.dumps(records, separators=(",", ":")))
    return records


# ---------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------

def print_summary(
    records: List[Dict[str, Any]], elapsed: float, seeded: bool, reanchor: bool
) -> None:
    total = len(records)
    print("=" * 64)
    print(f"CONTINUATION SOLVER: {total} games "
          f"(seed = {'optimal.json' if seeded else 'cold/empty'}, "
          f"reanchor-solvent = {'on' if reanchor else 'off'})")
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

    # Certificate rate, broken down by certificate kind.
    cert = sum(1 for r in records if r["certified"])
    cert_exact = sum(1 for r in records if r.get("certificate") == "exact")
    cert_prime = sum(1 for r in records if r.get("certificate") == "prime")
    cert_upper = sum(1 for r in records if r.get("certificate") == "upper-delta")
    print(f"\nCertificate rate: {cert}/{total} "
          f"({100 * cert / total:.1f}%) -- "
          f"exact {cert_exact}, prime-sacrifice {cert_prime}, "
          f"upper-delta {cert_upper}")

    # Solvent re-anchor adoptions.
    adopted = sum(1 for r in records if r.get("source") == "solvent")
    print(f"Solvent re-anchor adoptions: {adopted}/{total} "
          f"({100 * adopted / total:.1f}%)")

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
    parser.add_argument("--reanchor-solvent", action="store_true", dest="reanchor_solvent",
                        help="after solving game n, also try the solvent strategy's "
                             "solution and adopt it (score + pick set) if it beats "
                             "the chain's solution")
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--out", type=Path,
                        default=MINI_DIR / "continuation_results.json")
    parser.add_argument("--resume", action="store_true",
                        help="resume from an existing --out checkpoint left "
                             "by an interrupted run")
    args = parser.parse_args(argv)

    sys.setrecursionlimit(100_000)
    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    spf = smallest_prime_factors(args.to_n)
    mf = maximal_factor_lists(args.to_n)

    initial_records: Optional[List[Dict[str, Any]]] = None
    if args.resume:
        if not args.out.exists() or args.out.stat().st_size == 0:
            raise SystemExit(
                f"--resume: {args.out} does not exist or is empty"
            )
        loaded = json.loads(args.out.read_text())
        if loaded:
            expected_n = args.from_n
            for r in loaded:
                if r["n"] != expected_n:
                    raise SystemExit(
                        f"--resume: {args.out} does not hold consecutive "
                        f"games starting at --from {args.from_n} (expected "
                        f"n={expected_n}, found n={r['n']})"
                    )
                expected_n += 1
            if "_set" not in loaded[-1]:
                raise SystemExit(
                    f"--resume: {args.out} has no \"_set\" in its last "
                    "record -- it looks like a finished (stripped) output "
                    "file, not a checkpoint left by an interrupted run."
                )
            if loaded[-1]["n"] >= args.to_n:
                print(f"{args.out}: already complete through "
                      f"n={loaded[-1]['n']} >= --to {args.to_n}; nothing "
                      "to resume.")
            initial_records = loaded

    started = time.monotonic()
    records = run(args.from_n, args.to_n, args.seed_from_optimal,
                  args.bundle_limit, args.reanchor_solvent, optimal, spf, mf,
                  out_path=args.out, initial_records=initial_records)
    elapsed = time.monotonic() - started

    for r in records:
        r.pop("_set", None)
        r.pop("_upper_sum", None)
    args.out.write_text(json.dumps(records, separators=(",", ":")))
    print_summary(records, elapsed, args.seed_from_optimal, args.reanchor_solvent)
    print(f"\n{args.out}: {len(records)} records, "
          f"{args.out.stat().st_size} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
