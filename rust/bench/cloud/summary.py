#!/usr/bin/env python3
"""The tables of the Graviton calibration, from the rows parse.py made: summary.py LAPTOP.jsonl MACHINE.jsonl... [-r STRICT]

The first file is the reference (the laptop); every table has a column per machine.  Speed is the reference's time over
the machine's (above 1: faster than the reference's core).  The committed results file (-r, default
../../results/strict.jsonl) gives the seconds the chain's games took when the reference solved them, for the re-solved
range.  Standard library only.
"""

import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent


def load(path: str) -> list:
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def pick(rows, **want):
    return [r for r in rows if all(r.get(k) == v for k, v in want.items())]


def solo(rows, game):
    """The one-thread proof of a game that ran alone: a ladder run, or the single copy of the contention test's K = 1."""
    return pick(rows, kind="ladder", game=game, threads=1) or pick(rows, kind="contention-solver", game=game, k=1)


def best(rows, key):
    """The smallest value of a field over the rows (the fastest repetition), or None."""
    values = [r[key] for r in rows if key in r]
    return min(values) if values else None


def mean(values):
    values = list(values)
    return sum(values) / len(values) if values else None


def fmt(x, digits=2):
    return "-" if x is None else f"{x:.{digits}f}"


def table(header, body):
    print("| " + " | ".join(header) + " |")
    print("|" + "|".join("---" for _ in header) + "|")
    for row in body:
        print("| " + " | ".join(str(c) for c in row) + " |")
    print()


def per_core(machines):
    names = [m for m in machines]
    print("### Per-core speed (one thread, nothing else running; the long proofs two at a time)\n")
    body = []
    for label, kind, key in (("probe, default (s)", "probe-default", "probe_wall_s"), ("probe, GF(2) (s)", "probe-gf2", "probe_wall_s")):
        body.append([label] + [fmt(best(pick(machines[m], kind=kind), key)) for m in names])
    games = (1215, 1232, 1287, 1326, 2170, 2172)
    for g in games:
        body.append([f"proof {g}: search s (nodes)"] + [
            "-" if not r else f"{r[0]['search_s']:.2f} ({r[0]['nodes']:,})"
            for r in (solo(machines[m], g) for m in names)
        ])
    for g in (1300, 990):
        body.append([f"prover {g}: s (nodes)"] + [
            "-" if not r else f"{r[0]['prover_s']:.1f} ({r[0]['nodes']:,})"
            for r in (pick(machines[m], kind="prover", game=g) for m in names)
        ])
    table(["", *names], body)

    def total(rows, kind, games_, key):
        found = [pick(rows, kind=kind, game=g, threads=1) for g in games_] if kind == "ladder" else [
            pick(rows, kind=kind, game=g) for g in games_
        ]
        return sum(r[0][key] for r in found) if all(found) else None

    ref = machines[names[0]]
    print("Speed relative to " + names[0] + " (its time over the machine's):\n")
    body = []
    for label, get in (
        ("probe, default", lambda rows: best(pick(rows, kind="probe-default"), "probe_wall_s")),
        ("probe, GF(2)", lambda rows: best(pick(rows, kind="probe-gf2"), "probe_wall_s")),
        ("solver proofs, 5 frontier games, search s", lambda rows: total(rows, "ladder", (1215, 1232, 1287, 2170, 2172), "search_s")),
        ("solver proofs, 5 frontier games, user s", lambda rows: total(rows, "ladder", (1215, 1232, 1287, 2170, 2172), "user_s")),
        ("prover, 1300 + 990", lambda rows: total(rows, "prover", (1300, 990), "prover_s")),
    ):
        base = get(ref)
        body.append([label] + [fmt(base / get(machines[m]), 3) if base and get(machines[m]) else "-" for m in names])
    table(["", *names], body)


def contention(machines):
    print("### Contention: K copies of the same one-thread run at once\n")
    for kind, game, key, what in (
        ("contention-solver", 1326, "search_s", "solver proof of 1326, mean search s per copy"),
        ("contention-prover", 990, "prover_s", "prover on 990, mean s per copy"),
    ):
        ks = sorted({r["k"] for rows in machines.values() for r in pick(rows, kind=kind)})
        body = []
        for k in ks:
            row = [k]
            for m, rows in machines.items():
                copies = pick(rows, kind=kind, k=k)
                base = pick(rows, kind=kind, k=1)
                if copies and base:
                    mu = mean(r[key] for r in copies)
                    row.append(f"{mu:.2f} ({mu / base[0][key]:.3f}x), max {max(r[key] for r in copies):.2f}, n={len(copies)}")
                else:
                    row.append("-")
            body.append(row)
        print(what + "\n")
        table(["K", *machines], body)


