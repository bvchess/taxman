# The search

Part of the [design](../DESIGN.md).

The search is a branch and bound over promotion sets.  It keeps the best promotion set found so far, the *incumbent*,
and explores the sets that could beat it, pruning every part of the tree that a bound shows cannot.  This section builds
it up: the state and the order of the search, the matching bound (which the search's start rounds to a first
incumbent), and the conflicts and nogoods that the branching rests on; the GF(2) bound, which bounds every node, and the
rule of symmetric pairs are in [gf2-bound.md](gf2-bound.md).  "The default step" then puts them together, "Conflict branching"
changes which conflict a node branches on, "The invariant of a running search" says what may change while it runs, and
the last two parts are the search's start and the cut that the game before gives it.  The finder, a thread beside a
long search, is in [finder.md](finder.md), and the search of a problem with one-sided candidates in
[extended-mode.md](extended-mode.md).  The [glossary](../../../docs/GLOSSARY.md#the-search) defines the search's words.

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

The matching value plus `sum(promoted)` bounds every completion of a node.  At the roots of the 94 golden games that
need a search, its mean gap above P\* is 31, against 76 for the Java's bound; on the former benchmark games (n = 480 to
760), 107 to 292 against 363 to 769.  The GF(2) bound is never above it and usually far below it, and it is the bound
of every node ([gf2-bound.md](gf2-bound.md)); the matching bound was the node bound before it, and the search now uses
it only at its start, where the dive rounds the root's heaviest matching to a promotion set ("The starting incumbent,
the ceiling and the seed", below).

