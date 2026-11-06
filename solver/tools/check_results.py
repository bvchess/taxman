#!/usr/bin/env python3
"""Independent referee for the solver's results files (`taxman solve --out`, one JSON object per line).

Standard library only, and it shares no code with the Rust crate: the rules, the bounds and the provenance rules
are written out again here, so that a bug in the solver cannot vouch for itself.  A file whose name ends in .gz is
read through gzip, every member of it, as the committed file results/optimal.jsonl.gz is.

The table is the lines of the committed file (results/optimal.jsonl.gz) unless `--table` names another file: the
lines whose scores a file's lines are compared with.  The committed file is checked as any other file is; its
comparison with the table is then each line with itself.

For every completed line:
  legal       the moves replay under the rules: a move is a number still in the pot with a proper divisor still in
              the pot, and it takes the number and all of its divisors out of the pot
  score       the replayed score is the recorded one
  over        no legal move is left at the end (one more move would only add to the score)
  first       the first move is the largest prime <= n (every optimal sequence of moves opens with it)
  promoted    the promotions are moves, in ascending order
  fields      the fields the proof and the assumptions come from agree with each other, as the solver writes them: a
              searched or repeat line, and only such a line, has a setup, which records the digest of its search
              problem; every line but a prime one records in_the_bag; only a search reaches its ceiling; a prime
              line's n is prime; a trivial line promotes nothing; a repeat, and only a repeat, records the line it
              repeats (same_search), and its setup has no `cap`; only a search counts the nodes its near-repeat cut
              cut (`stats.near_repeat_cut`, a count the proof does not rest on); a start line (a search whose start
              reached a ceiling, so that it took no step) is completed with nodes 0, and a completed search has
              nodes above 0, so a line that calls a start line a search is a problem
  optimal     the score is the table's, where the table has a line for n
  chain       score(n-1) <= score(n) <= n + score(n-1), with n-1 from the same file, else from the table
              (both are theorems about optimal scores: the moves for n-1 are legal for n, and deleting the move n from
              the moves for n leaves legal moves for n-1)
  bound       score <= the Franklin-Moniot upper bound (heuristics/results/fm_bound_2000.json)
  heuristic   score >= the best-known heuristic chain (heuristics/results/chain_1001_2000.json)
  arithmetic  a searched line: in_the_bag + promotions = score, the ceiling (`setup.cap`) is
              n + score(n-1) - in_the_bag, the promotions do not exceed it, and a line records reaching the ceiling
              exactly when its promotions sum to it (where the search stops); a prime line scores n + score(n-1) - P,
              P the largest prime below n
  provenance  the moves for n-1 a line names (prev, source run or results) are those of the file's line of n-1, and
              the line's record of them matches (score, FNV-1a digest of the moves, proof kind, assumptions); a line
              that names none (the first game of a standalone run) has no ceiling (setup.cap) and no cap or
              prime_formula proof; the recorded proof kind and assumptions are the ones the rules below give
  unfinished  an incomplete line (a search that a limit or a signal stopped) that records its moves (best_so_far,
              moves, promotions) is legal, scores the best_so_far, has ascending promotions that are moves, and is no
              better than the table's score; it need not be over.  A file may start above 1 or have a gap, where a
              run began with a standalone game
  repeat      a repeat (method "repeat", proof same_search) names a board m whose line in the same file is completed
              and earlier, whose proof is exhausted, cap or same_search; the record's proof and
              assumptions are m's; the repeat's setup.digest, its record's digest and m's setup.digest agree, and so do
              the two setups' frames, candidates and max_total; the promotions are m's, and promotion_sum is their sum;
              score(n) - in_the_bag(n) = score(m) - in_the_bag(m)
and across files: every board that two files share has the same score in both.

What this referee cannot check, because it does not rebuild the frame model: that an exhausted search explored its
whole tree (it trusts the line's proof kind, as the Rust verifier must too), and that a repeat's search problem is
m's.  The digests a line records are hashes of the problems, which only the solver can compute; this referee checks
that they agree and that the two lines are consistent with sharing one problem, and `taxman verify-results` sets
both problems up again and compares them.

Proof kinds (solver/frame/src/results.rs explains them): trivial, exhausted, cap, prime_formula, same_search.
The assumptions of a line are what its proof kind takes as given beyond the frame model: a proof by the ceiling (`cap`)
or by the prime formula rests on the moves for n-1, so it assumes what the proof kind of their line assumes; a repeat
assumes what the line it repeats assumes; other proof kinds assume nothing.  A line of another schema than this
referee reads is a problem.

The report counts lines by proof kind and by assumption, and the arithmetic certificates: scores equal to
n + score(n-1) or to the prime formula, which certify a score whatever found it, given that the moves for n-1 are
optimal.
It also counts, by proof kind, the searches that the near-repeat cut shortened: those with a stats.near_repeat_cut
above 0.

usage: check_results.py [FILE ...] [--fm PATH] [--chain PATH] [--table PATH]
Without FILEs it checks results/*.jsonl and results/*.jsonl.gz.
The exit status is 1 if any check fails.
"""
import argparse
import gzip
import json
import re
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RESULTS = REPO / "results"
COMMITTED = RESULTS / "optimal.jsonl.gz"  # the committed file: the table of optimal moves and the committed chain
FM_BOUND = REPO / "heuristics" / "results" / "fm_bound_2000.json"
HEURISTIC_CHAIN = REPO / "heuristics" / "results" / "chain_1001_2000.json"

