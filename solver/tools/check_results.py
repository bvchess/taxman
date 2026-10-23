#!/usr/bin/env python3
"""Independent referee for the solver's results files (`taxman solve --out`, one JSON object per line).

Standard library only, and it shares no code with the Rust crate: the rules, the bounds and the provenance rules
are written out again here, so that a bug in the solver cannot vouch for itself.

For every completed game:
  legal       the moves replay under the rules: a move is a number still in the pot with a proper divisor still in
              the pot, and it takes the number and all of its divisors out of the pot
  score       the replayed score is the recorded one
  over        no legal move is left at the end (one more move would only add to the score)
  first       the first move is the largest prime <= n (every optimal game opens with it)
  promoted    the promotions are moves, in ascending order
  fields      the fields the proof and the assumptions come from agree with each other, as the solver writes them: a
              searched or repeat game, and only such a game, has a setup, which records the digest of its search
              problem; every game but a prime or a known one records in_the_bag; only a search reaches its ceiling;
              a prime game's n is prime; a trivial game promotes nothing; a repeat, and only a repeat, records the
              game it repeats (same_search), and its setup has no `cap`; a known game, and a search that the bounds
              store's upper bound ended, and only those, cite a source (`known`); only a search counts the nodes its
              near-repeat cut cut (`stats.near_repeat_cut`, a count the proof does not rest on); a start game (a
              search whose starting game reached a ceiling, so that it took no step) is completed with nodes 0, and a
              completed search has nodes above 0, so a line of an older build, which called a start game a search,
              is a problem
  optimal     the score is optimal.json's, where the table has the game
  chain       score(n-1) <= score(n) <= n + score(n-1), with n-1 from the same file, else from optimal.json
              (both are theorems about optimal games: a game of n-1 is a game of n, and deleting the move n from
              a game of n leaves a game of n-1)
  bound       score <= the Franklin-Moniot upper bound (heuristics/results/fm_bound_2000.json)
  heuristic   score >= the best-known heuristic chain (heuristics/results/chain_1001_2000.json)
  arithmetic  a searched game: in_the_bag + promotions = score, the ceiling (`setup.cap`) is
              n + score(n-1) - in_the_bag, the promotions do not exceed it, and a game records reaching the ceiling
              exactly when its promotions sum to it (where the search stops); a prime game scores n + score(n-1) - P,
              P the largest prime below n
  provenance  the game of n-1 a line names (prev) is the file's game of n-1 (source run or results), optimal.json's
              (source optimal), the committed chain's (strict) or a board the bounds store has proven (bounds), and
              the line's record of it matches that game (score, FNV-1a digest of the moves, proof, assumptions; not
              the digest of a store's game, which only the Rust program can play; a record of the proof known, of
              the file's own game, needs no more than the same score, moves and assumptions); the recorded proof
              and assumptions are the ones the rules below give
  known       a line that cites a known source (`known`: optimal, strict, or bounds with the rule and run of its lower
              and upper lines) has its citation checked: optimal.json or the committed chain (--committed) holds a
              game of n that scores the line's score, or the bounds store (--bounds) has a lower line (if cited) and
              an upper line of board n worth the score, of that rule and from that run.  The assumptions are those
              of the cited lines: optimal.json's game assumed optimal, the committed line's own, and statement U at
              each board the upper line rests on.  The sources are read as they are; their own referees check them
  unfinished  an incomplete line that records its game (best_so_far, moves, promotions) is legal, scores the
              best_so_far, has ascending promotions that are moves, and is no better than optimal.json's score; it need
              not be over.  A line whose prev has source run or results and no proof was built on such a game
              (`solve --keep-going`): an earlier incomplete line of n-1 has that score and those moves.  That game seeded
              the line and proves nothing, so the line has no ceiling (setup.cap), no cap or prime_formula proof, and
              scores no less than it.  A file may have holes (games with no line, or only incomplete lines)
  repeat      a repeat (method "repeat", proof same_search) names a game m that is a completed game of the same file
              on an earlier line, whose proof is exhausted, cap, known or same_search; the record's proof and
              assumptions are m's; the repeat's setup.digest, its record's digest and m's setup.digest agree, and so do
              the two setups' frames, candidates and max_total; the promotions are m's, and promotion_sum is their sum;
              score(n) - in_the_bag(n) = score(m) - in_the_bag(m)
and across files: every game that two files share has the same score in both.

What this referee cannot check, because it does not rebuild the frame model: that an exhausted search explored its
whole tree (it trusts the line's proof kind, as the Rust verifier must too), and that a repeat's search problem is
m's.  The digests a line records are hashes of the problems, which only the solver can compute; this referee checks
that they agree and that the two games are consistent with sharing one problem, and `taxman verify-results` sets
both problems up again and compares them.

Proof kinds (solver/frame/src/results.rs explains them): trivial, exhausted, cap, prime_formula, same_search, known.
The assumptions of a game are what its proof takes as given beyond the frame model: a proof by the ceiling (`cap`) or
by the prime formula rests on the game of n-1, so it assumes what that game's proof assumes, and for a game of n-1
taken from optimal.json, that "the game of n-1 in optimal.json is optimal"; a repeat assumes what the game it repeats
assumes; a known proof assumes what its source's game does; other proofs assume nothing.  A line of another schema
than this referee reads is a problem.

The report counts games by proof kind and by assumption, and the arithmetic certificates: scores equal to
n + score(n-1) or to the prime formula, which certify a game whatever found it, given that the game of n-1 is optimal.
It also counts, by proof kind, the searches that the near-repeat cut shortened: those with a stats.near_repeat_cut
above 0.

usage: check_results.py [FILE ...] [--fm PATH] [--chain PATH] [--committed PATH] [--bounds PATH]
Without FILEs it checks results/*.jsonl but the bounds store, which is not a file of results.  The exit status is 1
if any check fails.
"""
import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
RESULTS = REPO / "results"
COMMITTED = RESULTS / "strict.jsonl"  # the committed chain, a source of known games
BOUNDS = RESULTS / "bounds.jsonl"  # the bounds store, the other (it is not a results file)
OPTIMAL = RESULTS / "optimal.json"
FM_BOUND = REPO / "heuristics" / "results" / "fm_bound_2000.json"
HEURISTIC_CHAIN = REPO / "heuristics" / "results" / "chain_1001_2000.json"

