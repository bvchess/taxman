#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Frame 2's graph split by the large prime: the smooth core and one fiber a prime above T = floor(sqrt(n/8)).
Fibers meet only through the core's hubs (a mover s·p lists s, in the core, and the (s/q)·p of its own fiber).

python3 fibers.py instances.jsonl.gz
"""

import sys
from collections import Counter

from f2lib import (Frame2, fibers, induced, read_instances, smallest_prime_factor_table, treewidth_lower,
                   treewidth_upper, two_core)


def main():
    spf_cache = {}
    seen = set()
    print("| node | T | vertices | core (movers/factors) | fibers | largest fiber | vertices in fibers of 1-2 movers |"
          " hubs (core factors with fiber movers) | hub degree max/median | core tw lower/min-fill |"
          " largest fiber tw | tw without the hubs |")
    print("|" + "---|" * 12)
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        spf = spf_cache[n]
        fr = Frame2(inst, 2, spf)
        key = (n, tuple(sorted(fr.movers)), tuple(sorted(fr.factors)))
        if key in seen:
            continue
        seen.add(key)
        t, g, core, fib = fibers(fr, fr.movers, spf)
        core_m = len([v for v in core if v[0] == "m"])
        sizes = sorted((len(vs) for vs in fib.values()), reverse=True)
        small = sum(len(vs) for vs in fib.values() if len([v for v in vs if v[0] == "m"]) <= 2)
        core_set = set(core)
        hubs = [v for v in core if v[0] == "f" and any(w not in core_set for w in g[v])]
        hub_deg = sorted((len([w for w in g[v] if w not in core_set]) for v in hubs), reverse=True)
        cg = two_core(induced(g, core))
        core_tw = (treewidth_lower(cg), treewidth_upper(cg)) if cg else (0, 0)
        fib_tw = max((treewidth_upper(two_core(induced(g, vs))) if two_core(induced(g, vs)) else 0)
                     for vs in fib.values())
        rest = two_core(induced(g, [v for v in g if v not in set(hubs)]))
        rest_tw = treewidth_upper(rest) if rest else 0
        label = f'{inst["id"]} #{inst["node"]}' if inst["node"] is not None else f'{inst["id"]} start'
        med = hub_deg[len(hub_deg) // 2] if hub_deg else 0
        print(f"| {label} | {t} | {len(g)} | {len(core)} ({core_m}/{len(core) - core_m}) | {len(fib)} |"
              f" {sizes[0] if sizes else 0} | {small} | {len(hubs)} | {hub_deg[0] if hub_deg else 0}/{med} |"
              f" {core_tw[0]}/{core_tw[1]} | {fib_tw} | {rest_tw} |", flush=True)


if __name__ == "__main__":
    main()
