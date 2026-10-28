#!/usr/bin/env python3
"""Summarizes CB trace lines of .err files: decisions, by gap band, how often the node's weights and the context
weights pick another conflict than the search's own.  Usage: tsum.py FILE..."""
import collections
import re
import sys

pat = re.compile(r'CB d=(\d+) gap=(-?\d+) k0=\[([^\]]*)\] w0=(\S+) c0=(\S+) nb=\[([^\]]*)\] wb=(\S+) cb=(\S+) pick=(\d) '
                 r'look=(\d)')
for f in sys.argv[1:]:
    n = 0
    bands = collections.Counter()
    node_switch = collections.Counter()
    pick = collections.Counter()
    looks = 0
    sizes = collections.Counter()
    for line in open(f, errors='replace'):
        m = pat.match(line)
        if not m:
            continue
        n += 1
        gap = int(m.group(2))
        band = gap // 200 * 200
        bands[band] += 1
        k0 = m.group(3)
        nb = m.group(6)
        if k0 != nb:
            node_switch[band] += 1
        pick[(band, int(m.group(9)))] += 1
        looks += int(m.group(10))
        sizes[(len(k0.split(',')), len(nb.split(',')))] += 1
    print(f, 'decisions', n, 'looks', looks)
    for b in sorted(bands):
        print('  gap %4d: %6d decisions, node weights switch %6d, picks own %6d node-best %6d other %6d' % (
            b, bands[b], node_switch[b], pick[(b, 0)], pick[(b, 1)], pick[(b, 2)]))
    print('  sizes (k0, best):', sizes.most_common(8))