def scaling(machines):
    print("### Thread scaling of the proof of 2172 (search seconds; nodes; speedup and efficiency over one thread)\n")
    ts = sorted({r["threads"] for rows in machines.values() for r in pick(rows, kind="ladder", game=2172)})
    body = []
    for t in ts:
        row = [t]
        for m, rows in machines.items():
            here = pick(rows, kind="ladder", game=2172, threads=t)
            one = pick(rows, kind="ladder", game=2172, threads=1)
            if here and one:
                s = one[0]["search_s"] / here[0]["search_s"]
                row.append(f"{here[0]['search_s']:.1f} s, {here[0]['nodes']:,} nodes, x{s:.1f}, {100 * s / t:.0f}%")
            else:
                row.append("-")
        body.append(row)
    table(["threads", *machines], body)


def memory(machines):
    print("### Maximum resident set size (MB)\n")
    rows_spec = (
        ("probe", lambda r: r["kind"].startswith("probe")),
        ("solver proof, 1 thread", lambda r: r["kind"] == "ladder" and r["threads"] == 1),
        ("solver proof, 4 threads", lambda r: r["kind"] == "ladder" and r["threads"] == 4),
        ("solver proof, 16 threads", lambda r: r["kind"] == "ladder" and r["threads"] == 16),
        ("solver proof, 64 threads", lambda r: r["kind"] == "ladder" and r["threads"] == 64),
        ("prover, 1 thread", lambda r: r["kind"] == "prover"),
        ("solver, 64 copies at once (each)", lambda r: r["kind"] == "contention-solver" and r["k"] == 64),
        ("prover, 64 copies at once (each)", lambda r: r["kind"] == "contention-prover" and r["k"] == 64),
        ("solver chain", lambda r: r["kind"] == "chain"),
    )
    body = []
    for label, test in rows_spec:
        row = [label]
        for rows in machines.values():
            found = [r["max_rss_kb"] / 1024 for r in rows if "max_rss_kb" in r and test(r)]
            row.append(f"{max(found):.0f}" if found else "-")
        body.append(row)
    table(["", *machines], body)


def chain(machines, strict):
    print("### The chain (games added to a copy of the committed file)\n")
    committed = {g["n"]: g for g in strict}
    for m, rows in machines.items():
        games = pick(rows, kind="chain-game")
        runs = pick(rows, kind="chain")
        if not games:
            continue
        for first in sorted({g["chain_from"] for g in games}):
            sel = sorted((g for g in games if g["chain_from"] == first), key=lambda g: g["game"])
            run = [r for r in runs if r.get("game") == first]
            wall = run[0]["wall_s"] if run else None
            search = sum(g["search_s"] for g in sel)
            line = f"{m}: from {first}: {len(sel)} games ({sel[0]['game']}..{sel[-1]['game']}), {sel[0]['threads']} threads, "
            line += f"search {search:.1f} s"
            if wall:
                line += f", wall {wall:.1f} s ({3600 * len(sel) / wall:.0f} games/hour)"
            known = [g for g in sel if g["game"] in committed]
            if known:
                ref_search = sum(committed[g["game"]]["seconds"]["search"] for g in known)
                wrong = [g["game"] for g in known if g["score"] != committed[g["game"]]["score"]]
                line += f"; committed file (the reference, {committed[known[0]['game']]['threads']} threads): {ref_search:.1f} s over the same games "
                line += f"(ratio {ref_search / search:.2f}); scores differing: {wrong or 'none'}"
            print(line + "\n")


def main() -> None:
    args = sys.argv[1:]
    strict_path = HERE / "../../results/strict.jsonl"
    if "-r" in args:
        i = args.index("-r")
        strict_path = Path(args[i + 1])
        del args[i : i + 2]
    machines = {Path(p).stem: load(p) for p in args}
    per_core(machines)
    contention(machines)
    scaling(machines)
    memory(machines)
    chain(machines, load(str(strict_path)))


if __name__ == "__main__":
    main()
