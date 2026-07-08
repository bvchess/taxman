"""Unit tests for core.py, anchored to the examples in the wiki.

Run with:  python3 -m pytest evaluation/test_taxman.py   (from polytime/)
"""

import json
from pathlib import Path

import pytest

from core import (
    Infeasible,
    maximal_factors,
    optimize_mini,
    smallest_prime_factors,
    solve_mini,
    solve_upper_half,
    upper_half_game,
)
from evaluation.verify import replay

SPF = smallest_prime_factors(1000)

RESULTS_DIR = Path(__file__).resolve().parent.parent / "results"


def mf_table(numbers):
    return {n: maximal_factors(n, SPF) for n in numbers}


def test_maximal_factors():
    # From the wiki definitions page: the maximal factors of 18 are 9 and 6,
    # and the maximal factor of 5 is 1.
    assert maximal_factors(18, SPF) == {9, 6}
    assert maximal_factors(5, SPF) == {1}
    assert maximal_factors(1, SPF) == set()
    assert maximal_factors(16, SPF) == {8}
    assert maximal_factors(20, SPF) == {10, 4}


def test_solve_mini_single_selection():
    mf = mf_table([7])
    sequence, matching = solve_mini({7}, {1}, mf)
    assert sequence == [7]
    assert matching == {7: 1}


def test_solve_mini_infeasible_two_primes_one_factor():
    # Two primes share the single factor 1: only one can be selected.
    mf = mf_table([5, 7])
    with pytest.raises(Infeasible):
        solve_mini({5, 7}, {1}, mf)


def test_solve_mini_shared_core_is_infeasible():
    # 12 and 18 both have maximal factors {6, ...} restricted here to {6, 9}
    # in a way that forms a 2-core: 12 -> {6, 4}, 18 -> {6, 9}.  With only
    # {6} available both compete for one factor.
    mf = mf_table([12, 18])
    with pytest.raises(Infeasible):
        solve_mini({12, 18}, {6}, mf)


def test_optimize_mini_takes_highest_prime():
    # From the wiki: optimize_mini({2,3,5,7}, {1}) returns {7}.
    mf = mf_table([2, 3, 5, 7])
    opt_c, r_f = optimize_mini({2, 3, 5, 7}, {1}, mf)
    assert opt_c == {7}
    assert r_f == {1}


def test_upper_half_game_n21():
    c_set, f_set, mf = upper_half_game(21, SPF)
    assert c_set == set(range(11, 22))
    assert f_set == {1, 2, 3, 4, 5, 6, 7, 8, 9, 10}
    assert mf[21] == {7, 3}
    assert mf[16] == {8}


def test_upper_half_n21_matches_walkthrough():
    # The wiki walkthrough's optimal solution for N=21 is
    # 19, 9, 15, 21, 14, 18, 12, 20, 16.  Its numbers greater than 21/2 are
    # 19, 15, 21, 14, 18, 12, 20, 16 (9 is a promoted factor, not upper-half).
    sequence, tax_pool = solve_upper_half(21, SPF)
    assert set(sequence) == {19, 15, 21, 14, 18, 12, 20, 16}
    assert replay(sequence, set(sequence) | tax_pool)


def test_upper_half_small_games():
    # n=1: the pot is {1}, nothing can pay tax, no selections.
    assert solve_upper_half(1, SPF)[0] == []
    # n=2: select 2, paying 1.
    assert solve_upper_half(2, SPF)[0] == [2]
    # n=5: the known optimal game is 5 (tax 1) then 4 (tax 2); both are >2.5.
    sequence, tax_pool = solve_upper_half(5, SPF)
    assert set(sequence) == {4, 5}
    assert replay(sequence, set(sequence) | tax_pool)


def test_replay_rejects_taxless_selection():
    assert not replay([5], {5})  # nothing left to pay tax with
    assert replay([5], {1, 5})


def test_strategies_are_legal_and_sane():
    from core import check_sequence, divisor_lists, maximal_factor_lists
    from strategies.cascade import cascade
    from strategies.onetax import one_tax
    from strategies.solvent import solvent

    divs = divisor_lists(128)
    mf = maximal_factor_lists(128)
    for n in (2, 5, 10, 21, 50, 128):
        solvent_score = check_sequence(n, solvent(n, mf))
        cascade_score = check_sequence(n, cascade(n, divs))
        assert solvent_score >= cascade_score
        assert one_tax(n, divs) > 0

    # solvent finds the known optimal for the wiki's walkthrough game...
    assert check_sequence(21, solvent(21, mf)) == 144
    # ...and reproduces the N=5 example from the wiki's game introduction.
    assert check_sequence(5, solvent(5, mf)) == 9


