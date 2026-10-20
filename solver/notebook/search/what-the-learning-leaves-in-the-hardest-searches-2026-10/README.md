# What the learning leaves in the hardest searches (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-20.

**Question.**  Once the search learns its conflicts' images under the cheap exchanges, where do a hard unit's steps go,
and which conflicts does it still have to compute: images of conflicts it met, under exchanges the learning does not
make, or something else?  **Answer.**  The learning leaves the anatomy of a tree as it was and removes breadth: 5.2
alternatives an exclusion fall to 4.9 at 3300-14.  Of the conflicts it still computes there, 57% are nogoods it forgot
at a clearing of its table and 26% conflicts met before under another witness, and exchanges outside the closure explain
at most 8%, so a wider closure has little to catch.  None of the ideas that followed paid: learning the witness's
exchanges makes 22 units' trees 14.4% larger and costs 32% more CPU, and a bound that imposes a conflict's knot would
save under 4% of the steps.  The learning of closures stays closed, not adopted ([the
verdict](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted)).

The study follows [conflicts closed under cheap exchanges](../conflicts-closed-under-cheap-exchanges-2026-10/README.md),
whose learning took 20.6% off the steps of the tuning suite and only 4.5% off the CPU of its whole games.  Stage 1 built
the instruments and tried them on two units of the old [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md); follow-ups on that suite tested the ideas it
suggested; stage 2 asked the same questions of the hardest units of the run to 5000 and tested a bound that imposes a
conflict's knot.  Every search here completed with the optimum as its best.

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

## Method

**The instrument.**  `wall anatomy` takes `--learn [--closure]` and `--root` (examples/wall/anatomy.rs, with its tests),
and traces each step as before.  The hook keeps a mirror of the searcher's nogood table, with the source of each nogood,
own or image, so the trace says which source gave each known conflict (the mirror explained every known conflict on
every run here), and `ID.nogoods.tsv` lists every nogood the search learned.  `residual.py` reads them.  `profile` puts
two searches of a unit side by side.  `misses` takes each computed conflict and looks, among the nogoods learned before
it, for one that a few moves of members turn into a nogood that would have given the conflict at its node.  It names
each move by the factorizations: a twin swap (m·p for m·q), a kin move (m·u for m·v, u and v at most 16), a rescaled
payer (m·p for m′·p), or unrelated; a twin swap is in the learning's class when the learning would have proposed it at
the earlier nogood's node.

**The units.**  Stage 1 uses 3300-14, the old suite's biggest, and 3441-5459, its small contrast at the same gap ([the
anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md#what-was-run)).  Each was solved on one thread from the
known optimum with fresh tables, on the laptop under `nice -n 10`, with and without the learning, from the build of
`--profile test`; the trees are those of the earlier studies, step for step.  Stage 2 uses the suite of the run to 5000:
30 units of 3940, 4225, 4416 and 4548, built and solved with the learning by its own entry.

## Results

### Stage 1: where the steps go

The learning changes how many leaves a tree has, and little else:

| | 3300-14 default | learning | 3441-5459 default | learning |
|---|---|---|---|---|
| steps | 1,248,609 | 790,609 | 52,855 | 27,073 |
| leaves | 104,400 | 66,012 | 4,706 | 2,473 |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 7 (5-10) | 5 (4-7) | 5 (3-7) |
| price of an exclusion, median | 125 | 123 | 153 | 153 |
| exclusions priced under 50 | 19.9% | 19.7% | 3.7% | 6.2% |
| gap at which a leaf closes, median | 94 | 91 | 169 | 173 |
| alternatives an exclusion (leaves^(1/exclusions)) | 5.2 | 4.9 | 5.4 | 4.8 |
| a decade of subtree per ... points of gap | 238 | 241 | 245 | 262 |
| conflicts computed, of the checks with one | 0.8% | 0.5% | 0.8% | 0.7% |
| known from image nogoods (alone or with own) | 0 | 15.6% | 0 | 14.8% |

* **What did not change:** a path pays the gap in 7 (or 5) exclusions at a median price of 123-153, a fifth of 3300-14's
  exclusions are still cheap (under 50), and the subtree of a node still grows a decade for every 240-260 points of its
  gap.  The learning does not make the bound pay more for leaving a member out.
* **What changed:** the alternatives for each exclusion, 5.2 to 4.9 and 5.4 to 4.8; with 7 exclusions on a path that is
  the whole gain (0.94^7 ≈ 0.63).  The image nogoods give 15% of the known conflicts.
