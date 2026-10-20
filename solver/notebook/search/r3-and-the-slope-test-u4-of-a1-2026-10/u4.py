#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of U4, the next step of A1 after the E3 pilot (the advisor's fourth consultation): R3 against the
reference on saved states (Phase 1), the slope test (Phase 2), patch mode with R3 (Phase 3) and R3's cost at the
end of the chain (Phase 4).

Reads the output of `examples/fields.rs` in u4/ beside this script (`--dir`): trees.jsonl (`bnb` trees, a line per
run), replay.jsonl (`replay`, a line per saved state and method), fits.jsonl (`fit`, a line per root fit),
dives.jsonl (`bnb --knuth`, the dives at the end of the chain) and the per-node logs of r3's trees kept in logs/
(gzipped), and the plain branch and bound's trees and roots of the matroid study (../matroid/bnb.jsonl and
roots.jsonl), whose trees are v0's.  A line that repeats the key of an earlier one (a rerun) replaces it; a missing
file is noted and its sections skipped.  Prints Markdown: 1. each method's reach (the share of the reference's cuts it
makes) by GF(2) excess band and its cost, and Phase 1's stop rule; 2. the trees, the regression of each variant's log
ratio to v0 on the root gap, its effective share and Phase 2's criteria, and where R3 cuts in the trees; 3. the patch
arms' fits and trees, and Phase 3's criteria; 4. R3 at the nodes of games at the end of the chain and the projection
at a gap of 1,400.  The criteria are the design's, fixed before the runs.  Pure Python; `./u4.py > u4_output.md`.
"""
import argparse
import gzip
import json
import math
import statistics as st
import sys
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # the script's own directory is not on the path under `python3 -I`
from fields import fmt, load, mean, median, pct, table  # noqa: E402

# Phase 1 (design U4), fixed in advance.
PHASE1_HOSTS = ["n992-r2-all-k20", "n1215-r2-all-k20"]
REFERENCE = "ref:10"
STOP_METHOD = "r3:1"  # R3 at 1 s ...
STOP_EXCESS = 100  # ... at the nodes of GF(2) excess above this ...
STOP_REACH = 0.6  # ... reaching less than this share of the reference's cuts: R3 is not reference strength
EXCESS_BANDS = [(0, 50), (51, 100), (101, 200), (201, 300), (301, None)]

# Phase 2, fixed in advance.
KILL_T, KILL_SHARE = 2.0, 0.3  # every R3 policy with t < 2 or s_eff < 0.3: A1 is killed in the search
PASS_T, PASS_SHARE = 2.0, 0.45  # some policy with s_eff >= 0.45 and t >= 2: a pass
FLOOR_CHECK = 2000  # the context's regression: games whose v0 tree has at least this many nodes
PHASE2_R3 = "t1-p300-coresfirst"  # R3 as Phase 1 picked it (1 s, the reference's order): the criterion's variants

# Phase 3, fixed in advance.
ANCHORS = {1197, 1274, 990}
HARVEST_TREE, HARVEST_TIME, CHAIN_TREE = 1.5, 0.5, 1.3

# Phase 4: the solver's cost of a node-thread at 3424 (ms), the decades its tree gains per unit of s_eff at a gap of
# 1,400 (1,400 / 255), the cost of Tier 1 and a cheap refit in solver nodes, and the projection's line for E4.
SOLVER_NODE_MS = 0.35
DECADES = 1400 / 255
BASE_COST = 20
PROJECTION_LINE = 3.0


# ---------------------------------------------------------------------------------------------------------
# Reading and arithmetic

def tree_key(r):
    return (r.get("n"), r.get("variant"), r.get("policy"), r.get("k"), r.get("mode"), r.get("r3"), r.get("label"))


def replay_key(r):
    return (r.get("n"), r.get("host"), r.get("index"), r.get("method"))


def band_of(x):
    for lo, hi in EXCESS_BANDS:
        if x >= lo and (hi is None or x <= hi):
            return f"{lo}-{hi}" if hi is not None else f"over {lo - 1}"
    return "below 0"


def band_labels():
    return [f"{lo}-{hi}" if hi is not None else f"over {lo - 1}" for lo, hi in EXCESS_BANDS]


def ols(xs, ys):
    """Least squares of ys on xs: (slope, its standard error, t, intercept); None for fewer than three points."""
    if len(xs) < 3:
        return None
    mx, my = st.fmean(xs), st.fmean(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    if sxx == 0:
        return None
    b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx
    sse = sum((y - my - b * (x - mx)) ** 2 for x, y in zip(xs, ys))
    se = math.sqrt(sse / (len(xs) - 2) / sxx)
    t = b / se if se > 0 else math.copysign(math.inf, b) if b else 0.0
    return b, se, t, my - b * mx


def num(x):
    return "-" if x is None else f"{x:,}"


def ms(x):
    return "-" if x is None else f"{x:,.1f}"


def run_name(r):
    """A tree's variant as the tables name it: v0, r2, r3 with its policy (and R3's settings unless the default)."""
    if r["variant"] != "r3":
        return r["variant"]
    name = f"r3 {r['policy']}"
    return name if r.get("r3") == "t1-p300" else f"{name} ({r.get('r3')})"


def plain_trees(matroid):
    """v0's trees by n (the matroid study's plain branch and bound, which `fields bnb v0` prints again) and the roots'
    GF(2) gaps."""
    roots = load(matroid / "roots.jsonl", lambda r: r["n"]) or []
    gap = {r["n"]: r["gf2"] - r["opt"] for r in roots}
    trees = load(matroid / "bnb.jsonl", lambda r: (r["n"], r["bound"])) or []
    return {r["n"]: r for r in trees if r["bound"] == "Gf2"}, gap


# ---------------------------------------------------------------------------------------------------------
# Phase 1: R3 against the reference on saved states

def reach(states, method):
    """(made, of): the reference's cuts among `states` (dicts method -> line) that `method` also makes."""
    ref = [s for s in states if s.get(REFERENCE, {}).get("cut") and method in s]
    return sum(1 for s in ref if s[method]["cut"]), len(ref)


