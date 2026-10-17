#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The standalone searches of the sample (sa/N.jsonl: `taxman solve N --threads 1 --time-limit 1800`) against the
chain's own search, and the map of 3441..8191 (map.jsonl.gz: `wall landscape 3441-8191`) by band of n.

Reads cal.jsonl, sa/*.jsonl, map.jsonl.gz and ../../results/strict.jsonl.
usage: ./map.py
"""
import statistics as st

from land import CHAIN, HERE, by_n, lines

chain = by_n(CHAIN)
cal = by_n(HERE / "cal.jsonl")

# 1. The standalone searches: the last line of each file is the search's result.
print("standalone searches of the sample (one thread, at most 1800 s):")
rows = []
for path in sorted((HERE / "sa").glob("*.jsonl")):
    d = lines(path)[-1]
    n, done = d["n"], bool(d.get("completed"))
    best = d["score"] if done else d["best_so_far"]
    rows.append(dict(n=n, done=done, seconds=d["seconds"]["search"], nodes=d["nodes"], chain_nodes=chain[n]["nodes"],
                     chain_proof=chain[n]["proof"], gap=cal[n]["gap"], shortfall=cal[n]["shortfall"],
                     below=chain[n]["score"] - best))
done = [r for r in rows if r["done"]]
print(f"  {len(rows)} games, {len(done)} finished; best game below the chain's optimum: "
      f"{sum(r['below'] > 0 for r in done)} of the finished, {sum(r['below'] > 0 for r in rows if not r['done'])} "
      f"of the {len(rows) - len(done)} stopped at the limit")
print(f"  {'n':>5} {'done':>5} {'standalone s':>12} {'nodes':>11} {'chain nodes':>11} {'chain proof':>13} {'gap':>5} "
      f"{'short':>5} {'below opt':>9}")
for r in sorted(rows, key=lambda r: r["seconds"]):
    print(f"  {r['n']:>5} {r['done']!s:>5} {r['seconds']:12.1f} {r['nodes']:>11} {r['chain_nodes']:>11} "
          f"{r['chain_proof']:>13} {r['gap']:>5} {r['shortfall']:>5} {r['below']:>9}")
print("  stopped at the limit, by gap: " + ", ".join(f"{r['gap']} ({r['n']})" for r in
                                                   sorted(rows, key=lambda r: r["gap"]) if not r["done"]))
near = sorted((r for r in rows if 1100 <= r["gap"] <= 1300), key=lambda r: r["seconds"])
finished = ", ".join(f"{r['seconds']:.0f} s ({r['n']}, gap {r['gap']})" for r in near if r["done"])
stopped = ", ".join(f"{r['n']} (gap {r['gap']})" for r in near if not r["done"])
print(f"  gaps 1,100 to 1,300: {len(near)} games; finished in {finished}; not finished in 1800 s: {stopped}")

print("  by standalone gap (finished: seconds, median and longest):")
for lo, hi in ((0, 700), (700, 1000), (1000, 1300), (1300, 1600), (1600, None)):
    band = [r for r in rows if lo <= r["gap"] and (hi is None or r["gap"] < hi)]
    secs = [r["seconds"] for r in band if r["done"]]
    label = f"{lo}+" if hi is None else f"{lo}-{hi - 1}"
    print(f"    gap {label:<9} {len(band):>3} games, {len(secs):>3} finished" +
          (f", median {st.median(secs):6.1f} s, longest {max(secs):6.1f} s" if secs else ""))


# 2. The bands of n: the standalone gap, and the medians of the CPU seconds of a game's bounds (`cpu_s`) with the
# parts of it: the setup (`setup_s`), the start incumbent (`start_s`) and the root's bound (`bound_s`).
def bands(games, first, last):
    print(f"  {'band':>11} {'games':>5} {'gap p10':>8} {'median':>7} {'p90':>7} {'gap<700':>8} {'gap<1000':>9} "
          f"{'cpu s':>7} {'setup s':>8} {'start s':>8} {'bound s':>8}")
    for lo in range(first, last + 1, 500):
        band = [games[n] for n in range(lo, min(lo + 499, last) + 1)]
        gaps = sorted(d["gap"] for d in band)
        times = "".join(f" {st.median(d[key] for d in band):{width}.1f}" for key, width in
                        (("cpu_s", 7), ("setup_s", 8), ("start_s", 8), ("bound_s", 8)))
        print(f"  {lo:>5}-{band[-1]['n']:<5} {len(band):>5} {gaps[len(gaps) // 10]:>8} {st.median(gaps):>7.0f} "
              f"{gaps[9 * len(gaps) // 10]:>7} {sum(g < 700 for g in gaps):>8} {sum(g < 1000 for g in gaps):>9}{times}")


print("\ncalibration, 2176..3440 (for comparison):")
bands(cal, 2176, 3440)
mp = by_n(HERE / "map.jsonl.gz")
print(f"\nmap: {len(mp)} games 3441..8191; met at once: {sum(d['meets'] for d in mp.values())}; "
      f"{sum(d['cpu_s'] for d in mp.values()) / 3600:.1f} core-hours")
print("  smallest standalone gaps: " + ", ".join(f"{d['gap']} ({d['n']})" for d in
                                                 sorted(mp.values(), key=lambda d: d["gap"])[:6]))
print("  games of standalone gap under " + ", ".join(
    f"{limit}: {sum(d['gap'] < limit for d in mp.values())}" for limit in (1300, 1500, 2000, 3000)))
bands(mp, 3441, 8191)
