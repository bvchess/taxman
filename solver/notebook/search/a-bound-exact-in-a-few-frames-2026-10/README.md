# A bound exact in a few frames (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-20.

**Question.**  Is the GF(2) bound's error at a node local, and could a bound exact in the frames of the hot home, and
GF(2) in the others, remove much of it cheaply, perhaps cached where the same local situation recurs?

**Answer.**  The error is local, and a bound exact in one frame removes all of it where the tree is, but not cheaply:
computing that bound is the search itself.

* **Where it sits.**  Frame 2 holds it.  Its factors are home 2's candidates, and its promoted moves are home 3's.  At
  45 nodes of three hard units and two whole games with gaps under 1,000, a bound exact in frame 2 alone cuts 40.  At
  the roots of the whole games the heaviest set's excess is at home 3 (+1,612 and +2,093 points, against −565 and
  −834 at home 2).
* **What it costs.**  Frame 2's conflicts are the tree: 50-83% of the conflicts the searches meet, and 85-97% of the
  members they go on without.  A bound exact in frame 2 is a search of those conflicts, and it took a median 1.08
  times the steps of the node's own proof (quartiles 0.57 and 1.94).  The cheap part, frame 3 alone, closes a median
  16% of the gap.
* **Recurrence.**  Frame 2's local situation recurs at 3-22% of a search's node checks, frames 2 and 3 together at
  1-8%, so no cache helps.

So nothing was prototyped, and the avenue is closed: the prize is large where the tree is, but the bound costs what
the search costs, and its situations do not recur.

## Words

* A node's *gap* is its GF(2) bound less what its completions must add to reach the optimum: the
  [glossary's](../../../../docs/GLOSSARY.md) node gap, with the optimum as the incumbent.  Frames, homes, candidates and
  conflicts are the glossary's; a unit and an exclusion (a member a node goes on without) are those of [the anatomy of
  a hard subtree](../the-anatomy-of-a-hard-subtree-2026-10/README.md).
* The *hybrid bound* of a node *exact in the frames F* is the heaviest set of its candidates that is independent in
  both GF(2) frame matroids and leaves every frame of F winnable.  Exact in no frame it is the GF(2) bound; exact in
  every frame it is the node's best completion.
* The *share* it closes is the GF(2) bound less the larger of the hybrid bound and the optimum's part, over the gap: 1
  when the hybrid bound cuts the node.
* A node's *local situation* for the frames F is its promotions and its candidates among the candidates that touch F
  (a candidate of home h touches the frames h − 1 and h).  It is all that a computation exact in F, with fixed
  weights, depends on.

## Why the hybrid bound is valid

Every completion of a node lies in the family that defines the hybrid bound, so the family's heaviest member weighs at
least the best completion.  This is proved, given the frame model.

