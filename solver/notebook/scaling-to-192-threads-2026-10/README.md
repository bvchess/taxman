# Scaling to 192 threads (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-15.

**Question.**  After the inner loop's round, round A, the prime-exchange rule and round B's window, where does a
thread's time go at 64 and 192 threads, what does the window of games in flight do at those counts, and should the next
run at the end of the chain use a 192-vCPU `c8g.48xlarge` in place of a 64-vCPU `c8g.16xlarge`?  **Answer.**  A
`c8g.48xlarge` runs the chain past 2225 1.9 to 2.7 times as fast at 1.1 to 1.9 times the cost a game, so it buys wall
time, not games: the run stays on a `c8g.16xlarge`, with a window of 4, the best at both counts (2226..2276 in 55.0 s at
64 threads and 29.6 s at 192, against 56.5 s and 32.4-34.0 s at a window of 1).

## Method

One `c8g.48xlarge` on spot in us-east-1b on 2026-10-15: Graviton4 (Neoverse V2), 192 vCPUs on two sockets of 96, two
NUMA nodes (CPUs 0-95 and 96-191), 371 GiB, Amazon Linux 2023, rustc 1.99.0, `-C target-cpu=native`.  The code was that
of 55287ff (the renames merged after it change no behaviour), built by `solver/tools/cloud/setup.sh` with its smoke
checks and without the tests, with an instance role and SSM as in
[launch.md](../../docs/runbook/launch.md#steps-through-ssm-or-ssh).  One script ran the items in order, one process at a
time, and two more ran after it (the files below).  The 64-thread runs of the window and of the next games were bound
(`taskset`) to 64 CPUs of node 0, a stand-in for the one-node `c8g.16xlarge`; the ladder ran unbound, and once bound to
node 0.  Item 1 (one thread) ran on CPU 191 beside the 64-thread runs.  Every game of the window's runs was checked:
both referees (`solver/tools/cloud/verify.sh`) accept the window-16 files at 64 and 192 threads, and in all 13 runs of
2226..2276 every score, proof and `same_search` is the c8g chain's (`c8g48/checks.txt`).

The items are those of the commands that follow, which the [wall regime](../search/the-wall-regime-2026-10/README.md)
prepared for a cloud machine: 1, 3, 4, 7 and 8 ran; 2, 5 and 6 did not.

### For a cloud machine

Commands for an instance set up by `solver/tools/cloud/setup.sh`, from `solver/frame`.  `K` is the committed
results file; `K2` is `--known KNOWN`, with `KNOWN` the committed results followed by the c8g chain's lines
(`solver/frame/testdata/c8g.chain-2176.jsonl`).  Each line is one process; one-thread processes can run side by side,
one a vCPU.

1. The fixed costs on Graviton, every 25 boards past 2175 (one thread; 652 s):
   `wall setup K $(seq 2200 25 4095) > setup_c8g.jsonl`.
2. (Not run.)  Large GF(2) trees past 2175, one thread each, a time series of 10^7 nodes and more:
   `wall grow K2 --nodes 40000000 --progress 30 --ckpt-dir ck_N --every 60 N > grow_N.jsonl 2> grow_N.progress` for
   N = 2184 (25.6 million nodes on 64 threads), 2260, 2268 and 2254 (hours each: about 0.5 ms a node).
3. The pause of a checkpoint with 192 threads near the wall, on a copy of the committed results cut after 2183 and
   extended with the c8g lines to 2183: `taxman solve 2184 --threads 192 --out COPY --checkpoint-dir DIR
   --checkpoint-interval 1 --progress 10 --time-limit 600`; the end of its progress line gives `pauses N (x%) max Y park
   Z`.  Then the same at `--checkpoint-interval 300`, the chain's default.
4. Memory, the queue and the checkpoint at 192 threads at 4000, standalone (no game of 3999): `/usr/bin/time -v taxman
   solve 4000 --threads 192 --out scratch.jsonl --checkpoint-dir DIR --checkpoint-interval 300 --progress 10
   --time-limit 600`, with `ls -l DIR` once a minute.
5. (Not run apart from `verify.sh`.)  The results check: `time taxman verify-results results/strict.jsonl`.
6. (Not run.)  What an S3 save costs: with `S3_DEST` set, `/usr/bin/time -v solver/tools/cloud/save.sh` three times
   while a chain runs.
7. The node rate of a thread at 64 and 192 threads, on the proofs of 2170 and 2172 (the proof of 2172 scaled 58.5 times
   on 64 threads in the calibration, with the node rate of a thread at 93 to 94% of one thread's), one process at a time
   (`known.jsonl` is `KNOWN`):

   ```
   cargo run --release --example ladder -- --threads 1,16,64,96,192 --seed-opt --known known.jsonl \
       --seconds 3600 2170 2172
   ```

   In the ladder's columns, `µs/step` is the time of a step; `reb %` and `1st %` are the units' starts (the rebuild of
   the searcher's state, and the first node's GF(2) bound solved from scratch), which are part of it; `lock %`, `ramp %`
   and `tail %` are time not stepping (the lock after a run, the idle time before all workers first ran at once, and
   after the last moment one finished a unit while all were running); `µs/tree`, which is `µs/step` without the units'
   starts, is the step itself (caches, memory, nogoods learned by fewer steps a thread).
8. The window at 64 and 192 threads (`--window W`: W consecutive games in flight, sharing the threads), one process at a
   time, with the thread count in `T`:
   * (a) the short searches, the chain to 1300 at W = 1, 4, 8, 16 and 32, twice each: `taxman regress --to 1300
     --threads $T --window $W`, the seconds in the line `solved 1..=1300 ... in X s`;
   * (b) the chain past 2175: the first 2175 lines of the committed results and the c8g chain's lines to 2225 (each line
     starts `{"schema":5,"n":2176,...`, so `awk -F'"n":' '{ if ($2 + 0 <= 2225) print }'` cuts them), then
     `taxman solve 2226-2276 --threads $T --window $W --out wW.jsonl --progress 0` at W = 1 and 16 (51 games; games an
     hour = 51 × 3600 over the wall seconds), once more with `TAXMAN_WINDOW_TRACE=1` to count the window's events
     (`workers n=N a->b`: the threads of the search of N changed from a to b, below a for a reclaim, 0 for a suspension;
     `defer`; `hint`).  The results with the window must equal those without it: both referees on the file
     (`solver/tools/cloud/verify.sh`), and the scores, proofs and `same_search` of its games against the c8g file's
     lines (none should differ).

`WINDOW` in `solver/tools/cloud/chain.sh` is to be the W with the most games an hour in (b) at the machine's thread
count, if (a) points the same way; 1 if no W beats 1 by more than the difference between two runs of one W in (a); and
if 16 is still gaining on 8, 32 is to be tried in (b) first.

## Results

### Where a thread's time goes (item 7)

`ladder --seed-opt`, the proofs of 2170 and 2172 summed, from one thread's 147 s to a second at 192
(`c8g48/ladder.txt`):

| threads | search s | speedup | µs/step | µs/tree | eff % | idle % | 1st % | ramp % | tail % | lock % |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 147.31 | 1.00 | 154.00 | 154.00 | 100.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.00 |
| 16 | 9.32 | 15.81 | 154.72 | 152.91 | 99.3 | 0.6 | 1.3 | 0.4 | 0.1 | 0.00 |
| 64 | 2.67 | 55.24 | 159.99 | 154.33 | 96.0 | 3.8 | 3.7 | 2.6 | 1.2 | 0.00 |
| 96 | 1.91 | 77.29 | 162.96 | 156.06 | 93.5 | 6.0 | 4.3 | 3.8 | 2.1 | 0.00 |
| 192 | 1.16, 1.24 | 127.0, 118.8 | 176.24, 176.82 | 164.27, 164.35 | 88.3, 87.3 | 10.4, 11.7 | 6.4, 6.8 | 6.6, 6.4 | 3.4, 4.8 | 0.00, 0.01 |
| 192, after one thread | 1.88, 1.89 | 78.2, 78.1 | 286.62, 303.91 | 268.15, 284.52 | 90.8, 90.8 | 8.4, 8.5 | 6.5, 6.3 | 4.9, 4.8 | 3.1, 3.2 | 0.01, 0.00 |

The rows of 1 to 96 threads are one ladder's (a second gave the same to a few per cent; a ladder bound to node 0 gave
the same step at 64 and 96 threads: 155.1 and 155.5 µs/tree).  The 192-thread row is the two 192-thread runs of a
process that ran 64, 192, 64 and 192 threads; the last row is the 192-thread runs of the two processes that ran 1, 16,
64 and 96 threads first.  A longer proof, 2184 (3.7 million nodes; the c8g chain's search of it took 25.6 million nodes
and 186 s on 64 threads in the calibration's build):

| threads | search s | speedup over 16 | µs/step | µs/tree | eff % | 1st % | ramp % | tail % |
|---|---|---|---|---|---|---|---|---|
| 16 | 37.61 | 1.00 | 163.68 | 163.27 | 99.9 | 0.3 | 0.1 | 0.0 |
| 64 | 9.92 | 3.79 | 166.19 | 164.15 | 99.3 | 1.3 | 0.4 | 0.2 |
| 96 | 6.07 | 6.20 | 170.03 | 167.26 | 99.0 | 1.8 | 0.7 | 0.2 |
| 192 | 3.76 | 10.00 | 185.04 | 180.31 | 97.0 | 2.7 | 1.5 | 1.0 |

* *64 threads.*  The step inside a unit's tree costs what it costs on one thread (µs/tree 154.3 against 154.0; 164.2
  against 163.3 on 2184).  On proofs of 1.3 s the node rate of a thread falls 9%, half of it the units' first nodes (1st
  3.7%, the rebuilds 0.3%) and half the ramp and the tail (3.8%); on 2184, at 10 s, they are 1.4% and 0.6%, and the
  engine is 99.3% effective.