SCHEMA = 5  # the only format of a result line this referee reads
RESTS_ON_PREV = {"prime_formula", "cap"}
SETTLES_SEARCH = {"exhausted", "cap", "same_search"}  # the proofs a repeat can repeat
SETS_UP = {"start", "search", "repeat"}  # the methods whose lines have a setup
SEARCHED = {"start", "search"}  # the methods of a line that set a search up to find its moves
NO_BAG = {"prime"}  # the methods whose lines have no in_the_bag: they set no board up
DIGEST = re.compile("[0-9a-f]{16}")
PREV_SOURCES = {"run", "results"}


def read_text(path):
    """The text of a results file; a path whose name ends in .gz is read through gzip, every member of it."""
    path = Path(path)
    if path.name.endswith(".gz"):
        with gzip.open(path, "rt", encoding="utf-8") as f:
            return f.read()
    return path.read_text()


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
    """(score, legal moves left) of the moves, or raises ValueError naming the first illegal move."""
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


def finished_problems(n, moves, divisors):
    """(score, problems) of a line that claims to be finished: the replayed score (None if the moves are illegal, and
    then the one problem says why) and what else is wrong with it: not over, or not opening with the largest prime."""
    try:
        score, left = referee(n, moves, divisors)
    except ValueError as e:
        return None, [f"illegal: {e}"]
    found = []
    if left:
        found.append(f"not over: legal moves left {left[:8]}")
    if n >= 2 and (not moves or moves[0] != largest_prime_at_most(n)):
        found.append(f"opens with {moves[:1]}, not the largest prime {largest_prime_at_most(n)}")
    return score, found


def read_table(path):
    """The score of each completed line of the results file `path` by n: the table that a file's lines are compared
    with (none, if there is no file).  The Rust program's built-in table is the committed file as of its build, which
    a test or a newer checkout can make differ from the file."""
    if path is None:
        return {}
    rows = (json.loads(text) for text in read_text(path).split("\n") if text.strip())
    return {row["n"]: row["score"] for row in rows if row.get("schema") == SCHEMA and row.get("completed")}


def read_lines(path, problems):
    """The parsed lines of a results file as (line number, object); an unparsable last line is torn."""
    raw = read_text(path).split("\n")
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


def near_repeat_cut(row):
    """The nodes the near-repeat cut cut, as the line of a search armed with it records them (`stats.near_repeat_cut`),
    else None."""
    stats = row.get("stats")
    return stats.get("near_repeat_cut") if isinstance(stats, dict) else None


