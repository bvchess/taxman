#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Independent referee for the Rust solver's results files (`taxman solve --out`, one JSON object per line).

Standard library only, and it shares no code with the Rust crate: the rules, the bounds and the provenance rules
are written out again here, so that a bug in the solver cannot vouch for itself.

For every completed game:
  legal       the moves replay under the rules: a move is a number still in the pot with a proper divisor still in
              the pot, and it takes the number and all of its divisors out of the pot
  score       the replayed score is the recorded one
  over        no legal move is left at the end (one more move would only add to the score)
  first       the first move is the largest prime <= n (every optimal game opens with it)
  promoted    the promotions are moves, in ascending order
  chain       score(n-1) <= score(n) <= n + score(n-1), with n-1 from the same file, else from optimal.json
              (both are theorems about optimal games: a game of n-1 is a game of n, and deleting the move n from
              a game of n leaves a game of n-1)
  bound       score <= the Franklin-Moniot upper bound (heuristics/results/fm_bound_2000.json)
  heuristic   score >= the best-known heuristic chain (heuristics/results/chain_1001_2000.json)
  arithmetic  a searched game: in_the_bag + promotions = score (in_the_bag is recorded from schema 2 on, and
              otherwise follows from the cap), the cap is n + score(n-1) - in_the_bag, the promotions do not exceed
              it, and a game proven by the cap reaches it; a prime game scores n + score(n-1) - P, P the largest
              prime below n
  provenance  the predecessor a line names is in the file (or optimal.json) and, from schema 2 on, its record
              matches that game (score, FNV-1a digest of the moves, proof, taint); the recorded proof and taint
              are the ones the rules below give
and across files: every board size that two files share has the same score in both.

Proof kinds (rust/src/results.rs explains them): trivial, exhausted, cap, prime_formula, reuse_prev; a line without
a "schema" field (schema 1) proves by the cap when its promotions sum to setup.cap.  The taint is what a proof rests
on that is unproven: reuse_prev for a game it derived; revise_freedoms for a searched game whose setup.revise is
GreedyIsMax, Searched or FromCheckpoint; legacy_bound for a searched game with it on; and, for a proof by the cap,
the prime formula or reuse_prev, the taint of the game of n-1 ("optimal.json" for a game taken from that file).

The report counts games by proof kind and by taint, and the arithmetic certificates: scores equal to n + score(n-1)
or to the prime formula, which certify a game whatever found it, given that the game of n-1 is optimal.

