#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The plan of the later measurement: every item run with the floor off and on, for `batch.py`.

usage: python3 -I plan.py PLAIN_BIN [--units GAME,...] [--wholes GAME,...] [--labels off,on] [--only ID,...]
                                    > plan.jsonl

An *item* is a unit of the hardest searches' suite (suite-hard.jsonl, the units of the games `--units` names) or a whole
game (`--wholes`).  Its floor is `F = len(moves(n - 1)) - len(plain(n))` of its game n, in promotions (the entry's
README, "The floor"): `PLAIN_BIN` is the example `floor_plain`, `cargo build --profile test --features studies --example
floor_plain`.  The chain has no line for 4548, so its F takes the moves of 4547's.  A plan line is
`{"id", "game", "kind", "floor", "label", "nodes", "suite"}`: `label` is `off` or `on`, `floor` is null for `off`,
`suite` is the units' file from the top of the repository (null for a whole game), and `nodes` is the run's limit
(three times the unit's steps in the suite, 20 million for a whole game).  With both labels (the default) the two lines
of an item are next to each other, so that `batch.py`'s two workers run them side by side; `--labels on` plans the
floor-on runs alone (the floor-off run is then made only for an item where the floor applied, since where it never
applies the tree is the same), and `--only` keeps the items it names.  The items go in the order of the arguments: the
units of the games in `--units` order, the largest first, then the wholes.

A game whose chain line has fewer promotions than its floor is refused: the theorem would be false there.
"""
import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # the script's own directory is not on the path under `python3 -I`
from check_floor import chain, plain_lengths  # noqa: E402

REPO = HERE.parents[3]
SUITE = HERE.parent / "the-hardest-searches-of-the-run-to-5000-2026-10" / "suite-hard.jsonl"
UNIT_LIMIT = 3
WHOLE_LIMIT = 20_000_000


def floors(binary, games):
    """The floor of each game in promotions, and the chain's promotions of the game (None if the chain has no line)."""
    rows = chain()
    plain = plain_lengths(binary, games)
    out = {}
    for n in games:
        out[n] = (len(rows[n - 1]["moves"]) - plain[n], len(rows[n]["promotions"]) if n in rows else None)
    return out


def units(games):
    """The suite's units of the games, largest first within a game: (id, game, steps, best)."""
    found = []
    for line in SUITE.read_text().splitlines():
        row = json.loads(line)
        if row["kind"] == "unit" and row["game"] in games:
            found.append((row["id"], row["game"], row["default"]["nodes"], row["default"]["best"]))
    return sorted(found, key=lambda unit: (games.index(unit[1]), -unit[2]))


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("plain")
    parser.add_argument("--units", default="")
    parser.add_argument("--wholes", default="")
    parser.add_argument("--labels", default="off,on")
    parser.add_argument("--only", default="")
    args = parser.parse_args()
    unit_games = [int(n) for n in args.units.split(",") if n]
    whole_games = [int(n) for n in args.wholes.split(",") if n]
    floor = floors(args.plain, unit_games + whole_games)
    items = [(id_, game, "unit", UNIT_LIMIT * steps) for id_, game, steps, _ in units(unit_games)]
    items += [(f"{game}-root", game, "whole", WHOLE_LIMIT) for game in whole_games]
    for id_, game, kind, nodes in items:
        if args.only and id_ not in args.only.split(","):
            continue
        at, promoted = floor[game]
        if promoted is not None and promoted < at:
            sys.exit(f"{game}: the chain's game has {promoted} promotions, below the floor {at}")
        for label, at_least in (("off", None), ("on", at)):
            if label not in args.labels.split(","):
                continue
            print(json.dumps({"id": id_, "game": game, "kind": kind, "floor": at_least, "label": label,
                              "nodes": nodes, "suite": str(SUITE.relative_to(REPO)) if kind == "unit" else None}))


if __name__ == "__main__":
    main()
