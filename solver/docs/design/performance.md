# Performance

Part of the [design](../DESIGN.md).

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other
load on the machine moves wall times by up to 3×.  The probe measures it:

* **The probe** (`cargo run --release --example perf`) replays the hot paths of the hardest games from fixtures in
  `testdata/perf/` (0.2 MB), and measures them in cycles and instructions retired of the process (`proc_pid_rusage`).
  Each part has a fingerprint of what it computes, which an exact optimization must leave unchanged:
  * Fixture F1: 300 search nodes from each of the proofs of 936 and 1215, every kernel over all of them (the rebuild of
    a node, the promotion of every candidate, the conflicts of the heaviest matching, the matching from scratch, nogood
    lookups and rule-outs);
  * Fixture F2: the frame checks of those promotions;
  * Fixture F4: the GF(2) bound's kernels at the nodes of F1 (the bound from scratch; with its weight split and forced
    members; without each member of its heaviest set's conflict, solved, and as strong branching reads them, certified
    or solved);
  * Fixture F3: a checkpoint of each proof (`perf make-window 936 4500`, `perf make-window 1215 600000`, made on the
    matching path, each with `--tactic gf2=off --tactic lagrange=on --tactic symmetry=off`, by the builds that had
    them), resumed on one thread for 50,000 steps, one period of forgetting nogoods: the search itself, with the GF(2)
    bound and the other tactics its checkpoint records (`symmetry` and conflict branching off; `--tactic` changes them),
    which finishes both searches, fingerprinted by its counters less the GF(2) bound's counts of solves and
    augmentations, which measure work (a search that ends inside the window is costed by the steps it took).

  The kernels and the windows search the forced members as `--tactic force_window` says (the default, `all`).  The
  probe's fingerprint is c3181decd7f69390, and 2b7ddd68a115466f with `--tactic force_window=80`.  The builds that had
  the matching path searched the windows on it by default, and searched them as this build does with `--tactic gf2=on`
  (with the Lagrangian bound of the windows' tactics, which changed neither tree); every kernel line and both windows
  are the same as theirs then, but for the nogood lookups, whose forced members were read off the matching bound's
  potentials, which this build no longer computes, and the kernels of the matching bound as the node bound, which went
  with it.  It takes about 5 s in all; `perf make-nodes` (with the arguments each `.nodes` file's header records) makes
  the nodes again, deterministically, and `perf make-window` makes a window of this build's search, not of the matching
  path.  For whole searches,
  `/usr/bin/time -l` reports the same counters for a process.  `cargo test --release` rebuilds the examples in its
  own configuration, with about 2% more instructions: run `cargo build --release --examples` again before measuring.
The engine's own benchmark (`cargo test --release --test engine -- --ignored --nocapture bench`, its sizes and thread
counts set by `TXENGINE_BENCH_*`) measures the engine's overhead per node, with no work per node.

The cloud scripts (`solver/tools/cloud`) run the chain on a machine of many CPUs
([runbook/before-a-big-run.md](../runbook/before-a-big-run.md) and [runbook/chain.md](../runbook/chain.md)).  The
command `taxman bench`, the example `ladder` and `scaling.sh` were removed on 2026-10-30; they build at commit 3ca11bf.
The file `solver/frame/bench/history.jsonl` is the record of the benchmarks of 2026-10.

**Where it stands.**  At the end of the chain a step of the search cost 888k cycles on one thread of the M1 Max, 92% of
it in the GF(2) bound, and [cheaper GF(2) nodes](../../notebook/search/cheaper-gf2-nodes-same-trees-2026-10/README.md)
took it to 542k, with the same trees, and [what a GF(2) step carries from the
last](../../notebook/search/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md) 31 to 39% more.  The
cost of a game comes in waves that follow the root's GF(2) gap, and past 2175 it grows tenfold for about every 1,040 in
n ([the chain to 3440](../../notebook/the-frontier-chain-to-3440-2026-10/README.md));
[results/runs.md](../../../results/runs.md) says how long the committed chain took.  The measurements of the earlier
builds, from the matching path to the GF(2) bound's rounds, are in [the benchmarks
entry](../../notebook/search/benchmarks-the-probe-pgo-and-scaling/README.md#later-2026-10-the-measurements-that-designmd-held)
and the entries of each round.