* *192 threads*, both sockets full: the step is 7% dearer than on one thread on the short proofs (164.3) and 10% dearer
  than on 16 threads on 2184 (180.3 against 163.3), which one full socket's 96 threads did not show (156), so it is the
  second socket (remote memory or the other chip's caches; not separated).  The units' starts are 7.1 to 7.4% of the
  time on the short proofs and 2.9% on 2184, the idle time 10 to 12% and 2.9%, and the lock 0.00 to 0.02%: the one
  shared deque is not contended at 192.  All workers ran after 59 to 112 ms.
* The node rate of a thread at 192 threads is 4.7 thousand (0.72 of one thread's) on the short proofs and 5.2 (0.85 of
  16 threads') on 2184; on 2288 (57 million nodes, 85 s), 3.48 thousand against 3.85 at 64 threads over the first minute
  (0.90).  2184 ran 10.0 times as fast on 192 threads as on 16 (the thread ratio is 12), so on long searches 192 threads
  do about 158 times one thread's work, about 60 times 2 threads of the M1 Max at the 0.79 below: the middle of the 43
  to 86 that the plan assumed.
* *The trees* differ by up to 18% between thread counts (948 thousand to 1.12 million nodes for the two proofs, 3.4 to
  3.8 million for 2184), with no trend in the threads.
* *A Graviton4 core against the M1 Max*, on this build: the one-thread proof of 2172 (510,340 nodes) took 78.4 s against
  round B's 61.8 s, a step of 152.8 µs against 120.5: 0.79 of the M1 Max's speed, against the calibration's 0.90 in its
  build.
* *The slow rows.*  In both ladder processes that ran the one-thread proofs first, the 192-thread rows had a step 1.6 to
  1.7 times as dear (µs/tree 268 and 285), with about the same effectiveness and units' starts, and 11 to 12 thousand
  involuntary context switches against 3 thousand in a process that began at 64 threads.  Two processes, two games each:
  not noise.  No `solve` run showed it: the five-minute run of the next games at 192 threads kept its node rate to the
  end.  The suspected cause, not checked, is the kernel's automatic NUMA balancing (`kernel.numa_balancing` is 1 on
  Amazon Linux 2023) drawing the workers toward the node whose memory the one-thread phase touched, two to a CPU.

### The window at 64 and 192 threads (item 8)

(a) The chain 1..1300 (`taxman regress`, the seconds of its `solved` line, two runs each):

| window | 1 | 4 | 8 | 16 | 32 |
|---|---|---|---|---|---|
| 64 threads | 11.0, 11.0 | 7.7, 7.8 | 6.3, 6.4 | 5.7, 5.9 | 5.7, 5.7 |
| 192 threads | 16.1, 16.1 | 10.0, 10.0 | 7.0, 7.0 | 5.4, 5.5 | 5.5, 5.5 |

(b) 2226..2276 behind the committed chain and the c8g chain's lines to 2225 (`taxman solve`; wall seconds of the run,
about 7 s of it the start-up check of the 2,225 games of the file; the nodes of the 51 games):

