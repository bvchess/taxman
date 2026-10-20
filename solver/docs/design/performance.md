# Performance

Part of the [design](../DESIGN.md).

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other
load on the machine moves wall times by up to 3×.  Five tools measure it:

* **`taxman bench`** runs a fixed set of games that need a search and take about a second each on one thread: n = 936
  (W = 16), and 1050, 1100, 1133, 1170 and 1184 (W = 20).  It solves them standalone, with acceleration off (the game
  of n − 1 from `optimal.json`, past it from `results/strict.jsonl`), prints the times of each phase and the node
  counts, and appends them to `solver/frame/bench/history.jsonl`, so that performance can be followed across changes.
  Each line has the commit the binary was built from, every tactic's value, and for each game the nodes, the seconds
  of each phase and the search's counters.  `--history FILE` and `--known FILE` name other files; by default the
  command finds them in the repository.
* **The probe** (`cargo run --release --example perf`) replays the hot paths of the hardest games from fixtures in
  `testdata/perf/` (0.2 MB), and measures them in cycles and instructions retired of the process (`proc_pid_rusage`).
  Each part has a fingerprint of what it computes, which an exact optimization must leave unchanged:
  * F1: 300 search nodes from each of the proofs of 936 and 1215, every kernel over all of them (the rebuild of a node,
    the promotion of every candidate, conflicts, the matching from scratch and warm-started, the bounds without each
    member, strong branching's exact solves, children, nogood lookups and rule-outs);
  * F2: the frame checks of those promotions;
  * F3: a checkpoint of each proof (`perf make-window 936 4500`, `perf make-window 1215 600000`, each with
    `--tactic gf2=off --tactic lagrange=on --tactic symmetry=off`), resumed on one thread
    for 50,000 steps, one period of forgetting nogoods: the search itself, fingerprinted by its counters, with the
    tactics its checkpoint records, the matching path's (`--tactic` changes them; a search that ends inside the window
    is costed by the steps it took);
  * F4, with `--tactic gf2=on`: the GF(2) bound's kernels at the F1 nodes (the bound from scratch; with its weight
    split and forced members; without each member of its heaviest set's conflict, solved, and as strong branching
    reads them, certified or solved), and the windows resumed with the GF(2) bound, which finish both searches.  A
    window's fingerprint leaves out the GF(2) bound's counts of solves and augmentations, which measure work.  The
    kernels and the windows search the forced members as `--tactic force_window` says (the default, `all`).

  The probe's fingerprint is 7e8e724717d26545, 5b83bbebf0bb2899 with `--tactic lagrange=off`, a58ade4629ab6186 with
  `--tactic gf2=on`, and f3deaac0dbdc030a with `--tactic gf2=on --tactic force_window=80`.  It takes about 5 s in all;
  `perf make-nodes` (with the arguments each `.nodes` file's header records) and `perf make-window` (as above) make the
  fixtures again, deterministically.  For whole searches,
  `/usr/bin/time -l` reports the same counters for a process.  `cargo test --release` rebuilds the examples in its
  own configuration, with about 2% more instructions: run `cargo build --release --examples` again before measuring.
* **Profile-guided optimization** (`solver/tools/pgo.sh`) gains 0 to 2%, with the same trees and fingerprints, since
  the hot loop is inlined by hand.  An instrumented build runs the probe and a short search at every set width (code
  that a profile never saw is optimized as cold), and `-Cprofile-use` builds `target/pgo/release/taxman` and the
  examples.  Profile the plain build: with PGO the search is inlined into a few functions, and a profiler sees little.
  On Linux, rustup's `llvm-tools` has an llvm-profdata that matches rustc's LLVM, and the script adds whatever
  `RUSTFLAGS` it is given (for instance `-C target-cpu=native`, which a plain build does not use) to the profile flags.
* **The ladder** (`cargo run --release --example ladder -- [--threads T[,T…] | --scaling] [--seconds S] [--diag]
  [--seed-opt] [--known FILE] [--tactic …] [--node-limit N] N…`; the top of `examples/ladder.rs` documents its flags)
  makes standalone searches of hard games from the frame builder, seeded from the known game of n − 1 and capped as
  the solver does, each with a time limit.  It reports completion, nodes, time, when the final incumbent was found,
  and the engine's counters.  `--seed-opt` seeds with an optimal game of n itself, to measure the proof alone; `--diag`
  prints `search::Diagnostics` (nodes by depth, what pruned, bound gaps, conflict sizes, matching work).  Games for
  fast progress benchmarks (one thread): 880, 936, 1050, 1100, 1170, 1184; harder ones: 992, 996, 1025, 1215, 1216 on
  several threads.
* **Scaling** is the ladder with a list of thread counts (`--threads 1,2,4,8`, or `--scaling` for 1, 2, 4, … up to all
  the CPUs).  It searches each game with each count, and prints a table for each game and one summed over the games:
  search time, speedup, nodes per second per thread, effective parallelism, idle and lock-wait time, splits and the
  units they gave, steals, runs, tiny runs, and the time to full occupancy.  With `--seed-opt` the node count barely
  depends on the threads, so the speedup measures the parallel proof alone.  `--cores C` lets only C of the threads
  search at once (`EngineConfig::cores`), to measure in counts, not time, what many more workers than CPUs would do.
  The engine's own benchmark (`cargo test --release --test engine -- --ignored --nocapture bench`, its sizes and thread
  counts set by `TXENGINE_BENCH_*`) measures the engine's overhead per node, with no work per node.

The cloud scripts (`solver/tools/cloud`) run the scaling sweep, the cost of a checkpoint pause and the chain on a
machine of many CPUs ([runbook/before-a-big-run.md](../runbook/before-a-big-run.md) and
[runbook/chain.md](../runbook/chain.md)).

**Where it stands.**  At the end of the chain a step of the search cost 888k cycles on one thread of the M1 Max, 92% of
it in the GF(2) bound, and [cheaper GF(2) nodes](../../notebook/cheaper-gf2-nodes-same-trees-2026-10/README.md) took it
to 542k, with the same trees, and [what a GF(2) step carries from the
last](../../notebook/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md) 31 to 39% more.  The cost of a
game comes in waves that follow the root's GF(2) gap, and past 2175 it
grows tenfold for about every 1,040 in n ([the chain to
3440](../../notebook/the-frontier-chain-to-3440-2026-10/README.md));
[results/runs.md](../../../results/runs.md) says how long the committed chain took.  The
measurements of the earlier builds, from the matching path to the GF(2) bound's rounds, are in [the benchmarks
entry](../../notebook/benchmarks-the-probe-pgo-and-scaling/README.md#later-2026-10-the-measurements-that-designmd-held)
and the entries of each round.
