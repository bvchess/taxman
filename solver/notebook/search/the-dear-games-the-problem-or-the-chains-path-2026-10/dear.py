#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The dear games: the problem or the chain's path.

The entry is solver/notebook/search/the-dear-games-the-problem-or-the-chains-path-2026-10/README.md.

The 20 searched games of 2176-3312 whose trees are most above the cost model of the gap past 2175 (the model
`log10 nodes = a + b G / n^0.3` fitted to the EC2 chain's 400 exhausted searches) and the 20 most below it, each run
from two starts on one thread with `wall nodes`:

* a, the proof from opt(n): `--mode opt` (the incumbent is opt(n) with no set, no seed and no ceiling);
* b, the search from the start the chain would have in order: `--mode start` (greedy set, the seed of game n - 1 from
  results/strict.jsonl, dive, local search; the ceiling of game n - 1);
* c, the search from the start of no seed: `--mode start --no-seed`;
* f, the proof from opt(n) with the tactic of the chain's build: `--mode opt --tactic force_window=80` (the
  chain of the snapshot ran with the forcing window at 80; the default is `all` since 2026-10-16, which makes
  smaller trees);
* dK (d50), the proof with the incumbent K below opt(n) and no set behind it: `--mode opt --delta -K`.

    ./dear.py select > games.jsonl            the 40 games (needs the data of ../the-gap-past-2175-2026-10)
    ./dear.py run [a b c] [N ...] [--limit STEPS]   each game (only the games N, if any) from each start, cheapest
                                              first, appended to runs.jsonl; resumes where it stopped; waits while
                                              the 1-minute load is above LOAD
    ./dear.py starts [TOP]                    the in-order start of the TOP searched games past A4's snapshot (3313 on)
                                              that took the most CPU time (all by default), appended to
                                              starts.jsonl; resumes where it stopped
    ./dear.py report                          the tables of the entry

`select` and `report` read the inputs named in this file; `run` needs `wall` built (`cargo build -j 2 --profile test
--example wall --features studies` in solver/frame).
"""
import importlib.util
import json
import math
import os
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
FRAME = REPO / "solver" / "frame"
WALL = FRAME / "target" / "debug" / "examples" / "wall"
STRICT = REPO / "results" / "strict.jsonl"
GAP_PY = HERE.parent / "the-gap-past-2175-2026-10" / "gap.py"
GAMES = HERE / "games.jsonl"
RUNS = HERE / "runs.jsonl"
STARTS = HERE / "starts.jsonl"

PER_GROUP = 20  # games with the largest and with the smallest residuals
REFERENCE = (3020,)  # the game whose 317,361 nodes against 35,857 from opt(n) made the question, run besides
STEP_LIMIT = 5_000_000  # steps a run may take
LOAD = 8.0  # the 1-minute load above which a run waits
LOAD_WAIT = 30  # seconds between looks at the load
NICE = 10
MODES = {
    "a": ["--mode", "opt"],
    "b": ["--mode", "start"],
    "c": ["--mode", "start", "--no-seed"],
    "f": ["--mode", "opt", "--tactic", "force_window=80"],
}
KEPT = ["n", "mode", "delta", "completed", "nodes", "seconds", "start", "opt", "root_bound", "gap_opt", "best",
        "node_limit", "busy_s", "max_rss"]


def load_gap_module():
    spec = importlib.util.spec_from_file_location("gap", GAP_PY)
    gap = importlib.util.module_from_spec(spec)
    sys.modules["gap"] = gap
    spec.loader.exec_module(gap)
    return gap


def read_jsonl(path):
    if not Path(path).exists():
        return []
    with open(path) as f:
        return [json.loads(text) for text in f if text.strip()]


def strict_games():
    return {r["n"]: r for r in read_jsonl(STRICT) if r.get("completed")}


# ---------------------------------------------------------------------------------------------------------
# select

def model(gap):
    """The residuals of the cost model on G / n^p for the EC2 chain's exhausted searches: the rows with their
    residuals, and the model's coefficients."""
    games = gap.load_games()
    rows, fit, residuals = gap.residual_series(games, gap.P)
    return games, rows, fit, residuals


