#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The seeding study: how much closer a game's start incumbent gets to the optimum when it is seeded from a
neighbour's start incumbent (never proven), as a chain seeds from the proven game of n - 1.

Reads the variants' files in this directory (README.md has the commands that made them, and each line has its game's
`opt` from the committed chain) and ../cal.jsonl, the standalone bounds of the cloud run.
usage: ./seeding.py
"""
import statistics as st
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LANDSCAPE = HERE.parent / "the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10"
sys.path.insert(0, str(LANDSCAPE))
from land import by_n, quantile  # noqa: E402

GAMES = range(2176, 3441)
BAND = 250
# The chain runs, in the order of the tables: file, and the label.
RUNS = {"R1": "r1.jsonl", "A": "a.jsonl", "A64": "a64.jsonl", "A256": "a256.jsonl", "B": "b.jsonl"}
LABELS = {"cal": "standalone (cal, cloud)", "R1": "R1 standalone (laptop)"}
# The bins of the games since the last restart.
DRIFT = [(0, 0), (1, 15), (16, 63), (64, 255), (256, 1023), (1024, 10**9)]


def summary(xs):
    """The shortfalls' median, mean, 90th percentile and share exact (percent)."""
    return st.median(xs), st.mean(xs), quantile(xs, 0.9), 100 * sum(1 for x in xs if x == 0) / len(xs)


def versus(xs, ys):
    """The mean of ys - xs (the shortfall xs gains over ys), and the percent of games where xs is better and worse."""
    gains = [y - x for x, y in zip(xs, ys)]
    return st.mean(gains), 100 * sum(g > 0 for g in gains) / len(gains), 100 * sum(g < 0 for g in gains) / len(gains)


def bands(games):
    """The games of each band of BAND, as (label, games)."""
    out = []
    for lo in range(GAMES.start, GAMES.stop, BAND):
        band = [n for n in games if lo <= n < lo + BAND]
        if band:
            out.append((f"{band[0]}-{band[-1]}", band))
    return out


runs = {name: by_n(HERE / file) for name, file in RUNS.items() if (HERE / file).exists()}
cal = by_n(LANDSCAPE / "cal.jsonl")
short = {name: {n: d["shortfall"] for n, d in run.items()} for name, run in runs.items()}
start_s = {name: {n: d["start_s"] for n, d in run.items()} for name, run in runs.items()}
total_s = {name: {n: d["setup_s"] + d["start_s"] for n, d in run.items()} for name, run in runs.items()}
if (HERE / "c.jsonl").exists():
    # `wall start`: the proven game of n - 1 as the seed; its `pairs` is the promotion sum the local search gives.
    c = {n: d for n, d in by_n(HERE / "c.jsonl").items() if "pairs" in d}
    short["C"] = {n: d["opt"] - d["pairs"] for n, d in c.items()}
    start_s["C"] = {n: d["seed_kept_s"] + d["pairs_s"] for n, d in c.items()}
# The cloud run's standalone start as the calibration measured it (64 threads busy on a c8g.16xlarge).
short["cal"] = {n: cal[n]["shortfall"] for n in GAMES}
start_s["cal"] = {n: cal[n]["start_s"] for n in GAMES}
if "A" in short and "B" in short:
    short["max(A,B)"] = {n: min(short["A"][n], short["B"][n]) for n in short["A"].keys() & short["B"].keys()}

if "R1" in runs:
    differ = [n for n in GAMES if n in runs["R1"] and runs["R1"][n]["score"] != cal[n]["lower"]]
    print(f"R1 (--restart 1) against cal.jsonl's standalone lower bound: {len(differ)} games differ"
          f"{' ' + str(differ[:5]) if differ else ''}")

print(f"\nThe shortfall (optimum less the start's score) on {GAMES.start}-{GAMES.stop - 1}; the time of the start "
      "incumbent in ms (C: seed_kept_s + pairs_s); the games where a variant is better or worse than R1, the "
      "standalone start")
print(f"  {'':<24} {'games':>5} {'median':>7} {'mean':>7} {'p90':>6} {'exact':>7} {'t median':>9} {'t mean':>8} "
      f"{'better':>7} {'worse':>6}")
for name in short:
    games = sorted(short[name])
    xs = [short[name][n] for n in games]
    median, mean, p90, exact = summary(xs)
    line = f"  {LABELS.get(name, name):<24} {len(games):>5} {median:>7.0f} {mean:>7.1f} {p90:>6} {exact:>6.1f}%"
    line += (f" {st.median(start_s[name][n] for n in games) * 1000:>9.1f} "
             f"{st.mean(start_s[name][n] for n in games) * 1000:>8.1f}") if name in start_s else f" {'':>18}"
    if name not in ("R1", "cal") and "R1" in short:
        _, better, worse = versus(xs, [short["R1"][n] for n in games])
        line += f" {better:>6.1f}% {worse:>5.1f}%"
    print(line)

