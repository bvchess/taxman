# The inner loop of the GF(2) search (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  The GF(2) bound cut the trees about 400-fold, but a node cost 9 to 13 times a node of the matching path:
340k cycles at the proof of 1287, of which the intersection's shortest paths took 57%, the levels' own work 22% and the
pivots 8%, and strong branching made 70% of the augmentations, 22 to 32 a solve.  How much cheaper can a node be made,
and which changes keep the trees, or change them for the better?  **Answer.**  Much cheaper.  Reading the bound without
a member off the exchange graph, certifying levels instead of solving them, and finding forced members by cycles cut the
146 one-thread proofs of the slope study from 231.5 to 52.1 G cycles (×0.225) and from 697,346 to 287,101 nodes, and the
dear games from 1430 to 2172 by 4 to 9 times in cycles.

## Method

The measurements are one-thread proofs (`examples/matroid.rs prove`, the incumbent the optimum as a value, no seed or
ceiling), in cycles and instructions of the process, one process at a time on the M1 Max.  Other agents' jobs kept the
load at 3 to 30, so cycles moved by a few per cent from run to run and instructions far less.  After each step the quick
set of 12 proofs ran (936, 990, 992, 1024, 1100, 1184, 1215, 1232, 1254, 1287, 1300, 1350); the 146 proofs of the slope
study and the dearest games past 1300 (1430, 1472, 1475, 1521, 2170, 2172) ran at the start and the end.  The probe was
`perf --tactic gf2=on`.  The window's fingerprint used to fold in the GF(2) bound's counts of solves and augmentations,
which a cheaper bound changes with the same tree; it counts them as zero (30f600f).

## Results