* A completion of a node with the promotions P is a set L of its candidates such that P ∪ L can be promoted from the
  root.  By Theorem M ([exactness.md, "The exactness
  theorem"](../../../../docs/theory/exactness.md#the-exactness-theorem)), that holds exactly when
  every frame is winnable with its part of P ∪ L applied.  So every frame of F is winnable with L applied.
* By Theorem 11 ([frame-bounds.md, "The frame
  bound"](../../../../docs/theory/frame-bounds.md#the-frame-bound)), in a winnable frame
  the vectors of the movers and the unit vectors of the factors promoted out are independent over GF(2).  So L is
  independent in both frame matroids of the node, in every frame, checked exactly or not.
* So L is in the family, and the hybrid bound is at least the best completion.  It is never above the GF(2) bound, and
  it can only fall as F grows.
* The interface between the exact frames and the others needs nothing more.  The family is one of single sets with
  both constraints on each, so a candidate on F's boundary, which touches one exact frame and one GF(2) frame, meets
  both at once.  No candidate is split into copies, and no price on the interface is needed.  A Lagrangian
  decomposition that does split them, as [the per-frame bound](../the-per-frame-bound-2026-10/README.md) does, is
  valid for any prices but weaker.

**How it is computed.**  The solver's own search, from the node, with the GF(2) bound, the optimum as its incumbent
and without the rule of symmetric pairs, on a model whose frames outside F are not checked (`FrameGraph::relaxing`, in
the studies build).  Every fact it prunes by holds for the family:

* the GF(2) bound bounds the family, and forced members follow from it by weak duality;
* a conflict found in a frame of F lies in no member, since a subset of a member is a member (fewer moves and more
  factors keep a frame winnable, and independence is inherited), and the nogoods are such conflicts;
* the relaxed model promotes the members of the search's heaviest sets one at a time exactly when it can promote them
  at once.  A reduction in a relaxed frame never takes a member of a set independent in the matroids; the comment of
  `FrameGraph::relaxing` has the argument, and a test checks it on 4,728 independent random sets of the golden boards.

So the search's best is the hybrid bound if that beats the optimum, and if it does not, the hybrid bound cuts the node.
At all 49 nodes the search exact in no frame returned the GF(2) bound, and at the 46 where the search exact in every
frame finished it returned the optimum; the example's test checks both at 1215.

## What was run

`wall local` (`solver/frame/examples/wall/local.rs`, with its tests), from the studies build with `--profile test`,
one thread, under `nice -n 10`, on a laptop.

* **Searches.**  Three units of [the hardest searches of the run to
  5000](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md), 3940-104, 4225-1.15 and 4416-95, each solved to
  the end by the default search from the optimum; and the first 400,000 steps of the proofs of the whole games 4050 and
  4176.  A hook recorded every node whose subtree took 100 steps or more, the frame of every conflict and exclusion,
  and each frame set's local situation at every node check.
* **Nodes.**  Nine or ten nodes a search, about as many from each decade of subtree size, the unit's root among them:
  49 nodes of 100 to 374,349 steps, with gaps of 185 to 1,211.  At each, the hybrid searches exact in frame 1, 2 or 3,
  in frames 2 and 3, in frames 1 to 3 (the units only), in every frame (the node's proof) and in none.  Each stopped at
  400,000 steps (200,000 in the whole games), and a stopped search gives a range: what it found, and the largest bound
  left on its path.
* **Consecutive games.**  The frames and candidates of 120 pairs of games n and n + 1 in 3930-3960, 4040-4060,
  4170-4185, 4215-4235 and 4400-4430.

The runs used about 1.1 CPU-hours, and the builds and tests about 0.4 more; none ran on EC2.

## Results

### Where the error sits

The searches branch in frame 2.  The conflicts their default searches met, and the members they went on without, by
frame:

| search | steps | conflicts by frame (1 / 2 / 3 / 4) | exclusions by frame (1 / 2 / 3 / 4) |
|---|---|---|---|
| 3940-104 | 104,080 | 0.20 / 0.74 / 0.04 / 0.01 | 0.10 / 0.86 / 0.03 / 0.00 |
| 4225-1.15 | 136,320 | 0.04 / 0.83 / 0.11 / 0.02 | 0.07 / 0.91 / 0.02 / 0.00 |
| 4416-95 | 119,309 | 0.00 / 0.77 / 0.09 / 0.14 | 0.00 / 0.97 / 0.01 / 0.01 |
| 4050, the first steps | 400,383 | 0.27 / 0.50 / 0.17 / 0.06 | 0.10 / 0.85 / 0.05 / 0.00 |
| 4176, the first steps | 400,383 | 0.08 / 0.68 / 0.20 / 0.04 | 0.05 / 0.88 / 0.05 / 0.02 |

And the gap is frame 2's.  By band of gap, how many nodes the bound exact in frame 2 cuts, with its steps over those of
the node's proof (the search exact in every frame, with fresh tables), and the median shares of the others:

| gap | nodes | frame 2: cut; steps over the proof's, median | frames 2 and 3: cut | frame 1: median share | frame 3: median share; steps over the proof's |
|---|---|---|---|---|---|
| 185-399 | 8 | 8; 2.25 | 8 | 0.45 | 0.06; 0.12 |
| 400-699 | 25 | 24; 1.06 | 24 | 0.25 | 0.20; 0.02 |
| 700-999 | 12 | 8; 0.87 | 7 | 0.23 | 0.12; 0.00 |
| 1,000-1,211 | 4 | 0 (a median share of 0.36); 5.07 | 2 | 0.09 | 0.29; 0.01 |

* **Under a gap of 1,000, frame 2 holds nearly all of it.**  Exact in frame 2 alone, the bound cuts 40 of the 45
  nodes.  Frames 1 and 3 alone close a median 25% and 16%.  The nodes it does not cut are the largest of the whole
  games: at three nodes of 4050 near its root (gaps 869-923, subtrees of 278,000 to 374,000 steps) frame 2 closes
  59-62%, frames 2 and 3 together 62% where their search finished, and frame 1 alone 24%; at a fourth (gap 613) frame
  2 closes 90%; and at 4176's largest no search exact in frame 2 finished.
* **Above a gap of 1,000, frame 3 joins.**  At the root of 3940-104 and a node below it (gap 1,061) frame 2 closes 72%
  and frame 3 47%, and the two together cut them.  At the two largest nodes of 4416-95 (gap 1,211) no search exact in
  frame 2 finished within 400,000 steps, 3.4 and 8.5 times the proof's.
* **By home.**  Frame 2's conflicts are moves promoted from home 3 and factors of home 2.  At the roots of the whole
  games, which hold the optimum, the heaviest set less the optimum's completion is, by home 1, 2, 3, ...:

  | root | gap | by home |
  |---|---|---|
  | 4050 | 1,013 | 0, −565, +1,612, −34, 0, 0, 0 |
  | 4176 | 1,011 | 0, −834, +2,093, −118, −386, +256, 0, 0 |

  So the error is home 3's, the separator between frames 2 and 3, as [decomposing across the
  frames](../decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md) found at 936-2170.

### The prize, and what it costs

The prize is the whole gap where the tree is: a bound exact in frame 2 would cut the node.  Its price is the node's
proof.

* **The bound exact in frame 2 is the proof.**  Where it cuts a node it takes a median 1.08 times the steps of the
  node's proof, and its quartiles are 0.57 and 1.94.  Leaving the other frames to the GF(2) bound does not make the
  search cheaper, since their small conflicts also cut; at one node of 4416-95 it took 18 times the proof's steps.  So
  where the bound would cut, computing it costs what it cuts, and above that it cuts nothing.
* **The cheap part is small.**  Exact in frame 3, the bound costs a median 0-12% of the proof and closes a median 16%
  of the gap under 1,000 (4-47% at the unit roots).  By the rule of a tenfold of tree for every 300-470 points of gap,
  a sixth of a gap of 600 is worth a factor of 1.6 to 2.2 in the tree, and each node would cost tens of steps.
* **Frame 1** is exact at no cost where it has no conflicts (4416-95), and elsewhere closes a median quarter of the
  gap, the most near the leaves.

The table of every node, with its shares and costs, is in `analysis.txt`.

### Recurrence

How often a node check found a local situation that an earlier check of the same search had:

| search | node checks | frame 1 | frame 2 | frame 3 | frames 2 and 3 | frames 1 to 3 |
|---|---|---|---|---|---|---|
| 3940-104 | 30,347 | 0.08 | 0.09 | 0.74 | 0.08 | 0.01 |
| 4225-1.15 | 43,602 | 0.29 | 0.03 | 0.52 | 0.01 | 0.00 |
| 4416-95 | 38,154 | 0.55 | 0.22 | 0.34 | 0.03 | 0.03 |
| 4050, the first steps | 118,381 | 0.28 | 0.10 | 0.47 | 0.07 | |
| 4176, the first steps | 120,682 | 0.30 | 0.04 | 0.45 | 0.02 | |

* **The frame where the search branches never repeats itself.**  Each exclusion of frame 2 changes frame 2's
  candidates, so its situation changes at nearly every check.  The frames where the search seldom branches keep theirs.
* **What recurs carries no bound that pays.**  Frame 3's situation recurs at a third to three quarters of the checks,
  but the hybrid bound exact in frame 3 depends on the whole node.  What depends on frame 3's situation alone is its
  exact answer at fixed weights, and combined with the GF(2) bound elsewhere that gains nothing: [the per-frame
  bound](../the-per-frame-bound-2026-10/README.md) found the exact frames within 11 points of the GF(2) bound at its
  split's multipliers, its gain coming from multipliers refitted at each node.
* **From one game to the next.**  Of the 120 pairs of consecutive games, 62 have the same frames, number for number,
  which [same search](../same-search-2026-10/README.md) already settles.  Frame 2 differs in 17 pairs (by 1 to 3
  numbers) and frame 3 in 9.  Frame 2's local knowledge could carry from game to game, but that knowledge is its
  conflicts, and [carrying nogoods over](../carrying-nogoods-over-2026-10-measured-not-kept/README.md) did not pay.

## Decision

Not prototyped, and the avenue is closed: the local exact bound has a large prize and no cheap form.

* The error is local: in frame 2 below the roots, with frame 3 at the units' largest gaps, and frames 1 and 3 near the
  roots of whole games.  A bound exact there cuts the nodes that hold the tree.
* Computing it is the search.  Frame 2 holds most of the conflicts and nearly all of the exclusions, so a search of its
  conflicts is the tree, and the hybrid search costs about the proof.
* Nothing recurs to cache.  Frame 2's situation recurs at 3-22% of the checks, and what does recur (frame 3's
  situation, frame 2 from game to game) carries no bound that pays.

It agrees with [the multi-field bound at the
nodes](../the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md) and [the knot in the
bound](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md): a bound strong enough to cut where the
tree is costs about what the tree costs.  Two things would reopen it: a relaxation of one frame that sees its odd cores
without a search (the per-frame entry's open question; none is known), or games whose error moves to a frame small
enough to make exact cheaply, as frame 3 is.

Commits: ba8ec39 (`wall local`, `FrameGraph::relaxing` in the studies build, and their tests) and the commit of this
entry.

## Files

* `local.jsonl.gz`: the lines of the runs, one after another (the fields are in the module comment of
  `solver/frame/examples/wall/local.rs`).  Made, from `solver/frame`, after `cargo build --profile test --example wall
  --features studies`, by these commands under `nice -n 10`:
  * for each unit ID of game N, `target/debug/examples/wall local --known ../../results/strict.jsonl --suite
    ../notebook/search/the-hardest-searches-of-the-run-to-5000-2026-10/suite-hard.jsonl --ids ID --per-unit 12 --checked
    "none/1/2/3/2,3/1,2,3/all" --compare N`;
  * for N = 4050 and 4176, `target/debug/examples/wall local --known ../../results/strict.jsonl --root --nodes 400000
    --per-unit 10 --checked "none/1/2/3/2,3/all" --hybrid-cap 200000 N`;
  * `target/debug/examples/wall local --compare 3930-3960 4040-4060 4170-4185 4215-4235 4400-4430`.
* `analyze.py`: the tables, `python3 -I analyze.py local.jsonl.gz`; `analysis.txt` is its output.
