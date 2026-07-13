"""
This is a readable reference implementation of the continuation solver
- the strategy that plays every game n by warm-starting from game n-1's
own answer instead of solving from scratch.  As with reference/solvent.py,
the code favors clarity over speed: it re-derives everything from
scratch each game rather than maintaining the fast code's incremental
matching state (strategies/seteval.py's SetEval), so a single call
here costs what several thousand calls to reference.solvent.playable
cost.  That is the accepted trade for a reference file - it exists to
be read, not raced.  See the bottom of this docstring for how slow
"slow" actually is.

The game, and the membership-first idea behind solvent, are described
in reference/solvent.py; read that file first, since this one imports
and calls it directly (playable, ordered, build_maximal_factor_table)
rather than re-deriving playability testing from scratch.

The strategy, game by game, seeded by game n-1's own produced solution
(never by an externally known optimum):

1. Seed the upper half: the numbers greater than n/2 are exactly the
   candidates that cannot divide each other in the pot, so a plain
   greedy scan - largest candidate first, kept whenever solvent's
   playable() still accepts it - reproduces the same upper-half set as
   core.solve_upper_half's specialized matching code (both are exact
   playability oracles over the same restricted game; see THEORY.md's
   playability characterization).  upper_half_selection() does this.
2. Carry forward game n-1's own lower-half picks (the numbers <= n/2),
   largest first, keeping whichever still fit.
3. A lucky exit: if that carried-forward set already scores exactly
   n + (game n-1's score), it is provably optimal given game n-1 (see
   the "exact" certificate below) and the search below is skipped.
4. Otherwise, tier 1: repeatedly add the single largest lower number
   that keeps the set playable, until none does (steepest-ascent hill
   climbing).
5. Tier 2: when tier 1 stalls, try small "remove a lower pick or two,
   then refill" bundles - the only way past a local optimum that no
   single add can climb.  Removals are the anchor because there are
   only a couple of dozen lower picks to try removing, far fewer than
   there are candidate numbers to try adding.
6. Re-anchor: also run reference.solvent's own solvent() on n from
   scratch, and adopt its set if it scores higher.  This is a floor
   (the chain can never do worse than plain solvent) and, empirically,
   an occasional rescue out of a bad basin.
7. Validate: replay the finished set as an honest game of Taxman and
   confirm the replayed score matches, before recording it.

Certificates. Three record-level labels - computed only from the
FINAL settled score, after every step above, and never used to steer
the search - flag a score as provably (two of them) or very probably
(the third) optimal given the previous game's score alone:

  * "exact": score == n + previous score.  Playing n first and then
    game n-1's own solution unchanged is always legal, so opt(n) <=
    n + opt(n-1); meeting that bound closes the gap.
  * "prime": for prime n, score == n + previous score - (largest prime
    below n).  A prime's only possible tax payment is 1, and 1 is
    always claimed by whichever selection plays first, so a solution
    holds at most one prime and any solution containing one plays it
    first.  Taking prime n therefore forces sacrificing whichever prime
    the previous game's solution held - the largest one below n, since
    a smaller prime could always be swapped up.
  * "upper-delta": CONJECTURE-GRADE, unproven (see
    strategies/continuation.py's module docstring for the measurement
    behind it).  Let U*(k) be upper_half_selection(k)'s sum and
    d_upper(n) = U*(n) - U*(n-1).  Fires when score == previous score +
    d_upper(n), or, for even n, when score == previous score +
    d_upper(n) + n // 2 (n/2 sits on the n-1 boundary and can cross
    into the lower half of game n).

This file is a from-scratch reimplementation, checked (not derived by
import) against strategies/continuation.py's flagship "cold chain with
solvent re-anchor" configuration: run with --reanchor-solvent from a
cold (empty) start at n=2, it is score-identical and pick-set-identical
to that fast run for every game 2..250, and its certificate labels
agree with the committed results/chain_cold_1000.json over the same
range.

Run it with the final game size as the only argument; it always chains
from 2:

    $ python3 -m reference.continuation 21
    n= 2  score=    2  source=chain    certificate=-
    n= 3  score=    3  source=chain    certificate=prime
    n= 4  score=    7  source=chain    certificate=exact
    n= 5  score=    9  source=chain    certificate=prime
    n= 6  score=   15  source=chain    certificate=exact
    n= 7  score=   17  source=chain    certificate=prime
    n= 8  score=   21  source=chain    certificate=upper-delta
    n= 9  score=   30  source=chain    certificate=exact
    n=10  score=   40  source=chain    certificate=exact
    n=11  score=   44  source=chain    certificate=prime
    n=12  score=   50  source=chain    certificate=upper-delta
    n=13  score=   52  source=chain    certificate=prime
    n=14  score=   66  source=chain    certificate=exact
    n=15  score=   81  source=chain    certificate=exact
    n=16  score=   89  source=chain    certificate=upper-delta
    n=17  score=   93  source=chain    certificate=prime
    n=18  score=  111  source=chain    certificate=exact
    n=19  score=  113  source=chain    certificate=prime
    n=20  score=  124  source=chain    certificate=-
    n=21  score=  144  source=chain    certificate=-
    picks, in playing order: [19, 9, 15, 21, 14, 18, 12, 16, 20]
    score (sum of picks):    144

(The n=2 certificate reads "-": the chain has no game 1 score to compare
against, so no certificate condition can fire there.  See this file's
verification notes for why results/chain_cold_1000.json shows "exact"
at n=2 instead - a one-record artifact of how that dataset was
generated, not a divergence in this file's algorithm.)

Runtime scaling: every game re-derives its playability from scratch
(one or more full peel() calls per candidate move, each recursing and
rescanning over the whole pick set), where the fast code keeps an
incremental matching alive across moves and only ever touches what a
single mutation can change.  Measured under pypy3, chaining 2..250
takes about 52s here against about 5s for strategies.continuation.py's
--reanchor-solvent chain over the same range - roughly 10x slower, and
that ratio should widen with N, since each game's own playable() cost
grows with the pick set while the fast code's incremental update does
not.  Chains much beyond a few hundred are a multi-minute proposition
under this file; it exists to be read against the algorithm's
description, not to reproduce results/chain_cold_1000.json (that file,
covering n up to 1000, comes from strategies/continuation.py).
"""

