#!/usr/bin/env python3
"""Does some home split a node's conflicts into two sides that do not interact?  The tables of
solver/notebook/search/decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md, from the lines of `wall
decompose` (solver/frame/examples/wall/decompose.rs defines every field):

    python3 -I analyze.py FILE... [--beta B]

A line is a node of a proof that today's bound leaves open (the cut ones are counted and left out), with the subtree
that the search makes of it (`subtree`, the engine's steps; a solve that was stopped counts for the steps it made).
Its conflicts are seen from every *separator* home h (fixing the open candidates of home h leaves the conflicts of
frames below h on one side and those of frames from h on the other): `cores` are the cores of today's heaviest set,
`learned` the nogoods the subtree's searcher held at its end.  Once the candidates of home h are fixed a conflict
lies on the *left* (members below h, with or without members at h), on the *right*, or has members only at h; none
has members on both sides (`cross`, which the program checks to be 0).  The separator's *width* is the number of
distinct open candidates of home h that some conflict holds.

A home *splits* a node if both sides hold a conflict, *balanced* if the smaller side holds at least `--beta` of the
conflicts on the two sides (default 0.2), and *cheap* at width w if its width is at most w.  The shares are of the
nodes weighted by their subtrees (the work a node costs).  The oracle table is of the nodes with a known best
completion (`ideal` is the steps of the proof from its own optimum; `tl + tr` those of the two sides apart under the
separator that the best completion has, the cheapest home taken); the AND/OR table is of the nodes the AND/OR search
was run on.
"""

import argparse
import gzip
import json
import statistics
import sys
from pathlib import Path

# The order of a spread's `places`.
LEFT, RIGHT, TOUCH_LEFT, TOUCH_RIGHT, TOUCH_ONLY, CROSS = range(6)
BINS = [(1, 9), (10, 99), (100, 999), (1_000, 9_999), (10_000, 99_999), (100_000, 10**12)]
WIDTHS = (4, 8, 16)
EVERYTHING = (1, 10**12)


def name_of(bounds):
    lo, hi = bounds
    return "all" if bounds == EVERYTHING else (f"{lo:,}-{hi:,}" if hi < 10**12 else f"{lo:,}+")


def read(path):
    """The games in a file (plain or gzipped; each game starts with its `game` line): a list of (game line, node
    lines that were open, number of nodes that were cut)."""
    path = Path(path)
    text = gzip.decompress(path.read_bytes()).decode() if path.suffix == ".gz" else path.read_text()
    games = []
    for text in text.splitlines():
        try:
            line = json.loads(text)
        except ValueError:  # a line cut short when a run was killed
            continue
        if line.get("kind") == "game":
            games.append((line, [], 0))
        elif line.get("kind") == "node" and games:
            game, nodes, cut = games[-1]
            if line.get("cut"):
                games[-1] = (game, nodes, cut + 1)
            else:
                nodes.append(line)
    return games


def separators(spread):
    """For each home h that splits the conflicts of `spread`: (h, left, right, touching, width), where left and right
    are the conflicts on each side once home h is fixed and touching those that have a member at h."""
    out = []
    for h, places in enumerate(spread["places"], start=1):
        left, right = places[LEFT] + places[TOUCH_LEFT], places[RIGHT] + places[TOUCH_RIGHT]
        touching = places[TOUCH_LEFT] + places[TOUCH_RIGHT] + places[TOUCH_ONLY]
        if left and right:
            out.append((h, left, right, touching, spread["width"][h]))
    return out


def balanced(split, beta):
    _, left, right, _, _ = split
    return min(left, right) >= beta * (left + right)


def verdicts(node, beta, kind):
    """What a node's conflicts of `kind` ('cores' or 'learned') allow."""
    spread = node[kind]
    splits = separators(spread)
    good = [s for s in splits if balanced(s, beta)]
    best = min(good, key=lambda s: (s[4], -min(s[1], s[2])), default=None)
    return {
        "splits": bool(splits),
        "balanced": bool(good),
        "free": any(s[4] == 0 for s in splits),
        "cheap": {w: any(s[4] <= w for s in good) for w in WIDTHS},
        "best": best,
        "crossing": sum(places[CROSS] for places in spread["places"]),
        "in_frames_1_3": sum(spread["frames"][1:4]) / max(spread["n"], 1),
        "touching": (best[3] if best else 0, spread["n"]),
    }


def share(rows, test):
    """The share of the subtree steps of `rows` ((node, verdict) pairs) whose verdict passes `test`."""
    total = sum(x["subtree"] for x, _ in rows)
    return sum(x["subtree"] for x, v in rows if test(v)) / total if total else float("nan")


def pct(value):
    return f"{100 * value:.0f}%"


