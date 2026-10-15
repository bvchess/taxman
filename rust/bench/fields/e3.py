#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The E3 pilot's tables: the node methods of the A1 bound in a branch and bound (`fields bnb`, design S4 "E3-pilot").

Reads the output of `examples/fields.rs bnb`, next to this script unless an option says otherwise: bnb.jsonl (a line
per run, `--bnb`), audit.jsonl (a line per audited node, `--audit`) and the per-node logs in logs/, named
`n{N}-{variant}-{policy}-k{K}.jsonl` (`--logs`).  A line that repeats the key of an earlier one (a rerun) replaces it;
a missing file or directory is noted and its sections skipped.  Prints Markdown to standard output: 1. the exact trees,
their ratios against v0 and the node cost model; 2. the time per node; 3. the audit (the methods against the
reference, by game and host variant and by GF(2) excess); 4. the pilot's kill criterion, fixed in advance, per game
(only 1472 decides); 5. the refits by GF(2) excess and depth, from the logs; 6. Knuth's estimates beside the exact
counts.  A run that stopped at a budget gives only bounds, and its ratios are shown as `≥` or `≤`.  Pure Python;
`./e3.py > e3_output.md`.
"""
import argparse
import json
import sys
from array import array
from bisect import bisect_left
from collections import Counter, namedtuple
from dataclasses import dataclass
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))  # the script's own directory is not on the path under `python3 -I`
from fields import fmt, load, mean, pct, table  # noqa: E402

VARIANTS = ["v0", "v1", "r1", "r2", "r2c"]
EXTRA = VARIANTS[1:]  # the variants with an extra bound: the kill criterion's candidates for the best tree
METHODS = ["r1", "r2", "r2c"]  # the refit methods the audit compares with the reference
HORIZONS = [1, 2, 5, 10, 20]  # pricings, the audit's checkpoints
HORIZON = HORIZONS[-1]  # the design's horizon: reach is measured within this many pricings
COST_NODE, COST_PRICING = 2.5, 3.0  # the model of design S4 "E3": a node costs c = 2.5 + 3p v0 nodes

# The pilot's kill criterion (design S4), fixed in advance.
DECIDING_GAME = 1472
RATIO_KILL = 3.0  # the best variant's tree less than this many times smaller than v0's ...
REACH_KILL = 0.5  # ... and the best method reaching less than this share of the reference's cuts

Bands = namedtuple("Bands", "labels uppers")  # a band is "up to uppers[i]" (inclusive); the last has no upper bound
EXCESS_BANDS = Bands(["0-20", "21-50", "51-100", "101-200", "over 200"], [20, 50, 100, 200])
DEPTH_BANDS = Bands(["0-4", "5-9", "10-14", "15+"], [4, 9, 14])
Bounded = namedtuple("Bounded", "value kind")  # a ratio of two counts; kind "=", or "≥" / "≤" when a run is partial
NO_TREES = "No trees (the runs file is missing or has none): skipped.\n"


# ---------------------------------------------------------------------------------------------------------
# Reading and arithmetic

def run_key(r):
    return (r.get("n"), r.get("variant"), r.get("policy"), r.get("k"), r.get("mode", "tree"))


def audit_key(r):
    return (r.get("n"), r.get("variant"), r.get("node"))


def div(a, b):
    """a / b, None if either is None or b is zero."""
    return None if a is None or not b else a / b


def num(x):
    return "-" if x is None else f"{x:,}"


def big(x):
    """A node count or its estimate: with thousands separators below 10^7, else in scientific notation."""
    return "-" if x is None else f"{x:,.0f}" if abs(x) < 1e7 else f"{x:.3e}"


def yes_no(flag):
    return "?" if flag is None else "yes" if flag else "no"


def band_index(bands, x):
    return bisect_left(bands.uppers, x)


def variant_rank(variant):
    return (VARIANTS.index(variant) if variant in VARIANTS else len(VARIANTS), variant)


def run_order(r):
    """Sort key of the runs: game, variant, policy, k."""
    return (r["n"],) + variant_rank(r["variant"]) + (str(r.get("policy")), r.get("k") or 0)


def label(r):
    """A run's name beyond its game: variant, policy, k."""
    return f"{r['variant']} ({r.get('policy')}, k {r.get('k')})"


