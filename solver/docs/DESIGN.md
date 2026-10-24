# Design: the frame solver for optimal Taxman

The frame solver ("the solver" below) finds optimal games of Taxman: for each n, a game on the numbers 1 to n that
scores as much as any game can.  With each game it records how the game's optimality is proven;
[frame-model.md](design/frame-model.md) says what "optimal" means here.

The design explains what the solver does, and why, a file a part (the map below).

The other documents:

* [STATUS.md](STATUS.md) is the current state of the work: the priority, and the projects in flight.  Read it first.
* The [README](../frame/README.md) is the user guide: how to build and run the solver.  The [reference](REFERENCE.md)
  has every flag and command, the output, and what a result records.  The [glossary](../../docs/GLOSSARY.md) defines the
  project's words.
* The [lab notebook](../notebook/README.md) says what each part was worth and what was tried instead.  Its register, a
  file a subject, holds everything tried, kept or not; each entry is a folder in [solver/notebook/](../notebook/).
* In the code, each module starts with a comment that says what it is for, and [`src/lib.rs`](../frame/src/lib.rs) has
  the module map.

## Goals and constraints

* **Go past n = 1000**, where the Java solver's table of optimal games stopped, as far as the machines at hand reach: a
  laptop (an Apple M1 Max: 10 CPUs, 32 GB of RAM, a fast SSD) and a cloud machine of many CPUs ([RUNBOOK](RUNBOOK.md)).
  [results/README.md](../../results/README.md#strictjsonl) says how far the committed chain of games reaches, and
  [results/runs.md](../../results/runs.md) which machine made each part of it.
* **Be right before being fast.**  A game counts only once an independent
  [referee](../../docs/GLOSSARY.md#results-and-trust) has replayed it.  Every result records how its optimality is
  proven and what that proof takes as given.  Every [tactic](../../docs/GLOSSARY.md#the-search) is proven or sound
  ([tactics.md](design/tactics.md)), and can be switched off to measure its effect on speed.
* **Search an enormous tree efficiently.**  Each node is fast on a single thread.  The search scales cleanly to the CPUs
  available, up to a 192-CPU machine, without depending on many of them.
* **Never rely on everything fitting in RAM.**  The configuration bounds memory use, whatever the size of the tree.
  Work that does not fit goes to disk.
* **Lose little to a restart.**  A machine can restart without warning, and a job that writes its output only at the end
  then loses all of its progress.  So every long run checkpoints often and resumes after a restart, losing at most one
  checkpoint interval.  Stopped by SIGINT or SIGTERM, it loses nothing.
* **Iterate fast.**  Correctness and performance are worked out on small and medium games that finish in seconds: the
  tests, the benchmark set and the probe ([performance.md](design/performance.md)).  Big runs come last.
* **Standard library only**: no third-party crates.  The solver runs on Linux and macOS.

## The files

Each file explains one part of the solver and the reasons for it, and defines each term where it first matters.

| topic | file | read it to |
|---|---|---|
| the game and the frame model | [frame-model.md](design/frame-model.md) | see how the solver models a game, and what "optimal" means for each kind of proof |
| the architecture | [architecture.md](design/architecture.md) | find your way in the code: the modules in their reading order |
| the search | [search.md](design/search.md) | change the search: its state and order, the matching bound and its conflicts, forced members and nogoods, the Lagrangian bound, the default step, conflict branching, the start, the near-repeat cut |
| price reuse | [price-reuse.md](design/price-reuse.md) | change how conflict branching keeps its prices from one check to the next, and see why that keeps the search exact |
| the finder | [finder.md](design/finder.md) | see how a thread beside a long search looks for a better incumbent, and why it cannot make an answer wrong |
| extended mode | [extended-mode.md](design/extended-mode.md) | search the extended problem, with one-sided candidates, that a certificate of U searches |
| the GF(2) bound, prime exchanges | [gf2-bound.md](design/gf2-bound.md) | change the GF(2) bound, or the symmetries of the promotions that prime exchanges give |
| the tactics | [tactics.md](design/tactics.md) | add, change or switch off a tactic, and see what each one is proven or sound by |
| the engine, the per-game pipeline | [engine.md](design/engine.md) | change how the threads share a search, how one game goes from its board to its result, or the window of games in flight |
| results and the trust model | [trust.md](design/trust.md) | see why a result can be trusted: what a window changes, past limits, known games, the ledger |
| same search | [same-search.md](design/same-search.md) | see why a game whose search problem an earlier game posed needs no search |
| certificates of U | [certificates.md](design/certificates.md) | make or check a certificate of [statement U](../../docs/theory/exactness.md#statement-u) |
| testing | [testing.md](design/testing.md) | test a change: why small games, the quick check and its limits, and the checklist before a merge |
| the tests | [tests.md](design/tests.md) | find the tests that cover a part, and where a new test belongs |
| performance | [performance.md](design/performance.md) | make the solver faster: what it costs, the benchmark set and the probe |
| the lab channel | [lab.md](design/lab.md) | let a lab watch and steer a live chain (the hook, the command folder), and see why nothing it does can make a result wrong |
