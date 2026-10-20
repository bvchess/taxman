#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Checks of the reconstruction: each node's frames are winnable as dumped (peel in Python), and each frame with the
GF(2) heaviest set applied: winnable, or the size of its residual (the movers left in self-covering sets).

python3 check.py instances.jsonl.gz
"""

import sys

from f2lib import Frame2, read_instances, smallest_prime_factor_table


def main():
    spf_cache = {}
    bad = 0
    print("| node | frames winnable as dumped | heaviest set's residual by frame 0/1/2/3/4 |")
    print("|---|---|---|")
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        if n not in spf_cache:
            spf_cache[n] = smallest_prime_factor_table(n)
        spf = spf_cache[n]
        chosen = set(inst["chosen"])
        home = dict((x, h) for x, h in inst["cands"])
        ok = []
        res = []
        for h in range(len(inst["frames"])):
            if h + 1 < len(inst["frames"]):
                fr = Frame2(inst, h, spf)
            else:
                continue
            ok.append(not fr.residual(fr.moves, fr.factors))
            movers = set(fr.moves) | {x for x in chosen if home[x] == h + 1}
            factors = set(fr.factors) - {x for x in chosen if home[x] == h}
            res.append(len(fr.residual(movers, factors)))
        bad += not all(ok)
        label = f'{inst["id"]} #{inst["node"]}' if inst["node"] is not None else f'{inst["id"]} start'
        print(f"| {label} | {all(ok)} | {'/'.join(map(str, res[:5]))} |")
    print("nodes with a frame not winnable as dumped:", bad)


if __name__ == "__main__":
    main()
