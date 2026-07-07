"""Diagnose where OneTax and solvent diverge from optimal Taxman games.

For every game in a range where OneTax or solvent scores below the known
optimal, this classifies each missed optimal selection m (an optimal pick
the strategy never makes) into a "fate":

  taxed         m was swept as tax by some other pick the strategy made.
  sniped        m was starved down to its last divisor, then that last
                divisor was taken by a pick that left m unplayable.
  double-swept  m's divisors vanished two-or-more at a time in some single
                pick, never leaving m at exactly one divisor to snipe.
  drained       m's divisors trickled away one at a time (or the game
                ended before m was fully starved).

For solvent, each missed m is additionally tagged with the reason solvent's
own playability test rejected it ("infeasible"); solvent makes a single
descending pass, so each m is rejected at most once.

Writes two machine-readable logs, one record per diverging game, to
divergence_onetax.json and divergence_solvent.json, and prints an aggregate
summary per strategy over the analyzed range.

Usage:
    python3 diagnose.py [--from 500] [--to 1000] [--optimal PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

from approx import (
    check_sequence, divisor_lists, maximal_factor_lists, solvent, one_tax,
)
from verify import DEFAULT_OPTIMAL

MINI_DIR = Path(__file__).resolve().parent

FATES = ("taxed", "sniped", "double-swept", "drained")


def replay_tax_map(
    n: int, moves: Sequence[int], divs: Sequence[List[int]]
) -> Dict[int, List[int]]:
    """Replay a known-legal move sequence, recording each pick's tax."""
    pot = set(range(1, n + 1))
    tax_map: Dict[int, List[int]] = {}
    for p in moves:
        tax = [d for d in divs[p] if d in pot]
        if p not in pot or not tax:
            raise RuntimeError(f"illegal optimal move {p} in game {n}")
        pot -= set(tax)
        pot.discard(p)
        tax_map[p] = tax
    return tax_map


def classify_starved(
    events: List[Dict[str, Any]], remaining_left: Any
) -> Tuple[str, Optional[int]]:
    """Sub-classify a missed number that was never taxed outright."""
    if remaining_left:
        # the strategy's game ended with divisors of m still un-swept:
        # m was never fully starved of divisors, just never reached.
        return "drained", None
    if not events:
        return "drained", None  # should not happen (m always has divisor 1)
    last = events[-1]
    if last["before"] == 1:
        return "sniped", last["pick"]
    if any(len(e["hit"]) >= 2 for e in events):
        return "double-swept", None
    return "drained", None


def lookup_reason(m: int, rejections: Sequence[Tuple[int, str]]) -> str:
    reasons = [r for (x, r) in rejections if x == m]
    if not reasons:
        return "not-rejected"
    return "+".join(reasons)


def build_record(
    n: int,
    opt_score: int,
    score: int,
    opt_moves: Sequence[int],
    opt_tax_map: Dict[int, List[int]],
    seq: Sequence[int],
    divs: Sequence[List[int]],
    solvent_reasons: Optional[List[Tuple[int, str]]],
) -> Dict[str, Any]:
    opt_set = set(opt_moves)
    strat_set = set(seq)
    missed = sorted(opt_set - strat_set)
    extra = sorted(strat_set - opt_set)
    assert score - opt_score == sum(extra) - sum(missed), \
        f"score identity broken at n={n}"

    pot = set(range(1, n + 1))
    remaining = {m: set(divs[m]) for m in missed}
    events: Dict[int, List[Dict[str, Any]]] = {m: [] for m in missed}
    taxed_by: Dict[int, int] = {}
    alive = set(missed)

    for p in seq:
        tax = [d for d in divs[p] if d in pot]
        if p not in pot or not tax:
            raise RuntimeError(f"illegal strategy sequence for game {n} at {p}")
        removed = set(tax)
        removed.add(p)
        pot -= removed

        for m in list(alive):
            if m in tax:
                taxed_by[m] = p
                alive.discard(m)
            hit = remaining[m] & removed
            if hit:
                before = len(remaining[m])
                remaining[m] -= hit
                after = len(remaining[m])
                events[m].append({
                    "pick": p, "hit": sorted(hit),
                    "before": before, "after": after,
                })

    missed_entries: List[Dict[str, Any]] = []
    for m in missed:
        if m in taxed_by:
            killer = taxed_by[m]
            fate = "taxed"
            killer_in_opt = killer in opt_set
            killer_opt_tax = opt_tax_map.get(killer, [])
            sniper = None
        else:
            fate, sniper = classify_starved(events[m], remaining[m])
            killer = None
            killer_in_opt = None
            killer_opt_tax = []

        opt_tax_list = opt_tax_map[m]  # always present: m is always an optimal pick
        entry: Dict[str, Any] = {
            "m": m,
            "upper": m * 2 > n,
            "fate": fate,
            "killer": killer,
            "killer_in_opt": killer_in_opt,
            "killer_opt_tax": killer_opt_tax,
            "sniper": sniper,
            "opt_tax_count": len(opt_tax_list),
            "opt_tax": opt_tax_list,
        }
        if solvent_reasons is not None:
            entry["solvent_reason"] = lookup_reason(m, solvent_reasons)
        missed_entries.append(entry)

    return {
        "n": n,
        "opt": opt_score,
        "score": score,
        "missed": missed_entries,
        "extra": extra,
    }


