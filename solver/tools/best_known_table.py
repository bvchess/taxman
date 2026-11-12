#!/usr/bin/env python3
"""Makes the two files about the games past the end of the committed chain: `results/best-known.jsonl.gz`, what is
known of each game, and `results/search-effort.jsonl`, what has been tried on it.  Standard library only.

    python3 -I solver/tools/best_known_table.py --best-known FILE --parity FILE [FILE ...] \\
        [--plays FILE[:LEVEL] ...] [--finder FILE:SECONDS ...] [--out-dir DIR]

The sources are the best-known file (`--best-known`), the files of the parity bound of each board (`--parity`), the
ledgers of the dedicated plays (`--plays`; a ledger with no level column takes the `:LEVEL` that follows its path) and
the ledger of the finder (`--finder`; the ledger does not record the time limit of its searches, so the path is
followed by it, in seconds).  The committed chain and the proven games past it are read from `results/` (`--chain`,
`--beyond`).  The files go to `--out-dir`, `results/` by default.  Nothing is read twice and no source is written.

The best-known file is read as it is when the tool starts, up to the size it has then, because other runs append to
it; a line that was only partly written is skipped.  The output follows from the sources alone: the games are in
order, so the same sources give the same files.  results/README.md, "best-known.jsonl.gz" and "search-effort.jsonl",
says what each field is and how to check the files.
"""

import argparse
import array
import bisect
import collections
import gzip
import itertools
import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
MEMBER_LINES = 500  # lines in each gzip member of the table, as in results/optimal.jsonl.gz

# The rules that settle a game from the one before it, first the ones that need no statement U, as the names the
# committed results give to their proofs, and in the order in which a game is given the first that holds.
RULES = ("prime_formula", "cap", "theorem_8p", "semiprime_formula", "theorem_12p", "same_search")

# What a row of the finder's ledger says (`transplant/driver/finder_driver.py`): the outcomes that name a better game
# are one, and an error is no answer, since the driver tries the game again.
FINDER_OUTCOME = {"none": "none", "stopped": "stopped", "gain-exact": "gain", "gain-stopped": "gain",
                  "gain-covered": "gain", "gain-not-added": "gain"}


def fail(message):
    raise SystemExit(f"best_known_table.py: {message}")


# ---------------------------------------------------------------------------------------------------------------
# Reading the sources
# ---------------------------------------------------------------------------------------------------------------

def complete_lines(path):
    """The complete lines of the file at `path` (gzip if its name ends in .gz) as bytes, up to the size it has when
    this is called: what is appended later is not read.  A last line that is not finished at that size is left out,
    and the second result says whether one was."""
    opener = gzip.open if str(path).endswith(".gz") else open
    lines, torn = [], False
    with opener(path, "rb") as f:
        size = os.fstat(f.fileno()).st_size if opener is open else None
        read = 0
        for raw in f:
            if size is not None and read >= size:
                break
            read += len(raw)
            if not raw.endswith(b"\n") or (size is not None and read > size):
                torn = True
                break
            if raw.strip():
                lines.append(raw)
    return lines, torn


def json_lines(path):
    """The complete lines of a file of JSON lines, parsed."""
    lines, _ = complete_lines(path)
    for number, raw in enumerate(lines, 1):
        try:
            yield json.loads(raw)
        except json.JSONDecodeError as e:
            fail(f"{path}: a line that is not JSON ({e}), the line after {number - 1} complete ones")


def root_problem_digest(line):
    """The digest of the root problem a results line set up (a repeat's line names it in `same_search`), or None."""
    for key in ("setup", "same_search"):
        if line.get(key):
            return line[key]["digest"]
    return None


