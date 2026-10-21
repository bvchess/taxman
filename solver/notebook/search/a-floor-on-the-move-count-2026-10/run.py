#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""One run of the floor probe: `wall images --count` on a whole game (`--root`) or, with `--suite` and `--ids`, on a
unit of the hardest searches' suite, from the committed chain's known best set (or the suite's incumbent), on one
thread, as the hardest searches' entry ran them (solver/notebook/search/the-hardest-searches-of-the-run-to-5000-2026-10/
README.md, "The runs"), with the floor set or not.

usage: python3 -I run.py WALL RESULTS.jsonl GAME [--floor F] [--max-load 8] [--nodes 20000000] [--label TEXT]
                         [--suite SUITE.jsonl --ids ID]

WALL is the `wall` example of the probe's build (`cargo build --profile test --features studies --example wall`).  The
run waits until the machine's load average over one minute is at most `--max-load`, then starts `wall` under
`nice -n 10` and `/usr/bin/time -l` (the floor in the environment variable TAXMAN_FLOOR if `--floor` is given), and
appends a line to RESULTS.jsonl: `wall images`' own line (solver/frame/examples/wall/images.rs, with the floor and its
counters, which the probe adds, src/search/floor.rs), the label, the load at the start, and from `time -l` the user and
system CPU seconds, the wall seconds, the peak memory, and the instructions and cycles.  `batch.py` makes many runs by
`run_once`.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
KNOWN = REPO / "results" / "strict.jsonl"


def quiet(max_load):
    """Waits until the one-minute load average is at most `max_load`; returns it."""
    while (load := os.getloadavg()[0]) > max_load:
        print(time.strftime("%H:%M:%S"), f"load {load:.2f}, waiting", file=sys.stderr, flush=True)
        time.sleep(30)
    return load


def time_fields(text):
    """The numbers of `/usr/bin/time -l`'s report (macOS): seconds, peak memory, instructions and cycles."""
    real, user, system = re.search(r"([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys", text).groups()
    found = {"real_s": float(real), "user_s": float(user), "sys_s": float(system)}
    for name, label in (("max_rss", "maximum resident set size"), ("instructions", "instructions retired"),
                        ("cycles", "cycles elapsed")):
        match = re.search(rf"(\d+)\s+{label}", text)
        if match:
            found[name] = int(match.group(1))
    return found


def run_once(wall, game, floor=None, nodes=20_000_000, label="", max_load=8.0, suite=None, unit=None):
    """Waits for a quiet machine, then runs `wall images --count` on the whole game or, with `suite` and `unit`, on that
    unit of the suite, with the floor `floor` (promotions) or none.  Returns `wall images`' line with the label, the
    load at the start, the wall seconds and `time -l`'s numbers; exits if the run fails."""
    env = {key: value for key, value in os.environ.items() if key != "TAXMAN_FLOOR"}
    if floor is not None:
        env["TAXMAN_FLOOR"] = str(floor)
    load = quiet(max_load)
    where = ["--suite", str(suite), "--ids", unit] if suite else ["--root"]
    command = ["nice", "-n", "10", "/usr/bin/time", "-l", str(wall), "images", "--known", str(KNOWN), *where,
               "--count", "--nodes", str(nodes), str(game)]
    began = time.time()
    done = subprocess.run(command, env=env, capture_output=True, text=True)
    if done.returncode != 0:
        sys.exit(f"{' '.join(command)}\nexit {done.returncode}\n{done.stderr}")
    line = json.loads(done.stdout.strip().splitlines()[-1])
    line.update({"label": label, "load": load, "wall_s": round(time.time() - began, 1), **time_fields(done.stderr)})
    return line


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("wall")
    parser.add_argument("results")
    parser.add_argument("game", type=int)
    parser.add_argument("--floor", type=int)
    parser.add_argument("--max-load", type=float, default=8.0)
    parser.add_argument("--nodes", type=int, default=20_000_000)
    parser.add_argument("--label", default="")
    parser.add_argument("--suite")
    parser.add_argument("--ids")
    args = parser.parse_args()
    line = run_once(args.wall, args.game, args.floor, args.nodes, args.label, args.max_load, args.suite, args.ids)
    with open(args.results, "a") as out:
        out.write(json.dumps(line) + "\n")
    print(json.dumps(line))


if __name__ == "__main__":
    main()
