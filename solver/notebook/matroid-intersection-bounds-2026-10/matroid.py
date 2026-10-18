#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Matroid intersection bounds: the tables of solver/notebook/matroid-intersection-bounds-2026-10/README.md.

The data files here are the output of `examples/matroid.rs` (the commands are in that entry); this script joins
them with the slope study's ../curve/slope_roots.jsonl (the matching and Lagrangian bounds of every root) and
../curve/slope_proofs.jsonl (the solver's one-thread proofs) and prints markdown.  Pure Python;
`./matroid.py > matroid_output.md`.

The example `matroid` was removed from master (2026-10-17); it builds at commit 8753189 (`git checkout 8753189`, then
`cargo build --release --example matroid` in rust/).
"""
import json
import math
import statistics as st
from pathlib import Path

HERE = Path(__file__).resolve().parent
CURVE = HERE.parent / "what-drives-the-slope-2026-10"
BANDS = [(851, 900), (901, 950), (951, 1000), (1001, 1050), (1051, 1100), (1101, 1150), (1151, 1200), (1201, 1250),
         (1251, 1300), (1301, 1310), (1350, 1352)]


def load(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.startswith("{")]


def fit(xs, ys):
    """Least squares y = a + b x: (b, standard error of b, R^2)."""
    mx, my = st.mean(xs), st.mean(ys)
    sxx = sum((x - mx) ** 2 for x in xs)
    b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sxx
    res = [y - my - b * (x - mx) for x, y in zip(xs, ys)]
    se = math.sqrt(sum(r * r for r in res) / (len(xs) - 2) / sxx)
    r2 = 1 - sum(r * r for r in res) / sum((y - my) ** 2 for y in ys)
    return b, se, r2


def in_band(rows, lo, hi):
    return [r for r in rows if lo <= r["n"] <= hi]


def roots_table():
    gf2 = {r["n"]: r for r in load(HERE / "roots.jsonl")}
    slope = {r["n"]: r for r in load(CURVE / "slope_roots.jsonl")}
    rows = []
    for n, g in sorted(gf2.items()):
        s = slope[n]
        assert s["opt"] == g["opt"] and s["matching"]["bound"] == g["matching"], n
        assert g["gf2"] >= g["opt"] and g["gf2"] <= g["matching"], n
        rows.append(dict(n=n, m=g["matching"] - g["opt"], lag=s["lagrange"]["bound"] - g["opt"], g=g["gf2"] - g["opt"],
                         free=g["gf2_uncapped"] - g["opt"], ms=1000 * (g["build_s"] + g["solve_s"])))
    print("## 1. Gaps at the root\n")
    print("| n | games | matching gap | Lagrangian gap | GF(2) gap | GF(2) / matching | GF(2) / Lagrangian | ms |")
    print("|---|---|---|---|---|---|---|---|")
    for lo, hi in BANDS:
        b = in_band(rows, lo, hi)
        m, lg, g = (st.mean(r[k] for r in b) for k in ("m", "lag", "g"))
        print(f"| {lo}-{hi} | {len(b)} | {m:.0f} | {lg:.0f} | {g:.0f} | {g / m:.2f} | {g / lg:.2f} | "
              f"{st.mean(r['ms'] for r in b):.2f} |")
    upto = [r for r in rows if r["n"] <= 1300]
    print(f"\nThe cap `max_total` binds at {sum(1 for r in rows if r['free'] != r['g'])} roots.  "
          f"Growth per 100 in n over the {len(upto)} games of 880-1300:\n")
    for key, name in (("m", "matching"), ("lag", "Lagrangian"), ("g", "GF(2)")):
        b, se, r2 = fit([r["n"] for r in upto], [r[key] for r in upto])
        mean = st.mean(r[key] for r in upto)
        print(f"* {name} gap: {100 * b:+.1f} ± {100 * se:.1f} (R² {r2:.2f}), mean {mean:.0f}")
    for key, name in (("m", "matching"), ("lag", "Lagrangian")):
        share = [1 - r["g"] / r[key] for r in upto]
        b, se, _ = fit([r["n"] for r in upto], share)
        print(f"* share of the {name} gap the GF(2) bound closes: mean {st.mean(share):.3f}, "
              f"{100 * b:+.4f} ± {100 * se:.4f} per 100 in n")
    return {r["n"]: r for r in rows}


def bnb_table(roots):
    bnb = {r["n"]: r for r in load(HERE / "bnb.jsonl")}
    proofs = {r["n"]: r for r in load(CURVE / "slope_proofs.jsonl") if r["delta"] == 0 and r["completed"]}
    rows = []
    for n, b in sorted(bnb.items()):
        assert b["completed"] and b["better"] == 0, n
        p = proofs[n]
        rows.append(dict(n=n, gn=b["nodes"], gs=b["seconds"], sn=p["nodes"], ss=p["seconds"], **{
            k: roots[n][k] for k in ("m", "lag", "g")}))
    print("\n## 2. A plain branch and bound with the GF(2) bound against the solver's proofs\n")
    print("| n | games | log10 nodes, solver | log10 nodes, GF(2) B&B | ratio (geometric) | s, solver | s, GF(2) B&B |")
    print("|---|---|---|---|---|---|---|")
    for lo, hi in BANDS:
        b = in_band(rows, lo, hi)
        if not b:
            continue
        ls, lg = (st.mean(math.log10(r[k]) for r in b) for k in ("sn", "gn"))
        print(f"| {lo}-{hi} | {len(b)} | {ls:.2f} | {lg:.2f} | {10 ** (ls - lg):.0f} | {sum(r['ss'] for r in b):.1f} | "
              f"{sum(r['gs'] for r in b):.1f} |")
    print(f"\n{len(rows)} games; total seconds: solver {sum(r['ss'] for r in rows):.0f}, "
          f"GF(2) B&B {sum(r['gs'] for r in rows):.0f}.\n")
    for key, name in (("sn", "the solver"), ("gn", "the GF(2) B&B")):
        b, se, r2 = fit([r["n"] for r in rows], [math.log10(r[key]) for r in rows])
        print(f"* log10 nodes of {name} per 100 in n: {100 * b:.3f} ± {100 * se:.3f} (R² {r2:.2f})")
    for key, name in (("g", "GF(2)"), ("lag", "Lagrangian"), ("m", "matching")):
        b, se, r2 = fit([r[key] for r in rows], [math.log10(r["gn"]) for r in rows])
        print(f"* log10 nodes of the GF(2) B&B against the root's {name} gap: {b:.5f} ± {se:.5f} per unit "
              f"(R² {r2:.2f})")
    big = sorted(rows, key=lambda r: -r["gn"])[:6]
    print("* largest GF(2) B&B trees (n, nodes, solver's nodes, GF(2) gap): "
          + ", ".join(f"{r['n']} {r['gn']} {r['sn']} {r['g']}" for r in big))
    for r in load(HERE / "bnb_matching.jsonl"):
        print(f"* the same branch and bound with the matching bound, n = {r['n']}: "
              f"{'done' if r['completed'] else 'unfinished'} after {r['nodes']} nodes, {r['seconds']:.1f} s")


def nodes_table():
    print("\n## 3. At the probe's search nodes\n")
    rows = load(HERE / "nodes.jsonl")
    lag = load(HERE / "nodes_lagrange.jsonl")
    exact = [r for r in load(HERE / "nodes_exact.jsonl") if r["exact"] is not None]
    forced = load(HERE / "forced.jsonl")
    print("| n | nodes | matching cuts | GF(2) cuts | GF(2) ms | of every 3rd: node-fitted Lagrangian cuts | "
          "either | true gap, matching | true gap, GF(2) | forced exactly | by the split |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for n in sorted({r["n"] for r in rows}):
        a = [r for r in rows if r["n"] == n]
        lg = [r for r in lag if r["n"] == n]
        ex = [r for r in exact if r["n"] == n]
        fo = [r for r in forced if r["n"] == n and r["value"] >= r["need"]]

        def cuts(rs, key):
            return sum(1 for r in rs if r[key] < r["target"] + 1)

        either = sum(1 for r in lg if min(r["gf2"], r["lagrange"]) < r["target"] + 1)
        assert all(r["gf2"] >= r["exact"] for r in ex), n
        tm = st.mean(r["matching"] - r["exact"] for r in ex)
        tg = st.mean(r["gf2"] - r["exact"] for r in ex)
        fe = sum(r["forced_exact"] for r in fo)
        fs = sum(r["forced_split"] for r in fo)
        print(f"| {n} | {len(a)} | {cuts(a, 'matching')} | {cuts(a, 'gf2')} | "
              f"{1000 * st.mean(r['seconds'] for r in a):.2f} | {cuts(lg, 'lagrange')} of {len(lg)} "
              f"(GF(2): {cuts(lg, 'gf2')}) | {either} | {tm:.0f} | {tg:.0f} ({len(ex)} nodes) | "
              f"{fe / len(fo):.1f} a node | {100 * fs / fe:.0f}% |")


def lagrange_and_cores():
    print("\n## 4. What the GF(2) bound misses\n")
    for r in load(HERE / "lagrange.jsonl"):
        print(f"* n = {r['n']}: GF(2) gap {r['gf2'] - r['opt']}, with the Lagrangian over {r['nogoods']} nogoods "
              f"{r['lagrange_gf2'] - r['opt']}; the GF(2) optimum's conflicts have "
              f"{min(r['first_conflicts'])}-{max(r['first_conflicts'])} members")
    for r in load(HERE / "cores.jsonl"):
        cores = "; ".join(f"frame {c['frame']}: {c['movers']} movers ({c['promoted']} promoted, {c['wide']} with 3+ "
                          f"factors) over {c['factors']} factors, Lorea {'sees' if c['lorea'] else 'misses'} it"
                          for c in r["cores"])
        print(f"* n = {r['n']}: {cores}")


def whole_table(roots):
    rows = load(HERE / "whole.jsonl")
    print("\n## 5. The model-free bound on opt(n)\n")
    print("| n | games | gap mean | median | max | exact | ms |")
    print("|---|---|---|---|---|---|---|")
    for lo in range(0, 1300, 100):
        b = [r for r in rows if lo < r["n"] <= lo + 100]
        gaps = [r["gap"] for r in b]
        print(f"| {max(lo + 1, 2)}-{lo + 100} | {len(b)} | {st.mean(gaps):.1f} | {st.median(gaps):.0f} | {max(gaps)} | "
              f"{sum(1 for g in gaps if g == 0)} | {1000 * st.mean(r['seconds'] for r in b):.0f} |")
    same = [r for r in rows if r["n"] in roots]
    equal = sum(1 for r in same if r["gap"] == roots[r["n"]]["g"])
    print(f"\nThe model-free gap equals the frame model's GF(2) gap at {equal} of the {len(same)} games both have.  "
          "Its growth per 100 in n:\n")
    for lo, hi in ((2, 1300), (300, 1000), (600, 1300), (880, 1300)):
        b = [r for r in rows if lo <= r["n"] <= hi]
        slope, se, r2 = fit([r["n"] for r in b], [r["gap"] for r in b])
        print(f"* {lo}-{hi}: {100 * slope:+.1f} ± {100 * se:.1f} (R² {r2:.2f})")


def main():
    roots = roots_table()
    bnb_table(roots)
    nodes_table()
    lagrange_and_cores()
    whole_table(roots)


if __name__ == "__main__":
    main()
