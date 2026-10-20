#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The table of solver/notebook/search/flat-cycle-cuts-on-the-matching-lp-2026-10/README.md from its data.
Standard library only.

usage: table.py   (from anywhere)

Reads cycles.jsonl (the lines of `wall cycles`) and, for the A1 bound, the node-choice measurement's files
(solver/notebook/search/the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/cloud-20261018-a1/:
prepare.jsonl for a game's root, measure.jsonl.gz for a suite unit, matched by its game and `need`).  Prints, per
instance, the share of its gap that each bound closes, `(gf2 - bound) / gap`, the gap being `gf2 - target` (`target` the
optimum at a root, `need` at a unit, as the A1 measurements have it), then the medians by kind.
"""
import gzip
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
A1 = HERE.parent / "the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10" / "cloud-20261018-a1"
COLUMNS = [("cover", "cover"), ("a", "(a)"), ("b_hex", "(b) hex"), ("b", "(b)"), ("c", "(c)"), ("c_all", "(c) all")]


def a1_bounds():
    """The A1 bound of each root (by game) and unit (by game and need), with the GF(2) bound it was measured against."""
    roots, units = {}, {}
    for line in (A1 / "prepare.jsonl").read_text().splitlines():
        d = json.loads(line)
        roots[d["n"]] = (d["a1_bound"], d["root"])
    with gzip.open(A1 / "measure.jsonl.gz", "rt") as f:
        for line in f:
            d = json.loads(line)
            if d.get("source") == "suite" and d.get("a1"):
                units[(d["n"], d["need"])] = (d["a1"]["full"]["bound"], d["today"])
    return roots, units


def main():
    roots, units = a1_bounds()
    rows = [json.loads(line) for line in (HERE / "cycles.jsonl").read_text().splitlines() if line.strip()]
    header = ["instance", "gap"] + [name for _, name in COLUMNS] + ["A1", "GF(2) = A1's"]
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    shares = {key: {"root": [], "suite": []} for key, _ in COLUMNS + [("a1", "")]}
    for d in rows:
        kind = "root" if d["id"].endswith("root") else "suite"
        gap = d["gap"]
        cells = [d["id"], str(gap)]
        for key, _ in COLUMNS:
            share = (d["gf2"] - d[key]) / gap
            shares[key][kind].append(share)
            cells.append(f"{share:.2f}")
        a1 = roots.get(d["n"]) if kind == "root" else units.get((d["n"], d["target"]))
        if a1 is None:
            cells += ["", ""]
        else:
            share = (d["gf2"] - a1[0]) / gap
            shares["a1"][kind].append(share)
            cells += [f"{share:.2f}", "yes" if a1[1] == d["gf2"] else f"no ({a1[1]})"]
        print("| " + " | ".join(cells) + " |")
    print()
    print("| medians | " + " | ".join(name for _, name in COLUMNS) + " | A1 |")
    print("|---|" + "---|" * (len(COLUMNS) + 1))
    for kind in ("root", "suite"):
        cells = []
        for key, _ in COLUMNS + [("a1", "")]:
            values = shares[key][kind]
            cells.append(f"{statistics.median(values):.2f}" if values else "")
        print(f"| {kind} ({len(shares['a'][kind])}) | " + " | ".join(cells) + " |")
    print()
    print("| instance | (a) capped | cycle cuts by movers (b, c) | of two-factor movers only | stage seconds |")
    print("|---|---|---|---|---|")
    for d in rows:
        seconds = ", ".join(str(round(d[f"{key}_s"])) for key, _ in COLUMNS)
        print(
            f"| {d['id']} | {d['a_capped']} | {json.dumps(d['cycle_cuts_by_movers'])} | {d['cycle_cuts_of_two']} | "
            f"{seconds} |"
        )


if __name__ == "__main__":
    main()
