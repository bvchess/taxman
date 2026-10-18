# Faster nodes with the Lagrangian bound, same trees (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  With the Lagrangian bound ("A Lagrangian bound over the nogoods", above) proofs take about 3.3 times fewer
cycles, but each step costs about half as much again (the probe's window of 1215: 24.6k cycles a step against 16.4k
with `--tactic lagrange=off`).  Where do the cycles go now, and what can be made cheaper without changing any tree?
And does `match_refine` = 0, which the last round flagged, now pay?

*Method.*  As in "Faster nodes, same trees": every change keeps both fingerprints of the probe (36981d0eaeeae319 with
the defaults, 6197a47eae6cf8c8 with `--tactic lagrange=off`) and the node counts of one-thread proof-only searches
(`ladder --threads 1 --seed-opt --known results/strict.jsonl --tactic revise_freedoms=off N`), and is measured in
cycles and instructions retired (the probe's own, the fastest of its repetitions, three runs of each build,
interleaved; for whole searches `/usr/bin/time -l`).  The laptop was shared with a chain run and other agents (load
2 to 30), so instructions separate signal from noise where cycles move by a percent or two.

**The profile.**  `solver/tools/profile/profile.sh N 60 2` (commit a2da702) on the hardest games of the strict chain as the
chain runs them, two threads: 1254 (done in 25.3 s), 1272 (17.2 s), 1288 (34.1 s), 1290 and 1300 (stopped at 60 s).
`attribute.py` now charges the Lagrangian levels' work to kinds of its own (commit dd4f89c).  Shares of busy
samples:

