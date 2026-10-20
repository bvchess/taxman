# The model-free proofs of 1301-2175: cost and the cloud run (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The committed chain now reaches 2175, and the model-free proofs of [prover
3](../../noframe/docs/proofs.md) stopped at 1300 (the entry above, when this one began, said that the check
of 1301..2175 was pending).
What does it cost, can `exact` use the many cores of a cloud machine, and how should it be run?  Measured on the M1 Max
(10 cores, other jobs running throughout), with `exact` as it is: no proving code was changed.

*How `exact` uses threads (from the code, then measured).*  One game per thread: `campaign` (`evidence/src/jsonl.rs`)
gives the games of a range to its workers through an atomic counter, each worker runs one game's search alone (a
`Prover` is one thread), and a finished game is appended and synced as one line.  `--seconds` is a pause deadline for
every search in flight and the end of new work; the paused searches write `exact_N.ckpt` (target, nodes, seconds,
nogoods, open units; temp file, `fsync`, rename) and a rerun resumes them.  Checkpoints are written only at the pause
and there is no signal handler, so a SIGTERM or a kill loses the pass in progress.  The same 12 games (1302-1364,
275,105 nodes in all) at 1, 2, 4 and 8 threads, longest first:

| threads | wall s | user s | speedup | user ms a node | longest game s |
|---|---|---|---|---|---|
| 1 | 606 | 601 | 1.00 | 2.19 | 141 |
| 2 | 312 | 615 | 1.94 | 2.23 | 140 |
| 4 | 171 | 637 | 3.53 | 2.31 | 147 |
| 8 | 151 | 675 | 4.00 | 2.46 | 151 |

The node counts are identical at every thread count (no sharing between games); the user time per node rises 12 % at 8
threads on a busy machine; the 8-thread wall time is the longest game's, not the threads'.  Pausing does not change a
tree: the same games stopped every 20-25 s (and by SIGTERM) and resumed took exactly the node counts of an uninterrupted
run (1316: 12,567; 1320: 62,784; and the rest of the 13 searches of 1301-1320, in `evidence.sh` runs on macOS and in the
Linux image).