def test_solvent_regression_floor():
    # The simplified two-tier solvent must never score below the recorded
    # baseline in results/solvent_1000.json (it may legitimately improve on
    # it by shedding bogus rejections, hence a floor check rather than ==).
    from core import check_sequence, maximal_factor_lists
    from strategies.solvent import solvent

    stored = json.loads((RESULTS_DIR / "solvent_1000.json").read_text())
    mf = maximal_factor_lists(1000)  # a larger table is valid for any smaller n
    for n in (21, 100, 250, 500, 750, 1000):
        assert check_sequence(n, solvent(n, mf)) >= stored[str(n)]


def test_solvent_simple_matches_canonical():
    # reference/solvent.py is a plain, from-scratch executable spec of the
    # README's "Solvent, final form" pseudocode; it must agree exactly with
    # both the recorded canonical scores and the fast strategies.solvent()
    # output (the README states the canonical set is a deterministic
    # function of playability alone), and produce a legal game.
    from core import check_sequence, maximal_factor_lists
    from strategies.solvent import solvent as strategies_solvent

    import reference.solvent as solvent_simple

    stored = json.loads((RESULTS_DIR / "solvent_1000.json").read_text())
    mf = maximal_factor_lists(60)
    for n in range(1, 61):
        seq = solvent_simple.solvent(n)
        score = sum(seq)
        assert score == stored[str(n)]
        assert set(seq) == set(strategies_solvent(n, mf))
        assert check_sequence(n, seq) == score


def test_prime_sacrifice_identity():
    # Wiki "Reusing a previous solution", sections "If N is prime" and
    # "Generalizing": for prime N, opt(N) = N + opt(N-1) - p_hat, where
    # p_hat is the largest prime below N. Pure data check against
    # optimal.json -- no solver runs -- for every prime N in 3..1000
    # (validated 167/167 in the original measurement).
    from evaluation.bound import DEFAULT_OPTIMAL

    optimal = json.loads(DEFAULT_OPTIMAL.read_text())
    scores = {g["n"]: g["score"] for g in optimal}

    largest_prime_below = [None] * 1001
    last_prime = None
    for k in range(1001):
        largest_prime_below[k] = last_prime
        if k >= 2 and SPF[k] == k:
            last_prime = k

    checked = 0
    for n in range(3, 1001):
        if SPF[n] != n:
            continue  # n not prime
        p_hat = largest_prime_below[n]
        assert p_hat is not None
        assert scores[n] == n + scores[n - 1] - p_hat
        checked += 1
    assert checked > 0  # sanity: the loop actually exercised primes


def test_upper_delta_certificate_is_benign():
    # strategies.continuation's third certificate kind ("upper-delta",
    # CONJECTURE-GRADE): let U*(k) = sum(solve_upper_half(k, spf)[0]) and
    # d_upper(n) = U*(n) - U*(n-1). The certificate fires when
    # score == prev_score + d_upper(n), or -- for even n -- score ==
    # prev_score + d_upper(n) + n//2 (the boundary-crosser n/2 re-picked as
    # a lower number). Validated data-only (scratchpad eviction_cert_check.py)
    # as zero-harmful (gap never increases where it fires) over all 998
    # cold-chain transitions at n<=1000. This test re-derives that check,
    # restricted to n<=400: solve_upper_half over the full n<=1000 range
    # takes ~70s in CPython (well over a reasonable pytest budget), while
    # n<=400 takes ~4s and still exercises many firings.
    from evaluation.bound import DEFAULT_OPTIMAL

    n_max = 400
    spf = smallest_prime_factors(n_max)
    u_star = {n: sum(solve_upper_half(n, spf)[0]) for n in range(0, n_max + 1)}

    optimal = json.loads(DEFAULT_OPTIMAL.read_text())
    opt_score = {g["n"]: g["score"] for g in optimal}

    cold_path = RESULTS_DIR / "chain_cold_1000.json"
    chain = json.loads(cold_path.read_text())
    chain_by_n = {r["n"]: r for r in chain if r["n"] <= n_max}

    fired = 0
    for n in range(3, n_max + 1):
        if n not in chain_by_n or (n - 1) not in chain_by_n:
            continue
        prev_score = chain_by_n[n - 1]["score"]
        score_n = chain_by_n[n]["score"]
        d_upper = u_star[n] - u_star[n - 1]
        fires = (score_n == prev_score + d_upper) or (
            n % 2 == 0 and score_n == prev_score + d_upper + n // 2
        )
        if not fires:
            continue
        fired += 1
        gap_n = opt_score[n] - score_n
        gap_prev = opt_score[n - 1] - prev_score
        assert gap_n <= gap_prev, (
            f"upper-delta certificate fired harmfully at n={n}: "
            f"gap {gap_prev} -> {gap_n}"
        )
    assert fired > 0  # sanity: the rule actually exercised firings in range


def test_fm_bound_matches_published_values():
    pytest.importorskip("networkx")
    from evaluation.bound import fm_bound

    assert fm_bound(21) == 145
    assert fm_bound(50) == 811
    assert fm_bound(100) == 3173
    assert fm_bound(128) == 5310


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-q"]))
