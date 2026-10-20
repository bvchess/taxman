# The GF(2) bound and prime exchanges

Part of the [design](../DESIGN.md).

## The GF(2) bound

The *GF(2) bound*, the default node bound (tactic `gf2`), relaxes winnability less than matchability does, and so prunes
more: at the roots of 880 to 1300 its gap is about a fifth of the matching bound's ([matroid intersection
bounds](../../notebook/search/matroid-intersection-bounds-2026-10/README.md)).  In each frame, give the factors
coordinates over GF(2), each *mover* (a move of the frame, or a number promoted into it) the vector of its maximal
factors in the frame, and each number promoted out of it its own unit vector.  A winnable frame peels completely, and
the peel's order makes these vectors linearly independent.  The moves of a frame are fixed, so its condition is a linear
matroid on the candidates that touch it, contracted by the moves, whose rank is the frame's freedoms.  A candidate of
home `h` touches frames `h` and `h − 1`, one even and one odd.  So the even frames' matroids are the blocks of a matroid
`A` on the candidates, the odd frames' the blocks of a matroid `B`, and every promotable set is independent in both.
The heaviest common independent set of at most `max_add` members (weighted matroid intersection, solved in polynomial
time), the node's *heaviest set* `I*`, bounds its completions.

[THEORY.md](../../../docs/theory/bounds.md#the-frame-matroids-a-bound-on-the-promotions), Theorem 11, has the proof; the
theorem is the frame model's form of Theorem 10, the parity matroids' bound on the whole game, which the prover uses.
The module comments of `src/gf2.rs` and `src/gf2/meet.rs` have the algorithms.  The GF(2) bound is never above the
matching bound, since independent vectors have a system of distinct representatives, which is a matching.  What it
misses are the *odd* cores of movers with three or more factors.

* **Kept along the path.**  `gf2::Gf2Graph` builds the root's matroids once a search, and `gf2::Gf2Bound` keeps a level
  for each node of the searcher's path.  A child's level is its parent's, contracted by what the child promoted (one
  pivot per member and matroid), with the candidates it lost deleted.  The intersection (`gf2::meet`: Lawler's shortest
  augmenting paths, over a tableau of bitset rows that a pivot updates per exchange) starts from the last of the sets
  that the parent's augmentations passed through and that the changes keep.  Each such set is the heaviest of its size
  and heavier than all smaller ones, and when the set kept is the parent's heaviest set, the child inherits it with no
  solve.  The parent here is the node as it was before it went on without the member the child promotes (the search
  keeps it aside for that), so that the child finds the node's heaviest set among those sets.
* **Tableaux derived, not built.**  A solved level needs its tableau for its weight split even when the workspace no
  longer holds it: a child that inherited its heaviest set or was certified, or a node going on without the member it
  branched on.  A solve from `I*` would build the tableau, a pivot per member over its block.  `Meet::derive` makes the
  same tableau (the same free elements, circuits, slots and lists, which is all that is read of a tableau outside a
  solve) from a related one, at a cost linear in the elements.  It derives a child's from the parent's, which the
  workspace still holds when the child is checked: the promoted members' slots are dropped from the circuits, a
  contraction, and for a certified child the set is first flipped along its cycle by exchanges, each a pivot that keeps
  the span.  It derives a node's from a copy of its own tableau, saved when it kept the certified level (`Snapshot`).  In
  a block whose span changes, the circuits that hold no member leaving carry over, and the other elements are reduced
  by the block's new members, as are the elements the parent did not have (its own loops, candidates again in the
  child).  The docs of `Meet` in `src/gf2/meet.rs` ("Deriving a tableau") have the argument, and the random paths of
  the tests check every derived tableau against the one a solve builds.  The trees are the same.
* **The heaviest set without a member, with no solve.**  In practice the heaviest set without a member `x` of `I*` is
  `I*` flipped along one cycle through `x` of its exchange graph.  `Meet::without` finds the shortest such cycle by
  Dijkstra's search over the weight split's reduced lengths (below), checks that the flipped set is common independent,
  and certifies it the heaviest when what the sets of other sizes lose against `I*` is at least the cycle's length.  The
  proof: the symmetric difference of `I*` and any set without `x` splits into cycles and paths of the exchange graph,
  all of non-negative length but the one through `x` (the module comment of `src/gf2/meet.rs`).  What the smaller sets
  lose comes from the level's history (a dominant set one smaller), from a floor carried along the path, or from a
  search for the shortest path between members; what the larger sets lose is the solve's last shortest path.  A child
  that loses one member of its parent's heaviest set besides those it promotes gets its own the same way, from the
  parent's exchange graph with the dropped elements banned from the cycle.
