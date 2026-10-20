# Before a big run

Part of the [runbook](../RUNBOOK.md).

Check a long run on a big instance in three steps, the cheapest first, so that each step finds the problems that the
next one would cost more to find:

1. **Optional, on your machine:** build and test in Docker ([Dry run in Docker
   (optional)](#dry-run-in-docker-optional)).
2. **A small instance of the same family,** on spot, for an hour or two (cents): `setup.sh`, the tests and the scaling
   sweep up to its CPU count ([A small instance first](#a-small-instance-first)).
3. **The production instance** ([machine.md](machine.md): mid-size), on spot, for the checks: the large-machine tests,
   the sweep to its CPU count, the cost of a checkpoint pause, and a rehearsal of the whole chain to 1231 with
   deliberate interruptions (the sections from [The big instance: correctness](#the-big-instance-correctness) on).

Then start the production chain ([chain.md](chain.md)), from n = 1 or from the committed `results/strict.jsonl` ([its
range](../../../results/README.md)), in tmux, on spot, with a deadline and a copy of the state kept off the machine.

## Dry run in Docker (optional)

A Linux image of the build finds a Linux-only problem on your own machine first.  On an Apple-silicon Mac the container
is `linux/arm64`, the instruction set of Graviton.  From the repository root:

```
docker build -f solver/tools/cloud/Dockerfile -t taxman-cloud .
docker run --rm --init --cpus 2 taxman-cloud                       # cargo test, in the container
docker run --rm --init --cpus 2 -v taxman-work:/work taxman-cloud /taxman/solver/tools/cloud/scaling.sh --quick
```

The image holds the sources, the built solver and ladder, `python3`, `tmux` and GNU `time`.  The solver and ladder are
generic code (`RUSTFLAGS` is empty in the image, so an image built on one machine runs on another).  `--cpus 2` limits
the threads the solver sees, because it reads the cgroup quota.  `--init` gives the container a real PID 1, which the
signal tests need.  `/work` is `TAXMAN_WORK` in the image: mount a volume on it to keep results and checkpoints between
runs.  To rehearse a stop and a resume:

```
docker run -d --init --cpus 2 --name txchain -v taxman-work:/work taxman-cloud /taxman/solver/tools/cloud/chain.sh 1000
docker logs -f txchain                  # Ctrl-C leaves it running
docker stop -t 60 txchain               # SIGTERM to chain.sh, which stops the solver cleanly
docker start txchain                    # runs the same command again: resumes
docker logs txchain | tail
```

Docker tests the Linux code paths: signals, `flock`, `fsync` of a directory, and the shell scripts on `bash` 5.  It says
nothing about speed: the scaling numbers of a container on a laptop mean nothing for a cloud machine.

## A small instance first

1. Launch it ([Launch](launch.md#launch) with `c8g.xlarge` or `c8g.2xlarge` and `Purpose=trial`), on spot, with a timer
   of 180 minutes.
2. Get the code and set up, as [Setup](launch.md#setup) says (5 to 10 minutes).
3. In `tmux new -s tx` (detach with Ctrl-b d, return with `tmux attach -t tx`), run the sweep:

   ```
   solver/tools/cloud/scaling.sh                        # brief: its default games are short (The scaling sweep)
   ```

   [Reading the sweep](#reading-the-sweep) reads the tables.  From this machine, take three numbers: the single-thread
   rate (`k/s/thr` at 1 thread: an M1 Max does 12.3 at n = 1215, the sample of [Reading the sweep](#reading-the-sweep),
   and the calibration found a Graviton4 core runs the solver at 0.9 of an M1 Max core, so expect about 11 from a
   `c8g`), the speedup to the machine's threads, and the `Maximum resident set size` in the log (GNU `time` reports it,
   if installed).
4. Try the pieces that will matter on the big machine, on a small chain:

   ```
   TAXMAN_WORK=~/small EXTRA_ARGS="--time-limit 2" solver/tools/cloud/chain.sh 1000
   ```

   Every search that takes more than 2 seconds stops, and the next pass resumes it from its checkpoint.  The log shows
   the passes ("going on"), and it must end with "the range 1-1000 is complete", both referees agreeing.
5. Copy the logs home (`scp -r HOST:small/logs .`), then do what the rules say for the end: terminate the instance
   and confirm that nothing remains.

## The big instance: correctness

Launch the production instance ([machine.md](machine.md)) on spot, with a timer of 240 minutes, and run `setup.sh` as in
[Setup](launch.md#setup).  Use tmux for everything.  Count on one to one and a half hours, about $1 on a `c8g.16xlarge`
at the spot price of 2026-10-14 ($0.68 an hour).

```
cd ~/taxman/solver/frame
cargo test -- --test-threads=4                                       # also run by setup.sh
cargo test --release --test engine -- --ignored --nocapture --test-threads 1 many_threads
TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored stress
TXENGINE_CONSERVATION=500 cargo test --release --test robustness conservation
```

* **The suite** takes about 30 s after compiling, with four test threads.  It includes the small trees of the node-count
  tripwires (`tests/tripwires.rs`; the large ones need `--release`), which assert exact trees on one thread, and so
  check the arithmetic and the search against a different CPU and
  compiler.
* **`many_threads`** runs the engine on a toy problem at 64 and at 192 workers: answers, splitting, checkpoints every 2
  ms, spilling, a panicking worker.  `--nocapture` prints each run's counters (`eff`, `splits`, `full`), the first look
  at 192 threads.
* **`stress`** draws random engine configurations (1 to 192 threads, tiny queues, checkpoints every few ms, limits) and
  runs each to completion through any number of resumes.  5000 iterations take 200 s on an M1; a failure names the
  configuration, and `TXENGINE_STRESS_SEED=n` draws others.
* **`conservation`** checks that no unit of work is lost or done twice, by counting the nodes of a tree with no pruning.

All of these must pass, or the machine is not ready.  Save the output.

## The scaling sweep

```
solver/tools/cloud/scaling.sh              # --seed-opt proofs of 1215 and 1225 at 1, 2, 4, ... and all CPUs: brief
solver/tools/cloud/scaling.sh --engine     # adds the engine's own benchmark (no work per node): 5 to 10 minutes
solver/tools/cloud/scaling.sh --pauses     # adds one real game at all CPUs with a checkpoint every 0.5 s
GAMES="2170 2172" THREADS=1,4,16 solver/tools/cloud/scaling.sh    # dearer proofs, for the scaling to 16 threads
```

Each ladder line searches one game at one thread count, with the game's optimal score as the starting incumbent
(`--seed-opt`), so the work is the proof alone and the node count barely depends on the threads.  It uses the default
tactics and the ceiling of the chain.  The script prints a table for each game and one for all games together.  Run it
on an idle machine; the log (`~/taxman-work/logs/scaling-*.txt`) starts with a description of the machine and the build.

The default games are brief (the proof of 1215 takes 0.3 s on one M1 thread), so the default sweep shows that the
machine and the build work, and the rates of a few threads, not the scaling.  The proofs of 2170 and 2172 take about a
minute on one thread, which suits 4 to 16 threads and takes seconds at 64.  For 64 and 192 threads use items 7 and 8
(the node rate a thread, and the window of games in flight) of the [cloud commands in the scaling
entry](../../notebook/scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine); the [scaling
run](../../notebook/scaling-to-192-threads-2026-10/README.md) ran them once on a `c8g.48xlarge`.

## Reading the sweep

A sample table (an M1, three thread counts, `--seed-opt`, n = 1215, a search of a third of a second):

```
scaling, n=1215 (speedup over 1 thread(s), times 1; * = stopped by the time limit):
  threads   search s speedup       nodes  k/s/thr  µs/step  µs/tree  eff % idle % lock %  reb %  1st % ramp % tail %  splits    units  steals     runs    tiny  full s
        1      0.30     1.00        3647     12.3    63.64    63.48   99.9    0.0   0.00    0.0    0.3    0.0    0.0       0        0       0        1       0   0.000
        2      0.16     1.80        3855     11.7    64.96    62.73   96.9    3.0   0.00    0.2    3.7    2.7    0.2       2       27       2       30       4   0.007
        4      0.10     2.92        4493     11.0    66.02    62.87   91.2    8.5   0.00    0.3    4.9    6.5    0.3       4       43       8       48       4   0.007
```

| column | meaning | what to look for |
|---|---|---|
| `search s` | wall time of the search; `*` if the time limit stopped it | a `*` makes that row useless: raise `SECONDS_LIMIT` |
| `speedup` | the first row's time times its threads, over this row's time | close to the thread count for a long game (see the first note below) |
| `nodes` | steps taken, all threads | compare with the 1-thread row: tens of percent either way is normal (first note) |
| `k/s/thr` | thousands of nodes per second per thread | the engine's efficiency apart from the tree size. It falls a little with more threads (11 % from 1 to 4 threads above; 93 to 94 % of the one-thread rate at 64 threads on Graviton3 and 4, in the [calibration](../../notebook/graviton-calibration-2026-10/README.md)), and it depends on the game (12.3 at n = 1215 above, 8.3 at 2172 on one thread: [round B](../../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)). A step down at one thread count suggests memory, frequency or NUMA |
| `µs/step` | the workers' busy time per step (busy time over `nodes`), in µs | `k/s/thr` is about 10 × `eff %` / `µs/step` for a search that is long against its own setup, so a fall of `k/s/thr` is a rise of `µs/step` (a step costs more) or a fall of `eff %` (the workers wait); compare the rows of a game |
| `µs/tree` | the same without the units' starts: (busy time − rebuild − first nodes) over (`nodes` − units with a first node) | the cost of a step inside a unit's tree. If it stays level while `µs/step` rises, the units' starts are the loss (`reb %` and `1st %`); if it rises with `µs/step`, the step itself costs more on more threads (caches, memory, fewer nogoods learned a thread) |
| `eff %` | busy time / (wall × threads) | 99 to 100 up to 8 threads on the M1; below about 90 at 192 only means the game is too short, if `idle %` explains it |
| `idle %` | time workers waited for work | ramp-up and tail: they matter when the whole search takes seconds, and a long game keeps it small |
| `lock %` | time waiting for the deque's lock | stays near 0; a few percent on long games at high counts means the engine's one shared deque is contended, and per-worker deques are the next step (`src/engine.rs`, "Why one deque") |
| `reb %` | time units spent rebuilding their state, from the start of a unit to its first step | part of `µs/step`, paid once a unit, so it rises with the number of units and the threads (0.0, 0.2 and 0.3 % in the sample) |
| `1st %` | time units spent in their first node, whose GF(2) bound is solved from scratch | the same (0.3, 3.7 and 4.9 % in the sample); if `reb %` and `1st %` together are most of the rise of `µs/step` from 1 thread, the units' starts are the loss |
| `ramp %` | idle time before all workers first ran at once | part of `idle %`: the start of the search, which is most of a short game at many threads |
| `tail %` | idle time after the last moment a worker finished a unit while every worker was running | part of `idle %`: the end of the search, when the work runs out |
| `splits`, `units` | times a worker handed work over, and the units those gave | grow with the thread count: the demand for work, not a defect |
| `steals` | units taken from the front of the deque by workers that had waited | |
| `runs`, `tiny` | units run, and those that polled once or never (nothing left in them) | compare the share with the rows of few threads; ["The engine"](../../notebook/the-engine/README.md) records 13 of 227 at 2 threads |
| `full s` | time until every worker was running at once | milliseconds at 8 threads (0.9 to 2.8 ms seen); expect tens of ms at 192; seconds would mean the split protocol ramps up too slowly |

Notes on judging it:

* **Time is the measure; the speedup mixes two effects.**  Each thread keeps its own table of learned conflicts
  (nogoods) and forgets it every 50,000 of its steps, so the tree differs with the thread count.  The proof of 2172 took
  1.43M nodes on 1 thread and 1.27M on 4 on a Graviton4 core, and sped up 4.5x on 4 threads while the node rate sped up
  4.0x ([calibration](../../notebook/graviton-calibration-2026-10/README.md), which has 16 and 64 threads too).
  Speedup(time) = speedup(nodes per second) × (nodes at 1 thread / nodes at T threads).  Judge the engine by `k/s/thr`
  and `eff %`, the search by `nodes`, and the result by time.
* **Games must be long enough for the thread count.**  A proof of 1215 takes 0.3 s on one M1 thread and one of 2172 a
  minute: at 64 or 192 threads that is milliseconds to seconds, of which the ramp-up and the tail are a large part.  An
  `eff %` below 100 at 192 threads for such a game is expected and says nothing about long games.  The long games are
  the dear ones past 2175 (in the calibration's chain on 64 threads, 2184 took three minutes and 2260 a minute and a
  half: `solver/frame/testdata/c8g.chain-2176.jsonl`), and the chain's progress lines are the real test:
  [Monitoring](chain.md#monitoring) lists what a healthy line shows.
* **Memory** is the `Maximum resident set size` line near the end of the log, if GNU `time` was available.
* **Compare with the small instance:** the 1- and 8-thread rows of both should agree on `k/s/thr`.
* **Stop if** `lock %` reaches several percent or `full s` reaches seconds at 192 threads on a long game, if `k/s/thr`
  at 192 falls below half that at 1 thread (the machine, not the code, is probably the limit, and a smaller instance is
  cheaper for the same work), or if any test fails.

## The cost of a checkpoint

`scaling.sh --pauses` solves game 1230 (from the committed chain up to 1229) at all CPUs, with a checkpoint every 0.5 s,
and prints the progress line of the end: `pauses N (x%) max Yms park Zms`.  `x` is the share of the workers' time spent
parked in checkpoints at that absurd rate; at the real interval of 300 s the share is 600 times smaller.  `max` is the
longest pause (the wait for the slowest worker, then writing and `fsync`-ing a few MB to EBS), and `park` the longest
wait for the workers to park (10 to 15 ms at most on the M1).  A `max` of a few hundred milliseconds is fine at a
300-second interval; seconds would mean slow storage, and `CHECKPOINT_INTERVAL` can go up further.

## Rehearsal of the chain, with interruptions

This is the chain in miniature, to 1231, in a separate work directory, with every search that lasts more than 5
seconds stopped and resumed:

```
TAXMAN_WORK=~/rehearsal EXTRA_ARGS="--time-limit 5" solver/tools/cloud/chain.sh 1231
```

It takes minutes.  The log (`~/rehearsal/logs/chain.log`) shows each pass, and at the end `verify.sh` runs both referees
on the rehearsal's file and the committed one.  **It passes if the chain completes, the log says "both referees agree",
and `verify.sh` prints `1231 games are in more than one file, with equal scores in all`** (the scores of 1..1231
equal the committed chain's; the moves may differ).  A game that stopped and resumed is no different from one that ran
in one piece (`"assumes":[]`), so the chain goes on from it.  Then rehearse the two harder interruptions by hand, with
`TAXMAN_WORK` exported as `~/rehearsal2` in both tmux windows:

```
solver/tools/cloud/chain.sh 1231                    # window 1
until ls ~/rehearsal2/ckpt/n*.ckpt >/dev/null 2>&1; do sleep 1; done    # window 2: wait for a checkpoint to exist
kill -TERM $(cat ~/rehearsal2/chain.pid)          # window 2: a clean stop (chain.sh exits with 143)
solver/tools/cloud/chain.sh 1231                    # window 1: resumes where it stopped
until ls ~/rehearsal2/ckpt/n*.ckpt >/dev/null 2>&1; do sleep 1; done    # window 2: wait for a checkpoint again
kill -KILL $(pgrep -x taxman)                     # window 2: a crash
```

The stops wait for a checkpoint to exist, not for a number of seconds: the games past 2175 finish in seconds on a big
machine, and a stop before the first checkpoint tests nothing.  A checkpoint is a file `ckpt/n*.ckpt`; the solver writes
one every `CHECKPOINT_INTERVAL` seconds (300 by default) and at a stop, so a short interval (`CHECKPOINT_INTERVAL=10`)
keeps the wait brief.

After the TERM the log says "stopped by a signal (a final checkpoint was written)".  After the KILL it says "the solver
crashed (status 137)", `chain.sh` starts the solver again after 10 s, and the next pass resumes from the last checkpoint
(the KILL loses at most one checkpoint interval, and the finished games stay).  Let it run to the end, with the same
pass criteria.  Delete `~/rehearsal*` afterwards.
