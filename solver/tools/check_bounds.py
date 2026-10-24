#!/usr/bin/env python3
"""Independent referee for the bounds store (results/bounds.jsonl, one JSON line a bound; results/README.md, "bounds.jsonl", has the format).  Standard library only; it shares no code with the Rust crate.  The rules of the game
and of a finished game are solver/tools/check_results.py's (the referee of the results files), which this loads from
beside itself; the bounds' arithmetic (monotonicity N3, the ceiling lemma N4 and the prime formula N5 of
docs/theory/neighbors.md, and the repeat lemma R6 of docs/theory/search-rules.md) is written out again here, and so is
the format.

For every line:
  format      the fields a line of its side and rule has and no others; ascending `rests_on`; a repeat's two bags
  improve     the bound improves the best of its board and side so far (lines are appended, never changed), and a lower
              bound never passes an upper one
  game        a lower bound's game replays under the rules, is over, opens with the largest prime <= n, and scores the
              value of the line.  A game given as `moves` is replayed as it is.  A game given as `promotions` cannot be
              rebuilt without the frame model, so the Rust program writes it (`ledger games FILE`) and this referee
              replays what it wrote (--games): the moves must contain the promotions and score the line's value, so a
              line whose promotions the frame model cannot play is refused by the Rust verifier, and a game that is not
              a game is refused here.  A game of a monotone, ceiling or prime relation is not written but cited
              (`from` and `from_value`, no game): it is the construction of the lemma (written out again here) applied
              to the game of the line before it that gives that board that value, and is replayed in turn
  relation    an upper bound by monotone, ceiling, prime or repeat is at least what the arithmetic of the lemma gives
              from the value of the bound it cites; the cited bound is a line before it (a bound of that board at most
              that value) or a game of the chain (results/strict.jsonl, whose score is exact); and it rests on
              every board that cited bound rests on (and, for a repeat, on its own board)
  given       a given upper bound has the given game as the lower bound, at its value
and across the file: where the chain has a game of a board, the file's bounds of it enclose the chain's score.

What this referee takes on the record of the lines, because only the Rust program can compute it: that a root bound is
the root's GF(2) bound, that a repeat's bags are the boards' and that its two boards pose the same search problem
(`ledger verify-bounds` recomputes all three), and the bounds of searches (`exhausted`, `dual`, `floor`), the Lagrangian
bounds over nogoods (`nogoods`) and given games, which it lists.

usage: check_bounds.py FILE [--games GAMES] [--strict FILE]
  GAMES  the output of `ledger games FILE` (a file, or - for the standard input)
The exit status is 1 if any check fails, including a lower bound whose game could not be replayed.
"""
import argparse
import importlib.util
import json
import re
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
STRICT = REPO / "results" / "strict.jsonl"

SCHEMA = 1
MAX_N = 8191  # the widest board of the solver (solver/frame/src/solver.rs)
KEYS = {
    "schema", "n", "side", "value", "rule", "from", "from_value", "bag", "from_bag", "rests_on", "checkpoint",
    "source", "date", "promotions", "moves",
}
RELATIONS = {"monotone", "ceiling", "prime", "repeat"}
LOWER_RULES = {"start", "search", "lns", "given"} | RELATIONS
UPPER_RULES = {"root", "dual", "exhausted", "floor", "nogoods", "given"} | RELATIONS
DATE = re.compile(r"\d{4}-(\d{2})-(\d{2})")