def select():
    gap = load_gap_module()
    games, rows, fit, residuals = model(gap)
    _, _, residuals0 = gap.residual_series(games, 0.0)
    by_n = {g["n"]: g for g in games}
    strict = strict_games()
    out = []
    for r, e, e0 in zip(rows, residuals, residuals0):
        g = by_n[r["n"]]
        out.append({"n": r["n"], "gap": r["gap"], "residual": round(e, 3), "residual_gap_alone": round(e0, 3),
                    "chain_nodes": g["nodes"], "chain_seconds": g["search_s"], "strict_nodes": strict[r["n"]]["nodes"],
                    "start_below_opt": g["opt"] - g["start"], "opt": g["opt"], "start": g["start"]})
    ranked = sorted(out, key=lambda r: -r["residual"])
    for group, part in (("dear", ranked[:PER_GROUP]), ("cheap", ranked[-PER_GROUP:]),
                        ("reference", [r for r in out if r["n"] in REFERENCE])):
        for r in part:
            print(json.dumps({"group": group, **r}, separators=(",", ":")))


# ---------------------------------------------------------------------------------------------------------
# run

def wait_for_quiet():
    while os.getloadavg()[0] > LOAD:
        time.sleep(LOAD_WAIT)


def is_mode(word):
    return word in MODES or re.fullmatch(r"d\d+", word) is not None


def mode_arguments(mode):
    """The arguments of `wall nodes` for a start: a letter of MODES, or `dK`, the proof with the incumbent K below
    opt(n) (`--delta -K`, no set behind it)."""
    return MODES[mode] if mode in MODES else ["--mode", "opt", "--delta", f"-{mode[1:]}"]


def run_one(n, mode, limit):
    """One `wall nodes` run on one thread; the JSON line of the game."""
    wait_for_quiet()
    command = [str(WALL), "nodes", "--known", str(STRICT), "--nodes", str(limit), *mode_arguments(mode), str(n)]
    done = subprocess.run(command, cwd=FRAME, capture_output=True, text=True, preexec_fn=lambda: os.nice(NICE))
    if done.returncode != 0:
        sys.exit(f"{' '.join(command)}\n{done.stderr}")
    lines = [json.loads(text) for text in done.stdout.splitlines() if text.startswith("{")]
    if len(lines) != 1:
        sys.exit(f"{' '.join(command)}: {len(lines)} lines\n{done.stdout}\n{done.stderr}")
    return lines[0]


def run(modes, limit, only):
    """Each game (the ones in `only` if any), cheapest first by the chain's nodes, from each start in `modes`."""
    games = sorted(read_jsonl(GAMES), key=lambda g: g["chain_nodes"])
    have = {(r["n"], r["run"]) for r in read_jsonl(RUNS)}
    for g in games:
        for mode in modes:
            if (g["n"], mode) in have or (only and g["n"] not in only):
                continue
            line = run_one(g["n"], mode, limit)
            kept = {k: line[k] for k in KEPT if k in line}
            record = {"run": mode, "limit": limit, **kept}
            with open(RUNS, "a") as f:
                f.write(json.dumps(record, separators=(",", ":")) + "\n")
            print(json.dumps(record, separators=(",", ":")), flush=True)


def starts(top):
    """The start of the searched games of the committed chain past A4's snapshot (the `top` that took the most CPU
    time, all if 0), as the chain would have it in order (`wall nodes --nodes 1`, so no search): how far below
    opt(n) it is.  Appended to starts.jsonl; resumes."""
    gap = load_gap_module()
    strict = strict_games()
    have = {r["n"] for r in read_jsonl(STARTS)}
    searched = [n for n, r in strict.items() if n > gap.LAST and r["method"] == "search"]
    searched.sort(key=lambda n: -strict[n]["seconds"]["search_all_runs"] * strict[n]["threads"])
    for n in sorted(searched[:top or len(searched)]):
        r = strict[n]
        if n in have:
            continue
        line = run_one(n, "b", 1)
        record = {"n": n, "start": line["start"], "opt": line["opt"], "gap": line["gap_opt"], "proof": r["proof"],
                  "chain_nodes": r["nodes"], "chain_seconds": round(r["seconds"]["search_all_runs"], 3),
                  "threads": r["threads"]}
        with open(STARTS, "a") as f:
            f.write(json.dumps(record, separators=(",", ":")) + "\n")


