# What a GF(2) step carries from the last, same trees (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.

*Question.*  Where do the cycles of a GF(2) step go at the end of the chain, now that the default searches every member
for being forced (`force_window=all`), and what does a step compute again that it could carry from the step before,
with every search tree the same?

*Answer.*  Most of a step went into two searches that a child repeats after its parent: the searches for forced
members (44% of a step at 3441) find again in a child the cycles its parent found, and the weight split (22%, besides
12% for its tableau) runs Bellman–Ford from zero although its parent's distances nearly all hold.  Kept: the cycles
found are kept and checked again before any search, and the split starts from the last split of the same node or of
its parent.  A step falls by 31 to 39% in cycles on five units of 3441, 4225 and 4548, the same trees (every count the
same, the probe's fingerprints unchanged): at 4548, 3,420 to 5,210 steps a CPU-second on one thread.

*How it was measured.*  Units of searches solved to completion on one thread (`tune solve`, `--known
results/strict.jsonl`, the default tactics; the binary built with `--features studies`): three of 3441 from the old
suite ([the tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md), `suite.jsonl`) and two
of 4548 and one of 4225 cut from the checkpoints of the hardest searches of the run to 5000 (another agent's scratch
units files; the checkpoint's incumbent where the optimum is not known).  Cycles and instructions are the process's
(`/usr/bin/time -l`), a step is a step of the search (`nodes`), and the setup (about 0.2 s) is in them; CPU seconds
are the worker thread's.  Two runs of a build agree to 0.5% in cycles and better in instructions.  The profiles are
Instruments' Time Profiler (`xctrace record`, 1 ms samples, every frame with its inlined frames); a split of the time
by the kind of level (inherited, certified, kept) and by the outcome of each search came from counters of the timer
`cntvct_el0` in a scratch build (`prof.diff`).  The laptop (M1 Max) had other load throughout (load 3 to 6).

## Where a step went

Unit 3441-2994 (36,500 steps, 784k cycles and 2.75 M instructions a step), share of the samples:

| work | share |
|---|---|
| searches for forced members (`force_by_cycles`: `has_cycle_within`, Dijkstra over the exchange graph) | 44% |
| the weight split's Bellman–Ford (`split_bounds`) | 22% |
| tableaux derived again for the split (`resolve`: `derive`) | 12% |
| solves (children neither inherited nor certified) | 7% |
| strong branching (`certified_excluding`: the shortest cycle through each member of the conflict) | 6% |
| the child: promotion and peels (`make_child`) | 4% |
| levels pushed (`push_all`) | 3% |
| conflicts and nogood lookups | 1.5% |

The levels evaluated are of three kinds: 11,729 children that inherit their parent's heaviest set, 6,033 certified
children (the parent's set flipped along a cycle) and 11,797 nodes going on without the member they branched on (the
*kept* level, certified the same way), besides 985 solved.  The chain is: a level finds its forced members, the
child promotes them all and inherits the rest of the set, finds none forced, and branches.

* *The searches for forced members.*  A member is forced when no cycle of the exchange graph through it is within the
  slack `value − need`.  In the inherited children 677,315 searches found such a cycle and 496 found none, and 99.8% of
  the members searched lay on a cycle the parent had found that was still a cycle of the child: the child's exchange
  graph is the parent's less the elements it promoted or dropped (contracting a member keeps the others' circuits), and
  no cycle within the slack passes through a forced member, which is what the child promotes.  So the child's searches
  found nothing new, at 17.7% of a step.  A search cost about 8k cycles, whether it found a cycle or proved the member
  forced.
* *The split.*  The split is the shortest distances from a virtual source, unique.  In an inherited child its
  Bellman–Ford visited 2,432 vertices in 14 rounds for 748 live elements, and at the end 12 of them on average had a
  distance other than the parent's (none at all in 23% of the children).
* *Strong branching* was already read off the exchange graph (the inner-loop round): the heaviest set without a member
  is one exchange cycle away, found with no solve, so there is no solve to replace.  The nogood lookups (0.4%, at
  most 36,500 steps of learning on one thread) cost little here.

## What was kept

| unit | steps | before | the cycles kept | and the split started | CPU seconds |
|---|---|---|---|---|---|
| 3441-2994 | 36,500 | 784k | 569k | 530k | 9.20 to 6.16 |
| 3441-5231 | 82,172 | 815k | 539k | 495k | 21.68 to 13.19 |
| 4225-24 | 32,030 | 918k | 671k | 636k | 9.43 to 6.48 |
| 4548-236 | 22,613 | 908k | 654k | 602k | 6.50 to 4.23 |
| 4548-124 | 83,169 | 898k | 649k | 593k | 24.29 to 15.96 |

Cycles a step; instructions fell with them (4548-124: 3.20 M to 2.14 M a step).  Every unit's steps, best and
counters are the same with each change.

*Checks.*  The probe's fingerprints are unchanged (7e8e724717d26545; a58ade4629ab6186 with `--tactic gf2=on`,
f3deaac0dbdc030a with `force_window=80` besides).  The chain 1..1300 on one thread (`taxman solve 1-1300 --threads 1`
to a scratch file) is line for line that of 752071f but for the times and the build: 276,918 steps, 89.6 to 79.8 G
cycles, its searches 25.4 to 21.9 s.  `taxman regress --to 1000 --threads 1` matches every game.  The quick check
passes; its tests have debug assertions, under which every split started is checked against the split from zero and
every member freed by a kept cycle against a search.  The full suite (`cargo test --release --features studies`)
passes but for two tests that do not touch the GF(2) bound: `the_work_list_on_600_to_1300_agrees_with_the_certificates`,
whose failure is in what it reads, not in the search (`certify_plan.py` does not know the proof `known` of the games
from 3915 in `results/strict.jsonl`; neither file is changed here), and the wall test of `tests/window.rs` on the
matching path, a clock of 3 ms that failed once under load and passes alone.

