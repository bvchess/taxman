#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Makes the tables of the entry from its data files.  Standard library only.

usage: python3 -I tables.py [all|games|fit|suite|runs|totals|group|order|checks|cpu]
       python3 -I tables.py probed UNITS.jsonl...

The data files, beside this script: `suite.jsonl` (the problems), `probes.jsonl` (the default search's solves that sized
them), `runs.jsonl` (the lines of `wall images`, as the closure entry's `suite.py run` writes them), `checkpoints.jsonl`
and `cpu.jsonl` (the CPU of what the other files do not record).  The hard games' units were solved with the learning
only (`closure`), their default steps being the probes' (the steps do not depend on the build); the whole games and
their units were solved both ways (`count`, the default search, and `closure`), side by side.

Each table is printed as Markdown.  The pieces that do not depend on this entry (`percent`, `table`, `read_results`,
`problems`) are those of the closure entry's `suite.py`.
"""
import collections
import json
import math
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "conflicts-closed-under-cheap-exchanges-2026-10"))
import suite as old  # noqa: E402

SUITE, PROBES, RUNS = HERE / "suite.jsonl", HERE / "probes.jsonl", HERE / "runs.jsonl"
CHECKPOINTS, CPU = HERE / "checkpoints.jsonl", HERE / "cpu.jsonl"
PROBED, STOPPED = HERE / "probed-units.jsonl", HERE / "runs-stopped.jsonl"
MINUS = old.MINUS
# The bands of the default search's steps: from, up to but not including, and the name.
SIZES = [(0, 100_000, "under 100,000"), (100_000, 300_000, "100,000-299,999"), (300_000, 600_000, "300,000-599,999"),
         (600_000, 1_200_000, "600,000-1,199,999"), (1_200_000, 10**9, "1,200,000 and over")]


def read_jsonl(path):
    return [json.loads(line) for line in Path(path).read_text().splitlines() if line.strip()] if Path(path).exists() \
        else []


def load():
    """The problems, each with its default steps (the suite's), its learning run and its default run if it has one,
    and the games in the two groups: the hard games (units only) and the whole games with their units."""
    results = old.read_results(RUNS)
    problems = old.problems(SUITE)
    wholes = {p["game"] for p in problems if p["whole"]}
    rows = []
    for p in problems:
        rows.append({**p, "group": "whole" if p["game"] in wholes else "hard",
                     "learning": results.get((p["id"], "closure")), "default": results.get((p["id"], "count"))})
    return rows, results


def change(before, after):
    return old.percent(before, after)


def mean(values):
    return math.exp(statistics.fmean(math.log(x) for x in values))


# --- the suite ---------------------------------------------------------------------------------------------------
def suite_table():
    rows, _ = load()
    print("**The problems**: the default search's steps (the probes') and CPU seconds (the probes', of the release "
          "build of `tune solve`)\n")
    body = []
    for r in sorted(rows, key=lambda r: (r["group"] == "whole", r["game"], r["whole"], r["steps"])):
        kind = "whole game" if r["whole"] else "unit"
        body.append([r["id"], kind, r["gap"], f"{r['steps']:,}", f"{r['cpu_s']:.1f}"])
    old.table(["problem", "kind", "gap", "default steps", "probe CPU s"], body)


# --- the games ---------------------------------------------------------------------------------------------------
def games_table():
    """Each game: its checkpoint, the units chosen from it, and what probing cost."""
    lines = read_jsonl(SUITE)
    headers = [line for line in lines if line["kind"] == "game"]
    units = collections.defaultdict(list)
    for line in lines:
        if line["kind"] == "unit":
            units[line["game"]].append(line)
    cpu = {line["game"]: line["cpu_s"] for line in read_jsonl(CHECKPOINTS)}
    probes = read_jsonl(PROBES)
    print("**The games**: the checkpoint (a 120 s search on 4 threads), the units chosen from it and their probes\n")
    body = []
    for h in sorted(headers, key=lambda h: h["game"]):
        g, chosen = h["game"], units[h["game"]]
        mine = [p for p in probes if p["game"] == g]
        steps = sorted(u["default"]["nodes"] for u in chosen)
        body.append([g, f"{h['checkpoint_nodes']:,}", h["units_in_checkpoint"], h["incumbent_sum"],
                     h["opt"] if h["opt"] is not None else "unknown", f"{cpu.get(g, 0):.0f}", len(chosen),
                     sum("." in u["id"] for u in chosen), f"{steps[0]:,} to {steps[-1]:,}", len(mine),
                     sum(not p["completed"] for p in mine), f"{sum(p['cpu_s'] for p in mine):,.0f}"])
    old.table(["game", "checkpoint steps", "units in it", "its incumbent", "P*", "checkpoint CPU s", "units chosen",
               "of them expanded", "default steps", "probe solves", "stopped at the limit", "probe CPU s"], body)


# --- the gap band -------------------------------------------------------------------------------------------------
def fit_table():
    """The size of a unit against its gap, game by game, over the probes that finished: a line through the log of the
    steps, and the gaps at which it crosses 100,000 and 3,000,000 steps."""
    units = {line["id"]: line for line in read_jsonl(PROBED)}
    by_game = collections.defaultdict(list)
    for p in read_jsonl(PROBES):
        if p["completed"] and p["id"] in units and p["nodes"] > 0:
            by_game[p["game"]].append((units[p["id"]]["gap"], math.log10(p["nodes"])))
    print("**The size of a unit against its gap**, over the probes that finished (a unit that stopped at its limit is "
          "not in it, so the line is low for the large gaps)\n")
    body = []
    for g in sorted(by_game):
        xs, ys = zip(*by_game[g])
        slope, intercept = statistics.linear_regression(xs, ys)
        body.append([g, len(xs), f"{statistics.correlation(xs, ys):+.2f}", f"{100 * slope:.2f}",
                     f"{(5 - intercept) / slope:.0f}", f"{(math.log10(3e6) - intercept) / slope:.0f}"])
    old.table(["game", "units", "correlation with the gap", "decades of steps per 100 of gap",
               "gap at 100,000 steps", "gap at 3,000,000 steps"], body)


# --- the hard games' units -----------------------------------------------------------------------------------------
def runs_table():
    rows, _ = load()
    hard = [r for r in rows if r["group"] == "hard" and r["learning"]]
    print("**The units of the hard games**, by game and size: the default search's steps (the probe's) and the "
          "learning's steps, leaves and CPU\n")
    body = []
    for r in sorted(hard, key=lambda r: (r["game"], r["steps"])):
        b = r["learning"]
        body.append([r["id"], r["gap"], f"{r['steps']:,}", f"{b['nodes']:,}", change(r["steps"], b["nodes"]),
                     f"{b['leaves']:,}", f"{b['cpu_s']:.1f}", f"{1000 * b['cpu_s'] / b['nodes']:.3f}"])
    old.table(["unit", "gap", "default steps", "learning steps", "change", "learning leaves", "learning CPU s",
               "ms a step"], body)


def totals_table():
    rows, _ = load()
    hard = [r for r in rows if r["group"] == "hard" and r["learning"]]

    def line(label, rs):
        if not rs:
            return [label, 0] + ["-"] * 7
        steps, learned = sum(r["steps"] for r in rs), sum(r["learning"]["nodes"] for r in rs)
        ratios = [r["learning"]["nodes"] / r["steps"] for r in rs]
        cpu, ms = sum(r["learning"]["cpu_s"] for r in rs), 1000 * sum(r["learning"]["cpu_s"] for r in rs) / learned
        return [label, len(rs), f"{steps:,}", f"{learned:,}", change(steps, learned),
                change(1, statistics.median(ratios)), change(1, mean(ratios)), f"{cpu:.0f}", f"{ms:.3f}"]

    head = ["", "units", "default steps", "learning steps", "change: total", "median", "geometric mean",
            "learning CPU s", "ms a step"]
    print("**The hard games' units, by game**: the learning's steps against the default search's (the probes')\n")
    old.table(head, [line("all", hard)] + [line(str(g), [r for r in hard if r["game"] == g])
                                           for g in sorted({r["game"] for r in hard})])
    print("**By the steps of the default search**\n")
    old.table(head, [line(f"steps {name}", [r for r in hard if lo <= r["steps"] < hi]) for lo, hi, name in SIZES])
    if len(hard) > 2:
        changes = [math.log(r["learning"]["nodes"] / r["steps"]) for r in hard]
        by_gap = statistics.correlation([r["gap"] for r in hard], changes)
        by_size = statistics.correlation([math.log(r["steps"]) for r in hard], changes)
        print("Correlation (Pearson) of the log of the steps ratio, learning over default, with the gap: "
              f"{by_gap:+.2f}; with the log of the default's steps: {by_size:+.2f}.\n")


# --- a whole game against its units ----------------------------------------------------------------------------------
def pair(r):
    """(default run, learning run) of a problem of the whole-game group, or None if it lacks one."""
    return (r["default"], r["learning"]) if r["default"] and r["learning"] else None


def group_table():
    rows, _ = load()
    wholes = [r for r in rows if r["group"] == "whole"]
    games = sorted({r["game"] for r in wholes if r["whole"]})
    print("**Each problem of the whole games and their units**: the default search and the learning\n")
    head = ["problem", "gap", "steps", "learning", "change", "leaves", "learning", "change", "CPU s", "learning",
            "change"]
    body = []
    for g in games:
        for r in sorted((r for r in wholes if r["game"] == g), key=lambda r: (not r["whole"], r["steps"])):
            if pair(r):
                a, b = pair(r)
                body.append([r["id"], r["gap"], f"{a['nodes']:,}", f"{b['nodes']:,}", change(a["nodes"], b["nodes"]),
                             f"{a['leaves']:,}", f"{b['leaves']:,}", change(a["leaves"], b["leaves"]),
                             f"{a['cpu_s']:.1f}", f"{b['cpu_s']:.1f}", change(a["cpu_s"], b["cpu_s"])])
    old.table(head, body)
    for s in read_jsonl(STOPPED):
        r = next(r for r in wholes if r["id"] == s["id"])
        print(f"The learning on {s['id']} was stopped by hand at about {s['cpu_s']:,.0f} CPU s, "
              f"{s['cpu_s'] / r['default']['cpu_s']:.1f} times the default search's {r['default']['cpu_s']:,.0f} s "
              f"({r['default']['nodes']:,} steps, {r['default']['leaves']:,} leaves), and has no line.\n")

    print("**Each whole game against its own units**: the learning's change in the steps and in the CPU, for the game "
          "and for its units together and one by one\n")
    head = ["game", "problems", "default steps", "steps", "leaves", "CPU", "median unit: steps", "CPU", "ms a step: "
            "default", "learning"]
    body = []
    for g in games:
        whole = next(r for r in wholes if r["game"] == g and r["whole"])
        units = [r for r in wholes if r["game"] == g and not r["whole"] and pair(r)]
        for label, rs in ((f"{g} whole", [whole]), (f"{g} units", units)):
            rs = [r for r in rs if pair(r)]
            if not rs:
                continue
            a = {k: sum(pair(r)[0][k] for r in rs) for k in ("nodes", "leaves", "cpu_s")}
            b = {k: sum(pair(r)[1][k] for r in rs) for k in ("nodes", "leaves", "cpu_s")}
            steps = [pair(r)[1]["nodes"] / pair(r)[0]["nodes"] for r in rs]
            cpus = [pair(r)[1]["cpu_s"] / pair(r)[0]["cpu_s"] for r in rs]
            body.append([label, len(rs), f"{a['nodes']:,}", change(a["nodes"], b["nodes"]),
                         change(a["leaves"], b["leaves"]), change(a["cpu_s"], b["cpu_s"]),
                         change(1, statistics.median(steps)), change(1, statistics.median(cpus)),
                         f"{1000 * a['cpu_s'] / a['nodes']:.3f}", f"{1000 * b['cpu_s'] / b['nodes']:.3f}"])
    old.table(head, body)

    print("**The whole games run both ways, and the units of all three games, together**\n")
    body = []
    for label, rs in (("whole games", [r for r in wholes if r["whole"]]),
                      ("their units", [r for r in wholes if not r["whole"]])):
        rs = [r for r in rs if pair(r)]
        if not rs:
            continue
        a = {k: sum(pair(r)[0][k] for r in rs) for k in ("nodes", "leaves", "cpu_s")}
        b = {k: sum(pair(r)[1][k] for r in rs) for k in ("nodes", "leaves", "cpu_s")}
        body.append([label, len(rs), f"{a['nodes']:,}", f"{b['nodes']:,}", change(a["nodes"], b["nodes"]),
                     change(a["leaves"], b["leaves"]), f"{a['cpu_s']:.0f}", f"{b['cpu_s']:.0f}",
                     change(a["cpu_s"], b["cpu_s"])])
    old.table(["", "problems", "default steps", "learning steps", "change", "leaves", "default CPU s", "learning CPU s",
               "change"], body)


# --- the difficulty order ----------------------------------------------------------------------------------------
def order_table(count=20):
    rows, _ = load()
    done = [r for r in rows if r["learning"]]
    print(f"**The {count} dearest problems with the learning on**, by CPU seconds\n")
    body = []
    for rank, r in enumerate(sorted(done, key=lambda r: -r["learning"]["cpu_s"])[:count], 1):
        b = r["learning"]
        body.append([rank, r["id"], r["gap"], f"{r['steps']:,}", f"{b['nodes']:,}", f"{b['leaves']:,}",
                     f"{b['cpu_s']:.1f}", f"{1000 * b['cpu_s'] / b['nodes']:.3f}"])
    old.table(["rank", "problem", "gap", "default steps", "learning steps", "learning leaves", "learning CPU s",
               "ms a step"], body)


# --- the checks ----------------------------------------------------------------------------------------------------
def checks():
    rows, results = load()
    found = [text for r in rows for text in old.complaints(r, results)]
    ran = [r for r in rows if r["learning"]]
    print(f"Checks over {len(ran)} problems with a learning run: "
          + ("all agree: the same best and conclusion as the default search's (the probe's), the known optimum's where "
             "there is one, and the default runs' steps the suite's." if not found else "\n".join(found)))
    unknown = [r for r in ran if r["learning"]["opt"] is None]
    if unknown:
        print("\n**Games with no known optimum**: what the learning found against the incumbent the units started "
              "from and against the default search's best (the probe's)\n")
        old.table(["problem", "incumbent", "default best", "learning best", "better than the incumbent"],
                  [[r["id"], r["learning"]["incumbent"], r["best"], r["learning"]["best"],
                    "yes" if r["learning"]["best"] > r["learning"]["incumbent"] else "no"] for r in unknown])


# --- the CPU spent --------------------------------------------------------------------------------------------------
def cpu_table():
    probes = read_jsonl(PROBES)
    runs = read_jsonl(RUNS)
    rows = []

    def add(label, items, key="cpu_s"):
        rows.append([label, len(items), f"{sum(i[key] for i in items):,.0f}"])

    add("checkpoints (`taxman solve`, 4 threads, 120 s)", read_jsonl(CHECKPOINTS))
    ledger = read_jsonl(CPU)
    groups = collections.defaultdict(list)
    for item in ledger:
        groups[item["label"].split(":")[0]].append(item)
    for label in sorted(groups):
        add(label, groups[label])
    done = [p for p in probes if p["completed"]]
    add("probes that finished", done)
    add("probes that stopped at their limit", [p for p in probes if not p["completed"]])
    for mode, label in (("count", "runs: the default search, counted"), ("closure", "runs: the learning")):
        add(label, [r for r in runs if r["mode"] == mode])
    total = sum(float(r[2].replace(",", "")) for r in rows)
    old.table(["what", "runs", "CPU s"], rows)
    print(f"In all {total:,.0f} CPU seconds, {total / 3600:.2f} hours.\n")


COMMANDS = {"games": games_table, "fit": fit_table, "suite": suite_table, "runs": runs_table,
            "totals": totals_table, "group": group_table, "order": order_table, "checks": checks, "cpu": cpu_table}


def write_probed(units_files):
    """`probed-units.jsonl`: the gap and depth of every unit the probes tried, from the units files of `tune units`
    (and `tune expand`), which are not kept."""
    tried = {p["id"] for p in read_jsonl(PROBES)}
    keep = ("id", "game", "index", "depth", "n_cands", "gap")
    lines = [{k: line[k] for k in keep} for path in units_files for line in read_jsonl(path)
             if line["kind"] == "unit" and line["id"] in tried]
    PROBED.write_text("".join(json.dumps(line) + "\n" for line in lines))
    print(f"{len(lines)} units in {PROBED}")


def main():
    if sys.argv[1:2] == ["probed"]:
        return write_probed(sys.argv[2:])
    wanted = sys.argv[1:] or ["all"]
    for name in (COMMANDS if wanted == ["all"] else wanted):
        COMMANDS[name]()


if __name__ == "__main__":
    main()
