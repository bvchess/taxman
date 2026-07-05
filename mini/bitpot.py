"""Bitmask primitives for representing a Taxman pot as a single Python int.

Today pots are Python `set[int]` objects; this module provides the pieces
needed to represent a pot instead as one arbitrary-precision int, where bit
i (1-indexed: `mask >> i & 1`) means "i is in the pot".  For n=10000 that is
just a ~10000-bit int, which Python handles natively.

`divisor_masks` and `multiple_masks` mirror the multiples-sieve pattern used
by `divisor_lists` in approx.py (for each d, walk its multiples with a step
of d), so both run in O(n log n) rather than the O(n sqrt n) or O(n^2) cost
of computing divisors per number.
"""

from __future__ import annotations

from typing import Iterable, Iterator, List


def divisor_masks(n: int) -> List[int]:
    """masks[c] has bit d set for every proper divisor d of c, for c <= n.

    masks[0] = masks[1] = 0 (no proper divisors).  Built with the same
    multiples sieve as `divisor_lists` in approx.py: O(n log n) total.
    """
    masks: List[int] = [0] * (n + 1)
    for d in range(1, n // 2 + 1):
        for m in range(2 * d, n + 1, d):
            masks[m] |= 1 << d
    return masks


def multiple_masks(n: int) -> List[int]:
    """masks[c] has bit m set for every proper multiple m of c with m <= n.

    masks[0] = 0.  masks[1] has every bit from 2..n set (every number is a
    multiple of 1).  O(n log n) total (harmonic sum).
    """
    masks: List[int] = [0] * (n + 1)
    for c in range(1, n + 1):
        for m in range(2 * c, n + 1, c):
            masks[c] |= 1 << m
    return masks


def mask_of(numbers: Iterable[int]) -> int:
    """The bitmask with exactly the given bits set."""
    mask = 0
    for i in numbers:
        mask |= 1 << i
    return mask


def bits(mask: int) -> Iterator[int]:
    """Yield the set-bit indices of mask in ascending order."""
    while mask:
        low = mask & -mask
        i = low.bit_length() - 1
        yield i
        mask ^= low


if hasattr(int, "bit_count"):
    def popcount(mask: int) -> int:
        """The number of set bits in mask."""
        return mask.bit_count()
else:
    def popcount(mask: int) -> int:
        """The number of set bits in mask."""
        return bin(mask).count("1")