from __future__ import annotations

import itertools
import sys
from math import gcd
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

import reference.solvent as solvent_ref

# The flagship configuration's hardcoded defaults (strategies/continuation.py
# TIER1_STEP_CAP, TIER2_REFILL_K, and its --bundle-limit default), reproduced
# here rather than exposed as parameters: this file plays exactly one
# strategy, so there is nothing for a caller to usefully vary.
TIER1_STEP_CAP = 80
TIER2_REFILL_K = 30
BUNDLE_LIMIT = 2000


# ---------------------------------------------------------------------------
# small number-theoretic helpers (primality, for the "prime" certificate)
# ---------------------------------------------------------------------------

def is_prime(k: int) -> bool:
    """Trial division up to sqrt(k) - plenty fast for the one-off queries
    this module makes (at most a couple of hundred per game, at chain
    sizes in the hundreds), so no sieve is built for it."""
    if k < 2:
        return False
    d = 2
    while d * d <= k:
        if k % d == 0:
            return False
        d += 1
    return True


def largest_prime_below(k: int) -> Optional[int]:
    """The largest prime strictly less than k, or None if k <= 2."""
    candidate = k - 1
    while candidate >= 2:
        if is_prime(candidate):
            return candidate
        candidate -= 1
    return None


# ---------------------------------------------------------------------------
# step 1: the upper half, expressed through reference.solvent's playable()
# ---------------------------------------------------------------------------

