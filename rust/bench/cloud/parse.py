#!/usr/bin/env python3
"""Turn the logs of one machine's calibration (calibrate.sh, timed.sh) into one JSON line per run.

    parse.py MACHINE DIR > MACHINE.jsonl

DIR holds, per run, LABEL.time (the report of GNU time -v or BSD time -l), LABEL.out and LABEL.status, and the prover's
files under prover/ and the chain runs' results files chain-FROM.jsonl (with chain-FROM.from).  The label says what the run was:

    probe-default.rN, probe-gf2.rN        the probe (N-th repetition)
    ladder-GAME.tTHREADS                  the solver's proof of a game
    prover-GAME.t1                        the prover on a game
    contention-solver-GAME.kK.cI          copy I of K at once (solver); contention-prover-GAME.kK.cI the same for the prover
    chain-FROM.tTHREADS                   taxman solve from game FROM over a copy of results/strict.jsonl cut before it

Standard library only.
"""

import json
import re
import sys
from pathlib import Path

LADDER = re.compile(r"^n=(\d+) T=(\d+) (\S+)\s+search\s+([\d.]+) s nodes\s+(\d+)", re.M)
PROBE = re.compile(r"probe total: (?:cycles ([\d.]+) G, instructions ([\d.]+) G, )?wall ([\d.]+) s; fingerprint (\w+)")
LABEL = re.compile(
    r"^(?P<kind>probe-default|probe-gf2|ladder|prover|contention-solver|contention-prover|chain)"
    r"(?:-(?P<game>\d+))?"
    r"(?:\.r(?P<rep>\d+))?(?:\.t(?P<threads>\d+))?(?:\.k(?P<k>\d+)\.c(?P<copy>\d+))?$"
)


def clock(text: str) -> float:
    """GNU time's h:mm:ss or m:ss.ss."""
    seconds = 0.0
    for part in text.split(":"):
        seconds = seconds * 60 + float(part)
    return seconds


def parse_time(text: str) -> dict:
    """The resource report of GNU time -v (Linux) or BSD time -l (macOS), in common units."""
    r = {}
    if "Maximum resident set size (kbytes)" in text:
        r["wall_s"] = clock(re.search(r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)", text).group(1))
        r["user_s"] = float(re.search(r"User time \(seconds\): ([\d.]+)", text).group(1))
        r["sys_s"] = float(re.search(r"System time \(seconds\): ([\d.]+)", text).group(1))
        r["max_rss_kb"] = int(re.search(r"Maximum resident set size \(kbytes\): (\d+)", text).group(1))
    else:
        m = re.search(r"([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys", text)
        r["wall_s"], r["user_s"], r["sys_s"] = (float(m.group(i)) for i in (1, 2, 3))
        r["max_rss_kb"] = int(re.search(r"(\d+)\s+maximum resident set size", text).group(1)) // 1024
        for key, name in (("instructions", "instructions retired"), ("cycles", "cycles elapsed")):
            m = re.search(rf"(\d+)\s+{name}", text)
            if m:
                r[key] = int(m.group(1))
    return r


def parse_run(machine: str, directory: Path, time_file: Path) -> dict:
    label = time_file.name[: -len(".time")]
    m = LABEL.match(label)
    if not m:
        raise SystemExit(f"unknown label {label}")
    row = {"machine": machine, "label": label, "kind": m["kind"]}
    for key in ("game", "rep", "threads", "k", "copy"):
        if m[key] is not None:
            row[key] = int(m[key])
    status = directory / f"{label}.status"
    if not status.exists():  # still running
        return None
    row["exit"] = int(status.read_text())
    row.update(parse_time(time_file.read_text()))
    out = (directory / f"{label}.out").read_text()
    if m["kind"] in ("ladder", "contention-solver"):
        found = LADDER.search(out)
        if found:
            row["completed"] = found[3] == "DONE"
            row["search_s"] = float(found[4])
            row["nodes"] = int(found[5])
    elif m["kind"].startswith("probe"):
        found = PROBE.search(out)
        if found:
            row["probe_wall_s"] = float(found[3])
            row["fingerprint"] = found[4]
            if found[1] and float(found[1]) > 0:  # Linux has no per-process cycle counter: the probe prints 0
                row["probe_cycles_g"], row["probe_instructions_g"] = float(found[1]), float(found[2])
        row["kernels_ns_per_op"] = kernels(out)
    elif m["kind"] in ("prover", "contention-prover"):
        data = directory / "prover" / f"{label}.jsonl"
        if data.exists():
            line = json.loads(data.read_text().splitlines()[-1])
            row.update({"nodes": line["nodes"], "prover_s": line["secs"], "search": line["search"]})
    return row


def kernels(out: str) -> dict:
    """ns/op of every kernel line of the probe (the fourth number from the end of the line), by game and name."""
    result, game = {}, None
    for line in out.splitlines():
        head = re.match(r"^n=(\d+) W=", line)
        if head:
            game = head[1]
            continue
        # name, ops, cycles per op, instructions per op ("-" on Linux), ns per op, fingerprint
        cells = re.match(r"^\s+(.+?)\s{2,}(\d+)\s+(\S+)\s+(\S+)\s+([\d.]+)\s+[0-9a-f]{16}$", line)
        if cells and game:
            result[f"{game}: {cells[1]}"] = float(cells[5])
    return result


def parse_chain(machine: str, directory: Path) -> list:
    """The games each chain run added to its copy of results/strict.jsonl (those from the game in chain-FROM.from on)."""
    rows = []
    for marker in sorted(directory.glob("chain-*.from")):
        first = int(marker.read_text())
        path = marker.with_suffix(".jsonl")
        games = [json.loads(line) for line in path.read_text().splitlines() if line.strip()]
        rows += [
            {
                "machine": machine,
                "kind": "chain-game",
                "chain_from": first,
                "game": g["n"],
                "score": g["score"],
                "search_s": g["seconds"]["search"],
                "total_s": g["seconds"]["total"],
                "nodes": g["nodes"],
                "threads": g["threads"],
                "proof": g["proof"],
                "invocations": g["invocations"],
            }
            for g in games
            if g["n"] >= first
        ]
    return rows


def main() -> None:
    machine, directory = sys.argv[1], Path(sys.argv[2])
    for time_file in sorted(directory.glob("*.time")):
        row = parse_run(machine, directory, time_file)
        if row:
            print(json.dumps(row))
    for row in parse_chain(machine, directory):
        print(json.dumps(row))


if __name__ == "__main__":
    main()
