#!/usr/bin/env python3
"""Compares settings on the units they share: cmp.py REF OTHER[,OTHER...] FILE... (FILE: collect.py output).  Prints
per unit steps and CPU, and totals over units completed under every setting compared, and over all units."""
import collections
import sys

ref = sys.argv[1]
others = sys.argv[2].split(',')
quiet = '-q' in sys.argv
d = collections.defaultdict(dict)
for f in sys.argv[3:]:
    if f == '-q':
        continue
    for line in open(f):
        r = line.split()
        d[r[2]][r[1]] = (int(r[3]), int(r[4]), float(r[5]), int(r[6]))
names = [ref] + others
tot = collections.Counter()
totall = collections.Counter()
games = collections.defaultdict(collections.Counter)
for u in sorted(d, key=lambda u: (u.split('-')[0], d[u].get(ref, (0,))[0])):
    if not all(n in d[u] for n in names):
        continue
    r = d[u][ref]
    line = '%-11s %4d %s %7d %6.1f' % (u, r[0], ' ' if r[3] else '*', r[1], r[2])
    for n in others:
        o = d[u][n]
        line += ' | %s %7d %6.1f %s %5.2f %5.2f' % (n[:10], o[1], o[2], ' ' if o[3] else '*', o[1] / r[1],
                                                   o[2] / max(r[2], 1e-3))
    if not quiet:
        print(line)
    done = all(d[u][n][3] for n in names)
    g = u.split('-')[0]
    for n in names:
        totall[(n, 's')] += d[u][n][1]
        totall[(n, 'c')] += d[u][n][2]
        games[g][(n, 's')] += d[u][n][1]
        games[g][(n, 'c')] += d[u][n][2]
        if done:
            tot[(n, 's')] += d[u][n][1]
            tot[(n, 'c')] += d[u][n][2]
            games[g][(n, 'sd')] += d[u][n][1]
            games[g][(n, 'cd')] += d[u][n][2]
for label, t in (('completed in all', tot), ('all units', totall)):
    print(label, ' '.join('%s steps %.3f cpu %.3f' % (n, t[(n, 's')] / max(1, t[(ref, 's')]),
                                                      t[(n, 'c')] / max(1e-9, t[(ref, 'c')])) for n in others),
          '(ref steps %d cpu %.0f)' % (t[(ref, 's')], t[(ref, 'c')]))
for g in sorted(games):
    t = games[g]
    print(' game', g, ' '.join('%s done: steps %.3f cpu %.3f, all: steps %.3f cpu %.3f' % (
        n, t[(n, 'sd')] / max(1, t[(ref, 'sd')]), t[(n, 'cd')] / max(1e-9, t[(ref, 'cd')]),
        t[(n, 's')] / max(1, t[(ref, 's')]), t[(n, 'c')] / max(1e-9, t[(ref, 'c')])) for n in others))
