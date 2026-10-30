# Testing

Part of the [design](../DESIGN.md).

The tests check correctness on games that finish in seconds.  Two runs use them: the quick check, which every change
passes, and the full suite.  A reviewer of a change runs what the checklist "Before a change is merged" says, and
[tests.md](tests.md) is the catalog of the tests, by subject.

**Why the tests run on small games.**  The large games cannot run in a test, so the fast fixtures and the small games
stand in for them, and the tripwires and soundness tests catch the failures that no assertion on an answer sees.  The
tests run in the test profile: `opt-level = 2`, because they replay real games, with debug assertions on, which the
release profile does not have.

**The quick check and the full suite.**  The *quick check* is the command `solver/tools/quick_check.py` (it works in
`solver/frame`, and runs the formatting check, the lints and the tests of `solver/gzip` too): `cargo fmt --check`,
`cargo clippy --all-targets`, the build of the tests, and the tests of a non-release build, which
`solver/tools/cloud/run_tests.py` runs, every test a process of its own and all the CPUs busy.  After a one-line change
it takes about 12 s to rebuild and about 10 s to run the tests on a laptop of ten cores (a plain non-release `cargo
test`, one binary after another, takes about 19 s).  It verifies most merges, a refactoring included.

**It waits for a quiet machine**, because its limits are for one with nothing else to do.  When the build has ended it
samples what everything on the machine uses of the CPUs for 2 s, and if other work uses more than 25% of the CPUs it
says so, names the busiest processes and samples again until the machine is quiet.  Work run niced (a long search at
nice 10 or SCHED_IDLE) counts as idle, not busy: it gives its CPUs up to a test at once, so the machine is as quiet for
the tests as one with nothing else to do; they still share a core's other hyperthread and the caches with it, so on such
a machine a test may use more CPU seconds than alone, which is the chain's doing and not the test's.  After 10 minutes it fails with
"machine busy" and runs no test, so that a busy machine is never taken for a slow suite.  Two checks do not run at once:
the second waits for the first's lock (a file in the system's temporary directory, which the system frees when a check
ends, however it ends) before it builds anything, says so, and counts that wait in the same 10 minutes.

**It fails**, naming the tests that are over, if the tests take more than 15 s (the target is 10 s; the build and the
wait are not counted) or if one test uses more than 3 s of CPU (CPU, not seconds on the clock, which grow several times
over when every CPU is busy).  The limits and the thresholds of the wait are the constants at the top of the script, and
raising a limit needs the project owner's approval: a test that goes over is made smaller or long, as below, never
allowed more.

**A long test** is one that takes more than about a second in the test profile.
`#[cfg_attr(debug_assertions, ignore = "long: the quick check is quick_one")]` skips it in a non-release run, where
debug assertions are on, and `--release` runs it.  `quick_one` is the quick test that checks the same property at a
small size: a prefix of the range that the long test walks (which then takes the rest of it), a sample of the chain or
of the golden games, fewer random draws, a smaller game.  Where several tests walk one path at different sizes (a ledger
run for each of its features, a certify test for each refusal), the quick check keeps one small test of the path, and
the `ignore` message of each long one names the test that stands in for it.

**The budget.**  The target of 10 s on ten cores is a budget of about 80 CPU seconds for all the tests of the quick
check together, so a test that is added is paid for by shrinking another.  A few tests that draw a count, or build an
instance that must outlast a script, take a smaller one in a non-release run (`QUICK` in `tests/common/support.rs`,
shared with the unit tests: the engine's instances, the random configurations of `robustness`).  Tests with a plain
`#[ignore]` are ignored in both runs.

**A test that stops a search with a node limit** needs a tree that stops it however its workers share it.  A worker looks
at the limit only after 1,024 steps of its own, so a search of fewer than 2,047 steps may never be stopped on two
threads, whatever the limit.  A game of more than twice that stops, as game 1000 does from an incumbent
`QUICK_HANDICAP` below P* (`tests/soundness.rs`); a threshold on the number of stops does not make a
test deterministic.

The *full suite* is `cargo test --release`: every test, the long ones too, about 3 minutes of tests after a build of 6
minutes.  It runs before a cloud run, after any change that changes search trees or results, and in the gates (the
one-thread re-solve of 1301-2175).  A default build, and so every run above and the cloud setup, covers the library, the
binary, the tests, and the examples `ledger` and `perf`.

**Before a change is merged** (what a reviewer of a change runs, besides reading it; the tests are described below, the
probe in [performance.md](performance.md)):

* every change: `solver/tools/quick_check.py`, and `cargo test --release`, `cargo clippy --all-targets` and `cargo fmt
  --check` in `solver/noframe/`;
* a test added or made bigger: `solver/tools/quick_check.py` again; it fails a test that is too slow, which is then
  shrunk, or made long with a `quick_one` (above);
* a change that can change a search tree or a result (the next three items): the full suite in `solver/frame/` besides;
* a bound, or anything that prunes: the bound-soundness tests and the soundness harness pass, and a bound never falls
  below the best completion of its node;
* a pure speedup: every search tree identical, so the probe's fingerprints and the node-count tripwires do not move;
* anything that touches the search, the chain or the results: `taxman regress --to 1000`; a scratch chain (`taxman
  solve 1-N --out FILE`, not in `results/`) compared with `results/optimal.jsonl.gz`; and both verifiers on it, since a
  results line carries provenance that they check (`solver/tools/cloud/verify.sh FILE` runs `taxman verify-results` and
  `solver/tools/check_results.py`, and compares with the committed chain);
* a change to an on-disk format (results line, checkpoint header, `testdata/`): the commit message says so, and why.