SCHEMA = 5  # the only format of a result line this referee reads
RESTS_ON_PREV = {"prime_formula", "cap"}
SETTLES_SEARCH = {"exhausted", "cap", "same_search", "known"}  # the proofs a repeat can repeat
SETS_UP = {"start", "search", "repeat"}  # the methods whose lines have a setup
SEARCHED = {"start", "search"}  # the methods of a game that set a search up to find its game
NO_BAG = {"prime", "known"}  # the methods whose lines have no in_the_bag: they set no board up
DIGEST = re.compile("[0-9a-f]{16}")
PREV_SOURCES = {"run", "results", "optimal", "strict", "bounds"}


def assumed_optimal(n):
    """What a proof that rests on optimal.json's game of n assumes."""
    return f"the game of {n} in optimal.json is optimal"


U_STATEMENT = "statement U holds at n="


def assumed_u(n):
    """What a bound of the bounds store that rests on statement U at the board n assumes."""
    return f"{U_STATEMENT}{n}"


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


def finished_problems(n, moves, divisors):
    """(score, problems) of a game that claims to be finished: the replayed score (None if the game is illegal, and
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


class Sources:
    """The known sources a line that cites one is checked against: optimal.json, the committed chain and the bounds
    store, read as they are (their own referees check them: `taxman verify-results`, this script on the chain;
    `ledger verify-bounds`, `check_bounds.py`).  A source that was not given is empty, and a line that cites it is a
    problem."""

    def __init__(self, optimal, committed=None, bounds=None):
        self.optimal = optimal
        self.committed = {}  # n -> (score, moves, proof, assumptions) of the committed chain's completed games
        self.lines = {}  # (n, side, value) -> the store's first line of that board, side and value
        self.lower = {}  # n -> the store's best lower line (the highest value, the later of equals)
        self.upper = {}  # n -> its best upper line (the lowest value, the later of equals)
        self.have_store = False
        for line in self._rows(committed):
            if line.get("schema") == SCHEMA and line.get("completed"):
                self.committed[line["n"]] = (line["score"], line["moves"], line["proof"], set(line["assumes"]))
        if bounds is not None:
            self.have_store = True
            for line in self._rows(bounds):
                n, side, value = line["n"], line["side"], line["value"]
                self.lines.setdefault((n, side, value), line)
                best, better = (self.lower, lambda a, b: a <= b) if side == "lower" else (self.upper, lambda a, b: a >= b)
                if n not in best or better(best[n]["value"], value):
                    best[n] = line

    @staticmethod
    def _rows(path):
        if path is None:
            return []
        return [json.loads(text) for text in Path(path).read_text().split("\n") if text.strip()]

    def proven(self, n):
        """The store's upper line of the board n if its best bounds meet, else None."""
        lower, upper = self.lower.get(n), self.upper.get(n)
        return upper if lower and upper and lower["value"] == upper["value"] else None

    def prev_game(self, source, n):
        """(score, moves or None, proof, assumptions) of the game of n that `source` ("strict" or "bounds") holds, as
        the game of n-1 of a line is recorded: None if it holds none.  The store's moves are a game the Rust program
        rebuilds from promotions, which this script cannot, so they are not known (their digest is not checked)."""
        if source == "strict":
            return self.committed.get(n)
        upper = self.proven(n)
        if upper is None:
            return None
        return (upper["value"], None, "known", {assumed_u(board) for board in upper.get("rests_on", [])})

    def cite_problems(self, n, score, cite):
        """(assumptions, problems) of a line that cites a source: the source holds a game of n that scores `score`, or
        lines of the store with that value as their bound of n, by the rules and from the runs cited.  The assumptions
        are what the cited lines make the game assume."""
        allowed = {"optimal": {"source"}, "strict": {"source"}, "bounds": {"source", "lower", "upper"}}
        source = cite.get("source")
        if source not in allowed:
            return None, [f"known cites the source {source!r}, which is none of optimal, strict and bounds"]
        extra = sorted(set(cite) - allowed[source])
        if extra:
            return None, [f"known has the field {extra[0]!r}, which a citation of {source} does not"]
        if source == "optimal":
            entry = self.optimal.get(n)
            if entry is None or entry["score"] != score:
                return None, [f"it cites optimal.json, which has no game of {n} scoring {score}"]
            return {assumed_optimal(n)}, []
        if source == "strict":
            game = self.committed.get(n)
            if game is None or game[0] != score:
                return None, [f"it cites the committed chain, which has no game of {n} scoring {score}"]
            return set(game[3]), []
        if not self.have_store:
            return None, ["it cites the bounds store, which was not given to the check"]
        problems = []

        def find(side, wanted):
            line = self.lines.get((n, side, score))
            ok = isinstance(wanted, dict) and set(wanted) == {"rule", "source"}
            if line is None:
                problems.append(f"the bounds store has no {side} bound of board {n} worth {score}")
            elif not ok or (line["rule"], line["source"]) != (wanted["rule"], wanted["source"]):
                problems.append(f"the bounds store's {side} bound of board {n} worth {score} is not the one cited")
            return line

        if "lower" not in cite or "upper" not in cite:
            return None, ["a citation of the bounds store has its lower and upper bound (the lower null for a search)"]
        if cite["lower"] is not None:
            find("lower", cite["lower"])
        upper = find("upper", cite["upper"])
        if problems:
            return None, problems
        return {assumed_u(board) for board in upper.get("rests_on", [])}, []


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