* **Forced members.**  The weight split of `I*` bounds the intersection without each member in one pass, by weak
  duality, and a member without which that bound is below the need is forced, as the matching's are.  The split is
  Frank's: `w = w1 + w2`, with `I*` heaviest in `A` for `w1` and in `B` for `w2`, found from difference constraints by
  Bellman–Ford, each round from the vertices that changed.  It starts from the last split of the same node, or else of
  its parent, wherever the path of predecessors that gave a potential is still a path (`Meet::split_bounds_from`), and
  reaches the same split, the shortest distances.  Its bound is loose, so for the members it leaves free within the
  tactic `force_window` of the need (all of them, by default), the exchange graph decides.  A member is forced too when
  no cycle through it is shorter than the slack `value − need` and the sets of other sizes lose more than that
  (`Meet::has_cycle_within`).  A member on a cycle that the search for another found within the slack is not searched,
  since that cycle passes through it.  Nor is a member on a cycle that the last searches at the parent's depth, the
  node's own or its child's found, if it is still a cycle of this exchange graph within the slack (`Meet::cycle_length`,
  a bit test an arc).  A child's exchange graph is its parent's less what it promoted or dropped, so a child that
  inherits its parent's heaviest set finds nearly all its members free on its parent's cycles ([what a GF(2) step
  carries from the last](../../notebook/search/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md)).
  The forced members are the same.  The window `all` costs each node more, since every member is checked exactly, and
  the trees fall by more: at the end of the chain it takes 0.41 to 0.43 of the nodes for 0.60 to 0.63 of the CPU of the
  former window, 80 (`gf2::LEGACY_FORCE_WINDOW`; [the tuning
  campaign](../../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)).  The searches reach
  the elements free in `A` from the members, and the members from the elements free in `B`, through one hub each (the
  least distance plus `w1`, less `w2`), which keeps their distances, order and cycles.  Their queue is a radix heap of
  packed (distance, vertex) keys, which come out in the binary heap's order.
* **Exact strong branching.**  The member of the conflict to branch on is the one without which the intersection is
  lowest.  The search reads each such bound off the exchange graph where it certifies it, and solves it where not: in
  the order of the split's bounds, each solve stopped once it reaches both the lowest so far and the need, after a
  greedy lower bound.  A member whose bound falls below the need is forced too, and the chosen member's level, certified
  or solved, is what the node goes on with.  The cheaper choices (the split's bound alone, the largest member) take more
  nodes and more time ([the GF(2) bound in the
  search](../../notebook/search/the-gf2-bound-in-the-search-2026-10/README.md)).  The tactics `branching`,
  `strong_limit`, `strong_depth`, `deep_branching` and `more_forced` choose among them, and set how far strong branching
  goes ([tactics.md](tactics.md)).
* **What it replaces.**  With `gf2` the search does not compute the matching bound, which as a first check before the
  intersection costs more than the cuts it saves; so `match_refine` has no effect.  The conflicts, nogoods, forced
  members and exact sets work as with the matching bound.  The Lagrangian bound over the nogoods still applies after the
  GF(2) bound when it is on, but changes no tree of the GF(2) search measured and costs 2 to 4%, so `lagrange` is off by
  default.
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

## Prime exchanges

The rule of symmetric pairs (tactic `symmetry`, on by default) prunes the parts of the tree that are mirror images of
others.  Exchanging two primes `p < q` in every number (`p^a q^b m ↦ p^b q^a m`) is an automorphism of divisibility.
Where the exchange maps every frame of the root onto itself (moves to moves, factors to factors) and the candidates onto
themselves, the pair is *symmetric*, and the exchange maps promotion sets to promotion sets worth `(q − p)(c_p − c_q)`
more.  Here `c_r(S)` is the sum of `x / r` over the members `x` of `S` divisible by `r`; in the pairs used, no candidate
is divisible by `pq`, `p²` or `q²`.  So every heaviest promotion set keeps `c_p ≤ c_q` for every symmetric pair at once:
a set that breaks one pair's inequality is beaten by its image under that pair's exchange alone
([docs/THEORY.md](../../../docs/theory/bounds.md#prime-exchanges-symmetries-of-the-promotions), Theorem 13).

The pairs come in classes of primes that the frame builder treats alike.  At 1300 the classes are {41, 43}, {59, 61},
{67, 71}, {73, 79}, {89, …, 107}, {109, …, 139} and {149, 151, 157}, which make 32 pairs; there are 99 at 2175 and 300
at 4000.  `search::symmetry::root_pairs` finds them when a search is set up, in under a millisecond: it makes the full
check only for the primes whose multiples have the same places and exponents in the frames, and only over the multiples.
[Dominance and symmetry](../../notebook/search/dominance-and-symmetry-2026-10/README.md) found the exchanges, and [prime
exchanges in the search](../../notebook/search/prime-exchanges-in-the-search-2026-10/README.md) put them in.

The search uses the pairs through the cofactor sums of a node's promotions `P` and candidates `C`
(`Symmetries::exclude`, a few hundred bit tests: under 1% of the time of a proof of 1472):

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
the best of those that keep the inequalities, which can be lighter.  The matching path (`SearchTactics::matching`)
runs without the rule.
