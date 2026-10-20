# The search

Part of the [design](../DESIGN.md).

The search is a branch and bound over promotion sets.  It keeps the best promotion set found so far, the *incumbent*,
and explores the sets that could beat it, pruning every part of the tree that a bound shows cannot.  This section builds
it up: the state and the order of the search, two bounds on a node (the matching bound and the Lagrangian bound over the
nogoods), and the conflicts and nogoods that the branching rests on; the GF(2) bound, the default, and the rule of
symmetric pairs are in [gf2-bound.md](gf2-bound.md).  "The default step" then puts them together, and the last two parts
are the search's start and its
extended mode.  The [glossary](../../../docs/GLOSSARY.md#the-search) defines the search's words.

## Search state and order

The search runs depth first over *nodes*, one *step* at a time.  A node is a set of promotions made from the root, with
the candidates left to it.  A step tries one candidate at a node: it promotes the candidate in a *child*, the node with
one more promotion, and explores the child before the node tries its next candidate.  The search counts steps, not the
nodes of the tree, and `nodes` in the output is the count of steps.  One explicit stack, reused from node to node, holds
the path, with no recursion and no allocation per node.

In the code a node is `(state: FrameSet, promoted: Bits, cands: Bits, max_add: u32)`:

* `state` is the root with the numbers of `promoted` promoted, in any order, since the result does not depend on it;
* `cands` holds the candidates that the node has still to try;
* `max_add` bounds how many more the node can promote,
  `max_additional = min(max_total − |promoted|, state.max_promotions(cands))`, where `max_total` is the most promotions
  the frames' freedoms allow.

A *work unit*, what the engine passes between threads and writes to checkpoints, is just `(promoted, cands)`, two
bitsets, from which a thread rebuilds the state in one pass.  A rebuilt node may prune a little differently from the
node it replaces, with the same answer: the rebuild recomputes `max_add` from the remaining candidates, which can only
tighten it, and the conflicts and nogoods that a searcher learns along its path are a cache that units do not record.

A node's *need* is what its completions must reach to beat the incumbent: `need = best + 1 − sum(promoted)`, where
`best` is the incumbent's promotion sum.  A node's loop is lazy, so that the
search explores each child as soon as it makes it, instead of generating them all first.  The loop ends when a bound on
the node's completions is below its need.  Otherwise it takes a candidate out of the loop, promotes it in a child,
narrows the child's candidates, and explores the child if the child's own bound reaches the child's need.  A bound that
only equals the incumbent cannot lead to a better one, so the loop ends then too, where the Java ended only below it.
The bounds, the choice of candidate and what the child promotes are the frame solver's own: the parts below build them
up, and "The default step" puts them together.

## The matching bound

The *matching bound* relaxes winnability to *matchability*: a winnable frame gives its moves distinct maximal factors.
Each move of a winning order takes at least one factor that is still in the frame and removes it, so no factor pays for
two moves.  A promotable set `Q` therefore gives a distinct maximal factor to every move of every frame and to every
promoted number (in the frame in front of its home), and uses no member of `Q` as a factor.  Every edge `v → v/p`
changes the number of prime factors by one, so the graph is bipartite.  The matching bound is the largest sum of a
candidate set that has such an assignment: a maximum-weight bipartite matching (a min-cost flow), solved exactly.  It
bounds the best promotable completion from above, and ignores the freedoms and `max_total`, which only remove
completions.

The bound is an upper bound on `sum(promoted)` plus a completion's sum, so the search cuts a node whose matching value
is below its need.  At the roots of the 94 golden games that need a search, its mean gap above P\* is 31, against 76 for
the Java's bound; on the former benchmark games (n = 480 to 760), 107 to 292 against 363 to 769.  The Java's
bound, which the matching bound replaced, never pruned where the matching bound did not: the diagnostics counted none at
every check.

*Incremental.*  The search keeps the bound along its path instead of solving it at each node.  `matchbound::MatchGraph`
is the root's graph, whose edges never change, and a node's problem is that graph with each vertex in one of three
roles: required (the root's moves and the promoted numbers), candidate, or factor.  `MatchBound` keeps an optimal
matching and an optimal dual solution, the *potentials*, for every node on the search's path, and repairs them after
each change by Hungarian searches on a radix heap.  The changes along a depth-first path are small, so an evaluation
costs 0.7 to 2.4 µs, where a solve from scratch costs 30 to 170 µs (at n = 480 to 760).  The potentials also bound the
matching *without* a given candidate at once (`bound_excluding`): lower that candidate's potential and its partner's as
far as their remaining edges allow.  The module comment of `matchbound.rs` is the best explanation of all this.

## Conflicts of the matching bound

When the candidates `Q*` of the heaviest matching cannot all be promoted, the matching bound yields a *conflict*: a few
candidates that cannot be promoted together from the node.  Some frame is unwinnable with all of `Q*`.  A peel of that
frame stops at its *residual*, the moves that no order can take (THEORY.md's word for where every dismantling ends).
The members of `Q*` that the residual needs cannot be promoted together: those promoted into the frame that are residual
moves, and those promoted out of it that are factors of residual moves.  Peeling only the residual makes the set
minimal, and the result `K ⊆ Q*` is the conflict (`FrameGraph::conflict_using`).  Conflicts are tiny: a median of two
members at n = 760 to 1000, and a quarter of them have one.

Everything that follows rests on one fact.  A subset of a promotable set is promotable, since moves left out and factors
kept only help a frame.  So **every promotable completion leaves out some member of a conflict**.

* **Refinement** (`match_refine`, default 1).  Where the matching bound does not prune, the search bounds the branches
  of the conflict too.  With members `k₁ < k₂ < … < k_r`, every promotable completion `F` is in exactly one branch
  "`k₁ … k_{j−1}` in `F`, `k_j` not".  So the best completion is at most the largest of the branches' matching bounds,
  each the matching with `k_j` no longer a candidate and `k₁ … k_{j−1}` required.  Each branch is refined the same way,
  one level less deep, and by induction the refined bound is never below the best completion
  (`search::refine::Refiner`).
* **Strong branching on the conflict.**  A node branches on a member of the conflict instead of its largest candidate:
  the child promotes the member, and the node goes on without it, so that neither can reproduce the heaviest matching.
  The member is the one whose removal lowers the matching bound most.  That takes one bound without each member, each
  solved exactly (`MatchBound::value_excluding`).  A node with no conflict among its candidates
  branches on its largest.  The smallest member, the largest, and the Java's largest candidate were once tactics; at the
  hardest games they took 6 to more than 17 times as long, and were removed.
* **Exact matchings.**  A heaviest matching that can be promoted is the node's best completion: the search offers it as
  the incumbent, and the node is done.
* **Propagation into children.**  A conflict known at a node, of which a child promotes all but one member, rules that
  member out of the child's candidates: the child's state is the node's with more promotions, so promoting the member
  would promote the whole conflict.  Without this rule the child's matching bound would equal the node's.

"The starting incumbent, the ceiling and the seed", below, says where the incumbent starts.

## Forced members and nogoods

The tree is mostly chains, and two devices make the most of it: forced members turn a node whose loop would end after
one child into a single step, and nogoods carry conflicts across the tree.  At four nodes in five a node's loop ends
right after its first child, because removing the member that strong branching chose already brings the bound below the
node's need.  Such a member is *forced*: every completion that beats the incumbent promotes it.

* **Forced members** (`fix_forced`).  The child promotes every forced member of the conflict, not only the chosen one,
  and the node's loop ends after that child, which covers every completion that can beat the incumbent.  If the forced
  members cannot all be promoted, or the child's candidate filter dropped one, the child has no such completion at all.
  The child promotes them together, checking each frame once (`FrameGraph::promote_all_using`).  The search finds the
  forced members from the potentials, for every member of the heaviest matching, and solves strong branching's bounds
  only where it finds none.  A conflict whose members are all forced ends the node at once.
* **Nogoods** (`nogoods`).  A conflict `K` found at a node with promotions `P` holds relative to `P`.  A minimal subset
  of `P ∪ K` that keeps a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from the root, so at every
  node of the tree, on any thread and after any resume, the nogood less the node's promotions is a conflict of the node
  (`model::Nogoods` has the proof).  Each searcher keeps a table of nogoods.  A node's conflict is the smallest one the
  table knows within `P ∪ Q*` (of those as small, the one with the largest member), and the searcher computes and learns
  a conflict only when the table knows none.  Nogoods also rule candidates out of children, beyond the conflicts known
  along the path.  The table is scanned exactly, over compact member indices.  What a searcher learns depends on the
  order its search went, so the tree depends on the searcher's history (its threads, its interruptions), not only on the
  node; the answers do not.
* **Forgetting nogoods** (`forget_nogoods`).  Each searcher clears its table after every 50,000 of its own steps
  (`search::NOGOOD_PERIOD`), because a lookup scans the whole table: a larger table slows every node, and old nogoods
  cost more than they help.  The table is a cache, so forgetting is sound; it is deterministic on one thread, and a game
  of fewer steps than the period is unchanged.  Carrying the tables from one game to the next, or storing them in
  checkpoints, does not pay ([carrying nogoods
  over](../../notebook/search/carrying-nogoods-over-2026-10-measured-not-kept/README.md)).  Which nogoods a searcher
  keeps, and for how long, is a policy and nothing more, since any subset of valid nogoods is as sound as all of them.
  Three research tactics set it (`nogood_period`, `nogood_keep` and `nogood_limit`; [tactics.md](tactics.md)), whose
  defaults are the behaviour above, with the same trees.  The table counts what it does (nogoods learned, lookups and
  the share they answered, candidates ruled out, the table's size at each forgetting and what was kept, evictions), and
  the ladder and the progress lines print the counts.  A checkpoint does not record the policies, since the answer does
  not depend on them and a resumed search starts with empty tables; the run that resumes says which it uses.
* **Single-member conflicts belong to the node.**  Many conflicts have one member, which looks like a candidate that
  cannot be promoted from the root at all and could be filtered out once.  It cannot: on every game from 2 to 1230 the
  frame builder's candidates all pass that test at the root (a test checks every game to 511 and every sixth
  past it).  A single-member conflict is a
  candidate that cannot be promoted *once the node's promotions are made*: its frame may have used it to pay for a move
  left with one live factor, say.  In another branch it is part of a nogood of two or more members.  The promotions that
  fail are under 4% of the steps, so little would be saved ([single-member
  conflicts](../../notebook/search/single-member-conflicts-belong-to-the-node-2026-10/README.md)).

## The Lagrangian bound over the nogoods

The *Lagrangian bound* (tactic `lagrange`, off by default) puts the nogoods into the matching bound, which knows none:
its heaviest matching often promotes all of a nogood.  At a node with promotions `P` and candidates `C`, a nogood `N` is
*live* when its members outside `P` are all still candidates, and then every completion `F` leaves one of them out:
`|F ∩ N| ≤ |N − P| − 1`.  Lagrangian relaxation, as in integer programming, moves these inequalities into the objective
with multipliers `λ_N ≥ 0`.  The result bounds the completions for any multipliers (weak duality): the heaviest matching
with each candidate weighed by its number less the multipliers of the live nogoods it is in, plus `Σ λ_N (|N − P| − 1)`.
The search computes it in integers (weights in 64ths), with nothing rounded; the module comment of
`matchbound::lagrange` has the argument.

* **Incremental.**  The bound keeps a second stack of matching levels, with weights of its own (`matchbound::lagrange`;
  the levels and the Hungarian repairs are the matching bound's, generic over how a level weighs its candidates).  The
  searcher makes a level for a node when it asks for the bound there, from the nearest level on its path: what the node
  promoted is promoted, the candidates it no longer has are dropped, and the live nogoods they were in die.  So the
  weights of those nogoods' other members only *rise*, which needs a repair only where a member's potential must rise
  past a tight edge.  Weights fall only when the multipliers change, and then the searcher drops the levels and solves
  them afresh.  A level costs about what a matching level does.
* **Where, and what for.**  The search asks for the bound at every check of a node or a child that the matching bound
  and refinement leave open.  Below the need it cuts the node (cut reason `lagrange`).  Otherwise it yields forced
  members: the members of its heaviest matching without which the bound, read off its potentials as `bound_excluding`
  reads the matching's, would be below the need.  Every completion that beats the incumbent promotes them, and with
  `fix_forced` the node's child promotes them with the matching's.  The forced members are most of the gain.
* **The multipliers.**  Each searcher fits its own, at the root, to the nogoods it has learned: after 2,000 of its
  steps, and then just before each forgetting of its table (every 50,000), it takes 50 subgradient steps (Polyak's rule,
  towards what the root needs) over the table's nogoods and those of its present multipliers, and keeps the nogoods
  whose multipliers are positive (`search::lagrange`).  Any multipliers give a bound, so the schedule decides only the
  speed.  Like the nogoods, the multipliers are a cache that units and checkpoints do not record, and the tree depends
  on the searcher's history.  A search of fewer than 2,000 steps is unchanged.
* **What it is worth** (on the matching path).  Proofs of the hardest games, 1184 to 1300, take 2 to 6 times fewer
  cycles on one thread, about 3 times on average, with no trend in n; the chain 1201–1231 on four threads takes 2.4
  times less game time.  Each step costs about half as much again, so a search whose time goes into finding its best set
  late, as 992's does, is slower ([a Lagrangian bound over the
  nogoods](../../notebook/search/a-lagrangian-bound-over-the-nogoods-2026-10/README.md)).  With the GF(2) bound it
  changes no tree measured ([tactics.md](tactics.md), the row of `lagrange`).

## The default step

One step of a node's loop, with the default tactics, takes the parts above in this order (`Stepper::step` in
`src/search/searcher.rs`; the module comment of `src/search/mod.rs` says which of its four phases takes which step):

```
need = best + 1 − sum(promoted)                   # the loop goes on while the bound reaches this
1. end the loop if cands is empty, or the max_add largest candidates sum below need; unless this is the node's
   first step (its candidates as step 7 left them), drop from cands the candidates that no completion keeping the
   symmetric pairs' inequalities holds, and end the loop if none keeps them (symmetry), or if what is left is empty
   or its max_add largest sum below need
2. look at the node's heaviest common independent set I* (gf2) of at most max_add members:
     value < need                                  -> end the loop (cut)
     F = the forced members of I* (by the weight split: those without which the bound is below need; and those it
         leaves free within force_window of need, all by default, whose exchange graph has no cycle through them
         within value − need)
     the nogood table knows a conflict of I* all of whose members are in F or promoted
                                                   -> end the loop (cut)
     I* can be promoted                            -> offer promoted ∪ I* as incumbent; end the loop
     otherwise: a conflict K ⊆ I* (the nogood table's smallest, else computed and learned)
3. if K ⊆ F: end the loop (every completion beating the incumbent promotes all of K, which cannot be)
4. if F is empty: the bound without each member of K, read off the exchange graph of I* or solved (exact strong
   branching); a member without which it is below need joins F; n = the member without which it is lowest
5. n = a member of F if there are forced members; remove n from cands; if F is non-empty the node has nothing left
   to try after this child
6. child = the node with F (or n) promoted; if that fails, the step is over
7. child cands = the rest, narrowed, minus the members that a known conflict or a nogood rules out, minus those
   that no completion keeping the symmetric pairs' inequalities holds (symmetry)
8. cut the child if the max_next largest of its cands sum below its need (max_next: how many more promotions the
   child can make among its remaining candidates, the child's own max_add), or no completion of it keeps the
   inequalities (symmetry), or its GF(2) bound is below it, or its heaviest set is exact (offered as incumbent); a
   child that stays open keeps this verdict, with its forced members, for its first step; a child with no
   candidates is not pushed
9. otherwise offer the child's own promotions as incumbent, and push it
```

The *matching path* (`SearchTactics::matching`, which is `--tactic gf2=off --tactic lagrange=on --tactic
symmetry=off`) takes the matching bound in place of the GF(2) bound, and the rule of symmetric pairs only if asked.  Its
step 2 looks at the heaviest matching `Q*` instead: its forced members from the potentials, then, where none is forced,
refinement of its conflict `match_refine` levels deep (a cut if every branch is below `need`), then the Lagrangian bound
over the nogoods (a cut, or more forced members).  Its step 4 chooses the member whose removal lowers the matching bound
most, and its step 8 checks the child's matching bound, refinement and Lagrangian bound.

*Why the step is exact.*  Exactness rests on five facts:

1. the bound is an upper bound: the GF(2) bound by Theorem 11, the matching bound with refinement, and the Lagrangian
   bound for any multipliers;
2. every promotable completion leaves out a member of a conflict, so branching on a member and then going on without
   it explores every promotable set;
3. every completion that can beat the incumbent promotes the forced members (by weak duality, for the weight split, the
   potentials and the Lagrangian bound alike);
4. nogoods and known conflicts hold at every node they are applied to, since a subset of a promotable set is
   promotable;
5. every heaviest promotion set keeps the symmetric pairs' inequalities (Theorem 13), so the completions that the rule
   leaves out hold none of them.

A node is cut only when its bound is below `need`, since a completion that merely ties the incumbent is of no use, and
a unit rebuilt from a checkpoint is a node whose bound can only be tighter.  `tests/soundness.rs` attacks each of these
facts ([testing.md](testing.md)).

## The starting incumbent, the ceiling and the seed

The search starts from the best of three promotable sets, and stops as soon as it reaches a proven ceiling.  The three
sets:

* the *greedy* set: promote every candidate that can be, from the largest down;
* the *seed*: the promotions of the game of n − 1 that are candidates here and still promotable, taken one by one from
  the largest;
* the *dive*: a rounding of the root's matching, which drops the smallest member of a conflict until the rest can be
  promoted.

A local search then improves the best of them: it fills the set greedily, then drops one or two members and refills,
until nothing improves.  The *ceiling* on the promotion sum, `n + score(n − 1) − in_the_bag`, is proven, and reaching
it ends the search (tactic `prev_cap`).  When the greedy set or the seed's survivors already reach the ceiling, the
search skips the dive, and it skips the local search when the incumbent after the dive reaches it: nothing can beat a
set at the ceiling, so neither would change it, and the result is the same.

*Details.*  The local search runs on the search's threads, the refills of a pass in parallel, and keeps the first
improvement in the order of the sequential pass, so its answer does not depend on the thread count.  It reads the
ceiling before each pass, so a ceiling given while it runs (through the control, [Engine](engine.md#engine)) ends it as
soon as its set reaches it.  The start is offered to the run's control as it is made (after the greedy set and the seed,
after the dive, after the local search), so whoever steers the search sees the best so far before the engine starts, and
the engine's start is unchanged.  `board::Board` sets a board and its search up: it is the one place that knows the path
from the tables to the frames, the prepared search, the ceiling and the seed.

*A floor.*  A search can start instead from a *floor* (`search::search_above`): an incumbent worth a given promotion
sum, with no set and no start made.  The search prunes nodes against the floor as against a set of that weight, and a
set it finds heavier becomes the incumbent as usual.  A search that completes with none found has proven that nothing is
heavier than the floor ([docs/THEORY.md](../../../docs/THEORY.md), Lemma F, the argument of the dual bound of a stopped
search), which `SearchResult::floor_held` says.  Its tree is that of a search from an incumbent of that worth: at P\*
the proof's tree, node for node, as a search seeded with an optimal set makes it; above P\* a smaller one, by a factor
of about 27 for each quarter of the gap between the root's bound and P\*, on the games measured ([searches under a
floor](../../notebook/search/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md)).

