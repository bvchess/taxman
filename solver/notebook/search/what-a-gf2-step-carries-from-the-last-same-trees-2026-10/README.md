# What a GF(2) step carries from the last, same trees (2026-10)

Part of the [lab notebook](../../README.md).

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

## Files

* `prof.diff`: the counters by kind of level and by the outcome of each search, with the parent's cycles checked in
  the child (on 752071f, a scratch build, not kept), which gave the split of the time above.
* `runs.jsonl`: every run of this round, one line a run: the build's label, the unit, steps, CPU seconds, best,
  cycles and instructions a step (`summ.py` of the scratch scripts, from `tune solve` under `/usr/bin/time -l`).
* `profile_3441.txt`, `profile_4548.txt`: the parts of a step and the samples by function, before the round (3441)
  and after it (both), from the traces with `tp.py`.
* `tp.py`: reads an `xctrace export` of a Time Profiler trace and sums the samples by function, self or inclusive,
  or under a function by the next frame (`tp.py EXPORT.xml self|incl|children FUNCTION`).

*Commits*: 1735e60 (the cycles kept), 28c5669 (the split started), and this entry.
