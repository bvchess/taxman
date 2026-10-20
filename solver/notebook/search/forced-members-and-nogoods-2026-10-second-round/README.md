# Forced members and nogoods (2026-10, second round)

Part of the [lab notebook](../../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  The tree of a proof is mostly chains, and the matching bound's work at each node is repeated over and
over: how far do forced members, nogoods and cheaper bounds and children cut the cost of a search?  **Answer.**  Far:
on the fast set the six games take 0.23 to 3.9 s where they took 2.7 to 24.8 s (1184: 21.6 s to 1.34 s), the hard games
of 992 to 1216 fall from 51-196 s to 3-24 s on 8 threads, and `taxman solve 1001-1217` reproduces every score of
`results/strict.jsonl` in 164 s of game time instead of 1,244 s.  Nogoods did most of it: 936 went from 8.05 s (320k
nodes) to 1.09 s (50k).

## Method

The ladder on one thread (deterministic node counts), with the frontier's tactics (`revise_freedoms=off`), seeded from
n − 1 (880 and 936 from `optimal.json`, the rest from `results/strict.jsonl` with `--known`).  The fast set finishes
each game in under 25 s: 880, 936, 1050 (optimum found late), 1100, 1170 and 1184 (proof only).  The first rows ran
while a production run used 6 cores; the first row measured again on an idle machine came out the same to 1%.

At 1184 (`sample`) the time went to peeling for conflicts, 29% (nearly all of it minimizing them), matching repairs for
refinement 18% and for strong branching 16%, and the candidate filter's peels 16%.  The tree is mostly chains: at four
nodes in five the loop ends right after its first child, because removing the member that strong branching chose
already brings the bound below the node's need.  That member is *forced*: every completion that beats the incumbent
promotes it.  At 1184 more than one member was forced at 54% of the nodes.  The design is in
[solver/docs/design/search.md](../../../docs/design/search.md).

## Results

Each step, in the order made (the commits are below):

1. **Forced members** (tactic `fix_forced`, sound): the child promotes every forced member of the conflict, not only the
   chosen one.  Nodes −31% (880) to −60% (1170), time −21% to −36%.
2. **Reusing the child check:** a child keeps the verdict of its bound check and makes one candidate list for several
   promotions.  −3% to −23% time.
3. **Nogoods** (tactic `nogoods`, sound): a conflict found at a node is learned as a nogood of the root, in a table each
   searcher keeps, and a node's conflict is the smallest the table knows.  At 1184, 291k computed conflicts were only
   854 distinct.  Lookups beat computing, and the smallest known conflict is often smaller than one frame's
   minimization finds (936: 320k to 50k nodes).  Lookups alone gave most of it (880 to 39k nodes, 936 to 57k, 1184 to
   287k); propagation alone, with every conflict computed and made a nogood, cut nodes less (880 to 61k) and cost more
   than it saved.  The tree depends on the searcher's history; the answers do not.
4. **All members forced:** the node is done.  It is free once strong branching has the bounds, and matters only with
   `match_refine=0`, since refinement prunes those nodes first.
5. **Bounds without a member, from the potentials** (`MatchBound::bound_excluding`): with conflicts looked up, matching
   repairs were two thirds of the time, nearly all for the bound without one member.  Lowering that member's potential
   and its factor's gives an upper bound at once, never below the exact value; it shows 89% of the forced members forced
   at 1184, and the exact solves are left for the rest.  The same trees, 13-28% less time.
6. **Forced members of the whole matching,** found the same way: if the table knows a conflict all of whose members are
   forced, or the conflict found is such, the node is done; otherwise the child promotes the forced members and strong
   branching is not needed.  Nodes −8% (936) to −52% (1184), time −2% to −39%; the proof of 992 seeded with its optimum:
   2.89M to 2.11M nodes.
7. **Cheaper children:** the Java's promotability checks are left out of a child's candidate list, forced members are
   promoted together with each frame checked once, and `blameGame` is not computed (it never pruned where the matching
   bound did not).  Peeling had been the largest cost, 37% of the time at 1215.  −30% to −59% time, nodes within ±8%.
8. **Nogood signatures:** each nogood keeps a 256-bit signature, and a scan skips those not within the node's.  The
   scans had been 17% of the time at 1215.  −5% to −10%.
9. **No refinement where members are forced:** that node's single child refines instead.  Nodes +3% to +11%, time −2% to
   −8% (992 and 1215 seeded with their optimum: −3.5% and −6.5%).
10. **No preclusion table** (`preclusions` off by default): with nogoods it no longer pays for its setup.  992 and 1215
    seeded with their optimum take +5% and −4% without it (one thread), and 1101-1217 through `taxman solve` on 8
    threads spent 53 s of 131 s building tables (0.8 s per searched game, 4.4 s on one thread at 1215) and 72 s
    searching; without them it took 80 s in all.

Tried and not kept:

* **More forced members proved,** by a second pass of the potentials' bounds with the forced ones required: it pruned
  nothing more.
* **Fewer exact solves** (skipping strong branching's when the potentials found some member forced, or refinement's, or
  choosing the member to branch on by the potentials alone): each traded nodes for time at about even.
* **Rounding the heaviest matching into an incumbent at every node:** five times slower per node, and the optimum of
  1050 was found at the same step.
* **A large-neighbourhood search before the search** (free a window of the incumbent's members, keep the rest promoted,
  search that unit with a node limit): at 1025 it reached 25080 in 1.3 s (the search takes 11 s to), but the optimum,
  7 members out and 8 in across frames 1-4, was found only by the search, at about the same point (one thread: 160 s
  instead of 170 s).  The incumbents of 1025 and 1050 improve late because the search reaches the subtree that holds
  them late, not because a nearby set is missed.

One thread, fast set (search seconds and nodes; the preclusion table's setup, 1.3-3.9 s on one thread for these games,
is left out):

| step | 880 | 936 | 1050 | 1100 | 1170 | 1184 |
|---|---|---|---|---|---|---|
| before | 2.72 s 142k | 11.65 s 500k | 24.8 s 598k | 4.81 s 161k | 4.58 s 165k | 21.6 s 786k |
| 1 forced members | 2.14 s 98k | 8.61 s 320k | 18.2 s 342k | 3.17 s 74k | 3.09 s 66k | 14.7 s 338k |
| 2 reuse the child check | 1.97 s 98k | 8.05 s 320k | 16.8 s 340k | 2.65 s 74k | 2.35 s 66k | 11.9 s 338k |
| 3 nogoods | 0.66 s 34k | 1.09 s 50k | 11.8 s 318k | 2.12 s 75k | 1.18 s 46k | 6.41 s 276k |
| 5 bounds from the potentials | 0.52 s 34k | 0.85 s 50k | 8.72 s 318k | 1.65 s 75k | 0.91 s 46k | 4.87 s 276k |
| 6 forced members of the whole matching | 0.46 s 31k | 0.83 s 45k | 6.45 s 241k | 1.31 s 51k | 0.92 s 35k | 2.94 s 133k |
| 7 cheaper children | 0.29 s 30k | 0.53 s 47k | 4.68 s 264k | 0.73 s 51k | 0.40 s 35k | 1.28 s 126k |
| 8 nogood signatures | 0.26 s 30k | 0.49 s 47k | 4.43 s 264k | 0.68 s 51k | 0.38 s 35k | 1.21 s 126k |
| 9 no refinement where members are forced | 0.24 s 32k | 0.45 s 52k | 3.99 s 290k | 0.67 s 56k | 0.37 s 38k | 1.13 s 130k |
| 10 no preclusion table | 0.23 s 31k | 0.50 s 57k | 3.88 s 289k | 0.63 s 55k | 0.36 s 38k | 1.34 s 159k |

The hard games, standalone in the ladder on an idle machine (search seconds, and in parentheses when the final incumbent
was found; with several threads the times vary from run to run with when the optimum is found, so two runs are shown
for the last version):

| game | before, 8 threads | after, 8 threads | after, 2 threads |
|---|---|---|---|
| 1215 | 195.6 s (71.4) | 12.1 s (7.4), 9.8 s (2.6) | 35.7 s (13.8) |
| 1216 | 51.5 s (start) | 3.1 s, 3.3 s | 10.4 s |
| 1025 | 100.1 s (76.7) | 12.5 s (4.6), 23.9 s (15.0) | 93.5 s (70.0) |
| 992 | 71.4 s (56.8) | 11.8 s (11.4), 12.0 s (11.8) | 40.0 s (39.2) |
| 996 | 76.4 s (start) | 10.2 s, 10.0 s | 31.5 s |

`taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off --tactic revise_freedoms=off` (no unproven tactic)
reproduces every score of `results/strict.jsonl` in 164 s of game time; those results took 1,244 s on 10 threads.  Its
slowest games: 1025 21 s (110 s before), 1215 10 s (161 s), 1020 6 s (40 s).

## Decision

Steps 1 to 7 and 9 are kept, and step 10 turned the preclusion table off by default.  Two were replaced later: the
signatures of step 8 by exact lookups over compact member indices, and the preclusion table was removed (the
[register](../../register/nogoods-and-conflicts.md) has both).  What the round left to try:

* the incumbents of games like 992 and 1025, whose optimum the search reaches late (992 on 8 threads: at 11.8 s of
  12.0 s; seeded with its optimum it takes about 6 s);
* the matching repairs, then about 40% of the time (strong branching where nothing is forced, and refinement), for which
  see [faster nodes, same trees](../faster-nodes-same-trees-2026-10/README.md);
* the multi-thread search: each thread learns its own nogoods, and sharing them did not look worth it, since a proof on
  8 threads takes about as many nodes as on one.

## Commits

All on 2026-10-12, in order of the steps: `8686be0` (the baseline: optimal.json reproduced with no unproven tactic),
`53a442a` (1), `261b342` (2), `c33c3c2` (3), `667b24d` (5), `356ef92` (6), `fe73c1c` (7), `372c3f9` (8), `49517a1` (9),
`8bcfd36` (10).
