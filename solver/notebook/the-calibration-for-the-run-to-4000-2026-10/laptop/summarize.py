#!/usr/bin/env python3
"""One line of runs.jsonl for each timed run of run.sh (the rate runs t1, t2, ... and the setup-only runs).

usage: summarize.py RUNS_DIR CLOUD_RESULTS TAG [TAG ...] > runs.jsonl

RUNS_DIR/TAG/ holds what run.sh kept.  CLOUD_RESULTS is the cloud run's results file, whose last line is the
incomplete line of game 3441 at the checkpoint: the counts of a run are the incomplete line it appended to its own
out.jsonl less that one.  The rate is taken between the first progress line WARM seconds into the search (the first
seconds include the units' rebuilds and the empty nogood tables filling) and the last one.  A run's cycles,
instructions and CPU time are net of the setup-only run on as many threads (RUNS_DIR/setupT), which `net` records.
Tags that begin with "copy" are also pooled in one line, "copies": independent one-thread runs made at once.
"""
import json
import re
import statistics
import sys
from pathlib import Path

WARM = 20
PROGRESS = re.compile(
    r"^n=3441 (?P<clock>\S+) nodes (?P<nodes>\d+) \((?P<rate>\S+)/s\) best (?P<best>\d+) queue (?P<queue>\d+) "
    r"spilled (?P<spilled>\d+) \| eff (?P<eff>[\d.]+)% idle (?P<idle>[\d.]+)% lock (?P<lock>[\d.]+)% "
    r"splits (?P<splits>\S+) steals (?P<steals>\S+) full (?P<full>\S+) ramp (?P<ramp>[\d.]+)% tail (?P<tail>[\d.]+)% "
    r"rebuild (?P<rebuild>[\d.]+)% first (?P<first>[\d.]+)%(?: pauses (?P<pauses>\d+) \((?P<pause_share>[\d.]+)%\))?"
)
TIME = re.compile(r"^\s*([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys")
CLOCK = re.compile(r"(?:(\d+)h)?(?:(\d+)m)?(\d+)s")
RUSAGE = {"maximum resident set size": "max_rss", "instructions retired": "instructions", "cycles elapsed": "cycles",
          "involuntary context switches": "invol_switches"}


def seconds(clock):
    h, m, s = CLOCK.fullmatch(clock).groups()
    return 3600 * int(h or 0) + 60 * int(m or 0) + int(s)


def last_line(path):
    with open(path, "rb") as f:
        return json.loads(f.readlines()[-1])


def rusage(path):
    """`/usr/bin/time -l`'s numbers of a run."""
    out = {}
    for line in Path(path).read_text().splitlines():
        if m := TIME.match(line):
            out["real_s"], out["user_s"], out["sys_s"] = map(float, m.groups())
        for key, name in RUSAGE.items():
            if line.rstrip().endswith(key):
                out[name] = int(line.split()[0])
    return out


def counts(run, base):
    done = last_line(run / "out.jsonl")
    return {"threads": done["threads"], "nodes_run": done["nodes"] - base["nodes"],
            "counts_run": {k: v - base["stats"][k] for k, v in done["stats"].items()},
            "max_queue": done["engine"]["max_queue"]}


def progress(run):
    return [m.groupdict() for m in map(PROGRESS.match, (run / "progress.txt").read_text().splitlines()) if m]


def summarize(runs, base, tag):
    run = runs / tag
    out = {"tag": tag, "load_before": (run / "load_before.txt").read_text().split("load")[1].strip(),
           "load_after": (run / "load_after.txt").read_text().split("load")[1].strip()}
    out.update(counts(run, base))
    out.update(rusage(run / "time.txt"))
    lines = progress(run)
    if len(lines) < 2:
        return out
    t = [seconds(line["clock"]) for line in lines]
    begin = next((i for i, x in enumerate(t) if x - t[0] >= WARM), len(t) - 1)
    end = len(lines) - 1
    span = t[end] - t[begin]
    nodes = int(lines[end]["nodes"]) - int(lines[begin]["nodes"])
    out["window_seconds"] = span
    out["window_nodes"] = nodes
    out["nodes_per_s"] = round(nodes / span)
    out["nodes_per_s_per_thread"] = round(nodes / span / out["threads"])
    out["counters"] = {k: lines[end][k] for k in ("eff", "idle", "lock", "splits", "steals", "full", "ramp", "tail",
                                                   "rebuild", "first", "queue")}
    out["counters"]["pauses"] = lines[end]["pauses"]
    out["intervals_per_s"] = [
        round((int(b["nodes"]) - int(a["nodes"])) / max(1, seconds(b["clock"]) - seconds(a["clock"])))
        for a, b in zip(lines, lines[1:])
    ]
    setup = runs / f"setup{out['threads']}"
    if (setup / "time.txt").exists() and "cycles" in out:
        net = rusage(setup / "time.txt")
        nodes_net = out["nodes_run"] - counts(setup, base)["nodes_run"]
        cpu = out["user_s"] + out["sys_s"] - net["user_s"] - net["sys_s"]
        cycles, instructions = out["cycles"] - net["cycles"], out["instructions"] - net["instructions"]
        out["net"] = {
            "setup_tag": setup.name, "nodes": nodes_net, "cpu_s": round(cpu, 1), "cpu_per_wall": round(cpu / 130, 2),
            "nodes_per_cpu_s": round(nodes_net / cpu), "cycles_per_node": round(cycles / nodes_net),
            "instructions_per_node": round(instructions / nodes_net), "ipc": round(instructions / cycles, 2),
            "ghz": round(cycles / cpu / 1e9, 2),
        }
    return out


def pooled(lines):
    """One line for the independent runs `lines` made at once."""
    rates = [x["nodes_per_s"] for x in lines]
    cpu = [x["net"]["nodes_per_cpu_s"] for x in lines]
    return {"tag": "copies", "runs": len(lines), "threads_each": 1, "nodes_per_s_mean": round(statistics.mean(rates)),
            "nodes_per_s_min": min(rates), "nodes_per_s_max": max(rates),
            "nodes_per_cpu_s_mean": round(statistics.mean(cpu)),
            "ipc_mean": round(statistics.mean(x["net"]["ipc"] for x in lines), 2),
            "cycles_per_node_mean": round(statistics.mean(x["net"]["cycles_per_node"] for x in lines)),
            "instructions_per_node_mean": round(statistics.mean(x["net"]["instructions_per_node"] for x in lines)),
            "cpu_per_wall_mean": round(statistics.mean(x["net"]["cpu_per_wall"] for x in lines), 2)}


def main():
    runs = Path(sys.argv[1])
    base = last_line(sys.argv[2])
    lines = [summarize(runs, base, tag) for tag in sys.argv[3:]]
    copies = [x for x in lines if x["tag"].startswith("copy") and "net" in x]
    for line in lines + ([pooled(copies)] if copies else []):
        print(json.dumps(line))


main()
