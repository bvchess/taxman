# The A1 bound at the nodes of the node-choice measurement (2026-10)

Part of the [lab notebook](../../README.md).

*Question.*  Does the multi-field (A1) bound, fitted fully at a node, cut the nodes that head big subtrees?  The
[E3 pilot](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md) measured A1 in a branch and bound (trees five
times smaller at 23 times the cost of a node, a constant factor and no change of slope, and [U4](../r3-and-the-slope-test-u4-of-a1-2026-10/README.md)
found the factor falling with the gap); [the node-choice measurement](../the-node-choice-where-the-strong-bound-pays-2026-10/README.md) asks per
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
port reproduces the study: `the_study_fits_give_their_bounds_again` evaluates the saved fits of `solver/frame/testdata/fits` at
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
solver/frame/testdata/fits`, `choice --a1 --a1-seconds 10 --cap 60`; 936 and 1215, 30 open nodes each of the probe's fixtures; the
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

*The cloud run* (`A1=1 solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/cloud.sh`; [the README](../the-node-choice-where-the-strong-bound-pays-2026-10/README.md#the-a1-bounds) has the
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

## Later (2026-10-18): the run on EC2

*Answer.*  No.  The full A1 bound is far stronger than the nogood bound at moderate gaps, but the nodes it cuts head
small subtrees that search in milliseconds, and at the nodes that head the big subtrees (gap 500 and up) it cuts
nothing.  Computing it only where the forecast subtree is big does not pay either.  This closes the lead of a selective
A1 in the search.

*The run.*  `A1=1 KEEP=100 WORKERS=64 solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/cloud.sh` at commit 260e74f (no solver change), on one spot
`c8g.16xlarge`, launched 19:46:25Z; the release suite passed there (752 tests, 306 s).  The fixtures began at 20:02:32Z,
the measurement ran 20:10:02Z to 20:16:27Z and the machine ended at about 20:18Z: 32 minutes at $0.676 an hour, about
**$0.36** (the estimate was $0.40).  840 open nodes were measured, all solved to the end, none capped: the suite's
3300, 3425, 3440 and 3441 (10 each, gaps 746-1001, subtrees of 33,000 to 1.25 million steps), the made games 2700-3267
and the probes 936 and 1215 (100 each, gaps 0-942), and 36 path nodes (none had a bound below its best completion; no
faults).  The workers used 10,606 CPU s: the solves 4,463, `full` 5,770, the nogood fits 253, `r2` 55.  The A1 root
fits close 0.33-0.34 of the gap at 3441, 3440 and 3425 (300 s each), 0.40 at 3300 (on file) and 0.45-0.64 at the
others.

Share of the open nodes cut, by gap (the nogood bound is the entry's "strong", 200 steps; `r2` the warm refit from the
root; `full` the node's own master within 30 s):

| gap | nodes | strong | A1 r2 | A1 full | median subtree |
|---|---|---|---|---|---|
| 0-49 | 199 | 91% | 90% | 100% | 2 |
| 50-99 | 156 | 71% | 61% | 99% | 5 |
| 100-199 | 208 | 22% | 24% | 90% | 20 |
| 200-299 | 124 | 3% | 3% | 77% | 70 |
| 300-399 | 69 | 0% | 1% | 65% | 175 |
| 400-499 | 31 | 0% | 0% | 16% | 706 |
| 500+ | 53 | 0% | 0% | 0% | 1,167 to 676,043 |

By the decade of the subtree, `full` cuts 100% of the 368 nodes at 1e0 steps, 96% of the 262 at 1e1, 46% of the 144 at
1e2, and none of the 66 at 1e3 and above (the nogood bound: 78%, 21%, 0%, 0%).  At gap 700 and up (46 nodes) `full` is
capped at 30 s in every node and closes a median 42% of the excess (`r2`: 29%); at 400-499 it is capped in 84% and
closes 89%.  The bound is right to the end of its tether and the tether is the cost: a node's `full` takes a median
1.2 s at gap 100-199, 6 s at 200-299 and 16 s at 300-399, against 0.1 to 0.3 s for the nogood fit.

*The policy* (compute the bound only where the forecast subtree exceeds N) loses for both A1 forms, in all four groups
(all, made, probe, suite) and in all three analyses (all the steps, the floor entry's slope, the first 50 steps), at
every N.  Over the 840 nodes, where searching them all costs 4,460 CPU s, `full` computed everywhere costs 5,770 CPU s
more than it saves (cutting 686 nodes); `r2` costs 56 more than it saves.  By group, `full` loses 4,330 CPU s on the
made nodes (cutting 500 of 600, whose searches cost 374 s in all), 233 s on the probes (cutting 186 of 200, searches
0.9 s) and 1,200 s on the suite, where it cuts none of the 40 and is capped in all.  The nogood bound loses too (126
CPU s over the 840), as it did in [the first run](../the-node-choice-where-the-strong-bound-pays-2026-10/README.md).

*Verdict.*  The A1 bound cuts the nodes that are cheap to search and none that are dear, so a selective A1 in the
search has nothing to select.  Lead 1 is closed.  Lead 2,
[A5](../../../docs/AGENDA.md#a5-decompose-across-the-frames-andor-search), is next.  The data is in
[cloud-20261018-a1/](cloud-20261018-a1/), with the
[section on it](#the-a1-cloud-run-2026-10-18) below; `analyze.py` of [the node-choice entry](../the-node-choice-where-the-strong-bound-pays-2026-10/README.md) on the gunzipped lines
gives its three analyses again, byte for byte.

Commits: 260e74f (the code measured), d5b3415 (the data) and the commit of this section.

## Files

**The run** is the same script with `A1=1`, sized for one `c8g.16xlarge` (64 CPUs, `WORKERS` 64) and an hour at the
most (user data as above with `shutdown -h +60` and `S3_DEST`, and `A1=1` in the environment of `cloud.sh`; `KEEP` 100,
the root fits 300 s):

1. the fixtures, as before (the six proofs in parallel; 2 to 4 minutes);
2. the preparation, a process a game, the twelve at once (the pool 20 CPU s, the A1 root fit up to `FIT_SECONDS` 300 for
   the nine games without a fit on file; about 6 minutes);
3. the measurement on 64 workers, the heaviest node first: about 1,000 nodes, the suite's and the made and probe games'
   (`KEEP` 100 of each source), their solves about 6,000 CPU s as before (the dearest node 314 s), the nogood fits about
   1,400, and the A1 bounds: `r2` about 0.1 to 1 s a node, `full` up to 30 s a node, which the nodes of gap 300 and
   more are expected to use (about 250 nodes), so some 10,000 CPU s; 8 to 12 minutes with the dearest node last;
4. `analyze.py` three ways, as before.

That is **about 30 to 40 minutes** of the machine with the build cache (10 minutes more with a build), **about 40 cents
at spot** ($0.68 an hour for a `c8g.16xlarge`, [STATUS](../../../docs/STATUS.md)), $1.50 on demand ($2.55 an hour, $0.0399 a vCPU-hour;
[RUNBOOK section 9.2](../../../docs/runbook/machine.md#quota-prices-and-spot-interruptions)); the timer of 60 minutes bounds it at **$0.68 at spot** ($2.55 on
demand) plus the volume.  The figures are estimates: the A1 bounds' times at 3300 to 3441 are not known (a pricing is 1.2
to 2.9 ms at 2200 to 3400, a node's master 100 to 1,000 of them), and a `full` that overruns its 30 s by a few seconds
in the exact checks makes the run longer, not dearer.

### The A1 cloud run (2026-10-18)

What it found: [the entry's section](../the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md#later-2026-10-18-the-run-on-ec2).
`A1=1 KEEP=100 WORKERS=64 solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/cloud.sh` at commit 260e74f (the other settings the script's: `NODE_STEPS`
200, `SUB` 1, `LIMIT` 900, `FIT_SECONDS` 300, `A1_PRICINGS` 20, `A1_SECONDS` 30) on one spot `c8g.16xlarge`, launched at
19:46:25Z; no build-cache entry, so the setup built the examples (5 minutes), built the tests and ran the release suite
(752 tests, 306 s).  The fixtures began at 20:02:32Z, the preparation at 20:04:34Z, the measurement ran 20:10:02Z to
20:16:27Z, and the machine ended at about 20:18Z: 32 minutes, about $0.36 at $0.676 an hour.  `faults` was empty.  Its
output is in `cloud-20261018-a1/`, the files of `cloud-20261018/` made the same way:

| file | what it is |
|---|---|
| `analysis.txt`, `analysis-floor.txt`, `analysis-steps50.txt` | the script's three analyses (all the steps; the floor entry's slope; the first 50 steps), with the A1 tables |
| `log` | the stages' times (UTC) and the commit |
| `prepare.jsonl` | the line each `wall choice-prepare` printed (suite, made, probe games in the script's order), now with the A1 root fit: `a1_source` (`file` for 3300, 2700 and 1215, `fitted` for the other nine), `a1_fit_s`, `a1_bound`, `a1_share` of the gap it closes (0.33-0.34 for 3441, 3440 and 3425 fitted in 300 s, 0.40 for 3300, 0.45-0.64 for the others), `a1_stop` (`Converged` at 3213, 3129, 2976 and 936, `Seconds` at 3441, 3440, 3425, 3260 and 3267) and `a1_lb` |
| `fixtures.txt` | the six proofs that made the fixtures: runs, nodes and distinct units (the numbers of `cloud-20261018/`, the proofs being the same) |
| `workers.jsonl` | a line for each worker and game, from the workers' output: the nodes it measured, skipped or left to other workers, and its CPU seconds (10,606 in all: 133 to 385 a worker) |
| `measure.jsonl.gz` | the 876 lines of `measure/worker-0.jsonl` to `worker-63.jsonl`, joined in the order of the workers and gzipped (1.3 MB): 840 open nodes (the 40 of the suite and 100 of each of the 8 other games, whose `source` is `fixture`) and the 36 path nodes.  `gunzip -c measure.jsonl.gz > measure.jsonl`, then `python3 -I solver/notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/analyze.py measure.jsonl [--model floor] [--steps 50]` makes the three analyses again, byte for byte (A1 needs no flag: `analyze.py` reads the `a1` of the lines) |

Not kept, as before: `prep/` (25 MB), `fixtures/` and `logs/` (the workers' output repeats `measure.jsonl.gz`'s lines
and `workers.jsonl`'s, which was checked line for line when the files were made).  The work was 10,606 CPU s, not the
17,000 or so of the estimate above: the solves 4,463, the nogood fits 253 (the two of `SUB`), `r2` 55 and `full` 5,770
(124 nodes ran out of their 30 s).
