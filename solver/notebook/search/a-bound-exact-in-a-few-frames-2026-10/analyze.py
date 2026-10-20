#!/usr/bin/env python3
"""The tables of the entry from the lines of `wall local`: python3 -I analyze.py local.jsonl.gz (or plain files).

A node's gap is its GF(2) bound less what its completions must add to reach the optimum (`need` - 1).  The share of
it that a hybrid bound closes is (bound - max(hybrid, need - 1)) / gap: 1 when the hybrid bound cuts the node.  A
hybrid search stopped at its step limit gives a range: its dual bound from above, its best from below.
"""
import gzip
import json
import statistics
import sys


def load(paths):
    units, nodes, compares = [], [], []
    for path in paths:
        for line in (gzip.open(path, 'rt') if path.endswith('.gz') else open(path)):
            line = line.strip()
            if not line.startswith('{'):
                continue
            d = json.loads(line)
            {'unit': units, 'node': nodes, 'compare': compares}[d['kind']].append(d)
    return units, nodes, compares


def gap(node):
    return node['bound'] - node['need'] + 1


def closed(node, h):
    """The share of the node's gap that the hybrid bound `h` closes: (low, high), equal unless the search stopped."""
    _, best, completed, _, dual, _, _ = h
    floor = node['need'] - 1
    if completed:
        s = (node['bound'] - max(best, floor)) / gap(node)
        return (s, s)
    top = dual if dual is not None else node['bound']
    return ((node['bound'] - max(top, floor)) / gap(node), (node['bound'] - max(best, floor)) / gap(node))


def share(s):
    if s is None:
        return '-'
    lo, hi = s
    return f'{lo:.2f}' if lo == hi else f'{lo:.2f}-{hi:.2f}'


def hybrids(node):
    return {h[0]: h for h in node['hybrid']}


def frames_table(units):
    print('| unit | steps | conflicts by frame (1 / 2 / 3 / 4) | exclusions by frame (1 / 2 / 3 / 4) |')
    print('|---|---|---|---|')
    for u in units:
        c, e = u['conflicts'], u['exclusions']
        pc = ' / '.join(f'{x / sum(c):.2f}' for x in c[1:5])
        pe = ' / '.join(f'{x / sum(e):.2f}' for x in e[1:5])
        print(f"| {u['id']} | {u['steps']:,} | {pc} | {pe} |")


def recurrence_table(units):
    sets = []
    for u in units:
        sets += [s for s in u['sets'] if s not in ('none', 'all') and s not in sets]
    print('| unit | node checks | ' + ' | '.join(f'repeats, frames {s}' for s in sets) + ' |')
    print('|---|---|' + '---|' * len(sets))
    for u in units:
        repeats = dict(zip(u['sets'], u['repeats']))
        cells = [f"{repeats[s] / u['checks']:.2f}" if s in repeats else '-' for s in sets]
        print(f"| {u['id']} | {u['checks']:,} | " + ' | '.join(cells) + ' |')


def nodes_table(nodes, sets):
    print('| node | subtree | gap | ' + ' | '.join(f'frames {s}' for s in sets) + ' | the proof\'s steps |')
    print('|---|---|---|' + '---|' * len(sets) + '---|')
    for n in sorted(nodes, key=lambda n: (n['id'], -n['subtree'])):
        hs = hybrids(n)
        proof = hs['all'][3] if 'all' in hs else None
        cells = []
        for s in sets:
            h = hs.get(s)
            if h is None:
                cells.append('-')
                continue
            cost = f'{h[3] / proof:.2f}' if proof else '-'
            cells.append(f'{share(closed(n, h))} ({cost})')
        stopped = '' if hs['all'][2] else 'stopped at '
        print(f"| {n['id']} #{n['node']} | {n['subtree']:,} | {gap(n)} | " + ' | '.join(cells)
              + f' | {stopped}{proof:,} |')


