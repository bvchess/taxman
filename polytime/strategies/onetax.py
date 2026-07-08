"""onetax (Moniot): pick the largest number with exactly one divisor left.

Compared against heuristics from Robert Moniot's strategy comparison
(https://www.dsm.fordham.edu/~moniot/taxman-strategies-comparison.html).
"""

from __future__ import annotations

from typing import List, Sequence


def one_tax(n: int, divs: Sequence[List[int]], refined: bool = True) -> int:
    """Moniot's OneTax heuristic; returns the player's score.

    Picks the largest number with exactly one divisor left in play, with
    Moniot's refinement: if that pick would strand a multiple m (leaving
    m with no divisors while m divides nothing left in play), pick m
    instead.
    """
    in_pot = bytearray(n + 1)
    for i in range(1, n + 1):
        in_pot[i] = 1
    count = [sum(1 for d in divs[m] if in_pot[d]) for m in range(n + 1)]

    def remove(x: int) -> None:
        in_pot[x] = 0
        for m in range(2 * x, n + 1, x):
            count[m] -= 1

    score = 0
    while True:
        pick = 0
        for c in range(n, 1, -1):
            if in_pot[c] and count[c] == 1:
                pick = c
                break
        if not pick:
            break

        if pick and count[pick] == 1 and refined:
            d = next(x for x in divs[pick] if in_pot[x])
            rescue = 0
            for m in range(2 * pick, n + 1, pick):
                if not in_pot[m]:
                    continue
                stranded = all(x in (pick, d) for x in divs[m] if in_pot[x])
                useless = not any(in_pot[k] for k in range(2 * m, n + 1, m))
                if stranded and useless:
                    rescue = max(rescue, m)
            if rescue:
                pick = rescue

        tax = [x for x in divs[pick] if in_pot[x]]
        remove(pick)
        for x in tax:
            remove(x)
        score += pick
    return score
