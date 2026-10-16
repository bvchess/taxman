#!/usr/bin/env python3
"""The tuning driver: runs (configuration, problem) jobs of the tuning suite and ranks the configurations.

Standard library only; run it as `python3 -I bench/tuning/tune.py` from rust/ (README.md has the commands).

A configuration is a name and a list of `NAME=VALUE` tactic settings (what `taxman solve --tactic` takes; the
configurations file is a JSON object of them, and `default`, no settings, is always there).  A problem is a line of the
suite (suite.jsonl): a subtree of a frontier search, or a whole game, which `examples/tune.rs` solves to completion on
one thread.  A job is one configuration on one problem, one `tune solve` process, and its result line is appended to the
results file as soon as it ends; a pair already in the results file is not run again, so a run that stops resumes.

The configurations are ranked by successive halving: rung 0 runs every configuration on a random third of the suite and
keeps the best third (by the geometric mean of CPU seconds, relative to the default on the same problems), rung 1 runs
those on two thirds, rung 2 on the whole suite.  The default runs on everything, as the yardstick.  The report gives
each configuration's geometric mean ratios of CPU seconds and of nodes against the default, its worst problem, and any
problem whose best promotion sum differs from the default's, which would be a bug.

Time is the CPU time of the solving thread (`cpu_s` of `tune solve`), not the wall time, so that other load on the
machine does not matter much; the nodes are exact.  A job is stopped when it passes CUTOFF times the default's nodes
(its ratio is then a lower bound: the report counts those).
"""

import argparse
import json
import math
import os
import random
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parents[1]
DEFAULT = "default"
PROBLEM_KINDS = ("unit", "whole")


# ---------------------------------------------------------------------------------------------------------
# Reading the inputs


def load_suite(path):
    """The problems of the suite file, in file order: dicts with `id`, `game`, `kind` and `default` (the default's
    nodes and CPU seconds when the suite was made).  The `game` lines, which `tune solve` reads, are not problems."""
    problems = []
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if row.get("kind") in PROBLEM_KINDS:
            if "id" not in row or "game" not in row:
                raise SystemExit(f"{path}: line {number}: a problem needs an id and a game")
            problems.append(row)
    ids = [p["id"] for p in problems]
    if len(set(ids)) != len(ids):
        raise SystemExit(f"{path}: two problems have the same id")
    return problems


def load_configs(path):
    """The configurations: {name: [setting, ...]} from a JSON object, with `default` (no settings) added."""
    raw = json.loads(Path(path).read_text()) if path else {}
    configs = {DEFAULT: []}
    for name, settings in raw.items():
        settings = [settings] if isinstance(settings, str) else list(settings)
        bad = [s for s in settings if "=" not in s]
        if bad:
            raise SystemExit(f"{path}: configuration {name}: {bad[0]!r} is not NAME=VALUE")
        if name == DEFAULT and settings:
            raise SystemExit(f"{path}: the configuration {DEFAULT} is the yardstick: it takes no settings")
        configs[name] = settings
    return configs


def read_results(path):
    """The records of the results file: those of jobs that ended (ok), by (configuration, problem); the last of a
    pair wins.  A job that failed is in the file too (it says why) but is run again."""
    done = {}
    if Path(path).exists():
        for line in Path(path).read_text().splitlines():
            if line.strip():
                try:
                    row = json.loads(line)
                except json.JSONDecodeError:
                    continue  # a line cut short by a kill
                if row.get("ok"):
                    done[(row["config"], row["problem"])] = row
    return done


# ---------------------------------------------------------------------------------------------------------
# Statistics


def geomean(values):
    """The geometric mean of positive numbers (nan if there are none)."""
    values = list(values)
    return math.exp(sum(math.log(v) for v in values) / len(values)) if values else float("nan")


def ratio(a, b):
    """a / b with a floor under both, so that a problem of no measurable time cannot make 0 or infinity."""
    floor = 1e-6
    return max(a, floor) / max(b, floor)


def comparison(done, config, problems):
    """What `config` did against the default on the `problems` both have results for: a dict with `n`, the geometric
    mean ratios `cpu` and `nodes`, the `worst` problem (id and CPU ratio), the number `cut` of jobs stopped by the
    cutoff, the CPU seconds `cpu_s` of the configuration, and the ids whose best sum `differs` from the default's."""
    rows = []
    for p in problems:
        mine, base = done.get((config, p["id"])), done.get((DEFAULT, p["id"]))
        if mine and base:
            rows.append((p["id"], mine["result"], base["result"]))
    if not rows:
        return None
    cpu = [ratio(m["cpu_s"], b["cpu_s"]) for _, m, b in rows]
    nodes = [ratio(m["nodes"], b["nodes"]) for _, m, b in rows]
    worst = max(zip(cpu, (r[0] for r in rows)))
    # A search that stopped at a limit has found only part of what it would: its best proves nothing.
    differs = [i for i, m, b in rows if m["completed"] and b["completed"] and m["best"] != b["best"]]
    return {
        "n": len(rows),
        "cpu": geomean(cpu),
        "nodes": geomean(nodes),
        "worst": (worst[1], worst[0]),
        "cut": sum(not m["completed"] for _, m, _ in rows),
        "cpu_s": sum(m["cpu_s"] for _, m, _ in rows),
        "differs": differs,
    }


