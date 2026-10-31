# The prover's proofs of opt(n) (`prover.sh`)

Part of the [runbook](../RUNBOOK.md).

`prover.sh A B` proves opt(n) = V(n), the score that `results/optimal.jsonl.gz` records, for every n from A to B, on the
whole board.  It runs the prover's `exact` over the range in passes on every CPU, and it is a small job next to the
chain.  It runs on EC2 ([Run](#run)) or on a machine of your own ([On a machine of your
own](#on-a-machine-of-your-own)).  [The prover's proofs](../../noframe/docs/proofs.md) holds the proofs made so far and
their cost.

**A run searches from the roles of the bottom-layers theorem.**  `exact` starts there by default (`--fix-low 1`), and
the search is still a proof, because every optimal set obeys the roles (the bottom-layers theorem
(F4)).  Each line records `"fix_low":1`, and the check
counts the proofs that rest on the theorem.  Over 2..999 the roles took the plain root's nodes within 5 % at half the
time a node, 0.61 ms against 1.16 ms (proofs.md, the paragraph "From the roles of F4"),
so by that measurement a run from the roles costs about half as much as one from the plain root.  `EXTRA_ARGS='--fix-low
0'` searches from the plain root.  The two kinds of search keep separate checkpoints: `ckpt/exact_N_low.ckpt` from the
roles, `ckpt/exact_N.ckpt` from the plain root.

## How `exact` uses the CPUs

Every thread stays busy until the last game is proven: a game's search spreads over the threads that have no game of
their own to start.

* **A game a thread, then several threads a game.**  `exact` hands the games of the range to its threads one at a
  time.  When fewer games are left than threads, a thread with nothing to do helps a search still running: it takes the
  largest open subtree that another thread of that game holds, and searches it.  A subtree that moves starts cold, its
  first node solving its bounds from scratch, so subtrees move only to an idle thread.  The game's line then counts
  its threads: `nodes` adds them up, `secs` is its wall time, `thread_secs` its threads' seconds added up, and
  `threads` the most threads it ran on at once.  A search on one thread takes the same nodes as before.  One on several
  threads takes more or fewer, since each thread keeps nogoods of its own, and how many depends on the timing.  [The
  prover's proofs](../../noframe/docs/proofs.md#a-game-on-several-threads) has the speedup measured.
* **Passes.**  A search still running when a pass ends (`--seconds`) pauses: it saves the open subtrees of all its
  threads, and its learned conflicts, to its checkpoint, and the next pass resumes it on any number of threads.  `exact`
  writes checkpoints only then, and it has no signal handler, so a kill or a SIGTERM sends the searches of the pass in
  progress back to their last checkpoints.  `prover.sh` therefore runs `exact` in passes of `PASS_SECONDS` until every
  game has a line.  The end of a pass costs a little: every open subtree restarts cold in the next pass, and every game
  in flight is set up again.  So the default, 3600 s, suits a machine that nothing interrupts, and 300 s suits spot,
  where an interruption costs at most one pass of every thread.
* **The dearest games first.**  The games go in the order of the nodes that the solver's search of each took in the
  committed chain (`summarize.py order`), so that the long games start at once.  The order is made once and kept in
  `$TAXMAN_WORK/prover-order_A_B.txt`.  A game that repeats the search of a game settled at its ceiling records no
  nodes, so it goes late.
* **The log.**  `exact` writes a line on stderr (`prover.err`) for each game it proves after a search, with its nodes,
  the most threads it ran on, and the subtrees that moved between threads (`units moved`), and one for each search a
  pass pauses, with its open subtrees.

## What it costs

A run costs the thread-hours of its searches, spread over the vCPUs, the last searches included: a search on up to 4
threads took about the thread-hours of one thread ([the prover's
proofs](../../noframe/docs/proofs.md#a-game-on-several-threads)).  Many games need no search: those at their ceiling
and those that meet the prime formula or the semiprime formula.

For the estimate that the account's owner approves, price the range from the cost by bands of 100 in n in [the
prover's proofs](../../noframe/docs/proofs.md#to-2175-on-a-cloud-machine): the thread-hours, halved for the roles where
the bands are from the plain root, over the vCPUs, with the idle tail, at the spot price of the hour
([machine.md](machine.md)).  Past the last band, [taxman-notebook/agenda/AGENDA.md, "The diagnosis of the
wall"](https://github.com/bvchess/taxman-notebook/blob/main/agenda/AGENDA.md#2-the-diagnosis-of-the-wall) carries the
trend of the prover's nodes against the solver's, an extrapolation.  The cost of a game follows the root's parity gap,
not n, so the bands are a guide, not a rule, and the first passes of the run check the estimate ([Run](#run), step 4,
reads the cost of a node from the log).

*Details.*  The [Graviton
calibration](https://github.com/bvchess/taxman-notebook/blob/main/graviton-calibration-2026-10/README.md) measured the
prover's speed: a Graviton4 core runs it at 0.93 of an M1 Max core, and a Graviton3 core at 0.75.  The cost of a node
grows with n: from the plain root on Graviton4 it was 2.0 to 2.6 ms through 1899, then 4.1 ms at 2170 to 2173.

## Which instance

**Use a `c8g.4xlarge` (16 vCPUs) at spot**, or on demand if spot has no capacity.  A `c8g.2xlarge` (8 vCPUs) takes
twice as long for the same price.  A larger instance shortens the run for about the same price, since a game's search
spreads over the threads, though nobody has measured a game on more than 4 threads.  The calibration
settled the family: a Graviton4 core does a unit of work for 12 to 15 % less than a Graviton3 core.  Nobody has measured
`c9g`.  The `c8g.4xlarge` cost $0.24 to $0.31 an hour at spot in four zones of us-east-1 on 2026-10-14.  A gp3 root
volume of 20 GB is enough: the prover's build is under 2 GB, the output a few hundred KB and the checkpoints a few MB.
The [rules for every launch](launch.md#rules-for-every-launch) apply, with the instance profile of [Access, and what
the account holds](launch.md#access-and-what-the-account-holds) (S3 for `S3_DEST`, SSM for the steps).

## Run

Launch as [Launch](launch.md#launch) says, with `TIMER_MINUTES` 60 × (`HOURS` + 1), `HOURS` being the run's
`DEADLINE_HOURS`: the estimate's hours with a margin.  Then send the steps through SSM ([Steps through SSM, or
SSH](launch.md#steps-through-ssm-or-ssh)), the first as root and the others as `ec2-user`.  With no SSH there is no
`rsync`, so the copy of the repository is a pushed branch, cloned, or a `git archive` tarball in the bucket.

1. **Setup.**  `PROVER_ONLY=1` builds and checks the prover alone; with the commit in the build cache it builds nothing
   ([build-cache.md](build-cache.md)), and `BUILD_PROVER="--bin exact"` builds `exact` alone.

   ```
   dnf install -y git tmux                                       # as root; setup.sh would do it with sudo
   git clone https://github.com/bvchess/taxman.git && cd taxman    # --branch NAME for another branch
   PROVER_ONLY=1 solver/tools/cloud/setup.sh                       # 46 s on a c8g.4xlarge, 10 s of it the build
   ```

2. **A rehearsal on the instance itself** (minutes, cents): a small range, a SIGTERM and a resume, and the check, in a
   work directory of its own, as a step of its own.  It tests the scripts on the machine, whatever the run's range, and
   replaces a dry run on a laptop.

   ```
   export TAXMAN_WORK=~/rehearsal THREADS=2 PASS_SECONDS=20
   solver/tools/cloud/prover.sh 1301 1310 &              # makes the order, then passes of 20 s
   sleep 40; kill -TERM $(cat ~/rehearsal/prover.pid)      # a stop in the middle: "ended: stopped", status 143
   wait $!; echo $?; ls ~/rehearsal/ckpt                   # 143 (a bare `wait` returns 0); exact_N_low.ckpt, paused
   solver/tools/cloud/prover.sh 1301 1310                  # resumes ("N searches paused"), finishes, runs the check
   OUT=~/rehearsal/results/exact_parity_1301_1310.jsonl solver/tools/cloud/prover.sh --verify 1301 1311   # must FAIL
   unset TAXMAN_WORK THREADS PASS_SECONDS; rm -rf ~/rehearsal
   ```

   The second `prover.sh` must end with `OK: opt(n) is proven equal to the recorded score for every n in 2..1310` and
   exit status 0, and the last command must print `without a line: 1311` and `FAILED`.  The node counts are the same
   whatever the stops.  The lines of `~/rehearsal/results/exact_parity_1301_1310.jsonl` give the cost of a node on this
   CPU, `secs` over `nodes`.  From the plain root these searches took 2.44 ms a node on a `c8g.4xlarge`; from the roles
   expect about half, an estimate from the measurement over 2..999, not a measurement on this machine.

3. **The run,** in tmux, with `spot-watch.sh` beside it, in passes of 300 s, since a spot interruption loses the pass
   in progress.  `PREFIX` is the run's own prefix of the bucket (`prover-A-B`), one a run, since a save deletes from it
   the checkpoints that the machine lacks:

   ```
   cd ~/taxman && tmux new-session -d -s pv "DEADLINE_HOURS=HOURS POWEROFF=always PASS_SECONDS=300 \
       S3_DEST=s3://taxman-013764204840-us-east-1/PREFIX solver/tools/cloud/prover.sh A B"
   tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh              # on spot: stops the run
   ```

   `prover.sh` stops at once unless `results/optimal.jsonl.gz` holds game B.  `spot-watch.sh` starts without `S3_DEST`,
   so its own `save.sh` copies nothing: the save to S3 is `prover.sh`'s, made as it exits (the exit trap of `run.sh`).
   The commands of [Watch, stop, resume](#watch-stop-resume) and [Bring the data home, verify,
   commit](#bring-the-data-home-verify-commit) on your machine reach it as `$D`: `export
   D=s3://taxman-013764204840-us-east-1/PREFIX`.

4. **The first minutes.**  `prover.sh` prints the machine, makes the order, and logs `pass 1: 0 of N games done`.  In
   `~/taxman-work/logs/prover.err`, the lines `exact: n=N not finished: paused after N nodes, U open units, M units
   moved` give the nodes of a long game.  The seconds of a pass over the nodes of one pass give the milliseconds a node,
   to compare with the estimate ([What it costs](#what-it-costs)); the late games cost more.

**`BASELINE`, from A = 1302 on.**  The check at the end needs every n in 2..A − 1 proven, from one file: `BASELINE`,
whose default holds the proofs of 2..1300.  For a later range, join the committed files of the proofs that cover 2..A −
1 and set `BASELINE` to the joined file before `prover.sh` in step 3 and before `prover.sh --verify`; for 2176..B:

```
cat solver/noframe/data/exact_parity_2_1300.jsonl solver/noframe/data/exact_parity_1301_2175.jsonl \
    > ~/baseline_2_2175.jsonl
```

`prover.sh` reads `BASELINE` only in that check, so a run without it ends in a failed check after the whole run; with
`S3_DEST` the results are saved, and `--verify` with the file runs the check again.  The joined file is also the first
file of `summarize.py exact` ([Bring the data home, verify, commit](#bring-the-data-home-verify-commit)).  `prover.sh
--list`, a measurement on chosen games, is in the script's header.

The guardrails are those of [launch.md](launch.md#cost-guardrails), with these differences:

* **`S3_DEST` is required with `POWEROFF`.**  A shutdown terminates the instance and deletes its root volume, so
  `prover.sh` copies the results to S3 before it powers off.  Without `S3_DEST`, leave out `POWEROFF` and pull the files
  yourself ([Bring the data home, verify, commit](#bring-the-data-home-verify-commit)) before the timer ends.
* **`POWEROFF=always`** powers the machine off whenever the script ends: the range complete and checked, the deadline,
  or an error.  `POWEROFF=done` powers it off only when the range is complete.
* **`DEADLINE_HOURS`** stops the run after that many hours, and `prover.sh` exits with status 3.  The stop is a SIGTERM,
  so the searches of the pass in progress go back to their last checkpoints ([How `exact` uses the
  CPUs](#how-exact-uses-the-cpus)).
* **`SAVE_EVERY`** defaults to `PASS_SECONDS` (300 s in the command above).  A save takes about 80 s at the lowest
  priority ([Surviving a spot interruption](launch.md#surviving-a-spot-interruption)).
* **`THREADS`** defaults to every vCPU.

## On a machine of your own

On a machine that nothing interrupts, `prover.sh` runs as on EC2 without the parts for the cloud: no `S3_DEST`, no
`POWEROFF` and no `spot-watch.sh`, in passes of the default 3600 s, on every CPU.  Prove a range already proven first,
as a calibration: the cost of a node and the wall time on this machine then compare with the committed proofs, and the
check at its end tests the build and the scripts on a whole range.

1. **Build** `exact` for this machine's CPU, in a clean checkout at the commit to run, where `prover.sh` takes it from
   (`EXACT_BIN`):

   ```
   RUSTFLAGS="-C target-cpu=native" cargo build --release --bin exact --manifest-path solver/noframe/Cargo.toml
   ```

2. **Rehearse** as [Run](#run), step 2, says, which takes a few minutes.

3. **The calibration**, 1301-2175, whose check needs no `BASELINE`.  `TAXMAN_WORK` holds the results, the checkpoints
   and the logs, on a disk that survives a reboot (`W` below); `OUT` names a file other than the committed one of the
   range.  The settings go in tmux's command, since a tmux server already running keeps its own environment:

   ```
   W=~/prover-work
   tmux new-session -d -s pv \
       "TAXMAN_WORK=$W OUT=$W/results/exact_threads_1301_2175.jsonl solver/tools/cloud/prover.sh 1301 2175"
   ```

   It ends with `OK: opt(n) is proven equal to the recorded score for every n in 2..2175`.  Compare `summarize.py cost
   --band 100` of its file with the table of [the prover's
   proofs](../../noframe/docs/proofs.md#to-2175-on-a-cloud-machine), which searched from the plain root ([What it
   costs](#what-it-costs)).

4. **A new range,** 2176-2530 for instance, with the `BASELINE` of [Run](#run) that covers 2..2175:

   ```
   cat solver/noframe/data/exact_parity_2_1300.jsonl solver/noframe/data/exact_parity_1301_2175.jsonl \
       > $W/baseline_2_2175.jsonl
   tmux new-session -d -s pv \
       "TAXMAN_WORK=$W BASELINE=$W/baseline_2_2175.jsonl solver/tools/cloud/prover.sh 2176 2530"
   ```

On a machine shared with other work, set `THREADS` below the number of CPUs and start the script under `nice -n 19`.
Watch and stop the run with the commands for the instance in [Watch, stop, resume](#watch-stop-resume), with `$W` for
`~/taxman-work`, and commit its file as [Bring the data home, verify, commit](#bring-the-data-home-verify-commit) says.

## Watch, stop, resume

From your machine, with no access to the instance, read the saves in S3 (`$D` of [Run](#run)).  They are as old as the
last save, about 6 minutes at most:

```
aws s3 cp $D/logs/prover.log - | tail            # a line a pass: games done, searches paused, the exit status
aws s3 cp $D/logs/prover.err - | tail -n 20      # exact's lines: finished games and paused searches with their nodes
aws s3 cp $D/results/exact_parity_A_B.jsonl - | grep -c .      # games done, of the range's
aws s3 ls $D/ckpt/                               # a checkpoint per unfinished search, rewritten every pass
```

On the instance, in a shell or as steps through SSM (where root's `~` is not ec2-user's: write `/home/ec2-user/` out):

```
tail -f ~/taxman-work/logs/prover.log            # a line a pass: games done, searches paused, the exit status
tail -n 20 ~/taxman-work/logs/prover.err         # exact's lines: finished games and paused searches with their nodes
grep -c . ~/taxman-work/results/exact_parity_A_B.jsonl      # games done, of the range's
ls -l ~/taxman-work/ckpt                         # a checkpoint per unfinished search, rewritten every pass
```

* **Stop** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/prover.pid)`; through SSM, which runs as root, the
  pid file is `/home/ec2-user/taxman-work/prover.pid`.  The searches of the pass in progress go back to their last
  checkpoints.  **Resume** with the same command; `THREADS` and `S3_DEST` may differ.
* **A spot interruption:** `spot-watch.sh` stops the run, which saves as it exits.  On a replacement machine run
  `PROVER_ONLY=1 solver/tools/cloud/setup.sh`, then `S3_DEST=... solver/tools/cloud/save.sh --restore`, then the same
  `prover.sh` command, minding the two cautions of [launch.md](launch.md#surviving-a-spot-interruption) (the logs and
  the deadline).  Without a saved copy, the results and the checkpoints are on the root volume, which a spot
  termination deletes: use `S3_DEST`, or `rsync` the work directory home from time to time.
* **An error stop** (`prover.sh` exits with status 1): `exact` stopped with an error, the final check failed, or
  `MAX_FAILS` passes in a row crashed without a new line.  Nothing is retried; read the end of `prover.log` and
  `prover.err`.  The message "a playable set worth X beats the recorded Y" is a counterexample: a better game than the
  committed chain's for that n, the finding a run exists to look for.  The set is in the message; keep `~/taxman-work`
  as it is, and report it.  The message "the recorded game does not replay to its score" means that the results file
  or the build is wrong.

## Bring the data home, verify, commit

```
export AWS_PROFILE=bvc                                                    # after: aws login --profile bvc
aws s3 cp $D/results/exact_parity_A_B.jsonl solver/noframe/data/          # $D: as in Run, above
aws s3 sync $D/logs/ ./prover-logs/
# or, over SSH, without S3:
rsync -av ec2-user@HOST:taxman-work/results/exact_parity_A_B.jsonl solver/noframe/data/
rsync -av ec2-user@HOST:taxman-work/logs/ ./prover-logs/
OUT=solver/noframe/data/exact_parity_A_B.jsonl solver/tools/cloud/prover.sh --verify A B   # with BASELINE from A = 1302
```

The check (`summarize.py check A B`, which `prover.sh` also ran at the end) passes only if two things hold:

* every n in 2..B is proven: each n of A..B has a line, and with `BASELINE` each is proven by a bound, a search run to
  the end, or the ceiling, the prime formula or the semiprime formula from a proven predecessor;
* every line's score is the score that `results/optimal.jsonl.gz` records.

It then exits with status 0 and prints `OK: opt(n) is proven equal to the recorded score for every n in 2..B`;
anything else prints which n failed.  Then, on your machine, count the proofs and their cost by bands of 100 in n:

```
python3 solver/noframe/tools/summarize.py exact BASELINE solver/noframe/data/exact_parity_A_B.jsonl
python3 solver/noframe/tools/summarize.py cost --band 100 solver/noframe/data/exact_parity_A_B.jsonl
```

Commit `solver/noframe/data/exact_parity_A_B.jsonl`, and never let a later run overwrite another run's file: `OUT`
names another file.  Do not commit the work directory, the checkpoints or the logs.  Record the run:

* the totals of the two commands (`summarize.py exact` gives the searches' thread-hours), and the real milliseconds a
  node by n, in [the prover's proofs](../../noframe/docs/proofs.md), which holds the proofs and their cost;
* the new end of the prover's proofs in [results/README.md](../../../results/README.md), which holds how far each kind
  of proof reaches; then search the docs for the old end and update what still names it;
* the run itself in a notebook entry, with its rows in the register ([the notebook's
  rules](https://github.com/bvchess/taxman-notebook/blob/main/README.md#how-this-notebook-is-kept)), and its cost as
  [The spending, and the end](launch.md#the-spending-and-the-end) says.

Then do the end of the [rules for every launch](launch.md#rules-for-every-launch): `POWEROFF=always` has shut the
instance down, which terminates it; confirm that nothing is left, the launch's security group included.

## An example

[The run of
1301..2175](https://github.com/bvchess/taxman-notebook/blob/main/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)
searched from the plain root on one `c8g.4xlarge` at spot; its entry has the estimate it was planned by, the launch's
values, what it cost and what its model got wrong.
