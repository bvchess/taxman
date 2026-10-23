# The unattended run to the wall

Part of the [runbook](../RUNBOOK.md).

`wall.sh` runs the chain on one `c8g.16xlarge` (64 threads, window 4) at spot, with no one to watch it.  The machine
builds a pinned commit of master and solves the chain from n = 1, or, with `--use-prev`, takes the games that the
committed chain and the bounds store already hold and goes on past them ([Drawing on the known
games](#launch-watch-stop-clean-up), below).  It stops by itself at the wall, when it has used its budget, or on a date.
When AWS takes the machine away, a group starts another that goes on from the saved state.  [The
glossary](../../../docs/GLOSSARY.md#the-search) gives the wall's limit.  A launch needs the approval of whoever owns the
AWS account, with the estimate of [What needs approval, and what it costs](#what-needs-approval-and-what-it-costs).

## How it ends

The run ends at the first of these.  Each but the last is the run's end: the machine records it, takes the group to
capacity 0 and powers off, and nothing starts a machine in place of the one that ends it.

| end | what it is | the log says | status of `chain.sh` |
|---|---|---|---|
| **the wall** | a game's own search ([glossary](../../../docs/GLOSSARY.md#the-search)) has passed `GAME_HOURS` (24) | `wall at N after H hours` | 4 |
| **the budget** | `BUDGET_HOURS` (74) instance-hours over every machine of the run, counted in the ledger; the solver stops five minutes (`LEDGER_MARGIN`) before they are used up | `stopped by the budget of 74 instance-hours` | 3 |
| **the date** | `RUN_UNTIL`, the hard date; the solver stops five minutes before it too | `stopped by the date ...` | 3 |
| the range | the chain reached `CHAIN_TO` (8191, the largest board) | `the range 1-N is complete` | 0 |
| an error | the solver reported one, the setup or the restore failed, a ledger could not be read, more than `MAX_INSTANCES` (20) machines started, or the node script died | the reason | 1 |
| the operator | `wall.sh stop` | `stopped on request` | 143 |
| *not an end:* **a spot interruption** | `spot-watch.sh` stops the chain on the notice (final checkpoint, save); AWS takes the machine away within two minutes, and the group launches another, which restores from S3 and goes on | `an interruption, it seems` | 143 |

At the wall, the checkpoint of the game (and its clock) stays in S3 and the run records no line for it; the results hold
the games before it, each appended by the `chain.sh` that concluded it.  The object `run/ended` in S3 holds the run's
end (the reason and the time).  A machine that starts later finds it, sets the capacity to 0 again and powers off, which
stops a failing end from repeating.  `wall.sh launch --continue` removes it on purpose.

Three backstops depend on none of these scripts, and a fourth on `wall-node.sh` alone:

* the timer in the user data (`shutdown -h +MINUTES`, the budget and two hours), so that no machine outlives the whole
  run;
* a scheduled action of the group that takes it to capacity 0 at the date plus two hours;
* `MAX_INSTANCES`, which `wall-node.sh` checks as each machine starts;
* the ceiling on the spot price (`--spot-max`, dollars an hour, $1.0): above it no machine launches, and the run waits.

## The parts

| what | where | does |
|---|---|---|
| `wall.sh` | your machine | `launch`, `status`, `stop` and `cleanup`; it prints every aws command that changes something (`+ aws ...`; `--verbose` prints the reads too), `DRY_RUN=1` prints the changes without making them, and the files the commands take stay in `~/taxman-wall/RUN` |
| the launch template `taxman-wall`, the security group `taxman-wall` (no inbound rule), the group `taxman-wall` | the account | one `c8g.16xlarge` (or `INSTANCE_TYPE`) at spot, lowest price first among the zones (`SpotAllocationStrategy=lowest-price`, one pool), capacity rebalancing off, `min 0, max 1`, EC2 health checks, tags `Project=taxman`, `Purpose=wall` |
| the user data | the template | sets the timer, installs git and tmux, clones the repository, checks out the pinned commit and starts the `wall-node.sh` that commit has (in `solver/tools/cloud`, or `rust/tools/cloud` before the restructure of 2026-10-18) in a tmux session `tx` as `ec2-user`, checking a few seconds later that the session lives; if any of that fails (the install and the clone after five tries), it records the end and takes the group to 0 |
| `wall-node.sh` | each machine | restores the logs, runs `setup.sh` (the solver alone, `BUILD=--bin taxman`: from the build cache if it has the commit, tested; no tests of its own; its smoke checks run), then `save.sh --restore`; starts `spot-watch.sh` and a loop that looks for `run/stop`; runs `chain.sh` with the config; decides what its status means ([How it ends](#how-it-ends)) |
| `run.sh`'s ledger | each machine | counts the budget and the date (below) |
| `wall-policy.json` | the role `taxman-ec2` | lets the machines set the capacity of this one group and read groups ([What needs approval](#what-needs-approval-and-what-it-costs)) |
| `test/run.sh` | your machine | tests these scripts against `test/bin/aws`, a stand-in over directories |

The run's state is one prefix of the project's bucket, `s3://taxman-013764204840-us-east-1/wall-RUN` (`RUN` defaults to
`run1` for `launch`; [Launch, watch, stop, clean up](#launch-watch-stop-clean-up) says what the other commands take).
It holds `results/`, `ckpt/` (with each game's `nNNNN.clock` beside its checkpoint) and `logs/` as `save.sh` writes
them, and `run/`:

| object | holds |
|---|---|
| `run/config` | the settings, which `launch` writes: `COMMIT`, `INSTANCE_TYPE`, `CHAIN_TO`, `WINDOW`, `GAME_HOURS`, `BUDGET_HOURS`, `RUN_UNTIL` (seconds since the epoch), `THREADS`, `EXTRA_ARGS`, `CHECKPOINT_INTERVAL`, `SAVE_EVERY`, `LEDGER_MARGIN`, `MAX_INSTANCES`, `PRICE`, and `RELEASE_TESTS` and `REQUIRE_CACHED` if the environment of `launch` had them ([Launch, watch, stop, clean up](#launch-watch-stop-clean-up)). Every machine reads it, so a budget or a date changed there (by `launch --continue`) reaches the next machine |
| `run/ledger` | `start` (the run's first start), `boot` (the boot id of the machine that wrote it last), `base` (the instance-hours of the machines before it), `hours` (base plus its uptime), `updated` |
| `run/boots/*` | one object a machine that started |
| `run/ended`, `run/stop` | the end, and the request of `wall.sh stop` |

**The ledger.**  A machine that starts `chain.sh` with `BUDGET_HOURS` or `RUN_UNTIL` and `S3_DEST` reads the ledger (if
there is none, the run begins now).  It takes the `hours` of the machines before it as its `base` (unless its boot id is
the ledger's: a second `chain.sh` of the same boot), and writes `base` plus its uptime every minute and when the script
ends.  Instance-hours are uptime, which is what AWS bills a spot instance for, to within its boot (a few seconds to a
minute a machine); a machine killed without notice loses at most a minute of count.  The stop falls where `BUDGET_HOURS`
less the hours counted less `LEDGER_MARGIN` falls from now, and the date's where `RUN_UNTIL` less the margin falls; the
earliest of those and `DEADLINE_HOURS` is the deadline (`timeout` stops the solver, which writes its final checkpoint).
A machine stops if it cannot read the ledger or the ledger has no numbers, because it must not run uncounted.

## What needs approval, and what it costs

The [rules for every launch](launch.md#rules-for-every-launch) cover these account changes: the inline policy
`taxman-wall` on the role `taxman-ec2`; a security group, a launch template, an Auto Scaling group and a scheduled
action (all named `taxman-wall`, the action `taxman-wall-date`); the spot instances the group launches, and their
volumes; and objects in the bucket.
The first Auto Scaling group of the account also creates the service-linked role `AWSServiceRoleForAutoScaling`, which
stays.  `cleanup` deletes the group, the template, the security group and the policy; the service-linked role, the
bucket and `taxman-ec2` stay on purpose.

**The policy** is [wall-policy.json](../../tools/cloud/wall-policy.json).  It allows `autoscaling:SetDesiredCapacity` on
the one group (its ARN `arn:aws:autoscaling:us-east-1:013764204840:autoScalingGroup:*:autoScalingGroupName/taxman-wall`,
the group's id being a `*`), and `autoscaling:DescribeAutoScalingGroups` on `*` (the action takes no resource), which
the node uses to read that the capacity went to 0.  It allows nothing else: a machine cannot launch, terminate or change
anything.  `launch` puts it with

```
aws iam put-role-policy --role-name taxman-ec2 --policy-name taxman-wall --policy-document file://solver/tools/cloud/wall-policy.json
```

and `cleanup` deletes it.

**The cost** is the budget: 74 instance-hours of a `c8g.16xlarge` at spot, which cost $0.66 (us-east-1b) to $0.81
(us-east-1c) an hour on 2026-10-17 and count at $0.676, so about $50 ($60 at the dearest zone, never above $74 at the
ceiling of $1.0), plus a 40 GB gp3 volume ($0.005 an hour, under $1 for the run) and the bucket (a few MB of results and
checkpoints).  `status` counts the spend as instance-hours times `PRICE` (0.676); the bill is the spot price of the
hours the machines ran, which the console or `describe-spot-price-history` gives afterwards.

## Launch, watch, stop, clean up

```
cd ~/dev/taxman_work/taxman
export AWS_PROFILE=bvc AWS_DEFAULT_REGION=us-east-1                 # after: aws login --profile bvc
git fetch && git rev-parse origin/master                              # the merged master, pushed: the machines clone it
export COMMIT=THE-40-DIGIT-ID-IT-PRINTED RUN=NAME UNTIL=YYYY-MM-DD
DRY_RUN=1 solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL      # reads, prints what it would do
solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL               # after the account owner's yes
```

A run past the committed chain should take the games that the chain and the bounds store already hold instead of
solving them again: add `--extra-args "--use-prev"` to both commands (the paragraph "Drawing on the known games",
below).

The defaults are the run: `c8g.16xlarge`, window 4, wall 24 hours, budget 74 instance-hours, the chain to 8191, the spot
ceiling $1.0, a volume of 40 GB, zones by price.  `launch --help` prints the header of the script, and every option but
`--continue` has
an environment variable.  `launch` prints the files (the user data, the config, the mixed instances policy) and each
command that changes something before it runs the command (`--verbose` adds the reads).  It puts the config in S3, then
makes the policy, the security group, the launch template, the group at capacity 0 and the scheduled action, and last
sets the capacity to 1, which starts the first machine.  Each machine runs `setup.sh` for the solver alone
(`BUILD=--bin taxman`) and its smoke checks, and starts the chain.  With the commit's tested binaries in the build cache
([build-cache.md](build-cache.md)) it builds nothing, and every machine of the run, the replacements after an
interruption included, takes the same binary.  Without them it builds the solver (a few minutes on 64 vCPUs, untested,
nothing stored).  Two settings in the environment of `launch`, `RELEASE_TESTS` and `REQUIRE_CACHED`, go into the config
for `setup.sh` on each machine ([build-cache.md](build-cache.md)); neither is on by default.

* **Release first,** once for the commit on the CPU of the run (any `c8g` will do: the key is the CPU, not the size).
  Either run `RELEASE_TESTS=1 solver/tools/cloud/setup.sh` on one machine, as [The release: the suite, then the
  cache](build-cache.md#the-release-the-suite-then-the-cache) says, or set `RELEASE_TESTS=1` in the environment of
  `launch`, which has the first machine of the run do it before the chain starts (the solver's suite only, since the
  machine builds `taxman` alone: its entry serves this run and not a launch of anything else).  A suite that fails ends
  the run (`ended`: `setup.sh failed`) and leaves `RELEASE_TESTS_FAILED`, the failing tests, in the run's prefix in S3.
* **`REQUIRE_CACHED=1`** makes a machine without tested binaries in the cache end the run instead of building them.  Use
  it after a release, so that nothing untested runs for the hours of the run.

```
solver/tools/cloud/wall.sh status          # the run, the ledger, the group, the games recorded and in flight
solver/tools/cloud/wall.sh status --host USER@HOST   # a chain on a machine of yours, over ssh: no S3, no AWS
solver/tools/cloud/wall.sh stop            # clean: within 30 s the machine stops the chain (final checkpoint), saves, ends
solver/tools/cloud/wall.sh stop --now      # at once: records the end, takes the group to 0 (the machine is terminated;
                                         #   what it did since its last save, at most SAVE_EVERY, is lost)
solver/tools/cloud/wall.sh cleanup         # once it has ended: delete the group, template, security group, policy; end checks
```

`status` reads S3 and the group. It shows whether the run has ended and why; the run's age; the instance-hours against
the budget and the spend against its estimate; how many machines have started; the group's capacity and instances; and,
from what the machine last saved (every `SAVE_EVERY`, 10 minutes), the time of that save and how long ago it was, the
game the results are recorded through, and one table of the last 6 games recorded and the games in flight ([the
window](../../frame/README.md#games-in-flight)).  `--recorded N` shows the last N games recorded instead of 6, and 0
leaves them out; the option goes before the command or after it, as `--verbose` does:

```
the chain (as of the last save to S3)
  last save to S3: 2026-10-18 15:17 PDT, 5 min 24 s ago
  recorded through 4871 (score 7,461,794)
  the last 6 games recorded and the 4 in flight (window 4; "own search" counts toward the wall of 24 hours):
      n  own search           nodes       best  state
   4866  0.0s                     0    537,485  recorded, score 7,446,689 (repeat, same_search of 4865)
   4867  1m49s           34,218,766    538,284  recorded, score 7,449,976 (search, exhausted)
   4868  50m11s       1,133,462,769    538,760  recorded, score 7,453,452 (search, exhausted)
   4869  0.0s                     0    538,745  recorded, score 7,456,914 (start, cap)
   4870  0.0s                     0    538,760  recorded, score 7,459,353 (repeat, same_search of 4868)
   4871  0.0s                     0          -  recorded, score 7,461,794 (prime, prime_formula)
   4872  1h15m00s     1,086,013,550    341,541  running, 242.8k nodes/s
   4873  -                        -          -  no recent progress line
   4874  -                        -          -  no recent progress line
   4875  -                       63    341,541  suspended
```

The recorded games come first, in increasing n, and the games in flight follow.  Scores, nodes and best sums have
thousands separators; game numbers and times do not.  A recorded game's row comes from its
line in `logs/chain.out` ([the summary line](../REFERENCE.md#the-summary-line-stdout-one-per-game)), the last one if a
replacement machine printed the game again after an interruption:

* `own search` and `best`: the line's `own=` and `best=`, the game's own search time over every run of it
  ([glossary](../../../docs/GLOSSARY.md#the-search)) and its best promotion sum, the score less `in_the_bag`.  Both are
  `-` where the solver that printed the line did not print them, and `best` is `-` for a prime or known game; a game that
  ran no search has an own search of 0.0s;
* `nodes`: the line's nodes;
* `state`: `recorded`, the score, and in parentheses the line's method and proof, then `optimal ok` when the score is
  the one in `optimal.json`.

`status` tells the state of a game in flight from its last progress line in `logs/chain.err`:

* `running`: 1000 nodes a second or more in its last interval;
* `suspended`: its threads belong to another game;
* `search done`: the line says `done` or `optimal` and the game is not recorded yet (`being recorded` for the oldest
  game, else `waiting for the game before`);
* `search stopped` or `failed`;
* `no recent progress line`: the game has no line, or three lines of another game follow its line. A setup, a prime or
  repeated game and a search not started look alike in the logs.

For a game in flight, `own search` is the wall clock that the oldest game keeps in `ckpt/nNNNN.clock` (up to a
checkpoint interval old; `-` for a game without one), and `best` the progress line's sum of promotions so far.  Times are
in the local time zone (`TZ`) with its abbreviation.  `save.sh` copies the logs before the checkpoints, so the clock can
be a minute or two ahead of the nodes and the best sum, which come from the last progress line of the logs.

`status --host USER@HOST` reports a chain that runs on a machine you reach by ssh (`chain.sh` in the tmux session `tx`,
its state in the chain's data folder of [the machine's folders](lab.md#the-machines-folders)) instead of a spot
instance: it reads that state in one ssh call (key authentication, `BatchMode`) and needs no AWS profile and makes no
aws call.  It prints the host and its `uptime` line; whether the chain
runs (the session exists and a `taxman solve` process is alive); `ENDED:` and the line of `chain.outcome`, if there is
one; the window and the wall, from the last line of `logs/chain.log` that starts the chain; when `logs/chain.err` last
changed, as `last progress line`; and `recorded through`, taken from the last completed game of the results file; then
the same table.  The recorded rows come from `logs/chain.out`, which holds only the games that the host's own run
recorded, so a run that has recorded none shows the games in flight alone; the clocks come from `ckpt/nNNNN.clock` and
the progress lines from the last 3000 lines of `logs/chain.err`.  `--recorded N` goes with it as above.

`RUN` names the run.  Without it, `launch` makes `run1`, and `status`, `stop` and `cleanup` take the one run in
progress, the one with a config in S3 and no `ended` object.  With none, or several, they refuse and say so, and list
the several; so a `cleanup` of a run that has ended needs `RUN`.  The region defaults to us-east-1, and the profile is
never defaulted (export `AWS_PROFILE`).  `status` prints the aws commands that read (`+ aws ...`) only with `--verbose`.
To look at the machine, use `aws ssm start-session --target INSTANCE` (the Session Manager plugin; the role has the SSM
policy), then `sudo -u ec2-user tmux attach -t tx` (the node script and, in the window `sw`, `spot-watch.sh`).

`cleanup` refuses a run that has not ended (`--force` overrides), deletes in order, waits for the instances to go, and
ends with the checks of the rules for every launch, which must print nothing or `[]`:

```
aws ec2 describe-instances --filters Name=tag:Project,Values=taxman Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped --query 'Reservations[].Instances[].InstanceId'
aws ec2 describe-volumes --filters Name=tag:Project,Values=taxman --query 'Volumes[].VolumeId'
aws ec2 describe-spot-instance-requests --filters Name=tag:Project,Values=taxman Name=state,Values=open,active --query 'SpotInstanceRequests[].SpotInstanceRequestId'
aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names taxman-wall --query 'AutoScalingGroups[].AutoScalingGroupName'
aws ec2 describe-launch-templates --filters Name=launch-template-name,Values=taxman-wall --query 'LaunchTemplates[].LaunchTemplateName'
aws ec2 describe-security-groups --filters Name=tag:Project,Values=taxman --query 'SecurityGroups[].GroupId'
aws iam list-role-policies --role-name taxman-ec2 --query PolicyNames          # taxman-s3-bucket only
```

**After the end,** pull the results
(`aws s3 sync s3://taxman-013764204840-us-east-1/wall-RUN/results results-from-cloud/`) and verify them at home as
[Verifying, and bringing the results home](chain.md#verifying-and-bringing-the-results-home) says.  `verify.sh` must
report that the games of the committed file agree with it with equal scores, "assumptions: none" (a run with
`--use-prev` reports the statements of the store's games instead), and both referees agreeing; the referees alone check
the games past its end.  The checkpoint of the walled game stays in `ckpt/`, because a run that goes on needs it.

**To go on** after the wall or the budget (a longer wall, more money), run
`wall.sh cleanup`, then
`wall.sh launch --continue --commit THE-SAME-COMMIT --until DATE --budget-hours H --game-hours G`.  Use the commit the
run began with, because a checkpoint resumes only on a build that sets up the same search ([Run](chain.md#run)), and
give the budget
as the new total.  `--continue` removes the end, the stop and the count of machines (`run/boots/`), writes the config
again, and keeps the ledger, the results
and the checkpoints.

**Drawing on the known games.**  The chain above searches every game from n = 1, which re-proves the committed chain
with a build.  A run that goes on past what the repository holds needs none of that.  With `--extra-args "--use-prev"`
(`EXTRA_ARGS`; the machines run `taxman solve 1-N --use-prev ...` in the checkout of the pinned commit), the chain draws
on the known games of that checkout's `results`: the committed chain `strict.jsonl`, the bounds store `bounds.jsonl`
([results/README.md](../../../results/README.md#boundsjsonl) says what it holds) and the built-in `optimal.json`
([README, "Known games"](../../frame/README.md#known-games)):

```
solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL --extra-args "--use-prev"
```

* **The chain takes the games of the committed chain from it,** each recorded in seconds with proof `known`.  The build
  re-proves nothing in that stretch, and the first minutes of the run go to replaying it.  The chain searches the game
  after the committed chain's end from the chain's last game, as the run from n = 1 does.
* **The chain records a game the store has proven with no search** (method `known`, citing the store's two lines), and
  searches the game after it from it, with its ceiling and seed.  The games the store has not proven still help: the
  lower bound's game seeds the search, and the upper bound stops it, which can shorten the game that sets the wall (the
  wall is then a game whose store bounds leave a gap, or that the store lacks).
* **What the lines assume.**  A game that rests on the store assumes statement U at the boards its upper bound rests on
  (`"statement U holds at n=B"`), and so do the games that rest on it, by the ceiling or the prime formula.  The results
  are therefore not "assumptions: none" past the first proven board of the store, and `verify.sh` counts them.
* **Both referees check the file against the same files.**  `taxman verify-results` and `solver/tools/check_results.py`
  take the committed chain and the store of the checkout they run in (`verify.sh` does, and `chain.sh` runs it at the
  end of a complete range), and they refuse a line that cites a store line that the store at that checkout lacks.  The
  store is append-only, so a checkout at a later commit has every line that the pinned one had.  Run
  `ledger verify-bounds results/bounds.jsonl` on the store first ([ledger.md](ledger.md)), because the chain takes its
  bounds on its word.

**A finder beside the long searches.**  A search that starts far below its final best spends most of its steps before
the incumbent catches up.  So by default a game whose search has taken a million steps in all gets a thread of its
own, beside the workers, for 60 s: it looks for a better game than the incumbent, from the incumbent and from the game
of n − 1, and offers it to the search ([the reference](../REFERENCE.md#1-the-flags-of-solve); [the
design](../design/finder.md)).  `--extra-args` can change the two with `--finder-after NODES` (0 starts
none) and `--finder-seconds S`.  A checkpoint resumes with any finder flags or none, the answers do not change, and a
progress line `n=N finder: ...` says when it started and what it found.  A game that ends before NODES steps never
starts one.
* **Bringing it home.**  [Verifying, and bringing the results home](chain.md#verifying-and-bringing-the-results-home)
  says how to append the run's lines to the committed chain.
* **A resumed machine** passes the flag again (it is in `run/config`, which every machine reads), and the results file
  it restores from S3 gets the same checks.  The checkpoint of a searched game resumes as ever.
* **The trade-off.**  Without the flag, the chain from n = 1 re-proves the committed chain (in the run of 2026-10-17,
  game 3440 alone took 1.76 hours of the machine); with it, that stretch costs seconds and the run checks nothing of the
  build against the committed chain.  Use it for a run that goes past the repository's knowledge, and not for a run
  whose point is that check.

## Rehearsing a change to the scripts

Rehearse a change to `wall.sh` or `wall-node.sh` on a small machine before a long run: the chain to a small n with the
limits tiny, so that the wall and the budget come in minutes, and a forced replacement of the machine.  [The rehearsal
of
2026-10-17](../../notebook/the-run-to-the-wall-2026-10/README.md#later-2026-10-20-the-rehearsal-as-the-runbook-gave-it)
is the procedure that tested the scripts, with its two runs, what each must show, and what it cost.

## What was tested, and what can go wrong

The tests run on the laptop (macOS, bash 3.2):

* the solver's wall, by unit, integration and command-line tests (`cargo test --release --test window --test cli`);
* the ledger across machines, the wall and the date through `chain.sh`, and `wall-node.sh` through every ending
  (complete, the wall, the budget, a boot loop, a denied capacity, no config, a stop request, an interruption);
* `wall.sh` through a launch, `status`, both stops, `cleanup` and `--continue`, against the stand-in `test/bin/aws`
  (`solver/tools/cloud/test/run.sh`; all but `launch` and `status` need the release build of the solver and `rsync`);
* the arguments of every aws command, checked by the CLI's own parameter validation against an endpoint that refuses
  connections, so that nothing reaches AWS.

Nothing tests what needs AWS to answer: the behavior of the group (the replacement, `lowest-price` across zones,
`resolve:ssm` as the image of a template that a group uses), the user data on Amazon Linux, and `tmux` with the options
used there.  The instances of the earlier runs exercised all of these but the group.

| symptom | cause and remedy |
|---|---|
| `launch` stops with an error after some commands | the script prints each command's error; `wall.sh cleanup --force` deletes what exists, then launch again |
| the group has no instance and the capacity is 1 | no spot capacity at the ceiling in those zones: `aws autoscaling describe-scaling-activities --auto-scaling-group-name taxman-wall` says why; add zones or raise `--spot-max` (`cleanup --force`, then `launch --continue`) |
| `status` shows `ENDED` with `boot failed` | the user data failed (install, clone, checkout, finding or starting `wall-node.sh`); the reason names the step; fix, `cleanup`, then `launch --continue` |
| machines keep starting | `run/boots/` counts them; the one after the `MAX_INSTANCES`th ends the run as a boot loop; read `logs/wall-node.log` in S3 |
| the machine is up and `status` shows nothing new for over 20 minutes | `aws ssm start-session`, `tmux attach -t tx`; `tail ~/taxman-work/logs/chain.err` |
| `wall.sh stop` and nothing happens | the machine looks for the stop object every 30 s while the chain runs, and not during the setup; `stop --now` ends it at once |

## Examples

[The run to the wall](../../notebook/the-run-to-the-wall-2026-10/README.md) was the first, from n = 1, with its
rehearsal; [the run to 5000](../../notebook/the-run-to-5000-2026-10/README.md) drew on the known games and ended on its
budget.
