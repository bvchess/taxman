# Benchmarks, the probe, PGO and scaling

Part of the [lab notebook](../README.md).

What the measuring tools were first used to find ([DESIGN.md, "Performance"](../../docs/DESIGN.md#10-performance) says how to
use them).

* **The bench set.**  Until 2026-10 `taxman bench` ran n = 480–760 (W = 8 and 16, milliseconds per game), which no
  longer exercised the frontier's hot paths; the Java took 2.0–65.6 s for those on one thread and 1.4–12.4 s on ten.
  The set is now 936, 1050, 1100, 1133, 1170 and 1184, about a second each on one thread.  A window of a long proof is
  not a game the solver can run, so it is in the probe.
* **The Java's best run, for comparison** (the owner's notes of 2022-09-09): the chain 1..1000 on an EC2 r6a.metal
  (192 vCPUs, 1,536 GB, $3.60 an hour), with `reuse_prev` and `revise_freedoms` on.  Its ten slowest games took
  29,766 s together, over 8 hours: 990 15,858 s, 987 3,896, 996 3,624, 992 3,519, 988 1,559, 999 516, 1000 314,
  833 188, 969 175, 984 119.  The strict chain of commit 243af9b (2026-10-13, nothing unproven)
  took 58.7 s of game time for the whole of 1..1000 on 4 threads of the laptop; its times for those ten games are
  4.9, 0.84, 11.2, 12.9, 2.7, 3.6, 1.3, 0.09, 0.63 and 0.21 s (140 to 4,600 times less wall time on 4 threads
  against 192).  The same cluster, 987-999, holds the hardest games of both.  On this laptop's single thread, against
  the Java's on the same machine (2026-10-12): 700 18.0 s against 0.030 s, 720 23.1 s against 0.026 s, 760 65.6 s
  against 0.047 s, each from `optimal.json`'s game of n − 1.
* **The probe** (`examples/perf.rs`), about 5 s in all: F1, 300 search nodes of each of the proofs of 936 and 1215
  with every kernel over all of them; F2, the frame checks of those promotions; F3, a checkpoint of each proof
  (1215 after 1.5M nodes) resumed on one thread for 50,000 steps, one period of forgetting nogoods.  Cycles and
  instructions retired of the process are what it reports, since other load on the machine barely moves them,
  unlike wall time (up to 3x).
* **Profile-guided optimization** (`solver/tools/pgo.sh`).  On the code before "Faster nodes, same trees" it took 12% off
  the cycles of the proof of 1215 (151.1 → 132.5 G), 13% at 1184, 6% at 936 and 9–11% on games it was not trained on
  (880, 1050, 1100, 1170), mostly, it turned out, by inlining the Hungarian search's heap; with that inlined by
  hand it adds 0–2% (1215: 106.0 → 105.1–106.4 G, 1184: 3.16 → 3.10 G).  Profile the plain build: with PGO the
  search is inlined into a few functions and `sample` sees little.
