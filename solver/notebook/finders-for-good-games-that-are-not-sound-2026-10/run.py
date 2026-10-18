#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Runs the finders experiment on the laptop: `wall sound` and `wall finders` over a list of games, a few processes at
once under `nice -n 10`, one process a game, each line appended to the stage's file as its game finishes.  A run that
is interrupted goes on where it was: the (game, search) pairs the file has are skipped.

usage (from rust/, after `cargo build --profile test --example wall` at commit 9d9dae3: the commands were removed from
master on 2026-10-18):
  ./bench/finders/run.py sound FILE --from start|opt|lns|restricted [--cap S] [--cpu S] [--jobs 4] GAME...
  ./bench/finders/run.py finders FILE [--cpu S] [--jobs 4] [--shuffle] GAME...
where a GAME is N, or A-B, or A-B/STEP (every STEP-th game of A..B), and the options not named here (`--fix`,
`--extra`, `--pool`, `--unit-nodes`, `--seed`) are given after the games and go to `wall` as they are.
"""
import argparse
import json
import random
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]  # the repository: this folder is solver/notebook/<entry>
CRATE = REPO / "solver" / "frame"
WALL = CRATE / "target" / "debug" / "examples" / "wall"
KNOWN = REPO / "results" / "strict.jsonl"


def games(texts):
    """The games of the arguments: N, A-B or A-B/STEP."""
    out = []
    for text in texts:
        span, _, step = text.partition("/")
        first, _, last = span.partition("-")
        out.extend(range(int(first), int(last or first) + 1, int(step or 1)))
    return out


def done_keys(path, key):
    """The keys of the lines a stage's file has."""
    if not path.exists():
        return set()
    return {key(json.loads(line)) for line in path.read_text().splitlines() if line.strip()}


def wall(args):
    """`wall ARGS` under `nice -n 10`: its output lines."""
    done = subprocess.run(["nice", "-n", "10", str(WALL), *map(str, args)], cwd=CRATE, capture_output=True, text=True)
    if done.returncode != 0:
        sys.exit(f"wall {' '.join(map(str, args))}: exit {done.returncode}\n{done.stdout}{done.stderr}")
    return done.stdout.splitlines()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("command", choices=["sound", "finders"])
    parser.add_argument("file", type=Path, help="the stage's file of lines, appended to")
    parser.add_argument("games", nargs="+")
    parser.add_argument("--from", dest="source", default="start")
    parser.add_argument("--cap")
    parser.add_argument("--cpu")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--shuffle", action="store_true", help="in a random order (the dear games are not all last)")
    args, extra = parser.parse_known_args()

    options = ["--known", KNOWN, *extra]
    if args.command == "sound":
        options += ["--from", args.source] + (["--cap", args.cap] if args.cap else [])
    if args.cpu:
        options += ["--cpu", args.cpu]

    def key(line):
        return (line["n"], line.get("from"))

    have = done_keys(args.file, key)
    todo = [n for n in dict.fromkeys(games(args.games)) if (n, args.source if args.command == "sound" else None) not in have]
    if args.shuffle:
        random.Random(1).shuffle(todo)
    print(f"{len(todo)} games to do, {len(have)} done", file=sys.stderr)

    def one(n):
        lines = wall([args.command, *options, n])
        with args.file.open("a") as out:  # one line a game, appended whole
            out.write("".join(line + "\n" for line in lines))
        return n

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for n in pool.map(one, todo):
            print(f"n={n} done", file=sys.stderr)


if __name__ == "__main__":
    main()
