#!/usr/bin/env python3
"""The anatomy's first hypothesis (count the prices instead of searching them), tested on traced units: is a tree the
count of the combinations of exclusions, at fixed prices, that fit under its gap?

usage: python3 -I prices.py TRACE_DIR ID...

TRACE_DIR holds `ID.steps.tsv` (`wall anatomy --trace`; the fields are in solver/frame/examples/wall/anatomy.rs).  The
trace is read with the anatomy's own script (../the-anatomy-of-a-hard-subtree-2026-10/analyze.py).

A node's loop makes its children one after the other, and the node goes on without each member it branched on; the
price of that exclusion is the bound before it less the bound at the node's next step.  The *fixed* price of an
exclusion is the median price of its pair of conflict and member over the tree, as the anatomy measured them.  The
*menu* of a node is what reaching each of its children costs: the sum of the prices of the exclusions its loop made
before that child.  If the prices were fixed and every path met the same menus, the tree's leaves would be the
combinations, one child chosen at each node of any path, whose costs add up to no more than the unit's gap; so each
path's own menus would count the leaves.  For a sample of the leaves, the script counts those combinations along the
leaf's path, at the fixed prices and at the prices the tree met, and compares them with the leaves.  Standard library
only.
"""
import random
import statistics
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "the-anatomy-of-a-hard-subtree-2026-10"))
from analyze import Steps, build_tree, quantiles  # noqa: E402

SAMPLE = 400  # leaves whose paths are counted
SEED = 1


def conflict_of(s, a):
    """The conflict a step branched on: its own check's, or at a node's first step the one it had as a child."""
    if s.kid[a] >= 0:
        return s.kid[a]
    return s.ckid[a - 1] if a > 0 and s.inh[a] else -1


def exclusions(s, nodes, value):
    """Each exclusion with a price: (node, its row, the next row of the node, the price, the pair of conflict and
    member)."""
    for k, n in enumerate(nodes):
        for a, b in zip(n.rows, n.rows[1:]):
            if s.br[a] and value[a] is not None and value[b] is not None:
                yield k, a, b, value[a] - value[b], (conflict_of(s, a), s.br[a])


def menus(s, nodes, actual, fixed):
    """For each node, its children with what reaching each costs: (child, actual cost, fixed cost)."""
    out = []
    for n in nodes:
        made = {nodes[c].first - 1: c for c in n.children}
        cost_a = cost_f = 0
        menu = []
        for a in n.rows:
            if a in made:
                menu.append((made[a], cost_a, cost_f))
            if s.br[a] and a in actual:
                cost_a += actual[a]
                cost_f += fixed[a]
        out.append(menu)
    return out


def count(path_menus, gap, which):
    """The combinations of one child a node along the path whose costs (`which`: 1 actual, 2 fixed) add up to no more
    than `gap`."""
    ways = {0: 1}
    for menu in path_menus:
        new = defaultdict(int)
        for c, k in ways.items():
            for entry in menu:
                total = c + max(0, entry[which])
                if total <= gap:
                    new[total] += k
        ways = new
    return sum(ways.values())


def unit(trace, uid):
    s = Steps(trace / f"{uid}.steps.tsv")
    nodes, _, value = build_tree(s)
    gap = nodes[0].gap0
    by_pair, actual = defaultdict(list), {}
    for _, a, _, price, pair in exclusions(s, nodes, value):
        by_pair[pair].append(price)
        actual[a] = price
    median = {pair: statistics.median(ps) for pair, ps in by_pair.items()}
    fixed = {}
    for _, a, _, _, pair in exclusions(s, nodes, value):
        fixed[a] = median[pair]
    menu = menus(s, nodes, actual, fixed)
    leaves = [k for k, n in enumerate(nodes) if not n.children]
    rng = random.Random(SEED)
    sample = rng.sample(leaves, min(SAMPLE, len(leaves)))
    counts = {1: [], 2: []}
    depths = []
    for leaf in sample:
        path, k = [], leaf
        while nodes[k].parent >= 0:
            path.append(nodes[k].parent)
            k = nodes[k].parent
        path.reverse()
        depths.append(len(path))
        path_menus = [menu[k] for k in path]
        for which in (1, 2):
            counts[which].append(count(path_menus, gap, which))
    spread = [(quantiles(ps)[2] - quantiles(ps)[0]) / max(1, abs(statistics.median(ps)))
              for ps in by_pair.values() if len(ps) >= 20]
    print(f"## {uid}\n")
    print(f"{s.n:,} steps, {len(nodes):,} nodes, {len(leaves):,} leaves; gap {gap}; {len(actual):,} exclusions priced, "
          f"{len(by_pair):,} pairs of conflict and member; the spread (p90 less p10) over the median price of the "
          f"pairs met 20 times or more: median {statistics.median(spread):.2f}.  Paths to {len(sample)} leaves, "
          f"{statistics.median(depths)} nodes deep (median).\n")
    print("| prices | combinations a path, p10 | median | p90 | median over the leaves |")
    print("|---|---|---|---|---|")
    for which, name in ((2, "fixed"), (1, "as met")):
        q = quantiles(counts[which])
        print(f"| {name} | {q[0]:,} | {q[1]:,} | {q[2]:,} | {q[1] / len(leaves):.3g} |")
    print()


def main():
    trace = Path(sys.argv[1])
    for uid in sys.argv[2:]:
        unit(trace, uid)


if __name__ == "__main__":
    main()