# ---------------------------------------------------------------------------------------------------------
# report

def main(argv):
    command = argv[0] if argv else "report"
    if command == "select":
        select()
    elif command == "starts":
        starts(int(argv[1]) if len(argv) > 1 else 0)
    elif command == "run":
        rest = argv[1:]
        limit = STEP_LIMIT
        if "--limit" in rest:
            i = rest.index("--limit")
            limit = int(rest[i + 1])
            del rest[i:i + 2]
        only = {int(a) for a in rest if a.isdigit()}
        run([a for a in rest if is_mode(a)] or ["a", "b"], limit, only)
    elif command == "report":
        print(report())
    else:
        sys.exit(__doc__)


# ---------------------------------------------------------------------------------------------------------
# The tables

def mean(v):
    return sum(v) / len(v)


def sd(v):
    m = mean(v)
    return math.sqrt(sum((x - m) ** 2 for x in v) / (len(v) - 1))


def median(v):
    s = sorted(v)
    return (s[(len(s) - 1) // 2] + s[len(s) // 2]) / 2


def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out.extend("| " + " | ".join(str(c) for c in row) + " |" for row in rows)
    return "\n".join(out)


class Entry:
    """One game with its runs: the chain's steps and each start's.  A run that stopped at its limit of steps is a
    lower bound (`limited`).  A b that was not run is a's when the start is opt(n) (the same tree: the games where
    both ran show it)."""

    def __init__(self, game, runs):
        self.game = game
        self.n = game["n"]
        self.runs = {mode: runs.get((self.n, mode)) for mode in "abcf"}

    def run(self, mode):
        run = self.runs[mode]
        if run is None and mode == "b" and self.game["start_below_opt"] == 0:
            return self.runs["a"]
        return run

    def steps(self, mode):
        run = self.run(mode)
        return None if run is None else run["nodes"]

    def limited(self, mode):
        run = self.run(mode)
        return run is not None and not run["completed"]

    def text(self, mode):
        s = self.steps(mode)
        if s is None:
            return "-"
        return f"{'≥' if self.limited(mode) else ''}{s:,}{'*' if self.runs[mode] is None else ''}"

    def ratio(self, mode):
        """The chain's steps over a start's, as log10 (None where there is no finished run)."""
        s = self.steps(mode)
        return None if s is None or self.limited(mode) else math.log10(self.game["chain_nodes"] / s)


def penalty_curve(runs):
    """What a start `d` below opt(n) costs, as a multiple of the proof from opt(n): game 3020's sweep (`dK`
    runs over a's steps), as a non-decreasing list of (d, multiple)."""
    base = runs[(3020, "a")]["nodes"]
    points = sorted((-r["delta"], r["nodes"] / base) for (n, mode), r in runs.items() if n == 3020 and mode[0] == "d")
    curve, top = [(0, 1.0)], 1.0
    for d, multiple in points:
        top = max(top, multiple)
        curve.append((d, top))
    return curve


def chain_start_curve(entries):
    """What the starts the chain would have in order cost, from the games run from both: b over a, in bins of how far
    the start is below opt(n) (the mean of the bin and the geometric mean of the ratios), as a list of
    (d, multiple) that never falls."""
    points = sorted((e.game["start_below_opt"], e.steps("b") / e.steps("a")) for e in entries
                    if e.runs["b"] is not None and e.runs["a"] is not None and e.game["start_below_opt"] > 0
                    and not e.limited("a") and not e.limited("b"))
    curve, top = [(0, 1.0)], 1.0
    for low, high in ((1, 49), (50, 99), (100, 199), (200, 10 ** 9)):
        part = [(d, m) for d, m in points if low <= d <= high]
        if part:
            top = max(top, 10 ** mean([math.log10(m) for _, m in part]))
            curve.append((round(mean([d for d, _ in part])), top))
    return curve


def penalty(curve, d):
    """The multiple at a start `d` below opt(n), by log-linear interpolation, and the last measured multiple
    beyond the last measured `d` (no extrapolation)."""
    if d <= 0:
        return 1.0
    for (d0, m0), (d1, m1) in zip(curve, curve[1:]):
        if d <= d1:
            return 10 ** (math.log10(m0) + (math.log10(m1) - math.log10(m0)) * (d - d0) / (d1 - d0))
    return curve[-1][1]


BANDS = [(0, 0, "0"), (1, 49, "1-49"), (50, 99, "50-99"), (100, 149, "100-149"), (150, 249, "150-249"),
         (250, 10 ** 9, "250 or more")]


def report():
    gap = load_gap_module()
    _, _, fit, _ = model(gap)
    intercept, slope = fit["coef"]
    runs = {(r["n"], r["run"]): r for r in read_jsonl(RUNS)}
    entries = [Entry(g, runs) for g in read_jsonl(GAMES)]
    order = {"dear": 0, "cheap": 1, "reference": 2}
    entries.sort(key=lambda e: (order[e.game["group"]], e.n))

    def predicted(e):
        return intercept + slope * e.game["gap"] / e.n ** gap.P

    def residual(e, mode):
        s = e.steps(mode)
        return None if s is None else math.log10(s) - predicted(e)

    out = [f"The cost model: log10 nodes = {intercept:.3f} + {slope:.5f} G / n^{gap.P}, for one thread in order (the "
           f"chain's nodes less {-(gap.SPECULATION + gap.ONE_THREAD):.2f} decades).", "", "### The games", ""]
    header = ["group", "n", "gap", "residual", "start below opt(n)", "chain's nodes", "b: the chain's start",
              "a: opt(n)", "chain / a", "residual of a", "f: a, the chain's build", "chain / f"]
    rows = []
    for e in entries:
        g = e.game
        a, f = e.steps("a"), e.steps("f")
        ra = residual(e, "a")
        rows.append([g["group"], e.n, g["gap"], f"{g['residual']:+.2f}", g["start_below_opt"],
                     f"{g['chain_nodes']:,}", e.text("b"), e.text("a"),
                     "-" if a is None else f"{'≤' if e.limited('a') else ''}{g['chain_nodes'] / a:.1f}",
                     "-" if ra is None else f"{'≥' if e.limited('a') else ''}{ra:+.2f}", e.text("f"),
                     "-" if f is None else f"{g['chain_nodes'] / f:.1f}"])
    out += [table(header, rows), "", "A star marks a b that was not run: the start is opt(n), so b is a.  A "
            "`≥` or `≤` marks a run that stopped at its limit of steps.", ""]

    out += ["### The groups", ""]
    header = ["group", "games", "mean gap", "mean residual: chain", "b", "a", "geometric mean steps: chain", "b", "a",
             "sum of steps: chain", "b", "a"]
    rows, means = [], {}
    for group in ("dear", "cheap"):
        part = [e for e in entries if e.game["group"] == group and e.steps("a") is not None and not e.limited("a")]
        means[group] = {"chain": mean([e.game["residual"] for e in part]),
                        "b": mean([residual(e, "b") for e in part]), "a": mean([residual(e, "a") for e in part])}
        rows.append([group, len(part), f"{mean([e.game['gap'] for e in part]):.0f}", f"{means[group]['chain']:+.2f}",
                     f"{means[group]['b']:+.2f}", f"{means[group]['a']:+.2f}"] +
                    [f"{10 ** mean([math.log10(x) for x in xs]):,.0f}" for xs in (
                        [e.game["chain_nodes"] for e in part], [e.steps("b") for e in part],
                        [e.steps("a") for e in part])] +
                    [f"{sum(xs):,}" for xs in ([e.game["chain_nodes"] for e in part], [e.steps("b") for e in part],
                                               [e.steps("a") for e in part])])
    searched = {g["n"]: g for g in gap.load_games()}
    cpu = {n: g["search_s"] * g["threads"] for n, g in searched.items()}
    shares = {group: sum(cpu[e.n] for e in entries if e.game["group"] == group) for group in order}
    out += [table(header, rows), "",
            f"The chain's search time over its {len(searched)} searched games of 2176-{gap.LAST} is "
            f"{sum(cpu.values()) / 3600:,.0f} CPU hours.  The {PER_GROUP} dear games took {shares['dear'] / 3600:.0f} "
            f"({100 * shares['dear'] / sum(cpu.values()):.1f}%), the {PER_GROUP} cheap games "
            f"{shares['cheap'] / 3600:.1f} ({100 * shares['cheap'] / sum(cpu.values()):.2f}%).", "",
            "The dear games' residual above the cheap games', in decades, from each start "
            "(the games whose proof from opt(n) finished):", ""]
    delta = {k: means["dear"][k] - means["cheap"][k] for k in ("chain", "b", "a")}
    out += [table(["from", "dear less cheap", "share of the chain's"],
                  [[k, f"{delta[k]:.2f}", f"{delta[k] / delta['chain']:.2f}"] for k in ("chain", "b", "a")]), ""]

    out += ["### The chain's build", "",
            "The chain ran with the forcing window at 80; the default is `all`.  Proofs from opt(n) with 80 (f) "
            "against the default (a), and the chain against f, on the games with an f:", ""]
    header = ["group", "games", "f / a: geometric mean", "lowest", "highest", "chain / f: geometric mean", "lowest",
              "highest"]
    rows = []
    calibration = {}
    for group in ("dear", "cheap", "reference"):
        part = [e for e in entries if e.game["group"] == group and e.steps("f") is not None]
        if not part:
            continue
        fa = [math.log10(e.steps("f") / e.steps("a")) for e in part]
        cf = [math.log10(e.game["chain_nodes"] / e.steps("f")) for e in part]
        calibration[group] = (mean(fa), mean(cf))
        rows.append([group, len(part), f"{10 ** mean(fa):.2f}", f"{10 ** min(fa):.2f}", f"{10 ** max(fa):.2f}",
                     f"{10 ** mean(cf):.2f}", f"{10 ** min(cf):.2f}", f"{10 ** max(cf):.2f}"])
    out += [table(header, rows), ""]
    if "dear" in calibration and "cheap" in calibration:
        shift = calibration["dear"][0] - calibration["cheap"][0]
        out += [f"The forcing window's 80 makes the dear games' proofs {10 ** shift:.2f} times as much (against the "
                f"cheap games') on these; with that, the dear games' residual above the cheap games' from opt(n) "
                f"is {delta['a'] + shift:.2f} decades, {(delta['a'] + shift) / delta['chain']:.2f} of the "
                f"chain's {delta['chain']:.2f}.", ""]

    out += ["### What a start below opt(n) costs", ""]
    curve = penalty_curve(runs)
    a3020 = runs[(3020, "a")]["nodes"]
    sweeps = sorted((-r["delta"], r["nodes"]) for (n, mode), r in runs.items() if n == 3020 and mode[0] == "d")
    rows = [[0, f"{a3020:,}", "1.00"]] + [[d, f"{s:,}", f"{s / a3020:.2f}"] for d, s in sweeps]
    out += ["Game 3020, the proof with the incumbent d below opt(n) and no set behind it (236 is where the start of no "
            f"seed lies), the proof from opt(n) having {a3020:,} steps and the chain's search "
            f"{next(e for e in entries if e.n == 3020).game['chain_nodes']:,}:", "",
            table(["d", "steps", "times the proof from opt(n)"], rows), ""]
    c_rows = []
    for e in entries:
        if e.runs["c"] is not None:
            c_rows.append([e.n, f"{e.game['chain_nodes']:,}", e.text("a"), e.text("c"),
                           f"{e.runs['c']['opt'] - e.runs['c']['start']}",
                           f"{e.steps('c') / e.steps('a'):.1f}"])
    out += ["The search from the start of no seed (c):", "",
            table(["n", "chain's nodes", "a", "c", "c's start below opt(n)", "c / a"], c_rows), ""]
    b_rows = []
    for e in entries:
        if e.runs["b"] is not None and e.game["start_below_opt"] > 0 and not e.limited("a"):
            b_rows.append((e.game["start_below_opt"], e.n, e.steps("b") / e.steps("a")))
    b_rows.sort()
    out += ["The search from the chain's start (b) against the proof from opt(n) (a), the games whose start is "
            "below opt(n):", "",
            table(["start below opt(n)", "n", "b / a"], [[d, n, f"{r:.2f}"] for d, n, r in b_rows]), ""]
    out += worth(gap, entries, [("game 3020's sweep", curve), ("the chain's starts in order (b over a)",
                                                              chain_start_curve(entries))])
    return "\n".join(out)


def worth(gap, entries, curves):
    """What a start at opt(n) would take from the chain's search time, from the starts the chain would have in
    order and the curves of what they cost."""
    out = ["### What a start at opt(n) is worth", ""]
    out += ["The curves of what a start `d` below opt(n) costs (times the proof from opt(n)):", "",
            table(["curve", "(d, multiple)"], [[name, ", ".join(f"({d}, {m:.2f})" for d, m in c)]
                                               for name, c in curves]), ""]
    games, rows, _, residuals = model(gap)
    by_n = {g["n"]: g for g in games}
    header = ["start below opt(n)", "searches", "mean residual", "sd", "mean nodes (millions)"]
    table_rows = []
    for low, high, name in BANDS:
        part = [(e, by_n[r["n"]]) for r, e in zip(rows, residuals)
                if low <= by_n[r["n"]]["opt"] - by_n[r["n"]]["start"] <= high]
        if part:
            table_rows.append([name, len(part), f"{mean([e for e, _ in part]):+.2f}",
                               f"{sd([e for e, _ in part]):.2f}" if len(part) > 1 else "-",
                               f"{mean([g['nodes'] for _, g in part]) / 1e6:.2f}"])
    out += [f"The chain's own searches ({len(rows)} exhausted, 2176-{gap.LAST}): their residuals of the cost model by "
            "how far the start in order is below opt(n):", "", table(header, table_rows), ""]
    rows_old =[{"n": g["n"], "deficit": g["opt"] - g["start"], "cpu": g["search_s"] * g["threads"]}
                for g in gap.load_games()]
    rows_new = [{"n": r["n"], "deficit": r["opt"] - r["start"], "cpu": r["chain_seconds"] * r["threads"]}
                for r in read_jsonl(STARTS)]
    header = ["start below opt(n)", "games", "share of the games", "share of the search time"]
    for label, rows in ((f"2176-{gap.LAST}", rows_old), (f"{gap.LAST + 1}-3914", rows_new)):
        if not rows:
            continue
        total = sum(r["cpu"] for r in rows)
        table_rows = []
        for low, high, name in BANDS:
            part = [r for r in rows if low <= r["deficit"] <= high]
            table_rows.append([name, len(part), f"{len(part) / len(rows):.2f}",
                               f"{sum(r['cpu'] for r in part) / total:.2f}"])
        out += [f"The {len(rows)} searched games of {label} ({total / 3600:,.0f} CPU hours), by how far the start in "
                "order is below opt(n):", "", table(header, table_rows), ""]
        for name, c in curves:
            saved = sum(r["cpu"] * (1 - 1 / penalty(c, r["deficit"])) for r in rows)
            out += [f"* by {name}, those starts cost {100 * saved / total:.1f}% of the search time "
                    f"({saved / 3600:,.1f} CPU hours)"]
        out += [""]
    return out


if __name__ == "__main__":
    main(sys.argv[1:])
