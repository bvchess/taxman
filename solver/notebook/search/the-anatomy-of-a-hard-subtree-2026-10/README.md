# The anatomy of a hard subtree (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-18.  Concluded 2026-10-18.

**Question.**  What, in Taxman terms, makes one hard search big: which numbers, which cores, which decisions, and why
is the tree as big as it is?  **Answer.**  The proof pays the gap in small, fixed prices.  Leaving one member of a core
out costs the GF(2) bound only about a tenth of that member's value, because the bound at once puts in a near-equal
number of the same home, often the member with one prime exchanged for a smaller one (1292 = 2²·17·19 gives way to
1156 = 2²·17²).  So a gap of 1,000 takes 5 to 8 such exclusions on every path, and the tree enumerates every way of
choosing them.  This is a diagnosis, not a method.

## Method

Three units of the [tuning suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) were solved with a
trace of every step: the biggest, another big one, and a small subtree of the same gap for contrast.

### Words

* A *unit* is a node of a search with the candidates its loop has left; solved alone, its subtree is a proof that
  nothing in it beats the optimum.  Its *gap* is its GF(2) bound less what a completion needs.
* A candidate's *home* is the frame it is promoted out of; it moves into the frame in front, home − 1.
* A *core* here is a conflict: candidates that cannot all be promoted from the node.  The search branches on one member
  of it: first the child that promotes the member, then the node goes on without it.  Going on without a member is an
  *exclusion*: every completion below leaves that member out.
* The *price* of an exclusion is how much the node's bound falls when it goes on without the member.
* A member's *live factors* are its maximal factors that are still factors of the frame it moves into: 1292 = 2²·17·19
  has the maximal factors 646, 76 and 68.
* Two numbers are *twins* when they share a maximal factor: x = m·p and y = m·q with p and q prime.  1292 = 68·19 and
  1156 = 68·17 are twins, sharing 68.