def is_count(value):
    """Whether `value` is a count: a whole number, not a boolean, and not below 0."""
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def contradictions(n, row, reached):
    """What the fields of a line say against each other: combinations the solver never writes, through which a
    hand-edited line could change the proof or the assumptions derived from it."""
    method, setup = row["method"], row.get("setup")
    found = []
    if (method in SETS_UP) != (setup is not None):
        found.append(f"a {method} game without a setup" if method in SETS_UP else f"a {method} game with a setup")
    repeat = method == "repeat"
    if repeat != (row.get("same_search") is not None):
        found.append(f"a {method} game {'without' if repeat else 'with'} a same_search record")
    if setup is not None:
        digest = setup.get("digest")
        if not (isinstance(digest, str) and DIGEST.fullmatch(digest)):
            found.append(f"setup.digest {digest!r} is not 16 hexadecimal digits")
        if repeat and setup.get("cap") is not None:
            found.append("a repeat game whose setup has a cap, which only a search has")
    if (row.get("in_the_bag") is None) != (method in NO_BAG):
        found.append(f"a {method} game {'without' if method not in NO_BAG else 'with'} in_the_bag")
    if reached and method not in SEARCHED:
        found.append(f"a {method} game that reached a cap, which only a search does")
    cut = near_repeat_cut(row)
    if cut is not None and not is_count(cut):
        found.append(f"stats.near_repeat_cut {cut!r} is not a count")
    elif cut is not None and method not in SEARCHED:
        found.append(f'a {method} game with "stats.near_repeat_cut", which only a search has')
    # A start game is a completed search that took no step: its start reached a ceiling.  A completed search with
    # nodes 0 is a start game with the wrong label.
    nodes = row.get("nodes")
    if method == "start" and nodes != 0:
        found.append(f"a start game with nodes {nodes}, which took no step")
    if method == "search" and nodes == 0:
        found.append("a completed search with nodes 0, which took no step: a start game")
    if method == "prime" and not is_prime(n):
        found.append(f"a prime game, but {n} is not prime")
    if method == "trivial" and row["promotions"]:
        found.append("a trivial game with promotions")
    return found


def reached_cap(row):
    """Whether the game reached its cap, as the line records."""
    return row["reached_cap"]


def proof_of(row):
    method = row["method"]
    if method in ("trivial", "prime", "repeat"):
        return {"trivial": "trivial", "prime": "prime_formula", "repeat": "same_search"}[method]
    return "cap" if reached_cap(row) else "exhausted"


def repeat_problems(n, number, row, done):
    """What is wrong with a repeat's line against the line of game m it repeats (done[m], if the file completed it), as
    far as the lines tell: the two search problems themselves only the solver can compare."""
    record = row["same_search"]
    m = record.get("n")
    if not isinstance(m, int) or m >= n or m not in done:
        return [f"it repeats the search of game {m}, which has no completed line in the file below it"]
    _score, _moves, proof, assumes, line, earlier = done[m]
    if line > number:
        return [f"it repeats the search of game {m}, whose line ({line}) comes after it"]
    found = []
    if proof not in SETTLES_SEARCH:
        found.append(f"it repeats the search of game {m}, whose proof ({proof}) settles no search")
    if record.get("proof") != proof or set(record.get("assumes") or []) != assumes:
        found.append(f"its record of game {m} does not match line {line}")
    digests = {(r.get("setup") or {}).get("digest") for r in (row, earlier)} | {record.get("digest")}
    if len(digests) != 1:
        found.append(f"it and game {m} (line {line}) record different search digests")
    size = [(r.get("setup") or {}).get(key) for r in (row, earlier) for key in ("frames", "candidates", "max_total")]
    if size[:3] != size[3:]:
        found.append(f"its setup's frames, candidates and max_total are not those of game {m} (line {line})")
    if row["promotions"] != earlier["promotions"]:
        found.append(f"its promotions are not those of game {m} (line {line})")
    if record.get("promotion_sum") != sum(row["promotions"]):
        found.append(f"its promotions sum to {sum(row['promotions'])}, not the promotion_sum it records")
    # (A line without in_the_bag is flagged by its own line's checks.)
    bags = [r.get("in_the_bag") for r in (row, earlier)]
    if None not in bags and row["score"] - bags[0] != earlier["score"] - bags[1]:
        found.append(f"score - in_the_bag is not that of game {m} (line {line})")
    return found


def check_unfinished(path, number, row, table, divisors, problems):
    """An incomplete line that records its moves (best_so_far, moves, promotions): the moves are legal, score the
    best_so_far, has ascending promotions that are moves, and is no better than the table's score.  It need not be
    over, which only a completed line claims."""
    best = row.get("best_so_far")
    if best is None:
        return  # a line made by hand, with no moves: it counts and checks nothing

    def fail(message):
        problems.append(f"{path.name} line {number} (n={n}): {message}")

    n, moves, promotions = row["n"], row.get("moves"), row.get("promotions")
    if not isinstance(moves, list) or not isinstance(promotions, list):
        fail("best_so_far without its moves and promotions")
        return
    try:
        score, _left = referee(n, moves, divisors)
    except ValueError as e:
        fail(f"illegal: {e}")
        return
    if score != best:
        fail(f"replays to {score}, not the recorded best_so_far {best}")
    if promotions != sorted(set(promotions)) or not set(promotions) <= set(moves):
        fail("the promotions are not moves in ascending order")
    if n in table and best > table[n]:
        fail(f"best_so_far {best} is above the table's {table[n]}")