| kind of work | 1254 | 1272 | 1288 | 1290 | 1300 |
|---|---|---|---|---|---|
| matching repairs (Hungarian searches) | 34.3% | 33.4% | 36.0% | 38.8% | 36.8% |
| Lagrangian levels: repairs (Hungarian searches) | 17.5% | 18.1% | 17.7% | 16.4% | 16.0% |
| Lagrangian levels: refits (50 solves from scratch each) | 4.2% | 4.3% | 4.0% | 3.7% | 4.1% |
| Lagrangian levels: forced members (from the potentials) | 3.1% | 3.3% | 2.9% | 3.3% | 2.9% |
| Lagrangian levels: applying changes (`derive`) | 1.9% | 1.5% | 2.2% | 2.0% | 1.7% |
| Lagrangian levels: copies | 1.4% | 1.7% | 1.5% | 1.3% | 1.5% |
| peels and promotions | 19.1% | 19.9% | 17.4% | 17.1% | 18.3% |
| the node's own work | 5.0% | 4.8% | 6.4% | 6.0% | 6.5% |
| nogood table | 5.1% | 4.8% | 4.0% | 4.0% | 4.2% |
| bounds from the potentials (the matching's) | 4.4% | 4.5% | 4.2% | 4.0% | 4.3% |
| matching level copies | 2.7% | 2.7% | 2.4% | 2.3% | 2.4% |
| other copies (frame sets, nodes) | 0.8% | 0.7% | 0.7% | 0.8% | 0.8% |

| purpose | 1254 | 1272 | 1288 | 1290 | 1300 |
|---|---|---|---|---|---|
| the Lagrangian bound of a node or child, and its forced members | 24.3% | 25.0% | 25.0% | 23.5% | 22.7% |
| making the child (its promotion, candidate list, rule-outs) | 22.4% | 23.5% | 20.6% | 20.3% | 21.9% |
| strong branching (exact bounds without each member) | 16.2% | 13.4% | 16.6% | 19.9% | 18.4% |
| refinement (bounds of the branches of the conflict) | 13.5% | 13.6% | 14.8% | 14.2% | 13.6% |
| the matching bound of a node or child | 8.8% | 9.9% | 8.8% | 8.8% | 9.0% |
| forced members and the conflict (`Refiner::top`) | 5.5% | 5.5% | 5.0% | 4.9% | 5.3% |
| conflict lookup, or computation | 4.1% | 3.7% | 4.4% | 4.3% | 4.5% |
| refits of the multipliers | 4.1% | 4.3% | 4.0% | 3.6% | 4.0% |

So the Hungarian searches are 55-60% of the time, the matching's and the Lagrangian levels' together; a Lagrangian
level costs about what a matching level does.  The searches are large: at the proof of 1215 (one thread, its first
300,000 steps) 88% of the vertices the searches put in their trees are in trees of 64 to 511 vertices, of the
graph's 686, so their work is per edge, not per search.  And two counts at 1250 (one thread, per step): strong
branching ran 0.26 times with 0.70 exact solves, each run after refinement had solved the level without the
conflict's smallest member (0.45 of those a step), the very level strong branching solves first.

`sample` lists the several addresses of a leaf on one line and the attribution symbolicates only the first, which is
right to the function but coarse below it (`attribute.py` says so).  For the picture by source line the proof of
1215 was recorded, after the first change below, with Instruments' Time Profiler
(`xcrun xctrace record --template 'Time Profiler' --launch -- ...`, every sample with its own address), which agrees on
the kinds (the matching's searches 31.9%, the Lagrangian levels' 19.0%, peels 18.4%) and shows the Hungarian search's
edges at about 20 instructions when an edge is skipped and 45 when it pushes an event, at about 3.3 instructions a
cycle.

**Changes kept.**  Cumulative, all builds measured together (k cycles a step in the probe's windows, the defaults and
`lagrange=off`; G cycles of the proof of 1250):

| commit | change | window 936 | window 1215 | off: 936 | off: 1215 | proof 1250 |
|---|---|---|---|---|---|---|
| a2da702 | before | 24.15 | 24.56 | 17.35 | 16.44 | 20.56 |
| 40bc726 | strong branching takes the level refinement solved for the first member | 22.41 | 22.85 | 15.78 | 15.01 | 19.66 |
| 602a4b6 | the bound from the potentials stops its scans at a tight edge | 21.98 | 22.53 | 15.52 | 14.78 | 19.25 |
| 34648e1 | one settle for the forced members, a membership test for known conflicts | 21.86 | 22.36 | 15.36 | 14.71 | 19.04 |
| 5755c3e | the Hungarian search's fields apart while reaching, a fixed-size code set | 21.63 | 22.22 | 15.29 | 14.68 | 18.99 |
| 31b9c95 | the Lagrangian bound's forced members only among those not forced yet | 21.59 | 22.04 | 15.35 | 14.61 | 18.91 |

* **Refinement's first branch for strong branching** (40bc726).  Where nothing is forced, refinement solves the node
  without the smallest member of its conflict, and strong branching then solved that level again.  Refinement now
  keeps it (`MatchBound::keep_solved`, in the spare slot that holds strong branching's kept level), `value_excluding`
  of that member takes it, and a node that goes on without that member (a conflict of one) takes it as its kept level;
  anything else done to the node drops it.  The same level, so the same tree; the walk of matchbound's tests keeps
  levels solved ahead and checks each one taken against the level solved afresh.  The windows −7.2% and −7.0%,
  `lagrange=off` −9.1% and −8.7%, the proof of 1250 −4.3%.
* **The bound from the potentials stops at a tight edge** (602a4b6).  `bound_excluding` lowers the potentials of `x`
  and of the factor it pays with as far as their edges allow, a scan of every mover above each.  No counted edge has
  negative slack, so neither can need more than its potential: a scan stops when it gets there, the same value (the
  agreement tests check every member against the full scan).  −1.4% to −2.1%.
* **Cheaper node work** (34648e1): the matching's forced members found with one settle of the level instead of one
  per member (the scan shared with the Lagrangian bound's), and whether a known conflict can rule out a candidate of
  a child that promotes one number is a membership test, not a set intersection.  −0.5% to −1.1%.
* **The Hungarian search's fields apart** (5755c3e).  `add_inner` reached each neighbour through `&mut self`, so the
  slots' place and the epoch were loaded again at every edge (an event pushed might have moved them); taken apart,
  they stay in registers.  And bucket 0's code set is a bitset of fixed size, whose insertion checks nothing.
  Instructions −2.0% to −2.2%, cycles −0.2% to −1.1% (five runs each when it was committed: −1.3% and −1.1% in
  the windows, none on the proof of 1250).
* **Forced members only among those not forced yet** (31b9c95): the Lagrangian bound's forced members join the
  matching's, so its heaviest matching's members already among them need no bound.  Instructions −0.5%, cycles
  −0.8% to +0.4%.

In all, the windows −10.6% and −10.2% cycles (−15.3% and −14.0% instructions), with `lagrange=off` −11.6% and −11.1%
(−16.2% and −15.6%); the probe in all −5.5% (its kernels of peels and frame checks are unchanged).  Whole proofs,
before and after (G cycles, nodes unchanged): 936 1.22 → 1.11 (−9.1%), 1050 2.11 → 1.97 (−6.3%), 1100 1.07 → 1.01
(−5.4%), 1170 0.56 → 0.52 (−5.7%), 1184 1.65 → 1.51 (−8.5%), 1215 33.1 → 30.1 (−9.2%), 1250 20.6 → 18.9 (−8.0%), 1254
146.9 → 134.0 (−8.8%); with `lagrange=off` 936 −8.3%, 1100 −6.0%, 1184 −9.5%, 1215 107.3 → 96.0 (−10.5%).  (1133,
2,122 steps, is mostly setup: +2.3% cycles, −1.7% instructions.)  The strict chain 1201-1231 on one thread (`taxman
solve 1201-1231 --strict --allow-tainted-prev`, from optimal.json's 1200): 1092.0 G cycles (345 s) before, 1003.2 G
(315 s) after, −8.1% (instructions −14.0%), every game's nodes the same; `taxman regress --to 600 --strict` −1%.
1254 as the chain runs it on two threads, profiled again: 21.4 s against 25.3 s.  A step of the window of 1215 now costs
22.0k cycles with the Lagrangian bound and 14.6k without.

Where the time goes now (the proof of 1215, one thread, by Instruments): the matching's searches 32.6%, the
Lagrangian levels' 19.2%, peels 18.9%, the node's own work 9.7%, the nogood table 3.7%, refits 3.2%, bounds from the
potentials 3.0% and the Lagrangian bound's 2.6%, level copies 2.3% and 1.7%, the Lagrangian levels' changes 1.7%;
by purpose the Lagrangian bound 25.8%, making children 23.5%, refinement 13.8%, strong branching 11.4% (16-20%
before), the matching bound of a node or child 10.5%.

**Measured and not kept** (instructions and cycles, one thread):

* One key for a search slot's epoch and tag (the epoch times 4 plus the tag, one comparison): instructions −0.02%,
  cycles within ±0.8%.  The compiler had made the two tests cheap already.
* `-C target-cpu=native` (not the default build): instructions ±0.05%, cycles within noise.
* Skipping the forced-member check of a member `x` with `value − x ≥ need` (its bound without `x` cannot fall below
  `need`): counted first, it applies to 0.17 of the 19.8 checks of a step at 1250 (0.009 of 17.8 for the Lagrangian
  bound's); not built.
* Strong branching skipping a member whose lower bound `value − x` is above the best so far and at least `need`: it
  can never apply, since the members are solved in increasing order and `value − x` is below the exact bound of every
  member before `x`.
* Refinement only where the Lagrangian bound does not cut: refinement one level deep has no side effect but its
  levels, yet a Lagrangian level is derived from the levels made before it, and which optimal matching it reaches
  (so which members its potentials show forced) depends on that history: it changes trees.  Not built.
* A level as one record per vertex (potential, weight, partner and role together, so that an edge loads one place):
  not built; at 24 bytes a vertex against 11 now, its copies (4% of the time) would cost what it saves.

**`match_refine` = 0.**  One-thread proof-only searches with the build of 31b9c95 (G cycles and nodes; the ratio is
of cycles, 0 against 1):

| proof | nodes, 1 | nodes, 0 | G cycles, 1 | G cycles, 0 | cycles |
|---|---|---|---|---|---|
| 936 | 41,653 | 44,119 | 1.14 | 1.12 | −1.5% |
| 1050 | 46,527 | 48,049 | 2.00 | 1.86 | −6.8% |
| 1100 | 29,614 | 30,690 | 1.03 | 1.01 | −2.3% |
| 1133 | 2,122 | 2,156 | 0.36 | 0.36 | −0.3% |
| 1170 | 8,013 | 8,136 | 0.51 | 0.51 | −1.0% |
| 1184 | 52,259 | 54,083 | 1.50 | 1.46 | −2.7% |
| 1196 | 216,076 | 258,764 | 5.38 | 5.89 | +9.5% |
| 1204 | 498,121 | 530,763 | 12.9 | 12.7 | −1.6% |
| 1215 | 1,111,451 | 1,123,153 | 29.7 | 27.8 | −6.6% |
| 1220 | 2,503,016 | 2,739,817 | 80.1 | 80.1 | 0.0% |
| 1225 | 2,178,362 | 2,875,317 | 79.6 | 95.3 | +19.7% |
| 1240 | 496,460 | 567,144 | 17.0 | 17.5 | +2.9% |
| 1250 | 709,893 | 669,488 | 19.0 | 16.7 | −12.1% |
| 1254 | 4,830,651 | 4,075,698 | 134.4 | 106.2 | −21.0% |
| 1257 | 3,686,322 | 3,447,999 | 97.0 | 83.5 | −13.9% |
| 1272 | 4,341,863 | 4,420,692 | 116.4 | 108.7 | −6.6% |
| 1280 | 3,080,373 | 3,930,824 | 99.2 | 113.7 | +14.6% |
| 1285 | 3,442,845 | 3,692,516 | 106.2 | 105.2 | −0.9% |
| 1288 | 4,822,476 | 4,398,614 | 150.4 | 123.8 | −17.7% |

Thirteen proofs are cheaper without refinement, five dearer, one even: a geometric mean of −3.0% (the logarithms'
standard error 2.4%), −5.3% in all (953.8 G against 903.3 G), −3.3% on the thirteen from 1196 up.  Without
refinement nodes are cheaper and usually more; but what the tree becomes (the nogoods learned, the Lagrangian
bound's multipliers and forced members) swings each game by up to 20% either way.  The strict chains, one thread, the
same build: `taxman regress --to 600 --strict` 5.76 G against 5.70 G (−1%, two runs each, within their spread);
`--to 1000` 513.4 G (162.1 s) against 465.0 G (145.4 s), −9.4%; and the frontier, `taxman solve 1201-1231 --strict
--allow-tainted-prev` from optimal.json's 1200, 1003.2 G (315 s) against 1007.3 G (317 s), +0.4% (1215 8.5 → 13.5 s,
1221 20.2 → 16.7 s, 1225 24.8 → 29.9 s, 1230 201.0 → 197.4 s).  (The last round's four proofs gave 0 to 26% less;
refinement's first branch, now shared with strong branching, narrowed that by 2 to 5 points.)

*Decision.*  Kept: the five changes, the same trees with about 10% fewer cycles at the frontier.  `match_refine` stays
1: refinement no longer pays on average, by a few percent, but the evidence is not clear across the range, a mean
of about one standard error with games swinging 20% either way, and the frontier's chain is even.  It would take a
larger sample of the frontier (the searched games of a strict chain past 1231, on the many-core machine) to move it.

*What is left.*  The Hungarian searches are over half the time and large, and their edges are already cheap; fewer
of them would change trees (refits that keep the levels, fewer Lagrangian levels) or take a refactor of the levels
(see the record per vertex above).  The peels, 17-21%, are the work "Faster nodes, same trees" took on and left.
