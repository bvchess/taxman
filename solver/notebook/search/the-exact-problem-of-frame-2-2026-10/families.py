#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The twin families of frame 2: movers m·p (p their largest prime, to the first power) grouped by the cofactor m, a
factor of the frame (the family's hub).  Also the factors' degrees and the C4 check (no two movers share two factors).

python3 families.py instances.jsonl.gz
"""

import sys
from collections import Counter
from itertools import combinations

from f2lib import Frame2, primes_of, read_instances, smallest_prime_factor_table


def main():
    spf_cache = {}
    seen = set()
    print("| node | movers | in families of 2+ | families of 2+ | largest families (hub: members) |"
          " factor degree max/mean | movers sharing 2 factors |")
    print("|" + "---|" * 7)
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
        fam = {}
        for y in fr.movers:
            p, e = primes_of(y, spf)[-1]
            if e == 1 and y // p in fr.factors:
                fam.setdefault(y // p, []).append(y)
        big = {m: ys for m, ys in fam.items() if len(ys) >= 2}
        top = sorted(big.items(), key=lambda kv: -len(kv[1]))[:4]
        degs = [len(us) for us in fr.users.values()]
        pairs = Counter()
        for f, us in fr.users.items():
            for a, b in combinations(sorted(us), 2):
                pairs[(a, b)] += 1
        doubled = sum(1 for c in pairs.values() if c >= 2)
        label = f'{inst["id"]} #{inst["node"]}' if inst["node"] is not None else f'{inst["id"]} start'
        print(f"| {label} | {len(fr.movers)} | {sum(len(v) for v in big.values())} | {len(big)} |"
              f" {', '.join(f'{m}: {len(ys)}' for m, ys in top)} | {max(degs)}/{sum(degs) / len(degs):.2f} |"
              f" {doubled} |", flush=True)


if __name__ == "__main__":
    main()
