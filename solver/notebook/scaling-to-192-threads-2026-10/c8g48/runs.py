#!/usr/bin/env python3
"""The runs of the 192-thread measurement (measure.sh and its extras) as JSON lines, from the directory they wrote.

    runs.py runs DIR > runs.jsonl      one line per run: its command, status, wall seconds (steps.log), GNU time's user
                                       and system seconds, peak memory, context switches and page faults (NAME.time),
                                       and for `regress` the program's own seconds (NAME.out)
    runs.py games DIR > games.jsonl    one line per game of each `solve` run: the run, and the game's method, proof,
                                       score, nodes, threads and seconds, from the results lines the run added

DIR holds what measure.sh, extras.sh and extras2.sh wrote (the copy of their S3 prefix); the lines are in the order of
steps.log.
"""
import json
import re
import sys
from pathlib import Path

# The results lines of a `solve` run, by run name, when measure.sh named the file after the results file and not after
# the run.
RESULTS = {
    "i8b_T64_W1": "w64_1.new.jsonl",
    "i8b_T64_W16": "w64_16.new.jsonl",
    "i8b_T192_W1": "w192_1.new.jsonl",
    "i8b_T192_W16": "w192_16.new.jsonl",
    "i8b_T192_W16_trace": "w192_16t.new.jsonl",
    "i4_4000": "i4_4000.line.jsonl",
}


def steps(d):
    """(name, command, status, wall) of each run in steps.log, in order."""
    commands, runs = {}, []
    for line in (d / "steps.log").read_text().splitlines():
        if m := re.match(r"\S+ start (\S+?):? (.*)$", line):
            commands[m[1]] = m[2]
        elif m := re.match(r"\S+ end (\S+): status (\d+), wall ([\d.]+) s", line):
            runs.append((m[1], commands.get(m[1], ""), int(m[2]), float(m[3])))
    return runs


def gnu_time(path):
    if not path.exists():
        return {}
    text = path.read_text()
    fields = {
        "user_s": r"User time \(seconds\): ([\d.]+)",
        "sys_s": r"System time \(seconds\): ([\d.]+)",
        "max_rss_kb": r"Maximum resident set size \(kbytes\): (\d+)",
        "minor_faults": r"Minor \(reclaiming a frame\) page faults: (\d+)",
        "voluntary_cs": r"Voluntary context switches: (\d+)",
        "involuntary_cs": r"Involuntary context switches: (\d+)",
    }
    out = {}
    for key, pattern in fields.items():
        if m := re.search(pattern, text):
            out[key] = float(m[1]) if key.endswith("_s") else int(m[1])
    return out


def option(command, flag):
    m = re.search(rf"{flag} (\S+)", command)
    return m[1] if m else None


def runs(d):
    for name, command, status, wall in steps(d):
        line = {"run": name, "status": status, "wall_s": wall}
        for flag in ("--threads", "--window"):
            if value := option(command, flag):
                line[flag[2:]] = value if "," in value else int(value)
        if "taskset -c 0,1," in command:
            line["cpus"] = "node 0, CPUs 0-63"
        elif "taskset -c" in command:
            line["cpus"] = "CPUs " + option(command, "taskset -c")
        line.update(gnu_time(d / f"{name}.time"))
        out = d / f"{name}.out"
        if " regress " in command and out.exists():
            if m := re.search(r"^solved .* in ([\d.]+) s", out.read_text(), re.M):
                line["regress_s"] = float(m[1])
        line["command"] = re.sub(r"taskset -c [\d,-]+", "taskset -c CPUS", command)
        print(json.dumps(line))


def games(d):
    for name, _, _, _ in steps(d):
        path = d / RESULTS.get(name, f"{name}.new.jsonl")
        if not path.exists():
            continue
        for text in path.read_text().splitlines():
            r = json.loads(text)
            seconds = r.get("seconds") or {}
            print(json.dumps({
                "run": name,
                "n": r["n"],
                "method": r.get("method"),
                "completed": r.get("completed"),
                "proof": r.get("proof"),
                "score": r.get("score"),
                "nodes": r.get("nodes"),
                "threads": r.get("threads"),
                "search_s": seconds.get("search"),
                "total_s": seconds.get("total"),
            }))


if __name__ == "__main__":
    {"runs": runs, "games": games}[sys.argv[1]](Path(sys.argv[2]))