def reach_cell(made, of):
    return "-" if not of else f"{made / of:.2f} ({made}/{of})"


def section_phase1(replays):
    print("## 1. R3 against the reference on saved states (Phase 1)\n")
    lines = [r for r in replays if r["host"] in PHASE1_HOSTS]
    if not lines:
        print("No replays of Phase 1's hosts: skipped.\n")
        return None
    by_state = defaultdict(dict)
    for r in lines:
        by_state[(r["n"], r["host"], r["index"])][r["method"]] = r
    methods = sorted({r["method"] for r in lines}, key=method_order)
    games = sorted({r["n"] for r in lines})
    print("Each state is a Tier-1-open node of the host tree (r2, the fixed master), every 10th at 992 and every 5th "
          "at 1215; each method runs from the state the node inherited.  *Reach*: the share of the reference's cuts "
          "that the method also makes.  Times in ms of one thread.\n")
    for n in games + [None]:
        states = [s for k, s in by_state.items() if n is None or k[0] == n]
        title = f"n = {n}" if n else "both games"
        print(f"### {title}: {len(states)} states\n")
        above = [s for s in states if first(s)["excess_gf2"] > STOP_EXCESS]
        rows = []
        for m in methods:
            got = [s[m] for s in states if m in s]
            cut = sum(1 for r in got if r["cut"])
            stops = defaultdict(int)
            for r in got:
                stops[r["stop"]] += 1
            rows.append([m, len(got), pct(cut / len(got)) if got else "-", reach_cell(*reach(states, m)),
                         reach_cell(*reach(above, m)), ms(mean(r["ms"] for r in got)),
                         ms(median([r["ms"] for r in got])), fmt(mean(r["pricings"] for r in got), 1),
                         fmt(mean(r["rounds"] for r in got), 2), pct(stops["Seconds"] / len(got)) if got else "-"])
        table(["method", "states", "cut", "reach", f"reach, excess > {STOP_EXCESS}", "ms mean", "ms median",
               "pricings", "rounds", "out of time"], rows)
        rows = []
        for m in methods:
            if m == REFERENCE:
                continue
            cells = [m]
            for label in band_labels():
                cells.append(reach_cell(*reach([s for s in states if band_of(first(s)["excess_gf2"]) == label], m)))
            rows.append(cells)
        ref_cells = ["reference cuts"]
        for label in band_labels():
            band = [s for s in states if band_of(first(s)["excess_gf2"]) == label]
            ref_cells.append(f"{sum(1 for s in band if s.get(REFERENCE, {}).get('cut'))}/{len(band)}")
        rows.append(ref_cells)
        print("Reach by the node's GF(2) excess:\n")
        table(["method"] + band_labels(), rows)
        rows = []
        for m in methods:
            got = [s[m] for s in states if m in s]
            parts = ["ms_pricing", "ms_simplex", "ms_exact", "ms_separate", "ms_build"]
            rows.append([m] + [ms(mean(r[p] for r in got)) for p in ["ms"] + parts]
                        + [fmt(mean(r["cuts_exact"] for r in got), 1), fmt(mean(r["new_signings"] for r in got), 2)])
        print("Where the time goes (means a state; the rest of `ms` is the cheap separation and bookkeeping):\n")
        table(["method", "ms", "pricing", "simplex", "exact checks", "separation", "master builds", "exact cuts",
               "new signings"], rows)
    hosts_same = [r for r in lines if r.get("host_same") is not None]
    same = sum(1 for r in hosts_same if r["host_same"])
    print(f"Replays with the host's own method that gave the host's refit exactly: {same} of {len(hosts_same)}.\n")
    states = list(by_state.values())
    above = [s for s in states if first(s)["excess_gf2"] > STOP_EXCESS]
    made, of = reach(above, STOP_METHOD)
    value = made / of if of else None
    stop = value is not None and value < STOP_REACH
    print(f"**Stop rule** (fixed): R3 at 1 s ({STOP_METHOD}) reaches {reach_cell(made, of)} of the reference's cuts at "
          f"the states of GF(2) excess above {STOP_EXCESS} (both games); the rule stops below {STOP_REACH}: "
          f"**{'stop' if stop else 'go on'}**.\n")
    return stop


