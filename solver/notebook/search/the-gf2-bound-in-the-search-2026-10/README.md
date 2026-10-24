# The GF(2) bound in the search (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-14.

**Question.**  The [matroid study](../matroid-intersection-bounds-2026-10/README.md) found a bound whose gap at the root
is a tenth of the matching bound's and grows a thirtieth as fast, and a plain branch and bound with it, computed from
scratch at every node, that needs 12 to 613 times fewer nodes than the solver.  Can the search use it as its node bound
at a cost per node that pays, what of the solver's machinery still earns its keep with it, is it sound, and what does it
do to the cost of the chain and the reach of a large machine?  **Answer.**  Yes, and it is the default.  The 146
one-thread proofs of the slope study run 45 times faster (76 s against 3,408 s) with 426 times fewer nodes, and the
chain 1001-1300 takes 60 s on 2 threads against 988 s on 4.  The chain reached 2175 in 92 minutes of game time with no
growth a fit resolves (−0.012 ± 0.033 decades per 100 n), and a day on 192 CPUs reaches 4095 at every slope the data
support, about 3200 at the steepest measured.

## Method

**What was built** (commits 679635b to 7af8b0e; [the GF(2) bound](../../../docs/design/gf2-bound.md) describes the code
as it stands, and [the frame matroids](../../../../docs/theory/frame-bounds.md#the-frame-bound)
give the proof, Theorem 11):

* `gf2::Gf2Graph` holds the root's two matroids (the even and the odd frames' quotients by their moves, 1 to 3 words a
  block: the frames' ranks at 1215 are 33, 52, 34, 18, 7, 3, 1).  `gf2::Gf2Bound` keeps a level per node of the
  searcher's path: the root's vectors contracted by the node's promotions, the live candidates, and the *history* of
  dominant sets its augmentations passed through, from which a child, or the node without a member, starts again; a
  child whose parent's heaviest set survives inherits it with no solve.
* `gf2::meet` is the intersection: Lawler's shortest augmenting paths (Bellman-Ford, lexicographic in length and arcs)
  over a tableau of bitset rows that each exchange updates by one pivot, a start from the heaviest uncontended
  candidates, and the weight split (Frank) that bounds the intersection without each member of the optimum in one pass.
* The tactic `gf2` (checkpoint header bit 7; a checkpoint of an earlier build has it clear and resumes on the matching
  path) gives the node bound, the conflicts of its heaviest set to branch on, forced members from the split, and exact
  strong branching: the bound without each member of the conflict solved, in the order of the split's bounds, each solve
  stopped at the larger of the best so far and `need`, after a greedy lower bound; the chosen member's level is kept for
  the node.  The cut reason is `gf2`.

**What measured it.**  `examples/matroid.rs prove` (one-thread proofs with the knobs below, cycles and the bound's
counters), the probe's F4 (`perf --tactic gf2=on`: the bound from scratch, with its split, and without each conflict
member, at the 600 fixture nodes), `slope prove` for the 146 proofs, and the knobs `SearchSpace::gf2_prefilter`,
`gf2_branching`, `gf2_strong_limit` and `gf2_more_forced`, which are not tactics.  One-thread proofs set the incumbent
`P*` as a value, with no seed or cap.

## Results

**The inner loop.**  Each step measured on the probe's nodes or on proofs, values and trees unchanged:

* Bitset rows per block instead of a span rebuilt per element; members and outside elements kept in lists along each
  path; the sources and the best sink found as distances are set; the pull phase skipping the blocks of `A` with no
  changed member; the split reusing the workspace when it still holds the level's solve: the bound from scratch takes
  32% fewer cycles at 1215's nodes, the split 40% (da49d8a).
* Branchless pivots (the row selected by a mask made from the pivot bit), and no allocation in the split: 11% less time
  at 1254 and 1300 (f1d09e7).
* A greedy lower bound before each strong-branching solve (the level's heaviest set without the member, completed
  greedily): when it reaches the solve's stop, no solve; 10% less time at 1254 (9a99f34).
* Profile-guided optimization (`solver/tools/pgo.sh`, trained on the GF(2) probe too): 1.5% fewer cycles on the probe's
  GF(2) kernels and windows, 1.8% on the proof of 1287, the same fingerprint and tree.
