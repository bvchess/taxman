# Cheaper GF(2) nodes, same trees (2026-10)

Part of the [lab notebook](../../README.md).

*Question.*  The profile of a search at the end of the chain (game 3441 resumed from the cloud run's checkpoint,
[solver/notebook/the-calibration-for-the-run-to-4000-2026-10/README.md](../../the-calibration-for-the-run-to-4000-2026-10/README.md)) put about 92% of a node in the GF(2) bound: tableaux built again
after a solve 25-28%, the weight split 19-20%, the cycle searches for forced members 13-19%, strong branching 15-18%,
augmenting paths 8-14%, at about four instructions a cycle.  How much of that work can go without changing a single
search tree?

*How it was measured.*  The probe (`examples/perf.rs`, `--tactic gf2=on`, its fixtures of 936 and 1215) and a window
at the end of the chain: a copy of the checkpoint of 3441 (626 M nodes, format 5) as `n3441.window.ckpt` in a fixtures
directory of its own, which the probe now runs alone (a game with a window and no nodes has only the window),
resumed for 50,000 steps on one thread, its fingerprint the search's counters.  Cycles and instructions are the
probe's (the process's own counters); nodes per CPU-second are 50,000 steps over the user and system time of
`/usr/bin/time -l`, net of a run of one step.  Every step was checked as DESIGN.md's "Testing" asks of a pure
speedup: the probe's fingerprints (7e8e724717d26545; f3deaac0dbdc030a with `gf2=on`) and the window's (677529cc18c7b649)
unchanged, the tripwires, `taxman regress --to 1000 --threads 1`, the chain 1..1300 on one thread line for line equal
to master's but for the times and the build (290,503 nodes), and the gf2 tests (also in the test profile, with its
debug assertions, after the radix heap).  The M1 Max had other load throughout (load 4 to 11).

*What changed, in order, with a step of the 3441 window (cycles, instructions) after each.*

| commit | change | cycles | instructions |
|---|---|---|---|
| 97db411 (master) | | 887,767 | 3,578,969 |
| eed1e75 | tableaux derived, not built | 771,739 | 2,536,962 |
| dbd52fd | members reached from the elements free in B through a hub | 609,822 | 1,828,022 |
| fd4a917 | the gap found when asked for; stamps cleared before the counter wraps | 597,273 | 1,825,456 |
| a0ac3d0 | members on a cycle found within the slack not searched; blocks in order not renumbered | 562,599 | 1,735,599 |
| 1d27eff | the cycle searches' queue a radix heap | 542,474 | 1,802,018 |

Nodes per CPU-second on the window, two runs each: master 3,574 and 3,564, after the round 5,848 and 5,882 (x1.64).
On the probe the GF(2) kernels move little (they are solves from scratch and splits at fixed nodes, which the round
hardly touches), but strong branching as the search does it (the split, then the bounds without each member of the
conflict) went from 83.5k to 68.2k cycles a member at 936 and from 145.8k to 125.4k at 1215, and the probe's windows
with `gf2=on` (806 and 3,972 steps) from 162.7k to 137.9k and from 182.4k to 162.4k cycles a step; the probe in all
5.772 to 5.631 G cycles.

* *Tableaux derived, not built* (eed1e75).  15,151 of 20,000 steps of the window made a solved level's tableau again
  (a resolve: every block built from its members, about 52,000 pivots of an element by a member each), for the weight
  split: 7,015 children that inherited their parent's heaviest set, 2,992 certified children, 5,144 nodes going on
  without the member they branched on (from the certified level they kept).  Only part of a tableau is read outside a
  solve, the free elements, the circuits, the member in each slot and the lists, and all of it depends on the set
  alone but for the order of the slots, which a build gives by increasing member.  `Meet::derive` makes that tableau
  from a related one: exchanges for the elements that enter (a pivot that keeps the span; two independent sets with
  one span are always a pivot apart), the contracted members' slots dropped from the circuits, the slots renumbered.
  The children derive theirs from the parent's tableau, which the workspace still holds for 10,000 of the 10,007;
  the nodes from a copy of their own tableau saved when they kept the certified level (`Snapshot`, 74 KB at 3441).
  A block where an element that enters is free, or the numbers that enter and leave differ, is built (5,204 blocks
  in 20,000 steps); the elements the source did not have (the parent's own loops, candidates of the child again, 1.7
  a level) are reduced by the block's members.  Along the gf2 tests' random paths every derived tableau is checked
  against the one a solve builds (2,009 of them).
* *A hub for the elements free in B* (dbd52fd).  An element free in B has an arc to every member, and each such
  element settled offered every member its distance: about 1,560 offers a search for a forced member and 9,200 a
  bound without a member at 3441, the most of their work.  As the elements free in A were already reached through one
  member, the members are now reached through one hub: the least distance less `w2` over the elements free in B
  settled (the split makes `w2 ≤ 0` there, `≥ 0` at members), the members queued by increasing `w2`, the next one
  unsettled at a time.  The distances, the order of the vertices settled and the predecessors are the same: a member's
  predecessor is the one whose offer reached its distance first, the hub's or its own offers', compared by when they
  were made.  In the search for the loss of the smaller sets those arcs improve nothing (each member starts at its
  `w2`) and are dropped.
