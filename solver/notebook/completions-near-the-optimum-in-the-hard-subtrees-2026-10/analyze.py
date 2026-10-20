#!/usr/bin/env python3
"""The tables of the entry "Completions near the optimum in the hard subtrees": the counts of `wall near`, the
completions near the best described against it, and the arithmetic of the problems.

usage: python3 -I analyze.py RUNS.jsonl SETS_DIR CANDIDATES_DIR SUITE.jsonl

RUNS.jsonl holds the lines of `wall near` and of `wall nodes --mode opt`, each with a field `run` added: `count`,
`estimate` or `proof`; SETS_DIR the files ID.sets.tsv of the counts made with --sets; CANDIDATES_DIR the files
N.candidates.tsv they wrote (each candidate's home and maximal factors); SUITE.jsonl the tuning suite (the units'
candidates, gaps and one-thread proofs).
"""
import json
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path

# The anatomy's measurements of the traced units (solver/notebook/the-anatomy-of-a-hard-subtree-2026-10/README.md):
# leaves, and the gap at which a leaf closes (median).
ANATOMY = {"3300-14": (104_400, 94), "3441-1262": (88_755, 142), "3441-5459": (4_706, 169)}
# The chain's own searches, for its nodes on 64 threads (solver/notebook/the-gap-past-2175-2026-10/), and the anatomy's
# lines of its traced units, for their cores.
NOTEBOOK = Path(__file__).resolve().parent.parent
CHAIN = NOTEBOOK / "the-gap-past-2175-2026-10" / "gaps_chain.jsonl"
ANATOMY_RUNS = NOTEBOOK / "the-anatomy-of-a-hard-subtree-2026-10" / "runs.jsonl"
CHEAP = 50  # a move priced under this is cheap (the anatomy's line)
DISTANCES = (0, 10, 30, 50, 100, 200)


def is_prime(k):
    return k > 1 and all(k % d for d in range(2, int(k ** 0.5) + 1))


