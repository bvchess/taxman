#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""Which test the extra GF(2) candidates fail: a third computation of the candidates, from the theory's definitions.

    ./which_test.py N [N ...]

For each board n it forms the frames as components of the graph that joins each source to its maximal factors, U* by
descending greedy, the candidates by tests (a) and (b) with playability by peel (`heuristics/core.py`), and the GF(2)
candidates (the numbers up to n/2 that are loops of neither parity matroid contracted by U*) by Gaussian elimination,
frame by frame.  It prints the numbers that are GF(2) candidates and no candidates, with the test each fails, as a JSON
line a board, to compare with `extras.jsonl.gz`.
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "heuristics"))
from core import Infeasible, maximal_factors, optimize_mini, peel, smallest_prime_factors, upper_half_game  # noqa: E402


def layer(x, spf):
    k = 0
    while x > 1:
        x //= spf[x]
        k += 1
    return k


def playable(members, pool, mf):
    try:
        peel(members, pool, mf)
        return True
    except Infeasible:
        return False


def outside_span(v, rows):
    """Whether the bitmask v is outside the GF(2) span of the bitmasks rows."""
    basis = {}
    for r in rows:
        while r:
            top = r.bit_length() - 1
            if top not in basis:
                basis[top] = r
                break
            r ^= basis[top]
    while v:
        top = v.bit_length() - 1
        if top not in basis:
            return True
        v ^= basis[top]
    return False


def frames_of(n, spf, mf, ustar):
    half = n // 2
    parent = {}

    def find(x):
        while parent.setdefault(x, x) != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for s in range(half + 1, n + 1):
        for f in mf[s]:
            parent[find(s)] = find(f)
    for x in range(1, half + 1):
        find(x)
    parts = {}
    for x in list(parent):
        parts.setdefault(find(x), set()).add(x)
    frames = []
    for members in parts.values():
        factors = {x for x in members if x <= half}
        moves = {x for x in members if x > half} & ustar
        frames.append({"factors": factors, "moves": moves, "layer": layer(min(factors), spf) + 1,
                       "freedoms": len(factors) - len(moves)})
    open_layers = [fr["layer"] for fr in frames if fr["freedoms"] > 0]
    k0 = min(open_layers, default=0)
    for fr in frames:
        fr["number"] = fr["layer"] - k0 if fr["freedoms"] > 0 else None
    return frames


def extras(n):
    spf = smallest_prime_factors(n + 1)
    mf = {x: maximal_factors(x, spf) for x in range(2, n + 1)}
    sources, pool, _ = upper_half_game(n, spf)
    ustar, _ = optimize_mini(sources, pool, mf)
    frames = frames_of(n, spf, mf, set(ustar))
    frame_of = {x: fr for fr in frames for x in fr["factors"]}

    def vector(x, coords):
        return sum(1 << coords[f] for f in mf[x] if f in coords)

    found = []
    for y in range(2, n // 2 + 1):
        home = frame_of[y]
        if home["number"] is None or home["number"] == 0:
            continue  # e_y is spanned by a closed frame's moves, or v(y) by those of the closed layer in front
        coords = {f: i for i, f in enumerate(sorted(home["factors"]))}
        free_a = outside_span(1 << coords[y], [vector(s, coords) for s in home["moves"]])
        free_b = False
        for fr in frames:
            if fr["layer"] == home["layer"] - 1 and fr["freedoms"] > 0:
                cs = {f: i for i, f in enumerate(sorted(fr["factors"]))}
                v = vector(y, cs)
                free_b = free_b or (v and outside_span(v, [vector(s, cs) for s in fr["moves"]]))
        if not (free_a and free_b):
            continue
        test_a = playable(home["moves"], home["factors"] - {y}, mf)
        test_b = any(playable(fr["moves"] | {y}, fr["factors"], mf) for fr in frames
                     if fr["layer"] == home["layer"] - 1 and fr["freedoms"] > 0)
        if not (test_a and test_b):
            found.append({"y": y, "fails": "ab" if not (test_a or test_b) else ("a" if not test_a else "b")})
    return found


if __name__ == "__main__":
    for arg in sys.argv[1:]:
        print(json.dumps({"n": int(arg), "extra": extras(int(arg))}))
