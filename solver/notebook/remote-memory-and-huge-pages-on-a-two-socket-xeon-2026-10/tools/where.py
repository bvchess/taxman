#!/usr/bin/env python3
"""For each running process named NAME of this user: its CPU, its memory policy, and its anonymous pages by node
(/proc/PID/numa_maps), to check where a run's memory is.  Usage: where.py NAME"""
import collections
import os
import sys

name = sys.argv[1]
for pid in os.listdir('/proc'):
    if not pid.isdigit():
        continue
    try:
        if open(f'/proc/{pid}/comm').read().strip() != name:
            continue
        stat = open(f'/proc/{pid}/stat').read()
        cpu = int(stat[stat.rindex(')') + 2:].split()[36])
        pages = collections.Counter()
        policies = collections.Counter()
        for line in open(f'/proc/{pid}/numa_maps'):
            fields = line.split()
            if 'anon=' not in line:
                continue
            policies[fields[1]] += 1
            for x in fields:
                if x[0] == 'N' and '=' in x:
                    k, v = x.split('=')
                    pages[k] += int(v)
    except (FileNotFoundError, ProcessLookupError):
        continue
    print(pid, 'cpu', cpu, dict(policies), {k: f'{v * 4 / 1024:.1f} MiB' for k, v in sorted(pages.items())})
