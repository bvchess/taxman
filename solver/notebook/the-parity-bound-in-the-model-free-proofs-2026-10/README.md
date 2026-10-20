# The parity bound in the model-free proofs (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The model-free proofs of opt(n) ([prover 3](../../noframe/docs/proofs.md))
stopped at 989 (and 999-1019): the matching bound overrates the optimum by about 1,170 at n = 1000, and games 990,
992, 993, 995, 996, 1020 and 1023-1027 had spent millions to tens of millions of nodes each without finishing.  The
[matroid study](../matroid-intersection-bounds-2026-10/README.md) found a model-free bound with a tenth of that gap: every playable
set is independent in two linear matroids over GF(2), split by the parity of Ω.  Reimplemented in the evidence crate
from its mathematics (no code shared with the solver or the study) and proven there as a theorem, does it carry the
model-free proofs to the end of the committed chain, 1300?  What does a proof cost, and how fast does that grow?

*The theorem.*  THEORY.md, [the parity
matroids](../../../docs/theory/bounds.md#the-parity-matroids-a-bound-on-the-whole-game) (Theorem 10): a playable set's
vectors of maximal factors, with the unit vectors of its own members up to n/2, are linearly independent (from a legal
order and Theorem 1's lifting argument; for the upper half it is Theorem 7's lemma), and each vector lies in the
coordinates of one parity of Ω, so every playable set is independent in both parity matroids; at a node of a search,
with the forced picks contracted and the barred numbers deleted.

*Method.*  `src/parity.rs` (commits ff714b3, 7efc38b, 44803b6): the two matroids from the board alone, an echelon basis
over GF(2) kept in reduced form whose rows carry the members they sum (a circuit costs one row per coordinate of a
sparse vector), and the heaviest common independent set by Schrijver's shortest augmenting paths (Theorem 41.7) with
Dijkstra on potentials carried as Frank's weight split.  Every value is *certified*: a split w = w1 + w2 is checked
against the local optimality conditions of each matroid over sets of all sizes, so the bound rests on the theorem and
weak duality, not on the algorithm (none failed).  The same split bounds the value with one member barred.
`Prover::with_parity` (`src/game.rs`) runs it at every node after the matching bound: it cuts, bars the free numbers
dependent on the forced picks, forces the members whose barring alone brings it to the target, and the node branches
on a minimal unplayable subset of its heaviest set.  A child starts from its parent's certificate (warm starts, below).
`exact` (`--parity`, on by default) records the root's parity bound for every game and checks it against the recorded
score, and the recorded game independent in both matroids.

*Validity.*  Theorem 10 on every board to 18: every playable subset of 2..n independent in both matroids (enumerated).
The certified intersection equals enumeration on 3,000 random pairs of small GF(2) matroids with random weights and
roles (15,000 solves), and so do warm starts on 7,571 children of random parents.  The prover's brute-force tests run
with the parity bound too (opt(n) for n <= 20 from nothing, the most moves to 20, every set beating a target to 19,
the optimum found from one below it every 7th game to 420, pause and resume at 600).  At the root of every game to 1300
the bound is at least the recorded optimum, the recorded game is independent in both matroids, and the value equals
the matroid study's whole-game bound at all 1,088 games the study measured (`examples/parityroot.rs`).

**1. At the root it overrates the optimum by a tenth of what the matching bound does.**  `examples/parityroot.rs`
over every game 2..1300 (34 s in all): mean gap 2 to 300 (the matching bound's 23), 26 over 301–600 (172), 77 over
601–900 (510), 160 over 901–1000 (945), 132 over 1001–1300 (1,249); exact at 244 games, the last 527.  A solve from
scratch takes 22 ms at 990 and 43 ms at 1300 (400–560 augmentations from the longest independent prefix), against
0.29 and 0.59 s for the study's reference algorithm.  The certificate's split already forces 284 of the 433 members
at the root of 990 and 464 of 565 at 1300.

**2. Every game of the committed chain is proven.**  `exact` with the parity bound on three threads in nine-minute
checkpointed rounds (`data/exact_parity_990_1300.jsonl`, about 1 hour of wall time): the open searches resumed and
finished in 4,000–215,000 more nodes each (990: 89,639 after 70.5 million; 992: 214,329 after 65.2 million; 1026 and
1027: about 4,500), then every game from 1028 to 1300 (the hardest, 1287, 146,855 nodes in 279 s).  With
`data/exact_2_1231.jsonl`: every n in 2..1300.  A second campaign proves every game again from scratch with the parity
bound alone and the final build (`data/exact_parity_2_1300.jsonl`: 557 searches, 1.17 thread-hours on two threads,
199 games already proven by the root's parity bound), and `examples/paritycheck.rs` runs each of the 665 searched
games against one below its optimum: every one finds a set worth exactly V(n), which replays by the rules
(`data/paritycheck_2_1300.jsonl`, 31 minutes).

**3. A proof's cost follows the root's parity gap.**  Over 900–1300, ×10 nodes per 84 points of gap (R² 0.76); against
n, log10 nodes grows 0.16 ± 0.04 and log10 seconds 0.23 ± 0.04 per 100 (R² 0.06 and 0.11, the gap's swings between
bands dominating), about 1.6 ms a node.  The trees are the size of the study's frame-model branch and bound with the
same bound (992: 66,892 nodes from scratch against its 77,567).  The band table and the extrapolation are in
[prover 3](../../noframe/docs/proofs.md): at that slope 1300–2000 would take about
57 thread-hours, the hardest single proofs about 3 hours; the recorded optima, not the search, stop the proofs at 1300.

**4. Warm starts: kept, after one failed form.**  A child starts from its parent's certified set and split.  The split
stays feasible on every circuit arc of the child (forcing members of a heaviest set or barring elements only removes
vertices and shortens circuits), so only the root's arcs (adding an element, dropping a member) can be violated, and
the set is improved by shortest cycles through the root.  With Dijkstra on the inherited potentials 36% of the warm
starts at 990 fell back to a solve from scratch: the parent's set minus a barred member is not the heaviest set of
its size, so after an exchange new circuit arcs can violate the potentials (Schrijver's update needs extremality).
Label-correcting searches (a vertex scanned again when its distance falls, giving up after 20 scans a vertex on
average) cut the fallbacks to 2–12% (n = 1250 and 990), at 0.7 improving cycles a warm start against 15–40
augmentations from scratch: the same trees twice as fast (990: 61 s against 122 s; the 3,215 nodes of 1250: 4.6 s
against 13.6 s).  Forced numbers outside the parent's set, which the warm start cannot use, occurred once in 7,084.

**5. Measured, not kept.**  The Lagrangian bound over the nogoods beside the parity bound cut nothing at 990 (the same
44,411 nodes, 7% slower), so `exact` turns it on only without the parity bound.  Resuming the matching bound's
checkpoints costs more than starting afresh (990: 89,639 more nodes and 232 s, against 44,411 nodes and 122 s from
scratch; 992: 214,329 against 66,892): the old open subtrees follow the matching bound's branching; the campaign
resumed them anyway, so the old searches are part of the proofs (`"matching_nodes"` counts their nodes).

*Conclusion.*  The parity matroids' bound, proven as THEORY.md's Theorem 10 and reimplemented independently with a
certificate checked on every value, carries the model-free proofs of opt(n) from 989 to the end of the committed
chain: the frame model's answer is the true optimum for every game n ≤ 1300, each proven twice (to 989 by each bound,
from 990 by a resumed and a fresh search), at about a thousandth of the matching bound's nodes where both ran.  Its cost
grows ×1.7 per 100 in n, so the reach is limited by the recorded optima, not by the laptop.
