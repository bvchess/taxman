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