def done(r):
    return bool(r.get("completed"))


def net_ms(r):
    """The run's time without the audit's (the audit is observational and outside the run's budget)."""
    return None if r.get("ms") is None else r["ms"] - (r.get("ms_audit") or 0)


def per_node(r, key):
    return div(r.get(key), r["nodes"])


def ms_per_node(r):
    return div(net_ms(r), r["nodes"])


def bounded(numerator, numerator_done, denominator, denominator_done):
    """numerator / denominator as a Bounded.  A count or time from a run that did not complete is a lower bound of
    its final value, so the ratio is exact when both runs completed, a lower bound when only the denominator's did,
    an upper bound when only the numerator's did, and unknown (None) when neither did."""
    value = div(numerator, denominator)
    if value is None:
        return None
    if numerator_done and denominator_done:
        return Bounded(value, "=")
    if denominator_done:
        return Bounded(value, "≥")
    if numerator_done:
        return Bounded(value, "≤")
    return None


def show(b, digits=2):
    return "-" if b is None else ("" if b.kind == "=" else b.kind) + fmt(b.value, digits)


def below(b, limit):
    """Whether the ratio is surely below `limit` (True), surely not (False), or its bounds leave it open (None)."""
    if b is None:
        return None
    if b.kind == "=":
        return b.value < limit
    if b.kind == "≥":
        return False if b.value >= limit else None
    return True if b.value < limit else None


def base_run(trees, n):
    """The v0 tree of game n (v0 does not depend on the policy): a completed line if there are several."""
    return max((r for r in trees if r["n"] == n and r["variant"] == "v0"), key=done, default=None)


def best_run(runs):
    """The run with the smallest tree: among the completed ones; failing those, the partial one that got furthest."""
    return max(runs, key=lambda r: (done(r), -r["nodes"] if done(r) else r["nodes"]), default=None)


def game_line(r):
    """The game's constants, from any of its lines."""
    return (f"Root GF(2) gap g = {r.get('g')}, optimum {r.get('opt')}, root GF(2) bound {r.get('gf2')}; Tier 1 at the "
            f"fit: {r.get('fit')}, the fit file's bound: {r.get('fit_bound')}.\n")


# ---------------------------------------------------------------------------------------------------------
# 1. Trees and 2. Times

def model_cost(r):
    """The model's cost of a node in v0-node units: 1 for v0, else 2.5 + 3p, p the refit pricings per Tier-1 eval."""
    if r["variant"] == "v0":
        return 1.0
    p = div(r.get("pricings_refit"), r.get("tier1_evals"))
    return None if p is None else COST_NODE + COST_PRICING * p


def tree_ratio(r, base):
    """v0's nodes over the run's, as a Bounded; None without a v0 line."""
    return None if base is None else bounded(base["nodes"], done(base), r["nodes"], done(r))