def summary(nodes, sets):
    print('| band of gap | nodes | ' + ' | '.join(f'frames {s}: cut; median share; median cost' for s in sets) + ' |')
    print('|---|---|' + '---|' * len(sets))
    bands = [(0, 400), (400, 700), (700, 1000), (1000, 2000)]
    for lo, hi in bands:
        group = [n for n in nodes if lo <= gap(n) < hi]
        if not group:
            continue
        cells = []
        for s in sets:
            rows = [(closed(n, hybrids(n)[s]), hybrids(n)[s][3] / hybrids(n)['all'][3]) for n in group
                    if s in hybrids(n) and 'all' in hybrids(n)]
            cut = sum(1 for c, _ in rows if c[0] >= 1)
            med = statistics.median(c[0] for c, _ in rows)
            cost = statistics.median(r for _, r in rows)
            cells.append(f'{cut} of {len(rows)}; {med:.2f}; {cost:.2f}')
        print(f'| {lo}-{hi} | {len(group)} | ' + ' | '.join(cells) + ' |')


def under(nodes):
    """At the nodes with gaps under 1,000: how many the bound exact in frame 2 cuts and its steps over the proof's
    where it does, and the shares that frames 1 and 3 alone close (median and quartiles)."""
    low = [n for n in nodes if gap(n) < 1000]
    costs = [hybrids(n)['2'][3] / hybrids(n)['all'][3] for n in low if closed(n, hybrids(n)['2'])[0] >= 1]
    quartiles = lambda xs: ', '.join(f'{q:.2f}' for q in statistics.quantiles(xs, n=4))
    print(f'{len(low)} nodes; frame 2 cuts {len(costs)}, at {quartiles(costs)} times the proof\'s steps (quartiles)')
    for s in ['1', '3']:
        shares = [closed(n, hybrids(n)[s])[0] for n in low if hybrids(n)[s][2]]
        print(f'frame {s} alone closes {quartiles(shares)} of the gap (quartiles)')


def homes_table(units):
    rows = [u for u in units if u.get('opt_homes')]
    if not rows:
        return
    print('| root | gap | the heaviest set less the optimum, by home 1, 2, 3, ... |')
    print('|---|---|---|')
    for u in rows:
        oh = u['opt_homes']
        ex = [h - (oh[i] if i < len(oh) else 0) for i, h in enumerate(u['homes'])][1:]
        print(f"| {u['id']} | {u['bound'] - u['need'] + 1} | {', '.join(f'{x:+d}' for x in ex)} |")


def compare_table(compares):
    same = [c for c in compares if all(f['moves'][0] + f['moves'][1] + f['factors'][0] + f['factors'][1] == 0
                                       for f in c['frames'])]
    print(f'{len(same)} of {len(compares)} games have the frames of the next game, number for number.')
    print()
    print('| game | numbers that differ in frames 0, 1, 2, ... (moves and factors) |')
    print('|---|---|')
    for c in compares:
        diff = [f['moves'][0] + f['moves'][1] + f['factors'][0] + f['factors'][1] for f in c['frames']]
        if any(diff):
            print(f"| {c['n']} -> {c['next']} | {', '.join(map(str, diff))} |")


def main():
    units, nodes, compares = load(sys.argv[1:])
    sets = [s for s in ['1', '2', '3', '2,3', '1,2,3'] if any(s in hybrids(n) for n in nodes)]
    if units:
        print('## Where the units branch\n')
        frames_table(units)
        print('\n## How often a node check finds a local situation an earlier check had\n')
        recurrence_table(units)
        print('\n## The error by home at the roots that hold the optimum\n')
        homes_table(units)
    if nodes:
        print('\n## The share of the gap closed by the hybrid bound (its steps against the proof\'s)\n')
        nodes_table(nodes, sets)
        print('\n## By band of gap\n')
        summary(nodes, sets)
        print('\n## Under a gap of 1,000\n')
        under(nodes)
    if compares:
        print('\n## From one game to the next\n')
        compare_table(compares)
    cpu = sum(u['cpu_s'] for u in units) + sum(h[5] for n in nodes for h in n['hybrid'])
    print(f'\nCPU of the searches: {cpu:.0f} s')


main()
