#!/usr/bin/env python3
"""lk-study3.py LABEL: the LOOK3 lines of runs/LABEL/*.err summed up."""
import glob
import sys

S = '/tmp/claude-1001/-home-brian-taxman-work-research/ee866c42-a3bc-4d6b-b3f5-6d2038b6659f/scratchpad/runs/'
for f in sorted(glob.glob(S + sys.argv[1] + '/*.err')):
    rows = [dict(kv.split('=') for kv in l.split()[1:]) for l in open(f) if l.startswith('LOOK3')]
    split = [r for r in rows if r['kind'] == 'split']
    ev = [r for r in rows if r['kind'] == 'eval']
    evcut = [r for r in ev if int(r['val']) < int(r['need'])]
    cut = [r for r in ev if r['cut'] == '1']

    def us(rs):
        return sum(int(r['us']) for r in rs) / 1e6

    print(f"{f.split('/')[-1]}: looks reaching the bound {len(split) + len(ev)}: cut by the split bound {len(split)} "
          f"({us(split):.2f} s), solved {len(ev)} ({us(ev):.2f} s), of those below the need {len(evcut)} "
          f"({us(evcut):.2f} s), cut in all {len(cut)}")
    gaps = sorted(int(r['sb']) - int(r['need']) for r in evcut)
    if gaps:
        print('  split bound less need where the solve cut:', gaps[:10], 'median', gaps[len(gaps) // 2])
    over = sorted(int(r['sb']) - int(r['val']) for r in ev if int(r['sb']) >= 0)
    if over:
        print('  split bound less the solved bound:', 'median', over[len(over) // 2], 'zero', sum(o == 0 for o in over))
