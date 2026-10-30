# The GF(2) bound and prime exchanges

Part of the [design](../DESIGN.md).

## The GF(2) bound

The *GF(2) bound*, the bound of every node of the search, relaxes winnability less than matchability does, and so
prunes more: at the roots of 880 to 1300 its gap is about a fifth of the matching bound's ([matroid intersection
bounds](../../notebook/search/matroid-intersection-bounds-2026-10/README.md)).  In each frame, give the factors
coordinates over GF(2), each *mover* (a move of the frame, or a number promoted into it) the vector of its maximal
factors in the frame, and each number promoted out of it its own unit vector.  A winnable frame peels completely, and
the peel's order makes these vectors linearly independent.  The moves of a frame are fixed, so a set of candidates is
*independent in the frame* when the classes of its members that touch it, modulo the span of the moves' vectors, are
independent, and the frame's freedoms bound its size.  A candidate of home `h` touches frames `h` and `h − 1`, one even
and one odd.  So independence in every frame is two conditions on the candidates, the even frames' blocks (side `A`) and
the odd frames' blocks (side `B`), and every promotable set meets both.  The heaviest set of at most `max_add` members
that is independent in every frame (found by weighted matroid intersection, solved in polynomial time), the node's
*heaviest set* `I*`, bounds its completions.

