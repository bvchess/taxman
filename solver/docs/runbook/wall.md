# The unattended run to the wall

Part of the [runbook](../RUNBOOK.md).

`wall.sh` runs the chain from n = 1 on one `c8g.16xlarge` (64 threads, window 4) at spot, with no one to watch it.  The
machine builds a pinned commit of master, re-solves the games of the committed `results/strict.jsonl` (to compare with
it at home, [Verifying, and bringing the results home](chain.md#verifying-and-bringing-the-results-home)), and goes on
past them.  It stops by itself at the wall, when it has used its budget, or on a date.  When AWS takes the machine away,
a group starts another that goes on from the saved state.  [The run to the
wall](../../notebook/the-run-to-the-wall-2026-10/README.md) records the first run and the rehearsal;
[STATUS.md](../STATUS.md) holds the decision for the wall (Brian, 2026-10-16) and its reasons.  A launch needs Brian's
approval, with the estimate of [What needs Brian's approval, and what it
costs](#what-needs-brians-approval-and-what-it-costs).

## How it ends

The run ends at the first of these.  Each but the last is the run's end: the machine records it, takes the group to
capacity 0 and powers off, and nothing starts a machine in place of the one that ends it.

| end | what it is | the log says | status of `chain.sh` |
|---|---|---|---|
| **the wall** | a game's own search has passed `GAME_HOURS` (24): the hours it has been the oldest game of the window with its search running, over every run of it ([DESIGN.md, "The window: games in flight"](../DESIGN.md#the-window-games-in-flight)); waiting behind another game does not count | `wall at N after H hours` | 4 |
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

Four backstops depend on none of these scripts:

* the timer in the user data (`shutdown -h +MINUTES`, the budget and two hours), so that no machine outlives the whole
  run;
* a scheduled action of the group that takes it to capacity 0 at the date plus two hours;
* `MAX_INSTANCES`;
* the ceiling on the spot price (`--spot-max`, dollars an hour, $1.0): above it no machine launches, and the run waits.

## The parts

| what | where | does |
|---|---|---|
| `wall.sh` | your machine | `launch`, `status`, `stop` and `cleanup`; it prints every aws command that changes something (`+ aws ...`; `--verbose` prints the reads too), `DRY_RUN=1` prints the changes without making them, and the files the commands take stay in `~/taxman-wall/RUN` |
| the launch template `taxman-wall`, the security group `taxman-wall` (no inbound rule), the group `taxman-wall` | the account | one `c8g.16xlarge` (or `INSTANCE_TYPE`) at spot, lowest price first among the zones (`SpotAllocationStrategy=lowest-price`, one pool), capacity rebalancing off, `min 0, max 1`, EC2 health checks, tags `Project=taxman`, `Purpose=wall` |
| the user data | the template | sets the timer, installs git and tmux, clones the repository, checks out the pinned commit and starts the `wall-node.sh` that commit has (in `solver/tools/cloud`, or `rust/tools/cloud` before the restructure of 2026-10-18) in a tmux session `tx` as `ec2-user`, checking a few seconds later that the session lives; if any of that fails after five tries, it records the end and takes the group to 0 |
| `wall-node.sh` | each machine | restores the logs, runs `setup.sh` (the solver alone, `BUILD=--bin taxman`: from the build cache if it has the commit, tested; no tests of its own; its smoke checks run), then `save.sh --restore`; starts `spot-watch.sh` and a loop that looks for `run/stop`; runs `chain.sh` with the config; decides what its status means ([How it ends](#how-it-ends)) |
| `run.sh`'s ledger | each machine | counts the budget and the date (below) |
| `wall-policy.json` | the role `taxman-ec2` | lets the machines set the capacity of this one group and read groups ([What needs Brian's approval](#what-needs-brians-approval-and-what-it-costs)) |
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

## What needs Brian's approval, and what it costs

The [rules for every launch](launch.md#rules-for-every-launch) cover these account changes: the inline policy
`taxman-wall` on the role `taxman-ec2`; a security group, a launch template, an Auto Scaling group and a scheduled
action (all named `taxman-wall`); the spot instances the group launches, and their volumes; and objects in the bucket.
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
export COMMIT=THE-40-DIGIT-ID-IT-PRINTED RUN=chain-wall UNTIL=2026-10-27
DRY_RUN=1 solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL      # reads, prints what it would do
solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL               # after Brian's yes
```

The defaults are the run: `c8g.16xlarge`, window 4, wall 24 hours, budget 74 instance-hours, the chain to 8191, the spot
ceiling $1.0, a volume of 40 GB, zones by price.  `launch --help` prints the header of the script, and every option has
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
solver/tools/cloud/wall.sh stop            # clean: within 30 s the machine stops the chain (final checkpoint), saves, ends
solver/tools/cloud/wall.sh stop --now      # at once: records the end, takes the group to 0 (the machine is terminated;
                                         #   what it did since its last save, at most SAVE_EVERY, is lost)
solver/tools/cloud/wall.sh cleanup         # once it has ended: delete the group, template, security group, policy; end checks
```

`status` reads S3 and the group. It shows whether the run has ended and why; the run's age; the instance-hours against
the budget and the spend against its estimate; how many machines have started; the group's capacity and instances; and,
from what the machine last saved (every `SAVE_EVERY`, 10 minutes), the time of that save and how long ago it was, the
game the results are recorded through, and a line for each game in flight ([the
window](../../frame/README.md#games-in-flight)):

```
the chain (as of the last save to S3)
  last save to S3: 2026-10-18 15:17 PDT, 5 min 24 s ago
  recorded through 3929 (score 4854280)
  games in flight (window 4; "own search" counts toward the wall of 24 hours):
      n  own search  elapsed             nodes     best  state
   3930  1h15m00s    1h45m00s       1086013550   341541  running, 242.8k nodes/s
   3931  -           -                       -        -  no recent progress line
   3932  -           -                       -        -  no recent progress line
   3933  -           1h15m00s                63   341541  suspended
```

`status` tells a game's state from its last progress line in `logs/chain.err`:

* `running`: 1000 nodes a second or more in its last interval;
* `suspended`: its threads belong to another game;
* `search done`: the line says `done` or `optimal` and the game is not recorded yet (`being recorded` for the oldest
  game, else `waiting for the game before`);
* `search stopped` or `failed`;
* `no recent progress line`: the game has no line, or three lines of another game follow its line. A setup, a prime or
  repeated game and a search not started look alike in the logs.

`own search` is the wall clock that the oldest game keeps in `ckpt/nNNNN.clock` (up to a checkpoint interval old; `-`
for a game without one), `elapsed` the time since its search began, waits included, and `best` the progress line's sum
of promotions so far.  Times are in the local time zone (`TZ`) with its abbreviation.

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
`wall.sh launch --continue --commit THE-SAME-COMMIT --until DATE --budget-hours H --game-hours G`.  Use the commit the
run began with, because a checkpoint resumes only on the build that made it ([Run](chain.md#run)), and give the budget
as the new total.  `--continue` removes the end and the stop, writes the config again, and keeps the ledger, the results
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
* **Bringing it home.**  [Verifying, and bringing the results home](chain.md#verifying-and-bringing-the-results-home)
  says how to append the run's lines to the committed chain.
* **A resumed machine** passes the flag again (it is in `run/config`, which every machine reads), and the results file
  it restores from S3 gets the same checks.  The checkpoint of a searched game resumes as ever.
* **The trade-off.**  Without the flag, the chain from n = 1 re-proves the committed chain (in the run of 2026-10-17,
  game 3440 alone took 1.76 hours of the machine); with it, that stretch costs seconds and the run checks nothing of the
  build against the committed chain.  Use it for a run that goes past the repository's knowledge, and not for a run
  whose point is that check.

## The rehearsal

The rehearsal tests the whole mechanism on a small machine, with the chain to 1300 instead of the whole committed chain
and the limits tiny.  It takes two runs, because the wall is the end of a run: the first shows the wall and the end that
takes the group to 0, and the second the replacement and the budget.  It costs about $0.15 in all.

Use a `c8g.2xlarge` (8 vCPUs, 16 GiB, about $0.13 an hour at spot), whose machines build in about 7 minutes.  A
`c8g.large` (2 vCPUs, 4 GiB) cannot build the solver: its first machine stopped answering Systems Manager three minutes
after boot and sat at half its CPU for 35 minutes with the chain not started.

**Slow the chain on purpose.**  No game up to 1300 takes a second under the default search (on 2 threads of an M1 Max,
1232 takes 1.0 s, 1287 0.7 s, and none up to 400 takes 10 ms), so neither a wall of seconds nor a chain that outlasts
the budget would come.  Both runs pass `--extra-args "--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"`, the
matching path (the search before the GF(2) bound), whose games are slower and give the same answers.  On 2 threads of
the M1 Max it takes 2.1 s for game 950, 4.1 s for 988 and 989, 9.1 s for 990, 2.0 s for 1200, and hours for 1232
to 1300.

```
cd ~/dev/taxman_work/taxman
export AWS_PROFILE=bvc AWS_DEFAULT_REGION=us-east-1                 # after: aws login --profile bvc
git fetch && git rev-parse origin/master                              # the merged master with these scripts, pushed
export COMMIT=THE-40-DIGIT-ID-IT-PRINTED
export UNTIL=$(date -u -v+2d +%F)                                     # GNU date: date -u -d '2 days' +%F.  Check it.
```

**Run 1: the wall, and the end that takes the group to 0** (budget 0.3 hours; about 15 minutes).  A wall of 3.6 seconds
(`--game-hours 0.001`) falls at the first game whose search is longer as the head of the window.  No game from 400 to
987 takes more than 2.1 s on 2 threads of the M1 Max, 988 and 989 take 4.1 s and 990 9.3 s (a Graviton4 core is a little
slower), so the wall falls at 988, or at 989 or 990 if the head had less than 3.6 s of the search to do:

```
export RUN=rehearsal-1
DRY_RUN=1 solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL --instance-type c8g.2xlarge --volume-gb 20 \
    --spot-max 0.40 --budget-hours 0.3 --game-hours 0.001 --chain-to 1300 --margin 60 --max-instances 4 \
    --extra-args "--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL --instance-type c8g.2xlarge --volume-gb 20 \
    --spot-max 0.40 --budget-hours 0.3 --game-hours 0.001 --chain-to 1300 --margin 60 --max-instances 4 \
    --extra-args "--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
solver/tools/cloud/wall.sh status          # every few minutes
```

It passes if `status` ends in `ENDED ...: wall at N after H hours` within about 15 minutes of the launch, with N between
988 and 992 (the run of 2026-10-17 ended at 992) and H at least 0.001, and:

* `aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names taxman-wall --query 'AutoScalingGroups[0].DesiredCapacity'`
  says 0 and no instance remains (`describe-instances` of the end checks): the end took the group to 0 and nothing
  replaced the machine.  Check again 5 minutes later.
* `aws s3 ls s3://taxman-013764204840-us-east-1/wall-rehearsal-1/ckpt/` has `nN.ckpt` and `nN.clock`, and the last line
  of `results/strict.jsonl` there is game N − 1 (no line for N).
* The results pulled home agree with the committed chain:
  `aws s3 sync s3://taxman-013764204840-us-east-1/wall-rehearsal-1/results results-rehearsal-1/`, then
  `solver/tools/cloud/verify.sh results-rehearsal-1/strict.jsonl` prints
  `N − 1 games are in more than one file, with equal scores in all`.
* If the build was slow enough that the budget of 0.3 hours came first, the end says
  `deadline: the budget of 0.3 instance-hours`.  That tests the budget, and you run the wall again with
  `--budget-hours 0.6`.

Then run `solver/tools/cloud/wall.sh cleanup`, which ends with the checks.

**Run 2: a forced replacement, and the budget** (budget 0.6 hours, about 40 minutes).  The matching path's chain is
still going when you terminate the machine and when the budget ends (it reaches 1231 in minutes and is then in games of
hours).  Copies to S3 every 2 minutes and checkpoints every minute give the replacement something recent to restore:

```
export RUN=rehearsal-2
solver/tools/cloud/wall.sh launch --commit $COMMIT --until $UNTIL --instance-type c8g.2xlarge --volume-gb 20 \
    --spot-max 0.40 --budget-hours 0.6 --game-hours 24 --chain-to 1300 --margin 60 --max-instances 4 \
    --save-every 120 --checkpoint-interval 60 --extra-args "--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
solver/tools/cloud/wall.sh status          # until a game is in flight and the last save is recent
```

Once the chain is running and a save has followed it (about 13 minutes after the launch), terminate the machine without
a notice, which is the worst case (no final checkpoint, no final save):

```
I=$(aws ec2 describe-instances --filters Name=tag:Project,Values=taxman Name=tag:Purpose,Values=wall \
    Name=instance-state-name,Values=running --query 'Reservations[].Instances[].InstanceId' --output text)
aws ec2 terminate-instances --instance-ids $I
aws autoscaling describe-scaling-activities --auto-scaling-group-name taxman-wall --max-items 6 \
    --query 'Activities[].[StartTime,StatusCode,Description]' --output text      # a termination, then a launch
solver/tools/cloud/wall.sh status          # machines started: 2; the hours go on from where the first left them
```

It passes if:

* the group launches a second machine without anyone (within about 3 minutes);
* the log in S3
  (`aws s3 cp s3://taxman-013764204840-us-east-1/wall-rehearsal-2/logs/chain.log - | grep -E 'starting|stopped|deadline'`)
  shows a second `starting the solver` whose `games done so far` is at least the first's last save, and `chain.err`
  shows the game in progress continuing from its checkpoint (a node count above zero at its first progress line);
* `run/ledger` shows `base` equal to the first machine's `hours`;
* the run ends within a few minutes of 0.6 instance-hours with `deadline: the budget of 0.6 instance-hours`, the group
  at 0, no instance, and `hours` in the ledger between 0.55 and 0.65.

Then run `solver/tools/cloud/wall.sh cleanup`.  Once you have read the logs, the two prefixes can go (a deletion in the
bucket: Brian's yes first, like everything else here):

```
aws s3 rm --recursive s3://taxman-013764204840-us-east-1/wall-rehearsal-1/
aws s3 rm --recursive s3://taxman-013764204840-us-east-1/wall-rehearsal-2/
```

**The run of 2026-10-17** (commit eb84a6f; about $0.15 in all).  Run 1 ended `wall at 992 after 0.00100 hours`, with the
group at 0, the checkpoint and clock of 992 in S3, and the results 1..991 equal to the committed scores.  In run 2 the
machine terminated without notice at game 1289; the group replaced it 2 seconds later, the replacement resumed from S3
(`games done so far: 1289`, the ledger's `base` the first machine's hours), and the chain ended
`complete: the range 1-1300` before the budget, with the group at 0.  A third run, with a budget of 0.2 hours, ended
`deadline: the budget of 0.2 instance-hours` at 0.18, with the group at 0.

**Cost.**  Run 1 takes about 0.25 instance-hours and run 2 0.6, so 0.85 instance-hours of a `c8g.2xlarge`, with a 20 GB
gp3 volume for as long (under a cent) and a few thousand requests to S3: **about $0.15 in all**, and under $0.40 at the
spot ceiling of $0.40 an hour.  The account changes are those of [What needs Brian's approval, and what it
costs](#what-needs-brians-approval-and-what-it-costs), with the spot instance of a `c8g.2xlarge` and the
group's termination by `aws ec2 terminate-instances`.

**What the rehearsal leaves out:** the 64-thread machine, the hours-long searches (the Rust tests check the clock at the
wall for the window and its resume, not for 24 hours), and the spot interruption notice itself.  `terminate-instances`
sends none, so the path through `spot-watch.sh` (a SIGTERM, the final checkpoint, the wait for AWS to take the machine)
is what the local test `test/run.sh node` exercises with a signal.  AWS Fault Injection Service can send a real notice
(`aws:ec2:send-spot-instance-interruptions`), at the cost of an experiment template and a role, which this procedure
does not make.

## What was tested, and what can go wrong

The tests run on the laptop (macOS, bash 3.2):

* the solver's wall, by unit, integration and command-line tests (`cargo test --release --test window --test cli`);
* the ledger across machines, the wall and the date through `chain.sh`, and `wall-node.sh` through every ending
  (complete, the wall, the budget, a boot loop, a denied capacity, no config, a stop request, an interruption);
* `wall.sh` through a launch, `status`, both stops, `cleanup` and `--continue`, against the stand-in `test/bin/aws`
  (`solver/tools/cloud/test/run.sh`; all but `launch` and `status` need the release build of the solver and `rsync`);
* the arguments of every aws command, checked by the CLI's own parameter validation against an endpoint that refuses
  connections, so that nothing reaches AWS.

Nothing tests what needs AWS to answer: the behaviour of the group (the replacement, `lowest-price` across zones,
`resolve:ssm` as the image of a template that a group uses), the user data on Amazon Linux, and `tmux` with the options
used there.  The instances of the earlier runs exercised all of these but the group.

| symptom | cause and remedy |
|---|---|
| `launch` stops with an error after some commands | the script prints each command's error; `wall.sh cleanup --force` deletes what exists, then launch again |
| the group has no instance and the capacity is 1 | no spot capacity at the ceiling in those zones: `aws autoscaling describe-scaling-activities --auto-scaling-group-name taxman-wall` says why; add zones or raise `--spot-max` (`cleanup`, then `launch --continue`) |
| `status` shows `ENDED` with `boot failed` | the user data failed (install, clone, checkout, finding or starting `wall-node.sh`); the reason names the step; fix and `launch --continue` |
| machines keep starting | `run/boots/` counts them; the `MAX_INSTANCES`th ends the run as a boot loop; read `logs/wall-node.log` in S3 |
| the machine is up and `status` shows nothing new for over 20 minutes | `aws ssm start-session`, `tmux attach -t tx`; `tail ~/taxman-work/logs/chain.err` |
| `wall.sh stop` and nothing happens | the machine looks for the stop object every 30 s while the chain runs, and not during the setup; `stop --now` ends it at once |
