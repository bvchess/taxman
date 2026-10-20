#!/usr/bin/env python3
"""The tables of "What the learning leaves in the hardest searches" (README.md beside this file), from the files of
`wall anatomy --trace` and `wall images --trace`.

usage: python3 -I residual.py profile DIR_A DIR_B ID...
       python3 -I residual.py misses DIR ID... [--every K] [--period P]
       python3 -I residual.py windows DIR ID... [--period P]
       python3 -I residual.py branched DIR_A DIR_B ID...

DIR holds what `wall anatomy --trace DIR` writes, with `--learn --closure` or without: `ID.steps.tsv`,
`ID.nogoods.tsv` and `N.candidates.tsv`; their fields are defined in the module comment of
solver/frame/examples/wall/anatomy.rs.

* `profile` puts two searches of each unit side by side, as the anatomy took one apart: the tree, how its steps end,
  the exclusions and their prices, and the conflicts computed and known, with where the known ones came from.
* `misses` takes apart the conflicts a search computed.  For each, it looks among the nogoods learned before it (the
  search's own and those learned within images) for one that a few moves of members turn into a nogood that would have
  given the conflict at its node, and names the moves by the factorizations of the members.  `--every K` takes every
  K-th computed conflict only; `--period P` is the searcher's forgetting period (50,000 steps by default).
* `windows` follows a search period by period (`--period P`): the conflicts computed and the share whose nogood was
  learned before, so forgotten at a clearing; the conflicts known and the share that image nogoods give.
* `branched` puts side by side the conflicts two searches of each unit branched on: their sizes, the prices of the
  exclusions, and the cheap ones by the substitution the bound made.  Its DIRs hold `ID.images.tsv`, the rows that
  `wall images --trace DIR` writes without `--learn` (solver/frame/examples/wall/images.rs, "The rows").

Standard library only; the anatomy's script (beside its README.md) supplies the tree.
"""
import itertools
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "the-anatomy-of-a-hard-subtree-2026-10"))
import analyze as anatomy  # noqa: E402  (the anatomy's tables: build_tree, quantiles, pct, table)

CHEAP = 50  # an exchange priced under this is cheap (the learning's line)
MOVES = 3  # the most members a transformation moves
KIN = 16  # kin share a divisor m with cofactors at most this

# ---------------------------------------------------------------------------------------------------------------------
# The files


class Steps:
    """The columns of a steps file that the tables use (`end` as text, the rest as integers)."""

    COLS = "d np need nv kid kc ksrc inh br nadd sadd nf cv ckid ckc cksrc".split()

    def __init__(self, path):
        with open(path) as f:
            header = next(f).rstrip("\n").split("\t")
            where = {h: i for i, h in enumerate(header)}
            want = [c for c in self.COLS if c in where] + ["end"]
            cols = {c: [] for c in want}
            pos = [(cols[c], where[c]) for c in want]
            for line in f:
                v = line.split("\t")
                for col, i in pos:
                    col.append(v[i])
        for c in want:
            setattr(self, c, cols[c] if c == "end" else list(map(int, cols[c])))
        self.n = len(cols["end"])


def read_candidates(path):
    """The game's candidates in ascending order (bit i of a set in the nogoods file is the i-th), with each one's
    home."""
    home, _ = anatomy.read_candidates(path)
    return sorted(home), home


class Record:
    """A line of the nogoods file: a nogood the searcher learned, with the check it came from."""

    __slots__ = ("step", "src", "d", "np", "nogood", "conflict", "set", "promoted", "chosen", "cands", "origin")

    def __init__(self, fields, origin):
        hexint = lambda v: None if v == "-" else int(v, 16)  # noqa: E731
        step, src, d, np, nogood, conflict, set_, promoted, chosen, cands = fields
        self.step, self.src, self.d, self.np = int(step), src, int(d), int(np)
        self.nogood, self.conflict, self.set = hexint(nogood), hexint(conflict), hexint(set_)
        self.promoted, self.chosen, self.cands = hexint(promoted), hexint(chosen), hexint(cands)
        # The check the nogood was learned at: its own for C, the C line it follows for I.
        self.origin = self if src == "C" else origin


