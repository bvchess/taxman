#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Could a node branch on a conflict whose members all cost the bound much to leave out?

usage: conflictchoice.py INSTANCES.jsonl[.gz] [SLOPE]

At each node of `wall local --out`: in each frame that the heaviest set leaves unwinnable, minimal conflicts of the
chosen candidates touching the frame, made by a deletion filter in three orders: the search's (try leaving out each
member from the smallest up, so the larger stay; `FrameGraph::conflict_using`), cheapest first (by the member's price,
so the members that cost most stay) and costliest first.  A member's price is the node's bound less its flipped bound (the GF(2) bound
without it).  A conflict's *weight* is the sum over its members of 10^(-price/SLOPE) (SLOPE 291 points a decade): the
subtrees of the node's children, each leaving one member out, against the node's own, if a child's subtree follows
its gap.  The search uses, of the frames' conflicts, the smallest (the first of equals); the table compares that with
the conflict of least weight among all frames and orders.
"""

import statistics
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "the-exact-problem-of-frame-2-2026-10"))
from f2lib import Frame2, read_instances, smallest_prime_factor_table  # noqa: E402


def stuck(fr, cin, cout):
    return bool(fr.residual(set(fr.moves) | cin, set(fr.factors) - cout))


def minimal(fr, cin, cout, order):
    keep_in, keep_out = set(cin), set(cout)
    for x in order:
        if x in keep_in:
            keep_in.discard(x)
            if not stuck(fr, keep_in, keep_out):
                keep_in.add(x)
        elif x in keep_out:
            keep_out.discard(x)
            if not stuck(fr, keep_in, keep_out):
                keep_out.add(x)
    return keep_in | keep_out


def main():
    slope = float(sys.argv[2]) if len(sys.argv) > 2 else 291.0
    spf = {}
    rows = []
    for inst in read_instances(sys.argv[1]):
        n = inst["n"]
        if inst["need"] is None:
            continue
        spf.setdefault(n, smallest_prime_factor_table(n))
        home = {x: h for x, h in inst["cands"]}
        chosen = set(inst["chosen"])
        bound, need = inst["bound"], inst["need"]
        flipped = dict(zip([x for x, _ in inst["cands"]], inst["flipped"]))
        price = {x: bound - flipped[x] for x in chosen if flipped.get(x) is not None}

        def weight(k):
            return sum(10 ** (-price.get(x, 0) / slope) for x in k)

        found = []  # (frame, order, conflict)
        for h in range(len(inst["frames"]) - 1):
            fr = Frame2(inst, h, spf[n])
            cin = {x for x in chosen if home[x] == h + 1}
            cout = {x for x in chosen if home[x] == h and x in fr.factors}
            if not stuck(fr, cin, cout):
                continue
            members = sorted(cin | cout)
            orders = {
                "search": members,
                "cheap first": sorted(members, key=lambda x: (price.get(x, 0), x)),
                "costliest first": sorted(members, key=lambda x: (-price.get(x, 0), x)),
            }
            for name, order in orders.items():
                found.append((h, name, frozenset(minimal(fr, cin, cout, order))))
        if not found:
            continue
        search = min((k for _, name, k in found if name == "search"), key=len)
        best_h, best_name, best = min(found, key=lambda t: weight(t[2]))
        gap = bound - need
        rows.append((gap, weight(search), weight(best), min(price.get(x, 0) for x in search),
                     min(price.get(x, 0) for x in best), len(search), len(best), best_name, best_h))
        label = f'{inst["id"]}#{inst["node"]}' if inst["node"] is not None else f'{inst["id"]}:start'
        print(f"{label} gap {gap} subtree {inst['subtree']}: search's conflict size {len(search)} weight "
              f"{weight(search):.2f} cheapest {min(price.get(x, 0) for x in search)} prices "
              f"{sorted(price.get(x, 0) for x in search)}; best: frame {best_h}, {best_name}, size {len(best)} "
              f"weight {weight(best):.2f} prices {sorted(price.get(x, 0) for x in best)}")
    if rows:
        print(f"\n{len(rows)} nodes: weight of the search's conflict median {statistics.median(r[1] for r in rows):.2f}, "
              f"of the best {statistics.median(r[2] for r in rows):.2f}; ratio best/search median "
              f"{statistics.median(r[2] / r[1] for r in rows):.2f}; cheapest member median "
              f"{statistics.median(r[3] for r in rows)} against {statistics.median(r[4] for r in rows)}")


if __name__ == "__main__":
    main()
