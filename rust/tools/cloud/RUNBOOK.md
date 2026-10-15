# Running the solver, and the prover's proofs, on a cloud machine

The target is an AWS Graviton instance of 4 to 64 vCPUs (`c8g`, `c9g`, `c7g`), running Linux; a mid-size one for
hours or days, not the biggest one (section 1, "Instance"; section 9 for the prover's proofs, which need only 4 to 16
vCPUs).  The scripts take the thread count from the machine, work on x86_64 too, and on Amazon Linux 2023 and Ubuntu.
What the solver is and how to read its output is in [../../README.md](../../README.md); this page is the procedure.
Before launching anything, read [Rules for every launch](#rules-for-every-launch).

**Status of this procedure.**  The scripts were exercised on macOS (arm64): `chain.sh` through a complete chain, a
SIGTERM stop, deliberate interruptions and resumes, a crash loop and a deadline; `scaling.sh` in all three modes (a
sweep, `--engine`, `--pauses`); `setup.sh` through its build and checks; `verify.sh`.  `prover.sh` (section 9) ran on
macOS and in the Docker image (Linux, arm64) over a range, with a SIGTERM stop and a resume; `chain.sh` also ran in the
image after the `run.sh` split.  Nothing has been run on Amazon Linux, Ubuntu or EC2; `save.sh` and `spot-watch.sh` need
S3 and EC2 and have not been run at all.  Sections 2 and 3 exist to find the problems that a macOS run cannot, on a
machine that costs cents.  Nothing about scaling beyond 8 threads or about the speed of a Graviton CPU has been
measured (section 8).

* [Rules for every launch](#rules-for-every-launch)
* [0. The plan](#0-the-plan)
* [1. Decisions](#1-decisions)
* [2. Dry run in Docker](#2-dry-run-in-docker-optional)
* [3. A small instance first](#3-a-small-instance-first)
* [4. The big instance: checks before the chain](#4-the-big-instance-checks-before-the-chain)
* [5. The production chain](#5-the-production-chain)
* [6. Verifying, and bringing the results home](#6-verifying-and-bringing-the-results-home)
* [7. When something goes wrong](#7-when-something-goes-wrong)
* [8. What has not been measured](#8-what-has-not-been-measured)
* [9. The prover's proofs of opt(n), 1301 to 2175](#9-the-provers-proofs-of-optn-1301-to-2175)

## Rules for every launch

They hold for every instance this project starts: a trial of the scripts, the prover's proofs, the chain, a
calibration.  The sections below point here instead of repeating them.

* **Approval first.**  Anything that costs money or changes the AWS account (an instance, a volume, a key pair or
  security group, a spot request, a bucket, a role) needs Brian's explicit yes, with a cost estimate, before the call.
  Reading (describe, the spot price history, quotas) does not.  The estimate is the hours at the spot price of the hour
  (read the history first, section 9.3); the timer, below, bounds the hours.
* **Spot, on `c8g`.**  Spot is the default (section 1, "Spot or on-demand", says where an interruption would ruin the
  run), on the `c8g` family (Graviton4) at the sizes the calibration settled on: `c8g.xlarge` to try the scripts,
  `c8g.4xlarge` for the prover's proofs, `c8g.16xlarge` for the chain (section 9.3).  What was measured is the entry
  "Graviton calibration (2026-10)" of [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#graviton-calibration-2026-10);
  its scripts and data are in [bench/cloud/](../../bench/cloud/README.md).
* **A timer on every instance.**  `--instance-initiated-shutdown-behavior terminate` at launch, and `shutdown -h
  +MINUTES` at boot (in the user data), with MINUTES a little past the run's own deadline (`DEADLINE_HOURS`, section
  5).  Any shutdown (the timer, `POWEROFF`, the spot service) then terminates the instance, and one that is forgotten
  ends by itself.  It is the backstop that depends on no script.
* **Tags** `Project=taxman` and `Purpose=...` (`trial`, `prover`, `chain`) on everything the launch creates that takes
  one: the instance, its volumes and spot request, a key pair, a security group.  The check at the end finds them by
  `Project`.
* **The root volume is deleted on termination**, so what must outlive the instance is copied off it first (`S3_DEST`
  or `rsync`, sections 5 and 6).
* **At the end**, once the results are home: terminate the instance, and confirm that nothing of the project is left,
  in the region of the launch (each command prints `[]`; the CLI takes its profile from `AWS_PROFILE`, and section 9.3
  shows signing in):

  ```
  aws ec2 describe-instances --filters Name=tag:Project,Values=taxman \
      Name=instance-state-name,Values=pending,running,stopping,stopped --query 'Reservations[].Instances[].InstanceId'
  aws ec2 describe-volumes --filters Name=tag:Project,Values=taxman --query 'Volumes[].VolumeId'
  aws ec2 describe-spot-instance-requests --filters Name=tag:Project,Values=taxman Name=state,Values=open,active \
      --query 'SpotInstanceRequests[].SpotInstanceRequestId'
  ```

  Then delete the key pairs and security groups the launch created, and only those (found by the same tag; a launch
  may use a key pair or security group that existed before, and leaves it alone), and report the instance-hours (launch
  to the terminate call), the spot price of the hour and the cost, as the "Cost" paragraph of the calibration entry
  does.

## 0. The plan

1. Optional, on your machine: build and test in Docker (section 2).
2. A small instance of the same family, on-demand, for an hour or two (cents): `setup.sh`, the tests, the scaling sweep
   up to its 8 threads (section 3).
3. The production instance (section 1: mid-size), on-demand, for the checks: the large-machine tests, the sweep to
   its CPU count, the cost of a checkpoint pause, and a rehearsal of the whole chain to 1231 with deliberate
   interruptions (section 4).
4. The production chain, from n = 1 or from the committed `results/strict.jsonl` (which holds it to 2175), in
   tmux, on spot or on-demand, with a deadline and a copy of the state kept off the machine (section 5).
5. Verify, bring the results home, shut everything down (section 6).

The files in this directory:

| file | what it does |
|---|---|
| `env.sh` | settings and helpers the scripts share (paths, defaults, `env_report`); sourced, not run |
| `run.sh` | what the long runs share (one at a time, log, pid, deadline, S3 saves, clean stop, power-off); sourced |
| `setup.sh` | packages, Rust, the checkout, the builds (`target-cpu=native`; the solver and the prover), smoke checks and the tests; idempotent |
| `scaling.sh` | the scaling sweep (`--engine`: the engine benchmark; `--pauses`: the cost of checkpoints; `--quick`) |
| `chain.sh N` | the production run: the chain 1..N in a loop, a window of games in flight (`WINDOW`), resumable, logged, optional deadline and power-off |
| `verify.sh [FILE...]` | both referees on a results file, and against the committed results |
| `save.sh` | copy results, logs and checkpoints to S3 (`--restore` on a fresh machine) |
| `prover.sh A B` | the prover's proofs of opt(n) for n = A..B (section 9): resumable passes, all CPUs; `--verify` checks the output |
| `spot-watch.sh` | on a spot interruption notice: stop `chain.sh` or `prover.sh` and save |
| `Dockerfile`, `Dockerfile.dockerignore` | a Linux build-and-test image (section 2) |

All state lives in one directory, `$TAXMAN_WORK` (default `~/taxman-work`): `results/strict.jsonl` (and
`exact_parity_A_B.jsonl` of `prover.sh`), `ckpt/` (the checkpoints, `taxman.lock`), `logs/`, `chain.pid`
(`prover.pid`), `deadline`.  Nothing is written inside the checkout except
`rust/target` (and `rust/bench/history.jsonl` if you run `taxman bench` without `--history`).

## 1. Decisions

### What is being computed

`chain.sh N` runs `taxman solve 1-N` into one results file: every game from n = 1, each built from the one before.
Nothing is taken from `optimal.json` (it is only compared with, to n = 1300), so nothing assumes it.  The committed
`results/strict.jsonl` is exactly such a file for n = 1 to 2175, made on a laptop
([results/README.md](../../results/README.md)): a run from n = 1 reproduces its scores (the run is checked against it by
`verify.sh`), and the production chain can instead start from it, as section 5 says, so that only the games above 2175
are new.  The solver is the same code as on a laptop; the machine only changes how long it takes.

**With the GF(2) bound** (the default since 2026-10;
[docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#the-gf2-bound-in-the-search-2026-10)): a chain from `optimal.json`'s game of 1300 reached 2175 in 92 minutes of game time on 2 threads of the M1 Max, the
dearest game 2170 at 290 s, its cost in waves (bands of 25 n from 11 s to 850 s) that follow the root's GF(2) gap,
with no growth a fit over 1301-2175 resolves.  Taking the 192-CPU machine as 43 to 86 times those 2 threads (an
assumption), a day reaches the end of the set widths, n = 4095, at every slope those data support, and even at the
steepest any GF(2) measurement has shown (×10 per 413 n), a day reaches about 3200 and a week about 3600.  How a
search of 10^5-10^6 nodes spreads over 192 threads has not been measured (to 4 threads: 2.9 times one).  What
follows is the matching path's estimate (`--tactic gf2=off --tactic lagrange=on --tactic symmetry=off`, the default
before), kept for comparison.

Cost grew by a factor of 10 for every 55 to 70 in n past 1200 ([bench/curve/README.md](../../bench/curve/README.md)).
From that file, the cost of extending the chain from 1231 to N on a 192-CPU machine, taking it to be 10 to 19 times
the 8 M1 threads of the measurements (an assumption, not a measurement), and the prices it assumes ($3.33/h spot,
$8.35/h on-demand for the 48xlarge; check the current ones).  Starting from a file that holds 1..1300 (the committed
file did when this was written; it now holds 2175), the part of the cost up to 1300 is already paid:

| N | time | spot | on-demand |
|---|---|---|---|
| 1300 | 1 to 2 minutes | cents | cents |
| 1400 | 34 minutes to 1.1 hours | $2 to $4 | $5 to $9 |
| 1450 | 3 to 6 hours | $10 to $19 | $26 to $49 |
| 1500 | 16 hours to 1.3 days | $55 to $103 | $137 to $259 |
| 1550 | 4 to 8 days | $336 to $638 | $842 to $1,600 |

These are extrapolations from a handful of points: treat them as good to a factor of several in either direction, and
let the first hours of the real run decide (the time per game is in the log; section 5).  One point has been measured
since: games 1232 to 1300 took 2866 s of search on 6 threads of the M1 Max, which another CPU-heavy job was sharing (the
committed chain of commit bd9bb5d, which `results/strict.jsonl` has replaced), about 2.5 to 5 minutes at the factors
above, more than the 1 to 2 minutes of the 1300 row (a loaded machine and 6 threads make that the pessimistic side).
The first 1300 games take minutes.  A single hard game near 1500 can take hours.  Choose N by money and patience;
rerunning `chain.sh` with a larger N later continues the same file.

### Instance

* **Workload.**  CPU-bound integer and bit-set code (no floating point, no SIMD requirement), one process, as many
  threads as CPUs; memory is tiny: about 30 MB plus 2 MB per thread were measured on an M1 (33 MB at 2 threads, 45 MB
  at 8), so a few hundred MB at 192 threads, plus 64 MiB of reserved address space per thread for stacks, which is not
  memory.  Disk I/O is one checkpoint a minute, tens of KB at 2 threads and at most a few MB at 192 (it holds the work
  stacks of all workers).  So: the cheapest vCPUs, with 1 GiB per vCPU or less.
* **A mid-size instance, 16 to 64 vCPUs** (`c8g.4xlarge` to `c8g.16xlarge`; the same sizes of `c9g` or `c7g`), for
  hours or days, rather than the largest for minutes.  Graviton has no SMT, so a vCPU is a core and each worker thread
  gets a whole one; on Intel families a vCPU is a hyperthread and the throughput per thread is lower.  The price per
  vCPU-hour is the same at every size of a family on demand and within a factor of 1.5 on spot, the largest spot pools
  are the thinnest, and the prover's proofs need no more than 16.  (A new account's vCPU quota is 5, the owner's
  account allows 640 spot vCPUs: section 9.3.)  Which family: `c8g`
  (Graviton4) is the safe default; `c9g` (Graviton5) is up to 25 % faster per vCPU by AWS's figure, 9 % dearer, and
  offered in four regions only; `c7g` (Graviton3) is the cheapest per vCPU-hour and slower; the `m` families give 4 GiB
  per vCPU, which this workload does not use, at 12 % more per vCPU.  Prices, spot interruption rates and their sources:
  section 9.3.  The solver chain's times below were estimated for 192 CPUs (they scale with the CPUs only as far as the
  engine does, which has not been measured past 8 threads).  The 192-vCPU `c9g.48xlarge` this page was first written
  for still works with every script, if you have the quota and want the chain in hours: `scaling.sh` and `chain.sh`
  take the CPU count from the machine.  On x86_64 compare the nodes per second per thread (`k/s/thr` in the sweep)
  before choosing.
* **A small instance of the same family first** (`c8g.xlarge`, 4 vCPUs, which fits the default quota; or `2xlarge`): the
  same CPU, so the same build and the same per-thread speed, for a sixteenth of the price (or less).  It is where setup
  problems on Linux, the tests, the 1-to-4-thread part of the sweep and the single-thread rate are found.
* **Check `lscpu` on a large one** for the number of NUMA nodes.  The engine does not pin threads; each searcher
  allocates its own state on its own thread, so first touch keeps it local.  If there are several nodes and the sweep
  shows a step in `k/s/thr` where the thread count crosses a node, that is the cause.
* **Build on the machine type that runs it.**  `-C target-cpu=native` is opt-in: a plain `cargo build` is generic code
  (README, section 2), and these scripts add it through `RUSTFLAGS` (the default of `env.sh`; set `RUSTFLAGS=` to build
  generic code).  It tunes for the CPU that builds; a binary built on a newer CPU can fail with an illegal instruction
  on an older one.  `setup.sh` prints the CPU that `native` resolved to (`native cpu:` line); it should be a Neoverse
  CPU name, not `generic`.  The gain of `native` has not been measured on Graviton (expect 0 to 3 %; on the M1 it
  measured 0 %).
* **Profile-guided optimization** (PGO) is optional: `rust/tools/pgo.sh` makes the build (its header says how; on the
  M1 it adds 0 to 2 % now, see [DESIGN.md](../../DESIGN.md#10-performance)).  PGO changes code
  generation only: the search and its answers are the same, a checkpoint can be resumed by either binary, and
  `TAXMAN_BIN` / `LADDER_BIN` point these scripts at another binary.  Keep a non-PGO build too if you want to profile.

### Spot or on-demand

* **On-demand for sections 3 and 4**: a spot interruption in the middle of a measurement ruins it.
* **Spot for the production chain is reasonable**, because the run is built for it: a stop by SIGTERM loses nothing, a
  kill loses at most one checkpoint interval (300 s), and `chain.sh` is safe to start again.  What an interruption
  really costs is the time to get a machine again (the spot pool of a large size can be thin) and your attention; fall back to
  on-demand, or to a smaller size (`THREADS` can differ between resumes), if you cannot.  Use `spot-watch.sh` to turn the
  two-minute notice into a clean stop and a save.
* A **persistent** spot request launches a replacement after an interruption: that is convenient and also a way to
  keep paying for something nobody watches.  If you use one, `DEADLINE_HOURS` and `POWEROFF` (section 5) are your
  protection, and the request is cancelled at the end like everything else
  ([Rules for every launch](#rules-for-every-launch)).

### Storage

Checkpoints and results go to a disk that survives a stop or reboot, with local semantics (`flock`, `fsync`): the
root EBS volume (gp3, 40 GB is plenty: the toolchain and `target/` take 3 to 4 GB, results about 2 KB per game), or a
separate EBS volume mounted somewhere and named by `TAXMAN_WORK`.  **Not tmpfs**: `/tmp` is tmpfs on Amazon Linux 2023,
and `chain.sh` refuses a memory filesystem.  **Not instance-store NVMe** as the only copy: it is lost when the instance
stops or is terminated.  Not a network filesystem either (`flock`, and an `fsync` per checkpoint).

An EBS root volume is deleted when the instance is terminated ([Rules for every launch](#rules-for-every-launch)), so
the copy that matters is the one off the machine: `S3_DEST` (section 5) or a pull with `rsync` from your laptop.

### Launch checklist (console fields)

* AMI: Amazon Linux 2023, arm64 (user `ec2-user`); or Ubuntu 22.04 or 24.04 arm64 (user `ubuntu`).
* Storage: one gp3 root volume, 40 GB.  A second, if you want the state to outlive the instance: mount it, set
  `TAXMAN_WORK` to a directory on it, and turn "delete on termination" off (it is deleted at the end, with the rest).
* Security group: SSH from your address only; nothing else inbound.  A key pair.  Require IMDSv2 (`spot-watch.sh` uses it).
* Instance role (for S3 only): list, get, put and delete on one bucket prefix, nothing else.
* Shutdown behaviour, the timer, and the tags (with a `Name`): as in [Rules for every launch](#rules-for-every-launch).
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

1. Launch it (section 1; the [rules for every launch](#rules-for-every-launch)), on-demand, with a timer of 180
   minutes.
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
5. Copy the logs home (`scp -r HOST:taxman-work/logs .`), then do what the rules say for the end: terminate the
   instance and confirm that nothing is left.

## 4. The big instance: checks before the chain

Launch the production instance (section 1) on-demand, with a timer of 240 minutes, and run `setup.sh` as above (about
the same time).  Use tmux for everything.  Count on one to one and a half hours, roughly $10 to $15 at the assumed
price.

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
(`--seed-opt`) so that the work is the proof alone and the node count barely depends on the threads, the default
tactics, and the ceiling of the chain.  The tables are printed per game and for all games together.  Run it on an idle
machine; the log (`~/taxman-work/logs/scaling-*.txt`) starts with the machine and the build.

### 4.3 Reading the sweep

A table (from an M1, three thread counts, `--seed-opt`, n = 1215, a search of a third of a second):

```
scaling, n=1215 (speedup over 1 thread(s), times 1; * = stopped by the time limit):
  threads   search s speedup       nodes  k/s/thr  µs/step  µs/tree  eff % idle % lock %  reb %  1st % ramp % tail %  splits    units  steals     runs    tiny  full s
        1      0.30     1.00        3647     12.3    63.64    63.48   99.9    0.0   0.00    0.0    0.3    0.0    0.0       0        0       0        1       0   0.000
        2      0.16     1.80        3855     11.7    64.96    62.73   96.9    3.0   0.00    0.2    3.7    2.7    0.2       2       27       2       30       4   0.007
        4      0.10     2.92        4493     11.0    66.02    62.87   91.2    8.5   0.00    0.3    4.9    6.5    0.3       4       43       8       48       4   0.007
```

| column | meaning | what to look for |
|---|---|---|
| `search s` | wall time of the search; `*` if the time limit stopped it | a `*` makes that row useless; raise `SECONDS_LIMIT` |
| `speedup` | time of the first row times its threads, over this time | close to the thread count for a long game; see the next paragraph |
| `nodes` | steps taken, all threads | compare with the 1-thread row: tens of percent either way is normal (below) |
| `k/s/thr` | thousands of nodes per second per thread | the engine's efficiency apart from the tree size: it falls a little with more threads (M1, the proof of 1225: 80 to 65 from 1 to 8 threads, 19 %; the rate depends on the game: the 1-thread row above is 12.3 at n = 1215); a step down at one thread count suggests memory, frequency or NUMA |
| `µs/step` | the workers' busy time per step (busy time over `nodes`), in µs | `k/s/thr` is about 10 x `eff %` / `µs/step` for a search that is long against its own setup, so a fall of `k/s/thr` is a rise of `µs/step` (a step costs more) or a fall of `eff %` (the workers wait); compare the rows of a game |
| `µs/tree` | the same without the units' starts: (busy time - rebuild - first nodes) over (`nodes` - units with a first node) | the cost of a step inside a unit's tree; if it stays level while `µs/step` rises, the units' starts are the loss (`reb %` and `1st %`), if it rises with `µs/step`, the step itself is dearer on more threads (caches, memory, fewer nogoods learned a thread) |
| `eff %` | busy time / (wall x threads) | 99 to 100 up to 8 threads on the M1; below ~90 at 192 only means the game is too short, if `idle %` explains it |
| `idle %` | time workers waited for work | ramp-up and tail: they matter when the whole search takes seconds; a long game should keep it small |
| `lock %` | time waiting for the deque's lock | should stay near 0; if it reaches a few percent on long games at high counts, the engine's one shared deque is contended and per-worker deques are the next step (`src/engine.rs`, "Why one deque") |
| `reb %` | time units spent rebuilding their state, from the start of a unit to its first step | part of `µs/step`, paid once a unit: it rises with the number of units, so with the threads (0.0, 0.2 and 0.3 % in the sample above) |
| `1st %` | time units spent in their first node, whose GF(2) bound is solved from scratch | the same (0.3, 3.7 and 4.9 % above); if `reb %` and `1st %` together are most of the rise of `µs/step` from 1 thread, the units' starts are the loss |
| `ramp %` | idle time before all workers first ran at once | a part of `idle %`: the start of the search, which a short game at many threads is mostly made of |
| `tail %` | idle time after the last moment a worker finished a unit while every worker was running | a part of `idle %`: the end of the search, when the work runs out |
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

`scaling.sh --pauses` solves game 1230 (from the committed chain up to 1229) at all CPUs, with a checkpoint every 0.5 s,
and prints the progress line of the end: `pauses N (x%) max Yms park Zms`.  `x` is the share of the workers' time spent
parked in checkpoints at that absurd rate; at the real interval of 300 s the share is 600 times smaller.  `max` is the
longest pause (the wait for the slowest worker, then writing and `fsync`-ing a few MB to EBS), and `park` the longest
wait for the workers to park (10 to 15 ms at most on the M1).  A `max` of a few hundred milliseconds is fine at a
300-second interval; seconds would mean slow storage, and `CHECKPOINT_INTERVAL` can go up further.

### 4.5 Rehearsal of the chain, with interruptions

The whole production run to 1231 in miniature, in a separate work directory, with every search that lasts more than 5
seconds stopped and resumed:

```
TAXMAN_WORK=~/rehearsal EXTRA_ARGS="--time-limit 5" rust/tools/cloud/chain.sh 1231
```

It takes minutes.  The log (`~/rehearsal/logs/chain.log`) shows each pass; at the end `verify.sh` runs both referees on
the new file and the committed one.  **Pass = the chain completes, "both referees agree", and the line `1231 board sizes
are in more than one file, with equal scores in all`** (the scores of 1..1231 equal the committed chain's; the moves may
differ, which is allowed).  A game that was stopped and resumed is no different from one that ran in one piece
(`"assumes":[]`), so the chain goes on from it.  Then the two harder interruptions, by hand, with `TAXMAN_WORK` exported
as `~/rehearsal2` in both tmux windows:

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

In a second window, if spot: `rust/tools/cloud/spot-watch.sh`.  `chain.sh N` is the whole command: the chain 1..N
into `~/taxman-work/results/strict.jsonl`, all CPUs, sixteen games in flight at once, a checkpoint every 300 s and a
progress line every 60 s.  Its variables (all optional) are documented at the top of the script: `THREADS`, `WINDOW`,
`CHECKPOINT_INTERVAL`, `PROGRESS`, `EXTRA_ARGS`, `DEADLINE_HOURS`, `POWEROFF`, `MAX_FAILS`, `S3_DEST`, `SAVE_EVERY`.
`WINDOW` is the number of games in flight at once, which share the threads (`taxman solve --window`): 1 solves the
games one after the other, and the scores and proofs are the same for every value; what to set it to on 64 or 192
threads is measured in items 7 and 8 of [bench/wall/README.md](../../bench/wall/README.md#for-a-cloud-machine).

* **Where the chain starts.**  From an empty `~/taxman-work`, `chain.sh` solves from n = 1, which reproduces the
  committed `rust/results/strict.jsonl` (n = 1 to 2175; the scores equal, the moves may differ) in minutes on the big
  machine, and `verify.sh` then checks the machine and the build against it.  To skip those games, copy the committed
  file in before the first run, once:

  ```
  mkdir -p ~/taxman-work/results && cp rust/results/strict.jsonl ~/taxman-work/results/strict.jsonl
  rust/tools/cloud/chain.sh 2300                    # skips the 2175 games it holds and solves 2176 on
  ```

  The file is checked when `chain.sh` opens it, assumes nothing (a chain from n = 1), and its lines
  keep the build that made them (commit feba6d0), as the lines added will keep the cloud's.  It was made in one
  invocation and holds no incomplete line.  The comparison of `verify.sh` with the committed file is then
  trivial for 1..2175, so the games above 2175 are checked by the referees alone (each game replayed, the chain
  bounds, Franklín–Moniot, the heuristic chain); a run from n = 1 is the cross-check of the machine and the build.  On
  a replacement machine, restore the saved state instead (below), never the committed file over it.

* **Stop on purpose**: Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/chain.pid)`.  The solver writes a final
  checkpoint; `chain.sh` ends with status 143.  **Start again**: run the same command.  It skips every completed game
  and resumes the game with a checkpoint.  The thread count, `S3_DEST`, `DEADLINE_HOURS` (the first deadline stays in
  `~/taxman-work/deadline`; delete the file to set another) and N may differ between runs.
* **What it does by itself**: a crash (status other than 0, 1, 2, 124, 130 and 143) is retried after 10 s; an error
  status (1, 2: a refused checkpoint, a game that failed a check) is not, because it would repeat: read
  `~/taxman-work/logs/chain.err`.
* **Same build between a stop and its resume.**  A checkpoint records the game, the frames the build made, the
  candidates and the search tactics; one from a build that differs is refused with the reason (delete it, or run the old
  build).  The width of the sets is part of that: a checkpoint of a game of 2048 to 3583 made at 64, before the widths
  of 36, 40, 48 and 56 words were added, is refused by a build that has them.  Do not `git pull` in the middle of a
  long game without a reason.  Changing only code generation (PGO, `target-cpu`) is harmless.

### Monitoring

```
tail -f ~/taxman-work/logs/chain.err          # the progress lines of the game in progress
tail -n 3 ~/taxman-work/logs/chain.out        # the last finished games
tail ~/taxman-work/logs/chain.log             # this script's own messages
tail -n 1 ~/taxman-work/results/strict.jsonl | cut -c1-40        # the last completed board size
ls -l ~/taxman-work/ckpt                      # a checkpoint no older than ~2 minutes while a long game runs
htop                                          # all 192 CPUs near 100 %
```

A healthy progress line at 192 threads: `eff` 95 % or more, `lock` under 1 %, `idle` small, `queue` a few hundred at most,
`spilled 0`, `pauses N (x%)` with x under 1, `best` rising early and then still (the incumbent is often, not always,
found long before the proof ends).  The node rate grows with the thread count: with the GF(2) bound 7 to 9 thousand
per thread on the M1 at n = 1200-1300 and about 3 thousand past 2000 (the matching path's was 100 to 130 thousand, in
trees a hundred times larger), so about a million per second at 192 threads if the CPUs scale.  Estimate when a long
game will end from the previous ones: the log has each game's seconds; past 1300 they come in waves of a few hundred
n, not on a smooth curve.

### Surviving a spot interruption

1. `spot-watch.sh` sees the two-minute notice, sends SIGTERM to `chain.sh`, waits for it, and runs `save.sh`.  Without
   it, the OS shutdown signals the processes, and in the worst case you lose one checkpoint interval.
2. With `S3_DEST` set, `chain.sh` has been copying results, logs and checkpoints every 10 minutes (`save.sh`; the
   checkpoints with `--delete`, so a finished game's disappears), at the lowest CPU and I/O priority (`nice -n 19`,
   `ionice -c 3` on Linux), so that a save (about 1.5 s of CPU) never competes with the workers.
3. On a replacement machine: `setup.sh` (same commit: `git checkout` it if the branch moved, or the checkpoint may be
   refused), then `S3_DEST=... rust/tools/cloud/save.sh --restore`, then `chain.sh N` again.  It refuses to restore over
   an existing results file unless `FORCE=1`.  With a separate EBS volume instead, attach it in the same availability
   zone, mount it, and set `TAXMAN_WORK`.

### Cost guardrails

* `DEADLINE_HOURS`: after that many hours from the first start, `chain.sh` stops the solver cleanly (final checkpoint) and
  ends with status 3.  `POWEROFF=done` powers the machine off when the range is complete, `POWEROFF=always` whenever
  `chain.sh` ends for any reason (needs passwordless `sudo`, as on both AMIs): the choice for an unattended run, since an
  idle machine after an error or a deadline costs as much as a busy one.  `chain.sh` saves to `S3_DEST` before it powers
  off, which matters because a shutdown terminates the instance.
* The timer (a backstop that depends on no script), the tags and the cleanup are in
  [Rules for every launch](#rules-for-every-launch); a budget alert on the account is the other half.
* When done: what the rules say for the end, and the S3 prefix you no longer need.

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

It runs `taxman verify-results` and `tools/check_results.py` on the file together with the committed
`results/strict.jsonl`: every game replayed legally and finished and opening with the largest prime, its score
confirmed against `optimal.json`, the chain bounds `score(n-1) <= score(n) <= n + score(n-1)`, each line's arithmetic
and provenance, the Franklin-Moniot bound and the best heuristic chain, and that every board size shared with the
committed file has the same score.  For a chain from 1: the report says `assumptions: none in N of N games`.  Any
assumption, or any problem line, means the run is not what it should be: stop and read the line.  (A count of
incomplete lines is not a problem: every search that was stopped and resumed left one.)

Then decide what to keep.  The file is about 2 KB per game.  The committed `results/` must not be overwritten by
scripts; a new file next to them (say `results/strict-1-1400.jsonl`) and an update of `results/README.md`, and the
scaling logs under `bench/` (say `bench/cloud/`), are your call.

To put the new games into the table of optimal games, run `taxman extend-optimal` on the verified file from a checkout:
`taxman extend-optimal results-from-cloud/strict.jsonl --out /tmp/optimal.json`.  It appends the games past the table's
end that the file completed, and says where it stopped; check the result with `taxman verify-optimal /tmp/optimal.json`
and `diff`, then copy it over `src/main/resources/optimal.json` and rebuild: the binary carries the table it was built
with.

## 7. When something goes wrong

| symptom | cause and remedy |
|---|---|
| `error: ... is in use by another process (process N)` | a solver already uses that checkpoint directory (`pgrep -a taxman`); a lock is released when its process ends, however it ends |
| `error: FILE is in use by another process (another run appending to it?): it is locked` | a solver already appends to that results file (`--out`); the same remedy |
| `chain.sh: another chain.sh is running` | the same, one level up; `~/taxman-work/chain.pid` names it |
| `cannot resume from ...: made with different search tactics: ...` | resume with the `--tactic` flags it prints (`EXTRA_ARGS`), or delete the checkpoint to start that game over |
| `cannot resume from ...` for another game or build | delete the named `.ckpt` file; the game restarts from its beginning |
| the solver exits with status 137 | the OOM killer or a spot reclaim; `chain.sh` retries; look at `dmesg` if it repeats (it should not: memory use is tiny) |
| `rustc ... older than 1.89` / an error about `try_lock` | `rustup update stable`; `setup.sh` does this when rustup is present |
| `eff` low and `idle` high on a short game at 192 threads | expected (section 4.3); judge long games |
| `lock` of several percent on long games | the shared deque is contended: report it; fewer threads is the immediate remedy (`THREADS=96`) |
| `/tmp` or `$TAXMAN_WORK` is a memory filesystem | `chain.sh` refuses: set `TAXMAN_WORK` to a directory on EBS |
| `Permission denied` installing packages | run as a user with sudo, or install git, a C compiler, python3 and curl and use `SKIP_PACKAGES=1` |
| the SSH connection dropped | the run is in tmux: reconnect, `tmux attach -t tx`; if tmux is gone the machine rebooted: `setup.sh` is not needed again, run `chain.sh N` |
| the machine rebooted | tmux and `chain.sh` are gone; the state is in `~/taxman-work`: run `chain.sh N` again |

## 8. What has not been measured

* **Anything above 8 threads.**  The engine's design (one shared deque; a demand count so that all workers are busy in
  a few rounds of splits) was tested on a toy problem at 64 and 192 threads on a 10-CPU machine, and on the solver at
  up to 8 threads.  The 10x to 19x assumed for 192 CPUs (bench/curve/README.md) comes from the design, not from data.
  Section 4.2 produces the first data.
* **Graviton5.**  The speed of its CPUs against the M1's, the effect of `target-cpu=native`, the cache and memory
  behaviour of 192 CPUs sharing a chip.
* **NUMA**, if the 48xlarge has more than one node.
* **PGO.**  `tools/pgo.sh` exists and was run on the M1 only; no gain from profile-guided builds is claimed for Graviton.
* **The scripts on Linux and on EC2.**  See the status box.  The first run on the small instance is the test.

## 9. The prover's proofs of opt(n), 1301 to 2175

The committed chain (`results/strict.jsonl`) reaches 2175, and the prover in `rust/prover` has proven opt(n) = V(n)
without the frame model only to 1300 ([the prover's README, section 3](../../prover/README.md#3-the-proofs-of-optn)).
`prover.sh` runs the same program, `exact`, over 1301..2175 on a cloud machine.  It is a small job next to the
chain: about 140 thread-hours (100 to 250), half a day on 16 vCPUs, for a few dollars.  The estimate rests on
measurements on a laptop and on the committed data of the 1..1300 proofs, not on a Graviton CPU; a separate
calibration (`rust/bench/cloud/`, when it exists) measures the prover and the solver per CPU on Graviton3, 4 and 5 spot
instances, and its figures replace the milliseconds per node and the factor of 1.0 to 1.6 below.

### 9.1 How `exact` uses the CPUs, and what the script does about it

* **One game per thread.**  `exact` hands the games of a range to its worker threads one at a time; each game's search
  is a single thread (the prover has no parallel search).  So the CPUs are used perfectly as long as there are
  games left, and the wall time cannot be shorter than the dearest game.  Measured on 12 games of 1302-1364 at 1, 2, 4
  and 8 threads (M1 Max, other jobs running): the same 275,105 nodes at every thread count (they do not depend on it),
  606, 312, 171 and 151 s of wall time; the user time per node 2.19, 2.23, 2.31 and 2.46 ms (+12 % at 8 threads, a busy
  laptop); the 8-thread run was bounded by its longest game (141 to 151 s), not by the threads
  ([EXPERIMENTS.md](../../docs/EXPERIMENTS.md#the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10)).
* **Passes, not a signal.**  A search that is still going when `--seconds` run out pauses: its open subtrees and learned
  conflicts go to `ckpt/exact_N.ckpt`, and a rerun resumes it (the tree is the same: node counts are identical to an
  uninterrupted run).  Checkpoints are written only then, and `exact` has no signal handler, so a kill or a SIGTERM
  loses the searches of the pass in progress back to their last checkpoint.  `prover.sh` therefore runs `exact` in
  passes of `PASS_SECONDS` (default 300) and starts it again until every game has a line: an interruption costs at
  most one pass of the games in flight (the restart of a pass costs about a second).
* **Order.**  The games are given dearest first, by the nodes the solver's search of each took in the committed chain
  (`summarize.py order`; the proofs of `exact` take 1 to 3 times as many, section 9.2), so that the long games start
  at once instead of last.  The order is made once and kept in `$TAXMAN_WORK/prover-order_A_B.txt`.
* **The tail.**  The last games run alone, so at the end most of a big machine idles; the log says so
  ("N games are left for T threads").  This is why more than 16 to 32 vCPUs buys nothing (section 9.2).  If the tail
  matters to you, stop (Ctrl-C), start a smaller instance, restore (`save.sh --restore`) and run `prover.sh A B` again
  there: `THREADS` may differ between runs.
* **What would make `exact` use many CPUs on one game** is not needed here: its open units are independent subtrees
  (`Prover::run_units` takes a list), so a `--shard i/K` that gives a process every K-th unit of a game's checkpoint
  would split a long game over processes.  Not done; no proving code was changed for this run.

### 9.2 What it costs

The cost of a game follows the nodes of its search.  The solver's own search of the same game, which the committed chain
records, predicts it (the root's parity gap, which predicted the cost over 900-1300, does not here: a game of gap 364
took 18 thousand nodes and one of gap 315 300 thousand).  Over 34 games of 1302-2124 that `exact` finished, it took 0.13
to 13 times the nodes of the solver's search (of the source game's, for a game whose search the solver repeated), the
ratio growing about tenfold per 1000 in n: geometric mean 0.97 at 1300-1500, 1.6 at 1500-1800, 3.8 at 1800-2124, with a
scatter of 0.28 decades; `exact` has the weaker bound, the parity one.  Five dear games were stopped unfinished at 1.1
to 2.2 million nodes (1430, 1472, 1475, 2052, 2170).  The committed proofs of 1..1300 cost 1.2 to 1.7 ms a node on two
quiet laptop threads (1.73 at 1200-1300); the sample at 1300-1500 on a busy machine, 2.2 to 2.5 (user time; 8 threads in
parallel add 12 %), and the node hardly changes with n in that range (above it nothing clean was measured).  490 of the
875 games need a search (the rest are decided by the ceiling or the prime formula).  With the fitted ratio and 2.5 ms a
node:

| | |
|---|---|
| nodes, all 490 searches | about 200 million (150 to 300) |
| thread-hours at the laptop's speed | **about 140** (100 to 250; 115 at 2.0 ms a node, 170 at 3.0) |
| the dearest games | 2170-2173 (gap 767; the model says 8 million nodes each, 6 hours; 2170 had done 1.25 million nodes after 90 minutes), then 1475, 1430, 1472, 1521 (1.7 to 2.5 million, 1.2 to 1.7 hours), 1849-1863, 2160 |

Wall time and price by size, by list scheduling over costs drawn from that model (the order is `summarize.py order`),
at the laptop's speed and at 1.6 times as long (a Graviton CPU against an M1 Max CPU: not measured; the first pass
of the real run gives it, as milliseconds per node in `prover.err`).  Add 10 % for restarts.  Spot prices are the
cheapest zone of us-east-1 in the owner's account's spot price history on 2026-10-14 (4 and 8 vCPUs: Vantage, the
same day); on demand is Vantage's Linux price in US East (`c8g`: $0.0399 per vCPU-hour at every size):

| vCPUs | shape | wall | spot $/hour | spot total | on-demand total |
|---|---|---|---|---|---|
| 4 | `c8g.xlarge` | 36 to 57 h | 0.078 | $2.8 to 4.4 | $5.8 to 9.1 |
| 8 | `c8g.2xlarge` | 18 to 28 h | 0.158 | $2.8 to 4.4 | $5.7 to 8.9 |
| **16** | `c8g.4xlarge` | **11 to 17 h** | 0.2391 | **$2.6 to 4.1** | **$7.0 to 10.8** |
| 32 | `c8g.8xlarge` | 11 to 17 h | 0.4134 | $4.5 to 7.0 | $14 to 22 |
| 64 | `c8g.16xlarge` | 11 to 17 h | 0.6794 | $7.5 to 11.6 | $28 to 43 |
| 192 | `c8g.48xlarge` | 11 to 17 h | 1.9939 | $22 to 34 | $84 to 130 |

From 16 vCPUs up the dearest games set the wall time, not the CPUs: 2170-2173 start at once and the longest of them
decides (6 to 24 hours over the draws, at the laptop's speed), and the idle CPUs are paid for.

### 9.3 Which instance, and the quota

* **Quota.**  AWS's documented default for a new account is 5 vCPUs for Standard (A, C, D, H, I, M, R, T, Z) On-Demand
  instances and 5 for Standard Spot requests, per region, raised automatically with use
  ([EC2 instance type quotas](https://docs.aws.amazon.com/ec2/latest/instancetypes/ec2-instance-quotas.html),
  [On-Demand](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-on-demand-instances.html#ec2-on-demand-instances-limits),
  [Spot](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-spot-limits.html)): a 4-vCPU shape fits, 8 does not.
  The owner's account, read with the AWS CLI in us-east-1 on 2026-10-14, allows **1280 vCPUs On-Demand and 640 Spot**,
  so any shape here launches without a request.  To read it again, in the console (Service Quotas, Amazon EC2, the region
  at the top right, "Applied quota value" of "Running On-Demand Standard (A, C, D, H, I, M, R, T, Z) instances" and "All
  Standard (A, C, D, H, I, M, R, T, Z) Spot Instance Requests") or:

  ```
  aws login --profile bvc                      # short-lived credentials; or export AWS_PROFILE=bvc after it
  aws service-quotas get-service-quota --profile bvc --region us-east-1 --service-code ec2 --quota-code L-1216C47A --query Quota.Value  # on-demand
  aws service-quotas get-service-quota --profile bvc --region us-east-1 --service-code ec2 --quota-code L-34B43A08 --query Quota.Value  # spot
  ```
* **Price per vCPU-hour** (Linux, US East; on demand from Vantage's pages of 2026-10-14, spot from the owner's price
  history of us-east-1 on 2026-10-14, cheapest zone, divided by the vCPUs; spot moves by the hour, look at the price
  history before launching):

  | family | CPU | on demand | spot: 16 vCPUs | 32 | 64 | 96 | 192 |
  |---|---|---|---|---|---|---|---|
  | `c7g` | Graviton3, 2.5 GHz | $0.0363 | 0.0126 | 0.0123 | **0.0097** | | |
  | `c8g` | Graviton4, 2.7 GHz | $0.0399 | 0.0149 | 0.0129 | 0.0106 | **0.0095** | 0.0104 |
  | `c9g` | Graviton5, 3.3 GHz | $0.0435 | | | 0.0119 | | 0.0123 |
  | `m8g` | Graviton4, 4 GiB a vCPU | $0.0449 | | | | | |

  Within a family the price per vCPU on demand is the same at every size and on spot it varies by 1.5 times between
  sizes, so the size is chosen by wall time, not by price.  `c9g` is up to 25 % faster than `c8g` per vCPU (AWS's
  [announcement](https://aws.amazon.com/about-aws/whats-new/2026/06/ec2-c9g-c9gd-instances-graviton5-processors-available/),
  a maximum, not measured here) for 9 % more, and is offered in us-east-1, us-east-2, us-west-2 and eu-central-1 only;
  `c7g` is slower per CPU; the `m` families give memory the run does not use (a thread needs about 25 to 50 MB).
* **Spot interruptions.**  The Spot Instance Advisor's rate for us-east-1 (the
  [public data](https://spot-bid-advisor.s3.amazonaws.com/spot-advisor-data.json) behind the
  [advisor page](https://aws.amazon.com/ec2/spot/instance-advisor/), fetched 2026-10-14; the share of instances
  interrupted in the trailing month) is `c8g`: 4 vCPUs above 20 %, 8 10 to 15 %, 16 above 20 %, 32 5 to 10 %, 64 5 to 10 %,
  96 above 20 %, 192 10 to 15 %; `c7g`: 16 and 32 under 5 %, 64 5 to 10 %; `c9g` is not listed yet.  At even 20 % a month
  a run of 12 hours has a chance of about one in 300 if the rate is uniform, and an interruption costs one pass and
  a few minutes (section 9.5), so spot is fine; on demand costs about $4 to $7 more.
* **The recommendation: `c8g.4xlarge` (16 vCPUs), spot or on demand, about half a day.**  `c8g.2xlarge` (8) takes a day
  and costs the same.  Nothing larger buys time, since the dearest games set it; once the calibration has measured the
  Graviton CPUs, `c9g` (or `c7g` at 64 vCPUs, the cheapest spot per vCPU) may beat it on price per unit of work.
  Storage: one gp3 root volume of 20 GB (the build of the prover alone is under 2 GB; the output is 200 KB and the
  checkpoints a few MB).  The launch checklist of section 1 applies, with the instance role only if you use `S3_DEST`.

### 9.4 Run

Launch, under the [rules for every launch](#rules-for-every-launch) (the console, section 1's checklist: Amazon Linux
2023 arm64, `ec2-user`, the timer and tags, a security group with SSH from your address; or the CLI, an example whose
AMI parameter you can check first with `aws ssm get-parameter --name ...`; the timer is 49 hours, a little past the
`DEADLINE_HOURS=48` below):

```
tags='{Key=Project,Value=taxman},{Key=Purpose,Value=prover}'
aws ec2 run-instances --profile bvc --region us-east-1 --instance-type c8g.4xlarge \
    --image-id resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64 \
    --key-name YOUR-KEY --security-group-ids sg-YOURS \
    --block-device-mappings 'DeviceName=/dev/xvda,Ebs={VolumeSize=20,VolumeType=gp3,DeleteOnTermination=true}' \
    --instance-initiated-shutdown-behavior terminate --user-data $'#!/bin/bash\nshutdown -h +2940' \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=taxman-prover},$tags]" \
        "ResourceType=volume,Tags=[$tags]" "ResourceType=spot-instances-request,Tags=[$tags]" \
    --instance-market-options 'MarketType=spot'      # on demand: leave out this line and the spot request's tags
```

On the instance (a copy of the repository with these scripts: a pushed branch or `rsync`, section 3):

```
sudo dnf install -y git tmux
git clone https://github.com/bvchess/taxman.git && cd taxman           # BRANCH=... for another branch
PROVER_ONLY=1 rust/tools/cloud/setup.sh                              # a few minutes: the prover, its checks
tmux new -s pv
DEADLINE_HOURS=48 POWEROFF=always S3_DEST=s3://YOUR-BUCKET/prover1 rust/tools/cloud/prover.sh 1301 2175
```

`S3_DEST` needs an instance role that may write that prefix.  It is not optional with `POWEROFF`: the instance is
terminated when it shuts down, and its root volume with it (the rules above), so `prover.sh` copies the results to S3
first.  Without `S3_DEST`, leave `POWEROFF` out and pull the files yourself (below) before the timer ends.
`POWEROFF=always` shuts the machine down whenever the script ends (all 875 games and the check, the deadline, an error:
an idle machine costs as much as a busy one; `POWEROFF=done` only when the range is complete), and `DEADLINE_HOURS`
stops the run cleanly after that many hours.  On spot, run
`rust/tools/cloud/spot-watch.sh` in a second tmux window.  `THREADS` defaults to every vCPU.

**Before the full run, a rehearsal on the instance itself** (minutes, cents): a small range, a SIGTERM and a resume, and
the check, in a work directory of its own.  It replaces a dry run on a laptop.

```
export TAXMAN_WORK=~/rehearsal THREADS=2 PASS_SECONDS=20
rust/tools/cloud/prover.sh 1301 1310 &              # makes the order, then passes of 20 s
sleep 40; kill -TERM $(cat ~/rehearsal/prover.pid)      # a stop in the middle: "ended: stopped", status 143
wait; ls ~/rehearsal/ckpt                               # exact_N.ckpt: the searches that were paused
rust/tools/cloud/prover.sh 1301 1310                  # resumes ("N searches paused"), finishes, runs the check
OUT=~/rehearsal/results/exact_parity_1301_1310.jsonl rust/tools/cloud/prover.sh --verify 1301 1311   # must FAIL
unset TAXMAN_WORK THREADS PASS_SECONDS; rm -rf ~/rehearsal
```

The second command must end with `OK: opt(n) is proven equal to the recorded score for every n in 2..1310` and exit
status 0, and the last command must print `without a line: 1311` and `FAILED` (on a laptop the five searches among those
ten games take about 3 minutes of one thread; their node counts are the same whatever the stops).  The `ms/node` of the
real CPUs is also here: `secs` over `nodes` of the lines of `~/rehearsal/results/exact_parity_1301_1310.jsonl`.

First minutes of the full run: `prover.sh` prints the machine, makes the order, and logs `pass 1: 0 of 875 games done`.
In `~/taxman-work/logs/prover.err` the lines `exact: n=... paused after N nodes, U open units` give the nodes a long
game has done; divide the seconds of a pass by the nodes of one pass for milliseconds per node (the laptop: 2.2 to 2.5).

### 9.5 Watch, stop, resume, the tail

```
tail -f ~/taxman-work/logs/prover.log            # a line a pass: games done, searches paused, the exit status
tail -n 20 ~/taxman-work/logs/prover.err         # exact's lines: finished games and paused searches with their nodes
grep -c . ~/taxman-work/results/exact_parity_1301_2175.jsonl      # games done, of 875
ls -l ~/taxman-work/ckpt                         # a checkpoint per unfinished search, rewritten every pass
```

* **Stop**: Ctrl-C in tmux or `kill -TERM $(cat ~/taxman-work/prover.pid)`; the searches of the pass in progress go
  back to their last checkpoint.  **Resume**: the same command (`THREADS`, `S3_DEST` may differ).
* **Spot interruption**: `spot-watch.sh` stops it and saves; on a replacement machine run `PROVER_ONLY=1 setup.sh`, then
  `S3_DEST=... rust/tools/cloud/save.sh --restore`, then the same `prover.sh` command.  Without a saved copy, the
  results file and checkpoints are on the root volume, which a spot termination deletes: use `S3_DEST`, or `rsync`
  the work directory home from time to time.
* **An error stop** (exit status 1): `exact` panicked.  Read the end of `prover.err`.  A message
  "a playable set worth X beats the recorded Y" is a counterexample: a better game than the committed chain's
  for that n.  Keep `~/taxman-work` as it is, the set is in the message, and tell the author; it is the finding
  this run exists to look for.  "does not replay to its score" means the file or the build is wrong.
* **The tail**: toward the end only the dearest games are left (the log says how many, against the threads).  Let them
  finish; with 16 vCPUs the idle CPUs cost cents an hour.

### 9.6 Bring the data home, verify, commit

```
export AWS_PROFILE=bvc                                                    # after: aws login --profile bvc
aws s3 cp s3://YOUR-BUCKET/prover1/results/exact_parity_1301_2175.jsonl rust/prover/data/     # or, without S3:
rsync -av ec2-user@HOST:taxman-work/results/exact_parity_1301_2175.jsonl rust/prover/data/
rsync -av ec2-user@HOST:taxman-work/logs/ ./prover-logs/
OUT=rust/prover/data/exact_parity_1301_2175.jsonl rust/tools/cloud/prover.sh --verify 1301 2175
```

The check (`summarize.py check 1301 2175`, which `prover.sh` already ran at the end) passes only if every n in
2..2175 is proven (a line for each n of 1301..2175, and with `data/exact_parity_2_1300.jsonl` each proven by a bound,
a search to completion, or the ceiling or the prime formula from a proven predecessor) and every line's score is the
recorded optimum of `results/strict.jsonl`.  It exits 0 and prints `OK: opt(n) is proven equal to the recorded score
for every n in 2..2175`; anything else prints which n.  Then, on your machine:

```
python3 rust/prover/tools/summarize.py exact rust/prover/data/exact_parity_2_1300.jsonl rust/prover/data/exact_parity_1301_2175.jsonl
python3 rust/prover/tools/summarize.py cost rust/prover/data/exact_parity_1301_2175.jsonl      # the cost table by band
```

Commit `rust/prover/data/exact_parity_1301_2175.jsonl` (about 200 KB, never overwritten by a later run: `OUT` names
another file).  Do not commit the work directory, the checkpoints or the logs.  Then update the three places that say
the prover's proofs stop at 1300 or are pending: the table and sections 3 and 8 of
[../../prover/README.md](../../prover/README.md), the "pending" sentence of the entry "The committed chain to 2175" and
the register row of [../../docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md), with the totals from the two commands above
and the real milliseconds per node of a Graviton CPU (this section's table is a prediction).  Then the end of the
[rules for every launch](#rules-for-every-launch): terminate the instance and confirm that nothing is left.