*The cost of the proofs of 1301..2175.*  (a) *The root's parity gap does not predict it.*  Over 900-1300 a proof's
nodes grew tenfold per 84 points of gap (R² 0.76); over 1301-1360 the 18 games finished give tenfold per 130 (the
solver's slope).  The gaps of 1301..2175 run 125 to 767 (mean 380 over the 490 games that need a search; 2170-2173 are
the widest), and that slope from gap 310 (270 thousand nodes) predicts 90 to 900 thread-hours for one game of gap 767
and 1,000 to 5,000 for the campaign.  But a game of gap 364 (1576) took 18 thousand nodes and one of gap 401 (1501)
63 thousand; one of 315 (1326) took 301 thousand.  (b) *The solver's own node counts do.*  The committed chain records,
for every game, the nodes of the search that settled it (`nodes`; for a repeated search, the source game's).  Over
the 34 games finished, `exact`'s nodes over those are 0.13 to 13:

| n | 1302-1364 (19 games) | 1497-1612 (6) | 1655-1923 (6) | 1975-2103 (3) |
|---|---|---|---|---|
| geometric mean of the ratio (range) | 0.9 (0.13-2.1) | 1.6 (0.66-3.4) | 2.4 (0.75-8.4) | 7.0 (4.7-13) |

(the fit over all 34: log10 ratio = -0.07 + 1.02 (n - 1300)/1000, residual sd 0.28; the ratio of the dearest games run
to the end are 1521: 0.66, 1612: 1.7, 1803: 3.1, 1805: 2.1, 1975: 5.4.)  Five dear games were stopped unfinished:
1430 (2.19 million nodes so far, the solver's search 1.61 million), 1472 (1.83, 1.35), 1475 (1.73, 1.92), 2052
(1.15, 0.21) and 2170 (1.25, 1.25), after 1 to 1.6 hours each of a thread on the busy machine.  The first estimate,
from the 20 games of 1302-1576 alone (ratio 0.95, 61 thread-hours), was low by 2.3 times: the ratio grows with n.  490
of the 875 games need a search; with the fit (and the order `summarize.py order`, which weights the solver's nodes by
10^((n - 1300)/1000)) the model gives, at 2.5 ms a node (the committed proofs of 1..1300 cost 1.2 to 1.7 ms on two quiet
threads, 1.73 at 1200-1300; this sample 2.2 to 2.5 user time; no clean measure above 1500):

| | nodes (M) | thread-hours | the dearest game |
|---|---|---|---|
| 2.0 ms a node | 207 | 115 | 9.2 h (4.6-22) |
| **2.5** | 205 | **142** (126-165 over the draws) | 10.9 h (5.7-24) |
| 3.0 | 206 | 171 | 13.4 h (7-31) |

By list scheduling the wall time on 4, 8, 16, 32 and more threads is 36, 18, 11 and 11 hours at 2.5 ms a node; from 16
threads up the dearest games (2170-2173, the model's 8 million nodes each) set it.  The range of the estimate is wider
than the draws': the trend in n rests on 7 games above 1800 and a sample, not a random one, and the speed of a Graviton
core is unmeasured.  The RUNBOOK ([section 9](#later-2026-10-the-runbooks-estimate-of-the-run))
gives the wall time and the price by size (about $3 to $4 on spot for 16 vCPUs) and says that the calibration on
Graviton3, 4 and 5 (`rust/bench/cloud/`) refines the per-core figures.

*Tooling.*  `solver/tools/cloud/evidence.sh A B` runs `exact` in passes of 300 s over the range, dearest first,
until every game has a line, then the check `summarize.py check A B` (every n in 2..B proven, every score the recorded
one); `run.sh` holds what it shares with `chain.sh` (one run at a time, log, pid, deadline, S3 saves, clean stop,
power-off), `setup.sh` builds and checks the evidence crate (`EVIDENCE_ONLY=1` skips the solver), `spot-watch.sh` stops
either run, the Dockerfile builds the crate.  Tried on macOS and in the Linux image (arm64): 1301-1320, SIGTERM
(status 143), a resume, `--verify` (passing, and failing on a missing game), a second run refused, `chain.sh` after the
split (a complete chain to 120, a SIGTERM and a deadline).  The Docker build had to start from the image of an earlier
build (`--build-arg RUST_IMAGE=taxman-cloud`): the registry's metadata did not answer.

*Not done, and why.*  No change to `exact` to use many cores on one game (its open units are independent subtrees, so a
`--shard i/K` over a checkpoint's units would do it), nor to pause on a signal: at 16 vCPUs the campaign ends when the
dearest game does, and a pass of 300 s bounds what a kill loses.  The run on EC2 and the check of 1301..2175 were the
owner's, and are below; the results are in `solver/noframe/data/exact_parity_1301_2175.jsonl`.

*The run (2026-10-14 and 15).*  The plan above ran, the check passed, and the cost came out higher than the projection's
central figure, inside its ranges.

*What ran.*  `solver/tools/cloud/prover.sh 1301 2175` as [RUNBOOK 9.4](../../docs/runbook/prover.md#run) describes, on one
`c8g.4xlarge` (Graviton4, 16 vCPUs, 31 GiB) on spot in us-east-1b at $0.2402 an hour, launched 2026-10-14 23:22:58 UTC
(the run began at 23:28:46 and ended "complete" at 15:21:58 on the 15th: 15.9 hours), the build of commit 4db7a14 with
`-C target-cpu=native`.  The instance shut itself down at the end (`POWEROFF=always`) and was terminated: 16.0
instance-hours, **$3.84**.  190 passes of 300 s, every one ended with status 0: no spot interruption, no crash, no
error.  The results, 875 lines of 152 KB, are `solver/noframe/data/exact_parity_1301_2175.jsonl`; the logs and the checkpoints
are not kept.

*The check.*  `OK: opt(n) is proven equal to the recorded score for every n in 2..2175`, printed on the instance and
again at home (`prover.sh --verify 1301 2175`, which is `summarize.py check`): every score of the committed chain is the
optimum on the whole board, and no search found a set that beat one.  With the 1..1300 proofs of
`solver/noframe/data/exact_parity_2_1300.jsonl`, `summarize.py exact` counts 2,174 games proven: 45 by the Franklín–Moniot bound
and 199 by the parity bound at the root, 1,047 by a search (557 and 490), 607 at their ceiling and 276 by the prime
formula (337 and 270, 161 and 115).  Of the 875 games 490 needed a search, and all of them ran to the end.  They took
309.2 million nodes and **226.0 thread-hours**, 2.63 ms a node; the 385 others took 83 seconds in all.
[The prover's README](../../noframe/docs/proofs.md#to-2175-on-a-cloud-machine) has the table by band (`summarize.py cost --band
100`).

*Against the projection above.*

| | projected | the run |
|---|---|---|
| nodes, the 490 searches | about 200 million (150 to 300) | 309.2 million |
| thread-hours | about 140 (100 to 250), at 2.5 ms a node | 226.0, at 2.63 ms a node |
| a node | 2.5 ms throughout | 2.0 to 2.6 ms through 1899, 2.8 at 1900-1999, 3.1 at 2000-2099, 3.9 at 2100-2175; 4.10 to 4.15 at 2170-2173 |
| 2170-2173 | 8 million nodes each, 6 hours at 2.5 ms a node | 6.79 to 6.87 million nodes, 7.7 to 7.9 hours each |
| 1475, 1430, 1472, 1521 | 1.7 to 2.5 million nodes, 1.2 to 1.7 hours | 6.33, 2.88, 3.85 and 0.89 million nodes; 4.9, 1.9, 3.0 and 0.6 hours |
| 16 vCPUs, wall time | 11 to 17 hours | 15.9 hours |
| spot price | $2.6 to 4.1 | $3.84 |

The wall time and the price are inside the projected ranges, the thread-hours too, the nodes just above (1.5 times the
central figure, 1.6 times for the hours).  The dearest four took 0.85 of the projected nodes, at 1.6 times the cost of
a node it assumed; the next ones were dearer than projected: 1848-1851 took 5.33, 5.33, 4.51 and 4.50 million nodes, 3.0
to 3.6 hours each, 2060 and 2064 3.4 and 4.0 hours, 1473 3.1; of the projection's list, 2160 and 1863 took 0.2 and 0.6
hours.  Its order put 12 of the 16 dearest games among its first 16.  The dear tail is wide: 77 searches took a million
nodes or more, 57 an hour or more, the ten dearest 24% of the thread-hours and the fifty dearest 59%.

*What the model got wrong.*

* **The ratio to the solver's nodes grows threefold per 1000 in n, not tenfold.**  Over the 457 searched games for
  which the committed chain records nodes (the next bullet is the other 33), `exact` took 0.13 to 28 times the solver's
  nodes: geometric mean 2.5 at 1301-1499 (112 games), 3.3 at 1500-1799 (156), 4.0 at 1800-2059 (129), 6.1 at 2060-2169
  (56) and 5.3 at 2170-2175 (4).  The fit is log10 ratio = 0.33 + 0.50 (n - 1300)/1000, residual sd 0.35 decades; the 34
  finished games on which the projection rested gave -0.07 + 1.02, sd 0.28 (0.9 at 1300-1400, against 2.5 for all the
  games of 1301-1499; a sample of the games a busy laptop finished is not a random one).  The old fit would have given
  these 457 games 170 million nodes, and they took 287 million: by the geometric mean of the games 2.4 times as many at
  1301-1499, 1.7 at 1500-1799, as many at 1800-2169 (1.05 and 1.06) and 0.8 times at 2170-2175.  Carried to 3000, the
  old trend priced the prover's nodes at about 60 times the solver's ([agenda
  2](../../docs/AGENDA.md#2-the-diagnosis-of-the-wall)), this one at about 15: an extrapolation of 800 in n.
* **The model priced 33 games at zero, and `exact` took 22.1 million nodes and 13.9 thread-hours on them** (15 thousand
  to 4.2 million nodes each).  The solver's chain records no nodes for these: each is a repeat (`same_search`, through
  one to five other repeats) of a game whose search ended before its first node, at the ceiling (the proof `cap`).  The
  prover has no such bound, and `summarize.py order` ranks a game by the solver's nodes, so these went after the 457
  priced ones and before the 385 that `exact` skips (at their ceiling or prime).  1837 and 1839 (4.17 million nodes,
  2.6 hours each, the same tree) started only in the 159th pass, at 12:42 UTC, and ended the run.
* **The root's parity gap does predict the cost, with a flatter slope than over 900-1300.**  The 490 searches fit
  log10 nodes = a + 0.477 (0.017) per 100 points of gap (tenfold per 210; R² 0.61, residual sd 0.45 decades, a factor
  of 2.8), against tenfold per 84 over 900-1300 (R² 0.76): the projection from the gap used the old slope, and was
  wrong by that.  The gap (mean 384, 125 to 767; 2170-2173 at 767, 1475 at 590, 1848-1851 at 543) and the solver's
  nodes are both guides to which games are dear.  Against n there is no growth that a fit resolves (-0.009 +- 0.013
  decades per 100 in n for the nodes, +0.007 +- 0.013 for the seconds, R² 0.00), as for the solver's own search of the
  chain.

*The tail.*  All 16 threads were busy until 13:07 UTC; from then fewer than 16 games were left (13 at 13:07, 8 at 13:12,
5 at 13:47, 3 at 13:52, and from 14:12 to the end at 15:22 two, 1837 and 1839).  The last 2.2 hours left 27 thread-hours
of CPUs idle, 11% of the machine's 254 in all (226 of search, so 89% busy; the restarts of 190 passes cost under 1%: the
games in flight times the passes' length come to 227 thread-hours).  Packing 226 thread-hours on 16 threads perfectly
takes 14.1 hours, 1.8 fewer (about $0.42): the most that a better order could have saved.

*Not measured.*  Where a proof's tree branches (members above n/2 against below, the premise of [agenda
A2](../../docs/AGENDA.md#a2-certify-the-frame-model-then-prove-with-its-facts-a-change-of-slope-for-the-proven-wall)): the lines
hold totals only.  Graviton5.  A spot interruption, which never came in 15.9 hours.

## Later (2026-10): the RUNBOOK's estimate of the run

RUNBOOK.md's section 8.2, "What it costs", held this estimate of the run until 2026-10-19, when it moved here
unchanged; the RUNBOOK keeps how to price a run.  Its "section 8.4" and "section 8.2" are the RUNBOOK's sections of
that date, and its link named this entry.

The cost of a game follows the nodes of its search.  The solver's own search of the same game, which the committed chain
records, predicts it (the root's parity gap, which predicted the cost over 900-1300, does not here: a game of gap 364
took 18 thousand nodes and one of gap 315 300 thousand).  Over 34 games of 1302-2124 that `exact` finished, it took 0.13
to 13 times the nodes of the solver's search (of the source game's, for a game whose search the solver repeated), the
ratio growing about tenfold per 1000 in n: geometric mean 0.97 at 1300-1500, 1.6 at 1500-1800, 3.8 at 1800-2124, with a
scatter of 0.28 decades; `exact` has the weaker bound, the parity one.  Five dear games were stopped unfinished at 1.1
to 2.2 million nodes (1430, 1472, 1475, 2052, 2170).  The committed proofs of 1..1300 cost 1.2 to 1.7 ms a node on two
quiet laptop threads (1.73 at 1200-1300); the sample at 1300-1500 on a busy machine, 2.2 to 2.5 (user time; 8 threads in
parallel add 12 %).  Past 1500 the node is not flat in n: the run on Graviton4 (16 threads of a `c8g.4xlarge`) costs
about 2.2 ms a node at 1430-1863, 3.1 to 3.3 at 2060-2160 and 3.9 to 4.2 at 2170-2173, against 2.44 for the searches of
1301-1310 in the rehearsal (section 8.4).  The model below takes 2.5 ms throughout, so it is low for the late games, the
dear ones above all.  490 of the 875 games need a search (the rest are decided by the ceiling or the prime formula).
With the fitted ratio and 2.5 ms a node:

| | |
|---|---|
| nodes, all 490 searches | about 200 million (150 to 300) |
| thread-hours at the laptop's speed | **about 140** (100 to 250; 115 at 2.0 ms a node, 170 at 3.0) |
| the dearest games | 2170-2173 (gap 767; the model says 8 million nodes each: 6 hours at 2.5 ms a node, about 9 at Graviton4's 3.9 to 4.2; 2170 had done 1.25 million nodes after 90 minutes on the laptop), then 1475, 1430, 1472, 1521 (1.7 to 2.5 million, 1.2 to 1.7 hours at 2.5 ms), 1849-1863, 2160 |

Wall time and price by size, by list scheduling over costs drawn from that model (the order is `summarize.py order`),
at the laptop's speed and at 1.6 times as long (the factor of a Graviton CPU against an M1 Max CPU, which the
calibration has measured since: a Graviton4 core takes 1.08 times as long, so the lower end of each range is the nearer,
before the rise of the node's cost with n above).  Add 10 % for restarts.  Spot prices are the cheapest zone of
us-east-1 in the owner's account's spot price history on 2026-10-14 (4 and 8 vCPUs: Vantage, the same day); on demand is
Vantage's Linux price in US East (`c8g`: $0.0399 per vCPU-hour at every size):

| vCPUs | shape | wall | spot $/hour | spot total | on-demand total |
|---|---|---|---|---|---|
| 4 | `c8g.xlarge` | 36 to 57 h | 0.078 | $2.8 to 4.4 | $5.8 to 9.1 |
| 8 | `c8g.2xlarge` | 18 to 28 h | 0.158 | $2.8 to 4.4 | $5.7 to 8.9 |
| **16** | `c8g.4xlarge` | **11 to 17 h** | 0.2391 | **$2.6 to 4.1** | **$7.0 to 10.8** |
| 32 | `c8g.8xlarge` | 11 to 17 h | 0.4134 | $4.5 to 7.0 | $14 to 22 |
| 64 | `c8g.16xlarge` | 11 to 17 h | 0.6794 | $7.5 to 11.6 | $28 to 43 |
| 192 | `c8g.48xlarge` | 11 to 17 h | 1.9939 | $22 to 34 | $84 to 130 |

From 16 vCPUs up the dearest games set the wall time, not the CPUs: 2170-2173 start at once and the longest of them
decides (6 to 24 hours over the draws, at the laptop's speed; about 9 hours at Graviton4's cost per node for the
model's 8 million nodes), and the idle CPUs are paid for.  The run's own totals replace this table: 15.9 hours and $3.84
on 16 vCPUs, inside the row's ranges; 2170-2173 took 7.7 to 7.9 hours each, and what set the wall time was the packing
of the late starts, not those games
([EXPERIMENTS](#the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10)).

## Later (2026-10-20): the run's launch, as the runbook gave it

The runbook's page on the prover took this run as its example until the page was made general.  The values it gave:

* the command, from the plain root: `EXTRA_ARGS='--fix-low 0' solver/tools/cloud/prover.sh 1301 2175`, with
  `DEADLINE_HOURS=36 POWEROFF=always` and `S3_DEST=s3://taxman-013764204840-us-east-1/prover-1301-2175`;
* the timer, `TIMER_MINUTES` 2220;
* the results file, `solver/noframe/data/exact_parity_1301_2175.jsonl` (152 KB), checked with the default `BASELINE`,
  the proofs of 2..1300;
* the first passes on the `c8g.4xlarge`, 2.2 to 3.3 ms a node from the plain root, more for the late games; the
  rehearsal's ten games, 2.44 ms a node, and about 3 minutes of one thread on a laptop.

