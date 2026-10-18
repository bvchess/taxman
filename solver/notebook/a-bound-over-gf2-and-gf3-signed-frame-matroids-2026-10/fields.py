#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The fields study's tables: rust/docs/experiments/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10.md.

The data files are the output of `examples/fields.rs`: roots.jsonl (`fields roots`, a line per root and arm),
nodes.jsonl (`fields nodes`) and census.jsonl (`fields roots --census`), next to this script unless an option says
otherwise; a missing file is noted and its sections skipped.  A line that repeats the key of an earlier one (a rerun)
replaces it.  The share of a root is the fraction of the GF(2) gap the arm's bound closes, (gf2 - bound) / (gf2 - opt),
null where gf2 = opt; a root counts as converged when its bracket is within 1% of that gap.  Pure Python;
`./fields.py > fields_output.md`, or `./fields.py --roots lite.jsonl --lite` for the quick look at the main arm.
"""
import argparse
import json
import math
import statistics as st
import sys
from collections import Counter, namedtuple
from pathlib import Path

HERE = Path(__file__).resolve().parent
ARMS = ["gf2", "f1", "f1r1", "f1r3", "cores", "f1cores"]
MAIN = "f1cores"
FIXED = ["f1", "f1r1", "f1r3"]
BEST = "best fixed"
SIGNING_ARMS = ["f1cores", "cores"]
BAND = 100
POINTS = 100 * 100  # a share per unit of n, as percentage points per 100 n

# The design's criteria, fixed in advance.
K_FROM = 1700  # K1 and K2 count the roots from this n on
UNCONVERGED_MAX = 0.20  # inconclusive above this fraction of unconverged roots
K1_KILL = 0.25  # killed if the mean share is below it
K2_KILL_SLOPE, K2_KILL_T = -2.0, -2.0  # killed if the slope (points per 100 n) and t are both at most these
STRONG_SHARE, STRONG_SLOPE = 0.5, -1.0  # passes strongly: mean share at least, slope above
REACH = 0.8  # the design asks whether F1 + R3 reaches this fraction of the main arm's share
LITE_FROM, LITE_KILL = 1500, 0.15  # the E1-lite criterion: early kill below the mean share, all converged
K3_FIXTURE, K3_KILL = 1215, 0.5  # K3: node mode at this fixture must cut at least this fraction of the open nodes
CAP = 8  # core signings per frame at most (`--per-frame` of examples/fields.rs)

# Figures recorded in rust/docs/experiments/, by fixture n: the cuts of the probe's 300 nodes by the GF(2) bound alone
# ("Matroid intersection bounds (2026-10)", nodes table) and by the LP over all conflicts ("How far can nogoods go
# (2026-10)", the probe's nodes), and the range of the cuts of the per-frame bound at every 10th node, 30 a fixture
# ("The per-frame bound (2026-10)": the root's multipliers to 20 steps of the node's own).
REFERENCE = {
    936: dict(gf2=163, conflicts=278, per_frame="25-29"),
    1215: dict(gf2=167, conflicts=249, per_frame="23-27"),
}
REFERENCE_NODES, REFERENCE_SAMPLE = 300, 30
MODES = ["root", "node"]

Trend = namedtuple("Trend", "slope se t")  # points per 100 n, its standard error, t


# ---------------------------------------------------------------------------------------------------------
# Reading and arithmetic

def load(path, key):
    """The records of a JSON-lines file, the last of each `key` (a rerun's) kept; None if there is no file."""
    if not path.exists():
        return None
    records = {}
    for number, line in enumerate(path.read_text().splitlines(), 1):
        if not line.startswith("{"):
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            print(f"(line {number} of {path.name} is not complete JSON: skipped)", file=sys.stderr)
            continue
        records[key(record)] = record
    return list(records.values())


def mean(values):
    values = [v for v in values if v is not None]
    return st.fmean(values) if values else None


def median(values):
    values = [v for v in values if v is not None]
    return st.median(values) if values else None


def ratio(a, b):
    return a / b if b else None


def fraction(rows, test):
    """The fraction of the rows for which `test` holds, among those where it is decided (not None)."""
    decided = [t for t in (test(r) for r in rows) if t is not None]
    return ratio(sum(decided), len(decided))


def val(r, key, scale=1):
    v = r.get(key)
    return None if v is None else v * scale


def flag(r, key, test=bool):
    return None if r.get(key) is None else test(r[key])


def fmt(x, digits=3):
    return "-" if x is None else f"{x:.{digits}f}"


def pct(x):
    return "-" if x is None else f"{100 * x:.1f}%"


def of(k, m):
    return f"{k} of {m} ({100 * k / m:.0f}%)" if m else f"{k} of 0"


def trend(rows):
    """Least squares of the share against n: a Trend, or None for fewer than three roots or a single n."""
    xs, ys = [r["n"] for r in rows], [share(r) for r in rows]
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
    return Trend(POINTS * b, POINTS * se, t)


def trend_cells(tr):
    return ["-", "-"] if tr is None else [f"{tr.slope:+.2f} ± {tr.se:.2f}", f"{tr.t:+.1f}"]


def table(header, rows):
    print("| " + " | ".join(header) + " |")
    print("|" + "---|" * len(header))
    for row in rows:
        print("| " + " | ".join(str(c) for c in row) + " |")
    print()


def by_band(rows):
    """(label, rows) for each band of BAND in n that has rows, then the whole."""
    groups = {}
    for r in rows:
        groups.setdefault(r["n"] // BAND * BAND, []).append(r)
    bands = [(f"{lo}-{lo + BAND - 1}", groups[lo]) for lo in sorted(groups)]
    return bands + [("all", rows)] if rows else []


# ---------------------------------------------------------------------------------------------------------
# Roots

def share(r):
    return r.get("share")


def is_open(r):
    """A root with a defined share: the GF(2) bound leaves a gap."""
    return share(r) is not None


def converged(r):
    c = r.get("converged")
    return c if c is not None else r.get("stop") == "Converged"


def counted(r):
    return is_open(r) and converged(r)


def arms_in(roots):
    names = {r["arm"] for r in roots}
    return [a for a in ARMS if a in names] + sorted(names - set(ARMS))


def section_bands(roots_by_arm):
    print("## 1. Per arm, by band of 100 n\n")
    print("Share: the fraction of the GF(2) gap the bound closes; its mean and median are over the converged roots "
          "(bracket within 1% of the gap).  Bracket and seconds are means over the roots with gf2 > opt.\n")
    for arm, rows in roots_by_arm.items():
        print(f"### {arm}\n")
        out = []
        for label, rs in by_band(rows):
            open_ = [r for r in rs if is_open(r)]
            done = [share(r) for r in open_ if converged(r)]
            out.append([label, len(rs), len(open_), len(done), fmt(mean(done)), fmt(median(done)),
                        fmt(mean(val(r, "bracket") for r in open_), 2),
                        fmt(mean(val(r, "ms", 1e-3) for r in open_), 1)])
        table(["n", "roots", "gf2 > opt", "converged", "mean share", "median share", "mean bracket", "mean s"], out)


def criteria(rows):
    """The numbers of the kill criteria over the roots of one arm."""
    open_ = [r for r in rows if is_open(r)]
    done = [r for r in open_ if converged(r)]
    open_big = [r for r in open_ if r["n"] >= K_FROM]
    big = [r for r in done if r["n"] >= K_FROM]
    return dict(open=len(open_), counted=len(done), open_big=len(open_big), big=len(big),
                unconverged=ratio(len(open_) - len(done), len(open_)),
                unconverged_big=ratio(len(open_big) - len(big), len(open_big)),
                mean_big=mean(share(r) for r in big), trend=trend(big))


def verdict(c):
    if not c["open"] or c["unconverged"] > UNCONVERGED_MAX or c["mean_big"] is None:
        return "inconclusive"
    tr = c["trend"]
    k1 = c["mean_big"] < K1_KILL
    k2 = tr is not None and tr.slope <= K2_KILL_SLOPE and tr.t <= K2_KILL_T
    if k1 or k2:
        return "killed by " + " and ".join(name for name, hit in (("K1", k1), ("K2", k2)) if hit)
    strong = c["mean_big"] >= STRONG_SHARE and tr is not None and tr.slope > STRONG_SLOPE
    return "passes strongly" if strong else "passes"


def main_arm_criteria(c):
    tr = c["trend"]
    print(f"* unconverged: {of(c['open'] - c['counted'], c['open'])} of the roots with a defined share "
          f"(inconclusive above {100 * UNCONVERGED_MAX:.0f}%); of those with n ≥ {K_FROM}: "
          f"{of(c['open_big'] - c['big'], c['open_big'])}")
    print(f"* K1: mean share over the {c['big']} counted roots with n ≥ {K_FROM}: {fmt(c['mean_big'])} "
          f"(killed below {K1_KILL})")
    slope = "no fit (under three roots)"
    if tr is not None:
        slope = f"{tr.slope:+.2f} ± {tr.se:.2f} points per 100 n, t {tr.t:+.1f}"
    print(f"* K2: slope of the share against n over the same roots: {slope} "
          f"(killed at slope ≤ {K2_KILL_SLOPE:g} and t ≤ {K2_KILL_T:g})")
    print(f"* passes strongly: mean share ≥ {STRONG_SHARE} and slope above {STRONG_SLOPE:g} point per 100 n\n")
    if c["open"] and c["unconverged"] <= UNCONVERGED_MAX and (c["unconverged_big"] or 0) > UNCONVERGED_MAX:
        print(f"Note: more than {100 * UNCONVERGED_MAX:.0f}% of the roots with n ≥ {K_FROM} are unconverged, "
              "though not of all roots: K1 and K2 rest on the converged ones.\n")
    print(f"**Verdict for {MAIN}: {verdict(c)}**\n")


def lite_criterion(rows):
    big = [r for r in rows if is_open(r) and r["n"] >= LITE_FROM]
    done = [r for r in big if converged(r)]
    m = mean(share(r) for r in done)
    if not big:
        outcome = f"no roots with n ≥ {LITE_FROM} yet"
    elif m is None:
        outcome = f"none of the {len(big)} roots with n ≥ {LITE_FROM} has converged yet"
    elif m >= LITE_KILL:
        outcome = "no early kill"
    elif len(done) == len(big):
        outcome = "early kill"
    else:
        outcome = f"below {LITE_KILL} but only {len(done)} of {len(big)} converged: no early kill yet"
    print(f"**E1-lite criterion** ({MAIN}): mean share over the {len(done)} converged of the {len(big)} roots with "
          f"n ≥ {LITE_FROM}: {fmt(m)} (early kill below {LITE_KILL} with all of them converged): {outcome}\n")


def section_criteria(roots_by_arm, lite):
    print(f"## 2. The kill criteria on {MAIN}\n")
    print(f"Fixed in advance by the design.  Only converged roots with a defined share count; K1 and K2 use the "
          f"roots with n ≥ {K_FROM}.\n")
    crit = {arm: criteria(rows) for arm, rows in roots_by_arm.items()}
    if MAIN in crit:
        main_arm_criteria(crit[MAIN])
    else:
        print(f"No {MAIN} roots.\n")
    print("Every arm, the same numbers:\n")
    table(["arm", "roots, gf2 > opt", "counted", "unconverged", f"counted, n ≥ {K_FROM}",
           f"unconverged, n ≥ {K_FROM}", f"mean share, n ≥ {K_FROM}", "slope ± se, points per 100 n", "t"],
          [[arm, c["open"], c["counted"], pct(c["unconverged"]), c["big"], pct(c["unconverged_big"]),
            fmt(c["mean_big"])] + trend_cells(c["trend"]) for arm, c in crit.items()])
    print("The same over all roots with a defined share, converged or not (the bound is valid either way, and the "
          "share of an unconverged root is at most the LP's):\n")
    out = []
    for arm, rows in roots_by_arm.items():
        every = [r for r in rows if is_open(r)]
        big = [r for r in every if r["n"] >= K_FROM]
        out.append([arm, len(every), fmt(mean(share(r) for r in every)), len(big), fmt(mean(share(r) for r in big))]
                   + trend_cells(trend(big)))
    table(["arm", "roots", "mean share", f"roots, n ≥ {K_FROM}", f"mean share, n ≥ {K_FROM}",
           "slope ± se, points per 100 n", "t"], out)
    if lite and MAIN in roots_by_arm:
        lite_criterion(roots_by_arm[MAIN])


def section_side_by_side(roots_by_arm):
    print("## 3. Arms side by side\n")
    arms = list(roots_by_arm)
    by_n = {arm: {r["n"]: r for r in rows} for arm, rows in roots_by_arm.items()}

    def cell(arm, n):
        r = by_n[arm].get(n)
        return "-" if r is None or not is_open(r) else fmt(share(r)) + ("" if converged(r) else "*")

    def gap(n):
        return next((r["gf2"] - r["opt"] for d in by_n.values() if n in d for r in [d[n]] if "gf2" in r and "opt" in r),
                    "-")

    ns = sorted({n for d in by_n.values() for n in d})
    print("Share of the GF(2) gap closed, per root; `*` marks a root that did not converge.\n")
    table(["n", "gap"] + arms, [[n, gap(n)] + [cell(arm, n) for arm in arms] for n in ns])
    if MAIN not in by_n or not any(arm in by_n for arm in FIXED):
        print(f"The ratio to {MAIN} needs roots of {MAIN} and of a fixed arm ({', '.join(FIXED)}): skipped.\n")
        return
    ratios = {name: [] for name in FIXED + [BEST]}
    for n, main in by_n[MAIN].items():
        if n < K_FROM or not counted(main) or share(main) <= 0:
            continue
        fixed = {arm: share(by_n[arm][n]) for arm in FIXED if n in by_n.get(arm, {}) and counted(by_n[arm][n])}
        for arm, s in fixed.items():
            ratios[arm].append(s / share(main))
        if fixed:
            ratios[BEST].append(max(fixed.values()) / share(main))
    print(f"The share of the fixed arms against {MAIN}'s, per converged root with n ≥ {K_FROM} (both arms converged; "
          f"the best fixed arm is the highest of the converged ones at that n):\n")
    table(["arm", "roots", f"mean ratio to {MAIN}", f"reaches {100 * REACH:.0f}%"],
          [[name, len(rs), fmt(mean(rs)), "-" if not rs else "yes" if mean(rs) >= REACH else "no"]
           for name, rs in ratios.items()])
    print(f"The design asks whether F1 + R3 (arm f1r3) reaches {100 * REACH:.0f}% of the main arm: "
          f"mean ratio {fmt(mean(ratios['f1r3']))} over {len(ratios['f1r3'])} roots.\n")


def section_signings(roots_by_arm):
    print("## 4. Signings and cores\n")
    print(f"Over the roots with gf2 > opt.  Signings per frame: core-derived signings (`core_signings`, the fixed "
          f"ones left out) per frame that carries an extra matroid; cores per signing: pooled over the signings; "
          f"capped: roots with {CAP} core signings (the cap) on some frame.\n")
    for arm in SIGNING_ARMS:
        if arm not in roots_by_arm:
            continue
        print(f"### {arm}\n")
        out = []
        for label, rs in by_band(roots_by_arm[arm]):
            rs = [r for r in rs if is_open(r)]
            with_signings = [r for r in rs if r.get("core_signings") is not None]
            signings = frames = cores = capped = 0
            for r in with_signings:
                per_frame = Counter(f for f, _ in r["core_signings"])
                carrying = {f for f, m in enumerate(r.get("matroids") or []) if m > 0} | set(per_frame)
                signings += len(r["core_signings"])
                cores += sum(c for _, c in r["core_signings"])
                frames += len(carrying)
                capped += any(c >= CAP for c in per_frame.values())
            out.append([label, len(rs), fmt(ratio(signings, frames), 2), fmt(ratio(cores, signings), 2), capped]
                       + [fmt(mean(r.get(k) for r in rs), 1)
                          for k in ("cores_found", "cores_assigned", "cores_skipped", "cores_repeated", "gf3_loops")])
        table(["n", "roots", "signings per frame", "cores per signing", "capped", "cores found", "assigned",
               "skipped (unplaced)", "repeated", "GF(3) loops"], out)


def section_support(roots_by_arm):
    print(f"## 5. The LP's support ({MAIN})\n")
    if MAIN not in roots_by_arm:
        print(f"No {MAIN} roots.\n")
        return
    print("Over the roots with gf2 > opt.  Promotable: the support columns whose set can be promoted whole; "
          "unpromotable mass: the multiplier mass on the others.\n")
    out = []
    for label, rs in by_band(roots_by_arm[MAIN]):
        rs = [r for r in rs if is_open(r)]
        out.append([label, len(rs), fmt(mean(r.get("support") for r in rs), 1),
                    fmt(mean(ratio(r["support_promotable"], r["support"]) if r.get("support") and
                             r.get("support_promotable") is not None else None for r in rs)),
                    fmt(mean(r.get("mass_unpromotable") for r in rs))])
    table(["n", "roots", "support size", "share promotable", "mass unpromotable"], out)


COSTS = [("pricings, MI", "pricings_mi", 1, 0), ("pricings, greedy", "pricings_greedy", 1, 0),
         ("cuts, cheap", "cuts_cheap", 1, 0), ("cuts, exact", "cuts_exact", 1, 0),
         ("exact checks", "checks_exact", 1, 0), ("pivots, master", "pivots", 1, 0),
         ("pivots, exact checks", "pivots_exact", 1, 0), ("repairs", "repairs", 1, 1),
         ("s, total", "ms", 1e-3, 2), ("s, intersections", "ms_mi", 1e-3, 2), ("s, simplex", "ms_simplex", 1e-3, 2)]


def section_cost(roots_by_arm):
    print("## 6. Cost\n")
    print("Means over the roots with gf2 > opt.  MI: matroid intersection; the exact checks are the separation's.\n")
    for arm, rows in roots_by_arm.items():
        print(f"### {arm}\n")
        out = []
        for label, rs in by_band(rows):
            rs = [r for r in rs if is_open(r)]
            out.append([label, len(rs)] + [fmt(mean(val(r, key, scale) for r in rs), digits)
                                           for _, key, scale, digits in COSTS])
        table(["n", "roots"] + [name for name, *_ in COSTS], out)
    if MAIN in roots_by_arm:
        print(f"### Time per pricing in the intersections ({MAIN})\n")
        out = []
        for label, rs in by_band(roots_by_arm[MAIN]):
            rs = [r for r in rs if is_open(r) and r.get("ms_mi") is not None and r.get("pricings_mi") is not None]
            out.append([label, len(rs), fmt(ratio(sum(r["ms_mi"] for r in rs), sum(r["pricings_mi"] for r in rs)), 3)])
        table(["n", "roots", "ms per pricing (total ms_mi / total pricings_mi)"], out)


# ---------------------------------------------------------------------------------------------------------
# Nodes

def cut(r):
    c = r.get("cut")
    return c if c is not None else r.get("bound") is not None and r["bound"] <= r["target"]


def excess(r):
    e = r.get("excess")
    return e if e is not None else r["bound"] - r["target"] if r.get("bound") is not None else None


def gf2_open(r):
    return r["gf2"] > r["target"]


def mode_order(mode):
    return (MODES.index(mode) if mode in MODES else len(MODES), mode)


def section_nodes(nodes, path):
    print("## 7. Nodes\n")
    if not nodes:
        print(f"No nodes in `{path.name}` (missing or empty): skipped.\n")
        return
    rows_of = {}
    for r in nodes:
        rows_of.setdefault((r["n"], r["mode"]), []).append(r)
    print("A node is cut when the mode's bound (the lower of it and the node's GF(2) bound) is at most the target.  "
          "Open: nodes the GF(2) bound alone leaves uncut (gf2 > target).\n")
    out = []
    for (n, mode), rows in sorted(rows_of.items(), key=lambda kv: (kv[0][0], mode_order(kv[0][1]))):
        opened = [r for r in rows if gf2_open(r)]
        out.append([n, mode, len(rows), sum(1 for r in rows if not gf2_open(r)), sum(1 for r in rows if cut(r)),
                    of(sum(1 for r in opened if cut(r)), len(opened)),
                    fmt(mean(excess(r) for r in rows if not cut(r)), 1), fmt(mean(r.get("ms") for r in rows), 1)])
    table(["n", "mode", "nodes", "cut by GF(2)", "cut", "cut among the open", "mean excess, uncut", "mean ms"], out)

    def cuts_here(n, mode):
        rows = rows_of.get((n, mode))
        return "-" if not rows else of(sum(1 for r in rows if cut(r)), len(rows))

    def gf2_here(n):
        rows = max((rs for (m, _), rs in rows_of.items() if m == n), key=len, default=None)
        return "-" if not rows else of(sum(1 for r in rows if not gf2_open(r)), len(rows))

    print("The recorded figures (rust/docs/experiments/): GF(2) alone, \"Matroid intersection bounds (2026-10)\"; "
          "the LP over all conflicts, \"How far can nogoods go (2026-10)\"; the per-frame bound at every 10th node "
          "(the root's multipliers to 20 steps of the node's own), \"The per-frame bound (2026-10)\".  "
          "Here: cuts of the nodes in the data.\n")
    table(["n", "GF(2), recorded", "GF(2), here", "all conflicts, recorded", "per-frame, recorded", "root mode, here",
           "node mode, here"],
          [[n, f"{ref['gf2']} of {REFERENCE_NODES}", gf2_here(n), f"{ref['conflicts']} of {REFERENCE_NODES}",
            f"{ref['per_frame']} of {REFERENCE_SAMPLE}", cuts_here(n, "root"), cuts_here(n, "node")]
           for n, ref in REFERENCE.items()])
    k3_rows = [r for r in rows_of.get((K3_FIXTURE, "node"), []) if gf2_open(r)]
    if k3_rows:
        k3 = sum(1 for r in k3_rows if cut(r))
        outcome = "killed by K3" if k3 / len(k3_rows) < K3_KILL else "passes K3"
        print(f"**K3**: node mode at {K3_FIXTURE} cuts {of(k3, len(k3_rows))} of the nodes GF(2) leaves open "
              f"(killed below {100 * K3_KILL:.0f}%): {outcome}\n")
    else:
        print(f"**K3**: no node-mode nodes of {K3_FIXTURE} that GF(2) leaves open.\n")
    out = []
    for n in sorted({n for n, _ in rows_of}):
        root = {r["node"]: r for r in rows_of.get((n, "root"), [])}
        node = {r["node"]: r for r in rows_of.get((n, "node"), [])}
        common = [k for k in root if k in node and gf2_open(root[k])]
        if not common:
            continue
        a, b = (sum(1 for k in common if cut(d[k])) for d in (root, node))
        out.append([n, len(common), a, b, fmt(ratio(a, b), 2)])
    if out:
        print("Root-mode cuts against node-mode cuts, among the open nodes both modes have:\n")
        table(["n", "open nodes", "root mode cuts", "node mode cuts", "ratio"], out)


# ---------------------------------------------------------------------------------------------------------
# Census

def section_census(census, path):
    print("## 8. Census of the cores\n")
    if not census:
        print(f"No cores in `{path.name}` (missing or empty): skipped.\n")
        return
    print(f"The cores the main arm found.  Square: as many columns as members; three primes: `primes` = 3; "
          f"det0 = 0: the plain 0/1 matrix is singular over GF(3), among the square cores; F1, R3: the plain signing, "
          f"the three random signings of arm f1r3 see the core; assigned: it went to a signing.\n")
    out = []
    for label, rs in by_band(census):
        square = [r for r in rs if r.get("square")]
        out.append([label, len(rs), fmt(mean(r.get("size") for r in rs), 1),
                    fmt(fraction(rs, lambda r: flag(r, "square"))),
                    fmt(fraction(rs, lambda r: flag(r, "primes", lambda p: p == 3))),
                    fmt(fraction(square, lambda r: flag(r, "det0", lambda d: d == 0))),
                    fmt(fraction(rs, lambda r: flag(r, "seen_f1"))), fmt(fraction(rs, lambda r: flag(r, "seen_r3"))),
                    fmt(fraction(rs, lambda r: "assigned_to" in r and r["assigned_to"] is not None))])
    table(["n", "cores", "mean size", "square", "three primes", "det0 = 0 among square", "seen by F1", "seen by R3",
           "assigned"], out)


# ---------------------------------------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    parser.add_argument("--roots", type=Path, default=HERE / "roots.jsonl",
                        help="the roots file (default: roots.jsonl next to the script)")
    parser.add_argument("--nodes", type=Path, default=HERE / "nodes.jsonl",
                        help="the nodes file (default: nodes.jsonl next to the script)")
    parser.add_argument("--census", type=Path, default=HERE / "census.jsonl",
                        help="the census file (default: census.jsonl next to the script)")
    parser.add_argument("--lite", action="store_true",
                        help=f"also print the E1-lite criterion: the mean share of {MAIN} over the converged roots "
                             f"with n >= {LITE_FROM} (early kill below {LITE_KILL} with all of them converged)")
    args = parser.parse_args()

    roots = load(args.roots, lambda r: (r.get("n"), r.get("arm")))
    nodes = load(args.nodes, lambda r: (r.get("n"), r.get("mode"), r.get("node")))
    census = load(args.census, lambda r: (r.get("n"), r.get("index")))
    print(f"# The fields study\n\nroots: `{args.roots.name}`, {len(roots or [])} records; nodes: `{args.nodes.name}`, "
          f"{len(nodes or [])}; census: `{args.census.name}`, {len(census or [])}.\n")
    if roots:
        roots_by_arm = {arm: sorted((r for r in roots if r["arm"] == arm), key=lambda r: r["n"])
                        for arm in arms_in(roots)}
        section_bands(roots_by_arm)
        section_criteria(roots_by_arm, args.lite)
        section_side_by_side(roots_by_arm)
        section_signings(roots_by_arm)
        section_support(roots_by_arm)
        section_cost(roots_by_arm)
    else:
        print(f"No roots in `{args.roots.name}` (missing or empty): sections 1-6 skipped.\n")
    section_nodes(nodes, args.nodes)
    section_census(census, args.census)


if __name__ == "__main__":
    main()