def read_nogoods(path):
    records, origin = [], None
    with open(path) as f:
        next(f)
        for line in f:
            r = Record(line.rstrip("\n").split("\t"), origin)
            if r.src == "C":
                origin = r
            records.append(r)
    return records


def bits(v):
    """The indices of the set bits of `v`."""
    out = []
    while v:
        low = v & -v
        out.append(low.bit_length() - 1)
        v ^= low
    return out


# ---------------------------------------------------------------------------------------------------------------------
# profile


def profile(path):
    """The figures of one search, from its steps file (see the module docs)."""
    s = Steps(path)
    nodes, _, value = anatomy.build_tree(s)
    f = {}
    leaves = sum(1 for n in nodes if not n.children)
    f["steps"] = s.n
    f["nodes; leaves"] = (len(nodes), leaves)
    f["nodes with one child"] = anatomy.pct(sum(1 for n in nodes if len(n.children) == 1), len(nodes))
    f["deepest stack"] = max(n.depth for n in nodes)
    ends = Counter(s.end)
    for codes, name in ((("L7", "C7"), "steps ended by the GF(2) bound (node or child)"),
                        (("L3", "C3"), "steps ended by a conflict of forced members (node or child)"),
                        (("F",), "promotions that failed"), (("L0",), "loops that ran out of candidates")):
        f[name] = anatomy.pct(sum(ends.get(c, 0) for c in codes), s.n)
    branching = sum(1 for x in s.br if x)
    f["branching steps"] = branching
    # Exclusions: the branching steps of a node before the one that made the next node on a path (as the anatomy).
    excl = [0] * len(nodes)
    drops = []
    for k, n in enumerate(nodes):
        made, branched = {nodes[c].first - 1: c for c in n.children}, 0
        for i in n.rows:
            if i in made:
                excl[made[i]] = excl[k] + branched
            if s.br[i]:
                branched += 1
        for a, b in zip(n.rows, n.rows[1:]):
            if s.br[a] and value[a] is not None and value[b] is not None:
                drops.append((value[a] - s.need[a]) - (value[b] - s.need[b]))
    leaf_excl = [excl[k] for k, n in enumerate(nodes) if not n.children]
    q = anatomy.quantiles(leaf_excl)
    f["exclusions on a path to a leaf, median (p10-p90)"] = f"{q[1]} ({q[0]}-{q[2]})"
    f["exclusions priced"] = len(drops)
    f["price of an exclusion, median"] = anatomy.quantiles(drops)[1]
    f["exclusions priced under 50"] = anatomy.pct(sum(1 for d in drops if d < CHEAP), len(drops))
    f["exclusions priced 50-149"] = anatomy.pct(sum(1 for d in drops if CHEAP <= d < 150), len(drops))
    f["exclusions priced 150 or more"] = anatomy.pct(sum(1 for d in drops if d >= 150), len(drops))
    leaf_gaps = [n.gap0 for n in nodes if not n.children and n.gap0 is not None]
    f["gap at which a leaf closes, median"] = anatomy.quantiles(leaf_gaps)[1]
    f["leaves^(1/exclusions)"] = f"{leaves ** (1 / max(1, q[1])):.1f}"
    pts = [(n.gap0, math.log10(n.sub)) for n in nodes if n.gap0 is not None and n.gap0 >= 0 and n.sub >= 2]
    if len(pts) > 2:
        mx, my = sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts)
        slope = sum((p[0] - mx) * (p[1] - my) for p in pts) / sum((p[0] - mx) ** 2 for p in pts)
        f["a decade of subtree per ... points of gap"] = f"{1 / slope:.0f}"
    # The conflicts of the checks, computed or known, and where the known came from (`ksrc`: 1 own, 2 image, 3 both).
    checks = Counter()
    for kid, kc, src in ((s.kid, s.kc, getattr(s, "ksrc", None)), (s.ckid, s.ckc, getattr(s, "cksrc", None))):
        for i in range(s.n):
            if kid[i] < 0:
                continue
            checks["computed" if kc[i] else ("known", src[i] if src else -1)] += 1
    total = sum(checks.values())
    f["checks with a conflict"] = total
    f["conflicts computed"] = f"{checks['computed']:,} ({anatomy.pct(checks['computed'], total)})"
    for code, name in ((1, "own nogoods only"), (2, "image nogoods only"), (3, "both"), (0, "neither (a fault)")):
        c = checks[("known", code)]
        f[f"known from {name}"] = f"{c:,} ({anatomy.pct(c, total)})"
    # The branching steps by where the conflict they branch on came from: the node's check, or at a node's first step
    # the check it had as a child.
    by, resolved = Counter(), set()
    for i in range(s.n):
        if not s.br[i]:
            continue
        if s.kid[i] >= 0:
            kid, kc, src = s.kid[i], s.kc[i], s.ksrc[i] if hasattr(s, "ksrc") else -1
        elif s.inh[i] and i > 0 and s.ckid[i - 1] >= 0:
            kid, kc, src = s.ckid[i - 1], s.ckc[i - 1], s.cksrc[i - 1] if hasattr(s, "cksrc") else -1
        else:
            by["no conflict"] += 1
            continue
        resolved.add(kid)
        by["computed" if kc else {1: "own", 2: "image", 3: "both"}.get(src, "?")] += 1
    f["distinct conflicts branched on; branchings a conflict"] = \
        f"{len(resolved):,}; {(branching - by['no conflict']) / max(1, len(resolved)):.1f}"
    for key in ("computed", "own", "image", "both", "no conflict"):
        f[f"branching on a conflict: {key}"] = anatomy.pct(by[key], branching)
    return f


