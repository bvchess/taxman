#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""How many disjoint conflicts does a node's GF(2) heaviest set hold, and what would their prices add up to?

usage: disjoint.py INSTANCES.jsonl[.gz] [F2LIB_DIR]

For each node of `wall local --out` (its frames, candidates, heaviest set `chosen` and flipped bounds): in each frame
h = 0..7, the heaviest set's configuration (the frame's moves plus the chosen candidates of home h + 1 as movers, its
factors less the chosen candidates of home h); if the peel leaves a residual, a minimal conflict is cut from the chosen
candidates touching the frame by a deletion filter, its members are taken back (movers out, factors in), and the next
one is cut from what is left, until the frame is winnable.  The conflicts so found are disjoint.  A member's price is
the node's bound less its flipped bound (the GF(2) bound without it); a conflict's price is its cheapest member's.  The
sum of the prices of the disjoint conflicts is what a bound that charged each of them independently would take off.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "the-exact-problem-of-frame-2-2026-10"))
from f2lib import Frame2, read_instances, smallest_prime_factor_table  # noqa: E402


def config(fr, chosen_in, chosen_out):
    movers = set(fr.moves) | chosen_in
    factors = set(fr.factors) - chosen_out
    return movers, factors


def stuck(fr, cin, cout):
    movers, factors = config(fr, cin, cout)
    return bool(fr.residual(movers, factors))


def minimal_conflict(fr, cin, cout):
    """A minimal subset (as (in, out)) of the chosen candidates touching the frame that leaves it unwinnable."""
    keep_in, keep_out = set(cin), set(cout)
    for x in sorted(cin, reverse=True):
        keep_in.discard(x)
        if not stuck(fr, keep_in, keep_out):
            keep_in.add(x)
    for x in sorted(cout, reverse=True):
        keep_out.discard(x)
        if not stuck(fr, keep_in, keep_out):
            keep_out.add(x)
    return keep_in, keep_out


def main():
    spf = {}
    rows = []
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        spf.setdefault(n, smallest_prime_factor_table(n))
        home = {x: h for x, h in inst["cands"]}
        chosen = set(inst["chosen"])
        bound, need = inst["bound"], inst["need"]
        if need is None:
            continue
        cand_list = [x for x, _ in inst["cands"]]
        flipped = dict(zip(cand_list, inst["flipped"]))
        price = {x: (bound - flipped[x]) if flipped.get(x) is not None else None for x in chosen}
        nframes = len(inst["frames"])
        per_frame = []
        total = 0
        sizes = []
        for h in range(nframes - 1):
            fr = Frame2(inst, h, spf[n])
            cin = {x for x in chosen if home[x] == h + 1}
            cout = {x for x in chosen if home[x] == h and x in fr.factors}
            res = len(fr.residual(*config(fr, cin, cout)))
            found = []
            while stuck(fr, cin, cout):
                kin, kout = minimal_conflict(fr, cin, cout)
                members = kin | kout
                if not members:
                    break
                ps = [price[x] for x in members if price.get(x) is not None]
                found.append((len(members), min(ps) if ps else 0))
                cin -= kin
                cout -= kout
            per_frame.append((h, res, found))
            total += sum(p for _, p in found)
            sizes += [s for s, _ in found]
        label = f'{inst["id"]}#{inst["node"]}' if inst["node"] is not None else f'{inst["id"]}:start'
        gap = bound - need
        desc = "; ".join(f"f{h}: res {r}, {len(f)} conflicts, prices {sorted(p for _, p in f)}"
                         for h, r, f in per_frame if r)
        print(f"{label} depth {inst['depth']} subtree {inst['subtree']} gap {gap} |H| {len(chosen)} "
              f"sum of prices {total} ({total / gap if gap > 0 else float('nan'):.2f} of the gap)\n    {desc}")
        rows.append((gap, total, inst["subtree"]))
    if rows:
        import statistics
        print("median share of the gap:", statistics.median(t / g for g, t, _ in rows if g > 0))


if __name__ == "__main__":
    main()