[The frame bound](../../../docs/theory/frame-bounds.md#the-frame-bound) has the proof; it is the frame model's form of
[the parity bound](../../../docs/theory/parity-bound.md#the-parity-bound), the prover's bound on the whole game.
The module comments of `src/gf2.rs` and `src/gf2/meet.rs` have the algorithms.  The GF(2) bound is never above the
matching bound, since independent vectors have a system of distinct representatives, which is a matching.  What it
misses are the *odd* cores of movers with three or more factors.

* **Kept along the path.**  `gf2::Gf2Graph` builds the root's two sides once a search, and `gf2::Gf2Bound` keeps a level
  for each node of the searcher's path.  A child's level is its parent's with every vector reduced modulo the vectors of
  what the child promoted (independence together with them), and with the candidates it lost left out.  The promoted
  members of each block become rows with a pivot each, and each vector of the block is reduced by the rows of the pivots
  it has (`contract_all`): the vectors that one pivot per member and side would make, in one pass over the block, where
  a child that promotes many members, its node's forced members, made a pass for each.  A candidate that the reduction
  leaves with the vector 0 (in no set independent in every frame) leaves the level's live candidates and stays out below
  it.  The intersection (`gf2::meet`: Lawler's shortest augmenting paths, over a tableau of bitset rows that a pivot
  updates per exchange) starts from the last of the sets that the parent's augmentations passed through and that the
  changes keep.  Each such set is the heaviest of its size and heavier than all smaller ones, and when the set kept is
  the parent's heaviest set, the child inherits it with no solve.  The parent here is the node as it was before it went
  on without the member the child promotes (the search keeps it aside for that), so that the child finds the node's
  heaviest set among those sets.
* **Tableaux derived, not built.**  A solved level needs its tableau for its weight split even when the workspace no
  longer holds it: a child that inherited its heaviest set or was certified, or a node going on without the member it
  branched on.  A solve from `I*` would build the tableau, a pivot per member over its block.  `Meet::derive` makes a
  tableau that reads the same (the same free elements, circuits and lists, which is all that is read of a tableau
  outside a solve; its slots, which only name the members, may be numbered otherwise) from a related one, at a cost
  linear in the elements.  It derives a child's from the parent's, which the workspace still holds when the child is
  checked: the promoted members' slots are freed and dropped from the circuits, a contraction, and for a certified child
  the set is first flipped along its cycle by exchanges, each a pivot that keeps the span.  It derives a node's from a
  copy of its own tableau, saved when it kept the certified level (`Snapshot`): the workspace's arrays that hold each
  element's circuit and whether it is free, copied whole and taken back by exchanging them with the workspace's.  A
  derived tableau keeps no residues, which only a solve's pivots read: outside a solve a residue is read only for being
  zero, which the flag of a free element says.  In a block whose span changes, the
  circuits that hold no member leaving carry over, and the other elements are reduced by the block's new members.  A
  child that keeps its parent's heaviest set, whose tableau only loses the promoted members and the elements it
  dropped, is derived with no marks, exchanges or reductions (`Meet::derive_by_contraction`).  The columns of `A` (the
  elements outside the set by the members of their circuits, which the split and the searches read) are derived too
  where the workspace has the source's: the elements no longer live leave them, and those whose circuit changed move
  (`Meet::derive_columns`).  The docs of `Meet` in `src/gf2/meet.rs` ("Deriving a tableau") have the argument, and the
  random paths of the tests check every derived tableau, and its columns, against those a solve builds.  The trees
  are the same.
* **The heaviest set without a member, with no solve.**  In practice the heaviest set without a member `x` of `I*` is
  `I*` flipped along one cycle through `x` of its exchange graph.  `Meet::without` finds the shortest such cycle by
  Dijkstra's search over the weight split's reduced lengths (below), checks that the flipped set is common independent
  (in a block where every element that comes in is in the span of `I*`, by a few rows of their circuits, else by a span
  of the block's members), and certifies it the heaviest when what the sets of other sizes lose against `I*` is at least
  the cycle's length.  The proof: the symmetric difference of `I*` and any set without `x` splits into cycles and paths
  of the exchange graph, all of non-negative length but the one through `x` (the module comment of `src/gf2/meet.rs`).
  What the smaller sets lose comes from the level's history (a dominant set one smaller), from a floor carried along the
  path, or from a search for the shortest path between members, which the search for forced members makes only as far as
  its slack: whether the smaller sets lose more than the slack is all it asks, and a loss found no shorter is kept as a
  floor (`Meet::smaller_loss_within`).  What the larger sets lose (the gap) is the length of the shortest augmenting
  path, found only where a floor carried along the path does not decide: a child that keeps its parent's heaviest set
  has the parent's gap as its floor, and a node's level without a member the node's less the cycle's length.  That holds
  of a gap with no path at all too, where no common independent set is larger than the heaviest set, as at every level
  of the units of 4984 measured: the levels below have none either.  A child that a warm start found by deletions
  (below), and a node that went on without a member by one, take both floors from the level they came from, less what
  the heaviest set lost, where its size allows (`floors_after`): with what the child promoted, each of its sets is one
  of that level's, so a set smaller than its heaviest set is smaller than that level's too where its heaviest set, with
  the promoted, is no larger, and a larger set is larger where it is no smaller
  ([the root
  gap](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-28-floors-from-the-level-before-offers-dropped-before-their-bounds-and-copies-swapped-back)
  has the measurements).  Both searches run over the split's reduced lengths,
  by A* with the bounds of the searches for forced members below (`Meet::smaller_loss`, `Meet::gap_with`); debug builds
  check each against Dijkstra's search and Bellman–Ford's.  A child that loses one member of its parent's heaviest set
  besides those it promotes gets its own the same way, from the parent's exchange graph with the dropped elements banned
  from the cycle.  Conflict branching prices a hundred members of a node this way
  ([search.md](search.md#conflict-branching)), and a child that inherits its parent's heaviest set carries its parent's
  prices (`Gf2Bound::prices_excluding`): its exchange graph is its parent's less the elements it promoted or dropped,
  since contracting a member keeps the circuits of the others, so every cycle of the child's graph is one of the
  parent's, as long, and a member's shortest cycle at the parent that is still a cycle of the child
  (`Meet::cycle_length`) is its shortest there.  Such children hold three quarters of the pricing's searches, nearly all
  of which carry, and a step of the units of 4875 and 4914 costs 0.66 to 0.81 of the CPU, with the same trees ([cheaper
  steps of conflict branching](../../notebook/search/cheaper-steps-of-conflict-branching-2026-10/README.md)).
* **Warm starts across deletions** (tactic `warm_start`, on by default).  A child that drops members of its parent's
  `I*` keeps only the dominant sets from before those members came in, a few members in the hardest games, and solves
  from there; so does a node that goes on without a member of its own `I*` with no level kept for it.  With the tactic,
  such a level deletes the members from `I*` one at a time instead, with no augmentation.  `Meet::without_any_size`
  finds the shortest cycle through the member in the exchange graph of `I*` with two elements of weight 0 added, each
  independent of everything: one outside the set, through which a cycle is a path that makes a set one smaller, and one
  in it, through which a cycle is a path that makes a set one larger.  `I*` flipped along that cycle is the heaviest set
  without the member of every size, with no bound needed on what the other sizes lose (the comment of `without_any_size`
  has the proof).  A child's second member is deleted in its own problem, from the tableau of the set so far, derived
  from the parent's, and its weight split, which starts from the parent's split and each later one from the one before.
  Each deletion only lowers the set, so the search, which cuts a child whose bound is below its need, stops a child's
  deletions at the first set below it (`Gf2Bound::push_all_at`); the first deletion, in the parent's problem, is made
  before the child's vectors are, which it does not read.  Nor does a deletion look for a set below the need
  (`Meet::without_any_size_reaching`): its search stops at the cycles longer than the set's weight less the need, where
  it would go on to the member's own weight, and below the need the child is cut with its bound given as the need less
  1, an upper bound on it.  The cycle it finds otherwise is the one the search with no need finds, which takes the same
  vertices in the same order up to it.  The bound is the same, but where several sets weigh the same another may be
  found, and with it another conflict branched on, so the trees can change ([the root
  gap](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-warm-starts-by-deletion)
  has the measurements).
* **Forced members.**  The weight split of `I*` bounds the intersection without each member in one pass, by weak
  duality, and a member without which that bound is below the need is forced, as the matching's are.  The split is
  Frank's: `w = w1 + w2`, with `I*` heaviest in `A` for `w1` and in `B` for `w2`, found from difference constraints by
  Bellman–Ford, each round from the vertices that changed.  It starts from the last split of the same node, or else of
  its parent, wherever the path of predecessors that gave a potential is still a path (`Meet::split_from`), and reaches
  the same split, the shortest distances.  Where the level's tableau derives from the start's (`Meet::repair_from`),
  `derive` has marked the elements whose circuit, freedom or side changed in each matroid since, and the constraint
  graph among the others is the start's.  The paths that break are then found from those elements and the ones no
  longer live, down the forest of the start's predecessors, and only the vertices whose paths broke and the changed
  elements are pulled along their arcs in and relaxed in the first round.  Elsewhere every vertex's path is walked, and
  where the set is the start's less the members promoted since, the first round relaxes only the arcs into and out of
  the vertices whose paths broke.  Its bound is loose, so for every member the exchange graph decides.  A member is
  forced too when no cycle through it is shorter than the slack `value − need` and the sets of other sizes lose more
  than that (`Meet::has_cycle_within`).  The searches find every member forced that the split's bound shows forced,
  since a cycle through a member leaves it by one arc and comes back by another, which together are at least what the
  bound takes off without it.  So the bounds are found only where the searches cannot run, and else one member
  at a time where strong branching orders a conflict's members by them (`Meet::split_bound_without`, from the member's
  column of `A` and its block of `B`), which saves a pass over every circuit.  Where many members are left to search,
  one pass over the circuits of `B` finds the least arc into each (`Meet::closing_lengths`), and a member's column the
  least arc out of it (`Meet::opening_length`): a member whose two arcs are longer than the slack is forced with no
  search, and the search for each other offers no vertex a distance from which no cycle within the slack can close
  ([the root
  gap](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-a-cheaper-search-for-forced-members)
  has the measurements).  Each search is A*: it takes the vertices in the order of their distance from the member plus
  a lower bound on the path back to it, an element's least arc out where a member's column offers it, else 0, and so
  finds the shortest cycle within the slack, which serves more of the levels after than the first one found would
  ([an A* search](../../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-an-a-search-for-forced-members)).
  An element's bound is found only where its offer could be made whatever the bound, below the slack and the shortest
  cycle found: the columns a search reads offer few of their elements.
  A member on a cycle that the search for another found within the slack is not searched, since
  that cycle passes through it.  Nor is a member on a cycle that the last searches at the parent's depth, the node's own
  or its child's found, if it is still a cycle of this exchange graph within the slack (`Meet::cycle_length`, a bit test
  an arc).  A child's exchange graph is its parent's less what it promoted or dropped, so a child that inherits its
  parent's heaviest set finds nearly all its members free on its parent's cycles ([what a GF(2) step carries from the
  last](../../notebook/search/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md)).  The forced members
  are the same.  Checking every member exactly costs each node more, and the trees fall by more: at the end of the
  chain it takes 0.41 to 0.43 of the nodes for 0.60 to 0.63 of the CPU of a window of 80 members within the need, the
  former tactic `force_window`, so a search checks them all ([the tuning
  campaign](../../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).  The searches reach
  the elements free in `A` from the members, and the members from the elements free in `B`, through one hub each (the
  least distance plus `w1`, less `w2`), which keeps their distances, order and cycles.  An element free in `B` that
  would not lower the hub is not offered, since it reaches no member sooner; the search for the loss of the smaller
  sets offers none.  Their queue is a radix heap of packed (distance, vertex) keys, bucketed by distance and the keys of
  one distance sorted once, which come out in the binary heap's order.  A*'s keys are the distance plus the bound,
  with no count of arcs, so they can tie in distance with the last one taken, which the queue allows.
* **A child's bound from its parent's split.**  The same weak duality bounds a child before it is made.  A child that
  promotes members `F` of `I*` asks for sets `J` independent in every frame together with `F`, so each makes `J ∪ F` a
  set of the parent's problem, with no candidate the child lacks.  So `w1(J) + w2(J)` is at most the heaviest set
  independent on side `A` for `w1` plus that on side `B` for `w2`, each among the sets of the child's candidates that
  are independent with `F`, and each of those is greedy: two passes over the child's candidates, with no solve and the
  workspace's tableau untouched (`Gf2Bound::child_bound`).  The look of `conflict_look` cuts its child where this is
  below the child's need, and solves the child only where it is not ([search.md](search.md#conflict-branching)).
* **Exact strong branching.**  The member of the conflict to branch on is the one without which the intersection is
  lowest.  The search reads each such bound off the exchange graph where it certifies it, and solves it where not: in
  the order of the split's bounds, each solve stopped once it reaches both the lowest so far and the need, after a
  greedy lower bound.  A member whose bound falls below the need is forced too, and the chosen member's level, certified
  or solved, is what the node goes on with.  A certified bound needs only the length of the shortest cycle, which the A*
  search of the forced members finds (`Meet::shortest_cycle_length`); `Meet::without` then finds the cycle of the member
  whose level the node keeps alone, the shortest with the fewest arcs, whose flip is that level's heaviest set, its
  search bounded by that length.  The cheaper choices (the split's bound alone, the largest member) took more nodes and
  more time ([the GF(2) bound in the search](../../notebook/search/the-gf2-bound-in-the-search-2026-10/README.md)),
  and so did a limit to the members whose bounds are found, and a cheaper choice deep in the tree: they were tactics
  once, and are removed ([tactics.md](tactics.md), the paragraph on the tactics removed).  Every member of a conflict
  is tried, and once a member is found forced the others still are, to find more forced members.
* **What it replaced.**  The matching bound was the node bound before it, with a refinement by the branches of its
  conflicts and a Lagrangian bound over the nogoods after it, the *matching path*.  With the GF(2) bound the 146
  one-thread proofs of the slope study ran 45 times faster with 426 times fewer nodes ([the GF(2) bound in the
  search](../../notebook/search/the-gf2-bound-in-the-search-2026-10/README.md)), and the matching path was removed.
  The search does not compute the matching bound as a first check either, which costs more than the cuts it saves.
  The conflicts, nogoods, forced members and exact sets work as they did with the matching bound.  The Lagrangian bound
  after the GF(2) bound changed no tree measured and cost 3.6 to 4.3% ([the tuning campaign at the end of the
  chain](../../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).
* **Block words.**  A level's vectors are as wide as the root's largest frame rank (its freedoms) needs: one to three
  words of 64 bits on every game to 4095 (ranks up to 169), four from 4740 and five from 6392 (ranks up to 314 at 8191;
  [boards past 4095](../../notebook/boards-past-4095-2026-10/README.md) has the scan of every game).  Each word count is
  a copy of the bound's code (`Levels<W, R>`, R from 1 to `gf2::MAX_WORDS` = 5) for every set width that has it.  Only
  the sets of 128 words have the copies of four and five words, so the narrower widths build as before.  Tests run those
  copies on small games with the vectors padded with zero words (`Gf2Graph::padded`), where they must search the same
  tree.
* **Other weights.**  `meet::Problem` takes weights that are positive and do not decrease with an element's index, as
  the root's candidates, in increasing order, have them; two of its starts rely on that: the start from the heaviest
  uncontended elements, and the greedy lower bound.  A bound that weighs the candidates otherwise (a Lagrangian over the
  nogoods, or a cover LP with this intersection as its inner problem, as the nogoods study suggests) relabels them by
  increasing weight first, with `Gf2Graph::reweighted`.  The levels, the contraction and the split do not depend on the
  order.

## The decomposition bound

[The decomposition bound](../../../docs/theory/frame-bounds.md#the-decomposition-bound) joins the GF(2) bound and the
independence of the frames under signed GF(3) signings through Lagrange multipliers.  It is no tactic of the search: the
study of the multi-field bound computed it ([a bound over GF(2) and GF(3) signed frame
matroids](../../notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md)), and the example
`wall`, removed since, evaluated it at nodes with that study's code, [`examples/wall/fields/` at commit
3ca11bf](https://github.com/bvchess/taxman/tree/3ca11bf/solver/frame/examples/wall/fields).  The code evaluated the
bound exactly in integers, so that its floor bounds every promotion sum whatever produced the multipliers: the weights
are `x·2^16` and the multipliers are rounded to multiples of `2^-16`, the intersection is `gf2::meet`'s and each GF(3)
frame's term is the greedy algorithm over its vectors, all in 64-bit integers, and a sum that would leave them
saturates, which keeps the bound valid.  A floating-point linear program proposed the multipliers by column generation
and was no part of the proof.  The module comment of
[`dw.rs`](https://github.com/bvchess/taxman/blob/3ca11bf/solver/frame/examples/wall/fields/dw.rs) has the details.

## Prime exchanges

The rule of symmetric pairs (tactic `symmetry`, on by default) prunes the parts of the tree that are mirror images of
others.  Exchanging two primes `p < q` in every number (`p^a q^b m ↦ p^b q^a m`) is an automorphism of divisibility.
The pair is *symmetric* when the exchange maps the root onto itself and no candidate is divisible by `pq`, `p²` or
`q²`.  Mapping the root onto itself means that each frame's moves go to its moves, its factors to its factors, and the
candidates to the candidates (the code also asks that the lists of maximal factors that can pay for the frames'
numbers map onto each other, and that `p` divides some candidate).  The exchange then maps promotion sets to promotion
sets worth `(q − p)(c_p − c_q)` more, where `c_r(S)` is the sum of `x / r` over the members `x` of `S` divisible by `r`.
So every heaviest promotion set keeps `c_p ≤ c_q` for every symmetric pair at once: a set that breaks one pair's
inequality is beaten by its image under that pair's exchange alone ([the prime-exchange theorem
(R2)](../../../docs/theory/search-rules.md#the-prime-exchange-theorem)).

The pairs come in classes of primes that the frame builder treats alike.  At 1300 the classes are {41, 43}, {59, 61},
{67, 71}, {73, 79}, {89, …, 107}, {109, …, 139} and {149, 151, 157}, which make 32 pairs; there are 99 at 2175 and 300
at 4000.  `search::symmetry::root_pairs` finds them when a search is set up, in under a millisecond: it makes the full
check only for the primes whose multiples have the same places and exponents in the frames, and only over the multiples.
[Dominance and symmetry](../../notebook/search/dominance-and-symmetry-2026-10/README.md) found the exchanges, and [prime
exchanges in the search](../../notebook/search/prime-exchanges-in-the-search-2026-10/README.md) put them in.

The search uses the pairs through the cofactor sums of a node's promotions `P` and candidates `C` ([the
exchange-pruning lemma (R3)](../../../docs/theory/search-rules.md#pruning-by-prime-exchanges); `Symmetries::exclude`, a
few hundred bit tests: under 1% of the time of a proof of 1472):

* it cuts a node when some pair has `c_p(P) > c_q(P) + c_q(C)`, since no completion lifts `c_q` to `c_p` (the node is
  *hopeless*);
* it drops from the node's candidates a candidate `mp` with `c_p(P) + m > c_q(P) + c_q(C)`, which is in no completion
  that keeps the pair's inequality.  The drops tighten the node's bound, and repeat until nothing more drops, since a
  drop lowers `c_q(C)` of other pairs.  The search makes children without the dropped candidates, a node's later
  steps drop more as the node goes on without the members it branched on, and a dropped candidate leaves the GF(2) and
  matching levels as a branch member does.

The drops are most of the gain: at the proof of 1287 the hopeless test alone leaves 0.97 of the nodes, and the whole
rule 0.57.  A Lagrangian of the inequality on the GF(2) bound (the heaviest set under the weights `m(p − μ)` and
`m(q + μ)` for the pairs that its heaviest set breaks) was measured and not kept: it cut little beyond the drops, and
cost more.

*Why the rule is sound.*  It holds for the search of the root's problem.  If the incumbent is not optimal, every
heaviest set beats it and keeps every inequality, and the other prunings (bounds above every completion, members that
every completion beating the incumbent promotes, sets that cannot be promoted) hold for the completions that keep the
inequalities as for all.  The rule learns nothing: a cut ends a loop and a drop shrinks a candidate set, while the
conflicts and nogoods that the search learns are facts about promotability, true whatever the rule cut.  The rule
changes which conflicts a searcher meets, as any change of the tree does, never their truth.  It is a function of a
node's `P` and `C`, so a unit (a subtree of the root) rebuilt from a checkpoint or on another thread keeps the
guarantee.  A search of one node's completions for their own best, as the soundness harness runs below the root, finds
the best of those that keep the inequalities, which can be lighter.
