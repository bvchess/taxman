# Design: the frame solver for optimal Taxman

This is the design of the frame solver as it is now ("the solver" below), written to be read from top to bottom: the
goals, the game and the model of it that the solver searches, the architecture, the search and its tactics, the engine
that runs it on many threads, the per-game pipeline, how results are recorded and trusted, how it is tested, and how
performance is measured.  Each term is defined where it first matters.

* The [README](README.md) is the user guide (how to build, run and read the output) and has the glossary.
* [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md) is the lab notebook and the register of everything tried, kept or not:
  every round of improvements with its measurements, and the ideas that were measured and not kept.  This page states
  what the solver does and why; the notebook says what it was worth and what was tried instead.
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

* Find optimal games beyond n = 1000, where the Java solver's table of optimal games stopped, as far as the machines
  at hand reach: today every n to 2175 on one laptop (Apple M1 Max, 10 CPUs, 32 GB RAM, fast SSD), and the same
  chain on a many-core cloud machine for what is beyond (`tools/cloud/RUNBOOK.md`).
* Be right before being fast.  Every game is replayed by an independent referee before it counts, every result
  records how its optimality is established and what that proof takes as given, every tactic is proven or sound, and
  every tactic can be switched off so that its effect on speed can be measured.
* Search an enormous tree efficiently: fast per node on a single thread, and scaling cleanly to the CPUs available
  (up to a 192-CPU machine) without depending on many of them.
* Never rely on everything fitting in RAM.  Memory use is bounded by configuration, independent of tree size; work
  that does not fit goes to disk.
* Every long run checkpoints frequently and resumes after a restart, losing at most one checkpoint interval; stopped
  by SIGINT or SIGTERM, it loses nothing.  (A machine can restart without warning, and a job that wrote its output
  only when it finished then loses all of its progress.)
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

### Sources, factors, frames

Every number above n/2 divides nothing else on the board, so it can only be a move, never tax: it is a *source*.
Every other number is a *factor*: tax for a larger number, and sometimes a move itself.  A move m is paid for by one
of its *maximal factors* m/p (p a prime dividing m); and a factor can pay for one move.

