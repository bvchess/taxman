# The build cache and the release suite

Part of the [runbook](../RUNBOOK.md).

A machine does not build what another machine has already built and tested: `setup.sh` takes the commit's release
binaries from S3.  The build is the dearest part of a start, about 10 minutes, with fat LTO and one codegen unit, which
stay, since the speed of the programs matters more than build minutes.  A run that is replaced after an interruption
would pay it again on every machine.  Nothing untested is stored, so a hit starts the job at once.

## The key and the layout

An entry of the cache is a set of binaries of one commit, built for one architecture and one CPU, under the key of the
three:

```
s3://taxman-013764204840-us-east-1/builds/COMMIT-ARCH-CPU/SET/binaries.tar.gz
                                                          /manifest
s3://taxman-013764204840-us-east-1/builds/test-times.json
```

The docstring of `solver/tools/cloud/build_cache.py` documents the key, the sets and the manifest.  What a launch
depends on:

* **The key** is the commit of a clean checkout (no uncommitted change, no file git does not know of; build outputs are
  ignored files and do not count), `uname -m`, and the CPU the build targets.  For `-C target-cpu=native`, the default
  of `env.sh`, that is the name the compiler reports (`neoverse-v2` on Graviton4), because the flag makes a binary for
  the CPU that builds it; with no flag it is `generic`.
* **The cache is not used**, with a message that says why, and the build goes on as before, never reading or writing the
  cache, for a dirty checkout, no commit (a copy without `.git`), `RUSTFLAGS` other than empty or one
  `-C target-cpu=...`, no `aws` command, or `BUILD_CACHE=off`.  With no credentials a lookup is a miss and a put fails.
* **SET** names what an entry's tarball holds, by its parts: `solver-bin-taxman+solver-example-ledger`, say, or
  `prover-bins+prover-examples+solver-bins+solver-examples` for everything.
