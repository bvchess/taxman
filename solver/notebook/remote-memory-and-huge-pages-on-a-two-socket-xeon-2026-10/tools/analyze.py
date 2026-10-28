#!/usr/bin/env python3
"""Reads the samples of share.py (keyed by thread id) and prints, over the CPU time of the engine threads between
samples: the share whose NUMA faults (private and shared pages apart) were on the other node than the thread's CPU, the
threads seen on both nodes, the samples off a thread's majority node, and the moves between nodes a minute.
Usage: analyze.py SAMPLES.jsonl[.gz]"""
import gzip
import json
import sys

NODE_OF = {}
for n in range(8):
    try:
        text = open(f'/sys/devices/system/node/node{n}/cpulist').read().strip()
    except FileNotFoundError:
        continue
    for part in text.split(','):
        a, _, b = part.partition('-')
        for cpu in range(int(a), int(b or a) + 1):
            NODE_OF[cpu] = n

path = sys.argv[1]
samples = [json.loads(line) for line in (gzip.open(path, 'rt') if path.endswith('.gz') else open(path))]
total = remote = total_p = remote_p = private_share = 0.0
moves = 0
for a, b in zip(samples, samples[1:]):
    for tid, tb in b['threads'].items():
        ta = a['threads'].get(tid)
        if not ta:
            continue
        if NODE_OF[ta['cpu']] != NODE_OF[tb['cpu']]:
            moves += 1
        dt = tb['ticks'] - ta['ticks']
        if dt <= 0:
            continue
        here = NODE_OF[tb['cpu']]
        f = {}
        for t in (ta, tb):
            for n, v in t['faults'].items():
                f[int(n)] = f.get(int(n), 0) + v
        if not sum(f.values()):
            continue
        total += dt
        remote += dt * sum(v for n, v in f.items() if n != here) / sum(f.values())
        if 'private' in ta and 'private' in tb:
            p = {}
            for t in (ta, tb):
                for n, v in t['private'].items():
                    p[int(n)] = p.get(int(n), 0) + v
            if sum(p.values()):
                total_p += dt
                remote_p += dt * sum(v for n, v in p.items() if n != here) / sum(p.values())
                private_share += dt * sum(p.values()) / sum(f.values())
nodes = {}
for s in samples:
    for tid, t in s['threads'].items():
        nodes.setdefault(tid, []).append(NODE_OF[t['cpu']])
both = sum(1 for ns in nodes.values() if len(set(ns)) > 1)
off = sum(len(ns) - max(ns.count(0), ns.count(1)) for ns in nodes.values())
count = sum(len(ns) for ns in nodes.values())
span = samples[-1]['t'] - samples[0]['t']
print(f"{len(samples)} samples over {span:.0f} s; threads seen {len(nodes)}, on both nodes {both}")
print(f"CPU time weighted by the share of the thread's NUMA faults on the other node: {100 * remote / total:.1f}%")
if total_p:
    print(f"private pages: {100 * private_share / total_p:.1f}% of the faults; CPU time weighted by the share of the "
          f"private faults on the other node: {100 * remote_p / total_p:.1f}%")
print(f"samples off the thread's majority node: {100 * off / count:.1f}%; moves between nodes "
      f"{moves} ({60 * moves / span / max(len(nodes), 1):.2f} a thread a minute)")
