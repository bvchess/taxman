# The model-free proofs of 1301-2175: cost and the cloud run (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-15.

**Question.**  The committed chain reaches 2175, and the prover's model-free proofs ([prover
3](../../noframe/docs/proofs.md)) stopped at 1300.  What does `exact` cost on 1301..2175, can it use the many cores of a
cloud machine, and how should it be run?  **Answer.**  One game per thread is enough.  The 490 games that need a search
took 309.2 million nodes and 226.0 thread-hours, 15.9 hours and $3.84 on one `c8g.4xlarge` on spot, and opt(n) is proven
equal to the recorded score for every n in 2..2175.  The projection of 140 thread-hours was low, because the prover's
nodes grow with the solver's by a ratio of about 2.5 at 1301-1499 that triples per 1000 in n.

## Method

**How `exact` uses threads** (from the code, then measured).  `campaign` (`solver/noframe/src/jsonl.rs`) gives the games
of a range to its workers through an atomic counter; each worker runs one game's search alone (a `Prover` is one
thread), and a finished game is appended and synced as one line.  `--seconds` is a pause deadline for every search in
flight and the end of new work; the paused searches write `exact_N.ckpt` (target, nodes, seconds, nogoods, open units;
temp file, `fsync`, rename) and a rerun resumes them.  Checkpoints are written only at the pause and there is no signal
handler, so a SIGTERM or a kill loses the pass in progress.

**The projection.**  The measurements ran on the M1 Max (10 cores, other jobs running throughout), with `exact`
unchanged.  The cost of a game follows the nodes of its search, and two predictors were tried:

* *The root's parity gap.*  Over 900-1300 a proof's nodes grew tenfold per 84 points of gap (R² 0.76); over 1301-1360
  the 18 games finished give tenfold per 130.  The gaps of 1301..2175 run 125 to 767 (mean 380 over the 490 games that
  need a search), and that slope predicts 90 to 900 thread-hours for one game of gap 767.  But a game of gap 364 (1576)
  took 18 thousand nodes, one of gap 401 (1501) 63 thousand, and one of 315 (1326) 301 thousand.
* *The solver's own nodes*, which the committed chain records for every game (for a repeated search, the source game's).
  Over the 34 games `exact` had finished, its nodes over those are 0.13 to 13:

| n | 1302-1364 (19 games) | 1497-1612 (6) | 1655-1923 (6) | 1975-2103 (3) |
|---|---|---|---|---|
| geometric mean of the ratio (range) | 0.9 (0.13-2.1) | 1.6 (0.66-3.4) | 2.4 (0.75-8.4) | 7.0 (4.7-13) |

