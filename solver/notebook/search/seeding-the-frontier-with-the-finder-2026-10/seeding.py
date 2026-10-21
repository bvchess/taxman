#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Seeding the frontier's games with the finder's game: the measurement and the estimate.

The entry is solver/notebook/search/seeding-the-frontier-with-the-finder-2026-10/README.md.

For each searched game of the stretch FIRST-LAST of results/strict.jsonl (the lines with `method` "search"):

* the start in order, `wall nodes --nodes 1 --known results/strict.jsonl N` (the game set up as the chain sets it up
  and stopped before its first step), appended to starts.jsonl;
* the finder's game after `--lns-seconds` CPU seconds of one thread: `ledger run N --lns-seconds S` on a ledger of
  that game alone, whose first job, the map, sets the game up standalone, finds the start incumbent and runs the
  finder from it, the ledger's own finder (examples/common/lns.rs); the run ends when the map is done.  Appended to
  finders.jsonl.

    ./seeding.py games > games.jsonl         the stretch's searched games: the committed lines' facts
    ./seeding.py starts [N ...]              each game's start in order (the games named, else all with nodes)
    ./seeding.py finders [N ...]             the finder's run on each game that starts below opt(n)
    ./seeding.py seeded [N ...]              the probe: the same with the game of n - 1 given to the ledger, on the
                                             games the finder leaves SEEDED_FROM or more below opt(n)
    ./seeding.py report > report.md          the tables and the estimates

`starts`, `finders` and `seeded` run at most JOBS processes at once, each under `nice -n 10`, each started when the
1-minute load is LOAD or less, and resume where they stopped (the games the file has are skipped).  They need `wall`
and `ledger` built (`cargo build --profile test --features studies --example wall --example ledger` in solver/frame).
"""
import importlib.util
import json
import math
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]  # the repository: this folder is solver/notebook/search/<entry>
FRAME = REPO / "solver" / "frame"
BUILT = FRAME / "target" / "debug" / "examples"  # the test profile builds into target/debug
WALL = BUILT / "wall"
LEDGER = BUILT / "ledger"
STRICT = REPO / "results" / "strict.jsonl"
DEAR = HERE.parent / "the-dear-games-the-problem-or-the-chains-path-2026-10" / "dear.py"
GAMES = HERE / "games.jsonl"
STARTS = HERE / "starts.jsonl"
FINDERS = HERE / "finders.jsonl"
SEEDED = HERE / "seeded.jsonl"

FIRST, LAST = 3915, 4547  # the stretch the run added to the chain
LNS_SECONDS = 5.0  # the finder's seconds of one thread
BUDGET_HOURS = 0.0001  # the ledger run's CPU budget: the map spends it, so that no search follows
SEEDED_BUDGET_HOURS = 0.012  # that of a seeded run (43.2 s): the map leaves some, the search spends the rest
SEEDED_FROM = 50  # the seeded runs are of the games the finder leaves this far below opt(n) or more
JOBS = 2  # processes at once
LOAD = 8.0  # the 1-minute load above which a process waits for a start
LOAD_WAIT = 30  # seconds between looks at the load
NICE = 10
BANDS = [(0, 0, "0"), (1, 49, "1-49"), (50, 99, "50-99"), (100, 149, "100-149"), (150, 249, "150-249"),
         (250, 399, "250-399"), (400, 10 ** 9, "400 or more")]
DEAREST = 20  # the games of the most nodes
TOP = 10  # the games a table of savings lists


def read_jsonl(path):
    if not Path(path).exists():
        return []
    with open(path) as f:
        return [json.loads(text) for text in f if text.strip()]


def load_dear():
    """The dear-games entry's driver, for the curves of what a start below opt(n) costs."""
    spec = importlib.util.spec_from_file_location("dear", DEAR)
    dear = importlib.util.module_from_spec(spec)
    sys.modules["dear"] = dear
    spec.loader.exec_module(dear)
    return dear


# ---------------------------------------------------------------------------------------------------------
# games