* *The cycles kept* (1735e60).  The searches for forced members keep the cycles they find (`meet::Cycles`, by depth),
  and a search first takes the cycles kept at the parent's depth, its own and the child's, checks each against its own
  tableau (`Meet::cycle_length`: a bit test an arc, where a search settles hundreds of vertices) and its slack, and
  frees the members on those that hold.  A member is free exactly when some cycle within the slack passes through it,
  so the forced members are the same; debug builds check every member freed so with a search.  With the parent's
  depth and its own as sources, the inherited children's searches that found a cycle fell from 677,315 to 1,062, the
  certified children's from 220,384 to 42,097, the kept levels' from 390,407 to 145,014 (the searches that prove a
  member forced stay).  The child's depth as a source took 3.5% more off a step at 3441-2994 (the cycles of the last
  child of a node, checked in the node's kept level).
* *The split started* (28c5669).  Each split keeps its potentials and the predecessor that gave each
  (`meet::SplitStart`), and the next split of the same node, or else of its child, lowers its starting potentials to
  those whose path of predecessors is still a path of its constraint graph, as long: every vertex on it live then and
  now, a member then exactly when a member now (which fixes the lengths of its arcs), every arc still an arc.  Such a
  potential is the length of a path, so no lower than the distance, and Bellman–Ford reaches the same distances from
  it.  About 660 of 750 vertices start from a path that holds, and Bellman–Ford visits 915 vertices a split instead
  of 2,400; the walks that check the paths cost about 4% of a step.  Debug builds compare every split started so with
  the split from zero, and a release build with debug assertions checked every split of 3441-2994 and 4548-124.

*Not kept.*

* *The cycles of strong branching* (the shortest cycle through each member of the conflict) as more candidates: 570.1k
  cycles a step against 571.0k, nothing.
* *Two starts for the split*, the node's last and its parent's, each vertex at the lower: 525k cycles a step against
  500k with one (both before the fix below), since each start's walk costs as much as the visits it saves.