def table(title, nodes, beta):
    nodes = [x for x in nodes if not x["leaf"]]
    print(f"\n{title}: {len(nodes)} open nodes with conflicts, {sum(x['subtree'] for x in nodes):,} steps\n")
    print("| subtree | nodes | steps | conflicts in frames 1-3, cores / learned | some home splits, cores / learned "
          f"| balanced | free | cheap at width {' / '.join(map(str, WIDTHS))} | width of the cheapest balanced, median "
          "| cores touching it, median | home 2's open candidates, median |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for bounds in BINS + [EVERYTHING]:
        part = [x for x in nodes if bounds[0] <= x["subtree"] <= bounds[1]]
        if not part:
            continue
        cores = [(x, verdicts(x, beta, "cores")) for x in part]
        learned = [(x, verdicts(x, beta, "learned")) for x in part]
        total = sum(x["subtree"] for x in part)
        mass = lambda rows: sum(v["in_frames_1_3"] * x["subtree"] for x, v in rows) / total
        cheap = " / ".join(pct(share(learned, lambda v, w=w: v["cheap"][w])) for w in WIDTHS)
        widths = [v["best"][4] for _, v in learned if v["best"]]
        touching = [v["touching"] for _, v in cores if v["best"]]
        print(
            f"| {name_of(bounds)} | {len(part)} | {total:,} | {pct(mass(cores))} / {pct(mass(learned))} | "
            f"{pct(share(cores, lambda v: v['splits']))} / {pct(share(learned, lambda v: v['splits']))} | "
            f"{pct(share(learned, lambda v: v['balanced']))} | {pct(share(learned, lambda v: v['free']))} | {cheap} | "
            f"{statistics.median(widths) if widths else '-'} | "
            f"{statistics.median(t for t, _ in touching) if touching else '-'} of "
            f"{statistics.median(n for _, n in touching) if touching else '-'} | "
            f"{statistics.median(x['open'][2] for x in part):.0f} |"
        )
    crossing = sum(v["crossing"] for x in nodes for v in (verdicts(x, beta, "cores"), verdicts(x, beta, "learned")))
    print(f"\nConflicts that cross a separator: {crossing}.")


def oracle(node):
    """The cheapest oracle separator of a node with a best completion: (its entry, the ideal proof's steps)."""
    sides = [s for s in node.get("sides", []) if s["completed"]]
    if "ideal" not in node or not node["ideal"]["completed"] or not sides:
        return None
    return min(sides, key=lambda s: s["tl"] + s["tr"]), node["ideal"]["steps"]


def oracle_table(nodes):
    rows = [(x, oracle(x)) for x in nodes if not x["leaf"]]
    rows = [(x, o) for x, o in rows if o]
    if not rows:
        return
    print(f"\nThe oracle separator: {len(rows)} nodes with a best completion\n")
    print("| subtree | nodes | steps | ideal proof | sides apart (tl + tr) | sides / ideal, median "
          "| sides under a tenth of ideal | gap the separator resolves, median "
          "| bound's error at its top home, share of the whole, median | top home (nodes at home 1 / 2 / 3 / 4+) |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for bounds in BINS + [EVERYTHING]:
        part = [(x, o) for x, o in rows if bounds[0] <= x["subtree"] <= bounds[1]]
        if not part:
            continue
        total = sum(x["subtree"] for x, _ in part)
        ratios = [(o[0]["tl"] + o[0]["tr"]) / max(o[1], 1) for _, o in part]
        tenth = sum(x["subtree"] for (x, _), r in zip(part, ratios) if r < 0.1) / total
        resolved, top_share, tops = [], [], [0, 0, 0, 0]
        for x, o in part:
            excess = x["excess_homes"]
            gap = sum(excess)
            if gap > 0:
                resolved.append((gap - o[0]["gl"] - o[0]["gr"]) / gap)
            top = max(range(len(excess)), key=lambda h: abs(excess[h]))
            top_share.append(abs(excess[top]) / max(sum(abs(e) for e in excess), 1))
            tops[min(top, 4) - 1] += 1
        print(
            f"| {name_of(bounds)} | {len(part)} | {total:,} | {sum(o[1] for _, o in part):,} | "
            f"{sum(o[0]['tl'] + o[0]['tr'] for _, o in part):,} | {statistics.median(ratios):.4f} | {pct(tenth)} | "
            f"{pct(statistics.median(resolved)) if resolved else '-'} | {pct(statistics.median(top_share))} | "
            f"{' / '.join(map(str, tops))} |"
        )


def and_or_table(nodes):
    rows = [x for x in nodes if "and_or" in x]
    if not rows:
        return
    print(f"\nThe AND/OR search with the best separator, {len(rows)} nodes\n")
    print("| node | subtree | separator | order | OR evaluations | finished | survivors | sides' steps, sample mean "
          "| OR evaluations / subtree |")
    print("|---|---|---|---|---|---|---|---|---|")
    for x in rows:
        for name, order in (("and_or", "largest first"), ("and_or_conflict", "by conflict")):
            run = x[name]
            sides = run["side_steps"]
            print(
                f"| {x['id']} | {x['subtree']:,} | home {run['h']} | {order} | {run['or_nodes']:,} | "
                f"{'yes' if run['complete'] else 'no (cap)'} | {run['survivors']:,}{'' if run['complete'] else '+'} | "
                f"{sum(sides) / len(sides) if sides else 0:.1f} | {run['or_nodes'] / x['subtree']:.1f}x |"
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", nargs="+")
    parser.add_argument("--beta", type=float, default=0.2)
    args = parser.parse_args()
    for path in args.files:
        for game, nodes, cut in read(path):
            print(
                f"\n## Game {game.get('n')}: {game.get('candidates')} candidates by home {game.get('open')}, "
                f"{len(nodes)} open nodes, {cut} cut"
            )
            table(f"Game {game.get('n')}", nodes, args.beta)
            oracle_table(nodes)
            and_or_table(nodes)


if __name__ == "__main__":
    sys.exit(main())
