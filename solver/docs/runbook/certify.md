# Certifying U (`certify.sh`)

Part of the [runbook](../RUNBOOK.md).

`certify.sh LIST[:THREADS]...` runs `taxman certify` on lists of games that rest on the frame model, so that each game
it certifies is proven on the whole board, as the prover's games are ([results/README.md](../../../results/README.md)).
[solver/docs/design/certificates.md](../design/certificates.md) says what a certificate is, and
[solver/tools/certify/README.md](../../tools/certify/README.md) describes the work list, its data and the lists.  A
campaign launches as [Every launch, step by step](launch.md#every-launch-step-by-step) says: an instance profile, steps
through SSM, `S3_DEST`, a timer, `spot-watch.sh` beside the run, tmux.  Only the machine, the commands and the checks
differ.  A launch needs the approval of whoever owns the AWS account, with an estimate made as [What it
costs](#what-it-costs) says ([rules for every launch](launch.md#rules-for-every-launch)).

## What it costs

**Price the lists from the plan.**  `solver/tools/certify/plan.jsonl` prices each game's certificate at a multiple of
the plain search of its problem, by its nodes at the cost of a node in the chain's own run, and `plan.md` says how.  A
game that repeats a search which the rule of the same problem settles costs nothing; a repeat that the rule cannot
settle is searched like any other.  A list's thread-hours, over the threads it runs on, give its hours, at the spot
price of the hour ([Quota, prices and spot interruptions](machine.md#quota-prices-and-spot-interruptions)).

* **Measure the overhead first.**  The multiple is the plan's guess; the certificates of eight games of 1400..2156
  took a median of 1.55 times the plain search ([one-sided candidates in the
  search](../../notebook/one-sided-candidates-in-the-search-2026-10/README.md)).  A pilot of a few games where the
  money goes, before the rest, finds out where it matters ([A pilot first](#a-pilot-first)).
* **Bound each phase.**  Run each phase under a `DEADLINE_HOURS` of 1.5 times its expected hours, after which the script
  stops cleanly and the campaign waits for a report; resume only with a new approval.
* **Risks.**  A spot interruption costs at most a checkpoint interval (5 minutes), and nothing with `spot-watch.sh`.  A
  counterexample (a game that beats a recorded score) stops everything and is the result itself: the chain's score
  would be too low at that game.

## A pilot first

A pilot certifies a few games of the campaign one after the other on all the threads, so that each game's time per node
is measured cleanly, and `solver/tools/certify_pilot.py` (its header defines the quantities) compares them with the
plan.  For each game that was searched, with N its nodes in the chain (`plan.jsonl`, `cost_nodes`) and c the cost of a
node in its stretch of the chain (the table of eras in `plan.md`):

* **R** = the certificate's thread-seconds (`seconds_all_runs` x `threads`) / (N x c), against the plan's multiple;
* **R_nodes** = the certificate's nodes / N, and **R_node** = its thread-microseconds per node / c, so that R = R_nodes
  x R_node splits the overhead into the tree and the node.

Decide a stop rule before the pilot runs (a median R and a largest R past which the campaign stops, to build speedups
first), and stop at once on any `beaten` verdict.  The report's exit status is 0 for GO, 3 for STOP, 2 for
COUNTEREXAMPLE, and 4 for UNFINISHED: a game has no verdict yet.

## The machine and the build

* **Machine.**  One `c8g.16xlarge` (64 vCPUs, Graviton4) at spot, in the zone with the lowest price of the spot price
  history, or on demand if spot has no capacity.  A gp3 root volume of 40 GB, `Purpose=certify`, and the rest as
  [Launch](launch.md#launch) launches it.  Memory is no concern: a few hundred MB a process, the results file loaded in
  each.
* **One build for a campaign.**  The commit of master that the campaign runs on, pushed first, since the instance
  clones it.  The lines of every certificate record it (`build`).  A checkpoint is resumed only by a build that sets up
  the same search, and the lines of one list should come from one build, so a list never changes build; a list that is
  to run on another build starts with its checkpoint directory deleted.  The build is `setup.sh` with the tests skipped
  (they run on the laptop), which builds the solver and the prover for the CPU of the machine and runs its smoke checks.
* **Inputs,** all committed: `results/strict.jsonl` (the recorded games), `solver/noframe/data/upper_*.jsonl` (the
  prover's barred bounds), `solver/tools/certify/plan.jsonl` and `plan.md`, and the lists in
  `solver/tools/certify/lists/`.  `certify.sh` takes them from the checkout.

## The lists

`solver/tools/certify_campaign.py lists` makes the lists from `plan.jsonl` (its header says how), and
`solver/tools/certify_campaign.py check` checks them; run the check on the machine before the first list.  A game that
rests on another's certificate follows it in the same list, since the rule reads only the certificates of its own output
file, and no game is in two lists.

`certify.sh LIST[:THREADS]...` runs each list as a process of its own: `taxman certify --list` the list, with its output
file `certificates_LIST.jsonl` in `$TAXMAN_WORK/results`, its checkpoint directory `$TAXMAN_WORK/ckpt/LIST` and a
checkpoint every 300 seconds.  Without `:THREADS` the machine's threads are shared equally among the lists, so four
lists on 64 threads are four processes of 16.  All of them share one script, one deadline, one S3 saver and one
`spot-watch.sh`; the script's header has the exit statuses and the variables.  When every list is done, the script runs
`taxman verify-certificates` over their output files and exits 0 only if that passes.  A counterexample (status 2 of
`taxman certify`) stops all the processes at once, with their checkpoints, and ends the script with status 2.

## Run

Launch as [Launch](launch.md#launch) says, with `TIMER_MINUTES` the deadline plus an hour or more for the setup, the
final save and the verification, and a price ceiling above any price seen:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=certify TIMER_MINUTES=... MAX_PRICE=1.2
```

Then send the steps through SSM as in [Steps through SSM, or SSH](launch.md#steps-through-ssm-or-ssh), the first as
root and the others as `ec2-user`, with `COMMIT` the campaign's:

```
dnf install -y git tmux                                                     # as root
git clone https://github.com/bvchess/taxman.git && cd taxman && git checkout COMMIT
NO_PULL=1 SKIP_TESTS=1 solver/tools/cloud/setup.sh                            # builds for this CPU, smoke checks
```

and the check of the lists ([The lists](#the-lists)).

**A rehearsal first** (minutes, cents): six small games, in a work directory of its own, to see the build, the S3 saves
and the deadline work on the machine itself.  It must end `ended: complete`, with `verify-certificates` passing (status
0).

```
export TAXMAN_WORK=~/rehearsal D=s3://taxman-013764204840-us-east-1/certify-rehearsal
mkdir -p ~/rehearsal/lists && printf '%s\n' 1312 1313 1314 1315 1316 1317 >~/rehearsal/lists/r.txt
LISTS=~/rehearsal/lists DEADLINE_HOURS=1 S3_DEST=$D solver/tools/cloud/certify.sh r:4    # 6 games: 3 searched, 3 by the rule
unset TAXMAN_WORK; aws s3 rm --recursive $D; rm -rf ~/rehearsal
```

**Then the lists,** with `PREFIX` a prefix of the bucket for each machine or phase (a machine saves its checkpoint
directory with `--delete`, so it must not touch another machine's prefix) and `HOURS` the phase's deadline:

```
cd ~/taxman && export D=s3://taxman-013764204840-us-east-1/PREFIX
tmux new-session -d -s cert "DEADLINE_HOURS=HOURS POWEROFF=done S3_DEST=$D solver/tools/cloud/certify.sh LIST..."
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

`POWEROFF=done` shuts the machine down, and so terminates it, when the lists are complete, after the final save.  A run
that stops for any other reason leaves the machine up until its timer, so that the work directory can be looked at:
pull what you need and terminate it (the rules for every launch).  Lists that rest on the certificates of earlier ones
start from a copy of the earlier output files, joined in their order, as their own output file: a game follows the game
it rests on.  At home, run `certify_pilot.py` on a pilot's certificates (`--list` its list, `--bands`) to apply the stop
rule.

## Watch, stop, resume, and a counterexample

```
tail -n 5 ~/taxman-work/logs/certify_LIST.log          # this script: a status line every 10 minutes, the exits of the processes
tail -n 3 ~/taxman-work/logs/certify_LIST.out          # the result lines of the games finished (nodes, seconds, verdict)
tail -n 3 ~/taxman-work/logs/certify_LIST.err          # the progress lines of the game in progress (nodes, eff, best)
ls -l ~/taxman-work/ckpt/*                              # a checkpoint no older than 5 minutes while a long game runs
```

The log of a run is `certify_LIST.log`, LIST being the list's name, or for several lists the first list's with `x` and
the number of lists (`certify_rest_1x4.log`); each list has its `certify_LIST.out` and `.err`.  The saves in S3, as old
as the last save (about 10 minutes), hold the same files under `$D/logs/` and the certificates under `$D/results/`.

* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/certify_LIST.pid)`.  Every process writes
  a final checkpoint, and the script ends with status 143.  **Resume** with the same command: it skips every game with a
  verdict and resumes the search with a checkpoint.  The thread counts and `S3_DEST` may differ; the build may not.
* **A spot interruption:** `spot-watch.sh` stops the run and saves.  On a replacement machine, set up as above with the
  same `COMMIT`, run `S3_DEST=$D solver/tools/cloud/save.sh --restore` and copy the logs back (`aws s3 sync $D/logs
  ~/taxman-work/logs`), or the new machine's first save overwrites the old ones.  Then run the same command with
  `DEADLINE_HOURS` set to the hours that remain, since the deadline is not saved ([Surviving a spot
  interruption](launch.md#surviving-a-spot-interruption)).
* **An error stop** (status 1): a game failed a check (a recorded game that does not replay, a certificate that
  contradicts another, a checkpoint of another build).  Nothing is retried; read `certify_LIST.err`.
* **A counterexample** (status 2, `COUNTEREXAMPLE` in the log): a game scores more than its recorded score.  The
  processes stopped with their checkpoints, and the script saved to S3.  The game is in
  `certificates_LIST.counterexample.jsonl` and in the last lines of `certify_LIST.out`.  Keep `~/taxman-work` as it is,
  pull what is in S3, terminate the machine, and report it: the recorded score of that game is too low, and the chain
  is wrong from there.  Do not go on.

## Results home, verification, and the spending

```
aws s3 sync $D certify-run
cp certify-run/results/certificates_LIST.jsonl results/certificates_A_B.jsonl
nice -n 10 solver/frame/target/release/taxman verify-certificates results/certificates_A_B.jsonl \
    --results results/strict.jsonl --upper solver/noframe/data/upper_FILE.jsonl ... --threads 4
python3 solver/tools/certify_pilot.py results/certificates_A_B.jsonl --bands > certify-run/overhead.txt
```

`verify-certificates`, given every file of the prover's barred bounds, must report the games of the range proven
(`proven frontier: every game to B is proven`).  Commit the certificates under a name of their own, and never let a
rerun overwrite them: a rerun writes another file.  Then record the campaign:

* the new end of the proofs on the whole board in [results/README.md](../../../results/README.md), which holds how far
  each kind of proof reaches; then search the docs for the old end and update what still names it;
* a notebook entry with the times against the estimate and the overhead by |O| and gap (the report above, `--bands`),
  and its rows in the register ([the notebook's rules](../../notebook/README.md#how-this-notebook-is-kept));
* the spending, as [The spending, and the end](launch.md#the-spending-and-the-end) says, with the thread-hours of the
  search against the plan's, and what they come to on one machine, which `certify_pilot.py` prints.

## An example

[Certificates past
2175](../../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md) prepared a
campaign for 2176..3440 ("stage 4"), not run; [its section on the
campaign](../../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md#the-stage-4-campaign-as-the-runbook-planned-it)
has its plan: the prices of its seven lists, the stop rule of its pilot, and the commands of its three phases.
