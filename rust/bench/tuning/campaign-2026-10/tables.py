#!/usr/bin/env python3
"""Recomputes the tables of the tuning campaign of 2026-10 from its data files (README.md says what they are).

Standard library only; run it as `python3 -I bench/tuning/campaign-2026-10/tables.py` from rust/.  It prints what the
notebook entry quotes: stage A and stage B against the default (the geometric means of the CPU seconds and of the
nodes over the problems both ran, as the driver's report has them), stage B against `force_window=all`, stage B by the
source of the problems, and stage C per game.  The problems' sources are read from the suite (`../suite.jsonl`); the
`default` of the data is the default of the build the campaign ran on, `force_window=80`.
"""

import collections
import json
import math
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "out"
SUITE = HERE.parent / "suite.jsonl"
DEFAULT = "default"
STAGE_C = ("default", "fw_all", "all_sl4", "all_sl4_mfo")
STAGE_C_LINE = re.compile(
    r"^n=(\d+) T=\d+ DONE\s+search\s+([\d.]+) s nodes\s+(\d+) \(\s*(\d+) k/s\) best (\d+) opt (\d+).*? cpu ([\d.]+) s"
)


def read_results(path):
    """{(configuration, problem): the job's record} of the jobs that ended, and the rung each ran in."""
    done = {}
    for line in Path(path).read_text().splitlines():
        record = json.loads(line) if line.strip() else {}
        if record.get("ok"):
            done[(record["config"], record["problem"])] = record
    return done


def read_suite(path):
    """{id: line} of the suite's problems (units and whole games)."""
    lines = (json.loads(line) for line in Path(path).read_text().splitlines())
    return {row["id"]: row for row in lines if row.get("kind") in ("unit", "whole")}


def source(problem):
    """The game a unit is a subtree of, or `whole games`."""
    return "whole games" if problem["kind"] == "whole" else str(problem["game"])


def geometric_mean(xs):
    return math.exp(sum(math.log(x) for x in xs) / len(xs))


def compare(done, config, base, ids):
    """What `config` did against `base` on the problems of `ids` both ran, or None if there are none."""
    cpu, nodes, worst, cut, seconds, differs = [], [], (0.0, ""), 0, 0.0, []
    for i in ids:
        a, b = done.get((config, i)), done.get((base, i))
        if not a or not b or not b["result"]["completed"]:
            continue
        ra, rb = a["result"], b["result"]
        seconds += ra["cpu_s"]
        if not ra["completed"]:
            cut += 1
        elif ra["best"] != rb["best"]:
            differs.append(i)
        cpu.append(ra["cpu_s"] / rb["cpu_s"])
        nodes.append(ra["nodes"] / rb["nodes"])
        worst = max(worst, (cpu[-1], i))
    if not cpu:
        return None
    return {"n": len(cpu), "cpu": geometric_mean(cpu), "nodes": geometric_mean(nodes), "worst": worst, "cut": cut,
            "cpu_s": seconds, "differs": differs}


def print_table(done, configs, base, ids, title):
    print(title)
    head = f"{'configuration':<22} {'n':>3} {'cpu':>7} {'nodes':>7} {'worst':>6} {'worst problem':<14}"
    print(head + f" {'cut':>3} {'cpu s':>9}")
    rows = [(c, compare(done, c, base, ids)) for c in configs]
    for config, s in sorted((r for r in rows if r[1]), key=lambda r: r[1]["cpu"]):
        flag = "  BEST DIFFERS ON " + ",".join(s["differs"]) if s["differs"] else ""
        print(f"{config:<22} {s['n']:>3} {s['cpu']:>7.3f} {s['nodes']:>7.3f} {s['worst'][0]:>6.2f} {s['worst'][1]:<14} "
              f"{s['cut']:>3} {s['cpu_s']:>9.1f}{flag}")
    print()


def stage_a(suite):
    done = read_results(OUT / "stageA.jsonl")
    configs = sorted({c for c, _ in done})
    ran = collections.Counter(c for c, _ in done)
    print(f"STAGE A: {len(done)} jobs ended; problems run by each configuration: {sorted(set(ran.values()))}")
    print_table(done, configs, DEFAULT, sorted(suite), "stage A against the default, over the problems both ran")