def check_file(path, fm, heuristic, divisors, problems, table):
    """Checks one file against the table, the scores by n; returns {n: (score, moves, proof, assumptions, line, row)}
    of its completed lines, the report's counters (including the searches the near-repeat cut shortened, by proof),
    and the boards that only an unfinished search is recorded for."""
    rows = read_lines(path, problems)
    games = {}
    stopped = set()  # the boards with an incomplete line
    for number, row in rows:
        if row.get("schema") != SCHEMA:
            problems.append(
                f"{path.name} line {number} (n={row.get('n')}): schema {row.get('schema')!r} is not the schema "
                f"{SCHEMA} that this referee reads"
            )
            continue
        if not row.get("completed"):
            if row.get("method") == "start":
                problems.append(f"{path.name} line {number} (n={row.get('n')}): a start game that is not completed")
            stopped.add(row["n"])
            check_unfinished(path, number, row, table, divisors, problems)
            continue
        n = row["n"]
        if n in games:
            problems.append(f"lines {games[n][0]} and {number}: two completed lines for n={n}")
            continue
        games[n] = (number, row)

    done = {}  # n -> (score, moves, proof, assumptions, line, row)
    counts = Counter()
    certificates = Counter()
    assumed = Counter()
    shortened = Counter()  # by proof: the searches the near-repeat cut shortened
    for n in sorted(games):
        number, row = games[n]

        def fail(message):
            problems.append(f"{path.name} line {number} (n={n}): {message}")

        moves, promotions = row["moves"], row["promotions"]
        score, found = finished_problems(n, moves, divisors)
        if score is None:
            fail(found[0])
            continue
        if score != row["score"]:
            fail(f"replays to {score}, not the recorded {row['score']}")
        for message in found:
            fail(message)
        if promotions != sorted(set(promotions)) or not set(promotions) <= set(moves):
            fail("the promotions are not moves in ascending order")
        if n in table and score != table[n]:
            fail(f"scores {score}, but the table has {table[n]}")
        reached = reached_cap(row)
        for message in contradictions(n, row, reached):
            fail(message)
        if str(n) in fm and score > fm[str(n)]:
            fail(f"scores {score}, above the Franklin-Moniot bound {fm[str(n)]}")
        if n in heuristic and score < heuristic[n]:
            fail(f"scores {score}, below the heuristic chain's {heuristic[n]}")

        # The chain against whichever moves for n-1 there are.
        below = done.get(n - 1)
        before = below[0] if below else table.get(n - 1)
        if before is not None:
            if score < before:
                fail(f"scores {score}, less than score(n-1) = {before}")
            if score > before + n:
                fail(f"scores {score}, more than n + score(n-1) = {before + n}")

        # The predecessor the line names.
        record = row["prev"]
        source = record["source"] if record is not None else None
        prev = None  # (score, moves, proof, assumptions)
        if source in PREV_SOURCES:
            if below is None:
                fail(f"built from game {n - 1} of this file, which it does not hold")
                continue
            prev = below[:4]
            same = (
                record["n"] == n - 1
                and record["score"] == prev[0]
                and record["moves_digest"] == digest(prev[1])
                and record["proof"] == prev[2]
                and set(record["assumes"]) == prev[3]
            )
            if not same:
                fail("its record of the moves for n-1 does not match them")
        elif record is not None:
            fail(f"unknown prev.source {source!r}")

        # Arithmetic.  What rests on the moves for n-1 (the ceiling, the prime formula) needs them.
        prev_score = prev[0] if prev else None
        total = sum(promotions)
        setup = row.get("setup") or {}
        cap = setup.get("cap")
        bag = row.get("in_the_bag")
        if bag is not None and score != bag + total:
            fail(f"scores {score}, not in_the_bag {bag} + promotions {total}")
        if cap is not None and prev_score is None:
            fail("it has a ceiling (a cap in its setup), which needs moves for n-1 it names")
        if cap is not None:
            if total > cap:
                fail(f"promotions {total} exceed the cap {cap}")
            if prev_score is not None and prev_score + n - cap != score - total:
                fail(f"the cap {cap} is not n + score(n-1) - in_the_bag")
        # The search stops as soon as its incumbent reaches the cap, and only then.
        if reached and total != cap:
            fail(f"records reaching the cap {cap}, but its promotions sum to {total}")
        if not reached and total == cap:
            fail(f"its promotions reach the cap {cap}, but it records not reaching it")
        proof = proof_of(row)
        replaced = largest_prime_at_most(n - 1)
        if row["method"] == "prime" and prev_score is not None and score != prev_score + n - replaced:
            fail(f"a prime game must score n + score(n-1) - {replaced}")

        # The line a repeat repeats.
        repeated = None
        if row["method"] == "repeat" and isinstance(row.get("same_search"), dict):
            for message in repeat_problems(n, number, row, done):
                fail(message)
            m = row["same_search"].get("n")
            repeated = done.get(m) if isinstance(m, int) and m < n else None

        # Proof and assumptions.
        if proof in RESTS_ON_PREV and prev is None:
            fail(f"its proof ({proof}) rests on moves for n-1 it does not name")
        if repeated is not None:
            assumes = repeated[3]
        else:
            assumes = prev[3] if prev and proof in RESTS_ON_PREV else set()
        if row["proof"] != proof:
            fail(f"records the proof {row['proof']}, but the rules give {proof}")
        if set(row["assumes"]) != assumes:
            fail(f"records the assumptions {sorted(row['assumes'])}, but the rules give {sorted(assumes)}")

        done[n] = (score, moves, proof, assumes, number, row)
        counts[proof] += 1
        cut = near_repeat_cut(row)
        if is_count(cut) and cut > 0:
            shortened[proof] += 1
        assumed.update(assumes)
        if not assumes:
            assumed["(nothing)"] += 1
        if prev_score is not None:
            if score == prev_score + n:
                certificates["n + score(n-1)"] += 1
            elif is_prime(n) and score == prev_score + n - replaced:
                certificates["prime formula"] += 1
    return done, counts, assumed, certificates, shortened, sorted(stopped - set(games))


