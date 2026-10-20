#!/usr/bin/env python3
"""The tables of the anatomy of a hard subtree (README.md beside this file), from the files of `wall anatomy`.

usage: python3 -I analyze.py TRACE_DIR RUNS.jsonl...
       python3 -I analyze.py --orders RUNS.jsonl...
       python3 -I analyze.py --model SUITE TRACE_DIR RUNS.jsonl...

TRACE_DIR holds `ID.steps.tsv`, `ID.conflicts.tsv` and `N.candidates.tsv` (`wall anatomy --trace`); the RUNS files hold
the lines of `wall anatomy` (with `--best-cpu`, the best completion and the cores of the heaviest set).  Every unit of
the runs whose trace is in TRACE_DIR is taken apart: the shape of its tree, what it branches on, the gap along the
tree, the repetition of residual problems, the conflicts, and the heaviest set against the best completion.  The
fields of the files are defined in the module comment of solver/frame/examples/wall/anatomy.rs.  Standard library
only.
"""
import itertools
import json
import math
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

# The `search::cut` reasons, by number.
CUTS = ["empty", "largest", "match", "refined", "exact", "forced", "lagrange", "gf2", "symmetry"]


def end_name(code):
    """`P`, `E`, `F`, `C<r>` or `L<r>` in words."""
    if code[0] in "CL":
        return ("child cut: " if code[0] == "C" else "loop end: ") + CUTS[int(code[1:])]
    return {"P": "child pushed", "E": "child with no candidates", "F": "promotion failed"}[code]


def read_candidates(path):
    """For each candidate: its home frame and its maximal factors."""
    home, factors = {}, {}
    with open(path) as f:
        next(f)
        for line in f:
            x, h, fs = line.rstrip("\n").split("\t")
            home[int(x)] = int(h)
            factors[int(x)] = [int(y) for y in fs.split(",") if y]
    return home, factors


def read_conflicts(path):
    with open(path) as f:
        next(f)
        return [[int(x) for x in line.rstrip("\n").split("\t")[2].split(",") if x] for line in f]


class Steps:
    """The columns of a steps file."""

    def __init__(self, path):
        with open(path) as f:
            header = next(f).rstrip("\n").split("\t")
            cols = [[] for _ in header]
            for line in f:
                for c, v in zip(cols, line.rstrip("\n").split("\t")):
                    c.append(v)
        self.names = header
        for name, col in zip(header, cols):
            setattr(self, name, col if name in ("end", "gain", "loss") else [int(v) for v in col])
        self.frames = [n for n in header if n.startswith("r") and n != "rall"]  # r0, r1, ...
        self.n = len(cols[0])


class Node:
    __slots__ = ("parent", "depth", "first", "rows", "gap0", "sub", "children", "np", "need")

    def __init__(self, parent, depth, first, np, need):
        self.parent, self.depth, self.first, self.np, self.need = parent, depth, first, np, need
        self.rows, self.children, self.gap0, self.sub = [], [], None, 0


def build_tree(s):
    """The tree's nodes in the order of their first steps, and for each step its node and the bound it had (the
    node's own check, or at its first step the check it had as a child; None where it had none)."""
    nodes, stack, node_of, value = [], [], [0] * s.n, [None] * s.n
    for i in range(s.n):
        d = s.d[i]
        if d == len(stack):
            parent = stack[-1] if stack else -1
            node = Node(parent, d, i, s.np[i], s.need[i])
            nodes.append(node)
            if parent >= 0:
                nodes[parent].children.append(len(nodes) - 1)
            stack.append(len(nodes) - 1)
        else:
            del stack[d + 1:]
        k = stack[d]
        node_of[i] = k
        nodes[k].rows.append(i)
        if s.nv[i] >= 0:
            value[i] = s.nv[i]
        elif s.inh[i] and i > 0 and s.cv[i - 1] >= 0:
            value[i] = s.cv[i - 1]
        if nodes[k].gap0 is None and value[i] is not None and nodes[k].rows[0] == i:
            nodes[k].gap0 = value[i] - s.need[i]
    for node in reversed(nodes):
        node.sub += len(node.rows)
        if node.parent >= 0:
            nodes[node.parent].sub += node.sub
    return nodes, node_of, value


def pct(a, b):
    return f"{100.0 * a / b:.1f}%" if b else "-"


