#!/Users/brian/dev/taxman_work/taxman/.venv/bin/python
"""The finite part of the square theorem (docs/theory/frame-model.md, "The square theorem").

Two checks, with the standard library only:

1. Short intervals.  For every real x with 25 <= x < 396738 there is a prime p with x < p <= 6x/5.  Dusart's
   Proposition 6.8 gives it for x >= 396738, so it holds for every x >= 25 (Nagura's theorem).  A real x lies between
   consecutive primes p <= x < p', and the worst x of that stretch is its least, so it is enough that
   5p' <= 6 max(p, 25) for every pair of consecutive primes with p < 396738 and p' > 25.
2. The frames of layer 2 of every board n = 2..N (default 2499).  It forms them as the components of the graph that
   joins each source of layer 2 (pq or p^2 in (n/2, n]) to its maximal factors, which are primes up to n/2.  It runs
   descending greedy on these sources as the scan of the graphic matroid of G_n (step 3 of the proof of the
   bottom-layers theorem) to get U*_2, and checks part 1 of the square theorem on every frame: it has one freedom if
   none of its sources is a square, and none if one is.  It lists the boards with several frames of layer 2, with no
   square among the sources, and with a frame of layer 2 that has freedoms, and checks that none is 187 or more: from
   187 on, layer 2 is a single frame with a square (part 2), and closed.  It also lists the boards from 36 on with no
   prime in (n/6, n/5], where the entry proves that the frame of 3 has a freedom, and checks that none is closed.

With --census PATH (structure_2_8191.jsonl.gz of the entry on the claims of unzip), it also compares the number of
frames of layer 2 and their freedoms with the census's, which forms the frames with the frame solver's own setup.

Run from anywhere, at low priority:

    nice -n 19 .venv/bin/python layer2.py [N] [--census PATH]
"""

from __future__ import annotations

import argparse
import gzip
import json
import time
from bisect import bisect_right
from typing import Dict, List, Optional, Tuple

DUSART_FROM = 396738  # Dusart, arXiv:1002.0442, Proposition 6.8: a prime in (x, x(1 + 1/(25 ln^2 x))] for x >= this
NAGURA_FROM = 25
CLOSED_FROM = 187  # the bound of part 2 of the square theorem
ANALYTIC_FROM = 2500  # the proof covers n >= this; the computation covers CLOSED_FROM <= n < ANALYTIC_FROM
SHARP_FROM = 36  # from here on, the frame of 3 has a freedom unless a prime lies in (n/6, n/5]


def sieve(limit: int) -> List[int]:
    """The primes up to limit."""
    is_prime = bytearray([1]) * (limit + 1)
    is_prime[0:2] = b"\x00\x00"
    for p in range(2, int(limit**0.5) + 1):
        if is_prime[p]:
            is_prime[p * p :: p] = bytearray(len(range(p * p, limit + 1, p)))
    return [p for p in range(limit + 1) if is_prime[p]]


def short_intervals(primes: List[int]) -> Tuple[bool, Tuple[int, int, float]]:
    """Check that every real x in [25, DUSART_FROM) has a prime in (x, 6x/5].  Returns the verdict and the tightest
    pair of consecutive primes (p, p', p' / max(p, 25))."""
    ok, tightest = True, (0, 0, 0.0)
    for p, q in zip(primes, primes[1:]):
        if q <= NAGURA_FROM:
            continue
        if p >= DUSART_FROM:
            break
        low = max(p, NAGURA_FROM)
        if 5 * q > 6 * low:
            ok = False
        if q / low > tightest[2]:
            tightest = (p, q, q / low)
    assert primes[-1] > DUSART_FROM, "sieve too short"
    return ok, tightest


class UnionFind:
    def __init__(self) -> None:
        self.parent: Dict[int, int] = {}

    def find(self, x: int) -> int:
        self.parent.setdefault(x, x)
        root = x
        while self.parent[root] != root:
            root = self.parent[root]
        while self.parent[x] != root:
            self.parent[x], x = root, self.parent[x]
        return root

    def union(self, a: int, b: int) -> bool:
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return False
        self.parent[ra] = rb
        return True


GROUND = 0  # the vertex g of G_n; no prime is 0