def section_trees(trees):
    print("## 1. Trees\n")
    if not trees:
        print(NO_TREES)
        return
    print("The exact tree of each variant per game against the same game's v0.  Tree ratio: v0 nodes / nodes.  p: "
          "refit pricings per Tier-1 evaluation; c = 2.5 + 3p the model's cost of a node in v0-node units (v0: 1).  "
          "Measured cost: (ms - ms_audit) per node over v0's ms per node.  Time ratio: (ms - ms_audit) / v0 ms.  Work "
          "ratio: v0 nodes / (nodes · c), the model's prediction of the time ratio's inverse (above 1: the variant "
          "wins).  `≥` and `≤`: a run stopped at a budget, so its counts are only lower bounds and the ratio is "
          "bounded on one side.\n")
    for n in sorted({r["n"] for r in trees}):
        runs = sorted((r for r in trees if r["n"] == n), key=run_order)
        base = base_run(trees, n)
        print(f"### {n}\n")
        print(game_line(runs[0]))
        if base is None:
            print("No v0 line for this game: the ratios are skipped.\n")
        out = []
        for r in runs:
            c = model_cost(r)
            work = None if base is None or not c else bounded(base["nodes"], done(base), r["nodes"] * c, done(r))
            time = None if base is None else bounded(net_ms(r), done(r), net_ms(base), done(base))
            measured = None if base is None else div(ms_per_node(r), ms_per_node(base))
            out.append([r["variant"], r.get("policy"), r.get("k"), num(r["nodes"]), yes_no(done(r)),
                        show(tree_ratio(r, base)), num(r.get("cut_gf2")), num(r.get("cut_tier1")),
                        num(r.get("cut_refit")), num(r.get("tier1_evals")), num(r.get("refits")),
                        fmt(div(r.get("pricings_refit"), r.get("refits")), 1),
                        fmt(div(r.get("pricings_refit"), r.get("tier1_evals")), 2), fmt(c, 2), fmt(measured, 2),
                        show(time), show(work)])
        table(["variant", "policy", "k", "nodes", "completed", "tree ratio", "cut gf2", "cut tier1", "cut refit",
               "Tier-1 evals", "refits", "pricings per refit", "p", "c", "measured cost per node", "time ratio",
               "work ratio"], out)
        worse = [label(r) for r in runs if r.get("better")]
        if worse:
            print(f"**Warning**: `better` is not zero for {', '.join(worse)}: a bound or P* is wrong.\n")


def section_times(trees):
    print("## 2. Times\n")
    if not trees:
        print(NO_TREES)
        return
    print("Milliseconds per node, wall time of one thread.  Total: (ms - ms_audit) / nodes.  The next four: the "
          "GF(2) bound (ms_gf2), the builds of the signed matroids (ms_builds), Tier 1 (ms_tier1) and the refits "
          "(ms_refit).  The last three are inside the evaluations of Tier 1 and the refits: matroid intersection "
          "(ms_mi), the greedies (ms_greedy) and the simplex (ms_simplex).\n")
    keys = ["ms_gf2", "ms_builds", "ms_tier1", "ms_refit", "ms_mi", "ms_greedy", "ms_simplex"]
    for n in sorted({r["n"] for r in trees}):
        print(f"### {n}\n")
        out = [[r["variant"], r.get("policy"), r.get("k"), num(r["nodes"]), fmt(ms_per_node(r), 3)]
               + [fmt(per_node(r, key), 3) for key in keys]
               for r in sorted((r for r in trees if r["n"] == n), key=run_order)]
        table(["variant", "policy", "k", "nodes", "total", "GF(2)", "builds", "Tier 1", "refits", "MI", "greedy",
               "simplex"], out)


# ---------------------------------------------------------------------------------------------------------
# 3. The audit

AuditCounts = namedtuple("AuditCounts", "audited ref_cuts within hits missed pricings ms")


def ref_cut(line):
    return bool(line["reference"]["cut"])


def cut_within(block, pricings):
    """Whether the method's excess is at most 0 after `pricings` pricings (the last value stands past the trace)."""
    after = block["after"]
    return after[min(pricings, len(after) - 1)] <= 0


def count_method(lines, method):
    """The audit's counts for one method over the lines that have it.  Within: the nodes it cuts within each of the
    HORIZONS pricings; hits: the reference's cuts it also cuts within HORIZON; missed: its cuts the reference does
    not make."""
    pairs = [(line, line[method]) for line in lines if line.get(method) and line[method].get("after")]
    return AuditCounts(
        len(pairs), sum(ref_cut(line) for line, _ in pairs),
        [sum(cut_within(block, h) for _, block in pairs) for h in HORIZONS],
        sum(ref_cut(line) and cut_within(block, HORIZON) for line, block in pairs),
        sum(not ref_cut(line) and cut_within(block, HORIZON) for line, block in pairs),
        mean(block.get("pricings") for _, block in pairs), mean(block.get("ms") for _, block in pairs))


