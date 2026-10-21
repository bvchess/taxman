#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Part 1 of the entry: why game 4875 was dear, from the data of the run past 5000.

usage: analysis.py DATA_DIR      (DATA_DIR holds past5000.jsonl, chain.err.3, results-B.jsonl, units.jsonl,
                                   units-b.jsonl and landscape.jsonl; this folder's results-B.jsonl and landscape.jsonl
                                   are copies)

It prints, in this order: the frames of the games before 4875, the 100 solves of experiment B and the fit of log10 steps
against gap, the work left in the open units of the two checkpoints, the work between them, the root gaps of the
landscape against 4875's, and every search of the run over 100 million nodes with the incumbent it started at.
"""
import json
import math
import re
import statistics
import sys

D = sys.argv[1].rstrip("/") + "/"
INCS = (537927, 537971, 538000, 538030, 538080)
ROOT_BOUND = 540110  # the root GF(2) bound of 4875 as a promotion sum (score 7478404 less in_the_bag 6938294)


def units(name):
    with open(D + name) as f:
        game = json.loads(f.readline())
        out = [json.loads(line) for line in f]
    return game, [u for u in out if u["kind"] == "unit"]


def clock(text):
    return sum(float(x) * {"h": 3600, "m": 60, "s": 1}[u] for x, u in re.findall(r"(\d+(?:\.\d+)?)([hms])", text))


def progress(path):
    """Per game: the incumbent of its first progress line, its last, the nodes of its last, and each rise."""
    games = {}
    line_re = re.compile(r"^n=(\d+) (\S+) nodes (\d+) \(.*?\) best (\d+)")
    with open(path) as f:
        for line in f:
            m = line_re.match(line)
            if not m:
                continue
            n, t, nodes, best = int(m.group(1)), clock(m.group(2)), int(m.group(3)), int(m.group(4))
            g = games.setdefault(n, {"first": best, "rises": [], "lines": 0})
            g["lines"] += 1
            g["last"], g["nodes"] = best, nodes
            if not g["rises"] or g["rises"][-1][1] != best:
                g["rises"].append((t, best, nodes))
    return games


print("== the frames: frames, candidates, max_total, greedy sum, search nodes")
setups = {}
with open(D + "past5000.jsonl") as f:
    for line in f:
        r = json.loads(line)
        if r["n"] in (4863, 4864, 4865, 4871, 4872) and r.get("setup"):
            setups[r["n"]] = r
for n, r in sorted(setups.items()):
    s = r["setup"]
    print(n, r["method"], s["frames"], s["candidates"], s["max_total"], s["greedy_sum"], r["nodes"])

print("== experiment B: 20 subtrees solved from 5 incumbents")
B = [json.loads(line) for line in open(D + "results-B.jsonl")]
for inc in INCS:
    v = [r[f"ratio_{inc}"] for r in B if r[f"ratio_{inc}"] is not None]
    g = math.exp(sum(map(math.log, v)) / len(v))
    slope = math.log10(g) / ((inc - 537971) / 100) if inc != 537971 else 0.0
    print(f"  {inc}: {len(v)} subtrees, geometric mean of steps relative to 537971 {g:.3f}, {slope:+.3f} decades per 100")
xs, ys = [], []
for r in B:
    for inc in INCS:
        if r[f"steps_{inc}"] and r[f"completed_{inc}"]:
            xs.append((r["gap"] - (inc - 537971)) / 100)
            ys.append(math.log10(r[f"steps_{inc}"]))
mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
slope = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
base = my - slope * mx
sd = statistics.pstdev([y - (base + slope * x) for x, y in zip(xs, ys)])
sxx = sum((x - mx) ** 2 for x in xs)
slope_se = sd / math.sqrt(sxx)
print(f"  fit over {len(xs)} solves: log10 steps = {base:.3f} + {slope:.3f} * gap/100, sd {sd:.2f}, slope error "
      f"{slope_se:.3f}; gaps of the solves {min(r['gap'] for r in B)} to {max(r['gap'] for r in B)} at 537971")

print("== the work left in the open units of the two checkpoints (gap > 0), by the fit")
ckpt = {}
for name in ("units.jsonl", "units-b.jsonl"):
    game, us = units(name)
    live = [u for u in us if u["gap"] > 0]
    ckpt[name] = (game, live)
    pred = [10 ** (base + slope * u["gap"] / 100) for u in live]
    total = sum(pred)
    big = [p for u, p in zip(live, pred) if u["gap"] >= 1500]
    gaps = [u["gap"] for u in live if u["gap"] >= 1500]
    print(f"  {name}: {game['checkpoint_nodes']} nodes, incumbent {game['incumbent_sum']}, {len(us)} units, {len(live)} "
          f"with a gap above 0; {total:.4g} steps left; the {len(big)} units of gap {min(gaps)} to {max(gaps)} hold "
          f"{sum(big) / total:.1%}")
    print("   units with gap above 1,000, by gap/100: ",
          {k: sum(1 for u in live if u["gap"] // 100 == k) for k in range(10, 20)})
for name, (game, live) in ckpt.items():
    # the same units priced with the slope two errors lower and higher, through the mean of the solves
    low_high = []
    for k in (-2, 2):
        steeper = slope + k * slope_se
        low_high.append(sum(10 ** (my + steeper * (u["gap"] / 100 - mx)) for u in live))
    print(f"  {name}: with the slope two errors lower and higher, {low_high[0]:.4g} and {low_high[1]:.4g} steps left")
(ga, la), (gb, lb) = ckpt["units.jsonl"], ckpt["units-b.jsonl"]
shift = gb["incumbent_sum"] - ga["incumbent_sum"]
left_a = sum(10 ** (base + slope * (u["gap"] - shift) / 100) for u in la if u["gap"] - shift > 0)
left_b = sum(10 ** (base + slope * u["gap"] / 100) for u in lb)
done = gb["checkpoint_nodes"] - ga["checkpoint_nodes"]
print(f"== between the checkpoints: {done} steps taken; the fit's work left at the first checkpoint, at the second one's "
      f"incumbent, {left_a:.4g}; at the second {left_b:.4g}; the fit's work done {left_a - left_b:.4g}, "
      f"steps taken over the fit's {done / (left_a - left_b):.2f}")

print("== the root gap: root bound less the incumbent (a promotion sum)")
rows = sorted((json.loads(line) for line in open(D + "landscape.jsonl")), key=lambda r: r["n"])
results = {}
with open(D + "past5000.jsonl") as f:
    for line in f:
        r = json.loads(line)
        if r.get("completed"):
            results[r["n"]] = r["nodes"]
for r in rows:
    print(f"  {r['n']}: root gap {r['root_gap']} (best {r['opt_proof']}), nodes of the run {results.get(r['n'], '-')}")
gaps = sorted(r["root_gap"] for r in rows)
print(f"  {len(gaps)} games: {gaps[0]} to {gaps[-1]}, median {gaps[len(gaps) // 2]}")
for inc in (537927, 537971, 538080):
    print(f"  4875 against {inc}: {ROOT_BOUND - inc}")

print("== the searches of the run over 100 million nodes: incumbent of the first progress line, final, nodes")
games = progress(D + "chain.err.3")
for n, g in sorted(games.items()):
    if g["nodes"] >= 100_000_000:
        mark = f"  starts {g['last'] - g['first']} below its best" if g["last"] != g["first"] else ""
        print(f"  {n}: {g['first']} {g['last']} {g['nodes']}{mark}")
print("  4875 rises (seconds, incumbent, steps):", games[4875]["rises"])
print("  4864 rises (seconds, incumbent, steps):", games[4864]["rises"])