def twins(x, y):
    """Whether x and y share a maximal factor: x = m·p and y = m·q with p and q prime (the anatomy's definition)."""
    g = math.gcd(x, y)
    return x != y and is_prime(x // g) and is_prime(y // g)


def primes_of(x):
    out, p = [], 2
    while p * p <= x:
        if x % p == 0:
            out.append(p)
            while x % p == 0:
                x //= p
        p += 1
    return out + ([x] if x > 1 else [])


def factor(x):
    """x as a product of primes, e.g. 1292 = 2^2·17·19."""
    parts, p = [], 2
    while p * p <= x:
        k = 0
        while x % p == 0:
            x, k = x // p, k + 1
        if k:
            parts.append(f"{p}^{k}" if k > 1 else str(p))
        p += 1
    if x > 1:
        parts.append(str(x))
    return "·".join(parts)


def from_hex(text):
    """The members of a set in the suite's hex form (character k, bit b: the number 4k + b)."""
    return {4 * k + b for k, c in enumerate(text) for b in range(4) if int(c, 16) >> b & 1}


def read_candidates(path):
    """For each candidate of a game: its home frame."""
    home = {}
    with open(path) as f:
        next(f)
        for line in f:
            x, h, _ = line.rstrip("\n").split("\t")
            home[int(x)] = int(h)
    return home


def read_sets(path):
    """The reference set and every completion of a sets' file, as (value, set of members beyond the unit's
    promotions); the lines of a first build had a weight column too."""
    with open(path) as f:
        ref = {int(x) for x in next(f).rstrip("\n").split("\t")[1].split(",") if x != "-"}
        out = []
        for line in f:
            fields = line.rstrip("\n").split("\t")
            members = set(ref)
            members |= {int(x) for x in fields[-2].split(",") if x != "-"}
            members -= {int(x) for x in fields[-1].split(",") if x != "-"}
            out.append((int(fields[0]), frozenset(members)))
    return ref, out


def pairs_off(removed, added, home):
    """The twin pairs (x removed, y added, the same home) a swap pairs off, as many as possible (augmenting paths; the
    swaps are small): their number and the matching, added member to removed member."""
    match = {}

    def augment(x, seen):
        for y in added:
            if y in seen or not twins(x, y) or home.get(x) != home.get(y):
                continue
            seen.add(y)
            if y not in match or augment(match[y], seen):
                match[y] = x
                return True
        return False

    return sum(1 for x in removed if augment(x, set())), match


def pct(a, b):
    return f"{100 * a / b:.0f}%" if b else "-"


def table(rows, header):
    widths = [max(len(str(r[i])) for r in [header] + rows) for i in range(len(header))]
    line = lambda r: "| " + " | ".join(str(v).ljust(w) for v, w in zip(r, widths)) + " |"
    print(line(header))
    print("|" + "|".join("-" * (w + 2) for w in widths) + "|")
    for r in rows:
        print(line(r))
    print()


# ---------------------------------------------------------------------------------------------------------------
# 1. The counts


def counts(lines, units, chain):
    """For each problem, the completions within d of the optimum from its widest count, and what its proof costs."""
    print("## 1. The counts\n")
    proofs = {str(d["n"]): d for d in lines if d["run"] == "proof"}
    widest = {}
    for d in lines:
        if d["run"] == "count" and (d["id"] not in widest or d["within"] > widest[d["id"]]["within"]):
            widest[d["id"]] = d
    rows = []
    for uid, d in widest.items():
        if uid in units:
            gap, proof = units[uid]["gap"], units[uid]["default"]["nodes"]
        else:
            gap, proof = d["bound"] - d["opt"], proofs[uid]["nodes"]
        at_least = "" if d["completed"] else "≥ "
        cum = lambda dmax: sum(c for dist, c in d["by_distance"] if dist <= dmax)
        chained = uid.isdigit() and int(uid) in chain
        rows.append([uid, gap, f"{proof:,}", f"{chain[int(uid)]:,}" if chained else "-", d["within"],
                     f"{d['steps']:,}" + ("" if d["completed"] else " (stopped)"), f"{d['cpu_s']:.0f}",
                     "none" if d["best"] is None else f"opt − {d['opt'] - d['best']}"] +
                    [f"{at_least}{cum(x):,}" if x <= d["within"] else "-" for x in DISTANCES])
    table(rows, ["problem", "gap", "proof's steps", "chain's nodes", "counted within", "count's steps", "CPU s",
                 "best counted"] + [f"≤ {x}" for x in DISTANCES])
    estimates = [d for d in lines if d["run"] == "estimate"]
    if estimates:
        print("The estimates (random descents to a frontier, exact counts there):\n")
        rows = [[d["id"], d["within"], d["probes"], d["frontier"], f"{d['steps']:,}", f"{d['frontier_steps']:,}",
                 d["hits"], f"{d['cpu_s']:.1f}"] for d in estimates]
        table(rows, ["unit", "within", "descents", "frontier", "descents' steps", "frontier counts' steps",
                     "descents that met a completion", "CPU s"])


# ---------------------------------------------------------------------------------------------------------------
# 2. The completions near the best, against it


def cloud(uid, opt, sets, home, deltas):
    """How the completions near the best differ from it: the swap's size, whether it pairs off as twins of the same
    home, the homes, the primes exchanged, the paired exchanges and the members of the best that move."""
    b = max(v for v, _ in sets)
    tops = [s for v, s in sets if v == b]
    best = tops[0]
    print(f"### {uid}: {len(sets):,} completions counted; the best is opt − {opt - b}, {len(tops)} of them; it has "
          f"{len(best)} members beyond the unit's promotions, and the others have "
          f"{', '.join(f'{k} ({c:,})' for k, c in sorted(Counter(len(s) for _, s in sets).items()))}\n")
    rows = []
    for delta in deltas:
        near = [(v, s) for v, s in sets if b - v <= delta]
        swapped = sorted(len(best - s) for _, s in near if s != best)
        if not swapped:
            continue
        paired, pure, in_pairs, twin_pairs = 0, 0, 0, 0
        moved_out, moved_in = set(), set()
        for v, s in near:
            if s == best:
                continue
            removed, added = best - s, s - best
            m, match = pairs_off(removed, added, home)
            paired += m
            pure += m == len(removed) == len(added)
            moved_out |= removed
            moved_in |= added
            ways = Counter()
            for y, x in match.items():
                g = math.gcd(x, y)
                ways[(x // g, y // g)] += 1
            twin_pairs += len(match)
            in_pairs += sum(2 * min(c, ways[(q, p)]) for (p, q), c in ways.items() if p < q)
        rows.append([f"≤ {delta}", f"{len(swapped):,}", swapped[len(swapped) // 2], swapped[-1],
                     pct(pure, len(swapped)), pct(paired, sum(swapped)), pct(in_pairs, twin_pairs), len(moved_out),
                     len(moved_in)])
    table(rows, ["within δ of the best", "others", "members swapped, median", "most", "all pair off as twins",
                 "members swapped that pair off as twins", "twin pairs in paired exchanges",
                 "members of the best that move", "numbers that come in"])
    near = [(v, s) for v, s in sets if b - v <= deltas[-1] and s != best]
    homes_out, prime_pairs, moved = Counter(), Counter(), Counter()
    for v, s in near:
        removed, added = best - s, s - best
        _, match = pairs_off(removed, added, home)
        for y, x in match.items():
            g = math.gcd(x, y)
            prime_pairs[tuple(sorted((x // g, y // g)))] += 1
            moved[(x, y)] += 1
        homes_out.update(home.get(x, -1) for x in removed)
    print(f"Within {deltas[-1]} of the best, the members swapped out by home: "
          f"{', '.join(f'{h}: {pct(c, sum(homes_out.values()))}' for h, c in sorted(homes_out.items()))}.  The primes "
          f"exchanged most: " + ", ".join(f"{p} ↔ {q} ({c:,})" for (p, q), c in prime_pairs.most_common(8)) + ".\n")
    rows = [[f"{x} = {factor(x)}", f"{y} = {factor(y)}", x - y, home.get(x), pct(c, len(near))]
            for (x, y), c in moved.most_common(10)]
    table(rows, ["out", "in", "x − y", "home", "of the others"])
    singles = sorted(b - v for v, s in sets if len(best - s) == 1 and len(s - best) == 1)
    print(f"One-for-one swaps of the best: {len(singles)}, priced {singles}.\n")


# ---------------------------------------------------------------------------------------------------------------
# 3. The arithmetic of the problems


def cheap_moves(chosen, cands, home, limit=CHEAP):
    """The cheap moves of the set `chosen` among `cands`: twin swaps x = m·p out for y = m·q in (the same home) that
    cost 0 to `limit`, and paired exchanges, x1 = m1·p and x2 = m2·q out for y1 = m1·q and y2 = m2·p in (each pair of
    the same home), that cost (p − q)(m1 − m2), 0 to `limit`."""
    outside = defaultdict(list)  # (m, q) for each candidate y = m·q not in the set
    for y in cands - chosen:
        for q in primes_of(y):
            outside[y // q].append((q, y))
    singles, paired = set(), set()
    for x1 in chosen:
        for p in primes_of(x1):
            for q, y1 in outside[x1 // p]:
                if q == p or home.get(y1) != home.get(x1):
                    continue
                if 0 <= x1 - y1 < limit:
                    singles.add((x1, y1))
                for x2 in chosen:
                    if x2 == x1 or x2 % q:
                        continue
                    y2 = x2 // q * p
                    if y2 in cands and y2 not in chosen and y2 != y1 and home.get(y2) == home.get(x2) \
                            and 0 <= (x1 + x2) - (y1 + y2) < limit:
                        paired.add(frozenset(((x1, y1), (x2, y2))))
    return singles, paired


def arithmetic(uid, cands, ref, home, gap, proof):
    """The problem's first heaviest set against its candidates: each member's twins among the candidates of its home,
    and the cheapest below it; the cheap moves; the smooth members and the large primes."""
    by_home = defaultdict(list)
    for y in cands:
        by_home[home.get(y)].append(y)
    cheapest, counts = [], []
    for x in sorted(ref):
        same = [y for y in by_home[home.get(x)] if twins(x, y)]
        below = [x - y for y in same if y < x]
        counts.append(len(same))
        cheapest.append(min(below) if below else None)
    priced = sorted(c for c in cheapest if c is not None)
    singles, paired = cheap_moves(set(ref), cands, home)
    in_moves = {x for x, _ in singles} | {x for pair in paired for x, _ in pair}
    smooth = sum(1 for x in ref if max(primes_of(x)) <= 7)
    return [uid, gap, f"{proof:,}", len(cands), len(ref), sorted(counts)[len(counts) // 2],
            pct(sum(1 for c in priced if c < CHEAP), len(ref)), priced[len(priced) // 2] if priced else "-",
            len(singles), len(paired), pct(len(in_moves), len(ref)), pct(smooth, len(ref))]


def main():
    runs, sets_dir, cand_dir, suite_path = sys.argv[1:5]
    lines = [json.loads(line) for line in open(runs) if line.strip()]
    units = {d["id"]: d for d in map(json.loads, open(suite_path)) if d.get("kind") == "unit"}
    chain = {d["n"]: d["nodes"] for d in map(json.loads, open(CHAIN))}
    counts(lines, units, chain)
    homes = {int(p.name.split(".")[0]): read_candidates(p) for p in Path(cand_dir).glob("*.candidates.tsv")}
    opts = {d["id"]: d["opt"] for d in lines if d["run"] == "count"}
    proofs = {str(d["n"]): d for d in lines if d["run"] == "proof"}
    print("## 2. The completions near the best\n")
    refs = {}
    for path in sorted(Path(sets_dir).glob("*.sets.tsv")):
        uid = path.name.split(".")[0]
        refs[uid], sets = read_sets(path)
        if len(sets) > 1:
            deltas = (10, 30, 50) if "-" in uid else (10, 20, 30)
            cloud(uid, opts[uid], sets, homes[int(uid.split("-")[0])], deltas)
    print("## 3. The arithmetic of the problems\n")
    rows = []
    for uid, ref in refs.items():
        home = homes[int(uid.split("-")[0])]
        if uid in units:
            cands, gap, proof = from_hex(units[uid]["cands"]), units[uid]["gap"], units[uid]["default"]["nodes"]
        else:
            cands, gap, proof = set(home), proofs[uid]["gap_opt"], proofs[uid]["nodes"]
        rows.append(arithmetic(uid, cands, ref, home, gap, proof))
    table(rows, ["problem", "gap", "proof's steps", "candidates", "first heaviest set", "twins a member (median)",
                 "members with a twin under 50 below", "cheapest twin below (median)", "cheap twin swaps",
                 "cheap paired exchanges", "members in a cheap move", "7-smooth members"])
    # The cores of the anatomy's traced units: the minimal conflicts of their first heaviest sets.
    rows = []
    for d in map(json.loads, open(ANATOMY_RUNS)):
        if d.get("run") != "traced":
            continue
        uid, home = d["id"], homes[d["n"]]
        cands, cores = from_hex(units[uid]["cands"]), d["cores"]
        members = [x for k in cores for x in k]
        twin_counts, cheap, homes_of = [], 0, Counter()
        for x in members:
            same = [y for y in cands if home.get(y) == home.get(x) and twins(x, y)]
            twin_counts.append(len(same))
            cheap += any(0 <= x - y < CHEAP for y in same)
            homes_of[home.get(x)] += 1
        rows.append([uid, f"{units[uid]['default']['nodes']:,}", " + ".join(str(len(k)) for k in cores),
                     sorted(twin_counts)[len(twin_counts) // 2], cheap,
                     sum(1 for x in members if max(primes_of(x)) <= 7),
                     ", ".join(f"{h}: {c}" for h, c in sorted(homes_of.items()))])
    table(rows, ["unit", "proof's steps", "cores' members", "twins of a member in its home (median)",
                 "members with a twin under 50 below", "7-smooth members", "members by home"])


if __name__ == "__main__":
    main()
