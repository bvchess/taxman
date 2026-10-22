# When a node should use conflict branching (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-22.

**Question.**  The study of game 4895 ([the live lab](../the-live-lab-2026-10/README.md)) found that conflict branching
off closes units of depth 72 to 105 in 0.005 to 0.05 of the steps it takes on, while on wins on shallow units.  Why, and
which rule for when a node uses conflict branching is better on whole games, keeping the search exact?  A rule that
chooses per node between the search's own conflict and the one conflict branching chooses only chooses among true
conflicts, so it is exact for the reason [search.md](../../../docs/design/search.md#conflict-branching) gives.

## 2026-10-22: the setup

The variants are on a local branch, `research-cb`, of a clone on the lab's machine (not pushed): the gate of
`TAXMAN_CB_GAP` (the lab's branch `study-4884-gate`), then switches read once from the environment, all under the
feature `studies`.  `TAXMAN_CB_TRACE=1` writes a line to stderr at every step, check and choice: the node's depth, its
gap, the search's own conflict `k0` with the price of each member, the conflict chosen and its weight, and how the step
ended.  `TAXMAN_CB_POLICY` takes a list of rules, each described where it is measured below.  With neither set, a build
of the branch takes the same steps as master (game 3000 107,724, 4050 582,353; unit 4895-3288 19,756, 4895-5268 562).

Steps are exact and repeat on one thread.  CPU seconds on this machine are not: it has 44 cores of two hardware threads
each, the chain keeps 72 threads busy, and a unit's CPU seconds changed by up to two times between runs of the same tree
(4895-1943: 80 s and 43 s for 71,421 and 71,909 steps).  So the comparisons below are in steps, and CPU only from runs
side by side.

**Baselines, whole games** (`tune solve N --ids root --known`, one thread, proved against the optimum, the default gate
of 300):

| game | on: steps | CPU s | off: steps | CPU s |
| --- | ---: | ---: | ---: | ---: |
| 2000 | 25,845 | 7.5 | | |
| 3000 | 107,724 | 39.3 | 167,554 | 55.8 |
| 3500 | 1,166,725 | 520 | | |
| 4050 | 582,353 | 449 | 3,086,223 | 1,722 |

Game 3500 proves in 1,166,725 steps at gate 300; the study of 4895 stopped it at 540 s, just short.

## 2026-10-22: why off closes the deep units

The traces of 4895-5268 (depth 98, gap 812; on 562 steps, off 11) and 4895-3288 (depth 92, gap 908; on 19,756, off 92)
show one mechanism.

* **Off.**  The search's own conflict is small and its members cheap: at 5268, `k0` = {2416, 2424}, prices 8 and 16.
  Strong branching takes 2424; the child that promotes it leaves 2416 out, and its check finds the conflict {2384}, a
  member that the child forces and cannot promote: the child is cut on sight (conflict within the forced members).  The
  node goes on without 2424.  The next conflicts are {2328, 2416}, {2136, 2416} and {1992, 2416}, each with the same
  end: every child that leaves 2416 out dies, so each pair costs one step and the node pays the other member's price
  (430, 252, 96).  The gap falls 812, 796, 366, 114, and the unit ends in 11 steps.  The unit 3288 runs the same way,
  through the same 2416 and 2384, in 92.
* **On.**  Conflict branching weighs {2416, 2424} at 1.82 (two members that cost the bound almost nothing to leave out)
  and chooses a conflict of six members whose prices are 140 to 215, weight 1.43.  Its first five members are promoted
  one by one, each child keeping the whole gap of 812, and each leaves a sibling, the node without that member, of gap
  597 to 672.  Each sibling has the 2416 structure still to find, and chooses large conflicts again; the unit takes 562
  steps.  At 3288 the chosen conflict has nine members.
* **What the weight misses.**  A price is what leaving one member out costs the bound.  The child that promotes the
  other member of a pair is cut by a conflict that only the promotion makes (2424 and 2384 cannot both be promoted),
  together with a member that leaving 2416 out forces (2384).  The weight counts that child at its bound's gap.  In
  effect 2416 is forced at these nodes, though its price is 8.

Off is not right everywhere.  On the shallow unit 4895-1065 (depth 8, gap 848; on 6,158, off 30,732), the child of a
two-member conflict of the search's own is cut on sight 1,460 times in 3,234 (45%); at 3288 under off, 16 times in 17.

**Where whole games spend.**  In game 3000 the search makes a choice of conflict branching at only 1,346 of its
107,724 steps; most nodes have gaps under 300.  Its steps lie at depth 90 to 130 under both settings.  So a game of
3000 to 4050 exercises the rule far less than 4895 does, whose deep units still have gaps of 900.

**Where 4895's work lies.**  By the notebook's line (log10 steps = 0.059 + 0.396 · gap/100) summed over the 5,459 units
of the snapshot, the units of depth 0 to 19 hold 48% of the work left, 20 to 39 42%, 40 to 69 8.3%, 70 to 99 1.7% and
100 or more 0.1%.  The deep units where off wins hold a few percent of the game, even if they run five times above the
line; the work is in the shallow units, whose subtrees reach every depth.

## 2026-10-22: rules on the units of 4895

The panel is the 48 units of the study of 4895 (`work/s4895/ids.txt`), each alone on one thread with fresh tables from
the snapshot's best, limited to 150 CPU s; 45 finish under the default.  Ratios are of the steps on the units that every
rule compared finished, against the default (on, gate 300); "shallow" is depth under 40 (21 units), "deep" the rest.

| rule | units | all | shallow | deep |
| --- | ---: | ---: | ---: | ---: |
| default (on): steps | 45 | 694,422 | 284,152 | 410,270 |
| `forced0`: a member that costs the whole gap or more weighs 0 | 42 | 1.006 | 1.007 | 1.004 |
| `maxdepth=60`: conflict branching only above depth 60 | 42 | 1.539 | 2.261 | 0.071 |
| `look=1` (below) | 45 | 0.975 | 0.985 | 0.967 |

* **A depth gate fails.**  Off below depth 60 makes the deep units 14 times cheaper and the shallow ones 2.3 times
  dearer: the shallow units' own deep nodes want conflict branching.  Depth is not the signal.
* **`forced0` changes almost nothing**, as the study's forced-members variant did: a conflict holding a forced member
  is rarely the one chosen.
* **`look=1`.**  Before branching on another conflict, at a node with no forced member whose own conflict has two
  members, look at the child of the search's own conflict that promotes the member strong branching would take; if
  that child is cut on sight (its promotion fails, its largest candidates or its GF(2) bound fall below its need, or a
  conflict of its heaviest set lies within its forced members), branch on the search's own conflict, which then weighs
  only the node without that member.  It takes 5268 from 562 steps to 10, but leaves 3288 at 19,648: there the
  search's own conflict has four members, and the pair that dies comes at the end of its chain.  Of 36,699 choices on
  the panel it looked 5,610 times and found 664 children cut; the panel took 2.5% fewer steps.
* **`after=X`**: conflict branching only at a node whose subtree has taken X steps so far, a per-node form of the
  study's probe (a node's first choice is then always the search's own).  On four units:

| unit | depth | on | `after=1` | `after=64` | `after=1024` |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4895-1065 | 8 | 6,158 | 13,200 | 20,910 | 23,833 |
| 4895-1943 | 80 | 71,421 | 116,724 | > 196,371 | > 216,984 |
| 4895-3288 | 92 | 19,756 | 342 | 92 | 92 |
| 4895-5268 | 98 | 562 | 16 | 11 | 11 |

  A node's first choice is the one that matters, at the shallow units as at the deep: off for it alone doubles 1065.
  The study's probe gains on units because it throws a failed probe away and starts again with on; a rule that keeps
  what the probe built cannot.

**The slope, and a moving average.**  Two more rules, on four units first (steps):

| unit | depth | on | `slope=150` | `slope=600` | `ema=0.5` | `ema=0.65` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 4895-1065 | 8 | 6,158 | 6,868 | 12,956 | 6,335 | 6,158 |
| 4895-1943 | 80 | 71,421 | 73,200 | 77,398 | 80,366 | 70,014 |
| 4895-3288 | 92 | 19,756 | 26,557 | 92 | 1,339 | 2,093 |
| 4895-5268 | 98 | 562 | 1,682 | 11 | 253 | 408 |

* **`slope=S`**, the weight's slope in place of 291.  A larger slope flattens the weights towards a count of members,
  so it prefers small conflicts, as off does: 600 closes the deep units as off does and doubles 1065.  The slope trades
  the deep units against the shallow ones; no single value serves both.
* **`ema=T`**: conflict branching only while the searcher's moving average (weight 1/16) of how often the child of a
  two-member conflict with no member forced is cut on sight stays below T.  The traces suggested it: that rate is 58% at
  3288 and 66% at 5268 under on, 24% at 1065 and 36% at 1943.  At T = 0.65 it takes the deep units of the 4895 panel to
  0.26 of the default's steps but costs 2% on the shallow ones (36 units; the rest were not run), and on the 20 units
  of the study of 4899 (`work/s4899`) 0.64 on the deep and +1.8% on the shallow.  The shallow units carry the work, so
  it would gain nothing on a game.

**Game 3200, gate 150 against 300**, side by side: 636,052 steps and 699 CPU s at 150, 1,054,752 and 579 at 300.  A
step at gate 150 cost 2.0 times one at 300 here (1.10 ms against 0.55); on game 3500 it was 2.3 times (445 s for
410,298 steps against 558 s for 1,166,725, in two studies at different times).

The study of 4899 (the lab's report `work/reports/4899-20261022T151018Z.md`) refuted a depth gate on
whole games (D = 20 to 60: +17% to +86% steps on 3000 and 3500) and exact prices for the search's own conflict alone
(+38% and +61% steps), and found on losing at 4899-3586 (28,186 steps against 326 off) and 4899-3379 (1,712 against
78).

## 2026-10-22: `look` at conflicts of any size

**The rule.**  `look=1,lookn` generalizes `look` from pairs to the search's own conflict of any size.  The children of a
conflict each leave one member out, the members before it in; in strong branching's order (the dearest first) the last
child promotes every member but the cheapest, c, and leaves c out.  At a check where conflict branching would branch on
another conflict and no member is forced, if the search's own conflict would weigh less than that conflict with c's
term taken off, the rule makes that last child aside (promotes the members together, as `make_child` does, narrows its
candidates and drops those the nogood table rules out, then the cheap checks, the GF(2) bound and a conflict of its
heaviest set: the table's, then the smallest a peel finds) and pops it again.  If it is cut on sight, the node branches
on its own conflict, and the conflict that cut the child is learned as a nogood, so the search's own check of that
child cuts it too.  For a pair this is `look=1`.  At 3288 the root's own conflict {2196, 2416, 2424, 2440} weighs 2.70
against 1.85 for the nine-member one chosen; its last child (2440, 2196 and 2424 in, 2416 out) dies, so it weighs 1.76,
and the rule takes it.

*Why it is exact.*  It only chooses between two true conflicts of the node.  The child it makes aside is pushed on the
GF(2) stack and popped before the node reads its own level again, which then resolves its level as it does after any
child; the conflict it learns is a conflict of that child, made into a nogood that the root checks, as every nogood
is; and no node's list of known conflicts gains anything from it.

**On the units** (steps; the 4895 panel at 150 CPU s, and the 20 units of 4899):

| units | finished | on | `look=1` | `look=1,lookn` |
| --- | ---: | ---: | ---: | ---: |
| 4895, all | 44 | 605,675 | 0.972 | 0.801 |
| 4895, depth under 40 | 21 | 284,152 | 0.985 | 0.861 |
| 4895, depth 40 or more | 23 | 321,523 | 0.960 | 0.748 |
| 4899, depth under 40 | 5 | 106,362 | 0.999 | 0.992 |
| 4899, depth 40 or more | 15 | 74,400 | 0.968 | 0.556 |

It takes 4895-3288 from 19,756 steps to 273, 4899-3586 from 28,186 to 474 and 4899-2578 from 1,636 to 221, and saves 5%
to 25% on most shallow units of 4895 (1186: 36,779 to 27,536; 1366: 33,255 to 27,855).  It looked at 21,717 of 35,371
choices and found 4,788 last children cut.

**On whole games** (one thread, proved against the optimum):

| game | on: steps | `look=1,lookn`: steps | CPU s | look: choices, looked, cut |
| --- | ---: | ---: | ---: | --- |
| 3000 | 107,724 | 114,052 | 42.5 | 239, 188, 69 |
| 4050 | 582,353 | 581,104 | 508 | 5,547, 4,620, 393 |

Whole games of this size barely use it: game 3000 makes 239 choices that the rule could change, and 4050 finds a dead
last child at 393 of its 5,547.  Their nodes rarely have gaps of 300 deep down, where the hubs that kill last children
show.  The games the chain runs now do: 4895 and 4899 have units of gap 750 to 1,050 at depth 55 to 105.

**On the units of a fresh search of 4875.**  The fresh units of [where 4875
spends](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md) (`units-fresh.jsonl`, a
fresh search's checkpoint after 3 million steps, proved from 538080) are the bulk of a game the size the chain runs:
the deque of a search, not a worker's deep stack.  Twenty of them that the default takes 12,932 to 74,491 steps,
depths 0 to 72 and gaps 918 to 1,415, ran under both rules, interleaved four at a time so that their CPU seconds
compare.  The default's steps match that entry's (21,740 and 31,920 for 4875-0 and 4875-1).

| units | on: steps | CPU s | `look=1,lookn`: steps | CPU |
| --- | ---: | ---: | ---: | ---: |
| all 20 | 963,153 | 2,108 | 0.813 | 0.925 |
| depth under 40 (10) | 413,473 | 1,064 | 0.877 | 0.967 |
| depth 40 or more (10) | 549,680 | 1,044 | 0.765 | 0.882 |

Every unit took fewer steps under the rule, 0.67 to 0.99 of the default's.  A step costs 14% more for the looks, so
the CPU gain is about two fifths of the steps'.

**On the units of 4900** that its study (`work/reports/4900-20261022T163917Z.md`) found split: five where off wins 20 to
100 times and a gate of 700 cuts the steps 3 to 25 times, and five where both of those cost (steps, 200 CPU s each, the
two rules side by side):

| unit | depth | gap | on | off (study) | `look=1,lookn` |
| --- | ---: | ---: | ---: | ---: | ---: |
| 4900-3570 | 83 | 808 | 3,284 | 64 | 262 |
| 4900-5835 | 65 | 1,031 | 14,456 | 146 | 824 |
| 4900-3294 | 58 | 950 | 16,079 | 164 | 245 |
| 4900-175 | 77 | 752 | 6,183 | 44 | 2,488 |
| 4900-592 | 99 | 886 | 4,468 | | 167 |
| 4900-6273 | 57 | 935 | 16,810 | 66,369 | 16,468 |
| 4900-834 | 59 | 930 | 12,737 | 18,173 | 12,307 |
| 4900-975 | 56 | 759 | 7,985 | 15,032 | 7,878 |
| 4900-941 | 13 | 848 | 10,603 | | 10,260 |
| 4900-1963 | 44 | 915 | 33,252 | > 165,489 | 33,285 |

The rule separates the two groups that depth and the gate did not: it takes the first five to 0.04 to 0.40 of the
default's steps and leaves the other five within 4% of it (four a little below, 1963 0.1% above).  In all, 0.67 of
the steps and 0.65 of the CPU.

**Two more whole games** under `look=1,lookn`: game 3200 in 1,093,391 steps (the default 1,054,752, so +3.7%) and 590
CPU s; game 3500 in 1,081,914 (the default 1,166,725, −7.3%) and 492 s.  Over the four games 3000, 3200, 3500 and 4050
the rule is even in steps, within ±7% on each.  Whole games of this size do not decide it either way; they test only
that it costs nothing where it has little to do.

**The eleven largest fresh units of 4875** that the entry on 4875 ran to the end (170,760 to 1,074,316 steps under
the default, gaps 1,158 to 1,479), both rules side by side, four at a time:

| unit | depth | gap | on: steps | CPU s | `look=1,lookn`: steps | CPU s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 4875-13 | 13 | 1,345 | 386,048 | 849 | 314,941 | 769 |
| 4875-15 | 15 | 1,478 | 1,074,316 | 2,278 | 879,786 | 2,012 |
| 4875-26 | 26 | 1,319 | 411,478 | 677 | 425,624 | 744 |
| 4875-28 | 28 | 1,421 | 239,349 | 529 | 211,171 | 520 |
| 4875-29 | 29 | 1,431 | 513,954 | 1,039 | 394,114 | 879 |
| 4875-30 | 30 | 1,478 | 889,824 | 1,781 | 835,349 | 1,790 |
| 4875-31 | 31 | 1,479 | 197,731 | 463 | 179,542 | 458 |
| 4875-40 | 40 | 1,158 | 394,604 | 645 | 368,288 | 654 |
| 4875-41 | 41 | 1,284 | 170,760 | 290 | 144,950 | 273 |
| 4875-49 | 49 | 1,279 | 265,731 | 382 | 262,069 | 404 |
| 4875-50 | 50 | 1,405 | 491,546 | 959 | 383,314 | 799 |

With the twenty above, these 31 units hold 6.0 million of the 6.07 million steps of the 59 fresh units that entry
finished: under the rule they take 0.864 of the steps and 0.938 of the CPU (shallow ones 0.873 and 0.945, deep 0.843
and 0.919).  The rule loses on one, 4875-26 (+3.4% steps).  These units are the part of a fresh search of 4875 that a
thread can finish in an hour; the 37 left, of gaps up to 1,900, hold most of its work by that entry's projection.

**What a look costs, and which looks pay.**  `TAXMAN_CB_TRACE_LOOKS` writes a line a look (v7 of the branch, the same
trees).  On 4875-49, where the rule saved 1.4% of the steps and cost 6% of the CPU, it looked 7,305 times in 262,069
steps and found 1,107 last children cut (15%):

| the cheapest member's price | looks | cut |
| --- | ---: | ---: |
| 0 | 27 | 0 |
| 1 to 10 | 1,192 | 532 (45%) |
| 11 to 30 | 2,311 | 92 (4%) |
| 31 to 100 | 2,899 | 183 (6%) |
| 101 to 300 | 876 | 300 (34%) |

By that unit's CPU, which carries the machine's noise, a look costs about three steps.  Pairs die more often (519 of
1,471) than larger conflicts (6% to 13%).  The looks that pay are those of the hubs, a member that costs the bound
almost nothing to leave out (8 at 3288), and those of conflicts whose cheapest member is already dear.

**A filter: `lookpc=10`**, a look only where the cheapest member's price is 10 or less (v7).  On the twenty mid-size
fresh units of 4875 it looked 12,032 times against 33,701 and found 8,156 last children cut against 9,978, and its
trees are as small or smaller; on the ten units of 4900 the same:

| units | on: steps | `look=1,lookn` | `look=1,lookn,lookpc=10` |
| --- | ---: | ---: | ---: |
| twenty fresh units of 4875 | 963,153 | 0.813 | 0.809 |
| of them, depth under 40 | 413,473 | 0.877 | 0.859 |
| ten units of 4900 | 125,857 | 0.669 | 0.669 |

Its CPU was 0.876 of the default's on the fresh units and 0.624 on 4900's, against 0.925 and 0.650 for the rule
without it, but those runs were not side by side with the default's (an hour apart), so the CPU is a guess within this
machine's noise.

## 2026-10-22: gate 150 against 300 on whole games

Side by side on one thread, proved against the optimum (`TAXMAN_CB_GAP`):

| game | gate 150 | gate 300 |
| --- | --- | --- |
| 3200 | 636,052 steps, 699 s | 1,054,752 steps, 579 s |
| 3800 | not done at 3,600 s (2,782,370 steps) | not done at 3,600 s (5,635,902 steps) |
| 4000 | not done at 3,600 s (1,865,338 steps) | not run to the end here |

A step at gate 150 costs about twice one at 300 (1.10 ms against 0.55 ms on 3200), so gate 150 pays only where it
halves the steps.  The study of 4901 (`work/reports/4901-20261022T175934Z.md`) ran the same games to 3,000 s and found
3200 1.21 times dearer at 150, 4000 done at 300 in 2,828 s and not done at 150 by 2,999 s, and 3800 done at neither.
With 3500 the only game where 150 won (0.80 of the CPU), the gate stays 300, and the runs of 3800 here were stopped.

## 2026-10-22: the filtered rule on whole games and on 4901

**Whole games** under `look=1,lookn,lookpc=10` (one thread, proved against the optimum): 3000 in 122,852 steps (+14.0%
on the default's 107,724), 3200 in 1,076,212 (+2.0%), 3500 in 1,159,847 (−0.6%), 4050 in 555,804 (−4.6%).  The four
together take 2,914,715 steps against 2,911,554: even, as without the filter.

**The deep units of 4901** that its study found off to close (`work/reports/4901-20261022T175934Z.md`; on's and off's
steps are the study's), and two where off fails:

| unit | on | off (study's probe) | `look=1,lookn,lookpc=10` |
| --- | ---: | ---: | ---: |
| 4901-5114 | 1,440 | 42 | 1,233 |
| 4901-5597 | 3,500 | 53 | 260 |
| 4901-5779 | 1,756 | 47 | 1,697 |
| 4901-5106 | 4,310 | 54 | 307 |
| 4901-2174 | 1,108 | 72 | 1,108 |
| 4901-4822 | 813 | 45 | 125 |
| 4901-4461 | 683 | 34 | 134 |
| 4901-1917 | 7,835 | 113 | 6,751 |
| 4901-524 | 2,099 | 130 | 1,870 |
| 4901-1935 | 6,398 | 171 | 6,332 |
| 4901-5968 | 15,704 | 194 | 14,447 |
| 4901-5752 | 45,956 | 941 | 34,685 |
| 4901-1757 | > 50,402 | 3,382 | > 79,994 at 200 s |
| 4901-3882 | > 47,988 | 3,246 | > 79,697 at 200 s |
| 4901-659 (off fails) | 39,399 | 57,725 | 39,399 |
| 4901-5425 (off fails) | 2,824 | 6,257 | 2,824 |

On the twelve that both finish the rule takes 0.75 of on's steps (68,949 against 91,602); off takes 2% of them.  It
catches four units whole and touches the rest little, and costs nothing where off fails.  The trace of 4901-2174 shows
why it misses: the same
hub as in 4895 (2416 and 2384), with the search's own conflict {2196, 2416, 2424, 2440} weighing 2.81 against 1.25 for
the chosen one.  Its last child dies, but even without 2416's term it weighs 1.87, so the rule does not look.  Off's
other children of that conflict, of gaps 451 to 697, also close in a few steps, which the weight cannot see.  Neither
`look=1,lookn` without the filter nor `look=3,lookall,lookn` (the last children of every conflict weighed) did better
on these units.

## 2026-10-22: trusting a dead last child (`lookslack`)

**The rule.**  `lookslack=A` (v8) looks at the search's own conflict, and branches on it if its last child is cut on
sight, while the conflict's weight less its cheapest member's is below A times the best weight, not only below it.  A
dead last child is evidence of a hub, and at a hub the weight overrates every child of the search's own conflict, not
only the last.  With A = 2, `look=1,lookn,lookpc=10,lookslack=2`:

| units | on: steps | `lookpc=10` | `lookslack=2` |
| --- | ---: | ---: | ---: |
| the twelve deep units of 4901 that off closes | 91,602 | 0.75 | 0.097 |
| 4901-659 and 4901-5425, where off fails | 42,223 | 1.00 | 1.00 |
| the ten units of 4900 | 125,857 | 0.669 | 0.609 |
| twenty fresh units of 4875 | 963,153 | 0.809 | 0.820 |
| of them, depth under 40 | 413,473 | 0.859 | 0.847 |

On 4901 it now closes 2174 in 177 steps (on 1,108, off 72), 1917 in 440 (7,835; 113), 5968 in 723 (15,704; 194) and
5752 in 4,241 (45,956; 941).  On 4900 it takes 4900-175 to 172 steps (on 6,183, off 44) and the shallow 4900-941 to
0.84 of on's.

**Whole games** (one thread, proved against the optimum, steps):

| game | on | `lookpc=10` | `lookslack=2` |
| --- | ---: | ---: | ---: |
| 3000 | 107,724 | 122,852 | 117,197 |
| 3200 | 1,054,752 | 1,076,212 | 1,036,383 |
| 3500 | 1,166,725 | 1,159,847 | 1,131,834 |
| 4050 | 582,353 | 555,804 | 553,889 |
| all four | 2,911,554 | 1.001 | 0.975 |

The CPU of the `lookslack=2` games, run at once, was 42.6, 570, 514 and 491 s, against 39.3, 579, 520 and 449 s for
the default's runs of the morning: within this machine's noise of the steps' ratios.

With A = 4 the units of 4901 are the same but for 524 (630 steps against 1,321), those of 4900 the same, the twenty
fresh units of 4875 0.810, and the whole games 3000, 3200, 3500 and 4050 take 117,197, 1,044,787, 1,135,542 and
595,281 steps, 0.994 of the default's together: no better than A = 2, and 4050 2% above the default.  Under A = 2 the
twenty fresh units looked 21,542 times in 790,000 steps (2.7%) and found 11,481 last children cut (53%); the whole
games looked 124 to 3,070 times.

**The CPU, side by side.**  The default and `lookslack=2` on the twenty fresh units of 4875 again, each pair at once
(two threads): 963,153 steps and 2,139 CPU s against 0.820 of the steps and 0.922 of the CPU (depth under 40: 0.847
and 0.956; 40 or more: 0.800 and 0.889).  A step costs 12% more under the rule.

**The other unit sets** under `lookslack=2` (steps against the default's):

| units | finished | on | `look=1,lookn` | `lookslack=2` |
| --- | ---: | ---: | ---: | ---: |
| 4895 panel, all | 44 | 605,675 | 0.801 | 0.664 |
| 4895, depth under 40 | 21 | 284,152 | 0.861 | 0.733 |
| 4895, depth 40 or more | 23 | 321,523 | 0.748 | 0.604 |
| 4899, depth under 40 | 5 | 106,362 | 0.992 | 0.941 |
| 4899, depth 40 or more | 15 | 74,400 | 0.556 | 0.518 |

On the shallow units of 4895, the ones that carry a game's work, the rule takes 0.62 to 1.00 of the default's steps
(4895-1366: 33,255 to 20,765; 2795: 52,678 to 35,681).

**The eleven largest fresh units of 4875** under `lookslack=2` (four to six at a time, a few hours after the default's
runs, so the CPU is not side by side): 4875-13 0.823 of the default's steps, 15 0.827, 26 1.072, 28 0.868, 29 0.839,
30 0.928, 31 0.908, 40 0.958, 41 0.878, 49 0.960, 50 0.750.  With the twenty mid-size ones, the 31 units take 0.872 of
the default's 5,998,494 steps (`look=1,lookn` 0.864) and 0.927 of its CPU; depth under 40 0.883, 40 or more 0.850.  Of
the 119 units of 4875, 4895, 4899, 4900 and 4901 compared under the rule, the worst are 4875-26 at +7.2% and 4895-1941
at +3.1%.

## Where this leaves the question

**Why off wins where it does.**  At some deep nodes of the games the chain runs now there is a hub: a member that
costs the bound almost nothing to leave out, because a substitute can take its place, while the substitute cannot be
promoted together with most of the heaviest set.  Every completion that beats the incumbent then holds the hub, though
its price is a few points.  The search's own conflicts are small and hold the hub, so off branches on pairs whose
child without the hub is cut on sight, and drains the gap a member at a time.  Conflict branching weighs each child at
its bound's gap, rates the hub's conflicts the worst, and branches on conflicts of dear members whose children all keep
the hub to find again.  Inexact prices (the study of 4900), forced members (the study of 4895), the slope and the depth
are not the cause.

**A rule that sees it.**  Look at the last child of the search's own conflict, the one that promotes every member but
the cheapest and leaves the cheapest out, where that member costs 10 or less; if the child is cut on sight, branch on
the search's own conflict whenever its weight less the cheapest member's is below twice the best weight.  It is exact,
since it only chooses between true conflicts.  On the units of the chain's games it takes 0.61 to 0.87 of the
default's steps (4900 0.61, 4895 0.66, 4899 0.77, the 31 fresh units of 4875 0.87), and 0.10 on the deep units where
off wins; on whole games 3000 to 4050 0.975.  Side by side on twenty fresh units of 4875 it took 0.92 of the CPU.
The whole games small enough to run here have few deep nodes of large gap, so they cannot show the gain; a game near
4900 can.

**Not kept:** a depth gate, a gate on the subtree so far, other slopes, forced members weighing nothing, a moving
average of children cut on sight, looks at every conflict weighed, and gate 150 in place of 300.

**What would decide it,** ranked:

1. The rule as a tactic of the solver, on a game of the chain near 4900 against the default.  Expected: 0.75 to 0.9
   of the steps and 0.85 to 0.95 of the CPU, a guess from the units, which run with fresh tables.  Games near 4900
   differ from one another by up to four times (games 4880 to 4893 took 1.7 to 6.1 × 10⁸ steps, by the study of
   4895), so the test is a game run both ways from the same start, or the lab switching the rule on alternate games.
2. A cheaper look.  A step costs 12% more under the rule.  Pairs and cheapest prices of 10 or less carry most of the
   cuts (45% of such looks cut at 4875-49, 4% to 6% of the others).
3. The levers a lab agent could set per game: the rule on or off, its slack (2; at 4 the units were the same and game
   4050 took 7% more steps), its price filter (10), and the gate (300).  The data show the gate's best value varying
   from game to game (150 wins on 3500 and loses on 3200 and 4000); the slack and the filter were not seen to.

## Files

* `study-cb.patch`: the prototype branch `research-cb` against 8fd0345 (`git diff`, the frame solver's sources), in a
  clone on the lab's machine, not pushed; every switch is read from the environment under the feature `studies`.
* `unit.sh`, `game.sh`, `panel.sh`, `queue.sh`: one unit of a snapshot, one whole game, the 48 units of the study of
  4895, and a queue of such lines run P at a time; each writes `runs/LABEL/ID.jsonl` beside the binaries `bin/tune-vN`.
* `cmp.py`: the steps and CPU of labels side by side on the units every label finished (`SNAP` names the snapshot).
* `tsum.py`: a summary of a trace (`TAXMAN_CB_TRACE=1`); `looks.py`: of the looks (`TAXMAN_CB_TRACE_LOOKS`).
* `collect.py`: makes `runs.jsonl`, every run of this entry, a line each: the label, the unit or game, steps, CPU
  seconds, whether it completed, its best and the optimum.

The labels: `p-` the 4895 panel, `u99-`, `u00-`, `u01-` units of 4899, 4900 and 4901, `f-` the fresh units of 4875
(`f-on2` and `f-slack2b` the runs side by side), `q-` to `q6-` four-unit tests, `tr-` to `tr01-` and `tl-` traces,
`w-` and `v0-` whole games, `g150`, `g300` the gates; the rule follows (`lookn` is `look=1,lookn`, `pc10` adds
`lookpc=10`, `slack2` and `slack4` add `lookslack`).  The binaries: v0 8eabf67 (the gate alone), v1 6b8d7f5 (the
trace, `maxdepth`), v2 d1558ae (`look`, `forced0`), v3 2928608, v4 9cd935f (`after`), v5 bd69d65 (`slope`, `ema`), v6
a96ddf0 (`lookn`), v7 a8800dc (the filters and the trace of looks), v8 c2fe0f7 (`lookslack`).

## Commits

The prototype's commits are on the local branch above and in `study-cb.patch`.  This entry and its register rows are
the repository's only change.

## Later (2026-10-22): built as the tactic `conflict_look`

The rule `look=1,lookn,lookpc=10,lookslack=2` is built as the tactic `conflict_look`, off by default, which a lab
switches per game through the `branching` command ([the design](../../../docs/design/search.md#conflict-branching)).
Game 3000 proved from its root on one thread takes 117,197 steps with it on, as the prototype did.  Its whole-game test
on the chain is pending.