**The key fact.**  In every one of the 10,183 strong-branching bounds of the proof of 1215, the heaviest common
independent set without the member `x` had the size of the optimum `I*` and differed from it by 1 to 15 elements besides
`x`: `I*` flipped along one cycle of its exchange graph through `x`.  In 66% of the bounds the flip was a cycle, and in
20% and 14% a path to `x` from a source or from `x` to a sink, which close into cycles.  The solves found the same
values from the uncontended prefix, with 22 augmentations on average and up to 62.  So the bound without a member needs
no solve: it is the shortest cycle through `x`, by Dijkstra's search over the weight split's reduced lengths, which are
never negative, and a proof that nothing of another size does better (`src/gf2/meet.rs`, "The heaviest set without a
member").

**The steps** (quick set, one thread; a tree change is marked):

| step | commit | nodes | G cycles | G instructions |
|---|---|---|---|---|
| before | feba6d0 | 218,728 | 76.67 | 278.5 |
| strong branching reads the certified bounds | 52e6512 | 218,728 | | |
| the weight split by Bellman–Ford on a frontier | 0c121c5 | 218,728 | 48.42 | 155.9 |
| the chosen member's certified level kept (tree change) | 15cca11 | 217,625 | 37.85 | 125.9 |
| a child starts from its node's level before the drop (tree change) | 10243b6 | 217,487 | 34.47 | 108.9 |
| smaller sets' loss from the history's dominant set | 48987ee | 217,487 | 33.56 | 107.6 |
| a child reads its optimum off the parent's graph (tree change) | 44c05fc | 217,374 | 30.08 | 92.4 |
| a carried floor on the smaller sets' loss | f84bb89 | 217,374 | 26.43 | |
| forced members by the exchange graph (tree change) | 87447e5 | 89,943 | 16.02 | 48.2 |

* **Certified bounds without a member** (52e6512).  `Meet::without` runs for every member of the conflict before any
  solve spoils the tableau, and a solve is made only where the bound is not certified (53 solves at the proof of 1287).
  The decisions are the solves', so the trees are the same: 1287 went from 14.03 to 9.60 G cycles and 1215 from 3.86 to
  2.42.  At the probe 100% (936) and 92.9% (1215) of the bounds are certified.
* **The split on a frontier** (0c121c5).  The potentials are the shortest distances from a virtual source, the same in
  any order of relaxation, so the split, the trees and the forced members are the same.  A split cost 75k cycles at the
  probe's nodes of 936 and 114k at 1215, then 46k and 77k; 1287 went from 9.60 to 8.57 G cycles.
* **The kept level** (15cca11).  The certified set is the heaviest of all sizes without the member, so the node goes on
  from it with no solve.  Where several sets are heaviest the one kept may differ from the solve's, and the tree with
  it: 9 of the 12 quick trees changed by −2.1% to +0.3%.
* **The node as it was** (10243b6).  The searcher dropped the branched member from the node before it pushed the child
  that promotes it, so the child found no dominant set holding the member and solved from the uncontended prefix (10,900
  solves of 24 augmentations at 1287, the bulk of what was left).  Keeping the level from before the drop changed the
  quick set by ×0.91.
* **Certified children** (44c05fc).  A child that promotes the branched member and loses one other member of the optimum
  gets the parent's sets that hold its promotions and avoid what it dropped, with those elements banned from the cycle.
  Solves at 1287 fell from 7,709 to 1,202, and the quick set changed by ×0.90.
* **A floor on the smaller sets' loss** (f84bb89).  Levels made without a solve have no dominant set one smaller, and
  the search between members was a fifth of the time.  A floor is carried and made exact only for a cycle longer than
  it.  The trees are the same, and 1287 went from 5.53 to 4.46 G cycles.
* **Forced members by the exchange graph** (87447e5).  A member is forced when no cycle through it lies within the slack
  `value − need` and the other sizes lose more (`Meet::has_cycle_within`).  Searching every member cut the nodes from
  217,374 to 85,349 but cost 18.0 G cycles.  Searching only the members whose split bound is within `FORCE_WINDOW` of
  `need` cost 20.6 G with a window of 25, 18.2 with 40, 17.3 with 60, **16.9 with 80** and 17.6 with 120; a budget on
  the search (16 to 64 vertices) did no better.

**The 146 one-thread proofs of the slope study**, before (feba6d0) and after (87447e5): 697,346 nodes fell to 287,101
and 231.5 G cycles to 52.1 (×0.225), and the instructions from 850 to 155 G (×0.18).  A node cost 332k cycles on average
before and 181k after.  The ratio of cycles falls with n, from 0.43 in 851-900 and 0.21 in 1251-1300 to 0.15 in
1301-1352.  Per game, the geometric means are ×0.28 in cycles and ×0.42 in nodes.  The slope of log10 cycles against n
went from +0.242 ± 0.051 per 100 n to +0.187 ± 0.048, and that of the nodes from +0.176 ± 0.052 to +0.190 ± 0.051: the
node count's slope is unchanged within its error, and the cost of a node no longer grows with n here.  The dearer games
gain more:

| game | nodes before | after | G cycles before | after | ratio | seconds after |
|---|---|---|---|---|---|---|
| 1430 | 646,005 | 308,209 | 328.2 | 82.6 | 0.25 | 26.6 |
| 1472 | 1,277,064 | 512,103 | 826.8 | 171.0 | 0.21 | 55.0 |
| 1475 | 1,478,700 | 581,225 | 921.4 | 164.4 | 0.18 | 52.8 |
| 1521 | 269,058 | 93,420 | 168.5 | 26.0 | 0.15 | 8.4 |
| 2170 | 1,430,720 (588 s) | 519,394 | | 202.1 | | 65.0 |
| 2172 | 1,432,206 (603 s) | 516,133 | | 197.4 | | 63.8 |

(The seconds of 2170 and 2172 before are the ladder's `--seed-opt` times of the [Graviton
calibration](../../graviton-calibration-2026-10/README.md).)  At the probe a step of the window of 936 cost 192.6k
cycles before and 161.4k after, and the window ended after 806 steps instead of 1,834; for 1215 the figures are 330.7k
and 183.7k, and 3,972 steps instead of 12,633.  After the last step the proof of 1521 spends a quarter of its time in
the searches for forced members, a quarter in the weight split, 28% in the solves that are left (mostly resolves of the
level's heaviest set from itself), 11% in the cycles of strong branching and 4% in the peels.

**Soundness.**  `cargo test --release` passes with two new tests and one extended: `meet::tests::
heaviest_set_without_a_member_against_brute_force` (4,000 random problems, against brute force at every size limit), and
the random-path test of the levels, which checks every certified bound (3,579), certified child (228) and forced member
(53,634) against matroids built afresh.  The tripwires moved for the tree changes (936 from 2201 to 665 nodes, 1000 from
5386 to 2657).  `taxman regress --to 1000` on one thread gives every score in 15.4 s.  The chain 1..1300 on one thread
(`taxman solve 1-1300 --threads 1`) took 63 s of wall time, 34.4 s of search and 402,205 nodes.  Every score, method and
proof equals `results/strict.jsonl`'s, the move lists of 1,291 of the 1,300 games are identical (9 are other optimal
sets), and `verify-results` and `solver/tools/check_results.py` accept it.

**Tried and not kept:**

* Starting the solve without `x` from `I* − x`, certified dominant by a size-limited weight split: `I* − x` is the
  heaviest of its size at 5 of the 10,183 members of 1215.
* Certifying the cycle by excluding every path with `x` inside it (two Bellman–Ford searches): it certified all 10,183
  at 1215, but cost 28% of the time at 1287; the size bounds with one Dijkstra's search certify 99.2% for far less.
