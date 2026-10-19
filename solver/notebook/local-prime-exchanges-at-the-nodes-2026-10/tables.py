#!/usr/bin/env python3
"""The tables of the entry (README.md beside this file), from the lines of `wall exchanges`.

usage: python3 -I tables.py runs.jsonl soundness.jsonl

The fields of the lines are defined in the module comment of solver/frame/examples/wall/exchanges.rs.  Standard library
only.
"""
import json
import sys
from collections import defaultdict


def pct(a, b):
    return f"{100.0 * a / b:.1f}%" if b else "-"


def table(rows, header):
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    for r in rows:
        print("| " + " | ".join(str(x) for x in r) + " |")
    print()


def read(path):
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def units(lines):
    check = {d["id"]: d for d in lines if d["mode"] == "check" and d.get("sample", 0) == 0 and d["below"] == 0}
    prune = {d["id"]: d for d in lines if d["mode"] == "prune" and d["below"] == 0}
    sampled = {d["id"]: d for d in lines if d.get("sample", 0) > 0}
    ids = sorted(check, key=lambda i: -check[i]["nodes"])

    print("## How often an exchange is a symmetry of its node\n")
    rows = []
    for name, key, of in [
        ("exclusions", "exclusions", None),
        ("an exchange maps the member onto another candidate", "with_exchange", "exclusions"),
        ("one passes the whole-node check", "whole", "exclusions"),
        ("one passes the component check", "component", "exclusions"),
        ("one passes the reduced check", "reduced", "exclusions"),
        ("one passes it and is no symmetric pair of the root", "beyond_root", "exclusions"),
        ("exclusions priced at the node's next step", "priced", None),
        ("exchange swaps: the heaviest set took the member's image", "exchange_swaps", "priced"),
        ("the swap's exchange passes the whole-node check", "whole_swaps", "exchange_swaps"),
        ("the reduced check", "reduced_swaps", "exchange_swaps"),
        ("the reduced check, for no pair of the root", "beyond_root_swaps", "exchange_swaps"),
        ("cheap exclusions (priced under 50)", "cheap_priced", "priced"),
        ("cheap exchange swaps", "cheap_exchange", "cheap_priced"),
        ("their exchange passes the reduced check", "cheap_reduced", "cheap_exchange"),
        ("the reduced check, for no pair of the root", "cheap_beyond_root", "cheap_exchange"),
    ]:
        cells = []
        for i in ids:
            d = check[i]
            cells.append(f"{d[key]:,}" + (f" ({pct(d[key], d[of])})" if of else ""))
        rows.append([name] + cells)
    table(rows, [""] + ids)

    print("## The exchange swaps met most\n")
    for i in ids:
        rows = []
        for x, y, p, q, times, price, whole, component, reduced, root, *_ in check[i]["swaps"][:10]:
            rows.append([f"{x} for {y}", f"{p}, {q}", f"{times:,}", price, f"{reduced:,}", "yes" if root else "no"])
        print(f"{i}:\n")
        table(rows, ["member for image", "primes", "times", "median price", "reduced check passed", "root's pair"])

    print("## The cheap exclusions met most\n")
    for i in ids:
        rows = []
        for x, gained, times, price, exchange, reduced in check[i]["cheap"][:10]:
            rows.append([x, " ".join(map(str, gained)), f"{times:,}", price, "yes" if exchange else "no",
                         f"{reduced:,}"])
        print(f"{i}:\n")
        table(rows, ["member left out", "came in", "times", "median price", "an exchange swap", "reduced check passed"])

    print("## Where the reduced check fails at the exchange swaps\n")
    kinds = sorted({k for i in ids for k, _ in check[i]["swap_fails"]},
                   key=lambda k: -sum(dict(check[i]["swap_fails"]).get(k, 0) for i in ids))
    rows = []
    for k in kinds:
        rows.append([k] + [pct(dict(check[i]["swap_fails"]).get(k, 0), sum(v for _, v in check[i]["swap_fails"]))
                           for i in ids])
    table(rows, ["kind"] + ids)
    for i in ids:
        rows = [[kind, f"{p}, {q}", member, x, y, f"{n:,}"] for kind, p, q, member, x, y, n in
                check[i]["fail_examples"][:8]]
        print(f"{i}:\n")
        table(rows, ["kind", "primes", "member", "number", "its image", "times"])

    print("## What pruning by the passing exchanges saves\n")
    rows = []
    for i in ids:
        c, p = check[i], prune.get(i)
        if p is None:
            continue
        rows.append([i, f"{c['nodes']:,}", f"{p['nodes']:,}", f"{c['tree_nodes']:,}", f"{p['tree_nodes']:,}",
                     f"{c['leaves']:,}", f"{p['leaves']:,}", f"{p['constraints']:,}", f"{p['dropped']:,}",
                     f"{p['cut']:,}", "yes" if p["best"] == c["best"] and p["completed"] == c["completed"] else "NO"])
    table(rows, ["unit", "steps", "pruned", "nodes", "pruned", "leaves", "pruned", "constraints", "dropped", "cut",
                 "same best and end"])

    if sampled:
        print("## The pair's dominance on samples, where the check failed\n")
        rows = []
        for i in ids:
            d = sampled.get(i)
            if d is None:
                continue
            rows.append([i, f"{d['nodes']:,}", d["sample"], f"{d['sampled']:,}",
                         f"{d['sampled_held']:,} ({pct(d['sampled_held'], d['sampled'])})", f"{d['cheap_sampled']:,}",
                         f"{d['cheap_sampled_held']:,} ({pct(d['cheap_sampled_held'], d['cheap_sampled'])})"])
        table(rows, ["unit", "steps", "samples", "swaps sampled", "held", "cheap ones sampled", "held"])
        for i in ids:
            d = sampled.get(i)
            if d is None:
                continue
            rows = [[f"{x} for {y}", f"{p}, {q}", f"{times:,}", price, f"{s:,}", f"{h:,}"]
                    for x, y, p, q, times, price, _, _, _, _, s, h in d["swaps"][:10]]
            print(f"{i}, {'the whole unit' if d['completed'] else 'its first {:,} steps'.format(d['nodes'])}:\n")
            table(rows, ["member for image", "primes", "times", "median price", "sampled", "held"])


def soundness(lines):
    print("## Whole games from a floor, with the rule and without\n")
    by = defaultdict(dict)
    for d in lines:
        by[(d["tactics"], d["mode"])][d["n"]] = d
    rows = []
    for (tactics, mode), games in sorted(by.items()):
        wrong = [n for n, d in games.items() if d["best"] != d["opt"] or not d["completed"]]
        rows.append([tactics, mode, f"{min(games)}-{max(games)}", len(games), len(wrong),
                     f"{sum(d['nodes'] for d in games.values()):,}", f"{sum(d['constraints'] for d in games.values()):,}",
                     f"{sum(d['dropped'] for d in games.values()):,}", f"{sum(d['cut'] for d in games.values()):,}"])
    table(rows, ["tactics", "mode", "games", "searched", "optimum missed", "steps", "constraints", "dropped", "cut"])


def main():
    units(read(sys.argv[1]))
    if len(sys.argv) > 2:
        soundness(read(sys.argv[2]))


if __name__ == "__main__":
    main()
