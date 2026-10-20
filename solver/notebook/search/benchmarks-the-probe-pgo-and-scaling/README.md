# Benchmarks, the probe, PGO and scaling

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-16.

**Question.**  How fast was the solver when the measuring tools ([DESIGN.md,
"Performance"](../../../docs/design/performance.md) says how to use them) were first turned on it, what did
profile-guided optimization and more threads buy, and where did the time go after each round of work?  **Answer.**  The
strict chain 1..1000 took 58.7 s of game time on 4 threads of the laptop, against over 8 hours for the Java's ten
slowest games on 192 vCPUs.  PGO took 12% off the cycles before the inner-loop round and 0-2% after it, threads scale to
the 8 performance cores (the proof of 1215: 62.5 s on one thread, 12.9 s on 6), and the Hungarian repairs stayed the
largest cost.

## Method

* **The bench set.**  `taxman bench` ran n = 480–760 (W = 8 and 16, milliseconds per game) until 2026-10, which no
  longer exercised the frontier's hot paths (the Java took 2.0–65.6 s for those on one thread and 1.4–12.4 s on ten).
  The set became 936, 1050, 1100, 1133, 1170 and 1184, about a second each on one thread.  A window of a long proof is
  not a game the solver can run, so it is in the probe.
* **The probe** (`examples/perf.rs`), about 5 s in all: F1, 300 search nodes of each of the proofs of 936 and 1215 with
  every kernel over all of them; F2, the frame checks of those promotions; F3, a checkpoint of each proof (1215 after
  1.5M nodes) resumed on one thread for 50,000 steps, one period of forgetting nogoods.  It reports cycles and
  instructions retired of the process, since other load on the machine barely moves them, unlike wall time (up to 3x).
* **Profile-guided optimization** is `solver/tools/pgo.sh`.  Profile the plain build: with PGO the search is inlined
  into a few functions and `sample` sees little.
* **Thread scaling** used the proof of 1215 on the M1 Max (8 performance cores, with another job on up to 4 of them):
  `ladder --seed-opt --known results/default.jsonl --tactic revise_freedoms=off`.
* **The fingerprints** of the probe's windows changed with the checkpoint header (versions 4, 5 and 6) and when
  `force_window` became `all`; with `--tactic gf2=on` it was f3deaac0dbdc030a before that, which `--tactic gf2=on
  --tactic force_window=80` still gives.  The fingerprints 36981d0eaeeae319 and 6197a47eae6cf8c8 are of the windows
  made before `lagrange`.

## Results

* **Against the Java.**  The Java's best run (the notes of 2022-09-09) made the chain 1..1000 on an EC2 r6a.metal (192
  vCPUs, 1,536 GB, $3.60 an hour), with `reuse_prev` and `revise_freedoms` on.  Its ten slowest games took 29,766 s
  together, over 8 hours (990 15,858 s, 987 3,896, 996 3,624, 992 3,519, 988 1,559, then 999, 1000, 833, 969 and 984 at
  119-516 s).  The strict chain of commit 243af9b (2026-10-13, nothing unproven) took 58.7 s of game time for the whole
  of 1..1000 on 4 threads of the laptop, and 0.09 to 12.9 s for those ten games (140 to 4,600 times less wall time on 4
  threads against 192).  The same cluster, 987-999, holds the hardest games of both.  On the laptop's single thread
  (2026-10-12), the Java took 18.0 s at 700, 23.1 s at 720 and 65.6 s at 760, where the solver took 0.030, 0.026 and
  0.047 s, each from `optimal.json`'s game of n − 1.
* **PGO.**  On the code before [faster nodes, same trees](../faster-nodes-same-trees-2026-10/README.md) it took 12% off
  the cycles of the proof of 1215 (151.1 → 132.5 G), 13% at 1184, 6% at 936 and 9–11% on games it was not trained on
  (880, 1050, 1100, 1170), mostly, it turned out, by inlining the Hungarian search's heap; with that inlined by hand it
  adds 0–2% (1215: 106.0 → 105.1–106.4 G, 1184: 3.16 → 3.10 G).
* **Thread scaling.**  The proof of 1215 took 62.5 s on one thread, 24.6 s on 2, 15.8 s on 4 and 12.9 s on 6, with
  effective parallelism 99.9–100% and all workers busy within 3 ms; node counts vary by ±10% with the threads (what
  each thread's nogood table has learned).  Games that take minutes on one thread (1215, 1220, 1225 with `--seed-opt`:
  about 60, 150 and 200 s) suit a many-core machine; at 192 threads a game of a minute is a third of a second, mostly
  ramp-up and tail.  [cost-curve.md](cost-curve.md) has the later measurement of the proof of 1225 on 1, 2, 4 and 8
  threads and the cost curve beyond the frontier.
