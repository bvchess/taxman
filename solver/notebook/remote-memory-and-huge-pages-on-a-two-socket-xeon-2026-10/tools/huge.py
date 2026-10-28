#!/usr/bin/env python3
"""For each running process named NAME of this user: its anonymous memory and the part of it in transparent huge pages
(/proc/PID/smaps_rollup), and the largest mappings with their huge pages (/proc/PID/smaps).  Usage: huge.py NAME"""
import os
import sys

name = sys.argv[1]
for pid in os.listdir('/proc'):
    if not pid.isdigit():
        continue
    try:
        if open(f'/proc/{pid}/comm').read().strip() != name:
            continue
        env = open(f'/proc/{pid}/environ').read().split('\0')
        tunables = [e for e in env if e.startswith('GLIBC_TUNABLES=')]
        roll = {line.split(':')[0]: line.split(':')[1].strip() for line in open(f'/proc/{pid}/smaps_rollup')
                if ':' in line and not line[0].isdigit()}
        maps = []
        current = None
        for line in open(f'/proc/{pid}/smaps'):
            fields = line.split()
            if '-' in fields[0] and ':' not in fields[0]:
                a, b = (int(x, 16) for x in fields[0].split('-'))
                current = [b - a, fields[0], ' '.join(fields[5:]), 0, 0]
                maps.append(current)
            elif fields[0] == 'Rss:':
                current[3] = int(fields[1])
            elif fields[0] == 'AnonHugePages:':
                current[4] = int(fields[1])
    except (FileNotFoundError, ProcessLookupError, PermissionError):
        continue
    print(pid, tunables, 'Anonymous', roll.get('Anonymous'), 'AnonHugePages', roll.get('AnonHugePages'))
    for size, rng, what, rss, huge in sorted(maps, key=lambda m: -m[3])[:8]:
        print(f"   {rng} {size >> 10:9} kB  rss {rss:8} kB  huge {huge:8} kB  {what}")