* The other terms, such as conflict, forced member, strong branching and step, are the
  [glossary's](../../../../docs/GLOSSARY.md).

### What was run

* **A hook on the searcher,** the solver's one change, inert by default.  `search::StepHook`
  ([src/search/hooks.rs](../../../frame/src/search/hooks.rs)) is told after every step what the step saw and did, and
  may choose which member of a conflict to branch on, which keeps the search exact.  The trees are unchanged: the three
  units take exactly the suite's steps, and the test of `wall anatomy` runs a whole search with and without the hook.
* **`wall anatomy`** ([examples/wall/anatomy.rs](../../../frame/examples/wall/anatomy.rs), with a test) solves units as
  `tune solve` does: one thread, fresh tables, from the known optimum.  `--trace` writes a line a step; `--prefer FILE`
  branches in an order of its own; `--best-cpu S` looks for the unit's best completion by large neighborhood search
  below the node (not sound) and lists the heaviest set's minimal conflicts.
* **The units,** each solved fully with the trace on 2026-10-18, on the laptop, one thread: 3300-14, the suite's biggest
  (17 promotions, 856 candidates, gap 998, 1,248,609 steps); 3441-1262 (10 promotions, 911 candidates, gap 970,
  1,099,410 steps); and the contrast 3441-5459 (44 promotions, 867 candidates, gap 960, 52,855 steps, the smallest
  subtree in the suite at a gap above 950).  Then the best completion of each, for 120 CPU seconds; five branching
  orders on the contrast and one on 3300-14; and the first 30,000 steps of all 40 units of the suite.  About 70 CPU
  minutes in all, none on EC2.

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

* **Shape: chains, deep.**  Three nodes in five have one child, and half the steps are below depth 47, where a node
  has 0.99 children on average; the top ten levels hold 0.1% of the steps.
* **What is branched on: the members of the big cores of frames 1-3.**  The top levels of each tree decide the heaviest
  set's biggest core, member by member: 3300-14 promotes 1485, 1640, 1400 and 1380 in turn, all of the 16-member core
  of frame 3, and promoting 1485 heads 1,219,770 steps against 28,836 for going on without it.  Most members have three
  live factors (44-56% of the branching steps), and half the branching steps promote two or more forced members at once.
* **The gap falls only by exclusions, at fixed prices.**  Promoting the member strong branching picks leaves the gap
  where it was (a median change of 0).  An exclusion lowers it by a median 125, 144 and 153, 10-12% of the member's
  value.  Leaves close when the gap is below the price of every exclusion left, at a median gap of 94, 142 and 169.  So
  a path needs about (gap − closing gap) / price exclusions, 7.2, 5.8 and 5.2, against 7, 8 and 5 measured.  Cheap
  exclusions add depth without paying: a fifth of the big units' exclusions cost the bound under 50 points, against
  3.7% at the contrast.  A node's subtree grows a decade per 238-251 points of its gap, and at the same gap the
  contrast's subtrees are 3 to 10 times smaller.
* **Repetition: none of whole problems, much of cores.**  Of 512,966 nodes of 3300-14, none has the frames' states and
  candidates of another, and no split of the frames makes the tree a product (the distinct states in front are nearly
  as many as the nodes, against products of 10⁹ to 10¹¹).  What repeats is the cores: 20,825 distinct conflicts are
  resolved 735,643 times, 35 times each on average; 49-73% of the lighter of a heavy node's two biggest children's
  conflicts are the heavier child's too (29-86% at 3441-1262, 59-78% at the contrast); and a core's price does not
  depend on where it is met (leaving 1364 out of {1155, 1364} costs 88 in each of its 5,293 occurrences; the spread of
  the price over 1,830 repeated pairs is a median 0.06 of the price, 0.11 and 0.12 at the other two).
* **The cores are small, overlapping and in frames 1-3; the heaviest set's own are big.**  The conflicts the branching
  resolves have a median size of 5 at 3300-14 and 7 at 3441-1262, and the 20,780 of 3300-14 hold only 226 numbers; the
  heaviest set's own minimal conflicts have 16 members in frame 3 and 33 in frame 1.  So the tree is a product, but of
  core resolutions, not of frames: about 5-8 exclusions in sequence, each among about 4-5 alternatives, the same cores
  met again under every combination of the others.
* **The error sits mostly at home 3.**  At 3300-14 the search below the node found a completion worth 226,503, 41 short
  of beating the optimum, in 60 and in 120 CPU seconds; the bound is 227,543.  The two sets have 165 promotions and
  differ by 23 swaps.  The bound's error by home is 296 at home 1, 160 at home 2, 434 at home 3, 150 at home 4 and 0
  above, 1,040 in all.  The other two units' completions were 839 and 619 short, too far to place the error.
* **The order of branching is worth less than a factor of two.**  At the contrast every other order is bigger than
  strong branching's 52,855 steps:

  | order | steps | against strong branching |
  |---|---|---|
  | members of the most often resolved conflicts first | 75,206 | 1.42 |
  | the largest first | 74,466 | 1.41 |
  | home 2 first | 72,805 | 1.38 |
  | home 3 first | 96,583 | 1.83 |
  | `branching=largest`: no strong branching at all | 70,214 | 1.33 |

  At 3300-14 the largest first took 1,975,229 steps, 1.58 times strong branching's 1,248,609.  Strong branching already
  picks the member whose exclusion costs the most, and no order changes the prices or the gap they must pay.
* **The substitutes are prime exchanges.**  Of 385,000 exclusions in the first 30,000 steps of the 40 units, 47-63% swap
  the member for exactly one other candidate, of the same home in 96-100% of those, and the substitute is a twin or a
  close kin of the member (sharing a divisor m, x = m·a and y = m·b with a and b at most 16) in 57-69% of the swaps.  A
  twin swap exchanges one prime for another on one cofactor, at the price (p − q)·m, and a third of the two-for-two
  swaps are the same exchange on two cofactors at once (at 3441, 1704 = 24·71 out brings 1608 = 24·67 in and swaps
  1340 = 20·67 for 1420 = 20·71, a price of (71 − 67)·(24 − 20) = 16).  That is the formula of the solver's symmetric
  pairs ("prime exchange" in the glossary) on two numbers instead of the whole board; 31-41% of all swaps are one
  exchange of two primes.  The twin swaps met most often at 3300 are 1197 for 1071 (price 126, 3,291 times), 1292 for
  1156 (136, 2,916) and 1274 for 1078 (196, 2,817).  A node does not walk down a twin's ladder at once: after a twin
  came in, it branched next on that twin in only 1-3% of the cases.
* **The price of the first exclusions does not forecast a unit's size.**  If a tree is about b^(gap/price), the gap over
  the price should predict a unit's size better than the gap does.  With the price measured in the first 30,000 steps
  of each of the 40 units it does not: log10 of the size against the gap has r² 0.49, against the gap over the price
  0.30.

The coordinator's three questions after [A5's
test](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md) have these answers.  Home 3 carries
the most error at 3300-14 (434 of 1,040), less concentrated than at A5's roots.  The branching spends its tree partly
on that home's candidates, which head 30% of the steps from 17% of the branching steps.  Each is a member of a big core
with a near-equal substitute in its own home, which is why the bound prices leaving it out at a tenth of its value.