* *A start that knew only the members*, not which elements were live: an element live in the child but not in the
  split it started from (a loop of the parent's, a candidate again) passed the check with what an older split had
  left there, and started below its distance.  4548-124 took 83,166 steps instead of 83,169.  The start now records
  the live elements and the members (`SplitStart::role`); a step at 3441-2994 then takes 530k cycles, where the
  version without it took 500k.

## Where a step goes now

The split and its tableaux are the largest part now, and at 4548 the larger (share of the samples; the files
`profile_3441.txt` and `profile_4548.txt` have the rest):

| work | 3441-2994, 530k cycles a step | 4548-124, 593k |
|---|---|---|
| the split, without its tableaux (`split_bounds_from`) | 23% | 26% |
| of which Bellman–Ford with the walks that start it (`split_potentials`) | 11% | 10% |
| of which the walks (`warm_start`) | 4% | 4% |
| tableaux derived again for the split (`derive`) | 15% | 18% |
| of which blocks built because an element entering is free (`rebuild`) | 6% | 7.5% |
| of which slots renumbered (`renumber`) | 4% | 5% |
| searches for forced members (`force_by_cycles`) | 25% | 18% |
| solves | 10% | 13% |
| strong branching | 8% | 4% |
| the child: promotion and peels | 6% | 9% |
| levels pushed | 4% | 6% |
| the gap of a derived tableau | 2% | 2% |
| nogood lookups and conflicts | 2% | 1% |

## Left for later, with estimates

* *Prove forced members without a search*: a member on no cycle of the subgraph of arcs within the slack is forced
  (one pass for the level, Tarjan's components with the hubs' arcs cut at the slack).  The searches that prove a member
  forced are about 9% of a step; how many such members a component test settles is unmeasured.
* *The first round of the started split* (750 of the 915 visits): in an inherited child every vertex whose path holds
  is at its distance already, so only the arcs into the others need relaxing; their arcs from members of `B`-circuits
  need the columns of `B`.  Perhaps 3-4%.
* *The blocks built in a derivation because an element entering is free* (6-7.5% of a step now), pivoted with the rows
  of the members kept: the last round's fourth idea, larger now that the rest has shrunk.
* *A tighter limit for strong branching's searches*: a kept cycle through the member bounds the length of its shortest
  cycle, and a search bounded there finds the same cycle (every vertex settled below the bound is settled the same).
  Perhaps 1-2% of a step.

## 2026-10-20: the derived tableaux and the cycle searches' queue

*Question.*  After the round above, where do a step's cycles go, what is each of its four candidates (left for later,
with estimates) worth, and what else takes cycles off a step with every search tree the same?

*Answer.*  The tableaux a step derives and the queue of its cycle searches held more than the candidates.  Kept: a block
whose span changes is reduced where its circuits change instead of built; a derived tableau keeps its slot numbers
instead of renumbering every circuit; the searches' radix heap buckets its keys by distance; a search offers nothing
past the best cycle it has found; and the split's first round is narrowed where the arcs are the start's.  A step falls
by 13.5 to 17.4% in cycles and 19.5 to 23.7% in instructions on the five units above, every count the same (3441-2994:
539k to 451k cycles a step).  Of the four candidates, the blocks are reduced rather than pivoted, the narrowed first
round gave 1.5 to 2.2%, a component test would settle three quarters of the searches that prove a member forced but a
pass costs about what they do, and strong branching's bound gave way to the cut at the best cycle found, in every
search.

*How it was measured.*  As above: the units solved to completion on one thread (`tune solve`, `--known
results/strict.jsonl`, the default tactics, built with `--features studies`), cycles and instructions of the process
(`/usr/bin/time -l`), Instruments' Time Profiler at 1 ms for the parts of a step (`tp.py`), and counters of the timer
`cntvct_el0` in scratch builds for what a profile cannot split (`round2-counters.diff`, `round2-components.diff`).  The
laptop (M1 Max) had other load throughout; every run waited for the load to fall to 8.  Two runs of one build on one
unit agree to about 1% in cycles and to 0.1% in instructions, so each change was measured against the build before it in
alternation.

### Where a step went

The base (fd3aa9e; share of the samples, `round2-profile_3441.txt` and `round2-profile_4548.txt` have the rest):

| work | 3441-2994 | 4548-124 |
|---|---|---|
| searches for forced members (`has_cycle_within`) | 21.5% | 14.1% |
| the cycle searches' queue (`Queue`, in these and strong branching's) | 11.3% | 6.7% |
| the split without its tableaux (`split_bounds_from`) | 23.1% | 25.9% |
| of which Bellman–Ford with its walks | 10.4% | 9.9% |
| tableaux derived again (`derive`) | 16.0% | 17.9% |
| of which blocks built (`rebuild`) | 6.4% | 7.7% |
| of which slots renumbered (`renumber`) | 4.8% | 5.2% |
| solves | 10.1% | 12.7% |
| strong branching | 8.9% | 3.9% |
| the child: promotion and peels | 5.6% | 8.5% |
| levels pushed | 3.8% | 5.4% |

Counters at 3441-2994 (4548-124 in parentheses):