| window | 1 | 4 | 16 | 32 |
|---|---|---|---|---|
| 64 threads, s | 56.57, 56.52 | 54.97 | 68.16, 74.70 | 72.55 |
| 64 threads, nodes | 17.2 M, 17.1 M | 17.2 M | 22.5 M, 24.9 M | 24.0 M |
| 192 threads, s | 32.36, 34.01 | 29.56 | 34.37, 34.38, 32.79 (traced) | 31.98 |
| 192 threads, nodes | 16.1 M, 16.9 M | 19.2 M | 24.9 M, 25.7 M, 24.6 M | 22.4 M |

On the short searches of 1..1300 the window pays as round B found, more with the threads: 11.0 s to 5.7 at 64 threads at
16 or 32, and at 192 from 16.1 s (slower than 64 threads: searches of milliseconds on 192 threads) to 5.4.  Past 2225 a
search fills 64 threads by itself, and the window's speculation (younger searches run with a weak incumbent until the
game before them is concluded) adds 30 to 55% nodes at 16 and 32.  At 64 threads that makes the stretch 20 to 32% slower
than in order (68-75 s against 56.5); at 192 the spare threads absorb it (32-34 s at 1, 16 and 32 alike).  A window of 4
is the best measured at both counts in (b): 55.0 s at 64 threads with no more nodes, 29.6 s at 192 with 16 to 19% more
(one run each), and (a) points the same way (7.7 and 10.0 s against 11.0 and 16.1).  So `WINDOW` should be 4 for the
chain at 64 and at 192 threads, and `chain.sh`'s default of 16 costs a run at 64 threads a fifth of its games an hour
past 2225.  The trace at 192 threads with a window of 16 (`c8g48/window-trace.txt`): 555 grants, 152 reclaims, 36
suspensions, 75 hints and 4 deferrals for the 51 games.

