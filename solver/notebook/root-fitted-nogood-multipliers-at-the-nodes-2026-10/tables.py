#!/usr/bin/env python3
"""The tables of solver/notebook/root-fitted-nogood-multipliers-at-the-nodes-2026-10/README.md from the lines of `wall
relaxed` (the files given, default `nodes.jsonl` here).

A node is *open* when today's bound reaches its need (`today >= need`: today's search would not cut it); the root-λ bound
*cuts* it when it is below the need.  The drop is today's bound less the root-λ bound, at the open nodes.  The path
nodes (on the way to the optimum) check validity and are left out of everything else."""
import json
import statistics
import sys
from pathlib import Path


def median(values):
    return statistics.median(values) if values else float("nan")


def load(paths):
    games = []
    for path in paths:
        for line in Path(path).read_text().splitlines():
            if line.strip():
                games.append(json.loads(line))
    games.sort(key=lambda g: g["n"])
    return games


def sampled(game):
    return [x for x in game["nodes"] if x["source"] != "path"]


def is_open(x):
    return x["today"] >= x["need"]


def share(part, whole):
    return f"{part / whole:.2f}" if whole else "-"


def per_game(games):
    print("Per game (nodes of the fixtures and the suite; the path nodes apart):")
    print()
    print("| n | gap | pool | active | root share | nodes | open | cut | share cut | tight cut | share | tight below"
          " today | median drop | tight | median excess | today ms | fresh plain ms | root-λ ms | ratio |")
    print("|" + "---|" * 19)
    for g in games:
        nodes = sampled(g)
        opened = [x for x in nodes if is_open(x)]
        cut = [x for x in opened if x["relaxed"] < x["need"]]
        tight = [x for x in opened if x["tight"] < x["need"]]
        below = sum(x["tight"] < x["today"] for x in opened)
        drop = median([x["today"] - x["relaxed"] for x in opened])
        tdrop = median([x["today"] - x["tight"] for x in opened])
        excess = median([x["today"] - x["need"] + 1 for x in opened])
        today = 1e3 * median([x["today_s"] for x in nodes])
        plain = 1e3 * median([x["graph_s"] + x["plain_s"] for x in nodes])
        relaxed = 1e3 * median([x["graph_s"] + x["relaxed_s"] for x in nodes])
        print(f"| {g['n']} | {g['gap']} | {g['pool']} | {g['active']} | {g['share']} | {len(nodes)} | {len(opened)} |"
              f" {len(cut)} | {share(len(cut), len(opened))} | {len(tight)} | {share(len(tight), len(opened))} |"
              f" {share(below, len(opened))} | {drop:.0f} | {tdrop:.0f} | {excess:.0f} | {today:.1f} | {plain:.1f} | {relaxed:.1f} |"
              f" {relaxed / plain:.2f} |")
    print()


def pooled(games):
    print("Pooled (the open nodes of the fixtures and the suite):")
    print()
    print("| games | open | share cut | tight | tight below today | median drop | tight | median excess |"
          " median root-λ ms / fresh plain ms |")
    print("|---|---|---|---|---|---|---|---|---|")
    for label, keep in [("936, 1215", lambda g: g["n"] < 2176), ("2700-3440", lambda g: g["n"] >= 2176),
                        ("all", lambda g: True)]:
        rows = [x for g in games if keep(g) for x in sampled(g) if is_open(x)]
        nodes = [x for g in games if keep(g) for x in sampled(g)]
        if not rows:
            continue
        cut = sum(x["relaxed"] < x["need"] for x in rows)
        tight = sum(x["tight"] < x["need"] for x in rows)
        below = sum(x["tight"] < x["today"] for x in rows)
        ratio = median([(x["graph_s"] + x["relaxed_s"]) / (x["graph_s"] + x["plain_s"]) for x in nodes])
        print(f"| {label} | {len(rows)} | {share(cut, len(rows))} | {share(tight, len(rows))} |"
              f" {share(below, len(rows))} | {median([x['today'] - x['relaxed'] for x in rows]):.0f} |"
              f" {median([x['today'] - x['tight'] for x in rows]):.0f} |"
              f" {median([x['today'] - x['need'] + 1 for x in rows]):.0f} | {ratio:.2f} |")
    print()


def by_depth(games, label, keep):
    rows = [x for g in games if keep(g) for x in sampled(g) if is_open(x)]
    if not rows:
        return
    print(f"By depth (promotions), {label}, open nodes:")
    print()
    print("| depth | open | share cut | tight | median drop | tight | median excess |")
    print("|---|---|---|---|---|---|---|")
    edges = [0, 10, 20, 30, 40, 50, 60, 1000]
    for lo, hi in zip(edges, edges[1:]):
        band = [x for x in rows if lo <= x["depth"] < hi]
        if not band:
            continue
        cut = sum(x["relaxed"] < x["need"] for x in band)
        tight = sum(x["tight"] < x["need"] for x in band)
        print(f"| {lo}-{hi - 1 if hi < 1000 else ''} | {len(band)} | {share(cut, len(band))} | {share(tight, len(band))} |"
              f" {median([x['today'] - x['relaxed'] for x in band]):.0f} |"
              f" {median([x['today'] - x['tight'] for x in band]):.0f} |"
              f" {median([x['today'] - x['need'] + 1 for x in band]):.0f} |")
    print()


def suite(games):
    rows = [(g["n"], x) for g in games for x in sampled(g) if x["source"] == "suite" and x["subtree"] is not None]
    if not rows:
        return
    print("The tuning suite's units: the nodes of their subtrees (the default's solve) under the units each bound cuts:")
    print()
    print("| n | units | their nodes | cut by root-λ | their nodes | tight | their nodes |")
    print("|---|---|---|---|---|---|---|")
    for n in sorted({n for n, _ in rows}):
        units = [x for m, x in rows if m == n]
        total = sum(x["subtree"] for x in units)
        cut = [x for x in units if x["relaxed"] < x["need"]]
        tight = [x for x in units if x["tight"] < x["need"]]
        print(f"| {n} | {len(units)} | {total:,} | {len(cut)} | {sum(x['subtree'] for x in cut):,} | {len(tight)} |"
              f" {sum(x['subtree'] for x in tight):,} |")
    print()


def validity(games):
    print("Validity: the nodes on the way to the optimum (no bound may be below `lower`), and the roots:")
    print()
    print("| n | path nodes | least today - lower | least root-λ - lower | least tight - lower |"
          " root-λ at the root - opt |")
    print("|---|---|---|---|---|---|")
    for g in games:
        path = [x for x in g["nodes"] if x["source"] == "path"]
        least = [min((x[k] - x["lower"] for x in path), default="-") for k in ("today", "relaxed", "tight")]
        print(f"| {g['n']} | {len(path)} | {least[0]} | {least[1]} | {least[2]} | {g['relaxed_root'] - g['opt']} |")
    print()


def main():
    paths = sys.argv[1:] or [Path(__file__).parent / "nodes.jsonl"]
    games = load(paths)
    per_game(games)
    pooled(games)
    by_depth(games, "games 2700-3440", lambda g: g["n"] >= 2176)
    by_depth(games, "games 936 and 1215", lambda g: g["n"] < 2176)
    suite(games)
    validity(games)


main()