* *The searches for forced members.*  303,320 searches proved a member forced (1,027,442), 8.3 a step (12.4), each
  settling 42 vertices (23) for about 7k cycles; 100,992 found a cycle (206,674), each settling 123 vertices (101) for
  about 22k.  They take 11% and 11.5% of a step (7.5% and 7.1%), two thirds of it (three quarters) at the levels a
  node keeps without the member it branched on.
* *The blocks built in derivations.*  11,918 blocks in 29,559 derivations (36,540 in 66,157), 86% of their time in
  blocks where an element that enters stays free after the exchanges, the rest where the numbers that enter and leave
  differ.  Such a block had 538 live elements (417) and 35 members, of which 1.1 left and 1.1 entered; 57% of its
  elements (63%) were free or had a circuit through a member that leaves.
* *The renumbering.*  80,220 blocks renumbered (161,789), 172 elements each with 4.9 slots a circuit; 58% of them only
  dropped slots.  It was 5.0% of a step (5.8%).
* *The split's first round.*  In the children that inherit their parent's heaviest set it took, with its walks, 3.9% of
  a step (4.5%); 98% of their vertices' paths held, and half of those splits had vertices their start had not (9.6 a
  split: the parent's loops, live again in the child).

### The candidates

* *Forced members without a search.*  A member alone in its strongly connected component of the arcs within the slack
  is forced.  Of the members that a search proved forced, 224,179 of 303,320 (74%) are alone (803,336 of 1,027,442,
  78%), and their searches are 48% (61%) of the time of those searches, about 5% (4%) of a step after this round.  But
  a level has 3,170 arcs within the slack (5,570), and a pass that lists them and runs Tarjan's algorithm costs about
  what the searches it replaces cost: the measuring pass took 5.9 (9.3) times their time.  Not built.  A cut with the
  shortest arc into the member (a constant A* bound) was measured too: the searches that prove a member forced settle
  24% (22%) fewer vertices, but finding that arc costs a scan of the member's block of `B`, about what it saves.  Not
  built.
* *Blocks built because an element entering is free* (`Meet::derive`).  Kept, in a better form than pivots: where a
  block's span changes, an element whose circuit holds no member that leaves keeps it (the circuit lies in the new set),
  and only the others are reduced by the new members, as the elements the source tableau did not have already were.  A
  derived tableau's residues are read only for being zero, so a reduction's residue will do.
* *The split's first round* (`Meet::split_bounds_from`).  Kept.  Where the level's set is its start's less the members
  promoted since (checked by the roles the start recorded and its number of members), the arcs among the vertices the
  start had are the start's.  The walks then check only that the vertices are live, and a potential whose path holds is
  the distance among those vertices, so no arc between two of them is relaxed: the first round pulls each other vertex's
  potential in along its arcs, and relaxes the arcs out of those vertices.  It narrows 11,715 of the 11,729 inherited
  children's splits at 3441-2994, whose time falls from 5.6 M to 3.2 M ticks of the timer.  The vertices the start did
  not have make the round need the arcs out of the broken vertices, not only into them.