* **An entry serves a request it covers**, and a request that none covers is a miss: `--bin X` by `--bin X` or `--bins`,
  `--example X` by `--example X` or `--examples`.  The release holds everything (the default of `setup.sh`), so it
  serves any `BUILD`.  An entry is never completed: a request that no entry covers builds what it needs and stores
  nothing.  The study tools (cargo's feature `studies`: `tune`, `extended`, `structure` and `wall`) are a part of their
  own, so a default build never claims to hold `wall`, and an entry with them also serves a request without them (the
  smaller entry first, when both exist).
* **Only a tested entry is used.**  The manifest, uploaded last, records the build and the release suite's result
  (`tests=passed`, the counts in lines such as `tests_solver=P passed, 0 failed, I ignored`, its seconds and the
  tests at once); an entry without one does not exist.  The tarball holds the executables only
  (`solver/frame/target/release/taxman`, `.../examples/ledger`, `solver/noframe/target/release/exact`).
  `test-times.json` is how long each test took last time, for ordering the suite ([The
  release](#the-release-the-suite-then-the-cache)).
* **A download is checked**: the tarball's sha256 against the manifest's before anything is unpacked, and every member
  of the tarball must be a file under a crate's `target/release`.  A mismatch is a miss, and says so.
* **Listing and deleting.**  `aws s3 ls builds/` lists the commits, and `aws s3 cp .../manifest -` says what an entry
  is.  Old entries cost cents, and are deleted by hand; a deletion in the bucket needs the account owner's yes first,
  like every change there.

## How a launch uses it, and what it builds

`setup.sh` looks for an entry that covers the set it was asked for (`BUILD`).  Finding one, it downloads, checks and
unpacks it, builds nothing, and goes on to the smoke checks, which show that this machine runs the binaries.  Finding
none, it builds the set and does not store it.  `REQUIRE_CACHED=1` turns a miss, or a cache it cannot use, into an
error, for a launch that must run only tested binaries.  The user data of [the ledger's experiment](ledger.md) and [the
sweep](sweep.md#launch-watch-stop-bring-the-results-home) use it, and the run to the wall takes it from the environment
of `wall.sh launch` ([wall.md](wall.md#launch-watch-stop-clean-up)), so that a commit that has not been released ends
the machine at once and costs minutes, not a run of untested binaries.

**Build only what a run needs**, with `BUILD`, cargo's arguments for `solver/frame/` (`--bins`, `--examples`,
`--bin NAME`, `--example NAME`; default `--bins --examples`).  A `BUILD` leaves the prover out, unless `BUILD_PROVER`
asks for it (the same for `solver/noframe/`; `none` for none; `PROVER_ONLY=1` is the prover alone).  The smoke checks of
a part that is not built are left out.  `STUDIES=1` adds `--features studies` to the solver's build and to its tests
(the release suite, `cargo test`), for the study tools, which a build without it skips.  What the jobs need:

| job | `BUILD` |
|---|---|
| the chain, the wall run (`wall-node.sh` sets it), the sweep, `verify.sh` | `--bin taxman` |
| the ledger ([ledger.md](ledger.md)) | `--bin taxman --example ledger` |
| the node-choice measurement | `STUDIES=1`, `--bin taxman --example wall --example perf` |
| the scaling sweep (`scaling.sh`) | `--bin taxman --example ladder` |
| the prover's proofs ([prover.md](prover.md)) | `PROVER_ONLY=1` (`BUILD_PROVER="--bin exact"` for `exact` alone) |

`taxman` is there for the smoke checks.  A machine of 32 CPUs or more builds the solver and the prover at the same time
(each has fewer binaries to link than the machine has CPUs, and every link is a single thread); a smaller one builds
them in turn.  Nothing passes `-j` to cargo, so every build and every test build uses every CPU.

## The release: the suite, then the cache

The release is `RELEASE_TESTS=1 solver/tools/cloud/setup.sh` on **one machine, once for each commit and CPU**, with the
default `BUILD`: everything, so that every smaller request hits it.  It is a launch under the [rules for every
launch](launch.md#rules-for-every-launch): a `c8g.16xlarge` at spot for about 10 to 20 minutes, **about $0.15 to
$0.25**, after the account owner's yes.  Any size serves, since the key is the CPU, which every `c8g` shares; the suite
gains nothing past about 64 vCPUs.  The study tools are the one thing that "everything" leaves out: `STUDIES=1` makes a
release that also serves the node-choice measurement.  Its entry holds `wall`, its suite runs the study tools' tests
too, and it serves every smaller request as well.

```
export S3_DEST=s3://taxman-013764204840-us-east-1/release-COMMIT       # where a failure is reported
RELEASE_TESTS=1 NO_PULL=1 solver/tools/cloud/setup.sh                     # in the clone checked out at COMMIT
```

A hit ends it ("the binaries come from the build cache": nothing to run).  On a miss, it builds everything, runs the
smoke checks, then the **whole release suite of every crate built**, then stores.  If the suite fails, **nothing is
stored**, and `setup.sh` exits 1, so a user data that chains the job with `&&` does not start it.  The failing tests are
in `~/taxman-work/logs/release-tests.txt` (also saved with the logs) and, when `S3_DEST` is set, in
`$S3_DEST/RELEASE_TESTS_FAILED`.  With the cache unusable (a dirty checkout, `BUILD_CACHE=off`) the suite still runs,
and nothing is stored.  `RELEASE_TESTS=1` in a job's own environment does the same on the job's first machine: it
releases what the job needs before starting.  `TEST_JOBS` and `TEST_TIMEOUT` (3600 s a test) adjust the suite.

**The suite runs every test as a process of its own, on every CPU** (`run_tests.py`).  `cargo test` runs the test
binaries one after another, and a long test in one leaves the other CPUs idle.  So the runner builds the test
executables of both crates at once (`cargo test --release --no-run --message-format=json`, which names them all), and
lists each one's tests (`--list`, leaving out the `#[ignore]`d ones that `cargo test` also leaves out,
`--list --ignored`).  It then runs every test with `--exact NAME`, as many at once as there are CPUs, over all the
binaries together: the tests with no time on record first, then the longest first (`test-times.json`, rewritten after
each run, kept in the cache), so that the longest does not start last.  The doc tests run as `cargo test --doc`.

* **A failing test** does not stop the others (as `--no-fail-fast`); the first 20 are run once more alone afterwards, so
  that the
  report says whether it fails only in company.  A test that outlasts `TEST_TIMEOUT` is killed and fails.
* **The summary** (`--summary`) names the ten tests that used the most CPU, with their seconds of CPU: a test's own,
  which do not grow when the machine is busy.  `solver/tools/quick_check.py` runs the suite without `--release` and
  judges it by them ([solver/docs/design/testing.md](../design/testing.md)).
* **Every test in its own process** is stricter than threads of one process (no shared statics, no leftover
  environment), and costs each test its own start; the suite passes that way.  Tests that start a thread per CPU
  themselves share the machine with all the others, so a test that depends on its speed shows it: lower `TEST_JOBS` to
  see whether that is the cause.

**Time.**  On a `c8g.16xlarge` the test pass of the solver's suite in the release of ca357bf ran 699 tests on 64 jobs in
311 s, and that of 260e74f, with the study tools, 752 tests in 306 s; `run_tests.py` reports the test pass and the build
apart.  The build of the test executables comes first, in the release profile with one codegen unit each: the library,
then the test executables side by side.  The release of 260e74f took 16 minutes from the launch to the start of its job,
with the build of the examples (5 minutes), the build of the tests and the suite ([the A1
measurement](../../notebook/search/the-a1-bound-at-the-nodes-of-the-node-choice-measurement-2026-10/README.md)).  The
suite's times on a laptop are in [solver/docs/design/testing.md](../design/testing.md).

## What was tested

* **On the laptop** (macOS, bash 3.2 and 5): `python3 -I test/test_build_cache.py` (the key, the dirty and the
  commit-less refusals, other `RUSTFLAGS`, which entry serves which request, a corrupted tarball, an entry without its
  manifest or its passed suite, a build that lacks a file, a stranger in the tarball, a build with the studies as an
  entry of its own, the same through the stand-in for aws); `python3 -I test/test_run_tests.py` (a toy crate: a failure,
  an ignored test, a doc test, a timeout, the order, a feature, a build that fails); `python3 -I test/test_setup.py`
  (`setup.sh` with a stand-in for cargo: what `BUILD` and `STUDIES` build, a hit and a miss, the release suite on a pass
  and on a failure, the dirty checkout, `REQUIRE_CACHED`, the two builds at once); and `test/run.sh node launch` (the
  settings reach `wall-node.sh` and `setup.sh`).  The runner ran the whole suite of both crates in the test profile; no
  release build was made on the laptop.
* **On EC2, on 2026-10-18:** the releases of ca357bf, for the run to 5000, and of 260e74f, with the study tools, for the
  A1 measurement, each tested before it was stored ([The release](#the-release-the-suite-then-the-cache)); and the
  node-choice measurement at 9d9dae3, which took its binaries from the cache ([its
  entry](../../notebook/search/the-node-choice-where-the-strong-bound-pays-2026-10/README.md)).  All three used the S3
  commands with the instance profile's credentials, and the two releases built the tests in the release profile and ran
  the suite at 64 processes at once.