* **What a check computes:** under 1% of the checks that find a conflict, with the learning or without; the rest find it
  in the table.  The learning cuts the computed conflicts by 58% at 3300-14, more than the steps (37%).

### Stage 1: what the learning misses

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
* **The same conflict with another witness.**  A quarter to a third are a conflict the search had computed before, whose
  nogood no longer fits: its witness holds promotions the node has not made, and the node has others that serve.  Of the
  662 such conflicts at 3300-14 with the learning, the closest earlier nogood holds one member that the node has neither
  promoted nor in its heaviest set in 57%, two in 14%, and none in 26% (that nogood would have fitted, and was
  forgotten); at 3441-5459, one in 34 of 40.
* **Exchanges outside the learning's class** explain 8% at 3300-14 and 19% at 3441-5459 together, none of them more than
  8%: two twin swaps of one prime pair on two payers (2.6-6.1%), kin moves, twin swaps up, dear twin swaps, and twin
  swaps of a member that the earlier node had promoted.  Proposing every twin swap of a conflict's members at its node
  would cost 44-48 sets a computed conflict at 3300-14, against the closure's 15.
* **The calibration:** without the learning, 0.7% (3300-14) and 3.4% (3441-5459) of the computed conflicts are first
  explained by a cheap twin swap that the learning proposes; with it, 0.1% and 0.9%.  So the instrument sees what the
  learning removes.

### Stage 1: keeping the used nogoods at a clearing

The largest group is the forgotten nogoods, so the cheapest test keeps them: the tactic `nogood_keep=hit` keeps, at a
clearing, the nogoods used since the last one.  [Keeping and
forgetting](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md) found that it makes ten whole
games 28-31% larger on one thread.  Does the learning, whose classes a clearing throws away, change that?  No: keeping
moves each unit by the same factor with the learning or without it, and the factor swings from a third to one and a
half.

| unit | default | `nogood_keep=hit` | change | learning | learning, `nogood_keep=hit` | change |
|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 464,165 | −62.8% | 790,609 | 453,073 | −42.7% |
| 3441-1262 | 1,099,410 | 1,602,571 | +45.8% | 648,846 | 950,397 | +46.5% |

The cells are steps; the CPU seconds follow them (308, 118, 204 and 117 at 3300-14; 342, 506, 220 and 326 at 3441-1262).
A unit's tree is a chaotic function of the nogoods it holds, as [the tuning
campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) found, so two units cannot say whether
keeping pays on average; the earlier entry says it does not.

### Follow-up: the witness's exchanges

**The test.**  The search learns each computed conflict's nogood again with one member of its witness exchanged: for
each member `w` of the witness (the nogood's members outside the conflict), and each root candidate `y` outside the
nogood that touches a frame `w` touches and is `w`'s twin (`w = m·p`, `y = m·q`) or kin (`w = m·u`, `y = m·v`, `u` and
`v` at most 16), the searcher peels the nogood with `y` for `w` from the root.  A minimal conflict it finds there is
learned as a nogood; a conflict of the root holds at every node, so the search stays exact.  That is 519 sets peeled and
29 nogoods learned a computed conflict, on top of `--learn --closure`.  The prototype,
`wall images --learn --closure --witness` with a hook in the searcher behind the feature `studies`, is not in the code:
[witness.patch](witness.patch) adds it to commit 60976f3.

**The result,** on 3300-14 and the 21 units of the suite whose closure took under 50 CPU seconds (the closure alone is
the closure's lines of [cheap exchanges](../conflicts-closed-under-cheap-exchanges-2026-10/README.md),
`suite-runs.jsonl`, which a run of 3441-5459 here repeated step for step):

| | the closure | the closure and the witness |
|---|---|---|
| steps, the 22 units | 2,812,162 | 3,217,865 (+14.4%) |
| steps, a unit's ratio: geometric mean; range | | 1.034; 0.44 (3425-13) to 2.97 (3441-571) |
| units larger | | 12 of 22 |
| conflicts computed; a thousand steps | 10,167; 3.6 | 8,819; 2.7 |
| CPU seconds | 733 | 971 (+32%) |