* *Strong branching bounded by a kept cycle.*  Not built; in its place every cycle search (strong branching's too) no
  longer offers a distance at or past the best cycle it has found, which it would end before taking.

Two parts larger than the candidates were found in the profile:

* *The renumbering* (5% of a step).  Slots only name members: the searches order their vertices by distance, arcs and
  element, and what else is read off a derived tableau (the split, the gap) is unique, so its numbering does not
  matter outside a solve, which builds its tableau anew (only the split's predecessors, which choose where a later
  split starts, can differ).  A derived tableau now keeps its slots: a member gone frees its slot, whose bit leaves the
  circuits, and a member that enters takes a free one.  The tests compare circuits as sets of members.
* *The queue* (11.3% of a step at 3441-2994).  Its keys are a distance above a vertex, and the radix heap spread the
  keys of one distance over the buckets of their vertices' bits, redistributing at nearly every pop.  It now buckets by
  distance alone and sorts the keys of one distance once; they come out in the same order, which a test checks
  against a binary heap.

### What was kept

Each change against the build before it, 3441-2994, cycles a step (alternating runs):

| change | from | to |
|---|---|---|
| the split's first round narrowed (measured last, on the final code) | 460.6k | 450.8k |
| blocks reduced where the span changes, not built | 532.4k | 514.2k |
| slots kept | 510.5k | 494.0k |
| the queue by distance (both with the closing below, not kept) | 493.6k | 467.3k |
| no offer past the best cycle | 462.6k | 455.1k |

The five units, base (fd3aa9e) against the round, two runs each:

| unit | steps | cycles a step | instructions a step | CPU seconds |
|---|---|---|---|---|
| 3441-2994 | 36,500 | 539.0k to 450.8k | 1,868k to 1,495k | 6.17 to 5.06 |
| 3441-5231 | 82,172 | 512.8k to 423.8k | 1,750k to 1,370k | 13.43 to 10.96 |
| 4225-24 | 32,030 | 650.5k to 542.5k | 2,346k to 1,790k | 6.51 to 5.33 |
| 4548-236 | 22,613 | 615.7k to 527.0k | 2,223k to 1,747k | 4.28 to 3.55 |
| 4548-124 | 83,169 | 604.7k to 523.0k | 2,135k to 1,719k | 15.98 to 13.63 |

At 4548 a CPU-second takes 5,205 steps before and 6,102 after.  Every unit's steps, best and counters are the same.

*Not kept: a cycle closed as soon as it is reached.*  A search for a cycle within the slack could stop when it offers
a distance to an element with an arc to the member, before settling it: exact, since the distance offered is a path's.
It made a step dearer, 466.7k cycles against 452.9k, because the cycles it keeps for later levels are longer, and
free fewer members there.

*Checks.*  The chain 1..1300 on one thread (`taxman solve 1-1300 --threads 1` to a scratch file) is line for line that
of master (be4c92b) but for the times and the build: 276,918 steps, 80.7 to 76.3 G cycles, its searches 22.1 to 20.7 s.
`taxman regress --to 1000 --threads 1` matches every game.  The probe's fingerprints are unchanged (7e8e724717d26545;
5b83bbebf0bb2899 with `--tactic lagrange=off`; a58ade4629ab6186 with `--tactic gf2=on`, f3deaac0dbdc030a with
`force_window=80` besides).  A release build with debug assertions, under which every split started is checked against
the split from zero and every member freed by a kept cycle against a search, solved 3441-2994, 4225-24 and 4548-124
with the same counts.  The tests check every derived tableau against a solve's, circuits as sets of members (the long
random paths of the levels too), and the radix heap against a binary heap.  The full suite (`cargo test --release
--features studies`) and the quick check pass.

### Where a step goes now

The split and the cycle searches are the largest parts, and none of the rest is above 15% (share of the samples):

| work | 3441-2994, 451k cycles a step | 4548-124, 523k |
|---|---|---|
| searches for forced members | 19.6% | 12.7% |
| the cycle searches' queue | 9.3% | 5.3% |
| the split without its tableaux | 26.7% | 28.7% |
| of which Bellman–Ford with its walks | 11.2% | 9.9% |
| of which the columns of `A` (`build_columns`) | 4.9% | 6.7% |
| tableaux derived again | 8.8% | 9.5% |
| solves | 12.3% | 14.6% |
| strong branching | 7.0% | 3.4% |
| the child: promotion and peels | 6.8% | 10.4% |
| levels pushed | 3.8% | 6.4% |
| the gap of a derived tableau | 2.7% | 2.6% |

The searches that prove a member forced, three quarters of which a component test would settle, are the clearest
prize left if a cheaper test than a pass over the arcs within the slack is found; the split's pass over the circuits
for the bounds without each member (`split_bounds_from` less Bellman–Ford and the columns, about 10%) is the next
largest part not yet studied.

## 2026-10-20: the loops that come back

*Question.*  The counters of the round above found, in half of the inherited children's splits, vertices that the start
had not, 9.6 a split: the parent's loops (elements whose vector is zero in a matroid, so that no common independent set
holds them), live again in the child.  Why do they come back, what do they cost, and does leaving them out change a
tree?

*Answer.*  A child's live candidates are rebuilt from the search's candidates, which know nothing of loops, so the loops
of every contraction above the child return (`Levels::push_all`).  A split holds 52 of them among 745 live elements at
3441-2994 and 67 among 1,011 at 4548-124, and they cost 4.5% of a step (3.0 to 5.4% on five units).  Leaving them out
is sound and takes one line, but it changes the trees of 25 of the games 1..1300 (+178 steps of 276,918): the weight
split's bounds without a member differ, which reorders the members that strong branching tries, and where two tie it
chooses the other.  The saving is under the 5% that a change which alters trees has to reach to be measured against the
10 to 25% by which the size of a tree varies from game to game, so it is not kept (`loops-drop.diff`).