def quantiles(xs, qs=(0.1, 0.5, 0.9)):
    if not xs:
        return ["-" for _ in qs]
    xs = sorted(xs)
    return [xs[min(len(xs) - 1, int(q * len(xs)))] for q in qs]


def bucket(gap, width=100):
    if gap is None:
        return None
    if gap < 0:
        return "<0"
    lo = (gap // width) * width
    return f"{lo}-{lo + width - 1}"


def bucket_key(b):
    return -1 if b == "<0" else int(b.split("-")[0])


def table(rows, header):
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    for r in rows:
        print("| " + " | ".join(str(x) for x in r) + " |")
    print()


def shape(s, nodes):
    print("### 1. Shape\n")
    leaves = sum(1 for n in nodes if not n.children)
    chain = sum(1 for n in nodes if len(n.children) == 1)
    print(f"{s.n:,} steps at {len(nodes):,} nodes ({s.n / len(nodes):.2f} steps a node); {leaves:,} leaves "
          f"({pct(leaves, len(nodes))}), {chain:,} nodes with one child ({pct(chain, len(nodes))}); "
          f"deepest stack {max(n.depth for n in nodes)}.\n")
    by_depth = defaultdict(lambda: [0, 0, 0])
    for n in nodes:
        b = by_depth[n.depth]
        b[0] += 1
        b[1] += len(n.rows)
        b[2] += len(n.children)
    rows, seen = [], 0
    for d in sorted(by_depth):
        cnt, steps, kids = by_depth[d]
        seen += steps
        if d < 12 or d % 4 == 0 or d == max(by_depth):
            rows.append([d, f"{cnt:,}", f"{steps:,}", f"{kids / cnt:.2f}", pct(seen, s.n)])
    print("Nodes by their depth on the stack below the unit's node (rows every 4 past 11):\n")
    table(rows, ["depth", "nodes", "steps", "children a node", "steps so far"])
    ends = Counter(s.end)
    print("How the steps ended: " + "; ".join(f"{end_name(k)} {pct(v, s.n)}" for k, v in ends.most_common()) + ".\n")


def branches(s, nodes, node_of, home, top):
    """What the steps branch on: the members' homes and live factors, by count and by the steps under the child."""
    print("### 2. What is branched on\n")
    child_of = {}
    for k, n in enumerate(nodes):
        for c in n.children:
            child_of[nodes[c].first - 1] = c
    by_home, by_lf, by_x = defaultdict(lambda: [0, 0]), defaultdict(lambda: [0, 0]), defaultdict(lambda: [0, 0])
    forced = [0, 0]
    total = 0
    for i in range(s.n):
        x = s.br[i]
        if not x:
            continue
        under = nodes[child_of[i]].sub if i in child_of else 0
        total += under
        for table_, key in ((by_home, home.get(x)), (by_lf, s.lf[i]), (by_x, x)):
            table_[key][0] += 1
            table_[key][1] += under
        forced[0 if s.nadd[i] <= 1 else 1] += 1
    nb = sum(v[0] for v in by_home.values())
    print(f"{nb:,} branching steps; {forced[1]:,} of them promote two or more forced members at once.\n")
    table([[h, f"{v[0]:,}", pct(v[0], nb), pct(v[1], total)] for h, v in sorted(by_home.items())],
          ["home of the member", "steps", "share", "share of the steps under the children"])
    table([[lf, f"{v[0]:,}", pct(v[0], nb), pct(v[1], total)] for lf, v in sorted(by_lf.items())],
          ["live factors of the member", "steps", "share", "share of the steps under the children"])
    heavy = sorted(by_x.items(), key=lambda kv: -kv[1][1])[:top]
    rows = []
    for x, (cnt, under) in heavy:
        rows.append([x, home.get(x), f"{cnt:,}", pct(under, total)])
    print(f"The {top} members with the most steps under the children that promote them:\n")
    table(rows, ["member", "home", "times branched on", "share of the steps under"])


def top_levels(s, nodes, home, levels=3, width=6):
    """The decisions of the unit's node and of the heaviest child at each of the next levels, by name."""
    print("The top levels by name: at each node, its loop's branches in order (\"promote x\", the steps under that "
          "child, the node's gap at that step), then the heaviest child's loop:\n")
    k = 0
    for level in range(levels + 1):
        n = nodes[k]
        kids = {nodes[c].first - 1: c for c in n.children}
        parts = []
        for i in n.rows:
            if s.br[i]:
                under = nodes[kids[i]].sub if i in kids else 0
                tag = f"promote {s.br[i]} (home {home.get(s.br[i])}, {s.lf[i]} live)"
                if s.nadd[i] > 1:
                    tag = f"promote {s.nadd[i]} forced incl. {s.br[i]}"
                parts.append(f"{tag}: {under:,}")
        heavy = max(n.children, key=lambda c: nodes[c].sub, default=None)
        print(f"* depth {n.depth} ({n.np} promotions, gap {n.gap0}, {n.sub:,} steps, {len(n.rows)} steps of its "
              f"own): " + "; ".join(parts[:width]) + (f"; ... {len(parts) - width} more" if len(parts) > width else ""))
        if heavy is None:
            break
        k = heavy
    print()


def gaps(s, nodes, node_of, value, home):
    print("### 3. The gap along the tree\n")
    groups = defaultdict(list)
    for n in nodes:
        b = bucket(n.gap0)
        if b is not None:
            groups[b].append(n.sub)
    rows = []
    for b in sorted(groups, key=bucket_key):
        subs = groups[b]
        q = quantiles(subs)
        rows.append([b, f"{len(subs):,}", q[0], q[1], q[2], f"{sum(subs):,}"])
    print("Nodes by their gap when they are made (bound less need), and the steps of their subtrees:\n")
    table(rows, ["gap", "nodes", "subtree p10", "median", "p90", "steps under all"])
    pts = [(n.gap0, math.log10(n.sub)) for n in nodes if n.gap0 is not None and n.gap0 >= 0 and n.sub >= 2]
    if len(pts) > 2:
        mx, my = statistics.fmean(p[0] for p in pts), statistics.fmean(p[1] for p in pts)
        sxx = sum((p[0] - mx) ** 2 for p in pts)
        slope = sum((p[0] - mx) * (p[1] - my) for p in pts) / sxx
        print(f"log10(subtree) on the gap over the {len(pts):,} nodes of two steps or more: a decade per "
              f"{1 / slope:.0f} points.\n")
    # How the decisions move the gap.
    promote, skip = defaultdict(list), defaultdict(list)
    for n in nodes:
        rows_ = n.rows
        for a, b in zip(rows_, rows_[1:] + [None]):
            ga = value[a] - s.need[a] if value[a] is not None else None
            if ga is None or not s.br[a]:
                continue
            h = home.get(s.br[a])
            if s.cv[a] >= 0:
                promote[h].append(s.cv[a] - (s.need[a] - s.sadd[a]) - ga)
            if b is not None and value[b] is not None:
                skip[h].append(value[b] - s.need[b] - ga)
    rows = []
    for h in sorted(set(promote) | set(skip)):
        p, k = promote.get(h, []), skip.get(h, [])
        rows.append([h, f"{len(p):,}", quantiles(p)[1], f"{statistics.fmean(p):.1f}" if p else "-",
                     f"{len(k):,}", quantiles(k)[1], f"{statistics.fmean(k):.1f}" if k else "-"])
    print("What a decision does to the gap, by the home of the member: promoting it (the child's gap less the "
          "node's) and going on without it (the node's next gap less this one):\n")
    table(rows, ["home", "promote: steps", "median change", "mean", "without: steps", "median change", "mean"])
    # The heaviest path.
    k, path = 0, []
    while True:
        n = nodes[k]
        path.append((n.depth, n.np, n.gap0, n.sub))
        if not n.children:
            break
        k = max(n.children, key=lambda c: nodes[c].sub)
    print("Along the heaviest path (each node's heaviest child): depth, promotions, gap, subtree:\n")
    print("  " + "; ".join(f"{d}/{p}/{g}/{sub:,}" for d, p, g, sub in path[:: max(1, len(path) // 25)]) + "\n")
    # Exclusions along a path: the branching steps of its nodes before the one that made the next node on it, each a
    # member the path's completions leave out.
    excl = [0] * len(nodes)
    for k, n in enumerate(nodes):
        branched = 0
        kids = {nodes[c].first - 1: c for c in n.children}
        for i in n.rows:
            if i in kids:
                excl[kids[i]] = excl[k] + branched
            if s.br[i]:
                branched += 1
    leaf_excl = [excl[k] for k, n in enumerate(nodes) if not n.children]
    q = quantiles(leaf_excl)
    drops = [d for h in skip for d in skip[h]]
    ratio = []
    for n in nodes:
        for a, b in zip(n.rows, n.rows[1:]):
            if s.br[a] and value[a] is not None and value[b] is not None:
                ratio.append((value[a] - value[b]) / s.br[a])
    print(f"Exclusions on the way to a leaf p10/median/p90 {q[0]}/{q[1]}/{q[2]}; the root's gap over the median drop "
          f"of an exclusion ({-quantiles(drops)[1]}) is {nodes[0].gap0 / max(1, -quantiles(drops)[1]):.1f}.  An "
          f"exclusion lowers the bound by a median {100 * quantiles(ratio)[1]:.1f}% of the member's value (p10/p90 "
          f"{100 * quantiles(ratio)[0]:.1f}%/{100 * quantiles(ratio)[2]:.1f}%): the bound finds a substitute for the "
          f"rest.  Exclusions that cost the bound under 25 points: {pct(sum(1 for d in drops if d > -25), len(drops))}; "
          f"under 50: {pct(sum(1 for d in drops if d > -50), len(drops))}.\n")
    leaf_gaps = [n.gap0 for n in nodes if not n.children and n.gap0 is not None]
    small = [n.gap0 for n in nodes if n.sub <= 3 and n.gap0 is not None]
    q, r = quantiles(leaf_gaps), quantiles(small)
    print(f"Where subtrees close: the gap of a leaf (a node with no child) p10/median/p90 {q[0]}/{q[1]}/{q[2]}; of a "
          f"node of at most three steps {r[0]}/{r[1]}/{r[2]}.\n")


def repetition(s, nodes):
    print("### 4. Repetition\n")
    firsts = [n.first for n in nodes]
    m = len(firsts)
    print(f"At the first steps of the {m:,} nodes: the distinct residues (a frame's state and the node's candidates "
          f"touching it; all: every frame and every candidate) and the distinct states (a frame's moves, factors and "
          f"freedoms; all: every frame's):\n")
    rows = []
    for f in range(len(s.frames)):
        r, st = getattr(s, f"r{f}"), getattr(s, f"s{f}", None)
        rows.append([f, f"{len(set(r[i] for i in firsts)):,}",
                     f"{len(set(st[i] for i in firsts)):,}" if st else "-"])
    rall, sall = s.rall, getattr(s, "sall", None)
    rows.append(["all", f"{len(set(rall[i] for i in firsts)):,}",
                 f"{len(set(sall[i] for i in firsts)):,}" if sall else "-"])
    table(rows, ["frame", "residues", "states"])
    # A node whose whole residue an earlier node had at a need no larger is refuted by that node's subtree (the same
    # completions, needing at least as much): a table of residues would have saved its subtree.
    least, again, dominated, saved, inside = {}, 0, 0, 0, -1
    for n in nodes:
        key = rall[n.first]
        if key in least:
            again += 1
            if least[key] <= n.need and n.first > inside:
                dominated += 1
                saved += n.sub
                inside = n.first + n.sub - 1
        least[key] = min(least.get(key, n.need), n.need)
    print(f"Whole residues met again: {again:,} of {m:,} nodes; {dominated:,} outermost repeats at a need no smaller, "
          f"whose subtrees hold {saved:,} steps ({pct(saved, s.n)}).\n")
    # Splits: the frames in front of a home against those behind it.
    for prefix, what in (("r", "residues"), ("s", "states")):
        if not hasattr(s, f"{prefix}0"):
            continue
        cols = [getattr(s, f"{prefix}{f}") for f in range(len(s.frames))]
        rows = []
        for h in range(1, len(cols)):
            front = len(set(tuple(c[i] for c in cols[:h]) for i in firsts))
            back = len(set(tuple(c[i] for c in cols[h:]) for i in firsts))
            both = len(set(tuple(c[i] for c in cols) for i in firsts))
            rows.append([f"< {h} against >= {h}", f"{front:,}", f"{back:,}", f"{front * back:,}", f"{both:,}"])
        print(f"The distinct {what} of the frames in front of a split and behind it, at the nodes' first steps, "
              f"against their product and the pairs that occur:\n")
        table(rows, ["frames", "front", "behind", "product", "pairs met"])


def conflict_frames(k, home):
    """The frames every member of the conflict touches (a member of home h moves into h - 1 and out of h)."""
    common = None
    for x in k:
        h = home[x]
        mine = {h - 1, h}
        common = mine if common is None else common & mine
    return sorted(common or [])


def conflicts(s, nodes, listed, home, top):
    print("### 5. The conflicts\n")
    sizes = [len(k) for k in listed]
    q = quantiles(sizes)
    print(f"{len(listed):,} distinct conflicts for {s.n:,} steps; sizes p10/median/p90 {q[0]}/{q[1]}/{q[2]}.\n")
    # The conflict each branching step resolved: its own check's, or at a first step the check it had as a child.
    used, under = Counter(), Counter()
    child_of = {}
    for n in nodes:
        for c in n.children:
            child_of[nodes[c].first - 1] = c
    for i in range(s.n):
        if not s.br[i]:
            continue
        kid = s.kid[i] if s.kid[i] >= 0 else (s.ckid[i - 1] if i > 0 and s.inh[i] else -1)
        if kid < 0:
            continue
        used[kid] += 1
        under[kid] += nodes[child_of[i]].sub if i in child_of else 0
    by_frame = Counter()
    for kid, cnt in used.items():
        by_frame[tuple(conflict_frames(listed[kid], home))] += cnt
    total = sum(used.values())
    print("The frame of the conflicts the branching steps resolved (the frame all members touch):\n")
    table([[" and ".join(map(str, f)) or "none", f"{c:,}", pct(c, total)] for f, c in by_frame.most_common()],
          ["frame", "branching steps", "share"])
    # How the conflicts overlap: the distinct conflicts each member is in.
    member_of = defaultdict(int)
    for kid in used:
        for x in listed[kid]:
            member_of[x] += 1
    rows = []
    for h in sorted({home[x] for x in member_of}):
        counts = [c for x, c in member_of.items() if home[x] == h]
        q = quantiles(counts)
        rows.append([h, len(counts), q[1], q[2], max(counts)])
    print(f"The {len(member_of)} numbers in the {len(used):,} distinct conflicts resolved, by home, and the distinct "
          f"conflicts each is in:\n")
    table(rows, ["home", "numbers", "conflicts each, median", "p90", "most"])
    heavy = [kid for kid, _ in used.most_common(top)]
    print(f"The {top} conflicts resolved most often:\n")
    table([[kid, len(listed[kid]), f"{used[kid]:,}", " ".join(map(str, listed[kid]))] for kid in heavy],
          ["id", "size", "branching steps", "members"])


def core_prices(s, nodes, value, listed, top):
    """Whether a core costs the same wherever it is met: for the conflicts resolved most often, the bound's drop when a
    node goes on without a member, over the occurrences."""
    drops = defaultdict(list)
    for n in nodes:
        for a, b in zip(n.rows, n.rows[1:]):
            x = s.br[a]
            kid = s.kid[a] if s.kid[a] >= 0 else (s.ckid[a - 1] if a > 0 and s.inh[a] else -1)
            if x and kid >= 0 and value[a] is not None and value[b] is not None:
                drops[(kid, x)].append(value[a] - value[b])
    by_conflict = Counter()
    for (kid, _), ds in drops.items():
        by_conflict[kid] += len(ds)
    rows = []
    for kid, _ in by_conflict.most_common(top):
        x, ds = max(((x, ds) for (k, x), ds in drops.items() if k == kid), key=lambda p: len(p[1]))
        q = quantiles(ds)
        rows.append([" ".join(map(str, listed[kid])), x, f"{len(ds):,}", q[0], q[1], q[2]])
    print("Whether a core costs the same wherever it is met: for the conflicts most often gone on without a member, "
          "that member's exclusion drop over its occurrences:\n")
    table(rows, ["conflict", "member left out", "times", "drop p10", "median", "p90"])
    spread = [(quantiles(ds)[2] - quantiles(ds)[0]) / max(1, quantiles(ds)[1]) for ds in drops.values() if len(ds) >= 20]
    print(f"Over the {len(spread):,} pairs of a conflict and a member met 20 times or more, the spread of the drop "
          f"(p90 less p10) is a median {quantiles(spread)[1]:.2f} times the median drop.\n")


def root(line, home, factors):
    print("### 6. The heaviest set against the best completion\n")
    if "chosen" not in line:
        print("(no `--best-cpu` in this run)\n")
        return
    chosen, best = set(line["chosen"]), set(line["completion"])
    need = line["need"]
    print(f"The unit needs {need:,}; its bound is {line['root_value']:,} ({len(chosen)} promotions); the best "
          f"completion found is worth {line['best_completion']:,} ({len(best)} promotions), "
          f"{need - 1 - line['best_completion']:,} below what the optimum leaves it and "
          f"{line['root_value'] - line['best_completion']:,} below the bound.\n")
    homes = sorted({home[x] for x in chosen | best})
    rows = []
    for h in homes:
        a = [x for x in chosen if home[x] == h]
        b = [x for x in best if home[x] == h]
        rows.append([h, len(a), sum(a), len(b), sum(b), sum(a) - sum(b),
                     " ".join(map(str, sorted(set(a) - set(b)))), " ".join(map(str, sorted(set(b) - set(a))))])
    table(rows, ["home", "heaviest set", "its sum", "best completion", "its sum", "error",
                 "only in the heaviest set", "only in the best completion"])
    lf = {x: v for x, _, v in line.get("chosen_lf", [])}
    rows = []
    for k in line["cores"]:
        hs = Counter(home[x] for x in k)
        out = [x for x in k if x not in best]
        rows.append([len(k), ", ".join(f"{c} of home {h}" for h, c in sorted(hs.items())),
                     " ".join(f"{x}" + (f"({lf[x]})" if x in lf else "") for x in k), " ".join(map(str, out))])
    print("The heaviest set's minimal conflicts, one after the other (members with their live factors at the node, "
          "and the members the best completion leaves out):\n")
    table(rows, ["size", "homes", "members (live factors)", "left out by the best completion"])


def factor(x):
    """x as a product of primes, e.g. 1647 = 3^3·61."""
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


def is_prime(k):
    return k > 1 and all(k % d for d in range(2, int(k ** 0.5) + 1))


def twins(x, y):
    """Whether x and y share a maximal factor: x = m·p and y = m·q with p and q prime."""
    g = math.gcd(x, y)
    return is_prime(x // g) and is_prime(y // g)


def swaps(s, nodes):
    """Each time a node went on without the member x it had branched on and checked its bound again: x, what its
    heaviest set lost (x among them), what it gained, and the member the node branched on next (0 if none)."""
    for n in nodes:
        for a, b in zip(n.rows, n.rows[1:]):
            x = s.br[a]
            if not x or s.loss[b] == "-":
                continue
            lost = [int(v) for v in s.loss[b].split(",")]
            if x in lost:
                yield x, lost, [int(v) for v in s.gain[b].split(",")] if s.gain[b] != "-" else [], s.br[b]


def twin_share(found):
    """Of the members gained in the swaps, the share that is a twin of a member lost."""
    gained = [(y, lost) for _, lost, gain, _ in found for y in gain]
    return sum(1 for y, lost in gained if any(twins(x, y) for x in lost)) / max(1, len(gained))


def exchange(lost, gained):
    """Whether the swap is one exchange of two primes: the members lost and gained pair off as twins x = m·p, y = m·q
    with one pair of primes {p, q} for all (1340 = 20·67 and 1704 = 24·71 for 1420 = 20·71 and 1608 = 24·67)."""
    if len(lost) != len(gained) or not 1 <= len(lost) <= 4:
        return False
    for order in itertools.permutations(gained):
        pairs = set()
        for x, y in zip(lost, order):
            if not twins(x, y):
                break
            g = math.gcd(x, y)
            pairs.add(frozenset((x // g, y // g)))
        else:
            if len(pairs) == 1:
                return True
    return False


def kin(x, y):
    """Whether x and y share a divisor m with x / m and y / m both at most 16 (twins are kin; so are 1210 = 10·121 and
    1089 = 9·121)."""
    g = math.gcd(x, y)
    return x // g <= 16 and y // g <= 16


def substitutes(found, home, top):
    """What replaced a member in the heaviest set when a node went on without it (`swaps`)."""
    if not found:
        return
    sizes, same, pairs, twin, kins, ladder = Counter(), [0, 0], Counter(), [0, 0], [0, 0], [0, 0]
    exchanges = Counter()  # by the size of the swap: one exchange of two primes, or not
    drops = [sum(lost) - sum(gained) for _, lost, gained, _ in found]
    for x, lost, gained, after in found:
        sizes[(len(lost), len(gained))] += 1
        if len(lost) == len(gained) <= 4:
            exchanges[(len(lost), exchange(lost, gained))] += 1
        if len(lost) == 1 and len(gained) == 1:
            y = gained[0]
            same[0 if home[y] == home[x] else 1] += 1
            pairs[(x, y)] += 1
            twin[0 if twins(x, y) else 1] += 1
            kins[0 if twins(x, y) or kin(x, y) else 1] += 1
            if twins(x, y):
                ladder[0 if after == y else 1] += 1
    total = len(found)
    q = quantiles(drops)
    print(f"{total:,} exclusions with the bound checked again after them; {pct(sizes[(1, 1)], total)} swap the member "
          f"for one other candidate, of the same home in {pct(same[0], sum(same))} of those; the drop (the members lost "
          f"less those gained) p10/median/p90 {q[0]}/{q[1]}/{q[2]}.  The shapes (lost, gained) most met: "
          + ", ".join(f"{k[0]} for {k[1]} {pct(v, total)}" for k, v in sizes.most_common(5)) + ".\n")
    print(f"Of the one-for-one swaps, {pct(twin[0], sum(twin))} put in a twin of the member: a number that shares a "
          f"maximal factor with it (x = m·p, y = m·q, p and q prime), and {pct(kins[0], sum(kins))} a twin or a kin "
          f"(x = m·a, y = m·b with a and b at most 16); of all the members gained, {100 * twin_share(found):.0f}% are a twin of a "
          f"member lost.  After a twin came in, the node branched next on that twin in {pct(ladder[0], sum(ladder))} of "
          f"{sum(ladder):,} cases: one rung down the ladder of its payer's composites.  The swaps that are one exchange of two primes, by their size: "
          + ", ".join(f"{k} for {k} {pct(exchanges[(k, True)], exchanges[(k, True)] + exchanges[(k, False)])}"
                      for k in (1, 2, 3, 4))
          + f"; of all the swaps, {pct(sum(v for (k, ok), v in exchanges.items() if ok), total)}.\n")
    rows = [[f"{x} = {factor(x)}", home[x], f"{y} = {factor(y)}", home[y], x - y, f"{c:,}"]
            for (x, y), c in pairs.most_common(top)]
    print(f"The {top} one-for-one swaps met most often:\n")
    table(rows, ["left out", "home", "its substitute", "home", "difference", "times"])


def siblings(s, nodes, listed, top):
    """How far the subtrees of two children of one node resolve the same conflicts."""
    print("### Siblings\n")
    used_in = []  # the conflicts each node's own steps resolved
    for n in nodes:
        mine = set()
        for i in n.rows:
            kid = s.kid[i] if s.kid[i] >= 0 else (s.ckid[i - 1] if i > 0 and s.inh[i] else -1)
            if s.br[i] and kid >= 0:
                mine.add(frozenset(listed[kid]))
        used_in.append(mine)
    under = [set() for _ in nodes]
    for k in range(len(nodes) - 1, -1, -1):
        under[k] |= used_in[k]
        p = nodes[k].parent
        if p >= 0:
            under[p] |= under[k]
    rows = []
    heavy = sorted((k for k, n in enumerate(nodes) if len(n.children) >= 2), key=lambda k: -nodes[k].sub)
    for k in heavy:
        kids = sorted(nodes[k].children, key=lambda c: -nodes[c].sub)[:2]
        if nodes[kids[1]].sub < 1000:
            continue
        a, b = under[kids[0]], under[kids[1]]
        rows.append([nodes[k].depth, f"{nodes[kids[0]].sub:,}", f"{nodes[kids[1]].sub:,}", f"{len(a):,}",
                     f"{len(b):,}", pct(len(a & b), len(b))])
        if len(rows) == top:
            break
    print("At the heaviest nodes with two children of 1,000 steps or more: the distinct conflicts (as sets of numbers) "
          "each of the two heaviest children's subtrees resolved, and the share of the lighter one's that the heavier "
          "one resolved too:\n")
    table(rows, ["depth", "heavier child", "lighter child", "its conflicts", "the lighter's", "shared"])


def exclusion_drops(s, nodes, value):
    """How much the bound fell each time a node went on without the member it had branched on."""
    drops = []
    for n in nodes:
        for a, b in zip(n.rows, n.rows[1:]):
            if s.br[a] and value[a] is not None and value[b] is not None:
                drops.append(value[a] - value[b])
    return drops


def fit(xs, ys):
    """Least squares y = a + b x: (a, b, r squared)."""
    mx, my = statistics.fmean(xs), statistics.fmean(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    syy = sum((y - my) ** 2 for y in ys)
    b = sxy / sxx
    return my - b * mx, b, sxy * sxy / (sxx * syy)


def model(suite, trace, lines):
    """The units' sizes against their gap, and against their gap over the median drop of an exclusion in the first
    steps of their trees (the runs stopped by `--nodes`)."""
    size = {}
    with open(suite) as f:
        for line in f:
            d = json.loads(line)
            if d.get("kind") == "unit":
                size[d["id"]] = d["default"]["nodes"]
    rows, pts, by_game = [], [], defaultdict(list)
    for line in lines:
        uid = line["id"]
        s = Steps(trace / f"{uid}.steps.tsv")
        nodes, _, value = build_tree(s)
        drops = exclusion_drops(s, nodes, value)
        gap = line["root_value"] - line["need"]
        delta = quantiles(drops)[1] if drops else None
        found = list(swaps(s, nodes)) if hasattr(s, "gain") else []
        by_game[line["n"]] += found
        rows.append([uid, line["depth"], line["cands"], gap, len(drops), delta,
                     f"{gap / delta:.2f}" if delta else "-", f"{100 * twin_share(found):.0f}%", f"{size[uid]:,}"])
        if delta:
            pts.append((gap, gap / delta, math.log10(size[uid])))
    rows.sort(key=lambda r: r[3])
    table(rows, ["unit", "depth", "candidates", "gap G", "exclusions seen", "median drop d", "G / d",
                 "substitutes that are twins", "subtree (the suite's)"])
    for name, k in (("G", 0), ("G / d", 1)):
        a, b, r2 = fit([p[k] for p in pts], [p[2] for p in pts])
        print(f"log10(subtree) = {a:.2f} + {b:.4f} {name}: r squared {r2:.2f} over {len(pts)} units.\n")
    for game, found in sorted(by_game.items()):
        print(f"The substitutes in the first steps of the units of {game}: ", end="")
        substitutes(found, read_candidates(trace / f"{game}.candidates.tsv")[0], 8)


def orders(lines):
    """The runs of each unit with each branching order."""
    rows = []
    for line in lines:
        order = Path(line["prefer"]).name if line.get("prefer") else line.get("tactics", "strong branching")
        rows.append([line["id"], order, f"{line['nodes']:,}", "yes" if line["completed"] else "no (stopped)",
                     f"{line['cpu_s']:.0f}", f"{line['overridden']:,} of {line['asked']:,}"])
    table(rows, ["unit", "order", "steps", "completed", "CPU s", "choices changed"])


def main():
    if sys.argv[1] == "--model":
        suite, trace = sys.argv[2], Path(sys.argv[3])
        lines = [json.loads(line) for path in sys.argv[4:] for line in open(path) if line.strip()]
        return model(suite, trace, lines)
    if sys.argv[1] == "--orders":
        return orders([json.loads(line) for path in sys.argv[2:] for line in open(path) if line.strip()])
    trace = Path(sys.argv[1])
    lines = []
    for path in sys.argv[2:]:
        with open(path) as f:
            lines += [json.loads(line) for line in f if line.strip()]
    for line in lines:
        uid = line["id"]
        steps = trace / f"{uid}.steps.tsv"
        print(f"## {uid}: depth {line['depth']}, {line['cands']} candidates, need {line['need']:,}, "
              f"{line['nodes']:,} steps, {line['cpu_s']:.0f} CPU s\n")
        home, factors = read_candidates(trace / f"{line['n']}.candidates.tsv")
        if steps.exists():
            s = Steps(steps)
            nodes, node_of, value = build_tree(s)
            shape(s, nodes)
            branches(s, nodes, node_of, home, 15)
            top_levels(s, nodes, home)
            gaps(s, nodes, node_of, value, home)
            repetition(s, nodes)
            listed = read_conflicts(trace / f"{uid}.conflicts.tsv")
            conflicts(s, nodes, listed, home, 10)
            core_prices(s, nodes, value, listed, 10)
            siblings(s, nodes, listed, 8)
            if hasattr(s, "gain"):
                print("### Substitutes\n")
                substitutes(list(swaps(s, nodes)), home, 12)
        root(line, home, factors)


if __name__ == "__main__":
    main()