# ---------------------------------------------------------------------------------------------------------
# The report


def report(done, configs, problems, out=None):
    """Prints the table, one line a configuration, best first; and, loudly, any best sum that differs."""
    rows = []
    for name in configs:
        stats = comparison(done, name, problems)
        if stats:
            rows.append((name, stats))
    rows.sort(key=lambda r: r[1]["cpu"])
    ran = sum(1 for p in problems if (DEFAULT, p["id"]) in done)
    print(f"{len(problems)} problems in the suite, the default has run {ran} of them", file=out)
    head = f"{'configuration':<28} {'n':>3} {'cpu ratio':>9} {'nodes ratio':>11} {'worst':>7}  {'worst problem':<14}"
    print(head + f" {'cut':>3} {'cpu s':>9}", file=out)
    for name, s in rows:
        flag = "  <- BEST DIFFERS" if s["differs"] else ""
        print(
            f"{name:<28} {s['n']:>3} {s['cpu']:>9.3f} {s['nodes']:>11.3f} {s['worst'][1]:>7.2f}  {s['worst'][0]:<14}"
            f" {s['cut']:>3} {s['cpu_s']:>9.1f}{flag}",
            file=out,
        )
    print("ratios are geometric means over the problems both the configuration and the default ran (below 1 is "
          "faster than the default); cut: jobs stopped at the cutoff, whose ratios are lower bounds", file=out)
    for name, s in rows:
        if s["differs"]:
            print(f"!!! BUG? {name}: the best promotion sum differs from the default's on {', '.join(s['differs'])}",
                  file=out)
    for p in problems:
        base = done.get((DEFAULT, p["id"]))
        recorded = p.get("default", {}).get("nodes")
        if base and base["result"]["completed"] and recorded is not None and base["result"]["nodes"] != recorded:
            print(f"!!! the default's nodes on {p['id']} are {base['result']['nodes']}, the suite's {recorded}: "
                  "another build than the suite's?", file=out)


# ---------------------------------------------------------------------------------------------------------
# Running jobs


def solve_command(binary, known, units, problem, settings=(), node_limit=None, nice=0):
    """The `tune solve` command line for a problem (a suite line: `kind`, `game`, `id`): a whole game is solved from
    its root, a unit is read from the units file `units`; `settings` are `NAME=VALUE` tactics."""
    cmd = [str(binary), "solve", str(problem["game"]), "--known", str(known)]
    if problem["kind"] == "whole":
        cmd += ["--ids", "root"]
    else:
        cmd += ["--units", str(units), "--ids", problem["id"]]
    for setting in settings:
        cmd += ["--tactic", setting]
    if node_limit:
        cmd += ["--node-limit", str(node_limit)]
    return (["nice", "-n", str(nice)] if nice else []) + cmd


def solve(command):
    """Runs a `tune solve` command and returns its result line; raises RuntimeError if it failed."""
    try:
        run = subprocess.run(command, capture_output=True, text=True, check=False)
        lines = [line for line in run.stdout.splitlines() if line.strip()]
        if run.returncode != 0 or not lines:
            raise RuntimeError(f"exit {run.returncode}: {run.stderr.strip()[-400:]}")
        return json.loads(lines[-1])
    except (OSError, json.JSONDecodeError) as e:
        raise RuntimeError(str(e)) from e


class Runner:
    """Runs jobs on a pool of worker threads, each job a `tune solve` process, and appends their results."""

    def __init__(self, args, problems):
        self.args, self.problems = args, {p["id"]: p for p in problems}
        self.lock = threading.Lock()

    def command(self, settings, problem):
        a = self.args
        recorded = problem.get("default", {}).get("nodes")
        limit = math.ceil(a.cutoff * recorded) if a.cutoff and recorded else None
        return solve_command(a.bin, a.known, a.suite, problem, settings, limit, a.nice)

    def job(self, rung, config, settings, problem):
        began = time.time()
        record = {"config": config, "problem": problem["id"], "rung": rung, "settings": settings}
        try:
            record.update(ok=True, result=solve(self.command(settings, problem)))
        except RuntimeError as e:
            record.update(ok=False, error=str(e))
        record["elapsed_s"] = round(time.time() - began, 3)
        with self.lock:
            with open(self.args.results, "a") as out:
                out.write(json.dumps(record) + "\n")
        return record

    def run(self, rung, jobs):
        """Runs the (config, settings, problem) jobs, the dearest first; returns how many failed."""
        order = sorted(jobs, key=lambda j: -(j[2].get("default", {}).get("cpu_s") or 0))
        failed = 0
        with ThreadPoolExecutor(max_workers=self.args.workers) as pool:
            futures = [pool.submit(self.job, rung, c, s, p) for c, s, p in order]
            for number, future in enumerate(futures, 1):
                record = future.result()
                if not record["ok"]:
                    failed += 1
                    print(f"  job failed: {record['config']} on {record['problem']}: {record['error']}", flush=True)
                elif number % 10 == 0 or number == len(futures):
                    print(f"  rung {rung}: {number}/{len(futures)} jobs done", flush=True)
        return failed


