#!/usr/bin/env python3
"""The steps of each unit from two incumbents: the units file's own (the checkpoint's) and the known optimum.

Usage: pairs.py UNITS RUNS

UNITS is the units file the runs were made from (its gaps are against the checkpoint's incumbent), RUNS the jsonl lines
of `tune solve` (runpairs.sh).  Prints a row a unit, sorted by gap: the gap, the steps and CPU seconds from each
incumbent ("+" where a limit stopped the run) and the ratio of steps, then the sums over the units that both runs
finished.
"""
import json
import sys


def main():
    units = {}
    for line in open(sys.argv[1]):
        d = json.loads(line)
        if d.get("kind") == "unit":
            units[d["id"]] = d
    runs = {}
    for line in open(sys.argv[2]):
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        runs[(d["id"], "opt" if d["proved"] else "start")] = d
    print("| unit | depth | gap | from the start: steps | CPU s | from the optimum: steps | CPU s | ratio of steps |")
    print("|---|---:|---:|---:|---:|---:|---:|---:|")
    sums = [0, 0.0, 0, 0.0]
    for uid in sorted({k[0] for k in runs}, key=lambda u: -units[u]["gap"]):
        a, b = runs.get((uid, "start")), runs.get((uid, "opt"))
        if not a or not b:
            continue
        mark = lambda r: "" if r["completed"] else "+"
        ratio = a["nodes"] / max(b["nodes"], 1)
        print(f"| {uid} | {units[uid]['depth']} | {units[uid]['gap']:,} | {a['nodes']:,}{mark(a)} | {a['cpu_s']:.1f} | "
              f"{b['nodes']:,}{mark(b)} | {b['cpu_s']:.1f} | {ratio:.2f}{'+' if mark(a) else ''} |")
        if a["completed"] and b["completed"]:
            sums = [sums[0] + a["nodes"], sums[1] + a["cpu_s"], sums[2] + b["nodes"], sums[3] + b["cpu_s"]]
        assert b["best"] == b["opt"], f"{uid}: the best from the optimum is {b['best']}, not {b['opt']}"
    print(f"both finished: steps {sums[0]:,} against {sums[2]:,} ({sums[0] / max(sums[2], 1):.2f}), "
          f"CPU {sums[1]:.0f} s against {sums[3]:.0f} s ({sums[1] / max(sums[3], 1e-9):.2f})")


if __name__ == "__main__":
    main()