Why maximal factors are enough, although a move takes every divisor of it: the lifting argument of `numtheory` and
[THEORY.md](../heuristics/THEORY.md) (Theorem 1), summarised in the
[README](README.md#how-the-solver-sees-the-game-the-frame-model).

Sources and the factors they compete for fall into independent groups, the *frames*: sources that share a factor are in
one frame (`frames::BoardFrames` forms them).  Frame X is *in front of* frame Y (and Y *behind* X) when a number of Y
has a maximal factor in X, that is, Y's numbers are multiples of X's; the frames in front are played first.  Simplifying
a frame finds its mandatory moves (`unzip`: a source with one usable factor must take it), the sources that cannot be
played together with the larger ones (`removeImpossible`), and its *freedoms*, factors minus sources: how many factors
it can spare; a frame with none is *closed*.  The frames with freedoms are numbered from 0 at the front
(`frames::build_frames`): number 0 is every one that has no other in front of it, number k + 1 every one directly behind
number k.  Frames with the same number are merged into one, so that a promotion always has one frame to go to (the
README's example shows where that happens).  A frame is a game of *Taxman Mini*: a set of moves and a set of factors, in
which every move must still have one of its maximal factors in the frame when it is taken, and taking it removes all of
its factors still in the frame (not only the one that pays for it).  A frame is *winnable* when some order takes every
move; a frame is winnable exactly when repeatedly dropping a move that has a factor no other move has (a *peel*) drops
them all, and `model` tests that with counters instead of searching orders.

### Promotions and the score

What a game can still choose is which of the spare factors to *promote*: to play it as a move instead of leaving it
as tax.  Promoting a factor of frame h makes it a move of frame h − 1, one frame forward (a promoted number pays tax
only from the frame directly in front of its own), and is legal when both frames stay winnable.  The numbers that
could be promoted are the *candidates*; a *promotion set* is a set of candidates that is *promotable*: all of it can
be promoted from the root.  Afterwards the frames are *reduced*: a move left with one live factor must be paid by it,
so the pair is dropped, repeatedly.  A frame's *freedoms* (factors minus moves) drop by one for each promotion into or
out of it, and no frame may go below zero.

The score of a game of the model is `in_the_bag` plus the sum of its promotions, where `in_the_bag` is what the game
with no promotions scores (the Java's `inTheBag`).  So the solver looks for the promotion set with the largest sum; the
moves of the game follow from the promotions (`frames::BoardFrames` with the promotions treated as sources).  A worked
example, the board 1..=21, is in the [README](README.md#how-the-solver-sees-the-game-the-frame-model).

### What "optimal" means

* **Within the frame model.**  The decomposition rests on assumptions that are validated, not proven: the greedy
  removal of impossible sources inside frames, the mandatory moves of `unzip`, freedoms = factors − sources, closed
  frames (those with no freedom) taking no part in the search, and a promoted number paying tax only from the frame one
  forward.  They are validated by agreement with the Java's optimal.json for n ≤ 1000; by the prover (a branch and
  bound over sets of moves on the whole board, from the playability theorems, the Franklín–Moniot bound, the parity
  bound of THEORY.md's Theorem 10, the ceiling and the prime formula, and nothing of frames), which confirms the
  optimum of every game of the committed chain, to n = 2175; by an independent
  brute force (`heuristics/evaluation/certify.py`) for n ≤ 62; and by `brute`, the solver's own exhaustive search,
  which checks frame-model games on small boards (up to 40 in the test suite, 52 in `examples/sacrifice validate`).
  The greedy removal of impossible sources at the root, where it shapes the search space, is descending greedy on each
  component of the sources' instance, which the prover certifies maximum-weight (for every weighting consistent with
  the order) in every game n ≤ 100,000.  The sacrifice bound (`sacrifice`) is an upper bound on opt(n) by a different
  route (games on punctured boards); it proves game 1133 and disagrees with nothing.  [The prover's
  README](prover/README.md) has the details.
* **Given the pruning is exact.**  Every tactic is proven or sound (section 5 grades each).  The Java used two
  shortcuts that rest on conjectures (`reuse_prev` and `revise_freedoms`); the frame solver had them as tactics for a
  while, measured that they no longer paid, and removed them (EXPERIMENTS, "Which tactics earn their keep").
* **Given the previous game is optimal.**  The game of n is built from the game of n − 1: opt(n) ≤ n + opt(n − 1)
  (deleting the move n from an optimal game leaves a game of n − 1), and a prime n plays the game of n − 1 with
  its first move replaced by n.  A chain of games from n = 1 is therefore proven game by game (section 8).
  Optimal games need not be unique, though, which matters to any step that reuses the game of n − 1: from
  n = 425 some games have two or three optimal sets (all with upper part U\*, differing below n/2), and the
  recorded game of 685 plus 686 is unplayable although opt(686) = 686 + opt(685), through another optimal set of
  685.  A chain step is decided by the complete list of n − 1's optimal sets, not by one of them
  ([the prover's README, section 5](prover/README.md#5-is-the-optimal-set-unique)).

### The Java solver and the frame solver

The optimization strategy is the Java solver's (and the [wiki's](https://github.com/bvchess/taxman/wiki)): decompose the
game into frames of Taxman Mini and search for the optimal promotion set.  The Java is research code, three generations
of the same frame idea layered on each other, so the frame solver is a reimplementation of the *algorithm*, designed for
clarity and speed, not a translation of the classes.  What must match the Java is *results*: the setup (frames,
candidates, the greedy incumbent) defines the search space, and reproduces the Java's values exactly (its golden data in
`testdata/golden/` is the check); every final score equals optimal.json.  The search itself is new.  It started as the
Java's loop (its `blameGame` bound, its depth-2 preclusion table and the promotability checks of its candidate lists,
branching on the largest candidate), and each of those was replaced by something measured to be faster, then removed
(the notebook has the rounds, and [which tactics earn their
keep](docs/EXPERIMENTS.md#which-tactics-earn-their-keep-2026-10) the last measurements of what was removed).  Not
ported: `oldsearch/` (the 2021 disk-queue search), `frame.GreedySolver`, `util.Memo`, `WorkingFrame.fancyFit/fits`, the
picklers and compressor.

The Java's and the wiki's words map onto this document's.  The Java's `Hive` is a frame during setup (before or after
frames of one number are merged: `frames::Frame`) and `Apiary` all the frames of a board in playing order
(`frames::BoardFrames`); `FrameBuilder`'s levels are the frame numbers; `WorkingFrame` and `search.Frame` are a frame
(as the search takes it, `model::Frame`); `freeFactorCount` is the freedoms; `downstream` and `upstream` are in front
of and behind; `fits` is promotable; `getMaxPromotionSum` is the ceiling as a promotion sum.
`isWinnable`, `inTheBag`, `maxTotalPromotions`, `promotionCandidates` and `mandatoryMoves` are used as they are.  The
wiki's *selection* is a move, its *front* and *back* are the same, and its upper bound N + opt(N − 1) is the ceiling.

One defect of the Java's bound was found in the port: `UpperBoundEstimator.getRunnerUp` replaces the failing number
with the next candidate below the smallest bid member, and a skipped candidate from the failing number's frame can be
larger, so the Java bound can be too low (at the root of n = 114: 358 against the best completion's 360; [the
story](docs/EXPERIMENTS.md#the-port-and-its-first-improvements)).

## 3. Architecture

One Cargo crate in `rust/`: a library and one binary, `taxman`, with no third-party dependencies.  `src/lib.rs` has the
module map with a line on each module, in the order the modules are read (README, "Reading order").  The layers, bottom
up:

| layer | modules | responsibility |
|---|---|---|
| foundations | `game`, `numtheory`, `bits`, `json` | the rules as a referee and the table of optimal games; maximal factors and composites; fixed-width bitsets; a small JSON reader and writer |
| frame model | `frames`, `model`, `accelerate` | the board as frames (`frames` forms and simplifies them, numbers and merges them); the search state (frames, winnability, promotion, conflicts, nogoods); deriving a game from the previous one |
| search | `matchbound`, `gf2`, `search`, `bounds` | the matching bound, and the Lagrangian bound over nogoods built on it; the GF(2) bound, the default node bound; the branch-and-bound searcher with its setup and checkpoint header; the greedy incumbent and the count of promotions a node can still make |
| engine | `engine` | the problem-independent parallel depth-first driver |
| solving | `tactics`, `board`, `solver`, `results`, `paths` | the switchable tactics; a board prepared for a search; the per-game pipeline and the range driver; what a result records and the checks of a results file; the default files |
| studies | `brute`, `sacrifice` | exhaustive search to certify small boards; the sacrifice bound and punctured games (not used by the solver) |
| front end | `main.rs`, `cli.rs`, `commands.rs` | `solve`, `regress`, `bench`, `verify-optimal`, `verify-results`, `extend-optimal`, `tables` |

On the path of a solve are all the layers but the studies.  `examples/` holds the ladder (standalone searches of hard
games with time limits, thread scaling, diagnostics), the probe `perf` (the hot paths from fixtures, in cycles and
instructions), the sacrifice study, `repeats` (the games whose search repeats an earlier game's, from the setups alone,
and what a results file spent on them), `slope` (the gaps of a game's root, and proofs with every bound lowered by a
margin), `matroid` (the study of the GF(2) bound: at roots and nodes, as a plain branch and bound, and the search's
proofs with it, `matroid prove`) and `wall` (what grows with n and with a tree: a game's fixed costs piece by piece, a
node's cost at any set width, large trees with their memory and checkpoints, the frames of any board); `tests/` the
integration tests (section 9); `tools/` the results referee in Python, the PGO build, the Java golden export and the
cloud scripts.

**Width dispatch.**  Everything that holds sets of game numbers is generic over `const W: usize`, the number of 64-bit
words, and the macro `with_width!` (in `bits`, used by the solver, the examples and the tests) picks the narrowest of
`bits::WIDTHS` = {8, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 128} that holds n (n ≤ 511, 1023, 1279, 1535, 1791, 2047,
2303, 2559, 3071, 3583, 4095, 8191).  Every set operation is a loop over all W words, so the steps of 4 to 8 words past
n = 1023 pay; past 4095 they would not (a node at 4100 costs 1% more at 128 words than at 72, the board layer the
same), so 128 is the only width there.  Each width is a copy of the search's code, which costs build time (a release
build takes about 100 seconds on an M1 Max, 12 more for a width; EXPERIMENTS has the times as widths were added).  The
largest board, 8191 (`solver::MAX_N`, from the widest width), is also what the other fixed sizes hold, every board to
it checked in 2026-10: at most nine frames (`model::MAX_FRAMES`), a peel's worklist of two summary words, and the
GF(2) bound's vectors of at most five words (section 4).  A checkpoint records its width, and one made at another
width than this build gives its game is refused, so a checkpoint of a game of 2048 to 3583 made at 64, before the
widths of 36, 40, 48 and 56 were added, is refused by a build that has them; adding 128 moved no board, so the
checkpoints of the games to 4095 resume with the build that goes past it.

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

The loop is the Java's (`BranchExplorer.explore`, made lazy so that children are explored as they are made rather
than all generated first): end the loop when a bound on the node's completions cannot beat the incumbent, else take
a candidate out of the loop, promote it in a child, narrow the child's candidates, and explore the child if its own
bound can.  What bounds a node, which candidate it branches on and what the child promotes are this solver's own;
the next sections build them up, and "The default step" below puts them together.  A bound equal to `best` cannot
lead to a better incumbent, so loops break then (the Java breaks only below).

### The matching bound

Winnability relaxes to *matchability*.  Each move of a winning order takes at least one factor that is present when
it is taken and removes it, so a winnable frame gives its moves distinct maximal factors.  A promotable set
`Q` therefore gives every move of every frame, and every promoted number (in the frame in front of its home), a distinct
maximal factor, and no member of `Q` is used as a factor.  Every edge `v → v/p` changes the number of prime factors
by one, so the graph is bipartite and the *matching bound* — the largest sum of a candidate set that has such an
assignment — is a maximum-weight bipartite matching (a min-cost flow), exactly solvable and an upper bound on the
sum of the best promotable completion.  It ignores freedoms and `max_total`, the most promotions the freedoms allow,
which only remove completions.  Measured at the root of all 94 golden games that need search, its mean gap to the
optimum is 31 against 76 for the Java's bound; on the former benchmark games (n = 480–760) 107–292 against 363–769.

It is used incrementally.  `matchbound::MatchGraph` is the root's graph, whose edges never change; every node's
problem is that graph with each vertex in one of three roles (required: the root's moves and the promoted numbers;
candidate; factor).  `MatchBound` keeps an optimal matching and an optimal dual solution (potentials) for every node
on the search's path, and repairs them after each change by Hungarian searches on a radix heap; the changes along a
depth-first path are small, so an evaluation costs 0.7–2.4 µs where a solve from scratch costs 30–170 µs (at n = 480 to
760).  The potentials also give, at once, an upper bound on the matching *without* a given candidate
(`bound_excluding`): lowering that candidate's potential and its partner's as far as their remaining edges allow.  The
module comment of `matchbound.rs` is the best explanation of all this.

The Java's `blameGame` bound, which it replaced, never pruned where the matching bound did not (diagnostics: zero at
every check).  The bound is an upper bound on `sum(promoted) +` completions, so a node whose matching value is below
`need = best + 1 − sum(promoted)` is cut.

### Conflicts of the matching bound

When the heaviest matching's candidates `Q*` cannot all be promoted, some frame is unwinnable with them.  Let a *peel*
of that frame run: the moves it cannot drop, its *residual* (heuristics/THEORY.md's word for what every dismantling
ends at), are stuck however the order goes, and the members of `Q*` that the residual needs (promoted into the frame and
residual moves, or promoted out of it and factors of residual moves) cannot be promoted together.  Minimized by peeling
only the residual, such a set `K ⊆ Q*` is a *conflict*: a set of candidates that
cannot be promoted together from the node's state (`FrameGraph::conflict_using`).  Conflicts are tiny (a median of two
members at n = 760–1000, a quarter of them single members).

The idea behind everything that follows: a subset of a promotable set is promotable (moves left out and factors kept
only help a frame), so **every promotable completion leaves out some member of a conflict**.

* **Refinement** (`match_refine`, default 1).  Where the matching bound does not prune, bound the branches of the
  conflict too: with members `k₁ < k₂ < … < k_r`, every promotable completion `F` is in exactly one branch
  "`k₁ … k_{j−1}` in `F`, `k_j` not", and the best completion is at most the largest of the branches' matching bounds
  (the matching with `k_j` no longer a candidate and `k₁ … k_{j−1}` required).  Each branch is refined the same way, one
  level less deep.  By induction the refined bound is never below the best completion (`search::refine::Refiner`).
* **Strong branching on the conflict.**  A node branches on a member of the conflict instead of its largest
  candidate: the child promotes it, and the node goes on without it, so that neither can reproduce the heaviest
  matching.  The member is the one whose removal lowers the matching bound most (one bound without each member; with
  `bound_excluding` most are settled by the potentials alone).  A node with no conflict among its candidates
  branches on its largest.  (The smallest member, the largest, and the Java's largest candidate were once tactics; at
  the hardest games they took 6 to more than 17 times as long, and were removed.)
* **Exact matchings.**  A heaviest matching that can be promoted is the node's best completion: it is offered as the
  incumbent and the node is done.
* **Propagation into children.**  A conflict known at the node, of which a child promotes all but one member, rules
  that member out of the child's candidates (the child's state is the node's with more promotions, so promoting it
  would promote the whole conflict).  Without it the child's matching bound equals the node's.
* **Starting incumbent.**  See "The starting incumbent, the ceiling and the seed", below.

### Forced members and nogoods

The tree is mostly chains: at four nodes in five the node's loop ends right after its first child, because
removing the member that strong branching chose already brings the bound below what the node needs.  Such a member
is *forced*: every completion that beats the incumbent promotes it.

* **Forced members** (`fix_forced`).  The child promotes every forced member of the conflict, not only the chosen
  one; the node's loop ends after that child, since the child covers every completion that can beat the incumbent.
  If they cannot all be promoted, or the child's candidate filter dropped one, the child has no such completion at
  all.  They are promoted together, each frame checked once (`FrameGraph::promote_all_using`).  The forced members are
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

### The Lagrangian bound over the nogoods

The matching bound knows no nogood, and its heaviest matching often promotes all of one.  The *Lagrangian bound*
(tactic `lagrange`) takes them in.  At a node with promotions `P` and candidates `C`, a nogood `N` is *live* when its
members outside `P` are all still candidates; every completion then leaves out one of them, `|F ∩ N| ≤ |N − P| − 1`.
Moving these inequalities into the objective with multipliers `λ_N ≥ 0`, as Lagrangian relaxation does in integer
programming, gives a bound for any multipliers (weak duality): the heaviest matching with each candidate weighed by
its number less the multipliers of the live nogoods it is in, plus `Σ λ_N (|N − P| − 1)`.  It is computed in integers
(weights in 64ths, nothing rounded); the module comment of `matchbound::lagrange` has the argument.

* **Incremental.**  A second stack of matching levels, with weights of their own (`matchbound::lagrange`; the
  levels and the Hungarian repairs are the matching bound's, generic over how a level weighs its candidates).  A
  level is made for a node when the bound is asked for there, from the nearest level on its path: what the node
  promoted is promoted, the candidates it no longer has are dropped, and the live nogoods they were in die, so the
  weights of their other members *rise*, which needs a repair only where a member's potential must rise past a tight
  edge.  Weights fall only when the multipliers change, and then the levels are dropped and solved afresh.  A level
  costs about what a matching level does.
* **Where, and what for.**  At every check of a node or a child that the matching bound and refinement leave open:
  below `need` it cuts (cut reason `lagrange`); otherwise the members of its heaviest matching without which the
  bound, read off its potentials as `bound_excluding` reads the matching's, would be below `need` are forced members
  too (every completion that beats the incumbent promotes them), and with `fix_forced` the node's child promotes
  them with the matching's.  The forced members are most of the gain.
* **The multipliers.**  Each searcher fits its own, at the root, to the nogoods it has learned: after 2,000 of its
  steps and then just before each forgetting of its table (every 50,000), 50 subgradient steps (Polyak's rule towards
  what the root needs) over the table's nogoods and those of the multipliers it had, keeping the nogoods whose
  multipliers are positive (`search::lagrange`).  Any multipliers give a bound, so the schedule decides speed only;
  like the nogoods, the multipliers are a cache that units and checkpoints do not record, and the tree depends on
  the searcher's history.  A search of fewer than 2,000 steps is unchanged.
* **What it is worth.**  Proofs of the hardest games (1184 to 1300) take 2 to 6 times fewer cycles on one thread, about
  3 times on average, with no trend in n; the chain 1201–1231 on four threads 2.4 times less game time.  Each
  step costs about half as much again, so a search whose time goes into finding the optimum late, as 992's does, is
  slower (EXPERIMENTS: "A Lagrangian bound over the nogoods").

### The GF(2) bound

The default node bound (tactic `gf2`) relaxes winnability less than matchability does.  In each frame, give the
factors coordinates over GF(2), each mover (a move of the frame, or a number promoted into it) the vector of its
maximal factors there, and each number promoted out of it its own unit vector: a winnable frame peels completely, and
the peel's order makes these vectors linearly independent.  The moves of a frame are fixed, so its condition is a
linear matroid on the candidates that touch it (contracted by the moves; its rank is the frame's freedoms).  A
candidate of home `h` touches frames `h` and `h − 1`, one even and one odd, so the even frames' matroids are the
blocks of a matroid `A` on the candidates and the odd frames' of a matroid `B`; every promotable set is independent
in both, and the heaviest common independent set of at most `max_add` members (weighted matroid intersection,
polynomial) bounds a node's completions.
[heuristics/THEORY.md](../heuristics/THEORY.md#the-frame-matroids-a-bound-on-the-promotions), Theorem 11, has the proof
(the frame model's form of Theorem 10, the parity matroids' bound on the whole game, which the prover uses); the module
comments of `src/gf2.rs` and `src/gf2/meet.rs` the algorithms.  It is never above the matching bound (independent
vectors have a system of distinct representatives, which is a matching); what it misses are the *odd* cores of movers
with three or more factors.  At the roots of 880–1300 its gap to the optimum is a tenth of the matching bound's
(EXPERIMENTS: "Matroid intersection bounds").

* **Kept along the path.**  `gf2::Gf2Graph` builds the root's matroids once per search; `gf2::Gf2Bound` keeps a level
  per node of the searcher's path, a child's being its parent's contracted by what the child promoted (one pivot per
  member and matroid) with the candidates it lost deleted.  The intersection (`gf2::meet`: Lawler's shortest
  augmenting paths, over a tableau of bitset rows that a pivot updates per exchange) starts from the last of the sets
  the parent's augmentations passed through that the changes keep (every one is the heaviest of its size and heavier
  than all smaller ones); when that is the parent's optimum, the child inherits it with no solve.  The parent here
  is the node as it was before it went on without the member the child promotes (it is kept aside for that), so
  that the child finds the node's optimum among those sets.
* **The heaviest set without a member, with no solve.**  The heaviest set without a member `x` of the optimum `I*`
  is, in practice, `I*` flipped along one cycle of its exchange graph through `x`; `Meet::without` finds the shortest
  such cycle by Dijkstra's search over the weight split's reduced lengths (below), checks the flipped set common
  independent, and certifies it the heaviest when what the sets of other sizes lose against `I*` is at least the
  cycle's length (the symmetric difference of `I*` and any set without `x` splits into cycles and paths of the
  exchange graph, all of non-negative length but the one through `x`; the module comment of `src/gf2/meet.rs` has the
  proof).  What the smaller sets lose comes from the level's history (a dominant set one smaller), a floor carried
  along the path, or a search for the shortest path between members; what the larger sets lose is the solve's last
  shortest path.  A child that loses one member of its parent's optimum besides those it promotes gets its own
  optimum the same way, from the parent's exchange graph with the dropped elements banned from the cycle.
* **Forced members.**  The optimum's weight split (Frank: `w = w1 + w2` with the optimum heaviest in `A` for `w1` and
  in `B` for `w2`, from difference constraints by Bellman–Ford, each round from the vertices that changed) bounds the
  intersection without each member in one pass, by weak duality; a member without which it is below `need` is
  forced, as the matching's are.  The split's bound is loose, so for the members it leaves free within `FORCE_WINDOW`
  (80) of `need`, the exchange graph decides: a member is forced too when no cycle through it is shorter than the
  slack `value − need` and the sets of other sizes lose more than that (`Meet::has_cycle_within`).
* **Exact strong branching.**  The member of the conflict to branch on is the one without which the intersection
  is lowest, each bound read off the exchange graph where it is certified and solved where not (in the order of the
  split's bounds, each solve stopped once it reaches both the lowest so far and `need`, a greedy lower bound tried
  first); a member whose bound falls below `need` is forced too, and the chosen member's level, certified or solved,
  is what the node goes on with.  The cheaper choices (the split's bound alone, the largest member) take more nodes
  and more time (EXPERIMENTS: "The GF(2) bound in the search").
* **What it replaces.**  With `gf2` the matching bound is not computed (as a first check before the intersection it
  costs more than the cuts it saves), so `match_refine` has no effect; the conflicts, nogoods, forced members and
  exact sets are used as with the matching bound.  The Lagrangian bound over nogoods still applies after it when on,
  but changes no tree of the GF(2) search measured and costs 2–4%, so `lagrange` is off by default.
* **Block words.**  A level's vectors are as wide as the root's largest frame rank (its freedoms) needs: one to three
  words of 64 bits on every board to 4095 (ranks up to 169), four from 4740 and five from 6392 (up to 314 at 8191; the
  scan of every board is in EXPERIMENTS, "Boards past 4095").  Each word count is a copy of the bound's code
  (`Levels<W, R>`, R = 1 to `gf2::MAX_WORDS` = 5) for every set width that has it; only the sets of 128 words have the
  copies of four and five words, so the narrower widths are built as before.  Tests run those copies on small boards
  with the vectors padded with zero words (`Gf2Graph::padded`), where they must search the same tree.
* **Open for other weights.**  `meet::Problem` takes any positive weights, but the start from the heaviest
  uncontended elements and the greedy lower bound assume that weights do not decrease with an element's index (the
  root's candidates by increasing number).  A bound that reweighs the candidates (a Lagrangian over nogoods or a
  cover LP with this intersection as its inner problem, as the nogoods study suggests) needs its own element order,
  or those two starts switched off; the levels, the contraction and the split do not depend on the order.

### Prime exchanges

Exchanging two primes `p < q` in every number (`p^a q^b m ↦ p^b q^a m`) is an automorphism of divisibility.  Where it
maps every frame of the root onto itself (moves to moves, factors to factors) and the candidates onto themselves, it
maps promotion sets to promotion sets, worth `(q − p)(c_p − c_q)` more, `c_r(S)` being the sum of `x / r` over the
members of `S` divisible by `r` (no candidate is divisible by `pq`, `p²` or `q²` in the pairs used).  So every
heaviest promotion set keeps `c_p ≤ c_q` for every such *symmetric pair* at once: a set that breaks one pair's
inequality is beaten by its image under that pair's exchange alone
([heuristics/THEORY.md](../heuristics/THEORY.md#prime-exchanges-symmetries-of-the-promotions), Theorem 13).  The
pairs come in classes of primes that the frame builder treats alike (at 1300 {41, 43}, {59, 61}, {67, 71}, {73, 79},
{89, …, 107}, {109, …, 139}, {149, 151, 157}: 32 pairs; 99 at 2175, 300 at 4000), and `search::symmetry::root_pairs`
finds them when a search is set up, in under a millisecond: the full check only for the primes whose multiples have
the same places and exponents in the frames, and over the multiples alone (EXPERIMENTS: "Dominance and symmetry"
found the exchanges, "Prime exchanges in the search" put them in).

With the tactic `symmetry` (the default) the search uses them, from the cofactor sums of a node's promotions `P` and
candidates `C` (`Symmetries::exclude`, a few hundred bit tests: under 1% of the time of a proof of 1472):

* a node is cut when some pair has `c_p(P) > c_q(P) + c_q(C)`: no completion lifts `c_q` to `c_p` (it is
  *hopeless*);
* a candidate `mp` with `c_p(P) + m > c_q(P) + c_q(C)` is in no completion that keeps the pair's inequality, and is
  dropped from the node's candidates, which tightens its bound (repeated until nothing more drops, a drop lowering
  `c_q(C)` of other pairs).  Children are made without them, and a node's later steps drop more as the node goes on
  without the members it branched on; the dropped candidates leave the GF(2) and matching levels as a branch member
  does.

The drops are most of the gain: the hopeless test alone takes 0.97 of the nodes at the proof of 1287, the rule
0.57.  A Lagrangian of the inequality on the GF(2) bound (the heaviest set under weights `m(p − μ)` and `m(q + μ)`
for the pairs its heaviest set breaks) was measured and not kept: it cut little that the drops left, and cost more.

The rule is sound for the search of the root's problem: if the incumbent is not optimal, every heaviest set beats it
and keeps every inequality, and the other prunings (bounds above every completion, members that every completion
beating the incumbent promotes, sets that cannot be promoted) hold for the completions that keep the inequalities as
for all.  It learns nothing: a cut ends a loop and a drop shrinks a candidate set, while the conflicts and nogoods the
search learns are facts about promotability, which hold everywhere whatever the rule cut; the rule changes which
conflicts a searcher meets, as any change of the tree does, never their truth.  It is a function of a node's `P` and
`C`, so a unit (a subtree of the root) rebuilt from a checkpoint or on another thread keeps the guarantee.  A search
of one node's completions for their own best, as the soundness harness runs below the root, finds the best of those
that keep the inequalities, which can be lighter.  The matching path (`SearchTactics::matching`) is without the rule,
as it was before it.

### The default step

One step of a node's loop (`Stepper::step` in `src/search/searcher.rs`; the module comment of `src/search/mod.rs` says
which of its four phases takes which of the steps below), with the default tactics:

```
need = best + 1 − sum(promoted)                   # the loop goes on while the bound reaches this
1. end the loop if cands is empty, or the max_add largest candidates sum below need; unless this is the node's
   first step (its candidates as step 7 left them), drop from cands the candidates that no completion keeping the
   symmetric pairs' inequalities holds, and end the loop if none keeps them (symmetry), or if what is left is empty
   or its max_add largest sum below need
2. look at the node's heaviest common independent set I* (gf2) of at most max_add members:
     value < need                                  -> end the loop (cut)
     F = the forced members of I* (by the weight split: those without which the bound is below need; and those
         near it whose exchange graph has no cycle through them within value − need)
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
   inequalities (symmetry), or its GF(2) bound is below it, or its
   heaviest set is exact (offered as incumbent); a child that stays open keeps this verdict, with its forced
   members, for its first step; a child with no candidates is not pushed
9. otherwise offer the child's own promotions as incumbent, and push it
```

On the matching path (`SearchTactics::matching`, `--tactic gf2=off --tactic lagrange=on --tactic symmetry=off`: the
default before the GF(2) bound, without the rule of symmetric pairs, which it takes too if asked) step 2 looks at
the heaviest matching `Q*` instead: its forced members from the potentials, then, where none is
forced, refinement of its conflict `match_refine` levels deep (a cut if every branch is below `need`), then the
Lagrangian bound over the nogoods (a cut, or more forced members); step 4 chooses the member whose removal lowers
the matching bound most; and step 8 checks the child's matching bound, refinement and Lagrangian bound.

Exactness rests on five facts: the bound is an upper bound (the GF(2) bound by Theorem 11, the matching bound with
refinement, and the Lagrangian bound for any multipliers); every promotable completion leaves out a member of a
conflict, so branching on a member and then going on without it explores every promotable set; forced members (by
weak duality, for the weight split and the potentials and the Lagrangian bound alike) are promoted by every
completion that can beat the incumbent; nogoods and known conflicts hold at every node they are applied to
(subset closure); and every heaviest promotion set keeps the symmetric pairs' inequalities (Theorem 13), so the
completions the rule leaves out hold none of them.  A node is cut only when its bound is below `need` (a completion
that merely ties the incumbent is of no use), and a unit rebuilt from a checkpoint is a node whose bound can only be
tighter.  `tests/soundness.rs` attacks each of these (section 9).

### The starting incumbent, the ceiling and the seed

The search starts from the best of three promotable sets: the *greedy* set (promote every candidate that can be, from
the largest down), the *seed* (the previous game's promotions that are candidates and still promotable, one by one
from the largest), and a rounding of the root's matching (the *dive*: drop the smallest
member of a conflict until the rest can be promoted); then a local search improves it (fill greedily, then drop one
or two members and refill, until nothing improves).  The *ceiling* on the promotion sum,
`n + score(n − 1) − in_the_bag`, is proven; reaching it ends the search (`prev_cap`).  When the greedy set or the
seed's survivors already reach it, the dive is skipped, and the local search when the incumbent after the dive does:
nothing can beat a set at the ceiling, so neither would change it, and the result is the same.  The local search runs
on the search's threads, the refills of a pass in parallel, and keeps the first improvement in the order of the
sequential pass, so its answer does not depend on the thread count; it reads the ceiling before each pass, so one given
while it runs (the control's) ends it as soon as its set reaches it.  The start is offered to the run's control as it is
made (after the greedy set and the seed, the dive and the local search), so whoever steers the search sees the best so
far before the engine starts, and the engine's start is unchanged.  Board/search setup is in
`board::Board`: it is the one place that knows the tables → frames → prepare → ceiling and seed path.

## 5. Tactics

Each row is a tactic in `tactics::Tactics`, set from the CLI with `--tactic name=value` (`on` or `off`; `match_refine`
`0`…`7`).  The ones that act inside the search are the fields of `search::SearchTactics`; `examples/ladder.rs` takes
the same names for them.  Status words: *proven* (a theorem, given a premise) and *sound* (never prunes a promotable
optimum); every tactic is one or the other, and switching one off changes how fast an answer comes, never the answer.
A test (`tests/docs.rs`) checks that this table lists the tactics of `tactics::Tactics`, in order, with their
defaults.

| tactic | default | status | effect |
|---|---|---|---|
| `prime_shortcut` | on | proven (given an optimal n−1 game: proof at `accelerate::prime_shortcut`) | prime n: replace the n−1 solution's first move with n |
| `same_search` | on | proven (given the proof of the earlier game: section 8, "Same search") | a game whose search problem an earlier game of the chain posed and settled is that game's promotions on this board, with no search (method `repeat`, proof `same_search`); new in the Rust |
| `match_refine` | 1 | sound (see `search::refine::Refiner`) | refine the matching bound by branching on conflicts this many levels deep, where it does not prune (0 = off, at most 7); no effect with `gf2`, which does not use the matching bound |
| `fix_forced` | on | sound (every completion that beats the incumbent promotes them) | a node's child promotes all its forced candidates; the node's loop ends after that child |
| `nogoods` | on | sound (a nogood holds at every node; see `model::Nogoods`) | learn each conflict as a nogood of the root and use the table at every node, for the conflict to branch on and refine with and to rule candidates out of children |
| `forget_nogoods` | on | sound (the table is a cache) | with `nogoods`, each searcher forgets every nogood it knows after each 50,000 of its steps |
| `lagrange` | off | proven (weak duality, for any multipliers over valid nogoods; `matchbound::lagrange`) | with `nogoods`, where the node bound leaves a node open, bound it also by a Lagrangian relaxation of the searcher's nogoods kept along its path (over the matching bound), and (with `fix_forced`) force the members without which that bound would cut; the multipliers are refitted at the root as the searcher learns (section 4).  On by default until `gf2`; with `gf2` it changes no tree and costs 2-4% |
| `gf2` | on | proven (heuristics/THEORY.md, Theorem 11; weak duality for the forced members) | bound every node by the GF(2) bound (section 4) instead of the matching bound and its refinement, branch on the conflicts of its heaviest set by exact strong branching, and (with `fix_forced`) force the members without which it would cut; the Lagrangian bound over nogoods, if on, applies after it |
| `symmetry` | on | proven (heuristics/THEORY.md, Theorem 13) | cut a node whose completions all break a symmetric pair's inequality, and drop from a node's candidates those that no completion keeping the inequalities holds (section 4, "Prime exchanges") |
| `prev_cap` | on | proven if opt(n−1) is optimal | stop as soon as the incumbent reaches n + opt(n−1) − in_the_bag (`FrameSolver.getMaxPromotionSum`) |
| `seed_incumbent` | on | sound (any promotion set is a valid incumbent) | start from the better of greedy and the n−1 solution's promotions that are still promotable |

What each tactic was worth, and the order they were found in, is in [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md); so
are the tactics that were removed: the Java's two conjectures, `reuse_prev` (accept the previous game's promotions
with no search) and `revise_freedoms` (lower frame 0's freedoms by solving the two-frame game, assuming that the
heaviest promotion set also has the most members, which is false at n = 833), and the tactics that lost, the Java's
`blameGame` bound with its runner-up, its preclusion table, and the branchings other than strong branching
([which tactics earn their keep](docs/EXPERIMENTS.md#which-tactics-earn-their-keep-2026-10)).

**Model assumptions** (unproven, inherent to the frame model, validated as in section 2): greedy removal of impossible
sources inside frames (the Java's `Hive.removeImpossible`); mandatory moves (its `Hive.unzip`); freedoms = factors −
sources (its `freeFactorCount`, set by `Hive.finishSetup`); closed frames take no part in the search; a promoted number
pays tax only from the frame one forward.  They define the search space and cannot be switched off inside this model.

**Where tactics are recorded.**  A result line records the tactics it was produced with (`tactics`: the changes from
the defaults, such as `lagrange=on,gf2=off`; `tactic_switches`: every tactic's value; a line of a build before
`gf2` has no such key, and ran the matching path).  A checkpoint records
the search tactics and is refused when they differ (the error names each tactic that differs and the `--tactic`
flags that resume it).

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
  raise it, and a searcher can declare it optimal, which ends the run, as an incumbent that reaches the ceiling given
  through the control (below) does.
* **Checkpoint.**  Every `checkpoint_interval` all workers yield and park, the engine writes the caller's header, the
  incumbent, statistics and every unit (memory and spilled) to `<checkpoint>.tmp`, fsyncs, renames it over the
  checkpoint and syncs the directory, and releases the workers.  Resuming loads the file and continues; nothing is lost
  or duplicated beyond partially explored nodes.  Any interval works, zero included.  The caller's header identifies the
  search (section 8: it is what makes a checkpoint of another game, width, build or tactics refuse to resume).  A run
  that completes deletes its checkpoint unless the caller asks to keep it until it has recorded the result.
* **Stopping and sharing.**  Time and node limits, and SIGINT and SIGTERM (after `stop_on_signals`), stop a run with a
  final checkpoint; a second signal kills the process.  A stop that already holds when a run starts (a time limit of
  zero, a signal received before) does no search work, however the threads are scheduled: the run checks before it
  starts a worker (engine docs, "Starting stopped").  `taxman solve` takes an exclusive `flock` on `taxman.lock` in
  its checkpoint directory, so a second process there fails at once.  Spill segments carry their process id, and a run
  deletes those of processes that no longer exist.  Progress lines and results on stdout ignore write errors, so a
  closed pipe cannot kill a run.  A panic in a release build aborts (`panic = "abort"`); the previous checkpoint stays
  valid, which is the property that matters.
* **Control** (`engine::Control`): a handle through which another thread steers a run while it goes: a ceiling (a
  proven upper bound, which ends the run once the incumbent reaches it), outside incumbents, a stop, and the number of
  workers, which grows by starting workers and shrinks by retiring workers that hold no unit (a running one hands all
  its units to the deque first); 0 suspends the search, its units waiting in the deque until the number rises again.
  Requests made before the run apply when it starts (engine docs, "The control").
* **Instrumentation** (`engine::Counters`, per invocation): each worker's time split into busy, idle, parked and lock
  wait; the effective parallelism busy / capacity (the workers' lifetimes summed, wall × threads when their number is
  fixed); splits and the units they gave, steals, runs and tiny runs; when all workers first ran at once; the workers
  added and retired; the idle time of the ramp and of the tail, the time to a unit's first step (the rebuild) and the
  time in it; the pauses and the longest wait for the workers to park.  Every progress line ends with them in short
  (README, "Reading the output").

## 7. Per-game pipeline (`solver`)

`solver::solve_game` solves one board size, and `solve_range` a range; the exact steps and checks are the module
comment of `src/solver.rs`.  Each step, with the Java class it reimplements:

| step | what | Java |
|---|---|---|
| 1 | the board's tables, and the set width | `FactorTable` |
| 2 | prime shortcut: a prime n plays the game of n − 1 with its first move replaced by n (method `prime`) | `FrameSolver.solutionForPrime` |
| 3 | frames, formed, simplified, numbered and merged; with no candidates the game with no promotions is the only one (method `trivial`) | `bombus.Apiary`, `Hive`, `frame.FrameBuilder`, `WorkingFrame` |
| 4 | the search (method `search`): the frame set and greedy incumbent (`search::prepare`); a repeat of an earlier game's search (method `repeat`, tactic `same_search`, section 8); the starting incumbent, ceiling and seed; the engine; promotions → moves | `search.SearchManager.create`, `Frame`, `FrameSet`, `LowerBoundEstimator.greedy`, `BranchExplorer`; `Apiary(board, promotions).getSolution()` |
| 5 | `conclude`: replay and check the game, confirm its score against optimal.json, assign its proof and what it assumes | `game.Solution.verify` |
| 6 | record the result (a range) | |

Step 2 needs the previous game, and the repeat of step 4 an earlier one; both are skipped by `force_search` and when
the game has a checkpoint (a search that was interrupted is resumed, never replaced by a derivation).

The previous game is checked before anything uses it: it must replay legally to its recorded score, be over, and open
with the largest prime (a game this run made was checked by `conclude` already).  The results whose proofs rest on it
assume what it assumes (section 8).

A search that a time or node limit stops gives an incomplete result (`completed` false, `best_so_far` the best game
so far) and keeps its checkpoint; a range stops there.

**Range runs** feed each game's solution to the next and resume from the results file, which is checked when it is
opened (every game replayed, the chain from n − 1, each line's arithmetic and provenance; the search problems that the
checks of repeats set up again are set up first, on the run's threads).  Games completed in it
are skipped; each result is appended and synced as soon as it is concluded, in the order of n, and only then is the
game's checkpoint deleted (a checkpoint left for a game the file has completed is deleted when a range starts).  The
n − 1 solution comes from, in order: this run, the results file, and with `use_prev` (`--use-prev`) optimal.json; with
none, the game is solved standalone, with no seed and no ceiling.

### The window: games in flight

A game of the chain needs two things of the game before it: its moves, for the seed, and its score, for the ceiling.
Everything else (its board, its frames, its search problem, whether an earlier game settled that problem) is its own,
and the ceiling only stops a search early.  So a range keeps a *window* of K consecutive games in flight (`--window K`,
`SolveOptions::window`; 1 by default, and always 1 with one thread; `tools/cloud/chain.sh` runs with 16): game n + 1 is
set up while n searches, its search may start before n is proven (*speculatively*), seeded from n's best game so far
and without a ceiling, and it takes n's ceiling and the seed of n's final game the moment n is concluded, through its
engine's control (section 6).  `solver::window` is the driver.  With K = 1 it is the range solved in order, line for
line: a game enters the window only once the game before it is concluded, so its search starts with the final seed and
the ceiling, on every thread.

* *Phases.*  A game is set up on a thread of its own (the prime shortcut, the trivial game, or the search problem); its
  repeat is looked up on the driver's thread in the order of n (a game of the window that set a search up counts, see
  section 8); its search runs on its thread; and it is concluded on the driver's thread when it is the oldest game of
  the window and the game before it is concluded: the result is built and checked (`conclude`), appended and synced,
  and only then is the next game told its ceiling.  A repeat waits for the game it repeats.
* *Seeds.*  A speculative search starts once the game before it has a game to seed it with: concluded, or its search's
  best so far (a search offers its start to its control as the start is made, section 4), or what that gives through a
  repeat or the prime formula.  Every 5 ms it is offered the survivors of that game again whenever its score has
  improved, and at the conclusion those of the final game.
* *Deferring.*  A search whose seed's survivors reach the *provisional ceiling*, n + the score of the game before's best
  so far − `in_the_bag`, waits for that game's conclusion: if that best is final, the ceiling settles the game at once,
  and a speculative search of it would be wasted.
* *Threads.*  `--threads T` is a budget the window shares (`solver::window::allot`, a pure policy, unit-tested): a setup
  holds one thread, a search the workers of its engine, which takes on workers and retires them while it runs
  (`Control::set_workers`; 0 suspends it, its units waiting in its deque).  Every 5 ms the driver samples each engine
  and serves the claims oldest game first: a setup asks one thread; a search ready to start every free thread if it is
  the only claim, else one; a busy search (idle under 5% of the last tick) asks to double; a search idle 25% or more
  gives its idle workers back to another game's claim, unless it was given threads in the last 10 ms; and the oldest
  search, if it is left short, reclaims threads from the younger ones, youngest first, suspending them if need be.  So
  the game the chain waits for runs on every thread it keeps busy, speculative searches run on what it leaves, and the
  threads of a search that ends or idles go to the oldest game that can use them.  The tree is not estimated: a search
  earns threads by keeping them busy, the online form of a running estimate of what is left of it.
* *Stops.*  A failure is reported, and an unfinished search ends the range, only when its game is the oldest, so a run
  fails or stops at the game, and with the lines before it, where a range solved in order would.  The searches still
  in flight are then stopped with a final checkpoint (the next run resumes them), and what they found is dropped.
* *Trace.*  With `TAXMAN_WINDOW_TRACE` set, the driver prints its events on stderr: setups, the starts of searches with
  their seeds and ceilings, workers given and taken back, hints, deferrals and conclusions.

What it buys and what was tried on the way are in
[docs/EXPERIMENTS.md](docs/EXPERIMENTS.md#round-b-the-chain-on-many-threads-2026-10).

**optimal.json** is built into the binary (`include_str!`, `game::Optimal::built_in`): the table as it was when the
crate was compiled, which a rebuild picks up when the file changes.  Every completed game's score is confirmed against
it, and a score that differs is a hard error naming the game; no option names another table, so a run cannot be
checked against anything else.  The library takes another table where a test needs one
(`solver::solve_range_each_with`, `results::read_results`).

## 8. Results and the trust model

Every completed game records *how its optimality is established* (its proof) and *what that assumes beyond the frame
model* (`assumes`); `src/results.rs` has the full rules and the checks.

| proof | meaning | rests on the game of n − 1? |
|---|---|---|
| `trivial` | the board has no promotion candidates | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached `n + score(n − 1)`, which no game of n can exceed | yes |
| `prime_formula` | a prime n plays the game of n − 1 with its first move replaced by n (proved at `accelerate::prime_shortcut`) | yes |
| `same_search` | an earlier game m posed the same search problem and settled it; the game is m's promotions ("Same search", below) | no: on the game m |

What a game *assumes* is what its proof rests on beyond the frame model: if its proof rests on the game of n − 1,
what that game assumes, and so on down the chain (a repeat rests on the game it repeats instead, and assumes what that
game assumes).  A game taken from optimal.json (`--use-prev`) is assumed optimal, since the table records nothing
about how a game was found: a proof that rests on its game of n − 1 assumes "the game of n − 1 in optimal.json is
optimal".  A game that assumes nothing is proven within the frame model, given that the solver is right.  A chain from
n = 1 assumes nothing beyond it.  Nothing refuses a game for what it assumes; the line says it, and the verifiers
count it.

**Checks and referees.**  Every game is replayed by `game::replay_finished` before it counts, with the arithmetic and
the ceilings `score(n−1) ≤ score(n) ≤ n + score(n−1)`; a game that fails after it was found goes to
`<results>.rejected.jsonl` and the run fails.  `taxman verify-results` reads results files without changing them and
runs the same checks plus agreement between files; `tools/check_results.py` is an independent referee in Python that
re-derives every proof and assumption and compares scores with optimal.json, the Franklín–Moniot upper bound and the
best-known heuristic chain.  Both read results lines of schema 5 only, and refuse a line whose fields contradict each
other (say a sum at the ceiling without `reached_cap`), since the proof and the assumptions are derived from them.  What
no check can catch is a game that is legal, finished and within the bounds but not optimal when the game of n − 1 is
not optimal either; that is what the proof kinds and the assumptions track, and why a chain from n = 1 matters.
[results/README.md](results/README.md) says what supports each committed score.

**Result lines** are one JSON object per line (README, "A results line"): the game, its proof and assumptions, the
game of n − 1 it was built from (score, source, digest of its moves, proof, assumptions), node counts and search
counters, seconds per
phase, the tactics, limits, options and the commit the binary was built from; the digest of the search problem a
setup poses and a repeat's record of the game it repeats.  A line is never rewritten, and a run locks the results
file it appends to.

**What identifies a checkpoint.**  The header the search stores: a format version, the set width, n, digests of the root
candidates and of the frames (every frame's moves, factors and freedoms, as built), `max_total` and the tactics (format
version 4; this build refuses every other version with one message, so a checkpoint of an older build cannot resume with
tactics that are gone).  Resuming checks all of them and says which differs, and checks that the stored incumbent is a
set of candidates that can be promoted from the root and is worth the stored value; so a checkpoint of another game, or
of an older build whose frame builder differs, is refused instead of pruning with a foreign incumbent or failing to
rebuild its units.  A resumed game rests on nothing a game that ran in one piece does not.

### What a window changes in the results

A range solved with a window of games in flight (section 7) records what the range solved in order records, but for
what a multi-threaded search may vary:

* A line is appended only after every line it rests on: that of n − 1 (its `prev`) and, for a repeat, that of the game
  m it repeats.  Both are earlier lines, and lines are appended in the order of n.
* `prev`, `same_search` and `assumes` are derived as in order, from concluded games.  A repeat names the same m: n looks
  among the eight board sizes before it for the latest game whose problem is n's, counting a game of the window that
  set a search up and is not concluded yet.  In order, that game is concluded when n looks, and every search or repeat
  that completes settles its problem, while one that does not complete stops the range, or fails it, before n's line
  could be written.
* The proof kinds are those of the range solved in order.  A completed search's promotion sum is its problem's best,
  P\*, and its proof is `cap` exactly when P\* reaches the ceiling, however the search ran.  A search started before
  its ceiling was known may have explored its whole tree to an optimum at the ceiling: its line records `cap` and
  `reached_cap`, the weaker of the two proofs it has, as the verifiers' rule wants (a line reaches the ceiling exactly
  when its promotions sum to it).  One stopped by a signal or a limit with its best at the ceiling is complete by the
  ceiling, as it would have stopped there given the ceiling from the start.
* A seed from a provisional game, a hint offered while a search runs, a ceiling given late, a deferral and a
  suspension change how a search runs, never what it proves: every set offered is promotable (the survivors of a
  seed), a deferral only delays a search, a suspended search keeps every unit, and the ceiling is proven given the
  game of n − 1 optimal, which a proof by `cap` records that it rests on, as before.
* What may differ: the moves and promotions where a problem has several optimal sets (as between thread counts), and
  with them the next line's `prev.moves_digest`; the nodes, the search's counters, the engine's, and the times.

### Same search

Consecutive boards often pose exactly the same search problem with another ceiling: of the 1836 games of the committed
chain 1..2175 that need a search, 914 pose the problem of the game before them (or the one before that, past a prime).
The chain of commit bd9bb5d, which searched the 545 of them in 1..1300, spent 953 of its 3359 s on those, 851 s of them
in 1232–1300 (EXPERIMENTS, "Same search").  The tactic `same_search` (proven, on by default) concludes such a game from
the earlier one with no search; the committed `results/strict.jsonl` was made with it.

*The problem.*  After setup (`search::prepare`) the searcher is given the model (the factor graph among the frames'
numbers and each number's home frame), the root frame set (every frame's moves, factors and freedoms), the candidates
and `max_total`, besides the tactics, the ceiling and the seed; the greedy set and the matching graph are functions of
those.  Call Π = (model, root, candidates, `max_total`) the game's *search problem*, and P\*(Π) the largest sum of a
*promotable* set of Π: a set of candidates that can be promoted from the root, keeping every frame winnable and its
freedoms at least 0, with at most `max_total` members.  Which sets are promotable depends on the frames and on the
graph's edges among their numbers, and on nothing else: promotion and reduction see only the numbers in the frames, and
an edge is the maximal-factor relation (y = x/p, p prime), a property of the two numbers that does not depend on n.
`search::Problem` writes Π out canonically, as numbers (so independently of the set width): the frames front first, each
its freedoms, moves and factors; the candidates; `max_total`; and for every number of the frames its home frame and the
lists of maximal factors and maximal composites the model's graph keeps for it, which name some numbers that reduction
removed and the search never sees, erring on the side of telling problems apart.  Equal forms are equal problems, with
the same promotable sets and the same P\*.  The tactics, the ceiling and the seed are not in it: they change how the
search runs, not what it finds (section 4).  A results line records the form's 64-bit digest (`setup.digest`); nothing
rests on the digest alone (below).

*Which proofs settle P\*.*  A game m whose proof is
* `exhausted`: the search explored the whole tree of Π_m, so the sum of its promotions is P\*(Π_m), given that the
  pruning is exact, which every exhausted proof rests on;
* `cap`: its promotions S are promotable and sum to ceiling(m) = m + score(m − 1) − in_the_bag(m), so P\*(Π_m) ≥
  ceiling(m).  And every promotable set T of Π_m is a legal game of m scoring in_the_bag(m) + sum(T): this is the
  direction of the frame model that every search relies on each time it offers an incumbent, and the referee replays
  every game built from one (none has ever failed).  So in_the_bag(m) + P\*(Π_m) ≤ opt(m) ≤ m + opt(m − 1) = m + score(m
  − 1), given that the game of m − 1 is optimal (which the proof by the ceiling rests on, and m's assumptions record),
  and P\*(Π_m) = ceiling(m), the sum of m's promotions, whatever tactics shaped the search;
* `same_search`: by induction, m repeats a game k with Π_k = Π_m whose proof settles P\*(Π_k), and m's promotions are
  k's.

A trivial game has no candidates and sets no search up, and a prime one is the prime formula's and sets none up
either: neither is ever repeated, and the verifiers refuse a repeat that names one.  An unfinished search settles
nothing.

*The repeat.*  If game n poses Π_n = Π_m and m's proof settles it, then P\*(Π_n) = P\*(Π_m) = P, the sum of m's
promotions S, and S is promotable in Π_n.  The game of n is S played on the board of n (`solution_for_promotions`), and
it goes through `conclude` like any game: it must replay legally, be over and open with the largest prime, score
in_the_bag(n) + P, stay within `score(n − 1) ≤ score(n) ≤ n + score(n − 1)`, and match optimal.json where that has n.
It is optimal on the premise of an exhausted proof of n, the frame model's (the best game of n scores in_the_bag(n) +
P\*(Π_n)), with P\*(Π_n) established by m's proof instead of a search.  Its proof `same_search` rests on m, not on
the game of n − 1, and it assumes what m assumes.

*Finding m.*  Before n's search (and only for a search that is not resumed from a checkpoint, and without
`force_search`), the solver looks at the completed games of the results file and of the run among the
`SAME_SEARCH_WINDOW` = 8 board sizes before n, latest first, for one that set a search up and whose proof settles it.
A game whose recorded digest differs is passed over; for one whose digest agrees, its problem is set up again from
its board size (milliseconds; `board::search_problem`), or, when the run set that board up itself, taken from memory
(the problem its setup posed, which is the same problem), and compared with n's whole.  The first equal one is m.
Setting m up again with this build stands for the problem m's own build searched, which the digest recorded by that
build ties to it (a problem with the recorded digest is that problem, but for a 64-bit collision), and the problem
set up again must also have the size its line records (frames, candidates, `max_total`).  Every repeat of 1..2175
repeats the last game before it that set a search up (n − 1, or n − 2 past a prime), so the window loses nothing
there, keeps the setups a range starts with to eight, and makes m a function of the window's games.

*The verifiers.*  `taxman verify-results` checks a repeat's line against m's: m is a completed game of the same file on
an earlier line, its proof settles its search, the line's record of m (proof, assumptions, digest) is m's, the
promotions are m's and sum to the recorded promotion sum; then it sets both problems up again from the two board sizes,
checks that each is as large as its line records, that their digest is the recorded one and that they are equal, and
that both games score their board's in_the_bag plus that sum; and it derives the proof and the assumptions (m's) as for
any line.  `tools/check_results.py` shares no code with the solver and cannot build the model: it checks the same
structure from the lines (m earlier, completed, settling its search; the recorded digests and sizes equal; the
promotions equal; score − in_the_bag equal; the assumptions inherited), and says that it takes the problems' equality,
like an exhausted search, on trust.

## 9. Testing

**Before a change is merged** (what a reviewer of a subagent's work runs, besides reading it; the tests are described
below, the probe in section 10):

* every change: `cargo test --release` in `rust/` and in `rust/prover/`, then `cargo clippy --all-targets` and
  `cargo fmt --check` in both;
* a bound, or anything that prunes: the bound-soundness tests and the soundness harness pass, and a bound never falls
  below the best completion of its node;
* a pure speedup: every search tree identical, so the probe's fingerprints and the node-count tripwires do not move;
* anything that touches the search, the chain or the results: `taxman regress --to 1000`; a scratch chain (`taxman
  solve 1-N --out FILE`, not in `results/`) compared with `results/strict.jsonl`; and both verifiers on it, since a
  results line carries provenance that they check (`tools/cloud/verify.sh FILE` runs `taxman verify-results` and
  `tools/check_results.py`, and compares with the committed chain);
* a change to an on-disk format (results line, checkpoint header, `testdata/`): the commit message says so, and why.

Correctness, all on games that finish in seconds: the fast fixtures and small boards stand in for the large boards
that cannot be run in a test, and the tripwires and soundness tests catch the failures that no assertion on an
answer sees.  `cargo test` takes about half a minute after compiling, with two test threads, in the test profile
(`opt-level = 2`, because the tests replay real games); the engine's heavy tests are ignored by default.

* **Unit tests** per module, including the Java tests' cases (`FactorTableTest`, `FrameTest`, `FrameSetTest` for n =
  85). Helpers shared by the unit and integration tests (temporary directories, a random generator) are one file,
  `tests/common/support.rs`.
