# How the cost of solving Taxman grows with n (the curve near the wall)

Purpose: measure, on this machine, how the solver's cost grows around and beyond the end of the chain
(n = 1231), so that what a 192-CPU cloud machine could reach can be predicted.  Not an attempt to extend the
chain.

This is the curve of the matching path, the search before the GF(2) bound.  The search with the GF(2) bound (the
default since 2026-10) changes it: a chain from 1300 reached 2175 in 92 minutes on 2 threads, with no growth a fit
resolves; its curve and projections are in
[solver/notebook/the-gf2-bound-in-the-search-2026-10/README.md](../the-gf2-bound-in-the-search-2026-10/README.md) and its data in `solver/notebook/the-gf2-bound-in-the-search-2026-10/`.  The
first 2175 games of the committed `results/strict.jsonl` are a chain from n = 1 made with it, in 27 minutes on 8 threads
([results/README.md](../../../results/README.md)); the file reaches much further now.

* Machine: Apple M1 Max (8 performance + 2 efficiency CPUs), 8 search threads for every main run.
* Build: master at e710d16, `cargo build --release --examples`; binaries not rebuilt during the runs.
* Tactics: no unproven tactic (`reuse_prev=off`, `revise_freedoms=off`); everything else at the build's
  defaults (match bound, conflict-strong branching, `match_refine=1`, `fix_forced`, `nogoods`,
  `forget_nogoods` every 50,000 steps, `preclusions` off).  The ladder only searches and rejects solver tactics,
  so it gets `--tactic revise_freedoms=off` alone (`reuse_prev` does not exist there).  The unproven tactics
  (`reuse_prev`, `revise_freedoms`) and `preclusions` have since been removed, with the flags that switched them: the
  commands quoted below need no `--tactic` for them today, and the solver runs only proven or sound tactics.
* Everything here was produced by the scripts in this directory from the raw JSONL files in this directory.

## Files

| file | content |
|---|---|
| `common.py` | paths, constants (search limit, per-call budget), JSONL helpers, primes, background-load sampling |
| `run_chain.py` | step 1: `taxman solve 1001-1231 --use-prev` (8 threads), resumable; writes `chain.jsonl` |
| `run_games.py` | steps 2-4: one `ladder` process per game, resumable (one JSON line per finished game) |
| `analysis.py` | step 5: calibration, band statistics, fits, projections; prints markdown |
| `analysis_output.md` | the output of `./analysis.py` (all tables, including the ones left out below) |
| `chain.jsonl` | step 1, the chain 1001-1231 as `taxman solve --out` writes it (with the moves) |
| `calib.jsonl` | step 2, standalone ladder runs of the 197 non-primes in 1001-1231 |
| `bands.jsonl` | step 3, standalone runs beyond the end of the chain: bands at 1250, 1300, 1350 (3 games) |
| `scaling.jsonl` | step 4, thread scaling of the proof of 1225 |
| `recheck.jsonl` | four games repeated at the end, to see run-to-run and background-load noise |
| `load.jsonl` | what the machine was doing apart from us, sampled before and after each invocation |
| `slope_roots.jsonl`, `slope_proofs.jsonl`, `slope.py` | the later study of what drives the slope: `examples/slope.rs`'s root measurements of the non-prime games 880-1300 and of this study's games past 1300, its one-thread proofs (with margins), and the script that makes its tables, from `results/strict.jsonl` as of commit 2487671 (the 1..1300 chain of the older build, read from git; `--strict PATH` for another file) ([solver/notebook/what-drives-the-slope-2026-10/README.md](../what-drives-the-slope-2026-10/README.md)) |

