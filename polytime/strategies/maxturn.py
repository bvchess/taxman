"""maxturn (Carmony & Holliday, 1993): maximize a single turn's net take.

Pick the number maximizing the player's take minus the taxman's take for
the turn.
"""

from __future__ import annotations

from typing import List, Sequence


def max_turn(n: int, divs: Sequence[List[int]]) -> int:
    """Carmony & Holliday's MaxTurn heuristic; returns the player's score."""
    in_pot = bytearray(n + 1)
    for i in range(1, n + 1):
        in_pot[i] = 1
    count = [len(divs[m]) for m in range(n + 1)]
    dsum = [sum(divs[m]) for m in range(n + 1)]

    def remove(x: int) -> None:
        in_pot[x] = 0
        for m in range(2 * x, n + 1, x):
            count[m] -= 1
            dsum[m] -= x

    score = 0
    while True:
        pick, best = 0, None
        for c in range(n, 1, -1):  # ties go to the larger number
            if in_pot[c] and count[c] > 0:
                value = c - dsum[c]
                if best is None or value > best:
                    pick, best = c, value
        if not pick:
            break
        tax = [x for x in divs[pick] if in_pot[x]]
        remove(pick)
        for x in tax:
            remove(x)
        score += pick
    return score
