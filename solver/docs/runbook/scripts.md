# The scripts, and what has run where

Part of the [runbook](../RUNBOOK.md).

## The scripts

The scripts of `solver/tools/cloud/`; the ledger's are in `solver/tools/ledger/` ([ledger.md](ledger.md)).  The top of
each says how to run it, and the long runs' scripts (`run.sh`, `chain.sh`, `prover.sh`, `certify.sh`) also give their
variables, and `chain.sh` and `certify.sh` their exit statuses.

| file | what it does |
|---|---|
| `env.sh` | settings and helpers the scripts share (paths, defaults, `env_report`); sourced, not run |
| `layout.sh` | the folders of a machine that runs the chain beside other work ([chain.md](chain.md#the-machines-folders)), which `status.sh` reads; sourced, not run |
| `run.sh` | what the long runs share (one at a time, log, pid, deadline, S3 saves, clean stop, power-off); sourced |
| `setup.sh` | packages, Rust, the checkout, the binaries (from the build cache, or built with `target-cpu=native`: `BUILD` says what; [build-cache.md](build-cache.md)), smoke checks and the tests; `RELEASE_TESTS=1` runs the release suite and stores the binaries; idempotent |
| `build_cache.py`, `run_tests.py` | what `setup.sh` calls ([build-cache.md](build-cache.md)): the cache of tested binaries in S3, keyed by commit, architecture and CPU; the release suite with every test a process of its own on every CPU |
| `chain.sh N` | the production run: the chain 1..N in a loop, a window of games in flight (`WINDOW`), resumable, logged, optional deadline and power-off |
| `verify.sh [FILE...]` | both referees on a results file, and against the committed results |
| `status.sh [--recorded N] --host USER@HOST` | the games recorded and in flight of a chain on another machine, read over ssh, with no AWS ([chain.md](chain.md#from-another-machine)) |
| `save.sh` | copy results, logs and checkpoints to S3 (`--restore` on a fresh machine) |
| `prover.sh A B` | the prover's proofs of opt(n) for n = A..B ([prover.md](prover.md)): resumable passes, all CPUs; `--verify` checks the output; `--list N1,N2,...` runs `exact` on given games as a measurement ([study E6](../../notebook/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md#the-plan-of-e6)) |
| `certify.sh LIST[:THREADS]...` | the certificates campaign ([certify.md](certify.md)): `taxman certify` over the lists of `solver/tools/certify/lists/stage4`, several at once, resumable, with a final `verify-certificates` |
| `spot-watch.sh` | on a spot interruption notice: stop `chain.sh`, `prover.sh` or `certify.sh` and save |
| `test/run.sh` | tests of `chain.sh`'s date and ledger, the lock of `run.sh` and `status.sh`, against stand-ins for aws and ssh (no cloud) |
| `test/test_build_cache.py`, `test/test_run_tests.py`, `test/test_setup.py` | tests of the build cache (a directory for the bucket, and the stand-in for aws), of the release suite's runner (a toy crate) and of `setup.sh`'s build logic (a stand-in for cargo); no cloud, no release build |
| `Dockerfile`, `Dockerfile.dockerignore` | a Linux build-and-test image ([Dry run in Docker (optional)](before-a-big-run.md#dry-run-in-docker-optional)) |

The scripts that source `run.sh` keep their state in one directory, `$TAXMAN_WORK` (default `~/taxman-work`), among
it `results/strict.jsonl` (and
`exact_parity_A_B.jsonl` of `prover.sh`), `ckpt/` (the checkpoints, `taxman.lock`), `logs/`, `chain.pid` (`prover.pid`)
and `deadline`; the ledger's `cloud.sh` keeps its files in `OUT`.  Nothing is written
inside the checkout but `solver/frame/target` and `solver/noframe/target`.

## What has run where

The scripts have run on macOS (arm64), most of them on EC2 (Amazon Linux 2023, Graviton), and none on Ubuntu.  [Dry run
in Docker (optional)](before-a-big-run.md#dry-run-in-docker-optional) and [A small instance
first](before-a-big-run.md#a-small-instance-first) exist to find, on a machine that costs cents, the problems that a run
on macOS cannot.

| script | on macOS, or in the Docker image | on EC2 |
|---|---|---|
| `chain.sh` | a complete chain, a SIGTERM stop, deliberate interruptions and resumes, a crash loop and a deadline; in the image (Linux, arm64) after the split of `run.sh`; through the change of `run.sh` for `certify.sh` (`run_start`, `run_reap`) to a complete chain, a SIGTERM stop and a deadline | the chain's run of 2026-10-15 and 16 (2176 to 3440 on a `c8g.16xlarge`, 24 hours, two instances: [the cloud run to 3440](../../notebook/the-frontier-chain-to-3440-2026-10/README.md)), after a rehearsal of [Rehearsal of the chain, with interruptions](before-a-big-run.md#rehearsal-of-the-chain-with-interruptions) on the same machine type |
| `setup.sh` | its build and checks | the prover's run (`PROVER_ONLY=1`), the chain's run, the scaling run on a `c8g.48xlarge` (in full, without the tests), and the releases of [build-cache.md](build-cache.md) |
| `verify.sh` | yes | the chain's rehearsal; the scaling run |
| `save.sh` | | the prover's run (with an instance role); the chain's run (with its restore on the second instance) |
| `spot-watch.sh` | | the prover's run, the chain's run, and [the run to 5000](../../notebook/the-run-to-5000-2026-10/README.md), whose two spot interruptions it handled |
| `prover.sh` | over a range, with a SIGTERM stop and a resume, on macOS and in the image | the run of [prover.md](prover.md) (2026-10-14 and 15, to the end) |
| `certify.sh` | `taxman certify` on two lists at once (bash 3.2) through a complete run and its `verify-certificates`, a SIGTERM stop and its resume; with a stand-in for the binary, a counterexample, an error, a crash and a deadline | not run |
| the instance-hours ledger of `run.sh` | against a stand-in for aws (`test/run.sh`) | [the run to 5000](../../notebook/the-run-to-5000-2026-10/README.md) |
| `status.sh` | against a stand-in for ssh (`test/run.sh status`) | not run: it reads a chain on a machine reached by ssh |
| `ledger/cloud.sh` | the laptop's dry run of 1301..1500, with the test profile's `ledger` | the ledger's launches of 2026-10-17 and 18 ([its entry](../../notebook/search/the-ledger-bounds-from-both-sides-2026-10/README.md)) |
| the build cache and the release suite | against a directory for the bucket, the stand-in for aws and a stand-in for cargo; the runner, the whole suites of the solver and the prover in the test profile | the releases and the cache of 2026-10-18 ([What was tested](build-cache.md#what-was-tested)) |

The [calibration](../../notebook/graviton-calibration-2026-10/README.md) ran the solver and the prover on a
`c7g.16xlarge` and a `c8g.16xlarge` with its own scripts.  It and the [scaling
run](../../notebook/scaling-to-192-threads-2026-10/README.md) measured the speed of a Graviton core and the scaling of
the solver to 64 and 192 threads; what is not measured is in [solver/docs/AGENDA.md, "What the cloud runs should
measure"](../AGENDA.md#5-what-the-cloud-runs-should-measure).