def reach(counts):
    return div(counts.hits, counts.ref_cuts)


def hosts_of(lines):
    return sorted({line["variant"] for line in lines}, key=variant_rank)


def methods_table(lines):
    out = []
    for host in hosts_of(lines):
        for method in METHODS:
            c = count_method([line for line in lines if line["variant"] == host], method)
            out.append([host, method, c.audited, c.ref_cuts] + c.within
                       + [fmt(reach(c)), c.missed, fmt(c.pricings, 1), fmt(c.ms, 1)])
    table(["host", "method", "audited", "reference cuts"] + [f"within {h}" for h in HORIZONS]
          + [f"reach within {HORIZON}", "missed", "mean pricings", "mean ms"], out)


def reference_table(lines):
    out = []
    for host in hosts_of(lines):
        refs = [line["reference"] for line in lines if line["variant"] == host]
        cuts = sum(bool(ref["cut"]) for ref in refs)
        stops = Counter(ref.get("stop") for ref in refs)
        out.append([host, len(refs), cuts, pct(div(cuts, len(refs))), fmt(mean(ref.get("pricings") for ref in refs), 1),
                    fmt(mean(ref.get("ms") for ref in refs), 1),
                    ", ".join(f"{stop} {count}" for stop, count in sorted(stops.items(), key=str))])
    table(["host", "audited", "cuts", "cut rate", "mean pricings", "mean ms", "stops"], out)


def bands_table(lines):
    out = []
    for host in hosts_of(lines):
        for i, name in enumerate(EXCESS_BANDS.labels):
            rows = [line for line in lines if line["variant"] == host
                    and band_index(EXCESS_BANDS, line["excess_gf2"]) == i]
            if rows:
                counts = [count_method(rows, m) for m in METHODS]
                out.append([host, name, len(rows), pct(div(sum(ref_cut(line) for line in rows), len(rows)))]
                           + [pct(div(c.within[-1], c.audited)) for c in counts])
    table(["host", "GF(2) excess", "audited", "reference"] + METHODS, out)


def section_audit(audits):
    print("## 3. The audit\n")
    if not audits:
        print("No audit lines: skipped.\n")
        return
    print("At the audited nodes (Tier-1-open, chosen by a hash), each method's excess after 1, 2, 5, 10 and 20 "
          "pricings against the reference (E1n's node mode, certified, from the same inherited state).  Host: the "
          "variant whose tree the node is in.  Within i: the nodes the method cuts within i pricings (excess at "
          "most 0).  Reach: the share of the reference's cuts the method also cuts within 20 pricings.  Missed: the "
          "nodes the method cuts within 20 that the reference does not.  Pricings and ms: means over the nodes.\n")
    for n in sorted({line["n"] for line in audits}):
        lines = [line for line in audits if line["n"] == n]
        print(f"### {n}\n")
        methods_table(lines)
        print("The reference alone: the nodes it cuts, its pricings and time, and why it stopped.\n")
        reference_table(lines)
        print(f"By the node's GF(2) excess: the share of the audited nodes the reference cuts, and the share each "
              f"method cuts within {HORIZON} pricings.\n")
        bands_table(lines)


# ---------------------------------------------------------------------------------------------------------
# 4. The pilot's kill criterion

def best_method(lines):
    """(method, counts) of the highest reach within HORIZON, or None if no method has a reach."""
    counts = {m: count_method(lines, m) for m in METHODS}
    reaching = [m for m in METHODS if reach(counts[m]) is not None]
    return max(((m, counts[m]) for m in reaching), key=lambda mc: reach(mc[1]), default=None)


def reading(weak_ratio, weak_reach):
    """The criterion's verdict and what it points at (design S4: either alone is not a kill)."""
    if weak_ratio and weak_reach:
        return "kill", "both hold"
    if weak_ratio is False and weak_reach is False:
        return "no kill", "neither holds"
    if weak_ratio is False and weak_reach:
        return "no kill", "a strong ratio with a weak audit points at the method"
    if weak_reach is False and weak_ratio:
        return "no kill", "a weak ratio with a strong audit points at the policy"
    if weak_ratio is False or weak_reach is False:
        return "no kill", "one condition fails, the other is not settled by the data"
    return "undecided", "the data settle neither condition"


