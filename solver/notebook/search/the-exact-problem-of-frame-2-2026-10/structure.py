#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The structure of frame 2 at the nodes dumped by `wall local --out`: sizes, degrees, components, the kernel (the
movers that can lie in a self-covering set), cycle rank, treewidth bounds, and the twin families m·p.

python3 structure.py instances.jsonl.gz [h]
"""

import sys
from collections import Counter

from f2lib import (Frame2, bipartite_graph, components, read_instances, smallest_prime_factor_table,
                   treewidth_lower, treewidth_upper, primes_of, two_core)


def degree_hist(values):
    c = Counter(min(v, 4) for v in values)
    return "/".join(str(c.get(k, 0)) for k in range(0, 5))


def largest_prime_hub(y, spf):
    """y = m·p with p its largest prime, to the first power: (m, p); else None."""
    ps = primes_of(y, spf)
    p, e = ps[-1]
    if e != 1:
        return None
    return (y // p, p)


def main():
    path = sys.argv[1]
    h = int(sys.argv[2]) if len(sys.argv) > 2 else 2
    spf_cache = {}
    print("| node | subtree | gap | moves | in | factors | out | freedoms | live-factor degrees 0/1/2/3/4+ |"
          " kernel movers (moves+in) | kernel factors (out) | kernel comps (largest) | cycle rank |"
          " 2-core | tw lower/min-deg/min-fill | hubs: families >=2 (members) |")
    print("|" + "---|" * 15)
    for inst in read_instances(path):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        spf = spf_cache[n]
        fr = Frame2(inst, h, spf)
        node = inst["node"]
        label = f'{inst["id"]} #{node}' if node is not None else f'{inst["id"]} start'
        need = inst.get("need")
        gap = inst["bound"] - need + 1 if need is not None else None
        degs = [len(fr.adj[y]) for y in fr.movers]
        kern = fr.kernel()
        k_moves = len([y for y in kern if y in fr.moves])
        k_in = len(kern) - k_moves
        g = bipartite_graph(fr, sorted(kern))
        k_factors = [v[1] for v in g if v[0] == "f"]
        k_out = len([f for f in k_factors if f in set(fr.c_out)])
        comps = components(g)
        largest = max((len(c) for c in comps), default=0)
        edges = sum(len(ns) for ns in g.values()) // 2
        rank = edges - len(g) + len(comps)
        core = two_core(g)
        if core:
            tw_lo = treewidth_lower(core)
            tw_md = treewidth_upper(core, "mindegree")
            tw_mf = treewidth_upper(core, "minfill")
        else:
            tw_lo = tw_md = tw_mf = 0
        fams = Counter()
        for y in kern:
            hp = largest_prime_hub(y, spf)
            if hp is not None and hp[0] in fr.factors:
                fams[hp[0]] += 1
        big = [c for c in fams.values() if c >= 2]
        print(f"| {label} | {inst['subtree']} | {gap} | {len(fr.moves)} | {len(fr.c_in)} | {len(fr.factors)} |"
              f" {len(fr.c_out)} | {fr.freedoms} | {degree_hist(degs)} | {len(kern)} ({k_moves}+{k_in}) |"
              f" {len(k_factors)} ({k_out}) | {len(comps)} ({largest}) | {rank} | {len(core)} |"
              f" {tw_lo}/{tw_md}/{tw_mf} | {len(big)} ({sum(big)}) |", flush=True)


if __name__ == "__main__":
    main()
