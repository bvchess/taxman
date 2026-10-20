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

*Commits*: 1735e60 (the cycles kept), 28c5669 (the split started), and this entry; in the second round b0082ae (blocks
reduced where the span changes), 854d65d (slots kept), 2e07cd2 (the queue by distance), 994ae69 (no offer past the best
cycle), f22561c (the first round narrowed), and this section.
