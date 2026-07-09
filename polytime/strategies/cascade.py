"""cascade: the verified upper-half theory applied band by band.

The pure upper-half machinery applied band by band: play the numbers
above half the remaining maximum optimally with optimize_mini and
solve_mini, sweep, repeat on what is left.  Shows how much of the game
the verified theory captures on its own, with no promotions (~93% of
optimal) -- the ablation baseline that measures what solvent's and
the chain's remaining machinery is actually buying.

Note cascade plays solve_mini's raw sequence under real sweeps, so it
needs TRUE divisor lists, not the maximal-factor pool: feasibility is
pool-invariant (the lifting lemma) but raw play orders are not -- at
n=5 a maximal-factor pool orders [4, 5], and playing 4 sweeps the 1
that 5 needed.
"""

from __future__ import annotations

from typing import Dict, List, Sequence, Set

from core import optimize_mini, solve_mini


def cascade(n: int, divs: Sequence[List[int]]) -> List[int]:
    """Approximate a full game with repeated upper-half factor games.

    Unlike solvent, cascade uses TRUE proper divisors (`divs`) as its
    per-band edge pool, NOT the maximal-factor pool.  It replays
    solve_mini's returned ORDER directly under real-game (true-divisor)
    sweeping, so the order must be valid against every divisor a pick
    sweeps -- a stronger property than the matching-feasibility the lifting
    lemma equates the two pools on.  With a maximal-factor pool solve_mini
    can emit an order that strands a pick whose only surviving payment is a
    non-maximal shared divisor swept early (e.g. n=5: playing 4 before 5
    sweeps their shared divisor 1).  The sweep replay and end-of-band
    liveness check need true divisors for the same reason.
    """
    pot: Set[int] = set(range(1, n + 1))
    sequence: List[int] = []
    band_top = n

    while band_top >= 1 and pot:
        # Candidates are source nodes: nothing in the pot is a multiple.
        c_set = {
            c for c in pot
            if 2 * c > band_top
            and not any(m in pot for m in range(2 * c, n + 1, c))
        }
        band_top //= 2
        if not c_set:
            continue
        edges: Dict[int, Set[int]] = {
            c: {d for d in divs[c] if d in pot} for c in c_set
        }
        f_set: Set[int] = set().union(*edges.values())

        opt_c, r_f = optimize_mini(c_set, f_set, edges)
        order, _ = solve_mini(opt_c, r_f, edges)

        for c in order:
            tax = [d for d in divs[c] if d in pot]
            if not tax:
                raise RuntimeError(f"illegal move {c} in game {n}")
            pot.difference_update(tax)
            pot.discard(c)
            sequence.append(c)

        # Unselected candidates with no divisor left can never be played;
        # ones that still have a divisor get another chance in later bands.
        pot -= {c for c in c_set - opt_c if not any(d in pot for d in divs[c])}

    return sequence