* **Thread scaling.**  On the M1 Max (8 performance cores, with another job on up to 4 of them) with the proof of 1215
  (`ladder --seed-opt --known results/default.jsonl --tactic revise_freedoms=off`): 62.5 s on one thread, 24.6 s on
  2, 15.8 s on 4 and 12.9 s on 6, effective parallelism 99.9–100%, all workers busy within 3 ms; node counts vary
  by ±10% with the threads (what each thread's nogood table has learned).  Games that take minutes on one thread
  (1215, 1220, 1225 with `--seed-opt`: about 60, 150 and 200 s) suit a many-core machine; at 192 threads a game of a
  minute is a third of a second, mostly ramp-up and tail.  [solver/notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md](cost-curve.md) has
  the later measurement of the proof of 1225 on 1, 2, 4 and 8 threads and the cost curve beyond the frontier.
* **The ladder before and after the conflicts round** (10 threads, standalone from the frame builder, seeded from
  n − 1): before, 992 and 1000 were open at 100 s (992's incumbent not even optimal), 960 took 37 s and 800 13 s,
  and 945, 948, 950, 969, 976, 984, 987, 988, 990, 996, 999 were open at 20–30 s.  After, all complete: 760 0.03 s,
  800 0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s, 960 1.6 s, 945–999 1.4–24 s except 992 (62 s, of which 42 s finding the
  optimum) and 996 (69 s, all proof).
* **Whole ranges through `taxman solve`** (after the conflicts round).  761–1000 (prime shortcut and `reuse_prev` on;
  81 games searched) about 5 minutes of game time on 10 threads, every score matching optimal.json; 1001–1100 about
  6.5 minutes (36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with `reuse_prev` and
  `revise_freedoms`, as the Java's results).  Threads scale linearly to the 8 performance cores; the 2 efficiency
  cores add little.
* **The strict chain after the forced-members round.**  `taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off
  --tactic revise_freedoms=off` reproduces every score of `results/strict.jsonl` in 164 s of game time; those
  results took 1244 s on 10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s), 1020 6 s
  (40 s).  After "Faster nodes, same trees" the strict chain 1..1231 on 8 threads takes 279 s of game time
  (1201–1231: 145 s), every score as committed.
* **The hard games, one and many threads, before and after the second round** (see the table in "Forced members and
  nogoods").  Where the time went at 1184 before it (`sample`): peeling for conflicts 29%, matching repairs for
  refinement 18% and for strong branching 16%, the candidate filter's peels 16%.  At the 1215 proof after
  "Faster nodes, same trees": the Hungarian repairs 46%, the peels 24%, the bounds from the potentials 7%, the node's
  own work 7%, level copies 4%, nogood lookups 3%.
* **The test suite.**  About 28 s with two test threads in the test profile (`opt-level = 2`), 24 s in release,
  after the tests of the wide-board, fixture, tripwire, soundness and CLI rounds and the trimming (before: 30 s and
  26 s).  A few tests sample what an exhaustive run would cover (every 18th golden board for the peeling agreement,
  every sixth board above 511 for the root promotability of the frame builder's candidates, every second line of the
  Java fixtures); the comment on each says what the whole run costs.
* **Left to try** (from the end of the second round): the incumbents of games like 992 and 1025, whose optimum the
  search reaches late (992 on 8 threads: at 11.8 s of 12.0 s; seeded with its optimum it takes about 6 s); the
  matching repairs (still the largest cost); and `i32` potentials (no bound on the potentials is proven, so not exact
  by construction), a warm-started unit rebuild (another optimal matching can change the tree after a split or a
  resume), candidate-index sets for every search-level set (a large refactor), and watched members for the nogoods
  (now 3% of the time).

## Later (2026-10): the measurements that DESIGN.md held

DESIGN.md's section 10, "Performance", held these figures of past builds until 2026-10-18, when they moved here
unchanged; DESIGN.md keeps the tools and where performance stands.  "Now" in them is the date they were written.

*The probe's fixtures and fingerprints.*  The probe's fingerprint with `--tactic gf2=on` was f3deaac0dbdc030a before the
window of the forced members became `all`, which `--tactic gf2=on --tactic force_window=80` still gives.  (The windows
were made again when the search's counters were renumbered and the checkpoint header went to version 4, 2026-10, and
again for version 5, with the tactics they record, the matching path's, and the same fingerprints, and again when
`force_window` became `all`, as a header of version 6, with the same trees for the matching path; the fingerprints
EXPERIMENTS quotes before version 4, 36981d0eaeeae319 and 6197a47eae6cf8c8, are of the earlier windows, made before
`lagrange`.)

*Profile-guided optimization.*  Before the hot loop was inlined by hand, PGO gained 12%.

**Where it stands with the GF(2) bound** (the default since commit 7af8b0e; EXPERIMENTS, "The GF(2) bound in the search"
and "The inner loop of the GF(2) search", have the tables; on the M1 Max): the proof of 1215 takes 4,634 nodes and 0.85
G cycles on one thread, against the matching path's 1.1 M nodes and 29 G; the 146 one-thread proofs of the slope study
287,101 nodes and 52 G cycles (697,346 and 232 G before the inner-loop round; 3,408 s on the matching path); the proofs
of 2170 and 2172 65 s each on one thread; `taxman regress --to 1000` on one thread 15.4 s; the chain 1..1300 on one
thread 63 s (34 s of search), against the 988 s for 1001–1300 that the chain of commit 271c817 (the
`results/strict.jsonl` before it; git has it) records on 4 threads.  Strong branching makes almost no augmentations (the
bounds without each member are read off the exchange graph), and most children inherit or are certified: where the time
goes now (the proof of 1521, one thread) is the searches for forced members about a quarter, the weight split a quarter,
the solves that are left a fifth (with the tableaux they build, a quarter), and the cycles of strong branching a ninth.
A node costs 180k cycles on average over the 146 proofs (330k before the round), in trees 2.4 times smaller.  These are
without the rule of symmetric pairs (section 4, "Prime exchanges"), on by default since: it takes the proof of 1215 to
3,647 nodes and 0.74 G cycles, 2170 and 2172 to 55 s and 62 s, the chain 1..1300 on one thread to 36 s (26 s of search)
and 290,503 nodes against 42 s (31 s) and 402,205 just before it; the one-thread proofs of the quick set and the bench
set 0.75 of the nodes and 0.81 of the cycles (geometric means; EXPERIMENTS, "Prime exchanges in the search").  The
window `all` of the forced members, the default since 2026-10-16, takes the chain 1..1300 on one thread to 276,918 nodes
against 290,503 and 24.8 s of search against 23.7 (the small games' trees are too small for the extra searches to pay),
and the trees at the end of the chain to 0.41-0.43 of the nodes and 0.60-0.63 of the CPU ([the tuning
campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).  A chain from optimal.json's game of 1300
reached 2175 in 92 minutes of game time on 2 threads, and the committed chain from n = 1 to 2175 took 27 minutes on 8
([results/README.md](../../../results/README.md)); its cost comes in waves that follow the root's GF(2) gap, with no
growth that a fit over 1301–2175 resolves.  Past 2175 it grows, tenfold per about 1,040 in n, and a day on one 64-CPU
cloud machine took the chain from 2176 to 3440 (the [cloud run](../the-frontier-chain-to-3440-2026-10/README.md) sets it
beside the projections of the day: near 4,100, and 3,400 to 3,700 at their pessimistic end); the widths go to 8191.  At
the end of that chain (3441 resumed from the cloud run's checkpoint, one thread) a step cost 888k cycles, 92% of it in
the GF(2) bound; with its tableaux derived, a hub for the elements free in `B` and the other changes of [cheaper GF(2)
nodes](../cheaper-gf2-nodes-same-trees-2026-10/README.md) it costs 543k, the same trees (3,570 to 5,865 nodes a
CPU-second).

**The matching path** (the default before the GF(2) bound; these are the build of commit 4b6ef38, on the M1 Max): the
proof of 1215 takes 33 s on one thread (106 G cycles), and 1184 takes 3.2 G cycles.  The chain 1..1231 took 279 s of
game time on 8 threads, 145 s of it for 1201–1231, at that commit; the committed
`solver/notebook/benchmarks-the-probe-pgo-and-scaling/chain.jsonl` (1001–1231), made by the build before it (e710d16),
records 318 s, and 207 s for 1201–1231.  The cost of a game grows by a factor of ten for every 55 to 70 in n past 1200
([solver/notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md](cost-curve.md)).  The chain 1..1300 of commit
bd9bb5d (on 6 threads of an M1 Max shared with another CPU-heavy job; in git history, `results/strict.jsonl` being a
later run, whose times [results/README.md](../../../results/README.md) gives) records 3359 s of search, 2866 s of it for
1232–1300; its slowest game is 1232 at 439 s.  All of this is without the Lagrangian bound (tactic `lagrange`, section
4), which came later: with it the proof of 1215 takes 1.1 M nodes and 34 G cycles instead of 5.5 M and 108 G, 1184 1.6 G
cycles, and the chain 1201–1231 on four threads 117 s of game time instead of 283 s (EXPERIMENTS: "A Lagrangian bound
over the nogoods").  The round after it took about a tenth off the cycles with the same trees (commit 31b9c95: the proof
of 1215 30 G cycles, 1184 1.5 G, the chain 1201–1231 on one thread 315 s instead of 345 s; a step of the probe's window
of 1215 22.0k cycles, 14.6k with `lagrange=off`; EXPERIMENTS: "Faster nodes with the Lagrangian bound, same trees").
Where the time goes now (the proof of 1215, one thread): the Hungarian searches of the matching 33% and of the
Lagrangian levels 19%, the peels 19%, the node's own work 10%, the nogood table 4%, the refits of the multipliers 3%,
the bounds from the potentials 6%, level copies 4%; by purpose the Lagrangian bound a quarter, making children a
quarter, refinement 14% and strong branching 11%.