Regenerate the tables with `./analysis.py > analysis_output.md` (and the slope study's with `./slope.py`).  The example
`slope`, which wrote the slope study's data, was removed from master (2026-10-17); it builds at commit 8753189 (`git
checkout 8753189`, then `cargo build --release --example slope` in `rust/`).

## Method

1. **Chain baseline.**  `taxman solve 1001-1231 --threads 8 --tactic reuse_prev=off --tactic revise_freedoms=off
   --checkpoint-dir ... --progress 0`: each game starts from the previous game (seed and the n + opt(n-1) ceiling), so
   a game's time is its *chained* cost.  Every score equals `results/strict.jsonl` (1001-1219, as it was then) and
   `results/default.jsonl` (1001-1231).  (`strict.jsonl` then ended at 1219, not 1231; the chain file written here is
   the chain through 1231 and is what `--known` used wherever a game past 1219 was needed.  `strict.jsonl` has
   since been regenerated as a chain 1-2175 (and extended since), and its games 1001-1231 are this file's, move for
   move but for 1215 and 1221-1224, which have at least two optimal sets; see `results/README.md`.)
2. **Calibration.**  `ladder --threads 8 --no-seed --no-cap --tactic revise_freedoms=off` on every non-prime in
   1001-1231, one process per game.  Each standalone result was checked against the chain's optimum (all 197
   agree).  Compared per game with step 1.
3. **Beyond the end of the chain.**  Bands = the first 8 non-primes from the band start, standalone, 8 threads.
4. **Thread scaling.**  The proof of 1225 (`--seed-opt --known chain.jsonl`) on 1, 2, 4 and 8 threads.
5. **Analysis.**  `analysis.py`.

**Where the runs departed from the plan** (each imposed by the environment or decided during the run):

* The search limit is **585 s, not 600 s**.  The runs were made in sessions limited to 10 minutes per command, a
  timed-out game must fit one command together with its setup, and runs are resumable only between games (the ladder has
  no checkpoint), so 585 s is the limit and a timeout means "at least 585 s".  No game in this study reached it.
* A game started late in an invocation gets a shortened limit (what is left of the invocation); if it runs out of that
  it is discarded and searched again with a whole limit in the next invocation.  Two such runs were discarded: 1230
  (114 s, calibration) and 1350 (477 s).  Only the second counts against step 3's time.
* The thread-scaling step was cut down to one game (1225, 1-8 threads, no 10); the band sampling stopped after three
  games of band 1350 and no further bands.  For 1215 the proof-only scaling from the engine work (65.0 s on 1 thread,
  28.3 s on 2, 17.7 s on 4, 11.4 s on 8) is cited, not re-measured here.  It is a different run from the one in
  [Benchmarks](README.md) (62.5, 24.6, 15.8 and 12.9 s on 1, 2, 4
  and 6 threads, an earlier build), and the two are not comparable thread for thread.
* The stopping rule of step 3 ("stop after the first band where at least half time out") was **not reached**: the
  last band had no timeouts, and its games take 510-540 s, just under the limit.  Step 3 used 62 minutes of the 3
  hours the study was given (3241 s of recorded runs, 477 s discarded).

## Results

### Chain baseline (step 1)

231 games in **318 s** on 8 threads (149 s for 1001-1217, which [the
benchmarks](README.md) give as 164 s for an earlier build, and 169
s for 1218-1231).  34 primes cost 0.43 s together, 70 non-primes reach the ceiling with their seed (0 nodes; 6.9 s in
all, i.e. setup), 117 small searches take 110 s, and 10 big ones (chain >= 5 s) take 200 s.  The slowest: 1230 (91.5 s,
29% of the whole chain), 1227 (19.7 s), 1220 (19.6 s), 1025 (18.0 s), 1225 (17.0 s).

### Calibration: standalone against chained (step 2)

| class (of the chained game) | games | chain s (sum) | standalone s (sum) | standalone/chain (sums) | median per-game ratio | min-max per-game | nodes standalone/chain |
|---|---|---|---|---|---|---|---|
| cap reached (0 nodes) | 70 | 6.9 | 146.8 | 21.4 | 3.43 | 0.78-191.6 | n/a (chain has 0) |
| small search (chain < 5 s) | 117 | 110.1 | 156.3 | 1.42 | 1.05 | 0.68-4.1 | 1.52 |
| big search (chain >= 5 s) | 10 | 200.5 | 370.1 | 1.85 | 1.28 | 0.96-2.3 | 1.84 |
| big searches, n >= 1150 | 8 | 177.4 | 342.6 | 1.93 | 1.42 | 0.98-2.3 | 1.92 |
| all 197 non-primes | 197 | 317.4 | 673.2 | 2.12 | 1.23 | 0.68-191.6 | 2.23 |

* For the games that matter (big searches), a standalone run costs **1.9x** the chained one (1.85-1.93 by sums,
  per game 1.0-2.3x; 2.2x for 1230, the biggest).  The extra work is the nodes (1.84-1.92x), not speed.
* Small searches: 1.4x.  Games the chain settles with 0 nodes cost 0.1 s chained but 2.1 s standalone on average
  (up to 24 s: 1228), ratio 21x by sums; in absolute terms they are 22% of the standalone total and 2% of the
  chain total.
* Overall the chain costs **0.47 of the standalone total** (0.45 over n >= 1150, 0.52 over the big searches); the
  projections use **0.45**, applied to the standalone cost of every non-prime.
* The ratio is not a constant of the games: where the optimum is found late the seed saves most of it.  In band
  1350 the final incumbent is found at 92-99% of the search (1350: 506 s of 538 s), so a standalone run is
  nearly all search for the optimum; a chain gets that for free only when n - 1 has the same optimum (1351 after
  1350) and not when n opens a new class of frames.  The ratio of 0.45 was measured at 1150-1231 and assumed beyond.

### Standalone cost per band (step 3), 8 threads, no seed, no ceiling, limit 585 s

Bands starting at 1001-1200 are the calibration runs, grouped the same way.  Times are search seconds.

| band start | games (n) | median s | p75 s | max s | mean s | timeouts | median nodes (M) | max nodes (M) | median share of time before the final incumbent |
|---|---|---|---|---|---|---|---|---|---|
| 1001 | 1001-1008 (8) | 1.4 | 1.5 | 1.9 | 1.5 | 0 | 1.0 | 1.3 | 81% |
| 1050 | 1050-1058 (8) | 0.4 | 0.5 | 0.7 | 0.4 | 0 | 0.2 | 0.3 | 57% |
| 1100 | 1100-1108 (8) | 0.1 | 0.2 | 0.2 | 0.1 | 0 | 0.0 | 0.1 | 55% |
| 1150 | 1150-1159 (8) | 0.1 | 0.1 | 0.1 | 0.1 | 0 | 0.0 | 0.0 | 0% |
| 1200 | 1200-1208 (8) | 5.0 | 5.9 | 6.2 | 4.6 | 0 | 4.2 | 5.3 | 43% |
| 1250 | 1250-1257 (8) | 10.0 | 16.8 | 21.8 | 11.4 | 0 | 8.5 | 17.7 | 11% |
| 1300 | 1300-1310 (8) | 184 | 287 | 291 | 196 | 0 | 103.8 | 134.9 | 9% |
| 1350 | 1350-1352 (**3**) | 530 | 534 | 538 | 526 | 0 | 213.9 | 323.2 | 94% |

The step from one band's median to the next is x2.0 (1200 to 1250), **x18** (1250 to 1300) and x2.9 (1300 to
1350): the curve is jagged, not smooth.  The games inside a band cluster tightly because consecutive n often
share their frames and candidates (1250-1253 all find 37285, 1254-1255 find 37834, 1256-1257 find 37858; in band
1300 the times fall into three groups, 127-193 s, 287-291 s and 103-106 s), so a band holds about three
independent samples, and band 1350 two.  Timeouts: none, so no statistic is censored.

### Fits of log10(time) against n (step 5)

x is the mean n of the band's games; one point per band; least squares.  The three bands below 1200 are cheap
whatever n is (few candidates), so they say little about growth and are left out of the main fit; the table gives
the fits from 1150 and from 1250 too.  Censoring: a band statistic that depends on a timed-out game would be a
lower bound and is left out of the fit (that biases a fit low); no statistic was censored here.

| statistic | bands used | b (per n) | s.e. of b | x10 per dn | residual sd (log10) | s at 1231 | s at 1300 | s at 1400 |
|---|---|---|---|---|---|---|---|---|
| median, bands from 1150 | 5 | 0.01832 | 0.00238 | 55 | 0.37 | 5 | 96 | 6521 |
| mean, bands from 1150 | 5 | 0.01840 | 0.00227 | 54 | 0.35 | 5 | 99 | 6865 |
| max, bands from 1150 | 5 | 0.01786 | 0.00240 | 56 | 0.37 | 8 | 133 | 8117 |
| **median, bands from 1200** | 4 | 0.01498 | 0.00244 | **67** | 0.27 | 9 | 98 | 3095 |
| **mean, bands from 1200** | 4 | 0.01512 | 0.00218 | **66** | 0.24 | 9 | 102 | 3304 |
| **max, bands from 1200** | 4 | 0.01420 | 0.00201 | **70** | 0.22 | 14 | 136 | 3585 |
| median, bands from 1250 | 3 | 0.01785 | 0.00410 | 56 | 0.28 | 5 | 86 | 5270 |
| mean, bands from 1250 | 3 | 0.01722 | 0.00411 | 58 | 0.28 | 6 | 93 | 4885 |
| max, bands from 1250 | 3 | 0.01446 | 0.00453 | 69 | 0.31 | 14 | 135 | 3760 |

**Headline: the standalone cost of a game on 8 threads grows by x10 per ~55-70 n (x5-8 per 50 n; main fit
x10 per 66 n, x5.7 per 50 n), with the slope uncertain by +-14% (one standard error) and 0.014-0.018 across the
fit choices.**  The fitted standalone cost reaches the 585 s limit at about n = 1350 (the 1350 band is at
510-540 s already), and the expected game at n = 1400 takes ~3100-3600 s standalone (~1400-1600 s chained),
roughly an hour on this machine.  The single step of x18 per 50 n from 1250 to 1300 is the only place where the
data show a rate near "x20 per 50 n"; the regression over four bands averages x5.7 per 50 n (0.76 against 1.26 in
log10).  A fit with 3-5 points and a residual of x1.7-2.3 (one sd) is a rough guide, not a law.

### Projection: cumulative cost of extending the chain from 1231 to N

Cost of the chain per non-prime n = 0.45 x 10^(a + b n) for the mean fit from 1200 (a = -17.65, b = 0.01512),
summed over the non-primes in 1232..N; the 192-CPU machine at 19x (optimistic: 192 CPUs at 0.8 of an M1
performance CPU each, linear scaling) or 10x (pessimistic) this machine's 8 threads; spot $3.33/h and
on-demand $8.35/h for c9g.48xlarge.  The median and max fits from 1200 give cumulative times within 8% (median)
and 16% (max) of these up to 1500, and within 25% up to 1600 (see `analysis_output.md`).

| N | game N standalone (s) | cumulative, this machine (8 thr.) | 192 cores, 19x | spot $ | on-demand $ | 192 cores, 10x | spot $ | on-demand $ |
|---|---|---|---|---|---|---|---|---|
| 1300 | 102 | 16.8 min | 53.1 s | 0.05 | 0.12 | 1.68 min | 0.09 | 0.23 |
| 1400 | 3.3e+03 | 10.7 h | 33.9 min | 1.88 | 4.72 | 1.07 h | 3.58 | 8.97 |
| 1450 | 1.88e+04 | 2.43 days | 3.07 h | 10.2 | 25.6 | 5.83 h | 19.4 | 48.7 |
| 1500 | 1.07e+05 | 12.9 days | 16.4 h | 54.5 | 137 | 1.29 days | 103 | 259 |
| 1550 | 6.12e+05 | 79.9 days | 4.2 days | 336 | 842 | 7.99 days | 638 | 1,600 |
| 1600 | 3.49e+06 | 1.26 years | 24.1 days | 1,930 | 4,840 | 45.9 days | 3,670 | 9,200 |
| 1700 | 1.13e+08 | 39.8 years | 2.09 years | 61,100 | 153,000 | 3.98 years | 116,000 | 291,000 |
| 1800 | 3.68e+09 | 1,340 years | 70.7 years | 2.06e+06 | 5.17e+06 | 134 years | 3.92e+06 | 9.82e+06 |
| 2000 | 3.89e+12 | 1.34e+06 years | 7.04e+04 years | 2.05e+09 | 5.15e+09 | 1.34e+05 years | 3.90e+09 | 9.78e+09 |

Uncertainty of the slope alone (mean fit from 1200, slope +-1 s.e. through the same centroid), cumulative time on
this machine: to 1400 6.8 h - 17 h, to 1500 5 - 34 days, to 1600 106 days - 5.6 years.  Other fit choices give
(to 1500, this machine): 12 days (median, from 1200), 27-47 days (from 1150 or 1250).

How far the chain gets on a budget (largest N; range = slope +1 s.e. to -1 s.e.):

| budget | 192 cores, 19x | 192 cores, 10x | this machine (8 thr.) |
|---|---|---|---|
| 1 day | 1509 (1484-1541) | 1493 (1467-1520) | 1423 (1407-1443) |
| 1 week | 1564 (1532-1609) | 1545 (1515-1586) | 1479 (1459-1509) |
| 30 days | 1607 (1568-1655) | 1588 (1553-1635) | 1521 (1496-1557) |
| 1 year | 1678 (1632-1739) | 1659 (1616-1717) | 1593 (1557-1642) |
| $100 spot | 1515 (1490-1549) | 1499 (1473-1528) | 1430 (1413-1453) |
| $1,000 spot | 1581 (1545-1629) | 1563 (1529-1605) | 1499 (1473-1528) |
| $10,000 spot | 1647 (1604-1704) | 1630 (1588-1682) | 1563 (1529-1605) |
| $100,000 spot | 1713 (1661-1780) | 1694 (1645-1760) | 1630 (1588-1682) |
| $1,000 on-demand | 1555 (1523-1595) | 1536 (1507-1575) | 1471 (1449-1499) |
| $10,000 on-demand | 1623 (1581-1674) | 1603 (1564-1651) | 1536 (1507-1575) |

In words: every factor of 10 in time or money buys +66 n.  A 19x machine therefore gains log10(19)/0.0151 = 85 n
over this one for the same time (1 day: 1423 to 1509), a 10x machine 66 n.  A day on 192 CPUs (about $80 spot)
reaches about N = 1510, a week (about $560 spot, $1,400 on-demand) about 1565 (1530-1610), a month ($2,400 spot)
about 1605, a year ($29,000 spot) about 1680, and $100,000 of spot time about 1710 (1660-1780).  The data show a
steep exponential (x10 per ~66 n) and no knee; it is the same curve, not a wall at one n.  What varies sharply is
the cost of single games, by the factorization of n: 1230 = 2*3*5*41 costs 207 s standalone, 1250-1253 about
4-6 s.

### Thread scaling (step 4): the proof of 1225, seeded with its optimum

| n | threads | search s | speed-up over 1 thread | nodes (M) | k nodes/s per thread | speed-up of nodes/s | effective parallelism % |
|---|---|---|---|---|---|---|---|
| 1225 | 1 | 150.9 | 1.00 | 12.07 | 80.0 | 1.00 | 100.0 |
| 1225 | 2 | 62.7 | 2.41 | 9.79 | 78.0 | 1.95 | 100.0 |
| 1225 | 4 | 31.5 | 4.79 | 9.45 | 75.1 | 3.75 | 100.0 |
| 1225 | 8 | 18.0 | 8.39 | 9.30 | 64.6 | 6.46 | 100.0 |

The wall-time speed-up is 8.4x on 8 threads, because the tree gets smaller with more threads (12.07M to 9.30M
nodes; each thread's nogood tables are emptied every 50,000 of its steps, so the tree depends on the threads);
the node rate itself scales 6.5x on 8 threads (80 to 65 k nodes/s per thread, -19%).  The earlier engine measurement
for 1215 (65.0, 28.3, 17.7, 11.4 s on 1, 2, 4, 8 threads: speed-ups 2.3, 3.7, 5.7) gives a lower
figure, so 5.7-8.4x on 8 threads is the range seen on this machine.  Nothing was measured above 8 threads
(the 10-thread point, with its 2 efficiency CPUs, was not run), so the 10x / 19x assumptions for 192 CPUs
rest on the engine design, not on data here.

### Repeats, and what else was running

| n | first run s | nodes (M) | k nodes/s | repeat s | nodes (M) | k nodes/s |
|---|---|---|---|---|---|---|
| 1215 | 9.8 | 7.8 | 796 | 9.0 | 6.3 | 698 |
| 1225 | 33.1 | 19.3 | 583 | 25.5 | 14.0 | 548 |
| 1300 | 127.0 | 82.9 | 653 | 134.6 | 84.7 | 629 |
| 1230 | 206.9 | 111.7 | 540 | 219.9 | 116.3 | 529 |

The machine was not idle.  `load.jsonl` (a one-second `top` sample before and after each invocation, with no
search running) shows 11% of the 10 logical CPUs busy during the chain (WindowServer, kernel_task), 11-23% during
the calibration (daemons, and another session compiling and running tests in a worktree: `rustc` and a test
binary at ~70% of a CPU each around 12:22), 4-16% around band 1250, and **21-36% from 12:32 on, when a Zoom call
was running** (Zoom ~0.5 CPU, WindowServer ~0.4, kernel_task ~0.25, audio helpers ~0.2, more in short bursts):
roughly 1-2.5 CPUs' worth by the process list, 2-3.5 by the idle share, some of it on the efficiency CPUs at low
clocks, the rest on the performance CPUs that the 8 search threads use.  The chain, the calibration and band 1250
ran mostly before Zoom showed up in the samples (12:32); the rest of band 1300, band 1350 and the repeats and the
scaling run ran with it.

How much that inflated the times: the four repeats above, run with Zoom up, differ from the first runs by -12%,
-6%, -4%, -2% in node rate (median -5%), and by 0.77-1.06 in time; the time differences are dominated by the
run-to-run variation of the node count with 8 threads (the tree depends on scheduling: 1225 took 14.0M and 19.3M
nodes), which is +-10-25% on its own.  So a loss of ~5% per node typically and <= ~12% at worst can be attributed
to the background load, i.e. at most ~0.05 in log10 of time: 1-3 n at the fitted slope, against x5.7 per 50 n
on average (x18 across 1250-1300) and a spread of up to x45 between neighbouring n (1230 against 1250).  The
data agree that it is small: it would move the later bands' times by <= 10%, the fitted slope by ~3% and the
cumulative times by <= 10%, much less than the slope's own standard error (14%).  The chain/standalone ratio is
unaffected (both ran in the quiet period, the chain in the quietest).  A measurement on a quiet machine would
still be better than this estimate.

## Caveats

* **Small, correlated samples.**  Four to six points per fit, each a band of 8 (or 3) neighbouring n that share
  frames in groups of two to four, so about two to three independent cost values per band.  Band 1350 has three
  games; the first, 1350, was also stopped at 477 s without finishing in a shortened run (consistent with its
  538 s).
* **Heavy tails and jagged structure.**  Cost depends on the factorization of n: 1230 (207 s standalone) is 29% of
  the chained time of 1001-1231, and the next ~60 n are cheaper; whichever n in a range is "bad" dominates the
  cumulative sum.  The projection uses a smooth fit of band means as the expected cost per n and is exact only if
  the unsampled n in a range are like the sampled ones.  A single bad n can cost a multiple of its band's
  mean.
* **Extrapolation.**  The fits are used 50-650 n beyond their data (1352), assuming log-linear growth.  A change
  in the frame structure (a further frame, more candidates) could bend the curve; the data show a x18 step and
  x2 and x3 steps, so the single-step uncertainty is large.  Costs past ~1700 (years of 192-CPU time) are only an
  order-of-magnitude statement.  A later study
  ([solver/notebook/what-drives-the-slope-2026-10/README.md](../what-drives-the-slope-2026-10/README.md)) found that a proof's cost follows
  the gap between the root's bound and the optimum, not n: for the proofs, the slope fitted here is the steep rise of
  that gap over 1190-1300, which did not go on in the games sampled past 1300 (these bands; the standalone cost of
  band 1350 is mostly finding its optimum).
* **Ratio assumption.**  The chained cost is taken as 0.45 x standalone from games at 1150-1231 (10 big
  searches, 8 of them at n >= 1150); it will drift if the share of the search before the final incumbent (median
  43% in band 1200, 11% in 1250, 9% in 1300, 94% in 1350) changes with n.
* **Hardware multiplier.**  The 19x / 10x factors are assumptions of the study.  Measured here: 5.7-8.4x on 8 threads
  over 1 thread, and a falling node rate per thread (-19% at 8); no data above 8 threads, none for Graviton5.  The
  ramp-up and tail of a 192-thread run are negligible for games of minutes (all workers are busy within 3 ms,
  [the benchmarks](README.md)), but the cheap games of a chain,
  each a second or less, cannot use 192 CPUs.
* **Limit 585 s** instead of 600 s (see above); no run hit it, so it does not affect the data.
* **Background load**, see above.  The timing of single multi-thread runs varies by +-10-25% from the search
  alone.
* Only proven tactics were measured.  `reuse_prev` and `revise_freedoms` would make the chain cheaper, but their
  answers are unproven.
