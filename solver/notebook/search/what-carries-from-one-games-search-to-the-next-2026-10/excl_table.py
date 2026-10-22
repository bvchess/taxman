#!/usr/bin/env python3
"""The stratified units' results: per unit its depth, gap, members of the incumbent left out, and the steps of each
setting (+ when the CPU limit stopped it); then, by members left out, how often off beat on and the sums of steps.
Usage: excl_table.py SNAPNAME JOBS RESULTS_DIR"""
import collections
import json
import os
import sys


def bits(h):
    return frozenset(4 * k + b for k, c in enumerate(h) for b in range(4) if int(c, 16) >> b & 1)


snapname, jobs, rdir = sys.argv[1:4]
units, game = {}, None
for line in open(f"/home/brian/taxman-lab/snap/{snapname}/units.jsonl"):
    r = json.loads(line)
    if r["kind"] == "game":
        game = r
    else:
        units[r["id"]] = r
opt = bits(game["incumbent"])
ids, settings = [], []
for line in open(jobs):
    _, uid, s = line.split()
    if uid not in ids:
        ids.append(uid)
    if s not in settings:
        settings.append(s)


def res(uid, s):
    f = f"{rdir}/{uid}-{s}.json"
    if not os.path.exists(f):
        return None
    d = json.load(open(f))
    return d["nodes"], d["completed"]


groups = collections.defaultdict(list)
for uid in ids:
    u = units[uid]
    x = len(opt - bits(u["promoted"]) - bits(u["cands"]))
    r = {s: res(uid, s) for s in settings}
    groups[min(max(x, 3), 6)].append((u["depth"], u["gap"], uid, r))
    print(u["depth"], u["gap"], x, uid,
          *(f"{s} {'-' if r[s] is None else str(r[s][0]) + ('' if r[s][1] else '+')}" for s in settings))
print()
for x in sorted(groups):
    rows = [row for row in groups[x] if all(row[3][s] for s in settings)]
    wins = sum(1 for row in rows if row[3]["off"][1] and (not row[3]["on"][1] or row[3]["off"][0] < row[3]["on"][0]))
    sums = {s: sum(row[3][s][0] for row in rows) for s in settings}
    done = {s: sum(row[3][s][1] for row in rows) for s in settings}
    label = {3: "3 or fewer", 6: "6 or more"}.get(x, str(x))
    print(f"left out {label}: {len(rows)} units; off beats on in {wins}; steps (completed) "
          + ", ".join(f"{s} {sums[s]} ({done[s]})" for s in settings))
