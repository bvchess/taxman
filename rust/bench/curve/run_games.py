#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Standalone ladder searches (steps 2-4), one process per game, resumable.

Appends one JSON line per finished game to --out and skips the games already in it (same n, threads and tag).
One invocation stops before it would exceed one Bash call: a game is started with the full search limit when the
budget allows it and otherwise with what is left; a game that runs out of that shortened limit is not recorded
(it is searched again, with a whole limit, by the next invocation).

Games: numbers, ranges A-B (with --non-primes, only the non-primes), and bandA = the first --band-size non-primes
>= A.  Examples:
    run_games.py --out calib.jsonl --tag calib --non-primes 1001-1231
    run_games.py --out bands.jsonl --tag band band1250 band1300
    run_games.py --out scaling.jsonl --tag seedopt --seed-opt --threads 4 1215 1225
"""
import argparse
import re
import subprocess
import sys
import time

from common import (INVOCATION_BUDGET, LADDER, LADDER_STRICT, SEARCH_LIMIT, Budget, CHAIN, append_jsonl, is_prime,
                    load_jsonl, log_load, non_primes_from)

LINE = re.compile(
    r"n=\s*(?P<n>\d+) T=(?P<threads>\d+) (?P<status>\w+)\s+search\s+(?P<search_s>[\d.]+) s "
    r"nodes\s+(?P<nodes>\d+) \(\s*(?P<knps>[\d.]+) k/s\) best (?P<best>\d+) opt (?P<opt>\S+) greedy (?P<greedy>\d+) "
    r"cap (?P<cap>\S+) \| (?P<found>.*?) \| prep (?P<prep_s>[\d.]+) s \(prec (?P<prec_s>[\d.]+), "
    r"revise (?P<revise_s>[\d.]+) (?P<revise>.*?)\) max_total (?P<max_total>\d+) \| (?P<counters>.*)$"
)


def parse_line(text):
    m = LINE.search(text)
    if not m:
        return None
    g = m.groupdict()
    found = re.search(r"start (\d+), found at ([\d.]+) s, (\d+) steps", g["found"])
    eff = re.search(r"eff ([\d.]+)%", g["counters"])
    return {
        "status": g["status"],
        "search_s": float(g["search_s"]),
        "nodes": int(g["nodes"]),
        "knps": float(g["knps"]),
        "best": int(g["best"]),
        "opt": None if g["opt"] == "?" else int(g["opt"]),
        "greedy": int(g["greedy"]),
        "start": int(found.group(1)) if found else int(re.search(r"start incumbent (\d+)", g["found"]).group(1)),
        "found_s": float(found.group(2)) if found else None,
        "found_steps": int(found.group(3)) if found else None,
        "prep_s": float(g["prep_s"]),
        "max_total": int(g["max_total"]),
        "eff_pct": float(eff.group(1)) if eff else None,
        "counters": g["counters"],
    }


def expand(tokens, non_primes_only, band_size):
    games = []
    for tok in tokens:
        if tok.startswith("band"):
            games += non_primes_from(int(tok[4:]), band_size)
        elif "-" in tok:
            a, b = map(int, tok.split("-"))
            games += [n for n in range(a, b + 1) if not (non_primes_only and is_prime(n))]
        else:
            games.append(int(tok))
    return games


def run_one(n, args, seconds):
    cmd = [str(LADDER), "--threads", str(args.threads), "--seconds", str(seconds), *LADDER_STRICT]
    if args.known:
        cmd += ["--known", args.known]
    if args.seed_opt:
        cmd += ["--seed-opt"]
    elif not args.seeded:
        cmd += ["--no-seed", "--no-cap"]
    cmd.append(str(n))
    t0 = time.monotonic()
    proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
    wall = time.monotonic() - t0
    line = next((ln for ln in proc.stdout.splitlines() if ln.startswith("n=")), "")
    parsed = parse_line(line) if line else None
    record = {"n": n, "threads": args.threads, "tag": args.tag, "limit_s": seconds, "wall_s": round(wall, 2)}
    if parsed is None:
        record.update(status="ERROR", error=(proc.stderr or proc.stdout)[-400:])
    else:
        record.update(parsed)
    return record


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("games", nargs="+")
    ap.add_argument("--out", required=True)
    ap.add_argument("--tag", default="")
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--seconds", type=int, default=SEARCH_LIMIT, help="search limit of a whole run")
    ap.add_argument("--budget", type=int, default=INVOCATION_BUDGET, help="wall seconds of this invocation")
    ap.add_argument("--min-start", type=int, default=60, help="do not start a game with less budget than this")
    ap.add_argument("--known", default=str(CHAIN), help="results file with the known games ('' for none)")
    ap.add_argument("--seed-opt", action="store_true", help="seed with the game for n itself (proof only)")
    ap.add_argument("--seeded", action="store_true", help="seed from n-1 and cap, as the chain does (default: not)")
    ap.add_argument("--non-primes", action="store_true", help="drop the primes from ranges")
    ap.add_argument("--band-size", type=int, default=8)
    args = ap.parse_args()

    games = expand(args.games, args.non_primes, args.band_size)
    done = {(r["n"], r["threads"], r["tag"]) for r in load_jsonl(args.out)}
    todo = [n for n in games if (n, args.threads, args.tag) not in done]
    print(f"{len(games)} games, {len(games) - len(todo)} already in {args.out}, {len(todo)} to do", flush=True)
    budget = Budget(args.budget)
    log_load(f"{args.tag} start: {args.games}")
    for i, n in enumerate(todo):
        left = budget.left()
        if left < args.min_start:
            break
        seconds = min(args.seconds, int(left - 6))
        record = run_one(n, args, seconds)
        cut = seconds < args.seconds and record["status"] == "OPEN"
        shown = {k: record.get(k) for k in ("status", "search_s", "nodes", "best", "opt", "wall_s")}
        note = " (shortened, discarded)" if cut else ""
        print(f"n={n} T={args.threads} limit {seconds} s{note}: {shown}", flush=True)
        if cut:
            break
        if record["status"] == "ERROR":
            print("  error:", record["error"], file=sys.stderr)
        append_jsonl(args.out, record)
        todo[i] = None
    remaining = [n for n in todo if n is not None]
    print(f"remaining: {len(remaining)}" + (f" (next {remaining[0]})" if remaining else ""), flush=True)
    log_load(f"{args.tag} end")


if __name__ == "__main__":
    main()