* Where the time goes (a sampled proof of 1287, one thread): the shortest paths 57%, the level's own work (the solve's
  setup, history, the split) 22%, the pivots 8%, the peels 3%.  An augmentation takes 2.4-2.5 Bellman-Ford rounds, so
  Dijkstra with potentials would save little.  A strong-branching solve makes 22-32 augmentations at the probe's nodes,
  because it restarts from the last dominant set without the member, which for a heavy member is the uncontended prefix;
  strong branching is 70% of the augmentations (1287: 549,223 of 799,278).
* The probe's `perf --tactic gf2=on` has the fingerprint 162e4e94a52a95c6 (the default fingerprint is unchanged:
  7e8e724717d26545).  A step of the window of 1215 (a checkpoint 600,063 steps into the matching path's proof) costs
  329k cycles and the window ends after 12,633 steps, where the matching path's steps cost 22k and 25k and its window
  does not end.  The bound from scratch at a node costs 219k and 455k cycles (936, 1215), with the split 294k and 570k,
  and the bound without each conflict member, warm-started, 205k and 378k a member.

**What the machinery is worth with it.**  One-thread proofs of 936, 1100, 1184, 1215, 1250, 1254 and 1288
(`matroid prove`).  The default's proofs take 409 (1184) to 11,910 (1288) nodes and 0.13 to 3.83 G cycles.  Each
variant's nodes and cycles, relative to the default's, as the geometric mean over the seven:

| variant | nodes | cycles |
|---|---|---|
| `branching=split` | 2.87 | 1.96 |
| `branching=largest` | 3.60 | 2.43 |
| `branching=forced` | 1.78 | 1.54 |
| `prefilter` (the matching bound as a first check) | 1.00 | 1.02 |
| `strong=1` (at most one member solved) | 2.87 | 1.67 |
| `strong=3` | 1.31 | 1.21 |
| first-forced (stop at the first forced member) | 1.23 | 1.05 |
| `lagrange=on` | 1.00 | 1.02 |
| `nogoods=off` | 1.23 | 1.22 |
| `fix_forced=off` | 1.13 | 2.76 |
| `matching` (`gf2=off,lagrange=on`, the former default; five games, not run at 1254 and 1288) | 79.18 | 6.58 |

* **`lagrange`:** no tree changes (992 and 1025 by one to three nodes), and 0-7% more cycles: off.  Its multipliers are
  first fitted after 2,000 steps, and the GF(2) bound already holds what the nogoods it relaxes would add.
* **The matching bound as a first check (`prefilter`):** the same trees, 1-4% more cycles: not kept.
* **`nogoods`:** 22% more cycles without (1100 is 16% cheaper without, 1184 75% dearer): on.  `forget_nogoods` changes
  nothing at these sizes (fewer than 50,000 steps a searcher) and stays on.
* **`fix_forced`:** 2.8 times the cycles without (fewer nodes at three games, each much dearer): on.
* **`match_refine`** has no effect with `gf2`, since the matching bound is not computed.
* **Branching:** exact strong branching beats choosing by the split's bound (`split`: 2.9 times the nodes, 2.0 times the
  cycles), the largest member (3.6 and 2.4) and exact solves only to find the forced members (`forced`: 1.8 and 1.5).
  Solving at most one or three members takes 1.7 and 1.2 times the cycles, and stopping at the first forced member 5%
  more: exact, every member.
* **The matching path** takes 2.1 times the cycles at 936 and 1100 and 40 times at 1250, and 79 times the nodes and 6.6
  times the cycles of the GF(2) proofs over the five games.

**Soundness.**  Promotable sets are independent in both matroids, at every node in their contractions, and the forced
members rest on weak duality for the split (Theorem 11).  The tests check the intersection against brute force; every
promotable set of the boards up to 140 and the optimal promotions of every board up to 400 against the bound; levels
along random paths of games 60-1000 against matroids built afresh at the node; random completions at the probe's 600
nodes of 936 and 1215, with the matching path's search from the bound finding nothing above it (597 of the 600 searches
end within 200 ms); the soundness harness (handicapped incumbents) under 11 GF(2) configurations on 2-300, the
Lagrangian games of 301-600, the perturbed games and the wide units; and resumes, tripwires, checkpoints and the CLI on
1000 and 480-1133.

