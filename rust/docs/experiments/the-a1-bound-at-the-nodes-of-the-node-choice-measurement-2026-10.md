# The A1 bound at the nodes of the node-choice measurement (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  Does the multi-field (A1) bound, fitted fully at a node, cut the nodes that head big subtrees?  The
[E3 pilot](the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10.md) measured A1 in a branch and bound (trees five
times smaller at 23 times the cost of a node, a constant factor and no change of slope, and [U4](r3-and-the-slope-test-u4-of-a1-2026-10.md)
found the factor falling with the gap); [the node-choice measurement](../../bench/node_choice/README.md) asks per
node instead, for sampled nodes with known subtrees, what share a strong bound cuts by the node's gap and by the
decade of its subtree, and from what forecast subtree up computing it pays.  It was built for the nogood bound; this
puts A1 in it as two more strong bounds.  The run on EC2 is the lead's; this entry records the instrument and its
first numbers, and the run's results follow as a section of it.

*What was brought back* (research code, no solver change, under the Cargo feature `studies`; commit e8af0ea).
`examples/wall/fields.rs` (253 lines, the node-facing code) with `examples/wall/fields/` (the study's modules from
commit 8753189, adapted to today's library and cut to what a node needs): `dw.rs` (1,921 lines with its tests: the bound
`L`, its integer evaluation, the Dantzig–Wolfe master with rank inequalities and exact checks, the column generation, the
separation, `Warm::child`, Tier 1, R2), `gf3.rs` (899: GF(3) vectors and bases, signings, the signed frame matroids and
their greedy), `cores.rs` (857: a frame's cores with a set applied, column-sum signings, merging), `lp.rs` (1,502: the
master's simplex), `frames.rs` (177: the two GF(2) frame matroids as vectors, the study's `frames.rs` without its
intersection, setup and optima), `fit.rs` (143: a root fit as a file).  Left out: the reference intersection of
`common/meet.rs` (the run-time check of the intersection; a test compares it with the crate's own bound instead), R1, R3,
R2c's node-local signings, the state handed from a node to its children, the checkpoint formats.  `wall/choice.rs`
and `wall/case.rs` add the options `choice-prepare --fit-seconds S --fits DIR` (the game's root fit, once, to
`nNNNN.fit.json`: `DIR/nN.json` if it has one, else the study's main arm, certified, within S seconds) and `choice --a1
--a1-pricings K --a1-seconds S`, which adds to a node's line `a1`: `build_s` and, for the two bounds, `bound`, `cut`,
`stop`, `capped`, `pricings`, `s` (and `tier1` for the first, `rounds` for the second).  The two bounds, each the lowest
`⌊L⌋` it evaluated:

* **r2**, the warm refit: the root fit's multipliers (Tier 1), then at most 20 more evaluations of the column
  generation from the root's master columns and rank inequalities that still hold at the node (`Warm::child`), without
  the exact checks and the separation;
* **full**, the node's own master, the reference of E1n and of the pilot's audit: from the root fit's multipliers and
  frozen signings, with the exact checks and the separation of the node's own cores, stopping at the first bound that
  cuts, within 30 s (then *capped*).

`analyze.py` adds them to the tables of the cut share by gap and by subtree decade, a table of how they ended (cut,
capped, the share of the node's excess closed, the dearest bound), and the policy table (cost: `build_s` and `s`).

*Checks* (laptop, non-release, 4 threads at most; `cargo test --features studies --example wall` 44 pass in 29 s).  The
port reproduces the study: `the_study_fits_give_their_bounds_again` evaluates the saved fits of `bench/fields/fits` at
the roots of 987, 1215, 2700 and 3300 (blocks of 1, 1, 2 and 3 words) and finds the bounds the study recorded (23562,
35168, 163215, 251783); the intersection on this module's vectors equals the crate's bound on `Gf2Graph::reweighted` at
random reweightings of 120, 936, 2700 and 3300; the study's tests of the simplex, the signed matroids, the cores and
the master's breakdown pass.  The harness's test (936, the fixture's nodes, three incumbents each) now also asserts of
both A1 bounds that they are at least the best completion an exhaustive search of the node finds, that they cut
exactly when below `need`, that they never cut a node whose completion beats the incumbent, and that the run has no
fault (a cut agreeing with the solve); the A1 bounds cut 23 times in its 36 measurements (two bounds each).  A path
node's best completion is the check at every run: at the six path nodes of the local run neither bound is below it, r2
equals it at four and full at five (at the others they are above, r2 out of its 20 pricings and full out of its 2 s).

*First numbers* (the M1 Max, test profile, one thread; `choice-prepare --keep 30 --fit-seconds 60 --fits
bench/fields/fits`, `choice --a1 --a1-seconds 10 --cap 60`; 936 and 1215, 30 open nodes each of the probe's fixtures; the
root fits close 0.64 (936, fitted in 5.5 s) and 0.45 (1215, the pilot's) of the root gap, the nogood fit 0.27 and 0.22).
Share of the open nodes cut, by gap, and the median CPU seconds:

| gap | nodes | root λ | nogood fit | A1 r2 | A1 full | r2 s | full s |
|---|---|---|---|---|---|---|---|
| 0-49 | 30 | 43% | 100% | 100% | 100% | 0.002 | 0.002 |
| 50-99 | 16 | 0% | 56% | 69% | 100% | 0.012 | 0.069 |
| 100-199 | 12 | 0% | 8% | 0% | 75% | 0.017 | 1.3 |
| 200-299 | 2 | 0% | 0% | 0% | 0% | 0.063 | 8.6 (capped at 10) |

The subtrees of these nodes are small (median 12 steps at gaps of 50-99, 72 at 100-199; the largest, at 1215's gap 212,
2,997), so the policy table loses everywhere here: the sample says nothing yet about the nodes that head big subtrees,
only that the instrument runs and that the bounds order as the pilot's audit found: the warm refit from the root's
state cuts none of the 14 nodes of gap 100 and more (the pilot's r2, which inherited its parent's state, made 0.41 of
the node's own master's cuts within 20 pricings), and the node's own master cuts three quarters of the 12 at 100-199
within seconds and neither of the two above 200 within 10 s.  A rerun of the 132 A1 bounds (66 lines) after the code was cut
down gave the same bound, cut, pricings and stop at 131, and at the other a `full` out of time at 653 pricings
instead of 654.

*The cloud run* (`A1=1 bench/node_choice/cloud.sh`; [the README](../../bench/node_choice/README.md#the-a1-bounds) has the
stages): one `c8g.16xlarge` (64 workers), the 4 suite games (3441, 3440, 3300, 3425), the 6 games whose proofs make nodes
and 936 and 1215, `KEEP` 100 (about 1,000 nodes), the root fits within 300 s (the fits on file for 1215, 2700 and 3300),
the node's own master within 30 s.  Expected work: the nogood measurement's 6,000 CPU s scaled to the larger sample,
and some 10,000 for the A1 bounds (the nodes of gap 300 and more are expected to use their 30 s); 30 to 40 minutes of
the machine with the build cache, about $0.40 at spot ($0.68 an hour) or $1.50 on demand ($2.55 an hour), the timer of
60 minutes bounding it at $0.68 and $2.55.  The times of an A1 bound at 3300-3441 are not known (a pricing is 1.2 to 2.9
ms at 2200-3400; a node's master takes 100 to 1,000 of them), so the estimate is the less certain half.

*Choices and deviations.*  (1) The warm refit starts from the root's fit, not from a parent: the nodes are sampled
independently, so `Warm::child` takes the root's columns that hold all of the node's promotions (few at depth) and all
its inequalities; its cut share is a lower bound on what a tree would get.  (2) `full` starts from the root's signings
and multipliers with an empty master, as E1n's `node` mode and the pilot's audit did, not from the root's columns and
inequalities.  (3) The intersection is no longer checked against a reference algorithm while running; a test does it
against the crate's bound.  (4) The bound's time includes its Tier 1 evaluation and excludes the node's frame
matroids as vectors (`build_s`, charged to each bound by `analyze.py`); a bound that ran out of its 30 s may overrun by
the exact check or the separation in progress.  (5) A path node's `full` gets 2 s at most: it cannot cut, only its
validity is checked.

Commits: e8af0ea (the code), 296f650 (`analyze.py`, `cloud.sh`, the README) and the commit of this entry.
