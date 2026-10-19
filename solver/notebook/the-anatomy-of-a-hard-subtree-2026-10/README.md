# The anatomy of a hard subtree (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  What, in Taxman terms, makes one hard search big: which numbers, which cores, which decisions, and why
is the tree as big as it is?

**Answer.**  The proof pays the gap in small, fixed prices.  Leaving one member of a core out costs the GF(2) bound only
about a tenth of that member's value, because the bound at once puts in a near-equal number of the same home, often the
member with one prime exchanged for a smaller one (1292 = 2²·17·19 gives way to 1156 = 2²·17²).  So a gap of 1,000
takes 5 to 8 such exclusions on every path, and the tree enumerates every way of choosing them.

This is a diagnosis, not a method.  It follows the seven questions of the brief on three units of the [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md): the biggest, another big one, and a small
subtree of the same gap for contrast.  Three hypotheses for a Taxman-specific method close the entry, each with a cheap
test.

## Words

* A *unit* is a node of a search with the candidates its loop has left; solved alone, its subtree is a proof that
  nothing in it beats the optimum.  Its *gap* is its GF(2) bound less what a completion needs.
* A candidate's *home* is the frame it is promoted out of; it moves into the frame in front, home − 1.
* A *core* here is a conflict: candidates that cannot all be promoted from the node.  The search branches on one
  member of it: first the child that promotes the member, then the node goes on without it.  Going on without a member
  is an *exclusion*: every completion below leaves that member out.
* The *price* of an exclusion is how much the node's bound falls when it goes on without the member.
* A member's *live factors* are its maximal factors that are still factors of the frame it moves into: 1292 = 2²·17·19
  has the maximal factors 646, 76 and 68.
* Two numbers are *twins* when they share a maximal factor: x = m·p and y = m·q with p and q prime.  1292 = 68·19 and
  1156 = 68·17 are twins, sharing 68.