* *The gap when asked for, and the stamps* (fd4a917).  A derived tableau's gap (its shortest augmenting path, one
  Bellman–Ford pass) is found when the losses of the larger sets first ask for it, which is most of the time (2%).
  The searches' stamps (`seen`, `settled_at`) are compared for equality with a 32-bit counter of a few tens a step: on
  one thread past about 70 M steps the counter wraps, and a stamp left from 2^32 stamps before could pass for a new
  one (an offer or a vertex lost, which in the search for forced members could call a member forced that is not).
  The stamps are now cleared before the counter wraps; before that point the stamps are the same.
* *Members on a cycle found; blocks in order* (a0ac3d0).  A member on a cycle that the search for another member
  found within the slack is not forced (the cycle passes through it, as long), so it is not searched; a derived block
  whose members are already in their slots in increasing order is not renumbered.
* *A radix heap* (1d27eff).  Every arc adds one to the packed distance (length, then arcs), and the hubs' entries are
  later still, so the keys a search pushes are never below the last it took: the queue is a radix heap of the packed
  (distance, vertex) keys when they stay below 2^49, which gives the binary heap's order, ties included (checked by a
  debug assertion in the tests' searches).  More instructions, fewer cycles.

*Not kept: the weight split from a hint.*  The split is the shortest distances from a virtual source, unique, so any
exact search gives it.  Nearly every level has a related split at hand (its parent's, or its own before it lost a
member), and a label-correcting Dijkstra keyed by the distance less that hint settled each vertex about once (778 a
split against Bellman–Ford's 2,522 visits); but with a binary heap a step cost 682k cycles against 563k, and with a
radix heap the hints of certified children and kept levels (whose sets were flipped along a cycle) and the parents'
loops that come back put keys below the queue's last, so it fell back to Bellman–Ford almost every time (579k).
Bellman–Ford, its rounds over the vertices whose potential fell (about three visits a vertex), stays.

*Left for later, with estimates (none tried).*

* The split of a child that inherited its parent's heaviest set is the parent's restricted to it, if a split need
  not be the shortest distances (the parent's satisfies every constraint of the child's): no split at all for 45% of
  the levels, about a tenth of a step at 3441.  It changes the split's bounds and the cycle searches' reduced lengths,
  so the trees; a tactic.
* An exact split by dynamic shortest paths for those children (only the vertices whose shortest path passed through
  a deleted vertex are searched again, from the parent's distances and their tree): perhaps 8-10%, a large change.
* A child that inherited keeps its parent's gap when the parent's shortest augmenting path avoids the promoted and
  the dropped elements (exact): perhaps 3%.
* The blocks built in a derivation because an element that enters is free (5,204 in 20,000 steps; `add` is 6.6% of
  a step) could be pivoted with the rows of the members kept in the tableau.

*The tactics that tune the GF(2) search* (this round, for a harness that will sweep them; the defaults are the
behaviour before, every tree the same): `force_window` (80; a number, a percentage of the slack such as `25%`, or
`all`), `branching` (`exact`; or `forced`, `split`, `largest`), `strong_limit` (`all`; a number), `strong_depth`
(`all`; a number of promotions from which `deep_branching` chooses), `deep_branching` (`split`) and `more_forced`
(`on`).  A checkpoint records them in a header of version 6 when they are not the defaults, and in the header of
version 5 as before when they are.  Each value other than the default finds the optimum of games 1..300 and
931..940 on one thread (`tests/solver.rs`).

*Commits*: fd93371 (the probe's window alone), eed1e75, dbd52fd, fd4a917, a0ac3d0, 1d27eff, f6ad43f (the tactics), and this entry.

## Later (2026-10): the stamp's wrap and the committed chain

The counter that fd4a917 keeps from wrapping, `Meet.generation` (32 bits), belongs to one engine worker for one game:
each worker makes its searcher once, keeps it across the units it runs in that game, and drops it when the game ends
or the window retires it; a resume is a new process.  Before a wrap every comparison of stamps is exact.  After one,
the old code could lose propagation in the augmenting paths (a heaviest set's value below the truth), leave invalid
split potentials, find no cycle through a member that has one (a member wrongly forced), or overestimate the loss of
the smaller sets; each prunes completions, so a search could end exhausted below the optimum, and only a negative
cycle is caught (the process aborts).  The tests of a78dc07 show it: of 2,800 workspaces started just below the wrap
with the old counter, 6 gave a heaviest value below the truth, 51 a member wrongly forced, 136 a wrong loss, 1 the
abort; with the fix, none.

How near the committed games came: a step at n = 3441 takes 46.7 stamps on average (median 30, 99th percentile 208;
the old code, 50,000 steps of the window), so a wrap needs about 92 million steps of one worker in one game.  On the
64 workers of the cloud run, 3440 (3.72 billion nodes in 18,184 s) took about 58 million steps a worker, at most 64
million: 63-70% of the wrap at the mean rate, a margin under two that is measured, not proven.  3425 took at most 44
million (under half), 3424 at most 25 million, every other exhausted search fewer; the repeats and primes, the
ceilings given the games before them, the one-thread gates (a wrap needs about 7 hours of one game on one thread at
these sizes) and the prover (which shares no code) are not exposed.  So 3440 is the one committed game the wrap cannot
be ruled out for; it is to be re-solved with the fixed build before 3441 is resumed on its ceiling.

