#!/usr/bin/env python3
"""Samples, read-only, where the chain's engine threads run and where the memory they touch lives: every INTERVAL
seconds for DURATION seconds, each engine thread's CPU (/proc/PID/task/TID/stat, field 39), CPU time (utime + stime)
and the kernel's NUMA-balancing faults by the node of the page (/proc/PID/task/TID/sched, numa_faults: a decaying
count of the hinting faults the thread took on pages of each node), with /proc/vmstat's NUMA counters at both ends.
Writes one JSON line a sample to OUT and prints a summary.  Usage: share.py PID DURATION INTERVAL OUT"""
import json
import os
import re
import sys
import time

pid, duration, interval, out = int(sys.argv[1]), float(sys.argv[2]), float(sys.argv[3]), sys.argv[4]


def nodes():
    result = {}
    for n in range(8):
        try:
            text = open(f'/sys/devices/system/node/node{n}/cpulist').read().strip()
        except FileNotFoundError:
            continue
        for part in text.split(','):
            a, _, b = part.partition('-')
            for cpu in range(int(a), int(b or a) + 1):
                result[cpu] = n
    return result


NODE_OF = nodes()


def vmstat():
    keep = ('numa_hint_faults', 'numa_hint_faults_local', 'numa_pages_migrated', 'numa_pte_updates')
    return {k: int(v) for k, v in (line.split() for line in open('/proc/vmstat')) if k in keep}


def sample():
    threads = {}
    for tid in os.listdir(f'/proc/{pid}/task'):
        base = f'/proc/{pid}/task/{tid}'
        try:
            comm = open(f'{base}/comm').read().strip()
            if not comm.startswith('engine-'):
                continue
            stat = open(f'{base}/stat').read()
            fields = stat[stat.rindex(')') + 2:].split()
            cpu = int(fields[36])
            ticks = int(fields[11]) + int(fields[12])
            faults = {}
            private = {}
            migrated = 0
            for line in open(f'{base}/sched'):
                m = re.match(r'numa_faults node=(\d+) task_private=(\d+) task_shared=(\d+)', line)
                if m:
                    faults[int(m.group(1))] = int(m.group(2)) + int(m.group(3))
                    private[int(m.group(1))] = int(m.group(2))
                elif line.startswith('numa_pages_migrated'):
                    migrated = int(line.split(':')[1])
        except (FileNotFoundError, ProcessLookupError, ValueError):
            continue
        threads[tid] = {'comm': comm, 'cpu': cpu, 'ticks': ticks, 'faults': faults, 'private': private,
                        'migrated': migrated}
    return threads


samples = []
v0 = vmstat()
t0 = time.time()
with open(out, 'w') as f:
    while True:
        s = {'t': time.time() - t0, 'threads': sample()}
        samples.append(s)
        f.write(json.dumps(s) + '\n')
        f.flush()
        if s['t'] >= duration:
            break
        time.sleep(interval)
v1 = vmstat()

# Remote share: over each interval, a thread's CPU time weighted by the share of its NUMA faults (as read at the
# interval's start and end, averaged) on the node other than the one its CPU is on at the interval's end.
total = remote = 0.0
both = set()
seen = {}
for a, b in zip(samples, samples[1:]):
    for name, tb in b['threads'].items():
        ta = a['threads'].get(name)
        if not ta:
            continue
        dt = tb['ticks'] - ta['ticks']
        if dt <= 0:
            continue
        here = NODE_OF[tb['cpu']]
        fa, fb = ta['faults'], tb['faults']
        all_ = sum(fa.values()) + sum(fb.values())
        if all_ == 0:
            continue
        away = sum(v for n, v in fa.items() if n != here) + sum(v for n, v in fb.items() if n != here)
        total += dt
        remote += dt * away / all_
for s in samples:
    for name, t in s['threads'].items():
        seen.setdefault(name, set()).add(NODE_OF[t['cpu']])
both = [n for n, ns in seen.items() if len(ns) > 1]
off_home = n_samples = 0
for name in seen:
    counts = {}
    for s in samples:
        t = s['threads'].get(name)
        if t:
            counts[NODE_OF[t['cpu']]] = counts.get(NODE_OF[t['cpu']], 0) + 1
    off_home += sum(counts.values()) - max(counts.values())
    n_samples += sum(counts.values())
migrated = sum(samples[-1]['threads'][n]['migrated'] - samples[0]['threads'][n]['migrated']
               for n in samples[-1]['threads'] if n in samples[0]['threads'])
hint = v1['numa_hint_faults'] - v0['numa_hint_faults']
local = v1['numa_hint_faults_local'] - v0['numa_hint_faults_local']
span = samples[-1]['t']
print(f"samples {len(samples)} over {span:.0f} s, engine threads {len(seen)}, on both nodes at some sample {len(both)}")
print(f"CPU time weighted by the share of a thread's NUMA faults on the other node: "
      f"{100 * remote / max(total, 1):.1f}%")
print(f"samples off the thread's majority node: {100 * off_home / max(n_samples, 1):.1f}%")
print(f"system hinting faults {hint} ({hint / span:.0f}/s), local {100 * local / max(hint, 1):.1f}%; "
      f"pages migrated {v1['numa_pages_migrated'] - v0['numa_pages_migrated']} system, "
      f"{migrated} by the engine threads")