def print_summary(
    name: str,
    records: List[Dict[str, Any]],
    total_games: int,
    is_solvent: bool,
) -> None:
    print(f"=== {name} ===")
    diverging = len(records)
    lost_total = sum(r["opt"] - r["score"] for r in records)
    print(f"games diverging: {diverging}/{total_games}   "
          f"total points lost: {lost_total}")

    upper_count = lower_count = 0
    upper_sum = lower_sum = 0
    fate_count = {(f, u): 0 for f in FATES for u in (True, False)}
    fate_sum = {(f, u): 0 for f in FATES for u in (True, False)}
    two_tax_count = two_tax_sum = 0
    taxed_in_opt = taxed_not_in_opt = 0
    reason_count: Dict[str, int] = {}
    reason_sum: Dict[str, int] = {}

    for r in records:
        for entry in r["missed"]:
            m = entry["m"]
            upper = entry["upper"]
            if upper:
                upper_count += 1
                upper_sum += m
            else:
                lower_count += 1
                lower_sum += m

            fate_count[(entry["fate"], upper)] += 1
            fate_sum[(entry["fate"], upper)] += m

            if entry["opt_tax_count"] == 2:
                two_tax_count += 1
                two_tax_sum += m

            if entry["fate"] == "taxed":
                if entry["killer_in_opt"]:
                    taxed_in_opt += 1
                else:
                    taxed_not_in_opt += 1

            if is_solvent:
                for token in entry["solvent_reason"].split("+"):
                    reason_count[token] = reason_count.get(token, 0) + 1
                    reason_sum[token] = reason_sum.get(token, 0) + m

    print(f"missed (upper): count={upper_count} sum={upper_sum}   "
          f"missed (lower): count={lower_count} sum={lower_sum}")

    print("fate breakdown (count/sum, upper vs lower):")
    for f in FATES:
        print(f"  {f:<13} upper: count={fate_count[(f, True)]:>4} "
              f"sum={fate_sum[(f, True)]:>8}   "
              f"lower: count={fate_count[(f, False)]:>4} "
              f"sum={fate_sum[(f, False)]:>8}")

    print(f"2-tax optimal moves missed: count={two_tax_count} sum={two_tax_sum}")
    print(f"taxed fate: killer_in_opt=True: {taxed_in_opt}   "
          f"killer_in_opt=False: {taxed_not_in_opt}")

    if is_solvent:
        print("rejection-reason histogram (atomic tokens, may overlap):")
        for token in ("infeasible",):
            c = reason_count.get(token, 0)
            s = reason_sum.get(token, 0)
            print(f"  {token:<10} count={c:>4} sum={s:>8}")

    top = sorted(records, key=lambda r: r["opt"] - r["score"], reverse=True)[:5]
    print("top 5 games by net points lost:")
    for r in top:
        counts = {f: 0 for f in FATES}
        for entry in r["missed"]:
            counts[entry["fate"]] += 1
        breakdown = " ".join(f"{f}:{counts[f]}" for f in FATES)
        print(f"  n={r['n']} lost={r['opt'] - r['score']} ({breakdown})")

    print()


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--from", dest="from_n", type=int, default=500)
    parser.add_argument("--to", dest="to_n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    sys.setrecursionlimit(100_000)
    divs = divisor_lists(args.to_n)
    mf = maximal_factor_lists(args.to_n)

    onetax_records: List[Dict[str, Any]] = []
    solvent_records: List[Dict[str, Any]] = []
    total_games = 0
    started = time.monotonic()

    for n in range(args.from_n, args.to_n + 1):
        if n not in optimal:
            print(f"n={n}: no known optimal solution, stopping", file=sys.stderr)
            break
        total_games += 1

        opt_game = optimal[n]
        opt_score = opt_game["score"]
        opt_moves = opt_game["moves"]
        opt_tax_map = replay_tax_map(n, opt_moves, divs)

        seq: List[int] = []
        score = one_tax(n, divs, sequence=seq)
        if score != opt_score:
            onetax_records.append(build_record(
                n, opt_score, score, opt_moves, opt_tax_map, seq, divs, None,
            ))

        rejections: List[Tuple[int, str]] = []
        gseq = solvent(n, mf, rejections=rejections)
        gscore = check_sequence(n, gseq)
        if gscore != opt_score:
            solvent_records.append(build_record(
                n, opt_score, gscore, opt_moves, opt_tax_map, gseq, divs,
                rejections,
            ))

    elapsed = time.monotonic() - started
    print(f"analyzed {total_games} games in {elapsed:.1f}s", file=sys.stderr)

    onetax_path = MINI_DIR / "divergence_onetax.json"
    solvent_path = MINI_DIR / "divergence_solvent.json"
    onetax_path.write_text(json.dumps(onetax_records, separators=(",", ":")))
    solvent_path.write_text(json.dumps(solvent_records, separators=(",", ":")))
    print(f"{onetax_path.name}: {onetax_path.stat().st_size} bytes")
    print(f"{solvent_path.name}: {solvent_path.stat().st_size} bytes")
    print()

    print_summary("OneTax", onetax_records, total_games, is_solvent=False)
    print_summary("Solvent", solvent_records, total_games, is_solvent=True)

    return 0


if __name__ == "__main__":
    sys.exit(main())
