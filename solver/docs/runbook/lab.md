# The lab (`lab.sh`)

Part of the [runbook](../RUNBOOK.md).

The lab studies the most expensive subtrees of a running search on the spare threads of its machine, to find rules that
make the search faster.  A search keeps its unexplored work as *units*, each the root of a subtree that can be solved
by itself; the [gap](../../../docs/GLOSSARY.md#bounds) of a unit's node predicts its size.  The example
[`tune`](../../frame/examples/tune.rs) solves units one thread to a unit, so two settings compare in steps with no
noise.  A rule that shrinks the largest units is a candidate for the search; whole games decide it.

## Rules

* **The chain's files are read-only.**  The lab copies from the chain's checkout and from `~/taxman-work`, writes
  nothing in them, and starts, stops and changes nothing of the chain.  It writes only in `~/taxman-lab`.
* **The lab is light.**  Its processes run at `nice -n 19`, a unit takes one thread, and `LAB_THREADS` (default 16)
  bounds the units at once.

## The commands

`solver/tools/lab/lab.sh` runs from your machine with `LAB_HOST=USER@HOST` set, each command one ssh call.  On the
machine itself, `--local` (or a `LAB_HOST` that names it) runs the commands in place, with no ssh.  The machine needs
git, a rustup toolchain and jq.

* `setup [COMMIT]` clones the repository to `src/` at COMMIT (default origin/master) and builds `taxman` and `tune`.
* `snapshot [N]` copies the checkpoint and clock of game N in flight (default: the longest own search) to
  `snap/N-STAMP/`, whose name is SNAPDIR below.
* `units SNAPDIR` writes `units.jsonl` in the snapshot, and prints the summary and the 20 units with the largest gaps.
* `run SNAPDIR IDS` solves the units IDS (ids as `units` printed them, separated by commas), `LAB_THREADS` at a time.

Arguments after `units` or `run` go to `tune` as they are: `--known FILE`, `--tactic NAME=VALUE` and `--node-limit L`
([the top of `tune.rs`](../../frame/examples/tune.rs) lists them).  A leading `~/` in one is the machine's home.

## Reading the results

* `run` prints a row for each unit as it ends: steps, CPU seconds of its thread, wall seconds, the best promotion sum,
  and the state, `done` or `limit` (the node limit stopped it).  Compare CPU seconds: at nice 19 the wall seconds run
  longer.
* `tune` has no time limit and prints no progress, and a step costs more CPU in some units than in others.  Try a small
  node limit first, and set the real one from the `cpu_s` it shows.
* `runs.jsonl` in the snapshot gets the result line of each unit, with the `commit` of the build, and only grows.  If
  the connection drops, the units started go on to their end and add their lines; `pkill -f 'taxman-lab/.*/[t]une'` on
  the machine stops them.
* A unit starts from the checkpoint's incumbent; `--known` with a results file that holds the game starts it from the
  optimum, which makes the unit a proof.

`solver/tools/lab/test.sh` tests the script with no machine and no build.
