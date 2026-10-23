#!/usr/bin/env python3
"""pstudy.py [STUDY]: the study's lines (study.txt, the default: `LABEL UNIT PRICESTUDY {...}` and `LABEL UNIT
STEPTIMERS {...}`) summed up a run at a time: the pricings by how they relate to the one before, the share of the
searches' time, how many earlier cycles hold and are as long, and the timers of a step's parts and of the looks."""
import json
import sys

RELS = ["child_inherited", "child_other", "same_node", "unrelated"]

runs = {}
for line in open(sys.argv[1] if len(sys.argv) > 1 else "study.txt"):
    label, unit, tag, body = line.split(" ", 3)
    runs.setdefault(f"{label} {unit}", {})[tag] = json.loads(body)

for path, lines in runs.items():
    p = lines.get("PRICESTUDY")
    t = lines.get("STEPTIMERS")
    if p is None:
        print(path, "no study line")
        continue
    total_ns = sum(p[r]["nanos"] for r in RELS)
    total_m = sum(p[r]["members"] for r in RELS)
    print(f"== {path}: pricing searches {total_m}, {total_ns / 1e9:.2f} s")
    print(f"{'relation':16} {'pricings':>8} {'members':>8} {'m/pr':>5} {'ns share':>8} {'new':>6} {'same$':>6} "
          f"{'holds':>6} {'h=':>6} {'h<':>5} {'none':>5} {'nOK':>5} {'save%':>6} {'mism':>4}")
    save_all = 0
    for r in RELS:
        c = p[r]
        m = max(c["members"], 1)
        old = max(c["members"] - c["new"], 1)
        save = c["holds_nanos"] + c["none_carry_nanos"]
        if r == "child_inherited":
            save_all += save
        print(f"{r:16} {c['pricings']:8d} {c['members']:8d} {c['members'] / max(c['pricings'], 1):5.1f} "
              f"{c['nanos'] / max(total_ns, 1):8.3f} {c['new'] / m:6.3f} {c['same_price'] / old:6.3f} "
              f"{c['holds'] / old:6.3f} {c['holds_equal'] / max(c['holds'], 1):6.3f} "
              f"{c['holds_shorter'] / max(c['holds'], 1):5.3f} {c['none_carry'] / old:5.3f} "
              f"{c['none_carry_ok'] / max(c['none_carry'], 1):5.3f} {save / max(c['nanos'], 1):6.3f} "
              f"{c['self_mismatch']:4d}")
    print(f"exact carry (inherited children): {save_all / max(total_ns, 1):.3f} of the pricing searches' time")
    if t:
        ev, pr, st = t["evaluate_ns"], t["pricing_ns"], t["strong_ns"]
        print(f"timers: evaluate {ev / 1e9:.2f} s ({t['evaluates']}), pricing {pr / 1e9:.2f} s ({t['pricings']}), "
              f"strong {st / 1e9:.2f} s ({t['strongs']}), resolve in pricing {t['resolve_pricing_ns'] / 1e9:.2f} s, "
              f"in strong {t['resolve_strong_ns'] / 1e9:.2f} s, choose {t['choose_ns'] / 1e9:.2f} s, "
              f"ordered {t['ordered_ns'] / 1e9:.2f} s, look {t['look_ns'] / 1e9:.2f} s")
        if t["looks"]:
            print(f"looks {t['looks']}, cut {t['look_cuts']}, avoid {t['look_avoid']} (cut {t['look_avoid_cuts']}), "
                  f"pairs {t['look_pairs']} (cut {t['look_pair_cuts']}), uncertified {t['look_uncertified']}")
