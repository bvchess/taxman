#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Runs `wall nogoods` over a list of games on the laptop, a few processes at once under `nice -n 10`, one process a
game, each line appended to the file as its game finishes.  A run that is interrupted goes on where it was: the games
the file has are skipped.

usage (from rust/, after `cargo build --profile test --example wall`):
  ./bench/finders/nogoods/run.py FILE --cpu S [--jobs 4] [--shuffle] GAME... [-- WALL OPTIONS]
where a GAME is N, or A-B, or A-B/STEP (every STEP-th game of A..B); the options after `--` (`--steps`, `--long`,
`--seed`, `--fix`, `--unit-nodes`) go to `wall nogoods` as they are.
"""
import argparse
import random
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from run import KNOWN, RUST, WALL, done_keys, games  # noqa: E402  (the helpers of the finders' runner)


def nogoods(args):
    """`wall nogoods ARGS` under `nice -n 10`: its output lines.  A fault (a nogood that promotes, a bound below the
    known optimum) exits the whole run with what the process printed: it is a bug to report."""
    done = subprocess.run(["nice", "-n", "10", str(WALL), "nogoods", *map(str, args)], cwd=RUST,
                          capture_output=True, text=True)
    if done.returncode != 0:
        sys.exit(f"wall nogoods {' '.join(map(str, args))}: exit {done.returncode}\n{done.stdout}{done.stderr}")
    return done.stdout.splitlines()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("file", type=Path, help="the file of lines, appended to")
    parser.add_argument("games", nargs="+")
    parser.add_argument("--cpu", required=True, help="CPU seconds of the finder and of the control")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--shuffle", action="store_true", help="in a random order (the dear games are not all last)")
    args, extra = parser.parse_known_args()
    extra = [a for a in extra if a != "--"]

    have = done_keys(args.file, lambda line: line["n"])
    todo = [n for n in dict.fromkeys(games(args.games)) if n not in have]
    if args.shuffle:
        random.Random(1).shuffle(todo)
    print(f"{len(todo)} games to do, {len(have)} done", file=sys.stderr)

    def one(n):
        lines = nogoods(["--known", KNOWN, "--cpu", args.cpu, *extra, n])
        with args.file.open("a") as out:  # one line a game, appended whole
            out.write("".join(line + "\n" for line in lines))
        return n

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for n in pool.map(one, todo):
            print(f"n={n} done", file=sys.stderr)


if __name__ == "__main__":
    main()