The exchanges do their job, and the tree does not care.  A quarter fewer conflicts are computed for each step, and at
3441-5459 the nogoods rule 115,415 candidates out of children against 8,112.  But a conflict the search no longer
computes is mostly the one the table gives (stage 1's groups), and a node branches on a conflict alike whether it found
it or computed it.  What moves the tree is the candidates the nogoods rule out, and that moves it both ways, as keeping
the nogoods did.  The exchanges also cost CPU: the tables hold 2,300-5,400 nogoods at a clearing, and a lookup scans the
whole table.  On the two units run alone on the same day, the witness took 11% more CPU at 3300-14 (204 to 227 seconds)
and 35% more at 3441-5459.

### Follow-up: whole games and units

Stage 1 guessed that a whole game, one long search, loses more to the clearings than a unit.  It does not: the share
grows with the length of the search.  The whole game 3128-root was traced with `wall anatomy --root --learn --closure`
(377,696 steps), and `residual.py windows` follows each search period by period:

* **The forgotten share is the length of the search.**  None of a period's computed conflicts are forgotten in the first
  period, a third in the second (35% at 3128-root, 30% at 3300-14), and most after ten (70-90% in each period after
  500,000 steps at 3300-14).  Over its first 400,000 steps 3128-root forgets 42.8% and 3300-14 41.0%; 3300-14 reaches
  57.0% over its whole search, the longer one.  A searcher of a big run on many threads takes millions of steps, so most
  of the conflicts it computes will be nogoods it forgot; by the witness's test, that costs peels, not tree.
* **The image share is lower in the whole game.**  Its image nogoods give 12.4% of its known conflicts, against 16.5%
  for 3300-14 over the same steps and 14.8% for 3441-5459.  `misses` puts 18.1% of 3128-root's computed conflicts in
  none of the groups of moves, against 9.1% at 3300-14; the same conflict with another nogood is 27.2% (26.0%).

### Follow-up: the conflicts branched on

At 3300-14, keeping the used nogoods takes the tree from 1,248,609 steps to 464,165.  The guess was that what the table
holds changes which conflict a node branches on, and that a better conflict (fewer members, dearer exclusions) is the
lever.  `wall images`, which leaves the tree as it is, gives a row for each exclusion with the size of the conflict and
the price; `residual.py branched` compares the two searches (345,237 exclusions by default, 131,989 with the keeping):

* **Not smaller conflicts:** the conflicts are the same size (median 3, mean 3.07 and 3.18), and the table already gives
  a node the smallest conflict it knows within the heaviest set.
* **Fewer cheap ones:** exclusions priced under 50 are 14.0% against 19.9%, most of them conflicts of two members (79.6%
  against 73.3%), and paired exchanges are 4.7% of the exclusions against 7.9%.  The median price is the same (126
  against 125).  Strong branching takes a conflict's dearest member, so a cheap exclusion is a conflict all of whose
  members are cheap to leave out.
* **The trees part near the top:** both branch at the unit's node on 1485 (a conflict of 16 members) and then on 1640
  (15 members); after that they differ.

### Stage 2: the anatomy at 4416

Three of 4416's biggest units were traced with `wall anatomy --learn --closure` from the known optimum, and taken apart
by `residual.py profile` and `misses` (every 8th computed conflict at 4416-83, every 2nd at 4416-148).  `wall anatomy`
needs a known optimum, which 4548 does not have, so 4416 stands in for it.  The old suite's 3300-14, with the learning,
is from stage 1:

| | 3300-14 | 4416-169 | 4416-83 | 4416-148 |
|---|---|---|---|---|
| the unit's gap | 998 | 1,307 | 1,407 | 1,684 |
| exclusions on a path to a leaf, median (p10-p90) | 7 (5-10) | 7 (5-9) | 6 (4-9) | 6 (4-8) |
| price of an exclusion, median | 123 | 176 | 156 | 176 |
| exclusions priced under 50 | 19.7% | 5.2% | 12.1% | 5.1% |
| a decade of subtree per ... points of gap | 241 | 301 | 335 | 473 |
| known conflicts given by image nogoods | 15.6% | 7.6% | 13.7% | 5.3% |
| computed: forgotten nogoods; another nogood | 57.0%; 26.0% | 29.2%; 48.3% | 45.2%; 48.1% | 39.8%; 34.2% |
| nogood size, median | 25 | 36 | 39 | 30 |

* **The same shape, dearer steps:** a path pays its gap in 6-7 exclusions, as before, but at a median of 156-176 points
  instead of 123, and only 5-12% of the exclusions are cheap, against 20% (most of them of conflicts of two members,
  56-68%).  Leaves close at twice the gap, and a decade of subtree takes 300-470 points of gap.
* **What differs:** nogoods are larger (a median of 30-39 members), and a third to a half of the conflicts the search
  computes are conflicts it met under another nogood (34-48%).  By the follow-ups that costs peels, not tree.  The image
  nogoods give fewer of the known conflicts, 5-14%.
* **4548:** the learning took 7.2% off 4548's ten units, against 25-30% for the other three games (the suite's own
  runs).  The start does not explain it: no unit beat the incumbent, so each unit's search is the one it would make from
  an optimum equal to the incumbent, and the gaps (929-1,224) are those of 3940 and 4225.  The learning finds less to
  learn there: 4.3 conflicts learned a computed conflict, against 6.5 at 4416, and 3.2 conflicts computed a thousand
  steps, against 7.8.  So the difference is in the problem, not in the start.

