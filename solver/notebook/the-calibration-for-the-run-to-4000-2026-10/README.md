# The calibration for the run to 4000 (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  What will one `c8g.16xlarge` do at the end of the chain?  The laptop's profile of game 3441 resumed from the
second run's checkpoint ([solver/notebook/the-calibration-for-the-run-to-4000-2026-10/README.md](README.md); no software loss with threads, the GF(2)
bound about 92% of a node) left the cloud machine unmeasured at that depth: does a thread's node rate hold from 1 to 64
threads, is the engine as effective, and does turning off the forgetting of nogoods change the rate at 64 threads?  The
answers set what the next run (to 4000, projected then) could count on.

*What was run* (2026-10-16 16:41:00Z to 17:07:23Z, 26 minutes of script, commit 97db411, one `c8g.16xlarge`; the
script and its output are in
[solver/notebook/the-calibration-for-the-run-to-4000-2026-10/calib-20261016](README.md#the-calibration-on-the-cloud-2026-10-16)):

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
proof of 2172 the [Graviton calibration](../graviton-calibration-2026-10/README.md) had found a thread's node rate at 64 threads
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
entry](../search/forgetting-nogoods-2026-10/README.md) measured the trees).

*What it decided.*  On 64 threads the search loses nothing to threads at the end of the chain, so what remains is cheaper
nodes and smaller trees ([STATUS](../../docs/STATUS.md)), which is what [the tuning
campaign](../search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md) went on to measure the same day.  The cost of the
machine was not read: about $0.40 at spot with the build, an estimate.

*Commits.*  97db411 (the build; the laptop's profile merged just before).  The checkpoint (5.3 MB) and the four copies of
the chain's results file (17 MB each) that the script wrote are not kept.

## Files

### On the laptop (`laptop/`)

The files are in `laptop/`.

Game 3441, resumed from the c8g.16xlarge chain's checkpoint (`n3441.ckpt`, 625,763,557 nodes, format 4, written when
the chain was stopped on 2026-10-15), on the M1 Max (8 performance cores) with the release build of master ad9fcac
(`debug = 1`), one process at a time under `nice -n 10` (the eight-copies run excepted), with other jobs and a video
call on the machine (load 5 to 16).  Wall rates are therefore not trustworthy; the rates to use are per CPU-second
(user + system from `/usr/bin/time -l`, net of the setup-only run on as many threads) and the proportions of the
profiles.  Neither the checkpoint nor `results/strict.jsonl` is modified: each run works on copies.

| file | what it is |
|---|---|
| `run.sh` | one run: `TAXMAN_BENCH_SRC=DIR TAXMAN_BENCH_OUT=DIR laptop/run.sh THREADS SECONDS TAG [PROFILE_SECONDS]` after `cargo build --release` in `solver/frame` (SRC holds the checkpoint and games 1..3440 of `strict.jsonl`); `SECONDS` 0 is the setup alone |
| `runs.jsonl` | `summarize.py RUNS CLOUD_STRICT setup1 setup2 setup4 setup8 t1 t2 t4 t8 copy1 ... copy8`: counts, rusage, the window's rates (from 20 s into the search), the solver's counters, and the net cycles, instructions and IPC; `copies` pools eight independent one-thread runs made at once (the hardware's share of the loss with threads) |
| `cloud.jsonl` | the same figures from the c8g's 64-thread progress lines for games 3441 and 3440 |
| `profile_t1.txt`, `profile_t8.txt` | `groups.py target/release/taxman sample.txt` on `run.sh 1 100 prof1 60` and `run.sh 8 100 prof8 60` (macOS `sample`, 1 ms, from 25 s into the run): groups, kernels, inclusive and self time of the workers' busy samples |

Commands as run: `t1`, `t2`, `t4`, `t8` = `run.sh T 130 tT`; `setupT` = `run.sh T 0 setupT`; `copyK` = eight at once of
`run.sh 1 130 copyK`.  `groups.py` and `solver/tools/profile/attribute.py` share one parser and symbolicator (`atos -i`).
The search began about 42 s after the start on one thread (12 s on eight: the start incumbent's local search is
parallel) and its clock starts then; the cloud log's "5.9 hours" of game 3441 were 5 hours of waiting in the window
for game 3440 and a 50-minute search.

### The calibration on the cloud (2026-10-16)

The same game, resumed
from the same checkpoint, on one `c8g.16xlarge` (64 CPUs; 2026-10-16, 16:41:00Z to 17:07:23Z, commit 97db411),
and four proofs of the ladder.  The files are in `calib-20261016/`:

| file | what it is |
|---|---|
| `calib.sh` | the script as it ran, from `~/taxman` after `setup.sh`: `taxman solve 3441 --threads T --time-limit 180 --checkpoint-interval 300 --progress 10` for T = 1, 16, 32 and 64, each on copies of the checkpoint and of `results/strict.jsonl`, under `perf stat -d` and `/usr/bin/time -v`; then `ladder --threads T --seed-opt --known results/strict.jsonl [--tactic forget_nogoods=off] 3232 3360 3388 3179` (60 s a game) at 64 threads with the default tactics, at 64 with `forget_nogoods=off`, and at 32 with the default |
| `out/calib.log` | its log: the machine (`lscpu`), the stages' times, the last two progress lines of each resumed run and the ladder's rows (one of its lines is the S3 download's progress, left in) |
| `out/t1/`, `out/t16/`, `out/t32/`, `out/t64/` | for each thread count: `progress.txt` (the solver's progress line every 10 s), `perf.txt` (cycles, instructions, branch and L1 misses), `time.txt` (`/usr/bin/time -v`) and `stdout.txt` (the game's line: stopped, incomplete) |
| `out/ladder_default64.txt`, `out/ladder_noforget64.txt`, `out/ladder_default32.txt` | the ladder's rows, a game a line (the nodes, the rate and the engine's counters; all four are `OPEN` at 60 s), each with its `.time` (`/usr/bin/time -v`) |

Not kept: the checkpoint (5.3 MB) and the four copies of the chain's results file that each run wrote (`out.jsonl`, 17
MB each), which the S3 prefix of the run held.