def stage_b(suite):
    done = read_results(OUT / "stageB2.jsonl")
    ids = sorted(suite)
    configs = sorted({c for c, _ in done})
    ran = {c: {i for cc, i in done if cc == c} for c in configs}
    print_table(done, configs, DEFAULT, ids, "stage B against the default, over the problems both ran")

    common = sorted(ran["fw_all"] & ran["all_sl4"] & ran["all_more_forced_off"])
    print_table(done, ["fw_all", "all_sl4", "all_more_forced_off", "fw240"], DEFAULT, common,
                f"stage B against the default, the {len(common)} problems that force_window=all, strong_limit=4 and "
                "more_forced=off all ran")
    print("stage B against force_window=all, over the problems both ran")
    for config in ("all_sl4", "all_more_forced_off", "all_ng_p5k", "all_ng_p10k", "all_ng_p20k", "all_sl8",
                   "all_lagrange_on", "all_ng_p20k_sl8", "fw400", "fw240"):
        s = compare(done, config, "fw_all", sorted(ran[config] & ran["fw_all"]))
        print(f"{config:<22} {s['n']:>3} cpu {s['cpu']:.3f} nodes {s['nodes']:.3f} worst {s['worst'][0]:.2f} "
              f"{s['worst'][1]}")
    print()

    print("stage B by the source of the problem, against the default: cpu / nodes (problems)")
    by_source = collections.OrderedDict()
    for i in ids:
        by_source.setdefault(source(suite[i]), []).append(i)
    by_source["3425, 3440, 3441"] = [i for i in ids if source(suite[i]) in ("3425", "3440", "3441")]
    names = ("fw_all", "all_sl4", "all_more_forced_off")
    print(f"{'source':<18}" + "".join(f"{c:>26}" for c in names))
    for name, members in by_source.items():
        cells = []
        for config in names:
            s = compare(done, config, DEFAULT, members)
            cells.append(f"{s['cpu']:.3f} / {s['nodes']:.3f} ({s['n']})")
        print(f"{name:<18}" + "".join(f"{c:>26}" for c in cells))
    print()

    for config in ("fw_all", "all_sl4"):
        ratios = sorted((done[(config, i)]["result"]["cpu_s"] / done[(DEFAULT, i)]["result"]["cpu_s"], i)
                        for i in ids if (config, i) in done)
        print(f"{config}: the CPU ratio of a problem is from {ratios[0][0]:.3f} ({ratios[0][1]}) to "
              f"{ratios[-1][0]:.3f} ({ratios[-1][1]}) over {len(ratios)} problems; above 1: "
              f"{[(i, round(r, 3)) for r, i in ratios if r > 1]}")
    print()
    for config in ("fw_all", "all_sl4", "all_more_forced_off", DEFAULT):
        have = [i for i in ids if (config, i) in done]
        cpu = sum(done[(config, i)]["result"]["cpu_s"] for i in have)
        nodes = sum(done[(config, i)]["result"]["nodes"] for i in have)
        base_cpu = sum(done[(DEFAULT, i)]["result"]["cpu_s"] for i in have)
        base_nodes = sum(done[(DEFAULT, i)]["result"]["nodes"] for i in have)
        print(f"{config:<22} {len(have):>2} problems: {cpu:,.0f} CPU s ({cpu / base_cpu:.3f} of the default's "
              f"{base_cpu:,.0f}), {nodes:,} nodes ({nodes / base_nodes:.3f} of {base_nodes:,})")
    print()


def stage_c():
    runs = {}
    for name in STAGE_C:
        rows = {}
        for line in (OUT / f"stageC_{name}.txt").read_text().splitlines():
            m = STAGE_C_LINE.match(line)
            if m:
                n, seconds, nodes, rate, best, opt, cpu = m.groups()
                assert best == opt, line
                rows[int(n)] = {"s": float(seconds), "nodes": int(nodes), "rate": int(rate), "cpu": float(cpu)}
        runs[name] = rows
    games = list(runs[DEFAULT])
    print("STAGE C: search seconds, millions of nodes, thousands of nodes a second (64 threads)")
    print(f"{'game':<7}" + "".join(f"{name:>30}" for name in runs))
    for g in games:
        cells = (f"{r[g]['s']:>12.1f} {r[g]['nodes'] / 1e6:>8.1f} {r[g]['rate']:>6}" for r in runs.values())
        print(f"{g:<7}" + "".join(cells))
    totals = {name: (sum(r["s"] for r in rows.values()), sum(r["nodes"] for r in rows.values()))
              for name, rows in runs.items()}
    print(f"{'total':<7}" + "".join(f"{s:>12.0f} {n / 1e6:>8.1f} {n / s / 1e3:>6.0f}" for s, n in totals.values()))
    base_s, base_n = totals[DEFAULT]
    for name, rows in runs.items():
        s, n = totals[name]
        rate = (n / s) / (base_n / base_s)
        time_ratios = " ".join(f"{g}:{rows[g]['s'] / runs[DEFAULT][g]['s']:.2f}" for g in games)
        node_ratios = " ".join(f"{g}:{rows[g]['nodes'] / runs[DEFAULT][g]['nodes']:.2f}" for g in games)
        print(f"{name:<12} time {s / base_s:.3f}, nodes {n / base_n:.3f}, node rate {rate:.3f} of the default's; "
              f"per game, time {time_ratios}, nodes {node_ratios}")


def main():
    suite = read_suite(SUITE)
    stage_a(suite)
    stage_b(suite)
    stage_c()


if __name__ == "__main__":
    main()