### The knot in the bound

**The question.**  [The anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md) found that leaving a member out
costs the bound little because a twin steps in, and [completions near the
optimum](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md) found that the hardness is the bound's
error.  Does a bound that sees a conflict's whole *knot* close a real share of a node's gap?  The knot is the conflict
with the conflicts within its images under twin swaps and paired exchanges.

**The measure.**  At every 10th node check with a conflict and every 10th exclusion of a unit's default tree, the hook
keeps the node's knot: its conflict, and the minimal conflict at the node within each of the conflict's images that is
one.  The *narrow* knot takes the images of the closure under the cheap exchanges, as the learning does; the *wide* knot
also takes the images under every twin swap of a member, at any price.  After the search, a best-first search over the
members left out finds the heaviest set of the node's candidates that holds no conflict of the knot whole, with the
GF(2) bound.  Every completion of the node leaves out a member of each conflict, so that is a bound on them: the bound
*with the knot*.  At an exclusion the member is left out from the start, and a bound with the knot below the need would
force it.  The hook also records the steps after each check, and under each exclusion, to the end of the node's loop;
their union over the checks the knot closes, or the exclusions it forces, is a static estimate of what cutting by the
knot saves.  The prototype is `wall images --count --knots N [--wide]` ([knots.patch](knots.patch)); `residual.py knots`
makes the table, here for 4416-169 and 3940-104.

| | 4416-169 narrow | 4416-169 wide | 3940-104 narrow | 3940-104 wide |
|---|---|---|---|---|
| node checks kept; median gap | 6,565; 160 | 6,565; 160 | 2,654; 90 | 2,654; 90 |
| the knot's conflicts, median | 2 | 13 | 6 | 34 |
| fall with the conflict alone, median | 49 | 49 | 4 | 4 |
| fall with the knot, median | 66 | 78 | 72 | 76 |
| checks the knot closes | 16.4% | 23.0% | 40.2% | 45.6% |
| steps after them (union) | 0.9% | 1.2% | 2.9% | 3.6% |
| exclusions kept | 10,656 | 10,656 | 3,034 | 3,034 |
| exclusions whose member the knot forces | 8.6% | 16.2% | 18.2% | 25.6% |
| steps under them (union) | 0.6% | 1.0% | 1.5% | 2.0% |
| exclusions over 100 steps: count; median slack; median fall | 188; 529; 0 | 188; 529; 0 | 58; 328; 0 | 58; 328; 0 |

* **The knot's fall is about one exclusion, at every gap:** a median of 66-78 points, against 4-49 for the conflict
  alone, at gaps under 100 as at gaps over 400 (a median fall of 52-76 under 100, 72-87 between 100 and 400, 72-76 over
  400).  That is 40-85% of the median gap of the nodes kept, most of them near the leaves, but only 5-20% of the gaps of
  400 to 1,300 at the nodes that hold the tree.
* **What it closes was closing:** the bound with the knot is below the need at 16-46% of the checks kept, and those
  nodes had a median of one step left in their loops, so cutting them saves 0.9-3.6% of the steps.
* **Where the tree is, the knot has nothing to impose:** it would force the member at 9-26% of the exclusions kept, with
  a median of 1-2 steps under them (0.6-2.0% of the steps).  Under the exclusions with more than 100 steps the slack is
  330-530 points and the knot's fall is 0 at the median: the heaviest set without the member holds none of the knot's
  conflicts, so at this scale what the bound puts in for a member left out is rarely a twin that makes an image of the
  conflict a conflict.  On the old suite's 3441-5459 the narrow knot does no better (0.2-0.3% of the steps).

### Ladder branching

The anatomy's second hypothesis was a bound or branching that charges a payer's whole ladder at once: its composites
`m·p` over the primes `p`, which step in for each other one rung at a time.  The wide knot is that ladder for each
member of a conflict: every twin swap of a member, imposed at once.  It forces members only above subtrees of a step or
two, and finds nothing to impose under the big exclusions, so a branching that walks the ladder at once has nothing
there to gain.  It was not tested further.

