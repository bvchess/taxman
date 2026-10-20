#!/usr/bin/env python3
"""At which nodes is the strong bound worth computing?  The tables of
solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/README.md, from the lines of `wall choice`
(examples/wall/choice.rs defines every field):

    python3 -I analyze.py DIR-OR-FILE... [--model fit|floor] [--steps K]

A line is a node that today's bound leaves open (`gap` >= 0), with the subtree today's search makes of it (`subtree`,
the engine's steps) and the strong bound refitted at it: the Lagrangian bound over nogoods, `--node-steps` subgradient
steps from the root's multipliers (`strong`; `sub` with the nogoods of the node's own solve too, which is hindsight;
`root` below is the first step alone, the root's multipliers kept at the node), and with `--a1` the two A1 bounds
(`a1`: `a1r2` below is the warm refit of at most 20 pricings, `a1full` the node's own master with the exact checks and
the separation, within its seconds; a node where it ran out of them without cutting is *capped*).  A node on the way
to the optimum (`source` path) only checks the bound's validity and is left out.

A *policy* computes the strong bound at the nodes whose subtree is forecast above N steps, and only there.  The
forecast is log10(subtree) = a + b * gap, `--model fit` (a and b fitted to the nodes whose solve completed) or `floor`
(b = 1/147, the cost of a search under a floor: a decade of steps per 147 points; a fitted to the nodes), so a forecast
above N is a gap above a threshold.  Each node is a problem of its own: its search costs subtree * c CPU seconds (c the
game's seconds a step, from its completed solves), and the policy replaces that by the strong bound's cost (the
matroids built afresh and the steps of the fit, up to the cut) when the bound cuts the node, and adds it when not.  So
the *net saving* of a policy over a set of nodes is the CPU seconds of the subtrees it cuts less the CPU seconds of
every strong bound it computes (an A1 bound's cost is its frame matroids, `a1.build_s`, and its `s`), and its *share*
is that over what searching every node costs.  The nodes are a sample
(of a proof's nodes, spread over gaps), so the shares are of the sample, and the sample's mix is the lab's: see the
README.  A solve that `--cap` stopped counts for the steps it made (a lower bound on its subtree).
"""

import argparse
import json
import math
import statistics
import sys
from pathlib import Path

FLOOR_SLOPE = 1 / 147
VARIANTS = ("root", "strong", "sub")
A1_VARIANTS = ("a1r2", "a1full")
ALL_VARIANTS = VARIANTS + A1_VARIANTS
GRID = [0] + [10 ** (k / 2) for k in range(2, 15)]


def read(paths):
    """The lines of the files (or the .jsonl files of the directories) given, a unit once."""
    files = []
    for path in map(Path, paths):
        files.extend(sorted(path.glob("*.jsonl")) if path.is_dir() else [path])
    lines, seen = [], set()
    for file in files:
        for text in file.read_text().splitlines():
            try:
                line = json.loads(text)
            except ValueError:  # a line cut short when a run was killed
                continue
            if line.get("id") and line["id"] not in seen:
                seen.add(line["id"])
                lines.append(line)
    return lines


def outcome(line, fit, steps):
    """Whether the first `steps` steps of `fit` cut the node, and the CPU seconds they cost with the matroids."""
    lows, ts = fit["lows"][:steps], fit["ts"]
    cut = next((i + 1 for i, low in enumerate(lows) if low < line["need"]), None)
    taken = cut or len(lows)
    return cut is not None, line["graph_s"] + fit["s"] * ts[taken - 1] / (ts[-1] or 1e-12)


def a1_outcome(line, name):
    """Whether the A1 bound `name` (`r2` or `full`) cuts the node, and the CPU seconds it cost with its matroids."""
    bound = line["a1"][name]
    return bound["cut"], line["a1"]["build_s"] + bound["s"]


def population(line):
    if line["source"] == "suite":
        return "suite (3300-3441)"
    return "made (2700-3267)" if line["n"] >= 2000 else "probe (936, 1215)"


def rates(nodes):
    """The CPU seconds a step costs, by game: the completed solves' seconds over their steps (all games' if none)."""
    by_game, everywhere = {}, [0.0, 0]
    for node in nodes:
        by_game.setdefault(node["n"], [0.0, 0])
        if node["completed"] and node["subtree"] >= 1000:
            for total in (by_game[node["n"]], everywhere):
                total[0] += node["solve_cpu_s"]
                total[1] += node["subtree"]
    overall = everywhere[0] / everywhere[1] if everywhere[1] else 1 / 3000
    return {n: (seconds / steps if steps else overall) for n, (seconds, steps) in by_game.items()}


def nodes_of(lines, steps):
    """The open nodes with a solve: for each variant whether it cuts and what it costs, and what searching costs."""
    nodes = []
    for line in lines:
        if line["source"] == "path" or line["subtree"] is None or line["gap"] < 0:
            continue
        node = dict(line, outcomes={}, population=population(line))
        node["outcomes"]["root"] = outcome(line, line["strong"], 1)
        node["outcomes"]["strong"] = outcome(line, line["strong"], steps)
        if line.get("sub"):
            node["outcomes"]["sub"] = outcome(line, line["sub"], steps)
        if line.get("a1"):
            node["outcomes"]["a1r2"] = a1_outcome(line, "r2")
            node["outcomes"]["a1full"] = a1_outcome(line, "full")
        nodes.append(node)
    rate = rates(nodes)
    for node in nodes:
        node["search_s"] = node["subtree"] * rate[node["n"]]
    return nodes


