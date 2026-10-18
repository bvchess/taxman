#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Runs the target experiment on the laptop: `wall target` over a list of games, a few processes at once under
`nice -n 10`, one process a game, each line appended to the file as its game finishes.  A run that is interrupted goes
on where it was: the games the file has are skipped.

usage (from rust/, after `cargo build --profile test --example wall`):
  ./bench/target/run.py FILE [--jobs 4] [--fractions F,F,...] [--wall PATH]

The games and the CPU cap of each are in GAMES: the cap is 1.3 times the time of the game's standalone search from
its start incumbent in ../landscape/sa/ (one thread, the c8g instance), a little above what the full proof (the sound
search from the known optimum, which takes fewer nodes) needs.
"""
import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUST = HERE.parents[1]
WALL = RUST / "target" / "debug" / "examples" / "wall"
KNOWN = RUST / "results" / "strict.jsonl"

# game: seconds of its standalone search in ../landscape/sa/.  The first ten are the main games (300 to 1800 s there,
# 286 for 3259); the four after them are cheap extras for the shape of the curve.
MAIN = {2265: 493, 2299: 797, 2423: 1033, 2485: 607, 2528: 1036, 2555: 448, 3156: 1450, 3204: 1238, 3221: 648,
        3259: 286}
EXTRA = {2416: 87, 2560: 118, 2696: 148, 3064: 228}
GAMES = {**MAIN, **EXTRA}

# The shares of the gap: the three of the question (0.25, 0.5, 0.75) and others, for the curve.
FRACTIONS = "0.1,0.2,0.25,0.3,0.4,0.5,0.6,0.7,0.75,0.8,0.9"


def done_games(path):
    """The games the file has a line of."""
    if not path.exists():
        return set()
    return {json.loads(line)["n"] for line in path.read_text().splitlines() if line.strip()}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("file", type=Path, help="the file of lines, appended to")
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--fractions", default=FRACTIONS)
    parser.add_argument("--wall", type=Path, default=WALL)
    args = parser.parse_args()

    have = done_games(args.file)
    # The dearest first, so that the last games to finish are the cheap ones.
    todo = [n for n in sorted(GAMES, key=GAMES.get, reverse=True) if n not in have]
    print(f"{len(todo)} games to do, {len(have)} done", file=sys.stderr)

    def one(n):
        cap = round(GAMES[n] * 1.3)
        command = ["nice", "-n", "10", str(args.wall), "target", "--known", str(KNOWN), "--fractions", args.fractions,
                   "--cap", str(cap), str(n)]
        done = subprocess.run(command, cwd=RUST, capture_output=True, text=True)
        if done.returncode != 0:
            sys.exit(f"{' '.join(command)}: exit {done.returncode}\n{done.stdout}{done.stderr}")
        with args.file.open("a") as out:  # one line a game, appended whole
            out.write(done.stdout)
        return n

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for n in pool.map(one, todo):
            print(f"n={n} done", file=sys.stderr)


if __name__ == "__main__":
    main()