* **Golden tests** (`tests/golden_frames.rs`, `tests/solver.rs`): every stage of the port against
  `testdata/golden/n*.json` (exported from the Java by `tools/golden/export.sh`): the setup's frames, the numbered and
  merged frames, candidates, reduced frame set, greedy incumbent, the prime shortcut, solve result.  The searched golden
  games (94, up to 760) give the Java's promotion sums with the default tactics; the tactic variants give the same
  answers on those up to 400.  Past 760 the Java's structure is fixed in `testdata/golden_wide/` (n = 511, 512, 1000,
  1001, 1023, 1024, 1025, 1100, 1231, 2047 and 2048, checked at the width `with_width!` picks) and `games_java.txt` (the
  Java's game of the board's frames, move for move, for 171 sets of promotions at 762–2049).
* **The board layer against the chain** (`tests/board_layer.rs`): for every 25th searched game of `results/strict.jsonl`
  (and the last, n = 2175), the moves that the game's promotions make and the digest of the board's search problem are
  the line's, which the golden data (up to 2049) cannot say of the boards that matter.  The unit tests of `frames` check
  what the frames remember (which numbers each can take on, which sources `removeImpossible` has been through) against a
  fresh computation.
* **Regression**: `taxman regress --to N` solves 1..N as one range and checks every score against optimal.json.
* **Tactic invariance**: games 1..300 forced to search with each tactic off or changed must give identical scores,
  and a checkpoint is refused when any of the search tactics differs.
