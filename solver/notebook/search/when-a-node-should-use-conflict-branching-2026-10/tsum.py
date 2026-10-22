#!/usr/bin/env python3
"""Summarize a conflict-branching trace (TAXMAN_CB_TRACE): steps by depth, how branchings end, the choices."""
import re
import sys
from collections import Counter

for path in sys.argv[1:]:
    steps = Counter()  # steps by node depth
    ends = Counter()  # step ends
    cb = Counter()  # CB choices: (k0 size, same as k0)
    chosen_len = Counter()
    min_depth = None
    last_b = None
    child_end = Counter()  # for branchings without forced members: (k size, how the step ended)
    for line in open(path):
        if line.startswith("S "):
            d = int(re.search(r"d=(\d+)", line).group(1))
            min_depth = d if min_depth is None else min(min_depth, d)
            steps[d] += 1
            last_b = None
        elif line.startswith("B "):
            m = re.search(r"k=(\d+) forced=(\d+).*ends=(\w+)", line)
            last_b = (int(m.group(1)), m.group(3) == "true")
        elif line.startswith("E "):
            e = re.sub(r"\(.*", "", line.split()[1])
            ends[e] += 1
            if last_b is not None and not last_b[1]:
                child_end[(min(last_b[0], 4), e)] += 1
        elif line.startswith("CB "):
            m = re.search(r"chose w=([\d.]+) computed=(\w+) same=(\w+) K=\[([^\]]*)\]", line)
            k0 = re.search(r"k0=\[([^\]]*)\]", line).group(1).split()
            same = m.group(3) == "true"
            cb[(min(len(k0), 4), same)] += 1
            if not same:
                chosen_len[min(len(m.group(4).split()), 8)] += 1
    total = sum(steps.values())
    print(f"== {path}: {total} steps, unit depth {min_depth}")
    rel = Counter()
    for d, c in steps.items():
        rel[(d - min_depth) // 10 * 10] += c
    print("  steps by depth below the unit (bins of 10):", dict(sorted(rel.items())))
    print("  step ends:", dict(ends))
    print("  CB choices (k0 size, same as k0):", dict(sorted(cb.items())))
    print("  size of the chosen K when not k0:", dict(sorted(chosen_len.items())))
    print("  branchings without forced members, (k size, end):", dict(sorted(child_end.items())))
