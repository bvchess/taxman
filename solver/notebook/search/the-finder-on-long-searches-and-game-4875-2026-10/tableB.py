#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Table of experiment B from B-runs/*.jsonl (the tune solves, one file a subtree and incumbent) and units.jsonl; writes
results-B.jsonl.

usage: tableB.py DATA_DIR      (DATA_DIR holds units.jsonl and B-runs/)
"""
import glob
import json
import math
import sys

D = sys.argv[1].rstrip("/")
INCS = (537927, 537971, 538000, 538030, 538080)
units = {}
gap = {}
for line in open(D + "/units.jsonl"):
    j = json.loads(line)
    if j["kind"] == "unit":
        gap[j["id"]] = j["gap"]
for f in sorted(glob.glob(D + "/B-runs/*.jsonl")):
    j = json.loads(open(f).read())
    inc = int(f.rsplit("-", 1)[1].split(".")[0])
    units.setdefault(j["id"], {"depth": j["depth"], "cands": j["cands"]})[inc] = j
rows = []
for uid, u in sorted(units.items(), key=lambda kv: gap[kv[0]]):
    if 537971 not in u or not u[537971]["completed"]:
        continue
    row = {"id": uid, "depth": u["depth"], "gap": gap[uid]}
    for inc in INCS:
        r = u.get(inc)
        row[f"steps_{inc}"] = r["nodes"] if r else None
        row[f"completed_{inc}"] = r["completed"] if r else None
        row[f"best_{inc}"] = r["best"] if r else None
        row[f"cpu_s_{inc}"] = round(r["cpu_s"], 3) if r else None
    base = row["steps_537971"]
    for inc in INCS:
        s = row[f"steps_{inc}"]
        row[f"ratio_{inc}"] = round(s / base, 3) if s is not None and row[f"completed_{inc}"] else None
    rows.append(row)
with open(D + "/results-B.jsonl", "w") as out:
    for row in rows:
        out.write(json.dumps(row) + "\n")
print(f"{'unit':10s}{'depth':>6s}{'gap':>5s}" + "".join(f"{i:>10d}" for i in INCS) + "   | ratios to 537971 " + "".join(f"{i:>8d}" for i in INCS if i != 537971))
for r in rows:
    def cell(inc):
        s = r[f"steps_{inc}"]
        return f"{s:>10d}" if r[f"completed_{inc}"] else f"{'>' + str(s):>10s}"
    def ratio(inc):
        v = r[f"ratio_{inc}"]
        return f"{v:>8.2f}" if v is not None else f"{'-':>8s}"
    print(f"{r['id']:10s}{r['depth']:>6d}{r['gap']:>5d}" + "".join(cell(i) for i in INCS) + "   |                  " + "".join(ratio(i) for i in INCS if i != 537971))
bests = {r[f"best_{i}"] for r in rows for i in INCS}
print("max best over all runs:", max(b for b in bests if b is not None))
def gm(inc, minsteps=0):
    v = [r[f"ratio_{inc}"] for r in rows if r[f"ratio_{inc}"] is not None and r["steps_537971"] >= minsteps]
    return math.exp(sum(map(math.log, v)) / len(v)), len(v)
for minsteps in (0, 200, 1000):
    print(f"units with >= {minsteps} steps at 537971:")
    for inc in INCS:
        if inc == 537971:
            continue
        g, n = gm(inc, minsteps)
        # slope in decades of steps per 100 of incumbent: log10(ratio) / ((inc - 537971)/100)
        slope = math.log10(g) / ((inc - 537971) / 100)
        print(f"  {inc}: geometric mean ratio {g:.3f} over {n} units; slope {slope:+.3f} decades per 100")
