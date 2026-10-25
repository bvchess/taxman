#!/usr/bin/env python3
"""Which games of the chain needed no search, by the cofactor of n.

Standard library only.  Run from the top of the repository, with the results files in order, the earlier file first:

    solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/cofactor.py \
        results/strict.jsonl ~/taxman_work/chain-data/results/strict.jsonl --last 4979

Each file supplies the games past the last game of the files before it, each game by its first completed line.  The
lines of method `known` (copies of the bounds store) are left out, since no search was in question.  A game is
*searched* when its method is `search`, and *free* when it is `prime`, `repeat`, `start` or `trivial`.  Writing
n = c * P with P the largest prime factor of n, c is the *cofactor*.  The output has these parts: the games by method,
the cofactors by class, the families of the cofactors that are never searched, the rule "c is 1, a prime, twice a
prime or 9" by range, the cofactors added to the rule from 1301..3500 and tested on 3501.., the free games the rule
does not name, and the setup fields of the games against the problem before them.
"""
import argparse
import collections
import json
import textwrap
from pathlib import Path

FREE = ("prime", "repeat", "start", "trivial")
RANGES = ((1301, 2175), (2176, 3500), (3501, None))
FIT = (1301, 3500)
CLASS_FROM, MIN_GAMES, MIN_SHARE = 2176, 5, 0.9


def load(paths, last):
    """The completed game of each n up to last, from the files in order: {n: line}, the lines of method `known` too."""
    games, top = {}, 0
    for path in paths:
        first = {}
        for text in Path(path).expanduser().read_text().splitlines():
            row = json.loads(text)
            if row["completed"] and row["n"] not in first:
                first[row["n"]] = row
        games.update({n: row for n, row in first.items() if top < n <= last})
        top = max(top, max(first))
    return games


def largest_prime_factors(limit):
    """The largest prime factor of every n up to limit, by a sieve."""
    largest = list(range(limit + 1))
    for p in range(2, limit + 1):
        if largest[p] == p:
            for multiple in range(p, limit + 1, p):
                largest[multiple] = p
    return largest


def show_list(label, items):
    """Print label and the items, wrapped to the width of the page."""
    print(textwrap.fill(f"{label}: " + " ".join(map(str, items)), width=118, subsequent_indent="    "))


def percent(part, whole):
    return f"{100 * part / whole:.1f}%" if whole else "-"


