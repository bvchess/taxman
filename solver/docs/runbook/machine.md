# Choosing the machine

Part of the [runbook](../RUNBOOK.md).

Every launch decides four things first: the instance, spot or on demand, the disk, and whether the quota allows it.
Each job's file names the instance that suits it; this page says why.

## Instance

* **The workload is CPU-bound.**  It is integer and bit-set code, with no floating point and no SIMD requirement, in one
  process with as many threads as CPUs.  Memory is small: the chain needs 400 to 540 MB with the results file loaded,
  about 1 GB at 192 threads and 2.3 GB at n = 4000, and a proof needs 370 MB at most on 64 threads ([the
  calibration](../../notebook/graviton-calibration-2026-10/README.md), [the scaling
  run](../../notebook/scaling-to-192-threads-2026-10/README.md)).  Each thread also reserves 64 MiB of address space for
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
  sweep ([A small instance first](before-a-big-run.md#a-small-instance-first)).
* **Check `lscpu` on a large machine** for the number of NUMA nodes.  The engine does not pin threads: each searcher
  allocates its own state on its own thread, so first touch keeps it local.  If there are several nodes and the sweep
  shows a step in `k/s/thr` where the thread count crosses a node, that is the cause.  The 64-vCPU instances of the
  calibration were one socket and one node (`solver/notebook/graviton-calibration-2026-10/machines/`).
* **Build on the machine type that runs the binary.**  A plain `cargo build` makes generic code; `-C target-cpu=native`
  is opt-in ([README, section 2](../../frame/README.md#2-build-test-run)).  These scripts add it through `RUSTFLAGS`,
  the default of `env.sh`; set `RUSTFLAGS=` to build generic code.  `native` tunes for the CPU that builds, so a binary
  built on a newer CPU can fail with an illegal instruction on an older one.  The build cache
  ([build-cache.md](build-cache.md)) therefore keys its binaries by the CPU that `native` resolved to, and a machine
  takes only binaries built for its own CPU.  `setup.sh` prints that CPU in its `native cpu:` line, which should name a
  Neoverse CPU (`neoverse-v2` on Graviton4), not `generic`.  That line is the only place the logs name the CPU:
  `env_report` greps `lscpu` for `^Model name`, and on Graviton4 with Amazon Linux 2023 `lscpu` prints only
  `BIOS Model name: AWS Graviton4`.  Nobody has measured the gain of `native` on Graviton; expect 0 to 3 % (on the M1 it
  measured 0 %).
* **Profile-guided optimization** (PGO) is optional.  `solver/tools/pgo.sh` makes the build (its header says how); on
  the M1 it adds 0 to 2 % ([design/performance.md](../design/performance.md)).  PGO changes code generation only, so the
  search
  and its answers stay the same, and either binary can resume a checkpoint.  `TAXMAN_BIN` and `LADDER_BIN` point the
  scripts at another binary.  Keep a non-PGO build too if you want to profile.

## Spot or on demand

* **Use spot for the checks of [before-a-big-run.md](before-a-big-run.md) too,** as one-time requests.  An interruption
  in the middle of a measurement ruins it, and you repeat it: AWS took back the calibration's `c9g.16xlarge` five
  minutes in, at a cost of $0.06, and on demand costs 3 to 4 times as much a vCPU-hour (below).  Use on demand for a
  measurement you cannot afford to lose, or when the spot pool has no capacity.
* **Use spot for the production chain.**  The run suits it: a stop by SIGTERM loses nothing, a kill loses at most one
  checkpoint interval (300 s), and `chain.sh` is safe to start again.  An interruption really costs the time to get a
  machine again (the spot pool of a large size can be thin) and your attention.  If you cannot get one, fall back to on
  demand or to a smaller size (`THREADS` can differ between resumes).  Run `spot-watch.sh`, which turns the two-minute
  notice into a clean stop and a save.
* **A persistent spot request** launches a replacement after an interruption.  That is convenient, and it is also a way
  to keep paying for something nobody watches.  If you use one, `DEADLINE_HOURS` and `POWEROFF` ([Cost
  guardrails](launch.md#cost-guardrails)) protect you, and you cancel the request at the end like everything else
  ([rules for every launch](launch.md#rules-for-every-launch)).
* **Name a price ceiling.**  `aws ec2 run-instances` needs `SpotOptions` with a `MaxPrice` in its market options
  (`MarketType=spot,SpotOptions={MaxPrice=PRICE}`, PRICE in dollars an hour); `MarketType=spot` alone does not work.
  The instance pays the spot price of the hour, never the ceiling, which only bounds the price at which AWS takes it
  back.  Brian approves a cost estimate at the spot price, and the ceiling goes above it.  [Launch](launch.md#launch)
  shows where it goes.

## Storage

Put checkpoints and results on a disk that survives a stop or a reboot and has local semantics (`flock`, `fsync`).  Use
the root EBS volume (gp3; 40 GB is plenty, since the toolchain and `target/` take 3 to 4 GB and the results 30 MB for
the 4,547 games of the committed chain on 2026-10-20), or a separate EBS volume that you mount and name with
`TAXMAN_WORK`.  Turn "delete on termination" off for a
separate volume if the state should outlive the instance; the end of the run deletes it with the rest.  Avoid three
things:

* **tmpfs.**  `/tmp` is tmpfs on Amazon Linux 2023, and `chain.sh` refuses a memory filesystem.
* **Instance-store NVMe as the only copy.**  A stop or a termination loses it.
* **A network filesystem.**  It lacks local `flock`, and each checkpoint does an `fsync`.

## Quota, prices and spot interruptions

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
  the rate is uniform, and an interruption costs a few minutes and, for the prover, one pass ([How `exact` uses the
  CPUs](prover.md#how-exact-uses-the-cpus)).  So spot is fine; for the prover's proofs on 16 vCPUs, on demand costs
  about $4 to $7 more.
