# Before a big run

Part of the [runbook](../RUNBOOK.md).

Check a long run on a big instance in three steps, the cheapest first, so that each step finds the problems that the
next one would cost more to find:

1. **Optional, on your machine:** build and test in Docker ([Dry run in Docker
   (optional)](#dry-run-in-docker-optional)).
2. **A small instance of the same family,** on spot, for an hour or two (cents): `setup.sh`, the tests and a short chain
   ([A small instance first](#a-small-instance-first)).
3. **The production instance** ([machine.md](machine.md): mid-size), on spot, for the checks: the large-machine tests
   and a rehearsal of the whole chain to 1231 with deliberate interruptions (the sections from
   [The big instance: correctness](#the-big-instance-correctness) on).

Then start the production chain ([chain.md](chain.md)), from n = 1 or from the committed `results/optimal.jsonl.gz`
([its range](../../../results/README.md#optimaljsonlgz)), in tmux, on spot, with a deadline and a copy of the state
kept off the machine.

## Dry run in Docker (optional)

A Linux image of the build finds a Linux-only problem on your own machine first.  On an Apple-silicon Mac the container
is `linux/arm64`, the instruction set of Graviton.  From the repository root:

```
docker build -f solver/tools/cloud/Dockerfile -t taxman-cloud .
docker run --rm --init --cpus 2 taxman-cloud                       # cargo test, in the container
```

The image holds the sources, the built solver, `python3` and `tmux`.  The solver is generic code (`RUSTFLAGS` is empty
in the image, so an image built on one machine runs on another).  `--cpus 2` limits
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
nothing about speed: the timings of a container on a laptop mean nothing for a cloud machine.

## A small instance first

1. Launch it ([Launch](launch.md#launch) with `c8g.xlarge` or `c8g.2xlarge` and `Purpose=trial`), on spot, with a timer
   of 180 minutes.
2. Get the code and set up, as [Setup](launch.md#setup) says (5 to 10 minutes).
3. Try the pieces that will matter on the big machine, on a small chain, in `tmux new -s tx` (detach with Ctrl-b d,
   return with `tmux attach -t tx`):

   ```
   TAXMAN_WORK=~/small EXTRA_ARGS="--time-limit 2" solver/tools/cloud/chain.sh 1000
   ```

   Every search that takes more than 2 seconds stops, and the next pass resumes it from its checkpoint.  The log shows
   the passes ("going on"), and it must end with "the range 1-1000 is complete", both referees agreeing.
4. Copy the logs home (`scp -r HOST:small/logs .`), then do what the rules say for the end: terminate the instance
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
