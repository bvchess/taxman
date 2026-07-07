"""Measure how deep OneTax's and solvent's coupon re-routing chains run.

diagnose.py classifies each optimal selection m that a strategy misses; the
"taxed" fate means m left the strategy's pot as tax of some other pick, the
"killer". That killer used a coupon (a tax divisor) that, in the known
optimal game, m itself was entitled to spend. This asks: where did the
coupon m was owed actually go?

For every "taxed" miss this replays the strategy's own game to find, for
the killer's own optimal tax divisors, who actually consumed them and when.
If a divisor was swept as tax by another pick q, and q had no surviving
alternative of its own (from q's optimal tax list) at the time, the walk
recurses onto q: the reroute chain grows by one hop. It stops when a
divisor was the strategy's own *pick* rather than someone's tax
("coupon-was-picked"), when the consuming pick isn't part of the optimal
game at all ("killer-not-optimal"), when the walk revisits a pick it has
already visited ("cyclic"), or once depth 8 is reached ("deep"). This is
run for both strategies' kill_count/coupon-alive/fodder statistics, but the
chain walk itself is only meaningful for OneTax, where every pick pays (by
construction) a single tax divisor - see the killer_fodder sanity check
below, which is NOT always 1 in practice: Moniot's "refined" rescue rule
can substitute a pick that sweeps two divisors at once, so the assert is
softened into a counted, reported exception rather than a hard crash.

Usage:
    python3 chains.py [--from 500] [--to 1000] [--optimal PATH]
                       [--onetax-log PATH] [--solvent-log PATH]
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Sequence, Set, Tuple, Union

from approx import divisor_lists, solvent, one_tax
from diagnose import replay_tax_map
from verify import DEFAULT_OPTIMAL

MINI_DIR = Path(__file__).resolve().parent
DEFAULT_ONETAX_LOG = MINI_DIR / "divergence_onetax.json"
DEFAULT_SOLVENT_LOG = MINI_DIR / "divergence_solvent.json"

MAX_DEPTH = 8

# (turn, consumer_pick, role); role is "pick" if the number was itself the
# pick, "tax" if it was swept as someone else's tax.
HistoryEvent = Tuple[int, int, str]

DEPTH_BINS = ("1", "2", "3", "4-8", "deep",
              "coupon-was-picked", "killer-not-optimal", "cyclic", "anomaly")


def build_history(
    n: int, sequence: Sequence[int], divs: Sequence[List[int]]
) -> Dict[int, HistoryEvent]:
    """Replay a legal sequence, recording when/how every number leaves the pot."""
    pot = set(range(1, n + 1))
    history: Dict[int, HistoryEvent] = {}
    for turn, p in enumerate(sequence):
        tax = [d for d in divs[p] if d in pot]
        if p not in pot or not tax:
            raise RuntimeError(f"illegal sequence for game {n} at {p}")
        for d in tax:
            history[d] = (turn, p, "tax")
        history[p] = (turn, p, "pick")
        pot.difference_update(tax)
        pot.discard(p)
    return history


def kill_info(
    n: int,
    sequence: Sequence[int],
    divs: Sequence[List[int]],
    taxed: Sequence[Dict[str, Any]],
) -> Dict[int, Tuple[int, bool, int]]:
    """For each taxed miss, (kill_count, m_coupon_alive, killer_fodder).

    All three are measured against the pot just before the killer's turn
    removes anything - a second replay of the same sequence build_history
    already walked, since that one no longer has the "before" pot handy.
    """
    by_killer: Dict[int, List[Dict[str, Any]]] = {}
    for entry in taxed:
        by_killer.setdefault(entry["killer"], []).append(entry)

    pot = set(range(1, n + 1))
    result: Dict[int, Tuple[int, bool, int]] = {}
    for p in sequence:
        tax = [d for d in divs[p] if d in pot]
        if p not in pot or not tax:
            raise RuntimeError(f"illegal sequence for game {n} at {p}")
        for entry in by_killer.get(p, []):
            m = entry["m"]
            kill_count = sum(1 for d in divs[m] if d in pot)
            coupon_alive = any(c in pot for c in entry["opt_tax"])
            result[m] = (kill_count, coupon_alive, len(tax))
        pot.difference_update(tax)
        pot.discard(p)
    return result


def resolve(
    p: int,
    t_p: int,
    depth: int,
    visited: Set[int],
    coupons: Sequence[int],
    history: Dict[int, HistoryEvent],
    opt_set: Set[int],
    opt_tax_map: Dict[int, List[int]],
    anomalies: List[Tuple[int, int]],
) -> Union[int, str]:
    """Walk p's optimal coupons to find how far the reroute chain runs.

    Returns the minimum resolved depth (an int) over p's coupons, or a
    terminal string ("coupon-was-picked", "killer-not-optimal", "cyclic",
    "deep", or "anomaly" if every coupon hit the t_d < t_p consistency
    check and none could be walked at all).
    """
    if depth > MAX_DEPTH:
        return "deep"

    candidates: List[Union[int, str]] = []
    for d in coupons:
        t_d, q, role_d = history[d]  # d divides p: must have left by t_p
        if not t_d < t_p:
            # d was consumed at or after p's own turn - only possible if p
            # paid it itself (t_d == t_p). Not a crash: see module docstring.
            anomalies.append((p, d))
            continue
        if role_d == "pick":
            candidates.append("coupon-was-picked")
            continue

        # role_d == "tax": q swept d as tax at turn t_d.
        if q in visited:
            candidates.append("cyclic")
        elif q not in opt_set:
            candidates.append("killer-not-optimal")
        else:
            q_coupons = opt_tax_map[q]
            # q's own alternative: one of its optimal coupons consumed at a
            # strictly later turn, or never at all. Same-turn consumption
            # (q swept its own true coupon and the contested one together in
            # one multi-divisor pick) is not a spare alternative - it was
            # taken too, just simultaneously - so it does not count here.
            alt = any(c not in history or history[c][0] > t_d for c in q_coupons)
            if alt:
                candidates.append(depth + 1 if depth + 1 <= MAX_DEPTH else "deep")
            else:
                candidates.append(resolve(
                    q, t_d, depth + 1, visited | {q}, q_coupons,
                    history, opt_set, opt_tax_map, anomalies,
                ))

    numeric = [c for c in candidates if isinstance(c, int)]
    if numeric:
        return min(numeric)
    if candidates:
        return candidates[0]
    return "anomaly"


def depth_bucket(result: Union[int, str]) -> str:
    if isinstance(result, int):
        return str(result) if result <= 3 else "4-8"
    return result


def count_bucket(x: int) -> str:
    return str(x) if x < 3 else "3+"


class Stats:
    """Accumulated per-strategy results over the analyzed games."""

    def __init__(self) -> None:
        self.total = 0
        self.kill_count_hist: Dict[str, int] = {}
        self.coupon_alive = 0
        self.fodder_hist: Dict[str, int] = {}
        self.fodder_violations: List[Tuple[int, int, int, int]] = []
        self.depth_count: Dict[str, int] = {b: 0 for b in DEPTH_BINS}
        self.depth_sum: Dict[str, int] = {b: 0 for b in DEPTH_BINS}
        self.anomalies: List[Tuple[int, int]] = []

    def record_common(self, kill_count: int, coupon_alive: bool, fodder: int) -> None:
        self.total += 1
        b = count_bucket(kill_count)
        self.kill_count_hist[b] = self.kill_count_hist.get(b, 0) + 1
        if coupon_alive:
            self.coupon_alive += 1
        fb = count_bucket(fodder)
        self.fodder_hist[fb] = self.fodder_hist.get(fb, 0) + 1

    def record_depth(self, m: int, result: Union[int, str]) -> None:
        b = depth_bucket(result)
        self.depth_count[b] += 1
        self.depth_sum[b] += m


def analyze_onetax(
    n: int,
    divs: Sequence[List[int]],
    record: Dict[str, Any],
    opt_set: Set[int],
    opt_tax_map: Dict[int, List[int]],
    stats: Stats,
    chain_records: List[Dict[str, Any]],
) -> None:
    taxed = [e for e in record["missed"] if e["fate"] == "taxed"]
    if not taxed:
        return
    seq: List[int] = []
    one_tax(n, divs, sequence=seq)
    history = build_history(n, seq, divs)
    info = kill_info(n, seq, divs, taxed)

    for entry in taxed:
        m = entry["m"]
        killer = entry["killer"]
        kill_count, coupon_alive, fodder = info[m]
        stats.record_common(kill_count, coupon_alive, fodder)
        if fodder != 1:
            stats.fodder_violations.append((n, m, killer, fodder))

        t_p = history[killer][0]
        if not entry["killer_in_opt"]:
            result: Union[int, str] = "killer-not-optimal"
        else:
            result = resolve(
                killer, t_p, 1, {killer}, entry["killer_opt_tax"],
                history, opt_set, opt_tax_map, stats.anomalies,
            )
        stats.record_depth(m, result)
        chain_records.append({
            "n": n, "m": m, "strategy": "onetax",
            "kill_count": kill_count, "m_coupon_alive": coupon_alive,
            "depth_or_terminal": result,
        })


def analyze_solvent(
    n: int,
    divs: Sequence[List[int]],
    record: Dict[str, Any],
    stats: Stats,
    chain_records: List[Dict[str, Any]],
) -> None:
    taxed = [e for e in record["missed"] if e["fate"] == "taxed"]
    if not taxed:
        return
    seq = solvent(n, divs)
    info = kill_info(n, seq, divs, taxed)

    for entry in taxed:
        m = entry["m"]
        kill_count, coupon_alive, fodder = info[m]
        stats.record_common(kill_count, coupon_alive, fodder)
        chain_records.append({
            "n": n, "m": m, "strategy": "solvent",
            "kill_count": kill_count, "m_coupon_alive": coupon_alive,
            "depth_or_terminal": None,
        })


def print_summary(name: str, stats: Stats, is_onetax: bool) -> None:
    print(f"=== {name} ===")
    print(f"total taxed misses: {stats.total}")
    if stats.total == 0:
        print()
        return

    print("kill_count distribution:", " ".join(
        f"{b}:{stats.kill_count_hist.get(b, 0)}" for b in ("0", "1", "2", "3+")))
    print(f"m_coupon_alive: {stats.coupon_alive}/{stats.total} "
          f"({100 * stats.coupon_alive / stats.total:.1f}%)")

    if is_onetax:
        ok = stats.fodder_hist.get("1", 0)
        print(f"killer_fodder==1: {ok}/{stats.total} "
              f"(expected always, by construction)")
        if stats.fodder_violations:
            print(f"  VIOLATIONS: {len(stats.fodder_violations)} cases where "
                  f"the killer's pick swept more than one divisor (Moniot's "
                  f"rescue rule substitutes a multi-tax pick) - not asserted "
                  f"away, just counted; first few: "
                  f"{stats.fodder_violations[:5]}")
        if stats.anomalies:
            print(f"t_d < t_p consistency anomalies (coupon consumed at/after "
                  f"its own pick's turn): {len(stats.anomalies)}")

        print("chain-depth histogram (count/sum of m):")
        for b in DEPTH_BINS:
            print(f"  {b:<18} count={stats.depth_count[b]:>5} "
                  f"sum={stats.depth_sum[b]:>9}")
    else:
        print("killer_fodder distribution:", " ".join(
            f"{b}:{stats.fodder_hist.get(b, 0)}" for b in ("1", "2", "3+")))
    print()


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--from", dest="from_n", type=int, default=500)
    parser.add_argument("--to", dest="to_n", type=int, default=1000)
    parser.add_argument("--optimal", type=Path, default=DEFAULT_OPTIMAL)
    parser.add_argument("--onetax-log", type=Path, default=DEFAULT_ONETAX_LOG)
    parser.add_argument("--solvent-log", type=Path, default=DEFAULT_SOLVENT_LOG)
    args = parser.parse_args(argv)

    optimal = {g["n"]: g for g in json.loads(args.optimal.read_text())}
    onetax_log = {r["n"]: r for r in json.loads(args.onetax_log.read_text())}
    solvent_log = {r["n"]: r for r in json.loads(args.solvent_log.read_text())}

    sys.setrecursionlimit(100_000)
    divs = divisor_lists(args.to_n)

    onetax_stats = Stats()
    solvent_stats = Stats()
    chain_records: List[Dict[str, Any]] = []

    started = time.monotonic()
    analyzed = 0
    for n in range(args.from_n, args.to_n + 1):
        if n not in optimal:
            print(f"n={n}: no known optimal solution, stopping", file=sys.stderr)
            break
        analyzed += 1

        opt_moves = optimal[n]["moves"]
        opt_set = set(opt_moves)
        opt_tax_map = replay_tax_map(n, opt_moves, divs)

        if n in onetax_log:
            analyze_onetax(n, divs, onetax_log[n], opt_set, opt_tax_map,
                            onetax_stats, chain_records)
        if n in solvent_log:
            analyze_solvent(n, divs, solvent_log[n], solvent_stats, chain_records)

    elapsed = time.monotonic() - started
    print(f"analyzed {analyzed} games in {elapsed:.1f}s", file=sys.stderr)
    print()

    print_summary("OneTax", onetax_stats, is_onetax=True)
    print_summary("Solvent", solvent_stats, is_onetax=False)

    out = MINI_DIR / "divergence_chains.json"
    out.write_text(json.dumps(chain_records, separators=(",", ":")))
    print(f"{out.name}: {len(chain_records)} records, {out.stat().st_size} bytes")

    return 0


if __name__ == "__main__":
    sys.exit(main())
