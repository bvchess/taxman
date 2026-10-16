# Conflicts of the matching bound (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

Measured with `examples/ladder.rs` (see DESIGN.md, "Performance") on 1 thread for 760,
880 and 936 and on 10 threads for 1000 and the hard games near it.  With the
matching bound alone, n=1000 was not proven in 4 minutes on 10 threads.

What the diagnostics showed first (n=760–1000, `--diag`): the matching bound
does nearly all the pruning (blameGame almost never prunes where it does not),
and at the nodes it fails to prune it is 25–45 above what was needed
(median); the heaviest matching there essentially never promotes a feasible
set; the tree peaks at 25–40 promotions deep.  Making the bound tighter was
the lever, not cheaper nodes.

* **Conflicts** (`Model::conflict_using`).  When the heaviest matching's
  candidates `Q*` cannot all be promoted, some frame is unwinnable with them;
  the moves a peel of it cannot drop (its *core*) are stuck however the order
  goes, and the members of `Q*` the core needs (promoted into the frame and
  core moves, or promoted out of it and factors of core moves) cannot be
  promoted together.  Minimized by peeling only the core, these conflicts are
  tiny: two members at the median and p75 at n=760, p50 2 / p90 5 at 996 (a
  quarter are single members: a candidate that cannot be promoted from that
  node's state, though it can from the root; see "Single-member conflicts
  belong to the node").  A subset of a promotable set is promotable, so
  every completion leaves out some member of a conflict — the basis of all
  that follows, and of `search::refine::Refiner`'s soundness proof.
* **Refinement** (`match_refine`).  Bound the branches of the conflict too
  (`k₁` out; `k₁` in and `k₂` out; …), a few levels deep.  Alone, on the
  Java's branching, it cut nodes 2–70x but cost more per node than it saved
  (n=880: 4.8M → 68k nodes at depth 7, 31 s → 21 s; at depth 1–3 slower than
  without).  Caching known conflicts along the path and branching disjointly
  made it pay at 760; with conflict branching it is worth one level (default
  1; 2–15% faster than 0 on the benchmarks, deeper is slower).
* **Conflict branching** (`branching`).  Branch on a member of the conflict
  instead of the largest candidate, so that neither the child nor the node
  going on can reproduce the matching: the decisive change.  `conflict-strong`
  (default) branches on the member whose removal lowers the matching bound
  most (one matching solve per member; the level for the chosen one is kept
  for the node, `MatchBound::keep_excluded`).  Promoting a member is not worth
  looking at: the heaviest matching promotes them all already; and a lookahead
  that also scores the child with the other member of a pair out did worse.
  `conflict-smallest` gives the smallest proof trees at 880/936 but finds
  good incumbents late; `conflict-largest` follows the dive and finds them
  early.  Branching on the members of every failing frame's conflict instead
  of the smallest conflict was worse (936: 754k → 1.0M nodes).
* **Exact matchings**.  A heaviest matching that can be promoted is the
  node's best completion: offered as incumbent, node done.
* **Propagation**.  A known conflict of which a child promotes all but one
  member rules that member out of the child's candidates.  Without it the
  child's matching bound equals the node's (its matching promotes the child's
  number already).  −17% nodes.
* **Starting incumbent**.  `dive` rounds the root's matching (drop the
  smallest member of a conflict until feasible) — weaker than greedy on most
  games; a one-step lookahead in the dive was no better.  A local search
  (`Neighbourhood::improve`: fill greedily, then drop one or two members and
  refill) reaches the optimum at the start for 945 and 988 and gets close for
  950/984/990 (20–40 ms).  Running it on every incumbent found during the
  search, and dives at search nodes, gave nothing measurable.
* **Cheaper nodes**: a per-frame cache of conflict answers (consecutive
  matchings change few frames; −5%); a radix heap in the Hungarian search
  (−16–19%); stopping the scan of frames at a single-member conflict.
  Choosing the frame by the smallest *core* and minimizing only it was much
  cheaper per node but its conflicts branch far worse (n=1000 unfinished in
  2 min).  Checking every candidate a promotion affects (a complete
  `can_promote` filter instead of the Java's partial one) cut nodes 25–35% but
  quadrupled the cost per node.  The preclusion table still pays on long
  searches (936: 15.7 s → 11.8 s) though it costs ~1.4 s of setup on one
  thread.  (No longer: see "Forced members and nogoods", below.)

Per change (search seconds and nodes; 1 thread for 760/880/936 seeded from
n−1, 10 threads for 1000; 10-thread times vary with when the optimum is
found):

| commit | change | 760 | 880 | 936 | 1000 (10 thr.) |
|---|---|---|---|---|---|
| f8d922d | conflicts, ladder, diagnostics (Java's branching) | 1.13 s 205k | 30.7 s 4.78M | 90.8 s 12.3M | open at 120 s |
| 6c4e6ee | branch on conflict (largest member) | 0.13 s 10.1k | 11.2 s 909k | 31.8 s 2.62M | 27.5 s |
| 264d1ab | cache of frames' conflicts | 0.12 s 10.1k | 10.5 s 909k | 29.5 s 2.62M | 30.7 s |
| d1c256c | strong branching | 0.12 s 7.2k | 3.8 s 243k | 17.7 s 909k | 22.8 s |
| 75155b4 | propagation into children | 0.12 s 6.5k | 3.6 s 202k | 16.8 s 759k | 18.7 s |
| 5ef83ea | keep the strong-branching level | 0.11 s 6.5k | 3.25 s 202k | 15.0 s 759k | 18.7 s |
| f6aef0f | local search on the start | 0.12 s 6.5k | 3.2 s 202k | 14.8 s 758k | 17.1 s |
| 1e7a3de | radix heap | 0.11 s 6.5k | 2.7 s 202k | 12.0 s 758k | 12.6 s |
| 38cf168 | refine one level | 0.09 s 4.0k | 2.6 s 142k | 11.1 s 500k | 11.1 s |

The ladder (10 threads, standalone from the frame builder, seeded from n−1):
before, 992 and 1000 were open at 100 s (992's incumbent not even optimal),
960 took 37 s and 800 13 s, and 945, 948, 950, 969, 976, 984, 987, 988, 990,
996, 999 were open at 20–30 s.  After, all of them complete: 760 0.03 s, 800
0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s, 960 1.6 s, 945–999 1.4–24 s except
992 (62 s, of which 42 s finding the optimum) and 996 (69 s, all proof).
The whole range 761–1000 through `taxman solve` (prime shortcut and
`reuse_prev` on; 81 games searched) takes about 5 minutes of game time on 10
threads, every score matching optimal.json; 1001–1100 about 6.5 minutes
(36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with
`reuse_prev` and `revise_freedoms`, as the Java's results).  Threads scale
linearly to the 8 performance cores; the 2 efficiency cores add little.

Left to try: making the per-node conflict work cheaper (a quarter of the
time: peeling each frame `Q*` touches), fewer matching solves per node, and
incumbents for games like 992 whose optimum is found late.