`taxman regress --to 1000` (2 threads) matches every score of optimal.json, in 21.6 s and 92.6 G cycles against 96.9 s
and 554 G on the matching path.  The chain 1-1300 (`taxman solve 1-1300 --threads 2`, to a scratch file) has every score
equal to `results/strict.jsonl`'s, with the same methods and proofs (trivial 13, prime 211, cap 115, exhausted 416, same
search 545); `verify-results` accepts it, and it took 88 s of wall time and 395 G cycles.

**The cost.**

1. **The chain.**  1001-1300 took 60.3 s of game time on 2 threads (48.1 s of search, 736,530 nodes), against
   `results/strict.jsonl`'s 988.4 s on 4 threads (976.3 s, 315,916,127 nodes): 16 times less time on half the threads,
   429 times fewer nodes.  1201-1300: 35.3 s against 901.7 s (26 times).  The dearest game is 1287 at 4.9 s (strict:
   1232 at 206.7 s; with GF(2) 3.4 s).  1-1000: 21.4 s against 53.4 s.
2. **The 146 proofs of the slope study** (`slope prove`, one thread, two at a time; against [the slope study's
   proofs](../what-drives-the-slope-2026-10/slope_proofs.jsonl), the matching path at its commit), by bands:

   | band | games | nodes, matching | nodes, GF(2) | ratio | s, matching | s, GF(2) | ratio | Gcycles, GF(2) |
   |---|---|---|---|---|---|---|---|---|
   | 851-900 | 6 | 48,833 | 1,267 | 39 | 0.5 | 0.1 | 6.5 | 0.9 |
   | 951-1000 | 16 | 8,915,952 | 123,724 | 72 | 107.0 | 11.8 | 9.1 | 37.7 |
   | 1101-1150 | 17 | 141,744 | 4,798 | 30 | 1.5 | 0.5 | 3.3 | 3.8 |
   | 1201-1250 | 19 | 64,621,157 | 106,869 | 605 | 872.2 | 13.6 | 64.2 | 44.3 |
   | 1251-1300 | 16 | 171,570,238 | 164,180 | 1045 | 1750.5 | 19.4 | 90.3 | 61.8 |
   | all | 146 | 297,409,715 | 697,346 | 426 | 3407.6 | 75.8 | 45.0 | 251.5 |

   The other bands, from 851 to 1400, are in [proofs.jsonl](proofs.jsonl); the ratio of nodes runs from 30 (1101-1150)
   to 1045 (1251-1300).  The geometric mean ratio is 111 in nodes and 11.5 in seconds.  Fits of log10 against n over the
   146 games: nodes +0.176 ± 0.052 per 100 n (ten times per 568 n) against +0.476 ± 0.066 (per 210 n); seconds +0.242 ±
   0.052 (per 414 n) against +0.483 ± 0.067 (per 207 n); the processes' cycles +0.195 ± 0.035 (per 514 n).  A node costs
   more as n grows (more candidates, larger blocks), so time grows faster than nodes, but at half the matching path's
   rate.  The dearest proofs: 1350 5.8 s (matching 282 s), 1287 4.6 s (239 s), 1352 4.3 s, 996 3.6 s, 992 3.4 s, 1232
   3.0 s (359 s).