def rung_problems(problems, rung, seed):
    """The problems of rung `rung`: the first third (rung 0), two thirds or all of the suite in an order shuffled by the
    seed, so that each rung's problems contain the last one's."""
    order = random.Random(seed).sample(problems, len(problems))
    return order[: math.ceil(len(order) * min(rung + 1, 3) / 3)]


def survivors(done, alive, problems):
    """The best third (at least one) of the configurations in `alive` other than the default, by the geometric mean of
    their CPU ratio on `problems`, and the default; configurations that did not finish them rank last."""
    ranked = []
    for name in alive:
        if name != DEFAULT:
            stats = comparison(done, name, problems)
            complete = stats is not None and stats["n"] == len(problems)
            ranked.append((stats["cpu"] if complete else math.inf, name))
    ranked.sort()
    keep = max(1, math.ceil(len(ranked) / 3)) if ranked else 0
    return [DEFAULT] + [name for _, name in ranked[:keep]]


def halve(args, configs, problems):
    """Successive halving: runs rung after rung, resuming from the results file."""
    runner = Runner(args, problems)
    alive = list(configs)
    for rung in range(args.rungs):
        subset = rung_problems(problems, rung, args.seed)
        done = read_results(args.results)
        jobs = [(c, configs[c], p) for c in alive for p in subset if (c, p["id"]) not in done]
        print(f"rung {rung}: {len(alive)} configurations ({', '.join(alive)}) on {len(subset)} problems, "
              f"{len(jobs)} jobs to run", flush=True)
        if args.dry_run:
            for c, _, p in jobs:
                print("  ", " ".join(runner.command(configs[c], p)))
            return
        runner.run(rung, jobs)
        done = read_results(args.results)
        report(done, {c: configs[c] for c in alive}, subset)
        alive = survivors(done, alive, subset)


# ---------------------------------------------------------------------------------------------------------


def parse(argv=None):
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--configs", help="JSON object of configurations: name -> list of NAME=VALUE (default: none)")
    p.add_argument("--results", default="results.jsonl", help="the results file, appended to (default results.jsonl)")
    p.add_argument("--suite", default=HERE / "suite.jsonl", type=Path, help="the suite (default: suite.jsonl here)")
    p.add_argument("--known", default=CRATE / "results" / "strict.jsonl", type=Path, help="the known games")
    p.add_argument("--bin", default=CRATE / "target" / "release" / "examples" / "tune", type=Path,
                   help="the tune example (cargo build --release --example tune)")
    p.add_argument("--workers", type=int, default=os.cpu_count() or 1, help="jobs at once, one thread each")
    p.add_argument("--nice", type=int, default=0, help="run the jobs under nice -n N (default 0: not niced)")
    p.add_argument("--seed", type=int, default=1, help="seed of the random third of the suite (default 1)")
    p.add_argument("--rungs", type=int, default=3, choices=(1, 2, 3), help="rungs to run, 1 to 3 (default 3)")
    p.add_argument("--cutoff", type=float, default=10.0,
                   help="stop a job at this many times the default's nodes, 0 for no limit (default 10)")
    p.add_argument("--report", action="store_true", help="print the table from the results file and exit")
    p.add_argument("--dry-run", action="store_true", help="print the commands of the first rung and stop")
    return p.parse_args(argv)


def main(argv=None):
    args = parse(argv)
    problems = load_suite(args.suite)
    configs = load_configs(args.configs)
    if args.report:
        done = read_results(args.results)
        names = {c: configs.get(c, []) for c in {key[0] for key in done} | {DEFAULT}}
        ordered = {DEFAULT: [], **{c: names[c] for c in sorted(names) if c != DEFAULT}}
        report(done, ordered, problems)
        return
    if not args.bin.exists() and not args.dry_run:
        raise SystemExit(f"{args.bin} does not exist: cargo build --release --example tune")
    halve(args, configs, problems)


if __name__ == "__main__":
    main()