def section_kill(trees, audits):
    print("## 4. The pilot's kill criterion\n")
    if not trees:
        print(NO_TREES)
        return
    print(f"Fixed in advance by the design: stop E3 if the best variant's tree is less than {RATIO_KILL:g} times "
          f"smaller than v0's **and** the best method reaches less than {100 * REACH_KILL:.0f}% of the reference's "
          f"cuts within {HORIZON} pricings.  Either alone is not a kill: a weak tree ratio with a strong audit points "
          f"at the policy, a weak audit with a strong ratio at the method.  Best variant: the smallest tree among "
          f"{', '.join(EXTRA)} (a completed one before a partial one).  Best method: the highest reach among "
          f"{', '.join(METHODS)}, over the audited nodes of every host variant of the game.  **Only {DECIDING_GAME} "
          f"decides**; the other games show the trend.\n")
    out, verdicts = [], {}
    for n in sorted({r["n"] for r in trees}):
        base = base_run(trees, n)
        best = best_run([r for r in trees if r["n"] == n and r["variant"] in EXTRA])
        ratio_b = None if best is None else tree_ratio(best, base)
        lines = [line for line in audits or [] if line["n"] == n]
        method = best_method(lines)
        weak_ratio = below(ratio_b, RATIO_KILL)
        weak_reach = None if method is None else reach(method[1]) < REACH_KILL
        outcome, why = reading(weak_ratio, weak_reach)
        verdicts[n] = (outcome, why)
        out.append([n, "-" if best is None else label(best), num(None if best is None else best["nodes"]),
                    show(ratio_b), yes_no(weak_ratio), "-" if method is None else method[0],
                    "-" if method is None else f"{method[1].hits} of {method[1].ref_cuts}",
                    "-" if method is None else fmt(reach(method[1])), yes_no(weak_reach),
                    outcome + ("" if n == DECIDING_GAME else " (does not decide)")])
    table(["game", "best variant", "its nodes", "tree ratio", f"below {RATIO_KILL:g}", "best method",
           f"reference cuts hit within {HORIZON}", "reach", f"below {REACH_KILL:g}", "verdict"], out)
    if DECIDING_GAME in verdicts:
        outcome, why = verdicts[DECIDING_GAME]
        print(f"**Verdict at {DECIDING_GAME}: {outcome}** ({why}).\n")
    else:
        print(f"**No verdict**: {DECIDING_GAME} is not in the data, and only {DECIDING_GAME} decides.\n")


# ---------------------------------------------------------------------------------------------------------
# 5. Refits by excess and depth (the per-node logs)

@dataclass
class Group:
    """What the log says of the Tier-1-open nodes in one band."""
    opened: int = 0  # Tier-1-open nodes (Tier 1 left them uncut)
    refits: int = 0  # of them, the nodes a refit ran at
    cuts: int = 0  # refits that cut the node
    pricings: int = 0  # pricings of all refits
    subtree: int = 0  # sizes of the refitted nodes' subtrees in the run's tree
    uncut: int = 0  # refitted nodes not cut
    uncut_subtree: int = 0  # their subtree sizes
    covered: int = 0  # nodes of the run in the subtrees of the refitted nodes (each counted once)


# A node belongs to one excess band, one depth band, and to the whole: the groups are numbered in that order.
GROUPS = ([("GF(2) excess", name) for name in EXCESS_BANDS.labels] + [("depth", name) for name in DEPTH_BANDS.labels]
          + [("all", "all")])
FIRST_DEPTH, WHOLE = len(EXCESS_BANDS.labels), len(GROUPS) - 1
GROUPS_OF_MASK = [tuple(g for g in range(len(GROUPS)) if mask >> g & 1) for mask in range(1 << len(GROUPS))]