3. **Threads** (`ladder --seed-opt`, the proofs of 1215, 1232, 1287 and 1290): summed, 10.8 s on one thread, 6.1 s on
   two (1.79 times), 3.75 s on four (2.89 times).  The trees grow a little with threads (89,181 nodes, then 96,427 and
   101,737: each thread's nogoods), the effective parallelism is 97-99%, and the node rate per thread falls from 8.2k/s
   to 8.0k and 6.8k (another job used three of the ten cores meanwhile).  These searches last 0.5-5 s, and all four
   threads were busy 30 ms after the start.
4. **Past 1300** (`taxman solve 1301-4095 --use-prev --threads 2`, a chain from optimal.json's game of 1300, in three
   invocations, the last stopped by its 29-minute cap during game 2176) reached 2175: 875 games, all completed (prime
   115, cap 77, exhausted 314, same search 369), in 5,490 s of game time and 48.9 M nodes.  `verify-results` accepts the
   file, and the ten games of 1302-1352 that the matching path had solved standalone
   ([bands.jsonl](../benchmarks-the-probe-pgo-and-scaling/bands.jsonl)) have the same scores.  Per band of 50 n, the
   extremes ([chain.jsonl](chain.jsonl) has the rest):

   | band | game time s | dearest game s (n) | nodes | µs a node (thread time) | root GF(2) gap, searched games |
   |---|---|---|---|---|---|
   | 1301-1350 | 79 | 22 (1326) | 1,222,512 | 124 | 204 |
   | 1451-1500 | 896 | 188 (1475) | 9,836,779 | 181 | 481 |
   | 1901-1950 | 48 | 7 (1938) | 258,232 | 293 | 249 |
   | 2101-2150 | 182 | 19 (2140) | 829,229 | 385 | 473 |
   | 2151-2175 | 847 | 290 (2170) | 4,286,865 | 389 | 612 |

   The cost comes in waves (up to ×8 between neighboring bands of 25 n, ×75 between the cheapest and the dearest,
   2151-2175, which holds 2170 at 290 s), and over 1301-2175 there is no growth a fit resolves: log10 of a band's time
   changes by −0.012 ± 0.033 per 100 n (residual sd 0.49).  The waves follow the root's GF(2) gap: an exhausted search's
   nodes grow ×10 per 236 of gap (314 searches, residual sd 0.44), and the gap grows only 6.9 ± 1.7 per 100 n (sd 115
   from game to game; the matroid study's 4.6 ± 2.4 over 880-1300), where over 880-1300 the matching bound's grew 142
   and the Lagrangian bound's 56.  A node costs more as n grows, +0.045 ± 0.004 decades per 100 n (candidates, frames
   and wider sets: 120-180 µs of thread time at 1300-1500, 320-390 at 2050-2175, past the start of W = 64 and of an
   eighth frame at 2048).  Together: ×10 per about 1,350 n.

   *Projections*, made as [the cost curve](../benchmarks-the-probe-pgo-and-scaling/cost-curve.md) made them: a 192-core
   c9g.48xlarge counted as 192 × 0.8 M1 cores, linearly or at half that, which is 86 or 43 times this machine's 2
   threads (1.79 one-thread equivalents); $3.33/h spot, $8.35/h on-demand; a game at n costing the mean of 1301-2175
   (6.27 s on 2 threads, at 1738) times 10^(slope × (n − 1738)), summed from 2176 (a day and a week are on 192 cores at
   spot):

   | slope | to 3000, 192 cores | to 4095, 2 threads | to 4095, 192 cores | a day ($80) reaches | a week ($559) |
   |---|---|---|---|---|---|
   | the fit over 1301-2175 (none) | 1-2 minutes | 2.3 hours | 2-3 minutes | 4095 | 4095 |
   | the fit + 2 s.e. (×10 per 1,878 n) | 3-6 minutes | 23 hours | 16-32 minutes | 4095 | 4095 |
   | the gap model (×10 per 1,350 n) | 5-9 minutes | 55 hours | 38-77 minutes | 4095 | 4095 |
   | the proofs' slope, 880-1352 (×10 per 413 n) | 4-8 hours | 18 years | 77-154 days | 3191-3315 | 3540-3664 |

   With the GF(2) bound the chain is not a question of a large machine's time up to the largest board the set widths
   cover (4095; beyond it is a code change, a wider set, [boards past 4095](../../boards-past-4095-2026-10/README.md)).
   A day on 192 cores takes it there at every slope the chain past 1300 supports, and even at the steepest any GF(2)
   measurement has shown (the proofs of 880-1352, where the gap grows fastest), a day reaches about 3200 and a week
   about 3600; the matching path's curve gave 1510 and 1565.  Not measured: games past 2175, where frames and widths
   change again; how a search of 10^5-10^6 nodes spreads over 192 threads (measured to 4 here); single games in the
   waves, which can cost many times their band's mean.
5. **Against the model-free search** (the prover's `exact`, which proves opt(n) without the frame model with the parity
   bound, Theorem 10, the same bound on the whole game; [the prover](../../../noframe/README.md); this machine,
   `--threads 2`, one game at a time, which it searches on one thread): 1232 in 70.5 s (34,456 nodes, 221 G cycles),
   1300 in 33.9 s (17,771 nodes, 106 G), 990 in 60.9 s (44,389 nodes, 192 G).  The frame model's search with the GF(2)
   bound proves them (one thread, the incumbent the optimum, as `exact` does) in 3.0 s (22,524 nodes, 9.2 G cycles),
   1.35 s (11,128, 4.3 G) and 1.7 s (20,975, 5.4 G): 24, 25 and 36 times fewer cycles.  The trees are within a factor of
   two (the bounds agree at the roots), so the difference is the cost of a node, 6.4, 6.0 and 4.3 M cycles there against
   0.41, 0.39 and 0.26 M here.  The frame model's matroids are the whole game's with the upper half and the frames'
   moves contracted away (at 1302, 331 candidates in blocks of rank 1 to 55, against the numbers 2..n over n/2
   coordinates), and each node's are its parent's contracted by one pivot a promotion, where `exact` solves each node's
   matroids warm-started from its parent's certificate.  In the chain the frame model also has the seed and the cap:
   1232 in 3.4 s on 2 threads.

## Decision

* **Defaults:** `gf2` on, `lagrange` off, the rest as they were (`match_refine` 1, which matters only on the matching
  path).  Made the defaults in 7af8b0e; `SearchTactics::matching()` names the former default, which the plumbing tests
  use where they must stop a search of a small game.
* **To remove** (not removed here): the knobs `gf2_prefilter`, `gf2_strong_limit` and `gf2_more_forced` and the
  branchings `Split`, `Largest` and `Forced` (`Gf2Branching`), which lost; and, once a production run has been made with
  the GF(2) search, what only the matching path uses and the GF(2) bound has made idle: the Lagrangian bound in the
  search (`search::lagrange`, `matchbound::lagrange`, the tactic `lagrange`) and refinement (the branches of
  `search::refine::Refiner`, the tactic `match_refine`).  The matching path itself (the matching bound at nodes and its
  strong branching) stays: it is the tests' independent second search (the probe-node test, the plumbing tests' long
  searches, the tripwires), and the setup's dive rounds the root's matching into the starting incumbent.
