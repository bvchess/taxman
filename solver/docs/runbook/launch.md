# Launching a cloud machine

Part of the [runbook](../RUNBOOK.md).

## Rules for every launch

These hold for every instance this project starts: a trial of the scripts, the prover's proofs, the chain, a
calibration.  The jobs link here instead of repeating them.

* **Get Brian's approval first.**  An instance, a volume, a key pair, a security group, a spot request, a bucket and a
  role each cost money or change the AWS account, so each needs his explicit yes and a cost estimate before you make the
  call.  Reading (describe, the spot price history, quotas) needs none.  The estimate is the hours times the spot price
  of the hour: read the history first ([Quota, prices and spot
  interruptions](machine.md#quota-prices-and-spot-interruptions)), and let the timer, below, bound the hours.
* **Use spot, on `c8g`.**  Spot is the default ([Spot or on demand](machine.md#spot-or-on-demand) says where an
  interruption would ruin a run), on the `c8g` family (Graviton4), in the sizes that the calibration settled on:
  `c8g.xlarge` to try the scripts, `c8g.4xlarge` for the prover's proofs ([Which instance](prover.md#which-instance)),
  `c8g.16xlarge` for the chain.  The entry [Graviton calibration
  (2026-10)](../../notebook/graviton-calibration-2026-10/README.md) holds the measurements, with their scripts and data.
* **Set a timer on every instance.**  Launch with `--instance-initiated-shutdown-behavior terminate`, and put
  `shutdown -h +MINUTES` in the user data, with MINUTES a little past the run's own deadline (`DEADLINE_HOURS`, [Cost
  guardrails](#cost-guardrails)).  Any shutdown (the timer, `POWEROFF`, the spot service) then terminates the instance,
  so a forgotten instance ends by itself.  The timer is the backstop that depends on no script.
* **Tag everything.**  Put `Project=taxman` and `Purpose=...` (`trial`, `prover`, `chain`, `wall`, `ledger`, `sweep`,
  `certify`) on everything the launch creates that takes a tag: the instance, its volumes and spot request, a key pair,
  a security group.  The check at the end finds them by `Project`.
* **Take the binaries from the build cache.**  A machine does not build what a machine has already built and tested.
  `setup.sh` takes the commit's release binaries from S3 ([build-cache.md](build-cache.md)) and, when the cache has
  none, builds only what the job needs (`BUILD`).  The release suite runs once for each commit and CPU, on one machine
  with every vCPU busy, before `setup.sh` stores the binaries (`RELEASE_TESTS=1`); a failing suite stores nothing, and
  the job does not start.
* **Copy off what must outlive the instance.**  Termination deletes the root volume, so copy the results first
  (`S3_DEST` or `rsync`: [Cost guardrails](#cost-guardrails) and [Getting the results off the
  machine](#getting-the-results-off-the-machine)).
* **End clean.**  Once the results are home, terminate the instance and confirm that nothing of the project remains, in
  the region of the launch.  Each command prints `[]`; the CLI takes its profile from `AWS_PROFILE`, and [Access, and
  what the account holds](#access-and-what-the-account-holds) shows signing in:

  ```
  aws ec2 describe-instances --filters Name=tag:Project,Values=taxman \
      Name=instance-state-name,Values=pending,running,stopping,stopped --query 'Reservations[].Instances[].InstanceId'
  aws ec2 describe-volumes --filters Name=tag:Project,Values=taxman --query 'Volumes[].VolumeId'
  aws ec2 describe-spot-instance-requests --filters Name=tag:Project,Values=taxman Name=state,Values=open,active \
      --query 'SpotInstanceRequests[].SpotInstanceRequestId'
  ```

  Then delete the key pairs and security groups that the launch created, and only those: find them by the same tag, and
  leave alone any that the launch used but did not create.  Report the instance-hours (launch to the terminate call),
  the spot price of the hour and the cost, as the "Cost" paragraph of the calibration entry does.  Keep the bucket and
  the role and instance profile `taxman-ec2` ([Access, and what the account holds](#access-and-what-the-account-holds))
  for the next run; a finished run's prefix in the bucket can go once its results are home.

## Every launch, step by step

Every job launches the same way.  This section has the steps; each job says only what differs: its
instance, its timer, its setup and its command.

### Access, and what the account holds

A launch needs no SSH.  The instance has an instance profile (for S3 and SSM), its security group has no inbound rule,
and it has no key pair; you send it the steps through AWS Systems Manager (SSM) with `aws ssm send-command`.  SSH is the
fallback ([Steps through SSM, or SSH](#steps-through-ssm-or-ssh)).  The [rules for every
launch](#rules-for-every-launch) govern all of it.  Run every `aws` command as `AWS_PROFILE=bvc` in us-east-1:

```
aws login --profile bvc
export AWS_PROFILE=bvc AWS_DEFAULT_REGION=us-east-1
```

The bucket and the role exist (Brian approved making them on 2026-10-14) and stay between runs.  The security group is
made for each launch and deleted at its end.

| what | name | content |
|---|---|---|
| S3 bucket | `taxman-013764204840-us-east-1` | the state of every run, one prefix a run: `prover-1301-2175/` holds `results/`, `logs/` and `ckpt/`, the layout `save.sh` writes |
| IAM role and instance profile | `taxman-ec2` | trusted by EC2; an inline policy that lists that bucket and gets, puts and deletes its objects; the managed policy `AmazonSSMManagedInstanceCore` |
| security group | one a launch | no inbound rules (the SSM agent connects out, over HTTPS); tagged like everything else |

For another account or region, make the bucket and the role like this:

```
aws s3api create-bucket --bucket taxman-013764204840-us-east-1 --region us-east-1
aws iam create-role --role-name taxman-ec2 --assume-role-policy-document \
    '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"ec2.amazonaws.com"},
      "Action":"sts:AssumeRole"}]}'
aws iam put-role-policy --role-name taxman-ec2 --policy-name taxman-s3-bucket --policy-document \
    '{"Version":"2012-10-17","Statement":[
      {"Effect":"Allow","Action":"s3:ListBucket","Resource":"arn:aws:s3:::taxman-013764204840-us-east-1"},
      {"Effect":"Allow","Action":["s3:GetObject","s3:PutObject","s3:DeleteObject"],
       "Resource":"arn:aws:s3:::taxman-013764204840-us-east-1/*"}]}'
aws iam attach-role-policy --role-name taxman-ec2 --policy-arn arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore
aws iam create-instance-profile --instance-profile-name taxman-ec2
aws iam add-role-to-instance-profile --instance-profile-name taxman-ec2 --role-name taxman-ec2
```

### Launch

Each job gives its own instance type, volume, `Purpose` and timer.  The values below are the prover's
([prover.md](prover.md)): one `c8g.4xlarge` on spot, Amazon Linux 2023 arm64 (user `ec2-user`; Ubuntu 22.04 or 24.04
arm64 has `ubuntu`), no key pair, in the zone with the lowest price in the spot price history.  The timer is the job's
`DEADLINE_HOURS` plus an hour, in minutes: 36 + 1 hours is 2220.  The ceiling `MAX_PRICE` goes above the spot price
([Spot or on demand](machine.md#spot-or-on-demand)).  `run-instances` resolves the AMI from an SSM parameter; check the
parameter first with `aws ssm get-parameter --name ...` if you like.

```
INSTANCE_TYPE=c8g.4xlarge ZONE=us-east-1b VOLUME_GB=20 PURPOSE=prover TIMER_MINUTES=2220 MAX_PRICE=0.40
tags="{Key=Project,Value=taxman},{Key=Purpose,Value=$PURPOSE}"
vpc=$(aws ec2 describe-vpcs --filters Name=is-default,Values=true --query 'Vpcs[0].VpcId' --output text)
sg=$(aws ec2 create-security-group --group-name taxman-$PURPOSE-$(date +%Y%m%d) --vpc-id $vpc \
    --description "taxman $PURPOSE, no inbound" --tag-specifications "ResourceType=security-group,Tags=[$tags]" \
    --query GroupId --output text)
I=$(aws ec2 run-instances --instance-type $INSTANCE_TYPE --placement AvailabilityZone=$ZONE \
    --image-id resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64 \
    --security-group-ids $sg --iam-instance-profile Name=taxman-ec2 --metadata-options HttpTokens=required \
    --block-device-mappings "DeviceName=/dev/xvda,Ebs={VolumeSize=$VOLUME_GB,VolumeType=gp3,DeleteOnTermination=true}" \
    --instance-initiated-shutdown-behavior terminate \
    --user-data "$(printf '#!/bin/bash\nshutdown -h +%s' $TIMER_MINUTES)" \
    --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=taxman-$PURPOSE},$tags]" \
        "ResourceType=volume,Tags=[$tags]" "ResourceType=spot-instances-request,Tags=[$tags]" \
    --instance-market-options "MarketType=spot,SpotOptions={MaxPrice=$MAX_PRICE}" \
    --query 'Instances[0].InstanceId' --output text)
# on demand: leave out the market options and the spot request's tags
```

The instance requires IMDSv2 (`HttpTokens=required`), which `spot-watch.sh` uses.  A job that starts itself adds its
steps to the user data, as [the ledger's experiment](ledger.md) and [the
sweep](sweep.md#launch-watch-stop-bring-the-results-home) do.  To launch from the console, set the same fields: the
image, the volume, no inbound rule, the instance profile, shutdown behaviour "terminate", the timer in the user data,
and the tags with a `Name`.  For SSH access, see [Steps through SSM, or SSH](#steps-through-ssm-or-ssh).

### Steps through SSM, or SSH

**Steps through SSM.**  The instance is ready for commands when
`aws ssm describe-instance-information --filters Key=InstanceIds,Values=$I --query 'InstanceInformationList[].PingStatus'`
says `Online` (`$I` is the instance id from [Launch](#launch)).  A step is then a `send-command` and a read of its
result:

```
aws ssm send-command --instance-ids $I --document-name AWS-RunShellScript --timeout-seconds 60 \
    --parameters '{"commands":["sudo -u ec2-user -i bash /tmp/step.sh"],"executionTimeout":["600"]}' \
    --query Command.CommandId --output text                                # prints the command id, CID
aws ssm get-command-invocation --instance-id $I --command-id CID \
    --query '[Status,StandardOutputContent,StandardErrorContent]' --output text
```

`send-command` runs as root, with no login shell, so run the steps as `ec2-user`, from script files, with
`sudo -u ec2-user -i bash FILE` (a multi-line `bash -lc '...'` through `sudo -i` mangles its quoting).  A first
`send-command` writes the file (its text base64-encoded, `base64 -d > /tmp/step.sh`), or the step fetches it from the
bucket, which the role may read.  A command stops at its `executionTimeout` (an hour unless you set it), so a step that
lasts hours is not a foreground command: the full run starts in a detached tmux session, and its step returns at once.

**With SSH instead** (a console launch, say), the security group allows SSH from your address only and nothing else
inbound, and the instance has a key pair.  It needs the instance profile only if you use `S3_DEST`.  Run the same
commands in a shell on the instance, the run in tmux (`tmux new -s pv`), and use `rsync` or `scp` for the code and the
results ([A small instance first](before-a-big-run.md#a-small-instance-first)).

### Setup

On the instance, as `ec2-user` (over SSH, or through SSM as [Steps through SSM, or SSH](#steps-through-ssm-or-ssh)
says):

```
sudo dnf install -y git tmux                       # Ubuntu: sudo apt-get install -y git tmux
git clone https://github.com/bvchess/taxman.git && cd taxman
solver/tools/cloud/setup.sh                          # 5 to 10 minutes
```

The scripts must be on the branch that gets cloned (`git clone --branch NAME` clones another).  Without a pushed branch,
copy the tree from your machine (`rsync -a --exclude target --exclude .git ~/dev/taxman_work/taxman/ HOST:taxman/`) and
run `setup.sh` from inside it; it then works on that tree and does not pull, and the results record no commit.

`setup.sh`:

* installs packages, and installs Rust with rustup if there is none (stable, at least 1.89);
* looks first, in a clean checkout at a commit, for that commit's tested binaries in the build cache, and builds nothing
  when it finds them ([build-cache.md](build-cache.md)); otherwise builds with `RUSTFLAGS="-C target-cpu=native"` the
  solver, its examples `ledger`, `perf` and `ladder`, and the prover (`solver/noframe`) (`STUDIES=1` adds the study
  tools `tune`, `extended`,
  `structure` and `wall`, which a default build skips);
* replays `optimal.json`, runs both result verifiers on the committed files, and runs `taxman regress --to 400`;
* runs `cargo test -- --test-threads=4`, and the prover's `cargo test --release`.

Its output is also in `~/taxman-work/logs/setup.log`.  Run it again to pick up later commits; a reboot loses nothing it
made.  `RELEASE_TESTS=1 solver/tools/cloud/setup.sh` puts the tested binaries into the cache
([build-cache.md](build-cache.md)).

### Cost guardrails

These variables bound a run.  `DEADLINE_HOURS`, `POWEROFF`, `BUDGET_HOURS` and `RUN_UNTIL` belong to `run.sh`, so
`chain.sh`,
`sweep.sh`, `prover.sh` and `certify.sh` read them; `GAME_HOURS` is `chain.sh`'s.  The table says what `chain.sh` does.

| variable | effect |
|---|---|
| `DEADLINE_HOURS` | after that many hours from the first start (on a replacement machine, from its own first start: [Surviving a spot interruption](#surviving-a-spot-interruption)), `chain.sh` stops the solver cleanly with a final checkpoint and ends with status 3 |
| `POWEROFF` | `done` powers the machine off when the range is complete; `always` powers it off whenever `chain.sh` ends for any reason, which needs passwordless `sudo`, as on both AMIs |
| `GAME_HOURS` | the wall: the chain stops at the first game whose own search has passed that many hours, over every run of it, keeping every checkpoint and recording nothing for the game; the log says `wall at N after H hours`, and the status is 4 |
| `BUDGET_HOURS` | instance-hours over every machine of the run, counted in the ledger in S3; the solver stops cleanly five minutes before they are used up, with status 3 like `DEADLINE_HOURS` |
| `RUN_UNTIL` | a date in seconds since the epoch, treated like `BUDGET_HOURS` |

Use `POWEROFF=always` for an unattended run, since an idle machine after an error or a deadline costs as much as a busy
one.  `chain.sh` saves to `S3_DEST` before it powers off, which matters because a shutdown terminates the instance.
[wall.md](wall.md) puts the budget and the date together with a group that restarts the machine.  The timer (a backstop
that depends on no script), the tags and the cleanup are in the [rules for every launch](#rules-for-every-launch); a
budget alert on the account, set before the first launch, is the other half.

### Surviving a spot interruption

1. `spot-watch.sh` sees the two-minute notice, sends SIGTERM to `chain.sh`, waits for it, and runs `save.sh`.  Without
   it, the OS shutdown signals the processes, and in the worst case you lose one checkpoint interval.
2. With `S3_DEST` set, `chain.sh` has been copying results, logs and checkpoints every 10 minutes (`save.sh`; the
   checkpoints with `--delete`, so a finished game's disappears), at the lowest CPU and I/O priority (`nice -n 19`,
   `ionice -c 3` on Linux), so that a save (about 1.5 s of CPU) never competes with the workers.  The price of that
   priority is time: against 16 busy threads a save took about 80 s of wall time on a `c8g.4xlarge`, and the next waits
   `SAVE_EVERY` after it ends, so the copies are `SAVE_EVERY` plus the save apart (about 380 s for `prover.sh`, whose
   `SAVE_EVERY` is 300 s).  That is harmless: the final saves, at a stop or the end, run after the work has stopped.
   The copy of `save.log` in S3 trails by one save, because a save logs that it is done after it uploads the logs; the
   file on the machine is current.
3. On a replacement machine, run `setup.sh` at the same commit (`git checkout` it if the branch moved, or the solver may
   refuse the checkpoint), then `S3_DEST=... solver/tools/cloud/save.sh --restore`, then `chain.sh N` again.
   `--restore` refuses to overwrite an existing results file unless `FORCE=1`.  With a separate EBS volume instead,
   attach it in the same availability zone, mount it, and set `TAXMAN_WORK`.  Two things of the old machine do not come
   back, and you must see to both:
   * **The logs.**  `--restore` copies the results and the checkpoints, not `logs/`.  Copy them back before the chain
     starts (`aws s3 sync $S3_DEST/logs ~/taxman-work/logs`), or the first save of the replacement uploads its own
     `chain.log`, `chain.out`, `chain.err`, `save.log` and `spot-watch.log` over the old machine's in S3, which share
     their names.
   * **The deadline.**  `save.sh` does not save `~/taxman-work/deadline`, so a replacement machine starts a fresh
     deadline at its first start.  Give it `DEADLINE_HOURS` set to the hours that remain, so that the deadline falls
     where the first one did.  A run that replacements continue by themselves does not count that way: its
     `BUDGET_HOURS` and `RUN_UNTIL` live in S3 ([wall.md](wall.md)).

### Getting the results off the machine

Pull from your own machine, so that the instance holds no credentials:

```
rsync -av ec2-user@HOST:taxman-work/results/ ./results-from-cloud/        # ubuntu@HOST on Ubuntu
rsync -av ec2-user@HOST:taxman-work/logs/ ./logs-from-cloud/
```

or set `S3_DEST` as in [Surviving a spot interruption](#surviving-a-spot-interruption) and run `aws s3 sync` from your
laptop.  A git push from the instance needs a deploy key or a token on it; if you do that, push the results file alone
to a branch.  The results file is a valid copy at any moment: the script appends and syncs lines one at a time, and the
reader drops a torn last line.

### The spending, and the end

The bill is the instance's hours, not the search's.  For each instance, find its launch and termination times
(`aws ec2 describe-instances --instance-ids I --query 'Reservations[].Instances[].[LaunchTime,StateTransitionReason]'`
while it exists; the console's instance history or CloudTrail afterwards), multiply the hours by the spot price of those
hours (`describe-spot-price-history`: [machine.md](machine.md)), and set the product beside the estimate Brian approved.
Report the job's own measure of work beside it where it has one: the thread-hours of the prover's searches
(`summarize.py exact`, [Bring the data home, verify, commit](prover.md#bring-the-data-home-verify-commit)), or those of
the certificates against the plan's ([Results home, verification, and the
spending](certify.md#results-home-verification-and-the-spending)).  The "Cost" paragraph of the calibration entry is the
model.  Then do the end of the [rules for every launch](#rules-for-every-launch): terminate what is left, confirm that
nothing of the project remains, and delete the launch's security group.

## When something goes wrong

Most failures leave a message in the logs of `~/taxman-work/logs/`; the table gives the cause and the remedy of each.  A
job's own failures are in its section: the prover's error stop in [Watch, stop, resume](prover.md#watch-stop-resume),
the run to the wall's in [What was tested, and what can go wrong](wall.md#what-was-tested-and-what-can-go-wrong), and a
counterexample to U in [Watch, stop, resume, and a counterexample](certify.md#watch-stop-resume-and-a-counterexample).

| what you see | cause | remedy |
|---|---|---|
| `DIR is in use by another process (process N): DIR/... is locked` | another solver uses that checkpoint directory | find it with `pgrep -a taxman`; a lock is released when its process ends, however it ends |
| `FILE is in use by another process (another run appending to it?): it is locked` | another solver appends to that results file (`--out`) | the same |
| `another chain run is going (it holds ~/taxman-work/chain.lock)` | a second `chain.sh`; `prover.sh` says `another prover run` | `~/taxman-work/chain.pid` (or `prover.pid`) names the first |
| `FILE was made with different search tactics: ...; resume it with FLAGS, or delete it to start the game over` | the run's `--tactic` flags differ from the checkpoint's | resume with the flags it prints (`EXTRA_ARGS`), or delete the checkpoint to start that game over |
| `cannot resume from FILE: ...` for another game or build | the checkpoint is not this game's or this build's | delete the named `.ckpt` file; the game starts from its beginning |
| the solver exits with status 137 | a SIGKILL: the OOM killer, or a shutdown | `chain.sh` retries a crash; if it repeats, read `dmesg` (memory use is small: [machine.md](machine.md)) |
| `rustc ... is older than 1.89 and there is no rustup to update it`, or an error about `try_lock` | the engine needs `File::try_lock`, stable from Rust 1.89 | install rustup, with which `setup.sh` updates stable itself, or a Rust of 1.89 or later; by hand, `rustup update stable` |
| `DIR is on a memory filesystem: it would vanish with the instance (set TAXMAN_WORK)` | `/tmp` or `$TAXMAN_WORK` is tmpfs | set `TAXMAN_WORK` to a directory on EBS ([Storage](machine.md#storage)) |
| `installing packages needs root or sudo (or SKIP_PACKAGES=1 if git, cc, python3 and curl are there)` | `setup.sh` runs as a user without sudo | run it as a user with sudo, or install those four and set `SKIP_PACKAGES=1` |
| a step sent with `aws ssm send-command` runs as root, or its quoting is mangled | `send-command` runs as root, and `sudo -i` mangles a multi-line `bash -lc '...'` | put the step in a script file and run `sudo -u ec2-user -i bash FILE` ([Steps through SSM, or SSH](#steps-through-ssm-or-ssh)) |
| the SSH connection dropped | the run is in tmux | reconnect and `tmux attach -t NAME` (`tx` for the chain and the sweep, `pv` for the prover, `cert` for the certificates); if tmux is gone, the machine rebooted (next row) |
| the machine rebooted | tmux and the job are gone; the state is in `~/taxman-work`, and the build survives | run the job's command again (`chain.sh N`) |
| `eff` low and `idle` high on a short game at 192 threads | expected ([Reading the sweep](before-a-big-run.md#reading-the-sweep)) | judge the long games |
| `lock` of several percent on long games | the shared deque is contended | report it; fewer threads is the remedy at hand (`THREADS=96`) |