def upper_half_selection(n: int) -> Set[int]:
    """The optimal upper-half picks of game n: every candidate c > n/2
    that keeps the set playable, taken largest first.

    This is core.solve_upper_half's specialized matching code (C = the
    numbers above n/2, F = their maximal factors) expressed instead
    through reference.solvent's general playable() - the two coincide
    exactly because no number above n/2 can divide another number in
    the game, so every one of their maximal factors is at most n/2 and
    the upper-half game is already a plain instance of the same
    membership question solvent asks of the whole game.  Both peel()
    (here) and core.solve_mini (the fast code) are exact playability
    oracles over that same question, so the two approaches must
    agree - verified (evaluation/test_taxman.py) by direct comparison,
    not merely argued.

    Rebuilds reference.solvent's module-level maximal-factor table for
    n, which doubles as the shared table the rest of this game's steps
    (tier 1, tier 2, and the final replay) rely on: this is meant to be
    the first thing solve_game does each game.

    Args:
        n: The game size.

    Returns:
        The set of upper-half picks (every one strictly greater than
        n/2 that belongs to the kept set).
    """
    solvent_ref.build_maximal_factor_table(n)
    pick_set: Set[int] = set()
    for c in range(n, n // 2, -1):
        if solvent_ref.playable(pick_set | {c}) is not None:
            pick_set.add(c)
    return pick_set


# ---------------------------------------------------------------------------
# step 4: tier 1 - steepest-ascent single flips
# ---------------------------------------------------------------------------

def tier1(n: int, pick_set: Set[int]) -> int:
    """Hill-climb pick_set, in place, by the single steepest improving add.

    Adding x raises the score by exactly x, so the steepest improving
    add is the largest playable non-pick; sweeping candidates
    descending and taking the first acceptance is therefore equivalent
    to always taking the single best move.  Candidates are restricted
    to the lower half (x <= n/2) sharing a prime factor with n - the
    perturbation a new game size introduces lands close to n in the
    divisor lattice (see strategies/continuation.py's module docstring,
    "the transition anatomy's measured locality"), so this keeps the
    sweep to a couple dozen candidates instead of all of n/2.

    Args:
        n: The game size.
        pick_set: The current candidate set; mutated in place.

    Returns:
        The number of accepted flips.
    """
    local = [x for x in range(n // 2, 1, -1)
             if solvent_ref.all_maximal_factors[x] and gcd(x, n) > 1]
    flips = 0
    while flips < TIER1_STEP_CAP:
        moved = False
        for x in local:
            if x in pick_set:
                continue
            if solvent_ref.playable(pick_set | {x}) is not None:
                pick_set.add(x)
                flips += 1
                moved = True
                break
        if not moved:
            break
    return flips


# ---------------------------------------------------------------------------
# step 5: tier 2 - removal-anchored add/remove bundles
# ---------------------------------------------------------------------------

def _refill(pick_set: Set[int], candidates: List[int]) -> None:
    """Add each candidate, largest first, that keeps pick_set playable.

    Descending order matters: a big candidate that turns out to be
    unplayable simply fails and leaves pick_set untouched, so a smaller
    genuine beneficiary later in the list still finds its payment free.
    """
    for x in candidates:
        if x not in pick_set and solvent_ref.playable(pick_set | {x}) is not None:
            pick_set.add(x)


def _refill_candidates(removed: Tuple[int, ...], pool: List[int]) -> List[int]:
    """The best few candidates to try refilling with after removing `removed`.

    A genuine beneficiary of a removal shares a *large* common factor
    with what it displaces (e.g. removing 192 lets 240 in - they share
    48), while most of the pool shares only a factor of 2 and never
    becomes playable.  Ranking by the largest shared factor and keeping
    only the top TIER2_REFILL_K keeps the refill's playable() calls
    down to a few dozen instead of the whole pool, without losing the
    beneficiary.
    """
    ranked = sorted(pool, key=lambda x: max(gcd(x, r) for r in removed),
                     reverse=True)[:TIER2_REFILL_K]
    ranked.sort(reverse=True)  # refill largest-value-first, see _refill
    return ranked


def tier2_pass(n: int, pick_set: Set[int], budget: List[int]) -> bool:
    """Try one improving remove-and-refill bundle; apply and return True
    if one is found, otherwise leave pick_set unchanged and return False.

    Tier 1 alone gets stuck wherever the next improvement needs a
    *coordinated* move - some lower pick removed to free room for two
    bigger ones, net positive, with every single step along the way
    downhill.  The search is anchored on removals rather than adds:
    there are only a couple dozen lower picks to consider removing
    (singly or in pairs), which is far cheaper than searching the much
    larger space of candidate adds directly, and after a removal only
    the numbers sharing a prime factor with what was removed can newly
    become playable (freeing a pick's payment or its slot only matters
    to numbers built from the same primes), which is what keeps the
    refill search local too.

    budget[0] caps the number of remove/refill combinations tried, and
    is decremented in place so it can be shared across repeated calls
    within one game.

    Args:
        n: The game size.
        pick_set: The current candidate set; mutated in place only if
            an improving bundle is found and applied.
        budget: A one-element list holding the remaining combination
            budget for this game (shared, mutated in place).

    Returns:
        True if an improving bundle was found and applied.
    """
    lower_picks = sorted(p for p in pick_set if 2 * p <= n)
    local_blocked = sorted(
        (x for x in range(2, n // 2 + 1)
         if x not in pick_set and solvent_ref.all_maximal_factors[x] and gcd(x, n) > 1),
        reverse=True,
    )
    if not lower_picks or not local_blocked:
        return False

    # For each removable pick, the local blocked adds it shares a factor
    # with - the only adds a refill after removing it could recruit.
    share: Dict[int, List[int]] = {
        r: [x for x in local_blocked if gcd(x, r) > 1] for r in lower_picks
    }

    base = frozenset(pick_set)
    before = sum(base)

    def try_combo(removed: Tuple[int, ...], pool: List[int]) -> bool:
        if not pool:
            return False
        budget[0] -= 1
        pick_set.clear()
        pick_set.update(base)
        for r in removed:
            pick_set.discard(r)
        _refill(pick_set, _refill_candidates(removed, pool))
        return sum(pick_set) > before

    # Singles first (one correctly-priced swap), then pairs (the coupled
    # 2-for-2 valley moves).  The first improving combination wins.
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

    pick_set.clear()
    pick_set.update(base)
    return False


# ---------------------------------------------------------------------------
# step 7: validation by replay
# ---------------------------------------------------------------------------

def replay_and_score(n: int, sequence: Sequence[int]) -> int:
    """Score `sequence` while checking it is a legal game of Taxman n.

    Written locally rather than imported from core.check_sequence so
    this file stays a self-contained spec of the strategy (its only
    external dependency is reference.solvent, which the algorithm
    itself is defined in terms of) - the logic is a direct reading of
    the rules: a pick keeps c and surrenders every divisor of c still
    in the pot, and every pick must surrender at least one.

    Args:
        n: The game size.
        sequence: The picks, in the order they are claimed to be playable.

    Returns:
        The score (sum of the picks), once every pick has been checked.

    Raises:
        RuntimeError: if any pick is missing from the pot, or has no
            surviving divisor to surrender as tax.
    """
    pot = set(range(1, n + 1))
    score = 0
    for c in sequence:
        tax = {d for d in pot if d != c and c % d == 0}
        if c not in pot or not tax:
            raise RuntimeError(f"illegal sequence for game {n} at {c}")
        pot -= tax
        pot.discard(c)
        score += c
    return score


def playing_order(n: int, pick_set: Set[int]) -> List[int]:
    """A legal playing order for pick_set, via reference.solvent's own
    playable() / ordered() (pick_set is already known playable, so this
    cannot fail short of a bug)."""
    pay = solvent_ref.playable(pick_set)
    assert pay is not None, f"n={n}: a chosen set turned out unplayable"
    return solvent_ref.ordered(pay)


# ---------------------------------------------------------------------------
# certificates (label-only; computed after the fact, never steer the search)
# ---------------------------------------------------------------------------

def certificate_label(
    n: int,
    score: int,
    prev_score: Optional[int],
    upper_sum: int,
    prev_upper_sum: Optional[int],
) -> Optional[str]:
    """Which (if any) of the three certificates (module docstring) fires.

    Checked in order exact, then prime, then upper-delta: "exact" and
    "prime" are proven identities and, empirically, special cases of
    the (unproven) upper-delta identity, so checking them first is
    what keeps the proven labels from being shadowed by the conjectured
    one.  Computed purely from the final settled score - never gates or
    skips the search itself.
    """
    if prev_score is None:
        return None
    if score == n + prev_score:
        return "exact"
    if is_prime(n):
        p_hat = largest_prime_below(n)
        if p_hat is not None and score == n + prev_score - p_hat:
            return "prime"
    if prev_upper_sum is not None:
        d_upper = upper_sum - prev_upper_sum
        if score == prev_score + d_upper:
            return "upper-delta"
        if n % 2 == 0 and score == prev_score + d_upper + n // 2:
            return "upper-delta"
    return None


# ---------------------------------------------------------------------------
# per-game solve
# ---------------------------------------------------------------------------

def solve_game(
    n: int,
    prev_set: Set[int],
    prev_score: Optional[int],
    prev_upper_sum: Optional[int],
) -> Dict[str, Any]:
    """Solve game n, warm-started from the previous game's own solution.

    Args:
        n: The game size.
        prev_set: Game n-1's final pick set (chain output, possibly a
            solvent re-anchor adoption from that game).
        prev_score: Game n-1's final score, or None for the very first
            game of a cold chain.
        prev_upper_sum: U*(n-1) = upper_half_selection(n-1)'s sum, or
            None if unavailable.

    Returns:
        A record with keys "n", "score", "source" ("chain" or
        "solvent"), "certificate" (a label or None), "tier" (0, 1, or
        2 - the deepest tier that produced the solution), "flips", and
        "set" (the final pick set, carried forward by the caller).
    """
    # Step 1: seed the always-playable upper half.  This also rebuilds
    # the shared maximal-factor table to size n, which every playable()
    # call for the rest of this game (and the reanchor below) relies on.
    pick_set = upper_half_selection(n)
    upper_sum = sum(pick_set)

    # Step 2: carry forward the previous game's lower picks.
    for x in sorted((m for m in prev_set if 2 * m <= n), reverse=True):
        if solvent_ref.playable(pick_set | {x}) is not None:
            pick_set.add(x)

    incumbent_score = sum(pick_set)
    tier = 0
    flips = 0

    # Step 3: the lucky exit - see certificate_label's "exact" case,
    # re-checked below on the settled score so the label always ends up
    # correct even though this early exit only rechecks the incumbent.
    if prev_score is not None and incumbent_score == n + prev_score:
        pass
    else:
        # Step 4: tier 1 always runs to quiescence first.
        f = tier1(n, pick_set)
        flips += f
        if f > 0:
            tier = max(tier, 1)

        # Step 5: interleave tier-2 bundles with tier-1 re-convergence.
        budget = [BUNDLE_LIMIT]
        while budget[0] > 0:
            if tier2_pass(n, pick_set, budget):
                tier = max(tier, 2)
                f = tier1(n, pick_set)
                flips += f
                if f > 0:
                    tier = max(tier, 1)
            else:
                break

    our_score = sum(pick_set)

    # Step 7 (validate the chain's own candidate before it can be
    # recorded or compared against the reanchor).
    order = playing_order(n, pick_set)
    replay_score = replay_and_score(n, order)
    assert replay_score == our_score == sum(pick_set), (
        f"replay mismatch at n={n}: replay={replay_score} "
        f"score={our_score} sum={sum(pick_set)}"
    )

    # Step 6: re-anchor against reference.solvent's own from-scratch
    # solution, adopting it if (and only if) it scores strictly higher.
    source = "chain"
    output_set = pick_set
    solvent_seq = solvent_ref.solvent(n)
    solvent_score = replay_and_score(n, solvent_seq)  # validates solvent's own sequence
    if solvent_score > our_score:
        our_score = solvent_score
        output_set = set(solvent_seq)
        source = "solvent"

    certificate = certificate_label(n, our_score, prev_score, upper_sum, prev_upper_sum)

    return {
        "n": n,
        "score": our_score,
        "source": source,
        "certificate": certificate,
        "tier": tier,
        "flips": flips,
        "set": output_set,
        "upper_sum": upper_sum,
    }


# ---------------------------------------------------------------------------
# driver
# ---------------------------------------------------------------------------

def solve_chain(to_n: int, from_n: int = 2) -> List[Dict[str, Any]]:
    """Solve every game from_n..to_n in order, each fed the previous one's
    own result; a cold chain (from_n's predecessor is unseeded) unless
    the caller has its own reason to start elsewhere.

    Args:
        to_n: The last game size to solve.
        from_n: The first game size to solve (default 2, the smallest
            game with a choice to make).

    Returns:
        One record per game (solve_game's return value), in order.
    """
    prev_set: Set[int] = set()
    prev_score: Optional[int] = None
    prev_upper_sum = sum(upper_half_selection(from_n - 1)) if from_n > 1 else 0

    records = []
    for n in range(from_n, to_n + 1):
        rec = solve_game(n, prev_set, prev_score, prev_upper_sum)
        records.append(rec)
        prev_set = rec["set"]
        prev_score = rec["score"]
        prev_upper_sum = rec["upper_sum"]
    return records


def main(argv: List[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if len(argv) != 1 or not argv[0].isdigit():
        sys.exit("usage: python3 -m reference.continuation N   (chains 2..N)")
    n_max = int(argv[0])

    # Every playable() call recurses once per pick in the candidate set
    # being tested (reference.solvent.peel), so the deepest recursion
    # needs roughly one stack frame per pick - at most n_max.  Grant
    # that plus headroom, never going below Python's default of 1000.
    sys.setrecursionlimit(max(1000, n_max + 500))

    records = solve_chain(n_max)
    for rec in records:
        print(f"n={rec['n']:2d}  score={rec['score']:5d}  "
              f"source={rec['source']:7s}  "
              f"certificate={rec['certificate'] or '-'}")

    final_order = playing_order(n_max, records[-1]["set"])
    print("picks, in playing order:", final_order)
    print("score (sum of picks):   ", sum(final_order))
    return 0


if __name__ == "__main__":
    sys.exit(main())