* **Bound soundness**: on small frame sets, the upper bound is never below the exhaustively computed best completion;
  the same for the matching bound and its refinement by conflicts (whose exact completions must be promotable, and
  whose conflicts, carried down a path, must stay conflicts), and for the Lagrangian bound over nogoods of the root
  with fitted and random multipliers.  The incremental solvers agree with the reference min-cost flow (with the
  Lagrangian weights, live nogoods and constant worked out afresh, under random relaxations changed mid-walk) and
  with a solver started afresh, and their levels pass the optimality certificate.  The GF(2) bound (`src/gf2/tests.rs`,
  `src/gf2/meet.rs`): the intersection against brute force under every size limit, from every set it reports as a
  warm start, with members deleted, and its weight split's bounds without each member; on the boards up to 400 the
  bound is never below the exhaustive best completion and the optimal promotions are independent in both matroids;
  and along random paths of games 60–1000 (promotions, deletions, backtracking) every level equals the matroids built
  afresh at its node, and every member it calls forced is (the bound without it, solved afresh, is below what the
  node needs), and along the same kind of paths the copies of every word count (1 to 5, padded) give the same results
  and do the same work; blocks of more than 255 members (five words) against the greedy algorithm, one matroid free
  and the other random; at the 600 nodes of the probe's fixtures (936 and 1215), completions promoted in random orders
  are independent in the node's matroids and no heavier than its bound, and the matching path's search started from
  the bound finds nothing above it (`tests/soundness.rs`).  Conflicts and residuals are checked against promoting the
  sets.
  The prime exchanges (`src/search/symmetry.rs`): the symmetric pairs against their definition (every number looked
  at, every pair of primes tried) on the boards 2..=300 and a sample to 2200; the images of the optimal set and of
  random maximal promotable sets promotable and worth the formula, every optimum keeping every inequality, and on the
  boards with at most 22 candidates every promotable set so checked and the best that keeps the inequalities the
  optimum; the rule's cuts and drops against its definition by enumeration (exact where the pairs are independent).
