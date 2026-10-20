# Improvements after parity

Part of the [lab notebook](../../README.md).

Started 2026-10-11.  Concluded 2026-10-12.

**Question.**  Once the Rust port reproduced the Java's setup and scores, what bound and what start prune the search
harder?  **Answer.**  A matching bound, kept incrementally, has a mean gap at the root of 31 against the Java bound's 76
on the 94 golden games that need search, and visits 1.5 to 85 times fewer nodes at 480-720.  Seeding a game's incumbent
from n − 1 and stopping at n + opt(n − 1) ends 415 of the 1300 proofs of the chain of bd9bb5d at the ceiling, before the
first node ([results](../../../../results/README.md#what-supports-the-scores)).

## Method

**The matching bound** is sound.  It relaxes winnability (the peel) to plain matchability across the whole frame chain:
maximize the promoted sum subject to every original move and every promoted number getting a distinct maximal factor in
its frame, and no promoted number being used as a factor.  Edges v → v/p change Ω(v) by one, so this is a max-weight
*bipartite* matching (min-cost flow), exactly solvable.  It is implemented in the module `matchbound` (used by `search`,
the tactic `match_bound`, since removed): an optimal matching and its dual potentials per level of the DFS path,
repaired by Hungarian searches after each change.  The first plan combined it with the Java bound by taking the minimum.

**The seed**: the tactics `seed_incumbent` and `prev_cap` start a game from the game of n − 1 and stop at the ceiling.

## Results

* *The gap at the root.*  On all 94 golden games that need search (a scratch study) the mean gap to the optimum is 31
  against 76 for the Java bound; on the bench games 107-292 against 363-769.  The cardinality cap never binds.
* *The cost of the bound.*  0.7-2.4 µs per evaluation in a DFS at n = 480-760, against 30-170 µs from scratch.
* *The nodes.*  A test DFS that starts with the optimum as incumbent and evaluates the matching bound only where
  `blameGame` fails to prune visits 85, 34, 1.5, 2.7, 1.7, 32 and 35 times fewer nodes at n = 480, 500, 600, 640, 660,
  700 and 720 (taking 57, 26, 1.5, 2.5, 1.6, 24 and 28 times less time), and at n = 760 finishes in 90k nodes where
  `blameGame` alone passes 3M.  The cap never bound there either.
* *The minimum of both bounds* pruned no more: `blameGame` never pruned where the matching bound did not (0 at every
  check).

## Decision

The shipped bound is the matching bound alone, and the seed and the ceiling are kept as tactics.

## Files

The folder holds this README.md only.

## Commits

Found in the history; the entry named none.

* 474f9ab, cfc72ab (2026-10-11): the matching bound, and the matching bound in the search.
* e180bd7 (2026-10-11): the per-game pipeline, with the seeded incumbent and the stop at the ceiling.
* 5f558e4 (2026-10-12): the matching bound replaces `blameGame`.