*How it was measured.*  As above: the five units on one thread (`tune solve`, `--known results/strict.jsonl`, the
default tactics, built with `--features studies`), the cycles and instructions of the process (`/usr/bin/time -l`), the
laptop (M1 Max) waited to a load of 8 before each run.  The counts come from a scratch build (`loops.diff`) that counts
the loops in each pass of the split and the derivations, times those passes with the timer `cntvct_el0`, leaves the
loops out of the live set (`LOOPS_DROP`), computes beside each split the split of the same tableau without the loops
(`LOOPS_SHADOW`), and writes the value, heaviest set and forced members of each evaluation and the member each node
branches on (`LOOPS_TRACE`; `loops_trace.py` finds the first difference of two traces).  The speed comes from clean
builds, master (384ee7b) and master with `loops-drop.diff`, run in turn, three rounds.

### Why they come back

`Level::live` holds the candidates that are elements, with the loops left out, and `contract` takes out the candidates
whose vectors fall to zero when a member is promoted.  `reset` contracts every promoted member from the root's vectors,
so a unit's root is right.  `push_all` sets the child's live candidates to `live_of(child_cands)`, the candidates that
are elements of the root's matroids, and then contracts the child's own promotions.  The loops those make leave, but the
loops that the parent's level had taken out are live again, with the zero vectors that the child copied from the
parent.  So a level holds the loops of every contraction above it but its own.  A tableau derived from the parent's
meets the loops of the parent's contraction as elements the parent's tableau did not have (`fresh` in `Meet::derive`,
role 0 in `SplitStart`), which the round above saw as 9.6 vertices in half of the splits.

The search never drops a loop by hand: none of the 12,153 candidates removed from a node at 3441-2994 was one.  So a
child's live candidates are exactly the parent's that it keeps, less the loops of its own contraction.

### What they cost

Counts of a split (the build of `loops.diff`, loops live), over the whole unit:

| | 3441-2994 | 4548-124 |
|---|---|---|
| live elements | 745 | 1,011 |
| loops | 52.4 (7.0%) | 67.1 (6.6%) |
| loops zero in A only, in B only | 33.5, 18.8 | 42.4, 23.3 (1.4 in both) |
| loops that the parent's split lacked | 3.0 | 9.2 |
| Bellman–Ford vertex visits (without the loops) | 629 (592) | 638 (590) |
| arcs offered, from and into loops | 3,666, 1.1% and 0.8% | 3,646, 0.9% and 1.0% |
| steps of the walks that start it (without the loops) | 747 (694) | 1,013 (946) |

A loop has arcs in one matroid only, so the relaxations of the split barely feel the loops (under 2% of the arcs); the
vertices and the walks do (6 to 8%).  The split takes 31.5% of the unit's CPU with the loops and 30.2% without them at
3441-2994 (34.3% and 33.1% at 4548-124), the derivations 8.8% and 8.5% (9.2% and 8.9%).  Of the 0.33 s that leaving the
loops out saves of 5.32 s (0.72 s of 14.56 s), half is in the split (52%; 57%), a seventh in the derivations (13%; 15%),
and the rest in the other passes over the live elements, which were not timed.

The speed of the clean builds, the means of three runs, with every step count equal in the three rounds:

| unit | steps | cycles a step | instructions a step | CPU seconds |
|---|---|---|---|---|
| 3441-2994 | 36,500 | 450.9k to 431.7k (-4.2%) | 1,495k to 1,437k | 5.06 to 4.87 |
| 3441-5231 | 82,172 | 424.0k to 402.2k (-5.2%) | 1,370k to 1,310k | 10.96 to 10.38 |
| 4225-24 | 32,030 | 542.1k to 525.9k (-3.0%) | 1,790k to 1,744k | 5.32 to 5.16 |
| 4548-236 | 22,613 | 526.7k to 501.3k (-4.8%) | 1,747k to 1,674k | 3.56 to 3.39 |
| 4548-124 | 83,169 | 522.8k to 494.4k (-5.4%) | 1,719k to 1,638k | 13.65 to 12.92 |

The mean is 4.5% fewer cycles and 3.9% fewer instructions a step.  On the chain 1..1300 (one thread) the cycles fall by
2.5%, 75.4 G to 73.5 G, the instructions by 1.6%, and the searches from 20.3 s to 19.5 s.