*Incremental.*  `matchbound::MatchGraph` is the root's graph, whose edges never change, and a state's problem is that
graph with each vertex in one of three roles: required (the root's moves and the promoted numbers), candidate, or
factor.  `MatchBound` keeps an optimal matching and an optimal dual solution, the *potentials*, for one state, and
repairs them after candidates are removed by Hungarian searches on a radix heap, as the dive removes them one at a time.
The module comment of `matchbound.rs` is the best explanation of all this.

## Conflicts

When the members of a bound's heaviest set (the GF(2) bound's `I*` at a node, the heaviest matching's `Q*` in the dive)
cannot all be promoted, the bound yields a *conflict*: a few candidates that cannot be promoted together from the node.
Some frame is unwinnable with all of the set.  A peel of that frame stops at its *residual*, the moves that no order can
take, the same set whatever the order of the drops
([the residual theorem (P5)](../../../docs/theory/playability.md#the-residual-theorem)).
The members of the set that the residual needs cannot be promoted together: those promoted into the frame that are
residual moves, and those promoted out of it that are factors of residual moves.  Peeling only the residual makes the
set minimal, and the result `K` is the conflict (`FrameGraph::conflict_using`).  Conflicts are tiny: a median of two
members at n = 760 to 1000, and a quarter of them have one.

Everything that follows rests on one fact.  A subset of a promotable set is promotable, since moves left out and factors
kept only help a frame.  So **every promotable completion leaves out some member of a conflict**.

* **Strong branching on the conflict.**  A node branches on a member of the conflict instead of its largest candidate:
  the child promotes the member, and the node goes on without it, so that neither can reproduce the heaviest set.  The
  member is the one whose removal lowers the bound most, which takes the bound without each member
  ([gf2-bound.md](gf2-bound.md#the-gf2-bound), the item "Exact strong branching").  A node with no conflict among its
  candidates branches on its largest.
* **Exact sets.**  A heaviest set that can be promoted is the node's best completion: the search offers it as the
  incumbent, and the node is done.
* **Propagation into children.**  A conflict known at a node, of which a child promotes all but one member, rules that
  member out of the child's candidates: the child's state is the node's with more promotions, so promoting the member
  would promote the whole conflict.  Without this rule the child's bound would often equal the node's.

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
  forced members from the weight split of the heaviest set, for every member of it
  ([gf2-bound.md](gf2-bound.md#the-gf2-bound), the item "Forced members"), and strong branching finds more.  A conflict
  whose members are all forced ends the node at once.
* **Nogoods** (`nogoods`).  A conflict `K` found at a node with promotions `P` holds relative to `P`.  A minimal subset
  of `P ∪ K` that keeps a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from the root, so at every
  node of the tree, on any thread and after any resume, the nogood less the node's promotions is a conflict of the node
  (`model::Nogoods` has the proof).  Each searcher keeps a table of nogoods.  A node's conflict is the smallest one the
  table knows within `P ∪ I*` (of those as small, the one with the largest member), and the searcher computes and learns
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
  keeps, and for how long, is a policy and nothing more, since any subset of valid nogoods is as sound as all of them:
  the period was a tactic of its own once, with a choice of nogoods to keep and a limit to the table, and the notebook
  has what they were worth ([keeping and forgetting
  nogoods](../../notebook/search/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md)).  The table
  counts what it does (nogoods learned, lookups, candidates ruled out, forgettings), and the progress lines print the
  counts.  A checkpoint does not record when the searchers forget, since the answer does not depend on it and a resumed
  search starts with empty tables.
* **Single-member conflicts belong to the node.**  Many conflicts have one member, which looks like a candidate that
  cannot be promoted from the root at all and could be filtered out once.  It cannot: every candidate of every game
  from 2 to 1230 passes that test at the root (a test checks every game to 511 and every sixth past it).  A
  single-member conflict is a candidate that cannot be promoted *once the node's promotions are made*: its frame may
  have used it to pay for a move left with one live factor, say.  The promotions that fail are under 4% of the steps,
  so little would be saved ([single-member
  conflicts](../../notebook/search/single-member-conflicts-belong-to-the-node-2026-10/README.md)).

## The default step

One step of a node's loop, with the default tactics, takes the parts above in this order (`Stepper::step` in
`src/search/searcher.rs`; the module comment of `src/search/mod.rs` says which of its four phases takes which step):

```
need = best + 1 − sum(promoted)                   # the loop goes on while the bound reaches this
1. end the loop if cands is empty, or the max_add largest candidates sum below need; unless this is the node's
   first step (its candidates as step 7 left them), drop from cands the candidates that no completion keeping the
   symmetric pairs' inequalities holds, and end the loop if none keeps them (symmetry), or if what is left is empty
   or its max_add largest sum below need; end the loop if the near-repeat cut holds (near_repeat, below)
2. look at the node's heaviest common independent set I* (gf2) of at most max_add members:
     value < need                                  -> end the loop (cut)
     F = the forced members of I* (by the weight split: those without which the bound is below need; and those it
         leaves free, whose exchange graph has no cycle through them within value − need)
     the nogood table knows a conflict of I* all of whose members are in F or promoted
                                                   -> end the loop (cut)
     I* can be promoted                            -> offer promoted ∪ I* as incumbent; end the loop
     otherwise: a conflict K ⊆ I* (the nogood table's smallest, else computed and learned); where value − need is
         at least 1200, the one conflict branching chooses instead ("Conflict branching", below)
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
   inequalities (symmetry), or the near-repeat cut holds, or its GF(2) bound is below it, or its heaviest set is
   exact (offered as incumbent); a child that stays open keeps this verdict, with its forced members, for its first
   step; a child with no candidates is not pushed
9. otherwise offer the child's own promotions as incumbent, and push it
```

*Why the step is exact.*  Exactness rests on six facts:

1. the bound is an upper bound: the GF(2) bound by [the frame bound
   (B2)](../../../docs/theory/frame-bounds.md#the-frame-bound);
2. every promotable completion leaves out a member of a conflict, so branching on a member and then going on without
   it explores every promotable set;
3. every completion that can beat the incumbent promotes the forced members (by weak duality, for the weight split, and
   by the bound without the member, for strong branching);
4. nogoods and known conflicts hold at every node they are applied to, since a subset of a promotable set is
   promotable;
5. every heaviest promotion set keeps the symmetric pairs' inequalities
   ([the prime-exchange theorem (R2)](../../../docs/theory/search-rules.md#the-prime-exchange-theorem)), so the
   completions that the rule leaves out hold none of them;
6. every completion of a node that the near-repeat cut ends is worth no more than the incumbent
   ([the near-repeat lemma (R7)](../../../docs/theory/search-rules.md#the-near-repeat-lemma)).

A node is cut only when its bound is below `need`, since a completion that merely ties the incumbent is of no use, and
a unit rebuilt from a checkpoint is a node whose bound can only be tighter.  `tests/soundness.rs` attacks each of these
facts ([testing.md](testing.md)).

## Conflict branching

With the tactic `conflict_branching`, on by default, a node chooses the conflict it branches on for what its children
leave of the gap; strong branching then chooses the member.  Each child of a conflict leaves out one member, and its
gap is the node's less what leaving that member out costs the bound, the member's *price*.  The smallest conflict the
search knows often has a member whose price is a few points, and the child that leaves it out keeps nearly the whole
gap, so the subtree hardly shrinks.  A conflict whose members all have high prices makes every child's gap fall, and is
often a set of forced members, which closes the node at step 3.

*The choice.*  At step 2, where the gap, the bound less `need`, is at least `CONFLICT_BRANCHING_GAP` (1,200; the *gate*,
which the run's `--gate` may set from 0 to `MAX_GATE`, 4,000, and the [gate trial](gate-trial.md) may change), the
check weighs:

* `K`, the conflict the search found;
* every conflict the nogood table knows within `I*` (`Nogoods::all_within`);
* in each frame that `I*` leaves unwinnable, the minimal conflict made by leaving out the cheapest members first
  (`FrameGraph::conflicts_ordered`), learned as a nogood if it is chosen.

A conflict weighs the sum over its members of 10^(−price/`CONFLICT_BRANCHING_SLOPE`), with the slope 291: its children's
subtrees against the node's, if a subtree grows tenfold for every 291 points of its gap.  The node branches on the
lightest.  Prices come off the exchange graph of `I*`, as strong branching reads them (`Gf2Bound::prices_excluding`),
for the members that a failing frame's conflict can hold (`FrameGraph::conflict_members`); the others take the weight
split's estimate.  A child that inherits its parent's heaviest set takes most of its prices from its parent's pricing
with no search ([gf2-bound.md](gf2-bound.md)), and the tactic `price_reuse` keeps prices from check to check
([price-reuse.md](price-reuse.md)).  Below the gate the search's own `K` is kept, since a small subtree does not repay
the pricing.  On units of the games 4991, 4992 and 4995, a gate of 1,200 took 0.90 to 0.92 of the CPU of conflict
branching off, and a gate of 300 took 2.03 of it on those of 4991 ([the settings
checks](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-settings-checks-on-games-4991-4992-and-4995)).

*Why it is exact.*  It changes only which conflict a node branches on, and the second fact of [the default
step's](#the-default-step) exactness holds for any conflict.  Each conflict made here is one: with its members alone
applied, its frame keeps a stuck set of moves, so it is not winnable ([the peel theorem
(P4)](../../../docs/theory/playability.md#the-peel-theorem)), and a stuck set survives every larger set of promotions
([P5](../../../docs/theory/playability.md#the-residual-theorem)), so no completion holding the whole conflict is
promotable ([the exactness theorem (X7)](../../../docs/theory/exactness.md#the-exactness-theorem)).  `conflicts_ordered`
checks each with the whole peel.  Since it only chooses among true conflicts, the tactic may change between a search's
units ([the invariant](#the-invariant-of-a-running-search)), and a checkpoint resumes with it on or off.

*What it does.*  On one thread from the known optimum, the whole games 4050 and 4176 take 0.19 and 0.16 of the
default's steps and 0.26 and 0.27 of its CPU, the share of steps saved growing with the gap (a step costs more, mostly
in the pricing).  The measurements, and the gap and slope chosen, are in [where the search of 4875
spends](../../notebook/search/where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md).

*The look.*  With the tactic `conflict_look`, on by default, a check that would branch on a conflict other than `K`
looks at one child of `K` first.  At some deep nodes the choice above avoids `K` although one child of `K` dies at
once, and the node then drains its gap a member at a time.  The cause is a hub: a member that costs the bound almost
nothing to leave out, because a substitute can take its place, but that every completion beating the incumbent holds.
The weight rates the conflicts that hold the hub worst, so it overrates every child of `K`, not only the dead one.  The
look is at the last child of `K`, the child that promotes every member but the cheapest, `c`, and leaves `c` out.  It
looks only where no member is forced, and where:

* `c` has a price of 10 or less (`CONFLICT_LOOK_PRICE`);
* the weight of `K` less the term of `c` is below twice (`CONFLICT_LOOK_SLACK`) the weight of the conflict chosen.

If that child is cut on sight, the node branches on `K`.  A child is cut on sight when its promotion fails, or its
largest candidates, its GF(2) bound, or a conflict of its heaviest set all of whose members it forces put it below its
need.  The look does not apply the rule of `symmetry`, the near-repeat cut, the carried nogoods or the conflicts that
the node knows, so a child that it lets through may still be cut when the search makes it.  The child leaves out a
member of the node's heaviest set, so it inherits no heaviest set, and a solve of its bound costs several evaluations of
a node.  So the look first bounds the child's bound from the node's weight split, with no solve
([gf2-bound.md](gf2-bound.md#the-gf2-bound), the item "A child's bound from its parent's split"), and solves the child
only where that bound reaches the need.  It cuts nearly every child that the solve cuts: there the substitute that made
`c` cheap is mostly a candidate that the child lost, and the split's bound loses both.

*Why the look is exact.*  It only chooses between two true conflicts of the node, `K` and the one conflict branching
chose, so the second fact of [the default step's](#the-default-step) exactness holds for it as for the choice.  It must
also leave the node's state as it found it.  The child is pushed on the GF(2) stack and popped before the node reads its
own level again, which then resolves its level as it does after any child.  Where the split's bound cuts the child,
nothing is pushed and the workspace keeps the node's tableau; the split's bound is never below the child's own (weak
duality), so it cuts only children that the solve would cut, and the look chooses as it would with the solve.  The
conflict that cuts the child is learned as a nogood of the root, which holds at every node, as the search learns its
own.  So the look may change between a search's units, as the choice may.

*What the look does.*  On the units of games 4875 to 4901 it takes 0.61 to 0.87 of the steps without it ([when a node
should use conflict
branching](../../notebook/search/when-a-node-should-use-conflict-branching-2026-10/README.md)); games of the chain
decided it, 4918 to 4948 taking 0.85 of their neighbors' steps without it, in geometric mean ([the live
lab](../../notebook/search/the-live-lab-2026-10/README.md#2026-10-23-the-look-tested-on-whole-games)).  With the split's
bound, units take 0.79 to 0.95 of the CPU of a look that solves every child, with the same trees ([cheaper steps of
conflict branching](../../notebook/search/cheaper-steps-of-conflict-branching-2026-10/README.md#2026-10-23-a-cheaper-look)).

## The invariant of a running search

Five things change while a search runs, and none changes its answer: the order and share of its work (which worker
explores which unit, and when, is free: [engine.md](engine.md#engine), the item "Control", and
[trust.md](trust.md#what-a-window-changes-in-the-results)), its incumbent, through offers that are checked
([finder.md](finder.md)), its branching, and, if the game before it concludes meanwhile, its ceiling and near-repeat
cut.  Both end work only where the incumbent reaches a proven bound ([the
ceiling](#the-starting-incumbent-the-ceiling-and-the-seed), [the cut](#the-near-repeat-cut)).  Nothing acting on a
running search adds a nogood.

A searcher reads the branching settings, the two switches and the gate, when it starts a unit, and explores the unit's
subtree to the end.  So a search whose units branch differently is exhaustive unit by unit: every conflict that
`conflict_branching` chooses among is a true conflict, and the children of any true conflict cover every completion of
their node ("Conflict branching", the paragraphs "Why it is exact" and "Why the look is exact").  The gate is the gap
from which a node chooses a conflict at all, and below it the node branches on the search's own conflict, so every
gate from 0 to `MAX_GATE` keeps the search exact, and the gate trial may change it between units
([gate-trial.md](gate-trial.md)).  The nogoods learned on the way are conflicts whichever conflict a node chose, and
they hold at every node (fact 4 of [the default step's](#the-default-step) exactness).

## The starting incumbent, the ceiling and the seed

The search starts from the best start of its seeds, and stops as soon as it reaches a proven ceiling. A *seed* is a game
to start from: the game of n − 1, and with `--use-prev` the bounds store's lower-bound game of n, if it has one. A
seed's *survivors* are its promotions that are candidates here and still promotable, taken one by one from the largest
down (all of the store's game survives, being a game of n). A seed's *start* is the best of three promotable sets:

* the *greedy* set: promote every candidate that can be, from the largest down;
* the seed's survivors;
* the *dive*: a rounding of the root's matching, which drops the smallest member of a conflict until the rest can be
  promoted;

improved by a local search, which fills the set greedily, then drops one or two members and refills, until nothing
improves. The greedy set and the dive are made once, and the local search runs once from each distinct set the seeds'
starts have reached. The search starts from the heaviest result (of equals, the store's game or else the game of n − 1),
or from the greedy set or the dive alone with no seed. A seed needs its own start because a local search climbs only as
far as its start leads: the store's game can outweigh the survivors of the game of n − 1 and still be a local optimum,
while those survivors climb higher.

The *ceiling* on the promotion sum, `n + score(n − 1) − in_the_bag`, is proven, and reaching it ends the search (tactic
`prev_cap`). Nothing can beat a set at the ceiling, so the dive, the local search and the starts not yet made are
skipped once a set reaches it, and the result is the same.

*Why the start cannot make an answer wrong.* It only chooses the incumbent, a promotable set: the greedy set, the
survivors, the dive and every move of the local search promote their members one after another ([trust.md](trust.md),
"Files with holes"). The search is exhaustive from any promotable incumbent, because a node is cut only when its bound is
below its need, `best + 1 − sum(promoted)` ("Search state and order"): a heavier incumbent prunes only subtrees that
cannot beat a set already held. A start at the ceiling proves the game by it, as no promotable set is heavier.

*Details.* The local search runs on the search's threads, the refills of a pass in parallel, and keeps the first
improvement in the order of the sequential pass, so neither it nor the start depends on the thread count. It reads the
ceiling before each pass, so a ceiling given while it runs (through the control, [Engine](engine.md#engine)) ends it as
soon as its set reaches it. The start is offered to the run's control as it is made, so a caller sees the best so far
before the engine starts, and the engine's start is unchanged. With one seed, or seeds that give the same start, the
start and tree are that seed's alone. `board::Board` is the one place that knows the path from the tables to the
frames, the prepared search, the ceiling and the seeds.

*A floor.*  A search can start instead from a *floor* (`search::search_above`): an incumbent worth a given promotion
sum, with no set and no start made.  The search prunes nodes against the floor as against a set of that weight, and a
set it finds heavier becomes the incumbent as usual.  A search that completes with none found has proven that nothing is
heavier than the floor ([the floor lemma (R5)](../../../docs/theory/search-rules.md#the-floor-lemma)), which
`SearchResult::floor_held` says.  Its tree is that of a search from an incumbent of that worth: at P\*
the proof's tree, node for node, as a search seeded with an optimal set makes it; above P\* a smaller one, by a factor
of about 27 for each quarter of the gap between the root's bound and P\*, on the games measured ([searches under a
floor](../../notebook/search/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md)).

## The near-repeat cut

Consecutive games often pose nearly the same problem, and the proven game before closes most of such a search.
The cut ends a node whose completions are all promotion sets of the problem Π_m of a game m proven before, once the
incumbent reaches a proven bound Z of P\*(Π_m).  [The near-repeat lemma
(R7)](../../../docs/theory/search-rules.md#the-near-repeat-lemma) proves it under three conditions, which the search
checks (tactic `near_repeat`, on by default; `search::NearRepeat`).

* **Setup.**  The solver takes as m the game of n − 1, once that game is concluded, if it is proven and assumes nothing
  (`solver::near_repeat`; [what a game assumes](../REFERENCE.md#assumptions)).  Π_m is set up as Π_n is, Z is
  score(n − 1) less its `in_the_bag`, and the candidates of Π_n that are candidates of Π_m are marked, and among them
  those with the same home in both.
* **Arming.**  A search that starts after the game of n − 1 is concluded is armed with its go; one that starts while
  that game is in the window is armed when it is concluded, beside its ceiling (`Job::arm`).  The cut sits in a lock
  set once, so it is armed at most once, and never by a provisional game.  Each searcher picks it up at its next unit,
  and the nodes searched before are only not cut.
* **The incumbent.**  A node is cut only where the incumbent, as its step read it, reaches Z, as it then does at every
  later node.  A search armed late,
  seeded with a provisional game of n − 1 that the final game improved on, waits for its incumbent to reach Z: through
  the final game's seed, offered with its ceiling, or its own finds.
* **At a node,** after the rule of symmetric pairs and before the bound: (1) every promotion is marked, and every
  candidate left is marked as having the same home; (2) the promotions are promotable in Π_m, which gives their state
  there; (3) on every frame j that is the home h of a candidate left, or h − 1, every move of Π_m's state is a move of
  Π_n's, and every factor y of Π_n's state that Π_m's lacks is no candidate left, and no number of its list
  `composites(y)` in Π_n's factor graph is a move of Π_m's frame j or a candidate left.  That list holds y's maximal
  composites among frame j's moves and frame j + 1's factors as setup forms them: every move of Π_n's state and
  candidate of home j + 1 that y pays for, and no other candidate.  The factor graphs need no check: on a state's frame
  both list the maximal factors among its factors.
* **The record.**  A search armed with the cut counts the nodes it cuts, as `stats.near_repeat_cut` in its results line
  ([the reference](../REFERENCE.md#a-results-line---out-schema-5)).  The proof stays `exhausted`.
* **Cost.**  Below Z the cut costs one comparison, and where the first check fails a few set operations.  A node's
  state in Π_m is found only where it passes, once, from its parent's (one promotion) or from Π_m's root, and kept
  beside the node's own.

The cut gives up only completions no heavier than the incumbent, and learns nothing, so the search stays exact.  It
does not treat the one-sided candidates of extended mode.  A checkpoint records the tactic in bit 11 of its tactics,
and the bit binds nothing: each run sets its cut up again, as it takes its ceiling, so a checkpoint resumes with the
cut on or off.
