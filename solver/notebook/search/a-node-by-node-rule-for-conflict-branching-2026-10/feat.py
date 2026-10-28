#!/usr/bin/env python3
"""Joins the first decisions of each unit's trace (out-trace4/tr-ID.err) with the outcome off against gate 1200 (from
result tables): feat.py RESULTS..."""
import collections
import glob
import math
import os
import re
import sys

pat = re.compile(r'CB d=(\d+) gap=(-?\d+) k0=\[([^\]]*)\] w0=(\S+) c0=(\S+) nb=\[([^\]]*)\] wb=(\S+) cb=(\S+) pick=(\d) '
                 r'look=(\d)')
d = collections.defaultdict(dict)
for f in sys.argv[1:]:
    for line in open(f):
        r = line.split()
        d[r[2]][r[1]] = (int(r[3]), int(r[4]), float(r[5]), int(r[6]))
rows = []
for f in sorted(glob.glob('/home/brian/taxman_work/lab/work/branch-2026-10-27/out-trace4/tr-*.err')):
    u = os.path.basename(f)[3:-4]
    if 'off' not in d[u] or 'g1200' not in d[u]:
        continue
    o, g = d[u]['off'], d[u]['g1200']
    if not (o[3] or g[3]):
        continue
    dec = [pat.match(l) for l in open(f)]
    dec = [m for m in dec if m]
    if not dec:
        continue
    m = dec[0]
    k0 = [int(x) for x in m.group(3).split(',')]
    nb = [int(x) for x in m.group(6).split(',')]
    switches = sum(1 for x in dec if x.group(9) != '0')
    looks = sum(1 for x in dec if x.group(10) == '1')
    out = math.log10(o[1] / g[1])
    rows.append((out, u, o[0], int(m.group(2)), len(k0), min(k0), float(m.group(4)), len(nb), min(nb), float(m.group(7)),
                 m.group(9), m.group(10), len(dec), switches, looks, o[1], g[1], o[3], g[3]))
rows.sort()
print('log10(off/g1200) unit gap gap0 |k0| min0 w0 |nb| minb wb pick look decisions switches looks off g1200')
for r in rows:
    print('%6.2f %-10s %4d %4d %2d %3d %5.2f %2d %3d %5.2f %s %s %3d %3d %3d %7d%s %7d%s' % (
        r[:15] + (r[15], '' if r[17] else '*', r[16], '' if r[18] else '*')))