def first(state):
    return next(iter(state.values()))


def method_order(m):
    kind, _, t = m.partition(":")
    order = ["r1", "r2", "r2c", "r3", "r3x", "r3c", "ref"]
    return (order.index(kind) if kind in order else len(order), float(t) if t else 0.0)


# ---------------------------------------------------------------------------------------------------------
# Phase 2: the slope test

def section_phase2(trees, plain, gap):
    print("## 2. The slope test (Phase 2)\n")
    runs = [r for r in trees if r.get("mode") == "tree" and not r.get("label")]
    if not runs:
        print("No trees: skipped.\n")
        return None
    games = sorted({r["n"] for r in runs if r["variant"] != "v0"})
    names = sorted({run_name(r) for r in runs if r["variant"] != "v0"})
    by = {(r["n"], run_name(r)): r for r in runs}
    print("v0's trees are the matroid study's "
          "(`solver/notebook/search/matroid-intersection-bounds-2026-10/bnb.jsonl`); "
          "the v0 reruns below check them.  *ratio*: v0's nodes over the variant's; *f₃*: the share of the "
          "variant's nodes that got R3; *R3 ms*: its time a refit; *time*: the run's over v0's.\n")
    reruns = [r for r in runs if r["variant"] == "v0"]
    if reruns:
        rows = [[r["n"], num(r["nodes"]), num(plain[r["n"]]["nodes"]), "yes" if r["nodes"] == plain[r["n"]]["nodes"]
                 else "**no**"] for r in sorted(reruns, key=lambda r: r["n"]) if r["n"] in plain]
        table(["n", "v0 rerun", "bnb.jsonl", "equal"], rows)
    rows = []
    for n in sorted(games, key=lambda n: (gap.get(n, 0), n)):
        v0 = plain.get(n)
        cells = [n, gap.get(n), num(v0["nodes"]) if v0 else "-"]
        for name in names:
            r = by.get((n, name))
            if r is None:
                cells += ["-", "-", "-"]
                continue
            done = "" if r["completed"] else " (stopped)"
            ratio = v0["nodes"] / r["nodes"] if v0 and r["nodes"] else None
            time = (r["ms"] / 1e3) / v0["seconds"] if v0 and v0["seconds"] else None
            f3 = r.get("r3_refits", 0) / r["nodes"] if r["nodes"] else None
            r3ms = r["ms_r3"] / r["r3_refits"] if r.get("r3_refits") else None
            extra = f", f₃ {fmt(f3, 3)}, R3 {ms(r3ms)} ms" if r["variant"] == "r3" else ""
            cells += [num(r["nodes"]) + done, fmt(ratio, 2), f"{fmt(time, 1)}{extra}"]
        rows.append(cells)
    header = ["n", "gap", "v0"]
    for name in names:
        header += [name, "ratio", "time"]
    table(header, rows)
    print("The regression of `y = log10(N_v0 / N_variant)` on the root gap `G` over the completed games: slope `β` (in "
          "10⁻³ decades per unit of gap) with its `t`; `b₀` the slope of `log10 N_v0` on `G` over the same games; "
          "*own slope* the variant's, of `log10 N_variant` on `G` (`b₀ − β`); `s_eff = β / b₀`, the share of the "
          "tree's growth with the gap that the variant takes away; *mean y* the mean log ratio (a constant factor of "
          "`10^y`).\n")
    rows, results = regression(names, games, by, plain, gap, 0)
    table(["variant", "games", "β", "t", "b₀", "own slope", "s_eff", "mean y", "mean f₃", "R3 ms a refit"], rows)
    print(f"Context, not the criterion: the same over the games whose v0 tree has at least {FLOOR_CHECK:,} nodes (the "
          "design's reason for a gap of at least 120: A1's trees not at their floor).\n")
    rows, _ = regression(names, games, by, plain, gap, FLOOR_CHECK)
    table(["variant", "games", "β", "t", "b₀", "own slope", "s_eff", "mean y", "mean f₃", "R3 ms a refit"], rows)
    r3 = {k: v for k, v in results.items() if k.startswith("r3") and k.endswith(f"({PHASE2_R3})")}
    if r3:
        kill = all(v["t"] < KILL_T or (v["s_eff"] or 0) < KILL_SHARE for v in r3.values())
        passed = [k for k, v in r3.items() if v["t"] >= PASS_T and (v["s_eff"] or 0) >= PASS_SHARE]
        verdict = "pass" if passed else "kill" if kill else "weak"
        print(f"**Criteria** (fixed), on the R3 policies of the slope test (R3 as Phase 1 picked it, `{PHASE2_R3}`; "
              f"the others are context): kill if every one has `t < {KILL_T}` or `s_eff < {KILL_SHARE}`; pass if "
              f"some has `s_eff ≥ {PASS_SHARE}` and `t ≥ {PASS_T}`; weak in between.  "
              f"**{verdict}**{' (' + ', '.join(passed) + ')' if passed else ''}.\n")
    return results


