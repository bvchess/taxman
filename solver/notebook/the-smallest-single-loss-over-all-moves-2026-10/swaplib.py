#!/usr/bin/env python3
"""Playability through the heuristics' peel, the games of strict.jsonl, and the one-loss rule.

The scripts of this folder import the heuristics' modules (core.py and strategies/solvent.py), so run them from the top
of the repository with PYTHONPATH=heuristics.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Set

from core import Infeasible, divisor_lists, maximal_factors, peel, smallest_prime_factors
from strategies.solvent import _playable_order

REPO = Path(__file__).resolve().parents[3]
MAIN = REPO / "results" / "strict.jsonl"
CHAIN = Path.home() / "taxman_work" / "chain-data" / "results" / "strict.jsonl"
TOP = 4955


def load_games(top: int = TOP) -> Dict[int, dict]:
    """The first completed line of each n: the committed file for 1..4949, then the chain's file for the rest."""
    games: Dict[int, dict] = {}
    for path in (MAIN, CHAIN):
        for text in path.read_text().splitlines():
            line = json.loads(text)
            if line.get("completed") and line["n"] <= top:
                games.setdefault(line["n"], line)
    return games


class Playability:
    """The trusted check: peel over the maximal factors outside the set (docs/theory/decision-procedure.md, Theorems
    1-3: peel succeeds exactly when the set is playable), the same call as evaluation/wiki_questions.py's
    playable_order."""

    def __init__(self, top: int = TOP) -> None:
        self.top = top
        self.spf = smallest_prime_factors(top)
        self.mf = {c: maximal_factors(c, self.spf) for c in range(2, top + 1)}
        self.divs = divisor_lists(top)
        self.calls = 0

    def playable(self, n: int, selected: Set[int]) -> bool:
        """True if the set can be played, in some order, on the board 1..n."""
        self.calls += 1
        if 1 in selected or any(c > n or c < 2 for c in selected):
            return False
        try:
            peel(selected, set(range(1, n + 1)) - selected, self.mf)
        except Infeasible:
            return False
        return True

    def order(self, n: int, selected: Set[int]) -> Optional[List[int]]:
        """A legal order of the set (peel's matching, then the precedence sort), or None if it is unplayable."""
        if 1 in selected or any(c > n or c < 2 for c in selected):
            return None
        try:
            _, match = peel(selected, set(range(1, n + 1)) - selected, self.mf)
        except Infeasible:
            return None
        return _playable_order(set(selected), match, n)

    def replay(self, n: int, moves: Sequence[int]) -> bool:
        """True if the moves, in this order, are a legal game on 1..n (each has a divisor left, the taxman takes
        all)."""
        pot = bytearray([1]) * (n + 1)
        for c in moves:
            if c < 2 or c > n or not pot[c]:
                return False
            tax = [d for d in self.divs[c] if pot[d]]
            if not tax:
                return False
            pot[c] = 0
            for d in tax:
                pot[d] = 0
        return True

    def core(self, n: int, selected: Set[int]) -> List[int]:
        """A subset-minimal unplayable subset of an unplayable set (QuickXplain).  Playable sets are downward closed
        (a matching and an acyclic precedence survive deleting members), so unplayability is monotone upward, and
        every x whose removal makes the set playable lies in every unplayable subset, this core among them."""
        return self._qx([], False, sorted(selected), n)

    def _qx(self, base: List[int], changed: bool, rest: List[int], n: int) -> List[int]:
        if changed and not self.playable(n, set(base)):
            return []
        if len(rest) == 1:
            return list(rest)
        half = len(rest) // 2
        first, second = rest[:half], rest[half:]
        d2 = self._qx(base + first, bool(first), second, n)
        d1 = self._qx(base + d2, bool(d2), first, n)
        return d1 + d2


def one_loss_brute(pl: Playability, n: int, prev: Sequence[int]) -> Optional[int]:
    """The smallest x of the game of n - 1 whose removal from prev + {n} makes it playable, trying every x upward."""
    total = set(prev) | {n}
    for x in sorted(prev):
        if pl.playable(n, total - {x}):
            return x
    return None


def one_loss_core(pl: Playability, n: int, prev: Sequence[int]) -> Optional[int]:
    """The same x, trying only the members of one minimal unplayable subset of prev + {n}."""
    total = set(prev) | {n}
    for x in sorted(pl.core(n, total)):
        if x != n and pl.playable(n, total - {x}):
            return x
    return None
