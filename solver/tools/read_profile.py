#!/usr/bin/env python3
"""Reads the profile of a search (`taxman solve --profile DIR` writes DIR/nNNNN.profile.jsonl) and prints where its
steps went.  Standard library only.  solver/docs/design/engine.md, "The profile of a search", describes the file.

usage: read_profile.py FILE [--depths D,D,...] [--top K]

It prints:

  * what the file covers: the game, the steps, the ids made, the depth of the tree, the runs;
  * the depth histogram: for each depth of the tagged tree, its nodes, the steps they took themselves and the steps of
    their subtrees (at the last depth, a node's own steps are its whole subtree);
  * the largest subtrees at each of the depths shown, K of them (default 12), with what was known when each was made
    (how its parent branched, its promotions, candidates, bound, need and gap, the gap at the final best, when it was
    made), and how much of the game the largest k of the depth hold;
  * how a subtree's size follows its gap at those depths: the decades of steps for each 100 of gap (a least-squares
    slope of log10 of the size), and the rank correlation;
  * the timeline at the first depth shown: each checkpoint interval's steps in its largest subtrees;
  * the steps by number of promotions, and the open units at each checkpoint (their number in the largest subtrees
    of the first depth shown, and the spread of their gaps).

The top of a search tree is often a spine, each node making one or two children of which one holds nearly all the
steps, so the depths shown by default are those where the tree branches: the first depth whose largest subtree holds
less than 90% of the steps, the last depth of the profile, and the depth halfway between.  `--depths` names others.

A node's subtree is its own steps and its children's subtrees, summed here; the steps the profile could not place
(a search it joined after it began) are counted apart.
"""
import argparse
import json
import math
import sys
from collections import defaultdict


def read(path):
    """The file's lines, as dicts; a line cut short at the end (a crash) is left out."""
    lines = []
    with open(path) as f:
        for text in f:
            try:
                lines.append(json.loads(text))
            except json.JSONDecodeError:
                break
    return lines


