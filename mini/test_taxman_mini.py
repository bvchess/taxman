"""Unit tests for taxman_mini, anchored to the examples in the wiki.

Run with:  python3 -m pytest test_taxman_mini.py   (or just python3 test_taxman_mini.py)
"""

import pytest

from taxman_mini import (
    MiniInfeasible,
    maximal_factors,
    optimize_mini,
    smallest_prime_factors,
    solve_mini,
    solve_upper_half,
    upper_half_game,
)
from verify import replay

SPF = smallest_prime_factors(1000)


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
    with pytest.raises(MiniInfeasible):
        solve_mini({5, 7}, {1}, mf)


def test_solve_mini_shared_core_is_infeasible():
    # 12 and 18 both have maximal factors {6, ...} restricted here to {6, 9}
    # in a way that forms a 2-core: 12 -> {6, 4}, 18 -> {6, 9}.  With only
    # {6} available both compete for one factor.
    mf = mf_table([12, 18])
    with pytest.raises(MiniInfeasible):
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


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-q"]))
