# Conflicts closed under cheap exchanges (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-19.  Concluded 2026-10-20.

**Question.**  When the search goes on without a member of a conflict, its bound swaps in the member's twin at a small
price.  Is the conflict, with the same swap made, a conflict too, and does learning those images make the hard trees
smaller?  **Answer.**  Yes on units, no on whole games.  Learning each computed conflict's closure under the cheap
exchanges took 37-49% off the steps and 34-46% off the CPU of three hard units, soundly.  Over the 46 problems of the
tuning suite it took 15.9% off the CPU, but only 4.5% over its 6 whole games, three of them dearer, so the learning is
closed, not adopted.

The idea is the first of
[near completions](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md#ideas), tested on the units
of [the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md).

## Words

* A *conflict* of a node is a set of its candidates that cannot be promoted together; *exclusion*, *price*, *home* and
  *unit* are the [anatomy's](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words).  An exclusion is *cheap* when
  its price is under 50 points.
* Two numbers are *twins* when they share a maximal factor: 1292 = 68·19 and 1156 = 68·17.  A *twin swap* takes x out of
  a set and puts its twin y in.  A *paired exchange* is two twin swaps of one pair of primes in opposite directions:
  1608 = 24·67 and 1220 = 20·61 out, 1464 = 24·61 and 1340 = 20·67 in, at a price of (67 − 61)(24 − 20) = 24
  ([near completions](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md#words)).
* When a node goes on without the member x of its conflict K, the bound's heaviest set at the node's next step has
  lost some numbers, x among them, and gained others: the *substitution*.  When it is a twin swap or a paired
  exchange, the *image* of K is K with each member that the substitution takes out replaced by the twin it puts in.
* The *closure* of a conflict under the cheap exchanges at a node is the conflict's images, the images of those that
  are conflicts, and so on.  A *class* is a conflict the search computed, with the conflicts it learned within its
  images.

## Method

**The instrument** is `wall images` ([examples/wall/images.rs](../../../frame/examples/wall/images.rs), with tests), on
three hooks of the studies build ([src/search/hooks.rs](../../../frame/src/search/hooks.rs)): `StepHook::excluding` is
told the conflict the node branches on as well, `StepHook::propose` is asked, whenever a bound check computes and
learns a conflict, for more sets to learn, and `StepHook::learned` is told what the searcher learned from them.  The
searcher learns, of each set proposed, a minimal conflict that a peel finds within it (`FrameGraph::conflict_using`), as
it learned the conflict itself: known for the node and its subtree, and as a nogood, unless its table already knows a
conflict within it.  A default build compiles none of this.  The modes of `wall images`:

* the test (the default): at each exclusion, whether the image of the node's conflict under the substitution is a
  conflict of the node, with a row an exclusion;
* `--learn`: the hook proposes the conflict's images under the cheap twin swaps and paired exchanges at the node.  A
  member a = m·p of K gives way to a candidate a' = m·q outside K; the swap is proposed when it costs 0 < a − a' < 50,
  or when a paired exchange makes it that cheap.  `--closure` proposes the images of those images that are conflicts
  too, up to 64 sets a conflict.  With `--trace` it writes the classes;
* `--bare`: the search with no hook, for its time; `--count`: the hook proposing nothing, so that the default search's
  `steps`, `tree_nodes` and `leaves` are counted (on seven units it takes the same CPU time as `--bare`, within 0.2%);
* `--root` makes the unit the whole game, as `tune solve` solves `GAME-root`.

**The first trial** solved three units of the [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md): 3300-14 and 3441-1262, the big ones, and
3441-5459, of about the same gap and a twentieth of the size.  Each ran on one thread from the known optimum, with
fresh tables, on the project's M1 Max laptop on 2026-10-19, from the build of `--profile test`, in about 56 CPU minutes
in all.

**The suite trial** solved each of the suite's 46 problems, 40 units and 6 whole games, twice the same way, with
`--count` and with `--learn --closure`, two runs at a time under `nice -n 10`, in 9,704 CPU seconds in all.  Steps and
leaves are exact and the same every time; the CPU seconds carry the laptop's noise.  The default search's steps are the
suite's `default.nodes` on all 46.  The gap of a whole game, which the suite lacks, is the root gap of `wall landscape
--known results/strict.jsonl N` less one; the gaps run from 746 to 1,001.

## Results

The learning is sound by construction.  The hook only proposes sets, and the searcher learns, of each, a subset that a
peel shows cannot be promoted from the node's state: a conflict of the node and of every node below it, whose nogood is
a nogood of the root, as `FrameGraph::nogood` makes for the search's own conflicts.  The argument of the search's
exactness covers it ([DESIGN.md, "Forced members and
nogoods"](../../../docs/design/search.md#forced-members-and-nogoods)).  The tests check that the test's hook leaves the
tree as it is (game 1215), that each image's verdict agrees with promoting it, and that the learning finds the optimum
of games 800, 1000 and 1215 from a floor 60 below it, with the closure and without.  Every search of both trials found
the optimum and ended with the same best and conclusion as the default search's.

### 1. The images are conflicts

Of the exclusions, 30-37% have a twin swap or a paired exchange for their substitution, and 44-59% of their images
are conflicts.  The cheap ones' images are conflicts almost always (`analysis.txt`, "By substitution"):

| | 3300-14 | 3441-1262 | 3441-5459 |
|---|---|---|---|
| exclusions | 345,237 | 342,969 | 18,075 |
| twin swaps; the image a conflict | 101,670; 52.8% | 61,911; 34.1% | 5,052; 39.0% |
| paired exchanges; the image a conflict | 27,285; 80.8% | 41,636; 95.3% | 412; 100% |
| cheap twin swaps and paired exchanges; the image a conflict | 35,592; 77.4% | 47,470; 95.1% | 429; 100% |

* **The cheaper the exchange, the likelier its image is a conflict**: at 3441-1262 the share is 95-96% under a price of
  25, 90% at 25-49, 61% at 50-99 and 18-26% above.  The paired exchanges of close primes dominate (67 ↔ 71 on 24 and
  20, 98-100%), and the expensive twin swaps of small primes do not (17 ↔ 19 gives 14-60%, 11 ↔ 13 8-22%).
* **Small conflicts carry it**: images of conflicts of two members are conflicts 60-83% of the time, of six or more
  26-33%.
* **The search already learns many images, late**: where the image is a conflict, the node's next conflict is the
  image itself, found in the nogood table, 64% of the times at 3441-1262 (22% at 3300-14, 9% at 3441-5459), only after
  the bound has walked into it, and the search forgets it with the rest of its table every 50,000 steps.
* **The steps under the exclusions do not bound the saving**: the steps under the cheap exclusions whose image is a
  conflict are 51%, 27% and 9% of the three units' steps, and the learning saves more, because the images act wherever
  their conflicts are met.

### 2. Learning the images on the three units

The learning shrinks every tree, and the closure shrinks it much more (`analysis.txt`, "Learning the images"):

| unit | steps alone | images | closure | CPU s alone | images | closure |
|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 1,020,828 (−18.2%) | 790,609 (−36.7%) | 284 | 232 (−18.3%) | 189 (−33.6%) |
| 3441-1262 | 1,099,410 | 744,669 (−32.3%) | 648,846 (−41.0%) | 352 | 256 (−27.2%) | 222 (−37.0%) |
| 3441-5459 | 52,855 | 48,598 (−8.1%) | 27,073 (−48.8%) | 15.3 | 14.4 (−5.9%) | 8.3 (−45.6%) |

The CPU seconds include what the learning costs, 5-7% a step with the closure.  The closure proposes 3.0 to 4.7 times
as many sets as the first images, learns 2.1 to 3.5 times as many conflicts, and needs fewer conflicts computed, since
the images supply some that the search would otherwise compute.

**Why it helps.**  A conflict K that the search knows rules its last member a out of a child whose promotions hold the
rest of K; the bound then takes a's twin a' in its place, at the same small price as at an exclusion.  If K with a'
for a is a conflict as well and is known, it rules a' out at the same time, and the bound has to pay for both.  The
nogoods of the images rule candidates out of children three to five times as often as they give the conflict a node
branches on.

**No few knots carry the pruning** (`analysis.txt`, "Classes"; `classes/`).  With the closure the two big units compute
1,225 and 1,081 classes, 746 and 790 of them with a conflict learned in an image; 545 and 363 are never used.  The class
used most has 3.1% and 4.0% of the uses, and half the uses take 42 and 38 classes; only the small unit, with 115
classes, has three classes with half.  A precomputed exact value for one knot would reach a few percent of the uses at
most, and to matter it would have to cover dozens of classes a unit, which the learning already finds as the search
meets them.

### The whole tuning suite

**The gain holds across the suite, at less than half the size.**  Over the 46 problems the learning takes 20.6% off the
steps and 15.9% off the CPU seconds in total (720 of 4,523 s), against 37-49% and 34-46% on the three units of the first
trial.  The median problem gains 14.0% of the steps and 13.6% of the CPU, 33 of the 46 get faster, and on every one the
best and the conclusion are the same.

| | problems | steps: total | median | CPU: total | median |
|---|---|---|---|---|---|
| all | 46 | −20.6% | −14.0% | −15.9% | −13.6% |
| units | 40 | −22.6% | −16.6% | −17.7% | −14.2% |
| whole games | 6 | −6.8% | −10.7% | −4.5% | −1.2% |
| steps under 100,000 | 12 | −8.2% | −15.5% | −6.4% | −13.6% |
| steps 100,000-299,999 | 14 | −10.4% | −9.1% | −5.9% | −4.1% |
| steps 300,000-599,999 | 11 | −21.7% | −26.2% | −17.7% | −22.2% |
| steps 600,000 and over | 9 | −24.4% | −19.9% | −19.2% | −15.8% |

The groups of steps are the default search's.  The leaves fall by 21.3% in total.

* **The first trial's units were among the best**: they rank 4th, 7th and 8th of 46 by CPU; the other 43 problems gain
  17.4% of the steps and 12.8% of the CPU.
* **The gain grows with the tree**, correlating with the size of the default tree at −0.25: eight of the nine units of
  600,000 steps or more gain 10-47% of the CPU, and the seven of 300,000 to 600,000 steps gain 14-41%.
* **The gap does not predict the gain**: the three bands of gap (under 850, 850-949, 950 and over) gain 11.2%, 15.4% and
  17.9% of the CPU in total, but the medians do not rise, and the change in steps correlates with the gap at +0.03.
* **By game**: the units of 3441 gain most (29.3% of the steps, 24.6% of the CPU), then 3425 (25.0% and 20.0%) and 3300
  (24.3% and 19.5%), and those of 3440 least (8.9% and 6.0%; three of its ten cost more CPU).
* **A step costs 5.8% more** (0.280 ms to 0.297 ms; 6.2% on the units, 2.5% on the whole games), as in the first trial
  (5-7%), so the CPU falls by less than the steps.
* **Whole games gain least**: 6.8% of the steps and 4.5% of the CPU.  3272 (3.0% more CPU), 3129 (3.9%) and 3213 (10.4%)
  cost more, and 3128 and 3275 gain 14% and 9%; the units of the same size gain more (14-41%).  What makes a whole
  game different is not measured.

### Where it costs more

The learning costs more CPU than the default search on 13 of the 46 problems, and takes more steps on nine.  The table
gives the learning and, for 12 of them, the images alone:

| problem | default steps | learning: steps | CPU | images alone: steps | CPU |
|---|---|---|---|---|---|
| 3425-147 | 33,333 | +2.5% | +4.2% | +6.3% | +6.5% |
| 3441-2994 | 36,500 | +127.7% | +122.3% | +94.1% | +82.4% |
| 3425-150 | 48,615 | +70.0% | +67.1% | −7.1% | −10.2% |
| 3440-39 | 71,515 | +13.0% | +15.0% | −2.5% | −0.8% |
| 3300-39 | 84,880 | +3.0% | +1.5% | −20.3% | −25.1% |
| 3425-59 | 160,283 | +17.6% | +26.3% | +38.7% | +42.2% |
| 3440-89 | 166,262 | −2.2% | +0.7% | −0.2% | −2.3% |
| 3425-13 | 167,978 | +4.7% | +6.6% | +3.8% | +5.8% |
| 3300-98 | 203,516 | −5.9% | +0.2% | +10.3% | +8.9% |
| 3272-root | 277,366 | −0.5% | +3.0% | +0.8% | +4.8% |
| 3129-root | 304,091 | −9.7% | +3.9% | −7.7% | −2.0% |
| 3213-root | 450,606 | +4.1% | +10.4% | +8.1% | +16.9% |
| 3440-12 | 870,739 | +5.5% | +10.7% | - | - |

* **Small trees move by tens of percent under any change of the nogoods.**  Nine of the 13 are units of fewer than
  300,000 steps, five of them under 100,000.  The default search on the twelve units under 100,000 steps with the
  tactic `nogood_period` at 20,000 and at 30,000 instead of 50,000, settings that moved the mean steps of the [tuning
  campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) by 5% or less, moved from 55% fewer steps
  to 226% more (3425-147 at 20,000), and the learning from 63% fewer to 128% more, so the losses of 3441-2994 (128%
  more) and 3425-150 (70% more) lie inside the first range, and only totals over many units say what the learning does.
* **The images alone do not repair the losses.**  They cost more CPU on seven of the 12, and take 10% and 25% off
  3425-150 and 3300-39, where the closure costs 67% and 1.5% more: neither form is better, so the losses are not the
  closure's.
* **A tree that shrinks little does not pay for the dearer step.**  On 3129-root the tree falls 9.7% in steps and the
  CPU rises 3.9%, because a step costs 15% more (0.180 ms to 0.207 ms; there the learning proposes 19.5 sets for each
  conflict computed, against 11.6 over the suite).  3300-98, 3272-root and 3440-89 save 5.9%, 0.5% and 2.2% of the
  steps and end with 0.2%, 3.0% and 0.7% more CPU.
* **One large unit loses without an explanation.**  3440-12 (gap 991) takes 5.5% more steps and 10.7% more CPU, though
  27.5% of its exclusions have a twin swap or a paired exchange and 51.8% of their images are conflicts, against 37.4%
  and 58.7% at 3300-14, 30.2% and 58.7% at 3441-1262 and 30.2% and 43.6% at 3441-5459.

`suite.py table suite-runs.jsonl` prints every problem; the best are 3440-100 (63.2% fewer steps, 59.7% less CPU),
3441-4840 and 3425-131.

## Decision

The instrument and the learning stay in the studies build (`--features studies`), and the production search is
unchanged.  After the first trial the entry planned three steps: the whole tuning suite; then whole games of the chain
from their start incumbents on many threads, where each searcher has its own table; then a tactic in the searcher, off
until measured, with the price line (50) and the cap on a closure (64) as its knobs.  The suite gave less than the
units, and its whole games least, so the verdict below withdraws the second and third steps.

## Later (2026-10-20): closed, not adopted

**Verdict.**  The learning is closed, not adopted.  It stays in the studies build as a record, and the searcher does not
get it.  The two steps that the decisions above plan, a test on whole games on many threads on EC2 and then a tactic in
the searcher, are withdrawn.  Units gain from the learning, and whole games, where the chain's cost lies, do not gain
reliably.

**The evidence** is this entry and two later ones:

* **The tuning suite** ([the whole tuning suite](#the-whole-tuning-suite), above).  The learning takes
  20.6% off the steps and 15.9% off the CPU of the 46 problems, but only 4.5% off the CPU of the six whole games, and
  three of those cost more.
* **The hardest searches of the run to 5000** ([the hardest searches of the run to
  5000](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md)).  On 30 units of 3940, 4225, 4416 and 4548 the
  learning takes 20.0% off the steps, and 7.2% at 4548.  On whole games it goes both ways: at 4176 it takes 13.4% off
  the CPU, at 4050 it puts 30.9% on, and at 4005 we stopped it at 2.2 times the default search's CPU, without a
  result.
* **Every attempt to push it further failed** ([what the learning leaves in the hardest
  searches](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md)):
  * A wider closure, with the exchanges the closure lacks, could catch at most 8% of the conflicts the search still
    computes at 3300-14.
  * Learning each computed conflict's nogood under other witnesses made the trees 14% larger on 22 units and cost 32%
    more CPU.
  * Keeping the used nogoods at a clearing moved the trees by a factor from a third to one and a half, the same with
    the learning as without it.
  * A bound that imposes the whole knot of a conflict would save 0.6-3.6% of the steps by the entry's estimate, so we
    did not build it, and ladder branching has nothing to impose.
  * At the scale of the run to 5000 the learning has less to work on: at 4416 an exclusion costs a median of 156-176
    points against 123 at 3300-14, and 5-12% of the exclusions are cheap, against 20%.

**Why the early results misled.**  A unit solved alone overstates the gain, and a tree's size swings chaotically under
any change to the nogoods the search holds, so a unit's gain does not predict a whole game's cost.

* The first trial's units were among the suite's best: they rank 4th, 7th and 8th of 46 by the share of CPU gained.
  The suite's units gained 17.7% of the CPU and its whole games 4.5%.
* A unit's gain does not predict its game's.  The three units of 4005 took 4.2% less CPU, and the game, stopped at
  2.2 times the default search's CPU, had not finished.
* The learning changes which nogoods the search holds, and a small tree moves by tens of percent when anything does.
  13 of the suite's 46 problems cost more CPU, and the default search on twelve units under 100,000 steps moved from 55%
  fewer steps to 226% more under a change of the nogood period alone ([where it costs
  more](#where-it-costs-more)).

**What would reopen it** is a consistent gain in CPU on whole games of the scale of the chain's end, not a gain on
units.  Those games take 3.1 to 6.2 million steps on one thread, as 4005, 4050 and 4176 do.  A new form of the learning
is tested on them first.


## Files

* `runs.jsonl`: the lines of `wall images` of the first trial, whose fields the module comment of
  [examples/wall/images.rs](../../../frame/examples/wall/images.rs) gives: for each unit the test (`mode` `check`), the
  search alone (`bare`), the first images (`learn`) and the closure (`closure`), and the closure again with `--trace`
  for the classes, marked `"run":"classes"` (the same trees, about 7% more CPU).
* `classes/ID.classes.tsv`: the classes of the closure, as `--trace` writes them, for 3300-14, 3441-1262 and 3441-5459.
* `analyze.py` makes the first trial's tables: `./analyze.py runs.jsonl classes ROWS`, with `ROWS` a directory of the
  test's rows; `analysis.txt` is its output.  The rows, 25 MB a big unit, are not kept, because a one-thread solve of a
  unit is the same every time.
* `suite.py` runs the suite (`run`: two runs at a time, waiting while the load average is over 8, no run made twice,
  stopping when a search differs from the other in its best or conclusion or when the default search's steps are not
  the suite's) and makes the tables (`table`, `spread`).  Its module comment has the options.
* `suite-runs.jsonl`: a line of `wall images` for each run of the suite, with the problem's `gap`, the one-minute `load`
  at its start and its `wall_s` added: the 46 `count` and 46 `closure` runs, 12 `learn` (the images alone), 7 `bare`
  and one `check` (the test, at 3440-12).
* `suite-perturbed.jsonl`: the default search (`count`) of the twelve units under 100,000 steps with `nogood_period`
  at 20,000 and at 30,000.

The first trial, from `solver/frame` after `cargo build -j 2 --profile test --example wall --features studies`:

```text
target/debug/examples/wall images \
  --suite ../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl \
  --known ../../results/strict.jsonl --ids 3300-14 [--trace DIR | --bare | --learn [--closure] [--trace DIR]] 3300
```

and the same with `--ids 3441-1262,3441-5459` and `3441`.  The suite trial, from this folder after the same build (the
runs used a copy of the binary, named with `--bin`, so that a rebuild during them could not replace it):

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
nogood_period=30000` makes the other half of `suite-perturbed.jsonl`.  Run as `python3 -I suite.py ...`.

## Commits

* 9bb8c0b: `wall images` and the hooks `excluding` with the conflict, `propose` and `learned`, studies only; the first
  trial and its entry.
* b371143: `wall images --root` and `--count` with their test, and `suite.py`; the suite trial.
* 0343c5a: the verdict, closed, not adopted.
