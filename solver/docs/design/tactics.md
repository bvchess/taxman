# Tactics

Part of the [design](../DESIGN.md).

A *tactic* is a switchable piece of the search: each row below is one, in `tactics::Tactics`, set from the command line
with `--tactic name=value`.  Every tactic is *proven* (a theorem, given a premise) or *sound* (it never prunes a
promotable optimal set), so switching one off changes how fast an answer comes, never the answer.  The values are `on`
or `off`, but for `match_refine` (`0` to `7`), `nogood_period` and `nogood_limit` (whole numbers), `nogood_keep` (a
policy), and the tactics that tune the GF(2) search, `force_window` to `more_forced`, which their rows give.  The
tactics that act inside the search are the fields of `search::SearchTactics`, and `examples/ladder.rs` takes the same
names for them.  A checkpoint records the search tactics in its header ([trust.md](trust.md), and "Adding a tactic"
below), but for the three that set how the nogoods are kept.  A test (`tests/docs.rs`) checks that this table lists the
tactics of `tactics::Tactics`, in order, with their defaults.  Two of them, `conflict_branching` and `conflict_look`,
only choose how a node branches: a checkpoint records them but does not bind them, and a lab may switch them while a
search runs ([lab.md](lab.md#which-tactics-may-change-while-a-search-runs) lists which tactics may change, and why the
others may not).  A checkpoint does not bind `near_repeat` either, since each run of a search sets its cut up again, nor
`price_reuse`, which only changes how the prices that rank the conflicts are made, nor `carried_nogoods`, whose store
holds only facts of the root, nor `warm_start`, which only changes which of several heaviest sets of the same weight a
node finds.

| tactic | default | status | effect |
|---|---|---|---|
| `prime_shortcut` | on | proven (given an optimal game of n − 1: the proof is at `accelerate::prime_shortcut`) | a prime n plays the game of n − 1 with its first move replaced by n |
| `same_search` | on | proven (given the proof of the earlier game: [same-search.md](same-search.md)) | a game whose search problem an earlier game of the chain posed and settled is that game's promotions on this board, with no search (method `repeat`, proof `same_search`) |
| `match_refine` | 1 | sound (see `search::refine::Refiner`) | where the matching bound does not prune, refine it by branching on conflicts this many levels deep (0 = off, at most 7); no effect with `gf2`, which does not use the matching bound |
| `fix_forced` | on | sound (every completion that beats the incumbent promotes them) | a node's child promotes all its forced members, and the node's loop ends after that child |
| `nogoods` | on | sound (a nogood holds at every node; see `model::Nogoods`) | learn each conflict as a nogood of the root, and use the table at every node: for the conflict to branch on and refine with, and to rule candidates out of children |
| `forget_nogoods` | on | sound (the table is a cache) | with `nogoods`, each searcher forgets every nogood it knows after each `nogood_period` of its steps, or keeps some (`nogood_keep`); off: never |
| `nogood_period` | 50000 | sound (the table is a cache) | with `nogoods` and `forget_nogoods`, the steps of a searcher between its forgettings (a whole number from 1); a research knob, with the next two ([search.md](search.md#forced-members-and-nogoods), the item "Forgetting nogoods") |
| `nogood_keep` | none | sound (the table is a cache) | what a forgetting keeps: `none` (nothing), `hit` (the nogoods used since the last forgetting), `sizeN` (those of at most N members), `topN` (the N most used since the last forgetting, of those used at all) |
| `nogood_limit` | 0 | sound (the table is a cache) | with `nogoods`, a searcher's table holds at most this many nogoods, the least recently used dropped to make room for a new one (0: no limit) |
| `lagrange` | off | proven (weak duality, for any multipliers over valid nogoods; `matchbound::lagrange`) | with `nogoods`, where the node bound leaves a node open, bound it also by a Lagrangian relaxation of the searcher's nogoods kept along its path, over the matching bound, and (with `fix_forced`) force the members without which that bound would cut; the multipliers are refitted at the root as the searcher learns ([search.md](search.md)); with `gf2` it changes no tree and costs 2 to 4% |
| `gf2` | on | proven ([the frame bound (B2)](../../../docs/theory/frame-bounds.md#the-frame-bound); weak duality for the forced members) | bound every node by the GF(2) bound ([search.md](search.md)) instead of the matching bound and its refinement, branch on the conflicts of its heaviest set by exact strong branching, and (with `fix_forced`) force the members without which it would cut; the Lagrangian bound over the nogoods, if on, applies after it |
| `symmetry` | on | proven ([the prime-exchange theorem (R2)](../../../docs/theory/search-rules.md#the-prime-exchange-theorem)) | cut a node whose completions all break a symmetric pair's inequality, and drop from a node's candidates those that no completion keeping the inequalities holds ([Prime exchanges](gf2-bound.md#prime-exchanges)) |
| `force_window` | all | sound (a member is called forced only where the exchange graph shows every completion without it short) | with `gf2` and `fix_forced`, which members of a node's heaviest set that the weight split leaves free the exchange graph searches for being forced: those whose split bound is less than this far above the need (`80`; `0`: none), less than a percentage of the node's slack above it (`25%`), or `all`; a wider window forces more members, for a smaller tree, at the cost of more searches ([gf2-bound.md, "The GF(2) bound"](gf2-bound.md#the-gf2-bound), has the measurements of `all` against `80`) |
| `branching` | exact | sound (every completion leaves out some member of the conflict) | with `gf2`, how a node chooses the member of its conflict to branch on: `exact` (the bound without each member, certified or solved), `forced` (each solved only as far as the need, the branch by the split's bound), `split` (the split's bound alone), `largest`; the cheaper choices take more nodes |
| `strong_limit` | all | sound (as `branching`) | with `gf2`'s strong branching (`exact` or `forced`), the most members of a conflict whose bounds are found, the most promising by the split's bound first |
| `strong_depth` | all | sound (as `branching`) | with `gf2`, a node with at least this many promotions branches as `deep_branching` says instead of `branching` (`all`: none does), so that the deep nodes, the most numerous, can choose more cheaply |
| `deep_branching` | split | sound (as `branching`) | the choice of `branching` at the nodes `strong_depth` names |
| `more_forced` | on | sound (as `fix_forced`) | with `gf2`'s strong branching and `fix_forced`: once a member is found forced, the bounds of the other members are still found, to find more forced members |
| `conflict_branching` | on | proven (every completion leaves out a member of any conflict; the conflicts it makes are conflicts by [the peel theorem (P4)](../../../docs/theory/playability.md#the-peel-theorem), [the residual theorem (P5)](../../../docs/theory/playability.md#the-residual-theorem) and [the exactness theorem (X7)](../../../docs/theory/exactness.md#the-exactness-theorem)) | with `gf2`, where a check's gap is at least 300, branch on the conflict of the heaviest set whose members cost the bound most to leave out, instead of the smallest the search knows ([search.md, "Conflict branching"](search.md#conflict-branching)); on the whole games 4050 and 4176, on one thread, a fifth of the steps and a quarter of the CPU |
| `conflict_look` | on | proven (as `conflict_branching`: it only chooses between two true conflicts of a node, and leaves the node's state as it found it) | with `conflict_branching`, where a check would branch on a conflict other than the search's own, look first at the last child of the search's own conflict, and branch on the search's own conflict if that child is cut on sight ([search.md, "Conflict branching"](search.md#conflict-branching), the paragraphs on the look); on the units of the games near 4900, 0.61 to 0.87 of the steps; `off` branches on the conflict chosen with no look |
| `near_repeat` | on | proven ([the near-repeat lemma (R7)](../../../docs/theory/search-rules.md#the-near-repeat-lemma), given a proven bound of the earlier game's best promotion sum; the solver takes it only from a game that assumes nothing) | cut a node whose completions are all promotion sets of the game of n − 1, once that game is concluded and proven (with the search's go, or while it runs) and the incumbent reaches its best promotion sum ([search.md, "The near-repeat cut"](search.md#the-near-repeat-cut)); it ends many searches of the games that pose nearly the problem of the game before at their first step |
| `price_reuse` | off | proven (as `conflict_branching`: the prices only rank true conflicts of a node, so any ranking keeps the search exact) | with `conflict_branching`, keep the prices of a node's members that the choice of the conflict makes, take them again at the node's later checks and at its descendants' first checks, and make only the prices a check lacks; on the 315 units of games 4920-4949 it takes 0.82 (hard games) and 0.86 (easy games) of the CPU, and 0.91 of the steps ([price-reuse.md](price-reuse.md)) |
| `carried_nogoods` | off | proven (each carried nogood is checked at the root, so it holds at every node as the search's own do; the cut rests on the forced members, as `fix_forced`) | use the nogoods carried from another search, each kept only if a peel at the root finds a conflict within it, only to prune: rule candidates out of a unit's node and of children, and end a node whose forced members complete one; never to choose the conflict a node branches on; it acts only where the search is given a store (`taxman solve --carried-dir DIR`, which turns it on, or `tune solve --carried FILE`; [carried-nogoods.md](carried-nogoods.md)); on 71 units of 4984 with conflict branching off, a store from 144 units of 4983 takes 0.62 of the steps and 0.85 of the CPU, with it on 0.87 and 0.97 |
| `warm_start` | off | proven (the bound is the heaviest weight, found exactly; only which of several heaviest sets is found can differ, and with it the true conflict branched on) | with `gf2`, a child that drops members of its parent's heaviest set, and a node that goes on without a member of its own, find the new heaviest set from the old one, deleting the members one at a time along the shortest cycle through each in an exchange graph that takes in the sets of every size, instead of augmenting from a set a few members large ([gf2-bound.md](gf2-bound.md#the-gf2-bound), "Warm starts across deletions") |
| `prev_cap` | on | proven, given an optimal game of n − 1 | stop as soon as the incumbent reaches the ceiling, n + score(n − 1) − in_the_bag as a promotion sum |
| `seed_incumbent` | on | sound (any promotion set is a valid incumbent) | start from the best of the greedy set and the promotions of each seed (the game of n − 1, the bounds store's game of n) that are still promotable |

**The finder is not a tactic.**  `--finder-after` and `--finder-seconds` start a thread beside a long search that looks
for a better incumbent ([finder.md](finder.md)); they are flags of `solve`, in no checkpoint's
header and no row above, so a checkpoint resumes with any value, and a result line records them in `options`, not in
`tactic_switches`.

**The frame model's own rules are not tactics.**  They define the search space, and cannot be switched off inside the
model: the greedy removal of impossible sources inside frames; the mandatory moves; freedoms = factors − sources; closed
frames taking no part in the search; and a promoted number paying tax only from the frame one forward.  [frame-model.md,
“What "optimal" means”](frame-model.md#what-optimal-means) says what is proven of them, and what remains: [statement
U](../../../docs/theory/exactness.md#statement-u).

**Where the tactics are recorded.**  A result line records the tactics it was made with: `tactics`, the changes from the
defaults (such as `lagrange=on,gf2=off`), and `tactic_switches`, every tactic's value.  A line of a build before `gf2`
has no such key, and ran the matching path.  A checkpoint records the search tactics, and is refused when they differ,
but for `conflict_branching`, `conflict_look`, `near_repeat`, `price_reuse`, `carried_nogoods` and `warm_start`: the
error names each tactic that differs, and the `--tactic` flags that resume it.

*The tactics removed.*  The [notebook](../../notebook/README.md) has what each tactic was worth and the order they were
found in, and the tactics that were removed: the Java's two conjectures, `reuse_prev` (accept the previous game's
promotions with no search) and `revise_freedoms` (lower frame 0's freedoms by solving the two-frame game, assuming that
the heaviest promotion set also has the most members, which is false at n = 833); and the tactics that lost, the
Java's bound with its runner-up, its preclusion table, and the branchings other than strong branching
([which tactics earn their keep](../../notebook/search/which-tactics-earn-their-keep-2026-10/README.md)).

## Adding a tactic

A new tactic changes the code, its tests and this table together:

* **Its name and value**: a slot in `Tactics::slots_mut` (`src/tactics.rs`), in this table's order, with
  `TACTIC_COUNT` and that module's tests of the names and the values.  The command line, the results line and the
  ladder take the names from there.
* **Inside the search**: a field of `search::SearchTactics` (`src/search/tactics.rs`) and of `SearchSpace`
  (`search::searcher`), which the soundness harness builds too (`tests/common/mod.rs`).  A checkpoint's header records
  a switch through `bits`, `from_bits` and `with_switches_of`, and a setting that tunes the search through `tuning`,
  `with_tuning` and `legacy_tuning`; the paragraph "What identifies a checkpoint" of [trust.md](trust.md) says which
  header version a search writes, and `src/search/tests.rs` pins the bits.
* **The tests**: the tactic among the variants of `every_tactic_gives_the_same_scores_at` (`tests/solver.rs`); a
  tactic that prunes, among the configurations of `tests/soundness.rs` and the rows of `tests/tripwires.rs`; and the
  checklist of [testing.md](testing.md) before the merge.
* **The docs**: its row here, with its grade and the reason for it; the mechanism in the design file of its part (the
  [map](../DESIGN.md)); the example results line of [REFERENCE.md](../REFERENCE.md#a-results-line---out-schema-5),
  whose `tactic_switches` lists every switch; and a notebook entry once it is measured.