def section_phase2_logs(logs):
    """Where R3 cuts in the trees: from the per-node logs in `logs` (`n{N}-r3-...jsonl`, gzipped or not), per band of
    GF(2) excess, the nodes R3 ran at and the share it cut."""
    files = sorted(p for p in logs.glob("n*-r3-*.jsonl*")) if logs.is_dir() else []
    if not files:
        return
    rows = []
    for path in files:
        opener = gzip.open if path.suffix == ".gz" else open
        got, cut = defaultdict(int), defaultdict(int)
        with opener(path, "rt") as f:
            for line in f:
                r = json.loads(line)
                if r.get("method") == "r3":
                    got[band_of(r["excess_gf2"])] += 1
                    cut[band_of(r["excess_gf2"])] += r["cut"] == "refit"
        name = path.name.split(".jsonl")[0]
        rows.append([name] + [f"{fmt(cut[b] / got[b], 2)} of {got[b]}" if got[b] else "-" for b in band_labels()])
    print("Where R3 cuts in the trees (the per-node logs kept in `u4/logs`): per band of the node's GF(2) excess, the "
          "share of the nodes R3 ran at that it cut, of how many.\n")
    table(["run"] + band_labels(), rows)


def regression(names, games, by, plain, gap, floor):
    """Each variant's regression (see section 2) over the completed games whose v0 tree has at least `floor` nodes:
    the table's rows, and the figures by variant."""
    results, rows = {}, []
    for name in names:
        pts = []
        for n in games:
            r, v0 = by.get((n, name)), plain.get(n)
            if r and v0 and v0["nodes"] >= floor and r["completed"] and r["nodes"] and n in gap:
                pts.append((gap[n], math.log10(v0["nodes"] / r["nodes"]), math.log10(v0["nodes"]), r))
        fit = ols([p[0] for p in pts], [p[1] for p in pts])
        base = ols([p[0] for p in pts], [p[2] for p in pts])
        if fit is None or base is None:
            rows.append([name, len(pts)] + ["-"] * 8)
            continue
        beta, se, t, _ = fit
        b0, se0, _, _ = base
        s_eff = beta / b0 if b0 else None
        f3 = mean([p[3].get("r3_refits", 0) / p[3]["nodes"] for p in pts])
        r3ms = [p[3]["ms_r3"] / p[3]["r3_refits"] for p in pts if p[3].get("r3_refits")]
        results[name] = {"beta": beta, "t": t, "s_eff": s_eff, "f3": f3, "r3ms": mean(r3ms), "games": len(pts)}
        rows.append([name, len(pts), f"{1e3 * beta:+.2f} ± {1e3 * se:.2f}", f"{t:+.1f}",
                     f"{1e3 * b0:.2f} ± {1e3 * se0:.2f}", f"{1e3 * (b0 - beta):.2f}", fmt(s_eff, 3),
                     fmt(mean(p[1] for p in pts), 2), fmt(f3, 3), ms(mean(r3ms))])
    return rows, results