class Profile:
    def __init__(self, lines):
        self.header = next((l for l in lines if l['kind'] == 'profile'), {})
        self.nodes = {l['id']: l for l in lines if l['kind'] == 'node'}
        self.starts = [l for l in lines if l['kind'] == 'start']
        self.steps = [l for l in lines if l['kind'] == 'steps']
        self.opens = [l for l in lines if l['kind'] == 'open']
        self.ends = [l for l in lines if l['kind'] == 'end']
        self.children = defaultdict(list)
        for node in self.nodes.values():
            if node['parent'] is not None:
                self.children[node['parent']].append(node['id'])
        self.own = defaultdict(int)
        self.unplaced = sum(s['unplaced'] for s in self.starts) + sum(s['unplaced'] for s in self.steps)
        self.promotions = defaultdict(int)
        for line in self.steps:
            for node, count in line['ids']:
                self.own[node] += count
            for k, count in line['promotions']:
                self.promotions[k] += count
        self.subtree = {}
        for node in sorted(self.nodes, reverse=True):  # a child's id is larger than its parent's
            self.subtree[node] = self.own.get(node, 0) + sum(self.subtree[c] for c in self.children[node])
        self.total = sum(self.own.values()) + self.unplaced
        self.best = self.ends[-1]['best'] if self.ends else None
        self.deepest = max((line['depth'] for line in self.nodes.values()), default=0)

    def at(self, depth):
        """The nodes at `depth`, the largest subtree first."""
        return sorted((n for n, line in self.nodes.items() if line['depth'] == depth),
                      key=lambda n: -self.subtree[n])

    def ancestors(self, depth):
        """Each node's ancestor at `depth` (itself at that depth; none above it)."""
        ancestor = {}
        for n in sorted(self.nodes):
            line = self.nodes[n]
            if line['depth'] == depth:
                ancestor[n] = n
            elif line['depth'] > depth:
                ancestor[n] = ancestor.get(line['parent'])
        return ancestor

    def final_gap(self, node):
        line = self.nodes[node]
        if self.best is None or line['bound'] is None:
            return None
        return line['bound'] - (self.best + 1 - line['sum'])

    def branching_depths(self):
        """The first depth whose largest subtree holds less than 90% of the steps, the deepest, and halfway."""
        first = next((d for d in range(self.deepest + 1)
                      if self.at(d) and self.subtree[self.at(d)[0]] < 0.9 * self.total), self.deepest)
        return sorted({first, (first + self.deepest) // 2, self.deepest})


def si(x):
    """`x` in a short form: 1234567 -> 1.23M."""
    for unit, scale in (('G', 1e9), ('M', 1e6), ('k', 1e3)):
        if abs(x) >= scale:
            return f'{x / scale:.3g}{unit}'
    return f'{x:.0f}' if isinstance(x, float) else str(x)


def percent(part, whole):
    return f'{100 * part / whole:.1f}%' if whole else '-'


def summary(p):
    h = p.header
    full = ', all made' if any(s.get('full') for s in p.steps) else ''
    print(f"game {h.get('n')}: {si(p.total)} steps counted, {si(p.ends[-1]['nodes']) if p.ends else '?'} in the "
          f"search; {len(p.nodes)} ids at depth <= {h.get('depth')} (at most {h.get('most_ids')}{full}); "
          f"{len(p.starts)} run(s); {si(p.unplaced)} steps unplaced ({percent(p.unplaced, p.total)})")
    if p.ends:
        end = p.ends[-1]
        print(f"  last run: completed {end['completed']}, best {end['best']}, {end['elapsed']:.0f} s of search")
    if any(not s['rooted'] for s in p.starts):
        print('  the profile joined the search after it began: its steps before then, and the units it found, are '
              'unplaced')


def depth_histogram(p):
    print('\ndepth histogram (own: steps at the nodes themselves; at the last depth, their whole subtrees; largest: '
          'the share of the largest subtree):')
    print(f"  {'depth':>5} {'ids':>7} {'own steps':>10} {'share':>7} {'subtrees':>10} {'share':>7} {'largest':>8}")
    for d in range(p.deepest + 1):
        nodes = p.at(d)
        own = sum(p.own.get(n, 0) for n in nodes)
        below = sum(p.subtree[n] for n in nodes)
        top = p.subtree[nodes[0]] if nodes else 0
        print(f'  {d:>5} {len(nodes):>7} {si(own):>10} {percent(own, p.total):>7} {si(below):>10} '
              f'{percent(below, p.total):>7} {percent(top, p.total):>8}')


def features(p, node):
    line = p.nodes[node]
    final = p.final_gap(node)
    dash = lambda x: '-' if x is None else x
    return (f"{dash(line['branch']):>8} {dash(line['child']):>5} {line['promotions']:>5} {line['cands']:>6} "
            f"{dash(line['bound']):>8} {line['need']:>8} {dash(line['gap']):>6} {dash(final):>6} {line['t']:>8.1f}")


def largest(p, depths, top):
    head = (f"  {'id':>7} {'parent':>7} {'steps':>9} {'game':>6} {'parent':>6} {'branch':>8} {'child':>5} "
            f"{'prom':>5} {'cands':>6} {'bound':>8} {'need':>8} {'gap':>6} {'gap*':>6} {'made s':>8}")
    for d in depths:
        nodes = p.at(d)
        if not nodes:
            continue
        print(f'\nthe largest subtrees at depth {d} ({len(nodes)} nodes; game: share of the steps; parent: share of '
              f"the parent's subtree; gap*: the gap at the final best):")
        print(head)
        for n in nodes[:top]:
            line = p.nodes[n]
            size, parent = p.subtree[n], p.subtree.get(line['parent'], 0)
            print(f"  {n:>7} {line['parent']:>7} {si(size):>9} {percent(size, p.total):>6} "
                  f'{percent(size, parent):>6} {features(p, n)}')
        sizes = [p.subtree[n] for n in nodes]
        held = [f'top {k}: {percent(sum(sizes[:k]), p.total)}' for k in (1, 2, 5, 10, 20, 50, 100, 1000)
                if k <= len(sizes)]
        print(f"  held by the largest: {', '.join(held)}")


def fit(points):
    """The least-squares slope of y on x, and the rank correlation (Spearman) of the points, or None."""
    if len(points) < 3:
        return None
    xs, ys = [x for x, _ in points], [y for _, y in points]
    mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    if sxx == 0:
        return None
    slope = sum((x - mx) * (y - my) for x, y in points) / sxx

    def ranks(values):
        order = sorted(range(len(values)), key=lambda i: values[i])
        r = [0.0] * len(values)
        i = 0
        while i < len(order):
            j = i
            while j + 1 < len(order) and values[order[j + 1]] == values[order[i]]:
                j += 1
            for k in range(i, j + 1):
                r[order[k]] = (i + j) / 2
            i = j + 1
        return r

    rx, ry = ranks(xs), ranks(ys)
    mrx, mry = sum(rx) / len(rx), sum(ry) / len(ry)
    num = sum((a - mrx) * (b - mry) for a, b in zip(rx, ry))
    den = math.sqrt(sum((a - mrx) ** 2 for a in rx) * sum((b - mry) ** 2 for b in ry))
    return slope, (num / den if den else None)


def size_and_gap(p, depths):
    print('\nsize against gap (the gap when the node was made, and at the final best; the nodes with steps):')
    for d in depths:
        nodes = [n for n in p.at(d) if p.subtree[n] > 0 and p.nodes[n]['gap'] is not None]
        cells = []
        for name, gap in (('made', lambda n: p.nodes[n]['gap']), ('final', p.final_gap)):
            got = fit([(gap(n), math.log10(p.subtree[n])) for n in nodes if gap(n) is not None])
            if got:
                slope, rho = got
                rank = f', rank correlation {rho:+.2f}' if rho is not None else ''
                cells.append(f'{name}: {100 * slope:+.2f} decades per 100{rank}')
        print(f"  depth {d} ({len(nodes)} nodes): {'; '.join(cells) if cells else '-'}")


def timeline(p, depth, top):
    if not p.steps:
        return
    nodes = p.at(depth)[:top]
    ancestor = p.ancestors(depth)
    shown = set(nodes)
    extra = ['rest', 'above'] + (['unplaced'] if p.unplaced else [])
    print(f'\ntimeline at depth {depth}: the steps of each checkpoint interval in the largest subtrees '
          f'(rest: the other subtrees; above: the steps of the nodes above depth {depth}):')
    print(f"  {'seq':>4} {'elapsed':>8} {'steps':>8} " + ' '.join(f'{n:>7}' for n in nodes + extra))
    for line in p.steps:
        cells = defaultdict(int)
        for n, count in line['ids']:
            a = ancestor.get(n)
            cells['above' if a is None else (a if a in shown else 'rest')] += count
        cells['unplaced'] += line['unplaced']
        total = sum(cells.values())
        if total == 0:
            continue
        print(f"  {line['seq']:>4} {line['elapsed']:>8.0f} {si(total):>8} "
              + ' '.join(f'{si(cells[n]):>7}' for n in nodes + extra))


def by_promotions(p):
    if not p.promotions:
        return
    counted = sum(p.promotions.values())
    most = max(p.promotions)
    width = max(1, math.ceil((most + 1) / 25))
    print(f'\nsteps by number of promotions of the node (in buckets of {width}):')
    for lo in range(0, most + 1, width):
        steps = sum(p.promotions.get(k, 0) for k in range(lo, lo + width))
        if steps:
            bar = '#' * round(50 * steps / counted)
            print(f'  {lo:>4}-{lo + width - 1:<4} {si(steps):>8} {percent(steps, counted):>6} {bar}')


def open_work(p, depth, top):
    if not p.opens:
        return
    print(f'\nthe open units at each checkpoint (in the largest subtrees at depth {depth}; above: units of nodes above '
          'it; the gaps of a sample: min, p10, p25, p50, p75, p90, max; s: the seconds the summary took):')
    nodes = p.at(depth)[:top]
    ancestor = p.ancestors(depth)
    shown = set(nodes)
    print(f"  {'seq':>4} {'nodes':>8} {'units':>6} " + ' '.join(f'{n:>6}' for n in nodes)
          + f" {'rest':>6} {'above':>6} {'s':>6} gaps")
    for line in p.opens:
        cells = defaultdict(int)
        for node, units, _, _ in line['ids']:
            a = ancestor.get(node) if node is not None else None
            cells['above' if a is None else (a if a in shown else 'rest')] += units
        gaps = ' '.join(str(g) for g in line['gaps']) if line['gaps'] else '-'
        print(f"  {line['seq']:>4} {si(line['nodes']):>8} {line['units']:>6} "
              + ' '.join(f'{cells[n]:>6}' for n in nodes)
              + f" {cells['rest']:>6} {cells['above']:>6} {line['seconds']:>6.1f} {gaps}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    parser.add_argument('file')
    parser.add_argument('--depths', help='the depths shown, as D,D,... (default: where the tree branches)')
    parser.add_argument('--top', type=int, default=12, help='subtrees shown at a depth (default 12)')
    args = parser.parse_args()
    p = Profile(read(args.file))
    if not p.header:
        sys.exit(f'{args.file}: not a profile')
    depths = [int(d) for d in args.depths.split(',')] if args.depths else p.branching_depths()
    summary(p)
    depth_histogram(p)
    largest(p, depths, args.top)
    size_and_gap(p, depths)
    timeline(p, depths[0], min(args.top, 8))
    by_promotions(p)
    open_work(p, depths[0], min(args.top, 8))


if __name__ == '__main__':
    main()
