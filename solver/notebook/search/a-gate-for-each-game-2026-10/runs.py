#!/usr/bin/env python3
"""Every solve of this study, a line each: the folder, the setting, the unit, its gap, the steps, the CPU seconds,
whether it finished, and its best.  usage: runs.py > runs.txt"""
import json
import os

D = "/home/brian/taxman_work/lab/work/dyngate-2026-10-28"


def main():
    print("folder setting unit gap steps cpu_s completed best")
    for game in ("5054", "5052", "4984"):
        folder = f"{D}/{game}"
        gaps = {}
        for line in open(f"{folder}/units-{game}.jsonl"):
            r = json.loads(line)
            if r["kind"] == "unit":
                gaps[r["id"]] = r["gap"]
        out = f"{folder}/out"
        for name in sorted(os.listdir(out)):
            if not name.endswith(".json"):
                continue
            j = json.load(open(f"{out}/{name}"))
            setting = name[: -len(j["id"]) - len(".json") - 1]
            print(f"{game} {setting} {j['id']} {gaps[j['id']]} {j['nodes']} {j['cpu_s']:.1f} {j['completed']} "
                  f"{j['best']}")


if __name__ == "__main__":
    main()
