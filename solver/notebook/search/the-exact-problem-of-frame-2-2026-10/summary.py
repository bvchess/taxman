#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The entry's tables: frame 2 at the nodes dumped by `wall local --out`, summarized by search and by band of gap.

python3 summary.py instances.jsonl.gz
"""

import statistics
import sys
from collections import Counter, defaultdict

from f2lib import (Frame2, bipartite_graph, components, fibers, induced, primes_of, read_instances,
                   smallest_prime_factor_table, treewidth_lower, treewidth_upper, two_core)
from fixing import analyze
from gf2check import rank


def rng(values, fmt="{:.0f}"):
    values = list(values)
    if not values:
        return "-"
    lo, hi, med = min(values), max(values), statistics.median(values)
    if lo == hi:
        return fmt.format(lo)
    return f"{fmt.format(med)} ({fmt.format(lo)}-{fmt.format(hi)})"


def measure(inst, spf):
    fr = Frame2(inst, 2, spf)
    m = {"search": inst["id"], "node": inst["node"], "n": inst["n"]}
    need = inst.get("need")
    m["gap"] = inst["bound"] - need + 1 if need is not None else None
    m["moves"], m["in"], m["factors"], m["out"] = len(fr.moves), len(fr.c_in), len(fr.factors), len(fr.c_out)
    m["freedoms"] = fr.freedoms
    degs = Counter(min(len(fr.adj[y]), 4) for y in fr.movers)
    total = len(fr.movers)
    m["deg"] = [degs.get(k, 0) / total for k in (1, 2, 3, 4)]
    g = bipartite_graph(fr, fr.movers)
    comps = components(g)
    edges = sum(len(ns) for ns in g.values()) // 2
    m["vertices"], m["comps"], m["rank"] = len(g), len(comps), edges - len(g) + len(comps)
    core2 = two_core(g)
    m["tw"] = (treewidth_lower(core2), treewidth_upper(core2, "mindegree"), treewidth_upper(core2, "minfill"))
    t, g2, core, fib = fibers(fr, fr.movers, spf)
    core_set = set(core)
    hubs = [v for v in core if v[0] == "f" and any(w not in core_set for w in g2[v])]
    m["T"], m["core"], m["fibers"] = t, len(core), len(fib)
    m["fiber_max"] = max(len(vs) for vs in fib.values())
    m["fiber_tw"] = max((treewidth_upper(two_core(induced(g2, vs))) if two_core(induced(g2, vs)) else 0)
                        for vs in fib.values())
    cg = two_core(induced(g2, core))
    m["core_tw"] = (treewidth_lower(cg), treewidth_upper(cg))
    m["hubs"] = len(hubs)
    rest = two_core(induced(g2, [v for v in g2 if v not in set(hubs)]))
    m["rest_tw"] = treewidth_upper(rest) if rest else 0
    fam = defaultdict(list)
    for y in fr.movers:
        p, e = primes_of(y, spf)[-1]
        if e == 1 and y // p in fr.factors:
            fam[y // p].append(y)
    big = [len(v) for v in fam.values() if len(v) >= 2]
    m["families"], m["in_families"] = len(big), sum(big) / total
    m["largest_families"] = sorted(((len(v), k) for k, v in fam.items()), reverse=True)[:4]
    if need is not None and "flipped" in inst:
        m["fix"] = analyze(inst, spf)
    chosen = set(inst["chosen"])
    home = dict((x, h) for x, h in inst["cands"])
    ins = [x for x in chosen if home[x] == 3]
    outs = [x for x in chosen if home[x] == 2]
    idx = {f: i for i, f in enumerate(sorted(fr.factors))}
    vec = lambda y: sum(1 << idx[f] for f in fr.adj[y])
    vs = [vec(y) for y in fr.moves] + [vec(y) for y in ins] + [1 << idx[x] for x in outs]
    m["heavy_closed"] = len(ins) + len(outs) == fr.freedoms
    m["heavy_indep"] = rank(vs) == len(vs)
    m["heavy_residual"] = len(fr.residual(set(fr.moves) | set(ins), fr.factors - set(outs)))
    m["heavy_inout"] = (len(ins), len(outs))
    return m


def main():
    spf_cache = {}
    rows = []
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        rows.append(measure(inst, spf_cache[n]))
    by_search = defaultdict(list)
    for r in rows:
        by_search[r["search"]].append(r)

    print("## Sizes, by search (median and range over its nodes)\n")
    print("| search | nodes | moves | home-3 candidates | factors | home-2 candidates among them | freedoms |"
          " movers with 1 / 2 / 3 / 4+ live factors | vertices | cycle rank |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for s, rs in by_search.items():
        deg = " / ".join(f"{statistics.median(r['deg'][k] for r in rs):.2f}" for k in range(4))
        print(f"| {s} | {len(rs)} | {rng(r['moves'] for r in rs)} | {rng(r['in'] for r in rs)} |"
              f" {rng(r['factors'] for r in rs)} | {rng(r['out'] for r in rs)} | {rng(r['freedoms'] for r in rs)} |"
              f" {deg} | {rng(r['vertices'] for r in rs)} | {rng(r['rank'] for r in rs)} |")

    print("\n## Treewidth and the arithmetic shape, by search\n")
    print("| search | components | treewidth: lower bound; min-degree; min-fill | T | smooth core (vertices) |"
          " core treewidth: lower; min-fill | fibers | largest fiber (vertices) | fiber treewidth at most |"
          " hubs | treewidth without the hubs, min-fill | twin families of 2+; share of movers in them |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|")
    for s, rs in by_search.items():
        tw = "; ".join(rng(r["tw"][k] for r in rs) for k in range(3))
        ctw = "; ".join(rng(r["core_tw"][k] for r in rs) for k in range(2))
        print(f"| {s} | {rng(r['comps'] for r in rs)} | {tw} | {rng(r['T'] for r in rs)} |"
              f" {rng(r['core'] for r in rs)} | {ctw} | {rng(r['fibers'] for r in rs)} |"
              f" {rng(r['fiber_max'] for r in rs)} | {max(r['fiber_tw'] for r in rs)} | {rng(r['hubs'] for r in rs)} |"
              f" {rng(r['rest_tw'] for r in rs)} | {rng(r['families'] for r in rs)};"
              f" {rng((r['in_families'] for r in rs), '{:.2f}')} |")
    print("\nLargest families (hub: members), first node of each search:",
          "; ".join(f"{s}: " + ", ".join(f"{m}·p: {c}" for c, m in rs[0]["largest_families"])
                    for s, rs in by_search.items()))

    print("\n## The heaviest set in frame 2, by search\n")
    print("| search | nodes | fills frame 2's freedoms | independent in its GF(2) matroid | promoted in; out |"
          " movers left in the peel's residual |")
    print("|---|---|---|---|---|---|")
    for s, rs in by_search.items():
        print(f"| {s} | {len(rs)} | {sum(r['heavy_closed'] for r in rs)} | {sum(r['heavy_indep'] for r in rs)} |"
              f" {rng(r['heavy_inout'][0] for r in rs)}; {rng(r['heavy_inout'][1] for r in rs)} |"
              f" {rng(r['heavy_residual'] for r in rs)} |")

    print("\n## After fixing by the GF(2) bound, by band of gap\n")
    print("| gap | nodes | candidates | fixed in; out | free | frame 2: optional movers; optional removals |"
          " kernel movers | kernel 2-core (vertices) | treewidth: lower bound; min-fill |")
    print("|---|---|---|---|---|---|---|---|---|")
    fixed = [r for r in rows if r.get("fix")]
    for lo, hi in [(0, 399), (400, 1000), (1001, 10 ** 6)]:
        rs = [r for r in fixed if lo <= r["gap"] <= hi]
        if not rs:
            continue
        f = [r["fix"] for r in rs]
        print(f"| {lo}-{hi if hi < 10 ** 6 else ''} | {len(rs)} |"
              f" {rng(x['free'] + x['fixed_in'] + x['fixed_out'] for x in f)} |"
              f" {rng(x['fixed_in'] for x in f)}; {rng(x['fixed_out'] for x in f)} | {rng(x['free'] for x in f)} |"
              f" {rng(x['opt_in'] for x in f)}; {rng(x['opt_out'] for x in f)} | {rng(x['kernel'] for x in f)} |"
              f" {rng(x['core'] for x in f)} | {rng(x['tw'][0] for x in f)}; {rng(x['tw'][1] for x in f)} |")


if __name__ == "__main__":
    main()