class Game:
    """What the rules and the repeat check need of one game: its score, the sum of its sources (`in_the_bag`, the
    score a game of no promotions has), the sum of its moves above n/2 (`big`, where its moves are kept) and the digest
    of its root problem."""

    __slots__ = ("n", "score", "bag", "big", "digest", "moves", "bound")

    def __init__(self, line, keep_moves):
        self.n, self.score = line["n"], line["score"]
        self.bag, self.digest, self.bound = line.get("in_the_bag"), root_problem_digest(line), line.get("bound")
        moves = line["moves"] if keep_moves else None
        self.big = sum(x for x in moves if x > self.n // 2) if keep_moves else None
        self.moves = array.array("H", moves) if keep_moves else None

    def promotion_sum(self):
        """The score less the sum of the sources, if the game plays all its sources and some promotions (the moves
        above n/2 add up to the sources), else None."""
        if self.bag is None or (self.big is not None and self.big != self.bag):
            return None
        return self.score - self.bag


def read_proven(path, keep_moves=False):
    """The games of a file of proven results, by n."""
    return {line["n"]: Game(line, keep_moves) for line in json_lines(path)}


def read_best_known(path, first):
    """The best line of each game from `first` on in the best-known file: the highest score, the later line of two
    with one score.  Returns the games by n and how many lines and bytes were read, and whether a line was torn."""
    lines, torn = complete_lines(path)
    best = {}
    for number, raw in enumerate(lines, 1):
        try:
            line = json.loads(raw)
        except json.JSONDecodeError as e:
            fail(f"{path}, line {number}: not JSON ({e})")
        n = line["n"]
        if n >= first and (n not in best or line["score"] >= best[n].score):
            best[n] = Game(line, True)
    return best, len(lines), sum(len(raw) for raw in lines), torn


def read_parity(paths):
    """The prover's parity bound and sum of sources of each board, by n, from the files of its `paritysplit` example
    (the notebook's entry "The parity bound against the frame bound" says how they were made)."""
    found = {}
    for path in paths:
        for line in json_lines(path):
            n = line["n"]
            row = (line["parity"], line["ustar_sum"], line["diamond"])
            if found.setdefault(n, row) != row:
                fail(f"{path}: board {n} has two different lines in the parity files")
    return found


def read_ledger_rows(path, columns, what):
    """The rows of a ledger of tab-separated numbers and words, as lists of text, without a last row that is still
    being written; a row with another number of columns than `columns` stops the tool."""
    with open(path, "rb") as f:
        raw = f.read()
    pieces = raw.split(b"\n")
    pieces.pop()  # the text after the last newline: empty, or a row not finished
    rows = []
    for number, piece in enumerate(pieces, 1):
        row = piece.decode().split("\t")
        if len(row) not in columns:
            fail(f"{path}, line {number}: {len(row)} columns in a {what} ledger, which has {sorted(columns)}")
        rows.append(row)
    return rows


def split_option(value, what):
    """`PATH:NUMBER` as (PATH, NUMBER), or (PATH, None) if there is no number after the last colon."""
    path, colon, number = value.rpartition(":")
    if colon and number.isdigit():
        return path, int(number)
    if what == "required":
        fail(f"{value}: the path must be followed by :SECONDS")
    return value, None


def read_plays(specs):
    """The dedicated plays of every ledger in `specs`, as (n, level, seed, raised) rows in the order of the ledgers.
    A ledger row is the game, the level of the play (not in the first ledger of the pilot's round), the seed, the
    best-known score before the play, the score the play reached and the time of day, which is not kept.  A play
    raised the game if it reached a score above the best-known one."""
    plays = []
    for spec in specs:
        path, level = split_option(spec, "optional")
        for row in read_ledger_rows(path, {5, 6}, "plays"):
            if len(row) == 6:
                n, row_level, seed, known, score = (int(x) for x in row[:5])
            elif level is None:
                fail(f"{path}: rows with no level need the level after the path, as PATH:LEVEL")
            else:
                (n, seed, known, score), row_level = (int(x) for x in row[:4]), level
            plays.append((n, row_level, seed, int(score > known)))
    return plays


def read_finder(specs):
    """The passes of the finder in every ledger of `specs` (each PATH:SECONDS), as (n, B, SECONDS, outcome) rows.  A
    ledger row is the game, the bound B of the smooth part, the status, the best-known score before, the score
    reached, the CPU seconds and the time of day, of which the status is kept; a row of an error is left out."""
    passes = []
    for spec in specs:
        path, limit = split_option(spec, "required")
        for row in read_ledger_rows(path, {7}, "finder"):
            if row[2] == "error":
                continue
            if row[2] not in FINDER_OUTCOME:
                fail(f"{path}: the status {row[2]!r} of game {row[0]} is not one of {sorted(FINDER_OUTCOME)}")
            passes.append((int(row[0]), int(row[1]), limit, FINDER_OUTCOME[row[2]]))
    return passes


# ---------------------------------------------------------------------------------------------------------------
# Numbers
# ---------------------------------------------------------------------------------------------------------------

class Numbers:
    """The prime factors of 1..top: the smallest prime factor, how many prime factors (with multiplicity), the
    largest prime at most n and the prime before a prime."""

    def __init__(self, top):
        self.spf = list(range(top + 1))
        for p in range(2, int(top ** 0.5) + 1):
            if self.spf[p] == p:
                for m in range(p * p, top + 1, p):
                    if self.spf[m] == m:
                        self.spf[m] = p
        self.omega = [0] * (top + 1)
        for x in range(2, top + 1):
            self.omega[x] = self.omega[x // self.spf[x]] + 1
        self.prime_before = {}
        self.largest_prime = [0] * (top + 1)
        last = None
        for x in range(2, top + 1):
            if self.is_prime(x):
                self.prime_before[x] = last
                last = x
            self.largest_prime[x] = last
        self.layer2 = [x for x in range(4, top + 1) if self.omega[x] == 2]

    def is_prime(self, x):
        return x > 1 and self.spf[x] == x

    def maximal_factors(self, x):
        """{x / p : p a prime factor of x}; for a prime, {1}."""
        out, y = set(), x
        while y > 1:
            p = self.spf[y]
            out.add(x // p)
            while y % p == 0:
                y //= p
        return out

    def greedy_sources(self, n, layer):
        """The numbers of the given layer (number of prime factors) that descending greedy keeps among the sources
        of board n, which are 1/2 < x/n <= 1: a source is kept if its vector, the set of its maximal factors, is not
        the sum over GF(2) of the vectors kept before it (the span conjecture makes this the set U*)."""
        basis, kept = {}, []
        for x in range(n, n // 2, -1):
            if self.omega[x] != layer:
                continue
            v = 0
            for f in self.maximal_factors(x):
                v |= 1 << f
            while v:
                high = v.bit_length() - 1
                if high not in basis:
                    basis[high] = v
                    kept.append(x)
                    break
                v ^= basis[high]
        return kept

    def semiprime_x(self, n):
        """For an odd n with two prime factors: the smallest member other than n of the cycle that n closes with the
        sources of layer 2 of board n - 1 that descending greedy keeps (the forest that Kruskal's algorithm builds
        from the largest, in the graph with the primes and a ground vertex 0 as vertices and p*q as the edge p-q, p*p
        as the edge p-0), or None if n closes no cycle."""
        parent = {}

        def find(a):
            while parent.setdefault(a, a) != a:
                parent[a] = parent[parent[a]]
                a = parent[a]
            return a

        def ends(x):
            p = self.spf[x]
            q = x // p
            return p, (0 if q == p else q)

        adjacent = collections.defaultdict(list)
        low = (n - 1) // 2
        i = bisect.bisect_right(self.layer2, n - 1) - 1
        while i >= 0 and self.layer2[i] > low:
            x = self.layer2[i]
            u, v = ends(x)
            ru, rv = find(u), find(v)
            if ru != rv:
                parent[ru] = rv
                adjacent[u].append((v, x))
                adjacent[v].append((u, x))
            i -= 1
        u, v = ends(n)
        if find(u) != find(v):
            return None
        reached = {u: None}
        queue = collections.deque([u])
        while queue and v not in reached:
            a = queue.popleft()
            for b, x in adjacent[a]:
                if b not in reached:
                    reached[b] = (a, x)
                    queue.append(b)
        path, a = [], v
        while reached[a] is not None:
            a, x = reached[a]
            path.append(x)
        return min(path)


# ---------------------------------------------------------------------------------------------------------------
# The table
# ---------------------------------------------------------------------------------------------------------------

class Table:
    """The best-known games past the end of the chain with what is known of each: the games of `best` that are not
    proven, the proven games, the chain and the parity bounds."""

    def __init__(self, chain, beyond, best, parity, numbers):
        self.chain, self.beyond, self.best, self.parity, self.numbers = chain, beyond, best, parity, numbers
        self.first = max(chain) + 1
        self.last = max(best)
        self.proven = dict(chain)
        self.proven.update(beyond)
        self.games = [n for n in range(self.first, self.last + 1) if n not in beyond]
        for n in range(self.first, self.last + 1):
            if n not in best and n not in beyond:
                fail(f"the best-known file has no line of game {n}")
        missing = [n for n in range(self.first - 1, self.last + 1) if n not in parity]
        if missing:
            fail(f"the parity files have no line of the boards {missing[:5]}...")

    def known(self, n):
        """The game n as the rules take it: the proven game if there is one, else the best-known line."""
        return self.proven[n] if n in self.proven else self.best[n]

    def step(self, n):
        """How the parity bound moves from game n - 1 to game n, by the change of the sum of the sources U*: at an odd
        n, an exchange step if U* gains n and loses a source (every prime is one, and loses the prime before it) and
        an added step if it only gains n; at an even n, a half-out step if it only gains n and a half-in step if it
        gains n and loses n/2 (docs/theory/parity-bound.md, "The parity steps").  The steps are those of the greedy
        basis wherever the span conjecture holds, which the parity files record."""
        (_, sources, diamond), (_, before, _) = self.parity[n], self.parity[n - 1]
        if diamond:
            fail(f"the parity file records {diamond} violations of the span conjecture at board {n}")
        change = sources - before
        if self.numbers.is_prime(n):
            if change != n - self.numbers.prime_before[n]:
                fail(f"the sum of the sources changes by {change} at the prime {n}, not by n less the prime before it")
            return "exchange"
        if n % 2:
            if change == n:
                return "added"
            if 0 < change < n:
                return "exchange"
        elif change == n:
            return "half-out"
        elif change == n - n // 2:
            return "half-in"
        fail(f"the sum of the sources changes by {change} at game {n}: no kind of step")

    def twelve_p_holds(self, n, r):
        """The hypothesis of the 12p theorem at n = 12p, r the prime before p and 15r > n: 8p, 8r, 12r and 8t for a
        prime t with 4p/3 < t <= 3p/2 are sources that descending greedy keeps on layer 4 of board n."""
        p = n // 12
        kept = set(self.numbers.greedy_sources(n, 4))
        primes = [t for t in range(4 * p // 3, 3 * p // 2 + 1) if self.numbers.is_prime(t)]
        ts = [t for t in primes if 3 * t > 4 * p and 2 * t <= 3 * p]
        return {8 * p, 8 * r, 12 * r} <= kept and any(8 * t in kept for t in ts)

    def repeats_an_earlier_game(self, n):
        """Whether n poses the root problem of the game before it (of the one before that where the game before is
        prime) and its best-known game has that game's promotion sum."""
        game = self.known(n)
        mine = game.promotion_sum()
        if game.digest is None or mine is None:
            return False
        before = [n - 1] + ([n - 2] if self.numbers.is_prime(n - 1) else [])
        return any(self.known(m).digest == game.digest and self.known(m).promotion_sum() == mine for m in before)

    def rule(self, n):
        """The first of RULES that makes the best-known score of game n the score of an optimal game if the score of
        game n - 1 is that of an optimal game, or None.  A rule says what opt(n) is, or bounds it, from opt(n - 1),
        and the best-known score has it exactly.  Where n is a half-out step the half lemma gives the ceiling, which
        the rule `cap` already gives with no statement U, so the half lemma never names a game."""
        numbers = self.numbers
        below, mine = self.known(n - 1).score, self.known(n).score
        gain = mine - below  # what the best-known game of n scores over the one of n - 1
        if numbers.is_prime(n):
            return "prime_formula" if gain == n - numbers.prime_before[n] else None
        if gain == n:
            return "cap"
        if n % 8 == 0 and n >= 187 and numbers.is_prime(n // 8):
            r = numbers.prime_before[n // 8]
            if 9 * r > n and gain == n - 4 * r:
                return "theorem_8p"
        if n % 2 and numbers.omega[n] == 2 and n >= 189:
            x = numbers.semiprime_x(n)
            if x is None:
                fail(f"game {n} closes no cycle, which the semiprime formula needs")
            if gain == n - x:
                return "semiprime_formula"
        if n % 12 == 0 and n >= 187 and numbers.is_prime(n // 12):
            r = numbers.prime_before[n // 12]
            if 15 * r > n and gain == n - 6 * r and self.twelve_p_holds(n, r):
                return "theorem_12p"
        return "same_search" if self.repeats_an_earlier_game(n) else None

    def rows(self):
        """The lines of the table, in order of n."""
        numbers = self.numbers
        for n in self.games:
            game = self.best[n]
            bound = self.parity[n][0]
            moves = game.moves
            if sum(moves) != game.score or len(set(moves)) != len(moves) or max(moves) > n:
                fail(f"the best-known line of game {n} is not a set of moves of the board that adds up to its score")
            if moves[0] != numbers.largest_prime[n]:
                fail(f"the best-known game {n} does not open with the largest prime, {numbers.largest_prime[n]}")
            if game.score > bound:
                fail(f"the best-known score of game {n}, {game.score}, is above its parity bound, {bound}")
            if game.bound is not None and game.bound != bound:
                fail(f"the best-known line of game {n} has the bound {game.bound}, the parity file {bound}")
            yield {"n": n, "score": game.score, "parity_bound": bound, "step": self.step(n), "rule": self.rule(n),
                   "moves": game.moves}


def dump_row(row):
    """A row of the table as a line of JSON text, the moves last."""
    moves = row["moves"]
    head = {key: value for key, value in row.items() if key != "moves"}
    return json.dumps(head, separators=(",", ":"))[:-1] + ',"moves":[' + ",".join(map(str, moves)) + "]}\n"


def write_gzip_members(path, lines):
    """Writes `lines` (text, each ending in a newline) to a gzip file, MEMBER_LINES lines to a member, the same bytes
    for the same lines (no time in the header), by way of a temporary file that replaces the old one."""
    temporary = path.with_name(path.name + ".tmp")
    with open(temporary, "wb") as f:
        for i in range(0, len(lines), MEMBER_LINES):
            f.write(gzip.compress("".join(lines[i:i + MEMBER_LINES]).encode(), compresslevel=9, mtime=0))
    os.replace(temporary, path)


def write_text(path, lines):
    temporary = path.with_name(path.name + ".tmp")
    with open(temporary, "w") as f:
        f.writelines(lines)
    os.replace(temporary, path)


# ---------------------------------------------------------------------------------------------------------------
# The effort
# ---------------------------------------------------------------------------------------------------------------

def effort_rows(games, plays, passes):
    """One row for each game of `games` that has a play or a pass of the finder, in order of n: its plays as [level,
    seed, raised] sorted by level and seed, its passes of the finder as [B, SECONDS, outcome] sorted by B.  Returns
    the rows and how many plays and passes were left out, for games that the table does not have."""
    by_game = collections.defaultdict(lambda: ([], []))
    left_out = [0, 0]
    wanted = set(games)
    for n, level, seed, raised in plays:
        if n in wanted:
            by_game[n][0].append([level, seed, raised])
        else:
            left_out[0] += 1
    for n, bound, limit, outcome in passes:
        if n in wanted:
            by_game[n][1].append([bound, limit, outcome])
        else:
            left_out[1] += 1
    rows = [{"n": n, "plays": sorted(by_game[n][0], key=lambda p: p[:2]),
             "finder": sorted(by_game[n][1], key=lambda p: p[:2])} for n in sorted(by_game)]
    return rows, left_out


def dump_effort(row):
    return json.dumps(row, separators=(",", ":")) + "\n"


# ---------------------------------------------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0], epilog="See the top of this file.")
    parser.add_argument("--best-known", required=True, type=Path, help="the best-known file, read as it is now")
    parser.add_argument("--parity", required=True, nargs="+", type=Path, help="files of the parity bound, by board")
    parser.add_argument("--plays", action="append", default=[], metavar="FILE[:LEVEL]", help="a ledger of plays")
    parser.add_argument("--finder", action="append", default=[], metavar="FILE:SECONDS", help="a ledger of the finder")
    parser.add_argument("--chain", type=Path, default=REPO / "results" / "optimal.jsonl.gz")
    parser.add_argument("--beyond", type=Path, default=REPO / "results" / "beyond-the-chain.jsonl")
    parser.add_argument("--out-dir", type=Path, default=REPO / "results")
    args = parser.parse_args()

    chain = read_proven(args.chain)
    beyond = read_proven(args.beyond, keep_moves=True)
    best, line_count, byte_count, torn = read_best_known(args.best_known, max(chain) + 1)
    print(f"best-known: {byte_count:,} bytes, {line_count:,} lines of {len(best):,} games, "
          f"{'a torn last line left out' if torn else 'no torn last line'}", file=sys.stderr)
    table = Table(chain, beyond, best, read_parity(args.parity), Numbers(max(best)))

    rows = list(table.rows())
    effort, left_out = effort_rows(table.games, read_plays(args.plays), read_finder(args.finder))
    args.out_dir.mkdir(parents=True, exist_ok=True)
    write_gzip_members(args.out_dir / "best-known.jsonl.gz", [dump_row(row) for row in rows])
    write_text(args.out_dir / "search-effort.jsonl", [dump_effort(row) for row in effort])

    steps = collections.Counter(row["step"] for row in rows)
    rules = collections.Counter(row["rule"] for row in rows)
    print(f"best-known.jsonl.gz: {len(rows)} games from {rows[0]['n']} to {rows[-1]['n']}, "
          f"not {sorted(beyond)}")
    print("  steps: " + ", ".join(f"{k} {v}" for k, v in sorted(steps.items())))
    print("  rules: " + ", ".join(f"{k or 'none'} {rules[k]}" for k in itertools.chain(RULES, [None])))
    print(f"search-effort.jsonl: {len(effort)} games, {sum(len(r['plays']) for r in effort)} plays, "
          f"{sum(len(r['finder']) for r in effort)} passes of the finder; left out for games that are not in the "
          f"table: {left_out[0]} plays, {left_out[1]} passes")


if __name__ == "__main__":
    main()
