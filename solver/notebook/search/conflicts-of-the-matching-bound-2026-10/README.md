# Conflicts of the matching bound (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  With the matching bound alone, 1000 was not proven in 4 minutes on 10 threads.  How does the search prune
harder?  **Answer.**  By branching on a member of the conflict of the heaviest matching, so that neither the child nor
the node going on can reproduce the matching: 936 fell from 90.8 s (12.3M nodes) to 31.8 s (2.62M), and 1000 from
unfinished at 120 s to 27.5 s on 10 threads.  Strong branching on the conflict, propagation of known conflicts and one
level of refinement brought 936 to 11.1 s (500k nodes) and 1000 to 11.1 s.

## Method

Everything was measured with `examples/ladder.rs` ([performance.md](../../../docs/design/performance.md)) on 1 thread
for 760, 880 and 936, seeded from n − 1, and on 10 threads for 1000 and the hard games near it; 10-thread times vary
with when the optimum is found.  The diagnostics (`--diag`, n = 760-1000) showed first that the matching bound does
nearly all the pruning (blameGame almost never prunes where it does not); that at the nodes it fails to prune it is
25-45 above what was needed (median); that the heaviest matching there essentially never promotes a feasible set; and
that the tree peaks at 25-40 promotions deep.  Making the bound tighter was the lever, not cheaper nodes.

**Conflicts** (`FrameGraph::conflict_using`).  When the heaviest matching's candidates `Q*` cannot all be promoted, some
frame is unwinnable with them.  The moves a peel of it cannot drop (its residual, here called its *core*) are stuck
however the order goes, and the members of `Q*` the core needs (promoted into the frame and core moves, or promoted out
of it and factors of core moves) cannot be promoted together.  Minimized by peeling only the core, these conflicts are
tiny: two members at the median and p75 at n = 760, p50 2 and p90 5 at 996.  A quarter are single members: a candidate
that cannot be promoted from that node's state, though it can from the root ([single-member conflicts belong to the
node](../single-member-conflicts-belong-to-the-node-2026-10/README.md)).  A subset of a promotable set is promotable, so
every completion leaves out some member of a conflict: the basis of all that follows, and of `search::refine::Refiner`'s
soundness proof.

The changes tried were refinement (`match_refine`: bound the branches of the conflict too, `k₁` out; `k₁` in and `k₂`
out; … a few levels deep); conflict branching (`branching`); exact matchings (a heaviest matching that can be promoted
is the node's best completion, offered as incumbent); propagation (a known conflict of which a child promotes all but
one member rules that member out of the child's candidates, since otherwise the child's matching bound equals the
node's); the starting incumbent (`dive` rounds the root's matching by dropping the smallest member of a conflict until
feasible; the local search `Neighborhood::improve` fills greedily, then drops one or two members and refills); and
cheaper nodes.

## Results

Per change, search seconds and nodes (1 thread for 760, 880 and 936; 10 threads for 1000):

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

Kept, with the measure that decided it:

* *Conflict branching* (the decisive change), on a member of the conflict in place of the largest candidate.  The
  default `conflict-strong` branches on the member whose removal lowers the matching bound most, one matching solve per
  member, and keeps the level for the chosen one for the node (`MatchBound::keep_excluded`): 936 from 29.5 s to 17.7 s,
  2.62M to 909k nodes.
* *Propagation*: −17% nodes.
* *Refinement one level deep* (`match_refine` = 1): 2-15% faster than none on the benchmarks; deeper is slower.
* *The per-frame cache of conflict answers* (−5%) and *the radix heap* (−16-19%).
* *The local search on the starting incumbent*: it reaches the optimum at the start for 945 and 988 and gets close for
  950, 984 and 990 (20-40 ms).
* *`conflict-smallest` and `conflict-largest`*, as switches (removed since; see [the tactics
  review](../which-tactics-earn-their-keep-2026-10/README.md#the-alternatives-that-are-off)): smallest gives the
  smallest proof trees at 880 and 936 but finds good incumbents late; largest follows the dive and finds them early.

Not kept:

* *Refinement on the Java's branching*: it cut nodes 2-70 times (880: 4.8M to 68k nodes at depth 7) but cost more per
  node than it saved (31 s to 21 s, and slower than none at depth 1-3).  Caching known conflicts along the path and
  branching disjointly made it pay at 760.
* *Promoting a member of the conflict*, and a lookahead that also scores the child with the other member of a pair out:
  the heaviest matching promotes the members already, and the lookahead did worse.
* *Branching on the members of every failing frame's conflict* in place of the smallest conflict: 936 from 754k to 1.0M
  nodes.
* *Choosing the frame by the smallest core and minimizing only it*: nodes much cheaper, but its conflicts branch far
  worse (1000 unfinished in 2 minutes).
* *A complete `can_promote` filter of the candidates* in place of the Java's partial one: nodes −25-35%, but the cost
  per node ×4.
* *Local search on every incumbent found during the search, and dives at search nodes*: nothing measurable.  A one-step
  lookahead in the root dive was no better than the plain dive, which is itself weaker than greedy on most games.
* The preclusion table paid on long searches (936: 15.7 s to 11.8 s) at about 1.4 s of setup on one thread; it no longer
  does ([forced members and nogoods](../forced-members-and-nogoods-2026-10-second-round/README.md)).

**The ladder** (10 threads, standalone from the frame builder, seeded from n − 1).  Before, 992 and 1000 were open at
100 s (992's incumbent not even optimal), 960 took 37 s and 800 13 s, and 945, 948, 950, 969, 976, 984, 987, 988, 990,
996 and 999 were open at 20-30 s.  After, all of them complete: 760 0.03 s, 800 0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s,
960 1.6 s, 945-999 1.4-24 s except 992 (62 s, of which 42 s finding the optimum) and 996 (69 s, all proof).  The whole
range 761-1000 through `taxman solve` (prime shortcut and `reuse_prev` on; 81 games searched) takes about 5 minutes of
game time on 10 threads, every score matching `optimal.json`; 1001-1100 about 6.5 minutes (36 searched, slowest 1025 at
100 s) and 1101-1200 about 2.5 minutes (with `reuse_prev` and `revise_freedoms`, as the Java's results).  Threads scale
linearly to the 8 performance cores; the 2 efficiency cores add little.

## Decision

The conflicts of the heaviest matching are the branching and the pruning of the search, with the changes marked kept
above.  Left to try: making the per-node conflict work cheaper (a quarter of the time: peeling each frame `Q*` touches),
fewer matching solves per node, and incumbents for games like 992 whose optimum is found late.

## Files

The folder holds this README.md only.

## Commits

* f8d922d: conflicts of the matching bound, the ladder harness and the diagnostics.
* 6c4e6ee: branch on the conflicts of the heaviest matching by default.
* 264d1ab: cheaper conflicts (the per-frame cache).
* d1c256c: strong branching on conflicts by default.
* 75155b4: propagate known conflicts into children.
* 5ef83ea: keep the matching that strong branching solved.
* f6aef0f: local search on the starting incumbent.
* 1e7a3de: the radix heap in the matching bound.
* 38cf168: refine the matching bound one level by default.

All on 2026-10-12.
