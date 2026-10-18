# The run to the wall (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  How far does the unattended run of [RUNBOOK section
12](../../tools/cloud/RUNBOOK.md#12-the-unattended-run-to-the-wall) take the chain from n = 1 on one `c8g.16xlarge` at
spot, with the settings of [the tuning campaign](the-tuning-campaign-at-the-end-of-the-chain-2026-10.md)
(`force_window=all`) and the counter that could no longer wrap ([the
wrap](cheaper-gf2-nodes-same-trees-2026-10.md#later-2026-10-the-stamps-wrap-and-the-committed-chain)); does it solve
1..3440 again with the committed chain's scores; and where is the wall, the first game whose own search takes more than
24 hours?

*What was run* (2026-10-17 07:20:47Z to 2026-10-18 18:58:04Z, commit eb84a6f, a clean tree): `wall.sh launch`, an Auto
Scaling group of one spot `c8g.16xlarge` (Graviton4, 64 vCPUs, 124 GiB) that replaces an interrupted machine, running
`chain.sh 8191`, that is `taxman solve 1-8191` from n = 1 with a window of 4 games in flight on all 64 threads, the
default tactics of that commit, and `--game-hours 24` (the wall).  The run's limits were the wall, 72 instance-hours
and 2026-10-22; its settings and ending are `run/config` and `run/ended` in
[bench/wall_run](../../bench/wall_run/README.md).  One machine ran all of it (no interruption notice came).  It was
stopped on request (`wall.sh stop`, a final checkpoint written) in the middle of game 3915, 49 minutes and 138 million
nodes into its search, after 128,236 s of the solver: 35.72 instance-hours, about $24.14 at the $0.676 the run
recorded.  The reason was to restart with the chain drawing on the known games (`solve --use-prev`, [RUNBOOK
12.4](../../tools/cloud/RUNBOOK.md#124-launch-watch-stop-clean-up)): the committed chain now holds 1..3914, so a run
that draws on it records those games with no search and starts at 3915, where this one stopped.

*What it made.*  The 3,914 games 1..3914, each built from the one before, nothing assumed.

* **1..3440 again: the same scores, proofs and methods, game for game** (3,440 of 3,440;
  `bench/wall_run/reproof_check.py` compares the run's lines, kept in `reproof.jsonl.gz`, with the committed file's).
  The lines differ in nodes and seconds, as the build does.  Over 2176..3440 the summed search seconds were 14.1 hours
  against the second run's 47.3 (0.30) and the nodes 7.80 billion against 17.73 (0.44); the games overlap in the
  window, so these add up to more than the time.  Game 3440 took 6,323 s (1.76 hours, 1.36 billion nodes) against
  18,184 s (5.05 hours, 3.72 billion).  Of the 165 games that the second run searched for more than a minute, the
  median took 0.31 of its time.
* **3441..3914 are new** and are appended to `results/strict.jsonl` byte for byte (the [results
  README](../../results/README.md) has the run's third part): 221 searches (178 exhausted, 43 at the ceiling), 193
  repeats and 60 primes; 21.28 billion nodes and 49.0 hours of game time; 107 games of 30 million nodes or more took
  20.65 billion of them.

| n | games | searches | nodes | search hours, summed | median of the exhausted searches |
|---|---|---|---|---|---|
| 3441-3550 | 110 | 53 | 1.63 G | 5.7 | 61 s |
| 3551-3700 | 150 | 69 | 2.75 G | 7.4 | 115 s |
| 3701-3800 | 100 | 49 | 6.72 G | 14.7 | 1,224 s |
| 3801-3914 | 114 | 50 | 10.18 G | 21.3 | 1,502 s |

*The longest games* (search seconds on 64 threads): 3441 at 9,973 s (2.77 hours, 0.79 billion nodes), 3440 at 6,323 s
(1.76 hours), 3888 at 5,271 s (1.46 hours, 0.48 billion), 3444 at 4,687 s (1.30 hours), then 3807 (1.21), 3896 (1.11),
3864 (1.08) and 3906 (1.06 hours): eight games over an hour, 35 over half an hour.

*What it says.*  The wall was far off: the dearest game took a ninth of the 24 hours.  But the median exhausted search
rose tenfold from the games of 3551-3700 to those of 3701-3800 (115 s to 1,224 s) and has stayed there, so that each
block of 100 games since has cost 15 to 19 hours of game time; the patches of dear games that the second run met here
and there, and from 3399 on, are the rule past 3700.  The table of STATUS of 2026-10-16 (the second run's trend) put
a day at 3,740 and three days at 4,050; 35.7 hours reached 3,914.

*Commits.*  eb84a6f (the build and `wall.sh`), 687d467 (3440 re-proven, in STATUS), c7d81c2 (games 3441..3914
appended); the logs, the chart of the search times and `reproof_check.py` are in
[bench/wall_run](../../bench/wall_run/README.md).

*Not kept.*  The checkpoint of game 3915 (2.9 MB; a build other than eb84a6f cannot resume it), the 22 MB results file
(1..3440 as reduced lines; 3441..3914 are the committed ones) and the other checkpoints.