def layer2_frames(n: int, primes: List[int]) -> List[dict]:
    """The frames of layer 2 of board n, each with its primes, whether it holds a square, and its freedoms."""
    half = n // 2
    factors = primes[: bisect_right(primes, half)]
    # The sources of layer 2: (value, p, q) for pq with p < q, and (p^2, p, GROUND) for a square.
    sources: List[Tuple[int, int, int]] = []
    for i, p in enumerate(factors):
        if p * p > n:
            break
        if p * p > half:
            sources.append((p * p, p, GROUND))
        lo = max(i + 1, bisect_right(factors, half // p))  # q > p and pq > n/2
        hi = bisect_right(factors, n // p)  # pq <= n
        sources.extend((p * q, p, q) for q in factors[lo:hi])
    # Frames: the components of the frame graph.  A square joins only its prime, so the frames are the components of
    # the primes under the edges pq.
    frames = UnionFind()
    for p in factors:
        frames.find(p)
    for _, p, q in sources:
        if q != GROUND:
            frames.union(p, q)
    # U*_2: the scan of the graphic matroid of G_n over the sources, largest first.
    scan = UnionFind()
    kept = [s for s in sorted(sources, reverse=True) if scan.union(s[1], s[2])]
    by_root: Dict[int, dict] = {}
    for p in factors:
        by_root.setdefault(frames.find(p), {"primes": 0, "moves": 0, "square": False})["primes"] += 1
    for _, p, q in kept:
        by_root[frames.find(p)]["moves"] += 1
    for _, p, q in sources:
        if q == GROUND:
            by_root[frames.find(p)]["square"] = True
    result = []
    for frame in by_root.values():
        frame["freedoms"] = frame["primes"] - frame["moves"]
        result.append(frame)
    return result


def ranges(ns: List[int]) -> str:
    """12, 13, 14, 18 as '12-14, 18'."""
    out: List[str] = []
    for n in ns:
        if out and int(out[-1].split("-")[-1]) == n - 1:
            out[-1] = out[-1].split("-")[0] + "-" + str(n)
        else:
            out.append(str(n))
    return ", ".join(out)


def load_census(path: str) -> Dict[int, Tuple[int, int]]:
    """n -> (frames of layer 2, freedoms of layer 2), as the census recorded them."""
    census = {}
    with gzip.open(path, "rt") as f:
        for line in f:
            row = json.loads(line)
            frames, freedoms = row["frames_by_layer"], row["freedoms_by_layer"]
            census[row["n"]] = (frames[2] if len(frames) > 2 else 0, freedoms[2] if len(freedoms) > 2 else 0)
    return census


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("last", nargs="?", type=int, default=ANALYTIC_FROM - 1, help="the last board (default 2499)")
    parser.add_argument("--census", help="the census file structure_2_8191.jsonl.gz, to compare with")
    args = parser.parse_args()
    start = time.process_time()
    primes = sieve(max(DUSART_FROM * 6 // 5 + 1, args.last))
    ok_intervals, (p, q, ratio) = short_intervals(primes)
    print(f"short intervals: every real x in [{NAGURA_FROM}, {DUSART_FROM}) has a prime in (x, 6x/5]: "
          f"{'yes' if ok_intervals else 'NO'}; tightest: {p}, {q} (ratio {ratio:.4f})")

    census: Optional[Dict[int, Tuple[int, int]]] = load_census(args.census) if args.census else None
    part1_failures: List[int] = []
    census_failures: List[int] = []
    with_freedoms: List[int] = []
    several_frames: List[int] = []
    no_square: List[int] = []
    no_link: List[int] = []
    for n in range(2, args.last + 1):
        frames = layer2_frames(n, primes)
        if any(f["freedoms"] != (0 if f["square"] else 1) for f in frames):
            part1_failures.append(n)
        if any(f["freedoms"] for f in frames):
            with_freedoms.append(n)
        if len(frames) > 1:
            several_frames.append(n)
        if not any(f["square"] for f in frames):
            no_square.append(n)
        if n >= SHARP_FROM and 5 * primes[bisect_right(primes, n // 6)] > n:  # no prime in (n/6, n/5]
            no_link.append(n)
        if census is not None and n in census:
            if census[n] != (len(frames), sum(f["freedoms"] for f in frames)):
                census_failures.append(n)

    print(f"boards: 2..{args.last}")
    print(f"part 1 (one freedom without a square, none with one) fails on: {ranges(part1_failures) or 'none'}")
    print(f"boards where a frame of layer 2 has freedoms ({len(with_freedoms)}): {ranges(with_freedoms)}")
    print(f"boards with several frames of layer 2 ({len(several_frames)}): {ranges(several_frames)}")
    print(f"boards with no square in (n/2, n] ({len(no_square)}): {ranges(no_square)}")
    unlinked_closed = [n for n in no_link if n not in set(with_freedoms)]
    print(f"boards from {SHARP_FROM} with no prime in (n/6, n/5] ({len(no_link)}): {ranges(no_link)}; "
          f"closed among them: {ranges(unlinked_closed) or 'none'}")
    if census is not None:
        compared = sum(1 for n in range(2, args.last + 1) if n in census)
        print(f"census: {compared} boards compared, frames and freedoms of layer 2 differ on: "
              f"{ranges(census_failures) or 'none'}")
    late = sorted({n for n in several_frames + no_square + with_freedoms if n >= CLOSED_FROM})
    verdict = ok_intervals and not part1_failures and not census_failures and not late and not unlinked_closed
    print(f"on {CLOSED_FROM}..{args.last}, layer 2 is a single frame with a square, and closed: "
          f"{'yes' if not late else 'NO: ' + ranges(late)}")
    print(f"CPU seconds: {time.process_time() - start:.1f}")
    print("PASS" if verdict else "FAIL")


if __name__ == "__main__":
    main()