* The other terms, such as conflict, forced member, strong branching and step, are the
  [glossary's](../../frame/README.md#7-glossary).

## What was run

* **A hook on the searcher.**  This is the solver's one change, and it is inert by default.  `search::StepHook`
  ([src/search/hooks.rs](../../frame/src/search/hooks.rs)) is told, after every step, what the step saw and did: the
  node, its bound, heaviest set and forced members, its conflict and whether it was computed or known, the member
  branched on, the child's check and how the step ended.  It may also choose which member of a conflict to branch on,
  which keeps the search exact.  A search without a hook tests a `None` at each site, as the `Diagnostics` probe does.
  The trees are unchanged: the three units below take exactly the suite's steps, and the test of `wall anatomy` runs a
  whole search with and without the hook.
* **`wall anatomy`** ([examples/wall/anatomy.rs](../../frame/examples/wall/anatomy.rs), with a test).  It solves units
  as `tune solve` does: one thread, fresh tables, from the known optimum.  With `--trace` it writes a line a step, with
  what the heaviest set gained and lost since the node's last check, and hashes of each frame's state.  With `--prefer
  FILE` it branches in an order of its own.  With `--best-cpu S` it looks for the unit's best completion by large
  neighbourhood search below the node, which is not sound, and lists the heaviest set's minimal conflicts.
* **The units.**  Each was solved fully with the trace, on 2026-10-18, on this laptop, one thread, `nice -n 10`, from
  the build of `--profile test`:
  * 3300-14, the suite's biggest: 17 promotions, 856 candidates, gap 998, 1,248,609 steps;
  * 3441-1262: 10 promotions, 911 candidates, gap 970, 1,099,410 steps;
  * 3441-5459, the contrast: 44 promotions, 867 candidates, gap 960, 52,855 steps, the smallest subtree in the suite
    at a gap above 950.
* **Then:** the best completion of each, for 120 CPU seconds; five branching orders on the contrast and one on 3300-14;
  and the first 30,000 steps of all 40 units of the suite, traced.  `analyze.py` reads the traces and `analysis.txt` is
  its output.  About 70 CPU minutes in all, on one or two threads; nothing ran on EC2.

## Results

The three units side by side (`analysis.txt` has every table):

| | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| root gap | 998 | 970 | 960 |
| steps; nodes; leaves | 1,248,609; 512,966; 104,400 | 1,099,410; 437,940; 88,755 | 52,855; 22,441; 4,706 |
| nodes with one child | 59.5% | 59.5% | 58.2% |
| deepest stack; half the steps below depth | 107; 47 | 104; 47 | 61; 30 |
| steps ended by the GF(2) bound | 5.6% | 4.1% | 3.1% |
| steps ended by forced members | 42.0% | 42.0% | 47.6% |
| promotions that failed | 11.3% | 14.2% | 6.8% |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 8 (5-10) | 5 (4-7) |
| price of an exclusion, median | 125 | 144 | 153 |
| exclusions priced under 50 points | 19.9% | 20.6% | 3.7% |
| gap at which a leaf closes, median | 94 | 142 | 169 |
| leaves^(1/exclusions): alternatives an exclusion | 5.2 | 4.2 | 5.4 |
| median subtree of a node made at a gap of 700-799 | 10,916 | 5,033 | 1,047 |
| distinct conflicts; branching steps | 20,825; 735,643 | 38,918; 661,470 | 2,053; 30,414 |

**1. Shape: chains, deep.**  Three nodes in five have one child and a node takes about 2.4 steps.  The stack goes 104
to 107 deep in the big units, and half the steps are below depth 47, where a node has 0.99 children on average; the top
ten levels hold 0.1% of the steps.  A fifth of the nodes are leaves.

**2. What is branched on: members of the big cores of frames 1-3.**  The top levels of each tree decide the heaviest
set's biggest core, member by member, by promoting them:

* 3300-14 promotes 1485, 1640, 1400 and 1380 in turn, all of the 16-member core of frame 3 (homes 3 and 4: 1215, 1242,
  1260, 1288, 1350, 1360, 1377, 1380, 1386, 1400, 1428, 1476, 1485, 1608, 1640, 1648).  Promoting 1485 heads 1,219,770
  steps, and going on without it 28,836; then 1,192,614 against 27,153 for 1640; 1,074,790 against 117,821 for 1400;
  1,013,785 against 61,002 for 1380.  The gap stays at 998 for nine levels while the core is promoted.
* 3441-1262 first decides a core of four smooth numbers of homes 5 and 6: 1440 = 2⁵·3²·5 heads 781,224 steps and
  going on without it 318,183; then 1200 = 2⁴·3·5² 329,234 against 451,987.  Below it lie cores of 20, 26 and 32
  members in frames 2, 3 and 1.
* 3441-5459 promotes 1700, 1575, 1672 and 1500 in turn, of the 19-member core of frame 3, and the exclusions head 121
  to 803 steps each.

Over the whole of 3300-14, a branching step's member is of home 1 in 27% of the steps, home 2 in 31%, home 3 in 17% and
home 4 in 6%; the subtrees under them hold 27%, 28%, 30% and 13% of the steps.  At 3441-1262, home 3 is 48% of the
branching steps.  Most members have three live factors: 44-56% of the branching steps and 54-60% of the steps under
them; two live factors 25-37%, four 11-20%.  Half the branching steps promote two or more forced members at once.

**3. The gap falls only by exclusions, at fixed prices.**  Promoting the member strong branching picks leaves the gap
where it was: the median change is 0.  An exclusion lowers it by a median 125, 144 and 153 in the three trees, 10-12% of
the member's value.  Leaves close when the gap is below the price of every exclusion left, at a median gap of 94, 142
and 169, and the forced-member rule then ends the branch.  So a path needs about (gap − closing gap) / price
exclusions: 7.2, 5.8 and 5.2, against 7, 8 and 5 measured.  Cheap exclusions add depth without paying: a fifth of the
big units' exclusions cost the bound under 50 points, against 3.7% at the contrast.  Within a tree, a node's subtree
grows a decade per 238-251 points of its gap; at the same gap, the contrast's subtrees are 3 to 10 times smaller.

**4. Repetition: none of whole problems, much of cores.**  No two nodes face the same residual problem: of 512,966 nodes
of 3300-14, none has the frames' states and candidates of another, and neither do the frames' states alone.  No split
of the frames makes the tree a product either: at every split, the distinct states in front are nearly as many as the
nodes, against products of 10⁹ to 10¹¹ (`analysis.txt`).  What repeats is the cores, three ways:

* 20,825 distinct conflicts are resolved 735,643 times at 3300-14, 35 times each on average, and the most frequent ones
  are small: {1029, 1190, 1197} 12,962 times, {1155, 1364} 7,872, {1626, 1636} 7,492.
* The two biggest children of a heavy node resolve the same cores: 49-73% of the lighter child's conflicts are the
  heavier child's too at 3300-14, 29-86% at 3441-1262, 59-78% at the contrast.
* A core's price does not depend on where it is met.  Leaving 1364 out of {1155, 1364} costs 88 in each of its 5,293
  occurrences, 1292 out of {1155, 1292} 136 in each of 6,051, 1636 out of {1626, 1636} 560 in each of 4,588.  Over the
  1,830 pairs of a conflict and a member met 20 times or more at 3300-14, the spread of the price (p90 less p10) is a
  median 0.06 of the price; 0.11 and 0.12 at the other two.

**5. The cores: small, overlapping, in frames 1-3; the heaviest set's own are big.**  The conflicts the branching
resolves sit in frames 1-3: at 3300-14, a quarter in frame 1, a quarter among home 2's candidates alone, 19% in frame 3.
Their median size is 5 at 3300-14 and 7 at 3441-1262, and they overlap heavily: the 20,780 resolved at 3300-14 hold only
226 numbers, and a number of home 2 or 3 is in a median 276-310 of them.  The heaviest set's own minimal conflicts are
big: 16 members in frame 3 and 33 in frame 1 at 3300-14, most of them with three live factors.  So the tree is a
product, but of core resolutions, not of frames: about 5-8 exclusions in sequence, each among about 4-5 alternatives,
the same cores met again under every combination of the others.

**6. The heaviest set against the best completion: the error sits mostly at home 3.**  At 3300-14 the search below the
node found a completion worth 226,503, 41 short of beating the optimum, and found the same in 60 and 120 CPU seconds.
The bound is 227,543.  The two sets have the same size, 165 promotions, and differ by 23 swaps.  The bound's error by
home, the heaviest set's sum there less the completion's, is:

| home | 1 | 2 | 3 | 4 | 5-7 | all |
|---|---|---|---|---|---|---|
| error | 296 | 160 | 434 | 150 | 0 | 1,040 |

The completion leaves out 2 of the frame-3 core's 16 members (1260 and 1648) and 3 of the frame-1 core's 33 (1148,
1364 and 1410).  At 3441-1262 and 3441-5459 the search found only completions 839 and 619 short, too far to place the
error by home.

**7. Branching against bounding: the order is worth less than a factor of two.**  At the contrast, every other order
is bigger than strong branching's 52,855 steps:

| order | steps | against strong branching |
|---|---|---|
| members of the most often resolved conflicts first | 75,206 | 1.42 |
| the largest first | 74,466 | 1.41 |
| home 2 first | 72,805 | 1.38 |
| home 3 first | 96,583 | 1.83 |
| `branching=largest`: no strong branching at all | 70,214 | 1.33 |

At 3300-14 the largest first took 1,975,229 steps, 1.58 times strong branching's 1,248,609.  Strong branching already
picks the member whose exclusion costs the most, which shortens the paths, and no order changes the prices or the gap
they must pay.

**The substitutes are prime exchanges.**  The traces of the first 30,000 steps of the 40 units record what the heaviest
set gained when a node went on without a member.  Of 385,000 exclusions, 47-63% swap the member for exactly one other
candidate, of the same home in 96-100% of those.  The substitute is a twin of the member in 43-50% of the one-for-one
swaps, and a twin or a close kin in 57-69%; a kin shares a divisor m with it, x = m·a and y = m·b with a and b at most
16, as 1210 = 10·121 and 1089 = 9·121 do.  A twin swap exchanges one prime for another on one cofactor, and its price
is x − y = (p − q)·m.  A third of the two-for-two swaps are the same exchange on two cofactors at once: at 3441,
leaving 1704 = 24·71 out brings 1608 = 24·67 in and swaps 1340 = 20·67 for 1420 = 20·71, a price of
(71 − 67)·(24 − 20) = 16.  That is the formula of the solver's symmetric pairs ("prime exchange" in the
[glossary](../../frame/README.md#7-glossary)), applied to two numbers instead of the whole board.  In all, 31-41% of
the swaps are one exchange of two primes.  The twin swaps met most often:

| left out | its substitute | price | times |
|---|---|---|---|
| 1197 = 3²·7·19 | 1071 = 3²·7·17 | 126 | 3,291 |
| 1292 = 2²·17·19 | 1156 = 2²·17² | 136 | 2,916 |
| 1274 = 2·7²·13 | 1078 = 2·7²·11 | 196 | 2,817 |
| 1254 = 2·3·11·19 | 1122 = 2·3·11·17 | 132 | 2,720 |
| 1572 = 2²·3·131 | 1524 = 2²·3·127 | 48 | 2,155 |
| 1539 = 3⁴·19 | 1377 = 3⁴·17 | 162 | 2,188 |

The first five are at 3300 and the last at 3425; `analysis.txt` has eight a game.  A node does not walk down a twin's
ladder at once: after a twin came in, the node branched next on that twin in only 1-3% of the cases.

**The price of the first exclusions does not forecast a unit's size.**  If a tree is about b^(gap/price), the gap over
the price should predict a unit's size better than the gap.  With the price measured in the first 30,000 steps of each
of the 40 units it does not: log10 of the size against the gap has r² 0.49, against the gap over the price 0.30.  The
price at the top of a tree is not the price over the whole of it, so this does not refute the mechanism of question 3,
but it does not give a cheap forecast either.

**A5's three questions** (asked by the coordinator after [A5's
test](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)):

