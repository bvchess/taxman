# Running the Rust Taxman solver on a many-core cloud machine

The target is an AWS Graviton5 instance, `c9g.48xlarge` (192 cores), running Linux; the scripts here also work on
Graviton4 (`c8g`) and on x86_64, on Amazon Linux 2023 and Ubuntu.  What the solver is and how to read its output is in
[../../README.md](../../README.md); this page is the procedure.

**Status of this procedure.**  The scripts were exercised on macOS (arm64): `chain.sh` through a complete chain, a
SIGTERM stop, forced interruptions and resumes, a crash loop and a deadline; `scaling.sh` in all three modes (a sweep,
`--engine`, `--pauses`); `setup.sh` through its build and checks; `verify.sh`.  They have not been run on Amazon Linux,
Ubuntu or EC2; `save.sh` and `spot-watch.sh` need S3 and EC2 and have not been run at all; the Dockerfile has not been
built.  Sections 2 and 3 exist to find the problems that a macOS run cannot, on a machine that costs cents.  Nothing
about scaling beyond 8 threads or about Graviton5 has been measured (section 8).

* [0. The plan](#0-the-plan)
* [1. Decisions](#1-decisions)
* [2. Dry run in Docker](#2-dry-run-in-docker-optional)
* [3. A small instance first](#3-a-small-instance-first)
* [4. The big instance: checks before the chain](#4-the-big-instance-checks-before-the-chain)
* [5. The production chain](#5-the-production-chain)
* [6. Verifying, and bringing the results home](#6-verifying-and-bringing-the-results-home)
* [7. When something goes wrong](#7-when-something-goes-wrong)
* [8. What has not been measured](#8-what-has-not-been-measured)

## 0. The plan

1. Optional, on your machine: build and test in Docker (section 2).
2. A small instance of the same family, on-demand, for an hour or two (cents): `setup.sh`, the tests, the scaling sweep
   up to its 8 threads (section 3).
3. The big instance, on-demand, for the checks: the large-machine tests, the sweep to 192 threads, the cost of a
   checkpoint pause, and a rehearsal of the whole chain to 1231 with forced interruptions (section 4).
4. The production chain, strict from n = 1, in tmux, on spot or on-demand, with a deadline and a copy of the state
   kept off the machine (section 5).
5. Verify, bring the results home, shut everything down (section 6).

The files in this directory:

| file | what it does |
|---|---|
| `env.sh` | settings and helpers the scripts share (paths, defaults, `env_report`); sourced, not run |
| `setup.sh` | packages, Rust, the checkout, the build (`target-cpu=native`), smoke checks and the tests; idempotent |
| `scaling.sh` | the scaling sweep (`--engine`: the engine benchmark; `--pauses`: the cost of checkpoints; `--quick`) |
| `chain.sh N` | the production run: the strict chain 1..N in a loop, resumable, logged, optional deadline and power-off |
| `verify.sh [FILE...]` | both referees on a results file, and against the committed results |
| `save.sh` | copy results, logs and checkpoints to S3 (`--restore` on a fresh machine) |
| `spot-watch.sh` | on a spot interruption notice: stop `chain.sh` cleanly and save |
| `Dockerfile`, `Dockerfile.dockerignore` | a Linux build-and-test image (section 2) |

All state lives in one directory, `$TAXMAN_WORK` (default `~/taxman-work`): `results/strict.jsonl`, `ckpt/` (the
checkpoints, `taxman.lock`), `logs/`, `chain.pid`, `deadline`.  Nothing is written inside the checkout except
`rust/target` (and `rust/bench/history.jsonl` if you run `taxman bench` without `--history`).

## 1. Decisions

### What is being computed

`chain.sh N` runs `taxman solve 1-N --tactic reuse_prev=off --tactic revise_freedoms=off` into one results file:
every game from n = 1, each built from the one before, with no unproven tactic.  Nothing rests on `optimal.json` (it is
only compared with, to n = 1000) or on the committed results; the committed files are what the run is checked against
(`verify.sh`).  The solver is the same code as on a laptop; the machine only changes how long it takes.

Cost grows by a factor of 10 for every 55 to 70 in n past 1200 ([bench/curve/README.md](../../bench/curve/README.md)).
From that file, the cost of extending the chain from 1231 to N on a 192-core machine, taking it to be 10 to 19 times
the 8 M1 threads of the measurements (an assumption, not a measurement), and the prices it assumes ($3.33/h spot,
$8.35/h on-demand for the 48xlarge; check the current ones):

| N | time | spot | on-demand |
|---|---|---|---|
| 1300 | 1 to 2 minutes | cents | cents |
| 1400 | 34 minutes to 1.1 hours | $2 to $4 | $5 to $9 |
| 1450 | 3 to 6 hours | $10 to $19 | $26 to $49 |
| 1500 | 16 hours to 1.3 days | $55 to $103 | $137 to $259 |
| 1550 | 4 to 8 days | $336 to $638 | $842 to $1,600 |

These are extrapolations from a handful of points: treat them as good to a factor of several in either direction, and
let the first hours of the real run decide (the time per game is in the log; section 5).  The first 1231 games take
minutes.  A single hard game near 1500 can take hours.  Choose N by money and patience; rerunning `chain.sh` with a
larger N later continues the same file.

### Instance

* **Workload.**  CPU-bound integer and bit-set code (no floating point, no SIMD requirement), one process, as many
  threads as cores; memory is tiny: about 30 MB plus 2 MB per thread were measured on an M1 (33 MB at 2 threads, 45 MB
  at 8), so a few hundred MB at 192 threads, plus 64 MiB of reserved address space per thread for stacks, which is not
  memory.  Disk I/O is one checkpoint a minute, tens of KB at 2 threads and at most a few MB at 192 (it holds the work
  stacks of all workers).  So: the cheapest cores, with 1 GiB per core or less.
* **`c9g.48xlarge`** (compute-optimized, Graviton5, 192 vCPUs): Graviton has no SMT, so a vCPU is a core and each
  worker thread gets a whole one; on Intel families a vCPU is a hyperthread and the throughput per thread is lower.
  If `c9g` is not offered in your region, `c8g.48xlarge` (Graviton4, 192 vCPUs) is the nearest, with the same
  instruction set family; on x86_64 compare the nodes per second per thread (`k/s/thr` in the sweep) before choosing.
* **A small instance of the same family first** (`c9g.2xlarge`, 8 vCPUs, or `c9g.4xlarge`): the same CPU, so the same
  build and the same per-thread speed, for a twenty-fourth of the price (or less).  It is where setup problems on Linux, the tests, the
  1-to-8-thread part of the sweep and the single-thread rate are found.
* **Check `lscpu` on the big one** for the number of NUMA nodes.  The engine does not pin threads; each searcher
  allocates its own state on its own thread, so first touch keeps it local.  If there are several nodes and the sweep
  shows a step in `k/s/thr` where the thread count crosses a node, that is the cause.
* **Build on the machine type that runs it.**  `-C target-cpu=native` is opt-in: a plain `cargo build` is generic code
  (README, section 2), and these scripts add it through `RUSTFLAGS` (the default of `env.sh`; set `RUSTFLAGS=` to build
  generic code).  It tunes for the CPU that builds; a binary built on a newer core can fail with an illegal instruction
  on an older one.  `setup.sh` prints the CPU that `native` resolved to (`native cpu:` line); it should be a Neoverse
  core name, not `generic`.  The gain of `native` has not been measured on Graviton (expect 0 to 3 %; on the M1 it
  measured 0 %).
* **Profile-guided optimization** (PGO) is optional: `rust/tools/pgo.sh` makes the build (its header says how; on the
  M1 it adds 0 to 2 % now, see [DESIGN.md](../../DESIGN.md#10-performance)).  PGO changes code
  generation only: the search and its answers are the same, a checkpoint can be resumed by either binary, and
  `TAXMAN_BIN` / `LADDER_BIN` point these scripts at another binary.  Keep a non-PGO build too if you want to profile.

### Spot or on-demand

* **On-demand for sections 3 and 4**: a spot interruption in the middle of a measurement ruins it.
* **Spot for the production chain is reasonable**, because the run is built for it: a stop by SIGTERM loses nothing, a
  kill loses at most one checkpoint interval (60 s), and `chain.sh` is safe to start again.  What an interruption
  really costs is the time to get a machine again (a 192-vCPU spot pool can be thin) and your attention; fall back to
  on-demand, or to a smaller size (`THREADS` can differ between resumes), if you cannot.  Use `spot-watch.sh` to turn the
  two-minute notice into a clean stop and a save.
* A **persistent** spot request launches a replacement after an interruption: that is convenient and also a way to
  keep paying for something nobody watches.  If you use one, `DEADLINE_HOURS` and `POWEROFF` (section 5) are your
  protection, and cancel the request when the run is done.

### Storage

Checkpoints and results go to a disk that survives a stop or reboot, with local semantics (`flock`, `fsync`): the
root EBS volume (gp3, 40 GB is plenty: the toolchain and `target/` take 3 to 4 GB, results about 2 KB per game), or a
separate EBS volume mounted somewhere and named by `TAXMAN_WORK`.  **Not tmpfs**: `/tmp` is tmpfs on Amazon Linux 2023,
and `chain.sh` refuses a memory filesystem.  **Not instance-store NVMe** as the only copy: it is lost when the instance
stops or is terminated.  Not a network filesystem either (`flock`, and an `fsync` per checkpoint).

An EBS root volume is deleted when a spot instance is terminated unless "delete on termination" is off, so the copy
that matters is the one off the machine: `S3_DEST` (section 5) or a pull with `rsync` from your laptop.

### Launch checklist (console fields)

* AMI: Amazon Linux 2023, arm64 (user `ec2-user`); or Ubuntu 22.04 or 24.04 arm64 (user `ubuntu`).
* Storage: one gp3 root volume, 40 GB.  A second, if you want the state to outlive the instance: mount it, set
  `TAXMAN_WORK` to a directory on it, and turn "delete on termination" off.
* Security group: SSH from your address only; nothing else inbound.  A key pair.  Require IMDSv2 (`spot-watch.sh` uses it).
* Instance role (for S3 only): list, get, put and delete on one bucket prefix, nothing else.
* Shutdown behaviour "stop" for on-demand test machines; for spot, whatever the request says.
* Tags: a name and an owner, so that a forgotten instance is findable.
* A budget alert on the account, before the first launch.  Cost guardrails for the run itself are in section 5.

## 2. Dry run in Docker (optional)

A Linux image of the build, to find a Linux-only problem on your own machine first.  On an Apple-silicon Mac the
container is `linux/arm64`, the instruction set of Graviton.  From the repository root:

```
docker build -f rust/tools/cloud/Dockerfile -t taxman-cloud .
docker run --rm --init --cpus 2 taxman-cloud                       # cargo test, in the container
docker run --rm --init --cpus 2 -v taxman-work:/work taxman-cloud /taxman/rust/tools/cloud/scaling.sh --quick
```

The image holds the sources, the built solver and ladder (generic code: `RUSTFLAGS` is empty in it, so that an image
built on one machine runs on another), `python3`, `tmux` and GNU `time`.  `--cpus 2` limits the threads the solver
sees (it reads the cgroup quota).  `--init` gives the container a real PID 1, which tests that signal processes
need.  `/work` is `TAXMAN_WORK` there: mount a volume on it to keep results and checkpoints between runs.  To rehearse a
stop and a resume:

```
docker run -d --init --cpus 2 --name txchain -v taxman-work:/work taxman-cloud /taxman/rust/tools/cloud/chain.sh 1000
docker logs -f txchain                  # Ctrl-C leaves it running
docker stop -t 60 txchain               # SIGTERM to chain.sh, which stops the solver cleanly
docker start txchain                    # runs the same command again: resumes
docker logs txchain | tail
```

Docker is for the Linux code paths (signals, `flock`, `fsync` of a directory, the shell scripts on `bash` 5), not for
speed; the scaling numbers of a container on a laptop mean nothing for the cloud machine.

## 3. A small instance first

1. Launch it (section 1), on-demand.  Put a timer on it so that it cannot be forgotten: `sudo shutdown -h +180`.
2. Get the code and set up:

   ```
   sudo dnf install -y git tmux                       # Ubuntu: sudo apt-get install -y git tmux
   git clone https://github.com/bvchess/taxman.git && cd taxman
   rust/tools/cloud/setup.sh                          # 5 to 10 minutes
   ```

   The scripts must be on the branch that gets cloned (`BRANCH=...` chooses another).  Without a pushed branch, copy the
   tree from your machine (`rsync -a --exclude target --exclude .git ~/dev/taxman_work/taxman/ HOST:taxman/`) and
   run `setup.sh` from inside it: it then works on that tree and does not pull (results record no commit, then).
   `setup.sh` installs packages, installs Rust with rustup if there is none (stable, at least 1.89), builds with
   `RUSTFLAGS="-C target-cpu=native"`, replays `optimal.json`, runs both result verifiers on the committed files, runs
   `taxman regress --to 400`, and runs `cargo test -- --test-threads=4`.  Its output is also in
   `~/taxman-work/logs/setup.log`.  Run it again after any reboot or to pick up new commits.
3. In `tmux new -s tx` (detach with Ctrl-b d, return with `tmux attach -t tx`), run the sweep:

   ```
   rust/tools/cloud/scaling.sh                        # about 10 minutes at 8 vCPUs
   ```

   Read section 4.3 for the tables.  What to take from this machine: the single-thread rate (`k/s/thr` at 1 thread; the
   M1 Max did 100 to 130 thousand nodes per second per thread on single games from n = 1184 to 1215, depending on
   the game), the speedup to 8 threads, and a
   `Maximum resident set size` in the log (GNU `time` is used if installed).
4. Try the pieces that will matter on the big machine, on a small chain:

   ```
   TAXMAN_WORK=~/small EXTRA_ARGS="--time-limit 2" rust/tools/cloud/chain.sh 1000
   ```

   Every search that takes more than 2 seconds is stopped, and the next pass resumes it from its checkpoint; the log
   shows the passes ("going on"), and the end must be "the range 1-1000 is complete", both referees agreeing.
5. Copy the logs home (`scp -r HOST:taxman-work/logs .`), terminate the instance, delete its volumes.

## 4. The big instance: checks before the chain

Launch the 48xlarge on-demand, set the timer (`sudo shutdown -h +240`), and run `setup.sh` as above (about the same
time).  Use tmux for everything.  Count on one to one and a half hours, roughly $10 to $15 at the assumed price.

### 4.1 Correctness

```
cd ~/taxman/rust
cargo test -- --test-threads=4                                       # also run by setup.sh
cargo test --release --test engine -- --ignored --nocapture --test-threads 1 many_threads
TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored stress
TXENGINE_CONSERVATION=500 cargo test --release --test robustness conservation
```

* The suite takes about 30 s to run after compiling, with two or four test threads ([DESIGN.md](../../DESIGN.md#9-testing), "Testing"); it includes
  the node-count tripwires (`tests/tripwires.rs`), which assert exact trees on one thread and so check the arithmetic
  and the search against a different CPU and compiler.
* `many_threads` runs the engine on a toy problem at 64 and at 192 workers: answers, splitting, checkpoints every 2 ms,
  spilling, a panicking worker.  `--nocapture` prints each run's counters (`eff`, `splits`, `full`): the first look at
  192 threads.
* `stress` draws random engine configurations (1 to 192 threads, tiny queues, checkpoints every few ms, limits) and
  runs each to completion through any number of resumes: 5000 iterations took 200 s on an M1; a failure names the
  configuration.  `TXENGINE_STRESS_SEED=n` draws others.
* `conservation` checks that no unit of work is lost or done twice, by counting the nodes of a tree with no pruning.

All of these pass or the machine is not ready.  Save the output.

### 4.2 The scaling sweep

```
rust/tools/cloud/scaling.sh              # --seed-opt proofs of 1215 and 1225 at 1, 2, 4, ... 128, 192 threads; 10 to 15 minutes
rust/tools/cloud/scaling.sh --engine     # adds the engine's own benchmark (no work per node): 5 to 10 minutes
rust/tools/cloud/scaling.sh --pauses     # adds one real game at 192 threads with a checkpoint every 0.5 s
GAMES=1230 THREADS=4,16,64,192 rust/tools/cloud/scaling.sh    # a longer proof, from 4 threads up
```

Each ladder line is a search of one game at one thread count, with the optimum given as the starting incumbent
(`--seed-opt`) so that the work is the proof alone and the node count barely depends on the threads, strict tactics, and
the cap of the chain.  The tables are printed per game and for all games together.  Run it on an idle machine; the log
(`~/taxman-work/logs/scaling-*.txt`) starts with the machine and the build.

### 4.3 Reading the sweep

A table (from an M1, two thread counts, n = 1184):

```
scaling, n=1184 (speedup over 1 thread(s), times 1; * = stopped by the time limit):
  threads   search s speedup       nodes  k/s/thr  eff % idle % lock %  splits    units  steals     runs    tiny  full s
        1      1.33     1.00      167960    126.5  100.0    0.0   0.00       0        0       0        1       0   0.000
        2      0.68     1.94      164282    120.3   99.8    0.2   0.00       6       91       6       98      12   0.002
```

| column | meaning | what to look for |
|---|---|---|
| `search s` | wall time of the search; `*` if the time limit stopped it | a `*` makes that row useless; raise `SECONDS_LIMIT` |
| `speedup` | time of the first row times its threads, over this time | close to the thread count for a long game; see the next paragraph |
| `nodes` | steps taken, all threads | compare with the 1-thread row: tens of percent either way is normal (below) |
| `k/s/thr` | thousands of nodes per second per thread | the engine's efficiency apart from the tree size: it falls a little with more threads (M1, the proof of 1225: 80 to 65 from 1 to 8 threads, 19 %; the rate depends on the game: the 1-thread row above is 126.5 at n = 1184); a step down at one thread count suggests memory, frequency or NUMA |
| `eff %` | busy time / (wall x threads) | 99 to 100 up to 8 threads on the M1; below ~90 at 192 only means the game is too short, if `idle %` explains it |
| `idle %` | time workers waited for work | ramp-up and tail: they matter when the whole search takes seconds; a long game should keep it small |
| `lock %` | time waiting for the deque's lock | should stay near 0; if it reaches a few percent on long games at high counts, the engine's one shared deque is contended and per-worker deques are the next step (`src/engine.rs`, "Why one deque") |
| `splits`, `units` | times a worker handed work over, and the units those gave | grows with the thread count; the demand for work, not a defect |
| `steals` | units taken from the front of the deque by workers that had waited | |
| `runs`, `tiny` | units run, and those that polled once or never (nothing left in them) | compare the share with the rows of few threads; [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md) ("The engine") records 13 of 227 at 2 threads |
| `full s` | time until every worker was running at once | milliseconds at 8 threads (0.9 to 2.8 ms observed); expect tens of ms at 192; seconds would mean the split protocol is too slow to ramp up |

Notes on judging it:

* **Time is the measure; the speedup mixes two effects.**  Each thread keeps its own table of learned conflicts
  (nogoods) and forgets it every 50,000 of its steps, so the tree differs with the thread count: the proof of 1225 took 12.07M
  nodes on 1 thread and 9.30M on 8, and sped up 8.4x on 8 threads while the node rate sped up 6.5x
  (bench/curve/README.md).  Speedup(time) = speedup(nodes per second) x (nodes at 1 thread / nodes at T threads).
  Judge the engine by `k/s/thr` and `eff %`, the search by `nodes`, and the result by time.
* **Games must be long enough for the thread count.**  A proof of 1225 takes about 150 s on one M1 thread: at 192 threads
  that is a second, of which the ramp-up and the tail are a large part.  `eff %` below 100 at 192 threads for such a
  game is expected and says nothing about long games.  The long games are in the chain itself (n above ~1300 take minutes
  to hours at 192 threads), and its progress lines (section 5) are the real test: ask of them `eff` of 95 % or more,
  `lock` near 0, `pauses` under 1 %.
* **Memory** is the `Maximum resident set size` line near the end of the log, if GNU `time` was available.
* **Compare with the small instance**: the 1- and 8-thread rows of both should agree on `k/s/thr`.
* What would make you stop: `lock %` of several percent or `full s` in seconds at 192 threads on a long game; a `k/s/thr`
  at 192 below half that at 1 thread (the machine, not the code, is probably the limit, and a smaller instance is cheaper
  for the same work); any failed test.

### 4.4 The cost of a checkpoint

`scaling.sh --pauses` solves game 1230 (from the committed chain up to 1229) at all CPUs, strict, with a checkpoint every 0.5 s,
and prints the progress line of the end: `pauses N (x%) max Yms park Zms`.  `x` is the share of the workers' time spent
parked in checkpoints at that absurd rate; at the real interval of 60 s the share is 120 times smaller.  `max` is the
longest pause (the wait for the slowest worker, then writing and `fsync`-ing a few MB to EBS), and `park` the longest
wait for the workers to park (10 to 15 ms at most on the M1).  A `max` of a few hundred milliseconds is fine at a
60-second interval; seconds would mean slow storage, and `--checkpoint-interval` can go up (`CHECKPOINT_INTERVAL=300`).

### 4.5 Rehearsal of the chain, with interruptions

The whole production run to 1231 in miniature, in a separate work directory, with every search that lasts more than 5
seconds stopped and resumed:

```
TAXMAN_WORK=~/rehearsal EXTRA_ARGS="--time-limit 5" rust/tools/cloud/chain.sh 1231
```

It takes minutes.  The log (`~/rehearsal/logs/chain.log`) shows each pass; at the end `verify.sh` runs both referees on
the new file and the committed ones.  **Pass = the chain completes, "both referees agree", and the line `231 board sizes
are in more than one file, with equal scores in all`** (the scores of 1001..1231 equal the committed strict chain's; the
moves may differ, which is allowed).  A strict game that was stopped and resumed is no different from one that ran in
one piece (`"taint":[]`, `setup.revise` `Off`), so the chain goes on from it.  Then the two harder interruptions, by
hand, with `TAXMAN_WORK` exported as `~/rehearsal2` in both tmux windows:

```
rust/tools/cloud/chain.sh 1231                    # window 1
kill -TERM $(cat ~/rehearsal2/chain.pid)          # window 2, a few seconds in: a clean stop (chain.sh exits with 143)
rust/tools/cloud/chain.sh 1231                    # window 1: resumes where it stopped
kill -KILL $(pgrep -x taxman)                     # window 2, a few seconds in: a crash
```

After the TERM the log says "stopped by a signal (a final checkpoint was written)"; after the KILL it says "the solver
crashed (status 137)", `chain.sh` starts the solver again after 10 s, and the next pass resumes from the last
checkpoint (the KILL loses at most one checkpoint interval; the games finished are not lost).  Let it run to the end:
the same pass criteria.  Delete `~/rehearsal*` afterwards.

## 5. The production chain

Start in tmux, with a deadline and with the state copied off the machine:

```
tmux new -s tx
DEADLINE_HOURS=30 POWEROFF=always S3_DEST=s3://YOUR-BUCKET/taxman-run1 rust/tools/cloud/chain.sh 1400
```

In a second window, if spot: `rust/tools/cloud/spot-watch.sh`.  `chain.sh N` is the whole command: the strict chain 1..N
into `~/taxman-work/results/strict.jsonl`, all CPUs, a checkpoint and a progress line every 60 s.  Its variables (all
optional) are documented at the top of the script: `THREADS`, `CHECKPOINT_INTERVAL`, `PROGRESS`, `EXTRA_ARGS`,
`DEADLINE_HOURS`, `POWEROFF`, `MAX_FAILS`, `S3_DEST`, `SAVE_EVERY`.

* **Stop on purpose**: Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/chain.pid)`.  The solver writes a final
  checkpoint; `chain.sh` ends with status 143.  **Start again**: run the same command.  It skips every completed game
  and resumes the game with a checkpoint.  The thread count, `S3_DEST`, `DEADLINE_HOURS` (the first deadline stays in
  `~/taxman-work/deadline`; delete the file to set another) and N may differ between runs.
* **What it does by itself**: a crash (status other than 0, 1, 2, 124, 130 and 143) is retried after 10 s; an error
  status (1, 2: a refused checkpoint, a game that failed a check) is not, because it would repeat: read
  `~/taxman-work/logs/chain.err`.
* **Same build between a stop and its resume.**  A checkpoint records the game, the frames the build made, the
  candidates and the search tactics; one from a build that differs is refused with the reason (delete it, or run the old
  build).  Do not `git pull` in the middle of a long game without a reason.  Changing only code generation (PGO,
  `target-cpu`) is harmless.

### Monitoring

```
tail -f ~/taxman-work/logs/chain.err          # the progress lines of the game in progress
tail -n 3 ~/taxman-work/logs/chain.out        # the last finished games
tail ~/taxman-work/logs/chain.log             # this script's own messages
tail -n 1 ~/taxman-work/results/strict.jsonl | cut -c1-40        # the last completed board size
ls -l ~/taxman-work/ckpt                      # a checkpoint no older than ~2 minutes while a long game runs
htop                                          # all 192 cores near 100 %
```

A healthy progress line at 192 threads: `eff` 95 % or more, `lock` under 1 %, `idle` small, `queue` a few hundred at most,
`spilled 0`, `pauses N (x%)` with x under 1, `best` rising early and then still (the incumbent is often, not always,
found long before the proof ends).  The node rate grows with the thread count: 100 to 130 thousand per thread on the M1 at
n = 1200, so several tens of millions per second at 192 threads if the cores scale.  Estimate when a long game will end
from the previous ones: the log has each game's seconds, and the curve moves by roughly 10 per 55 to 70 in n.

### Surviving a spot interruption

1. `spot-watch.sh` sees the two-minute notice, sends SIGTERM to `chain.sh`, waits for it, and runs `save.sh`.  Without
   it, the OS shutdown signals the processes, and in the worst case you lose one checkpoint interval.
2. With `S3_DEST` set, `chain.sh` has been copying results, logs and checkpoints every 10 minutes (`save.sh`; the
   checkpoints with `--delete`, so a finished game's disappears).
3. On a replacement machine: `setup.sh` (same commit: `git checkout` it if the branch moved, or the checkpoint may be
   refused), then `S3_DEST=... rust/tools/cloud/save.sh --restore`, then `chain.sh N` again.  It refuses to restore over
   an existing results file unless `FORCE=1`.  With a separate EBS volume instead, attach it in the same availability
   zone, mount it, and set `TAXMAN_WORK`.

### Cost guardrails

* `DEADLINE_HOURS`: after that many hours from the first start, `chain.sh` stops the solver cleanly (final checkpoint) and
  ends with status 3.  `POWEROFF=done` powers the machine off when the range is complete, `POWEROFF=always` whenever
  `chain.sh` ends for any reason (needs passwordless `sudo`, as on both AMIs): the choice for an unattended run, since an
  idle machine after an error or a deadline costs as much as a busy one.  `chain.sh` saves to `S3_DEST` before it powers
  off, which matters when a shutdown terminates the instance (spot).  A stopped on-demand instance still costs for its
  volumes until you delete them.
* A backstop that does not depend on any script: `sudo shutdown -h +N` at launch (minutes).
* A budget alert; the instance tagged with an owner; the persistent spot request cancelled.
* When done: terminate the instance, delete the volumes and the S3 prefix you no longer need, cancel spot requests.

### Getting the results off the machine

Pull from your own machine (no credentials on the instance):

```
rsync -av ec2-user@HOST:taxman-work/results/ ./results-from-cloud/        # ubuntu@HOST on Ubuntu
rsync -av ec2-user@HOST:taxman-work/logs/ ./logs-from-cloud/
```

or `S3_DEST` as above and `aws s3 sync` from your laptop.  A git push from the instance needs a deploy key or a token on
it; if you do that, push the results file alone to a branch.  The results file is a valid copy at any moment (lines are
appended and synced one at a time; a torn last line is dropped when the file is read).

## 6. Verifying, and bringing the results home

`chain.sh` runs `verify.sh` when the range is complete.  On any copy of the file, anywhere:

```
rust/tools/cloud/verify.sh results-from-cloud/strict.jsonl
```

It runs `taxman verify-results` and `tools/check_results.py` on the file together with the three committed ones
(`results/default.jsonl`, `results/strict.jsonl`, `bench/curve/chain.jsonl`): every game replayed legally and finished
and opening with the largest prime, the chain bounds `score(n-1) <= score(n) <= n + score(n-1)`, each line's arithmetic and
provenance, the Franklin-Moniot bound and the best heuristic chain, and that every board size shared with a committed
file has the same score.  For a clean strict chain from 1: the report says `taint: N of N games rest on nothing unproven`.
Any other taint, or any problem line, means the run is not what it should be: stop and read the line.

Then decide what to keep.  The file is about 2 KB per game.  The committed `results/` must not be overwritten by
scripts; a new file next to them (say `results/strict-1-1400.jsonl`) and an update of `results/README.md`, and the
scaling logs under `bench/` (say `bench/cloud/`), are your call.

## 7. When something goes wrong

| symptom | cause and remedy |
|---|---|
| `error: ... is in use by another process (process N)` | a solver already uses that checkpoint directory (`pgrep -a taxman`); a lock is released when its process ends, however it ends |
| `chain.sh: another chain.sh is running` | the same, one level up; `~/taxman-work/chain.pid` names it |
| `cannot resume from ...: made with different search tactics: ...` | resume with the `--tactic` flags it prints (`EXTRA_ARGS`), or delete the checkpoint to start that game over |
| `cannot resume from ...` for another game or build | delete the named `.ckpt` file; the game restarts from its beginning |
| the solver exits with status 137 | the OOM killer or a spot reclaim; `chain.sh` retries; look at `dmesg` if it repeats (it should not: memory use is tiny) |
| `rustc ... older than 1.89` / an error about `try_lock` | `rustup update stable`; `setup.sh` does this when rustup is present |
| `optimal.json` not found | the binary looks for `src/main/resources/optimal.json` at run time, above the working directory and then above the binary (`src/paths.rs`), and neither is inside a checkout: run from inside one, or pass `--optimal FILE` or set `TAXMAN_OPTIMAL` |
| `eff` low and `idle` high on a short game at 192 threads | expected (section 4.3); judge long games |
| `lock` of several percent on long games | the shared deque is contended: report it; fewer threads is the immediate remedy (`THREADS=96`) |
| `/tmp` or `$TAXMAN_WORK` is a memory filesystem | `chain.sh` refuses: set `TAXMAN_WORK` to a directory on EBS |
| `Permission denied` installing packages | run as a user with sudo, or install git, a C compiler, python3 and curl and use `SKIP_PACKAGES=1` |
| the SSH connection dropped | the run is in tmux: reconnect, `tmux attach -t tx`; if tmux is gone the machine rebooted: `setup.sh` is not needed again, run `chain.sh N` |
| the machine rebooted | tmux and `chain.sh` are gone; the state is in `~/taxman-work`: run `chain.sh N` again |

## 8. What has not been measured

* **Anything above 8 threads.**  The engine's design (one shared deque; a demand count so that all workers are busy in
  a few rounds of splits) was tested on a toy problem at 64 and 192 threads on a 10-core machine, and on the solver at
  up to 8 threads.  The 10x to 19x assumed for 192 cores (bench/curve/README.md) comes from the design, not from data.
  Section 4.2 produces the first data.
* **Graviton5.**  The speed of its cores against the M1's, the effect of `target-cpu=native`, the cache and memory
  behaviour of 192 cores sharing a chip.
* **NUMA**, if the 48xlarge has more than one node.
* **PGO.**  `tools/pgo.sh` exists and was run on the M1 only; no gain from profile-guided builds is claimed for Graviton.
* **The scripts on Linux and on EC2.**  See the status box.  The first run on the small instance is the test.
