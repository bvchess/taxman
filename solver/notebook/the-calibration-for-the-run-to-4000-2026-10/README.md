# The calibration for the run to 4000 (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  What will one `c8g.16xlarge` do at the end of the chain?  The laptop's profile of game 3441 resumed from the
second run's checkpoint ([bench/chain_end](../../bench/chain_end/README.md); no software loss with threads, the GF(2)
bound about 92% of a node) left the cloud machine unmeasured at that depth: does a thread's node rate hold from 1 to 64
threads, is the engine as effective, and does turning off the forgetting of nogoods change the rate at 64 threads?  The
answers set what the next run (to 4000, projected then) could count on.

*What was run* (2026-10-16 16:41:00Z to 17:07:23Z, 26 minutes of script, commit 97db411, one `c8g.16xlarge`; the
script and its output are in
[bench/chain_end/calib-20261016](../../bench/chain_end/README.md#the-calibration-on-the-cloud-2026-10-16)):

1. Game 3441 resumed from the second run's checkpoint (625.7 million nodes) for 180 s of search at 1, 16, 32 and 64
   threads, one after another, under `perf stat -d` (`taxman solve 3441 --threads T --time-limit 180`).
2. The ladder on 3232, 3360, 3388 and 3179, seeded with their optima, 60 s each: at 64 threads with the default
   tactics and with `forget_nogoods=off`, and the default at 32 threads.

*Results.*  The resumed search, over its last 170 s (the nodes between the first and the last progress line):

| threads | nodes | thousand nodes a second | a thread | engine | instructions a cycle | branch misses | L1 misses |
|---|---|---|---|---|---|---|---|
| 1 | 0.51 M | 3.0 | 3.0 | eff 100%, idle 0.0%, lock 0.0% | 3.70 | 1.87% | 0.85% |
| 16 | 9.26 M | 54.5 | 3.4 | eff 100%, idle 0.0%, lock 0.0% | 3.80 | 2.44% | 1.08% |
| 32 | 18.4 M | 108.3 | 3.4 | eff 100%, idle 0.0%, lock 0.0% | 3.80 | 2.47% | 1.02% |
| 64 | 37.2 M | 218.9 | 3.4 | eff 100%, idle 0.0%, lock 0.0% | 3.81 | 2.48% | 1.05% |

(The counters are those of `perf` over the whole process, setup included.)  A thread does 3.4 thousand nodes a second at
16, 32 and 64 threads, the same to two digits, so 64 threads do 220 thousand a second, 19 billion nodes a day; the
one-thread run is the slowest a thread (3.0), not a faster one.  The engine's counters show no loss at any count: all
workers busy, no time in the locks, a checkpoint pause of 8.0 to 9.4 ms at most, the queue 4,600 to 6,200 units.  (On the
proof of 2172 the [Graviton calibration](graviton-calibration-2026-10.md) had found a thread's node rate at 64 threads
93-94% of one thread's.)

The ladder runs were stopped by their 60 s, none finished, so they measure node rates and not the growth of the tree:

| game | 64 threads, thousand nodes a second | `forget_nogoods=off` | 32 threads |
|---|---|---|---|
| 3232 | 191 | 191 | 93 |
| 3360 | 173 | 177 | 88 |
| 3388 | 161 | 163 | 85 |
| 3179 | 213 | 224 | 112 |

Doubling the threads doubles the rate (1.90 to 2.05 times, a mean 1.95); without the forgetting the rate is 0.3 to 5.2%
higher, as a table that grows unchecked would give, which says nothing about the tree (the [forgetting
entry](forgetting-nogoods-2026-10.md) measured the trees).

*What it decided.*  On 64 threads the search loses nothing to threads at the end of the chain, so what remains is cheaper
nodes and smaller trees ([STATUS](../STATUS.md)), which is what [the tuning
campaign](the-tuning-campaign-at-the-end-of-the-chain-2026-10.md) went on to measure the same day.  The cost of the
machine was not read: about $0.40 at spot with the build, an estimate.

*Commits.*  97db411 (the build; the laptop's profile merged just before).  The checkpoint (5.3 MB) and the four copies of
the chain's results file (17 MB each) that the script wrote are not kept.
