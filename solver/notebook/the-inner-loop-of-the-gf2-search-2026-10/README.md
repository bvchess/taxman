# The inner loop of the GF(2) search (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The GF(2) bound cut the trees about 400-fold, but a node cost 9 to 13 times a node of the matching
path: 340k cycles at the proof of 1287, of which the intersection's shortest paths took 57%, the levels' own work
22% and the pivots 8%, and strong branching made 70% of the augmentations, 22 to 32 a solve.  How much cheaper can
a node be made: fewer augmentations in strong branching, cheaper splits, less level work?  Which changes keep the
trees, and which change them for the better?  And what does it take for the search to use many threads well
(Part 2, a design: the owner moved the many-thread measurements to EC2)?

*How it was measured.*  One-thread proofs (`examples/matroid.rs prove`, the incumbent the optimum as a value, no
seed or ceiling), in cycles and instructions of the process, one process at a time on the M1 Max (other agents'
jobs kept the load at 3 to 30: cycles moved by a few per cent from run to run, instructions far less); the quick set
of 12 proofs (936, 990, 992, 1024, 1100, 1184, 1215, 1232, 1254, 1287, 1300, 1350) after each step, the 146 proofs of
the slope study and the dearest games past 1300 (1430, 1472, 1475, 1521, 2170, 2172) at the start and the end; the
probe (`perf --tactic gf2=on`); sampled profiles by source line (Instruments' Time Profiler, `xctrace record
--launch`, every sample's address symbolicated with its inlined frames by `atos -i`; the scripts were scratch and
are not kept: `tools/profile/attribute.py` does the same from `sample` output, coarser).  The window's fingerprint
used to fold in the GF(2) bound's counts of solves and augmentations, which a cheaper bound changes with the same
tree: it now counts them as zero (30f600f; the matching path's fingerprint is unchanged, the GF(2) probe of feba6d0
is 87991b2903990d65 so measured).

*The key fact.*  In every one of the 10,183 strong-branching bounds of the proof of 1215, the heaviest common
independent set without the member `x` had the size of the optimum `I*` and differed from it by 1 to 15 elements
besides `x`: `I*` flipped along one cycle of its exchange graph through `x` (66%: a cycle; 20% and 14%: a path to
`x` from a source or from `x` to a sink, which close into cycles through the arcs every source has from every member
and every sink to every member).  The solves found the same values, from the uncontended prefix (4 or 5 elements
when `x` is heavy) up, 22 augmentations on average and up to 62.  So the bound without a member needs no solve: the
shortest cycle through `x`, by Dijkstra's search over the weight split's reduced lengths (never negative), and a
proof that nothing of another size does better.  The proof (`src/gf2/meet.rs`, "The heaviest set without a
member"): `I* Δ J` for any set `J` without `x` splits into cycles and paths of the exchange graph (Schrijver,
Corollary 39.12a, in each matroid), all of non-negative length under the split but the part through `x`; if `|J| =
|I*|` that part is a cycle; a smaller `J` loses at least the shortest path between members (exactly what `I*` gains
over a dominant set one smaller, where the history has one), a larger one at least the gap to the heaviest set one
larger (the solve's last shortest path).  The flipped set is checked common independent (it always was).

*The steps* (quick set, one thread; a tree change is marked; commits on the branch of this entry):

| step | commit | nodes | G cycles | G instructions |
|---|---|---|---|---|
| before (feba6d0) | | 218,728 | 76.67 | 278.5 |
| strong branching reads the certified bounds; solves only the chosen member, so that the node goes on from the same level | 52e6512 | 218,728 | | |
| the weight split by Bellman–Ford on a frontier (only the arcs out of vertices whose potential fell) | 0c121c5 | 218,728 | 48.42 | 155.9 |
| the chosen member's certified level kept, not solved again (tree change: ties between heaviest sets) | 15cca11 | 217,625 | 37.85 | 125.9 |
| a child starts from its node's level as it was before it dropped the branched member; what smaller sets lose by a search between members (tree change) | 10243b6 | 217,487 | 34.47 | 108.9 |
| what smaller sets lose from the history when it has a dominant set one smaller | 48987ee | 217,487 | 33.56 | 107.6 |
| a child that loses one member of its parent's optimum gets its optimum from the parent's exchange graph (tree change) | 44c05fc | 217,374 | 30.08 | 92.4 |
| a floor on what smaller sets lose, carried along, made exact only when it does not decide | f84bb89 | 217,374 | 26.43 | |
| forced members by the exchange graph (tree change) | 87447e5 | 89,943 | 16.02 | 48.2 |

What each did, beyond the table:

* *Certified bounds without a member* (52e6512).  `Meet::without`, called for every member of the conflict before
  any solve spoils the tableau; a solve only where it is not certified (53 solves at the proof of 1287 before the
  last step).
  The decisions are the solves' (an exact value leads to the same choice and the same forced members as a solve
  stopped at any `stop`), and the chosen member was solved again for the node's level: the same trees, 1287 14.03 to
  9.60 G cycles, 1215 3.86 to 2.42.  The probe has a kernel for it (the split, then the bounds certified or solved:
  83k cycles a member at 936 and 147k at 1215 against 201k and 379k solved, 100% and 92.9% certified, the same
  fingerprint as the solves').
* *The split on a frontier* (0c121c5).  The potentials are the shortest distances from a virtual source, the same in
  any order of relaxation: the same split, trees and forced members.  A split cost 75k cycles at the probe's nodes
  of 936 and 114k at 1215, then 46k and 77k (5.1 rounds of every constraint, then 3,400 relaxations a split at 1287).
* *The kept level* (15cca11).  The certified set is the heaviest of all sizes without the member, so the node can go
  on from it with no solve; where several sets are heaviest the one kept may differ from the solve's, and the tree
  with it: 9 of 12 quick trees by −2.1% to +0.3%.
* *The node as it was* (10243b6).  The searcher drops the branched member from the node before it pushes the child
  that promotes it, so the child found no dominant set holding the member and solved from the uncontended prefix
  (10,900 solves of 24 augmentations at 1287, the bulk of what was left).  The level before the drop is kept aside
  for the child (swapped out with the kept level, or its sets copied).
* *Certified children* (44c05fc).  A child that promotes the branched member and loses one other member of the
  optimum (a conflict completed, a frame narrowed): the child's sets with its promotions are the parent's sets that
  hold them and avoid what it dropped, so the same certificate applies with those elements banned from the cycle.
  Solves at 1287: 7,709 to 1,202.
* *A floor on the smaller sets' loss* (f84bb89).  Levels made without a solve have no dominant set one smaller; the
  search between members was a fifth of the time.  A floor is carried (a child that inherits keeps its parent's, a
  certified level the parent's less its cycle) and made exact only for a cycle longer than it; the searches go as
  far as any cycle could certify (the gap to the larger sets, the lightest member).  The same decisions and trees:
  1287 5.53 to 4.46 G cycles.
* *Forced members by the exchange graph* (87447e5).  The split's bound without a member is loose (half the members
  it leaves free that turn out forced are within 15 of `need`).  A member is forced when no cycle through it is
  within the slack `value − need` and the other sizes lose more (`Meet::has_cycle_within`, stopped at the first
  cycle found).  Every member searched: nodes 217,374 to 85,349 but 18.0 G cycles; the members whose split bound is
  within `FORCE_WINDOW` of `need` only: window 25 20.6 G, 40 18.2, 60 17.3, **80 16.9**, 120 17.6; a budget on the
  search (16 to 64 vertices) did no better.

*Results.*  The 146 one-thread proofs of the slope study, before (feba6d0) and after (87447e5):

| band | games | nodes before | nodes after | G cycles before | after | ratio | k cycles a node before | after |
|---|---|---|---|---|---|---|---|---|
| 851-900 | 6 | 1,267 | 490 | 0.2 | 0.1 | 0.43 | 173 | 192 |
| 901-950 | 16 | 30,295 | 9,390 | 6.0 | 1.7 | 0.29 | 200 | 186 |
| 951-1000 | 16 | 123,724 | 46,663 | 35.9 | 8.2 | 0.23 | 291 | 177 |
| 1001-1050 | 19 | 154,215 | 65,189 | 44.1 | 10.5 | 0.24 | 286 | 161 |
| 1051-1100 | 18 | 5,862 | 2,535 | 1.8 | 0.6 | 0.30 | 315 | 220 |
| 1101-1150 | 17 | 4,798 | 2,225 | 1.5 | 0.5 | 0.31 | 310 | 207 |
| 1151-1200 | 16 | 18,014 | 7,798 | 5.8 | 1.5 | 0.26 | 322 | 197 |
| 1201-1250 | 19 | 106,869 | 49,688 | 41.4 | 11.1 | 0.27 | 388 | 223 |
| 1251-1300 | 16 | 164,180 | 71,184 | 59.0 | 12.4 | 0.21 | 359 | 174 |
| 1301-1352 | 3 | 88,122 | 31,939 | 35.7 | 5.4 | 0.15 | 405 | 168 |
| all | 146 | 697,346 | 287,101 | 231.5 | 52.1 | 0.225 | 332 | 181 |

Instructions 850 to 155 G (×0.18); per game, geometric means ×0.28 in cycles and ×0.42 in nodes.  log10 cycles
against n: +0.242 ± 0.051 per 100 n before, +0.187 ± 0.048 after; nodes +0.176 ± 0.052 and +0.190 ± 0.051 (the node
count's slope unchanged within its error, the cost of a node no longer growing with n here).  The dearer games gain
more:

| game | nodes before | after | G cycles before | after | ratio | seconds after |
|---|---|---|---|---|---|---|
| 1430 | 646,005 | 308,209 | 328.2 | 82.6 | 0.25 | 26.6 |
| 1472 | 1,277,064 | 512,103 | 826.8 | 171.0 | 0.21 | 55.0 |
| 1475 | 1,478,700 | 581,225 | 921.4 | 164.4 | 0.18 | 52.8 |
| 1521 | 269,058 | 93,420 | 168.5 | 26.0 | 0.15 | 8.4 |
| 2170 | 1,430,720 (ladder `--seed-opt`, 588 s; Graviton calibration) | 519,394 | | 202.1 | | 65.0 |
| 2172 | 1,432,206 (603 s) | 516,133 | | 197.4 | | 63.8 |

The probe (`perf --tactic gf2=on`): a step of the window of 936 192.6k cycles before and 161.4k after, and the window
ends after 806 steps instead of 1,834 (130 M cycles in all against 353 M); 1215 330.7k and 183.7k, 3,972 steps
instead of 12,633 (0.73 G against 4.18 G).  Its fingerprint went 87991b2903990d65 (feba6d0, the window's counters
of work left out), 016eada3371203df (the certified kernel added), 16f83ac3a8ce6080, 455038e9314a2486 and
f3deaac0dbdc030a with the tree changes; the steps with the same trees kept it.

*Where the time goes now* (the proof of 1521, one thread, sampled): the searches for forced members a quarter, the
weight split a quarter, the solves that are left about 28% (their shortest paths 18%, the tableaux they build 8%;
mostly resolves, the level's heaviest set solved again from itself so that the workspace holds its tableau), the
cycles of strong branching 11%, the peels 4%.  A node costs 181k cycles on average over the 146 proofs.  What would
cut more: a cheaper forcing test (a necessary condition first: a member on no cycle of the subgraph of arcs within the
slack is forced without a search; or the search shared between members), a split warm-started from the parent's (a
tree change: another split gives other bounds), and tableaux derived from the parent's instead of built again
(contraction of members is clearing their slots, a deletion leaves the lists; a level's tableau kept for its return).

*Soundness.*  `cargo test --release` passes, with two new tests and one extended: `meet::tests::
heaviest_set_without_a_member_against_brute_force` (4,000 random problems: what the smaller sets lose, the gap, and
every certified bound without a member against brute force, at every size limit; debug assertions on), and the
random-path test of the levels, which now branches as strong branching does and checks every certified bound (3,579)
and certified child (228) and every forced member (53,634) against matroids built afresh; three tests whose searches
had become too short to stop were given longer ones (51e354e).  The tripwires were moved for the tree changes (936
from 2201 to 665 nodes, 1000 from 5386 to 2657).  `taxman regress --to 1000` on one thread: every score, 15.4 s.
The chain 1..1300 on one thread (`taxman solve 1-1300 --threads 1`, to a scratch file): 63 s of wall time, 34.4 s of
search and 402,205 nodes; every score, method and proof equal to `results/strict.jsonl`'s (trivial 13, prime 211,
`cap` 115, exhausted 416, same search 545), the move lists of 1,291 of the 1,300 games identical (9 are other
optimal sets), and `verify-results` and `tools/check_results.py` accept it.  Clippy and fmt clean.

*Tried and not kept.*

* Starting the solve without `x` from `I* − x`, certified dominant by a size-limited weight split: `I* − x` is the
  heaviest of its size at 5 of the 10,183 members of 1215 (removing a heavy member is beaten by removing a light
  one), so this start is almost never valid.
* Certifying the cycle by excluding every path with `x` inside it (two Bellman–Ford searches, from and to `x`, and
  lower bounds on paths through `x` between members and from a source to a sink): it certified all 10,183 at 1215,
  but cost 28% of the time at 1287; the size bounds with one Dijkstra's search certify 99.2% for far less.
* Bounds on what other sizes lose carried along the levels and never recomputed: they decay along certified levels
  (each loses its cycle's length), and strong branching fell back to 3,083 solves at 1287 against 53.
* The split's first round from the elements outside the set only (no arc out of a member or the zero node is
  negative from potentials all zero): +3% instructions, because the in-place round loses the members' propagation.
* A budget on the forcing search (16, 32, 64 vertices settled): no better than the window.

*The fixed time of a game* (a profile of `taxman solve 2126-2135 --threads 1` on a copy of `results/strict.jsonl` cut
at 2125, M1 Max).  Each game that is not a prime spends about 0.5 s outside the frames and the search at this size
(0.63 s in all for a repeat; 0.10 s for a prime): within a game, building the move list from the promotions
(`accelerate::solution_for_promotions`) 45% of the samples, the frames 22%, the local search on the starting
incumbent (`Neighbourhood::improve`) 30%, the repeat lookup's search problem 8% (they overlap).  The run's start-up,
checking the 2125 games of the results file (each repeat's search problem built again), took 77% of the run's samples
(43 s on Graviton4).  With searches 3 to 9 times cheaper, these now weigh more in a chain: section *Many threads*
below.

*Many threads: what the measurements say, and a design.*  Not measured here: the owner moved the many-thread work to
EC2, and the Graviton calibration (the entry before) has since measured the proof of 2172 (1.43 M nodes before this
round) at 15.1 times on 16 threads and 58.5 on 64 (c8g; the engine 98% effective, the rest a tree 3% larger and a node
rate a thread of 93 to 94%).  The poor scaling of the frontier proofs on 4 threads (2.9 times) was a small-tree
effect: proofs of 0.5 to 5 s.  This round makes trees 2 to 3 times smaller and nodes 2 times cheaper, so for a given
n the searches are shorter, and the chain's throughput on many threads is decided by what is serial in a game and by
short searches, until the wall, where trees are large again.  What matters, in order:

1. *The serial time of a game* (0.6 s at 2126 on one core, growing with n: the frames take 0.017 s at 1300 and 0.12
   s at 2126; the move list, the local search) against a search that is now often a second or two on 64 threads: a chain
   on 64 threads spent 43 s of 900 on start-up and about 0.6 s a game outside the search.  Changes: (a) pipeline the
   chain: game n + 1's setup (frames, problem, repeat lookup) needs only n's promotions and score, so it can run while
   n's move list is built and checked, and n + 1's search can start with n's incumbent as its seed while n's proof is
   still running, the ceiling installed when n's score is proven (the ceiling only stops a search; a result whose proof
   uses it records that it assumes n, as `assumes` does now); (b) make the parts cheaper: the move list from the
   promotions (45%) is built from scratch each game, and the local search can stop at once when the seed reaches the
   ceiling; (c) skip the start-up's rebuilding of every repeat's search problem (a digest check, or check only the games
   the run will use).  Several consecutive games at once, each with the provisional n − 1 as its seed, are the general
   form of (a); the results' provenance rules hold if a game's proof is recorded only after the games it rests on are
   proven.
2. *Short searches on many threads*: the ramp-up (all threads busy within milliseconds on the matching path; to be
   measured on GF(2) trees, whose unit restarts are dearer: a unit's level is solved from the uncontended prefix) and
   the end of a search, where threads idle while the last subtrees finish.  Changes: start a game's search on fewer
   threads and let the others begin the next game (games side by side for the short ones, one game on all threads for
   the long ones, by a running estimate of the tree, as Knuth's estimate would give), and warm-start a unit's level
   from its parent's history carried in the unit.
3. *The node rate at 64 threads* (93 to 94% a thread): each thread learns its own nogoods and rebuilds units; with
   the GF(2) bound a unit restart is a solve from scratch (tens of augmentations, a few nodes' worth).  Measure
   `Counters` (unit starts, rebuild time, lock wait) at 16 and 64 threads first.
4. *Parallelism inside a node* is no longer worth it: strong branching's solves, the work that idle threads could
   have shared (22 to 32 independent solves a node), are now cycles of a few microseconds, and what is left in a node
   (the split, the forcing searches) is a few dozen small Dijkstra's searches.

A day on 192 CPUs: with the calibration's 58.5 times at 64 threads and the large trees of the wall, a proof scales;
the chain does as well as its serial part allows, and at this round's cost a game past 2175 averages a few seconds
of search on one thread (2170 and 2172, the dearest of 2151-2175, 65 s), so throughput is set by item 1.  The
measurements to make on a 64-vCPU instance are in the report of this round (the thread scaling of 1472, 1475, 2170
and 2172 at 1, 4, 16, 32 and 64 threads with `--seed-opt` and with `--no-seed --no-cap`; the chain from 2176 for 30
minutes at 64 threads; the fixed time profiled at 2200; the engine's counters).

## Later (2026-10): the program removed from master

The example `matroid`, with the files of `examples/common/` it included, was removed from master on 2026-10-17.  It
builds at commit 8753189: `git checkout 8753189`, then `cargo build --release --example matroid` in `rust/`.