### What removing them changes

On the five units the steps, the best and every counter are the same, but for the augmentations of the solves at the two
units of 3441 (47,057 to 47,001 and 69,386 to 69,305).  A solve starts from the longer of its last dominant set and the
run of the heaviest live elements that is common independent, and a loop among the heaviest elements ended the run: 19
of the 985 solves at 3441-2994.  The start is longer without the loops, and the heaviest set has the same weight.  A
trace of 4548-236 with and without the loops is the same in all 18,739 evaluations (value, heaviest set, forced
members) and 13,202 branch choices.

The weight split is what differs.  Beside each split, the split of the same tableau without the loops:

| unit | splits | splits that differ | bounds that differ, of those compared | higher with the loops | through replacements, through potentials |
|---|---|---|---|---|---|
| 3441-2994 | 30,544 | 3,360 (11%) | 3,599 of 1.94 M | 3,524 | 3,363, 458 |
| 3441-5231 | 66,734 | 1,495 (2.2%) | 2,371 of 4.60 M | 2,255 | 2,168, 585 |
| 4548-124 | 68,714 | 951 (1.4%) | 1,319 of 5.62 M | 1,319 | 1,319, 0 |
| 4548-236 | 18,739 | 83 (0.4%) | 83 of 1.51 M | 83 | 83, 0 |

Two things make a bound without a member differ:

* *The replacements.*  The bound without a member `x` adds the heaviest element of positive weight whose circuit in a
  matroid holds `x`.  A loop of A with a circuit in B counts as such an element in B (a loop of B in A), though it is in
  no common independent set, so the bound without `x` is higher.
* *The potentials.*  A loop of A has no arc into it, since nothing in A bounds its `w1`, but Bellman–Ford starts it at
  potential 0 and relaxes its arcs out, to the zero node if it is free in B and to the members of its circuit in B
  otherwise.  The loop then caps their potentials as if its own were 0, which no constraint of the others asks, so some
  members' potentials change, and some bounds are lower with the loops (75 and 116 bounds in the two units of 3441).

What the search takes from the bounds is less:

* *Forced members.*  The searches for forced members run in 96 to 97% of the evaluations and then give the exact set,
  whatever the split: a member that the split's bound shows forced lies on no cycle within the slack.  The bounds that
  cross `need` (674 members at 3441-2994, 562 at 3441-5231, 150 at 4548-124, 7 at 4548-236) were all in such
  evaluations; in the other 3 to 4% (5,870 evaluations of the four units) none did.
* *Strong branching.*  It tries the members of the conflict in the order of their split bounds and chooses the first
  with the lowest exact bound, so a tie between two members goes to the one the split puts first.  The order differs in
  26 of 72,240 choices on the four units, and the choice with it in none of the 1,756 ties.

The chain finds what the units do not: 25 of the 1300 games change, all from 930 up, 23 of them in their number of
steps (from -2.5% at 1269 to +14.7% at 936, 401 to 460), 178 steps more in all (+0.06%); the best, the proofs and
every other field are the same, and `taxman regress --to 1000 --threads 1` matches every game.  The whole games 940 and
936 (`tune solve GAME --ids root`) reproduce the difference, and their traces first differ in a strong-branching
choice.  At 940 the conflict has two members, 340 and 434, whose bounds without them are equal (9,713).  With the loops
the split's bounds without them are 9,705 and 9,713 (the loops lower the first, which only the potentials can do), and
without the loops both are 9,713; the order of the two, and with it the member chosen, changes.  At 936 it is the same
kind of tie.

### Decision

*Not kept.*  Leaving the loops out saves 4.5% of a step and changes 25 games of the chain by a tie in strong branching,
so it cannot be measured against the swings of the trees, and the chain's total saving is 2.5%.  `loops-drop.diff` is
the change: the child's live candidates are the parent's that its candidates keep, a debug check that they are the
elements of its candidates, and the paragraphs of `gf2-bound.md`.  It is sound, since the loops are in no common
independent set and any weight split of the others bounds them.  A way to keep the trees would have to give the split
the loops' replacements and arcs, which need the circuits that the change saves.  Loops that are a larger share of the
live elements, or a tie-break of strong branching that does not depend on the split's bounds, would reopen it.

## 2026-10-20: the loops left out, adopted

