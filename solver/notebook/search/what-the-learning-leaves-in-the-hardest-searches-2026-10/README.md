# What the learning leaves in the hardest searches (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  Once the search learns its conflicts' images under the cheap exchanges, where do a hard unit's steps
go, and which conflicts does it still have to compute: images of conflicts it met, under exchanges the learning does
not make, or something else?

**Answer, on two units of the old tuning suite.**  The learning leaves the anatomy of a tree as it was: the same
exclusions on a path, at the same prices, closing at the same gap.  It removes breadth, not depth: fewer alternatives
for each exclusion.  The conflicts it still computes are mostly ones it met before.  At 3300-14, 57% are a nogood the
search learned and then forgot at a clearing of its table, and 26% are a conflict met before whose nogood no longer
fits the node.  Exchanges outside the learning's class explain at most 8%.  So a wider closure has little to catch.
Keeping the nogoods used since the last clearing (the tactic `nogood_keep=hit`) is no answer either: it moves the trees
by the same factor with the learning as without it, 0.36-0.37 at 3300-14 and 1.46 at 3441-1262.

**Follow-ups, on the old suite (2026-10-20).**  None leaves an idea to keep.  Learning each computed conflict's nogood
under other witnesses makes the trees 14% larger on 22 units.  A whole game forgets as much at the clearings as a unit
does, step for step, and its image nogoods give fewer of its known conflicts.  Keeping the used nogoods does not make
the search branch on smaller conflicts.  ([Three follow-ups](#three-follow-ups-on-the-old-suite).)

**Stage 2, on today's hardest units (2026-10-20).**  The anatomy at 4416 is the old suite's at higher prices: 6-7
exclusions a path at a median of 156-176 points, with a thinner tail of cheap ones.  A bound that imposes the knot of a
node's conflict (its images under twin swaps and paired exchanges that are conflicts too) falls by about one exclusion
at every gap, so it closes only nodes that were closing anyway: cutting by it would save under 4% of the steps.  So
ladder branching is moot too.  ([Stage 2](#stage-2-todays-hardest-units-and-the-knot).)

**Closed (2026-10-20).**  None of this reopens the learning, which is closed, not adopted: [the
verdict](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted) has the
evidence.

The study follows [conflicts closed under cheap exchanges](../conflicts-closed-under-cheap-exchanges-2026-10/README.md),
whose learning took 20.6% off the steps of the tuning suite and only 4.5% off the CPU of its whole games.  It has three
stages: the instruments, built and tried on two units of the old [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) (this section); the same questions on a suite
of the chain's hardest searches near 4,500; and tests of the ideas they suggest.

## Words

* *The learning* is `wall images --learn --closure`: whenever a check computes a conflict, the search learns the
  conflicts within its images under the cheap twin swaps and paired exchanges, and the images of those, up to 64 sets
  ([cheap exchanges, "Words"](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#words)).
* *Unit*, *exclusion*, *price* and *home* are [the anatomy's](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words);
  *conflict*, *nogood* and *forced member* are the [glossary's](../../../../docs/GLOSSARY.md).
* A check's conflict is *computed* when the searcher's table of nogoods gives none within the heaviest set, and *known*
  when it does.  A known conflict comes from the search's *own* nogoods (those of the conflicts it computed) or from
  *image* nogoods (those the learning added).
* A nogood is a conflict of a node together with some of the node's promotions, enough to keep a frame of the root
  unwinnable: its *witness*.  Every nogood of these units lies in one frame: its members are of two adjacent homes.

## What was run

**The instrument.**  `wall anatomy` takes `--learn [--closure]` and `--root` (examples/wall/anatomy.rs, with its
tests).  With `--learn` the search learns as `wall images --learn` makes it, and the anatomy's trace of each step is
written as before.  Two things are new, with or without the learning:

* The hook keeps a mirror of the searcher's nogood table, with the source of each nogood, own or image.  The trace
  says, for each known conflict, which source gave it.  The mirror is exact: on every run here it explained every
  known conflict.
* `ID.nogoods.tsv` lists every nogood the search learned, in order, with the step, the conflict, and the node's
  promotions, heaviest set and candidates.

`residual.py` reads them.  `profile` puts two searches of a unit side by side, as the anatomy took one apart.  `misses`
takes each computed conflict and looks, among the nogoods learned before it, for one that a few moves of members turn
into a nogood that would have given the conflict at its node.  It names each move by the factorizations: a twin swap
(m·p for m·q), a kin move (m·u for m·v, u and v at most 16), a rescaled payer (m·p for m′·p), or unrelated.  A twin swap
is in the learning's class when the learning would have proposed it at the earlier nogood's node; if not, `misses` says
why (a swap up, a dear swap, a promoted member moved, a twin that was no candidate there).

**The units**: 3300-14, the old suite's biggest, and 3441-5459, its small contrast at the same gap
([the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md#what-was-run)).  Each was solved on one thread from
the known optimum with fresh tables, on this laptop under `nice -n 10`, with and without the learning, from the build
of `--profile test`.  The trees are those of the earlier studies, step for step: 1,248,609 and 790,609 steps at
3300-14, 52,855 and 27,073 at 3441-5459.  Every search found the optimum and completed.

## Where the steps go

The learning changes how many leaves a tree has, and little else:

| | 3300-14 default | learning | 3441-5459 default | learning |
|---|---|---|---|---|
| steps | 1,248,609 | 790,609 | 52,855 | 27,073 |
| leaves | 104,400 | 66,012 | 4,706 | 2,473 |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 7 (5-10) | 5 (4-7) | 5 (3-7) |
| price of an exclusion, median | 125 | 123 | 153 | 153 |
| exclusions priced under 50 | 19.9% | 19.7% | 3.7% | 6.2% |
| gap at which a leaf closes, median | 94 | 91 | 169 | 173 |
| leaves^(1/exclusions): alternatives an exclusion | 5.2 | 4.9 | 5.4 | 4.8 |
| a decade of subtree per ... points of gap | 238 | 241 | 245 | 262 |
| conflicts computed, of the checks with one | 0.8% | 0.5% | 0.8% | 0.7% |
| known from image nogoods (alone or with own) | 0 | 15.6% | 0 | 14.8% |
| distinct conflicts branched on; branchings a conflict | 20,780; 35.4 | 15,114; 30.8 | 2,027; 15.0 | 1,096; 14.4 |

* **What did not change.**  A path still pays the gap in 7 (or 5) exclusions at a median price of 123-153.  A fifth
  of 3300-14's exclusions are still cheap, under 50: the learning does not make the bound pay more for leaving a member
  out.  The subtree of a node still grows a decade for every 240-260 points of its gap.
* **What changed.**  The alternatives for each exclusion: leaves^(1/exclusions) falls from 5.2 to 4.9 and from 5.4 to
  4.8, and with 7 exclusions on a path that is the whole gain (0.94^7 ≈ 0.63).  The image nogoods give 15% of the known
  conflicts, and the same share of the conflicts the branching resolves; the rest of their work is ruling candidates
  out of children, which the trace does not count.
* **What a check computes.**  Under 1% of the checks that find a conflict compute it, with the learning or without; the
  rest find it in the table.  The learning cuts the computed conflicts by 58% at 3300-14, more than the steps (37%).

## What the learning misses

Each conflict the search still computes, in the first group that holds it (`residual.py misses`):

| group | 3300-14 default | learning | 3441-5459 default | learning |
|---|---|---|---|---|
| conflicts computed | 6,095 | 2,549 | 233 | 115 |
| the same nogood, met before and forgotten | 61.1% | 57.0% | 8.6% | 0.0% |
| the same conflict, another nogood | 28.4% | 26.0% | 37.8% | 34.8% |
| a cheap twin swap, the learning's class | 0.7% | 0.1% | 3.4% | 0.9% |
| one twin swap outside the class | 0.9% | 1.8% | 2.6% | 4.3% |
| two twin swaps | 1.9% | 2.6% | 6.4% | 6.1% |
| three twin swaps | 0.2% | 0.4% | 3.4% | 0.9% |
| kin moves or rescaled payers | 2.4% | 3.0% | 3.4% | 7.8% |
| unrelated moves, or none within 3 moves | 4.3% | 9.1% | 34.3% | 45.2% |

* **Forgotten nogoods.**  The searcher clears its table every 50,000 steps.  At 3300-14, 57% of the conflicts the
  learning still computes are nogoods it had learned before a clearing.  3441-5459 is solved in fewer steps than that.
* **The same conflict with another witness.**  A quarter to a third are a conflict the search had computed before,
  whose nogood no longer fits: its witness holds promotions the node has not made, and the node has others that
  serve.  Of the 662 such conflicts at 3300-14 with the learning, the closest earlier nogood of the same conflict
  holds one member that the node has neither promoted nor in its heaviest set in 57%, two in 14%, and none in 26%
  (that nogood would have fitted, and was forgotten); at 3441-5459, one in 34 of 40.  For example, the conflict {936} at 3300-14 had a witness with 1215 = 3⁵·5 in it; at a node without 1215 its
  new nogood holds 810 = 2·3⁴·5, 1215's twin, and 1404 instead.
* **Exchanges outside the learning's class** explain 8% at 3300-14 and 19% at 3441-5459 together, none of them more
  than 8%.  The largest are two twin swaps of one prime pair on two payers (2.6-6.1%), kin moves, twin swaps up, dear
  twin swaps, and twin swaps of a member that the earlier node had promoted.  Three-prime cycles and rescaled payers are
  rare.  Proposing every twin swap of a conflict's members at its node would cost 44-48 sets a computed conflict at
  3300-14, against the closure's 15.
* **The calibration.**  Without the learning, 0.7% (3300-14) and 3.4% (3441-5459) of the computed conflicts are first
  explained by a cheap twin swap that the learning proposes; with it, 0.1% and 0.9%.  So the instrument sees what the
  learning removes.

## A first test: keeping the used nogoods at a clearing

The largest group is the forgotten nogoods, so the cheapest test keeps them.  The tactic `nogood_keep=hit` keeps, at a
clearing, the nogoods used since the last one.  [Keeping and
forgetting](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md) found that it makes ten whole
games 28-31% larger on one thread.  The question here was whether the learning, whose classes a clearing throws away,
changes that.  It does not: keeping moves each unit by the same factor with the learning or without it, and the factor
swings from a third to one and a half.

| unit | default | `nogood_keep=hit` | change | learning | learning, `nogood_keep=hit` | change |
|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 464,165 | −62.8% | 790,609 | 453,073 | −42.7% |
| 3441-1262 | 1,099,410 | 1,602,571 | +45.8% | 648,846 | 950,397 | +46.5% |

The cells are steps; the CPU seconds follow them (308, 118, 204 and 117 at 3300-14; 342, 506, 220 and 326 at
3441-1262).  The best and the conclusion are the optimum's in all eight.  A unit's tree is a chaotic function of the
nogoods it holds, as [the tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) found, so
two units cannot say whether keeping pays on average; the earlier entry says it does not.  The run of 3440-12 was
stopped before it ended, to save the budget.

## Next

* **Stage 2** applies `profile` and `misses` to the suite of the chain's hardest searches, and to its whole games,
  to ask why whole games gain less than units.
* **The witness.**  A quarter of what the learning computes is a conflict it knew under another witness.  Each nogood
  lies in one frame, and its witness is promotions of the two homes that touch that frame.  A test: with each
  computed conflict, also learn its nogood with one witness member exchanged for a twin or kin among the root's
  candidates, each checked by a peel from the root.  That costs a few hundred peels for each computed conflict, and
  computed conflicts are under 1% of the checks.
* **Not a wider closure.**  The exchanges the closure lacks explain at most 8% of the computed conflicts at 3300-14.

## Three follow-ups on the old suite

Before stage 2, three questions that stage 1 left were each given a rough number on the old suite (2026-10-20).  None
leaves an idea to keep:

* **Learning the witness's exchanges makes the trees larger.**  It cuts the conflicts the search computes, as it was
  meant to, and the trees grow 14% on 22 units.
* **A whole game forgets as much as a unit does, step for step.**  The share of computed conflicts that are forgotten
  nogoods grows with the length of any search.  The image nogoods give fewer of a whole game's known conflicts.
* **Keeping the used nogoods does not make the search branch on smaller conflicts.**  At 3300-14 it branches on
  conflicts of the same size and fewer cheap ones; nothing points to a rule for choosing among conflicts.

Every search here completed with the optimum as its best.

### The witness's exchanges

**The test.**  The search learns each computed conflict's nogood again with one member of its witness exchanged.  For
each member `w` of the witness (the nogood's members outside the conflict), and each root candidate `y` outside the
nogood that touches a frame `w` touches and is `w`'s twin (`w = m·p`, `y = m·q`, `p` and `q` prime) or kin (`w = m·u`,
`y = m·v`, `u` and `v` at most 16), the searcher peels the nogood with `y` for `w` from the root.  A minimal conflict it
finds there is learned as a nogood; a conflict of the root holds at every node, so the search stays exact.  That is 519
sets peeled and 29 nogoods learned a computed conflict, on top of `--learn --closure`.  The prototype, `wall images
--learn --closure --witness` with a hook in the searcher behind the feature `studies`, is not in the code:
`witness.patch` adds it to commit 60976f3.

**The result.**  The units are 3300-14 and the 21 units of the suite whose closure took under 50 CPU seconds.  The
closure alone is the closure's lines of [cheap exchanges](../conflicts-closed-under-cheap-exchanges-2026-10/README.md)
(`suite-runs.jsonl`), which a run of 3441-5459 here repeated step for step.

| | the closure | the closure and the witness |
|---|---|---|
| steps, the 22 units | 2,812,162 | 3,217,865 (+14.4%) |
| steps, a unit's ratio: geometric mean; range | | 1.034; 0.44 (3425-13) to 2.97 (3441-571) |
| units larger | | 12 of 22 |
| conflicts computed | 10,167 | 8,819 |
| conflicts computed a thousand steps | 3.6 | 2.7 |
| CPU seconds | 733 | 971 (+32%) |

* **The exchanges do their job, and the tree does not care.**  A quarter fewer conflicts are computed for each step,
  and at 3441-5459 the nogoods rule 115,415 candidates out of children against 8,112.  But a conflict the search no
  longer computes is mostly the one the table now gives (stage 1's groups), and a node branches on a conflict alike
  whether it found it or computed it.  What moves the tree is the candidates the nogoods rule out, and that moves it
  both ways, as keeping the nogoods did in stage 1.
* **They cost CPU.**  The tables hold 2,300-5,400 nogoods at a clearing, and a lookup scans the whole table.  The CPU
  of the closure alone comes from the earlier entry's runs; on the two units run alone on the same day, the witness
  took 11% more CPU at 3300-14 (204 to 227 seconds) and 35% more at 3441-5459.
* **Decision: dropped.**  With it goes the aim of cutting the computed conflicts for their own sake: they are under 1%
  of the checks, and finding one instead of computing it saves a peel, not tree.

### Whole games and units

Stage 1 guessed that a whole game, one long search, loses more to the clearings than a unit.  It does not: the share
grows with the length of the search, and 3128-root and 3300-14 forget as much over the same steps.  The whole game was
traced with `wall anatomy --root --learn --closure` (377,696 steps, as in the earlier entry), and `residual.py windows`
follows each search period by period:

| | 3128-root | 3300-14 |
|---|---|---|
| steps | 377,696 | 790,609 |
| computed conflicts learned before, in the first period; the second | 0%; 35% | 0%; 30% |
| the same, over the first 400,000 steps; the whole search | 42.8%; 42.8% | 41.0%; 57.0% |
| the same, in each period after 500,000 steps | | 70-90% |
| known conflicts given by image nogoods, first 400,000 steps; the whole search | 12.4%; 12.4% | 16.5%; 15.7% |

* **The forgotten share is the length of the search.**  None of a period's computed conflicts are forgotten in the
  first period, a third in the second, and most after ten.  3128-root forgets 42.8% against 3300-14's 57.0% because it
  is half as long.  A searcher of a big run on many threads takes millions of steps, so most of the conflicts it
  computes will be nogoods it forgot; by the witness's test, that costs peels, not tree.
* **The image share is lower in the whole game.**  Its image nogoods give 12.4% of its known conflicts, against 16.5%
  for 3300-14 over the same steps and 14.8% for 3441-5459.  `misses` puts 18.1% of 3128-root's computed conflicts in
  none of the groups of moves, against 9.1% at 3300-14; the same conflict with another nogood is 27.2% (26.0%).

### The conflicts branched on, keeping the used nogoods or not

At 3300-14, keeping the used nogoods at a clearing (`nogood_keep=hit`) takes the tree from 1,248,609 steps to 464,165.
The guess was that what the table holds changes which conflict a node branches on, and that a better conflict (fewer
members, dearer exclusions) is the lever.  The test of `wall images`, which leaves the tree as it is, gives a row for
each exclusion with the size of the conflict and the price; `residual.py branched` compares the two searches:

| | default | `nogood_keep=hit` |
|---|---|---|
| exclusions | 345,237 | 131,989 |
| conflict size, p10/median/p90; mean | 2/3/5; 3.07 | 2/3/5; 3.18 |
| price, p10/median/p90 | 24/125/297 | 41/126/258 |
| priced under 50 | 19.9% | 14.0% |
| of those, conflicts of 2 members | 73.3% | 79.6% |
| paired exchanges: share of the exclusions (cheap: 99-100% of them) | 7.9% | 4.7% |
| twin swaps: share of the exclusions; cheap | 29.4%; 8.5% | 31.2%; 4.0% |

* **Not smaller conflicts.**  The conflicts are the same size, and the table already gives a node the smallest
  conflict it knows within the heaviest set.
* **Fewer cheap ones.**  What differs is the tail of cheap exclusions: 14.0% against 19.9%, most of them conflicts of
  two members, and fewer paired exchanges.  Strong branching takes a conflict's dearest member, so a cheap exclusion is
  a conflict all of whose members are cheap to leave out.
* **The trees part near the top.**  Both branch at the unit's node on 1485 (a conflict of 16 members) and then on 1640
  (15 members); after that they differ.
* **Decision: dropped.**  A rule could only choose, among the conflicts the table knows within the heaviest set, the
  one whose dearest member is dearest, at the cost of a bound for each member of each.  Nothing here shows that the
  cheap conflicts had dear alternatives; keeping the nogoods also rules more candidates out, which changes the prices
  as much as a choice would.

## Stage 2: today's hardest units, and the knot

Stage 2 asks stage 1's questions of today's hardest units, and tests a bound that sees the knot of a conflict
(2026-10-20).  The units are those of the suite of the run to 5000: 30 units of 3940, 4225, 4416 and 4548, built and
solved with the learning by its own entry, the hardest searches of the run to 5000.  In short:

* **The anatomy is the same, at higher prices.**  A path still pays its gap in 6-7 exclusions, but each costs more, and
  the cheap ones are rarer.
* **A bound that imposes the knot saves little.**  Its fall is about one exclusion at every gap, so it closes nodes that
  were about to close, and where the tree is it has nothing to impose.
* **Ladder branching is moot**, for the same reason.

Every search here completed with the optimum as its best.

### The anatomy at 4416

Three of 4416's biggest units were traced with `wall anatomy --learn --closure` from the known optimum, and taken apart
by `residual.py profile` and `misses` (every 8th computed conflict at 4416-83, every 2nd at 4416-148).  `wall anatomy`
needs a known optimum, which 4548 does not have, so 4416 stands in for it.  The old suite's 3300-14, with the learning,
is from stage 1:

| | 3300-14 | 4416-169 | 4416-83 | 4416-148 |
|---|---|---|---|---|
| the unit's gap | 998 | 1,307 | 1,407 | 1,684 |
| steps, default; with the learning | 1,248,609; 790,609 | 327,742; 226,756 | 1,048,736; 907,314 | 831,246; 420,508 |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 7 (5-9) | 6 (4-9) | 6 (4-8) |
| price of an exclusion, median | 123 | 176 | 156 | 176 |
| exclusions priced under 50 | 19.7% | 5.2% | 12.1% | 5.1% |
| of those, of conflicts of 2 members | 71% | 65% | 56% | 68% |
| gap at which a leaf closes, median | 91 | 181 | 168 | 196 |
| leaves^(1/exclusions) | 4.9 | 4.1 | 6.5 | 5.7 |
| a decade of subtree per ... points of gap | 241 | 301 | 335 | 473 |
| known conflicts given by image nogoods | 15.6% | 7.6% | 13.7% | 5.3% |
| computed conflicts: forgotten nogoods | 57.0% | 29.2% | 45.2% | 39.8% |
| the same conflict, another nogood | 26.0% | 48.3% | 48.1% | 34.2% |
| exchanges outside the closure | 7.9% | 9.1% | 2.5% | 14.6% |
| nogood size, median | 25 | 36 | 39 | 30 |

* **The same shape, dearer steps.**  A path pays its gap in 6-7 exclusions, as before, but at a median of 156-176
  points instead of 123, and only 5-12% of the exclusions are cheap, against 20%.  Most cheap ones are still of
  conflicts of two members.  Leaves close at twice the gap, and a decade of subtree takes 300-470 points of gap.
* **What is new.**  Nogoods are larger (a median of 30-39 members), and a third to a half of the conflicts the search
  computes are conflicts it met under another nogood.  By the follow-ups that costs peels, not tree.  The image nogoods
  give fewer of the known conflicts, 5-14%.
* **4548.**  The learning took 7.2% off 4548's ten units, against 25-30% for the other three games (the suite's own
  runs).  The start does not explain it.  No unit beat the incumbent, so each unit's search is the one it would make
  from an optimum equal to the incumbent, and the gaps (929-1,224) are those of 3940 and 4225.  The learning finds less
  to learn there: 4.3 conflicts learned a computed conflict, against 6.5 at 4416, and 3.2 conflicts computed a thousand
  steps, against 7.8.  So the difference is in the problem, not in the start.

### The knot in the bound

**The question.**  [The anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md) found that leaving a member out
costs the bound little because a twin steps in, and [completions near the
optimum](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md) found that the hardness is the bound's
error.  Does a bound that sees a conflict's whole *knot* close a real share of a node's gap?  The knot is the conflict
with the conflicts within its images under twin swaps and paired exchanges.

**The measure.**  At every 10th node check with a conflict and every 10th exclusion of a unit's default tree, the hook
keeps the node's knot: its conflict, and the minimal conflict at the node within each of the conflict's images that is
one.  The *narrow* knot takes the images of the closure under the cheap exchanges, as the learning does; the *wide*
knot also takes the images under every twin swap of a member, at any price.  After the search, a best-first search over
the members left out finds the heaviest set of the node's candidates that holds no conflict of the knot whole, with
the GF(2) bound.  Every completion of the node leaves out a member of each conflict, so that is a bound on them: the
bound *with the knot*.  At an exclusion, the member is left out from the start.  There, a bound with the knot below
the need would force the member, and the node's loop would end after the child that promotes it.  The hook also
records the steps after each check, and under each exclusion, to the end of the node's loop.  Their union over the
checks the knot closes, or the exclusions it forces, is a static estimate of what cutting by the knot saves.  The
prototype is `wall images --count --knots N [--wide]` (`knots.patch`); `residual.py knots` makes the table.

| | 4416-169 narrow | 4416-169 wide | 3940-104 narrow | 3940-104 wide |
|---|---|---|---|---|
| node checks kept | 6,565 | 6,565 | 2,654 | 2,654 |
| gap, p25/median/p75 | 103/160/243 | 103/160/243 | 50/90/152 | 50/90/152 |
| the knot's conflicts, median | 2 | 13 | 6 | 34 |
| fall with the conflict alone, median | 49 | 49 | 4 | 4 |
| fall with the knot, p25/median/p75 | 49/66/103 | 49/78/140 | 23/72/120 | 25/76/160 |
| fall with the knot, median: gaps under 100; 100-400; over 400 | 52; 72; 72 | 55; 87; 76 | 72; 72; 72 | 76; 76; 76 |
| checks the knot closes | 16.4% | 23.0% | 40.2% | 45.6% |
| steps left in their loops, median | 1 | 1 | 1 | 1 |
| steps after them (union) | 0.9% | 1.2% | 2.9% | 3.6% |
| exclusions kept | 10,656 | 10,656 | 3,034 | 3,034 |
| exclusions whose member the knot forces | 8.6% | 16.2% | 18.2% | 25.6% |
| steps under them, median | 2 | 1 | 2 | 2 |
| steps under them (union) | 0.6% | 1.0% | 1.5% | 2.0% |
| exclusions over 100 steps: count; median slack; median fall with the knot | 188; 529; 0 | 188; 529; 0 | 58; 328; 0 | 58; 328; 0 |

* **The knot's fall is about one exclusion, at every gap.**  Imposing the knot lowers a node's bound by a median of
  66-78 points, against 4-49 for its conflict alone, at gaps under 100 as at gaps over 400.  The best usually leaves out
  a single member.  That is 40-85% of the median gap of the nodes kept, most of them near the leaves.  But it is 5-20%
  of the gaps of 400 to 1,300 at the nodes that hold the tree.
* **What it closes was closing.**  The bound with the knot is below the need at 16-46% of the checks kept, and those
  nodes had a median of one step left in their loops: cutting them saves 0.9-3.6% of the steps.
* **Where the tree is, the knot has nothing to impose.**  The knot would force the member at 9-26% of the exclusions
  kept, with a median of 1-2 steps under them (0.6-2.0% of the steps).  Under the exclusions with more than 100 steps,
  the slack is 330-530 points and the knot's fall is 0 at the median: the heaviest set without the member holds none of
  the knot's conflicts.  At this scale what the bound puts in for a member left out is rarely a twin that makes an
  image of the conflict a conflict.  On the old suite's 3441-5459 the narrow knot does no better (0.2-0.3% of the
  steps).
* **Decision: not built.**  A bound carrying the knot would save a few percent of the steps by this estimate, and cost
  a dozen bound evaluations at each node (the measure's median is 13-16 a check, 4-5 an exclusion).

### Ladder branching

The anatomy's second hypothesis was a bound or branching that charges a payer's whole ladder at once: its composites
`m·p` over the primes `p`, which step in for each other one rung at a time.  The wide knot is that ladder for each
member of a conflict: every twin swap of a member, imposed at once.  It forces members only above subtrees of a step or
two, and finds nothing to impose under the big exclusions, so a branching that walks the ladder at once has nothing
there to gain.  It was not tested further.

## Files

* `residual.py`: `profile` and `misses`, the tables of this entry; its module comment has the usage.
* `runs.jsonl`: the lines of `wall anatomy` (the four traced searches) and of `wall images` (the tests of keeping).
* The traces, 125-190 MB a big unit, are not kept, because a one-thread solve of a unit is the same every time.  From
  `solver/frame`, after `cargo build -j 2 --profile test --example wall --features studies`:
  `target/debug/examples/wall anatomy --suite ../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl
  --known ../../results/strict.jsonl --ids 3300-14 [--learn --closure] --trace DIR 3300`, and the same with `--ids
  3441-5459` and `3441`; then `python3 -I residual.py profile DIR_DEFAULT DIR_LEARNING 3300-14 3441-5459` and
  `python3 -I residual.py misses DIR 3300-14 3441-5459`.  The tests of keeping are `wall images` with `--learn --closure`
  or `--count`, and `--tactic nogood_keep=hit`.
* The follow-ups: `residual.py windows` and `branched`; `witness.patch`, the witness's prototype; in `runs.jsonl`, the
  lines labelled `witness` and `base` (`wall images --learn --closure [--witness]`), the line of 3128-root (`wall
  anatomy --root --learn --closure --trace DIR 3128`, then `residual.py windows DIR 3128-root` and `misses`), and the
  lines labelled `test-default` and `test-keep` (`wall images --ids 3300-14 [--tactic nogood_keep=hit] --trace DIR
  3300`, then `residual.py branched DIR_DEFAULT DIR_KEEP 3300-14`).
* Stage 2: `knots.patch`, the knot's prototype, against commit 752071f; `residual.py knots`, and `profile`'s row of
  the cheap exclusions of conflicts of two members; in `runs.jsonl`, the lines labelled `stage 2` (`wall anatomy
  --suite SUITE --ids ID --learn --closure --trace DIR 4416`, SUITE the suite of the run to 5000) and those labelled
  `knots` and `knots-wide` (`wall images --suite SUITE --ids ID --count --knots 10 [--wide] --trace DIR GAME`, then
  `residual.py knots DIR/ID.knots.tsv STEPS ...`).

Commits: the commit of this entry (`wall anatomy --learn`, `--closure`, `--root`, the mirror, the nogood log, with
their tests).
