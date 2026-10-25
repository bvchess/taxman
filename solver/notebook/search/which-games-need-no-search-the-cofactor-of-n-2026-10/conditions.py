#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Whether the games of the chain that need no search keep the conditions of their cofactor's family.

Standard library only.  Run from the top of the repository:

    .venv/bin/python -I solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/conditions.py

The chain is results/strict.jsonl unless a path is given, and --from sets the first n (default 3, at least 2).  Each n
keeps its first completed line.  Writing n = c * P with P the largest prime factor of n, c is the cofactor, and n falls
in a family: "1" (c is 1), "a prime" (c is prime), "twice a prime" (c is 2p with p prime, 4 included), "9", or "other".
A game of method known is unrecorded.  The conditions checked: a game of family 1 is a prime game; a game of family
"a prime" or "twice a prime" has the setup digest of game m, where m is n - 1, or n - 2 when n - 1 is a prime game; a
game of family 9 has score(n) = score(n - 1) + n.  The output has the counts of the four families, every failure, the
smallest n from which every recorded game of the four families obeys, and from that n to the last game the games settled
without a search, with the share of them that the four families hold.

The last part asks whether layer 2 being closed explains where the conditions hold.  The sources of layer 2 of board n
are the numbers s with n / 2 < s <= n and two prime factors: each source p * q joins the primes p and q, and p * p holds
a square.  A frame of layer 2 is a group of joined primes that holds a source, open when none of its primes holds a
square, and board n is open when some frame of its layer 2 is.  The script checks that the open boards from 2 to 8191
are the 80 of the notebook entry the-closed-frames-of-layer-2-2026-10, and prints the frames of boards 186 and 187.
Then for each game n >= 28 of the families "a prime", "twice a prime" and "9", m is as above (n - 1 for family 9),
and the pair is closed when boards n and m are both closed.  The output counts the games that obey and break in each
of the four cases, prints the games that break while closed (none is expected), and prints those that break while open.
"""
import argparse
import bisect
import collections
import json
from operator import itemgetter
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
DEFAULT_CHAIN = REPO / "results" / "strict.jsonl"
FAMILIES = ("1", "a prime", "twice a prime", "9")
RULE_FAMILIES = ("a prime", "twice a prime", "9")
RULE_FIRST = 28  # the boards up to 27 were settled by the trivial case and record no root problem
OPEN_LAST = 8191  # the open boards are checked from 2 to this board
NOTEBOOK_OPEN = "8, 12-14, 18-24, 28-34, 42-54, 70-76, 78-84, 98-120, 138-144, 182-186"
Case = collections.namedtuple("Case", "n fam m method obeys closed")


def prime_factors(n):
    """The prime factors of n with multiplicity, in ascending order."""
    factors, p = [], 2
    while p * p <= n:
        while n % p == 0:
            factors.append(p)
            n //= p
        p += 1
    if n > 1:
        factors.append(n)
    return factors


def is_prime(k):
    return k > 1 and prime_factors(k) == [k]


def factorization(n):
    return "*".join(map(str, prime_factors(n)))


def family(n):
    """The family of n, by its cofactor: n divided by its largest prime factor."""
    c = n // prime_factors(n)[-1]
    if c == 1:
        return "1"
    if is_prime(c):
        return "a prime"
    if c % 2 == 0 and is_prime(c // 2):
        return "twice a prime"
    if c == 9:
        return "9"
    return "other"


def load(path):
    """The first completed line of each n, by n."""
    games = {}
    for line in Path(path).read_text().splitlines():
        row = json.loads(line)
        if row["completed"] and row["n"] not in games:
            games[row["n"]] = row
    return games


def digest(row):
    setup = row["setup"]
    return setup.get("digest") if setup else None


def partner(games, n):
    """The game whose setup digest n must share: n - 1, or n - 2 when n - 1 is a prime game."""
    return n - 2 if games[n - 1]["method"] == "prime" else n - 1


def compared_with(games, n, fam):
    """The game m that game n is checked against: n - 1 for family 9, else partner(games, n)."""
    return n - 1 if fam == "9" else partner(games, n)


def obeys(games, n, fam):
    """Whether game n keeps the condition of its family; None when the family has no condition."""
    if fam == "1":
        return games[n]["method"] == "prime"
    if fam in ("a prime", "twice a prime"):
        own = digest(games[n])
        return own is not None and own == digest(games[compared_with(games, n, fam)])
    if fam == "9":
        return games[n]["score"] == games[compared_with(games, n, fam)]["score"] + n
    return None


def check(games, n, fam):
    """'unrecorded' for a game of method known, else 'obeys' or 'fails', or None when the family is not checked."""
    if games[n]["method"] == "known":
        return "unrecorded"
    ok = obeys(games, n, fam)
    if ok is None:
        return None
    return "obeys" if ok else "fails"


def percent(part, whole):
    return f"{100 * part / whole:.1f}%" if whole else "-"


def two_factor_numbers(last):
    """Each s from 4 to last with two prime factors, with multiplicity, as (s, p, q) with p <= q, by s."""
    rows = []
    for s in range(4, last + 1):
        factors = prime_factors(s)
        if len(factors) == 2:
            rows.append((s, factors[0], factors[1]))
    return rows


def layer2_frames(n, sources):
    """The frames of layer 2 of board n, each as (its sorted primes, its squares), in order of their primes.

    sources is two_factor_numbers(...); the sources of board n are its rows with n / 2 < s <= n.
    """
    lo = bisect.bisect_right(sources, n // 2, key=itemgetter(0))
    hi = bisect.bisect_right(sources, n, key=itemgetter(0))
    parent = {}

    def root(p):
        while parent[p] != p:
            parent[p] = parent[parent[p]]
            p = parent[p]
        return p

    squares = set()
    for _, p, q in sources[lo:hi]:
        parent.setdefault(p, p)
        parent.setdefault(q, q)
        if p == q:
            squares.add(p)
        else:
            parent[root(p)] = root(q)
    groups = collections.defaultdict(list)
    for p in parent:
        groups[root(p)].append(p)
    return sorted(
        (tuple(sorted(ps)), tuple(p * p for p in sorted(ps) if p in squares)) for ps in groups.values()
    )


def as_ranges(ns):
    """The sorted numbers as runs, comma-separated: 8, 12-14, 18."""
    runs = []
    for n in sorted(ns):
        if runs and n == runs[-1][1] + 1:
            runs[-1][1] = n
        else:
            runs.append([n, n])
    return ", ".join(str(a) if a == b else f"{a}-{b}" for a, b in runs)


def case_line(c):
    return f"{c.n:>6}  {factorization(c.n):<12}  {c.fam:<16}  {c.method:<8}  m = {c.m}"


def layer2_part(games, last):
    """Whether layer 2 being closed explains where the conditions hold."""
    boards = max(OPEN_LAST, last)
    sources = two_factor_numbers(boards)
    is_open = {n: any(not squares for _, squares in layer2_frames(n, sources)) for n in range(2, boards + 1)}

    open_list = [n for n in range(2, OPEN_LAST + 1) if is_open[n]]
    ranges = as_ranges(open_list)
    matches = ranges == NOTEBOOK_OPEN
    print(f"== The open boards of layer 2, 2..{OPEN_LAST}")
    print(f"open: {ranges} ({len(open_list)} boards)")
    print(f"matches the notebook entry: {'yes' if matches else 'no'}")
    if not matches:
        raise SystemExit("the open boards differ from the notebook entry")
    print()

    print("== The frames of layer 2 of boards 186 and 187: the sorted primes, then the squares")
    for n in (186, 187):
        print(f"board {n}")
        for primes, squares in layer2_frames(n, sources):
            print(f"  frame {', '.join(map(str, primes))}: squares {', '.join(map(str, squares)) or 'none'}")
    print()

    cases = []
    for n in range(RULE_FIRST, last + 1):
        fam = family(n)
        if fam not in RULE_FAMILIES:
            continue
        result = check(games, n, fam)
        if result == "unrecorded":
            continue
        m = compared_with(games, n, fam)
        closed = not is_open[n] and not is_open[m]
        cases.append(Case(n, fam, m, games[n]["method"], result == "obeys", closed))

    print(f"== The rule against closed boards, from n = {RULE_FIRST}: closed when boards n and m are both closed")
    print(f"{'family':<16}{'closed obeys':>14}{'closed breaks':>15}{'open obeys':>12}{'open breaks':>13}")
    groups = [(fam, [c for c in cases if c.fam == fam]) for fam in RULE_FAMILIES]
    groups.append(("total", cases))
    for name, group in groups:
        tally = collections.Counter((c.closed, c.obeys) for c in group)
        closed_obeys, closed_breaks = tally[True, True], tally[True, False]
        open_obeys, open_breaks = tally[False, True], tally[False, False]
        print(f"{name:<16}{closed_obeys:>14}{closed_breaks:>15}{open_obeys:>12}{open_breaks:>13}")
    print()

    breaks = [c for c in cases if not c.obeys]
    for title, group in (("closed", [c for c in breaks if c.closed]), ("open", [c for c in breaks if not c.closed])):
        print(f"== The games that break while their boards are {title}: n, factorization, family, method, m")
        for c in group:
            print(case_line(c))
        if not group:
            print("none")
        print()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("chain", nargs="?", type=Path, default=DEFAULT_CHAIN,
                        help="the chain file (default: %(default)s)")
    parser.add_argument("--from", dest="start", type=int, default=3, help="the first n to count (default 3)")
    args = parser.parse_args()
    if args.start < 2:
        parser.error("--from must be at least 2: 1 has no prime factor")

    games = load(args.chain)
    last = max(games)
    numbers = range(args.start, last + 1)
    fams = {n: family(n) for n in numbers}
    status = {n: check(games, n, fams[n]) for n in numbers}

    print(f"chain {args.chain}: games 1..{last}, counted from n = {args.start}")
    print("== The families: games, obey, fail, unrecorded")
    print(f"{'family':<16}{'games':>7}{'obey':>7}{'fail':>7}{'unrecorded':>12}")
    for fam in FAMILIES:
        ns = [n for n in numbers if fams[n] == fam]
        tally = collections.Counter(status[n] for n in ns)
        print(f"{fam:<16}{len(ns):>7}{tally['obeys']:>7}{tally['fails']:>7}{tally['unrecorded']:>12}")
    print()

    fails = [n for n in numbers if status[n] == "fails"]
    print("== The failures: n, factorization, family, method")
    for n in fails:
        print(f"{n:>6}  {factorization(n):<12}  {fams[n]:<16}  {games[n]['method']}")
    if not fails:
        print("none")
    print()

    threshold = max(fails) + 1 if fails else args.start
    print(f"smallest n from which every recorded game of the four families obeys: {threshold}")
    print()

    tail = range(threshold, last + 1)
    settled = [n for n in tail if games[n]["method"] not in ("known", "search")]
    settled_four = [n for n in settled if fams[n] in FAMILIES]
    searched_four = [n for n in tail if fams[n] in FAMILIES and games[n]["method"] == "search"]
    print(f"== From n = {threshold} to {last}, the games not unrecorded")
    print(f"settled without a search: {len(settled)}; of them in the four families: {len(settled_four)} "
          f"({percent(len(settled_four), len(settled))})")
    print(f"games of the four families with method search: {len(searched_four)}")
    print()
    layer2_part(games, last)


if __name__ == "__main__":
    main()
