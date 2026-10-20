#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Makes the checkpoint of a game as the tuning campaign made its checkpoints: `taxman solve N --threads 4
--time-limit 120` from the committed chain cut to N - 1.  Standard library only.

usage: python3 -I checkpoint.py N --dir DIR [--taxman BINARY] [--nice 10] [--threads 4] [--time-limit 120]
                                [--log checkpoints.jsonl]

The cut chain (the first N - 1 lines of results/strict.jsonl) is copied to DIR/cutN.jsonl, which `--out` appends to and
which gives the search the game of N - 1 for its seed and ceiling; without `--use-prev` the solver draws on nothing
else, so it searches game N, whatever the chain holds for it.  The chain has lines that cite the bounds store (the
method `known`, from 3984 on), and a file with such a line is opened only with `--use-prev`, which would take game N
from the chain too; so then the solve also gets `--force-search`, which leaves game N to its search (the seed and the
ceiling from N - 1 stay).  The checkpoint is DIR/ckptN/nN.ckpt.  The solve is run alone, since it takes `--threads`
threads, and a line with its CPU seconds (the children's user and system time) goes to the log.
"""
import argparse
import json
import resource
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
STRICT = REPO / "results" / "strict.jsonl"
TAXMAN = REPO / "solver" / "frame" / "target" / "release" / "taxman"


def cut_chain(strict, n, path):
    """Writes the lines of `strict` for the games 1..n-1 to `path`, after checking that line i is game i; returns
    whether a line of them cites a known source (the method `known`)."""
    lines = Path(strict).read_text().splitlines()
    if len(lines) < n - 1:
        sys.exit(f"{strict} has {len(lines)} lines, not {n - 1}")
    rows = [json.loads(line) for line in lines[: n - 1]]
    for i, row in enumerate(rows, 1):
        if row["n"] != i:
            sys.exit(f"{strict}: line {i} is not game {i}")
    Path(path).write_text("".join(line + "\n" for line in lines[: n - 1]))
    return any(row["method"] == "known" for row in rows)


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("n", type=int)
    p.add_argument("--dir", required=True, type=Path)
    p.add_argument("--taxman", default=TAXMAN, type=Path)
    p.add_argument("--strict", default=STRICT, type=Path)
    p.add_argument("--nice", type=int, default=10)
    p.add_argument("--threads", type=int, default=4)
    p.add_argument("--time-limit", type=int, default=120)
    p.add_argument("--log", type=Path)
    a = p.parse_args()
    a.dir.mkdir(parents=True, exist_ok=True)
    cut, ckpt = a.dir / f"cut{a.n - 1}.jsonl", a.dir / f"ckpt{a.n}"
    known = cut_chain(a.strict, a.n, cut)
    command = ["nice", "-n", str(a.nice), str(a.taxman), "solve", str(a.n), "--threads", str(a.threads),
               "--time-limit", str(a.time_limit), "--out", str(cut), "--checkpoint-dir", str(ckpt)]
    if known:
        command += ["--use-prev", "--force-search"]
    before, began = resource.getrusage(resource.RUSAGE_CHILDREN), time.time()
    run = subprocess.run(command, cwd=REPO, capture_output=True, text=True, check=False)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    print(run.stdout, end="")
    print(run.stderr[-3000:], end="", file=sys.stderr)
    record = {"game": a.n, "command": " ".join(command[3:]), "status": run.returncode, "cpu_s": round(cpu, 1),
              "wall_s": round(time.time() - began, 1), "checkpoint": str(ckpt / f"n{a.n}.ckpt"),
              "checkpoint_bytes": (ckpt / f"n{a.n}.ckpt").stat().st_size if (ckpt / f"n{a.n}.ckpt").exists() else None}
    print(json.dumps(record))
    if a.log:
        with open(a.log, "a") as out:
            out.write(json.dumps(record) + "\n")


if __name__ == "__main__":
    main()