### The next games, and which machine

2277 on, behind the c8g chain's lines to 2276, window 1, five minutes each (`c8g48/progress.txt`): 192 threads made 60
games (2277..2336, 21 of them searched, 232 million nodes) in 281 s of game time; 64 threads made 11 (2277..2287) and
were 207.8 s and 49.3 million nodes into 2288, which took 85.3 s and 56.9 million nodes on 192.  These games are new,
past the c8g chain, checked only by the solver's own conclusion of each game (the two runs agree on the scores and
proofs of 2277..2287): measurements, not results.  Their searches did 3.3 to 6.2 thousand nodes a second a thread on 192
threads and 3.9 to 5.3 thousand on 64 (`c8g48/games.jsonl`).  The cost a game compares the `c8g.16xlarge` at its
cheapest zone's spot price ($0.6752 an hour, us-east-1d) with the `c8g.48xlarge` at the price paid ($2.4142, us-east-1b)
and at its cheapest zone's ($1.9775, us-east-1d, which had no capacity at the launch):

| games | 192 threads, s | 64 threads, s | speed of 192 | its cost a game, paid; cheapest |
|---|---|---|---|---|
| 2226..2276 (window 1, the games' seconds) | 25.4, 27.1 | 49.7, 49.6 | 1.89 | 1.89; 1.55 |
| 2277..2288 | 125.0 | more than 293.1 | more than 2.34 | under 1.53; under 1.25 |
| 2277 alone (13 million nodes at both) | 14.3 | 38.3 | 2.68 | 1.34; 1.09 |
| the proof of 2184 (ladder) | 3.76 | 9.92 | 2.64 | 1.35; 1.11 |

So a `c8g.48xlarge` runs the chain past 2225 1.9 to 2.7 times as fast as a `c8g.16xlarge`, the more the longer its
searches, and costs 1.1 to 1.9 times as much a game: it buys wall time, not games, and nears the smaller machine's price
only on long searches at its cheapest zone's price.  Its spot pool is also thin: two of the five zones (us-east-1d and
us-east-1f) had no capacity at the launch, and the instance in us-east-1b had its interruption notice 44 minutes in.  At
these prices a game of 2226..2276 costs $0.00018 of game time on the `c8g.16xlarge` (the calibration's build: $0.0014,
387 s for the 51 games) and one of 2277..2288 at least $0.0046.

