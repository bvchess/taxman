#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Independent replay of a Taxman game: python replay.py FILE  (FILE: json with n, moves, score)."""
import json
import sys

game = json.load(open(sys.argv[1]))
n, moves = game["n"], game["moves"]
divisors = [[] for _ in range(n + 1)]
for d in range(1, n + 1):
    for m in range(2 * d, n + 1, d):
        divisors[m].append(d)
free = [True] * (n + 1)
player = taxman = 0
for m in moves:
    assert 1 <= m <= n and free[m], f"{m} not available"
    gift = [d for d in divisors[m] if free[d]]
    assert gift, f"{m} has no free proper divisor"
    free[m] = False
    player += m
    for d in gift:
        free[d] = False
        taxman += d
assert not any(free[m] and any(free[d] for d in divisors[m]) for m in range(1, n + 1)), "game not finished"
taxman += sum(m for m in range(1, n + 1) if free[m])
assert player + taxman == n * (n + 1) // 2
print(f"n={n} moves={len(moves)} player={player} taxman={taxman} claimed={game['score']} ok={player == game['score']}")
