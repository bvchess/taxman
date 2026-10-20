#!/usr/bin/env python3
"""The tables of "The headroom of branching, in hindsight" (README.md beside this file), from runs.jsonl.

usage: python3 -I headroom.py [runs.jsonl]

Each line of runs.jsonl is a line of `wall headroom` (headroom.patch; its module comment defines the fields) with two
fields added: `label`, the experiment (`levels`, `root`, `conflicts`, `butterflies` or `perturbed`), and `tactics`,
the learning and the tactics it ran with.
"""
import json
import statistics
import sys
from collections import defaultdict


def size(line):
    return line['size'] if line['size'] is not None else float('inf')


def levels(rows):
    print('== The hindsight, level by level')
    groups = defaultdict(list)
    for r in rows:
        if r['label'] == 'levels':
            groups[(r['id'], r['tactics'])].append(r)
    for (unit, tactics), lines in groups.items():
        base = next(r for r in lines if r['run'] == 'base')
        print(f"{unit} [{tactics}]: base {base['nodes']} steps")
        slots = defaultdict(list)
        for r in lines:
            if r['run'] == 'alt':
                slots[(r['level'], r['slot'])].append(r)
        for (level, slot), alts in sorted(slots.items()):
            strong = next(a for a in alts if a['strong'])
            best = min(alts, key=size)
            ratios = [size(a) / strong['size'] for a in alts]
            print(f"  level {level} slot {slot}: depth {strong['d']}, conflict of {len(strong['k'])},"
                  f" gap {strong['value'] - strong['need']}; strong {strong['m']} {strong['size']}"
                  f" (child {strong['child']}, rest {strong['rest']}); best {best['m']} {best['size']}"
                  f" = {size(best) / strong['size']:.2f}; median {statistics.median(ratios):.2f}")
        for r in lines:
            if r['run'] == 'level':
                ok = r['completed'] and r['best'] == r['opt']
                print(f"  plan after level {r['level']}: {r['nodes']} = {r['nodes'] / base['nodes']:.3f}"
                      f" of the base; sound {ok}")


def alternatives(rows, label, title):
    print(f'== {title}')
    for r in rows:
        if r['label'] == label and r['run'] == 'alt':
            price = r['value'] - r['next_value'] if r['next_value'] is not None else None
            child_gap = r['child_value'] - (r['need'] - r['m']) if r['child_value'] is not None else None
            extra = ''
            if 'child_forced' in r:
                extra = f" child: gap {child_gap}, forced {r['child_forced']}, conflict of {r['child_k']};"
            core = f" core {r['core']} of {r['cores']}" if r.get('cores') else ''
            print(f"  {r['id']} [{r['tactics']}]{core} {r['m']}{' (strong)' if r['strong'] else ''}: {r['size']}"
                  f" (child {r['child']}, rest {r['rest']}); price {price};{extra} home {r['home']},"
                  f" twins {r['twins']}")


def cores(rows):
    print('== The root of 3940-104, history-free: each core of the heaviest set')
    by_core = defaultdict(list)
    for r in rows:
        if r['label'] == 'conflicts' and r['run'] == 'alt':
            by_core[r['core']].append(size(r))
    base = next(r for r in rows if r['label'] == 'conflicts' and r['run'] == 'base')['nodes']
    for core, sizes in sorted(by_core.items()):
        sizes.sort()
        print(f"  core {core}: {len(sizes)} members; best {sizes[0] / base:.2f}, median"
              f" {statistics.median(sizes) / base:.2f}, worst {sizes[-1] / base:.2f} of strong's {base}")


def butterflies(rows):
    print('== The butterflies')
    groups = defaultdict(list)
    for r in rows:
        if r['label'] == 'butterflies' and r['run'] == 'butterfly':
            groups[(r['id'], r['tactics'])].append(r)
    for (unit, tactics), lines in groups.items():
        ratios = [r['nodes'] / r['base'] for r in lines]
        moved = sum(abs(x - 1) > 0.05 for x in ratios)
        print(f"  {unit} [{tactics}]: base {lines[0]['base']}; {moved} of {len(lines)} move the tree by more than"
              f" 5%; totals {min(ratios):.3f}-{max(ratios):.3f} of the base")
        for r in lines:
            print(f"    step {r['step']}, depth {r['d']}: its subtree {r['before']} -> {r['after']} steps;"
                  f" the tree {r['nodes']} = {r['nodes'] / r['base']:.3f}")


# The root's members followed under other histories: the hindsight's best with the learning, and without the table.
TRACKED = {('3940-104', 1350), ('3940-104', 1452), ('4416-95', 1800)}


def perturbed(rows):
    print('== The root choices with the learning, under other forgetting periods')
    groups = defaultdict(lambda: defaultdict(dict))
    for r in rows:
        learning = r['tactics'].startswith('learn')
        if learning and r['label'] in ('perturbed', 'levels') and r['run'] == 'alt' and r['level'] == 1:
            groups[r['id']][r['tactics']][r['m']] = (r['size'], r['strong'])
    for unit, by_tactics in groups.items():
        if not any((unit, m) in TRACKED for members in by_tactics.values() for m in members):
            continue
        print(f"  {unit}")
        logs = defaultdict(list)
        for tactics, members in by_tactics.items():
            strong = next(s for s, is_strong in members.values() if is_strong)
            listed = []
            for m, (s, is_strong) in sorted(members.items()):
                if is_strong:
                    continue
                if (unit, m) in TRACKED:
                    listed.append(f"{m} {s} ({s / strong:.2f})")
                    logs[m].append(s / strong)
            print(f"    [{tactics}] strong {strong}; " + '; '.join(listed))
        for m, ratios in logs.items():
            mean = statistics.geometric_mean(ratios)
            print(f"    {m}: geometric mean {mean:.2f} of strong over {len(ratios)} histories")


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else 'runs.jsonl'
    rows = [json.loads(line) for line in open(path)]
    levels(rows)
    alternatives(rows, 'root', 'The root of 3940-104 with the learning, with the child\'s check')
    cores(rows)
    butterflies(rows)
    perturbed(rows)


if __name__ == '__main__':
    main()