1. *Which home carries the bound's error?*  At 3300-14, home 3 most: 434 of 1,040, then home 1 with 296.  The error
   is less concentrated than at A5's roots.
2. *Does the branching spend its tree deciding that home's candidates?*  Partly.  The top of the tree decides the
   frame-3 core of homes 3 and 4, and home 3's members head 30% of the steps from 17% of the branching steps; homes 1
   and 2 head as many, 27% and 28%.
3. *What makes those candidates hard to decide?*  Each is a member of a big core, and each has a near-equal substitute
   in its own home, often itself with one prime exchanged for a nearby one.  So the bound prices leaving it out at a
   tenth of its value, and the search must leave out many of them, in every combination, before the gap is paid.

## Hypotheses for a Taxman-specific method

1. **Count the prices instead of searching them.**  Each exclusion's price is fixed by its core and member, to within
   6-12% over thousands of contexts, and a path dies once its prices add up to the gap less about one price.  So the
   tree enumerates the combinations of fixed prices that fit under the gap.  A method that treats the prices as data,
   such as a dynamic program over the cores' price lists or a memo of each core's resolution, could replace the
   enumeration.  *Cheap test*: from the trace of 3300-14, take each core's list of member prices and count the
   combinations, one exclusion per core met along a path, whose total stays under the gap.  If the count matches the
   tree's 104,400 leaves within a factor of a few, the tree is that count.
