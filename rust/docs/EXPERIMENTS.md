# Experiments: the lab notebook of the Rust solver

What was tried, in what order, and what it was worth.  [DESIGN.md](../DESIGN.md) says how the solver works now;
this page is the record of how it got there: every round of improvements with the measurements that decided it, and
the ideas that were measured and not kept, with their numbers.  Entries are dated by month (the whole record so far
is from 2026-10 unless a section says otherwise) and name the commits that made each step, so `git show COMMIT`
finds the code.  Statements are true of the code at the time of the entry; where a later entry changed the picture
it says so, and DESIGN.md is the authority on the present.

Terms (conflict, nogood, forced member, step, `W`, ...) are defined in the [glossary of the README](../README.md#7-glossary)
and in DESIGN.md; the measurements were made with the ladder, the probe and `taxman bench`, which DESIGN.md
("Performance") describes.

## Index of rounds

| round | question | outcome | commits |
|---|---|---|---|
| [The port](#the-port-and-its-first-improvements) | does a fresh implementation of the Java's algorithm reproduce its answers? | yes, and the Java's upper bound had a defect (the runner-up), fixed behind `legacy_bound` | before 2026-10 |
| [Matching bound](#improvements-after-parity) | can a relaxation to bipartite matching bound the nodes better than `blameGame`? | kept; the default bound | before 2026-10 |
| [Seed and cap](#improvements-after-parity) | start from the previous game and stop at `n + opt(n − 1)` | kept (`seed_incumbent`, `prev_cap`) | before 2026-10 |
| [Conflicts of the matching bound](#conflicts-of-the-matching-bound-2026-10) | does the conflict of the heaviest matching make a better branching and a tighter bound? | kept: conflict branching, strong branching, refinement; 1000 from open at 120 s to 11 s | f8d922d to 38cf168 |
| [Forced members and nogoods](#forced-members-and-nogoods-2026-10-second-round) | can the search promote what every completion promotes, and learn conflicts as facts of the root? | kept: `fix_forced`, `nogoods`, bounds from the potentials, cheaper children; 1184 from 21.6 s to 1.3 s | 53a442a to 8bcfd36 |
| [Carrying nogoods over](#carrying-nogoods-over-2026-10-measured-not-kept) | start a game's table with the nogoods of the last | not kept: a larger table costs every node | f41fff8 (taken out again) |
| [Forgetting nogoods](#forgetting-nogoods-2026-10) | clear each searcher's table every N steps | kept (`forget_nogoods`, N = 50,000); 15–40% less time on hard games | 576e2e2 |
| [Single-member conflicts](#single-member-conflicts-belong-to-the-node-2026-10) | are single-member conflicts root facts that can be filtered once? | no: nothing to filter | (a prototype, not kept) |
| [Faster nodes, same trees](#faster-nodes-same-trees-2026-10) | make the inner loop cheaper without changing any tree | kept: 30% fewer cycles at the 1215 proof | e3b8ddc to 4b6ef38 |
| [Engine](#the-engine) | scaling to many cores, and checkpoints at any interval | kept: split requests per idle worker, pauses that cannot starve the workers | 2026-10 |
| [Benchmarks](#benchmarks-the-probe-pgo-and-scaling) | what the bench set, the probe, PGO and the thread scaling showed | numbers | e3b8ddc and after |

## Tried and not kept

Everything in this table is measured below; the number is the one that decided it.

| idea | round | result |
|---|---|---|
| Take the minimum of `blameGame` and the matching bound | matching bound | `blameGame` pruned nowhere the matching bound did not (0 at every check), so it is not computed with the matching bound |
| Refinement on the Java's branching | conflicts | nodes 2–70x fewer but each costs more than it saves: 880 from 4.8M to 68k nodes at depth 7 but only 31 s to 21 s, and slower than none at depth 1–3 |
| Promote a member of the conflict (instead of looking at what its removal does) | conflicts | the heaviest matching promotes them all already; a lookahead that scores the child with the other member of a pair out did worse |
| Branch on the members of every failing frame's conflict | conflicts | 936: 754k to 1.0M nodes |
| Choose the frame by the smallest core and minimize only it | conflicts | much cheaper per node, but its conflicts branch far worse (1000 unfinished in 2 min) |
| A complete `can_promote` filter of the candidates (instead of the Java's partial one) | conflicts | nodes −25–35%, cost per node ×4 |
| Local search on every incumbent found, and dives at search nodes | conflicts | nothing measurable |
| Prove more members forced with the forced ones required | forced members | pruned nothing more |
| Skip the exact solves of strong branching when the potentials found some member forced, or choose by the potentials alone | forced members | each traded nodes for time at about even |
| Round the heaviest matching into an incumbent at every node | forced members | 5x slower per node; the optimum of 1050 was found at the same step |
| A large-neighbourhood search before the search | forced members | at 1025 25080 in 1.3 s, but the optimum only from the search, at the same point |
| Carry the nogoods of game `n − 1` into game `n`, or store them in checkpoints | carrying nogoods | 1227 (the ceiling): −10% nodes, +3% time; the strict chain 1201–1231 617 s to 941 s |
| Forget nogoods by table size (64, 128, 256) | forgetting | 15–22% less time on the hard four, but the smaller games slower (936: 0.51 s against 0.85/0.61/0.53 s) |
| Keep the nogoods used since the last clearing | forgetting | every 100,000 steps 15%, 500,000 6%: the ones kept cost lookups as much and help no more |
| A filter of candidates that cannot be promoted from the root | single-member conflicts | dropped 0 of 225, 273, 287 and 302 candidates; identical node counts |
| An undo trail for the matching levels | faster nodes | windows +0.8% and +0.9% cycles, instructions +4% |
| The peel in frame-local form | faster nodes | bit-parallel peel +8% and +16% on forced sets; counting peel +0.5% and +0.8% |
| Caching frame checks | faster nodes | 14–17% of peels repeat one exactly, but only 5% one of the last eight |
| Unchecked indexing in the Hungarian search | faster nodes | −4% instructions, −0.7% cycles; not worth `unsafe` |
| Set widths of 12 and 48 words | faster nodes | 3–4% at 512–767 (solved) and some at 2048–3071, for 12 s more of every build |

## The port and its first improvements

The Rust solver reimplements the algorithm of the Java solver rather than translating it (DESIGN.md, "The Java
solver and this one").  Its setup had to reproduce the Java's values exactly (the golden data in
`testdata/golden/` is the check) and every final score had to equal optimal.json; the search itself was
designed for speed from scratch.  One defect of the Java was found and fixed on the way:

* **Upper bound runner-up (a defect of the Java's bound, fixed in the port; `legacy_bound` brings it back)** (`UpperBoundEstimator.getRunnerUp`): when
  `largestThatFitInFrames` skips candidates because frame 1 or the last frame
  is at its cap, the Java replaces the failing number with the next candidate
  below the smallest bid member.  A skipped candidate from the failing
  number's frame can be larger, so the Java bound can be too low (exhaustive
  checks find it below the best completion, e.g. at the root of n=114: 358
  against 360).  Sound version: the heaviest set within the caps that leaves
  out some bid member at or above the failure, which by matroid exchange is
  the bid with one member swapped for the best outside candidate the caps
  allow.  (Simply taking the largest candidate not in the bid is also sound
  but swaps skipped candidates of a full frame in for members of other frames;
  it made n=700 take over 10 minutes instead of 18 s.)  (Re-solving n=2..760
  in Java with the simple fix changed no answers; n=740 timed out.)
* `Memo.test` computes its value twice — moot, `Memo` is not ported.
* `TxSet.MAX_VALUE = 1023` — gone; set width is chosen from n.

## Improvements after parity

* **Matching bound** (sound).  Relax winnability (peel) to plain
  matchability across the whole frame chain: maximize the promoted sum
  subject to every original move and every promoted number getting a
  distinct maximal factor in its frame, and no promoted number being used
  as a factor.  Edges v → v/p change Ω(v) by one, so this is a max-weight
  *bipartite* matching (min-cost flow), exactly solvable.  Measured at the
  root of all 94 golden games that need search (scratch study): mean gap to
  the optimum 31 vs 76 for the Java bound; on the bench games 107–292 vs
  363–769.  The cardinality cap never binds.  To be used incrementally
  (re-optimized along the DFS path), at first combined with the Java bound
  by taking the minimum.  (That was dropped later: `blameGame` never pruned
  where the matching bound did not, so the shipped bound is the matching
  bound alone.)
  *Implemented* in module `matchbound` (used by `search`, tactic
  `match_bound`): an optimal matching and its dual potentials per level of the DFS path,
  repaired by Hungarian searches after each change; 0.7–2.4 µs per
  evaluation in a DFS at n=480–760, against 30–170 µs from scratch.  A test
  DFS that starts with the optimum as incumbent and evaluates the matching
  bound only where blameGame fails to prune visits 85, 34, 1.5, 2.7, 1.7,
  32 and 35 times fewer nodes at n=480, 500, 600, 640, 660, 700 and 720
  (taking 57, 26, 1.5, 2.5, 1.6, 24 and 28 times less time), and at n=760
  finishes in 90k nodes where blameGame alone passes 3M.  The cap never
  bound there either.
* Seeding the incumbent from n−1 and stopping at n + opt(n−1) (tactics
  `seed_incumbent`, `prev_cap`).

## Conflicts of the matching bound (2026-10)

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

## Forced members and nogoods (2026-10, second round)

Measured with the ladder on one thread (deterministic node counts) with the
frontier's tactics (`revise_freedoms=off`), seeded from n−1 (from
`results/strict.jsonl` past 1000, `--known`), on a fast set that each finish
in under 25 s: 880, 936, 1050 (optimum found late), 1100, 1170, 1184 (proof
only).  (The first rows were measured while a production run used 6 cores,
the rest on an idle machine; the first row measured again idle came out the
same to 1%.)  Where the time went at 1184 (`sample`): peeling for conflicts 29%
(nearly all of it minimizing them), matching repairs for refinement 18% and
for strong branching 16%, the candidate filter's peels 16%.

The tree is mostly chains: at four nodes in five the node's loop ends right
after its first child, because removing the member that strong branching
chose already brings the bound below what the node needs.  That member is
*forced*: every completion that beats the incumbent promotes it.  Strong
branching computes the bound without each member of the conflict, and at
1184 more than one member was forced at 54% of the nodes (two at 16%, three
at 10%, four or more at 28%).  Before, the child promoted one of them and
found the others again, one level each, further down.

* **Forced members** (tactic `fix_forced`, sound).  The child promotes every
  forced member of the conflict, not only the chosen one: the child's
  completions are some of the node's, so those that beat the incumbent
  promote them all, and if one cannot be promoted any more (or the child's
  candidate filter dropped it) the child has no such completion at all.
  Nodes −31% (880) to −60% (1170), time −21% to −36%.
* **Reusing the child check.**  A child's bound is checked (with refinement
  and its conflict) when it is made; its loop's first step repeated that with
  the same need for the same verdict.  It now keeps the verdict.  And a child
  that promotes several numbers makes its candidate list once, after all of
  them.  −3% to −23% time.
* **Nogoods** (tactic `nogoods`, sound).  A conflict `K` found at a node with
  promotions `P` is relative to `P`; a minimal subset of `P ∪ K` that keeps
  a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from
  the root, so at every node anywhere in the tree (any thread, any path, and
  after a resume) the nogood minus that node's promotions is a conflict of
  the node (a subset of a promotable set is promotable; the proof is in
  `model::Nogoods`).  Measured first: at 1184, 291k conflicts were computed
  but they were only 854 distinct nogoods — the search found the same few
  hundred over and over.  So each searcher keeps a table of them (at most
  65536, forgotten all at once when full; about a thousand are learned near
  n = 1200), and a node's conflict is the smallest the table knows within
  `P ∪ Q*` (of those as small, the one with the largest member); only when it knows none
  is one computed, and then learned.  Nogoods also rule candidates out of a
  child (a nogood all of whose other members the child promotes, indexed by
  member), beyond the conflicts known along the path.  Looking conflicts up
  rather than computing them is cheaper, and also better for the tree: the
  smallest of the many conflicts known is often smaller than the one the
  minimization of a single frame finds, and branching on it and on the
  child's propagation cut nodes (936: 320k → 50k).  The tree now depends on
  what the searcher has learned, so on its history (threads, interruptions),
  not only on the node; answers do not.  Alone, lookups gave most of it
  (880 → 39k nodes, 936 → 57k, 1184 → 287k); propagation alone, with every
  conflict computed and made a nogood, cut nodes less (880 → 61k) and cost
  more than it saved.
* **All members forced.**  When every member of the conflict is forced, no
  completion beats the incumbent: the node is done (free once strong
  branching has computed the bounds; only matters with `match_refine=0`,
  since refinement prunes those nodes first).
* **Bounds without a member from the potentials** (`MatchBound::bound_excluding`).
  With the conflicts looked up, matching repairs were two thirds of the
  time, nearly all of them for the bound without one member of a conflict
  (strong branching, and refinement's branches).  The level's potentials are
  a dual solution whose sum is its bound; without the member as a candidate
  they stay feasible, and lowering the member's potential and that of the
  factor it paid with as far as their remaining edges allow gives an upper
  bound at once: `value − x + y′ₓ + y′_w`.  At 1184 it shows 89% of the forced
  members forced (and is never below the exact value, which the matching
  tests check).  Strong branching (with `fix_forced`) and refinement solve
  only for the members it does not settle; when some member is forced, the
  node's loop ends after the child that promotes them (it covers every
  completion that can beat the incumbent), so no level is kept for it.  The
  same trees, 13–28% less time.
* **Forced members of the whole matching.**  The bound from the potentials
  is cheap enough to compute for every member of the heaviest matching, not
  only for the conflict's: at each check that refines (or wants a conflict)
  the forced members of `Q*` are found that way.  If the nogood table knows
  a conflict all of whose members are forced, or the conflict found is, the
  node is done (diagnostics had shown that refining by the fresh minimal
  conflict instead of the table's would prune 16% of the nodes left open:
  other conflicts matter).  Otherwise, if any are forced, the child promotes
  them all and strong branching is not needed; only when none is does strong
  branching solve for its conflict's members.  The table is scanned once
  for both (the smallest conflict, and any conflict all forced), its
  nogoods' members ordered with the conflict they came from first, as those
  are the likeliest to be missing at another node.  Nodes −8% (936) to −52%
  (1184), time −2% to −39%; 992 seeded with its optimum (the proof alone,
  one thread): 2.89M → 2.11M nodes.

  Tried and not kept: proving more members forced with the forced ones
  required (a second pass of the bounds from the potentials on a level with
  them promoted) pruned nothing more; skipping the exact solves of strong
  branching when the bounds from the potentials found some member forced
  (instead of whenever they found any candidate forced), or of the
  refinement's branches, or choosing the member to branch on by those bounds
  alone, each traded nodes for time at about even.
* **Cheaper children.**  Peeling had become the largest cost (37% of the
  time at 1215: promotions and the candidate filter's checks).  With the
  matching bound, a child's candidate list leaves out the Java's
  promotability checks: a candidate that cannot be promoted is found by the
  conflicts and nogoods anyway, and nodes stayed within ±8%.  The forced
  members are promoted together, each frame checked once
  (`Model::promote_all_using`) instead of two frames per member (together
  they fit the frames' freedoms exactly when no frame they touch ends below
  zero).  And blameGame is not computed with the matching bound: it never
  pruned where the matching bound did not (diagnostics: 0 at every check).
  −30% to −59% time.
* **Nogood signatures.**  The table's scans were then 17% of the time at
  1215 (a few hundred nogoods, a lookup per check).  Each nogood keeps a
  256-bit signature (bit `x mod 256` per member), and a scan skips those
  whose signature is not within the node's; the conflicts chosen are the
  same as before (the smallest, then the one with the largest member).
  −5% to −10%.
* **No preclusion table** (`preclusions` now off by default).  With the
  nogoods it no longer pays for its setup: the searches of 992 and 1215
  seeded with their optimum take +5% and −4% without it (one thread), and
  1101–1217 through `taxman solve` on 8 threads spent 53 s of its 131 s
  building tables (0.8 s per searched game, 4.4 s on one thread at 1215) and
  72 s searching; without them 80 s in all, the search 74 s.
* **No refinement where members are forced.**  A node with forced members
  has a single child, which promotes them; refining the node's conflict is
  left to that child.  Nodes +3% to +11%, time −2% to −8% (992 and 1215
  seeded with their optimum: −3.5% and −6.5%).

Tried and not kept in this round:

* Rounding the heaviest matching into an incumbent at every node (drop the
  smallest member of a conflict until the rest can be promoted, then fill
  greedily): five times slower per node, and the optimum of 1050 was found
  at the same step.
* A large-neighbourhood search before the search: free a window of the
  incumbent's members (by frame pair, or 15–40 consecutive members), keep
  the rest promoted, and search that unit with a node limit.  At 1025 it
  reached 25080 in 1.3 s (the search takes 11 s to), but the optimum, 7
  members out and 8 in from there across frames 1–4, was found only by the
  search, at about the same point (one thread: 160 s instead of 170 s).
  The incumbents of 1025 and 1050 improve late because the search reaches
  the subtree that holds them late, not because a nearby set is missed.

One thread, fast set (search seconds and nodes; 880 and 936 seeded from
optimal.json, the rest from `results/strict.jsonl`):

| commit | change | 880 | 936 | 1050 | 1100 | 1170 | 1184 |
|---|---|---|---|---|---|---|---|
| 8686be0 | before | 2.72 s 142k | 11.65 s 500k | 24.8 s 598k | 4.81 s 161k | 4.58 s 165k | 21.6 s 786k |
| 53a442a | forced members | 2.14 s 98k | 8.61 s 320k | 18.2 s 342k | 3.17 s 74k | 3.09 s 66k | 14.7 s 338k |
| 261b342 | reuse the child check | 1.97 s 98k | 8.05 s 320k | 16.8 s 340k | 2.65 s 74k | 2.35 s 66k | 11.9 s 338k |
| c33c3c2 | nogoods | 0.66 s 34k | 1.09 s 50k | 11.8 s 318k | 2.12 s 75k | 1.18 s 46k | 6.41 s 276k |
| 667b24d | bounds from the potentials | 0.52 s 34k | 0.85 s 50k | 8.72 s 318k | 1.65 s 75k | 0.91 s 46k | 4.87 s 276k |
| 356ef92 | forced members of the whole matching | 0.46 s 31k | 0.83 s 45k | 6.45 s 241k | 1.31 s 51k | 0.92 s 35k | 2.94 s 133k |
| fe73c1c | cheaper children | 0.29 s 30k | 0.53 s 47k | 4.68 s 264k | 0.73 s 51k | 0.40 s 35k | 1.28 s 126k |
| 372c3f9 | nogood signatures | 0.26 s 30k | 0.49 s 47k | 4.43 s 264k | 0.68 s 51k | 0.38 s 35k | 1.21 s 126k |
| 49517a1 | no refinement where members are forced | 0.24 s 32k | 0.45 s 52k | 3.99 s 290k | 0.67 s 56k | 0.37 s 38k | 1.13 s 130k |
| 8bcfd36 | no preclusion table | 0.23 s 31k | 0.50 s 57k | 3.88 s 289k | 0.63 s 55k | 0.36 s 38k | 1.34 s 159k |

(Setup is left out: the preclusion table took 1.3–3.9 s of it on one thread
for these games, now nothing.)

The hard games, standalone in the ladder as above, on an idle machine (search
seconds; with several threads the times vary from run to run with when the
optimum is found, so two runs are shown for the last version; in
parentheses, when the final incumbent was found):

| game | 8686be0, 8 threads | 8bcfd36, 8 threads | 8bcfd36, 2 threads |
|---|---|---|---|
| 1215 | 195.6 s (71.4) | 12.1 s (7.4), 9.8 s (2.6) | 35.7 s (13.8) |
| 1216 | 51.5 s (start) | 3.1 s, 3.3 s | 10.4 s |
| 1025 | 100.1 s (76.7) | 12.5 s (4.6), 23.9 s (15.0) | 93.5 s (70.0) |
| 992 | 71.4 s (56.8) | 11.8 s (11.4), 12.0 s (11.8) | 40.0 s (39.2) |
| 996 | 76.4 s (start) | 10.2 s, 10.0 s | 31.5 s |

`taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off --tactic
revise_freedoms=off` (no unproven tactic) reproduces every score of
`results/strict.jsonl` in 164 s of game time; those results took 1244 s on
10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s),
1020 6 s (40 s).

Left to try: the incumbents of games like 992 and 1025, whose optimum the
search reaches late (992 on 8 threads: at 11.8 s of 12.0 s; seeded with its
optimum it takes about 6 s); the matching repairs, then about 40% of the
time (strong branching where nothing is forced, and refinement; see "Faster
nodes, same trees" for later); and the multi-thread search: each thread
learns its own nogoods (sharing them did not look worth it: a proof on 8
threads takes about as many nodes as on one).

## Carrying nogoods over (2026-10): measured, not kept

A nogood is a fact about the root of a game's frame model, and consecutive
games have nearly the same model, so the nogoods a search ends with could
start the tables of the next game's search, each checked at the new root
first: a set that cannot be promoted from the root (an exact test, frame by
frame) is a nogood of it whatever it came from, and `conflict_using` makes it
minimal again.  For the same reason a checkpoint could store the tables and a
resumed search start with them.  A prototype measured both (commit f41fff8:
`search::valid_nogoods`, the solver's tactic `carry_nogoods`, `ladder
--carry`, `examples/resume_carry.rs`); neither pays, and both were taken out
again.

* Nearly every nogood survives into the next game: 1374 of 1374 from 1024 at
  1025, 482 of 494 from 1219 at 1220, 1216 of 1251 from 1229 at 1230.  But
  game n − 1 often does not search: a prime takes the shortcut, and 1214,
  1224 and 1226 reach the cap with their seed (no nodes), so in a range the
  nogoods come from the last game searched (1227's from 1225, through 1226).
* The table never stops growing, but learning costs little.  On one thread,
  half of what a hard game learns is learned in the first 3–27% of its search,
  90% by 18–64%, and the last nogood in its last 1–17% (1024, 1025, 1215,
  1219, 1220, 1225, and 1230 stopped at 560 s); each is one conflict computed
  where the table knew none, a few thousand in a search of millions of nodes.
  So what a preloaded table could save is not the learning but the search's
  knowing smaller conflicts sooner, which changes the tree.
* It changes, but not for the better (one thread, strict tactics, seeded from
  n − 1 and capped as the solver does; search seconds and nodes):

  | game | tables start empty | with the nogoods of |
  |---|---|---|
  | 1025 | 158 s 8.50M | 1024: 136 s 6.30M |
  | 1025, proof only (seeded with its optimum) | 99 s 5.25M | 1024: 104 s 4.97M |
  | 1220 | 186 s 13.72M | 1219: 232 s 16.47M |
  | 1220, proof only | 177 s 13.51M | 1219: 240 s 17.81M |
  | 1227 | 260 s 17.42M | 1225: 267 s 15.72M |
  | 1230 | open at 560 s (best 36570 at 118 s) | 1229: open at 560 s (36570 at 186 s) |
  | 1025, 4 threads | 35.2 s 8.51M | 1024: 38.4 s 6.82M |
  | 1220, 4 threads | 54.0 s 17.16M | 1219: 68.0 s 15.91M |

  1227 is the ceiling: 1225–1229 have the same frames and candidates (the
  same search, 17,417,736 nodes on one thread), so 1227 started with all that
  1225 learned is the best a carried table can be, and it takes 10% fewer
  nodes and 3% more time.  The chain `taxman solve 1201-1231 --threads 4
  --tactic reuse_prev=off --tactic revise_freedoms=off` from 1200 in
  `results/strict.jsonl` took 617 s (154.7M nodes) without and 941 s (174.7M)
  with it, every searched game slower (1230: 305 s against 445 s); there the
  table grows from game to game.
* Why: a larger table costs every node.  A lookup scans every nogood's
  signature and the nogoods of each new promotion rule candidates out, so
  nodes are slower the more the table holds (1227: 59k nodes/s from a full
  table against 67k/s from an empty one; on 4 threads, whose tables start
  with the union of all threads' nogoods, 177k/s against 242k/s at 1025).
* Keeping the tables over a resume: a search stopped every K nodes and
  resumed from its checkpoint, with the tables as they were when it stopped
  (in memory, what a stored table would give), reproduces the uninterrupted
  search; resumed with empty tables it is a different tree, and faster (one
  thread):

  | game, stopped every | uninterrupted | resumed with empty tables | with the tables kept |
  |---|---|---|---|
  | 1024, 250k nodes | 41 s 2.28M | 34 s 2.32M | 42 s 2.26M |
  | 1215, 500k | 66 s 5.49M | 60 s 6.22M | 69 s 5.47M |
  | 1220, 1.5M | 186 s 13.72M | 143 s 11.91M | 191 s 13.48M |

  So a resume loses nothing by starting empty, and checkpoints stay as they
  were.  That old nogoods cost more than they help is taken up in
  "Forgetting nogoods", below.

Noticed on the way: many games of a range have exactly the frames and
candidates of an earlier one (in 1201–1231: 1202–1203 those of 1201, 1205 of
1204, 1207 of 1206, 1210–1211 of 1209, 1213–1214 of 1212, 1217 of 1216, 1219
of 1218, 1222–1223 of 1221, 1226–1229 of 1225, 1231 of 1230), so the same best
promotion set, which a search of one of them has already established.  Only
the cap differs: where it is higher than the optimum, the search is repeated
in full.  In the strict chain above that is 1203, 1205, 1207, 1211, 1219 and
1227: 90 s of the 617 s.

## Forgetting nogoods (2026-10)

The measurements of carrying nogoods over pointed the other way: a search
resumed with empty tables was faster than one that kept them.  So each
searcher now clears its table after each 50,000 of its own steps (tactic
`forget_nogoods`, `search::NOGOOD_PERIOD`; sound, since the table is a cache
and every search starts with an empty one anyway; deterministic on one
thread).

The interval, on one thread (search time of 1024, 1025, 1215 and 1220 added
up, against never forgetting run alongside): every 10,000 steps 22% less
time, 25,000 22%, 50,000 26%, 100,000 25%, 250,000 21%, 1,000,000 15%.  The
node counts move both ways (1025 +7% to +28%, 1220 −11% to −28%); what the
interval buys is cheaper nodes, since a lookup scans the whole table (1215:
110k steps/s forgetting every 10,000 steps, 83k/s never).  A game of fewer
steps than the interval is unchanged.  Tried and not kept:

* Forgetting by size, clearing the table whenever it holds 64, 128 or 256
  nogoods: 15%, 22% and 17% less time on those four, but the smaller games
  slower, since they clear while still learning (936: 0.51 s against
  0.85/0.61/0.53 s; 1050: 3.96 s against 5.19/4.49/4.33 s).
* Keeping the nogoods used since the last clearing (returned by a lookup, or
  ruling a candidate out of a child): every 100,000 steps 15% less time,
  every 500,000 6%.  The ones kept cost lookups as much as the others and help
  no more.

Never forgetting against forgetting every 50,000 steps (search seconds and
nodes; strict tactics, seeded from n − 1 and capped as the solver does; the
fast set 880–1184 is unchanged to ±4%):

| game | one thread, never | one thread, forgetting | 4 threads, never (two runs) | 4 threads, forgetting |
|---|---|---|---|---|
| 992 | 73.3 s 4.00M | 52.6 s 4.20M | 19.5 s, 16.9 s | 18.0 s, 16.5 s |
| 996 | 69.0 s 4.17M | 61.9 s 4.78M | | |
| 1024 | 43.0 s 2.28M | 34.3 s 2.38M | | |
| 1025 | 168.5 s 8.50M | 140.5 s 10.11M | 35.3 s, 35.3 s | 32.1 s, 32.3 s |
| 1215 | 69.6 s 5.49M | 50.6 s 5.16M | 21.8 s, 21.0 s | 15.3 s, 16.1 s |
| 1216 | 22.4 s 2.10M | 21.1 s 2.22M | | |
| 1220 | 192.2 s 13.72M | 125.4 s 10.51M | 60.6 s, 55.1 s | 38.8 s, 40.5 s |
| 1225 | 262.4 s 17.42M | 154.2 s 12.07M | 54.7 s, 55.2 s | 31.9 s, 33.4 s |

The strict chain 1201–1231 on 4 threads (as above) takes 377 s instead of
571 s, with every score the same (1230: 158 s instead of 275 s; 1225 33 s
instead of 55 s; 1220 39 s instead of 62 s).

## Single-member conflicts belong to the node (2026-10)

Many of the conflicts the search computes have one member (`--diag`'s minimal
conflict sizes: the 25th percentile is 1 at 936, the 10th at 1184).  They
looked like candidates that the Java's filter lets through and that cannot be
promoted from the root at all, which every searcher, work unit and resume
would find again; testing each candidate once from the root and dropping the
failures would then be free.  They are not root facts:

* **The frame builder's candidates all pass that test.**  A hive's promotion
  candidates are its factors that can leave it (the remaining sources can
  still be matched without them) and that some hive can take on as a source,
  which comes to what `Model::can_promote` checks at the root (the home frame
  without the factor, the frame below with it as a move).  For every game from
  2 to 1230 none of the candidates fails that check (a test), so a filter at
  the root drops nothing.  A version built as a tactic (the checkpoint header
  recording the candidates it leaves, older checkpoints refused as a different
  candidate set) searched 936, 1100, 1184 and 1215 on one thread in exactly the
  same nodes as without it (58,903, 56,399, 167,960 and 5,161,817), with 0 of
  225, 273, 287 and 302 candidates dropped; it was not kept.
* **The conflicts are relative to the node.**  A nogood of one member would be
  a candidate that cannot be promoted from the root, so the table never holds
  one.  A single-member conflict is a candidate that cannot be promoted once the
  node's promotions are made (its frame may have used it up in a forced pair,
  say); in another branch, with other promotions, it is a nogood of two or more
  members.  Looking it up is cheap, and the node's loop then goes on without
  the candidate.
* **Little is left to save.**  The promotions that fail are 2,282 of 58,903
  steps at 936 (3.9%) and 698 of 167,960 at 1184 (0.4%) (`--diag`, one thread),
  which bounds what a search that never tried a dead candidate could gain.

## Faster nodes, same trees (2026-10)

The inner loop, made faster without changing any answer or tree: every
change keeps the node counts of every game and the fingerprints of the
probe (`examples/perf.rs`, see DESIGN.md, "Performance"), and was measured in cycles and
instructions retired of the process (other load moves wall times by up to
3×), on one thread, plain release build.  Before, at the 1215 proof: the
Hungarian repairs about 50% of the time, the peels 19%, the bounds from the
potentials 5%, the nogood table 8%, level copies 4%.

| commit | change | 1215 window k cycles/step | 936 G cycles | 1184 | 1215 proof |
|---|---|---|---|---|---|
| e3b8ddc | before | 22.1 | 1.59 | 4.33 | 151.3 |
| b4cb2b6 | set widths of 20, 24 and 28 words (see "The engine", below) | 20.4 | 1.59 | 4.05 | 142.3 |
| ab76772 | radix heap's bucket 0 a bitset, the search's per-vertex fields in one slot, a level settled only when something changed | 18.8 | 1.47 | 3.65 | 129.3 |
| 130e4ec | `MAX_FRAMES` 12 → 8 (every board up to 4095 has at most 8) | 18.7 | 1.46 | 3.58 | 125.9 |
| 9bfb4d6 | nogood lookups exact over compact member indices, not signatures and member loops | 18.2 | 1.43 | 3.52 | 123.2 |
| 319753b | no peel for a promotion whose new move has a factor no other move uses (a sixth of them) | 18.0 | 1.41 | 3.53 | 122.5 |
| ad1237c | nogood rule-outs over the compact sets too | 17.8 | 1.35 | 3.49 | 118.9 |
| 4b6ef38 | the radix heap's push and pop and `add_inner` inlined (they were calls, 300 a step) | 16.1 | 1.23 | 3.16 | 106.2 |

In all 30% fewer cycles (1.42×) and 28% fewer instructions on the proof of
1215 (33 s instead of 47 s), 27% at 1184, 23% at 936; the probe's windows
27% (1215) and 26% (936).  Profile-guided optimization, 12% on the code
before, now adds 0–2% (see DESIGN.md, "Performance").  The strict chain 1..1231 on 8
threads takes 279 s of game time (1201–1231: 145 s), every score as
committed.  Where the time goes now (1215): the Hungarian repairs 46%, the
peels 24%, the bounds from the potentials 7%, the node's own work 7%,
level copies 4%, nogood lookups 3%.

Measured and not kept (cycles, one thread):

* An undo trail for the matching levels instead of a copy per push: pushes
  cost little, but logging every change of a repair costs more (windows
  +0.8% and +0.9%, instructions +4%).
* The peel in frame-local form (each frame's movers and factors numbered
  within it, kept in the frame set): with the bit-parallel peel in rounds the
  checks of single promotions were 17–26% cheaper, but the frames of forced
  sets need many rounds (windows +8% and +16%); with the counting peel on
  local indices, +0.5% and +0.8%.
* Caching frame checks: 14–17% of the peels in a window repeat an earlier
  one exactly, but only 5% one of the last eight.
* The no-peel test above for the several moves of a forced set; promoting a
  forced set straight into the child's state; stopping the bound from the
  potentials once its first part decides; the Dijkstra horizon (no events
  later than a known settling one: windows +0.4% and +2.5%, as in an earlier
  measurement); one branch for `reach`'s two tests: each within ±0.5% or
  worse.
* Unchecked indexing in the Hungarian search: −4% instructions but −0.7%
  cycles, not worth `unsafe`.
* Widths of 12 and 48 words: 3–4% at 512–767 (solved) and some at 2048–3071,
  for 12 s more of every build.
* Not done: `i32` potentials (no bound on the potentials is proven, so not
  exact by construction); a warm-started unit rebuild (another optimal
  matching can change the tree after a split or a resume); candidate-index
  sets for every search-level set (a large refactor); watched members for
  the nogoods (now 3% of the time).


## The engine

How the engine ([DESIGN.md, "Engine"](../DESIGN.md#6-engine); the protocol is in the module comment of
`src/engine.rs`) came to be as it is.  All of this is from 2026-10 unless noted.

* **Yields leave out nodes with nothing to do.**  The Taxman searcher leaves out of its yield the nodes of its stack
  that have no candidates left (a node whose loop `fix_forced` ended): as units they would only be runs of one step
  that end the loop.  At 2 threads on 936, 1100 and 1184 (twice each, `ladder --threads 2,2`) the runs of at most
  one poll fell from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200, the nodes and times unchanged.
* **Splits, sized for many cores** (for a 192-core machine).  The first design answered one split request at a time,
  which serialized the distribution of work at many threads.  The engine now keeps one open split request per idle
  worker that the queued and held units will not serve, and any running worker can claim one (an atomic decrement
  of a count in the word `poll` reads), so as many workers split at once as there are idle workers to feed.  From
  one unit, all workers are busy after a few rounds in which every running worker splits, not after one split per
  idle worker.  A worker answers only after 64 polls of its current unit, so a fresh unit first gets deep enough
  to have something to give, and no worker restarts its unit at every step while the demand lasts.
* **One deque.**  The deque stays a single locked one: units exist only after yields, each costs its searcher a
  state rebuild (for Taxman a matching solved from scratch, 30–170 µs already at n = 480–760, against about a
  microsecond under the lock), and with every worker busy nobody takes the lock.  Per-worker deques with stealing
  would remove contention that is not there, at the price of distributed ordering, spill, checkpoint and
  termination arguments.  The lock's wait time is measured (`Counters::lock_wait`) so that a many-core run shows
  whether that ever changes.
* **Checkpoint pauses at any interval.**  Before, an interval of 0 paused again at once without letting go of the
  lock, and the workers never ran again.  Now a periodic pause begins only once the workers released from the last
  one all run again, and no sooner than the last pause lasted; and a worker answers it (as a split request) only
  after 64 polls of its current unit, so every pause is followed by work.  A pause costs every worker the wait for
  the last one to park, bounded by the longest stretch between two polls, one Taxman step: about 10 µs, over a
  millisecond about once in 10⁵ steps, the longest measured 10–15 ms at n = 1215–1225, plus the write and fsync.
* **Checkpoint headers (2026-10-12).**  Headers before version 3 (magic `TXS2`) recorded neither n nor the frames: a
  checkpoint of 140 passed for one of 141 (same candidates, same frame 0, other frames).  They are refused with that
  reason rather than accepted unchecked.
* **Set widths.**  The widths of 20, 24 and 28 words between 16 and 32 pay near the frontier: `Bits<20>` instead of
  `Bits<32>` takes 6% fewer cycles at 1184 and 1215 (2026-10).  Each width is a copy of the search's code: the
  release build took 25 s with four widths, 50 s with the seven now built (62 s with 12 and 48 too, which were left
  out: 512–767 is solved, and 2048–3071 far off).  A checkpoint records its width, and a search resumes at that
  width if this build has it (an older build chose 32 for 1024–2047).

## Benchmarks, the probe, PGO and scaling

What the measuring tools were first used to find ([DESIGN.md, "Performance"](../DESIGN.md#10-performance) says how to
use them).

* **The bench set.**  Until 2026-10 `taxman bench` ran n = 480–760 (W = 8 and 16, milliseconds per game), which no
  longer exercised the frontier's hot paths; the Java took 2.0–65.6 s for those on one thread and 1.4–12.4 s on ten.
  The set is now 936, 1050, 1100, 1133, 1170 and 1184, about a second each on one thread.  A window of a long proof is
  not a game the solver can run, so it is in the probe.
* **The probe** (`examples/perf.rs`), about 5 s in all: F1, 300 search nodes of each of the proofs of 936 and 1215
  with every kernel over all of them; F2, the frame checks of those promotions; F3, a checkpoint of each proof
  (1215 after 1.5M nodes) resumed on one thread for 50,000 steps, one period of forgetting nogoods.  Cycles and
  instructions retired of the process are what it reports, since other load on the machine barely moves them,
  unlike wall time (up to 3x).
* **Profile-guided optimization** (`tools/pgo.sh`).  On the code before "Faster nodes, same trees" it took 12% off
  the cycles of the proof of 1215 (151.1 → 132.5 G), 13% at 1184, 6% at 936 and 9–11% on games it was not trained on
  (880, 1050, 1100, 1170), mostly, it turned out, by inlining the Hungarian search's heap; with that inlined by
  hand it adds 0–2% (1215: 106.0 → 105.1–106.4 G, 1184: 3.16 → 3.10 G).  Profile the plain build: with PGO the
  search is inlined into a few functions and `sample` sees little.
* **Thread scaling.**  On the M1 Max (8 performance cores, with another job on up to 4 of them) with the proof of 1215
  (`ladder --seed-opt --known results/default.jsonl --tactic revise_freedoms=off`): 62.5 s on one thread, 24.6 s on
  2, 15.8 s on 4 and 12.9 s on 6, effective parallelism 99.9–100%, all workers busy within 3 ms; node counts vary
  by ±10% with the threads (what each thread's nogood table has learned).  Games that take minutes on one thread
  (1215, 1220, 1225 with `--seed-opt`: about 60, 150 and 200 s) suit a many-core machine; at 192 threads a game of a
  minute is a third of a second, mostly ramp-up and tail.  [bench/curve/README.md](../bench/curve/README.md) has
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

## Same search (2026-10)

*Question.*  Rank 1 of [IDEAS.md](IDEAS.md#1-the-recommended-order): many games pose exactly the search problem of an
earlier game of the chain (the same frames, candidates, `max_total` and factor graph; only the cap differs), so they
have its best promotion sum, yet the solver proves it again.  How many, what does it cost, and does concluding them
from the earlier game hold up?  The pilot (`ideas repeats` on the branch `ideas-pilots`) hashed the root frames, the
candidates and `max_total` with Rust's `DefaultHasher`; this round makes it a proof kind (commit f5a2af3; DESIGN.md,
["Same search"](../DESIGN.md#same-search), has the argument).

*Method.*  The problem is written out canonically (`search::Problem`, which also records the model's factor lists and
home frames for every number of the frames) and compared whole; the digest only finds candidates.  The repeats are
found from the setups alone, with no search: `cargo run --release --example repeats -- 1 1300 --known
results/strict.jsonl` sets up every game of 1..1300 that is neither prime nor trivial as a strict run does, finds
the latest earlier game with an equal problem, and adds up the `seconds.search_all_runs` that the committed strict
chain spent on each repeat (8.4 s for the 1076 setups, about 8 ms a game; 25–30 ms near 1300).

*How many, and how far back.*  545 of the 1076 games of 1..1300 that set a search up repeat an earlier game: 416 in
1..1000, 100 in 1001..1231 (12 of them in 1201..1231) and 29 in 1232..1300.  Each repeats the last board size before
it that set a search up: n − 1 for 471, n − 2 for 74, and for every one of those 74, n − 1 is prime.  So a window of 8
board sizes (`SAME_SEARCH_WINDOW`) misses none, and the setups a range must make for lines that record no digest
(schema 3) stay at eight.

*What they cost in the committed `strict.jsonl`* (6 threads, commit bd9bb5d), so what a rerun of its chain would not
spend:

| range | repeats | their search | the range's search | share |
|---|---|---|---|---|
| 1..1000 | 416 | 29.0 s | 82.8 s | 35.0% |
| 1001..1231 | 100 | 72.9 s | 409.7 s | 17.8% |
| (1201..1231) | 12 | 45.8 s | 276 s | 16.6% |
| 1232..1300 | 29 | 851.2 s | 2866.3 s | 29.7% |
| 1..1300 | 545 | 953.1 s | 3358.8 s | 28.4% |

The repeats of 1232..1300 (game ← the game it repeats, its committed search seconds): 1234←1233 0.1, 1238←1236 0.1,
1241←1240 6.7, 1243←1242 3.6, 1244←1243 0.1, 1246←1245 0.1, 1247←1246 3.6, 1252←1251 0.1, 1253←1252 3.8, 1255←1254
24.2, 1257←1256 23.0, 1258←1257 0.1, 1261←1260 48.2, 1262←1261 0.1, 1263←1262 51.3, 1266←1265 0.1, 1267←1266 63.6,
1268←1267 0.1, 1270←1269 0.1, 1271←1270 75.3, 1273←1272 38.5, 1276←1275 0.1, 1282←1281 0.1, 1285←1284 36.1,
1286←1285 0.1, 1293←1292 207.1, 1294←1293 0.1, 1298←1296 0.1, 1299←1298 265.1.  The pilot named the first game of
each class (1299 repeats 1296, 1263 and 1261 repeat 1260); the solver takes the latest, which is the same problem.
The searches of 0.1 s are the ones that reached the cap at once: a repeat saves little there, and much where the
earlier game's proof is a long exhausted search.

*At the frontier, without big runs.*  The committed `strict.jsonl` cut after 1298 (its lines for n ≤ 1298, incomplete
ones included) and extended with `taxman solve 1299 --strict --threads 2 --out FILE`: 1299 is a repeat of 1298, 0.079
s for the game and 0.46 s for the whole process (opening and checking the 1301 lines, and setting the window's eight
schema-3 games up again), score 531389 as committed, against 265 s of search in the committed run.  The same from the
file cut after 1292: 1293 repeats 1292 (0.081 s, 526200; 207 s committed) and 1294 repeats 1293, itself a repeat
(0.082 s); cut after 1270: 1271 repeats 1270 (0.064 s, 509880; 75 s committed).  Every file passes `taxman
verify-results` and `tools/check_results.py`.

*Chains with the tactic on and off.*  The strict chain 1..300 concludes 107 games as repeats with the tactic on
(proofs: trivial 13, prime 62, cap 34, exhausted 84, same_search 107), against cap 101 and exhausted 124 with it off,
every score the same; its repeats are exactly the 107 that the setups alone find.  With `revise_freedoms` on as well
(only `reuse_prev` off) there are 107 repeats too, 69 of the 300 games resting on `revise_freedoms`.  With the default
tactics `taxman regress --to 400` has no repeat at all (trivial 13, prime 78, reuse 200, repeat 0, search 109):
`reuse_prev`, which comes first, accepts every game whose problem is its predecessor's; `--strict` gives repeat 151,
search 158.  The probe's fingerprint is unchanged (6197a47eae6cf8c8; the shortcut is outside the search), and
`cargo test --release` passes, with `tests/same_search.rs` (2.7 s) and the forged repeats of `tests/results.rs`.

*Decision.*  Kept: tactic `same_search`, on by default and in strict runs; proof `same_search`, results schema 4.  A
rerun of the strict chain to 1300 would skip 28% of its search time, and the frontier games that repeat cost
nothing; it changes no score, tree or node count of a game that is searched.  Verifying a repeat sets two problems up
again (the verifier caches them by board size), so a schema-4 chain to 1300 will take a few seconds more to open.
