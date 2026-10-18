#!/usr/bin/env python3
"""The tables of the Graviton calibration, from the rows parse.py made:
summary.py LAPTOP.jsonl MACHINE.jsonl... [-r STRICT] [-e LAST]

The first file is the reference (the laptop); every table has a column per machine.  Speed is the reference's time over
the machine's (above 1: faster than the reference's core).  The committed results file (-r, default
../../results/strict.jsonl) gives the seconds the chain's games took when the reference solved them, for the re-solved
range.  Its games 1..LAST count (-e, default CHAIN_END = 2175, the length of the chain when the entry was written; the
file has grown since, and its later games would add a row for the games that the chain run solved new).
Standard library only.
"""

import json
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent

# The last game of the committed chain that the tables compare with: the chain ended here when the entry was written.
CHAIN_END = 2175

# The games whose one-thread proofs the speed table sums.
PROOF_GAMES = (1215, 1232, 1287, 2170, 2172)


def load(path: str) -> list:
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def pick(rows, **want):
    return [r for r in rows if all(r.get(k) == v for k, v in want.items())]


def solo(rows, game):
    """The one-thread proof of a game that ran alone: a ladder run, or the contention test's single copy (K = 1)."""
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
    probes = (
        ("probe, default (s)", "probe-default", "probe_wall_s"),
        ("probe, GF(2) (s)", "probe-gf2", "probe_wall_s"),
    )
    for label, kind, key in probes:
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
        ("solver proofs, 5 frontier games, search s", lambda rows: total(rows, "ladder", PROOF_GAMES, "search_s")),
        ("solver proofs, 5 frontier games, user s", lambda rows: total(rows, "ladder", PROOF_GAMES, "user_s")),
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
                    top = max(r[key] for r in copies)
                    row.append(f"{mu:.2f} ({mu / base[0][key]:.3f}x), max {top:.2f}, n={len(copies)}")
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
    body = []
    for m, rows in machines.items():
        games = pick(rows, kind="chain-game", completed=True)
        runs = pick(rows, kind="chain")
        for first in sorted({g["chain_from"] for g in games}):
            sel = sorted((g for g in games if g["chain_from"] == first), key=lambda g: g["game"])
            run = [r for r in runs if r.get("game") == first]
            wall = run[0]["wall_s"] if run else None
            search = sum(g["search_s"] for g in sel)
            total = sum(g["total_s"] for g in sel)
            known = [g for g in sel if g["game"] in committed]
            body.append([
                m, f"{sel[0]['game']}..{sel[-1]['game']}", len(sel), sel[0]["threads"], f"{search:.1f}", f"{total:.1f}",
                fmt(wall, 1), f"{3600 * len(sel) / total:.0f}", f"{3600 * len(sel) / wall:.0f}" if wall else "-",
                "-" if not known else (
                    "none" if all(g["score"] == committed[g["game"]]["score"] for g in known)
                    else [g["game"] for g in known if g["score"] != committed[g["game"]]["score"]]
                ),
            ])
            if known:
                ref = [committed[g["game"]] for g in known]
                ref_total = sum(r["seconds"]["total"] for r in ref)
                body.append([
                    f"(committed file, same games, {ref[0]['threads']} threads)",
                    f"{ref[0]['n']}..{ref[-1]['n']}", len(ref),
                    ref[0]["threads"], f"{sum(r['seconds']['search'] for r in ref):.1f}", f"{ref_total:.1f}", "-",
                    f"{3600 * len(ref) / ref_total:.0f}", "-", "-",
                ])
    table(
        ["machine", "games", "n", "threads", "search s", "sum of games' total s", "run wall s",
         "games/hour (total s)", "games/hour (wall)", "scores differing"],
        body,
    )



def take_option(args: list, flag: str, default):
    """The value that follows `flag` in args, which both leave the list, or the default if the flag is not there."""
    if flag not in args:
        return default
    i = args.index(flag)
    value = args[i + 1]
    del args[i : i + 2]
    return value


def main() -> None:
    args = sys.argv[1:]
    strict_path = Path(take_option(args, "-r", HERE / "../../results/strict.jsonl"))
    last = int(take_option(args, "-e", CHAIN_END))
    machines = {Path(p).stem: load(p) for p in args}
    per_core(machines)
    contention(machines)
    scaling(machines)
    memory(machines)
    chain(machines, [g for g in load(str(strict_path)) if g["n"] <= last])


if __name__ == "__main__":
    main()
