#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Calibration of the standalone bounds on 2176..3440 against the committed chain: how far the heuristic falls short
of the optimum, and how well each gap predicts the nodes of the chain's search.

Reads cal.jsonl (`wall landscape 2176-3440 --known results/strict.jsonl`) and ../../results/strict.jsonl.
usage: ./cal.py
"""
import math
import statistics as st

from land import CHAIN, HERE, by_n, quantile

cal = by_n(HERE / "cal.jsonl")
chain = by_n(CHAIN)
ns = sorted(cal)

short = [cal[n]["shortfall"] for n in ns if cal[n]["shortfall"] is not None]
rg = [cal[n]["root_gap"] for n in ns if cal[n]["root_gap"] is not None]
gap = [cal[n]["gap"] for n in ns]
cpu = [cal[n]["cpu_s"] for n in ns]
print(f"{len(ns)} games; meets (proved at once): {sum(cal[n]['meets'] for n in ns)}; "
      f"shortfall 0: {sum(1 for s in short if s == 0)}")
for name, xs in (("shortfall", short), ("true root gap", rg), ("standalone gap", gap)):
    print(f"  {name:15} median {st.median(xs):7.0f}  p10 {quantile(xs, .1):6}  p90 {quantile(xs, .9):6}  "
          f"mean {st.mean(xs):7.1f}")
print(f"  cpu per game: median {st.median(cpu):.2f} s, max {max(cpu):.2f} s, all {sum(cpu) / 3600:.2f} core-hours")


print(f"  {'band':>11} {'games':>5} {'shortfall':>10} {'root gap':>9} {'standalone gap':>15}   (medians)")
for lo in range(2176, 3441, 500):
    band = [cal[n] for n in range(lo, min(lo + 499, 3440) + 1)]
    print(f"  {lo:>5}-{band[-1]['n']:<5} {len(band):>5} {st.median(d['shortfall'] for d in band):>10.0f} "
          f"{st.median(d['root_gap'] for d in band):>9.0f} {st.median(d['gap'] for d in band):>15.0f}")


def fit(xs, ys):
    """Least squares of ys on xs: the slope, the correlation and the residuals' standard deviation."""
    mx, my = st.mean(xs), st.mean(ys)
    b = sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / sum((x - mx) ** 2 for x in xs)
    a = my - b * mx
    return b, st.correlation(xs, ys), st.pstdev([y - (a + b * x) for x, y in zip(xs, ys)])


ex = [n for n in ns if n in chain and chain[n].get("proof") == "exhausted" and chain[n]["nodes"] > 0]
y = [math.log10(chain[n]["nodes"]) for n in ex]
print(f"the committed chain: {len(ex)} exhausted searches")
for name, key in (("true root gap", "root_gap"), ("standalone gap", "gap")):
    b, r, sd = fit([cal[n][key] for n in ex], y)
    print(f"  log10 nodes vs {name:15}: a decade per {1 / b:6.0f} of gap, r = {r:.2f}, residual sd {sd:.2f} decades")