def stretch_games():
    """The searched games of the stretch, as the committed chain has them."""
    out = []
    for r in read_jsonl(STRICT):
        if FIRST <= r["n"] <= LAST and r["method"] == "search" and r.get("completed"):
            out.append({"n": r["n"], "nodes": r["nodes"], "proof": r["proof"], "threads": r["threads"],
                        "search_s": round(r["seconds"]["search_all_runs"], 3), "score": r["score"],
                        "in_the_bag": r["in_the_bag"], "opt": r["score"] - r["in_the_bag"]})
    return out


# ---------------------------------------------------------------------------------------------------------
# the runs

_gate = threading.Lock()


def wait_for_quiet():
    """Returns when the 1-minute load is LOAD or less.  Starts are gated one at a time, so that the processes of
    a batch do not all start in the same quiet moment."""
    with _gate:
        while os.getloadavg()[0] > LOAD:
            time.sleep(LOAD_WAIT)


def niced(command):
    """Runs `command` under nice: (exit status, stdout, stderr, the CPU seconds of the process and its threads)."""
    with tempfile.TemporaryFile("w+") as out, tempfile.TemporaryFile("w+") as err:
        process = subprocess.Popen(command, stdout=out, stderr=err, cwd=FRAME, preexec_fn=lambda: os.nice(NICE))
        _, status, usage = os.wait4(process.pid, 0)
        out.seek(0)
        err.seek(0)
        return os.waitstatus_to_exitcode(status), out.read(), err.read(), usage.ru_utime + usage.ru_stime


def start_of(n):
    """`wall nodes --nodes 1`: the line of the game's start in order."""
    wait_for_quiet()
    command = [str(WALL), "nodes", "--known", str(STRICT), "--nodes", "1", str(n)]
    status, out, err, cpu = niced(command)
    lines = [json.loads(text) for text in out.splitlines() if text.startswith("{")]
    if status != 0 or len(lines) != 1:
        sys.exit(f"{' '.join(command)}: exit {status}, {len(lines)} lines\n{out}\n{err}")
    kept = ["n", "start", "opt", "root_bound", "gap_opt", "seconds", "busy_s", "max_rss"]
    return {**{k: lines[0][k] for k in kept if k in lines[0]}, "cpu_s": round(cpu, 3)}


def finder_of(n, seconds, seeded=False):
    """The ledger's run on the game alone, with the finder: the lines of the map's event (the finder from the
    standalone start incumbent), of the search that follows if there is one, and of the run's end, and the CPU seconds
    of the process.  With `seeded`, the game of n - 1 is given to the ledger (a file of that one line of the chain), so
    that the map is followed by a search of the game seeded with it, which runs the finder first from the seed (the
    survivors of the game of n - 1) for `seconds` more, and then searches for the ledger's base of 2 s."""
    wait_for_quiet()
    work = Path(tempfile.mkdtemp(prefix=f"finder{n}_", dir=os.environ.get("TMPDIR")))
    try:
        # A budget the map (or the map and the search of a seeded run) exhausts, so that no more job follows.
        hours = SEEDED_BUDGET_HOURS if seeded else BUDGET_HOURS
        command = [str(LEDGER), "run", str(n), "--dir", str(work), "--workers", "1", "--lns-seconds", str(seconds),
                   "--climb", "off", "--floor-step", "0", "--cpu-hours", str(hours)]
        if seeded:
            previous = [r for r in read_jsonl(STRICT) if r["n"] == n - 1]
            (work / "previous.jsonl").write_text(json.dumps(previous[0], separators=(",", ":")) + "\n")
            command += ["--proven", str(work / "previous.jsonl")]
        status, out, err, cpu = niced(command)
        events = read_jsonl(work / "events.jsonl")
        init = [e for e in events if e.get("action") == "init" and e.get("n") == n]
        searches = [e for e in events if e.get("action") == "search" and e.get("n") == n]
        if status != 0 or len(init) != 1 or (seeded and not searches):
            sys.exit(f"{' '.join(command)}: exit {status}, {len(init)} init and {len(searches)} search events\n"
                     f"{out}\n{err}")
        end = [e for e in events if e.get("action") == "end"]
        return {"n": n, "lns_seconds": seconds, "init": init[0], "searches": searches,
                "end": end[0] if end else None, "cpu_s": round(cpu, 3)}
    finally:
        shutil.rmtree(work, ignore_errors=True)


