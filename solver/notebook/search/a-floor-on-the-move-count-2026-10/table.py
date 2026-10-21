#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The table of the entry from runs.jsonl (the lines of run.py): a row a run, with the steps, the CPU seconds, the
score against the chain's, and the floor's counters.

usage: python3 -I table.py [RUNS.jsonl]
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
STRICT = HERE.parents[3] / "results" / "strict.jsonl"


def chain():
    """The chain's lines by n."""
    return {row["n"]: row for row in map(json.loads, STRICT.read_text().splitlines())}


def main(path):
    rows = chain()
    runs = [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]
    print("| game | floor | steps | CPU s | score = chain's | open | applies | cut | floor s | tight |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    for run in sorted(runs, key=lambda run: (-run["n"], run["floor"])):
        row = rows[run["n"]]
        score = row["in_the_bag"] + run["best"]
        matches = run["completed"] and score == row["score"]
        cpu = run["user_s"] + run["sys_s"]
        floor = run["floor"] or "off"
        opened = run["floor_open"]
        share = f"{run['floor_applies']:,} ({100 * run['floor_applies'] / opened:.2f}%)" if opened else "-"
        print(f"| {run['n']} | {floor} | {run['steps']:,} | {cpu:,.1f} | {'yes' if matches else 'NO'} {score} | "
              f"{opened:,} | {share} | {run['floor_cuts']:,} | {run['floor_ns'] / 1e9:.2f} | {run['floor_tight']:,} |")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else HERE / "runs.jsonl")
