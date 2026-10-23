#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Frame 2 at a node after fixing by the GF(2) bound: a candidate whose bound with its membership flipped against the
heaviest set is below the node's need is fixed (a member forced in, another member fixed out), as no completion that
beats the incumbent can hold or leave it.  What is left free, and frame 2's kernel among it.

python3 fixing.py instances.jsonl.gz [slack]

`slack` (default 0) lowers the threshold: a candidate is fixed only if its flipped bound is below need - slack.
"""

import sys
from collections import Counter

from f2lib import (Frame2, bipartite_graph, components, max_factors, read_instances, smallest_prime_factor_table,
                   treewidth_lower, treewidth_upper, two_core)


def analyze(inst, spf, slack=0, h=2):
    need = inst["need"]
    if need is None:
        return None
    cands = [x for x, _ in inst["cands"]]
    home = dict((x, hh) for x, hh in inst["cands"])
    chosen = set(inst["chosen"])
    flipped = dict(zip(cands, inst["flipped"]))
    fixed_in, fixed_out, free = set(), set(), set()
    for x in cands:
        v = flipped[x]
        if v is None:
            fixed_out.add(x)
        elif v + slack < need:
            (fixed_in if x in chosen else fixed_out).add(x)
        else:
            free.add(x)
    fr = Frame2(inst, h, spf)
    # The frame after fixing: forced movers in, forced factors out; the free ones optional.
    moves = set(fr.moves) | {x for x in fixed_in if home[x] == h + 1}
    factors = set(fr.factors) - {x for x in fixed_in if home[x] == h}
    opt_in = sorted(x for x in free if home[x] == h + 1)
    opt_out = sorted(x for x in free if home[x] == h and x in factors)
    fr2 = Frame2.__new__(Frame2)
    fr2.n, fr2.h, fr2.spf = fr.n, h, spf
    fr2.moves, fr2.factors, fr2.freedoms = moves, factors, None
    fr2.c_in, fr2.c_out = opt_in, opt_out
    fr2.movers = sorted(moves) + opt_in
    fr2.adj = {y: [f for f in max_factors(y, spf) if f in factors] for y in fr2.movers}
    fr2.users = {f: [] for f in factors}
    for y in fr2.movers:
        for f in fr2.adj[y]:
            fr2.users[f].append(y)
    winnable_now = not fr2.residual(moves, factors)
    kern = fr2.kernel()
    g = bipartite_graph(fr2, sorted(kern))
    comps = components(g)
    core = two_core(g)
    tw = (treewidth_lower(core), treewidth_upper(core, "minfill")) if core else (0, 0)
    by_home = Counter(home[x] for x in free)
    return {
        "free": len(free), "fixed_in": len(fixed_in), "fixed_out": len(fixed_out), "by_home": by_home,
        "opt_in": len(opt_in), "opt_out": len(opt_out), "kernel": len(kern),
        "kernel_in": len([y for y in kern if y in set(opt_in)]),
        "kernel_out": len([f for v, f in g if v == "f" and f in set(opt_out)]),
        "comps": len(comps), "largest": max((len(c) for c in comps), default=0), "core": len(core), "tw": tw,
        "winnable": winnable_now,
    }


def main():
    path = sys.argv[1]
    slack = int(sys.argv[2]) if len(sys.argv) > 2 else 0
    spf_cache = {}
    print("| node | subtree | gap | candidates | fixed in/out | free (by home 1/2/3/4/5+) | frame 2: optional in/out |"
          " kernel movers (optional in; optional out) | comps (largest) | 2-core | tw lower/min-fill |")
    print("|" + "---|" * 11)
    for inst in read_instances(path):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        r = analyze(inst, spf_cache[n], slack)
        if r is None:
            continue
        gap = inst["bound"] - inst["need"] + 1
        bh = r["by_home"]
        homes = "/".join(str(bh.get(k, 0)) for k in range(1, 5)) + "/" + str(sum(v for k, v in bh.items() if k >= 5))
        print(f"| {inst['id']} #{inst['node']} | {inst['subtree']} | {gap} | {len(inst['cands'])} |"
              f" {r['fixed_in']}/{r['fixed_out']} | {r['free']} ({homes}) | {r['opt_in']}/{r['opt_out']} |"
              f" {r['kernel']} ({r['kernel_in']}; {r['kernel_out']}) | {r['comps']} ({r['largest']}) | {r['core']} |"
              f" {r['tw'][0]}/{r['tw'][1]} |", flush=True)


if __name__ == "__main__":
    main()