* **Soundness harness** (`tests/soundness.rs`): the search is started with an incumbent worth 1 or 5 less than the
  optimum and an empty payload, so that any pruning that loses every best completion leaves the fake incumbent in place
  (this tests the `<` and `<=` boundaries too): every game 2..=300 under each of 34 configurations (19 of the matching
  path: every tactic, forgetting every step, every third, never; the Lagrangian bound refitted every few steps; one
  thread and two; 11 of the default, the GF(2) bound with the rule of symmetric pairs: every way of choosing the branch,
  the matching bound as a first check, without forced members or nogoods, forgetting every step, two threads, the
  Lagrangian bound after it; the GF(2) bound without the rule on one thread, two, and forgetting every step; and the
  matching path with the rule), every board 2..=600 with symmetric pairs with the rule over either bound (and the oracle
  without the frame model to 40), units on the way to the optimum (an optimal set less 30 to 70% of its members, boards
  600..1000) with the rule, and games 301..=600 (every tenth, searches of hundreds to thousands of steps) with the
  Lagrangian bound refitted early and often, over either bound, where it cuts and forces thousands of times; the same
  for 500 perturbed frame sets (numbers dropped, role flipped, moved a frame) with the truth found exhaustively;
  promoting a set at once against one by one in random orders, conflicts and nogoods on perturbed models; and searches
  interrupted every 1,000 nodes and resumed from a checkpoint with units spilling (game 600 on the matching path, one of
  them with the Lagrangian bound refitted every four steps; game 1000 with the GF(2) bound, whose search of 600 is too
  short to stop).  Units of real boards at W = 20 to 64 (thinned so that the truth is found exhaustively) from a
  handicapped incumbent, and games 1133 (matching) and 1000 (GF(2)) resumed every 1,024 steps at W = 20, 24 and 64,
  carry the harness past the widths of the Java's table (n ≤ 1000).  A search below the root with the rule of symmetric
  pairs is held to the best completion that keeps their inequalities (an exhaustive oracle of its own, which prunes by
  the same bound but shares no code with the rule), which it must reach; at the root that is the optimum (Theorem 13).
