#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Runs every problem of the tuning suite with the default search and with the learning of conflicts closed under the
cheap exchanges, and makes the tables of the entry's section "The whole tuning suite" from the lines.

usage: python3 -I suite.py run RESULTS.jsonl [--bin WALL] [--workers 2] [--max-load 8] [--nice 10] [--only ID,...]
                                             [--modes MODE,...] [--tactic NAME=VALUE] [--bare ID,...]
       python3 -I suite.py table RESULTS.jsonl
       python3 -I suite.py spread RESULTS.jsonl PERTURBED.jsonl

`run` starts `wall images` (solver/frame/examples/wall/images.rs, whose module comment defines the lines), one process a
run and one thread a process, for each problem of the suite and each mode (`--modes`, `count,closure` by default):
`count` is the default search with its tree counted (steps, tree nodes, leaves), `closure` the learning of conflicts
closed under the cheap exchanges (`--learn --closure`), `learn` the learning of their images alone and `check` the test
of the images, both for a look at the problems where `closure` loses.  `--bare` runs the named problems once more with
no hook, to measure what the counting hook costs.  At most `--workers` runs go at once, and a run starts only when the
machine's load average over one minute is at most `--max-load`.  Each run's line goes to RESULTS.jsonl as the run ends,
and a run already in the file is not made again, so an interrupted `run` is started again with the same command.  It
stops starting runs when a run fails, when the two searches of a problem differ in their best or their conclusion, or
when the default search's steps are not the suite's.  Ctrl-C ends its own processes and no others.

