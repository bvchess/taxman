#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Inclusive sample counts of functions in a macOS `sample` call graph, the engine thread's:
profile.py FILE [--under ANCESTOR] PATTERN...

A pattern's inclusive count is the sum of the counts of the frames it matches that have no frame matching it above them
(so recursion counts once), as a share of the engine thread's samples.  With --under, each count is split between the
frames that have a frame matching ANCESTOR above them and the others."""
import argparse
import re

parser = argparse.ArgumentParser()
parser.add_argument("file")
parser.add_argument("--under")
parser.add_argument("patterns", nargs="+")
args = parser.parse_args()

lines = open(args.file).read().splitlines()
start = next(i for i, l in enumerate(lines) if l.startswith("Call graph:"))
end = next(i for i, l in enumerate(lines) if l.startswith("Total number in stack"))
frames = []
for l in lines[start + 1:end]:
    m = re.match(r"^([\s+!:|]*)(\d+)\s+(.*)$", l)
    if m:
        frames.append((len(m.group(1)), int(m.group(2)), m.group(3)))
total = next(c for d, c, name in frames if "engine-0" in name)
print("engine thread samples", total)
for pat in args.patterns:
    stack = []  # (depth, matches the pattern, matches the ancestor)
    under = elsewhere = 0
    for depth, count, name in frames:
        while stack and stack[-1][0] >= depth:
            stack.pop()
        matched = re.search(pat, name) is not None
        if matched and not any(m for _, m, _ in stack):
            if any(a for _, _, a in stack):
                under += count
            else:
                elsewhere += count
        stack.append((depth, matched, args.under is not None and re.search(args.under, name) is not None))
    if args.under is None:
        print(f"{pat:30s} {elsewhere:7d} {100 * elsewhere / total:5.1f}%")
    else:
        print(f"{pat:30s} under {under:7d} {100 * under / total:5.1f}%  elsewhere {elsewhere:7d} "
              f"{100 * elsewhere / total:5.1f}%")
