#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Whether an optimal promotion set keeps c_p <= c_q for near pairs, and the heaviest set's: optpairs.py ROOT GAME P,Q...

ROOT is the root instance (`wall local --out`), GAME the game file with `promotions`."""
import json
import sys


def c(s, r):
    return sum(x // r for x in s if x % r == 0)


inst = next(json.loads(line) for line in open(sys.argv[1]) if '"instance"' in line)
game = json.load(open(sys.argv[2]))
cands = {x for x, _ in inst["cands"]}
opt = set(game["promotions"]) & cands
chosen = set(inst["chosen"])
print("optimal promotions", len(game["promotions"]), "of them candidates", len(opt), "sum", sum(opt))
for arg in sys.argv[3:]:
    p, q = map(int, arg.split(","))
    print((p, q), "optimum c_p, c_q:", c(opt, p), c(opt, q), "keeps" if c(opt, p) <= c(opt, q) else "BREAKS",
          "| heaviest set:", c(chosen, p), c(chosen, q), "keeps" if c(chosen, p) <= c(chosen, q) else "breaks")