## Decision

A diagnosis: the hook and the instrument stay, and the search is unchanged (with no hook its trees are the same).
Three hypotheses for a Taxman-specific method, each with a cheap test, were left open:

1. **Count the prices instead of searching them.**  An exclusion's price is fixed by its core and member to within
   6-12% over thousands of contexts, so a dynamic program over the cores' price lists, or a memo of each core's
   resolution, could replace the enumeration.  *Test:* from the trace of 3300-14, count the combinations of one
   exclusion per core met along a path whose total stays under the gap; the tree is that count if it matches the
   104,400 leaves within a factor of a few.
2. **Prices are arithmetic.**  A price is (p − q)·m, readable from the factorizations without solving the bound, and a
   bound or branching that charges a whole ladder of near-equal movers at once would raise the price per decision.
   *Test:* the share of the frequent prices that equal the member less a twin or kin among the node's candidates, and
   how often a path's later exclusions descend the same ladder.
3. **Local prime exchanges are near-symmetries.**  Where an exchange of two primes on one or two cofactors maps the
   frames the swapped numbers touch onto themselves, the branch that leaves the heavier side out could leave out the
   lighter side too, which matters most for the cheap exclusions (at 3441-1262 the two most frequent, 1704 out of
   {1648, 1704} at 16 points, 13,486 times, and 1592 out of {1592, 1668} at 8 points, 5,401 times, are both paired
   exchanges).  *Test:* at the node, check whether exchanging p and q maps the swapped numbers' live factors and
   composites onto each other's.  [Local prime exchanges at the
   nodes](../local-prime-exchanges-at-the-nodes-2026-10/README.md) ran it, and its answer is no.

## Files

* `analyze.py` makes the tables: `python3 -I analyze.py TRACE_DIR RUNS.jsonl...` for the traced units, `--orders
  RUNS...` for the branching orders, and `--model SUITE TRACE_DIR RUNS...` for the 40 units' first steps.
  `analysis.txt` is its output on the three traced units, the orders and the model.
* `orders.py` makes the branching orders: `python3 -I orders.py TRACE_DIR ID cores|largest|home H`.  `orders/` holds
  them (`3300-14.largest`, and `3441-5459.cores`, `.home2`, `.home3` and `.largest`).
* `runs.jsonl` holds the lines of `wall anatomy`, with a field `run` added: `traced` for the three traced units with
  their best completions and cores, `order` for the orders, and `first steps` for the 40 units.

The traces, 130-150 bytes a step and 350 MB for the three units, are not kept, because a one-thread solve of a unit is
the same every time; the three were made before the trace had the columns `gain` and `loss`, which a trace made again
has.  From `solver/frame`, after `cargo build --profile test --example wall --features studies`:

```
nice -n 10 target/debug/examples/wall anatomy \
  --suite ../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl \
  --known ../../results/strict.jsonl --ids 3300-14 --best-cpu 120 --trace DIR 3300
```

and the same with `--ids 3441-5459,3441-1262 ... 3441`.  The orders add `--prefer FILE --nodes 1000000` (`--nodes
3750000` for 3300-14), with `FILE` made by `orders.py` from the trace; the model runs `--nodes 30000 --trace DIR` on
the units of each game.

## Commits

* `46c4e76` (2026-10-18): `search::StepHook` and `wall anatomy`, with its test.
* `6794492` (2026-10-18): this entry.