* **The ladder before and after the conflicts round** (10 threads, standalone from the frame builder, seeded from n −
  1).  Before, 992 and 1000 were open at 100 s (992's incumbent not even optimal), 960 took 37 s and 800 13 s, and
  eleven games of 945–999 were open at 20–30 s.  After, all complete: 760 0.03 s, 800 0.5 s, 840 0.1 s, 880 0.3 s, 900
  0.1 s, 960 1.6 s, 945–999 1.4–24 s except 992 (62 s, of which 42 s finding the optimum) and 996 (69 s, all proof).
* **Whole ranges through `taxman solve`** (after the conflicts round).  761–1000 (prime shortcut and `reuse_prev` on; 81
  games searched) took about 5 minutes of game time on 10 threads, every score matching optimal.json; 1001–1100 about
  6.5 minutes (36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with `reuse_prev` and
  `revise_freedoms`, as the Java's results).  Threads scale linearly to the 8 performance cores; the 2 efficiency cores
  add little.
* **The strict chain after the forced-members round.**  `taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off
  --tactic revise_freedoms=off` reproduces every score of `results/strict.jsonl` in 164 s of game time; those results
  took 1244 s on 10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s), 1020 6 s (40 s).  After
  [faster nodes, same trees](../faster-nodes-same-trees-2026-10/README.md) the strict chain 1..1231 on 8 threads took
  279 s of game time (1201–1231: 145 s), every score as committed.
* **Where the time went on the hard games.**  At 1184 before the second round (`sample`): peeling for conflicts 29%,
  matching repairs for refinement 18% and for strong branching 16%, the candidate filter's peels 16%.  At the 1215 proof
  after faster nodes, same trees: the Hungarian repairs 46%, the peels 24%, the bounds from the potentials 7%.
* **The test suite** took about 28 s with two test threads in the test profile (`opt-level = 2`) and 24 s in release,
  after the tests of the wide-board, fixture, tripwire, soundness and CLI rounds and the trimming (before: 30 s and
  26 s).  A few tests sample what an exhaustive run would cover; the comment on each says what the whole run costs.

## Later (2026-10): the measurements that DESIGN.md held

DESIGN.md's section "Performance" held these figures of past builds until 2026-10-18; DESIGN.md keeps the tools and
where performance stands.  They are dated by the build, on the M1 Max.

**With the GF(2) bound** (the default since commit 7af8b0e; [the GF(2) bound in the
search](../the-gf2-bound-in-the-search-2026-10/README.md) and [the inner loop of the GF(2)
search](../the-inner-loop-of-the-gf2-search-2026-10/README.md) have the tables):

* The proof of 1215 takes 4,634 nodes and 0.85 G cycles on one thread, against the matching path's 1.1 M nodes and 29 G;
  the 146 one-thread proofs of the slope study 287,101 nodes and 52 G cycles (697,346 and 232 G before the inner-loop
  round; 3,408 s on the matching path); the proofs of 2170 and 2172 65 s each on one thread; `taxman regress --to 1000`
  on one thread 15.4 s; the chain 1..1300 on one thread 63 s (34 s of search), against the 988 s for 1001–1300 that the
  chain of commit 271c817 records on 4 threads.
* Strong branching makes almost no augmentations (the bounds without each member are read off the exchange graph), and
  most children inherit or are certified.  In the proof of 1521 (one thread) the searches for forced members take
  about a quarter, the weight split a quarter, the solves that are left a fifth (with the tableaux they build, a
  quarter), and the cycles of strong branching a ninth.  A node costs 180k cycles on average over the 146 proofs (330k
  before the round), in trees 2.4 times smaller.
* The rule of symmetric pairs ([prime exchanges](../prime-exchanges-in-the-search-2026-10/README.md)), on by default
  since, takes the proof of 1215 to 3,647 nodes and 0.74 G cycles, 2170 and 2172 to 55 s and 62 s, the chain 1..1300 on
  one thread to 36 s (26 s of search) and 290,503 nodes against 42 s (31 s) and 402,205 just before it; the one-thread
  proofs of the quick set and the bench set 0.75 of the nodes and 0.81 of the cycles (geometric means).
* The window `all` of the forced members, the default since 2026-10-16, takes the chain 1..1300 on one thread to
  276,918 nodes against 290,503 and 24.8 s of search against 23.7 (the small games' trees are too small for the extra
  searches to pay), and the trees at the end of the chain to 0.41-0.43 of the nodes and 0.60-0.63 of the CPU ([the
  tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).
* A chain from optimal.json's game of 1300 reached 2175 in 92 minutes of game time on 2 threads, and the committed chain
  from n = 1 to 2175 took 27 minutes on 8 ([results/README.md](../../../../results/README.md)); its cost comes in waves
  that follow the root's GF(2) gap, with no growth that a fit over 1301–2175 resolves.  Past 2175 it grows, tenfold per
  about 1,040 in n, and a day on one 64-CPU cloud machine took the chain from 2176 to 3440 (the [cloud
  run](../../the-frontier-chain-to-3440-2026-10/README.md) sets it beside the projections of the day: near 4,100, and
  3,400 to 3,700 at their pessimistic end); the widths go to 8191.
* At the end of that chain (3441 resumed from the cloud run's checkpoint, one thread) a step cost 888k cycles, 92% of
  it in the GF(2) bound; with its tableaux derived, a hub for the elements free in `B` and the other changes of
  [cheaper GF(2) nodes](../cheaper-gf2-nodes-same-trees-2026-10/README.md) it costs 543k, the same trees (3,570 to 5,865
  nodes a CPU-second).

**The matching path** (the default before the GF(2) bound; the build of commit 4b6ef38):

* The proof of 1215 takes 33 s on one thread (106 G cycles), and 1184 takes 3.2 G cycles.  The chain 1..1231 took 279 s
  of game time on 8 threads, 145 s of it for 1201–1231, at that commit; the committed `chain.jsonl` (1001–1231), made by
  the build before it (e710d16), records 318 s, and 207 s for 1201–1231.  The cost of a game grows by a factor of ten
  for every 55 to 70 in n past 1200 ([cost-curve.md](cost-curve.md)).  The chain 1..1300 of commit bd9bb5d (on 6 threads
  of an M1 Max shared with another CPU-heavy job) records 3359 s of search, 2866 s of it for 1232–1300; its slowest game
  is 1232 at 439 s.
* With the [Lagrangian bound over the nogoods](../a-lagrangian-bound-over-the-nogoods-2026-10/README.md) the proof of
  1215 takes 1.1 M nodes and 34 G cycles instead of 5.5 M and 108 G, 1184 1.6 G cycles, and the chain 1201–1231 on four
  threads 117 s of game time instead of 283 s.  The round after it took about a tenth off the cycles with the same trees
  (commit 31b9c95: the proof of 1215 30 G cycles, 1184 1.5 G, the chain 1201–1231 on one thread 315 s instead of 345 s;
  a step of the probe's window of 1215 22.0k cycles, 14.6k with `lagrange=off`; [faster nodes with the Lagrangian
  bound](../faster-nodes-with-the-lagrangian-bound-same-trees-2026-10/README.md)).
* Where the time went after that (the proof of 1215, one thread): the Hungarian searches of the matching 33% and of the
  Lagrangian levels 19%, the peels 19%, the node's own work 10%, the nogood table 4%, the refits of the multipliers 3%,
  the bounds from the potentials 6%, level copies 4%; by purpose the Lagrangian bound a quarter, making children a
  quarter, refinement 14% and strong branching 11%.

## Decision

Kept: the probe and the bench set as the project's measuring tools, and PGO as an option (`solver/tools/pgo.sh`).  What
was left to try at the end of the second round:

* the incumbents of games like 992 and 1025, whose optimum the search reaches late (992 on 8 threads: at 11.8 s of
  12.0 s; seeded with its optimum it takes about 6 s);
* the matching repairs, still the largest cost;
* `i32` potentials (no bound on the potentials is proven, so not exact by construction), a warm-started unit rebuild
  (another optimal matching can change the tree after a split or a resume), candidate-index sets for every
  search-level set (a large refactor), and watched members for the nogoods (3% of the time).

## Files

* `cost-curve.md`: the cost-curve study, how the cost of solving grows with n around and beyond the end of the chain
  (the matching path), with its method, tables, fits and projections; it is its own record.
* `run_chain.py` makes `chain.jsonl`: `taxman solve 1001-1231 --use-prev` on 8 threads, resumable (the chain 1001–1231
  as `taxman solve --out` writes it).
* `run_games.py` makes `calib.jsonl` (standalone ladder runs of the 197 non-primes in 1001–1231), `bands.jsonl`
  (standalone runs beyond the end of the chain: bands at 1250, 1300 and 1350, 3 games), `scaling.jsonl` (the thread
  scaling of the proof of 1225) and `recheck.jsonl` (four games repeated at the end, for run-to-run and background-load
  noise): one `ladder` process per game, resumable.  `load.jsonl` is what the machine was doing apart from the runs,
  sampled before and after each invocation.
* `common.py`: paths, constants, JSONL helpers, primes and the load sampling that the two scripts share.
* `analysis.py` makes the tables from the JSONL files (calibration, band statistics, fits, projections):
  `./analysis.py > analysis_output.md`.

## Commits

* fe215ba: DESIGN.md splits into the current design and EXPERIMENTS.md, where these measurements went.
* ab8a683: the cost-curve study and its scripts and data.
* 243af9b, e710d16, 4b6ef38, 7af8b0e, bd9bb5d, 271c817 and 31b9c95: the builds the figures above name.
* 7c350d4: the Performance section's measurements of past builds move here.
