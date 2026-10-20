"""Compares the traces of two runs of the build of loops.diff (stderr of `tune solve`, run with LOOPS_TRACE=1): the first
evaluation (TR: value, heaviest set, forced members) or branch (TB: member chosen, forced members) that differs, with the
lines of each run from the node's evaluation to it: the order in which strong branching tried the members (TO) and the
splits whose bounds differ with and without the loops (TS: member, bound with the loops, bound without them; only in a
run with LOOPS_SHADOW=1).  Standard library only.

usage: loops_trace.py RUN_A.err RUN_B.err
"""
import sys


def load(path):
    with open(path) as f:
        return [line.rstrip('\n') for line in f if line.startswith(('TR ', 'TB ', 'TO ', 'TS '))]


def core(lines):
    return [line for line in lines if line.startswith(('TR ', 'TB '))]


def around(lines, index):
    """The lines from the evaluation before the `index`th TR or TB line to that line."""
    seen = -1
    for j, line in enumerate(lines):
        if line.startswith(('TR ', 'TB ')):
            seen += 1
            if seen == index:
                k = j
                while k > 0 and not lines[k].startswith('TR '):
                    k -= 1
                return lines[max(0, k - 3):j + 1]
    return []


a, b = load(sys.argv[1]), load(sys.argv[2])
ca, cb = core(a), core(b)
print(len(ca), len(cb), 'evaluations and branches')
for i, (x, y) in enumerate(zip(ca, cb)):
    if x != y:
        print('first difference at', i, ':', x[:80], '|', y[:80])
        for name, lines in (('A', a), ('B', b)):
            print('=====', name)
            for line in around(lines, i):
                print(line[:1500])
        break
else:
    print('identical' if len(ca) == len(cb) else 'one is a prefix of the other')
