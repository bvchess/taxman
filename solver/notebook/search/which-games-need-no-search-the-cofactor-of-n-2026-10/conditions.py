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
"""
import argparse
import collections
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
DEFAULT_CHAIN = REPO / "results" / "strict.jsonl"
FAMILIES = ("1", "a prime", "twice a prime", "9")


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


def obeys(games, n, fam):
    """Whether game n keeps the condition of its family; None when the family has no condition."""
    if fam == "1":
        return games[n]["method"] == "prime"
    if fam in ("a prime", "twice a prime"):
        own = digest(games[n])
        return own is not None and own == digest(games[partner(games, n)])
    if fam == "9":
        return games[n]["score"] == games[n - 1]["score"] + n
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


if __name__ == "__main__":
    main()
