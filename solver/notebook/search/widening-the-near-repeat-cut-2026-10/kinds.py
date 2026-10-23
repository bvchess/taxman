#!/usr/bin/env python3
"""Searched games of the chain by the sign of the shift of P* from game n - 1 and the parity of n, with their share
of the steps.

Usage: python3 -I kinds.py STRICT.jsonl FROM TO [list]"""
import collections
import json
import sys

rows = {}
for line in open(sys.argv[1]):
    d = json.loads(line)
    rows.setdefault(d["n"], []).append(d)
lo, hi = int(sys.argv[2]), int(sys.argv[3])


def pstar(d):
    ib = d.get("in_the_bag")
    return d["score"] - ib if ib else None


tot, cnt = collections.Counter(), collections.Counter()
for n in range(lo, hi + 1):
    if n not in rows or n - 1 not in rows:
        continue
    d = rows[n][-1]
    if d["method"] != "search" or d["proof"] != "exhausted":
        continue
    nodes = sum(r["nodes"] for r in rows[n])
    b, pb = pstar(d), pstar(rows[n - 1][-1])
    if pb is None:
        k = "prev prime"
    else:
        k = "equal" if b == pb else "rise" if b > pb else "fall"
    k += " even" if n % 2 == 0 else " odd"
    if n == 4875:
        k += " (4875)"
    tot[k] += nodes
    cnt[k] += 1
    if len(sys.argv) > 4:
        print(n, k, nodes, b, pb, None if pb is None else b - pb)
everything = sum(tot.values())
without = everything - sum(v for k, v in tot.items() if "4875" in k)
for k in sorted(tot):
    share = 0 if "4875" in k else tot[k] / without
    print(f"{k:22} games {cnt[k]:3d} steps {tot[k]:.3g} share {tot[k] / everything:.3f} without 4875 {share:.3f}")
print(f"total {everything:.3g}, without 4875 {without:.3g}")