class Study:
    """The games of the chain with their cofactors, and the rule's calls."""

    def __init__(self, games, last):
        self.games, self.last = games, last
        self.largest = largest_prime_factors(last)
        self.cofactor = {n: n // self.largest[n] for n in games}
        self.counted = {n: row for n, row in games.items() if row["method"] != "known"}

    def is_prime(self, k):
        return k > 1 and self.largest[k] == k

    def named(self, c, added=frozenset()):
        """Whether the rule calls a game of cofactor c free: c is 1, a prime, twice a prime or 9 (or one added)."""
        return c == 1 or self.is_prime(c) or (c % 2 == 0 and self.is_prime(c // 2)) or c == 9 or c in added

    def span(self, lo, hi):
        """The counted games of lo..hi (hi None: to the last game)."""
        hi = self.last if hi is None else min(hi, self.last)
        return [n for n in sorted(self.counted) if lo <= n <= hi]

    def by_cofactor(self, ns):
        """Per cofactor, the counts of the games of ns by method."""
        counts = collections.defaultdict(collections.Counter)
        for n in ns:
            counts[self.cofactor[n]][self.counted[n]["method"]] += 1
        return counts

    def is_searched(self, n):
        return self.counted[n]["method"] == "search"


def print_methods(study):
    print("== The games by method (first completed line of each n; lines of method known left out)")
    counts = collections.Counter(row["method"] for row in study.games.values())
    print(f"games 1..{study.last}: {len(study.games)}; " + ", ".join(f"{m} {k}" for m, k in sorted(counts.items())))
    for lo, hi in RANGES:
        ns = study.span(lo, hi)
        searched = sum(study.is_searched(n) for n in ns)
        print(f"{lo}..{hi or study.last}: {len(ns)} games counted, {searched} searched, {len(ns) - searched} free")
    print()


def classify(counts):
    """never / always / mixed for the cofactors of counts that have at least MIN_GAMES games."""
    classes = {"never": [], "always": [], "mixed": []}
    for c, by_method in sorted(counts.items()):
        games = sum(by_method.values())
        if games >= MIN_GAMES:
            kind = "never" if by_method["search"] == 0 else "always" if by_method["search"] == games else "mixed"
            classes[kind].append(c)
    return classes


def print_classes(study):
    ns = study.span(CLASS_FROM, None)
    counts = study.by_cofactor(ns)
    classes = classify(counts)
    print(f"== The cofactors with at least {MIN_GAMES} games in {CLASS_FROM}..{study.last}, by class")
    print(f"{'class':<8}{'cofactors':>10}{'games':>7}{'searched':>10}{'repeat':>8}{'start':>7}{'prime':>7}")
    classes["fewer"] = [c for c, m in sorted(counts.items()) if sum(m.values()) < MIN_GAMES]
    for kind, cs in classes.items():
        tally = collections.Counter()
        for c in cs:
            tally.update(counts[c])
        print(f"{kind:<8}{len(cs):>10}{sum(tally.values()):>7}{tally['search']:>10}{tally['repeat']:>8}"
              f"{tally['start']:>7}{tally['prime']:>7}")
    print(f"(fewer: the cofactors with fewer than {MIN_GAMES} games, left out of the lists below)")
    for kind in ("never", "always", "mixed"):
        show_list(f"{kind} cofactors", classes[kind])
    print()
    print("cofactor: games, searched, repeat, start  (cofactors of the mixed class, and the examples)")
    for c in sorted(set(classes["mixed"]) | {8, 12, 16, 20, 21, 24, 30, 32, 35, 39, 51, 57, 9, 15}):
        m = counts[c]
        print(f"  c = {c}: {sum(m.values())}, {m['search']}, {m['repeat']}, {m['start']}")
    print()
    return classes, counts


def family(study, c):
    """The family of a cofactor, for the table of the cofactors that are never searched."""
    if c == 1:
        return "c = 1 (n is prime)"
    if study.is_prime(c):
        return "c prime"
    if c % 2 == 0 and study.is_prime(c // 2):
        return "c twice a prime"
    if c == 9:
        return "c = 9"
    return "other"


def print_families(study, classes, counts):
    print(f"== The cofactors never searched in {CLASS_FROM}..{study.last}, by family")
    print(f"{'family':<20}{'cofactors':>10}{'games':>7}{'searched':>10}{'repeat':>8}{'start':>7}{'prime':>7}")
    groups = collections.defaultdict(list)
    never_all = [c for c, m in counts.items() if m["search"] == 0]
    for c in sorted(never_all):
        groups[family(study, c)].append(c)
    for name in ("c = 1 (n is prime)", "c prime", "c twice a prime", "c = 9", "other"):
        tally = collections.Counter()
        for c in groups[name]:
            tally.update(counts[c])
        print(f"{name:<20}{len(groups[name]):>10}{sum(tally.values()):>7}{tally['search']:>10}{tally['repeat']:>8}"
              f"{tally['start']:>7}{tally['prime']:>7}")
    show_list("never searched, with at least 5 games, in no family",
              [c for c in groups["other"] if sum(counts[c].values()) >= MIN_GAMES])
    print("never-searched cofactors with 1 to 4 games hold "
          f"{sum(sum(counts[c].values()) for c in never_all if sum(counts[c].values()) < MIN_GAMES)} games")
    always3 = [c for c in classes["always"] if c % 3 == 0 and study.is_prime(c // 3)]
    show_list("always searched, three times a prime", always3)
    print("c three times a prime p (games, searched):")
    for p in (2, 3, 5, 7, 11, 13):
        print(f"  p = {p}, c = {3 * p}: {sum(counts[3 * p].values())}, {counts[3 * p]['search']}")
    print()


def score(study, ns, added=frozenset()):
    """For the games ns: (free games, free games named, searched games named, named in all)."""
    free = [n for n in ns if not study.is_searched(n)]
    named_free = [n for n in free if study.named(study.cofactor[n], added)]
    wrong = [n for n in ns if study.is_searched(n) and study.named(study.cofactor[n], added)]
    return free, named_free, wrong


def print_rule(study):
    print("== The rule: no search when c is 1, a prime, twice a prime or 9")
    print(f"{'games':<12}{'counted':>8}{'free':>6}{'named':>7}{'share':>8}{'wrong calls':>13}")
    for lo, hi in ((RANGES[0][0], None),) + RANGES:
        ns = study.span(lo, hi)
        free, named_free, wrong = score(study, ns)
        print(f"{lo}..{hi or study.last:<5}{len(ns):>8}{len(free):>6}{len(named_free):>7}"
              f"{percent(len(named_free), len(free)):>8}{len(wrong):>13}")
    unnamed = [n for n in study.span(RANGES[0][0], None) if not study.named(study.cofactor[n])]
    searched = sum(study.is_searched(n) for n in unnamed)
    print(f"games the rule does not name: {len(unnamed)}, of which {searched} searched "
          f"({percent(searched, len(unnamed))})")
    print()


def added_cofactors(study):
    """The cofactors the rule does not name that are free in MIN_SHARE of at least MIN_GAMES games of FIT."""
    counts = study.by_cofactor(study.span(*FIT))
    found = []
    for c, m in sorted(counts.items()):
        games = sum(m.values())
        if games >= MIN_GAMES and not study.named(c) and (games - m["search"]) >= MIN_SHARE * games:
            found.append(c)
    return found, counts


def print_held_out(study):
    added, counts = added_cofactors(study)
    extra = frozenset(added)
    print(f"== Cofactors added from {FIT[0]}..{FIT[1]}: free in at least {percent(MIN_SHARE, 1)} of at least "
          f"{MIN_GAMES} games")
    for c in added:
        m = counts[c]
        print(f"  c = {c}: {sum(m.values())} games, {m['search']} searched, {m['repeat']} repeat, {m['start']} start")
    print(f"{'games':<12}{'free':>6}{'named':>7}{'share':>8}{'wrong calls':>13}   (with {sorted(extra)} added)")
    for lo, hi in RANGES:
        ns = study.span(lo, hi)
        free, named_free, wrong = score(study, ns, extra)
        note = "fitted" if hi is not None else "held out"
        print(f"{lo}..{hi or study.last:<5}{len(free):>6}{len(named_free):>7}"
              f"{percent(len(named_free), len(free)):>8}{len(wrong):>13}   {note}")
        for n in wrong:
            print(f"    wrong call: n = {n} = {study.cofactor[n]} * {study.largest[n]}, "
                  f"method {study.counted[n]['method']}")
    print()


def print_rest(study):
    ns = study.span(RANGES[0][0], None)
    free, named_free, _ = score(study, ns)
    rest = [n for n in free if n not in set(named_free)]
    print(f"== The free games of {RANGES[0][0]}..{study.last} that the rule does not name: {len(rest)}")
    print("by method: " + ", ".join(f"{m} {k}" for m, k in sorted(
        collections.Counter(study.counted[n]["method"] for n in rest).items())))
    counts = study.by_cofactor(rest)
    print("by cofactor: " + ", ".join(f"{c} x{sum(m.values())}" for c, m in sorted(
        counts.items(), key=lambda item: -sum(item[1].values()))[:12]))
    print()


def print_setups(study):
    """Compare each game's setup fields with those of the problem before it (the latest game that has a setup)."""
    ns = study.span(CLASS_FROM, None)
    counts = study.by_cofactor(ns)
    never = {c for c, m in counts.items() if m["search"] == 0}
    always = {c for c, m in counts.items() if m["search"] == sum(m.values())}
    setups = {n: row["setup"] for n, row in study.games.items() if row["setup"]}
    tally = collections.defaultdict(collections.Counter)
    for n in ns:
        row = study.counted[n]
        if not row["setup"]:
            continue
        before = n - 1
        while before not in setups:
            before -= 1
        now, was = row["setup"], setups[before]
        c = study.cofactor[n]
        kind = "never searched" if c in never else "always searched" if c in always else "mixed"
        key = (kind, row["method"])
        tally[key]["games"] += 1
        for name in ("digest", "candidates", "max_total", "greedy_sum"):
            tally[key][name] += now[name] == was[name]
    print(f"== The setup of each game of {CLASS_FROM}..{study.last} against the problem before it")
    print(f"{'cofactor class':<17}{'method':<8}{'games':>6}{'same digest':>13}{'same candidates':>17}"
          f"{'same max_total':>16}{'same greedy_sum':>17}")
    for (kind, method), t in sorted(tally.items()):
        print(f"{kind:<17}{method:<8}{t['games']:>6}{t['digest']:>13}{t['candidates']:>17}{t['max_total']:>16}"
              f"{t['greedy_sum']:>17}")
    repeats = [n for n in ns if study.counted[n]["method"] == "repeat"]
    gaps = collections.Counter(n - study.counted[n]["same_search"]["n"] for n in repeats)
    disagree = sum(
        study.counted[n]["same_search"]["digest"] != study.counted[n]["setup"]["digest"]
        or study.games[study.counted[n]["same_search"]["n"]]["setup"]["digest"]
        != study.counted[n]["same_search"]["digest"]
        for n in repeats if study.counted[n]["same_search"]["n"] in setups)
    print(f"repeats: {len(repeats)}, repeating game n - 1 in {gaps[1]} and n - 2 in {gaps[2]}; "
          f"digest of the line differs from the repeated game's in {disagree}")
    print()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("files", nargs="+", help="results files, the earlier one first")
    parser.add_argument("--last", type=int, default=4979, help="the last game to count")
    args = parser.parse_args()
    games = load(args.files, args.last)
    study = Study(games, max(games))
    print_methods(study)
    classes, counts = print_classes(study)
    print_families(study, classes, counts)
    print_rule(study)
    print_held_out(study)
    print_rest(study)
    print_setups(study)


if __name__ == "__main__":
    main()