def near_repeat_cut(row):
    """The nodes the near-repeat cut cut, as the line of a search armed with it records them (`stats.near_repeat_cut`),
    else None: the lines made before the cut do not have the key."""
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
    # A start game is a completed search that took no step: its starting game reached a ceiling.  A search with nodes
    # 0 is a line of an older build, which called it a search.
    nodes = row.get("nodes")
    if method == "start" and nodes != 0:
        found.append(f"a start game with nodes {nodes}, which took no step")
    if method == "search" and nodes == 0:
        found.append("a completed search with nodes 0, which took no step: a start game")
    known = row.get("known")
    kind = known.get("source") if isinstance(known, dict) else None
    if known is None and method == "known":
        found.append("a known game that cites no source (known)")
    elif known is not None and not isinstance(known, dict):
        found.append("known is not an object")
    elif known is not None and method not in SEARCHED | {"known"}:
        found.append(f"a {method} game with a known citation, which only a known or searched game has")
    elif method in SEARCHED and known is not None and (kind != "bounds" or known.get("lower") is not None):
        found.append("a searched game that cites something other than the store's upper bound")
    elif method in SEARCHED and known is not None and reached:
        found.append("a searched game that reached its cap and cites the store's bound as well")
    elif method == "known" and kind == "bounds" and known.get("lower") is None:
        found.append("a known game that cites the store without the lower bound whose game it plays")
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
    if method == "known" or row.get("known") is not None:
        return "known"
    if method in ("trivial", "prime", "repeat"):
        return {"trivial": "trivial", "prime": "prime_formula", "repeat": "same_search"}[method]
    return "cap" if reached_cap(row) else "exhausted"


