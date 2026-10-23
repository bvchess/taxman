#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The tables of the finders experiment: stage 1 (what a better start saves a sound search), stage 2 (the finders
against the control) and stage 3 (a sound search seeded with a finder's game).

Reads stage1.jsonl, stage2.jsonl and stage3-*.jsonl in this directory, and ../landscape/cal.jsonl.
usage: ./tables.py > tables.txt
"""
import statistics as st
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LANDSCAPE = HERE.parent / "the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10"
sys.path.insert(0, str(LANDSCAPE))
from land import by_n, lines, quantile  # noqa: E402

METHODS = ("start", "lns", "restricted", "control")
BANDS = ((2176, 2425), (2426, 2675), (2676, 2925), (2926, 3175), (3176, 3440))


def mean(xs):
    return sum(xs) / len(xs)


def spread(xs):
    """median, mean, p10, p90, smallest and largest of a list."""
    return (f"median {st.median(xs):.3f}, mean {mean(xs):.3f}, p10 {quantile(xs, .1):.3f}, p90 {quantile(xs, .9):.3f}, "
            f"range {min(xs):.3f}-{max(xs):.3f}")


# ---------------------------------------------------------------------------------------------------------
# Stage 1: the sound search from today's start (i) and from the known optimum (ii)

def stage1():
    rows = {}
    for d in lines(HERE / "stage1.jsonl"):
        rows.setdefault(d["n"], {})[d["from"]] = d
    games = {n: r for n, r in rows.items() if "start" in r and "opt" in r}
    print(f"STAGE 1: {len(games)} games with both searches; {len(rows) - len(games)} with only one")
    print(f"  {'n':>5} {'short':>5} | {'(i) nodes':>10} {'cpu s':>7} {'done':>4} | {'(ii) nodes':>10} {'cpu s':>7} "
          f"{'done':>4} | {'nodes':>6} {'cpu':>6}")
    both = []
    for n, r in sorted(games.items()):
        i, ii = r["start"], r["opt"]
        short = i["opt"] - i["start"]
        done = i["search_completed"] and ii["search_completed"]
        ratio = (ii["search_nodes"] / i["search_nodes"], ii["search_cpu_s"] / i["search_cpu_s"]) if done else None
        both.append(dict(n=n, short=short, i=i, ii=ii, ratio=ratio))
        print(f"  {n:>5} {short:>5} | {i['search_nodes']:>10} {i['search_cpu_s']:>7.1f} {str(i['search_completed'])[0]:>4} | "
              f"{ii['search_nodes']:>10} {ii['search_cpu_s']:>7.1f} {str(ii['search_completed'])[0]:>4} | "
              + (f"{ratio[0]:>6.3f} {ratio[1]:>6.3f}" if ratio else f"{'-':>6} {'-':>6}"))
    done = [b for b in both if b["ratio"]]
    print(f"\n  both searches finished: {len(done)} of {len(both)} games; (i) took {sum(b['i']['search_cpu_s'] for b in done):.0f} "
          f"CPU s, (ii) {sum(b['ii']['search_cpu_s'] for b in done):.0f}")
    for label, subset in (("all", done), ("start short of the optimum", [b for b in done if b["short"] > 0]),
                          ("start is the optimum", [b for b in done if b["short"] == 0]),
                          ("(i) took 300 CPU s or less", [b for b in done if b["i"]["search_cpu_s"] <= 300]),
                          ("(i) took more than 300 CPU s", [b for b in done if b["i"]["search_cpu_s"] > 300])):
        if not subset:
            continue
        print(f"  {label} ({len(subset)} games):")
        print(f"    nodes (ii)/(i): {spread([b['ratio'][0] for b in subset])}")
        print(f"    CPU   (ii)/(i): {spread([b['ratio'][1] for b in subset])}")
        print(f"    in total: nodes {sum(b['ii']['search_nodes'] for b in subset) / sum(b['i']['search_nodes'] for b in subset):.3f}, "
              f"CPU {sum(b['ii']['search_cpu_s'] for b in subset) / sum(b['i']['search_cpu_s'] for b in subset):.3f}")
    print("  by the start's shortfall (nodes (ii)/(i), games that finished):")
    for lo, hi in ((0, 0), (1, 150), (151, 300), (301, 500), (501, 10 ** 6)):
        subset = [b["ratio"][0] for b in done if lo <= b["short"] <= hi]
        if subset:
            print(f"    {lo}-{hi if hi < 10 ** 6 else '':<6} {len(subset):>3} games: median {st.median(subset):.3f}, "
                  f"mean {mean(subset):.3f}, range {min(subset):.3f}-{max(subset):.3f}")
    return {b["n"]: b for b in both}


# ---------------------------------------------------------------------------------------------------------
# Stage 2: the finders and the control, from the same start

def stage2(name):
    cal = by_n(LANDSCAPE / "cal.jsonl")
    rows = [d for d in lines(HERE / name) if not d.get("trivial")]
    searched = [d for d in rows if not d["meets"]]
    print(f"\nSTAGE 2 ({name}): {len(rows)} games, {len(rows) - len(searched)} whose start meets the root bound (nothing to find), "
          f"{len(searched)} searched")
    for d in searched:
        # Repeats and primes need no search of their own (the chain settles them from the game before).
        d["plain"] = not cal[d["n"]]["prime"] and cal[d["n"]]["opt_proof"] != "same_search"
        d["short"] = {m: d["opt"] - d[m] for m in METHODS}
        d["cpu"] = {"start": d["start_cpu_s"], "lns": d["lns_cpu_s"], "control": d["control_cpu_s"],
                    "restricted": d["restricted_cpu_s"]}

    def table(label, subset):
        if not subset:
            return
        print(f"\n  {label}: {len(subset)} games; the shortfall (the optimum less the game's score)")
        print(f"    {'':<11} {'median':>7} {'mean':>7} {'p90':>6} {'exact':>6} {'<control':>8} {'>control':>8} "
              f"{'cpu s':>6} {'+neighbors':>11}")
        for m in METHODS:
            short = [d["short"][m] for d in subset]
            exact = sum(s == 0 for s in short) / len(short)
            less = sum(d["short"][m] < d["short"]["control"] for d in subset) / len(subset)
            more = sum(d["short"][m] > d["short"]["control"] for d in subset) / len(subset)
            cpu = mean([d["cpu"][m] for d in subset])
            extra = mean([d["neighbours_cpu_s"] for d in subset]) if m == "restricted" else 0
            print(f"    {m:<11} {st.median(short):>7.0f} {mean(short):>7.1f} {quantile(short, .9):>6} {exact:>6.1%} "
                  f"{less:>8.1%} {more:>8.1%} {cpu:>6.2f} {extra:>11.2f}")

    table("all searched games", searched)
    table("games that need a search of their own (no primes or repeats)", [d for d in searched if d["plain"]])
    for lo, hi in BANDS:
        table(f"n {lo}-{hi}", [d for d in searched if lo <= d["n"] <= hi])
    # Where the methods agree with each other.
    best = {m: sum(d["short"][m] == min(d["short"].values()) for d in searched) for m in METHODS}
    print("\n  the best of the four (ties count for each): " + ", ".join(f"{m} {best[m]}" for m in METHODS))
    both = sum(d["short"]["lns"] == 0 and d["short"]["restricted"] == 0 for d in searched)
    print(f"  games both finders have exactly: {both}; neither: {sum(d['short']['lns'] > 0 and d['short']['restricted'] > 0 for d in searched)}")
    print(f"  CPU of a game: start {mean([d['start_cpu_s'] for d in searched]):.2f} s, neighbors' starts "
          f"{mean([d['neighbours_cpu_s'] for d in searched]):.2f} s, each method "
          f"{mean([d['lns_cpu_s'] for d in searched]):.2f} s; the run in all "
          f"{sum(d['start_cpu_s'] + d['neighbours_cpu_s'] + d['lns_cpu_s'] + d['restricted_cpu_s'] + d['control_cpu_s'] for d in searched) / 3600:.2f} core-hours")
    lns = [d for d in searched if d["lns_units"]]
    print(f"  lns: {mean([d['lns_units'] for d in lns]):.0f} units a game, {mean([d['lns_finished'] / d['lns_units'] for d in lns]):.1%} "
          f"of them searched to the end, {mean([d['lns_improved'] for d in lns]):.1f} improvements a game")
    print(f"  restricted: {mean([d['restricted_rounds'] for d in searched]):.1f} rounds a game, ended with "
          f"{mean([d['restricted_candidates'] / d['candidates'] for d in searched]):.1%} of the candidates; "
          f"finished the whole search in {sum(d['restricted_completed'] for d in searched)} games, the control in "
          f"{sum(d['control_completed'] for d in searched)}")
    return searched


# ---------------------------------------------------------------------------------------------------------
# Stage 3: a sound search seeded with a finder's game, against (i) and (ii)

def stage3(stage1_rows):
    for path in sorted(HERE.glob("stage3-*.jsonl")):
        finder = path.stem.split("-")[1]
        rows = [d for d in lines(path) if d["n"] in stage1_rows]
        print(f"\nSTAGE 3, seeded with {finder} ({path.name}): {len(rows)} games")
        print(f"  {'n':>5} {'short':>5} {'seed':>5} | {'nodes':>9} {'(i)':>9} {'(ii)':>9} | {'cpu in all':>10} {'(i)':>7} "
              f"{'(ii)':>7}")
        kept = []
        for d in sorted(rows, key=lambda d: d["n"]):
            b = stage1_rows[d["n"]]
            i, ii = b["i"], b["ii"]
            short, seed = d["opt"] - d["start"], d["opt"] - d["search_from"]
            # Everything the seeded search costs: the start, the finder, the search.  (i) is the start and the search;
            # (ii) is the search alone (the optimum is given).
            seeded = d["start_cpu_s"] + d[finder + "_cpu_s"] + d["search_cpu_s"]
            standalone = i["start_cpu_s"] + i["search_cpu_s"]
            print(f"  {d['n']:>5} {short:>5} {seed:>5} | {d['search_nodes']:>9} {i['search_nodes']:>9} {ii['search_nodes']:>9} "
                  f"| {seeded:>10.1f} {standalone:>7.1f} {ii['search_cpu_s']:>7.1f}"
                  + ("" if d["search_completed"] else "  (not finished)"))
            if d["search_completed"] and i["search_completed"]:
                kept.append((d["search_nodes"] / i["search_nodes"], seeded / standalone, seed, short, seeded, standalone, d, i, ii))
        if kept:
            print(f"  against (i), {len(kept)} games that finished: nodes {spread([k[0] for k in kept])}")
            print(f"    CPU in all, finder and start counted: {spread([k[1] for k in kept])}")
            print(f"    the seed's shortfall {mean([k[2] for k in kept]):.0f} on the mean (the start's {mean([k[3] for k in kept]):.0f})")
            nodes = sum(k[6]["search_nodes"] for k in kept)
            print(f"    in total: nodes {nodes / sum(k[7]['search_nodes'] for k in kept):.3f} of (i)'s and "
                  f"{nodes / sum(k[8]['search_nodes'] for k in kept):.3f} of (ii)'s; CPU in all {sum(k[4] for k in kept):.0f} s "
                  f"against (i)'s {sum(k[5] for k in kept):.0f} s and (ii)'s search alone {sum(k[8]['search_cpu_s'] for k in kept):.0f} s "
                  f"(the finder {sum(k[6][finder + '_cpu_s'] for k in kept):.0f} s of it); "
                  f"{sum(k[4] < k[5] for k in kept)} of {len(kept)} games cost less in all")


# ---------------------------------------------------------------------------------------------------------
# The light tuning, on 26 games that stage 2 does not play

def tuning():
    print("\nTUNING: 26 games n = 2178 + 50k, 5 CPU s each; the shortfall of the finder's game (the optimum less its score)")
    for path in sorted((HERE / "tuning").glob("*.jsonl")):
        rows = [d for d in lines(path) if d.get("meets") is False]
        method = path.stem.split("-")[0]
        short = [d["opt"] - d[method] for d in rows]
        print(f"  {path.stem:<30} {len(rows):>3} games: mean {mean(short):6.1f}, median {st.median(short):4.0f}, "
              f"exact {sum(s == 0 for s in short) / len(short):5.1%}")


def budgets():
    """The 5 s and the 20 s runs on the games both have: the shortfall of each method."""
    short = {name: {d["n"]: d for d in lines(HERE / name) if not d.get("trivial") and not d["meets"]}
             for name in ("stage2.jsonl", "stage2-20s.jsonl")}
    common = sorted(set(short["stage2.jsonl"]) & set(short["stage2-20s.jsonl"]))
    print(f"\nBUDGETS: the {len(common)} games both runs have; the shortfall, mean (exact)")
    for m in METHODS[1:]:
        cells = []
        for name in ("stage2.jsonl", "stage2-20s.jsonl"):
            xs = [short[name][n]["opt"] - short[name][n][m] for n in common]
            cells.append(f"{mean(xs):6.1f} ({sum(x == 0 for x in xs) / len(xs):5.1%})")
        print(f"  {m:<11} 5 s {cells[0]}, 20 s {cells[1]}")


def cpu_spent():
    """The CPU each file cost, from the lines' own CPU fields (the neighbors' starts of a tuning line of `restricted`
    are not in it)."""
    fields = ("start_cpu_s", "neighbours_cpu_s", "lns_cpu_s", "restricted_cpu_s", "control_cpu_s", "search_cpu_s")
    print("\nCPU SPENT (core-hours, from the CPU fields of the lines)")
    total = 0
    for path in sorted(HERE.glob("*.jsonl")) + sorted((HERE / "tuning").glob("*.jsonl")):
        cpu = sum(d.get(f, 0) for d in lines(path) for f in fields)
        total += cpu
        print(f"  {str(path.relative_to(HERE)):<34} {cpu / 3600:5.2f}")
    print(f"  {'in all':<34} {total / 3600:5.2f}")


def finder_time():
    """When the LNS found its last improvement, in the files that have it (stage 3)."""
    for path in sorted(HERE.glob("stage3-lns*.jsonl")):
        found = [d["lns_found_ms"] / 1000 for d in lines(path) if "lns_found_ms" in d]
        if found:
            print(f"\nLNS: CPU seconds of its last improvement, {len(found)} games of {path.name}: median {st.median(found):.2f}, "
                  f"p90 {quantile(found, .9):.2f}, largest {max(found):.2f}; {sum(f <= 1 for f in found)} within 1 s, "
                  f"{sum(f <= 2.5 for f in found)} within 2.5 s")


if __name__ == "__main__":
    first = stage1()
    for name in ("stage2.jsonl", "stage2-20s.jsonl"):
        if (HERE / name).exists():
            stage2(name)
    stage3(first)
    budgets()
    finder_time()
    cpu_spent()
    tuning()
