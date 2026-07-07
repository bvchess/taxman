"""Unit tests for taxman_mini, anchored to the examples in the wiki.

Run with:  python3 -m pytest test_taxman_mini.py   (or just python3 test_taxman_mini.py)
"""

import json
from pathlib import Path

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


def test_two_tax_is_a_measured_noop():
    # OneTax always drains the pot completely, so the (provably safe)
    # stranded harvest never fires; see the one_tax docstring.
    from approx import divisor_lists, one_tax

    divs = divisor_lists(300)
    for n in (10, 21, 100, 274, 300):
        assert one_tax(n, divs, two_tax=True) == one_tax(n, divs)


def test_oracle_dominates_onetax_and_hybrid():
    from approx import (
        check_sequence, divisor_lists, one_tax, one_tax_forced_upper,
        one_tax_oracle,
    )

    divs = divisor_lists(200)
    for n in (10, 21, 55, 100, 158, 200):
        score, seq = one_tax_oracle(n, divs, SPF)
        assert check_sequence(n, seq) == score
        assert score >= one_tax(n, divs)
        assert score >= one_tax_forced_upper(n, divs, SPF)[0]


def test_approx_strategies_are_legal_and_sane():
    from approx import cascade, check_sequence, divisor_lists, solvent, one_tax

    divs = divisor_lists(128)
    for n in (2, 5, 10, 21, 50, 128):
        solvent_score = check_sequence(n, solvent(n, divs))
        cascade_score = check_sequence(n, cascade(n, divs))
        assert solvent_score >= cascade_score
        assert one_tax(n, divs) > 0

    # solvent finds the known optimal for the wiki's walkthrough game...
    assert check_sequence(21, solvent(21, divs)) == 144
    # ...and reproduces the N=5 example from the wiki's game introduction.
    assert check_sequence(5, solvent(5, divs)) == 9


def test_solvent_regression_floor():
    # The simplified two-tier solvent must never score below the recorded
    # baseline in solvent_results.json (it may legitimately improve on
    # it by shedding bogus rejections, hence a floor check rather than ==).
    from approx import check_sequence, divisor_lists, solvent

    stored = json.loads(
        (Path(__file__).resolve().parent / "solvent_results.json").read_text()
    )
    divs = divisor_lists(1000)  # a larger table is valid for any smaller n
    for n in (21, 100, 250, 500, 750, 1000):
        assert check_sequence(n, solvent(n, divs)) >= stored[str(n)]


def test_solvent_simple_matches_canonical():
    # solvent_simple.py is a plain, from-scratch executable spec of the
    # README's "Solvent, final form" pseudocode; it must agree exactly with
    # both the recorded canonical scores and the fast approx.solvent() output
    # (the README states the canonical set is a deterministic function of
    # playability alone), and produce a legal game.
    from approx import check_sequence, divisor_lists, solvent as approx_solvent

    import solvent_simple

    stored = json.loads(
        (Path(__file__).resolve().parent / "solvent_results.json").read_text()
    )
    divs = divisor_lists(60)
    for n in range(1, 61):
        seq = solvent_simple.solvent(n)
        score = sum(seq)
        assert score == stored[str(n)]
        assert set(seq) == set(approx_solvent(n, divs))
        assert check_sequence(n, seq) == score


def test_fm_bound_matches_published_values():
    pytest.importorskip("networkx")
    from bound import fm_bound

    assert fm_bound(21) == 145
    assert fm_bound(50) == 811
    assert fm_bound(100) == 3173
    assert fm_bound(128) == 5310


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-q"]))