def regression(xs, ys):
    """Least squares y = a + b x: a, b and r squared."""
    mx, my = statistics.fmean(xs), statistics.fmean(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx if sxx else 0.0
    a = my - b * mx
    syy = sum((y - my) ** 2 for y in ys)
    r2 = 1 - sum((y - a - b * x) ** 2 for x, y in zip(xs, ys)) / syy if syy else 0.0
    return a, b, r2


def forecast(nodes, model):
    """(a, b, r2) of log10(subtree) = a + b * gap, from the nodes whose solve completed."""
    done = [n for n in nodes if n["completed"]]
    if len(done) < 3:
        return 0.0, FLOOR_SLOPE, 0.0
    xs, ys = [n["gap"] for n in done], [math.log10(max(n["subtree"], 1)) for n in done]
    a, b, r2 = regression(xs, ys)
    if model == "floor":
        return statistics.fmean(y - FLOOR_SLOPE * x for x, y in zip(xs, ys)), FLOOR_SLOPE, r2
    return a, b, r2


def median(values):
    return statistics.median(values) if values else float("nan")


def share(nodes, variant):
    have = [n for n in nodes if variant in n["outcomes"]]
    return sum(n["outcomes"][variant][0] for n in have) / len(have) if have else float("nan")


def table(title, header, rows):
    print(f"\n{title}\n")
    widths = [max(len(str(cell)) for cell in column) for column in zip(header, *rows)]
    for row in [header, *rows]:
        print("  ".join(str(cell).rjust(width) for cell, width in zip(row, widths)))


def percent(x):
    return "-" if math.isnan(x) else f"{100 * x:.0f}%"


def seconds(x):
    return f"{x:.3g}"


def sizes(nodes):
    """The sample: per game and source, its nodes, subtrees and times."""
    rows = []
    for game, source in sorted({(n["n"], n["source"]) for n in nodes}):
        group = [n for n in nodes if (n["n"], n["source"]) == (game, source)]
        solved = [n["subtree"] for n in group if n["completed"]]
        micro = next((n["search_s"] / n["subtree"] * 1e6 for n in group if n["subtree"]), float("nan"))
        cost = [n["outcomes"]["strong"][1] / n["today_s"] for n in group if n["today_s"] > 0]
        rows.append([
            game, source, len(group), len(group) - len(solved), min(n["gap"] for n in group),
            max(n["gap"] for n in group), f"{median(solved):.0f}", max(solved, default=0), f"{micro:.0f}",
            f"{median([n['today_s'] for n in group]) * 1e3:.2f}", f"{median(cost):.0f}",
        ])
    header = ["n", "source", "nodes", "capped", "gap min", "gap max", "subtree med", "subtree max", "us/step",
              "today ms", "strong/today"]
    table("The sample: open nodes by game and source (strong/today: median of the fit's seconds over today's)",
          header, rows)


def spent_by(nodes, variant):
    """The CPU seconds of `variant` at the nodes that have it."""
    return [n["outcomes"][variant][1] for n in nodes if variant in n["outcomes"]]


def by_gap(nodes):
    edges = [0, 50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 10 ** 9]
    rows = []
    for lo, hi in zip(edges, edges[1:]):
        group = [n for n in nodes if lo <= n["gap"] < hi]
        if group:
            done = [n["subtree"] for n in group if n["completed"]]
            spent = [n["outcomes"]["strong"][1] for n in group]
            rows.append([f"{lo}+" if hi > 10 ** 8 else f"{lo}-{hi - 1}", len(group)]
                        + [percent(share(group, v)) for v in ALL_VARIANTS]
                        + [f"{median(done):.0f}", seconds(median(spent))]
                        + [seconds(median(spent_by(group, v))) for v in A1_VARIANTS])
    table("Share of the open nodes each strong bound cuts, by gap",
          ["gap", "nodes", *ALL_VARIANTS, "subtree med", "strong s med", *(f"{v} s med" for v in A1_VARIANTS)], rows)


def by_subtree(nodes):
    rows = []
    for k in range(8):
        group = [n for n in nodes if 10 ** k <= n["subtree"] < 10 ** (k + 1)]
        if group:
            rows.append([f"1e{k}", len(group)] + [percent(share(group, v)) for v in ALL_VARIANTS]
                        + [f"{median([n['gap'] for n in group]):.0f}"])
    table("Share of the open nodes each strong bound cuts, by the decade of their subtree",
          ["subtree", "nodes", *ALL_VARIANTS, "gap med"], rows)


def gap_above(threshold, model):
    """The gap above which the forecast subtree exceeds `threshold` steps (log10 N = a + b gap); all gaps for 0."""
    a, b, _ = model
    return -math.inf if threshold == 0 else (math.log10(threshold) - a) / b


def evaluate(nodes, variant, floor):
    """The policy of computing the strong bound `variant` at the nodes with a gap above `floor`: the nodes it is
    computed at, those it cuts, and its net saving in CPU seconds."""
    applied = [n for n in nodes if n["gap"] > floor and variant in n["outcomes"]]
    cut = [n for n in applied if n["outcomes"][variant][0]]
    return len(applied), len(cut), sum(n["search_s"] for n in cut) - sum(n["outcomes"][variant][1] for n in applied)


def policies(nodes, model, label, variants=VARIANTS):
    """The policies N = GRID over `nodes`: for each of `variants` the nodes it cuts and its net saving, and the best N."""
    total = sum(n["search_s"] for n in nodes)
    if total <= 0 or not any(v in n["outcomes"] for n in nodes for v in variants):
        return
    rows, best = [], {v: (0.0, None) for v in variants}
    for threshold in GRID:
        floor = gap_above(threshold, model)
        row = [f"{threshold:.0e}" if threshold else "0", f"{floor:.0f}" if threshold else "-"]
        row.append(evaluate(nodes, variants[0], floor)[0])
        for variant in variants:
            _, cut, net = evaluate(nodes, variant, floor)
            row += [cut, seconds(net), percent(net / total)]
            if net > best[variant][0]:
                best[variant] = (net, threshold)
        rows.append(row)
    header = ["N", "gap above", "applied"] + [f"{v} {col}" for v in variants for col in ("cut", "net s", "share")]
    table(f"Policies: the strong bound where the forecast subtree exceeds N ({label}: {len(nodes)} nodes, searching "
          f"them all costs {seconds(total)} CPU s; applied: the nodes {variants[0]} is computed at)", header, rows)
    for variant in variants:
        net, threshold = best[variant]
        text = "none (every policy loses)" if threshold is None else (
            f"N = {threshold:.0e}, a net saving of {seconds(net)} CPU s, {percent(net / total)} of the total")
        print(f"best for {variant}: {text}")


def closed(node, name):
    """The share of the node's excess over a cut (`gap + 1`) that the A1 bound `name` takes off today's bound, at most 1."""
    return min(1.0, (node["today"] - node["a1"][name]["bound"]) / (node["gap"] + 1))


def a1_ends(nodes):
    """How the A1 bounds ended at the open nodes, by gap: cut, capped (out of seconds or pricings, not cut), and the
    median share of the excess they close; and the CPU seconds of the dearest."""
    have = [n for n in nodes if n.get("a1")]
    if not have:
        return
    edges = [0, 50, 100, 200, 300, 400, 500, 700, 10 ** 9]
    rows = []
    for lo, hi in zip(edges, edges[1:]):
        group = [n for n in have if lo <= n["gap"] < hi]
        if not group:
            continue
        row = [f"{lo}+" if hi > 10 ** 8 else f"{lo}-{hi - 1}", len(group)]
        for name in ("r2", "full"):
            ends = [n["a1"][name] for n in group]
            row += [percent(sum(e["cut"] for e in ends) / len(ends)),
                    percent(sum(e["capped"] and not e["cut"] for e in ends) / len(ends)),
                    percent(median([closed(n, name) for n in group])),
                    seconds(max(e["s"] for e in ends))]
        rows.append(row)
    table("The A1 bounds: share of the open nodes cut, capped without a cut, the median share of the excess closed, "
          "the dearest bound's CPU s, by gap",
          ["gap", "nodes", *(f"{name} {col}" for name in ("r2", "full") for col in ("cut", "capped", "closed", "max s"))],
          rows)


def report(lines, steps, model_name):
    nodes = nodes_of(lines, steps)
    if not nodes:
        sys.exit("no open node with a solve in the lines")
    done = sum(n["completed"] for n in nodes)
    print(f"{len(lines)} lines, {len(nodes)} open nodes with a solve, {len(nodes) - done} capped")
    model = forecast(nodes, model_name)
    a, b, r2 = model
    print(f"forecast ({model_name}): log10(subtree) = {a:.2f} + {b:.5f} gap, a decade per {1 / b:.0f} points, "
          f"r2 {r2:.3f}, over the {done} completed solves")
    sizes(nodes)
    by_gap(nodes)
    by_subtree(nodes)
    a1_ends(nodes)
    for variants in (VARIANTS, A1_VARIANTS):
        policies(nodes, model, "all", variants)
        for label in sorted({n["population"] for n in nodes}):
            policies([n for n in nodes if n["population"] == label], model, label, variants)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("paths", nargs="+", help="files or directories of `wall choice` lines (*.jsonl)")
    parser.add_argument("--model", choices=("fit", "floor"), default="fit",
                        help="the forecast of a subtree from its gap (default: fitted)")
    parser.add_argument("--steps", type=int, default=10 ** 6, help="use at most this many steps of each fit")
    args = parser.parse_args()
    report(read(args.paths), args.steps, args.model)


if __name__ == "__main__":
    main()