# ---------------------------------------------------------------------------------------------------------
# Phase 3: patch mode with R3

def section_phase3(trees, fits, plain):
    print("## 3. Patch mode with R3 (Phase 3)\n")
    patch = [r for r in trees if r.get("mode") == "tree" and r.get("label")]
    if not patch and not fits:
        print("No patch runs: skipped.\n")
        return
    if fits:
        rows = []
        for f in sorted(fits, key=lambda f: (f["n"], f["label"])):
            carried = f.get("carried_share") is not None  # an own fit carries nothing (its early lines count 3)
            rows.append([f["n"], f["label"], f["gf2"] - f["opt"], fmt(f["share"], 3), fmt(f.get("carried_share"), 3),
                         f.get("carried_signings") if carried else "-",
                         f"{f.get('offered_placed')} of {f.get('offered')}", f["signings"],
                         "yes" if f.get("converged") else "no", fmt(f["seconds"], 1)])
        print("The root fits (`fit`): *share* the share of the root's GF(2) gap the fit closes, *carried* that of "
              "the family carried from the previous game before its refit.\n")
        table(["n", "arm", "gap", "share", "carried", "carried signings", "cores offered placed", "signings",
               "converged", "s"], rows)
    if not patch:
        return
    by = {(r["n"], r["label"]): r for r in patch}
    games = sorted({r["n"] for r in patch})
    rows, harvest_ratios, chain_ratios, time_ratios = [], [], [], []
    for n in games:
        own, chain, harvest = (by.get((n, a)) for a in ("own", "chain", "harvest"))
        per_refit = {a: r["ms_r3"] / r["r3_refits"] for a, r in (("own", own), ("chain", chain), ("harvest", harvest))
                     if r and r.get("r3_refits")}
        h = own["nodes"] / harvest["nodes"] if own and harvest else None
        c = own["nodes"] / chain["nodes"] if own and chain else None
        tr = per_refit["harvest"] / per_refit["own"] if "harvest" in per_refit and "own" in per_refit else None
        if n not in ANCHORS:
            harvest_ratios += [h] if h else []
            chain_ratios += [c] if c else []
            time_ratios += [tr] if tr else []
        v0 = plain.get(n)
        rows.append([n, "anchor" if n in ANCHORS else "", num(v0["nodes"]) if v0 else "-"]
                    + [num(r["nodes"]) if r else "-" for r in (own, chain, harvest)]
                    + [fmt(c, 2), fmt(h, 2)] + [ms(per_refit.get(a)) for a in ("own", "chain", "harvest")])
    print("The trees with each arm's root fit, the same node method; *own/chain*, *own/harvest*: own's nodes over the "
          "arm's; R3's ms a refit per arm.\n")
    table(["n", "", "v0", "own", "chain", "harvest", "own/chain", "own/harvest", "R3 ms own", "R3 ms chain",
           "R3 ms harvest"], rows)
    mh, mc, mt = median(harvest_ratios), median(chain_ratios), median(time_ratios)
    if mh is None and mc is None:
        return
    into = (mh is not None and mh >= HARVEST_TREE) or (mt is not None and mt <= HARVEST_TIME)
    verdict = ("patch mode (harvest with chain) into E4's design" if into
               else "chain alone" if mc is not None and mc >= CHAIN_TREE else "per-game fits only")
    print(f"**Criteria** (fixed), at the median non-anchor game: own/harvest {fmt(mh, 2)} (≥ {HARVEST_TREE}?), "
          f"harvest's R3 time a refit over own's {fmt(mt, 2)} (≤ {HARVEST_TIME}?), own/chain {fmt(mc, 2)} "
          f"(≥ {CHAIN_TREE}?): **{verdict}**.\n")


