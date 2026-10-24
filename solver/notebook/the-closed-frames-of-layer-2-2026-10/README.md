# The closed frames of layer 2 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-24.  Concluded 2026-10-24.

**Question.**  Are the frames of layer 2 closed on every board from 187 on, as the census of the frames found on every
board to 8191?

**Answer.**  Yes, for every n ≥ 187, by [the square theorem
(F10)](../../../docs/theory/frame-model.md#the-square-theorem).  A frame of layer 2 has one freedom if none of its
sources is the square of a prime, and none if one is; from 187 on, layer 2 is a single frame with a square among its
sources.  The proof covers n ≥ 2500 through primes in short intervals, and this entry's script covers 187 to 2499, in
0.6 s.

## Method

The theorem and its proof are in the theory; this entry holds the computations that the proof cites and what they
showed besides.

* **The bound of the proof.**  For n ≥ 2500 the proof needs a prime in (x, 6x/5] for every real x ≥ 25, Nagura's
  theorem (J. Nagura, "On the interval containing at least one prime number", *Proc. Japan Acad.* 28 (1952) 177–181).
  Secondary sources state it for an integer or a real x, with open or closed ends, so the proof takes it from a
  statement that is exact: P. Dusart, "Estimates of some functions over primes without R.H.",
  [arXiv:1002.0442](https://arxiv.org/abs/1002.0442) (2010), Proposition 6.8, which for every real x ≥ 396,738 gives a
  prime p with x < p ≤ x(1 + 1/(25 ln² x)).  The script checks the rest, 25 ≤ x < 396,738.  A real x lies between
  consecutive primes p ≤ x < p′, and the least such x is the worst, so it is enough that 5p′ ≤ 6·max(p, 25) for every
  pair of consecutive primes with p < 396,738 and p′ > 25.
* **The boards below 2500.**  The script forms the frames of layer 2 of every board n = 2..2499 from their definition:
  the components of the graph that joins each source of layer 2 (pq or p² in (n/2, n]) to its maximal factors, the
  primes up to n/2.  It checks that from 187 on there is one frame, with a square among its sources.
* **Part 1 against the frame solver.**  The script also computes U\*_2 by the scan of the graphic matroid (step 3 of
  [the bottom-layers theorem](../../../docs/theory/frame-model.md#the-bottom-layers-theorem)'s proof), and checks part 1
  of the theorem on every frame.  Given the census file of the entry on [the claims of
  unzip](../the-claims-of-unzip-in-the-extended-problem-2026-10/README.md), it compares the number of frames of layer 2
  and their freedoms with those the census recorded, which the frame solver's own setup formed.

## Results

* **Short intervals.**  Every real x in [25, 396,738) has a prime in (x, 6x/5].  The tightest pair is 31 and 37, a
  ratio of 1.1935.
* **The boards 187 to 2499.**  Layer 2 is a single frame with a square on every one, so part 2 of the theorem holds
  there.  The whole run, 2..2499 with the sieve, takes 0.6 s of CPU on one thread.
* **The boards below 187.**  A frame of layer 2 has a freedom on 80 boards: 8, 12-14, 18-24, 28-34, 42-54, 70-76,
  78-84, 98-120, 138-144 and 182-186, the census's list exactly.  Two causes give them, both on 20 and 114.  On 31 of
  them no square lies in (n/2, n] at all (8, 18-24 and 98-120, where no prime lies in (√(n/2), √n]).  On 51 layer 2
  splits into several frames, some with no square.  At 182 to 186, for instance, 11 and 13 meet no other prime in a
  source (7 · 13 = 91 is at most n/2, and 11 · 17 = 187 is off the board), so the frame of 121, 143 and 169 takes both
  squares of (n/2, n], and the frame of 2 has none; the source 187 joins them.
* **The census.**  On all 8,190 boards of 2..8191, the number of frames of layer 2 and their freedoms agree with the
  census, and part 1 holds on every frame (6 s).

**Why 6/5.**  The constant of Nagura's theorem is the one the proof needs, since the frames of layer 2 being closed
implies it at x = n/6.  Let n ≥ 36.  The partners of 3, the primes q with 3q in (n/2, n], lie in (n/6, n/3], and those
of 2 in (n/4, n/2].  A prime ℓ in (n/6, n/2] has its partners in (n/(2ℓ), n/ℓ], so among 2, 3 and 5, and 5 only if
ℓ ≤ n/5.  So unless a prime lies in (n/6, n/5], the frame of 3 has its primes among 2, 3 and those of (n/6, n/2], and
none of their squares lies in (n/2, n]: 4 and 9 are at most n/2, and (n/6)² ≥ n.  By part 1 that frame has a freedom.
The script lists the 28 boards of 36..2499 with no prime in (n/6, n/5], 42-54, 78-84, 114 and 138-144, and none of them
has its frames of layer 2 closed.

## Decision

The theory states the result as the square theorem (F10), at the end of
[frame-model.md](../../../docs/theory/frame-model.md#the-square-theorem), and the bottom-layers theorem's sentence that
reported the census's range now cites it.  For n ≥ 187 every optimal set plays exactly U\*_2 in layer 2, and the frame
model starts at layer 3 or above.  Nothing in the solvers changes: both decide each board's closure from its own U\*.
An error in Dusart's Proposition 6.8 or in the script would reopen part 2; the first would need the check of short
intervals carried up to where another explicit bound starts.

## Files

| file | what it is | the command that made it |
|---|---|---|
| `layer2.py` | the checks: short intervals for 25 ≤ x < 396,738, the frames of layer 2 of every board to N (default 2499), part 1 on each frame, the boards with no prime in (n/6, n/5], and the comparison with the census | |
| `layer2_output.txt` | the run that the theory cites, boards 2..2499 | `nice -n 19 .venv/bin/python solver/notebook/the-closed-frames-of-layer-2-2026-10/layer2.py` |
| `layer2_census_output.txt` | the run over 2..8191 against the census | the same, with `8191 --census solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/structure_2_8191.jsonl.gz` |
