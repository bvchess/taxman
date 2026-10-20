# Certifying U (stage 4)

Part of the [runbook](../RUNBOOK.md).

`certify.sh` runs `taxman certify` on every game of 2176..3440 that rests on the frame model, so that every game to 3440
is proven on the whole board, as the games to 2175 are ([results/README.md](../../../results/README.md)).  It is the
campaign of the entry [Certificates past
2175](../../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md); the
heading's "stage 4" names the campaign as the notebook's register does (the row "S4 preparation", after E8's stages 1
and 2).  [STATUS](../STATUS.md) says where it stands, and a launch waits for Brian's approval with the estimate of [What
it costs, and what to approve](#what-it-costs-and-what-to-approve) ([rules for every
launch](launch.md#rules-for-every-launch)).  [DESIGN.md, "Certificates of U"](../DESIGN.md#certificates-of-u) says what
a certificate is, and [solver/tools/certify/README.md](../../tools/certify/README.md) describes the work list, its data
and the lists.  The campaign launches as [Every launch, step by step](launch.md#every-launch-step-by-step) says: an
instance profile, steps through SSM, `S3_DEST`, a timer, `spot-watch.sh` beside the run, tmux.  Only the machine, the
commands and the checks differ.

## What it costs, and what to approve

**Approve up to $50 at spot (or $160 on demand), in two steps:** phase A first ($3 at spot, $11 on demand), then phases
B and C once the stop rule of 9.2 says GO.  The table below prices the campaign at $31 to $35 at spot.

708 games need a certificate.  The rule of the same problem settles 145 of them with no search: each sits in the list of
the game it rests on, after it.  115 more repeat a search that the rule cannot settle; they are searched too, and go
last.  The plan prices a certificate at twice the plain search of its problem, by its nodes at the cost of a node in the
chain's own run (`solver/tools/certify/plan.md`).  The lists and their prices, in thread-hours of the plan, in hours of
one `c8g.16xlarge` (64 threads), and in dollars at the spot price paid on 2026-10-15 ($0.6752 an hour) and on demand
($2.5517):

| phase | lists | games | thread-hours | hours on 64 threads | spot | on demand |
|---|---|---:|---:|---:|---:|---:|
| A: the pilot, with the stop rule | `pilot7` | 10 (7 searched) | 288 | 4.5 | $3.04 | $11.48 |
| B: the dear games, the dearest first (3440, 3424, 3425, 2346, ...) | `dear_rest` | 14 (12 searched) | 1,837 | 28.7 | $19.38 | $73.23 |
| B: the others, four processes of 16 threads | `rest_1` to `rest_4` | 567 | 815 | 12.7 | $8.60 | $32.50 |
| C: the repeats the rule cannot settle | `deferred` | 117 (115 searched) | 0-361 | 0-5.6 | $0-3.81 | $0-14.40 |
| **all** | | **708** | **2,940-3,301** | **46-52** | **$31-35** | **$117-132** |

Phase C costs nothing if a build with Lemma M5⁺ is merged by then: that lemma, unzip's claims in the extended problem,
is the open item of [the
entry](../../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md), and its
rule settles 3427 and its like from the certificates of phases A and B (3427 alone is 223 thread-hours).  Without it,
the 117 games are searched, for 361 thread-hours.

* **If the overhead is not 2.**  The plan prices a certificate at 2 times the plain search; the certificates of eight
  games of 1400..2156 took a median of 1.55 times it ([one-sided candidates in the
  search](../../notebook/one-sided-candidates-in-the-search-2026-10/README.md), the second stage).  At 1.6 the campaign
  costs 2,350-2,640 thread-hours ($25-28 at spot, $94-105 on demand), at 2.4 3,530-3,960 ($37-42, $141-158).  The pilot
  measures it where the money goes (|O| near 200-300, root gaps 1,100-1,550).
* **Spending past 1.5 times the estimate pauses the campaign.**  Each phase runs under a `DEADLINE_HOURS` of 1.5 times
  its expected hours (the table of 9.4), after which the script stops cleanly and the campaign waits for a report.
  Resume only with Brian's yes.
* **Wall time.**  About two days on one machine; the dearest game, 3440, takes 10.6 hours by itself.  Two machines (the
  dear list on one, the four rest lists on the other, each with its own S3 prefix, 9.6) bring the rest down to about 29
  hours at the same cost.
* **Risks.**  The overhead at the dear end could be worse than the plan's; the pilot finds that out for $3.  A spot
  interruption costs at most a checkpoint interval (5 minutes), and nothing with `spot-watch.sh`.  A counterexample (a
  game that beats a recorded score) stops everything and is the result itself: the chain's score would be too low at
  that game.

## The stop rule of the pilot

**Go on if the median R is at most 3 and no R is above 5,** where R is a certificate's cost over the plan's price of its
plain search; the campaign then costs at most 1.5 times the plan.  **Stop** otherwise: report, and build the speedups
(Lemma M5⁺ and a faster node) before spending more.  **Stop at once** on any `beaten` verdict.
`solver/tools/certify_pilot.py` (its header defines the quantities) computes all of it from the certificates and the
plan ([Phase A](#phase-a-the-pilot)).

The seven games of `pilot7` (2340, 3175, 3300, 3335, 3336, 3408, 3420, with the games that rest on them) run one after
the other on all 64 threads, so that each game's time per node is measured cleanly.  For each game that was searched,
with N its nodes in the chain (`plan.jsonl`, `cost_nodes`) and c the cost of a node in its stretch of the chain (243.84
microseconds of a thread up to 3310, 326.73 after; the table of eras in `plan.md`):

* **R** = the certificate's thread-seconds (`seconds_all_runs` x `threads`) / (N x c): the plan priced it at 2.
* **R_nodes** = the certificate's nodes / N, and **R_node** = its thread-microseconds per node / c, so that R = R_nodes
  x R_node splits the overhead into the tree and the node.  The second stage predicts R_nodes about 1.1-1.5 (the rule of
  symmetric pairs is missing) and R_node about 1.2-1.6.

## The machine and the build

* **Machine.**  One `c8g.16xlarge` (64 vCPUs, Graviton4) at spot in us-east-1, in the zone with the lowest price of the
  spot price history ([Quota, prices and spot interruptions](machine.md#quota-prices-and-spot-interruptions): read it
  first, since it is the price of the estimate), or on demand if spot has no capacity.  A gp3 root volume of 40 GB,
  `Purpose=certify`, and the rest as [Launch](launch.md#launch) launches it.  Memory is no concern: a few hundred MB a
  process, the results file loaded in each.
* **The commit.**  The commit of master that holds this section, pushed first, since the instance clones it.  Call it
  `COMMIT`: the whole campaign runs on it, and the lines of every certificate record it (`build`).  The build is
  `setup.sh` with the tests skipped (they run on the laptop), which builds the solver and the prover for the CPU of the
  machine and runs its smoke checks.
* **Phase C on another build.**  If a build with Lemma M5⁺ is merged by phase C, check it out on the machine of phase C
  (`git fetch && git checkout M5COMMIT`) and build again.  Delete the checkpoint directory of `deferred` if an earlier
  attempt left one, since a checkpoint of another build is refused.  Phases A and B never change build: a checkpoint is
  resumed only by the build that made it, and the lines of one list should come from one build.
* **Inputs,** all committed: `results/strict.jsonl` (the recorded games), `solver/noframe/data/upper_*.jsonl` (the
  prover's barred bounds: 2..1300, 1301..2175, 2176..3314, 3315..3440), `solver/tools/certify/plan.jsonl` and `plan.md`,
  and the lists in `solver/tools/certify/lists/stage4/`.  `certify.sh` takes them from the checkout.

## The lists, and the commands of the phases

`solver/tools/certify_campaign.py lists` makes the lists from `plan.jsonl` (its header says how), and
`solver/tools/certify_campaign.py check` checks them; run the check on the machine before the first phase.  A game that
rests on another's certificate follows it in the same list, since the rule reads only the certificates of its own output
file, and no game is in two lists.

```
python3 solver/tools/certify_campaign.py check --plan solver/tools/certify/plan.jsonl \
    --lists-dir solver/tools/certify/lists/stage4              # 7 lists, 708 games: ...
```

| list | phase | games | thread-hours | threads | expected hours | `DEADLINE_HOURS` (1.5 times) |
|---|---|---:|---:|---:|---:|---:|
| `pilot7` | A | 10 | 288.0 | 64 | 4.5 | 7 |
| `dear_rest` | B | 14 | 1,836.6 | 64 | 28.7 | 43 |
| `rest_1`, `rest_2`, `rest_3`, `rest_4` | B | 151, 139, 134, 143 | 203.8 each | 16 each | 12.7 | 19 |
| `deferred` | C | 117 | 361.2 | 64 | 5.6 | 9 |

`certify.sh LIST[:THREADS]...` runs each list as a process of its own: `taxman certify --list` the list, with its output
file `certificates_LIST.jsonl` in `$TAXMAN_WORK/results`, its checkpoint directory `$TAXMAN_WORK/ckpt/LIST` and a
checkpoint every 300 seconds.  Without `:THREADS` the machine's threads are shared equally among the lists, so
`rest_1 rest_2 rest_3 rest_4` is four processes of 16.  All of them share one script, one deadline, one S3 saver and one
`spot-watch.sh`; the script's header has the exit statuses and the variables.  When every list is done, the script runs
`taxman verify-certificates` over their output files and exits 0 only if that passes.  A counterexample (status 2 of
`taxman certify`) stops all the processes at once, with their checkpoints, and ends the script with status 2.

| phase | command (in tmux, from `~/taxman`) | S3 prefix | timer at launch |
|---|---|---|---|
| A | `DEADLINE_HOURS=7 POWEROFF=done S3_DEST=$D/A solver/tools/cloud/certify.sh pilot7` | `$D/A` | `shutdown -h +480` |
| B | `DEADLINE_HOURS=43 S3_DEST=$D/B solver/tools/cloud/certify.sh dear_rest && DEADLINE_HOURS=19 POWEROFF=always S3_DEST=$D/B solver/tools/cloud/certify.sh rest_1 rest_2 rest_3 rest_4` | `$D/B` | `shutdown -h +3900` |
| C | `DEADLINE_HOURS=9 POWEROFF=done S3_DEST=$D/C solver/tools/cloud/certify.sh deferred` | `$D/C` | `shutdown -h +600` |

`D=s3://taxman-013764204840-us-east-1/certify-2176-3440`, with one prefix a phase: a machine saves its checkpoint
directory with `--delete`, so it must not touch another machine's prefix.  `POWEROFF=done` shuts the machine down, and
so terminates it, when the phase is complete, after the final save.  A phase that stops for any other reason leaves the
machine up until its timer, so that the work directory can be looked at: pull what you need and terminate it (the rules
for every launch).  The timers are the deadlines plus an hour or more for the setup, the final save and the
verification.

## Phase A: the pilot

Launch as [Launch](launch.md#launch) says, after Brian's yes, with these values:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=certify TIMER_MINUTES=480 MAX_PRICE=1.2
```

`ZONE` is the zone of the lowest price that day ([The machine and the build](#the-machine-and-the-build)), and the
ceiling of $1.2 an hour is above any price seen.  Then send the steps through SSM as in [Steps through SSM, or
SSH](launch.md#steps-through-ssm-or-ssh), the first as root and the others as `ec2-user`, with `COMMIT` the campaign's:

```
dnf install -y git tmux                                                     # as root
git clone https://github.com/bvchess/taxman.git && cd taxman && git checkout COMMIT
NO_PULL=1 SKIP_TESTS=1 solver/tools/cloud/setup.sh                            # builds for this CPU, smoke checks
```

and the check of the lists ([The lists, and the commands of the
phases](#the-lists-and-the-commands-of-the-phases)).

**A rehearsal first** (minutes, cents): six small games of 1301..2175, in a work directory of its own, to see the build,
the S3 saves and the deadline work on the machine itself.  It must end `ended: complete`, with `verify-certificates`
passing (status 0).

```
export TAXMAN_WORK=~/rehearsal D=s3://taxman-013764204840-us-east-1/certify-rehearsal
mkdir -p ~/rehearsal/lists && printf '%s\n' 1312 1313 1314 1315 1316 1317 >~/rehearsal/lists/r.txt
LISTS=~/rehearsal/lists DEADLINE_HOURS=1 S3_DEST=$D solver/tools/cloud/certify.sh r:4    # 6 games: 3 searched, 3 by the rule
unset TAXMAN_WORK; aws s3 rm --recursive $D; rm -rf ~/rehearsal
```

**Then the pilot:**

```
cd ~/taxman && export D=s3://taxman-013764204840-us-east-1/certify-2176-3440
tmux new-session -d -s cert "DEADLINE_HOURS=7 POWEROFF=done S3_DEST=$D/A solver/tools/cloud/certify.sh pilot7"
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

In about 4.5 hours the log says `ended: complete` and the machine shuts itself down; the results are in
`$D/A/results/certificates_pilot7.jsonl`.  At home, apply the stop rule:

```
export AWS_PROFILE=bvc                                                     # after: aws login --profile bvc
mkdir -p certify-run && aws s3 sync $D/A certify-run/A
python3 solver/tools/certify_pilot.py certify-run/A/results/certificates_pilot7.jsonl \
    --list solver/tools/certify/lists/stage4/pilot7.txt --bands
```

The report has a line for each searched game (|O|, root gap, GF(2) words, set width, nodes against the chain's, R_nodes,
R_node, R, thread-hours against the plan's), the median and largest R, the search time in dollars, and the verdict.  Its
exit status is 0 for GO, 3 for STOP, 2 for COUNTEREXAMPLE, and 4 for UNFINISHED: a game has no verdict yet, and the
report says STOP at once if a game is already above 5.  While the pilot runs, the same command on the copy in S3
(`aws s3 cp $D/A/results/certificates_pilot7.jsonl -`, saved every 10 minutes) shows the games so far.  On GO, report to
Brian with the table and ask for phases B and C; on STOP, report and build the speedups; on COUNTEREXAMPLE, see [Watch,
stop, resume, and a counterexample](#watch-stop-resume-and-a-counterexample).

## Phase B

Phase B runs on a new machine, since the pilot's is gone.  Launch and set it up as in [Phase A](#phase-a-the-pilot) with
`TIMER_MINUTES=3900`, with no rehearsal:

```
cd ~/taxman && export D=s3://taxman-013764204840-us-east-1/certify-2176-3440
tmux new-session -d -s cert "DEADLINE_HOURS=43 S3_DEST=$D/B solver/tools/cloud/certify.sh dear_rest && \
    DEADLINE_HOURS=19 POWEROFF=always S3_DEST=$D/B solver/tools/cloud/certify.sh rest_1 rest_2 rest_3 rest_4"
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

`dear_rest` goes first, the dearest games first (3440 alone takes 10.6 hours), on all threads, one game after the other;
if it stops for any reason, the second command does not run.  Then the four rest lists run together.  On two machines
instead, run the same two commands, one on each: `dear_rest` with `D/B1` and `POWEROFF=done`, and the four rest lists
with `D/B2` and `POWEROFF=done`, each machine with a timer of its own deadline plus an hour.  The second launch reuses
the first's security group (`sg`), since 2.2 names a group by the purpose and the day.  Phase C then fetches both
prefixes.

## Phase C

Phase C searches the repeats that the rule cannot settle, or settles them by the rule of a build with Lemma M5⁺ ([The
machine and the build](#the-machine-and-the-build)).  Its output file starts as a copy of everything the earlier phases
wrote (with two machines in phase B, sync `$D/B1/results` and `$D/B2/results` in place of `$D/B/results` below), so that
the rule finds the certificates of the games it rests on, and so that the file at the end is the whole campaign's.  On a
machine set up as in [Phase A](#phase-a-the-pilot), on the build that phase C is to run on, with `TIMER_MINUTES=600`:

```
cd ~/taxman && export D=s3://taxman-013764204840-us-east-1/certify-2176-3440 W=$HOME/taxman-work/results
mkdir -p $W && aws s3 cp $D/A/results/certificates_pilot7.jsonl $W/ && aws s3 sync $D/B/results $W
cat $W/certificates_pilot7.jsonl $W/certificates_dear_rest.jsonl $W/certificates_rest_{1,2,3,4}.jsonl \
    >$W/certificates_deferred.jsonl                          # the order matters: a game follows the game it rests on
tmux new-session -d -s cert "DEADLINE_HOURS=9 POWEROFF=done S3_DEST=$D/C solver/tools/cloud/certify.sh deferred"
tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh
```

`certify.sh` ends with `verify-certificates` over that file, which then holds all 708 games.  The files of phases A and
B, which `aws s3 sync` copies alongside, hold the same lines again.

## Watch, stop, resume, and a counterexample

```
tail -n 5 ~/taxman-work/logs/certify_pilot7.log        # this script: a status line every 10 minutes, the exits of the processes
tail -n 3 ~/taxman-work/logs/certify_dear_rest.out     # the result lines of the games finished (nodes, seconds, verdict)
tail -n 3 ~/taxman-work/logs/certify_dear_rest.err     # the progress lines of the game in progress (nodes, eff, best)
ls -l ~/taxman-work/ckpt/*                              # a checkpoint no older than 5 minutes while a long game runs
```

The log of a phase is `certify_PHASE.log`, PHASE being the list's name, or for several lists the first list's with `x`
and the number of lists (`certify_rest_1x4.log`); each list has its `certify_LIST.out` and `.err`.  The saves in S3, as
old as the last save (about 10 minutes), hold the same files under `$D/X/logs/` and the certificates under
`$D/X/results/`, X being the phase's letter (A, B or C) of [the lists](#the-lists-and-the-commands-of-the-phases).

* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/certify_PHASE.pid)`.  Every process writes
  a final checkpoint, and the script ends with status 143.  **Resume** with the same command: it skips every game with a
  verdict and resumes the search with a checkpoint.  The thread counts and `S3_DEST` may differ; the build may not.
* **A spot interruption:** `spot-watch.sh` stops the run and saves.  On a replacement machine, set up as in [Phase
  A](#phase-a-the-pilot) with the same `COMMIT`, run `S3_DEST=$D/X solver/tools/cloud/save.sh --restore` and copy the
  logs back (`aws s3 sync $D/X/logs ~/taxman-work/logs`), or the new machine's first save overwrites the old ones.  Then
  run the same command with `DEADLINE_HOURS` set to the hours that remain, since the deadline is not saved ([Surviving a
  spot interruption](launch.md#surviving-a-spot-interruption)).
* **An error stop** (status 1): a game failed a check (a recorded game that does not replay, a certificate that
  contradicts another, a checkpoint of another build).  Nothing is retried; read `certify_LIST.err`.
* **A counterexample** (status 2, `COUNTEREXAMPLE` in the log): a game scores more than its recorded score.  The
  processes stopped with their checkpoints, and the script saved to S3.  The game is in
  `certificates_LIST.counterexample.jsonl` and in the last lines of `certify_LIST.out`.  Keep `~/taxman-work` as it is,
  pull what is in S3, terminate the machine, and tell Brian: the recorded score of that game is too low, and the chain
  is wrong from there.  Do not go on.

## Results home, verification, and the spending

```
aws s3 sync $D certify-run                                 # A/, B/ and C/ (the whole campaign is C's file)
cp certify-run/C/results/certificates_deferred.jsonl results/certificates_2176_3440.jsonl
nice -n 10 solver/frame/target/release/taxman verify-certificates results/certificates_2176_3440.jsonl \
    --results results/strict.jsonl --upper solver/noframe/data/upper_2_1300.jsonl \
    --upper solver/noframe/data/upper_1301_2175.jsonl --upper solver/noframe/data/upper_2176_3314.jsonl \
    --upper solver/noframe/data/upper_3315_3440.jsonl --threads 4
python3 solver/tools/certify_pilot.py results/certificates_2176_3440.jsonl --bands > certify-run/overhead.txt
```

`verify-certificates` must report every game to 3440 proven (`proven frontier: every game to 3440 is proven`).  If phase
C was left for later, the file of the earlier phases is the `cat` of [Phase C](#phase-c), the proven end of the chain is
the first of the 117 games, and the same commands finish it.  Commit `results/certificates_2176_3440.jsonl` (under 1
MB), and never let a rerun overwrite it: a rerun writes another file.  Then record the campaign:

* the new end of the proofs on the whole board in [results/README.md](../../../results/README.md), which holds how far
  each kind of proof reaches; then search the docs for the old end and update what still names it;
* a notebook entry with the times against the estimate and the overhead by |O| and gap (the report above, `--bands`),
  and its rows in the register ([the notebook's rules](../../notebook/README.md#how-this-notebook-is-kept));
* the spending, as [The spending, and the end](launch.md#the-spending-and-the-end) says, with the thread-hours of the
  search against the plan's, and what they come to on one machine, which `certify_pilot.py` prints.