`table` prints the tables from RESULTS.jsonl, in the entry's form.  `run --tactic` makes the default search under
another tactic (one at a time, to a file of its own), and `spread` sets the steps of those runs beside the learning's.
"""
import argparse
import json
import math
import os
import signal
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
FRAME = REPO / "solver" / "frame"
SUITE = HERE.parent / "the-tuning-campaign-at-the-end-of-the-chain-2026-10" / "suite.jsonl"
KNOWN = REPO / "results" / "strict.jsonl"
WALL = FRAME / "target" / "debug" / "examples" / "wall"

# The gap of a whole game, in the suite's measure for a unit (the bound less what the node needs): the game's root gap
# from `wall landscape --known results/strict.jsonl N` (`root_gap`, the bound over P*) less one.
WHOLE_GAPS = {"3128-root": 923, "3129-root": 923, "3213-root": 933,
              "3270-root": 834, "3272-root": 834, "3275-root": 834}

# The most steps a run may take, as a multiple of the suite's default steps: a stop for a run that has gone wrong.
LIMITS = {"count": 2, "bare": 2, "check": 2, "learn": 3, "closure": 3}
# The options of `wall images` that make each run: the default search counted, the learning of the images alone, the
# learning of their closures, the default search with no hook, and the test of the images.
OPTIONS = {"count": ["--count"], "learn": ["--learn"], "closure": ["--learn", "--closure"], "bare": ["--bare"],
           "check": []}

# The gap bands of the tables: from, up to but not including.
BANDS = [(0, 850), (850, 950), (950, 10**9)]
# The bands of the default search's steps: from, up to but not including, and the name.
SIZES = [(0, 100_000, "under 100,000"), (100_000, 300_000, "100,000-299,999"), (300_000, 600_000, "300,000-599,999"),
         (600_000, 10**9, "600,000 and over")]
MINUS = "−"


# --- the problems --------------------------------------------------------------------------------------------------
def problems():
    """The suite's problems in its order: id, game, whole (a whole game), gap, the default's steps and CPU seconds."""
    out = []
    for line in SUITE.read_text().splitlines():
        row = json.loads(line)
        if row["kind"] in ("unit", "whole"):
            whole = row["kind"] == "whole"
            gap = WHOLE_GAPS[row["id"]] if whole else row["gap"]
            out.append({"id": row["id"], "game": row["game"], "whole": whole, "gap": gap,
                        "steps": row["default"]["nodes"], "cpu_s": row["default"]["cpu_s"]})
    return out


def key(line):
    """The key of a run: its problem and mode, and its tactics if they are not the default."""
    return (line["id"], line["mode"]) + (() if line["tactics"] == "default" else (line["tactics"],))


def read_results(path):
    """The lines of RESULTS.jsonl by key."""
    if not Path(path).exists():
        return {}
    lines = [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]
    return {key(line): line for line in lines}


# --- running -------------------------------------------------------------------------------------------------------
def command(binary, problem, mode, tactics=()):
    """The `wall images` command line of one run, with the search tactics `NAME=VALUE` given (their trees are not the
    suite's, so the limit on the steps is doubled)."""
    where = ["--root"] if problem["whole"] else ["--suite", str(SUITE), "--ids", problem["id"]]
    limit = LIMITS[mode] * problem["steps"] * (2 if tactics else 1)
    tuned = [option for spec in tactics for option in ("--tactic", spec)]
    return [str(binary), "images", "--known", str(KNOWN), *where, *OPTIONS[mode], *tuned, "--nodes", str(limit),
            str(problem["game"])]


def complaints(problem, results):
    """What is wrong with the runs of a problem so far: every search must complete with the problem's best, and the
    default search's steps must be the suite's."""
    found = []
    lines = [line for line in results.values() if line["id"] == problem["id"]]
    for line in lines:
        if not line["completed"] or line["best"] != line["opt"]:
            found.append(f"{problem['id']} {line['mode']} {line['tactics']}: completed {line['completed']}, "
                         f"best {line['best']}, opt {line['opt']}")
    if len({(line["best"], line["completed"]) for line in lines}) > 1:
        found.append(f"{problem['id']}: the best or the conclusion differs between the searches")
    count = results.get((problem["id"], "count"))
    if count and count["nodes"] != problem["steps"]:
        found.append(f"{problem['id']}: the default search took {count['nodes']} steps, "
                     f"the suite says {problem['steps']}")
    return found


class Runner:
    """The workers: they take the runs in order, each waiting for the machine to be quiet enough."""

    def __init__(self, args, jobs, results):
        self.args, self.jobs, self.results = args, jobs, results
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.children = []

    def log(self, text):
        print(time.strftime("%H:%M:%S"), text, file=sys.stderr, flush=True)

    def next_job(self):
        with self.lock:
            return self.jobs.pop(0) if self.jobs and not self.stop.is_set() else None

    def wait_for_load(self):
        """Waits while the machine's one-minute load average is over the limit; false if told to stop meanwhile."""
        waited = 0
        while os.getloadavg()[0] > self.args.max_load:
            if self.stop.wait(30):
                return False
            waited += 30
            if waited % 300 == 0:
                self.log(f"waiting for the load to fall: {os.getloadavg()[0]:.1f} over {self.args.max_load}")
        return not self.stop.is_set()

    def run_one(self, problem, mode):
        if not self.wait_for_load():
            return
        load = os.getloadavg()[0]
        began = time.time()
        tactics = self.args.tactic
        argv = ["nice", "-n", str(self.args.nice), *command(self.args.bin, problem, mode, tactics)]
        child = subprocess.Popen(argv, cwd=FRAME, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        with self.lock:
            self.children.append(child)
        out, err = child.communicate()
        with self.lock:
            self.children.remove(child)
        if child.returncode != 0:
            self.log(f"FAILED {problem['id']} {mode}: status {child.returncode}\n{err}")
            self.stop.set()
            return
        line = json.loads(out.strip().splitlines()[-1])
        line.update(gap=problem["gap"], load=round(load, 2), wall_s=round(time.time() - began, 1))
        with self.lock:
            self.results[key(line)] = line
            with open(self.args.results, "a") as f:
                f.write(json.dumps(line) + "\n")
            found = complaints(problem, self.results)
        self.log(f"{problem['id']} {mode}: {line['nodes']} steps, {line['cpu_s']:.1f} s CPU, load {load:.1f}")
        if found:
            self.log("STOPPING, look into it:\n" + "\n".join(found))
            self.stop.set()

    def work(self):
        while (job := self.next_job()) is not None:
            self.run_one(*job)

    def go(self):
        threads = [threading.Thread(target=self.work) for _ in range(self.args.workers)]
        for t in threads:
            t.start()

        def end(*_):
            self.stop.set()
            with self.lock:
                for child in self.children:
                    child.terminate()

        signal.signal(signal.SIGINT, end)
        signal.signal(signal.SIGTERM, end)
        for t in threads:
            while t.is_alive():
                t.join(1)


def run(args):
    ps = {p["id"]: p for p in problems()}
    wanted = args.only.split(",") if args.only else list(ps)
    bare = args.bare.split(",") if args.bare else []
    done = read_results(args.results)
    # The dearest problems first, so that the workers end together; a problem's two runs go side by side.
    order = sorted(wanted, key=lambda i: -ps[i]["cpu_s"])
    modes = args.modes.split(",")
    tuned = tuple(args.tactic)
    if len(tuned) > 1:
        sys.exit("run: one --tactic at a time")
    jobs = [(ps[i], mode) for i in order for mode in modes if (i, mode, *tuned) not in done]
    jobs += [(ps[i], "bare") for i in bare if (i, "bare", *tuned) not in done]
    print(f"{len(jobs)} runs to make, {len(done)} already in {args.results}", file=sys.stderr)
    Runner(args, jobs, done).go()


# --- the tables ----------------------------------------------------------------------------------------------------
def percent(before, after):
    """The change from `before` to `after` as a signed percentage, with the minus sign of the entries."""
    return f"{100 * (after / before - 1):+.1f}%".replace("-", MINUS)


def table(head, rows):
    print("| " + " | ".join(head) + " |")
    print("|" + "---|" * len(head))
    for row in rows:
        print("| " + " | ".join(str(c) for c in row) + " |")
    print()


def pairs(path):
    """Each problem with both of its runs, as (problem, default line, closure line)."""
    results = read_results(path)
    return [(p, results[(p["id"], "count")], results[(p["id"], "closure")]) for p in problems()
            if (p["id"], "count") in results and (p["id"], "closure") in results]


def summary(label, rows):
    """A line of the summary tables: the totals, the median and the geometric mean of the per-problem changes."""
    def total(key, which):
        return sum(r[which][key] for r in rows)

    def ratios(key):
        return [r[2][key] / r[1][key] for r in rows]

    def mean(rs):
        return math.exp(statistics.fmean(math.log(x) for x in rs))

    if not rows:
        return [label, 0] + ["-"] * 7
    cells = [label, len(rows)]
    for key in ("nodes", "leaves", "cpu_s"):
        cells += [percent(total(key, 1), total(key, 2))]
        if key != "leaves":
            cells += [percent(1, statistics.median(ratios(key))), percent(1, mean(ratios(key)))]
    return cells


SUMMARY_HEAD = ["", "problems", "steps: total", "median", "geometric mean", "leaves: total", "CPU: total", "median",
                "geometric mean"]


def band_of(gap):
    lo, hi = next((lo, hi) for lo, hi in BANDS if lo <= gap < hi)
    return f"under {hi}" if lo == 0 else f"{lo}-{hi - 1}" if hi < 10**9 else f"{lo} and over"


def tables(path):
    rows = pairs(path)
    if not rows:
        sys.exit(f"{path}: no problem has both of its runs")
    units = [r for r in rows if not r[0]["whole"]]
    wholes = [r for r in rows if r[0]["whole"]]
    print("**Totals**: the change from the default search to the learning, with the median and geometric mean of the "
          "problems' changes\n")
    table(SUMMARY_HEAD, [summary("all", rows), summary("units", units), summary("whole games", wholes)])

    print("**By gap**\n")
    table(SUMMARY_HEAD, [summary(f"gap {band_of(lo)}", [r for r in rows if lo <= r[0]["gap"] < hi])
                         for lo, hi in BANDS])
    print("**By the steps of the default search**\n")
    table(SUMMARY_HEAD, [summary(f"steps {label}", [r for r in rows if lo <= r[1]["nodes"] < hi])
                         for lo, hi, label in SIZES])
    change = [math.log(r[2]["nodes"] / r[1]["nodes"]) for r in rows]
    by_gap = statistics.correlation([r[0]["gap"] for r in rows], change)
    by_size = statistics.correlation([math.log(r[1]["nodes"]) for r in rows], change)
    print("Correlation (Pearson) of the log of the steps ratio, learning over default,\n"
          f"with the gap: {by_gap:+.2f}; with the log of the default's steps: {by_size:+.2f}.\n")
    print("**By game**\n")
    games = sorted({r[0]["game"] for r in units})
    table(SUMMARY_HEAD, [summary(str(g), [r for r in units if r[0]["game"] == g]) for g in games]
          + [summary("whole games", wholes)])

    print("**Each problem, by gap**\n")
    head = ["problem", "gap", "steps", "learning", "change", "leaves", "learning", "change", "CPU s", "learning",
            "change"]
    body = []
    for p, a, b in sorted(rows, key=lambda r: r[0]["gap"]):
        body.append([p["id"], p["gap"], f"{a['nodes']:,}", f"{b['nodes']:,}", percent(a["nodes"], b["nodes"]),
                     f"{a['leaves']:,}", f"{b['leaves']:,}", percent(a["leaves"], b["leaves"]),
                     f"{a['cpu_s']:.1f}", f"{b['cpu_s']:.1f}", percent(a["cpu_s"], b["cpu_s"])])
    table(head, body)

    print("**The problems where the learning costs more CPU than it saves**, with the learning of the images alone\n")
    results = read_results(path)
    worse = sorted((r for r in rows if r[2]["cpu_s"] >= r[1]["cpu_s"]), key=lambda r: r[1]["nodes"])
    head = ["problem", "gap", "default steps", "learning: steps", "CPU", "default: ms a step", "learning: ms a step",
            "images alone: steps", "CPU"]
    body = []
    for p, a, b in worse:
        images = results.get((p["id"], "learn"))
        body.append([p["id"], p["gap"], f"{a['nodes']:,}", percent(a["nodes"], b["nodes"]),
                     percent(a["cpu_s"], b["cpu_s"]), f"{1000 * a['cpu_s'] / a['nodes']:.3f}",
                     f"{1000 * b['cpu_s'] / b['nodes']:.3f}",
                     percent(a["nodes"], images["nodes"]) if images else "-",
                     percent(a["cpu_s"], images["cpu_s"]) if images else "-"])
    table(head, body)
    print(f"{len(worse)} of {len(rows)} problems; the learning takes more steps on "
          f"{sum(1 for _, a, b in rows if b['nodes'] >= a['nodes'])}.\n")

    print("**The cost of a step**\n")
    head = ["", "default: ms a step", "learning: ms a step", "change"]
    body = []
    for label, rs in (("all", rows), ("units", units), ("whole games", wholes)):
        if not rs:
            continue
        before = 1000 * sum(a["cpu_s"] for _, a, _ in rs) / sum(a["nodes"] for _, a, _ in rs)
        after = 1000 * sum(b["cpu_s"] for _, _, b in rs) / sum(b["nodes"] for _, _, b in rs)
        body.append([label, f"{before:.3f}", f"{after:.3f}", percent(before, after)])
    table(head, body)

    bare = [(p, results[(p["id"], "bare")], results[(p["id"], "count")]) for p in problems()
            if (p["id"], "bare") in results and (p["id"], "count") in results]
    if bare:
        print("**What the counting hook costs**: the default search with no hook (bare) and with it (count)\n")
        table(["problem", "steps", "bare: CPU s", "count: CPU s", "change"],
              [[p["id"], f"{c['nodes']:,}", f"{b['cpu_s']:.2f}", f"{c['cpu_s']:.2f}", percent(b["cpu_s"], c["cpu_s"])]
               for p, b, c in bare])

    checks = [p for p, _, _ in rows]
    found = [text for p in checks for text in complaints(p, results)]
    print(f"Checks over {len(rows)} problems: " + ("all agree: the same best and conclusion, the default's steps the "
                                                    "suite's." if not found else "\n".join(found)))
    cpu = sum(line["cpu_s"] for line in results.values())
    print(f"\nCPU seconds of the {len(results)} runs in the file: {cpu:,.0f}")


def spread(runs_path, perturbed_path):
    """The steps of the default search under other tactics, against its own and the learning's: how far a tree moves
    when something else that changes which nogoods the search holds is changed."""
    base, tuned = read_results(runs_path), read_results(perturbed_path)
    tactics = sorted({k[2] for k in tuned})
    rows = []
    for p in problems():
        if all((p["id"], "count", t) in tuned for t in tactics) and (p["id"], "count") in base:
            steps = base[(p["id"], "count")]["nodes"]
            moved = [tuned[(p["id"], "count", t)]["nodes"] for t in tactics]
            learning = base[(p["id"], "closure")]["nodes"]
            rows.append((p["id"], steps, moved, learning))
    head = ["problem", "steps", *tactics, "learning"]
    table(head, [[i, f"{s:,}", *[percent(s, m) for m in moved], percent(s, learned)]
                 for i, s, moved, learned in rows])
    changes = [m / s - 1 for _, s, moved, _ in rows for m in moved]
    learning = [learned / s - 1 for _, s, _, learned in rows]
    print(f"Over {len(rows)} problems, the other tactics move the steps from {percent(1, 1 + min(changes))} to "
          f"{percent(1, 1 + max(changes))}, and the learning from {percent(1, 1 + min(learning))} to "
          f"{percent(1, 1 + max(learning))}.")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    runs = sub.add_parser("run")
    runs.add_argument("results")
    runs.add_argument("--bin", default=WALL)
    runs.add_argument("--workers", type=int, default=2)
    runs.add_argument("--max-load", type=float, default=8.0)
    runs.add_argument("--nice", type=int, default=10)
    runs.add_argument("--only")
    runs.add_argument("--modes", default="count,closure")
    runs.add_argument("--tactic", action="append", default=[])
    runs.add_argument("--bare")
    tab = sub.add_parser("table")
    tab.add_argument("results")
    spreads = sub.add_parser("spread")
    spreads.add_argument("results")
    spreads.add_argument("perturbed")
    args = parser.parse_args()
    if args.command == "run":
        run(args)
    elif args.command == "table":
        tables(args.results)
    else:
        spread(args.results, args.perturbed)


if __name__ == "__main__":
    main()