## Extended mode: one-sided candidates

The search can also take the *extended problem* Π⁺(O), the problem that a certificate of U searches
([certificates.md](certificates.md)).  It is the frame model with a set O of the sources of three or more prime factors
that setup keeps made optional (THEORY.md, ["The extended problem"](../../../docs/theory/extended-problem.md), and
the corollary of Theorem M⁺; `frames::build_extended`).  Besides the candidates of the frame model, which are
*two-sided* (out of one frame, into the one in front), it has *one-sided* candidates: the members of O and the sources
that greedy rejected beside them, each a possible move of its *own* frame.  It is research code: the solver's own path
never builds an extended problem, and with no one-sided candidate every piece below is what it is without them.

*The claims of `unzip`* (THEORY.md, Lemma M5⁺).  The problem comes in two formulations, which have the same promotion
sets (the lemma's corollary): without any claim, and, the default, with the claims of the frame model
(`SearchFrames::claims`) that the lemma keeps.  `build_extended` gives each claim the frame, as setup forms it, of its
source.  A frame keeps its claims if it holds no member of O ∪ R and the factor of each of its back claims passes the
check (`frames::ClaimCheck`): it is no two-sided candidate, and no candidate has it among its maximal factors.  The
pairs of a frame that keeps them leave the moves and the factors before the root is reduced; the freedoms and the
candidates do not change.  `ExtendedFrames` records the claims kept and those refused (`refused`).  With nothing open
every claim is kept, and Π⁺(∅) is Π_n: the same problem, so the same trees (`tests/tripwires.rs`; the census's check
H).  `Board::extended(open, claims)` is the one place the board's claims are passed.

The check matters.  A frame that holds an open source has more freedoms, and can take on composites of a claimed factor
that Π_n could not.  At 24 with 16, 20 and 24 open, 12 becomes a candidate that lists 6, which 18 claims; taking the
pair out anyway loses the promotion set {8, 12}, which the oracle catches.  The census counts, game by game, the
claims that some O refuses (`m5_refused`).

* **The model** (`model`, "One-sided candidates").  `FrameGraph::build_extended` gives each one-sided candidate its own
  frame (`own`), and the factor graph lists its maximal factors there, as it does a move's.  Promoting one adds it to
  that frame's moves if the frame stays winnable; it takes no factor, and spends one freedom of that frame only
  (`frames_touched`).  The frame-by-frame check of a set, the reduction, conflicts and nogoods need nothing more: a
  one-sided member is one more move of its frame.  The state does not record which were promoted (a promoted
  two-sided candidate is no longer a factor of its home), so each is promoted once along a path, which the search's
  candidates ensure.  `FrameGraph::narrow` keeps a one-sided candidate while its frame has a freedom, and
  `FrameGraph::max_promotions_among` adds their number to the freedoms' count (sound, not tight).
* **The GF(2) bound** (`gf2`, "One-sided candidates"; THEORY.md, Theorem 11 for Π⁺).  A one-sided candidate's element
  has the vector of its maximal factors in its own frame's matroid, and is a coloop of the other: its own unit vector,
  in a *free block* after the frames' blocks, `64·words` of them to a block, so that hundreds of them widen no vector.
* **The search.**  `Board::prepare_extended_with(open)` (`search::prepare_extended`) makes them candidates of the
  search.  The problem's form ends with them, after a tag that no plain problem's form has, and the checkpoint header
  (format 5) records their digest, so neither a repeat nor a resumed checkpoint can take an extended problem for
  another.  A checkpoint of version 4 resumes only a search without them ([trust.md](trust.md)).
* **What it refuses** (`SearchTactics::refusal_with_one_sided`).  With one-sided candidates the search needs `gf2`: the
  matching bound gives a one-sided candidate no vertex, so it is no bound there (the dive, which rounds the root's
  matching for a starting incumbent, still runs and never picks one).  It refuses `lagrange`, which is built on the
  matching bound, and `symmetry`: Theorem 13 is proven for the frame model, and for Π⁺ each prime exchange would have
  to map the one-sided candidates of each frame onto themselves, which nothing checks.  `SearchTactics::extended` is
  the default without `symmetry`.  Π⁺(∅) has no one-sided candidate and takes any tactics, and its search is the
  default engine's on the same problem, node for node (`tests/tripwires.rs` has a table of its own).
* **Its frames.**  Π⁺(O) has a frame for each layer from the lowest with freedoms to the highest: nine at most up to
  4095 (layers 3 to 11), whatever O is.  From 4096 it has ten with every source open, which `build_extended` refuses
  (`MAX_FRAMES`), naming the game and the layers.  The census of the frames records the count with every such source
  open for every game to 8191 ([the claims of `unzip` in the extended
  problem](../../notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)).  Opening a source also adds
  one to its frame's rank, so a problem with many open sources can need more words of the GF(2) bound than the narrow
  set widths compile (three, rank 192) or the widest (five, rank 320): `search::prepare_extended` refuses it, saying so,
  rather than fail in the search.