2. **Prices are arithmetic.**  The substitute is a twin or kin in most swaps, so a price is (p − q)·m, readable from
   the factorizations without solving the bound.  A payer m's composites m·p, over the primes p, form a ladder of
   near-equal movers, and leaving out the top one steps down one rung.  A bound or a branching that charges a whole
   ladder at once, for each payer that is the last one its movers have, would raise the price per decision.  *Cheap
   test*: the share of the frequent conflict-member prices of the three traces that equal the member less a twin or
   kin among the node's candidates, and how often a path's later exclusions descend the same ladder.  The 1-3% of
   immediate descents measured above say the ladder is not walked at once; whether it is walked along the path is open.
3. **Local prime exchanges are near-symmetries.**  The solver already uses exchanges of two primes over the whole
   board: a heaviest set keeps the lighter side of each symmetric pair, and the search cuts the rest.  A third to two
   fifths of the substitutions are the same exchange on one or two cofactors.  Where such a local exchange maps the
   frames the swapped numbers touch onto themselves, the completions on one side of it are beaten by their images, and
   the branch that leaves the heavier side out could leave out the lighter side too.  This matters most for the cheap
   exclusions, which add depth while paying almost nothing: at 3441-1262 the two most frequent are 1704 out of {1648,
   1704} at 16 points, 13,486 times, and 1592 out of {1592, 1668} at 8 points, 5,401 times; both are paired exchanges
   (67 ↔ 71 on 20 and 24; 197 ↔ 199 on 4 and 8).  *Cheap test*: for the frequent exchange swaps of the traces, check at
   the node whether exchanging p and q in the swapped numbers maps their live factors and composites onto each other's;
   the share that pass bounds the exclusions a local exchange rule could merge, and a high share at the cheap exchanges
   would say that most of the depth is symmetry the search does not see.

## Files

* `analyze.py` makes the tables: `python3 -I analyze.py TRACE_DIR RUNS.jsonl...` for the traced units, `--orders
  RUNS...` for question 7, and `--model SUITE TRACE_DIR RUNS...` for the 40 units' first steps.  `orders.py` makes the
  branching orders of question 7: `python3 -I orders.py TRACE_DIR ID cores|largest|home H`.
* `analysis.txt` is the output of `analyze.py` on the three traced units, the orders and the model.
* `runs.jsonl` holds the lines of `wall anatomy`, with a field `run` added: `traced` for the three traced units with
  their best completions and cores, `order` for question 7, and `first steps` for the 40 units.
* `orders/` holds the branching orders of question 7, made by `orders.py` from the traces.
* The traces, 130-150 bytes a step and 350 MB for the three units, are not kept, because a one-thread solve of a unit is
  the same every time.  The three were made before the trace had the columns `gain` and `loss`, which the 40 units'
  first steps have; a trace made again has them.  From `solver/frame`, after
  `cargo build --profile test --example wall --features studies`:
  `nice -n 10 target/debug/examples/wall anatomy --suite ../notebook/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl --known ../../results/strict.jsonl --ids 3300-14 --best-cpu 120 --trace DIR 3300`,
  and the same with `--ids 3441-5459,3441-1262 ... 3441`.  The orders add `--prefer FILE --nodes 1000000`
  (`--nodes 3750000` for 3300-14), with `FILE` made by `orders.py` from the trace; the model runs `--nodes 30000 --trace
  DIR` on the units of each game.

Commits: 46c4e76 (the hook and `wall anatomy`, with its test) and the commit of this entry.  The search is unchanged:
with no hook its trees are the same.
