# Conflicts closed under cheap exchanges (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  When the search goes on without a member of a conflict, its bound swaps in the member's twin at a small
price.  Is the conflict, with the same swap made, a conflict too?  And if it is, does learning those images along with
each conflict make the hard trees smaller?

**Answer.**  Yes to both.  Under the twin swaps and paired exchanges the bound makes, 44-59% of the conflicts' images
are conflicts too, and 77-100% of the cheap ones' images, those priced under 50.  Learning, with each conflict the
search computes, the conflicts within its images under the cheap exchanges at its node takes 8-32% off the steps of
three hard units.  Learning their closure, the images of the images and so on, takes 37-49% off, and 34-46% off the
CPU time with the checks paid.  The best and the conclusion are unchanged: the search learns only what a peel shows to
be a conflict, so the learning is sound by construction.

The idea is the first of
[near completions](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md#ideas), tested on the units
of [the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md).  The learning lives in the studies build only; the
next step is to measure it on the whole tuning suite and on whole games.

## Words

* A *conflict* of a node is a set of its candidates that cannot be promoted together; *exclusion*, *price*, *home* and
  *unit* are the [anatomy's](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words).  An exclusion is *cheap* when
  its price is under 50 points.
* Two numbers are *twins* when they share a maximal factor: x = m·p and y = m·q with p and q prime, as 1292 = 68·19 and
  1156 = 68·17.  A *twin swap* takes x out of a set and puts y in.  A *paired exchange* is two twin swaps of one pair of
  primes in opposite directions: 1608 = 24·67 and 1220 = 20·61 out, 1464 = 24·61 and 1340 = 20·67 in, at a price of
  (67 − 61)(24 − 20) = 24
  ([near completions](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md#words)).
* When a node goes on without the member x of its conflict K, the bound's heaviest set at the node's next step has
  lost some numbers, x among them, and gained others: the *substitution*.  When the substitution is a twin swap or a
  paired exchange, the *image* of K is K with each member that the substitution takes out replaced by the twin it puts
  in.  The image lies in the new heaviest set.
* The *closure* of a conflict under the cheap exchanges at a node is the conflict's images, the images of those that
  are conflicts, and so on.  A *class* is a conflict the search computed, with the conflicts it learned within its
  images.

## What was run

**The instrument** is `wall images` ([examples/wall/images.rs](../../frame/examples/wall/images.rs), with tests), on
three hooks of the studies build ([src/search/hooks.rs](../../frame/src/search/hooks.rs)):

* `StepHook::excluding` is now told the conflict the node branches on as well;
* `StepHook::propose` is asked, whenever a bound check computes and learns a conflict, for more sets to learn;
* `StepHook::learned` is told what the searcher learned from them.

The searcher learns, of each set proposed, a minimal conflict that a peel finds within it
(`FrameGraph::conflict_using`), as it learned the conflict itself: known for the node and its subtree, and as a
nogood, unless its table already knows a conflict within it.  A default build compiles none of this, and its search
loop is unchanged.  `wall images` has three modes:

* the test (the default): at each exclusion, whether the image of the node's conflict under the substitution is a
  conflict of the node, with a row an exclusion;
* `--learn`: the hook proposes the conflict's images under the cheap twin swaps and paired exchanges at the node.  A
  member a = m·p of K gives way to a candidate a' = m·q outside K.  The swap is proposed when it costs 0 < a − a' < 50,
  or when a paired exchange makes it that cheap: a member b = m₂·q of the heaviest set whose twin m₂·p is a candidate
  outside it.  `--closure` proposes the images of those images that are conflicts too, and so on, up to 64 sets a
  conflict.  With `--trace` it writes the classes;
* `--bare`: the search with no hook at all, for its time.

**The units** are the anatomy's three of the
[tuning suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md): 3300-14 and 3441-1262, the big ones,
and 3441-5459, of about the same gap and a twentieth of the size.  Each was solved on one thread from the known
optimum, with fresh tables, on this laptop on 2026-10-19, from the build of `--profile test`.  The runs took about 56
CPU minutes, and the builds and checks about 50 more; nothing ran on EC2.

## 1. The images of the conflicts under the bound's substitutions

Of the exclusions, 30-37% have a twin swap or a paired exchange for their substitution, and 44-59% of their images
are conflicts.  The cheap ones' images are conflicts almost always (`analysis.txt`, "By substitution"):

| | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| exclusions | 345,237 | 342,969 | 18,075 |
| twin swaps; the image a conflict | 101,670; 52.8% | 61,911; 34.1% | 5,052; 39.0% |
| paired exchanges; the image a conflict | 27,285; 80.8% | 41,636; 95.3% | 412; 100% |
| cheap twin swaps and paired exchanges; the image a conflict | 35,592; 77.4% | 47,470; 95.1% | 429; 100% |
| steps under those cheap ones whose image is a conflict | 50.8% | 26.5% | 9.3% |
| steps under any whose image is a conflict | 57.0% | 35.0% | 16.7% |

The steps *under* an exclusion are the rest of its node's loop, from the node's next step to its end, and the rows of
the table count the union of those spans.  The idea's premise was that they bound what learning the images could save.
They do not, as section 2 shows, because the images act elsewhere too.

**The cheaper the exchange, the likelier its image is a conflict.**  At 3441-1262 the share is 95-96% under a price of
25, 90% at 25-49, 61% at 50-99 and 18-26% above.  At 3300-14 it is 80% under 25, 69% at 25-49, 56% at 50-99 and 38% at
100-199.  Its 71% above 200 comes mostly from one swap, 1636 = 4·409 out for 1076 = 4·269 at 560 points, whose image
is a conflict every time (12,905 of the 20,101 images above 200 that are conflicts).  That is the swap that held as a
dominance on every sample of
[local exchanges](../local-prime-exchanges-at-the-nodes-2026-10/README.md#3-what-pruning-would-save).

**Small conflicts carry it.**  Images of conflicts of two members are conflicts 60-83% of the time, of three to five
29-49%, of six or more 26-33%.  The paired exchanges almost never move two members of the conflict: the image swaps the
member alone.

**By prime pair, the paired exchanges of close primes dominate.**  At 3441-1262, 67 ↔ 71 on 24 and 20 (1704 out for
1608, 1340 out for 1420; 16,966 times, 16 points), 199 ↔ 211, 103 ↔ 107 and 197 ↔ 199 give images that are conflicts
98-100% of the time.  At 3300-14, 61 ↔ 67 (the example above, 11,021 times) gives 94%, and 131 ↔ 137 100%.  The
expensive twin swaps of small primes do not: 17 ↔ 19 gives 14-60% and 11 ↔ 13 8-22%.  By the member's home, homes 2
and 3 give 42-83%, home 1 28-62%, and home 4 at 3441-1262 9%.

**The search already learns many images, late.**  Where the image is a conflict, the node's next conflict is the image
itself, found in the nogood table, 64% of the times at 3441-1262 (22% at 3300-14, 9% at 3441-5459).  So the search
does meet the images, but only after its bound has walked into them, and it forgets them with the rest of its table
every 50,000 steps.

## 2. Learning the images

**The learning makes every tree smaller, and the closure much smaller** (`analysis.txt`, "Learning the images"):

| unit | search | steps | change | leaves | change | CPU seconds | change | conflicts computed | conflicts learned in images |
|---|---|---|---|---|---|---|---|---|---|
| 3300-14 | alone | 1,248,609 | | 104,400 | | 284 | | | |
| | images | 1,020,828 | −18.2% | 84,881 | −18.7% | 232 | −18.3% | 3,613 | 5,386 |
| | closure | 790,609 | −36.7% | 66,012 | −36.8% | 189 | −33.6% | 2,549 | 18,699 |
| 3441-1262 | alone | 1,099,410 | | 88,755 | | 352 | | | |
| | images | 744,669 | −32.3% | 57,475 | −35.2% | 256 | −27.2% | 3,384 | 5,744 |
| | closure | 648,846 | −41.0% | 51,942 | −41.5% | 222 | −37.0% | 1,999 | 11,913 |
| 3441-5459 | alone | 52,855 | | 4,706 | | 15.3 | | | |
| | images | 48,598 | −8.1% | 4,252 | −9.6% | 14.4 | −5.9% | 150 | 188 |
| | closure | 27,073 | −48.8% | 2,473 | −47.5% | 8.3 | −45.6% | 115 | 474 |

The CPU seconds include everything the learning costs: the proposals, the peels that check them, and the larger
tables.  That cost is 5-7% a step with the closure, and the trees shrink by far more.  The closure proposes 3.0 to 4.7
times as many sets as the first images, learns 2.1 to 3.5 times as many conflicts, and needs fewer conflicts computed,
since the images supply some that the search would otherwise compute.  Every search found the optimum and completed:
with the incumbent at the optimum, nothing beats it.

**Why it helps.**  A conflict K that the search knows rules its last member a out of a child whose promotions hold the
rest of K; the bound then takes a's twin a' in its place, at the same small price as at an exclusion.  If K with a'
for a is a conflict as well and is known, it rules a' out at the same time, and the bound has to pay for both.  The
uses of the images' nogoods measured in section 3 agree.  They rule candidates out of children three to five times as
often as they give the conflict a node branches on.  This is why the steps under the exclusions of section 1 are no
bound: the images act wherever their conflicts are met, not only where the bound walked into them.

**The learning is sound by construction.**  The hook only proposes sets, and the searcher learns, of each, a subset
that a peel shows cannot be promoted from the node's state.  That subset is a conflict of the node, and of every node
below it, which have more promotions; its nogood is a minimal subset of the node's promotions and the conflict that
keeps a frame of the root unwinnable, so a nogood of the root, as `FrameGraph::nogood` makes for the search's own
conflicts.  These are the facts the search learns from its own conflicts, by the same functions, so the argument of
its exactness covers them ([DESIGN.md, "Forced members and nogoods"](../../docs/DESIGN.md#forced-members-and-nogoods)).
A set that can be promoted teaches nothing.  The tests check the rest: the test's hook leaves the tree as it is (game
1215), each image's verdict agrees with promoting it, and the learning finds the optimum of games 800, 1000 and 1215
from a floor 60 below it, with the closure and without.

## 3. The classes

With the closure, the classes of the two big units are many, and their use is spread over dozens of them
(`analysis.txt`, "Classes"; the files in `classes/`):

| | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| conflicts computed (classes) | 1,225 | 1,081 | 115 |
| computed more than once, after a forgetting | 475, at most 15 times | 343, at most 13 times | 0 |
| with a conflict learned in an image | 746 | 790 | 42 |
| with 6 or more distinct nogoods learned | 367 | 472 | 33 |
| steps that branched on a conflict an image's nogood gave | 72,668 | 82,641 | 2,350 |
| candidates such a nogood left alone in a child (an estimate of its rule-outs) | 379,393 | 255,585 | 7,680 |
| uses of the class used most | 3.1% | 4.0% | 21.6% |
| of the 10 used most | 19.9% | 22.6% | 85.6% |
| classes holding half the uses; four fifths | 42; 120 | 38; 120 | 3; 8 |
| classes never used | 545 | 363 | 75 |

A third to a half of the classes are of single-member conflicts, which have an image only when a twin of the member
cannot be promoted at the node either.  Many classes are large: 29-44% of them learn six or more distinct nogoods.
The estimate of rule-outs counts a candidate left alone by a nogood of the class in a child that promoted one of its
members; it also counts candidates that another conflict or the frames' freedoms rule out.

**No few knots carry the pruning.**  In the big units the class used most has 3-4% of the uses, and half of them take
about 40 classes; only the small contrast has three classes with half.  A precomputed exact value for one knot would
reach a few percent of the uses at most.  To matter, it would have to cover dozens of classes a unit, which the
learning already finds by itself as the search meets them.  So the other form of Brian's idea, precomputing a knot's
exact value, would add little beyond this one.

## Decision

**Keep the instrument, and take the learning toward the production search.**  Learning the closure of each computed
conflict under the cheap exchanges shrinks the three hard trees by 37-49% at 5-7% more cost a step, and it is sound by
construction.  Before it goes into the searcher it needs measuring where the chain uses it:

1. the whole tuning suite (its 40 units and 6 whole games) against the default, one thread, for the trees and the time;
2. whole games of the chain from their start incumbents on many threads, where each searcher has its own table;
3. then a tactic in the searcher, off until measured, with the price line (50) and the cap on a closure (64) as its
   knobs, and a test that the default search is unchanged.

The production search is unchanged here: everything new is behind `--features studies`.

## Files

* `runs.jsonl`: the lines of `wall images`, whose fields the module comment of
  [examples/wall/images.rs](../../frame/examples/wall/images.rs) gives: the test (`mode` `check`), the search alone
  (`bare`), the first images (`learn`) and the closure (`closure`) for each unit, and the closure again with `--trace`
  for the classes, marked `"run":"classes"` (the same trees, about 7% more CPU for the tracking).
* `classes/ID.classes.tsv`: the classes of the closure, as `--trace` writes them.
* `analyze.py` makes the tables: `./analyze.py runs.jsonl classes ROWS`, with `ROWS` a directory of the test's
  rows; `analysis.txt` is its output.  The rows, 25 MB a big unit, are not kept, because a one-thread solve of a unit
  is the same every time.  A first build ran the test before the line had `tree_nodes` and `leaves`; its rows were
  the same as the final build's, byte for byte, on all three units.
* From `solver/frame`, after `cargo build -j 2 --profile test --example wall --features studies`:
  `target/debug/examples/wall images --suite ../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl
  --known ../../results/strict.jsonl --ids 3300-14 [--trace DIR | --bare | --learn [--closure] [--trace DIR]] 3300`,
  and the same with `--ids 3441-1262,3441-5459` and `3441`.

Commits: the commit of this entry (the hooks `excluding` with the conflict, `propose` and `learned`, and `wall images`,
with its tests).

## Later (2026-10-20): the whole tuning suite

**Question.**  Does learning conflicts closed under the cheap exchanges (`wall images --learn --closure`, *the
learning* below) take as much off the search across [the tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md), its 40 units and 6 whole games, as it took off
the three hard units above?

**Answer.**  The gain holds across the suite, at less than half the size.  Over the 46 problems the learning takes
20.6% off the steps and 15.9% off the CPU seconds in total (720 of 4,523 s), against 37-49% and 34-46% on the three
units above (*the first trial* below).  The median problem gains 14.0% of the steps and 13.6% of the CPU, 33 of the 46
get faster, and on every one the best and the conclusion are the same.  The three units of the first trial are among
the best: they rank 4th, 7th and 8th by CPU, and the other 43 problems gain 17.4% of the steps and 12.8% of the CPU.
The gain grows with the tree.  The nine units whose default search takes 600,000 steps or more gain 24.4% of the steps
and 19.2% of the CPU (eight of the nine gain), and the twelve under 100,000 steps 8.2% and 6.4%.  The six whole games
gain least, 6.8% and 4.5%, and three of them cost more CPU.  The gap does not separate the problems.  In all, 13
problems cost more CPU than they save, nine of them units of fewer than 300,000 steps, whose trees move by tens of
percent when anything changes the nogoods the search holds.

### The runs of the suite

We solved each of the 46 problems twice with `wall images`, on one thread from the known optimum with fresh tables, as
the first trial solved its units: with `--count`, the default search with its tree counted, and with `--learn
--closure`.  The default search's steps are the suite's `default.nodes` on all 46 problems, so the trees are the
suite's.  The existing command loaded neither the whole games nor the leaves of the default search, so the studies build
has two small additions:

* `--root` makes the unit the whole game, the root unit with every candidate, as `tune solve` solves `GAME-root` and as
  `wall exchanges` and `wall near` take `--root`; the line's `id` is `GAME-root`.
* `--count` solves with the learning's hook, which proposes nothing: the default search, with `steps`, `tree_nodes` and
  `leaves` counted, which `--bare` cannot do.  On seven units the default search takes the same CPU time with the
  counter and with no hook, within 0.2%.

The test `the_count_finds_the_tree_of_the_search_and_root_names_the_whole_game` checks on game 1215 that the counter's
steps, tree nodes and leaves are the search's and the test's, that it proposes and learns nothing, and that `--root`
names the whole game as the suite does.

The runs went two at a time, each under `nice -n 10` and started when the machine's one-minute load average was at
most 8 (it was 2.2 to 5.9 at the starts), on this laptop (an M1 Max, the machine of the suite's CPU seconds) on
2026-10-19, from the build of `--profile test`; nothing ran on EC2.  The two runs of a problem went side by side,
the dearest problems first, so they met the same load.  Steps and leaves are exact and the same every time; the CPU
seconds carry the laptop's noise.  The comparison took 8,326 CPU seconds in 92 runs; the runs of the sections below
took 1,378 more, 9,704 in all (2.7 hours).

The *gap* of a unit is the suite's: the bound less what the node needs.  The suite has none for a whole game, so
`suite.py` takes the root gap of `wall landscape --known results/strict.jsonl N` (the bound over P*) less one: 923 for
3128 and 3129, 933 for 3213 and 834 for 3270, 3272 and 3275.  The gaps run from 746 to 1,001, in three bands: under 850
(17 problems), 850-949 (18) and 950 and over (11).

### What the tables show

The tables are at the end of this section; the reading, with the numbers from them:

* **Total and median.**  The learning takes 20.6% off the steps, 21.3% off the leaves and 15.9% off the CPU in total
  (the units 22.6% and 17.7%).  The median change of a problem is 14.0% of the steps and 13.6% of the CPU, and the
  geometric mean 17.5% and 13.7%.  The cost of a step rises 5.8% (6.2% on the units, 2.5% on the whole games), as it
  did in the first trial (5-7%), so the CPU falls by less than the steps.
* **By gap.**  The totals rise a little with the gap: 16.5%, 19.7% and 22.5% of the steps and 11.2%, 15.4% and 17.9% of
  the CPU in the three bands.  The medians (11.9%, 12.0% and 19.9% of the steps) and the geometric means do not rise
  steadily, and across the 46 problems the change in steps has no correlation with the gap (+0.03).  Within this range
  of the gap, 746 to 1,001, the gap does not predict the gain.
* **By the size of the tree.**  Totals of steps and CPU: 8.2% and 6.4% under 100,000 steps (12 problems), 10.4% and
  5.9% from 100,000 to 299,999 (14), 21.7% and 17.7% from 300,000 to 599,999 (11), and 24.4% and 19.2% from 600,000 up
  (9).  The size of the default tree correlates with the change in steps at −0.25.  The seven units of 300,000 to
  600,000 steps all gain 14-41% of the CPU, and the nine of 600,000 or more gain 10-47%, but for 3440-12.
* **By game.**  The units of 3441 gain most (29.3% of the steps, 24.6% of the CPU), then 3425 (25.0% and 20.0%) and
  3300 (24.3% and 19.5%), and those of 3440 least (8.9% and 6.0%; three of its ten cost more CPU).  The first trial's
  units were two of 3441 and one of 3300.
* **Whole games.**  They gain 6.8% of the steps and 4.5% of the CPU (median 1.2%).  Three of the six cost more CPU:
  3272 (3.0% more), 3129 (3.9%) and 3213 (10.4%).  The units of the same size gain more: the four whole games between
  300,000 and 600,000 steps gain 14% (3128) and 9% (3275) of the CPU and lose 4% (3129) and 10% (3213), against 14-41%
  for the seven units there.  What makes a whole game different is not measured.

### Where it costs more

The learning costs more CPU than the default search on 13 of the 46 problems, and takes more steps on nine of them
(the table "The problems where the learning costs more CPU than it saves", below).  Four things show.

**Small trees move by tens of percent under any change of the nogoods.**  Nine of the 13 are units of fewer than 300,000
steps, five of them under 100,000 (3425-147, 3441-2994, 3425-150, 3440-39 and 3300-39).  To see how far such a tree
moves without the learning, we ran the default search again on the twelve units under 100,000 steps with the tactic
`nogood_period` at 20,000 and at 30,000 instead of 50,000, settings that moved the mean steps of the [tuning
campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) by 5% or less.  The default's steps moved
from 55% fewer to 226% more, and the learning's from 63% fewer to 128% more.  The losses of 3441-2994 (128% more) and
3425-150 (70% more) lie inside the first range, and only the totals over many units say what the learning does.

**The default search under other nogood periods**, against the default (the learning's column is the learning's change
of the steps, from the tables below):

| problem | steps | nogood_period=20000 | nogood_period=30000 | learning |
|---|---|---|---|---|
| 3300-91 | 40,811 | +20.2% | +2.6% | −16.5% |
| 3300-39 | 84,880 | −34.1% | −22.9% | +3.0% |
| 3300-108 | 58,543 | −6.8% | +13.6% | −30.2% |
| 3425-150 | 48,615 | +23.1% | +29.8% | +70.0% |
| 3425-147 | 33,333 | +225.6% | +94.4% | +2.5% |
| 3440-100 | 61,285 | +10.1% | +9.0% | −63.2% |
| 3440-39 | 71,515 | +25.8% | +3.2% | +13.0% |
| 3440-102 | 71,009 | +4.4% | −2.1% | −14.6% |
| 3441-5459 | 52,855 | −11.7% | −5.1% | −48.8% |
| 3441-5231 | 82,172 | +17.1% | −55.1% | −45.3% |
| 3441-2994 | 36,500 | +70.0% | +7.3% | +127.7% |
| 3441-5634 | 76,412 | −17.9% | −3.9% | −20.5% |


**The images alone do not repair the losses.**  We ran the learning of the images alone (no closure) on 12 of the 13
problems too, and its columns sit beside the closure's in the table of losers.  It costs more CPU on seven of them,
3441-2994 (82% more) and 3425-59 (42%) among them, and takes 10% and 25% off 3425-150 and 3300-39, where the closure
costs 67% and 1.5% more.  Neither form is better on these problems, so the losses are not the closure's.

**A tree that shrinks little does not pay for the dearer step.**  The learning raises the cost of a step by 5.8% over
the suite.  On 3129-root the tree falls 9.7% in steps and the CPU rises 3.9%, because a step costs 15% more (0.180 ms to
0.207 ms; there the learning proposes 19.5 sets for each conflict computed, against 11.6 over the suite).  3300-98 (5.9%
fewer steps, a step 6.6% dearer), 3272-root (0.5% fewer, 3.3% dearer) and 3440-89 (2.2% fewer, 3.0% dearer) end with
0.2%, 3.0% and 0.7% more CPU.

**One large unit loses without an explanation.**  3440-12 (870,739 steps, gap 991) takes 5.5% more steps and 10.7% more
CPU.  Its exclusions look like those of the units that gain.  The test of the images, which we ran on it for this,
finds that 27.5% of its exclusions have a twin swap or a paired exchange and that 51.8% of their images are conflicts,
against 37.4% and 58.7% at 3300-14, 30.2% and 58.7% at 3441-1262 and 30.2% and 43.6% at 3441-5459 (the first trial's
lines).  So the test does not show why it loses, and 3440, the game that gains least, stays unexplained.

### Verdict

**The gain holds across the suite in the totals and for the large trees, and it is smaller than the three units of the
first trial suggested.**  The learning takes 15.9% off the CPU of the suite and 20.6% off its steps.  Eight of the nine
units of 600,000 steps or more and all seven of 300,000 to 600,000 gain 10-47% and 14-41% of the CPU.  Every one of
the 46 searches ended with the same best and conclusion as the default's, as the learning's soundness requires.  It does
not hold problem by problem: 13 of the 46 cost more CPU, the trees of fewer than 300,000 steps swing by tens of percent
either way, and the six whole games gain 4.5% of the CPU, with three of them dearer.

**Decision.**  Step 2 of the first trial's decision stands, and is the test that matters: whole games of the chain from
their start incumbents on many threads, where each searcher has its own table.  The expectation falls from the first
trial's 34-46% of the CPU to 15-20% for the large subtrees, and the whole games here say a whole game of this size may
gain only 5%.  The trend
with the size of the tree points to more for the largest games, but the suite's largest unit has 1.2 million steps, so
it cannot show how far the trend goes.  Step 3, the tactic in the searcher, waits for step 2.  The production search is
unchanged: the two additions to `wall images` and the script are behind `--features studies` or outside the crate.

### The tables

The tables are `suite.py table suite-runs.jsonl`, as it printed them.  A problem's *learning* columns are its
`--learn --closure` run, and the *default* columns its `--count` run.

**Totals**: the change from the default search to the learning, with the median and geometric mean of the problems' changes

|  | problems | steps: total | median | geometric mean | leaves: total | CPU: total | median | geometric mean |
|---|---|---|---|---|---|---|---|---|
| all | 46 | −20.6% | −14.0% | −17.5% | −21.3% | −15.9% | −13.6% | −13.7% |
| units | 40 | −22.6% | −16.6% | −18.9% | −23.4% | −17.7% | −14.2% | −15.3% |
| whole games | 6 | −6.8% | −10.7% | −7.4% | −6.2% | −4.5% | −1.2% | −2.2% |

**By gap**

|  | problems | steps: total | median | geometric mean | leaves: total | CPU: total | median | geometric mean |
|---|---|---|---|---|---|---|---|---|
| gap under 850 | 17 | −16.5% | −11.9% | −20.5% | −17.7% | −11.2% | −9.2% | −16.5% |
| gap 850-949 | 18 | −19.7% | −12.0% | −10.1% | −20.0% | −15.4% | −10.5% | −6.7% |
| gap 950 and over | 11 | −22.5% | −19.9% | −24.2% | −23.4% | −17.9% | −15.8% | −20.0% |

**By the steps of the default search**

|  | problems | steps: total | median | geometric mean | leaves: total | CPU: total | median | geometric mean |
|---|---|---|---|---|---|---|---|---|
| steps under 100,000 | 12 | −8.2% | −15.5% | −13.0% | −6.0% | −6.4% | −13.6% | −9.9% |
| steps 100,000-299,999 | 14 | −10.4% | −9.1% | −11.8% | −11.7% | −5.9% | −4.1% | −8.3% |
| steps 300,000-599,999 | 11 | −21.7% | −26.2% | −22.6% | −21.5% | −17.7% | −22.2% | −18.2% |
| steps 600,000 and over | 9 | −24.4% | −19.9% | −25.2% | −25.7% | −19.2% | −15.8% | −20.7% |

Correlation (Pearson) of the log of the steps ratio, learning over default,
with the gap: +0.03; with the log of the default's steps: -0.25.

**By game**

|  | problems | steps: total | median | geometric mean | leaves: total | CPU: total | median | geometric mean |
|---|---|---|---|---|---|---|---|---|
| 3300 | 10 | −24.3% | −17.6% | −19.0% | −25.5% | −19.5% | −15.0% | −15.3% |
| 3425 | 10 | −25.0% | −20.8% | −13.7% | −25.8% | −20.0% | −15.9% | −10.7% |
| 3440 | 10 | −8.9% | −11.8% | −16.7% | −9.8% | −6.0% | −11.0% | −13.8% |
| 3441 | 10 | −29.3% | −30.7% | −25.8% | −29.6% | −24.6% | −25.9% | −20.9% |
| whole games | 6 | −6.8% | −10.7% | −7.4% | −6.2% | −4.5% | −1.2% | −2.2% |

**Each problem, by gap**

| problem | gap | steps | learning | change | leaves | learning | change | CPU s | learning | change |
|---|---|---|---|---|---|---|---|---|---|---|
| 3440-100 | 746 | 61,285 | 22,537 | −63.2% | 5,006 | 1,916 | −61.7% | 20.3 | 8.2 | −59.7% |
| 3300-91 | 753 | 40,811 | 34,095 | −16.5% | 3,204 | 2,698 | −15.8% | 11.3 | 9.4 | −17.3% |
| 3440-39 | 754 | 71,515 | 80,808 | +13.0% | 5,494 | 6,340 | +15.4% | 28.2 | 32.4 | +15.0% |
| 3441-5634 | 776 | 76,412 | 60,741 | −20.5% | 6,634 | 5,160 | −22.2% | 17.8 | 15.5 | −13.2% |
| 3441-5231 | 782 | 82,172 | 44,965 | −45.3% | 6,538 | 3,921 | −40.0% | 23.4 | 14.6 | −37.7% |
| 3425-59 | 793 | 160,283 | 188,524 | +17.6% | 13,046 | 15,633 | +19.8% | 53.9 | 68.1 | +26.3% |
| 3300-39 | 800 | 84,880 | 87,450 | +3.0% | 6,474 | 6,441 | −0.5% | 19.1 | 19.4 | +1.5% |
| 3300-108 | 802 | 58,543 | 40,884 | −30.2% | 4,503 | 3,153 | −30.0% | 12.4 | 9.3 | −25.0% |
| 3440-167 | 802 | 200,201 | 173,831 | −13.2% | 15,696 | 13,382 | −14.7% | 75.6 | 64.6 | −14.6% |
| 3441-4840 | 804 | 180,733 | 78,223 | −56.7% | 16,003 | 6,550 | −59.1% | 40.8 | 20.5 | −49.9% |
| 3441-571 | 820 | 316,028 | 176,756 | −44.1% | 30,144 | 17,478 | −42.0% | 69.5 | 40.9 | −41.1% |
| 3425-147 | 834 | 33,333 | 34,170 | +2.5% | 2,665 | 2,764 | +3.7% | 10.1 | 10.5 | +4.2% |
| 3275-root | 834 | 305,113 | 269,193 | −11.8% | 23,512 | 20,656 | −12.1% | 85.0 | 77.2 | −9.2% |
| 3272-root | 834 | 277,366 | 275,959 | −0.5% | 21,281 | 20,953 | −1.5% | 75.4 | 77.7 | +3.0% |
| 3270-root | 834 | 283,156 | 249,531 | −11.9% | 22,307 | 19,446 | −12.8% | 76.3 | 72.2 | −5.4% |
| 3425-13 | 838 | 167,978 | 175,955 | +4.7% | 13,816 | 14,449 | +4.6% | 44.2 | 47.2 | +6.6% |
| 3300-97 | 843 | 130,890 | 119,305 | −8.9% | 10,257 | 9,088 | −11.4% | 31.1 | 29.3 | −5.6% |
| 3440-101 | 850 | 153,309 | 138,978 | −9.3% | 12,824 | 11,193 | −12.7% | 43.4 | 42.2 | −2.9% |
| 3300-98 | 854 | 203,516 | 191,487 | −5.9% | 16,134 | 14,740 | −8.6% | 46.4 | 46.5 | +0.2% |
| 3440-9 | 855 | 152,705 | 136,500 | −10.6% | 12,313 | 11,212 | −8.9% | 48.8 | 43.2 | −11.3% |
| 3300-11 | 865 | 530,438 | 377,816 | −28.8% | 45,885 | 33,757 | −26.4% | 111.3 | 85.4 | −23.3% |
| 3440-102 | 869 | 71,009 | 60,614 | −14.6% | 4,962 | 4,061 | −18.2% | 25.5 | 21.9 | −13.9% |
| 3425-149 | 872 | 253,336 | 182,116 | −28.1% | 22,661 | 15,893 | −29.9% | 56.0 | 39.6 | −29.2% |
| 3425-150 | 879 | 48,615 | 82,660 | +70.0% | 4,215 | 7,459 | +77.0% | 12.8 | 21.5 | +67.1% |
| 3425-78 | 879 | 423,314 | 279,358 | −34.0% | 36,721 | 24,805 | −32.5% | 103.4 | 70.6 | −31.7% |
| 3425-60 | 901 | 721,555 | 610,817 | −15.3% | 59,889 | 50,642 | −15.4% | 230.6 | 208.4 | −9.6% |
| 3441-2994 | 904 | 36,500 | 83,098 | +127.7% | 3,332 | 7,895 | +136.9% | 9.9 | 21.9 | +122.3% |
| 3440-89 | 905 | 166,262 | 162,601 | −2.2% | 14,157 | 13,420 | −5.2% | 66.6 | 67.0 | +0.7% |
| 3128-root | 923 | 436,384 | 377,696 | −13.4% | 33,440 | 30,040 | −10.2% | 225.7 | 194.2 | −13.9% |
| 3129-root | 923 | 304,091 | 274,548 | −9.7% | 25,959 | 23,433 | −9.7% | 54.6 | 56.8 | +3.9% |
| 3441-1854 | 926 | 167,782 | 163,138 | −2.8% | 12,241 | 12,391 | +1.2% | 47.9 | 46.6 | −2.6% |
| 3425-131 | 931 | 709,361 | 356,228 | −49.8% | 61,669 | 29,654 | −51.9% | 174.9 | 93.0 | −46.8% |
| 3213-root | 933 | 450,606 | 469,256 | +4.1% | 38,453 | 40,218 | +4.6% | 104.5 | 115.4 | +10.4% |
| 3425-14 | 936 | 530,240 | 391,460 | −26.2% | 42,400 | 30,689 | −27.6% | 167.9 | 130.6 | −22.2% |
| 3440-90 | 941 | 387,319 | 276,847 | −28.5% | 33,495 | 23,117 | −31.0% | 142.9 | 111.1 | −22.2% |
| 3441-2978 | 953 | 221,178 | 199,156 | −10.0% | 18,084 | 16,542 | −8.5% | 67.1 | 62.8 | −6.4% |
| 3300-12 | 959 | 449,366 | 373,726 | −16.8% | 35,559 | 29,930 | −15.8% | 110.5 | 94.9 | −14.1% |
| 3441-5459 | 960 | 52,855 | 27,073 | −48.8% | 4,706 | 2,473 | −47.5% | 15.2 | 8.3 | −45.2% |
| 3425-2 | 970 | 495,503 | 355,231 | −28.3% | 41,685 | 29,749 | −28.6% | 156.0 | 118.5 | −24.0% |
| 3441-1262 | 970 | 1,099,410 | 648,846 | −41.0% | 88,755 | 51,942 | −41.5% | 342.3 | 220.1 | −35.7% |
| 3441-255 | 971 | 1,024,953 | 820,515 | −19.9% | 82,170 | 64,633 | −21.3% | 312.3 | 261.8 | −16.2% |
| 3300-83 | 978 | 836,173 | 645,787 | −22.8% | 67,720 | 50,722 | −25.1% | 216.6 | 182.5 | −15.8% |
| 3300-13 | 986 | 884,160 | 720,959 | −18.5% | 74,658 | 58,128 | −22.1% | 200.9 | 172.2 | −14.3% |
| 3440-12 | 991 | 870,739 | 918,933 | +5.5% | 70,916 | 75,099 | +5.9% | 256.6 | 284.1 | +10.7% |
| 3300-14 | 998 | 1,248,609 | 790,609 | −36.7% | 104,400 | 66,012 | −36.8% | 275.8 | 184.2 | −33.2% |
| 3440-13 | 1001 | 676,043 | 588,650 | −12.9% | 54,875 | 47,478 | −13.5% | 202.7 | 180.9 | −10.7% |

**The problems where the learning costs more CPU than it saves**, with the learning of the images alone

| problem | gap | default steps | learning: steps | CPU | default: ms a step | learning: ms a step | images alone: steps | CPU |
|---|---|---|---|---|---|---|---|---|
| 3425-147 | 834 | 33,333 | +2.5% | +4.2% | 0.304 | 0.309 | +6.3% | +6.5% |
| 3441-2994 | 904 | 36,500 | +127.7% | +122.3% | 0.270 | 0.264 | +94.1% | +82.4% |
| 3425-150 | 879 | 48,615 | +70.0% | +67.1% | 0.264 | 0.260 | −7.1% | −10.2% |
| 3440-39 | 754 | 71,515 | +13.0% | +15.0% | 0.395 | 0.402 | −2.5% | −0.8% |
| 3300-39 | 800 | 84,880 | +3.0% | +1.5% | 0.225 | 0.221 | −20.3% | −25.1% |
| 3425-59 | 793 | 160,283 | +17.6% | +26.3% | 0.336 | 0.361 | +38.7% | +42.2% |
| 3440-89 | 905 | 166,262 | −2.2% | +0.7% | 0.400 | 0.412 | −0.2% | −2.3% |
| 3425-13 | 838 | 167,978 | +4.7% | +6.6% | 0.263 | 0.268 | +3.8% | +5.8% |
| 3300-98 | 854 | 203,516 | −5.9% | +0.2% | 0.228 | 0.243 | +10.3% | +8.9% |
| 3272-root | 834 | 277,366 | −0.5% | +3.0% | 0.272 | 0.281 | +0.8% | +4.8% |
| 3129-root | 923 | 304,091 | −9.7% | +3.9% | 0.180 | 0.207 | −7.7% | −2.0% |
| 3213-root | 933 | 450,606 | +4.1% | +10.4% | 0.232 | 0.246 | +8.1% | +16.9% |
| 3440-12 | 991 | 870,739 | +5.5% | +10.7% | 0.295 | 0.309 | - | - |

13 of 46 problems; the learning takes more steps on 9.

**The cost of a step**

|  | default: ms a step | learning: ms a step | change |
|---|---|---|---|
| all | 0.280 | 0.297 | +5.8% |
| units | 0.277 | 0.294 | +6.2% |
| whole games | 0.302 | 0.310 | +2.5% |

**What the counting hook costs**: the default search with no hook (bare) and with it (count)

| problem | steps | bare: CPU s | count: CPU s | change |
|---|---|---|---|---|
| 3300-91 | 40,811 | 11.30 | 11.32 | +0.1% |
| 3425-150 | 48,615 | 12.84 | 12.85 | +0.0% |
| 3425-147 | 33,333 | 10.13 | 10.12 | −0.1% |
| 3440-100 | 61,285 | 20.26 | 20.27 | +0.0% |
| 3441-5459 | 52,855 | 15.23 | 15.21 | −0.1% |
| 3441-2994 | 36,500 | 9.84 | 9.85 | +0.1% |
| 3441-5634 | 76,412 | 17.80 | 17.83 | +0.2% |

Checks over 46 problems: all agree: the same best and conclusion, the default's steps the suite's.

CPU seconds of the 112 runs in the file: 9,268

### Files of this section

* `suite.py`: runs the suite (`run`, two runs at a time, waiting while the load average is over 8, one line a run as it
  ends, no run made twice, stopping when a search differs from the other in its best or conclusion or when the default
  search's steps are not the suite's) and makes the tables (`table`, `spread`).  Its module comment has the options.
* `suite-runs.jsonl`: a line of `wall images` for each run, with the problem's `gap`, the one-minute `load` at its start
  and its `wall_s` added: the 46 `count` and 46 `closure` runs of the comparison, 12 `learn` (the images alone), 7
  `bare` (no hook) and one `check` (the test, at 3440-12).
* `suite-perturbed.jsonl`: the default search (`count`) of the twelve units under 100,000 steps with `nogood_period`
  at 20,000 and at 30,000.

After `cargo build -j 2 --profile test --example wall --features studies` in `solver/frame` (the runs used a copy of the
binary, named with `--bin`, so that a rebuild during them could not replace it), from this folder, as `python3 -I
suite.py ...`:

```text
suite.py run suite-runs.jsonl --bare 3441-5459,3425-147,3441-2994,3300-91,3425-150,3440-100,3441-5634
suite.py run suite-runs.jsonl --modes learn --only 3300-39,3300-98,3425-150,3425-59,3425-147,3425-13,3440-39,3440-89,
    3441-2994,3129-root,3213-root,3272-root
suite.py run suite-runs.jsonl --modes check --only 3440-12
suite.py run suite-perturbed.jsonl --modes count --tactic nogood_period=20000 --only 3300-91,3300-39,3300-108,
    3425-147,3425-150,3440-100,3440-39,3440-102,3441-5459,3441-2994,3441-5231,3441-5634
suite.py table suite-runs.jsonl
suite.py spread suite-runs.jsonl suite-perturbed.jsonl
```

The first command makes the 92 runs of the comparison and the seven `bare` runs.  The same `run` with `--tactic
nogood_period=30000` makes the other half of `suite-perturbed.jsonl`.

Commits: the commit of this section (`wall images --root` and `--count` with their test, and `suite.py`).
