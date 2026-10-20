# The parity bound in the model-free proofs (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-13.  Concluded 2026-10-14.

**Question.**  Does the parity bound, a bound on the whole board with a tenth of the matching bound's gap, carry the
prover's proofs of opt(n) ([the prover's proofs](../../noframe/docs/proofs.md)) from game 989 to the end of the
committed chain, 1300?  **Answer.**  Yes: it proved every game 2..1300 on the whole board, 990 in 44,389 nodes from
scratch where the matching bound had spent 70 million, and the hardest game, 1287, in 146,855 nodes.  A proof's cost
follows the root's parity gap.

## Method

The matching bound overrates opt(n) by about 1,170 at n = 1000, and games 990, 992, 993, 995, 996, 1020 and
1023-1027 had spent millions to tens of millions of nodes each without finishing.  The [matroid
study](../search/matroid-intersection-bounds-2026-10/README.md) found a bound with a tenth of that gap: every playable
set is independent in two linear matroids over GF(2), split by the parity of the number of prime factors (Theorem 10 of
THEORY.md, [the parity matroids](../../../docs/theory/bounds.md#the-parity-matroids-a-bound-on-the-whole-game)).  The
prover reimplements it from the mathematics, sharing no code with the frame solver or the study.

* **The bound** (`solver/noframe/src/parity.rs`).  It builds the two matroids from the board alone and finds the
  heaviest common independent set by Schrijver's shortest augmenting paths, with Dijkstra on potentials carried as
  Frank's weight split.  Every value is *certified*: the split w = w1 + w2 is checked against the local optimality
  conditions of each matroid over sets of all sizes, so the bound rests on the theorem and weak duality, not on the
  algorithm (no check failed).  The same split bounds the value with one member barred.
* **In the search** (`Prover::with_parity`, `solver/noframe/src/game.rs`).  At every node, after the matching bound, it
  cuts, bars the free numbers dependent on the forced picks, forces the members whose barring alone brings it to the
  target, and branches on a minimal unplayable subset of its heaviest set.  `exact` (`--parity`, on by default) also
  records the root's parity bound for every game and checks the recorded game independent in both matroids.
* **Validity.**  Theorem 10 holds on every board to 18, by enumeration.  The certified intersection equals enumeration
  on 3,000 random pairs of small GF(2) matroids with random weights (15,000 solves), and so do warm starts on 7,571
  children of random parents.  The prover's brute-force tests pass with the bound.  At the root of every game to 1300
  the bound is at least the recorded optimum, and it equals the matroid study's whole-game bound at all 1,088 games the
  study measured.

## Results

**1. At the root the bound overrates opt(n) by a tenth of what the matching bound does.**  The mean root gap over
every game 2..1300 (`examples/parityroot.rs`, 34 s in all), against the matching bound's in brackets: 2 over games 2 to
300 (23), 26 over 301-600 (172), 77 over 601-900 (510), 160 over 901-1000 (945), 132 over 1001-1300 (1,249).  The
bound is exact at 244 games, the last 527.  A solve from scratch takes 22 ms at 990 and 43 ms at 1300, against 0.29 and
0.59 s for the study's reference algorithm.  The certificate's split already forces 284 of the 433 members at the root
of 990 and 464 of 565 at 1300.

**2. Every game of the committed chain is proven.**  `exact` with the parity bound, on three threads in nine-minute
checkpointed rounds, took about 1 hour of wall time.  The open searches resumed and finished in 4,000-215,000 more nodes
each (990: 89,639 after 70.5 million; 992: 214,329 after 65.2 million; 1026 and 1027: about 4,500), then every game from
1028 to 1300 (the hardest, 1287, in 146,855 nodes and 279 s).  With the earlier campaign that covers every n in 2..1300.
A second campaign proved every game again from scratch, with the parity bound alone and the final build: 557 searches,
1.17 thread-hours on two threads, 199 games already proven by the root's bound.  `examples/paritycheck.rs` then ran each
of the 665 searched games against one below its optimum, and every one finds a set worth exactly V(n), which replays by
the rules (31 minutes).  So the frame model's answer is the true optimum for every game n <= 1300, each proven twice: to
989 by each bound, from 990 by a resumed and a fresh search.  Where both ran, the parity bound took about a thousandth
of the matching bound's nodes.

**3. A proof's cost follows the root's parity gap.**  Over 900-1300 the cost grows by a factor of 10 in nodes per 84
points of gap (R² 0.76), at about 1.6 ms a node.  Against n, log10 nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04
per 100 (R² 0.06 and 0.11, the gap's swings between bands dominating).  The trees match the size of the study's
frame-model branch and bound with the same bound (992: 66,892 nodes from scratch against its 77,567).  At that slope the
games 1300 to 2000 would take about 57 thread-hours, the hardest single proofs about 3 hours ([the prover's
proofs](../../noframe/docs/proofs.md) has the band table); the recorded optima, not the search, stop the proofs at 1300.

**4. Warm starts: kept, after one failed form.**  A child starts from its parent's certified set and split.  The split
stays feasible on every circuit arc of the child, since forcing members or barring elements only removes vertices and
shortens circuits, so only the root's arcs can be violated, and shortest cycles through the root improve the set.

* With Dijkstra on the inherited potentials, 36% of the warm starts at 990 fell back to a solve from scratch (108 s
  against 122 s cold).  The parent's set minus a barred member is not the heaviest of its size, so an exchange can
  break the potentials.
* With label-correcting searches (a vertex scanned again when its distance falls, 20 scans a vertex on average), 2-12%
  fell back (n = 1250 and 990), at 0.7 improving cycles a warm start against 15-40 augmentations from scratch.  The
  trees are the same and twice as fast: 990 took 61 s against 122 s, and the 3,215 nodes of 1250 4.6 s against 13.6 s.

**5. Measured, not kept.**  The Lagrangian bound over the nogoods beside the parity bound cut nothing at 990 (the same
44,411 nodes, 7% slower), so `exact` turns it on only without the parity bound.  Resuming the matching bound's
checkpoints costs more than starting afresh (990: 89,639 more nodes and 232 s, against 44,411 nodes and 122 s from
scratch; 992: 214,329 against 66,892), since the old open subtrees follow the matching bound's branching.  The campaign
resumed them anyway, so the old searches are part of the proofs (`"matching_nodes"` counts their nodes).

## Decision

Kept: the parity bound, with label-correcting warm starts, is the prover's bound, and `exact` uses it by default.  The
reach of the proofs is limited by the recorded optima, not by the laptop, since the cost grows by a factor of 1.7 per
100 in n.

## Files

The folder holds only this README.md.  The data are in `solver/noframe/data/`, the programs in `solver/noframe/`:

* [exact_parity_990_1300.jsonl](../../noframe/data/exact_parity_990_1300.jsonl): the resumed searches and the games
  1028 to 1300, made by `exact` with the parity bound.
* [exact_2_1231.jsonl](../../noframe/data/exact_2_1231.jsonl): the earlier campaign.
* [exact_parity_2_1300.jsonl](../../noframe/data/exact_parity_2_1300.jsonl): the second campaign, every game again from
  scratch with the parity bound alone.
* [paritycheck_2_1300.jsonl](../../noframe/data/paritycheck_2_1300.jsonl): the 665 searched games against one below
  their optimum, made by `examples/paritycheck.rs`.
* `examples/parityroot.rs`: the root gaps of every game 2..1300.

## Commits

* ff714b3: the parity matroids, a certified weighted intersection bound on opt(n).
* 7efc38b: the parity bound in the prover's search.
* 44803b6: warm starts for the parity bound, from the parent node's certificate.
