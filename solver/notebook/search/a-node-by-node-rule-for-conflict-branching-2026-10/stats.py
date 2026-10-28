#!/usr/bin/env python3
"""Step statistics of settings against a reference on units completed under both: total ratio, geometric mean, units
better and worse, and a bootstrap interval of the total ratio.  Usage: stats.py REF A,B,... FILE..."""
import collections
import math
import random
import sys

ref = sys.argv[1]
others = sys.argv[2].split(',')
d = collections.defaultdict(dict)
for f in sys.argv[3:]:
    for line in open(f):
        r = line.split()
        d[r[2]][r[1]] = (int(r[4]), float(r[5]), int(r[6]))
random.seed(1)
for n in others:
    us = [u for u in d if ref in d[u] and n in d[u] and d[u][ref][2] and d[u][n][2]]
    if not us:
        continue
    rs = sum(d[u][ref][0] for u in us)
    ns = sum(d[u][n][0] for u in us)
    logs = [math.log(d[u][n][0] / d[u][ref][0]) for u in us]
    better = sum(1 for x in logs if x < -0.01)
    worse = sum(1 for x in logs if x > 0.01)
    boots = []
    for _ in range(2000):
        s = [random.choice(us) for _ in us]
        boots.append(sum(d[u][n][0] for u in s) / sum(d[u][ref][0] for u in s))
    boots.sort()
    print('%-10s units %3d  total %.3f [%.3f, %.3f]  geomean %.3f  better %2d worse %2d' % (
        n, len(us), ns / rs, boots[50], boots[1950], math.exp(sum(logs) / len(logs)), better, worse))
