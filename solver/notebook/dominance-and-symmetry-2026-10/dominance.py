#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of docs/experiments/dominance-and-symmetry-2026-10.md, from the data in this directory.

Run from rust/: bench/dominance/dominance.py

The example `dominance`, which wrote the node samples and the symmetry checks, was removed from master (2026-10-17); it
builds at commit 8753189 (`git checkout 8753189`, then `cargo build --release --example dominance` in rust/).
"""
import ast
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).parent


def lines(name):
    return [json.loads(line) for line in open(HERE / name) if line.startswith('{')]


def by_n(rows):
    out = defaultdict(list)
    for r in rows:
        out[r['n']].append(r)
    return out


def pct(k, total):
    return f"{100 * k / total:.0f}%" if total else "-"


def screening(title, name):
    """Per game: the share of open nodes dominated, by what proves it and by the sampled upper estimate."""
    print(f"\n### {title} ({name})\n")
    print('| n | nodes | open | promoted (median) | relevant (median) | literal cover, all of C '
          '| literal cover, relevant |'
          ' prime exchange cover | symmetry rule (hopeless / bound) | any proof | sampled upper estimate | twins |')
    print('|---|---|---|---|---|---|---|---|---|---|---|---|')
    for n, rs in sorted(by_n(lines(name)).items()):
        live = [r for r in rs if not r.get('closed')]
        cover_all = sum(any(r['cover_all']) for r in live)
        cover_rel = sum(any(r['cover_relevant']) or any(r['cover_relabelled']) for r in live)
        prime = sum(r['prime_cover'] for r in live)
        hopeless = sum(r['symmetry_hopeless'] for r in live)
        bound = sum(r['symmetry_bound'] for r in live)
        proved = sum(any(r['cover_relevant']) or any(r['cover_relabelled']) or r['prime_cover'] or r['symmetry_bound']
                     for r in live)
        sampled = sum(any(r['sampled']) or r['prime_sampled'] or r['symmetry_bound'] for r in live)
        twins = sum(r['twins_relevant'] > 0 for r in live)
        k = len(live)
        print(f"| {n} | {len(rs)} | {k} | {statistics.median(r['promoted'] for r in live):.0f} | "
              f"{statistics.median(r['relevant'] for r in live):.0f} | {pct(cover_all, k)} | {pct(cover_rel, k)} | "
              f"{pct(prime, k)} | {pct(hopeless, k)} / {pct(bound, k)} | {pct(proved, k)} | {pct(sampled, k)} | "
              f"{twins} |")


def pairs(title, name):
    print(f"\n### {title} ({name})\n")
    print('| n | nodes | sampled: one for one | two for one or two '
          '| either, or a prime exchange or the symmetry rule |')
    print('|---|---|---|---|---|')
    for n, rs in sorted(by_n(lines(name)).items()):
        live = [r for r in rs if not r.get('closed')]
        k = len(live)
        one = sum(r['sampled'][0] or r['sampled'][1] for r in live)
        two = sum(r['sampled'][2] for r in live)
        anyk = sum(any(r['sampled']) or r['prime_sampled'] or r['symmetry_bound'] for r in live)
        print(f"| {n} | {k} | {pct(one, k)} | {pct(two, k)} | {pct(anyk, k)} |")


def classes(pair_list):
    parent = {}

    def find(x):
        while parent.setdefault(x, x) != x:
            x = parent[x]
        return x
    for a, b in pair_list:
        parent[find(a)] = find(b)
    groups = defaultdict(set)
    for x in list(parent):
        groups[find(x)].add(x)
    return [sorted(g) for g in groups.values()]


def symmetries():
    print("\n### The root's prime symmetries by band (symmetries.jsonl)\n")
    print('| n | games | candidates | classes of primes | largest class | candidates moved '
          '| log10 of the group order |')
    print('|---|---|---|---|---|---|---|')
    bands = defaultdict(list)
    for r in lines('symmetries.jsonl'):
        cl = classes(ast.literal_eval(r['pairs']))
        order = sum(math.log10(math.factorial(len(c))) for c in cl)
        largest = max((len(c) for c in cl), default=0)
        bands[(r['n'] - 1) // 50].append((r['candidates'], len(cl), largest, r['moved'], order))
    for k in sorted(bands):
        v = bands[k]
        m = [sum(x[i] for x in v) / len(v) for i in range(5)]
        print(f"| {50 * k + 1}-{50 * k + 50} | {len(v)} | {m[0]:.0f} | {m[1]:.1f} | {m[2]:.1f} | "
              f"{m[3]:.1f} ({100 * m[3] / m[0]:.0f}%) | {m[4]:.1f} |")


def compare(title, base, rule):
    b = {r['n']: r for r in lines(base)}
    r = {x['n']: x for x in lines(rule)}
    ns = [n for n in sorted(b) if n in r]
    assert all(b[n]['completed'] and r[n]['completed'] for n in ns)
    lr = [math.log10(r[n]['nodes'] / b[n]['nodes']) for n in ns]
    xs = [math.log10(b[n]['nodes']) for n in ns]
    mx, my = sum(xs) / len(xs), sum(lr) / len(lr)
    slope = sum((x - mx) * (y - my) for x, y in zip(xs, lr)) / sum((x - mx) ** 2 for x in xs)
    nb, nr = sum(b[n]['nodes'] for n in ns), sum(r[n]['nodes'] for n in ns)
    tb, tr = sum(b[n]['seconds'] for n in ns), sum(r[n]['seconds'] for n in ns)
    best = min(zip(lr, ns))
    worst = max(zip(lr, ns))
    print(f"| {title} | {len(ns)} | {10 ** my:.3f} | {nr / nb:.3f} | {10 ** best[0]:.2f} ({best[1]}) | "
          f"{10 ** worst[0]:.2f} ({worst[1]}) | {slope:+.3f} | {tb:.0f} -> {tr:.0f} s |")


def pilots():
    print('\n### The pilots: nodes with the rule against without (pilots/)\n')
    print('| search, rule | games | nodes, geometric ratio | total ratio | fewest | most '
          '| slope of log ratio on log nodes | time |')
    print('|---|---|---|---|---|---|---|---|')
    compare('solver, hopeless', 'pilots/solver_off.jsonl', 'pilots/solver_hopeless.jsonl')
    compare('solver, matching bound of the constraint', 'pilots/solver_off.jsonl', 'pilots/solver_bound.jsonl')
    compare('GF(2) B&B, hopeless', 'pilots/gf2_off.jsonl', 'pilots/gf2_hopeless.jsonl')
    compare('GF(2) B&B, Lagrangian of the constraint', 'pilots/gf2_off.jsonl', 'pilots/gf2_lagrange.jsonl')
    print('\n### Margin scans of the GF(2) pilot (pilots/gf2_delta.jsonl: δ = 0, 25, 50, 100 in that order)\n')
    print('| n | δ | nodes without | with | ratio |')
    print('|---|---|---|---|---|')
    rows = lines('pilots/gf2_delta.jsonl')
    games = sorted({r['n'] for r in rows})
    per = defaultdict(list)
    for r in rows:
        per[(r['n'], r['rule'])].append(r['nodes'])
    for n in games:
        for d, off, lag in zip((0, 25, 50, 100), per[(n, 'Off')], per[(n, 'Lagrange')]):
            print(f"| {n} | {d} | {off} | {lag} | {lag / off:.3f} |")


def soundness():
    print('\n### Soundness runs\n')
    g = lines('pilots/gf2_sound.jsonl')
    s = lines('pilots/solver_sound.jsonl')
    c = lines('symcheck.jsonl')
    print(f"* GF(2) pilot, incumbent P* - 1: {len(g)} games, found the optimum in "
          f"{sum(r['better'] > 0 for r in g)}, the rule cut in {sum(r['hopeless'] + r['lagrange'] > 0 for r in g)}")
    print(f"* solver pilot, incumbent P* - 1: {len(s)} games, found the optimum in {sum(r['ok'] for r in s)}, "
          f"the rule cut in {sum(r['cuts'] > 0 for r in s)}")
    small = [r for r in c if r['enumerated']]
    print(f"* symcheck: {len(c)} games with pairs; {len(small)} boards enumerated "
          f"({sum(r['enumerated'] for r in small)} promotable sets), the constrained best equal to P* in all")


def main():
    screening("The probe's fixtures", 'nodes_fixtures.jsonl')
    screening("The solver's proofs, nodes sampled over their steps", 'nodes_solver.jsonl')
    screening("The same, incumbent P* + 100", 'nodes_solver_d100.jsonl')
    screening("The same, incumbent P* + 200", 'nodes_solver_d200.jsonl')
    screening("The GF(2) branch and bound's trees", 'nodes_gf2.jsonl')
    pairs("Two-member exchanges, the solver's proofs", 'nodes_pairs_solver.jsonl')
    pairs("Two-member exchanges, the GF(2) trees", 'nodes_pairs_gf2.jsonl')
    symmetries()
    pilots()
    soundness()


if __name__ == '__main__':
    main()