## Decision

**Nothing here is kept, and the learning stays closed, not adopted.**

* **A wider closure:** the exchanges the closure lacks explain at most 8% of the computed conflicts at 3300-14.
* **Learning the witness's exchanges:** dropped.  It cuts the conflicts computed, as it was meant to, but the trees grow
  14.4% on 22 units (12 larger) and the CPU 32%.  The aim of cutting the computed conflicts for their own sake goes with
  it: they are under 1% of the checks, and finding one instead of computing it saves a peel, not tree.
* **Keeping the used nogoods** (`nogood_keep=hit`): not an answer.  It moves the trees by the same factor with the
  learning as without it (0.37 and 1.46 at 3300-14 and 3441-1262), and at 3300-14 it branches on conflicts of the same
  size and fewer cheap ones (14.0% under 50 against 19.9%), so no rule for choosing a conflict follows: it would choose
  the conflict whose dearest member is dearest, at the cost of a bound for each member of each, and keeping the nogoods
  also rules more candidates out, which changes the prices as much as a choice would.  [The headroom of branching in
  hindsight](../the-headroom-of-branching-in-hindsight-2026-10/README.md) later found that changing only the period at
  which the table forgets moves strong branching's own trees by −26% to +17%.
* **A bound that carries the knot:** not built.  By the estimate it would save 0.6-3.6% of the steps (under 4%), and it
  would cost a dozen bound evaluations at each node (the measure's median is 13-16 a check, 4-5 an exclusion).  Ladder
  branching is moot for the same reason.
* **A whole game forgets as much as a unit,** step for step, so the clearings do not explain why whole games gain less
  than units.

What would reopen the question is a change that makes a tree depend less on the nogoods its table holds, or a closure
that catches more than the 8% of computed conflicts that exchanges outside it explain.

## Files

* [residual.py](residual.py): `profile` and `misses`, the tables of stage 1; `windows` and `branched` of the follow-ups;
  `knots` of stage 2, and `profile`'s row of the cheap exclusions of conflicts of two members.  Its module comment has
  the usage.
* [runs.jsonl](runs.jsonl): the lines of `wall anatomy` (the four traced searches of stage 1 and those labelled
  `stage 2`) and of `wall images` (the tests of keeping; the lines labelled `witness` and `base`, `test-default` and
  `test-keep`, `knots` and `knots-wide`).
* [witness.patch](witness.patch): the witness's prototype, against commit 60976f3.
* [knots.patch](knots.patch): the knot's prototype, against commit 752071f.

The traces, 125-190 MB a big unit, are not kept, because a one-thread solve of a unit is the same every time.  The
commands, from `solver/frame`, after `cargo build -j 2 --profile test --example wall --features studies`:

* **Stage 1:**

  ```text
  target/debug/examples/wall anatomy \
      --suite ../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl \
      --known ../../results/strict.jsonl --ids 3300-14 [--learn --closure] --trace DIR 3300
  ```

  and the same with `--ids 3441-5459` and `3441`; then
  `python3 -I residual.py profile DIR_DEFAULT DIR_LEARNING 3300-14 3441-5459` and
  `python3 -I residual.py misses DIR 3300-14 3441-5459`.  The tests of keeping are `wall images` with
  `--learn --closure` or `--count`, and `--tactic nogood_keep=hit`.
* **The witness:** `wall images --learn --closure [--witness]` (the lines labelled `witness` and `base`).
* **A whole game:** `wall anatomy --root --learn --closure --trace DIR 3128`, then `residual.py windows DIR 3128-root`
  and `misses`.
* **The branched conflicts:** `wall images --ids 3300-14 [--tactic nogood_keep=hit] --trace DIR 3300` (the lines
  labelled `test-default` and `test-keep`), then `residual.py branched DIR_DEFAULT DIR_KEEP 3300-14`.
* **Stage 2:** `wall anatomy --suite SUITE --ids ID --learn --closure --trace DIR 4416`, SUITE the suite of the run to
  5000; and, for the knot, `wall images --suite SUITE --ids ID --count --knots 10 [--wide] --trace DIR GAME` (the lines
  labelled `knots` and `knots-wide`), then `residual.py knots DIR/ID.knots.tsv STEPS ...`.

## Commits

* 60976f3: the base of the witness prototype (`witness.patch`), a merge of 2026-10-20.
* 752071f: the base of the knot prototype (`knots.patch`), a merge of 2026-10-20.