The section above is adopted without a measurement of its own: the project's owner judged its numbers enough, and its
whole-game runs (4050 and 4176) and cycle counts were not repeated.  A child's live candidates are now its parent's that
its candidates keep (`Levels::push_all`, the change of `loops-drop.diff`, with its debug assertion), which saves 4.5% of
the cycles of a step (3.0 to 5.4% on five units, 2.5% on the chain 1..1300) and changes the trees only where strong
branching ties (25 of the games 1..1300, +178 steps of 276,918; every best, proof and other field the same).  The
release suite (with the study tools' tests) passes but for the node-count tripwires of game 936, which move: default
390 to 449, `nogoods=off` 344 to 345, `lagrange=on` 390 to 449, `symmetry=off` 663 to 652, and in the extended table
`default` 363 to 366 and `extended` 663 to 652 (the answers, 20,507, stay).  The probe's fingerprints with `gf2=on`
move, from a58ade4629ab6186 to 38f7687db16be4a7 and, with `force_window=80`, from f3deaac0dbdc030a to 547eda4e1dc3967b,
through the windows of 936 (both) and of 1215 (the second); every kernel's fingerprint, and the probe's other two, are
the same.  The code is in 1cef63c.

## Files

* `prof.diff`: the counters by kind of level and by the outcome of each search, with the parent's cycles checked in
  the child (on 752071f, a scratch build, not kept), which gave the split of the time above.
* `runs.jsonl`: every run of this round, one line a run: the build's label, the unit, steps, CPU seconds, best,
  cycles and instructions a step (`summ.py` of the scratch scripts, from `tune solve` under `/usr/bin/time -l`).
* `profile_3441.txt`, `profile_4548.txt`: the parts of a step and the samples by function, before the round (3441)
  and after it (both), from the traces with `tp.py`.
* `tp.py`: reads an `xctrace export` of a Time Profiler trace and sums the samples by function, self or inclusive,
  or under a function by the next frame (`tp.py EXPORT.xml self|incl|children FUNCTION`).
* `round2-counters.diff`: the second round's counters of the searches for forced members (with the cut by the
  shortest arc into the member), of the blocks built and renumbered in derivations, and of the split's first round (on
  fd3aa9e, a scratch build, not kept).
* `round2-components.diff`: the component test of the members that a search proves forced (on the second round's
  code, not kept).
* `round2-runs.jsonl`: every run of the second round, one line a run: the build, the unit, steps, CPU seconds, best,
  cycles and instructions a step (from `tune solve` under `/usr/bin/time -l`).
* `round2-profile_3441.txt`, `round2-profile_4548.txt`: the parts of a step and the samples by function, before the
  second round and after it, from the traces with `tp.py`.
* `loops.diff`: the scratch build of the section on the loops (on 384ee7b, not kept): counters of the loops in the
  split and the derivations with the timer `cntvct_el0`, the switch `LOOPS_DROP` that leaves the loops out of a
  child's live candidates, the shadow split without the loops beside each split (`LOOPS_SHADOW`; with its parts, the
  replacements and the potentials, and the choice of strong branching under its order), and the trace
  (`LOOPS_TRACE`) of each evaluation and branch.
* `loops-drop.diff`: the change that leaves the loops out (adopted in the last section, in 1cef63c): the child's live
  candidates are the parent's that its candidates keep, a debug check that they are the elements of its candidates, and
  the paragraphs of `solver/docs/design/gf2-bound.md` it changes.
* `loops-runs.jsonl`: the runs of the section, one line a run: the build, the unit, steps, CPU seconds, best, cycles
  and instructions a step (`b0_`, `b1_`: the scratch build with the loops live and out; `ab1_` to `ab3_`: master and
  master with `loops-drop.diff` in turn, from `tune solve` under `/usr/bin/time -l`).
* `loops_trace.py`: the first evaluation or branch that differs between two traces, with the order of strong branching
  and the splits that differ (`loops_trace.py A.err B.err`, the stderr of `tune solve` with `LOOPS_TRACE=1`).

*Commits*: 1735e60 (the cycles kept), 28c5669 (the split started), and this entry; in the second round b0082ae (blocks
reduced where the span changes), 854d65d (slots kept), 2e07cd2 (the queue by distance), 994ae69 (no offer past the best
cycle), f22561c (the first round narrowed), and this section.  The section on the loops (2026-10-20) kept no code;
its notebook files are in its commit.  The loops left out: 1cef63c.