* Bounds on what other sizes lose, carried along the levels and never recomputed: they decay along certified levels, and
  strong branching fell back to 3,083 solves at 1287 against 53.
* The split's first round from the elements outside the set only: +3% instructions, because the in-place round loses the
  members' propagation.

**The fixed time of a game** (a profile of `taxman solve 2126-2135 --threads 1` on a copy of `results/strict.jsonl` cut
at 2125, M1 Max).  Each game that is not a prime spends about 0.5 s outside the frames and the search at this size (0.63
s in all for a repeat; 0.10 s for a prime).  Within a game the samples go 45% to building the move list from the
promotions (`accelerate::solution_for_promotions`), 22% to the frames, 30% to the local search on the starting incumbent
(`Neighbourhood::improve`) and 8% to the repeat lookup's search problem (they overlap).  The run's start-up, checking
the 2125 games of the results file, took 77% of the run's samples (43 s on Graviton4).  With searches 3 to 9 times
cheaper, these weigh more in a chain.

## Decision

Kept: every step above, with the tree changes accepted since the scores and proofs hold.  What would cut more is a
cheaper forcing test (a necessary condition first: a member on no cycle of the subgraph of arcs within the slack is
forced without a search), a split warm-started from the parent's (a tree change, since another split gives other
bounds), and tableaux derived from the parent's instead of built again.

**Many threads: a design.**  The owner moved the many-thread measurements to EC2, so this round only designed for them.
With trees 2 to 3 times smaller and nodes 2 times cheaper, the chain's throughput on many threads is decided by what is
serial in a game and by short searches, until the wall, where trees are large again.  The Graviton calibration later
measured the proof of 2172 at 15.1 times on 16 threads and 58.5 on 64, so a proof scales; the poor scaling of the
frontier proofs on 4 threads (2.9 times) was a small-tree effect, proofs of 0.5 to 5 s.  In order of what matters:

1. *The serial time of a game* (0.6 s at 2126 on one core, growing with n) against a search that is often a second
   or two on 64 threads: a chain on 64 threads spent 43 s of 900 on start-up and about 0.6 s a game outside the search.
   Pipeline the chain, so that game n + 1's setup runs while n's move list is built and its search starts with n's
   incumbent as its seed while n's proof is still running; make the move list and the local search cheaper; skip the
   start-up's rebuilding of every repeat's search problem.
2. *Short searches on many threads*: the ramp-up and the end of a search, where threads idle while the last subtrees
   finish.  Start a game's search on fewer threads and let the others begin the next game, and warm-start a unit's level
   from its parent's history.
3. *The node rate at 64 threads* (93 to 94% a thread): each thread learns its own nogoods and rebuilds units, and with
   the GF(2) bound a unit restart is a solve from scratch.  Measure the counters at 16 and 64 threads first.
4. *Parallelism inside a node* is no longer worth it: strong branching's solves are cycles of a few microseconds.

Throughput past 2175 is set by item 1, since a game averages a few seconds of search on one thread at this round's cost
(2170 and 2172, the dearest of 2151-2175, 65 s).  What was built of this design is in [round A, the fixed costs of a
game](../round-a-the-fixed-costs-of-a-game-2026-10/README.md) and [round B, the chain on many
threads](../../round-b-the-chain-on-many-threads-2026-10/README.md).

## Files

* `README.md`: this record.  The folder holds no data.
* The example `matroid`, with the files of `examples/common/` it included, made the measurements.  It was removed from
  master on 2026-10-17 and builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example
  matroid` in `rust/`.
* The profiling scripts (Instruments' Time Profiler, `xctrace record --launch`, `atos -i`) were scratch and are not
  kept; `solver/tools/profile/attribute.py` does the same from `sample` output, coarser.

## Commits

* feba6d0 (2026-10-14): the GF(2) bound merged into the search, the state before this round.
* 30f600f: the window's fingerprint leaves out the GF(2) bound's counts of solves and augmentations.
* 52e6512: strong branching reads the bound without a member off the exchange graph.
* 0c121c5: the weight split by Bellman–Ford on a frontier.
* 15cca11: strong branching keeps the chosen member's certified level (a tree change).
* 10243b6: children start from their node's level before it dropped the branched candidate (a tree change).
* 48987ee: the loss of the smaller sets from the history when it holds a dominant set one smaller.
* 44c05fc: children that lose one member of their parent's heaviest set read theirs off the parent's exchange graph.
* f84bb89: a floor on what the smaller sets lose, carried along, made exact only when it does not decide.
* 87447e5: forced members found by the exchange graph where the split leaves them free (a tree change).
* 51e354e: tests with searches long enough to stop.
* 8753189 (2026-10-17): the last commit that builds the example `matroid`.