def run_all(path, only, games, one):
    """Runs `one(n)` for each game (those of `only` if any) on JOBS threads and appends its line to `path` as it
    ends; the games the file has are skipped."""
    have = {r["n"] for r in read_jsonl(path)}
    todo = [g["n"] for g in games if g["n"] not in have and (not only or g["n"] in only)]
    lock = threading.Lock()

    def work(n):
        record = one(n)
        with lock, open(path, "a") as f:
            f.write(json.dumps(record, separators=(",", ":")) + "\n")
        print(json.dumps(record, separators=(",", ":"))[:200], flush=True)

    with ThreadPoolExecutor(JOBS) as pool:
        list(pool.map(work, todo))


# ---------------------------------------------------------------------------------------------------------
# main

def main(argv):
    command = argv[0] if argv else "report"
    only = {int(a) for a in argv[1:] if a.isdigit()}
    games = [g for g in read_jsonl(GAMES) if g["nodes"] > 0] if GAMES.exists() else []
    if command == "games":
        for g in stretch_games():
            print(json.dumps(g, separators=(",", ":")))
    elif command == "starts":
        run_all(STARTS, only, games, start_of)
    elif command == "finders":
        # Only a game whose start in order is below opt(n) has anything for the finder to add.
        below = {r["n"] for r in read_jsonl(STARTS) if r["start"] < r["opt"]}
        run_all(FINDERS, only, [g for g in games if g["n"] in below], lambda n: finder_of(n, LNS_SECONDS))
    elif command == "seeded":
        rows, _ = joined()
        far = {r["n"] for r in rows if r["d_finder"] >= SEEDED_FROM}
        run_all(SEEDED, only, [g for g in games if g["n"] in far], lambda n: finder_of(n, LNS_SECONDS, seeded=True))
    elif command == "report":
        print(report())
    else:
        sys.exit(__doc__)


# ---------------------------------------------------------------------------------------------------------
# report

def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    out.extend("| " + " | ".join(str(c) for c in row) + " |" for row in rows)
    return "\n".join(out)


def multiple(dear, curve, d, extrapolate=False):
    """What a start `d` below opt(n) costs, as a multiple of the proof from opt(n): the curve's log-linear
    interpolation, and beyond its last point either that point's multiple (`extrapolate` false) or the line of its
    last two points continued."""
    last_d, last_m = curve[-1]
    if d <= last_d or not extrapolate:
        return dear.penalty(curve, d)
    (d0, m0), (d1, m1) = curve[-2], curve[-1]
    return 10 ** (math.log10(m1) + (math.log10(m1) - math.log10(m0)) * (d - d1) / (d1 - d0))


def joined():
    """The stretch's searched games with nodes, each with its start in order and the finder's game: d_start, the
    finder's score as a promotion sum, and d_finder (opt(n) less the better of the two; a game that starts at opt(n)
    has no finder run).  Also the games whose finder has not run."""
    starts = {r["n"]: r for r in read_jsonl(STARTS)}
    finders = {r["n"]: r for r in read_jsonl(FINDERS)}
    rows, missing = [], []
    for g in read_jsonl(GAMES):
        s, f = starts.get(g["n"]), finders.get(g["n"])
        if g["nodes"] == 0 or s is None:
            continue
        assert s["opt"] == g["opt"], f"{g['n']}: the opt of wall ({s['opt']}) and of the chain ({g['opt']}) differ"
        row = {**g, "start": s["start"], "d_start": g["opt"] - s["start"], "gap": s["gap_opt"],
               "start_cpu_s": s["cpu_s"], "finder_from": None, "finder_found": None, "d_finder": 0,
               "finder_cpu_s": 0.0, "finder_s": None}
        if row["d_start"] > 0:
            if f is None:
                missing.append(g["n"])
                continue
            init = f["init"]
            found = init["lns_found"] - g["in_the_bag"]
            row.update(finder_from=init["lns_from"] - g["in_the_bag"], finder_found=found,
                       d_finder=g["opt"] - max(s["start"], found), finder_cpu_s=f["cpu_s"], finder_s=init["lns_s"])
        rows.append(row)
    return rows, missing


