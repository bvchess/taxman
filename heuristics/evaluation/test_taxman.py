# Copyright (c) Brian Chess 2026
# SPDX-License-Identifier: MIT

"""Unit tests for core.py, anchored to the examples in the wiki.

Run with:  python3 -m pytest evaluation/test_taxman.py   (from heuristics/)
"""

import json
from pathlib import Path

import pytest

from core import (
    Infeasible,
    maximal_factors,
    optimize_mini,
    smallest_prime_factors,
    peel,
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


def test_peel_single_selection():
    mf = mf_table([7])
    sequence, matching = peel({7}, {1}, mf)
    assert sequence == [7]
    assert matching == {7: 1}


def test_peel_infeasible_two_primes_one_factor():
    # Two primes share the single factor 1: only one can be selected.
    mf = mf_table([5, 7])
    with pytest.raises(Infeasible):
        peel({5, 7}, {1}, mf)


def test_peel_shared_core_is_infeasible():
    # 12 and 18 both have maximal factors {6, ...} restricted here to {6, 9}
    # in a way that forms a 2-core: 12 -> {6, 4}, 18 -> {6, 9}.  With only
    # {6} available both compete for one factor.
    mf = mf_table([12, 18])
    with pytest.raises(Infeasible):
        peel({12, 18}, {6}, mf)


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


def test_solvent_b_matches_experiment_anchors():
    # Anchor scores recorded by the band_audit.py structural-audit experiment
    # (scratchpad band_audit_results.json, n_from=2 n_to=1000, cap 6 audit
    # iterations), covering distinct behaviors: 21 and 100 untouched
    # (already optimal, the audit never fires); 81 untouched despite a
    # nonempty audit set (audited, but no candidate ban ever improved the
    # score); 54 fired and fully repaired to optimal; 120 fired across two
    # audit iterations; 208 fired but only partially closes the gap to
    # optimal.
    from core import check_sequence, maximal_factor_lists
    from strategies.solvent_b import solvent_b

    anchors = {21: 144, 54: 940, 81: 2095, 100: 3164, 120: 4593, 208: 13806}
    mf = maximal_factor_lists(1000)
    for n, expected in anchors.items():
        assert check_sequence(n, solvent_b(n, mf)) == expected


def test_solvent_b_never_below_solvent():
    # Every audit adoption is a strict improvement (see solvent_b's
    # docstring), so a fixpoint with zero adoptions must reproduce solvent's
    # own score exactly, and any fired audit can only raise it.
    from core import check_sequence, maximal_factor_lists
    from strategies.solvent import solvent
    from strategies.solvent_b import solvent_b

    mf = maximal_factor_lists(300)
    for n in (21, 54, 81, 100, 120, 150, 208, 300):
        assert check_sequence(n, solvent_b(n, mf)) >= check_sequence(n, solvent(n, mf))


def test_solvent_b_fork_matches_full_rerun():
    # solvent-b's speedup is prefix-reuse forking: within one audit round, a
    # single descending scan forks at each candidate instead of rerunning
    # the whole scan from n per candidate.  This checks that speedup is
    # lossless: at n=150 and n=200, every forked trial encountered while
    # replaying solvent_b's own audit trajectory is compared against an
    # independently-coded full rerun (a verbatim reimplementation of
    # solvent's driving loop under one extra ban, built directly from
    # strategies.solvent's tier-1/tier-2 primitives -- NOT solvent_b's own
    # _finish_scan/_make_try_select) and must yield the identical pick set.
    from core import maximal_factor_lists
    from strategies.solvent import (
        _augment,
        _complete_matching,
        _is_acyclic,
        _playable_order,
        _rollback,
        solvent,
    )
    from strategies.solvent_b import (
        MAX_ITERS,
        _finish_scan,
        _make_try_select,
        _structural_audit_set,
    )

    def full_rerun(n, mf, banned):
        # Independent copy of solvent's driving loop, banned-set
        # parameterized (mirrors band_audit.py's solvent_banned; duplicated
        # here, rather than imported, so this test's oracle does not depend
        # on solvent_b's own forking machinery).
        selected = set()
        owner = {}

        def try_select(m):
            pre_trail = []
            if m in owner:
                holder = owner.pop(m)
                if _augment(holder, owner, selected, mf, {m}, pre_trail):
                    pre_trail.append((m, holder))
                else:
                    owner[m] = holder
                    return False
            selected.add(m)
            trail = []
            if _augment(m, owner, selected, mf, set(), trail):
                if _is_acyclic(selected, owner, n):
                    return True
                _rollback(owner, trail)
                selected.discard(m)
                _rollback(owner, pre_trail)
                matching = _complete_matching(selected | {m}, mf)
                if matching is None:
                    return False
                owner_candidate = {f: c for c, f in matching.items()}
                owner.clear()
                owner.update(owner_candidate)
                selected.add(m)
                return True
            selected.discard(m)
            _rollback(owner, pre_trail)
            return False

        for m in range(n, 1, -1):
            if not mf[m] or m in banned:
                continue
            try_select(m)
        return selected

    mf = maximal_factor_lists(200)
    checks = 0
    for n in (150, 200):
        current_set = set(solvent(n, mf))
        banned = set()
        for _ in range(MAX_ITERS):
            B = _structural_audit_set(current_set, mf)
            if not B:
                break
            candidates = sorted(B, reverse=True)
            current_score = sum(current_set)

            selected, owner = set(), {}
            try_select = _make_try_select(n, mf, selected, owner)
            remaining = iter(candidates)
            next_candidate = next(remaining, None)
            adopted = None
            for m in range(n, 1, -1):
                if not mf[m] or m in banned:
                    continue
                if m == next_candidate:
                    fork_selected = set(selected)
                    fork_owner = dict(owner)
                    _finish_scan(n, mf, banned | {m}, fork_selected, fork_owner, m - 1)

                    oracle_selected = full_rerun(n, mf, banned | {m})
                    assert fork_selected == oracle_selected, (n, m)
                    checks += 1

                    if adopted is None and sum(fork_selected) > current_score:
                        adopted = (m, fork_selected)
                    next_candidate = next(remaining, None)
                try_select(m)

            if adopted is None:
                break
            m, new_set = adopted
            banned = banned | {m}
            current_set = new_set

    assert checks > 0  # sanity: the audit actually exercised fork points


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


def test_reference_continuation_matches_committed_chain():
    # reference/continuation.py is a from-scratch reimplementation of the
    # flagship "cold chain with solvent re-anchor" configuration behind
    # results/chain_cold_1000.json.  Chained from n=2 to 60 (well under the
    # committed dataset's n=1000, and fast enough under plain CPython to
    # belong in the suite -- ~0.1s here, see the module's own docstring for
    # the full n<=250 timing under pypy3), its per-game scores must
    # reproduce the committed dataset exactly.
    from reference.continuation import solve_chain

    committed = json.loads((RESULTS_DIR / "chain_cold_1000.json").read_text())
    committed_scores = {g["n"]: g["score"] for g in committed}

    records = solve_chain(60)
    assert len(records) == 59  # n = 2..60
    for rec in records:
        assert rec["score"] == committed_scores[rec["n"]], (
            f"n={rec['n']}: reference score {rec['score']} != "
            f"committed {committed_scores[rec['n']]}"
        )


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
