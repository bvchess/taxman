# Cheaper steps of conflict branching (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-23.  Concluded 2026-10-23.

**Question.**  A step of conflict branching costs about twice a default step on the chain: of a step of the unit
4875-10, 43% is the pricing, one search for the shortest cycle through each member priced, and 39% the GF(2) bound's
solves ([the cost of a step of conflict
branching](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#2026-10-21-the-cost-of-a-step-of-conflict-branching)).
Can a step cost a fraction of that and keep most of the rule's benefit?  A child differs from its parent by the members
it promotes, so many of its prices may be its parent's: how many, and can a child take them with a proof that they are
the same?  And the tactic `conflict_look` adds about 12% to a step: can its look cost less?

**Answer.**  Three quarters of the pricing's searches are at children that inherit their parent's heaviest set, and
there the child's exchange graph is the parent's less what it promoted or dropped, so the parent's shortest cycle
through a member, where it is still a cycle of the child, is the child's: it carried for 99.8 to 100% of the members,
with no exception.  Kept: a child carries those prices, and a check whose bound is below the need skips its weight
split.  With the same trees, a step costs 0.66 to 0.81 of the CPU on units of 4875 and 4914, and the whole game 4050
0.79.  Carrying the other prices too, where it is not proved, saves another 11 to 14% a step but changes the trees, and
the whole games split; not kept.  The look's cost is the GF(2) solve of the child it looks at, 3.6 to 7 times an
evaluation.  Kept: the look bounds its child from the node's weight split first, with no solve, which cuts 86 to 100%
of the children that the solve cuts; with the same trees, units of 4875 and 4950 take 0.79 to 0.95 of the CPU of the
look that solves every child.  A repair of the node's heaviest set by certified cycles would not have cut them: the
children lose `c` and its substitute, and their heaviest set is smaller.

## 2026-10-23: the setup

The work is in a clone on the lab's machine (`~/taxman-lab/work/research-stepcost`, not pushed), at 7dec259.  Each
variant is a build of `tune` with the feature `studies` (`build2.sh SRC TARGET`, at nice 19), kept as
`bin/tune-NAME`.  `unit.sh BIN LABEL UNITS ID [ARGS]` solves one unit on one thread from the known optimum
(`--known` the chain's `strict.jsonl`), and `queue.sh FILE P` runs the lines of FILE P at a time.

Units: 4875-10 of [the fresh units of
4875](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#files) (`units-fresh.jsonl`,
gap 1,166, 45,962 steps), and three units of the lab's snapshot of game 4914 (`snap/4914-20261023T010919Z`): 4914-2825
(depth 60, gap 1,159, 32,047 steps), 4914-10 (depth 10, gap 1,153) and 4914-1240 (depth 54, gap 1,166), the last two
stopped at 60,415 steps (`--node-limit 60000`).  Every run below ended with the known optimum as its best.

CPU seconds on this machine vary by up to two times between runs, since the chain shares its cores, so CPU is compared
only between runs side by side: the two builds on the same unit at the same time, four processes at once, in two
rounds with the order swapped.  The lab's studies of games 4920 and 4921 ran meanwhile, so at most four threads were
used.

## 2026-10-23: how many prices a child carries

**A study of the prices** (`study.patch`, `STEPCOST_STUDY=1`).  At each pricing, the study keeps every member's
shortest cycle found within the reach, and compares the pricing with the one before it on the node's path: the
parent's, at a child's first check, or the node's own last one, at a later check of the same node (after it went on
without the member it branched on).  A child's first check is *inherited* where its heaviest set is its parent's less
the members it promoted.  For each member priced at both, the study asks whether the earlier cycle through it is still
a cycle of this exchange graph, every element live and every arc there, as long (`Meet::cycle_length`), and whether
the search here found a cycle as long.  Timers count the CPU of each search and of a step's parts.

The pricing prices about a hundred members at a check (87 to 130), and each search took 54 to 62 µs on this machine:

| unit | pricing's searches, share of CPU | inherited: share of searches' time | earlier cycle holds | as long | other children: share | holds | as long | same node: share | holds | as long |
|---|---|---|---|---|---|---|---|---|---|---|
| 4875-10 | 45% | 75.2% | 99.8% | 100% | 8.0% | 88.2% | 97.9% | 16.7% | 86.6% | 97.8% |
| 4914-10 | 33% | 72.5% | 99.9% | 100% | 11.3% | 90.2% | 98.9% | 16.2% | 88.8% | 98.2% |
| 4914-1240 | 29% | 71.6% | 100.0% | 100% | 10.4% | 93.5% | 99.3% | 18.0% | 94.9% | 99.0% |
| 4914-2825 | 28% | 73.6% | 99.9% | 100% | 10.0% | 92.7% | 98.7% | 16.4% | 92.5% | 98.6% |

("holds": of the members priced at both; "as long": of those that hold, the search here found a cycle as long, the
others a shorter one.)  Every cycle the searches found checked as a cycle of its own length.  An inherited child
priced no member its parent had not.

**Why an inherited child's prices are its parent's.**  Its problem is its parent's with the promoted members
contracted and other candidates dropped, and its heaviest set is its parent's less the promoted.  Contracting a member
of the set keeps every other element's circuits, and which elements are free, in both matroids, and dropping an
element outside the set deletes it; so the child's exchange graph is the parent's less those elements, with the same
lengths.  Every cycle of the child's graph is then one of the parent's, so a member's shortest cycle in the child is
at least as long as in the parent, and if the parent's shortest cycle is still a cycle of the child, it is the child's
shortest.  The bound without the member, and whether the sizes certify it, follow from that length as after a search.
The study found no exception in 2.1 million comparisons.  At the other checks the set was flipped along a cycle, which
changes the arcs of the blocks the cycle touches, and 1 to 2% of the earlier cycles that hold have a shorter one now:
a carry there would not be exact.

## 2026-10-23: a child carries its parent's prices

**The change** (`Gf2Bound::prices_excluding`, which the pricing calls in place of `certified_excluding`).  Each
pricing keeps, for its level, the node, the heaviest set, the live candidates, and each member's shortest cycle within
the reach with its length.  A pricing first looks for its parent node's: where its own level's promoted members lie in
that pricing's heaviest set, its heaviest set is that set less them and its live candidates are among that pricing's,
a member's cycle there that is still a cycle here (every element live, `Meet::cycle_length`) gives the member's
shortest cycle with no search.  Its bound, and whether the sizes certify it, are found from the length as after a
search, and a member with no such cycle is searched.  Strong branching's own calls do not carry, since the level it
keeps for the node depends on which of several shortest cycles its search finds, where the pricing reads only the
length.  Debug builds check every carried length against a search, and the random paths of the GF(2) tests price every
level both ways.

**Side by side** (`tune-base` 7dec259, `tune-carry` with the change; two rounds, the order swapped):

| unit | steps | CPU s, base | carry | ratio | second round: base | carry | ratio |
|---|---|---|---|---|---|---|---|
| 4875-10 | 45,962 | 133.4 | 89.4 | 0.670 | 136.7 | 90.2 | 0.660 |
| 4914-10 | 60,415 | 121.4 | 93.5 | 0.770 | 121.6 | 92.9 | 0.764 |
| 4914-1240 | 60,415 | 80.5 | 65.1 | 0.808 | 80.2 | 64.2 | 0.801 |
| 4914-2825 | 32,047 | 76.6 | 60.3 | 0.787 | 75.5 | 60.5 | 0.802 |
| all four | | 411.9 | 308.3 | 0.748 | 413.9 | 307.9 | 0.744 |

Every unit took the same steps, found the same best and had the same counters of its results line.  The saving is
what the study predicted, three quarters of the pricing's searches: a third of a step at 4875-10, where the pricing
was 45% of the CPU, and a fifth to a quarter at 4914.

**A bound below the need.**  A check whose GF(2) bound is below the need cuts its node, so it needs no weight split
and no forced members, but `Levels::evaluate` found them anyway.  It now skips them there (`Eval::forced` is then
empty).  It took 2 to 7% of the splits and saved 1 to 2% of a step, with the same trees (`tune-carry2`, both changes,
against `tune-carry`, two rounds side by side: all four units 0.981 and 0.994 of the CPU, every unit lower).

**Whole games** (`tune solve N --ids root`, one thread from the optimum, the two builds side by side):

| game | steps | CPU s, base | carry and the skipped split | ratio |
|---|---|---|---|---|
| 3000 | 107,724 | 38.3 | 35.3 | 0.922 |
| 4050 | 582,353 | 482.2 | 382.9 | 0.794 |

The same steps and best.  Game 3000 has few checks with a gap of 300, so it gains little.

**Tests** (`tests.sh`, in the lab's clone): the GF(2) tests of a debug build, where every carried length is checked
against a search (265 prices carried on the few random paths), and of a release build (5,486 on the long random paths,
every price equal to the bound found without the carry); the tripwires (19, every pinned tree the same) and the
soundness tests (29) in release.  All pass.

## 2026-10-23: where the look spends

**The look's cost** (the study build with `--tactic conflict_look=on`, the same four units; its timers split the look
of `in_child_cut` into the promotion and narrowing, the child's push and GF(2) evaluation, and the conflict check, and
count each cut by its reason):

| unit | steps | looks | cut | by the bound | conflict all forced | look's CPU | the node's tableau again | share of the unit's CPU |
|---|---|---|---|---|---|---|---|---|
| 4875-10 | 38,540 | 669 | 425 | 383 | 42 | 6.6 s | 1.4 s | 6.8% |
| 4914-10 | 60,415 | 1,621 | 820 | 795 | 25 | 11.2 s | 3.3 s | 9.4% |
| 4914-1240 | 60,415 | 410 | 68 | 62 | 3 | 1.1 s | 0.8 s | 2.4% |
| 4914-2825 | 10,425 | 758 | 592 | 587 | 5 | 7.6 s | 1.5 s | 19.6% |

(4914-1240's other three cuts are promotions that fail.)  The look took 4914-2825 from 32,047 steps to 10,425 and
4875-10 from 45,962 to 38,540.  Nearly all of a look is the child's GF(2) evaluation (6.4 of 6.6 s at 4875-10, 10.6
of 11.2 at 4914-10), 3.6 to 7 times an evaluation at a node of the search (9.6 ms against 1.4 ms at 4875-10): the
look's child leaves out a member of the node's heaviest set and promotes the others of `K`, so it inherits no
heaviest set and its bound is solved.  After the look the node's tableau is made again for
strong branching (`resolve`), a fifth more.  97% of the cuts are by the bound itself; the conflicts all of whose
members are forced give the rest.

In all but 74 of the 3,458 looks, the node's heaviest set without the cheapest member `c`, which the pricing
certified, holds every other member of `K`.  So before its candidates are narrowed, the child's bound is the node's
less the price of `c` (at most 10), far above the need at a gap of 300 or more, and what cuts the child is what the
promotions take away: candidates narrowed out, members of that set among them, and the limit on promotions.  Only a
solve of the child sees that.

A cheaper look would solve the child from that set rather than from scratch: less the promoted members it is still
the heaviest of its size, and only the members the narrowing takes from it need repairing, each by one shortest path,
as a certified child repairs the one member it loses.  Not built.  It could save most of the look's 2 to 20% of the
CPU on these units (an estimate: how many members the narrowing takes from the set was not counted), with the same
bound, so the same cuts.

## 2026-10-23: the gate, with the carried prices

The gate of 300 was chosen when a priced check cost much more, so a lower gate, which prices more checks, may pay now.
`tune-carry2` on whole games at the gates 300, 200 and 150 (the three runs of 4050 side by side, those of 3000 one
after the other beside them):

| game | gate 300: steps | CPU s | gate 200: steps | CPU s | gate 150: steps | CPU s |
|---|---|---|---|---|---|---|
| 3000 | 107,724 | 35.5 | 69,901 | 28.8 (0.81) | 55,270 | 27.4 (0.77) |
| 4050 | 582,353 | 393.5 | 387,868 | 430.4 (1.09) | 369,504 | 498.3 (1.27) |

Without the carry, gate 200 took 1.23 times the CPU of gate 300 on 4050 ([the gate
table](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#2026-10-21-conflict-branching)),
and gate 150 lost on 3200 and 4000 ([gate 150 against
300](../when-a-node-should-use-conflict-branching-2026-10/README.md#2026-10-22-gate-150-against-300-on-whole-games)).
With it, the lower gates win on 3000 and still lose on 4050, by less.  The gate stays 300 here; it is the lab's lever
per game, and the chain's games, whose gaps are larger, are where it would be tried again.

## 2026-10-23: carrying where it is not exact

The other quarter of the pricing's searches is at checks whose heaviest set was flipped along a cycle: a node's later
checks, after it went on without the member it branched on, and the children that lost a member of their parent's set.
There 87 to 95% of the earlier cycles still hold, and 1 to 2% of those have a shorter cycle now.  No cheap proof of
which was found: the flip changes the arcs of every block the cycle touches, and the blocks are the frames, of which
the large ones hold most elements.  The variant `tune-all` (`variant_all.py` on top of both changes, read from
`STEPCOST_CARRY_ALL=1`) carries every cycle that holds, from the node's own last pricing or else its parent's, so it
overrates the price of a member that now has a shorter cycle.  Against `tune-carry2`, side by side:

| run | steps, exact | carried everywhere | CPU s, exact | carried everywhere | ratio |
|---|---|---|---|---|---|
| unit 4875-10 | 45,962 | 46,873 | 88.7 | 78.2 | 0.881 |
| unit 4914-10 (stopped) | 60,415 | 60,415 | 90.3 | 83.0 | 0.919 |
| unit 4914-1240 (stopped) | 60,415 | 60,415 | 62.4 | 55.4 | 0.888 |
| unit 4914-2825 | 32,047 | 32,235 | 59.2 | 53.6 | 0.906 |
| game 3000 | 107,724 | 116,796 | 35.6 | 37.2 | 1.045 |
| game 4050 | 582,353 | 586,518 | 392.1 | 374.4 | 0.955 |

A step costs 11 to 14% less, but the trees change, by +0.6 to +8.4% where they finished, and the whole games split:
3000 loses 4.5%, 4050 gains 4.5%.  Not kept: the gain is within what such a change does to a tree, and a search
exact in its prices is easier to trust and to measure.

## 2026-10-23: a cheaper look

**The units.**  The lab's snapshot of game 4914 is gone, so three units of game 4950 stand in for its three: sampled
from the chain's checkpoint of 4950 at 08:38 UTC (`tune units 4950 n4950.ckpt --sample 400 --seed 7`), chosen as the
4914 ones were, by a gap near 1,150 and a depth of about 10, 55 and 60: 4950-1486 (depth 14, gap 1,267), 4950-4352
(depth 53, gap 1,153) and 4950-853 (depth 60, gap 1,161), in `units-4950.jsonl`.  Game 4950 was not yet proven, so
they run from the checkpoint's incumbent, 549,269, the best of every run.  4875-10 is the same unit as above.  Every
run had the look on (`--tactic conflict_look=on`), one thread, `--node-limit 60000`, at nice 19 beside the chain,
which takes every thread of the machine at idle priority, four runs at a time.

**What the look's children lose.**  A study build (`LOOKSTUDY=1`) printed, at each look, the node's heaviest set
without `c` that the pricing certified, less the promoted members, and the members of it that the child lost.  It
lost none at 24 to 603 looks a unit and one at nearly all the others (2 at 30 of 3,540), and every child that the
bound cut had lost one.  Of the node's own heaviest set the child lost only `c` at 95% of the looks: the member lost
is the substitute that `c`'s cycle brought in, which promoting the rest of `K` takes from the child's candidates.  The
children's solves took 59 to 117 augmentations on average, and the cut children 83 to 93% of the look's solving time.
The loss is large (the solved bound is a median 1,334 to 1,486 below the need where it cut), and the heaviest set
without `c` and its substitute is smaller than the node's, which a cycle of the exchange graph does not certify, so
the repair by certified cycles that the last section proposed would not have cut them.  It was not built.

**The bound from the node's split.**  The node's weight split bounds every common independent set of its problem that
holds the promoted members and no candidate that the child lacks (weak duality: `w = w1 + w2` on every element), and
so the child's bound, with two greedy passes over the child's candidates and no solve (`Gf2Bound::child_bound`,
`Meet::split_bound_among`).  The look takes it first, and cuts the child where it is below the need; otherwise it
solves the child as before.  A bound above the child's own cuts only children that the solve cuts, so the trees are
the same, and the workspace keeps the node's tableau where the split cuts, so the node does not make it again.  The
bound of the split without one member, which a node has already for every member of its heaviest set, cut none: it
loses `c` but not its substitute, and was at least 147 above the need where the solve cut (300 on the units of
4950).

At each look a second study build (`look-study.patch`, the same variable) printed which way the look went and its
time (`look-study.py` sums it up, `look-study.txt`):

| unit | looks that reached the bound | cut by the split's bound | solved | of those, cut by the solve | the split's bound less the solved one (median) |
|---|---|---|---|---|---|
| 4875-10 | 669 | 328 (0.10 s) | 341 (2.13 s) | 55 (0.80 s) | 6 |
| 4950-1486 | 1,260 | 635 (0.17 s) | 625 (1.14 s) | 0 | 8 |
| 4950-4352 | 1,441 | 994 (0.25 s) | 447 (1.32 s) | 26 | 8 |
| 4950-853 | 170 | 128 (0.04 s) | 42 (0.26 s) | 0 | 8 |

So the split's bound cuts 86 to 100% of the children that the bound cuts, at about 0.3 ms a look, where their solve
took 10 to 14 ms.

**Side by side** (`tune-base` at cab9648, `tune-warm1` with the change; two rounds, the order swapped;
`look-runs.jsonl`, `look-cmp.py`):

| unit | steps | CPU s, base | split | ratio | second round: base | split | ratio |
|---|---|---|---|---|---|---|---|
| 4875-10 | 38,540 | 80.3 | 75.6 | 0.941 | 79.4 | 75.7 | 0.954 |
| 4950-1486 (stopped) | 60,415 | 118.8 | 110.1 | 0.927 | 118.6 | 108.9 | 0.918 |
| 4950-4352 (stopped) | 60,415 | 107.5 | 93.8 | 0.873 | 110.3 | 99.2 | 0.899 |
| 4950-853 | 8,170 | 15.5 | 12.2 | 0.787 | 16.5 | 14.4 | 0.871 |
| all four | | 322.2 | 291.7 | 0.906 | 324.9 | 298.2 | 0.918 |

Every unit took the same steps, found the same best and had the same counters of its results line but the GF(2)
bound's own: 0.81 to 0.89 of the solves and 0.71 to 0.85 of the augmentations.  The whole game 3000 with the look on
(`tune solve 3000 --ids root --known results/strict.jsonl`, the two builds side by side) took the same 117,197 steps
and the same GF(2) solves, 39.6 and 39.2 CPU seconds: its looks are few and none was cut by the split.

**Tests.**  The random paths of the GF(2) tests look at a child at every node, as the look does (some members of the
heaviest set promoted, another left out, the candidates narrowed and a few more dropped), and check that the split's
bound does no other work and is no lower than the bound of the matroids built afresh from the child's state: 223
children on the few paths, 4,126 on the long ones, 81 and 1,094 of them equal.  The looks draw from a generator of
their own, so the paths and every other count are those of the walks without them.  Debug builds of the search check
every bound the split gives against the child's solved bound.  The tripwires and soundness tests pass in release.

## Where this leaves the question

**Kept, with the same trees:** a child carries its parent's prices where its heaviest set is its parent's less what it
promoted, a check below the need skips its split, and the look bounds its child from the node's split before it
solves it.  A step of conflict branching on the units of 4875 and 4914
costs 0.66 to 0.81 of what it did, the whole game 4050 0.79.  What a step spends now, at 4875-10 (the study's timers,
scaled to the carry): the GF(2) bound's evaluations about half, the prices left (at a node's later checks and at the
children that lost a member of their parent's set) about a sixth, the ordered conflicts' peels 7%, strong branching 4%.

**What would go further,** ranked:

1. *The look's last solves.*  Done in part: [a cheaper look](#2026-10-23-a-cheaper-look) cuts most of the look's
   children from the node's split.  What is left is the solve of the children that the split does not cut, 1 to 3% of
   a unit's CPU, of which the solve cuts at most a sixth.  An exact repair of the node's heaviest set without
   `c` and its substitute would need the smaller sets' structure (a path through `c`, not a cycle), and could reach
   the bound only where the split falls short.
2. *The gate on a chain game.*  With the carry, gates 200 and 150 win on game 3000 (0.81, 0.77 of the CPU) and lose
   on 4050 (1.09, 1.27).  The lab's per-game gate is the place to try 200 on a game near 4900.
3. *The prices left, exactly.*  A flip along a cycle changes the arcs of the blocks it touches, and a shorter cycle
   through a member must use a changed arc; a bound on the cycles through the changed region (one search from it each
   way) could prove most carried prices exact.  Worth at most a sixth of a step; the region is large where the cycle
   touches the large frames, so it may prove little.

## Files

* `study.patch`: the study against 7dec259 (`src/gf2/price_study.rs`, and its hooks in `gf2.rs` and `searcher.rs`),
  under the feature `studies`, read from `STEPCOST_STUDY=1`: the comparison of each pricing with the one before it on
  the node's path, and timers of a step's parts and of the look (one line `PRICESTUDY` and one `STEPTIMERS` to stderr
  at the end of a unit).
* `variant_all.py`: the edit to `gf2.rs`, on top of the kept change, that makes the variant carrying every price that
  holds (`STEPCOST_CARRY_ALL=1`).
* `build2.sh`, `unit.sh`, `queue.sh`: a build of `tune`, one unit or whole game on one thread, and a queue of such
  lines P at a time, in the lab's clone; `round2.sh`, `round4.sh` and `tests.sh` chained them, and `jobs-*.txt` are
  the lines run: `s1` the study, `c1`/`c2` base against carry, `d1`/`d2` carry against both changes, `look`/`look2`
  the look, `w` the whole games, `g` the gates, `e1`/`e2` the variant.
* `runs.jsonl`: every run, a line each with its label (the job's), and `study.txt` the study's lines; `collect.py`
  made both from the runs' folders.  `cmp.py runs.jsonl A B` sets two labels side by side; `pstudy.py study.txt`
  sums up the study.
* The cheaper look, run on this machine's chain host from a scratch folder: `look-build.sh NAME` (a build of `tune`),
  `look-unit.sh` (one unit or whole game, as `unit.sh`), `look-jobs.py LABEL A B` (the four units, two builds side
  by side, run by `queue.sh`), `look-cmp.py A B` (two labels side by side), `look-runs.jsonl` (every run of the side
  by side comparison, a line each with its label and CPU seconds), `units-4950.jsonl` (the three units of 4950), and
  `look-study.patch` with `look-study.py` and its output `look-study.txt` (the study of which way each look went).

## Commits

The change, its docs (`gf2-bound.md`, `search.md`) and this entry are one commit on master, its message opening "A
child carries its parent's prices".  The cheaper look is a commit of its own, its message opening "The look of
conflict_look bounds its child from the node's weight split", and the commit after it, opening "conflict_look is on
by default", makes the look the default.
