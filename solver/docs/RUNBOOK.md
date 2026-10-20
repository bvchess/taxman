# Running the solver, and the prover's proofs, on a cloud machine

This page is the procedure for running the frame solver and the prover on an AWS machine, for Brian or an agent he has
approved.  The target is a Graviton instance of 4 to 64 vCPUs (`c8g`, `c9g` or `c7g`) running Linux, for hours or days
(section 1, "Instance"); the prover's proofs (section 8) need only 4 to 16.  The scripts take the thread count from the
machine, and they also work on x86_64, on Amazon Linux 2023 and on Ubuntu.  [The frame solver's
README](../frame/README.md) says what the solver is, and [the reference](REFERENCE.md) says how to read its output.

Read the [rules for every launch](#rules-for-every-launch) before you start anything.  Section 1 chooses the machine and
section 2 launches it.  Section 3 is what to run before a big job.  Sections 4 to 9 are the jobs, one each; each says
what it computes, what it costs, its command, and only what differs from section 2.  Section 10 is for when something
goes wrong, section 11 describes the build cache, and section 12 lists the scripts.

* [Rules for every launch](#rules-for-every-launch)
* [1. Choosing the machine](#1-choosing-the-machine)
* [2. Every launch, step by step](#2-every-launch-step-by-step)
* [3. Before a big run](#3-before-a-big-run)
* [4. The production chain](#4-the-production-chain)
* [5. The unattended run to the wall](#5-the-unattended-run-to-the-wall)
* [6. The ledger's experiment](#6-the-ledgers-experiment)
* [7. The sweep](#7-the-sweep)
* [8. The prover's proofs of opt(n)](#8-the-provers-proofs-of-optn)
* [9. Certifying U (stage 4)](#9-certifying-u-stage-4)
* [10. When something goes wrong](#10-when-something-goes-wrong)
* [11. The build cache and the release suite](#11-the-build-cache-and-the-release-suite)
* [12. The scripts, and what has run where](#12-the-scripts-and-what-has-run-where)

## Rules for every launch

These hold for every instance this project starts: a trial of the scripts, the prover's proofs, the chain, a
calibration.  The jobs link here instead of repeating them.

* **Get Brian's approval first.**  An instance, a volume, a key pair, a security group, a spot request, a bucket and a
  role each cost money or change the AWS account, so each needs his explicit yes and a cost estimate before you make the
  call.  Reading (describe, the spot price history, quotas) needs none.  The estimate is the hours times the spot price
  of the hour: read the history first (section 1, "Quota, prices and spot interruptions"), and let the timer, below,
  bound the hours.
* **Use spot, on `c8g`.**  Spot is the default (section 1, "Spot or on demand", says where an interruption would ruin a
  run), on the `c8g` family (Graviton4), in the sizes that the calibration settled on: `c8g.xlarge` to try the scripts,
  `c8g.4xlarge` for the prover's proofs (section 8.3), `c8g.16xlarge` for the chain.  The entry [Graviton calibration
  (2026-10)](../notebook/graviton-calibration-2026-10/README.md) holds the measurements, with their scripts and data.
* **Set a timer on every instance.**  Launch with `--instance-initiated-shutdown-behavior terminate`, and put
  `shutdown -h +MINUTES` in the user data, with MINUTES a little past the run's own deadline (`DEADLINE_HOURS`, section
  2.5).  Any shutdown (the timer, `POWEROFF`, the spot service) then terminates the instance, so a forgotten instance
  ends by itself.  The timer is the backstop that depends on no script.
* **Tag everything.**  Put `Project=taxman` and `Purpose=...` (`trial`, `prover`, `chain`, `wall`, `ledger`, `sweep`,
  `certify`) on everything the launch creates that takes a tag: the instance, its volumes and spot request, a key pair,
  a security group.  The check at the end finds them by `Project`.
* **Take the binaries from the build cache.**  A machine does not build what a machine has already built and tested.
  `setup.sh` takes the commit's release binaries from S3 (section 11) and, when the cache has none, builds only what the
  job needs (`BUILD`).  The release suite runs once for each commit and CPU, on one machine with every vCPU busy, before
  `setup.sh` stores the binaries (`RELEASE_TESTS=1`); a failing suite stores nothing, and the job does not start.
* **Copy off what must outlive the instance.**  Termination deletes the root volume, so copy the results first
  (`S3_DEST` or `rsync`, sections 2.5 and 2.7).
* **End clean.**  Once the results are home, terminate the instance and confirm that nothing of the project remains, in
  the region of the launch.  Each command prints `[]`; the CLI takes its profile from `AWS_PROFILE`, and section 2.1
  shows signing in:

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
  the role and instance profile `taxman-ec2` (section 2.1) for the next run; a finished run's prefix in the bucket can
  go once its results are home.

## 1. Choosing the machine

Every launch decides four things first: the instance, spot or on demand, the disk, and whether the quota allows it.
Each job (sections 4 to 9) names the instance that suits it; this section says why.

### Instance

* **The workload is CPU-bound.**  It is integer and bit-set code, with no floating point and no SIMD requirement, in one
  process with as many threads as CPUs.  Memory is small: the chain needs 400 to 540 MB with the results file loaded,
  about 1 GB at 192 threads and 2.3 GB at n = 4000, and a proof needs 370 MB at most on 64 threads ([the
  calibration](../notebook/graviton-calibration-2026-10/README.md), [the scaling
  run](../notebook/scaling-to-192-threads-2026-10/README.md)).  Each thread also reserves 64 MiB of address space for
  its stack, which is not memory.  Disk I/O is one checkpoint every 300 s by default: tens of KB at 2 threads and a few
  MB at most at 192, since a checkpoint holds the work stacks of all workers.  So buy the cheapest vCPUs, with 1 GiB per
  vCPU or less.
* **Use a mid-size instance, 16 to 64 vCPUs,** for hours or days, rather than the largest one for minutes (`c8g.4xlarge`
  to `c8g.16xlarge`, or the same sizes of `c9g` or `c7g`).  Graviton has no SMT, so a vCPU is a core and each worker
  thread gets a whole one; on Intel a vCPU is a hyperthread, and the throughput per thread is lower.  The price of a
  vCPU-hour changes little with the size (below), the largest spot pools are the thinnest, and the prover's proofs need
  no more than 16 vCPUs.  A new account's quota is 5 vCPUs; the owner's account allows 640 on spot (below).  Every
  script takes the CPU count from the machine, so any size works, up to a 192-vCPU `c8g.48xlarge`, if you have the quota
  and want the chain in hours.  On x86_64, compare the nodes per second per thread (`k/s/thr` in the sweep) before you
  choose.
* **Choose the family.**  `c8g` (Graviton4) is the safe default, and what the calibration chose.  `c9g` (Graviton5) is
  up to 25 % faster per vCPU by AWS's
  [figure](https://aws.amazon.com/about-aws/whats-new/2026/06/ec2-c9g-c9gd-instances-graviton5-processors-available/), a
  maximum that nobody measured here (AWS reclaimed the calibration's `c9g` before it ran), for 9 % more, and it is
  offered in us-east-1, us-east-2, us-west-2 and eu-central-1 only.  `c7g` (Graviton3) is the cheapest per vCPU-hour and
  slower.  The `m` families give 4 GiB per vCPU, which the run does not use (a thread needs about 25 to 50 MB), for 12 %
  more per vCPU.
* **Try a small instance of the same family first** (`c8g.xlarge`, 4 vCPUs, which fits the default quota; or `2xlarge`).
  It has the same CPU, so the same build and the same per-thread speed, for a sixteenth of the price or less.  It is
  where you find setup problems on Linux, test failures, and the single-thread rate and the 1-to-4-thread part of the
  sweep (section 3.2).
* **Check `lscpu` on a large machine** for the number of NUMA nodes.  The engine does not pin threads: each searcher
  allocates its own state on its own thread, so first touch keeps it local.  If there are several nodes and the sweep
  shows a step in `k/s/thr` where the thread count crosses a node, that is the cause.  The 64-vCPU instances of the
  calibration were one socket and one node (`solver/notebook/graviton-calibration-2026-10/machines/`).
* **Build on the machine type that runs the binary.**  A plain `cargo build` makes generic code; `-C target-cpu=native`
  is opt-in ([README, section 2](../frame/README.md#2-build-test-run)).  These scripts add it through `RUSTFLAGS`, the
  default of `env.sh`; set `RUSTFLAGS=` to build generic code.  `native` tunes for the CPU that builds, so a binary
  built on a newer CPU can fail with an illegal instruction on an older one.  The build cache (section 11) therefore
  keys its binaries by the CPU that `native` resolved to, and a machine takes only binaries built for its own CPU.
  `setup.sh` prints that CPU in its `native cpu:` line, which should name a Neoverse CPU (`neoverse-v2` on Graviton4),
  not `generic`.  That line is the only place the logs name the CPU: `env_report` greps `lscpu` for `^Model name`, and
  on Graviton4 with Amazon Linux 2023 `lscpu` prints only `BIOS Model name: AWS Graviton4`.  Nobody has measured the
  gain of `native` on Graviton; expect 0 to 3 % (on the M1 it measured 0 %).
* **Profile-guided optimization** (PGO) is optional.  `solver/tools/pgo.sh` makes the build (its header says how); on
  the M1 it adds 0 to 2 % ([DESIGN.md](DESIGN.md#10-performance)).  PGO changes code generation only, so the search and
  its answers stay the same, and either binary can resume a checkpoint.  `TAXMAN_BIN` and `LADDER_BIN` point the scripts
  at another binary.  Keep a non-PGO build too if you want to profile.

### Spot or on demand

* **Use spot for the checks of section 3 too,** as one-time requests.  An interruption in the middle of a measurement
  ruins it, and you repeat it: AWS took back the calibration's `c9g.16xlarge` five minutes in, at a cost of $0.06, and
  on demand costs 3 to 4 times as much a vCPU-hour (below).  Use on demand for a measurement you cannot afford to lose,
  or when the spot pool has no capacity.
* **Use spot for the production chain.**  The run suits it: a stop by SIGTERM loses nothing, a kill loses at most one
  checkpoint interval (300 s), and `chain.sh` is safe to start again.  An interruption really costs the time to get a
  machine again (the spot pool of a large size can be thin) and your attention.  If you cannot get one, fall back to on
  demand or to a smaller size (`THREADS` can differ between resumes).  Run `spot-watch.sh`, which turns the two-minute
  notice into a clean stop and a save.
* **A persistent spot request** launches a replacement after an interruption.  That is convenient, and it is also a way
  to keep paying for something nobody watches.  If you use one, `DEADLINE_HOURS` and `POWEROFF` (section 2.5) protect
  you, and you cancel the request at the end like everything else ([rules for every launch](#rules-for-every-launch)).
* **Name a price ceiling.**  `aws ec2 run-instances` needs `SpotOptions` with a `MaxPrice` in its market options
  (`MarketType=spot,SpotOptions={MaxPrice=PRICE}`, PRICE in dollars an hour); `MarketType=spot` alone does not work.
  The instance pays the spot price of the hour, never the ceiling, which only bounds the price at which AWS takes it
  back.  Brian approves a cost estimate at the spot price, and the ceiling goes above it.  Section 2.2 shows where it
  goes.

### Storage

Put checkpoints and results on a disk that survives a stop or a reboot and has local semantics (`flock`, `fsync`).  Use
the root EBS volume (gp3; 40 GB is plenty, since the toolchain and `target/` take 3 to 4 GB and the results about 2 KB
per game), or a separate EBS volume that you mount and name with `TAXMAN_WORK`.  Turn "delete on termination" off for a
separate volume if the state should outlive the instance; the end of the run deletes it with the rest.  Avoid three
things:

* **tmpfs.**  `/tmp` is tmpfs on Amazon Linux 2023, and `chain.sh` refuses a memory filesystem.
* **Instance-store NVMe as the only copy.**  A stop or a termination loses it.
* **A network filesystem.**  It lacks local `flock`, and each checkpoint does an `fsync`.

### Quota, prices and spot interruptions

* **Quota.**  AWS's documented default for a new account is 5 vCPUs for Standard (A, C, D, H, I, M, R, T, Z) On-Demand
  instances and 5 for Standard Spot requests, per region, raised automatically with use ([EC2 instance type
  quotas](https://docs.aws.amazon.com/ec2/latest/instancetypes/ec2-instance-quotas.html),
  [On-Demand](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-on-demand-instances.html),
  [Spot](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-spot-limits.html)).  A 4-vCPU shape fits, and an
  8-vCPU shape does not.  The owner's account, read with the AWS CLI in us-east-1 on 2026-10-14, allows **1280 vCPUs
  On-Demand and 640 Spot**, so any shape here launches without a request.  To read it again, use the console (Service
  Quotas, Amazon EC2, the region at the top right, "Applied quota value" of "Running On-Demand Standard (A, C, D, H, I,
  M, R, T, Z) instances" and "All Standard (A, C, D, H, I, M, R, T, Z) Spot Instance Requests") or:

  ```
  aws login --profile bvc                      # short-lived credentials; or export AWS_PROFILE=bvc after it
  aws service-quotas get-service-quota --profile bvc --region us-east-1 --service-code ec2 \
      --quota-code L-1216C47A --query Quota.Value                                            # on demand
  aws service-quotas get-service-quota --profile bvc --region us-east-1 --service-code ec2 \
      --quota-code L-34B43A08 --query Quota.Value                                            # spot
  ```

* **Prices.**  The table gives Linux prices in US East per vCPU-hour.  On-demand prices come from Vantage's pages of
  2026-10-14; spot prices are the cheapest zone of the owner's price history of us-east-1 on 2026-10-14, divided by the
  vCPUs.  Spot moves by the hour, so read the history before you launch:

  ```
  aws ec2 describe-spot-price-history --instance-types c8g.16xlarge --product-descriptions Linux/UNIX \
      --start-time $(date -u -v-3H +%Y-%m-%dT%H:%M:%SZ) --query 'SpotPriceHistory[].[AvailabilityZone,SpotPrice]' \
      --output text                                          # GNU date: -d '3 hours ago' in place of -v-3H
  ```

  | family | CPU | on demand | spot: 16 vCPUs | 32 | 64 | 96 | 192 |
  |---|---|---|---|---|---|---|---|
  | `c7g` | Graviton3, 2.5 GHz | $0.0363 | 0.0126 | 0.0123 | **0.0097** | | |
  | `c8g` | Graviton4, 2.7 GHz | $0.0399 | 0.0149 | 0.0129 | 0.0106 | **0.0095** | 0.0104 |
  | `c9g` | Graviton5, 3.3 GHz | $0.0435 | | | 0.0119 | | 0.0123 |
  | `m8g` | Graviton4, 4 GiB a vCPU | $0.0449 | | | | | |

  Within a family, a vCPU-hour costs the same on demand at every size and varies by up to 1.5 times on spot, so wall
  time chooses the size, not price.
* **Spot interruptions.**  The Spot Instance Advisor reports the share of instances interrupted in the trailing month
  (the [public data](https://spot-bid-advisor.s3.amazonaws.com/spot-advisor-data.json) behind the [advisor
  page](https://aws.amazon.com/ec2/spot/instance-advisor/), fetched 2026-10-14).  For us-east-1 it said:

  | family | 4 vCPUs | 8 | 16 | 32 | 64 | 96 | 192 |
  |---|---|---|---|---|---|---|---|
  | `c8g` | above 20 % | 10 to 15 % | above 20 % | 5 to 10 % | 5 to 10 % | above 20 % | 10 to 15 % |
  | `c7g` | | | under 5 % | under 5 % | 5 to 10 % | | |

  It did not list `c9g`.  At even 20 % a month, a run of 12 hours has a chance of about one in 300 of an interruption if
  the rate is uniform, and an interruption costs a few minutes and, for the prover, one pass (section 8.1).  So spot is
  fine; for the prover's proofs on 16 vCPUs, on demand costs about $4 to $7 more.

## 2. Every launch, step by step

Every job of sections 4 to 9 launches the same way.  This section has the steps; each job says only what differs: its
instance, its timer, its setup and its command.

### 2.1 Access, and what the account holds

A launch needs no SSH.  The instance has an instance profile (for S3 and SSM), its security group has no inbound rule,
and it has no key pair; you send it the steps through AWS Systems Manager (SSM) with `aws ssm send-command`.  SSH is the
fallback (section 2.3).  The [rules for every launch](#rules-for-every-launch) govern all of it.  Run every `aws`
command as `AWS_PROFILE=bvc` in us-east-1:

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

### 2.2 Launch

Each job gives its own instance type, volume, `Purpose` and timer.  The values below are the prover's (section 8): one
`c8g.4xlarge` on spot, Amazon Linux 2023 arm64 (user `ec2-user`; Ubuntu 22.04 or 24.04 arm64 has `ubuntu`), no key pair,
in the zone with the lowest price in the spot price history.  The timer is the job's `DEADLINE_HOURS` plus an hour, in
minutes: 36 + 1 hours is 2220.  The ceiling `MAX_PRICE` goes above the spot price (section 1, "Spot or on demand").
`run-instances` resolves the AMI from an SSM parameter; check the parameter first with
`aws ssm get-parameter --name ...` if you like.

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
steps to the user data (sections 6 and 7.3).  To launch from the console, set the same fields: the image, the volume, no
inbound rule, the instance profile, shutdown behaviour "terminate", the timer in the user data, and the tags with a
`Name`.  For SSH access, see section 2.3.

### 2.3 Steps through SSM, or SSH

**Steps through SSM.**  The instance is ready for commands when
`aws ssm describe-instance-information --filters Key=InstanceIds,Values=$I --query 'InstanceInformationList[].PingStatus'`
says `Online` (`$I` is the instance id from section 2.2).  A step is then a `send-command` and a read of its result:

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
results (section 3.2).

### 2.4 Setup

On the instance, as `ec2-user` (over SSH, or through SSM as section 2.3 says):

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
  when it finds them (section 11); otherwise builds with `RUSTFLAGS="-C target-cpu=native"` the solver and the examples
  `ledger`, `perf` and `ladder` (`STUDIES=1` adds the study tools `tune`, `extended`, `structure` and `wall`, which a
  default build skips);
* replays `optimal.json`, runs both result verifiers on the committed files, and runs `taxman regress --to 400`;
* runs `cargo test -- --test-threads=4`.

Its output is also in `~/taxman-work/logs/setup.log`.  Run it again to pick up later commits; a reboot loses nothing
it made.
`RELEASE_TESTS=1 solver/tools/cloud/setup.sh` puts the tested binaries into the cache (section 11).

### 2.5 Cost guardrails

These variables bound a run.  `DEADLINE_HOURS`, `POWEROFF`, `BUDGET_HOURS` and `RUN_UNTIL` belong to `run.sh`, so every
long-run script reads them; `GAME_HOURS` is `chain.sh`'s.  The table says what `chain.sh` does.

| variable | effect |
|---|---|
| `DEADLINE_HOURS` | after that many hours from the first start (on a replacement machine, from its own first start: section 2.6), `chain.sh` stops the solver cleanly with a final checkpoint and ends with status 3 |
| `POWEROFF` | `done` powers the machine off when the range is complete; `always` powers it off whenever `chain.sh` ends for any reason, which needs passwordless `sudo`, as on both AMIs |
| `GAME_HOURS` | the wall: the chain stops at the first game whose own search has passed that many hours, over every run of it, keeping every checkpoint and recording nothing for the game; the log says `wall at N after H hours`, and the status is 4 |
| `BUDGET_HOURS` | instance-hours over every machine of the run, counted in the ledger in S3; the solver stops cleanly five minutes before they are used up, with status 3 like `DEADLINE_HOURS` |
| `RUN_UNTIL` | a date, treated like `BUDGET_HOURS` |

Use `POWEROFF=always` for an unattended run, since an idle machine after an error or a deadline costs as much as a busy
one.  `chain.sh` saves to `S3_DEST` before it powers off, which matters because a shutdown terminates the instance.
Section 5 puts the budget and the date together with a group that restarts the machine.  The timer (a backstop that
depends on no script), the tags and the cleanup are in the [rules for every launch](#rules-for-every-launch); a budget
alert on the account, set before the first launch, is the other half.

### 2.6 Surviving a spot interruption

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
     `BUDGET_HOURS` and `RUN_UNTIL` live in S3 ([section 5](#5-the-unattended-run-to-the-wall)).

### 2.7 Getting the results off the machine

Pull from your own machine, so that the instance holds no credentials:

```
rsync -av ec2-user@HOST:taxman-work/results/ ./results-from-cloud/        # ubuntu@HOST on Ubuntu
rsync -av ec2-user@HOST:taxman-work/logs/ ./logs-from-cloud/
```

or set `S3_DEST` as in section 2.6 and run `aws s3 sync` from your laptop.  A git push from the instance needs a deploy
key or a token on it; if you do that, push the results file alone to a branch.  The results file is a valid copy at any
moment: the script appends and syncs lines one at a time, and the reader drops a torn last line.

### 2.8 The spending, and the end

The bill is the instance's hours, not the search's.  For each instance, find its launch and termination times
(`aws ec2 describe-instances --instance-ids I --query 'Reservations[].Instances[].[LaunchTime,StateTransitionReason]'`
while it exists; the console's instance history or CloudTrail afterwards), multiply the hours by the spot price of those
hours (`describe-spot-price-history`: section 1), and set the product beside the estimate Brian approved.  Report the
job's own measure of work beside it where it has one: the thread-hours of the prover's searches (`summarize.py exact`,
section 8.6), or those of the certificates against the plan's (section 9.9).  The "Cost" paragraph of the calibration
entry is the model.  Then do the end of the [rules for every launch](#rules-for-every-launch): terminate what is left,
confirm that nothing of the project remains, and delete the launch's security group.

## 3. Before a big run

Check a long run on a big instance in three steps, the cheapest first, so that each step finds the problems that the
next one would cost more to find:

1. **Optional, on your machine:** build and test in Docker (section 3.1).
2. **A small instance of the same family,** on spot, for an hour or two (cents): `setup.sh`, the tests and the scaling
   sweep up to its CPU count (section 3.2).
3. **The production instance** (section 1: mid-size), on spot, for the checks: the large-machine tests, the sweep to its
   CPU count, the cost of a checkpoint pause, and a rehearsal of the whole chain to 1231 with deliberate interruptions
   (sections 3.3 to 3.7).

Then start the production chain (section 4), from n = 1 or from the committed `results/strict.jsonl` ([its
range](../../results/README.md)), in tmux, on spot, with a deadline and a copy of the state kept off the machine.

### 3.1 Dry run in Docker (optional)

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

### 3.2 A small instance first

1. Launch it (section 2.2 with `c8g.xlarge` or `c8g.2xlarge` and `Purpose=trial`), on spot, with a timer of 180 minutes.
2. Get the code and set up, as section 2.4 says (5 to 10 minutes).
3. In `tmux new -s tx` (detach with Ctrl-b d, return with `tmux attach -t tx`), run the sweep:

   ```
   solver/tools/cloud/scaling.sh                        # brief: its default games are short (section 3.4)
   ```

   Section 3.5 reads the tables.  From this machine, take three numbers: the single-thread rate (`k/s/thr` at 1 thread:
   an M1 Max does 12.3 at n = 1215, the sample of section 3.5, and the calibration found a Graviton4 core runs the
   solver at 0.9 of an M1 Max core, so expect about 11 from a `c8g`), the speedup to the machine's threads, and the
   `Maximum resident set size` in the log (GNU `time` reports it, if installed).
4. Try the pieces that will matter on the big machine, on a small chain:

   ```
   TAXMAN_WORK=~/small EXTRA_ARGS="--time-limit 2" solver/tools/cloud/chain.sh 1000
   ```

   Every search that takes more than 2 seconds stops, and the next pass resumes it from its checkpoint.  The log shows
   the passes ("going on"), and it must end with "the range 1-1000 is complete", both referees agreeing.
5. Copy the logs home (`scp -r HOST:taxman-work/logs .`), then do what the rules say for the end: terminate the instance
   and confirm that nothing remains.

### 3.3 The big instance: correctness

Launch the production instance (section 1) on spot, with a timer of 240 minutes, and run `setup.sh` as in section 2.4.
Use tmux for everything.  Count on one to one and a half hours, about $1 on a `c8g.16xlarge` at the spot price of
2026-10-14 ($0.68 an hour).

```
cd ~/taxman/solver/frame
cargo test -- --test-threads=4                                       # also run by setup.sh
cargo test --release --test engine -- --ignored --nocapture --test-threads 1 many_threads
TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored stress
TXENGINE_CONSERVATION=500 cargo test --release --test robustness conservation
```

* **The suite** takes about 30 s after compiling, with two or four test threads ([DESIGN.md, section
  9](DESIGN.md#9-testing)).  It includes the node-count tripwires (`tests/tripwires.rs`), which assert exact trees on
  one thread, and so check the arithmetic and the search against a different CPU and compiler.
* **`many_threads`** runs the engine on a toy problem at 64 and at 192 workers: answers, splitting, checkpoints every 2
  ms, spilling, a panicking worker.  `--nocapture` prints each run's counters (`eff`, `splits`, `full`), the first look
  at 192 threads.
* **`stress`** draws random engine configurations (1 to 192 threads, tiny queues, checkpoints every few ms, limits) and
  runs each to completion through any number of resumes.  5000 iterations take 200 s on an M1; a failure names the
  configuration, and `TXENGINE_STRESS_SEED=n` draws others.
* **`conservation`** checks that no unit of work is lost or done twice, by counting the nodes of a tree with no pruning.

All of these must pass, or the machine is not ready.  Save the output.

### 3.4 The scaling sweep

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
entry](../notebook/scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine); the [scaling
run](../notebook/scaling-to-192-threads-2026-10/README.md) ran them once on a `c8g.48xlarge`.

### 3.5 Reading the sweep

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
| `k/s/thr` | thousands of nodes per second per thread | the engine's efficiency apart from the tree size. It falls a little with more threads (11 % from 1 to 4 threads above; 93 to 94 % of the one-thread rate at 64 threads on Graviton3 and 4, in the [calibration](../notebook/graviton-calibration-2026-10/README.md)), and it depends on the game (12.3 at n = 1215 above, 8.3 at 2172 on one thread: [round B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)). A step down at one thread count suggests memory, frequency or NUMA |
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
| `runs`, `tiny` | units run, and those that polled once or never (nothing left in them) | compare the share with the rows of few threads; ["The engine"](../notebook/the-engine/README.md) records 13 of 227 at 2 threads |
| `full s` | time until every worker was running at once | milliseconds at 8 threads (0.9 to 2.8 ms seen); expect tens of ms at 192; seconds would mean the split protocol ramps up too slowly |

Notes on judging it:

* **Time is the measure; the speedup mixes two effects.**  Each thread keeps its own table of learned conflicts
  (nogoods) and forgets it every 50,000 of its steps, so the tree differs with the thread count.  The proof of 2172 took
  1.43M nodes on 1 thread and 1.27M on 4 on a Graviton4 core, and sped up 4.5x on 4 threads while the node rate sped up
  4.0x ([calibration](../notebook/graviton-calibration-2026-10/README.md), which has 16 and 64 threads too).
  Speedup(time) = speedup(nodes per second) × (nodes at 1 thread / nodes at T threads).  Judge the engine by `k/s/thr`
  and `eff %`, the search by `nodes`, and the result by time.
* **Games must be long enough for the thread count.**  A proof of 1215 takes 0.3 s on one M1 thread and one of 2172 a
  minute: at 64 or 192 threads that is milliseconds to seconds, of which the ramp-up and the tail are a large part.  An
  `eff %` below 100 at 192 threads for such a game is expected and says nothing about long games.  The long games are
  the dear ones past 2175 (in the calibration's chain on 64 threads, 2184 took three minutes and 2260 a minute and a
  half: `solver/frame/testdata/c8g.chain-2176.jsonl`), and the chain's progress lines are the real test: section 4.3
  lists what a healthy line shows.
* **Memory** is the `Maximum resident set size` line near the end of the log, if GNU `time` was available.
* **Compare with the small instance:** the 1- and 8-thread rows of both should agree on `k/s/thr`.
* **Stop if** `lock %` reaches several percent or `full s` reaches seconds at 192 threads on a long game, if `k/s/thr`
  at 192 falls below half that at 1 thread (the machine, not the code, is probably the limit, and a smaller instance is
  cheaper for the same work), or if any test fails.

### 3.6 The cost of a checkpoint

`scaling.sh --pauses` solves game 1230 (from the committed chain up to 1229) at all CPUs, with a checkpoint every 0.5 s,
and prints the progress line of the end: `pauses N (x%) max Yms park Zms`.  `x` is the share of the workers' time spent
parked in checkpoints at that absurd rate; at the real interval of 300 s the share is 600 times smaller.  `max` is the
longest pause (the wait for the slowest worker, then writing and `fsync`-ing a few MB to EBS), and `park` the longest
wait for the workers to park (10 to 15 ms at most on the M1).  A `max` of a few hundred milliseconds is fine at a
300-second interval; seconds would mean slow storage, and `CHECKPOINT_INTERVAL` can go up further.

### 3.7 Rehearsal of the chain, with interruptions

This is the production run to 1231 in miniature, in a separate work directory, with every search that lasts more than 5
seconds stopped and resumed:

```
TAXMAN_WORK=~/rehearsal EXTRA_ARGS="--time-limit 5" solver/tools/cloud/chain.sh 1231
```

It takes minutes.  The log (`~/rehearsal/logs/chain.log`) shows each pass, and at the end `verify.sh` runs both referees
on the rehearsal's file and the committed one.  **It passes if the chain completes, the log says "both referees agree",
and `verify.sh` prints `1231 board sizes are in more than one file, with equal scores in all`** (the scores of 1..1231
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

## 4. The production chain

### 4.1 What is being computed

`chain.sh N` runs `taxman solve 1-N` into one results file: every game from n = 1, each built from the one before.
Nothing comes from `optimal.json`, which the run only compares with, to n = 1300, so the chain assumes nothing.  The
committed `results/strict.jsonl` is exactly such a file: a laptop made its first 2175 games and cloud machines the rest
([results/README.md](../../results/README.md) has its range and how each part was made).  A run from n = 1 reproduces
its scores, and `verify.sh` checks the run against it.  The production chain can instead start from the committed file,
as section 4.2 says, so that the run solves only the games above its end.  The solver is the same code as on a laptop;
the machine only changes how long it takes.

**What it costs.**  The games to 2175 take minutes on a big machine.  Past 2175 the search time of a game grows about
tenfold for every 1,040 in n, in waves, and one day on 64 CPUs reached n = 3440 ([the chain to
3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md)).  A `c8g.48xlarge` ran the chain past 2225 1.9 to 2.7
times as fast as 64 threads, for 1.1 to 1.9 times the cost a game ([the scaling
run](../notebook/scaling-to-192-threads-2026-10/README.md)), and the
[calibration](../notebook/graviton-calibration-2026-10/README.md) gives the rate and the price of the games past 2175 on
a `c8g.16xlarge`.  Estimate the cost of a target N from those entries, at the spot price of the hour (a `c8g.16xlarge`
costs about $0.68 an hour), and let the first hours of the real run decide: the log has each game's seconds.  A single
hard game can take hours.  Choose N by money and patience; running `chain.sh` again with a larger N later continues the
same file.

### 4.2 Run

Start in tmux, with a deadline and with the state copied off the machine:

```
tmux new -s tx
DEADLINE_HOURS=30 POWEROFF=always S3_DEST=s3://taxman-013764204840-us-east-1/chain-run1 solver/tools/cloud/chain.sh 1400
```

The bucket is the one of section 2.1, with a prefix of its own for each run.  In a second window, on spot, start
`spot-watch.sh`.  It logs to `~/taxman-work/logs/spot-watch.log` as well as to its terminal, so its notice and its save
leave a trace after the window is gone; the output of the save is in `save.log` beside it.

```
solver/tools/cloud/spot-watch.sh
```

`chain.sh N` is the whole command.  It runs the chain 1..N into `~/taxman-work/results/strict.jsonl` on all CPUs, with
four games in flight at once (`WINDOW`), a checkpoint every 300 s and a progress line every 60 s.  The top of the script
documents its variables, all optional: `THREADS`, `WINDOW`, `CHECKPOINT_INTERVAL`, `PROGRESS`, `EXTRA_ARGS`,
`MAX_FAILS`, `S3_DEST`, `SAVE_EVERY`, and the guardrails of section 2.5 (`GAME_HOURS`, `DEADLINE_HOURS`, `BUDGET_HOURS`,
`RUN_UNTIL`, `POWEROFF`; the budget and the date outlive the machine, section 5).

`WINDOW` is the number of consecutive games kept in flight at once (`taxman solve --window`).  While the solver still
proves game n, it sets up and searches game n + 1 from n's best game so far, and the threads move to the searches that
keep them busy, so one game's search does not leave a machine of many threads idle.  Games still conclude and record in
order, with the scores, proofs and repeats of a window of 1, which solves them one after the other ([the
README](../frame/README.md#games-in-flight); [DESIGN.md](DESIGN.md#the-window-games-in-flight)).  The default is 4, in
`chain.sh` and in `taxman solve --window`: on 64 and 192 threads the [scaling
run](../notebook/scaling-to-192-threads-2026-10/README.md) found 4 the best of 1, 4, 16 and 32 on the games past 2225,
and a window of 16 20 to 32 % slower than 1 on 64 threads.

* **Where the chain starts.**  From an empty `~/taxman-work`, `chain.sh` solves from n = 1, which reproduces the
  committed `results/strict.jsonl` (the scores equal; the moves may differ).  The first 2175 games take minutes on the
  big machine, and `verify.sh` then checks the machine and the build against the committed file.  To skip those games,
  copy the committed file in before the first run, once:

  ```
  mkdir -p ~/taxman-work/results && cp results/strict.jsonl ~/taxman-work/results/strict.jsonl
  solver/tools/cloud/chain.sh 4095                    # skips the games it holds and solves the next on
  ```

  `chain.sh` checks the file when it opens it.  The file assumes nothing (it is a chain from n = 1), holds no incomplete
  line, and its lines keep the builds that made them ([results/README.md](../../results/README.md) names them), as the
  lines added will keep the cloud's.  The comparison of `verify.sh` with the committed file is then trivial for the
  games it holds, so the referees alone check the games above them (each game replayed, the chain bounds,
  Franklín–Moniot, the heuristic chain); a run from n = 1 is the cross-check of the machine and the build.  On a
  replacement machine, restore the saved state instead (section 2.6), never the committed file over it.
* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/chain.pid)`.  The solver writes a final
  checkpoint, and `chain.sh` ends with status 143.  **Start again** by running the same command: it skips every
  completed game and resumes the game with a checkpoint.  The thread count, the window, `S3_DEST`, `DEADLINE_HOURS` and
  N may differ between runs.  The first deadline stays in `~/taxman-work/deadline`: delete the file to set another (a
  replacement machine: section 2.6).
* **What it does by itself.**  `chain.sh` retries a crash (a status other than 0, 1, 2, 124, 130 and 143) after 10 s.
  It does not retry an error status (1 or 2: a refused checkpoint, a game that failed a check), because it would repeat:
  read `~/taxman-work/logs/chain.err`.
* **Use the same build between a stop and its resume.**  A checkpoint records the game, the frames the build made, the
  candidates and the search tactics, and the solver refuses one from a different build with the reason: delete it, or
  run the old build.  The width of the sets is part of that: a build that gives a game another width refuses its old
  checkpoint.  Do not `git pull` in the middle of a long game without a reason.  Changing only code generation (PGO,
  `target-cpu`) is harmless.
* **Past 4095.**  The solver supports games up to n = 8191 (`taxman::solver::MAX_N`), those from 4096 with sets of 128
  words ([the entry on boards past 4095](../notebook/boards-past-4095-2026-10/README.md)).  A build from before commit
  4316168 stops at n = 4095, the widest set it has.  To take a chain past 4095, stop it (or let `chain.sh 4095` finish),
  build the newer commit in the same checkout (`git pull`, then `setup.sh`, or `cargo build --release` in
  `solver/frame`), and run `chain.sh N` again with N up to 8191 (or `CHAIN_TO=N`).  It skips the completed games, and
  the checkpoint of a game below 4096 that the old build wrote resumes, since the game keeps its width, frames,
  candidates and tactics (tried with checkpoints of 1133 and 3600).  The solver refuses a game past 8191 ("beyond the
  largest supported board"), which ends the script with an error.

### 4.3 Monitoring

```
tail -f ~/taxman-work/logs/chain.err          # the progress lines of the game in progress
tail -n 3 ~/taxman-work/logs/chain.out        # the last finished games
tail ~/taxman-work/logs/chain.log             # this script's own messages
tail -n 1 ~/taxman-work/results/strict.jsonl | cut -c1-40        # the last completed game
ls -l ~/taxman-work/ckpt                      # a checkpoint no older than ~2 minutes while a long game runs
htop                                          # all CPUs near 100 %
```

A healthy progress line at 192 threads shows `eff` of 95 % or more, `lock` under 1 %, small `idle`, `queue` of a few
hundred at most, `spilled 0`, `pauses N (x%)` with x under 1, and `best` rising early and then holding still (the
incumbent is often, not always, found long before the proof ends).  The node rate grows with the thread count: one M1
Max thread does 12 thousand nodes a second at n = 1215 and 8 thousand at 2172 (the table of [round
B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)).  On a Graviton4 core the proof of 2172 does 6.5
thousand a second on one thread, and the chain's searches of 2277..2336 did 3.3 to 6.2 thousand a thread on 192 threads
(0.6 to 1.2 million a second) and 3.9 to 5.3 thousand on 64 (0.25 to 0.34 million) ([the scaling
run](../notebook/scaling-to-192-threads-2026-10/README.md)).  Estimate when a long game will end from the previous ones:
the log has each game's seconds, and past 1300 they come in waves of a few hundred n, not on a smooth curve.

### 4.4 Verifying, and bringing the results home

`chain.sh` runs `verify.sh` when the range is complete.  On any copy of the file, anywhere:

```
solver/tools/cloud/verify.sh results-from-cloud/strict.jsonl
```

`verify.sh` runs `taxman verify-results` and `solver/tools/check_results.py` on the file together with the committed
`results/strict.jsonl`.  The two verifiers check that:

* every game replays legally, finishes, and opens with the largest prime;
* each score agrees with `optimal.json`;
* the chain bounds `score(n-1) <= score(n) <= n + score(n-1)` hold, and each line's arithmetic and provenance;
* the Franklín–Moniot bound and the best heuristic chain hold;
* every game that the file shares with the committed file has the same score.

For a chain from 1, the report says `assumptions: none in N of N games`.  Any assumption or problem line means the run
is not what it should be: stop and read the line.  (A count of incomplete lines is not a problem: every search that
stopped and resumed left one.)

Then decide what to keep.  The file takes about 2 KB a game.  Scripts must not overwrite the committed `results/`: put a
file next to them (say `results/strict-1-1400.jsonl`) and update `results/README.md`.  The scaling logs under
`solver/notebook/` (in a folder of their own, or in `solver/notebook/graviton-calibration-2026-10/`) are your call.

To extend the table of optimal games, run `taxman extend-optimal` on the verified file from a checkout:
`taxman extend-optimal results-from-cloud/strict.jsonl --out /tmp/optimal.json`.  It appends the games past the table's
end that the file completed, and says where it stopped.  Check the result with `taxman verify-optimal /tmp/optimal.json`
and `diff`, copy it over `results/optimal.json`, and rebuild, because the binary carries the table it was built with.

## 5. The unattended run to the wall

`wall.sh` runs the chain from n = 1 on one `c8g.16xlarge` (64 threads, window 4) at spot, with no one to watch it.  The
machine builds a pinned commit of master, re-solves the games of the committed `results/strict.jsonl` (to compare with
it at home, section 4.4), and goes on past them.  It stops by itself at the wall, when it has used its budget, or on a
date.  When AWS takes the machine away, a group starts another that goes on from the saved state.  [The run to the
wall](../notebook/the-run-to-the-wall-2026-10/README.md) records the first run and the rehearsal; [STATUS.md](STATUS.md)
holds the decision for the wall (Brian, 2026-10-16) and its reasons.  A launch needs Brian's approval, with the estimate
of section 5.3.

### 5.1 How it ends

The run ends at the first of these.  Each but the last is the run's end: the machine records it, takes the group to
capacity 0 and powers off, and nothing starts a machine in place of the one that ends it.

| end | what it is | the log says | status of `chain.sh` |
|---|---|---|---|
| **the wall** | a game's own search has passed `GAME_HOURS` (24): the hours it has been the oldest game of the window with its search running, over every run of it ([DESIGN.md, "The wall"](DESIGN.md#the-window-games-in-flight)); waiting behind another game does not count | `wall at N after H hours` | 4 |
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

### 5.2 The parts

| what | where | does |
|---|---|---|
| `wall.sh` | your machine | `launch`, `status`, `stop` and `cleanup`; it prints every aws command that changes something (`+ aws ...`; `--verbose` prints the reads too), `DRY_RUN=1` prints the changes without making them, and the files the commands take stay in `~/taxman-wall/RUN` |
| the launch template `taxman-wall`, the security group `taxman-wall` (no inbound rule), the group `taxman-wall` | the account | one `c8g.16xlarge` (or `INSTANCE_TYPE`) at spot, lowest price first among the zones (`SpotAllocationStrategy=lowest-price`, one pool), capacity rebalancing off, `min 0, max 1`, EC2 health checks, tags `Project=taxman`, `Purpose=wall` |
| the user data | the template | sets the timer, installs git and tmux, clones the repository, checks out the pinned commit and starts `wall-node.sh` in a tmux session `tx` as `ec2-user`; if any of that fails after five tries, it records the end and takes the group to 0 |
| `wall-node.sh` | each machine | restores the logs, runs `setup.sh` (the solver alone, `BUILD=--bin taxman`: from the build cache if it has the commit, tested; no tests of its own; its smoke checks run), then `save.sh --restore`; starts `spot-watch.sh` and a loop that looks for `run/stop`; runs `chain.sh` with the config; decides what its status means (5.1) |
| `run.sh`'s ledger | each machine | counts the budget and the date (below) |
| `wall-policy.json` | the role `taxman-ec2` | lets the machines set the capacity of this one group and read groups (5.3) |
| `test/run.sh` | your machine | tests these scripts against `test/bin/aws`, a stand-in over directories |

The run's state is one prefix of the project's bucket, `s3://taxman-013764204840-us-east-1/wall-RUN` (`RUN` defaults to
`run1` for `launch`; section 5.4 says what the other commands take).  It holds `results/`, `ckpt/` (with each game's
`nNNNN.clock` beside its checkpoint) and `logs/` as `save.sh` writes them, and `run/`:

| object | holds |
|---|---|
| `run/config` | the settings, which `launch` writes: `COMMIT`, `INSTANCE_TYPE`, `CHAIN_TO`, `WINDOW`, `GAME_HOURS`, `BUDGET_HOURS`, `RUN_UNTIL` (seconds since the epoch), `THREADS`, `EXTRA_ARGS`, `CHECKPOINT_INTERVAL`, `SAVE_EVERY`, `LEDGER_MARGIN`, `MAX_INSTANCES`, `PRICE`, and `RELEASE_TESTS` and `REQUIRE_CACHED` if the environment of `launch` had them (5.4). Every machine reads it, so a budget or a date changed there (by `launch --continue`) reaches the next machine |
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

### 5.3 What needs Brian's approval, and what it costs

The [rules for every launch](#rules-for-every-launch) cover these account changes: the inline policy `taxman-wall` on
the role `taxman-ec2`; a security group, a launch template, an Auto Scaling group and a scheduled action (all named
`taxman-wall`); the spot instances the group launches, and their volumes; and objects in the bucket.  The first Auto
Scaling group of the account also creates the service-linked role `AWSServiceRoleForAutoScaling`, which stays.
`cleanup` deletes the group, the template, the security group and the policy; the service-linked role, the bucket and
`taxman-ec2` stay on purpose.

**The policy** is [wall-policy.json](../tools/cloud/wall-policy.json).  It allows `autoscaling:SetDesiredCapacity` on
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

### 5.4 Launch, watch, stop, clean up

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
(section 11) it builds nothing, and every machine of the run, the replacements after an interruption included, takes the
same binary.  Without them it builds the solver (a few minutes on 64 vCPUs, untested, nothing stored).  Two settings in
the environment of `launch`, `RELEASE_TESTS` and `REQUIRE_CACHED`, go into the config for `setup.sh` on each machine
(section 11); neither is on by default.

* **Release first,** once for the commit on the CPU of the run (any `c8g` will do: the key is the CPU, not the size).
  Either run `RELEASE_TESTS=1 solver/tools/cloud/setup.sh` on one machine, as section 11.3 says, or set
  `RELEASE_TESTS=1` in the environment of `launch`, which has the first machine of the run do it before the chain starts
  (the solver's suite only, since the machine builds `taxman` alone: its entry serves this run and not a launch of
  anything else).  A suite that fails ends the run (`ended`: `setup.sh failed`) and leaves `RELEASE_TESTS_FAILED`, the
  failing tests, in the run's prefix in S3.
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
window](../frame/README.md#games-in-flight)):

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
section 4.4 says.  `verify.sh` must report that the games of the committed file agree with it with equal scores,
"assumptions: none", and both referees agreeing; the referees alone check the games past its end.  The checkpoint of the
walled game stays in `ckpt/`, because a run that goes on needs it.

**To go on** after the wall or the budget (a longer wall, more money), run
`wall.sh launch --continue --commit THE-SAME-COMMIT --until DATE --budget-hours H --game-hours G`.  Use the commit the
run began with, because a checkpoint resumes only on the build that made it (section 4.2), and give the budget as the
new total.  `--continue` removes the end and the stop, writes the config again, and keeps the ledger, the results and
the checkpoints.

**Drawing on the known games.**  The chain above searches every game from n = 1, which re-proves the committed chain
with a build.  A run that goes on past what the repository holds needs none of that.  With `--extra-args "--use-prev"`
(`EXTRA_ARGS`; the machines run `taxman solve 1-N --use-prev ...` in the checkout of the pinned commit), the chain draws
on the known games of that checkout's `results`: the committed chain `strict.jsonl`, the bounds store `bounds.jsonl`
([results/README.md](../../results/README.md#boundsjsonl) says what it holds) and the built-in `optimal.json` ([README,
"Known games"](../frame/README.md#known-games)):

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
  `ledger verify-bounds results/bounds.jsonl` on the store first (section 6), because the chain takes its bounds on its
  word.
* **A resumed machine** passes the flag again (it is in `run/config`, which every machine reads), and the results file
  it restores from S3 gets the same checks.  The checkpoint of a searched game resumes as ever.
* **The trade-off.**  Without the flag, the chain from n = 1 re-proves the committed chain (1 to 3440 took 1.76 hours on
  the machine in the run of 2026-10-17); with it, that stretch costs seconds and the run checks nothing of the build
  against the committed chain.  Use it for a run that goes past the repository's knowledge, and not for a run whose
  point is that check.

### 5.5 The rehearsal

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
  `solver/tools/cloud/verify.sh results-rehearsal-1/strict.jsonl` says N − 1 board sizes are in more than one file, with
  equal scores in all.
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
spot ceiling of $0.40 an hour.  The account changes are those of 5.3, with the spot instance of a `c8g.2xlarge` and the
group's termination by `aws ec2 terminate-instances`.

**What the rehearsal leaves out:** the 64-thread machine, the hours-long searches (the Rust tests check the clock at the
wall for the window and its resume, not for 24 hours), and the spot interruption notice itself.  `terminate-instances`
sends none, so the path through `spot-watch.sh` (a SIGTERM, the final checkpoint, the wait for AWS to take the machine)
is what the local test `test/run.sh node` exercises with a signal.  AWS Fault Injection Service can send a real notice
(`aws:ec2:send-spot-instance-interruptions`), at the cost of an experiment template and a role, which this procedure
does not make.

### 5.6 What was tested, and what can go wrong

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
| `status` shows `ENDED` with `boot failed` | the user data failed (install, clone, checkout, tmux); the reason names the step; fix and `launch --continue` |
| machines keep starting | `run/boots/` counts them; the `MAX_INSTANCES`th ends the run as a boot loop; read `logs/wall-node.log` in S3 |
| the machine is up and `status` shows nothing new for over 20 minutes | `aws ssm start-session`, `tmux attach -t tx`; `tail ~/taxman-work/logs/chain.err` |
| `wall.sh stop` and nothing happens | the machine looks for the stop object every 30 s while the chain runs, and not during the setup; `stop --now` ends it at once |

## 6. The ledger's experiment

Does the ledger (`examples/ledger/`; [README, "Keeping going past
limits"](../frame/README.md#keeping-going-past-limits-and-the-sweep)) prove much of a stretch the chain has proven, for
a small share of what the chain spent there, with the truth hidden from it?  On 2176..3440 (the run to the wall
re-proved it in about 4.4 hours of 64 threads, about 280 core-hours), one `c8g.16xlarge` at spot for an hour of its 64
CPUs: `solver/tools/ledger/cloud.sh 2176-3440` (its comment has the environment), which runs `ledger run` with no proven
game, then `ledger report --known results/strict.jsonl`, which exits with status 1 if a bound is on the wrong side of
the chain's optimum or a game does not replay (a bug: stop and report).  The laptop's dry run of 1301..1500 is [the
entry](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md).  A first launch (2026-10-17) was stopped after
9 minutes with the machine 95% idle, the main thread of the ledger the bottleneck; that is fixed ([the entry's section
on
it](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-the-first-cloud-attempt-stopped-and-the-fix)).
A second (48 core-hours on 64 workers) proved 916 of 1,265 boards, all at the optimum, for about 2.2 times what the
chain paid on the same games, because most searches began before the board below was proven.  The scheduler now climbs
from each board it proves (the board above, and the board below, are searched at once, seeded with its game, for
`CLIMB_LIMIT` seconds, 60 by default; [the entry's section on
it](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-wedge-and-climb)), and a launch
with the climbs needs Brian's approval (the [rules for every launch](#rules-for-every-launch)).

**Cost.**  The build of `setup.sh` (about 10 minutes, or none with the commit's binaries in the build cache, section
15), the run (`CPU_HOURS` 64, an hour of the 64 CPUs, and the searches still going when it is spent, each limited to the
budget left over the workers), the report (a minute): about 1.3 instance-hours, **about $0.9 at spot** ($0.68 an hour),
about $3.3 on demand; the timer of 150 minutes bounds it at about $1.7 at spot.  The 40 GB volume is a few cents.

**Launch** as in section 7.3, with `Purpose=ledger` and this user data (`COMMIT` the pushed master that has the
ledger, `DATE` the day):

```
#!/bin/bash
shutdown -h +150
export AWS_DEFAULT_REGION=us-east-1
dnf install -y -q git tmux
sudo -u ec2-user bash -lc 'cd ~ && git clone -q https://github.com/bvchess/taxman.git && cd taxman &&
    git checkout -q COMMIT'
sudo -u ec2-user tmux new-session -d -s ledger -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/ledger-DATE
     BUILD="--bin taxman --example ledger" REQUIRE_CACHED=1 SKIP_TESTS=1 solver/tools/cloud/setup.sh > ~/setup.log 2>&1 &&
         POWEROFF=1 solver/tools/ledger/cloud.sh 2176-3440;
     sudo shutdown -h now'                          # also if the setup failed
```

The setup is a step of the tmux session now, after the job's `S3_DEST` is exported and before the job, so that a
setup that fails (no tested binaries in the cache with `REQUIRE_CACHED=1`, or, with `RELEASE_TESTS=1` in its place, a
suite that fails: `RELEASE_TESTS_FAILED` in `S3_DEST`) ends the machine and the job never starts.

**Options.**  The environment of `solver/tools/ledger/cloud.sh` has the options (`WORKERS`, `CPU_HOURS`, `BASE`,
`MAX_LIMIT`, `PROVEN`, and for the climbs `CLIMB`, their ways: `up`, `down`, `both` or `off`, unset the program's
`both`; and `CLIMB_LIMIT`, their limit in seconds: unset the program's 60; `CLIMB=off` or `CLIMB_LIMIT=0` runs without
climbs, as the second launch did; and `LNS_SECONDS`, the seconds of one thread the finder gets: unset the program's 1, 0
runs without it).  The climbs are searches of a board next to a proven one, up from a proven board and down from it; the
events and the report say what each way cost and proved.  The finder is large neighbourhood search ([the entry's section
on it](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-the-finder-in-the-ledger)),
which runs on a worker after a board's map and before a search whose best seed has changed; it finds a legal game, not a
proof, and its game is a lower bound once the referee has replayed it.  Its events (`lns_s`, `lns_from`, `lns_found`,
`seed_from`, `below_proven`, `above_proven`, on the maps and the searches) and the report's `finder:` line are what the
run is for: what sets up a run past 3440 best.

**Floor steps** (`FLOOR_STEP`, `FLOOR_SLACK`; [the entry's section on
them](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-18-floor-guided-drilling)).  An open
board that is not climbed to is stepped down before its proof: a search under a floor `FLOOR_STEP` points (unset the
program's 150; 0 turns the steps off and the order back to `level + gap / gap_unit`) below its upper bound, which if it
completes having found nothing makes the floor the upper bound (rule `floor`), and whose cost grows about tenfold a
step; a step's time limit is `FLOOR_SLACK` (unset 4) times its forecast, at least `BASE`, and doubles each time the
limit stops it (the step is kept: its checkpoint is `ckpt/nN.floor.ckpt`, beside the board's `nN.ckpt`).  The boards go,
after the climbs, by the forecast seconds of one thread of their proofs (the line of the board's own steps), cheapest
first, so that a board forecast far beyond the rest waits until the cheaper work is done.  The status line says how many
steps have ended (and how: lowered U, proved their board, found a set, stalled) and how many boards are forecast within
an hour, a day, or longer; the events have each job's `forecast` (log10 of nodes) and `forecast_s`, and
`solver/tools/ledger/events.py` compares the forecasts with the proofs.  **A run that goes on from a ledger written
before the steps** (the stage on `ledger-frontier-2/out/run`, written by afe4513) reads it as it is: the boards get
their root bounds at the start (a fraction of a second a board, on the workers), they have no steps, and the checkpoints
of their ordinary searches stay where they are and are resumed when the board comes to its proof (nothing is dropped); a
board with a wide gap takes its steps first, so a board whose old search was far along is forecast at least twice what
that search spent and waits behind cheaper boards, as `level` made it wait.

**The bounds store.**  The bounds a run finds are kept in `results/bounds.jsonl` ([results/README,
"bounds.jsonl"](../../results/README.md#boundsjsonl)).  `BOUNDS=results/bounds.jsonl` (an option of
`solver/tools/ledger/cloud.sh`, with the file in the checkout the machine cloned) starts the run from the file's best
bounds of its stretch, and a board with both needs no map (the maps of 3441..8191 cost about 65 core-hours); run
`ledger verify-bounds` on the file first.  After the run, on the laptop with the results home (below),
`ledger export solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE/run --bounds results/bounds.jsonl --source NAME --checkpoints s3://BUCKET/ledger-DATE/out/run/ckpt`
appends what improved and prints it; `ledger verify-bounds results/bounds.jsonl` and
`ledger games results/bounds.jsonl > games.jsonl; python3 -I solver/tools/check_bounds.py results/bounds.jsonl --games games.jsonl`
check the file with both referees, and the file is then committed.  The checkpoints are in S3 only for the searches that
are still going: a board proven has none.

**Watch, stop, results home.**  `tail ~/ledger/run.err` (a status line a minute: the CPU spent, the boards proven, the
searches going, the climbs of each way ended, those that proved their board and those that stopped at their limit
(parked), and how long the main thread has been busy and results waited for it; after the maps the CPU spent should rise
by nearly 64 a second, and if it does not, look at those two numbers before anything else) and `~/ledger/log`.
`pkill -TERM -f 'examples/ledger run'` stops the run cleanly (each search writes its checkpoint, the ledger is saved)
and the script goes on to the report; the same command again resumes (on a replacement machine, with `S3_DEST/out`
copied back to `~/ledger` first).  Home:
`aws s3 sync S3_DEST/out solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE/ --exclude 'run/ckpt/*'`,
then on the laptop
`solver/frame/target/debug/examples/ledger report solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE/run --known results/strict.jsonl`
and `solver/tools/ledger/events.py solver/notebook/the-ledger-bounds-from-both-sides-2026-10/cloud-DATE/run`; then the
end of the rules for every launch.

## 7. The sweep

`sweep.sh FIRST LAST` proves as many games of a long stretch as a cheap machine allows, with holes, on one
`c8g.16xlarge` at spot.  Brian chose 3441 to 8191 for it (2026-10-17).  The chain of section 4 proves every game in turn
and stops at the first it cannot (3441 took 5.9 hours on 64 threads).  The sweep instead gives each game at most
`GAME_SECONDS` (180) of one thread, goes on past the games that do not finish, and keeps what does.  [STATUS](STATUS.md)
says whether it has run, and [a local test](../notebook/the-sweep-a-local-test-2026-10/README.md) tried it on the
laptop.  A launch needs Brian's approval with the estimate of section 7.2 ([rules for every
launch](#rules-for-every-launch)).

### 7.1 What it does

The sweep runs in passes over blocks of consecutive games, each block on one thread.  The top of `sweep.sh` documents
the passes, the exit statuses and the variables.

* **Pass 1.**  FIRST..LAST is split into `BLOCKS` blocks (default one a CPU), sized so that their expected work is about
  equal.  A game costs its setup and at most the limit of its search, and the setup grows steeply with n: on one thread
  about 1 s at 2200, 3 s at 3000, 9 s at 4000 and 48 s at 6000 (`solver/tools/sweep_plan.py` interpolates).  Each block
  is one `taxman solve --threads 1 --time-limit $GAME_SECONDS --keep-going` over its range, in increasing n, into a file
  of its own.
* **What a game gets from the one before.**  A game whose search finishes is a proof.  One that reaches the limit leaves
  an unfinished line with its best game, and the next game takes that game as a seed and nothing more: the ceiling n +
  opt(n − 1), the prime formula and a repeat of an earlier search come only from a proven game ([README, "Keeping going
  past limits"](../frame/README.md#keeping-going-past-limits-and-the-sweep)).  A block's first game has no game before
  it and is solved standalone, but for the first block's.  If `KNOWN` (default the committed `results/strict.jsonl`)
  holds a proven game of FIRST − 1, the first block's file starts as a copy of KNOWN's games, and the solver goes on
  from the last, as it extends a chain.
* **Passes 2, 3, ...**  A block's first game began without the game before it, which another block made.  So each pass
  searches again, with the same limit, every game that has only unfinished searches and whose game before it, as the
  merged file holds it now, has more to give than when the game was last tried: it is now proven, or it has a better
  best game than the seed it gave, or it gave none.  A pass runs one `taxman solve --keep-going --retry-improved` a
  block that has such a game, on one thread, in increasing n, so that a game it has just proven helps the next.  Each
  reads an excerpt of the merged file: `sweep_plan.py passes` reads that file once and cuts all the excerpts, each the
  block, the 8 values of n before it that a repeat can repeat, and what their lines rest on, so that the solver's checks
  of its file hold.  Passes repeat until one finds nothing to do, at most `MAX_PASSES` (default 20) of them.  The games
  are then as an unbroken chain would have left them, with what it could prove at this limit.
* **No checkpoints.**  A game that is searched again starts from the start, since a game costs at most `GAME_SECONDS`.

**The files.**  The blocks are merged into `$TAXMAN_WORK/results/explore.jsonl` every `MERGE_EVERY` seconds and at the
end.  The merge drops repeated lines (`awk '!seen[$0]++'`), so it can run any number of times.  Beside it,
`results/explore-blocks/` holds `plan` (the blocks, kept so that a rerun finds them), `block-NNN.jsonl` (each block's
file, which only grows) and `passes` (the last pass made).  `save.sh` copies the directory, so both reach S3.  A pass
works in `$TAXMAN_WORK/sweep-pass/`, which is not saved; a pass that was stopped leaves its lines there, and the next
start merges them first.  Each line says what its game had from n − 1 in `prev`, and which pass made it in
`options.pass` ([REFERENCE.md, "What a game had from n − 1"](REFERENCE.md#what-a-game-had-from-n---1)).  Pass 1 is the
blocks, and a rerun after an interruption counts as another pass.

**`explore.jsonl` is not a chain.**  It has holes, and games built on unfinished ones, so it never goes into
`results/strict.jsonl`.  [`verify.sh`](#44-verifying-and-bringing-the-results-home) accepts it (both referees, and the
agreement of its scores with the committed chain), and `sweep.sh` runs it when the passes are done.

### 7.2 What it costs

**About $3.3 for pass 1.**  By `sweep_plan.py`'s sizes, 3441..8191 at `GAME_SECONDS` 180 is 4,751 games: 76 core-hours
of setup and at most 238 of search, so at most 314 core-hours, 4.9 hours of 64 CPUs.  It splits into 64 blocks of 4.83
to 4.97 core-hours each, the first of 95 games (3441..3535) and the last of 50 (8142..8191).  A `c8g.16xlarge` at spot
cost $0.66 to $0.81 an hour on 2026-10-17, counted at $0.676 as in section 5.3, so pass 1 costs about $3.3.  The later
passes cost a fraction of that, since they search again only the games whose game before has gained.  A deadline of 8
hours (`DEADLINE_HOURS`) bounds the run at about $5.4 at $0.676 an hour ($6.5 at $0.81), and the timer of 9 hours bounds
the machine at $9 at the ceiling of $1.0; the 40 GB volume costs a few cents.  The launch builds nothing: it takes the
solver from the build cache, which must already hold the commit (7.3).  Releasing a commit is a launch of its own, $0.15
to $0.25 (section 11.3).

*Details.*  The estimate is an upper bound for pass 1: a game that a repeat or the prime formula settles costs its setup
alone.  The setup times past 4000 are extrapolated from four measured points.  Nobody has measured what finishes in 180
s past 3441.  Most searches will not, since the cost of a game grows tenfold in about 1,040 in n, so expect most lines
unfinished, with the proofs among the primes, the repeats after a proven game and the easy searches.

### 7.3 Launch, watch, stop, bring the results home

**Launch one instance, not a group,** after Brian's yes.  Its user data runs the sweep by itself: it sets the timer,
installs, clones the pinned commit, sets up from the build cache, restores what S3 holds, and runs the sweep in tmux as
`ec2-user`.  `POWEROFF=always` shuts the machine down when the sweep ends, for any reason, and the timer is the
backstop.  Launch as section 2.2 says, with these values, `ZONE` the zone of the lowest price that day (section 1) and
the ceiling the run to the wall's:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=sweep MAX_PRICE=1.0
```

Pass `--user-data file://sweep-user-data.sh` in place of 2.2's `--user-data` argument: the file sets the timer itself,
so `TIMER_MINUTES` is not needed.  `sweep-user-data.sh` is, with `COMMIT` the id of the pushed master:

```
#!/bin/bash
shutdown -h +540                                    # the timer: DEADLINE_HOURS below, and an hour
dnf install -y git tmux || { shutdown -h now; exit 1; }
sudo -u ec2-user git clone https://github.com/bvchess/taxman.git /home/ec2-user/taxman
sudo -u ec2-user git -C /home/ec2-user/taxman checkout COMMIT
sudo -u ec2-user -i tmux new-session -d -s tx -c /home/ec2-user/taxman \
    'export S3_DEST=s3://taxman-013764204840-us-east-1/sweep-3441-8191
     BUILD="--bin taxman" REQUIRE_CACHED=1 SKIP_TESTS=1 NO_PULL=1 solver/tools/cloud/setup.sh &&
         { solver/tools/cloud/save.sh --restore; DEADLINE_HOURS=8 POWEROFF=always GAME_SECONDS=180 \
               solver/tools/cloud/sweep.sh 3441 8191; }
     sudo shutdown -h now'                          # also if the setup failed
```

* **Setup.**  `setup.sh` takes the solver from the build cache and runs its smoke checks.  With `REQUIRE_CACHED=1` the
  commit must have been released (section 11.3), or the machine ends at once.  Without it, a miss builds the solver with
  `target-cpu=native`, a few minutes on 64 vCPUs, untested.
* **Watch.**  `tail ~/taxman-work/logs/sweep.log` shows a status line every `STATUS_EVERY` seconds: the pass, the blocks
  done, the lines in the merged file.  `~/taxman-work/logs/sweep-*.err` is a block's own log.
* **Stop** cleanly with `kill -TERM $(cat ~/taxman-work/sweep.pid)`: the games in progress have no line, and the others
  are merged and saved.  **Go on** after a stop or an interruption with the same command; on a replacement machine, run
  `save.sh --restore` first, with the same `S3_DEST`.  The plan is in the saved directory, so the sweep finds the same
  blocks.
* **Bring the results home** and check them:

  ```
  aws s3 sync s3://taxman-013764204840-us-east-1/sweep-3441-8191/results/ results-from-cloud/
  solver/tools/cloud/verify.sh results-from-cloud/explore.jsonl
  ```

  `taxman verify-results results-from-cloud/explore.jsonl` reports the completed games, the missing ones, the games that
  only an unfinished search is recorded for, and the proofs.
* **Spend more on the failures** by hand: search them again with a longer limit,
  `taxman solve A-B --keep-going --retry --time-limit 600 --out BLOCKFILE` on a block's file, and run `sweep.sh` again.
  It merges the new lines, and its passes search again the games next to the ones that improved.
* **End** with the checks of the [rules for every launch](#rules-for-every-launch); the bucket's prefix goes once the
  results are home.

*Details.*  The tests of the script are `test/run.sh sweep` and, for the planner,
`python3 -I solver/tools/test_sweep_plan.py`.  `test/run.sh sweep` runs `verify.sh`, so it replays the committed games
three times; set `TAXMAN_BIN` to a non-release build if there is no release build.

## 8. The prover's proofs of opt(n)

`prover.sh A B` proves opt(n) = V(n), the score that `results/strict.jsonl` records, for every n from A to B, on the
whole board.  It runs the prover's `exact` over the range in passes, one game a thread, and it is a small job next to
the chain.  The run of 1301..2175 is the example throughout this section: it took 15.9 hours on one `c8g.4xlarge` at
spot, for $3.84, on 2026-10-14 and 15.  [The prover's README](../noframe/README.md#to-2175-on-a-cloud-machine) has its
results, and [its entry](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md) the
account of the run and the estimate it was planned by.

**A run searches from the roles of Theorem B.**  `exact` starts there by default (`--fix-low 1`), and the search is
still a proof, because every optimal set obeys the roles ([THEORY.md, Theorem
B](../../docs/THEORY.md#the-bottom-layers)).  Each line records `"fix_low":1`, and the check counts the proofs that rest
on the theorem.  Over 2..999 the roles took the plain root's nodes within 5 % at half the time a node ([the prover's
README](../noframe/README.md#3-the-proofs-of-optn)).  The run of 1301..2175 searched from the plain root, so by that
measurement a run from the roles costs about half as much.
`EXTRA_ARGS='--fix-low 0' solver/tools/cloud/prover.sh 1301 2175` reproduces that run.  The two kinds of search keep
separate checkpoints: `ckpt/exact_N_low.ckpt` from the roles, `ckpt/exact_N.ckpt` from the plain root.

### 8.1 How `exact` uses the CPUs

A game's search runs on one thread, so the wall time of a run is at least that of its dearest game, and a big machine
idles at the end.

* **One game a thread.**  `exact` hands the games of the range to its threads one at a time.  The CPUs stay busy while
  games are left, and the node counts do not depend on the number of threads ([the
  entry](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md) measured 12 games at 1
  to 8 threads).
* **Passes.**  A search still running when a pass ends (`--seconds`) pauses: it saves its open subtrees and learned
  conflicts to its checkpoint, and the next pass resumes it on the same tree, so its node count is that of a run with no
  pause.  `exact` writes checkpoints only then, and it has no signal handler, so a kill or a SIGTERM sends the searches
  of the pass in progress back to their last checkpoints.  `prover.sh` therefore runs `exact` in passes of
  `PASS_SECONDS` (default 300) until every game has a line.  An interruption costs at most one pass of the games in
  flight, and a restart about a second.
* **The dearest games first.**  The games go in the order of the nodes that the solver's search of each took in the
  committed chain (`summarize.py order`), so that the long games start at once.  The order is made once and kept in
  `$TAXMAN_WORK/prover-order_A_B.txt`.  A game that repeats the search of a game settled at its ceiling records no
  nodes, so it goes late: two of the 33 such games, 1837 and 1839, were the last of the run of 1301..2175.
* **The tail.**  The last games run alone, and most of the machine idles; the log says so ("N games are left for T
  threads").  With 16 vCPUs the idle CPUs cost cents an hour, so let them finish.  To finish the tail of a bigger
  machine for less, stop the run, start a smaller instance, restore the state there (`save.sh --restore`, section 2.6)
  and run `prover.sh A B` again: `THREADS` may differ between runs.

### 8.2 What it costs

A run costs the thread-hours of its searches, spread over the vCPUs, unless its dearest game takes longer.  The run of
1301..2175, from the plain root, took 226 thread-hours: 490 of its 875 games needed a search, and the others were at
their ceiling or met the prime formula.  Its dearest games, 2170 to 2173, took 7.7 to 7.9 hours each, and the idle tail
was 11 % of the machine's thread-hours.  From the roles, expect about half the thread-hours (above).

For the estimate that Brian approves, price the range from the cost by bands of 100 in n in [the prover's
README](../noframe/README.md#to-2175-on-a-cloud-machine): the thread-hours, halved for the roles, over the vCPUs, with
the idle tail, at the spot price of the hour (section 1).  The bands stop at 2175, the end of the prover's proofs; past
it, [AGENDA, section 2](AGENDA.md#2-the-diagnosis-of-the-wall) carries the trend of the prover's nodes against the
solver's to about 15 times them near 3000, an extrapolation.  The cost of a game follows the root's parity gap, not n,
so the bands are a guide, not a rule, and the first passes of the run check the estimate (8.4, step 4, reads the cost of
a node from the log).

*Details.*  [The entry](../notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md) holds
the estimate the run was planned by (about 140 thread-hours at 2.5 ms a node, and a table of wall time and price by
size), the run against it, and what its model got wrong: the run took 1.5 times the projected nodes and 1.6 times the
thread-hours.  The cost of a node on Graviton4 was 2.0 to 2.6 ms through 1899, then rose to 4.1 ms at 2170 to 2173.  The
[Graviton calibration](../notebook/graviton-calibration-2026-10/README.md) measured the prover's speed: a Graviton4 core
runs it at 0.93 of an M1 Max core, and a Graviton3 core at 0.75.

### 8.3 Which instance

**Use a `c8g.4xlarge` (16 vCPUs) at spot, for about 16 hours**, or on demand if spot has no capacity.  A `c8g.2xlarge`
(8 vCPUs) takes a day for the same price, and nothing larger buys time, since the dearest games set it.  The calibration
settled the family: a Graviton4 core does a unit of work for 12 to 15 % less than a Graviton3 core.  Nobody has measured
`c9g`.  The `c8g.4xlarge` cost $0.24 to $0.31 an hour at spot in four zones of us-east-1 on 2026-10-14.  A gp3 root
volume of 20 GB is enough: the prover's build is under 2 GB, the output about 200 KB and the checkpoints a few MB.  The
[rules for every launch](#rules-for-every-launch) apply, with the instance profile of section 2.1 (S3 for `S3_DEST`, SSM
for the steps).

### 8.4 Run

Launch as section 2.2 says; its values are this job's.  Then send the steps through SSM (section 2.3), the first as root
and the others as `ec2-user`.  With no SSH there is no `rsync`, so the copy of the repository is a pushed branch,
cloned, or a `git archive` tarball in the bucket.

1. **Setup.**  `PROVER_ONLY=1` builds and checks the prover alone; with the commit in the build cache it builds nothing
   (section 11), and `BUILD_PROVER="--bin exact"` builds `exact` alone.

   ```
   dnf install -y git tmux                                       # as root; setup.sh would do it with sudo
   git clone https://github.com/bvchess/taxman.git && cd taxman    # --branch NAME for another branch
   PROVER_ONLY=1 solver/tools/cloud/setup.sh                       # 46 s on a c8g.4xlarge, 10 s of it the build
   ```

2. **A rehearsal on the instance itself** (minutes, cents): a small range, a SIGTERM and a resume, and the check, in a
   work directory of its own, as a step of its own.  It replaces a dry run on a laptop.

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
   CPU, `secs` over `nodes`: 2.44 ms for the searches of these ten games on the `c8g.4xlarge`, from the plain root.  On
   a laptop those searches took about 3 minutes of one thread.

3. **The run,** in tmux, with `spot-watch.sh` beside it:

   ```
   cd ~/taxman && tmux new-session -d -s pv "DEADLINE_HOURS=36 POWEROFF=always \
       S3_DEST=s3://taxman-013764204840-us-east-1/prover-1301-2175 solver/tools/cloud/prover.sh 1301 2175"
   tmux new-session -d -s sw solver/tools/cloud/spot-watch.sh              # on spot: stops the run, saves
   ```

   The prefix `prover-1301-2175` is this run's own, one prefix a run.  The commands of 8.5 and 8.6 on your machine reach
   it as `$D`: `export D=s3://taxman-013764204840-us-east-1/prover-1301-2175`.

4. **The first minutes.**  `prover.sh` prints the machine, makes the order, and logs `pass 1: 0 of 875 games done`.  In
   `~/taxman-work/logs/prover.err`, the lines `exact: n=N not finished: paused after N nodes, U open units` give the
nodes of a long game.  The seconds of a pass over the nodes of one pass give the milliseconds a node: about 2.2 to 3.3
for the games of the first passes on a `c8g.4xlarge` from the plain root, more for the late games (section 8.2).

The guardrails are section 2.5's, with these differences:

* **`S3_DEST` is required with `POWEROFF`.**  A shutdown terminates the instance and deletes its root volume, so
  `prover.sh` copies the results to S3 before it powers off.  Without `S3_DEST`, leave out `POWEROFF` and pull the files
  yourself (8.6) before the timer ends.
* **`POWEROFF=always`** powers the machine off whenever the script ends: the range complete and checked, the deadline,
  or an error.  `POWEROFF=done` powers it off only when the range is complete.
* **`DEADLINE_HOURS`** stops the run after that many hours, and `prover.sh` exits with status 3.  The stop is a
  SIGTERM, so the searches of the pass in progress go back to their last checkpoints (8.1).
* **`SAVE_EVERY`** defaults to `PASS_SECONDS` (300 s).  A save takes about 80 s at the lowest priority (section 2.6).
* **`THREADS`** defaults to every vCPU.

**A range that starts above 1301** needs `BASELINE`, one file of the proofs of 2..A − 1 for the check to build on (a
`cat` of the committed files of `solver/noframe/data/`); its default is the proofs of 2..1300.  `prover.sh --list`, a
measurement on chosen games, is in the script's header.

### 8.5 Watch, stop, resume

From your machine, with no access to the instance, read the saves in S3 (`$D` of 8.4).  They are as old as the last
save, about 6 minutes at most:

```
aws s3 cp $D/logs/prover.log - | tail            # a line a pass: games done, searches paused, the exit status
aws s3 cp $D/logs/prover.err - | tail -n 20      # exact's lines: finished games and paused searches with their nodes
aws s3 cp $D/results/exact_parity_1301_2175.jsonl - | grep -c .      # games done, of 875
aws s3 ls $D/ckpt/                               # a checkpoint per unfinished search, rewritten every pass
```

On the instance, in a shell or as steps through SSM (where root's `~` is not ec2-user's: write `/home/ec2-user/` out):

```
tail -f ~/taxman-work/logs/prover.log            # a line a pass: games done, searches paused, the exit status
tail -n 20 ~/taxman-work/logs/prover.err         # exact's lines: finished games and paused searches with their nodes
grep -c . ~/taxman-work/results/exact_parity_1301_2175.jsonl      # games done, of 875
ls -l ~/taxman-work/ckpt                         # a checkpoint per unfinished search, rewritten every pass
```

* **Stop** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/prover.pid)`; through SSM, which runs as root, the
  pid file is `/home/ec2-user/taxman-work/prover.pid`.  The searches of the pass in progress go back to their last
  checkpoints.  **Resume** with the same command; `THREADS` and `S3_DEST` may differ.
* **A spot interruption:** `spot-watch.sh` stops the run and saves.  On a replacement machine run
  `PROVER_ONLY=1 solver/tools/cloud/setup.sh`, then `S3_DEST=... solver/tools/cloud/save.sh --restore`, then the same
  `prover.sh` command, minding section 2.6's two cautions (the logs and the deadline).  Without a saved copy, the
results and the checkpoints are on the root volume, which a spot termination deletes: use `S3_DEST`, or `rsync` the work
directory home from time to time.
* **An error stop** (`prover.sh` exits with status 1): `exact` stopped with an error, the final check failed, or
`MAX_FAILS` passes in a row crashed without a new line.  Nothing is retried; read the end of `prover.log` and
`prover.err`.  The message "a playable set worth X beats the recorded Y" is a counterexample: a better game
  than the committed chain's for that n, the finding a run exists to look for.  The set is in the message; keep
  `~/taxman-work` as it is, and tell Brian.  The message "the recorded game does not replay to its score" means that the
  results file or the build is wrong.

### 8.6 Bring the data home, verify, commit

```
export AWS_PROFILE=bvc                                                    # after: aws login --profile bvc
aws s3 cp $D/results/exact_parity_1301_2175.jsonl solver/noframe/data/       # $D: as in 8.4
aws s3 sync $D/logs/ ./prover-logs/
# or, over SSH, without S3:
rsync -av ec2-user@HOST:taxman-work/results/exact_parity_1301_2175.jsonl solver/noframe/data/
rsync -av ec2-user@HOST:taxman-work/logs/ ./prover-logs/
OUT=solver/noframe/data/exact_parity_1301_2175.jsonl solver/tools/cloud/prover.sh --verify 1301 2175
```

The check (`summarize.py check 1301 2175`, which `prover.sh` also ran at the end) passes only if two things hold:

* every n in 2..2175 is proven: each n of 1301..2175 has a line, and with `data/exact_parity_2_1300.jsonl` each is
  proven by a bound, a search run to the end, or the ceiling or the prime formula from a proven predecessor;
* every line's score is the score that `results/strict.jsonl` records.

It then exits with status 0 and prints `OK: opt(n) is proven equal to the recorded score for every n in 2..2175`;
anything else prints which n failed.  Then, on your machine, count the proofs and their cost by bands of 100 in n:

```
python3 solver/noframe/tools/summarize.py exact solver/noframe/data/exact_parity_2_1300.jsonl \
    solver/noframe/data/exact_parity_1301_2175.jsonl
python3 solver/noframe/tools/summarize.py cost --band 100 solver/noframe/data/exact_parity_1301_2175.jsonl
```

Commit `solver/noframe/data/exact_parity_1301_2175.jsonl` (152 KB), and never let a later run overwrite it: `OUT` names
another file.  Do not commit the work directory, the checkpoints or the logs.  Record the run:

* the totals of the two commands (`summarize.py exact` gives the searches' thread-hours), and the real milliseconds a
  node by n, in [the prover's README](../noframe/README.md#3-the-proofs-of-optn), which holds the proofs and their cost;
* the new end of the prover's proofs in [results/README.md](../../results/README.md), which holds how far each kind of
  proof reaches; then search the docs for the old end and update what still names it;
* the run itself in a notebook entry, with its rows in the register ([the notebook's
  rules](../notebook/README.md#how-this-notebook-is-kept)), and its cost as section 2.8 says.

Then do the end of the [rules for every launch](#rules-for-every-launch): `POWEROFF=always` has shut the instance down,
which terminates it; confirm that nothing is left, the launch's security group included.

## 9. Certifying U (stage 4)

`certify.sh` runs `taxman certify` on every game of 2176..3440 that rests on the frame model, so that every game to 3440
is proven on the whole board, as the games to 2175 are ([results/README.md](../../results/README.md)).  It is the
campaign of the entry [Certificates past
2175](../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md); the heading's
"stage 4" names the campaign as the notebook's register does (the row "S4 preparation", after E8's stages 1 and 2).
[STATUS](STATUS.md) says where it stands, and a launch waits for Brian's approval with the estimate of 9.1 ([rules for
every launch](#rules-for-every-launch)).  [DESIGN.md, "Certificates of U"](DESIGN.md#certificates-of-u) says what a
certificate is, and [solver/tools/certify/README.md](../tools/certify/README.md) describes the work list, its data and
the lists.  The campaign launches as section 2 says: an instance profile, steps through SSM, `S3_DEST`, a timer,
`spot-watch.sh` beside the run, tmux.  Only the machine, the commands and the checks differ.

### 9.1 What it costs, and what to approve

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
entry](../notebook/certificates-past-2175-the-same-problem-the-work-list-the-verifier-2026-10/README.md), and its rule
settles 3427 and its like from the certificates of phases A and B (3427 alone is 223 thread-hours).  Without it, the 117
games are searched, for 361 thread-hours.

* **If the overhead is not 2.**  The plan prices a certificate at 2 times the plain search; the certificates of eight
  games of 1400..2156 took a median of 1.55 times it ([one-sided candidates in the
  search](../notebook/one-sided-candidates-in-the-search-2026-10/README.md), the second stage).  At 1.6 the campaign
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

### 9.2 The stop rule of the pilot

**Go on if the median R is at most 3 and no R is above 5,** where R is a certificate's cost over the plan's price of its
plain search; the campaign then costs at most 1.5 times the plan.  **Stop** otherwise: report, and build the speedups
(Lemma M5⁺ and a faster node) before spending more.  **Stop at once** on any `beaten` verdict.
`solver/tools/certify_pilot.py` (its header defines the quantities) computes all of it from the certificates and the
plan (9.5).

The seven games of `pilot7` (2340, 3175, 3300, 3335, 3336, 3408, 3420, with the games that rest on them) run one after
the other on all 64 threads, so that each game's time per node is measured cleanly.  For each game that was searched,
with N its nodes in the chain (`plan.jsonl`, `cost_nodes`) and c the cost of a node in its stretch of the chain (243.84
microseconds of a thread up to 3310, 326.73 after; the table of eras in `plan.md`):

* **R** = the certificate's thread-seconds (`seconds_all_runs` x `threads`) / (N x c): the plan priced it at 2.
* **R_nodes** = the certificate's nodes / N, and **R_node** = its thread-microseconds per node / c, so that R = R_nodes
  x R_node splits the overhead into the tree and the node.  The second stage predicts R_nodes about 1.1-1.5 (the rule of
  symmetric pairs is missing) and R_node about 1.2-1.6.

### 9.3 The machine and the build

* **Machine.**  One `c8g.16xlarge` (64 vCPUs, Graviton4) at spot in us-east-1, in the zone with the lowest price of the
  spot price history (section 1, "Quota, prices and spot interruptions": read it first, since it is the price of the
  estimate), or on demand if spot has no capacity.  A gp3 root volume of 40 GB, `Purpose=certify`, and the rest as
  section 2.2 launches it.  Memory is no concern: a few hundred MB a process, the results file loaded in each.
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

### 9.4 The lists, and the commands of the phases

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

### 9.5 Phase A: the pilot

Launch as section 2.2 says, after Brian's yes, with these values:

```
INSTANCE_TYPE=c8g.16xlarge ZONE=... VOLUME_GB=40 PURPOSE=certify TIMER_MINUTES=480 MAX_PRICE=1.2
```

`ZONE` is the zone of the lowest price that day (9.3), and the ceiling of $1.2 an hour is above any price seen.  Then
send the steps through SSM as in section 2.3, the first as root and the others as `ec2-user`, with `COMMIT` the
campaign's:

```
dnf install -y git tmux                                                     # as root
git clone https://github.com/bvchess/taxman.git && cd taxman && git checkout COMMIT
NO_PULL=1 SKIP_TESTS=1 solver/tools/cloud/setup.sh                            # builds for this CPU, smoke checks
```

and the check of the lists (9.4).

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
Brian with the table and ask for phases B and C; on STOP, report and build the speedups; on COUNTEREXAMPLE, see 9.8.

### 9.6 Phase B

Phase B runs on a new machine, since the pilot's is gone.  Launch and set it up as in 9.5 with `TIMER_MINUTES=3900`,
with no rehearsal:

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

### 9.7 Phase C

Phase C searches the repeats that the rule cannot settle, or settles them by the rule of a build with Lemma M5⁺ (9.3).
Its output file starts as a copy of everything the earlier phases wrote (with two machines in phase B, sync
`$D/B1/results` and `$D/B2/results` in place of `$D/B/results` below), so that the rule finds the certificates of the
games it rests on, and so that the file at the end is the whole campaign's.  On a machine set up as in 9.5, on the build
that phase C is to run on, with `TIMER_MINUTES=600`:

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

### 9.8 Watch, stop, resume, and a counterexample

```
tail -n 5 ~/taxman-work/logs/certify_pilot7.log        # this script: a status line every 10 minutes, the exits of the processes
tail -n 3 ~/taxman-work/logs/certify_dear_rest.out     # the result lines of the games finished (nodes, seconds, verdict)
tail -n 3 ~/taxman-work/logs/certify_dear_rest.err     # the progress lines of the game in progress (nodes, eff, best)
ls -l ~/taxman-work/ckpt/*                              # a checkpoint no older than 5 minutes while a long game runs
```

The log of a phase is `certify_PHASE.log`, PHASE being the list's name, or for several lists the first list's with `x`
and the number of lists (`certify_rest_1x4.log`); each list has its `certify_LIST.out` and `.err`.  The saves in S3, as
old as the last save (about 10 minutes), hold the same files under `$D/X/logs/` and the certificates under
`$D/X/results/`, X being the phase's letter (A, B or C) of 9.4.

* **Stop on purpose** with Ctrl-C in tmux, or `kill -TERM $(cat ~/taxman-work/certify_PHASE.pid)`.  Every process writes
  a final checkpoint, and the script ends with status 143.  **Resume** with the same command: it skips every game with a
  verdict and resumes the search with a checkpoint.  The thread counts and `S3_DEST` may differ; the build may not.
* **A spot interruption:** `spot-watch.sh` stops the run and saves.  On a replacement machine, set up as in 9.5 with the
  same `COMMIT`, run `S3_DEST=$D/X solver/tools/cloud/save.sh --restore` and copy the logs back
  (`aws s3 sync $D/X/logs ~/taxman-work/logs`), or the new machine's first save overwrites the old ones.  Then run the
  same command with `DEADLINE_HOURS` set to the hours that remain, since the deadline is not saved (section 2.6).
* **An error stop** (status 1): a game failed a check (a recorded game that does not replay, a certificate that
  contradicts another, a checkpoint of another build).  Nothing is retried; read `certify_LIST.err`.
* **A counterexample** (status 2, `COUNTEREXAMPLE` in the log): a game scores more than its recorded score.  The
  processes stopped with their checkpoints, and the script saved to S3.  The game is in
  `certificates_LIST.counterexample.jsonl` and in the last lines of `certify_LIST.out`.  Keep `~/taxman-work` as it is,
  pull what is in S3, terminate the machine, and tell Brian: the recorded score of that game is too low, and the chain
  is wrong from there.  Do not go on.

### 9.9 Results home, verification, and the spending

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
C was left for later, the file of the earlier phases is the `cat` of 9.7, the proven end of the chain is the first of
the 117 games, and the same commands finish it.  Commit `results/certificates_2176_3440.jsonl` (under 1 MB), and never
let a rerun overwrite it: a rerun writes another file.  Then record the campaign:

* the new end of the proofs on the whole board in [results/README.md](../../results/README.md), which holds how far each
  kind of proof reaches; then search the docs for the old end and update what still names it;
* a notebook entry with the times against the estimate and the overhead by |O| and gap (the report above, `--bands`),
  and its rows in the register ([the notebook's rules](../notebook/README.md#how-this-notebook-is-kept));
* the spending, as section 2.8 says, with the thread-hours of the search against the plan's, and what they come to on
  one machine, which `certify_pilot.py` prints.

## 10. When something goes wrong

Most failures leave a message in the logs of `~/taxman-work/logs/`; the table gives the cause and the remedy of each.  A
job's own failures are in its section: the prover's error stop in section 8.5, the run to the wall's in section 5.6, and
a counterexample to U in section 9.8.

| what you see | cause | remedy |
|---|---|---|
| `DIR is in use by another process (process N): DIR/... is locked` | another solver uses that checkpoint directory | find it with `pgrep -a taxman`; a lock is released when its process ends, however it ends |
| `FILE is in use by another process (another run appending to it?): it is locked` | another solver appends to that results file (`--out`) | the same |
| `another chain run is going (it holds ~/taxman-work/chain.lock)` | a second `chain.sh`; `prover.sh` says `another prover run` | `~/taxman-work/chain.pid` (or `prover.pid`) names the first |
| `FILE was made with different search tactics: ...; resume it with FLAGS, or delete it to start the game over` | the run's `--tactic` flags differ from the checkpoint's | resume with the flags it prints (`EXTRA_ARGS`), or delete the checkpoint to start that game over |
| `cannot resume from FILE: ...` for another game or build | the checkpoint is not this game's or this build's | delete the named `.ckpt` file; the game starts from its beginning |
| the solver exits with status 137 | a SIGKILL: the OOM killer, or a shutdown | `chain.sh` retries a crash; if it repeats, read `dmesg` (memory use is small: section 1) |
| `rustc ... is older than 1.89 and there is no rustup to update it`, or an error about `try_lock` | the engine needs `File::try_lock`, stable from Rust 1.89 | install rustup, with which `setup.sh` updates stable itself, or a Rust of 1.89 or later; by hand, `rustup update stable` |
| `DIR is on a memory filesystem: it would vanish with the instance (set TAXMAN_WORK)` | `/tmp` or `$TAXMAN_WORK` is tmpfs | set `TAXMAN_WORK` to a directory on EBS (section 1, "Storage") |
| `installing packages needs root or sudo (or SKIP_PACKAGES=1 if git, cc, python3 and curl are there)` | `setup.sh` runs as a user without sudo | run it as a user with sudo, or install those four and set `SKIP_PACKAGES=1` |
| a step sent with `aws ssm send-command` runs as root, or its quoting is mangled | `send-command` runs as root, and `sudo -i` mangles a multi-line `bash -lc '...'` | put the step in a script file and run `sudo -u ec2-user -i bash FILE` (section 2.3) |
| the SSH connection dropped | the run is in tmux | reconnect and `tmux attach -t NAME` (`tx` for the chain and the sweep, `pv` for the prover, `cert` for the certificates); if tmux is gone, the machine rebooted (next row) |
| the machine rebooted | tmux and the job are gone; the state is in `~/taxman-work`, and the build survives | run the job's command again (`chain.sh N`) |
| `eff` low and `idle` high on a short game at 192 threads | expected (section 3.5) | judge the long games |
| `lock` of several percent on long games | the shared deque is contended | report it; fewer threads is the remedy at hand (`THREADS=96`) |

## 11. The build cache and the release suite

A machine that needs the solver does not build it when a machine has already built and tested it: `setup.sh` takes the
release binaries of the commit from S3.  The build is the dearest part of a start (about 10 minutes, with fat LTO and
one codegen unit, which stay: the speed of the programs matters more than build minutes), and a run that is replaced
after an interruption pays it again on every machine.  Nothing untested is stored, so a hit starts the job at once.
Run on EC2 on 2026-10-18: the release of ca357bf for the run to 5000 (the solver's suite, 699 tests on 64 jobs in 311 s)
and of 260e74f with the study tools for the A1 measurement, each tested before it was stored.

### 11.1 The key and the layout

```
s3://taxman-013764204840-us-east-1/builds/COMMIT-ARCH-CPU/SET/binaries.tar.gz
                                                          /manifest
s3://taxman-013764204840-us-east-1/builds/test-times.json
```

* **COMMIT** is `git rev-parse HEAD` of a clean checkout: no uncommitted change and no file git does not know of (build
  outputs are ignored files and do not count).  **ARCH** is `uname -m`.  **CPU** is the CPU the build targets, which for
  `-C target-cpu=native` (the default of `env.sh`) is the name the compiler reports (`rustc --print target-cpus`:
  `neoverse-v2` on Graviton4), because that flag makes a binary for the CPU that builds it; no flag is `generic`.
  `RUSTFLAGS` other than empty or one `-C target-cpu=...` is not in the key, so it bypasses the cache.  A dirty
  checkout, no commit (a copy without `.git`), no `aws` command or no credentials, or `BUILD_CACHE=off` is a miss that
  says why, and the build goes on as before.  The cache is never read or written then.
* **SET** is what the tarball holds, named by its parts: `solver-bin-taxman+solver-example-ledger`,
  `prover-bins+prover-examples+solver-bins+solver-examples` (everything: the default of `setup.sh`).  An entry serves a
  request it covers: `--bin X` by `--bin X` or `--bins`, `--example X` by `--example X` or `--examples`.  So the entry
  of the release, which is everything, serves any `BUILD`; an entry of a few binaries serves only those.  An entry is
  never completed: a request that no entry covers builds what it needs, and nothing is stored.  The study tools (cargo's
  feature `studies`: `tune`, `extended`, `structure` and `wall`, which `--examples` alone does not build) are a part of
  the set when `STUDIES=1` asks for them: `solver-bins+solver-examples+solver-features-studies`, covered only by an
  entry that has the part.  So a default build never claims to hold `wall`, and an entry with the studies serves a
  request without them (the smaller entry first, when both exist).
* **binaries.tar.gz** holds the executables only (paths relative to the checkout: `solver/frame/target/release/taxman`,
  `.../examples/ledger`, `solver/noframe/target/release/exact`), not the objects, libraries or cargo's hash-named
  copies.  **manifest** (`key=value` lines, uploaded last: an entry without one does not exist) has the commit,
  architecture, CPU, `RUSTFLAGS`, `rustc`, host and time of the build, the **sha256** of the tarball, the parts
  (`set=`), and the suite's result: `tests=passed`, `tests_solver=608 passed, 0 failed, 138 ignored`, its seconds and
  the tests at once.  Only an entry with `tests=passed` is used.  `test-times.json` is how long each test took last
  time, for ordering (11.2).
* A download is **checked**: the tarball's sha256 against the manifest's before anything is unpacked, and every member
  of the tarball must be a file under a crate's `target/release`.  A mismatch is a miss (the build goes on) and says so.
* `aws s3 ls builds/` lists the commits; `aws s3 cp .../manifest -` says what an entry is.  Old entries are deleted by
  hand (a deletion in the bucket is Brian's yes first, like every change there); they cost cents.

### 11.2 How a launch uses it, and what it builds

`setup.sh` looks for an entry that covers the set it was asked for (`BUILD`) and, finding one, downloads, checks and
unpacks it, builds nothing, and goes on to the smoke checks (they show that this machine runs the binaries).  Finding
none, it builds the set and does not store it.  `REQUIRE_CACHED=1` turns that miss into an error, for a launch that must
run only tested binaries: the user data of 5.4, 7.3 and 6 use it, so that a commit that has not been released ends the
machine at once and costs minutes, not a run of untested binaries.

**Build only what a run needs**, with `BUILD`, cargo's arguments for `solver/frame/` (`--bins`, `--examples`,
`--bin NAME`, `--example NAME`; default `--bins --examples`, as before).  A `BUILD` leaves the prover out, unless
`BUILD_PROVER` asks for it (the same for `solver/noframe/`; `none` for none; `PROVER_ONLY=1` is the prover alone, as
before); the smoke checks of a part that is not built are left out.  `STUDIES=1` adds `--features studies` to the
solver's build (and to its tests: the release suite, `cargo test`), for the study tools, which a build without it skips.
What the jobs need:

| job | `BUILD` |
|---|---|
| the chain, the wall run (`wall-node.sh` sets it), the sweep, `verify.sh` | `--bin taxman` |
| the ledger (14) | `--bin taxman --example ledger` |
| the node-choice measurement | `STUDIES=1`, `--bin taxman --example wall --example perf` |
| the scaling sweep (`scaling.sh`) | `--bin taxman --example ladder` |
| the prover's proofs (9) | `PROVER_ONLY=1` (`BUILD_PROVER="--bin exact"` for `exact` alone) |

`taxman` is there for the smoke checks.  A machine of 32 CPUs or more builds the solver and the prover at the same time
(each has fewer binaries to link than the machine has CPUs, and every link is a single thread); a smaller one builds
them in turn.  Nothing passes `-j` to cargo, so every build and every test build uses every CPU.

### 11.3 The release: the suite, then the cache

The release is `RELEASE_TESTS=1 solver/tools/cloud/setup.sh` on **one machine, once for each commit and CPU**, with the
default `BUILD` (everything, so that every smaller request hits it).  It is a launch under the rules at the top: a
`c8g.16xlarge` on spot for about 10 to 20 minutes, **about $0.15 to $0.25**, Brian's yes first.  Any size serves (the
key is the CPU, which every `c8g` shares); the suite gains nothing past about 64 vCPUs.  The study tools are the one
thing that "everything" leaves out: `STUDIES=1` makes a release that also serves the node-choice measurement (its entry
holds `wall`, and the suite runs the study tools' tests too; it serves every smaller request as well).

```
export S3_DEST=s3://taxman-013764204840-us-east-1/release-COMMIT       # where a failure is reported
RELEASE_TESTS=1 NO_PULL=1 solver/tools/cloud/setup.sh                     # in the clone checked out at COMMIT
```

In order: a hit ends it ("the binaries come from the build cache": nothing to run).  On a miss it builds everything,
runs the smoke checks, then the **whole release suite of every crate built**, then stores.  If the suite fails,
**nothing is stored**, `setup.sh` exits 1 (so a user data that chains the job with `&&` does not start it), and the
failing tests are in `~/taxman-work/logs/release-tests.txt` (also saved with the logs) and, when `S3_DEST` is set, in
`$S3_DEST/RELEASE_TESTS_FAILED`.  With the cache unusable (a dirty checkout, `BUILD_CACHE=off`) the suite still runs and
nothing is stored.  `RELEASE_TESTS=1` in a job's own environment is the same on the job's first machine: it releases
what it needs before starting; `TEST_JOBS` and `TEST_TIMEOUT` (3600 s a test) adjust the suite.

**How the suite runs** (`run_tests.py`; `cargo test` runs the test binaries one after another, and a long test in one
leaves the other CPUs idle).  It builds the test executables of both crates at once
(`cargo test --release --no-run --message-format=json`, which names them all), lists each one's tests (`--list`, leaving
out the `#[ignore]`d ones that `cargo test` also leaves out, `--list --ignored`), and runs **every test as a process of
its own** (`--exact NAME`), as many at once as there are CPUs, over all the binaries together: the tests with no time on
record first, then the longest first (`test-times.json`, rewritten after each run, kept in the cache), so that the
longest does not start last.  The doc tests run as `cargo test --doc`.  A failing test does not stop the others (as
`--no-fail-fast`); each is run once more alone afterwards, to say in the report whether it fails only in company.  A
test that outlasts `TEST_TIMEOUT` is killed and fails.  The summary (`--summary`) names the ten tests that used the most
CPU, with their seconds of CPU (a test's own, which do not grow when the machine is busy); `solver/tools/quick_check.py`
runs the suite without `--release` and judges it by them (DESIGN.md, section 9).  Every test in its own process is
stricter than threads of one process (no shared statics, no leftover environment) and costs each test its own start; the
suite passes that way (below).  Tests that start a thread per CPU themselves share the machine with all the others, so a
test that depends on its speed shows it: lower `TEST_JOBS` to see whether that is it.

**Time**, by estimate for 64 vCPUs (measured on the laptop, below): the tests themselves about 30 to 60 seconds, the
build of the test executables 3 to 5 minutes (the release profile, one codegen unit each: the library first, then the 35
test executables side by side), so **4 to 6 minutes in all, against 13.5 minutes on the laptop**, where the test
binaries run one after another.  The build is most of it.  **Measured** on the laptop, with the test profile
(`cargo test` without `--release`), 5 tests at once, niced: the solver's 608 tests (138 ignored) and the prover's 41
passed in 73 s, 276 CPU seconds in all, the longest test 24 s (so 64 CPUs cannot take less than that); a second run
orders by the first's times.

### 11.4 What was tested, and what was not

On the laptop (macOS, bash 3.2 and 5): `python3 -I test/test_build_cache.py` (the key, the dirty and the commit-less
refusals, other `RUSTFLAGS`, which entry serves which request, a corrupted tarball, an entry without its manifest or its
passed suite, a build that lacks a file, a stranger in the tarball, a build with the studies as an entry of its own, the
same through the stand-in for aws), `python3 -I test/test_run_tests.py` (a toy crate: a failure, an ignored test, a doc
test, a timeout, the order, a feature, a build that fails), `python3 -I test/test_setup.py` (`setup.sh` with a stand-in
for cargo: what `BUILD` and `STUDIES` build, a hit and a miss, the release suite on a pass and on a failure, the dirty
checkout, `REQUIRE_CACHED`, the two builds at once), and
`test/run.sh node launch` (the settings reach `wall-node.sh` and `setup.sh`).  The runner ran the whole suite of both
crates in the test profile; no release build was made on the laptop.  **Not run on EC2**: the S3 commands against S3
(the listing's format, credentials from the instance profile, the transfer times), the release profile's build of the
tests, and the suite at 64 processes at once.  The first release is its test: read its log for the build and suite
times and the report for any test that fails only in company.

## 12. The scripts, and what has run where

### The scripts

The files in this directory:

| file | what it does |
|---|---|
| `env.sh` | settings and helpers the scripts share (paths, defaults, `env_report`); sourced, not run |
| `run.sh` | what the long runs share (one at a time, log, pid, deadline, S3 saves, clean stop, power-off); sourced |
| `setup.sh` | packages, Rust, the checkout, the binaries (from the build cache, or built with `target-cpu=native`: `BUILD` says what; section 11), smoke checks and the tests; `RELEASE_TESTS=1` runs the release suite and stores the binaries; idempotent |
| `build_cache.py`, `run_tests.py` | what `setup.sh` calls (section 11): the cache of tested binaries in S3, keyed by commit, architecture and CPU; the release suite with every test a process of its own on every CPU |
| `scaling.sh` | the scaling sweep (`--engine`: the engine benchmark; `--pauses`: the cost of checkpoints; `--quick`) |
| `chain.sh N` | the production run: the chain 1..N in a loop, a window of games in flight (`WINDOW`), resumable, logged, optional deadline and power-off |
| `verify.sh [FILE...]` | both referees on a results file, and against the committed results |
| `sweep.sh FIRST LAST` | the sweep (section 7): FIRST..LAST in blocks of consecutive games, one thread each and a limited search a game, then passes that search again the games whose game before them has gained, into `explore.jsonl`; resumable, with `DEADLINE_HOURS`, `POWEROFF` and `S3_DEST` as the others; `../sweep_plan.py` sizes the blocks and cuts the excerpts of the passes |
| `save.sh` | copy results, logs and checkpoints to S3 (`--restore` on a fresh machine) |
| `prover.sh A B` | the prover's proofs of opt(n) for n = A..B (section 8): resumable passes, all CPUs; `--verify` checks the output; `--list N1,N2,...` runs `exact` on given games as a measurement ([E6](../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md#101-e6-does-leaving-the-sources-free-cost-the-provers-search-anything)) |
| `certify.sh LIST[:THREADS]...` | the certificates campaign (section 9): `taxman certify` over the lists of `solver/tools/certify/lists/stage4`, several at once, resumable, with a final `verify-certificates` |
| `spot-watch.sh` | on a spot interruption notice: stop `chain.sh`, `prover.sh` or `certify.sh` and save |
| `wall.sh launch\|status\|stop\|cleanup` | the unattended run to the wall (section 5): the launch template, the Auto Scaling group of one spot instance and the policy, from your machine; prints every aws command that changes something |
| `wall-node.sh`, `wall-policy.json` | what each machine of that group does from boot to the end of the run; the one permission the machines need |
| `test/run.sh` | tests of `chain.sh`'s wall and ledger, `wall-node.sh`, `wall.sh` and `sweep.sh` against a stand-in for aws (no cloud) |
| `test/test_build_cache.py`, `test/test_run_tests.py`, `test/test_setup.py` | tests of the build cache (a directory for the bucket, and the stand-in for aws), of the release suite's runner (a toy crate) and of `setup.sh`'s build logic (a stand-in for cargo); no cloud, no release build |
| `Dockerfile`, `Dockerfile.dockerignore` | a Linux build-and-test image (section 3.1) |

All state lives in one directory, `$TAXMAN_WORK` (default `~/taxman-work`): `results/strict.jsonl` (and
`exact_parity_A_B.jsonl` of `prover.sh`), `ckpt/` (the checkpoints, `taxman.lock`), `logs/`, `chain.pid`
(`prover.pid`), `deadline`.  Nothing is written inside the checkout except
`solver/frame/target` (and `solver/frame/bench/history.jsonl` if you run `taxman bench` without `--history`).

### What has run where

The scripts were exercised on macOS (arm64): `chain.sh` through a complete chain, a SIGTERM stop, deliberate
interruptions and resumes, a crash loop and a deadline; `scaling.sh` in all three modes (a sweep, `--engine`,
`--pauses`); `setup.sh` through its build and checks; `verify.sh`.  `prover.sh` (section 8) ran on macOS and in the
Docker image (Linux, arm64) over a range, with a SIGTERM stop and a resume; `chain.sh` also ran in the image after the
`run.sh` split.  On EC2 (Amazon Linux 2023, Graviton) the calibration
([solver/notebook/graviton-calibration-2026-10/README.md](../notebook/graviton-calibration-2026-10/README.md)) ran the
solver and the prover on a `c7g.16xlarge` and a `c8g.16xlarge` with its own scripts, and the prover's run of section 8
(2026-10-14 and 15, to the end) used `setup.sh` (`PROVER_ONLY=1`), `prover.sh`, `save.sh` (with an instance role) and
`spot-watch.sh`, all of which have run there; no spot interruption has yet tested `spot-watch.sh`, and the run's 15.9
hours had none.  The chain's cloud run of 2026-10-15 and 16 (games 2176 to 3440 on a `c8g.16xlarge`, 24 hours, in two
instances; section 4, and the entry on [the cloud run to
3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md)) used `setup.sh`, `chain.sh` (after a rehearsal of
section 3.7 on the same machine type, which ran `verify.sh`), `save.sh` (with its restore on the second instance) and
`spot-watch.sh`; no spot interruption came in it either, the first instance having been stopped on purpose.  The scaling
run on a `c8g.48xlarge` ran `setup.sh` in full (without the tests) and `verify.sh` there.  `certify.sh` (section 9) ran
on macOS (arm64, bash 3.2) with `taxman certify` on two lists at once through a complete run and its
`verify-certificates`, a SIGTERM stop and its resume, and, with a stand-in for the binary, a counterexample, an error, a
crash and a deadline; `run.sh` changed for it (`run_start` and `run_reap`), and `chain.sh` ran through the change to a
complete chain, a SIGTERM stop and a deadline.  Not run on EC2: `certify.sh`, `scaling.sh` (the calibration and the
scaling run did by hand what it does), and nothing has run on Ubuntu.  `wall.sh`, `wall-node.sh` and the ledger of
`run.sh` (section 5) ran on macOS against a stand-in for aws (`test/run.sh`) and on EC2, in the rehearsal of section 5.5
and in [the run to the wall](../notebook/the-run-to-the-wall-2026-10/README.md).  The build cache and the release suite
(section 11) ran on macOS against a directory for the bucket, the stand-in for aws and a stand-in for cargo, and the
suite's runner ran the whole solver and prover suites in the test profile; both ran on EC2 on 2026-10-18 (section 11).
Sections 3.1 and 3.2 exist to find the problems that a macOS run cannot, on a machine that costs cents.  The speed of a
Graviton core and the scaling of the solver to 64 and 192 threads are measured (the calibration and the [scaling
run](../notebook/scaling-to-192-threads-2026-10/README.md)); what is not is in [the research agenda, section
6](AGENDA.md#6-what-the-cloud-runs-should-measure).