def run_profile(dir_a, dir_b, ids):
    for uid in ids:
        a, b = profile(Path(dir_a) / f"{uid}.steps.tsv"), profile(Path(dir_b) / f"{uid}.steps.tsv")
        print(f"## {uid}\n")
        rows = []
        for key in a:
            va, vb = a[key], b.get(key, "-")
            fmt = lambda v: f"{v:,}" if isinstance(v, int) else (f"{v[0]:,}; {v[1]:,}" if isinstance(v, tuple) else v)  # noqa: E731
            rows.append([key, fmt(va), fmt(vb)])
        anatomy.table(rows, ["", Path(dir_a).name, Path(dir_b).name])


# ---------------------------------------------------------------------------------------------------------------------
# misses: the relations of two numbers


def sieve(top):
    spf = list(range(top + 1))
    for p in range(2, int(top ** 0.5) + 1):
        if spf[p] == p:
            for k in range(p * p, top + 1, p):
                if spf[k] == k:
                    spf[k] = p
    return spf


class Numbers:
    def __init__(self, xs, home):
        self.xs, self.home = xs, home
        self.index = {x: i for i, x in enumerate(xs)}
        self.top = max(xs)
        self.spf = sieve(self.top)
        self.primes = [p for p in range(2, self.top + 1) if self.spf[p] == p]
        # For each candidate, its twins among the candidates (y = x / p · q, p and q prime), as a bit mask.
        self.twins = {}
        for x in xs:
            mask = 0
            for p in self.prime_factors(x):
                m = x // p
                for q in self.primes:
                    if m * q > self.top:
                        break
                    if q != p and m * q in self.index:
                        mask |= 1 << self.index[m * q]
            self.twins[x] = mask

    def is_prime(self, k):
        return 2 <= k <= self.top and self.spf[k] == k

    def prime_factors(self, x):
        out = []
        while x > 1:
            p = self.spf[x]
            out.append(p)
            while x % p == 0:
                x //= p
        return out

    def members(self, v):
        return [self.xs[i] for i in bits(v)]

    def relation(self, r, a):
        """How a member r of the earlier nogood relates to the member a that takes its place: `twin` (r = m·p,
        a = m·q, p and q prime), `kin` (r = m·u, a = m·v, u and v at most 16), `rescale` (one largest prime p, r = m·p,
        a = m'·p), or `other`; with the primes or cofactors."""
        g = math.gcd(r, a)
        u, v = r // g, a // g
        if self.is_prime(u) and self.is_prime(v):
            return ("twin", u, v, g)
        if u <= KIN and v <= KIN:
            return ("kin", u, v, g)
        pr, pa = self.prime_factors(r)[-1], self.prime_factors(a)[-1]
        if pr == pa and pr > KIN:
            return ("rescale", pr, r // pr, a // pr)
        return ("other",)


RANK = {"twin": 0, "kin": 1, "rescale": 2, "other": 3}


def rank(moves):
    """How good an explanation is, the smaller the better: the fewest unrelated moves, then the fewest moves, then the
    strongest relations."""
    kinds = [RANK[x[2][0]] for x in moves]
    return (kinds.count(RANK["other"]), len(kinds), sorted(kinds))


def cheap_proposal(nums, origin, conflict, r, a):
    """Whether the learning's closure proposes the swap r → a at the check `origin` (with the conflict `conflict` there,
    r a member of it): a cheap twin swap, or one a paired exchange makes cheap (images.rs, `cheap_images`)."""
    ix = nums.index
    cands, chosen = origin.cands, origin.chosen
    if not (cands >> ix[a]) & 1 or (conflict >> ix[a]) & 1:
        return False
    swap = r - a
    if 0 < swap < CHEAP:
        return True
    g = math.gcd(r, a)
    p, q = r // g, a // g
    for b in nums.members(chosen):
        if b in (r, a) or b % q:
            continue
        d = b // q * p
        if d > nums.top or d == r or d not in ix or not (cands >> ix[d]) & 1 or (chosen >> ix[d]) & 1:
            continue
        if 0 <= swap + b - d < CHEAP:
            return True
    return False


# ---------------------------------------------------------------------------------------------------------------------
# misses: the explanations


def explain(nums, n, m, fit):
    """The moves that turn the earlier nogood m into a set that holds the nogood n and fits the node of n (lies within
    `fit`, its promotions and heaviest set): each member a of n outside m takes the place of a member r of m outside
    n, every member of m outside n and outside `fit` must move, and the rest may stay.  Returns the best assignment
    (the fewest unrelated moves, then the strongest relations) as a list of (r, a, relation), or None."""
    added, removed = n & ~m, m & ~n
    out = removed & ~fit
    na, nout = added.bit_count(), out.bit_count()
    if na == 0 or na > MOVES or nout > na:
        return None
    A, R, must = nums.members(added), nums.members(removed), set(nums.members(out))
    options = []
    for a in A:
        rel = [(RANK[nums.relation(r, a)[0]], r) for r in R]
        related = sorted(x for x in rel if x[0] < RANK["other"] or x[1] in must)
        options.append([r for _, r in related] or [None])
    best = None
    for pick in itertools.product(*options):
        if None in pick or len(set(pick)) < len(pick) or not must <= set(pick):
            continue
        moves = [(r, a, nums.relation(r, a)) for r, a in zip(pick, A)]
        score = rank(moves)
        if best is None or score < best[0]:
            best = (score, moves)
    if best is None:
        # No assignment of related members covers what must move: the moves are unrelated.
        pool = list(must) + [r for r in R if r not in must]
        moves = [(r, a, nums.relation(r, a)) for r, a in zip(pool, A)]
        return moves
    return best[1]


def describe(nums, moves, earlier, alive):
    """The class of an explanation: its moves' relations and, for a single twin swap, why the learning missed it."""
    kinds = tuple(sorted(x[2][0] for x in moves))
    if len(moves) == 1 and kinds == ("twin",):
        r, a, (_, p, q, g) = moves[0]
        origin = earlier.origin
        ix = nums.index
        conflict = earlier.conflict
        if not (conflict >> ix[r]) & 1:
            why = "moves a member the node of the earlier one had promoted"
        elif not (origin.cands >> ix[a]) & 1:
            why = "its twin was no candidate at the node of the earlier one"
        elif a < r and cheap_proposal(nums, origin, conflict, r, a):
            why = "cheap: the closure proposes it" + ("" if alive else " (the earlier one forgotten)")
        elif a > r:
            why = "the twin is larger (a swap up)"
        else:
            why = "dear: priced 50 or more"
        home = "" if nums.home[r] == nums.home[a] else ", another home"
        return f"one twin swap: {why}{home}"
    if kinds == ("twin", "twin"):
        (_, _, (_, p1, q1, _)), (_, _, (_, p2, q2, _)) = moves
        if (p1, q1) == (p2, q2):
            shape = "one exchange on two payers"
        elif (p1, q1) == (q2, p2):
            shape = "a paired exchange"
        elif len({p1, q1} & {p2, q2}) == 1:
            shape = "two exchanges sharing a prime"
        else:
            shape = "two exchanges of four primes"
        return f"two twin swaps: {shape}"
    if kinds == ("twin", "twin", "twin"):
        primes = Counter()
        for _, _, (_, p, q, _) in moves:
            primes[p] += 1
            primes[q] += 1
        cycle = len(primes) == 3 and all(v == 2 for v in primes.values())
        return "three twin swaps: " + ("a cycle of three primes" if cycle else "other")
    unrelated = kinds.count("other")
    if unrelated:
        return f"{len(moves)} moves, {unrelated} of them unrelated"
    return f"{len(moves)} moves: " + " + ".join(kinds)


GROUPS = [
    "the same nogood, met before and forgotten",
    "the same conflict, another nogood",
    "a cheap twin swap (the closure's class)",
    "one twin swap, outside the closure's class",
    "two twin swaps",
    "three twin swaps",
    "moves of kin or rescaled payers (with twin swaps or not)",
    "unrelated moves, or none within 3 moves",
]


def group(label, same):
    """The first of GROUPS that holds a computed conflict, from its best explanation `label` and whether the same
    conflict was met before (`same`)."""
    if label.startswith("met before"):
        return GROUPS[0]
    if same is not None:
        return GROUPS[1]
    if label.startswith("one twin swap: cheap"):
        return GROUPS[2]
    for prefix, g in (("one twin", 3), ("two twin", 4), ("three twin", 5)):
        if label.startswith(prefix):
            return GROUPS[g]
    if "unrelated" in label or label.startswith("none"):
        return GROUPS[7]
    return GROUPS[6]


def run_misses(dir_, ids, every, period):
    dir_ = Path(dir_)
    for uid in ids:
        game = uid.split("-")[0]
        xs, home = read_candidates(dir_ / f"{game}.candidates.tsv")
        nums = Numbers(xs, home)
        records = read_nogoods(dir_ / f"{uid}.nogoods.tsv")
        computed = [i for i, r in enumerate(records) if r.src == "C" and r.nogood is not None]
        sample = computed[::every]
        print(f"## {uid}: {len(records):,} nogoods learned, {len(computed):,} of them of computed conflicts; "
              f"{len(sample):,} taken apart\n")
        classes, flags, nearest, sizes = Counter(), Counter(), Counter(), Counter()
        conflict_near, same_conflict, groups = Counter(), Counter(), Counter()
        sources = Counter()
        prices = defaultdict(list)
        cost = Counter()
        for i in sample:
            rec = records[i]
            n, fit = rec.nogood, rec.promoted | rec.chosen
            sizes[n.bit_count()] += 1
            near, best, label = None, None, None
            caught = set()
            k, knear, same = rec.conflict, None, None
            for j in range(i - 1, -1, -1):
                e = records[j]
                dk = (k ^ e.conflict).bit_count()
                if knear is None or dk < knear:
                    knear = dk
                if dk == 0 and same is None:
                    same = "own" if e.src == "C" else "image"
                m = e.nogood
                if m is None:
                    continue
                d = (n ^ m).bit_count()
                if near is None or d < near:
                    near = d
                if m == n:
                    alive = (e.step - 1) // period == (rec.step - 1) // period
                    caught.add("met before" + ("" if not alive else " and alive (a fault)"))
                    cand = (0, 0, [])
                    if best is None or cand < best:
                        best, label = cand, "met before, forgotten" if not alive else "met before, alive (a fault)"
                    continue
                moves = explain(nums, n, m, fit)
                if moves is None:
                    continue
                alive = (e.step - 1) // period == (rec.step - 1) // period
                text = describe(nums, moves, e, alive)
                caught.add(text)
                score = rank(moves)
                if best is None or score < best:
                    best, label = score, text
                    if text.startswith("one twin"):
                        price = moves[0][0] - moves[0][1]
                        best_price, best_src = price, e.src
            if label is None:
                label = f"none within {MOVES} moves"
            classes[label] += 1
            groups[group(label, same)] += 1
            nearest[near] += 1
            conflict_near[knear] += 1
            same_conflict[(same, label.startswith("met before"))] += 1
            if label.startswith("one twin"):
                prices[label].append(best_price)
                sources[(label, best_src)] += 1
            for c in caught:
                flags[c] += 1
            # What proposing every twin swap would cost here: of the conflict's members, at the node; of the nogood's
            # members, among all candidates.
            for r in nums.members(rec.conflict):
                cost["twin swaps of the conflict's members, at the node"] += \
                    (nums.twins[r] & rec.cands & ~rec.conflict).bit_count()
            for r in nums.members(n):
                cost["twin swaps of the nogood's members, all candidates"] += (nums.twins[r] & ~n).bit_count()
        total = len(sample)
        rows = [[c, f"{groups[c]:,}", anatomy.pct(groups[c], total)] for c in GROUPS]
        print("Each computed conflict in the first group that holds it:\n")
        anatomy.table(rows, ["group", "conflicts", "share"])
        rows = [[c, f"{v:,}", anatomy.pct(v, total)] for c, v in classes.most_common()]
        print("Each computed conflict by its simplest explanation (fewest moves, then the strongest relations):\n")
        anatomy.table(rows, ["class", "conflicts", "share"])
        rows = [[c, f"{v:,}", anatomy.pct(v, total)] for c, v in flags.most_common()]
        print("What each kind of transformation would have caught (a conflict may be caught by several):\n")
        anatomy.table(rows, ["transformation", "conflicts", "share"])
        rows = []
        for label, ps in sorted(prices.items(), key=lambda kv: -len(kv[1])):
            q = anatomy.quantiles(ps)
            src = sources[(label, "C")]
            rows.append([label, f"{len(ps):,}", f"{q[0]} / {q[1]} / {q[2]}", anatomy.pct(src, len(ps))])
        print("The single twin swaps: the price (r − a) p10 / median / p90, and the share whose earlier nogood was a "
              "computed conflict's (not one learned in an image):\n")
        anatomy.table(rows, ["class", "conflicts", "price", "from a computed conflict"])
        rows = [[d, f"{v:,}", anatomy.pct(v, total)] for d, v in sorted(nearest.items(), key=lambda kv: (kv[0] is None, kv[0] or 0))]
        print("The nearest earlier nogood, by symmetric difference:\n")
        anatomy.table(rows, ["distance", "conflicts", "share"])
        rows = [[d, f"{v:,}", anatomy.pct(v, total)]
                for d, v in sorted(conflict_near.items(), key=lambda kv: (kv[0] is None, kv[0] or 0))]
        print("The nearest earlier conflict (the conflicts computed or learned, not their nogoods):\n")
        anatomy.table(rows, ["distance", "conflicts", "share"])
        rows = [[f"{same or 'none'}; the nogood {'met before' if met else 'new'}", f"{v:,}", anatomy.pct(v, total)]
                for (same, met), v in same_conflict.most_common()]
        print("The same conflict met before (computed: own; learned in an image), and whether its nogood was:\n")
        anatomy.table(rows, ["same conflict before", "conflicts", "share"])
        q = anatomy.quantiles([k for k, v in sizes.items() for _ in range(v)])
        print(f"Nogood size p10/median/p90 {q[0]}/{q[1]}/{q[2]}.  Proposals a computed conflict would cost: " +
              "; ".join(f"{k} {v / max(1, total):.1f}" for k, v in cost.items()) + ".\n")


# ---------------------------------------------------------------------------------------------------------------------
# windows


def run_windows(dir_, ids, period):
    dir_ = Path(dir_)
    for uid in ids:
        seen, computed, known = set(), Counter(), Counter()
        for r in read_nogoods(dir_ / f"{uid}.nogoods.tsv"):
            if r.src == "C" and r.nogood is not None:
                w = (r.step - 1) // period
                computed[(w, "all")] += 1
                computed[(w, "met")] += r.nogood in seen
            if r.nogood is not None:
                seen.add(r.nogood)
        s = Steps(dir_ / f"{uid}.steps.tsv")
        for i in range(s.n):
            for code in (s.ksrc[i], s.cksrc[i]):
                if code >= 1:
                    known[(i // period, "all")] += 1
                    known[(i // period, "image")] += code >= 2
        print(f"## {uid}, by period of {period:,} steps\n")
        rows, total = [], Counter()
        for w in range(max(k[0] for k in known) + 1):
            for key in ("all", "met"):
                total[("c", key)] += computed[(w, key)]
            for key in ("all", "image"):
                total[("k", key)] += known[(w, key)]
            rows.append([f"{w * period:,}", f"{computed[(w, 'all')]:,}",
                         anatomy.pct(computed[(w, "met")], computed[(w, "all")]),
                         anatomy.pct(total[("c", "met")], total[("c", "all")]), f"{known[(w, 'all')]:,}",
                         anatomy.pct(known[(w, "image")], known[(w, "all")]),
                         anatomy.pct(total[("k", "image")], total[("k", "all")])])
        anatomy.table(rows, ["from step", "computed", "learned before", "so far", "known", "from images", "so far"])


# ---------------------------------------------------------------------------------------------------------------------
# branched


def read_rows(path):
    """The test's rows that `branched` uses: depth, price (None if unpriced), substitution and conflict size."""
    rows = []
    with open(path) as f:
        at = {h: i for i, h in enumerate(next(f).rstrip("\n").split("\t"))}
        for line in f:
            v = line.rstrip("\n").split("\t")
            price = None if v[at["price"]] == "-" else int(v[at["price"]])
            rows.append((int(v[at["d"]]), price, v[at["kind"]], int(v[at["nk"]])))
    return rows


def branched(rows):
    """The figures of the conflicts one search branched on (see the module docs)."""
    f = {"exclusions": f"{len(rows):,}"}
    sizes = [r[3] for r in rows]
    priced = [r for r in rows if r[1] is not None]
    cheap = [r for r in priced if r[1] < CHEAP]
    q = anatomy.quantiles(sizes)
    f["conflict size, p10/median/p90; mean"] = f"{q[0]}/{q[1]}/{q[2]}; {sum(sizes) / len(sizes):.2f}"
    f["conflicts of 2 members"] = anatomy.pct(sizes.count(2), len(sizes))
    q = anatomy.quantiles([r[1] for r in priced])
    f["price, p10/median/p90"] = f"{q[0]}/{q[1]}/{q[2]}"
    f["priced under 50"] = anatomy.pct(len(cheap), len(priced))
    f["of those, conflicts of 2 members"] = anatomy.pct(sum(1 for r in cheap if r[3] == 2), len(cheap))
    for kind, name in (("T", "twin swaps"), ("P", "paired exchanges"), ("O", "other substitutions")):
        n, c = sum(1 for r in priced if r[2] == kind), sum(1 for r in cheap if r[2] == kind)
        f[f"{name}: share of the exclusions; cheap"] = f"{anatomy.pct(n, len(priced))}; {anatomy.pct(c, n)}"
    for lo, hi in ((0, 2), (3, 9), (10, None)):
        part = [r for r in priced if lo <= r[0] and (hi is None or r[0] <= hi)]
        if part:
            mean = sum(r[3] for r in part) / len(part)
            name = f"depth {lo}+" if hi is None else f"depth {lo}-{hi}"
            f[f"{name}: exclusions; mean size; median price"] = \
                f"{len(part):,}; {mean:.2f}; {anatomy.quantiles([r[1] for r in part])[1]}"
    return f


def run_branched(dir_a, dir_b, ids):
    for uid in ids:
        a, b = (branched(read_rows(Path(d) / f"{uid}.images.tsv")) for d in (dir_a, dir_b))
        print(f"## {uid}\n")
        anatomy.table([[key, a[key], b.get(key, "-")] for key in a], ["", Path(dir_a).name, Path(dir_b).name])


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args[0] == "profile":
        return run_profile(args[1], args[2], args[3:])
    if args[0] == "misses":
        every, period, rest = 1, 50_000, []
        it = iter(args[1:])
        for a in it:
            if a == "--every":
                every = int(next(it))
            elif a == "--period":
                period = int(next(it))
            else:
                rest.append(a)
        return run_misses(rest[0], rest[1:], every, period)
    if args[0] == "windows":
        period, rest = 50_000, []
        it = iter(args[1:])
        for a in it:
            if a == "--period":
                period = int(next(it))
            else:
                rest.append(a)
        return run_windows(rest[0], rest[1:], period)
    if args[0] == "branched":
        return run_branched(args[1], args[2], args[3:])
    sys.exit(__doc__)


if __name__ == "__main__":
    main()
