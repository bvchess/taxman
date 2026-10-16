# Graviton calibration (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The cloud runs (the chain on a many-CPU machine, the prover over 1301..2175) were planned with the M1 Max's
figures and an assumed Graviton core.  How fast is a Graviton core against an M1 Max performance core, for the solver
and for the prover; does a 64-vCPU instance run 64 one-thread copies as fast as one (the prover's mode: one game a
thread); how does one game's proof scale to 64 threads; how many chain games an hour does one instance make; and which
instance suits the prover's run and the solver's chain?  The first measurements on any Graviton.

*What was run.*  Two one-time spot instances in us-east-1, launched, measured and terminated within an hour each
(`rust/bench/cloud/`: `instance.sh` for the order, `calibrate.sh` for the phases, `parse.py` and `summary.py` for the
tables; the data and the commands are described in its [README](../../bench/cloud/README.md)): **c7g.16xlarge**
(Graviton3, Neoverse V1, 1 MiB of L2 a core, 32 MiB of L3) at $0.6218 an hour and **c8g.16xlarge** (Graviton4,
Neoverse V2, 2 MiB of L2 a core, 36 MiB of L3) at $0.6794; each 64 vCPUs (a vCPU is a core: one thread a core, one
socket, one NUMA node) and 128 GiB, Amazon Linux 2023, rustc 1.99.0, `-C target-cpu=native`, the code of 407645f
shipped by `git archive`, built as `tools/cloud/setup.sh` builds it (without the tests).  A **c9g.16xlarge** (Graviton5)
was launched too, and the spot service took it back for want of capacity five minutes in, before it had run anything; it
was not launched again (the authorization was one instance of each type), so Graviton5 is still unmeasured.  The
reference is the M1 Max, one process at a time (load average 3 to 5, other jobs running; the same proof of 2170
took 785 s wall at load 150 and 588 s at load 4), built the same way.  All one-thread figures are search seconds of the
ladder (`--threads 1 --seed-opt`) or the prover's `secs`; the long proofs 2170 and 2172 ran together on the instances
(two of 64 cores; the contention test below finds no cost).  Every node count is the same on all three machines
(10,371; 22,524; 41,282; 160,839; 1,430,720; 1,432,206 for 1215, 1232, 1287, 1326, 2170, 2172; the prover's 17,771 and
44,389 for 1300 and 990), and so are the probe's fingerprints (7e8e724717d26545, 162e4e94a52a95c6 with `gf2=on`).

*Per core* (seconds; speed is the M1 Max's time over the machine's):

| | M1 Max | c7g (Graviton3) | c8g (Graviton4) |
|---|---|---|---|
| probe, default (wall of the replays) | 1.52 | 2.18 | 1.68 |
| probe, `gf2=on` | 2.84 | 3.95 | 3.14 |
| proof 1215 / 1232 / 1287 | 1.32 / 3.04 / 4.76 | 1.82 / 4.10 / 6.33 | 1.45 / 3.28 / 5.10 |
| proof 1326 (160,839 nodes) | 19.06 | 25.78 | 20.71 |
| proof 2170 / 2172 (1.43 M nodes) | 588 / 603 | 865 / 843 | 671 / 656 |
| prover 1300 (17,771 nodes) / 990 (44,389) | 34.2 / 61.6 | 48.5 / 80.0 | 37.8 / 65.2 |
| **speed, the solver's five proofs** (sum of search seconds) | 1 | **0.70** | **0.90** |
| speed, the probe (default; `gf2=on`) | 1 | 0.70; 0.72 | 0.91; 0.90 |
| **speed, the prover** (1300 and 990) | 1 | **0.75** | **0.93** |
| ms a node: the proof of 2172; the prover at 1300; at 990 | 0.42; 1.93; 1.39 | 0.59; 2.73; 1.80 | 0.46; 2.12; 1.47 |

A Graviton4 core is 0.9 of an M1 Max performance core and a Graviton3 core 0.7 (0.93 and 0.75 on the prover).  The speed
varies little from game to game: 0.88 to 0.93 (c8g) and 0.68 to 0.75 (c7g) over the six proofs.

*Contention* (K copies of the same one-thread run at once; mean time of a copy over the time alone, the slowest copy in
brackets): the proof of 1326 at K = 16, 32 and 64: c7g 1.001, 1.000, 1.001 (1.006); c8g 1.001, 1.001, 1.001 (1.011);
the prover on 990 at K = 64: c7g 1.002 (1.010), c8g 0.996 (0.998).  There is none to speak of: a vCPU is a core with
its own L2, and the working sets (28 MB resident for a solver, 25 MB for a prover) are small.  So 64 games a
machine is 64 times one game, which is how the prover runs (`exact` gives each thread a game).

*Thread scaling* of the proof of 2172 (`--seed-opt`, search seconds; speedup over one thread; effective parallelism of
the engine 99.9, 99.6 to 99.7 and 98.1 % at 4, 16 and 64 threads on both):

| threads | c7g s | nodes | speedup | c8g s | nodes | speedup |
|---|---|---|---|---|---|---|
| 1 | 843.0 | 1,432,206 | 1 | 656.2 | 1,432,206 | 1 |
| 4 | 191.3 | 1,279,228 | 4.4 | 146.9 | 1,268,902 | 4.5 |
| 16 | 53.6 | 1,416,134 | 15.7 | 43.5 | 1,503,004 | 15.1 |
| 64 | 16.2 | 1,647,592 | 51.9 | 11.2 | 1,474,335 | 58.5 |

