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
evaluation; a solve warm-started from the node's heaviest set without the cheapest member would cut the same children
for less (not built).

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

## Where this leaves the question

**Kept, with the same trees:** a child carries its parent's prices where its heaviest set is its parent's less what it
promoted, and a check below the need skips its split.  A step of conflict branching on the units of 4875 and 4914
costs 0.66 to 0.81 of what it did, the whole game 4050 0.79.  What a step spends now, at 4875-10 (the study's timers,
scaled to the carry): the GF(2) bound's evaluations about half, the prices left (at a node's later checks and at the
children that lost a member of their parent's set) about a sixth, the ordered conflicts' peels 7%, strong branching 4%.

**What would go further,** ranked:

1. *A cheaper look* (with `conflict_look` on): solve the look's child from the node's heaviest set without the cheapest
   member, less the promoted, repairing only the members the narrowing takes from it.  The same bound, so the same
   cuts and trees; it could save most of the look's 2 to 20% of the CPU.  To build: a repair of several members in
   `meet` (a certified child repairs one).  The test: the four units above with the look on, steps the same, CPU side
   by side.
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

## Commits

The change, its docs (`gf2-bound.md`, `search.md`) and this entry are one commit on master, its message opening "A
child carries its parent's prices".
