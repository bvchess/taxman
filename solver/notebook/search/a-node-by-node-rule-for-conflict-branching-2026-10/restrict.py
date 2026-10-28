#!/usr/bin/env python3
"""Keeps the lines of collect.py tables whose unit is listed in IDS: restrict.py IDS FILE..."""
import sys

ids = set(line.split()[0] for line in open(sys.argv[1]) if line.strip())
for f in sys.argv[2:]:
    for line in open(f):
        if line.split()[2] in ids:
            sys.stdout.write(line)