names = [name for name in short if name != "cal"]
for title, index in (("median", 0), ("mean", 1), ("share exact (%)", 3)):
    print(f"\nBy band of {BAND} n: {title} of the shortfall")
    print(f"  {'band':<11} {'games':>5} " + " ".join(f"{name:>9}" for name in names))
    for label, band in bands(GAMES):
        cells = [summary([short[name][n] for n in band])[index] if all(n in short[name] for n in band) else None
                 for name in names]
        shown = ["-" if x is None else format(x, ".1f") for x in cells]
        print(f"  {label:<11} {len(band):>5} " + " ".join(f"{x:>9}" for x in shown))

print("\nDrift: the shortfall against the games since the last restart, with R1's, the same games with no seed; "
      "gain is R1's less the variant's, in mean and in the percent of games where the variant is better and worse; "
      "ratio is the variant's mean over R1's")
print(f"  {'':<5} {'since':>9} {'games':>6} {'median':>7} {'mean':>7} {'exact':>7}   {'R1 median':>9} {'R1 mean':>8}"
      f" {'gain':>7} {'ratio':>6} {'better':>7} {'worse':>6}")
for name in ("A", "A64", "A256", "B"):
    if name not in runs:
        continue
    for lo, hi in DRIFT:
        ns = [n for n, d in runs[name].items() if lo <= d["since_restart"] <= hi]
        if not ns:
            continue
        mine, alone = [short[name][n] for n in ns], [short["R1"][n] for n in ns]
        gain, better, worse = versus(mine, alone)
        label = f"{lo}" if lo == hi else f"{lo}-{hi}" if hi < 10**9 else f"{lo}+"
        print(f"  {name:<5} {label:>9} {len(ns):>6} {st.median(mine):>7.0f} {st.mean(mine):>7.1f} "
              f"{100 * sum(1 for x in mine if x == 0) / len(ns):>6.1f}%   {st.median(alone):>9.0f} "
              f"{st.mean(alone):>8.1f} {gain:>7.1f} {st.mean(mine) / st.mean(alone):>6.2f} "
              f"{better:>6.1f}% {worse:>5.1f}%")

print("\nWhat a restart costs: on the games within 15 of a restart, the mean shortfall of the unbroken chain A (which "
      "has gone on for at least 64 games), of the variant that restarted, and of R1")
for name in ("A64", "A256"):
    if name in runs and "A" in runs:
        ns = [n for n, d in runs[name].items() if 1 <= d["since_restart"] <= 15 and runs["A"][n]["since_restart"] >= 64]
        print(f"  {name:<5} {len(ns):>4} games: A {st.mean(short['A'][n] for n in ns):6.1f}, "
              f"{name} {st.mean(short[name][n] for n in ns):6.1f}, R1 {st.mean(short['R1'][n] for n in ns):6.1f}")

print("\nInherited errors: a seeded game's shortfall against the shortfall of the game it was seeded from (its "
      "neighbour in the chain, not restarted), over the games with a seed; R1 has none, its neighbour is n - 1")
for name in ("R1", "A", "A256", "B"):
    if name not in runs:
        continue
    pairs = [(d["shortfall"], runs[name][n + 1 if name == "B" else n - 1]["shortfall"])
             for n, d in runs[name].items() if d["seed"] == "neighbour" or (name == "R1" and n > GAMES.start)]
    mine, theirs = zip(*pairs)
    print(f"  {name:<5} {len(pairs):>5} games: correlation {st.correlation(mine, theirs):.2f}; "
          f"lower than the neighbour's {100 * sum(a < b for a, b in pairs) / len(pairs):.1f}%, "
          f"equal {100 * sum(a == b for a, b in pairs) / len(pairs):.1f}%, "
          f"higher {100 * sum(a > b for a, b in pairs) / len(pairs):.1f}%; "
          f"mean change {st.mean(a - b for a, b in pairs):+.1f}")

if "R1" in runs:
    print("\nTime: the seconds of the start incumbent, and of setup and start, summed over the games of each run, "
          "against R1's (the same laptop, side by side)")
    for name in runs:
        ns = sorted(runs[name])
        print(f"  {name:<5} start {sum(start_s[name][n] for n in ns):8.1f} s = "
              f"{sum(start_s[name][n] for n in ns) / sum(start_s['R1'][n] for n in ns):.2f} of R1's; "
              f"setup and start {sum(total_s[name][n] for n in ns):8.1f} s = "
              f"{sum(total_s[name][n] for n in ns) / sum(total_s['R1'][n] for n in ns):.2f} of R1's")
    print("\nTime by band of 250 n: the start incumbent's median ms, R1 then A")
    for label, band in bands(GAMES):
        print(f"  {label:<11} " + " ".join(f"{st.median(start_s[name][n] for n in band) * 1000:>8.0f}"
                                           for name in ("R1", "A", "B") if name in runs))
