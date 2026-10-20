#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Is the GF(2) heaviest set independent in frame h's matroid, as the dump reconstructs it?  The moves' vectors, the
promoted-in members' vectors and the unit vectors of the promoted-out members, over the frame's factors.

python3 gf2check.py instances.jsonl.gz [h]
"""

import sys

from f2lib import Frame2, read_instances, smallest_prime_factor_table


def rank(vectors):
    basis = {}  # pivot bit -> vector
    r = 0
    for v in vectors:
        while v:
            p = v.bit_length() - 1
            if p in basis:
                v ^= basis[p]
            else:
                basis[p] = v
                r += 1
                break
    return r


def main():
    h = int(sys.argv[2]) if len(sys.argv) > 2 else 2
    spf_cache = {}
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        fr = Frame2(inst, h, spf_cache[n])
        chosen = set(inst["chosen"])
        home = dict((x, hh) for x, hh in inst["cands"])
        idx = {f: i for i, f in enumerate(sorted(fr.factors))}
        vec = lambda y: sum(1 << idx[f] for f in fr.adj[y])
        ins = [x for x in chosen if home[x] == h + 1]
        outs = [x for x in chosen if home[x] == h]
        vs = [vec(y) for y in fr.moves] + [vec(y) for y in ins] + [1 << idx[x] for x in outs]
        r = rank(vs)
        movers = set(fr.moves) | set(ins)
        res = fr.residual(movers, fr.factors - set(outs))
        label = f'{inst["id"]} #{inst["node"]}' if inst["node"] is not None else f'{inst["id"]} start'
        print(label, "vectors", len(vs), "rank", r, "independent", r == len(vs), "in", len(ins), "out", len(outs),
              "residual", len(res))


if __name__ == "__main__":
    main()