def saved(dear, rows, curve, key, extrapolate=False, weight="nodes"):
    """The nodes the seeding saves: each game's nodes less the nodes at the seeded start, which are the nodes times
    the multiple at the seeded start over the multiple at the start in order (`key`: "d_finder", or "zero" for a start
    at opt(n)).  With `weight` "cpu", the CPU seconds of the chain's search in place of its nodes."""
    total = 0.0
    for r in rows:
        d_new = 0 if key == "zero" else r[key]
        cost = r["nodes"] if weight == "nodes" else r["search_s"] * r["threads"]
        total += cost * (1 - multiple(dear, curve, d_new, extrapolate) / multiple(dear, curve, r["d_start"],
                                                                                   extrapolate))
    return total


def saved_as_extra_steps(dear, rows, curve, key, base):
    """The nodes the seeding saves if a start's cost were the extra steps game 3020 takes (its `base` steps from
    opt(n) times the multiple less 1) and not a multiple of the game's own tree: each game's saving is the extra
    steps at its start in order less those at the seeded start, at most its nodes."""
    total = 0.0
    for r in rows:
        d_new = 0 if key == "zero" else r[key]
        extra = base * (multiple(dear, curve, r["d_start"]) - multiple(dear, curve, d_new))
        total += min(extra, r["nodes"])
    return total



def own_residuals(rows, dear, curve):
    """The chain's own tree sizes against what a start below opt(n) should cost: a cost model fitted to the games
    that start at opt(n), log10 nodes = a + b G / n^0.3 for G the root gap over opt(n) (the form of the gap entry's
    model, refitted here: its fit is of 2176-3312), and the mean residual of the games that start below it, by band,
    next to the mean log10 of the multiple that `curve` gives for their starts.  Games of fewer than 100,000 nodes are
    left out."""
    big = [r for r in rows if r["nodes"] >= 100_000]
    x = [r["gap"] / r["n"] ** 0.3 for r in big]
    y = [math.log10(r["nodes"]) for r in big]
    base = [i for i, r in enumerate(big) if r["d_start"] == 0]
    mx, my = (statistics.mean(v[i] for i in base) for v in (x, y))
    slope = sum((x[i] - mx) * (y[i] - my) for i in base) / sum((x[i] - mx) ** 2 for i in base)
    intercept = my - slope * mx
    residual = [y[i] - (intercept + slope * x[i]) for i in range(len(big))]
    body = []
    for low, high, name in BANDS:
        part = [i for i, r in enumerate(big) if low <= r["d_start"] <= high]
        if len(part) < 2:
            continue
        e = [residual[i] for i in part]
        body.append([name, len(part), f"{statistics.mean(e):+.2f}", f"{statistics.stdev(e) / math.sqrt(len(e)):.2f}",
                     f"{statistics.mean(math.log10(multiple(dear, curve, big[i]['d_start'])) for i in part):+.2f}"])
    return (f"The model fitted to the {len(base)} games that start at opt(n): log10 nodes = {intercept:.3f} + "
            f"{slope:.5f} G / n^0.3, with residuals of sd {statistics.pstdev([residual[i] for i in base]):.2f}.  The "
            f"others' residuals, by how far the start is below opt(n):\n\n" +
            table(["start below opt(n)", "games", "mean residual", "standard error",
                   "mean log10 of the sweep's multiple"], body))