def repeat_problems(n, number, row, done):
    """What is wrong with a repeat's line against the game m it repeats (done[m] if the file completed it), as far as
    the lines tell: the two search problems themselves only the solver can compare."""
    record = row["same_search"]
    m = record.get("n")
    if not isinstance(m, int) or m >= n or m not in done:
        return [f"it repeats the search of game {m}, which is not a completed game of the file below it"]
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
    # (A game without in_the_bag is flagged by its own line's checks.)
    bags = [r.get("in_the_bag") for r in (row, earlier)]
    if None not in bags and row["score"] - bags[0] != earlier["score"] - bags[1]:
        found.append(f"score - in_the_bag is not that of game {m} (line {line})")
    return found


def check_unfinished(path, number, row, optimal, divisors, unfinished, problems):
    """An incomplete line that records its game (best_so_far, moves, promotions): the game is legal, scores the
    best_so_far, has ascending promotions that are moves, and is no better than optimal.json's score.  It need not be
    over, which only a completed game claims.  The line is kept for the games built on it."""
    best = row.get("best_so_far")
    if best is None:
        return  # a line made by hand, with no game: it counts and seeds nothing

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
    if n in optimal and best > optimal[n]["score"]:
        fail(f"best_so_far {best} is above optimal.json's {optimal[n]['score']}")
    unfinished.setdefault(n, []).append((number, best, digest(moves), moves))