* **Strong branching's solves** (70% of the augmentations) restart far below their answer, and the study proposed
  canceling the negative cycles of the optimum's exchange graph to start each from an extreme set one smaller.  That
  was not needed: [the inner loop of the GF(2) search](../the-inner-loop-of-the-gf2-search-2026-10/README.md), the round
  after this one, did without most of the solves, since the heaviest set without a member is the heaviest set flipped
  along one cycle of its exchange graph, read off with no solve and certified by what the sets of other sizes lose (the
  rows of 2026-10-14 in [the register](../../register/node-bound.md)).
* **Seams kept open** for studies that would build on this bound (the cover LP or a Lagrangian over the conflicts with
  this intersection as the inner problem, cut pools at the root, the symmetry rule): `meet::Problem` takes any positive
  weights, but the start from the heaviest uncontended candidates and the greedy lower bound assume that weights do not
  decrease with the element's index (the candidates by increasing number).  A reweighting needs its own element order
  (the root's matroids built in it) or those two starts switched off; the levels, the contraction and the split do not
  depend on the order.

## Files

* [proofs.jsonl](proofs.jsonl): the 146 proofs, the `slope prove` lines with the process's cycles and instructions.
* [variants.jsonl](variants.jsonl): the `matroid prove` lines of the variants table, each with its variant.
* [chain.jsonl](chain.jsonl): the chain 1301-2175 as `taxman solve 1301-4095 --use-prev --threads 2 --out` wrote it,
  moves included, and the unfinished line of 2176.
* [roots.jsonl](roots.jsonl): `matroid roots` of 1301-2175.

The chain 1-1300 and the scaling run are not kept: the first reproduces `results/strict.jsonl`, and the numbers of both
are above.  The examples `matroid` and `slope`, with the files of `examples/common/` they included (among them
`meet.rs`, the study's implementation with matroids built afresh at each node, kept as the independent check of the
crate's), were removed from master on 2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example matroid --example slope` in `rust/`.

## Commits

* 679635b: the GF(2) frame matroids and their intersection as a module of the crate.
* da49d8a: cheaper augmentations and splits.
* f1d09e7: branchless pivots, no allocation in the split, knobs for measurement.
* 9a99f34: a greedy lower bound before a strong-branching solve (with Theorem 10 in the theory).
* 7af8b0e: `gf2` on by default, `lagrange` off.
* 8753189: the last commit that has the examples `matroid` and `slope`, removed from master after it.