# ---------------------------------------------------------------------------------------------------------
# Phase 4: R3 at the end of the chain

def section_phase4(replays, dives, phase2):
    print("## 4. What R3 costs at the end of the chain (Phase 4)\n")
    lines = [r for r in replays if r["host"] not in PHASE1_HOSTS]
    if not lines:
        print("No replays at the end of the chain: skipped.\n")
        return
    if dives:
        rows = [[d["n"], d.get("label"), d["dives"], d.get("max_depth"), d["tier1_open"], d["refits"],
                 d["cut_refit"], ms(d["ms"] / 1e3)] for d in sorted(dives, key=lambda d: d["n"])]
        print("The dives (r2's state inherited along each; the states of every Tier-1-open node on them saved):\n")
        table(["n", "label", "dives", "depth", "Tier-1-open", "R2 refits", "cut by R2", "s"], rows)
    methods = sorted({r["method"] for r in lines}, key=method_order)
    cost = {}
    for n in sorted({r["n"] for r in lines}) + [None]:
        got = [r for r in lines if n is None or r["n"] == n]
        print(f"### {'n = ' + str(n) if n else 'all the states at the end of the chain'}\n")
        rows = []
        for m in methods:
            rs = [r for r in got if r["method"] == m]
            for label in band_labels() + ["all"]:
                band = [r for r in rs if label == "all" or band_of(r["excess_gf2"]) == label]
                if not band:
                    continue
                rows.append([m, label, len(band), ms(mean(r["ms"] for r in band)), ms(median([r["ms"] for r in band])),
                             fmt(mean(r["rounds"] for r in band), 2), pct(sum(r["cut"] for r in band) / len(band)),
                             ms(mean(r["ms_exact"] for r in band))])
            if n is None:
                cost[m] = mean(r["ms"] for r in rs)
        table(["method", "GF(2) excess", "states", "ms mean", "ms median", "rounds", "cut", "exact checks ms"], rows)
    if not phase2:
        return
    rows = []
    for name, v in sorted(phase2.items()):
        if not name.startswith("r3") or v["s_eff"] is None:
            continue
        for m, c in sorted(cost.items(), key=lambda kv: method_order(kv[0])):
            c3 = c / SOLVER_NODE_MS
            gain = 10 ** (DECADES * v["s_eff"])
            net = gain / (BASE_COST + v["f3"] * c3)
            rows.append([name, fmt(v["s_eff"], 3), fmt(v["f3"], 3), m, ms(c), f"{c3:,.0f}", f"{gain:,.1f}",
                         f"{net:,.2f}", "yes" if net >= PROJECTION_LINE else "no"])
    print(f"The projection at a gap of 1,400 (design U4): `10^({DECADES:.2f} s_eff) / ({BASE_COST} + f₃ c₃)`, `c₃` "
          f"R3's mean time a refit there in solver nodes of {SOLVER_NODE_MS} ms; E4 is designed only if it "
          f"is at least {PROJECTION_LINE} for some policy and budget.\n")
    table(["policy", "s_eff", "f₃", "R3", "ms", "c₃", "tree factor", "net speed-up", f"≥ {PROJECTION_LINE}"], rows)


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    p.add_argument("--dir", type=Path, default=HERE / "u4")
    p.add_argument("--matroid", type=Path, default=HERE.parent / "matroid-intersection-bounds-2026-10")
    a = p.parse_args()
    trees = load(a.dir / "trees.jsonl", tree_key) or []
    replays = load(a.dir / "replay.jsonl", replay_key) or []
    fits = load(a.dir / "fits.jsonl", lambda r: (r["n"], r["label"])) or []
    dives = load(a.dir / "dives.jsonl", tree_key) or []
    plain, gap = plain_trees(a.matroid)
    print("# U4: R3, the slope test, patch mode and the end of the chain\n")
    print("Made by `u4.py` from `u4/`, both in the folder of this entry (see the README).\n")
    section_phase1(replays)
    phase2 = section_phase2(trees, plain, gap)
    section_phase2_logs(a.dir / "logs")
    section_phase3(trees, fits, plain)
    section_phase4(replays, dives, phase2)


if __name__ == "__main__":
    main()
