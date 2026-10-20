#!/usr/bin/env python3
"""Shared helpers for the cost-curve scripts: paths, JSONL files, primes, wall-clock budgets, machine load."""
import json
import os
import re
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parents[2] / "frame"  # the frame solver's crate (this folder is solver/notebook/search/<entry>)
TAXMAN = CRATE / "target/release/taxman"
LADDER = CRATE / "target/release/examples/ladder"

# The study was made with `--tactic reuse_prev=off --tactic revise_freedoms=off` (no unproven tactic), for `solve`,
# and `--tactic revise_freedoms=off` for the ladder; both tactics are gone, and the defaults are that search now.

# The runs were made in sessions limited to 10 minutes per command, so a timed-out standalone game (search limit +
# setup) has to fit in one: the search limit is 585 s rather than 600 s, and one invocation of a driver may run for
# 592 s at most.  A timeout therefore means "at least 585 s".
SEARCH_LIMIT = 585
INVOCATION_BUDGET = 592

CHAIN = HERE / "chain.jsonl"


def is_prime(n):
    return n > 1 and all(n % d for d in range(2, int(n**0.5) + 1))


def non_primes_from(start, count):
    """The first `count` non-prime numbers >= start."""
    found, n = [], start
    while len(found) < count:
        if not is_prime(n):
            found.append(n)
        n += 1
    return found


def load_jsonl(path):
    path = Path(path)
    if not path.exists():
        return []
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def append_jsonl(path, record):
    """Append one line and sync it, so that a killed run loses at most the game it was in."""
    with open(path, "a") as f:
        f.write(json.dumps(record) + "\n")
        f.flush()
        os.fsync(f.fileno())


class Budget:
    """Wall-clock budget of one invocation."""

    def __init__(self, seconds):
        self.end = time.monotonic() + seconds

    def left(self):
        return self.end - time.monotonic()


_CPU_LINE = re.compile(r"CPU usage: ([\d.]+)% user, ([\d.]+)% sys, ([\d.]+)% idle")


def background_load():
    """What the machine is doing apart from us, sampled over one second: idle share and the busiest processes."""
    out = subprocess.run(
        ["top", "-l", "2", "-n", "5", "-o", "cpu", "-stats", "command,cpu", "-s", "1"],
        capture_output=True, text=True, check=False,
    ).stdout
    last = out.rsplit("Processes:", 1)[-1]
    cpu = _CPU_LINE.findall(out)
    procs = []
    in_table = False
    for line in last.splitlines():
        if line.startswith("COMMAND"):
            in_table = True
        elif in_table and line.strip():
            name, _, pct = line.strip().rpartition(" ")
            procs.append([name.strip(), float(pct)])
    idle = float(cpu[-1][2]) if cpu else None
    return {"idle_pct": idle, "top": procs[:5]}


def log_load(label, log_path=HERE / "load.jsonl"):
    record = {"time": time.strftime("%Y-%m-%d %H:%M:%S"), "label": label, **background_load()}
    append_jsonl(log_path, record)
    return record
