#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Makes the runs of a plan (plan.py) with the floor probe, two at a time, and appends each run's line to a results
file.

usage: python3 -I batch.py WALL PLAN.jsonl RESULTS.jsonl [--workers 2] [--max-load 8] [--budget-s 21000]

Each plan line is one run by `run.run_once` (so under `nice -n 10` and `/usr/bin/time -l`, started with the one-minute
load at most `--max-load`).  The workers take the plan's lines in order.  A run already in RESULTS.jsonl (the same `id`
and `label`) is not made again, so an interrupted batch is started again with the same command.  The batch stops
starting runs when a run fails, does not complete, or ends with a best set other than the one the chain (or, for a game
the chain lacks, the suite's default search) has, and when the CPU seconds of RESULTS.jsonl (user plus system) reach
`--budget-s`.  The results lines are `wall images`' lines with the plan's `kind` and `label`.
"""
import argparse
import json
import sys
import threading
import time
from pathlib import Path


HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # the script's own directory is not on the path under `python3 -I`
from run import run_once  # noqa: E402

REPO = HERE.parents[3]
SUITES = HERE.parent / "the-hardest-searches-of-the-run-to-5000-2026-10"


def suite_bests():
    """The default search's best of each unit of the suite: what a game with no known optimum must end with."""
    found = {}
    for line in (SUITES / "suite-hard.jsonl").read_text().splitlines():
        row = json.loads(line)
        if row["kind"] == "unit":
            found[row["id"]] = row["default"]["best"]
    return found


class Batch:
    def __init__(self, args):
        self.args = args
        self.plan = [json.loads(line) for line in Path(args.plan).read_text().splitlines() if line.strip()]
        self.results = Path(args.results)
        self.lines = [json.loads(line) for line in self.results.read_text().splitlines() if line.strip()] \
            if self.results.exists() else []
        self.done = {(line["id"], line["label"]) for line in self.lines}
        self.cpu = sum(line["user_s"] + line["sys_s"] for line in self.lines)
        self.bests = suite_bests()
        self.lock = threading.Lock()
        self.stop = threading.Event()

    def log(self, text):
        print(time.strftime("%H:%M:%S"), text, file=sys.stderr, flush=True)

    def next_job(self):
        with self.lock:
            if self.stop.is_set() or self.cpu >= self.args.budget_s:
                return None
            while self.plan:
                job = self.plan.pop(0)
                if (job["id"], job["label"]) not in self.done:
                    return job
            return None

    def check(self, job, line):
        """What is wrong with a finished run: it must complete with the best the unit or game has."""
        want = line["opt"] if line["opt"] is not None else self.bests.get(job["id"])
        if not line["completed"]:
            return f"{job['id']} {job['label']}: did not complete within {job['nodes']} steps"
        if want is not None and line["best"] != want:
            return f"{job['id']} {job['label']}: best {line['best']}, expected {want}"
        return None

    def work(self):
        while (job := self.next_job()) is not None:
            line = run_once(self.args.wall, job["game"], job["floor"], job["nodes"], job["label"], self.args.max_load,
                            REPO / job["suite"] if job["suite"] else None, job["id"] if job["suite"] else None)
            line["kind"] = job["kind"]
            with self.lock:
                with open(self.results, "a") as out:
                    out.write(json.dumps(line) + "\n")
                self.cpu += line["user_s"] + line["sys_s"]
                self.done.add((job["id"], job["label"]))
                spent = self.cpu
            wrong = self.check(job, line)
            took = line["user_s"] + line["sys_s"]
            self.log(f"{job['id']} {job['label']}: {line['steps']:,} steps, {took:.1f} s CPU, floor applied "
                     f"{line['floor_applies']}, cut {line['floor_cuts']}; {spent:.0f} s CPU in all")
            if wrong:
                self.log("STOPPING: " + wrong)
                self.stop.set()

    def go(self):
        threads = [threading.Thread(target=self.work) for _ in range(self.args.workers)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.log(f"finished; {self.cpu:.0f} s CPU in the results file; {len(self.plan)} planned runs not started")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("wall")
    parser.add_argument("plan")
    parser.add_argument("results")
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--max-load", type=float, default=8.0)
    parser.add_argument("--budget-s", type=float, default=21_000)
    Batch(parser.parse_args()).go()


if __name__ == "__main__":
    main()
