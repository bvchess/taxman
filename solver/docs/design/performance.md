# Performance

Part of the [design](../DESIGN.md).

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other
load on the machine moves wall times by up to 3×.  The probe measures it:

* **The probe** (`cargo run --release --example perf`) replays the hot paths of the games 936, 992 and 1215 from
  fixtures in `testdata/perf/` (0.2 MB), and measures them in cycles and instructions retired of the process
  (`proc_pid_rusage`).  Each part has a fingerprint of what it computes, which an exact optimization must leave unchanged:
  * Fixture F1: 300 search nodes from each of the proofs of 936 and 1215, every kernel over all of them (the rebuild of
    a node, the promotion of every candidate, the conflicts of the heaviest matching, the matching from scratch, nogood
    lookups and rule-outs).  The current search proves those games in 449 and 3,704 nodes, in one unit each, so `perf
    make-nodes` would make a fixture of one node.  These were made when the proofs took 81,119 and 4,652,709 nodes
    (commit 3ca11bf), and stay: a node is a node of the game whatever search found it;
  * Fixture F2: the frame checks of those promotions;
  * Fixture F4: the GF(2) bound's kernels at the nodes of F1 (the bound from scratch; with its weight split and forced
    members; without each member of its heaviest set's conflict, solved, and as strong branching reads them, certified
    or solved);
  * Fixture F3: a checkpoint of the proofs of 992 (11,441 nodes) and 1215 (3,704), made by this build's default search
    (`perf make-window 992 1024`, `perf make-window 1215 1024`: stopped by a node limit of 1,024, which the engine
    reaches after 2,047 nodes), resumed on one thread for 50,000 steps: the search itself, with the tactics its
    checkpoint records (the defaults; `--tactic` changes them), which finishes both searches, fingerprinted by its
    counters less the GF(2) bound's counts of solves and augmentations, which measure work (a search that ends inside
    the window is costed by the steps it took).  Game 936 has no window: its proof takes 449 nodes, fewer than the
    1,024 steps after which the engine looks at a node limit, so no point exists to stop it at.

  The probe's fingerprint is dc66a29d865d44d3: it combines those of every kernel and of both windows, so a change of
  the tree moves it by the windows' and an exact optimization leaves it alone.  A window records the header of the
  build that made it, so a change of the header's format makes the windows again (`perf make-window` with the arguments
  above), and the fingerprint with them.  It takes about 5 s in all (more when the machine is busy).  For whole
  searches, `/usr/bin/time -l` reports the same counters for a process.  `cargo test --release` rebuilds the examples
  in its own configuration, with about 2% more instructions: run `cargo build --release --examples` again before
  measuring.

  **The probe's windows are small.**  The current search proves the games of its windows in a few thousand nodes, so
  the windows fingerprint the code, which an exact optimization must leave alone, and say little about the speed of the
  search on a hard game.  Measuring speed there takes a window of a hard game, made with `perf make-window`.

The engine's own benchmark (`cargo test --release --test engine -- --ignored --nocapture bench`, its sizes and thread
counts set by `TXENGINE_BENCH_*`) measures the engine's overhead per node, with no work per node.

The cloud scripts (`solver/tools/cloud`) run the chain on a machine of many CPUs
([runbook/before-a-big-run.md](../runbook/before-a-big-run.md) and [runbook/chain.md](../runbook/chain.md)).  The
command `taxman bench`, the example `ladder`, `scaling.sh`, the profile tools and the profile-guided build `pgo.sh` were
removed on 2026-10-30; they build at commit 3ca11bf.  The file
[taxman-notebook/bench/history.jsonl](https://github.com/bvchess/taxman-notebook/blob/main/bench/history.jsonl) is the
record of the benchmarks of 2026-10.

**Where it stands.**  At the end of the chain a step of the search cost 888k cycles on one thread of the M1 Max, 92% of
it in the GF(2) bound, and [cheaper GF(2)
nodes](https://github.com/bvchess/taxman-notebook/blob/main/search/cheaper-gf2-nodes-same-trees-2026-10/README.md) took
it to 542k, with the same trees, and [what a GF(2) step carries from the
last](https://github.com/bvchess/taxman-notebook/blob/main/search/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md)
31 to 39% more.  The cost of a game comes in waves that follow the root's GF(2) gap, and past 2175 it grows tenfold for
about every 1,040 in n ([the chain to
3440](https://github.com/bvchess/taxman-notebook/blob/main/the-frontier-chain-to-3440-2026-10/README.md));
[results/runs.md](../../../results/runs.md) says how long the committed chain took.  The measurements of the earlier
builds, from the matching path to the GF(2) bound's rounds, are in [the benchmarks
entry](https://github.com/bvchess/taxman-notebook/blob/main/search/benchmarks-the-probe-pgo-and-scaling/README.md#later-2026-10-the-measurements-that-designmd-held)
and the entries of each round.
