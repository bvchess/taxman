#!/usr/bin/env python3
"""Sums GAPPROF lines (bucket:steps:ns; bucket 0 a step with no bound reached, bucket b >= 1 a gap of 100(b-1) to
100b - 1, the last 3000 and more) by setting over a folder: steps, time and ms a step by gap band.
Usage: gapsum.py DIR [GLOB]"""
import collections
import glob
import os
import sys

pattern = sys.argv[2] if len(sys.argv) > 2 else '*'
agg = collections.defaultdict(lambda: collections.defaultdict(lambda: [0, 0]))
for f in glob.glob(os.path.join(sys.argv[1], pattern + '.err')):
    tag = os.path.basename(f).split('-')[0]
    for line in open(f):
        if line.startswith('GAPPROF'):
            for part in line.split()[1:]:
                b, n, t = (int(x) for x in part.split(':'))
                agg[tag][b][0] += n
                agg[tag][b][1] += t
bands = [(0, 0, 'no bound'), (1, 3, '0-299'), (4, 6, '300-599'), (7, 9, '600-899'), (10, 12, '900-1199'),
         (13, 31, '1200+')]
for tag in sorted(agg):
    a = agg[tag]
    ts = sum(v[0] for v in a.values())
    tt = sum(v[1] for v in a.values())
    print('%-8s steps %8d  time %7.1f s  %.3f ms/step' % (tag, ts, tt / 1e9, tt / ts / 1e6))
    for lo, hi, name in bands:
        n = sum(a[b][0] for b in range(lo, hi + 1))
        t = sum(a[b][1] for b in range(lo, hi + 1))
        if n:
            print('   %-9s steps %8d (%5.1f%%)  time %6.1f s (%5.1f%%)  %.3f ms/step' % (
                name, n, 100 * n / ts, t / 1e9, 100 * t / tt, t / n / 1e6))
