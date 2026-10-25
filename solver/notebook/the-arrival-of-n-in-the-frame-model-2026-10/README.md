# The arrival of n in the frame model (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-24.  Concluded 2026-10-24.

**Question.**  Does game n pose the root problem of game n − 1 whenever every frame that n joins is closed in game
n − 1?  The idea came as a general form of [the cofactor rule](../../../docs/theory/arrival.md#the-cofactor-rule): the
search works only on the frames with freedoms, and n changes only the frames that hold its maximal factors.

**Answer.**  Yes, on every board from 3 to 8191, and it is now proved: [the arrival lemma
(J1)](../../../docs/theory/arrival.md#the-arrival-lemma), given (♦) for odd n and when U\*(n − 1) leaves n/2 out for
even n, and [its corollary for layer 2 (J2)](../../../docs/theory/arrival.md#the-layer-2-corollary), which needs no
(♦).  The 2,630 odd games that meet the hypothesis each give up exactly one source, the shadow of n, and the 555 even
ones give up none.  The run also found two laws, measured: an even n poses the root problem of game n − 1 iff n/2 ∉
U\*(n − 1), on every even board to 8190 but 14 ([the half law](../../../docs/theory/arrival.md#the-half-law)), and
every odd game that poses it, 2,651 to 8191, gives up the shadow ([the shadow
law](../../../docs/theory/arrival.md#the-shadow-law)).

## Method

* **The program.**  `arrival.rs` builds the root problem Π_n and the frames of every board from the definitions of the
  theory, with `taxman::root_problem::build_with_frames`, added to solver/frame/src/root_problem.rs for it.  `build`,
  which the test tests/canonical_root.rs checks against setup, is unchanged and now returns the first part of the new
  function.  The program compares the boards n − 1 and n, searching nothing.
* **The hypothesis, `cond`.**  For odd n: every maximal factor n/p of n is a factor of a frame of game n − 1 with no
  freedom.  For even n: n/2 is not in U\*(n − 1), and every maximal factor n/p with p odd is a factor of a frame of game
  n − 1 with no freedom.  The even test leaves out the frame that holds n/2, which J1 asks to be closed too; it was
  closed in all 555 even games that meet the test.
* **The outcome, `repeat`.**  Games n − 1 and n have the same candidates and root frames, moves and factors alike.  The
  program also records what U\* gains and loses, the shadow of an odd n, and the freedoms of the frame that holds n.
* **A check against the chain.**  For every game of the committed chain to 4949 with a digest of its setup, the
  program's comparison (with game n − 2 when n − 1 is prime, as the chain compares) agrees with the chain's digests: all
  3,920 comparisons agree.
* **The run**, on 2026-10-24: boards 3 to 8191, 7 min 36 s of wall time on 2 threads at nice 19, 909 s summed over the
  boards.  A rerun with a cut last line resumed and matched a clean run.

## Results

| boards | cond and repeat | cond, no repeat | repeat, no cond | neither |
|---|---|---|---|---|
| odd, 3 to 8191 | 2,630 | 0 | 21 | 1,444 |
| even, 4 to 8190 | 555 | 0 | 1,285 | 2,254 |

* **Under the hypothesis**, every odd game gave up exactly one source, the shadow of n, from a joined frame, and the
  frame that holds n came out closed: 2,630 of 2,630, 1,027 of them primes.  Every even game added n and gave up
  nothing: 555 of 555.
* **Odd repeats outside the hypothesis**: 21, all from 2197 = 13³ on, each with a prime factor 13, 17 or 19 (2197, 2873,
  3211, 3887, 3971, 4199, 5083, 5239, 5491, 5681, 6137, 6253, 6409, 6851, 6859, 6877, 6929, 7163, 7267, 7429, 7657).
  Each joins a frame of layer 3 with 49 to 110 freedoms, and each gives up the shadow.
* **Even games**: the 1,841 whose half U\*(n − 1) leaves out repeat but for 14, which joins a frame of layer 2 with a
  freedom; the 2,253 whose half it keeps never repeat.
* **Odd games with neither**: 1,444, among them 16 that join a frame with exactly one freedom and use it, so that
  nothing drops and the root problem changes: 9, 15, 25, 27, 35, 55, 77, 81, 85, 121, 145, 187, 243, 729, 2187 and 6561.
* **By the families of the cofactor rule**, from 188 to 8191: the 986 primes, 1,585 odd numbers with two prime factors
  and 540 numbers 2P meet the hypothesis and repeat; the 1,092 numbers 2pP repeat without it; the 147 numbers 9P do
  neither.

## The theory

The run confirmed the hypothesis, and the proof followed the same day.  The half lemma (N7, in
[neighbors.md](../../../docs/theory/neighbors.md#the-half-lemma)) puts an even game at its ceiling when an optimal game
of n − 1 leaves n/2 out.  J1 and J2 are in [arrival.md](../../../docs/theory/arrival.md), with the shadow law, the half
law and the cofactor rule.  A fresh reader checked the draft proofs.  They found three errors, all fixed: a misstated
set in step 5, a wrong reason in step 6, and opening sentences that dropped J1's hypotheses, with game 12 as the
counterexample to them.  They also noted that J1 uses (♦) only on the frames involved, and in layers 1 and 2 not at all,
which gave J2: the cofactor rule's first case is a theorem.

## Decision

No change to the solver: setup already finds a repeated root problem by comparing the problems, in under a second.  The
results explain which games need no search, and the questions left open are item 5 of [what to try after
5000](../../docs/agenda-after-5000.md).

## Files

* `arrival.rs`: the program.  Its module comment says how to run it (copy it into solver/frame/examples/) and what
  each field of a line means.
* `arrival.jsonl.gz`: one line per board, 3 to 8191.
* `summary.txt`: the program's summary, the source of every count above.
