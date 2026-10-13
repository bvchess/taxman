# Design: the Rust solver for optimal Taxman

This is the design of the solver as it is now, written to be read from top to bottom: the goals, the game and the
model of it that the solver searches, the architecture, the search and its tactics, the engine that runs it on many
cores, the per-game pipeline, how results are recorded and trusted, how it is tested, and how performance is
measured.  Each term is defined where it first matters.

* The [README](README.md) is the user guide (how to build, run and read the output) and has the glossary.
* [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md) is the lab notebook: every round of improvements with its measurements,
  and the ideas that were measured and not kept.  This page states what the solver does and why; the notebook says
  what it was worth and what was tried instead.
* Each module starts with a comment that says what it is for; [`src/lib.rs`](src/lib.rs) has the module map.

* [1. Goals and constraints](#1-goals-and-constraints)
* [2. The game and the frame model](#2-the-game-and-the-frame-model)
* [3. Architecture](#3-architecture)
* [4. The search](#4-the-search)
* [5. Tactics](#5-tactics)
* [6. Engine](#6-engine)
* [7. Per-game pipeline (`solver`)](#7-per-game-pipeline-solver)
* [8. Results and the trust model](#8-results-and-the-trust-model)
* [9. Testing](#9-testing)
* [10. Performance](#10-performance)
* [11. Status and what is left](#11-status-and-what-is-left)

## 1. Goals and constraints

* Find optimal games beyond n = 1000, where the Java solver's table of optimal games stops, as far as the machines
  at hand reach: today every n to 1231 on one laptop (Apple M1 Max, 10 cores, 32 GB RAM, fast SSD), and the same
  chain on a many-core cloud machine for what is beyond (`tools/cloud/RUNBOOK.md`).
* Be right before being fast.  Every game is replayed by an independent referee before it counts, every result
  records how its optimality is established and what that rests on that is unproven, and every unproven tactic
  can be switched off so that its effect on both answers and speed can be measured.
* Search an enormous tree efficiently: fast per node on a single core, and scaling cleanly to the cores available
  (up to a 192-core machine) without depending on many of them.
* Never rely on everything fitting in RAM.  Memory use is bounded by configuration, independent of tree size; work
  that does not fit goes to disk.
* Every long run checkpoints frequently and resumes after a restart, losing at most one checkpoint interval (see
  the repository's CLAUDE.md); stopped by SIGINT or SIGTERM, it loses nothing.
* Fast iteration: correctness and performance are worked out on small and medium games that finish in seconds
  (the tests, the benchmark set, the probe).  Big runs come last.
* Standard library only, no third-party crates; Linux and macOS.

## 2. The game and the frame model

### The game

The pot holds the numbers 1 to n.  A move takes a number that still has a proper divisor in the pot; the player
scores it, and the taxman takes it and every divisor of it still in the pot.  The game is over when no number in the
pot has a divisor left in it.  An *optimal game* scores the most.  `game::replay` is this rule in code, and
`game::replay_finished` also requires the game to be over and to open with the largest prime, as every optimal game
does; together they are the referee that does not share code with the solver.

### Sources, factors, hives, frames

Every number above n/2 divides nothing else on the board, so it can only be a move, never tax: it is a *source*.
Every other number is a *factor*: tax for a larger number, and sometimes a move itself.  A move m is paid for by one
of its *maximal factors* m/p (p a prime dividing m); and a factor can pay for one move.

Sources and the factors they compete for fall into independent groups, the *hives* (`hives`).  Simplifying a hive
finds the moves it forces (`unzip`), the sources that cannot be played together with the larger ones
(`removeImpossible`), and its *free factor count*, factors minus sources: how many factors it can spare.  Hives with
factors to spare are stacked into levels and merged: each level is a *frame* (`frames`), numbered from 0 at the
bottom.  A frame is a game of *Taxman Mini*: a set of moves and a set of factors, in which every move must still have
one of its maximal factors in the frame when it is taken, and taking it removes all of the frame's factors it uses.
A frame is *winnable* when some order takes every move; a frame is winnable exactly when repeatedly dropping a move
that has a factor no other move has (a *peel*) drops them all, and `model` tests that with counters instead of
searching orders.

### Promotions and the score

What a game can still choose is which of the spare factors to *promote*: to play it as a move instead of leaving it
as tax.  Promoting a factor of frame h makes it a move of frame h − 1 (a promoted number pays tax only from the frame
directly below), and is legal when both frames stay winnable.  The numbers that could be promoted are the
*candidates*; a promotion set is *feasible* when all of it can be promoted from the root.  Afterwards the frames are
*reduced*: a move left with one live factor must be paid by it, so the pair is dropped, repeatedly.  A frame's
*freedoms* (factors minus moves) drop by one for each promotion into or out of it, and no frame may go below zero.

The score of a game of the model is `in_the_bag` plus the sum of its promotions, where `in_the_bag` is what the game
with no promotions scores (the Java's `inTheBag`).  So the solver looks for the feasible promotion set with the
largest sum; the moves of the game follow from the promotions (`hives::Apiary` with the promotions treated as
sources).  A worked example, the board 1..=85, is in the
[README](README.md#how-the-solver-sees-the-game-the-frame-model).

### What "optimal" means

* **Within the frame model.**  The decomposition rests on assumptions that are validated, not proven: the greedy
  removal of impossible sources inside hives, the forced moves of `unzip`, free factor count = factors − sources,
  closed hives (those with no free factor) taking no part in the search, and a promoted number paying tax only from
  the frame directly below.  They are validated by agreement with the Java's optimal.json for n ≤ 1000, which an
  independent exhaustive search (`heuristics/evaluation/certify.py`) confirms for n ≤ 62, and by the Rust exact
  oracle `brute`, which checks frame-model games on small boards (up to 40 in the test suite, 52 in
  `examples/sacrifice validate`).  The sacrifice bound (`sacrifice`) is an upper bound on opt(n) by a
  different route (games on punctured boards); it proves game 1133 and disagrees with nothing.
* **Given the pruning is exact.**  The default search tactics are sound (section 5 grades each).  Two tactics are on
  by default for speed but unproven: `reuse_prev` and `revise_freedoms`.  A run with both off is *strict*.
* **Given the previous game is optimal.**  The game of n is built from the game of n − 1: opt(n) ≤ n + opt(n − 1)
  (deleting the move n from an optimal game leaves a game of n − 1), and a prime n plays the game of n − 1 with
  its first move replaced by n.  A chain of games from n = 1 is therefore proven game by game (section 8).

### The Java solver and this one

The optimization strategy is the Java solver's (and the [wiki's](https://github.com/bvchess/taxman/wiki)): decompose
the game into frames of Taxman Mini and search for the optimal promotion set.  The Java is research code, three
generations of the same frame idea layered on each other, so the Rust is a reimplementation of the *algorithm*,
designed for clarity and speed, not a translation of the classes.  What must match the Java is *results*: the
setup (hives, frames, candidates, the greedy incumbent, revised freedoms) defines the search space, and reproduces
the Java's values exactly (its golden data in `testdata/golden/` is the check); every final score equals
optimal.json.  The search itself is new.  Its baseline is the Java's loop (`branching=largest`, `match_bound=off`,
`blameGame` as the bound); everything the default search does beyond that is a tactic that is measured and
switchable.  Not ported: `oldsearch/` (the 2021 disk-queue search), `frame.GreedySolver`, `util.Memo`,
`WorkingFrame.fancyFit/fits`, the picklers and compressor.

One defect of the Java's bound was found and fixed in the port: `UpperBoundEstimator.getRunnerUp` replaces the
failing number with the next candidate below the smallest bid member, and a skipped candidate from the failing
number's frame can be larger, so the Java bound can be too low (at the root of n = 114: 358 against the best
completion's 360).  The sound runner-up is the default; `legacy_bound` reproduces the Java's ([the
story](docs/EXPERIMENTS.md#the-port-and-its-first-improvements)).

## 3. Architecture

One Cargo crate in `rust/`: a library and one binary, `taxman`, with no third-party dependencies.  `src/lib.rs` has the
module map with a line on each module.  The layers, bottom up:

| layer | modules | responsibility |
|---|---|---|
| foundations | `bits`, `numtheory`, `json`, `game` | fixed-width bitsets; maximal factors and composites; a small JSON reader and writer; the rules as a referee and the table of optimal games |
| frame model | `hives`, `frames`, `model`, `accelerate` | the board as hives and frames; the search state (frames, winnability, promotion, conflicts, nogoods); deriving a game from the previous one |
| search | `matchbound`, `search`, `bounds` | the matching bound; the branch-and-bound searcher with its setup and checkpoint header; the Java's bound, preclusions, greedy incumbent and candidate lists |
| engine | `engine` | the problem-independent parallel depth-first driver |
| solving | `tactics`, `board`, `solver`, `results`, `paths` | the switchable tactics; a board prepared for a search; the per-game pipeline and the range driver; what a result records and the checks of a results file; the default files |
| studies | `brute`, `sacrifice` | exhaustive search to certify small boards; the sacrifice bound and punctured games (not used by the solver) |
| front end | `main.rs`, `cli.rs`, `commands.rs` | `solve`, `regress`, `bench`, `verify-optimal`, `verify-results`, `tables` |

On the default path of a solve are all the layers but the studies and the Java-parity layer `bounds` (of which the
default search uses only the greedy incumbent, `max_additional` and the candidate lists).  `examples/` holds the
ladder (standalone searches of hard games with time limits, thread scaling, diagnostics), the probe `perf` (the hot
paths from fixtures, in cycles and instructions) and the sacrifice study; `tests/` the integration tests (section 9);
`tools/` the results referee in Python, the PGO build, the Java golden export and the cloud scripts.

**Width dispatch.**  Everything that holds sets of game numbers is generic over `const W: usize`, the number of 64-bit
words, and the macro `with_width!` (in `bits`, used by the solver, the examples and the tests) picks the narrowest
of `bits::WIDTHS` = {8, 16, 20, 24, 28, 32, 64} that holds n (n ≤ 511, 1023, 1279, 1535, 1791, 2047, 4095).  Every
set operation is a loop over all W words, so the steps of 4 words near the frontier pay.  Each width is a copy of
the search's code, which costs build time (about 50 s for a release build).  A checkpoint records its width, and a
search resumes at that width if this build has it.

## 4. The search

### Search state and order

The search runs over *nodes*.  A node is `(state: FrameSet, promoted: Bits, cands: Bits, max_add: u32)`: `state` is
the root with `promoted` promoted (in any order: the result does not depend on it), `cands` the candidates the
node's loop has still to try, and `max_add` a bound on how many more can be promoted
(`max_additional = min(max_total − |promoted|, state.max_promotions(cands))`, where `max_total` comes from the
frames' freedoms).  A *work unit*, what the engine passes between threads and writes to checkpoints, is just
`(promoted, cands)`, two bitsets, from which the state is rebuilt in one pass.  Rebuilding recomputes `max_add` from
the remaining candidates, which can only tighten it; and what a searcher has learned along its path (conflicts,
nogoods) is a cache that units do not record, so a rebuilt node may prune a little differently from the node it
replaces, with the same answer.

A node's loop tries its candidates one at a time, each time making a child (the node with one more promotion) and
exploring it depth-first before going on; the search keeps an explicit stack that is reused from node to node (no
recursion, no allocation per node).  The unit of work counted everywhere (`nodes` in the output) is one *step* of a
node's loop, one candidate attempt, not one tree node.

The Java's loop, the baseline (`BranchExplorer.explore`, made lazy so that children are explored as they are made
rather than all generated first):

```
loop while cands non-empty:
    if blame_bound(state, cands, max_add) + sum(promoted) <= best: break
    n = cands.max()                               # the Java's branching
    cands.remove(n)
    new_state = state.promote(n) else continue
    child_cands = new_candidate_list(n, ...)      # narrow, remove preclusions, can_promote filter
    if sum(promoted) + n + blame_bound(new_state, child_cands, ...) > best: push the child
```

The default search keeps this skeleton and changes what bounds a node, which candidate to branch on, and what the
child promotes; the next sections build it up, and "The default step" below puts it together.  A bound equal to
`best` cannot lead to a better incumbent, so loops break then (the Java breaks only below).

### The matching bound

Winnability relaxes to *matchability*.  Each move of a winning order takes at least one factor that is present when
it is taken and removes it, so a winnable frame gives its moves distinct maximal factors.  A feasible promotion set
`Q` therefore gives every move of every frame, and every promoted number (in the frame below its home), a distinct
maximal factor, and no member of `Q` is used as a factor.  Every edge `v → v/p` changes the number of prime factors
by one, so the graph is bipartite and the *matching bound* — the largest sum of a candidate set that has such an
assignment — is a maximum-weight bipartite matching (a min-cost flow), exactly solvable and an upper bound on the
sum of the best feasible completion.  It ignores freedoms and the cap on the number of promotions, which only
remove completions.  Measured at the root of all 94 golden games that need search, its mean gap to the optimum is
31 against 76 for the Java's bound; on the former benchmark games (n = 480–760) 107–292 against 363–769.

It is used incrementally.  `matchbound::MatchGraph` is the root's graph, whose edges never change; every node's
problem is that graph with each vertex in one of three roles (required: the root's moves and the promoted numbers;
candidate; factor).  `MatchBound` keeps an optimal matching and an optimal dual solution (potentials) for every node
on the search's path, and repairs them after each change by Hungarian searches on a radix heap; the changes along a
depth-first path are small, so an evaluation costs 0.7–2.4 µs where a solve from scratch costs 30–170 µs (at n = 480 to
760).  The potentials also give, at once, an upper bound on the matching *without* a given candidate
(`bound_excluding`): lowering that candidate's potential and its partner's as far as their remaining edges allow.  The
module comment of `matchbound.rs` is the best explanation of all this.

With the matching bound as the node bound, `blameGame` is not computed at all: it never pruned where the matching
bound did not (diagnostics: zero at every check).  The bound is an upper bound on `sum(promoted) +` completions, so
a node whose matching value is below `need = best + 1 − sum(promoted)` is cut.

### Conflicts of the matching bound

When the heaviest matching's candidates `Q*` cannot all be promoted, some frame is unwinnable with them.  Let a *peel*
of that frame run: the moves it cannot drop, its *core*, are stuck however the order goes, and the members of `Q*`
that the core needs (promoted into the frame and core moves, or promoted out of it and factors of core moves) cannot be
promoted together.  Minimized by peeling only the core, such a set `K ⊆ Q*` is a *conflict*: a set of candidates that
cannot be promoted together from the node's state (`model::conflict_using`).  Conflicts are tiny (a median of two
members at n = 760–1000, a quarter of them single members).

The idea behind everything that follows: a subset of a promotable set is promotable (moves left out and factors kept
only help a frame), so **every feasible completion leaves out some member of a conflict**.

* **Refinement** (`match_refine`, default 1).  Where the matching bound does not prune, bound the branches of the
  conflict too: with members `k₁ < k₂ < … < k_r`, every feasible completion `F` is in exactly one branch "`k₁ … k_{j−1}`
  in `F`, `k_j` not", and the best completion is at most the largest of the branches' matching bounds (the matching
  with `k_j` no longer a candidate and `k₁ … k_{j−1}` required).  Each branch is refined the same way, one level
  less deep.  By induction the refined bound is never below the best completion (`search::Refiner`).
* **Conflict branching** (`branching`, default `conflict-strong`).  A node branches on a member of the conflict
  instead of its largest candidate: the child promotes it, and the node goes on without it, so that neither can
  reproduce the heaviest matching.  `conflict-strong` picks the member whose removal lowers the matching bound
  most (one bound without each member; with `bound_excluding` most are settled by the potentials alone).  The
  alternatives are the smallest member, the largest, and the Java's order.
* **Exact matchings.**  A heaviest matching that can be promoted is the node's best completion: it is offered as the
  incumbent and the node is done.
* **Propagation into children.**  A conflict known at the node, of which a child promotes all but one member, rules
  that member out of the child's candidates (the child's state is the node's with more promotions, so promoting it
  would promote the whole conflict).  Without it the child's matching bound equals the node's.
* **Starting incumbent.**  See "The starting incumbent, the cap and the seed", below.

### Forced members and nogoods

The tree is mostly chains: at four nodes in five the node's loop ends right after its first child, because
removing the member that strong branching chose already brings the bound below what the node needs.  Such a member
is *forced*: every completion that beats the incumbent promotes it.

* **Forced members** (`fix_forced`).  The child promotes every forced member of the conflict, not only the chosen
  one; the node's loop ends after that child, since the child covers every completion that can beat the incumbent.
  If they cannot all be promoted, or the child's candidate filter dropped one, the child has no such completion at
  all.  They are promoted together, each frame checked once (`Model::promote_all_using`).  The forced members are
  found from the potentials for every member of the heaviest matching, and strong branching solves only where none
  is found; a conflict all of whose members are forced ends the node at once.
* **Nogoods** (`nogoods`).  A conflict `K` found at a node with promotions `P` is relative to `P`; a minimal subset of
  `P ∪ K` that keeps a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from the root, so at every
  node anywhere in the tree, on any thread and after any resume, the nogood minus that node's promotions is a conflict
  of the node (`model::Nogoods` has the proof).  Each searcher keeps a table of them: a node's conflict is the smallest
  the table knows within `P ∪ Q*` (of those as small, the one with the largest member), and a conflict is computed
  and learned only when the table knows none.  Nogoods also rule candidates out of children, beyond the conflicts known
  along the path.  The table is scanned over compact member indices, exactly.  Because what is learned depends on the
  order the search went, the tree depends on the searcher's history (threads, interruptions), not only on the node;
  the answers do not.
* **Forgetting nogoods** (`forget_nogoods`).  A lookup scans the whole table, so a larger table makes every node
  slower and old nogoods cost more than they help: each searcher clears its table after every 50,000 of its own steps
  (`search::NOGOOD_PERIOD`).  The table is a cache, so this is sound; it is deterministic on one thread, and a game of
  fewer steps than the period is unchanged.  Carrying the tables from one game to the next, or storing them in
  checkpoints, was measured and does not pay (EXPERIMENTS: "Carrying nogoods over").
* **Single-member conflicts belong to the node.**  Many conflicts have one member, which looks like a candidate
  that cannot be promoted from the root at all and could be filtered once.  It is not: the frame builder's
  candidates all pass that test at the root for every game from 2 to 1230 (a test), and a single-member conflict is a
  candidate that cannot be promoted *once the node's promotions are made* (its frame may have used it up in a forced
  pair, say), which in another branch is a nogood of two or more members.  The promotions that fail are under 4%
  of steps, so little is left to save.

### The default step

One step of a node's loop (`TaxSearcher::step`), with the default tactics:

```
need = best + 1 − sum(promoted)                   # the loop goes on while the bound reaches this
1. end the loop if cands is empty, or the max_add largest candidates sum below need
2. look at the node's heaviest matching (matchbound), refining match_refine levels deep:
     value < need                                  -> end the loop (cut)
     refined branches all < need                   -> end the loop (cut)
     Q* can be promoted                            -> offer promoted ∪ Q* as incumbent; end the loop
     otherwise: a conflict K ⊆ Q* (the nogood table's smallest, else computed and learned), and the
                forced members F of Q* (those whose removal alone brings the bound below need)
3. if K ⊆ F: end the loop (every completion beating the incumbent promotes all of K, which cannot be)
4. n = a member of F if there are forced members, else the member of K whose removal lowers the bound most
5. remove n from cands; if F is non-empty the node has nothing left to try after this child
6. child = the node with F (or n) promoted; if that fails, the step is over
7. child cands = the rest, narrowed, minus the members that a known conflict or a nogood rules out
8. cut the child if the max_next largest of its cands sum below its need, or its matching bound is below it,
   or its heaviest matching is exact (offered as incumbent); a child that stays open keeps this verdict
   for its first step; a child with no candidates is not pushed
9. otherwise offer the child's own promotions as incumbent, and push it
```

Exactness rests on four facts: the matching bound with refinement is an upper bound (above); every feasible completion
leaves out a member of a conflict, so branching on a member and then going on without it explores every feasible set;
forced members are promoted by every completion that can beat the incumbent; and nogoods and known conflicts hold at
every node they are applied to (subset closure).  A node is cut only when its bound is below `need` (a completion that
merely ties the incumbent is of no use), and a unit rebuilt from a checkpoint is a node whose bound can only be tighter.
`tests/soundness.rs` attacks each of these (section 9).

### The starting incumbent, the cap and the seed

The search starts from the best of three feasible sets: the *greedy* set (promote every candidate that can be, from
the largest down), the *seed* (the previous game's promotions that are candidates and still promotable, one by one
from the largest), and with the matching bound a rounding of the root's matching (the *dive*: drop the smallest
member of a conflict until the rest can be promoted); then a local search improves it (fill greedily, then drop one
or two members and refill, until nothing improves).  The *cap* is a proven ceiling on the promotion sum,
`n + score(n − 1) − in_the_bag`; reaching it ends the search (`prev_cap`).  Board/search setup is in `board::Board`:
it is the one place that knows the tables → frames → prepare → cap and seed path.

### `revise_freedoms`

`max_total` is a cap on the number of promotions, from the frames' freedoms.  The Java's `reviseFreedoms` lowers frame
0's freedoms, hence `max_total`, by solving the two-frame game of frames 0 and 1 (a nested search through the same
engine, within the run's time and node limits).  It assumes that the max-sum promotion set also has the max count, which
is unproven; the soundness tests leave out the perturbed games on which it fails (about one in 1,300).

## 5. Tactics

Each row is a tactic in `tactics::Tactics`, set from the CLI with `--tactic name=value` (`on` or `off`; `match_refine`
`0`…`7`; `branching` `largest`, `conflict-smallest`, `conflict-largest` or `conflict-strong`).  The ones that act inside
the search are the fields of `search::SearchTactics`; `examples/ladder.rs` takes the same names for them.  Status words:
*proven* (a theorem, given a premise), *sound* (never prunes a feasible optimum), *unproven* (a conjecture that the test
games bear out), *unsound* (known to give wrong answers).  A test (`tests/docs.rs`) checks that this table lists the
tactics of `tactics::Tactics`, in order, with their defaults, and grades exactly `reuse_prev`, `revise_freedoms` and
`legacy_bound` as unproven or unsound.

| tactic | default | status | effect |
|---|---|---|---|
| `prime_shortcut` | on | proven (model-free, given an optimal n−1 game: proof at `accelerate::prime_shortcut`) | prime n: replace the n−1 solution's first move with n |
| `reuse_prev` | on | unproven (conjecture-grade "upper-delta" certificate) | `FrameSolver.reusePrev`; keeps the previous game's promotions that are still possible and accepts the game they make, with no search, if it scores what the previous game suggests |
| `revise_freedoms` | on | unproven ("max-sum promotion set also has max count") | `UpperBoundEstimator.reviseFreedoms`; can shrink frame 0's freedoms |
| `preclusions` | off | sound (promotion failure is monotone: more moves or fewer factors never make a frame winnable) | depth-2 preclusion table; prunes candidates (off since 2026-10: with nogoods it no longer pays for its setup) |
| `match_bound` | on | sound (matchability relaxes winnability) | node bound = the matching relaxation instead of `blameGame`, and the children's candidate lists without the Java's promotability checks (the conflicts find those candidates); a heaviest matching that can be promoted is the node's best completion |
| `branching` | conflict-strong | sound (any order explores every promotion set) | which candidate a node promotes next: `largest` (the Java's order), or a member of a conflict of the heaviest matching (these need `match_bound`): the smallest, the largest, or the one whose removal lowers the matching bound most |
| `match_refine` | 1 | sound (see `search::Refiner`) | refine the matching bound by branching on conflicts this many levels deep, where it does not prune (0 = off, at most 7; needs `match_bound`) |
| `fix_forced` | on | sound (every completion that beats the incumbent promotes them) | with `conflict-strong` branching, a node's child promotes all its forced candidates; the node's loop ends after that child |
| `nogoods` | on | sound (a nogood holds at every node; see `model::Nogoods`) | learn each conflict as a nogood of the root and use the table at every node, for the conflict to branch on and refine with and to rule candidates out of children (needs `match_bound`) |
| `forget_nogoods` | on | sound (the table is a cache) | with `nogoods`, each searcher forgets every nogood it knows after each 50,000 of its steps |
| `legacy_bound` | off | unsound (the runner-up defect) | reproduce the Java bound exactly |
| `prev_cap` | on | proven if opt(n−1) is optimal | stop as soon as the incumbent reaches n + opt(n−1) − in_the_bag (`FrameSolver.getMaxPromotionSum`) |
| `seed_incumbent` | on | sound (any feasible promotion set is a valid incumbent) | start from the better of greedy and the n−1 solution's promotions that are still feasible |

The tactics that need `match_bound` do nothing without it (`branching` falls back to `largest`, and the others are
ignored); the result file still records the values as given.  A *strict* run turns `reuse_prev` and
`revise_freedoms` off; `legacy_bound` is never on in a run that means to be right.  What each tactic was worth, and
the order they were found in, is in [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md).

**Model assumptions** (unproven, inherent to the frame model, validated as in section 2): greedy removal of impossible
sources inside hives (`Hive.removeImpossible`); mandatory moves (`Hive.unzip`); `freeFactorCount = factors − sources`
(`Hive.finishSetup`); closed hives take no part in the search; a promoted number pays tax only from the frame directly
below.  They define the search space and cannot be switched off inside this model.

**Where tactics are recorded.**  A result line records the tactics it was produced with (`tactics`: the changes from
the defaults, such as `branching=largest,match_refine=0`; `tactic_switches`: every tactic's value), whether any
unproven tactic was on, and which ones the game's proof actually rests on (`taint`, section 8).  A checkpoint
records the nine search tactics and is refused when they differ (the error names each tactic that differs and the
`--tactic` flags that resume it); one made before a sound search tactic existed (`fix_forced`, `nogoods`,
`forget_nogoods`) reads back with it at its default.

## 6. Engine

`engine` is a problem-independent parallel depth-first branch-and-bound driver; the Taxman search is a `Searcher`
that it runs.  Its protocol — conservation, splitting, spill, checkpoint, termination — is the module comment of
`src/engine.rs`, with the types (`Unit`, `Searcher`, `Ctx`, `EngineConfig`, `run`) next to it; this section is the
design in outline.

* **Units and workers.**  The caller's problem is a search tree whose subtrees are *units* (serialized by the `Unit`
  trait: for Taxman two bitsets).  `threads` OS threads (std only), each with its own `Searcher` instance made on that
  thread (so per-thread caches need no locks, and on a NUMA machine each searcher's state is local), explore units
  depth-first.  A searcher calls `ctx.poll()` at least every ~1000 nodes; when it says `Yield`, the searcher pushes
  all of its unexplored work as units (shallowest first) and returns.
* **Work distribution.**  One shared deque of units, ordered shallow (front) to deep (back).  A worker that yields
  pushes its stack's remaining work and then pops from the back, resuming its own deepest work; idle workers take from
  the front, which holds the largest subtrees.  A worker yields when idle workers are left that the queued units
  cannot serve (a *split*), when a checkpoint is due, or when a stop is requested.  With one thread the deque is a
  pure stack: units are explored in exact depth-first order, unchanged by checkpoints and spilling.  The engine
  keeps one open split request per idle worker, and any running worker can claim one with an atomic decrement, so as
  many workers split at once as there are idle workers to feed; a worker answers only after 64 polls of its unit so
  that a fresh unit first gets deep enough to have something to give.  One locked deque is enough because units
  exist only after yields and a rebuild costs the searcher far more than the lock (EXPERIMENTS: "The engine").
* **Memory bound.**  The in-memory deque holds at most `queue_mem_cap` units; beyond that the oldest are spilled to
  segment files in `spill_dir` and reloaded when memory runs dry.  Searchers keep only their stack plus bounded
  caches.
* **Incumbent.**  An `AtomicU64` value plus a mutex-protected payload (the promotion set, encoded); offers only ever
  raise it, and a searcher can declare it optimal (the cap), which ends the run.
* **Checkpoint.**  Every `checkpoint_interval` all workers yield and park, the engine writes the caller's header, the
  incumbent, statistics and every unit (memory and spilled) to `<checkpoint>.tmp`, fsyncs, renames it over the
  checkpoint and syncs the directory, and releases the workers.  Resuming loads the file and continues; nothing is lost
  or duplicated beyond partially explored nodes.  Any interval works, zero included.  The caller's header identifies the
  search (section 8: it is what makes a checkpoint of another game, width, build or tactics refuse to resume).  A run
  that completes deletes its checkpoint unless the caller asks to keep it until it has recorded the result.
* **Stopping and sharing.**  Time and node limits, and SIGINT and SIGTERM (after `stop_on_signals`), stop a run with a
  final checkpoint; a second signal kills the process.  `taxman solve` takes an exclusive `flock` on `taxman.lock` in
  its checkpoint directory, so a second process there fails at once.  Spill segments carry their process id, and a run
  deletes those of processes that no longer exist.  Progress lines and results on stdout ignore write errors, so a
  closed pipe cannot kill a run.  A panic in a release build aborts (`panic = "abort"`); the previous checkpoint stays
  valid, which is the property that matters.
* **Instrumentation** (`engine::Counters`, per invocation): each worker's time split into busy, idle, parked and lock
  wait; the effective parallelism busy / (wall × threads); splits and the units they gave, steals, runs and tiny runs;
  when all workers first ran at once; the pauses and the longest wait for the workers to park.  Every progress line ends
  with them in short (README, "Reading the output").

## 7. Per-game pipeline (`solver`)

`solver::solve_game` solves one board size, and `solve_range` a range; the exact steps and checks are the module
comment of `src/solver.rs`.  Each step, with the Java class it reimplements:

| step | what | Java |
|---|---|---|
| 1 | the board's tables, and the set width | `FactorTable` |
| 2 | prime shortcut: a prime n plays the game of n − 1 with its first move replaced by n (method `prime`) | `FrameSolver.solutionForPrime` |
| 3 | hives → frames; with no candidates the game with no promotions is the only one (method `trivial`) | `bombus.Apiary`, `Hive`, `frame.FrameBuilder`, `WorkingFrame` |
| 4 | `reuse_prev`: accept a game derived from the previous one, with no search (method `reuse`) | `FrameSolver.reusePrev` |
| 5 | the search (method `search`): the frame set, bounds and greedy incumbent (`search::prepare`); the starting incumbent, cap and seed; the engine; promotions → moves | `search.SearchManager.create`, `Frame`, `FrameSet`, `UpperBoundEstimator`, `LowerBoundEstimator.greedy`, `BranchExplorer`; `Apiary(board, promotions).getSolution()` |
| 6 | `conclude`: replay and check the game, compare with optimal.json, assign its proof and taint | `game.Solution.verify` |
| 7 | record the result (a range) | |

Steps 2 and 4 need the previous game and are skipped by `force_search` and when the game has a checkpoint (a search that
was interrupted is resumed, never replaced by a derivation).

The previous game is checked before anything uses it: it must replay legally to its recorded score, be over, and open
with the largest prime.  A run with no unproven tactic on refuses one whose proof rests on something unproven (its
taint, such as `optimal.json` for that file's games); `--allow-tainted-prev` overrides it, and the results that rest
on it inherit the taint.

A search that a time or node limit stops gives an incomplete result (`completed` false, `best_so_far` the best game
so far) and keeps its checkpoint; a range stops there.  The result of `revise_freedoms`'s own search is subject to the
same limits: if they stop it, the game stops there too, incomplete and with no checkpoint, and the next run starts it
again.

**Range runs** feed each game's solution to the next and resume from the results file, which is checked when it is
opened (every game replayed, the chain from n − 1, each line's arithmetic and provenance).  Games completed in it
are skipped; each result is appended and synced as soon as it is done, and only then is the game's checkpoint deleted
(a checkpoint left for a game the file has completed is deleted when a range starts).  The n − 1 solution comes
from, in order: this run, the results file, optimal.json; with none, the game is solved standalone, with no seed and
no cap.

## 8. Results and the trust model

Every completed game records *how its optimality is established* (its proof) and *what that rests on that is unproven*
(its taint); `src/results.rs` has the full rules and the checks.

| proof | meaning | rests on the game of n − 1? |
|---|---|---|
| `trivial` | the board has no promotion candidates | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached `n + score(n − 1)`, which no game of n can exceed | yes |
| `prime_formula` | a prime n plays the game of n − 1 with its first move replaced by n (proved at `accelerate::prime_shortcut`) | yes |
| `reuse_prev` | the tactic accepted a derived game: a conjecture | yes |

The *taint* of a game is the set of unproven things its proof rests on: the unproven tactics it used itself, plus,
if its proof rests on the game of n − 1, that game's taint, and so on down the chain.  A game taken from optimal.json
has the taint `optimal.json` (the Java made it with both unproven tactics on and records nothing about which games
used them).  An empty taint means: proven within the frame model, given that the solver is right.  A chain from n = 1
with the strict tactics rests on nothing unproven.

**Checks and referees.**  Every game is replayed by `game::replay_finished` before it counts, with the arithmetic and
the ceilings `score(n−1) ≤ score(n) ≤ n + score(n−1)`; a game that fails after it was found goes to
`<results>.rejected.jsonl` and the run fails.  `taxman verify-results` reads results files without changing them and
runs the same checks plus agreement between files; `tools/check_results.py` is an independent referee in Python that
re-derives every proof and taint and compares scores with the Franklín–Moniot upper bound and the best-known
heuristic chain.  What no check can catch is a game that is legal, finished and within the bounds but not optimal
when the game of n − 1 is not optimal either; that is what the proof kinds and the taint track, and why a chain from
n = 1 matters.  [results/README.md](results/README.md) says what supports each committed score.

**Result lines** are one JSON object per line (README, "A results line"): the game, its proof and taint, the game of
n − 1 it was built from (score, source, digest of its moves, proof, taint), node counts and search counters, seconds per
phase, the tactics, limits, options and the commit the binary was built from.  A line is never rewritten.  Lines
written before provenance existed (schema 1) have their proof and taint derived from the fields they have.

**What identifies a checkpoint.**  The header the search stores: a format version, the set width, n, digests of the
root candidates and of the frames (every frame's moves, factors and freedoms, as built), frame 0's revised freedoms,
`max_total` and the tactics.  Resuming checks all of them and says which differs, and checks that the stored incumbent
is a set of candidates that can be promoted from the root and is worth the stored value; so a checkpoint of another
game, or of an older build whose frame builder differs, is refused instead of pruning with a foreign incumbent or
failing to rebuild its units.  A resumed game takes frame 0's freedoms from the checkpoint, and its `setup.revise` is
`FromCheckpoint` (a taint of `revise_freedoms`) only if the checkpoint's tactics have that on; a resumed strict game
says `Off` and rests on nothing unproven, as one that ran in one piece.

## 9. Testing

Correctness, all on games that finish in seconds: the fast fixtures and small boards stand in for the large boards
that cannot be run in a test, and the tripwires and soundness tests catch the failures that no assertion on an
answer sees.  `cargo test` takes about half a minute after compiling, with two test threads, in the test profile
(`opt-level = 2`, because the tests replay real games); the engine's heavy tests are ignored by default.

* **Unit tests** per module, including the Java tests' cases (`FactorTableTest`, `FrameTest`, `FrameSetTest` for n =
  85). Helpers shared by the unit and integration tests (temporary directories, a random generator) are one file,
  `tests/common/support.rs`.
* **Golden tests** (`tests/golden_hives.rs`, `tests/solver.rs`): every stage of the port against
  `testdata/golden/n*.json` (exported from the Java by `tools/golden/export.sh`): hives, frames, candidates, reduced
  frame set, greedy incumbent, revised freedoms, solve result.  The searched golden games (94, up to 760) give the
  Java's promotion sums with the default tactics; the tactic variants give the same answers on those up to 400.  Past
  760 the Java's structure is fixed in `testdata/golden_wide/` (n = 511, 512, 1000, 1001, 1023, 1024, 1025, 1100, 1231,
  2047 and 2048, checked at the width `with_width!` picks), `accel_java.txt` (the Java's acceptance of the previous
  game's promotions for 94 boards of 763–1228) and `games_java.txt` (its apiary game, move for move, for 171 sets of
  promotions at 762–2049).
* **Regression**: `taxman regress --to N` solves 1..N as one range and checks every score against optimal.json (the
  Java's own test: 146 accelerated games in 1..200).
* **Tactic invariance**: the regression with each unproven tactic off must give identical scores; so must games 1..300
  forced to search with each sound tactic off or changed, and a checkpoint is refused when any of the nine search
  tactics differs.
* **Bound soundness**: on small frame sets, the upper bound is never below the exhaustively computed best completion;
  the same for the matching bound and its refinement by conflicts (whose exact completions must be promotable, and
  whose conflicts, carried down a path, must stay conflicts).  Conflicts and cores are checked against promoting the
  sets.
* **Soundness harness** (`tests/soundness.rs`): the search is started with an incumbent worth 1 or 5 less than the
  optimum and an empty payload, so that any pruning that loses every best completion leaves the fake incumbent in place
  (this tests the `<` and `<=` boundaries too): every game 2..=300 under each of 20 configurations (every tactic,
  forgetting every step, every third, never; one thread and two); the same for 500 perturbed frame sets (numbers
  dropped, role flipped, moved a frame; half of them with frame 0's freedoms revised as `prepare` does) with the truth
  found exhaustively; promoting a set at once against one by one in random orders, conflicts and nogoods on perturbed
  models; and searches interrupted every 1,500 nodes and resumed from a checkpoint with units spilling.  A perturbed
  game whose revision changes the best sum is left out: the search takes the revised freedoms for a fact about the best
  set, and for a game that breaks that (one in about 1,300) may return a set that they forbid.
* **Node-count tripwires** (`tests/tripwires.rs`): a pruning tactic that stops working changes no answer, so no
  assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` costs 27 to 67 % more nodes).
  With one thread the tree is deterministic, so the exact node count of a fixed game under fixed tactics (n = 480 to
  1133, every tactic off or changed in turn) is asserted, and the tree of game 600 with the searchers forgetting their
  nogoods every 500 steps.  **A change that alters the tree on purpose fails them.**  Run `cargo test --test tripwires
  -- --nocapture`: it prints the rows as observed, in the layout of the table at the top of the file; check that the
  promotion sums (the answers) have not moved, paste the rows in, and update any node counts quoted in the docs.
* **Engine** (`tests/engine.rs`, `tests/robustness.rs`): a toy problem with a known optimum (knapsack against dynamic
  programming), run with 1 and many threads, with a tiny `queue_mem_cap` to force spilling, and killed and resumed from
  checkpoints, must give the same answer; ignored tests do the same with 64 and 192 threads, checkpoints every 2 ms, and
  a panicking worker (`cargo test --release --test engine -- --ignored many_threads`), and draw random configurations
  through any number of resumes (`TXENGINE_STRESS=<n>`).  A tree with no pruning checks conservation exactly: under
  random configurations the nodes counted must equal the tree's size, so one unit lost or explored twice shows
  (`TXENGINE_CONSERVATION=<n>` for more).  Robustness: an interval of zero finishes; limits too large for the clock are
  never reached; spill segments of dead processes are removed; a locked checkpoint directory is refused; a checkpoint of
  another game, with other tactics or with an incumbent that does not fit is refused; and the binary rejects bad times,
  survives a closed stdout and stderr, and stops on SIGINT and SIGTERM with a checkpoint it then resumes.
* **Integrity of results** (`tests/integrity.rs`, `tests/results.rs`, `tests/solver.rs`): a previous game that is not
  what it says is refused; taint travels down a chain; a tampered results file is refused naming the line; a torn last
  line is dropped; the committed results pass the verifier with the counts `results/README.md` quotes; a strict game
  stopped by a node limit and resumed reads back with an empty taint.
* **Large boards** (`tests/wide.rs`): `optimal.json` and the Java stop at 1000, and the sets that hold a board are 8 to
  64 words wide, so nothing above 1023 meets anything but the committed results and these tests.  The same game at every
  width that holds it has the same frames, candidates, greedy set, promotion cap, answer and tree; game 1133 (W = 20,
  about 2,400 nodes) under every sound tactic against `strict.jsonl`; a slice of the strict chain; the sacrifice bound
  proves game 1133 by an independent route; a W = 64 board (2050) through frames, matching bound, conflicts, greedy set
  and a node-limited search; a committed W = 32 checkpoint of game 1133 (`testdata/n1133_after_1000_nodes.ckpt`) that
  must still resume, at its own width (a deliberate change of the checkpoint format fails it; the test says how to make
  the file again); and the levels of a real depth-first stack of game 1215 (`testdata/n1215_deep_units.txt`): the
  incremental matching bound and the quick bounds without a member against the min-cost-flow reference.
* **Command line** (`tests/cli.rs`): every subcommand's happy path and its usage errors through the real binary;
  `bench --history FILE` keeps the tracked `bench/history.jsonl` out of the tests.
* **Documentation** (`tests/docs.rs`): the table of tactics in section 5 matches the code.

A few tests sample what an exhaustive run would cover (every 18th golden board for the peeling agreement, every sixth
board above 511 for the root promotability of the frame builder's candidates, every second line of the Java fixtures);
the comment on each says what the whole run costs.

## 10. Performance

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other load
on the machine moves wall times by up to 3×.  What measures what:

* **`taxman bench`** runs a fixed set of games that need search and take about a second each on one thread — n = 936
  (W = 16) and 1050, 1100, 1133, 1170, 1184 (the frontier's width) — standalone (n − 1 from optimal.json, past it from
  `results/strict.jsonl`; acceleration off), prints per-phase times and node counts, and appends them to
  `bench/history.jsonl` (the commit the binary was built from, every tactic's value, per game the nodes, seconds per
  phase and search counters), so that performance can be followed across changes.  `--history FILE` and `--known FILE`
  name other files; by default they are found in the repository.
* **The probe** (`cargo run --release --example perf`): replays of the hot paths at the frontier from fixtures in
  `testdata/perf/` (0.2 MB), measured in cycles and instructions retired of the process (`proc_pid_rusage`); each with a
  fingerprint of what it computes, which an exact optimization must leave unchanged.  F1: 300 search nodes each of the
  proofs of 936 and 1215, every kernel over all of them (the rebuild of a node, the promotion of every candidate,
  conflicts, the matching from scratch and warm-started, bounds without each member, strong branching's exact solves,
  children, nogood lookups and rule-outs).  F2: the frame checks of those promotions.  F3: a checkpoint of each proof
  resumed on one thread for 50,000 steps, one period of forgetting nogoods: the search itself, fingerprinted by its
  counters.  About 5 s in all; `perf make-nodes` and `perf make-window` make the fixtures again, deterministically.
  For whole searches, `/usr/bin/time -l` reports the same counters per process.
* **Profile-guided optimization** (`tools/pgo.sh`): an instrumented build runs the probe and a short search at every
  set width (code a profile never saw is optimized as cold), and `-Cprofile-use` builds `target/pgo/release/taxman` and
  the examples.  The same trees and fingerprints.  Since the hot loop is inlined by hand it adds 0–2% (it was 12%
  before).  Profile the plain build: with PGO the search is inlined into a few functions and a profiler sees little.
  On Linux, rustup's `llvm-tools` has an llvm-profdata that matches rustc's LLVM, and the script keeps
  `RUSTFLAGS="-C target-cpu=native"`.
* **The ladder** (`cargo run --release --example ladder -- [--threads T[,T…] | --scaling] [--seconds S] [--diag]
  [--seed-opt] [--known FILE] [--tactic …] N…`; its flags are documented at the top of `examples/ladder.rs`): standalone
  searches of hard games from the frame builder, seeded from the known n − 1 game and capped as the solver does, each
  with a time limit; it reports completion, nodes, time, when the final incumbent was found, and the engine's counters.
  `--seed-opt` seeds with the game for n itself, to measure the proof alone; `--diag` prints `search::Diagnostics`
  (nodes by depth, what pruned, bound gaps, conflict sizes, matching work).  Fast progress benchmarks (one thread,
  `--tactic revise_freedoms=off`): 880, 936, 1050, 1100, 1170, 1184; harder ones: 992, 996, 1025, 1215, 1216 on several
  threads.
* **Scaling**: the ladder with a list of thread counts (`--threads 1,2,4,8`, or `--scaling` for 1, 2, 4, … up to all
  cores) searches each game with each count and prints a table per game and one summed over the games: search time,
  speedup, nodes per second per thread, effective parallelism, idle and lock-wait time, splits and the units they gave,
  steals, runs, tiny runs and the time to full occupancy.  With `--seed-opt` the node count barely depends on the
  threads, so the speedup measures the parallel proof alone.  The engine's own benchmark (`cargo test --release --test
  engine -- --ignored --nocapture bench`, sizes and thread counts set by `TXENGINE_BENCH_*`) measures its overhead per
  node with no work per node.
* **The cloud** (`tools/cloud`) has the scripts for a many-core machine: the sweep to 192 threads, the cost of a
  checkpoint pause, and the production chain (RUNBOOK.md).

**Where it stands** (the notebook has the tables): the proof of 1215 takes 33 s on one thread (106 G cycles), and 1184
takes 3.2 G cycles; the strict chain 1..1231 takes 279 s of game time on 8 threads of the M1 Max, 145 s of it for
1201–1231.  The cost of a game grows by a factor of ten for every 55 to 70 in n past 1200
([bench/curve/README.md](bench/curve/README.md)).  The Hungarian repairs of the matching are about 46% of the time, the
peels 24%, the bounds from the potentials 7%, the node's own work 7%, level copies 4% and nogood lookups 3%.

## 11. Status and what is left

Every n to 1231 has a game whose proof rests on nothing unproven: n ≤ 1000 are optimal.json's, reproduced by a strict
run from n = 1 (that output was not kept), 1001–1219 are in `results/strict.jsonl` and 1001–1231 in the cost-curve
study's `bench/curve/chain.jsonl` (`results/README.md` says what supports each score).  1232 is unfinished.  What is
left to try: the incumbents of games like 992 and 1025, whose optimum the search reaches late; cheaper matching repairs
(the largest cost); a warm-started unit rebuild; and, for the many-core machine, whatever its first sweep shows that a
laptop cannot.  Nothing about scaling beyond 8 threads or about Graviton has been measured (`tools/cloud/RUNBOOK.md`,
section 8).