The fit over the 34 is log10 ratio = −0.07 + 1.02 (n − 1300)/1000, residual sd 0.28.  Five dear games were stopped
unfinished after 1 to 1.6 hours of a thread each: 1430 (2.19 million nodes so far, the solver's search 1.61 million),
1472 (1.83, 1.35), 1475 (1.73, 1.92), 2052 (1.15, 0.21) and 2170 (1.25, 1.25).  The first estimate, from the 20 games of
1302-1576 alone (ratio 0.95, 61 thread-hours), was low by 2.3 times.

With that fit and the order `summarize.py order` (the solver's nodes weighted by 10^((n − 1300)/1000)), 490 of the 875
games need a search.  The committed proofs of 1..1300 cost 1.2 to 1.7 ms a node on two quiet threads (1.73 at
1200-1300); this sample, 2.2 to 2.5 ms of user time (8 threads in parallel add 12%).

| | nodes (M) | thread-hours | the dearest game |
|---|---|---|---|
| 2.0 ms a node | 207 | 115 | 9.2 h (4.6-22) |
| **2.5** | 205 | **142** (126-165 over the draws) | 10.9 h (5.7-24) |
| 3.0 | 206 | 171 | 13.4 h (7-31) |

By list scheduling over costs drawn from the model, at 2.5 ms a node (add 10% for restarts; spot prices of the cheapest
zone of us-east-1 on 2026-10-14):

| vCPUs | shape | wall | spot $/hour | spot total | on-demand total |
|---|---|---|---|---|---|
| 4 | `c8g.xlarge` | 36 to 57 h | 0.078 | $2.8 to 4.4 | $5.8 to 9.1 |
| 8 | `c8g.2xlarge` | 18 to 28 h | 0.158 | $2.8 to 4.4 | $5.7 to 8.9 |
| **16** | `c8g.4xlarge` | **11 to 17 h** | 0.2391 | **$2.6 to 4.1** | **$7.0 to 10.8** |
| 32 | `c8g.8xlarge` | 11 to 17 h | 0.4134 | $4.5 to 7.0 | $14 to 22 |
| 64 | `c8g.16xlarge` | 11 to 17 h | 0.6794 | $7.5 to 11.6 | $28 to 43 |
| 192 | `c8g.48xlarge` | 11 to 17 h | 1.9939 | $22 to 34 | $84 to 130 |

From 16 vCPUs up the dearest games (2170-2173, the model's 8 million nodes each) set the wall time, 11 to 17 hours, and
the idle CPUs are paid for.  The range is wider than the draws': the trend in n rests on 7 games above 1800 and a sample
that is not random, and the speed of a Graviton core was unmeasured.

**The run.**  `EXTRA_ARGS='--fix-low 0' solver/tools/cloud/prover.sh 1301 2175` with
`DEADLINE_HOURS=36 POWEROFF=always`, `S3_DEST=s3://taxman-013764204840-us-east-1/prover-1301-2175` and `TIMER_MINUTES`
2220, as the [prover's runbook](../../docs/runbook/prover.md#run) describes: `exact` in passes of 300 s over the range,
dearest first, until every game has a line, then the check `summarize.py check` (every n in 2..2175 proven, every score
the recorded one).  The machine was one `c8g.4xlarge` (Graviton4, 16 vCPUs, 31 GiB) on spot in us-east-1b at $0.2402 an
hour, launched 2026-10-14 23:22:58 UTC, the build of commit 4db7a14 with `-C target-cpu=native`.  The tools were tried
first on macOS and in the Linux image (arm64) on 1301-1320: SIGTERM (status 143), a resume, `--verify` (passing, and
failing on a missing game), and a second run refused.

## Results

**Scaling of `exact` with threads.**  The same 12 games (1302-1364, 275,105 nodes in all) at 1, 2, 4 and 8 threads,
longest first:

| threads | wall s | user s | speedup | user ms a node | longest game s |
|---|---|---|---|---|---|
| 1 | 606 | 601 | 1.00 | 2.19 | 141 |
| 2 | 312 | 615 | 1.94 | 2.23 | 140 |
| 4 | 171 | 637 | 3.53 | 2.31 | 147 |
| 8 | 151 | 675 | 4.00 | 2.46 | 151 |

The node counts are identical at every thread count (no sharing between games), the user time per node rises 12% at 8
threads on a busy machine, and the 8-thread wall time is the longest game's, not the threads'.  Pausing does not change
a tree: the same games stopped every 20-25 s (and by SIGTERM) and resumed took exactly the node counts of an
uninterrupted run (1316: 12,567; 1320: 62,784; and the rest of the 13 searches of 1301-1320).

**The run (2026-10-14 and 15).**  The run began at 23:28:46 and ended "complete" at 15:21:58 on the 15th: 15.9 hours,
190 passes of 300 s, every one ended with status 0, with no spot interruption, no crash, no error.  The instance shut
itself down at the end and was terminated: 16.0 instance-hours, **$3.84**.  The check printed
`OK: opt(n) is proven equal to the recorded score for every n in 2..2175`, on the instance and again at home: every
score of the committed chain is the optimum on the whole board, and no search found a set that beat one.  With the
1..1300 proofs, `summarize.py exact` counts 2,174 games proven: 45 by the Franklín–Moniot bound and 199 by the parity
bound at the root, 1,047 by a search (557 in 2..1300, 490 in 1301..2175), 607 at their ceiling and 276 by the prime
formula.  The 490 searches all ran to the end, took 309.2 million nodes and **226.0 thread-hours**, 2.63 ms a node; the
385 other games took 83 seconds in all.

| | projected | the run |
|---|---|---|
| nodes, the 490 searches | about 200 million (150 to 300) | 309.2 million |
| thread-hours | about 140 (100 to 250), at 2.5 ms a node | 226.0, at 2.63 ms a node |
| a node | 2.5 ms throughout | 2.0 to 2.6 ms through 1899, 2.8 at 1900-1999, 3.1 at 2000-2099, 3.9 at 2100-2175; 4.10 to 4.15 at 2170-2173 |
| 2170-2173 | 8 million nodes each, 6 hours at 2.5 ms a node | 6.79 to 6.87 million nodes, 7.7 to 7.9 hours each |
| 1475, 1430, 1472, 1521 | 1.7 to 2.5 million nodes, 1.2 to 1.7 hours | 6.33, 2.88, 3.85 and 0.89 million nodes; 4.9, 1.9, 3.0 and 0.6 hours |
| 16 vCPUs, wall time | 11 to 17 hours | 15.9 hours |
| spot price | $2.6 to 4.1 | $3.84 |

The wall time and the price are inside the projected ranges, and so are the thread-hours; the nodes are just above (1.5
times the central figure, 1.6 times for the hours).  The dearest four took 0.85 of the projected nodes, at 1.6 times the
cost of a node it assumed; the next ones were dearer than projected: 1848-1851 took 5.33, 5.33, 4.51 and 4.50 million
nodes, 3.0 to 3.6 hours each, 2060 and 2064 3.4 and 4.0 hours, 1473 3.1.  The dear tail is wide: 77 searches took a
million nodes or more, 57 an hour or more, the ten dearest 24% of the thread-hours and the fifty dearest 59%.

**What the model got wrong.**

* *The ratio to the solver's nodes is higher and grows threefold per 1000 in n, not tenfold.*  Over the 457 searched
  games for which the committed chain records nodes, `exact` took 0.13 to 28 times the solver's nodes: geometric mean
  2.5 at 1301-1499 (112 games), 3.3 at 1500-1799 (156), 4.0 at 1800-2059 (129), 6.1 at 2060-2169 (56) and 5.3 at
  2170-2175 (4).  The fit is log10 ratio = 0.33 + 0.50 (n − 1300)/1000, residual sd 0.35 decades.  The 34 finished games
  the projection rested on were not a random sample (0.9 at 1300-1400 against 2.5 for all the games of 1301-1499), since
  a busy laptop finishes the easy games first.  The old fit would have given these 457 games 170 million nodes, and they
  took 287 million.  Carried to 3000, the old trend priced the prover's nodes at about 60 times the solver's ([agenda
  2](../../docs/AGENDA.md#2-the-diagnosis-of-the-wall)), this one at about 15: an extrapolation of 800 in n.
* *The model priced 33 games at zero, and `exact` took 22.1 million nodes and 13.9 thread-hours on them* (15 thousand to
  4.2 million nodes each).  The solver's chain records no nodes for these: each is a repeat (`same_search`) of a game
  whose search ended before its first node, at the ceiling.  The prover has no such bound, and `summarize.py order`
  ranks a game by the solver's nodes, so these went after the 457 priced ones.  1837 and 1839 (4.17 million nodes, 2.6
  hours each, the same tree) started only in the 159th pass, at 12:42 UTC, and ended the run.
* *The root's parity gap does predict the cost, with a flatter slope than over 900-1300.*  The 490 searches fit log10
  nodes = a + 0.477 (0.017) per 100 points of gap (tenfold per 210; R² 0.61, residual sd 0.45 decades), against tenfold
  per 84 over 900-1300; the projection from the gap used the old slope.  The gap (mean 384, 125 to 767; 2170-2173 at
  767, 1475 at 590, 1848-1851 at 543) and the solver's nodes are both guides to which games are dear.  Against n there
  is no growth that a fit resolves (−0.009 ± 0.013 decades per 100 in n for the nodes, R² 0.00), as for the solver's own
  search.

**The tail.**  All 16 threads were busy until 13:07 UTC; from then fewer than 16 games were left (13 at 13:07, 8 at
13:12, 5 at 13:47, 3 at 13:52, and from 14:12 to the end at 15:22 two, 1837 and 1839).  The last 2.2 hours left 27
thread-hours of CPUs idle, 11% of the machine's 254 in all (226 of search, so 89% busy; the restarts of 190 passes cost
under 1%).  Packing 226 thread-hours on 16 threads perfectly takes 14.1 hours, 1.8 fewer (about $0.42): the most that a
better order could have saved.

**Not measured.**  Where a proof's tree branches (members above n/2 against below, the premise of [agenda
A2](../../docs/AGENDA.md#a2-certify-the-frame-model-then-prove-with-its-facts-a-change-of-slope-for-the-proven-wall)):
the lines hold totals only.  Graviton5.  A spot interruption, which never came in 15.9 hours.

## Decision

* **Kept.**  `prover.sh` as it ran: `exact` in passes of 300 s over the range, dearest first, on one `c8g.4xlarge` on
  spot.  A pass of 300 s bounds what a kill loses (the run's 190 passes cost under 1% and none was killed), so `exact`
  needs no clean pause on SIGTERM.  The order of `prover.sh` is the solver's nodes weighted by n, which kept 12 of the
  16 dearest games among the first 16.
* **Not kept: a larger machine.**  From 16 vCPUs up the dearest games (2170-2173) set the wall time, 11 to 17 hours;
  192 vCPUs cost eight times as much for the same hours.  What set the wall time of the run was the late start of
  1837 and 1839, not the CPUs: the four dearest games took 7.7 to 7.9 hours each.
* **Open, not needed: several threads on one game** (`--shard i/K` over a checkpoint's open units, which are independent
  subtrees).  The four dearest games took 7.7 to 7.9 hours, and the wall time of 15.9 hours was set by the late starts
  of 1837 and 1839.
* **Open: should `summarize.py order` price the games the solver settles with no search** (0 nodes in `strict.jsonl`)?
  The 33 games it priced at zero took 22.1 million nodes and 13.9 thread-hours, and two of them ended the run.
* **For the next estimate**: the ratio of the prover's nodes to the solver's, 2.5 at 1301-1499 and tripling per 1000 in
  n (fit above); a node of 2.0 to 2.6 ms through 1899 and up to 4.15 ms at 2170-2173 on Graviton4, against 2.5 ms
  throughout in the projection.

## Files

The folder holds this README.md only.  The run's results, 875 lines of 152 KB, are
`solver/noframe/data/exact_parity_1301_2175.jsonl` (made by `prover.sh 1301 2175` above); the logs and the checkpoints
are not kept.  The tools are `solver/tools/cloud/prover.sh` (with `run.sh`, which it shares with `chain.sh`: one run at
a time, log, pid, deadline, S3 saves, clean stop, power-off), `setup.sh`, `spot-watch.sh` and the Dockerfile.

## Commits

* 4db7a14 (2026-10-14): the build of the run, the merge of round A.