def check_file(path, optimal, fm, heuristic, divisors, problems, sources):
    """Checks one file; returns {n: (score, moves, proof, assumptions, line, row)} of its completed games, the
    report's counters (including the searches the near-repeat cut shortened, by proof), and the games that only an
    unfinished search is recorded for."""
    rows = read_lines(path, problems)
    games = {}
    unfinished = {}  # n -> [(line, best_so_far, digest of the moves)] of the incomplete lines that record their game
    stopped = set()  # the games with an incomplete line
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
            check_unfinished(path, number, row, optimal, divisors, unfinished, problems)
            continue
        n = row["n"]
        if n in games:
            problems.append(f"lines {games[n][0]} and {number}: two completed games of n={n}")
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
        if n in optimal and score != optimal[n]["score"]:
            fail(f"scores {score}, but optimal.json has {optimal[n]['score']}")
        reached = reached_cap(row)
        for message in contradictions(n, row, reached):
            fail(message)
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
        record = row["prev"]
        source = record["source"] if record is not None else None
        prev = None  # (score, moves, proof, assumptions)
        proven = True  # whether the game of n-1 is optimal (given its assumptions); not the best game of a stopped search
        if source in ("run", "results") and record["proof"] is None:
            # The best game of an unfinished search of n-1 (`solve --keep-going`): an earlier incomplete line of n-1
            # with that score and those moves.
            found = [
                u
                for u in unfinished.get(n - 1, [])
                if u[0] < number and u[1] == record["score"] and u[2] == record["moves_digest"]
            ]
            if not found:
                fail("built on the best game of an unfinished search of n-1, and no earlier line records one with that "
                     "score and those moves")
                continue
            prev = (found[0][1], found[0][3], None, set())
            proven = False
            if score < prev[0]:
                fail(f"scores {score}, less than the {prev[0]} of the unfinished search of n-1 it was built on")
        elif source in ("run", "results"):
            if below is None:
                fail(f"built from game {n - 1} of this file, which it does not hold")
                continue
            prev = below[:4]
        elif source == "optimal":
            # A game of optimal.json is assumed optimal; the table at hand may lack it (a line made with a longer one).
            entry = optimal.get(n - 1)
            score_before = entry["score"] if entry is not None else record["score"]
            moves_before = entry["moves"] if entry is not None else None
            prev = (score_before, moves_before, None, {assumed_optimal(n - 1)})
        elif source in ("strict", "bounds"):
            # The committed chain's game, or a board the store has proven.
            prev = sources.prev_game(source, n - 1)
            if prev is None:
                fail(f"it was built on the game of n-1 of the {source} source, which holds none")
                continue
        elif record is not None:
            fail(f"unknown prev.source {source!r}")
        if prev is not None:
            # A record of the proof `known` says where the run got its copy of the game of n-1 (a known source,
            # `--use-prev`), not how that game is proven.  When the lines of such a run are appended to the file it
            # drew on, the game of n-1 is the file's own line, which carries and checks its own proof, and this
            # line's search depends only on that game's score and moves, which must still match, and on what it
            # assumes, which must too.  So the record's proof need not be the line's, if the line is the file's.
            known_copy = record["proof"] == "known" and source in ("run", "results")
            same = (
                record["n"] == n - 1
                and record["score"] == prev[0]
                and (prev[1] is None or record["moves_digest"] == digest(prev[1]))
                and (record["proof"] == prev[2] or known_copy)
                and set(record["assumes"]) == prev[3]
            )
            if not same:
                fail("its record of the game of n-1 does not match that game")

        # Arithmetic.  What rests on the game of n-1 (the ceiling, the prime formula) needs it proven.
        prev_score = prev[0] if prev and proven else None
        total = sum(promotions)
        setup = row.get("setup") or {}
        cap = setup.get("cap")
        bag = row.get("in_the_bag")
        if bag is not None and score != bag + total:
            fail(f"scores {score}, not in_the_bag {bag} + promotions {total}")
        if cap is not None and prev_score is None:
            fail("it has a ceiling (a cap in its setup), which needs a proven game of n-1 it names")
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

        # The game a repeat repeats.
        repeated = None
        if row["method"] == "repeat" and isinstance(row.get("same_search"), dict):
            for message in repeat_problems(n, number, row, done):
                fail(message)
            m = row["same_search"].get("n")
            repeated = done.get(m) if isinstance(m, int) and m < n else None

        # Proof and assumptions.
        if proof in RESTS_ON_PREV and prev is None:
            fail(f"its proof ({proof}) rests on a game of n-1 it does not name")
        elif proof in RESTS_ON_PREV and not proven:
            fail(f"its proof ({proof}) rests on the game of n-1, which it records as the best game of an unfinished "
                 "search")
        cited = None  # what the cited source makes the game assume
        if isinstance(row.get("known"), dict):
            cited, messages = sources.cite_problems(n, score, row["known"])
            for message in messages:
                fail(message)
        if repeated is not None:
            assumes = repeated[3]
        elif cited is not None:
            assumes = cited
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
        # (Statement U at a board is one assumption of many: counted together, a game once.)
        assumed.update({s for s in assumes if not s.startswith(U_STATEMENT)})
        if any(s.startswith(U_STATEMENT) for s in assumes):
            assumed["statement U at some boards"] += 1
        if not assumes:
            assumed["(nothing)"] += 1
        if prev_score is not None:
            if score == prev_score + n:
                certificates["n + score(n-1)"] += 1
            elif is_prime(n) and score == prev_score + n - replaced:
                certificates["prime formula"] += 1
    return done, counts, assumed, certificates, shortened, sorted(stopped - set(games))


def listing(counter):
    return ", ".join(f"{key} {count}" for key, count in sorted(counter.items())) or "none"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("files", nargs="*", type=Path)
    ap.add_argument("--fm", type=Path, default=FM_BOUND)
    ap.add_argument("--chain", type=Path, default=HEURISTIC_CHAIN)
    ap.add_argument("--committed", type=Path, default=COMMITTED, help="the committed chain, a source of known games")
    ap.add_argument("--bounds", type=Path, default=BOUNDS, help="the bounds store, the other source of known games")
    args = ap.parse_args()
    # (The bounds store is not a file of results: it is read for the lines that cite it.)
    files = args.files or sorted(f for f in RESULTS.glob("*.jsonl") if f.name != BOUNDS.name)
    files = [f for f in files if not f.name.endswith(".rejected.jsonl")]

    optimal = {entry["n"]: entry for entry in json.loads(OPTIMAL.read_text())}
    sources = Sources(optimal, args.committed if args.committed.is_file() else None,
                      args.bounds if args.bounds.is_file() else None)
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
        done, counts, assumed, certificates, shortened, unfinished = check_file(
            path, optimal, fm, heuristic, divisors, problems, sources
        )
        if done:
            missing = sum(1 for n in range(min(done), max(done) + 1) if n not in done)
            print(f"  {len(done)} completed games, n = {min(done)}..{max(done)} ({missing} missing)")
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