def if_file(path):
    """`path` if it is a file, else None: a table that is not given is a path that is not there."""
    return path if path.is_file() else None


def listing(counter):
    return ", ".join(f"{key} {count}" for key, count in sorted(counter.items())) or "none"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("files", nargs="*", type=Path)
    ap.add_argument("--fm", type=Path, default=FM_BOUND)
    ap.add_argument("--chain", type=Path, default=HEURISTIC_CHAIN)
    ap.add_argument("--table", type=Path, default=COMMITTED, help="the file of the table of optimal moves")
    args = ap.parse_args()
    files = args.files or sorted(f for pattern in ("*.jsonl", "*.jsonl.gz") for f in RESULTS.glob(pattern))
    files = [f for f in files if not f.name.endswith(".rejected.jsonl")]

    table = read_table(if_file(args.table))
    fm = json.loads(args.fm.read_text())
    heuristic = {entry["n"]: entry["score"] for entry in json.loads(args.chain.read_text())}
    largest = 2
    for path in files:
        for line in read_text(path).split("\n"):
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
        done, counts, assumed, certificates, shortened, unfinished = check_file(
            path, fm, heuristic, divisors, problems, table
        )
        if done:
            missing = sum(1 for n in range(min(done), max(done) + 1) if n not in done)
            print(f"  {len(done)} completed lines, n = {min(done)}..{max(done)} ({missing} missing)")
        if unfinished:
            print(f"  unfinished: {len(unfinished)} games have only an unfinished search "
                  f"(n = {unfinished[0]} to {unfinished[-1]}); they are not proven")
        print("  proof: " + listing(counts))
        print("  assumed: " + listing(assumed))
        print("  arithmetic certificates: " + listing(certificates))
        print("  searches shortened by the near-repeat cut, by proof: " + listing(shortened))
        for n, (score, *_rest) in done.items():
            if n in seen:
                shared.add(n)
                other, other_score = seen[n]
                if other_score != score:
                    problems.append(f"n={n}: {other} scores {other_score}, {path} scores {score}")
            else:
                seen[n] = (path, score)
    print(f"{len(shared)} games are in more than one file")
    if problems:
        print(f"{len(problems)} PROBLEMS:")
        for problem in problems:
            print("  " + problem)
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
