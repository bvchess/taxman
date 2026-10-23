# Faster nodes with the Lagrangian bound, same trees (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  With the [Lagrangian bound over the nogoods](../a-lagrangian-bound-over-the-nogoods-2026-10/README.md)
proofs take about 3.3 times fewer cycles, but each step costs about half as much again (the probe's window of 1215:
24.6k cycles a step against 16.4k with `--tactic lagrange=off`).  Where do the cycles go, what can be made cheaper
without changing any tree, and does `match_refine` = 0 pay with the Lagrangian bound?  **Answer.**  The Hungarian
searches take 55-60% of the time and the Lagrangian levels 26-29% in all.  Five changes, which leave every tree as it
was, take 10.6% and 10.2% off the cycles of the probe's windows, 5% to 10% off whole proofs of 936-1254 and 8.1% off the
chain 1201-1231.  `match_refine` stays 1: 0 gives a geometric mean of −3.0% over 19 proofs, from −21% to +20%, and +0.4%
on the chain.

## Method

Every change keeps both fingerprints of the probe (36981d0eaeeae319 with the defaults, 6197a47eae6cf8c8 with
`--tactic lagrange=off`) and the node counts of one-thread proof-only searches (`ladder --threads 1 --seed-opt --known
results/strict.jsonl --tactic revise_freedoms=off N`), and is measured in cycles and instructions retired (the probe's
own, the fastest of its repetitions, three runs of each build, interleaved; for whole searches `/usr/bin/time -l`).  The
laptop was shared with a chain run and other agents (load 2 to 30), so instructions separate signal from noise where
cycles move by a percent or two.

