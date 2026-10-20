#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Writes the start file of the whole games: their problems before they are solved.  Standard library only.

usage: python3 -I wholes_start.py --units UNITS.jsonl... --landscape LANDSCAPE.jsonl --out wholes-start.jsonl
                                  [--nominal 10000000]

The closure entry's `suite.py run` takes a suite file whose problems say what the default search takes: its steps set
the limit of each run (twice for the default search, three times for the learning).  The whole games were not probed
(their default search, counted, is the probe), so this file gives each a nominal size, 10 million steps by default:
limits of 20 and 30 million steps, which is the rule that a whole game over 20 million steps on one thread is left out.
Each line has the game's root gap less one (the suite's measure of a gap, the bound less what the node needs) from
`wall landscape --known results/strict.jsonl N`, and its best, P*, from the game line of its units file.  After the
runs, `make_suite.py assemble --runs RUNS.jsonl` writes the suite with the sizes the count runs found.
"""
import argparse
import json
from pathlib import Path


def lines_of(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--units", nargs="+", required=True, help="the units file of each whole game")
    p.add_argument("--landscape", required=True, help="the lines of `wall landscape` for the games")
    p.add_argument("--out", required=True)
    p.add_argument("--nominal", type=int, default=10_000_000)
    a = p.parse_args()
    roots = {line["n"]: line for line in lines_of(a.landscape)}
    out = [{"kind": "suite", "note": "the whole games before their runs: nominal sizes", "nominal": a.nominal}]
    for path in a.units:
        game = next(line for line in lines_of(path) if line["kind"] == "game")
        n, opt = game["game"], game["opt"]
        if opt is None or roots[n]["root_gap"] is None:
            raise SystemExit(f"{n}: a whole game needs its known optimum")
        default = {"nodes": a.nominal, "cpu_s": 0.0, "wall_s": 0.0, "best": opt, "completed": True}
        out.append({"kind": "whole", "game": n, "id": f"{n}-root", "gap": roots[n]["root_gap"] - 1,
                    "default": default})
    Path(a.out).write_text("".join(json.dumps(line) + "\n" for line in out))


if __name__ == "__main__":
    main()
