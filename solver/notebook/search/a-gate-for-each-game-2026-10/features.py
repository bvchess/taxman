#!/usr/bin/env python3
"""Per unit of the gate re-check: its features (gap, depth, candidates, sum) and its steps at each gate."""
import json
import math
import os

D = "/home/brian/taxman_work/lab/work/gate-2026-10-28"
GAMES = ["5052", "4995", "4992", "4984"]
SETS = ["g1200", "g1000", "g800", "g600", "g400"]


def load(game):
    units = {}
    for line in open(f"{D}/{game}/units-{game}.jsonl"):
        r = json.loads(line)
        if r["kind"] == "unit":
            units[r["id"]] = r
    ids = open(f"{D}/{game}/sample.txt").read().split()
    rows = []
    for i in ids:
        u = units[i]
        res = {}
        for s in SETS:
            p = f"{D}/{game}/out/{s}-{i}.json"
            if os.path.exists(p):
                j = json.load(open(p))
                res[s] = (j["nodes"], j["cpu_s"], j["completed"])
        rows.append((u, res))
    return rows


def main():
    for game in GAMES:
        print(f"== {game}")
        print(f"{'id':10} {'gap':>5} {'dep':>4} {'cands':>5} {'sum':>7} {'need':>7} {'s1200':>7} {'s600':>7} "
              f"{'r600':>6} {'r800':>6}")
        for u, res in sorted(load(game), key=lambda x: -x[0]["gap"]):
            b = res["g1200"][0]
            if b < 1000:
                continue
            r6 = res["g600"][0] / b
            r8 = res["g800"][0] / b
            cap = "*" if not res["g1200"][2] else ""
            print(f"{u['id']:10} {u['gap']:5} {u['depth']:4} {u['n_cands']:5} {u['sum']:7} {u['need']:7} {b:7}{cap:1}"
                  f"{res['g600'][0]:7} {r6:6.2f} {r8:6.2f}")


if __name__ == "__main__":
    main()