The profile is `solver/tools/profile/profile.sh N 60 2` (commit a2da702) on the hardest games of the strict chain as the
chain runs them, two threads: 1254 (done in 25.3 s), 1272 (17.2 s), 1288 (34.1 s), 1290 and 1300 (stopped at 60 s).
`attribute.py` charges the Lagrangian levels' work to kinds of its own (commit dd4f89c).  For the picture by source line
the proof of 1215 was recorded with Instruments' Time Profiler (`xcrun xctrace record --template 'Time Profiler'
--launch -- ...`, every sample with its own address).  `sample` lists the several addresses of a leaf on one line and
the attribution symbolicates only the first, which is right to the function but coarse below it.

## Results

### Where the time goes

Shares of busy samples across the five games, by kind of work:

* **The Hungarian searches are 55-60% of the time**: the matching's repairs 33-39% (33.4% to 38.8%), and the
  Lagrangian levels' repairs 16-18%.  A Lagrangian level costs about what a matching level does.
* **The rest of the Lagrangian levels** is 10.2-10.8%: refits (50 solves from scratch each) 3.7-4.3%, forced members
  from the potentials 2.9-3.3%, applying changes (`derive`) 1.5-2.2%, copies 1.3-1.7%.  With the repairs the levels
  take 26-29%.
* **Peels and promotions** are 17-20% (17.1-19.9%), the node's own work 4.8-6.5%, the nogood table 4.0-5.1%, the
  matching's bounds from the potentials 4.0-4.5%, matching level copies 2.3-2.7% and other copies 0.7-0.8%.
* **By purpose**: the Lagrangian bound of a node or child and its forced members 22.7-25.0%, making the child
  20.3-23.5%, strong branching 13.4-19.9%, refinement 13.5-14.8%, the matching bound of a node or child 8.8-9.9%,
  forced members and the conflict 4.9-5.5%, conflict lookup or computation 3.7-4.5%, refits of the multipliers
  3.6-4.3%.
* **The searches are large**: at the proof of 1215 (one thread, its first 300,000 steps) 88% of the vertices the
  searches put in their trees are in trees of 64 to 511 vertices, of the graph's 686, so their work is per edge, not per
  search.
* **Strong branching repeats refinement's work.**  At 1250 (one thread, per step) strong branching ran 0.26 times with
  0.70 exact solves, each run after refinement had solved the level without the conflict's smallest member (0.45 of
  those a step), the very level strong branching solves first.
* **The edges are cheap already**: Instruments agrees on the kinds (the matching's searches 31.9%, the Lagrangian
  levels' 19.0%, peels 18.4%) and shows the Hungarian search's edges at about 20 instructions when an edge is skipped
  and 45 when it pushes an event, at about 3.3 instructions a cycle.

### Changes kept

Cumulative, all builds measured together (k cycles a step in the probe's windows, the defaults and `lagrange=off`; G
cycles of the proof of 1250):

| commit | change | window 936 | window 1215 | off: 936 | off: 1215 | proof 1250 |
|---|---|---|---|---|---|---|
| a2da702 | before | 24.15 | 24.56 | 17.35 | 16.44 | 20.56 |
| 40bc726 | strong branching takes the level refinement solved for the first member | 22.41 | 22.85 | 15.78 | 15.01 | 19.66 |
| 602a4b6 | the bound from the potentials stops its scans at a tight edge | 21.98 | 22.53 | 15.52 | 14.78 | 19.25 |
| 34648e1 | one settle for the forced members, a membership test for known conflicts | 21.86 | 22.36 | 15.36 | 14.71 | 19.04 |
| 5755c3e | the Hungarian search's fields apart while reaching, a fixed-size code set | 21.63 | 22.22 | 15.29 | 14.68 | 18.99 |
| 31b9c95 | the Lagrangian bound's forced members only among those not forced yet | 21.59 | 22.04 | 15.35 | 14.61 | 18.91 |

* **Refinement's first branch for strong branching** (40bc726).  Refinement solves the node without the smallest member
  of its conflict, and strong branching then solved that level again; refinement keeps it
  (`MatchBound::keep_solved`) and strong branching takes it.  The same level, so the same tree.  The windows −7.2% and
  −7.0%, `lagrange=off` −9.1% and −8.7%, the proof of 1250 −4.3%.
* **The bound from the potentials stops at a tight edge** (602a4b6).  `bound_excluding` scans every mover above `x`
  and above the factor it pays with; no counted edge has negative slack, so a scan stops when it reaches the
  potential, with the same value.  −1.4% to −2.1%.
* **Cheaper node work** (34648e1): the matching's forced members found with one settle of the level instead of one
  per member, and whether a known conflict can rule out a candidate of a child that promotes one number is a
  membership test, not a set intersection.  −0.5% to −1.1%.
* **The Hungarian search's fields apart** (5755c3e): `add_inner` reached each neighbor through `&mut self`, so the
  slots' place and the epoch were loaded again at every edge; taken apart, they stay in registers.  Bucket 0's code set
  is a bitset of fixed size.  Instructions −2.0% to −2.2%, cycles −0.2% to −1.1%.
* **Forced members only among those not forced yet** (31b9c95): the Lagrangian bound's forced members join the
  matching's, so its heaviest matching's members already among them need no bound.  Instructions −0.5%, cycles
  −0.8% to +0.4%.

Matchbound's tests check each level taken ahead against the level solved afresh, and the agreement tests check every
member against the full scan.

**In all**, the windows −10.6% and −10.2% cycles (−15.3% and −14.0% instructions), with `lagrange=off` −11.6% and
−11.1%; the probe in all −5.5% (its kernels of peels and frame checks are unchanged).  Whole proofs lose 5.4% (1100) to
9.2% (1215) of their cycles with the nodes unchanged: 936 1.22 → 1.11 G (−9.1%), 1050 −6.3%, 1170 −5.7%, 1184 −8.5%,
1215 33.1 → 30.1 G, 1250 20.6 → 18.9 G (−8.0%), 1254 146.9 → 134.0 G (−8.8%); with `lagrange=off` 936 −8.3%, 1100 −6.0%,
1184 −9.5%, 1215 107.3 → 96.0 G (−10.5%).  The strict chain 1201-1231 on one thread (`taxman solve 1201-1231 --strict
--allow-tainted-prev`, from optimal.json's 1200) takes 1092.0 G cycles (345 s) before and 1003.2 G (315 s) after, −8.1%
(instructions −14.0%), every game's nodes the same; `taxman regress --to 600 --strict` −1%.  1254 as the chain runs it
on two threads takes 21.4 s against 25.3 s.  A step of the window of 1215 costs 22.0k cycles with the Lagrangian bound
and 14.6k without.  Strong branching falls from 16-20% of the time to 11.4% at the proof of 1215, the matching's
searches to 32.6% and the Lagrangian levels' to 19.2% (Instruments).

### Measured and not kept

Instructions and cycles, one thread:

* **One key for a search slot's epoch and tag**: instructions −0.02%, cycles within ±0.8%; the compiler had made the two
  tests cheap already.
* **`-C target-cpu=native`** (not the default build): instructions ±0.05%, cycles within noise.
* **Skipping the forced-member check of a member `x` with `value − x ≥ need`**: it applies to 0.17 of the 19.8 checks
  of a step at 1250 (0.009 of 17.8 for the Lagrangian bound's); not built.
* **Strong branching skipping a member whose lower bound `value − x` is above the best so far**: it can never apply,
  since the members are solved in increasing order.
* **Refinement only where the Lagrangian bound does not cut**: a Lagrangian level is derived from the levels made
  before it, so which optimal matching it reaches depends on that history, and it changes trees.  Not built.
* **A level as one record per vertex**: not built; at 24 bytes a vertex against 11, its copies (4% of the time)
  would cost what it saves.

### `match_refine` = 0

One-thread proof-only searches with the build of 31b9c95 (G cycles, with `match_refine` 1 and 0; the last column is the
change of 0 against 1; 0 takes more nodes in 15 of the 19):

| proof | G cycles, 1 | G cycles, 0 | cycles |
|---|---|---|---|
| 936 | 1.14 | 1.12 | −1.5% |
| 1050 | 2.00 | 1.86 | −6.8% |
| 1100 | 1.03 | 1.01 | −2.3% |
| 1133 | 0.36 | 0.36 | −0.3% |
| 1170 | 0.51 | 0.51 | −1.0% |
| 1184 | 1.50 | 1.46 | −2.7% |
| 1196 | 5.38 | 5.89 | +9.5% |
| 1204 | 12.9 | 12.7 | −1.6% |
| 1215 | 29.7 | 27.8 | −6.6% |
| 1220 | 80.1 | 80.1 | 0.0% |
| 1225 | 79.6 | 95.3 | +19.7% |
| 1240 | 17.0 | 17.5 | +2.9% |
| 1250 | 19.0 | 16.7 | −12.1% |
| 1254 | 134.4 | 106.2 | −21.0% |
| 1257 | 97.0 | 83.5 | −13.9% |
| 1272 | 116.4 | 108.7 | −6.6% |
| 1280 | 99.2 | 113.7 | +14.6% |
| 1285 | 106.2 | 105.2 | −0.9% |
| 1288 | 150.4 | 123.8 | −17.7% |

Thirteen proofs are cheaper without refinement, five dearer, one even: a geometric mean of −3.0% (the logarithms'
standard error 2.4%), −5.3% in all (953.8 G against 903.3 G), −3.3% on the thirteen from 1196 up.  Without refinement
nodes are cheaper and usually more; but what the tree becomes (the nogoods learned, the Lagrangian bound's multipliers
and forced members) swings each game by up to 20% either way.  The strict chains, one thread, the same build:
`taxman regress --to 600 --strict` 5.76 G against 5.70 G (−1%, two runs each, within their spread); `--to 1000` 513.4 G
(162.1 s) against 465.0 G (145.4 s), −9.4%; and the frontier, `taxman solve 1201-1231 --strict --allow-tainted-prev`
from optimal.json's 1200, 1003.2 G (315 s) against 1007.3 G (317 s), +0.4% (1215 8.5 → 13.5 s, 1221 20.2 → 16.7 s,
1225 24.8 → 29.9 s, 1230 201.0 → 197.4 s).

## Decision

Kept: the five changes, the same trees with about 10% fewer cycles at the frontier.  `match_refine` stays 1:
refinement no longer pays on average, by a few percent, but the evidence is not clear across the range, a mean of about
one standard error with games swinging 20% either way, and the frontier's chain is even.  It would take a larger sample
of the frontier (the searched games of a strict chain past 1231, on the many-core machine) to move it.

What is left: the Hungarian searches are over half the time and large, and their edges are already cheap.  Fewer of
them would change trees (refits that keep the levels, fewer Lagrangian levels) or take a refactor of the levels (the
record per vertex above).  The peels, 17-21%, are the work [faster nodes, same
trees](../faster-nodes-same-trees-2026-10/README.md) took on and left.

## Files

* `README.md`: this record.  The code of the changes is in the commits below; the profile is
  `solver/tools/profile/profile.sh N 60 2`, with its `attribute.py`.

## Commits

* a2da702: the Lagrangian bound merged; the build the profile and the "before" row measure.
* dd4f89c: `attribute.py` charges the Lagrangian levels' work to kinds of its own.
* 40bc726: strong branching takes the level refinement solved for the first member.
* 602a4b6: the bound from the potentials stops its scans at a tight edge.
* 34648e1: one settle for the forced members, a membership test for known conflicts.
* 5755c3e: the Hungarian search's fields apart while reaching, a fixed-size code set.
* 31b9c95: the Lagrangian bound's forced members only among those not forced yet; the final build of the study.