* **Node-count tripwires** (`tests/tripwires.rs`): a pruning tactic that stops working changes no answer, so no
  assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` costs 27 to 67 % more nodes).
  With one thread the tree is deterministic, so the exact node count of a fixed game under fixed tactics (n = 480 to
  1133, every tactic off or changed in turn, with the GF(2) bound and on the matching path) is asserted, and the tree
  of game 600 on the matching path with the searchers forgetting their nogoods every 500 steps; the matching path's
  rows are the default's trees before the GF(2) bound, its rows with `lagrange=off` those of the build before
  that tactic, and the rows `symmetry=off` the default's before the rule of symmetric pairs.  **A change that alters
  the tree on purpose fails them.**  Run `cargo test --test tripwires
  -- --nocapture`: it prints the rows as observed, in the layout of the table at the top of the file; check that the
  promotion sums (the answers) have not moved, paste the rows in, and update any node counts quoted in the docs.
* **Engine** (`tests/engine.rs`, `tests/robustness.rs`, `tests/stop_signal.rs`): a toy problem with a known optimum
  (knapsack against dynamic programming), run with 1 and many threads, with a tiny `queue_mem_cap` to force spilling,
  and killed and resumed from checkpoints, must give the same answer; ignored tests do the same with 64 and 192 threads,
  checkpoints every 2 ms, and a panicking worker (`cargo test --release --test engine -- --ignored many_threads`), and
  draw random configurations through any number of resumes (`TXENGINE_STRESS=<n>`).  A tree with no pruning checks
  conservation exactly: under random configurations the nodes counted must equal the tree's size, so one unit lost or
  explored twice shows (`TXENGINE_CONSERVATION=<n>` for more).  Robustness: an interval of zero finishes; limits too
  large for the clock are never reached; a deadline that has passed, or a stop signal received, before a run starts
  stops it before any search, with every unit in its checkpoint; spill segments of dead processes are removed; a locked
  checkpoint directory or results file is refused; a checkpoint of another game, with other tactics or with an incumbent
  that does not fit is refused; and the binary rejects bad times, survives a closed stdout and stderr, and stops on
  SIGINT and SIGTERM with a checkpoint it then resumes.  The control: a ceiling given before a run, or while it runs at
  the incumbent it has, ends it as optimal (with no search work if the start reaches it); offers from outside raise the
  incumbent; a stop through it checkpoints a run that resumes exactly; workers added, retired and down to none
  (suspended, then resumed, checkpointed, stopped, or ended by a ceiling) give the exact optimum; and the conservation
  test runs with a controller that sets 0 to 4 workers every millisecond or so while the run checkpoints, spills and
  resumes.  The counters: capacity is wall × threads when the workers do not change, and a searcher that sleeps before
  its first and second polls shows in the rebuild and first-step times.
* **The window** (`tests/window.rs`; the policy's unit tests in `src/solver/window/allot.rs`): chains 1..600 with
  windows of 1, 2, 4 and 8 on 4 threads have the scores, methods, proofs, ceilings, assumptions, repeats and previous
  games of the committed chain, and pass `verify_files`; with one thread any window gives the lines of a window of one
  but for the times and the build; a repeat of a game still in flight (301 of 300) names it, on a later line; a range
  stopped by a node limit ends at its first unfinished game with the lines before it in order and completes when run
  again; a score that is not the table's fails the range at that game, with the lines before it, as a window of one
  does.  The policy: claims served oldest first, a lone ready search takes every free thread, a busy one doubles, idle
  workers go back only to another claim and after the last grant has settled, the oldest search reclaims from the
  youngest first and suspends them if need be, a suspended search claims one thread.
* **Integrity of results** (`tests/integrity.rs`, `tests/results.rs`, `tests/solver.rs`): a previous game that is not
  what it says is refused; an assumption travels down a chain; a score that is not the table's is a hard error; a
  tampered results file, or one whose line contradicts itself or is of another schema, is refused naming the line, by
  both verifiers; a torn last line is dropped; the committed results pass the verifier with the counts
  `results/README.md` quotes; a game stopped by a node limit and resumed reads back assuming nothing.  Forged repeats
  (another game named, digests or promotions or the promotion sum changed, the game repeated unfinished, after the
  repeat, settling no search, or its assumptions dropped) are refused by both verifiers, and two that only setting the
  problems up again can catch are refused by `verify-results` (the Python referee accepts them, as its docstring says).
* **Repeats** (`tests/same_search.rs`): the games that repeat an earlier search are found from the setups alone, and
  a chain concludes exactly those, with the scores of a chain that searches them: from n = 1 (1..300), and from a
  results file a stopped run left; a game with a checkpoint is resumed, not repeated; a repeat assumes what the game
  it repeats assumes.  A unit test of `board` checks that the problem set up again from a board size is the one the
  setup posed, and the same at two set widths.
* **Large boards** (`tests/wide.rs`): the Java stopped at 1000, and the sets that hold a board are 8 to 128 words
  wide, so nothing above 1023 meets anything but the committed results and these tests.  The same game at every
  width that holds it (128 included) has the same frames, candidates, greedy set, `max_total`, answer and tree; game
  1133 (W = 20, about 2,400 nodes) under every tactic against `strict.jsonl`; a slice of that chain; the sacrifice bound
  proves game 1133 by an independent route; a board of 2050 built at W = 64 (it dispatches to 36) and one of 4100 at
  128 through frames, matching bound, conflicts, greedy set and a node-limited search; boards of 2050, 2400, 2800 and
  3100 at the widths they dispatch to (36, 40, 48 and 56) and at 64 (2050 also at 128), with the same start and the
  same node-limited search, and 4095 at 64 and 128 with the same start; the boards past 4095 (4096, 4100, 4740, 6000,
  6392, 8191) at 128 words against the board layer's own invariants (nine frames, the plain game, the freedoms, the
  greedy set and subsets of it promoted and played to the score their sum says, the root's GF(2) bound between the
  greedy sum and the matching bound, with three, four and five block words); the GF(2) bound's copies of four and
  five words (only at 128) searching game 1000's tree as the copy of one word does; a committed
  checkpoint of game 1133 (`testdata/n1133_after_1000_nodes.ckpt`, W = 20) that must still resume (a deliberate
  change of the checkpoint format fails it; the test says how to make the file again); the levels of a real
  depth-first stack of game 1215 (`testdata/n1215_deep_units.txt`): the
  incremental matching bound and the quick bounds without a member against the min-cost-flow reference; and promoting
  one number at a time (with its no-peel shortcut) against the peel of every frame, in reached states of boards at
  W = 20, 28 and 64.  (The unit tests check the nogood table's lookups against their definition at compact strides of
  1 to 16 words, and debug builds check that every root frame is winnable, which the shortcut needs.)
* **Command line** (`tests/cli.rs`): every subcommand's happy path and its usage errors through the real binary;
  `bench --history FILE` keeps the tracked `bench/history.jsonl` out of the tests.
* **Documentation** (`tests/docs.rs`): the table of tactics in section 5 matches the code; every relative link and
  `#anchor` in the markdown files resolves; the README's table of the fields of a result line, and its example line,
  have the keys the code writes; its reading order is the module map of `src/lib.rs`; `rust-version` is what the README
  and `setup.sh` say; the docs call `unzip`'s moves *mandatory*, never *forced*; and the guides and comments of the
  repository do not use the words the project has retired (the list, with what to say instead, is in the test; the
  dated entries of EXPERIMENTS.md and the note that maps their words are exempt).