The 4-thread speedup is above 4 because that tree is 11% smaller (the trees differ run to run once threads share
nogoods); at 64 threads the tree is 3% (c8g) and 15% (c7g) larger and the node rate a thread falls to 93 to 94% of the
one-thread rate on both, which is the whole loss of efficiency.  Memory (maximum resident set size): a solver proof 32
MB on one thread, 52 at 4, 130 at 16 and 363 to 374 at 64 (46 MB for the M1 Max, which counts differently); the prover
27 MB; the probe 56; the chain run (a checkpointed `solve` with the whole results file loaded) 400 MB on c7g and 538 on
c8g.

*The chain* (`taxman solve FROM-TO --threads 64 --out COPY --checkpoint-dir DIR` on a copy of `results/strict.jsonl` cut
after FROM − 1; the laptop is not run again, its figures are the committed file's, solved on 8 threads with nothing
else running):

| | games | search s | sum of the games' total s | games an hour (total s) | scores |
|---|---|---|---|---|---|
| committed file (M1 Max, 8 threads), 2126..2175 | 50 | 226.4 | 250.1 | 720 | |
| c7g, 64 threads, 2126..2175 again | 50 | 62.0 | 120.7 | 1491 | the file's |
| c8g, 64 threads, 2126..2175 again | 50 | 45.2 | 69.3 | 2599 | the file's |
| c8g, 64 threads, 2176..2276 (15 minutes) | 101 | 712.3 | 769.2 | 473 | new |

The 101 new games went through `tools/check_results.py` behind the committed file (all checks passed, the chain 1..2276;
the file `bench/cloud/c8g.chain-2176.jsonl`); game 2277 was in its 78th second (7.75 M nodes) when the stop came and is
the file's last line, unfinished.  The run took 900 s of wall time, of which 43 s were the start-up (one thread checking
the 2125 games of the copy; the same at 1, 8 and 64 threads) and 79 s the unfinished game: 404 games an hour in all, 429
without the start-up.  Against the laptop on the games 2126..2175: 3.6 times as fast in the games' total seconds on c8g
(5.0 in search seconds, against the 7.2 that 8 times the threads at 0.9 of the speed would give: short searches do
not fill 64 threads) and 2.1 on c7g.  The gap between the instances is not the search: c7g's per-game fixed time (a
game that repeats a search: frames 0.26 s, 1.49 s in all) is 2.4 times c8g's (frames 0.11 s, 0.62 s), and c8g's is the
M1 Max's (0.11 s, 0.61 s), at 1, 8 or 64 threads alike (`bench/cloud/c8g.startup-and-fixed-cost.jsonl`); why Graviton3
is so slow there was not looked into.  The laptop's rate past 2175 was not measured; the ratio above puts it near 130
games an hour.

*Cost.*  Instance-hours from launch to the terminate call, at the spot price of the hour (it did not move): c7g 0.594
h, $0.37; c8g 0.775 h, $0.53; the c9g 0.078 h, $0.06; **$0.96 in all**, no volume, address or spot request left.  A
core-hour of work costs, in M1 Max performance-core-hours at the 16xlarge spot prices, $0.0118 (solver) and $0.0114
(prover) on c8g and $0.0139 and $0.0130 on c7g: Graviton4 is 12 to 15% cheaper a unit of work, and for the chain 1.7
times as fast a game at 1.09 times the price.  A proof of 2172 on one core costs $0.002 on c8g; 1000 chain games at this
stage cost $1.6 at 64 threads and take 2.3 to 2.5 hours.

*Which instance.*  (a) The prover over 1301..2175: the work is about 150 thread-hours on c8g (the entry above gives
142 at its 2.5 ms a node, on the M1 Max; divided by the prover's speed of 0.93), but the wall time is the dearest
game's, about 12 hours on c8g (10.9 on the M1 Max; 14.6 on c7g), from 16 threads up.  So rent 16 vCPUs, not 64:
**c8g.4xlarge on spot** ($0.24 to $0.31 an hour in us-east-1b, a, c and d on 2026-10-14; $3 to $4 for the 12 hours); a
c8g.16xlarge would cost $8 for the same 12 hours with 48 vCPUs idle after the first, and a c7g.4xlarge ($0.20 to $0.25)
the same as the c8g for a run 2.6 hours longer.  The prices of the smaller sizes were read, not run: a vCPU is a core,
so the figures above carry over.  (b) The solver's chain: **c8g.16xlarge on spot** ($0.68 an hour), 64 threads: a proof
scales to 58 times at 64 threads, 98% effective, and the instance makes 430 to 470 games an hour at the present
difficulty, $0.0016 a game.  Beyond 64 threads nothing is measured, and the fixed 0.6 s a game and the ramp-up of short
searches will cost: the average game here is 7.6 s at 64 threads, so a c8g.48xlarge ($2.0 to $2.5 an hour) would have to
make 3 to 3.7 times the games to pay.  Graviton5 would have to be 12% faster a core than Graviton4 to pay at the
c9g.16xlarge's $0.76 to $0.79; it is the one thing a repeat of the calibration (about $0.5) should add.