### Checkpoints and memory (items 3 and 4)

2184 at 192 threads behind the c8g chain's lines to 2183: with a checkpoint every second, 7 pauses in 7.6 s, 2.2% of the
workers' time, the longest 29.7 ms (21.3 ms waiting for the workers to park), and up to 5,749 units queued (each pause
makes every worker hand its stack to the deque); with 300 s, no pause (the game took 8.4 s) and 92 units at most.  A
pause costs about 25 ms of the machine at 192 threads: under 0.01% at the chain's 300 s.  4000 standalone at 192
threads: the search ended in 42.8 s, exhausted (19.8 million nodes, 99.5% effective, lock 0.0%, at most 81 units queued,
all workers running after 169 ms), before its first checkpoint, so the size of a checkpoint at 4000 on 192 threads is
unmeasured (the wall regime's 58 KB a thread stands).  Its score, 5,034,874, is a standalone search's (no seed, no
ceiling), kept as a measurement (`c8g48/solve-4000.jsonl`).  Memory (maximum resident set size, `c8g48/runs.jsonl`): the
chain past 2225 0.28-0.38 GB at 64 threads and 0.71-1.01 GB at 192; the ladder 0.53-0.77 GB at 192; 4000 2.28 GB.

### The fixed costs on Graviton (item 1)

`wall setup` on one thread, every 25th board of 2200..4075 (652 s; [setup_c8g.jsonl](setup_c8g.jsonl)), seconds against
round A's M1 Max figures in brackets:

| n | local search | the search's setup | board | moves | prime path |
|---|---|---|---|---|---|
| 2200 | 1.216 (1.046) | 1.512 (1.304) | 0.019 (0.015) | 0.007 (0.005) | 0.011 (0.009) |
| 4000 | 6.870 (6.131) | 6.921 (6.189) | 0.061 (0.048) | 0.022 (0.016) | 0.038 (0.029) |

The local search, nearly all of a searched game's setup, runs at 0.86 to 0.90 of the M1 Max's speed, the board layer's
pieces at 0.70 to 0.79; round A's 0.69 hours of one thread for the fixed costs of 2200..4095 are about 0.78 on a
Graviton4 core.  How fast the local search runs on 64 and 192 threads was not separated from the search.

### Cost of the run

From the launch (03:50:00 UTC) to the terminate call (04:35:26): 0.757 instance-hours at $2.4142 an hour (the price did
not move), **$1.83**, with the 30 GB root volume and the S3 copies adding cents.  The spot request records an
interruption for want of capacity at 04:33:49, the two-minute notice, as the last run was ending (04:34:33, its outputs
copied to S3); the terminate call came first, and nothing was lost.  Afterwards no instance, volume, spot request or
security group of the run was left.

## Decision

* **Machine.**  The next run at the end of the chain stays on a `c8g.16xlarge`.  A `c8g.48xlarge` is for when the days
  matter more than 10 to 90% more a game, with `spot-watch.sh` and `S3_DEST`.
* **Window.**  A window of 4 for the chain at 64 and at 192 threads.  This entry did not change `chain.sh`'s default of
  16; it is 4 since 2026-10-17.
* **A unit that carries its GF(2) level** (round B's design, so that the first node checks the level rather than solving
  it) would win at most the units' starts: 1.4 to 4.0% of a thread's time at 64 threads, 2.9 to 7.4% at 192 on searches
  of 1 to 10 s, 0.3 to 1.1% on long ones (2288 and 4000), and nearly all of that is the first node.  Not worth building
  for the `c8g.16xlarge`; [round B](../round-b-the-chain-on-many-threads-2026-10/README.md) measured the units' starts
  at 0.2% of a thread's time on 4 threads.
* **If 192-thread machines are wanted**, the larger losses are the second socket's dearer step (7-10%), the idle time of
  searches of a few seconds (3-12%) and the slow rows.  The slow rows come first: sample the threads' CPUs (`ps -L -o
  psr`) during such a row, and repeat the row under `numactl --interleave=all`; it takes minutes on the next 192-thread
  machine.  Then find where the second socket's cost goes.
* **Open:** a Graviton5 (a spot instance was reclaimed before it ran anything); the hours-long trees of item 2.

## Files

The measurements are not results: the games of the runs (2226..2276 again, 2184 again, 2277..2336 and 4000 new) are in
no results file.

| file | content |
|---|---|
| `machines/c8g48.txt` | the machine: `lscpu`, `numactl --hardware`, memory, instance metadata, compiler, commit |
| `setup_c8g.jsonl` | `wall setup K $(seq 2200 25 4095)` on one thread of the `c8g.48xlarge`, the code of 55287ff (item 1) |
| `c8g48/measure.sh` | the run: items 7, 8 (a and b), 1, 3 and 4 in that order, one process at a time, every output copied to S3 after each item; then 2226..2276 again at windows 1 and 16, and at 32, on both thread counts |
| `c8g48/extras.sh`, `c8g48/extras2.sh` | run after it: the first ladder again and a window of 4 (`extras.sh`); the games from 2277 on at 192 and at 64 threads, five minutes each (`extras2.sh`) |
| `c8g48/runs.py` | `runs.py runs DIR` and `runs.py games DIR` make `runs.jsonl` and `games.jsonl` from the directory the scripts wrote |
| `c8g48/steps.log` | every run of the three scripts: its command, the time it started and ended, its exit status and wall seconds |
| `c8g48/runs.jsonl` | one line per run: threads, window, CPUs, status, wall, user and system seconds, maximum resident set size, context switches, page faults, and `regress`'s own seconds |
| `c8g48/games.jsonl` | one line per game of each `solve` run: method, proof, score, nodes, threads, search and total seconds |
| `c8g48/ladder.txt` | the five ladders (item 7): each line of a search and the scaling tables, under the command that made them |
| `c8g48/progress.txt` | the progress lines of 2184 with a checkpoint every second and every 300 s (item 3), of 4000 standalone (item 4), and of the runs from 2277 on |
| `c8g48/window-trace.txt` | the window's events (`TAXMAN_WINDOW_TRACE`) for 2226..2276 at 192 threads with a window of 16 |
| `c8g48/checks.txt` | the scores, proofs and `same_search` of every window run's 2226..2276 against the c8g chain's lines, the trace's counts, and both referees on the window-16 files at 64 and 192 threads |
| `c8g48/solve-4000.jsonl` | the line of the standalone search of 4000 on 192 threads (exhausted in 43 s) |

## Commits

* 55287ff (2026-10-14): the code the run built; the renames merged after it change no behaviour.