A few tests sample what an exhaustive run would cover (every 18th golden board for the peeling agreement, every sixth
board above 511 for the root promotability of the frame builder's candidates, every second line of the Java fixtures);
the comment on each says what the whole run costs.

## 10. Performance

Performance is worked on in cycles and instructions retired, on one thread and a plain release build, because other load
on the machine moves wall times by up to 3×.  What measures what:

* **`taxman bench`** runs a fixed set of games that need search and take about a second each on one thread — n = 936
  (W = 16) and 1050, 1100, 1133, 1170, 1184 (W = 20) — standalone (n − 1 from optimal.json, past it from
  `results/strict.jsonl`; acceleration off), prints per-phase times and node counts, and appends them to
  `bench/history.jsonl` (the commit the binary was built from, every tactic's value, per game the nodes, seconds per
  phase and search counters), so that performance can be followed across changes.  `--history FILE` and `--known FILE`
  name other files; by default they are found in the repository.
* **The probe** (`cargo run --release --example perf`): replays of the hot paths of the hardest games from fixtures in
  `testdata/perf/` (0.2 MB), measured in cycles and instructions retired of the process (`proc_pid_rusage`); each with a
  fingerprint of what it computes, which an exact optimization must leave unchanged.  F1: 300 search nodes each of the
  proofs of 936 and 1215, every kernel over all of them (the rebuild of a node, the promotion of every candidate,
  conflicts, the matching from scratch and warm-started, bounds without each member, strong branching's exact solves,
  children, nogood lookups and rule-outs).  F2: the frame checks of those promotions.  F3: a checkpoint of each proof
  with the default tactics of the time, the matching path's (`perf make-window 936 4500`, `perf make-window 1215
  600000`), resumed on one thread for 50,000 steps, one period of forgetting nogoods: the search itself,
  fingerprinted by its counters, with the tactics its checkpoint records (`--tactic` changes them; a search that ends
  inside the window is costed by the steps it took).  F4, with `--tactic gf2=on`: the GF(2) bound's kernels at the F1
  nodes (the bound from scratch; with its weight split and forced members; without each member of its heaviest set's
  conflict, solved, and as strong branching reads them, certified or solved), and the windows resumed with the GF(2)
  bound, which finish both searches; a window's fingerprint leaves out the GF(2) bound's counts of solves and
  augmentations, which measure work.  The probe's fingerprint is 7e8e724717d26545, 5b83bbebf0bb2899 with `--tactic
  lagrange=off`, and f3deaac0dbdc030a with `--tactic gf2=on`.  (The windows were made again when the search's
  counters were
  renumbered and the checkpoint header went to version 4, 2026-10; the fingerprints EXPERIMENTS quotes before then,
  36981d0eaeeae319 and 6197a47eae6cf8c8, are of the earlier windows, made before `lagrange`.)  About 5 s in all;
  `perf make-nodes` and `perf make-window` make the fixtures again, deterministically.
  For whole searches, `/usr/bin/time -l` reports the same counters per process.
  `cargo test --release` rebuilds the examples in its own configuration, with about 2% more instructions: run
  `cargo build --release --examples` again before measuring.
* **Profile-guided optimization** (`tools/pgo.sh`): an instrumented build runs the probe and a short search at every
  set width (code a profile never saw is optimized as cold), and `-Cprofile-use` builds `target/pgo/release/taxman` and
  the examples.  The same trees and fingerprints.  Since the hot loop is inlined by hand it adds 0–2% (it was 12%
  before).  Profile the plain build: with PGO the search is inlined into a few functions and a profiler sees little.
  On Linux, rustup's `llvm-tools` has an llvm-profdata that matches rustc's LLVM, and the script adds whatever
  `RUSTFLAGS` it is given (for instance `-C target-cpu=native`, which a plain build does not use) to the profile flags.
* **The ladder** (`cargo run --release --example ladder -- [--threads T[,T…] | --scaling] [--seconds S] [--diag]
  [--seed-opt] [--known FILE] [--tactic …] [--node-limit N] N…`; its flags are documented at the top of
  `examples/ladder.rs`): standalone
  searches of hard games from the frame builder, seeded from the known n − 1 game and capped as the solver does, each
  with a time limit; it reports completion, nodes, time, when the final incumbent was found, and the engine's counters.
  `--seed-opt` seeds with the game for n itself, to measure the proof alone; `--diag` prints `search::Diagnostics`
  (nodes by depth, what pruned, bound gaps, conflict sizes, matching work).  Fast progress benchmarks (one thread):
  880, 936, 1050, 1100, 1170, 1184; harder ones: 992, 996, 1025, 1215, 1216 on several threads.
* **Scaling**: the ladder with a list of thread counts (`--threads 1,2,4,8`, or `--scaling` for 1, 2, 4, … up to all
  CPUs) searches each game with each count and prints a table per game and one summed over the games: search time,
  speedup, nodes per second per thread, effective parallelism, idle and lock-wait time, splits and the units they gave,
  steals, runs, tiny runs and the time to full occupancy.  With `--seed-opt` the node count barely depends on the
  threads, so the speedup measures the parallel proof alone.  `--cores C` lets only C of the threads search at once
  (`EngineConfig::cores`), to measure in counts, not time, what many more workers than CPUs would do.  The engine's
  own benchmark (`cargo test --release --test engine -- --ignored --nocapture bench`, sizes and thread counts set by
  `TXENGINE_BENCH_*`) measures its overhead per node with no work per node.
* **The cloud** (`tools/cloud`) has the scripts for a many-core machine: the sweep to 192 threads, the cost of a
  checkpoint pause, and the production chain (RUNBOOK.md).

**Where it stands with the GF(2) bound** (the default since commit 7af8b0e; EXPERIMENTS, "The GF(2) bound in the
search" and "The inner loop of the GF(2) search", have the tables; on the M1 Max): the proof of 1215 takes 4,634 nodes
and 0.85 G cycles on one thread, against the matching path's 1.1 M nodes and 29 G; the 146 one-thread proofs of the
slope study 287,101 nodes and 52 G cycles (697,346 and 232 G before the inner-loop round; 3,408 s on the matching
path); the proofs of 2170 and 2172 65 s each on one thread; `taxman regress --to 1000` on one thread 15.4 s; the
chain 1..1300 on one thread 63 s (34 s of search), against the 988 s for 1001–1300 that the chain of commit 271c817
(the `results/strict.jsonl` before it; git has it) records on 4 threads.  Strong branching makes almost no
augmentations (the bounds without each member are read off the exchange graph), and most children inherit or are
certified: where the time goes now (the proof of 1521, one thread) is the searches for forced members about a
quarter, the weight split a quarter, the solves that are left a fifth (with the tableaux they build, a quarter), and
the cycles of strong branching a ninth.  A node costs 180k cycles on average over the 146 proofs (330k before the
round), in trees 2.4 times smaller.  These are without the rule of symmetric pairs (section 4, "Prime exchanges"),
on by default since: it takes the proof of 1215 to 3,647 nodes and 0.74 G cycles, 2170 and 2172 to 55 s and 62 s,
the chain 1..1300 on one thread to 36 s (26 s of search) and 290,503 nodes against 42 s (31 s) and 402,205 just
before it; the one-thread proofs of the quick set and the bench set 0.75 of the nodes and 0.81 of the cycles
(geometric means; EXPERIMENTS, "Prime exchanges in the search").  A chain from
optimal.json's game of 1300 reached 2175 in 92 minutes of game time on 2 threads, and the committed chain from n = 1
took 27 minutes on 8 ([results/README.md](results/README.md)); its cost comes in waves that
follow the root's GF(2) gap, with no growth that a fit over 1301–2175 resolves, and a day on 192 CPUs would take
it to 4095, the end of the set widths then, at every slope the data support (EXPERIMENTS has the projections); the
widths now go to 8191.

**The matching path** (the default before the GF(2) bound; these are the build of commit 4b6ef38, on the M1 Max): the
proof of 1215 takes 33 s on one thread (106 G cycles), and 1184 takes 3.2 G cycles.  The chain 1..1231 took 279 s of
game time on 8 threads, 145 s of it for 1201–1231, at that commit; the committed `bench/curve/chain.jsonl` (1001–1231),
made by the build before it (e710d16), records 318 s, and 207 s for 1201–1231.  The cost of a game grows by a factor of
ten for every 55 to 70 in n past 1200 ([bench/curve/README.md](bench/curve/README.md)).  The chain 1..1300 of commit
bd9bb5d (on 6 threads of an M1 Max shared with another CPU-heavy job; in git history, `results/strict.jsonl` being a
later run, whose times [results/README.md](results/README.md) gives) records 3359 s of search, 2866 s of it for
1232–1300; its slowest game is 1232 at 439 s.  All of this is without the Lagrangian bound (tactic `lagrange`, section
4), which came later: with it the proof of 1215 takes 1.1 M nodes and 34 G cycles instead of 5.5 M and 108 G, 1184 1.6 G
cycles, and the chain 1201–1231 on four threads 117 s of game time instead of 283 s (EXPERIMENTS: "A Lagrangian bound
over the nogoods").  The round after it took about a tenth off the cycles with the same trees (commit 31b9c95: the proof
of 1215 30 G cycles, 1184 1.5 G, the chain 1201–1231 on one thread 315 s instead of 345 s; a step of the probe's window
of 1215 22.0k cycles, 14.6k with `lagrange=off`; EXPERIMENTS: "Faster nodes with the Lagrangian bound, same trees").
Where the time goes now (the proof of 1215, one thread): the Hungarian searches of the matching 33% and of the
Lagrangian levels 19%, the peels 19%, the node's own work 10%, the nogood table 4%, the refits of the multipliers 3%,
the bounds from the potentials 6%, level copies 4%; by purpose the Lagrangian bound a quarter, making children a
quarter, refinement 14% and strong branching 11%.

## 11. Status and what is left

Every n to 2175 has a game proven within the frame model: `results/strict.jsonl` is a chain from n = 1 whose games to
1000 reproduce optimal.json's scores, whose games 1001–1300 go beyond it (and are now appended to optimal.json too, by
`taxman extend-optimal`) and whose games 1301–2175 are past the table; `results/README.md` says what supports each
score.  The production chain on a many-core machine can start from `strict.jsonl` (`tools/cloud/RUNBOOK.md`), and a
chain from n = 1 reproduces its scores.  With the GF(2) bound (section 4), the chain goes past 1300 on a laptop
(EXPERIMENTS, "The GF(2) bound in the search" and "The committed chain to 2175").  What is left to try: strong
branching's solves, 70% of the intersection's work, which restart far below their answer (canceling the negative cycles
of the optimum less the member would start them next to it); the removal of the matching path's search machinery that
the GF(2) bound leaves idle (refinement, the Lagrangian bound in the search; EXPERIMENTS has the list), once a
production run has been made with it; and, for the many-core machine, whatever its first sweep shows that a laptop
cannot.  The chain keeps several games in flight (section 7, "The window") and the engine reports what a thread loses
at many threads (the units' starts, the lock, the ramp and the tail); on 4 threads of the laptop a step costs 7% more
than on one, none of it in the units' starts, and the window makes 1..1300 16% faster and the games past 2225 no
slower.  On Graviton4 at 64 threads a step inside a unit's tree costs what it does on one thread, and the loss is the
units' first nodes (1.4 to 4%) and the ramp and tail; at 192 threads, on two sockets, the step is 7 to 10% dearer; a
window of 16 makes the games past 2225 a fifth slower on 64 threads, and 4 is the best measured (EXPERIMENTS, "Scaling
to 192 threads").  A unit that carries its GF(2) level, so that a rebuilt one is checked rather than solved, would win
at most the units' first nodes, and is not built; what is still unmeasured is in `tools/cloud/RUNBOOK.md`, section 8.
