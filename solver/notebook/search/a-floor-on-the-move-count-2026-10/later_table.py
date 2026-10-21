#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of the entry's section "Later (2026-10-21): more searches" from the runs of `batch.py`.

usage: python3 -I later_table.py [all|entry] [LATER_RUNS.jsonl] [FIRST_RUNS.jsonl]

LATER_RUNS.jsonl is the results file of `batch.py` (later-runs.jsonl), FIRST_RUNS.jsonl the first probe's (runs.jsonl),
whose two whole games, 4176 and 4050, join the later whole games.  An *item* is a unit or a whole game with its
floor-off and floor-on runs.  Where an item has a floor-on run in which the floor applied at no check and no floor-off
run, the tree is the floor-off tree (the floor is a cut, and a check where it does not apply changes nothing), so the
item counts with the steps of its floor-on run, and its CPU is left out of the CPU columns.  `entry` prints the entry's table: the
units of each game together, each whole game, and the three totals (the steps saved over all the units together, over
all the whole games together, and the share of items with a cut).  `all` (the default) prints also a row for each unit,
which `later-tables.txt` holds.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
MINUS = "−"


def read(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()]


def cpu(run):
    return run["user_s"] + run["sys_s"]


def change(before, after):
    """The change from `before` to `after` in percent, as the entry prints it (a true minus sign)."""
    if before == 0:
        return "-"
    pct = 100 * (after - before) / before
    return f"{pct:+.1f}%".replace("-", MINUS) if abs(pct) >= 0.05 else "0.0%"


def items(later, first):
    """The items in the order of the runs: {id: {"off", "on", "kind", "game", "derived"}}.  The first probe's lines for
    4176 and 4050 are wholes (labels `default` for off and `floor` for on).  An item whose floor applied nowhere and
    that has no floor-off run takes its floor-on run for the floor-off one (`derived`)."""
    found = {}
    for run in first:
        if run["label"] in ("default", "floor"):
            side = "off" if run["label"] == "default" else "on"
            found.setdefault(run["id"], {"kind": "whole", "game": run["n"]})[side] = run
    for run in later:
        found.setdefault(run["id"], {"kind": run["kind"], "game": run["n"]})[run["label"]] = run
    for id_, item in list(found.items()):
        item["derived"] = "off" not in item and "on" in item and item["on"]["floor_applies"] == 0
        if item["derived"]:
            item["off"] = item["on"]
        if "off" not in item or "on" not in item:
            print(f"(left out: {id_} has no {'off' if 'off' not in item else 'on'} run)")
            del found[id_]
    return found


def unit_row(id_, item):
    off, on = item["off"], item["on"]
    cpu_change = "-" if item["derived"] else change(cpu(off), cpu(on))
    return (f"| {id_} | {on['floor_first_promoted']} | {on['floor']} | {off['steps']:,} | {on['steps']:,} | "
            f"{change(off['steps'], on['steps'])} | {cpu_change} | {on['floor_open']:,} | {on['floor_tight']:,} | "
            f"{on['floor_applies']:,} | {on['floor_cuts']:,} |")


def total(rows):
    """The steps of a set of items, off and on, the CPU seconds of those with a floor-off run, how many have a check
    where the floor applied, how many have a cut, and the checks and nodes cut."""
    timed = [r for r in rows if not r["derived"]]
    return {"items": len(rows), "off": sum(r["off"]["steps"] for r in rows), "on": sum(r["on"]["steps"] for r in rows),
            "cpu_off": sum(cpu(r["off"]) for r in timed), "cpu_on": sum(cpu(r["on"]) for r in timed),
            "timed": len(timed), "cut": sum(r["on"]["floor_cuts"] > 0 for r in rows),
            "applies": sum(r["on"]["floor_applies"] for r in rows), "cuts": sum(r["on"]["floor_cuts"] for r in rows),
            "open": sum(r["on"]["floor_open"] for r in rows)}


def line(name, t, floor=""):
    """A row of the entry's table.  The CPU columns cover the items that have a floor-off run; a note says how many."""
    cpu_columns = (f"{t['cpu_off']:,.0f} | {t['cpu_on']:,.0f} | {change(t['cpu_off'], t['cpu_on'])}"
                   if t["timed"] else "- | - | -")
    note = "" if t["timed"] in (0, t["items"]) else f" ({t['timed']} of {t['items']})"
    return (f"| {name} | {t['items']} | {floor} | {t['off']:,} | {t['on']:,} | {change(t['off'], t['on'])} | "
            f"{cpu_columns}{note} | {t['open']:,} | {t['applies']:,} | {t['cuts']:,} | {t['cut']} of {t['items']} |")


def entry(found):
    unit = {id_: it for id_, it in found.items() if it["kind"] == "unit"}
    whole = {id_: it for id_, it in found.items() if it["kind"] == "whole"}
    print("| items | n | floor | steps, off | steps, on | steps | CPU s, off | CPU s, on | CPU | open checks "
          "| checks where the floor applied | nodes cut | items with a cut |")
    print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for game in sorted({it["game"] for it in unit.values()}):
        rows = [it for it in unit.values() if it["game"] == game]
        print(line(f"units of {game}", total(rows), rows[0]["on"]["floor"]))
    for id_, it in whole.items():
        print(line(f"{id_.removesuffix('-root')} whole", total([it]), it["on"]["floor"]))
    print(line("all units", total(list(unit.values()))))
    print(line("all whole games", total(list(whole.values()))))
    print(line("all items", total(list(found.values()))))


def units_table(found):
    print("| unit | depth | floor | steps, off | steps, on | steps | CPU | open checks | tight "
          "| checks where the floor applied | nodes cut |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for id_, it in found.items():
        if it["kind"] == "unit":
            print(unit_row(id_, it))


def main(mode, later_path, first_path):
    found = items(read(later_path), read(first_path))
    entry(found)
    if mode == "all":
        print()
        units_table(found)


if __name__ == "__main__":
    args = sys.argv[1:]
    mode = args.pop(0) if args and args[0] in ("all", "entry") else "all"
    main(mode, args[0] if args else HERE / "later-runs.jsonl", args[1] if len(args) > 1 else HERE / "runs.jsonl")
