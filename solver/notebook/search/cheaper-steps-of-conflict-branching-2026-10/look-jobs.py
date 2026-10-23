#!/usr/bin/env python3
"""mkjobs.py LABEL A B [extra args]: the unit jobs, builds A and B side by side on each unit, to stdout."""
import sys

S = '/tmp/claude-1001/-home-brian-taxman-work-research/ee866c42-a3bc-4d6b-b3f5-6d2038b6659f/scratchpad'
U75 = ('/home/brian/taxman_work/research/.claude/worktrees/agent-a0b8d02e93ffd8afe/solver/notebook/search/'
       'where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/units-fresh.jsonl')
U50 = S + '/units/units-4950.jsonl'
label, a, b = sys.argv[1:4]
extra = ' '.join(sys.argv[4:]) or '--tactic conflict_look=on'
for units, uid in [(U75, '4875-10'), (U50, '4950-1486'), (U50, '4950-4352'), (U50, '4950-853')]:
    for build in (a, b):
        print(f'bash {S}/unit.sh tune-{build} {label}-{build} {units} {uid} --node-limit 60000 {extra}')
