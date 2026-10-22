#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The defects of a few prime pairs at a root: pairdefect.py ROOT_INSTANCE.jsonl P,Q ..."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from nearpairs import swap  # noqa: E402

inst = next(json.loads(line) for line in open(sys.argv[1]) if '"instance"' in line)
role = {}
for h, (moves, factors, _) in enumerate(inst["frames"]):
    for x in moves:
        role[x] = ("move", h)
    for x in factors:
        role[x] = ("factor", h)
cands = {x for x, _ in inst["cands"]}
numbers = set(role) | cands
for arg in sys.argv[2:]:
    p, q = map(int, arg.split(","))
    touched = [x for x in numbers if x % p == 0 or x % q == 0]
    defect = [x for x in touched
              if role.get(x) != role.get(swap(x, p, q)) or ((x in cands) != (swap(x, p, q) in cands))]
    bad = [x for x in cands if x % (p * q) == 0 or x % (p * p) == 0 or x % (q * q) == 0]
    print((p, q), "touched", len(touched), "defect", len(defect), "cands moved",
          sum(1 for x in touched if x in cands), "cands divisible by pq, p2 or q2", len(bad))
    print("   ", [(x, role.get(x), x in cands, swap(x, p, q), role.get(swap(x, p, q))) for x in sorted(defect)[:12]])
