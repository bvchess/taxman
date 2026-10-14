#!/usr/bin/env python3
"""Step 1: the chain baseline, `taxman solve 1001-1231` with strict tactics, resumable.

Each invocation runs the solver for at most `INVOCATION_BUDGET` seconds of wall time (see common.py) and then
stops it; the results file and the per-game checkpoints let the next invocation continue (finished games are
skipped, a game in progress resumes from its checkpoint).

Usage: run_chain.py [--first 1001] [--last 1231] [--threads 8] [--out FILE]
"""
import argparse
import subprocess
import time

from common import CHAIN, HERE, INVOCATION_BUDGET, TAXMAN, Budget, load_jsonl, log_load


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--first", type=int, default=1001)
    ap.add_argument("--last", type=int, default=1231)
    ap.add_argument("--threads", type=int, default=8)
    ap.add_argument("--out", default=str(CHAIN))
    ap.add_argument("--checkpoint-dir", default=str(HERE / "chain_ckpt"))
    ap.add_argument("--budget", type=int, default=INVOCATION_BUDGET)
    args = ap.parse_args()

    budget = Budget(args.budget)
    log_load(f"chain start (budget {args.budget} s)")
    # The chain starts from optimal.json's game of 1000; the games whose proofs rest on it say so in their lines.
    cmd = [
        str(TAXMAN), "solve", f"{args.first}-{args.last}", "--threads", str(args.threads),
        "--out", args.out, "--checkpoint-dir", args.checkpoint_dir, "--progress", "0",
    ]
    proc = subprocess.Popen(cmd)
    while proc.poll() is None and budget.left() > 3:
        time.sleep(1)
    if proc.poll() is None:
        print("budget used up: stopping the solver (it resumes from its checkpoint next time)", flush=True)
        proc.terminate()
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    done = {r["n"] for r in load_jsonl(args.out) if r.get("completed")}
    todo = [n for n in range(args.first, args.last + 1) if n not in done]
    print(f"exit code {proc.returncode}; {len(done)} games done, {len(todo)} to do"
          + (f" (next {todo[0]})" if todo else ""), flush=True)
    log_load("chain end")


if __name__ == "__main__":
    main()