def read_log(path):
    """One pass over a run's node log: (the nodes, a Group per GROUPS entry).  The groups' `covered` is the number of
    nodes in the union of the subtrees of the refitted nodes of the group: ids are in visiting order and a child's id
    exceeds its parent's, so one sweep in id order marks a node when it or any ancestor was refitted."""
    groups = [Group() for _ in GROUPS]
    ids, parents, masks = array("q"), array("q"), array("H")  # a node's id, its parent's (-1: none), its groups
    with path.open() as f:
        for line in f:
            if not line.startswith("{"):
                continue
            r = json.loads(line)
            ids.append(r["id"])
            parents.append(-1 if r.get("parent") is None else r["parent"])
            tier1 = r.get("excess_tier1")
            if tier1 is None or tier1 <= 0 or r.get("excess_gf2") is None:
                masks.append(0)
                continue
            member = (band_index(EXCESS_BANDS, r["excess_gf2"]), FIRST_DEPTH + band_index(DEPTH_BANDS, r["depth"]),
                      WHOLE)
            refitted = r.get("pricings") is not None
            for g in member:
                group = groups[g]
                group.opened += 1
                if refitted:
                    group.refits += 1
                    group.pricings += r["pricings"]
                    group.subtree += r["subtree"]
                    if r["cut"] == "refit":
                        group.cuts += 1
                    else:
                        group.uncut += 1
                        group.uncut_subtree += r["subtree"]
            masks.append(sum(1 << g for g in member) if refitted else 0)
    in_order = all(a < b for a, b in zip(ids, ids[1:]))
    order = range(len(ids)) if in_order else sorted(range(len(ids)), key=ids.__getitem__)
    covering = array("H", bytes(2 * (max(ids, default=-1) + 1)))  # per id, the groups with a refitted node above it
    for i in order:
        p = parents[i]
        covering[ids[i]] = mask = masks[i] | (covering[p] if 0 <= p < len(covering) else 0)
        for g in GROUPS_OF_MASK[mask]:
            groups[g].covered += 1
    return len(ids), groups


def log_path(directory, r):
    return directory / f"n{r['n']}-{r['variant']}-{r.get('policy')}-k{r.get('k')}.jsonl"


def section_refits(trees, directory):
    print("## 5. Refits by excess and depth\n")
    runs = sorted((r for r in trees if r.get("refits")), key=run_order)
    if not runs:
        print("No run with refits: skipped.\n")
        return
    if not directory.is_dir():
        print(f"No log directory `{directory}`: skipped.\n")
        return
    print("From the per-node logs: the Tier-1-open nodes (Tier 1 left them uncut) by the GF(2) excess and by the depth "
          "of the node, and all together.  Refit cut rate: the share of the refits that cut their node.  Subtree: "
          "the size in the run's tree of the subtree under a refitted node (1 if the refit cut it), of all refits, "
          "and of those that did not cut.  Nodes in those subtrees: the share of the run's nodes that lie under a "
          "refitted node of the band, each counted once.\n")
    missing = []
    for r in runs:
        path = log_path(directory, r)
        if not path.exists():
            missing.append(path.name)
            continue
        nodes, groups = read_log(path)
        print(f"### {r['n']}, {label(r)}\n")
        if nodes != r["nodes"]:
            print(f"**Warning**: the log has {num(nodes)} nodes, the run's line {num(r['nodes'])}.\n")
        out = []
        for (kind, name), g in zip(GROUPS, groups):
            if g.opened:
                out.append([kind, name, num(g.opened), num(g.refits), pct(div(g.cuts, g.refits)),
                            fmt(div(g.pricings, g.refits), 1), fmt(div(g.subtree, g.refits), 1),
                            fmt(div(g.uncut_subtree, g.uncut), 1), pct(div(g.covered, nodes))])
        table(["by", "band", "Tier-1-open nodes", "refits", "refit cut rate", "mean pricings", "mean subtree",
               "mean subtree, uncut refits", "nodes in those subtrees"], out)
    if missing:
        print(f"No log for {', '.join(missing)} in `{directory}`: skipped.\n")