usage: check_results.py [FILE ...] [--optimal PATH] [--fm PATH] [--chain PATH]
Without FILEs it checks rust/results/*.jsonl.  The exit status is 1 if any check fails.
"""
import argparse
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RESULTS = REPO / "rust" / "results"
OPTIMAL = REPO / "src" / "main" / "resources" / "optimal.json"
FM_BOUND = REPO / "heuristics" / "results" / "fm_bound_2000.json"
HEURISTIC_CHAIN = REPO / "heuristics" / "results" / "chain_1001_2000.json"

RESTS_ON_PREV = {"prime_formula", "cap", "reuse_prev"}
REVISED = ("GreedyIsMax", "Searched", "FromCheckpoint")


def is_prime(n):
    return n >= 2 and all(n % d for d in range(2, int(n ** 0.5) + 1))


def largest_prime_at_most(n):
    """The largest prime <= n, or 0 below 2."""
    return next((p for p in range(n, 1, -1) if is_prime(p)), 0)


def digest(moves):
    """64-bit FNV-1a of the moves written in decimal and separated by commas, as 16 hex digits."""
    h = 0xCBF29CE484222325
    for byte in ",".join(map(str, moves)).encode():
        h = ((h ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


class Divisors:
    """Proper divisors of 1..limit, by a sieve."""

    def __init__(self, limit):
        self.limit = limit
        self.table = [[] for _ in range(limit + 1)]
        for d in range(1, limit // 2 + 1):
            for m in range(2 * d, limit + 1, d):
                self.table[m].append(d)

    def of(self, m):
        return self.table[m]


def referee(n, moves, divisors):
    """(score, legal moves left) of the game, or raises ValueError naming the first illegal move."""
    pot = [False] + [True] * n
    score = 0
    for turn, m in enumerate(moves, 1):
        if not 1 <= m <= n or not pot[m]:
            raise ValueError(f"move {turn} ({m}) is not in the pot")
        taxed = [d for d in divisors.of(m) if pot[d]]
        if not taxed:
            raise ValueError(f"move {turn} ({m}) pays no tax")
        for d in taxed:
            pot[d] = False
        pot[m] = False
        score += m
    left = [m for m in range(2, n + 1) if pot[m] and any(pot[d] for d in divisors.of(m))]
    return score, left


def read_lines(path, problems):
    """The parsed lines of a results file as (line number, object); an unparsable last line is torn."""
    raw = path.read_text().split("\n")
    numbered = [(i + 1, text) for i, text in enumerate(raw) if text.strip()]
    rows = []
    for k, (number, text) in enumerate(numbered):
        try:
            rows.append((number, json.loads(text)))
        except json.JSONDecodeError as e:
            if k == len(numbered) - 1:
                print(f"  note: line {number} is torn (does not parse) and is ignored")
            else:
                problems.append(f"line {number}: {e}")
    return rows


def own_taint(row):
    taint = set()
    if row["method"] == "reuse":
        taint.add("reuse_prev")
    if row["method"] == "search":
        if (row.get("setup") or {}).get("revise", "").startswith(REVISED):
            taint.add("revise_freedoms")
        if row["tactic_switches"].get("legacy_bound"):
            taint.add("legacy_bound")
    return taint


def proof_of(row):
    method = row["method"]
    if method in ("trivial", "prime", "reuse"):
        return {"trivial": "trivial", "prime": "prime_formula", "reuse": "reuse_prev"}[method]
    if row.get("schema", 1) >= 2:
        reached = row["reached_cap"]
    else:
        cap = (row.get("setup") or {}).get("cap")
        reached = cap is not None and sum(row["promotions"]) >= cap
    return "cap" if reached else "exhausted"


def check_file(path, optimal, fm, heuristic, divisors, problems):
    """Checks one file; returns {n: (score, proof, taint)} of its completed games and the report's counters."""
    rows = read_lines(path, problems)
    games = {}
    for number, row in rows:
        if not row.get("completed"):
            continue
        n = row["n"]
        if n in games:
            problems.append(f"lines {games[n][0]} and {number}: two completed games of n={n}")
            continue
        games[n] = (number, row)

    done = {}  # n -> (score, moves, proof, taint, line)
    counts = Counter()
    certificates = Counter()
    taints = Counter()
    for n in sorted(games):
        number, row = games[n]

        def fail(message):
            problems.append(f"{path.name} line {number} (n={n}): {message}")

        moves, promotions = row["moves"], row["promotions"]
        try:
            score, left = referee(n, moves, divisors)
        except ValueError as e:
            fail(f"illegal: {e}")
            continue
        if score != row["score"]:
            fail(f"replays to {score}, not the recorded {row['score']}")
        if left:
            fail(f"not over: legal moves left {left[:8]}")
        if n >= 2 and (not moves or moves[0] != largest_prime_at_most(n)):
            fail(f"opens with {moves[:1]}, not the largest prime {largest_prime_at_most(n)}")
        if promotions != sorted(set(promotions)) or not set(promotions) <= set(moves):
            fail("the promotions are not moves in ascending order")
        if str(n) in fm and score > fm[str(n)]:
            fail(f"scores {score}, above the Franklin-Moniot bound {fm[str(n)]}")
        if n in heuristic and score < heuristic[n]:
            fail(f"scores {score}, below the heuristic chain's {heuristic[n]}")

        # The chain against whichever game of n-1 there is.
        below = done.get(n - 1)
        before = below[0] if below else optimal.get(n - 1, {}).get("score")
        if before is not None:
            if score < before:
                fail(f"scores {score}, less than score(n-1) = {before}")
            if score > before + n:
                fail(f"scores {score}, more than n + score(n-1) = {before + n}")

        # The predecessor the line names.
        source = row.get("prev_source")
        prev = None  # (score, moves, proof, taint)
        if source in ("run", "results"):
            if below is None:
                fail(f"built from game {n - 1} of this file, which it does not hold")
                continue
            prev = below[:4]
        elif source == "optimal":
            entry = optimal.get(n - 1)
            if entry is not None:
                prev = (entry["score"], entry["moves"], None, {"optimal.json"})
            elif row.get("prev"):
                prev = (row["prev"]["score"], None, None, {"optimal.json"})
            else:
                prev = (None, None, None, {"optimal.json"})
        elif source is not None:
            fail(f"unknown prev_source {source!r}")
        schema = row.get("schema", 1)
        if schema >= 2 and (row["prev"] is None) != (prev is None):
            fail("prev and prev_source disagree")
        if schema >= 2 and prev is not None:
            record = row["prev"]
            same = (
                record["n"] == n - 1
                and record["source"] == source
                and record["score"] == prev[0]
                and (prev[1] is None or record["moves_digest"] == digest(prev[1]))
                and record["proof"] == prev[2]
                and set(record["taint"]) == prev[3]
            )
            if not same:
                fail("its record of the game of n-1 does not match that game")

        # Arithmetic.
        prev_score = prev[0] if prev else None
        total = sum(promotions)
        setup = row.get("setup") or {}
        cap = setup.get("cap")
        bag = row.get("in_the_bag") if schema >= 2 else None
        if bag is not None and score != bag + total:
            fail(f"scores {score}, not in_the_bag {bag} + promotions {total}")
        if cap is not None:
            if total > cap:
                fail(f"promotions {total} exceed the cap {cap}")
            if prev_score is not None and prev_score + n - cap != score - total:
                fail(f"the cap {cap} is not n + score(n-1) - in_the_bag")
        proof = proof_of(row)
        if proof == "cap" and total != cap:
            fail(f"proven by the cap {cap}, but its promotions sum to {total}")
        replaced = largest_prime_at_most(n - 1)
        if row["method"] == "prime" and prev_score is not None and score != prev_score + n - replaced:
            fail(f"a prime game must score n + score(n-1) - {replaced}")

        # Proof and taint.
        if proof in RESTS_ON_PREV and prev is None:
            fail(f"its proof ({proof}) rests on a game of n-1 it does not name")
        taint = own_taint(row) | (prev[3] if prev and proof in RESTS_ON_PREV else set())
        if schema >= 2:
            if row["proof"] != proof:
                fail(f"records the proof {row['proof']}, but the rules give {proof}")
            if set(row["taint"]) != taint:
                fail(f"records the taint {sorted(row['taint'])}, but the rules give {sorted(taint)}")

        done[n] = (score, moves, proof, taint, number)
        counts[proof] += 1
        taints.update(taint)
        if not taint:
            taints["(nothing unproven)"] += 1
        if prev_score is not None:
            if score == prev_score + n:
                certificates["n + score(n-1)"] += 1
            elif is_prime(n) and score == prev_score + n - replaced:
                certificates["prime formula"] += 1
    return done, counts, taints, certificates


def listing(counter):
    return ", ".join(f"{key} {count}" for key, count in sorted(counter.items())) or "none"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("files", nargs="*", type=Path)
    ap.add_argument("--optimal", type=Path, default=OPTIMAL)
    ap.add_argument("--fm", type=Path, default=FM_BOUND)
    ap.add_argument("--chain", type=Path, default=HEURISTIC_CHAIN)
    args = ap.parse_args()
    files = args.files or sorted(RESULTS.glob("*.jsonl"))
    files = [f for f in files if not f.name.endswith(".rejected.jsonl")]

    optimal = {entry["n"]: entry for entry in json.loads(args.optimal.read_text())}
    fm = json.loads(args.fm.read_text())
    heuristic = {entry["n"]: entry["score"] for entry in json.loads(args.chain.read_text())}
    largest = 2
    for path in files:
        for line in path.read_text().split("\n"):
            try:
                largest = max(largest, json.loads(line)["n"])
            except (json.JSONDecodeError, KeyError, TypeError):
                pass
    divisors = Divisors(largest)

    problems = []
    seen = {}  # n -> (file, score)
    shared = set()
    for path in files:
        print(f"{path}:")
        done, counts, taints, certificates = check_file(path, optimal, fm, heuristic, divisors, problems)
        if done:
            missing = sum(1 for n in range(min(done), max(done) + 1) if n not in done)
            print(f"  {len(done)} completed games, n = {min(done)}..{max(done)} ({missing} missing)")
        print("  proof: " + listing(counts))
        print("  taint: " + listing(taints))
        print("  arithmetic certificates: " + listing(certificates))
        for n, (score, *_rest) in done.items():
            if n in seen:
                shared.add(n)
                other, other_score = seen[n]
                if other_score != score:
                    problems.append(f"n={n}: {other} scores {other_score}, {path} scores {score}")
            else:
                seen[n] = (path, score)
    print(f"{len(shared)} board sizes are in more than one file")
    if problems:
        print(f"{len(problems)} PROBLEMS:")
        for problem in problems:
            print("  " + problem)
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