def report():
    dear = load_dear()
    runs = {(r["n"], r["run"]): r for r in dear.read_jsonl(dear.RUNS)}
    curves = [("game 3020's sweep", dear.penalty_curve(runs)),
              ("the real starts", dear.chain_start_curve([dear.Entry(g, runs) for g in dear.read_jsonl(dear.GAMES)]))]
    rows, missing = joined()
    stretch = read_jsonl(GAMES)
    total = sum(g["nodes"] for g in stretch)
    below = [r for r in rows if r["d_start"] > 0]
    out = [f"The searched games of {FIRST}-{LAST}: {len(stretch)} lines of results/strict.jsonl with `method` search, "
           f"{len(rows) + len(missing)} of them with nodes, {total:,} nodes in all.  {len(below)} of the "
           f"{len(rows)} games reported start below opt(n)."]
    if missing:
        out += ["", f"The finder has not run on {len(missing)} games that start below opt(n): {missing}."]
    out += ["", "The curves of what a start d below opt(n) costs (times the proof from opt(n)):", "",
            table(["curve", "(d, multiple)"],
                  [[name, ", ".join(f"({d}, {m:.2f})" for d, m in c)] for name, c in curves]), ""]

    out += ["### The games", "", "Scores are promotion sums; `d` is how far below opt(n).  The finder's game is the "
            "ledger's, from the standalone start incumbent (`finder from`); the better of it and the start in order "
            "is what the seeded search would begin with.", ""]
    header = ["n", "nodes", "d of the start", "d of the finder's game", "d of the better", "d of the finder's start",
              "finder s", "finder CPU s"]
    body = []
    for r in sorted(rows, key=lambda r: -r["nodes"]):
        f = r["finder_found"]
        body.append([r["n"], f"{r['nodes']:,}", r["d_start"], "-" if f is None else r["opt"] - f, r["d_finder"],
                     "-" if r["finder_from"] is None else r["opt"] - r["finder_from"],
                     "-" if r["finder_s"] is None else f"{r['finder_s']:.1f}",
                     "-" if r["finder_s"] is None else f"{r['finder_cpu_s']:.1f}"])
    out += [table(header, body), ""]

    out += ["### By how far the start in order is below opt(n)", ""]
    header = ["start below opt(n)", "games", "nodes (millions)", "share of the nodes", "finder lifts to opt(n)",
              "mean d of the better", "perfect start saves", "finder saves"]
    body = []
    sweep = curves[0][1]
    for low, high, name in BANDS:
        part = [r for r in rows if low <= r["d_start"] <= high]
        if not part:
            continue
        nodes = sum(r["nodes"] for r in part)
        body.append([name, len(part), f"{nodes / 1e6:,.0f}", f"{nodes / total:.3f}",
                     "-" if name == "0" else sum(1 for r in part if r["d_finder"] == 0),
                     "-" if name == "0" else f"{sum(r['d_finder'] for r in part) / len(part):.0f}",
                     f"{saved(dear, part, sweep, 'zero') / total:.3f}",
                     f"{saved(dear, part, sweep, 'd_finder') / total:.3f}"])
    out += [table(header, body), "",
            f"The last two columns are shares of the stretch's {total:,} nodes, valued by {curves[0][0]}.", ""]

    out += ["### The estimate", ""]
    header = ["valued by", "perfect start saves", "finder's game saves", "finder's share of the perfect",
              "perfect (millions of nodes)", "finder (millions)"]
    body = []
    for name, curve in curves:
        for extrapolate in (False, True):
            if extrapolate and not any(r["d_start"] > curve[-1][0] for r in rows):
                continue
            perfect, finder = (saved(dear, rows, curve, k, extrapolate) for k in ("zero", "d_finder"))
            body.append([name + (", the last line continued beyond its last point" if extrapolate else ""),
                         f"{perfect / total:.1%}", f"{finder / total:.1%}", f"{finder / perfect:.2f}",
                         f"{perfect / 1e6:,.0f}", f"{finder / 1e6:,.0f}"])
    out += [table(header, body), ""]

    base = runs[(3020, "a")]["nodes"]
    perfect, finder = (saved_as_extra_steps(dear, rows, sweep, k, base) for k in ("zero", "d_finder"))
    out += [f"If a start's cost were the extra steps game 3020 takes ({base:,} steps from opt(n) times the multiple "
            f"less 1) and not a multiple of the game's own tree, a perfect start would save {perfect / total:.2%} of "
            f"the nodes ({perfect / 1e6:,.0f} million) and the finder's game {finder / total:.2%} "
            f"({finder / 1e6:,.0f} million).", ""]

    cpu_total = sum(g["search_s"] * g["threads"] for g in stretch)
    for name, curve in curves[:1]:
        out += [f"Weighted by the CPU time the chain took instead of its nodes ({cpu_total / 3600:,.0f} CPU hours), "
                f"valued by {name}: a perfect start saves "
                f"{saved(dear, rows, curve, 'zero', weight='cpu') / cpu_total:.1%}, the finder's game "
                f"{saved(dear, rows, curve, 'd_finder', weight='cpu') / cpu_total:.1%}.", ""]

    lifted = [r for r in below if r["d_finder"] == 0]
    better = [r for r in below if r["d_finder"] < r["d_start"]]
    dearest = sorted(rows, key=lambda r: -r["nodes"])[:DEAREST]
    dearest_below = [r for r in dearest if r["d_start"] > 0]
    dearest_lifted = [r for r in dearest_below if r["d_finder"] == 0]
    out += [f"Of the {len(below)} games that start below opt(n) ({sum(r['nodes'] for r in below) / total:.1%} of the "
            f"nodes), the finder's game beats the start in {len(better)} and reaches opt(n) in {len(lifted)} "
            f"({sum(r['nodes'] for r in lifted) / sum(r['nodes'] for r in below):.0%} of their nodes).  Of the dearest "
            f"{DEAREST} games ({sum(r['nodes'] for r in dearest) / total:.1%} of the nodes), "
            f"{len(dearest_below)} start below opt(n) and the finder lifts {len(dearest_lifted)} of them.", ""]
    out += ["The dearest games that start below opt(n):", "",
            table(["n", "nodes", "d of the start", "d of the better"],
                  [[r["n"], f"{r['nodes']:,}", r["d_start"], r["d_finder"]]
                   for r in sorted(below, key=lambda r: -r["nodes"])[:DEAREST]]), ""]

    out += [f"The {TOP} games where a perfect start would save most, valued by {curves[0][0]}, and what the finder's "
            "game saves in them (millions of nodes):", ""]
    body = []
    for r in sorted(below, key=lambda r: -saved(dear, [r], sweep, "zero"))[:TOP]:
        perfect, finder = (saved(dear, [r], sweep, key) / 1e6 for key in ("zero", "d_finder"))
        body.append([r["n"], f"{r['nodes']:,}", r["d_start"], r["d_finder"], f"{perfect:,.0f}", f"{finder:,.0f}"])
    out += [table(["n", "nodes", "d of the start", "d of the better", "perfect start saves", "finder saves"], body), ""]

    finder_s = sum(r["finder_s"] for r in below)
    finder_cpu = sum(r["finder_cpu_s"] for r in below)
    out += [f"The finder ran {finder_s:.0f} s of its own on the {len(below)} games (5 s each), {finder_cpu / 3600:.2f} "
            f"CPU hours with the ledger's set-up, against the {cpu_total / 3600:,.0f} CPU hours of the stretch's "
            f"search; the starts in order took {sum(r['start_cpu_s'] for r in rows) / 3600:.2f} CPU hours of "
            f"`wall nodes` on {len(rows)} games.", ""]

    share = statistics.mean(r["cpu_s"] / r["end"]["t"] for r in read_jsonl(FINDERS))
    out += [f"The ledger's runs had {share:.0%} of a core on average (CPU seconds of the process over its wall "
            f"seconds, set-up included), under `nice` on the shared laptop; the finder's 5 s are wall seconds.", ""]

    out += ["### The chain's own residuals", "", own_residuals(rows, dear, sweep), ""]

    out += ["### The finder from the seed of the game before (a probe)", "",
            f"The games the finder leaves {SEEDED_FROM} or more below opt(n), run again with the game of n - 1 given "
            "to the ledger, whose search of the game then runs the finder first from the seed (the promotions of the "
            "game of n - 1 that survive) if that seed is not the first finder's game itself:", ""]
    body = []
    by_n = {r["n"]: r for r in rows}
    for r in read_jsonl(SEEDED):
        game = by_n[r["n"]]
        ran = [x for x in r["searches"] if x.get("lns_from") is not None]
        if ran:
            x = ran[0]
            bag = game["in_the_bag"]
            cells = [game["opt"] - (x["lns_from"] - bag), game["opt"] - (x["lns_found"] - bag)]
        else:
            cells = ["-", "-"]
        body.append([r["n"], f"{game['nodes']:,}", game["d_start"], game["d_finder"], *cells])
    out += [table(["n", "nodes", "d of the start", "d of the better of it and the first finder's game",
                   "d of the seed", "d of the finder's game from it"],
                  sorted(body, key=lambda row: -int(row[1].replace(",", ""))))]
    return "\n".join(out)


if __name__ == "__main__":
    main(sys.argv[1:])