# ---------------------------------------------------------------------------------------------------------
# 6. Knuth

def lies_in(value, lo, hi):
    """Whether `value` lies in [lo, hi]; None if there is no value."""
    return None if value is None else lo <= value <= hi


def exact_nodes(r):
    """The node count of a completed tree line, None for a partial one or no line."""
    return r["nodes"] if r and done(r) else None


def interval(mean_, lo, hi, show_=big):
    return f"{show_(mean_)} [{show_(lo)}, {show_(hi)}]"


def section_knuth(knuths, trees):
    print("## 6. Knuth\n")
    if not knuths:
        print("No Knuth lines: skipped.\n")
        return
    print("Paired estimates of the tree sizes from the same dives: the GF(2) tree's (that of v0) and the variant's "
          "(A1), and the ratio of their means (the estimated tree ratio), each with the bootstrap interval of the "
          "mean at the level shown.  Beside them the exact counts, where a completed tree line of the same game "
          "exists: v0's nodes for the GF(2) estimate, the variant's for the A1 estimate, their ratio for the ratio.  "
          "In: whether the exact figure lies in the interval.\n")
    exact = {(r["n"], r["variant"], r.get("policy"), r.get("k")): r for r in trees}
    out = []
    for r in sorted(knuths, key=run_order):
        v0, mine = base_run(trees, r["n"]), exact.get((r["n"], r["variant"], r.get("policy"), r.get("k")))
        ratio_exact = tree_ratio(mine, v0) if mine else None
        ratio_value = ratio_exact.value if ratio_exact and ratio_exact.kind == "=" else None
        out.append([r["n"], r["variant"], r.get("policy"), r.get("k"), num(r.get("dives")), pct(r.get("ci")),
                    interval(r["gf2_mean"], r["gf2_lo"], r["gf2_hi"]), big(exact_nodes(v0)),
                    yes_no(lies_in(exact_nodes(v0), r["gf2_lo"], r["gf2_hi"])),
                    interval(r["a1_mean"], r["a1_lo"], r["a1_hi"]), big(exact_nodes(mine)),
                    yes_no(lies_in(exact_nodes(mine), r["a1_lo"], r["a1_hi"])),
                    interval(r["ratio"], r["ratio_lo"], r["ratio_hi"], fmt), fmt(ratio_value, 2),
                    yes_no(lies_in(ratio_value, r["ratio_lo"], r["ratio_hi"]))])
    table(["game", "variant", "policy", "k", "dives", "level", "GF(2) estimate [interval]", "v0 nodes", "in",
           "A1 estimate [interval]", "variant nodes", "in", "ratio [interval]", "exact ratio", "in"], out)


# ---------------------------------------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    parser.add_argument("--bnb", type=Path, default=HERE / "bnb.jsonl",
                        help="the runs file, a line per run of `fields bnb` (default: bnb.jsonl next to the script)")
    parser.add_argument("--audit", type=Path, default=HERE / "audit.jsonl",
                        help="the audit file, a line per audited node (default: audit.jsonl next to the script)")
    parser.add_argument("--logs", type=Path, default=HERE / "logs",
                        help="the directory of the per-node logs (default: logs next to the script)")
    args = parser.parse_args()

    bnb = load(args.bnb, run_key)
    audits = load(args.audit, audit_key)
    trees = [r for r in bnb or [] if r.get("mode", "tree") == "tree"]
    knuths = [r for r in bnb or [] if r.get("mode") == "knuth"]
    print(f"# The E3 pilot\n\nruns: `{args.bnb.name}`, {len(trees)} trees and {len(knuths)} Knuth lines; audit: "
          f"`{args.audit.name}`, {len(audits or [])} nodes; logs: `{args.logs}`.\n")
    section_trees(trees)
    section_times(trees)
    section_audit(audits)
    section_kill(trees, audits)
    section_refits(trees, args.logs)
    section_knuth(knuths, trees)


if __name__ == "__main__":
    main()
