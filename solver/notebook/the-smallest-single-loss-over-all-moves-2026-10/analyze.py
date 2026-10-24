#!/usr/bin/env python3
"""Counts, the violations with what the true optimal game does instead, and whether another optimal game of n - 1
avoids each violation (a constructive witness, a proof that none can, or the complete list of optimal sets where the
prover has it).

Run from the top of the repository, on a complete run of run.py (every n of 2..4955):

    PYTHONPATH=heuristics nice -n 19 python3 PATH/analyze.py RUN.jsonl

It prints the summary and writes violations.tsv beside this script.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Set

from swaplib import REPO, TOP, Playability, load_games, one_loss_core

OPTIMAL_JSON = REPO / "results" / "optimal.json"
OPTSETS = REPO / "solver" / "noframe" / "data" / "optsets_2_989.jsonl"
VIOLATIONS = Path(__file__).resolve().parent / "violations.tsv"


def optimal_sets(pl: Playability, games: Dict[int, dict]) -> Dict[int, Optional[List[Set[int]]]]:
    """Every optimal set of n for the games the prover enumerated completely (2..989), None where it did not finish.
    The prover's 'others' are exchanges (out, in) from its canonical set; the recorded set of optimal.json is the
    canonical one or one of the others, and the base is found by trying each reading and keeping the ones that check."""
    table = {g["n"]: g["moves"] for g in json.loads(OPTIMAL_JSON.read_text())}
    sets: Dict[int, Optional[List[Set[int]]]] = {}
    for text in OPTSETS.read_text().splitlines():
        row = json.loads(text)
        n = row["n"]
        rec = set(table[n])
        if row["ended"] != "complete":
            sets[n] = None
            continue
        if not row["others"]:
            sets[n] = [set(games[n]["moves"])]
            continue
        found: List[Set[int]] = []
        for k in range(len(row["others"]) + 1):
            base = rec if k == 0 else (rec - set(row["others"][k - 1]["in"])) | set(row["others"][k - 1]["out"])
            ok = all(set(o["out"]) <= base and not set(o["in"]) & base for o in row["others"])
            cand = [base] + [(base - set(o["out"])) | set(o["in"]) for o in row["others"]] if ok else []
            if ok and all(pl.playable(n, s) and sum(s) == row["score"] for s in cand):
                found += [s for s in cand if s not in found]
        assert len(found) == row["sets"], (n, len(found), row["sets"])
        sets[n] = found
    return sets


def main() -> None:
    rows = {r["n"]: r for r in map(json.loads, Path(sys.argv[1]).read_text().splitlines())}
    games = load_games()
    pl = Playability(TOP)
    assert sorted(rows) == list(range(2, TOP + 1)), "the run has a gap"
    sets = optimal_sets(pl, games)

    counts = Counter()
    violations = []
    by_range = {}
    for n in range(2, TOP + 1):
        r = rows[n]
        delta = n + r["opt_prev"] - r["opt"]
        if r["kind"] == "exact":
            assert r["swap_score"] == r["opt"], n
            kind = "exact"
        elif r["kind"] == "none":
            kind = "none"
        elif r["swap_score"] == r["opt"]:
            assert r["x_min"] == delta
            kind = "holds"
        elif r["swap_score"] < r["opt"]:
            kind = "violation"
            violations.append(n)
        else:
            kind = "BUG"
        counts[kind] += 1
        by_range.setdefault((n - 1) // 1000, Counter())[kind] += 1
        r["kind2"], r["delta"] = kind, delta

    print(f"n = 2..{TOP}: {TOP - 1} games")
    for k in ("exact", "holds", "violation", "BUG", "none"):
        print(f"  {k}: {counts[k]}")
    print("by thousand (n = 1..1000 is row 0; exact / holds / violation / none):")
    for t, c in sorted(by_range.items()):
        print(f"  n in {max(t * 1000, 2)}..{min(t * 1000 + 1000, TOP)}: "
              f"{c['exact']} / {c['holds']} / {c['violation']} / {c['none']}")

    lines = ["n\tx_min\tswap_score\topt\tgap\tdelta\tdropped\tadded\talternative"]
    kinds, shapes, deltas = Counter(), Counter(), Counter()
    unknown = []
    for n in violations:
        r = rows[n]
        prev, cur = set(games[n - 1]["moves"]), set(games[n]["moves"])
        dropped, added = sorted(prev - cur), sorted(cur - prev)
        delta, x = r["delta"], r["x_min"]
        gap = r["opt"] - r["swap_score"]
        assert gap == x - delta
        shapes[(len(dropped), len(added) - 1)] += 1
        deltas["delta=0" if delta == 0 else "delta=1" if delta == 1 else "delta>=2"] += 1
        # what avoids it: another optimal game S' of n - 1 for which the rule holds (S' + n playable, or x_min(S') =
        # delta)
        alt = ""
        if delta == 0:
            s2 = cur - {n}
            order = pl.order(n - 1, s2)
            assert order is not None and pl.replay(n - 1, order) and sum(s2) == r["opt_prev"]
            assert pl.playable(n, s2 | {n})
            alt = "A: G_n minus n is an optimal game of n-1, and with n it is playable (exact)"
        elif delta == 1:
            alt = "B: none can: the rule needs the dropped move to be worth delta = 1, and 1 is never a move"
        else:
            s2 = (cur - {n}) | {delta}
            if delta not in cur and pl.playable(n - 1, s2):
                order = pl.order(n - 1, s2)
                assert pl.replay(n - 1, order) and sum(s2) == r["opt_prev"] and pl.playable(n, (s2 | {n}) - {delta})
                alt = f"C: (G_n minus n) plus {delta} is an optimal game of n-1 that meets the rule"
        if not alt:
            known = sets.get(n - 1, "outside")
            if known == "outside":
                alt = "D: unknown (no list of the optimal sets of n-1)"
            else:
                hits = []
                for s2 in known or []:
                    y = one_loss_core(pl, n, sorted(s2))
                    if pl.playable(n, s2 | {n}) or (y is not None and n + r["opt_prev"] - y == r["opt"]):
                        hits.append(sorted(s2) == sorted(games[n - 1]["moves"]))
                if known is None:
                    alt = "D: unknown (the prover's list of the optimal sets of n-1 is incomplete)"
                elif hits:
                    alt = f"E: another of the {len(known)} listed optimal sets of n-1 meets the rule"
                else:
                    alt = f"F: none of the {len(known)} listed optimal sets of n-1 meets the rule"
        kinds[alt[0]] += 1
        if alt[0] == "D":
            unknown.append(n)
        lines.append("\t".join(map(str, (n, x, r["swap_score"], r["opt"], gap, delta, dropped, added, alt))))
    VIOLATIONS.write_text("\n".join(lines) + "\n")

    gaps = [rows[n]["opt"] - rows[n]["swap_score"] for n in violations]
    print(f"\nviolations: {len(violations)}; gaps: min {min(gaps)}, max {max(gaps)}")
    print("by delta(n) = n + opt(n-1) - opt(n):", dict(deltas))
    print("(moves dropped, moves added besides n):", sorted(shapes.items(), key=lambda kv: -kv[1])[:12])
    print("does another optimal game of n-1 avoid it? A/C: yes, constructed; E: yes, from the prover's list; B/F: no "
          f"optimal game of n-1 does; D: cannot tell: {dict(sorted(kinds.items()))}")
    print("D (cannot tell) first 40:", unknown[:40], "... total", len(unknown))
    print("n = 686:", {k: rows[686][k] for k in ("x_min", "swap_score", "opt")}, "delta", rows[686]["delta"])
    print("first violations:", violations[:10], "last:", violations[-10:])


if __name__ == "__main__":
    main()