def referee_module():
    """solver/tools/check_results.py, the referee of the results files, loaded from beside this file (`python3 -I` does
    not look there by itself)."""
    path = Path(__file__).resolve().with_name("check_results.py")
    spec = importlib.util.spec_from_file_location("check_results", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def is_int(x):
    return isinstance(x, int) and not isinstance(x, bool)


def is_list_of_ints(x):
    return isinstance(x, list) and all(is_int(m) for m in x)


def upper_by(referee, rule, source, target, value, bags=None):
    """The upper bound on opt(target) that `rule` gives from the upper bound `value` on opt(source); ValueError if the
    boards are not so related.  bags = (source's, target's), for a repeat."""
    prime, below = referee.is_prime, referee.largest_prime_at_most
    if rule == "ceiling" and target == source + 1:
        return value + target
    if rule == "monotone" and source == target + 1:
        return value
    if rule == "prime" and target == source + 1 and prime(target):
        return max(value + target - below(source), 0)
    if rule == "prime" and source == target + 1 and prime(source):
        return max(value + below(target) - source, 0)
    if rule == "repeat" and source != target and bags is not None:
        return max(value + bags[1] - bags[0], 0)
    raise ValueError(f"the {rule} bound of {target} cannot come from {source}")


def carries(referee, rule, source, target):
    """Whether the relation `rule` (named for the lemma that carries a game) carries a game from the board `source` to
    the board `target`: monotone up a board to a composite, the ceiling down a board from a composite, the prime
    formula across a prime, either way (N3 to N5, docs/theory/neighbors.md)."""
    prime = referee.is_prime
    if rule == "monotone":
        return source + 1 == target and not prime(target)
    if rule == "ceiling":
        return source == target + 1 and not prime(source)
    if rule == "prime":
        return (source + 1 == target and prime(target)) or (source == target + 1 and prime(source))
    return False


def complete(n, moves, divisors):
    """`moves` followed by legal moves for as long as there are any, the largest each time (ValueError if `moves` is
    not legal on the board 1..n)."""
    pot = [False] + [True] * n
    for turn, m in enumerate(moves, 1):
        if not 1 <= m <= n or not pot[m] or not any(pot[d] for d in divisors.of(m)):
            raise ValueError(f"move {turn} ({m}) is not legal on the board 1..{n}")
        for d in divisors.of(m):
            pot[d] = False
        pot[m] = False
    game = list(moves)
    for m in range(n, 1, -1):
        if pot[m] and any(pot[d] for d in divisors.of(m)):
            for d in divisors.of(m):
                pot[d] = False
            pot[m] = False
            game.append(m)
    return game


def carry_game(referee, rule, source, n, moves, divisors):
    """The game of the board n that the relation `rule` makes of the finished game `moves` of the board `source`."""
    if rule == "monotone":
        return complete(n, moves, divisors)
    if rule == "ceiling":
        return complete(n, [m for m in moves if m != n + 1], divisors)
    if source < n:  # the opening prime exchanged for n
        return [n] + list(moves[1:])
    below = referee.largest_prime_at_most(n)  # n = source - 1 is below the prime source: its opening exchanged for the prime below
    return [below] + list(moves[1:]) if moves and below else []


def shape_problems(referee, row):
    """What is wrong with the fields of a line, by itself."""
    extra = sorted(set(row) - KEYS)
    if extra:
        return [f"the field {extra[0]!r} is not in the format"]
    if row.get("schema") != SCHEMA:
        return [f"the schema is not {SCHEMA}"]
    n, side, rule, value = row.get("n"), row.get("side"), row.get("rule"), row.get("value")
    if not (is_int(n) and 2 <= n <= MAX_N and is_int(value) and value >= 0):
        return [f"n is not a board from 2 to {MAX_N}, the widest board of the solver, or value is not a number"]
    if side not in ("lower", "upper"):
        return [f"the side {side!r} is not lower or upper"]
    where = f"a {side} bound by {rule}"
    if rule not in (LOWER_RULES if side == "lower" else UPPER_RULES):
        return [f"{where}: there is no such rule"]
    found = []
    relation = rule in RELATIONS
    source = row.get("from")
    if (source is not None) != relation or (
        source is not None and not (is_int(source) and 1 <= source <= MAX_N and source != n)
    ):
        found.append(f"{where}: it needs a board it came from, other than its own, and only a relation has one")
    if not (isinstance(row.get("source"), str) and row["source"]):
        found.append("the source is empty")
    date = DATE.fullmatch(row.get("date") if isinstance(row.get("date"), str) else "")
    if not (date and 1 <= int(date[1]) <= 12 and 1 <= int(date[2]) <= 31):
        found.append("the date is not YYYY-MM-DD")
    upper_fields = ("from_value", "bag", "from_bag", "rests_on", "checkpoint")
    if side == "lower":
        sets, moves, cited = row.get("promotions"), row.get("moves"), row.get("from_value")
        if [sets, moves, cited].count(None) != 2:
            found.append(f"{where}: it needs its game as promotions or as moves, or the game it is carried from")
        if sets is not None and not (is_list_of_ints(sets) and sets == sorted(set(sets))):
            found.append(f"{where}: the promotions are not ascending numbers")
        if moves is not None and not is_list_of_ints(moves):
            found.append(f"{where}: the moves are not numbers")
        if cited is not None and not (is_int(cited) and cited >= 0 and is_int(source) and carries(referee, rule, source, n)):
            found.append(f"{where}: it cites a game, and that relation does not carry a game from its board to this one")
        if any(key in row for key in upper_fields if key != "from_value"):
            found.append(f"{where}: it has fields of an upper bound")
        return found
    if "promotions" in row or "moves" in row:
        found.append(f"{where}: it has a game")
    if ("from_value" in row) != relation or ("bag" in row) != (rule == "repeat") or ("from_bag" in row) != ("bag" in row):
        found.append(f"{where}: it needs the value it came from (a repeat also the two bags), and only a relation has them")
    if "checkpoint" in row and (rule != "dual" or not isinstance(row["checkpoint"], str)):
        found.append(f"{where}: only a dual bound has a checkpoint, a text")
    rests = row.get("rests_on")
    if not (is_list_of_ints(rests) and rests and rests == sorted(set(rests))):
        found.append(f"{where}: rests_on is not a list of boards, ascending, with one at least")
    elif not relation and rests != [n]:
        found.append(f"{where}: it is a bound of the frame model at its own board, so it rests on that board alone")
    ints = [row.get(key) for key in ("from_value", "bag", "from_bag") if key in row]
    if not all(is_int(x) and x >= 0 for x in ints):
        found.append(f"{where}: from_value and the bags are numbers")
    elif not found and relation:
        bags = (row["from_bag"], row["bag"]) if rule == "repeat" else None
        try:
            upper_by(referee, rule, source, n, 0, bags)
        except ValueError as e:
            found.append(str(e))
    return found


class Chain:
    """The lines checked in order, each against those before it; the same rules as `Chain` of examples/ledger/store.rs."""

    def __init__(self, referee, chain_scores):
        self.referee = referee
        self.chain = chain_scores  # n -> score of the committed chain's games
        self.lower = {}  # n -> [value], ascending: each improves on the one before
        self.upper = {}  # n -> [(value, rests_on)]

    def best_lower(self, n):
        return self.lower[n][-1] if n in self.lower else None

    def best_upper(self, n):
        return min((v for v, _ in self.upper.get(n, ())), default=None)

    def cites(self, n, at_most):
        """What the bounds of board `n` at most `at_most` rest on: lines before this one, and the chain's game."""
        found = [rests for value, rests in self.upper.get(n, ()) if value <= at_most]
        if n in self.chain and self.chain[n] <= at_most:
            found.append([n])
        return found

    def add(self, row):
        """Problems of the line against the lines before it; the line is added if there are none."""
        n, value, rule = row["n"], row["value"], row["rule"]
        if row["side"] == "lower":
            best, upper = self.best_lower(n), self.best_upper(n)
            if best is not None and best >= value:
                return [f"it does not improve the lower bound {best}"]
            if upper is not None and upper < value:
                return [f"the lower bound {value} is above the upper bound {upper}"]
            if "from_value" in row and row["from_value"] not in self.lower.get(row["from"], ()):
                return [f"no line before it gives a game of board {row['from']} worth {row['from_value']}"]
            self.lower.setdefault(n, []).append(value)
            return []
        best, lower = self.best_upper(n), self.best_lower(n)
        if best is not None and best <= value:
            return [f"it does not improve the upper bound {best}"]
        if lower is not None and lower > value:
            return [f"the upper bound {value} is below the lower bound {lower}"]
        if rule == "given" and lower != value:
            return ["a given bound needs the given game as the lower bound, at the same value"]
        if rule in RELATIONS:
            problems = self.follows(row)
            if problems:
                return problems
        self.upper.setdefault(n, []).append((value, row["rests_on"]))
        return []

    def follows(self, row):
        n, rule, source, from_value = row["n"], row["rule"], row["from"], row["from_value"]
        bags = (row["from_bag"], row["bag"]) if rule == "repeat" else None
        try:
            carried = upper_by(self.referee, rule, source, n, from_value, bags)
        except ValueError as e:
            return [str(e)]
        if row["value"] < carried:
            return [
                f"U({n}) is at most {carried} by {rule} from U({source}) <= {from_value}, but the line says "
                f"{row['value']}"
            ]
        cites = self.cites(source, from_value)
        if not cites:
            return [f"no line before it, and no game of the chain, bounds board {source} by {from_value}"]
        own = [n] if rule == "repeat" else []
        if not any(set(rests) | set(own) <= set(row["rests_on"]) for rests in cites):
            return [
                f"it rests on {row['rests_on']}, which leaves out some board that the bound of {source} it comes from "
                f"rests on, or its own board (a repeat)"
            ]
        return []

    def against_chain(self):
        found = []
        for n in sorted(set(self.lower) | set(self.upper)):
            if n not in self.chain:
                continue
            score = self.chain[n]
            if self.best_lower(n) is not None and self.best_lower(n) > score:
                found.append(f"n={n}: the lower bound {self.best_lower(n)} is above the chain's score {score}")
            if self.best_upper(n) is not None and self.best_upper(n) < score:
                found.append(f"n={n}: the upper bound {self.best_upper(n)} is below the chain's score {score}")
        return found


def chain_scores(path):
    """{n: score} of the completed games of a results file."""
    scores = {}
    for text in path.read_text().split("\n"):
        if text.strip():
            row = json.loads(text)
            if row.get("completed"):
                scores[row["n"]] = row["score"]
    return scores


def read_games(path):
    """{(n, value): moves} of the output of `ledger games`."""
    games = {}
    lines = sys.stdin if str(path) == "-" else path.open()
    for text in lines:
        if text.strip():
            row = json.loads(text)
            games[(row["n"], row["value"])] = row["moves"]
    return games


def ranges(boards):
    """Boards as ranges: 4400-4410, 4412."""
    out, i = [], 0
    while i < len(boards):
        j = i
        while j + 1 < len(boards) and boards[j + 1] == boards[j] + 1:
            j += 1
        out.append(f"{boards[i]}-{boards[j]}" if j > i + 1 else ", ".join(map(str, boards[i:j + 1])))
        i = j + 1
    return ", ".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("file", type=Path)
    ap.add_argument("--games", type=Path, help="the output of `ledger games FILE`, or - for the standard input")
    ap.add_argument("--strict", type=Path, default=STRICT)
    args = ap.parse_args()

    referee = referee_module()
    scores = chain_scores(args.strict)
    games = read_games(args.games) if args.games else {}
    problems = []
    rows = []
    for number, text in enumerate(args.file.read_text().split("\n"), 1):
        if not text.strip():
            continue
        try:
            row = json.loads(text)
        except json.JSONDecodeError as e:
            problems.append(f"line {number}: {e}")
            continue
        found = shape_problems(referee, row) if isinstance(row, dict) else ["not an object"]
        problems += [f"line {number} (n={row.get('n') if isinstance(row, dict) else None}): {p}" for p in found]
        if not found:
            rows.append((number, row))

    chain = Chain(referee, scores)
    for number, row in rows:
        problems += [f"line {number} (n={row['n']}): {p}" for p in chain.add(row)]
    problems += chain.against_chain()

    largest = max((row["n"] for _, row in rows if row["side"] == "lower"), default=2)
    divisors = referee.Divisors(largest)
    replayed = Counter()
    # The games that other lines cite, kept when they have replayed without a problem.
    cited = {(row["from"], row["from_value"]) for _, row in rows if row["side"] == "lower" and "from_value" in row}
    kept = {}
    for number, row in rows:
        if row["side"] != "lower":
            continue
        n, value = row["n"], row["value"]

        def fail(message):
            problems.append(f"line {number} (n={n}): the game: {message}")

        if "moves" in row:
            kind, moves = "moves", row["moves"]
        elif "promotions" in row:
            kind, moves = "promotions", games.get((n, value))
            if moves is None:
                fail("it is a promotion set and --games has no game of it; it was not replayed")
                continue
            if not set(row["promotions"]) <= set(moves):
                fail("the promotions are not all moves of the game")
        else:
            kind, source = "carried", kept.get((row["from"], row["from_value"]))
            if source is None:
                fail(f"it is carried from a game of board {row['from']} worth {row['from_value']} that was not replayed")
                continue
            try:
                moves = carry_game(referee, row["rule"], row["from"], n, source, divisors)
            except ValueError as e:
                fail(f"carried from the game of board {row['from']}: {e}")
                continue
        score, found = referee.finished_problems(n, moves, divisors)
        if score is None:
            fail(found[0])
            continue
        if score != value:
            fail(f"replays to {score}, not the {value} of the line")
        for message in found:
            fail(message)
        if score == value and not found and (n, value) in cited:
            kept[(n, value)] = moves
        replayed[kind] += 1

    lines = [row for _, row in rows]
    counts = Counter((row["side"], row["rule"]) for row in lines)
    print(f"{args.file}: {len(lines)} lines for {len({row['n'] for row in lines})} boards")
    for side in ("lower", "upper"):
        print(f"  {side} bounds: " + ", ".join(f"{rule} {k}" for (s, rule), k in sorted(counts.items()) if s == side))
    print(f"  games replayed: {sum(replayed.values())} ({replayed['promotions']} from promotion sets, "
          f"{replayed['moves']} from moves, {replayed['carried']} carried from the game of another line)")
    best = {}
    for row in lines:
        if row["side"] == "upper":
            if row["n"] not in best or row["value"] < best[row["n"]]["value"]:
                best[row["n"]] = row
    for rule in ("exhausted", "dual", "floor", "nogoods", "given"):
        boards = sorted(n for n, row in best.items() if row["rule"] == rule)
        print(f"  on record, not recomputed: {counts[('upper', rule)]} {rule} lines; best upper bound of "
              f"{len(boards)} boards" + (f": {ranges(boards)}" if boards else ""))
    proven = sorted(n for n, row in best.items() if chain.best_lower(n) == row["value"])
    print(f"  proven (best lower bound = best upper bound): {len(proven)} boards" + (f": {ranges(proven)}" if proven else ""))
    in_chain = len({row["n"] for row in lines} & scores.keys())
    print(f"  {in_chain} boards of the file are also games of the chain, whose scores its bounds enclose")
    if problems:
        print(f"{len(problems)} PROBLEMS:")
        for problem in problems[:50]:
            print("  " + problem)
        if len(problems) > 50:
            print(f"  ... and {len(problems) - 50} more")
        return 1
    print("all checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
